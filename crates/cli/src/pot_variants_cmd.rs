use std::path::Path as P;

use ootrr_core::error::{Error as E, Res};
use ootrr_core::modpack::Host;
use ootrr_core::pickup_pack::pot_kit::{run as add, ACTORS};
use ootrr_core::pickup_pack::write::{Done, Opts};

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(E::Format(what))
}

pub fn parse(a: &[String]) -> Res<Opts> {
    let mut o: Opts = Opts { host: Host::Eden, base: None, rst_from: None, places: Vec::new(), dry: false };
    let mut i: usize = 3;
    while i < a.len() {
        let step: usize = match a[i].as_str() {
            "--dry" => {
                o.dry = true;
                1
            }
            "--host" => {
                let v: &str = arg(a, i + 1, "missing host")?;
                o.host = Host::parse(v).ok_or_else(|| E::Msg(format!("{v} is not a known host")))?;
                2
            }
            "--base" => {
                o.base = Some(arg(a, i + 1, "missing mod romfs folder")?.into());
                2
            }
            "--rst-from" => {
                o.rst_from = Some(arg(a, i + 1, "missing size table file")?.into());
                2
            }
            x => return Err(E::Msg(format!("{x} is not an option of pot-variants"))),
        };
        i += step;
    }
    Ok(o)
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let o: Opts = parse(a)?;
    let d: Done = add(P::new(root), P::new(load), name, &ACTORS, &o)?;
    print!("{}", d.report);
    if o.dry {
        println!("dry run, nothing was written");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_are_read_and_unknown_ones_refused() {
        let a: Vec<String> = ["r", "l", "m", "--dry", "--host", "hookshot"].iter().map(|s| s.to_string()).collect();
        let o: Opts = parse(&a).unwrap();
        assert!(o.dry);
        assert_eq!(o.host, Host::Hookshot);
        assert!(parse(&["r".to_string(), "l".to_string(), "m".to_string(), "--what".to_string()]).is_err());
    }
}
