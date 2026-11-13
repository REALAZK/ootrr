use std::fs;
use std::path::Path as P;

use crate::error::{Error, Res};
use crate::modpack::rel;
use crate::tex::bntx::{self, Tex};
use crate::tex::image::Img;
use crate::tex::png;
use crate::zs::Zs;

pub const VERSION: u32 = 1;
pub const ICON_DIR: &str = "UI/Tex/Icon";
pub const COMMON: &str = "UI/Layout2Archive/Common.Product.100.Nin_Ounce_NVN2.bntx.zs";
pub const MAX_SIDE: usize = 144;

const GREEN: Option<[u8; 3]> = Some([112, 214, 110]);
const RED: Option<[u8; 3]> = Some([240, 104, 84]);
const BLUE: Option<[u8; 3]> = Some([110, 160, 250]);
const ORANGE: Option<[u8; 3]> = Some([244, 160, 82]);
const PURPLE: Option<[u8; 3]> = Some([184, 128, 240]);
const YELLOW: Option<[u8; 3]> = Some([240, 220, 100]);

pub const OWN: [&str; 46] = [
    "Item_DekuStick",
    "Item_MagicBean",
    "Item_Bomb",
    "Weapon_Range_SlingShot",
    "Item_Boomerang",
    "Item_Bombchu",
    "Weapon_Range_Bow",
    "Item_Arrow_Fire_PouchIcon",
    "Item_Arrow_Ice_PouchIcon",
    "Item_Arrow_Light_PouchIcon",
    "Item_Hammer_PouchIcon",
    "Item_Hookshot",
    "Item_Longshot",
    "Item_DekuNut",
    "Item_DinFire",
    "Item_FaroreWind",
    "Item_NayruLove",
    "Item_Ocarina_Fairy",
    "Item_Ocarina_Time",
    "Item_Bottle",
    "Weapon_Sword_Kokiri",
    "Weapon_Sword_Master",
    "Weapon_Sword_Giant",
    "Weapon_Sword_Biggoron",
    "Item_Bottle_GreenPotion",
    "Item_Wallet_Small",
    "Item_Wallet_Medium",
    "Item_Wallet_Large",
    "Armor_GlassTruth",
    "Item_Scale_Silver",
    "Item_Scale_Gold",
    "Weapon_Shield_Deku",
    "Weapon_Shield_Hylian",
    "Weapon_Shield_Mirror",
    "Armor_Adult_HeavyBoot",
    "Item_Token_Gerudo",
    "Armor_Adult_Normal",
    "Armor_Adult_Goron",
    "Armor_Adult_Zora",
    "Armor_Adult_HoverBoot",
    "Armor_Child_Gauntlet_Goron",
    "Armor_Adult_Gauntlet_Silver",
    "Armor_Adult_Gauntlet_Gold",
    "Item_Token_Gold",
    "Item_Dungeon_SmallKey",
    "Item_Dungeon_BossKey",
];

pub const SHARED: [(&str, &str, Option<[u8; 3]>); 22] = [
    ("Item_Stone_Forest", "DataFileImportantIcon_00", None),
    ("Item_Stone_Fire", "DataFileImportantIcon_01", None),
    ("Item_Stone_Water", "DataFileImportantIcon_02", None),
    ("Item_Medal_Light", "DataFileImportantIcon_03", None),
    ("Item_Medal_Forest", "DataFileImportantIcon_04", None),
    ("Item_Medal_Fire", "DataFileImportantIcon_05", None),
    ("Item_Medal_Water", "DataFileImportantIcon_06", None),
    ("Item_Medal_Spirit", "DataFileImportantIcon_07", None),
    ("Item_Medal_Shadow", "DataFileImportantIcon_08", None),
    ("Obj_Heart_Piece", "CollectHeartBase_00", None),
    ("Item_Melody_ZeldaLullaby", "OcarinaMelodyIcon_00", None),
    ("Item_Melody_EponaSong", "OcarinaMelodyIcon_01", None),
    ("Item_Melody_SariaSong", "OcarinaMelodyIcon_02", None),
    ("Item_Melody_SunSong", "OcarinaMelodyIcon_03", None),
    ("Item_Melody_TimeSong", "OcarinaMelodyIcon_04", None),
    ("Item_Melody_StormSong", "OcarinaMelodyIcon_05", None),
    ("Item_Melody_ForestMinuet", "OcarinaMelodyIcon_07", GREEN),
    ("Item_Melody_FireBolero", "OcarinaMelodyIcon_08", RED),
    ("Item_Melody_WaterSerenade", "OcarinaMelodyIcon_09", BLUE),
    ("Item_Melody_SpiritRequiem", "OcarinaMelodyIcon_10", ORANGE),
    ("Item_Melody_ShadowNocturne", "OcarinaMelodyIcon_11", PURPLE),
    ("Item_Melody_LightPrelude", "OcarinaMelodyIcon_12", YELLOW),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Src {
    pub id: &'static str,
    pub file: String,
    pub tex: &'static str,
    pub tint: Option<[u8; 3]>,
}

pub fn all() -> Vec<Src> {
    let mut out: Vec<Src> = Vec::with_capacity(OWN.len() + SHARED.len());
    for id in OWN {
        out.push(Src { id, file: format!("{ICON_DIR}/{id}.bntx.zs"), tex: id, tint: None });
    }
    for (id, tex, tint) in SHARED {
        out.push(Src { id, file: COMMON.to_string(), tex, tint });
    }
    out
}

pub fn find(id: &str) -> Option<Src> {
    all().into_iter().find(|s| s.id == id)
}

pub fn render(b: &[u8], ts: &[Tex], s: &Src) -> Res<Vec<u8>> {
    let t: &Tex = bntx::find(ts, s.tex).ok_or_else(|| Error::Msg(format!("the game file {} has no texture {}", s.file, s.tex)))?;
    let (im, failed): (Img, usize) = bntx::decode(b, t)?;
    if failed > 0 {
        return Err(Error::Msg(format!("texture {} has {failed} blocks that could not be read", s.tex)));
    }
    let mut small: Img = im.shrink(MAX_SIDE);
    if let Some(c) = s.tint {
        small.tint(c);
    }
    Ok(png::encode(&small))
}

pub fn render_file(root: &P, file: &str) -> Res<Vec<(&'static str, Res<Vec<u8>>)>> {
    let want: Vec<Src> = all().into_iter().filter(|s| s.file == file).collect();
    if want.is_empty() {
        return Err(Error::Msg(format!("no icon comes from {file}")));
    }
    let packed: Vec<u8> = fs::read(root.join(rel(file)?)).map_err(|_| Error::Msg(format!("the game folder has no {file}")))?;
    let b: Vec<u8> = Zs::new().decompress(&packed)?;
    let ts: Vec<Tex> = bntx::parse(&b)?;
    Ok(want.iter().map(|s| (s.id, render(&b, &ts, s))).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::PathBuf as PB;

    #[test]
    fn icon_ids_are_unique_safe_names() {
        let a: Vec<Src> = all();
        assert_eq!(a.len(), 68);
        let ids: HashSet<&str> = a.iter().map(|s| s.id).collect();
        assert_eq!(ids.len(), a.len());
        for s in &a {
            assert!(s.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'), "{}", s.id);
            assert!(s.file.starts_with("UI/") && s.file.ends_with(".bntx.zs") && !s.file.contains(".."), "{}", s.file);
        }
        let texes: HashSet<&str> = SHARED.iter().map(|s| s.1).collect();
        assert_eq!(texes.len(), SHARED.len());
    }

    #[test]
    fn find_knows_both_kinds_of_source() {
        let own: Src = find("Item_Bomb").unwrap();
        assert_eq!((own.file.as_str(), own.tex), ("UI/Tex/Icon/Item_Bomb.bntx.zs", "Item_Bomb"));
        let shared: Src = find("Item_Melody_FireBolero").unwrap();
        assert_eq!((shared.file.as_str(), shared.tex), (COMMON, "OcarinaMelodyIcon_08"));
        assert!(shared.tint.is_some());
        assert!(find("Item_Nope").is_none());
        assert!(find("../Item_Bomb").is_none());
    }

    #[test]
    fn render_shrinks_tints_and_reports_a_missing_texture() {
        use crate::tex::bntx::fixture::{Fake, build};
        fn white(_: usize, _: usize) -> [u8; 4] {
            [255, 255, 255, 255]
        }
        let b: Vec<u8> = build(&[Fake { name: "OcarinaMelodyIcon_08", fmt: bntx::ASTC_4X4_SRGB, w: 300, h: 200, layout: 2, comp: [2, 3, 4, 5], rgba: white }]);
        let ts: Vec<Tex> = bntx::parse(&b).unwrap();
        let p: Vec<u8> = render(&b, &ts, &find("Item_Melody_FireBolero").unwrap()).unwrap();
        assert!(png::looks_ok(&p));
        assert_eq!(&p[16..24], &[0, 0, 0, 100, 0, 0, 0, 67]);
        let at: usize = 8 + 25 + 8 + 2 + 5 + 1;
        assert_eq!(&p[at..at + 4], &[240, 104, 84, 255]);
        assert!(render(&b, &ts, &find("Item_Melody_SunSong").unwrap()).is_err());
    }

    #[test]
    fn render_file_reads_a_packed_file_from_a_folder() {
        use crate::tex::bntx::fixture::{Fake, build};
        fn red(_: usize, _: usize) -> [u8; 4] {
            [200, 0, 0, 255]
        }
        let d: PB = std::env::temp_dir().join(format!("ootrr_icon_map_{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("UI").join("Tex").join("Icon")).unwrap();
        let b: Vec<u8> = build(&[Fake { name: "Item_Bomb", fmt: bntx::ASTC_4X4_SRGB, w: 16, h: 16, layout: 0, comp: [2, 3, 4, 5], rgba: red }]);
        fs::write(d.join("UI").join("Tex").join("Icon").join("Item_Bomb.bntx.zs"), crate::zs::compress_raw(&b, None).unwrap()).unwrap();
        let got: Vec<(&str, Res<Vec<u8>>)> = render_file(&d, "UI/Tex/Icon/Item_Bomb.bntx.zs").unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0, "Item_Bomb");
        assert!(png::looks_ok(got[0].1.as_ref().unwrap()));
        assert!(render_file(&d, "UI/Tex/Icon/Item_Hookshot.bntx.zs").is_err());
        assert!(render_file(&d, "UI/Tex/Icon/Nope.bntx.zs").is_err());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn every_icon_decodes_from_a_real_dump_when_one_is_given() {
        let Ok(r) = std::env::var("OOTRR_TEST_ROMFS") else {
            return;
        };
        let root: PB = PB::from(r);
        let mut files: Vec<String> = all().into_iter().map(|s| s.file).collect();
        files.sort();
        files.dedup();
        let mut n: usize = 0;
        for f in files {
            for (id, p) in render_file(&root, &f).unwrap() {
                let p: Vec<u8> = p.unwrap_or_else(|e| panic!("{id}: {e}"));
                assert!(png::looks_ok(&p), "{id}");
                let w: u32 = u32::from_be_bytes([p[16], p[17], p[18], p[19]]);
                assert!((40..=MAX_SIDE as u32).contains(&w), "{id} {w}");
                n += 1;
            }
        }
        assert_eq!(n, 68);
    }
}
