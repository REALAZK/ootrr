use std::collections::HashMap as Map;

use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::flags::hash_name;
use crate::json::Json;
use crate::flow_sim::state::{State, V};
use crate::romfs::Romfs;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    Bool,
    Int,
    Enum,
    Exp,
    Str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tok {
    pub op: i32,
    pub a: u32,
    pub b: u32,
}

#[derive(Clone, Debug)]
pub struct Def {
    pub ty: Ty,
    pub dv: V,
    pub vals: Vec<String>,
    pub exp: Vec<Tok>,
}

#[derive(Clone, Debug)]
pub struct Chal {
    pub name: String,
    pub step: u32,
    pub dep: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct Db {
    pub defs: Map<u32, Def>,
    pub names: Map<u32, String>,
    pub chals: Vec<Chal>,
}

fn num(n: &Node) -> Option<i64> {
    match n {
        Node::I32(x) => Some(*x as i64),
        Node::U32(x) => Some(*x as i64),
        Node::I64(x) => Some(*x),
        Node::U64(x) => Some(*x as i64),
        _ => None,
    }
}

fn wide(n: &Node) -> Option<u32> {
    match n {
        Node::I32(x) => Some(*x as u32),
        Node::U32(x) => Some(*x),
        Node::I64(x) => Some(*x as u32),
        Node::U64(x) => Some(*x as u32),
        _ => None,
    }
}

fn tok(n: &Node) -> Option<Tok> {
    let a: &[Node] = n.as_arr()?;
    let op: i32 = num(a.first()?)? as i32;
    Some(Tok { op, a: a.get(1).and_then(wide).unwrap_or(0), b: a.get(2).and_then(wide).unwrap_or(0) })
}

fn one(ty: &str, e: &Node) -> Option<(u32, Def)> {
    let h: u32 = e.get("Hash").and_then(wide)?;
    let dv: Option<&Node> = e.get("DefaultValue");
    let vals: Vec<String> = e
        .get("RawValues")
        .and_then(|r| r.as_arr())
        .map(|r| r.iter().filter_map(|s| s.as_str().map(|t| t.to_string())).collect())
        .unwrap_or_default();
    let d: Def = match ty {
        "Bool" => Def { ty: Ty::Bool, dv: V::B(matches!(dv, Some(Node::Bool(true)))), vals, exp: Vec::new() },
        "Int" | "UInt" => Def { ty: Ty::Int, dv: V::I(dv.and_then(num).unwrap_or(0)), vals, exp: Vec::new() },
        "Enum" => Def { ty: Ty::Enum, dv: V::E(dv.and_then(wide).unwrap_or(0)), vals, exp: Vec::new() },
        "String64" => Def {
            ty: Ty::Str,
            dv: V::S(dv.and_then(|s| s.as_str()).unwrap_or("").to_string()),
            vals,
            exp: Vec::new(),
        },
        "BoolExp" => Def {
            ty: Ty::Exp,
            dv: V::B(false),
            vals,
            exp: e.get("Values").and_then(|v| v.as_arr()).map(|v| v.iter().filter_map(tok).collect()).unwrap_or_default(),
        },
        _ => return None,
    };
    Some((h, d))
}

impl Db {
    pub fn load(rf: &mut Romfs) -> Res<Db> {
        let y: Byml = rf.read_byml("GameData/GameDataList.Product.110.byml.zs")?;
        let data: &Node = y.root.get("Data").ok_or(Error::Format("flow sim: no Data map"))?;
        let Node::Map(types) = data else {
            return Err(Error::Format("flow sim: Data is not a map"));
        };
        let mut db: Db = Db::default();
        for (ty, arr) in types {
            for e in arr.as_arr().unwrap_or(&[]) {
                if let Some((h, d)) = one(ty, e) {
                    db.defs.insert(h, d);
                }
            }
        }
        db.load_deps(rf)?;
        Ok(db)
    }

    fn load_deps(&mut self, rf: &mut Romfs) -> Res<()> {
        let y: Byml = rf.read_byml("RSDB/Challenge.Product.111.rstbl.byml.zs")?;
        for r in y.root.as_arr().unwrap_or(&[]) {
            let id: &str = r.get("__RowId").and_then(|s| s.as_str()).unwrap_or("");
            let n: &str = id.rsplit('/').next().unwrap_or("").split('.').next().unwrap_or("");
            let d: &str = r.get("DependFlagName").and_then(|s| s.as_str()).unwrap_or("");
            if n.is_empty() {
                continue;
            }
            let step: u32 = self.reg(&format!("Step_{n}"));
            let dep: Option<u32> = if d.is_empty() { None } else { Some(self.reg(d)) };
            self.chals.push(Chal { name: n.to_string(), step, dep });
        }
        Ok(())
    }

    pub fn add_names(&mut self, j: &Json) {
        for f in j.get("flags").and_then(|v| v.as_arr()).unwrap_or(&[]) {
            let n: Option<&str> = f.get("name").and_then(|v| v.as_str());
            let h: Option<u32> = f.get("hash").and_then(|v| v.as_str()).and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok());
            if let (Some(n), Some(h)) = (n, h) {
                self.names.entry(h).or_insert_with(|| n.to_string());
            }
        }
    }

    pub fn reg(&mut self, n: &str) -> u32 {
        let h: u32 = hash_name(n);
        self.names.entry(h).or_insert_with(|| n.to_string());
        h
    }

    pub fn name(&self, h: u32) -> String {
        self.names.get(&h).cloned().unwrap_or_else(|| format!("{h:#010x}"))
    }

    pub fn get(&self, st: &State, h: u32) -> Option<V> {
        if let Some(v) = st.v.get(&h) {
            return Some(v.clone());
        }
        self.defs.get(&h).map(|d| d.dv.clone())
    }

    pub fn int(&self, st: &State, h: u32) -> Option<i64> {
        match self.get(st, h)? {
            V::I(x) => Some(x),
            _ => None,
        }
    }

    pub fn enum_name(&self, h: u32, v: u32) -> String {
        let d: Option<&Def> = self.defs.get(&h);
        d.and_then(|d| d.vals.iter().find(|n| hash_name(n) == v).cloned()).unwrap_or_else(|| format!("{v:#x}"))
    }

    pub fn show(&self, h: u32, v: &V) -> String {
        match v {
            V::B(b) => b.to_string(),
            V::I(i) => i.to_string(),
            V::E(e) => self.enum_name(h, *e),
            V::S(s) => format!("{s:?}"),
        }
    }

    pub fn parse(&self, h: u32, s: &str) -> Res<V> {
        let d: &Def = self.defs.get(&h).ok_or_else(|| Error::Msg(format!("unknown flag {}", self.name(h))))?;
        match d.ty {
            Ty::Bool | Ty::Exp => match s {
                "true" | "1" => Ok(V::B(true)),
                "false" | "0" => Ok(V::B(false)),
                _ => Err(Error::Msg(format!("{s} is not a bool"))),
            },
            Ty::Int => s.parse::<i64>().map(V::I).map_err(|_| Error::Msg(format!("{s} is not an int"))),
            Ty::Enum => {
                if d.vals.iter().any(|n| n == s) {
                    Ok(V::E(hash_name(s)))
                } else {
                    Err(Error::Msg(format!("{s} is not a value of {} ({})", self.name(h), d.vals.join(", "))))
                }
            }
            Ty::Str => Ok(V::S(s.to_string())),
        }
    }

    pub fn from_defs(defs: Vec<(&str, Def)>) -> Db {
        let mut db: Db = Db::default();
        for (n, d) in defs {
            let h: u32 = db.reg(n);
            db.defs.insert(h, d);
        }
        db
    }
}
