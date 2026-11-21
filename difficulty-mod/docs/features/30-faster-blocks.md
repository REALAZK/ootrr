# 30. Faster sliding blocks

## Requested (user, 20 Nov)

"Make a time-saver 'faster sliding blocks' so when you're pulling/pushing blocks, it's 2x faster." A general time saver, not a difficulty feature, so it is built on its own branch off upstream main (`feature/faster-blocks`, worktree `wt\blocks`) to go in as its own pull request.

## Setting

`faster_blocks`, number 141, "Faster sliding blocks", toggle, default off. Tip: "Pushable and pullable blocks slide twice as fast."

Menu: Time savers page, new section **Puzzles**. Cutscenes is about skipping scenes, Side quests about two NPC quests and Dungeons holds the safe Deku bridge (a dungeon fix). Blocks are neither a cutscene nor a quest, and they sit in dungeons and also in Hyrule Field and Lon Lon Ranch, so they get a section of their own that later puzzle time savers can join.

141 is the first free number above everything used: main stops at 135, our branches use 136 to 140, and the retired numbers stay untouched. Gaps below (47 to 50 and a few others) are unused in the list but were left alone, so a number the maintainers may have kept back is never taken.

## How the game moves a block (found in the game files)

- Pushing and pulling is driven by the player's graph `Player.action.grabpushpull` (in `Player_Base`, `Player_Adult`, `Player_Child`). Link "rides" the block (`ExecutePlayerRiderMovableBoxControl`) and the grab animation is chosen by the block's `GrabMoveActorType` (`Movable_Push`, `_M`, `_L`, `Movable_Pull` and others in `AS/Player_*.root.asb`).
- The block's own pack carries `GameParameter/GrabMoveActorParam/<name>`: a `MoveSpeed` curve of type `Hermit2DSplit`. It holds knots of four floats (frame, progress 0 to 1, slope in, slope out) and `MaxX`, the last frame. The plain block `Obj_MovableBox_A_01` has five knots over 44 frames, the 2.5-unit block `A_05` four knots over 53 frames, the large block `A_04` six knots over 50 frames. Progress runs from 0 to 1 and ends flat, so one push or pull is one pass through the curve.
- `MovableBoxParam` (`PushPerBlock` 2, or 10 for the large block, `IfCliffNearThenPushRate` 0.25, `MoveSpeed` 1.0) says how many pushes make one block length and is left alone.
- Other pushed things use the same curve type with their own numbers: tombstones (95 frames), the Amos statue, the Shadow Temple statue, the Goron statue (it inherits the `A_05` curve), the Spirit Temple mirror, the slider box and the Forest Temple pivoting floor. They are left alone.

## What the setting does

For the 13 block actor packs it rewrites each block curve so a push or pull plays twice as fast: every frame number and `MaxX` is halved and every slope doubled (slopes are progress per frame). Progress values are untouched, so a block still travels exactly one grid step per pass and stops where it always stopped. The edits are exact in floating point (halving and doubling). Nothing else in a pack changes. Data edit only, no code patch, no new crate.

Packs: `Obj_MovableBox_A_01`, `A_04`, `A_05`, `Obj_FireTemple_MovableBox_A_01`, `A_02`, `Obj_ForestTemple_MovableBox_A_01` to `A_05`, `Obj_LonLonRanch_MovableBox_A_01`, `Obj_SpiritTemple_MovableBox_A_01`, `ForestTemple_MovableBox_Puzzle_Manager`. The same curve file sits in several packs, so every copy is edited identically.

Maps with placed blocks (from the Static map files): Deku Tree (3), Dodongo's Cavern (5), Forest Temple (18), Fire Temple (6), Water Temple (4), Shadow Temple (3), Spirit Temple (9), Gerudo Training Ground (3), Hyrule Castle (3), Hyrule Field child (8) and the Stalfos stage (1). The counts are name matches in the map files, not placed-block counts.

## Left out on purpose

- Tombstones, statues, the mirror and the slider: not blocks.
- `Obj_SongOfTimeBlock_A_01` has no push curve (it appears and disappears), so it is not touched.
- Nothing about blocks is on a timer. The Forest Temple manager only compares block positions. No block was excluded for timing.

## Risk to look at first

The curve is read by frame. If the game takes the frame from Link's push animation instead of a clock, halving the curve makes the block finish in half the animation while Link keeps animating. The block still stops on its spot and the puzzle still works, but the speed-up would look wrong or be smaller than 2x. The fallbacks are to speed Link's push animations too (in the `.asb`, which this repo cannot edit yet) or to try the one float `MovableBoxParam.MoveSpeed` (1.0 on every block): set it to 2.0 in the same packs. Nothing we could read uses that value, so it is a guess.

## Tested

- Unit tests in `qol/blocks.rs`: frames halve, slopes double, progress and curve type stay; other curve shapes are refused or skipped; only block curves in a pack change.
- Game-file tests (`OOTRR_ROMFS`): every pack in the list changes only its curve files, every knot value is exactly what it should be, frame numbers still increase, `MaxX` halves, off writes nothing, on writes exactly the 13 packs.
- Settings and settings-string tests pass; default settings and the default seed are unchanged.

NOT played in the game. See `<playtest folder>\blocks\PLAN.md`.

## How to test in the game

Generate with `faster_blocks` on plus `skip_intro`, `skip_cutscenes` "all", `faster_chests`; build the mod; walk to the blocks in `PLAN.md`; push and pull a block one step each way, and compare with a build without the setting. Look for: speed about double, Link's hands on the block, the block stopping on its grid spot, the footstep and slide sounds, switches and holes still triggering, no soft lock when a block meets a wall.


## Moved from STATUS.md (20 Nov): request

- **New request (20 Nov): Faster sliding blocks** (push and pull 2x), a time saver. Built as its own upstream pull request off main (`feature/faster-blocks`).
