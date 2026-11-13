#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strip {
    Unique,
    Family,
    Counted,
    Keep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recipe {
    Pushed,
    Grant(&'static [&'static str]),
    Bag(Bag),
    Strength,
    Scale,
    Wallet,
    Magic,
    Ocarina,
    Bottles,
    Count(&'static str, i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bag {
    pub tool: &'static str,
    pub tiers: [&'static str; 3],
    pub ammo: &'static str,
    pub max: &'static str,
    pub kind: &'static str,
    pub caps: [i32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Group {
    pub id: &'static str,
    pub logic: &'static str,
    pub offset: u32,
    pub strip: Strip,
    pub recipe: Recipe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opt {
    pub id: &'static str,
    pub group: &'static str,
    pub tier: u32,
}

pub const BOW: Bag = Bag {
    tool: "Weapon_Range_Bow",
    tiers: ["Item_Quiver_Small", "Item_Quiver_Medium", "Item_Quiver_Large"],
    ammo: "Item_Arrow_Normal",
    max: "0x418db791",
    kind: "Arrow",
    caps: [30, 40, 50],
};

pub const SLING: Bag = Bag {
    tool: "",
    tiers: ["Item_BulletBag_Small", "Item_BulletBag_Medium", "Item_BulletBag_Large"],
    ammo: "Item_DekuSeed",
    max: "0xae8cc742",
    kind: "DekuSeed",
    caps: [30, 40, 50],
};

pub const BOMBS: Bag = Bag {
    tool: "",
    tiers: ["Item_BombBag_Small", "Item_BombBag_Medium", "Item_BombBag_Large"],
    ammo: "Item_Bomb",
    max: "0xe753508b",
    kind: "Bomb",
    caps: [20, 30, 40],
};

pub const STICKS: Bag = Bag {
    tool: "",
    tiers: ["Item_StickBag_Small", "Item_StickBag_Medium", "Item_StickBag_Large"],
    ammo: "Item_DekuStick",
    max: "0x457616b0",
    kind: "DekuStick",
    caps: [10, 20, 30],
};

pub const NUTS: Bag = Bag {
    tool: "",
    tiers: ["Item_NutBag_Small", "Item_NutBag_Medium", "Item_NutBag_Large"],
    ammo: "Item_DekuNut",
    max: "0x735b00f7",
    kind: "DekuNut",
    caps: [20, 30, 40],
};

pub const GROUPS: &[Group] = &[
    Group { id: "kokiri_sword", logic: "Kokiri Sword", offset: 0, strip: Strip::Unique, recipe: Recipe::Pushed },
    Group { id: "deku_shield", logic: "Deku Shield", offset: 0, strip: Strip::Unique, recipe: Recipe::Pushed },
    Group { id: "slingshot", logic: "Slingshot", offset: 0, strip: Strip::Family, recipe: Recipe::Bag(SLING) },
    Group { id: "bow", logic: "Bow", offset: 0, strip: Strip::Family, recipe: Recipe::Bag(BOW) },
    Group { id: "bombs", logic: "Bomb Bag", offset: 0, strip: Strip::Family, recipe: Recipe::Bag(BOMBS) },
    Group { id: "sticks", logic: "Deku Stick Capacity", offset: 1, strip: Strip::Family, recipe: Recipe::Bag(STICKS) },
    Group { id: "nuts", logic: "Deku Nut Capacity", offset: 1, strip: Strip::Family, recipe: Recipe::Bag(NUTS) },
    Group { id: "hookshot", logic: "Progressive Hookshot", offset: 0, strip: Strip::Family, recipe: Recipe::Grant(&["Item_Hookshot", "Item_Longshot"]) },
    Group { id: "strength", logic: "Progressive Strength Upgrade", offset: 0, strip: Strip::Family, recipe: Recipe::Strength },
    Group { id: "scale", logic: "Progressive Scale", offset: 0, strip: Strip::Family, recipe: Recipe::Scale },
    Group { id: "wallet", logic: "Progressive Wallet", offset: 0, strip: Strip::Family, recipe: Recipe::Wallet },
    Group { id: "magic", logic: "Magic Meter", offset: 0, strip: Strip::Family, recipe: Recipe::Magic },
    Group { id: "ocarina", logic: "Ocarina", offset: 0, strip: Strip::Family, recipe: Recipe::Ocarina },
    Group { id: "bottle", logic: "Bottle", offset: 0, strip: Strip::Counted, recipe: Recipe::Bottles },
    Group { id: "boomerang", logic: "Boomerang", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Item_Boomerang"]) },
    Group { id: "megaton_hammer", logic: "Megaton Hammer", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Item_Hammer"]) },
    Group { id: "lens_of_truth", logic: "Lens of Truth", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Armor_GlassTruth"]) },
    Group { id: "dins_fire", logic: "Dins Fire", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Item_DinFire"]) },
    Group { id: "farores_wind", logic: "Farores Wind", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Item_FaroreWind"]) },
    Group { id: "nayrus_love", logic: "Nayrus Love", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Item_NayruLove"]) },
    Group { id: "fire_arrows", logic: "Fire Arrows", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Item_Arrow_Fire"]) },
    Group { id: "ice_arrows", logic: "Ice Arrows", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Item_Arrow_Ice"]) },
    Group { id: "light_arrows", logic: "Light Arrows", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Item_Arrow_Light"]) },
    Group { id: "iron_boots", logic: "Iron Boots", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Armor_Adult_HeavyBoot"]) },
    Group { id: "hover_boots", logic: "Hover Boots", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Armor_Adult_HoverBoot"]) },
    Group { id: "goron_tunic", logic: "Goron Tunic", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Armor_Adult_Goron"]) },
    Group { id: "zora_tunic", logic: "Zora Tunic", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Armor_Adult_Zora"]) },
    Group { id: "hylian_shield", logic: "Hylian Shield", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Weapon_Shield_Hylian"]) },
    Group { id: "mirror_shield", logic: "Mirror Shield", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Weapon_Shield_Mirror"]) },
    Group { id: "stone_of_agony", logic: "Stone of Agony", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Item_AgonyStone"]) },
    Group { id: "gerudo_card", logic: "Gerudo Membership Card", offset: 0, strip: Strip::Unique, recipe: Recipe::Grant(&["Item_Token_Gerudo"]) },
    Group { id: "magic_beans", logic: "Magic Bean", offset: 0, strip: Strip::Keep, recipe: Recipe::Count("Item_MagicBean", 10) },
    Group { id: "bombchus", logic: "Bombchus", offset: 0, strip: Strip::Keep, recipe: Recipe::Count("Item_Bombchu", 20) },
];

pub const OPTS: &[Opt] = &[
    Opt { id: "kokiri_sword", group: "kokiri_sword", tier: 1 },
    Opt { id: "deku_shield", group: "deku_shield", tier: 1 },
    Opt { id: "slingshot", group: "slingshot", tier: 1 },
    Opt { id: "big_bullet_bag", group: "slingshot", tier: 2 },
    Opt { id: "biggest_bullet_bag", group: "slingshot", tier: 3 },
    Opt { id: "bow", group: "bow", tier: 1 },
    Opt { id: "big_quiver", group: "bow", tier: 2 },
    Opt { id: "biggest_quiver", group: "bow", tier: 3 },
    Opt { id: "bomb_bag", group: "bombs", tier: 1 },
    Opt { id: "big_bomb_bag", group: "bombs", tier: 2 },
    Opt { id: "biggest_bomb_bag", group: "bombs", tier: 3 },
    Opt { id: "deku_sticks", group: "sticks", tier: 1 },
    Opt { id: "big_stick_bag", group: "sticks", tier: 2 },
    Opt { id: "biggest_stick_bag", group: "sticks", tier: 3 },
    Opt { id: "deku_nuts", group: "nuts", tier: 1 },
    Opt { id: "big_nut_bag", group: "nuts", tier: 2 },
    Opt { id: "biggest_nut_bag", group: "nuts", tier: 3 },
    Opt { id: "hookshot", group: "hookshot", tier: 1 },
    Opt { id: "longshot", group: "hookshot", tier: 2 },
    Opt { id: "goron_bracelet", group: "strength", tier: 1 },
    Opt { id: "silver_gauntlets", group: "strength", tier: 2 },
    Opt { id: "golden_gauntlets", group: "strength", tier: 3 },
    Opt { id: "silver_scale", group: "scale", tier: 1 },
    Opt { id: "golden_scale", group: "scale", tier: 2 },
    Opt { id: "adult_wallet", group: "wallet", tier: 1 },
    Opt { id: "giant_wallet", group: "wallet", tier: 2 },
    Opt { id: "magic_meter", group: "magic", tier: 1 },
    Opt { id: "double_magic", group: "magic", tier: 2 },
    Opt { id: "ocarina", group: "ocarina", tier: 1 },
    Opt { id: "ocarina_of_time", group: "ocarina", tier: 2 },
    Opt { id: "bottle", group: "bottle", tier: 1 },
    Opt { id: "two_bottles", group: "bottle", tier: 2 },
    Opt { id: "three_bottles", group: "bottle", tier: 3 },
    Opt { id: "four_bottles", group: "bottle", tier: 4 },
    Opt { id: "boomerang", group: "boomerang", tier: 1 },
    Opt { id: "megaton_hammer", group: "megaton_hammer", tier: 1 },
    Opt { id: "lens_of_truth", group: "lens_of_truth", tier: 1 },
    Opt { id: "dins_fire", group: "dins_fire", tier: 1 },
    Opt { id: "farores_wind", group: "farores_wind", tier: 1 },
    Opt { id: "nayrus_love", group: "nayrus_love", tier: 1 },
    Opt { id: "fire_arrows", group: "fire_arrows", tier: 1 },
    Opt { id: "ice_arrows", group: "ice_arrows", tier: 1 },
    Opt { id: "light_arrows", group: "light_arrows", tier: 1 },
    Opt { id: "iron_boots", group: "iron_boots", tier: 1 },
    Opt { id: "hover_boots", group: "hover_boots", tier: 1 },
    Opt { id: "goron_tunic", group: "goron_tunic", tier: 1 },
    Opt { id: "zora_tunic", group: "zora_tunic", tier: 1 },
    Opt { id: "hylian_shield", group: "hylian_shield", tier: 1 },
    Opt { id: "mirror_shield", group: "mirror_shield", tier: 1 },
    Opt { id: "stone_of_agony", group: "stone_of_agony", tier: 1 },
    Opt { id: "gerudo_card", group: "gerudo_card", tier: 1 },
    Opt { id: "magic_beans", group: "magic_beans", tier: 1 },
    Opt { id: "bombchus", group: "bombchus", tier: 1 },
];

pub const REFUSED: &[(&str, &str)] = &[(
    "zeldas_letter",
    "Zelda's Letter shares the child trade slot in the remake, so the Weird Egg or the next child trade item would take its place and the logic would lose it. Use open_kakariko instead",
)];

pub const SONGS: &[(&str, &str, &str)] = &[
    ("zeldas_lullaby", "Zeldas Lullaby", "IsGet.Item_Melody_ZeldaLullaby"),
    ("eponas_song", "Eponas Song", "IsGet.Item_Melody_EponaSong"),
    ("sarias_song", "Sarias Song", "IsGet.Item_Melody_SariaSong"),
    ("suns_song", "Suns Song", "IsGet.Item_Melody_SunSong"),
    ("song_of_time", "Song of Time", "IsGet.Item_Melody_TimeSong"),
    ("song_of_storms", "Song of Storms", "IsGet.Item_Melody_StormSong"),
    ("minuet_of_forest", "Minuet of Forest", "IsGet.Item_Melody_ForestMinuet"),
    ("bolero_of_fire", "Bolero of Fire", "IsGet.Item_Melody_FireBolero"),
    ("serenade_of_water", "Serenade of Water", "IsGet.Item_Melody_WaterSerenade"),
    ("requiem_of_spirit", "Requiem of Spirit", "IsGet.Item_Melody_SpiritRequiem"),
    ("nocturne_of_shadow", "Nocturne of Shadow", "IsGet.Item_Melody_ShadowNocturne"),
    ("prelude_of_light", "Prelude of Light", "IsGet.Item_Melody_LightPrelude"),
];

pub fn group(id: &str) -> Option<&'static Group> {
    GROUPS.iter().find(|g| g.id == id)
}

pub fn opt(id: &str) -> Option<&'static Opt> {
    OPTS.iter().find(|o| o.id == id)
}

pub fn refused(id: &str) -> Option<&'static str> {
    REFUSED.iter().find(|r| r.0 == id).map(|r| r.1)
}

pub fn song(id: &str) -> Option<&'static (&'static str, &'static str, &'static str)> {
    SONGS.iter().find(|s| s.0 == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_option_names_a_group_and_tiers_start_at_one() {
        for o in OPTS {
            let g: &Group = group(o.group).unwrap_or_else(|| panic!("{}", o.id));
            assert!(o.tier >= 1, "{}", o.id);
            assert!(OPTS.iter().filter(|x| x.group == g.id).any(|x| x.tier == 1), "{}", g.id);
        }
        for g in GROUPS {
            assert!(OPTS.iter().any(|o| o.group == g.id), "{}", g.id);
        }
        let mut ids: Vec<&str> = OPTS.iter().map(|o| o.id).chain(REFUSED.iter().map(|r| r.0)).collect();
        let n: usize = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }

    #[test]
    fn every_setting_choice_is_either_given_or_refused() {
        let s: crate::settings::Schema = crate::settings::list::build();
        let st: &crate::settings::Setting = s.get("starting_items").unwrap();
        for c in &st.choices {
            assert!(opt(&c.id).is_some() != refused(&c.id).is_some(), "{}", c.id);
        }
        assert_eq!(st.choices.len(), OPTS.len() + REFUSED.len());
        let sg: &crate::settings::Setting = s.get("starting_songs").unwrap();
        for c in &sg.choices {
            assert!(song(&c.id).is_some(), "{}", c.id);
        }
        assert_eq!(sg.choices.len(), SONGS.len());
    }
}
