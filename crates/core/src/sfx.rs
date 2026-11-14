use std::collections::BTreeMap as BM;
use std::fs;
use std::io::Read;
use std::path::{Path as P, PathBuf as PB};

use crate::error::{Error as E, Res};
use crate::rng::Rng;

const STREAM: &str = "Sound/Resource/Stream";
const MAX_BYTES: u64 = 131072;
const BLOCK: usize = 6;
const MIN_POOL: usize = 4;
const SKIP: [&str; 8] = ["Env", "EnvAnimal", "Dm", "Demo", "Land", "CrumblingGanonCastle", "Escape", "Sys"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sfx {
    Off,
    Grouped,
    Anywhere,
}

impl Sfx {
    pub fn parse(s: &str) -> Option<Sfx> {
        match s {
            "off" => Some(Sfx::Off),
            "grouped" => Some(Sfx::Grouped),
            "anywhere" => Some(Sfx::Anywhere),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Sfx::Off => "off",
            Sfx::Grouped => "grouped",
            Sfx::Anywhere => "anywhere",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cand {
    pub name: String,
    pub group: String,
    pub samples: u32,
    pub bytes: u64,
}

fn le16(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

pub fn group_of(name: &str) -> Option<&str> {
    name.strip_prefix("SE_")?.split('_').next().filter(|g| !g.is_empty())
}

pub fn eligible(name: &str, head: &[u8], bytes: u64) -> Option<Cand> {
    if head.len() < 0x50 || &head[..4] != b"BWAV" || bytes > MAX_BYTES {
        return None;
    }
    let group: &str = group_of(name)?;
    if SKIP.contains(&group) {
        return None;
    }
    let mono: bool = le16(head, 0xE) == 1;
    let codec: bool = le16(head, 0x10) == 1;
    let rate: bool = le32(head, 0x14) == 48000;
    let once: bool = le32(head, 0x4C) == 0xFFFF_FFFF;
    if !(mono && codec && rate && once) {
        return None;
    }
    Some(Cand { name: name.to_string(), group: group.to_string(), samples: le32(head, 0x1C), bytes })
}

pub fn scan(dir: &P) -> Res<Vec<Cand>> {
    let mut out: Vec<Cand> = Vec::new();
    for e in fs::read_dir(dir)? {
        let e = e?;
        let file: String = e.file_name().to_string_lossy().to_string();
        let Some(stem) = file.strip_suffix(".bwav") else {
            continue;
        };
        if !stem.starts_with("SE_") {
            continue;
        }
        let bytes: u64 = e.metadata()?.len();
        if bytes > MAX_BYTES {
            continue;
        }
        let mut head: Vec<u8> = vec![0; 0x50];
        let mut f: fs::File = fs::File::open(e.path())?;
        if f.read_exact(&mut head).is_err() {
            continue;
        }
        if let Some(c) = eligible(stem, &head, bytes) {
            out.push(c);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn pools(c: &[Cand], m: Sfx) -> Vec<Vec<&Cand>> {
    let mut by: BM<String, Vec<&Cand>> = BM::new();
    for x in c {
        let key: String = if m == Sfx::Anywhere { String::new() } else { x.group.clone() };
        by.entry(key).or_default().push(x);
    }
    by.into_values().filter(|v| v.len() >= MIN_POOL).collect()
}

pub fn plan(c: &[Cand], m: Sfx, rng: &mut Rng) -> BM<String, String> {
    let mut out: BM<String, String> = BM::new();
    if m == Sfx::Off {
        return out;
    }
    for mut pool in pools(c, m) {
        pool.sort_by(|a, b| (a.samples, &a.name).cmp(&(b.samples, &b.name)));
        for block in pool.chunks(BLOCK) {
            let mut to: Vec<&Cand> = block.to_vec();
            rng.shuffle(&mut to);
            for (a, b) in block.iter().zip(to) {
                if a.name != b.name {
                    out.insert(a.name.clone(), b.name.clone());
                }
            }
        }
    }
    out
}

pub fn apply(mod_root: &P, romfs: &P, m: Sfx, seed: &str) -> Res<Vec<String>> {
    let mut notes: Vec<String> = Vec::new();
    if m == Sfx::Off {
        return Ok(notes);
    }
    let src: PB = romfs.join(STREAM);
    let cands: Vec<Cand> = scan(&src)?;
    let mut rng: Rng = Rng::from_text(&format!("{seed} sfx {}", m.id()));
    let map: BM<String, String> = plan(&cands, m, &mut rng);
    if map.is_empty() {
        return Err(E::Format("no sound effect could be moved"));
    }
    let dst: PB = mod_root.join("romfs").join(STREAM);
    fs::create_dir_all(&dst)?;
    let mut bytes: u64 = 0;
    for (target, from) in &map {
        let data: Vec<u8> = fs::read(src.join(format!("{from}.bwav")))?;
        bytes += data.len() as u64;
        fs::write(dst.join(format!("{target}.bwav")), data)?;
    }
    notes.push(format!(
        "Sound effects are shuffled {}. {} effects moved, {} MB added to the mod. Cutscene, ambient, menu and looping sounds keep their own.",
        if m == Sfx::Grouped { "within each kind of sound" } else { "across all kinds of sound" },
        map.len(),
        bytes / 1_000_000
    ));
    Ok(notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(ch: u16, rate: u32, samples: u32, loop_end: u32) -> Vec<u8> {
        let mut h: Vec<u8> = vec![0; 0x60];
        h[..4].copy_from_slice(b"BWAV");
        h[0xE..0x10].copy_from_slice(&ch.to_le_bytes());
        h[0x10..0x12].copy_from_slice(&1u16.to_le_bytes());
        h[0x14..0x18].copy_from_slice(&rate.to_le_bytes());
        h[0x1C..0x20].copy_from_slice(&samples.to_le_bytes());
        h[0x4C..0x50].copy_from_slice(&loop_end.to_le_bytes());
        h
    }

    fn cand(name: &str, samples: u32) -> Cand {
        Cand { name: name.to_string(), group: group_of(name).unwrap().to_string(), samples, bytes: 100 }
    }

    #[test]
    fn only_short_mono_one_shots_outside_the_skipped_groups_qualify() {
        let ok: Vec<u8> = head(1, 48000, 900, 0xFFFF_FFFF);
        assert!(eligible("SE_Player_Jump00", &ok, 100).is_some());
        assert!(eligible("SE_Env_Wind00", &ok, 100).is_none());
        assert!(eligible("SE_Sys_Cancel00", &ok, 100).is_none());
        assert!(eligible("SE_Player_Jump00", &head(2, 48000, 900, 0xFFFF_FFFF), 100).is_none());
        assert!(eligible("SE_Player_Jump00", &head(1, 48000, 900, 500), 100).is_none());
        assert!(eligible("SE_Player_Jump00", &head(1, 96000, 900, 0xFFFF_FFFF), 100).is_none());
        assert!(eligible("SE_Player_Jump00", &ok, MAX_BYTES + 1).is_none());
        assert!(eligible("Vo_PlAdult_Flail01", &ok, 100).is_none());
    }

    #[test]
    fn grouped_effects_trade_inside_their_group_and_near_in_length() {
        let c: Vec<Cand> = (0..12).map(|i| cand(&format!("SE_Player_A{i:02}"), 1000 + i * 10)).chain((0..8).map(|i| cand(&format!("SE_Item_B{i:02}"), 50 + i))).collect();
        let m: BM<String, String> = plan(&c, Sfx::Grouped, &mut Rng::from_text("x"));
        assert!(!m.is_empty());
        for (a, b) in &m {
            assert_eq!(group_of(a), group_of(b));
            let (sa, sb) = (c.iter().find(|x| &x.name == a).unwrap().samples as i64, c.iter().find(|x| &x.name == b).unwrap().samples as i64);
            assert!((sa - sb).abs() <= 60);
        }
    }

    #[test]
    fn anywhere_mixes_groups_and_is_deterministic() {
        let c: Vec<Cand> = (0..6).map(|i| cand(&format!("SE_Player_A{i}"), 100 + i)).chain((0..6).map(|i| cand(&format!("SE_Item_B{i}"), 100 + i))).collect();
        let a: BM<String, String> = plan(&c, Sfx::Anywhere, &mut Rng::from_text("seed"));
        let b: BM<String, String> = plan(&c, Sfx::Anywhere, &mut Rng::from_text("seed"));
        assert_eq!(a, b);
        assert!(a.iter().any(|(x, y)| group_of(x) != group_of(y)));
        assert!(plan(&c, Sfx::Off, &mut Rng::from_text("seed")).is_empty());
    }

    #[test]
    fn a_target_gets_exactly_one_source_and_every_source_is_used_once() {
        let c: Vec<Cand> = (0..30).map(|i| cand(&format!("SE_Npc_C{i:02}"), 500 + i * 3)).collect();
        let m: BM<String, String> = plan(&c, Sfx::Grouped, &mut Rng::from_text("q"));
        let mut from: Vec<&String> = m.values().collect();
        from.sort();
        from.dedup();
        assert_eq!(from.len(), m.len());
    }

    #[test]
    fn small_groups_are_left_alone() {
        let c: Vec<Cand> = (0..3).map(|i| cand(&format!("SE_Rare_D{i}"), 10 + i)).collect();
        assert!(plan(&c, Sfx::Grouped, &mut Rng::from_text("q")).is_empty());
    }

    #[test]
    fn mode_names_round_trip() {
        for m in [Sfx::Off, Sfx::Grouped, Sfx::Anywhere] {
            assert_eq!(Sfx::parse(m.id()), Some(m));
        }
        assert_eq!(Sfx::parse("loud"), None);
    }
}
