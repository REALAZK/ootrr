# Bosses spawn minions

Setting `boss_minions` (number 121), checkbox. Code: `crates/core/src/difficulty/minions.rs`. Built by a sub-agent on `feature/boss-minions` (`4966a8f`).

## Request
"bosses spawn minions"

## Interpretation
Each dungeon boss fight also has a few weak enemies in the room, chosen to suit the arena.

## Investigation
- **King Dodongo already does this.** His parameter file has `SpawnBabyDodongoParams`: how many Baby Dodongos he calls per roll at each health stage (0, 2, 3 in vanilla).
- For the others, free-standing enemies in dungeon `_Dynamic` maps need no logic or group entries, so placing flying enemies in the boss room is the simplest mechanism.
- Boss intro and outro events use a setting that pauses and hides everything not in the scene, so placed minions should not act during them.
- The warp opens from the boss's own death, not an enemy count (the agent read Gohma's graph in full).

## Implementation
| Boss | Minions |
|---|---|
| Gohma | 3 Keese |
| King Dodongo | Baby Dodongos 2/3/4 per roll, up from 0/2/3 |
| Barinade | 3 Biri |
| Phantom Ganon | 3 Keese |
| Volvagia | 3 Fire Keese |
| Morpha | 3 Keese, above the water |
| Bongo Bongo | 3 Keese, above the drum |
| Twinrova | 2 Fire Keese, 2 Ice Keese |

Placed minions sit in a ring 5 to 8 m around the arena centre, 1.5 to 4 m up. They start no battle music of their own. The build stops with a clear error if a boss is not where it was measured.

Skipped: Ganondorf and Ganon (a soft-lock there costs the whole seed).

## How it was tested
- rc 13 (19 Nov, script): the Deku Tree loads with the mod. The three Keese are in the actor list from map load at (20.0, 50.5, 62.8), (15.7, 50.5, 70.3), (24.3, 50.5, 70.3). Door walk NOT TESTED.
- 20 difficulty tests including a game-file test.
- A real build: all 7 maps hold the minions, Dodongo's values are 2/3/4, `flow-loads` is clean.

## In the game
Never played. To test:
- Start with King Dodongo: it uses the game's own mechanism and is the safest.
- Then Gohma: 3 Keese should be flying around the room.
- Pass: minions present, fight winnable, heart container and warp appear after the boss dies.
- Report: Keese stuck in geometry, minions attacking during the intro, or minions hitting you at the warp afterwards.

## Notes
- Placed minions are not tied to the boss. They are there before the intro, stay after the boss dies, and are probably back on a later visit.
- Keese do not normally exist in the Deku Tree or Forest Temple, so those rely on the game loading the actor on demand.
- A plain Keese placed in mid-air has no vanilla example (vanilla ones hang from a surface); Fire Keese in mid-air do.
- Lesson: check whether the boss already has a spawn mechanism in its own parameters before placing anything.

## Seen in the game, 19 Nov, and the change (feature/boss-adds)
- First time seen, in Gohma's room (with "Bosses spawn minibosses" on). The user: "the keeze and stalfos are in the room, but can we defer spawning them 'till after the fight starts?" The Keese were placed in the dynamic map and were flying around before Gohma's introduction.
- Change for Gohma: the three Keese are no longer placed in the dynamic map. They are added to the static map in the same logic group as the boss and the room graph gets one node per Keese whose create input is the module output `End_TrapEvent` (the end of the door-shutting scene, as Gohma's introduction starts; see feature 11 for why this is the closest signal). They are not deleted when the boss dies, as before. Because the signal does not fire again once the boss is beaten, they should not come back on a later visit.
- Other rooms (Barinade, Phantom Ganon, Volvagia, Morpha, Bongo Bongo, Twinrova): unchanged. Their room graphs expose no fight-started signal (see feature 11), so the minions are still placed in the dynamic map.
- Testing the Gohma room: walk through the boss door. Warping into the room skips the door scene, so the Keese never appear.

## Fight start, 20 Nov (feature/boss-adds-start)
Same requirement and mechanism as feature 11 (see its "Fight start" section for what the data says). Changes:
- **Every room** now uses it, not only Gohma: the minions of Barinade, Phantom Ganon, Volvagia, Morpha, Bongo Bongo and Twinrova moved from the `_Dynamic` map to the `_Static` map, in the boss room's logic group, each with a room-graph node created by `HavePlayedEvent.<boss intro>`. All seven room graphs took the edit in the game-file tests (the boss sits in one logic group with one heart container in each).
- The minions now **leave when the boss is beaten** (delete on the heart container's flag, as the miniboss), so they are not at the warp and not there on a later visit.
- New in the load list: the Fire, Water and Shadow Temples now carry new `_Static` actors (Fire Keese, Keese, Keese). Load-test them first; if one goes black, put that room's minions back in `_Dynamic`.

Not played. Plan: `<playtest folder>\boss-adds-start\PLAN.md`.

How to test in the game: as feature 11. In the rooms where the intro starts on entry, the Keese should show up as the intro ends and fly at you; none before.

## Release (rc 16, 20 Nov)
In rc 16, with the Shadow Temple fix from `test/handson2`: the scripted round 3 found the Shadow Temple loading to a black screen with this setting on (it loads without mods), caused by Bongo Bongo's Keese moved into `ShadowTemple_Static`. They now stay in `ShadowTemple_Dynamic` with no start condition, so in that room they are there from the start, not from the intro (unit test `the_shadow_temple_static_map_is_not_edited_by_minions`). The temple has not been loaded in the game with the fix yet. Menu text: "Bosses bring a few Keese or Biri, and King Dodongo calls more Baby Dodongos. They appear when the fight starts (Bongo Bongo's are there from the start) and leave when the boss is beaten. Ganondorf and Ganon are unchanged. Played with Gohma and Phantom Ganon; the other bosses are untested."

- **Played by hand:** Gohma and Phantom Ganon.
- **Played by script:** Phantom Ganon and Twinrova (see feature 11).
- **Not played:** Barinade, Volvagia, Morpha, Bongo Bongo, and the Shadow Temple load with the fix.


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 12):** Bosses spawn minions
- **State:** Built, in the game once
- **What was seen in the game:** Keese seen in Gohma's room before the introduction (19 Nov); Gohma's Keese now appear as the door-shut scene ends, the other rooms unchanged rc13 (19 Nov): the 3 Keese are listed at map load too (20.0/15.7/24.3, 50.5, 62.8/70.3/70.3); door walk NOT TESTED.
- **Next step:** Walk through the Gohma door on feature/boss-adds; then Dodongo's babies and the other rooms


## Moved from STATUS.md (20 Nov): pointer

Scripted and by-hand results for the boss adds are under feature 11.
