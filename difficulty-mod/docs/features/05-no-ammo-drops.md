# Limited ammo (was "No ammo drops")

Setting `no_ammo_drops` (number 115), checkbox. Code: `randomizer/core/src/difficulty/drops.rs` and `difficulty/lottery.rs`.

## Request
"no ammo drops (checkbox)"

## Your later request (18 Nov, feature 27)
"Ammo should still drop in boss fights and anywhere else the player cannot flee and would otherwise have no option but to die." Also answers the review finding that no ammo drops plus shuffled shops can leave a seed with no arrow source.

## Interpretation
Random drops never give arrows, Deku Seeds, bombs, Bombchus, Deku Nuts or Deku Sticks, except from the drop tables the game itself made to stock a boss fight (list below). Magic jars and rupees stay. Ammo from shops, chests and shuffled items stays. The setting keeps its id `no_ammo_drops`, its number 115 and its place in the settings string; only the label and note changed to "Limited ammo".

## Investigation
Same files as no heart drops. The ammo types, identified by hashing guessed names and by which tables use them:

| Type hash | What |
|---|---|
| 3237216321 | Bomb |
| 2211636230 | Deku Nut |
| 4192551701 | Deku Stick |
| 1113099761 | ranged ammo for your current weapon |
| 798871343 | arrows |
| 2565786591 | Bombchu |

One more type, 2121460961, means "whatever you are lowest on" and carries a list of the above.

## Implementation
- Ammo entries in every drop table get a chance of 0.
- "Whatever you are lowest on" entries have the ammo types removed from their list, leaving magic.
- In the global lottery file the low-ammo boosts are set to 0 and the eight ammo actor names are pointed at the green rupee.

## Limited ammo: what is kept (feature 27, redone in round 2, 20 Nov)
Your rule: remove drops from enemies, pots, grass, trees and crates; anything near a boss or in a shop stays; anything an enemy drops during a boss fight stays; and ammo must be obtainable during locked sequences that need it. Boss arenas are rooms inside the dungeon map files, so the exceptions are by drop table name (the game already stocks its fights through named tables).

How the game helps a player who is low: some tables carry an entry of ammo type with 0% chance, and the global lottery file adds a boost (`AddProbability`) to entries of that type when the player is low. Round 1 zeroed those boosts for every table, which silently removed the help from places that depend only on it (the remedy spawners in the Ganondorf tower and Ganon fights, the Bongo Bongo hands, Gohma's room). Round 2 leaves the ammo boosts as in the game and instead **deletes the typed ammo entries from every ordinary table**, so the boost has nothing to feed outside the kept tables. Named ammo entries (a table that names the Deku Seed actor) get 0%, as before. "Most needed" lists lose their ammo, keeping magic. Hearts are unchanged (their boost is still zeroed by the hearts setting).

Kept exactly as in the game:

| Table | Used by | Why kept |
|---|---|---|
| `DropTable_Enemy_RangedWeapon_01` | Poe Meg (miniboss, sealed room) | 100% ammo for the weapon you hold |
| `DropTable_Enemy_RangedWeapon_02` | Gohma eggs and larvae | 80% Deku Seed, plus the low-supply seed boost |
| `DropTable_ResourceSupply_Gohma` | pots, grass, crates, rocks in Gohma's room | seed and nut help by the low-supply boost (restored in round 2) |
| `DropTable_ResourceSupply_FinalBattle` | rubble in the final battle | 15% arrow bundle |
| `DropTable_Ganon_Remedies_00/01/02` | the periodic spawners in the Ganondorf tower and Ganon fights (also placed in the Shadow Temple Bongo Bongo room) | hearts, magic and arrows when low (restored in round 2) |
| `Enemy_DungeonBoss_BongoBongo_00` | the Bongo Bongo hands | arrows and magic when low (restored in round 2) |
| `Drop_ResourceSupply_BeforeBoss*` | pots and crates in the room before each boss door, and Ganondorf's room | pick the most needed of magic, ammo, bombs |
| `DropTable_RangeWeapon_01`, `DropTable_RangeWeapon_02`, `DropTable_Obj_Bomb`, `DropTable_Obj_DekuStick` | pots, grass, crates and rocks that the map places with that table by name | 100% ammo; the game's own supply for rooms that need seeds, arrows, bombs or sticks (list below) |

Everything else loses its ammo: enemy tables, the default pot, grass, tree and rock table (`DropTable_ResourceSupply_00`; trees reach it through their own tree table list), the Deku Nut table, the other bomb tables. Crates have no default drop of their own (their default is `DropTable_None`), so they only drop what the map names, which the same edit covers. The game test reads every actor pack and the foliage pack, so trees, crates, rocks, pots and grass are checked together.

## Locked sequences that need ammo (round 2)
Found by scanning every map file for eye switches and for supply placed by name. The game puts a 100% ammo pot, grass tuft or crate in or beside these, always through the four named tables above, so they are kept:

| Place | Needs | What stays |
|---|---|---|
| Inside the Deku Tree: two eye switches, the Gohma room | Deku Seeds | grass with `DropTable_RangeWeapon_01`, stick grass with `DropTable_Obj_DekuStick`, seed tufts named in the map |
| Dodongo's Cavern: three eye switches, King Dodongo | seeds, bombs | pots with `DropTable_RangeWeapon_01/02`; bomb flowers are placed objects, untouched |
| Forest Temple: five eye switches, Phantom Ganon | arrows, seeds | eight pots with `DropTable_RangeWeapon_01/02`, the pots before the boss door |
| Fire Temple: one eye switch | bombs | pot with `DropTable_Obj_Bomb` |
| Water Temple: two eye switches, Morpha | arrows or seeds | pot with `DropTable_RangeWeapon_02` |
| Spirit Temple: two eye switches, Twinrova | seeds, arrows, bombs | pots with `DropTable_RangeWeapon_01`, `DropTable_Obj_Bomb` |
| Shadow Temple: bomb rooms, Bongo Bongo | bombs, arrows | respawning bomb pots, the remedy spawners and the hands' table in the boss room |
| Bottom of the Well, Gerudo Training Ground (four eye switches), Gerudo Fortress | bombs, arrows | pots and crates with `DropTable_Obj_Bomb`, `DropTable_RangeWeapon_01` |
| Ganon's Castle (tower, Ganondorf, Ganon, final battle) | arrows, bombs | the pots in the tower and Ganondorf's room, the remedy spawners, rubble in the final battle, two pots that name a Bomb and an Arrow bundle directly in the map |

I did not trace which doors shut behind the player. A room is listed if the map has an eye switch or bomb puzzle and a placed supply. Every eye switch found sits in a dungeon that has such supply.

## Known losses and judgement calls
- The four puzzle tables are named per placement in the map, not per room, so a few overworld placements that use them still drop ammo: bomb rocks on Death Mountain Trail and in the Crater, one grass tuft in Hyrule Field, one pot in the Market, stick pots in Zora's River, Zora's Domain and Goron City. Cutting them needs per-actor map edits, which I left out; say if you want them.
- Every dungeon pot or crate the map gives those tables stays, whether or not its room is locked, because telling them apart needs per-room analysis.
- Ammo named directly in the map actor (not a table) is untouched: two Deku Tree seed tufts besides Gohma's two, one stick pot in Goron City, and the two Ganon's Castle pots.
- Bomb flowers in King Dodongo's room and the Flare Dancer's room are placed objects, not drops: untouched.
- Not played in the game. The reading that a 0% typed entry is fed by the low-supply boost comes from the data (the Gohma, Meg, remedy and Bongo tables are built that way), not from seeing it.

## How I tested it
- Unit tests for each case, including the "lowest on" list.
- The game-file test for the global file.
- rc 1 in the game, by you: **did not appear to work.** Same cause as hearts.
- Round 2 (20 Nov, cargo, branch `feature/limited-ammo`): the game test reads every actor pack and the foliage pack, applies the edit and asserts that no ordinary table keeps a live ammo entry, a typed ammo entry or an ammo "most needed" list; that the Gohma, Meg, final battle, remedy, Bongo Bongo and puzzle tables keep theirs; and the lottery test asserts the ammo boosts are untouched and the heart boost is still zero. Unit tests cover the kept and cut table names and the removal of typed entries. Not played in the game.
- Feature 27 (cargo, branch `feature/limited-ammo`): `boss_tables_keep_their_ammo_and_every_other_table_loses_it` (unit), `the_game_keeps_ammo_only_in_the_boss_tables` (reads every actor pack and the foliage pack from the game, applies the edit and asserts that the only tables with live ammo are the boss tables, including Gohma, Meg and the final battle, and that ordinary tables such as `DropTable_ResourceSupply_00` and `DropTable_Obj_Bomb` have none), and the lottery test now asserts the ammo actor names are unchanged. Not played in the game.

## In the game
rc 2's fix has not been played. To test:
- Turn this on alone. Empty your quiver or seed bag and bomb bag, then kill enemies and break pots.
- Pass: no ammo of any kind; magic jars still appear.
- Check a Deku Baba (drops sticks and nuts in vanilla) and a pot room before a boss.

## Scripted test plan (for a later agent; one game copy, windowed, captioned, see <playtest folder>\RECIPE.md)
Preset: `no_ammo_drops` on, plus `skip_intro`, `skip_cutscenes` "all", `faster_chests`, and the dungeon unlocks you need. Use `ootrr save-unlock` / `save-set` for a save with the slingshot or bow, bombs and full health, standing at the boss door (teleport ids only reach a map start). Never touch the user's saves. Caption: "Limited ammo: expect seeds from Gohma larvae, none from Deku Tree pots", then the result.
1. Control (Kokiri Forest or Hyrule Field): empty the seed bag (use the seeds), then cut 30 grass tufts and break 10 pots and kill a dozen enemies. Read the seed count from the give list or HUD after each batch. PASS: stays 0, only rupees and magic appear. FAIL: any ammo.
2. Gohma (Inside the Deku Tree, room before the boss door and the boss room): with 0 seeds, break the two pots/grass before the door (should be rupees or the supply the game gave) and then in the boss room kill Gohma larvae and break the grass. PASS: Deku Seeds drop from larvae/eggs (about 80% each) and from the two seed tufts. Record how many of 10 larvae gave seeds.
3. Meg (Forest Temple, Poe Meg room): defeat a Poe Meg copy with 0 ammo; PASS if it drops ammo for the held weapon.
4. Before-boss pots (any dungeon, for example Dodongo's Cavern room 19): with 0 ammo break the pots before the boss door. Record what comes out (expected: magic, hearts as set, ammo for the held weapon). This is the judgement call above.
5. Final battle rubble (Ganon's castle collapse, last fight): break rubble with 0 arrows; expected arrow bundle about 15%. Record only; reaching it needs a late save, so mark NOT TESTED if no save exists.
6. Ordinary table with a near-boss enemy: kill Keese and Octoroks with 0 ammo; no ammo.
Read: map name and the item counts from status/HUD; take a screenshot after each batch. Anything needing hands is NOT TESTED and goes to `docs\capture-playbook.md`.

## Notes
- The seed logic does not change. With this on you can run out of ammo with no way to refill except shops, so a seed may become very hard. The boss exceptions and the before-boss pots keep a source in every dungeon that has one; overworld seeds remain shop-or-chest only. I did not add logic for that.
- Deku Babas dropping sticks and nuts use named actors in their own tables; those are covered by the name rule (`Obj_DekuStick`, `Obj_DekuNut`).

## Seen in the game (18 Nov, batch 1, aborted)
Same mod as 04. Ammo read from the give list after bombs were used: bombs 20 to 9, sticks 10, nuts 20, seeds 30 unchanged during the walk. The number of things cut was not counted, so NOT TESTED.


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 05):** No ammo drops
- **State:** Done (small sample)
- **What was seen in the game:** Same run as 04: no ammo dropped
- **Next step:** Pots and enemy kills not sampled yet. With Shopsanity on, the note now warns that a special deal can replace an ammo shelf (review finding 3)


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 27):** "No ammo drops" becomes "Limited ammo" (change request, saved for later)
- **State:** Built, untested (20 Nov, `feature/limited-ammo` 3c2c674; difficulty tests 190 pass)
- **What was seen in the game:** Design: ammo is cut everywhere except the game's own boss supply tables (Gohma's eggs and larvae, Poe Meg, final-battle rubble, the pots before each boss door, Gohma's two grass tufts). Setting keeps its id and string position; label is now "Limited ammo". Requested 18 Nov: ammo still drops in boss fights and anywhere else the player cannot flee and would otherwise have no option but to die (the user thinks that is only bosses). Also answers the review's finding that no ammo drops plus shuffled shops can leave no arrow source
- **Next step:** After the current build is polished and out


## Moved from STATUS.md (20 Nov): limited ammo scripted result

  - **Limited ammo:** Kakariko pots gave no seeds or arrows (weak pass, small sample); boss-door pots inconclusive.
