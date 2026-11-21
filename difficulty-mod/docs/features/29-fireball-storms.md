# 29. Fireball storms

Setting `fireball_storms` (number 140), toggle, default off. Difficulty page, new section "Hazards" (it is a danger of the place, not an enemy). Branch `feature/fireball-storms`, worktree `wt\fireballs`. Not played yet. Extended on `feature/fireball-storms-2` (see "Extension 2" at the end).

## What was asked (user, 20 Nov)

"Random fireball storms. In the vanilla game there's one sequence the first time you go up the volcano where it erupts and lots of fireballs fly down. Can we make that storm of fireballs fall down at other times too? Maybe when you're sneaking into Hyrule Castle courtyard, or in some of the boring-er parts of Hyrule Field."

## How the Death Mountain Trail eruption works (read from the game files)

Both `DeathMountainTrail_Child` and `_Adult` `_Static` maps hold the same set of falling-rock managers. All are plain map actors with a Logic module; none is a Death-Mountain-only actor.

| Actor | Count | What it does |
|---|---|---|
| `Obj_FallingRockManager_A_04` | 5 | The burning-rock storm. Launches a rock from its own map position to each linked `DestinationAnchor` in turn. Rock `Obj_FallingRock_A_04` carries a sub-actor `Obj_FallingRock_Fire_A_01` (fire damage, attack power 2; the rock itself 4). |
| `Obj_FallingRockManager_A_05` | 1 | A fan of rocks (`Obj_FallingRock_A_02`, up to 25) every 2 to 6 frames, 80 to 100 units from the manager, within a wide angle. The large "demo" storm. Gated by the saved flag `MountainTrail_ChildFallingRockDemoActivated`. |
| `Obj_FallingRockManager_A_01` | 1 | Rocks (`Obj_FallingRock_A_05`, up to 20) along a rail, near the first slope. |

What starts and stops them. Each manager has a small AI Logic module (for example `DeathMountainTrail_Child_Prefab_7fa0`): an `Area` actor (layer Player) feeds a `HoldActorObserverBySensorHitInfo` node, and its `IsHit` output is wired to the manager's `IsActive` input. So the storm runs while Link stands in the area. The area's `Logic_Delete` is wired to an OR of the saved flag `MountainTrail_HasReachedPeak_Child` and a Great Fairy flag, which switches the storms off for good once the peak is reached. That is the only story coupling.

How the A_04 manager works (its AI, `Obj_FallingRockManager_A_04.root.ainb`). While `IsActive` is true: a loop with a counter (`IntervalFrame` from the map actor's `Dynamic`, 15 to 120 frames in vanilla); each time it fires it reads the `Reference` link whose destination actor is named with the current index ("0", "1", ...), launches a rock from the manager's home position to that anchor with flight time `FallTime` (3.5 to 4 s), and adds one to the index. When no link has that name the index returns to 0. At most 5 rocks are held at once. Landing spots are the anchors, so a storm is deterministic, readable and dodgeable. In vanilla the managers hang 65 to 150 units above and about 140 units away from anchors packed within 15 units of the path.

Damage. The rock hits for 4 base attack power (one heart), the fire sub-actor for 2. Unchanged by this setting. (Enemy damage multipliers do not apply: it is a map object.)

Packs and effects. Nothing new is loaded by hand: the manager's `ActorHolderParam` lists the rock (`DependencyWithHolder`), and the rock's holder lists the fire actor, so the engine pulls the packs in with the manager, as for every actor the build places that a map did not have (Dead Hand, Like Like, Iron Knuckle). Effect keys are shared in `ELink2` and keyed by actor, so the visuals travel with the rock. If a map shows no burning rock but the manager is active, this is the first thing to suspect.

## Design

Place the A_04 manager and its anchors in new maps and copy the vanilla Logic module's wiring. Data edits only: map placement files (`Banc/.../_Static`) and one new Logic graph per spot. Nothing is a code patch, nothing is Longshot-specific, nothing touches the volcano maps (the test checks that the Death Mountain files are not written and still hold 5 managers).

Per spot the build adds, to every map that carries it:

- `Obj_FallingRockManager_A_04`, high above and 60 to 110 units to the side of the spot (the same shape as on the mountain), with `Dynamic` `FallTime` and `IntervalFrame`, and `Reference` links to the anchors.
- 5 or 6 `DestinationAnchor` actors named "0", "1", ... on the ground around the spot (the landing points).
- One `Area` actor, the trigger box.
- `FarDeleteGroups` and `SimultaneousGroups` entries (when the map has them) holding all of the above, so the engine loads and unloads them together.
- An `AiGroups` entry whose Logic module is a new graph `Logic/Hazard_FireballStorm_<spot>_<tag>.logic.module.ainb`, built from the gates graph template with `ambush`/`gates` machinery: Area node, player sensor, AND, and the manager node (`ActorLogicFallingRockManager`) with `Logic_Create` true and `IsActive` wired to the AND output. So the storm runs only while Link is in the box, stops when he leaves and starts again on re-entry. The manager is created with the map and is never deleted by logic, exactly like the vanilla prefab.

Never tied to the volcano flags, never blocks anything (the manager, anchors and area have no links to doors, events or enemy counters).

Not done: a random chance per visit. The AI graph has random-number nodes, but a query re-rolls every frame, so a chance gate would flicker the storm on and off instead of deciding once per visit. A spot is therefore always on while the setting is on. A cheaper randomness is to ship fewer spots; the table below is where to cut. (Possible later: a rare-spawn variant that uses the A_05 manager's own random fan.)

## Spots

All coordinates are map coordinates. Heights come from the navigation mesh (child and adult agree within 0.3). Landing spots sit 0.5 above the ground. Each trigger box is 32 x 12 x 30 (width, height, depth) in the field, 44 x 10 x 26 in the castle.

| Spot | Maps | Trigger centre | Manager | Landing spots (x, z) | Interval, fall |
|---|---|---|---|---|---|
| `field_middle_road` | `HyruleField_Child`, `_Adult` | (75, 152.9, 95) | (170, 216, 60) | (75,95) (85,100) (65,100) (80,85) (70,105) (85,90) | 45 frames, 3.5 s |
| `field_south_central` | same | (55, 163, 190) | (150, 226, 160) | (55,190) (65,195) (45,195) (60,180) (50,200) (65,185) | 45, 3.5 |
| `field_west_road` | same | (-170, 137.1, 170) | (-75, 200, 135) | (-170,170) (-160,175) (-180,175) (-165,160) (-175,180) (-160,165) | 45, 3.5 |
| `castle_front_garden` | `HyruleCastle` | (14, 118, -76) | (14, 186, -165) | (25,-74) (28,-80) (4,-71) (25,-89) (-14,-86), all at height 114.5 | 60, 3.5 |

How the field spots were chosen (survey of the navigation mesh, child and adult together): open, flat ground (every point within 15 units on one floor, within 4 of the centre height, slope at most 18 degrees), at least 70 units from every arrival point, map exit and entrance request actor, and at least 45 from every hole, gossip stone, Great Fairy hole, Business Scrub, Gold Skulltula, Kaepora, ranch actor, Lake Hylia Like Like site and our special encounters (the Iron Knuckle at (134, 157.7, 168.5), the Dead Hand hole, the Guay flocks are all further than that). Middle road is the open stretch between the Market gate (0, 151, 32) and the Kakariko road; south-central is the grass south of the Iron Knuckle between the Lon Lon and Lost Woods roads; west road is the lowland on the way to Gerudo Valley (arrival at (-307, 137, 166), 120 away).

Castle spot. The child castle (`HyruleCastle`, one map) has stationary guards on `AnchorAction1` posts: the front garden holds six `Npc_Hylia_Soldier_*` posts at (13,-64), (-5,-77), (16,-82), (-5,-95), (3,-95), (35,-93), height 114, and the courtyard holds five more, from (-4,-132) on. The front garden is the sneaking stretch before the courtyard. The landing spots are all at least 9 units from every soldier post (checked by the test); the trigger box spans the garden between the gate guard (22, 108, -56) and the first guard line. The area, manager and anchors have no link to the guards' `FoundBySecurity` events. Open risks: a fireball landing in a guard's line of sight may or may not make him react (unknown, never seen), and Link may be knocked back into a guard's view. Cut the entry from `SPOTS` if that happens; the other three are independent. The approach from the entrance to the gate guard (Malon's wagon at (17.6,105.5,-25.1), the gossip stone at (21.4,112.2,-35.5)) is deliberately left alone: NPC event actors stand there and Malon is a quest NPC.

## Setting and menu

Number 134: free on upstream main and on every open branch and worktree (130 to 132 used by the base, 133 Volvagia, 136 to 139 Bowling, enemy randomizer and others; 134 and 135 were unused). The retired list (`RETIRED` in `settings/list.rs`) holds 55, 63, 92, the old shop settings 59 to 62 and 130 (old special encounters), so 134 is not retired either. Id `fireball_storms`, label "Fireball storms", tooltip "Burning rocks rain down on a few quiet stretches of Hyrule Field and the Hyrule Castle garden, like on Death Mountain. Dodge them." (23 words). Placed in a new "Hazards" section after "Boss fights" on the Difficulty page. Default off; the default settings string is unchanged (tested).

## How it was tested

`cargo test -p ootrr-core difficulty`: 214 passed (4 are new, in `difficulty/fireballs.rs`, run against the game files with `OOTRR_ROMFS`):

- Spot layout: 3 to 8 landing spots, manager 60 to 110 units away and 55 to 80 above, every landing spot within 30 of the box.
- The generated graph parses back, audits clean, has 4 more nodes than the template, the manager node has its `IsActive` input on the AND output, built twice gives the same bytes.
- Map files: only the 4 `_Static` files below are written (`HyruleField_Child`, `HyruleField_Adult`, `HyruleCastle`... see the test), the old actors are untouched and first in the list, no hash is used twice, AiGroups stay sorted, each manager links all its anchors, anchors are named "0".., no landing spot is within 9 units of an NPC or enemy actor at the same height, and the Death Mountain Trail maps are not written and still hold 5 managers.
- Every landing spot is on the navigation mesh within 1 unit in both ages.
- Setting off: `stage` writes nothing; setting on writes exactly the four graphs and the three map files.
- `cargo test -p ootrr-core settings`: 106 passed (default string unchanged, menu places every setting).

Not tested in the game. Nothing about the actual fall, the damage, the picture or the castle guards has been seen.

## How to test it in the game

See `<playtest folder>\fireballs\PLAN.md`: load each touched map first (3 maps: child Field, adult Field, Castle; Death Mountain Trail child as the control), then stand in each box and watch.

## Seen in the game (20 Nov)

The field_middle_road storm works: rocks arc in and burn on the ground (user's clip, handson-6). Expect the first impact about 5 s after stepping into the area (45-frame interval plus 3.5 s flight); after that a rock about every 0.75 to 1.5 s. It stops when Link leaves, though rocks already in flight still land. The setting number is 140 (134 is taken by door of time). The castle garden spot is not checked yet.

## Release (rc 16, 20 Nov)
In rc 16 as setting 140 (the "Setting and menu" section above still says 134, which upstream's door of time took). Number check on upstream 0.8.0: 133 and 134 door of time, 135 sword, 136 used upstream; 139 is bowling grass; 141 is held for faster blocks.

- **Played by hand** (session 6, 20 Nov): the middle-road storm (rocks arc in and burn; slow to start, about 5 s) and the castle garden storm (the guards behave).
- **Seen:** the frame rate in child Hyrule Field was about 30 to 37 with more enemies at 5x plus the storm (60 before). Not measured with the storm alone.
- **Not played:** `field_south_central`, `field_west_road`, and the adult field.

## Extension 2 (20 Nov): the Trail on every visit, and four more spots

Branch `feature/fireball-storms-2`, worktree `wt\storms2`. Not played yet.

### What was asked (user, 20 Nov)
The Death Mountain Trail storm should also run on every visit, child and adult, not only on the first child visit. And four more places: Desert Colossus (child), two in Hyrule Field (child) and the Haunted Wasteland (child; the game spells it `HountedWasteland`), each also in the adult map if the ground is the same.

### How the Trail storm is freed
The Trail's five `Obj_FallingRockManager_A_04` (per age) each have a Logic module `DeathMountainTrail_<Age>_Prefab_<tag>`. Four per age are the plain shape described above: an `Area` whose `Logic_Delete` input is an OR of a "story is past" flag, so the area is deleted for good and the storm stops. Child: `MountainTrail_HasReachedPeak_Child` and the Great Fairy flag. Adult: `MountainTrail_HasReachedPeak_Adult` and `HavePlayedEvent.Dm_Kakariko_0025`. With the setting on the build rewrites those 8 graphs (`TRAIL` in `fireballs.rs`) and turns that one `Logic_Delete` input into the constant false (the same graph edit `force_bool` the other features use). Nothing else in the graph changes, the flag nodes stay, and the story events that set the flags are untouched, so the storm now runs whenever Link stands in its area, on every visit and in both ages. Map files are not written; the Trail maps stay byte-identical.

Left vanilla on purpose: the fifth A_04 per age (`DeathMountainTrail_Child_Prefab_128e`, `_Adult_Prefab_e4bd`), the `A_05` fan (`..._4399`, `..._f137`) and the `A_01` rail rocks. They are wired to the demo flags `MountainTrail_ChildFallingRockDemoActivated` / `MountainTrail_AdultFallingRockDemoActivated` (set and cleared by the cutscene), so freeing them would fire them without the demo. The test checks that exactly these are the ones not rewritten and that each is demo-gated.

The adult Trail has the same managers (same positions), so adult gets the same four.

### New spots
Positions come from the actor the user pointed at (hash `id=` in the object map), read from the map file; the ground height from the navigation mesh. Child and adult navmesh agree within 0.3 at every point (the Haunted Wasteland meshes are identical). All four exist in both ages, so every spot is in both. Each box is 32 x 12 x 30 (the desert box 28 wide); landing spots 0.5 above the ground, at least 4 from any navmesh wall, at least 9 from NPCs and enemies at the same height and at least 12 from arrival points, entrances and holes.

| Spot | Maps | Anchor actor | Trigger centre | Manager | Landing spots (x, z) |
|---|---|---|---|---|---|
| `desert_colossus_flats` | `DesertColossus_Child`, `_Adult` | `0x19a1413932e31971`, a liftable rock at (-144.2, 95.0, -43.1) | (-137, 98.7, -43), 28 x 12 x 30 | (-60, 160, -70) | (-137,-43) (-131,-47) (-131,-38) (-141,-48) (-137,-33) (-126,-43) |
| `field_south_east` | `HyruleField_Child`, `_Adult` | `0xcc590b2369acd50c`, an insect effect at (135.4, 159.6, 218.6) next to a navmesh obstacle | (133, 162.8, 214) | (60, 225, 235) | (145,218) (145,208) (135,205) (120,208) (120,218) (145,228) |
| `field_near_market` | same | `0x92db1c457b586c10`, a rock at (-7.4, 148.4, 94.8) | (-33, 152.8, 92) | (-110, 212, 70) | (-26,92) (-36,88) (-41,98) (-31,100) (-26,106) (-36,80) |
| `wasteland_dunes` | `HountedWasteland_Child`, `_Adult` | `0x864c47bd60c6787e`, a forbid-auto-placement box at (-56.0, 102.1, -55.3) | (-56, 103.4, -55) | (-125, 165, -100) | (-52,-47) (-46,-52) (-66,-50) (-45,-67) (-61,-42) (-44,-60) |

Why the centres moved from the pointed-at place:
- `field_near_market`: the Lake Hylia arrival point `HyruleField_FromLakeHylia` is at (0.1, 149.7, 98.6), 8 from the pointed-at place, and the adult Big Poe stands at (-0.5, 156.8, 108.6). The box moved 25 to the west so neither is inside it and no rock lands within 17 of the arrival; adult landing spots are at least 25 from the Poe.
- `field_south_east`: a navmesh obstacle (probably a tree) sits at about (132, 218); landing spots ring it.
- `desert_colossus_flats`: the Guay flock manager stands at (-152.2, 100.8, -44.6) and Guay spawn anchors lie west of the rock; box and landing spots sit east of it. No landing spot is on the liftable rock.
- `wasteland_dunes`: the adult map has five `Enemy_Leever_Fixed`, one at (-59.0, 99.2, -59.6), so the centre landing spot of the child layout was moved off it (all at least 12 from every leever). The Wasteland ground is open in a 40-unit radius.

The Wasteland sandstorm may hide the rocks until they are close: that is a view question for the playtest.

### Tests added (`difficulty/fireballs.rs`, game files)
- `the_trail_storms_run_on_every_visit_only_with_the_setting_on`: the five A_04 modules per age are found from the Trail maps; four are in `TRAIL`, the fifth has the demo flag and is not written; each rewritten graph keeps its node count and audits clean, its one deletable area is linked before and not after, the manager and flag nodes are intact, and building twice gives the same bytes. Setting off writes no Trail file; on writes exactly the 8.
- `the_new_spots_have_open_ground_and_the_ages_they_claim`: every anchor at least 4 from a navmesh wall in both ages; each new spot lists both ages.
- Extended: the map test (now 8 spots, 7 map files) also checks that no trigger box holds an arrival point, entrance or hole and no landing spot is within 12 of one (static and dynamic actors); the setting-off/on file lists; the anchors-on-the-ground test covers every age of the new spots.

### How to test it in the game
See `<playtest folder>\storms2\PLAN.md`: load each map (child and adult), stand in each box, and run up the Trail on a save that has already been to the peak (child) and as an adult before the Kakariko event.

### Rocks come from the volcano (user, 20 Nov)

(The Manager columns in the spot tables above are the old positions; the current ones are in `SPOTS`.)

Feedback: "the fireballs should always come from the direction of the volcano". Each spot's thrower (the `Obj_FallingRockManager_A_04`) now sits on the bearing from the spot centre toward Death Mountain, at the same distance and height as before (60 to 110 away, 55 to 80 up), so rocks arc in from that side. Landing spots, boxes and graphs are unchanged. The freed Death Mountain Trail throwers already stand on the volcano and are not touched.

Bearing is `atan2(dz, dx)` in map x/z (0 degrees is +x, 90 is +z, so negative is toward -z).

| Maps | Volcano reference | How it was found | Bearing per spot |
|---|---|---|---|
| `HyruleField_Child`, `_Adult` | (801, -370) | The map's own backdrop actor `Land_DeathMountainFar_A_01` (with its cloud effect `EftLoc_DeathMountain_Cloud_Far_00` at (801, 552, -368)); the Kakariko exit at (211, 177, 10) is on the same side | middle_road -35.8, south_central -35.5, west_road -27.6, south_east -38.6, near_market -32.1 |
| `HyruleCastle` | (1468, 478) | The same backdrop actor placed in the castle map | castle_front_garden 13.9 |
| `DesertColossus_*` | direction (187, -10) | No volcano model in the map. In the world the desert is far west of the field, so the volcano lies back through the Wasteland: the direction is from the temple entrance (-168, 10) to the Wasteland exit (19, 0), due east | desert_colossus_flats -3.1 |
| `HountedWasteland_*` | direction (182, 87) | No volcano model either. The Colossus end is west (-105, -56), the far end toward Gerudo Fortress and the field is east (77, 31); the direction runs along that axis | wasteland_dunes 25.5 |

These two are the weakest: they follow the world layout, not a placed actor, so they are the ones to check by eye in the game.

Tests (`difficulty/fireballs.rs`): each thrower lies within 30 degrees of the volcano bearing from the spot centre (now exact to rounding); the field and castle volcano references equal the backdrop actor's position in the map; the navigation mesh has no floor within 20 of the thrower's height under it in any age.
Not played.


## Moved from STATUS.md (20 Nov): sessions 5 and 6, fireball storms

- **Session 6 (20 Nov evening), seen by the user:** ordinary effects still work after the shared effect edit; the full green heal works. **Fireball storm, middle road: works** in the user's clip (`<playtest folder>\handson-6\notes\clip-181440.mp4`): rocks arc in and burn on the ground. It was slow to start, so the user first reported nothing. Frame rate about 30 to 37 in child Hyrule Field at 5x more enemies plus the storm (60 before). Bottle gives in this build stopped being taken by the game (bridge says sent; to look into). Castle garden storm: works, and the guards behave. Bowling grass: passed by the user. Gate editor v5 (fairy markers, minimal text, delete fixed, key 0) built, not yet swapped in.

- **New request (20 Nov): random fireball storms.** The Death Mountain eruption's falling fireballs at other places: the child Hyrule Castle courtyard sneak, quiet parts of Hyrule Field. Being built (`feature/fireball-storms`).

- **Fireball storms, more places (user, 20 Nov):** the Death Mountain Trail eruption on every visit, child and adult; plus four spots the user picked on the object map: Desert Colossus (-144, -43), Hyrule Field (136, 219) and (-8, 95), Haunted Wasteland (-56, -55). Being built (`feature/fireball-storms-2` off `difficulty`).


## After the 21 Nov hands-on (`fix/playtest-misc`, on `playtest/next` only)
- Seen by hand: the storm at `field_middle_road` is good and the rocks come from the volcano side (step passed 12:06, 12:11).
- Bug seen: standing in the middle road ring, rocks landed in a different ring (the one by the Market). Fixed in `fireballs.rs` (`e8347f8`); not confirmed in the game afterwards.
- Every area is twice as big now (trigger 64 x 60, landing spread to match). A Torch Slug (`Enemy_TorchSlug`) stands at the centre of five of the seven spots as a hint (not at the castle garden or `field_south_east`); a unit test keeps it 8 units from every landing anchor.
- Desert Colossus is switched off: walking toward it made Link fall through the floor. `SPOTS` has 7 entries now (the desert spot is gone). Not fixed, only removed.
- Future: swap the Death Mountain backdrop to its erupting version during a storm (user, 12:06).
