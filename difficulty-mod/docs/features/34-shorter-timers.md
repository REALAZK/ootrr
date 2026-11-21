# 34. Shorter timers

## Requested (user, 20 Nov)

"A timed-switch multiplier": timed switches, timed torches and similar countdowns give the player less time. Third in the user's queue after enemy sizes.

## Setting

`shorter_timers`, number 147, "Shorter timers", choice: `off` (default), `three_quarters` (75%), `half` (50%). Menu: Difficulty page, section Hazards. 146 was the highest number used on other branches; 147 is next. Default settings string and seed unchanged.

Tip: "Timed floor switches, eye switches and torches that burn out give you 75% or 50% of the usual time. Covers the Deku Tree, Dodongo's Cavern, Fire Temple, Gerudo Training Ground and Ganon's Castle. Each puzzle keeps a minimum it can still be finished in. Experimental: played for a floor switch and one torch, eye switches not yet."

## Where the timers live (found in the game files)

The timers are not in the actor packs. They are per-placement values in the map files (`Banc/<Map>/<Map>_Static.bcett.byml.zs`, `Dynamic` block of the placed actor), read as instance parameters by the actor's AI graph (the step switch's graph multiplies `CountTime` by 30 frames per second and counts it). The pack holds only a default (`CountTime` 10, listed in `Obj_StepSwitch_A_02`'s blackboard table). Every placed copy found overrides it.

| Actor | Key | Placed values (seconds) | Maps |
|---|---|---|---|
| `Obj_StepSwitch_A_02` (timed floor switch) | `CountTime` | 20.2; 19.0, 12.95; 9999 (left alone) | Deku Tree; Fire Temple; Gerudo Training Ground |
| `Obj_EyeSwitch_A_05` (timed eye switch) | `LimitTime` | 10 (three); 80; 1 (two, left alone) | Dodongo's Cavern; Gerudo Training Ground; Water Temple |
| `Obj_CandleStick_TimeExtinguish_A_01` (torch that burns out) | `ExtinguishTime` | 8 (three); 22; no value on 5 more in Ganon's Castle room 2 and 3 in Dodongo's Cavern room 8 (the actor default of 15 applies) | Deku Tree room 7; Ganon's Castle; Dodongo's Cavern |

How it was found: a scan of every `Dynamic` key containing time, timer, interval, delay, limit or duration over all map files, then a look at each actor's graph.

The torch default: the pack of `Obj_CandleStick_TimeExtinguish_A_01` lists `ExtinguishTime` 15, `FireWeakTime` 5 and `ReignitionFrame` 90 in its blackboard table (`IsInstanceParam`, so a placement's `Dynamic` value overrides it). A placement without the key therefore burns for 15 s. Those placements are the ones that had been missed: the Ganon's Castle room 2 torches and the Dodongo's Cavern room 8 torches (and two in the Water Temple, outside the five maps).

## What the setting does

`difficulty/timers.rs` rewrites those values in the five Static map files (`stage`, a late step like the other map edits). New value = old value times the percentage, rounded to 0.01 s, never below the puzzle's floor, and never above the old value. Anything of 100 s or more (the 9999 "never expires" switch) is left alone. A torch placement with no `ExtinguishTime` is treated as 15 s and gets the key written when the scaled value is lower than 15. Nothing else in the map files changes (a game-file test compares everything but the timers).

## Floors (keeping each puzzle winnable)

A timer already at or below its floor is left as it is.

| Kind | Floor | Effect at 50% |
|---|---|---|
| Step switch `CountTime` | 10 s | Deku Tree 20.2 to 10.1; Fire Temple 19 to 10, 12.95 to 10 |
| Eye switch `LimitTime` | 6 s | Dodongo 10 to 6; Gerudo Training Ground 80 to 40; Water Temple 1 s unchanged |
| Torch `ExtinguishTime` | worked out per room (at least 4 s) | Deku Tree 8 stays 8; Dodongo's Cavern 15 to 12; Ganon's Castle room 2 15 to 13; Ganon's Castle room 8 22 to 11 |

The torch floor is worked out from the torches of one room (same `DefaultDungeonRoomIndex`): the shortest walk that visits them all (greedy nearest neighbour from every torch, straight lines) divided by 4.5 units per second, plus 1 s per torch for lighting, plus 1.5 s spare, rounded up to 0.5 s. 4.5 units per second is Link's measured walking speed with the `walk` command (about 19 units in 4.3 s in Dodongo's Cavern). That is what keeps a three-torch puzzle winnable: the three Deku Tree torches need about 8 s, so they are left as they are, and a lone torch needs about 4.5 s.

The step switch floor (the Deku Tree switch is 6 units from its door) and the eye switch floor are still guesses from the layout, not a walk of each puzzle. At 75% the Fire Temple 12.95 switch becomes 10, the Dodongo's Cavern torches 12 (75% of 15 is 11.25, under their 12 s floor) and the Ganon's Castle room 2 torches 13.

## Left alone, on purpose

- Zora's Domain torches (`ExtinguishTime` 45, 8 per map, `FireWeakTime` 3): the purpose is unclear and cannot be checked as winnable, so they keep their time.
- Graveyard `Obj_Graveyard_SwitchShutter_A_02` `LimitSec` (30 to 65): these are the gates of Dampe's race, a minigame with its own clock.
- Gerudo Training Ground's 9999 step switch, the Water Temple's 1 s eye switches, `FireWeakTime`, cutscene timers.
- **Forest Temple and Shadow Temple torches have no burn-out timer at all** (looked for in every place a timer can live):
  - `Obj_ForestTemple_SwitchCandleStick_A_01` to `_04` and `Obj_SwitchCandleStick`: the pack has no chemical (burning) data, only a graph that reads a `Burn` plug. The room logic (`ForestTemple_Level_Room_02_Prefab_7043` and the other prefabs) feeds that plug from saved game flags and a "light them in order" check (`HoldChallengeCurrentStepCheckOrder`), no counter. Once lit they stay lit.
  - `Obj_ShadowTemple_CandleStick_A_02` and `_05`: the graph only switches a light animation on a `BootUp` plug and a `CorrectCandleStick` check (Glass of Truth). The ordinary torch packs use the material `wood_fuel_noburnout_dry`, "no burn-out".
  - The delay and counter nodes found in the room logic of those temples (`HoldGeneralDelayBool` 384 frames in Forest Temple room 13, a `Common_FrameTimer` 180 frames and delays of 90 and 300 frames in Shadow Temple B1F) belong to the Poe fight, the falling floor and a room-clear flag, not to torches.
  - So there is nothing to scale there. The Ganon's Castle torches the user named are the `Obj_CandleStick_TimeExtinguish_A_01` ones, now scaled (room 8: 22 s, room 2: five torches at the default 15 s).
  - Ordinary torches (`Obj_CandleStick_A_01` and kin, `wood_fuel_noburnout`) never burn out, so there was nothing to change for them either. No shared pack was edited and none had to be cloned: the timer is a per-placement value.
- The Goron City and Hyrule Field `TimeExtinguish` torches and the two in the Water Temple: outside the five maps (the actor default of 15 s applies to them).

## Tested

- Unit tests in `difficulty/timers.rs`: scaling and floors, never grows, never under the floor, only listed actors and keys change, the 9999 switch is untouched, a torch without a timer gets the scaled default (and is left alone when the room floor is above it), the walk-based torch floor, choice maps to a percentage, setting off by default and read back from a settings string.
- Game-file tests (`OOTRR_ROMFS`): every timer in the five maps (and every torch default) stays between its floor and its old value at 50%, at least 17 timers found and 12 changed, nothing else in any of the five map files differs.
- `cargo test -p ootrr-core difficulty` (267) and `settings` (110) pass; default settings string and seed unchanged.
- Built mod checked by reading the timers back out of the built map files (50%): Deku Tree switch 10.1, Deku torches 8, Dodongo eye switches 6, Dodongo torches 12, Fire Temple switches 10, Ganon's Castle torch 11 and room 2 torches 13, Gerudo Training Ground eye switch 40.

### Played in the game (Longshot, 21 Nov, windowed, sound off, captions on)

- **Load test:** with the 50% mod the Deku Tree, Dodongo's Cavern, Fire Temple, Gerudo Training Ground and Ganon's Castle all loaded and Link was controllable (not black).
- **Deku Tree room 3 step switch (20.2 s), by screenshots at about 4 a second:** plate popped up at about 20 to 21 s with the setting off, about 10 to 11 s at 50% (expected 10.1 s), and about 15.5 to 16.4 s at 75% (expected 15.15 s).
- **Dodongo's Cavern room 8 torch (default 15 s, a placement with no `ExtinguishTime`), by window video at 20 frames a second:** Din's Fire cast beside the torch, flame counted in the picture. With the setting off the flame was there from 5.4 s to 20.6 s (about 15.2 s). At 50% it was there from 5.2 s to 17.6 s (about 12.4 s, expected 12). Same position, same cast, same 20 hearts.
- **Winnable at 50%:** with the switch pressed, `walk` from the switch to the shutter next to it (6 units) arrived in 4.3 s, well inside 10.1 s. Link's walking speed under `walk` is about 4.5 units a second (19.4 units in 4.3 s).

### Not played

- **Eye switches (Dodongo's Cavern, Gerudo Training Ground):** values checked in the built map only. A script could not shoot one: with the slingshot out (assigned and aimed in first person) the stick does not turn Link, the eye switches are not in the `enemies` list so `target` cannot aim at them, and the room 14 switch sits about 7 units above the spot the script could reach. In `docs\capture-playbook.md` for a person.
- **Walking the Dodongo torch route:** the `walk` from torch 1 towards torch 3 was blocked by the room's pillars after 8 units. The room floor (12 s) comes from straight-line distances at the measured walking speed, not from a real walk.
- **Fire Temple and Ganon's Castle timers:** values checked in the built map only. The Ganon's Castle torches could not be reached by script (a warp into the room drops Link through the floor: the room is not loaded).
- **Deku Tree torches:** already burning on arrival in the first test, so the burn-out was not seen there (the Dodongo torch above is the proof instead).
- **The stump platforms of the Deku Tree room 3:** they sank at about the same time with the setting off and at 50%, so they are not driven by the switch timer. What the switch's timer gates in that room is only the shutter beside it.
- **Dodongo's Cavern room 8 safety:** a Dodongo head breathes fire at the east side of the room, which took 6 to 9 hearts off Link in a few seconds at 3 hearts (twice he died). Tests used 20 hearts and stood west of the torches.

## How to test in the game

Build with `shorter_timers` at `half` (and `skip_intro`, `skip_cutscenes` "all", `faster_chests`). Teleport to the Deku Tree, warp to the switch (-17.5, 121.5, -4.9) from a spot just off it (-17.5, 121.5, -1.0), press, and watch the plate: it should pop up after about 10 s instead of 20 s. For torches, build with `starting_hearts` 20, teleport to Dodongo's Cavern, warp to (40.8, 100.5, -47), light the torch at (40.8, 100.0, -41.1) with Din's Fire and time the flame in a window video (15 s off, 12 s at 50%). For eye switches, hit the Dodongo's Cavern eye switches with a seed or arrow and time how long the door stays open (10 s off, 6 s at 50%). The Fire Temple switches should reset after 10 s at 50%.
