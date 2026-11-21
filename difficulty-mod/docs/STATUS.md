# Difficulty features: status

What is going on and what needs the user. Decisions: `DECISIONS.md`. Feature detail and results: `features\`. History: `CHANGELOG.md`, `HISTORY.md`.

## Needs your attention

- **Review and push rc 17 and rc 18:** `difficulty` is at `bd9da63` (rc 18) on top of `2a2fc47` (rc 17), both local only. Builds `builds_2026-11-21_rc17` and `builds_2026-11-21_rc18`. Include upstream 0.8.1.
- **Post rc 18 fixes:** `playtest/next` (`4aad32f`) holds the playtest fixes and the Mirror sparkle; they come as one more commit on `difficulty` once they are played.
- **Pull request #52:** the description `pr-drafts\difficulty-final.md` covers rc 16 to 18 plus today's results. Paste it after the next release candidate.
- **Screenshot wanted:** where on the Death Mountain Trail gates 15 and 16 should stand (above Goron City).
- **Question Q17:** is Volvagia in the field too harsh (about 3 hearts a hit every 6 s, only the hammer hurts her)? *Default: leave her deadly.*
- **Branches to delete (yours):** merged into `difficulty` or `playtest/next`: `feature/beamos-range`, `feature/boss-adds-start`, `feature/bowling-grass`, `feature/damage-options`, `feature/enemy-sizes`, `feature/fireball-storms`, `feature/fx-farore`, `feature/gate-sword`, `feature/mirror-return`, `feature/timed-switches`, `feature/zora-bottle-fixes`, `fix/playtest-gates`, `fix/playtest-misc`, `test/gates-all`, `test/handson`, `test/handson2`, tag `backup/next-pre-rc17`, and their worktrees in `wt\`. Keep `test/size-gallery` until sizes are fixed.

## Right now

| Running | Branch | State |
|---|---|---|
| Nothing | | |

## On `playtest/next`, not yet played by hand

- Storm fixes (own ring, doubled areas, Torch Slugs, desert off), Deku Stick burn-up, backup ladder, surprises 6%, night Stalchildren, Guay flocks, Dodongo swaps.
- Gates 9 (Stalfos), 14 (adult only), 20 (3 Stalfos), 21 and 22 (bridge only); gates 15 and 16 still to move.

## Queue

1. Volvagia: red rupee, then flee.
2. Enemy sizes: the real fix.
3. Next playtest (hands-on 10): the panel steps in `<playtest folder>\panel\steps.json`.
5. Enemy randomizer v3 load check (Bottom of the Well, Dodongo's Cavern, Ice Cavern, child Spirit Temple).

## Features

States: not built, built (never seen), scripted (seen by script), seen by hand, released. "Released" means in a numbered rc or merged upstream. Setting numbers in brackets.

| # | Feature | State | Left |
|---|---|---|---|
| 01 | [Enemy health](features/01-enemy-health.md) (110) | released | Peahat larva not read |
| 02 | [Enemy damage](features/02-enemy-damage.md) (111) | released | Grabs not covered |
| 03 | [Enemy speed](features/03-enemy-speed.md) (112) | built | Most enemies ignore it; no hand test |
| 04 | [No heart drops](features/04-no-heart-drops.md) (113) | released | None |
| 05 | [Limited ammo](features/05-no-ammo-drops.md) (115) | seen by hand (rc 17) | Boss help, Bongo Bongo's hands |
| 06 | [No gossip fairies](features/06-no-gossip-fairies.md) (114) | released | None |
| 07 | [Lose heart on death](features/07-lose-heart-on-death.md) (116) | released | Not kept across quit (accepted) |
| 08 | [No bottles in combat](features/08-no-bottles-in-combat.md) (117) | released (rc 18) | Notice wording |
| 09 | [Enemy regen](features/09-enemy-regen.md) (118) | released | None |
| 10 | [More enemies](features/10-more-enemies.md) (119) | released | Night Stalchildren doubled (`playtest/next` only); frame rate at 10x alone |
| 11 | [Boss minibosses](features/11-boss-minibosses.md) (120) | released (rc 16) | Barinade, Morpha, Bongo by hand |
| 12 | [Boss minions](features/12-boss-minions.md) (121) | released (rc 16) | As 11 |
| 13 | [No dupe items](features/13-no-dupe-items.md) (122) | released | Other duplicate cases |
| 14 | [Respawn sooner](features/14-enemies-respawn-sooner.md) (123) | built | Never tested |
| 15 | [No Z-targeting](features/15-no-z-targeting.md) (124) | released | None |
| 16 | [Fairies help enemies](features/16-fairies-help-enemies.md) (125) | released | Green heal seen by hand |
| 17 | [Enemy backup](features/17-enemies-call-for-backup.md) (126) | released; lesser-kind ladder on `playtest/next` only | Ladder not confirmed in the game |
| 18 | [Zora ambush](features/18-zora-ambush.md) (127, retired) | released (in more enemies) | Cave C failed by script |
| 19 | [Bean pitch](features/19-bean-sellers-pitch.md) (128, retired) | retired | None |
| 20 | [Gates](features/20-enemies-block-the-way.md) (129) | released (rc 16, 14-23 in rc 17); gate 23 seen by hand | Changes to 9, 14, 20, 21, 22 on `playtest/next` only; 15/16 to move up the Trail |
| 21 | [Volvagia in the field](features/21-volvagia-roams-hyrule-field.md) (143) | released (rc 17); Functional WIP | Freezes at death: red rupee then flee |
| 22 | [Special encounters](features/22-special-encounters.md) (130) | released (rc 17, 18); Beamos and Mad Scrub kill seen by hand | Octoroks by hand, Light Arrows scene |
| 23 | [Grass and pot surprises](features/23-grass-and-pot-surprises.md) (in 130) | released; 6% rate on `playtest/next` only | Fake pots never seen |
| 24 | [Cucco revenge](features/24-cucco-revenge.md) (131) | scripted | Control run with setting off |
| 25 | [Spawn combo](features/25-spawn-enemies-combo.md) (132) | released | None |
| 26 | [Enemy randomizer](features/26-enemy-randomizer.md) (138) | released (rc 17); v3 not loaded by hand | Dodongo swap fix on `playtest/next` only; judge movement and walls |
| 27 | [Bowling grass](features/27-unbreakable-bowling-grass.md) (139) | released (rc 16) | None |
| 28 | [Unequip gear](features/28-unequip-gear.md) | dropped | Replaced by feature 32 |
| 29 | [Fireball storms](features/29-fireball-storms.md) (140) | released (rc 16, 17); direction seen by hand | Doubled areas, slugs, desert off, wrong-spot fix: `playtest/next` only, unconfirmed |
| 30 | [Faster blocks](features/30-faster-blocks.md) (141) | released (upstream #55) | None |
| 31 | [Random fuses](features/31-random-fuses.md) (142) | released (rc 17); bombs and Bombchus seen by hand | Stick burn-up fix on `playtest/next` only; bomb flowers, Bowling |
| 32 | [Shield-eating mega Like Likes](features/32-shield-eating-like-likes.md) (144) | released (rc 17); seen by hand with the Mirror sparkle (sparkle on `playtest/next` only) | None |
| 33 | [Random enemy sizes](features/33-random-enemy-sizes.md) (145, 146) | released (rc 18); NOT WORKING | Real fix |
| 34 | [Shorter timers](features/34-shorter-timers.md) (147) | released (rc 18); scripted | Dodongo torch room at 50% by hand |
| 35 | [Damage taken](features/35-damage-options.md) (148) | released (rc 18); scripted | Pit and lava respawn, grabs |
| - | Skip area intros (137) | released (rc 17); Fire Temple entrance seen by hand | Other areas by hand |
| - | Skip first pickup text (136) | released (upstream #53) | None |
