use crate::error::{Error, Res};

const MAX_DEPTH: usize = 32;

#[derive(Debug, Clone, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    pub fn get(&self, k: &str) -> Option<&J> {
        match self {
            J::Obj(m) => m.iter().find(|e| e.0 == k).map(|e| &e.1),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[J]> {
        match self {
            J::Arr(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            J::Num(s) => s.parse().ok(),
            _ => None,
        }
    }
}

struct Pr<'a> {
    b: &'a [u8],
    i: usize,
}

fn bad(m: &'static str) -> Error {
    Error::Format(m)
}

impl<'a> Pr<'a> {
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
            Err(bad("json: unexpected character"))
        }
    }

    fn lit(&mut self, s: &str, v: J) -> Res<J> {
        let e: usize = self.i + s.len();
        if self.b.get(self.i..e) == Some(s.as_bytes()) {
            self.i = e;
            Ok(v)
        } else {
            Err(bad("json: bad literal"))
        }
    }

    fn hex4(&mut self) -> Res<u32> {
        let h: &[u8] = self.b.get(self.i..self.i + 4).ok_or(bad("json: short escape"))?;
        let s: &str = std::str::from_utf8(h).map_err(|_| bad("json: bad escape"))?;
        let n: u32 = u32::from_str_radix(s, 16).map_err(|_| bad("json: bad escape"))?;
        self.i += 4;
        Ok(n)
    }

    fn string(&mut self) -> Res<String> {
        self.eat(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let c: u8 = self.peek().ok_or(bad("json: unterminated string"))?;
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e: u8 = self.peek().ok_or(bad("json: unterminated string"))?;
                    self.i += 1;
                    let ch: char = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let mut n: u32 = self.hex4()?;
                            if (0xD800..0xDC00).contains(&n) {
                                self.eat(b'\\')?;
                                self.eat(b'u')?;
                                let lo: u32 = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return Err(bad("json: bad surrogate"));
                                }
                                n = 0x10000 + ((n - 0xD800) << 10) + (lo - 0xDC00);
                            }
                            char::from_u32(n).ok_or(bad("json: bad code point"))?
                        }
                        _ => return Err(bad("json: bad escape")),
                    };
                    let mut t: [u8; 4] = [0; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut t).as_bytes());
                }
                _ => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|_| bad("json: invalid utf-8"))
    }

    fn number(&mut self) -> Res<J> {
        let s: usize = self.i;
        while self.i < self.b.len() && matches!(self.b[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
            self.i += 1;
        }
        if s == self.i {
            return Err(bad("json: bad number"));
        }
        let t: &str = std::str::from_utf8(&self.b[s..self.i]).map_err(|_| bad("json: bad number"))?;
        Ok(J::Num(t.to_string()))
    }

    fn val(&mut self, depth: usize) -> Res<J> {
        if depth > MAX_DEPTH {
            return Err(bad("json: nesting too deep"));
        }
        self.ws();
        match self.peek().ok_or(bad("json: unexpected end"))? {
            b'{' => {
                self.i += 1;
                let mut m: Vec<(String, J)> = Vec::new();
                self.ws();
                if self.peek() == Some(b'}') {
                    self.i += 1;
                    return Ok(J::Obj(m));
                }
                loop {
                    self.ws();
                    let k: String = self.string()?;
                    self.ws();
                    self.eat(b':')?;
                    let v: J = self.val(depth + 1)?;
                    m.push((k, v));
                    self.ws();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(J::Obj(m));
                        }
                        _ => return Err(bad("json: bad object")),
                    }
                }
            }
            b'[' => {
                self.i += 1;
                let mut a: Vec<J> = Vec::new();
                self.ws();
                if self.peek() == Some(b']') {
                    self.i += 1;
                    return Ok(J::Arr(a));
                }
                loop {
                    a.push(self.val(depth + 1)?);
                    self.ws();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(J::Arr(a));
                        }
                        _ => return Err(bad("json: bad array")),
                    }
                }
            }
            b'"' => Ok(J::Str(self.string()?)),
            b't' => self.lit("true", J::Bool(true)),
            b'f' => self.lit("false", J::Bool(false)),
            b'n' => self.lit("null", J::Null),
            _ => self.number(),
        }
    }
}

pub fn parse(s: &str) -> Res<J> {
    let mut p: Pr = Pr { b: s.as_bytes(), i: 0 };
    let v: J = p.val(0)?;
    p.ws();
    if p.i != p.b.len() {
        return Err(bad("json: trailing data"));
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_values() {
        let j: J = parse(" {\"a\":[1,-2,true,null,\"x\"],\"b\":{\"c\":\"d\"}} ").unwrap();
        let a: &[J] = j.get("a").unwrap().as_arr().unwrap();
        assert_eq!(a.len(), 5);
        assert_eq!(a[1].as_i64(), Some(-2));
        assert_eq!(a[2], J::Bool(true));
        assert_eq!(a[3], J::Null);
        assert_eq!(j.get("b").unwrap().get("c").unwrap().as_str(), Some("d"));
    }

    #[test]
    fn decodes_escapes_and_surrogates() {
        let j: J = parse("\"a\\n\\\"\\u0041\\ud83d\\ude00\"").unwrap();
        assert_eq!(j.as_str(), Some("a\n\"A\u{1F600}"));
    }

    #[test]
    fn rejects_malformed_input() {
        assert!(parse("{\"a\":1,}").is_err());
        assert!(parse("[1 2]").is_err());
        assert!(parse("\"abc").is_err());
        assert!(parse("{} x").is_err());
        assert!(parse("\"\\ud83d\"").is_err());
        assert!(parse("").is_err());
    }

    #[test]
    fn limits_nesting_depth() {
        let deep: String = "[".repeat(64) + &"]".repeat(64);
        assert!(parse(&deep).is_err());
    }

    #[test]
    fn handles_empty_containers() {
        assert_eq!(parse("[]").unwrap(), J::Arr(Vec::new()));
        assert_eq!(parse("{}").unwrap(), J::Obj(Vec::new()));
    }
}
