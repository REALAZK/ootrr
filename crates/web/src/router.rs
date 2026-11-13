use ootrr_core::json::Json;

use crate::error::{Fail, Hr};
use crate::req::Req;
use crate::resp::Resp;
use crate::state::Shared;
use crate::{update, api_build, api_fs, api_generate, api_info, api_open, api_seed, api_settings, api_validate, assets, body};

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
            api_generate::run(&body::json(r)?)?
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
    fn query_paths_are_checked() {
        let s: Shared = sh();
        let mut r: Req = rq("GET", "/api/fs", &[], "");
        r.query = vec![("path".into(), "/a/../b".into())];
        assert_eq!(handle(&s, &r).status, 422);
        r.path = "/api/validate".into();
        assert_eq!(handle(&s, &r).status, 422);
    }
}
