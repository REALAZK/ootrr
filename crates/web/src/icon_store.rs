use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path as P, PathBuf as PB};
use std::sync::{Arc, Mutex, MutexGuard};

use ootrr_core::error::{Error, Res};
use ootrr_core::tex::png;
use ootrr_core::tracker::icon_map::{self, Src};

use crate::datadir;

pub const DISK_CACHE: bool = true;
pub const SWITCH: &str = "OOTRR_ICON_CACHE";

type Png = Arc<Vec<u8>>;

struct Held {
    root: PB,
    png: HashMap<&'static str, Png>,
    bad: HashMap<&'static str, String>,
}

pub struct Icons {
    held: Mutex<Option<Held>>,
    work: Mutex<()>,
    pub disk: bool,
}

pub fn disk_on() -> bool {
    DISK_CACHE && !matches!(env::var(SWITCH).as_deref(), Ok("off") | Ok("0") | Ok("no") | Ok("false"))
}

pub fn cache_dir(data: &P, game: &str) -> PB {
    data.join(datadir::game_dir_name(game)).join(format!("icons-v{}", icon_map::VERSION))
}

fn write_atomic(p: &P, b: &[u8]) -> Res<()> {
    if let Some(d) = p.parent() {
        fs::create_dir_all(d)?;
    }
    let tmp: PB = p.with_extension("tmp");
    fs::write(&tmp, b)?;
    fs::rename(&tmp, p)?;
    Ok(())
}

impl Icons {
    pub fn new(disk: bool) -> Icons {
        Icons { held: Mutex::new(None), work: Mutex::new(()), disk }
    }

    pub fn mode(&self) -> &'static str {
        if self.disk { "disk" } else { "memory" }
    }

    fn lock(&self) -> MutexGuard<'_, Option<Held>> {
        self.held.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn known(&self, root: &P, id: &str) -> Option<Res<Png>> {
        let mut g: MutexGuard<'_, Option<Held>> = self.lock();
        if g.as_ref().is_none_or(|h| h.root != root) {
            *g = Some(Held { root: root.to_path_buf(), png: HashMap::new(), bad: HashMap::new() });
        }
        let h: &Held = g.as_ref()?;
        if let Some(p) = h.png.get(id) {
            return Some(Ok(Arc::clone(p)));
        }
        h.bad.get(id).map(|m| Err(Error::Msg(m.clone())))
    }

    fn keep(&self, root: &P, id: &'static str, r: Result<Png, String>) {
        let mut g: MutexGuard<'_, Option<Held>> = self.lock();
        let Some(h) = g.as_mut().filter(|h| h.root == root) else {
            return;
        };
        match r {
            Ok(p) => {
                h.png.insert(id, p);
            }
            Err(m) => {
                h.bad.insert(id, m);
            }
        }
    }

    #[cfg(test)]
    pub fn held(&self) -> usize {
        self.lock().as_ref().map(|h| h.png.len()).unwrap_or(0)
    }

    pub fn get(&self, root: &P, cache: Option<&P>, id: &str) -> Res<Png> {
        let src: Src = icon_map::find(id).ok_or_else(|| Error::Msg("no such icon".to_string()))?;
        if let Some(r) = self.known(root, id) {
            return r;
        }
        let _one: MutexGuard<'_, ()> = self.work.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(r) = self.known(root, id) {
            return r;
        }
        let file: Option<PB> = cache.filter(|_| self.disk).map(|d| d.join(format!("{}.png", src.id)));
        if let Some(f) = &file
            && let Ok(b) = fs::read(f)
            && png::looks_ok(&b)
        {
            let p: Png = Arc::new(b);
            self.keep(root, src.id, Ok(Arc::clone(&p)));
            return Ok(p);
        }
        for (other, r) in icon_map::render_file(root, &src.file)? {
            match r {
                Ok(b) => {
                    if let Some(d) = cache.filter(|_| self.disk) {
                        let _ = write_atomic(&d.join(format!("{other}.png")), &b);
                    }
                    self.keep(root, other, Ok(Arc::new(b)));
                }
                Err(e) => self.keep(root, other, Err(e.to_string())),
            }
        }
        self.known(root, id).unwrap_or_else(|| Err(Error::Msg("the icon could not be read".to_string())))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use ootrr_core::tex::swizzle;
    use ootrr_core::zs;

    pub fn fake_bntx(name: &str, side: usize, c: [u8; 4]) -> Vec<u8> {
        let mut void: u128 = 0xdfc | 0xffff_ffff_ffff_f000u128;
        for (i, v) in c.iter().enumerate() {
            void |= ((*v as u128) << 8 | *v as u128) << (64 + 16 * i);
        }
        let nb: usize = side.div_ceil(4);
        let o: usize = 0x48;
        let off: usize = o + 0x100;
        let mut b: Vec<u8> = vec![0; off + swizzle::span(nb, nb, 1)];
        let put = |b: &mut Vec<u8>, at: usize, v: &[u8]| b[at..at + v.len()].copy_from_slice(v);
        put(&mut b, 0, b"BNTX");
        put(&mut b, 0x24, &1u32.to_le_bytes());
        put(&mut b, 0x28, &0x40u64.to_le_bytes());
        put(&mut b, 0x40, &(o as u64).to_le_bytes());
        put(&mut b, o, b"BRTI");
        put(&mut b, o + 0x1c, &0x2d06u32.to_le_bytes());
        put(&mut b, o + 0x24, &(side as u32).to_le_bytes());
        put(&mut b, o + 0x28, &(side as u32).to_le_bytes());
        put(&mut b, o + 0x58, &[2, 3, 4, 5]);
        put(&mut b, o + 0x60, &((o + 0x80) as u64).to_le_bytes());
        put(&mut b, o + 0x80, &(name.len() as u16).to_le_bytes());
        put(&mut b, o + 0x82, name.as_bytes());
        put(&mut b, o + 0x70, &((o + 0xf0) as u64).to_le_bytes());
        put(&mut b, o + 0xf0, &(off as u64).to_le_bytes());
        for y in 0..nb {
            for x in 0..nb {
                put(&mut b, off + swizzle::addr(x, y, nb, 1), &void.to_le_bytes());
            }
        }
        b
    }

    pub fn fake_game(tag: &str, ids: &[&str]) -> PB {
        let d: PB = env::temp_dir().join(format!("ootrr_icons_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        let dir: PB = d.join("UI").join("Tex").join("Icon");
        fs::create_dir_all(&dir).unwrap();
        for id in ids {
            let b: Vec<u8> = fake_bntx(id, 20, [10, 200, 30, 255]);
            fs::write(dir.join(format!("{id}.bntx.zs")), zs::compress_raw(&b, None).unwrap()).unwrap();
        }
        d
    }

    #[test]
    fn an_icon_is_decoded_once_and_then_served_from_memory() {
        let d: PB = fake_game("mem", &["Item_Bomb"]);
        let ic: Icons = Icons::new(false);
        let a: Png = ic.get(&d, None, "Item_Bomb").unwrap();
        assert!(png::looks_ok(&a));
        assert_eq!(ic.held(), 1);
        fs::remove_dir_all(d.join("UI")).unwrap();
        let b: Png = ic.get(&d, None, "Item_Bomb").unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        assert!(ic.get(&d, None, "Item_Hookshot").is_err());
        assert!(ic.get(&d, None, "Item_Nope").is_err());
        assert!(ic.get(&d, None, "../Item_Bomb").is_err());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_disk_cache_is_written_and_read_back_when_it_is_on() {
        let d: PB = fake_game("disk", &["Item_Bomb"]);
        let cache: PB = cache_dir(&d.join("data"), "oot-remake:abc");
        assert!(cache.ends_with(PB::from("oot-remake-abc").join("icons-v1")));
        let on: Icons = Icons::new(true);
        let a: Png = on.get(&d, Some(&cache), "Item_Bomb").unwrap();
        let f: PB = cache.join("Item_Bomb.png");
        assert_eq!(fs::read(&f).unwrap(), *a);
        fs::remove_dir_all(d.join("UI")).unwrap();
        let again: Icons = Icons::new(true);
        assert_eq!(*again.get(&d, Some(&cache), "Item_Bomb").unwrap(), *a);
        fs::write(&f, b"broken").unwrap();
        let third: Icons = Icons::new(true);
        assert!(third.get(&d, Some(&cache), "Item_Bomb").is_err());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn with_the_switch_off_nothing_is_written() {
        let d: PB = fake_game("off", &["Item_Bomb"]);
        let cache: PB = d.join("data").join("icons");
        let off: Icons = Icons::new(false);
        assert_eq!(off.mode(), "memory");
        off.get(&d, Some(&cache), "Item_Bomb").unwrap();
        assert!(!cache.exists());
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("Item_Boomerang.png"), png::encode(&ootrr_core::tex::image::Img::new(4, 4))).unwrap();
        assert!(off.get(&d, Some(&cache), "Item_Boomerang").is_err());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_new_game_folder_drops_the_old_icons() {
        let a: PB = fake_game("swap_a", &["Item_Bomb"]);
        let b: PB = fake_game("swap_b", &[]);
        let ic: Icons = Icons::new(false);
        ic.get(&a, None, "Item_Bomb").unwrap();
        assert!(ic.get(&b, None, "Item_Bomb").is_err());
        assert_eq!(ic.held(), 0);
        let _ = fs::remove_dir_all(&a);
        let _ = fs::remove_dir_all(&b);
    }

    #[test]
    fn the_switch_reads_the_environment_names() {
        assert!(DISK_CACHE);
        assert_eq!(SWITCH, "OOTRR_ICON_CACHE");
        assert_eq!(Icons::new(true).mode(), "disk");
    }
}
