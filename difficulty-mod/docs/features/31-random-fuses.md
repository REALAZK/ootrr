# 31. Random fuses

Setting `random_fuses` (number 151, was 142 before rc 19), toggle, default off. Difficulty page, new section "Hazards" (it exists on `test/handson2` with fireball storms; this branch is cut from `playtest/next`, which has no such section yet, so merging with the fireball branch needs the two section lines joined).

## What was requested

User, 20 Nov: "random fuses: bombs, Deku sticks and bombchus have a random fuse, randomly between 1 s and whatever the default is."

## The fuses in the game data (60 frames a second)

| Item | Actor / file | Default | Where it lives |
|---|---|---|---|
| Bomb | `Item_Bomb`, `AI/Item_Bomb.root.ainb` | 240 frames, 4 s (inferred) | The `Bomb_Blink` animation (AS `LiftableExplodeObject_BallBomb`). The graph plays it, then asks for death reason 10 (`OneShotDeathRequestDeath`) unless the bomb is being put down. A 240-frame `ExecuteGenericCounter` and the `TerrorStartTimer` 240 only time the fear signal enemies get. |
| Bombchu | `Item_Bombchu`, `AI/Item_Bombchu_Remake.root.ainb` | 210 frames, 3.5 s (inferred) | The graph has no death request at all; the native `ExecuteBombchuScheduler` and `ExecuteBarrelBombAI` (IsBombchu true) do it. The only number is the terror timer constant 210 (`InputA` of a subtraction from the `Bomb_Blink` frame). |
| Lit Deku Stick | `Weapon_DekuStick`, chemical param `Weapon_Chemical_Wood` | 1440 frames, 24 s | `FixedBurningDuration` in the chemical param (shared by every stick), then `RequestDeathIfBurned`. |

The defaults for bomb and Bombchu are inferred (the clip length is in the model, which we cannot read); the stick's is a plain number. All three are unverified in the game.

## Per item or per seed

Per item. `SelectorGenericFrameCounter` has `CountFrames` plus `RandomAddCountFrames`: the game rolls the extra for each instance when the node is entered, and the vanilla graphs rely on that (Wolfos, Obj_Fairy, Cucco; the backup feature uses it too). A random query would re-roll every frame; this node does not, so nothing needs latching and nothing is stored. The same node type is used in `regen.rs` and `backup.rs`.

## How it works

`difficulty/fuses.rs` edits three graphs (one pack each: `Item_Bomb`, `Item_Bombchu`, `Weapon_DekuStick`), adding three nodes each: an idle, the counter (NotFinish idle, Finish the action) and the action.

- Bomb: the counter is a new child of the parallel that already runs the blink and the death request (three levels above the vanilla `OneShotDeathRequestDeath`). Finish runs a new death request with the same reason 10. Fuse 60 to 240 frames (1 to 4 s). The vanilla death still fires at 240, so the bomb ends at the earlier of the two.
- Bombchu: the root of the graph is a parallel that holds the scheduler; the counter is another child of it, so it keeps running when the Bombchu changes between ground, wall, cliff and water (a per-state counter would restart on each change). Finish runs the same death request, reason 10. Fuse 120 to 210 frames (2 to 3.5 s; user, 20 Nov: the chu's minimum is 2 s, bombs stay 1 to 4 s).
- Stick: the counter is a new child of the parallel inside the burning state (the one that emits the fire terror). Finish runs `OneShotChemicalRequestExtinguish`: the stick goes out, and lighting it again rolls a new fuse. Fuse 600 to 1440 frames (10 to 24 s).

The three actors' heap sizes grow by 0x1000 (`RSDB/ActorInfo`), as the regen feature does for its added nodes.

## What is left alone, and why

- **Bomb flowers** (`Obj_BombFlower_*`, graph `Obj_BombFlower_Fruit`) and the bomb Imperial soldiers throw (`Npc_Hylia_ImperialSoldier_Bomb`) are separate actors. Their packs carry a copy of `Item_Bomb.root.ainb`, but we edit only `Item_Bomb.pack.zs`, so flower timers and Goron City / Dodongo's Cavern puzzles are unchanged.
- **Bombchu bowling** shares the one `Item_Bombchu` actor, so a short fuse made the minigame impossible (its prizes can hold seed items). Fix (21 Nov, `fix/bowling-fuse`): every Bombchu fuse is now only longer than vanilla, 210 to 315 frames (3.5 to 5.25 s, vanilla to 1.5x). Bombs and sticks unchanged. Not played in the game; check Bowling by hand.
- **Stick floor is 10 s, not 1 s.** A stick burning for 1 s cannot carry a flame across most rooms to a torch (for example the Deku Tree and Dodongo's Cavern torches). Ten seconds (user decision, 20 Nov; first 5 s) is an estimate, not measured against every torch puzzle; it is one constant (`STICK_FLOOR`) if it needs to be raised.
- The blink animation does not follow the new fuse. A short-fused bomb blinks slowly and explodes with no change in the blink speed. Syncing it (setting the animation frame from the counter's progress) needs the game to try.
- Bomb enemies, Dodongo's bomb eating and anything that is not the player's bomb are untouched.

## Risks (nothing is seen in the game)

1. Death reason 10 is copied from the bomb graph; if it does not blow up a Bombchu the way the native code does, the Bombchu would vanish without an explosion. Check this first.
2. The counter's random roll is taken from vanilla use, not seen working on these actors.
3. The stick's extinguish node was found by name (vanilla uses it on Bubbles and candlesticks); a stick that goes out may still be shown as burning for a moment.

## Tests

- Unit: the fuse constants (bomb floor 1 s, chu floor 2 s, tops 4 s, 3.5 s, 24 s, stick floor 10 s), garbage refused.
- Game files (`OOTRR_ROMFS`): each of the three packs gets exactly three new nodes and one counter with the expected `CountFrames` and `RandomAddCountFrames`, the audit is clean, the graphs read back, the bomb graph has two death requests with reason 10, the Bombchu root is the same parallel as before, the stick graph has the extinguish node, a second run is refused, and `RSDB/ActorInfo` is staged.
- `cargo test -p ootrr-core difficulty` (206 passed), `settings` (106 passed).

## How to test in the game

Turn on Random fuses, start a save with bombs, Bombchus and Deku Sticks.
1. Drop 10 bombs one after the other: the time to explosion should vary between about 1 and 4 seconds.
2. Same with Bombchus (about 2 to 3.5 s), and check that each one still explodes (a visible explosion, not just a vanish).
3. Light a stick on a torch and time it: 10 to 24 s, lit again it gets a new time.
4. Bomb flowers in Goron City and Dodongo's Cavern still take their usual time.
5. The bowling alley: note whether the shorter Bombchus are still playable.


## Moved from STATUS.md (20 Nov): random fuses scripted result and decisions

  - **Random fuses: works.** 20 bombs (1.1 to 3.9 s) and 10 chus (1.1 to 2.5 s) all exploded, none vanished. Sticks burned 11.3 to 18.6 s (tested on the 5 to 24 s build; now 10 to 24). Bombchu Bowling and bomb flowers not tested.

- **Random fuses** (user request, 20 Nov): built (`82d4176` on `feature/random-fuses`, setting 142, Hazards section, not played). A fuse is rolled once per item through the game's own frame counter with a random add. Bombs 1 to 4 s, bombchus 2 to 3.5 s (user, 20 Nov), lit sticks 10 to 24 s (user, 20 Nov). Bombchu Bowling shares the chu actor, so its chus get short fuses too. Risk: the death reason copied from the bomb might delete a chu without an explosion.


## After the 21 Nov hands-on (`fix/playtest-misc`, on `playtest/next` only)
- Seen by hand: bombs and Bombchus are perfect (user).
- Deku Sticks were wrong: when the random timer ended the flame went out but the stick stayed in Link's hand. The stick's counter now ends through the same death request the bombs use (reason 10, `death()` in `fuses.rs`) instead of `OneShotChemicalRequestExtinguish`, so it burns up and is used up like a vanilla stick. Not confirmed in the game afterwards (the stick step is the same panel step that was marked failed before the fix). The "How it works" line above that names the extinguish node describes the old build.


## Bombchu range changed (21 Nov)
The 120 to 210 frame range above is superseded by 210 to 315 frames (see Bombchu bowling). A unit test pins the floor at the vanilla 210 and the top at no more than 1.5x.
