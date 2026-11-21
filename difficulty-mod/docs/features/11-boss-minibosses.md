# Bosses spawn minibosses

Setting `boss_minibosses` (number 120), checkbox. Code: `crates/core/src/difficulty/minibosses.rs`. Built by a sub-agent on `feature/boss-minibosses` (`fd20c72`).

## Request
"bosses spawn minibosses"

## Interpretation
Each dungeon boss fight also has one strong enemy in the room. I told the agent a solid result for a few bosses beats a fragile one for all.

## Investigation
- The boss heart shuffle in the repo already knows the boss rooms, which was the starting point.
- Each boss room is run by a logic graph. In all five covered rooms the heart container and exit warp are created from "boss defeated" flags, never from an enemy count, so a live extra enemy should not block them.
- The game has 193 logic nodes of the simple "create this actor, delete it on a signal" kind, which gives an exact shape to copy.

## Implementation
Two files per boss:
- **Dungeon map (`_Static`):** one new enemy in the boss's room, added to the group that runs the room.
- **Room logic graph:** one new node that has the enemy present from entry and deletes it on the same signal that creates the heart container.

| Boss | Miniboss |
|---|---|
| Queen Gohma | Stalfos (child version) |
| King Dodongo | Lizalfos |
| Barinade | Stalfos (child version) |
| Phantom Ganon | Stalfos (weak) |
| Twinrova | Dinolfos |

Skipped: Volvagia, Morpha and Bongo Bongo (no safe floor), Ganondorf and Ganon (scripted), and the boss rush copies.

## How it was tested
- rc 13 (19 Nov, script): the Deku Tree loads with the mod (no black screen). Right after arriving the actor list holds the Stalfos at (16.5, 47.2, 74.0), hp 18, before any door scene; whether it is visible or active then was not seen. The door walk is NOT TESTED (capture playbook).
- 21 difficulty tests including a game-file test per boss room.
- A real build: all five maps and graphs carry the edit alongside the boss heart edit, the graph audit is clean, `flow-loads` is clean.

## Result of the scripted load test (18 Nov)
- With this setting on, **Dodongo's Cavern went to a black screen** in rc 4 (reproduced with this setting alone; the cavern loaded with every other setting on). No crash line in the log; the game kept running but Link never appeared.
- The Deku Tree, Jabu-Jabu, Forest Temple and Spirit Temple loaded with it on.
- King Dodongo was removed from the list in rc 5. Which of his three edited files is at fault (the added Lizalfos, the room logic graph, or the map file) was not isolated.
- Loading is all that was checked: nobody has seen a miniboss in a boss room yet.

## In the game
Never played by hand. To test:
- Turn this on alone and go to Gohma (shortest route).
- Pass: a Stalfos is in the room, the fight is winnable, and after Gohma dies the Stalfos vanishes and the heart container and blue warp appear.
- Report: a crash entering the dungeon (the graph edit is the suspect), the miniboss attacking during the intro cutscene, or it standing in a bad place (King Dodongo's lava ring, Barinade's arena).

## Notes
- This is the first time the repo adds a whole node to a logic graph, and the first appended actor in a `_Static` map. If loading the dungeon crashes, that is why.
- The miniboss is in the room before the intro triggers. Whether cutscenes pause it is inferred, not seen.
- In Jabu, Forest and Spirit the delete signal is a quest step. If a seed starts past that step, the miniboss never appears.

## Dodongo black screen: code-only investigation (18 Nov, no game run)
Cause not proven. What was ruled out by dumping the real files:
- The BYML writer is faithful: parse then write of all five `_Static` files and Dodongo's `_Dynamic` is byte-identical.
- The new hash is unique, the graph module `Gimmick_DodongoCavern_BossBattle_75a7` is used by exactly one group in one map, and the new actor has the same keys as the four spots that load.
- Not a "static has none of this actor" problem: the Deku Tree and Jabu-Jabu also have no Stalfos (child) in any of their files and load.

What is unique to the Dodongo edit (ranked hypotheses):
1. The cavern uses only `Enemy_MiddleBoss_Lizalfos` (4 in `_Static`, with `Links`, `Name`, `IsIntendedForEventAppear` or `IsEnterLandBattleOnInitialization`). Plain `Enemy_Lizalfos` is in no Dodongo file and in only one other map (Spirit Temple `_Static`). Its pack may depend on something the cavern does not load.
2. The spot is `[14.8, 74.0, -48.5]`, chosen by hand; the boss stands at `[33.2, 73.49, -77.6]` over lava. A bad spawn point is a possible cause of a stuck load.
3. Dodongo's boss group has the most nested `AiGroup` references (6) and a duplicate `Area_663f` id; a reference appended after them might not be handled the same way.
4. Dodongo's `_Static` has no `PlgGdTable` (the other four have one). This is vanilla, so a cause only if the edit interacts with it.

Experiment (build four `DodongoCavern_Static` variants, each alone, no graph edit, one launch each):
- A: actor `Enemy_Lizalfos` appended, no AiGroup reference. Black means hypothesis 1 or 2.
- B: AiGroup reference appended, no actor. Black means hypothesis 3.
- C: as A but `Enemy_MiddleBoss_Lizalfos` copied with the keys of a vanilla one. Loads means hypothesis 1.
- D: as A but `Enemy_Keese` (used in the cavern already) at the same spot. Black means position or "append to this file" (2 or other).
Other features: `minions.rs`, `extras.rs`, `ambush.rs` and `gates.rs` clone or add actors whose kind is already in the same map, mostly in `_Dynamic`; only `minibosses.rs` appends a kind new to the map to a `_Static` file. Load-test Deku Tree, Jabu-Jabu, Forest and Spirit Temples again with the final build and walk into the boss rooms.

## Code review reading of the Dodongo black screen (18 Nov, no game run)
From `docs\code-review-2026-11-18.md`, open problem (b). Not tested; nothing was changed.
- A hang with no crash line fits an actor that never becomes ready, not a corrupt file. Two things differ from the four spots that load.
- The kind. The cavern only ever uses `Enemy_MiddleBoss_Lizalfos`, placed with `Links`, a `Name` and a battle-start key. Plain `Enemy_Lizalfos` is in one vanilla map (Spirit Temple). It probably waits for a partner or arena link that our bare entry (room index only) does not give. A Stalfos needs nothing, which is why the other spots load.
- The spot. It was about 34 units from the boss's own position, against 5 to 9 units for the four spots that work, so it may lie outside the boss room whose index it carries.
- Experiment: variant D above (a Keese at the same spot) separates the two. If D loads, the kind is the cause; if D is black, the spot or appending to this file is. Also copy the full vanilla entry of the Spirit Temple's plain Lizalfos and compare its keys with ours (variant C).
- Simplest way out: use a kind that already loads with a bare entry (Stalfos or Dinolfos) at a spot checked against the room.

## Seen in the game, 19 Nov, and the change (feature/boss-adds)
- First time seen: with this and "Bosses spawn minions" on, in Gohma's room. The user: "the keeze and stalfos are in the room, but can we defer spawning them 'till after the fight starts?" The Stalfos was placed in the map, so it stood in the room and could attack before Gohma's introduction had played.
- Change: the miniboss is now created on a signal and is not placed at load. Per room:
  - **Gohma:** the room graph runs the boss through a module node (`Enemy_DungeonBoss_Gohma_Prefab_b75b`). That module has one output, `End_TrapEvent`, the end of the scene where the door shuts behind Link. Gohma's own introduction starts right after it (the boss waits for the same door flag, then plays its appear event). The Stalfos node's create input is now wired to `End_TrapEvent`. The node still leaves on the flag that brings the heart container. This is the closest signal the room graph offers: it is not "after the introduction", it is as the introduction starts. The game pauses and hides actors that are not in a scene during the boss introduction (not seen), so the Stalfos should first move when the fight starts.
  - **Barinade, Phantom Ganon, Twinrova:** left as they were. Their prefab modules have no output at all, and the flags their room graphs read (`Barrinade_doorFlag`, `ForestTemple_SageOfForest_BossAppearance`, `SageOfSpirit_BossIn`) are set at the start of the appearance scene or when the player walks in, so they are no better than creating it at load. The only fight-started signal found is the boss's own `IsBattle` output inside the prefab, and wiring to it would mean editing a graph that the dream world copy of the fight shares. Not done without a way to test it.
- Testing the Gohma room: walk through the boss door. Warping into the room skips the door scene, so the signal never fires and the Stalfos never appears.

## Fight start, 20 Nov (feature/boss-adds-start)
The user (20 Nov): the extra enemies must appear when the fight starts, not before; for Gohma that is after Link looks up at her and her introduction plays.

What the data says, per room (read from the room graphs and the boss flows, no game run):
- **Why the adds were listed from map load.** They are placed actors in `_Static`, in the boss room's logic group. The 20 Nov run warped past the boss door, so `End_TrapEvent` never fired and the room logic never created them, yet the bridge listed them, still and at full health, the whole time. Gohma's own larvae were listed the same way, all three at Gohma's spot. So the list shows actors the map holds even when the logic has not created them: "listed" is not "there".
- **What marks fight start.** Each boss's appearance is an entry of its flow (`Get_StoneOfForest` `Boss_Appear` for Gohma, `StoneOfWater_BossAppear`, `SageOfForest`, `SageOfSpirit_BossStart`); it plays the intro `Dm_<dungeon>_BossIn` and then the fight runs. Gohma's entry is requested by her own AI when Link looks up; it first sets `Gohma_IsLightingForBattle` and then plays `Dm_Deku_BossIn`. The game marks the intro as played in `HavePlayedEvent.Dm_<dungeon>_BossIn` (saved), and Gohma's room graph already reads `HavePlayedEvent.Dm_Deku_BossIn` to move the room to its fight step. The boss actor's `IsBattle` output exists but no vanilla graph uses Gohma's, and it sits inside the prefab module the dream world stage shares. Flags set by the other flows on the way in (`Barrinade_doorFlag`, `ForestTemple_SageOfForest_BossAppearance`, `SageOfSpirit_BossIn`) are set as the appearance starts; `Barrinade_ChageEnvPallet` is not even saved.
- **Story skip and the shorter cutscenes:** no upstream setting touches the boss intros (story skip only shortens the endings); "Skippable cutscenes" only lets the player skip, and a skipped intro has still been started.

Change: the miniboss node's create input is now the reader of `HavePlayedEvent.<intro>` in every covered room (Gohma reuses the reader her graph has; the others get one new `HoldGameDataGetBoolSimple`). It still leaves on the flag that brings the heart container, and that wins over create (as the game's heart container shows: it is created on the boss flag and deleted once obtained), so nothing comes back on a later visit. Retry: if no save happened after the intro the flag is false again on reload and the intro brings the adds again; if the game saved, they are created at load (see the plan, step iv).

Tests: 3 new unit tests (reuses the room's reader; adds one reader shared by every node; every covered boss has its own intro), the game-file test checks each room's node, its reader and that the boss flow plays the intro, and a new game-file test checks no other map uses an edited room graph. Not played. Plan: `<playtest folder>\boss-adds-start\PLAN.md`.

How to test in the game: fresh file, walk into Gohma's room. Before you look up: no Stalfos on screen. Look up: after the introduction the Stalfos walks at you. Die, Try Again: one Stalfos again, with the fight. Beat Gohma: it vanishes; come back later: none.

## Release (rc 16, 20 Nov)
In rc 16 (branch `release/rc16`, on top of upstream 0.8.0). Menu text: "A Stalfos or a Dinolfos joins the fight with Gohma, Barinade, Phantom Ganon and Twinrova. It appears when the fight starts and leaves when the boss is beaten. Played with Gohma and Phantom Ganon; the other bosses are untested."

- **Played by hand** (hands-on sessions 3 and later, 20 Nov): Phantom Ganon and Gohma, the adds appearing with the fight.
- **Played by script** (round 3, 20 Nov): Phantom Ganon and Twinrova; the adds stand still through the intro and attack within 1 s of the fight starting. Try Again reloads the dungeon entrance; walking in replays the intro and the adds come back once, with no doubling.
- **Not played:** Barinade (a warp does not start his intro).


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 11):** Bosses spawn minibosses
- **State:** Built, in the game once
- **What was seen in the game:** Stalfos seen in Gohma's room standing there before the introduction (19 Nov); now created as the door-shut scene ends in Gohma's room, the other three rooms unchanged rc13 (19 Nov): Deku Tree loads (not black). Stalfos listed at (16.5, 47.2, 74.0) right after the map loads, before any door scene; whether it is visible or active then is not seen. Door walk NOT TESTED (capture playbook).
- **Next step:** Walk through the Gohma door on feature/boss-adds (build in <playtest folder>\boss-adds); Dodongo stays out


## Moved from STATUS.md (20 Nov): session results (round 3, rc 16 scripted checks)

| **Round 3 (agent 14, done)** | `<playtest folder>\round3-1120\RESULTS.md`, 15 pass, 1 fail.
  **Boss adds at fight start (`feature/boss-adds-start`), seen:** Phantom Ganon and Twinrova. The adds stand still through the intro and attack within 1 s of the fight starting (video).
  **Try Again** reloads the last autosave, the dungeon entrance, never the boss room. Walking in replays the intro, and the adds come back once, with no doubling.
  **FAIL: the Shadow Temple does not load with `boss_minions`** (black screen, game frozen; it loads without mods). The cause is the Keese moved into the static map, the same as the King Dodongo case. Move that room back to the dynamic map before merging.
  **Not tested:** Barinade (a warp does not start the intro); skipping an intro by button (no bridge button skips it: needs a controller, in the playbook); Volvagia, Morpha, Bongo Bongo.
  **Gates only with a sword, seen:**
  - gates 9 and 2 absent without the sword, present after;
  - a Kokiri Sword from Starting items works;
  - the gate appears at once when the sword is given, without a reload.
  **Gate kill sequence:** inconclusive; a hit that knocks Link out of the trigger box takes the gate away.
  **Volvagia stage 1, seen:** the Market loads; she rises at 18 units, circles and breathes fire, with normal battle music and no event. The user says the Market is a bad spot: redesign stage 1 elsewhere |

- **rc 16 scripted checks (20 Nov evening): no FAILs, safe to merge** (`<playtest folder>\rc16-tests\RESULTS.md`). Shadow Temple loads 3 of 3 with boss adds on; every other temple loads, also at 10x with gates and entrance shuffle. Boss adds start with the fight for Volvagia, Bongo Bongo and Twinrova (video). Barinade inconclusive (a warp can't start his intro). Morpha not tested: a warp into the arena hangs on the door camera, also without our mod, so it needs a door walk with the boss key. Fireball storms: the other two spots work, child and adult, first hit 4.6 to 5.5 s. Gates under dungeon shuffle: 3, 4, 5, 9 present, 1 and 2 absent; gate 6 needs a closed-forest seed to tell. Jabu: 12 Octoroks in open water in the shots seen.
