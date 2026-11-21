# Enemies call for backup

Setting `enemy_backup` (number 126), checkbox. Code: `randomizer/core/src/difficulty/backup.rs`. Built by a sub-agent on `feature/enemy-backup` (`f3f674c`).

## Request
"enemes call for backup. if you take too long defeating an enemy, they can call summon another enemy"

## Interpretation
After a fight has lasted a while, the enemy summons one more enemy near itself, with limits so it cannot snowball. I suggested 20 to 30 seconds and at most once per enemy or a long cooldown.

## Investigation
- **"In a fight":** the enemy currently has a valid target (`QueryTargetIsValid`), the shape Gohma's own battle graph uses.
- **Spawning:** the game's "actor holder" component lets one actor own and create another. Vanilla examples: Bari holds Biri, big Leevers hold Leevers, Floormasters hold small ones, and a fishing lure creates its hat on demand with the exact node sequence used here.
- **What to summon:** a helper that does not run the shared enemy graph, so it can never summon in turn. Every Keese variant runs that graph, so there is no Keese helper. The choice depends on the caller (table below), and the Peahat larva (`Enemy_Peahat_Larva_Independent`, the standalone larva from child Hyrule Field) is the fallback.
- **Once per enemy:** two per-actor bool keys, `OotrrBackupEligible` (starts true) and `OotrrBackupUsed` (starts false), are added to the shared `EnemyCommon` blackboard table in every caller's pack. Vanilla precedent: `ExecuteBlackboardSetBool` with `ExecuteOnce` and `IsIgnoreNotExistsError` writes the `EnemyCommon` key `IsStunLightEnable` in the Tektite graph, and `QueryBlackboardGetBoolNoError` reads keys in the Amos, Gibdo and Lizalfos graphs.
- **Bosses out:** the excluded actors get no `Eligible` key and no holder of their own. A missing key reads false, so their graph stops at the first check.

## Implementation
- The shared enemy graph (same hook as regen) gains 24 nodes. Per frame: if `Eligible` is false, do nothing. Otherwise: if the target is valid and a created helper sleeps in the holder, wake it 1.5 m above the caller; else if the holder is empty and `Used` is false, run a 750-frame timer (about 25 seconds), then request the helper and set `Used`. `Used` is set at the same moment as the request and is never cleared, so there is no second call even after the helper dies. The holder nodes sit below the `Eligible` check, so an excluded actor never touches a missing `Backup` entry.
- Excluded: `Enemy_DungeonBoss_*`, `Enemy_MiddleBoss_*` (Dark Link, Nabooru's Iron Knuckle, minibosses), `Enemy_IronKnuckle_*`, `Enemy_DeadHand*`, `Enemy_PhantomGanon*` and `Enemy_GhostPicturePerformer_*` (Phantom Ganon, his horse, the performers), `Enemy_Guay_Boss`, `Enemy_KotakeKoumeBattleManager`. 33 of the 123 actors that run the graph are excluded. A few boss packs carry a parent enemy's parameter file that does gain the holder slot, but without the `Eligible` key it is never used.
- 81 callers get an `ActorHolderRef` (65 own a new shared holder, 10 existing holders gain a `Backup` entry) and the two keys in their common blackboard table. Three new shared holder files, one per helper.
- Extra AI memory for the 123 actors that run the graph, and a size table entry for each new file.

| Caller group | Helper | Why |
|---|---|---|
| Bari, Biri, Stinger (both), Shell Blade, Tentacles (Jabu-Jabu natives) | `Enemy_Shabom` | The game places Shabom only in Jabu-Jabu's Belly, where these callers live; it floats and does not run the shared graph. |
| Freezard, Fixed Freezard, Gibdo, ReDead (3), Wallmaster, Like Like, Floormaster (3) (Ice Cavern, Shadow and Spirit Temple natives) | `Enemy_FlyingPot` | The game places flying pots in the Ice Cavern, Shadow and Spirit Temples (and Gerudo Fortress); it does not run the shared graph. |
| Everything else, including Octorok and Blue Tektite | `Enemy_Peahat_Larva_Independent` | Fallback: flies, runs no shared graph, known to exist as a standalone actor. |

A holder file shared by callers of different groups falls back to the larva.

## How it was tested
- backup-r3 (19 Nov, script, `playtestackup-r3\RESULTS.md`): 32 callers spawned beside Link in Hyrule Field, one game run, no crash. Expected helper once after 15 to 30 s (scan every 15 s) for Stalfos, Wolfos, Lizalfos, Dodongo, club Moblin (Guay), both Tektites, Dekubaba and Scrub_Mad (each other), Gohma larva, Floormaster (Wallmaster), Like Like, Poe, green and red Bubble, Torch Slug, Octorok, Shell Blade (Stinger). Spear Moblin and Bari gave one helper, not two. Nothing in 75 s from Stalchild, Freezard, Skulltula, Wallmaster, Amos, Spike, Tailpasaran, Biri. Keese and ReDead called nobody. No helper called in turn. Not tested: weak Stalfos, Gibdo, white Bubble (not spawnable in the field), water and Jabu kinds.
- rc 13 (19 Nov, merged build, script): Keese and a ReDead spawned 6 units away for 90 s: no helper, game stayed up. Stalfos: one Keese after 24.5 s and 15.9 s (from spawn); Lizalfos: one Baby Dodongo after 25.4 s and 24.4 s; one helper each in 50 s, no crash. The moment the caller notices Link is not observable, so the 10 to 25 s delay is consistent but not timed from the notice.
- Difficulty tests with node shapes compared against the lure, Dodongo, Gohma, Twinrova, Tektite and Amos graphs, plus checks that every helper is outside the shared graph and every excluded actor lacks the `Eligible` key.
- A real build; the built mod has the patched graphs, new file, holder entries and memory growth.

## In the game
Played in a scripted test on 19 Nov after the Opus fix (see the end of this file). To test by hand:
- Turn this on alone. Find one ordinary enemy (a Deku Baba or Stalfos), stay close without killing it for 20 seconds.
- Pass: a Peahat larva appears above it. Kill the larva and keep fighting: no second one may come.
- First check: that the map loads at all, then that the helper actor is created outside the map it normally lives in (Shabom outside Jabu-Jabu, flying pot in the Ice Cavern, Shadow or Spirit Temple).
- Fight a boss or miniboss for a minute: nothing may appear.
- Report: a crash when enemies load, nothing happening, or helpers piling up.

## Notes
- It rests on untested assumptions: that the holder component switches on just from being added to an enemy's parameters, that a missing blackboard key reads false without an error, and that a new key in the common blackboard table needs nothing else.
- Helpers are created on demand, so their packs must load in maps that never placed them. Unknown until played.
- The 25 seconds assumes 30 counted frames per second.
- What happens to the helper when its caller dies is unknown.

## Red enemies (session fixes, 18 Nov, code only)
The user saw enemies tinted red with regen, fairy and backup on (screenshot of a Deku Baba in Kokiri Forest). Findings from the game files:
- The damage colour is made by one native node, `ExecuteSetEnemyDamageColorAndLight`, alone in `AI/EnemyCommon.common.reactiondamagecolor.module.ainb`. In the game's own file that node is the whole graph: no parameters, no inputs, no outputs, no links. Its colour logic is in game code, so the graph cannot be read for a cause.
- Regen, fairy and backup each wrap the old root in a Parallel (`EndPolicy` 0, `ResultPolicy` 0, the same values the game's own caller graphs use) with the old root as its first child. With all three the node sits three Parallels deep. Its own bytes and the link order are unchanged (new test `the_vanilla_damage_color_node_is_untouched_after_regen_fairy_and_backup`). Nothing writes a colour or damage key.
- Only regen writes to the life component (`OneShotLifeSetOwnLife`, add 1 every 150 frames, clamped at the maximum). The game uses that node only in the player's and NPCs' event graphs, never in an enemy graph.
- So no edit was found that changes the colour path, and none was made. Two causes remain:
  1. Regen: a heal, even one that changes nothing, may count as a life change that the colour node reads as a hit, giving a flash every 150 frames.
  2. The wrapper: the colour node runs as a child of a Parallel and not as the module root, and may behave differently there.
- Not caused by the speed setting (it scales keys ending in Speed and Interval only) or by the gate edit (the gate does not touch the guards' components).
- To tell them apart, build three mods with one setting each and look at a Deku Baba: `enemy_backup` only (wrapper but no heal), `enemy_regen` only (wrapper and heal), `enemy_speed` 200 only. Red with backup only means cause 2; red only with regen means cause 1; red with neither means another feature.

## Review fix: caller graphs, every table copy (branch `feature/review-fixes`)
- Hosting: as in 09, the guard switch is an extra child of the Parallel that holds the damage colour module call, in every caller graph. The module graphs stay as the game has them.
- Exclusion (review finding 6): the keys `OotrrBackupEligible` and `OotrrBackupUsed` were only in the copies of `EnemyCommon...BlackboardParamTable.bgyml` of packs of actors that may call, so with one copy per path the load order would decide. Now every one of the 157 packs that carries the table gets the same bytes (also boss and non-`Enemy_` packs, whose heap grows by the backup amount too). Bosses are kept out by the graph instead: their own caller graphs are not edited, and a shared caller graph is edited only when at least one ordinary `Enemy_` actor uses it (`Enemy_IronKnuckle`'s graph and `DungeonBossBase` are left alone). Test: `every copy of the common blackboard table carries the backup keys` in the game-file test and `graphs_of_bosses_are_left_alone_and_shared_ones_follow_their_users`.
- Known remainder: `Enemy_MiddleBoss_Lizalfos` (shares the `EnemyBase` graph) and `Enemy_Guay_Boss` (shares `Enemy_Guay_Flock`) are still excluded only by having no `Backup` holder entry, so they run the eligibility queries on a key they do not have. Check in the game that the Lizalfos miniboss in a dungeon calls nothing.

## Opus fix: the crash (19 Nov, `feature/opus-fixes`, commit `e4ff217`)
- **Defect.** Both flags were added with `IsInstanceParam: true`, which marks a key filled from the map placement (as the knight wake key is, set in the placement's `Dynamic`). Across the game's graphs (the AI pack and every actor pack), no type 1 blackboard write ever names such a key: about 3,600 type 1 writes all go to plain keys, while reads of placement keys do exist. Our latch (`ExecuteBlackboardSetBool` on `OotrrBackupUsed`) was the only write of a placement key in the game, and it crashed the game the moment it ran (`longshot.exe+0x753a5b3`, x8 = x9 = -1; the crashing node object has the same class pointer, `0x849a2148`, in every backup and spawn crash log, and a key handle of -1). The timer finishes after about 12 s and not 25 (750 frames at 60), which is why the old crash came a few seconds after the test's Lizalfos had been fighting.
- **Fix.** Both rows are plain keys now (`state_row`, `IsInstanceParam: false`); the knight wake key stays a placement key. Nothing else changed.
- **Checked by tests.** `the_game_reads_placement_keys_from_graphs_but_never_writes_one` (the rule above, over every graph in the game). `every_key_a_backup_node_uses_is_a_plain_key_where_its_blackboard_type_looks_for_every_actor_that_runs_it`: stages the real mod, finds every graph that hosts backup nodes (shared ones in the AI pack and ones inside actor packs), every actor that runs each of them, follows that actor's `BlackboardInfo` (and parents) to its tables, and requires each type 1 key to be there as a plain key, and each type 0 key to be in the graph's own key section. With the old rows it fails; no host actor lacks the keys.
- **Seen in the game** (scripted, Hyrule Field, backup only, `<playtest folder>\opus-fixes\TESTS.md`): no crash for the whole run, including 90 s with two Lizalfos next to Link (the old build crashed 4 to 8 s after a Lizalfos appeared). One new Peahat larva per Lizalfos, 10 and 16 s after the second one appeared, both staying beside Link; no further helper after both Lizalfos died. The scripted kill rounds did not run.
- The setting text now says about 12 seconds. If the user wants 25, `FRAMES` goes to 1500 (not tried).
- Still open: Shabom and flying pot helpers outside their home dungeons; the Lizalfos miniboss and `Enemy_Guay_Boss` run the shared graph and now have the `Eligible` key (every copy of the table is the same), so they are kept out only by having no `Backup` holder entry: check in Dodongo's Cavern that the miniboss calls nothing and does not crash.

## Leever helper (19 Nov, `feature/backup-leever`)
User, after seeing it in a fight: "peahats are a funny but terrible ally for these guys to summon, 'cause they actually damage the enemies. have them summon those little creates from the desert colossus that pop up from the sand"

- **Change.** The fallback helper is now `Enemy_Leever` (the ordinary small Leever) and not the Peahat larva. Shabom and the flying pot are unchanged: no evidence they hurt other enemies.
- **Which Leever.** `Enemy_Leever_Fixed` differs from `Enemy_Leever` only in a placement-style actor parameter set (its own death and blackboard files) and `PreparePopupTimeRand` 0 (no random delay before popping up). The AI is the same, so the plain one is used.
- **Sand.** The Leever's run, disappear, small-hit and repelled modules query `QueryPhysicsIsMaterialContactToSphere` in except mode on dynamics material 3 (sand), radius `RadiusToContactNotSand` 0.75, and steer away or go into the `Disappear` module on contact with anything else. The appear module has no sand query, so it can be created anywhere, but off sand it is likely to sink away again quickly. Not seen in the game. If it is useless away from sand, the options are to keep it only for callers on maps with sand or to go back to the larva.
- **No chains.** The Leever runs its own utility graph, which calls the damage colour module, so it would host the backup nodes. All Leever kinds (`Enemy_Leever*`) are now in `OUT_OF_BOUNDS` and never call: nothing is added to their graphs. Cost: desert Leevers never call for backup. Test: the helper kinds are never callers (`bosses_and_their_kin_are_left_out_and_helpers_follow_the_caller` and the game-file test).
- **Friendly fire.** Every enemy has the same affiliation and target affiliation, so nothing in the parameters separates the larva from a Leever. The Leever's charge attack is on the Leever's own damage blacklist, which only stops Leevers hurting each other; nothing excludes other enemy kinds. So a Leever's charge may still hurt other enemies, but it is a short rush and not a spinning blade. Unproven from the files; watch in the game.
- **Unloaded packs.** With the test tool, `Enemy_Leever` and `Enemy_Leever_Big` both appeared in Hyrule Field. Other maps are unverified.
- **To check in the game.** A Leever appears beside a fighting enemy off sand: does it stay, chase, or sink at once; does it hurt other enemies; does it summon anything itself (it must not).

## Rework after the hands-on playtest (19 Nov, `feature/backup-r2`)
User, after playing: "ok leevers are no good. first of all, they spawn exactly on top of the skeletons, rather than near them. second of all, they kill the skeletons. field skeletons should just spawn keeze. also the leever logic is not very good in the field... seemingly they're just kinda stuck in place and not really chasing me." / "don't spawn the backup enemies an exact number of seconds after the battle begins, randomise it a bit. because now, when there's 3 enemies on screen, 3 backups appear all at the exact same second, it's a bit jarring" / "let's make the backup enemies that each enemy spawns different. redeads don't need to spawn any backup enemy at all". Earlier: a flying pot "spawned but it was floating in the air".

- **Helper per caller kind.** One const table, `WHO_CALLS_WHOM` in `backup.rs` (first prefix match wins, default Keese), nine helper kinds with one shared holder file each (`HELPERS`). Skeletons, Gibdo, Wolfos, Tektites and anything unlisted: `Enemy_Keese`. White Wolfos, Freezard: `Enemy_Keese_Ice`. Torch Slug, bubbles: `Enemy_Keese_Fire`. Lizalfos, Dinolfos, Dodongo: `Enemy_BabyDodongo`. Moblins: `Enemy_Guay`. Skulltula, Deku Baba, Mad Scrub: `Enemy_Skullwalltula`. Floormaster, Wallmaster, Like Like: `Enemy_Floormaster_Little`. Armos (and Beamos, which does not run the graph): `Enemy_FlyingPot`. Octorok and the Jabu-Jabu natives: `Enemy_Shabom`. The Leever fallback is gone.
- **Small fry never call** (`OUT_OF_BOUNDS`): the mod cannot tell a summoned instance from a placed one, so every helper kind is barred (all Keese, Baby Dodongo, Guay, Skullwalltula, little Floormaster, flying pot, Shabom), plus ReDeads, Leevers, Peahats and larvae, and the old exclusions (bosses, minibosses, Iron Knuckles, Dead Hand, Phantom Ganon). Keese, Guay, Leevers etc. still run the shared enemy graph, so they carry the check nodes; they have no `Backup` holder entry, as the Lizalfos miniboss does.
- **Shared holder files.** Checked over every caller: no holder file is shared by callers of different groups (the only shared one, `Enemy_Poe`, is all one group, Keese), so none falls back; a test asserts every caller gets its own group's helper.
- **Random delay.** The frame counter node has a vanilla field `RandomAddCountFrames` (Wolfos run module: 40 plus up to 80; Obj_Fairy: 1800 plus up to 30; a Cucco: 0 plus up to 15). The timer is now 600 plus up to 900 frames, so 10 to 25 seconds at 60 fps, rolled by the node for each enemy. Nothing is stored.
- **Placement.** The wake node's `Pos` input is fed from the caller's own translation plus a constant (2.5, 0, 0): 2.5 units to the side on the caller's own height. The graph is shared by all callers, so there is one offset; ground helpers stand on the ground and Keese and Guay fly up on their own.
- **Keys.** Unchanged: plain keys `OotrrBackupEligible` and `OotrrBackupUsed`; no node writes a placement key; the key-per-host-graph tests pass.
- **Appearance in Hyrule Field with the test tool (user's note):** Keese, ice Keese, Baby Dodongo, Guay, Skullwalltula, little Floormaster and flying pot appeared; `Enemy_Keese_Fire` did not. A helper whose pack the area has not loaded simply does not appear; the tip says so. Fire Keese callers (Torch Slug, bubbles) may therefore call nothing outside the Fire Temple.
- **Tests.** Table rows, no helper kind is a caller, every caller gets its own helper, timer shape (600, 900) and offset (2.5, 0, 0), plus all the older tests. Not seen in the game.

## Option 3, "bring a friend" (19 Nov, `feature/backup-r3`)
User's choice from the three options in `enemy-table.md` (Who calls whom): option 3, the helper is the same kind or a peer.

- **Table.** `WHO_CALLS_WHOM` in `backup.rs` follows the option 3 column row for row (first prefix match wins, anything unlisted calls a Keese). 31 helper entries (`HELPERS`: actor, holder file, count), one shared holder file each. Two-helper rows: spear Moblin (two Guay) and Bari (two Biri). "Nobody" rows (`CALLS_NOBODY`): gold Skulltula and the Poe sisters (Amy, Beth, Joelle; also their base and the two composer Poes, `Poe_Flat` and `Poe_Sharp`, which are scripted too). ReDeads still never call (user's earlier choice); a Gibdo now calls a ReDead.
- **Fallback.** Club Moblin calls one Guay (its option 2 helper), because the spear Moblin did not work when created. No other row falls back. Every caller gets its own row's helper: no holder file is shared by callers with different helpers (test `FELL_BACK` is empty).
- **A called helper never calls.** The bar now applies to the created actor, not the kind. The wake node takes an overwrite parameter, the way the game's big Leever wakes the small Leevers it made: `QueryGenericMakeOverwriteParam` -> `QueryBlackboardOverwriteParamAddBool` (`IsSpawnFromLeeverBig` true) -> `OneShotActorHolderTryCancelSleepActor.OverwriteParam` (`AI/Leever_Big.action.Run.module.ainb`; the Gohma egg wakes its larva the same way with `Enemy_Gohma_Larva_IsBossSpawn`). Ours chains two marks: `OotrrBackupEligible` false and `OotrrBackupUsed` true, so the helper's own guard stops at the first check, and even past it the latch reads as spent. The game's overwrite keys are plain keys (`IsInstanceParam: false`), as ours are, and no graph node writes them: the overwrite is a creation argument. Every helper kind runs the shared graph and carries both keys as plain keys (test).
- **Ceiling.** Two create tries run together (`OneShotActorHolderTryRequestCreateActor`, the lure's shape); the holder's `MaxActors` (1 or 2, never more: test) decides how many come. The request only runs when the holder is empty and `Used` is false, and `Used` is set in the same frame. So one enemy makes at most two helpers in its life, and if the mark were ignored, a chain could grow by at most one generation every 10 to 25 s while each one fights.
- **Kept.** 600 + up to 900 frames delay, once per enemy, 2.5 units to the side, bosses, minibosses, Iron Knuckles and Dead Hand never call. Small fry (Keese, Baby Dodongo, Guay, Skullwalltula, small Floormaster, flying pot, Shabom, Leevers, Peahats) still never call.
- **Vanilla children.** Actors the game itself creates through a holder (Bari's three Biri, Gohma egg larvae) are not marked, so each may call once.
- **Tests.** Option 3 row by row, counts and the ceiling, the mark chain on the wake (keys and values), node shapes against the big Leever's mark, make and wake nodes and the lure's request, every helper has both keys as plain keys, plus the older key and placement tests. `cargo test -p ootrr-core difficulty` and `settings` pass with the game files.
- **In the game.** Not seen. Plan: `<playtest folder>\backup-r3\TESTS.md` (each caller in Hyrule Field, 60 s for the helper, 60 s more to see the helper call nobody). Known doubts: Gibdo, big Poe and blue Bubble do not appear when created outside their homes, so they are hard to test in the field; a created Biri only drifts.

## Lesser helpers, called at the caller's spot (19 Nov, branch `feature/fx-r1`, commit 91c3d0a)
- **Request (hands-on, 19 Nov).** "Let's not have things call exactly the same thing as backup. They should call a slightly-lesser thing." Also: a Lizalfos' helper appeared over lava in Dodongo's Cavern and fell in; Dodongos and Baby Dodongos must not be added where their blast can open walls.
- **Table now.** Stalfos -> weak Stalfos; weak Stalfos, Skull Kid -> Stalchild; Stalchild -> Keese; White Wolfos -> Wolfos; Wolfos -> two Keese; Freezard -> ice Keese; Dinolfos -> Lizalfos; Gibdo -> ReDead (kept); Club Moblin -> Guay, spear Moblin -> two Guay; Tektites -> Guay; Mad Scrub -> Deku Baba; Deku Baba, Business and Deku Scrubs, Skulltula, Gohma larva -> Skullwalltula; Floormaster, Wallmaster, Like Like -> small Floormaster; Bari -> two Biri; Shell Blade, Stinger, Tentacle -> Biri; Lizalfos, Dodongo, Poes, Armos, bubbles, Torch Slug, Octorok, Biri, Spike, Tailpasaran and anything else -> Keese. A test asserts no caller calls its own kind.
- **Departures from the proposed table.** (1) Lizalfos and Dodongo call a Keese, not a Baby Dodongo: the helper table lives in the enemy's pack, not the map, so "overworld only" cannot be expressed; Keese is the globally safe choice and no enemy calls a Dodongo kind any more. (2) Armos -> Keese, not a flying pot, and Octorok, Biri -> Keese, Shell Blade, Stinger -> Biri, not Shabom: `Enemy_FlyingPot` and `Enemy_Shabom` do not run the shared enemy graph, so they lack the `OotrrBackupEligible` / `OotrrBackupUsed` keys the overwrite marks write (the game-file test caught it; a missing key can crash).
- **Placement.** The helper now appears at the caller's own position raised 0.5 (`RISE`), not 2.5 to the side, so it lands on the ground the caller stands on. Same node count; only the `InputB` constant changed.
- **Kept.** Once per caller, 10 to 25 s, a called helper never calls (overwrite marks).
- **Tested.** `cargo test -p ootrr-core difficulty` (186) and `settings` (106) pass. Not run in the game.
- **How to test in the game.** Spawn a Stalfos in Hyrule Field with backup on, stay targeted 25 s: a weak Stalfos appears where the Stalfos stands. Wolfos: two Keese. In Dodongo's Cavern, a Lizalfos calls a Keese.

## Effects experiment (19 Nov, branch `feature/fx-r1`, investigated, nothing built)
- **Request.** Use particle effects to show a regen tick, a fairy heal and a backup call.
- **Found.** The only generic effect node enemy graphs use is `OneShotXLinkSearchAndEmit` (170 uses; props `ExecuteEveryFrame`; inputs `ELinkCall`, `SLinkCall`, `XLinkKeyName` as a constant string; output `HandleSet`). Example: `Enemy_Leever.utility.postsensor.root` emits `Upheave`. The key names (`Death_Start`, `Wakeup`, `Dig`, ...) are looked up in the actor's own XLink user, which lives in the global effect-link resource, not the actor pack. Notes from another modder confirm that particle sets belong to actor bundles and that a set not loaded for the scene does not appear.
- **Not built (time box).** A key every enemy's XLink user has, and that reads as a heal or alert, was not identified; emitting a key such as `Death_Start` on a heal would read as dying. Next step: read the global XLink resource, list the keys shared by every enemy user, pick one, and add one `OneShotXLinkSearchAndEmit` beside the heal / call nodes (no blackboard keys needed).

## Harmless scrubs never call (19 Nov, branch `feature/scrub-backup`)
- **Report.** A business scrub spawned backup in a played seed.
- **Cause.** `Enemy_Scrub_Business` had its own row in `WHO_CALLS_WHOM` (helper: Skullwalltula) and was not in `CALLS_NOBODY`.
- **Change.** `CALLS_NOBODY` in `backup.rs` now also holds `Enemy_Scrub_Business` (prefix: covers `_Leaf`, `_Seed`, `_Dictionary`), `Enemy_Scrub_ForestStage` (covers `_Elder`, `_Leaf`), `Enemy_Scrub_Mad_Leaf` and `Enemy_Scrub_Deku_Leaf`. These are the same harmless scrub kinds the gate code ignores (`IGNORE` in `gates.rs`). The business scrub row was removed from the call table. `Enemy_Scrub_Mad` and `Enemy_Scrub_Deku`, the scrubs that fight, still call (Deku Baba and Skullwalltula).
- **Tested.** New assertions in `bosses_and_their_kin_are_left_out_and_helpers_follow_the_caller`. `cargo test -p ootrr-core difficulty` (186) and `settings` (106) pass with the game files. Not run in the game.
- **How to test in the game.** Backup on. Stand targeted by a business scrub in a shop room (for example Lost Woods or Hyrule Field grotto) for 60 s: nothing is called. A mad scrub in a fight still calls a Deku Baba.

## Effects, round 2 (20 Nov, branch `feature/fx`, built, not run in the game)
- **Request (19 Nov).** Show a visible effect when an enemy calls for backup.
- **Found: how effect keys resolve.** The whole game has one effect-link file (`ELink2/elink2.Product.110.belnk.zs`, 1267 users) and one sound-link file (`SLink2/slink2.Product.111.bslnk.zs`, 1216 users). Each actor names its user in its pack (`Component/ELink/<actor>`, `UserName`, normally the actor's own name). `OneShotXLinkSearchAndEmit` looks the key up only in the running actor's own user. There is no common user an enemy graph can emit through: `ReactionHit`, `ReactionField` and `Chemical` users exist, but the engine's reaction system plays them, not a key from an AI graph.
- **Found: when an effect shows.** A key's asset names an emitter set (`RuntimeAssetName`), and the set is found by name among the loaded effect bundles. Bundles belong to actors (`Effect/EffectFileInfo`, `EsetbList`), plus one resident bundle, `static` (`StaticEsetb`), that every scene loads. A key whose emitter is in `static` therefore shows on any map.
- **Found: a missing key is harmless.** The game's own Leever graph emits `Upheave` with sound on, and the Leever's sound user has no such key. So a shared graph can name a key that some enemies lack.
- **Found: the one usable shared key.** Among keys many enemy users share, only `デクの実有効` ("Deku Nut worked") plays a resident emitter and reads as a cue: `RctHit_DekuNutStun_00`, the white Deku Nut flash, on the head bone, with no condition. 42 enemy users have it: Amos, Baby Dodongo, Bari, Beamos, the Bubbles, Deku Babas, Dinolfos, Dodongo, Gohma (boss, egg, larva), Floormaster, Wallmaster, Lizalfos, Big Octo, Gerudo thief, both Moblins, Octorok, Peahat larva, the Scrubs, the Skulltulas, Stalchild, big Stalchild, Stalfos, weak Stalfos, both Tektites, Torch Slug, both Wolfos. Not covered: Keese, Leever, Like Like, ReDead, Gibdo, Iron Knuckle, Poes, Peahat, Freezard, Spike, Shell Blade, Stinger, Guay, Biri, Shabom and the bosses. No enemy sound user has the key, so the cue is silent.
- **Found: better effects need the effect-link file edited.** The player's heal sparkle (`Ply_Heal_HP_01`) and the fairy pickup sparkle (`Obj_Fairy_Get_00`) are resident too, but no enemy user has a key for them. Adding one means rewriting the shared effect-link binary (new call entries, sorted index, table offsets, size table). That is a data edit, but one mistake breaks effects for the whole game, so it was not attempted.
- **Built.** `difficulty/fx.rs`. The two create requests now sit in a Parallel with one flash node (`OneShotXLinkSearchAndEmit`, key `デクの実有効`), so the caller flashes once at the moment it calls. The wake of a sleeping helper is unchanged. Nodes added per caller: 30 (was 28). No new setting: it comes with "Enemies call for backup".
- **Tested.** The caller test now checks the flash beside the two requests; the combined test checks three flash nodes per caller with all three settings on. `cargo test -p ootrr-core difficulty` (191) and `settings` (106) pass; not run in the game.
- **How to test in the game.** `<playtest folder>\fx\PLAN-r2.md` step 5: a Lizalfos flashes on its head in the second its helper appears, once only.

## Effects, round 3 (20 Nov, branch `feature/fx-farore`, commit a0ddefc)
- **Unchanged on purpose.** The backup call keeps the white Deku Nut flash (user, 20 Nov: only heals get Farore's Wind). With "Enemy regen" or "Fairies help enemies" on, the build now also edits the shared effect-link file (see `09-enemy-regen.md`, round 3); the backup cue does not use the new key, and with backup on its own the file is not touched.
- **How to test in the game.** `<playtest folder>\fx-farore\PLAN.md` step 5: the Lizalfos caller still shows the white flash, not the green effect.

## Release (rc 16, 20 Nov)
In rc 16: harmless scrubs (business, stage, leaf scrubs and the Deku Scrub brothers) never call; the caller shows the white Deku Nut flash when it calls.

- **By script** (checks-1120, 20 Nov): a business scrub targeting Link called nobody in 40 s. The fighting Deku Scrub (`Enemy_Scrub_Deku`) still calls a Skullwalltula, as designed (only scrubs that do not fight are left out).
- **Not played:** the stage and leaf scrubs and the brothers specifically; the call flash was not looked at on its own.


## Moved from STATUS.md (20 Nov): backup results

  - Backup: Wolfos and Bari bring 1 helper, not 2; spear Moblin called nobody in 40 s; Stalchild, Skulltula, Wallmaster, Armos and Biri still call nobody in their home maps (45 s).


## Difficulty ladder (21 Nov, `fix/playtest-misc`, on `playtest/next` only)
User, hands-on: a Stalfos calling Stalfos and a Wolfos calling Wolfos is too tough; a caller should call a lesser kind. Each caller now calls the kind one step below it on this ladder (code: `HELPERS` and the pairs in `backup.rs`): Keese, Gohma larva, Lizalfos, Wolfos, Stalfos, White Wolfos, Dinolfos, Iron Knuckle. In the code: Dinolfos calls White Wolfos, White Wolfos calls Stalfos, Stalfos calls Wolfos, Wolfos calls Lizalfos, Lizalfos calls Gohma larvae, Gohma larvae call Keese. Four helper packs are new (Gohma larva, Stalfos, White Wolfos, Dinolfos); the pack table is 19 entries.
Tests: unit and game-file tests only. By hand: the panel step on 21 Nov was marked failed with no note; what failed is not known. Treat the ladder as not confirmed in the game.
