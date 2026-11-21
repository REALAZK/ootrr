# No heart drops

Setting `no_heart_drops` (number 113), checkbox. Code: `crates/core/src/difficulty/drops.rs` and `difficulty/lottery.rs`.

## Request
"no heart drops (checkbox)"

## Interpretation
Enemies, pots, grass and other random drops never give recovery hearts. Hearts placed as shuffled items by the randomizer stay.

## Investigation
- Drops come from `Drop/DropSubTable/*.bgyml` files inside actor packs and `Pack/FoliageDropParam.pack.zs`. Each entry has a type, an optional named actor and a percentage.
- Types are hashed names. Hearts are type 1872319531, or a named `Obj_Heart`.
- The vanilla table `DropTable_Enemy_None` has every entry at 0%, so zero is a normal value.
- **What I missed first:** `GameParameter/DropGlobalParam/Default...bgyml`. It adds 10 to 30 points of drop chance to hearts when your health is low, and names the heart actor itself. The repo's parser could not read this file, so I had skipped it.

## Implementation
- **Tables:** every heart entry in every drop table gets a chance of 0.
- **Global file (added in rc 2):** the low-health boost for hearts is set to 0, the "heart actor" is pointed at the green rupee, and the rule that turns a fairy into a heart is pointed at the green rupee too. This is done by patching the file in place, since it cannot be parsed normally.

## How I tested it
- Unit tests for the table edit.
- A test against your game files that reads the global file before and after and checks only the heart boost changed.
- rc 1 in the game, by you: **hearts still dropped.** That led to the global file.

## In the game
rc 2's fix has not been played. To test:
- Turn this on with nothing else. Get down to one heart, then kill a dozen enemies and break pots and grass.
- Pass: no hearts at all; you may see more green rupees.
- Also break a pot that always gives a heart in vanilla (the ones before boss doors).
- If hearts still appear, tell me what dropped them (enemy type, pot, grass, a specific room).

## Notes
- The lesson for any drop work: the per-actor table is not the whole story; check the global lottery.
- If you use pot or grass shuffle, hearts placed there as items are intended to remain.

## Seen in the game (18 Nov, batch 1, aborted)
Mod: no_heart_drops, no_ammo_drops, no_gossip_fairies, enemy_damage 500, enemy_health 50 (all ON). Link at 58 of 80 quarter hearts (bombs on himself, 2 quarter hearts each), walking and swinging the sword in Kokiri Forest. A "You got a recovery heart!" box appeared at (-9.1, 111.8, 17.4): life 58 to 62. Two hearts were visible on the ground in the screenshot. The script did not log what was cut or which actor dropped them, so it is NOT known whether this was a drop (FAIL) or a heart the map places (allowed). Result: NOT TESTED; needs a run that watches each cut. Screenshot kept under playtestatch\shots.


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 04):** No heart drops
- **State:** Done (small sample)
- **What was seen in the game:** User (18 Nov): about 20 grass cut in the Lost Woods, only rupees and bugs dropped
- **Next step:** Hearts the map places on the ground are left alone (user: fine unless they respawn; to check from the files)
