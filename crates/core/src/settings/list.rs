use super::model::{Group, Schema, Setting, Tag};

pub const SCHEMA_VERSION: u32 = 1;

fn group(id: &str, label: &str, tip: &str) -> Group {
    Group { id: id.to_string(), label: label.to_string(), tip: tip.to_string() }
}

fn where_checks(s: Setting) -> Setting {
    s.opt("off", "Vanilla", "These checks keep their original items.")
        .opt("overworld", "Overworld only", "Only checks outside dungeons are shuffled.")
        .opt("dungeons", "Dungeons only", "Only checks inside dungeons are shuffled.")
        .opt("all", "Everywhere", "Every check of this kind is shuffled.")
}

fn key_places(s: Setting) -> Setting {
    s.opt("remove", "Removed", "The item is removed and the doors it opens start unlocked.")
        .opt("vanilla", "Vanilla spot", "The item stays in its original location.")
        .opt("dungeon", "Own dungeon", "The item appears somewhere in its own dungeon.")
        .opt("regional", "Same region", "The item appears in its dungeon or a nearby area.")
        .opt("overworld", "Overworld only", "The item appears outside of dungeons.")
        .opt("any_dungeon", "Any dungeon", "The item appears in any dungeon.")
        .opt("keysanity", "Anywhere", "The item can appear anywhere in the world.")
}

fn reward_goal(s: Setting) -> Setting {
    s.opt("stones", "Spiritual Stones", "Collect a set number of Spiritual Stones.")
        .opt("medallions", "Medallions", "Collect a set number of Medallions.")
        .opt("dungeons", "Dungeon rewards", "Collect a set number of Stones and Medallions together.")
        .opt("tokens", "Gold Skulltula Tokens", "Collect a set number of Gold Skulltula Tokens.")
        .opt("hearts", "Hearts", "Reach a set number of heart containers.")
}

fn bridge() -> Vec<Setting> {
    let b: Setting = reward_goal(
        Setting::choice(24, "bridge", "goal", "Rainbow Bridge", "medallions")
            .tag(Tag::Core)
            .tip("What it takes to make the Rainbow Bridge to Ganon's Castle appear.")
            .opt("open", "Always open", "The bridge is there from the start.")
            .opt("vanilla", "Vanilla", "The Shadow and Spirit Medallions and the Light Arrows are needed."),
    )
    .opt("random", "Random", "A random requirement other than tokens is chosen.");
    vec![
        b,
        Setting::number(25, "bridge_stones", "goal", "Stones for the bridge", 3, 0, 3)
            .tip("How many Spiritual Stones open the bridge.")
            .when("bridge", "stones"),
        Setting::number(26, "bridge_medallions", "goal", "Medallions for the bridge", 6, 0, 6)
            .tip("How many Medallions open the bridge.")
            .when("bridge", "medallions"),
        Setting::number(27, "bridge_rewards", "goal", "Rewards for the bridge", 9, 0, 9)
            .tip("How many Stones and Medallions together open the bridge.")
            .when("bridge", "dungeons"),
        Setting::number(28, "bridge_tokens", "goal", "Tokens for the bridge", 100, 0, 100)
            .tip("How many Gold Skulltula Tokens open the bridge.")
            .when("bridge", "tokens"),
        Setting::number(29, "bridge_hearts", "goal", "Hearts for the bridge", 20, 4, 20)
            .tip("How many hearts open the bridge.")
            .when("bridge", "hearts"),
    ]
}

fn ganon_key() -> Vec<Setting> {
    let k: Setting = reward_goal(key_places(
        Setting::choice(18, "shuffle_ganon_bosskey", "keys", "Ganon's Boss Key", "dungeon")
            .tag(Tag::Core)
            .tip("Where Ganon's Castle Boss Key is, or what earns it."),
    ));
    vec![
        k,
        Setting::number(19, "ganon_bosskey_stones", "keys", "Stones for the key", 3, 0, 3)
            .tip("How many Spiritual Stones award the key.")
            .when("shuffle_ganon_bosskey", "stones"),
        Setting::number(20, "ganon_bosskey_medallions", "keys", "Medallions for the key", 6, 0, 6)
            .tip("How many Medallions award the key.")
            .when("shuffle_ganon_bosskey", "medallions"),
        Setting::number(21, "ganon_bosskey_rewards", "keys", "Rewards for the key", 9, 0, 9)
            .tip("How many Stones and Medallions together award the key.")
            .when("shuffle_ganon_bosskey", "dungeons"),
        Setting::number(22, "ganon_bosskey_tokens", "keys", "Tokens for the key", 100, 0, 100)
            .tip("How many Gold Skulltula Tokens award the key.")
            .when("shuffle_ganon_bosskey", "tokens"),
        Setting::number(23, "ganon_bosskey_hearts", "keys", "Hearts for the key", 20, 4, 20)
            .tip("How many hearts award the key.")
            .when("shuffle_ganon_bosskey", "hearts"),
    ]
}

fn starting_items() -> Setting {
    Setting::multi(36, "starting_items", "starting", "Starting items")
        .tip("Items you begin the game with. They are removed from the item pool.")
        .opts(&[
            ("ocarina", "Ocarina"),
            ("slingshot", "Slingshot"),
            ("boomerang", "Boomerang"),
            ("bow", "Bow"),
            ("bomb_bag", "Bomb Bag"),
            ("bombchus", "Bombchus"),
            ("hookshot", "Hookshot"),
            ("longshot", "Longshot"),
            ("megaton_hammer", "Megaton Hammer"),
            ("lens_of_truth", "Lens of Truth"),
            ("magic_meter", "Magic Meter"),
            ("dins_fire", "Din's Fire"),
            ("farores_wind", "Farore's Wind"),
            ("nayrus_love", "Nayru's Love"),
            ("fire_arrows", "Fire Arrows"),
            ("ice_arrows", "Ice Arrows"),
            ("light_arrows", "Light Arrows"),
            ("iron_boots", "Iron Boots"),
            ("hover_boots", "Hover Boots"),
            ("goron_tunic", "Goron Tunic"),
            ("zora_tunic", "Zora Tunic"),
            ("mirror_shield", "Mirror Shield"),
            ("bottle", "Empty Bottle"),
            ("magic_beans", "Magic Beans"),
            ("zeldas_letter", "Zelda's Letter"),
            ("stone_of_agony", "Stone of Agony"),
            ("gerudo_card", "Gerudo Card"),
            ("goron_bracelet", "Goron's Bracelet"),
            ("silver_scale", "Silver Scale"),
            ("adult_wallet", "Adult's Wallet"),
            ("kokiri_sword", "Kokiri Sword"),
            ("deku_shield", "Deku Shield"),
        ])
}

fn starting_songs() -> Setting {
    Setting::multi(37, "starting_songs", "starting", "Starting songs")
        .tip("Songs you already know when the game starts.")
        .opts(&[
            ("zeldas_lullaby", "Zelda's Lullaby"),
            ("eponas_song", "Epona's Song"),
            ("sarias_song", "Saria's Song"),
            ("suns_song", "Sun's Song"),
            ("song_of_time", "Song of Time"),
            ("song_of_storms", "Song of Storms"),
            ("minuet_of_forest", "Minuet of Forest"),
            ("bolero_of_fire", "Bolero of Fire"),
            ("serenade_of_water", "Serenade of Water"),
            ("requiem_of_spirit", "Requiem of Spirit"),
            ("nocturne_of_shadow", "Nocturne of Shadow"),
            ("prelude_of_light", "Prelude of Light"),
        ])
}

pub fn build() -> Schema {
    let mut s: Vec<Setting> = vec![
        Setting::choice(1, "logic_rules", "world", "Logic", "glitchless")
            .tag(Tag::Core)
            .tip("How strictly the generator guarantees that the seed can be finished.")
            .opt("glitchless", "Glitchless", "Every item is reachable without glitches.")
            .opt("no_logic", "No logic", "Items are placed freely and the seed may be impossible."),
    ];
    s.push(
        Setting::choice(2, "starting_age", "world", "Starting age", "child")
            .tag(Tag::Core)
            .tip("Which Link you control at the start.")
            .opt("child", "Child", "Start as Child Link.")
            .opt("adult", "Adult", "Start as Adult Link with the Master Sword.")
            .opt("random", "Random", "The seed picks one at random."),
    );
    s.push(
        Setting::choice(3, "open_forest", "world", "Kokiri Forest", "closed")
            .tip("Whether the way out of the forest and into the Deku Tree is blocked.")
            .opt("open", "Open", "Nothing blocks the forest or the Deku Tree.")
            .opt("closed_deku", "Closed Deku", "The forest exit is open but the Deku Tree is closed.")
            .opt("closed", "Closed", "The forest and the Deku Tree stay closed until the story opens them."),
    );
    s.push(
        Setting::choice(4, "open_kakariko", "world", "Kakariko Gate", "open")
            .tip("Whether the gate to Kakariko Village needs Zelda's Letter. Open also skips the castle visit: a new game starts after Zelda and Impa, with their two items already given. Experimental, untested in the game.")
            .opt("open", "Open", "The gate is open from the start and the castle visit is skipped. Zelda's Letter and the Song from Impa, or whatever is placed there, are given when a new game starts.")
            .opt("zelda", "Zelda's Letter", "The gate opens when you show the letter.")
            .opt("closed", "Closed", "The gate stays closed until the story opens it."),
    );
    s.push(
        Setting::toggle(5, "open_door_of_time", "world", "Open Door of Time", true)
            .tip("Start with the Door of Time already open, so the Ocarina and the Song of Time are not needed to reach the Master Sword. Experimental, untested in the game."),
    );
    s.push(
        Setting::choice(6, "zora_fountain", "world", "Zora's Fountain", "closed")
            .tip("Whether King Zora blocks the way to Zora's Fountain.")
            .opt("closed", "Closed", "King Zora must be moved with Ruto's Letter.")
            .opt("adult", "Open as adult", "King Zora is gone once you are an adult.")
            .opt("open", "Open", "King Zora never blocks the way."),
    );
    s.push(
        Setting::choice(7, "gerudo_fortress", "world", "Gerudo Fortress", "normal")
            .tip("How many carpenters must be rescued to open the Fortress.")
            .opt("normal", "Rescue all four", "Free all four carpenters.")
            .opt("fast", "Rescue one", "Free only one carpenter.")
            .opt("open", "Open", "The gate is open and no carpenters need rescuing."),
    );
    s.push(
        where_checks(
            Setting::choice(8, "shuffle_chests", "shuffle", "Chests", "all")
                .tag(Tag::Core)
                .tip("Which treasure chests hold random items."),
        ),
    );
    s.push(
        where_checks(
            Setting::choice(9, "shuffle_pots", "shuffle", "Pots", "off")
                .tip("Which pots, including flying pots, hold random items."),
        ),
    );
    s.push(
        where_checks(
            Setting::choice(10, "shuffle_crates", "shuffle", "Crates", "off")
                .tip("Which crates hold random items."),
        ),
    );
    s.push(
        Setting::choice(11, "shuffle_scrubs", "shuffle", "Business Scrubs", "off")
            .tip("Whether Business Scrub sales hold random items, and what they cost.")
            .opt("off", "Vanilla", "Only the three scrubs that give one-time items are shuffled.")
            .opt("low", "Affordable", "Every scrub sells its item for 10 rupees.")
            .opt("regular", "Expensive", "Scrubs keep their original prices.")
            .opt("random", "Random prices", "Each scrub gets a random price."),
    );
    s.push(
        where_checks(
            Setting::choice(12, "shuffle_freestanding_items", "shuffle", "Freestanding items", "off")
                .tip("Which rupees, hearts and heart pieces lying on the ground hold random items. Only heart pieces, rupees and recovery hearts can be placed there. Experimental, untested in the game."),
        ),
    );
    s.push(
        Setting::list(13, "disabled_locations", "shuffle", "Excluded locations", 500)
            .tag(Tag::Niche)
            .tip("Locations that never hold an important item. Each entry is a location id from the location catalog."),
    );
    s.push(key_places(
        Setting::choice(14, "shuffle_smallkeys", "keys", "Small Keys", "dungeon")
            .tag(Tag::Core)
            .tip("Where Small Keys can appear."),
    ));
    s.push(key_places(
        Setting::choice(15, "shuffle_bosskeys", "keys", "Boss Keys", "dungeon")
            .tag(Tag::Core)
            .tip("Where dungeon Boss Keys can appear."),
    ));
    s.push(
        Setting::choice(16, "shuffle_mapcompass", "keys", "Maps and Compasses", "dungeon")
            .tip("Where Dungeon Maps and Compasses can appear.")
            .opt("remove", "Removed", "Maps and Compasses are removed and their effects are always on.")
            .opt("startwith", "Start with them", "You begin with every Map and Compass.")
            .opt("vanilla", "Vanilla spot", "They stay in their original locations.")
            .opt("dungeon", "Own dungeon", "They appear in their own dungeon.")
            .opt("regional", "Same region", "They appear in their dungeon or a nearby area.")
            .opt("overworld", "Overworld only", "They appear outside of dungeons.")
            .opt("any_dungeon", "Any dungeon", "They appear in any dungeon.")
            .opt("keysanity", "Anywhere", "They can appear anywhere in the world."),
    );
    s.push(
        Setting::choice(17, "shuffle_hideoutkeys", "keys", "Thieves' Hideout keys", "vanilla")
            .tag(Tag::Niche)
            .tip("Where the Small Keys of the Thieves' Hideout can appear.")
            .opt("vanilla", "Vanilla spot", "Each key stays with its carpenter.")
            .opt("regional", "Same region", "The keys appear in the Fortress region.")
            .opt("overworld", "Overworld only", "The keys appear outside of dungeons.")
            .opt("any_dungeon", "Any dungeon", "The keys appear in any dungeon.")
            .opt("keysanity", "Anywhere", "The keys can appear anywhere in the world."),
    );
    s.extend(ganon_key());
    s.extend(bridge());
    s.push(
        Setting::toggle(30, "trials_random", "goal", "Random Ganon's Trials", false)
            .tip("Pick the number of trials to clear at random."),
    );
    s.push(
        Setting::number(31, "trials", "goal", "Ganon's Trials", 6, 0, 6)
            .tip("How many of the six trials in Ganon's Castle must be cleared.")
            .when_off("trials_random"),
    );
    s.push(
        Setting::toggle(32, "triforce_hunt", "goal", "Triforce Hunt", false)
            .tag(Tag::Core)
            .tip("Finish by collecting Triforce pieces instead of defeating Ganon."),
    );
    s.push(
        Setting::number(33, "triforce_count_per_world", "goal", "Triforce pieces in the pool", 30, 1, 999)
            .tip("How many Triforce pieces are placed in the world.")
            .when_on("triforce_hunt"),
    );
    s.push(
        Setting::number(34, "triforce_goal_per_world", "goal", "Triforce pieces needed", 20, 1, 999)
            .tip("How many Triforce pieces you must collect. It cannot exceed the number in the pool.")
            .when_on("triforce_hunt")
            .max_by("triforce_count_per_world"),
    );
    s.push(
        Setting::number(35, "starting_hearts", "starting", "Starting hearts", 3, 1, 20)
            .tip("How many heart containers you begin with."),
    );
    s.push(
        Setting::choice(38, "shuffle_rewards", "shuffle", "Story and NPC rewards", "off")
            .tip("Whether the items that boss rooms, NPCs, minigames and Great Fairies hand out are shuffled. Songs are never shuffled yet.")
            .opt("off", "Vanilla", "These rewards keep their original items.")
            .opt("supported", "Supported rewards", "The rewards whose event the tool can edit. Experimental, none of these edits are tested in the game yet."),
    );
    s.push(
        Setting::toggle(51, "shuffle_rewards_extra", "shuffle", "More NPC rewards", false)
            .tip("Also shuffle the rewards that need bigger event edits: the 20 token Skulltula reward, Biggoron and Medigoron. Experimental, none of these edits are tested in the game yet.")
            .when("shuffle_rewards", "supported"),
    );
    s.push(
        Setting::toggle(53, "shuffle_trade_checks", "shuffle", "Adult trade quest checks", false)
            .tip("Each of the ten adult trade quest hand ins also gives one shuffled item the first time. The trade items themselves stay in their vanilla order. Experimental, none of these edits are tested in the game yet.")
            .when("shuffle_rewards", "supported"),
    );
    s.push(
        Setting::toggle(54, "shuffle_scrubs_a", "shuffle", "One time scrub sales", false)
            .tip("Also shuffle the Business Scrubs that already sell one item once: the Death Mountain Crater heart scrub and the Lost Woods stick and nut bag scrubs. The price stays and the sale text still names the old item. Experimental, none of these edits are tested in the game yet.")
            .when("shuffle_rewards", "supported"),
    );
    s.push(
        Setting::toggle(56, "shuffle_scrubs_b", "shuffle", "Menu scrubs as one time sales", false)
            .tip("Also shuffle the Business Scrubs in the Lost Woods scrub grotto and in Jabu Jabu's Belly. A scrub that holds another item sells it once for 20 or 30 rupees instead of its menu, and its sale text still names a heart piece. Experimental, none of these edits are tested in the game yet.")
            .when("shuffle_rewards", "supported"),
    );
    s.push(
        Setting::toggle(39, "shuffle_boss_hearts", "shuffle", "Boss Heart Containers", false)
            .tip("Whether the Heart Containers that bosses leave behind join the item pool. Each one becomes a chest that holds a random item. Experimental, untested in the game."),
    );
    s.push(
        Setting::choice(40, "shuffle_songs", "shuffle", "Songs", "off")
            .tip("Whether the songs you learn from NPCs and Sheik are shuffled. Experimental, untested in the game.")
            .opt("off", "Vanilla", "Every song comes from its usual teacher.")
            .opt("own", "Song locations", "The songs are shuffled among the places that teach songs.")
            .opt("anywhere", "Anywhere", "Songs can appear anywhere and song locations can hold any item."),
    );
    s.push(
        Setting::toggle(41, "gossip_hints", "world", "Gossip stone hints", false)
            .tip("Every gossip stone tells you where an important item lies. The stones need no Mask of Truth text change, only the words are replaced. Experimental, untested in the game."),
    );
    s.push(
        Setting::toggle(42, "faster_chests", "world", "Faster item pickups", false)
            .tip("Shortens the waiting time in chest openings and item pickup scenes. The text and the item are the same, only the pauses are shorter."),
    );
    s.push(
        Setting::toggle(44, "skip_intro", "world", "Skip the opening cutscenes", true)
            .tip("A new game skips the opening movie and the wake up scene in Link's house, and you get control right away. Experimental, untested in the game."),
    );
    s.push(
        Setting::choice(43, "start_reward", "starting", "Start with a reward", "off")
            .tip("Begin the game already holding one random Spiritual Stone or Medallion. The vanilla copy still exists. Experimental, untested in the game.")
            .opt("off", "No reward", "You start with no stones or medallions.")
            .opt("stone", "A Spiritual Stone", "One random Spiritual Stone.")
            .opt("medal", "A Medallion", "One random Medallion.")
            .opt("any", "Either", "One random Stone or Medallion."),
    );
    s.push(
        Setting::choice(52, "shuffle_skulltulas", "shuffle", "Gold Skulltulas", "off")
            .tip("Whether Gold Skulltulas drop random items and their tokens join the item pool. A skulltula can only drop items that the game has a pickup for, and it stays dead once killed. Experimental, untested in the game.")
            .opt("off", "Vanilla", "Every skulltula drops its token.")
            .opt("pinned", "Confident matches", "Only the skulltulas whose remake actor is known for sure are shuffled.")
            .opt("matched", "Confident and pooled", "Also the pools of skulltulas that share the same logic, so a mixed up pairing cannot matter."),
    );
    s.push(
        Setting::choice(55, "shuffle_remake_chests", "shuffle", "Remake chests", "off")
            .tip("Whether the chests that only the remake has, and the dungeon and overworld chests newly paired with their OoTR spot, join the item pool. Their rules come from research. Seven overworld chests whose spot is unproven only take items that are not progression. Experimental, untested in the game.")
            .opt("off", "Vanilla", "These chests keep their original items.")
            .opt("on", "Shuffled", "These chests hold random items."),
    );
    s.push(starting_items());
    s.push(starting_songs());
    s.sort_by_key(|x| x.num);
    Schema {
        version: SCHEMA_VERSION,
        groups: vec![
            group("world", "World", "How open the world is and who you start as."),
            group("shuffle", "Shuffles", "Which kinds of checks hold random items."),
            group("keys", "Keys", "Where dungeon keys and maps can appear."),
            group("goal", "Goals", "What you need to do to finish the game."),
            group("starting", "Starting items", "What you begin the game with."),
        ],
        settings: s,
    }
}
