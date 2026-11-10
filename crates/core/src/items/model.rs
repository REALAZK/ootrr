use std::collections::{BTreeMap as BM, HashMap as HM};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Actor,
    Tiers,
    Generic,
    Bundle,
    Bottle,
    Shop,
    Logic,
    None,
}

pub const MODES: [Mode; 8] = [
    Mode::Actor,
    Mode::Tiers,
    Mode::Generic,
    Mode::Bundle,
    Mode::Bottle,
    Mode::Shop,
    Mode::Logic,
    Mode::None,
];

impl Mode {
    pub fn name(&self) -> &'static str {
        match self {
            Mode::Actor => "actor",
            Mode::Tiers => "tiers",
            Mode::Generic => "generic",
            Mode::Bundle => "bundle",
            Mode::Bottle => "bottle",
            Mode::Shop => "shop",
            Mode::Logic => "logic",
            Mode::None => "none",
        }
    }

    pub fn from_name(s: &str) -> Option<Mode> {
        MODES.into_iter().find(|m| m.name() == s)
    }

    pub fn mapped(&self) -> bool {
        !matches!(self, Mode::Logic | Mode::None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seen {
    Seen,
    Partial,
    Guess,
}

impl Seen {
    pub fn name(&self) -> &'static str {
        match self {
            Seen::Seen => "seen",
            Seen::Partial => "partial",
            Seen::Guess => "guess",
        }
    }

    pub fn from_name(s: &str) -> Option<Seen> {
        [Seen::Seen, Seen::Partial, Seen::Guess].into_iter().find(|x| x.name() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Via {
    pub chest: Option<Seen>,
    pub flow: Option<Seen>,
    pub world: Option<Seen>,
    pub shop: Option<Seen>,
}

impl Via {
    pub fn any(&self) -> bool {
        self.chest.is_some() || self.flow.is_some() || self.world.is_some() || self.shop.is_some()
    }

    pub fn text(&self) -> String {
        let mut out: Vec<String> = Vec::new();
        for (k, v) in [("chest", self.chest), ("flow", self.flow), ("world", self.world), ("shop", self.shop)] {
            if let Some(s) = v {
                out.push(format!("{k} {}", s.name()));
            }
        }
        out.join(", ")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub name: String,
    pub kind: String,
    pub adv: bool,
    pub actor: Option<String>,
    pub conf: String,
    pub dungeon: Option<String>,
    pub mode: Mode,
    pub tiers: Vec<String>,
    pub world: Vec<String>,
    pub qty: Option<i64>,
    pub via: Via,
    pub why: String,
}

impl Item {
    pub fn actors(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for a in self.actor.iter().map(|s| s.as_str()).chain(self.tiers.iter().map(|s| s.as_str())).chain(self.world.iter().map(|s| s.as_str())) {
            if !out.contains(&a) {
                out.push(a);
            }
        }
        out
    }

    pub fn own_actors(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for a in self.actor.iter().map(|s| s.as_str()).chain(self.tiers.iter().map(|s| s.as_str())) {
            if !a.starts_with("Shop_") && !out.contains(&a) {
                out.push(a);
            }
        }
        out
    }

    pub fn actor_for(&self, copy: usize) -> Option<&str> {
        match self.mode {
            Mode::Tiers => self.tiers.get(copy).map(|s| s.as_str()),
            Mode::Logic | Mode::None => None,
            _ => self.actor.as_deref(),
        }
    }

    pub fn dungeon_scoped(&self) -> bool {
        self.mode == Mode::Generic
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Counts {
    pub total: usize,
    pub mapped: usize,
    pub logic: usize,
    pub unmapped: usize,
    pub adv: usize,
    pub adv_mapped: usize,
    pub adv_logic: usize,
    pub adv_unmapped: usize,
    pub by_mode: BM<&'static str, usize>,
    pub by_conf: BM<String, usize>,
    pub chest: BM<&'static str, usize>,
}

#[derive(Debug, Clone)]
pub struct Table {
    pub items: Vec<Item>,
    pub ix: HM<String, usize>,
}

impl Table {
    pub fn new(items: Vec<Item>) -> Table {
        let ix: HM<String, usize> = items.iter().enumerate().map(|(i, it)| (it.name.clone(), i)).collect();
        Table { items, ix }
    }

    pub fn get(&self, name: &str) -> Option<&Item> {
        self.ix.get(name).map(|i| &self.items[*i])
    }

    pub fn by_actor(&self, actor: &str) -> Vec<&Item> {
        self.items.iter().filter(|it| it.actors().contains(&actor)).collect()
    }

    pub fn chest_actor(&self, name: &str, copy: usize) -> Option<&str> {
        let it: &Item = self.get(name)?;
        it.via.chest?;
        it.actor_for(copy)
    }

    pub fn generic(&self, actor: &str, dungeon: &str) -> Option<&Item> {
        self.items.iter().find(|it| {
            it.mode == Mode::Generic && it.actor.as_deref() == Some(actor) && it.dungeon.as_deref() == Some(dungeon)
        })
    }

    pub fn counts(&self) -> Counts {
        let mut c: Counts = Counts { total: self.items.len(), ..Counts::default() };
        for it in &self.items {
            *c.by_mode.entry(it.mode.name()).or_insert(0) += 1;
            if it.mode.mapped() {
                c.mapped += 1;
                *c.by_conf.entry(it.conf.clone()).or_insert(0) += 1;
            } else if it.mode == Mode::Logic {
                c.logic += 1;
            } else {
                c.unmapped += 1;
            }
            if it.adv {
                c.adv += 1;
                if it.mode.mapped() {
                    c.adv_mapped += 1;
                } else if it.mode == Mode::Logic {
                    c.adv_logic += 1;
                } else {
                    c.adv_unmapped += 1;
                }
            }
            if let Some(s) = it.via.chest {
                *c.chest.entry(s.name()).or_insert(0) += 1;
            }
        }
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, mode: Mode, actor: Option<&str>, tiers: &[&str]) -> Item {
        Item {
            name: name.to_string(),
            kind: "Item".to_string(),
            adv: true,
            actor: actor.map(|s| s.to_string()),
            conf: if actor.is_some() { "exact".to_string() } else { String::new() },
            dungeon: None,
            mode,
            tiers: tiers.iter().map(|s| s.to_string()).collect(),
            world: Vec::new(),
            qty: None,
            via: Via::default(),
            why: String::new(),
        }
    }

    #[test]
    fn mode_and_seen_names_round_trip() {
        for m in MODES {
            assert_eq!(Mode::from_name(m.name()), Some(m));
        }
        for s in [Seen::Seen, Seen::Partial, Seen::Guess] {
            assert_eq!(Seen::from_name(s.name()), Some(s));
        }
        assert_eq!(Mode::from_name("x"), None);
        assert_eq!(Seen::from_name("x"), None);
    }

    #[test]
    fn tier_copies_pick_the_actor_in_order() {
        let it: Item = item("Hook", Mode::Tiers, Some("A_1"), &["A_1", "A_2"]);
        assert_eq!(it.actor_for(0), Some("A_1"));
        assert_eq!(it.actor_for(1), Some("A_2"));
        assert_eq!(it.actor_for(2), None);
        let one: Item = item("One", Mode::Actor, Some("B"), &[]);
        assert_eq!(one.actor_for(5), Some("B"));
        let none: Item = item("Nope", Mode::None, None, &[]);
        assert_eq!(none.actor_for(0), None);
    }

    #[test]
    fn lookups_and_counts_follow_the_modes() {
        let mut gk: Item = item("Key (X)", Mode::Generic, Some("K"), &[]);
        gk.dungeon = Some("X".to_string());
        gk.via.chest = Some(Seen::Seen);
        let mut shop: Item = item("Buy", Mode::Shop, Some("Shop_A"), &[]);
        shop.adv = false;
        let t: Table = Table::new(vec![
            gk,
            shop,
            item("Event", Mode::Logic, None, &[]),
            item("Gone", Mode::None, None, &[]),
        ]);
        assert!(t.get("Gone").is_some());
        assert!(t.get("Absent").is_none());
        assert_eq!(t.generic("K", "X").map(|i| i.name.as_str()), Some("Key (X)"));
        assert!(t.generic("K", "Y").is_none());
        assert_eq!(t.chest_actor("Key (X)", 3), Some("K"));
        assert_eq!(t.chest_actor("Buy", 0), None);
        assert_eq!(t.by_actor("Shop_A").len(), 1);
        let c: Counts = t.counts();
        assert_eq!((c.total, c.mapped, c.logic, c.unmapped), (4, 2, 1, 1));
        assert_eq!((c.adv, c.adv_mapped, c.adv_logic, c.adv_unmapped), (3, 1, 1, 1));
        assert_eq!(c.chest.get("seen"), Some(&1));
        assert_eq!(c.by_mode.get("generic"), Some(&1));
    }

    #[test]
    fn own_actors_skip_shop_boards_and_repeats() {
        let mut it: Item = item("Mask", Mode::Actor, Some("Item_M"), &[]);
        it.world = vec!["Shop_M".to_string(), "Item_M".to_string(), "Obj_M".to_string()];
        assert_eq!(it.own_actors(), vec!["Item_M"]);
        assert_eq!(it.actors(), vec!["Item_M", "Shop_M", "Obj_M"]);
    }
}
