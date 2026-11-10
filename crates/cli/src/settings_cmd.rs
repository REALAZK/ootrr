use std::path::Path as P;

use ootrr_core::error::{Error, Res};
use ootrr_core::json::Json;
use ootrr_core::settings::export::schema_json;
use ootrr_core::settings::presets::{self, Preset};
use ootrr_core::settings::{Loaded, Settings, schema};

struct Opts {
    compact: bool,
    presets: Option<String>,
    decode: Option<String>,
}

fn opts(a: &[String]) -> Res<Opts> {
    let mut o: Opts = Opts { compact: false, presets: None, decode: None };
    let mut i: usize = 0;
    while i < a.len() {
        match a[i].as_str() {
            "--compact" => o.compact = true,
            "--presets" => {
                i += 1;
                o.presets = Some(a.get(i).ok_or(Error::Format("--presets needs a folder"))?.clone());
            }
            "--decode" => {
                i += 1;
                o.decode = Some(a.get(i).ok_or(Error::Format("--decode needs a settings string"))?.clone());
            }
            other => return Err(Error::Msg(format!("unknown option {other}"))),
        }
        i += 1;
    }
    Ok(o)
}

fn render(j: &Json, compact: bool) -> String {
    if compact { j.to_string() } else { j.pretty() }
}

pub fn run(a: &[String]) -> Res<()> {
    let o: Opts = opts(a)?;
    if let Some(t) = &o.decode {
        let l: Loaded = Settings::decode(t)?;
        let j: Json = Json::obj()
            .with("settings", l.settings.to_json())
            .with("warnings", Json::strs(l.warnings.iter().map(|s| s.as_str())));
        println!("{}", render(&j, o.compact));
        return Ok(());
    }
    let ps: Vec<Preset> = match &o.presets {
        Some(d) => presets::load_dir(P::new(d))?,
        None => presets::builtin()?,
    };
    println!("{}", render(&schema_json(schema(), &ps), o.compact));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn options_parse() {
        let o: Opts = opts(&v(&["--compact", "--presets", "x"])).unwrap();
        assert!(o.compact);
        assert_eq!(o.presets.as_deref(), Some("x"));
        assert!(opts(&v(&["--presets"])).is_err());
        assert!(opts(&v(&["--decode"])).is_err());
        assert!(opts(&v(&["--wat"])).is_err());
    }

    #[test]
    fn decode_and_dump_run() {
        let t: String = Settings::default().encode().unwrap();
        assert!(run(&v(&["--decode", &t, "--compact"])).is_ok());
        assert!(run(&v(&["--decode", "garbage"])).is_err());
        assert!(run(&v(&["--compact"])).is_ok());
    }
}
