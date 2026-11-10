use ootrr_core::json::Json;

use crate::error::Fail;

pub const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'; font-src 'none'; object-src 'none'; media-src 'none'; frame-src 'none'; worker-src 'none'; manifest-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

#[derive(Debug, Clone, PartialEq)]
pub struct Resp {
    pub status: u16,
    pub ctype: &'static str,
    pub body: Vec<u8>,
}

impl Resp {
    pub fn new(status: u16, ctype: &'static str, body: Vec<u8>) -> Resp {
        Resp { status, ctype, body }
    }

    pub fn json(status: u16, j: &Json) -> Resp {
        Resp::new(status, "application/json; charset=utf-8", j.to_string().into_bytes())
    }

    pub fn ok(j: &Json) -> Resp {
        Resp::json(200, j)
    }

    pub fn err(f: &Fail) -> Resp {
        Resp::json(f.status, &Json::obj().with("error", f.msg.as_str().into()))
    }

    pub fn empty(status: u16) -> Resp {
        Resp::new(status, "text/plain; charset=utf-8", Vec::new())
    }
}

pub fn reason(s: u16) -> &'static str {
    match s {
        200 => "OK",
        202 => "Accepted",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        411 => "Length Required",
        413 => "Content Too Large",
        414 => "URI Too Long",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Content",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        505 => "HTTP Version Not Supported",
        _ => "Status",
    }
}

pub fn to_bytes(r: &Resp) -> Vec<u8> {
    let mut h: String = String::new();
    h.push_str(&format!("HTTP/1.1 {} {}\r\n", r.status, reason(r.status)));
    h.push_str(&format!("Content-Type: {}\r\n", r.ctype));
    h.push_str(&format!("Content-Length: {}\r\n", r.body.len()));
    h.push_str("Connection: close\r\n");
    h.push_str("Cache-Control: no-store\r\n");
    h.push_str("X-Content-Type-Options: nosniff\r\n");
    h.push_str("Referrer-Policy: no-referrer\r\n");
    h.push_str("X-Frame-Options: DENY\r\n");
    h.push_str("Cross-Origin-Resource-Policy: same-origin\r\n");
    h.push_str("Cross-Origin-Opener-Policy: same-origin\r\n");
    h.push_str("Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=(), usb=()\r\n");
    h.push_str(&format!("Content-Security-Policy: {CSP}\r\n"));
    h.push_str("\r\n");
    let mut out: Vec<u8> = h.into_bytes();
    out.extend_from_slice(&r.body);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(r: &Resp) -> String {
        String::from_utf8_lossy(&to_bytes(r)).into_owned()
    }

    #[test]
    fn every_response_carries_the_security_headers_and_no_cors() {
        for r in [Resp::empty(204), Resp::ok(&Json::obj()), Resp::err(&Fail::new(403, "no"))] {
            let t: String = text(&r);
            assert!(t.contains("Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'"));
            assert!(t.contains("X-Content-Type-Options: nosniff"));
            assert!(t.contains("X-Frame-Options: DENY"));
            assert!(t.contains("Cache-Control: no-store"));
            assert!(t.contains("Referrer-Policy: no-referrer"));
            assert!(!t.to_ascii_lowercase().contains("access-control-"));
        }
    }

    #[test]
    fn csp_loads_nothing_external() {
        assert!(!CSP.contains("http"));
        assert!(!CSP.contains('*'));
        assert!(!CSP.contains("unsafe"));
        assert!(!CSP.contains("data:"));
        assert!(CSP.contains("script-src 'self'"));
        assert!(CSP.contains("style-src 'self'"));
        assert!(CSP.contains("default-src 'none'"));
    }

    #[test]
    fn length_and_body_match() {
        let r: Resp = Resp::ok(&Json::obj().with("a", 1.into()));
        let b: Vec<u8> = to_bytes(&r);
        let t: String = String::from_utf8_lossy(&b).into_owned();
        assert!(t.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(t.contains("Content-Length: 7\r\n"));
        assert!(t.ends_with("\r\n\r\n{\"a\":1}"));
    }

    #[test]
    fn errors_are_json() {
        let r: Resp = Resp::err(&Fail::new(401, "token \"x\""));
        assert_eq!(r.status, 401);
        assert_eq!(String::from_utf8_lossy(&r.body), "{\"error\":\"token \\\"x\\\"\"}");
    }
}
