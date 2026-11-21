# Suggested improvements (for your review)

A running list. Nothing here is done unless it says so. Each item says what it would change, why, and how sure I am. Tick or strike what you want and I'll act on it.

Last updated: 18 Nov 2026, after rc 5.

## Your decisions so far (18 Nov)

| Item | Decision | Status |
|---|---|---|
| A1 | Do it | Being built: enemies that fight without battle music get it when "no bottles during combat" is on, so the music check covers them. A scripted test also found the music lingers while an aggroed enemy is alive, even far away (see A15) |
| A2 to A6 | Test them myself | A3 and the rest are in the scripted playtest queue, one game copy at a time |
| A7, A8 | Maybe later | Parked |
| A9, A10 | Agreed | Being built |
| A11 | Agreed | Built on `feature/dupes-v2` (`537a602`), not yet merged or played |
| A12 | Green rupee is fine | No change |
| A13 | Keeping progress since the last save is fine | No change |
| A14 | Fine as is | No change |
| Red enemies | See for myself | In the scripted playtest queue |
| Zora hole | Yes, that hole | Confirmed |
| Lose heart at three hearts | Whatever is simplest | Unchanged: no heart lost at three, progress still saved |
| Portability | The mod must work wherever OOTRR works; Longshot may only be changed as a test tool | C2 and A1 option d are dropped |

## New requests since rc 5

| # | Your request | Status |
|---|---|---|
| N1 | "Mido literally standing there and blocking you ... until you kill all the enemies in the area" | Idea for the next version of "Enemies block the way". The first gate uses an invisible wall and has not been loaded in the game yet; once it loads, a visible blocker such as Mido can replace the wall |
| N2 | "add a dead-hand into the hole which is really dark and spooky" | Need to know which hole (see D4) |
| N3 | "let volvagia roam above hyrule field as an adult" | Being investigated |

## A. Fixes to existing features

| # | Feature | Suggestion | Why | Confidence |
|---|---|---|---|---|
| A1 | No bottles during combat | Replace the music check with the thing that starts the music (your suggestion) | Enemies that start no battle music (Stalfos and others) don't block bottles today, and music randomization could break the check | Researched 18 Nov: see "A1 findings" below. Needs your pick |
| A2 | No bottles during combat | Confirm or replace the head-shake animation | `EvtTalkNo` is a guess at a "can't do that" gesture | Needs one look in the game |
| A3 | Bosses spawn minibosses | Find what in the edited static map file of Dodongo's Cavern breaks loading, then put King Dodongo back | Narrowed by script (18 Nov): the cave loads without our static map file and stays black without either of the other two edited files. What in that file is wrong is not known yet; the same mistake could be in the other four boss rooms, though Deku Tree and Fire Temple loaded | Compare our additions to that file with the ones that load |
| A4 | Bosses spawn minibosses | Create the miniboss after the boss intro instead of on room entry | It can currently attack before the cutscene starts | Medium: needs a different signal in each room's logic |
| A5 | Bosses spawn minions | Tie placed minions to the boss so they leave when it dies | They stay after the fight and are back on later visits | Medium: same mechanism the minibosses use |
| A6 | More enemies / minions / Zora ambush | Check every added position against real collision | Positions were chosen without wall or floor data; the object map site has it | High value, effort depends on how collision is read |
| A7 | Enemy speed | Cover animation-driven enemies (Octorok, Skulltula) | Only enemies with a movement file change today | Low: no global handle found yet |
| A8 | Enemy damage | Cover grabs (ReDead, Like Like, Wallmaster) | Their damage is not in the attack file | Low to medium: per-enemy research |
| A9 | Enemies call for backup | Make it strictly once per enemy, and exclude bosses | It can call again after its helper dies, and bosses are included only because exclusion was awkward | Medium |
| A10 | Enemies call for backup | Summon something that fits the caller instead of always a Peahat larva | A larva indoors or under water may look wrong or misbehave | Medium |
| A11 | No dupe items | Give refused catches and refused gifts a proper ending | Ending a flow at its entry froze Link in the bottle feature; the same risk applies here | High: same fix pattern as bottles |
| A12 | No heart / ammo drops | Drop nothing instead of a green rupee where the game "needs" to give a heart or ammo | You asked for no drops; rupees were the safe substitute | Easy once the current fix is confirmed in play |
| A13 | Lose heart on death | Decide whether "keeps everything collected since the last save" is wanted | It is a side effect of the save-before-retry switch | Your call |
| A14 | Fairies help enemies | The "respawn the last defeated enemy" half | Judged not possible with data edits | Low |
| A15 | No bottles during combat | Decide what "in combat" should mean when the enemy is far away | Scripted test: the battle music started when a Deku Baba noticed Link and was still playing 40 seconds later with Link 10 units away; it only stopped on a map change or death. So bottles would stay blocked for as long as an aggroed enemy is alive anywhere nearby | Your call: accept it (retreating properly means leaving the area), or add a distance or time rule if one can be found |
| A16 | Lose heart on death | Third version failed too (script, rc 6): 6 hearts before and after a game over and Try Again | Three versions have now failed the same way, so the likely cause is shared: either the steps added after "Try Again" never run, or the game restores health from a snapshot taken at the moment of death. Next: a diagnostic run that makes the added steps visible (a deliberate pause) and tries taking the heart at the moment of death instead of on Try Again | Queued for the next scripted run |
| A18 | Enemies block the way | Wall confirmed by script: Link is stopped at the expected line with both Deku Babas alive. Killing them and walking through is still untested (the script could not swing the sword) | Half tested | Next scripted run |
| A17 | Starting items | The generator refuses some combinations such as sword plus bottle with "switched off by its controlling setting" | Found by the playtest script; not a difficulty bug but it gets in the way of test seeds | Not looked into |

### A1 findings: what flips the combat music

- Every enemy has a "battle presentation" component that decides when a fight starts and ends, from range and target conditions. The music is only one of its effects.
- Enemies with and without battle music use the **same** start and end conditions. The no-music ones (Stalfos and others) simply have no music name set.
- That component leaves nothing behind that an event can read: no save flag, no readable state. The boss flag `IsBossBattle` is written by the game's own code. Navi has an internal "in combat" notion, but nothing exposes it.
- So from the game's data alone, the music is the only readable trace of "in combat".

Options, cheapest first:

| Option | What it does | Cost | Catch |
|---|---|---|---|
| a | Give the no-music enemies the battle music too | About an hour | Changes the game's audio: Stalfos fights get the normal battle theme |
| b | Test whether one of the game's player-state numbers means "in battle" | A few scripted checks in the game, then a one-line change | May turn out that none does |
| c | Add a node to the shared enemy AI graph that raises a flag while fighting | About a day | Needs a spare unsaved flag and the engage/disengage hooks, which weren't found |
| d | Expose "in combat" from Longshot itself | Unknown until the source is read | Only works on Longshot, and you said changing Longshot is in scope |

## B. Workflow and tooling

| # | Suggestion | Why |
|---|---|---|
| B1 | Use the Longshot rc.4 build that the co-op kit already made on this PC for scripted tests | Its control bridge reports health, max health, position, music and save flags; your installed rc.3 reports none of those. No download needed. In progress |
| B2 | Build the latest Longshot from source (your request) | Cloned to `<Longshot checkout>` at nightly-464; a local build to `<Longshot game folder>` is in progress. F: because C: has only about 41 GB free and the build wants about 60. Once built, scripted tests and any Longshot changes of ours use this copy; your installed rc.3 is untouched |
| B3 | Put a build number on the Difficulty page or the build report | So a screenshot of the page tells us which build you tested |
| B4 | A "difficulty test" preset in the tool | One click to get a seed with the settings under test, 10 hearts, sword, shield, bottles and open forest |
| B5 | Regenerate the entity maps for every build | rc 4's maps still show the King Dodongo Lizalfos |
| B6 | Fix the dictionary-id bug in `flow_locations::dict_id` | It misreads the global AI pack; two features work around it separately |
| B7 | Teach the repo's BYML parser hash-keyed maps (type 0x20) | Several global tuning files can't be read today; the drop lottery had to be patched by hand |
| B8 | Follow CONTRIBUTING.md before any upstream pull request | New settings are on the Difficulty page, not under "In Dev"; comments exist in some new files; one concern per pull request means splitting this into about twenty |

## C. Bigger ideas

| # | Idea | Notes |
|---|---|---|
| C1 | Enemies block the way, stateless (your simplification) | Wall exists while the guarding enemy is alive, gone when it is dead, no saved state. Being attempted now |
| C2 | ~~A code-side mod through Longshot's mod interface~~ | Dropped: you ruled that the mod must work wherever OOTRR works. Longshot changes are for testing only, for example exposing more state to test scripts |
| C3 | Scripted regression run per build | A fixed script: new game, teleport to each edited map, screenshot, compare. Would have caught the Dodongo's Cavern black screen before you saw it |

## D. Open questions for you

1. **Red enemies:** constant tint or a flash, and does it happen with enemy health at 1x?
2. **Zora's Domain hole:** is the hidden hole near the middle of the domain the one you meant?
4. **Dead Hand hole (N2):** which hole is "the hole which is really dark and spooky"? The Zora's Domain one, or another (for example one in Kakariko or the Graveyard)? An approximate place is enough; I can find it on the object map.
5. **A15:** is it acceptable that bottles stay blocked until you leave the area or the enemy is dead?

## E. From the code review of 18 Nov (review-fixes branch)

| # | Feature | Suggestion | Why | Confidence |
|---|---|---|---|---|
| E1 | No ammo drops | Options if the Shopsanity warning is not enough: (a) refuse the combination with a clear error; (b) leave the "most depleted resource" help for arrows only, so one source stays; (c) make the logic place a seed-guaranteed arrow shop. Only the note was built | Shopsanity special deals can replace the shelf that sold arrows; logic does not check that ammo can be bought. (c) needs a logic change well over 50 lines | Your call: (a) is a few lines, (b) changes what the setting means, (c) is a project |
| E2 | Enemy graph edits | Grow each size table entry once per file, not once per pack (review finding 7) | `staging::grow_out` adds the delta plus 0x400 on every call, so the shared blackboard table (157 packs) is reserved about 170 KB too large. Not done: it changes upstream `staging.rs` | Low risk, small change; I did not touch upstream code without asking |
| E3 | Build | Fail the build when `late` holds one path twice, or let `story_skip` and `deku_bridge` read through `late` (finding 8) | A later step would silently replace a difficulty edit of the same file; no overlap exists today | Medium |
| E4 | Lose heart | Prefer a test of an ordinary later save before building the autosave fix (finding 10 is separate: end the boot ladder at the original second event) | See the lose-heart feature file | Needs the game |
| E5 | Enemy damage | Resolve the parent chain before rounding (finding 12); said to be harmless at the usual factors | Child files that inherit part of the attack data can end up with a different factor | Medium, needs a test per case |
| E6 | Code | Move the "read through `have`, else the game file" helper and the fresh-hash functions into shared modules; delete the dead `SeatedUntilNear` branch in `wake.rs` or make it a setting | About ten copies of the helper; four fresh-hash variants with different scopes | Mechanical but wide; left for a quiet moment |
| E7 | Frame counts | One timing in the game settles regen, fairy and backup (30 frames per second assumed) | `FRAMES` in the three files | Needs the game |


## From the Opus fixes (19 Nov)
- Call for backup: the helper comes after about 12 s of fighting, not 25 (the timer counts 60 frames a second). Question for the user: keep 12 s or set 25 (`FRAMES` 1500, one line)?
- Spawn combo: after 12 alive enemies nothing more appears; whether killing one frees its slot is not yet seen. Test on a real pad with the Longshot L+R+D-pad shortcut off.

## From the horde rework (More enemies 1x to 10x, 19 Nov, not played)
- **Cap.** Hyrule Field reaches the 240 cap at 5x, so 5x and 10x look the same there. After the load test, raise `horde::MAP_CAP` (one const) if 240 runs well, or make the cap scale with a map's walkable area. Question for you: is a busier Hyrule Field worth the risk?
- **Water.** The navigation mesh has no water, so water enemies only spread 3 to 15 units from vanilla Octoroks. A reader for the collision files (`Phive/StaticCompoundBody`, water material) would find all water and its depth, and allow Shell Blades and Stingers underwater.
- **Ceilings.** Not in the mesh either: dungeons are treated as roofed and fliers kept 2.5 up. The same collision reader would give real ceiling heights for Skulltulas and Keese.
- **Guays by day.** The horde copies the vanilla field Guays, which carry the night-only key. Should horde Guays fly by day (drop the key)?
- **Minibosses at the start.** None in child Kokiri Forest (no sword yet). Should other early maps (Hyrule Field as child, Deku Tree) also be spared at 5x and 7x?
- **Lizalfos.** Plain Lizalfos are left out because of the Dodongo black screen; Dinolfos pairs stand in. Once the black screen is explained they could come back.
- **Skipped maps.** Towns (Zora's Domain, Goron City, child Market), Hyrule Castle, Gerudo Fortress, Gerudo Training Ground, the collapsing castle. Any you want included?
- **Old settings strings.** A string from an earlier build with More enemies at Half again or Double reads as 1x with a warning (the setting changed from a choice to a slider).
- **Ledges.** The in-bounds rule (actors nearby) still lets an enemy onto a roof or cliff top that has props. If the game test finds some, a reachability check from the map's arrival points over the mesh would remove them.


## From the enemy randomizer v2 (20 Nov, branch feature/enemy-randomizer, not played)
- **More enemies could change, if you allow it.** v2 draws from every map and changes about four in five of the enemies it may touch, but with your v1 safety rules only 117 enemies in the whole game may change (91 do with seed "game"). Unreferenced enemies blocked only by an extra placement key: `Presence` (a spawn condition such as night or a story flag; 22 enemies) and `IsPhysicsStable` (9). Allowing both keeps the "nothing refers to it" rule and adds about 30. Not done: your call. The other 102 blocked enemies are named by an AI group (room logic), so they stay.
- **Skulltulas in child maps never change** (18): the only other ceiling kind is the Wallmaster, which the Kokiri Sword alone cannot beat. A Keese hanging from the same ceiling would be the only option, untested. Not done.

## Future work from the 21 Nov hands-on (not started)
- **Volvagia in the field (Functional WIP):** when beaten she should drop a red rupee and flee, not freeze before her death animation. Q17 (too harsh?) is unanswered; the default is to leave her deadly.
- **Fireball storms:** swap the Death Mountain backdrop to its erupting version while a storm is active. Desert Colossus storm is switched off because Link fell through the floor there; find the cause before bringing it back. The storm wrong-spot bug (rocks landing in another ring) is fixed on `playtest/next` but unconfirmed in the game.
- **Random enemy sizes (145/146):** not working. The game ignores placement Scale for ordinary enemies. Find what made the mega Like Like and the temple Beamos bigger (a pack value or model scale) and use that.
- **Gate editor (Longshot tool):** needs a real blocking barrier placed live, tied to a dummy guard, instead of fairy markers.
- **Playtest panel:** a teleport button should switch to the age a place needs (the child Wasteland teleport goes to a black screen), or refuse with a message.
- **Gates 15 and 16:** move up the Death Mountain Trail above Goron City once the user sends a screenshot.
- **Enemy randomizer:** the user still sees few swaps (Dodongo's Cavern); v1 was called boring.
