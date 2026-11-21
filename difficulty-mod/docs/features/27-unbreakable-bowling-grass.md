# 27. Unbreakable Bombchu Bowling grass

The idea came from Sabriel. Thanks to Sabriel for the suggestion.

Setting `unbreakable_bowling_grass` (number 139), checkbox, label "Unbreakable Bombchu Bowling grass", on the Difficulty page in a new "Minigames" section, default off. Code: `randomizer/core/src/difficulty/bowling.rs`. Branch `feature/bowling-grass` (worktree `wt\bowling`). Played by hand (20 Nov, see the end).

## What was requested

"Makes the grass in the Bombchu Bowling minigame unbreakable, so you have to play the hard way." (user, 20 Nov)

## What the "grass" is

The alley is part of the Market (child) map: `Banc/Market/Market_Child_Static`. Its lanes hold `Obj_HrlCity_Shop_MovingObstacle_A_01` actors, 17 of them, each gated by a presence flag for one difficulty of the minigame (`MiniGame_BombchuBowling_Easy` 3, `_Normal` 5, `_Hard` 9). There is no actor in the alley with "grass", "kusa" or "bush" in its name; these obstacles are the only breakable lane objects, so they are taken to be what the request calls grass. Whether they look like grass in the game is not confirmed (the model is one flat plane). `Obj_HrlCity_Shop_MovingObstacle_B_01` (the two moving obstacles of the hard lane) has no AI graph, no death or destruction data and was never breakable. The fake cucco targets (`Obj_HrlCity_Shop_FakeCucco_*`) are the things you shoot at and are left alone. The actor is placed nowhere else in the game (scanned every Banc and AoC1 map).

## How it works

The obstacle is destroyed two ways, both found in its own pack `Pack/Actor/Obj_HrlCity_Shop_MovingObstacle_A_01.pack.zs`:

1. Its AI graph asks "was I damaged by an attacker within `HitDistance` 1.5" (`QueryGetLastAttackerInfoFromExternalDamageCalculator`, then a distance check) and, if so, runs `OneShotDeathRequestDeath`. That is a bombchu touching it, and any other damage that reaches it.
2. Its chemical file `Component/ChemicalParam/Obj_ContactOnly` has `RequestDeathIfBurned` and `RequestDeathIfExploded` both true, so fire (Din's Fire, fire arrows) and explosions (bombs, a bombchu going off) kill it.

The edit, applied to that one pack only:

- In the AI graph, the selector that reads the "was damaged" query is given the constant false in place of that link (`Ainb::force_bool`). The death branch can no longer run. Nothing else in the graph changes.
- In the chemical file, both death flags are set to false. Same size, nothing else changes.

Collision is untouched, so bombchus still have to go around the obstacles. The actor stays alive, so the minigame's reset (`RespawnTrigger`) still works. No new files, no size table growth (the pack gets slightly smaller).

Not covered, because it is unknown: any kill path that does not go through those two. The damage parameters file of this actor could not be parsed (a hash-keyed map), and the hit and impulse reaction data were not decoded. If the game still breaks them with something, the nearest further step is the actor's `LifeParam` invincibility type.

Risk: the vanilla layout was designed with breakable obstacles. The lanes have gaps (the hard lane rows are at x -43.5, -38.9, -34.4 and -31.9, with spacing in z of about 3 to 5 between obstacles), so a path exists on paper, but whether every difficulty stays winnable with the obstacles fixed has to be played.

## How it was tested

- Unit: bad graph and bad chemical file are refused; the chemical edit changes exactly the two flags (`cargo test -p ootrr-core difficulty`, 208 pass).
- Game files (`OOTRR_ROMFS` set): staging with the option on gives exactly one file, the obstacle pack, no bigger than before; only the AI graph and the chemical file inside it differ; the patched graph passes the layout audit, still has its death node, and no longer links the damage query to the gate; with the option off nothing is staged; the all-options test stages the pack.
- Settings: `cargo test -p ootrr-core settings` (106 pass), default settings string unchanged.
- Not played at all.

## How to test in the game

Not done yet; needs a person or a bridge session. Make a test mod from `<playtest folder>\bowling\ootrr.exe` with only this setting on, `skip_intro`, `skip_cutscenes`, `faster_chests`.

1. Teleport to the Market as a child, enter the Bombchu Bowling alley, pay and start a round on each difficulty (Easy, Normal, Hard).
2. Fire a bombchu straight at an obstacle: expect it to explode against it and the obstacle to stay.
3. Use bombs, sword, spin attack, Din's Fire (and fire arrows if available) on and near the obstacles: expect them all to stay. Note anything that still breaks.
4. Steer a bombchu around the obstacles to a target on each difficulty: confirm it can still be won.
5. Start a second round: obstacles are where they started.
6. With the setting off, an obstacle breaks as in the vanilla game.

## Seen in the game (20 Nov)
Passed by the user by hand in hands-on session 6: the lane obstacles stay standing. In rc 16. Menu text: "The grass in the Bombchu Bowling lanes can't be broken, so steer your Bombchus around it." Idea by Sabriel.


## Moved from STATUS.md (20 Nov): pointer

Passed by hand in session 6 (see feature 29's moved notes). - **Credit:** the unbreakable Bombchu Bowling grass was inspired by Sabriel. Thank them in the feature doc, the changelog and any pull request description that includes it.
- **New request (20 Nov): unbreakable Bombchu Bowling grass**, a new difficulty setting so the lane grass can't be cut or blown away. Being built on `feature/bowling-grass`.
