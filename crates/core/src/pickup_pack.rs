pub mod actor;
pub mod ammo;
pub mod build;
pub mod catalog;
pub mod closure;
pub mod guard;
pub mod kit;
pub mod magic;
pub mod place;
pub mod rows;
pub mod sizes;
pub mod tag;
pub mod write;

pub const BASE: &str = "Obj_Item_Base";
pub const DONOR: &str = "Obj_Arrow_Fire";
pub const PACK_DICT: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Touch,
    Press,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spec {
    pub item: String,
    pub name: String,
    pub style: Style,
    pub heap: Option<i32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Layers {
    pub tags: Vec<String>,
    pub ammo: bool,
    pub magic: bool,
}

impl Layers {
    pub fn plain(&self) -> bool {
        self.tags.is_empty() && !self.ammo && !self.magic
    }
}

pub fn pack_of(name: &str) -> String {
    format!("Pack/Actor/{name}.pack.zs")
}

pub fn default_name(item: &str) -> String {
    match item.strip_prefix("Item_") {
        Some(t) => format!("Obj_{t}"),
        None => format!("Obj_{item}"),
    }
}

pub fn is_actor_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pickup_name_follows_the_shipped_pattern() {
        assert_eq!(default_name("Item_Hookshot"), "Obj_Hookshot");
        assert_eq!(default_name("Weapon_Shield_Mirror"), "Obj_Weapon_Shield_Mirror");
        assert_eq!(pack_of("Obj_Hookshot"), "Pack/Actor/Obj_Hookshot.pack.zs");
    }

    #[test]
    fn actor_names_cannot_carry_path_parts() {
        assert!(is_actor_name("Obj_Hookshot"));
        assert!(!is_actor_name("../Obj"));
        assert!(!is_actor_name("Obj Hookshot"));
        assert!(!is_actor_name(""));
    }

    #[test]
    fn no_layer_is_on_by_default() {
        let l: Layers = Layers::default();
        assert!(l.plain());
        assert!(!Layers { ammo: true, ..Layers::default() }.plain());
        assert!(!Layers { magic: true, ..Layers::default() }.plain());
        assert!(!Layers { tags: vec!["LetterBottle".to_string()], ..Layers::default() }.plain());
    }
}
