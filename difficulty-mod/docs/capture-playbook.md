# Capture playbook: things that need the user's hands

Scripted tests never wait for the user. When a step needs a person at the controls, the test marks it NOT TESTED, adds a row here and carries on. When the user is back, we run through this list in one session: the script warps and sets up, the caption says what to do, and the recorder captures the input for later replay.

Each row: what to do, where (map and position), the setup the script does first, and what we record or look for.

| # | For feature | What the user does | Where | Setup by script | We capture / look for | Status |
|---|---|---|---|---|---|---|
| 1 | Test tooling | Walk forward and swing the sword for about 10 seconds | Kokiri Forest, near Link's house | Sword equipped | First real recording, replay drift check | Open |
| 2 | Test tooling | Equip sword and shield from the pause menu | Anywhere | New game with sword and shield in starting items | A replayable "equip" routine | Open |
| 3 | Lose heart on death, others | Die to a Deku Baba as fast as possible | Kokiri Forest or inside the Deku Tree | Warp next to the Baba (measured: Deku Tree baba room, warp 5.5 100.2 -1.3, no stick input, re-warp after each hit; scripts can do it) | A replayable "die here" routine | Open |
| 4 | Enemies block the way | Kill the two withered Deku Babas by the Deku Tree path, then walk through | Kokiri Forest, Babas near x 38, wall at z about -19 | Closed forest, sword equipped, warp past Mido (NOT TESTED by heart-v4: killing the Babas needs sword play the script could not do) | Whether the wall opens only after both die; a replayable kill routine | Hands-on: user walked through at z -19.08 with Babas alive and battle music (see playtesthandsonRESULT.md): NO wall; feature being redesigned with a visible wall. gate.seq captured (truncated) |
| Song of Storms holes | Play Song of Storms at hole spots (HF: no cover holes such as Lost Woods hole 2 and others), record routine | map and x y z from <playtest folder>\holes\INDEX.md | ocarina + song via give, record | note buttons |
Hands-on: hole 9 is a Song of Storms hole (user saw it); ocarina and song given; entry not confirmed in the log |
| 5 | Lose heart on death | Get a bottled fairy (give lists only swords and shields), die once with it | Deku Tree baba room | Bottle with a fairy in the inventory | Fairy revival must cost no heart (NOT TESTED) | Open |
| 6 | Batch 1 (06, 08) | Play a song at a gossip stone; fill a bottle (fairy) and use it in and out of combat; die with the fairy | Kokiri Forest / Deku Tree baba room | Mod with those settings, ocarina and song, empty bottle via give | Whether the fairy spirit appears; head shake in combat; automatic fairy revival | Open |

| 19 Nov | Gate guards: kill and barrier | Stand on each guard mark (gates 1, 2, 11, 12), kill the guard with a sword or Din Fire, look toward the wall | Maps and marks in docs; test exe with HELD empty at <playtest folder>\gatetest | Shots before and after of the purple barrier | Open |

| 19 Nov | Boss minibosses and minions, Gohma | Walk through the Gohma door (Deku Tree boss door) and watch the arena before and after the door shuts | InsideTheDekuTree; arena Stalfos spot (16.5, 47.2, 74.0), Keese about (20, 50.5, 63-70); the saved warp spot (-15.77, 48.2, 65.73) is 32 units away | Mod with boss_minibosses and boss_minions (rc13 `ad` preset) | Are the Stalfos and 3 Keese hidden or absent before the door scene ends, and present as it ends? The actor list shows them from map load | Open |
| 19 Nov | Zora's Domain ambush | Climb into the hole and walk into the back of each side cave | ZoraDomain_Child hole room, centre (16.99, -18.26), floor y about 60 | Mod with zora_ambush | Hanging Skulltulas must appear only at the back of a cave; the actor list shows them from map load | Open |

| 19 Nov | More enemies, room-clear doors | At 5x walk into an ordinary clear-the-room dungeon room (Deku Tree or Dodongo's Cavern early rooms), kill only the original enemies, see whether the door/chest opens; then kill the added ones | Mod with more_enemies 500 | Mod built, Din's Fire given | Does the door open with added enemies alive; are added enemies in that room | Open |
| 19 Nov | More enemies, Kokiri safe zone | Enter Kokiri Forest by a real exit (from Lost Woods or Hyrule Field), die, Try Again, stand still 5 s | Kokiri Forest, 10x | Mod more_enemies 1000 | Damage taken, nearest enemy. Scripted attempt: a death always reloaded Hyrule Field's arrival point, the Deku Tree exit did not change it | Open |
| 19 Nov | libfpv (Longshot, not OOTRR): fly as Navi | With a real controller: L3+R3 hold + D-Right x3, fly the Kokiri Forest course (yellow ring with a face is next), crash into a tree, land with D-Down; listen to the wings at hover and full throttle | KokiriForest_Child, from outside Link's house (script warps Link to -28.6 111.5 9.5) | `<local path>` with `LONGSHOT_FPV=1`, windowed | Does the drone feel right (rates, tilt, throttle centre hover); are the gates easy to see; does the wing sound feel like a drone; any place she passes through a wall (thin fences, water?) | Open |

| 20 Nov | Gohma adds through the real door (agent 7) | Walk through the real Gohma door from the previous room, then look up at Gohma | InsideTheDekuTree. Warp spot -15.74 49.4 65.8 is already INSIDE the shut door; the intro trigger is at about x 6.7 y 47.0 z 67.6 and the look-up starts Dm_Deku_BossIn. Warps to the west side (-45 49.5 66) fall out of the map; -26.5 54.4 44 is the 3-Scrub puzzle room | Mod with boss_minibosses and boss_minions | Actor list identical before and after the intro (Stalfos 16.5 47.2 74.0, Keese at 20.0/15.7/24.3 y 50.5): are they really created only at the fight start, and active? Script cannot pitch the camera up | Open |

| 20 Nov | Enemy gates: authoring session (the gate editor) | Walk the maps and place gates: F8 on, H new gate, B barrier (face the guards' side), nudge with Numpad, T/Y kind, G guard, U undo; controller: hold L+R. Tell the agent which preview you could not judge (marker row vs the real barrier) | Any overworld map; start with Kokiri Forest outside Link's house. One gate per spot, as many as you like | Agent launches `<playtest folder>\gate-editor\game` windowed 1024x576, mod with `enemy_gates` on, gets to play, `gate on`, then hands off | `<run folder>\gates.json` (path on the panel); then `json_to_gates.py` for the rows. Full steps: `docs\gate-editor.md` | Open (tool built 20 Nov; scripted self-test NOT TESTED, run selftest.py first) |

## Round 2 leftovers (round2-1120, 20 Nov): need hands
- Grass and pot surprises (OOTRR_SURPRISE_CHANCE=20 at build, more_enemies 110): cut Kokiri Forest grass and break pots in Kokiri houses, Kakariko and Zora's Domain; note Skulltula/Poe/hand and normal drops. Setup: any mod with more_enemies above 1x. Capture: video of 20 cuts, enemy kinds seen.
- Flying pot in Zora's Domain at (36.2, 114.4, 8.6): walk up to it on foot (the script's warp to 2 units did nothing for 8 s). Capture whether it flies at Link.
- Enemies respawn sooner (doc 14): kill ordinary enemies (Zora River Tektites/Octoroks) and a room's enemies in a dungeon, leave the room by a door, return; compare against a build without the setting. Capture: counts before/after.
- Room-clear door with more_enemies 500: in a clear-the-room room, kill the original enemies, see whether the door opens while added ones live.
- Regen/fairy/backup flash (fx): fairy heal flash and backup flash need a fight with the enemy kept in frame; backup never triggered in three 45-60 s Stalfos fights with enemy_backup on.
- Jabu-Jabu at 10x, business scrub room (checks-1120): stand in the pool at -27.7 74.1 -42.5 and look around from several angles; check by eye that no added Octorok (positions in <playtest folder>\checks-1120\RESULTS.md section 3) is under the low ceiling or sunk in a wall; also rooms 13 (y 80.4) and 4 (y 97.4).
- Backup callers that did nothing in 45 s (Stalchild, Skulltula, Wallmaster, Armos, Biri): watch one for 2 to 3 minutes in its home map to separate 'calls later than 45 s' from 'never calls'; Gibdo cannot be spawned outside its home.
- Gates 1 and 6 with dungeon entrance shuffle: walk the Sacred Forest Meadow lane (wall at z -82.6) and the Kokiri to Deku Tree path and confirm no invisible wall.
- Skipping a boss intro by button (round3-1120): in Forest Temple (teleport ForestTemple, warp 0 68.5 -45, then 0 80 -62 starts Phantom Ganon's 42 s intro) press the game's own skip button on a real controller (the prompt shown on screen, if any) and check on video that the Stalfos and Keese still appear and attack from the fight's first second. The bridge's PLUS, B, A, X, Y, MINUS, ZR, ZL, L, R, START, D-pad, LS and RS (held and tapped) did not skip it. Read `HavePlayedEvent.Dm_ForestTemple_BossIn` afterwards. Then die, Try Again (returns to the dungeon entrance), walk in and skip again.
- Barinade and Gohma intro triggers (round3-1120): a warp into Barinade's arena (45 106 -112) does not start the intro; walk through the real boss door by hand and watch whether the 3 Biri and the Stalfos stay idle until the intro ends.
- Farore's Wind heal effect (fx-farore, 20 Nov, build `<playtest folder>\fx-farore\ootrr.exe`, regen and fairies on): fight a Stalfos and a Keese, let them regen and release a fairy; say whether the green effect is the Farore's Wind animation you meant, where it sits on the enemy and its size. Also cast Farore's Wind yourself and throw a Deku Nut: both must look as before. Capture: video of two heal ticks and one fairy heal. Steps in `<playtest folder>\fx-farore\PLAN.md` step 6.

## Unreleased tests leftovers (unreleased-tests, 20 Nov): need hands
- Random fuses and Bombchu Bowling: play the bowling alley (Castle Town Market, child) with a random_fuses build; say whether a short-fuse chu still reaches the pins and whether the game is winnable. Capture: 5 runs, how many reach the pins.
- Random fuses and bomb flowers: Goron City and Dodongo's Cavern flowers keep their normal time. Capture: a clip.
- Limited ammo, pots before a boss door: Dodongo's Cavern room 19 (pots at -16.5 95.4 -76.1 and -26 96.6 -90.6) and Forest Temple rooms 18, 25, 27, with an empty seed bag, bomb bag below full, full life and magic; say what drops. Leave and re-enter the map between tries. Also Kokiri Forest grass at -4.4 110.7 7.2.
- Enemy randomizer: look at swapped enemies from a normal camera for ones sunk in or stuck in walls, especially Fire Temple rooms 4 and 5 and Jabu rooms 3 and 5.
- Gate editor `gate linkscan`: crashes the game host at once on the 20 Nov Longshot build; the Longshot side needs a look before a hands-on try.

## Lake Octoroks and field Mad Scrubs (added 20 Nov)
- **What:** (Mad Scrub kill as child: seen by hand 21 Nov.) Look at the field Mad Scrubs as an adult.
- **Also:** swim past the lab bridges as child. Is the Octorok fire fair or too much? (In the script, 3-4 at once took 15 hearts while swimming.) Watch the frame rate there.
- **Setup:** build from <playtest folder>\lake-octoroks\ootrr.exe with more enemies above 1x.
- **Capture:** a clip of each, plus the FPS counter.

## Untested round 2 leftovers (21 Nov, <playtest folder>\untested2)
- **Light Arrows scene and Temple of Time Beamos** (the Beamos themselves were seen by hand on 21 Nov; they killed Link): adult with the Shadow and Spirit medallions, `HavePlayedEvent.Dm_TimeTemple_0065` set, mod from <playtest folder>\temple-beamos\ootrr.exe (more_enemies 200). Walk into the temple through its door and let Zelda's scene play. Do the ten Beamos in the pillar gaps act while it plays? Giving the medallions and warping into the temple did not start it. Capture a clip with the FPS counter.
- **Limited ammo, supply spots** (the limited ammo step was passed by hand on 21 Nov; boss help and Bongo Bongo's hands remain): with ammo 0 break the grass and pots by the Deku Tree slingshot eye switches and fight Gohma (seed drops from larvae and eggs expected); Bongo Bongo's hands with arrows 0. Also try trees and crates in the field. Mod from <playtest folder>\limited-ammo\ootrr.exe.
- **Adult Death Mountain Trail storm:** same mod, adult, past-summit flags set; stand in the areas near the first slope (the script never reached one) and say whether burning rocks fall.

## Shorter timers: eye switches and torch rooms (21 Nov, feature/timed-switches)
- **What (eye switch):** with the 50% build, shoot the Dodongo's Cavern room 14 eye switch (placed at about 80.5, 121.0, -6.6; the fire wall it puts out is at 73.75, 115.5, -6.6) with the slingshot and time how long the fire wall stays out. Expected about 6 s at 50% (10 s with the setting off). Then try to get across the wall before it comes back.
- **What (torches):** light the three Dodongo's Cavern room 8 torches with Din's Fire or a stick and walk them all in turn. At 50% they burn 12 s (15 s off). Is it still comfortable? The script measured only one torch (about 12 s at 50%, 15 s off) and could not walk between them (walk was blocked by the room's pillars).
- **Setup:** build from <playtest folder>\timers\ with `shorter_timers` half, starting_hearts 20. In room 8 a Dodongo head breathes fire at the east side: keep to the west of the torches. A script cannot aim the slingshot at the switch (the stick does not turn Link while aiming).
- **Capture:** a clip with the FPS counter, and a note whether the eye switch time felt fair.

## Open after the 21 Nov hands-on (needs a person)
- **Fireball storms:** walk each of the five doubled field areas, the castle garden and the Wasteland (adult only) and confirm the rocks land in your own ring, that the Torch Slug stands in the middle, and that nothing drops Link through the floor. The adult Death Mountain Trail storm and the Wasteland direction are still not looked at.
- **Gates 20, 9, 14, 21, 22 after the changes:** gate 20 (3 Stalfos, day and night), gate 9 (Stalfos), gate 14 (adult only), the lake lab bridge (walk the whole area, look for falling through the floor), gate 15 after the guard dies (no warp).
- **Deku Stick random fuse:** light a stick and let it burn out; it must be used up.
- **Backup ladder:** a Stalfos should call a Wolfos, a Wolfos a Lizalfos.
- **Guay visible** near (97.7, 148, 66) in child Hyrule Field; **night Stalchildren** doubled; **Dodongo's Cavern swaps** (seed handson10).
