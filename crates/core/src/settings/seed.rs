use std::collections::BTreeMap as Map;
use std::fs;
use std::path::Path as P;

use super::digest::sha256_hex;
use super::{Loaded, Settings};
use crate::error::{Error, Res};
use crate::json::{self, Json};

pub const FORMAT: &str = "ootrr-seed";
pub const FORMAT_VERSION: u32 = 1;
pub const MAX_PLACEMENTS: usize = 100_000;
pub const MAX_SEED_BYTES: usize = 128;
pub const MAX_ID_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq)]
pub struct SeedFile {
    pub seed: String,
    pub settings_string: String,
    pub game: String,
    pub generator: String,
    pub placements: Map<String, String>,
}

fn bad(m: &str) -> Error {
    Error::Msg(format!("seed file: {m}"))
}

fn plain_text_ok(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && !s.chars().any(|c| c.is_control())
}

fn ident_ok(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_ID_BYTES
        && s.trim() == s
        && s.chars().all(|c| c.is_ascii_graphic() || c == ' ')
}

fn game_ok(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_ID_BYTES
        && s.chars().all(|c| c.is_ascii_alphanumeric() || "._:+-".contains(c))
}

fn field<'a>(j: &'a Json, k: &str) -> Res<&'a str> {
    match j.get(k) {
        Some(Json::Str(s)) => Ok(s),
        Some(_) => Err(bad(&format!("field {k} must be a string"))),
        None => Err(bad(&format!("field {k} is missing"))),
    }
}

impl SeedFile {
    pub fn new(seed: &str, settings: &Settings, game: &str, placements: Map<String, String>) -> Res<SeedFile> {
        let f: SeedFile = SeedFile {
            seed: seed.to_string(),
            settings_string: settings.encode()?,
            game: game.to_string(),
            generator: format!("ootrr-core {}", env!("CARGO_PKG_VERSION")),
            placements,
        };
        f.check_fields()?;
        Ok(f)
    }

    fn check_fields(&self) -> Res<()> {
        if !plain_text_ok(&self.seed, MAX_SEED_BYTES) {
            return Err(bad("the seed must be 1 to 128 bytes of text without control characters"));
        }
        if !game_ok(&self.game) {
            return Err(bad("the game fingerprint must be 1 to 128 characters from A-Z a-z 0-9 . _ : + -"));
        }
        if self.generator.len() > 64 || self.generator.chars().any(|c| c.is_control()) {
            return Err(bad("the generator field is too long or has control characters"));
        }
        if self.placements.len() > MAX_PLACEMENTS {
            return Err(bad("too many placements"));
        }
        for (k, v) in &self.placements {
            if !ident_ok(k) || !ident_ok(v) {
                return Err(bad(&format!("placement {k:?} to {v:?} has an id that is empty, too long or has odd characters")));
            }
        }
        Settings::decode(&self.settings_string)
            .map_err(|e| bad(&format!("the settings string is not usable: {e}")))?;
        Ok(())
    }

    pub fn settings(&self) -> Res<Loaded> {
        Settings::decode(&self.settings_string)
    }

    pub fn checksum(&self) -> String {
        let mut t: String = String::new();
        t.push_str(&format!("{FORMAT}\nversion\t{FORMAT_VERSION}\n"));
        t.push_str(&format!("seed\t{}\n", self.seed));
        t.push_str(&format!("settings\t{}\n", self.settings_string));
        t.push_str(&format!("game\t{}\n", self.game));
        for (k, v) in &self.placements {
            t.push_str(&format!("P\t{k}\t{v}\n"));
        }
        sha256_hex(t.as_bytes())
    }

    pub fn to_json(&self) -> Json {
        let p: Vec<(String, Json)> = self.placements.iter().map(|(k, v)| (k.clone(), Json::Str(v.clone()))).collect();
        Json::obj()
            .with("format", FORMAT.into())
            .with("version", Json::Int(FORMAT_VERSION as i64))
            .with("generator", self.generator.as_str().into())
            .with("seed", self.seed.as_str().into())
            .with("settings", self.settings_string.as_str().into())
            .with("game", self.game.as_str().into())
            .with("placements", Json::Obj(p))
            .with("checksum", self.checksum().into())
    }

    pub fn to_text(&self) -> String {
        let mut t: String = self.to_json().pretty();
        t.push('\n');
        t
    }

    pub fn from_json(j: &Json) -> Res<SeedFile> {
        if !matches!(j, Json::Obj(_)) {
            return Err(bad("the top level must be an object"));
        }
        if field(j, "format")? != FORMAT {
            return Err(bad("this is not an ootrr seed file"));
        }
        let ver: i64 = j
            .get("version")
            .and_then(|x| x.as_i64())
            .ok_or_else(|| bad("field version must be an integer"))?;
        match ver {
            1 => SeedFile::from_v1(j),
            v if v > FORMAT_VERSION as i64 => Err(bad(&format!(
                "format version {v} is newer than this build understands (version {FORMAT_VERSION}), update the tool"
            ))),
            v => Err(bad(&format!("format version {v} is not valid"))),
        }
    }

    fn from_v1(j: &Json) -> Res<SeedFile> {
        let m: &[(String, Json)] = j
            .get("placements")
            .and_then(|x| x.as_obj())
            .ok_or_else(|| bad("field placements must be an object"))?;
        if m.len() > MAX_PLACEMENTS {
            return Err(bad("too many placements"));
        }
        let mut placements: Map<String, String> = Map::new();
        for (k, v) in m {
            let item: &str = v.as_str().ok_or_else(|| bad(&format!("placement {k:?} must name an item as a string")))?;
            placements.insert(k.clone(), item.to_string());
        }
        let f: SeedFile = SeedFile {
            seed: field(j, "seed")?.to_string(),
            settings_string: field(j, "settings")?.to_string(),
            game: field(j, "game")?.to_string(),
            generator: match j.get("generator") {
                None => String::new(),
                Some(Json::Str(s)) => s.clone(),
                Some(_) => return Err(bad("field generator must be a string")),
            },
            placements,
        };
        f.check_fields()?;
        let want: String = field(j, "checksum")?.to_string();
        if want != f.checksum() {
            return Err(bad("the checksum does not match, the file was edited or damaged"));
        }
        Ok(f)
    }

    pub fn parse(text: &str) -> Res<SeedFile> {
        SeedFile::from_json(&json::parse(text)?)
    }

    pub fn read(p: &P) -> Res<SeedFile> {
        SeedFile::parse(&fs::read_to_string(p)?)
    }

    pub fn write(&self, p: &P) -> Res<()> {
        fs::write(p, self.to_text())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::Value;
    use super::*;

    fn placements(order: &[(&str, &str)]) -> Map<String, String> {
        order.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn sample() -> SeedFile {
        let mut s: Settings = Settings::default();
        s.set("shuffle_smallkeys", Value::s("keysanity")).unwrap();
        s.set("bridge", Value::s("open")).unwrap();
        let p: Map<String, String> = placements(&[
            ("deku_tree_compass_chest", "Hookshot"),
            ("kokiri_forest_chest", "Bow"),
            ("Skulltula House 10 Tokens", "Small Key (Forest Temple)"),
        ]);
        SeedFile::new("my seed 1", &s.finish().unwrap(), "oot-remake:1.0.0", p).unwrap()
    }

    fn edit(f: &str, find: &str, with: &str) -> String {
        assert!(f.contains(find), "{find} not in file");
        f.replacen(find, with, 1)
    }

    #[test]
    fn round_trip() {
        let f: SeedFile = sample();
        let t: String = f.to_text();
        let back: SeedFile = SeedFile::parse(&t).unwrap();
        assert_eq!(back, f);
        assert_eq!(back.to_text(), t);
        let l: Loaded = back.settings().unwrap();
        assert_eq!(l.settings.text("shuffle_smallkeys").unwrap(), "keysanity");
        assert!(l.warnings.is_empty());
    }

    #[test]
    fn same_inputs_give_the_same_bytes() {
        let a: String = sample().to_text();
        let b: String = sample().to_text();
        assert_eq!(a, b);
        let fwd: Map<String, String> = placements(&[("a", "x"), ("b", "y"), ("c", "z")]);
        let rev: Map<String, String> = placements(&[("c", "z"), ("b", "y"), ("a", "x")]);
        let s: Settings = Settings::default();
        assert_eq!(
            SeedFile::new("s", &s, "g", fwd).unwrap().to_text(),
            SeedFile::new("s", &s, "g", rev).unwrap().to_text()
        );
    }

    #[test]
    fn different_content_changes_the_checksum() {
        let a: SeedFile = sample();
        let mut b: SeedFile = a.clone();
        b.seed = "other".into();
        let mut c: SeedFile = a.clone();
        c.placements.insert("kokiri_forest_chest".into(), "Boomerang".into());
        let mut d: SeedFile = a.clone();
        d.game = "oot-remake:1.0.1".into();
        let mut e: SeedFile = a.clone();
        e.settings_string = Settings::default().encode().unwrap();
        let sums: Vec<String> = vec![a.checksum(), b.checksum(), c.checksum(), d.checksum(), e.checksum()];
        for i in 0..sums.len() {
            for j in i + 1..sums.len() {
                assert_ne!(sums[i], sums[j]);
            }
        }
    }

    #[test]
    fn tampering_is_caught() {
        let t: String = sample().to_text();
        for (find, with) in [
            ("\"Bow\"", "\"Longshot\""),
            ("my seed 1", "my seed 2"),
            ("oot-remake:1.0.0", "oot-remake:9.9.9"),
            ("\"kokiri_forest_chest\"", "\"kokiri_forest_chest2\""),
        ] {
            let e: String = SeedFile::parse(&edit(&t, find, with)).unwrap_err().to_string();
            assert!(e.contains("checksum"), "{find}: {e}");
        }
    }

    #[test]
    fn informational_and_unknown_fields_are_tolerated() {
        let t: String = sample().to_text();
        let a: String = edit(&t, "ootrr-core 0.1.0", "some other build");
        assert!(SeedFile::parse(&a).is_ok());
        let b: String = edit(&t, "\"format\"", "\"note\": \"hello\",\n  \"format\"");
        assert_eq!(SeedFile::parse(&b).unwrap(), sample());
    }

    #[test]
    fn versions() {
        let t: String = sample().to_text();
        let e: String = SeedFile::parse(&edit(&t, "\"version\": 1", "\"version\": 2")).unwrap_err().to_string();
        assert!(e.contains("newer"), "{e}");
        for v in ["0", "-1", "\"1\"", "1.5", "null"] {
            assert!(SeedFile::parse(&edit(&t, "\"version\": 1", &format!("\"version\": {v}"))).is_err(), "{v}");
        }
        let missing: String = edit(&t, "\"version\": 1,", "");
        assert!(SeedFile::parse(&missing).is_err());
    }

    #[test]
    fn malformed_files_are_rejected() {
        let t: String = sample().to_text();
        let cases: Vec<String> = vec![
            String::new(),
            "[]".into(),
            "{}".into(),
            "null".into(),
            edit(&t, "ootrr-seed", "something-else"),
            edit(&t, "\"seed\": \"my seed 1\"", "\"seed\": 5"),
            edit(&t, "\"seed\": \"my seed 1\"", "\"seed\": \"\""),
            edit(&t, "\"seed\": \"my seed 1\"", "\"seed\": \"a\\nb\""),
            edit(&t, "\"game\": \"oot-remake:1.0.0\"", "\"game\": \"has space\""),
            edit(&t, "\"game\": \"oot-remake:1.0.0\"", "\"game\": \"\""),
            edit(&t, "\"placements\": {", "\"placements\": [], \"x\": {"),
            edit(&t, "\"Bow\"", "7"),
            edit(&t, "\"Bow\"", "\"\""),
            edit(&t, "\"kokiri_forest_chest\"", "\" padded\""),
            edit(&t, "\"kokiri_forest_chest\"", "\"deku_tree_compass_chest\""),
            edit(&t, "\"checksum\": \"", "\"checksum\": \"0"),
            edit(&t, "\"checksum\"", "\"checksum_\""),
            edit(&t, "OOTRR1-", "OOTRR9-"),
            edit(&t, "OOTRR1-", "OOTRR1-AAAA"),
            t[..t.len() / 2].to_string(),
        ];
        for c in cases {
            assert!(SeedFile::parse(&c).is_err(), "accepted {c}");
        }
    }

    #[test]
    fn new_rejects_bad_fields() {
        let s: Settings = Settings::default();
        assert!(SeedFile::new("", &s, "g", Map::new()).is_err());
        assert!(SeedFile::new("s", &s, "", Map::new()).is_err());
        assert!(SeedFile::new(&"x".repeat(129), &s, "g", Map::new()).is_err());
        assert!(SeedFile::new("s", &s, "g", placements(&[("a\tb", "x")])).is_err());
        let many: Map<String, String> = (0..=MAX_PLACEMENTS).map(|i| (format!("l{i}"), "x".to_string())).collect();
        assert!(SeedFile::new("s", &s, "g", many).is_err());
    }

    #[test]
    fn large_seed_file_round_trips() {
        let s: Settings = Settings::default();
        let big: Map<String, String> = (0..MAX_PLACEMENTS).map(|i| (format!("location_{i}"), format!("item_{}", i % 97))).collect();
        let f: SeedFile = SeedFile::new("big", &s, "g", big).unwrap();
        let t: String = f.to_text();
        assert_eq!(SeedFile::parse(&t).unwrap(), f);
    }

    #[test]
    fn settings_with_unknown_entries_still_open() {
        let t: String = sample().to_text();
        let s: SeedFile = SeedFile::parse(&t).unwrap();
        assert_eq!(s.settings().unwrap().settings.text("bridge").unwrap(), "open");
    }

    #[test]
    fn file_round_trip() {
        let d: std::path::PathBuf = std::env::temp_dir().join(format!("ootrr_seed_test_{}.ootrr", std::process::id()));
        let f: SeedFile = sample();
        f.write(&d).unwrap();
        let back: SeedFile = SeedFile::read(&d).unwrap();
        let _ = fs::remove_file(&d);
        assert_eq!(back, f);
        assert!(SeedFile::read(&d).is_err());
    }

    const FROZEN_V1: &str = r#"{
  "format": "ootrr-seed",
  "version": 1,
  "generator": "ootrr-core 0.1.0",
  "seed": "frozen seed",
  "settings": "OOTRR1-BEAQGGABAUOACZBEAMBAABWEOQ",
  "game": "oot-remake:1.0.0",
  "placements": {
    "deku_tree_compass_chest": "Hookshot",
    "kokiri_forest_chest": "Bow"
  },
  "checksum": "5543b08c75002907feb0ae0af8fb6d7989e1de3725478725472b9c900e2c64b2"
}
"#;

    #[test]
    fn frozen_v1_file_keeps_loading() {
        let f: SeedFile = SeedFile::parse(FROZEN_V1).unwrap();
        assert_eq!(f.seed, "frozen seed");
        assert_eq!(f.placements.len(), 2);
        let l: Loaded = f.settings().unwrap();
        assert_eq!(l.settings.text("bridge").unwrap(), "tokens");
        assert_eq!(l.settings.int("bridge_tokens").unwrap(), 50);
        assert_eq!(l.settings.text("shuffle_pots").unwrap(), "all");
        assert_eq!(l.settings.items("starting_items").unwrap(), ["ocarina", "hookshot"]);
        assert!(l.warnings.is_empty());
        assert_eq!(f.to_text(), FROZEN_V1);
    }
}
