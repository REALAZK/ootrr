use std::path::Path as P;

use ootrr_core::error::{Error, Res};
use ootrr_core::qol::navi_return::Mode;
use ootrr_core::qol::{build, Opts, Skip};

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let o: Opts = match arg(a, 3, "missing option (skip-story, skip-all, anju, poe, navi-dungeons or navi-everywhere)")? {
        "skip-story" => Opts { skip: Skip::Story, ..Opts::default() },
        "skip-all" => Opts { skip: Skip::All, ..Opts::default() },
        "anju" => Opts { anju: true, ..Opts::default() },
        "poe" => Opts { poe: true, ..Opts::default() },
        "navi-dungeons" => Opts { navi: Mode::Dungeons, ..Opts::default() },
        "navi-everywhere" => Opts { navi: Mode::Everywhere, ..Opts::default() },
        x => return Err(Error::Msg(format!("unknown option {x}"))),
    };
    print!("{}", build(P::new(root), P::new(load), name, &o)?);
    Ok(())
}
