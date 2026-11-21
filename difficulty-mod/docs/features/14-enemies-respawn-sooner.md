# Enemies respawn sooner

Setting `enemies_respawn_sooner` (number 123), checkbox. Code: `crates/core/src/difficulty/respawn.rs`. Built by a sub-agent on `feature/faster-respawns` (`11a46b5`).

## Request
"enemies respawn more frequently"

## Interpretation
Defeated enemies come back sooner than in the unmodified game. Nobody knew how respawning worked, so finding that out was the first job.

## Investigation
- The game ships a few "AlwaysRespawn" enemy variants (a Stalfos, a Shabom, a ReDead, an Armos). Comparing one with its normal twin gave the answer.
- Whether a defeat is remembered is set in "death tables": loose files under `System/CombinationDataTableData/` named `..._DeathReasonProcedures...bgyml`. Two fields in the "life ran out" cell decide it. The AlwaysRespawn variants set them to false.
- The remembered state is a save flag keyed by the enemy's map id. Ordinary enemies already reset when the whole area reloads, so they do respawn in vanilla, just rarely.

## Implementation
- 31 death tables get both fields set to false, so a beaten enemy is never recorded.
- Covers about 50 ordinary enemy types (689 placements): Stalfos, Keese, Wolfos, Tektites, Octorok, Skulltula, ReDead, Deku Baba, Like Like, Moblin, Armos and more.
- Skipped on purpose: bosses, minibosses, Iron Knuckles, Dead Hand, Poes, Skull Kid, Anubis, Deku Scrubs, Gold Skulltulas.
- Skipped because their table is shared with a boss or miniboss: Lizalfos, Dinolfos, Baby Dodongo, Beamos, flying pots, Gohma larvae.

## How it was tested
- 20 difficulty tests, three against your game files. One walks every actor pack and asserts that every actor reading a changed table is an ordinary enemy.
- A real build writes exactly the 31 tables.

## In the game
Never played. To test:
- Turn this on alone. Kill some enemies in a room, leave through a door, come back.
- Pass: they are back. Without the setting they would stay gone until the whole dungeon reloads.
- Then clear a "defeat all enemies" room, leave and return. Pass: the door or chest stays open or present even though the enemies are back.
- Report a room that re-locks or a chest that disappears.

## Notes
- Room clears go through a separate counter object that stays permanent, and vanilla already links always-respawning Shaboms to such counters in Jabu-Jabu. That is the evidence it is safe; it is not proof.
- A half-cleared room probably starts over if you leave.
- The "diff a special variant against its normal twin" trick is the fastest way to find which field controls a behaviour.


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 14):** Enemies respawn sooner
- **State:** Built, untested
- **What was seen in the game:** Nothing
- **Next step:** Batch 2
