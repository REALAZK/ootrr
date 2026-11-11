use std::fs;
use std::path::PathBuf as PB;
use std::sync::Arc;

use ootrr_core::TITLE_ID;
use ootrr_core::json::Json;
use ootrr_core::settings::seed::SeedFile;
use ootrr_core::slice;

use crate::api_validate::{self, Report};
use crate::body;
use crate::error::{Fail, Hr};
use crate::jobs::{Ctl, Jobs};
use crate::pathsafe;

fn refuse(m: &str) -> Fail {
    Fail::new(422, m)
}

fn inside(inner: &PB, outer: &PB) -> bool {
    match (fs::canonicalize(inner), fs::canonicalize(outer)) {
        (Ok(a), Ok(b)) => a.starts_with(b),
        _ => false,
    }
}

pub struct Plan {
    pub seed: SeedFile,
    pub romfs: PB,
    pub load: PB,
    pub name: String,
    pub target: PB,
}

pub fn plan(b: &Json) -> Hr<Plan> {
    let seed: SeedFile = SeedFile::parse(body::text(b, "seed_text")?)?;
    let game: PB = pathsafe::abs(body::short(b, "game_dir")?)?;
    let rep: Report = api_validate::inspect(&game);
    if !rep.ok {
        return Err(refuse(&format!("Game folder: {}", rep.problems.join(" "))));
    }
    let md: PB = pathsafe::abs(body::short(b, "mod_dir")?)?;
    if !md.is_dir() {
        return Err(refuse("Mod folder: that folder does not exist. Create it first or pick another one."));
    }
    let name: &str = body::short(b, "mod_name")?;
    if !pathsafe::mod_name_ok(name) {
        return Err(refuse("Mod name: use 1 to 64 letters, digits, spaces and _ - ! . characters, with no spaces at the ends."));
    }
    if inside(&md, &game) {
        return Err(refuse("Mod folder: it may not be inside the game folder."));
    }
    let romfs: PB = rep.romfs.unwrap_or_else(|| game.clone());
    let target: PB = md.join(TITLE_ID).join(name);
    Ok(Plan { seed, romfs, load: md, name: name.to_string(), target })
}

pub fn run(jobs: &Arc<Jobs>, b: &Json) -> Hr<Json> {
    let p: Plan = plan(b)?;
    let target: String = pathsafe::text(&p.target);
    let n: usize = p.seed.placements.len();
    let id: u64 = jobs.start(move |c: &Ctl| {
        c.begin(&format!("Seed file checked, {n} placements"));
        c.ok(10);
        c.begin("Read the game files and edit the chests");
        match slice::build(&p.seed, &p.romfs, &p.load, &p.name) {
            Ok(built) => {
                c.ok(90);
                c.begin("Mod files written");
                c.ok(100);
                c.finish("done", &built.report);
            }
            Err(e) => {
                c.mark("failed");
                c.finish("failed", &e.to_string());
            }
        }
    })?;
    Ok(Json::obj()
        .with("job_id", Json::Int(id as i64))
        .with("target", target.into())
        .with("writes_files", true.into())
        .with("notice", "Building the mod. Only the mod folder is written, the game folder is only read.".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ootrr_core::settings::Settings;
    use std::collections::BTreeMap;
    use std::env;
    use std::thread;
    use std::time::Duration;

    fn tmp(tag: &str) -> PB {
        let p: PB = env::temp_dir().join(format!("ootrr_web_build_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn game(root: &PB) -> PB {
        let g: PB = root.join("game");
        fs::create_dir_all(g.join("Pack")).unwrap();
        fs::create_dir_all(g.join("Banc")).unwrap();
        fs::write(g.join("Pack").join("ZsDic.pack.zs"), b"").unwrap();
        crate::api_validate::fake_version(&g);
        g
    }

    fn req(seed: &str, g: &PB, m: &PB, name: &str) -> Json {
        Json::obj()
            .with("seed_text", seed.into())
            .with("game_dir", g.display().to_string().into())
            .with("mod_dir", m.display().to_string().into())
            .with("mod_name", name.into())
    }

    fn seed_text() -> String {
        SeedFile::new("s", &Settings::default(), "oot-remake:test", BTreeMap::new()).unwrap().to_text()
    }

    fn files_under(p: &PB) -> usize {
        fs::read_dir(p).map(|d| d.count()).unwrap_or(0)
    }

    #[test]
    fn a_good_request_against_a_fake_game_folder_fails_cleanly_and_writes_nothing() {
        let d: PB = tmp("ok");
        let g: PB = game(&d);
        let m: PB = d.join("mods");
        fs::create_dir_all(&m).unwrap();
        let jobs: Arc<Jobs> = Jobs::new();
        let r: Json = run(&jobs, &req(&seed_text(), &g, &m, "My Seed")).unwrap();
        let id: u64 = r.get("job_id").and_then(|x| x.as_i64()).unwrap() as u64;
        let mut state: String = String::new();
        for _ in 0..300 {
            let j: Json = jobs.get(id).unwrap();
            state = j.get("state").and_then(|x| x.as_str()).unwrap().to_string();
            if state != "running" {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(state, "failed");
        assert_eq!(files_under(&m), 0);
        assert!(r.get("target").and_then(|x| x.as_str()).unwrap().ends_with("My Seed"));
        assert_eq!(r.get("writes_files").and_then(|x| x.as_bool()), Some(true));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn every_bad_input_is_refused_before_a_job_starts() {
        let d: PB = tmp("bad");
        let g: PB = game(&d);
        let m: PB = d.join("mods");
        fs::create_dir_all(&m).unwrap();
        let inside_game: PB = g.join("Banc");
        let empty: PB = d.join("empty");
        fs::create_dir_all(&empty).unwrap();
        let jobs: Arc<Jobs> = Jobs::new();
        let s: String = seed_text();
        let cases: Vec<Json> = vec![
            req("not a seed", &g, &m, "x"),
            req(&s.replace("\"s\"", "\"t\""), &g, &m, "x"),
            req(&s, &empty, &m, "x"),
            req(&s, &d.join("nope"), &m, "x"),
            req(&s, &g, &d.join("nope"), "x"),
            req(&s, &g, &m, ".."),
            req(&s, &g, &m, "a/b"),
            req(&s, &g, &m, ""),
            req(&s, &g, &inside_game, "x"),
            req(&s, &g, &g, "x"),
            req(&s, &g, &m.join("..").join("mods"), "x"),
        ];
        for c in &cases {
            assert!(run(&jobs, c).is_err(), "{c}");
        }
        assert_eq!(jobs.running(), 0);
        assert_eq!(files_under(&m), 0);
        let _ = fs::remove_dir_all(&d);
    }
}
