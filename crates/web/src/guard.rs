use std::net::IpAddr;

use crate::error::{Fail, Hr};
use crate::req::Req;

pub const TOKEN_HEADER: &str = "x-ootrr-token";

#[derive(Debug, Clone)]
pub struct Guard {
    pub port: u16,
    pub token: String,
    pub lan: bool,
}

fn same(a: &str, b: &str) -> bool {
    let x: &[u8] = a.as_bytes();
    let y: &[u8] = b.as_bytes();
    if x.len() != y.len() {
        return false;
    }
    let mut d: u8 = 0;
    for i in 0..x.len() {
        d |= x[i] ^ y[i];
    }
    d == 0
}

fn is_ip_host(h: &str) -> bool {
    let t: &str = h.strip_prefix('[').and_then(|x| x.strip_suffix(']')).unwrap_or(h);
    t.parse::<IpAddr>().is_ok()
}

impl Guard {
    pub fn host_ok(&self, h: &str) -> bool {
        let h: String = h.to_ascii_lowercase();
        let p: u16 = self.port;
        if h == format!("127.0.0.1:{p}") || h == format!("localhost:{p}") {
            return true;
        }
        if !self.lan {
            return false;
        }
        match h.rsplit_once(':') {
            Some((a, b)) => b == p.to_string() && is_ip_host(a),
            None => false,
        }
    }

    pub fn origin_ok(&self, o: &str, host: &str) -> bool {
        self.host_ok(host) && same(&o.to_ascii_lowercase(), &format!("http://{}", host.to_ascii_lowercase()))
    }

    pub fn token_ok(&self, t: &str) -> bool {
        !self.token.is_empty() && same(t, &self.token)
    }

    pub fn check(&self, r: &Req, api: bool) -> Hr<()> {
        let host: &str = r.head("host").ok_or_else(|| Fail::deny("the Host header is missing"))?;
        if !self.host_ok(host) {
            return Err(Fail::deny("the Host header is not allowed"));
        }
        if !api {
            return Ok(());
        }
        if let Some(s) = r.head("sec-fetch-site")
            && s != "same-origin"
        {
            return Err(Fail::deny("cross-site requests are refused"));
        }
        match r.head("origin") {
            Some(o) if !self.origin_ok(o, host) => return Err(Fail::deny("the Origin header is not allowed")),
            None if r.method == "POST" => return Err(Fail::deny("POST needs an Origin header")),
            _ => {}
        }
        match r.head(TOKEN_HEADER) {
            Some(t) if self.token_ok(t) => {}
            Some(_) => return Err(Fail::new(401, "the session token is wrong")),
            None => return Err(Fail::new(401, "the session token is missing")),
        }
        if r.method == "POST" {
            let ct: &str = r.head("content-type").unwrap_or("");
            let mt: String = ct.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
            if mt != "application/json" {
                return Err(Fail::new(415, "POST bodies must be application/json"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(lan: bool) -> Guard {
        Guard { port: 8123, token: "s3cret".into(), lan }
    }

    fn req(method: &str, heads: &[(&str, &str)]) -> Req {
        Req {
            method: method.into(),
            path: "/api/x".into(),
            query: Vec::new(),
            heads: heads.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            body: Vec::new(),
        }
    }

    const GOOD: [(&str, &str); 3] = [("host", "127.0.0.1:8123"), ("x-ootrr-token", "s3cret"), ("sec-fetch-site", "same-origin")];

    #[test]
    fn host_rules() {
        let a: Guard = g(false);
        assert!(a.host_ok("127.0.0.1:8123"));
        assert!(a.host_ok("localhost:8123"));
        assert!(a.host_ok("LocalHost:8123"));
        for bad in [
            "evil.example:8123",
            "127.0.0.1",
            "127.0.0.1:8124",
            "127.0.0.1:8123.evil.example",
            "127.0.0.1.evil.example:8123",
            "localhost.evil.example:8123",
            "192.168.1.5:8123",
            "[::1]:8123",
            "",
            "127.0.0.1:8123:1",
        ] {
            assert!(!a.host_ok(bad), "{bad}");
        }
    }

    #[test]
    fn lan_mode_accepts_ip_literals_only() {
        let a: Guard = g(true);
        assert!(a.host_ok("192.168.1.5:8123"));
        assert!(a.host_ok("[fe80::1]:8123"));
        assert!(!a.host_ok("192.168.1.5:9"));
        assert!(!a.host_ok("printer.local:8123"));
        assert!(!a.host_ok("evil.example:8123"));
    }

    #[test]
    fn bad_host_is_refused_even_for_static_pages() {
        let r: Req = req("GET", &[("host", "evil.example:8123")]);
        assert_eq!(g(false).check(&r, false).unwrap_err().status, 403);
        let r: Req = req("GET", &[]);
        assert_eq!(g(false).check(&r, false).unwrap_err().status, 403);
        let r: Req = req("GET", &[("host", "127.0.0.1:8123")]);
        assert!(g(false).check(&r, false).is_ok());
    }

    #[test]
    fn rebinding_with_a_valid_token_still_fails_on_host() {
        let r: Req = req("GET", &[("host", "attacker.example:8123"), ("x-ootrr-token", "s3cret")]);
        assert_eq!(g(false).check(&r, true).unwrap_err().status, 403);
    }

    #[test]
    fn token_is_required_and_exact() {
        let a: Guard = g(false);
        assert!(a.check(&req("GET", &GOOD), true).is_ok());
        let none: Req = req("GET", &[("host", "127.0.0.1:8123")]);
        assert_eq!(a.check(&none, true).unwrap_err().status, 401);
        let wrong: Req = req("GET", &[("host", "127.0.0.1:8123"), ("x-ootrr-token", "s3cres")]);
        assert_eq!(a.check(&wrong, true).unwrap_err().status, 401);
        let longer: Req = req("GET", &[("host", "127.0.0.1:8123"), ("x-ootrr-token", "s3cret2")]);
        assert_eq!(a.check(&longer, true).unwrap_err().status, 401);
        let empty: Req = req("GET", &[("host", "127.0.0.1:8123"), ("x-ootrr-token", "")]);
        assert_eq!(a.check(&empty, true).unwrap_err().status, 401);
        let blank: Guard = Guard { port: 8123, token: String::new(), lan: false };
        assert_eq!(blank.check(&empty, true).unwrap_err().status, 401);
    }

    #[test]
    fn post_needs_a_matching_origin_and_json() {
        let a: Guard = g(false);
        let mut h: Vec<(&str, &str)> = GOOD.to_vec();
        h.push(("content-type", "application/json"));
        assert_eq!(a.check(&req("POST", &h), true).unwrap_err().status, 403);
        let mut ok: Vec<(&str, &str)> = h.clone();
        ok.push(("origin", "http://127.0.0.1:8123"));
        assert!(a.check(&req("POST", &ok), true).is_ok());
        let mut cs: Vec<(&str, &str)> = GOOD.to_vec();
        cs.push(("origin", "http://127.0.0.1:8123"));
        cs.push(("content-type", "Application/JSON; charset=utf-8"));
        assert!(a.check(&req("POST", &cs), true).is_ok());
        for o in ["http://evil.example", "http://127.0.0.1:9999", "https://127.0.0.1:8123", "null", "http://localhost:8123"] {
            let mut bad: Vec<(&str, &str)> = h.clone();
            bad.push(("origin", o));
            assert_eq!(a.check(&req("POST", &bad), true).unwrap_err().status, 403, "{o}");
        }
    }

    #[test]
    fn localhost_origin_must_match_the_host_used() {
        let a: Guard = g(false);
        let h: Vec<(&str, &str)> = vec![
            ("host", "localhost:8123"),
            ("x-ootrr-token", "s3cret"),
            ("content-type", "application/json"),
            ("origin", "http://localhost:8123"),
        ];
        assert!(a.check(&req("POST", &h), true).is_ok());
    }

    #[test]
    fn simple_form_posts_are_refused() {
        let a: Guard = g(false);
        for ct in ["text/plain", "application/x-www-form-urlencoded", "multipart/form-data; boundary=x", ""] {
            let mut h: Vec<(&str, &str)> = GOOD.to_vec();
            h.push(("origin", "http://127.0.0.1:8123"));
            h.push(("content-type", ct));
            assert_eq!(a.check(&req("POST", &h), true).unwrap_err().status, 415, "{ct:?}");
        }
    }

    #[test]
    fn cross_site_fetch_metadata_is_refused() {
        let a: Guard = g(false);
        for s in ["cross-site", "same-site", "none"] {
            let h: Vec<(&str, &str)> = vec![("host", "127.0.0.1:8123"), ("x-ootrr-token", "s3cret"), ("sec-fetch-site", s)];
            assert_eq!(a.check(&req("GET", &h), true).unwrap_err().status, 403, "{s}");
        }
    }

    #[test]
    fn a_foreign_origin_on_get_is_refused() {
        let h: Vec<(&str, &str)> = vec![("host", "127.0.0.1:8123"), ("x-ootrr-token", "s3cret"), ("origin", "http://evil.example")];
        assert_eq!(g(false).check(&req("GET", &h), true).unwrap_err().status, 403);
    }

    #[test]
    fn constant_time_compare_basics() {
        assert!(same("abc", "abc"));
        assert!(!same("abc", "abd"));
        assert!(!same("abc", "ab"));
        assert!(same("", ""));
    }
}
