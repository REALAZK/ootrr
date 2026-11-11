use crate::ainb::{start, Ainb};
use crate::error::Res;
use crate::rng::Rng;
use crate::romfs::Romfs;
use crate::staging::Out;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reward {
    Off,
    Stone,
    Medal,
    Any,
}

pub const STONES: [(&str, &str); 3] = [
    ("Kokiri Emerald", "Item_Stone_Forest"),
    ("Goron Ruby", "Item_Stone_Fire"),
    ("Zora Sapphire", "Item_Stone_Water"),
];

pub const MEDALS: [(&str, &str); 6] = [
    ("Forest Medallion", "Item_Medal_Forest"),
    ("Fire Medallion", "Item_Medal_Fire"),
    ("Water Medallion", "Item_Medal_Water"),
    ("Shadow Medallion", "Item_Medal_Shadow"),
    ("Spirit Medallion", "Item_Medal_Spirit"),
    ("Light Medallion", "Item_Medal_Light"),
];

pub fn pick(seed: &str, kind: Reward) -> Option<(&'static str, &'static str)> {
    let pool: Vec<(&'static str, &'static str)> = match kind {
        Reward::Off => return None,
        Reward::Stone => STONES.to_vec(),
        Reward::Medal => MEDALS.to_vec(),
        Reward::Any => STONES.iter().chain(MEDALS.iter()).copied().collect(),
    };
    let mut r: Rng = Rng::from_text(&format!("{seed}/start reward"));
    Some(pool[r.below(pool.len() as u64) as usize])
}

pub fn give(raw: &[u8], actor: &str) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    start::patch(&mut a, None, &[(actor.to_string(), 1)])?;
    a.write()
}

pub fn stage(rf: &mut Romfs, actor: &str) -> Res<Out> {
    let raw: Vec<u8> = rf.read(start::MODULE)?;
    let out: Vec<u8> = give(&raw, actor)?;
    Ok(Out { path: start::MODULE.to_string(), old: raw.len(), new: out.len(), z: out })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pick_is_deterministic_and_stays_in_its_group() {
        for n in 0..20 {
            let s: String = format!("seed {n}");
            assert_eq!(pick(&s, Reward::Stone), pick(&s, Reward::Stone));
            assert!(STONES.contains(&pick(&s, Reward::Stone).unwrap()));
            assert!(MEDALS.contains(&pick(&s, Reward::Medal).unwrap()));
        }
        assert!(pick("x", Reward::Off).is_none());
        let any: Vec<&str> = (0..60).filter_map(|n| pick(&format!("a{n}"), Reward::Any)).map(|x| x.0).collect();
        assert!(any.iter().any(|n| n.contains("Medallion")) && any.iter().any(|n| !n.contains("Medallion")));
    }

    #[test]
    fn the_actors_match_the_item_list() {
        let d = crate::slice::data::Data::load().unwrap();
        for (n, a) in STONES.iter().chain(MEDALS.iter()) {
            assert_eq!(d.item(n).and_then(|i| i.actor.as_deref()), Some(*a), "{n}");
        }
    }
}
