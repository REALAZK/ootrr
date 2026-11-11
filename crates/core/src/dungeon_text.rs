use std::collections::BTreeSet as BS;

use crate::error::{Error, Res};
use crate::msbt::{self, Msbt, Tok};
use crate::sarc;

pub const FILE: &str = "EventFlowMsg/Dm_SY_Get.msbt";
pub const FILE_ID: &str = "EventFlowMsg/Dm_SY_Get";
pub const NAMES: &str = "ActorMsg/PouchContent.msbt";
pub const PLACES: &str = "LocationMsg/Location.msbt";
pub const KEY_NAME: &str = "Item_Dungeon_SmallKey_Name";
pub const LABEL: &str = "OOTRR_SmallKey_";
pub const MARK: &str = "{0.3:0000}";
pub const UNMARK: &str = "{0.3:ffff}";
pub const BARE: [&str; 4] = ["InsideJabuJabuBelly", "DodongoCavern", "InsideGanonCastle", "FinalBattleField"];
pub const CASTLE: &str = "GanonCastle";

pub fn label(d: &str) -> String {
    format!("{LABEL}{d}")
}

pub fn message_id(d: &str) -> String {
    format!("{FILE_ID}:{}", label(d))
}

pub fn place_label(d: &str) -> &str {
    match d {
        "InsideGanonCastle" | "FinalBattleField" => CASTLE,
        _ => d,
    }
}

pub fn is_english(path: &str) -> bool {
    let name: &str = path.rsplit('/').next().unwrap_or(path);
    name.get(2..4) == Some("en")
}

pub fn count_tag(d: &str) -> Res<String> {
    let name: String = format!("DungeonInfo.{d}.SmallKeyNum");
    let n: u16 = u16::try_from(name.len()).map_err(|_| Error::Format("dungeon text: flag name too long"))?;
    let mut hex: String = String::new();
    for b in n.to_le_bytes().into_iter().chain(name.bytes()).chain([0xFF, 0xFF, 0x00]) {
        hex.push_str(&format!("{b:02x}"));
    }
    Ok(format!("{{2.1:{hex}}}"))
}

pub fn name(m: &Msbt, label: &str) -> Res<Vec<Tok>> {
    let t: Vec<Tok> = m.text(label).map_err(|_| Error::Msg(format!("the language archive has no text {label}")))?;
    if t.is_empty() {
        return Err(Error::Msg(format!("the text {label} is empty")));
    }
    Ok(t)
}

fn lit(out: &mut Vec<Tok>, s: &str) -> Res<()> {
    out.extend(msbt::from_markup(s)?);
    Ok(())
}

fn marked(out: &mut Vec<Tok>, t: &[Tok]) -> Res<()> {
    lit(out, MARK)?;
    out.extend(t.iter().cloned());
    lit(out, UNMARK)
}

pub fn compose(d: &str, item: &[Tok], place: &[Tok], english: bool, count: bool) -> Res<Vec<Tok>> {
    let mut out: Vec<Tok> = Vec::new();
    if english {
        lit(&mut out, "You got a ")?;
        marked(&mut out, item)?;
        lit(&mut out, if BARE.contains(&d) { " for " } else { " for the " })?;
        marked(&mut out, place)?;
        lit(&mut out, "!")?;
        if count {
            lit(&mut out, &format!("\nSmall Keys held for that place: {MARK}{}{UNMARK}", count_tag(d)?))?;
        }
        return Ok(out);
    }
    marked(&mut out, item)?;
    lit(&mut out, "\n")?;
    marked(&mut out, place)?;
    if count {
        lit(&mut out, &format!(": {}", count_tag(d)?))?;
    }
    Ok(out)
}

fn file(es: &[sarc::Entry], name: &str) -> Res<Msbt> {
    let e: &sarc::Entry = sarc::get(es, name).ok_or_else(|| Error::Msg(format!("the language archive has no {name}")))?;
    msbt::parse(&e.data)
}

pub fn apply(raw: &[u8], dungeons: &[&str], counted: &BS<&str>, english: bool) -> Res<Vec<u8>> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let names: Msbt = file(&es, NAMES)?;
    let places: Msbt = file(&es, PLACES)?;
    let item: Vec<Tok> = name(&names, KEY_NAME)?;
    let mut m: Msbt = file(&es, FILE)?;
    for d in dungeons {
        let place: Vec<Tok> = name(&places, place_label(d))?;
        m.put_tokens(&label(d), &compose(d, &item, &place, english, counted.contains(d))?)?;
    }
    sarc::set(&mut es, FILE, m.write()?);
    sarc::write(&es)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msbt::{label_group, Label, Sec};

    fn book(pairs: &[(&str, &str)]) -> Vec<u8> {
        let groups: usize = 2;
        let mut g: Vec<Vec<Label>> = vec![Vec::new(); groups];
        let mut t: Vec<Vec<u8>> = Vec::new();
        for (i, (k, v)) in pairs.iter().enumerate() {
            g[label_group(k, groups)].push(Label { name: k.to_string(), idx: i as u32 });
            t.push(msbt::encode(&msbt::from_markup(v).unwrap(), false).unwrap());
        }
        let m: Msbt = Msbt { be: false, unk_a: [0, 0], enc: 0, ver: 3, unk_b: [0, 0], pad: [0; 10], secs: vec![Sec::Lbl(g), Sec::Txt(t)] };
        m.write().unwrap()
    }

    fn archive() -> Vec<u8> {
        let es: Vec<sarc::Entry> = vec![
            sarc::Entry { name: NAMES.to_string(), data: book(&[(KEY_NAME, "Small Key")]) },
            sarc::Entry { name: PLACES.to_string(), data: book(&[("ForestTemple", "Forest Temple"), ("InsideJabuJabuBelly", "Lord Jabu-Jabu's Belly"), (CASTLE, "Ganondorf's Castle")]) },
            sarc::Entry { name: FILE.to_string(), data: book(&[("GetMessage", "{2.23:0100ffff}"), ("GetCaption", "x")]) },
        ];
        sarc::write(&es).unwrap()
    }

    fn read(raw: &[u8], l: &str) -> String {
        let es: Vec<sarc::Entry> = sarc::parse(raw).unwrap();
        let m: Msbt = file(&es, FILE).unwrap();
        msbt::to_markup(&m.text(l).unwrap())
    }

    fn line(d: &str, item: &str, place: &str, english: bool, count: bool) -> String {
        let i: Vec<Tok> = msbt::from_markup(item).unwrap();
        let p: Vec<Tok> = msbt::from_markup(place).unwrap();
        msbt::to_markup(&compose(d, &i, &p, english, count).unwrap())
    }

    #[test]
    fn the_english_line_names_the_item_and_the_dungeon() {
        let s: String = line("ForestTemple", "Small Key", "Forest Temple", true, false);
        assert_eq!(s, "You got a {0.3:0000}Small Key{0.3:ffff} for the {0.3:0000}Forest Temple{0.3:ffff}!");
        let t: String = line("InsideJabuJabuBelly", "Small Key", "Lord Jabu-Jabu's Belly", true, false);
        assert!(t.contains(" for {0.3:0000}Lord Jabu-Jabu's Belly"));
    }

    #[test]
    fn other_languages_keep_the_grammar_tags_of_the_names() {
        let u: String = line("ForestTemple", "Kleiner Schluessel", "{201.0:01050001}Waldtempel", false, false);
        assert_eq!(u, "{0.3:0000}Kleiner Schluessel{0.3:ffff}\n{0.3:0000}{201.0:01050001}Waldtempel{0.3:ffff}");
        let v: String = line("InsideTheDekuTree", "x", "{201.0:01050401}{201.10:0100}", false, true);
        assert!(v.contains("{201.10:0100}{0.3:ffff}: {2.1:"));
        let m: Msbt = msbt::parse(&book(&[("Empty", "")])).unwrap();
        assert!(name(&m, "Empty").is_err());
        assert!(name(&m, "Missing").is_err());
    }

    #[test]
    fn the_count_tag_names_the_key_count_of_that_dungeon() {
        let s: String = count_tag("ForestTemple").unwrap();
        let t: Vec<Tok> = msbt::from_markup(&s).unwrap();
        let Tok::Code { grp, kind, args } = &t[0] else {
            panic!("expected a code");
        };
        assert_eq!((*grp, *kind), (2, 1));
        let name: &[u8] = b"DungeonInfo.ForestTemple.SmallKeyNum";
        assert_eq!(&args[..2], &(name.len() as u16).to_le_bytes());
        assert_eq!(&args[2..2 + name.len()], name);
        assert_eq!(&args[2 + name.len()..], &[0xFF, 0xFF, 0x00]);
        assert!(line("ForestTemple", "Small Key", "Forest Temple", true, true).contains(&s));
    }

    #[test]
    fn the_archive_gains_one_label_per_dungeon_and_keeps_the_rest() {
        let raw: Vec<u8> = archive();
        let counted: BS<&str> = BS::from(["InsideJabuJabuBelly"]);
        let out: Vec<u8> = apply(&raw, &["ForestTemple", "InsideJabuJabuBelly", "FinalBattleField"], &counted, true).unwrap();
        assert_eq!(read(&out, "OOTRR_SmallKey_ForestTemple"), "You got a {0.3:0000}Small Key{0.3:ffff} for the {0.3:0000}Forest Temple{0.3:ffff}!");
        assert!(read(&out, "OOTRR_SmallKey_InsideJabuJabuBelly").contains("{2.1:"));
        assert!(read(&out, "OOTRR_SmallKey_FinalBattleField").contains("Ganondorf's Castle"));
        assert_eq!(read(&out, "GetMessage"), "{2.23:0100ffff}");
        assert_eq!(read(&out, "GetCaption"), "x");
        let again: Vec<u8> = apply(&out, &["ForestTemple", "InsideJabuJabuBelly", "FinalBattleField"], &counted, true).unwrap();
        assert_eq!(again, out);
        let other: Vec<u8> = apply(&raw, &["ForestTemple"], &BS::new(), false).unwrap();
        assert_eq!(read(&other, "OOTRR_SmallKey_ForestTemple"), "{0.3:0000}Small Key{0.3:ffff}\n{0.3:0000}Forest Temple{0.3:ffff}");
        assert!(apply(&raw, &["WaterTemple"], &BS::new(), true).is_err());
    }

    #[test]
    fn names_and_language_are_read_from_the_path_and_label() {
        assert!(is_english("Mals/USen.Product.110.sarc.zs"));
        assert!(is_english("Mals/EUen.Product.110.sarc.zs"));
        assert!(!is_english("Mals/EUde.Product.110.sarc.zs"));
        assert!(!is_english("Mals/JPja.Product.110.sarc.zs"));
        assert_eq!(message_id("ForestTemple"), "EventFlowMsg/Dm_SY_Get:OOTRR_SmallKey_ForestTemple");
        assert_eq!(place_label("InsideGanonCastle"), "GanonCastle");
        assert_eq!(place_label("ShadowTemple"), "ShadowTemple");
    }
}
