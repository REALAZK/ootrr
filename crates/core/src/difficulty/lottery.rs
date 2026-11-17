use super::drops::{Cut, AMMO, HEART};
use crate::error::{Error, Res};

pub const FILE: &str = "GameParameter/DropGlobalParam/Default.game__drop__DropGlobalParam.bgyml";
pub const MASTER: &str = "DropLotteryMasterData";
pub const NEEDS: [&str; 2] = ["DepletedInfoData1", "DepletedInfoData2"];
pub const BOOST: &str = "AddProbability";
pub const SWAPS: &str = "ReplaceSetting";
pub const SWAP_LIST: &str = "ReplaceActor";
pub const HEART_SLOTS: [&str; 1] = ["HpActor"];
pub const AMMO_SLOTS: [&str; 8] = ["ArrowActor", "ArrowActorMid", "BombActor", "BombchuActor", "DekuNutActor", "DekuSeedActor", "DekuSeedActorMid", "DekuStickActor"];
pub const HEART_ACTOR: &str = "Work/Actor/Obj_Heart.engine__actor__ActorParam.gyml";
pub const FILLER: &str = "Work/Actor/Obj_Rupee_Green.engine__actor__ActorParam.gyml";

const STR: u8 = 0xA0;
const ARR: u8 = 0xC0;
const MAP: u8 = 0xC1;
const HASH: u8 = 0x20;
const INT: u8 = 0xD1;

fn cut() -> Error {
    Error::Format("difficulty: the drop lottery file is truncated")
}

struct Doc<'a> {
    b: &'a [u8],
    keys: Vec<String>,
    strs: Vec<String>,
}

#[derive(Clone, Copy)]
struct Slot {
    ty: u8,
    val: u32,
    at: usize,
}

impl<'a> Doc<'a> {
    fn u32(&self, o: usize) -> Res<u32> {
        let s: &[u8] = self.b.get(o..o + 4).ok_or_else(cut)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn u8(&self, o: usize) -> Res<u8> {
        self.b.get(o).copied().ok_or_else(cut)
    }

    fn count(&self, o: usize) -> Res<usize> {
        Ok((self.u32(o)? >> 8) as usize)
    }

    fn table(&self, o: usize) -> Res<Vec<String>> {
        let mut out: Vec<String> = Vec::new();
        if o == 0 {
            return Ok(out);
        }
        for i in 0..self.count(o)? {
            let a: usize = o + self.u32(o + 4 + 4 * i)? as usize;
            let tail: &[u8] = self.b.get(a..).ok_or_else(cut)?;
            let z: usize = tail.iter().position(|c| *c == 0).ok_or_else(cut)?;
            out.push(String::from_utf8_lossy(&tail[..z]).to_string());
        }
        Ok(out)
    }

    fn open(b: &'a [u8]) -> Res<Doc<'a>> {
        if b.get(..2) != Some(b"YB") {
            return Err(Error::Format("difficulty: the drop lottery file is not a little endian byml"));
        }
        let mut d: Doc = Doc { b, keys: Vec::new(), strs: Vec::new() };
        d.keys = d.table(d.u32(4)? as usize)?;
        d.strs = d.table(d.u32(8)? as usize)?;
        Ok(d)
    }

    fn root(&self) -> Res<Slot> {
        let o: usize = self.u32(12)? as usize;
        Ok(Slot { ty: self.u8(o)?, val: o as u32, at: 12 })
    }

    fn map(&self, s: Slot) -> Res<Vec<(String, Slot)>> {
        let mut out: Vec<(String, Slot)> = Vec::new();
        if s.ty != MAP {
            return Ok(out);
        }
        let o: usize = s.val as usize;
        for i in 0..self.count(o)? {
            let p: usize = o + 4 + 8 * i;
            let k: usize = (self.u32(p)? & 0xFF_FFFF) as usize;
            let name: String = self.keys.get(k).cloned().ok_or_else(cut)?;
            out.push((name, Slot { ty: self.u8(p + 3)?, val: self.u32(p + 4)?, at: p + 4 }));
        }
        Ok(out)
    }

    fn hash(&self, s: Slot) -> Res<Vec<(u32, Slot)>> {
        let mut out: Vec<(u32, Slot)> = Vec::new();
        if s.ty != HASH {
            return Ok(out);
        }
        let o: usize = s.val as usize;
        let n: usize = self.count(o)?;
        for i in 0..n {
            let p: usize = o + 4 + 8 * i;
            out.push((self.u32(p)?, Slot { ty: self.u8(o + 4 + 8 * n + i)?, val: self.u32(p + 4)?, at: p + 4 }));
        }
        Ok(out)
    }

    fn arr(&self, s: Slot) -> Res<Vec<Slot>> {
        let mut out: Vec<Slot> = Vec::new();
        if s.ty != ARR {
            return Ok(out);
        }
        let o: usize = s.val as usize;
        let n: usize = self.count(o)?;
        let vo: usize = (o + 4 + n + 3) & !3;
        for i in 0..n {
            out.push(Slot { ty: self.u8(o + 4 + i)?, val: self.u32(vo + 4 * i)?, at: vo + 4 * i });
        }
        Ok(out)
    }

    fn text(&self, s: Slot) -> Option<&str> {
        if s.ty != STR {
            return None;
        }
        self.strs.get(s.val as usize).map(|x| x.as_str())
    }
}

fn find(m: &[(String, Slot)], k: &str) -> Option<Slot> {
    m.iter().find(|e| e.0 == k).map(|e| e.1)
}

pub fn edit(raw: &[u8], c: Cut) -> Res<Option<Vec<u8>>> {
    if !c.any() {
        return Ok(None);
    }
    let d: Doc = Doc::open(raw)?;
    let top: Vec<(String, Slot)> = d.map(d.root()?)?;
    let master: Vec<(String, Slot)> = d.map(find(&top, MASTER).ok_or(Error::Format("difficulty: the drop lottery file has no master data"))?)?;
    let filler: u32 = d.strs.iter().position(|s| s == FILLER).ok_or(Error::Format("difficulty: the drop lottery file never names the green rupee"))? as u32;
    let wanted = |t: u32| -> bool { (c.hearts && t == HEART) || (c.ammo && AMMO.contains(&t)) };
    let mut puts: Vec<(usize, u32)> = Vec::new();
    for name in NEEDS {
        let Some(s) = find(&master, name) else {
            continue;
        };
        let rows: Vec<(u32, Slot)> = d.hash(s)?;
        let shared: Vec<u32> = rows.iter().filter(|r| !wanted(r.0)).map(|r| r.1.val).collect();
        for (t, row) in &rows {
            if !wanted(*t) || shared.contains(&row.val) {
                continue;
            }
            if let Some(x) = find(&d.map(*row)?, BOOST)
                && x.ty == INT
                && x.val != 0
            {
                puts.push((x.at, 0));
            }
        }
    }
    let mut slots: Vec<&str> = Vec::new();
    if c.hearts {
        slots.extend(HEART_SLOTS);
    }
    if c.ammo {
        slots.extend(AMMO_SLOTS);
    }
    for k in slots {
        if let Some(x) = find(&master, k)
            && x.ty == STR
            && x.val != filler
        {
            puts.push((x.at, filler));
        }
    }
    if c.hearts
        && let Some(s) = find(&master, SWAPS)
    {
        for rule in d.arr(s)? {
            let Some(list) = find(&d.map(rule)?, SWAP_LIST) else {
                continue;
            };
            for item in d.arr(list)? {
                if let Some(x) = find(&d.map(item)?, SWAP_LIST)
                    && d.text(x) == Some(HEART_ACTOR)
                {
                    puts.push((x.at, filler));
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

pub fn boosts(raw: &[u8]) -> Res<Vec<(u32, i32)>> {
    let d: Doc = Doc::open(raw)?;
    let master: Vec<(String, Slot)> = d.map(find(&d.map(d.root()?)?, MASTER).ok_or_else(cut)?)?;
    let mut out: Vec<(u32, i32)> = Vec::new();
    for name in NEEDS {
        let Some(s) = find(&master, name) else {
            continue;
        };
        for (t, row) in d.hash(s)? {
            if let Some(x) = find(&d.map(row)?, BOOST) {
                out.push((t, x.val as i32));
            }
        }
    }
    Ok(out)
}

pub fn actor_of(raw: &[u8], key: &str) -> Res<Option<String>> {
    let d: Doc = Doc::open(raw)?;
    let master: Vec<(String, Slot)> = d.map(find(&d.map(d.root()?)?, MASTER).ok_or_else(cut)?)?;
    Ok(find(&master, key).and_then(|s| d.text(s)).map(|s| s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::romfs::Romfs;

    fn game() -> Option<Vec<u8>> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        Romfs::open(&p).ok()?.read(FILE).ok()
    }

    #[test]
    fn nothing_is_cut_without_an_option_and_bad_bytes_are_refused() {
        assert!(edit(b"YB", Cut::default()).unwrap().is_none());
        assert!(edit(b"XX\x07\x00", Cut { hearts: true, ammo: false }).is_err());
        assert!(edit(b"YB\x07\x00", Cut { hearts: true, ammo: false }).is_err());
    }

    #[test]
    fn the_game_lottery_stops_handing_out_hearts_and_ammo_when_supplies_are_low() {
        let Some(raw) = game() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let before: Vec<(u32, i32)> = boosts(&raw).unwrap();
        assert!(before.iter().any(|(t, v)| *t == HEART && *v > 0));
        let hearts: Vec<u8> = edit(&raw, Cut { hearts: true, ammo: false }).unwrap().unwrap();
        assert_eq!(hearts.len(), raw.len());
        let after: Vec<(u32, i32)> = boosts(&hearts).unwrap();
        for ((t, a), (_, b)) in before.iter().zip(after.iter()) {
            assert_eq!(*b, if *t == HEART { 0 } else { *a }, "type {t}");
        }
        assert_eq!(actor_of(&hearts, "HpActor").unwrap().as_deref(), Some(FILLER));
        assert_eq!(actor_of(&hearts, "BombActor").unwrap(), actor_of(&raw, "BombActor").unwrap());
        assert_eq!(actor_of(&hearts, "MpActor").unwrap(), actor_of(&raw, "MpActor").unwrap());
        assert!(!String::from_utf8_lossy(&hearts).is_empty());
        let both: Vec<u8> = edit(&raw, Cut { hearts: true, ammo: true }).unwrap().unwrap();
        for (t, v) in boosts(&both).unwrap() {
            if t == HEART || [super::super::drops::SHOTS, super::super::drops::BOMB, super::super::drops::NUT, super::super::drops::STICK, super::super::drops::ARROWS].contains(&t) {
                assert_eq!(v, 0, "type {t}");
            }
        }
        for k in AMMO_SLOTS {
            assert_eq!(actor_of(&both, k).unwrap().as_deref(), Some(FILLER), "{k}");
        }
        assert_eq!(actor_of(&both, "MpActor").unwrap(), actor_of(&raw, "MpActor").unwrap());
        assert_eq!(actor_of(&both, "FairyActorName").unwrap(), actor_of(&raw, "FairyActorName").unwrap());
        assert!(edit(&both, Cut { hearts: true, ammo: true }).unwrap().is_none());
    }
}
