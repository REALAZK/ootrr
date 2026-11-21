# Spawn enemies with L + R + D-pad

Setting `enemy_spawn_combo` (number 132), checkbox on the Difficulty page under "Just for fun". Code: `randomizer/core/src/difficulty/spawn.rs`. Built on `feature/spawn-combo` (`ab226b0`); fixed on `feature/opus-fixes` (`4efaaaf`) and seen working in a scripted test (end of this file).

## Request
"can you make left bumper + right bumper + dpad down spawn random, and l+r+up spawn prev?" and then "the l+r+d-pad thing shouldn't need longshot update, right? so it'll work anywhere the ootrr does?"

## Interpretation
While L and R (the shoulder buttons) are held, D-pad Down spawns a random ordinary enemy a few steps in front of Link, and D-pad Up spawns the same kind as last time. Data edits only, so it works wherever the mod does.

## Investigation
- **Buttons can be read.** The node table has `QueryControllerGetMask` (outputs a hold mask, a release mask, a repeat mask, a press mask and a press-or-repeat mask) and `QueryControllerTrigMaskChecker` (turns a mask into one bool per button: A, B, X, Y, L, R, ZL, ZR, Up, Down, Left, Right, Plus, Minus and both stick clicks). The game uses the pair once, in `GameSystemActor.event.root`, to wait for A, B, X or Y during events. No vanilla graph reads shoulder buttons or the D-pad this way.
- **Virtual keys are the wrong tool.** The player graphs use `QueryPlayerIsVirtualKeyHold` and `Trig` with numbers 4, 5 and 18 (grab, hookshot). Their names are in game code, not data, so the enum cannot be read from the files, and they are game actions and not raw buttons.
- **Creating the enemy.** `ExecuteGenericRequestCreateActor` does not exist in this game's node table. The data route is the actor holder: `OneShotActorHolderTryRequestCreateActorWithActorFileName` takes an actor name and a holder key (vanilla: treasure chests create their contents with it). The holder entry `Content` in `Obj_TBox` has no actor and `IsWildActor` true; the new `OotrrSpawn` entry copies it. The created actor sleeps in the holder until `OneShotActorHolderTryCancelSleepActor` places it at a position, the same two steps the backup feature (17) uses.
- **Random and same again.** There is no random integer node and no int or string blackboard value on the player. `QueryGenericGetRandomBool` is vanilla. Three coin flips are stored as three bool keys in the player's blackboard table (as `wake.rs` adds a key to the knight's), and a decision tree of switches picks one of eight enemies from the three bits. Down rolls and stores new bits; Up keeps the stored ones.
- **Where it runs.** `Player.scheduler.root.ainb` is a Parallel of three scheduler modules, identical in the Base, Child and Adult packs. A fourth child runs every frame.
- **Press edge.** The press mask is true on the frame of the press only, so holding Down does not create one enemy per frame, and no timer is needed.

## Implementation
- Graph (about 75 nodes added to `Player.scheduler.root`): a mask query and two checkers (hold mask for L and R, press mask for Down and Up); conjunctions for "L and R and Down" and "L and R and Up"; three random bools and six blackboard setters and getters; the tree of seven switches with eight create nodes (one per enemy, a const list in `spawn.rs`); the wake chain (ready check, position, forward direction times 3, plus 1 up, wake).
- To avoid reading a key in the same frame it was written, a press sets a `pending` flag and the tree runs on the next frame, reading the stored bits and clearing the flag.
- The pool, in tree order: Stalfos, Lizalfos, Wolfos, Dodongo, Keese, Octorok, Deku Baba, Torch Slug.
- Child and Adult packs: the graph, four bool keys in `PlayerComponent` (`OotrrSpawnBit0..2`, `OotrrSpawnPending`), and the holder entry `OotrrSpawn` (wild actor, at most 8). The actor table heap of both players grows (shared with the Bunny Hood edit).

## Version 2: 43 kinds (20 Nov)
The request: all 45 Longshot tool enemies except the spear Moblin and the Gerudo thief, so 43 (all exist in `Pack/Actor`). The pool is a const in `spawn.rs`. The README now lists the feature as experimental.
- **The cap is not on kinds.** The holder entry `OotrrSpawn` is one wild entry and the actor name is chosen at request time, so it holds any number of kinds. The only limit is `MaxActors` on that entry, which I set (vanilla entries use 1 to 20, the Zora shop 20). It is now 12: that many spawned enemies can be alive at once. Whether a dead enemy frees its slot is not visible in the files.
- **Choosing 43:** six random bools (64 codes) and a tree of 63 switches with 64 create nodes. Code i maps to kind i * 43 / 64, which reaches every kind once or twice (21 kinds are twice as likely). Six bool keys are stored, so "same again" (L+R+Up) repeats the last kind.
- The graph is about 190 nodes; the player heap growth is unchanged and the tests build it.
- Kinds that did not appear in Hyrule Field with the tool (Keese_Fire, Gibdo, IronKnuckle_Silver, Bubble_Blue, Poe_Big) are in; they appear only where the area has loaded them.

## Version 3: the keys move into the graph (18 Nov, not committed, not yet run in the game)
- **The defect.** Versions 1 and 2 crashed as soon as a spawn key was written (five crash records, all at `longshot.exe+0x753a5b3`, a read near -1; the crashing node object in the log holds a key handle of -1). The set and get nodes used `BlackboardType` 1, the actor's blackboard, and the keys were added to the `PlayerComponent` table. The player's blackboard list (`BlackboardInfo/Player_Base`) names only `Player_Base_Graphics`, so those keys were never part of it, and the write went through a handle that was never resolved. The no-error getter hid this: it just answers false.
- **What vanilla does.** All 46 blackboard nodes in the player's graphs use `BlackboardType` 0, the graph's own keys, declared in a section of the graph file itself (the hookshot, lift and push-pull modules do this). No player graph uses type 1. Enemies do (`HasArmor` on the Iron Knuckle), with keys in a table their blackboard list names.
- **The fix.** The seven bools are declared in `Player.scheduler.root`'s own key section (it was empty) and all spawn nodes use type 0. The `PlayerComponent` table is no longer touched. The section layout: seven kinds of 8 bytes each (count, first index, value offset), then one row per key (name with flag 0x00800000, notes), then 4 bytes per bool. A test rebuilds the section of four vanilla player modules byte for byte.
- Tests: 10 in `spawn.rs`, 133 difficulty, 107 settings. Build: `<playtest folder>\spawn-combo\v3\ootrr.exe`. Probe builds (keys written every frame; L+R held; both stick clicks held; stick clicks as the combination) are in `<playtest folder>\spawn-combo\mods`, made by `diag\ootrr.exe` (`diag\probe.patch`), driver `run3.py`.
- Still unknown until it runs: the controller read from the player's graph, and the holder creating an enemy.
- (Wrong, see 17: backup crashed for a related reason, a write to a placement key.) The backup feature (17) does not have this defect: its keys are in `EnemyCommon`, which ordinary enemies' blackboard lists name, and type 1 is what vanilla enemy graphs use. Actors that run the shared graph without that table read "not eligible" and never reach the write.

## How it was tested
- rc 13 (19 Nov, merged build `playtest/next` 24b9bba, bridge `sequence` L+R held, D-pad Down): six presses gave 5 new enemies (combo alone), 10 (with regen, fairies, backup, special encounters, cucco revenge, gates, no Z-targeting, no bottles, lose heart) and 9 (with all other settings). The reported failure of the merged build did not reproduce; likely a pre-fix build or real pad input. Up-repeat and a real controller were not tried.
- Nine tests in `spawn.rs` (with `OOTRR_ROMFS`): every new node has the same property, input and output names, kinds and pointer classes as a vanilla node it copies (controller pair from `GameSystemActor`, random bool, conjunction, set bool, vector multiply and add, forward direction, the create node from the chest graph, the blackboard getter); the checker reads L, R, Down and Up and the mask feeds the hold checker from output 0 and the press checker from output 3; the scheduler gains exactly a fourth child and keeps the old three; each enemy has one create node; the pack gets the graph, keys and holder entry and refuses a second patch; the holder entry equals the chest entry plus `MaxActors`.
- Settings read, string round trip, default off, 107 settings tests, 132 difficulty tests; a real build writes both player packs.
- Not run in the game. The game was not started.

## How to test in the game
See `<playtest folder>\spawn-combo\TESTS.md`. Turn the Longshot tool's own L + R + D-pad shortcut off first.

## Notes: what is a guess
1. The controller query works from the player's AI. Vanilla only uses it from the game system actor.
2. The hold mask can be given to the press checker, whose input is named for the press mask. Its outputs are named Trig, but the layout is expected to be the same.
3. A holder on the player accepts a wild entry and creates enemies. The player's own holder only lists items.
4. The enemy's pack loads on demand. Where the map has not loaded it, nothing appears (as with the Longshot tool: Gibdo and the Iron Knuckle did not appear in Hyrule Field).
5. Random bools are rolled once per read. The stored bits make that irrelevant, but if the three reads are not independent the pool is smaller.
6. Dead spawned enemies free their holder slot. If not, nothing new appears after eight.
7. Spawned enemies are outside the room, so they do not count for doors or counters. Cutscenes: the scheduler may not run, so nothing happens then. Underwater: untested.
8. L and R and the D-pad may already do things in the game (camera, items). The mod does not hide them.
- If the read fails, the nearest sure thing is an action already in the player graphs, for example Navi call (D-pad Up) while shielding; it needs new nodes and is not built.

## Opus fix: one press, one enemy (19 Nov, `feature/opus-fixes`, commits `e4ff217` and `4efaaaf`)
- **First run of v3 in the game** (scripted, bridge presses: L+R held 60 frames, D-pad at frame 20): the first L+R+Down made a Dinolfos 3.0 m in front of Link, no crash. The next 13 presses made nothing. So the player's graph is ticked, the controller query works from the player's graph, bridge presses reach it, and a holder on the player creates enemies. Only the second create failed.
- **Defect.** Every create node used holder slot `Index` 0. The holder keeps one actor per slot, so once slot 0 held a living enemy every later request was refused. Vanilla precedent: Gohma's egg summon feeds `Index` from a counter key and wakes by index.
- **Fix.** A local s32 key `OotrrSpawnSlot` in the scheduler's own key section; every create reads it (`QueryBlackboardGetS32`), and the firing Parallel also writes slot + 1, back to 0 at 12 (`QueryMathAddS32`, `QueryMathLessS32`, `QuerySelectS32ByBool`, `ExecuteBlackboardSetS32`). The wake takes its `Index` from `QueryActorHolderGetEnableCancelSleepIndex`, the index of the sleeping actor. Also from the earlier unfinished patch: a node that reads one query twice (the L and R conjunction) now lists it twice in its query list, as `GameSystemActor.event.root` does.
- **Tests.** `each_spawn_takes_the_next_holder_slot_and_the_wake_takes_the_sleeping_one` (shapes of the six new node kinds against vanilla nodes; all 64 creates read the slot; the wake reads the sleeper index), the key-section test now also rebuilds a vanilla s32 + bool section byte for byte, `a_node_that_reads_one_query_twice_lists_it_twice_as_the_game_does`.
- **Seen in the game** (scripted, Hyrule Field, `<playtest folder>\opus-fixes\TESTS.md`): Down: Peahat; Up: Peahat again; then Floormaster, Bari, Wallmaster, Lizalfos, Big Leever, Stalfos, Dodongo, Amos, ReDead, Poe, each 3 m ahead of Link and bringing its own children (Biri, small Floormasters, Leevers, Peahat larvae). After 12 alive, two more presses made nothing: the cap is 12. No crash.
- **Why the user saw nothing.** Most likely the Longshot test tool: its own L+R+D-pad spawn shortcut (rc 11 tools) takes the D-pad presses while the pad's L and R are held and the bridge is on, so the mod never sees them. The bumper combination is right (L and R, not ZL/ZR, and not the sticks); test with that shortcut off or with a Longshot without it. Not proven on a real pad.
- **Still open.** Whether a killed spawned enemy frees its slot (not tested: after 12 the slots are reused in turn, and a slot whose enemy still lives refuses the request, so with no kills nothing more appears).

## Stick clicks (19 Nov, branch `feature/fx-r1`, commit 91c3d0a)
- **Request (hands-on, 19 Nov).** L + R + D-pad never worked by hand: in the game L and R are both "Quick Select Items" and open the item wheel. User decision: hold both stick clicks (L3 + R3), then D-pad Down = random enemy, D-pad Up = same again.
- **Change.** The hold check now reads `TrigLSPush` and `TrigRSPush` (outputs 5 and 10 of `QueryControllerTrigMaskChecker` on the hold mask) instead of `TrigL` / `TrigR` (3 and 8). The press check is unchanged (`TrigDown` 2, `TrigUp` 11). The checker's outputs are named, so the bit is whatever the game maps to that name; the names come from the vanilla `GameSystemActor.event.root` checker, which the shape test copies.
- **Tested.** Unit and game-file tests (`the_buttons_are_read_from_the_hold_mask_and_the_press_mask` now expects the two stick pushes). Not run in the game.
- **How to test in the game.** Hold both sticks in, press D-pad Down: an enemy appears 3 m ahead. With both held, D-pad Up: same kind again. Stick clicks alone, or one stick and the D-pad, do nothing.


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 25):** Spawn enemies with BOTH STICK CLICKS + D-pad (rc 15; confirmed by hand 19 Nov: "glorious". The old L + R never worked by hand: both open the item wheel) (setting 132, `enemy_spawn_combo`): Down spawns a random ordinary enemy in front of Link, Up spawns the same again
- **State:** Fixed, scripted test passed
- **What was seen in the game:** Opus fix (19 Nov, `feature/opus-fixes` 4efaaaf): every create used holder slot 0, so only the first press worked; now a slot counter. Bridge L+R+Down: 12 presses, 12 different enemies 3 m ahead (Up repeated the kind), the 13th and 14th nothing (cap 12). The user probably saw nothing because Longshot's own L+R+D-pad tool shortcut takes the D-pad rc13 (19 Nov, merged build, script): 6 combo presses gave 5 to 10 new enemies with the combo alone, with every other setting group on, and with all other settings on; could not reproduce the failure.
- **Next step:** User: real pad with the Longshot shortcut off; check that a kill frees a slot fx-r1 (19 Nov, 91c3d0a, code only): combo is now both stick clicks (L3 + R3) + D-pad, user decision.
