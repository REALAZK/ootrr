use crate::murmur::murmur3_32;
use crate::tracker::peek::Peek;
use crate::tracker::targets::{How, Target};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Off,
    On,
    Weak,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub how: How,
    pub flags: Vec<u32>,
    pub keys: Vec<u64>,
    pub weak: bool,
}

pub fn rule_of(t: &Target) -> Rule {
    Rule {
        how: t.how,
        flags: t.flags.iter().map(|f| murmur3_32(f.as_bytes(), 0)).collect(),
        keys: t.keys.clone(),
        weak: t.weak,
    }
}

pub fn eval(r: &Rule, p: &Peek) -> Hit {
    match r.how {
        How::Flag => {
            if !r.flags.iter().any(|h| p.on(*h)) {
                Hit::Off
            } else if r.weak {
                Hit::Weak
            } else {
                Hit::On
            }
        }
        How::Key => {
            if r.keys.iter().any(|k| p.has_key(*k)) {
                Hit::On
            } else {
                Hit::Off
            }
        }
        How::None => Hit::Unknown,
    }
}

pub fn eval_all(rs: &[Rule], p: &Peek) -> Vec<Hit> {
    rs.iter().map(|r| eval(r, p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::fixture::{Cell, raw};
    use crate::tracker::peek::{Lim, peek_with};
    use crate::tracker::targets::{Target, all};

    fn first(kind: &str) -> &'static Target {
        all().unwrap().iter().find(|t| t.kind == kind && t.how != How::None).unwrap()
    }

    fn save_with(flags: &[&str], ints: &[(&str, u32)], keys: &[u64]) -> Peek {
        let mut cells: Vec<(u32, Cell)> = flags.iter().map(|f| (murmur3_32(f.as_bytes(), 0), Cell::Inline(1))).collect();
        for (n, v) in ints {
            cells.push((murmur3_32(n.as_bytes(), 0), Cell::Inline(*v)));
        }
        let mut ks: Vec<u64> = keys.to_vec();
        ks.sort();
        peek_with(&raw(&cells, &ks), &Lim::ANY).unwrap()
    }

    fn on_off(t: &Target, on: &Peek) -> (Hit, Hit) {
        let r: Rule = rule_of(t);
        (eval(&r, on), eval(&r, &save_with(&[], &[], &[])))
    }

    fn by_flag(kind: &str) {
        let t: &Target = first(kind);
        assert_eq!(t.how, How::Flag, "{kind}");
        let p: Peek = save_with(&[t.flags[0].as_str()], &[], &[]);
        let want: Hit = if t.weak { Hit::Weak } else { Hit::On };
        assert_eq!(on_off(t, &p), (want, Hit::Off), "{kind}");
    }

    fn by_key(kind: &str) {
        let t: &Target = first(kind);
        assert_eq!(t.how, How::Key, "{kind}");
        let p: Peek = save_with(&[], &[], &[t.keys[0]]);
        assert_eq!(on_off(t, &p), (Hit::On, Hit::Off), "{kind}");
    }

    #[test]
    fn chest_by_its_open_flag() {
        by_flag("chest");
    }

    #[test]
    fn small_key_by_its_named_flag() {
        by_flag("key");
    }

    #[test]
    fn cow_is_weak_when_the_shared_flag_is_set() {
        let c: &Target = all().unwrap().iter().find(|t| t.kind == "cow" && t.weak).unwrap();
        let p: Peek = save_with(&[c.flags[0].as_str()], &[], &[]);
        assert_eq!(on_off(c, &p), (Hit::Weak, Hit::Off));
        let home: &Target = all().unwrap().iter().find(|t| t.kind == "cow" && !t.weak).unwrap();
        let q: Peek = save_with(&[home.flags[0].as_str()], &[], &[]);
        assert_eq!(on_off(home, &q), (Hit::On, Hit::Off));
    }

    #[test]
    fn gift_by_its_get_event_flag() {
        by_flag("gift");
        let g: &Target = all().unwrap().iter().find(|t| t.kind == "gift" && t.weak).unwrap();
        let p: Peek = save_with(&[g.flags[0].as_str()], &[], &[]);
        assert_eq!(eval(&rule_of(g), &p), Hit::Weak);
    }

    #[test]
    fn shop_by_a_nonzero_sold_counter_on_either_keeper_copy() {
        let t: &Target = first("shop");
        assert_eq!(t.flags.len(), 2);
        let p: Peek = save_with(&[], &[(t.flags[1].as_str(), 3)], &[]);
        assert_eq!(on_off(t, &p), (Hit::On, Hit::Off));
        let z: Peek = save_with(&[], &[(t.flags[0].as_str(), 0)], &[]);
        assert_eq!(eval(&rule_of(t), &z), Hit::Off);
    }

    #[test]
    fn heart_piece_by_flag_or_by_key() {
        let all_t: &[Target] = all().unwrap();
        let f: &Target = all_t.iter().find(|t| t.kind == "heart_piece" && t.how == How::Flag).unwrap();
        let k: &Target = all_t.iter().find(|t| t.kind == "heart_piece" && t.how == How::Key).unwrap();
        assert_eq!(on_off(f, &save_with(&[f.flags[0].as_str()], &[], &[])), (Hit::On, Hit::Off));
        assert_eq!(on_off(k, &save_with(&[], &[], &[k.keys[0]])), (Hit::On, Hit::Off));
    }

    #[test]
    fn pot_by_key() {
        by_key("pot");
    }

    #[test]
    fn flying_pot_by_key() {
        by_key("flying_pot");
    }

    #[test]
    fn crate_by_key() {
        by_key("crate");
    }

    #[test]
    fn beehive_by_any_of_its_keys() {
        let t: &Target = all().unwrap().iter().find(|t| t.kind == "beehive" && t.keys.len() > 1).unwrap();
        let p: Peek = save_with(&[], &[], &[t.keys[t.keys.len() - 1]]);
        assert_eq!(on_off(t, &p), (Hit::On, Hit::Off));
    }

    #[test]
    fn rupee_by_key() {
        by_key("rupee");
    }

    #[test]
    fn silver_rupee_by_key() {
        by_key("silver_rupee");
    }

    #[test]
    fn heart_by_key() {
        by_key("heart");
    }

    #[test]
    fn heart_container_by_key() {
        by_key("heart_container");
    }

    #[test]
    fn scrub_by_key() {
        by_key("scrub");
    }

    #[test]
    fn pickup_by_key() {
        by_key("pickup");
    }

    #[test]
    fn skulltula_by_its_own_key_not_the_token_key() {
        let t: &Target = first("skulltula");
        assert_eq!(on_off(t, &save_with(&[], &[], &[t.keys[0]])), (Hit::On, Hit::Off));
        assert_eq!(eval(&rule_of(t), &save_with(&[], &[], &[t.keys2[0]])), Hit::Off);
    }

    fn ids(j: &crate::json::Json, k: &str) -> Vec<String> {
        let mut v: Vec<String> = j
            .get(k)
            .and_then(|x| x.as_arr())
            .unwrap()
            .iter()
            .map(|c| c.get("id").and_then(|x| x.as_str()).unwrap().to_string())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn golden_saves_match_check_log() {
        let Ok(dir) = std::env::var("OOTRR_TEST_SAVES") else {
            return;
        };
        let ts: &[Target] = all().unwrap();
        let rs: Vec<Rule> = ts.iter().map(rule_of).collect();
        let mut n: usize = 0;
        let mut on: usize = 0;
        for e in std::fs::read_dir(&dir).unwrap() {
            let d: std::path::PathBuf = e.unwrap().path();
            let want: std::path::PathBuf = d.join("check_log.json");
            if !want.is_file() {
                continue;
            }
            let b: Vec<u8> = std::fs::read(d.join("progress.sav")).unwrap();
            let p: Peek = crate::tracker::peek::peek(&b).unwrap();
            let j: crate::json::Json = crate::json::parse(&std::fs::read_to_string(&want).unwrap()).unwrap();
            let hits: Vec<Hit> = eval_all(&rs, &p);
            let mut got_on: Vec<String> = ts.iter().zip(&hits).filter(|x| *x.1 == Hit::On).map(|x| x.0.id.clone()).collect();
            let mut got_weak: Vec<String> = ts.iter().zip(&hits).filter(|x| *x.1 == Hit::Weak).map(|x| x.0.id.clone()).collect();
            got_on.sort();
            got_weak.sort();
            assert_eq!(got_on, ids(&j, "collected"), "{}", d.display());
            assert_eq!(got_weak, ids(&j, "weak"), "{}", d.display());
            assert_eq!(Some(p.keys.len() as i64), j.get("n_keys").and_then(|x| x.as_i64()), "{}", d.display());
            let pl: &crate::json::Json = j.get("place").unwrap();
            assert_eq!(p.place.map.as_deref(), pl.get("map").and_then(|x| x.as_str()), "{}", d.display());
            assert_eq!(Some(p.place.child), pl.get("child").and_then(|x| x.as_bool()), "{}", d.display());
            assert_eq!(p.place.room.map(|r| r as i64), pl.get("room").and_then(|x| x.as_i64()), "{}", d.display());
            n += 1;
            on += got_on.len();
        }
        eprintln!("golden: {n} saves, {on} collected checks, all equal");
        assert!(n > 0);
    }

    #[test]
    fn trap_chest_is_never_detected() {
        let t: &Target = all().unwrap().iter().find(|t| t.kind == "trap_chest").unwrap();
        assert_eq!(eval(&rule_of(t), &save_with(&[], &[], &[])), Hit::Unknown);
    }
}
