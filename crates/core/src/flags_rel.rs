use std::collections::HashMap as Map;

use crate::flags::{Body, FlagList, hash_name};
use crate::flags_harvest::Mem;

const SEPS: [char; 2] = ['.', '_'];

fn struct_list(l: &FlagList) -> Vec<(u64, Vec<(u32, u32)>)> {
    let mut out: Vec<(u64, Vec<(u32, u32)>)> = Vec::new();
    for f in &l.flags {
        if let Body::Members(m) = &f.body {
            out.push((f.hash, m.clone()));
        }
    }
    out
}

fn name_of(l: &FlagList, h: u64) -> Option<String> {
    l.by_hash(h).and_then(|f| f.name.clone())
}

pub fn structs(l: &mut FlagList, mn: &mut Mem) -> usize {
    let list: Vec<(u64, Vec<(u32, u32)>)> = struct_list(l);
    let mut total: usize = 0;
    loop {
        let mut new: usize = 0;
        for (sh, mem) in &list {
            for (mh, v) in mem {
                let vh: u64 = *v as u64;
                let sn: Option<String> = name_of(l, *sh);
                let vn: Option<String> = name_of(l, vh);
                match (&sn, &vn) {
                    (None, Some(n)) => {
                        let cuts: Vec<usize> = n
                            .char_indices()
                            .filter(|(i, c)| *i > 0 && SEPS.contains(c))
                            .map(|(i, _)| i)
                            .collect();
                        for i in cuts {
                            if l.name(*sh, &n[..i], "struct") {
                                new += 1;
                                break;
                            }
                        }
                    }
                    (Some(p), Some(n)) => {
                        if !mn.contains_key(mh)
                            && n.len() > p.len() + 1
                            && n.starts_with(p.as_str())
                            && n[p.len()..].starts_with(SEPS)
                        {
                            let m: &str = &n[p.len() + 1..];
                            if hash_name(m) == *mh {
                                mn.insert(*mh, m.to_string());
                                new += 1;
                            }
                        }
                    }
                    (Some(p), None) => {
                        if let Some(m) = mn.get(mh).cloned() {
                            for sep in SEPS {
                                let c: String = format!("{p}{sep}{m}");
                                if l.name(vh, &c, "struct") {
                                    new += 1;
                                    break;
                                }
                            }
                        }
                    }
                    (None, None) => {}
                }
            }
        }
        total += new;
        if new == 0 {
            return total;
        }
    }
}

fn spans(s: &str) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut from: Option<usize> = None;
    for (i, c) in s.char_indices() {
        if SEPS.contains(&c) {
            if let Some(f) = from.take() {
                out.push((f, i));
            }
        } else if from.is_none() {
            from = Some(i);
        }
    }
    if let Some(f) = from {
        out.push((f, s.len()));
    }
    out
}

type Rule = (String, String, String, String);

fn learn(a: &str, b: &str) -> Option<Rule> {
    let sa: Vec<(usize, usize)> = spans(a);
    let sb: Vec<(usize, usize)> = spans(b);
    let mut best: (usize, usize, usize) = (0, 0, 0);
    for i in 0..sa.len() {
        for j in 0..sb.len() {
            let mut k: usize = 0;
            while i + k < sa.len()
                && j + k < sb.len()
                && a[sa[i + k].0..sa[i + k].1] == b[sb[j + k].0..sb[j + k].1]
            {
                k += 1;
            }
            if k > best.2 {
                best = (i, j, k);
            }
        }
    }
    if best.2 == 0 {
        return None;
    }
    let (i, j, k) = best;
    let a0: usize = sa[i].0;
    let a1: usize = sa[i + k - 1].1;
    let b0: usize = sb[j].0;
    let b1: usize = sb[j + k - 1].1;
    Some((
        a[..a0].to_string(),
        a[a1..].to_string(),
        b[..b0].to_string(),
        b[b1..].to_string(),
    ))
}

fn pairs(l: &FlagList) -> Vec<(u64, u64)> {
    let mut out: Vec<(u64, u64)> = Vec::new();
    for f in &l.flags {
        if let Body::Exp(t) = &f.body {
            for (_, v) in t {
                let vh: u64 = *v as u64;
                if vh != f.hash && l.by_hash(vh).is_some() {
                    out.push((f.hash, vh));
                }
            }
        }
    }
    out
}

pub fn enum_values(l: &FlagList) -> Map<u32, String> {
    let mut out: Map<u32, String> = Map::new();
    for f in &l.flags {
        if let Body::Values(v) = &f.body {
            for n in v {
                out.insert(hash_name(n), n.clone());
            }
        }
    }
    out
}

const QUEST: [&str; 4] = [
    "{q}_IsAfter_{c}_Exp",
    "{q}_IsAfter{c}_Exp",
    "{q}_Is{c}_Exp",
    "{q}_{c}_Exp",
];

fn quest_names(l: &mut FlagList) -> usize {
    let vals: Map<u32, String> = enum_values(l);
    let mut todo: Vec<(u64, u64, u32)> = Vec::new();
    for f in &l.flags {
        if let Body::Exp(t) = &f.body
            && f.name.is_none()
            && t.len() == 2
            && t[0].0 == 12
            && t[1].0 == 2
        {
            todo.push((f.hash, t[0].1 as u64, t[1].1));
        }
    }
    let mut new: usize = 0;
    for (e, flag, val) in todo {
        let (Some(fname), Some(c)) = (name_of(l, flag), vals.get(&val)) else {
            continue;
        };
        let q: &str = fname.strip_prefix("Step_").unwrap_or(&fname);
        for t in QUEST {
            let n: String = t.replace("{q}", q).replace("{c}", c);
            if l.name(e, &n, "exp") {
                new += 1;
                break;
            }
        }
    }
    new
}

fn apply(rules: &[Rule], known: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (pa, sa, pb, sb) in rules {
        if known.len() > pa.len() + sa.len()
            && known.starts_with(pa.as_str())
            && known.ends_with(sa.as_str())
        {
            let core: &str = &known[pa.len()..known.len() - sa.len()];
            out.push(format!("{pb}{core}{sb}"));
        }
    }
    out
}

pub fn exps(l: &mut FlagList) -> usize {
    let rel: Vec<(u64, u64)> = pairs(l);
    let mut total: usize = 0;
    loop {
        let mut rules: Vec<Rule> = Vec::new();
        for (a, b) in &rel {
            if let (Some(x), Some(y)) = (name_of(l, *a), name_of(l, *b)) {
                if let Some(r) = learn(&x, &y) {
                    rules.push(r);
                }
                if let Some(r) = learn(&y, &x) {
                    rules.push(r);
                }
            }
        }
        rules.sort();
        rules.dedup();
        let mut new: usize = 0;
        for (a, b) in &rel {
            for (k, u) in [(*a, *b), (*b, *a)] {
                let (Some(kn), None) = (name_of(l, k), name_of(l, u)) else {
                    continue;
                };
                for c in apply(&rules, &kn) {
                    if l.name(u, &c, "exp") {
                        new += 1;
                        break;
                    }
                }
            }
        }
        for (a, b) in &rel {
            if let (Some(n), None) = (name_of(l, *a), name_of(l, *b)) {
                let cuts: Vec<usize> = n.match_indices('_').map(|(i, _)| i).collect();
                for i in cuts {
                    let c: String = format!("Step_{}", &n[..i]);
                    if i > 0 && (l.name(*b, &n[..i], "exp") || l.name(*b, &c, "exp")) {
                        new += 1;
                        break;
                    }
                }
            }
        }
        new += quest_names(l);
        total += new;
        if new == 0 {
            return total;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::Flag;
    use crate::json::Json;

    fn flag(n: &str, ty: &str, body: Body) -> Flag {
        Flag {
            hash: hash_name(n) as u64,
            ty: ty.to_string(),
            def: Json::Null,
            reset: 0,
            save: 0,
            extra: None,
            size: None,
            body,
            name: None,
            how: None,
        }
    }

    #[test]
    fn struct_names_flow_both_ways() {
        let m: u32 = hash_name("StockNum");
        let v: u32 = hash_name("PouchData.Item_X.StockNum");
        let s: Flag = flag("PouchData.Item_X", "Struct", Body::Members(vec![(m, v)]));
        let f: Flag = flag("PouchData.Item_X.StockNum", "Int", Body::Plain);
        let mut l: FlagList = FlagList::from_flags(vec![s, f]);
        let mut mn: Mem = Mem::new();
        l.name(v as u64, "PouchData.Item_X.StockNum", "t");
        assert_eq!(structs(&mut l, &mut mn), 2);
        assert_eq!(mn.get(&m).map(|s| s.as_str()), Some("StockNum"));
        assert_eq!(l.named(), 2);
    }

    #[test]
    fn rule_learning_pairs_siblings() {
        let r: Rule =
            learn("DreamWorldPlate_Enemy_Amos", "HaveDefeatedEnemy.Enemy_Amos").unwrap_or_default();
        assert_eq!(r.0, "DreamWorldPlate_");
        assert_eq!(r.2, "HaveDefeatedEnemy.");
        assert_eq!(
            apply(&[r], "DreamWorldPlate_Enemy_Bari"),
            vec!["HaveDefeatedEnemy.Enemy_Bari"]
        );
    }

    #[test]
    fn quest_template_names_exp() {
        let ef: &str = "Step_AllSage";
        let f: Flag = flag(ef, "Enum", Body::Values(vec!["Ready".to_string()]));
        let e: Flag = flag(
            "AllSage_IsAfter_Ready_Exp",
            "BoolExp",
            Body::Exp(vec![(12, f.hash as u32), (2, hash_name("Ready"))]),
        );
        let mut l: FlagList = FlagList::from_flags(vec![f, e]);
        l.name(hash_name(ef) as u64, ef, "t");
        assert_eq!(exps(&mut l), 1);
        assert_eq!(l.named(), 2);
    }
}
