pub const PICKUPS: [(&str, &str); 25] = [
    ("Rupee (1)", "Obj_Rupee_Green"),
    ("Rupees (5)", "Obj_Rupee_Blue"),
    ("Rupees (20)", "Obj_Rupee_Red"),
    ("Rupees (50)", "Obj_Rupee_Purple"),
    ("Rupees (200)", "Obj_Rupee_Gold"),
    ("Recovery Heart", "Obj_Heart"),
    ("Fairy Drop", "Obj_Fairy"),
    ("Deku Nuts (5)", "Obj_DekuNut"),
    ("Deku Nuts (10)", "Obj_DekuNut"),
    ("Deku Stick (1)", "Obj_DekuStick"),
    ("Deku Seeds (30)", "Obj_DekuSeed"),
    ("Bomb (1)", "Obj_Bomb"),
    ("Bombs (5)", "Obj_Bomb"),
    ("Bombs (10)", "Obj_Bomb"),
    ("Bombs (20)", "Obj_Bomb"),
    ("Arrows (5)", "Obj_ArrowBundle_Small"),
    ("Arrows (10)", "Obj_ArrowBundle_Medium"),
    ("Arrows (30)", "Obj_ArrowBundle_Large"),
    ("Fire Arrows", "Obj_Arrow_Fire"),
    ("Goron Tunic", "Obj_Armor_Goron"),
    ("Zora Tunic", "Obj_Armor_Zora"),
    ("Deku Shield", "Obj_Shield_Deku"),
    ("Hylian Shield", "Obj_Shield_Hylian"),
    ("Heart Container", "Obj_Heart_Container"),
    ("Gold Skulltula Token", "Obj_Token_Gold"),
];

pub fn of(item: &str) -> Option<&'static str> {
    PICKUPS.iter().find(|(n, _)| *n == item).map(|(_, a)| *a)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slice::data::Data;

    #[test]
    fn every_pickup_item_is_a_real_item_and_none_is_progressive_or_dungeon_scoped() {
        let d: Data = Data::load().unwrap();
        for (n, a) in PICKUPS {
            let i = d.item(n).unwrap_or_else(|| panic!("{n} is not an item"));
            assert!(a.starts_with("Obj_"), "{n} maps to {a}");
            assert!(i.tiers.is_empty(), "{n}");
            assert!(i.dungeon.is_none(), "{n}");
            assert!(!matches!(i.kind.as_str(), "SmallKey" | "BossKey" | "Map" | "Compass" | "SilverRupee"), "{n}");
        }
    }

    #[test]
    fn unlisted_items_have_no_pickup() {
        assert_eq!(of("Hookshot"), None);
        assert_eq!(of("Nothing"), None);
        assert_eq!(of("Piece of Heart"), None);
        assert_eq!(of("Ocarina"), None);
        assert_eq!(of("Fire Arrows"), Some("Obj_Arrow_Fire"));
    }
}
