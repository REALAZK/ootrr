use super::cont::{Cont, Val};
use super::flow::{Event, Flow, Kind};
use super::{Actor, Bf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub event: String,
    pub index: usize,
    pub role: &'static str,
    pub via: String,
    pub param: String,
    pub item: String,
}

fn ident(v: &Val) -> Option<&str> {
    match v {
        Val::Str(s) | Val::Actor(s, _) if !s.is_empty() => Some(s.as_str()),
        _ => None,
    }
}

fn num(c: &Cont) -> Option<i32> {
    match c.get("Num") {
        Some(Val::Int(n)) => Some(*n),
        _ => None,
    }
}

fn target<'a>(f: &'a Flow, e: &'a Event) -> Option<(String, &'static str, &'static [&'static str], &'a Cont)> {
    let c: &Cont = e.kind.params()?;
    match &e.kind {
        Kind::Action { actor, act, .. } => {
            let a: &Actor = f.actors.get(*actor as usize)?;
            let n: &str = a.actions.get(*act as usize)?;
            let via: String = format!("{}.{}", a.name, n);
            match (a.name.as_str(), n) {
                ("PouchSystemActor", "EventTriggerAddOrRemovePouchContent") => {
                    let role: &'static str = match num(c) {
                        Some(x) if x > 0 => "give",
                        Some(x) if x < 0 => "take",
                        _ => "pouch",
                    };
                    Some((via, role, &["Actor"], c))
                }
                ("GameDataActor", "EventTriggerSetGameDataBool") => {
                    let on: bool = matches!(c.get("Value"), Some(Val::Bool(x)) if *x != 0);
                    if on { Some((via, "flag", &["GameDataName"], c)) } else { None }
                }
                _ => None,
            }
        }
        Kind::Sub { flow, entry, .. } => {
            let via: String = format!("{flow}:{entry}");
            let e: &str = entry.as_str();
            if e.starts_with("GetInEvent")
                || matches!(e, "GetPouchContentByActorName" | "DisplayGetByActorName" | "DisplayRutoGetByActorName")
            {
                Some((via, "give", &["ActorName"], c))
            } else if e.starts_with("NpcCommon_HandOver_NpcToPlayer") {
                Some((via, "give", &["ReveiveItem", "ReceiveItem"], c))
            } else if e.starts_with("TakenReward_Common_Event") {
                Some((via, "give", &["TakenRewardActor"], c))
            } else if e.starts_with("SimpleShopRoot") {
                Some((via, "sell", &["ActorName", "DisplayItemActor"], c))
            } else if e.starts_with("SimpleShopSub_Scrub_Business") {
                Some((via, "sell", &["ActorName", "DisplayActor"], c))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_item(s: &str, role: &str) -> bool {
    if role == "flag" {
        return s.starts_with("Item_");
    }
    !s.starts_with("Shop_")
}

pub fn hits(b: &Bf) -> Vec<(String, Vec<Hit>)> {
    let mut out: Vec<(String, Vec<Hit>)> = Vec::new();
    for f in &b.flows {
        let mut v: Vec<Hit> = Vec::new();
        for (i, e) in f.events.iter().enumerate() {
            let Some((via, role, keys, c)) = target(f, e) else { continue };
            for k in keys {
                let Some(s) = c.get(k).and_then(ident) else { continue };
                if is_item(s, role) {
                    v.push(Hit {
                        event: e.name.clone(),
                        index: i,
                        role,
                        via: via.clone(),
                        param: k.to_string(),
                        item: s.to_string(),
                    });
                }
            }
        }
        out.push((f.name.clone(), v));
    }
    out
}

pub fn item_actors(b: &Bf, items: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for f in &b.flows {
        for a in &f.actors {
            if items.contains(&a.name.as_str()) && !out.contains(&a.name) {
                out.push(a.name.clone());
            }
        }
    }
    out
}

pub struct Entry {
    pub file: String,
    pub flow: String,
    pub hits: Vec<Hit>,
    pub actors: Vec<String>,
}

fn esc(s: &str) -> String {
    let mut o: String = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

pub fn entry(file: &str, b: &Bf) -> Option<Entry> {
    let (flow, hits) = hits(b).into_iter().next()?;
    if hits.is_empty() {
        return None;
    }
    let names: Vec<&str> = hits.iter().map(|h| h.item.as_str()).collect();
    let actors: Vec<String> = item_actors(b, &names);
    Some(Entry { file: file.to_string(), flow, hits, actors })
}

pub fn to_json(src: &str, v: &[Entry]) -> String {
    let mut o: String = String::new();
    o.push_str("{\n  \"format\": 1,\n");
    o.push_str(&format!("  \"source\": \"{}\",\n", esc(src)));
    o.push_str("  \"roles\": {\n");
    o.push_str("    \"give\": \"the event hands the item to the player\",\n");
    o.push_str("    \"sell\": \"the event offers the item in a shop\",\n");
    o.push_str("    \"take\": \"the event removes the item from the player\",\n");
    o.push_str("    \"pouch\": \"pouch add or remove with a count that is not a literal\",\n");
    o.push_str("    \"flag\": \"the event sets an Item_ game data flag to true\"\n");
    o.push_str("  },\n  \"flows\": [\n");
    for (i, e) in v.iter().enumerate() {
        o.push_str(&format!(
            "    {{\n      \"file\": \"{}\",\n      \"flow\": \"{}\",\n      \"actors\": [{}],\n      \"events\": [\n",
            esc(&e.file),
            esc(&e.flow),
            e.actors.iter().map(|a| format!("\"{}\"", esc(a))).collect::<Vec<String>>().join(", ")
        ));
        for (j, h) in e.hits.iter().enumerate() {
            o.push_str(&format!(
                "        {{\"event\": \"{}\", \"index\": {}, \"role\": \"{}\", \"via\": \"{}\", \"param\": \"{}\", \"item\": \"{}\"}}{}\n",
                esc(&h.event),
                h.index,
                h.role,
                esc(&h.via),
                esc(&h.param),
                esc(&h.item),
                if j + 1 < e.hits.len() { "," } else { "" }
            ));
        }
        o.push_str(&format!("      ]\n    }}{}\n", if i + 1 < v.len() { "," } else { "" }));
    }
    o.push_str("  ]\n}\n");
    o
}
