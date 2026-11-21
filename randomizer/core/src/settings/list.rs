use super::model::{Group, Schema, Setting, Tag, Value};

pub const SCHEMA_VERSION: u32 = 1;

/// Setting numbers and ids that are gone. An old settings string or file that carries one
/// still imports, with a warning that its value is ignored, and no setting may reuse them.
/// 55 and 92 (remake only chests and pots) now follow Chests, Pots and Crates.
pub const RETIRED: [(u16, &str); 11] = [
    (crate::difficulty::ENCOUNTERS_NUM, crate::difficulty::ENCOUNTERS),
    (crate::difficulty::ZORA_AMBUSH_NUM, crate::difficulty::ZORA_AMBUSH),
    (crate::difficulty::BEAN_PITCH_NUM, crate::difficulty::BEAN_PITCH),
    (55, "shuffle_remake_chests"),
    (63, "shop_deal_gear"),
    (92, "shuffle_remake_pots"),
    (106, crate::short_get::SETTING),
    crate::shops::plan::OLD[0],
    crate::shops::plan::OLD[1],
    crate::shops::plan::OLD[2],
    crate::shops::plan::OLD[3],
];

pub fn retired_num(num: u16) -> Option<&'static str> {
    RETIRED.iter().find(|r| r.0 == num).map(|r| r.1)
}

pub fn retired_id(id: &str) -> bool {
    RETIRED.iter().any(|r| r.1 == id)
}

fn group(id: &str, label: &str, tip: &str) -> Group {
    Group { id: id.to_string(), label: label.to_string(), tip: tip.to_string() }
}

fn where_checks(s: Setting) -> Setting {
    s.opt("off", "Off", "These checks keep their original items.")
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
        Setting::choice(24, "bridge", "goal", "Rainbow Bridge", "vanilla")
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
            .tip("Where Ganon's Castle Boss Key is, or what earns it.")
            .opt(crate::lacs_key::VALUE, "LACS - Dungeon Rewards", "Zelda gives the key in the Light Arrows Cutscene, which plays once you hold a set number of Stones and Medallions together. The Light Arrows are shuffled into the item pool. Experimental, untested in the game."),
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
        .tip("Items you begin the game with. One copy of each is taken out of the item pool and replaced by Rupee (1). Upgrades are ticked by tier and the highest ticked tier counts: Longshot means two progressive Hookshots, Big Quiver means the Bow and one quiver upgrade, Big Bullet Bag means the Slingshot and one bullet bag upgrade, Giant's Wallet means two wallet upgrades, Double Magic means two magic upgrades, Ocarina of Time means both ocarinas, Golden Gauntlets means all three strength upgrades. Bags start full. Bombchus need a bomb bag. Zelda's Letter cannot be given.")
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
            ("stone_of_agony", "Echo Gem (Stone of Agony)"),
            ("gerudo_card", "Gerudo Card"),
            ("goron_bracelet", "Goron's Bracelet"),
            ("silver_scale", "Silver Scale"),
            ("adult_wallet", "Adult's Wallet"),
            ("kokiri_sword", "Kokiri Sword"),
            ("deku_shield", "Deku Shield"),
            ("big_bullet_bag", "Big Bullet Bag"),
            ("biggest_bullet_bag", "Biggest Bullet Bag"),
            ("big_quiver", "Big Quiver"),
            ("biggest_quiver", "Biggest Quiver"),
            ("big_bomb_bag", "Big Bomb Bag"),
            ("biggest_bomb_bag", "Biggest Bomb Bag"),
            ("deku_sticks", "Deku Sticks"),
            ("big_stick_bag", "Bigger Stick Capacity"),
            ("biggest_stick_bag", "Biggest Stick Capacity"),
            ("deku_nuts", "Deku Nuts"),
            ("big_nut_bag", "Bigger Nut Capacity"),
            ("biggest_nut_bag", "Biggest Nut Capacity"),
            ("silver_gauntlets", "Silver Gauntlets"),
            ("golden_gauntlets", "Golden Gauntlets"),
            ("golden_scale", "Golden Scale"),
            ("giant_wallet", "Giant's Wallet"),
            ("double_magic", "Double Magic"),
            ("ocarina_of_time", "Ocarina of Time"),
            ("two_bottles", "Two Empty Bottles"),
            ("three_bottles", "Three Empty Bottles"),
            ("four_bottles", "Four Empty Bottles"),
            ("hylian_shield", "Hylian Shield"),
        ])
}

fn starting_songs() -> Setting {
    Setting::multi(37, "starting_songs", "starting", "Starting songs")
        .tip("Songs you already know when the game starts. Each one is taken out of the item pool when its lesson is shuffled. Playing them still needs an Ocarina.")
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

fn difficulty() -> Vec<Setting> {
    use crate::difficulty as d;
    vec![
        Setting::number(d::HEALTH_NUM, d::HEALTH, "difficulty", "Enemy health", d::NORMAL, d::STAT_RANGE.0, d::STAT_RANGE.1)
            .multiplier(d::STAT_STEP, d::NORMAL)
            .tip("How much health enemies and bosses have, from 0.5x to 5x. Health tops out at 99."),
        Setting::number(d::DAMAGE_NUM, d::DAMAGE, "difficulty", "Enemy damage", d::NORMAL, d::STAT_RANGE.0, d::STAT_RANGE.1)
            .multiplier(d::STAT_STEP, d::NORMAL)
            .tip("How hard enemies and their projectiles hit, from 0.5x to 5x. Traps, lava, falls and grabs do their usual damage."),
        Setting::choice(d::HURT_NUM, d::HURT, "difficulty", "Damage Link takes", "normal")
            .opt("normal", "Normal", "Enemies, traps and falls hurt as usual.")
            .opt("double", "Double", "Everything that hurts Link, enemy hits, grabs and falls, does twice the damage.")
            .opt("quadruple", "Quadruple", "Everything that hurts Link does four times the damage.")
            .opt("one_hit", "One hit kills", "Any hit takes all of Link's hearts. A bottled fairy still revives him.")
            .tip("How much damage Link takes from enemy hits, boss attacks, grabs, projectiles and falls. Double and Quadruple multiply it. One hit kills takes every heart at once, and a bottled fairy still revives you. Falling into a pit or lava still costs one heart. Stacks with Enemy damage."),
        Setting::number(d::SPEED_NUM, d::SPEED, "difficulty", "Enemy speed", d::NORMAL, d::SPEED_RANGE.0, d::SPEED_RANGE.1)
            .multiplier(d::SPEED_STEP, d::NORMAL)
            .tip("How fast enemies move and attack, from 0.75x to 2x. Experimental: most enemies ignore it so far, and only Lizalfos clearly speed up."),
        Setting::toggle(d::REGEN_NUM, d::REGEN, "difficulty", "Enemies regen health", false)
            .tip("Enemies you leave alone heal. About 15 seconds after your last hit they get back one point every 5 seconds."),
        Setting::toggle(d::BACKUP_NUM, d::BACKUP, "difficulty", "Enemies call for backup", false)
            .tip("Fight an enemy for too long and it calls in one weaker enemy, once. A Stalfos brings a weak Stalfos, a Wolfos two Keese. Bosses and minibosses never call."),
        Setting::toggle(d::NO_HEARTS_NUM, d::NO_HEARTS, "difficulty", "No heart drops", false)
            .tip("Enemies, pots and grass never drop recovery hearts. Hearts placed as items in the seed are still there."),
        Setting::toggle(d::NO_FAIRIES_NUM, d::NO_FAIRIES, "difficulty", "No gossip fairies", false)
            .tip("Playing a song at a gossip stone no longer brings out a healing fairy."),
        Setting::toggle(d::NO_AMMO_NUM, d::NO_AMMO, "difficulty", "Limited ammo", false)
            .tip("Enemies, pots and grass never drop arrows, seeds, bombs, nuts or sticks, except where a boss fight stocks them: Gohma's larvae and eggs, the pots before the boss doors, the final battle rubble and Poe Meg. Shops and chests still have them, and the logic does not plan for this. Not played in the game yet."),
        Setting::toggle(d::LOSE_HEART_NUM, d::LOSE_HEART, "difficulty", "Lose heart on death", false)
            .tip("Each game over costs a heart container, down to three. Being saved by a bottled fairy does not count. Quitting to the title screen gives the hearts back."),
        Setting::toggle(d::NO_BOTTLES_NUM, d::NO_BOTTLES, "difficulty", "No bottles during combat", false)
            .tip("You can't use a bottle while the battle music plays or in a boss fight. A bottled fairy still revives you."),
        Setting::toggle(d::NO_TARGET_NUM, d::NO_TARGET, "difficulty", "No Z-targeting", false)
            .tip("You can't lock on to enemies or bosses. Aim assist and targeting people and signs still work."),
        Setting::toggle(d::NO_DUPES_NUM, d::NO_DUPES, "difficulty", "No dupe items", false)
            .tip("Two bottles can't hold the same thing, so no second fairy, potion or Poe. Different things in different bottles are fine."),
        Setting::number(d::MORE_NUM, d::MORE, "difficulty", "More enemies", d::NORMAL, d::MORE_RANGE.0, d::MORE_RANGE.1)
            .multiplier(d::MORE_STEP, d::NORMAL)
            .tip("How many enemies each area has, up to 10x. From 5x, new kinds join in. Above 1x you also get special encounters, such as a Skulltula ambush in Zora's Domain once you have a sword, and rare enemies from grass and pots (untested). Towns and boss rooms get none."),
        Setting::toggle(d::RESPAWN_NUM, d::RESPAWN, "difficulty", "Enemies respawn sooner", false)
            .tip("Beaten enemies come back when you leave the room or area and return, not only after leaving the dungeon or region. Bosses and minibosses stay beaten. Experimental, untested in the game."),
        Setting::toggle(d::MINIBOSSES_NUM, d::MINIBOSSES, "difficulty", "Bosses spawn minibosses", false)
            .tip("A Stalfos or a Dinolfos joins the fight with Gohma, Barinade, Phantom Ganon and Twinrova. It appears when the fight starts and leaves when the boss is beaten. Played with Gohma and Phantom Ganon; the other bosses are untested."),
        Setting::toggle(d::MINIONS_NUM, d::MINIONS, "difficulty", "Bosses spawn minions", false)
            .tip("Bosses bring a few Keese or Biri, and King Dodongo calls more Baby Dodongos. They appear when the fight starts (Bongo Bongo's are there from the start) and leave when the boss is beaten. Ganondorf and Ganon are unchanged. Played with Gohma and Phantom Ganon; the other bosses are untested."),
        Setting::toggle(d::GATES_NUM, d::GATES, "difficulty", "Enemies block the way", false)
            .tip("At ten narrow spots a barrier blocks the way until you beat its guards. It comes back when the area reloads. As a child they only appear once you have the Kokiri Sword. Off with shuffled entrances."),
        Setting::toggle(d::CUCCO_NUM, d::CUCCO, "difficulty", "Cuccos take revenge at once", false)
            .tip("Hit a Cucco once and the whole flock attacks, instead of after a few hits. Anju's Cuccos and the ones you have to find keep their usual temper."),
        Setting::toggle(d::BOWLING_NUM, d::BOWLING, "difficulty", "Unbreakable Bombchu Bowling grass", false)
            .tip("The grass in the Bombchu Bowling lanes can't be broken, so steer your Bombchus around it."),
        Setting::toggle(d::FIREBALLS_NUM, d::FIREBALLS, "difficulty", "Fireball storms", false)
            .tip("Burning rocks rain down on quiet stretches of Hyrule Field, Desert Colossus, the Haunted Wasteland and the Castle garden, and on every visit to the Death Mountain Trail. Dodge them."),
        Setting::toggle(d::SPAWN_COMBO_NUM, d::SPAWN_COMBO, "difficulty", "Spawn enemies (sticks + D-pad)", false)
            .tip("For fun and practice. Hold both sticks in and press D-pad Down for a random enemy in front of Link, or Up for the same kind again. Up to twelve at once."),
        Setting::toggle(d::VOLVAGIA_NUM, d::VOLVAGIA, "difficulty", "Volvagia roams Hyrule Field (Functional WIP)", false)
            .tip("Functional WIP: when beaten in the field she freezes before her death animation; she should drop a red rupee and flee (planned). As an adult, a copy of Volvagia flies over the open middle of Hyrule Field, coming out of the ground and diving back in around one spot as in its own fight, and breathes fire at you. It is its own actor: it plays no intro or death scene, has no boss music, gives nothing when beaten and changes no story flag, and the Fire Temple fight is unchanged. It comes back when the field loads again. It never blocks a path and the logic ignores it. Experimental, untested in the game."),
        Setting::toggle(d::MEGA_LIKE_NUM, d::MEGA_LIKE, "difficulty", "Shield-eating mega Like Likes", false)
            .tip("Three bigger, tougher Like Likes wait in the Deku Tree meadow of Kokiri Forest, as a child and as an adult. When one swallows you it takes one of your shields, any of them, the Mirror Shield first, then the Hylian Shield, then the Deku Shield. The shield stays gone until a mega Like Like is killed: any one of them gives back one shield you have owned and no longer have, the Mirror Shield first. Nothing is saved for this: the missing shield is the record. They come back every time the area loads. A shield that a normal Like Like swallowed, or a Deku Shield that burned, is also given back by a mega Like Like. If you build without this setting while a shield is missing, nothing gives it back. Experimental, untested in the game."),
        Setting::toggle(d::FUSES_NUM, d::FUSES, "difficulty", "Random fuses", false)
            .tip("Each bomb, Bombchu and lit Deku Stick gets a random fuse, from one second up to its usual time."),
        Setting::choice(d::timers::NUM, d::timers::SETTING, "difficulty", "Shorter timers", "off")
            .tip("Timed floor switches, eye switches and torches that burn out give you 75% or 50% of the usual time. Covers the Deku Tree, Dodongo's Cavern, Fire Temple, Gerudo Training Ground and Ganon's Castle. Each puzzle keeps a minimum it can still be finished in. Experimental: played for a floor switch and one torch, eye switches not yet.")
            .opt("off", "Off", "Timers run as in the game.")
            .opt("three_quarters", "75%", "Three quarters of the usual time.")
            .opt("half", "50%", "Half the usual time, or the least that puzzle allows."),
        Setting::toggle(d::RANDOMIZER_NUM, d::RANDOMIZER, "difficulty", "Enemy randomizer", false)
            .tip("Enemies are swapped for other kinds that live on the same kind of ground. Enemies a door or puzzle counts are left alone."),
        Setting::choice(d::SIZES_NUM, d::SIZES, "difficulty", "Random enemy sizes (not working yet)", "off")
            .opt("off", "Off", "Enemies are their usual size.")
            .opt("mild", "Mild", "Each enemy is 0.75 to 1.5 times its usual size.")
            .opt("wild", "Wild", "Each enemy is 0.5 to 2.5 times its usual size.")
            .tip("Every ordinary enemy gets its own size, the same for the same seed. Bosses, minibosses and the enemies a door or puzzle counts keep theirs, and indoors the biggest sizes are capped so nothing is wedged into a wall. Not working yet: the game ignores the size, so enemies keep their usual size. Experimental."),
        Setting::toggle(d::SIZES_HEALTH_NUM, d::SIZES_HEALTH, "difficulty", "Health follows size (not working yet)", false)
            .tip("With random enemy sizes on, an enemy's health is multiplied by its size, so a double-size Stalfos has twice the health and a half-size one half, never less than one. Sizes then come in a few steps. It stacks with the enemy health setting. Not working yet, as the sizes themselves are ignored."),
        Setting::toggle(d::FAIRY_HELPS_NUM, d::FAIRY_HELPS, "difficulty", "Fairies help enemies too", false)
            .tip("When you use a bottled fairy, or one revives you, every awake enemy and boss heals to full too."),
    ]
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
            .tip("Whether the way out of the forest and into the Deku Tree is blocked. Closed also closes the Kakariko gate. An open Kakariko gate needs the forest open, since its story state skips the Deku Tree quest past Mido; with an open Kakariko gate, Closed is locked out.")
            .opt("open", "Open", "Nothing blocks the forest or the Deku Tree.")
            .opt("closed", "Closed", "The forest and the Deku Tree stay closed until the story opens them, and the Kakariko gate stays closed."),
    );
    s.push(
        Setting::choice(4, "open_kakariko", "world", "Kakariko Gate", "closed")
            .dep("open_forest", super::model::Cond::Is(super::model::Value::s("open")))
            .disabled_as(super::model::Value::s("closed"))
            .tip("Whether the Kakariko Village gate to Death Mountain starts open. Open also skips the castle visit: a new game starts after Zelda and Impa, with their two items already given. Needs Kokiri Forest set to Open; with Closed the gate is locked to Closed, and a seed that asks for both is refused.")
            .opt("open", "Open", "The gate is open from the start and the castle visit is skipped. Zelda's Letter and the Song from Impa, or whatever is placed there, are given when a new game starts.")
            .opt("closed", "Closed", "The gate opens when you show Zelda's Letter to the guard."),
    );
    s.push(
        Setting::toggle(5, "open_door_of_time", "world", "Open Door of Time", false)
            .legacy()
            .tip("Start with the Door of Time already open, so the Ocarina and the Song of Time are not needed to reach the Master Sword. The Door of Time quest is left alone, so Zelda's escape and the Song of Time stay checks. The adult pedestal works before the Forest Sage in every seed, with or without this setting."),
    );
    s.push(
        Setting::choice(6, "zora_fountain", "world", "Zora's Fountain", "closed")
            .tip("Whether King Zora blocks the way to Zora's Fountain.")
            .opt("closed", "Closed", "King Zora must be moved with Ruto's Letter.")
            .opt("adult", "Open as adult", "King Zora is gone once you are an adult.")
            .opt("open", "Open", "King Zora never blocks the way. Ruto's Letter stays in the item pool."),
    );
    s.push(
        Setting::toggle(75, "jabu_open", "world", "Jabu Jabu's Belly open", false)
            .tip("Jabu Jabu opens his mouth without a fish, so no fish and no bottle are needed to enter Jabu Jabu's Belly. King Zora still has to be moved first unless Zora's Fountain is open."),
    );
    s.push(
        Setting::toggle(76, "time_always_passes", "world", "Time always passes", true)
            .wire_as(Value::Bool(false))
            .tip("The day and night clock runs from the start instead of waiting for Kaepora in Hyrule Field and Impa's send off, so night can be reached before meeting Zelda."),
    );
    s.push(
        Setting::toggle(77, "early_beds", "world", "Beds before Zelda", false)
            .tip("Beds offer Sleep until morning and Sleep until night before the Zelda meeting.")
            .when_on("time_always_passes"),
    );
    s.push(
        Setting::choice(7, "gerudo_fortress", "world", "Gerudo Fortress", "normal")
            .tip("How many carpenters must be rescued to open the Fortress.")
            .opt("normal", "Rescue all four", "Free all four carpenters.")
            .opt("fast", "Rescue one", "Tested in game. Three carpenters start free. The Membership Card is vanilla and is granted when the last carpenter is rescued.")
            .opt("open", "Open", "Tested in game. Start with the Gerudo Membership Card and the bridge built. The Membership Card check is still in the game and randomized. It triggers when you interact with your first jail cell."),
    );
    s.push(
        where_checks(
            Setting::choice(8, "shuffle_chests", "shuffle", "Chests", "all")
                .tag(Tag::Core)
                .tip("Which treasure chests hold random items, including the chests only the remake has."),
        ),
    );
    s.push(
        where_checks(
            Setting::choice(9, "shuffle_pots", "shuffle", "Pots", "off")
                .tip("Which pots, including flying pots and the pots only the remake has, hold random items. Key items whose pickup pack is built can sit in a pot. A shuffled pot gives its item once and stays broken in the save. An item you do not pick up is lost and can make the seed unbeatable, so save before breaking pots that may hold key items. Flying pots work the same way. Experimental, untested in the game."),
        ),
    );
    s.push(
        where_checks(
            Setting::choice(10, "shuffle_crates", "shuffle", "Crates", "off")
                .tip("Which crates, including the crates only the remake has, hold random items. Key items whose pickup pack is built can sit in a crate. A shuffled crate gives its item once and stays broken in the save. An item you do not pick up is lost and can make the seed unbeatable, so save before breaking crates that may hold key items. Experimental, untested in the game."),
        ),
    );
    s.push(
        // Old settings strings carry no value here and always shuffled both, so they read as on.
        Setting::toggle(crate::slice::pick::OCARINAS_NUM, crate::slice::pick::OCARINAS, "shuffle", "Ocarina Shuffle", false)
            .wire_as(Value::Bool(true))
            .tip("Whether both Ocarinas join the item pool: the Fairy Ocarina from Saria's gift and the Ocarina of Time Zelda throws in the moat after her escape. Off keeps them at those two spots. Both are only shuffled while NPC rewards are."),
    );
    s.push(
        Setting::toggle(crate::slice::pick::SWORD_NUM, crate::slice::pick::SWORD, "shuffle", "Kokiri Sword Shuffle", false)
            .wire_as(Value::Bool(true))
            .tip("Whether the Kokiri Sword joins the item pool. Off keeps it in its chest in the Kokiri Forest training area. The chest is only shuffled while overworld chests are."),
    );
    s.push(
        Setting::toggle(67, crate::pot_slice::plan::HIVES, "shuffle", "Beehives", false)
            .tip("Whether the seven beehives in the covered hole of Kakariko Village hold random items. They are the only beehives in the remake that drop an item. Key items whose pickup pack is built can sit in a hive. A shuffled hive gives its item once and stays fallen in the save. An item you do not pick up is lost and can make the seed unbeatable, so save before bringing down hives that may hold key items. A hive needs an explosive to open the hole and then the Boomerang, the Hookshot or a bow to bring down. Experimental, untested in the game."),
    );
    s.push(
        where_checks(
            Setting::choice(86, "shuffle_rocks", "shuffle", "Small Rocks", "off")
                .tip("Which small liftable rocks hold random items. The adult and the child copy of a rock are separate checks. Key items whose pickup pack is built can sit under a rock. A shuffled rock gives its item once and stays broken in the save. An item you do not pick up is lost, also when a thrown rock drops it out of reach, and that can make the seed unbeatable, so save before lifting rocks that may hold key items. Experimental, untested in the game."),
        ),
    );
    s.push(
        where_checks(
            Setting::choice(87, "shuffle_grass", "shuffle", "Grass", "off")
                .tip("Which grass tufts hold random items, in the overworld and in the Deku Tree and the Bottom of the Well. Key items whose pickup pack is built can sit in grass. A shuffled tuft gives its item once and stays cut in the save. An item you do not pick up is lost and can make the seed unbeatable, so save before cutting grass that may hold key items. The bug grass of the Lost Woods is left alone. Experimental, untested in the game."),
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
        Setting::choice(12, "shuffle_freestanding_items", "shuffle", "Freestanding items", "off")
            .tip("Which items lying on the ground hold random items. Rupees, hearts, heart pieces, the two shields, the other world pickups the game ships except the Gold Skulltula Token and Fire Arrows, and every key item whose pickup pack is built can be placed there. A spot that exists once per age only takes items that do no harm when found twice, such as rupees, hearts, ammo and gear that a second copy does not change.")
            .opt("off", "Off", "Only Pieces of Heart are shuffled.")
            .opt("pieces", "Only Pieces of Heart", "Heart pieces on the ground and the three heart piece chests are shuffled. Rupees and recovery hearts on the ground stay as they are.")
            .opt("all", "All Items", "Heart pieces, rupees and recovery hearts on the ground are all shuffled, also the 98 rupees and hearts that only the remake has, whose logic region comes from the nearest known checks.")
            .opt("dungeons", "Dungeons", "Pieces of Heart, and every freestanding item inside dungeons, are shuffled.")
            .opt("overworld", "Overworld", "Pieces of Heart, and every freestanding item outside dungeons, are shuffled."),
    );
    s.push(
        Setting::list(13, "disabled_locations", "shuffle", "Excluded locations", 500)
            .tag(Tag::Core)
            .tip("Checks you drag in here never hold an item the seed needs, only filler and other junk. They stay in the shuffle, so they still change. If too many are excluded for the seed to be beatable, no seed is built."),
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
    s.extend(crate::medal_gate::settings());
    s.extend(crate::door_gate::settings());
    s.extend(crate::nocturne_gate::settings());
    s.extend(crate::prelude_gate::settings());
    s.push(
        Setting::toggle(30, crate::ganon_trials::SKIP, "goal", "Skip Ganon's Trials", false)
            .tip("The barrier in Ganon's Castle starts dispelled, so the tower is open without clearing any of the six trials. The trial rooms stay open and their chests and pots can still be checked. Experimental, untested in the game."),
    );
    s.push(
        Setting::toggle(crate::first_pickups::NUM, crate::first_pickups::SETTING, "world", "Skip first pickup text", false)
            .tip("No \"You got X!\" box the first time you pick up a rupee, a heart, a magic jar, arrows or Deku seeds. Chests and real items still show theirs."),
    );
    s.push(
        Setting::toggle(crate::area_intros::NUM, crate::area_intros::SETTING, "world", "Skip area intros", false)
            .tip("The short scene that shows off an area the first time you enter it is skipped."),
    );
    s.push(
        Setting::number(31, "trials", "goal", "Ganon's Trials", 6, 0, 6)
            .tip("How many of the six trials in Ganon's Castle must be cleared. Any of the six count: below 6, the barrier is dispelled once that many trials are cleared, whichever they are.")
            .when_off(crate::ganon_trials::SKIP),
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
            .tip("How many heart containers you begin with. Any value other than 3 is written into the game start module."),
    );
    s.push(
        Setting::choice(38, "shuffle_rewards", "shuffle", "Story and NPC rewards", "supported")
            .wire_as(Value::s("off"))
            .tip("Whether the items that NPCs, minigames, Great Fairies and story scenes hand out are shuffled. This is on by default. The Spiritual Stones and Medallions from the bosses and Rauru stay in place unless Dungeon Rewards is also set. Songs have their own setting.")
            .opt("off", "Vanilla", "These rewards keep their original items.")
            .opt("supported", "Supported rewards", "The rewards whose event the tool can edit."),
    );
    s.push(
        Setting::toggle(51, "shuffle_rewards_extra", "shuffle", "More NPC rewards", true)
            .wire_as(Value::Bool(false))
            .disabled_as(Value::Bool(false))
            .tip("Also shuffle the rewards that need bigger event edits: the 20 token Skulltula reward, Biggoron and Medigoron.")
            .when("shuffle_rewards", "supported"),
    );
    s.push(
        // Was a toggle: the choice order keeps old strings reading off as vanilla and on as anywhere.
        Setting::choice(74, "shuffle_dungeon_rewards", "shuffle", "Dungeon Rewards", "vanilla")
            .disabled_as(Value::s("vanilla"))
            .tip("Where the three Spiritual Stones and six Medallions that the bosses and Rauru hand out can be found.")
            .opt("vanilla", "Vanilla", "Every boss and Rauru give their usual Stone or Medallion.")
            .opt("anywhere", "Anywhere", "The Stones and Medallions can be anywhere, and the bosses and Rauru can give any item.")
            .opt("dungeons", "End of dungeons", "The Stones and Medallions are shuffled among the bosses and Rauru only: each of them gives one random Stone or Medallion.")
            .when("shuffle_rewards", "supported"),
    );
    s.push(
        Setting::toggle(53, "shuffle_trade_checks", "shuffle", "Adult trade quest checks", false)
            .tip("Each of the ten adult trade quest hand ins also gives one shuffled item the first time. The trade items themselves stay in their vanilla order.")
            .when("shuffle_rewards", "supported"),
    );
    s.push(
        Setting::toggle(65, "shuffle_cows", "shuffle", "Link's House and Gerudo Valley cows", false)
            .tip("The cow in Link's House and the cow in Gerudo Valley each give one extra shuffled item the first time their milk scene starts, with or without a bottle. The milk stays vanilla and stays repeatable. The other cows have their own setting.")
            .when("shuffle_rewards", "supported"),
    );
    s.push(
        Setting::toggle(66, "shuffle_cows_other", "shuffle", "Other cows", false)
            .tip("The six other cows each give one extra shuffled item the first time their milk scene starts, with or without a bottle: the Gerudo Fortress cow, the Death Mountain Trail and Hyrule Field grotto cows, the cow behind Impa's house and the two Lon Lon Ranch stable cows. The five ordinary cows become actor classes of their own. The milk stays vanilla and stays repeatable. Experimental, untested in the game.")
            .when("shuffle_rewards", "supported"),
    );
    s.push(
        Setting::toggle(93, crate::flow_extra::frogs::SONGS_SETTING, "shuffle", "Zora's River frog songs", false)
            .tip("The frogs in Zora's River give one shuffled item the first time you play each of Zelda's Lullaby, Epona's Song, Saria's Song, the Sun's Song and the Song of Time to them, in place of the rupees. The Song of Storms Heart Piece is a check of its own as before.")
            .when("shuffle_rewards", "supported"),
    );
    s.push(
        Setting::toggle(94, crate::flow_extra::frogs::BUG_SETTING, "shuffle", "Zora's River frog bug game Heart Piece", false)
            .tip("The first win of the frog bug game in Zora's River, which opens after all six frog songs, gives a shuffled item in place of the Heart Piece. Later wins still give a Blue Rupee.")
            .when("shuffle_rewards", "supported"),
    );
    s.push(
        Setting::toggle(90, crate::slice::silver::SETTING, "shuffle", "Silver rupees", false)
            .tip("The silver rupees of five rooms become items: the Forest and Light Trials in Ganon's Castle, the Ice Cavern push block room, the Bottom of the Well basement and the Gerudo Training Ground slopes room. Each room has five Silver Rupee items that are found in chests anywhere, and the room opens when you have found all five. The spots where the silver rupees hung hold other rupees. Needs Chests shuffled, because the Silver Rupee items are only found in chests. Experimental, untested in the game.")
            .when_not("shuffle_chests", "off"),
    );
    s.push(
        Setting::toggle(46, crate::flow_extra::potion::SETTING, "shuffle", "Market Potion Shop bottle and Red Potion", false)
            .tip("The Market Potion Shop keeper's free empty bottle and the Red Potion he pours as thanks for the escaped bug become two shuffled checks, and one more Bottle joins the pool. Both checks need a child who has met Zelda, by day, who carries an empty bottle, because while the free bottle holds another item he starts the bug tutorial only for a child with an empty bottle. His shelves still open after the tutorial.")
            .when("shuffle_rewards", "supported"),
    );
    s.push(
        Setting::toggle(54, "shuffle_scrubs_a", "shuffle", "One time scrub sales", false)
            .tip("Also shuffle the Business Scrubs that already sell one item once: the Death Mountain Crater heart scrub and the Lost Woods stick and nut bag scrubs. The price stays and the sale text still names the old item.")
            .when("shuffle_rewards", "supported"),
    );
    s.push(
        Setting::choice(crate::flow_extra::MERCHANTS, "shuffle_merchants", "shuffle", "Shuffle Merchants", "off")
            .tip("Whether the merchants sell a random item. Medigoron is the only one of All But Beans that is built; the Bean Merchant needs Skip Planting Beans.")
            .opt("off", "Off", "Medigoron and the Bean Merchant sell their usual items.")
            .opt("beans", "Bean Merchant Only", "The Bean Merchant sells a random item, when Skip Planting Beans is on.")
            .opt("no_beans", "All But Beans", "Medigoron sells a random item.")
            .opt("all", "All", "Medigoron, and the Bean Merchant when Skip Planting Beans is on, sell a random item."),
    );
    s.push(
        Setting::choice(crate::scrub_prices::NUM, crate::scrub_prices::SETTING, "shuffle", "Scrubs Prices", "vanilla")
            .tip("What the shuffled scrubs charge.")
            .opt("vanilla", "Vanilla", "The scrub's original price.")
            .opt("cheap", "Cheap Balanced", "0 to 95 rupees, favoring low prices.")
            .opt("fixed", "Fixed", "One price for every shuffled scrub."),
    );
    s.push(
        Setting::choice(crate::merchant_prices::NUM, crate::merchant_prices::SETTING, "shuffle", "Merchant Prices", "vanilla")
            .tip("What the shuffled merchants charge.")
            .opt("vanilla", "Vanilla", "The merchant's usual price.")
            .opt("cheap", "Cheap Balanced", "0 to 95 rupees, favoring low prices.")
            .opt("fixed", "Fixed", "One price for every shuffled merchant."),
    );
    s.push(
        Setting::number(crate::merchant_prices::FIXED_NUM, crate::merchant_prices::FIXED, "shuffle", "Merchant Fixed Price", 10, 0, 99)
            .tip("The price of every shuffled merchant when Merchant Prices is Fixed.")
            .when(crate::merchant_prices::SETTING, "fixed"),
    );
    s.push(
        Setting::choice(crate::fairy_prices::NUM, crate::fairy_prices::SETTING, "shuffle", "Great Fairy Prices", "vanilla")
            .tip("What the six Great Fairies ask as an offering before they grant their reward.")
            .opt("vanilla", "Vanilla", "The fairy's usual offering: 25, 50, 100, 200, 300 or 500 rupees.")
            .opt("cheap", "Cheap Balanced", "0 to 95 rupees, favoring low prices.")
            .opt("fixed", "Fixed", "One price for every Great Fairy."),
    );
    s.push(
        Setting::number(crate::fairy_prices::FIXED_NUM, crate::fairy_prices::FIXED, "shuffle", "Great Fairy Fixed Price", 10, 0, 99)
            .tip("The offering every Great Fairy asks when Great Fairy Prices is Fixed.")
            .when(crate::fairy_prices::SETTING, "fixed"),
    );
    s.push(
        Setting::toggle(crate::scrub_prices::HINT_NUM, crate::scrub_prices::HINT, "world", "Scrub Hint Text", false)
            .tip("Shuffled scrubs name the item they really sell. Off, they call it a Mystery Item. The price they say is always the price they take."),
    );
    s.push(
        Setting::number(crate::scrub_prices::FIXED_NUM, crate::scrub_prices::FIXED, "shuffle", "Scrubs Fixed Price", 10, 0, 99)
            .tip("The price of every shuffled scrub when Scrubs Prices is Fixed.")
            .when(crate::scrub_prices::SETTING, "fixed"),
    );
    s.push(
        Setting::toggle(56, "shuffle_scrubs_b", "shuffle", "Menu scrubs as one time sales", false)
            .tip("Also shuffle the Business Scrubs in the Lost Woods scrub grotto and in Jabu Jabu's Belly. A scrub that holds another item sells it once for 20 or 30 rupees instead of its menu, and its sale text still names a heart piece.")
            .when("shuffle_rewards", "supported"),
    );
    s.push(
        Setting::toggle(39, "shuffle_boss_hearts", "shuffle", "Boss Heart Containers", true)
            .wire_as(Value::Bool(false))
            .tip("Whether the Heart Containers that bosses leave behind join the item pool. The random item appears where the heart would, after the boss falls."),
    );
    s.push(
        Setting::choice(40, "shuffle_songs", "shuffle", "Songs", "off")
            .tip("Whether the songs you learn from NPCs and Sheik are shuffled. The lessons still play the usual melody scene. A song found anywhere else is handed out like an item with its get window and shows Sheik's harp. Scarecrow's Song is never shuffled. Experimental, untested in the game.")
            .opt("off", "Vanilla", "Every song comes from its usual teacher.")
            .opt("own", "Song locations", "The songs are shuffled among the places that teach songs.")
            .opt("anywhere", "Anywhere", "Songs can appear anywhere, also in pots, crates, freestanding spots and at Gold Skulltulas when those are shuffled, and song locations can hold any item."),
    );
    s.push(
        Setting::toggle(104, crate::song_models::SETTING, "shuffle", "Harp colour by song", true)
            .tip("A song found as an item shows Sheik's harp in the colour of that song on the ocarina menu, so you can tell the songs apart before you pick one up. Experimental, untested in the game.")
            .when("shuffle_songs", "anywhere"),
    );
    s.push(
        Setting::toggle(41, "gossip_hints", "world", "Gossip stone hints", false)
            .tip("Every gossip stone tells you where an important item lies. The stones need no Mask of Truth text change, only the words are replaced."),
    );
    s.push(
        Setting::multi(80, crate::npc_hints::SETTING, "world", "Character hints")
            .tip("Characters who know something tell you, in their own words, like the OoTR option of the same name. Altar: the Temple of Time altar says where each medallion rests. Dampé's diary: says where a Progressive Hookshot is. House of Skulltula: the cursed rich man names each token reward. Frogs: the Zora's River frogs name the prize of their ocarina game and of the rainy song. Great Fairies: each fairy names what she grants next to her price. Sheik: while waiting in the Temple of Time, Sheik says where the Light Arrows are. Malon: says where the Weird Egg went, only while the Kakariko gate starts closed. Mido: lets slip where the Kokiri Sword is. Saria: when you meet her in the Sacred Forest Meadow, she says where the Magic Meter is. Only the words change, in every language, with the hint itself in English. Experimental, untested in the game.")
            .opts(&[
                ("altar", "Temple of Time altar"),
                ("dampe_diary", "Dampé's diary"),
                ("skulltulas", "House of Skulltula"),
                ("frogs", "Zora's River frogs"),
                ("great_fairies", "Great Fairies"),
                ("sheik", "Sheik"),
                ("malon", "Malon"),
                ("mido", "Mido"),
                ("saria", "Saria"),
            ]),
    );
    s.push(
        Setting::toggle(42, "faster_chests", "world", "Faster item pickups", false)
            .tip("Shortens the waiting time in chest openings and item pickup scenes, and big chests open with the short small chest animation instead of the long big chest cutscene, like the OoTR option Fast Chest Cutscenes. The text, the item and the dungeon map, compass and boss key credit are the same."),
    );
    s.push(
        Setting::toggle(141, crate::qol::blocks::SETTING, "world", "Faster sliding blocks", false)
            .tip("Pushable and pullable blocks slide twice as fast."),
    );
    s.push(
        Setting::toggle(45, crate::chest_size::SETTING, "world", "Chest size matches contents", false)
            .tip("Chests holding major items, the items that can unlock progress, and boss keys become big chests, and chests holding anything else (rupees, ammo, hearts, maps, compasses and small keys) become small chests, like the OoTR option Chest Appearance Matches Contents. Only the plain wooden chests change; chests that run a story scene, invisible chests and the minigame chests keep their size. A big chest is wider than a small one, so a chest that grows may stand closer to a wall. Experimental, untested in the game."),
    );
    s.push(
        Setting::toggle(44, "skip_intro", "world", "Skip the opening cutscenes", true)
            .tip("A new game skips the opening movie and the wake up scene in Link's house, and you get control right away."),
    );
    s.push(
        Setting::choice(43, "start_reward", "starting", "Start with a reward", "off")
            .tip("Begin the game already holding one random Spiritual Stone or Medallion. The vanilla copy still exists.")
            .opt("off", "No reward", "You start with no stones or medallions.")
            .opt("stone", "A Spiritual Stone", "One random Spiritual Stone.")
            .opt("medal", "A Medallion", "One random Medallion.")
            .opt("any", "Either", "One random Stone or Medallion."),
    );
    s.push(
        Setting::choice(52, "shuffle_skulltulas", "shuffle", "Gold Skulltulas", "off")
            .tip("Whether Gold Skulltulas drop random items and their tokens join the item pool. A skulltula can drop items that the game has a pickup for and key items whose pickup pack is built, as a kit pickup that plays the get window. A killed skulltula stays gone only once its dropped item is picked up, and it comes back if the item is left behind.")
            .opt("off", "Vanilla", "Every skulltula drops its token.")
            .opt("pinned", "Confident matches", "Only the skulltulas whose remake actor is known for sure are shuffled.")
            .opt("matched", "Confident and pooled", "Also the pools of skulltulas that share the same logic, and the skulltulas that our hash research pairs with their spot. The skulltulas whose spot is still unsure only drop minor items such as rupees and hearts."),
    );
    s.push(
        Setting::toggle(57, "bombchus_in_logic", "world", "Bombchus in logic (experimental)", false)
            .tip("Experimental. Bombchus count as explosives in the seed logic, so walls, switches and enemies that need an explosive can be handled with Bombchus and no Bomb Bag. This follows the OoTR option that considers Bombchus in logic. Only the logic side is done here, the remake gets no separate Bombchu bag and no Bombchu drops. Caution: in our test of the remake, Bombchus only worked once a Bomb Bag and its ammo maximum were set, and a Bombchu pickup gave a count of 0 without them. The game is also reported to gate Bombchu use on a bag or a defeated Dodongo. A seed with this ticked may expect Bombchus where the game will not let you use them."),
    );
    s.push(
        Setting::choice(70, crate::qol::SKIP, "world", "Skippable cutscenes", "off")
            .tip("Removes the lock that stops the player from skipping some cutscenes. Most cutscenes can already be skipped by holding the skip button, only eight are locked.")
            .opt("off", "Vanilla", "The locked cutscenes stay locked.")
            .opt("story", "Opening and credits", "The opening movie and the first credits movie can be skipped. The six title screen demos stay locked.")
            .opt("all", "All locks", "Also unlocks the six title screen demos. Pressing the skip button on the title screen may then skip a demo."),
    );
    s.push(
        Setting::toggle(71, crate::qol::ANJU, "world", "Anju needs one cucco", false)
            .tip("Anju in Kakariko Village counts the cucco game as won when one cucco is in the pen instead of seven. The Bottle reward is the same. The logic needs one cucco too."),
    );
    s.push(
        Setting::toggle(72, crate::qol::POE, "world", "Big poe bottle at the first big poe", false)
            .tip("The Ghost Shop owner in the Market gives the Bottle for the first big poe, where the game wants five. The golden reward after ten big poes is unchanged. The logic needs one big poe."),
    );
    s.push(
        Setting::choice(78, crate::qol::NAVI, "world", "Return with Navi", "everywhere")
            .wire_as(Value::s("off"))
            .tip("Calling Navi with the D-pad asks whether to head back, like the original game restarting a saved file at the dungeon entrance, in Kokiri Forest or in the Temple of Time. Needs no item. The D-pad calls Navi from the start of a new game, and her room tutorial hints are replaced by the question; enemy hints while targeting stay. Boss rooms, miniboss rooms, the boss rush and the final battle never offer it, since leaving a fight that way left the save button disabled. Experimental, untested in the game.")
            .opt("off", "Off", "Navi stays quiet when she has no hint.")
            .opt("dungeons", "Dungeons", "Inside a dungeon, Navi offers a trip back to the dungeon entrance.")
            .opt("everywhere", "Dungeons and overworld", "Also outside dungeons: back to Kokiri Forest as a child, or the Temple of Time as an adult."),
    );
    s.push(
        Setting::choice(79, crate::qol::BUNNY, "world", "Bunny Hood effect", "off")
            .tip("Gives the Bunny Hood the speed boost it has in Majora's Mask, like the OoTR option of the same name. Works for child and adult Link. Done in the player's AI graphs, with no code patch: while the hood is worn, Link's forward speed is multiplied by 1.5 each frame, up to a cap that is higher for adult Link. Experimental.")
            .opt("off", "No effect", "The Bunny Hood is only a mask, as in the original game.")
            .opt("speed", "Faster running", "Link runs 1.5 times as fast while wearing the Bunny Hood.")
            .opt("jump", "Faster running and longer jumps", "Link also leaves the ground 1.5 times as fast when jumping, so jumps go further."),
    );
    s.push(
        Setting::toggle(81, crate::open_world::EPONA, "world", "Skip Epona's race", true)
            .tip("Learning Epona's Song is enough to own Epona, with no ranch visit and no race against Ingo. The first time adult Link plays the song where a horse can be ridden, Epona is registered as his horse and comes, and the Epona quest is marked complete. The logic counts Epona's Song as Epona when this is on, and needs Ingo's race at the ranch when it is off. Experimental, untested in the game."),
    );
    s.push(
        Setting::toggle(82, crate::open_world::MASKS, "world", "Skip the mask trading quest", false)
            .tip("The Happy Mask Shop quest starts complete, like the OoTR option Complete Mask Quest: once the shop opens with Zelda's Letter, every mask can be borrowed right away, including the Mask of Truth, without selling the Keaton, Skull, Spooky and Bunny masks first. Selling masks for rupees goes away, which costs no check. Experimental, untested in the game."),
    );
    s.push(
        Setting::toggle(83, crate::open_world::BEANS, "world", "Magic beans planted", false)
            .tip("Every magic bean patch starts planted, like the OoTR option Plant Magic Beans, so the adult bean platforms are there from the start. The Magic Bean leaves the item pool, and the bean salesman in Zora's River has one sale left, for 60 rupees, holding a random item when NPC rewards are shuffled. Experimental, untested in the game."),
    );
    s.push(
        Setting::toggle(84, crate::open_world::BRIDGE, "world", "Gerudo Valley bridge repaired", false)
            .tip("Opens the way to Gerudo Fortress: the broken bridge in Gerudo Valley is repaired from the start, so adult Link can cross without Epona or the Longshot. The carpenters in the Thieves' Hideout are not rescued, so the guards, the rescue and the Gerudo Membership Card stay as they are. Experimental, untested in the game."),
    );
    s.push(
        Setting::toggle(73, "shuffle_treasure_reward", "shuffle", "Treasure Chest Game reward", false)
            .tip("Whether the final reward of the Market Treasure Chest Game, a Piece of Heart, holds a random item. Only the first win gives it, later wins give the usual red rupee. The five rooms keep their keys and rupees. Only unique items that do nothing when found twice can be placed there, so no upgrades, ammo, bottles or keys."),
    );
    s.extend(crate::shops::plan::settings());
    s.push(
        Setting::choice(85, "shuffle_entrances", "shuffle", "Entrances", "off")
            .tip("Which doors and transitions lead somewhere else. Entrances are coupled, so the way back always returns you to the door you came in by. Experimental, untested in the game. Only Off and Dungeons work in this build.")
            .opt("off", "Vanilla", "Every entrance leads where it always does.")
            .opt("dungeons", "Dungeons", "The eleven dungeon doors lead to other dungeons: Deku Tree, Dodongo's Cavern, Jabu Jabu's Belly, Bottom of the Well, Ice Cavern, Gerudo Training Ground and the Forest, Fire, Water, Shadow and Spirit Temples. Ganon's Castle, boss rooms, blue warps and warp songs stay. Experimental, untested in the game.")
            .opt("overworld", "Overworld", "The area to area transitions of the overworld lead to other areas. Not built yet, a seed with this is refused.")
            .opt("everything", "Everything", "Dungeons, overworld transitions and later interiors and grottos, each kept in its own pool. Not built yet, a seed with this is refused."),
    );
    s.push(
        Setting::toggle(crate::deku_bridge::NUM, crate::deku_bridge::KEY, "world", "Deku Tree bridge never breaks", false)
            .wire_as(Value::Bool(false))
            .tip("The wooden bridge in the Deku Tree slingshot room stays up when Link walks, rolls or jumps on it, so you never drop into the pit below it. Nothing else in the dungeon changes and the logic is the same, because the room never needed the bridge to break. Without the Slingshot the jump across has a window of about nine frames, so this keeps the room fair when the chests there hold other items. A bridge that already broke in a save stays broken. Experimental, untested in the game."),
    );
    s.push(
        Setting::toggle(crate::debug_names::NUM, crate::debug_names::SETTING, "world", "Show location names", false)
            .tip("For checking the location data. After an item get, a text box shows the internal name of the location the item came from, such as [KF Midos Top Left Chest]. Covers chests, NPC, song and event rewards, Gold Skulltulas and items on the ground. Items on the ground need a button press, and a placed item missing from the location list says so. Experimental."),
    );
    s.push(starting_items());
    s.push(starting_songs());
    s.extend(difficulty());
    s.sort_by_key(|x| x.num);
    Schema {
        version: SCHEMA_VERSION,
        groups: vec![
            group("world", "World", "How open the world is and who you start as."),
            group("shuffle", "Shuffles", "Which kinds of checks hold random items."),
            group("keys", "Keys", "Where dungeon keys and maps can appear."),
            group("goal", "Goals", "What you need to do to finish the game."),
            group("starting", "Starting items", "What you begin the game with."),
            group("difficulty", "Difficulty", "How hard enemies hit and how much help the game hands out."),
        ],
        settings: s,
    }
}
