use super::info::flag_label;
use super::model::Save;
use super::text::show;
use super::val::Val;
use crate::flags::{Flag, FlagList, KEYED, hex};

fn name_of(l: &FlagList, h: u64, ty: &str) -> String {
    match l.by_hash(h) {
        Some(f) => flag_label(f),
        None => hex(h, ty),
    }
}

fn fmt(l: &FlagList, h: u32, v: &Val) -> String {
    match l.by_hash(h as u64) {
        Some(f) => show(f, v),
        None => format!("{v:?}"),
    }
}

pub fn diff(a: &Save, b: &Save, l: &FlagList) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for r in &a.rows {
        if r.hash == 0 {
            continue;
        }
        let Some(q) = b.row(r.hash) else {
            out.push(format!(
                "only in first: {}",
                name_of(l, r.hash as u64, "Bool")
            ));
            continue;
        };
        let (x, y): (Val, Val) = (r.get(), q.get());
        if x != y {
            let f: Option<&Flag> = l.by_hash(r.hash as u64);
            let n: String = f
                .map(flag_label)
                .unwrap_or_else(|| hex(r.hash as u64, "Bool"));
            out.push(format!(
                "{n}: {} -> {}",
                fmt(l, r.hash, &x),
                fmt(l, r.hash, &y)
            ));
        }
    }
    for r in &b.rows {
        if r.hash != 0 && a.row(r.hash).is_none() {
            out.push(format!(
                "only in second: {}",
                name_of(l, r.hash as u64, "Bool")
            ));
        }
    }
    for k in &b.keys {
        if !a.has_key(*k) {
            out.push(format!("key set: {}", name_of(l, *k, KEYED)));
        }
    }
    for k in &a.keys {
        if !b.has_key(*k) {
            out.push(format!("key cleared: {}", name_of(l, *k, KEYED)));
        }
    }
    out
}
