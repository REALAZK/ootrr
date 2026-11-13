use std::path::Path as P;

use ootrr_core::error::{Error, Res};
use ootrr_core::start_defaults::{build, opening, presets, Edit};

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

fn pair(s: &str, sep: char) -> Res<(String, String)> {
    s.split_once(sep).map(|(k, v)| (k.to_string(), v.to_string())).ok_or_else(|| Error::Msg(format!("expected NAME{sep}VALUE, got {s}")))
}

fn int(s: &str) -> Res<i32> {
    s.parse::<i32>().map_err(|_| Error::Msg(format!("{s} is not a 32-bit integer")))
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let mut edits: Vec<Edit> = Vec::new();
    let mut step: Option<String> = None;
    let mut i: usize = 3;
    while i < a.len() {
        let k: &str = a[i].as_str();
        if k == "--control" {
            edits.extend(presets::book()?.control);
            i += 1;
            continue;
        }
        let v: &str = arg(a, i + 1, "an option is missing its value")?;
        match k {
            "--preset" => {
                let (s, x): (String, String) = pair(v, '=')?;
                edits.extend(presets::chosen(&[(s.as_str(), x.as_str())])?);
            }
            "--bool" => {
                let (n, x): (String, String) = pair(v, '=')?;
                let b: bool = match x.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => return Err(Error::Msg(format!("{n} needs true or false"))),
                };
                edits.push(Edit::Bool(n, b));
            }
            "--int" => {
                let (n, x): (String, String) = pair(v, '=')?;
                edits.push(Edit::Int(n, int(&x)?));
            }
            "--enum" => {
                let (n, x): (String, String) = pair(v, '=')?;
                edits.push(Edit::Enum(n, x));
            }
            "--grant" => {
                let (n, x): (String, String) = pair(v, ':')?;
                edits.push(Edit::Grant(n, int(&x)?));
            }
            "--opening-step" => step = Some(v.to_string()),
            "--open-forest-step" => step = Some(opening::OPEN_STEP.to_string()),
            x => return Err(Error::Msg(format!("unknown option {x}"))),
        }
        i += 2;
    }
    let report: String = build(P::new(root), P::new(load), name, &edits, step.as_deref())?;
    print!("{report}");
    for e in &edits {
        println!("default: {e:?}");
    }
    if let Some(s) = &step {
        println!("opening: {} request set to {s}", opening::QUEST);
    }
    Ok(())
}
