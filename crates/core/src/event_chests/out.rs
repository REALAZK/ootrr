use std::collections::BTreeMap as BM;

use crate::event_chests::catalog::Loc;
use crate::event_chests::judge::{Cons, Verdict};
use crate::event_chests::scan::{folder, Chest};
use crate::event_chests::signals::Signals;
use crate::event_chests::{rate, Event, Rated, Report};
use crate::json::Json;

pub const BATCH: [(&str, &str); 6] = [
    ("Deku Tree Slingshot Chest", "TBox_Rupee_Red"),
    ("KF Midos Top Left Chest", "TBox_Rupee_Red"),
    ("Jabu Jabus Belly Compass Chest", "TBox_Rupee_Red"),
    ("Jabu Jabus Belly Boomerang Chest", "TBox_Rupee_Red"),
    ("Market Treasure Chest Game Room 1 Bottom", "TBox_Rupee_Red"),
    ("Ice Cavern Iron Boots Chest", "TBox_Rupee_Red"),
];

fn strs(v: &[String]) -> Json {
    Json::strs(v.iter().map(|s| s.as_str()))
}

fn pos(c: &Chest) -> Json {
    Json::Arr(c.pos.iter().map(|x| Json::Num((*x as f64 * 100.0).round() / 100.0)).collect())
}

fn cons(c: &Cons) -> Json {
    Json::obj()
        .with("token", c.token.as_str().into())
        .with("file", c.file.as_str().into())
        .with("path", c.path.as_str().into())
        .with("class", c.class.into())
        .with("note", c.note.as_str().into())
}

fn event(e: &Event, n: usize) -> Json {
    let (flow, entry, comp): (&str, &str, &str) = match &e.row {
        Some(r) => (r.stem.as_str(), r.entry.as_str(), r.comp.as_str()),
        None => ("", "", ""),
    };
    let s: &Signals = &e.signals;
    Json::obj()
        .with("event", e.name.as_str().into())
        .with("flow", flow.into())
        .with("entry", entry.into())
        .with("component", comp.into())
        .with("flow_has_110_variant", e.has_110.into())
        .with("chests", (n as i64).into())
        .with("verdict", e.judged.verdict.name().into())
        .with("confidence", e.judged.conf.into())
        .with("why", e.judged.why.as_str().into())
        .with("chest_demo", s.chest_demo.into())
        .with("demos", strs(&s.demos))
        .with("flags_set", strs(&s.sets))
        .with("flags_read", strs(&s.reads))
        .with("challenge_steps", strs(&s.challenge))
        .with("dialog", strs(&s.dialog))
        .with("item_grants", strs(&s.items))
        .with("equips", strs(&s.equips))
        .with("songs", strs(&s.songs))
        .with("guests", strs(&s.guests))
        .with("calls", strs(&s.calls))
        .with("lost_if_stripped", strs(&e.judged.lost))
        .with("stale_if_kept", strs(&e.judged.stale))
        .with("needs", strs(&e.judged.needs))
        .with("missing", strs(&e.reach.missing))
        .with("consumers", Json::Arr(e.cons.iter().map(cons).collect()))
}

fn chest(r: &Report, c: &Chest) -> Json {
    let loc: Option<&Loc> = r.snap.by_hash.get(&c.hash);
    let x0: Option<(&Event, Rated)> = rate(r, c);
    let (v, conf, why): (Verdict, &str, String) = match &x0 {
        Some((_, y)) => (y.verdict, y.conf, y.why.clone()),
        None => (Verdict::KeepVanilla, "guess", "no event analysis".to_string()),
    };
    let mut x: Json = Json::obj()
        .with("hash", c.hash.to_string().into())
        .with("gyaml", c.gyaml.as_str().into())
        .with("folder", folder(c).as_str().into())
        .with("files", strs(&c.files))
        .with("translate", pos(c))
        .with("drop", c.drop.as_str().into())
        .with("event", c.event.as_str().into())
        .with("copies_agree", c.agree.into())
        .with("verdict", v.name().into())
        .with("confidence", conf.into())
        .with("why", why.as_str().into());
    let mut ex: Json = Json::obj();
    for (k, v) in &c.extra {
        ex.put(k, v.as_str().into());
    }
    x.put("dynamic_extra", ex);
    match loc {
        Some(l) => {
            x.put("location", l.name.as_str().into());
            x.put("area", l.area.as_str().into());
            x.put("catalog_conf", l.conf.as_str().into());
            x.put("catalog_item", l.item.as_str().into());
            x.put("vanilla", l.vanilla.as_str().into());
            x.put("slice_state", l.state.as_str().into());
        }
        None => x.put("location", Json::Null),
    }
    x
}

fn hashes(r: &Report, want: Verdict, guess: Option<bool>) -> Json {
    let v: Vec<Json> = r
        .chests
        .iter()
        .filter(|c| match rate(r, c) {
            Some((_, y)) => y.verdict == want && guess.is_none_or(|g| (y.conf == "guess") == g),
            None => false,
        })
        .map(|c| c.hash.to_string().into())
        .collect();
    Json::Arr(v)
}

fn expect(v: Verdict, e: &Event, y: &Rated) -> String {
    match v {
        Verdict::StripSafe => format!(
            "the chest opens with the normal chest demo and gives the new item, and none of [{}] happens",
            e.judged.lost.join(", ")
        ),
        Verdict::KeepVanilla => format!("probe: {}", y.why),
        _ => format!(
            "the chest gives the new item and the event still runs, so [{}] still happens ({})",
            e.signals.demos.iter().chain(e.signals.songs.iter()).chain(e.signals.sets.iter()).cloned().collect::<Vec<String>>().join(", "),
            e.judged.needs.join(" | ")
        ),
    }
}

pub fn batch(r: &Report) -> Json {
    let mut out: Vec<Json> = Vec::new();
    for (name, item) in BATCH {
        let Some(c) = r.chests.iter().find(|c| r.snap.by_hash.get(&c.hash).is_some_and(|l| l.name == name)) else { continue };
        let Some((e, y)) = rate(r, c) else { continue };
        let (v, strip): (Verdict, bool) = (y.verdict, y.verdict == Verdict::StripSafe);
        out.push(
            Json::obj()
                .with("location", name.into())
                .with("area", folder(c).as_str().into())
                .with("hash", c.hash.to_string().into())
                .with("translate", pos(c))
                .with("verdict", v.name().into())
                .with("confidence", y.conf.into())
                .with("new_item", item.into())
                .with("set_drop_flag", if strip { "no-event" } else { "" }.into())
                .with("expect", expect(v, e, &y).into()),
        );
    }
    Json::Arr(out)
}

pub fn to_json(r: &Report) -> Json {
    let mut per: BM<&str, i64> = BM::new();
    for c in &r.chests {
        if let Some((_, y)) = rate(r, c) {
            *per.entry(y.verdict.name()).or_insert(0) += 1;
        }
    }
    let mut counts: Json = Json::obj();
    for (k, n) in &per {
        counts.put(k, (*n).into());
    }
    let mut n_by: BM<&str, usize> = BM::new();
    for c in &r.chests {
        *n_by.entry(c.event.as_str()).or_insert(0) += 1;
    }
    let events: Vec<Json> = r.events.values().map(|e| event(e, n_by.get(e.name.as_str()).copied().unwrap_or(0))).collect();
    let chests: Vec<Json> = r.chests.iter().map(|c| chest(r, c)).collect();
    let unresolved: Vec<Json> = r
        .events
        .values()
        .filter(|e| e.judged.verdict == Verdict::KeepVanilla)
        .map(|e| Json::obj().with("event", e.name.as_str().into()).with("why", e.judged.why.as_str().into()))
        .collect();
    Json::obj()
        .with("format", 1i64.into())
        .with(
            "note",
            "Identifiers only. verdict per chest from the event flow. strip_event lists chest actor hashes whose GetEventName can be removed, keep_event lists hashes that can take a new Drop__DropActor while the event stays."
                .into(),
        )
        .with(
            "snapshot",
            Json::obj().with("locations_json", r.snap.locations.as_str().into()).with("slice_json", r.snap.slice.as_str().into()),
        )
        .with("chest_verdicts", counts)
        .with("strip_event", hashes(r, Verdict::StripSafe, Some(false)))
        .with("strip_event_untested", hashes(r, Verdict::StripSafe, Some(true)))
        .with("keep_event", hashes(r, Verdict::KeepEvent, None))
        .with("strip_but_set_flags", hashes(r, Verdict::StripButSetFlags, None))
        .with("keep_vanilla", hashes(r, Verdict::KeepVanilla, None))
        .with("unresolved", Json::Arr(unresolved))
        .with("test_batch", batch(r))
        .with("events", Json::Arr(events))
        .with("chests", Json::Arr(chests))
}

pub fn summary(r: &Report) -> String {
    let mut per: BM<(&str, &str), usize> = BM::new();
    for c in &r.chests {
        if let Some((_, y)) = rate(r, c) {
            *per.entry((y.verdict.name(), y.conf)).or_insert(0) += 1;
        }
    }
    let mut o: String = format!("chests with an event: {}\nevents: {}\n", r.chests.len(), r.events.len());
    for ((v, c), n) in &per {
        o.push_str(&format!("{v} ({c}): {n}\n"));
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_chests::catalog::Snap;
    use crate::event_chests::judge::Judged;
    use crate::event_chests::resolve::Row;
    use crate::event_chests::signals::Signals;
    use crate::event_chests::walk::Reach;

    fn report() -> Report {
        let c: Chest = Chest {
            hash: 7,
            gyaml: "G".to_string(),
            files: vec!["Banc/Area/f".to_string()],
            pos: vec![1.234, 2.0, 3.0],
            drop: "I".to_string(),
            event: "E".to_string(),
            extra: Vec::new(),
            agree: true,
        };
        let e: Event = Event {
            name: "E".to_string(),
            row: Some(Row { stem: "S".to_string(), entry: "N".to_string(), comp: "C".to_string() }),
            reach: Reach::default(),
            signals: Signals::default(),
            cons: Vec::new(),
            judged: Judged {
                verdict: Verdict::StripSafe,
                conf: "flow_evidence",
                why: "w".to_string(),
                lost: vec!["navi:1".to_string()],
                stale: Vec::new(),
                needs: Vec::new(),
            },
            has_110: false,
        };
        let mut m: BM<String, Event> = BM::new();
        m.insert("E".to_string(), e);
        Report { snap: Snap::default(), chests: vec![c], events: m }
    }

    #[test]
    fn json_lists_stripable_hashes_and_counts() {
        let r: Report = report();
        let j: Json = to_json(&r);
        assert_eq!(j.get("strip_event").and_then(|x| x.as_arr()).map(|a| a.len()), Some(1));
        assert_eq!(j.get("keep_event").and_then(|x| x.as_arr()).map(|a| a.len()), Some(0));
        assert_eq!(j.get("chest_verdicts").and_then(|x| x.get("strip_safe")).and_then(|x| x.as_i64()), Some(1));
        let t: String = j.pretty();
        assert!(t.contains("\"translate\""));
        assert!(summary(&r).contains("strip_safe (flow_evidence): 1"));
    }

    #[test]
    fn repeatable_chests_stay_vanilla() {
        let mut r: Report = report();
        r.chests[0].extra = vec![("IsEnableRepeatOpen".to_string(), "true".to_string())];
        let j: Json = to_json(&r);
        assert_eq!(j.get("keep_vanilla").and_then(|x| x.as_arr()).map(|a| a.len()), Some(1));
        assert_eq!(j.get("strip_event").and_then(|x| x.as_arr()).map(|a| a.len()), Some(0));
    }

    #[test]
    fn disagreeing_copies_stay_vanilla() {
        let mut r: Report = report();
        r.chests[0].agree = false;
        let j: Json = to_json(&r);
        assert_eq!(j.get("keep_vanilla").and_then(|x| x.as_arr()).map(|a| a.len()), Some(1));
    }

    #[test]
    fn batch_names_are_unique() {
        let mut n: Vec<&str> = BATCH.iter().map(|b| b.0).collect();
        n.sort();
        n.dedup();
        assert_eq!(n.len(), BATCH.len());
    }
}








