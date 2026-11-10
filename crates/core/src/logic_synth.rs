use std::fmt::Write as W;

use crate::error::Res;
use crate::rng::Rng;
use crate::world::{Spec, World};

fn region_name(i: usize) -> String {
    if i == 0 { "Root".to_string() } else { format!("R{i}") }
}

fn key(k: usize) -> String {
    format!("Key_{k}")
}

pub fn synth_spec(locs: usize, seed: u64) -> Res<Spec> {
    let mut r: Rng = Rng::from_seed(seed);
    let nr: usize = (locs / 4).max(2);
    let nk: usize = (locs / 8).max(2);
    let mut s: Spec = Spec::new();
    for k in 0..nk {
        s.add_item(&format!("Key {k}"), true, 1);
    }
    s.add_item("Junk", false, (locs - nk.min(locs)) as u32);
    let goal_keys: usize = nk.min(8);
    let goal: Vec<String> = (0..goal_keys).map(key).collect();
    s.goal = "Win".to_string();
    s.add_helpers(r#"{"both(a, b)": "a and b", "either(a, b)": "a or b"}"#)?;
    let mut t: String = String::from("[");
    for i in 0..nr {
        if i > 0 {
            t.push(',');
        }
        let rn: String = region_name(i);
        let _ = write!(t, "{{\"region_name\": \"{rn}\", \"locations\": {{");
        let n: usize = locs / nr + usize::from(i < locs % nr);
        for j in 0..n {
            if j > 0 {
                t.push(',');
            }
            let a: usize = r.below(nk as u64) as usize;
            let b: usize = r.below(nk as u64) as usize;
            let rule: String = match r.below(10) {
                _ if i == 0 && j == 0 => "True".to_string(),
                0..=4 => "True".to_string(),
                5..=7 => key(a),
                8 => format!("both({}, {})", key(a), key(b)),
                _ => format!("either({}, {})", key(a), key(b)),
            };
            let _ = write!(t, "\"R{i} L{j}\": \"{rule}\"");
        }
        t.push_str("}, \"exits\": {");
        let mut first: bool = true;
        for c in (i + 1)..nr.min(i + 4) {
            if !first {
                t.push(',');
            }
            first = false;
            let g: String = if i > 0 && c - 1 < nk && r.below(3) > 0 { key(c - 1) } else { "True".to_string() };
            let _ = write!(t, "\"{}\": \"{g}\"", region_name(c));
        }
        t.push('}');
        if i == nr - 1 {
            let _ = write!(t, ", \"events\": {{\"Win\": \"{}\"}}", goal.join(" and "));
        }
        t.push('}');
    }
    t.push(']');
    s.add_regions(&t)?;
    Ok(s)
}

pub fn chain_spec(locs: usize) -> Res<Spec> {
    let nr: usize = (locs / 4).max(2);
    let mut s: Spec = Spec::new();
    for k in 0..nr - 1 {
        s.add_item(&format!("Key {k}"), true, 1);
    }
    s.add_item("Junk", false, (locs - (nr - 1).min(locs)) as u32);
    s.goal = "Win".to_string();
    let mut t: String = String::from("[");
    for i in 0..nr {
        if i > 0 {
            t.push(',');
        }
        let _ = write!(t, "{{\"region_name\": \"{}\", \"locations\": {{", region_name(i));
        let n: usize = locs / nr + usize::from(i < locs % nr);
        for j in 0..n {
            if j > 0 {
                t.push(',');
            }
            let rule: String = if i == 0 { "True".to_string() } else { key(i - 1) };
            let _ = write!(t, "\"R{i} L{j}\": \"{rule}\"");
        }
        t.push('}');
        if i + 1 < nr {
            let _ = write!(t, ", \"exits\": {{\"{}\": \"{}\"}}", region_name(i + 1), key(i));
        } else {
            t.push_str(", \"events\": {\"Win\": \"True\"}");
        }
        t.push('}');
    }
    t.push(']');
    s.add_regions(&t)?;
    Ok(s)
}

pub fn synth_world(locs: usize, seed: u64) -> Res<World> {
    World::build(&synth_spec(locs, seed)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fill::{fill, FillCfg, Filled};
    use crate::search::explore;
    use crate::logic_items::ItemId;

    #[test]
    fn generated_worlds_have_the_requested_size() {
        for n in [8usize, 100, 333, 1000] {
            let w: World = synth_world(n, 5).unwrap();
            assert_eq!(w.real.len(), n);
            assert_eq!(w.pool().len(), n);
        }
    }

    #[test]
    fn generated_worlds_are_winnable_and_fillable() {
        let w: World = synth_world(400, 11).unwrap();
        let pool: Vec<ItemId> = w.pool();
        for seed in 0..5u64 {
            let f: Filled = fill(&w, &pool, &mut Rng::from_seed(seed), &FillCfg::new()).unwrap();
            assert!(explore(&w, &f.place, &[]).won());
        }
    }
}
