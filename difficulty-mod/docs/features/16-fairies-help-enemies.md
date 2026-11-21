# Fairies help enemies too

Setting `fairy_helps_enemies` (number 125), checkbox. Code: `crates/core/src/difficulty/fairy.rs` and `flows.rs` (`fairy_used`). Built by a sub-agent on `feature/fairy-helps-enemies` (`df5f039`).

## Request
"fairies heal / respawn enemies. if a fairy gets used, all the enemies also heal, and the most recently defeated enemy gets resapwned"

## Interpretation
"Used" means releasing a bottled fairy to heal, or being revived by one on death. Two halves: (a) every living enemy nearby heals to full, (b) the last enemy you defeated comes back. I told the agent (b) was probably much harder and a solid (a) alone was acceptable.

## Investigation
- **(a)** needs a signal from the fairy events to every enemy. Events can set a game data flag; enemy AI can read one. The flag chosen is `MiniGame_Diving_Test`: it exists, is unsaved, defaults to false, and nothing in the game refers to it.
- "Heal to full" is the same heal node regen uses, in mode 1. The player's "heal to max" event is wired exactly that way.
- **(b)** A defeated enemy is deleted, so none of its own logic is left to run. No event action or AI node can name "the last one beaten" or recreate a placed enemy. A "this room's enemies come back" variant had no safe hook either.

## Implementation
- **Events raise the flag:** one added event in the release-fairy flow and one in the fairy revival chain of the game over flow.
- **Enemies poll it:** the shared enemy graph (the one regen uses) gains 15 nodes. While the flag is set, a living enemy heals to full. A 30-frame timer then clears the flag.
- Enemies clear the flag, not the event, because it is unknown whether enemy AI runs during the event.
- **(b) is not implemented.**

## How it was tested
- 26 difficulty tests, with node shapes compared against game-authored nodes.
- A real build: both flows carry the new event, `flow-loads` is clean, and with lose-heart and no-bottles also on each flow carries all edits.

## In the game
Never played. To test:
- Turn this on with enemy health at 3x. Damage a Stalfos or Lizalfos, then release a bottled fairy nearby.
- Pass: the enemy takes a full set of hits again.
- Also die with a fairy in a bottle next to a damaged enemy.
- Touching a wild fairy should not heal enemies.

## Notes
- If no enemy is awake when the fairy is used, the flag stays set and the next enemy to wake heals instead. A reload clears it.
- Not covered: Beamos and other enemies that do not run the shared graph.
- Must be applied after regen in the build (it wraps whatever it finds; regen expects the original graph).
- For (b), the control bridge's memory access or a code patch would be the only routes I can see.

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
Same change as in 09: the poll switch is now an extra child of the Parallel that holds the damage colour module call in each caller graph, and the module graphs stay as the game has them. A caller counts as already patched when it reads or clears `MiniGame_Diving_Test` (vanilla callers use the same game data nodes for other flags). Test: the all-8-combinations test described in 09. In the game: see `<playtest folder>\review-fixes\TESTS.md`.

## Fairy circling the enemy (19 Nov, investigated, nothing built)
You, after seeing the heal work: "can we get it to play the fairy spinning around the character animation on all enemies that they're healing, too?"
Searched every AI graph in the game (loose files and those inside actor packs) for effect nodes:
- The circling fairy on Link is made by the player's own nodes (`EventPlayerOutBottleFairy`, `EventTriggerPlayerUseFairy`, `OneShotPlayerUseFairy`) and the `Dm_SY_OutBottleFairy` event, all tied to the player actor.
- The only generic effect nodes (`EventNodeEffectEmitOneShot`, `OneShotGenericEmitEventEffectInputActor`) live in the `EventEffectEmitter` actor's event graph and run only inside an event flow, which an enemy graph cannot start.
- Enemy graphs have only effect nodes made for one actor (Peahat, Wallmaster and similar), plus `OneShotEnemyHandleCreateEffectAndFadeIn`, the spawn fade-in, which is not a heal effect.
- No node plays a named effect at an arbitrary actor from an enemy graph, so no data-only route exists. Nothing was built. The remaining routes need a code patch or the control bridge, which the project rules forbid.

## Effects experiment (19 Nov, branch `feature/fx-r1`, investigated, nothing built)
- **Request.** Use particle effects to show a regen tick, a fairy heal and a backup call.
- **Found.** The only generic effect node enemy graphs use is `OneShotXLinkSearchAndEmit` (170 uses; props `ExecuteEveryFrame`; inputs `ELinkCall`, `SLinkCall`, `XLinkKeyName` as a constant string; output `HandleSet`). Example: `Enemy_Leever.utility.postsensor.root` emits `Upheave`. The key names (`Death_Start`, `Wakeup`, `Dig`, ...) are looked up in the actor's own XLink user, which lives in the global effect-link resource, not the actor pack. Notes from another modder confirm that particle sets belong to actor bundles and that a set not loaded for the scene does not appear.
- **Not built (time box).** A key every enemy's XLink user has, and that reads as a heal or alert, was not identified; emitting a key such as `Death_Start` on a heal would read as dying. Next step: read the global XLink resource, list the keys shared by every enemy user, pick one, and add one `OneShotXLinkSearchAndEmit` beside the heal / call nodes (no blackboard keys needed).

## Effects, round 2 (20 Nov, branch `feature/fx`, built, not run in the game)
- **Request (19 Nov).** Show a visible effect when a fairy heals an enemy.
- **Found: how effect keys resolve.** The whole game has one effect-link file (`ELink2/elink2.Product.110.belnk.zs`, 1267 users) and one sound-link file (`SLink2/slink2.Product.111.bslnk.zs`, 1216 users). Each actor names its user in its pack (`Component/ELink/<actor>`, `UserName`, normally the actor's own name). `OneShotXLinkSearchAndEmit` looks the key up only in the running actor's own user. There is no common user an enemy graph can emit through: `ReactionHit`, `ReactionField` and `Chemical` users exist, but the engine's reaction system plays them, not a key from an AI graph.
- **Found: when an effect shows.** A key's asset names an emitter set (`RuntimeAssetName`), and the set is found by name among the loaded effect bundles. Bundles belong to actors (`Effect/EffectFileInfo`, `EsetbList`), plus one resident bundle, `static` (`StaticEsetb`), that every scene loads. A key whose emitter is in `static` therefore shows on any map.
- **Found: a missing key is harmless.** The game's own Leever graph emits `Upheave` with sound on, and the Leever's sound user has no such key. So a shared graph can name a key that some enemies lack.
- **Found: the one usable shared key.** Among keys many enemy users share, only `デクの実有効` ("Deku Nut worked") plays a resident emitter and reads as a cue: `RctHit_DekuNutStun_00`, the white Deku Nut flash, on the head bone, with no condition. 42 enemy users have it: Amos, Baby Dodongo, Bari, Beamos, the Bubbles, Deku Babas, Dinolfos, Dodongo, Gohma (boss, egg, larva), Floormaster, Wallmaster, Lizalfos, Big Octo, Gerudo thief, both Moblins, Octorok, Peahat larva, the Scrubs, the Skulltulas, Stalchild, big Stalchild, Stalfos, weak Stalfos, both Tektites, Torch Slug, both Wolfos. Not covered: Keese, Leever, Like Like, ReDead, Gibdo, Iron Knuckle, Poes, Peahat, Freezard, Spike, Shell Blade, Stinger, Guay, Biri, Shabom and the bosses. No enemy sound user has the key, so the cue is silent.
- **Found: better effects need the effect-link file edited.** The player's heal sparkle (`Ply_Heal_HP_01`) and the fairy pickup sparkle (`Obj_Fairy_Get_00`) are resident too, but no enemy user has a key for them. Adding one means rewriting the shared effect-link binary (new call entries, sorted index, table offsets, size table). That is a data edit, but one mistake breaks effects for the whole game, so it was not attempted.
- **Built.** `difficulty/fx.rs`, the same check as regen (see `09-enemy-regen.md`): when the fairy flag is seen, a hurt enemy gets the heal to full plus one flash; an enemy already at full gets the plain heal and no flash. Nodes added per caller: 19 (was 14). No new setting: it comes with "Fairies help enemies".
- **Tested.** As in 09: `cargo test -p ootrr-core difficulty` (191) and `settings` (106) pass; not run in the game.
- **How to test in the game.** `<playtest folder>\fx\PLAN-r2.md` step 4: hit a Wolfos and an Octorok, release a fairy: each flashes once and is back to full; an unhurt Moblin does not flash. The circling-fairy effect stays out of reach (see above).

## Effects, round 3: Farore's Wind (20 Nov, branch `feature/fx-farore`, commit a0ddefc, built, not run in the game)
- **Request (20 Nov).** Show Farore's Wind's green effect on enemies when they heal.
- **Built.** The fairy heal now emits the new key `OotrrHeal` instead of the Deku Nut key; the build adds that key to 114 enemy effect users in the shared effect-link file, playing `Ply_Magic_FaroreWind_Trigger_00` (resident, so on every map) for 90 frames on a body bone. Details, layout and risk in `09-enemy-regen.md` (round 3). Enemies that had no Deku Nut key (Keese, Leever, Like Like, ReDead, Gibdo, Iron Knuckle, Poes, Peahat, Freezard and others) now show it too. No new setting: it comes with "Fairies help enemies".
- **Tested.** As in 09; not run in the game.
- **How to test in the game.** `<playtest folder>\fx-farore\PLAN.md` step 4: hit a Wolfos and a Keese, release a fairy: each shows the green effect once and is back to full; an unhurt Moblin shows nothing.

## Release (rc 16, 20 Nov)
In rc 16: `fx.rs` and `elink.rs` with the full Farore's Wind sequence (commit 17134e6: ring at the feet, column at the chest, orb at the head, 240 frames, scaled by enemy size). No new setting.

- **Played by hand** (session 6, 20 Nov): the game's ordinary effects still work after the shared effect-link edit; the full green heal shows on enemies, including the full heal from a released fairy.
- **By script:** the effect-link round trip and per-user tests (`cargo test -p ootrr-core difficulty`).


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 16):** Fairies help enemies
- **State:** Seen working
- **What was seen in the game:** Confirmed in the game: the fairy healed the enemy (19 Nov)
- **Next step:** Circling-fairy visual on enemies not possible: no enemy-side effect node (see feature file) fx-r1 (19 Nov): heal effect investigated, not built (same reason).
(fragment kept from the old table) eview-fixes, not yet seen in the game) |
(fragment kept from the old table) | 17 | Enemies call for backup | rc 15: calls a LESSER kind, at the caller's spot (built, not played). Earlier: Option 3 seen in the game by script (19 Nov): no crash on 32 callers; 19 pairs gave the expected helper once (15 to 30 s), 2 gave one helper where two were expected (spear Moblin, Bari), 9 callers called nobody in 75 s (Stalchild, Freezard, Skulltula, Wallmaster, Amos, Spike, Tailpasaran, Biri) , no helper called in turn | Results in `<playtest folder>\backup-r3\RESULTS.md`; Gibdo, weak Stalfos and white Bubble not spawnable in the field fx-r1 (19 Nov, 91c3d0a, code only): never calls its own kind, a weaker one; helper appears at the caller, raised 0.5; no Dodongo kinds as helpers; flying pot and Shabom dropped (no backup keys).
(fragment kept from the old table) eview-fixes, not yet seen in the game) |


## Moved from STATUS.md (20 Nov): session 5, green heal

- **Session 5 (20 Nov evening), seen by the user:** the green heal works ("pretty good") but shows only part of Farore's animation. Recorded the full cast (about 4 s: a ring at the feet, a spiral column, an orb rising) in `<playtest folder>\handson-5\notes\clip-172926.mp4`. Fix built (``17134e6``): each heal emits the ring at the feet, the column at the chest and the orb at the head, for 240 frames, scaled by enemy (1.5 normal, 2.0 Lizalfos, 2.5 Wolfos and Moblins, 3.0 big ones). Not seen yet. The same clip shows randomizer v2 Shaboms in Jabu-Jabu.
