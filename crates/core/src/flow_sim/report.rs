use std::collections::BTreeMap as Map;
use std::fmt::Write;

use crate::flags::hash_name;
use crate::flow_sim::db::Db;
use crate::flow_sim::run::{lines, End, Fin, Sim, RK};
use crate::flow_sim::state::{State, V};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Out {
    pub end: String,
    pub writes: Vec<(String, String)>,
    pub grants: Vec<(String, i64)>,
    pub reqs: Vec<String>,
    pub trans: Vec<String>,
    pub viol: Vec<String>,
}

pub struct Group {
    pub out: Out,
    pub n: usize,
    pub at: usize,
}

pub fn end_text(e: &End) -> String {
    match e {
        End::Done => "ends".to_string(),
        End::Menu(w) => format!("external loop at {w}"),
        End::Loop(w) => format!("LOOP at {w}"),
        End::Limit => "LIMIT reached".to_string(),
        End::NoCase(w) => format!("NO CASE ({w})"),
        End::Fail(w) => format!("FAIL ({w})"),
    }
}

pub fn out_of(f: &Fin) -> Out {
    Out {
        end: end_text(&f.end),
        writes: f.m.writes.iter().filter(|(k, _)| !k.starts_with("UI_")).map(|(k, v)| (k.clone(), v.clone())).collect(),
        grants: f.m.gr.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        reqs: f.m.reqs.clone(),
        trans: f.m.trans.clone(),
        viol: f.m.viol.clone(),
    }
}

pub fn groups(fins: &[Fin]) -> Vec<Group> {
    let mut by: Map<Out, (usize, usize)> = Map::new();
    for (i, f) in fins.iter().enumerate() {
        if matches!(f.end, End::Menu(_)) {
            continue;
        }
        let e: &mut (usize, usize) = by.entry(out_of(f)).or_insert((0, i));
        e.0 += 1;
    }
    by.into_iter().map(|(out, (n, at))| Group { out, n, at }).collect()
}

pub fn trace(f: &Fin, verbose: bool) -> Vec<String> {
    let mut o: Vec<String> = Vec::new();
    for (k, t) in lines(&f.m.log) {
        let show: bool = match k {
            RK::Ev => verbose || t.starts_with("enter"),
            _ => true,
        };
        if show {
            let tag: &str = match k {
                RK::Ev => "  ",
                RK::Ask => "? ",
                RK::Set => "= ",
                RK::Grant => "+ ",
                RK::Chal => "# ",
                RK::Msg => "\" ",
                RK::Trans => "> ",
                RK::Scene => "> ",
                RK::Warn => "! ",
            };
            o.push(format!("{tag}{t}"));
        }
    }
    o
}

pub fn summary(g: &Group) -> String {
    let mut o: String = String::new();
    let _ = writeln!(o, "outcome: {} ({} path{})", g.out.end, g.n, if g.n == 1 { "" } else { "s" });
    for (k, v) in &g.out.writes {
        let _ = writeln!(o, "  wrote {k} = {v}");
    }
    for (k, v) in &g.out.grants {
        let _ = writeln!(o, "  granted {k} x{v}");
    }
    for r in &g.out.reqs {
        let _ = writeln!(o, "  step request {r}");
    }
    for t in &g.out.trans {
        let _ = writeln!(o, "  leaves to {t}");
    }
    for v in &g.out.viol {
        let _ = writeln!(o, "  INVARIANT BROKEN {v}");
    }
    o
}

pub fn render(s: &Sim, fins: &[Fin], shown: usize, verbose: bool) -> String {
    let mut o: String = String::new();
    let gs: Vec<Group> = groups(fins);
    let menus: usize = fins.iter().filter(|f| matches!(f.end, End::Menu(_))).count();
    let _ = writeln!(o, "{} paths explored, {} distinct outcomes, {} merged duplicates, {} paths pruned as external loops (a player, random, time or engine decision leaves them)", fins.len(), gs.len(), s.dropped, menus);
    for (i, g) in gs.iter().enumerate() {
        let _ = writeln!(o, "--- outcome {} of {}", i + 1, gs.len());
        o.push_str(&summary(g));
        if i < shown {
            let _ = writeln!(o, "  witness trace:");
            for l in trace(&fins[g.at], verbose) {
                let _ = writeln!(o, "    {l}");
            }
        }
    }
    o
}

pub fn diff(a: &[Group], b: &[Group]) -> String {
    let mut o: String = String::new();
    for g in a {
        if !b.iter().any(|x| x.out == g.out) {
            let _ = writeln!(o, "only in A:");
            o.push_str(&summary(g));
        }
    }
    for g in b {
        if !a.iter().any(|x| x.out == g.out) {
            let _ = writeln!(o, "only in B:");
            o.push_str(&summary(g));
        }
    }
    if o.is_empty() {
        o.push_str("A and B have identical outcome sets\n");
    }
    o
}

pub fn step_of(db: &Db, st: &State, chal: &str) -> String {
    let h: u32 = hash_name(&format!("Step_{chal}"));
    match db.get(st, h) {
        Some(V::E(e)) => db.enum_name(h, e),
        _ => "?".to_string(),
    }
}

fn cmp(v: &str) -> (String, char, i64) {
    for (t, c) in [("<=", '<'), (">=", '>'), ("=", '=')] {
        if let Some((a, b)) = v.split_once(t) {
            return (a.to_string(), c, b.parse().unwrap_or(1));
        }
    }
    (v.to_string(), '=', 1)
}

fn holds(got: i64, c: char, want: i64) -> bool {
    match c {
        '<' => got <= want,
        '>' => got >= want,
        _ => got == want,
    }
}

pub fn expect(s: &Sim, fins: &[Fin], rules: &[String]) -> Vec<String> {
    let mut bad: Vec<String> = Vec::new();
    let gs: Vec<Group> = groups(fins);
    for r in rules {
        let (k, v): (&str, &str) = r.split_once(':').unwrap_or((r.as_str(), ""));
        for (i, g) in gs.iter().enumerate() {
            let f: &Fin = &fins[g.at];
            let p: String = format!("outcome {} ({} paths, {})", i + 1, g.n, end_text(&f.end));
            match k {
                "end" => {
                    if f.end != End::Done {
                        bad.push(format!("{r}: {p} does not end cleanly"));
                    }
                }
                "grant" => {
                    let (it, c, want): (String, char, i64) = cmp(v);
                    let got: i64 = f.m.gr.get(&it).copied().unwrap_or(0);
                    if !holds(got, c, want) {
                        bad.push(format!("{r}: {p} granted {got}"));
                    }
                }
                "total" => {
                    let (it, c, want): (String, char, i64) = cmp(v);
                    let got: i64 = f.m.st.grants.get(&it).copied().unwrap_or(0);
                    if !holds(got, c, want) {
                        bad.push(format!("{r}: {p} granted {got} over the whole script"));
                    }
                }
                "flag" => {
                    let (n, x): (&str, &str) = v.split_once('=').unwrap_or((v, "true"));
                    match f.m.writes.get(n) {
                        Some(w) if w == x => {}
                        other => bad.push(format!("{r}: {p} wrote {other:?}")),
                    }
                }
                "step" => {
                    let (c, x): (&str, &str) = v.split_once('=').unwrap_or((v, ""));
                    let got: String = step_of(&s.db, &f.m.st, c);
                    if got != x {
                        bad.push(format!("{r}: {p} leaves {c} at {got}"));
                    }
                }
                "clean" => {
                    if !f.m.viol.is_empty() {
                        bad.push(format!("{r}: {p} breaks {}", f.m.viol.join(", ")));
                    }
                }
                _ => bad.push(format!("{r}: unknown rule kind")),
            }
        }
    }
    bad
}
