use std::collections::{BTreeMap as BM, BTreeSet as BS};

use crate::progressive::{needs_every_holder, Grant, FAMILIES};
use crate::settings::seed::SeedFile;
use crate::slice::data::Data;
use crate::slice::edit::is_sale;

pub fn wraps(d: &Data, seed: &SeedFile) -> BM<String, &'static str> {
    let mut out: BM<String, &'static str> = BM::new();
    for (loc, item) in &seed.placements {
        if !is_sale(d, loc) || !d.flow_ok(loc) || crate::scrubs_a::home(d, loc, item) {
            continue;
        }
        let Some(f) = d.family(item) else {
            continue;
        };
        let covered: bool = d.holders(f).iter().all(|h| seed.placements.contains_key(*h) || d.wrappable(h));
        if needs_every_holder(f) && !covered {
            continue;
        }
        out.insert(loc.clone(), f);
    }
    out
}

pub fn grants(d: &Data, seed: &SeedFile, chest_families: &BS<&str>, placed: &BM<String, &'static str>) -> Vec<Grant> {
    let mut fams: BS<&str> = chest_families.clone();
    fams.extend(placed.values().copied());
    let mut out: Vec<Grant> = Vec::new();
    for f in FAMILIES.iter().filter(|x| fams.contains(x.0)) {
        out.extend(d.grants_except(f.0, &|n| seed.placements.contains_key(n)));
    }
    for (loc, f) in placed {
        let Some(l) = d.flow.by_name(loc) else {
            continue;
        };
        for e in &l.edits {
            let files: Vec<&String> = std::iter::once(&e.file).chain(e.also.iter()).collect();
            for file in files {
                out.push(Grant { file: file.clone(), index: e.index, event: e.event.clone(), param: e.param.clone(), family: f });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_with(d: &Data, pairs: &[(&str, &str)]) -> SeedFile {
        let mut map: BM<String, String> = BM::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v.to_string());
        }
        SeedFile::new("t", &crate::settings::Settings::default(), &d.scan.game, map).unwrap()
    }

    #[test]
    fn a_family_item_on_a_flow_location_becomes_a_wrap_and_the_vanilla_holder_is_skipped() {
        let d: Data = Data::load().unwrap();
        let s: SeedFile = seed_with(&d, &[("Kak 50 Gold Skulltula Reward", "Deku Stick Capacity"), ("Kak 10 Gold Skulltula Reward", "Piece of Heart")]);
        let w: BM<String, &'static str> = wraps(&d, &s);
        assert_eq!(w.get("Kak 50 Gold Skulltula Reward"), Some(&"StickBag"));
        assert_eq!(w.len(), 1);
        let fams: BS<&str> = BS::new();
        let g: Vec<Grant> = grants(&d, &s, &fams, &w);
        assert!(g.iter().any(|x| x.index == 249 && x.family == "StickBag"));
        let mut chest: BS<&str> = BS::new();
        chest.insert("Wallet");
        let g2: Vec<Grant> = grants(&d, &s, &chest, &w);
        assert!(g2.iter().all(|x| x.index != 245));
        assert!(g2.iter().any(|x| x.index == 247 && x.family == "Wallet"));
    }

    #[test]
    fn non_family_items_and_unsupported_locations_never_wrap() {
        let d: Data = Data::load().unwrap();
        let s: SeedFile = seed_with(&d, &[("Kak 50 Gold Skulltula Reward", "Bombchus (20)"), ("Song from Impa", "Deku Stick Capacity")]);
        assert!(wraps(&d, &s).is_empty());
    }
}
