use std::collections::BTreeMap as BM;

use crate::byml::{Byml, Node};
use crate::entrances::{self as en, Ent, Scan, St, StartIdx};
use crate::error::{Error, Res};
use crate::placement::hash_of;
use crate::romfs::Romfs;

pub const SWAP_KEYS: [&str; 4] = [
    "NextBancParamNameAdult",
    "NextBancParamNameChild",
    "NextStartPosName",
    "NextEventName",
];

type Vals = Vec<(&'static str, Option<Node>)>;

pub struct FileEdit {
    pub file: String,
    pub old_len: usize,
    pub bytes: Vec<u8>,
    pub changed: Vec<String>,
    pub bytes_differing: usize,
}

fn msg(s: String) -> Error {
    Error::Msg(s)
}

pub fn select(s: &Scan, sel: &str) -> Res<Vec<usize>> {
    if let Ok(h) = sel.parse::<u64>() {
        let v: Vec<usize> = (0..s.ents.len()).filter(|i| s.ents[*i].hash == h).collect();
        if v.is_empty() {
            return Err(msg(format!("no entrance with hash {sel}")));
        }
        return Ok(v);
    }
    let Some((m, n)) = sel.split_once('/') else {
        return Err(msg(format!("selector '{sel}' is neither a hash nor Map/Name")));
    };
    let v: Vec<usize> = (0..s.ents.len())
        .filter(|i| en::base(&s.ents[*i].map) == m && s.ents[*i].name == n)
        .collect();
    if v.is_empty() {
        return Err(msg(format!("no entrance named '{n}' in map '{m}'")));
    }
    Ok(v)
}

fn key(e: &Ent) -> (String, bool) {
    (e.age.clone(), en::is_alt(&e.map))
}

pub fn pairs(s: &Scan, a: &str, b: &str) -> Res<Vec<(usize, usize)>> {
    let ga: Vec<usize> = select(s, a)?;
    let gb: Vec<usize> = select(s, b)?;
    if ga.iter().any(|i| gb.contains(i)) {
        return Err(msg("both selectors name the same entrance".to_string()));
    }
    let by_hash: bool = a.parse::<u64>().is_ok() && b.parse::<u64>().is_ok();
    if by_hash {
        return Ok(vec![(ga[0], gb[0])]);
    }
    if ga.len() != gb.len() {
        return Err(msg(format!(
            "'{a}' matches {} actor(s) and '{b}' matches {}, they cannot be paired one to one, pass hashes instead",
            ga.len(),
            gb.len()
        )));
    }
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut used: Vec<usize> = Vec::new();
    for i in &ga {
        let k: (String, bool) = key(&s.ents[*i]);
        let hit: Vec<usize> = gb
            .iter()
            .copied()
            .filter(|j| key(&s.ents[*j]) == k && !used.contains(j))
            .collect();
        if hit.len() != 1 {
            return Err(msg(format!(
                "entrance {} in {} has {} partner(s) of the same age in '{b}', pass hashes instead",
                s.ents[*i].hash,
                s.ents[*i].map,
                hit.len()
            )));
        }
        used.push(hit[0]);
        out.push((*i, hit[0]));
    }
    Ok(out)
}

pub fn check(s: &Scan, ix: &StartIdx, i: usize, j: usize, force: bool) -> Res<Vec<String>> {
    let mut warn: Vec<String> = Vec::new();
    let mut bad: Vec<String> = Vec::new();
    for (a, b) in [(i, j), (j, i)] {
        let ea: &Ent = &s.ents[a];
        let eb: &Ent = &s.ents[b];
        let tag: String = format!("{} {} ({})", ea.map, ea.name, ea.hash);
        if !ea.init_ev.is_empty() || !ea.init_flag.is_empty() || !ea.other.is_empty() {
            let ks: Vec<&str> = ea.other.iter().map(|k| k.0.as_str()).collect();
            warn.push(format!(
                "{tag} carries story or special fields (init event '{}', init flag '{}', extra {:?})",
                ea.init_ev, ea.init_flag, ks
            ));
        }
        for x in en::ages_of(&ea.age) {
            let p: &str = en::field(eb, x);
            let st: St = en::resolve(s, ix, p, &eb.start);
            if st != St::Ok {
                bad.push(format!(
                    "{tag} would send the {x} player to {p}/{} which is {}",
                    eb.start,
                    st.name()
                ));
            }
        }
    }
    if !force && !warn.is_empty() {
        bad.extend(warn.iter().map(|w| format!("refused: {w}, pass force to override")));
    }
    if !bad.is_empty() && !force {
        return Err(msg(bad.join("\n")));
    }
    let mut notes: Vec<String> = warn.into_iter().chain(bad).collect();
    for k in [i, j] {
        let e: &Ent = &s.ents[k];
        for a in &s.ancs {
            if en::stem(&a.file) != e.map {
                continue;
            }
            let hit: bool = a.strs.iter().any(|x| {
                x.0 == "NextBancParamName" && (x.1 == e.adult || x.1 == e.child) && !x.1.is_empty()
            });
            if hit {
                notes.push(format!(
                    "standalone preload anchor {} in {} names the old destination of {} and is left as it is",
                    a.hash, e.map, e.name
                ));
            }
        }
    }
    Ok(notes)
}

fn vals(a: &Node) -> Vals {
    let d: Option<&Node> = a.get("Dynamic");
    SWAP_KEYS.iter().map(|k| (*k, d.and_then(|x| x.get(k)).cloned())).collect()
}

fn put(a: &mut Node, v: &Vals) {
    if let Some(d) = a.get_mut("Dynamic") {
        for (k, x) in v {
            match x {
                Some(n) => {
                    d.set(k, n.clone());
                }
                None => {
                    d.remove(k);
                }
            }
        }
    }
}

fn find(y: &Byml, h: u64) -> Option<&Node> {
    y.root.get("Actors")?.as_arr()?.iter().find(|a| hash_of(a) == Some(h))
}

fn find_mut(y: &mut Byml, h: u64) -> Option<&mut Node> {
    match y.root.get_mut("Actors")? {
        Node::Arr(l) => l.iter_mut().find(|a| hash_of(a) == Some(h)),
        _ => None,
    }
}

fn leaf(n: &Node) -> String {
    match n {
        Node::Str(s) => s.clone(),
        o => format!("{:?}", o),
    }
}

pub fn diff(a: &Node, b: &Node, path: &str, out: &mut Vec<String>) {
    match (a, b) {
        (Node::Map(x), Node::Map(y)) => {
            let mut ks: Vec<&String> = x.iter().map(|e| &e.0).collect();
            for e in y {
                if !ks.contains(&&e.0) {
                    ks.push(&e.0);
                }
            }
            for k in ks {
                let p: String = format!("{path}.{k}");
                match (a.get(k), b.get(k)) {
                    (Some(u), Some(v)) => diff(u, v, &p, out),
                    (Some(_), None) => out.push(format!("{p}: removed")),
                    (None, Some(_)) => out.push(format!("{p}: added")),
                    (None, None) => {}
                }
            }
        }
        (Node::Arr(x), Node::Arr(y)) => {
            if x.len() != y.len() {
                out.push(format!("{path}: length {} -> {}", x.len(), y.len()));
                return;
            }
            for (i, (u, v)) in x.iter().zip(y.iter()).enumerate() {
                diff(u, v, &format!("{path}[{i}]"), out);
            }
        }
        (u, v) => {
            if u != v {
                out.push(format!("{path}: {} -> {}", leaf(u), leaf(v)));
            }
        }
    }
}

fn actor_tag(y: &Byml, path: &str) -> String {
    let Some(rest) = path.strip_prefix(".Actors[") else {
        return path.to_string();
    };
    let Some((i, tail)) = rest.split_once(']') else {
        return path.to_string();
    };
    let idx: usize = i.parse().unwrap_or(usize::MAX);
    let h: u64 = y
        .root
        .get("Actors")
        .and_then(|a| a.as_arr())
        .and_then(|a| a.get(idx))
        .and_then(hash_of)
        .unwrap_or(0);
    format!("actor {h}{tail}")
}

pub fn plan(rf: &mut Romfs, s: &Scan, pr: &[(usize, usize)]) -> Res<Vec<FileEdit>> {
    let mut files: BM<String, (Vec<u8>, Byml)> = BM::new();
    let mut want: Vec<(String, u64, Vals)> = Vec::new();
    for (i, j) in pr {
        for (a, b) in [(*i, *j), (*j, *i)] {
            let ea: &Ent = &s.ents[a];
            let eb: &Ent = &s.ents[b];
            for e in [ea, eb] {
                if !files.contains_key(&e.file) {
                    let raw: Vec<u8> = rf.read_zs(&e.file)?;
                    let y: Byml = Byml::parse(&raw)?;
                    files.insert(e.file.clone(), (raw, y));
                }
            }
            let src: &Byml = &files[&eb.file].1;
            let n: &Node = find(src, eb.hash).ok_or_else(|| msg(format!("actor {} vanished", eb.hash)))?;
            want.push((ea.file.clone(), ea.hash, vals(n)));
        }
    }
    let mut edits: Vec<FileEdit> = Vec::new();
    let mut new: BM<String, Byml> = BM::new();
    for (f, (_, y)) in &files {
        new.insert(f.clone(), y.clone());
    }
    for (f, h, v) in &want {
        let y: &mut Byml = new.get_mut(f).ok_or(Error::Format("file not loaded"))?;
        let n: &mut Node = find_mut(y, *h).ok_or_else(|| msg(format!("actor {h} vanished")))?;
        put(n, v);
    }
    for (f, y) in &new {
        let (raw, old) = &files[f];
        let bytes: Vec<u8> = y.write()?;
        let back: Byml = Byml::parse(&bytes)?;
        if back != *y {
            return Err(msg(format!("{f}: rewritten file does not parse to the same tree")));
        }
        let mut d: Vec<String> = Vec::new();
        diff(&old.root, &back.root, "", &mut d);
        let mut changed: Vec<String> = Vec::new();
        for p in &d {
            let ok: bool = SWAP_KEYS.iter().any(|k| p.split(": ").next().is_some_and(|q| q.ends_with(&format!("Dynamic.{k}"))));
            if !ok {
                return Err(msg(format!("{f}: unexpected change {p}")));
            }
            let (pp, rest) = p.split_once(": ").unwrap_or((p.as_str(), ""));
            changed.push(format!("{}: {}", actor_tag(old, pp), rest));
        }
        let n: usize = raw.iter().zip(bytes.iter()).filter(|(a, b)| a != b).count()
            + raw.len().abs_diff(bytes.len());
        edits.push(FileEdit {
            file: f.clone(),
            old_len: raw.len(),
            bytes,
            changed,
            bytes_differing: n,
        });
    }
    Ok(edits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(h: u64, a: &str, st: &str) -> Node {
        Node::Map(vec![
            (
                "Dynamic".to_string(),
                Node::Map(vec![
                    ("NextBancParamNameAdult".to_string(), Node::Str(a.to_string())),
                    ("NextStartPosName".to_string(), Node::Str(st.to_string())),
                ]),
            ),
            ("Hash".to_string(), Node::U64(h)),
        ])
    }

    #[test]
    fn put_moves_values_and_removes_missing() {
        let a: Node = actor(1, "X", "sx");
        let mut b: Node = actor(2, "Y", "sy");
        let v: Vals = vals(&a);
        put(&mut b, &v);
        assert_eq!(b.get("Dynamic").and_then(|d| d.get("NextBancParamNameAdult")).and_then(|x| x.as_str()), Some("X"));
        assert!(b.get("Dynamic").and_then(|d| d.get("NextEventName")).is_none());
        assert_eq!(b.get("Hash"), Some(&Node::U64(2)));
    }

    #[test]
    fn diff_reports_only_changed_leaves() {
        let a: Node = actor(1, "X", "sx");
        let b: Node = actor(1, "Y", "sx");
        let mut out: Vec<String> = Vec::new();
        diff(&a, &b, "", &mut out);
        assert_eq!(out.len(), 1);
        assert!(out[0].starts_with(".Dynamic.NextBancParamNameAdult"));
    }

    fn ent(map: &str, name: &str, h: u64) -> Ent {
        Ent {
            file: format!("Banc/X/{map}_Static.bcett.byml.zs"),
            map: map.to_string(),
            age: en::age_of(map).to_string(),
            hash: h,
            name: name.to_string(),
            adult: String::new(),
            child: String::new(),
            start: String::new(),
            first: String::new(),
            next: String::new(),
            init_ev: String::new(),
            init_flag: String::new(),
            other: Vec::new(),
            pos: Vec::new(),
            links: Vec::new(),
        }
    }

    #[test]
    fn pairs_match_by_age() {
        let mut s: Scan = Scan::default();
        s.ents.push(ent("M_Adult", "a", 1));
        s.ents.push(ent("M_Child", "a", 2));
        s.ents.push(ent("N_Adult", "b", 3));
        s.ents.push(ent("N_Child", "b", 4));
        let p: Vec<(usize, usize)> = pairs(&s, "M/a", "N/b").unwrap();
        assert_eq!(p, vec![(0, 2), (1, 3)]);
        assert_eq!(pairs(&s, "1", "4").unwrap(), vec![(0, 3)]);
        assert!(pairs(&s, "M/a", "M/a").is_err());
        s.ents.push(ent("N_Adult", "b", 5));
        assert!(pairs(&s, "M/a", "N/b").is_err());
    }

    #[test]
    fn selector_rejects_garbage() {
        let s: Scan = Scan::default();
        assert!(select(&s, "nothing").is_err());
        assert!(select(&s, "A/b").is_err());
        assert!(select(&s, "12345").is_err());
    }
}
