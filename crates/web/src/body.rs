use ootrr_core::json::{self, Json, Limits as JsonLimits};

use crate::error::{Fail, Hr};
use crate::req::{Limits, Req};

pub const MAX_STRING: usize = 1 << 20;

pub fn json(r: &Req) -> Hr<Json> {
    let t: &str = std::str::from_utf8(&r.body).map_err(|_| Fail::bad("the body is not valid UTF-8"))?;
    let lim: JsonLimits = JsonLimits { depth: 32, bytes: Limits::DEFAULT.body };
    let j: Json = json::parse_with(t, &lim).map_err(|e| Fail::bad(&e.to_string()))?;
    if !matches!(j, Json::Obj(_)) {
        return Err(Fail::bad("the body must be a JSON object"));
    }
    Ok(j)
}

pub fn opt_text<'a>(j: &'a Json, k: &str) -> Hr<Option<&'a str>> {
    match j.get(k) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Str(s)) if s.len() <= Limits::DEFAULT.body => Ok(Some(s)),
        Some(Json::Str(_)) => Err(Fail::bad(&format!("field {k} is too long"))),
        Some(_) => Err(Fail::bad(&format!("field {k} must be a string"))),
    }
}

pub fn text<'a>(j: &'a Json, k: &str) -> Hr<&'a str> {
    opt_text(j, k)?.ok_or_else(|| Fail::bad(&format!("field {k} is missing")))
}

pub fn short<'a>(j: &'a Json, k: &str) -> Hr<&'a str> {
    let s: &str = text(j, k)?;
    if s.len() > MAX_STRING {
        return Err(Fail::bad(&format!("field {k} is too long")));
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rq(b: &str) -> Req {
        Req { method: "POST".into(), path: "/".into(), query: Vec::new(), heads: Vec::new(), body: b.as_bytes().to_vec() }
    }

    #[test]
    fn bodies_must_be_json_objects() {
        assert!(json(&rq("{\"a\":1}")).is_ok());
        for b in ["", "[1]", "3", "{", "{\"a\":1,\"a\":2}", "\u{0}"] {
            assert_eq!(json(&rq(b)).unwrap_err().status, 400, "{b:?}");
        }
        let bad: Req = Req { body: vec![0xff, 0xfe], ..rq("") };
        assert_eq!(json(&bad).unwrap_err().status, 400);
        let deep: String = format!("{}1{}", "{\"a\":".repeat(40), "}".repeat(40));
        assert_eq!(json(&rq(&deep)).unwrap_err().status, 400);
    }

    #[test]
    fn field_helpers() {
        let j: Json = json(&rq("{\"s\":\"x\",\"n\":1,\"z\":null}")).unwrap();
        assert_eq!(text(&j, "s").unwrap(), "x");
        assert!(text(&j, "n").is_err());
        assert!(text(&j, "missing").is_err());
        assert_eq!(opt_text(&j, "z").unwrap(), None);
        assert_eq!(opt_text(&j, "missing").unwrap(), None);
        assert_eq!(short(&j, "s").unwrap(), "x");
    }
}
