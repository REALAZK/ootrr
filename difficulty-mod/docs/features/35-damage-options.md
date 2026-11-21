# Damage options: double, quadruple, one hit kills

Setting `damage_taken` (number 148), a choice on the Difficulty page under "Enemies": Normal (default), Double, Quadruple, One hit kills. Code: `randomizer/core/src/difficulty/hurt.rs`, wired in `difficulty.rs` (`edit_entry`).

## Request
"damage options, up to a one-hit-KO preset" (queued 20 Nov, built 21 Nov). Higher damage steps for everything that hurts Link, up to a hit that takes every heart, like the OHKO modes in other randomizers. Fairies in bottles and Nayru's Love keep working.

## Interpretation
A new setting, not an extension of `enemy_damage`. That slider (50 to 500) scales the numbers in each enemy's own attack file. A one-hit-KO step does not fit a slider, and the sources of damage Link takes are mostly not in enemy files at all. The new setting edits the player's own damage file, so it covers every source at once. The two stack: `enemy_damage` changes what an attack is worth, `damage_taken` changes what Link takes from it. Old settings strings stay valid and the default string is unchanged (checked by a test).

## Investigation
- The player's damage file is `Life/DamageParameters/Player.game__life__DamageParameters.bgyml`, inside `Player_Base`, `Player_Child` and `Player_Adult`. It has two parts:
  - `ExternalDamageParameters.DamageRates`: 19 decimals keyed by hash, all 1.0. This is a multiplier on damage that arrives from attacks (the keys are damage attributes; we scale every one). It is a hash-keyed map (node type 0x20) that the repo's BYML parser cannot read, so it is patched in place, the way `lottery.rs` patches the drop table.
  - `InternalDamageParameters`: whole-number damage for burning (`BurnContinuousDamage`, `IgnitionDamage`), electric shock, falls (`FallDamageMin`, `FallDamagePerMeter`, `FallDamageHighPerMeter`), ice (`ContinuousDamage`), lava, poison bog, scorching heat and drowning. Units are quarter hearts (Link starts with 12).
- Grabs do not go through the attack files (ReDead, Like Like have base attack 0). Each grabber calls `OneShotEnemyRequestPredationEatDamageToPlayer` with `PredationDamageValue` from its `GameParameter/RestraintParam/<Actor>` file. ReDead and Like Like have no value in their file, so the game's own default applies (2 quarter hearts per tick; measured). Dead Hand's hand and Floormaster's little ones have 1. Wallmaster has 0: it carries Link away instead of hurting him.
- The pit and lava respawn ("abyss": Link falls out, loses a heart, wakes at the safe spot) is run by the game's own code (`EventPlayerAbyss` in `AI/Player.event.root.ainb`, no damage value in any file we found). It is not changed.

## Implementation
- Double and Quadruple: every rate and every non-zero whole-number damage in the player's file, and `FallDamagePerMeter` and `FallDamageHighPerMeter`, times 2 or 4.
- One hit kills: every rate becomes 200, every non-zero whole-number damage and `FallDamageMin` becomes 999. Zero stays zero. The per-meter fall values are left alone because the minimum already kills any fall that hurts. Link's life tops out at 80 quarter hearts, so anything from 80 up takes it all.
- Grabs: for every enemy `RestraintParam` file with a non-zero `PredationDamageValue` (or none, which counts as 2), the value is multiplied (999 for one hit). A missing value is added; the file grows, and the late stage's size table handling covers that. Files outside the Player packs and the six grabber files are never touched.
- Bottled fairies: untouched. The fairy fires on death whatever killed Link (seen below). Nayru's Love blocks the hit before damage is worked out, so it is untouched too (not played).
- Build report line per step (`hurt::note`). New setting, default Normal.

## What is covered
| Source | Covered | Seen in the game |
|---|---|---|
| Enemy melee and projectiles (Deku Baba, Keese, others) | yes, through the rates | Baba and Keese |
| Boss attacks | yes, same rates | Volvagia |
| Grabs: ReDead, Gibdo (ReDead's file), Like Like | yes, `PredationDamageValue` | ReDead, Like Like |
| Grabs: Dead Hand's hands, Floormaster, Bongo Bongo's hands, Morpha's tentacle | yes, same value | not played; unit test only |
| Wallmaster | nothing to cover: it deals no damage | not played |
| Falls from height | yes, `FallDamage*` | yes |
| Fire, ice, electric shock, poison bog, heat, drowning | edited in the same table | not played |
| Lava as a damage floor | edited (`LavaDamageParameters`) | not played |
| Pit or lava void-out (the one-heart respawn) | **no**: game code | seen unchanged: 4 quarter hearts |
| Heart or damage taken through scripted events | no | |

## How I tested it
- Unit tests: setting names, the notes, which files count as grabs, the grab edit on small BYML files (default, scaled, zero kept), settings default string unchanged, each step survives the settings string. Game-file test (needs `OOTRR_ROMFS`): all three Player packs are staged with 19 rates at 200, fall minimum and lava damage at 999, lava continuous damage still 0, same file size; ReDead, Like Like, Dead Hand's hand and Floormaster's little files hold 999; Wallmaster has none.
- Scripted game tests on Longshot (the local build with the test commands), windowed, sound off, captions on, Link at 3 hearts (12 quarter hearts), `spawn` to place the enemy, life read every 0.2 to 0.4 s from the control bridge.

**One hit kills**
- Deku Baba, spawned 3 units away in Kokiri Forest: 12 to 0 in one bite.
- Keese: 12 to 0 in one bite.
- ReDead grab: first run before the grab edit existed, life fell 12, 10, 8, 6, 4, 2, 0 (2 per tick). After it: 12 to 0 at once.
- Like Like: 12 to 0.
- Fall: Hyrule Field, warped 25 units up: life 12 until landing, then 0, no enemy near.
- Boss: Volvagia's arena, intro finished, first hit 12 to 0 (position unchanged, standing on the floor).
- Bottled fairy: gave "Bottle: Fairy", Keese bite: 12 to 0, then 12 again with the revive scene (event true then false). Not checked: that the bottle is empty afterwards by eye.
- Pit and lava respawn in the Fire Temple boss arena (warp to y 98.5): life 12 to 8, back at the entrance. Unchanged by the option.

**Double** (Keese bite is 2 in the base game, a fall from 25 units cost a few)
- Keese bites: 12, 8, 4, 0 (4 per bite).
- ReDead grab: 12 to 4 to 0 between samples, which is 4 per tick.
- Fall from 25 units: 12 to 7 (5).

**Quadruple**
- Keese bite: 12 to 4 (8).
- Fall from 25 units: 12 to 1 (11, about twice the Double result).

Not played: Dead Hand, Floormaster, Wallmaster, Bongo Bongo's hands, Morpha's tentacle, fire, ice, electric, poison bog, heat, drowning, lava floors, Nayru's Love, other bosses. One unexplained death was seen once, standing at the Fire Temple entrance with no enemy near (One hit kills, after the lava respawn left Link at 8 quarter hearts); it did not repeat in a later 20 s stand. It may have been heat, which would be covered, but it was not confirmed.

## In the game: how to test it
- One hit kills: any hit from an enemy kills at full health; a bottled fairy revives.
- Double or Quadruple: take a Keese bite (2 normally) or a fall; compare. Grabs tick twice or four times as hard.
- Check Fire Temple heat and a lava floor in the Crater, and a Dead Hand or Floormaster grab, which were not played.

## Notes
- Where it lives matters: the player's file means every attacker is covered, including ones added by other difficulty settings (Volvagia in the field, the enemy randomizer), without listing them.
- The default grab value (2) was measured from the ReDead tick. If a grabber is found that uses another default, set an explicit value for it in `hurt::is_grab`'s file list.
- Stacks with `enemy_damage`: 3x enemy damage with Quadruple is 12x on an attack hit.
