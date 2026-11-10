use std::io::Read;

use crate::error::{Fail, Hr};

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub head: usize,
    pub body: usize,
}

impl Limits {
    pub const DEFAULT: Limits = Limits { head: 16 * 1024, body: 8 * 1024 * 1024 };
}

#[derive(Debug, Clone, PartialEq)]
pub struct Req {
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub heads: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Req {
    pub fn head(&self, k: &str) -> Option<&str> {
        self.heads.iter().find(|h| h.0 == k).map(|h| h.1.as_str())
    }

    pub fn param(&self, k: &str) -> Option<&str> {
        self.query.iter().find(|q| q.0 == k).map(|q| q.1.as_str())
    }
}

const MAX_HEADS: usize = 100;
const MAX_TARGET: usize = 2048;
const MAX_PARAMS: usize = 32;
const ONCE: [&str; 6] = ["host", "content-length", "content-type", "origin", "x-ootrr-token", "sec-fetch-site"];

fn hexv(c: u8) -> Option<u8> {
    (c as char).to_digit(16).map(|d| d as u8)
}

pub fn pct_decode(s: &str, plus: bool) -> Option<String> {
    let b: &[u8] = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i: usize = 0;
    while let Some(c) = b.get(i) {
        match *c {
            b'%' => {
                let h: u8 = hexv(*b.get(i + 1)?)?;
                let l: u8 = hexv(*b.get(i + 2)?)?;
                out.push(h * 16 + l);
                i += 3;
            }
            b'+' if plus => {
                out.push(b' ');
                i += 1;
            }
            x => {
                out.push(x);
                i += 1;
            }
        }
    }
    let t: String = String::from_utf8(out).ok()?;
    if t.chars().any(|c| c.is_control()) {
        return None;
    }
    Some(t)
}

fn token_ok(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&c))
}

fn parse_target(t: &str) -> Hr<(String, Vec<(String, String)>)> {
    if t.len() > MAX_TARGET {
        return Err(Fail::new(414, "the request target is too long"));
    }
    if !t.starts_with('/') || t.starts_with("//") || t.contains('#') {
        return Err(Fail::bad("bad request target"));
    }
    let (p, q): (&str, &str) = t.split_once('?').unwrap_or((t, ""));
    let path: String = pct_decode(p, false).ok_or_else(|| Fail::bad("bad escape in the path"))?;
    let mut query: Vec<(String, String)> = Vec::new();
    for kv in q.split('&').filter(|s| !s.is_empty()) {
        let (k, v): (&str, &str) = kv.split_once('=').unwrap_or((kv, ""));
        let k: String = pct_decode(k, true).ok_or_else(|| Fail::bad("bad escape in the query"))?;
        let v: String = pct_decode(v, true).ok_or_else(|| Fail::bad("bad escape in the query"))?;
        query.push((k, v));
    }
    if query.len() > MAX_PARAMS {
        return Err(Fail::bad("too many query parameters"));
    }
    Ok((path, query))
}

struct Head {
    method: String,
    path: String,
    query: Vec<(String, String)>,
    heads: Vec<(String, String)>,
}

fn parse_head(t: &str) -> Hr<Head> {
    let mut lines: std::str::Split<&str> = t.split("\r\n");
    let first: &str = lines.next().unwrap_or("");
    let mut p: std::str::Split<char> = first.split(' ');
    let (m, tg, v): (Option<&str>, Option<&str>, Option<&str>) = (p.next(), p.next(), p.next());
    let (m, tg, v): (&str, &str, &str) = match (m, tg, v, p.next()) {
        (Some(m), Some(tg), Some(v), None) => (m, tg, v),
        _ => return Err(Fail::bad("malformed request line")),
    };
    if !token_ok(m) {
        return Err(Fail::bad("malformed method"));
    }
    if v != "HTTP/1.1" && v != "HTTP/1.0" {
        return Err(Fail::new(505, "only HTTP/1.0 and HTTP/1.1 are supported"));
    }
    if m != "GET" && m != "POST" {
        return Err(Fail::new(405, "only GET and POST are supported"));
    }
    let (path, query): (String, Vec<(String, String)>) = parse_target(tg)?;
    let mut heads: Vec<(String, String)> = Vec::new();
    for l in lines {
        if heads.len() >= MAX_HEADS {
            return Err(Fail::new(431, "too many headers"));
        }
        if l.starts_with(' ') || l.starts_with('\t') {
            return Err(Fail::bad("folded headers are not supported"));
        }
        let (k, v): (&str, &str) = l.split_once(':').ok_or_else(|| Fail::bad("malformed header"))?;
        if !token_ok(k) {
            return Err(Fail::bad("malformed header name"));
        }
        let v: &str = v.trim_matches(|c| c == ' ' || c == '\t');
        if v.chars().any(|c| c.is_control() && c != '\t') {
            return Err(Fail::bad("control character in a header"));
        }
        heads.push((k.to_ascii_lowercase(), v.to_string()));
    }
    for k in ONCE {
        if heads.iter().filter(|h| h.0 == k).count() > 1 {
            return Err(Fail::bad(&format!("duplicate {k} header")));
        }
    }
    Ok(Head { method: m.to_string(), path, query, heads })
}

fn content_len(v: &str) -> Hr<usize> {
    if v.is_empty() || v.len() > 12 || !v.bytes().all(|c| c.is_ascii_digit()) {
        return Err(Fail::bad("bad Content-Length"));
    }
    v.parse::<usize>().map_err(|_| Fail::bad("bad Content-Length"))
}

fn find_end(b: &[u8]) -> Option<usize> {
    b.windows(4).position(|w| w == b"\r\n\r\n")
}

fn slow() -> Fail {
    Fail::new(408, "the request was not received in time")
}

pub fn read_req<R: Read>(r: &mut R, lim: &Limits) -> Hr<Req> {
    let mut buf: Vec<u8> = Vec::with_capacity(2048);
    let mut chunk: [u8; 4096] = [0; 4096];
    let end: usize = loop {
        if let Some(i) = find_end(&buf) {
            break i;
        }
        if buf.len() > lim.head {
            return Err(Fail::new(431, "the request headers are too large"));
        }
        let n: usize = r.read(&mut chunk).map_err(|_| slow())?;
        if n == 0 {
            return Err(Fail::bad("the connection closed before the request was complete"));
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    if end > lim.head {
        return Err(Fail::new(431, "the request headers are too large"));
    }
    let text: &str = std::str::from_utf8(&buf[..end]).map_err(|_| Fail::bad("the headers are not valid text"))?;
    let h: Head = parse_head(text)?;
    if h.heads.iter().any(|x| x.0 == "transfer-encoding") {
        return Err(Fail::new(501, "chunked bodies are not supported"));
    }
    let cl: Option<&str> = h.heads.iter().find(|x| x.0 == "content-length").map(|x| x.1.as_str());
    let n: usize = match (h.method.as_str(), cl) {
        ("POST", None) => return Err(Fail::new(411, "POST needs a Content-Length")),
        (_, Some(v)) => content_len(v)?,
        (_, None) => 0,
    };
    if h.method == "GET" && n > 0 {
        return Err(Fail::bad("GET requests may not have a body"));
    }
    if n > lim.body {
        return Err(Fail::new(413, &format!("the request body is larger than the {} byte limit", lim.body)));
    }
    let mut body: Vec<u8> = buf.split_off(end + 4);
    if body.len() > n {
        return Err(Fail::bad("extra bytes after the request body"));
    }
    while body.len() < n {
        let want: usize = (n - body.len()).min(chunk.len());
        let k: usize = r.read(&mut chunk[..want]).map_err(|_| slow())?;
        if k == 0 {
            return Err(Fail::bad("the body ended early"));
        }
        body.extend_from_slice(&chunk[..k]);
    }
    Ok(Req { method: h.method, path: h.path, query: h.query, heads: h.heads, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rd(raw: &str) -> Hr<Req> {
        read_req(&mut raw.as_bytes(), &Limits::DEFAULT)
    }

    #[test]
    fn simple_get() {
        let r: Req = rd("GET /api/fs?path=C%3A%5CUsers%5Ca+b&x=1 HTTP/1.1\r\nHost: 127.0.0.1:1\r\nX-Ootrr-Token: abc\r\n\r\n").unwrap();
        assert_eq!(r.method, "GET");
        assert_eq!(r.path, "/api/fs");
        assert_eq!(r.param("path"), Some("C:\\Users\\a b"));
        assert_eq!(r.param("x"), Some("1"));
        assert_eq!(r.head("host"), Some("127.0.0.1:1"));
        assert_eq!(r.head("x-ootrr-token"), Some("abc"));
        assert!(r.body.is_empty());
    }

    #[test]
    fn post_with_body() {
        let r: Req = rd("POST /api/generate HTTP/1.1\r\nHost: h\r\nContent-Length: 5\r\n\r\nhello").unwrap();
        assert_eq!(r.body, b"hello");
    }

    #[test]
    fn body_arrives_in_small_pieces() {
        struct Drip<'a>(&'a [u8]);
        impl Read for Drip<'_> {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                let n: usize = self.0.len().min(out.len()).min(3);
                out[..n].copy_from_slice(&self.0[..n]);
                self.0 = &self.0[n..];
                Ok(n)
            }
        }
        let raw: &[u8] = b"POST /x HTTP/1.1\r\nHost: h\r\nContent-Length: 11\r\n\r\nhello world";
        let r: Req = read_req(&mut Drip(raw), &Limits::DEFAULT).unwrap();
        assert_eq!(r.body, b"hello world");
    }

    #[test]
    fn oversize_body_is_413_before_reading_it() {
        let lim: Limits = Limits { head: 1024, body: 10 };
        let e: Fail = read_req(&mut "POST /x HTTP/1.1\r\nHost: h\r\nContent-Length: 11\r\n\r\n".as_bytes(), &lim).unwrap_err();
        assert_eq!(e.status, 413);
        let e: Fail = rd("POST /x HTTP/1.1\r\nHost: h\r\nContent-Length: 99999999999\r\n\r\n").unwrap_err();
        assert_eq!(e.status, 413);
    }

    #[test]
    fn oversize_headers_are_431() {
        let big: String = format!("GET / HTTP/1.1\r\nHost: h\r\nX-A: {}\r\n\r\n", "a".repeat(20_000));
        assert_eq!(rd(&big).unwrap_err().status, 431);
        let no_end: String = format!("GET / HTTP/1.1\r\nX-A: {}", "a".repeat(40_000));
        assert_eq!(rd(&no_end).unwrap_err().status, 431);
        let many: String = format!("GET / HTTP/1.1\r\n{}\r\n", "X-A: 1\r\n".repeat(150));
        assert_eq!(rd(&many).unwrap_err().status, 431);
    }

    #[test]
    fn smuggling_shapes_are_refused() {
        assert_eq!(rd("POST /x HTTP/1.1\r\nHost: h\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap_err().status, 501);
        assert_eq!(rd("POST /x HTTP/1.1\r\nHost: h\r\nContent-Length: 1\r\nContent-Length: 1\r\n\r\na").unwrap_err().status, 400);
        assert_eq!(rd("POST /x HTTP/1.1\r\nHost: h\r\nContent-Length: 1\r\n\r\nabc").unwrap_err().status, 400);
        assert_eq!(rd("POST /x HTTP/1.1\r\nHost: h\r\nContent-Length: -1\r\n\r\n").unwrap_err().status, 400);
        assert_eq!(rd("POST /x HTTP/1.1\r\nHost: h\r\nContent-Length: 1\r\n\r\n").unwrap_err().status, 400);
        assert_eq!(rd("POST /x HTTP/1.1\r\nHost: h\r\n\r\n").unwrap_err().status, 411);
        assert_eq!(rd("GET /x HTTP/1.1\r\nHost: h\r\nContent-Length: 3\r\n\r\nabc").unwrap_err().status, 400);
        assert_eq!(rd("GET /x HTTP/1.1\r\nHost: a\r\nHost: b\r\n\r\n").unwrap_err().status, 400);
        assert_eq!(rd("GET /x HTTP/1.1\r\nHost: h\r\n X: folded\r\n\r\n").unwrap_err().status, 400);
        assert_eq!(rd("GET /x HTTP/1.1\r\nHost: h\r\nX: a\x01b\r\n\r\n").unwrap_err().status, 400);
        assert_eq!(rd("GET /x HTTP/1.1\r\nBad Name: x\r\n\r\n").unwrap_err().status, 400);
    }

    #[test]
    fn bad_request_lines() {
        for t in [
            "GET /x\r\n\r\n",
            "GET /x HTTP/1.1 extra\r\n\r\n",
            "GET  /x HTTP/1.1\r\n\r\n",
            "GET x HTTP/1.1\r\n\r\n",
            "GET //evil HTTP/1.1\r\n\r\n",
            "GET http://evil/ HTTP/1.1\r\n\r\n",
            "GET /a#b HTTP/1.1\r\n\r\n",
            "GET /a%zz HTTP/1.1\r\n\r\n",
            "GET /a%00 HTTP/1.1\r\n\r\n",
            "GET /a%ff HTTP/1.1\r\n\r\n",
            "G(T /x HTTP/1.1\r\n\r\n",
        ] {
            assert_eq!(rd(t).unwrap_err().status, 400, "{t:?}");
        }
        assert_eq!(rd("DELETE /x HTTP/1.1\r\n\r\n").unwrap_err().status, 405);
        assert_eq!(rd("OPTIONS /x HTTP/1.1\r\n\r\n").unwrap_err().status, 405);
        assert_eq!(rd("GET /x HTTP/2.0\r\n\r\n").unwrap_err().status, 505);
        let long: String = format!("GET /{} HTTP/1.1\r\n\r\n", "a".repeat(3000));
        assert_eq!(rd(&long).unwrap_err().status, 414);
    }

    #[test]
    fn truncated_input_is_an_error_not_a_hang() {
        assert!(rd("GET / HTTP/1.1\r\nHost: h\r\n").is_err());
        assert!(rd("").is_err());
    }

    #[test]
    fn percent_decoding() {
        assert_eq!(pct_decode("a%20b", false).as_deref(), Some("a b"));
        assert_eq!(pct_decode("a+b", false).as_deref(), Some("a+b"));
        assert_eq!(pct_decode("a+b", true).as_deref(), Some("a b"));
        assert_eq!(pct_decode("%e2%82%ac", false).as_deref(), Some("\u{20ac}"));
        assert_eq!(pct_decode("%", false), None);
        assert_eq!(pct_decode("%4", false), None);
        assert_eq!(pct_decode("%0a", false), None);
    }

    #[test]
    fn header_names_are_lowercased_and_values_trimmed() {
        let r: Req = rd("GET / HTTP/1.1\r\nHOST:\t127.0.0.1:9  \r\nX-Thing: a: b\r\n\r\n").unwrap();
        assert_eq!(r.head("host"), Some("127.0.0.1:9"));
        assert_eq!(r.head("x-thing"), Some("a: b"));
    }

    #[test]
    fn random_garbage_never_panics() {
        let base: &[u8] = b"POST /api/x?a=b HTTP/1.1\r\nHost: h\r\nContent-Length: 3\r\n\r\nabc";
        for i in 0..base.len() {
            for c in [0u8, b'\r', b'\n', b' ', b':', b'%', 0xff, b'9'] {
                let mut v: Vec<u8> = base.to_vec();
                v[i] = c;
                let _ = read_req(&mut &v[..], &Limits::DEFAULT);
            }
        }
    }
}
