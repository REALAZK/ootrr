pub mod drops;
pub mod flows;
pub mod stats;

use std::fs;

use crate::error::Res;
use crate::flow_locations::dict_id;
use crate::qol::{edit_zs, Have};
use crate::romfs::Romfs;
use crate::sarc::{self, Entry};
use crate::settings::Settings;
use crate::staging::Out;
use crate::zs;

pub const HEALTH: &str = "enemy_health";
pub const DAMAGE: &str = "enemy_damage";
pub const SPEED: &str = "enemy_speed";
pub const NO_HEARTS: &str = "no_heart_drops";
pub const NO_FAIRIES: &str = "no_gossip_fairies";
pub const NO_AMMO: &str = "no_ammo_drops";
pub const LOSE_HEART: &str = "lose_heart_on_death";
pub const NO_BOTTLES: &str = "no_bottles_in_combat";

pub const HEALTH_NUM: u16 = 110;
pub const DAMAGE_NUM: u16 = 111;
pub const SPEED_NUM: u16 = 112;
pub const NO_HEARTS_NUM: u16 = 113;
pub const NO_FAIRIES_NUM: u16 = 114;
pub const NO_AMMO_NUM: u16 = 115;
pub const LOSE_HEART_NUM: u16 = 116;
pub const NO_BOTTLES_NUM: u16 = 117;

pub const NORMAL: i64 = 100;
pub const STAT_RANGE: (i64, i64) = (50, 500);
pub const SPEED_RANGE: (i64, i64) = (75, 200);
pub const ACTOR_PACKS: &str = "Pack/Actor";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opts {
    pub health: i64,
    pub damage: i64,
    pub speed: i64,
    pub no_hearts: bool,
    pub no_fairies: bool,
    pub no_ammo: bool,
    pub lose_heart: bool,
    pub no_bottles: bool,
}

impl Default for Opts {
    fn default() -> Opts {
        Opts { health: NORMAL, damage: NORMAL, speed: NORMAL, no_hearts: false, no_fairies: false, no_ammo: false, lose_heart: false, no_bottles: false }
    }
}

fn times(pct: i64) -> String {
    let s: String = format!("{:.2}", pct as f64 / 100.0);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

impl Opts {
    pub fn cut(&self) -> drops::Cut {
        drops::Cut { hearts: self.no_hearts, ammo: self.no_ammo }
    }

    pub fn packs(&self) -> bool {
        self.health != NORMAL || self.damage != NORMAL || self.speed != NORMAL || self.cut().any()
    }

    pub fn any(&self) -> bool {
        self.packs() || self.no_fairies || self.lose_heart || self.no_bottles
    }

    pub fn notes(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if self.health != NORMAL {
            out.push(format!("Enemies and bosses have {} times their usual health. Enemies that are beaten by a scripted hit, such as Deku Scrubs, are unchanged. Experimental, untested in the game.", times(self.health)));
        }
        if self.damage != NORMAL {
            out.push(format!("Enemy attacks and enemy projectiles deal {} times their usual damage. Grabs and traps that are not enemies are unchanged. Experimental, untested in the game.", times(self.damage)));
        }
        if self.speed != NORMAL {
            out.push(format!("Enemies move {} times as fast and wait that much less between attacks. Enemies that only move through their animations keep their pace. Experimental, untested in the game.", times(self.speed)));
        }
        if self.no_hearts {
            out.push("Enemies, pots, grass and other random drops never give recovery hearts. Experimental, untested in the game.".to_string());
        }
        if self.no_ammo {
            out.push("Enemies, pots, grass and other random drops never give arrows, seeds, bombs, Deku Nuts or Deku Sticks. Experimental, untested in the game.".to_string());
        }
        if self.no_fairies {
            out.push("Playing a song to a gossip stone no longer calls a fairy that heals you. Experimental, untested in the game.".to_string());
        }
        if self.lose_heart {
            out.push("Each game over that you continue from takes away one heart container, down to three hearts. A fairy in a bottle that revives you costs nothing. Experimental, untested in the game.".to_string());
        }
        if self.no_bottles {
            out.push("Drinking from a bottle or letting out its contents does nothing while the enemy battle music plays or during a boss battle. Experimental, untested in the game.".to_string());
        }
        out
    }
}

pub fn read(s: &Settings, bad: &mut Vec<String>) -> Opts {
    let mut num = |id: &str, r: (i64, i64)| -> i64 {
        match s.int(id) {
            Ok(v) if (r.0..=r.1).contains(&v) => v,
            Ok(v) => {
                bad.push(format!("{id} is {v}, which is outside {} to {}", r.0, r.1));
                NORMAL
            }
            Err(e) => {
                bad.push(e.to_string());
                NORMAL
            }
        }
    };
    let health: i64 = num(HEALTH, STAT_RANGE);
    let damage: i64 = num(DAMAGE, STAT_RANGE);
    let speed: i64 = num(SPEED, SPEED_RANGE);
    let mut flag = |id: &str| -> bool {
        s.flag(id).unwrap_or_else(|e| {
            bad.push(e.to_string());
            false
        })
    };
    Opts { health, damage, speed, no_hearts: flag(NO_HEARTS), no_fairies: flag(NO_FAIRIES), no_ammo: flag(NO_AMMO), lose_heart: flag(LOSE_HEART), no_bottles: flag(NO_BOTTLES) }
}

fn edit_entry(e: &Entry, o: &Opts) -> Res<Option<Vec<u8>>> {
    let n: &str = &e.name;
    if o.health != NORMAL
        && let Some(s) = stats::stem(n, stats::LIFE_DIR, stats::LIFE_EXT)
    {
        return if stats::is_enemy(s) { stats::life(&e.data, o.health) } else { Ok(None) };
    }
    if o.damage != NORMAL
        && let Some(s) = stats::stem(n, stats::ATTACK_DIR, stats::ATTACK_EXT)
    {
        return if stats::is_enemy_attack(s) { stats::attack(&e.data, o.damage) } else { Ok(None) };
    }
    if o.speed != NORMAL
        && let Some(s) = stats::stem(n, stats::MOVE_DIR, stats::MOVE_EXT)
    {
        return if stats::is_enemy(s) { stats::movement(&e.data, o.speed) } else { Ok(None) };
    }
    if o.speed != NORMAL
        && let Some(s) = stats::stem(n, stats::PACE_DIR, stats::PACE_EXT)
    {
        return if stats::is_enemy(s) { stats::pace(&e.data, o.speed) } else { Ok(None) };
    }
    if n.starts_with(drops::DIR) {
        return drops::edit(&e.data, o.cut());
    }
    Ok(None)
}

pub fn edit_pack(raw: &[u8], o: &Opts) -> Res<Option<Vec<u8>>> {
    let mut es: Vec<Entry> = sarc::parse(raw)?;
    let mut hit: bool = false;
    for e in es.iter_mut() {
        if let Some(d) = edit_entry(e, o)? {
            e.data = d;
            hit = true;
        }
    }
    if !hit {
        return Ok(None);
    }
    Ok(Some(sarc::write(&es)?))
}

fn pack_out(rf: &mut Romfs, have: Have, path: &str, o: &Opts) -> Res<Option<Out>> {
    let z: Vec<u8> = match have(path) {
        Some(b) => b,
        None => rf.read(path)?,
    };
    let id: Option<u32> = dict_id(&z);
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let Some(out) = edit_pack(&raw, o)? else {
        return Ok(None);
    };
    Ok(Some(Out { path: path.to_string(), old: raw.len(), new: out.len(), z: zs::compress_raw(&out, id)? }))
}

pub fn pack_files(rf: &Romfs, o: &Opts) -> Res<Vec<String>> {
    let mut out: Vec<String> = rf.list(ACTOR_PACKS)?.into_iter().filter(|p| p.ends_with(".pack.zs")).collect();
    if o.cut().any() {
        out.push(drops::FOLIAGE.to_string());
    }
    out.sort();
    Ok(out)
}

pub fn stage(rf: &mut Romfs, o: &Opts, have: Have) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    if o.packs() {
        for f in pack_files(rf, o)? {
            if let Some(x) = pack_out(rf, have, &f, o)? {
                outs.push(x);
            }
        }
    }
    if o.no_fairies {
        outs.push(edit_zs(rf, have, flows::GOSSIP_FILE, flows::no_gossip)?);
    }
    if o.lose_heart {
        outs.push(edit_zs(rf, have, flows::OVER_FILE, flows::lose_heart)?);
    }
    if o.no_bottles {
        for (file, name) in flows::BOTTLE_FLOWS {
            outs.push(edit_zs(rf, have, file, |raw| flows::no_bottles(raw, name))?);
        }
    }
    Ok(outs)
}

pub fn worked(rf: &mut Romfs, pk: &crate::modpack::Pack, o: &Opts) -> Res<Vec<Out>> {
    if !o.any() {
        return Ok(Vec::new());
    }
    let root: std::path::PathBuf = pk.work.join("romfs");
    let have = |p: &str| -> Option<Vec<u8>> { fs::read(p.split('/').fold(root.clone(), |a, s| a.join(s))).ok() };
    stage(rf, o, &have)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byml::{Byml, Node};
    use crate::settings::Value;

    #[test]
    fn everything_is_off_by_default_and_the_settings_string_is_unchanged() {
        let mut bad: Vec<String> = Vec::new();
        let o: Opts = read(&Settings::default(), &mut bad);
        assert!(bad.is_empty());
        assert_eq!(o, Opts::default());
        assert!(!o.any() && !o.packs() && o.notes().is_empty());
        let mut s: Settings = Settings::default();
        s.set(HEALTH, Value::Int(100)).unwrap();
        s.set(NO_HEARTS, Value::Bool(false)).unwrap();
        assert_eq!(s.encode().unwrap(), Settings::default().encode().unwrap());
    }

    #[test]
    fn each_setting_is_read_and_survives_the_settings_string() {
        let mut s: Settings = Settings::default();
        s.set(HEALTH, Value::Int(250)).unwrap();
        s.set(DAMAGE, Value::Int(50)).unwrap();
        s.set(SPEED, Value::Int(200)).unwrap();
        for id in [NO_HEARTS, NO_FAIRIES, NO_AMMO, LOSE_HEART, NO_BOTTLES] {
            s.set(id, Value::Bool(true)).unwrap();
        }
        let back: Settings = Settings::decode(&s.encode().unwrap()).unwrap().settings;
        let mut bad: Vec<String> = Vec::new();
        let o: Opts = read(&back, &mut bad);
        assert!(bad.is_empty());
        assert_eq!(o, Opts { health: 250, damage: 50, speed: 200, no_hearts: true, no_fairies: true, no_ammo: true, lose_heart: true, no_bottles: true });
        assert!(o.any() && o.packs());
        assert_eq!(o.notes().len(), 8);
        assert!(o.notes()[0].contains("2.5 times"));
        assert!(o.notes()[1].contains("0.5 times"));
        assert!(o.notes()[2].contains("2 times"));
        assert!(s.set(HEALTH, Value::Int(49)).is_err() || s.clone().finish().is_err());
        let mut t: Settings = Settings::default();
        assert!(t.set(SPEED, Value::Int(74)).is_err() || t.finish().is_err());
    }

    fn entry(name: &str, items: Vec<(&str, Node)>) -> Entry {
        let y: Byml = Byml { root: Node::Map(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect()), version: 7, big: false };
        Entry { name: name.to_string(), data: y.write().unwrap() }
    }

    fn max_life(es: &[Entry], name: &str) -> Option<Node> {
        Byml::parse(&sarc::get(es, name)?.data).ok()?.root.get("MaxLife").cloned()
    }

    #[test]
    fn a_pack_only_changes_its_enemy_files() {
        let foe: &str = "Life/LifeParameters/Enemy_Stalfos.game__life__LifeParameters.bgyml";
        let hero: &str = "Life/LifeParameters/Player.game__life__LifeParameters.bgyml";
        let es: Vec<Entry> = vec![entry(foe, vec![("MaxLife", Node::I32(18))]), entry(hero, vec![("MaxLife", Node::I32(12))])];
        let raw: Vec<u8> = sarc::write(&es).unwrap();
        assert!(edit_pack(&raw, &Opts::default()).unwrap().is_none());
        assert!(edit_pack(&raw, &Opts { damage: 300, speed: 150, no_hearts: true, ..Opts::default() }).unwrap().is_none());
        let out: Vec<Entry> = sarc::parse(&edit_pack(&raw, &Opts { health: 300, ..Opts::default() }).unwrap().unwrap()).unwrap();
        assert_eq!(max_life(&out, foe), Some(Node::I32(54)));
        assert_eq!(max_life(&out, hero), Some(Node::I32(12)));
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn the_game_files_take_every_option() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let o: Opts = Opts { health: 200, damage: 150, speed: 150, no_hearts: true, no_fairies: true, no_ammo: true, lose_heart: true, no_bottles: true };
        let outs: Vec<Out> = stage(&mut rf, &o, &none).unwrap();
        let at = |p: &str| -> &Out { outs.iter().find(|x| x.path == p).unwrap_or_else(|| panic!("{p} was not staged")) };
        for f in [flows::GOSSIP_FILE, flows::OVER_FILE, drops::FOLIAGE, "Pack/Actor/Enemy_Stalfos.pack.zs", "Pack/Actor/Enemy_Keese.pack.zs"] {
            at(f);
        }
        for (f, _) in flows::BOTTLE_FLOWS {
            at(f);
        }
        assert!(!outs.iter().any(|x| x.path.contains("Player_")), "the player packs are left alone");
        for x in outs.iter().filter(|x| x.path.ends_with(".pack.zs")) {
            assert!(x.new <= x.old, "{} grew", x.path);
        }
        let es: Vec<Entry> = sarc::parse(&rf.zs.decompress(&at("Pack/Actor/Enemy_Stalfos.pack.zs").z).unwrap()).unwrap();
        let was: Vec<Entry> = sarc::parse(&rf.read_zs("Pack/Actor/Enemy_Stalfos.pack.zs").unwrap()).unwrap();
        let life: &str = "Life/LifeParameters/Enemy_Stalfos.game__life__LifeParameters.bgyml";
        let Some(Node::I32(a)) = max_life(&was, life) else { panic!() };
        assert_eq!(max_life(&es, life), Some(Node::I32(a * 2)));
        let same: usize = es.iter().zip(was.iter()).filter(|(x, y)| x.name == y.name && x.data == y.data).count();
        assert!(same >= es.len() - 12, "only a handful of files change, {same} of {} match", es.len());
        assert!(stage(&mut rf, &Opts::default(), &none).unwrap().is_empty());
    }
}
