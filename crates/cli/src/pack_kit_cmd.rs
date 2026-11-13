use std::path::Path as P;

use ootrr_core::error::{Error as E, Res};
use ootrr_core::modpack::Host;
use ootrr_core::pickup_pack::kit::{default_kit, run as add};
use ootrr_core::pickup_pack::write::{Done, Opts};
use ootrr_core::pickup_pack::{Spec, Style};

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(E::Format(what))
}

pub fn parse(a: &[String]) -> Res<(Spec, Opts, Vec<u64>)> {
    let item: &str = arg(a, 3, "missing Item_Actor or Obj_Actor")?;
    let mut s: Spec = Spec { item: item.to_string(), name: default_kit(item), style: Style::Touch, heap: None };
    let mut o: Opts = Opts { host: Host::Eden, base: None, rst_from: None, places: Vec::new(), dry: false };
    let mut drops: Vec<u64> = Vec::new();
    let mut i: usize = 4;
    while i < a.len() {
        let step: usize = match a[i].as_str() {
            "--dry" => {
                o.dry = true;
                1
            }
            "--name" => {
                s.name = arg(a, i + 1, "missing kit actor name")?.to_string();
                2
            }
            "--heap" => {
                let v: &str = arg(a, i + 1, "missing heap size")?;
                s.heap = Some(v.parse().map_err(|_| E::Msg(format!("{v} is not a heap size")))?);
                2
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
            "--drop" => {
                let v: &str = arg(a, i + 1, "missing skulltula hash")?;
                drops.push(v.parse().map_err(|_| E::Msg(format!("{v} is not an actor hash")))?);
                2
            }
            "--place" | "--press" => return Err(E::Msg(format!("{} is not for a kit pickup, a kit is only dropped by a Gold Skulltula", a[i]))),
            x => return Err(E::Msg(format!("{x} is not an option of pack-kit"))),
        };
        i += step;
    }
    Ok((s, o, drops))
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let (s, o, drops): (Spec, Opts, Vec<u64>) = parse(a)?;
    let d: Done = add(P::new(root), P::new(load), name, &s, &o, &drops)?;
    print!("{}", d.report);
    println!("{} carries {} and the skulltula token kit, instance heap {}", s.name, s.item, d.heap);
    if o.dry {
        println!("dry run, nothing was written");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(x: &[&str]) -> Vec<String> {
        x.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_default_kit_name_marks_the_pickup_for_skulltulas() {
        let (s, o, drops): (Spec, Opts, Vec<u64>) = parse(&args(&["r", "l", "m", "Item_Hookshot"])).unwrap();
        assert_eq!(s.name, "Obj_Hookshot_GS");
        assert!(drops.is_empty() && o.places.is_empty() && !o.dry);
        let (s, _, _): (Spec, Opts, Vec<u64>) = parse(&args(&["r", "l", "m", "Obj_Rupee_Red"])).unwrap();
        assert_eq!((s.item.as_str(), s.name.as_str()), ("Obj_Rupee_Red", "Obj_Rupee_Red_GS"));
    }

    #[test]
    fn drops_and_the_shared_options_are_read() {
        let a: Vec<String> = args(&["r", "l", "m", "Item_Hookshot", "--drop", "2504273934093629549", "--drop", "7", "--name", "Obj_Hook_GS", "--heap", "200000", "--host", "hookshot", "--base", "x/romfs", "--rst-from", "t.zs", "--dry"]);
        let (s, o, drops): (Spec, Opts, Vec<u64>) = parse(&a).unwrap();
        assert_eq!(drops, vec![2504273934093629549, 7]);
        assert_eq!(s.name, "Obj_Hook_GS");
        assert_eq!(s.heap, Some(200000));
        assert_eq!(o.host, Host::Hookshot);
        assert!(o.dry && o.base.is_some() && o.rst_from.is_some());
    }

    #[test]
    fn a_placed_spot_the_press_style_and_bad_input_are_refused() {
        assert!(parse(&args(&["r", "l", "m"])).is_err());
        assert!(parse(&args(&["r", "l", "m", "Item_Hookshot", "--place", "7"])).is_err());
        assert!(parse(&args(&["r", "l", "m", "Item_Hookshot", "--press"])).is_err());
        assert!(parse(&args(&["r", "l", "m", "Item_Hookshot", "--drop", "abc"])).is_err());
        assert!(parse(&args(&["r", "l", "m", "Item_Hookshot", "--what"])).is_err());
    }
}
