use crate::bfevfl::Val;
use crate::error::{Error as E, Res};
use crate::flow_extra::op::{How, Op};
use crate::json::{self, Json};

pub const JSON: &str = include_str!("../../../../data/flow_extra.json");

#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub name: String,
    pub why: String,
    pub ops: Vec<Op>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Extra {
    pub specs: Vec<Spec>,
}

fn text(j: &Json, k: &str) -> Res<String> {
    j.get(k)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| E::Msg(format!("flow_extra json: missing text field {k}")))
}

fn num(j: &Json, k: &str) -> Res<i64> {
    j.get(k).and_then(|v| v.as_i64()).ok_or_else(|| E::Msg(format!("flow_extra json: missing number field {k}")))
}

fn ev(j: &Json, k: &str) -> Res<u16> {
    u16::try_from(num(j, k)?).map_err(|_| E::Msg(format!("flow_extra json: {k} is not an event index")))
}

fn val(j: &Json) -> Res<Val> {
    match j {
        Json::Str(s) => Ok(match s.strip_prefix("actor:") {
            Some(a) => Val::Actor(a.to_string(), String::new()),
            None => Val::Str(s.clone()),
        }),
        Json::Int(n) => i32::try_from(*n).map(Val::Int).map_err(|_| E::Format("flow_extra json: number out of range")),
        Json::Bool(b) => Ok(Val::Bool(u32::from(*b))),
        _ => Err(E::Format("flow_extra json: a parameter value must be text, a whole number or a bool")),
    }
}

fn params(j: &Json) -> Res<Vec<(String, Val)>> {
    let a: &[Json] = j.get("params").and_then(|v| v.as_arr()).ok_or(E::Format("flow_extra json: missing params"))?;
    let mut out: Vec<(String, Val)> = Vec::new();
    for p in a {
        match p.as_arr() {
            Some([k, v]) => {
                let n: &str = k.as_str().ok_or(E::Format("flow_extra json: a parameter name must be text"))?;
                out.push((n.to_string(), val(v)?));
            }
            _ => return Err(E::Format("flow_extra json: a parameter must be a [name, value] pair")),
        }
    }
    Ok(out)
}

fn op(j: &Json) -> Res<Op> {
    let kind: String = text(j, "kind")?;
    let file: String = text(j, "file")?;
    if !file.ends_with(".bfevfl") {
        return Err(E::Msg(format!("flow_extra json: {file} is not an event flow file")));
    }
    if kind == "rename_actor" {
        return Ok(Op { file, index: 0, event: String::new(), how: How::Rename { old: text(j, "old")?, new: text(j, "new")? } });
    }
    let index: usize = usize::try_from(num(j, "index")?).map_err(|_| E::Format("flow_extra json: negative index"))?;
    let event: String = text(j, "event")?;
    let how: How = match kind.as_str() {
        "retarget" => How::Retarget {
            case: u32::try_from(num(j, "case")?).map_err(|_| E::Format("flow_extra json: bad case value"))?,
            old: ev(j, "old")?,
            new: ev(j, "new")?,
        },
        "swap_sub" => How::SwapSub { from: text(j, "from")?, to: text(j, "to")?, params: params(j)? },
        "query" => How::Query { from: text(j, "from")?, to: text(j, "to")?, params: params(j)? },
        "set" => How::Set { param: text(j, "param")?, old: text(j, "old")?, new: text(j, "new")? },
        "insert_set" => How::Insert { flag: text(j, "flag")? },
        "clone_sub" => How::Clone { param: text(j, "param")?, old: text(j, "old")?, new: text(j, "new")? },
        _ => return Err(E::Msg(format!("flow_extra json: unknown edit kind {kind}"))),
    };
    Ok(Op { file, index, event, how })
}

impl Extra {
    pub fn load() -> Res<Extra> {
        Extra::parse(JSON)
    }

    pub fn parse(t: &str) -> Res<Extra> {
        let j: Json = json::parse(t)?;
        if j.get("format").and_then(|v| v.as_i64()) != Some(1) {
            return Err(E::Format("flow_extra json: unsupported format"));
        }
        let a: &[Json] = j.get("locations").and_then(|v| v.as_arr()).ok_or(E::Format("flow_extra json: missing locations"))?;
        let mut specs: Vec<Spec> = Vec::new();
        for x in a {
            let mut ops: Vec<Op> = Vec::new();
            for o in x.get("ops").and_then(|v| v.as_arr()).ok_or(E::Format("flow_extra json: missing ops"))? {
                ops.push(op(o)?);
            }
            let name: String = text(x, "name")?;
            if specs.iter().any(|s| s.name == name) {
                return Err(E::Msg(format!("flow_extra json: {name} is listed twice")));
            }
            specs.push(Spec { name, why: text(x, "why")?, ops });
        }
        Ok(Extra { specs })
    }

    pub fn by_name(&self, n: &str) -> Option<&Spec> {
        self.specs.iter().find(|s| s.name == n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow_locations::{Data, Loc};

    #[test]
    fn embedded_json_parses_and_matches_the_gated_locations() {
        let x: Extra = Extra::load().unwrap();
        let d: Data = Data::load().unwrap();
        assert!(!x.specs.is_empty());
        for s in &x.specs {
            let l: &Loc = d.by_name(&s.name).unwrap();
            assert!(l.gated && !l.supported, "{}", s.name);
            assert!(!s.ops.is_empty() && !s.why.is_empty(), "{}", s.name);
        }
        for l in d.locs.iter().filter(|l| l.gated && !crate::scrubs_a::is_a(l) && !crate::scrubs_b::is_b(l)) {
            assert!(x.by_name(&l.name).is_some(), "{} is gated without edits", l.name);
        }
    }

    #[test]
    fn values_keep_their_types() {
        assert_eq!(val(&Json::Str("actor:Npc_A".to_string())).unwrap(), Val::Actor("Npc_A".to_string(), String::new()));
        assert_eq!(val(&Json::Str("x".to_string())).unwrap(), Val::Str("x".to_string()));
        assert_eq!(val(&Json::Int(-1)).unwrap(), Val::Int(-1));
        assert_eq!(val(&Json::Bool(true)).unwrap(), Val::Bool(1));
        assert!(val(&Json::Null).is_err());
    }

    #[test]
    fn bad_files_are_refused() {
        assert!(Extra::parse("{\"format\":2,\"locations\":[]}").is_err());
        let bad_kind: &str = "{\"format\":1,\"locations\":[{\"name\":\"A\",\"why\":\"w\",\"ops\":[{\"kind\":\"nope\",\"file\":\"F.bfevfl\",\"index\":1,\"event\":\"E\"}]}]}";
        assert!(Extra::parse(bad_kind).is_err());
        let bad_file: &str = "{\"format\":1,\"locations\":[{\"name\":\"A\",\"why\":\"w\",\"ops\":[{\"kind\":\"insert_set\",\"file\":\"F\",\"index\":1,\"event\":\"E\",\"flag\":\"X\"}]}]}";
        assert!(Extra::parse(bad_file).is_err());
        let twice: &str = "{\"format\":1,\"locations\":[{\"name\":\"A\",\"why\":\"w\",\"ops\":[]},{\"name\":\"A\",\"why\":\"w\",\"ops\":[]}]}";
        assert!(Extra::parse(twice).is_err());
    }
}
