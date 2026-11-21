# Difficulty settings: changelog

One entry per release candidate (rc), newest first. Each rc is one commit on the `difficulty` branch. "Played" means seen in the running game, by hand or by a script through Longshot's control bridge (see [TESTING.md](TESTING.md)). Feature details are in [features/](features/).

## rc 19

On top of upstream 0.8.2 (0fa64f4) and rc 18. Everything on `playtest/next`: the fixes from the hands-on session of 21 Nov and the Mirror Shield sparkle. No new settings. The fixes made after the second hands-on session (backup ladder order, Lon Lon adult gate, tree enemies, Stalchild waves, the surprise check in grass and others) are **not** in this rc: they come as small separate pull requests.

**Played by hand on 21 Nov and working**
- Gate 23 (Dark Link) and gate 17 (Graveyard).
- Mega Like Likes: the shield swallow, their regen, and the green Mirror Shield sparkle when it comes back.
- Temple of Time Beamos (they reach the aisle and killed Link).
- Fireball storms: the rocks fly in from the right direction, they land in their own ring (the wrong-spot bug is gone), the areas are twice as big, and a Torch Slug marks each area.
- Random fuses: bombs, bombchus and Deku Sticks (the stick now burns up).
- Killing a Mad Scrub; limited ammo; Guay visible in child Hyrule Field; enemy randomizer swaps in Dodongo's Cavern; the Morpha room frame rate; unbreakable Bombchu Bowling grass.

**Fixes in this rc** (built after the first hands-on): Bombchu fuses are now only longer than the game's (3.5 to 5.25 s), bombs and sticks unchanged, so Bombchu Bowling is beatable again (played by hand: beatable, just hard); gates 9 (Stalfos guard), 14 (adult only, bigger wall), 20 (three Stalfos), 21 and 22 (Like Like guard, wall across the bridge only); Stalfos and Wolfos call a lesser kind for backup; pots and grass surprises at 6%; Guay no longer hidden; Baby Dodongos can be swapped; starting kit gains the Master Sword and Biggoron's Sword.

**Not played by hand:** the Desert Colossus storm is switched off (Link fell through the floor), night Stalchild doubling (built, no visible effect), the other gates' real sword kills, gate 20 at night.

**Known problems**
- **Random enemy sizes (145, 146) do not work.** The game ignores the placement Scale. Labelled "not working yet" on the settings page.
- **Volvagia in the field (143) is Functional WIP.** She freezes at death and does not regen. The wanted behaviour (red rupee, then she flees) is not built.
- **Surprises did not show in field grass** (about 30 to 60 tufts cut, none seen).
- **Gates 14, 19, 21 and 22 are easy to walk round.**
- **Gates 15 and 16 are in the wrong place** (they block Goron City, not the climb): to be removed.
- **The enemy backup ladder** is the user's rough order, not a derived difficulty order.
- **The night Stalchild doubling has no effect.**
- **The desert storm is switched off.**
- Mega Like Likes keep calling small Floormasters (they should call none).

**Settings renumbered:** upstream 0.8.2 took 137, 138 and 142 (Nocturne and Prelude triggers, debug names). Skip area intros is now 149 (was 137), Enemy randomizer 150 (was 138), Random fuses 151 (was 142). Settings strings made with rc 17 or 18 and these three settings will not carry them over.

**Build notes:** upstream 0.8.2 merged into `playtest/next` (conflicts in `slice/edit.rs` and `slice/support.rs`, resolved by keeping both sides). Tests pass with the game files: difficulty 296, whole workspace 2,215.

## rc 18

On top of upstream 0.8.1 (3ee77a6) and rc 17. Adds five features that passed scripted tests since rc 17. Branch `playtest/next`. Five new settings (145 to 148, with 145 and 146 for sizes) are all off by default. **Nothing in this entry has been played by hand.** "Script" means a scripted run in the game through Longshot's control bridge.

- **Temple of Time Beamos reach the aisle** ([features/22](features/22-special-encounters.md)). The ten Beamos are now a cloned pack with longer sight at 1.6x size, so they reach the centre of the aisle. Script: a run down the centre costs 0 to 1.5 hearts. Not played: by hand, the Light Arrows scene (still not checked).
- **Zora's Domain and bottle fixes** ([features/18](features/18-zora-ambush.md), [features/08](features/08-no-bottles-in-combat.md)). A unit test checks that each Zora cave dropper lies on the way out (cave C was never broken). A bottle used in combat now shows the game's own refusal notice; its wording names the Business Scrub. Script: both seen. Not played: by hand.
- **Random enemy sizes** (new settings 145 and 146, [features/33](features/33-random-enemy-sizes.md)). **Not working:** the 21 Nov hands-on size gallery showed no change in size, so the game ignores the placement Scale. The settings page labels them "not working yet". Wild was played by script in 5 maps, but that only showed the actors were placed. Not played: Mild, enemies stuck in walls, the mix with other settings. It adds about 27 MB to the build when health follows size.
- **Shorter timers** (new setting 147, [features/34](features/34-shorter-timers.md)). Timed switches, eye switches and torches give less time. Script: switch and torch times measured. Not proven: winning the Dodongo's Cavern torch room at 50%, and the eye switches.
- **Damage taken** (new setting 148, [features/35](features/35-damage-options.md)). Normal, Double, Quadruple or One hit kills. Script: played, including a fairy revive. Not covered: pit and lava respawn, several grabs.

**Build notes:** merged onto rc 17 by hand (the timers and damage branches conflicted in `difficulty.rs` and the menu; resolved by keeping both). Setting numbers 136 to 148 are unique. Tests pass (difficulty 292, settings 113, whole workspace 2,173 with the game files).

## rc 17

On top of upstream 0.8.1 (3ee77a6) and rc 16. Adds the features that passed scripted tests since rc 16. Branch `playtest/next`. Four new settings (138, 142, 143, 144) and the area-intros one (137); all off by default. **Nothing in this entry has been played by hand yet**, except where it says so. "Script" means a scripted run in the game through Longshot's control bridge.

- **Gates 14 to 23 and gate 9** ([features/20](features/20-enemies-block-the-way.md)). Gates at the crater pocket, the Death Mountain Trail (child and adult), the Graveyard (child and adult), the Hyrule Field neck, Lon Lon Ranch, Lake Hylia's bridge (child and adult) and Ganon's Castle (Dark Link), from the marks you drew on the map, plus a Wolfos at the Lost Woods gate. The guards must stand on the game's own navigation floor (a unit test checks every one). Script: the barrier holds and drops when the guards die at 9, 14, 15, 16 (after moving the Dodongo that fell through the floor), 17, 18, 19, 21, 22 and 23; gate 15 is absent without the sword and appears at once when it is given. Gate 20 (Lon Lon Ranch) guards are cloned Leevers (a giant with 8x life and five small ones): by script, real sword hits killed all six and the barrier dropped. Not played: a hand session; real sword kills at any other gate (the script killed those guards with its own damage command); gate 20 at night and the vanilla Wasteland Leevers; arrival at the Graveyard by the Nocturne; gate 22 with the lake drained; the climb out of the crater pocket (not possible for a child, same as the game's pocket).
- **Volvagia roams adult Hyrule Field** (new setting 143, [features/21](features/21-volvagia-roams-hyrule-field.md)). By script: she appears on a new save, wakes when Link comes close, flies, breathes fire and burns Link, dies to Megaton Hammer hits (about 7 minutes of swings), leaves no body within about 10 seconds, comes back after leaving and re-entering the field, and is gone once `HavePlayedEvent.Dm_FireTemple_BossOut` is set (it was set by hand with the control bridge to test this). Her health follows "Enemy health" (30 of 30 at 50%). The child field is unchanged. Not played: the real Fire Temple fight won with the setting on (the flag was only set by the test tool), her damage under "Enemy damage", a hand fight. Q17 (is she too harsh: about 3 hearts a hit every 6 s, only the hammer hurts her) is open.
- **Shield-eating mega Like Likes** (new setting 144, [features/32](features/32-shield-eating-like-likes.md)). Three bigger Like Likes (1.6x size, 3x life) in the Deku Tree meadow of Kokiri Forest, child and adult. When one swallows Link it takes a shield (Mirror, then Hylian, then Deku, one per swallow); killing one gives the Hylian and Deku shields back as pickups, and the Mirror goes back to the pouch silently. By script: both maps load, a swallow removes the shield (the pause menu showed the empty slot), a kill brings it back with no duplicate, they return after a reload, dying in the belly and Try Again keeps the saved shields. Not played: a fight by hand, how 3x life feels, the Mirror as a child, a get-item scene for the returned Mirror (not built).
- **Special encounters: Lake Hylia Octoroks, Mad Scrubs and Temple of Time Beamos** (part of "More enemies" above 1x, [features/22](features/22-special-encounters.md)). Twelve Octoroks in the open water of Lake Hylia, ten Mad Scrubs in the trees south west of Lon Lon Ranch, ten Beamos between the pillars of the adult Temple of Time. By script: Octoroks surface, spit rocks and die to slingshot seeds (3 or 4 at once took 15 hearts from a swimming Link; frame rate 35 to 50, dips to 22 to 29); the Mad Scrubs pop up and hit Link with nuts and none was seen inside a tree. Not played: killing a Mad Scrub (the script could not), the adult field's scrubs, the full lake. **The Beamos were not played at all**: they are built to appear after the first adult scenes in the temple and nobody has seen them. Not checked: whether they act during the Light Arrows scene.
- **Fireball storms: more spots, rocks from the volcano** (setting 140). New storm spots (Desert Colossus flats, Hyrule Field south east and near the Market, the Wasteland dunes, the Death Mountain Trail on every visit), and the rocks now arc in from the direction of Death Mountain. By script: all load at 58 to 61 frames a second and rocks and fire are seen at each spot, standing, dodging and on a landing spot. **The direction of the rocks is not seen** (the script cannot judge it). The child Wasteland is black in the game even without the mod (a teleport quirk), so it was not tested. The Trail walk could not be told apart from the game's own boulders.
- **Random fuses** (new setting 142, [features/31](features/31-random-fuses.md)). Bombs, Bombchus and lit Deku Sticks get a random fuse (bombs 1 to 4 s, Bombchus 2 to 3.5 s, sticks 10 to 24 s). By script: 20 bombs and 10 Bombchus all exploded, none vanished; sticks burned 11 to 19 s on an earlier build with the 5 to 24 s range. Not played: by hand, bomb flowers, Bombchu Bowling (its Bombchus share the actor, so they get short fuses too).
- **Skip area intros** (new setting 137). The first-visit intro scenes of overworld areas and dungeon entrances that only show the place are skipped, and each area is marked visited. Played by hand at the Fire Temple entrance (session 7: works); the other areas were not looked at one by one.
- **Limited ammo, round 2** ("No ammo drops" renamed "Limited ammo"; boss fights keep their ammo, puzzle supply kept). Unit tests only. **Not played in the game.** The Deku Tree, Dodongo's Cavern and the other eye-switch rooms were not walked.
- **Enemy randomizer v3** (new setting 138). Also swaps enemies that carry `Presence` or `IsPhysicsStable`. v2 maps loaded in the game; **v3 was not loaded or played.** Still to load: Bottom of the Well, Dodongo's Cavern, Ice Cavern, the child Spirit Temple. How enemies move and whether they stay inside walls is not judged.

**Not in this rc:** faster blocks (own upstream pull request, setting 141), skip first pickup text (#53).

**Build notes:** tests pass (difficulty 259, settings 110, whole workspace 2,140 with the game files). The branches were merged onto rc 16 and upstream 0.8.1 by hand: the old playtest branch had been left behind at rc 15 work. The test lists now also count Volvagia, mega Like Likes, fuses and the randomizer.

## rc 16

On top of upstream 0.8.0 (06cc58f). Adds the features that were tested since rc 15. Branch `release/rc16`.

**Already on `difficulty` since rc 15** (in pull request #52 or just after it):
- Shorter, plainer tooltips and notes; the "untested" notes removed where things were tested. The user reviewed the settings page.
- Bean Seller's pitch retired (setting 128 stays retired and off). Zora's Domain ambush is no longer its own setting (127 retired): it is part of more enemies, once you have a sword.
- Child gates wait for the Kokiri Sword. Played by hand (session 4): no gates without the sword, gates with it, a guard kill drops the barrier.
- Gates and the Zora ambush no longer vanish when you walk up close: the trigger stays held once Link has entered. Found by hand in session 3, fix played by hand in session 4.

**New in this rc:**
- **Boss adds arrive with the boss introduction.** The miniboss and minions of "Bosses spawn minibosses" and "Bosses spawn minions" appear when the fight starts (Gohma: after you look up at her) and leave when the boss is beaten. By hand: Phantom Ganon and Gohma. By script: Phantom Ganon and Twinrova (the adds wait through the intro and attack within 1 s of the fight); Try Again goes back to the dungeon entrance and the adds come back once. The Shadow Temple loaded to a black screen in the scripted test; Bongo Bongo's Keese are now kept where they were (in the room from the start, not from the intro). The temple has not been loaded with that fix. Not played: Barinade, Volvagia, Morpha, Bongo Bongo. See [features/11](features/11-boss-minibosses.md) and [features/12](features/12-boss-minions.md).
- **Green heal effect.** When an enemy regenerates or a fairy heals it, it shows Farore's Wind's green effect (ring, column and orb), sized to the enemy. A backup call shows a white flash. By hand (session 6): the game's own effects still work, and the full green heal shows on enemies, including the full fairy heal. The backup flash was not looked at on its own. See [features/09](features/09-enemy-regen.md).
- **Fireball storms** (new setting 140, Hazards section, off by default). Burning rocks rain down, as on Death Mountain, while you stand in three quiet stretches of Hyrule Field or the child Hyrule Castle garden. By hand: the middle road and the castle garden (the guards behave). Not played: the other two field stretches, the adult field. Frame rate in child Hyrule Field was about 30 to 37 with 5x more enemies plus the storm. See [features/29](features/29-fireball-storms.md).
- **Unbreakable Bombchu Bowling grass** (new setting 139, Minigames section, off by default). The lane obstacles can't be broken. Idea by Sabriel: thank you. Passed by hand. See [features/27](features/27-unbreakable-bowling-grass.md).
- **Harmless scrubs never call for backup** (business, stage and leaf scrubs, the Deku Scrub brothers). By script: a business scrub called nobody in 40 s. The fighting Deku Scrub still calls.
- **Gates with dungeon entrance shuffle.** Gates stay on, except the three with a dungeon door beyond the wall (1, 2, 6). By script: the Lost Woods gate is there and holds Link, the Death Mountain Trail gate is not. Not tested: gates 1 and 6 absent.
- **Jabu-Jabu water spawns** only go in open pools near a shore. By script: 10 Octoroks at water level in the scrub room pool, none seen in a wall. Not checked by eye: ceilings.

**Not in this rc** (not tested enough, or their own pull request): enemy randomizer, limited ammo, skip area intros and first pickup text (#53), Volvagia in Hyrule Field, faster blocks (own pull request, setting 141), random fuses.

## rc 15

Changes from the first hands-on session with rc 14 (19 Nov, at 10x). A second hands-on session the same day played this build: "everything I tested was perfect", with the spawn buttons and the extra enemies named; it was not recorded which other items below were looked at, so only those marked "by hand" count as seen.

- **Spawn enemies: new buttons.** Hold both stick clicks, then D-pad Down for a random enemy and D-pad Up for the same one again. Works by hand on a real controller. The old L + R never worked by hand: both open the game's item wheel.
- **Special encounters is no longer its own setting.** Any more-enemies value above 1x includes what it did (Dead Hand in two holes, the awake Iron Knuckle, the Lake Hylia Like Likes, grass and pot surprises, the Guays over child Hyrule Field). Old settings strings still load.
- **More enemies, from play at 10x** ("about the right number" in child Hyrule Field; by hand):
  - Peaceful places get no added enemies: child Kokiri Forest, Kakariko Village, child Market, child Hyrule Castle, Gerudo Fortress, Zora's Domain, Goron City; enemies keep away from Lon Lon Ranch and the Temple of Time.
  - Iron Knuckles: at most two in each Hyrule Field, one in any other map.
  - Large enemies (Like Like, Wolfos, Stalfos, Dinolfos, ReDead, Moblins and the like) stay well apart; small ones may cluster.
  - Keese of every kind are only placed on a perch (a tree outdoors, a wall indoors), since they sit still until approached.
  - Walkers only go on ground Link can reach; no more enemies standing in the lava of Dodongo's Cavern. Fire Keese were added there.
  - No added Dodongos or Baby Dodongos in Dodongo's Cavern (their explosion can clear a bombable wall).
  - Tougher enemies are steered toward landmarks such as trees and holes ("very good stuff" in the first session).
- **Enemies call for backup: a lesser kind.** Seen by hand in rc 14: Stalfos and Wolfos calling a second of their own kind ("nice", but too much), and a Lizalfos' helper falling into lava. Now a caller never calls its own kind (Stalfos a Stalchild, Wolfos two Keese, White Wolfos a Wolfos, Lizalfos and Dodongos a Keese), and the helper appears where the caller stands. Not played yet in this form.
- **Gate on Death Mountain Trail with the Dodongo guard: "perfect", by hand.**
- **Bosses spawn minibosses and minions (Gohma):** warped into the room by the test tool, the Stalfos and Keese never appeared (they wait for the door scene, which a warp skips). Entering through the door is still not played.
- **Not done:** an on-screen effect when enemies regenerate, are healed by a fairy or call for backup. The node that emits an effect is known; no effect that every enemy has loaded was found yet.

## rc 14

Results come from scripted runs in the game (19 Nov). Nothing in this entry has been played by hand yet.

- **Merged upstream 0.7.3.**
- **More enemies is now a slider from 1x (off) to 10x.** The value is a ceiling, not a target: each area gets as many extra enemies as sensibly fit, up to that multiple. Up to 2x adds more of an area's own enemies; above that come kinds that suit the ground (flyers over open ground, Octoroks in water, wall and ceiling kinds in dungeons, Leevers only on sand) and, from about 5x, kinds that never live there and the odd miniboss (Stalfos, Dinolfos pairs, an Iron Knuckle, White Wolfos) on flat open ground. Spots come from the game's own navigation data, with a slope limit per kind. See [features/10](features/10-more-enemies.md).
  - Safe zones: nothing is added near any place Link can arrive (entrances, exits, hole exits, warp pads, restart points), and only a couple of slow enemies in a wider ring, always leaving a way out.
  - Limits: 240 enemies per map, 120 within 60 units. At 10x that is about 1,800 added enemies over 39 maps; 9 of 172 areas reach the full 10x.
  - Seen: 37 of 38 edited maps load at 10x (the 38th is black without the mod too and is no longer edited). Kokiri Forest, both Hyrule Fields, Dodongo's Cavern and the Fire Temple hold 58 to 60 frames a second at 10x. Dying and retrying in Hyrule Field and Dodongo's Cavern: no damage in the first five seconds.
  - Seen going wrong: Freezards fell through floors, so they are not placed. In 30 seconds at 10x one Tektite fell and a few enemies dropped off the list.
  - Not tested: rooms whose door opens when the enemies are beaten; safe zones in Kokiri Forest and other dungeons; how it plays.
- **Enemies call for backup: "bring a friend".** The helper is now the same kind as the caller or a peer (a Stalfos calls a Stalfos, a Wolfos a Wolfos, a Deku Baba a Mad Scrub), once, 10 to 25 seconds into the fight, and a called helper never calls. The table is in [enemy-table.md](enemy-table.md), option 3.
  - Seen (32 callers spawned in Hyrule Field, no crash): working for Stalfos, Wolfos, Lizalfos, Dodongo, club Moblin, both Tektites, Deku Baba, Mad Scrub, Gohma larva, Floormaster, Like Like, Poe, green and red Bubble, Torch Slug, Octorok, Shell Blade. Keese and ReDeads call nobody. No helper called in turn.
  - Seen short: the spear Moblin and the Bari brought one helper, not two.
  - Nothing came in 75 seconds for Stalchild, Freezard, Skulltula, Wallmaster, Armos, Spike, Tailpasaran, Biri. Several of these do not fight normally when dropped in an open field, so this may be the test and not the feature.
  - Not tested: weak Stalfos, Gibdo, white Bubble, the water and Jabu-Jabu kinds.
- **Everything on together** (more enemies at 5x): Kokiri Forest, Hyrule Field, Dodongo's Cavern and the Deku Tree load, and three minutes in Hyrule Field ran without a crash at 46 to 56 frames a second.
- Testing tools: the `enemies` command now scans the whole heap (it missed most enemies before, which looked like a mod fault); frame rates are only measured with one copy of the game running.

## rc 13

Results come from scripted runs in the game (19 Nov) unless they say otherwise.

- **Merged upstream 0.7.1** (the title logo option). All upstream tests and ours pass.
- **Enemy regen, new timing, seen working:** the first heal comes 15 seconds after the last hit (15.0 and 15.2 measured), then 1 health about every 5 seconds.
- **Enemies call for backup, reworked, seen working.** Each kind of enemy calls its own kind of helper, after a random 10 to 25 seconds, once. Seen: a Stalfos called one Keese (15.9 and 24.5 seconds), a Lizalfos one Baby Dodongo (24.4 and 25.4 seconds); Keese and a ReDead called nobody for 90 seconds; no crash. The full table is in [enemy-table.md](enemy-table.md). Peahat larvae and Leevers are no longer used as helpers. See [features/17](features/17-enemies-call-for-backup.md).
- **Gates: eleven of twelve are on.** Played by hand on 19 Nov: gates 1, 2, 3, 4, 5, 6, 9, 11, 12 and 13. A gate now only appears once the player is well past its line on the guard's side. The Death Mountain Trail gate has a Dodongo as its guard: the map loads and he stands at his spot (script); the fight is not played. Gate 7 (Lost Woods east passage) stays held back, never tried.
- **Spawn enemies with L + R + D-pad** works by scripted presses in this build with every other difficulty setting on (six presses, five to ten enemies). By hand on a real controller it did nothing in the previous build; not retried by hand.
- **Special encounters:** 24 Guays over child Hyrule Field in six flocks. Seen: eight in range, flying, by day and by night (they were meant to be night-only; they are not). The Like Like near the lakeside laboratory is removed. Grass that always gave a bug joins the surprise pool: confirmed by hand.
- **Bosses spawn minibosses and minions:** in Gohma's room the Stalfos and Keese are now created when the door shuts, not when the room loads. The dungeon loads; the door scene is not played (a script cannot walk through the door yet). Other boss rooms are unchanged.
- **Zora's Domain ambush:** trigger adjusted. The map loads with the three Skulltulas present; the walk through the cave is not played.
- **Fairies help enemies:** confirmed by hand. The circling effect on the healed enemy is not possible with data edits.
- **Enemy speed, measured for the first time.** A script spawns each enemy beside a standing Link and tracks its position. At 2x only the Lizalfos comes close to double (1.72); Stalfos, Wolfos, Dinolfos, Freezard and Guay gain 30 to 50 percent; Tektites, Peahats, Poes, Bubbles, Shell Blades and Floormasters do not change. The table is in [enemy-speed.md](enemy-speed.md).
- **Not in this rc:** the "more enemies" slider rework (up to 10x, placed from the maps' navigation data). It is built and loads 37 of 38 maps at 10x, but many planned enemies do not appear in five maps and the frame rate halves at 10x. See [features/10](features/10-more-enemies.md).
- Testing tools: Longshot updated to the latest upstream with our tools re-applied; a new `enemies` command lists live enemies with positions and health; two copies of the game can run side by side for tests. See [TESTING.md](TESTING.md).

## rc 12

Results here come from scripted runs in the game unless they say otherwise. Enemy health can now be read during tests, which is what made several of these checks possible.

- **Enemies call for backup: fixed and seen working.** It crashed the game seconds after an enemy appeared; the cause was how its two flags were stored. Now: no crash through 90 seconds beside two Lizalfos, and each called one Peahat larva, once, at 10 and 16 seconds. The wait is about 12 seconds, not the 25 intended. See [features/17](features/17-enemies-call-for-backup.md).
- **Spawn enemies with L + R + D-pad: fixed and seen working** with scripted button presses: 12 presses gave 12 enemies in front of Link, D-pad Up repeated the last kind, and further presses gave nothing (12 alive at once). Before, only the first press ever worked. Not yet tried on a real controller. See [features/25](features/25-spawn-enemies-combo.md).
- **Enemy regen: seen working.** A Stalfos hit from 18 to 11 waited four seconds, then healed 1 about every two seconds back to 18; with the setting off it stayed at 12 for a minute.
- **Enemy health: seen working by the numbers.** At 2x a Stalfos has 36 and a Lizalfos 18.
- **Like Likes at Lake Hylia: present.** The map loads and they are at their spots.
- **"Red gate guards" was not a bug.** The big Deku Baba on the Deku Tree path has a red head with every difficulty setting off.
- **Gates:** unchanged from rc 11 (six on). The six held back all load their maps and show their guards; killing the guards and seeing the barrier drop is still to be done by hand.

## rc 11

- **Merged upstream** (32 commits; the repository layout changed: `crates/` is now `randomizer/`). All upstream tests and ours pass.
- **Surprises from grass and pots** are now 1 in 50 for each kind, and grass releases the wall Skulltula (on the ground) or a little Floormaster hand; pots a Poe or a little hand. Played by hand at a raised test rate: "perfect". The clumps of grass that always give a bug (for example round the Dead Hand hole in Hyrule Field) now join the pool; not played yet.
- **Cuccos take revenge at once** now covers Kakariko's cuccos too. Played: one hit brings the swarm.
- **Dead Hand** in both holes: placed further back and his rise distance shortened. Not played since; before this he still rose near the entrance.
- **Gates:** seven were in rc 10; six are in this rc. The Lost Woods room was changed to one Stalfos for both barriers and is held back until that is seen working, along with the five held back before (which got their fixes: own guards for the meadow and the trail, two Stalfos in Gerudo Valley, a rule that a gate only triggers from the guard's side, the Hyrule Field barrier half a unit lower). None of the fixes is played yet.
- **Enemies call for backup crashes the game** (found by a scripted test, twice). Leave it off. Regen alone and fairies alone did not crash.
- Docs: the status board, the capture playbook and the code review are now in this folder too.

## rc 10

Played by hand unless it says otherwise. Things known not to work are listed in [TODO.md](TODO.md).

- **Enemies block the way, rebuilt.** A gate is the game's own purple barrier. Nothing is there until you come near its guard; then the barrier rises and the guard appears with it; it drops when the guards are dead. Seven gates are in: three on Zora's River (Lizalfos; the upper one has two), the Deku Tree path (the withered Deku Babas, closed forest only), the small room in the Lost Woods (two barriers), and the road to Gerudo Valley in adult Hyrule Field (an Iron Knuckle). All seven were seen working. Five more are built but held back until they work (see TODO). See [features/20](features/20-enemies-block-the-way.md).
- **New in special encounters: grass and pot surprises.** Cut grass can release a wall Skulltula or a little Floormaster hand; a broken pot a Poe or a little hand; and about one plain pot in ten is a flying pot. The chance is 1 in 100 in this rc (1 in 50 is planned). Grass surprises were seen working at a raised test rate. Pots in the Market guard house gave nothing; not understood yet. See [features/23](features/23-grass-and-pot-surprises.md).
- **New in special encounters: ten Like Likes around Lake Hylia.** Not played yet.
- **Special encounters:** the Iron Knuckle in Hyrule Field is awake from the start ("perfect"). Dead Hand in the Hyrule Field hole was moved back twice and still rises near the entrance.
- **New: Cuccos take revenge at once.** One hit brings the swarm. Seen working in Kakariko. The two minigame cucco kinds are left out. See [features/24](features/24-cucco-revenge.md).
- **Enemy regen, fairies help enemies, call for backup** now run beside the game's damage-flash routine instead of inside it. Not confirmed: with these on, killing a gate guard crashed the game once. Leave them off for now.
- **New setting that does not work yet: spawn enemies with L + R + D-pad.** The checkbox is there; pressing the buttons does nothing. See [features/25](features/25-spawn-enemies-combo.md).
- Review fixes: a broken gate or shop setting is reported instead of ignored; the battle-music change is pinned to its five enemies; tips corrected.

## rc 9

- **Lose heart on death now works.** The heart is taken after the Try Again reload, in an event the game runs once the map is back. Played by script and by hand: 6 → 5 → 4 → 3 hearts, a further death stays at 3, a fairy revival costs no heart. Limit: the loss lasts for the play session; quitting and continuing restores the hearts. See [features/07](features/07-lose-heart-on-death.md).
- **No bottles during combat** also refuses bottles while miniboss music plays (it only knew the ordinary battle music). Played by hand against the Iron Knuckle: refused while he is aggro, allowed again at a distance, and a bottled fairy still revives Link in the fight. See [features/08](features/08-no-bottles-in-combat.md).
- **New: More enemies: special encounters** (one checkbox). A Dead Hand waits in two holes (Hyrule Field east, Death Mountain Trail) and an Iron Knuckle stands in the open part of Hyrule Field, awake from the start. Played: both maps load as child and adult; both Dead Hands rise and attack; the knight wakes without being hit. Dead Hand was then moved further from the landing spot, and "awake from the start" replaced "wakes when near"; those two changes are not played yet. See [features/22](features/22-special-encounters.md).
- **Enemies block the way:** the wall became two spider webs in front of a solid wall. Played by hand: the web blocks but can be bombed, and it sat on top of its guards. It is being rebuilt on positions marked in the game; treat this setting as unfinished in this rc. See [features/20](features/20-enemies-block-the-way.md).
- Confirmed in play with no code change: no Z-targeting, no dupe items (a second fairy cannot be bottled), no gossip fairies (six songs tried), no heart drops and no ammo drops (about 20 grass: rupees and bugs only), the Bean Seller's pitch.
- Known problem: with enemy regen, call for backup or fairies help enemies on, enemies can look tinted red. Cause not found yet.
- Not played yet: enemy speed, enemy regen, more enemies, boss minibosses and minions, enemies respawn sooner, fairies help enemies, call for backup, Zora's Domain ambush.

## rc 8

- **Enemies call for backup**, reworked. See [features/17](features/17-enemies-call-for-backup.md).
  - Strictly once per enemy: no second call, even after the helper dies.
  - Bosses, minibosses, Iron Knuckles, Dead Hand and Phantom Ganon never call.
  - The helper fits the caller: Shabom for Jabu-Jabu's enemies, a flying pot for the Ice Cavern, Shadow and Spirit Temple enemies, a Peahat larva for the rest.
- Played: nothing yet.

## rc 7

- **No bottles during combat:** Leevers and Moblins, the only enemies that fought without battle music, now start it when this setting is on, so the music check covers them. See [features/08](features/08-no-bottles-in-combat.md).
- **No dupe items:** a refused catch or hand-over now ends properly (head shake, then the same closing steps the game uses) instead of ending the event at its start. See [features/13](features/13-no-dupe-items.md).
- Played: nothing yet.

## rc 6

- **Lose heart on death, third version:** the heart is taken through the player's own "set max health" action on Retry. See [features/07](features/07-lose-heart-on-death.md).
- **New: Enemies block the way.** First gate: the path to the Deku Tree stays walled while the two withered Deku Babas beside it are alive. Nothing is saved. Skipped with an open forest or entrance shuffle. See [features/20](features/20-enemies-block-the-way.md).
- Played by script (rc.4 Longshot):
  - Lose heart on death still fails: 6 hearts before and after one game over and Try Again. The save flag read 0 before, at game over and after, so the added steps did not run at all.
  - Enemies block the way: Kokiri Forest loads, and the wall stops Link at the expected line with both Deku Babas alive (three attempts). Killing the Babas and walking through was not tested.
  - King Dodongo's black screen (rc 4 builder) comes from the edited static map file of Dodongo's Cavern: the cave loads without it, and stays black without either of the other two edited files.
  - Not tested: red enemies, Zora's Domain ambush.

## rc 5

- **Bosses spawn minibosses** no longer touches King Dodongo: his dungeon loaded to a black screen with it on.
- Played by script (rc.4 Longshot):
  - Enemy damage 3x works: a Deku Baba bite took 2 quarter hearts at 1x and 6 at 3x.
  - Dodongo's Cavern loads with boss minibosses and minions on.
  - Combat music is named `BGM_EnemyBattle`; it starts when an enemy notices Link and stays on until the map changes or Link dies.
  - Lose heart on death failed: 6 hearts before and after two game overs.

## rc 4

- Four more settings, 19 in total: **Enemies call for backup**, **Zora's Domain ambush**, **Fairies help enemies too**, **Bean Seller's sales pitch**.
- Played by script (rc.3 Longshot), load test only: Kokiri Forest, Zora's Domain, Hyrule Field, the Deku Tree and Fire Temple load with nine map and enemy settings on. Dodongo's Cavern went to a black screen, traced to boss minibosses.

## rc 3

- First combined candidate, 15 settings. Added: **Enemies regen health**, **No Z-targeting**, **More enemies**, **Enemies respawn sooner**, **No dupe items**, **Bosses spawn minibosses**, **Bosses spawn minions**.
- Edits that touch the same file now build on each other.
- Played: nothing.

## rc 2

- Fixes from the rc 1 playtest:
  - **Bottles:** the combat check was inverted, and a refused bottle froze Link.
  - **No heart / ammo drops:** the game's global drop lottery added drop chance at low health or ammo; those boosts are zeroed and the lottery's heart and ammo actors point at the green rupee.
  - **Lose heart on death:** Retry now asks the game to save before it reloads.
- The enemy sliders read as multipliers.
- Played: nothing.

## rc 1

- New Difficulty page with eight settings: **enemy health**, **enemy damage**, **enemy speed**, **no heart drops**, **no ammo drops**, **no gossip fairies**, **lose heart on death**, **no bottles during combat**.
- Played by hand: enemy health scaling works. Bottles never worked and froze Link, hearts and ammo still dropped, lose heart on death did not stick.
