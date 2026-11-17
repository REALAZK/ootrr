use crate::byml::{Byml, Node};
use crate::error::{Error, Res};

pub const DIR: &str = "Drop/DropSubTable/";
pub const FOLIAGE: &str = "Pack/FoliageDropParam.pack.zs";

pub const HEART: u32 = 1872319531;
pub const SHOTS: u32 = 1113099761;
pub const BOMB: u32 = 3237216321;
pub const NUT: u32 = 2211636230;
pub const STICK: u32 = 4192551701;
pub const NEEDED: u32 = 2121460961;
pub const AMMO: [u32; 4] = [SHOTS, BOMB, NUT, STICK];

pub const HEART_ACTORS: [&str; 1] = ["Obj_Heart"];
pub const AMMO_ACTORS: [&str; 6] = ["Obj_Arrow", "Obj_Bomb", "Obj_DekuSeed", "Obj_DekuNut", "Obj_DekuStick", "Obj_Bombchu"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cut {
    pub hearts: bool,
    pub ammo: bool,
}

impl Cut {
    pub fn any(&self) -> bool {
        self.hearts || self.ammo
    }
}

fn actor(n: &Node) -> &str {
    let s: &str = n.get("DropActorName").and_then(|x| x.as_str()).unwrap_or("");
    s.rsplit('/').next().unwrap_or("").split('.').next().unwrap_or("")
}

fn kind(n: &Node) -> u32 {
    match n.get("Type") {
        Some(Node::U32(v)) => *v,
        _ => 0,
    }
}

pub fn cut_item(n: &Node, c: Cut) -> bool {
    let a: &str = actor(n);
    let t: u32 = kind(n);
    let heart: bool = if a.is_empty() { t == HEART } else { HEART_ACTORS.contains(&a) };
    let ammo: bool = if a.is_empty() { AMMO.contains(&t) } else { AMMO_ACTORS.iter().any(|p| a.starts_with(p)) };
    (c.hearts && heart) || (c.ammo && ammo)
}

fn trim_needed(n: &mut Node) -> bool {
    if kind(n) != NEEDED {
        return false;
    }
    let Some(list) = n.get_mut("MostDepletedResource").and_then(|x| x.as_arr_mut()) else {
        return false;
    };
    let was: usize = list.len();
    list.retain(|x| !matches!(x, Node::U32(v) if AMMO.contains(v)));
    list.len() != was
}

pub fn edit(raw: &[u8], c: Cut) -> Res<Option<Vec<u8>>> {
    if !c.any() {
        return Ok(None);
    }
    let mut y: Byml = Byml::parse(raw)?;
    let Some(items) = y.root.get_mut("Items").and_then(|x| x.as_arr_mut()) else {
        return Ok(None);
    };
    let mut hit: bool = false;
    for it in items.iter_mut() {
        if cut_item(it, c) {
            if !matches!(it.get("DropProbability"), Some(Node::I32(0))) {
                it.set("DropProbability", Node::I32(0));
                hit = true;
            }
        } else if c.ammo && trim_needed(it) {
            hit = true;
        }
    }
    if !hit {
        return Ok(None);
    }
    let out: Vec<u8> = y.write()?;
    if out.len() > raw.len() {
        return Err(Error::Format("difficulty: a drop table grew"));
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(actor: &str, t: u32, p: i32, needed: &[u32]) -> Node {
        let a: String = if actor.is_empty() { String::new() } else { format!("Work/Actor/{actor}.engine__actor__ActorParam.gyml") };
        Node::Map(vec![
            ("DropActorName".to_string(), Node::Str(a)),
            ("DropProbability".to_string(), Node::I32(p)),
            ("MostDepletedResource".to_string(), Node::Arr(needed.iter().map(|x| Node::U32(*x)).collect())),
            ("Type".to_string(), Node::U32(t)),
        ])
    }

    fn table(items: Vec<Node>) -> Vec<u8> {
        Byml { root: Node::Map(vec![("Items".to_string(), Node::Arr(items))]), version: 7, big: false }.write().unwrap()
    }

    fn odds(raw: &[u8]) -> Vec<i32> {
        let y: Byml = Byml::parse(raw).unwrap();
        y.root.get("Items").unwrap().as_arr().unwrap().iter().map(|i| if let Some(Node::I32(v)) = i.get("DropProbability") { *v } else { -1 }).collect()
    }

    const RUPEE: u32 = 995674644;
    const NAMED: u32 = 126134492;
    const MAGIC: u32 = 832898412;

    #[test]
    fn hearts_and_ammo_are_cut_by_type_and_by_actor() {
        let raw: Vec<u8> = table(vec![
            item("Obj_Rupee_Green", RUPEE, 40, &[]),
            item("", HEART, 10, &[]),
            item("Obj_Heart", NAMED, 10, &[]),
            item("", SHOTS, 10, &[]),
            item("Obj_DekuSeed", NAMED, 10, &[]),
            item("Obj_ArrowBundle_Medium", 798871343, 10, &[]),
            item("", MAGIC, 10, &[]),
        ]);
        assert!(edit(&raw, Cut::default()).unwrap().is_none());
        assert_eq!(odds(&edit(&raw, Cut { hearts: true, ammo: false }).unwrap().unwrap()), vec![40, 0, 0, 10, 10, 10, 10]);
        assert_eq!(odds(&edit(&raw, Cut { hearts: false, ammo: true }).unwrap().unwrap()), vec![40, 10, 10, 0, 0, 0, 10]);
        assert_eq!(odds(&edit(&raw, Cut { hearts: true, ammo: true }).unwrap().unwrap()), vec![40, 0, 0, 0, 0, 0, 10]);
    }

    #[test]
    fn the_most_needed_drop_keeps_only_magic_without_ammo() {
        let raw: Vec<u8> = table(vec![item("", NEEDED, 10, &[MAGIC, SHOTS, NUT, BOMB])]);
        assert!(edit(&raw, Cut { hearts: true, ammo: false }).unwrap().is_none());
        let y: Byml = Byml::parse(&edit(&raw, Cut { hearts: false, ammo: true }).unwrap().unwrap()).unwrap();
        let it: &Node = &y.root.get("Items").unwrap().as_arr().unwrap()[0];
        assert_eq!(it.get("MostDepletedResource"), Some(&Node::Arr(vec![Node::U32(MAGIC)])));
        assert_eq!(it.get("DropProbability"), Some(&Node::I32(10)));
    }

    #[test]
    fn a_table_with_nothing_to_cut_is_left_alone() {
        let raw: Vec<u8> = table(vec![item("Obj_Rupee_Blue", RUPEE, 100, &[]), item("Obj_Spirit_Poe", NAMED, 100, &[]), item("", HEART, 0, &[])]);
        assert!(edit(&raw, Cut { hearts: true, ammo: true }).unwrap().is_none());
    }
}
