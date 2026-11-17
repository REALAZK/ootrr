use crate::byml::{Byml, Node};
use crate::error::{Error, Res};

pub const LIFE_DIR: &str = "Life/LifeParameters/";
pub const LIFE_EXT: &str = ".game__life__LifeParameters.bgyml";
pub const ATTACK_DIR: &str = "GameBalance/AttackParam/";
pub const ATTACK_EXT: &str = ".game__gamebalance__AttackParam.bgyml";
pub const MOVE_DIR: &str = "BSA/MovementParam/";
pub const MOVE_EXT: &str = ".game__bsa__MovementParam.bgyml";
pub const PACE_DIR: &str = "GameBalance/AttackInterval/";
pub const PACE_EXT: &str = ".game__gamebalance__AttackInterval.bgyml";

pub const SCRIPTED_LIFE: i32 = 100;
pub const EXTRA_ATTACKERS: [&str; 3] = ["Keese", "BeamosBeam", "ScrubDeku_DekuNut"];

pub fn stem<'a>(name: &'a str, dir: &str, ext: &str) -> Option<&'a str> {
    name.strip_prefix(dir)?.strip_suffix(ext)
}

pub fn is_enemy(stem: &str) -> bool {
    stem.starts_with("Enemy_") || stem == "Keese"
}

pub fn is_enemy_attack(stem: &str) -> bool {
    if stem.contains("MirrorShield") {
        return false;
    }
    stem.starts_with("Enemy_") || stem.starts_with("DungeonBoss") || stem.contains("_Bullet") || EXTRA_ATTACKERS.contains(&stem)
}

pub fn scaled_life(v: i32, pct: i64) -> i32 {
    if v <= 0 || v >= SCRIPTED_LIFE {
        return v;
    }
    let n: i64 = (v as i64 * pct + 50) / 100;
    n.clamp(1, (SCRIPTED_LIFE - 1) as i64) as i32
}

pub fn scaled_power(v: i32, pct: i64) -> (i32, f32) {
    let f: f32 = pct as f32 / 100.0;
    if v <= 0 {
        return (v, f);
    }
    let exact: f32 = v as f32 * f;
    let n: i32 = (exact.round() as i32).max(1);
    (n, exact / n as f32)
}

fn done(y: &Byml, raw: &[u8]) -> Res<Vec<u8>> {
    let out: Vec<u8> = y.write()?;
    if out.len() != raw.len() {
        return Err(Error::Format("difficulty: a scaled parameter file changed size"));
    }
    Ok(out)
}

pub fn life(raw: &[u8], pct: i64) -> Res<Option<Vec<u8>>> {
    let mut y: Byml = Byml::parse(raw)?;
    let Some(Node::I32(v)) = y.root.get("MaxLife") else {
        return Ok(None);
    };
    let n: i32 = scaled_life(*v, pct);
    if n == *v {
        return Ok(None);
    }
    y.root.set("MaxLife", Node::I32(n));
    Ok(Some(done(&y, raw)?))
}

pub fn attack(raw: &[u8], pct: i64) -> Res<Option<Vec<u8>>> {
    let mut y: Byml = Byml::parse(raw)?;
    let base: i32 = match y.root.get("BaseAttackPower") {
        Some(Node::I32(v)) => *v,
        _ => return Ok(None),
    };
    let (n, rest): (i32, f32) = scaled_power(base, pct);
    let mut hit: bool = n != base;
    y.root.set("BaseAttackPower", Node::I32(n));
    if let Some(list) = y.root.get_mut("AttackParams").and_then(|x| x.as_arr_mut()) {
        for a in list.iter_mut() {
            let Some(Node::F32(m)) = a.get("DamageMultiplier") else {
                continue;
            };
            let v: f32 = *m * rest;
            if v != *m {
                hit = true;
                a.set("DamageMultiplier", Node::F32(v));
            }
        }
    }
    if !hit {
        return Ok(None);
    }
    Ok(Some(done(&y, raw)?))
}

fn each_f32(y: &mut Byml, pick: impl Fn(&str) -> bool, f: impl Fn(f32) -> f32) -> bool {
    let Node::Map(m) = &mut y.root else {
        return false;
    };
    let mut hit: bool = false;
    for (k, v) in m.iter_mut() {
        if let Node::F32(x) = v
            && pick(k)
        {
            let n: f32 = f(*x);
            if n != *x {
                *x = n;
                hit = true;
            }
        }
    }
    hit
}

pub fn movement(raw: &[u8], pct: i64) -> Res<Option<Vec<u8>>> {
    let mut y: Byml = Byml::parse(raw)?;
    let f: f32 = pct as f32 / 100.0;
    if !each_f32(&mut y, |k| k.ends_with("Speed") && !k.contains("Rotate"), |x| x * f) {
        return Ok(None);
    }
    Ok(Some(done(&y, raw)?))
}

pub fn pace(raw: &[u8], pct: i64) -> Res<Option<Vec<u8>>> {
    let mut y: Byml = Byml::parse(raw)?;
    let f: f32 = pct as f32 / 100.0;
    if !each_f32(&mut y, |k| k.contains("Interval"), |x| x / f) {
        return Ok(None);
    }
    Ok(Some(done(&y, raw)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_pick_enemies_and_leave_the_player_alone() {
        assert_eq!(stem("Life/LifeParameters/Enemy_Stalfos.game__life__LifeParameters.bgyml", LIFE_DIR, LIFE_EXT), Some("Enemy_Stalfos"));
        assert_eq!(stem("Life/DamageParameters/Enemy_Stalfos.game__life__DamageParameters.bgyml", LIFE_DIR, LIFE_EXT), None);
        assert!(is_enemy("Enemy_Stalfos") && is_enemy("Keese"));
        assert!(!is_enemy("Player") && !is_enemy("Animal_Cucco") && !is_enemy("NpcGoron"));
        for n in ["Enemy_Octorok", "DungeonBossVolvagia_FireBreath", "Twinrova_Bullet_IceBeam", "Keese", "BeamosBeam", "SkullKid_Bullet", "Bubble_Green_Bullet_Aura"] {
            assert!(is_enemy_attack(n), "{n}");
        }
        for n in ["Player", "Player_Child", "Item_Boomerang", "NormalArrow", "Obj_Pot", "MirrorShield_Bullet_FireChargeBeam", "Twinrova_Bullet_IceBeam_MirrorShieldReflect", "BeamosBeam_MirrorShieldReflect", "Animal_Horse"] {
            assert!(!is_enemy_attack(n), "{n}");
        }
    }

    #[test]
    fn life_rounds_stays_alive_and_skips_scripted_values() {
        assert_eq!(scaled_life(18, 100), 18);
        assert_eq!(scaled_life(18, 50), 9);
        assert_eq!(scaled_life(1, 50), 1);
        assert_eq!(scaled_life(3, 150), 5);
        assert_eq!(scaled_life(70, 500), 99);
        assert_eq!(scaled_life(100, 500), 100);
        assert_eq!(scaled_life(0, 500), 0);
    }

    #[test]
    fn power_keeps_the_exact_product() {
        for (v, pct) in [(2, 150), (1, 50), (12, 500), (4, 75), (16, 125)] {
            let (n, rest): (i32, f32) = scaled_power(v, pct);
            assert!(n >= 1);
            assert!((n as f32 * rest - v as f32 * pct as f32 / 100.0).abs() < 1e-4, "{v} {pct}");
        }
        assert_eq!(scaled_power(2, 150), (3, 1.0));
        assert_eq!(scaled_power(0, 200), (0, 2.0));
        assert_eq!(scaled_power(2, 100), (2, 1.0));
    }

    fn file(items: Vec<(&str, Node)>) -> Vec<u8> {
        let y: Byml = Byml { root: Node::Map(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect()), version: 7, big: false };
        y.write().unwrap()
    }

    #[test]
    fn files_scale_in_place() {
        let raw: Vec<u8> = file(vec![("MaxLife", Node::I32(18))]);
        assert!(life(&raw, 100).unwrap().is_none());
        let out: Vec<u8> = life(&raw, 200).unwrap().unwrap();
        assert_eq!(Byml::parse(&out).unwrap().root.get("MaxLife"), Some(&Node::I32(36)));
        let raw: Vec<u8> = file(vec![("DashRotateSpeed", Node::F32(180.0)), ("RunSpeed", Node::F32(6.0)), ("WalkBackwardsSpeed", Node::F32(-3.0)), ("RunSpeedApproachRatio", Node::F32(0.1))]);
        let y: Byml = Byml::parse(&movement(&raw, 150).unwrap().unwrap()).unwrap();
        assert_eq!(y.root.get("RunSpeed"), Some(&Node::F32(9.0)));
        assert_eq!(y.root.get("WalkBackwardsSpeed"), Some(&Node::F32(-4.5)));
        assert_eq!(y.root.get("DashRotateSpeed"), Some(&Node::F32(180.0)));
        assert_eq!(y.root.get("RunSpeedApproachRatio"), Some(&Node::F32(0.1)));
        let raw: Vec<u8> = file(vec![("LongIntervalMax", Node::F32(240.0))]);
        let y: Byml = Byml::parse(&pace(&raw, 200).unwrap().unwrap()).unwrap();
        assert_eq!(y.root.get("LongIntervalMax"), Some(&Node::F32(120.0)));
        let one: Node = Node::Map(vec![("DamageMultiplier".to_string(), Node::F32(4.0))]);
        let raw: Vec<u8> = file(vec![("AttackParams", Node::Arr(vec![one])), ("BaseAttackPower", Node::I32(1))]);
        let y: Byml = Byml::parse(&attack(&raw, 150).unwrap().unwrap()).unwrap();
        assert_eq!(y.root.get("BaseAttackPower"), Some(&Node::I32(2)));
        assert_eq!(y.root.get("AttackParams").unwrap().as_arr().unwrap()[0].get("DamageMultiplier"), Some(&Node::F32(3.0)));
    }
}
