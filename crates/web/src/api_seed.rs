use ootrr_core::json::Json;
use ootrr_core::settings::Loaded;
use ootrr_core::settings::seed::SeedFile;

use crate::api_settings;
use crate::body;
use crate::error::Hr;

pub fn describe(sf: &SeedFile) -> Hr<Json> {
    let l: Loaded = sf.settings()?;
    let mut out: Json = api_settings::reply(&l)?;
    let ps: Vec<Json> = sf
        .placements
        .iter()
        .map(|(k, v)| Json::Arr(vec![k.as_str().into(), v.as_str().into()]))
        .collect();
    out.put("seed", sf.seed.as_str().into());
    out.put("game", sf.game.as_str().into());
    out.put("generator", sf.generator.as_str().into());
    out.put("checksum", sf.checksum().into());
    out.put("placement_count", Json::Int(sf.placements.len() as i64));
    out.put("placements_ready", (!sf.placements.is_empty()).into());
    out.put("placements", Json::Arr(ps));
    Ok(out)
}

pub fn read(b: &Json) -> Hr<Json> {
    let sf: SeedFile = SeedFile::parse(body::text(b, "seed_text")?)?;
    describe(&sf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ootrr_core::json;
    use ootrr_core::settings::Settings;
    use std::collections::BTreeMap;

    #[test]
    fn a_seed_file_reads_back_with_its_placements() {
        let mut p: BTreeMap<String, String> = BTreeMap::new();
        p.insert("kokiri_forest_chest".into(), "Bow".into());
        p.insert("deku_tree_compass_chest".into(), "Hookshot".into());
        let sf: SeedFile = SeedFile::new("abc", &Settings::default(), "oot-remake:test", p).unwrap();
        let req: Json = Json::obj().with("seed_text", sf.to_text().into());
        let r: Json = read(&req).unwrap();
        assert_eq!(r.get("seed").and_then(|x| x.as_str()), Some("abc"));
        assert_eq!(r.get("placement_count").and_then(|x| x.as_i64()), Some(2));
        assert_eq!(r.get("placements_ready").and_then(|x| x.as_bool()), Some(true));
        let first: &Json = &r.get("placements").and_then(|x| x.as_arr()).unwrap()[0];
        assert_eq!(first.to_string(), r#"["deku_tree_compass_chest","Hookshot"]"#);
        assert_eq!(r.get("checksum").and_then(|x| x.as_str()), Some(sf.checksum().as_str()));
    }

    #[test]
    fn damaged_seed_files_are_422() {
        let sf: SeedFile = SeedFile::new("abc", &Settings::default(), "g", BTreeMap::new()).unwrap();
        let bad: String = sf.to_text().replace("abc", "abd");
        let req: Json = Json::obj().with("seed_text", bad.into());
        assert_eq!(read(&req).unwrap_err().status, 422);
        let junk: Json = json::parse(r#"{"seed_text":"not json"}"#).unwrap();
        assert_eq!(read(&junk).unwrap_err().status, 422);
        assert_eq!(read(&Json::obj()).unwrap_err().status, 400);
    }
}
