use std::fmt;

use crate::error::{Error, Res};

const HARD_DEPTH: usize = 512;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub depth: usize,
    pub bytes: usize,
}

impl Limits {
    pub const DEFAULT: Limits = Limits { depth: 64, bytes: 16 << 20 };
}

fn fail<T>(m: &str, at: usize) -> Res<T> {
    Err(Error::Msg(format!("json: {m} at byte {at}")))
}

struct Parser<'a> {
    b: &'a [u8],
    t: &'a str,
    i: usize,
    max_depth: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn lit(&mut self, w: &str, v: Json) -> Res<Json> {
        let rest: &[u8] = self.b.get(self.i..).unwrap_or(&[]);
        if rest.starts_with(w.as_bytes()) {
            self.i += w.len();
            Ok(v)
        } else {
            fail("bad literal", self.i)
        }
    }

    fn value(&mut self, depth: usize) -> Res<Json> {
        self.ws();
        match self.peek() {
            None => fail("unexpected end", self.i),
            Some(b'n') => self.lit("null", Json::Null),
            Some(b't') => self.lit("true", Json::Bool(true)),
            Some(b'f') => self.lit("false", Json::Bool(false)),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b'[') => self.array(depth),
            Some(b'{') => self.object(depth),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.number(),
            Some(_) => fail("unexpected character", self.i),
        }
    }

    fn array(&mut self, depth: usize) -> Res<Json> {
        if depth >= self.max_depth {
            return fail("nesting too deep", self.i);
        }
        self.i += 1;
        let mut out: Vec<Json> = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Json::Arr(out));
        }
        loop {
            out.push(self.value(depth + 1)?);
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(Json::Arr(out));
                }
                _ => return fail("expected , or ]", self.i),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Res<Json> {
        if depth >= self.max_depth {
            return fail("nesting too deep", self.i);
        }
        let open: usize = self.i;
        self.i += 1;
        let mut out: Vec<(String, Json)> = Vec::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Json::Obj(out));
        }
        loop {
            self.ws();
            if self.peek() != Some(b'"') {
                return fail("expected a string key", self.i);
            }
            let k: String = self.string()?;
            self.ws();
            if self.peek() != Some(b':') {
                return fail("expected :", self.i);
            }
            self.i += 1;
            let v: Json = self.value(depth + 1)?;
            out.push((k, v));
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    break;
                }
                _ => return fail("expected , or }", self.i),
            }
        }
        let mut keys: Vec<&str> = out.iter().map(|(k, _)| k.as_str()).collect();
        keys.sort_unstable();
        for w in keys.windows(2) {
            if w[0] == w[1] {
                return Err(Error::Msg(format!(
                    "json: duplicate key {:?} in the object at byte {open}",
                    w[0]
                )));
            }
        }
        Ok(Json::Obj(out))
    }

    fn string(&mut self) -> Res<String> {
        self.i += 1;
        let mut out: String = String::new();
        let mut run: usize = self.i;
        loop {
            let c: u8 = match self.peek() {
                Some(c) => c,
                None => return fail("unterminated string", self.i),
            };
            match c {
                b'"' => {
                    out.push_str(self.t.get(run..self.i).unwrap_or(""));
                    self.i += 1;
                    return Ok(out);
                }
                b'\\' => {
                    out.push_str(self.t.get(run..self.i).unwrap_or(""));
                    self.i += 1;
                    self.escape(&mut out)?;
                    run = self.i;
                }
                0..=0x1F => return fail("control character in string", self.i),
                _ => self.i += 1,
            }
        }
    }

    fn hex4(&mut self) -> Res<u32> {
        let mut n: u32 = 0;
        for _ in 0..4 {
            let d: Option<u32> = self.peek().and_then(|c| (c as char).to_digit(16));
            match d {
                Some(d) => n = n * 16 + d,
                None => return fail("bad \\u escape", self.i),
            }
            self.i += 1;
        }
        Ok(n)
    }

    fn escape(&mut self, out: &mut String) -> Res<()> {
        let c: u8 = match self.peek() {
            Some(c) => c,
            None => return fail("bad escape", self.i),
        };
        self.i += 1;
        match c {
            b'"' => out.push('"'),
            b'\\' => out.push('\\'),
            b'/' => out.push('/'),
            b'b' => out.push('\u{8}'),
            b'f' => out.push('\u{c}'),
            b'n' => out.push('\n'),
            b'r' => out.push('\r'),
            b't' => out.push('\t'),
            b'u' => {
                let hi: u32 = self.hex4()?;
                let cp: u32 = if (0xD800..0xDC00).contains(&hi) {
                    if self.b.get(self.i) != Some(&b'\\') || self.b.get(self.i + 1) != Some(&b'u') {
                        return fail("lone surrogate", self.i);
                    }
                    self.i += 2;
                    let lo: u32 = self.hex4()?;
                    if !(0xDC00..0xE000).contains(&lo) {
                        return fail("bad surrogate pair", self.i);
                    }
                    0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                } else if (0xDC00..0xE000).contains(&hi) {
                    return fail("lone surrogate", self.i);
                } else {
                    hi
                };
                match char::from_u32(cp) {
                    Some(ch) => out.push(ch),
                    None => return fail("bad code point", self.i),
                }
            }
            _ => return fail("bad escape", self.i - 1),
        }
        Ok(())
    }

    fn digits(&mut self) -> bool {
        let s: usize = self.i;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.i += 1;
        }
        self.i > s
    }

    fn number(&mut self) -> Res<Json> {
        let s: usize = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        match self.peek() {
            Some(b'0') => self.i += 1,
            Some(c) if c.is_ascii_digit() => {
                self.digits();
            }
            _ => return fail("bad number", self.i),
        }
        let mut float: bool = false;
        if self.peek() == Some(b'.') {
            self.i += 1;
            if !self.digits() {
                return fail("bad number", self.i);
            }
            float = true;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !self.digits() {
                return fail("bad number", self.i);
            }
            float = true;
        }
        let txt: &str = self.t.get(s..self.i).unwrap_or("");
        if !float && let Ok(n) = txt.parse::<i64>() {
            return Ok(Json::Int(n));
        }
        match txt.parse::<f64>() {
            Ok(f) if f.is_finite() => Ok(Json::Num(f)),
            _ => fail("number out of range", s),
        }
    }
}

pub fn parse_with(text: &str, lim: &Limits) -> Res<Json> {
    if text.len() > lim.bytes {
        return Err(Error::Msg(format!(
            "json: input of {} bytes exceeds the {} byte limit",
            text.len(),
            lim.bytes
        )));
    }
    let t: &str = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut p: Parser = Parser { b: t.as_bytes(), t, i: 0, max_depth: lim.depth.min(HARD_DEPTH) };
    let v: Json = p.value(0)?;
    p.ws();
    if p.i != p.b.len() {
        return fail("trailing data", p.i);
    }
    Ok(v)
}

pub fn parse(text: &str) -> Res<Json> {
    parse_with(text, &Limits::DEFAULT)
}

fn put_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn newline(out: &mut String, pretty: bool, lvl: usize) {
    if pretty {
        out.push('\n');
        for _ in 0..lvl {
            out.push_str("  ");
        }
    }
}

impl Json {
    pub fn obj() -> Json {
        Json::Obj(Vec::new())
    }

    pub fn strs<'a>(items: impl IntoIterator<Item = &'a str>) -> Json {
        Json::Arr(items.into_iter().map(|s| Json::Str(s.to_string())).collect())
    }

    pub fn with(mut self, k: &str, v: Json) -> Json {
        self.put(k, v);
        self
    }

    pub fn put(&mut self, k: &str, v: Json) {
        if let Json::Obj(m) = self {
            for e in m.iter_mut() {
                if e.0 == k {
                    e.1 = v;
                    return;
                }
            }
            m.push((k.to_string(), v));
        }
    }

    pub fn get(&self, k: &str) -> Option<&Json> {
        match self {
            Json::Obj(m) => m.iter().find(|e| e.0 == k).map(|e| &e.1),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Int(n) => Some(*n as f64),
            Json::Num(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_obj(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Obj(m) => Some(m),
            _ => None,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Json::Null => "null",
            Json::Bool(_) => "boolean",
            Json::Int(_) | Json::Num(_) => "number",
            Json::Str(_) => "string",
            Json::Arr(_) => "array",
            Json::Obj(_) => "object",
        }
    }

    pub fn pretty(&self) -> String {
        let mut out: String = String::new();
        self.write(&mut out, true, 0);
        out
    }

    fn write(&self, out: &mut String, pretty: bool, lvl: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(n) => out.push_str(&n.to_string()),
            Json::Num(f) => {
                if f.is_finite() {
                    let s: String = f.to_string();
                    out.push_str(&s);
                    if !s.contains(['.', 'e', 'E']) {
                        out.push_str(".0");
                    }
                } else {
                    out.push_str("null");
                }
            }
            Json::Str(s) => put_str(out, s),
            Json::Arr(a) => {
                if a.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    newline(out, pretty, lvl + 1);
                    v.write(out, pretty, lvl + 1);
                }
                newline(out, pretty, lvl);
                out.push(']');
            }
            Json::Obj(m) => {
                if m.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    newline(out, pretty, lvl + 1);
                    put_str(out, k);
                    out.push(':');
                    if pretty {
                        out.push(' ');
                    }
                    v.write(out, pretty, lvl + 1);
                }
                newline(out, pretty, lvl);
                out.push('}');
            }
        }
    }
}

impl fmt::Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out: String = String::new();
        self.write(&mut out, false, 0);
        f.write_str(&out)
    }
}

impl From<bool> for Json {
    fn from(b: bool) -> Json {
        Json::Bool(b)
    }
}

impl From<i64> for Json {
    fn from(n: i64) -> Json {
        Json::Int(n)
    }
}

impl From<&str> for Json {
    fn from(s: &str) -> Json {
        Json::Str(s.to_string())
    }
}

impl From<String> for Json {
    fn from(s: String) -> Json {
        Json::Str(s)
    }
}

impl From<Vec<Json>> for Json {
    fn from(a: Vec<Json>) -> Json {
        Json::Arr(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(t: &str) -> Json {
        let a: Json = parse(t).unwrap();
        let c: String = a.to_string();
        assert_eq!(parse(&c).unwrap(), a);
        assert_eq!(parse(&a.pretty()).unwrap(), a);
        assert_eq!(a.to_string(), c);
        a
    }

    #[test]
    fn scalars() {
        assert_eq!(rt("null"), Json::Null);
        assert_eq!(rt(" true "), Json::Bool(true));
        assert_eq!(rt("false"), Json::Bool(false));
        assert_eq!(rt("0"), Json::Int(0));
        assert_eq!(rt("-0"), Json::Int(0));
        assert_eq!(rt("-17"), Json::Int(-17));
        assert_eq!(rt("9223372036854775807"), Json::Int(i64::MAX));
        assert_eq!(rt("-9223372036854775808"), Json::Int(i64::MIN));
        assert_eq!(rt("\"hi\""), Json::Str("hi".into()));
    }

    #[test]
    fn numbers() {
        assert_eq!(rt("1.5"), Json::Num(1.5));
        assert_eq!(rt("1e3"), Json::Num(1000.0));
        assert_eq!(rt("-2.5E-2"), Json::Num(-0.025));
        assert_eq!(rt("9223372036854775808"), Json::Num(9223372036854775808.0));
        assert_eq!(Json::Num(3.0).to_string(), "3.0");
        assert_eq!(parse("3.0").unwrap(), Json::Num(3.0));
        assert_eq!(Json::Num(f64::NAN).to_string(), "null");
        assert_eq!(Json::Num(f64::INFINITY).to_string(), "null");
        assert_eq!(rt("1e300").as_f64(), Some(1e300));
    }

    #[test]
    fn nested_round_trip_keeps_order() {
        let t: &str = r#"{"z":1,"a":[true,null,{"k":"v","b":[]}],"m":{}}"#;
        let a: Json = rt(t);
        assert_eq!(a.to_string(), t);
        assert_eq!(a.get("z"), Some(&Json::Int(1)));
        assert_eq!(a.get("a").and_then(|x| x.as_arr()).map(|x| x.len()), Some(3));
        assert_eq!(a.get("nope"), None);
    }

    #[test]
    fn string_escapes() {
        let a: Json = parse(r#""a\"b\\c\/d\b\f\n\r\t\u0041\u00e9""#).unwrap();
        assert_eq!(a.as_str(), Some("a\"b\\c/d\u{8}\u{c}\n\r\tA\u{e9}"));
        let s: String = Json::Str("q\"\\\n\u{1}\u{7f}\u{e9}\u{1F600}".into()).to_string();
        assert_eq!(s, "\"q\\\"\\\\\\n\\u0001\u{7f}\u{e9}\u{1F600}\"");
        assert_eq!(parse(&s).unwrap().as_str(), Some("q\"\\\n\u{1}\u{7f}\u{e9}\u{1F600}"));
    }

    #[test]
    fn surrogates() {
        assert_eq!(parse(r#""\ud83d\ude00""#).unwrap().as_str(), Some("\u{1F600}"));
        assert!(parse(r#""\ud83d""#).is_err());
        assert!(parse(r#""\ud83dx""#).is_err());
        assert!(parse(r#""\ude00""#).is_err());
        assert!(parse(r#""\ud83d\u0041""#).is_err());
    }

    #[test]
    fn malformed_is_rejected() {
        let bad: &[&str] = &[
            "", " ", "nul", "nullx", "tru", "[", "]", "[1,]", "[,1]", "[1 2]", "{", "}", "{\"a\"}",
            "{\"a\":}", "{\"a\":1,}", "{a:1}", "{'a':1}", "{\"a\":1 \"b\":2}", "01", "1.", ".5", "-", "+1",
            "1e", "1e+", "NaN", "Infinity", "\"abc", "\"a\nb\"", "\"\\x\"", "\"\\u12\"", "[1] 2",
            "\x2f\x2f c\n1", "\x2f* c *\x2f 1", "1e999", "{\"a\":1}}",
        ];
        for t in bad.iter() {
            assert!(parse(t).is_err(), "should reject {t:?}");
        }
    }

    #[test]
    fn duplicate_keys_rejected() {
        assert!(parse(r#"{"a":1,"b":2,"a":3}"#).is_err());
        assert!(parse(r#"{"a":{"x":1},"b":{"x":2}}"#).is_ok());
    }

    #[test]
    fn depth_limit() {
        let ok: String = format!("{}1{}", "[".repeat(64), "]".repeat(64));
        assert!(parse(&ok).is_ok());
        let deep: String = format!("{}1{}", "[".repeat(65), "]".repeat(65));
        assert!(parse(&deep).is_err());
        let huge: String = format!("{}1{}", "[".repeat(100_000), "]".repeat(100_000));
        assert!(parse(&huge).is_err());
        let lim: Limits = Limits { depth: 2, bytes: 1000 };
        assert!(parse_with("[[1]]", &lim).is_ok());
        assert!(parse_with("[[[1]]]", &lim).is_err());
        let objs: String = format!("{}1{}", "{\"a\":".repeat(70), "}".repeat(70));
        assert!(parse(&objs).is_err());
    }

    #[test]
    fn size_limit() {
        let lim: Limits = Limits { depth: 8, bytes: 10 };
        assert!(parse_with("[1,2,3,4]", &lim).is_ok());
        assert!(parse_with("[1,2,3,4,5]", &lim).is_err());
    }

    #[test]
    fn bom_is_tolerated() {
        assert_eq!(parse("\u{feff}[1]").unwrap(), Json::Arr(vec![Json::Int(1)]));
    }

    #[test]
    fn every_prefix_of_an_object_fails() {
        let t: &str = r#"{"a":[1,2.5,"x\n",{"b":null}],"c":true}"#;
        assert!(parse(t).is_ok());
        for n in 0..t.len() {
            assert!(parse(&t[..n]).is_err(), "prefix {n} parsed");
        }
    }

    #[test]
    fn single_byte_mutations_never_panic() {
        let t: &str = r#"{"a":[1,2.5,"x\n\u00e9",{"b":null}],"c":true}"#;
        for i in 0..t.len() {
            for c in ['"', '\\', '{', '}', '[', ']', ',', ':', '0', '-', 'e', ' ', 'x'] {
                let mut s: String = t.to_string();
                s.replace_range(i..i + 1, &c.to_string());
                let _ = parse(&s);
            }
        }
    }

    #[test]
    fn builders_and_accessors() {
        let o: Json = Json::obj()
            .with("n", 5.into())
            .with("s", "x".into())
            .with("b", true.into())
            .with("n", 6.into());
        assert_eq!(o.to_string(), r#"{"n":6,"s":"x","b":true}"#);
        assert_eq!(o.get("n").and_then(|x| x.as_i64()), Some(6));
        assert_eq!(o.get("s").and_then(|x| x.as_bool()), None);
        assert_eq!(Json::strs(["a", "b"]).to_string(), r#"["a","b"]"#);
        assert_eq!(o.kind(), "object");
        assert_eq!(Json::Null.kind(), "null");
    }

    #[test]
    fn pretty_layout() {
        let a: Json = parse(r#"{"a":[1,2],"b":{},"c":[]}"#).unwrap();
        assert_eq!(a.pretty(), "{\n  \"a\": [\n    1,\n    2\n  ],\n  \"b\": {},\n  \"c\": []\n}");
    }

    #[test]
    fn large_document() {
        let v: Vec<(String, Json)> = (0..20_000).map(|i| (format!("k{i}"), Json::Int(i))).collect();
        let o: Json = Json::Obj(v);
        let t: String = o.to_string();
        assert_eq!(parse(&t).unwrap(), o);
    }
}
