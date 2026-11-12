use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};
use crate::placement::hash_of;
use crate::scrubs_b::data::Spot;

pub const SHOWN: &str = "Obj_Heart_Piece";
const TALK: &str = "TalkEventName";
const HIT: &str = "AttackedEventName";
const ITEM0: &str = "ScrubBusiness_ShopItemActorName0";

fn have(d: &Node, k: &str) -> String {
    d.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

pub fn point(y: &mut Byml, sp: &Spot, entry: &str) -> Res<bool> {
    let Some(list) = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()) else {
        return Ok(false);
    };
    let Some(a) = list.iter_mut().find(|a| hash_of(a) == Some(sp.hash) && a.get("Gyaml").and_then(|g| g.as_str()) == Some("Enemy_Scrub_Business")) else {
        return Ok(false);
    };
    let d: &mut Node = a.get_mut("Dynamic").ok_or_else(|| E::Msg(format!("scrub {} has no Dynamic block, the game files differ", sp.hash)))?;
    let talk: String = have(d, TALK);
    let hit: String = have(d, HIT);
    if talk != sp.talk || hit != sp.attacked {
        return Err(E::Msg(format!(
            "scrub {} in {} talks through {talk:?} and {hit:?} and not {:?} and {:?}, the game files differ",
            sp.hash, sp.file, sp.talk, sp.attacked
        )));
    }
    d.set(TALK, Node::Str(entry.to_string()));
    if !sp.attacked.is_empty() {
        d.set(HIT, Node::Str(entry.to_string()));
    }
    d.set(ITEM0, Node::Str(SHOWN.to_string()));
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scrub(h: u64, dy: Vec<(&str, &str)>) -> Node {
        Node::Map(vec![
            ("Dynamic".to_string(), Node::Map(dy.into_iter().map(|(k, v)| (k.to_string(), Node::Str(v.to_string()))).collect())),
            ("Gyaml".to_string(), Node::Str("Enemy_Scrub_Business".to_string())),
            ("Hash".to_string(), Node::U64(h)),
        ])
    }

    fn sample() -> Byml {
        let leaf: Node = Node::Map(vec![("Gyaml".to_string(), Node::Str("Enemy_Scrub_Business_Leaf".to_string())), ("Hash".to_string(), Node::U64(7))]);
        Byml {
            root: Node::Map(vec![(
                "Actors".to_string(),
                Node::Arr(vec![
                    leaf,
                    scrub(5, vec![(TALK, "Enemy_Scrub_Business_DekuSeed_01"), (ITEM0, "Shop_DekuSeedBundle_02")]),
                    scrub(6, vec![(HIT, "Enemy_Scrub_Business_Item"), (TALK, "Enemy_Scrub_Business_Item")]),
                ]),
            )]),
            version: 7,
            big: false,
        }
    }

    fn spot(h: u64, talk: &str, hit: &str) -> Spot {
        Spot { file: "Banc/A/A.bcett.byml.zs".to_string(), hash: h, talk: talk.to_string(), attacked: hit.to_string() }
    }

    fn dynamic(y: &Byml, i: usize) -> Node {
        y.root.get("Actors").unwrap().as_arr().unwrap()[i].get("Dynamic").unwrap().clone()
    }

    #[test]
    fn the_talk_and_attack_rows_and_the_shown_item_are_set() {
        let mut y: Byml = sample();
        assert!(point(&mut y, &spot(5, "Enemy_Scrub_Business_DekuSeed_01", ""), "OOTRR_Scrub_A").unwrap());
        assert!(point(&mut y, &spot(6, "Enemy_Scrub_Business_Item", "Enemy_Scrub_Business_Item"), "OOTRR_Scrub_B").unwrap());
        let a: Node = dynamic(&y, 1);
        assert_eq!(have(&a, TALK), "OOTRR_Scrub_A");
        assert!(a.get(HIT).is_none());
        assert_eq!(have(&a, ITEM0), SHOWN);
        let b: Node = dynamic(&y, 2);
        assert_eq!((have(&b, TALK), have(&b, HIT), have(&b, ITEM0)), ("OOTRR_Scrub_B".to_string(), "OOTRR_Scrub_B".to_string(), SHOWN.to_string()));
        let back: Byml = Byml::parse(&y.write().unwrap()).unwrap();
        assert_eq!(back.root, y.root);
    }

    #[test]
    fn a_missing_scrub_is_reported_and_a_different_row_is_refused() {
        let mut y: Byml = sample();
        assert!(!point(&mut y, &spot(7, "", ""), "X").unwrap());
        assert!(!point(&mut y, &spot(9, "", ""), "X").unwrap());
        assert!(point(&mut y, &spot(5, "Enemy_Scrub_Business_Arrow_01", ""), "X").is_err());
        assert!(point(&mut y, &spot(6, "Enemy_Scrub_Business_Item", ""), "X").is_err());
        assert_eq!(y, sample());
    }
}
