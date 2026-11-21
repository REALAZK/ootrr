# Special encounters: Dead Hand in two holes, Iron Knuckle in Hyrule Field, Like Likes at Lake Hylia

Setting `special_encounters` (number 130), checkbox, label "More enemies: special encounters". Code: `crates/core/src/difficulty/encounters.rs`. Built on `feature/dead-hand-holes` (worktree `wt\dead-hand-holes`). Separate from the `more_enemies` choice (119), which is unchanged.

## Request
- "add a dead-hand into the hole which is really dark and spooky", then after the picture catalogue of all holes: "9 and 23 should get a dead hand".
- "can we put a darknut in hyrule field? just in the otherwise pretty open area between kakariko, lonlon, and deku".
- Then: one setting for all of these hand placed encounters.

## Interpretation
- Hole 9 is the "Dark ReDead Hole" in east Hyrule Field (the ReDead stays). Hole 23 is the "Dodongo Fog Hole" at the foot of the Death Mountain trail (map `DeathMountainTrail`, Baby Dodongos and bomb flowers inside). One Dead Hand in each, on the floor a few units from where Link lands.
- This game has no Darknut. The equivalent is the Iron Knuckle. One Iron Knuckle in the open grass of Hyrule Field, roughly in the middle of the triangle Kakariko entrance, Lon Lon Ranch gate, Kokiri Forest entrance.
- Each placement is an entry in `PLACES` (a const list) so one can be dropped if its map will not load.

## Investigation
### What a vanilla Dead Hand needs
Two vanilla instances exist: Bottom of the Well (`BottomOfTheWell_Static`) and Shadow Temple (`ShadowTemple_Static`).
- Actor `Enemy_DeadHand` (the body) plus `Enemy_DeadHand_Hand` (the arms that grab). The body has a `Links` list with one `Reference` link to each hand (Well: 4 hands, Shadow Temple: 6).
- Body `Dynamic` in the Well: `DeadHand_HoldActorCancelSleepFrame` 1800, `DeadHand_IsManageHandNumByLife` true, `Drop__DropTable` "None". Shadow Temple has `DeadHand_RangeOfAppearArea` 4.0 instead of the manage flag. The actor's own defaults (read from its pack's blackboard table): appear range 6.0, cancel frames 900. The body also carries `Name: "DeadHand"`.
- Hands: the Well ring puts the hands at 5 units along each axis around the middle, each rotated to face the middle. Hand `Dynamic`: one has `DeadHandHand_IsEnableAppearDelay` false, one has `DeadHandHand_AppearOrderNum` 2, two have 3. All of them also carry `DefaultDungeonRoomIndex`, which is a dungeon field and is left out here.
- Both vanilla rooms put the body and hands in a `FarDeleteGroups` and a `SimultaneousGroups` entry (hash lists the engine loads and unloads together), and an `AiGroups` entry with the room logic: the room module, an area, an enemy counter linked to the body, a chest that appears when the counter reaches zero, a push wall and an `EventRequestActor`.
- The body's AI (`Pack/Actor/Enemy_DeadHand.pack.zs`) has its own states (hide, appear, land battle, escape). The room logic looks like it only provides the reward (the chest and the wall), not what makes him rise.

### What I reproduced and what is missing
- Reproduced: body with its Dynamic keys and Name, four hands (Well layout and Dynamic keys), the four `Reference` links, and one group entry in `FarDeleteGroups` and `SimultaneousGroups` holding the five hashes in ascending order, like vanilla.
- Not reproduced: the room logic (`AiGroups`), enemy counter, reward chest and push wall. A hole has no such logic, so the fight gives no reward and nothing closes behind you. That is intentional.
- Changed from vanilla: the hand ring radius is 3.0, not 5.0. Both holes are narrow: the hole 9 cave is a corridor about 7 wide (its inner area box is 7.3 by 15.4), so a radius 5 ring would put hands in the walls. The ring is turned to follow the cave's long axis. This is a guess: if hands stand in rock, change `RING`.
- Not verified: whether the body wakes without the room's enemy counter and area. The Shadow Temple body uses its own appear range, which is why it is expected to work, but nobody has seen it outside a room.

### Positions
All from the real map files; there is no collision geometry reader in the repo.
- Hole 9 (`HyruleField_Child` and `_Adult`, identical): hole base `C_01` at (163.27, 150.74, 27.38); landing anchor (163.27, 136.12, 27.38); restart anchor (the survey's inside point) (161.81, 135.84, 25.93). Floor height 135.84 (the ReDeads stand on it). The cave is a corridor along a 45 degree line from the landing end to the chest (152.2, 136.0, 16.7); the ReDeads are at (155.0, 15.7) and (148.7, 17.7). Dead Hand at (156.9, 135.84, 21.2): 6.8 from the restart anchor, 5.9 from the nearest ReDead, 6.5 from the chest.
- Hole 23 (`DeathMountainTrail_Child` and `_Adult`, identical): base `I_03` at (-1.65, 71.17, -26.12); restart anchor (-2.19, 58.69, -28.15). Floor 58.17 (Baby Dodongos and rupees stand on it). The cave inner box is centred at (-3.5, -33.0), 11 by 16, turned 15 degrees. Dead Hand at (-5.2, 58.17, -34.2): 6.7 from the restart anchor, 5.4 from the nearest Baby Dodongo, a few units from the bomb flowers and rupees.
- Iron Knuckle: the corners are `HyruleField_FrontOfKakarikoVillage` (210.8, 176.9, 10.0), the Lon Lon gate `Land_LonLon_Gate_B_01` (-61.8, 150.6, 184.2) and `HyruleField_FrontOfLostWoodBridge` (200.1, 175.7, 309.1); the middle is about (117, 170). Ground standing actors near there are three rocks 4 to 6 units from the chosen spot (`Land_HrlField_RockGround_A_01` at y 157.63 and 157.01, `Land_HrlField_EponaObstacles_Rock_A_01` at y 158.04). The spot is (134.0, 157.7, 168.5), 17 east of the exact middle so rocks are beside it. It is 46 from the nearest hole, 36 from the river bank, and far from every entrance trigger and the ranch walls. The ground height comes from the rocks and may be a few tenths off. Both ages have the same rocks at the same heights.

### Which Iron Knuckle
Vanilla: `Enemy_IronKnuckle_Silver` (two in the Spirit Temple), `Enemy_IronKnuckle_White` and `_Black` (Ganon's Castle, linked to each other), `Enemy_MiddleBoss_IronKnuckle_Nabooru` (room logic and a cutscene).
- Chosen: `Enemy_IronKnuckle_Silver`. Its Spirit Temple instances carry `BreakTarget` links to the chair and pillars that crumble when it stands, plus `Drop__DropTable` "None". Its actor has one blackboard boolean (`HasArmor`, true) and its AI reads perception, so it does not depend on room logic to fight.
- Reproduced: the actor with `Drop__DropTable` "None", no links, no group. The chair and pillars are left out: they would be a second new kind in the map and are only scenery.
- Sit or stand: its idle action is `Sit`, so it is expected to sit (with no chair under it) until it is hit or notices Link, then stand and fight. Inferred from its AI graph, not seen. If it looks wrong without a chair, the chair (`Obj_IronKnuckle_Chair_A_02`, linked as `BreakTarget`) can be added.

## Implementation
Every addition goes in the `_Static` file of its map (4 files: `HyruleField_Child`, `HyruleField_Adult`, `DeathMountainTrail_Child`, `DeathMountainTrail_Adult`). Hyrule Field's two files carry the hole 9 Dead Hand and the Iron Knuckle in one rewrite. No `_Dynamic` file is touched.

Why `_Static`, not `_Dynamic`, which was the preferred option: no actor in a Hyrule Field `_Dynamic` file has a `Links` list, and that file has no `AiGroups` or `FarDeleteGroups` keys, so a Dead Hand cannot hold its hands there. Also the ReDead in hole 9 (`Enemy_ReDead_AlwaysRespawn`), the Baby Dodongos in hole 23 and the field's Octoroks, Tektites and Peahat Larvae are all in `_Static`, so putting the new enemies there mirrors them. The Iron Knuckle mirrors the Octorok and Tektite entries: a plain actor, no links, in no group.

Known risk: Dead Hand, Dead Hand Hand and Iron Knuckle Silver are all kinds that these four maps do not contain, appended to a `_Static` file. That is the same shape as the Dodongo's Cavern edit that went to a black screen, and it cannot be avoided for the Dead Hand. That black screen was not isolated, so this is a test, not a safe bet.

The new actors use the repo's usual shape (`Gyaml`, `Hash`, `Phive.Placement.ID`, `SRTHash`, `Rotate`, `Translate`) with fresh hashes that are in neither file of the map (`ambush::fresh`). The step reads the files through the same chain as the other features, so it combines with More enemies, Enemies block the way and the rest.

## How it was tested
- Unit tests in `encounters.rs`: a Dead Hand gets four hands, matching links and one group per list, unique hashes, byte round trip; hand positions and rotations match the vanilla ring; an Iron Knuckle adds one plain actor and touches no group; two places in one map share no hash; a map without the group lists is refused.
- Game file test (`OOTRR_ROMFS`): each placement alone and all three together on the real maps. Each file gains exactly the expected actors, the old actors are untouched, `AiGroups` is unchanged, all hashes are unique, the Dead Hand sits more than 6.5 from the restart anchor and level with it, and nothing new is within 4 of an existing enemy or chest.
- `difficulty` tests (113) and `settings` tests (107) pass. The everything-on game test passes with the new files, the build notes count is 21, and the settings string with the setting off is unchanged.
- A real build in `<playtest folder>\encounters`: a mod generated and built with this setting on. The four map files hold the new actors (6 more in each Hyrule Field map: Dead Hand, 4 hands, knight; 5 more in each Death Mountain Trail map).
- The game was not launched.

## In the game
Never played. Turn this on alone (preset and steps in `<playtest folder>\encounters\TESTS.md`).
- Hole 9: `teleport HyruleField_Child`, then `warp 161.81 136.84 25.93`.
- Hole 23: `teleport DeathMountainTrail_Child`, then `warp -2.19 59.69 -28.15`.
- Iron Knuckle: `teleport HyruleField_Child`, then `warp 128 159.5 163`.
- Look for: the map loads; Dead Hand's hands appear on the floor and he rises when Link is grabbed or comes close; he can be hurt and killed; hands are in the floor and not in a wall; the ReDeads and Baby Dodongos are still there; the room's exit still works. The knight sits beside the grey rocks, wakes when hit or approached, can be killed and is not in the ground.
- Repeat as adult (`age adult`, `teleport HyruleField_Adult` and so on).

## Knight wake on approach (v2)
Seen in the game (18 Nov): the knight spawned and sat still. You asked for it to wake when Link comes near.

### What holds the knight seated in vanilla
All Iron Knuckle variants carry the same graphs. `AI/Enemy_IronKnuckle.action.Sit.module.ainb` plays the `Sit` animation and runs a `SelectorStackNode` of four wake conditions, each ending the sit with Success: (1) a sequence that needs a perceived target, a 3 frame count and the nearest `BreakTarget` memory (the chair) to be gone; (2) a `Reference` link life check; (3) damage of type 9; (4) `QueryPhysicsContactPlayer` (bumping into it). Every vanilla instance (Spirit Temple, Ganon's Castle, the stage maps) is linked to a chair by `BreakTarget`; ours has no chair. No placed-instance key, no variant and no blackboard value sets "awake": the Silver, White and Black packs have identical graphs, and the placed instances carry only `Drop__DropTable` and `DefaultDungeonRoomIndex`. So route 1 (a parameter) does not exist. Read from the graph, not seen.

### What changed (route 2)
Only `Pack/Actor/Enemy_IronKnuckle_Silver.pack.zs` changes, two files in it (`crates/core/src/difficulty/wake.rs`):
- The knight's blackboard table gets a bool `OotrrKnightWake`, default false, `IsInstanceParam` true. This is how a placed actor's `Dynamic` key reaches a blackboard bool (the Dead Hand's `DeadHand_IsManageHandNumByLife` works the same way).
- The Sit graph gets a fifth child, placed first in the stack: a switch on `QueryBlackboardGetBoolNoError(OotrrKnightWake)`; when true, a switch on `QueryUserPerceptionIsInsideSphereByInput` with radius 9 centred on the `Spine_2` bone (the bone query, `QueryMathMtx34fToVec3fMat33f` and sphere query are copies of the sphere check already in the same graph, which uses radius 0.8); inside gives `ExecuteGenericSetResultSuccess`, otherwise `ExecuteGenericSetResultNone`. Same shape as the contact child (switch to Success or None). Built with the same builder and wiring helpers as feature 17 (`backup.rs`).
- Our placed knight carries `Dynamic` `OotrrKnightWake` true.
Scope: the Spirit Temple Silver knights share the pack and so run the new branch, but their key is false, so it returns None and they behave as before. The White, Black and Nabooru packs are not touched. Nothing else in the game changes.

### Evidence and what is unverified
- Test with the game files: the Sit graph has exactly one new child, first in the stack, the old four unchanged after it; the flag query reads the new key, the sphere radius is 9 (the old sphere stays 0.8); only the Sit graph and the knight blackboard differ in the pack; layout audit clean; the written graph reads back byte for byte.
- Not seen in the game. Open: the stack node may not run a child past a busy earlier child (ours is first to avoid this); the sphere query's "user perception" may only report targets that the knight's senses report; the empty territory check module above the stack might hold a field knight; the 9 unit radius assumes the same units as the map positions.

### How to test
`<playtest folder>\encounters2\TESTS.md`: knight still seated at 12 units, stands and attacks when Link walks to 5 units; Spirit Temple knights still sit until hit.

## Knight awake from the start (feature/knight-roam, 18 Nov, not played)
You asked: "ideally he'd already be awake and just roaming".

### Awake
`wake.rs` has a const `MODE` (`Wake::Awake` or `Wake::SeatedUntilNear`). In `Awake` the new first child of the Sit stack is the flag switch only: our knight (`OotrrKnightWake` true) ends the Sit with Success as soon as he is active, no sphere. Spirit Temple knights (flag false) are unchanged. `SeatedUntilNear` is the old v2 graph.

### What he does after release (read from the graphs, not seen)
Brain (`Enemy_IronKnuckle.brain.root`) handles exactly five verbs: initial wait (Sit), a discovered/stand up verb, land battle, a hidden/event verb and an inaccessible verb. Sit finishing with Success sends the brain to the stand up verb (StandUp plays `StandUp`, then `Wait`, then asks the brain to update); that verb searches for a target every frame and goes to land battle when one is found. With no target he stays in the stand up/wait state: standing still. Unknown: whether the stand up container replays the stand up animation in a loop while nothing is perceived. How far his senses reach is also unknown (the 9 unit gate hid it).

### Roaming: not built
- The brain's idle selector `EnemyCommon.SelectIdleBrainVerb.module` has a patrol flag (`IsEnablePatrol`, false in the knight's brain) and a return-to-home path using a map rail (`QueryActorGetNearestPointOnRail`). But the patrol module `EnemyCommon.Patrol.module` is an empty graph, and the selector sets brain verbs 18/19/62 that the knight's brain updater has no case for (it falls to an "unsupported brain verb" failure node). The knight also has no territory module in the brain and no patrol or wander action container in its scheduler (only Wait, Discovered, LandBattle, EventAIAndHidden).
- Other enemies (Stalfos, Lizalfos, Wolfos) have only return-to-home modules, no wander. Wander graphs exist for animals and NPCs (`WanderDefault`, Guay, Kokko, Horse), not for sword enemies.
- A faithful roam would need: a new brain verb handler in the knight's brain, a new scheduler entry and `ActionSeqContainer` data file with a new action verb in the pack, an action sequence graph that calls his Walk action with a target position (the Walk action takes its target from the action verb arguments, not a free position), and a rail actor linked to him in the map. That is new BSA data with no vanilla precedent for the knight, so it was not attempted.

How to test: `<playtest folder>\knight-roam\TESTS.md`.

## Notes
- If a map shows a black screen, drop the place from `PLACES` and rebuild. The three are independent. Both Hyrule Field maps carry two places, so test them separately by editing the list.
- Open questions: does the Dead Hand need the room's enemy counter to wake? Should the knight have a chair? Is a radius 3 hand ring right for the narrow caves?

## Seen in the game (first playtest, shadow game, windowed)
Preset: `special_encounters`, 20 hearts, enemy_health 50, Din's Fire with `["magic_meter","dins_fire"]` (the other order is refused by the generator). Din's Fire is on the X slot; the bridge button is `ZR`. Magic is empty after about 4 casts and `give` cannot refill it (status shows no magic). Shots: `<playtest folder>\encounters\shots\`.

| Place | Age | LOAD | PRESENT | REACTS | KILL | EXIT |
|---|---|---|---|---|---|---|
| A Iron Knuckle, Hyrule Field | child | PASS (map loads, bright picture) | PASS (armour seen at the spot, `A-child-wait5.png`) | Stays seated and still for 15 s with Link 2 units away. Din's Fire wakes it (music BGM_MiddleBoss); it hits for about 3 hearts a swing and killed Link (20 hearts) in 6 casts | Not killed in 6 casts | - |
| A | adult | PASS | picture taken | Same: still for 15 s, woke on the first cast, attacked; Link died | Not completed (2 casts) | - |
| B Dead Hand, hole 9 | child | PASS for the map; the warp into the room failed, Link ended in Kokiri Forest | NOT TESTED | NOT TESTED | NOT TESTED | NOT TESTED |
| B | adult | Warp into the room landed at (161.9, 135.7, 26.5) with an event for about 5 s, then Link was in Kokiri Forest (cause unknown: Dead Hand grab, or the room reload) | NOT TESTED | NOT TESTED | NOT TESTED | NOT TESTED |
| C Dead Hand, hole 23 | child | PASS | PASS (`C-child-wait5.png`) | Rose and attacked within 15 s (BGM_MiddleBoss, life 72 to 56) without any room logic | PASS: smoke, music stopped, after 3 to 4 Din's Fire casts (`C-child-cast4b.png`) | NOT TESTED |
| C | adult | PASS | PASS | Rose and attacked | Not killed: magic ran out after 5 casts and Link died | NOT TESTED |

Notes: no black screen in Hyrule Field or Death Mountain Trail, either age. The knight does nothing on proximity alone (a code change for that is being prepared). Hole 9 needs a retest with a better method (the earlier survey reached it by warp without the mod).

## Seen in the game (v2 build, wake on approach, 18 Nov)
Same shadow game, windowed, 20 hearts, `enemy_health` default. Shots `<playtest folder>\encounters\shots2-*.png`, log `<playtest folder>\encounters
esult.txt`, hole 9 samples `<playtest folder>\encounters2\hole9-log-a1.json`. Approach points are on the south-west side of the knight (the east side is off the map: Link fell and respawned in Kokiri Forest).

| Check | Age | Result |
|---|---|---|
| Knight stays seated at 14 units, 10 s | child, adult | PASS: no life change, field music only, no movement (`v2-c-14u-9s.png`) |
| Knight wakes at 5 units, no hit, Link still | child | PASS: battle music (BGM_MiddleBoss) already on within 1.6 s of the warp (the first poll), first hit at 6.1 s (life 80 to 68, Link knocked back 2.5), further hits at about 12 s and 18 s (`v2-c-5u-6s.png`: axe swing, Link knocked down) |
| same | adult | PASS: same signs, music at once, hit at 6.0 s (20 to 8 quarter hearts), Link died later and was recovered with Try Again |
| Spirit Temple silver knight, Link 5 units away for 15 s | adult | PASS (control): still seated, life and music unchanged (`v2-spirit-5u-8s.png`). Warp point from TESTS.md landed safely |
| Hole 9 Dead Hand | child | FAIL (case c): warp to (161.81, 136.84, 25.93) put Link in the room at once (y settled at 135.74, room music, dark picture, nothing seen). An event ran for 4.2 s, then the map changed to KokiriForest_Child (-126.5, 114.7, -38.4). Life unchanged (32), never dead, y never dropped. So a map change event fires at (161.81, 135.74, 25.93) about 4 s after entering. Second attempt not made. PRESENT and REACTS not seen |
| Hole 23 exit | child | PASS: warp to the exit trigger area (-1.71, 68.5, -25.99) never arrived (Link was at the entrance, -1.65, 120.17, -26.12, already); after warp to the `HidingHole_FieldExitGo` anchor (-1.65, 59.05, -26.12) Link stood at (-1.19, 120.17, -26.52): carried back up. The first warp into the room had already been followed by the carry-up, so which trigger did it is not isolated. Dead Hand music was on in the room |

Open: hole 9 event. Compare with the unmodded game (survey: stayed in the room, music BGM_Common_Dungeon) so the mod's change to hole 9 (Dead Hand placement or its room logic) triggers a map change; fix in code, then retest.



## Session fixes (18 Nov, feature/session-fixes, not played)
User, after entering hole 9 properly: the Dead Hand "could be moved back a bit tho, right now it's attacking the moment you land already".
- Hole 9 (Hyrule Field): Link lands at the exit anchor (163.27, 136.12, 27.38). The cave runs 15 units from there to the chest at (152.2, 16.7), with the ReDeads near its far end, so the very end is taken. Dead Hand now at (155.72, 135.84, 20.09): 10.5 from the landing spot, 4.5 from the nearest ReDead, 4.9 from the chest. Nearest hand 7.5 from the landing spot (was about 5.9).
- Hole 23 (Death Mountain Trail): landing at (-1.65, 59.05, -26.12). Dead Hand now at (-5.0, 58.17, -38.5), the far end of the room among the Baby Dodongos: 12.8 from the landing spot (was 8.8), 4.2 or more from every Dodongo. Nearest hand about 9.8 from the landing spot.
- The ring and floor heights are unchanged. Both child and adult maps use the same numbers. The game test now also requires every hand to be at least 6 units from the landing anchor.
- In the game: see `<playtest folder>\session-fixes\TESTS.md`.


## Like Likes at Lake Hylia (feature/likelikes, 18 Nov, not played)
You sent a picture of the Lake Hylia map with green dots and asked: "add a likelike at all the locations marked green".

### Interpretation
- Ten dots (north-west grass corner, two north-shore spots, the bridge island, a west island, the central island, the stone-ring island, a south-west islet, the west shore strip, a south island). One `Enemy_LikeLike` on each, in both the child and the adult map (you did not say which age). Each is its own row in `PLACES` (`like_like_lake_hylia_*`), so any one can be dropped.
- Lake Hylia is not a separate map in this game. Its objects (`Land_LakeHrl_*`, the lab, the fishing pond) live in `HyruleField_Child` and `HyruleField_Adult`, so the Like Likes go in those two `_Static` files, next to the Hyrule Field Dead Hand and Iron Knuckle.
- Actor: `Enemy_LikeLike`, copied from the Shadow Temple instance (others are in Gerudo Training Ground and the enemy museum): a plain actor with `Drop__DropTable` "None", no links, no group. The dungeon room key is left out. The game's own behaviour (it eats the shield and tunic) is left alone.

### Picture to world
Landmarks matched between the picture and the map files (picture pixel, world x/z): the lab house (1055,265 and -174.7,418.9), the fishing pond building (1555,695 and -99.2,487.3), the central island's dead tree (925,860 and -195.1,520.4), the bridge island (880,480 and -204.0,456.2), the stone-ring island (1177,908 and -157.5,523.5), the south-west islet (822,946 and -213.0,530.9), the south island (718,1255 and -233.1,579.0), the west island (310,635 and -295.2,480.5), two west shore trees, and the end of the second rope bridge (905,765 and -200.4,500.8). Eleven points, affine fit: picture x runs along world x, picture y along world z (so north is up and world z grows downward), scale 6.27 pixels per unit, about 1.3 degrees of turn. Fit error 2.6 world units rms (largest about 4.5), mostly from reading pixel centres off the picture. The islands' own objects sit within 0.5 to 2.4 of the dot positions, which is a second check.

### Positions
Ground heights come from objects standing on the same landmass (there is no collision reader). Dots 1, 2 and 3 are on the shore, where the ground slopes and no ground object is within 7 units, so those heights are estimates (up to about 1.5 off): 1 from lake-bottom rocks on that bank, 2 from the lab yard, 3 from the horse target area and rocks.

| # | x | y | z | notes |
|---|---|---|---|---|
| 1 | -202.4 | 133.5 | 403.7 | grass corner, 7 from the river bank pieces |
| 2 | -148.5 | 137.5 | 411.7 | 4.9 from a Tektite |
| 3 | -87.3 | 133.5 | 416.6 | |
| 4 | -206.3 | 130.9 | 452.85 | moved 0.9 from the dot, which sat on the hidden hole's anchors; now 2.5 from the entrance marker |
| 5 | -294.2 | 131.3 | 482.7 | |
| 6 | -188.5 | 131.3 | 510.3 | 13.8 from the warp pad |
| 7 | -155.5 | 131.46 | 523.0 | moved 2 from the dot, which sat on the adult Fire Arrow; now 2.6 from it |
| 8 | -214.3 | 131.3 | 530.7 | 2.8 from the adult scarecrow marker |
| 9 | -292.7 | 131.5 | 547.7 | nearest ground object 6.6 away |
| 10 | -231.7 | 131.0 | 580.8 | gossip stone on the island |

`<playtest folder>\likelikes\placed.png` draws a numbered marker at each of these, transformed back onto your picture (all within a pixel or two of your dots, except 7, 16 pixels off). Water: child level is about 131, so the islands (130.9 to 131.5) are land at the normal level; as adult the lake level changes during the game and the spots stay land either way (not confirmed in the game).

### Tests
- Unit: a Like Like is one plain actor with no links and no groups; the Hyrule Field test map with every place has 17 actors with unique hashes.
- Game files (`OOTRR_ROMFS`): each map gains exactly the expected Like Likes, old actors untouched, unique hashes, groups unchanged; every Like Like is at least 2 from any existing enemy, chest, scarecrow, gossip stone, sign, warp pad or Fire Arrow, and inside the lake area.
- `difficulty` (124) and `settings` (107) pass. A release build is in `<playtest folder>\likelikes\ootrr.exe`. The game was not launched.

### Known risk
Like Like is an enemy kind that Hyrule Field does not have. One earlier black screen (the Dodongo's Cavern edit) is suspected to come from a new kind in a map, but other new kinds (Dead Hand, Iron Knuckle) loaded fine in these same maps. Not isolated. If Hyrule Field goes black, drop rows from `PLACES`.

### In the game
`<playtest folder>\likelikes\TESTS.md`: `teleport HyruleField_Child`, then `warp` to each spot (Lake Hylia has no teleport entry of its own). Look for a Like Like on the ground at each spot, not in the water.

## Dead Hand, round 2 (branch `feature/enc-r2`)
User, after the move to (148.6, 136.41, 13.7): "deadhand still spawns right near the entrance."
- From the position logs: Link landed at (163.3, 136, 27.4). Battle music (`BGM_MiddleBoss`) began about one second after landing, 18 units from the body, so the Dead Hand notices Link from the landing spot whatever the placement. In the run with the old build Link was held at exactly the new spot (148.6, 13.7), so the body does rise at his placement. With the new build Link was held at (150.1, 17.4) after 6 s of walking, 3.8 from the body: that is the south hand of the ring (radius 3), which is the first thing Link walks past, and the grab happens there. The body's AI hides underground, walks towards Link and rises at the next appear position (`QueryDeadHandGetNextAppearPos`) inside an appear area whose range defaults to 6 (blackboard `DeadHand_RangeOfAppearArea`).
- Change: the body now carries `DeadHand_RangeOfAppearArea` 3.5 (the Shadow Temple uses 4.0), so he does not rise until Link is within 3.5 of the back spot, instead of 6. The ring stays at the back (centre unchanged). Hole 23 gets the same. The room is only 18 units long, so the music cannot be delayed by position alone. Not verified in the game.

### Round 4 (19 Nov)

Lake Hylia Like Likes confirmed in the game. Removed `like_like_lake_hylia_north_shore_east` (-148.5, 137.5, 411.7), in the lab's yard: they looked wrong on steep slopes. Nine remain. Slope check from the map data: object heights within 12 units are all within about 1.5 of the Like Like's own height for the island spots; no spot stands out as steep. The data is sparse (the north-west corner has no nearby objects). Candidates near cliff pieces, with edges 7 to 10 units lower: the stone-ring island, the south-west islet, the south island and the west shore strip. Not removed.

## Guays over child Hyrule Field (branch `feature/guays`)
User: "add heaps more guay in the field as child-link".
- Added: 24 `Enemy_Guay_Fixed` in `HyruleField_Child` only (the adult map gains none), as `Place` entries `guay_hyrule_field_<flock>_<n>` in `PLACES`, so any can be dropped. Six flocks of four, about 4 units apart, flocks 50 to 300 apart: south of the west river corner, east of the south edge, north of the ranch, north-middle, far west and the east middle.
- Copied from vanilla: the child map already has five `Enemy_Guay_Fixed` (west side, y 140 to 148) with `Dynamic` `Enemy_Guay_Nocturnal` true and no links or groups. The new ones carry the same key (they are out at night, like the vanilla ones) and nothing else. No new actor kind in the map.
- Positions: x and z from the Peahat Larvae, grass clumps and trees standing in the open field, flock centres between them. Height is the local ground (the larva or grass y) plus 4.5, with up to 1.2 of spread inside a flock: the vanilla Guays sit about 5 above the ground objects near them (the west ones are at 143 to 148 with the ground at 138 to 142). Kept at least 20 from the Iron Knuckle, away from the hole 9 Dead Hand, Lake Hylia, the map entrances and the Lon Lon Ranch walls.
- Not summoning: Guays are a backup helper kind and are barred from calling for backup themselves (`backup.rs`), so these do not call for help.
- Tests: unit test (24 Guays, unique ids and positions, none in the adult map, no links, none within 20 of the knight); game file test (each file gains exactly the expected Guays, none within 4 of an enemy, chest, stone or signpost). `difficulty` and `settings` tests pass.
- In the game: not seen. `teleport HyruleField_Child` at night and look for flocks near (-100, 60), (92, 70), (-100, 315), (40, 322), (-240, 296) and (140, 135). Heights are estimates from object heights, not collision.

## rc 13 scripted check (19 Nov)
- Guays over the field: at (123.68, 159.5, 159.04) eight Guays are in load range (two flocks of four). By day (clock 12) 3 moved over 5 units, at night (clock 23) 4 moved; none vanished or fell over 40 s each. Night-only was not seen: they were active by day too. Damage to Link was not attributed (the Iron Knuckle is in the same field). Details: `playtest
c13\RESULTS.md`.

## Folded into More enemies (19 Nov, `feature/horde-r2`)
The separate checkbox is gone (setting 130 retired). Every More enemies value above 1x now brings all of these encounters. The horde counts the Hyrule Field knight toward its Iron Knuckle limit (2 per Hyrule Field map).


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 22):** Special encounters (rc 15: no longer a setting; part of more enemies above 1x; setting 130 retired): Dead Hand in hole 9 (Hyrule Field) and hole 23 (Death Mountain Trail), Iron Knuckle in the open part of Hyrule Field
- **State:** Done, two tweaks wanted
- **What was seen in the game:** Lake Hylia Like Likes confirmed in the game (19 Nov); the one by the lab removed because of the steep slope, nine remain. Dead Hand and Iron Knuckle confirmed earlier. Build in <playtest folder>\tweaks-r4 Guays (24, child Hyrule Field only) added on `feature/guays`, build in <playtest folder>\guays, not seen in the game. rc13 (19 Nov): Guays over the field: 8 in load range, flying, none vanished or fell, but equally active by day as by night (night-only not seen).
- **Next step:** Tweaks as before

## Temple of Time Beamos reach (21 Nov, `feature/beamos-range`)
Result of the rc 17 test: running down the aisle centre (9.5 units from either row) cost 0 health; only hugging a row (5 to 6 units) did.
- Cause, from the game files: the Beamos sight (`SightParam.DiscoverHorizontalRange`) is 5.95 and the beam itself only reaches about 6 to 7 units. The beam length is not a data field we could find (no length in the actor, beam or shooter files; it is in the beam device code), so it cannot be raised directly.
- Fix 1, a new kind `Enemy_Beamos_Temple`: a copy of the ordinary Beamos pack (`Pack/Actor/Enemy_Beamos_Temple.pack.zs`, own actor file with `$parent` the ordinary one, own perception receiver file, actor and tag rows copied from `Enemy_Beamos`, size table entries). Only the sight differs: discover range 24 and discover angle 90 (was 5.95 and 20). Death table, life, AI and everything else are the ordinary Beamos' own. The ordinary `Enemy_Beamos` pack and its files are never written (a test checks this); Beamos elsewhere stay vanilla. Code: `difficulty/beamos.rs`, written from `slice/edit.rs` when special encounters are on.
- Fix 2, size: the ten placements carry `Scale` 1.6 (`BEAMOS_SCALE`). In the game this lengthens the beam: a vanilla-size Beamos' beams fell 2 to 3 units short of Link at 8 units (beams hit the floor beside him, clip frames), a 1.6 one reaches the whole aisle. Each Beamos still stands in its gap between the pillars, on the row; the aisle is not narrowed.
- Played (21 Nov, Longshot source build, adult, `HavePlayedEvent.Dm_TimeTemple_0065` set before the Market reload, `more_enemies` 200): ten listed; the picture shows several beams on Link along the whole aisle, and Link was lasered (red) mid-aisle. Standing at the centre for 12 s lost 16 quarter hearts (4 hearts) at z -34 and z -37. Six walks down the exact centre (z -34, x 84 to 112 and back, `walk`, about 7 s each): 2, 6, 2, 0, 4 and 0 quarter hearts. Before the fix, scale 1 with ordinary sight: 0. With sight 14 and angle 45 the same six walks gave 6, 0, 2, 2, 6 and 0, so the sight values matter little; the size does. A runner is hit now and then, not every pass: the beam aims, holds, and a moving Link slips out of it. Not tuned further.
- Open: the ten Beamos together deal modest damage to a runner; if more is wanted, a larger scale or more Beamos. The Light Arrows scene is still untested. The vanilla Beamos elsewhere were not touched and not tested again.
- Flag note for tests: set `HavePlayedEvent.Dm_TimeTemple_0065` after every game start (it is not saved by the script) and reload the Market (`teleport 3`) so the Beamos exist.


## Seen by hand (21 Nov, hands-on 9)
- Temple of Time Beamos: absent at first because the fresh save lacked `HavePlayedEvent.Dm_TimeTemple_0065` (by design: they come after Sheik's first talk as an adult). With the flag set and the temple re-entered: they appear and killed Link (life 0). Light Arrows scene not looked at.
- Mad Scrub kill: seen by hand, they die and stay dead until the map reloads.
- Octoroks at the Lake Hylia bridges: scripted only; 3 to 4 at once took 15 hearts while swimming and the frame rate dipped to 22 to 29; not judged by hand.
- Guay (crows) over child Hyrule Field were invisible near (97.7, 148, 66). Suspected the size-clone swap or randomizer; fix is on `playtest/next` (`f0297d2`, Guay flocks). Not seen again by hand.
