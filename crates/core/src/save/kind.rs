use crate::error::{Error, Res};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Sep,
    Bool,
    Int,
    UInt,
    Float,
    Enum,
    BoolArr,
    IntArr,
    UIntArr,
    FloatArr,
    EnumArr,
    Int64,
    UInt64,
    UInt64Arr,
    Vec3,
    Str32,
    Str64,
    Str64Arr,
    WStrArr,
    Bin,
    BinArr,
}

impl Kind {
    pub fn of(ty: &str) -> Option<Kind> {
        Some(match ty {
            "Bool" => Kind::Bool,
            "Int" => Kind::Int,
            "UInt" => Kind::UInt,
            "Float" => Kind::Float,
            "Enum" => Kind::Enum,
            "BoolArray" => Kind::BoolArr,
            "IntArray" => Kind::IntArr,
            "UIntArray" => Kind::UIntArr,
            "FloatArray" => Kind::FloatArr,
            "EnumArray" => Kind::EnumArr,
            "Int64" => Kind::Int64,
            "UInt64" => Kind::UInt64,
            "UInt64Array" => Kind::UInt64Arr,
            "Vector3" => Kind::Vec3,
            "String32" => Kind::Str32,
            "String64" => Kind::Str64,
            "String64Array" => Kind::Str64Arr,
            "WString16Array" => Kind::WStrArr,
            "Binary" => Kind::Bin,
            "BinaryArray" => Kind::BinArr,
            _ => return None,
        })
    }

    pub fn inline(self) -> bool {
        matches!(
            self,
            Kind::Sep | Kind::Bool | Kind::Int | Kind::UInt | Kind::Float | Kind::Enum
        )
    }
}

pub fn word(b: &[u8], o: usize) -> Res<usize> {
    let s: &[u8] = b
        .get(o..o + 4)
        .ok_or(Error::Format("save: segment ends early"))?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize)
}

fn counted(b: &[u8], per: usize) -> Res<usize> {
    let n: usize = word(b, 0)?;
    n.checked_mul(per)
        .and_then(|x| x.checked_add(4))
        .ok_or(Error::Format("save: segment count overflows"))
}

pub fn seg_len(k: Kind, b: &[u8]) -> Res<usize> {
    let n: usize = match k {
        Kind::Sep | Kind::Bool | Kind::Int | Kind::UInt | Kind::Float | Kind::Enum => 0,
        Kind::BoolArr => 4 + 4 * word(b, 0)?.div_ceil(32),
        Kind::IntArr | Kind::UIntArr | Kind::FloatArr | Kind::EnumArr => counted(b, 4)?,
        Kind::Int64 | Kind::UInt64 => 8,
        Kind::UInt64Arr => counted(b, 8)?,
        Kind::Vec3 => 12,
        Kind::Str32 => 32,
        Kind::Str64 => 64,
        Kind::Str64Arr => counted(b, 64)?,
        Kind::WStrArr => counted(b, 32)?,
        Kind::Bin => 4 + word(b, 0)?,
        Kind::BinArr => {
            let c: usize = word(b, 0)?;
            let mut o: usize = 4;
            for _ in 0..c {
                o = o
                    .checked_add(4 + word(b, o)?)
                    .ok_or(Error::Format("save: segment count overflows"))?;
            }
            o
        }
    };
    if n > b.len() {
        return Err(Error::Format("save: segment runs past the data"));
    }
    Ok(n)
}
