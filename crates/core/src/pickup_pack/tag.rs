use std::collections::BTreeSet as BS;

use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};

pub const TAG: &str = "RSDB/Tag.Product.111.rstbl.byml.zs";
pub const HEAD: &str = "Work/Actor/";
pub const TAIL: &str = ".engine__actor__ActorParam.gyml";
pub const ALLOWED: [(&str, &str); 1] = [("Item_Bottle_Letter", "LetterBottle")];
pub const QUEST: [&str; 3] = ["Obj_Fire_Arrow", "Token_Gold", "BiggoronSword"];
const BITS: &str = "BitTable";
const PATHS: &str = "PathList";
const RANKS: &str = "RankTable";
const TAGS: &str = "TagList";
const WORD: usize = 32;
const U24: usize = 1 << 24;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    pub version: u16,
    pub paths: Vec<[String; 3]>,
    pub tags: Vec<String>,
    pub bits: Vec<bool>,
}

fn strs(y: &Byml, k: &str) -> Res<Vec<String>> {
    let a: &[Node] = y.root.get(k).and_then(|v| v.as_arr()).ok_or_else(|| E::Msg(format!("tag table: {k} is missing")))?;
    let mut out: Vec<String> = Vec::with_capacity(a.len());
    for x in a {
        out.push(x.as_str().ok_or_else(|| E::Msg(format!("tag table: {k} holds a value that is not text")))?.to_string());
    }
    Ok(out)
}

fn bin<'a>(y: &'a Byml, k: &str) -> Res<&'a [u8]> {
    match y.root.get(k) {
        Some(Node::Bin(d)) => Ok(d.as_slice()),
        _ => Err(E::Msg(format!("tag table: {k} is missing"))),
    }
}

fn key(p: &[String; 3]) -> String {
    p.concat()
}

fn p32(out: &mut Vec<u8>, v: usize) -> Res<()> {
    let n: u32 = u32::try_from(v).map_err(|_| E::Format("tag table: the file does not fit in 32 bits"))?;
    out.extend_from_slice(&n.to_le_bytes());
    Ok(())
}

fn p24(out: &mut Vec<u8>, v: usize) -> Res<()> {
    if v >= U24 {
        return Err(E::Format("tag table: a list is too long"));
    }
    out.extend_from_slice(&(v as u32).to_le_bytes()[..3]);
    Ok(())
}

fn pad(out: &mut Vec<u8>) {
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

fn names(out: &mut Vec<u8>, v: &[String]) -> Res<()> {
    let at: usize = out.len();
    out.push(0xC2);
    p24(out, v.len())?;
    let offs: usize = out.len();
    out.resize(offs + 4 * (v.len() + 1), 0);
    for (i, s) in v.iter().chain(std::iter::once(&String::new())).enumerate() {
        let rel: u32 = u32::try_from(out.len() - at).map_err(|_| E::Format("tag table: the file does not fit in 32 bits"))?;
        out[offs + 4 * i..offs + 4 * i + 4].copy_from_slice(&rel.to_le_bytes());
        if i < v.len() {
            out.extend_from_slice(s.as_bytes());
            out.push(0);
        }
    }
    pad(out);
    Ok(())
}

fn list_len(n: usize) -> usize {
    4 + n.div_ceil(4) * 4 + 4 * n
}

fn list<'a>(out: &mut Vec<u8>, all: &[String], v: impl Iterator<Item = &'a String>, n: usize) -> Res<()> {
    out.push(0xC0);
    p24(out, n)?;
    out.resize(out.len() + n, 0xA0);
    pad(out);
    for s in v {
        let i: usize = all.binary_search(s).map_err(|_| E::Format("tag table: a text is missing from the string table"))?;
        p32(out, i)?;
    }
    Ok(())
}

impl Table {
    pub fn parse(b: &[u8]) -> Res<Table> {
        let y: Byml = Byml::parse(b)?;
        if y.big {
            return Err(E::Format("tag table: big endian is not supported"));
        }
        let flat: Vec<String> = strs(&y, PATHS)?;
        let tags: Vec<String> = strs(&y, TAGS)?;
        if !flat.len().is_multiple_of(3) {
            return Err(E::Format("tag table: the path list is not made of triples"));
        }
        let paths: Vec<[String; 3]> = flat.chunks(3).map(|c| [c[0].clone(), c[1].clone(), c[2].clone()]).collect();
        let d: &[u8] = bin(&y, BITS)?;
        let n: usize = paths.len() * tags.len();
        if d.len() != n.div_ceil(8) {
            return Err(E::Format("tag table: the bit table does not match the path and tag counts"));
        }
        if (n..d.len() * 8).any(|i| d[i / 8] >> (i % 8) & 1 == 1) {
            return Err(E::Format("tag table: the bit table has bits past its end"));
        }
        let bits: Vec<bool> = (0..n).map(|i| d[i / 8] >> (i % 8) & 1 == 1).collect();
        let t: Table = Table { version: y.version, paths, tags, bits };
        if t.ranks() != bin(&y, RANKS)? {
            return Err(E::Format("tag table: the rank table does not follow the counting rule, the game files differ from the ones this was made for"));
        }
        if !t.sorted() {
            return Err(E::Format("tag table: the paths or the tags are not sorted"));
        }
        Ok(t)
    }

    pub fn sorted(&self) -> bool {
        self.paths.windows(2).all(|w| key(&w[0]).as_bytes() < key(&w[1]).as_bytes()) && self.tags.windows(2).all(|w| w[0].as_bytes() < w[1].as_bytes())
    }

    pub fn packed(&self) -> Vec<u8> {
        let mut out: Vec<u8> = vec![0; self.bits.len().div_ceil(8)];
        for (i, b) in self.bits.iter().enumerate() {
            if *b {
                out[i / 8] |= 1 << (i % 8);
            }
        }
        out
    }

    pub fn ranks(&self) -> Vec<u8> {
        let n: usize = self.paths.len();
        let w: usize = self.tags.len();
        let k: usize = n.div_ceil(WORD);
        let mut out: Vec<u8> = Vec::with_capacity(w * k * 4);
        for t in 0..w {
            let mut c: u32 = 0;
            for b in 0..k {
                for i in b * WORD..n.min((b + 1) * WORD) {
                    c += u32::from(self.bits[i * w + t]);
                }
                out.extend_from_slice(&c.to_le_bytes());
            }
        }
        out
    }

    pub fn find(&self, name: &str) -> Option<usize> {
        self.paths.iter().position(|p| p[0] == HEAD && p[1] == name && p[2] == TAIL)
    }

    pub fn tags_of(&self, name: &str) -> Vec<String> {
        let w: usize = self.tags.len();
        match self.find(name) {
            Some(i) => (0..w).filter(|t| self.bits[i * w + t]).map(|t| self.tags[t].clone()).collect(),
            None => Vec::new(),
        }
    }

    pub fn insert(&mut self, name: &str, tags: &[String]) -> Res<usize> {
        if !self.sorted() {
            return Err(E::Format("tag table: the paths or the tags are not sorted"));
        }
        let w: usize = self.tags.len();
        let mut on: Vec<usize> = Vec::new();
        for t in tags {
            on.push(self.tags.binary_search(t).map_err(|_| E::Msg(format!("{t} is not a tag of the game")))?);
        }
        let p: [String; 3] = [HEAD.to_string(), name.to_string(), TAIL.to_string()];
        let at: usize = match self.find(name) {
            Some(i) => {
                self.bits[i * w..(i + 1) * w].fill(false);
                i
            }
            None => {
                let k: String = key(&p);
                let i: usize = self.paths.partition_point(|x| key(x).as_bytes() < k.as_bytes());
                self.paths.insert(i, p);
                self.bits.splice(i * w..i * w, std::iter::repeat_n(false, w));
                i
            }
        };
        for t in on {
            self.bits[at * w + t] = true;
        }
        Ok(at)
    }

    pub fn write(&self) -> Res<Vec<u8>> {
        let all: BS<String> = self.paths.iter().flatten().chain(self.tags.iter()).cloned().collect();
        let all: Vec<String> = all.into_iter().collect();
        let keys: Vec<String> = [BITS, PATHS, RANKS, TAGS].iter().map(|s| s.to_string()).collect();
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"YB");
        out.extend_from_slice(&self.version.to_le_bytes());
        out.resize(16, 0);
        names(&mut out, &keys)?;
        let so: usize = out.len();
        names(&mut out, &all)?;
        let bit_at: usize = out.len();
        let d: Vec<u8> = self.packed();
        p32(&mut out, d.len())?;
        out.extend_from_slice(&d);
        let rank_at: usize = out.len();
        let r: Vec<u8> = self.ranks();
        p32(&mut out, r.len())?;
        out.extend_from_slice(&r);
        pad(&mut out);
        let root: usize = out.len();
        let path_at: usize = root + 4 + 8 * keys.len();
        let tag_at: usize = path_at + list_len(self.paths.len() * 3);
        out.push(0xC1);
        p24(&mut out, keys.len())?;
        for (i, (ty, at)) in [(0xA1u8, bit_at), (0xC0, path_at), (0xA1, rank_at), (0xC0, tag_at)].iter().enumerate() {
            p24(&mut out, i)?;
            out.push(*ty);
            p32(&mut out, *at)?;
        }
        list(&mut out, &all, self.paths.iter().flatten(), self.paths.len() * 3)?;
        list(&mut out, &all, self.tags.iter(), self.tags.len())?;
        for (at, v) in [(4usize, 16usize), (8, so), (12, root)] {
            let n: u32 = u32::try_from(v).map_err(|_| E::Format("tag table: the file does not fit in 32 bits"))?;
            out[at..at + 4].copy_from_slice(&n.to_le_bytes());
        }
        Ok(out)
    }

    fn without(&self, name: &str) -> (Vec<[String; 3]>, Vec<bool>) {
        let w: usize = self.tags.len();
        let mut paths: Vec<[String; 3]> = self.paths.clone();
        let mut bits: Vec<bool> = self.bits.clone();
        if let Some(i) = self.find(name) {
            paths.remove(i);
            bits.drain(i * w..(i + 1) * w);
        }
        (paths, bits)
    }
}

pub fn allowed(item: &str, tag: &str) -> Res<()> {
    if QUEST.contains(&tag) {
        return Err(E::Msg(format!("the tag {tag} drives a quest branch of the pickup event, it is never written on a new pickup")));
    }
    if !ALLOWED.contains(&(item, tag)) {
        let known: Vec<String> = ALLOWED.iter().map(|p| format!("{} on {}", p.1, p.0)).collect();
        return Err(E::Msg(format!("no evidence covers the tag {tag} on a pickup of {item}, the known pairs are: {}", known.join(", "))));
    }
    Ok(())
}

pub fn add(old: &Table, item: &str, name: &str, tags: &[String]) -> Res<Table> {
    let own: Vec<String> = old.tags_of(item);
    for t in tags {
        allowed(item, t)?;
        if !own.contains(t) {
            return Err(E::Msg(format!("the shipped row of {item} does not carry the tag {t}, so the pickup may not carry it")));
        }
    }
    let mut now: Table = old.clone();
    now.insert(name, tags)?;
    Ok(now)
}

pub fn audit(old: &Table, now: &Table, name: &str, tags: &[String]) -> Res<String> {
    if now.tags != old.tags || now.version != old.version {
        return Err(E::Format("audit: the tag list of the tag table changed"));
    }
    if !now.sorted() {
        return Err(E::Format("audit: the tag table is not sorted"));
    }
    if now.without(name) != old.without(name) {
        return Err(E::Format("audit: the tag table changed a row it started with"));
    }
    let want: BS<&str> = tags.iter().map(|t| t.as_str()).collect();
    let got: Vec<String> = now.tags_of(name);
    if got.iter().map(|t| t.as_str()).collect::<BS<&str>>() != want || now.find(name).is_none() {
        return Err(E::Msg(format!("audit: the tag row of {name} holds {got:?}")));
    }
    if let Some(t) = got.iter().find(|t| QUEST.contains(&t.as_str())) {
        return Err(E::Msg(format!("audit: the tag row of {name} holds the quest tag {t}")));
    }
    Ok(format!("{TAG}: {} -> {} rows, {} tags, new row {name} carries {got:?}, every other row unchanged", old.paths.len(), now.paths.len(), now.tags.len()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf as PB;

    use super::*;
    use crate::romfs::Romfs;

    fn actor(name: &str) -> [String; 3] {
        [HEAD.to_string(), name.to_string(), TAIL.to_string()]
    }

    fn small(n: usize) -> Table {
        let tags: Vec<String> = ["Bomb", "LetterBottle", "Rupee"].iter().map(|s| s.to_string()).collect();
        let mut paths: Vec<[String; 3]> = (0..n).map(|i| actor(&format!("Item_{i:03}"))).collect();
        paths.push(["Work/Scene/".to_string(), "Field".to_string(), ".engine__scene__SceneParam.gyml".to_string()]);
        let mut bits: Vec<bool> = vec![false; paths.len() * tags.len()];
        for i in 0..paths.len() {
            bits[i * 3 + i % 3] = i % 4 != 0;
        }
        Table { version: 7, paths, tags, bits }
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

    #[test]
    fn a_table_reads_back_from_its_own_bytes() {
        for n in [0, 1, 31, 32, 33, 70] {
            let t: Table = small(n);
            let b: Vec<u8> = t.write().unwrap();
            assert_eq!(Table::parse(&b).unwrap(), t, "{n}");
            let y: Byml = Byml::parse(&b).unwrap();
            assert_eq!(y.root.get(PATHS).and_then(|v| v.as_arr()).map(|a| a.len()), Some((n + 1) * 3));
        }
    }

    #[test]
    fn the_rank_table_counts_each_tag_up_to_the_end_of_every_block_of_32() {
        let t: Table = small(70);
        let r: Vec<u8> = t.ranks();
        assert_eq!(r.len(), 3 * 3 * 4);
        let word = |i: usize| u32::from_le_bytes([r[4 * i], r[4 * i + 1], r[4 * i + 2], r[4 * i + 3]]);
        for tag in 0..3 {
            for b in 0..3 {
                let want: usize = (0..71.min((b + 1) * 32)).filter(|i| t.bits[i * 3 + tag]).count();
                assert_eq!(word(tag * 3 + b) as usize, want, "tag {tag} block {b}");
            }
        }
    }

    #[test]
    fn a_new_row_lands_in_sorted_order_and_shifts_no_other_row() {
        let old: Table = small(40);
        let mut now: Table = old.clone();
        let at: usize = now.insert("Item_0105", &["LetterBottle".to_string()]).unwrap();
        assert_eq!(at, 11);
        assert!(now.sorted());
        assert_eq!(now.paths.len(), old.paths.len() + 1);
        assert_eq!(now.tags_of("Item_0105"), vec!["LetterBottle".to_string()]);
        for p in &old.paths {
            assert_eq!(now.tags_of(&p[1]), old.tags_of(&p[1]), "{}", p[1]);
        }
        assert_eq!(now.without("Item_0105"), old.without("Item_0105"));
        let again: usize = now.insert("Item_0105", &["LetterBottle".to_string()]).unwrap();
        assert_eq!(again, 11);
        assert_eq!(now.paths.len(), old.paths.len() + 1);
        assert_eq!(Table::parse(&now.write().unwrap()).unwrap(), now);
        assert!(now.insert("Item_X", &["NoSuchTag".to_string()]).is_err());
        let last: usize = now.insert("Obj_Z", &[]).unwrap();
        assert_eq!(last, 41);
        assert_eq!(now.paths[42][0], "Work/Scene/");
    }

    #[test]
    fn only_a_tag_with_evidence_is_written() {
        assert!(allowed("Item_Bottle_Letter", "LetterBottle").is_ok());
        assert!(allowed("Item_Hookshot", "LetterBottle").is_err());
        assert!(allowed("Item_Bottle_Letter", "Rupee").is_err());
        for t in QUEST {
            assert!(allowed("Item_Bottle_Letter", t).unwrap_err().to_string().contains("quest"), "{t}");
            assert!(allowed("Item_Arrow_Fire", t).is_err(), "{t}");
        }
        let mut old: Table = small(8);
        old.insert("Item_Bottle_Letter", &["LetterBottle".to_string()]).unwrap();
        old.insert("Item_Plain", &[]).unwrap();
        let now: Table = add(&old, "Item_Bottle_Letter", "Obj_Bottle_LetterLand", &["LetterBottle".to_string()]).unwrap();
        assert!(audit(&old, &now, "Obj_Bottle_LetterLand", &["LetterBottle".to_string()]).is_ok());
        assert!(add(&old, "Item_Plain", "Obj_Plain", &["LetterBottle".to_string()]).is_err());
        let mut bare: Table = small(8);
        bare.insert("Item_Bottle_Letter", &[]).unwrap();
        assert!(add(&bare, "Item_Bottle_Letter", "Obj_Bottle_LetterLand", &["LetterBottle".to_string()]).unwrap_err().to_string().contains("does not carry"));
    }

    #[test]
    fn the_audit_catches_a_changed_row_a_wrong_tag_and_a_broken_rank_table() {
        let mut old: Table = small(40);
        old.insert("Item_Bottle_Letter", &["LetterBottle".to_string()]).unwrap();
        let tags: Vec<String> = vec!["LetterBottle".to_string()];
        let now: Table = add(&old, "Item_Bottle_Letter", "Obj_New", &tags).unwrap();
        assert!(audit(&old, &now, "Obj_New", &tags).unwrap().contains("42 -> 43 rows"));
        let mut other: Table = now.clone();
        other.bits[5] = !other.bits[5];
        assert!(audit(&old, &other, "Obj_New", &tags).unwrap_err().to_string().contains("changed a row"));
        let mut more: Table = now.clone();
        more.insert("Obj_New", &["LetterBottle".to_string(), "Rupee".to_string()]).unwrap();
        assert!(audit(&old, &more, "Obj_New", &tags).unwrap_err().to_string().contains("holds"));
        assert!(audit(&old, &old, "Obj_New", &tags).is_err());
        let mut b: Vec<u8> = now.write().unwrap();
        let y: Byml = Byml::parse(&b).unwrap();
        let len: usize = now.ranks().len();
        let Some(Node::Bin(r)) = y.root.get(RANKS) else { panic!() };
        assert_eq!(r.len(), len);
        let at: usize = (0..b.len() - len).rev().find(|i| b[*i..*i + len] == r[..]).unwrap();
        b[at + len - 4] ^= 1;
        assert!(Table::parse(&b).unwrap_err().to_string().contains("rank table"));
    }

    #[test]
    fn the_real_tag_table_rewrites_byte_for_byte() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read_zs(TAG).unwrap();
        let t: Table = Table::parse(&raw).unwrap();
        assert_eq!((t.paths.len(), t.tags.len(), t.packed().len(), t.ranks().len()), (4349, 167, 90786, 90848));
        assert_eq!(t.bits.iter().filter(|b| **b).count(), 9428);
        assert_eq!(t.write().unwrap(), raw);
        assert_eq!(t.tags_of("Obj_Bottle_Letter"), vec!["LetterBottle".to_string()]);
        assert_eq!(t.tags_of("Item_Bottle_Letter"), vec!["LetterBottle".to_string()]);
        assert_eq!(t.tags_of("Obj_Arrow_Fire"), vec!["Obj_Fire_Arrow".to_string()]);
        assert!(t.find("Item_Arrow_Fire").is_none());
        assert!(t.find("Obj_Item_Base").is_none());
        for (item, tag) in ALLOWED {
            assert!(t.tags_of(item).contains(&tag.to_string()), "{item}");
        }
        for q in QUEST {
            assert!(t.tags.contains(&q.to_string()), "{q}");
        }
    }

    #[test]
    fn the_real_letter_row_adds_one_row_and_leaves_every_quest_tag_alone() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let old: Table = Table::parse(&rf.read_zs(TAG).unwrap()).unwrap();
        let tags: Vec<String> = vec!["LetterBottle".to_string()];
        let now: Table = add(&old, "Item_Bottle_Letter", "Obj_Bottle_LetterLand", &tags).unwrap();
        let b: Vec<u8> = now.write().unwrap();
        let back: Table = Table::parse(&b).unwrap();
        assert_eq!(back, now);
        assert_eq!((back.paths.len(), back.packed().len(), back.ranks().len()), (4350, 90807, 90848));
        assert_eq!(back.bits.iter().filter(|x| **x).count(), 9429);
        assert_eq!(back.find("Obj_Bottle_LetterLand"), old.find("Obj_Bottle_Letter").map(|i| i + 1));
        assert!(audit(&old, &back, "Obj_Bottle_LetterLand", &tags).is_ok());
        for (i, t) in old.tags.iter().enumerate() {
            let was: usize = (0..old.paths.len()).filter(|a| old.bits[a * 167 + i]).count();
            let is: usize = (0..back.paths.len()).filter(|a| back.bits[a * 167 + i]).count();
            assert_eq!(is, was + usize::from(t == "LetterBottle"), "{t}");
        }
        for p in &old.paths {
            assert_eq!(back.tags_of(&p[1]), old.tags_of(&p[1]), "{}", p[1]);
        }
        assert!(add(&old, "Item_Bottle_Letter", "Obj_Bottle_LetterLand", &["Obj_Fire_Arrow".to_string()]).is_err());
        assert!(add(&old, "Item_Hookshot", "Obj_Hookshot", &["skipDropPreLoad".to_string()]).is_err());
        assert!(add(&old, "Item_Arrow_Fire", "Obj_Arrow_Fire2", &["Obj_Fire_Arrow".to_string()]).is_err());
    }
}
