use std::collections::{BTreeMap as BM, HashMap as HM};
use std::fmt::Write as W;
use std::path::Path as P;

use crate::TITLE_ID;
use crate::byml::Byml;
use crate::chest_slot::LIST;
use crate::error::Res;
use crate::modpack::Pack;
use crate::placement::{banc_files, hash_of};
use crate::pot_slice::scan::resets;
use crate::pots::one_time;
use crate::romfs::Romfs;
use crate::rsizetable;
use crate::slice::banc::actors;
use crate::staging::{rewrite, Out, MARGIN};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    Pot,
    Crate,
    Flying,
    Beehive,
}

impl Class {
    pub const ALL: [Class; 4] = [Class::Pot, Class::Crate, Class::Flying, Class::Beehive];

    pub fn name(self) -> &'static str {
        match self {
            Class::Pot => "pot",
            Class::Crate => "crate",
            Class::Flying => "flying pot",
            Class::Beehive => "beehive",
        }
    }

    pub fn of(g: &str) -> Option<Class> {
        if g.starts_with("Obj_Pot_A_") {
            Some(Class::Pot)
        } else if g.starts_with("Obj_BrokenBox_A_") {
            Some(Class::Crate)
        } else if g.starts_with("Enemy_FlyingPot") {
            Some(Class::Flying)
        } else if g.starts_with("Obj_Kokiri_Beehive_A_") {
            Some(Class::Beehive)
        } else {
            None
        }
    }

    pub fn reset(self) -> i64 {
        match self {
            Class::Flying => 280,
            _ => 88,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub hashes: Vec<u64>,
    pub changed: BM<Class, usize>,
    pub no_row: BM<Class, usize>,
    pub other: BM<Class, usize>,
}

pub fn classes(rf: &mut Romfs) -> Res<BM<u64, Class>> {
    let mut out: BM<u64, Class> = BM::new();
    for f in banc_files(rf)? {
        let y: Byml = rf.read_byml(&f)?;
        for a in actors(&y) {
            let c: Option<Class> = a.get("Gyaml").and_then(|v| v.as_str()).and_then(Class::of);
            if let (Some(c), Some(h)) = (c, hash_of(a)) {
                out.insert(h, c);
            }
        }
    }
    Ok(out)
}

pub fn plan(rows: &HM<u64, i64>, seen: &BM<u64, Class>) -> Plan {
    let mut p: Plan = Plan { hashes: Vec::new(), changed: BM::new(), no_row: BM::new(), other: BM::new() };
    for (h, c) in seen {
        match rows.get(h) {
            None => *p.no_row.entry(*c).or_insert(0) += 1,
            Some(r) if *r == c.reset() => {
                p.hashes.push(*h);
                *p.changed.entry(*c).or_insert(0) += 1;
            }
            Some(_) => *p.other.entry(*c).or_insert(0) += 1,
        }
    }
    p
}

#[derive(Debug, Clone)]
pub struct Built {
    pub plan: Plan,
    pub old: usize,
    pub new: usize,
    pub table: bool,
    pub report: String,
}

fn count(m: &BM<Class, usize>, c: Class) -> usize {
    m.get(&c).copied().unwrap_or(0)
}

pub fn build(romfs: &P, load: &P, name: &str) -> Res<Built> {
    let mut rf: Romfs = Romfs::open(romfs)?;
    let seen: BM<u64, Class> = classes(&mut rf)?;
    let rows: HM<u64, i64> = resets(&rf.read_byml(LIST)?);
    let p: Plan = plan(&rows, &seen);
    let o: Out = rewrite(&mut rf, LIST, |raw| {
        let mut y: Byml = Byml::parse(raw)?;
        one_time(&mut y, &p.hashes)?;
        y.write()
    })?;
    let pk: Pack = Pack::begin(load, TITLE_ID, name)?;
    pk.write_romfs(&o.path, &o.z)?;
    let table: bool = o.new > o.old;
    if table {
        let mut t: rsizetable::Rst = rsizetable::load(&mut rf)?;
        t.grow(&o.path, o.old as u64, o.new as u64, MARGIN)?;
        pk.write_romfs(rsizetable::TABLE_PATH, &t.to_zs()?)?;
    }
    pk.commit()?;
    let mut report: String = format!("Mod folder: {}\n", pk.root.display());
    for c in Class::ALL {
        let _ = writeln!(
            report,
            "{}: {} rows made permanent, {} placed without a row, {} with another reset type",
            c.name(),
            count(&p.changed, c),
            count(&p.no_row, c),
            count(&p.other, c)
        );
    }
    let _ = writeln!(report, "GameDataList: {} -> {} bytes, size table {}", o.old, o.new, if table { "grown and written" } else { "not needed and not written" });
    Ok(Built { plan: p, old: o.old, new: o.new, table, report })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byml::Node;
    use crate::pots::TABLE;
    use std::path::PathBuf as PB;

    fn row(h: u64, t: i32, extra: bool) -> Node {
        let mut m: Vec<(String, Node)> = Vec::new();
        if extra {
            m.push(("ExtraByte".to_string(), Node::I32(4)));
        }
        m.push(("Hash".to_string(), Node::U64(h)));
        m.push(("ResetTypeValue".to_string(), Node::I32(t)));
        m.push(("SaveFileIndex".to_string(), Node::I32(0)));
        Node::Map(m)
    }

    fn list(rows: Vec<Node>) -> Byml {
        let data: Node = Node::Map(vec![(TABLE.to_string(), Node::Arr(rows))]);
        Byml { root: Node::Map(vec![("Data".to_string(), data)]), version: 2, big: false }
    }

    #[test]
    fn classes_follow_the_actor_type_and_expected_reset() {
        assert_eq!(Class::of("Obj_Pot_A_03"), Some(Class::Pot));
        assert_eq!(Class::of("Obj_Pot_A_03_Dream"), Some(Class::Pot));
        assert_eq!(Class::of("Obj_BrokenBox_A_02"), Some(Class::Crate));
        assert_eq!(Class::of("Enemy_FlyingPot_IceCavern"), Some(Class::Flying));
        assert_eq!(Class::of("Obj_Kokiri_Beehive_A_01"), Some(Class::Beehive));
        assert_eq!(Class::of("Obj_Rupee_Green"), None);
        assert_eq!((Class::Pot.reset(), Class::Flying.reset(), Class::Beehive.reset()), (88, 280, 88));
    }

    #[test]
    fn the_plan_picks_only_rows_of_the_expected_reset_type() {
        let rows: HM<u64, i64> = [(1, 88), (2, 280), (3, 24), (4, 280), (5, 88)].into_iter().collect();
        let seen: BM<u64, Class> = [(1, Class::Pot), (2, Class::Flying), (3, Class::Crate), (4, Class::Pot), (6, Class::Beehive)].into_iter().collect();
        let p: Plan = plan(&rows, &seen);
        assert_eq!(p.hashes, vec![1, 2]);
        assert_eq!((count(&p.changed, Class::Pot), count(&p.changed, Class::Flying)), (1, 1));
        assert_eq!((count(&p.other, Class::Crate), count(&p.other, Class::Pot)), (1, 1));
        assert_eq!(count(&p.no_row, Class::Beehive), 1);
    }

    #[test]
    fn only_the_planned_rows_change_in_a_synthetic_list() {
        let mut y: Byml = list(vec![row(1, 88, true), row(2, 280, false), row(3, 25, false), row(4, 88, true)]);
        let before: Byml = y.clone();
        one_time(&mut y, &[1, 2]).unwrap();
        let t: &[Node] = y.root.get("Data").unwrap().get(TABLE).unwrap().as_arr().unwrap();
        let b: &[Node] = before.root.get("Data").unwrap().get(TABLE).unwrap().as_arr().unwrap();
        assert_eq!(t[0], row(1, 24, false));
        assert_eq!(t[1], row(2, 24, false));
        assert_eq!(t[2], b[2]);
        assert_eq!(t[3], b[3]);
        let back: Byml = Byml::parse(&y.write().unwrap()).unwrap();
        assert_eq!(back.root, y.root);
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
    fn the_real_flag_list_changes_only_the_pot_like_rows() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read_zs(LIST).unwrap();
        let orig: Byml = Byml::parse(&raw).unwrap();
        assert_eq!(orig.write().unwrap(), raw, "the flag list does not round trip byte for byte");
        let seen: BM<u64, Class> = classes(&mut rf).unwrap();
        let rows: HM<u64, i64> = resets(&orig);
        let p: Plan = plan(&rows, &seen);
        assert!(p.hashes.len() > 400, "{}", p.hashes.len());
        for c in Class::ALL {
            assert!(count(&p.changed, c) > 0, "{}", c.name());
        }
        assert_eq!(p.changed.values().sum::<usize>(), p.hashes.len());
        let mut y: Byml = orig.clone();
        one_time(&mut y, &p.hashes).unwrap();
        let after: HM<u64, i64> = resets(&y);
        let table = |b: &Byml| -> Vec<Node> { b.root.get("Data").unwrap().get(TABLE).unwrap().as_arr().unwrap().to_vec() };
        let (a, b): (Vec<Node>, Vec<Node>) = (table(&orig), table(&y));
        assert_eq!(a.len(), b.len());
        let mut touched: usize = 0;
        for (x, z) in a.iter().zip(b.iter()) {
            let h: u64 = hash_of(x).unwrap();
            if p.hashes.contains(&h) {
                touched += 1;
                assert_eq!(after[&h], 24);
                assert!(z.get("ExtraByte").is_none());
                assert_eq!(z.get("Hash"), x.get("Hash"));
                assert_eq!(z.get("SaveFileIndex"), x.get("SaveFileIndex"));
            } else {
                assert_eq!(x, z);
            }
        }
        assert_eq!(touched, p.hashes.len());
        let (Node::Map(ma), Node::Map(mb)) = (&orig.root, &y.root) else {
            panic!("the flag list root is not a map");
        };
        assert_eq!(ma.len(), mb.len());
        for (k, v) in ma {
            if k == "Data" {
                let (Node::Map(da), Node::Map(db)) = (v, y.root.get("Data").unwrap()) else {
                    panic!("Data is not a map");
                };
                assert_eq!(da.len(), db.len());
                for (dk, dv) in da {
                    if dk != TABLE {
                        assert_eq!(Some(dv), y.root.get("Data").unwrap().get(dk), "{dk}");
                    }
                }
            } else {
                assert_eq!(Some(v), y.root.get(k), "{k}");
            }
        }
        let out: Vec<u8> = y.write().unwrap();
        assert!(out.len() <= raw.len());
        assert_eq!(Byml::parse(&out).unwrap().root, y.root);
        let id: Option<u32> = crate::flow_locations::dict_id(&rf.read(LIST).unwrap());
        let z: Vec<u8> = crate::zs::compress_raw(&out, id).unwrap();
        assert_eq!(rf.zs.decompress(&z).unwrap(), out);
    }
}
