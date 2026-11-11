use ootrr_core::json::Json;
use ootrr_core::settings::Loaded;
use ootrr_core::slice::data::Data;
use ootrr_core::slice::plando::Entry;
use ootrr_core::slice::support::plan;
use ootrr_core::slice::{self, GenOpts, Generated};

use crate::api_seed;
use crate::api_settings;
use crate::body;
use crate::error::{Fail, Hr};
use crate::token;

fn slug(seed: &str) -> String {
    let s: String = seed
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .take(32)
        .collect();
    let t: &str = s.trim_matches('-');
    if t.is_empty() { "seed".to_string() } else { t.to_string() }
}

pub fn run(b: &Json) -> Hr<Json> {
    let l: Loaded = api_settings::from_body(b)?;
    let seed: String = match body::opt_text(b, "seed")? {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => token::seed_text(12),
    };
    let want_spoiler: bool = b.get("spoiler").and_then(|x| x.as_bool()).unwrap_or(true);
    let opts: GenOpts = GenOpts { spoiler: want_spoiler, plando: plando_of(b)?, ..GenOpts::default() };
    let g: Generated = slice::generate_with(&l.settings, &seed, &opts)?;
    let mut out: Json = api_seed::describe(&g.seed_file)?;
    let w: Json = Json::strs(l.warnings.iter().map(|x| x.as_str()));
    out.put("warnings", w);
    out.put("ok", true.into());
    out.put("report", g.report.as_str().into());
    out.put("notes", Json::strs(notes(&l).iter().map(|x| x.as_str())));
    out.put("file_name", format!("ootrr-{}.ootrr", slug(&seed)).into());
    out.put("seed_text", g.seed_file.to_text().into());
    out.put("spoiler", spoiler(&g, want_spoiler));
    Ok(out)
}

fn notes(l: &Loaded) -> Vec<String> {
    match plan(&l.settings) {
        Ok(p) => p.notes.into_iter().filter(|n| !n.starts_with("Not applied")).collect(),
        Err(_) => Vec::new(),
    }
}

fn plando_of(b: &Json) -> Hr<Vec<Entry>> {
    let Some(v) = b.get("plando") else {
        return Ok(Vec::new());
    };
    let list: &[Json] = v.as_arr().ok_or_else(|| Fail::bad("plando must be a list"))?;
    let mut out: Vec<Entry> = Vec::with_capacity(list.len());
    for x in list {
        let l: &str = x.get("location").and_then(|y| y.as_str()).ok_or_else(|| Fail::bad("each plando entry needs a location"))?;
        let i: &str = x.get("item").and_then(|y| y.as_str()).ok_or_else(|| Fail::bad("each plando entry needs an item"))?;
        if l.len() > body::MAX_STRING || i.len() > body::MAX_STRING {
            return Err(Fail::bad("a plando name is too long"));
        }
        out.push(Entry { location: l.to_string(), item: i.to_string() });
    }
    Ok(out)
}

pub fn options(b: &Json) -> Hr<Json> {
    let l: Loaded = api_settings::from_body(b)?;
    let d: Data = Data::load()?;
    let (locs, items): (Vec<String>, Vec<String>) = slice::plando_options(&d, &l.settings)?;
    Ok(Json::obj().with("locations", Json::strs(locs.iter().map(|x| x.as_str()))).with("items", Json::strs(items.iter().map(|x| x.as_str()))))
}

fn pair(l: &str, i: &str) -> Json {
    Json::Arr(vec![l.into(), i.into()])
}

fn spoiler(g: &Generated, wanted: bool) -> Json {
    if !wanted {
        return Json::obj()
            .with("available", false.into())
            .with("reason", "The spoiler was not requested for this seed.".into())
            .with("locations", Json::Arr(Vec::new()))
            .with("playthrough", Json::Arr(Vec::new()));
    }
    let locs: Vec<Json> = g.seed_file.placements.iter().map(|(k, v)| pair(k, v)).collect();
    let steps: Vec<Json> = g
        .spheres
        .iter()
        .map(|sp| Json::Arr(sp.iter().map(|(l, i)| pair(l, i)).collect()))
        .collect();
    Json::obj()
        .with("available", true.into())
        .with("locations", Json::Arr(locs))
        .with("playthrough", Json::Arr(steps))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ootrr_core::json;
    use ootrr_core::settings::seed::SeedFile;

    fn j(t: &str) -> Json {
        json::parse(t).unwrap()
    }

    #[test]
    fn a_generated_seed_has_placements_and_a_spoiler() {
        let r: Json = run(&j(r#"{"seed":"frozen seed","settings":{}}"#)).unwrap();
        assert_eq!(r.get("placements_ready").and_then(|x| x.as_bool()), Some(true));
        assert!(r.get("placement_count").and_then(|x| x.as_i64()).unwrap() > 0);
        assert_eq!(r.get("seed").and_then(|x| x.as_str()), Some("frozen seed"));
        assert_eq!(r.get("file_name").and_then(|x| x.as_str()), Some("ootrr-frozen-seed.ootrr"));
        let t: &str = r.get("seed_text").and_then(|x| x.as_str()).unwrap();
        let sf: SeedFile = SeedFile::parse(t).unwrap();
        assert!(!sf.placements.is_empty());
        assert_eq!(sf.checksum(), r.get("checksum").and_then(|x| x.as_str()).unwrap());
        assert!(t.ends_with("}
"));
        let sp: &Json = r.get("spoiler").unwrap();
        assert_eq!(sp.get("available").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(sp.get("locations").and_then(|x| x.as_arr()).unwrap().len(), sf.placements.len());
        assert!(!sp.get("playthrough").and_then(|x| x.as_arr()).unwrap().is_empty());
        assert!(r.get("report").and_then(|x| x.as_str()).unwrap().contains("Randomized"));
    }

    #[test]
    fn plando_entries_are_honoured_and_bad_ones_say_why() {
        let t: &str = r#"{"seed":"p","settings":{"shuffle_chests":"all"},"plando":[{"location":"KF Midos Top Left Chest","item":"Boomerang"}]}"#;
        let r: Json = run(&j(t)).unwrap();
        let sf: SeedFile = SeedFile::parse(r.get("seed_text").and_then(|x| x.as_str()).unwrap()).unwrap();
        assert_eq!(sf.placements.get("KF Midos Top Left Chest").map(|s| s.as_str()), Some("Boomerang"));
        let bad: &str = r#"{"seed":"p","settings":{"shuffle_chests":"all"},"plando":[{"location":"Nowhere Chest","item":"Boomerang"}]}"#;
        assert!(run(&j(bad)).is_err());
        let shape: &str = r#"{"seed":"p","settings":{},"plando":"x"}"#;
        assert!(run(&j(shape)).is_err());
        let o: Json = options(&j(r#"{"settings":{"shuffle_chests":"all"}}"#)).unwrap();
        let locs: &[Json] = o.get("locations").and_then(|x| x.as_arr()).unwrap();
        assert!(locs.iter().any(|x| x.as_str() == Some("KF Midos Top Left Chest")));
        assert!(o.get("items").and_then(|x| x.as_arr()).unwrap().iter().any(|x| x.as_str() == Some("Boomerang")));
    }

    #[test]
    fn the_spoiler_can_be_left_out() {
        let r: Json = run(&j(r#"{"seed":"s","settings":{},"spoiler":false}"#)).unwrap();
        let sp: &Json = r.get("spoiler").unwrap();
        assert_eq!(sp.get("available").and_then(|x| x.as_bool()), Some(false));
        assert_eq!(r.get("placements_ready").and_then(|x| x.as_bool()), Some(true));
    }

    #[test]
    fn same_inputs_give_the_same_bytes() {
        let a: Json = run(&j(r#"{"seed":"s1","settings":{"shuffle_mapcompass":"vanilla"}}"#)).unwrap();
        let b: Json = run(&j(r#"{"seed":"s1","settings":{"shuffle_mapcompass":"vanilla"}}"#)).unwrap();
        assert_eq!(a.get("seed_text"), b.get("seed_text"));
        let c: Json = run(&j(r#"{"seed":"s2","settings":{"shuffle_mapcompass":"vanilla"}}"#)).unwrap();
        assert_ne!(a.get("checksum"), c.get("checksum"));
    }

    #[test]
    fn a_blank_seed_is_replaced_by_a_random_one() {
        let a: Json = run(&j(r#"{"seed":"  ","settings":{}}"#)).unwrap();
        let b: Json = run(&j(r#"{"settings":{}}"#)).unwrap();
        let sa: &str = a.get("seed").and_then(|x| x.as_str()).unwrap();
        let sb: &str = b.get("seed").and_then(|x| x.as_str()).unwrap();
        assert_eq!(sa.len(), 12);
        assert_ne!(sa, sb);
    }

    #[test]
    fn the_settings_string_route_gives_the_same_seed_as_the_object_route() {
        let a: Json = run(&j(r#"{"seed":"x","settings":{"shuffle_mapcompass":"vanilla","shuffle_smallkeys":"vanilla"}}"#)).unwrap();
        let s: &str = a.get("settings_string").and_then(|x| x.as_str()).unwrap();
        let b: Json = run(&j(&format!("{{\"seed\":\"x\",\"settings_string\":\"{s}\"}}"))).unwrap();
        assert_eq!(a.get("checksum"), b.get("checksum"));
    }

    #[test]
    fn bad_input_is_refused() {
        assert_eq!(run(&j(r#"{"seed":"a\u0001b","settings":{}}"#)).unwrap_err().status, 422);
        let long: String = format!("{{\"seed\":\"{}\",\"settings\":{{}}}}", "a".repeat(129));
        assert_eq!(run(&j(&long)).unwrap_err().status, 422);
        assert_eq!(run(&j(r#"{"seed":"x","settings":{"bridge":"nope"}}"#)).unwrap_err().status, 422);
        assert_eq!(run(&j(r#"{"seed":5,"settings":{}}"#)).unwrap_err().status, 400);
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(slug("../../etc/passwd"), "etc-passwd");
        assert_eq!(slug(r"a b\c:d"), "a-b-c-d");
        assert_eq!(slug("???"), "seed");
        assert!(slug(&"x".repeat(100)).len() <= 32);
    }
}
