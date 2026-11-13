use crate::error::{Error as E, Res};
use crate::progressive::{owners, tiers};

pub const MODULE: &str = "AI/_LocalModule_f62224b7f673.module.ainb";

pub const PREGRANT: &[(&str, &[&str])] = &[
    ("Weapon_Range_SlingShot", &["Item_BulletBag_Small", "Item_DekuSeed"]),
    ("Item_BombBag_Small", &["Item_BombBag_Small", "Item_Bomb"]),
    ("Item_DekuStickBundle_Small", &["Item_StickBag_Small"]),
    ("Item_DekuNut", &["Item_NutBag_Small"]),
    ("Weapon_Range_Bow", &["Item_Quiver_Small", "Item_Arrow_Normal"]),
];

pub fn pregrant(identity: &str) -> &'static [&'static str] {
    PREGRANT.iter().find(|p| p.0 == identity).map(|p| p.1).unwrap_or(&[])
}

pub fn clashes(family: &str, tier: usize) -> Res<bool> {
    let o: &[&str] = owners(family)?;
    let id: &str = o.get(tier).ok_or_else(|| E::Msg(format!("{family} has no tier {tier}")))?;
    Ok(pregrant(id).iter().any(|g| o.contains(g)))
}

pub fn chest_drop(family: &str) -> Res<&'static str> {
    let t: &[&str] = tiers(family)?;
    for (i, a) in t.iter().enumerate() {
        if !clashes(family, i)? {
            return Ok(a);
        }
    }
    Err(E::Msg(format!("every tier of {family} triggers the chest pre-grant")))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet as BS;
    use std::path::PathBuf as PB;

    use super::*;
    use crate::ainb::show::show;
    use crate::ainb::Ainb;
    use crate::progressive::FAMILIES;
    use crate::romfs::Romfs;
    use crate::sarc::{self, Entry};

    #[test]
    fn the_bomb_bag_chest_holds_a_tier_the_pre_grant_ignores() {
        assert!(clashes("BombBag", 0).unwrap());
        assert_eq!(chest_drop("BombBag").unwrap(), "Item_BombBag_Medium");
        assert!(pregrant("Item_BombBag_Medium").is_empty());
    }

    #[test]
    fn every_other_family_keeps_its_first_tier_in_the_chest() {
        for (f, t) in FAMILIES {
            if *f == "BombBag" {
                continue;
            }
            assert_eq!(chest_drop(f).unwrap(), t[0], "{f}");
        }
        assert!(!clashes("Slingshot", 0).unwrap());
        assert!(!clashes("Bow", 0).unwrap());
        assert!(clashes("Bow", 9).is_err());
    }

    #[test]
    fn the_table_matches_the_module_in_the_dump() {
        let p: PB = std::env::var_os("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if !p.join("Pack").join("ZsDic.pack.zs").is_file() {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            return;
        }
        let mut rf: Romfs = Romfs::open(&p).unwrap();
        let raw: Vec<u8> = rf.read_zs("Pack/AI.Global.Product.111.pack.zs").unwrap();
        let es: Vec<Entry> = sarc::parse(&raw).unwrap();
        let e: &Entry = sarc::get(&es, MODULE).unwrap();
        let t: String = show(&Ainb::parse(&e.data).unwrap());
        let mut got: BS<String> = BS::new();
        for l in t.lines().map(|l| l.trim()) {
            if let Some(r) = l.strip_prefix("in str Actor = \"Work/Actor/") {
                got.insert(r.split('.').next().unwrap().to_string());
            }
        }
        let want: BS<String> = PREGRANT.iter().flat_map(|p| p.1.iter().map(|s| s.to_string())).collect();
        assert_eq!(got, want);
        assert!(t.contains("in str GameDataFullName = \"IsGet\""));
        assert!(t.contains("QueryActorNameFromActorFileName"));
    }

    #[test]
    fn no_chest_drop_is_pre_granted_into_its_own_chain() {
        for (f, _) in FAMILIES {
            let d: &str = chest_drop(f).unwrap();
            let i: usize = tiers(f).unwrap().iter().position(|x| *x == d).unwrap();
            let o: &[&str] = owners(f).unwrap();
            assert!(pregrant(o[i]).iter().all(|g| !o.contains(g)), "{f}");
        }
    }
}
