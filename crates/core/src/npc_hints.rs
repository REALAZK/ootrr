//! Character hints, the remake's take on the OoTR "misc_hints" option.
//!
//! Each ticked hint rewrites a few lines that a character already says, so
//! the hint is told in their own voice: Dampé's diary, the cursed Skulltula
//! family, the frogs, the Great Fairies, Sheik, Malon, Mido, Saria and the
//! Temple of Time altar. Only message text changes, no flow or actor edit.

use std::collections::BTreeSet as BS;

use crate::error::{Error, Res};
use crate::hints::area_of;
use crate::msbt::{self, Msbt};
use crate::sarc;
use crate::settings::seed::SeedFile;
use crate::slice::data::Data;

pub const SETTING: &str = "misc_hints";

/// Highlight on and off, the same pair the game uses for key words.
pub const HI: &str = "{0.3:0000}";
pub const LO: &str = "{0.3:ffff}";
pub const PAGE: &str = "{0.4}";
/// Dampé writes every diary page in the diary style.
pub const DIARY: &str = "{1.2}";

/// Visible characters per line and lines per box. The game's own talk lines
/// stay under about 50 characters and three lines, so these keep a margin.
pub const WIDTH: usize = 46;
pub const LINES: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Altar,
    Dampe,
    Skulltulas,
    Frogs,
    Fairies,
    Sheik,
    Malon,
    Mido,
    Saria,
}

pub const KINDS: [(Kind, &str, &str); 9] = [
    (Kind::Altar, "altar", "Temple of Time altar"),
    (Kind::Dampe, "dampe_diary", "Dampé's diary"),
    (Kind::Skulltulas, "skulltulas", "House of Skulltula"),
    (Kind::Frogs, "frogs", "Zora's River frogs"),
    (Kind::Fairies, "great_fairies", "Great Fairies"),
    (Kind::Sheik, "sheik", "Sheik in the Temple of Time"),
    (Kind::Malon, "malon", "Malon"),
    (Kind::Mido, "mido", "Mido"),
    (Kind::Saria, "saria", "Saria"),
];

pub fn kind_of(id: &str) -> Option<Kind> {
    KINDS.iter().find(|k| k.1 == id).map(|k| k.0)
}

pub fn id_of(k: Kind) -> &'static str {
    KINDS.iter().find(|x| x.0 == k).map_or("", |x| x.1)
}

/// The medallions in altar order, with the event that holds each one when
/// dungeon rewards are not shuffled.
pub const MEDALLIONS: [(&str, &str); 6] = [
    ("Forest Medallion", "Phantom Ganon"),
    ("Fire Medallion", "Volvagia"),
    ("Water Medallion", "Morpha"),
    ("Shadow Medallion", "Bongo Bongo"),
    ("Spirit Medallion", "Twinrova"),
    ("Light Medallion", "ToT Reward from Rauru"),
];

/// Places that are events rather than catalog spots.
const EVENT_PLACES: [(&str, &str); 10] = [
    ("Queen Gohma", "the Deku Tree"),
    ("King Dodongo", "Dodongo's Cavern"),
    ("Barinade", "Jabu Jabu's Belly"),
    ("Phantom Ganon", "the Forest Temple"),
    ("Volvagia", "the Fire Temple"),
    ("Morpha", "the Water Temple"),
    ("Bongo Bongo", "the Shadow Temple"),
    ("Twinrova", "the Spirit Temple"),
    ("ToT Reward from Rauru", "the Temple of Time"),
    ("Links Pocket", "the Temple of Time"),
];

pub const SKULL_REWARDS: [(&str, &str); 6] = [
    ("Talk_28", "Kak 10 Gold Skulltula Reward"),
    ("Talk_23", "Kak 20 Gold Skulltula Reward"),
    ("Talk_24", "Kak 30 Gold Skulltula Reward"),
    ("Talk_25", "Kak 40 Gold Skulltula Reward"),
    ("Talk_26", "Kak 50 Gold Skulltula Reward"),
    ("Talk_27", "Kak 100 Gold Skulltula Reward"),
];

/// Each fountain asks its own price, so the offer lines with that price
/// belong to one fountain even though they live in the shared Din file.
pub const FAIRIES: [(&str, &str, &str); 6] = [
    ("HC Great Fairy Reward", "0304", "Dm_FairyFountain_0009_01"),
    ("DMT Great Fairy Reward", "0305", "Dm_FairyFountain_0010"),
    ("ZF Great Fairy Reward", "0306", "Dm_FairyFountain_0011"),
    ("DMC Great Fairy Reward", "0307", "Dm_FairyFountain_0012"),
    ("Colossus Great Fairy Reward", "0308", "Dm_FairyFountain_0013"),
    ("OGC Great Fairy Reward", "0309", "Dm_FairyFountain_0014"),
];

pub const MALON_EGG: &str = "HC Malon Egg";
pub const FROG_GAME: &str = "ZR Frogs Ocarina Game";
pub const FROG_RAIN: &str = "ZR Frogs in the Rain";

pub const ALTAR_FILE: &str = "EventFlowMsg/SignboardWood.msbt";
pub const ALTAR_LABEL: &str = "TempleOfTime_000";
pub const DIARY_ADULT: &str = "EventFlowMsg/Diary_Hylia_Dampe_Adult.msbt";
pub const DIARY_CHILD: &str = "EventFlowMsg/Diary_Hylia_Dampe_Child.msbt";
pub const SKULL_FILE: &str = "EventFlowMsg/Npc_SkullDad.msbt";
pub const FROG_FILE: &str = "EventFlowMsg/MiniGame_FrogTenors.msbt";
pub const FAIRY_FILE: &str = "EventFlowMsg/Dm_FairyFountain_Din.msbt";
pub const SHEIK_FILE: &str = "EventFlowMsg/Npc_Hylia_Sheik_Adult.msbt";
pub const MALON_FILE: &str = "EventFlowMsg/Npc_Hylia_Malon_Child.msbt";
pub const MIDO_FILE: &str = "EventFlowMsg/Npc_Kokiri_Mido_Child.msbt";
pub const SARIA_FILE: &str = "EventFlowMsg/Npc_Kokiri_Saria_Child_SacredForestMeadow.msbt";
pub const SARIA_SCENE_FILE: &str = "EventFlowMsg/Dm_MelodyGet_Saria.msbt";
/// The line of the Saria's Song lesson where she says you have a lot to do.
pub const SARIA_SCENE: &str = "Dm_MelodyGet_Saria_0015";

/// Sheik's Temple of Time lines, before and after the Hookshot.
pub const SHEIK_TALKS: [&str; 11] = ["Talk_00", "Talk_01", "Talk_02", "Talk_03", "Talk_100", "Talk_101", "Talk_102", "Talk_103", "Talk_104", "Talk_105", "Talk_106"];
/// Malon's castle lines about her papa and her goodbyes.
pub const MALON_TALKS: [&str; 3] = ["Talk_109", "Talk_99", "Talk_113"];
/// Mido's "bring a sword" lines.
pub const MIDO_TALKS: [&str; 5] = ["Talk_03", "Talk_18", "Talk_19", "Talk_20", "Talk_26"];
/// Saria's greetings in the Sacred Forest Meadow after the lesson, so the
/// hint needs no ocarina or song.
pub const SARIA_TALKS: [&str; 4] = ["Talk_004", "Talk_007", "Talk_024", "Talk_025"];
/// The frog lines that invite a new song, where a rainy tune is suggested.
pub const FROG_INVITES: [&str; 3] = ["Talk_11", "Talk_37", "Talk_38"];

/// How a line is changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// The text is replaced. `$0`, `$1`, ... stand for the value codes
    /// (`{2.1:...}`, like a price) of the original line, in order.
    Set(String),
    /// New pages are added after the original text.
    Append(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub kind: Kind,
    pub file: &'static str,
    pub label: String,
    pub op: Op,
    /// Text put in front of every page this edit writes.
    pub page_prefix: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Opts {
    pub on: BS<Kind>,
}

impl Opts {
    pub fn any(&self) -> bool {
        !self.on.is_empty()
    }

    pub fn has(&self, k: Kind) -> bool {
        self.on.contains(&k)
    }
}

/// Reads the ticked hints. Malon only hints at the Weird Egg while the
/// Kakariko gate is closed, since the open gate skips the egg's story.
pub fn read(s: &crate::settings::Settings, kakariko_open: bool, bad: &mut Vec<String>) -> Opts {
    let ids: Vec<String> = match s.items(SETTING) {
        Ok(v) => v.to_vec(),
        Err(e) => {
            bad.push(e.to_string());
            Vec::new()
        }
    };
    let mut on: BS<Kind> = BS::new();
    for id in &ids {
        match kind_of(id) {
            Some(Kind::Malon) if kakariko_open => {}
            Some(k) => {
                on.insert(k);
            }
            None => bad.push(format!("{SETTING} has an unknown hint {id}")),
        }
    }
    Opts { on }
}

/// Item name as a phrase: "a Progressive Hookshot", "the Lens of Truth",
/// "20 rupees", "a Piece of Heart".
pub fn item_phrase(name: &str) -> String {
    let pretty: String = pretty(&name.replace(" (Treasure Chest Game)", ""));
    let name: &str = pretty.as_str();
    if NO_ARTICLE.contains(&name) {
        return name.to_string();
    }
    if let Some((base, n)) = name.strip_suffix(')').and_then(|x| x.rsplit_once(" (")) {
        if let Ok(k) = n.parse::<u32>() {
            let low: &str = base.trim();
            return match low {
                "Rupee" | "Rupees" if k == 1 => "a single rupee".to_string(),
                "Rupee" | "Rupees" => format!("{k} rupees"),
                "Bombs" | "Arrows" | "Deku Seeds" | "Deku Nuts" | "Deku Sticks" | "Bombchus" => format!("{k} {low}"),
                _ => format!("{low} ({k})"),
            };
        }
        if base == "Small Key" || base == "Boss Key" {
            let what: &str = if base == "Small Key" { "a Small Key" } else { "the Boss Key" };
            return format!("{what} for {}", the_place(n));
        }
        if base == "Map" || base == "Compass" {
            return format!("the {base} of {}", the_place(n));
        }
    }
    const A: [&str; 12] = [
        "Piece of Heart",
        "Heart Container",
        "Bottle",
        "Deku Shield",
        "Hylian Shield",
        "Mirror Shield",
        "Recovery Heart",
        "Magic Bean",
        "Gold Skulltula Token",
        "Ice Trap",
        "Double Defense",
        "Magic Meter",
    ];
    if name.starts_with("Progressive ") || A.iter().any(|a| name.starts_with(a)) {
        format!("a {name}")
    } else if name.starts_with("Rupee") {
        "some rupees".to_string()
    } else {
        format!("the {name}")
    }
}

/// OoTR item names leave out apostrophes.
const PRETTY: [(&str, &str); 13] = [
    ("Dins Fire", "Din's Fire"),
    ("Farores Wind", "Farore's Wind"),
    ("Nayrus Love", "Nayru's Love"),
    ("Zeldas Lullaby", "Zelda's Lullaby"),
    ("Eponas Song", "Epona's Song"),
    ("Sarias Song", "Saria's Song"),
    ("Suns Song", "Sun's Song"),
    ("Rutos Letter", "Ruto's Letter"),
    ("Zeldas Letter", "Zelda's Letter"),
    ("Poachers Saw", "Poacher's Saw"),
    ("Biggoron Sword", "Biggoron's Sword"),
    ("Dodongos Cavern", "Dodongo's Cavern"),
    ("Ganons Castle", "Ganon's Castle"),
];

/// Names a character says without an article.
const NO_ARTICLE: [&str; 9] = ["Din's Fire", "Farore's Wind", "Nayru's Love", "Zelda's Lullaby", "Epona's Song", "Saria's Song", "Sun's Song", "Ruto's Letter", "Zelda's Letter"];

pub fn pretty(name: &str) -> String {
    let mut out: String = name.to_string();
    for (a, b) in PRETTY {
        out = out.replace(a, b);
    }
    out
}

fn the_place(d: &str) -> String {
    const THE: [&str; 9] = [
        "Forest Temple",
        "Fire Temple",
        "Water Temple",
        "Shadow Temple",
        "Spirit Temple",
        "Deku Tree",
        "Bottom of the Well",
        "Ice Cavern",
        "Gerudo Training Ground",
    ];
    let d: String = pretty(d);
    if THE.contains(&d.as_str()) {
        format!("the {d}")
    } else {
        d
    }
}

/// The item at a location: its placement, or the vanilla item when the
/// location is not shuffled.
pub fn item_at(d: &Data, seed: &SeedFile, loc: &str) -> Option<String> {
    seed.placements.get(loc).or_else(|| d.vanilla.get(loc)).cloned()
}

/// Where an item lies: the first placement holding it, else an unshuffled
/// location that holds it in the vanilla game.
pub fn where_is(d: &Data, seed: &SeedFile, item: &str) -> Option<String> {
    if let Some((l, _)) = seed.placements.iter().find(|(_, i)| i.as_str() == item) {
        return Some(l.clone());
    }
    let mut v: Vec<&String> = d.vanilla.iter().filter(|(l, i)| i.as_str() == item && !seed.placements.contains_key(*l)).map(|(l, _)| l).collect();
    v.sort();
    v.first().map(|l| l.to_string())
}

/// A location as a place name a character could say.
pub fn place(d: &Data, loc: &str) -> Option<String> {
    if let Some((_, p)) = EVENT_PLACES.iter().find(|(l, _)| *l == loc) {
        return Some(p.to_string());
    }
    area_of(d, loc)
}

fn hi(s: &str) -> String {
    format!("{HI}{s}{LO}")
}

fn place_of(d: &Data, seed: &SeedFile, item: &str) -> Option<String> {
    where_is(d, seed, item).and_then(|l| place(d, &l))
}

/// All edits for the ticked hints of one seed. A hint whose item or place
/// cannot be told is left out, and the character keeps their usual words.
pub fn edits(d: &Data, seed: &SeedFile, o: &Opts) -> Vec<Edit> {
    let mut out: Vec<Edit> = Vec::new();
    let mut push = |kind: Kind, file: &'static str, label: &str, op: Op, page_prefix: &'static str| {
        out.push(Edit { kind, file, label: label.to_string(), op, page_prefix });
    };
    if o.has(Kind::Altar) {
        let mut lines: Vec<String> = Vec::new();
        for (m, boss) in MEDALLIONS {
            let at: Option<String> = where_is(d, seed, m).or_else(|| Some(boss.to_string()));
            if let Some(p) = at.and_then(|l| place(d, &l)) {
                lines.push(format!("The {} rests in {}.", hi(m), hi(&p)));
            }
        }
        if !lines.is_empty() {
            let mut t: String = "Ye who would awaken the six sages, heed the words carved upon this stone.".to_string();
            for pair in lines.chunks(2) {
                t.push('|');
                t.push_str(&pair.join(" "));
            }
            push(Kind::Altar, ALTAR_FILE, ALTAR_LABEL, Op::Append(t), "");
        }
    }
    if o.has(Kind::Dampe)
        && let Some(p) = place_of(d, seed, "Progressive Hookshot")
    {
        let p: String = hi(&p);
        push(
            Kind::Dampe,
            DIARY_ADULT,
            "Talk_02",
            Op::Set(format!(
                "Month 〇, Day △|Nobody's been by, and I'm bored outta my skull, so I decided to dig {HI}my own grave{LO}.|\
                 Figure it's a fine spot to keep that {HI}treasure{LO} I found the other day.|Thing is, for now I went and stashed it over in {p}.|\
                 As I understand it, this {HI}treasure{LO} is related to the {HI}Sheikah{LO}.|\
                 When you use it, it stretches out real long, and then if you hold on tight, you'll zoom over to where it went!|\
                 I wanna show it off, but I'm too old for a thing like this and feel embarrassed.|So it can stay in {p}. Heh heh..."
            )),
            DIARY,
        );
        push(
            Kind::Dampe,
            DIARY_ADULT,
            "Talk_03",
            Op::Set(format!(
                "Month 〇, Day □|{HI}Ol' Dampé's grave{LO} is dug!|\
                 I made something amazing, if I do say so myself, but it'll be sad if no one notices.|\
                 I did a special something to my tombstone... (That you can only see at {HI}night{LO}... Heh heh!)|\
                 And I gotta remember to fetch the {HI}treasure that shrinks and stretches{LO} ...|It's still sitting in {p}, and I can't go into the tomb without it."
            )),
            DIARY,
        );
        push(
            Kind::Dampe,
            DIARY_CHILD,
            "Talk_03",
            Op::Append(format!(
                "Dug up a real treasure today, one that stretches out real long!|Hid it away in {p} for safekeeping. Nobody'll find it there. Heh heh!"
            )),
            DIARY,
        );
    }
    if o.has(Kind::Skulltulas) {
        for (i, (label, loc)) in SKULL_REWARDS.iter().enumerate() {
            let Some(item) = item_at(d, seed, loc) else {
                continue;
            };
            let n: &str = ["10", "20", "30", "40", "50", "100"][i];
            let whom: &str = match i {
                0 => "you would surely free one of my children from this shocking curse...",
                5 => "I believe you would at last free me from this ghastly curse...",
                _ => "you would surely free another of my children from this terrible curse...",
            };
            push(
                Kind::Skulltulas,
                SKULL_FILE,
                label,
                Op::Set(format!(
                    "If you collect a total of {HI}{n}{LO} tokens, {whom}|And in my gratitude, I would bestow upon you {}.",
                    hi(&item_phrase(&item))
                )),
                "",
            );
        }
    }
    if o.has(Kind::Frogs) {
        if let Some(item) = item_at(d, seed, FROG_GAME) {
            push(
                Kind::Frogs,
                FROG_FILE,
                "Talk_19",
                Op::Set(format!(
                    "Play your ocarina, and we'll hop to the tune!|{HI}Make us jump at the right time{LO} so we can catch insects to fill our bellies.|\
                     Once we're full, we'll give you {}! Ribbit!",
                    hi(&item_phrase(&item))
                )),
                "",
            );
        }
        if let Some(item) = item_at(d, seed, FROG_RAIN) {
            let t: String = format!(
                "Ribbit! Nothing gets us froggier than a good {HI}rainstorm{LO}...|Play us a stormy tune, and we'll give you {}!",
                hi(&item_phrase(&item))
            );
            for l in FROG_INVITES {
                push(Kind::Frogs, FROG_FILE, l, Op::Append(t.clone()), "");
            }
        }
    }
    if o.has(Kind::Fairies) {
        for (loc, price, first) in FAIRIES {
            let Some(item) = item_at(d, seed, loc) else {
                continue;
            };
            let it: String = hi(&item_phrase(&item));
            let rupees: String = format!("{HI}$0 rupees{LO}");
            for (suffix, text) in [
                ("_01", format!("Will you offer me {rupees}? In return, I shall grant you {it}.")),
                ("_02", format!("An offering of {rupees} will suffice. Toss it in the fountain, and {it} shall be yours.")),
                ("_03", format!("Offer me {rupees}, and I shall grant you {it}.")),
            ] {
                push(Kind::Fairies, FAIRY_FILE, &format!("Dm_FairyFountain_{price}{suffix}"), Op::Set(text), "");
            }
            let text: String = if first.ends_with("_01") {
                format!("I believe... {rupees} should do. For that, I shall grant you {it}.")
            } else {
                format!("Well? As an {HI}offering{LO}, why don't you toss... {rupees} into the fountain? In return, I shall grant you {it}.")
            };
            push(Kind::Fairies, FAIRY_FILE, first, Op::Set(text), "");
        }
    }
    if o.has(Kind::Sheik)
        && let Some(p) = place_of(d, seed, "Light Arrows")
    {
        let t: String = format!(
            "One more thing, Link.|The {}, the sacred light that pierces the Demon King's darkness, await you in {}.",
            hi("Light Arrows"),
            hi(&p)
        );
        for l in SHEIK_TALKS {
            push(Kind::Sheik, SHEIK_FILE, l, Op::Append(t.clone()), "");
        }
    }
    // While the egg is still Malon's own gift she hands it over herself.
    if o.has(Kind::Malon)
        && let Some(l) = where_is(d, seed, "Weird Egg").filter(|l| l != MALON_EGG)
        && let Some(p) = place(d, &l)
    {
        let t: String = format!(
            "Oh, and Papa had this {} with him, the kind that hatches into a really loud bird.|But the silly guy dropped it somewhere in {}! Hee hee!",
            hi("weird egg"),
            hi(&p)
        );
        for l in MALON_TALKS {
            push(Kind::Malon, MALON_FILE, l, Op::Append(t.clone()), "");
        }
    }
    if o.has(Kind::Mido)
        && let Some(p) = place_of(d, seed, "Kokiri Sword")
    {
        let t: String = format!(
            "Not that someone like you could ever find the {} anyway.|Everybody knows it's hidden in {}! ...Wait, forget I said that!",
            hi("Kokiri Sword"),
            hi(&p)
        );
        for l in MIDO_TALKS {
            push(Kind::Mido, MIDO_FILE, l, Op::Append(t.clone()), "");
        }
    }
    if o.has(Kind::Saria)
        && let Some(p) = place_of(d, seed, "Magic Meter")
    {
        let t: String = format!(
            "Oh, before I forget! The forest whispered something to me...|It said {} waits for you in {}. Go find it, 'K?",
            hi("magic power"),
            hi(&p)
        );
        for l in SARIA_TALKS {
            push(Kind::Saria, SARIA_FILE, l, Op::Append(t.clone()), "");
        }
        push(Kind::Saria, SARIA_SCENE_FILE, SARIA_SCENE, Op::Append(t), "");
    }
    out
}

/// Number of visible characters, codes excluded.
pub fn visible(s: &str) -> usize {
    let mut n: usize = 0;
    let mut depth: usize = 0;
    for c in s.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => n += 1,
            _ => {}
        }
    }
    n
}

/// Lays out text into boxes: `|` separates pages the author chose, long
/// pages are wrapped at `WIDTH` and split after `LINES` lines.
pub fn layout(text: &str, page_prefix: &str) -> String {
    let mut boxes: Vec<String> = Vec::new();
    for page in text.split('|') {
        let mut lines: Vec<String> = Vec::new();
        let mut cur: String = String::new();
        for w in page.split_whitespace() {
            if !cur.is_empty() && visible(&cur) + 1 + visible(w) > WIDTH {
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
        for c in lines.chunks(LINES) {
            boxes.push(format!("{page_prefix}{}", c.join("\n")));
        }
    }
    boxes.join(PAGE)
}

/// The value codes of a line, like the `{2.1:...}` price of a fairy.
pub fn value_codes(markup: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest: &str = markup;
    while let Some(i) = rest.find("{2.1:") {
        let tail: &str = &rest[i..];
        let Some(j) = tail.find('}') else {
            break;
        };
        out.push(tail[..=j].to_string());
        rest = &tail[j + 1..];
    }
    out
}

fn fill(text: &str, codes: &[String]) -> Res<String> {
    let mut t: String = text.to_string();
    for (i, c) in codes.iter().enumerate() {
        t = t.replace(&format!("${i}"), c);
    }
    if t.contains('$') {
        return Err(Error::Msg(format!("character hints: a line needs a value the game text does not have: {text}")));
    }
    Ok(t)
}

/// The new markup of one line.
pub fn rewrite(old: &str, e: &Edit) -> Res<String> {
    match &e.op {
        Op::Set(t) => {
            let codes: Vec<String> = value_codes(old);
            let lead: String = lead_codes(old);
            fill(&format!("{lead}{}", layout(t, e.page_prefix)), &codes)
        }
        Op::Append(t) => Ok(format!("{old}{PAGE}{}", layout(t, e.page_prefix))),
    }
}

/// The expression and animation codes a line opens with, kept so the
/// character still moves as they speak.
pub fn lead_codes(markup: &str) -> String {
    let mut out: String = String::new();
    let mut rest: &str = markup;
    while rest.starts_with("{3.0:") || rest.starts_with("{4.0:") {
        let Some(j) = rest.find('}') else {
            break;
        };
        out.push_str(&rest[..=j]);
        rest = &rest[j + 1..];
    }
    out
}

/// Applies the edits to one language archive.
pub fn apply(raw: &[u8], es: &[Edit]) -> Res<Vec<u8>> {
    let mut files: Vec<sarc::Entry> = sarc::parse(raw)?;
    let mut names: Vec<&'static str> = es.iter().map(|e| e.file).collect();
    names.sort();
    names.dedup();
    for f in names {
        let entry: &sarc::Entry = sarc::get(&files, f).ok_or_else(|| Error::Msg(format!("character hints: the language archive has no {f}")))?;
        let mut m: Msbt = msbt::parse(&entry.data)?;
        for e in es.iter().filter(|e| e.file == f) {
            let old: String = msbt::to_markup(&m.text(&e.label).map_err(|_| Error::Msg(format!("character hints: {f} has no line {}", e.label)))?);
            m.set_markup(&e.label, &rewrite(&old, e)?)?;
        }
        sarc::set(&mut files, f, m.write()?);
    }
    sarc::write(&files)
}

pub fn note(o: &Opts) -> Option<String> {
    if !o.any() {
        return None;
    }
    let names: Vec<&str> = KINDS.iter().filter(|k| o.has(k.0)).map(|k| k.2).collect();
    Some(format!(
        "Character hints: {}. Their lines are rewritten in every language (the hint itself is in English), experimental and untested in the game.",
        names.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::seed::SeedFile;
    use std::collections::BTreeMap;

    fn seed(p: &[(&str, &str)]) -> SeedFile {
        SeedFile {
            seed: "s".into(),
            settings_string: String::new(),
            game: String::new(),
            generator: String::new(),
            placements: p.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<BTreeMap<_, _>>(),
        }
    }

    fn all() -> Opts {
        Opts { on: KINDS.iter().map(|k| k.0).collect() }
    }

    #[test]
    fn item_phrases_read_naturally() {
        assert_eq!(item_phrase("Progressive Hookshot"), "a Progressive Hookshot");
        assert_eq!(item_phrase("Piece of Heart"), "a Piece of Heart");
        assert_eq!(item_phrase("Rupees (20)"), "20 rupees");
        assert_eq!(item_phrase("Rupee (1)"), "a single rupee");
        assert_eq!(item_phrase("Bombs (5)"), "5 Bombs");
        assert_eq!(item_phrase("Lens of Truth"), "the Lens of Truth");
        assert_eq!(item_phrase("Small Key (Forest Temple)"), "a Small Key for the Forest Temple");
        assert_eq!(item_phrase("Boss Key (Water Temple)"), "the Boss Key for the Water Temple");
        assert_eq!(item_phrase("Farores Wind"), "Farore's Wind");
        assert_eq!(item_phrase("Biggoron Sword"), "the Biggoron's Sword");
        assert_eq!(item_phrase("Rupees (Treasure Chest Game) (50)"), "50 rupees");
        assert_eq!(item_phrase("Small Key (Ganons Castle)"), "a Small Key for Ganon's Castle");
    }

    #[test]
    fn layout_wraps_lines_and_pages_without_counting_codes() {
        let t: String = layout(&format!("{HI}word{LO} {}", "abcdefghij ".repeat(20)), "");
        for page in t.split(PAGE) {
            let lines: Vec<&str> = page.split('\n').collect();
            assert!(lines.len() <= LINES, "{page}");
            assert!(lines.iter().all(|l| visible(l) <= WIDTH), "{page}");
        }
        assert!(t.starts_with("{0.3:0000}word{0.3:ffff} abcdefghij"));
        assert_eq!(layout("a|b", DIARY), "{1.2}a{0.4}{1.2}b");
    }

    #[test]
    fn a_set_line_keeps_its_lead_codes_and_price() {
        let old: &str = "{4.0:0500486170707900}Offer me {0.3:0000}{2.1:1100477265617446616972795f507269636531ffff00} rupees{0.3:ffff}, and I shall grant you a\nnew power.";
        let e: Edit = Edit { kind: Kind::Fairies, file: FAIRY_FILE, label: "x".into(), op: Op::Set("Offer me $0 rupees, and I shall grant you the Hookshot.".into()), page_prefix: "" };
        let n: String = rewrite(old, &e).unwrap();
        assert!(n.starts_with("{4.0:0500486170707900}Offer me {2.1:1100477265617446616972795f507269636531ffff00} rupees"), "{n}");
        assert!(msbt::from_markup(&n).is_ok());
        let e2: Edit = Edit { op: Op::Set("$1".into()), ..e };
        assert!(rewrite(old, &e2).is_err());
    }

    #[test]
    fn appended_pages_follow_the_original() {
        let e: Edit = Edit { kind: Kind::Mido, file: MIDO_FILE, label: "x".into(), op: Op::Append("Hey!".into()), page_prefix: "" };
        assert_eq!(rewrite("Hrmph!", &e).unwrap(), "Hrmph!{0.4}Hey!");
    }

    #[test]
    fn every_hint_is_told_for_a_vanilla_seed() {
        let d: Data = Data::load().unwrap();
        let es: Vec<Edit> = edits(&d, &seed(&[]), &all());
        let kinds: BS<Kind> = es.iter().map(|e| e.kind).collect();
        let mut want: BS<Kind> = all().on;
        want.remove(&Kind::Malon);
        assert_eq!(kinds, want, "missing hints: {:?}", want.difference(&kinds).collect::<Vec<_>>());
        for e in &es {
            let t: &str = match &e.op {
                Op::Set(t) | Op::Append(t) => t,
            };
            assert!(!t.contains("{{") && !t.contains("}}"), "{t}");
            let laid: String = layout(t, e.page_prefix);
            assert!(msbt::from_markup(&laid).is_ok(), "{laid}");
            for page in t.split('|') {
                assert!(!layout(page, "").contains(PAGE), "a page of {} {} needs more than {LINES} lines: {page}", e.file, e.label);
            }
        }
        let altar: &Edit = es.iter().find(|e| e.kind == Kind::Altar).unwrap();
        let Op::Append(t) = &altar.op else { panic!() };
        assert!(t.contains("Forest Medallion{0.3:ffff} rests in {0.3:0000}the Forest Temple"), "{t}");
        assert!(t.contains("Light Medallion{0.3:ffff} rests in {0.3:0000}the Temple of Time"), "{t}");
    }

    #[test]
    fn a_shuffled_item_is_hinted_where_it_was_placed() {
        let d: Data = Data::load().unwrap();
        let s: SeedFile = seed(&[("KF Kokiri Sword Chest", "Bombs (5)"), ("Graveyard Dampe Race Hookshot Chest", "Kokiri Sword"), ("ZR Frogs Ocarina Game", "Light Arrows"), ("HC Malon Egg", "Bombs (5)"), ("LH Sun", "Weird Egg")]);
        let es: Vec<Edit> = edits(&d, &s, &all());
        let mido: &Edit = es.iter().find(|e| e.kind == Kind::Mido).unwrap();
        assert!(matches!(&mido.op, Op::Append(t) if t.contains("the Graveyard")), "{mido:?}");
        let frog: &Edit = es.iter().find(|e| e.label == "Talk_19").unwrap();
        assert!(matches!(&frog.op, Op::Set(t) if t.contains("the Light Arrows")), "{frog:?}");
        let sheik: &Edit = es.iter().find(|e| e.kind == Kind::Sheik).unwrap();
        assert!(matches!(&sheik.op, Op::Append(t) if t.contains("Zora's River")), "{sheik:?}");
        let malon: &Edit = es.iter().find(|e| e.kind == Kind::Malon).unwrap();
        assert!(matches!(&malon.op, Op::Append(t) if t.contains("Lake Hylia")), "{malon:?}");
    }

    #[test]
    fn malon_is_left_out_while_the_kakariko_gate_is_open() {
        let mut s: crate::settings::Settings = crate::settings::Settings::default();
        s.set(SETTING, crate::settings::Value::set(&["malon", "mido"])).unwrap();
        let mut bad: Vec<String> = Vec::new();
        assert_eq!(read(&s, true, &mut bad).on, [Kind::Mido].into_iter().collect());
        assert_eq!(read(&s, false, &mut bad).on, [Kind::Malon, Kind::Mido].into_iter().collect());
        assert!(bad.is_empty(), "{bad:?}");
    }

    #[test]
    fn unticked_hints_change_nothing() {
        let d: Data = Data::load().unwrap();
        assert!(edits(&d, &seed(&[]), &Opts::default()).is_empty());
        let only: Opts = Opts { on: [Kind::Mido].into_iter().collect() };
        assert!(edits(&d, &seed(&[]), &only).iter().all(|e| e.kind == Kind::Mido));
    }
}
