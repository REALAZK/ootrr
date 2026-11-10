use std::collections::BTreeMap as BMap;
use std::collections::BTreeSet as BSet;
use std::collections::HashMap as Map;
use std::collections::HashSet as Set;

use crate::flags::{Body, FlagList, KEYED};
use crate::flags_harvest::Mem;
use crate::murmur::murmur3_32;

const FAMILY_CAP: u64 = 400_000_000;
const DIGIT_MAX: u32 = 3000;
const MIN_FAMILY: usize = 3;
const SLOT_SUPPORT: usize = 3;
const WIDE_TYPES: [&str; 8] = [
    "Enum", "BoolExp", "Struct", "Float", "String64", "Vector3", "Binary", "UInt",
];
const FLAG_DIGITS: u32 = 300;

pub fn unresolved(l: &FlagList, mn: &Mem) -> Set<u32> {
    let mut out: Set<u32> = Set::new();
    for f in &l.flags {
        if let Body::Members(m) = &f.body {
            for (k, _) in m {
                if !mn.contains_key(k) {
                    out.insert(*k);
                }
            }
        }
    }
    out
}

fn product(pos: &[Vec<String>], f: &mut dyn FnMut(&str)) {
    let n: usize = pos.len();
    if n == 0 || pos.iter().any(|p| p.is_empty()) {
        return;
    }
    let mut idx: Vec<usize> = vec![0; n];
    let mut buf: String = String::new();
    loop {
        buf.clear();
        for i in 0..n {
            if i > 0 {
                buf.push('_');
            }
            buf.push_str(&pos[i][idx[i]]);
        }
        f(&buf);
        let mut k: usize = n;
        loop {
            if k == 0 {
                return;
            }
            k -= 1;
            idx[k] += 1;
            if idx[k] < pos[k].len() {
                break;
            }
            idx[k] = 0;
        }
    }
}

fn digit_runs(s: &str) -> Vec<(usize, usize)> {
    let b: &[u8] = s.as_bytes();
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut i: usize = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            let st: usize = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            out.push((st, i));
        } else {
            i += 1;
        }
    }
    out
}

fn families(keys: &[String], f: &mut dyn FnMut(&str)) {
    let mut fam: BMap<(usize, String), Vec<Vec<String>>> = BMap::new();
    for k in keys {
        let t: Vec<String> = k.split('_').map(|s| s.to_string()).collect();
        fam.entry((t.len(), t[0].clone())).or_default().push(t);
    }
    for ((n, _), ts) in &fam {
        if ts.len() < MIN_FAMILY {
            continue;
        }
        let mut pos: Vec<Vec<String>> = Vec::new();
        let mut size: u64 = 1;
        for i in 0..*n {
            let s: BSet<&String> = ts.iter().map(|t| &t[i]).collect();
            size = size.saturating_mul(s.len() as u64);
            pos.push(s.into_iter().cloned().collect());
        }
        if size <= FAMILY_CAP {
            product(&pos, f);
        }
    }
}

fn mutations(keys: &[String], f: &mut dyn FnMut(&str)) {
    let mut vocab: Map<(usize, usize), Set<String>> = Map::new();
    for k in keys {
        let t: Vec<&str> = k.split('_').collect();
        for (i, tk) in t.iter().enumerate() {
            vocab
                .entry((t.len(), i))
                .or_default()
                .insert(tk.to_string());
        }
    }
    for k in keys {
        for (s, e) in digit_runs(k) {
            let w: usize = e - s;
            for v in 0..DIGIT_MAX {
                let padded: String = format!("{}{:0w$}{}", &k[..s], v, &k[e..], w = w);
                f(&padded);
                let bare: String = format!("{}{}{}", &k[..s], v, &k[e..]);
                if bare != padded {
                    f(&bare);
                }
            }
        }
        let t: Vec<&str> = k.split('_').collect();
        for i in 0..t.len() {
            let Some(vs) = vocab.get(&(t.len(), i)) else {
                continue;
            };
            for v in vs {
                let mut c: String = String::new();
                for (j, tk) in t.iter().enumerate() {
                    if j > 0 {
                        c.push('_');
                    }
                    c.push_str(if j == i { v } else { tk });
                }
                f(&c);
            }
        }
    }
}

fn segments(n: &str) -> (Vec<&str>, Vec<&str>) {
    let mut toks: Vec<&str> = Vec::new();
    let mut seps: Vec<&str> = Vec::new();
    let mut from: usize = 0;
    let mut prev: Option<char> = None;
    for (i, c) in n.char_indices() {
        if c == '.' || c == '_' {
            toks.push(&n[from..i]);
            seps.push(&n[i..i + 1]);
            from = i + 1;
        } else if c.is_ascii_uppercase() && prev.is_some_and(|p| p.is_ascii_lowercase()) {
            toks.push(&n[from..i]);
            seps.push("");
            from = i;
        }
        prev = Some(c);
    }
    toks.push(&n[from..]);
    seps.push("");
    (toks, seps)
}

fn rebuild(toks: &[&str], seps: &[&str], at: usize, v: &str) -> String {
    let mut out: String = String::new();
    for (j, t) in toks.iter().enumerate() {
        out.push_str(if j == at { v } else { t });
        out.push_str(seps[j]);
    }
    out
}

pub fn siblings(l: &mut FlagList) -> usize {
    let mut want: Map<u32, (u64, String)> = Map::new();
    for f in &l.flags {
        if f.ty != KEYED && f.name.is_none() {
            want.insert(f.hash as u32, (f.hash, f.ty.clone()));
        }
    }
    if want.is_empty() {
        return 0;
    }
    let named: Vec<(String, String)> = l
        .flags
        .iter()
        .filter(|f| {
            f.ty != KEYED
                && f.how
                    .as_deref()
                    .is_some_and(|h| !h.starts_with("placement"))
        })
        .filter_map(|f| f.name.clone().map(|n| (n, f.ty.clone())))
        .collect();
    let mut slot: Map<(usize, usize, String), Map<String, usize>> = Map::new();
    let mut wide: Map<String, usize> = Map::new();
    for (n, ty) in &named {
        let (t, _) = segments(n);
        for (i, x) in t.iter().enumerate() {
            if !x.is_empty() {
                *slot
                    .entry((t.len(), i, ty.clone()))
                    .or_default()
                    .entry(x.to_string())
                    .or_default() += 1;
                *wide.entry(x.to_string()).or_default() += 1;
            }
        }
    }
    let mut found: Vec<(u64, String)> = Vec::new();
    for (n, ty) in &named {
        let (t, sp) = segments(n);
        for i in 0..t.len() {
            let all: bool = WIDE_TYPES.contains(&ty.as_str());
            let mut vs: Vec<&String> = Vec::new();
            if all {
                vs.extend(
                    wide.iter()
                        .filter(|(_, c)| **c >= SLOT_SUPPORT)
                        .map(|(k, _)| k),
                );
            } else if let Some(m) = slot.get(&(t.len(), i, ty.clone())) {
                vs.extend(
                    m.iter()
                        .filter(|(_, c)| **c >= SLOT_SUPPORT)
                        .map(|(k, _)| k),
                );
            }
            for v in vs {
                if v != t[i] {
                    let c: String = rebuild(&t, &sp, i, v);
                    let h: u32 = murmur3_32(c.as_bytes(), 0);
                    if let Some((fh, wt)) = want.get(&h)
                        && wt == ty
                    {
                        found.push((*fh, c));
                    }
                }
            }
            if t[i].len() <= 4 && !t[i].is_empty() && t[i].bytes().all(|b| b.is_ascii_digit()) {
                let w: usize = t[i].len();
                for d in 0..FLAG_DIGITS {
                    let c: String = rebuild(&t, &sp, i, &format!("{:0w$}", d, w = w));
                    let h: u32 = murmur3_32(c.as_bytes(), 0);
                    if let Some((fh, wt)) = want.get(&h)
                        && wt == ty
                    {
                        found.push((*fh, c));
                    }
                }
            }
        }
    }
    let mut new: usize = 0;
    for (h, c) in found {
        if l.name(h, &c, "sibling") {
            new += 1;
        }
    }
    new
}

pub fn run(l: &FlagList, mn: &mut Mem) -> usize {
    let want: Set<u32> = unresolved(l, mn);
    if want.is_empty() {
        return 0;
    }
    let mut keys: Vec<String> = mn.values().cloned().collect();
    keys.sort();
    keys.dedup();
    let mut found: Vec<(u32, String)> = Vec::new();
    let mut test = |c: &str| {
        let h: u32 = murmur3_32(c.as_bytes(), 0);
        if want.contains(&h) {
            found.push((h, c.to_string()));
        }
    };
    families(&keys, &mut test);
    mutations(&keys, &mut test);
    let mut new: usize = 0;
    for (h, s) in found {
        if mn.insert(h, s).is_none() {
            new += 1;
        }
    }
    new
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::hash_name;

    #[test]
    fn product_walks_every_combination() {
        let pos: Vec<Vec<String>> = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["x".to_string(), "y".to_string(), "z".to_string()],
        ];
        let mut got: Vec<String> = Vec::new();
        product(&pos, &mut |s| got.push(s.to_string()));
        assert_eq!(got.len(), 6);
        assert_eq!(got[0], "a_x");
        assert_eq!(got[5], "b_z");
    }

    #[test]
    fn mutation_reaches_a_neighbour_number() {
        let keys: Vec<String> = vec!["Dm_Kokiri_0030".to_string()];
        let want: u32 = hash_name("Dm_Kokiri_0045");
        let mut hit: bool = false;
        mutations(&keys, &mut |c| {
            if hash_name(c) == want {
                hit = true;
            }
        });
        assert!(hit);
    }

    #[test]
    fn segments_keep_separators() {
        let (t, sp) = segments("Step_SageOf.Light_01");
        assert_eq!(t, vec!["Step", "Sage", "Of", "Light", "01"]);
        assert_eq!(rebuild(&t, &sp, 3, "Fire"), "Step_SageOf.Fire_01");
    }

    #[test]
    fn siblings_swap_a_token_for_the_same_type() {
        use crate::flags::Flag;
        use crate::json::Json;
        let mk = |n: &str, ty: &str| Flag {
            hash: hash_name(n) as u64,
            ty: ty.to_string(),
            def: Json::Null,
            reset: 0,
            save: 0,
            extra: None,
            size: None,
            body: Body::Plain,
            name: None,
            how: None,
        };
        let mut v: Vec<Flag> = Vec::new();
        let mut known: Vec<String> = Vec::new();
        for s in ["Forest", "Fire", "Water", "Shadow"] {
            known.push(format!("Step_SageOf{s}"));
            known.push(format!("IsGet.Item_Medal_{s}"));
        }
        for s in [
            "IsGet.Item_Medal_Light",
            "CaptionData.Item_Light",
            "HaveTalked.Light",
        ] {
            known.push(s.to_string());
        }
        for n in &known {
            let ty: &str = if n.starts_with("Step") {
                "Enum"
            } else {
                "Bool"
            };
            v.push(mk(n, ty));
        }
        v.push(mk("Step_SageOfLight", "Enum"));
        let mut l: FlagList = FlagList::from_flags(v);
        for n in &known {
            l.name(hash_name(n) as u64, n, "t");
        }
        assert_eq!(siblings(&mut l), 1);
        assert_eq!(
            l.flags
                .iter()
                .filter(|f| f.how.as_deref() == Some("sibling"))
                .count(),
            1
        );
    }

    #[test]
    fn family_product_fills_a_gap() {
        let keys: Vec<String> = ["Npc_A_C0", "Npc_A_C1", "Npc_B_C0", "Npc_B_C1"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut got: Set<String> = Set::new();
        families(&keys, &mut |c| {
            got.insert(c.to_string());
        });
        assert_eq!(got.len(), 4);
    }
}
