use crate::ainb::{start, Ainb};
use crate::byml::{Byml, Node};
use crate::chest_slot::LIST;
use crate::error::{Error as E, Res};
use crate::flow_locations::dict_id;
use crate::romfs::Romfs;
use crate::staging::{rewrite, Out};
use crate::start_defaults::{allowed, key};
use crate::start_items::give::{Kit, Set};
use crate::zs;

pub const MP: &str = "MP";
pub const MP_MAX: &str = "MPMax";

pub fn hash(name: &str) -> Res<u32> {
    match name.strip_prefix("0x") {
        Some(h) => u32::from_str_radix(h, 16).map_err(|_| E::Msg(format!("start items: {name} is not a hex flag hash"))),
        None => Ok(key(name)),
    }
}

fn num(n: &Node) -> Option<i64> {
    match n {
        Node::I32(x) => Some(*x as i64),
        Node::U32(x) => Some(*x as i64),
        Node::I64(x) => Some(*x),
        Node::U64(x) => i64::try_from(*x).ok(),
        _ => None,
    }
}

fn row<'a>(y: &'a mut Byml, ty: &str, name: &str) -> Res<&'a mut Node> {
    let h: u32 = hash(name)?;
    let r: &mut Node = y
        .root
        .get_mut("Data")
        .and_then(|d| d.get_mut(ty))
        .and_then(|t| t.as_arr_mut())
        .and_then(|a| a.iter_mut().find(|r| r.get("Hash").and_then(num).map(|x| x as u32) == Some(h)))
        .ok_or_else(|| E::Msg(format!("start items: the flag list has no {ty} flag named {name}")))?;
    let reset: i32 = r.get("ResetTypeValue").and_then(num).unwrap_or(-1) as i32;
    let save: i32 = r.get("SaveFileIndex").and_then(num).unwrap_or(-1) as i32;
    if !allowed(reset, save) {
        return Err(E::Msg(format!("start items: {name} has reset mask {reset} and save file {save}, so a new game default would not stick")));
    }
    Ok(r)
}

fn put(y: &mut Byml, ty: &str, name: &str, v: Node) -> Res<()> {
    let r: &mut Node = row(y, ty, name)?;
    let old: Option<&Node> = r.get("DefaultValue");
    if std::mem::discriminant(old.ok_or_else(|| E::Msg(format!("start items: {name} has no default")))?) != std::mem::discriminant(&v) {
        return Err(E::Msg(format!("start items: {name} has a default of another kind")));
    }
    r.set("DefaultValue", v);
    Ok(())
}

fn enum_value(y: &mut Byml, name: &str, v: &str) -> Res<()> {
    let r: &mut Node = row(y, "Enum", name)?;
    let names: Vec<&str> = r.get("RawValues").and_then(|x| x.as_arr()).unwrap_or(&[]).iter().filter_map(|x| x.as_str()).collect();
    if !names.contains(&v) {
        return Err(E::Msg(format!("start items: {name} has no value {v}")));
    }
    r.set("DefaultValue", Node::U32(key(v)));
    Ok(())
}

pub fn apply(y: &mut Byml, sets: &[Set]) -> Res<()> {
    for s in sets {
        match s {
            Set::Bool(n, v) => put(y, "Bool", n, Node::Bool(*v))?,
            Set::Int(n, v) => put(y, "Int", n, Node::I32(*v))?,
            Set::Float(n, v) => put(y, "Float", n, Node::F32(*v))?,
            Set::Str(n, v) => put(y, "String64", n, Node::Str(v.clone()))?,
            Set::Enum(n, v) => enum_value(y, n, v)?,
        }
    }
    Ok(())
}

fn edit_list(raw: &[u8], sets: &[Set]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    apply(&mut y, sets)?;
    y.write()
}

pub fn stage_list(rf: &mut Romfs, more: &mut Vec<Out>, sets: &[Set]) -> Res<()> {
    if sets.is_empty() {
        return Ok(());
    }
    match more.iter().position(|o| o.path == LIST) {
        Some(i) => {
            let id: Option<u32> = dict_id(&more[i].z);
            let raw: Vec<u8> = rf.zs.decompress(&more[i].z)?;
            let out: Vec<u8> = edit_list(&raw, sets)?;
            more[i].new = out.len();
            more[i].z = zs::compress_raw(&out, id)?;
        }
        None => more.push(rewrite(rf, LIST, |raw| edit_list(raw, sets))?),
    }
    Ok(())
}

pub fn patch_module(a: &mut Ainb, k: &Kit) -> Res<()> {
    if let Some(n) = k.hearts {
        start::set_hearts(a, n)?;
    }
    if let Some(m) = k.magic {
        start::set_game_int(a, MP_MAX, m)?;
        start::set_game_int(a, MP, m)?;
    }
    for p in &k.pushes {
        start::add_item(a, p, 1)?;
    }
    Ok(())
}

pub fn stage_module(rf: &mut Romfs, more: &mut Vec<Out>, k: &Kit) -> Res<()> {
    if !k.module() {
        return Ok(());
    }
    let (old, raw): (usize, Vec<u8>) = match more.iter().position(|o| o.path == start::MODULE) {
        Some(i) => (more[i].old, more.remove(i).z),
        None => {
            let r: Vec<u8> = rf.read(start::MODULE)?;
            (r.len(), r)
        }
    };
    let mut a: Ainb = Ainb::parse(&raw)?;
    patch_module(&mut a, k)?;
    let out: Vec<u8> = a.write()?;
    more.push(Out { path: start::MODULE.to_string(), old, new: out.len(), z: out });
    Ok(())
}

pub fn stage(rf: &mut Romfs, more: &mut Vec<Out>, k: &Kit) -> Res<()> {
    stage_list(rf, more, &k.sets)?;
    stage_module(rf, more, k)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, Value};
    use crate::start_items::give::read;
    use std::path::PathBuf as PB;

    fn real() -> Option<PB> {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    fn every() -> Settings {
        let mut s: Settings = Settings::default();
        let all: Vec<&str> = crate::start_items::table::OPTS.iter().map(|o| o.id).collect();
        let songs: Vec<&str> = crate::start_items::table::SONGS.iter().map(|o| o.0).collect();
        s.set("starting_items", Value::set(&all)).unwrap();
        s.set("starting_songs", Value::set(&songs)).unwrap();
        s.set("starting_hearts", Value::Int(9)).unwrap();
        s
    }

    fn rows(y: &Byml) -> Vec<(String, Node)> {
        let mut out: Vec<(String, Node)> = Vec::new();
        if let Some(Node::Map(m)) = y.root.get("Data") {
            for (k, v) in m {
                for r in v.as_arr().unwrap_or(&[]) {
                    out.push((k.clone(), r.clone()));
                }
            }
        }
        out
    }

    fn def_of(y: &Byml, ty: &str, name: &str) -> Option<Node> {
        let h: u32 = hash(name).ok()?;
        y.root.get("Data")?.get(ty)?.as_arr()?.iter().find(|r| r.get("Hash").and_then(num).map(|x| x as u32) == Some(h))?.get("DefaultValue").cloned()
    }

    #[test]
    fn a_built_spread_mod_carries_the_defaults_the_module_and_grown_size_entries() {
        use crate::rsizetable::{self, Rst};
        let Some(root) = real() else {
            return;
        };
        let d: crate::slice::data::Data = crate::slice::data::Data::load().unwrap();
        let mut s: Settings = Settings::default();
        s.set("starting_items", Value::set(&["longshot", "bottle", "biggest_quiver", "big_bomb_bag", "golden_scale", "giant_wallet", "double_magic", "ocarina_of_time", "kokiri_sword"])).unwrap();
        s.set("starting_songs", Value::set(&["song_of_time"])).unwrap();
        s.set("starting_hearts", Value::Int(6)).unwrap();
        let g = crate::slice::generate::generate_data(&d, &s, "start items build", &crate::slice::generate::GenOpts::default()).unwrap();
        let load: PB = PB::from(env!("CARGO_MANIFEST_DIR")).join("../../target/start_items_mods");
        let b = crate::slice::edit::build_for(&g.seed_file, &root, &load, "OOTRR Start Items Test", crate::modpack::Host::Hookshot).unwrap();
        let out: PB = b.root.join("romfs");
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let list: Byml = Byml::parse(&rf.zs.decompress(&std::fs::read(out.join(LIST)).unwrap()).unwrap()).unwrap();
        assert_eq!(def_of(&list, "Int", "0xe753508b"), Some(Node::I32(30)));
        assert_eq!(def_of(&list, "Int", "0x418db791"), Some(Node::I32(50)));
        assert_eq!(def_of(&list, "String64", "CurrentExpansionActor_Arrow"), Some(Node::Str("Item_Quiver_Large".to_string())));
        assert_eq!(def_of(&list, "String64", "PlayerStatus.EquipOcarina"), Some(Node::Str("Item_Ocarina_Time".to_string())));
        assert_eq!(def_of(&list, "Float", "PlayerStatus.DiveDepth"), Some(Node::F32(9.0)));
        assert_eq!(def_of(&list, "Bool", "IsGet.Item_Longshot"), Some(Node::Bool(true)));
        assert_eq!(def_of(&list, "Bool", "IsGet.Item_Melody_TimeSong"), Some(Node::Bool(true)));
        assert_eq!(def_of(&list, "Bool", "PlayerStatus.AssignableItem.BottleInfo.BottleA.IsValid"), Some(Node::Bool(true)));
        let m: Ainb = Ainb::parse(&std::fs::read(out.join(start::MODULE)).unwrap()).unwrap();
        assert_eq!(start::hearts(&m), Some(6));
        assert_eq!(start::game_int(&m, MP_MAX), Some(480));
        let pushed: Vec<String> = start::items(&m).into_iter().map(|i| i.actor).collect();
        assert!(pushed.iter().any(|a| a.contains("/Item_Wallet_Large.")) && pushed.iter().any(|a| a.contains("/Weapon_Sword_Kokiri.")));
        let van: Rst = rsizetable::load(&mut rf).unwrap();
        let ours: Rst = Rst::parse(&rf.zs.decompress(&std::fs::read(out.join(rsizetable::TABLE_PATH)).unwrap()).unwrap()).unwrap();
        let raw_list: usize = rf.read_zs(LIST).unwrap().len();
        let new_list: usize = rf.zs.decompress(&std::fs::read(out.join(LIST)).unwrap()).unwrap().len();
        let raw_mod: usize = rf.read(start::MODULE).unwrap().len();
        let new_mod: usize = std::fs::read(out.join(start::MODULE)).unwrap().len();
        assert!(new_list > raw_list && new_mod > raw_mod, "{raw_list} {new_list} {raw_mod} {new_mod}");
        for (p, a, z) in [(LIST, raw_list, new_list), (start::MODULE, raw_mod, new_mod)] {
            let (Some(v), Some(o)) = (van.get(p), ours.get(p)) else {
                panic!("{p} has no size entry");
            };
            assert!(o as u64 >= v as u64 + rsizetable::align(z as u64) - rsizetable::align(a as u64), "{p} {v} {o}");
        }
        assert!(b.report.contains("The chosen starting items"));
    }

    #[test]
    fn hex_names_and_plain_names_hash() {
        assert_eq!(hash("0xe753508b").unwrap(), 0xe753508b);
        assert_eq!(hash("PlayerStatus.MP").unwrap(), 0x76b18200);
        assert!(hash("0xzz").is_err());
    }

    #[test]
    fn every_flag_of_every_choice_is_a_new_game_flag_of_the_right_type_in_the_dump() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read_zs(LIST).unwrap();
        let orig: Byml = Byml::parse(&raw).unwrap();
        let mut bad: Vec<String> = Vec::new();
        let k: Kit = read(&every(), &mut bad);
        assert!(bad.is_empty(), "{bad:?}");
        let mut y: Byml = orig.clone();
        apply(&mut y, &k.sets).unwrap();
        let out: Vec<u8> = y.write().unwrap();
        let back: Byml = Byml::parse(&out).unwrap();
        assert_eq!(back.root, y.root);
        let (a, b): (Vec<(String, Node)>, Vec<(String, Node)>) = (rows(&orig), rows(&back));
        assert_eq!(a.len(), b.len());
        let changed: usize = a.iter().zip(b.iter()).filter(|(x, z)| x != z).count();
        let names: std::collections::BTreeSet<u32> = k
            .sets
            .iter()
            .map(|s| match s {
                Set::Bool(n, _) | Set::Int(n, _) | Set::Enum(n, _) | Set::Str(n, _) | Set::Float(n, _) => hash(n).unwrap(),
            })
            .collect();
        assert_eq!(changed, names.len());
        for (x, z) in a.iter().zip(b.iter()) {
            if x != z {
                assert_eq!(x.1.get("Hash"), z.1.get("Hash"));
                assert_eq!(x.1.get("ResetTypeValue"), z.1.get("ResetTypeValue"));
            }
        }
    }

    #[test]
    fn the_start_module_takes_hearts_magic_and_wallets_after_the_vanilla_wallet() {
        let Some(root) = real() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read(start::MODULE).unwrap();
        let mut a: Ainb = Ainb::parse(&raw).unwrap();
        assert_eq!(start::game_int(&a, MP_MAX), Some(0));
        assert_eq!(start::game_int(&a, MP), Some(0));
        let mut bad: Vec<String> = Vec::new();
        let k: Kit = read(&every(), &mut bad);
        patch_module(&mut a, &k).unwrap();
        let c: Ainb = Ainb::parse(&a.write().unwrap()).unwrap();
        assert!(crate::ainb::audit(&c).is_empty());
        assert_eq!(start::hearts(&c), Some(9));
        assert_eq!(start::game_int(&c, MP_MAX), Some(480));
        assert_eq!(start::game_int(&c, MP), Some(480));
        let items: Vec<start::Item> = start::items(&c);
        let el = |n: &str| items.iter().find(|x| x.actor.contains(&format!("/{n}."))).map(|x| x.el).unwrap_or_else(|| panic!("{n}"));
        let order = |e: usize| -> (usize, usize) {
            (0..c.els.len()).find_map(|p| c.params[p].group(2).iter().position(|g| g.a as usize == e).map(|j| (p, j))).unwrap()
        };
        let (ps, js): (usize, usize) = order(el("Item_Wallet_Small"));
        let (pm, jm): (usize, usize) = order(el("Item_Wallet_Medium"));
        let (pl, jl): (usize, usize) = order(el("Item_Wallet_Large"));
        assert!(ps == pm && pm == pl, "{ps} {pm} {pl}");
        assert!(js < jm && jm < jl, "{js} {jm} {jl}");
    }
}
