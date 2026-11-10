use super::val::Val;
use crate::error::{Error, Res};
use crate::flags::{Body, Flag, FlagList, hash_name, parse_hash};

pub fn target<'a>(l: &'a FlagList, q: &str) -> Res<(&'a Flag, Option<usize>)> {
    let (n, i): (&str, Option<usize>) = match q.split_once('[') {
        Some((n, r)) => {
            let d: &str = r
                .strip_suffix(']')
                .ok_or(Error::Format("save: index needs a closing bracket"))?;
            let k: usize = d.parse().map_err(|_| Error::Format("save: bad index"))?;
            (n, Some(k))
        }
        None => (q, None),
    };
    let f: &Flag = l
        .find(n)
        .ok_or_else(|| Error::Msg(format!("save: no flag named {n}")))?;
    Ok((f, i))
}

pub fn hash_of(q: &str) -> u32 {
    match parse_hash(q) {
        Some(h) => h as u32,
        None => hash_name(q),
    }
}

fn enum_name(f: &Flag, h: u32) -> String {
    if let Body::Values(v) = &f.body {
        if let Some(n) = v.iter().find(|n| hash_name(n) == h) {
            return n.clone();
        }
    }
    format!("0x{h:08x}")
}

fn one(f: &Flag, v: &Val) -> String {
    match v {
        Val::Bool(x) => x.to_string(),
        Val::Int(x) => x.to_string(),
        Val::UInt(x) => x.to_string(),
        Val::Float(x) => x.to_string(),
        Val::Enum(x) => enum_name(f, *x),
        Val::Int64(x) => x.to_string(),
        Val::UInt64(x) => x.to_string(),
        Val::Vec3(x) => format!("{},{},{}", x[0], x[1], x[2]),
        Val::Str(s) => format!("{s:?}"),
        Val::Bin(b) => format!("<{} bytes>", b.len()),
        Val::List(l) => {
            let n: usize = l.len();
            let s: Vec<String> = l.iter().take(16).map(|x| one(f, x)).collect();
            let more: &str = if n > 16 { ", ..." } else { "" };
            format!("[{}{more}] ({n})", s.join(", "))
        }
    }
}

pub fn show(f: &Flag, v: &Val) -> String {
    one(f, v)
}

fn flt(s: &str) -> Res<f32> {
    s.parse().map_err(|_| Error::Format("save: bad number"))
}

pub fn parse(f: &Flag, s: &str, elem: bool) -> Res<Val> {
    let ty: &str = if elem {
        f.ty.trim_end_matches("Array")
    } else {
        f.ty.as_str()
    };
    if !elem && f.ty.ends_with("Array") {
        return Err(Error::Format(
            "save: arrays are set one element at a time as name[i]",
        ));
    }
    let bad = || Error::Format("save: value does not parse for this flag type");
    Ok(match ty {
        "Bool" => match s {
            "true" | "1" => Val::Bool(true),
            "false" | "0" => Val::Bool(false),
            _ => return Err(bad()),
        },
        "Int" => Val::Int(s.parse().map_err(|_| bad())?),
        "UInt" => Val::UInt(s.parse().map_err(|_| bad())?),
        "Int64" => Val::Int64(s.parse().map_err(|_| bad())?),
        "UInt64" => Val::UInt64(s.parse().map_err(|_| bad())?),
        "Float" => Val::Float(flt(s)?),
        "Enum" => {
            if let Body::Values(v) = &f.body {
                if v.iter().any(|n| n == s) {
                    return Ok(Val::Enum(hash_name(s)));
                }
            }
            match parse_hash(s) {
                Some(h) if s.starts_with("0x") => Val::Enum(h as u32),
                _ => return Err(Error::Msg(format!("save: {s} is not a value of this enum"))),
            }
        }
        "Vector3" => {
            let p: Vec<&str> = s.split(',').collect();
            if p.len() != 3 {
                return Err(bad());
            }
            Val::Vec3([flt(p[0])?, flt(p[1])?, flt(p[2])?])
        }
        "String32" | "String64" => Val::Str(s.to_string()),
        _ => return Err(bad()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::fixture::{flag_list, h};

    #[test]
    fn targets_split_indexes() {
        let l: FlagList = flag_list();
        let (f, i): (&Flag, Option<usize>) = target(&l, "Test_Arr[2]").unwrap();
        assert_eq!(f.ty, "IntArray");
        assert_eq!(i, Some(2));
        assert!(target(&l, "Test_Arr[x]").is_err());
        assert!(target(&l, "Test_Arr[2").is_err());
        assert!(target(&l, "No.Such.Flag").is_err());
        let hex: String = format!("0x{:08x}", h("IsGet.Test_A"));
        assert!(target(&l, &hex).is_ok());
    }

    #[test]
    fn values_parse_and_print() {
        let l: FlagList = flag_list();
        let e: &Flag = l.find("Step_Test").unwrap();
        let v: Val = parse(e, "Complete", false).unwrap();
        assert_eq!(v, Val::Enum(hash_name("Complete")));
        assert_eq!(show(e, &v), "Complete");
        assert!(parse(e, "Nope", false).is_err());
        let b: &Flag = l.find("IsGet.Test_A").unwrap();
        assert_eq!(parse(b, "true", false).unwrap(), Val::Bool(true));
        assert!(parse(b, "maybe", false).is_err());
        let p: &Flag = l.find("PlayerStatus.SavePos").unwrap();
        assert_eq!(
            parse(p, "1,2.5,-3", false).unwrap(),
            Val::Vec3([1.0, 2.5, -3.0])
        );
        assert!(parse(p, "1,2", false).is_err());
        let a: &Flag = l.find("Test_Arr").unwrap();
        assert!(parse(a, "5", false).is_err());
        assert_eq!(parse(a, "5", true).unwrap(), Val::Int(5));
    }
}
