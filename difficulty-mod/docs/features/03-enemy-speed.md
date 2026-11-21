# Enemy speed multiplier

Setting `enemy_speed` (number 112), slider 0.75x to 2x. Code: `crates/core/src/difficulty/stats.rs` (`movement`, `pace`).

## Request
"enemy speed multiplier (from 0.75 to 2x)"

## Interpretation
Enemies move faster or slower. I also made them attack more or less often, since "speed" in other randomizers covers both.

## Investigation
- `BSA/MovementParam/<Enemy>.game__bsa__MovementParam.bgyml` holds walk, run, dash, strafe and escape speeds. Only 53 enemies have their own file.
- `GameBalance/AttackInterval/<Enemy>...bgyml` holds minimum and maximum waits between attacks.
- Enemies without a movement file (Octorok, Skulltula) move through their animations or AI graphs. I found no single "time scale" for an actor.

## Implementation
- Multiply every field ending in `Speed` except the `Rotate` ones, in movement files named `Enemy_*` or `Keese`.
- Divide every interval in attack interval files named `Enemy_*`.
- Sizes do not change.

## How I tested it
- Unit tests on small made-up files.
- The all-packs diff after a real build: 96 movement files and 34 interval files changed, nothing else.

## In the game
Never played. To test:
- Set 2x and watch a Stalfos, Wolfos or Lizalfos approach; these have real movement files.
- Set 0.75x and check nothing gets stuck.
- Expect Octoroks, Skulltulas and other animation-driven enemies to look unchanged. That is the known gap.
- Watch for enemies overshooting ledges or clipping at 2x.

## Notes
- This is the weakest of the three sliders because coverage is partial. A fuller version would have to scale animation rates, which I did not find a safe global handle for.
- Turn speed was left alone on purpose; faster turning changes how fair a fight feels more than faster running does.

## Seen in the game
Not tested (batch 1 stopped early, 18 Nov).


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 03):** Enemy speed
- **State:** Built, untested
- **What was seen in the game:** Nothing (batch 1 aborted before this)
- **Next step:** Batch 1
