# More enemies (slider, 1x to 10x)

Setting `more_enemies` (number 119), now a slider like enemy health: 1x (off, default) to 10x in steps of 0.5x; the settings string stores a percentage (100 to 1000). Code: `crates/core/src/difficulty/extras.rs` (copies, up to 2x), `difficulty/ground.rs` (navigation mesh reader), `difficulty/horde.rs` (everything above 2x). Branch `feature/horde` from `playtest/next`, worktree `wt\horde`. Exe: `<playtest folder>\horde\ootrr.exe`. Built by an Opus agent on 19 Nov with no game run.

## Request
- First: "more enemies (spawn extras)".
- 19 Nov: "figure out some way using the map data, and automated agent testing, to identify good locations to add extra enemies. then go totally overboard. I want the 'extra enemies' slider to go from 1x to 10x. when at 5x, I want enemies that usually would never be in an area to be in an area. look where I put those likelikes in lake hylia as a reference. that, plus flying enemies, plus water enemies, plus minibosses, etc. figure out what works and make use of it. if anyone's crazy enough to play on 10x it should feel like playing hyrule warriors."
- Then: "from 1.5x to 10x, and add whatever enemies are 'fun' to add. doesn't have to only add out-of-region enemies at 5x. so far in my testing the main thing I noted was that many enemies only 'work' on flat-ground likelikes are fine so long as it's not too steep. but some enemies are just no good on slopes at all. so figure that out and place enemies accordingly."

## Interpretation
- The number is "about this many times the usual enemies in each area". 1x changes nothing; 1.5x and 2x keep the old Half again and Double behaviour exactly (same copies, byte for byte).
- Above 2x, new enemies are placed on real walkable ground read from the game's own navigation mesh, with each enemy kind limited to the ground it can handle.
- Rising with the value: more of the area's own kinds near where they already live, then fliers and Octoroks (3x), then any suitable kind, groups, and minibosses (5x and up). At 10x: groups everywhere, capped for safety.

## How it works

### Up to 2x (unchanged)
`extras.rs`: copies of plain enemies (21 kinds, nothing linked to them) 1 to 1.5 units away. 1.5x copies about half of them, 2x all of them. Above 2x this step still runs at full (every plain enemy copied), then the horde step fills the rest.

### Ground method: the navigation mesh (not an approximation)
Every map has a Havok navigation mesh: `Phive/NavMesh/<Map>.Nin_Ounce_NVN2.bphnm.zs` (78 files), or for Hyrule Field 256 tiles in `Phive/StitchedNavMesh/<Map>/X<i>_Z<j>...`. `ground.rs` reads the tag file inside (sections, type names, type layouts, item table), finds the `hkaiNavMesh` object and its faces, edges and vertices. Stitched tiles are 250 units wide in local coordinates; world = local + ((i - 8) x 250 + 125, 0, (j - 8) x 250 + 125) (fitted against Tektites and grass: 201 of 293 test points land on the mesh with this offset, 9 to 28 with the others). Empty filler tiles (one face at height 0) are dropped.

What it answers for a point:
- **Walkable floor and its height** (several floors at one x, z are kept apart).
- **Slope**: the face's own tilt, and the worst rise over a ring of 8 points at a class-dependent radius (also fails if any ring point is off the mesh, so edges and drops count as steep).
- **Room**: horizontal distance to the nearest open mesh edge (wall or drop) within 3 units of height, up to 12.
- **Connected patch** (faces joined through their edges) and its area.

How good it is: every placed vanilla ground enemy sits on the mesh within half a unit (Deku Babas, Like Likes, Stalfos, Wolfos, Iron Knuckles, Dodongos, Tektites: median offset 0.0). The mesh is the game's own "where an enemy may walk", so it already excludes walls, deep water and lava. It does not know ceilings or water depth.

Things the mesh alone gets wrong, and the extra rules:
- Some maps have huge flat planes under or around the world. **In bounds** = the face is no larger than 20,000 square units, its patch is at least 150 in area, at least 3 non-scenery actors of the map stand on that patch, and some actor of the map (grass, rock, pot, enemy, NPC...) is within 24 units horizontally and 12 vertically of the spot. Actors at the map origin are ignored (managers sit there).
- **Water**: the mesh has no water. Water spots are 3 to 15 units from a vanilla Octorok, at the Octorok's height, where there is no floor within 3 units up or down. So Octoroks spread through Lake Hylia (child), Zora's River and Fountain, Gerudo Valley; depth is not known.
  - **Fix after the rc 15 report (Jabu-Jabu, 19 Nov).** At 10x, Octoroks in the business scrub room were under a ceiling and one was inside a wall: "no floor within 3" is also true of rock, walls and the void beside a pool, and the 3 to 15 ring reaches past a small pool. A water spot (and each group member) now also needs (1) a walkable floor within 4.5 units horizontally in a dungeon, 9 outdoors (every vanilla Octorok stands at the shore, 0.5 from the mesh; the Lake Hylia ones are up to 7.5 out), and (2) no walkable floor within 10 units above it (`Ground::shore`, `Ground::roof`, `WATER_SHORE_ROOFED`, `WATER_SHORE_OPEN`, `WATER_ROOF`). Neither rule can see a ceiling that has no walkable floor, because the game's collision is not read. Ground spawns were not changed: they already sit on the mesh and keep the class clearance from its open edges (1 to 4 units), and none were found in that room.
  - Seed "game", all maps: at 10x Jabu-Jabu 34 to 16 added (water 32 to 15), Zora's River adult 69 to 61, Gerudo Valley adult 55 to 54, Gerudo Valley child water 3 to 0 (all three were far from any shore), Zora's River child 24 to 24 (water 9 to 8); Forest Temple 61 to 65 (the random stream moves). Total 1548 to 1524. At 5x: Jabu-Jabu 31 to 19, Hyrule Field child 143 to 133, Gerudo Valley adult 16 to 14, child 10 to 9, Zora's River child 15 to 14; total 1033 to 1007.
- **Ceilings**: not in the mesh. Dungeons (maps whose actors carry a dungeon room index) count as roofed; Skulltulas and Wallmasters only go there, and fliers are kept 2.5 above the floor indoors.
- **Sand**: by map: Haunted Wasteland and Desert Colossus.

### Spots to keep clear (radius in units)
Entrances and arrival points (`DestinationAnchor`, `StartPos`, `ChangeBancBeforehandAnchor` 8; scene-change areas and dungeon entrance events 10; map entrance icons 8), event and cutscene triggers 6, NPCs 10, shop items 8, chests 4, signs 3, gossip stones 4, doors 5, warps 6, house interiors 6, the fishing pond 15, horses 10, fairy fountains 5, enemy counters 4, Gold Skulltulas 3, bosses 40, minibosses 25. Every added enemy is also at least 2.5 from any other enemy (1.3 within its own group).

### The class table (`horde::KINDS`)
| Class | Ground rule (max slope, ring radius, room) | Kinds (fun weight) |
|---|---|---|
| Miniboss | 8 degrees, 3, 4 | Stalfos (3), Dinolfos in pairs (3), silver Iron Knuckle (2, gets the wake-on-approach edit), white Wolfos (2) |
| Flat only | 10 degrees, 2, 2.5 | Wolfos (3), Floormaster (2), Dodongo (2), Freezard (1), Armos (1) |
| Gentle slope | 20 degrees, 1.5, 2 | Like Like (3), ReDead (1), Baby Dodongo (2), Torch Slug (1), Deku Baba (2) |
| Any ground | 32 degrees, 1, 1 | red Tektite (3), blue Tektite (2), Skullwalltula (1) |
| Flies | any floor below, room 3 | Keese 3.5 up (3), ice Keese (1), Guay 6 up (2, outdoors only), Poe (2), green and red Bubble (1), Shabom (1), Biri (1) |
| Water | see water above | Octorok (2) |
| Ceiling | dungeons only, 20 degrees | Skulltula 4.5 up (2), Wallmaster 4 up (2) |
| Sand | sand maps only, 32 degrees | Leever (3) |
| Never placed | | bosses and every `MiddleBoss_` (Gerudo fighter, Dark Link, Flare Dancer, Big Octo, dungeon Lizalfos, Nabooru's knight, Poe Meg); plain Lizalfos (the Dodongo black screen); Dead Hand (needs hands and links); did not appear when spawned away from home: Gibdo, blue Bubble, Fire Keese, big Poe; the spear Moblin (does not work); scrubs, Gold Skulltulas, Peahats (larva hurts enemies), Gohma eggs and larvae, Jabu tentacles, Anubis, Beamos, floor tiles, tricky doors, flying pots (the surprises feature owns them), white and black Iron Knuckles (linked pair), Spike, Stinger, Shell Blade, Bari, Tailpasaran, Stalchild, little Floormaster, Skull Kid, big Leever, Poe sisters and field Poes |

Heights above the floor come from the vanilla placements read against the mesh (Guay median 7.2, Skulltula 7.3, Wallmaster 4.9; ground kinds 0.0). Slopes follow your notes: Like Likes on gentle slopes are fine, the one by the lab was too steep, so Gentle stops at 20 degrees; enemies with the most ground machinery (Stalfos, Iron Knuckle, Dinolfos, Wolfos) are flat only.

### The plan for one map
1. Count the map's vanilla fighting enemies N (both files; not scrubs, Gold Skulltulas, bosses, museum or decoy entries).
2. Target = round(N x multiplier), or, from 3x, open flat area / 150 x (multiplier - 2) / 8 if that is more (so empty areas like Kakariko still get some), at most `MAP_CAP` = 240. What is already there (vanilla, the 2x copies, other features' additions) counts toward it.
3. Pool of kinds: kinds already in the map (weight x3) at every value; from 3x Keese, Guays and Octoroks anywhere they fit; from 5x every kind in the table that fits the map. At most 8 kinds new to one map (`NEW_KINDS_CAP`). Minibosses from 5x, at most (multiplier - 4) per map, never in child Kokiri Forest (no sword yet) or the Water Temple.
4. Place: pick a kind by weight; 60 % of the time a home kind goes 4 to 14 units from one of its own, otherwise a random spot on in-bounds walkable ground (area weighted). Check the class's ground rule, the clear spots, spacing and the cluster cap. From 5x a group of up to 1 + (multiplier - 3) / 2 (2 at 5x, 3 at 7x, 4 at 10x) forms 1.6 to 3.6 units around it, same room, same checks.
5. In dungeons each enemy gets the room index of the nearest actor within 25 units and 6 of height; no index means no enemy; rooms holding a boss or miniboss are skipped.
6. New actors: a copy of a plain vanilla instance of that kind from anywhere in the game (only basic keys; keys tied to events, game data, drop items or the room are removed), or a bare entry with no drop table if none exists. Fresh 64-bit ids, checked against every map file after all other difficulty steps (the horde step runs last). Appended to the map file that holds most of its vanilla enemies.

Deterministic: per map, from the seed text, the map name and the value.

### Safe arrival zones (your rule of 19 Nov)
"When you die and respawn you shouldn't get stuck in a loop where you get killed immediately on spawn." Every place Link can arrive gets a safe zone:
- **Within 15 units (`SAFE_CLEAR`): no added enemy at all.**
- **Within 30 units (`SAFE_WIDE`): at most 2 added enemies, and only slow kinds that neither fly nor shoot** (Like Like, Deku Baba, Armos, Torch Slug, Skullwalltula, Floormaster). No minibosses, Octoroks, Bubbles, Guays or Keese there.
- **Never ringed:** the added enemies within 30 units always leave an open arc of at least 120 degrees to run through (`SAFE_ARC`).
- Distances are horizontal, for enemies within 12 units of height of the point (`SAFE_BAND`). Vanilla enemies are left where the game put them.
- The rule wins over the multiplier: a small room near an entrance simply gets fewer enemies.

Arrival points found in the map data: player start positions (`StartPos`, every map entrance, dungeon entrances and the start of each dungeon area), load-zone arrival anchors (`ChangeBancBeforehandAnchor`), field and dungeon entrance events and their walk-out points (`FieldAndDungeonEntranceEventRequestActor`, `DestinationAnchor` named `FieldDungeonEntranceGo`), map entrance markers (`MapIconEntranceAnchor`), warp pads (`Obj_WarpPoint_A_01`), restart points (`DestinationAnchor` named `SwapRestartPosAnchor`), hole entrances and exits (`HidingHole_HoleEntrance`, `HoleExit`, `FieldEntranceGo`, `FieldExitGo`), event player positions (`PlayerPos`), `ReturnHome` and the storm-hole safe position. Not found as data: the point Link is put back at after a fall into the void (the game picks the last ground he stood on at run time), warp-song arrival spots and the owl's drop points (if they are among the unnamed `DestinationAnchor`s they are not recognised; those unnamed anchors, 906, are mostly camera and event targets and are not treated as arrivals).

### Capacity: the multiplier is a ceiling (your rule of 19 Nov)
"Every room with an enemy doesn't literally need 10x as many enemies. Just the max amount that makes sense, up to a max of 10x."
- Each area is a dungeon room, or outdoors a connected piece of walkable mesh (Hyrule Field's mesh comes in 250-unit tiles, so there an area is at most a tile).
- Usable floor = in-bounds walkable faces of the area, minus faces whose middle is within 15 units of an arrival point.
- Ground capacity = usable floor no steeper than 20 degrees / 30 square units (`GROUND_AREA`). Air capacity = all usable floor / 60 (`AIR_AREA`), for fliers only. Water: 6 Octoroks per vanilla Octorok in the map (`WATER_PER_ANCHOR`).
- An area's limit = the smaller of (its vanilla enemies x the multiplier; for an area with none, its floor / 150 x (multiplier - 2) / 8) and (ground + air capacity). Vanilla enemies count toward it and are never removed.
- The map cap of 240 and the cluster cap still apply on top.

### Safety budget
- `MAP_CAP` = 240 enemies in one map (lowered to 160 after the first game test, raised back after the second) (vanilla's busiest maps: adult Hyrule Field 79, Fire Temple 78, Spirit Temple 76). About three times the busiest vanilla map. One const; lower it to test.
- `CLUSTER_CAP` = 12 enemies within 15 units of any enemy (vanilla's densest: 14 in Ganon's Castle, 12 in the Spirit Temple). Applied both ways: a new enemy may not push any neighbour's count over 12.
- `NEW_KINDS_CAP` = 8 kinds new to one map (vanilla has up to 23 kinds in one map).
- Dodongo black screen: the suspect edit was a plain `Enemy_Lizalfos` (a kind the cavern never uses, with no partner) in `_Static`, near the boss with the boss room's index. The horde never uses plain Lizalfos, never places in boss or miniboss rooms (and keeps 40 units from bosses), and takes the room index from the nearest actor. New kinds still go into maps that never had them, which loaded fine for the Like Likes, Dead Hand and Iron Knuckle.

### Rooms that open when enemies are beaten
Earlier reading of all 1,824 logic graphs (the 2x feature) found rooms count specific linked enemies (enemy counters and enemy groups), never everything in a room. Added enemies have no links, so they should not block doors. Not seen in the game; the test plan checks two such rooms first. Gerudo Training Ground (timed rooms) is skipped to be safe.

### Skipped maps
Boss rooms and dream world stages; `Market_Child` (town and minigames), `HyruleCastle` (stealth), `GerudoFortress_Adult` (guards catch you, archery), all `ZoraDomain` and `GoronCity` maps (towns), `CrumblingGanonCastle` (timed escape), `SageRoom`, `GerudoTrainingGround`, `FinalBattleField`, `Demo_` cutscene maps. Shops and houses are inside the overworld maps and are kept clear by the NPC, shop and interior radii.

## Numbers (seed text "game"; vanilla, then totals)
| Map | Vanilla | 2x | 3x | 5x | 10x |
|---|---|---|---|---|---|
| Hyrule Field, child (with Lake Hylia) | 49 | 55 | 131 | 205 | 240 |
| Hyrule Field, adult | 76 | 111 | 197 | 240 | 240 |
| Lake Hylia part (child, west of x -100, south of z 380) | 5 Octoroks | 5 | | | 26 (12 Octoroks, Like Likes, Deku Babas, Guays, Bubbles) |
| Kokiri Forest, child | 5 | 5 | 9 | 15 | 33 |
| Kokiri Forest, adult | 22 | 22 | 48 | 62 | 73 |
| Dodongo's Cavern | 43 | 71 | 77 | 104 | 135 |
| Fire Temple | 78 | 104 | 121 | 149 | 151 |

2x is vanilla plus the copies. Above 2x the copies are made first and the horde fills to the same target, so the totals do not depend on them. Hyrule Field reaches the cap at 5x (adult) and 10x (child). 39 maps are edited at 10x, 2,010 added enemies in all: 3,652 before the two rules, 2,699 with capacity alone, 2,010 with the safe zones too (they removed 689). Of the 172 areas that hold vanilla enemies, 9 reach the full 10x and 136 are held lower by their capacity (the rest by the map or cluster cap or by running out of places). Per-area vanilla, capacity, limit and counts at 3x, 5x and 10x are in `<playtest folder>\horde\counts.csv`.

## How it was tested
- Unit: navmesh reader on hand-made meshes (height, slope, room, two floors, patches, tile offsets); class table sanity; target grows with the slider and stops at the cap; group and miniboss limits; map names.
- Game files (`OOTRR_ROMFS`): every navmesh file parses (except the template one); vanilla Deku Babas, Tektites and Dodongos stand on their maps' meshes. For 1x and 2x the horde plans nothing. At 3x, 5x and 10x: counts never fall as the slider rises (six maps printed), Hyrule Field and Dodongo's Cavern more than double at 10x, child Kokiri Forest gets more than 20; every ground spot is on the mesh within 0.05, on a face and a ring no steeper than its class allows; every water spot has no floor within 3; minibosses only from 5x and not in the excluded maps; Leevers only on sand maps; no 15-unit cluster of more than 12 added enemies; the same seed gives the same plan and another seed a different one. Staged at 10x: old actors untouched and in order, added entries have no links, no name, no drop item, each id used exactly once; the Iron Knuckle wake edit is added once (not when special encounters already add it).
- Safe zones, at 3x, 5x and 10x on every edited map: no added enemy within 15 of any arrival point; within 30 at most 2, all of the slow kinds, with an open arc of 120 degrees or more.
- Capacity: every area's count equals its vanilla plus its added enemies and stays at or under its limit; the limit never exceeds vanilla x multiplier or the capacity; some small dungeon room ends under 5x its vanilla count and some area reaches 8x or more.
- With every difficulty setting on at 10x (`difficulty.rs`): no file staged twice, the wake pack once, more than 1,000 actors added, no actor id used twice except where vanilla already does.
- `cargo test -p ootrr-core difficulty` 181 passed, `settings` 106 passed; `python tools\audit.py randomizer` FAIL 0, no new WARN. Release build copied to `<playtest folder>\horde\ootrr.exe`.

## Jabu-Jabu water spots (19 Nov, game check)
Own shadow game, 10x, Jabu-Jabu loaded and ran; the scrub room held 139 actors and 20 or so Octoroks around the pool, positions read with `enemies`. NOT confirmed in the game: that none is under a ceiling or in a wall. Positions alone cannot show that (the game's collision is not read), so a person has to look at the pool; if one is still wrong, send its position.

## NOT TESTED in the game: everything
Nothing was run in the game. Unknown: whether 240 enemies in Hyrule Field load and run at a usable frame rate; whether new kinds load in every map (Like Likes, Dead Hand and Iron Knuckle did in Hyrule Field); whether dungeon room indices from the nearest actor are right; whether enemies stay on the ground they were given (especially Tektites and Wolfos at the edge of a ledge); whether fliers outdoors behave; whether doors that open on "all enemies beaten" still open; whether Guays copied from the night-only child-field ones appear by day (they carry the night key); whether the silver knights wake.

## How to test it in the game
Script: `<playtest folder>\horde\PLAN.md` (order by risk, pass criteria, what to read through the control bridge). Compare against `placements-10x.csv`. By hand: set More enemies to 10x, start a seed, walk out of Link's house: Keese, Guays and Tektites around the village; go to Hyrule Field and Lake Hylia.

## Notes
- The old choice values (Half again, Double) in a settings string from an earlier build read as out of range and fall back to 1x with a warning. Default settings string unchanged.
- Added enemies have no save flag: they are back when the area loads again.
- Out of reach ledges: the in-bounds rule needs actors nearby, so roofs and cliff tops with props on them can still get an enemy. The test plan watches for it.


## First game test (19 Nov, `<playtest folder>\horde\RESULTS.md`) and the fix round
Seen: 37 of 38 maps loaded at 10x, no crash, Dodongo's Cavern fine (174 live). Problems and what changed:
- **Freezards fell through the floor** (5 of 10 in the Ice Cavern, 1 of 2 in the Forest Temple): Freezard is no longer placed.
- **Child Haunted Wasteland** is black even without the mod: dropped from the edit list.
- **Frame rate**: 60 with no mod and at 5x; 28 at 10x in child Hyrule Field and Dodongo's Cavern. 5x held 60 with up to about 75 live enemies within 60 units of Link (Dodongo's Cavern) and 205 planned in child Hyrule Field. New caps: `MAP_CAP` 160 per map, and `ACTIVE_CAP` 80 enemies (vanilla and added together) within `ACTIVE_RADIUS` 60 units of any enemy, checked both ways like the cluster cap.
- **Slow maps with few live enemies** (both Desert Colossus maps 18 and 23 fps, adult Gerudo Valley 17, child Hyrule Field 28) are the maps with the most added Guays (14, 9, 16, 32); fast maps had 0 to 3. The Guays are copies of the night-only field Guays, so by day they are loaded but never seen. Hypothesis, not proved: hidden Guays (or their flock logic) cost frame time. Guays are no longer added (vanilla ones stay).
- **Kinds that never appear when created in the field** (speed survey: blue Bubble, Gibdo, silver Iron Knuckle, fire Keese, big Poe): blue Bubble, Gibdo, fire Keese and big Poe were never in the horde pool (the 2x copies still copy Gibdos and fire Keese in their own home maps, as before). The silver Iron Knuckle stays: special encounters places the same kind, `Enemy_IronKnuckle_Silver`, the same way (a map entry with the wake key), and that knight was seen working in Hyrule Field; only knights created at run time fail to appear. A test now fails if any of these kinds, or a Freezard or Guay, is placed.
- **Planned enemies missing** (Gerudo Valley 0 of 106, Water Temple 1 of 11, adult Hyrule Field 10 of 48, adult Lost Woods 14 of 118, Fire Temple 17 of 75; child Kokiri Forest all): cause not found from the data. Ruled out by comparing the maps: the added entries are built the same way everywhere (same template per kind, same keys, fresh unique ids checked against every map file); both groups include `_Static` and `_Dynamic` edits (Kokiri Dynamic arrived, adult Gerudo Valley Dynamic did not; Shadow and Spirit Temple Static arrived, Fire Temple Static did not); the navigation mesh lines up with the props in every one of these maps (Gerudo Valley child 14 of 18 grass, rocks and pots on the mesh, adult 19 of 23); the same kinds arrive in Kokiri Forest and go missing in Gerudo Valley; every map has a precomputed culling folder and a per-map scene parameter file, Kokiri included. Hypotheses and the checks are in `<playtest folder>\horde\PLAN.md`.

Totals now (seed text "game"): 10x adds 1,580 enemies over 39 maps (was 2,010). Child and adult Hyrule Field 160 from 5x; child Kokiri Forest 26; Dodongo's Cavern 121; Fire Temple 127. Of 172 areas with vanilla enemies, 7 reach the full 10x and 136 are held lower by their capacity.


## Second game test (19 Nov) and the final round
What the game showed, in plain words:
- **The "missing enemies" were the test tool's fault**, not the mod: with a wider scan the added enemies are there (adult Gerudo Valley 72 of 72, Fire Temple 50 of 49, adult Hyrule Field 81 of 84, child Hyrule Field 89 of 111).
- **The low frame rates came from a second copy of the game running.** Alone, 10x holds 59 to 60 frames a second everywhere measured (Dodongo's Cavern with 103 live enemies, both Hyrule Fields, Desert Colossus, Gerudo Valley).
- Nothing fell in the Shadow Temple, Spirit Temple or Zora's Fountain.
- Safe arrival zone passed in child Hyrule Field (nearest enemy 58.8 units, no damage in 5 s) and in Dodongo's Cavern (first test).

Changes in this round:
- Caps back up: `MAP_CAP` 240 per map, `ACTIVE_CAP` 120 enemies within 60 units.
- Guays are placed again as fliers (outdoors only). They fly by day and by night in the game, so the frame-rate theory against them was wrong. Freezards stay out (they fell through the floor).
- **Exits:** enemies had been planned on the Gerudo Valley exits to the lake, (71.2, 41.6, 273.7) adult and (76.5, 41.2, 257.9) child. That area holds only Lake Hylia trees (scenery that belongs to the next area); no trigger actor is near it in the valley's files. Two fixes: scene-change areas (`AbilityChangeAreaForSceneChange`) now count as arrival points (nothing within 15, the 30-unit rule around them), and "in bounds" now needs a non-scenery actor (objects, enemies, NPCs, anchors, areas; not models, trees, effects, sounds, animals) within 30 units, so ground with only scenery on it is no longer used. A test checks that no Gerudo Valley enemy is within 15 of those two points, and every map is checked against every arrival and scene-change point.
- **Why Dodongo's Cavern had about +100 live against 78 planned:** above 2x the 2x copies are made first (28 in the cavern) and were not in the CSV. `placements-10x.csv` now lists them too (class `Copy`), and the horde rows are planned the way the build does it, after the copies.
- **Tektites, Like Likes and Octoroks matched their planned spots only 2 to 34 percent** (Wolfos, Poe, Stalfos 88 to 100): no placement fault found in the data. Their heights come from the same mesh as the others (vanilla Tektites and Like Likes sit at median 0.1 and 0.0 above it); Octoroks go at the height of a vanilla Octorok. Tektites hop, Like Likes crawl toward Link and Octoroks swim and dive, so they leave their spot. Treated as movement, not a fault.

Totals at 10x now (seed text "game", copies included): 1,802 added over 39 maps (221 copies, 1,581 placed). Child Hyrule Field 240, adult Hyrule Field 240, Dodongo's Cavern 153, Fire Temple 175, child Kokiri Forest 23, adult Gerudo Valley 73.

## Still not tested in the game
- Doors that open when a room's enemies are beaten, with added enemies alive.
- Safe arrival zones in Kokiri Forest and a second dungeon through a real entrance and a death.
- Playing it by hand: whether 10x is fun or impossible, minibosses fighting, the Iron Knuckle waking.
- The raised caps (240 and 120) themselves: the 60 fps result was measured with 160 and 80.

## Round after the user's hands-on play of rc 14 (19 Nov, branch `feature/horde-r2`, commit 641c941, not played yet)
- **Peaceful places** get no added enemies: child Kokiri Forest, Kakariko Village (child and both adult maps: the game places no enemies there as adult either), child Market, Hyrule Castle, Gerudo Fortress, Zora's Domain, Goron City. Lon Lon Ranch and the Temple of Time are not maps of their own; added enemies keep 45 units from the ranch's buildings and 30 from the temple's.
- **Iron Knuckles:** at most 2 in a Hyrule Field map and 1 anywhere else, counting the special-encounter knight (the horde runs after it and counts it).
- **Big enemies spread out:** Like Like, Wolfos, Stalfos, Dinolfos, Dodongo, ReDead, Iron Knuckle, Moblin, Floormaster, Armos (and Gibdo, Dead Hand, Lizalfos when vanilla) keep 25 units (`BIG_GAP`) from any other big enemy, and are never placed in groups. Small kinds still group.
- **Keese perch:** every kind of Keese is placed only at a perch, outdoors 3.5 above the base of a tree, indoors 2.5 up by a wall, never in open air. Guays stay in the air. Fire Keese are back, only in maps where the game already has them (Dodongo's Cavern gets 28, perched by its walls).
- **No walkers on unreachable ground** (the lava in Dodongo's Cavern): in maps with a single navigation mesh, walkers only go on mesh pieces that hold an arrival point or a vanilla walking enemy. Fliers are exempt. Hyrule Field's tiled mesh keeps the earlier rule.
- **No added Dodongos or Baby Dodongos** in Dodongo's Cavern (also no 2x copies of them there), nor in a map with bombable walls, bomb-only walls or burnable webs (`WallCrack`, `CrackWall`, `HitOnlyBomb`, `HitBomb`, the Deku Tree webs) unless the kind is already in that map.
- **Guards:** tough kinds (big ones and minibosses) half the time stand 4 to 10 units from a tree, a hole entrance marker or an interest spot.
- **Special encounters folded in:** the `special_encounters` setting (130) is retired; any More enemies value above 1x includes everything it did. An old settings string that sets it gets a "was removed" warning; the default string is unchanged.

10x now (seed text "game", copies included, special encounters not in the CSV): 1,710 added; child Hyrule Field 240, Dodongo's Cavern 145.
Left: the rule is per map, not per room, for bombable obstacles; lava is excluded through reachability, not a lava attribute (the mesh has none).

## Release (rc 16, 20 Nov)
In rc 16: the Jabu-Jabu water rule above (water spots need a shore within reach and no floor overhead).

- **By script** (checks-1120, 10x): 19 Octoroks added in Jabu-Jabu, 10 of them at water level in the business scrub room pool; screenshots show none sunk in a wall.
- **Not checked by eye:** whether any sits under a ceiling (still in the capture playbook).


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 10):** More enemies (slider 1x to 10x)
- **State:** Played by hand at 10x in rc 14 and in the rc 15 build (19 Nov: "the extra enemies are great"); rc 15 placement rules not checked one by one
- **What was seen in the game:** Old 2x maps loaded (rc 4). New 19 Nov (`feature/horde` a6e2ce3, exe `<playtest folder>\horde\ootrr.exe`): slider 1x to 10x; 1.5x and 2x unchanged; above 2x enemies are placed on the game's own navigation mesh by ground class (flat, gentle slope, any, flier, water, ceiling, sand, miniboss), new kinds, groups and minibosses from 5x; caps 240 per map and 120 within 60 units, 12 within 15 units; safe zones around every arrival point (none within 15, at most 2 slow kinds within 30, a 120 degree way out); the multiplier is a ceiling per room or ground patch, limited by usable floor. 1,802 added at 10x over 39 maps (copies included). Second game test: all added enemies present with a wide scan, 59 to 60 fps at 10x with one game copy. Game test 19 Nov: 37 of 38 maps load at 10x; fix round done (Freezard and Guay dropped, child Wasteland dropped, caps lowered); Second test 19 Nov (exe 66bb967 build): 10x frame rate 59 to 60 at every spot tried (control 60); the "missing" enemies were mostly the `enemies` scan window (wider scan: Gerudo Valley adult 35 live instead of 6 to 8, 10x minus 1x live counts match planned); Hyrule Field child death reload with no damage; room-clear doors and Kokiri reload not tested. Played 19 Nov (`<playtest folder>\horde\RESULTS.md`): 37 of 38 edited maps load at 10x, Dodongo included; nothing crashed; slider rises 5x to 10x where enemies appear; frame rate 28 at the busiest spots (child Field, Dodongo; control 60); planned enemies missing in Gerudo Valley (0 of 106), Water Temple, Field adult, Lost Woods adult, Fire Temple; some Freezards fall; Try Again test passed at the Dodongo entrance only; door logic and fights not tested; Third test 19 Nov (rc 14 exe, `<playtest folder>\horde\RESULTS.md`): 10x loads in child Kokiri, child and adult Field, Dodongo, Fire Temple, 58.5 to 60.2 fps at every spot (1x control 60), live added enemies +18 to +127 near the spots, nothing vanished or fell except 1 Tektite and 2 Guays at one Field spot each; all difficulty settings on (5x): Kokiri, Field, Dodongo, Deku Tree load and Link moves, 3 minutes in Field with enemies: up 180 s, 46 to 56 fps (average 49.7)
- **Next step:** Scripted load test by risk: `<playtest folder>\horde\PLAN.md` (Dodongo's Cavern first, then Hyrule Field at the cap)


## Moved from STATUS.md (20 Nov): checks of rc 15 and rc 18 (agent 6)

| **rc 15 and rc 18 checks (agent 6, done)** | `<playtest folder>\checks-1120\RESULTS.md`: 30 pass, 10 fail, 7 not tested.
  **Seen working:**
  - 36 edited maps load at 10x.
  - Peaceful places get nothing added (Kokiri 7 = 7, Kakariko 6 = 6).
  - Dodongo's Cavern: no enemy loses health in lava, no added Dodongos.
  - About 95 % of Keese sit on perches.
  - Iron Knuckles within the cap.
  - Gates under dungeon shuffle: gate 9's guard is there and blocks; gate 2 has none.
  - Backup: 17 callers bring the expected lesser helper in 14 to 31 s; business scrub calls nobody.
  - Regen: a second hit restarts the wait.
  - Jabu-Jabu: 19 added Octoroks, none in a wall in the shots (the ceiling cannot be judged from a still).
  **Fails:**
  - Frame rate at 10x: 27 to 45 fps at busy spots in Dodongo's Cavern, Hyrule Field and the Market (1x: 60). The other game copy ran at the same time, so this is to remeasure alone.
  - Backup: Wolfos and Bari bring 1 helper, not 2; spear Moblin called nobody in 40 s; Stalchild, Skulltula, Wallmaster, Armos and Biri still call nobody in their home maps (45 s).
  **Found on the way:**
  - The `enemies` full scan stalls the game (49 fps idle, 17 fps with a scan every second). It does not grow memory: about 30 GB is normal for a test copy.
  - The window size comes from `HOS_WINDOW_SIZE` in 96-dpi pixels. The screen is 3840x2160 at 200 %, hence "2560x1440" (RECIPE.md) |
