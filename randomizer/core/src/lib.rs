#[cfg(test)]
macro_rules! parts {
    ($run:ident, $($name:ident = $k:expr),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                $run($k)
            }
        )+
    };
}

pub use ootrr_game::{bfevfl, bfres, byml, catalog, catalog_json, chest_slot, dungeon_text, entrance_swap, entrances, error, flags, flags_build, flags_gen, flags_harvest, flags_keyed, flags_map, flags_names, flags_rel, flags_scan, ips, items, json, modpack, msbt, murmur, placement, rng, romfs, rsizetable, sarc, save, tex, zs};
pub use ootrr_logic::{fill, logic_demo, logic_items, logic_json, logic_synth, logic_val, rule_ast, rules, search, spoiler, tier_chain, world};
pub use ootrr_cosmetics::{colors, cosmetic, sfx, slink, title_logo};

pub mod ainb;
pub mod boss_heart;
pub mod calibrate;
pub mod catalog_free;
pub mod chest_size;
pub mod container;
pub mod cow_clone;
pub mod debug_names;
pub mod deku_bridge;
pub mod difficulty;
pub mod always;
pub mod bombchu_open;
pub mod door_fix;
pub mod door_gate;
pub mod water_gate;
pub mod door_open;
pub mod dungeon_credit;
pub mod dungeon_grant;
pub mod entr;
pub mod event_chests;
pub mod fast;
pub mod gerudo_card;
pub mod malon_song;
pub mod flow_locations;
pub mod flow_extra;
pub mod flow_sim;
pub mod hints;
pub mod intro_skip;
pub mod gift_keys;
pub mod item_window;
pub mod lacs_key;
pub mod medal_gate;
pub mod nocturne_gate;
pub mod prelude_gate;
pub mod npc_hints;
pub mod rainbow_bridge;
pub mod open_world;
pub mod ganon_trials;
pub mod pickup_pack;
pub mod pickup_persist;
pub mod pot_slice;
pub mod pots;
pub mod pregrant;
pub mod qol;
pub mod progressive;
pub mod required;
pub mod bomb_lift;
pub mod bomb_pickup;
pub mod fairy_prices;
pub mod fire_gorons;
pub mod merchant_prices;
pub mod scrub_prices;
pub mod scrubs_a;
pub mod scrubs_b;
pub mod settings;
pub mod shelf;
pub mod shops;
pub mod first_pickups;
pub mod area_intros;
pub mod short_get;
pub mod slice;
pub mod song_lessons;
pub mod song_models;
pub mod special;
pub mod start_defaults;
pub mod start_items;
pub mod start_reward;
pub mod start_state;
pub mod starter_bags;
pub mod no_tutorials;
pub mod ocarina_hold;
pub mod story_skip;
pub mod songs;
pub mod spots;
pub mod staging;
pub mod teleport;
pub mod teleport_dest;
pub mod teleport_flow;
pub mod teleport_merge;
pub mod teleport_sim;
pub mod teleport_start;

pub const TITLE_ID: &str = "0400486024DEC000";
