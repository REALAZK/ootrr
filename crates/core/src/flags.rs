use std::collections::HashMap as Map;

use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::json::Json;
use crate::murmur::murmur3_32;

pub const KEYED: &str = "Bool64bitKey";

#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    Plain,
    Members(Vec<(u32, u32)>),
    Values(Vec<String>),
    Exp(Vec<(i32, u32)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Flag {
    pub hash: u64,
    pub ty: String,
    pub def: Json,
    pub reset: i64,
    pub save: i64,
    pub extra: Option<i64>,
    pub size: Option<i64>,
    pub body: Body,
    pub name: Option<String>,
    pub how: Option<String>,
}

pub struct FlagList {
    pub flags: Vec<Flag>,
    by_hash: Map<u64, usize>,
}

pub fn hash_name(s: &str) -> u32 {
    murmur3_32(s.as_bytes(), 0)
}

fn int(n: &Node) -> Option<i64> {
    match n {
        Node::I32(x) => Some(*x as i64),
        Node::U32(x) => Some(*x as i64),
        Node::I64(x) => Some(*x),
        Node::U64(x) => Some(*x as i64),
        _ => None,
    }
}

fn wide(n: &Node) -> Option<u64> {
    match n {
        Node::I32(x) => Some(*x as u32 as u64),
        Node::U32(x) => Some(*x as u64),
        Node::I64(x) => Some(*x as u64),
        Node::U64(x) => Some(*x),
        _ => None,
    }
}

fn scalar(n: &Node) -> Json {
    match n {
        Node::Bool(b) => Json::Bool(*b),
        Node::I32(x) => Json::Int(*x as i64),
        Node::U32(x) => Json::Int(*x as i64),
        Node::I64(x) => Json::Int(*x),
        Node::U64(x) => match i64::try_from(*x) {
            Ok(v) => Json::Int(v),
            Err(_) => Json::Str(x.to_string()),
        },
        Node::F32(x) => Json::Num(*x as f64),
        Node::F64(x) => Json::Num(*x),
        Node::Str(s) => Json::Str(s.clone()),
        Node::Map(m) => Json::Obj(m.iter().map(|(k, v)| (k.clone(), scalar(v))).collect()),
        Node::Arr(a) => Json::Arr(a.iter().map(scalar).collect()),
        _ => Json::Null,
    }
}

fn members(n: &Node) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for m in n.as_arr().unwrap_or(&[]) {
        let k: Option<u64> = m.get("Hash").and_then(wide);
        let v: Option<u64> = m.get("Value").and_then(wide);
        if let (Some(k), Some(v)) = (k, v) {
            out.push((k as u32, v as u32));
        }
    }
    out
}

fn exp(n: &Node) -> Vec<(i32, u32)> {
    let mut out: Vec<(i32, u32)> = Vec::new();
    for t in n.as_arr().unwrap_or(&[]) {
        let a: &[Node] = t.as_arr().unwrap_or(&[]);
        if let (Some(op), Some(v)) = (a.first().and_then(int), a.get(1).and_then(wide)) {
            out.push((op as i32, v as u32));
        }
    }
    out
}

fn raw_values(n: &Node) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in n.as_arr().unwrap_or(&[]) {
        if let Some(t) = s.as_str() {
            out.push(t.to_string());
        }
    }
    out
}

fn one(ty: &str, e: &Node) -> Res<Flag> {
    let hash: u64 = e
        .get("Hash")
        .and_then(wide)
        .ok_or(Error::Format("flags: entry without a hash"))?;
    let reset: i64 = e.get("ResetTypeValue").and_then(int).unwrap_or(0);
    let save: i64 = e.get("SaveFileIndex").and_then(int).unwrap_or(-1);
    let extra: Option<i64> = e.get("ExtraByte").and_then(int);
    let size: Option<i64> = e.get("TotalSize").or_else(|| e.get("Size")).and_then(int);
    let mut def: Json = e.get("DefaultValue").map(scalar).unwrap_or(Json::Null);
    let mut body: Body = Body::Plain;
    if ty == "Struct" {
        body = Body::Members(e.get("DefaultValue").map(members).unwrap_or_default());
        def = Json::Null;
    } else if ty == "BoolExp" {
        body = Body::Exp(e.get("Values").map(exp).unwrap_or_default());
    } else if let Some(r) = e.get("RawValues") {
        let names: Vec<String> = raw_values(r);
        if let Json::Int(d) = def {
            let h: u32 = d as u32;
            if let Some(n) = names.iter().find(|n| hash_name(n) == h) {
                def = Json::Str(n.clone());
            }
        }
        body = Body::Values(names);
    }
    Ok(Flag {
        hash,
        ty: ty.to_string(),
        def,
        reset,
        save,
        extra,
        size,
        body,
        name: None,
        how: None,
    })
}

impl FlagList {
    pub fn from_flags(flags: Vec<Flag>) -> FlagList {
        let mut by_hash: Map<u64, usize> = Map::with_capacity(flags.len());
        for (i, f) in flags.iter().enumerate() {
            by_hash.insert(f.hash, i);
        }
        FlagList { flags, by_hash }
    }

    pub fn from_byml(y: &Byml) -> Res<FlagList> {
        let data: &Node = y
            .root
            .get("Data")
            .ok_or(Error::Format("flags: no Data map"))?;
        let Node::Map(types) = data else {
            return Err(Error::Format("flags: Data is not a map"));
        };
        let mut flags: Vec<Flag> = Vec::new();
        for (ty, arr) in types {
            for e in arr.as_arr().unwrap_or(&[]) {
                flags.push(one(ty, e)?);
            }
        }
        Ok(FlagList::from_flags(flags))
    }

    pub fn len(&self) -> usize {
        self.flags.len()
    }

    pub fn is_empty(&self) -> bool {
        self.flags.is_empty()
    }

    pub fn by_hash(&self, h: u64) -> Option<&Flag> {
        self.by_hash.get(&h).map(|i| &self.flags[*i])
    }

    pub fn index_of(&self, h: u64) -> Option<usize> {
        self.by_hash.get(&h).copied()
    }

    pub fn find(&self, q: &str) -> Option<&Flag> {
        if let Some(h) = parse_hash(q) {
            return self.by_hash(h);
        }
        if let Some(f) = self.flags.iter().find(|f| f.name.as_deref() == Some(q)) {
            return Some(f);
        }
        self.by_hash(hash_name(q) as u64)
    }

    pub fn name(&mut self, h: u64, name: &str, how: &str) -> bool {
        let Some(i) = self.by_hash.get(&h).copied() else {
            return false;
        };
        let f: &mut Flag = &mut self.flags[i];
        if f.ty == KEYED || hash_name(name) as u64 != h || f.name.is_some() {
            return false;
        }
        f.name = Some(name.to_string());
        f.how = Some(how.to_string());
        true
    }

    pub fn tag(&mut self, h: u64, label: &str, how: &str) -> bool {
        let Some(i) = self.by_hash.get(&h).copied() else {
            return false;
        };
        let f: &mut Flag = &mut self.flags[i];
        if f.name.is_some() {
            return false;
        }
        f.name = Some(label.to_string());
        f.how = Some(how.to_string());
        true
    }

    pub fn named(&self) -> usize {
        self.flags.iter().filter(|f| f.name.is_some()).count()
    }
}

pub fn parse_hash(q: &str) -> Option<u64> {
    if let Some(h) = q.strip_prefix("0x") {
        return u64::from_str_radix(h, 16).ok();
    }
    if !q.is_empty() && q.bytes().all(|c| c.is_ascii_digit()) {
        return q.parse::<u64>().ok();
    }
    None
}

pub fn hex(h: u64, ty: &str) -> String {
    if ty == KEYED {
        format!("0x{h:016x}")
    } else {
        format!("0x{h:08x}")
    }
}

pub fn flag_json(f: &Flag) -> Json {
    let mut o: Json = Json::obj().with("hash", Json::Str(hex(f.hash, &f.ty)));
    if let Some(n) = &f.name {
        o.put("name", Json::Str(n.clone()));
        o.put("how", Json::Str(f.how.clone().unwrap_or_default()));
    }
    o.put("type", Json::Str(f.ty.clone()));
    match &f.body {
        Body::Members(m) => {
            let v: Vec<Json> = m
                .iter()
                .map(|x| {
                    Json::Arr(vec![
                        Json::Str(hex(x.0 as u64, "Bool")),
                        Json::Str(hex(x.1 as u64, "Bool")),
                    ])
                })
                .collect();
            o.put("members", Json::Arr(v));
        }
        Body::Values(v) => o.put("values", Json::strs(v.iter().map(|s| s.as_str()))),
        Body::Exp(t) => {
            let v: Vec<Json> = t
                .iter()
                .map(|x| {
                    Json::Arr(vec![
                        Json::Int(x.0 as i64),
                        Json::Str(hex(x.1 as u64, "Bool")),
                    ])
                })
                .collect();
            o.put("exp", Json::Arr(v));
        }
        Body::Plain => {}
    }
    if !matches!(f.def, Json::Null) {
        o.put("default", f.def.clone());
    }
    if let Some(x) = f.extra {
        o.put("extra", Json::Int(x));
    }
    if let Some(x) = f.size {
        o.put("size", Json::Int(x));
    }
    o.put("reset", Json::Int(f.reset));
    o.put("save", Json::Int(f.save));
    o
}

pub fn to_json(l: &FlagList) -> Json {
    let arr: Vec<Json> = l.flags.iter().map(flag_json).collect();
    Json::obj()
        .with("format", Json::Int(1))
        .with(
            "source",
            Json::Str("GameData/GameDataList.Product.110".to_string()),
        )
        .with("flags", Json::Arr(arr))
}

pub fn render(l: &FlagList, extra: Vec<(String, Json)>) -> String {
    let mut out: String = String::from(
        "{
\"format\":1,
\"source\":\"GameData/GameDataList.Product.110\",
",
    );
    for (k, v) in extra {
        out.push_str(&format!(
            "\"{k}\":{v},
"
        ));
    }
    out.push_str(
        "\"flags\":[
",
    );
    let n: usize = l.flags.len();
    for (i, f) in l.flags.iter().enumerate() {
        out.push_str(&flag_json(f).to_string());
        out.push_str(if i + 1 < n {
            ",
"
        } else {
            "
"
        });
    }
    out.push_str(
        "]
}
",
    );
    out
}

pub fn from_json(j: &Json) -> Res<FlagList> {
    let arr: &[Json] = j
        .get("flags")
        .and_then(|v| v.as_arr())
        .ok_or(Error::Format("flags json: no flags array"))?;
    let mut flags: Vec<Flag> = Vec::with_capacity(arr.len());
    for o in arr {
        let h: &str = o
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(Error::Format("flags json: entry without hash"))?;
        let hash: u64 = parse_hash(h).ok_or(Error::Format("flags json: bad hash"))?;
        let ty: String = o
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or(Error::Format("flags json: entry without type"))?
            .to_string();
        let name: Option<String> = o
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let how: Option<String> = o.get("how").and_then(|v| v.as_str()).map(|s| s.to_string());
        let def: Json = o.get("default").cloned().unwrap_or(Json::Null);
        let reset: i64 = o.get("reset").and_then(|v| v.as_i64()).unwrap_or(0);
        let save: i64 = o.get("save").and_then(|v| v.as_i64()).unwrap_or(-1);
        let extra: Option<i64> = o.get("extra").and_then(|v| v.as_i64());
        let size: Option<i64> = o.get("size").and_then(|v| v.as_i64());
        let body: Body = if let Some(m) = o.get("members").and_then(|v| v.as_arr()) {
            let mut out: Vec<(u32, u32)> = Vec::new();
            for x in m {
                let a: &[Json] = x.as_arr().unwrap_or(&[]);
                let k: Option<u64> = a.first().and_then(|v| v.as_str()).and_then(parse_hash);
                let v: Option<u64> = a.get(1).and_then(|v| v.as_str()).and_then(parse_hash);
                if let (Some(k), Some(v)) = (k, v) {
                    out.push((k as u32, v as u32));
                }
            }
            Body::Members(out)
        } else if let Some(v) = o.get("values").and_then(|v| v.as_arr()) {
            Body::Values(
                v.iter()
                    .filter_map(|x| x.as_str())
                    .map(|s| s.to_string())
                    .collect(),
            )
        } else if let Some(t) = o.get("exp").and_then(|v| v.as_arr()) {
            let mut out: Vec<(i32, u32)> = Vec::new();
            for x in t {
                let a: &[Json] = x.as_arr().unwrap_or(&[]);
                let op: Option<i64> = a.first().and_then(|v| v.as_i64());
                let v: Option<u64> = a.get(1).and_then(|v| v.as_str()).and_then(parse_hash);
                if let (Some(op), Some(v)) = (op, v) {
                    out.push((op as i32, v as u32));
                }
            }
            Body::Exp(out)
        } else {
            Body::Plain
        };
        flags.push(Flag {
            hash,
            ty,
            def,
            reset,
            save,
            extra,
            size,
            body,
            name,
            how,
        });
    }
    Ok(FlagList::from_flags(flags))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> FlagList {
        let a: Flag = Flag {
            hash: hash_name("IsGet.Item_Sword_001") as u64,
            ty: "Bool".to_string(),
            def: Json::Bool(false),
            reset: 8,
            save: 0,
            extra: None,
            size: None,
            body: Body::Plain,
            name: None,
            how: None,
        };
        let mut b: Flag = a.clone();
        b.hash = 0x1234_5678_9abc_def0;
        b.ty = KEYED.to_string();
        FlagList::from_flags(vec![a, b])
    }

    #[test]
    fn names_must_rehash() {
        let mut l: FlagList = sample();
        let h: u64 = hash_name("IsGet.Item_Sword_001") as u64;
        assert!(!l.name(h, "IsGet.Item_Sword_002", "t"));
        assert!(l.name(h, "IsGet.Item_Sword_001", "t"));
        assert!(!l.name(h, "IsGet.Item_Sword_001", "t"));
        assert_eq!(l.named(), 1);
    }

    #[test]
    fn lookup_by_name_or_hash() {
        let l: FlagList = sample();
        let h: u64 = hash_name("IsGet.Item_Sword_001") as u64;
        assert!(l.find("IsGet.Item_Sword_001").is_some());
        assert!(l.find(&h.to_string()).is_some());
        assert!(l.find(&format!("0x{h:08x}")).is_some());
        assert!(l.find("IsGet.Nothing").is_none());
        assert!(l.find("0x123456789abcdef0").is_some());
    }

    #[test]
    fn render_parses_back() {
        let mut l: FlagList = sample();
        let h: u64 = hash_name("IsGet.Item_Sword_001") as u64;
        l.name(h, "IsGet.Item_Sword_001", "t");
        let text: String = render(&l, vec![("note".to_string(), Json::Str("x".to_string()))]);
        let j: Json = crate::json::parse(&text).unwrap();
        assert_eq!(j.get("note").and_then(|v| v.as_str()), Some("x"));
        let back: FlagList = from_json(&j).unwrap();
        assert_eq!(back.flags[0].name, l.flags[0].name);
    }

    #[test]
    fn json_round_trip() {
        let mut l: FlagList = sample();
        let h: u64 = hash_name("IsGet.Item_Sword_001") as u64;
        l.name(h, "IsGet.Item_Sword_001", "t");
        let j: Json = to_json(&l);
        let back: FlagList = from_json(&j).unwrap();
        assert_eq!(back.flags.len(), 2);
        assert_eq!(back.flags[0].name, l.flags[0].name);
        assert_eq!(back.flags[1].hash, 0x1234_5678_9abc_def0);
    }
}
