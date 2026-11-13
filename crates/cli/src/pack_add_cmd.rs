use std::path::Path as P;

use ootrr_core::error::{Error as E, Res};
use ootrr_core::modpack::Host;
use ootrr_core::pickup_pack::write::{run_with as add, Done, Opts};
use ootrr_core::pickup_pack::{default_name, Layers, Spec, Style};

fn arg<'a>(a: &'a [String], i: usize, what: &'static str) -> Res<&'a str> {
    a.get(i).map(|s| s.as_str()).ok_or(E::Format(what))
}

pub fn parse(a: &[String]) -> Res<(Spec, Opts, Layers)> {
    let item: &str = arg(a, 3, "missing Item_Actor")?;
    let mut s: Spec = Spec { item: item.to_string(), name: default_name(item), style: Style::Touch, heap: None };
    let mut o: Opts = Opts { host: Host::Eden, base: None, rst_from: None, places: Vec::new(), dry: false };
    let mut l: Layers = Layers::default();
    let mut i: usize = 4;
    while i < a.len() {
        let step: usize = match a[i].as_str() {
            "--press" => {
                s.style = Style::Press;
                1
            }
            "--dry" => {
                o.dry = true;
                1
            }
            "--ammo" => {
                l.ammo = true;
                1
            }
            "--magic" => {
                l.magic = true;
                1
            }
            "--tag" => {
                let v: &str = arg(a, i + 1, "missing tag name")?;
                if !l.tags.iter().any(|t| t == v) {
                    l.tags.push(v.to_string());
                }
                2
            }
            "--name" => {
                s.name = arg(a, i + 1, "missing pickup actor name")?.to_string();
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
            "--place" => {
                let v: &str = arg(a, i + 1, "missing actor hash")?;
                o.places.push(v.parse().map_err(|_| E::Msg(format!("{v} is not an actor hash")))?);
                2
            }
            x => return Err(E::Msg(format!("{x} is not an option of pack-add"))),
        };
        i += step;
    }
    Ok((s, o, l))
}

pub fn run(a: &[String]) -> Res<()> {
    let root: &str = arg(a, 0, "missing romfs_root")?;
    let load: &str = arg(a, 1, "missing load_dir")?;
    let name: &str = arg(a, 2, "missing mod_name")?;
    let (s, o, l): (Spec, Opts, Layers) = parse(a)?;
    let d: Done = add(P::new(root), P::new(load), name, &s, &o, &l)?;
    print!("{}", d.report);
    println!("{} carries {} with instance heap {}", s.name, s.item, d.heap);
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
    fn the_defaults_are_a_touch_pickup_named_after_the_item() {
        let (s, o, l): (Spec, Opts, Layers) = parse(&args(&["r", "l", "m", "Item_Hookshot"])).unwrap();
        assert_eq!(s, Spec { item: "Item_Hookshot".to_string(), name: "Obj_Hookshot".to_string(), style: Style::Touch, heap: None });
        assert_eq!(l, Layers::default());
        assert!(l.plain());
        assert_eq!(o.host, Host::Eden);
        assert!(o.places.is_empty() && !o.dry && o.base.is_none());
    }

    #[test]
    fn every_option_is_read() {
        let a: Vec<String> = args(&[
            "r", "l", "m", "Item_Hookshot", "--press", "--name", "Obj_Hook", "--heap", "200000", "--host", "hookshot", "--place", "11219803099333988363", "--place", "7", "--base", "x/romfs", "--rst-from", "t.zs", "--dry",
        ]);
        let (s, o, l): (Spec, Opts, Layers) = parse(&a).unwrap();
        assert!(l.plain());
        assert_eq!(s.name, "Obj_Hook");
        assert_eq!(s.style, Style::Press);
        assert_eq!(s.heap, Some(200000));
        assert_eq!(o.host, Host::Hookshot);
        assert_eq!(o.places, vec![11219803099333988363, 7]);
        assert!(o.dry && o.base.is_some() && o.rst_from.is_some());
    }

    #[test]
    fn bad_input_is_refused() {
        assert!(parse(&args(&["r", "l", "m"])).is_err());
        assert!(parse(&args(&["r", "l", "m", "Item_Hookshot", "--place", "abc"])).is_err());
        assert!(parse(&args(&["r", "l", "m", "Item_Hookshot", "--host", "wii"])).is_err());
        assert!(parse(&args(&["r", "l", "m", "Item_Hookshot", "--what"])).is_err());
        assert!(parse(&args(&["r", "l", "m", "Item_Hookshot", "--tag"])).is_err());
    }

    #[test]
    fn the_layer_options_are_read_and_a_tag_is_kept_once() {
        let a: Vec<String> = args(&["r", "l", "m", "Item_Bottle_Letter", "--name", "Obj_Bottle_LetterLand", "--tag", "LetterBottle", "--tag", "LetterBottle"]);
        let (s, _, l): (Spec, Opts, Layers) = parse(&a).unwrap();
        assert_eq!(s.name, "Obj_Bottle_LetterLand");
        assert_eq!(l, Layers { tags: vec!["LetterBottle".to_string()], ammo: false, magic: false });
        let (_, _, l): (Spec, Opts, Layers) = parse(&args(&["r", "l", "m", "Item_Range_Bow", "--ammo"])).unwrap();
        assert_eq!(l, Layers { tags: Vec::new(), ammo: true, magic: false });
        let (_, _, l): (Spec, Opts, Layers) = parse(&args(&["r", "l", "m", "Item_FairyBless_MP", "--magic"])).unwrap();
        assert_eq!(l, Layers { tags: Vec::new(), ammo: false, magic: true });
    }
}
