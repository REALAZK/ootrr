use crate::ainb::build::Spec;
use crate::ainb::{Ainb, Plug};
use crate::difficulty::elink::{self, Link, Param, TEXT};
use crate::difficulty::hook::{both, flag, PARALLEL};
use crate::error::{Error, Res};
use crate::qol::{edit_zs, Have};
use crate::romfs::Romfs;
use crate::staging::Out;

pub const EMIT: &str = "OneShotXLinkSearchAndEmit";
pub const KEY: &str = "デクの実有効";
pub const EFFECT: &str = "RctHit_DekuNutStun_00";
pub const HEAL_KEY: &str = "OotrrHeal";
pub const HEAL_EFFECT: &str = "Ply_Magic_FaroreWind_Trigger_00";
pub const RING_KEY: &str = "OotrrHealRing";
pub const RING_EFFECT: &str = "Ply_Magic_FaroreWind_Trigger_02";
pub const ORB_KEY: &str = "OotrrHealOrb";
pub const ORB_EFFECT: &str = "Ply_Magic_FaroreWind_Trigger_01";
pub const HEAL_FROM: [&str; 2] = ["Player_Adult", "Player"];
pub const HEAL_FRAMES: f32 = 240.0;
pub const SCALE: u32 = 13;
pub const SCALE_NORMAL: f32 = 1.5;
pub const SCALE_TALL: f32 = 2.0;
pub const SCALE_LARGE: f32 = 2.5;
pub const SCALE_HUGE: f32 = 3.0;
pub const TALL: [&str; 2] = ["Lizalfos", "GerudoThief"];
pub const LARGE: [&str; 7] = ["Wolfos", "Dinolfos", "Moblin", "IronKnuckle", "Floormaster", "DarkLink", "FlareDancer"];
pub const HUGE: [&str; 6] = ["DungeonBoss_", "Enemy_Dodongo", "BigOcto", "PhantomGanonHorse", "Tailpasaran", "DeadHand"];
pub const PARTS: [(&str, &str, &[&str]); 3] = [(RING_KEY, RING_EFFECT, &["Root", "Skl_Root", "Body"]), (HEAL_KEY, HEAL_EFFECT, &[]), (ORB_KEY, ORB_EFFECT, &["Head", "Skl_Head"])];
pub const ENEMIES: &str = "Pack/Actor";
pub const ENEMY: &str = "Enemy_";
pub const SKIP: &str = "_Dictionary";
pub const BONES: [&str; 6] = ["Spine_1", "Spine_2", "Head", "Skl_Root", "Root", "Body"];
pub const ASSET_NAME: u32 = 0;
pub const RUNTIME_NAME: u32 = 1;
pub const DURATION: u32 = 7;
pub const BONE: u32 = 12;
pub const HANDLE: &str = "xlink2::HandleSet";
pub const LIFE: &str = "QueryActorLife";
pub const FULL: &str = "SelectorMathLessOrEqualS32";
pub const INT_LINKS: usize = 5;
pub const CUE_ADDED: usize = 2;
pub const HURT_ADDED: usize = 6;

pub fn cue(a: &mut Ainb, seed: &str, name: &str) -> Res<usize> {
    let key: Vec<u8> = a.pool.add(name).to_le_bytes().to_vec();
    a.add_node(
        &Spec::node(EMIT, seed)
            .prop(1, "ExecuteEveryFrame", flag(false))
            .idle(1, "ELinkCall", flag(true))
            .constant(1, "SLinkCall", flag(false))
            .constant(3, "XLinkKeyName", key)
            .spare_ptr("HandleSet", HANDLE),
    )
}

pub fn flash(a: &mut Ainb, seed: &str) -> Res<usize> {
    cue(a, seed, KEY)
}

pub fn with_cue(a: &mut Ainb, tag: &str, node: usize, name: &str) -> Res<usize> {
    let c: usize = cue(a, &format!("{tag} flash"), name)?;
    a.add_node(&both(&format!("{tag} and flash"), node, c))
}

pub fn with_cues(a: &mut Ainb, tag: &str, node: usize, names: &[&str]) -> Res<usize> {
    let mut cues: Vec<usize> = Vec::new();
    for name in names {
        cues.push(cue(a, &format!("{tag} {name}"), name)?);
    }
    let seed: String = format!("{tag} and heal");
    let mut spec: Spec = Spec::composite(PARALLEL, &seed).prop(0, "EndPolicy", vec![0; 4]).prop(0, "ResultPolicy", vec![0; 4]).kid(node, "");
    for c in cues {
        spec = spec.kid(c, "");
    }
    a.add_node(&spec)
}

pub fn with_flash(a: &mut Ainb, tag: &str, node: usize) -> Res<usize> {
    with_cue(a, tag, node, KEY)
}

pub fn when_hurt(a: &mut Ainb, tag: &str, full: usize, hurt: usize) -> Res<usize> {
    let life: usize = a.add_node(&Spec::query(LIFE, &format!("{tag} life")).out(0, "Max").out(0, "Now").spare(2, "RemainingPercent"))?;
    let lit: usize = with_cues(a, tag, hurt, &[RING_KEY, HEAL_KEY, ORB_KEY])?;
    let pick: usize = a.add_node(
        &Spec::node(FULL, &format!("{tag} full"))
            .prop(1, "NoChangeWhenChildIsBusy", flag(false))
            .prop(1, "SelectChildOnEnterOnly", flag(false))
            .linked(0, "S32", life, 0)
            .linked(0, "S32Base", life, 1)
            .kid(full, "if_true")
            .kid(lit, "if_false"),
    )?;
    one_source(a, pick, life, "S32")?;
    Ok(pick)
}

pub fn heal_template(l: &Link, effect: &str) -> Res<Param> {
    let missing = || Error::Format("elink: the player has no call for the Farore's Wind effect");
    let found: Option<&Param> = HEAL_FROM.iter().filter_map(|n| l.user(n)).find_map(|u| l.users[u].calls.iter().filter(|c| c.asset() != elink::CONTAINER).filter_map(|c| l.param_at(c.param_pos())).find(|p| p.get(RUNTIME_NAME).and_then(|v| l.text(v)) == Some(effect)));
    let p: &Param = found.ok_or_else(missing)?;
    let frames: u32 = l.direct_f32(HEAL_FRAMES).ok_or(Error::Format("elink: no stored value for the heal effect duration"))?;
    let keep: Vec<(u32, u32)> = p.pairs().into_iter().filter(|(b, _)| *b != ASSET_NAME && *b != BONE && *b != DURATION && *b != SCALE).chain([(DURATION, frames)]).collect();
    Param::from_pairs(&keep)
}

pub fn scale_of(user: &str) -> f32 {
    let has = |l: &[&str]| l.iter().any(|k| user.contains(k));
    if has(&HUGE) {
        SCALE_HUGE
    } else if has(&LARGE) {
        SCALE_LARGE
    } else if has(&TALL) {
        SCALE_TALL
    } else {
        SCALE_NORMAL
    }
}

fn seen_bones(l: &Link, u: usize) -> Vec<(u32, &str)> {
    let mut seen: Vec<(u32, &str)> = Vec::new();
    for c in &l.users[u].calls {
        if c.asset() == elink::CONTAINER {
            continue;
        }
        if let Some(v) = l.param_at(c.param_pos()).and_then(|p| p.get(BONE))
            && let Some(name) = l.text(v)
        {
            seen.push((v, name));
        }
    }
    seen
}

pub fn bone_of(l: &Link, u: usize) -> Option<u32> {
    let seen: Vec<(u32, &str)> = seen_bones(l, u);
    let deku: Option<u32> = l.find_key(u, KEY).and_then(|c| l.param_at(l.users[u].calls[c].param_pos())).and_then(|p| p.get(BONE));
    BONES[..2].iter().find_map(|b| seen.iter().find(|s| s.1 == *b).map(|s| s.0)).or(deku).or_else(|| BONES[2..].iter().find_map(|b| seen.iter().find(|s| s.1 == *b).map(|s| s.0))).or_else(|| seen.first().map(|s| s.0))
}

pub fn bone_for(l: &Link, u: usize, want: &[&str]) -> Option<u32> {
    let seen: Vec<(u32, &str)> = seen_bones(l, u);
    want.iter().find_map(|b| seen.iter().find(|s| s.1 == *b).map(|s| s.0)).or_else(|| bone_of(l, u))
}

pub fn heal_link(raw: &[u8], users: &[String]) -> Res<(Vec<u8>, Vec<String>)> {
    let mut l: Link = Link::parse(raw)?;
    let mut parts: Vec<(u32, Param, &str, &[&str])> = Vec::new();
    for (key, effect, bones) in PARTS {
        let at: u32 = (TEXT << 24) | l.add_name(key)? as u32;
        parts.push((at, heal_template(&l, effect)?, key, bones));
    }
    let mut done: Vec<String> = Vec::new();
    for name in users {
        let Some(u) = l.user(name) else {
            continue;
        };
        let scale: Option<u32> = l.direct_f32(scale_of(name));
        let mut added: bool = false;
        for (at, base, key, bones) in &parts {
            if l.find_key(u, key).is_some() {
                continue;
            }
            let mut pairs: Vec<(u32, u32)> = base.pairs();
            pairs.push((ASSET_NAME, *at));
            if let Some(b) = bone_for(&l, u, bones) {
                pairs.push((BONE, b));
            }
            if let Some(s) = scale {
                pairs.push((SCALE, s));
            }
            l.add_call(u, key, Param::from_pairs(&pairs)?)?;
            added = true;
        }
        if added {
            done.push(name.clone());
        }
    }
    Ok((l.write()?, done))
}

pub fn enemy_users(rf: &Romfs) -> Res<Vec<String>> {
    let mut out: Vec<String> = rf
        .list(ENEMIES)?
        .into_iter()
        .filter_map(|p| p.rsplit('/').next().and_then(|f| f.strip_suffix(".pack.zs")).map(|s| s.to_string()))
        .filter(|n| n.starts_with(ENEMY) && !n.ends_with(SKIP))
        .collect();
    out.sort();
    out.dedup();
    Ok(out)
}

pub fn link_stage_for(rf: &mut Romfs, have: Have, users: Vec<String>) -> Res<Vec<Out>> {
    Ok(vec![edit_zs(rf, have, elink::FILE, |raw| Ok(heal_link(raw, &users)?.0))?])
}

pub fn link_stage(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let users: Vec<String> = enemy_users(rf)?;
    Ok(vec![edit_zs(rf, have, elink::FILE, |raw| Ok(heal_link(raw, &users)?.0))?])
}

fn one_source(a: &mut Ainb, el: usize, from: usize, name: &str) -> Res<()> {
    let e: &mut [u8] = &mut a.els[el];
    let n: u16 = u16::from_le_bytes([e[0x26], e[0x27]]);
    if n != 2 || a.qids.len() < 4 {
        return Err(Error::Format("ainb: expected two reads of one query"));
    }
    e[0x26..0x28].copy_from_slice(&1u16.to_le_bytes());
    let keep: usize = a.qids.len() - 4;
    a.qids.truncate(keep);
    let nm: u32 = a.pool.add(name);
    let p: &mut crate::ainb::Param = &mut a.params[el];
    let kids: Vec<Plug> = p.plugs.split_off(2);
    let n2: u8 = kids.len() as u8;
    p.plugs = kids;
    p.plugs.push(Plug { a: from as u32, b: nm, extra: Vec::new() });
    for (g, pl) in p.pl.iter_mut().enumerate() {
        *pl = match g {
            2 => [n2, 0],
            INT_LINKS => [1, n2],
            g if g > INT_LINKS => [0, n2 + 1],
            g if g > 2 => [0, n2],
            _ => [0, 0],
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ainb::show::show;
    use crate::difficulty::hook::{GLOBAL, PARALLEL};
    use crate::romfs::Romfs;
    use crate::sarc::{self, Entry};

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn block(a: &Ainb, i: usize) -> String {
        let all: String = show(a);
        let from: usize = all.find(&format!("element {i} type")).unwrap();
        let body: &str = &all[from..];
        let end: usize = body.find(&format!("\nelement {} type", i + 1)).unwrap_or(body.len());
        body[..end].to_string()
    }

    fn fields(b: &str) -> Vec<String> {
        b.lines().skip(1).filter(|l| !l.contains("plug")).map(|l| l.split(" = ").next().unwrap_or(l).split(" from ").next().unwrap_or(l).split(" flags").next().unwrap_or(l).to_string()).collect()
    }

    fn graph(rf: &mut Romfs, name: &str) -> Ainb {
        let es: Vec<Entry> = sarc::parse(&rf.read_zs(GLOBAL).unwrap()).unwrap();
        Ainb::parse(&sarc::get(&es, name).unwrap().data).unwrap()
    }

    #[test]
    fn the_flash_matches_the_game_emit_node_and_names_the_deku_nut_key() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let mut a: Ainb = graph(&mut rf, "AI/EnemyBase.utility.root.ainb");
        let was: usize = a.els.len();
        let idle: usize = a.add_node(&Spec::node("ExecuteGenericDummyAction", "fx test idle")).unwrap();
        let top: usize = with_flash(&mut a, "fx test", idle).unwrap();
        assert_eq!(a.els.len(), was + 1 + CUE_ADDED);
        assert_eq!(a.el_type(top), PARALLEL);
        let back: Ainb = Ainb::parse(&a.write().unwrap()).unwrap();
        let cue: usize = back.find_els(EMIT)[0];
        let mine: String = block(&back, cue);
        for line in ["prop bool ExecuteEveryFrame = false", "in bool ELinkCall = true flags 0x1800000", "in bool SLinkCall = false flags 0x800000", "in str XLinkKeyName = \"デクの実有効\" flags 0x800000", "out ptr HandleSet flags 0x0"] {
            assert!(mine.contains(line), "{line} is missing from\n{mine}");
        }
        let stalchild: Ainb = graph(&mut rf, "AI/Enemy_Stalchild.utility.root.ainb");
        let theirs: usize = stalchild.find_els(EMIT)[0];
        assert_eq!(fields(&mine), fields(&block(&stalchild, theirs)));
        let ptr: &[u8] = &back.outs[crate::ainb::build::PTR];
        assert!(!ptr.is_empty());
        assert!(show(&back).contains("HandleSet"));
    }

    #[test]
    fn the_hurt_check_reads_the_life_query_once_like_the_game_does() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let mut a: Ainb = graph(&mut rf, "AI/EnemyBase.utility.root.ainb");
        let was: usize = a.els.len();
        let full: usize = a.add_node(&Spec::node("ExecuteGenericDummyAction", "fx test full")).unwrap();
        let hurt: usize = a.add_node(&Spec::node("ExecuteGenericDummyAction", "fx test hurt")).unwrap();
        let pick: usize = when_hurt(&mut a, "fx test", full, hurt).unwrap();
        assert_eq!(a.els.len(), was + 2 + HURT_ADDED);
        let back: Ainb = Ainb::parse(&a.write().unwrap()).unwrap();
        assert_eq!(back.write().unwrap(), a.write().unwrap());
        let life: usize = back.find_els(LIFE).into_iter().max().unwrap();
        let mine: String = block(&back, pick);
        for line in [format!("in s32 S32 = 0 from {life}.0 flags 0x0"), format!("in s32 S32Base = 0 from {life}.1 flags 0x0"), format!("plug 0 -> element {full} \"if_true\""), format!("plug 2 -> element {life} \"S32\""), "plug groups 0@0 0@0 2@0 0@2 0@2 1@2 0@3 0@3 0@3 0@3".to_string()] {
            assert!(mine.contains(&line), "{line} is missing from\n{mine}");
        }
        assert_eq!(back.params[pick].group(2).len(), 2);
        let lit: usize = back.params[pick].group(2)[1].a as usize;
        assert_eq!(back.el_type(lit), PARALLEL);
        let kids: Vec<usize> = back.params[lit].group(2).iter().map(|g| g.a as usize).collect();
        assert_eq!(kids.len(), 4, "the heal plays the three parts of the cast beside the heal");
        assert_eq!(kids[0], hurt);
        let keys: Vec<Option<&str>> = kids[1..].iter().map(|k| back.in_str(*k, "XLinkKeyName")).collect();
        assert_eq!(keys, [Some(RING_KEY), Some(HEAL_KEY), Some(ORB_KEY)], "the heal plays the ring, the column and the orb");
        assert!(kids[1..].iter().all(|k| back.el_name(*k) == EMIT));
        let beamos: Ainb = graph(&mut rf, "AI/Enemy_Beamos.root.ainb");
        let theirs: usize = beamos.find_els(FULL)[0];
        assert_eq!(fields(&mine), fields(&block(&beamos, theirs)));
        let q: usize = beamos.find_els(LIFE)[0];
        assert_eq!(fields(&block(&back, life)), fields(&block(&beamos, q)));
        let e: &[u8; crate::ainb::EL] = &back.els[pick];
        assert_eq!(u16::from_le_bytes([e[0x26], e[0x27]]), 1, "the selector lists its one source once");
    }

    #[test]
    fn the_deku_nut_key_plays_an_effect_from_the_resident_set() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let link: Vec<u8> = rf.read_zs("ELink2/elink2.Product.110.belnk.zs").unwrap();
        let has = |d: &[u8], s: &str| -> bool { d.windows(s.len() + 1).any(|w| &w[..s.len()] == s.as_bytes() && w[s.len()] == 0) };
        assert!(has(&link, KEY) && has(&link, EFFECT));
        let info: crate::byml::Byml = rf.read_byml("Effect/EffectFileInfo.Product.110.Nin_Ounce_NVN2.byml.zs").unwrap();
        let resident: Vec<&str> = info.root.get("StaticEsetb").and_then(|n| n.as_arr()).unwrap().iter().filter_map(|n| n.as_str()).collect();
        assert_eq!(resident, ["static"]);
        let set: Vec<u8> = rf.read_zs("Effect/static.Nin_Ounce_NVN2.vsetb.byml.zs").unwrap();
        assert!(has(&set, EFFECT), "the flash is in the set every scene loads");
    }

    fn healed(rf: &mut Romfs) -> (Link, Link, Vec<String>) {
        let raw: Vec<u8> = rf.read_zs(elink::FILE).unwrap();
        let users: Vec<String> = enemy_users(rf).unwrap();
        let (out, done): (Vec<u8>, Vec<String>) = heal_link(&raw, &users).unwrap();
        (Link::parse(&raw).unwrap(), Link::parse(&out).unwrap(), done)
    }

    #[test]
    fn the_heal_key_is_added_to_every_enemy_user_and_plays_farores_wind() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let (was, now, done): (Link, Link, Vec<String>) = healed(&mut rf);
        let named: Vec<usize> = done.iter().map(|n| was.user(n).unwrap()).collect();
        let reaction: Option<usize> = was.user("ReactionHit");
        assert!(reaction.is_some_and(|r| was.find_key(r, KEY).is_some()), "the engine plays the Deku Nut key through its reaction user");
        for u in (0..was.users.len()).filter(|u| was.find_key(*u, KEY).is_some() && Some(*u) != reaction) {
            assert!(named.contains(&u), "user {u} has the Deku Nut key but no heal key");
        }
        for n in ["Enemy_Stalfos", "Enemy_Keese", "Enemy_Leever", "Enemy_LikeLike", "Enemy_ReDead", "Enemy_Gibdo", "Enemy_IronKnuckle_Black", "Enemy_Poe", "Enemy_Peahat", "Enemy_Freezard", "Enemy_Wolfos", "Enemy_Octorok"] {
            assert!(done.iter().any(|d| d == n), "{n} is covered");
        }
        assert!(done.len() >= 100, "{} users", done.len());
        assert!(!done.iter().any(|d| d.ends_with(SKIP)));
        let info: crate::byml::Byml = rf.read_byml("Effect/EffectFileInfo.Product.110.Nin_Ounce_NVN2.byml.zs").unwrap();
        let resident: Vec<&str> = info.root.get("StaticEsetb").and_then(|n| n.as_arr()).unwrap().iter().filter_map(|n| n.as_str()).collect();
        assert_eq!(resident, ["static"]);
        let set: Vec<u8> = rf.read_zs("Effect/static.Nin_Ounce_NVN2.vsetb.byml.zs").unwrap();
        let has = |d: &[u8], s: &str| -> bool { d.windows(s.len() + 1).any(|w| &w[..s.len()] == s.as_bytes() && w[s.len()] == 0) };
        let frames: u32 = now.direct_f32(HEAL_FRAMES).unwrap();
        for n in &done {
            let u: usize = now.user(n).unwrap();
            let own_bones: Vec<&str> = was.users[u].calls.iter().filter_map(|c| was.param_at(c.param_pos())).filter_map(|q| q.get(BONE).and_then(|v| was.text(v))).collect();
            assert_eq!(now.users[u].calls.len(), was.users[u].calls.len() + PARTS.len());
            assert_eq!(&now.users[u].calls[..was.users[u].calls.len()], &was.users[u].calls[..]);
            assert_eq!((&now.users[u].boxes, &now.users[u].triggers, &now.users[u].pre), (&was.users[u].boxes, &was.users[u].triggers, &was.users[u].pre));
            for (key, effect, _) in PARTS {
                let c: usize = now.find_key(u, key).unwrap();
                let call: &elink::Call = &now.users[u].calls[c];
                let p: &Param = now.param_at(call.param_pos()).unwrap();
                assert_eq!(now.text(p.get(ASSET_NAME).unwrap()), Some(key));
                assert_eq!(now.text(p.get(RUNTIME_NAME).unwrap()), Some(effect));
                assert!(has(&set, effect), "{effect} is in the set every scene loads");
                assert_eq!(p.get(DURATION), Some(frames));
                assert_eq!(f32::from_bits(now.direct(frames as usize).unwrap()), HEAL_FRAMES);
                assert_eq!(f32::from_bits(now.direct(p.get(SCALE).unwrap() as usize).unwrap()), scale_of(n));
                if let Some(b) = p.get(BONE) {
                    let bone: &str = now.text(b).unwrap();
                    assert!(own_bones.contains(&bone), "{n} plays {key} on a bone it already uses ({bone})");
                }
                assert!(call.asset() as usize >= was.assets(u));
            }
        }
        let bone_of_key = |u: usize, key: &str| -> Option<&str> { now.param_at(now.users[u].calls[now.find_key(u, key).unwrap()].param_pos()).unwrap().get(BONE).and_then(|b| now.text(b)) };
        let stalfos: usize = now.user("Enemy_Stalfos").unwrap();
        assert_eq!(bone_of_key(stalfos, HEAL_KEY), Some("Spine_1"), "the body bone comes before the Deku Nut head bone");
        assert_eq!(bone_of_key(stalfos, RING_KEY), Some("Root"), "the ring sits at the feet");
        let beamos: usize = now.user("Enemy_Beamos").unwrap();
        assert_eq!(bone_of_key(beamos, HEAL_KEY), Some("Eye_1"), "with no body bone, the Deku Nut bone");
        let keese: usize = now.user("Enemy_Keese").unwrap();
        assert_eq!(bone_of_key(keese, HEAL_KEY), Some("Spine_1"));
        let wolfos: usize = now.user("Enemy_Wolfos").unwrap();
        assert_eq!(scale_of("Enemy_Wolfos"), SCALE_LARGE);
        assert!(bone_of_key(wolfos, ORB_KEY).is_some());
        assert_eq!((scale_of("Enemy_Keese"), scale_of("Enemy_Lizalfos"), scale_of("Enemy_DungeonBoss_Gohma")), (SCALE_NORMAL, SCALE_TALL, SCALE_HUGE));
        let touched: Vec<usize> = done.iter().map(|n| now.user(n).unwrap()).collect();
        let same: usize = (0..now.users.len()).filter(|u| !touched.contains(u)).inspect(|u| assert_eq!(now.users[*u], was.users[*u], "user {u} is untouched")).count();
        assert_eq!(same + touched.len(), was.users.len());
        assert_eq!(&now.params[..was.params.len()], &was.params[..]);
        assert!(now.params.len() > was.params.len() && now.params.len() <= was.params.len() + PARTS.len() * done.len());
        assert_eq!((&now.mid, &now.cond, &now.define, &now.hashes), (&was.mid, &was.cond, &was.define, &was.hashes));
        assert_eq!(&now.names[..was.names.len()], &was.names[..]);
        let again: Vec<u8> = heal_link(&now.write().unwrap(), &done).unwrap().0;
        assert_eq!(again, now.write().unwrap(), "a second pass changes nothing");
    }

    #[test]
    fn the_enemy_parts_are_the_children_of_the_players_farore_container() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let raw: Vec<u8> = rf.read_zs(elink::FILE).unwrap();
        let l: Link = Link::parse(&raw).unwrap();
        for name in HEAL_FROM {
            let u: usize = l.user(name).unwrap();
            let c: usize = l.find_key(u, "FaroreTrigger").unwrap();
            assert_eq!(l.users[u].calls[c].asset(), elink::CONTAINER);
            let kids: Vec<&str> = (0..l.users[u].calls.len()).filter(|k| u32::from_le_bytes(l.users[u].calls[*k].raw[0x10..0x14].try_into().unwrap()) as usize == c).filter_map(|k| l.param_at(l.users[u].calls[k].param_pos()).and_then(|p| p.get(RUNTIME_NAME)).and_then(|v| l.text(v))).collect();
            assert_eq!(kids, [RING_EFFECT, ORB_EFFECT, ORB_EFFECT, HEAL_EFFECT], "{name}: ring on the root, one orb per hand, column on the spine");
            assert!(l.users[u].calls[c].raw[0x28..0x30] == elink::NONE.to_le_bytes());
            for (_, effect, _) in PARTS {
                assert!(kids.contains(&effect));
            }
        }
    }

    #[test]
    fn enemy_variants_name_a_covered_effect_user() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let (_, now, _): (Link, Link, Vec<String>) = healed(&mut rf);
        let mut seen: usize = 0;
        for actor in ["Enemy_Stalfos", "Enemy_Keese_Fire", "Enemy_Keese_Ice", "Enemy_Stalfos_AlwaysRespawn", "Enemy_Freezard_Fixed", "Enemy_Leever_Fixed", "Enemy_ReDead_AlwaysRespawn"] {
            let es: Vec<Entry> = sarc::parse(&rf.read_zs(&format!("Pack/Actor/{actor}.pack.zs")).unwrap()).unwrap();
            for e in es.iter().filter(|e| e.name.starts_with("Component/ELink/")) {
                let b: crate::byml::Byml = crate::byml::Byml::parse(&e.data).unwrap();
                if let Some(user) = b.root.get("UserName").and_then(|n| n.as_str()) {
                    seen += 1;
                    assert!(now.user(user).is_some_and(|u| now.find_key(u, HEAL_KEY).is_some()), "{actor} plays through {user}, which has no heal key");
                }
            }
        }
        assert!(seen >= 1, "the packs name their effect users");
    }

    #[test]
    fn staging_writes_the_effect_link_once() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let none = |_: &str| -> Option<Vec<u8>> { None };
        let outs: Vec<Out> = link_stage(&mut rf, &none).unwrap();
        assert_eq!(outs.len(), 1);
        assert_eq!(outs[0].path, elink::FILE);
        assert!(outs[0].new > outs[0].old);
        let raw: Vec<u8> = rf.zs.decompress(&outs[0].z).unwrap();
        assert_eq!(raw.len(), outs[0].new);
        let l: Link = Link::parse(&raw).unwrap();
        assert!(l.find_key(l.user("Enemy_Lizalfos").unwrap(), HEAL_KEY).is_some());
        let mut t: crate::rsizetable::Rst = crate::rsizetable::load(&mut rf).unwrap();
        assert_eq!(crate::staging::grow_out(&mut rf, &mut t, &outs[0]).unwrap(), 1, "the size table grows with the file");
        let z: Vec<u8> = outs[0].z.clone();
        let have = move |p: &str| -> Option<Vec<u8>> { (p == elink::FILE).then(|| z.clone()) };
        let twice: Vec<Out> = link_stage(&mut rf, &have).unwrap();
        assert_eq!(rf.zs.decompress(&twice[0].z).unwrap(), raw, "a second pass changes nothing");
    }
}
