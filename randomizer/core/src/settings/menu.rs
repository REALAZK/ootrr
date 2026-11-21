
use std::collections::BTreeSet as Set;

use super::model::Schema;
use crate::json::Json;

pub struct Section {
    pub label: &'static str,
    pub ids: &'static [&'static str],
}

pub struct Page {
    pub id: &'static str,
    pub label: &'static str,
    pub heading: &'static str,
    pub sections: &'static [Section],
}

const fn sec(label: &'static str, ids: &'static [&'static str]) -> Section {
    Section { label, ids }
}

pub enum Widget {
    Check,
    Pick(&'static [&'static str], usize),
    Slider(i64, i64, i64),
}

pub enum Entry {
    Stub(&'static str, Widget, &'static str),
    View(&'static View),
    Plain(&'static str),
    Hidden(&'static str),
}

pub struct View {
    pub id: &'static str,
    pub label: &'static str,
    pub tip: &'static str,
    pub kind: Kind,
    pub opts: &'static [Opt],
}

pub enum Kind {
    Pick,
    Check,
    Steps(i64, i64),
    Number(i64, i64, bool),
}

pub struct Opt {
    pub label: &'static str,
    pub ours: &'static str,
    pub live: bool,
    pub needs: &'static str,
}

const fn yes(label: &'static str, ours: &'static str) -> Opt {
    Opt { label, ours, live: true, needs: "" }
}

const fn needs(label: &'static str, ours: &'static str, on: &'static str) -> Opt {
    Opt { label, ours, live: true, needs: on }
}

const fn broken(label: &'static str, ours: &'static str) -> Opt {
    Opt { label, ours, live: false, needs: "" }
}

const fn missing(label: &'static str) -> Opt {
    Opt { label, ours: "", live: false, needs: "" }
}

const fn gray_check(label: &'static str, tip: &'static str) -> Entry {
    Entry::Stub(label, Widget::Check, tip)
}

const fn gray_pick(label: &'static str, opts: &'static [&'static str], at: usize, tip: &'static str) -> Entry {
    Entry::Stub(label, Widget::Pick(opts, at), tip)
}

const fn gray_slider(label: &'static str, min: i64, max: i64, at: i64, tip: &'static str) -> Entry {
    Entry::Stub(label, Widget::Slider(min, max, at), tip)
}



const NO_LOGIC: View = View {
    id: "logic_rules",
    label: "No Logic",
    tip: "Item placement is completely random without any verification for obtainability. MAY BE IMPOSSIBLE TO BEAT.",
    kind: Kind::Check,
    opts: &[yes("Off", "glitchless"), broken("On", "no_logic")],
};

const STARTING_AGE: View = View {
    id: "starting_age",
    label: "Starting Age",
    tip: "Choose which age Link will start as.\n\nStarting as adult means you start with the Master Sword in your inventory.\nThe child option is forcefully set if it would conflict with other options.",
    kind: Kind::Pick,
    opts: &[yes("Child", "child"), broken("Adult", "adult"), broken("Random", "random")],
};

const BIG_POE: View = View {
    id: "big_poe_one",
    label: "Big Poe Target Count",
    tip: "The Poe collector will give a reward for turning in this many Big Poes.",
    kind: Kind::Steps(1, 5),
    opts: &[yes("1", "true"), yes("5", "false")],
};

const PLANT_BEANS: View = View {
    id: "plant_beans",
    label: "Skip Planting Beans",
    tip: "Beans will be planted once you find beans.\nIf bean souls are shuffled, you must still find the soul.",
    kind: Kind::Check,
    opts: &[yes("Off", "false"), yes("On", "true")],
};

const EPONA: View = View {
    id: "skip_epona_race",
    label: "Skip Epona Race",
    tip: "Epona can be summoned with Epona's Song without needing to race Ingo.",
    kind: Kind::Check,
    opts: &[yes("Don't Skip", "false"), yes("Skip", "true")],
};

const RAINBOW_BRIDGE: View = View {
    id: "bridge",
    label: "Rainbow Bridge",
    tip: RAINBOW_BRIDGE_TIP,
    kind: Kind::Pick,
    opts: &[
        yes("Vanilla", "vanilla"),
        broken("Always open", "open"),
        broken("Stones", "stones"),
        broken("Medallions", "medallions"),
        yes("Dungeon rewards", "dungeons"),
        missing("Dungeons"),
        broken("Tokens", "tokens"),
        missing("Triforce Pieces"),
        missing("Greg"),
    ],
};

const GANON_BK: View = View {
    id: "shuffle_ganon_bosskey",
    label: "Ganon's Boss Key",
    tip: "Vanilla - Ganon's Boss Key will appear in the vanilla location.\n\nOwn dungeon - Ganon's Boss Key can appear anywhere inside Ganon's Castle.\n\nStart with - Places Ganon's Boss Key in your starting inventory.\n\nAny dungeon - Ganon's Boss Key can only appear inside of any dungeon.\n\nOverworld - Ganon's Boss Key can only appear outside of dungeons.\n\nAnywhere - Ganon's Boss Key can appear anywhere in the world.\n\nTrigger - These settings put the boss key on a trigger, granting the key once the requirements are met:\n- Stones: Obtain the specified amount of Spiritual Stones.\n- Medallions: Obtain the specified amount of medallions.\n- Dungeon rewards: Obtain the specified total sum of Spiritual Stones or medallions.\n- Dungeons: Complete the specified amount of dungeons. Dungeons are considered complete after stepping into the blue warp after the boss.\n- Tokens: Obtain the specified amount of Skulltula tokens.\n- Triforce Pieces: Obtain the specified amount of Triforce Pieces.\n\nLACS - Dungeon Rewards - Zelda gives Ganon's Boss Key in the Light Arrows Cutscene, which plays once you hold the set number of Stones and Medallions together. The Light Arrows are shuffled into the item pool. Experimental, untested in the game.\n\nSame Region - Ganon's Boss Key can only appear in Ganon's Castle or a nearby area.",
    kind: Kind::Pick,
    opts: &[
        yes("Vanilla", "vanilla"),
        yes("Own Dungeon", "dungeon"),
        yes("Start With", "remove"),
        yes("Any Dungeon", "any_dungeon"),
        yes("Overworld", "overworld"),
        yes("Anywhere", "keysanity"),
        yes("LACS - Dungeon Rewards", "lacs_dungeons"),
        broken("Trigger-Stones", "stones"),
        broken("Trigger-Medallions", "medallions"),
        broken("Trigger-Rewards", "dungeons"),
        missing("Trigger-Dungeons"),
        broken("Trigger-Tokens", "tokens"),
        missing("Trigger-Triforce Pieces"),
        yes("Same Region", "regional"),
    ],
};

const FOREST: View = View {
    id: "open_forest",
    label: "Kokiri Forest",
    tip: "Whether the way out of the forest and into the Deku Tree is blocked.\n\nOpen - Nothing blocks the forest or the Deku Tree.\n\nClosed - The forest and the Deku Tree stay closed until the story opens them, and the Kakariko gate stays closed.

Open Kakariko needs Open: with the Kakariko gate open, Closed is locked out.",
    kind: Kind::Pick,
    // An open Kakariko gate needs the forest open (settings/list.rs), so the closed choice locks out.
    opts: &[yes("Open", "open"), needs("Closed", "closed", "open_kakariko!=open")],
};

const GV_BRIDGE: View = View {
    id: "gerudo_bridge_open",
    label: "Gerudo Valley Bridge",
    tip: "Vanilla - The bridge in Gerudo Valley is broken until the carpenters are rescued.\n\nRepaired - The broken bridge in Gerudo Valley is repaired from the start, so adult Link can cross without Epona or the Longshot. The carpenters in the Thieves' Hideout are not rescued, so the guards, the rescue and the Gerudo Membership Card stay as they are. Experimental, untested in the game.",
    kind: Kind::Pick,
    opts: &[yes("Vanilla", "false"), yes("Repaired", "true")],
};

const DOOR_OF_TIME: View = View {
    id: "door_of_time_condition",
    label: "Door of Time",
    tip: "What it takes to open the Door of Time. With a Closed choice, walk up to the altar in the Temple of Time and play the Song of Time once the requirement is met.\n\nClosed - Vanilla - Clear Dodongo's Cavern and Jabu-Jabu's Belly (the remake's own rule).\n\nClosed - Classic - Hold all three Spiritual Stones.\n\nClosed - Song Only - No other requirement: just the Song of Time and an Ocarina. Zelda's Escape then sets what the Ocarina of Time in the castle moat waits for.\n\nOpen - The Door of Time is already open. Zelda's Escape then sets what the Ocarina of Time in the castle moat waits for.",
    kind: Kind::Pick,
    opts: &[
        yes("Closed - Vanilla", "vanilla"),
        yes("Closed - Classic", "stones"),
        yes("Closed - Song Only", "song"),
        yes("Open", "open"),
    ],
};

const ZORAS_FOUNTAIN: View = View {
    id: "zora_fountain",
    label: "Zora's Fountain",
    tip: "Closed - King Zora obstructs the way to Zora's Fountain. Ruto's Letter must be shown as child Link in order to move him in both time periods.\n\nClosed as child - Ruto's Letter is only required to move King Zora as child Link. Zora's Fountain starts open as adult.\n\nOpen - King Zora has already mweeped out of the way in both time periods. Ruto's Letter is removed from the item pool.",
    kind: Kind::Pick,
    opts: &[yes("Closed", "closed"), broken("Closed as child", "adult"), yes("Open", "open")],
};

const JABU: View = View {
    id: "jabu_open",
    label: "Jabu-Jabu",
    tip: "Closed - A fish is required to open Jabu-Jabu's mouth.\n\nOpen - Jabu-Jabu's mouth opens without the need for a fish.",
    kind: Kind::Pick,
    opts: &[yes("Closed", "false"), yes("Open", "true")],
};

const CARPENTERS: View = View {
    id: "gerudo_fortress",
    label: "Fortress Carpenters",
    tip: "Sets the state of the carpenters captured by Gerudo in Gerudo Fortress, and with it the number of guards that spawn.\n\nNormal - All 4 carpenters are required to be saved.\n\nFast - Tested in game. Three carpenters start free. The Membership Card is vanilla and is granted when the last carpenter is rescued.\n\nOpen - Tested in game. Start with the Gerudo Membership Card and the bridge built. The Membership Card check is still in the game and randomized. It triggers when you interact with your first jail cell.\n\nOnly \"Normal\" is compatible with Gerudo Fortress Keyrings.",
    kind: Kind::Pick,
    opts: &[
        yes("Normal", "normal"),
        yes("Fast", "fast"),
        yes("Open", "open"),
    ],
};

const TRIALS: View = View {
    id: "skip_ganon_trials",
    label: "Ganon's Trials",
    tip: "Sets the number of Ganon's Trials required to dispel the barrier.\n\nSkip - No Trials are required and the barrier is already dispelled.\n\nSet Number - Select a number of trials from the slider below. Clearing that many of the six trials, any of them, dispels the barrier.",
    kind: Kind::Pick,
    opts: &[yes("Skip", "true"), yes("Set Number", "false")],
};

const TRIAL_COUNT: View = View {
    id: "trials",
    label: "Ganon's Trials Count",
    tip: "Set the number of trials required to enter Ganon's Tower. Any of the six trials count.",
    kind: Kind::Number(0, 6, true),
    opts: &[],
};

const DUNGEON_ENTRANCES: View = View {
    id: "shuffle_entrances",
    label: "Shuffle Dungeon Entrances",
    tip: "Shuffle the pool of dungeon entrances, including Bottom of the Well, Ice Cavern and Gerudo Training Ground.\n\nShuffling Ganon's Castle can be enabled separately.\n\nAdditionally, the entrances of Deku Tree, Fire Temple, Bottom of the Well and Gerudo Training Ground are opened for both child and adult.\n\n- Deku Tree will be open for adult after Mido has seen child Link with a sword and a shield.\n- Bottom of the Well will be open for adult after playing Song of Storms to the Windmill guy as child.\n- Gerudo Training Ground will be open for child after adult has paid to open the gate once.",
    kind: Kind::Pick,
    opts: &[yes("Off", "off"), yes("On", "dungeons"), missing("On + Ganon")],
};

const WIN_CONDITION_TIP: &str = "Defeat Ganon - Defeat Ganon to complete the game.\n\nAnywhere - Collect the Triforce to complete the game. It can appear anywhere in the world.\n\nTrigger - These settings put the completing the game on a trigger, playing the credits when the requirements are met:\n- Stones: Obtain the specified amount of Spiritual Stones.\n- Medallions: Obtain the specified amount of medallions.\n- Dungeon rewards: Obtain the specified total sum of Spiritual Stones or medallions.\n- Dungeons: Complete the specified amount of dungeons. Dungeons are considered complete after stepping into the blue warp after the boss.\n- Tokens: Obtain the specified amount of Skulltula tokens.\n- Triforce Pieces: Obtain the specified amount of Triforce Pieces.";

const RAINBOW_BRIDGE_TIP: &str = "Alters the requirements to open the bridge to Ganon's Castle.\n\nVanilla - Obtain the Shadow Medallion, Spirit Medallion and Light Arrows.\n\nAlways open - No requirements.\n\nStones - Obtain the specified amount of Spiritual Stones.\n\nMedallions - Obtain the specified amount of medallions.\n\nDungeon rewards - Obtain the specified total sum of Spiritual Stones or medallions.\n\nDungeons - Complete the specified amount of dungeons. Dungeons are considered complete after stepping into the blue warp after the boss.\n\nTokens - Obtain the specified amount of Skulltula tokens.\n\nGreg - Find Greg the Green Rupee";


const MAPS: View = View {
    id: "shuffle_mapcompass",
    label: "Maps/Compasses",
    tip: "Start with - You will start with Maps & Compasses from all dungeons.\n\nVanilla - Maps & Compasses will appear in their vanilla locations.\n\nOwn dungeon - Maps & Compasses can only appear in their respective dungeon.\n\nAny dungeon - Maps & Compasses can only appear inside of any dungeon.\n\nOverworld - Maps & Compasses can only appear outside of dungeons.\n\nAnywhere - Maps & Compasses can appear anywhere in the world.\n\nSame Region - Maps & Compasses can only appear in their dungeon or a nearby area.",
    kind: Kind::Pick,
    opts: &[
        yes("Start With", "startwith"),
        yes("Vanilla", "vanilla"),
        yes("Own Dungeon", "dungeon"),
        yes("Any Dungeon", "any_dungeon"),
        broken("Overworld", "overworld"),
        yes("Anywhere", "keysanity"),
        yes("Same Region", "regional"),
    ],
};

const SMALL_KEYS: View = View {
    id: "shuffle_smallkeys",
    label: "Small Key Shuffle",
    tip: "Start with - You will start with all Small Keys from all dungeons.\n\nVanilla - Small Keys will appear in their vanilla locations. You start with 3 keys in Spirit Temple MQ because the vanilla key layout is not beatable in logic.\n\nOwn dungeon - Small Keys can only appear in their respective dungeon. If Fire Temple is not a Master Quest dungeon, the door to the Boss Key chest will be unlocked.\n\nAny dungeon - Small Keys can only appear inside of any dungeon.\n\nOverworld - Small Keys can only appear outside of dungeons.\n\nAnywhere - Small Keys can appear anywhere in the world.",
    kind: Kind::Pick,
    opts: &[
        missing("Start With"),
        yes("Vanilla", "vanilla"),
        yes("Own Dungeon", "dungeon"),
        yes("Any Dungeon", "any_dungeon"),
        broken("Overworld", "overworld"),
        yes("Anywhere", "keysanity"),
    ],
};

const BOSS_KEYS: View = View {
    id: "shuffle_bosskeys",
    label: "Boss Key Shuffle",
    tip: "Start with - You will start with Boss Keys from all dungeons.\n\nVanilla - Boss Keys will appear in their vanilla locations.\n\nOwn dungeon - Boss Keys can only appear in their respective dungeon.\n\nAny dungeon - Boss Keys can only appear inside of any dungeon.\n\nOverworld - Boss Keys can only appear outside of dungeons.\n\nAnywhere - Boss Keys can appear anywhere in the world.\n\nSame Region - Boss Keys can only appear in their dungeon or a nearby area.",
    kind: Kind::Pick,
    opts: &[
        yes("Start With", "remove"),
        yes("Vanilla", "vanilla"),
        yes("Own Dungeon", "dungeon"),
        yes("Any Dungeon", "any_dungeon"),
        yes("Overworld", "overworld"),
        yes("Anywhere", "keysanity"),
        yes("Same Region", "regional"),
    ],
};

const DUNGEON_REWARDS: View = View {
    id: "shuffle_dungeon_rewards",
    label: "Dungeon Rewards",
    tip: "Where the Spiritual Stones and Medallions can be found.\n\nVanilla - Rewards are not shuffled. Each boss and Rauru give their usual Stone or Medallion.\n\nEnd of dungeons - The Stones and Medallions are shuffled only among themselves: each boss and Rauru give one random Stone or Medallion.\n\nAnywhere - The Stones and Medallions can appear anywhere, and the bosses and Rauru can give any item.",
    kind: Kind::Pick,
    opts: &[yes("Vanilla", "vanilla"), yes("End of dungeons", "dungeons"), yes("Anywhere", "anywhere")],
};

const GERUDO_KEYS: View = View {
    id: "shuffle_hideoutkeys",
    label: "Gerudo Fortress Keys",
    tip: "Vanilla - Thieves' Hideout Keys will appear in their vanilla locations.\n\nAny dungeon - Thieves' Hideout Keys can only appear inside of any dungeon.\n\nOverworld - Thieves' Hideout Keys can only appear outside of dungeons.\n\nAnywhere - Thieves' Hideout Keys can appear anywhere in the world.",
    kind: Kind::Pick,
    opts: &[yes("Vanilla", "vanilla"), broken("Any Dungeon", "any_dungeon"), broken("Overworld", "overworld"), broken("Anywhere", "keysanity")],
};

pub struct PortSection {
    pub label: &'static str,
    pub entries: &'static [Entry],
}

pub struct PortPage {
    pub id: &'static str,
    pub label: &'static str,
    pub sections: &'static [PortSection],
}

const fn port(label: &'static str, entries: &'static [Entry]) -> PortSection {
    PortSection { label, entries }
}

pub const SOH_PORT: &str = "SOH Port";

pub const PORT: &[PortPage] = &[];

pub const MOVED: &[(&str, &[PortSection])] = &[
    ("win", &[LOGIC, WIN_CONDITION]),
    ("world", &[AREA_ACCESS, port("Entrances", &[Entry::View(&DUNGEON_ENTRANCES)])]),
    ("locations", LOCATIONS_TAB),
    ("dungeon", DUNGEONS_TAB),
    ("experimental", EXPERIMENTAL_TAB),
];

/// A warning shown at the top of a page, above its settings.
pub const WARNINGS: &[(&str, &str)] = &[(
    "experimental",
    "These settings have known unexpected behavior that could lead to many seeds being unbeatable and other minor bugs.",
)];

const LOGIC: PortSection = port(
                "Logic",
                &[
                    Entry::View(&NO_LOGIC),
                    Entry::View(&STARTING_AGE),
                    gray_check(
                        "Blue Fire Arrows",
                        "Ice Arrows act like Blue Fire, making them able to melt red ice. Item placement logic will respect this option, so it might be required to use this to progress.",
                    ),
                    gray_check(
                        "Sunlight Arrows",
                        "Light Arrows can be used to light up the sun switches instead of using the Mirror Shield. Item placement logic will respect this option, so it might be required to use this to progress.",
                    ),
                    gray_check("Full Wallets", "Start with a full wallet. All wallet upgrades come filled with rupees."),
                    Entry::View(&PLANT_BEANS),
                    Entry::View(&EPONA),
                    Entry::Hidden("bombchus_in_logic"),
                    Entry::Plain("nocturne_trigger"),
                    Entry::Plain("prelude_trigger"),
                ],
            );

const WIN_CONDITION: PortSection = port(
                "Goal",
                &[
                    gray_slider(
                        "Triforce Hunt Total Pieces",
                        0,
                        100,
                        0,
                        "The number of Triforce pieces that will be placed in the world. Set to 0 to disable Triforce Hunt.\n\nTriforce Pieces can be used as a requirement for the Rainbow Bridge, Ganon's Boss Key, Ganon's Soul, or the win condition. Keep in mind seed generation can fail if more pieces are placed than there are junk items in the item pool.",
                    ),
                    Entry::Hidden("triforce_hunt"),
                    Entry::Hidden("triforce_count_per_world"),
                    Entry::Hidden("triforce_goal_per_world"),
                    gray_pick(
                        "Triforce Hunt Pieces Location",
                        &["Any Dungeon", "Overworld", "Anywhere"],
                        2,
                        "Any dungeon - Triforce pieces can only appear inside of any dungeon.\n\nOverworld - Triforce pieces can only appear outside of dungeons.\n\nAnywhere - Triforce pieces can appear anywhere in the world.",
                    ),
                    Entry::View(&GANON_BK),
                    Entry::Plain("lacs_rewards"),
                    Entry::Hidden("ganon_bosskey_stones"),
                    Entry::Hidden("ganon_bosskey_medallions"),
                    Entry::Hidden("ganon_bosskey_rewards"),
                    Entry::Hidden("ganon_bosskey_tokens"),
                    Entry::Hidden("ganon_bosskey_hearts"),
                    gray_pick(
                        "GBK Reward Options",
                        &["Standard Reward", "Greg as Reward", "Greg as Wildcard"],
                        0,
                        "Standard Rewards - Greg does not change logic, Greg does not help obtain GBK, max number of rewards on slider does not change.\n\nGreg as Reward - Greg does change logic (can be part of expected path for obtaining GBK), Greg helps obtain GBK, max number of rewards on slider increases by 1 to account for Greg.\n\nGreg as Wildcard - Greg does not change logic, Greg helps obtain GBK, max number of rewards on slider does not change.",
                    ),
                    gray_pick(
                        "Ganon's Soul",
                        &["Start With", "Any Dungeon", "Overworld", "Anywhere", "Trigger-Stones", "Trigger-Medallions", "Trigger-Rewards", "Trigger-Dungeons", "Trigger-Tokens", "Trigger-Triforce Pieces"],
                        0,
                        "",
                    ),
                    gray_pick(
                        "Win Condition",
                        &["Defeat Ganon", "Anywhere", "Trigger-Stones", "Trigger-Medallions", "Trigger-Rewards", "Trigger-Dungeons", "Trigger-Tokens", "Trigger-Triforce Pieces"],
                        0,
                        WIN_CONDITION_TIP,
                    ),
                ],
            );

const AREA_ACCESS: PortSection = port(
                "Area Access",
                &[
                    Entry::View(&FOREST),
                    Entry::Plain("open_kakariko"),
                    Entry::View(&DOOR_OF_TIME),
                    Entry::Plain("door_of_time_escape"),
                    Entry::Plain("door_of_time_song"),
                    Entry::Hidden("open_door_of_time"),
                    Entry::View(&ZORAS_FOUNTAIN),
                    gray_pick(
                        "Sleeping Waterfall",
                        &["Closed", "Open"],
                        0,
                        "Closed - Sleeping Waterfall obstructs the entrance to Zora's Domain. Zelda's Lullaby must be played in order to open it (but only once; then it stays open in both time periods).\n\nOpen - Sleeping Waterfall is always open. Link may always enter Zora's Domain.",
                    ),
                    Entry::View(&JABU),
                    gray_check("Lock Overworld Doors", "Add locks to all wooden overworld doors, requiring specific small keys to open them"),
                    Entry::View(&CARPENTERS),
                    Entry::View(&GV_BRIDGE),
                    Entry::View(&RAINBOW_BRIDGE),
                    Entry::Plain("bridge_rewards"),
                    Entry::Hidden("bridge_stones"),
                    Entry::Hidden("bridge_medallions"),
                    Entry::Hidden("bridge_tokens"),
                    Entry::Hidden("bridge_hearts"),
                    Entry::View(&TRIALS),
                    Entry::View(&TRIAL_COUNT),
                ],
            );

const DUNGEONS_TAB: &[PortSection] = &[
            port(
                "Dungeon Items",
                &[
                    Entry::View(&MAPS),
                    Entry::View(&SMALL_KEYS),
                    Entry::View(&BOSS_KEYS),
                    Entry::View(&DUNGEON_REWARDS),
                    Entry::View(&GERUDO_KEYS),
                    gray_pick(
                        "Shuffle Boss Souls",
                        &["Off", "On"],
                        0,
                        "Shuffles 8 boss souls (one for each blue warp dungeon). A boss will not appear until you collect its respective soul.",
                    ),
                ],
            ),
            port(
                "Keyrings",
                &[gray_pick(
                    "Keyrings",
                    &["Off", "Random", "Count", "Selection"],
                    0,
                    "Keyrings will replace all small keys from a particular dungeon with a single keyring that awards all keys for its associated dungeon.\n\nOff - No dungeons will have their keys replaced with keyrings.\n\nRandom - A random amount of dungeons will have their keys replaced with keyrings.\n\nCount - A specified amount of randomly selected dungeons will have their keys replaced with keyrings.\n\nSelection - Hand select which dungeons will have their keys replaced with keyrings\n(can also be left as random, in which case each one will have a 50% chance of being a keyring).\n\nSelecting keyring for dungeons will have no effect if Small Keys are set to Start With or Vanilla.\n\nThe maximum amount of Keyrings that can be selected by Random or Count is 8, plus one if Gerudo Fortress Carpenters is set to Normal and Gerudo Fortress Keys is set to anything other than Vanilla, plus one if Shuffle Chest Minigame is on.",
                )],
            ),
        ];

const FREESTANDING: View = View {
    id: "shuffle_freestanding_items",
    label: "Freestanding Items",
    tip: "Which items lying on the ground are shuffled. Pieces of Heart on the ground are always shuffled.\n\nOff - Only Pieces of Heart are shuffled.\n\nDungeons - Every freestanding item inside dungeons is shuffled.\n\nOverworld - Every freestanding item outside dungeons is shuffled.\n\nEverywhere - Every freestanding item is shuffled, including the ones only the remake has.",
    kind: Kind::Pick,
    opts: &[yes("Off", "off"), broken("Dungeons", "dungeons"), broken("Overworld", "overworld"), yes("Everywhere", "all")],
};

const SKULLTULAS: View = View {
    id: "shuffle_skulltulas",
    label: "Gold Skulltulas",
    tip: "Whether Gold Skulltulas drop random items, and their tokens join the item pool.\n\nOff - Every Gold Skulltula drops its token.\n\nDungeons - Gold Skulltulas inside dungeons drop random items.\n\nOverworld - Gold Skulltulas outside dungeons drop random items.\n\nEverywhere - Every Gold Skulltula drops a random item. The ones whose spot is still unsure only drop minor items such as rupees and hearts.",
    kind: Kind::Pick,
    opts: &[yes("Off", "off"), missing("Dungeons"), missing("Overworld"), yes("Everywhere", "matched")],
};

const COWS: View = View {
    id: "shuffle_cows",
    label: "Shuffle Cows",
    tip: "Each cow gives one extra shuffled item the first time its milk scene starts, with or without a bottle. The milk stays vanilla and stays repeatable.\n\nCovers all eight cows: Link's House, Gerudo Valley, Gerudo Fortress, the Death Mountain Trail and Hyrule Field grottos, behind Impa's house, and the two Lon Lon Ranch stable cows.",
    kind: Kind::Check,
    opts: &[yes("Off", "false"), yes("On", "true")],
};

const SCRUBS: View = View {
    id: "shuffle_scrubs_a",
    label: "Shuffle Scrubs",
    tip: "Off - Only the Hyrule Field grotto scrub (Piece of Heart) is shuffled.

One-Time Sales - Also the three scrubs that sell something once: the Death Mountain Crater scrub (Piece of Heart) and the two Lost Woods scrubs that sell the Deku Stick and Deku Nut upgrades. Prices stay the same, and the sale text still names the original item.

All - Also the two scrubs that sell ammo again and again (Lost Woods grotto Deku Seeds, Jabu Jabu's Belly Deku Nuts) become one-time sales of a random item.",
    kind: Kind::Pick,
    opts: &[yes("Off", "false"), yes("One-Time Sales", "true"), missing("All")],
};

const SCRUB_PRICES: View = View {
    id: "scrub_prices",
    label: "Scrubs Prices",
    tip: "Vanilla - The same price as the item it replaced.\nCheap Balanced - Prices will range between 0 to 95 rupees, favoring lower numbers.\nBalanced - Prices will range between 0 to 300 rupees, favoring lower numbers.\nFixed - A fixed number.\nRange - A random point between specific ranges.\nSet By Wallet - Set weights that decide the choice of each wallet, and get a random price in that range if that wallet is chosen.\n\nThe scrub always says the price it really takes. It calls its item a Mystery Item unless Scrub Hint Text (Hints page) is on.\n\nNot working yet:\n- Balanced, Range and Set By Wallet are not built.\n- Prices stop at 99, so the starting wallet can always pay.",
    kind: Kind::Pick,
    opts: &[
        yes("Vanilla", "vanilla"),
        yes("Cheap Balanced", "cheap"),
        missing("Balanced"),
        yes("Fixed", "fixed"),
        missing("Range"),
        missing("Set By Wallet"),
    ],
};

const SCRUB_FIXED: View = View {
    id: "scrub_price_fixed",
    label: "Scrubs Fixed Price",
    tip: "The price of every shuffled scrub when Scrubs Prices is Fixed.",
    kind: Kind::Number(0, 99, true),
    opts: &[],
};

const MERCHANT_PRICES: View = View {
    id: "merchant_prices",
    label: "Merchant Prices",
    tip: "Vanilla - The merchant's usual price.\nCheap Balanced - Prices will range between 0 to 95 rupees, favoring lower numbers.\nBalanced - Prices will range between 0 to 300 rupees, favoring lower numbers.\nFixed - A fixed number.\nRange - A random point between specific ranges.\nSet By Wallet - Set weights that decide the choice of each wallet, and get a random price in that range if that wallet is chosen.\n\nOnly shuffled merchants change price (see Shuffle Merchants): Medigoron, and the Bean Merchant. Medigoron shows and takes the new price. The Bean Merchant normally costs 60 once Skip Planting Beans is on.\n\nNot working yet:\n- Balanced, Range and Set By Wallet are not built.\n- Prices stop at 99, so the starting wallet can always pay.",
    kind: Kind::Pick,
    opts: &[
        yes("Vanilla", "vanilla"),
        yes("Cheap Balanced", "cheap"),
        missing("Balanced"),
        yes("Fixed", "fixed"),
        missing("Range"),
        missing("Set By Wallet"),
    ],
};

const MERCHANT_FIXED: View = View {
    id: "merchant_price_fixed",
    label: "Merchant Fixed Price",
    tip: "The price of every shuffled merchant when Merchant Prices is Fixed.",
    kind: Kind::Number(0, 99, true),
    opts: &[],
};

const FAIRY_PRICES: View = View {
    id: "fairy_prices",
    label: "Great Fairy Prices",
    tip: "Vanilla - The fairy's usual offering: 25, 50, 100, 200, 300 or 500 rupees.
Cheap Balanced - Prices will range between 0 to 95 rupees, favoring lower numbers.
Balanced - Prices will range between 0 to 300 rupees, favoring lower numbers.
Fixed - A fixed number.
Range - A random point between specific ranges.
Set By Wallet - Set weights that decide the choice of each wallet, and get a random price in that range if that wallet is chosen.

Every Great Fairy asks the new price, and her offering line names it.

Not working yet:
- Balanced, Range and Set By Wallet are not built.
- Prices stop at 99, so the starting wallet can always pay.
- Untested in the game.",
    kind: Kind::Pick,
    opts: &[
        yes("Vanilla", "vanilla"),
        yes("Cheap Balanced", "cheap"),
        missing("Balanced"),
        yes("Fixed", "fixed"),
        missing("Range"),
        missing("Set By Wallet"),
    ],
};

const FAIRY_FIXED: View = View {
    id: "fairy_price_fixed",
    label: "Great Fairy Fixed Price",
    tip: "The offering every Great Fairy asks when Great Fairy Prices is Fixed.",
    kind: Kind::Number(0, 99, true),
    opts: &[],
};

const MERCHANTS: View = View {
    id: "shuffle_merchants",
    label: "Shuffle Merchants",
    tip: "This setting governs if the Bean Salesman, Medigoron, Granny and the Carpet Salesman sell a random item.\n\nBean Merchant Only - Only the Bean Salesman will have a check.\n\nAll But Beans - Medigoron, Granny and the Carpet Salesman will have checks.\n\nAll - Apply both effects.\n\nNot working yet:\n- Granny and the Carpet Salesman are not built, so All But Beans and All only shuffle Medigoron.\n- The Bean Salesman can only be shuffled while Skip Planting Beans is on (Logic / Goal page). Until then, Bean Merchant Only and All are grayed out.",
    kind: Kind::Pick,
    opts: &[
        yes("Off", "off"),
        needs("Bean Merchant Only", "beans", "plant_beans"),
        yes("All But Beans", "no_beans"),
        needs("All", "all", "plant_beans"),
    ],
};

const SHOPSANITY: View = View {
    id: "shuffle_shops",
    label: "Shopsanity",
    tip: "Off - All shop items will be the same as vanilla.\n\n0 Items - Vanilla shop items will be shuffled among different shops.\n\n1-4 Items - Vanilla shop items will be shuffled among different shops, and each shop will contain 1-4 non-vanilla shop items (special deals).\n\nRandom - Vanilla shop items will be shuffled among different shops, and each shop will contain a random number (0-4) of non-vanilla shop items. The Kokiri Shop always gets at least one.\n\nA special deal sells its item once. It can hold anything that cannot be held twice, upgrades at the tier you must already have reached, and songs when songs are shuffled anywhere. Ammo, refills and hearts only go to the two Bazaars, the two Potion Shops and the Bombchu Shop, where one purchase closes the slot.\n\nA Goron or Zora Tunic moved out of its own shop costs a random price in the Shopsanity Prices range, and a child can buy tunics in every shop.\n\nNot working yet:\n- The Happy Mask Shop gets no special deals.\n- Untested in the game except one Kokiri Shop deal.",
    kind: Kind::Pick,
    opts: &[
        yes("Off", "off"),
        yes("0 Items", "shuffle"),
        yes("1 Item", "deals_1"),
        yes("2 Items", "deals_2"),
        yes("3 Items", "deals_3"),
        yes("4 Items", "deals_4"),
        yes("Random", "random"),
    ],
};

const SHOPSANITY_PRICES: View = View {
    id: "shopsanity_prices",
    label: "Shopsanity Prices",
    tip: "The price range of the non-vanilla shop items, named by the wallet that can pay the highest price. Prices lean to the middle of the range, in steps of 5 rupees, and vanilla shop items keep their own price.\n\nAffordable - Every item costs 10 rupees.\n\nStarting Wallet - 10 to 200 rupees.\n\nAdult Wallet - 10 to 500 rupees. Items above 200 need the Adult's Wallet in logic.\n\nGiant's Wallet - 10 to 995 rupees. Items above 500 need the Giant's Wallet in logic.\n\nThe remake's wallets hold 200, 500 and 1000 rupees, and there is no Tycoon's Wallet. Kokiri Shop items never cost more than 200, so the Kokiri Sword and other early items stay in reach.",
    kind: Kind::Pick,
    opts: &[yes("Affordable", "affordable"), yes("Starting Wallet", "starting"), yes("Adult Wallet", "adult"), yes("Giant's Wallet", "giant")],
};

const LOCATIONS_TAB: &[PortSection] = &[
    port(
        "Chests",
        &[
            Entry::Hidden("shuffle_chests"),
            Entry::Hidden("shuffle_boss_hearts"),
            Entry::Hidden("shuffle_cows_other"),
            Entry::Hidden("shuffle_rewards"),
            Entry::Hidden("shuffle_rewards_extra"),
            Entry::Hidden("shuffle_scrubs"),
            Entry::Hidden("shuffle_scrubs_b"),
        ],
    ),
    port(
        "Items",
        &[
            Entry::Plain("shuffle_songs"),
            Entry::Plain("song_harp_colors"),
            Entry::Plain("shuffle_ocarinas"),
            Entry::Plain("shuffle_kokiri_sword"),
            Entry::View(&SKULLTULAS),
            Entry::View(&FREESTANDING),
        ],
    ),
    port(
        "Shops",
        &[
            Entry::View(&SHOPSANITY),
            Entry::View(&SHOPSANITY_PRICES),
            Entry::View(&SCRUBS),
            Entry::View(&SCRUB_PRICES),
            Entry::View(&SCRUB_FIXED),
            Entry::View(&MERCHANTS),
            Entry::View(&MERCHANT_PRICES),
            Entry::View(&MERCHANT_FIXED),
            Entry::View(&FAIRY_PRICES),
            Entry::View(&FAIRY_FIXED),
        ],
    ),
    port(
        "Rewards",
        &[
            Entry::Plain("shuffle_treasure_reward"),
            Entry::Plain("shuffle_potion_shop"),
            Entry::Plain("shuffle_trade_checks"),
            Entry::View(&COWS),
            Entry::Plain("shuffle_frog_songs"),
            Entry::Plain("shuffle_frog_bug_heart"),
            Entry::View(&BIG_POE),
        ],
    ),
];

const EXPERIMENTAL_TAB: &[PortSection] = &[port(
    "Sanities",
    &[
        Entry::Plain("shuffle_pots"),
        Entry::Plain("shuffle_crates"),
        Entry::Plain("shuffle_rocks"),
        Entry::Plain("shuffle_grass"),
        Entry::Plain("shuffle_beehives"),
    ],
)];

pub const RANDOMIZER: &str = "Randomizer";
pub const GAMEPLAY: &str = "Gameplay";
pub const IN_DEV: &str = "In Dev";
pub const DEBUG: &str = "Debug";

pub const MENU: &[Page] = &[
    Page { id: "win", label: "Logic / Goal", heading: RANDOMIZER, sections: &[] },
    Page {
        id: "world",
        label: "World",
        heading: RANDOMIZER,
        sections: &[],
    },
    Page {
        id: "dungeon",
        label: "Dungeons",
        heading: RANDOMIZER,
        sections: &[
            sec("Silver Rupees", &["shuffle_silver_rupees"]),
        ],
    },
    Page {
        id: "locations",
        label: "Shuffle",
        heading: RANDOMIZER,
        sections: &[],
    },
    Page {
        id: "experimental",
        label: "Experimental",
        heading: RANDOMIZER,
        sections: &[],
    },
    Page {
        id: "npcs",
        label: "NPCs and rewards",
        heading: RANDOMIZER,
        sections: &[
        ],
    },
    Page {
        id: "starting",
        label: "Starting inventory",
        heading: RANDOMIZER,
        sections: &[sec("Start", &["starting_hearts", "start_reward"]), sec("Songs", &["starting_songs"]), sec("Items", &["starting_items"])],
    },
    Page { id: "excluded", label: "Excluded locations", heading: RANDOMIZER, sections: &[sec("Excluded locations", &["disabled_locations"])] },
    Page {
        id: "timesavers",
        label: "Time savers",
        heading: GAMEPLAY,
        sections: &[
            sec("Cutscenes", &["skip_intro", "skip_cutscenes", "skip_story_scenes", "faster_chests", "skip_first_pickup_text"]),
            sec("Side quests", &["skip_mask_quest", "anju_one_cucco", "skulls_by_day"]),
            sec("Dungeons", &["deku_bridge_safe"]),
            sec("Puzzles", &["faster_blocks"]),
        ],
    },
    Page {
        id: "tweaks",
        label: "Tweaks",
        heading: GAMEPLAY,
        sections: &[sec("Time of day", &["time_always_passes", "early_beds"]), sec("Link and Navi", &["navi_return", "bunny_hood"])],
    },
    Page { id: "hints", label: "Hints", heading: GAMEPLAY, sections: &[sec("Hints", &["gossip_hints", "chests_match_contents", "misc_hints", "scrub_hint_text"])] },
    Page {
        id: "difficulty",
        label: "Difficulty",
        heading: GAMEPLAY,
        sections: &[
            sec("Enemies", &["enemy_health", "enemy_damage", "enemy_speed", "enemy_regen", "enemy_backup", "no_z_targeting", "more_enemies", "enemies_respawn_sooner", "enemy_gates", "cucco_revenge"]),
            sec("Drops and healing", &["no_heart_drops", "no_ammo_drops", "no_gossip_fairies", "no_bottles_in_combat", "no_dupe_items", "fairy_helps_enemies"]),
            sec("Boss fights", &["boss_minibosses", "boss_minions"]),
            sec("Hazards", &["fireball_storms"]),
            sec("Game over", &["lose_heart_on_death"]),
            sec("Minigames", &["unbreakable_bowling_grass"]),
            sec("Just for fun", &["enemy_spawn_combo"]),
        ],
    },
    Page { id: "debug", label: "Debug", heading: DEBUG, sections: &[sec("Debug", &["debug_location_names"])] },
];

pub fn problems(sch: &Schema) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut placed: Set<&str> = Set::new();
    let mut pages: Set<&str> = Set::new();
    let moved = MOVED.iter().flat_map(|(_, t)| t.iter());
    for p in PORT {
        if !pages.insert(p.id) {
            out.push(format!("menu page {} is listed twice", p.id));
        }
    }
    for s in PORT.iter().flat_map(|p| p.sections.iter()).chain(moved) {
        {
            for e in s.entries {
                let id: Option<&str> = match e {
                    Entry::View(v) => Some(v.id),
                    Entry::Hidden(id) | Entry::Plain(id) => Some(id),
                    Entry::Stub(..) => None,
                };
                if let Some(id) = id {
                    if !placed.insert(id) {
                        out.push(format!("setting {id} is placed in the menu more than once"));
                    }
                }
            }
        }
    }
    for p in MENU {
        if !pages.insert(p.id) {
            out.push(format!("menu page {} is listed twice", p.id));
        }
        for s in p.sections {
            for id in s.ids {
                if !placed.insert(id) {
                    out.push(format!("setting {id} is placed in the menu more than once"));
                }
            }
        }
    }
    for s in &sch.settings {
        if !placed.contains(s.id.as_str()) {
            out.push(format!("setting {} has no place in the settings menu (settings/menu.rs)", s.id));
        }
    }
    out
}

pub fn menu_json(sch: &Schema) -> Json {
    let have: Set<&str> = sch.settings.iter().map(|s| s.id.as_str()).collect();
    let mut placed: Set<&str> = Set::new();
    let mut out: Vec<Json> = Vec::new();
    for p in PORT {
        let secs: Vec<Json> = port_secs(p.sections, &have, &mut placed);
        out.push(page_json(p.id, p.label, SOH_PORT, secs));
    }
    for p in MENU {
        let mut secs: Vec<Json> = match MOVED.iter().find(|(id, _)| *id == p.id) {
            Some((_, tab)) => port_secs(tab, &have, &mut placed),
            None => Vec::new(),
        };
        for s in p.sections {
            let ids: Vec<&str> = s.ids.iter().copied().filter(|x| have.contains(x) && placed.insert(x)).collect();
            if !ids.is_empty() {
                secs.push(Json::obj().with("label", s.label.into()).with("ids", Json::strs(ids.into_iter())));
            }
        }
        if !secs.is_empty() {
            out.push(page_json(p.id, p.label, p.heading, secs));
        }
    }
    let left: Vec<&str> = sch.settings.iter().map(|s| s.id.as_str()).filter(|x| !placed.contains(x)).collect();
    if !left.is_empty() {
        let sec: Json = Json::obj().with("label", "Not placed yet".into()).with("ids", Json::strs(left.into_iter()));
        out.push(page_json("other", "Unsorted", IN_DEV, vec![sec]));
    }
    Json::Arr(out)
}

fn port_secs<'a>(sections: &'a [PortSection], have: &Set<&str>, placed: &mut Set<&'a str>) -> Vec<Json> {
    let mut secs: Vec<Json> = Vec::new();
    for s in sections {
        let mut ids: Vec<&str> = Vec::new();
        let mut hidden: Vec<&str> = Vec::new();
        let mut items: Vec<Json> = Vec::new();
        for e in s.entries {
            match e {
                Entry::View(v) if have.contains(v.id) && placed.insert(v.id) => {
                    ids.push(v.id);
                    items.push(Json::obj().with("id", v.id.into()).with("view", view_json(v)));
                }
                Entry::Hidden(id) if have.contains(id) && placed.insert(id) => hidden.push(id),
                Entry::Plain(id) if have.contains(id) && placed.insert(id) => {
                    ids.push(id);
                    items.push(Json::obj().with("id", (*id).into()));
                }
                Entry::View(_) | Entry::Hidden(_) | Entry::Plain(_) => {}
                Entry::Stub(label, w, tip) => items.push(stub_json(label, w).with("tip", (*tip).into())),
            }
        }
        if !items.is_empty() || !hidden.is_empty() {
            secs.push(
                Json::obj()
                    .with("label", s.label.into())
                    .with("ids", Json::strs(ids.into_iter()))
                    .with("hidden", Json::strs(hidden.into_iter()))
                    .with("items", Json::Arr(items)),
            );
        }
    }
    secs
}

fn view_json(v: &View) -> Json {
    let opts: Vec<Json> =
        v.opts.iter().map(|o| Json::obj().with("label", o.label.into()).with("ours", o.ours.into()).with("live", o.live.into()).with("needs", o.needs.into())).collect();
    let j: Json = Json::obj().with("label", v.label.into()).with("tip", v.tip.into()).with("opts", Json::Arr(opts));
    match v.kind {
        Kind::Pick => j.with("kind", "pick".into()),
        Kind::Check => j.with("kind", "check".into()),
        Kind::Steps(min, max) => j.with("kind", "steps".into()).with("min", min.into()).with("max", max.into()),
        Kind::Number(min, max, live) => j.with("kind", "number".into()).with("min", min.into()).with("max", max.into()).with("live", live.into()),
    }
}

fn stub_json(label: &str, w: &Widget) -> Json {
    let j: Json = Json::obj().with("stub", label.into());
    match w {
        Widget::Check => j.with("kind", "toggle".into()),
        Widget::Pick(opts, at) => j.with("kind", "choice".into()).with("choices", Json::strs(opts.iter().copied())).with("value", (*at as i64).into()),
        Widget::Slider(min, max, at) => j.with("kind", "number".into()).with("min", (*min).into()).with("max", (*max).into()).with("value", (*at).into()),
    }
}

fn page_json(id: &str, label: &str, heading: &str, secs: Vec<Json>) -> Json {
    let j: Json = Json::obj().with("id", id.into()).with("label", label.into()).with("heading", heading.into()).with("sections", Json::Arr(secs));
    match WARNINGS.iter().find(|(p, _)| *p == id) {
        Some((_, w)) => j.with("warning", (*w).into()),
        None => j,
    }
}

#[cfg(test)]
mod tests {
    use super::super::schema;
    use super::*;

    #[test]
    fn every_setting_has_exactly_one_place_in_the_menu() {
        assert_eq!(problems(schema()), Vec::<String>::new());
    }

    #[test]
    fn exported_menu_lists_each_setting_once_and_skips_unknown_ids() {
        let sch: &Schema = schema();
        let j: Json = menu_json(sch);
        let mut seen: Vec<String> = Vec::new();
        for p in j.as_arr().unwrap() {
            for s in p.get("sections").and_then(|x| x.as_arr()).unwrap() {
                for id in s.get("ids").and_then(|x| x.as_arr()).unwrap() {
                    seen.push(id.as_str().unwrap().to_string());
                }
                for id in s.get("hidden").and_then(|x| x.as_arr()).map(|a| a.to_vec()).unwrap_or_default() {
                    seen.push(id.as_str().unwrap().to_string());
                }
            }
        }
        let mut want: Vec<String> = sch.settings.iter().map(|s| s.id.clone()).collect();
        seen.sort();
        want.sort();
        assert_eq!(seen, want);
    }

    #[test]
    fn the_sanities_sit_on_the_experimental_page_under_its_warning() {
        let j: Json = menu_json(schema());
        let p: &Json = j.as_arr().unwrap().iter().find(|p| p.get("id").and_then(|x| x.as_str()) == Some("experimental")).unwrap();
        assert!(p.get("warning").and_then(|x| x.as_str()).unwrap().contains("unbeatable"));
        let ids: Vec<&str> = p.get("sections").and_then(|x| x.as_arr()).unwrap().iter().flat_map(|s| s.get("ids").and_then(|x| x.as_arr()).unwrap().iter().map(|i| i.as_str().unwrap())).collect();
        assert_eq!(ids, vec!["shuffle_pots", "shuffle_crates", "shuffle_rocks", "shuffle_grass", "shuffle_beehives"]);
    }

    #[test]
    fn every_soh_port_tab_is_exported_even_when_empty() {
        let j: Json = menu_json(schema());
        let port: Vec<&str> = j
            .as_arr()
            .unwrap()
            .iter()
            .filter(|p| p.get("heading").and_then(|x| x.as_str()) == Some(SOH_PORT))
            .map(|p| p.get("id").and_then(|x| x.as_str()).unwrap())
            .collect();
        assert_eq!(port, PORT.iter().map(|p| p.id).collect::<Vec<_>>());
    }
}
