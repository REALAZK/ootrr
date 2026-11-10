use super::kind::Kind;
use super::model::{Row, Save};
use crate::error::{Error, Res};

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Bool(bool),
    Int(i32),
    UInt(u32),
    Float(f32),
    Enum(u32),
    Int64(i64),
    UInt64(u64),
    Vec3([f32; 3]),
    Str(String),
    Bin(Vec<u8>),
    List(Vec<Val>),
}

fn w32(b: &[u8], i: usize) -> u32 {
    let o: usize = 4 + 4 * i;
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn count(b: &[u8]) -> usize {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize
}

fn text(b: &[u8]) -> String {
    let n: usize = b.iter().position(|c| *c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..n]).into_owned()
}

fn wtext(b: &[u8]) -> String {
    let u: Vec<u16> = b
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|c| *c != 0)
        .collect();
    String::from_utf16_lossy(&u)
}

fn le64(b: &[u8]) -> [u8; 8] {
    [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]
}

fn elem(k: Kind, w: u32) -> Val {
    match k {
        Kind::IntArr => Val::Int(w as i32),
        Kind::FloatArr => Val::Float(f32::from_bits(w)),
        Kind::EnumArr => Val::Enum(w),
        _ => Val::UInt(w),
    }
}

pub fn read(r: &Row) -> Val {
    let b: &[u8] = &r.seg;
    match r.kind {
        Kind::Sep | Kind::Bool => Val::Bool(r.val != 0),
        Kind::Int => Val::Int(r.val as i32),
        Kind::UInt => Val::UInt(r.val),
        Kind::Float => Val::Float(f32::from_bits(r.val)),
        Kind::Enum => Val::Enum(r.val),
        Kind::BoolArr => {
            let n: usize = count(b);
            Val::List(
                (0..n)
                    .map(|i| Val::Bool(w32(b, i / 32) >> (i % 32) & 1 == 1))
                    .collect(),
            )
        }
        Kind::IntArr | Kind::UIntArr | Kind::FloatArr | Kind::EnumArr => {
            Val::List((0..count(b)).map(|i| elem(r.kind, w32(b, i))).collect())
        }
        Kind::Int64 => Val::Int64(i64::from_le_bytes(le64(b))),
        Kind::UInt64 => Val::UInt64(u64::from_le_bytes(le64(b))),
        Kind::UInt64Arr => Val::List(
            (0..count(b))
                .map(|i| Val::UInt64(u64::from_le_bytes(le64(&b[4 + 8 * i..]))))
                .collect(),
        ),
        Kind::Vec3 => Val::Vec3([
            f32::from_bits(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            f32::from_bits(u32::from_le_bytes([b[4], b[5], b[6], b[7]])),
            f32::from_bits(u32::from_le_bytes([b[8], b[9], b[10], b[11]])),
        ]),
        Kind::Str32 | Kind::Str64 => Val::Str(text(b)),
        Kind::Str64Arr => Val::List(
            (0..count(b))
                .map(|i| Val::Str(text(&b[4 + 64 * i..4 + 64 * i + 64])))
                .collect(),
        ),
        Kind::WStrArr => Val::List(
            (0..count(b))
                .map(|i| Val::Str(wtext(&b[4 + 32 * i..4 + 32 * i + 32])))
                .collect(),
        ),
        Kind::Bin => Val::Bin(b[4..].to_vec()),
        Kind::BinArr => {
            let mut out: Vec<Val> = Vec::new();
            let mut o: usize = 4;
            for _ in 0..count(b) {
                let n: usize = count(&b[o..]);
                out.push(Val::Bin(b[o + 4..o + 4 + n].to_vec()));
                o += 4 + n;
            }
            Val::List(out)
        }
    }
}

fn bad() -> Error {
    Error::Format("save: value does not fit the flag type")
}

fn word_of(k: Kind, v: &Val) -> Res<u32> {
    match (k, v) {
        (Kind::IntArr, Val::Int(x)) => Ok(*x as u32),
        (Kind::UIntArr, Val::UInt(x)) => Ok(*x),
        (Kind::FloatArr, Val::Float(x)) => Ok(x.to_bits()),
        (Kind::EnumArr, Val::Enum(x)) => Ok(*x),
        _ => Err(bad()),
    }
}

fn put_text(b: &mut [u8], s: &str) -> Res<()> {
    if s.len() >= b.len() {
        return Err(Error::Format("save: text is too long for the field"));
    }
    b.fill(0);
    b[..s.len()].copy_from_slice(s.as_bytes());
    Ok(())
}

impl Row {
    pub fn get(&self) -> Val {
        read(self)
    }

    pub fn put(&mut self, v: &Val) -> Res<()> {
        match (self.kind, v) {
            (Kind::Bool, Val::Bool(x)) => self.val = *x as u32,
            (Kind::Int, Val::Int(x)) => self.val = *x as u32,
            (Kind::UInt, Val::UInt(x)) => self.val = *x,
            (Kind::Float, Val::Float(x)) => self.val = x.to_bits(),
            (Kind::Enum, Val::Enum(x)) => self.val = *x,
            (Kind::Int64, Val::Int64(x)) => self.seg.copy_from_slice(&x.to_le_bytes()),
            (Kind::UInt64, Val::UInt64(x)) => self.seg.copy_from_slice(&x.to_le_bytes()),
            (Kind::Vec3, Val::Vec3(x)) => {
                for (i, f) in x.iter().enumerate() {
                    self.seg[4 * i..4 * i + 4].copy_from_slice(&f.to_le_bytes());
                }
            }
            (Kind::Str32 | Kind::Str64, Val::Str(s)) => put_text(&mut self.seg, s)?,
            _ => return Err(bad()),
        }
        Ok(())
    }

    pub fn put_at(&mut self, i: usize, v: &Val) -> Res<()> {
        let n: usize = if self.seg.len() >= 4 {
            count(&self.seg)
        } else {
            0
        };
        if i >= n {
            return Err(Error::Format("save: array index out of range"));
        }
        match (self.kind, v) {
            (Kind::BoolArr, Val::Bool(x)) => {
                let o: usize = 4 + 4 * (i / 32);
                let m: u32 = 1 << (i % 32);
                let mut w: u32 = w32(&self.seg, i / 32);
                if *x {
                    w |= m;
                } else {
                    w &= !m;
                }
                self.seg[o..o + 4].copy_from_slice(&w.to_le_bytes());
            }
            (Kind::UInt64Arr, Val::UInt64(x)) => {
                self.seg[4 + 8 * i..12 + 8 * i].copy_from_slice(&x.to_le_bytes());
            }
            (Kind::IntArr | Kind::UIntArr | Kind::FloatArr | Kind::EnumArr, _) => {
                let w: u32 = word_of(self.kind, v)?;
                self.seg[4 + 4 * i..8 + 4 * i].copy_from_slice(&w.to_le_bytes());
            }
            _ => return Err(bad()),
        }
        Ok(())
    }
}

impl Save {
    pub fn get(&self, h: u32) -> Option<Val> {
        self.row(h).map(read)
    }

    pub fn set(&mut self, h: u32, v: &Val) -> Res<()> {
        self.row_mut(h)
            .ok_or(Error::Format("save: flag is not stored in this file"))?
            .put(v)
    }

    pub fn set_at(&mut self, h: u32, i: usize, v: &Val) -> Res<()> {
        self.row_mut(h)
            .ok_or(Error::Format("save: flag is not stored in this file"))?
            .put_at(i, v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::hash_name;
    use crate::save::fixture::{flag_list, h, sample};
    use crate::save::read::parse;

    fn load() -> Save {
        parse(&sample(), &flag_list()).unwrap()
    }

    #[test]
    fn reads_every_kind() {
        let s: Save = load();
        assert_eq!(s.get(h("IsGet.Test_A")), Some(Val::Bool(true)));
        assert_eq!(s.get(h("PouchData.Test_A.StockNum")), Some(Val::Int(7)));
        assert_eq!(s.get(h("Step_Test")), Some(Val::Enum(hash_name("Ready"))));
        assert_eq!(
            s.get(h("Test_Arr")),
            Some(Val::List(vec![Val::Int(10), Val::Int(20), Val::Int(30)]))
        );
        assert_eq!(
            s.get(h("Test_Bits")),
            Some(Val::List(vec![
                Val::Bool(false),
                Val::Bool(false),
                Val::Bool(true),
                Val::Bool(false),
                Val::Bool(true)
            ]))
        );
        assert_eq!(
            s.get(h("Sequence_CurrentBanc")),
            Some(Val::Str("KokiriForest_Child".to_string()))
        );
        assert_eq!(
            s.get(h("PlayerStatus.SavePos")),
            Some(Val::Vec3([1.0, 2.0, 3.0]))
        );
        assert_eq!(s.get(h("PlayerStatus.SavePosRadY")), Some(Val::Float(0.5)));
        assert_eq!(s.get(h("Test_Bin")), Some(Val::Bin(vec![9; 8])));
        assert_eq!(s.get(h("Test_Big")), Some(Val::UInt64(5)));
        assert_eq!(s.get(h("Nothing")), None);
    }

    #[test]
    fn sets_round_trip() {
        let mut s: Save = load();
        s.set(h("PouchData.Test_A.StockNum"), &Val::Int(-3))
            .unwrap();
        s.set(h("Step_Test"), &Val::Enum(hash_name("Complete")))
            .unwrap();
        s.set(
            h("Sequence_CurrentBanc"),
            &Val::Str("Market_Adult".to_string()),
        )
        .unwrap();
        s.set(h("PlayerStatus.SavePos"), &Val::Vec3([4.0, 5.0, 6.0]))
            .unwrap();
        s.set(h("Test_Big"), &Val::UInt64(u64::MAX)).unwrap();
        assert_eq!(s.get(h("PouchData.Test_A.StockNum")), Some(Val::Int(-3)));
        assert_eq!(
            s.get(h("Step_Test")),
            Some(Val::Enum(hash_name("Complete")))
        );
        assert_eq!(
            s.get(h("Sequence_CurrentBanc")),
            Some(Val::Str("Market_Adult".to_string()))
        );
        assert_eq!(
            s.get(h("PlayerStatus.SavePos")),
            Some(Val::Vec3([4.0, 5.0, 6.0]))
        );
        assert_eq!(s.get(h("Test_Big")), Some(Val::UInt64(u64::MAX)));
    }

    #[test]
    fn element_edits_stay_in_range() {
        let mut s: Save = load();
        s.set_at(h("Test_Arr"), 1, &Val::Int(99)).unwrap();
        s.set_at(h("Test_Bits"), 0, &Val::Bool(true)).unwrap();
        s.set_at(h("Test_Bits"), 2, &Val::Bool(false)).unwrap();
        assert!(s.set_at(h("Test_Arr"), 3, &Val::Int(1)).is_err());
        assert!(s.set_at(h("Test_Bits"), 5, &Val::Bool(true)).is_err());
        assert_eq!(
            s.get(h("Test_Arr")),
            Some(Val::List(vec![Val::Int(10), Val::Int(99), Val::Int(30)]))
        );
        match s.get(h("Test_Bits")) {
            Some(Val::List(l)) => {
                assert_eq!(l[0], Val::Bool(true));
                assert_eq!(l[2], Val::Bool(false));
                assert_eq!(l[4], Val::Bool(true));
            }
            _ => panic!("not a list"),
        }
    }

    #[test]
    fn wrong_types_and_long_text_fail() {
        let mut s: Save = load();
        assert!(s.set(h("IsGet.Test_A"), &Val::Int(1)).is_err());
        assert!(s.set(h("Test_Arr"), &Val::Int(1)).is_err());
        assert!(
            s.set(h("Sequence_CurrentBanc"), &Val::Str("x".repeat(64)))
                .is_err()
        );
        assert!(s.set(h("Nothing"), &Val::Bool(true)).is_err());
        assert!(s.set_at(h("Test_Arr"), 0, &Val::UInt(1)).is_err());
    }
}
