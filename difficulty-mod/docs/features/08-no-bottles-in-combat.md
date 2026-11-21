# No bottles during combat

Setting `no_bottles_in_combat` (number 117), checkbox. Code: `crates/core/src/difficulty/flows.rs` (`no_bottles`, `refusal`).

## Request
"no bottles during combat (prevents using bottles while an enemy is aggro)"

After playtesting rc 1: "attempting to drink potion at any time doesn't work. also it just freezes me in place. it should play some sort of 'can't do that' animation. but most importantly: it should let me drink the potion if there's no nearby aggro enemy!! ... you can tell if 'in combat' by the music"

## Interpretation
While an enemy is after you, drinking from a bottle or letting out its contents is refused with a visible "no". Otherwise bottles work normally.

## Investigation
- Bottle use runs three flows: `Dm_SY_DrinkBottle`, `Dm_SY_OutBottle`, `Dm_SY_OutBottleFairy`.
- I found no "player is in battle" query. What exists: the game data flag `IsBossBattle`, and a sound query `EventQueryIsPlayingBgmName`. Ordinary enemies start `BGM_EnemyBattle` when they notice you.
- The sound query is not used by any shipped flow, so I read its parameter name (`BgmName`) out of the sound system's graph.
- **After your playtest, two bugs:**
  - True/false queries answer case 0 for true. I had it backwards, so bottles were blocked outside combat.
  - Ending the flow at its entry froze Link. Every vanilla path ends with `EventPlayerDrawnShieldAfterUseBottle`, so the refusal has to run it too.

## Implementation
Each entry of the three flows now starts with two checks: boss battle, then battle music. If either says "fighting", a refusal runs: start the event, play the head-shake animation (`EvtTalkNo`, the one Link uses to answer "no"), put the bottle away. Otherwise the vanilla flow runs.

## How I tested it
- Unit tests for the check order, case values and the three-step refusal.
- The game-file test stages all three real flows; `flow-loads` is clean.
- rc 1 in the game, by you: **inverted and froze.** Both fixed in rc 2.

## In the game
rc 2's fix has not been played. To test:
- With this on alone: drink a potion in a quiet place. Must work as normal.
- Get an enemy to chase you until the battle music starts, then try. Pass: head shake, bottle put away, you can move.
- Try in a boss fight, and try releasing a fairy or a bug in both situations.
- Things to report: any freeze, no animation at all, or bottles blocked for a long time after the fight (the music fading out).

## Notes
- Enemies that start no battle music (Stalfos, some minibosses) will not block bottles. You confirmed music is the right signal, so that gap follows from it.
- If you randomize music, the check still uses the name `BGM_EnemyBattle`; I believe the name is kept, but it is untested.
- The head shake is a guess at a "can't do that" gesture. If it looks wrong, the animation name is one constant, `REFUSE_AS`.

## Seen in the game
Not tested (batch 1 stopped early; no filled bottle was set up).

## Combat query research (19 Nov, agent combat-check): no data query found

Request: use the game's own combat check (the one that makes the Business Scrub refuse) instead of battle music, and show the game's notice when refused.

What the game does for the Business Scrub (all names are from the game's data):
- The scrub is a pouch item, "Item_ScrubBusinessSeed". Using it runs the player scheduler `Player.scheduler.usescrubbisinessseed.module` (selector node `SelectorPlayerSchedulerUseScrubBusinessSeed`, verb 61) and `Player.action.usescrubbusinessseed.module` (action node `ExecutePlayerUseScrubBusinessSeed`). Both are in `Player_Base/Child/Adult.pack.zs`. The selector and action are native nodes with no inputs that name a condition.
- The refusals are two notices in `LayoutMsg/SimpleInfo_00.msbt`: `CannotUseScrubBusinessSeedBattle` ("You cannot call the Business Scrub during combat.") and `CannotUseScrubBusinessSeedNoSpace`. The labels appear in no ainb, bgyml, byml or event flow anywhere in the romfs (searched Pack, AI, Event, GameParameter, Logic, Sequence, System, UI, Component, Banc), so they are chosen inside native code, together with the combat test. The combat test itself is therefore not a node, query or flag that data can call.
- The summoned scrub's own flow (`Enemy_Scrub_Business_Seed`) and AI contain no combat test.

What I searched for and did not find (a reusable combat query):
- Event flow queries (all 330 `EventQuery*` names in `Node.Product.111.aidefn`): none about battle, combat, enemies near or aggro. Closest: `EventQueryCheckPlayerState` (State enum 0..41, a player action state, no battle value seen in any flow), `EventQueryCheckTerrorLevel` (NPC panic), `EventQueryIsPlayingBgmName` (what we use now), `EventQueryUserPerceptionIsInsideSphere`.
- AI graph queries: `QueryGameBalanceClientHasMeleeBattlePermission*` (enemy attack-token system, per enemy, not "player is in combat"), `QueryGenericGetIsMultiBattleArea`, `QueryEnemyCounterDstActorCount`, `QueryPlayerIsOnNoResetFlag` and `QueryPlayerIsOnMainActionStatus` (enum values not in data; no battle flag seen set by any graph). None is reachable from a bottle flow.
- Save/game data: only `BossBattle` and `BattleOnShip` (already known `IsBossBattle`). `SeqQueryIsEnemyMuseum` is the Hall of Fallen Foes, not combat.

The notice mechanism IS reachable from data: `UISystemActor.EventTriggerRequestOpenSimpleInfo {Type=N}` (used by `Dm_SY_Ocarina` Type 11, `Dm_SY_Ocarina_EponaSong` 10, `WarpEvent` 14, `SubEvent_GetSwordBigGoron` 18 and 19). `Type` is a native enum (`game::ui::SimpleInfoType`); its value-to-label table is not in the data, so the number for `CannotUseScrubBusinessSeedBattle` is unknown (it would need to be probed in the game), and that text says "Business Scrub", which is wrong for bottles. A bottle-specific text cannot be added cleanly because the label is picked by native code from the enum.

Result: part A (reuse the scrub's combat query) is not possible from data. No code was changed. Decision for the user: keep the music check, or accept a different signal.


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 08):** No bottles in combat
- **State:** Done
- **What was seen in the game:** Confirmed by the user at the controls (18 Nov): a bottled fairy could NOT be used while the Iron Knuckle was aggro and COULD once far enough away; the refusal animation is right; a bottled fairy still revived Link automatically in the fight (6 hearts kept). Found and fixed on the way: the block ignored miniboss music (commit 630870b)
- **Next step:** Not seen against a full boss; low risk


## Moved from STATUS.md (20 Nov): request notes

- **Earlier bean-seed feedback, confirmed still wanted:** refusal notice when a bottle is refused in combat (probe of the notice numbers queued for a game slot).

- Combat check: NOT POSSIBLE with data. The scrub's combat refusal is decided in native code (`ExecutePlayerUseScrubBusinessSeed`, message `SimpleInfo_00` / `CannotUseScrubBusinessSeedBattle`); no event or AI query exposes it. Music check stays. A notice can be shown with `UISystemActor.EventTriggerRequestOpenSimpleInfo {Type=N}`, but N is a native enum; the scrub's notice says "Business Scrub". Option: probe the Type numbers in the game for a fitting existing notice. Findings in feature file 08.

## Refusal notice added (21 Nov, branch `feature/zora-bottle-fixes`)
A refused bottle now also shows the game's own notice, as you asked. How: after the player event starts, the refusal runs `UISystemActor.EventTriggerRequestOpenSimpleInfo` (the action the ocarina and Farore's Wind flows use for their refusals) with `Type` 23 (`NOTICE_TYPE`), then the head shake and the put-away as before. The text is not copied: Type 23 selects the label `CannotUseScrubBusinessSeedBattle` in `LayoutMsg/SimpleInfo_00.msbt`, the game's own "cannot use during combat" notice (it names the Business Scrub; the game has no bottle-specific one).
The Type numbers are native, so they were probed: Type 22 showed `CannotPutWarpPointOutsideDungeon`, so a Type is the label's position in that file plus 7 (Type 10 and 11 are the ocarina's Epona and warp refusals, 14 is Farore's Wind lacking magic).
The dupe-items refusal (feature 13) shows no text, only the head shake; it is unchanged.
Seen in the game (Kakariko, child, fairy bottle on the item button, build from this branch):
- Lizalfos spawned 9 units away, battle music playing, one use of the button: the notice appeared, Link shook his head, the fairy stayed in the bottle and in the slot, Link was free after about 0.4 s. Screenshot and 8 s clip in the test folder.
- Control, no enemy near: the same button released the fairy normally and Link held the empty bottle, no notice.
Not tested: boss fight, other bottle contents.
