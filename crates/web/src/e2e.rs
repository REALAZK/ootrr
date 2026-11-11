#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::path::PathBuf as PB;
    use std::thread;
    use std::time::Duration;

    use ootrr_core::json::{self, Json};

    use crate::server::{self, Handle, Opts};

    struct Out {
        status: u16,
        head: String,
        body: String,
    }

    impl Out {
        fn json(&self) -> Json {
            json::parse(&self.body).unwrap_or(Json::Null)
        }

        fn has_head(&self, k: &str) -> bool {
            self.head.to_ascii_lowercase().contains(&k.to_ascii_lowercase())
        }
    }

    fn up() -> Handle {
        server::start(&Opts { port: 0, lan: false, fallback: false, updates: false }).unwrap()
    }

    fn raw(port: u16, bytes: &[u8]) -> Out {
        let mut s: TcpStream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        s.write_all(bytes).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        let _ = s.read_to_end(&mut buf);
        let t: String = String::from_utf8_lossy(&buf).into_owned();
        let (h, b): (&str, &str) = t.split_once("\r\n\r\n").unwrap_or((&t, ""));
        let status: u16 = h.split(' ').nth(1).and_then(|x| x.parse().ok()).unwrap_or(0);
        Out { status, head: h.to_string(), body: b.to_string() }
    }

    fn req(h: &Handle, method: &str, path: &str, body: Option<&str>) -> Out {
        let mut t: String = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n", h.port);
        if path.starts_with("/api/") {
            t.push_str(&format!("X-OOTRR-Token: {}\r\n", h.token));
        }
        if let Some(b) = body {
            t.push_str(&format!("Origin: http://127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n", h.port, b.len()));
        }
        t.push_str("\r\n");
        t.push_str(body.unwrap_or(""));
        raw(h.port, t.as_bytes())
    }

    fn get(h: &Handle, path: &str) -> Out {
        req(h, "GET", path, None)
    }

    fn post(h: &Handle, path: &str, body: &str) -> Out {
        req(h, "POST", path, Some(body))
    }

    fn enc(s: &str) -> String {
        s.bytes()
            .map(|b| if b.is_ascii_alphanumeric() { (b as char).to_string() } else { format!("%{b:02X}") })
            .collect()
    }

    fn tmp(tag: &str) -> PB {
        let p: PB = env::temp_dir().join(format!("ootrr_web_e2e_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn jstr(p: &PB) -> String {
        Json::Str(p.display().to_string()).to_string()
    }

    #[test]
    fn pages_are_served_with_a_strict_csp_and_no_cors() {
        let mut h: Handle = up();
        for p in ["/", "/app.js", "/style.css", "/js/dom.js"] {
            let r: Out = get(&h, p);
            assert_eq!(r.status, 200, "{p}");
            assert!(r.has_head("content-security-policy: default-src 'none'; script-src 'self'; style-src 'self'"), "{p}");
            assert!(!r.has_head("access-control-"), "{p}");
            assert!(r.has_head("x-content-type-options: nosniff"));
            assert!(!r.body.is_empty());
        }
        assert!(get(&h, "/").has_head("text/html"));
        assert!(get(&h, "/app.js").has_head("text/javascript"));
        assert!(get(&h, "/style.css").has_head("text/css"));
        assert!(!get(&h, "/").body.contains(&h.token));
        h.stop();
    }

    #[test]
    fn unknown_paths_and_traversal_in_the_url_are_404() {
        let mut h: Handle = up();
        for p in ["/nope", "/js/../app.js", "/js/%2e%2e/app.js", "/..%2f..%2fCargo.toml", "/js/", "/Cargo.toml", "/src/lib.rs"] {
            let r: Out = get(&h, p);
            assert!(r.status == 404 || r.status == 400, "{p} gave {}", r.status);
        }
        h.stop();
    }

    #[test]
    fn info_schema_and_codec() {
        let mut h: Handle = up();
        let r: Out = get(&h, "/api/info");
        assert_eq!(r.status, 200);
        assert!(!r.body.contains(&h.token));
        assert_eq!(r.json().get("lan").and_then(|x| x.as_bool()), Some(false));
        let r: Out = get(&h, "/api/schema");
        assert_eq!(r.status, 200);
        let sch: Json = r.json();
        assert_eq!(sch.get("string_prefix").and_then(|x| x.as_str()), Some("OOTRR1-"));
        assert!(sch.get("presets").and_then(|x| x.as_arr()).map(|a| a.len()).unwrap_or(0) >= 5);
        let r: Out = post(&h, "/api/settings/encode", r#"{"settings":{"bridge":"tokens","bridge_tokens":50}}"#);
        assert_eq!(r.status, 200);
        let s: String = r.json().get("settings_string").and_then(|x| x.as_str()).unwrap().to_string();
        let r: Out = post(&h, "/api/settings/decode", &format!("{{\"settings_string\":\"{s}\"}}"));
        assert_eq!(r.status, 200);
        assert_eq!(r.json().get("settings").and_then(|x| x.get("bridge")).and_then(|x| x.as_str()), Some("tokens"));
        let r: Out = post(&h, "/api/settings/decode", r#"{"settings_string":"junk"}"#);
        assert_eq!(r.status, 422);
        assert!(r.json().get("error").and_then(|x| x.as_str()).is_some());
        h.stop();
    }

    #[test]
    fn generate_then_read_back_the_seed_file() {
        let mut h: Handle = up();
        let r: Out = post(&h, "/api/generate", r#"{"seed":"e2e seed","settings":{"shuffle_mapcompass":"vanilla"},"spoiler":true}"#);
        assert_eq!(r.status, 200);
        let g: Json = r.json();
        assert_eq!(g.get("placements_ready").and_then(|x| x.as_bool()), Some(true));
        assert!(g.get("placement_count").and_then(|x| x.as_i64()).unwrap() > 0);
        assert_eq!(g.get("spoiler").and_then(|x| x.get("available")).and_then(|x| x.as_bool()), Some(true));
        let text: &str = g.get("seed_text").and_then(|x| x.as_str()).unwrap();
        let back: Out = post(&h, "/api/seed/read", &format!("{{\"seed_text\":{}}}", Json::Str(text.to_string())));
        assert_eq!(back.status, 200);
        assert_eq!(back.json().get("checksum"), g.get("checksum"));
        assert_eq!(back.json().get("seed").and_then(|x| x.as_str()), Some("e2e seed"));
        let bad: Out = post(&h, "/api/seed/read", &format!("{{\"seed_text\":{}}}", Json::Str(text.replace("e2e seed", "e2e sead"))));
        assert_eq!(bad.status, 422);
        h.stop();
    }

    #[test]
    fn validate_and_fs_report_names_only() {
        let mut h: Handle = up();
        let d: PB = tmp("fs");
        let g: PB = d.join("game");
        fs::create_dir_all(g.join("Pack")).unwrap();
        fs::create_dir_all(g.join("Banc")).unwrap();
        fs::write(g.join("Pack").join("ZsDic.pack.zs"), b"CONTENT-MUST-NOT-LEAK").unwrap();
        crate::api_validate::fake_version(&g);
        let r: Out = get(&h, &format!("/api/validate?path={}", enc(&g.display().to_string())));
        assert_eq!(r.status, 200);
        assert_eq!(r.json().get("ok").and_then(|x| x.as_bool()), Some(true));
        assert!(!r.body.contains("CONTENT-MUST-NOT-LEAK"));
        let r: Out = get(&h, &format!("/api/validate?path={}", enc(&d.display().to_string())));
        assert_eq!(r.json().get("ok").and_then(|x| x.as_bool()), Some(false));
        let r: Out = get(&h, &format!("/api/fs?path={}", enc(&g.join("Pack").display().to_string())));
        assert_eq!(r.status, 200);
        assert!(r.body.contains("ZsDic.pack.zs"));
        assert!(!r.body.contains("CONTENT-MUST-NOT-LEAK"));
        let r: Out = get(&h, "/api/fs");
        assert_eq!(r.status, 200);
        assert!(!r.json().get("roots").and_then(|x| x.as_arr()).unwrap().is_empty());
        let _ = fs::remove_dir_all(&d);
        h.stop();
    }

    #[test]
    fn path_traversal_is_refused_on_every_path_parameter() {
        let mut h: Handle = up();
        let d: PB = tmp("trav");
        let sep: char = std::path::MAIN_SEPARATOR;
        let up_path: String = format!("{}{sep}..{sep}x", d.display());
        for p in [up_path.as_str(), "..", "relative", "%2e%2e", "/a/%2e%2e/b", "C:\\a\\..\\b"] {
            for ep in ["/api/fs", "/api/validate"] {
                let r: Out = get(&h, &format!("{ep}?path={}", if p.contains('%') { p.to_string() } else { enc(p) }));
                assert_eq!(r.status, 422, "{ep} {p}");
            }
        }
        let r: Out = post(&h, "/api/open-folder", &format!("{{\"path\":{}}}", Json::Str(up_path.clone())));
        assert_eq!(r.status, 422);
        let _ = fs::remove_dir_all(&d);
        h.stop();
    }

    #[test]
    fn build_validates_then_fails_cleanly_on_a_fake_game_and_writes_nothing() {
        let mut h: Handle = up();
        let d: PB = tmp("build");
        let g: PB = d.join("game");
        fs::create_dir_all(g.join("Pack")).unwrap();
        fs::create_dir_all(g.join("Banc")).unwrap();
        fs::write(g.join("Pack").join("ZsDic.pack.zs"), b"").unwrap();
        crate::api_validate::fake_version(&g);
        let m: PB = d.join("mods");
        fs::create_dir_all(&m).unwrap();
        let made: Json = post(&h, "/api/generate", r#"{"seed":"b","settings":{"shuffle_chests":"off"}}"#).json();
        let st: Json = made.get("seed_text").cloned().unwrap();
        let body: String = format!("{{\"seed_text\":{st},\"game_dir\":{},\"mod_dir\":{},\"mod_name\":\"Test Mod\"}}", jstr(&g), jstr(&m));
        let r: Out = post(&h, "/api/build", &body);
        assert_eq!(r.status, 202, "{}", r.body);
        let id: i64 = r.json().get("job_id").and_then(|x| x.as_i64()).unwrap();
        let mut last: Json = Json::Null;
        for _ in 0..200 {
            last = get(&h, &format!("/api/jobs/{id}")).json();
            if last.get("finished").and_then(|x| x.as_bool()) == Some(true) {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(last.get("state").and_then(|x| x.as_str()), Some("failed"));
        assert!(!last.get("message").and_then(|x| x.as_str()).unwrap().is_empty());
        assert_eq!(fs::read_dir(&m).unwrap().count(), 0);
        let bad: String = format!("{{\"seed_text\":{st},\"game_dir\":{},\"mod_dir\":{},\"mod_name\":\"..\"}}", jstr(&g), jstr(&m));
        assert_eq!(post(&h, "/api/build", &bad).status, 422);
        let bad: String = format!("{{\"seed_text\":{st},\"game_dir\":{},\"mod_dir\":{},\"mod_name\":\"x\"}}", jstr(&m), jstr(&m));
        assert_eq!(post(&h, "/api/build", &bad).status, 422);
        assert_eq!(get(&h, "/api/jobs/99999").status, 404);
        let _ = fs::remove_dir_all(&d);
        h.stop();
    }

    #[test]
    fn a_wrong_host_is_refused_even_with_the_right_token() {
        let mut h: Handle = up();
        for host in ["evil.example", "127.0.0.1.evil.example", "localhost", "192.168.0.9:80", ""] {
            let t: String = format!("GET /api/info HTTP/1.1\r\nHost: {host}\r\nX-OOTRR-Token: {}\r\n\r\n", h.token);
            assert_eq!(raw(h.port, t.as_bytes()).status, 403, "{host:?}");
            let t: String = format!("GET / HTTP/1.1\r\nHost: {host}\r\n\r\n");
            assert_eq!(raw(h.port, t.as_bytes()).status, 403, "page {host:?}");
        }
        let t: String = format!("GET /api/info HTTP/1.1\r\nX-OOTRR-Token: {}\r\n\r\n", h.token);
        assert_eq!(raw(h.port, t.as_bytes()).status, 403);
        let t: String = format!("GET /api/info HTTP/1.1\r\nHost: localhost:{}\r\nX-OOTRR-Token: {}\r\n\r\n", h.port, h.token);
        assert_eq!(raw(h.port, t.as_bytes()).status, 200);
        h.stop();
    }

    #[test]
    fn the_token_is_required_on_every_api_route() {
        let mut h: Handle = up();
        let port: u16 = h.port;
        for p in ["/api/info", "/api/schema", "/api/fs", "/api/validate?path=x", "/api/jobs/1"] {
            let t: String = format!("GET {p} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n");
            assert_eq!(raw(port, t.as_bytes()).status, 401, "{p}");
            let t: String = format!("GET {p} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-OOTRR-Token: nope\r\n\r\n");
            assert_eq!(raw(port, t.as_bytes()).status, 401, "{p} wrong");
            let t: String = format!("GET {p}{}token={} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n", if p.contains('?') { "&" } else { "?" }, h.token);
            assert_eq!(raw(port, t.as_bytes()).status, 401, "{p} token in the query must not count");
        }
        for p in ["/api/generate", "/api/settings/encode", "/api/settings/decode", "/api/seed/read", "/api/build", "/api/open-folder"] {
            let t: String = format!(
                "POST {p} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{{}}"
            );
            assert_eq!(raw(port, t.as_bytes()).status, 401, "{p}");
        }
        h.stop();
    }

    #[test]
    fn posts_need_a_matching_origin_and_a_json_content_type() {
        let mut h: Handle = up();
        let port: u16 = h.port;
        let tok: String = h.token.clone();
        let mk = |origin: Option<&str>, ct: &str| -> String {
            let mut t: String = format!("POST /api/settings/encode HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-OOTRR-Token: {tok}\r\n");
            if let Some(o) = origin {
                t.push_str(&format!("Origin: {o}\r\n"));
            }
            t.push_str(&format!("Content-Type: {ct}\r\nContent-Length: 15\r\n\r\n{{\"settings\":{{}}}}"));
            t
        };
        assert_eq!(raw(port, mk(None, "application/json").as_bytes()).status, 403);
        assert_eq!(raw(port, mk(Some("http://evil.example"), "application/json").as_bytes()).status, 403);
        assert_eq!(raw(port, mk(Some("null"), "application/json").as_bytes()).status, 403);
        assert_eq!(raw(port, mk(Some(&format!("http://127.0.0.1:{}", port + 1)), "application/json").as_bytes()).status, 403);
        assert_eq!(raw(port, mk(Some(&format!("http://127.0.0.1:{port}")), "text/plain").as_bytes()).status, 415);
        assert_eq!(raw(port, mk(Some(&format!("http://127.0.0.1:{port}")), "application/json").as_bytes()).status, 200);
        let sf: String = format!("GET /api/info HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-OOTRR-Token: {tok}\r\nSec-Fetch-Site: cross-site\r\n\r\n");
        assert_eq!(raw(port, sf.as_bytes()).status, 403);
        h.stop();
    }

    #[test]
    fn oversize_requests_are_refused_without_reading_them() {
        let mut h: Handle = up();
        let port: u16 = h.port;
        let tok: String = h.token.clone();
        let big: String = format!(
            "POST /api/generate HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-OOTRR-Token: {tok}\r\nOrigin: http://127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: 9000000\r\n\r\n"
        );
        assert_eq!(raw(port, big.as_bytes()).status, 413);
        let long: String = format!("GET /api/info HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-Pad: {}\r\n\r\n", "a".repeat(40_000));
        assert_eq!(raw(port, long.as_bytes()).status, 431);
        let uri: String = format!("GET /{} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n", "a".repeat(4000));
        assert_eq!(raw(port, uri.as_bytes()).status, 414);
        let chunked: String = format!("POST /api/generate HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n");
        assert_eq!(raw(port, chunked.as_bytes()).status, 501);
        assert_eq!(get(&h, "/api/info").status, 200);
        h.stop();
    }

    #[test]
    fn options_and_other_methods_get_no_cors_answer() {
        let mut h: Handle = up();
        let port: u16 = h.port;
        for m in ["OPTIONS", "PUT", "DELETE", "HEAD", "PATCH", "TRACE"] {
            let t: String = format!("{m} /api/info HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://evil.example\r\nAccess-Control-Request-Method: GET\r\n\r\n");
            let r: Out = raw(port, t.as_bytes());
            assert_eq!(r.status, 405, "{m}");
            assert!(!r.has_head("access-control-allow"), "{m}");
        }
        h.stop();
    }

    #[test]
    fn half_open_and_garbage_connections_do_not_hurt_the_server() {
        let mut h: Handle = up();
        let port: u16 = h.port;
        {
            let mut s: TcpStream = TcpStream::connect(("127.0.0.1", port)).unwrap();
            s.write_all(b"GET /api/info HTT").unwrap();
        }
        {
            let mut s: TcpStream = TcpStream::connect(("127.0.0.1", port)).unwrap();
            let _ = s.write_all(&[0xff, 0x00, 0x13, 0x37, b'\r', b'\n', b'\r', b'\n']);
        }
        let _ = TcpStream::connect(("127.0.0.1", port)).unwrap();
        assert_eq!(raw(port, b"nonsense\r\n\r\n").status, 400);
        assert_eq!(get(&h, "/api/info").status, 200);
        h.stop();
    }

    #[test]
    fn many_parallel_requests_all_succeed() {
        let mut h: Handle = up();
        let port: u16 = h.port;
        let tok: String = h.token.clone();
        let ts: Vec<thread::JoinHandle<u16>> = (0..24)
            .map(|_| {
                let tok2: String = tok.clone();
                thread::spawn(move || {
                    let t: String = format!("GET /api/info HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-OOTRR-Token: {tok2}\r\n\r\n");
                    raw(port, t.as_bytes()).status
                })
            })
            .collect();
        for t in ts {
            assert_eq!(t.join().unwrap(), 200);
        }
        h.stop();
    }

    #[test]
    fn binds_loopback_only_by_default_and_the_fallback_port_works() {
        let mut a: Handle = up();
        let b: Result<Handle, _> = server::start(&Opts { port: a.port, lan: false, fallback: false, updates: false });
        assert!(b.is_err());
        let mut c: Handle = server::start(&Opts { port: a.port, lan: false, fallback: true, updates: false }).unwrap();
        assert_ne!(c.port, a.port);
        assert_eq!(get(&c, "/api/info").status, 200);
        assert!(c.url().starts_with("http://127.0.0.1:"));
        assert!(c.url().contains("#token="));
        c.stop();
        a.stop();
    }

    #[test]
    fn lan_mode_still_needs_the_token_and_an_ip_host() {
        let mut h: Handle = server::start(&Opts { port: 0, lan: true, fallback: false, updates: false }).unwrap();
        let port: u16 = h.port;
        let t: String = format!("GET /api/info HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n");
        assert_eq!(raw(port, t.as_bytes()).status, 401);
        let t: String = format!("GET /api/info HTTP/1.1\r\nHost: 10.1.2.3:{port}\r\nX-OOTRR-Token: {}\r\n\r\n", h.token);
        assert_eq!(raw(port, t.as_bytes()).status, 200);
        let t: String = format!("GET /api/info HTTP/1.1\r\nHost: evil.example:{port}\r\nX-OOTRR-Token: {}\r\n\r\n", h.token);
        assert_eq!(raw(port, t.as_bytes()).status, 403);
        h.stop();
    }
}
