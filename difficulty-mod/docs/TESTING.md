# Testing the difficulty settings in the game

Nothing here is needed to use the randomizer. It is how we test. The mod itself is data only and works wherever OOTRR works; Longshot is just the most convenient place to script a playtest.

## By hand

1. Build a seed with one or two difficulty settings on (see the feature file for what to turn on).
2. Put the mod in the game's mods folder and play to the place the feature file names.
3. Report the release candidate number, the settings that were on and exactly what happened.

Change as little as possible per test. With everything on, a crash tells you nothing.

## Scripted, through Longshot's control bridge

[Longshot](https://git.magicbean.dev/MagicBeanDev/Longshot) is a PC runtime for the game, built on your own PC from your own copy of the game. It has a local control bridge that lets a script press buttons, take screenshots, teleport and read the game's state. It is not part of this repo and none of it is copied here.

### Set up Longshot

- Install it with its launcher, or build it from a clone with its own script. Its README section "Build from source" has the commands; in short: `python -I nx2/make_game.py --nup <your game file> --out <game folder> --language en-US`. It needs a recent Python, its compiler tool set and about 60 GB free while building.
- A recent version is needed for the bridge to report health, position, music and save flags. Older versions report only the map, age and whether an event is playing; with those, read the hearts from a screenshot.

### The bridge

- It is off unless the game is started with the environment variable `HOS_CONTROL` set. It is local only and for the current user only.
- The client is `scripts/longshot_ctl.py` in the Longshot repo (standard library only). Longshot's `docs/control.md` documents every command.
- Useful commands: `launch <game folder> --user <save folder>`, `status`, `press A --frames 8`, `stick L 0 1 --frames 90`, `shot <file.png>`, `teleport <map or id>`, `age adult`, `watch --events oot --flags <names>`, `quit`.

### Two local additions to the bridge

We added two commands to our own Longshot source build. They are not in Longshot itself and not in this repo; they are kept as a patch beside the Longshot clone and re-applied after an update.

- `warp X Y Z`: moves Link to a position in the current map (same units as `status`'s position). Refused while an event plays or a map loads. Seen working for short and 100-unit jumps; a jump to a spot with no ground made Link fall out, lose a heart and respawn.
- `caption --test "..." --expect "..." --result "..."`: a small panel at the bottom left of the game window saying what is being tested, what is expected and the result. Every scripted test uses it so a person watching can follow. It does not appear in `shot` screenshots.

### Enemy positions: `enemies` (our source build only)

`longshot_ctl.py --json enemies` (or `enemies --no-scan`) lists every live enemy actor as JSON: `t_ms` (host clock, monotonic), `frame`, `link` [x,y,z], `life`, `max_life`, and `enemies`: `name`, `id` (the actor's address, stable per instance), `x`, `y`, `z`, `hp`, `max_hp`. The first call scans memory (a hitch); `--no-scan` re-reads the last scan's enemies and is fast enough to sample at 10 to 20 Hz. Speed is horizontal distance between two samples over the `t_ms` difference, in the units `status` uses per second. In Python, `lib.enemies()` and `lib.horizontal_speed(a, b)` (playtest `encounters\lib.py`) wrap it. `docs\enemy-speed.md` has a whole survey built on it (`<playtest folder>\speed\survey.py`).

Run the game windowed (`HOS_FULLSCREEN=0`) so it can be watched beside other windows.

### More test commands (our source build only, added 21 Nov)

All run at Link's next update, so they need the game in play with a save loaded (not on the pause menu or a load). Names are the game's own; `flag` and `take` accept the labels of `give` too. In Python the same calls are `longshot_ctl.py`'s `edit(b, verb, arg, n)` helpers; `--json` prints the raw reply.

- `flag get NAME` / `flag set NAME VALUE`: any saved flag by its name (`HavePlayedEvent.*`, `IsGet.*`, `PlayerStatus.*`) or `0x` and 8 hex digits of its hash. A bool takes 1/0/on/off, an int a number. Reads back after a write. An unknown name is refused with "not found". Seen: `IsGet.Weapon_Sword_Kokiri` 0 to 1 and back, `HavePlayedEvent.Dm_Kokiri_0000` read as 1, `PlayerStatus.Life` read as an int. It writes the saved value only: anything the game decided when the map loaded (a gate already built, a cutscene already started) does not change until the map loads again, so set flags before `teleport`.
- `take ITEM [N]` removes all of an item (or N of a counted one), `setcount ITEM N` sets a count; the item is a label or pouch name (`take "Deku Shield"`, `setcount Bombs 3`). The game applies it a moment later, so the reply reads the count back after 1.2 s. Seen: Deku Shield 1 to 0, Bombs 20 to 16 to 3. Songs are unlearned through their flag. Whether an equipped shield also leaves the equipment slot was not checked.
- `kill TARGET`, `damage TARGET N`, `enemyhp TARGET N`: TARGET is a number from `enemies`, a name (nearest match) or a `0x` id. `damage` subtracts N from the life in the game's own units (Stalfos 18), `kill` sets it to 0. Seen: Stalfos 18 to 13 to 10, then 0, and it left the list once Link was near. The enemy dies when its own update sees the life, so it can take a moment, and far from Link, not at all until the map loads it. Not damage as a hit: no knockback, no hurt sound, no drops.
- `camera off | aim YAW PITCH | look X Y Z | at EX EY EZ TX TY TZ | follow TARGET`: points the picture's camera (degrees; yaw 0 toward +z, 90 toward +x; pitch up positive). It changes only the picture (and so `shot`): the game's camera, Link and the enemies are not touched. Eye is the game's own camera eye except in `at`. Seen: aim up 50 degrees looked at the underside of the treehouse, look at a point turned to it, off gave the game's picture back. Cutscene cameras are not overridden.
- `time day|night|HOUR`: sets the world clock (day 12, night 0). The call goes through (log "cheats: requested 00:00") but no change of the sky was seen in Kokiri Forest or Hyrule Field within 40 s (picture brightness within 2 %), so treat it as NOT WORKING until someone finds why.
- `event`: whether an event is playing and the names of the last event request and timeline (`Opening_Introduction`, `Npc_Kokiri_Saria_Child_C0_N`, `Delayed_GameOver`, `DmF_SY_GameOver`) with how long ago they came. `event --moving S` also says which listed enemies moved in S seconds, so a script can tell whether enemies act during a scene. Seen: the opening, Saria's meeting triggered by a walk, the game over scene.
- `walk X Z` or `walk ENEMY`: Link walks there under his own control, steering the left stick toward the point from `status` every 12 frames (`--radius`, `--timeout`, `--verbose`). Areas, loading zones and enemy aggro act as in play. It learns which way the stick points from his own movement. It stops with a reason: arrived, blocked (4 steps without moving: a wall), timeout, map changed, loading, event. Seen: 8 and 20 units in Kokiri Forest ("arrived" within 4 to 10 s); a walk into a wall ended "blocked"; a walk past Saria's trigger started her scene (`event` named it). Not seen: a loading zone, or the "event" and "map changed" stops.
- `target ENEMY [--hold S] [--no-camera] [--lockon S]`: Link turns to face the enemy by the stick (his facing was read back: -70 to -81 against a bearing of -75, within 6 degrees) and the picture's camera follows it. Writing his facing directly does not work (the game rewrites it every frame), so this moves him a step or two. `--hold S` keeps turning him back for S seconds. `--lockon S` also holds ZL for S seconds (the game's own lock-on): no reticle was seen on a Dekubaba 12 units away in the field, so it is untested whether it works at all or our "No Z-targeting" setting blocks it. Use the plain mode for aiming.
- Dev aids: `yawscan snap` / `yawscan diff DEG`, `linkpeek OFFSET [N]`, `linkpoke OFFSET FLOAT...` (hex offsets inside Link's actor, up to 0x6e0) through `edit`; they showed that Link's facing lives in matrix copies at +0x2f8, +0x328 and +0x60c that the game rewrites from an angle kept elsewhere.

### Keep the real install safe

Never test in the folder you play in. Make a shadow game folder:

1. A new empty folder.
2. Directory junctions inside it for the large read-only folders of the real game folder (the game's `romfs` and the like).
3. Copies of the small top-level files and the executable.
4. Its own empty `mods` folder, and its own save folder passed with `--user`.

One randomizer mod at a time in the mods folder; mods load only at game start. Use a fresh save folder for each new game so starting settings apply.

### A typical scripted check

1. `ootrr generate <preset.json> "<seed>" <out.ootrr>` then `ootrr build <out.ootrr> <game romfs> <shadow mods folder> <name> --host longshot`. A preset is `{"id":"t","name":"t","tip":"t","order":99,"settings":{...}}`.
   - Put the randomizer's time savers in every test preset: `"skip_intro": true, "skip_cutscenes": "all", "faster_chests": true`. Leave them off only when the scene itself is under test.
2. Launch the shadow game, press A through the title until `status` says `play`.
3. Read the starting state, do the thing, read the state again, take a screenshot.
4. Quit the game before starting the next run.

### Rules

- One copy of the game at a time.
- "Loads" is not "works": say which you saw.
- Teleports less than about eight seconds apart are refused.
- A new mod needs a restart of the game.

## What has been tested so far

See `CHANGELOG.md` for the results per release candidate, and each feature file for its own status.
