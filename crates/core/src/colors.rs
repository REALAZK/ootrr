use std::collections::BTreeMap as BM;
use std::fs;
use std::path::{Path as P, PathBuf as PB};

use crate::bfres::{self, Mat};
use crate::error::{Error as E, Res};
use crate::rng::Rng;
use crate::romfs::Romfs;
use crate::tex::bntx::{self, Tex};
use crate::tex::image::Img;
use crate::tex::png;
use crate::tex::repaint::{self, Count};
use crate::tex::shift::Shift;
use crate::tex::surface;
use crate::zs;

pub const NAMES: &str = "Shader/ExternalBinaryString.bfres.zs";
const SIDE: usize = 384;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Colors {
    Off,
    Random,
    Shared,
}

impl Colors {
    pub fn parse(s: &str) -> Option<Colors> {
        match s {
            "off" => Some(Colors::Off),
            "random" => Some(Colors::Random),
            "shared" => Some(Colors::Shared),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Colors::Off => "off",
            Colors::Random => "random",
            Colors::Shared => "shared",
        }
    }
}

pub struct Item {
    pub id: &'static str,
    pub label: &'static str,
    pub tex: &'static [&'static str],
    pub band: Option<(f32, f32)>,
    pub vals: &'static [(&'static str, &'static str, &'static str)],
}

const C0: &str = "ns_param_color0";
const C1: &str = "ns_param_color1";
const C2: &str = "ns_param_color2";
const C3: &str = "ns_param_color3";

pub const ITEMS: [Item; 14] = [
    Item { id: "kokiri", label: "Kokiri tunic", band: None, tex: &["Angin_Tile_A_Alb", "Angin_Tile_A_Dark_Alb"], vals: &[] },
    Item { id: "goron", label: "Goron tunic", band: Some((354.0, 18.0)), tex: &["Mt_Angin_Armor_Adult_Goron_Alb", "Mt_Inner_Armor_Adult_Goron_Alb"], vals: &[] },
    Item { id: "zora", label: "Zora tunic", band: None, tex: &["Mt_Angin_Armor_Adult_Zora_Alb", "Mt_Inner_Armor_Adult_Zora_Alb"], vals: &[] },
    Item { id: "kokiri_sword", label: "Kokiri Sword", band: Some((350.0, 18.0)), tex: &["Mt_Weapon_Sword_Kokiri_Alb"], vals: &[] },
    Item {
        id: "master_sword",
        label: "Master Sword",
        band: None,
        tex: &["Mt_Weapon_Sword_Master_Alb"],
        vals: &[
            ("Weapon_Sword_Master.Weapon_Sword_Master", "Mt_Weapon_Sword_Master", C0),
            ("Weapon_Sword_Master.Weapon_Sword_Master", "Mt_Weapon_Sword_Master", C1),
            ("Weapon_Sword_Master.Land_HrlCity_MasterSword", "Mt_Weapon_Sword_Master", C0),
            ("Weapon_Sword_Master.Land_HrlCity_MasterSword", "Mt_Weapon_Sword_Master", C1),
        ],
    },
    Item { id: "biggoron_sword", label: "Biggoron Sword", band: None, tex: &["Mt_Weapon_Sword_Biggoron_Alb.00", "Mt_Weapon_Sword_Biggoron_Alb.01"], vals: &[] },
    Item { id: "deku_shield", label: "Deku Shield", band: Some((2.0, 14.0)), tex: &["Mt_Body_Weapon_Shield_Deku_Alb"], vals: &[] },
    Item { id: "hylian_shield", label: "Hylian Shield", band: Some((231.0, 35.0)), tex: &["Mt_Weapon_Shield_Hylian_Alb"], vals: &[] },
    Item { id: "mirror_shield", label: "Mirror Shield", band: Some((2.0, 14.0)), tex: &["Mt_Weapon_Shield_Mirror_Alb"], vals: &[] },
    Item {
        id: "navi",
        label: "Navi",
        band: None,
        tex: &["Npc_Navi_Body_Alb.00", "Npc_Navi_Body_Alb.01", "Npc_Navi_Wings_Alb.00", "Npc_Navi_Wings_Emc.00", "Npc_Navi_Wings_Emc.01"],
        vals: &[("Npc_Navi.Npc_Navi", "Mt_Body", C1), ("Npc_Navi.Npc_Navi", "Mt_Body_AlphaMask", C1)],
    },
    Item {
        id: "boomerang",
        label: "Boomerang gem",
        band: None,
        tex: &[],
        vals: &[("Item_Boomerang.Item_Boomerang", "Mt_Item_Boomerang_Gem", C0), ("Item_Boomerang.Item_Boomerang", "Mt_Item_Boomerang_Gem", C1), ("Item_Boomerang.Item_Boomerang", "Mt_Item_Boomerang_Gem", C2)],
    },
    Item {
        id: "gauntlets",
        label: "Gauntlet jewels",
        band: None,
        tex: &[],
        vals: &[
            ("Armor_Adult_Gauntlet_Silver.Armor_Adult_Gauntlet_Silver", "Mt_Jewelry", C0),
            ("Armor_Adult_Gauntlet_Silver.Armor_Adult_Gauntlet_Silver", "Mt_Jewelry", C1),
            ("Armor_Adult_Gauntlet_Silver.Armor_Adult_Gauntlet_Silver", "Mt_Jewelry", C2),
            ("Armor_Adult_Gauntlet_Gold.Armor_Adult_Gauntlet_Gold", "Mt_Jewelry", C0),
            ("Armor_Adult_Gauntlet_Gold.Armor_Adult_Gauntlet_Gold", "Mt_Jewelry", C1),
            ("Armor_Adult_Gauntlet_Gold.Armor_Adult_Gauntlet_Gold", "Mt_Jewelry", C2),
        ],
    },
    Item {
        id: "hair",
        label: "Hair",
        band: None,
        tex: &[],
        vals: &[
            ("Player_Adult.Player_Adult", "Mt_Hairs", C0),
            ("Player_Adult.Player_Adult", "Mt_Hairs", C1),
            ("Player_Adult.Player_Adult", "Mt_Hairs", C3),
            ("Player_Child.Player_Child", "Mt_Hairs", C0),
            ("Player_Child.Player_Child", "Mt_Hairs", C1),
            ("Player_Child.Player_Child", "Mt_Hairs", C3),
        ],
    },
    Item { id: "hearts", label: "Recovery hearts", band: None, tex: &["Mt_Obj_Heart_Alb", "Mt_Heart_Container_Alb"], vals: &[] },
];

fn unit(rng: &mut Rng) -> f32 {
    rng.below(10_000) as f32 / 10_000.0
}

fn roll(rng: &mut Rng) -> Shift {
    let turn: f32 = 40.0 + 280.0 * unit(rng);
    let sat: f32 = 0.85 + 0.4 * unit(rng);
    let gain: f32 = 0.9 + 0.2 * unit(rng);
    Shift::new(turn, sat, gain)
}

pub fn plan(c: Colors, seed: &str) -> Vec<(&'static Item, Shift)> {
    if c == Colors::Off {
        return Vec::new();
    }
    let mut rng: Rng = Rng::from_text(&format!("{seed} colors {}", c.id()));
    let one: Shift = roll(&mut rng);
    ITEMS
        .iter()
        .map(|it| {
            let s: Shift = if c == Colors::Shared { one } else { roll(&mut rng) };
            (it, it.band.map(|b| s.band(b.0, b.1)).unwrap_or(s))
        })
        .collect()
}

fn tex_path(name: &str) -> String {
    format!("Tex/{name}.bntx.zs")
}

fn model_path(name: &str) -> String {
    format!("Model/{name}.bfres.zs")
}

fn dict(z: &[u8]) -> Option<u32> {
    let head: u8 = *z.get(4)?;
    let at: usize = if head & 0x20 == 0 { 6 } else { 5 };
    let n: usize = [0, 1, 2, 4][(head & 3) as usize];
    let s: &[u8] = z.get(at..at + n)?;
    (n > 0).then(|| s.iter().rev().fold(0u32, |v, x| (v << 8) | *x as u32))
}

fn put(root: &P, r: &str, raw: &[u8], z: &[u8]) -> Res<()> {
    let out: PB = root.join("romfs").join(r);
    if let Some(d) = out.parent() {
        fs::create_dir_all(d)?;
    }
    fs::write(&out, zs::compress_raw(raw, dict(z))?)?;
    Ok(())
}

pub fn tint_model(b: &mut [u8], keys: &BM<String, u64>, edits: &[(&str, &str, Shift)]) -> Res<usize> {
    let ms: Vec<Mat> = bfres::mats(b)?;
    let mut n: usize = 0;
    for (mat, param, s) in edits {
        let Some(p) = keys.get(*param).and_then(|k| bfres::find(&ms, mat, *k)) else {
            continue;
        };
        let v: [f32; 4] = bfres::read4(b, p.at)?;
        let l: [f32; 3] = s.light([v[0], v[1], v[2]]);
        bfres::write4(b, p.at, [l[0], l[1], l[2], v[3]])?;
        n += 1;
    }
    Ok(n)
}

pub fn apply(mod_root: &P, romfs: &P, c: Colors, seed: &str) -> Res<Vec<String>> {
    let mut notes: Vec<String> = Vec::new();
    let todo: Vec<(&Item, Shift)> = plan(c, seed);
    if todo.is_empty() {
        return Ok(notes);
    }
    let mut rf: Romfs = Romfs::open(romfs)?;
    let keys: BM<String, u64> = bfres::keys(&rf.read_zs(NAMES)?)?;
    let mut by_model: BM<&str, Vec<(&str, &str, Shift)>> = BM::new();
    let mut done: Vec<&str> = Vec::new();
    let (mut files, mut bytes): (usize, u64) = (0, 0);
    for (it, s) in &todo {
        let mut hit: bool = false;
        for t in it.tex {
            let r: String = tex_path(t);
            let Ok(z) = rf.read(&r) else {
                continue;
            };
            let mut raw: Vec<u8> = rf.zs.decompress(&z)?;
            let n: Count = repaint::repaint(&mut raw, &|c| s.rgb(c))?;
            if n.changed == 0 {
                continue;
            }
            put(mod_root, &r, &raw, &z)?;
            files += 1;
            bytes += raw.len() as u64;
            hit = true;
        }
        for (m, mat, param) in it.vals {
            by_model.entry(m).or_default().push((mat, param, *s));
            hit = true;
        }
        if hit {
            done.push(it.label);
        }
    }
    for (m, edits) in &by_model {
        let r: String = model_path(m);
        let Ok(z) = rf.read(&r) else {
            continue;
        };
        let mut raw: Vec<u8> = rf.zs.decompress(&z)?;
        if tint_model(&mut raw, &keys, edits)? == 0 {
            continue;
        }
        put(mod_root, &r, &raw, &z)?;
        files += 1;
        bytes += raw.len() as u64;
    }
    if files == 0 {
        return Err(E::Format("no colour could be changed"));
    }
    notes.push(format!(
        "Colours are randomized {}. {} files rewritten, {} MB added to the mod. Changed: {}. Particle glows, trails and the menu keep their own colours.",
        if c == Colors::Shared { "with one shared hue turn" } else { "one by one" },
        files,
        bytes / 1_000_000,
        done.join(", ")
    ));
    Ok(notes)
}

fn shot(b: &[u8]) -> Res<Img> {
    let t: Tex = bntx::parse(b)?.into_iter().next().ok_or(E::Format("the texture file is empty"))?;
    Ok(surface::decode(b, &t)?.shrink(SIDE))
}

pub fn preview(romfs: &P, out: &P, c: Colors, seed: &str) -> Res<Vec<String>> {
    let mut lines: Vec<String> = Vec::new();
    let mut rf: Romfs = Romfs::open(romfs)?;
    fs::create_dir_all(out)?;
    for (it, s) in plan(c, seed) {
        for t in it.tex {
            let Ok(old) = rf.read_zs(&tex_path(t)) else {
                lines.push(format!("{t}: missing"));
                continue;
            };
            let mut new: Vec<u8> = old.clone();
            let n: Count = repaint::repaint(&mut new, &|c| s.rgb(c))?;
            let (a, b): (Img, Img) = (shot(&old)?, shot(&new)?);
            let mut want: Img = a.clone();
            let (mut sum, mut worst): (u64, u32) = (0, 0);
            for (p, q) in want.px.chunks_exact_mut(4).zip(b.px.chunks_exact(4)) {
                let w: [u8; 3] = s.rgb([p[0], p[1], p[2]]);
                p[..3].copy_from_slice(&w);
                for i in 0..3 {
                    let d: u32 = (w[i] as i32 - q[i] as i32).unsigned_abs();
                    sum += d as u64;
                    worst = worst.max(d);
                }
            }
            fs::write(out.join(format!("{}_{t}_before.png", it.id)), png::encode(&a))?;
            fs::write(out.join(format!("{}_{t}_after.png", it.id)), png::encode(&b))?;
            lines.push(format!("{t}: {} of {} blocks changed, {} skipped, mean error {:.2}, worst {}", n.changed, n.blocks, n.skipped, sum as f64 / (a.px.len() as f64 * 0.75), worst));
        }
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfres::fixture;

    #[test]
    fn mode_names_round_trip() {
        for m in [Colors::Off, Colors::Random, Colors::Shared] {
            assert_eq!(Colors::parse(m.id()), Some(m));
        }
        assert_eq!(Colors::parse("loud"), None);
    }

    #[test]
    fn the_plan_is_fixed_by_the_seed_and_off_plans_nothing() {
        let a: Vec<Shift> = plan(Colors::Random, "s").into_iter().map(|x| x.1).collect();
        let b: Vec<Shift> = plan(Colors::Random, "s").into_iter().map(|x| x.1).collect();
        let c: Vec<Shift> = plan(Colors::Random, "t").into_iter().map(|x| x.1).collect();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), ITEMS.len());
        assert!(a.windows(2).any(|w| w[0] != w[1]));
        assert!(plan(Colors::Off, "s").is_empty());
    }

    #[test]
    fn shared_mode_gives_every_item_the_same_turn() {
        let a: Vec<Shift> = plan(Colors::Shared, "s").into_iter().map(|x| x.1).collect();
        assert!(a.windows(2).all(|w| w[0].m == w[1].m));
        assert_ne!(a[0].m, Shift::same().m);
    }

    #[test]
    fn item_ids_are_unique_and_every_item_changes_something() {
        let mut ids: Vec<&str> = ITEMS.iter().map(|i| i.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ITEMS.len());
        assert!(ITEMS.iter().all(|i| !i.tex.is_empty() || !i.vals.is_empty()));
    }

    #[test]
    fn a_model_value_is_turned_in_place_and_others_are_left() {
        let mut b: Vec<u8> = fixture::model(&[("Mt_A", &[(7, [0.8, 0.1, 0.05, 0.5]), (9, [0.3, 0.3, 0.3, 1.0])]), ("Mt_B", &[(7, [0.5, 0.6, 0.7, 0.8])])]);
        let old: Vec<u8> = b.clone();
        let keys: BM<String, u64> = [("c0".to_string(), 7u64), ("c1".to_string(), 9u64)].into_iter().collect();
        let s: Shift = Shift::new(120.0, 1.0, 1.0);
        let n: usize = tint_model(&mut b, &keys, &[("Mt_A", "c0", s), ("Mt_A", "c1", s), ("Mt_C", "c0", s), ("Mt_A", "c9", s)]).unwrap();
        assert_eq!((n, b.len()), (2, old.len()));
        let ms: Vec<Mat> = bfres::mats(&b).unwrap();
        let v: [f32; 4] = bfres::read4(&b, bfres::find(&ms, "Mt_A", 7).unwrap().at).unwrap();
        assert!((v[0] - 0.05).abs() < 0.01 && (v[1] - 0.8).abs() < 0.01 && (v[2] - 0.1).abs() < 0.01 && v[3] == 0.5, "{v:?}");
        let g: [f32; 4] = bfres::read4(&b, bfres::find(&ms, "Mt_A", 9).unwrap().at).unwrap();
        assert!(g.iter().take(3).all(|x| (x - 0.3).abs() < 0.01));
        assert_eq!(bfres::read4(&b, bfres::find(&ms, "Mt_B", 7).unwrap().at).unwrap(), [0.5, 0.6, 0.7, 0.8]);
    }

    #[test]
    fn the_real_files_keep_their_size_and_an_unchanged_map_keeps_their_bytes() {
        let Some(root) = std::env::var("OOTRR_ROMFS").map(PB::from).ok().filter(|p| p.join("Pack").join("ZsDic.pack.zs").is_file()) else {
            return;
        };
        let out: PB = std::env::temp_dir().join(format!("ootrr_colors_{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        let notes: Vec<String> = apply(&out, &root, Colors::Random, "colour seed").unwrap();
        assert!(notes[0].contains("Kokiri tunic") && notes[0].contains("Master Sword") && notes[0].contains("Navi"), "{notes:?}");
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut seen: usize = 0;
        for it in &ITEMS {
            for r in it.tex.iter().map(|t| tex_path(t)).chain(it.vals.iter().map(|v| model_path(v.0))) {
                let p: PB = out.join("romfs").join(&r);
                if !p.is_file() {
                    continue;
                }
                let old: Vec<u8> = rf.read_zs(&r).unwrap();
                let new: Vec<u8> = rf.zs.decompress(&fs::read(&p).unwrap()).unwrap();
                assert_eq!(old.len(), new.len(), "{r}");
                assert_ne!(old, new, "{r}");
                seen += 1;
            }
            for t in it.tex {
                let old: Vec<u8> = rf.read_zs(&tex_path(t)).unwrap();
                let mut same: Vec<u8> = old.clone();
                assert_eq!(repaint::repaint(&mut same, &|c| c).unwrap().changed, 0, "{t}");
                assert_eq!(same, old, "{t}");
            }
        }
        assert!(seen >= 25, "{seen}");
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn the_dictionary_id_is_read_from_the_frame_header() {
        assert_eq!(dict(&[0x28, 0xb5, 0x2f, 0xfd, 0xa1, 0x01, 0, 0]), Some(1));
        assert_eq!(dict(&[0x28, 0xb5, 0x2f, 0xfd, 0xa0, 0x70, 0, 0]), None);
        assert_eq!(dict(&[0x28, 0xb5, 0x2f, 0xfd, 0x81, 0x68, 0x01, 0]), Some(1));
        assert_eq!(dict(&[0x28, 0xb5, 0x2f, 0xfd, 0x22, 0x02, 0x01, 0]), Some(258));
    }
}
