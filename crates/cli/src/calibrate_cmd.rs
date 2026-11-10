use std::path::Path as P;

use ootrr_core::calibrate::{self, Done, Plan};
use ootrr_core::catalog::Catalog;
use ootrr_core::error::{Error, Res};
use ootrr_core::romfs::Romfs;
use ootrr_core::slice::data::CATALOG_JSON;

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(Error::Format(what))
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let area: &str = arg(a, 3, "missing area")?;
    let mut cat_path: Option<&str> = None;
    let mut dry: bool = false;
    let mut i: usize = 4;
    while i < a.len() {
        match a[i].as_str() {
            "--catalog" => {
                i += 1;
                cat_path = Some(arg(a, i, "--catalog needs a file")?);
            }
            "--dry" => dry = true,
            other => return Err(Error::Msg(format!("unknown option {other}"))),
        }
        i += 1;
    }
    let cat: Catalog = match cat_path {
        Some(p) => Catalog::load(P::new(p))?,
        None => Catalog::parse(CATALOG_JSON)?,
    };
    let mut rf: Romfs = Romfs::open(P::new(root))?;
    let p: Plan = calibrate::plan(&cat, &mut rf, area)?;
    if dry {
        print!("{}", calibrate::report(&p, None));
        return Ok(());
    }
    let d: Done = calibrate::build(&p, &mut rf, P::new(load), name)?;
    print!("{}", calibrate::report(&p, Some(&d)));
    if d.issues.is_empty() {
        Ok(())
    } else {
        Err(Error::Format("calibration mod failed the decode check"))
    }
}
