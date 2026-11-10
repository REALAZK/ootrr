use crate::error::{Error as E, Res};

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, k: &str) -> Option<&Json> {
        match self {
            Json::Obj(v) => v.iter().find(|(a, _)| a == k).map(|(_, b)| b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    pub fn as_obj(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Obj(v) => Some(v.as_slice()),
            _ => None,
        }
    }
}

pub fn clean(text: &str) -> String {
    let mut out: String = String::with_capacity(text.len());
    let mut in_str: bool = false;
    let mut esc: bool = false;
    let mut skip: bool = false;
    for c in text.chars() {
        if skip {
            if c == '\n' {
                skip = false;
                out.push(c);
            }
            continue;
        }
        if in_str {
            if esc {
                esc = false;
                out.push(c);
            } else if c == '\\' {
                esc = true;
                out.push(c);
            } else if c == '"' {
                in_str = false;
                out.push(c);
            } else if c == '\n' || c == '\r' {
                out.push(' ');
            } else {
                out.push(c);
            }
        } else if c == '#' {
            skip = true;
        } else {
            if c == '"' {
                in_str = true;
            }
            out.push(c);
        }
    }
    out
}

struct Rd<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Rd<'a> {
    fn fail<T>(&self, m: &str) -> Res<T> {
        Err(E::Msg(format!("json: {m} at byte {}", self.i)))
    }

    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn eat(&mut self, c: u8) -> Res<()> {
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            self.fail("unexpected character")
        }
    }

    fn word(&mut self, w: &str, v: Json) -> Res<Json> {
        let n: usize = w.len();
        if self.b.len() >= self.i + n && &self.b[self.i..self.i + n] == w.as_bytes() {
            self.i += n;
            Ok(v)
        } else {
            self.fail("bad literal")
        }
    }

    fn hex4(&mut self) -> Res<u32> {
        if self.b.len() < self.i + 4 {
            return self.fail("short unicode escape");
        }
        let mut n: u32 = 0;
        for _ in 0..4 {
            let d: u32 = match (self.b[self.i] as char).to_digit(16) {
                Some(d) => d,
                None => return self.fail("bad unicode escape"),
            };
            n = n * 16 + d;
            self.i += 1;
        }
        Ok(n)
    }

    fn string(&mut self) -> Res<String> {
        self.eat(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let c: u8 = match self.peek() {
                Some(c) => c,
                None => return self.fail("unterminated string"),
            };
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e: u8 = match self.peek() {
                        Some(e) => e,
                        None => return self.fail("unterminated escape"),
                    };
                    self.i += 1;
                    match e {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'u' => {
                            let mut u: u32 = self.hex4()?;
                            if (0xD800..0xDC00).contains(&u)
                                && self.b.get(self.i) == Some(&b'\\')
                                && self.b.get(self.i + 1) == Some(&b'u')
                            {
                                self.i += 2;
                                let lo: u32 = self.hex4()?;
                                u = 0x10000 + ((u - 0xD800) << 10) + (lo.wrapping_sub(0xDC00) & 0x3FF);
                            }
                            let ch: char = char::from_u32(u).unwrap_or('\u{FFFD}');
                            let mut buf: [u8; 4] = [0; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return self.fail("bad escape"),
                    }
                }
                _ => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|_| E::Format("json: invalid utf-8"))
    }

    fn number(&mut self) -> Res<Json> {
        let s: usize = self.i;
        while self.i < self.b.len()
            && matches!(self.b[self.i], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E')
        {
            self.i += 1;
        }
        let t: &str = match std::str::from_utf8(&self.b[s..self.i]) {
            Ok(t) => t,
            Err(_) => return self.fail("bad number"),
        };
        match t.parse::<f64>() {
            Ok(n) => Ok(Json::Num(n)),
            Err(_) => self.fail("bad number"),
        }
    }

    fn val(&mut self, depth: usize) -> Res<Json> {
        if depth > 200 {
            return self.fail("nesting too deep");
        }
        self.ws();
        match self.peek() {
            None => self.fail("unexpected end"),
            Some(b'{') => {
                self.i += 1;
                let mut out: Vec<(String, Json)> = Vec::new();
                self.ws();
                if self.peek() == Some(b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(out));
                }
                loop {
                    self.ws();
                    let k: String = self.string()?;
                    self.ws();
                    self.eat(b':')?;
                    let v: Json = self.val(depth + 1)?;
                    out.push((k, v));
                    self.ws();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(out));
                        }
                        _ => return self.fail("expected comma or closing brace"),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut out: Vec<Json> = Vec::new();
                self.ws();
                if self.peek() == Some(b']') {
                    self.i += 1;
                    return Ok(Json::Arr(out));
                }
                loop {
                    out.push(self.val(depth + 1)?);
                    self.ws();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(out));
                        }
                        _ => return self.fail("expected comma or closing bracket"),
                    }
                }
            }
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => self.word("true", Json::Bool(true)),
            Some(b'f') => self.word("false", Json::Bool(false)),
            Some(b'n') => self.word("null", Json::Null),
            Some(_) => self.number(),
        }
    }
}

pub fn parse(text: &str) -> Res<Json> {
    let c: String = clean(text);
    let mut r: Rd = Rd { b: c.as_bytes(), i: 0 };
    let v: Json = r.val(0)?;
    r.ws();
    if r.i != r.b.len() {
        return r.fail("trailing data");
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_values() {
        let j: Json = parse(r#"{"a": [1, 2.5, "x", true, null], "b": {"c": "d"}}"#).unwrap();
        let a: &[Json] = j.get("a").unwrap().as_arr().unwrap();
        assert_eq!(a.len(), 5);
        assert_eq!(a[1].as_f64(), Some(2.5));
        assert_eq!(j.get("b").unwrap().get("c").unwrap().as_str(), Some("d"));
    }

    #[test]
    fn keeps_key_order() {
        let j: Json = parse(r#"{"z": 1, "a": 2, "m": 3}"#).unwrap();
        let k: Vec<&str> = j.as_obj().unwrap().iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(k, vec!["z", "a", "m"]);
    }

    #[test]
    fn strips_hash_comments_and_joins_string_lines() {
        let t: &str = "{\n  # note\n  \"k\": \"a\n   b\" # tail\n}";
        let j: Json = parse(t).unwrap();
        assert_eq!(j.get("k").unwrap().as_str(), Some("a    b"));
    }

    #[test]
    fn hash_inside_string_survives() {
        let j: Json = parse(r##"{"k": "a#b"}"##).unwrap();
        assert_eq!(j.get("k").unwrap().as_str(), Some("a#b"));
    }

    #[test]
    fn handles_escapes() {
        let j: Json = parse(r#"["a\"b", "Aé", "😀"]"#).unwrap();
        let a: &[Json] = j.as_arr().unwrap();
        assert_eq!(a[0].as_str(), Some("a\"b"));
        assert_eq!(a[1].as_str(), Some("A\u{e9}"));
        assert_eq!(a[2].as_str(), Some("\u{1F600}"));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse("{").is_err());
        assert!(parse("[1,]").is_err());
        assert!(parse("{\"a\" 1}").is_err());
        assert!(parse("[1] x").is_err());
        assert!(parse("").is_err());
    }
}
