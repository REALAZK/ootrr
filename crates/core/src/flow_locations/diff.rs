use std::fs;
use std::path::{Path as P, PathBuf as PB};

use crate::bfevfl::{Bf, Cont, Event, Kind, Val};
use crate::error::Res;
use crate::flow_locations::build::Change;
use crate::flow_locations::edit::dict_id;
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diff {
    pub file: String,
    pub index: usize,
    pub event: String,
    pub param: String,
    pub old: String,
    pub new: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Size {
    pub file: String,
    pub old: usize,
    pub new: usize,
    pub entry: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct Checked {
    pub files: usize,
    pub diffs: Vec<Diff>,
    pub problems: Vec<String>,
    pub sizes: Vec<Size>,
}

impl Diff {
    pub fn of(c: &Change) -> Diff {
        Diff {
            file: c.file.clone(),
            index: c.index,
            event: c.event.clone(),
            param: c.param.clone(),
            old: c.old.clone(),
            new: c.new.clone(),
        }
    }
}

fn strip(e: &Event) -> Event {
    let mut x: Event = e.clone();
    match &mut x.kind {
        Kind::Action { params, .. }
        | Kind::Switch { params, .. }
        | Kind::Sub { params, .. }
        | Kind::SubSwitch { params, .. } => *params = None,
        _ => {}
    }
    x
}

fn same<T: std::fmt::Debug>(a: &T, b: &T) -> bool {
    format!("{a:?}") == format!("{b:?}")
}

fn names(v: &Val) -> Option<(&str, &str)> {
    match v {
        Val::Str(s) => Some((s, "")),
        Val::Actor(s, t) => Some((s, t)),
        _ => None,
    }
}

fn values(file: &str, i: usize, e: &Event, a: &Cont, b: &Cont, out: &mut Vec<Diff>, bad: &mut Vec<String>) {
    if a.items.len() != b.items.len() {
        bad.push(format!("{file} event {i}: the parameter count changed"));
        return;
    }
    for ((ka, va), (kb, vb)) in a.items.iter().zip(&b.items) {
        if ka != kb {
            bad.push(format!("{file} event {i}: parameter {ka} became {kb}"));
        } else if !same(va, vb) {
            match (names(va), names(vb)) {
                (Some((x, xs)), Some((y, ys))) if xs == ys => out.push(Diff {
                    file: file.to_string(),
                    index: i,
                    event: e.name.clone(),
                    param: ka.clone(),
                    old: x.to_string(),
                    new: y.to_string(),
                }),
                _ => bad.push(format!("{file} event {i}: parameter {ka} changed in more than its name")),
            }
        }
    }
}

pub fn compare(file: &str, a: &Bf, b: &Bf) -> (Vec<Diff>, Vec<String>) {
    let mut out: Vec<Diff> = Vec::new();
    let mut bad: Vec<String> = Vec::new();
    if a.ver != b.ver || a.bom != b.bom || a.shift != b.shift || a.pad != b.pad || a.name != b.name {
        bad.push(format!("{file}: header fields changed"));
    }
    if !same(&a.tls, &b.tls) {
        bad.push(format!("{file}: timelines changed"));
    }
    if a.flows.len() != b.flows.len() {
        bad.push(format!("{file}: flow count changed"));
        return (out, bad);
    }
    for (fa, fb) in a.flows.iter().zip(&b.flows) {
        if fa.name != fb.name || !same(&fa.actors, &fb.actors) || !same(&fa.entries, &fb.entries) {
            bad.push(format!("{file}: flow {} actors or entry points changed", fa.name));
        }
        if fa.events.len() != fb.events.len() {
            bad.push(format!("{file}: event count changed"));
            continue;
        }
        for (i, (ea, eb)) in fa.events.iter().zip(&fb.events).enumerate() {
            if same(ea, eb) {
                continue;
            }
            if !same(&strip(ea), &strip(eb)) {
                bad.push(format!("{file} event {i}: more than parameters changed"));
                continue;
            }
            match (ea.kind.params(), eb.kind.params()) {
                (Some(x), Some(y)) => values(file, i, ea, x, y, &mut out, &mut bad),
                _ => bad.push(format!("{file} event {i}: parameters appeared or vanished")),
            }
        }
    }
    (out, bad)
}

fn flow_files(dir: &P) -> Res<Vec<String>> {
    let mut v: Vec<String> = Vec::new();
    for e in fs::read_dir(dir)? {
        let n: String = e?.file_name().to_string_lossy().to_string();
        if n.ends_with(".bfevfl.zs") {
            v.push(n);
        }
    }
    v.sort();
    Ok(v)
}

fn mod_table(rf: &mut Romfs, root: &P) -> Res<Option<Rst>> {
    let p: PB = root.join("romfs").join(rsizetable::TABLE_PATH);
    if !p.is_file() {
        return Ok(None);
    }
    Ok(Some(Rst::parse(&rf.zs.decompress(&fs::read(p)?)?)?))
}

pub fn verify(romfs: &P, root: &P, want: Option<&[Diff]>) -> Res<Checked> {
    let mut rf: Romfs = Romfs::open(romfs)?;
    let orig_tab: Rst = rsizetable::load(&mut rf)?;
    let new_tab: Option<Rst> = mod_table(&mut rf, root)?;
    let dir: PB = root.join("romfs").join("Event").join("EventFlow");
    let mut c: Checked = Checked::default();
    for n in flow_files(&dir)? {
        let path: String = format!("Event/EventFlow/{n}");
        let z: Vec<u8> = fs::read(dir.join(&n))?;
        let oz: Vec<u8> = rf.read(&path)?;
        if dict_id(&z) != dict_id(&oz) {
            c.problems.push(format!("{n}: the zstd dictionary id differs from the original"));
        }
        let rb: Vec<u8> = rf.zs.decompress(&z)?;
        let ro: Vec<u8> = rf.zs.decompress(&oz)?;
        let b: Bf = Bf::parse(&rb)?;
        let a: Bf = Bf::parse(&ro)?;
        if b.write()? != rb {
            c.problems.push(format!("{n}: the file is not in the canonical layout"));
        }
        let base: String = n.trim_end_matches(".zs").to_string();
        let (d, bad): (Vec<Diff>, Vec<String>) = compare(&base, &a, &b);
        c.diffs.extend(d);
        c.problems.extend(bad);
        let entry: Option<u32> = new_tab.as_ref().unwrap_or(&orig_tab).get(&path);
        if entry.is_some_and(|e| (e as usize) < rb.len()) {
            c.problems.push(format!("{n}: the resource size table entry is smaller than the file"));
        }
        c.sizes.push(Size { file: base, old: ro.len(), new: rb.len(), entry });
        c.files += 1;
    }
    c.diffs.sort();
    if let Some(w) = want {
        let mut w: Vec<Diff> = w.to_vec();
        w.sort();
        for x in &w {
            if !c.diffs.contains(x) {
                c.problems.push(format!("expected edit missing: {} event {} {} {} -> {}", x.file, x.index, x.param, x.old, x.new));
            }
        }
        for x in &c.diffs {
            if !w.contains(x) {
                c.problems.push(format!("unexpected edit: {} event {} {} {} -> {}", x.file, x.index, x.param, x.old, x.new));
            }
        }
    }
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::{Actor, Entry, Flow};

    fn sub(n: &str, item: &str, other: &str) -> Event {
        Event {
            name: n.to_string(),
            kind: Kind::Sub {
                next: 0xFFFF,
                params: Some(Cont {
                    items: vec![
                        ("ActorName".to_string(), Val::Str(item.to_string())),
                        ("Who".to_string(), Val::Actor(other.to_string(), String::new())),
                        ("Num".to_string(), Val::Int(1)),
                    ],
                }),
                flow: "GetPouchContent".to_string(),
                entry: "GetPouchContentByActorName".to_string(),
            },
        }
    }

    fn bf(a: &str, b: &str) -> Bf {
        Bf {
            ver: [0, 4, 0, 0],
            bom: 0xFEFF,
            shift: 3,
            pad: 0,
            name: "T".to_string(),
            flows: vec![Flow {
                name: "T".to_string(),
                actors: vec![Actor {
                    name: "X".to_string(),
                    sub: String::new(),
                    arg: String::new(),
                    actions: Vec::new(),
                    queries: Vec::new(),
                    params: None,
                    ep: 0xFFFF,
                    cut: 1,
                }],
                events: vec![sub("Event0", a, "Npc"), sub("Event1", b, "Npc")],
                entries: vec![Entry { name: "Start".to_string(), subs: vec![0], vars: Vec::new(), main: 0, pad: 0 }],
            }],
            tls: Vec::new(),
        }
    }

    #[test]
    fn a_name_change_is_one_diff() {
        let (d, bad): (Vec<Diff>, Vec<String>) = compare("f", &bf("A", "B"), &bf("A", "C"));
        assert!(bad.is_empty());
        assert_eq!(d.len(), 1);
        assert_eq!((d[0].index, d[0].param.as_str(), d[0].old.as_str(), d[0].new.as_str()), (1, "ActorName", "B", "C"));
    }

    #[test]
    fn identical_models_have_no_diffs() {
        let (d, bad): (Vec<Diff>, Vec<String>) = compare("f", &bf("A", "B"), &bf("A", "B"));
        assert!(d.is_empty() && bad.is_empty());
    }

    #[test]
    fn structural_changes_are_problems() {
        let a: Bf = bf("A", "B");
        let mut b: Bf = bf("A", "B");
        b.flows[0].events.pop();
        assert!(!compare("f", &a, &b).1.is_empty());
        let mut c: Bf = bf("A", "B");
        c.flows[0].events[0] = Event { name: "Event0".to_string(), kind: Kind::Join { next: 0 } };
        assert!(!compare("f", &a, &c).1.is_empty());
        let mut d: Bf = bf("A", "B");
        d.flows[0].actors[0].name = "Y".to_string();
        assert!(!compare("f", &a, &d).1.is_empty());
    }

    #[test]
    fn a_changed_number_is_a_problem_not_a_diff() {
        let a: Bf = bf("A", "B");
        let mut b: Bf = bf("A", "B");
        if let Some(c) = b.flows[0].events[0].kind.params_mut() {
            c.items[2].1 = Val::Int(2);
        }
        let (d, bad): (Vec<Diff>, Vec<String>) = compare("f", &a, &b);
        assert!(d.is_empty());
        assert_eq!(bad.len(), 1);
    }

    #[test]
    fn an_actor_value_rename_is_a_diff() {
        let a: Bf = bf("A", "B");
        let mut b: Bf = bf("A", "B");
        if let Some(c) = b.flows[0].events[1].kind.params_mut() {
            c.items[1].1 = Val::Actor("Other".to_string(), String::new());
        }
        let (d, bad): (Vec<Diff>, Vec<String>) = compare("f", &a, &b);
        assert!(bad.is_empty());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].param, "Who");
    }
}
