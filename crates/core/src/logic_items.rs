use std::collections::HashMap as HM;

use crate::error::{Error as E, Res};

pub type ItemId = usize;

pub fn escape(name: &str) -> String {
    name.chars()
        .filter_map(|c| match c {
            ' ' => Some('_'),
            '\'' | '(' | ')' | '[' | ']' | '-' => None,
            _ => Some(c),
        })
        .collect()
}

#[derive(Clone, Debug, Default)]
pub struct Items {
    pub names: Vec<String>,
    pub adv: Vec<bool>,
    pub event: Vec<bool>,
    ix: HM<String, ItemId>,
}

impl Items {
    pub fn new() -> Items {
        Items::default()
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn add(&mut self, name: &str, adv: bool, event: bool) -> Res<ItemId> {
        let k: String = escape(name);
        if self.ix.contains_key(&k) {
            return Err(E::Msg(format!("duplicate item {name}")));
        }
        let id: ItemId = self.names.len();
        self.names.push(name.to_string());
        self.adv.push(adv || event);
        self.event.push(event);
        self.ix.insert(k, id);
        Ok(id)
    }

    pub fn find(&self, name: &str) -> Option<ItemId> {
        self.ix.get(&escape(name)).copied()
    }

    pub fn find_esc(&self, esc: &str) -> Option<ItemId> {
        self.ix.get(esc).copied()
    }

    pub fn event_named(&mut self, name: &str) -> ItemId {
        if let Some(i) = self.find(name) {
            return i;
        }
        let id: ItemId = self.names.len();
        self.names.push(name.to_string());
        self.adv.push(true);
        self.event.push(true);
        self.ix.insert(escape(name), id);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_matches_reference() {
        assert_eq!(escape("Boss Key (Forest Temple)"), "Boss_Key_Forest_Temple");
        assert_eq!(escape("Din's Fire"), "Dins_Fire");
        assert_eq!(escape("Ice-Arrows"), "IceArrows");
    }

    #[test]
    fn lookup_goes_through_escaping() {
        let mut it: Items = Items::new();
        let a: ItemId = it.add("Fire Arrows", true, false).unwrap();
        assert_eq!(it.find("Fire Arrows"), Some(a));
        assert_eq!(it.find_esc("Fire_Arrows"), Some(a));
        assert_eq!(it.find("Ice Arrows"), None);
        assert!(it.add("Fire_Arrows", true, false).is_err());
    }

    #[test]
    fn events_are_created_once_and_advance() {
        let mut it: Items = Items::new();
        let a: ItemId = it.event_named("Gate Open");
        let b: ItemId = it.event_named("Gate_Open");
        assert_eq!(a, b);
        assert!(it.adv[a] && it.event[a]);
        assert_eq!(it.len(), 1);
    }
}
