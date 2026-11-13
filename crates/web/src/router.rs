use ootrr_core::json::Json;

use crate::error::{Fail, Hr};
use crate::req::Req;
use crate::resp::Resp;
use crate::state::Shared;
use crate::{update, api_build, api_fs, api_generate, api_info, api_items, api_open, api_seed, api_settings, api_tracker, api_validate, assets, body};

fn need(r: &Req, m: &str) -> Hr<()> {
    if r.method == m {
        Ok(())
    } else {
        Err(Fail::new(405, &format!("this endpoint takes {m}")))
    }
}

fn job(sh: &Shared, r: &Req, id: &str) -> Hr<Resp> {
    need(r, "GET")?;
    let n: u64 = id.parse::<u64>().map_err(|_| Fail::new(404, "no such job"))?;
    let j: Json = sh.jobs.get(n).ok_or_else(|| Fail::new(404, "no such job"))?;
    Ok(Resp::ok(&j))
}

fn api(sh: &Shared, r: &Req) -> Hr<Resp> {
    if let Some(id) = r.path.strip_prefix("/api/jobs/") {
        return job(sh, r, id);
    }
    if r.path == "/api/tracker/icon" {
        need(r, "GET")?;
        return api_items::icon(sh, r);
    }
    let j: Json = match r.path.as_str() {
        "/api/info" => {
            need(r, "GET")?;
            api_info::info(sh)
        }
        "/api/update" => {
            need(r, "GET")?;
            update::check(sh)
        }
        "/api/schema" => {
            need(r, "GET")?;
            api_info::schema_reply()?
        }
        "/api/validate" => {
            need(r, "GET")?;
            api_validate::run(r.param("path"))?
        }
        "/api/fs" => {
            need(r, "GET")?;
            api_fs::run(r.param("path"))?
        }
        "/api/settings/encode" | "/api/settings/decode" => {
            need(r, "POST")?;
            api_settings::convert(&body::json(r)?)?
        }
        "/api/generate" => {
            need(r, "POST")?;
            api_generate::run_in(&sh.gens, &body::json(r)?)?
        }
        "/api/generate/progress" => {
            need(r, "GET")?;
            api_generate::progress(&sh.gens, r.param("ticket"))?
        }
        "/api/generate/cancel" => {
            need(r, "POST")?;
            api_generate::cancel(&sh.gens, &body::json(r)?)?
        }
        "/api/plando/options" => {
            need(r, "POST")?;
            api_generate::options(&body::json(r)?)?
        }
        "/api/seed/read" => {
            need(r, "POST")?;
            api_seed::read(&body::json(r)?)?
        }
        "/api/open-folder" => {
            need(r, "POST")?;
            api_open::run(&body::json(r)?)?
        }
        "/api/tracker/open" => {
            need(r, "POST")?;
            api_tracker::open(sh, &body::json(r)?)?
        }
        "/api/tracker/index" => {
            need(r, "GET")?;
            api_tracker::index(sh)
        }
        "/api/tracker/state" => {
            need(r, "GET")?;
            api_tracker::state(sh)
        }
        "/api/tracker/events" => {
            need(r, "GET")?;
            api_tracker::events(sh, r)?
        }
        "/api/tracker/mode" => {
            need(r, "POST")?;
            api_tracker::mode(sh, &body::json(r)?)?
        }
        "/api/tracker/items" => {
            need(r, "GET")?;
            api_items::items(sh)
        }
        "/api/tracker/mark" => {
            need(r, "POST")?;
            api_tracker::mark(sh, &body::json(r)?)?
        }
        "/api/build" => {
            need(r, "POST")?;
            let j: Json = api_build::run(&sh.jobs, &body::json(r)?)?;
            return Ok(Resp::json(202, &j));
        }
        _ => return Err(Fail::new(404, "no such endpoint")),
    };
    Ok(Resp::ok(&j))
}

fn page(r: &Req) -> Hr<Resp> {
    if r.path == "/favicon.ico" {
        need(r, "GET")?;
        return Ok(Resp::empty(204));
    }
    match assets::get(&r.path) {
        Some((t, b)) => {
            need(r, "GET")?;
            Ok(Resp::new(200, t, b.as_bytes().to_vec()))
        }
        None => match assets::get_bin(&r.path) {
            Some((t, b)) => {
                need(r, "GET")?;
                Ok(Resp::new(200, t, b.to_vec()))
            }
            None => Err(Fail::new(404, "not found")),
        },
    }
}

pub fn handle(sh: &Shared, r: &Req) -> Resp {
    let is_api: bool = r.path.starts_with("/api/");
    if let Err(f) = sh.guard.check(r, is_api) {
        return Resp::err(&f);
    }
    let out: Hr<Resp> = if is_api { api(sh, r) } else { page(r) };
    out.unwrap_or_else(|f| Resp::err(&f))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard::Guard;

    fn sh() -> Shared {
        Shared::new(Guard { port: 5000, token: "tok".into(), lan: false }, false)
    }

    fn rq(method: &str, path: &str, extra: &[(&str, &str)], body: &str) -> Req {
        let mut heads: Vec<(String, String)> = vec![("host".into(), "127.0.0.1:5000".into())];
        if path.starts_with("/api/") {
            heads.push(("x-ootrr-token".into(), "tok".into()));
        }
        if method == "POST" {
            heads.push(("origin".into(), "http://127.0.0.1:5000".into()));
            heads.push(("content-type".into(), "application/json".into()));
        }
        for (k, v) in extra {
            heads.push((k.to_string(), v.to_string()));
        }
        Req { method: method.into(), path: path.into(), query: Vec::new(), heads, body: body.as_bytes().to_vec() }
    }

    #[test]
    fn pages_need_a_valid_host_but_no_token() {
        let s: Shared = sh();
        assert_eq!(handle(&s, &rq("GET", "/", &[], "")).status, 200);
        assert_eq!(handle(&s, &rq("GET", "/favicon.ico", &[], "")).status, 204);
        assert_eq!(handle(&s, &rq("GET", "/nope", &[], "")).status, 404);
        assert_eq!(handle(&s, &rq("POST", "/", &[], "{}")).status, 405);
        let mut bad: Req = rq("GET", "/", &[], "");
        bad.heads[0].1 = "evil.example".into();
        assert_eq!(handle(&s, &bad).status, 403);
    }

    #[test]
    fn api_routes_enforce_methods_and_tokens() {
        let s: Shared = sh();
        assert_eq!(handle(&s, &rq("GET", "/api/info", &[], "")).status, 200);
        assert_eq!(handle(&s, &rq("POST", "/api/info", &[], "{}")).status, 405);
        assert_eq!(handle(&s, &rq("GET", "/api/generate", &[], "")).status, 405);
        assert_eq!(handle(&s, &rq("GET", "/api/nothing", &[], "")).status, 404);
        let mut no_tok: Req = rq("GET", "/api/info", &[], "");
        no_tok.heads.retain(|h| h.0 != "x-ootrr-token");
        assert_eq!(handle(&s, &no_tok).status, 401);
        assert_eq!(handle(&s, &rq("GET", "/api/jobs/zzz", &[], "")).status, 404);
        assert_eq!(handle(&s, &rq("GET", "/api/jobs/77", &[], "")).status, 404);
    }

    #[test]
    fn schema_generate_and_codec_work_through_the_router() {
        let s: Shared = sh();
        let r: Resp = handle(&s, &rq("GET", "/api/schema", &[], ""));
        assert_eq!(r.status, 200);
        assert!(String::from_utf8_lossy(&r.body).contains("\"shuffle_chests\""));
        let r: Resp = handle(&s, &rq("POST", "/api/generate", &[], r#"{"seed":"a","settings":{}}"#));
        assert_eq!(r.status, 200);
        assert!(String::from_utf8_lossy(&r.body).contains("\"placements_ready\":true"));
        let r: Resp = handle(&s, &rq("POST", "/api/settings/encode", &[], r#"{"settings":{"bridge":"zzz"}}"#));
        assert_eq!(r.status, 422);
        let r: Resp = handle(&s, &rq("POST", "/api/settings/encode", &[], "[1]"));
        assert_eq!(r.status, 400);
    }

    #[test]
    fn generate_progress_cancel_and_a_readable_failure() {
        let s: Shared = sh();
        let mut p: Req = rq("GET", "/api/generate/progress", &[], "");
        p.query = vec![("ticket".into(), "nobody".into())];
        let r: Resp = handle(&s, &p);
        assert_eq!(r.status, 200);
        assert!(String::from_utf8_lossy(&r.body).contains("\"running\":false"));
        p.query.clear();
        assert_eq!(handle(&s, &p).status, 400);
        let c: Resp = handle(&s, &rq("POST", "/api/generate/cancel", &[], r#"{"ticket":"nobody"}"#));
        assert!(String::from_utf8_lossy(&c.body).contains("\"cancelled\":false"));
        let bad: &str = r#"{"seed":"x","ticket":"t1","settings":{"shuffle_chests":"dungeons","shuffle_bosskeys":"overworld"}}"#;
        let e: Resp = handle(&s, &rq("POST", "/api/generate", &[], bad));
        assert_eq!(e.status, 422);
        let t: String = String::from_utf8_lossy(&e.body).into_owned();
        assert!(t.contains("No seed can be made with these settings") && t.contains("Boss Key"), "{t}");
        assert!(s.gens.progress("t1").get("running").and_then(|x| x.as_bool()) == Some(false));
        assert_eq!(handle(&s, &rq("POST", "/api/generate", &[], r#"{"seed":"x","ticket":"a b","settings":{}}"#)).status, 400);
    }

    fn tsh() -> Shared {
        let mut t: crate::tracker::Tracker = crate::tracker::Tracker::new(None);
        t.poll = std::time::Duration::from_millis(150);
        Shared::with_tracker(Guard { port: 5000, token: "tok".into(), lan: false }, false, std::sync::Arc::new(t))
    }

    fn body_of(r: &Resp) -> String {
        String::from_utf8_lossy(&r.body).into_owned()
    }

    #[test]
    fn tracker_routes_need_the_token_and_the_page_does_not() {
        let s: Shared = tsh();
        let p: Resp = handle(&s, &rq("GET", "/tracker", &[], ""));
        assert_eq!(p.status, 200);
        assert!(body_of(&p).contains("/js/tracker_app.js"));
        for (m, path, b) in [
            ("GET", "/api/tracker/state", ""),
            ("GET", "/api/tracker/index", ""),
            ("GET", "/api/tracker/items", ""),
            ("GET", "/api/tracker/icon", ""),
            ("GET", "/api/tracker/events?after=0", ""),
            ("POST", "/api/tracker/mode", "{\"mode\":\"all\"}"),
            ("POST", "/api/tracker/mark", "{\"id\":\"x\",\"on\":true}"),
            ("POST", "/api/tracker/open", "{}"),
        ] {
            let mut q: Req = rq(m, path.split('?').next().unwrap(), &[], b);
            q.heads.retain(|h| h.0 != "x-ootrr-token");
            assert_eq!(handle(&s, &q).status, 401, "{path}");
        }
        assert_eq!(handle(&s, &rq("POST", "/api/tracker/state", &[], "{}")).status, 405);
        assert_eq!(handle(&s, &rq("GET", "/api/tracker/mode", &[], "")).status, 405);
    }

    #[test]
    fn the_long_poll_times_out_empty() {
        let s: Shared = tsh();
        let seq: u64 = s.tracker.events.seq();
        let mut r: Req = rq("GET", "/api/tracker/events", &[], "");
        r.query = vec![("after".into(), seq.to_string()), ("wait".into(), "100".into())];
        let t0: std::time::Instant = std::time::Instant::now();
        let out: Resp = handle(&s, &r);
        assert_eq!(out.status, 200);
        assert!(t0.elapsed() >= std::time::Duration::from_millis(90));
        assert!(t0.elapsed() < std::time::Duration::from_secs(3));
        let j: Json = ootrr_core::json::parse(&body_of(&out)).unwrap();
        assert_eq!(j.get("events").and_then(|x| x.as_arr()).map(|a| a.len()), Some(0));
        r.query = vec![("after".into(), "zz".into())];
        assert_eq!(handle(&s, &r).status, 400);
    }

    #[test]
    fn hidden_mode_never_returns_an_unrevealed_location() {
        let s: Shared = tsh();
        let ts: &[ootrr_core::tracker::targets::Target] = ootrr_core::tracker::targets::all().unwrap();
        let shown: &str = &ts.iter().find(|t| t.kind == "chest").unwrap().id;
        let hidden: &str = &ts.iter().filter(|t| t.kind == "chest").nth(5).unwrap().id;
        assert_eq!(handle(&s, &rq("POST", "/api/tracker/mark", &[], &format!("{{\"id\":\"{hidden}\",\"on\":true}}"))).status, 404);
        assert_eq!(handle(&s, &rq("POST", "/api/tracker/mode", &[], "{\"mode\":\"all\"}")).status, 200);
        assert_eq!(handle(&s, &rq("POST", "/api/tracker/mark", &[], &format!("{{\"id\":\"{shown}\",\"on\":true}}"))).status, 200);
        assert_eq!(handle(&s, &rq("POST", "/api/tracker/mode", &[], "{\"mode\":\"hidden\"}")).status, 200);
        for path in ["/api/tracker/state", "/api/tracker/index"] {
            let b: String = body_of(&handle(&s, &rq("GET", path, &[], "")));
            assert!(b.contains(shown), "{path}");
            assert!(!b.contains(hidden), "{path}");
        }
        let mut r: Req = rq("GET", "/api/tracker/events", &[], "");
        r.query = vec![("after".into(), "0".into()), ("wait".into(), "10".into())];
        let ev: String = body_of(&handle(&s, &r));
        assert!(!ev.contains(hidden));
        let pot: &str = &ts.iter().find(|t| t.kind == "pot").unwrap().id;
        assert_eq!(handle(&s, &rq("POST", "/api/tracker/mode", &[], "{\"mode\":\"all\"}")).status, 200);
        assert!(!body_of(&handle(&s, &rq("GET", "/api/tracker/index", &[], ""))).contains(pot));
        assert_eq!(handle(&s, &rq("POST", "/api/tracker/mode", &[], "{\"kinds\":[\"boat\"]}")).status, 400);
    }

    #[test]
    fn query_paths_are_checked() {
        let s: Shared = sh();
        let mut r: Req = rq("GET", "/api/fs", &[], "");
        r.query = vec![("path".into(), "/a/../b".into())];
        assert_eq!(handle(&s, &r).status, 422);
        r.path = "/api/validate".into();
        assert_eq!(handle(&s, &r).status, 422);
    }
}
