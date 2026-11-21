# Grass and pot surprises

Part of the existing checkbox `special_encounters` (number 130, "More enemies: special encounters"), whose tip was extended. Code: `crates/core/src/difficulty/surprises.rs`. Built on `feature/surprises` (worktree `wt\surprises`, commit `163112b`), code only, never played.

## Request
"a 1/100 chance that a grass or pot will spawn a skultulla or poe. also any pots can be fake pots."

## Interpretation
- Part 1: cutting grass or breaking a pot has a 1 in 100 chance of an enemy standing there afterwards: a Skulltula or a Poe.
- Part 2: some of the ordinary pots in the maps are really flying pots (`Enemy_FlyingPot`, the game's own pot enemy in the Ice Cavern, Shadow and Spirit Temples and Gerudo Fortress). About 1 in 10 of the plain pots, decided by the seed. A flying pot does not drop what the pot would have, so only pots on the generic drop table are converted.
- No new setting; it is part of special encounters.

## Investigation
### Part 1: the drop system names actors
- Every drop table entry has a `Type` (a hashed category such as heart, magic, bomb) and an optional `DropActorName`. Type 126134492 means "the actor named in `DropActorName`". The game uses it all over: Deku Babas drop `Obj_DekuNut`, Like Likes drop `Obj_Shield_Hylian` and `Obj_Shield_Deku`, Poes drop `Obj_Spirit_Poe`, Gold Skulltulas drop `Obj_Token_Gold`, Ganon's Castle rubble drops `Obj_Fairy`, a palm tree drops `Obj_Rupee_Green`.
- So a drop entry can name any actor. No vanilla table names an enemy, but nothing in the entry limits it to items. The vanilla precedent for the pack question: Poes drop `Obj_Spirit_Poe`, an actor placed in no map and held by no actor holder (the Poe's holder only holds its lantern), so the game loads a named drop's pack when the drop is made.
- Tables seen: pots and the placed grass actors use `DropTable_ResourceSupply_00` (heart 20, magic 20, arrows 20, nuts 20, bombs 20, sticks 0: total 100). The swordable foliage grass uses `DropTable_Foliage_00` in `Pack/FoliageDropParam.pack.zs` (heart 10, magic 10, green rupee 50, bug 30: total 100). Placed pots can override the table with a `Drop__DropTable` or `Drop__DropActor` key in the map. Those are left alone.
- `DropGlobalParam` (the global lottery) only adds chance to hearts and ammo types by need, and swaps heart and ammo actors. It does not touch a named enemy entry.

### Part 2: which pots
Scan of every map file (`Banc` and `AoC1`, child and adult): 508 pots with a name like a pot, 12 ordinary kinds `Obj_Pot_A_01` to `_08` among them. A pot is converted only when all of these hold:
- the actor is exactly `Obj_Pot_A_01` to `_08` (shops' fake cuccos, archery pots, scenery pots with other names are out);
- its keys are only `Dynamic`, `Gyaml`, `Hash`, `IsPhysicsStable`, `Phive`, `Rotate`, `SRTHash`, `Translate`, so no `Links`, no `Presence`, no `Bakeable`;
- `Dynamic` is absent or holds only `DefaultDungeonRoomIndex`: a pot with a `Drop__DropActor` (a key, a fairy, a shuffled item, a rupee colour) or a `Drop__DropTable` is on a chosen drop, so it is left alone;
- the pot's hash appears nowhere else but its own placement, except in the load groups (`FarDeleteGroups`, `SimultaneousGroups`) and the links of a `SupplyItemResetSystemActor` (the actor that refills supply pots). Anything else (another actor's link, an AI group) means logic is tied to it;
- it is not in the lowest room index of its map (a cheap first room guard; overworld maps have no rooms, so they have no guard);
- not in the dream world and not in a map file whose name has Shop, Bazaar, Shooting, Bowling, Mask or Treasure in it.
The result with those rules: 60 candidates; with the default 1 in 10 and a given seed, about 6 to 20 fake pots in the whole game (19 in 13 maps for the test seed).

## Implementation
### Part 1
- New module `surprises.rs`. For each pack of `Obj_Pot_A_*` (not the dream ones), `Obj_Plant_Grass_A_01*` and the foliage pack, the table above gets a new first entry: `Type` 126134492, `DropActorName` the enemy, probability 1 (const `CHANCE`), count 1. If the table already totals 100 or more, one point is taken from the largest entry so the total stays 100; if a cut (no hearts, no ammo) has already zeroed some entries, nothing is taken. The new entry is neither a heart nor ammo, so it composes with `no_heart_drops` and `no_ammo_drops` in either order (a test runs both orders). Run twice, it changes nothing the second time.
- Which enemy: pots drop `Enemy_Poe` (the ordinary Poe, not the Poe sisters, the big Poe or the Hyrule field one), grass drops `Enemy_Skulltula`. One probability point is a whole percent, so one entry per table cannot be split between two enemies; each source gets one.
- Which Skulltula: `Enemy_Skulltula` is the hanging one (the one the Zora ambush hangs in its doorways). There is no Skulltula that walks on the ground (`Enemy_Skullwalltula` crawls on walls and `Enemy_Skulltula_Gold` is the token one). The hanging one is created where the grass was, so it hangs from its own position; how it behaves with no ceiling above it is not known.
- What decides whether the enemy's pack is available: the drop component creates the named actor on demand and the game loads the pack for it, the way the Poe's soul is created. This has not been seen for an enemy and not for a pack that is not already used in the map. If it fails, the likely sign is nothing appearing (or a crash when the drop is made).

### Part 2
- `fake` walks the actors of each map file with a seeded `Rng` (`seed | fake-pots | file`), and for each qualifying pot draws once; 0 of `ONE_IN` (10, a const) converts it. The new actor keeps its hash, position, rotation and room; `IsPhysicsStable` is removed; the `Gyaml` becomes the flying pot that looks like the pot: `Obj_Pot_A_04` becomes `Enemy_FlyingPot_IceCavern` (its model is that pot's), `_05` the Spirit Temple one, `_07` the Shadow Temple one, the other five the plain `Enemy_FlyingPot` (the vanilla pot's model).
- Pots listed in a `SupplyItemResetSystemActor` link list have their entry removed from that list when converted, so the refill actor does not point at a flying pot.
- Hooked in `difficulty::stage` right after the other special encounters step, so it reads the files other features changed (the shuffled pot drops, more enemies) and combines with them.

## How it was tested
- Unit tests: a full table gains the entry and stays at 100, a short table only gains it, running twice changes nothing, the cuts compose in both orders, the right packs and tables are picked, a pot with a drop key, a link, a link to it, a group-only reference, a first room, an archery pot is judged correctly, the dice are seeded and give 20 to 60 conversions in 400, shop and dream maps are skipped, a reset actor forgets a converted pot.
- Game files (`OOTRR_ROMFS`): every pot and grass pack and the foliage pack carry the table with the new first entry and a total of 100 and change exactly one file; the maps give the same result twice for one seed; each converted actor kept its hash and was in the table above; none had `Links`; the everything-on test stages all these files and the settings string with the setting off is unchanged. 138 `difficulty` tests and 107 `settings` tests pass.
- The game was not launched.

## In the game
Never played. Preset and steps in `<playtest folder>\surprises\TESTS.md`. Turn on special encounters alone. Kill with Din's Fire (`give Magic` refills).
- Grass: cut about 300 blades (a 1 in 100 chance each). Expect a Skulltula to appear about three times. Watch that the map does not hang and that hearts or rupees still drop.
- Pots: break about 200 plain pots (Kakariko, Zora's Domain). Expect about two Poes.
- Fake pots: walk Zora's River, Zora's Domain, Zora's Fountain, Hyrule Castle and Kakariko; a pot that flies at Link when he comes near is a fake. Pass: it behaves like the temple flying pots and looks like the pot it replaced.
- Report: nothing spawns, a crash when cutting or breaking, an enemy stuck in the ground or the ceiling, a flying pot that cannot be hurt.

## Notes
- Both parts are unverified in the game. The riskiest assumption is that a drop can create an enemy; the next is the Skulltula's behaviour on the ground.
- To make the odds higher for a test, change `CHANCE` or `ONE_IN` in `surprises.rs`.
- Poes are not in the map of a dungeon with no lantern light; a Poe drop in a lit room is fine.

## Small hands (branch `feature/surprise-hands`, worktree `wt\hands`)
Request: "I also want those creepy small hands that crawl around and attack you to randomly come out of grass or pots sometimes", and for the Skulltula "coming from the grass I want the type of skultulla that's on the ground."
- Each table now has a const list of surprises (`POT_SURPRISES` = Poe, `Enemy_Floormaster_Little`; `GRASS_SURPRISES` = Skulltula, `Enemy_Floormaster_Little`). Each is its own entry at `CHANCE` percent, taking its point from the largest remaining entry, so a table still totals 100. More kinds are one line each in the list. Running twice changes nothing.
- The game has only `Enemy_Skulltula` and `Enemy_Skullwalltula` (wall crawler). `Enemy_Skulltula` is kept for grass: it has been seen standing on the ground when created in the open.
- The little hands are `Enemy_Floormaster_Little`. Its pack is standalone: no actor holder component (the big Floormaster holds them), its own brain with Wander and LandBattle action sequences, perception receiver, jump and basic attack, a `TakeOrders` sequence and a `metamorphose` module that only run when the parent sends messages. Read from the AI files, so created alone it should wander near its spawn point and attack by itself; it will not merge into a big Floormaster. Not seen in the game.
- Tip of the setting updated.

## How to test in the game (small hands)
- Test hook, for test builds only: set the environment variable `OOTRR_SURPRISE_CHANCE` to a whole number from 1 to 20 when building the mod. It replaces `CHANCE` for every surprise entry (10 means each surprise is 1 in 10, so a pot or a blade of grass gives some enemy 2 times in 10). Anything else, or unset, keeps `CHANCE` (1). Default behaviour and default seeds are unchanged.
- Cut grass and break pots in Kakariko or Zora's Domain. Look for a small hand that crawls and attacks, a Poe from pots, a Skulltula standing on the ground from grass.

## Round 2 (branch `feature/enc-r2`, worktree `wt\enc-r2`)
- Rate: `CHANCE` is now 2 percent per surprise ("1 in 50 for the actual build"; two surprises per source, so some enemy about 1 in 25). `OOTRR_SURPRISE_CHANCE` still overrides it.
- Grass that normally spawns bugs: the clump in east Hyrule Field around the Dead Hand hole is 13 placed actors `Land_kokiri_Plant_A_02_DropFixed_03` (in `HyruleField_Child_Dynamic` and `_Adult_Dynamic`, nowhere else), each with a placement key `Drop__DropActor` = `Animal_Insect`, over a pack with its own table `Land_kokiri_Plant_A_02_DropFixed_03` (one entry: the bug type, 100). Neither `DropTable_Foliage_00` nor `DropTable_ResourceSupply_00`. That table now gets the grass surprises (wall Skulltula, little hands) like the others, and the placement key is removed from those actors in the two map files (the table already gives the same bug at 100, so nothing else changes; a placement key probably overrides the table, so without removing it the surprise could never roll). Composes with the cuts, idempotent. Tables now covered: pots and placed grass (`DropTable_ResourceSupply_00`), foliage grass (`DropTable_Foliage_00`), this bug grass.
- Guard house pots in the Market: all `Obj_Pot_A_01` with a placement key `Drop__DropActor` = `Obj_Rupee_Green` or `Drop__DropTable` = `None`. These are deliberately fixed (rupees or nothing), so they are left alone, as the pot rules above say.
- Not played.


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 23):** Grass and pot surprises (part of special encounters): 1 in 100 that cut grass or a broken pot spawns a Skulltula (grass) or Poe (pot), or a small crawling hand (both; branch `feature/surprise-hands`, exe `<playtest folder>\hands\ootrr.exe`, test hook env `OOTRR_SURPRISE_CHANCE` 1 to 20), and 1 in 10 plain pots are flying pots
- **State:** Built, untested (code only)
- **What was seen in the game:** Nothing. Commit `163112b` on `feature/surprises`; 138 difficulty and 107 settings tests pass
- **Next step:** Play it: cut about 300 blades and break about 200 pots; look for fake pots in Zora's Domain and Kakariko. Riskiest: whether a drop can create an enemy. Steps in `<playtest folder>\surprises\TESTS.md`. Round 2 (`feature/enc-r2`): rate now 2 percent per surprise (1 in 50), the bug grass round the hole 9 Dead Hand (`Land_kokiri_Plant_A_02_DropFixed_03`) now gets the grass surprises; Market guard house pots are fixed rupee pots and stay as they are


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 26):** Small hands and floor Skulltulas from grass and pots (new request, saved for later)
- **State:** Built in code, untested (see 23)
- **What was seen in the game:** Requested 18 Nov: the small crawling hands (the Floormaster's little ones) and the floor Skulltulas sometimes appear instead of bugs or rupees when grass is cut or a pot is broken. Extends the grass and pot surprises (23)
- **Next step:** After the current build is polished and out


## Rate tripled (21 Nov, `fix/playtest-misc`, on `playtest/next` only)
Hands-on: 91 pots and grass gave one small crawling hand and no rupees. `CHANCE` in `surprises.rs` is now 6 (was 2) per surprise, about one in eight per source; the setting tip says so. The missing rupees were handed to the same fix to check against the limited ammo drop-table edit; the outcome is not recorded here and nothing was played after. Fake pots still never seen. Not played again after the change.
