# Enemies regen health

Setting `enemy_regen` (number 118), checkbox. Code: `crates/core/src/difficulty/regen.rs`. Built by a sub-agent on `feature/enemy-regen` (`d582934`).

## Request
"enemies regen health"

## Interpretation
While alive, enemies slowly recover health, never above their maximum. I left the rate to the agent, asking for something small every few seconds.

## Investigation
- No parameter for regeneration exists, so it had to be added to enemy AI.
- Nearly every enemy's AI calls one tiny shared graph, `AI/EnemyCommon.common.reactiondamagecolor.module.ainb`, in the global AI pack. Stalfos use an identical private copy in four packs. That graph is the hook: one edit reaches about 123 actors.
- The heal itself is the node `OneShotLifeSetOwnLife`. The player's "heal" event uses it with mode 0 (add and clamp to max), and a Floormaster graph proves it works on enemies.
- A looping frame counter node is used the same way in Twinrova's own graph.

## Implementation
- The graph gets a new root that runs the old node alongside a 150-frame looping timer. Each time it fires: has a life component, is not dead, add 1 health.
- Every actor that runs the graph gets a little more AI memory (`InstanceHeapSize` + 0x4000), as the Bunny Hood feature does.
- Size table entries grow for the graphs and packs.

## How it was tested
- rc 13 (19 Nov, merged build, script, `playtest
c13\RESULTS.md`): a hit Stalfos regained +1 hp 15.2 s and 15.0 s after the hit, then every 4.9 and 5.4 s. The source of the hits was not identified (19 sword swings did no damage).
- 22 difficulty tests, including checks that each added node has the same shape as a game-authored node of the same kind.
- A real build with health at 2x and regen on; the built graph has 11 nodes in both places.
- Found and worked around a bug: the repo misreads the compression dictionary of the global AI pack.

## In the game
Never played. To test:
- Turn this on with enemy health at 3x or more, so there is health to regain.
- Hit a Stalfos or Lizalfos a few times, back off for 30 seconds, then count the hits to kill it against a fresh one.
- Pass: it takes noticeably more hits after the wait.
- Watch bosses with health phases (Volvagia, King Dodongo, Twinrova): healing back over a phase line may replay a phase.
- A crash when any enemy loads means the graph edit is rejected; report the map.

## Notes
- Rate is one point per 150 frames, about five seconds if the game counts 30 per second. That assumption is unverified.
- Enemies with 1 or 2 health show no effect.
- Not covered: Beamos, Anubis, flying pots, Shabom, Peahat larvae, Gohma eggs, and boss parts such as Morpha's tentacle.
- This shared graph is the best hook found for "make every enemy do X". Fairies-help-enemies and call-for-backup reuse it. Order matters: regen must be applied first.

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

## Review fix: nodes live in the caller graphs (branch `feature/review-fixes`)
The code review (finding 1) read the red tint as the wrapper: each feature made a Parallel the root of the damage colour module, so the module could never return and the colour node might never be left. What the files show: the module is one native node; the callers (16 shared graphs in the AI pack, and 85 actor packs that carry their own graph) run it as one child of a per-frame Parallel next to other modules, and none of them reads a result from it or does anything when it returns. So the module is unconditional and not "only entered on a hit", but whether a Parallel that never ends changes how the native node behaves cannot be told from the files.
- Built: the module graphs (`EnemyCommon...reactiondamagecolor` and the Stalfos one) are no longer edited, in any combination. Regen adds its looping timer as one more child of the Parallel that holds the module call, in every caller graph (`difficulty/hook.rs` is the one shared helper, all three features use it). Nodes are the same as before minus the old wrapper. Heap growth is unchanged.
- Test: `the_vanilla_damage_color_module_is_untouched_in_every_combination_of_the_three` builds all 8 on/off combinations of regen, fairy and backup and checks that both module graphs are byte-identical and the call itself is untouched.
- In the game: with regen, fairy and backup on, are enemies normal-coloured before and after being hit; does regen still heal. See `<playtest folder>\review-fixes\TESTS.md`.

## Wait before healing (branch `feature/regen-wait`, code only)
The user fought two Stalfos with regen on: "they're recovering health so fast I can't kill them!" Decision: "10 seconds wait, 1 hp every 5 seconds". An enemy starts healing only after 10 seconds without being hit, then heals 1 point every 5 seconds until full. A new hit restarts the 10 seconds.
- Hit signal: `QueryLifeIsExternalDamaged` (the life component's "damaged from outside" flag), no keys, no blackboard. Vanilla precedent: the Stalfos root graph (`AI/Enemy_Stalfos.utility.root.ainb`, element 5 and the Switch 38 that reads it) sits in the same kind of per-frame Parallel we host in; `EnemyCommon.common.ragdolldie.module` and Freezard, Wolfos and Bubble graphs use it the same way. The nodes are copied from those; a test compares their shapes.
- Wait: a Switch on the hit query. True runs an idle node. False runs a 600-frame non-looping `SelectorGenericFrameCounter` (60 frames a second). When a hit flips the Switch, the counter is left and starts from zero when it is entered again. Precedent for a counter under a Switch branch: `AI/Kokko.utility.root.ainb` (Switch 106, False to counter 60).
- Heal: the old looping 300-frame timer (`HEAL_EVERY`, 5 s) is the counter's Finish child, so it runs only after the wait and restarts with it. Its Finish runs the old guarded heal (has life, not dead, add 1). The first heal comes 15 s after the last hit (10 s wait, then one 5 s period), then every 5 s.
- Nodes added per hosted call: 14 (was 9). No keys are read or written, so the placement-key and blackboard problems of the backup feature do not apply; a test checks the count of Blackboard nodes in the patched caller is unchanged.
- Not verified in the game: that the flag is true on the frame an enemy takes a hit (the Stalfos graph reacts to it, so it should be), and that the Switch re-enters the counter from zero. To check: debug overlay enemy health on a Stalfos; hit it, 10 s of no change, then +1 per 5 s (first at about 15 s); hit again at 8 s and the wait restarts.

## Effects experiment (19 Nov, branch `feature/fx-r1`, investigated, nothing built)
- **Request.** Use particle effects to show a regen tick, a fairy heal and a backup call.
- **Found.** The only generic effect node enemy graphs use is `OneShotXLinkSearchAndEmit` (170 uses; props `ExecuteEveryFrame`; inputs `ELinkCall`, `SLinkCall`, `XLinkKeyName` as a constant string; output `HandleSet`). Example: `Enemy_Leever.utility.postsensor.root` emits `Upheave`. The key names (`Death_Start`, `Wakeup`, `Dig`, ...) are looked up in the actor's own XLink user, which lives in the global effect-link resource, not the actor pack. Notes from another modder confirm that particle sets belong to actor bundles and that a set not loaded for the scene does not appear.
- **Not built (time box).** A key every enemy's XLink user has, and that reads as a heal or alert, was not identified; emitting a key such as `Death_Start` on a heal would read as dying. Next step: read the global XLink resource, list the keys shared by every enemy user, pick one, and add one `OneShotXLinkSearchAndEmit` beside the heal / call nodes (no blackboard keys needed).

## Effects, round 2 (20 Nov, branch `feature/fx`, built, not run in the game)
- **Request (19 Nov).** Show a visible effect when an enemy regenerates a point of health.
- **Found: how effect keys resolve.** The whole game has one effect-link file (`ELink2/elink2.Product.110.belnk.zs`, 1267 users) and one sound-link file (`SLink2/slink2.Product.111.bslnk.zs`, 1216 users). Each actor names its user in its pack (`Component/ELink/<actor>`, `UserName`, normally the actor's own name). `OneShotXLinkSearchAndEmit` looks the key up only in the running actor's own user. There is no common user an enemy graph can emit through: `ReactionHit`, `ReactionField` and `Chemical` users exist, but the engine's reaction system plays them, not a key from an AI graph.
- **Found: when an effect shows.** A key's asset names an emitter set (`RuntimeAssetName`), and the set is found by name among the loaded effect bundles. Bundles belong to actors (`Effect/EffectFileInfo`, `EsetbList`), plus one resident bundle, `static` (`StaticEsetb`), that every scene loads. A key whose emitter is in `static` therefore shows on any map.
- **Found: a missing key is harmless.** The game's own Leever graph emits `Upheave` with sound on, and the Leever's sound user has no such key. So a shared graph can name a key that some enemies lack.
- **Found: the one usable shared key.** Among keys many enemy users share, only `デクの実有効` ("Deku Nut worked") plays a resident emitter and reads as a cue: `RctHit_DekuNutStun_00`, the white Deku Nut flash, on the head bone, with no condition. 42 enemy users have it: Amos, Baby Dodongo, Bari, Beamos, the Bubbles, Deku Babas, Dinolfos, Dodongo, Gohma (boss, egg, larva), Floormaster, Wallmaster, Lizalfos, Big Octo, Gerudo thief, both Moblins, Octorok, Peahat larva, the Scrubs, the Skulltulas, Stalchild, big Stalchild, Stalfos, weak Stalfos, both Tektites, Torch Slug, both Wolfos. Not covered: Keese, Leever, Like Like, ReDead, Gibdo, Iron Knuckle, Poes, Peahat, Freezard, Spike, Shell Blade, Stinger, Guay, Biri, Shabom and the bosses. No enemy sound user has the key, so the cue is silent.
- **Found: better effects need the effect-link file edited.** The player's heal sparkle (`Ply_Heal_HP_01`) and the fairy pickup sparkle (`Obj_Fairy_Get_00`) are resident too, but no enemy user has a key for them. Adding one means rewriting the shared effect-link binary (new call entries, sorted index, table offsets, size table). That is a data edit, but one mistake breaks effects for the whole game, so it was not attempted.
- **Built.** `difficulty/fx.rs`. The heal tick now goes through a check on `QueryActorLife`: `SelectorMathLessOrEqualS32` (Max <= Now) picks a plain heal when the enemy is at full health, and the heal plus the flash (`OneShotXLinkSearchAndEmit`, key `デクの実有効`, ELink only) when it is hurt. An unhurt enemy does not flash every 5 s. The heal runs in both branches, so a wrong life reading cannot stop regen. The selector reads the life query once and lists the link in plug group 5, as the game's Beamos graph does. Nodes added per caller: 19 (was 14). No new setting: it comes with "Enemy regen".
- **Tested.** `fx` tests: the emit node has the fields of the game's Stalchild emit node, the selector and the life query match the Beamos ones, and the key and its emitter are in the effect-link file and the resident set. Regen, fairy and backup tests updated. `cargo test -p ootrr-core difficulty` (191) and `settings` (106) pass with the game files; release build done. Not run in the game.
- **How to test in the game.** `<playtest folder>\fx\PLAN-r2.md`. Spawn a Stalfos, hit it once: about 15 s later a white flash on its head as it gains a point, then one every 5 s until full, none after. A Keese heals with no flash.
- **Question for you.** The flash is the Deku Nut stun flash. Clear enough as a heal cue, or should we try editing the effect-link file for the heal sparkle (riskier)?

## Effects, round 3: Farore's Wind (20 Nov, branch `feature/fx-farore`, commit a0ddefc, built, not run in the game)
- **Request (20 Nov, after seeing the flash).** "The green animation that happens when you use Farore's Wind: I want that on the enemies when they heal."
- **Found: where Farore's Wind's effect lives.** The player's effect user (`Player`, also `Player_Adult`) has a container key `FaroreTrigger` whose children `FaroreTrigger_00` .. `_02` play `Ply_Magic_FaroreWind_Trigger_02` (bone `Root`), `_Trigger_01` (both hands) and `_Trigger_00` (bone `Spine_1`), under conditions. The warp-out is `FaroreWind_Warp_00/01` (`Ply_Magic_FaroreWind_Warp_00/01`). All of these emitter sets are in the resident `static` bundle, so they show on every map; no bundle has to be added to an enemy.
- **Built: an effect-link reader and writer.** `difficulty/elink.rs` reads the shared effect-link file (`ELink2/elink2.Product.110.belnk.zs`, XLNK version 0x24) into sections: header, user hashes and positions, parameter definitions (kept as bytes), asset parameters (mask + values), the tables between (kept as bytes), each user (header, local properties and user parameters, sorted key index, calls, container parameters, trigger tables), conditions (kept as bytes) and the name table. Layout facts it relies on, all checked on the real file: users are stored back to back and their parts add up exactly; a user's trigger-table position is relative to the user; triggers name calls by byte offset from the call table; call parameters are offsets into the asset parameter table; names and conditions are offsets into their tables. Writing recomputes every position, count and size. The unchanged file rewrites byte for byte.
- **Built: the heal key.** When "Enemy regen" or "Fairies help enemies" is on, the build adds one asset call `OotrrHeal` at the end of each enemy user's call table (114 users: every `Enemy_*` user except the picture-book `_Dictionary` ones, bosses included), inserts it into the user's sorted key index, and appends one asset parameter per bone used (16 new). The parameter is a copy of the player's `Ply_Magic_FaroreWind_Trigger_00` parameter (matrix and scale as the player has them) with the key name, `Duration` 90 frames (a stored value already in the file) so the effect stops on its own even if it loops, and a bone the enemy already uses: `Spine_1` or `Spine_2` if it has one, else the bone of its Deku Nut key, else `Head`, `Skl_Root`, `Root` or `Body`, else its most used bone, else none. The name `OotrrHeal` is appended to the name table. Nothing else in the file moves except by the size of the new parameters; the size table grows with the file.
- **Built: the graphs.** The regen tick and the fairy heal now emit `OotrrHeal` instead of the Deku Nut key (same node, same hurt check: no effect on an enemy already at full health). Backup calls keep the white Deku Nut flash. No new setting.
- **Risk.** The effect-link file is global: if a shifted position were wrong, every actor's effects could fail, not only enemies. The tests rule out the known ways (byte-identical round trip, untouched users identical, every original parameter unchanged, all sorted indexes valid), but the game has not loaded the edited file yet. Also unknown until seen: whether `Ply_Magic_FaroreWind_Trigger_00` looks right on an enemy (it is made for Link, scale 1.5) and whether it is the green swirl the user means; swapping to `_Trigger_02` or `_Warp_00` is a one-line change (`HEAL_EFFECT`).
- **Tested.** `elink` tests: round trip of the real file, a new call lands in one user and leaves the rest alone. `fx` tests: every enemy user with the Deku Nut key (and Keese, Leever, Like Like, ReDead, Gibdo, Iron Knuckle, Poe, Peahat, Freezard) gets the key; it resolves to `Ply_Magic_FaroreWind_Trigger_00`, which is in the resident set; its bone is one the user already uses; untouched users are byte-identical; a second pass changes nothing; enemy variants (fire and ice Keese, respawning Stalfos and ReDead, fixed Freezard and Leever) name a covered user; the heal graph's emit node names `OotrrHeal`; the size table entry grows. `cargo test -p ootrr-core difficulty` (210) and `settings` (106) pass with the game files; release build done. Not run in the game.
- **How to test in the game.** `<playtest folder>\fx-farore\PLAN.md`: first a load test of many maps and of the player's own effects (the file is global), then a hurt Stalfos and a hurt Keese should show the green Farore's Wind effect at each +1 tick.

## Release (rc 16, 20 Nov)
In rc 16: `fx.rs` and `elink.rs` with the full Farore's Wind sequence (commit 17134e6: ring at the feet, column at the chest, orb at the head, 240 frames, scaled by enemy size). No new setting.

- **Played by hand** (session 6, 20 Nov): the game's ordinary effects still work after the shared effect-link edit; the full green heal shows on enemies.
- **By script:** the effect-link round trip and per-user tests (`cargo test -p ootrr-core difficulty`).


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 09):** Enemies regen health
- **State:** Built, waiting for a playtest
- **What was seen in the game:** Old timer seen (19 Nov): +1 hp every 2 s, too fast. Now 10 s without a hit, then +1 hp every 5 s (branch feature/regen-wait, playtest/regen-wait) rc13 (19 Nov, merged build, script): hit a Stalfos; +1 hp came 15.2 s and 15.0 s after the hit, then every 4.9 to 5.4 s. Matches 15 s then 5 s. Hit source unidentified (sword swings did no damage).
- **Next step:** Not seen yet: the hit flag resetting the wait fx-r1 (19 Nov): effect for regen ticks investigated, not built (`OneShotXLinkSearchAndEmit` found; no shared heal key yet).


## Moved from STATUS.md (20 Nov): round 2 results (effects, spawn combo, Zora ambush, fake pots)

| **Round 2 (agent 9, done)** | `<playtest folder>\round2-1120\RESULTS.md`. **Effects, seen:** 10 maps load. A hurt Stalfos flashes at its first regen tick (a blue-white burst on the head, about 0.17 s, clear when in frame, easy to miss in a fight; reads as "stunned"); an unhurt one never flashes. Regen 16 s after the hit, then every 5 to 6 s. Merged into `playtest/next` (e486cc5). Backup flash inconclusive (no helper came in 3 fights); fairy flash and Keese not tested. **Spawn combo, seen:** both stick clicks + D-Down, 12 slots, and a kill frees its slot (the slots are taken in turn, so the next press may hit a slot that is still full). **Cucco revenge off:** no swarm (control passed). **Zora ambush:** cave B passed; **cave C FAIL**: nothing dropped with Link 0.6 to 2.5 units from its wall Skulltula (11.3, 63.0, -3.8). **Fake pots:** one flying pot found in child Zora's Domain (36.2, 114.4, 8.6); it did not fly in 8 s, even after a sword swing (inconclusive). **Not tested:** limited ammo drops and grass and pot surprises (a script cannot find map-placed grass and pots), respawn sooner (Din's Fire kills did not register), room-clear door |
