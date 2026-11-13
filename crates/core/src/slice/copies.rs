use std::collections::BTreeSet as BS;

use crate::catalog::Loc;
use crate::slice::data::Data;

pub const REPEAT: [&str; 18] = [
    "Recovery Heart",
    "Fairy Drop",
    "Rupee (1)",
    "Rupees (5)",
    "Rupees (20)",
    "Rupees (50)",
    "Rupees (200)",
    "Deku Nuts (5)",
    "Deku Nuts (10)",
    "Deku Stick (1)",
    "Deku Seeds (30)",
    "Bomb (1)",
    "Bombs (5)",
    "Bombs (10)",
    "Bombs (20)",
    "Arrows (5)",
    "Arrows (10)",
    "Arrows (30)",
];

pub fn hashes(l: &Loc) -> usize {
    l.refs.iter().map(|r| r.hash).collect::<BS<u64>>().len()
}

pub fn copies(d: &Data, name: &str) -> usize {
    let l: Option<&Loc> = d.pots.get(name).or_else(|| d.cat.by_name(name));
    l.map_or(0, hashes)
}

pub fn fits(d: &Data, name: &str, item: &str) -> bool {
    copies(d, name) < 2 || REPEAT.contains(&item) || d.vanilla_of(name) == Some(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Ref;
    use crate::pot_slice::rules;
    use crate::settings::{Settings, Value};
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::pickups;

    #[test]
    fn a_hash_counts_once_even_when_it_sits_in_several_files() {
        let d: Data = Data::load().unwrap();
        let kak: &Loc = d.cat.by_name("Kak Impas House Freestanding PoH").unwrap();
        assert!(kak.refs.iter().map(|r| r.files.len()).sum::<usize>() >= 3);
        assert_eq!(hashes(kak), 1);
        let kf: &Loc = d.cat.by_name("KF Sarias House Recovery Heart 1").unwrap();
        assert_eq!(hashes(kf), 2);
        let mut twin: Loc = kf.clone();
        twin.refs = vec![Ref { hash: 7, files: vec!["a".to_string()] }, Ref { hash: 7, files: vec!["b".to_string()] }];
        assert_eq!(hashes(&twin), 1);
        assert_eq!(copies(&d, "no such place"), 0);
    }

    #[test]
    fn a_spot_with_two_copies_takes_only_consumables_or_its_own_item() {
        let d: Data = Data::load().unwrap();
        let kf: &str = "KF Top of Sarias Recovery Heart 1";
        assert_eq!(copies(&d, kf), 2);
        assert!(fits(&d, kf, "Recovery Heart"));
        assert!(fits(&d, kf, "Rupees (50)"));
        assert!(!fits(&d, kf, "Piece of Heart"));
        assert!(!fits(&d, kf, "Heart Container"));
        assert!(!fits(&d, kf, "Gold Skulltula Token"));
        let one: &str = "LLR Freestanding PoH";
        assert_eq!(copies(&d, one), 1);
        assert!(fits(&d, one, "Piece of Heart"));
        assert!(fits(&d, one, "Rupee (1)"));
        assert_eq!(pickups::at(&d, kf, "Piece of Heart"), Some(false));
        assert_eq!(pickups::at(&d, kf, "Rupees (20)"), Some(true));
        assert_eq!(pickups::at(&d, one, "Piece of Heart"), Some(true));
        let pot: &str = "ZF Hidden Cave Pot 2";
        assert_eq!(copies(&d, pot), 2);
        assert_eq!(rules::at(&d, pot, "Rupees (20)"), Some(true));
        assert_eq!(rules::at(&d, pot, "Heart Container"), Some(false));
        assert_eq!(rules::at(&d, pot, "Gold Skulltula Token"), Some(false));
    }

    #[test]
    fn seeds_never_put_a_counted_item_on_a_spot_that_exists_twice() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = Settings::default();
        s.set(pickups::SETTING, Value::s("all")).unwrap();
        s.set("shuffle_pots", Value::s("all")).unwrap();
        let mut twice: usize = 0;
        for n in 0..6 {
            let g: Generated = generate_data(&d, &s, &format!("copies {n}"), &GenOpts::default()).unwrap();
            for (loc, item) in &g.seed_file.placements {
                if copies(&d, loc) < 2 {
                    continue;
                }
                twice += 1;
                assert!(fits(&d, loc, item), "{item} at {loc}");
            }
        }
        assert!(twice > 0);
    }
}
