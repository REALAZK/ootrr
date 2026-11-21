# Enemies block the way

Setting `enemy_gates` (number 129), checkbox "Enemies block the way" on the Difficulty page, default off. Built on `feature/enemy-gates`. Experimental, never loaded in the game. Code: `crates/core/src/difficulty/gates.rs`.

## Request
"enemies block progression. not all enemies, but many, should block the way-forward. this could be by adding a new lock to the existing door, or by putting up some temporary wall. the idea here is that if enemies are too hard to defeat, you should retreat and come back when you're stronger. it should never make a game impossible to beat, or lock you without the option to return. obv in areas like hyrule field this doesn't make sense, and some rooms already have this (like the stalfos at the end of the lost woods) but there's some obvious places where this would make sense, like the deku babas between the kokiri forrest and the deku tree"

Then, simplifying it: "do we really need to perma-save the state of these enemies-block-passage? can't it just be that if a particular enemy is alive, the wall is in-place, and when the enemies are defeated, it goes away? seems pretty stateless to me."

## What it does
No saved state and no new flag. The wall exists exactly while its guarding enemies are alive. If the area loads again and the enemies are back, the wall is back. The earlier blocker (needing a persistent flag) is gone.

## Gates (an explicit list, `GATES` in gates.rs)
1. The two Deku Babas on the path from Kokiri Forest to the Deku Tree (child map only). Babas at (39.12, 113.19, -18.58) and (37.74, 113.26, -19.59). The path there runs north to south, so one invisible wall (`FldObj_AirWall_Player_GenerateNavMesh`, the kind Mido's blocker uses) stands across it at z = -19.08, 16 wide, with the two babas on it. The other three babas of that map are untouched.

I did not add more gates. None of the other candidates could be checked as safe for the same reasons (see below).

## Mechanism
- **Death signal:** a Logic graph. The nodes are `ActorLogicSimpleActor` ones, the same kind the game uses for an enemy whose `Dead` output is read (the wolfos in `Gimmick_DefeatEnemy_Open_03`) and for air walls (`FldObj_AirWall_Player_Unridable` in the Death Mountain Trail graph). For each guard the build adds two nodes: a guard node, and a wall node whose `Logic_Delete` input is wired to the guard node's `Dead` output. With two guards there are two walls stacked at the same place, so the passage opens only when both are dead, with no AND node needed.
- **Host graph:** no vanilla graph has the shape "N enemies dead, then delete one plain actor" without a save flag or a typed enemy node. The closest are `Gimmick_AllDay_Skulltula_115c` (typed to the Gold Skulltula) and the counter rooms, which all set a persistent flag. So the build adds the four nodes to `Gimmick_AllDay_Skulltula_115c`, the smallest graph, exactly the way `minibosses.rs` adds a node, and binds them through a new `AiGroups` entry in the Kokiri Forest child static map, the way `ambush.rs` does. Other maps that use the same graph get four extra nodes bound to nothing.
- **Dynamic to static:** the five babas live in `KokiriForest_Child_Dynamic`, which has no `AiGroups`, and across 40 maps no group refers to a dynamic actor. Graphs only ever bind static actors. So the two guard babas are moved, unchanged, with their `Presence` flag, from the dynamic map into the static one. None is added or lost (the game test counts them).
- **Same flag on the wall:** each wall copies the baba's `Presence` (`KokiriForest_DeleteDekubaba`, negated). Once the Stone of Forest is yours or the Kokiri 0150 event has played, the babas are not placed and neither is the wall, whatever the graph says about a guard that was never created.
- **Wall choice:** invisible, because the vanilla visible barriers (webs, brambles, Mido's gate) burn, break or need an event, and a barrier that can be bypassed is not a gate. An invisible wall is confusing, so the build report says where it is.

## Never trapping the player
- The babas stand in the wall (within 0.5 of its plane), so a sword swing, the boomerang, a slingshot shot or a bomb reaches them from both sides. Coming out of the Deku Tree with the babas alive again you meet the wall from behind and can kill the babas from there.
- Warp songs, Farore's Wind and "Return with Navi" do not land in this corridor. Entrance shuffle can put you out of the tree on the far side, so the gate is skipped when entrances are shuffled.
- If you have no weapon at all, Deku Sticks and the tree's own contents still give one.

## Never making a seed unbeatable
`KF Outside Deku Tree` needs `is_adult or open_forest == 'open' or 'Showed Mido Sword & Shield'`, and the Mido event needs the Kokiri Sword. With a closed or closed-Deku forest a child already needs the sword (and the logic already counts it as the way to kill these babas: "Deku Baba Sticks" needs it or the Boomerang), so the gate adds nothing. With `open_forest` open a child needs nothing, so the gate is skipped there. Adult Kokiri Forest is untouched. The build report says when the setting is on but nothing is gated.

## Earlier findings that still matter
- Only Logic graphs react to deaths; map actors have no link that does.
- `FldObj_AirWall_*` actors take a `Presence` block. Mido's wall is `FldObj_AirWall_Player_GenerateNavMesh` at (25.17, 110.84, 3.79), scale (1.11, 1.5, 0.12), next to `Land_Kokiri_FieldProp_StoneGate_A_01`.
- The Deku Tree entrance is at about (90, 115.5, -28). Coming out of the tree puts you east of the wall.
- Counter rooms (Spirit 04_03, Ice Cavern 01 and 04, Forest Temple 06 and 18, Jabu, Shadow) set persistent flags, which is why none can be reused.

## How it was tested
- `cargo test -p ootrr-core difficulty` (96 pass, with `OOTRR_ROMFS` set) and `cargo test -p ootrr-core settings` (107 pass), `cargo build -p ootrr`.
- Unit tests: guards found by position and moved with their flag, missing or flagless guard refused, a wall per guard with the guard's flag, sorted unique hashes, round trip of the map, skipping under open forest or shuffled entrances, setting read and settings string unchanged.
- Game-file test (gated on `OOTRR_ROMFS`): guard count preserved across the two maps, walls added, old actors untouched, groups sorted, graph audits clean, each wall's delete input wired to its guard's `Dead` output, same output on every build.
- A real build with the setting on completed; the mod holds both Kokiri Forest child maps and the edited graph.

## In the game: what to check first
1. Kokiri Forest (child, closed forest) loads at all, with the Deku Babas where they were and no crash. A crash on loading the forest points at the moved babas or the new graph nodes.
2. Walk up from Mido's gate: is the invisible wall at the second baba pair and does it stop you? Is anything bypassed around its ends?
3. Kill both babas: does the wall go? Kill only one: it should stay.
4. Go through the tree and come out: you are east of the wall, the babas are back, so is the wall; hit a baba from this side.
5. With the Stone of Forest, the wall must not be there.
If the engine refuses a simple actor node bound to a baba, the next step is a vanilla typed node or a counter room.


## Redesign (branch `feature/gates-v2`, never played)
User, after the hands-on session: "there was no wall. don't make it invisible. use the burnable spiderweb wall but give it the purple glow that invincible enemies have." Earlier: "the wall shouldn't be invisible. make it visible. make it mido, or maybe the spiderweb wall but with the purple glowing effect."

### What was built
- Each guard now has, besides the air wall, a visible web: `Obj_InsideTheDekuTree_SpiderWeb_A_02` (the solid, elastic wall web used in the Deku Tree, Hyrule Field and the Shadow Temple) plus its cosmetic partner `Obj_InsideTheDekuTree_InteractiveSpiderWebParts_A_02` (instance name `SpiderwebPart`), joined by a `Reference` link from the web to the part, as in the game's own maps. Two webs stand on the wall line at x 36.4 and 40.4, one per guard, at scale 1 (the game places these with no scale).
- The air wall stays as the real blocker and is thicker (z scale 0.12, like Mido's blocker, instead of 0.03). The old wall was thin and Link crossed it in the hands-on session.
- Each web, part and wall is deleted by the Dead output of its guard through the same graph nodes (`ActorLogicSimpleActor`, `Logic_Delete`), so the webs of a guard vanish when that guard dies and the air wall vanishes when both are dead. Webs and parts copy the guard's `Presence` flag.

### Fire-proof: not done
The web dies by fire through its chemical parameters (`RequestDeathIfBurned` and `RequestDeathIfExploded` are true in `Obj_SpiderWeb`) and through its AI graph `Spiderweb_Wall` (burning leads to a death request; a lit Deku Stick sends an ignite message). Changing that needs a private copy of the actor (a new actor class with its own pack, rows in the actor info tables, the tag table and the resource size table, and an edited AI graph), as the cow clone feature does. That is data only but large and cannot be checked here, so it was not built. Instead the air wall holds when the web burns: fire removes the picture, never the blocker. The closest alternative if the user wants an unburnable visible barrier: a cosmetic web prop (`Land_Deku_SpiderWebProp_*`, no chemical parameters, no collision) in front of the air wall; its size is unknown.

### Purple glow: not done
The tint is not a model or actor parameter. It is applied by the native node `ExecuteSetEnemyDamageColorAndLight` in `AI/EnemyCommon.common.reactiondamagecolor.module.ainb` (the node's only data is its name; the colour lives in game code) and runs from the enemy graphs. No map actor key, model info or material parameter that a non-enemy object can carry was found. A glow would need game code or a rebuilt model, so the webs have none.

### Why the user saw no wall
Not the open-forest setting: `open_forest` defaults to "closed", so the gate is on by default, and the preset in the hands-on session set `enemy_gates` with entrances off. The map file of that session's mod could not be inspected afterwards (the mod folder was replaced), so the cause is not proven. Candidates: the wall was only 0.3 thick (now 1.2), a guard counted as dead (Dead true) before it was spawned, or the mod lacked the map. Nothing else in the difficulty code edits the Kokiri Forest map files, so a later merge overwriting it is ruled out. The test must print the mod's map and graph first (see `playtest/gates-v2/TESTS.md`).

### Unverified
Everything in the game: that the map loads, the web size and facing, that the air wall blocks, the deletion by the guards, the return of the webs.


## Session fixes (18 Nov, feature/session-fixes, not played)
User, with the gate v2 build: the web is visible and blocks, a bomb destroys it, and after that, with both guards alive, he could walk through.
- Why the wall failed: the invisible wall was `FldObj_AirWall_Player_GenerateNavMesh`, whose box is 4 by 4 by 4 before scaling (half extents 2). At scale (1.6, 1.5, 0.12) it was 6.4 wide, 6 high and 0.5 thick, not the 16 wide the earlier text said, so it was narrower than the two webs together (34.4 to 42.4). The game places that class in `_Dynamic` maps (Mido's blocker); our wall is in the static map, where the game uses `FldObj_AirWall_Common_Cube` (six in this map). The earlier script test that "stopped" Link at z -19.9 was within a metre of a guard Baba and may have hit the Baba.
- Change: the wall is now `FldObj_AirWall_Common_Cube` (a 1 unit cube standing on its origin), at (38.43, 112.2, -19.08), scale (16, 7, 1.2): x 30.4 to 46.4, y 112.2 to 119.2, z -19.68 to -18.48. Both guards and both webs are inside it. Same deletion wiring (both guards dead).
- Webs: still the Deku Tree wall web and its part. A bomb or fire can still destroy the web, but the wall stays, so the way stays shut until both guards are dead. A bombed web leaves an invisible wall.
- Not built: a private bomb-proof web. It needs a new actor class (pack, actor info rows, tag table, size table, edited AI and life data) and a wrong entry would crash Kokiri Forest on load with no way to check here. The air wall does the blocking. The purple glow is still not done.
- Test: `the_wall_covers_the_webs_and_the_guards_and_uses_a_class_the_static_map_already_holds`.
- In the game: see `<playtest folder>\session-fixes\TESTS.md`.


## Redesign v4 (branch `feature/gates-v3`, never played)
User, after v3: the gates were "the wrong way around" and Link got stuck at the Death Mountain one. Redesign, in the user's words: "ideally, if possible, the gates will not be there, and then when you approach the guard, it does the little cutscene it does to show the gate appearing -- like it does w/ the stalfos at the grove -- and then when you kill the guard it does the little cutscene again to show it go down. another good side-effect of this is that we'll never spawn the gate unless you're already on the guard's side of it."

### The game's own construct
The Sacred Forest Meadow wolfos fight (`Gimmick_DefeatEnemy_Open_03`) is one AiGroup: a player-sensor area (`ActorLogicAreaActor`, `HoldActorObserverBySensorHitInfo`, `OperateANDFixInputTrue`) that creates the enemy, an `ActorLogicEnemyCounter` for the purple barrier (`Obj_EnemyCounter_A_08`), and an event request actor that plays the short scene when the enemy dies. The barrier's own graph counts the enemies linked to it, deletes itself when there are none at start, and on `Finish` plays its end effect and deletes itself. The vanilla group also sets a saved flag and its scene is placed at that one spot.

### What was reused
Area, sensor and AND nodes (built field by field as in `ForestTemple_Level_Room_01_Prefab_c0f3`), the counter node, the barrier actor and its reference link. Per gate the build adds an `Area` actor (box 20 x 8 x 20 round the guard mark, `RADIUS` 10), one barrier piece per 4 units of wall (marked ends plus 1.5 each side), and wires each counter's `Logic_Create` to the AND output and `Finish` to the guard's `Dead`. The barrier does not exist until the player is in the guard's box; a guard killed from outside makes it delete itself on creation.
- Not reused: the two scenes. Their event request actor and camera belong to the one meadow spot and the saved flag would be shared by every gate. The barrier rises and drops with its own animation and effect, no camera. Unverified in the game: that the rise animation plays when a piece is created late.
- Facing: the pieces face the guard spot. The side and arrival inference of v3 is gone (the earlier uncommitted edit, facing the guard spot whatever the mark order, was kept and tested).
- Guards: enemies within 8 units of the guard mark (also ones with a presence flag, like the withered Babas); harmless scrubs and Gold Skulltulas are ignored. New: Lizalfos in Zora's River (`ZORA_GUARD`), Wolfos in Lost Woods (`WOODS_GUARD`; that map holds no fighting enemy), a Dead Hand with its four hands in the Market (`MARKET_GUARD`), `Enemy_MiddleBoss_GerudoThief` in Gerudo Valley (`GERUDO_GUARD`; in the fortress it runs in a carpenter-set group with extra links, so outside it is unverified), an Iron Knuckle awake from the start in Hyrule Field (`FIELD_GUARD`, same wake patch as the special encounters). Dead Hand, Iron Knuckle and the Gerudo are always new, never an existing enemy.

### Never trapping the player
The barrier rises only with the player in the guard's box. Flagged: gate 4 (the guard stands beyond the wall for a player walking east; from the west the barrier rises behind you); gate 2 (climbable wall beside the barrier, where Link was wedged in v3: check it); gates 9 and 10 (two doors of one small room: both barriers rise, the player is shut in until the guards die, as in the game's fight locks and as marked). Gate 8 (no guard, same wall as 9) is dropped. Gate 6 stays closed-forest only: with an open forest a child may have no weapon.

### Tests
`cargo test -p ootrr-core difficulty` and `settings` pass: geometry, facing whatever the mark order, trigger box, hands inside the box, guard selection, graph audit with the area, sensor and AND nodes, create wired to the AND, finish wired to the guard, determinism, and the game-file test over every gate map. In the game: `<playtest folder>\gates-v4\TESTS.md`.

## v5 (branch `feature/gates-v3`, after the user's hands-on of v4, not played)
v4 in the game: gate 3 worked exactly as designed (nothing until near, barrier rose, dropped when the Lizalfos died). Faults found, in the user's words: "the guard spawns immediately but the gate only appears when i get nearby. the guard should spawn when the gate comes up"; at gate 5 the barrier rose with no Lizalfos present and killing the nearby Tektites did not open it; at gate 1 an extra Mad Scrub sat exactly on top of one of the game's two.

- New guards are created by the barrier's trigger. Every new guard (Lizalfos, Wolfos, Dead Hand and its four hands, Gerudo fighter, Iron Knuckle) has a graph node whose `Logic_Create` is wired to the sensor AND output, the same wire the barrier counters use and the same way the meadow wolfos is created. A gate that makes its guards never takes a vanilla enemy near the mark as its guard (gate 5's blue Tektites).
- Gate 1's extra scrub: the game's Mad Scrubs are already owned by a vanilla AiGroup. v4 added a second node for them with `Logic_Create` constant true, so the engine created them again. Now a guard that a vanilla group references gets a node with `Logic_Create` constant false (creates nothing); a guard nothing owns (the withered Deku Babas moved from the dynamic map) keeps a constant true, as before. Tektites (gate 2) and scrubs (gate 1) are owned, so they get false. Unverified: that the false node still reports `Dead` when the vanilla owner kills the enemy. If gate 1 or 2 never opens, that is the cause.
- Marks: gate 3 is replaced by wall 14 (left -60.62 100.11 -8.67, right -64.0 99.85 -7.09, guard -64.05 99.71 -14.09). Gate 5 keeps its wall, with two Lizalfos at the marks (34.73, 106.31, -24.81) and (29.84, 106.68, -20.6), both created by the trigger; each has its own barrier stack, so both must die.
- Private graph per gate: each gate writes its own copy of the logic graph (`Logic/Gimmick_EnemyGate_NN_xxxx.logic.module.ainb`, module `Logic/Module/Gimmick_EnemyGate_NN/...`) and its AiGroup points at it. The vanilla `Gimmick_AllDay_Skulltula_115c` graph is no longer written. A new file has no size table entry, so `staging::grow_out` now adds one for a path that is missing from the table when `old` is 0.
- Gate 6 (review finding 2 and 3): skipped when the forest is open, and when `open_kakariko` is open with a forest that is not closed (that story state takes Mido away, so the child could reach the babas with no sword). With a closed forest or Closed Deku and Mido present, the child has already shown Mido the Kokiri Sword, so a weapon is guaranteed. The other gates were not told to the logic at that point; see "Sword condition" at the end.
- Gate 4: left built. Its guard is beyond the wall for a player walking up-river, so the barrier rises behind them; harmless now that the guard is created with it, but the user should re-mark it.

### A barrier with no living guard
Covered now: the guard is created at the moment the barrier is, so it cannot have wandered or fallen away before the player arrived; the barrier counter deletes itself when it has no enemy at start; and, if the game's wiring follows the wolfos fight, leaving the box turns the trigger off and removes both barrier and guard, entering again brings fresh ones.
Not covered: a guard that leaves the area or is lost after it was created while the player stays inside. The counter's `Finish` is wired to the guard's `Dead` output only. A "no longer alive" test (the `Logic_Alive` output) was not wired: before the guard exists it would read as not alive and could drop the barrier the moment it rises, and what `Dead` and `Logic_Alive` read for an unloaded or despawned actor cannot be learned from the files. Test it in the game.

## Round 2 (branch `feature/gates-r2`, from the user's hands-on of v5, not played)
v5 in the game: gates 3, 4, 5 and 13 worked, gate 6 "perfect". Gates 1 and 2 never opened, gate 11 stood on the entrance, the Gerudo fighter and the Lost Woods guards needed changing.

- Gates 1 and 2 no longer adopt the game's enemies (the "creates nothing" node never reported their death). Each now creates its own guard with the trigger, like gates 3 to 5: a Mad Scrub (gate 1, at (-6.4, 104.81, -53.4), about 2.5 units from the nearest vanilla scrub) and a Red Tektite (gate 2, at its old mark). The vanilla scrubs and tektites are left exactly as the game has them. Only gate 6 still adopts existing enemies (its withered Deku Babas, wiring unchanged). The new scrub has no leaf bush with it.
- Same-side rule, every gate: the trigger box is still centred on the guard mark, 20 x 20, but is shrunk (never moved) until all four corners lie at least 1 unit on the guard's side of the wall line (a wall line is infinite here, so it is stricter than the segment). Minimum half-width 2. Marks nearer the wall than 11 units change: gates 2, 3, 4 and 6 (7 to 9 units away, box about 11 to 16 wide on the wall side) and the room, gate 9 (box 7 wide, between the two doors). Gate 11's mark is 19.6 units from its wall, so its box stays 20 x 20 (z 22.6 to 42.6); the wall (z about 52.2, x -4.5 to 0.65) is the south entrance, the one the player walks in through towards -z, so entering and walking to the Dead Hand raises the barrier behind the player. Not checked against the map's spawn points.
- Gate 12: two Stalfos (`Enemy_Stalfos`, the Forest Temple kind) 3 units apart at the mark, both must die. Gerudo fighter as a guard: to fix later (cutscene on defeat, jail on loss); the user likes the fight.
- Gates 9 and 10 are one gate (id 9): one Stalfos at the middle of the small room, one barrier stack for each door, both fed by the single guard.
- Gate 13: barrier pieces lowered by 0.5. The marked ends are at height 136.64 and 135.53; pieces stand at the lower mark minus 0.5, 135.03.
- Red guards (not found): the vanilla Wolfos fight carries no tint-related key. Its barrier actor has only `IsSkipClose`, its Wolfos only `Drop__DropTable` None and `IsCheckHidingHoleAsDungeonRoom`, and the link is a plain Reference. The gate sets nothing on a guard. Only the adopted guards of gate 6 were seen red (the user did not report the created Lizalfos as red), and the withered Deku Baba's own model has a red-brown head and red leaves; the screenshot shows the Z-target arrows, which are red for enemies. Open: compare a withered baba in an unmodified game.

## Review findings from 18 Nov (history, addressed in v5 above)
From the code review of 18 Nov (`docs\code-review-2026-11-18.md`, findings 2 and 4). The review read the old `gates.rs`; I checked `feature/gates-v3` (commit 81597f3, `crates/core/src/difficulty/gates.rs`). The same problems are all still there. Nothing was fixed; v4 is unmerged and unplayed.

1. Nodes added to a logic graph other maps use (still present).
   - `gates.rs:16-17` name the vanilla module `Gimmick_AllDay_Skulltula_115c` and its graph.
   - `gates.rs:337-345` give each new AiGroup `Logic` = that module.
   - `gates.rs:354-430` (`wire`) add per gate a guard node per guard, an `Area` node, a sensor node, an AND node and a counter node. `gates.rs:466-507` (`stage`) read that vanilla graph and write it back under the same path (`outs.push(Out { path: GRAPH ... })` at 507).
   - Every other group in the game that names this module now also runs those nodes, whose `InstanceName` values appear in none of that group's References. `minibosses.rs` refuses this case ("a new node would break the other room"). Only Kokiri Forest and the gate maps were load-tested.
   - Fix: write the nodes into a new graph file with its own name (and size table entry, as backup does for its holder files) and point the new groups at it; or list every map that references the module, load-test them, and record them here.
2. The forest gate assumes the player has a weapon (still present).
   - `gates.rs:77` (gate 6, the path from Kokiri Forest to the Deku Tree, `closed_forest_only: true`) and `gates.rs:159-161` (`active`): the gate is skipped only for an open forest or shuffled entrances.
   - The guards are the two withered Deku Babas. A child with no sword or slingshot cannot pass them, yet the seed logic may put an item behind the Deku Tree that needs nothing. The same holds, more widely, for every gate whose guard needs a weapon: gates 3 to 5 (Lizalfos), 7, 9 and 10 (Wolfos), 11 (Dead Hand), 12 (Iron Knuckle from the Gerudo thief), 13 (the Iron Knuckle knight). None of them tells the logic.
   - Fix: skip a gate unless the seed guarantees a weapon at that point, or accept the requirement and say so in the note; prove it per gate.
3. `closed_deku` with an open Kakariko gate can remove Mido (still present, not changed by v4).
   - `active` takes only `forest_open` and `entrances`. The Kakariko gate setting says an open gate "takes Mido away", and that is allowed together with `closed_deku`. If the guards' `Presence` flag is still true in that story state, the gate stands where the game itself would let the child through.
   - Fix: also skip gate 6 when any setting removes Mido, or prove from the map files that the guards' `Presence` flag is off in that state and write that here.

## Round 3 (19 Nov): the trigger starts well past the wall

Seen on 19 Nov: gate 12 worked perfectly and is now shipped (`HELD` is `[1, 2, 7, 9, 11]`). Gate 11 raised its barrier while the player was still outside it, at the south entrance, and the game's own respawn point after a death is on that line.

- Rule from the user, every gate: the player must be at least 2 units past the wall on the guard's side before the barrier rises and the guard is created.
- Margin now `INSET` = 2.5 + `BARRIER_HALF` (1.0, a conservative half-depth of the barrier piece) = 3.5, measured along the wall's normal toward the guard, to every corner of the trigger box. The box is shrunk (floor half-width 2); if that is not enough its centre is moved away from the wall; if two walls leave no room (gate 9) the floor drops to 0.5. A test checks all gates.
- Gate 11 evidence: the box was 20 x 20 round a mark 19.6 units from the wall, yet it fired at the wall. 32.58 + 20 = 52.58 matches the respawn z of 52.6, so the game very likely reads the area `Scale` as half-extents, not full size. The `Scale` written is now the computed half-width (so the real box is the same or smaller than computed, whichever reading is true). If triggers feel too small, double it.
- Nothing persists after a death or Try Again: the map reloads and the barrier pieces and created guards come only from the graph's `Logic_Create` when the trigger fires; no flag or save entry is written for them. Gate 6 adopts the game's own Deku Babas (their own presence flags are the game's) and its barrier is created the same way, so the same holds.
- Gate 13's lowered barrier and the same-side rule are unchanged.
- Not verified in the game.

## Round 4 (19 Nov)

Gates 2 (Death Mountain Trail) and 9 (the small Lost Woods room, one guard, two gates) were seen working and ship. `HELD` is now `[7]`: only gate 7 (Lost Woods east passage) is held, not tried. Gate 2's guard changed from a created Red Tektite to a created `Enemy_Dodongo` (the fire-breathing one, not `Enemy_BabyDodongo`) at the same guard position on the ground; the new-guard kind is `DODONGO_GUARD`. The actor pack exists and the game creates it in other maps without room logic. The Dodongo version itself is not played yet.

## rc 13 scripted check (19 Nov)
- Gate 2 (Death Mountain Trail): the map loads with the mod; after a warp to (-54.67, 106.0, 10.4) on the guard's side an `Enemy_Dodongo` (hp 6) stood at (-54.4, 106.4, 3.2) and moved under 1 unit in 25 s. Barrier and fight not observed. Details: `playtest
c13\RESULTS.md`.

## Entrance shuffle (19 Nov, branch `feature/gates-er`, not played)
User: "none of the enemy gates should block in such a way that it conflicts with the rando." Before, any `shuffle_entrances` other than off turned every gate off (a Dungeons seed lost the Lost Woods gate). Now `active` takes the value (`Shuffle::Off`, `Dungeons`, `Wide`; Wide is overworld, everything or an unreadable value, which fail closed).

| Entrances | Gates on | Gates off |
|---|---|---|
| Vanilla | all (7 still held) | none |
| Dungeons | 3, 4, 5, 9, 11, 12, 13 (7 held) | 1, 2, 6 |
| Overworld, Everything | none | all (these values are refused by the build today) |

Why 1, 2 and 6 go: each stands in a map that holds a dungeon door, and the door is on the far side of the wall from the guard (Forest Temple at z -121.7 beyond the wall at z -82.6; Dodongo's Cavern at z -31 beyond z -2; Deku Tree at x 92 beyond x 45). A dungeon exit, or a blue warp (blue warps stay put when dungeons are shuffled), lands you on that far side. The logic treats the way from there to the rest of the area as free, the barrier is not in the logic, and for gate 6 the closed-forest guarantee (the child has shown Mido the Kokiri Sword) no longer holds once another dungeon's exit can drop you east of the babas. The other gates sit in maps with no dungeon door (Zora's River, Lost Woods, Market, Gerudo Valley), or, gate 13, in Hyrule Field with the Water Temple door 300 units away at Lake Hylia on the guard's side of the wall, so the player meets that barrier exactly as in vanilla. Dungeon shuffle changes no overworld route, so these gates add what they add with entrances off.
The rule is computed from the entrance catalog in a test (`a_gate_is_left_out_exactly_when_a_dungeon_door_lies_beyond_its_wall`), and the constant `BEYOND_DUNGEON_DOOR` must match it. The build report names the gates left out and why (`shuffle_note`).

Logic conflict: unchanged by entrance shuffle for the gates that stay. A gate adds "beat the guards"; a child never meets a gate before holding the sword (see "Sword condition" below).

How tested: `cargo test -p ootrr-core difficulty` (187) and `settings` (106) with `OOTRR_ROMFS`; a real build with `enemy_gates` and Entrances Dungeons: report lists gates 3, 4, 5, 9, 11, 12, 13 and names 1, 2, 6 as left out; the mod holds `Gimmick_EnemyGate_09_*.logic.module.ainb` and its `LostWood_Child_Static` map reads back with 4 barrier pieces (`Obj_EnemyCounter_A_08`) and 1 `Enemy_Stalfos`. Not played.
In the game: Entrances Dungeons plus the gates, child, walk into the Lost Woods small room (gate 9: barrier at both doors, one Stalfos). Check Kokiri Forest path to the Deku Tree, Death Mountain Trail and the Meadow scrub lane have no barrier. Overworld and Everything are refused by the build, so they cannot be played yet.

## Sword condition (20 Nov, branch `feature/gate-sword`, not played)
Your decision (20 Nov), replacing the earlier logic requirement: "put some logic so that all the enemy gates don't spawn until you've got the sword at least". A gate (its barrier pieces and its guards) does not exist while the player has no sword. Once the player holds one, the gate behaves as before: guards die, the barrier goes. Because a swordless player is never blocked, the item logic needs no weapon requirement; the earlier `gate_logic.rs` module and its `enemy_gates_logic` hook were reverted.

Condition: the saved game flag `IsGet.Weapon_Sword_Kokiri` (a Bool, saved, default false; found in the game's flag list, and read by the game itself for the Kokiri house door and the forest tutorial). It is an inventory-possession flag set when the sword is obtained, not "equipped", so taking the sword off or swapping to Sticks does not remove the gates. Each child gate graph gets one `HoldGameDataGetBoolSimple` node reading it (index -1) and:
- the trigger AND node ("player inside the guard box") becomes `OperateAND(player inside, sword flag)`, a two-input multi AND built with the same helpers as the Gerudo card gate. The new guards (gates 1 to 5, 9, the child ones) and the barrier pieces are created from that node, so without the flag none is created;
- the vanilla Deku Babas of gate 6, which were created always, are created from the sword flag node instead;
- adult gates (11, 12, 13) are unchanged: the adult always holds the Master Sword. Biggoron's and the Giant's Knife are not checked: a child cannot own them, and an adult already has the Master Sword.
Gate-off output and default seeds are unchanged.

When the sword is obtained: the flag is read by the graph, so the gate may appear at once on the map the player stands in, or only when the map next loads. Both are acceptable (a gate whose trigger is somewhere else cannot fire until the player walks into the box anyway). To be confirmed in the game.

Risks: (1) a Kokiri Sword from the Starting items option is pushed into the pouch at new game and no `IsGet` flag is set by us for it (the existing tests insist on that). If the game's pouch push does not set `IsGet.Weapon_Sword_Kokiri`, such a player would see no gates at all; the failure is safe (easier), not a block. To confirm in the game with the control bridge. (2) `IsGet.*` is the flag the item logic simulation also assumes to be set on pickup.

Guards the child's sword may not kill: none was found by reading. The logic's own enemy rules said the Kokiri Sword kills the Lizalfos, the Stalfos, the Dodongo (gate 2) and the Mad Scrub (gate 1); that is rule text and has not been checked in the game for the remake. Gate 2 (Dodongo) is the one to watch.

How tested: `cargo test -p ootrr-core difficulty` and `settings`, `cargo test --workspace` with `OOTRR_ROMFS`. Every gate graph built from the game files is checked: each child barrier and each new guard has its Create input on a two-input AND of the area hit node and the sword flag node; gate 6's babas read the sword flag node; adult gates keep the plain trigger; the graph audits clean; the flag is a saved Bool in the game's flag list. Played: nothing. See `playtest/gate-sword/PLAN.md` for the scripted test.

## Release (rc 16, 20 Nov)
In rc 16: the entrance-shuffle rule above, on top of the sword condition (child gates wait for the Kokiri Sword; `gate_logic.rs` is not included). Shortened build note: "Because dungeon entrances are shuffled, these gates are left out, as a dungeon door lies beyond them: ...".

- **By script** (checks-1120, entrances Dungeons, closed forest): gate 9 in the Lost Woods is present and holds Link at its barrier; gate 2 on Death Mountain Trail is absent.
- **Not tested:** that gates 1 and 6 are absent (vanilla enemies stand at both spots, so a script could not tell).


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 20):** Enemies block the way
- **State:** Round 4: gates 2, 9, 11, 12 ship; 7 held
- **What was seen in the game:** Gates 2 and 9 seen working (gate 9: "lost woods 1 guard 2 gates confirmed"; gate 2 "worked perfect"). Gate 2 guard is now a created Enemy_Dodongo (not played yet). Only gate 7 stays held. Build in <playtest folder>\tweaks-r4 rc13 (19 Nov): gate 2 guard seen: Death Mountain Trail loads, an Enemy_Dodongo (hp 6) stands at (-54.4, 106.4, 3.2) after warping to the guard's side.
- **Next step:** Play the Dodongo guard at gate 2; try gate 7 (Lost Woods east passage); gate 11 from the south


## Moved from STATUS.md (20 Nov): gates: moved notes

- **New gate (user, 20 Nov): Death Mountain Crater (child), bottom-left ledge**, from a marked object-map screenshot: a Dodongo guard and a wall. Being built (`feature/gate-crater`), with the image mapped to world coordinates and a logic-safety check.

- **Gates do not spawn until the player has a sword** (answers Q1 and Q6). The logic requirement merged this morning (4194f62) is to be replaced by this. Being built on `feature/gate-sword`.

- **Hands-on session 3 (20 Nov afternoon), seen by the user:** first pickups and chest boxes (#53); hints; no area intro on entering Kakariko; boss adds at fight start (Phantom Ganon); regen heal flash. Found: gates and Zora ambush vanished when walking up close, my regression in #52. Fixed in `cc3572a` (trigger held once entered); on local `difficulty`, to push. Zora ambush and the gate kill still to re-check.

- **Push needed:** `difficulty` has the gate trigger fix `cc3572a` (confirmed by hand) for #52.


## Gates 14 to 22 (20 Nov, branch `feature/gate-crater`, not played)
Your requests (20 Nov), each a line and dots drawn on an object-map screenshot:
- Death Mountain Crater, child: "put a Dodongo guard and wall here".
- Death Mountain Trail, child: "and here as child. 4x Dodongo as adult."
- Graveyard (no age named): "put 3 Poes as a guard here".
- Hyrule Field, child, the neck down to Lake Hylia: "put 3 Lizalfos guards here".
- Lon Lon Ranch north gate, child: "put a giant peahat and 5 small peahats as guards here".
- Lake Hylia, the lab bridge (no age named): "put a like like and 4 keeze as guards here".
- Ganon's Castle entrance, adult: "add a dark link guard here". Not built, see below.

### From screenshot to world coordinates
The object map draws north up, x to the right, z downward, at one scale, so each screenshot is `px = s * x + cx`, `py = s * z + cz`. The scale and offset come from landmarks found both in the picture and in the map files; then the map's navmesh was drawn over the picture to check the whole outline.
- Crater: Bolero warp pad (the hexagon), the two lava rings (their smoke emitters), the Fire Temple door and the Goron City passage. s = 13.41, 5 landmarks, residuals 0.05 to 0.78 units.
- Trail: the Kakariko and Goron City loading zones. s = 6.95. The navmesh outline lies on the drawn trail within about 1 unit.
- Graveyard: the Nocturne warp pad and the Kakariko gate. s = 12.87. The outline matches.
- Hyrule Field: the lake laboratory and the castle drawbridge. s = 3.06. The outline matches. The ranch screenshot was read through the oval track (about 3.2), then the wall was put on the gate posts drawn in the field screenshot (the track reading was about 8 units off in z, so the posts and the navmesh decided). The lake screenshot: the laboratory and the owl island, s = 8.03 in both axes.

Each barrier end was then moved to where the navmesh floor along the drawn line ends (the passage walls), at floor height, and each guard to open floor on the side the dots were drawn, at least 4.5 units from the wall so the trigger box fits. A connectivity check cut the navmesh with the barrier pieces and confirmed the two sides no longer join (trail, field neck, lake shore with the lake drained).

### The gates
| Id | Where | Map | Wall ends | Guards |
|---|---|---|---|---|
| 14 | Crater, west ledge by the Deku Scrub | DeathMountainCrater_Child | (-26.75, 119.34, 33.22) to (-25.12, 119.32, 32.05) | 1 Dodongo at (-19.5, 119.58, 35.5), trigger radius 5 |
| 15 | Trail, the way up past the junction | DeathMountainTrail_Child | (-16.99, 122.43, -29.2) to (-8.78, 122.67, -29.2), plus (-6.0, 120.0, -29.2) to (3.8, 120.0, -29.2) across the lower terrace | 1 Dodongo at (-14.0, 123.61, -24.0) |
| 16 | the same | DeathMountainTrail_Adult | (-16.82, 122.28, -29.2) to (-8.89, 122.7, -29.2), plus the terrace wall | 4 Dodongos: (-14.0, 123.13, -24.0), (-12.5, 123.05, -26.3), (-16.5, 121.79, -20.5), (-17.5, 123.69, -17.0) |
| 17 | Graveyard, the way to the Shadow Temple | Graveyard_Child | (72.35, 108.56, 0.62) to (72.35, 108.46, 4.34) | 3 Poes (`Enemy_Poe`): (67.6, 108.64, 2.5), (69.0, 108.62, 0.9), (69.0, 108.54, 4.3) |
| 18 | the same | Graveyard_Adult | the same | 3 Poes, same places at adult floor height |
| 19 | Field, the way down to Lake Hylia | HyruleField_Child | (-104.13, 139.6, 371.34) to (-61.78, 139.6, 401.59) | 3 Lizalfos: (-80.6, 141.11, 374.0), (-88.8, 141.48, 371.7), (-75.1, 140.86, 379.9) |
| 20 | Lon Lon Ranch, the north gate | HyruleField_Child | (-65.5, 150.0, 183.5) to (-56.5, 150.0, 184.5) | 6 `Enemy_Leever_Fixed`: a giant (scale 2.5) at (-60.5, 150.02, 168.0) and 5 small ones round it, at y 150.2 on the floor (your decision kept in round 3, see 'Round 3 (21 Nov, third pass)') |
| 21 | Lake Hylia, the bridge by the laboratory | HyruleField_Child | (-166.92, 137.64, 421.84) to (-158.52, 137.32, 427.94) | 1 Like Like at (-153.5, 136.16, 423.0) and 4 `Enemy_Keese` at (-149.5, 138.2, 419.5), (-149.5, 138.2, 426.5), (-146.5, 139.0, 423.0), (-150.5, 140.0, 430.0) |
| 22 | the same | HyruleField_Adult | the same | 1 Like Like at (-153.5, 135.93, 423.0) and the same 4 Keese |
| 23 | Ganon's Castle, the north exit of the first round room | InsideGanonCastle | (-4.0, 110.3, 60.0) to (4.0, 110.3, 60.0) | `Enemy_MiddleBoss_DarkLink` at (0.0, 110.3, 76.0) |

New in the table: `more_kind`, the kind of the extra guards when it differs from the first (only the ranch uses it). Every guard is created by the trigger, as in gates 1 to 5, and each guard has its own barrier stack, so all must die.

Changes from the drawings:
- Crater: the drawn guard dot stood on the Deku Scrub; the Dodongo is 3.6 units further along the ledge. The drawn line was 8 units; the floor there is about 2 units wide, between the hammer boulder and the ledge edge.
- Field neck: the drawn line left a strip of floor open past its east end, beyond the rock cluster (the navmesh joins the lake road there), so the wall was extended to that strip (52 units, 14 pieces per guard).
- Graveyard: the Poes are 1 to 5 units west of the dots, so the trigger box fits between the wall and the Nocturne pad.
- Lake: Like Like only. The 4 Keese are left out: a Fire Keese did not appear when created in Hyrule Field (feature 10), and a guard that never appears keeps its barrier up. Question below.

### What each gate blocks and why it is safe
Child gates wait for the Kokiri Sword, as all child gates do; adult gates rely on the Master Sword.
- 14, crater ledge: beyond the wall are only the hammer boulder (an adult item) and the path past it. In the logic the child never reaches this part of the crater (`DMC Lower Nearby` to `DMC Lower Local`, and on to the ladder area, needs the Goron Tunic, an adult item), and the Deku Scrub stays on the guard side. Child only: for the adult this passage is a logic route (hover boots or hammer), and you marked the child map.
- 15 and 16, trail: beyond the wall are Dodongo's Cavern, the summit, the Great Fairy and the crater. The logic's enemy rules say the Kokiri Sword kills a Dodongo (gate 2 relies on the same); the adult has the Master Sword for four of them. The grotto beside the trail joins the trail north of the wall.
- 17 and 18, graveyard: beyond the wall is only the Shadow Temple door. The Nocturne pad and the gossip stone are on the guard side. Arriving by Nocturne puts you inside the trigger box, so the Poes and the barrier come at once (the way back to the graveyard stays open). Coming out of the temple puts you 0.45 units east of the wall line, outside the box. Both ages: the passage is the same. `Enemy_Poe` is placed by the game in the adult graveyard only; in the child graveyard it is away from home (a big Poe did not appear away from home, feature 10). The child gets nothing behind this wall in the logic (the temple needs adult items).
- 19, field neck: beyond it is Lake Hylia. `Hyrule Field` to `Lake Hylia` is free in the logic, so a child with the sword must beat three Lizalfos (the logic's rule text says the Kokiri Sword kills them, as at gates 3 to 5). Fire storm spot field_south_central (55, 163, 190) is 240 units away; the lake's warp pad, owl and arrival points are over 120 units away.
- 20, ranch: beyond it are Malon, Epona's Song and the ranch checks. Both guard kinds are already placed in the child field, so they are at home. Risk: in the original game the Peahat burrows at night; if it does here, the gate cannot be opened at night (leave and come back by day).
- 21 and 22, lake bridge: beyond it are the owl island, the warp pad island and the Water Temple door. The lake can be swum (both ages), so the gate is a fight, not a lock. The ground at the bridge end has the same height with the lake full or drained (137.4 to 137.7 on all three navmeshes).
- Entrance shuffle: with dungeon entrances shuffled all nine are left out, because a dungeon door lies beyond each wall line (Fire Temple, Dodongo's Cavern, Shadow Temple, Water Temple). For the ranch that is only the test's straight-line rule (the Water Temple lies far beyond the line, not in the ranch); left as the rule says.

### Dark Link: built and seen (21 Nov, gate 23)
The user's sketch (object map, Inside Ganondorf's Castle, 3D view turned a few degrees) was mapped by landmarks: the dungeon room centre anchors, the entrance anchors (`StartPos` at z 90 to 91.6, the entrance loading volume at z 101.9), the candle stands that outline the round room (radius about 12.6 at z 76 to 84) and the narrowing corridor (candles at x +-4.7 at z 62 and +-3 at z 51). The barrier line is at z 60, the start of the corridor north of the round room (8 wide, 3 pieces); the guard stands at the middle of the room (0.0, 110.3, 76.0); the trigger box (half width about 10) lies between the arrival point (z 90.7, outside it) and the wall. Floor height 110.3. Adult only (the map has no age suffix). No dungeon entrance lies beyond the wall, so the gate stays on under dungeon entrance shuffle.

What the files say (studied first):
- Dark Link is a plain placed actor in both maps that hold him (the Water Temple and his dream stage `DarkLinkStage`): no `Links`, only `DefaultDungeonRoomIndex 22` in the temple. His room group (`AiGroups`) holds the shutters, an event request actor, enemy counters and a sound actor; his death opens the shutters through the counters, nothing in his own files.
- His death table (`DarkLink_game__death__DeathReasonProcedures`) is a child of the ordinary enemy table with only the life-ran-out effect names changed; his `DungeonBossCommonParam` holds only chance time numbers, no event name. His start is perception (`BattleStartDistXZ` 4, 6 when attacked), not a room signal. So no clone was needed (no Volvagia style pack, no new death table): a created `Enemy_MiddleBoss_DarkLink`, exactly like the gate's Lizalfos, works. Not tried: a clone, other variants.
- His hp follows Link's max hearts (`LifeSyncCurve`: 64 at 20 hearts, 38 at 3 hearts); berserk at 60 % of it. Enemy health scaling may therefore not touch him (not checked).

In the game (new Longshot build with `walk` and `kill`; preset `enemy_gates`, 20 hearts; adult, Master Sword given):
- `teleport InsideGanonCastle` loads (no black screen, music BGM_InsideGanonCastle). Nothing stands in the room on arrival. A `walk` to (0, 78) enters the box: Dark Link stands at (0, 110.2, 76.0), hp 64, the purple barrier stands across the corridor mouth (screenshot), music BGM_MiddleBoss.
- He wakes as Link comes near, runs at Link, hits (80 to 68 quarter hearts), guards, goes berserk below 60 % and fights on. Real damage: Din's Fire took 64 to 40 in four casts; the Megaton Hammer (Z-target plus the item button) and the Master Sword took him down to 28 over about 25 minutes of scripted fighting. After the fourth cast the magic bar was empty every time (cause not found: `give Magic` filled it and it was empty again within a second), so in the game use the sword or hammer.
- `kill Enemy_MiddleBoss_DarkLink` at full hp: the list shows hp 0 for about 6 s (his death fade), then he is gone, the battle music ends, and a `walk` to (0, 45) passes the barrier line (Link ended at z 46.2). Seen, clip `<playtest folder>\darklink\clips\gate23_dark_link_kill.mp4`.
- Not seen: a death by fighting to the end (the script could not finish the fight; `kill` sets the life, it is not a hit), and what he drops.
- Leaving the room and coming back brings him back at full hp with the barrier (same as every gate).
- Do not `warp` Link south of z 94 in this room: the entrance loading zone is there (a warp to z 96 left the map once).
- Hands-on step for the playbook: kill him with the sword and watch the barrier drop.

### Questions
1. Lake: answered by the user (whatever works); the 4 Keese are in (see below).
2. Ganon's Castle entrance: answered, Dark Link works as a plain placed guard (see above).

### How it was tested
`cargo test -p ootrr-core difficulty` (210) and `settings` (109) with `OOTRR_ROMFS`. The game-file test stages every gate from the game files: each map takes its groups, barriers and areas, the old actors are untouched, every graph audits clean, child graphs read the sword flag. New test: each new gate makes the marked kinds and counts, and the child and adult copies share the wall. Every guard kind's actor pack exists. Not played.

### In the game
For each gate: warp to the approach point in `playtest/gate14/PLAN.md` (just outside the trigger, facing the wall), with the Kokiri Sword for the child ones. Check: the map loads; nothing appears until you walk in; then the guards and the barrier; the barrier spans the passage with no gap at the ends; it drops when all guards are dead. Then: whether a child can climb out of the crater ledge pocket, the ranch at night, the Like Like in the field, the Poes in the child graveyard.

## 21 Nov follow-ups (branch `feature/gate-crater`)
User changes: Lake lab bridge guards "whatever works" (Like Like plus 4 Keese); Lon Lon Ranch "a giant leever and 5 smaller leevers"; Lost Woods guard "a wolfos". New guard kinds in `gates.rs`: `LEEVER_BIG_GUARD`, `LEEVER_GUARD`, `KEESE_GUARD`, `DARKLINK_GUARD`; gate 9 uses `WOODS_GUARD` (`Enemy_Wolfos`). Tests updated (piece counts, kept-gate list under dungeon shuffle, one new test for the new kinds); `cargo test -p ootrr-core difficulty` and `settings` pass.

Keese: the game places ordinary Keese only in dungeons and the Graveyard (with `IsCreateOnSurface` and a hover offset). A Fire Keese created in the field never appeared (earlier test), but the ordinary `Enemy_Keese` created with no parameters does: no ceiling needed.

### Gates 21 and 22 (seen: child gate 21; the adult copy uses the same data, not played)
`walk` to (-152, 423): the Like Like (hp 12) at (-153.5, 136.2, 423.0), 4 Keese (hp 1) flying round it, battle music, barrier across the lab bridge. `kill` on every guard: the Keese vanish, the Like Like drops a blue rupee, the music stops, and Link walked west through the old wall line to x -169. Clip `gate21_keese_kill.mp4`.

### Gate 20 (seen in part)
Child, `walk` to (-60.5, 166): the Giant Leever (`Enemy_Leever_Big`, hp 8) and the small Leevers (hp 2) are there; the Big one's summons stack five more at its own spot, so 11 Leevers are listed (only the 6 placed ones are guards). They surface, circle and hit Link (80 to 76 quarter hearts), by day and also at night (the world clock ran from day to night during one run; the new build's `time` does not work), and a sword hit takes 1 hp off. The barrier stands (pink fence across the ranch gate).
- Not seen: all of them dying and the barrier dropping. The `kill` command (life set to 0) does NOT make a Leever die: it stays listed with hp 0 and the barrier stays up, because a burrowed Leever dies only from a real hit. One Leever set to 1 hp died to one real sword hit (about 90 s of scripted chasing); the script cannot catch the rest, as they burrow. Added to `capture-playbook.md`: a person kills the 6 placed Leevers with the sword, by day and by night, and watches the barrier.
- Risk: a Leever that stays burrowed and never surfaces would keep the barrier up. In every run they surfaced within seconds of Link standing at the spot.

### Gate 9 (seen)
`LostWood_Child`, `warp -2 100.4 -0.5`: one Wolfos (hp 9) appears at the middle of the room with the barrier pieces and battle music. `kill` on it removes it (once the first reading showed hp 9 again, the second `kill` finished it); then a `walk` through the east door from (0.5, 0) reached x 8.2, past the barrier line. Clip `gate9_wolfos_kill.mp4`. The gate has no adult copy (the map is the child Lost Woods only).


### Round 2 (21 Nov, commit after aa97399): gate 16 guard, guard floor test, gate 20 guards, gate 14 climb-out

**Gate 16.** The Dodongo at (-16.5, 121.79, -20.5) was 1.8 units under the navmesh floor (123.59 there), so it fell through on creation. Moved to (-16.5, 123.6, -20.5). Played (adult, `walk` to the mark, `enemies`): all four Dodongos stay at y 122.9 to 123.9 (x -19 to -12, z -18 to -27.6) after 5 s and after a map reload. Clip `gate16_barrier_drop.mp4`. With live guards the walk stops at z -28.65 (the barrier); after `kill` on all four the walk passes to z -40.

**Guard floor test** (`every_gate_guard_stands_on_the_navmesh_floor_with_clearance`, needs `OOTRR_ROMFS`): every guard of every gate (the mark plus `more`) must have a navmesh face within 0.6 units above or below it and at least 1.0 unit from a wall; flying kinds (Keese, Poe) need a floor within 10 below and are not checked for walls. It found two bad places besides gate 16: the two small guards at (-64.0, 151.08, 166.0) and (-62.5, 151.87, 162.0) of gate 20, now at y 150.2. The Keese (2 to 10 above the floor) and Poes (0.7 above) are fine.

**Gate 20, round 2.** Real hits with the Kokiri Sword (child, sword shown gold in the pause menu's Equipment tab, so it was equipped; the earlier misses were not about the sword): Stalchildren that came at night died to Y hits and dropped rupees, but the Leevers never moved, never surfaced and never lost hp: a hit does nothing to them, `target` + Y for 6 swings at the Big one's spot left hp 8. A plain `spawn Enemy_Leever` and `spawn Enemy_Leever_Big` 5 units from Link in the field stayed buried the same way (by day, 40 s), so it is not the Big one's summons and not our placement: the game places Leevers only on the Haunted Wasteland's sand (`Enemy_Leever_Fixed`, and the horde table in `horde.rs` treats them as a sand kind), and its pack is not loaded in the field. The `Enemy_Leever` listed in the old runs were dormant, and the 80 to 76 hits seen then were most likely the night Stalchildren. So a Leever gate on the ranch grass cannot be won. Changed to what the first plan had and the field already loads: `Enemy_Peahat` (hp 6) at the mark and 5 `Enemy_Peahat_Larva` (hp 1). Played by day: alive, the walk to (-61, 192) stops at z 183.59 (barrier); the larvae died to Y hits within seconds, the Peahat took about 60 s of `target` + Y (6 to 0), the three larvae it carries vanished with it, and then the walk reached z 191.2: barrier dropped. Clip `gate20_peahat_barrier_drop.mp4`. Not tested: at night (the ranch's own gate is closed then, the Stalchildren come, the Peahat may not stay); `time` still does not work. One wasted detail: the Peahat gate depends on the Peahat's own larvae count being irrelevant (only the six placed guards count).

**Gate 14 climb-out (child).** In the ledge pocket (x -26 to -16, z 32 to 41, floor y about 119.5) the way east and up is a 5 unit step (120 to 125 at x -16, z 38 to 41): `walk` stopped there (blocked) while the Dodongo hit him (71 to 67 quarter hearts). Stepping off the east edge drops 13 units to y 106 and the game puts him back on the ledge (earlier run). So a child who is in the pocket cannot climb out east and can only leave by the west neck past the rock pile (a hammer boulder, which the gate's barrier now also covers). That is how the pocket is in the game; the gate adds no new trap, but while the Dodongo lives he cannot leave west either; killing it is the only way on. Where a child can enter the pocket from (the drop at x -16 from the crater floor is one way) was not checked.

**Found, not changed: a bypass at gates 15 and 16.** East of the barrier's east end (x -7.4) the navmesh at z -29.2 has a lower strip (y 119.8 to 120.3) from x -4.5 to 3; the adult walked from (-4, -24) to (-4, -35) and on to the trail south of the wall while one Dodongo still lived (live guards only blocked him when they stood in his way). The strip is open in the child map too. The fix would be one more wall (`also`) from about x -6 to 3.5 at y 119.8; not done because the barrier layout was not asked for.

Tests: `cargo test -p ootrr-core difficulty` 212 passed, `settings` 109 passed (with `OOTRR_ROMFS`).


## Round 3 (21 Nov, third pass, branch `feature/gate-crater`)

### Gate 20 is Leevers again (your decision kept)
You said "make it a giant leever and 5 smaller leevers"; the last pass had fallen back to Peahats. What keeps Leevers buried and what was done:
- **Five sand tests.** The Leever graphs (the four `Leever.action.*` modules in the shared AI pack, and the Leever and Leever_Fixed LandBattle action sequence) each hold one `QueryPhysicsIsMaterialContactToSphere` (material 3 = sand, except-mode): "touching anything but sand" sends the Leever into its dive-away verb. The build cuts every reader of that query (`leevers.rs`, `unbury`), so a Leever acts as if it always stood on sand.
- **The manager kinds never start.** `Enemy_Leever` and `Enemy_Leever_Big` are made by the game's auto placement manager, which hands out pop-up times; placed or spawned in the field they sit still (the Big and its 5 summons stayed at their spot, hp 8 and 2, in every run). `Enemy_Leever_Fixed`, the Wasteland's placed kind (its `PreparePopupTimeRand` is 0), starts by itself and surfaces on grass.
- **A chase ends in a dive-away that deletes the Leever.** After about 4 s the Fixed ones dived and vanished. `stay` removes the last step of the LandBattle sequence (the disappear verb 96), so they keep coming until killed.
- **The giant.** The real Big could not be made to act: tried the pop-up wait 0 in its pack, the fixed placement bool true in its root and blackboard, and the same sand edits. Not cloned: a new actor pack and rows would still lack the manager's start. Stand-in: one Fixed Leever placed at scale 2.5 (`Gate::scale`); it has the small ones' hp 2. With the Big gone nothing summons: exactly the 6 placed Leevers count.
- **Seen (child, Kokiri Sword, real `target` + Y hits):** day: 6 of 6 died in 89 s and the walk through the gate line passed (blocked at z 183.6 before); night (`time night` plus a map reload): 6 of 6 in 35 s, walk passed; Stalchildren joined at night. Clips in `<playtest folder>\gate14\clips` (`g20G_fight`, `g20N_fight*`). The script topped up health when low. Caveat for the player: Leevers Link led far away wait where they stand; walking out of the box and back rebuilds all six.
- **Side effect:** with the gates on, the Haunted Wasteland Leevers no longer sink at the edge of the sand nor dive away (the settings note says so). With the gates off nothing changes.

### Gates 15 and 16: the lower terrace
A terrace (y 120, x -5 to 3.5) east of the trail is reached by walking off the trail's east edge at z -21 (a 3 to 4 unit gap, 3.4 down) and leads north past the wall. Seen (child, Dodongo alive): walked round to z -36. A second row of barrier pieces across the terrace (`STRIP`, the gate's `also` wall) closes it: the same walk now ends blocked at z -28.8 (child) and -28.7 (adult, 4 Dodongos). The west side is rock (blocked at x -19.7). Test `the_barrier_cuts_the_floor_so_a_walker_on_the_guard_side_cannot_reach_the_far_side` floods the navmesh in 0.5 unit cells (step 0.8, drop 3.6, jump 4.1) with the barrier cutting cells; it fails without the added wall and passes with it. It cannot see rocks the navmesh lacks, so it carries the observed west rock as a constant.

### Gate 14: the pocket
The pocket is the guard's side (Dodongo and Scrub inside; the barrier is across its west neck). It is a dead end with or without the gate: west is a solid rock face (a `walk` ends blocked at x -26.4 to -27.1 even with the Dodongo gone), east is a 5 unit step; the only way in is the 4.7 drop from the crater ledge above. A child does not reach that ledge by walking from the crater's arrival (navmesh flood: 9210 cells, none on the ledge; the real walk ended blocked or fell). So the gate creates no trap and nothing was moved.

### Tests
`cargo test -p ootrr-core difficulty` (218) and `settings` (109) with `OOTRR_ROMFS`. New: `leevers.rs` (no reader of the sand test is left and nothing else in the graphs changes, only the Leever graphs change in their packs, the pop-up wait is 0 and nothing else in the file changes, same output every time, a graph without the sand test is refused), the giant scale and the terrace wall piece counts in `gates.rs`, the navmesh flood test.


## Round 3b (21 Nov): clone packs, giant with more life, gate 16 kill

Change: the Leever edits no longer touch the game's Leever packs or the shared AI pack. Two new actor kinds are written into the mod instead, copied from `Enemy_Leever_Fixed` (`leevers.rs`, the way the cow and Volvagia clones are made: new pack, actor rows, tag row, size table entries):
- `Enemy_Leever_Gate` (the 5 small guards): the clone's pack holds the Fixed pack's files, its LandBattle graph with the sand test cut and the dive-away step removed, and the four edited `Leever.action.*` modules under their own names inside the pack (the actor reads its own pack first).
- `Enemy_Leever_Gate_Giant` (the giant, placed at scale 2.5): the same plus its own life file (MaxLife 16 = 8 x the small ones' 2) and attack file (BaseAttackPower 3 instead of 2), reached through its own parameter table.
- Test `the_vanilla_leever_packs_and_the_shared_ai_pack_are_never_written`: with the gates on, no output is the Leever, Leever_Big, Leever_Fixed or shared AI pack (so they stay byte-identical to the game's). Others check the clone graphs, rows, tags, life and attack numbers, determinism. 220 difficulty and 109 settings tests pass.

Seen (child, Kokiri Sword, day, real `target` + Y hits): the giant showed hp 16/16, the small ones 2/2; all 6 dead in 119 s (the giant took about 15 hits; the script topped up health twice); walk through the gate line arrived at z 185 (blocked at 183.6 before); ranch music. Winnable for a child. Clip `clips\g20H_fight.mp4`. Night was not repeated with the clone.

Gate 16 (adult): killed all 4 Dodongos (two needed a second `kill`), waited: the terrace walk went on to z -44 and the main trail to z -39.3; with two alive both were blocked.

Wasteland: NOT TESTED in the game. Link could not get onto the sand within a 50 s walk and no Leever appeared. The clone approach leaves the game's packs unwritten (the test above), and the clone pack is only loaded where a gate places it.

## Changes after the 21 Nov hands-on (branch fix/playtest-gates)
- Gate 20 (Lon Lon north gate): three Stalfos replace the Leever guards. Gate 9 (Lost Woods): a Stalfos again.
- Gates 21/22 (lake lab bridge): only the Like Like guards. The four Keese are plain extra enemies near it. The wall now runs across the landward end of the bridge strip only (z 428), not across the open area. Possible cause of the fall through the floor: the old wall started at the lab entrance sign and anchor, where Link arrives, and the guards stood on the ground beside it; not confirmed.
- Gate 14 (Crater): adult only, with a wall across the full width of the ledge, checked against the navmesh by the gate walk test.
- Gates 15/16 (Trail): left in place, but the east strip of wall that ran into the Goron City loading zone is gone and the wall ends 8 units short of it. Their walk test is no longer run, as the wall no longer closes the trail; a new position is coming.

### Hands-on 9 results (21 Nov)
- Gate 23 (Dark Link): seen by hand, passed.
- Gates 21/22 (lake lab): the user fell through the floor next to the Like Like and Keese; the new wall (only across the bridge, z 428) is a guess at the cause, not confirmed. Gate 15 (child Trail): after the guard died Link was warped (a loading zone right behind the wall); the user wants gates 15 and 16 moved higher up the Death Mountain Trail, above Goron City; a screenshot from the user is awaited, so they have not moved.
- Gate 20 and gate 9 changes are in the list above (3 Stalfos; Stalfos). Gate 14 is adult only. Not played again after the changes. Gate editor: near useless as built (fairy markers); a live gate with a real barrier tied to a dummy guard is wanted (`gate-editor.md`).
