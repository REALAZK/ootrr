# Enemy health multiplier

Setting `enemy_health` (number 110), slider 0.5x to 5x. Code: `crates/core/src/difficulty/stats.rs` (`life`).

## Request
"enemy health multiplier (from 0.5x to 5x)"

## Interpretation
One slider that scales the health of every enemy and boss. Stored as a whole percentage (50 to 500) because settings hold integers; shown as a multiplier in steps of 0.25.

## Investigation
- Listed the files inside `Enemy_Stalfos.pack.zs` and found `Life/LifeParameters/Enemy_Stalfos.game__life__LifeParameters.bgyml` with a single `MaxLife` number (18 for a Stalfos).
- Surveyed that file across all actor packs: 109 files, about 85 for enemies. Values run from 1 (Keese, Octorok) to 70 (Ganondorf).
- Two enemies have `MaxLife` 100 (Deku Scrubs, Morpha's tentacle). That looks like a "cannot die from damage" value for enemies beaten by a scripted hit.

## Implementation
- For every life file whose name starts with `Enemy_` (or is `Keese`), multiply `MaxLife`, round, and keep it between 1 and 99.
- Values of 100 or more are left alone.
- The file size does not change, so no size table work.

## How I tested it
- Unit tests for rounding and the skip rule.
- A test against your game files: a Stalfos pack built at 2x has `MaxLife` 36 and only a handful of files differ.
- A full mod build, then a field-by-field diff of all 243 rewritten packs against vanilla: 152 `MaxLife` values changed and nothing unexpected.

## In the game
**You confirmed this works in rc 1.** To test further:
- Set 5x and count sword hits on a Deku Baba or Stalfos against vanilla.
- Set 0.5x and check that 1-health enemies still die in one hit.
- Check a Deku Scrub still behaves normally at 5x.
- Bosses with phases (Volvagia, Ganondorf) are worth one run at 2x to see the phases still trigger.

## Notes
- This is the template for any "scale a number on every enemy" feature: find the parameter file in one pack, survey it across all packs, filter by file name.
- You reported enemies looking red. I found no cause in this edit; it only changes one number per enemy.


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 01):** Enemy health
- **State:** Done
- **What was seen in the game:** Scaling seen (19 Nov): at enemy_health 200 the overlay shows Stalfos 36/36 and Lizalfos 18/18, exactly double vanilla 18 and 9
- **Next step:** Peahat larva not read
