# 32. Shield-eating mega Like Likes

Status: **seen by hand on 21 Nov (swallow, kill, the Mirror sparkle; step passed), built and played by script on 20 Nov** (branch `feature/shield-likelikes`, setting `mega_like_likes`, number 144, default off). The feasibility study is kept below the build notes. What was played is listed under "How it was tested"; the feel, a hand-played fight and the Mirror Shield in the hand of a child Link are not played by a person.


## How it works (as built)

- **Setting:** `mega_like_likes` (144), default off, in the Difficulty page under Enemies. Code: `difficulty/mega_like.rs`.
- **The actor:** `Enemy_LikeLike_Mega`, a new actor class made like `Dm_` actors: an ActorParam whose parent is `Enemy_LikeLike`, rows in the actor tables and the tag table, size table entries. The clone pack is built from the staged `Enemy_LikeLike` pack, so `enemy_health` and `enemy_damage` still apply to it. Its own files: ActorParam, parameter table, life component and `MaxLife` times 3 (12 to 36), restraint parameters with the grip and mash-out frames times 1.5.
- **Bigger:** each placement carries `Scale` 1.6. The model grows (it towers over Link in the game) and the grab still works at the usual distance. The collision shapes were not scaled; nothing odd was seen.
- **Placement:** three per age in the Deku Tree meadow, no save flag, so they come back on every load. Child: (82, 113.1, -40), (85, 113.4, -28), (88, 113.6, -16). Adult: (70, 112, -35), (80, 113, -40), (88, 113.7, -16). All stood on the ground when played. The middle child spot is a point between the two checked ones.
- **Taking:** in the clone's copy of `AI/Enemy_LikeLike.action.BasicAttack.module.ainb`, the Enter step of the "Link is swallowed" state now runs a new sequence first: for the Mirror, then the Hylian, then the Deku Shield, read the pouch count (`QueryGetPouchContentNumByActorName`); the first one that is above 0 is removed (`OneShotAddOrRemovePouchContent`, `Num` -1). Order chosen: Mirror first, so the biggest loss comes first. One shield per swallow.
- **Giving back:** in the clone's `AI/Enemy_LikeLike.action.die.module.ainb`, a new sequence runs before the vanilla death steps. For the Mirror, then the Hylian, then the Deku Shield: if the game's own flag `IsGet.<shield>` is true (read with `QueryGetNonStructGameDataStruct` and `QueryGetGameDataBool`; the `HoldGameDataGetBoolSimple` node that the logic graphs use does not work in an AI graph) and the pouch count is 0, add one (`Num` +1). The first one that matches is given, then the rest is skipped, so one shield per death. Several deaths at the same moment cannot duplicate a shield, because the count is read again.
- **Vanilla steal untouched:** the game's own steal still takes an equipped Deku or Hylian Shield ("Your Hylian Shield was swiped.") and still drops it back when the Like Like dies. With a mega Like Like that can mean two shields lost in one swallow (ours, the Mirror, and the game's, the equipped one); both come back over the death, ours by the pouch and the game's by the pickup.
- **Not done:** no pickup or message when the shield comes back (a direct pouch add is silent); the Mirror is not given as a dropped pickup. `IsGetEvent` was left off.

## How it was tested (20 Nov, scripted, one game copy, windowed)

Mod built with the starting sword, Deku and Hylian Shield and the setting on; Mirror Shield, magic and Din's Fire given by the control bridge. Clips are in `<playtest folder>\shieldlike\clips`.

- **The blocker (flags):** `IsGet.Weapon_Shield_Hylian` and `IsGet.Weapon_Shield_Mirror` stay 1 after the shield is removed from the pouch (`PouchData...StockNum` goes to 0). Seen with the bridge's `pouch` command (the same pouch change the AI node makes) and again with the real eat. `HavePlayedGetEvent.Weapon_Shield_Hylian` stayed 0 for a start item, so `IsGet` is the record to use.
- **Map loads:** `KokiriForest_Child` and `KokiriForest_Adult` load; the bridge lists three `Enemy_LikeLike_Mega` with 36 of 36 life (enemy_health 100).
- **Eat:** child Link, Mirror, Hylian and Deku owned: stood by a mega Like Like; it swallowed him, battle music played, life dropped, and the Mirror Shield went (`StockNum` 0, `IsGet` 1). A second swallow took the Hylian Shield, a third the Deku Shield. The pause menu showed the empty slots. Adult Link with the Mirror Shield on his back: the Mirror went at the first swallow, no ghost shield on his back, no crash, no hang.
- **Return:** with the Mirror lost, two mega Like Likes killed with Din's Fire (6 damage per cast to each in range): the Mirror Shield came back (`StockNum` 1) and the Hylian and Deku counts did not change (no duplicate with two deaths).
- **Respawn:** after leaving and re-entering Kokiri Forest with the Mirror and Hylian lost, all three mega Like Likes were back at full life.
- **Death in the belly:** left in the belly, Link died; "Try Again" loaded the last save, which still had the shields. (A save written while a shield was missing would keep it missing; the mega Like Likes are there to win it back.)
- **Not played by a person:** how the fight feels, whether 3x life is the right amount, a fight with the Mirror Shield equipped as a child (a child cannot hold it), the game's own steal and ours together on a child with the Hylian Shield equipped beyond the one message seen, and Kokiri Forest with other settings on.

## How to test it by hand

Build a preset with `mega_like_likes` on (and `open_forest` open to skip Mido). Start a save with the Deku Shield, the Hylian Shield and the Mirror Shield owned. Go to the Deku Tree meadow of Kokiri Forest (east of the village, in front of the tree), as a child and as an adult. Let a mega Like Like swallow you: wait in the belly or mash A to get out. Open the pause menu: one shield (the Mirror, then the Hylian, then the Deku) should be gone. Kill a mega Like Like (Din's Fire kills one in six casts); the shield should be back in the Equipment screen. Leave the forest with a shield missing and come back: all three are back, kill one and the shield returns.

## The feasibility study (before the build)

## What was requested

- Three "mega Like Likes" in Kokiri Forest / outside the Deku Tree: bigger, tougher.
- If one eats Link, he loses one of his shields, any of them, even the Mirror Shield.
- The Like Like keeps the shield until it is killed; killing it gives the shield back.
- Question: possible at all, given that we cannot add saved flags?

## Verdict: feasible with changes

The game already does most of this for the Deku and Hylian Shields, half in code and half in data. What we would add is data only (a cloned actor pack and AI graph edits); nothing needs a code patch or Longshot. Three changes from the request:

1. **The belly is shared, not per Like Like.** What a Like Like has eaten is kept in a blackboard value that is never saved (`Enemy_LikeLike_StealItem`, `IsRecord: false`). So "this Like Like holds your shield" cannot survive a map reload. Instead the inventory is the record (your hypothesis holds): killing **any** mega Like Like gives back **one** shield that Link has had (`IsGet.Weapon_Shield_*` true) and does not have now (`PouchData.Weapon_Shield_*.StockNum` 0). On one visit, the Like Like that ate it is the one you are fighting, so it plays as asked.
2. **The Mirror Shield is taken by our graph, not by the game's eat code.** The game's code decides what is stolen and knows only Deku Shield, Hylian Shield, Goron Tunic and Zora Tunic. We add "if nothing was stolen, remove a shield" to the clone's graph with the pouch node the game's own AI already uses.
3. **"Bigger" is not proven.** Life, attack, grab time and mash-out time are plain data. Making the model and its collision bigger has no proven path (see "Mega").

## 1. How the vanilla Like Like eats a shield

Pack `Pack/Actor/Enemy_LikeLike.pack.zs`.

- **Code decides the steal.** `AI/Enemy_LikeLike.utility.root.ainb` element 1 is `ExecuteLikeLikeRiderMessage` (a code node, no inputs) with outputs `StealItem` (s32) and `IsValid`. When `IsValid` is true the graph sets `Enemy_LikeLike_IsPlayerEquipmetSteal` true and copies `StealItem` into `Enemy_LikeLike_StealItem`. The player's side is code too (`ExecutePlayerReactionRidableEaten` in `Player_Base/Child/Adult`). No data file lists which items can be stolen or how likely a steal is. `GameParameter/LikeLikeParam/Enemy_LikeLike.game__enemy__LikeLikeParam.bgyml` holds only timing (`EatingTime` 130, `AttackInterval` 60, `BasicAttackHoldTime` 120, `AttackDistance` 4.3 and others).
- **What can be stolen:** element 8 of the same graph switches on `StealItem` 1 to 14 and writes one drop table name into `Drop__DropTableOverWrite`: 1 `LikeLike_DekuShield`, 2 `LikeLike_HyliaShield`, 3 `LikeLike_GoronArmor`, 4 `LikeLike_ZoraArmor`, 5 to 10 the pairs (`LikeLike_TwoDekuHylia` and so on), 11 to 14 the triples. **No Mirror Shield value exists**, as in the original game.
- **It is given back on death, in the remake.** `AI/Enemy_LikeLike.action.die.module.ainb` runs `OneShotDropReserveDropItemCreate`, which uses the table named in `Drop__DropTableOverWrite`. `Drop/DropSubTable/LikeLike_DekuShield` drops `Obj_Shield_Deku` (probability 100, count 1); `LikeLike_HyliaShield` drops `Obj_Shield_Hylian`. `LikeLike_GoronArmor` and `LikeLike_ZoraArmor` have **no sub-tables**, so a stolen tunic does not drop back (either the tunic is only unequipped, or it is lost; not checked).
- **Removed, not just unequipped (inferred):** the shield comes back as a pickup (`Obj_Shield_*`) that puts it into the pouch, so the steal must take it out of the pouch. Not seen in the game.
- **If Link leaves the map first:** `Enemy_LikeLike_StealItem` is not saved, so the vanilla shield is gone (bought again in the shop).
- **Can data make it take the Mirror Shield, or take one every time?** Not through the game's code: the steal list and chance are in code. Yes through our own graph nodes (next sections).

## 2. Shield flags (`data/flags.json`)

| Shield | Ever got | Have now | Other |
|---|---|---|---|
| Deku | `IsGet.Weapon_Shield_Deku` | `PouchData.Weapon_Shield_Deku.StockNum` | `HavePlayedGetEvent.Weapon_Shield_Deku`, `HavePlayedGetEventAtShop.Weapon_Shield_Deku`, `BurnOutShieldDeku` |
| Hylian | `IsGet.Weapon_Shield_Hylian` | `PouchData.Weapon_Shield_Hylian.StockNum` | `HavePlayedGetEvent.Weapon_Shield_Hylian`, `HavePlayedGetEventAtShop.Weapon_Shield_Hylian` |
| Mirror | `IsGet.Weapon_Shield_Mirror` | `PouchData.Weapon_Shield_Mirror.StockNum` | `HavePlayedGetEvent.Weapon_Shield_Mirror` |

- "Ever got" and "have" are separate: `IsGet.*` is a bool set on first get; the pouch count is its own int. **Not yet proven** that `IsGet` stays true after the item leaves the pouch (it does in other games on this engine). `HavePlayedGetEvent.*` is a second "ever got" candidate. Check both with `ootrr save-get` on a save before and after a vanilla Like Like eats a Hylian Shield.
- The randomizer's start items set `IsGet.<actor>` with the pouch entry (`start_items/give.rs` line 81), so a starting Mirror Shield also has the flag.
- Ownership at run time is read from the pouch: shop flows use `EventQueryIsExistInPouch` (`DisplayShop`, out-of-stock entries such as `KokiriForest_Shop_SelectActor_Shield_Deku`); enemy AI can read it with `QueryGetPouchContentNumByActorName` (vanilla: `Enemy_Tentacle_Red.utility.root` reads `Item_Boomerang`). The randomizer's logic runs at build time and reads no game flags; it assumes found items stay found.

## 3. Giving it back on death

All of these exist and are used by vanilla graphs:

- **Drop table (closest to vanilla, preferred):** set `Drop__DropTableOverWrite` with `ExecuteBlackboardSetString` before the die module's `OneShotDropReserveDropItemCreate`, exactly as the utility graph does. Deku and Hylian have vanilla tables and pickups. The Mirror Shield needs a new sub-table dropping `Obj_Shield_Mirror`, which the randomizer's own pickup builder already makes (`pickup_pack/build.rs`: `Item_Shield_Mirror` → `Obj_Shield_Mirror`, credits `Weapon_Shield_Mirror`).
- **Direct pouch add:** AI node `OneShotAddOrRemovePouchContent` (inputs `Actor`, `Num`, `Reason`, `IsGetEvent`, `IsEquipDirect`). Used by `EquipmentCommon.event.root`, `PouchContent.event.root` and `NpcCommon.event.root` (`Reason` "Put", "Buy", "Borrow"). Flows give `Weapon_Shield_Mirror` with `Num=1` and remove items with `Num=-1` (`Item_AlarmCucco`, `Item_CojiroCucco`). No visible feedback; fallback if the drop fails.
- **Conditional on section 2:** `HoldGameDataGetBoolSimple` (`IsGet.*`, as gates read `IsGet.Weapon_Sword_Kokiri`) and `QueryGetPouchContentNumByActorName` / `QueryIsExistInPouch` (count 0). Order: Mirror, then Hylian, then Deku; at most one per death. Since the rule is "had it and has none", it cannot duplicate a shield, and a farmed respawning Like Like gives nothing once Link has everything back.
- **Not needed:** room logic. Unlike the gates, this lives in the Like Like's own graph (a clone), so no `Logic/*.ainb` edit.

Side effect: a Deku Shield that burned, or a Hylian Shield a vanilla dungeon Like Like kept, also comes back from a mega Like Like. Harmless; worth one line in the setting's help.

## 4. Persistence and soft-lock

- **Respawn:** vanilla dungeon Like Likes have placement defeat flags (`Enemy_LikeLike@<Map>` in `flags.json`), but **added map actors have no save flag and come back on every load** (`00-making-new-features.md`). So a mega Like Like can never stay dead while a shield is "inside".
- **Leaving with the shield eaten:** the shield is out of the pouch; nothing else is lost. Coming back respawns the mega Like Likes, and killing any one returns it (stateless rule). A dropped pickup that falls into water or despawns is also covered: the next kill gives it again.
- **Mirror Shield and the logic:** the randomizer treats it as permanent (Spirit Temple). With this design it can only be gone while Link has not yet killed a mega Like Like in adult Kokiri Forest, so it is a detour, not a dead end, provided adult Kokiri Forest stays reachable (it holds checks for both ages, so logic reaches it). **Paths to losing it for good:** (a) the mod is rebuilt without this setting on an existing save, so no mega Like Likes exist; (b) `IsGet.Weapon_Shield_Mirror` turns out to clear on loss and `HavePlayedGetEvent.Weapon_Shield_Mirror` too (to be tested first); (c) another feature (enemy randomiser, horde) replaces the clones. Mitigation for (c): exclude the clone kind from replacement. (a) is the user's choice; say it in the help text.
- **Optional safety:** take the Mirror Shield only when Link also owns a Hylian Shield, or never inside the Spirit Temple (they are not placed there anyway).

## 5. "Mega"

- **Clone pack** `Enemy_LikeLike_Mega`, built like `difficulty/volvagia.rs` and `cow_clone`: ActorParam whose `$parent` is `Enemy_LikeLike`, rows in `RSDB/ActorInfo`, `GameActorInfo`, `Tag`, size table entries. The vanilla Like Likes stay untouched.
- **Data per clone:** `Life/LifeParameters` `MaxLife` 12 (for example ×3); `GameBalance/AttackParam` `BaseAttackPower` 2; `LikeLikeParam` timings (`AttackDistance`, `EatingTime`, `AttackInterval`); `GameParameter/RestraintParam/Enemy_LikeLike` `MaxRestraintFrame` 360 and `GachaInputFrame` 330 (how long the grab lasts, how hard to mash out). The grab damage is `PredationDamage` on `OneShotEnemyRequestPredationEatDamageToPlayer` (BasicAttack element 49, blackboard-bound), so it is not in the attack file (known gap, `02-enemy-damage.md`). Build the clone from the staged pack so `enemy_health` and `enemy_damage` still apply (as Volvagia does).
- **Size:** unproven. Placements accept `Scale` (areas, fireballs), but no enemy has been scaled in the game, and the physics shapes (`Phive/ShapeParam/Enemy_LikeLike*`) would not follow. `ModelInfo` has only `DebugModelScale`. First load test: placement `Scale` 1.5 on one clone; if the model grows but its collision does not, scale the clone's shape files too, or drop "bigger" and use a colour tint instead.
- **Spots** (on the navmesh: next to vanilla Deku Baba placements, which sit on it; keep 3 to 5 units off them):

| Map | Spot | Near |
|---|---|---|
| `KokiriForest_Child` | (82, 113.1, -40) | Deku Tree meadow, south (withered Deku Baba at 82.1, -42.9) |
| `KokiriForest_Child` | (88, 113.6, -16) | Deku Tree meadow, north (withered Deku Baba at 90.1, -13.1) |
| `KokiriForest_Child` | (-25, 110.8, 8) | Village, near the adult Deku Baba spot (-25.3, 110.8, 11.8); child map not checked for an enemy here |
| `KokiriForest_Adult` | (70, 112.0, -35) | Meadow west (Deku Baba 67.0, -37.3) |
| `KokiriForest_Adult` | (80, 113.0, -40) | Meadow south (Deku Baba 81.7, -42.9) |
| `KokiriForest_Adult` | (88, 113.7, -16) | Meadow north (Deku Baba 90.1, -12.9) |

Keep clear of gate 6 (the Mido path, about x 38, z -17), and confirm the child meadow is on the same mesh with the `more-enemies` navmesh check before placing. Load-test both maps (lesson from King Dodongo's room).

## Build plan (if approved)

1. **Flag check first (no build):** with a save, read `IsGet.Weapon_Shield_Hylian`, `HavePlayedGetEvent.Weapon_Shield_Hylian` and the `StockNum` before and after a vanilla Like Like eats the shield (Lake Hylia Like Likes from rc 12, or a Spirit Temple one). This decides whether the stateless rule can work. **Blocker if both flags clear.**
2. Clone pack `Enemy_LikeLike_Mega` with scaled life, restraint and timings; unit tests in the style of `volvagia.rs`.
3. Utility graph edit in the clone: after the eat succeeds, if `Enemy_LikeLike_IsPlayerEquipmetSteal` is false, remove the first owned of Mirror, Hylian, Deku (`OneShotAddOrRemovePouchContent` `Num=-1`), then `OneShotRequestDeletePlayerEquipmentFromPouch` and `OneShotSyncPlayerEquipmentAndPouch` so a shield in hand disappears (the game pairs these in `SubEvent_AllBossBattle` and `LinkSleepInBed`). **Main risk:** removing the shield in Link's hand could leave a ghost shield or upset the player; prototype this alone first.
4. Die module edit in the clone: replace the vanilla table choice with the stateless one (Mirror, Hylian, Deku: `IsGet` true and count 0) via `Drop__DropTableOverWrite`; point the vanilla `LikeLike_*` choices at `DropTable_None` in the clone, so a code-stolen shield is not returned twice. Add `LikeLike_MirrorShield` with `Obj_Shield_Mirror` from the pickup builder.
5. Place three per age (table above), under a new setting (next free id, default off).
6. Test by script: map loads, life by hits, eat → `StockNum` 0, kill → pickup → `StockNum` 1; reload with the shield eaten → kill → returned. By hand: how it feels, the shield in hand, and the size.

## How to test in the game

Not testable yet. When built: adult save with the Mirror Shield equipped and a Hylian Shield owned, standing in the Deku Tree meadow. Let a mega Like Like grab Link and hold on; the Mirror Shield should leave his hand and the pouch. Kill it: a Mirror Shield pickup should drop. Repeat, but leave Kokiri Forest before killing it, come back, kill any mega Like Like: the shield should drop.

## Follow-up (21 Nov): one shield per swallow, return

- **Game steal switched off on the clone only:** in the clone's copy of `AI/Enemy_LikeLike.utility.root.ainb` the two root steps that read the rider message and act on it are removed. Seen: Hylian equipped, Mirror owned: one swallow took only the Mirror (stock 0, Hylian and Deku 1, no "swiped" message). Normal Like Likes are untouched.
- **Return stays a direct pouch add (silent).** Tried: setting `Drop__DropTableOverWrite` to the vanilla `LikeLike_HyliaShield` table in the death graph, and (with and without the placement's `Drop__DropTable` None) killing a mega with the bridge `kill`: no pickup appeared and the shield stayed missing, so the route was dropped. The Mirror has no vanilla pickup actor (`Obj_Shield_Mirror` exists only when the pickup builder ran), so it could not be a pickup either. `IsGetEvent` true on the add showed no window. A visible effect was not added.
- **Seen with the new build's `kill`:** Mirror lost, kill: Mirror back; Hylian lost, kill: Hylian back. A pickup left lying cannot lose a shield: nothing is dropped, the pouch is the record, and a reload brings the megas back.

## Follow-up 2 (21 Nov): drop route works for the Hylian and Deku Shields

- The earlier "no pickup" was a test artifact: the bridge `kill` (life set to 0) skips the normal death drops. With `enemyhp` 1 and one real Din's Fire hit, a mega died normally and dropped a Hylian Shield pickup (and a rupee); walking onto it gave `StockNum` 1 through the normal get. The death graph now sets `Drop__DropTableOverWrite` to the game's own `LikeLike_HyliaShield` or `LikeLike_DekuShield` table for those two.
- The Mirror Shield still goes straight into the pouch, silently: it has no vanilla pickup actor, `Obj_Shield_Mirror` exists only when the randomizer's pickup builder made it for the seed, and `IsGetEvent` showed no window. A sparkle effect and a get-item event were not built.
- An unclaimed pickup loses nothing: the pouch stays empty, so the next kill gives it again. Reload (Deku lost): all three megas back at 36 of 36.
- Deku drop not played; it uses the same route and the game's own table.

## Follow-up 3 (21 Nov): the Mirror return shows an effect (branch `feature/mirror-return`)

- **Tried first: (b), the effect keys.** The Mirror branch of the death graph now runs the pouch add in parallel with emits of `OotrrHealRing`, `OotrrHeal` and `OotrrHealOrb` (the Farore's Wind heal keys from `fx.rs`) and the Like Like's own `Eat_Success`, `ライト_00` and `ライト_01` keys (`SPARKLES`). The get-item event (a) was not tried: `IsGetEvent` showed no window earlier and no get-item node was found in the time. Hylian and Deku still come back as the game's own pickups.
- **The effect-link edit** (the heal keys on the `Enemy_LikeLike` user) is now also staged when only `mega_like_likes` is on (`fx::link_stage_for`, that one user); with regen or fairy help on it is the full edit as before.
- **Seen (scripted, one game copy, child Link, Kokiri Forest):** Mirror taken with `take`, a mega brought to 1 hp with `enemyhp` and killed by one real Din's Fire: a green sparkle column rose from the dying Like Like and the Mirror count was back to 1. It is not known which of the six keys made the green (the first attempt with the three heal keys alone was not seen; the camera was not on the body). Not played by a person; the sparkle is small and short.


## Seen by hand and later changes (21 Nov)
- Hands-on 9: the mega Like Likes passed by hand, including the Mirror Shield return with its sparkle (user).
- The Mirror sparkle is on `playtest/next` (`8672f58`, merged) and not yet on `difficulty` (rc 18 has the silent return).
- Size gallery note: five Like Likes at 0.5x to 2.5x all looked the same size, so placement Scale does nothing for them; the mega Like Like gets its size from its own clone pack. See 33.
