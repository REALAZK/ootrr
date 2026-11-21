use super::lottery::{find, Doc, Slot};
use crate::byml::{Byml, Node};
use crate::error::{Error, Res};

pub const FILE: &str = "Life/DamageParameters/Player.game__life__DamageParameters.bgyml";
pub const EXTERNAL: &str = "ExternalDamageParameters";
pub const RATES: &str = "DamageRates";
pub const INTERNAL: &str = "InternalDamageParameters";
pub const FALL: &str = "FallDamageParameters";
pub const FALL_MIN: &str = "FallDamageMin";
pub const FALL_RATES: [&str; 2] = ["FallDamagePerMeter", "FallDamageHighPerMeter"];
pub const FLAT: [&str; 4] = ["BurnContinuousDamage", "IgnitionDamage", "Damage", "ContinuousDamage"];
pub const GRAB_DIR: &str = "GameParameter/RestraintParam/";
pub const GRAB_EXT: &str = ".game__enemy__RestraintParam.bgyml";
pub const GRAB_COMMON: &str = "Enemy_Common";
pub const GRAB_KEY: &str = "PredationDamageValue";
pub const GRAB_DEFAULT: i32 = 2;
pub const KILL_INT: i32 = 999;
pub const KILL_RATE: f32 = 200.0;

const INT: u8 = 0xD1;
const FLOAT: u8 = 0xD2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hurt {
    Normal,
    Double,
    Quadruple,
    OneHit,
}

impl Hurt {
    pub fn from_setting(t: &str) -> Hurt {
        match t {
            "double" => Hurt::Double,
            "quadruple" => Hurt::Quadruple,
            "one_hit" => Hurt::OneHit,
            _ => Hurt::Normal,
        }
    }

    pub fn times(self) -> i32 {
        match self {
            Hurt::Normal => 1,
            Hurt::Double => 2,
            Hurt::Quadruple => 4,
            Hurt::OneHit => KILL_INT,
        }
    }

    pub fn rate(self) -> f32 {
        match self {
            Hurt::OneHit => KILL_RATE,
            h => h.times() as f32,
        }
    }
}

pub fn note(h: Hurt) -> String {
    match h {
        Hurt::Normal => String::new(),
        Hurt::OneHit => "Every hit takes all of Link's hearts: enemy and boss attacks, projectiles, grabs and falls. A bottled fairy still revives him. Falling into a pit or lava still costs one heart.".to_string(),
        h => format!("Link takes {} times the damage from enemy and boss attacks, projectiles, grabs and falls. Falling into a pit or lava still costs one heart.", h.times()),
    }
}

fn cut() -> Error {
    Error::Format("difficulty: the player's damage file is truncated")
}

fn put(out: &mut Vec<(usize, u32)>, s: Slot, h: Hurt) {
    match s.ty {
        FLOAT => out.push((s.at, (f32::from_bits(s.val) * h.rate()).to_bits())),
        INT if s.val != 0 => out.push((s.at, (s.val as i32).saturating_mul(h.times()).min(KILL_INT) as u32)),
        _ => {}
    }
}

pub fn edit(raw: &[u8], h: Hurt) -> Res<Option<Vec<u8>>> {
    if h == Hurt::Normal {
        return Ok(None);
    }
    let d: Doc = Doc::open(raw)?;
    let top: Vec<(String, Slot)> = d.map(d.root()?)?;
    let mut puts: Vec<(usize, u32)> = Vec::new();
    if let Some(ext) = find(&top, EXTERNAL)
        && let Some(rates) = find(&d.map(ext)?, RATES)
    {
        for (_, s) in d.hash(rates)? {
            if s.ty == FLOAT {
                put(&mut puts, s, h);
            }
        }
    }
    if let Some(int) = find(&top, INTERNAL) {
        for (_, group) in d.map(int)? {
            for (k, s) in d.map(group)? {
                if k == FALL_MIN || FLAT.contains(&k.as_str()) {
                    if s.ty == INT {
                        put(&mut puts, s, h);
                    }
                } else if FALL_RATES.contains(&k.as_str()) && h != Hurt::OneHit && s.ty == FLOAT {
                    put(&mut puts, s, h);
                }
            }
        }
    }
    if puts.is_empty() {
        return Ok(None);
    }
    let mut out: Vec<u8> = raw.to_vec();
    for (at, v) in puts {
        out.get_mut(at..at + 4).ok_or_else(cut)?.copy_from_slice(&v.to_le_bytes());
    }
    Ok(Some(out))
}

pub fn is_grab(name: &str) -> bool {
    name.strip_prefix(GRAB_DIR).and_then(|n| n.strip_suffix(GRAB_EXT)).is_some_and(|s| s.starts_with("Enemy_") && s != GRAB_COMMON)
}

pub fn grab(raw: &[u8], h: Hurt) -> Res<Option<Vec<u8>>> {
    if h == Hurt::Normal {
        return Ok(None);
    }
    let mut y: Byml = Byml::parse(raw)?;
    let now: i32 = match y.root.get(GRAB_KEY) {
        Some(Node::I32(v)) => *v,
        Some(Node::U32(v)) => *v as i32,
        Some(_) => return Ok(None),
        None => GRAB_DEFAULT,
    };
    if now <= 0 {
        return Ok(None);
    }
    y.root.set(GRAB_KEY, Node::I32(now.saturating_mul(h.times()).min(KILL_INT)));
    Ok(Some(y.write()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_setting_names_pick_the_steps_and_anything_else_is_normal() {
        assert_eq!(Hurt::from_setting("normal"), Hurt::Normal);
        assert_eq!(Hurt::from_setting("double"), Hurt::Double);
        assert_eq!(Hurt::from_setting("quadruple"), Hurt::Quadruple);
        assert_eq!(Hurt::from_setting("one_hit"), Hurt::OneHit);
        assert_eq!(Hurt::from_setting("nonsense"), Hurt::Normal);
        assert_eq!([Hurt::Normal.times(), Hurt::Double.times(), Hurt::Quadruple.times(), Hurt::OneHit.times()], [1, 2, 4, KILL_INT]);
        assert_eq!([Hurt::Double.rate(), Hurt::Quadruple.rate(), Hurt::OneHit.rate()], [2.0, 4.0, KILL_RATE]);
    }

    #[test]
    fn notes_say_what_each_step_does() {
        assert!(note(Hurt::Normal).is_empty());
        assert!(note(Hurt::Double).contains("2 times"));
        assert!(note(Hurt::Quadruple).contains("4 times"));
        assert!(note(Hurt::OneHit).contains("all of Link") && note(Hurt::OneHit).contains("fairy"));
    }

    #[test]
    fn only_enemy_restraint_files_count_as_grabs() {
        let f = |s: &str| format!("{GRAB_DIR}{s}{GRAB_EXT}");
        for s in ["Enemy_ReDead", "Enemy_LikeLike", "Enemy_DeadHand_Hand", "Enemy_Floormaster_Little", "Enemy_DungeonBoss_BongoBongo_RightHand", "Enemy_DungeonBoss_Morpha_Tentacle"] {
            assert!(is_grab(&f(s)), "{s}");
        }
        assert!(!is_grab(&f("Enemy_Common")) && !is_grab(&f("Player")));
        assert!(!is_grab("GameParameter/RestraintParam/Enemy_ReDead.other.bgyml"));
    }

    fn restraint(items: Vec<(&str, Node)>) -> Vec<u8> {
        Byml { root: Node::Map(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect()), version: 7, big: false }.write().unwrap()
    }

    fn damage_of(raw: &[u8]) -> Option<Node> {
        Byml::parse(raw).unwrap().root.get(GRAB_KEY).cloned()
    }

    #[test]
    fn a_grab_without_a_value_gets_the_default_times_the_step() {
        let raw: Vec<u8> = restraint(vec![("GachaInputFrame", Node::F32(330.0))]);
        assert!(grab(&raw, Hurt::Normal).unwrap().is_none());
        assert_eq!(damage_of(&grab(&raw, Hurt::Double).unwrap().unwrap()), Some(Node::I32(GRAB_DEFAULT * 2)));
        assert_eq!(damage_of(&grab(&raw, Hurt::OneHit).unwrap().unwrap()), Some(Node::I32(KILL_INT)));
    }

    #[test]
    fn a_grab_with_a_value_scales_it_and_a_zero_stays_zero() {
        let one: Vec<u8> = restraint(vec![(GRAB_KEY, Node::I32(1))]);
        assert_eq!(damage_of(&grab(&one, Hurt::Quadruple).unwrap().unwrap()), Some(Node::I32(4)));
        let none: Vec<u8> = restraint(vec![(GRAB_KEY, Node::I32(0))]);
        assert!(grab(&none, Hurt::OneHit).unwrap().is_none());
    }

    #[test]
    fn normal_leaves_the_players_file_alone() {
        assert!(edit(&[], Hurt::Normal).unwrap().is_none());
    }
}
