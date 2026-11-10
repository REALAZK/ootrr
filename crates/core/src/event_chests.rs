pub mod catalog;
pub mod judge;
pub mod out;
pub mod refs;
pub mod resolve;
pub mod scan;
pub mod signals;
pub mod testmod;
pub mod walk;

use std::collections::{BTreeMap as BM, BTreeSet as BS, HashMap as HM};

use crate::error::Res;
use crate::romfs::Romfs;
use catalog::Snap;
use judge::{Cons, Judged, Verdict};
use resolve::Row;
use scan::Chest;
use signals::Signals;
use walk::Reach;

pub struct Event {
    pub name: String,
    pub row: Option<Row>,
    pub reach: Reach,
    pub signals: Signals,
    pub cons: Vec<Cons>,
    pub judged: Judged,
    pub has_110: bool,
}

pub struct Report {
    pub snap: Snap,
    pub chests: Vec<Chest>,
    pub events: BM<String, Event>,
}

fn tokens_of(name: &str, s: &Signals) -> Vec<String> {
    let mut t: Vec<String> = vec![format!("HavePlayedEvent.{name}")];
    for d in &s.demos {
        t.push(format!("HavePlayedEvent.{d}"));
    }
    for x in &s.sets {
        t.push(judge::flag_of(x).to_string());
    }
    for c in &s.challenge {
        let (n, st): (&str, &str) = c.split_once(':').unwrap_or((c.as_str(), ""));
        t.push(if st == "Complete" { format!("{n}_IsCompleted_Exp") } else { format!("{n}_IsAfter_{st}_Exp") });
    }
    t.sort();
    t.dedup();
    t
}

fn is_own(file: &str, r: &Reach) -> bool {
    r.stems.iter().any(|s| file == format!("Event/EventFlow/{s}.bfevfl.zs") || file == format!("Event/EventFlow/{s}.110.bfevfl.zs"))
}

fn consumers(tokens: &[String], r: &Reach, files: &BM<String, Vec<String>>, uses: &BM<String, Vec<refs::Use>>) -> Vec<Cons> {
    let mut out: Vec<Cons> = Vec::new();
    for t in tokens {
        for f in files.get(t).map(|v| v.as_slice()).unwrap_or(&[]) {
            let class: &str = refs::class_of(f);
            if class == "rsdb" || class == "banc" || is_own(f, r) {
                continue;
            }
            let (c, n): (&str, String) = judge::classify(t, f, "");
            out.push(Cons { token: t.clone(), file: f.clone(), path: String::new(), class: c, note: n });
        }
        for u in uses.get(t).map(|v| v.as_slice()).unwrap_or(&[]) {
            let (c, n): (&str, String) = judge::classify(t, &u.file, &u.path);
            if c != "self" {
                out.push(Cons { token: t.clone(), file: u.file.clone(), path: format!("{}:{}", u.gyaml, u.path), class: c, note: n });
            }
        }
    }
    out
}

fn unresolved(name: &str, why: &str) -> Judged {
    Judged {
        verdict: Verdict::KeepVanilla,
        conf: "guess",
        why: format!("event {name} {why}, so the chest stays vanilla"),
        lost: Vec::new(),
        stale: Vec::new(),
        needs: Vec::new(),
    }
}

pub fn analyze(rf: &mut Romfs, snap: Snap) -> Res<Report> {
    let chests: Vec<Chest> = scan::chests(rf)?;
    let table: HM<String, Row> = resolve::table(rf)?;
    let names: BS<String> = chests.iter().map(|c| c.event.clone()).collect();
    let mut parts: Vec<(String, Option<Row>, Reach, Signals, bool)> = Vec::new();
    {
        let mut l: walk::Loader = walk::Loader::new(rf, &table);
        for n in &names {
            let row: Option<Row> = table.get(n).cloned();
            let (reach, sig, h110): (Reach, Signals, bool) = match &row {
                Some(r) => {
                    let w: Reach = walk::walk(&mut l, &r.stem, &r.entry)?;
                    let s: Signals = signals::of(&w);
                    let h: bool = w.stems.iter().any(|x| l.has_110(x));
                    (w, s, h)
                }
                None => (Reach::default(), Signals::default(), false),
            };
            parts.push((n.clone(), row, reach, sig, h110));
        }
    }
    let mut all: Vec<String> = parts.iter().flat_map(|p| tokens_of(&p.0, &p.3)).collect();
    all.sort();
    all.dedup();
    let files: BM<String, Vec<String>> = refs::scan(rf, &all)?;
    let uses: BM<String, Vec<refs::Use>> = refs::banc_uses(rf, &all)?;
    let mut events: BM<String, Event> = BM::new();
    for (name, row, reach, sig, h110) in parts {
        let cons: Vec<Cons> = consumers(&tokens_of(&name, &sig), &reach, &files, &uses);
        let judged: Judged = match &row {
            None => unresolved(&name, "has no EventSetting row"),
            Some(_) if !reach.missing.is_empty() => unresolved(&name, "reaches a flow or entry that is missing"),
            Some(r) => judge::judge(&name, &format!("{}:{}", r.stem, r.entry), &sig, &cons),
        };
        events.insert(name.clone(), Event { name, row, reach, signals: sig, cons, judged, has_110: h110 });
    }
    Ok(Report { snap, chests, events })
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rated {
    pub verdict: Verdict,
    pub conf: &'static str,
    pub why: String,
}

pub fn repeatable(c: &Chest) -> bool {
    c.extra.iter().any(|(k, v)| k == "IsEnableRepeatOpen" && v == "true")
}

pub fn rate<'a>(r: &'a Report, c: &Chest) -> Option<(&'a Event, Rated)> {
    let e: &Event = r.events.get(&c.event)?;
    let x: Rated = if !c.agree {
        Rated { verdict: Verdict::KeepVanilla, conf: "guess", why: "the copies of this actor differ between the Banc files".to_string() }
    } else if repeatable(c) {
        Rated {
            verdict: Verdict::KeepVanilla,
            conf: "guess",
            why: format!(
                "{}, but the chest has IsEnableRepeatOpen so a randomized item would be granted on every opening, which is untested",
                e.judged.why
            ),
        }
    } else {
        Rated { verdict: e.judged.verdict, conf: e.judged.conf, why: e.judged.why.clone() }
    };
    Some((e, x))
}






