use std::env;
use std::io::{self, BufRead, BufReader, Read, StdinLock, StdoutLock, Write};

use ootrr_core::error::Res;

use crate::guard::Guard;
use crate::req::{Limits, read_req};
use crate::resp::{Resp, to_bytes};
use crate::router::handle;
use crate::state::Shared;
use crate::token;

pub const PORT: u16 = 1;
pub const HELLO: &str = "OOTRR stdio";
pub const TOKEN_ENV: &str = "OOTRR_STDIO_TOKEN";

pub fn serve_on<R: Read, W: Write>(inp: R, out: &mut W, tok: &str) -> Res<()> {
    let sh: Shared = Shared::new(Guard { port: PORT, token: tok.to_string(), lan: false }, false);
    let mut r: BufReader<R> = BufReader::new(inp);
    out.write_all(format!("{HELLO} {tok}\r\n").as_bytes())?;
    out.flush()?;
    loop {
        if r.fill_buf()?.is_empty() {
            return Ok(());
        }
        let resp: Resp = match read_req(&mut r, &Limits::DEFAULT) {
            Ok(q) => handle(&sh, &q),
            Err(f) => Resp::err(&f),
        };
        out.write_all(&to_bytes(&resp))?;
        out.flush()?;
    }
}

pub fn token_of(given: Option<String>) -> String {
    given.filter(|t| t.len() >= 16 && t.chars().all(|c| c.is_ascii_alphanumeric())).unwrap_or_else(token::token)
}

pub fn serve() -> Res<()> {
    let inp: StdinLock<'static> = io::stdin().lock();
    let mut out: StdoutLock<'static> = io::stdout().lock();
    serve_on(inp, &mut out, &token_of(env::var(TOKEN_ENV).ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post(path: &str, body: &str) -> String {
        format!(
            "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:1\r\nOrigin: http://127.0.0.1:1\r\nX-OOTRR-Token: t\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
    }

    #[test]
    fn says_hello_then_answers_a_request_and_stops_at_the_end_of_input() {
        let get: &str = "GET /api/info HTTP/1.1\r\nHost: 127.0.0.1:1\r\nX-OOTRR-Token: t\r\n\r\n";
        let bad: &str = "GET /api/info HTTP/1.1\r\nHost: 127.0.0.1:1\r\nX-OOTRR-Token: no\r\n\r\n";
        let cases: [(String, &str); 3] = [
            (get.to_string(), "HTTP/1.1 200 "),
            (post("/api/settings/encode", "{\"settings\":{}}"), "HTTP/1.1 200 "),
            (bad.to_string(), "HTTP/1.1 401 "),
        ];
        for (input, want) in cases {
            let mut out: Vec<u8> = Vec::new();
            serve_on(input.as_bytes(), &mut out, "t").unwrap();
            let text: String = String::from_utf8_lossy(&out).to_string();
            assert!(text.starts_with(&format!("{HELLO} t\r\n{want}")), "{text}");
        }
    }

    #[test]
    fn a_given_token_is_used_only_when_it_is_long_and_plain() {
        assert_eq!(token_of(Some("abcdefghijklmnop1234".into())), "abcdefghijklmnop1234");
        for bad in [None, Some(String::new()), Some("short".into()), Some("abcdefghijklmnop 1234".into())] {
            let t: String = token_of(bad.clone());
            assert_eq!(t.len(), 32, "{bad:?}");
            assert_ne!(Some(t), bad);
        }
    }

    #[test]
    fn empty_input_only_says_hello() {
        let mut out: Vec<u8> = Vec::new();
        serve_on(&b""[..], &mut out, "t").unwrap();
        assert_eq!(out, format!("{HELLO} t\r\n").into_bytes());
    }
}
