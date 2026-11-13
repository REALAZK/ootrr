use std::collections::BTreeSet as BS;

use crate::error::{Error as E, Res};
use crate::pickup_pack::actor::{make_with, Made};
use crate::pickup_pack::closure::{chain, dangling_with, is_byml, select, Chain, Pool};
use crate::pickup_pack::{is_actor_name, Layers, Spec};
use crate::sarc::{self, Entry};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub spec: Spec,
    pub files: Vec<String>,
    pub fresh: Vec<(String, usize)>,
    pub peers: Vec<(String, usize)>,
    pub sarc: Vec<u8>,
    pub identity: String,
}

pub fn pool_of(packs: &[Vec<Entry>]) -> Res<Pool> {
    let mut p: Pool = Pool::default();
    for es in packs {
        p.add_pack(es)?;
    }
    Ok(p)
}

pub fn plan(packs: &[Vec<Entry>], s: &Spec) -> Res<Plan> {
    plan_with(packs, s, &Layers::default())
}

pub fn plan_with(packs: &[Vec<Entry>], s: &Spec, l: &Layers) -> Res<Plan> {
    if !is_actor_name(&s.name) || !is_actor_name(&s.item) {
        return Err(E::Msg(format!("{} or {} is not a plain actor name", s.name, s.item)));
    }
    let mut p: Pool = pool_of(packs)?;
    if let Some((f, _)) = p.files.iter().find(|e| !is_byml(e.1)) {
        return Err(E::Msg(format!("{f} is not a parameter file, an item whose pack carries graph or binary files needs a hand made template")));
    }
    let peers: Vec<(String, usize)> = p.files.iter().map(|e| (e.0.clone(), e.1.len())).collect();
    let m: Made = make_with(&p, s, l)?;
    let mut fresh: Vec<(String, usize)> = Vec::new();
    for (f, d) in [&m.actor, &m.pouch].into_iter().chain(m.extra.iter()) {
        if p.get(f).is_some() {
            return Err(E::Msg(format!("{f} already exists, {} is taken", s.name)));
        }
        p.add(f, d)?;
        fresh.push((f.clone(), d.len()));
    }
    let files: Vec<String> = select(&p, &s.name)?;
    if let Some((f, _)) = fresh.iter().find(|f| !files.contains(&f.0)) {
        return Err(E::Msg(format!("{f} did not make it into the pack")));
    }
    let c: Chain = chain(&p, &s.name)?;
    let known: BS<String> = p.files.keys().cloned().collect();
    if let Some(f) = dangling_with(&p, &files, &known, &c)?.first() {
        return Err(E::Msg(format!("{f} is referenced but left out of the pack")));
    }
    let es: Vec<Entry> = files.iter().map(|f| Entry { name: f.clone(), data: p.get(f).unwrap_or(&[]).to_vec() }).collect();
    Ok(Plan { spec: s.clone(), files, fresh, peers, sarc: sarc::write(&es)?, identity: m.identity })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf as PB;

    use super::*;
    use crate::byml::{Byml, Node};
    use crate::pickup_pack::actor::tests::{pool, spec};
    use crate::pickup_pack::actor::{pouch_file, AUTO_GET, FIRST_CHECK, MODEL_REF, POUCH_ACTOR};
    use crate::pickup_pack::closure::{actor_file, texts, COMPONENTS, PARENT};
    use crate::pickup_pack::{pack_of, Style, BASE, DONOR};
    use crate::romfs::Romfs;

    const FAMILY: [&str; 15] = [
        "Obj_Armor_Goron",
        "Obj_Armor_Zora",
        "Obj_ArrowBundle_Small",
        "Obj_Arrow_Fire",
        "Obj_Bomb",
        "Obj_Bombchu",
        "Obj_DekuNut",
        "Obj_DekuSeed",
        "Obj_DekuStick",
        "Obj_Item_Base",
        "Obj_MagicJar_Small",
        "Obj_Ocarina_Time",
        "Obj_ScrubBusinessSeed",
        "Obj_Shield_Deku",
        "Obj_Shield_Hylian",
    ];

    fn packs() -> Vec<Vec<Entry>> {
        vec![pool().files.into_iter().map(|e| Entry { name: e.0, data: e.1 }).collect()]
    }

    fn real() -> Option<PB> {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    fn load(rf: &mut Romfs, name: &str) -> Vec<Entry> {
        sarc::parse(&rf.read_zs(&pack_of(name)).unwrap()).unwrap()
    }

    #[test]
    fn the_pack_holds_the_base_the_item_parts_and_the_two_new_files() {
        let pl: Plan = plan(&packs(), &spec(Style::Touch)).unwrap();
        let want: Vec<&str> = vec![
            "Actor/Obj_Item_Base.engine__actor__ActorParam.bgyml",
            "Actor/Obj_Test.engine__actor__ActorParam.bgyml",
            "Component/ASInfo/Item_Obj.engine__component__ASInfo.bgyml",
            "Component/AnimationParam/Item_Test.engine__component__AnimationParam.bgyml",
            "Component/Attention/Auto.game__component__AttentionSenderParam.bgyml",
            "Component/ModelInfo/Item_Test.engine__component__ModelInfo.bgyml",
            "Component/Physics/Obj_Item_Base.engine__component__PhysicsParam.bgyml",
            "Component/PouchContentParam/Item_Test.game__component__PouchContentParam.bgyml",
            "Component/PouchContentParam/Obj_Test.game__component__PouchContentParam.bgyml",
        ];
        assert_eq!(pl.files, want);
        assert_eq!(pl.fresh.len(), 2);
        let back: Vec<Entry> = sarc::parse(&pl.sarc).unwrap();
        assert_eq!(back.len(), want.len());
        assert!(sarc::get(&back, "Actor/Item_Test.engine__actor__ActorParam.bgyml").is_none());
        assert!(sarc::get(&back, "Component/CaptureParam/Item_Test.game__component__CaptureParam.bgyml").is_none());
        assert!(sarc::get(&back, "Component/ModelInfo/Dummy.engine__component__ModelInfo.bgyml").is_none());
    }

    #[test]
    fn the_press_style_swaps_the_attention_file() {
        let pl: Plan = plan(&packs(), &spec(Style::Press)).unwrap();
        assert!(pl.files.iter().any(|f| f.contains("Attention/Press")));
        assert!(!pl.files.iter().any(|f| f.contains("Attention/Auto")));
    }

    #[test]
    fn a_taken_name_a_bad_name_and_a_binary_file_are_refused() {
        let mut taken: Spec = spec(Style::Touch);
        taken.name = "Item_Test".to_string();
        assert!(plan(&packs(), &taken).is_err());
        let mut odd: Spec = spec(Style::Touch);
        odd.name = "Obj/Test".to_string();
        assert!(plan(&packs(), &odd).is_err());
        let mut with_bin: Vec<Vec<Entry>> = packs();
        with_bin[0].push(Entry { name: "AI/Graph.root.ainb".to_string(), data: b"AIB ".to_vec() });
        assert!(plan(&with_bin, &spec(Style::Touch)).is_err());
    }

    #[test]
    fn the_real_source_packs_round_trip_byte_for_byte() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut files: usize = 0;
        let mut odd: usize = 0;
        for name in [BASE, DONOR, "Item_Hookshot", "Obj_Ocarina_Time", "Obj_Rupee_Green"] {
            let raw: Vec<u8> = rf.read_zs(&pack_of(name)).unwrap();
            let es: Vec<Entry> = sarc::parse(&raw).unwrap();
            assert_eq!(sarc::write(&es).unwrap(), raw, "{name} archive");
            for e in es.iter().filter(|e| is_byml(&e.data)) {
                files += 1;
                match Byml::parse(&e.data) {
                    Ok(y) => assert_eq!(y.write().unwrap(), e.data, "{name} {}", e.name),
                    Err(_) => {
                        odd += 1;
                        assert!(!texts(&e.data).unwrap().is_empty(), "{name} {}", e.name);
                    }
                }
            }
        }
        assert_eq!((files, odd), (193, 4));
    }

    #[test]
    fn the_file_rule_rebuilds_the_real_pickup_packs_exactly() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut all: Vec<Vec<Entry>> = Vec::new();
        for name in FAMILY.iter().chain(["Item_Hookshot", "Item_Arrow_Fire", "Item_Ocarina_Time"].iter()) {
            all.push(load(&mut rf, name));
        }
        let p: Pool = pool_of(&all).unwrap();
        for (i, name) in FAMILY.iter().enumerate() {
            let mut want: Vec<String> = all[i].iter().map(|e| e.name.clone()).collect();
            want.sort();
            assert_eq!(select(&p, name).unwrap(), want, "{name}");
        }
        for (i, name) in ["Item_Hookshot", "Item_Arrow_Fire", "Item_Ocarina_Time"].iter().enumerate() {
            let mut want: Vec<String> = all[FAMILY.len() + i].iter().map(|e| e.name.clone()).collect();
            want.sort();
            assert_eq!(select(&p, name).unwrap(), want, "{name}");
        }
    }

    #[test]
    fn the_real_hookshot_pickup_is_the_base_pack_with_five_changes() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let src: Vec<Vec<Entry>> = vec![load(&mut rf, BASE), load(&mut rf, DONOR), load(&mut rf, "Item_Hookshot")];
        let s: Spec = Spec { item: "Item_Hookshot".to_string(), name: "Obj_Hookshot".to_string(), style: Style::Touch, heap: None };
        let pl: Plan = plan(&src, &s).unwrap();
        let base: BS<String> = src[0].iter().map(|e| e.name.clone()).collect();
        let got: BS<String> = pl.files.iter().cloned().collect();
        let gone: Vec<&String> = base.difference(&got).collect();
        let added: Vec<&String> = got.difference(&base).collect();
        assert_eq!(
            gone,
            vec![
                "Component/ModelInfo/Dummy.engine__component__ModelInfo.bgyml",
                "Component/PouchContentParam/Obj_Item_Base.game__component__PouchContentParam.bgyml"
            ]
        );
        assert_eq!(
            added,
            vec![
                "Actor/Obj_Hookshot.engine__actor__ActorParam.bgyml",
                "Component/ASInfo/Item_Obj.engine__component__ASInfo.bgyml",
                "Component/AnimationParam/Item_Hookshot.engine__component__AnimationParam.bgyml",
                "Component/ModelInfo/Item_Hookshot.engine__component__ModelInfo.bgyml",
                "Component/PouchContentParam/Item_Hookshot.game__component__PouchContentParam.bgyml",
                "Component/PouchContentParam/Obj_Hookshot.game__component__PouchContentParam.bgyml"
            ]
        );
        assert_eq!(pl.files.len(), 40);
        let back: Vec<Entry> = sarc::parse(&pl.sarc).unwrap();
        for e in &back {
            let from: Option<&Entry> = src.iter().find_map(|es| sarc::get(es, &e.name));
            match from {
                Some(x) => assert_eq!(x.data, e.data, "{}", e.name),
                None => assert!(pl.fresh.iter().any(|f| f.0 == e.name), "{}", e.name),
            }
        }
        let a: Byml = Byml::parse(&sarc::get(&back, &actor_file("Obj_Hookshot")).unwrap().data).unwrap();
        assert_eq!(a.root.get(PARENT).and_then(|v| v.as_str()), Some("Work/Actor/Obj_Item_Base.engine__actor__ActorParam.gyml"));
        let c: &Node = a.root.get(COMPONENTS).unwrap();
        assert_eq!(c.get("ActorNameRef").and_then(|v| v.as_str()), Some("?ActorSystem/ActorName/Item_Hookshot.engine__actor__ActorName.bgyml"));
        assert_eq!(c.get(MODEL_REF).and_then(|v| v.as_str()), Some("?Component/ModelInfo/Item_Hookshot.engine__component__ModelInfo.bgyml"));
        assert_eq!(c.get("ASRef").and_then(|v| v.as_str()), Some("?AS/Item_Common.root.asb"));
        assert!(c.get("PhysicsRef").is_none() && c.get("DeathRef").is_none() && c.get("ActorGameDataSetting").is_none());
        let q: Byml = Byml::parse(&sarc::get(&back, &pouch_file("Obj_Hookshot")).unwrap().data).unwrap();
        assert_eq!(q.root.get(PARENT).and_then(|v| v.as_str()), Some("Work/Component/PouchContentParam/Item_Hookshot.game__component__PouchContentParam.gyml"));
        assert_eq!(q.root.get(POUCH_ACTOR).and_then(|v| v.as_str()), Some("Work/Actor/Item_Hookshot.engine__actor__ActorParam.gyml"));
        assert_eq!(q.root.get(FIRST_CHECK), Some(&Node::Bool(false)));
        assert_eq!(q.root.get(AUTO_GET), Some(&Node::Bool(true)));
        let shipped: Byml = Byml::parse(&sarc::get(&src[1], &actor_file(DONOR)).unwrap().data).unwrap();
        assert_eq!(a.version, shipped.version);
        let press: Plan = plan(&src, &Spec { style: Style::Press, ..s }).unwrap();
        assert!(press.files.iter().any(|f| f == "Component/AttentionSenderParam/PickUpItem.game__component__AttentionSenderParam.bgyml"));
        assert!(press.files.iter().any(|f| f == "Attention/ClientParam/Item_Catch.game__attention__ClientParam.bgyml"));
        assert!(!press.files.iter().any(|f| f.contains("AutoPickUpItem")));
    }

    fn pouch_of(rf: &mut Romfs, item: &str, name: &str) -> (Plan, Option<String>, Option<String>) {
        let src: Vec<Vec<Entry>> = vec![load(rf, BASE), load(rf, DONOR), load(rf, item)];
        let s: Spec = Spec { item: item.to_string(), name: name.to_string(), style: Style::Touch, heap: None };
        let pl: Plan = plan(&src, &s).unwrap();
        let back: Vec<Entry> = sarc::parse(&pl.sarc).unwrap();
        let q: Byml = Byml::parse(&sarc::get(&back, &pouch_file(name)).unwrap().data).unwrap();
        let a: Byml = Byml::parse(&sarc::get(&src[2], &actor_file(item)).unwrap().data).unwrap();
        let named: Option<String> = a.root.get(COMPONENTS).and_then(|c| c.get("ActorNameRef")).and_then(|v| v.as_str()).map(|v| v.to_string());
        (pl, q.root.get(POUCH_ACTOR).and_then(|v| v.as_str()).map(|v| v.to_string()), named)
    }

    #[test]
    fn gear_packs_credit_the_gear_identity_and_not_the_item_actor() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let cases: [(&str, &str, &str); 6] = [
            ("Item_HeavyBoot", "Obj_HeavyBoot", "Armor_Adult_HeavyBoot"),
            ("Item_HoverBoot", "Obj_HoverBoot", "Armor_Adult_HoverBoot"),
            ("Item_Shield_Mirror", "Obj_Shield_Mirror", "Weapon_Shield_Mirror"),
            ("Item_Sword_Kokiri", "Obj_Sword_Kokiri", "Weapon_Sword_Kokiri"),
            ("Item_Hookshot", "Obj_Hookshot", "Item_Hookshot"),
            ("Item_Boomerang", "Obj_Boomerang", "Item_Boomerang"),
        ];
        for (item, name, who) in cases {
            let (pl, pouch, named): (Plan, Option<String>, Option<String>) = pouch_of(&mut rf, item, name);
            assert_eq!(pl.identity, who, "{item}");
            assert_eq!(pouch, Some(format!("Work/Actor/{who}.engine__actor__ActorParam.gyml")), "{item}");
            assert_eq!(named, Some(format!("?ActorSystem/ActorName/{who}.engine__actor__ActorName.bgyml")), "{item}");
            assert!(rf.root.join(pack_of(who)).is_file(), "{who}");
        }
        let mirror: Vec<Entry> = load(&mut rf, "Item_Shield_Mirror");
        let own: Byml = Byml::parse(&sarc::get(&mirror, &pouch_file("Weapon_Shield_Mirror")).unwrap().data).unwrap();
        assert_eq!(own.root.get(POUCH_ACTOR).and_then(|v| v.as_str()), Some("Work/Actor/Weapon_Shield_Mirror.engine__actor__ActorParam.gyml"));
    }
}
