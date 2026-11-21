# Cuccos take revenge at once

Setting `cucco_revenge` (number 131), checkbox, label "Cuccos take revenge at once", on the Difficulty page, default off. Code: `crates/core/src/difficulty/cucco.rs`. Built on `feature/surprises` (worktree `wt\surprises`, commit `f9b7c87`), code only, never played.

## Request
"hurting a cuckoo even a tiny bit triggers the cuckoo revenge sequence"

## Interpretation
The first damaging hit on a Cucco makes the flock attack, instead of the several hits the game wants. Picking up, carrying and throwing a Cucco must not count as a hit, and the Cucco quests must stay possible.

## Investigation
- Actors: the Cucco is `Animal_Cucco` (the game calls its logic "Kokko"). Placed variants: `Animal_Cucco_CatchCucco` (16 in Kakariko Village as a child), `Animal_Cucco_FindOutCucco` (10 in Hyrule Field as a child), `_KakarikoVillage_Adult` (4), `_GerudoValley` (2), `_HrlCity` (1, the Market), `_SavePosition` (2, Zora's River), `Animal_SuperCucco` (3, Hyrule Field). The plain `Animal_Cucco` is placed in no map. The swarm is `Animal_FlockCucco`, created by the angry action (`Kokko.action.getangry`).
- The blackboard table `Kokko` has an `IsAngry` flag, `IsTriggerEvent` and an integer `HitCount` (starts at 0). No graph names `HitCount`, so the count lives in native code.
- The hit logic is `AI/Kokko.interruptseq.root.ainb` (one copy in each pack, plus `Animal_Cucco_FallLiftedCuccoWhenInArea.interruptseq.root.ainb` and `Animal_SuperCucco.interruptseq.root.ainb` in the variants that use their own). It holds one native node, `SelectorBSAKokkoInterruptAction`, with the constant inputs `StartSpecialAttackCount` = 4 and `WaitAngryFrame` = 30, and an output plug `GetAngryAtPlayer`. That plug runs a parallel that sets `IsTriggerEvent`, sets the sound property `IsKokkoRaging` and sets the target to the attacker. This is the hit counter threshold: the angry branch runs when the count reaches the start count. The same node has `EnableCheckActorRootThrownToOnGround` for the throw.
- Carrying and throwing: lifting is a physics and attention action, not damage (the `Liftable` parameters say `ThrownEntityAt` is disabled, so a thrown Cucco deals no damage), and the throw graph (`Kokko.action.interruptthrown`) forbids fall damage. The small hit graph (`Kokko.action.interruptsmallhit`) reads the last attacker through `QuerySensorHitGetLastAttacker`; only the player's attacks count (`QuerySensorHitIsHitTg` with `IsCheckPlayerAttackOnly` true).

## Implementation
- `StartSpecialAttackCount` is changed from 4 to 1 in every `SelectorBSAKokkoInterruptAction` of the covered packs: `Animal_Cucco`, `_GerudoValley`, `_HrlCity`, `_KakarikoVillage_Adult`, `_SavePosition` and `Animal_SuperCucco` (10 graphs). A data edit of one number in each graph; the file size does not change. Read back and audited after the write.
- Left out: `Animal_Cucco_CatchCucco` and `Animal_Cucco_FindOutCucco`. They are separate actors, they are the ones the game's finding and catching quests use, and leaving them alone means those quests are exactly as in the vanilla game. The compendium (`Dictionary`) packs, the flock itself, pocket Cuccos and the Guay crows are untouched.
- A Cucco you carry to Anju is still not counted as hit; if you hit one of the covered Cuccos with the sword, the flock comes.

## How it was tested
- Game file tests (`OOTRR_ROMFS`): the Kokko graph in the real pack has the node with value 4; after the edit exactly one line of the graph differs (the number), the byte size is the same, the audit is clean and a second pass changes nothing; all six covered packs are staged, 10 graphs change, none of the quest packs is staged; the everything-on test stages the packs; the settings string with the setting off is unchanged. `difficulty` (138) and `settings` (107) tests pass.
- The game was not launched.

## In the game
Never played. Steps in `<playtest folder>\surprises\TESTS.md`. Turn on this setting alone.
- Adult Kakariko Village (the Cuccos by the shed): hit one Cucco once with the sword. Pass: the flock of Cuccos rushes at Link after about a second and the raging music plays. With the setting off the same hit should not anger them.
- Child Market or Zora's River: pick up a Cucco, carry it, throw it. Pass: no anger.
- Child Kakariko (Anju's Cuccos) and Hyrule Field's find-out Cuccos: hit one once. Pass: same as vanilla, no revenge.
- Report: a Cucco that angers without a hit, no anger after a hit, or the Cucco quest broken.

## Notes
- Unverified: whether the game's test is "count at least 4" or "count more than 4", which changes whether the first hit or the second hit is needed. If the first hit is not enough, set `FIRST_HIT` to 0 (and watch that this does not anger them without being hit).
- The Cuccos Anju needs and the find-out Cuccos still need several hits to anger, as in vanilla.


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 24):** Cucco revenge on the first hit (setting 131, `cucco_revenge`)
- **State:** Built, untested (code only)
- **What was seen in the game:** Nothing. Commit `f9b7c87` on `feature/surprises`. The hit threshold `StartSpecialAttackCount` goes 4 to 1 in the Cucco hit graphs; Anju's and the find-out Cuccos are left alone
- **Next step:** Play it: one sword hit on an adult Kakariko Cucco; carry and throw one. If one hit is not enough, set `FIRST_HIT` to 0. Steps in `<playtest folder>\surprises\TESTS.md`


## Moved from STATUS.md (20 Nov): never-played round 1

| **Never-played round 1 (agent 7, done)** | `<playtest folder>\never-played-1120\RESULTS.md`. **Seen:** Cucco revenge: one sword hit, about 6 Cuccos attack, life 80 to 74 in 1.5 s (spawned Cucco; control with the setting off not run). Zora ambush cave A: the Skulltula drops when Link reaches the back (x 24.06), not when he is underneath. **Fails the new requirement:** Gohma's Stalfos and 3 Keese are in the actor list from map load, and still listed, unmoving and not visible in shots 15 s after the user started the fight by hand. **Inconclusive:** 24 spawned pots gave only rupees (spawned pots have a fixed drop; map-placed ones needed). **Not tested:** Zora caves B and C, grass, fake pots, respawn, room-clear door, gate 7 (its exe predates rc 15), spawn slots. A script cannot look up at Gohma. Real window is about 1293x756 (status misreports 2560x1440) |
