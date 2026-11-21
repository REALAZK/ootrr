# Lose heart on death

Setting `lose_heart_on_death` (number 116), checkbox. Code: `crates/core/src/difficulty/flows.rs` (`lose_heart`).

## Request
"lose heart on death (checkbox) when checked, each time you get a gameover, you lose 1 of the max hearts. down to a min of 3."

After playtesting rc 1: "losing a max-heart on death didn't work. I think you'll need to change the behavior so instead of showing the 'continue from last save / go to title' screen, the behavior is different... maybe it shows the gameover screen then auto-teleports you to the recent checkpoint. is this even possible?"

## Interpretation
Each game over you continue from removes one heart container, never below three. Being revived by a bottled fairy is not a game over.

## Investigation
- The game over scene is the flow `Event/EventFlow/DmF_SY_GameOver.bfevfl.zs`. Choosing Retry runs one event that sets the result to "Retry".
- Max health is game data `PlayerStatus.MaxLife`, in quarter hearts (3 hearts is 12). Flows can add to or compare game data numbers.
- **After your playtest:** the retry itself is `Sequence/Retry.module.ainb`. It reloads the last save, which is why the loss vanished. But it first checks a flag, `Sequence_RequestLastGameOverSave`; when set, the game saves its current state before reloading. That is the game's own "keep what happened" switch.

## Implementation
On Retry the flow now does three things before carrying on:
1. Sets `Sequence_RequestLastGameOverSave`.
2. Checks whether `PlayerStatus.MaxLife` is above 12.
3. If so, subtracts 4.

Choosing "return to title" is untouched. So no teleport was needed, which avoids the fight-state problems the repo's Navi warp ran into.

## How I tested it
- Unit test: the three added events are wired in the right order and the title path is unchanged.
- The game-file test stages the real flow; `flow-loads` is clean.
- rc 1 in the game, by you: **the loss did not stick** (before the save flag was added).

## In the game
rc 2's fix has not been played, and this is the fix I am least sure of. To test:
- Start with 5 or more hearts. Save. Pick up a few rupees so you can tell whether progress is kept. Die, choose Retry.
- Pass: one heart container fewer, and the rupees you picked up are still there.
- Repeat until 3 hearts and die again: it must stay at 3.
- Die with a fairy in a bottle: no heart lost.
- Report where you respawn. I expect the usual checkpoint, since the saved position is not changed.

## Notes
- Side effect by design: with this on, a game over keeps everything you collected since your last save, like the original game.
- If the save flag turns out not to work, the fallback is your idea: revive in place or warp, using the pieces the fairy revival and fall recovery already use in this same flow.

## Round 4 in the game (feature/heart-v4, source-built Longshot, 6 starting hearts)
Reading `status` life/max_life every half second. Death spot: Deku Tree baba room, see RECIPE.md "Known spots to take damage".
- Baseline, no mod: death, Try Again, 24/24 about 9 s later, spawn at the Deku Tree entrance.
- A (4 s wait after the Retry result event): no extra delay. Steps added after the result event never run; that event ends the flow.
- B (set max life 20 between the 60 frame wait and opening the Game Over screen): life 0/24 at death, 20/20 about 3 s later with the screen up (the action works while dead and also heals), after Try Again 24/24 again. The reload restores the saved value.
- C (B plus the save flag at the same place): same, 24/24, same spawn as baseline.
- D (max life 20 plus flag at the very start of the death entry): Link healed to full while dying and the death scene froze with no Game Over screen. Wrong as a feature.
- G (flag at the start of the death entry only when not revived by a fairy, heart ladder after the wait): Game Over screen shows 20/20, Try Again gives 24/24 at the entrance.
- H (G plus `GameSystemActor.EventTriggerRequestAutoSave` before the screen opens): 24/24 again.
Conclusion: anything written to the live game data is thrown away by Try Again, because `Sequence/Retry.module.ainb` loads the save (`OnLoadGameSaveData`) and then resets life. The save flag and an autosave request did not change that. Not working yet.

## Round 5: what the files say (code only, not yet played)
Retry path, in order (`Sequence/EventToMain` -> `Sequence/Retry`): the Game Over flow sets `Sequence_EventResult` = "Retry" and ends; `EventToMain` reads it, and if `IsSkipRetrySequence` is set it only releases the pause (no reload, `Sequence_LatestRetryType` = Skip), else it runs `Retry.module`: it picks the latest save index, shows the loading fade, closes the Game Over screen, then `ChangeBanc`. After the old map is destroyed: if `Sequence_RequestLastGameOverSave` then stop the safety system, run the AutoSaveMgr "last game over save", clear the flag; then `OnLoadGameSaveData` (reset game data types 3, 0, 8, then load the save, or the temporary save when `System_IsSaveTemporaryStorage`; then `AfterLoadSaveData`, which sets `Sequence_LatestRetryType` = None); then Life is set to 12 if it is below 12; last, `Sequence_LatestRetryType` = Normal.
What survives: game data has a reset mask (`reset`) and a save group (`save`) per flag (`data/flags.json`). The reload resets types 0, 3 and 8 (bit values 1, 8, 256) and then loads only flags with a save group, so a flag with `save` -1 and no 1, 8 or 256 bit in `reset` keeps its value. `PlayerStatus.MaxLife` has reset 8, save 0, so it is reset and reloaded: that is why every earlier attempt vanished. Survivors we use: `Sequence_ReservedBootEventName` (String64, reset 0, save -1), `Sequence_LatestRetryType` (Enum, reset 0, save -1), `DungeonInfo.GerudoFortress.BossRetryNum` (Int, reset 160 = types 5 and 7, save -1; Gerudo Fortress has no boss, so nothing else touches it). `BossRetryNum` for the other dungeons also survives a reload, so the boss retry counter is not lost. `GameOverReason` and `IsBossBattle` are reset by the reload (reset 8).
What runs after the reload: `AfterChangeBanc` (every ChangeBanc, including Retry's) requests the event named in `Sequence_ReservedBootEventName`; the name is kept through the banc load, and `AfterEvent` clears it afterwards, so it runs once. The vanilla user is `ShieldHylianNaviHintBootEvent`, set by `AfterLoadSaveData`. That flow starts with `EventTransition` to `DefaultBootEvent` (waits for the loading fade to close) and ends the usual way.
Saving while alive: `GameSystemActor.EventTriggerRequestAutoSave` is used 73 times in event flows with Link alive (item gets, shops, `Dm_SY_BancChangeArrive`). `AfterChangeBanc` itself requests an autosave (reason 2) after each scene change. The earlier failure was at the Game Over screen, with Link dead.

## What the code does now (v5a and v5b)
- Game Over flow: before the "Retry" result, `Sequence_ReservedBootEventName` is set to `ShieldHylianNaviHintBootEvent`. Title choice untouched.
- `ShieldHylianNaviHintBootEvent` flow: a gate in front of the entry. If `Sequence_LatestRetryType` is Normal: run `DefaultBootEvent`, then add 5 rupees (proof step, to be removed once the feature is confirmed), then the heart logic, then end. Any other boot runs the original chain unchanged.
- v5a (`feature/heart-v5a`): heart logic = max life one heart lower down to three hearts, then `EventTriggerRequestAutoSave`.
- v5b (`feature/heart-v5b`, same code, `KEEP` = Count): the Game Over flow stores "max life minus one heart" in `BossRetryNum` of Gerudo Fortress; the boot flow cuts max life down to it when it is larger. Nothing is saved; the loss lasts until you quit to the title.
- Side effect: the max life action fills health, so Link returns with full health. If the heart logic does nothing, the 5 rupees still show that the after-reload step ran.
- Tests: `<playtest folder>\heart\V5-TESTS.md`. Unit tests cover the wiring of both variants. `flow-loads` is clean.

## Ranked designs
1. Boot event after the reload, then autosave (v5a). Least state; needs the autosave to be accepted in the boot event (high chance, vanilla does it).
2. Boot event after the reload, remembered level in a surviving flag (v5b). Independent of saving. Loss is lost if you quit to the title and load.
3. Revive in place: set `IsSkipRetrySequence` on Retry and run the fairy revival chain already in this flow (no reload, so live max life is the counter and the next autosave keeps it). Not built: Link's death state and camera have to be put back by hand and the game only does this for boss rush retries.
Unproven in the game: that the boot event starts after a Retry reload, that `EventTriggerSetMaxLife` works in it, and that the autosave sticks.

## Variant test

Round 5, windowed, fresh save per variant, starting hearts 6, damage spot in the Deku Tree baba room (teleport confirmed before every warp). Values are max_life in quarter hearts / rupees. Start: 24 / 0.

| Moment | v5a | v5b |
|---|---|---|
| Game Over screen, 1st | 24 / 0 | 24 / 0 |
| 1st Try Again, +0 / +3 / +10 / +20 s | 20 / 5 | 20 / 5 |
| Game Over screen, 2nd | 20 / 5 | 20 / 5 |
| 2nd Try Again, +0 / +3 / +10 / +20 s | 20 / 5 | 16 / 5 |
| Result | FAIL (second death lost no heart, no rupees: hook did not run again) | PASS (24, 20, 16) |

Notes: play resumed normally in both (Link at the Deku Tree entrance, full new health, no frozen screen). Sequence_ReservedBootEventName read empty at every sample; Sequence_LatestRetryType changed value after the first death. v5a persistence check and the fairy check: NOT TESTED (v5a failed; fairy not supplied by give).

## Final design (branch `feature/heart-final`)
- Game Over flow, Retry only: before the result, `Sequence_ReservedBootEventName` is set to `ShieldHylianNaviHintBootEvent`, and "max life minus one heart" (never below 3 hearts) is stored in `DungeonInfo.GerudoFortress.BossRetryNum`, which survives the reload.
- After the reload the boot flow (gated on `Sequence_LatestRetryType` = Normal) waits for the usual boot event and cuts max life down to the stored value if it is larger. Nothing is saved. The 5-rupee proof step is gone.
- Fairy revival does not pass through the Retry result, so it costs nothing by construction. Not tested in the game.
- Settings text and tests: `cargo test -p ootrr-core difficulty` (110) and `settings` (107) pass.

## Final test (6 starting hearts, windowed, fresh save, autosave variant `final-save`, twice with the same result)
| Moment | max_life / rupees |
|---|---|
| Before 1st death | 24 / 0 |
| After 1st Try Again, +0, +3, +10, +20 s | 20 / 0 |
| After 2nd | 16 / 0 |
| After 3rd | 12 / 0 |
| 4th death, Game Over screen and after Try Again | 12 / 0 (floor holds) |
| Quit, relaunch same save folder, continue | 24 / 0 (loss did not persist) |
Link returns at the Deku Tree entrance with full new health each time.
- The autosave variant did not make the loss stick, so the shipped build omits it. The final exe (no save) was not replayed in the game: it is `final-save` minus one autosave action, and its logic matches the passed v5b.
- Not tested: fairy revival, persistence by any other method (revive in place would be needed).
- A mid-run alt-tab menu was reported by the user; the first run was repeated with a driver that checks for non-play state and no unexpected state was seen.

## Final build, played
`<playtest folder>\heart\final\ootrr.exe`, starting_hearts 6, two deaths: max life 24 -> 20 -> 16, rupees unchanged at 0. PASS. `give` lists 85 items (swords, shields, tunics, boots, quantities, upgrades, medallions, songs); there are no filled bottles: `give Bottle` gives an Empty Bottle, `Fairy` is ambiguous (Fairy Slingshot, Fairy Bow, Fairy Ocarina), `"Bottled Fairy"` does not exist, and the client has no `items` command.

## Code review reading of the lost persistence (18 Nov, no game run)
From `docs\code-review-2026-11-18.md`, open problem (c). The in-session design is sound (the stale counter is always either overwritten or already correct). The loss can only persist if the cut value reaches a real save, and the only test (`final-save`) quit straight after the respawn.
- Test it proposes: after a death and Try Again (24 down to 20), do an ordinary later save (walk through a loading zone, or save by hand), then quit and continue. If the loss is kept, an ordinary save already persists the cut: the second death reads the cut value from game data, so the player actor does sync it. The remaining hole is only "die, quit at once".
- Why the autosave variant most likely failed: `AfterChangeBanc` had just autosaved, or the request came while the boot event was still running.
- Suggested fix, data only, if the hole matters: write `PlayerStatus.MaxLife` with `EventTriggerSetGameDataInt` next to the player action (both then agree), wait some frames, then request the autosave. If no save keeps the cut, the fallback is the revive-in-place design already ranked third above. `Sequence_RequestLastGameOverSave` was tried and did not help.
- Not built in the review-fixes branch: it needs a wait step in the boot flow that I could not test without the game. The leftovers `Keep::CountSave` and `AUTOSAVE` in `flows.rs` stay as the hook for this experiment.


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 07):** Lose heart on death
- **State:** Done, with one limit
- **What was seen in the game:** 6 → 5 → 4 → 3 hearts over three deaths; a 4th death stays at 3; no side effects. The final build was replayed: 6 → 5 → 4 (script, 18 Nov). Confirmed by hand too (user died without a fairy: back with 5 hearts). Merged into `playtest/difficulty-all`
- **Next step:** Limit: the loss does not survive quitting and continuing (accepted as "simplest for now"). Fairy revival costs no heart: confirmed by the user (revived at 6 hearts max)
