use std::collections::BTreeSet as BS;

use crate::bfevfl::{Bf, Kind};
use crate::error::{Error, Res};
use crate::msbt;
use crate::rng::Rng;
use crate::romfs::Romfs;
use crate::sarc;
use crate::settings::seed::SeedFile;
use crate::slice::data::Data;
use crate::staging::{rewrite, Out};

pub const FILE: &str = "EventFlowMsg/GossipStone.msbt";
pub const FLOW: &str = "Event/EventFlow/GossipStone.bfevfl.zs";
pub const GATES: [&str; 2] = ["Event112", "Event25"];
pub const LABEL: &str = "GOSSIP_";
pub const COMMON: &str = "RumorCommon";
pub const FIRST: &str = "{0.3:0100}{0.2:5000}{3.0:0f0000}";
pub const NEXT: &str = "{0.2:5000}{0.3:0100}";
pub const PAGE: &str = "{0.4}";
pub const WIDTH: usize = 56;
pub const LINES: usize = 2;
pub const PAGES: usize = 3;
pub const SKIP: [&str; 5] = ["SmallKey", "BossKey", "GanonBossKey", "Map", "Compass"];
pub const MINOR: [&str; 4] = ["Piece of Heart", "Piece of Heart (Out of Logic)", "Piece of Heart (Treasure Chest Game)", "Heart Container"];

const PREFIXES: [(&str, &str); 22] = [
    ("KF ", "Kokiri Forest"),
    ("LW ", "the Lost Woods"),
    ("SFM ", "the Sacred Forest Meadow"),
    ("HF ", "Hyrule Field"),
    ("LH ", "Lake Hylia"),
    ("GV ", "Gerudo Valley"),
    ("GF ", "the Gerudo Fortress"),
    ("Wasteland ", "the Haunted Wasteland"),
    ("Colossus ", "the Desert Colossus"),
    ("DC ", "the Desert Colossus"),
    ("Market ", "the Market"),
    ("HC ", "Hyrule Castle"),
    ("Kak ", "Kakariko Village"),
    ("Graveyard ", "the Graveyard"),
    ("DMT ", "Death Mountain"),
    ("DMC ", "Death Mountain Crater"),
    ("GC ", "Goron City"),
    ("ZR ", "Zora's River"),
    ("ZD ", "Zora's Domain"),
    ("ZF ", "Zora's Fountain"),
    ("LLR ", "Lon Lon Ranch"),
    ("ToT ", "the Temple of Time"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hint {
    pub item: String,
    pub place: String,
}

const THE: [&str; 16] = [
    "Lost Woods",
    "Market",
    "Graveyard",
    "Deku Tree",
    "Ganon's Castle",
    "Forest Temple",
    "Fire Temple",
    "Water Temple",
    "Shadow Temple",
    "Spirit Temple",
    "Bottom of the Well",
    "Ice Cavern",
    "Gerudo Training Ground",
    "Haunted Wasteland",
    "Desert Colossus",
    "Sacred Forest Meadow",
];

fn place_name(a: &str) -> String {
    if THE.contains(&a) {
        format!("the {a}")
    } else if a == "Outside Ganon's Castle" {
        "the area outside Ganon's Castle".to_string()
    } else {
        a.to_string()
    }
}

pub fn area_of(d: &Data, loc: &str) -> Option<String> {
    if let Some(l) = d.cat.by_name(loc) {
        return Some(place_name(&l.area));
    }
    PREFIXES.iter().find(|(p, _)| loc.starts_with(p)).map(|(_, n)| n.to_string())
}

pub fn item_text(name: &str) -> String {
    if name.starts_with("Progressive ") {
        format!("a {name}")
    } else {
        format!("the {name}")
    }
}

pub fn candidates(d: &Data, seed: &SeedFile) -> Vec<Hint> {
    let mut seen: BS<(String, String)> = BS::new();
    let mut out: Vec<Hint> = Vec::new();
    for (loc, item) in &seed.placements {
        let Some(i) = d.item(item) else {
            continue;
        };
        if !i.adv || SKIP.contains(&i.kind.as_str()) || MINOR.contains(&item.as_str()) {
            continue;
        }
        let Some(place) = area_of(d, loc) else {
            continue;
        };
        if seen.insert((item.clone(), place.clone())) {
            out.push(Hint { item: item_text(item), place });
        }
    }
    out
}

pub fn pick(d: &Data, seed: &SeedFile, slots: usize) -> Vec<Hint> {
    let mut all: Vec<Hint> = candidates(d, seed);
    if all.is_empty() || slots == 0 {
        return Vec::new();
    }
    let mut rng: Rng = Rng::from_text(&format!("{}/hints", seed.seed));
    rng.shuffle(&mut all);
    (0..slots).map(|i| all[i % all.len()].clone()).collect()
}

pub fn sentence(h: &Hint) -> String {
    format!("They say that {} lies in {}.", h.item, h.place)
}

pub fn wrap(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur: String = String::new();
    for w in text.split_whitespace() {
        if !cur.is_empty() && cur.len() + 1 + w.len() > WIDTH {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(w);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

pub fn markup(text: &str) -> Res<String> {
    if text.contains(['{', '}', '\\']) {
        return Err(Error::Format("hints: a hint holds a markup character"));
    }
    let lines: Vec<String> = wrap(text);
    let pages: Vec<String> = lines.chunks(LINES).map(|c| c.join("\n")).collect();
    if pages.len() > PAGES {
        return Err(Error::Format("hints: a hint needs more pages than the message box allows"));
    }
    let mut out: String = String::new();
    for (i, p) in pages.iter().enumerate() {
        out.push_str(if i == 0 { FIRST } else { NEXT });
        out.push_str(p);
        if i + 1 < pages.len() {
            out.push_str(PAGE);
        }
    }
    Ok(out)
}

pub fn labels(m: &msbt::Msbt) -> Vec<String> {
    let mut v: Vec<String> = m
        .names()
        .into_iter()
        .map(|x| x.0.to_string())
        .filter(|n| n.starts_with(LABEL) && !n.contains(COMMON))
        .collect();
    v.sort();
    v
}

pub fn apply(raw: &[u8], hints: &[Hint]) -> Res<Vec<u8>> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let e: &sarc::Entry = sarc::get(&es, FILE).ok_or(Error::Format("hints: the language archive has no gossip stone text"))?;
    let mut m: msbt::Msbt = msbt::parse(&e.data)?;
    let ls: Vec<String> = labels(&m);
    if ls.is_empty() {
        return Err(Error::Format("hints: no gossip stone labels found"));
    }
    for (l, h) in ls.iter().zip(hints.iter().cycle()) {
        m.set_markup(l, &markup(&sentence(h))?)?;
    }
    sarc::set(&mut es, FILE, m.write()?);
    sarc::write(&es)
}

pub fn unmask(raw: &[u8]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let mut done: usize = 0;
    for f in b.flows.iter_mut() {
        for e in f.events.iter_mut() {
            if !GATES.contains(&e.name.as_str()) {
                continue;
            }
            if let Kind::Switch { cases, .. } = &mut e.kind {
                let to: u16 = cases.iter().find(|c| c.val == 0).map(|c| c.ev).ok_or(Error::Format("hints: a mask check has no true case"))?;
                for c in cases.iter_mut() {
                    c.ev = to;
                }
                done += 1;
            }
        }
    }
    if done != GATES.len() {
        return Err(Error::Format("hints: the mask checks of the gossip stone flow were not found, the game files differ"));
    }
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("hints: the written flow does not parse back to the edited model"));
    }
    Ok(out)
}

pub fn languages(rf: &Romfs) -> Res<Vec<String>> {
    let mut v: Vec<String> = rf.list("Mals")?.into_iter().filter(|f| f.ends_with(".Product.110.sarc.zs")).collect();
    v.sort();
    Ok(v)
}

pub fn stage(rf: &mut Romfs, d: &Data, seed: &SeedFile, on: bool, keys: &[&'static str]) -> Res<Vec<Out>> {
    let hints: Vec<Hint> = if on { pick(d, seed, 40) } else { Vec::new() };
    if hints.is_empty() && keys.is_empty() {
        return Ok(Vec::new());
    }
    let mut outs: Vec<Out> = Vec::new();
    if !hints.is_empty() {
        outs.push(rewrite(rf, FLOW, unmask)?);
    }
    let counted: std::collections::BTreeSet<&str> = keys.iter().copied().collect();
    for f in languages(rf)? {
        let english: bool = crate::dungeon_text::is_english(&f);
        outs.push(rewrite(rf, &f, |raw| {
            let mut b: Vec<u8> = raw.to_vec();
            if !hints.is_empty() {
                b = apply(&b, &hints)?;
            }
            if !keys.is_empty() {
                b = crate::dungeon_text::apply(&b, keys, &counted, english)?;
            }
            Ok(b)
        })?);
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_hint_is_one_page_with_the_game_prefix() {
        let h: Hint = Hint { item: "the Hookshot".to_string(), place: "Goron City".to_string() };
        let m: String = markup(&sentence(&h)).unwrap();
        assert!(m.starts_with(FIRST));
        assert!(m.ends_with("They say that the Hookshot lies in Goron City."));
        assert!(!m.contains(PAGE));
    }

    #[test]
    fn a_long_hint_breaks_into_lines_and_pages() {
        let h: Hint = Hint {
            item: "a Progressive Strength Upgrade".to_string(),
            place: "the Haunted Wasteland near the Desert Colossus".to_string(),
        };
        let t: String = sentence(&h);
        let lines: Vec<String> = wrap(&t);
        assert!(lines.iter().all(|l| l.len() <= WIDTH));
        let m: String = markup(&t).unwrap();
        assert_eq!(m.matches('\n').count() + m.matches(PAGE).count() + 1, lines.len());
    }

    #[test]
    fn markup_characters_are_refused() {
        assert!(markup("a {0.4} b").is_err());
        let long: String = "word ".repeat(120);
        assert!(markup(&long).is_err());
    }

    #[test]
    fn the_item_article_follows_the_progressive_name() {
        assert_eq!(item_text("Hookshot"), "the Hookshot");
        assert_eq!(item_text("Progressive Hookshot"), "a Progressive Hookshot");
    }
}
