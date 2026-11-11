use std::path::Path as P;

use ootrr_core::error::{Error, Res};
use ootrr_core::intro_skip::{build, Opts, Report};

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let mut o: Opts = Opts::FULL;
    for s in a.iter().skip(3) {
        match s.as_str() {
            "--movie-only" => o = Opts { movie: true, scene: false, hold: false },
            "--scene-only" => o = Opts { movie: false, scene: true, hold: false },
            "--hold" => o = Opts { movie: false, scene: false, hold: true },
            "--hold-and-scene" => o = Opts { movie: false, scene: true, hold: true },
            x => return Err(Error::Msg(format!("unknown option {x}"))),
        }
    }
    let (report, r): (String, Report) = build(P::new(root), P::new(load), name, o)?;
    print!("{report}");
    println!("flags set by the flow: {}", if r.flags.is_empty() { "none".to_string() } else { r.flags.join(", ") });
    println!("flow events added: {}, background music resume forced: {}", r.added_events, r.bgm_forced);
    Ok(())
}
