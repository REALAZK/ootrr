# Location catalog coverage

Generated from the extracted RomFS folder and the OoTR `LocationList.py` and `data/World` rule files (MIT, github.com/OoTRandomizer/OoT-Randomizer). Vanilla layout only, Master Quest locations are excluded.

## Summary

- In-scope OoTR locations: 988. Pinned (item anchor, only consistent partner or decisive evidence): 162 (16.4%). Pooled (guess inside an ambiguous pool): 119 (12.0%). Unmatched: 707 (71.6%).
- Remake candidate actors considered: 657. Matched: 281. Unmatched: 376.
- Verification (`ootrr check-catalog`): every hash in `data/locations.json` exists in the file it names, the item or Gyaml recorded for it matches the dump, no hash is used by two locations, and no hash is both matched and unmatched. 120834 actors scanned, 0 issues. VERIFIED.
- Percentages above are over the in-scope OoTR types listed below. Pots, crates and beehives drag the total down, see the per-kind table.

Confidence levels in the json `conf` field:

- `item`: both sides are the same specific item (map, compass, boss key, a named major item, a boss heart, a skulltula token, a cow) and the pair is the only consistent partner in its area. VERIFIED for item identity, the area assignment is by folder.
- `forced`: no other partner is consistent, but the item itself is not specific (for example the one remaining filler chest). GUESS-grade where the area has extra remake chests.
- `pool`: several locations share identical constraints and the assignment inside the pool is a guess. Treat the pool as a set. Entries carry a shared `pool` id.

Every location also carries an `ev` list (revision 2). The codes are `item` (specific item identity), `count` (only partner by counts), `hard` (only partner under the age and night constraints), `elim` (only partner left after other pins), `pool` (no distinguishing evidence), and for evidence pins `age`, `night`, `lbl` (developer room label), `rm` (room shared with a pinned anchor or the entry room), `nbr` (a named neighbor actor), `lm` (landmark distance), `hgt` (height), `kind` (container kind), `lin` and `anchor` (linear room order from a pinned end), `coloc` (same region and room as a pinned partner) and `pos` (position in the dungeon). Evidence pins are VERIFIED to exist and to be unique in the dump and are GUESS-grade until a play-through confirms them. `ootrr calibrate` is the way to confirm the remaining pools.

## Per kind

| Kind | OoTR | Remake candidates | Pinned | Pooled | OoTR unmatched | Remake unmatched |
|---|---|---|---|---|---|---|
| beehive | 32 | 3 | 0 | 0 | 32 | 3 |
| bossheart | 8 | 8 | 8 | 0 | 0 | 0 |
| chest | 179 | 203 | 75 | 32 | 72 | 92 |
| cow | 9 | 8 | 4 | 0 | 5 | 4 |
| crate | 80 | 9 | 1 | 0 | 79 | 8 |
| flying_pot | 21 | 3 | 0 | 0 | 21 | 3 |
| freestanding | 155 | 130 | 1 | 15 | 139 | 114 |
| gs | 100 | 100 | 49 | 47 | 4 | 4 |
| heart | 20 | 14 | 14 | 2 | 4 | 1 |
| key | 7 | 6 | 2 | 5 | 0 | 0 |
| other_drop | 0 | 63 | 0 | 0 | 0 | 63 |
| pot | 341 | 93 | 3 | 18 | 320 | 72 |
| scrub | 36 | 17 | 5 | 0 | 31 | 12 |

Remake candidates of kind `other_drop` are Drop__DropActor holders with no OoTR location type (grass, lift rocks, Iron Knuckle pillars, enemies, trees), so they can never match.

Remake `chest` candidates include 10 DreamWorld boss rush chests and the remake-only chests listed under unmatched.

## Per area, OoTR counts against remake counts

Remake candidates are counted per remake folder. `chest+heart+key` merges chests, standalone heart pieces and standalone small keys because the remake stores some of these as chest contents and some as free actors. Lake Hylia and Lon Lon Ranch live inside the HyruleField folder. Spirit Temple chests for the Silver Gauntlets and Mirror Shield live in DesertColossus.

| Area group | Slot kind | OoTR | Remake | Pinned | Pooled | OoTR unmatched | Remake unmatched |
|---|---|---|---|---|---|---|---|
| BottomOfTheWell | chest+heart+key | 14 | 13 | 6 | 7 | 1 | 0 |
| BottomOfTheWell | flying_pot | 3 | 0 | 0 | 0 | 3 | 0 |
| BottomOfTheWell | freestanding | 7 | 0 | 0 | 0 | 7 | 0 |
| BottomOfTheWell | gs | 3 | 3 | 1 | 2 | 0 | 0 |
| BottomOfTheWell | pot | 19 | 1 | 0 | 0 | 19 | 1 |
| BottomOfTheWell | scrub | 0 | 1 | 0 | 0 | 0 | 1 |
| DeathMountainCrater | beehive | 3 | 0 | 0 | 0 | 3 | 0 |
| DeathMountainCrater | chest+heart+key | 3 | 4 | 0 | 2 | 1 | 2 |
| DeathMountainCrater | crate | 0 | 1 | 0 | 0 | 0 | 1 |
| DeathMountainCrater | freestanding | 8 | 10 | 0 | 0 | 8 | 10 |
| DeathMountainCrater | gs | 2 | 2 | 2 | 0 | 0 | 0 |
| DeathMountainCrater | pot | 4 | 0 | 0 | 0 | 4 | 0 |
| DeathMountainCrater | scrub | 4 | 1 | 0 | 0 | 4 | 1 |
| DeathMountainTrail | beehive | 3 | 0 | 0 | 0 | 3 | 0 |
| DeathMountainTrail | chest+heart+key | 3 | 2 | 1 | 0 | 2 | 1 |
| DeathMountainTrail | cow | 1 | 1 | 1 | 0 | 0 | 0 |
| DeathMountainTrail | freestanding | 6 | 6 | 1 | 0 | 5 | 5 |
| DeathMountainTrail | gs | 4 | 4 | 4 | 0 | 0 | 0 |
| DesertColossus | beehive | 1 | 0 | 0 | 0 | 1 | 0 |
| DesertColossus | chest+heart+key | 1 | 1 | 1 | 0 | 0 | 0 |
| DesertColossus | gs | 3 | 3 | 1 | 0 | 2 | 2 |
| DesertColossus | scrub | 2 | 0 | 0 | 0 | 2 | 0 |
| DodongoCavern | bossheart | 1 | 1 | 1 | 0 | 0 | 0 |
| DodongoCavern | chest+heart+key | 6 | 9 | 3 | 0 | 3 | 6 |
| DodongoCavern | freestanding | 4 | 0 | 0 | 0 | 4 | 0 |
| DodongoCavern | gs | 5 | 5 | 1 | 4 | 0 | 0 |
| DodongoCavern | pot | 28 | 4 | 1 | 0 | 27 | 3 |
| DodongoCavern | scrub | 4 | 1 | 0 | 0 | 4 | 1 |
| DreamWorld | chest+heart+key | 0 | 10 | 0 | 0 | 0 | 10 |
| DreamWorld | pot | 0 | 8 | 0 | 0 | 0 | 8 |
| FireTemple | bossheart | 1 | 1 | 1 | 0 | 0 | 0 |
| FireTemple | chest+heart+key | 14 | 16 | 4 | 8 | 2 | 4 |
| FireTemple | crate | 2 | 0 | 0 | 0 | 2 | 0 |
| FireTemple | freestanding | 9 | 2 | 0 | 0 | 9 | 2 |
| FireTemple | gs | 5 | 5 | 0 | 5 | 0 | 0 |
| FireTemple | pot | 15 | 3 | 0 | 0 | 15 | 3 |
| ForestTemple | bossheart | 1 | 1 | 1 | 0 | 0 | 0 |
| ForestTemple | chest+heart+key | 13 | 11 | 9 | 0 | 4 | 2 |
| ForestTemple | freestanding | 4 | 0 | 0 | 0 | 4 | 0 |
| ForestTemple | gs | 5 | 5 | 2 | 3 | 0 | 0 |
| ForestTemple | pot | 19 | 0 | 0 | 0 | 19 | 0 |
| GanonCastle | gs | 1 | 1 | 1 | 0 | 0 | 0 |
| GanonCastle | pot | 0 | 9 | 0 | 0 | 0 | 9 |
| GerudoFortress | chest+heart+key | 5 | 6 | 1 | 4 | 0 | 1 |
| GerudoFortress | cow | 0 | 1 | 0 | 0 | 0 | 1 |
| GerudoFortress | crate | 33 | 2 | 1 | 0 | 32 | 1 |
| GerudoFortress | flying_pot | 0 | 3 | 0 | 0 | 0 | 3 |
| GerudoFortress | gs | 2 | 2 | 2 | 0 | 0 | 0 |
| GerudoFortress | pot | 16 | 3 | 0 | 3 | 13 | 0 |
| GerudoTrainingGround | chest+heart+key | 22 | 20 | 2 | 8 | 12 | 10 |
| GerudoTrainingGround | freestanding | 2 | 0 | 0 | 0 | 2 | 0 |
| GerudoValley | beehive | 1 | 0 | 0 | 0 | 1 | 0 |
| GerudoValley | chest+heart+key | 3 | 7 | 2 | 0 | 1 | 5 |
| GerudoValley | cow | 1 | 1 | 1 | 0 | 0 | 0 |
| GerudoValley | crate | 2 | 0 | 0 | 0 | 2 | 0 |
| GerudoValley | freestanding | 8 | 0 | 0 | 0 | 8 | 0 |
| GerudoValley | gs | 4 | 4 | 2 | 2 | 0 | 0 |
| GerudoValley | pot | 0 | 4 | 0 | 0 | 0 | 4 |
| GerudoValley | scrub | 2 | 0 | 0 | 0 | 2 | 0 |
| GoronCity | beehive | 1 | 0 | 0 | 0 | 1 | 0 |
| GoronCity | chest+heart+key | 4 | 6 | 0 | 0 | 4 | 6 |
| GoronCity | crate | 1 | 0 | 0 | 0 | 1 | 0 |
| GoronCity | freestanding | 8 | 0 | 0 | 0 | 8 | 0 |
| GoronCity | gs | 2 | 2 | 2 | 0 | 0 | 0 |
| GoronCity | pot | 9 | 2 | 0 | 0 | 9 | 2 |
| GoronCity | scrub | 3 | 0 | 0 | 0 | 3 | 0 |
| Graveyard | chest+heart+key | 7 | 7 | 3 | 0 | 4 | 4 |
| Graveyard | crate | 1 | 0 | 0 | 0 | 1 | 0 |
| Graveyard | freestanding | 8 | 8 | 0 | 8 | 0 | 0 |
| Graveyard | gs | 2 | 2 | 2 | 0 | 0 | 0 |
| Graveyard | pot | 6 | 0 | 0 | 0 | 6 | 0 |
| HountedWasteland | chest+heart+key | 1 | 1 | 1 | 0 | 0 | 0 |
| HountedWasteland | crate | 5 | 0 | 0 | 0 | 5 | 0 |
| HountedWasteland | gs | 1 | 1 | 1 | 0 | 0 | 0 |
| HountedWasteland | pot | 4 | 0 | 0 | 0 | 4 | 0 |
| HyruleCastle | gs | 2 | 2 | 2 | 0 | 0 | 0 |
| HyruleCastle | pot | 4 | 0 | 0 | 0 | 4 | 0 |
| HyruleField | beehive | 9 | 0 | 0 | 0 | 9 | 0 |
| HyruleField | chest+heart+key | 6 | 9 | 3 | 0 | 3 | 6 |
| HyruleField | cow | 5 | 3 | 0 | 0 | 5 | 3 |
| HyruleField | crate | 1 | 5 | 0 | 0 | 1 | 5 |
| HyruleField | freestanding | 6 | 26 | 0 | 0 | 6 | 26 |
| HyruleField | gs | 11 | 11 | 4 | 7 | 0 | 0 |
| HyruleField | pot | 12 | 7 | 0 | 0 | 12 | 7 |
| HyruleField | scrub | 7 | 4 | 1 | 0 | 6 | 3 |
| IceCavern | chest+heart+key | 4 | 4 | 3 | 0 | 1 | 1 |
| IceCavern | flying_pot | 1 | 0 | 0 | 0 | 1 | 0 |
| IceCavern | freestanding | 7 | 0 | 0 | 0 | 7 | 0 |
| IceCavern | gs | 3 | 3 | 0 | 3 | 0 | 0 |
| IceCavern | pot | 8 | 6 | 0 | 0 | 8 | 6 |
| InsideGanonCastle | chest+heart+key | 16 | 12 | 3 | 0 | 13 | 9 |
| InsideGanonCastle | freestanding | 5 | 0 | 0 | 0 | 5 | 0 |
| InsideGanonCastle | pot | 34 | 12 | 1 | 0 | 33 | 11 |
| InsideGanonCastle | scrub | 4 | 0 | 0 | 0 | 4 | 0 |
| InsideJabuJabuBelly | bossheart | 1 | 1 | 1 | 0 | 0 | 0 |
| InsideJabuJabuBelly | chest+heart+key | 3 | 4 | 3 | 0 | 0 | 1 |
| InsideJabuJabuBelly | crate | 2 | 0 | 0 | 0 | 2 | 0 |
| InsideJabuJabuBelly | gs | 4 | 4 | 0 | 4 | 0 | 0 |
| InsideJabuJabuBelly | pot | 17 | 6 | 0 | 6 | 11 | 0 |
| InsideJabuJabuBelly | scrub | 1 | 1 | 1 | 0 | 0 | 0 |
| InsideTheDekuTree | bossheart | 1 | 1 | 1 | 0 | 0 | 0 |
| InsideTheDekuTree | chest+heart+key | 6 | 6 | 6 | 0 | 0 | 0 |
| InsideTheDekuTree | freestanding | 5 | 12 | 0 | 0 | 5 | 12 |
| InsideTheDekuTree | gs | 4 | 4 | 2 | 2 | 0 | 0 |
| KakarikoVillage | beehive | 2 | 3 | 0 | 0 | 2 | 3 |
| KakarikoVillage | chest+heart+key | 4 | 5 | 2 | 0 | 2 | 3 |
| KakarikoVillage | cow | 1 | 1 | 1 | 0 | 0 | 0 |
| KakarikoVillage | crate | 18 | 0 | 0 | 0 | 18 | 0 |
| KakarikoVillage | gs | 6 | 6 | 2 | 4 | 0 | 0 |
| KakarikoVillage | pot | 11 | 0 | 0 | 0 | 11 | 0 |
| KokiriForest | beehive | 2 | 0 | 0 | 0 | 2 | 0 |
| KokiriForest | chest+heart+key | 6 | 7 | 1 | 4 | 1 | 2 |
| KokiriForest | cow | 1 | 1 | 1 | 0 | 0 | 0 |
| KokiriForest | freestanding | 15 | 24 | 0 | 7 | 8 | 17 |
| KokiriForest | gs | 3 | 3 | 3 | 0 | 0 | 0 |
| KokiriForest | pot | 5 | 4 | 1 | 3 | 1 | 0 |
| KokiriForest | scrub | 0 | 1 | 0 | 0 | 0 | 1 |
| LostWood | beehive | 3 | 0 | 0 | 0 | 3 | 0 |
| LostWood | chest+heart+key | 1 | 1 | 1 | 0 | 0 | 0 |
| LostWood | freestanding | 9 | 0 | 0 | 0 | 9 | 0 |
| LostWood | gs | 3 | 3 | 0 | 2 | 1 | 1 |
| LostWood | scrub | 5 | 7 | 3 | 0 | 2 | 4 |
| Market | chest+heart+key | 11 | 12 | 11 | 0 | 0 | 1 |
| Market | crate | 13 | 1 | 0 | 0 | 13 | 1 |
| Market | gs | 1 | 1 | 1 | 0 | 0 | 0 |
| Market | pot | 58 | 16 | 0 | 0 | 58 | 16 |
| SacredForestMeadow | beehive | 1 | 0 | 0 | 0 | 1 | 0 |
| SacredForestMeadow | chest+heart+key | 1 | 2 | 0 | 0 | 1 | 2 |
| SacredForestMeadow | gs | 1 | 1 | 1 | 0 | 0 | 0 |
| SacredForestMeadow | scrub | 2 | 0 | 0 | 0 | 2 | 0 |
| ShadowTemple | bossheart | 1 | 1 | 1 | 0 | 0 | 0 |
| ShadowTemple | chest+heart+key | 17 | 15 | 9 | 0 | 8 | 6 |
| ShadowTemple | flying_pot | 3 | 0 | 0 | 0 | 3 | 0 |
| ShadowTemple | freestanding | 16 | 0 | 0 | 0 | 16 | 0 |
| ShadowTemple | gs | 5 | 5 | 5 | 0 | 0 | 0 |
| ShadowTemple | pot | 21 | 7 | 0 | 6 | 15 | 1 |
| ShadowTemple | scrub | 0 | 1 | 0 | 0 | 0 | 1 |
| SpiritTemple | bossheart | 1 | 1 | 1 | 0 | 0 | 0 |
| SpiritTemple | chest+heart+key | 19 | 17 | 6 | 6 | 7 | 5 |
| SpiritTemple | crate | 2 | 0 | 0 | 0 | 2 | 0 |
| SpiritTemple | flying_pot | 14 | 0 | 0 | 0 | 14 | 0 |
| SpiritTemple | freestanding | 2 | 0 | 0 | 0 | 2 | 0 |
| SpiritTemple | gs | 5 | 5 | 2 | 3 | 0 | 0 |
| SpiritTemple | pot | 16 | 0 | 0 | 0 | 16 | 0 |
| WaterTemple | bossheart | 1 | 1 | 1 | 0 | 0 | 0 |
| WaterTemple | chest+heart+key | 10 | 9 | 4 | 0 | 6 | 5 |
| WaterTemple | freestanding | 4 | 0 | 0 | 0 | 4 | 0 |
| WaterTemple | gs | 5 | 5 | 1 | 4 | 0 | 0 |
| WaterTemple | pot | 23 | 1 | 0 | 0 | 23 | 1 |
| ZoraDomain | beehive | 3 | 0 | 0 | 0 | 3 | 0 |
| ZoraDomain | chest+heart+key | 1 | 1 | 1 | 0 | 0 | 0 |
| ZoraDomain | freestanding | 0 | 6 | 0 | 0 | 0 | 6 |
| ZoraDomain | gs | 1 | 1 | 1 | 0 | 0 | 0 |
| ZoraDomain | pot | 5 | 0 | 0 | 0 | 5 | 0 |
| ZoraFountain | chest+heart+key | 2 | 3 | 2 | 0 | 0 | 1 |
| ZoraFountain | freestanding | 18 | 16 | 0 | 0 | 18 | 16 |
| ZoraFountain | gs | 3 | 3 | 3 | 0 | 0 | 0 |
| ZoraFountain | pot | 7 | 0 | 0 | 0 | 7 | 0 |
| ZoraRiver | beehive | 3 | 0 | 0 | 0 | 3 | 0 |
| ZoraRiver | chest+heart+key | 3 | 3 | 3 | 0 | 0 | 0 |
| ZoraRiver | freestanding | 4 | 20 | 0 | 0 | 4 | 20 |
| ZoraRiver | gs | 4 | 4 | 1 | 2 | 1 | 1 |
| ZoraRiver | scrub | 2 | 0 | 0 | 0 | 2 | 0 |

## Out of scope OoTR types (not attempted)

Shops, songs, bosses, cutscenes, events, hints, event-reward NPCs, Wonderitems, silver rupees, rupee towers and enemy drops: Boss 9, Collectable 1, Cutscene 9, Drop 21, Event 4, Hint 14, HintStone 40, MaskShop 8, NPC (event reward) 62, RupeeTower 21, Shop 64, SilverRupee 80, Song 12, Wonderitem 116. Shops and event rewards are out of scope for this catalog. Wonderitems, silver rupees and rupee towers were left out because the remake structure for them was not studied.

## Unmatched OoTR locations and reasons

- **BottomOfTheWell / chest** (no remake counterpart in group): Bottom of the Well Like Like Chest
- **BottomOfTheWell / flying_pot** (no remake counterpart in group): Bottom of the Well West Inner Room Flying Pot 1, Bottom of the Well West Inner Room Flying Pot 2, Bottom of the Well West Inner Room Flying Pot 3
- **BottomOfTheWell / freestanding** (no remake counterpart in group): Bottom of the Well Center Room Pit Fall Blue Rupee 1, Bottom of the Well Center Room Pit Fall Blue Rupee 2, Bottom of the Well Center Room Pit Fall Blue Rupee 3, Bottom of the Well Center Room Pit Fall Blue Rupee 4, Bottom of the Well Center Room Pit Fall Blue Rupee 5, Bottom of the Well Coffin Recovery Heart 1, Bottom of the Well Coffin Recovery Heart 2
- **BottomOfTheWell / pot** (ambiguous: 7 oot vs 1 remake candidates of the same item class): Bottom of the Well Left Side Pot 2, Bottom of the Well Near Entrance Pot 1, Bottom of the Well Basement Pot 2, Bottom of the Well Basement Pot 4, Bottom of the Well Basement Pot 5, Bottom of the Well Basement Pot 11, Bottom of the Well Fire Keese Pot
- **BottomOfTheWell / pot** (no remake counterpart in group): Bottom of the Well Left Side Pot 1, Bottom of the Well Left Side Pot 3, Bottom of the Well Near Entrance Pot 2, Bottom of the Well Underwater Pot, Bottom of the Well Basement Pot 1, Bottom of the Well Basement Pot 3, Bottom of the Well Basement Pot 6, Bottom of the Well Basement Pot 7, Bottom of the Well Basement Pot 8, Bottom of the Well Basement Pot 9, Bottom of the Well Basement Pot 10, Bottom of the Well Basement Pot 12
- **DeathMountainCrater / beehive** (no remake counterpart in group): DMC Upper Grotto Beehive 1, DMC Upper Grotto Beehive 2, DMC Hammer Grotto Beehive
- **DeathMountainCrater / chest** (ambiguous: 1 oot vs 2 remake candidates): DMC Upper Grotto Chest
- **DeathMountainCrater / freestanding** (ambiguous: 2 oot vs 1 remake candidates of the same item class): DMC Child Red Rupee 1, DMC Child Red Rupee 2
- **DeathMountainCrater / freestanding** (no remake counterpart in group): DMC Child Blue Rupee 1, DMC Child Blue Rupee 2, DMC Child Blue Rupee 3, DMC Child Blue Rupee 4, DMC Child Blue Rupee 5, DMC Child Blue Rupee 6
- **DeathMountainCrater / pot** (no remake counterpart in group): DMC Near GC Pot 1, DMC Near GC Pot 2, DMC Near GC Pot 3, DMC Near GC Pot 4
- **DeathMountainCrater / scrub** (ambiguous: 4 oot vs 1 remake candidates): DMC Deku Scrub, DMC Deku Scrub Grotto Left, DMC Deku Scrub Grotto Center, DMC Deku Scrub Grotto Right
- **DeathMountainTrail / beehive** (no remake counterpart in group): DMT Cow Grotto Beehive, DMT Storms Grotto Beehive 1, DMT Storms Grotto Beehive 2
- **DeathMountainTrail / chest** (ambiguous: 2 oot vs 1 remake candidates): DMT Chest, DMT Storms Grotto Chest
- **DeathMountainTrail / freestanding** (ambiguous: 1 oot vs 2 remake candidates of the same item class): DMT Rock Blue Rupee
- **DeathMountainTrail / freestanding** (no remake counterpart in group): DMT Cow Grotto Recovery Heart 1, DMT Cow Grotto Recovery Heart 2, DMT Cow Grotto Recovery Heart 3, DMT Cow Grotto Recovery Heart 4
- **DesertColossus / beehive** (no remake counterpart in group): Colossus Grotto Beehive
- **DesertColossus / gs** (ambiguous pool with class conflicts): Colossus GS Tree, Colossus GS Hill
- **DesertColossus / scrub** (area has no remake counterpart in dump): Colossus Deku Scrub Grotto Front, Colossus Deku Scrub Grotto Rear
- **DodongoCavern / chest** (ambiguous: 3 oot vs 6 remake candidates): Dodongos Cavern Bomb Flower Platform Chest, Dodongos Cavern End of Bridge Chest, Dodongos Cavern Boss Room Chest
- **DodongoCavern / freestanding** (no remake counterpart in group): Dodongos Cavern Lizalfos Upper Recovery Heart 1, Dodongos Cavern Lizalfos Upper Recovery Heart 2, Dodongos Cavern Blade Room Behind Block Recovery Heart, Dodongos Cavern Lower Lizalfos Hidden Recovery Heart
- **DodongoCavern / pot** (ambiguous: 4 oot vs 1 remake candidates of the same item class): Dodongos Cavern Right Side Pot 1, Dodongos Cavern Right Side Pot 3, Dodongos Cavern Right Side Pot 5, Dodongos Cavern Torch Room Pot 2
- **DodongoCavern / pot** (ambiguous: 8 oot vs 2 remake candidates of the same item class): Dodongos Cavern Right Side Pot 2, Dodongos Cavern Right Side Pot 4, Dodongos Cavern Lower Lizalfos Pot 3, Dodongos Cavern Lower Lizalfos Pot 4, Dodongos Cavern Torch Room Pot 1, Dodongos Cavern Torch Room Pot 3, Dodongos Cavern Single Eye Switch Room Pot 2, Dodongos Cavern Double Eye Switch Room Pot 2
- **DodongoCavern / pot** (no remake counterpart in group): 15 locations, first: Dodongos Cavern Right Side Pot 6, Dodongos Cavern Lower Lizalfos Pot 1, Dodongos Cavern Lower Lizalfos Pot 2, Dodongos Cavern Torch Room Pot 4, Dodongos Cavern Staircase Pot 1, Dodongos Cavern Staircase Pot 2
- **DodongoCavern / scrub** (ambiguous: 4 oot vs 1 remake candidates): Dodongos Cavern Deku Scrub Side Room Near Dodongos, Dodongos Cavern Deku Scrub Lobby, Dodongos Cavern Deku Scrub Near Bomb Bag Left, Dodongos Cavern Deku Scrub Near Bomb Bag Right
- **FireTemple / chest** (ambiguous: 2 oot vs 4 remake candidates): Fire Temple Flare Dancer Chest, Fire Temple Scarecrow Chest
- **FireTemple / crate** (no remake counterpart in group): Fire Temple Hammer Staircase Small Wooden Crate 1, Fire Temple Hammer Staircase Small Wooden Crate 2
- **FireTemple / freestanding** (ambiguous: 9 oot vs 2 remake candidates of the same item class): Fire Temple Elevator Room Recovery Heart 1, Fire Temple Elevator Room Recovery Heart 2, Fire Temple Elevator Room Recovery Heart 3, Fire Temple Narrow Path Room Recovery Heart 1, Fire Temple Narrow Path Room Recovery Heart 2, Fire Temple Narrow Path Room Recovery Heart 3, Fire Temple Moving Fire Room Recovery Heart 1, Fire Temple Moving Fire Room Recovery Heart 2, Fire Temple Moving Fire Room Recovery Heart 3
- **FireTemple / pot** (no remake counterpart in group): 15 locations, first: Fire Temple Big Lava Room Pot 1, Fire Temple Big Lava Room Pot 2, Fire Temple Big Lava Room Pot 3, Fire Temple Near Boss Pot 1, Fire Temple Near Boss Pot 2, Fire Temple Near Boss Pot 3
- **ForestTemple / chest** (ambiguous: 4 oot vs 2 remake candidates): Forest Temple Raised Island Courtyard Chest, Forest Temple Eye Switch Chest, Forest Temple Falling Ceiling Room Chest, Forest Temple Basement Chest
- **ForestTemple / freestanding** (no remake counterpart in group): Forest Temple Courtyard Recovery Heart 1, Forest Temple Courtyard Recovery Heart 2, Forest Temple Well Recovery Heart 1, Forest Temple Well Recovery Heart 2
- **ForestTemple / pot** (no remake counterpart in group): 19 locations, first: Forest Temple Center Room Right Pot 1, Forest Temple Center Room Right Pot 2, Forest Temple Center Room Right Pot 3, Forest Temple Center Room Left Pot 1, Forest Temple Center Room Left Pot 2, Forest Temple Center Room Left Pot 3
- **GerudoFortress / crate** (no remake counterpart in group): 32 locations, first: GF Crate 1, GF Crate 2, GF Crate 3, GF Crate 4, GF Crate 5, GF Crate 6
- **GerudoFortress / pot** (no remake counterpart in group): Hideout Break Room Pot 1, Hideout 1 Torch Jail Pot 1, Hideout 1 Torch Jail Pot 2, Hideout 1 Torch Jail Pot 3, Hideout Kitchen Pot 1, Hideout Kitchen Pot 2, Hideout 4 Torch Jail Pot 2, Hideout 2 Torch Jail Pot 1, Hideout 2 Torch Jail Pot 3, Hideout 2 Torch Jail In Cell Pot 1, Hideout 2 Torch Jail In Cell Pot 2, Hideout 2 Torch Jail In Cell Pot 3, Hideout 2 Torch Jail In Cell Pot 4
- **GerudoTrainingGround / chest** (ambiguous: 12 oot vs 10 remake candidates): Gerudo Training Ground Lobby Left Chest, Gerudo Training Ground Lobby Right Chest, Gerudo Training Ground Before Heavy Block Chest, Gerudo Training Ground Heavy Block First Chest, Gerudo Training Ground Heavy Block Second Chest, Gerudo Training Ground Heavy Block Fourth Chest, Gerudo Training Ground Hammer Room Clear Chest, Gerudo Training Ground Maze Right Central Chest, Gerudo Training Ground Maze Right Side Chest, Gerudo Training Ground Maze Path First Chest, Gerudo Training Ground Maze Path Second Chest, Gerudo Training Ground Maze Path Third Chest
- **GerudoTrainingGround / freestanding** (no remake counterpart in group): Gerudo Training Ground Beamos Recovery Heart 1, Gerudo Training Ground Beamos Recovery Heart 2
- **GerudoValley / beehive** (no remake counterpart in group): GV Storms Grotto Beehive
- **GerudoValley / chest** (ambiguous: 1 oot vs 5 remake candidates): GV Chest
- **GerudoValley / crate** (no remake counterpart in group): GV Crate Near Cow, GV Freestanding PoH Crate
- **GerudoValley / freestanding** (no remake counterpart in group): GV Octorok Grotto Red Rupee, GV Octorok Grotto Blue Rupee 1, GV Octorok Grotto Blue Rupee 2, GV Octorok Grotto Blue Rupee 3, GV Octorok Grotto Green Rupee 1, GV Octorok Grotto Green Rupee 2, GV Octorok Grotto Green Rupee 3, GV Octorok Grotto Green Rupee 4
- **GerudoValley / scrub** (area has no remake counterpart in dump): GV Deku Scrub Grotto Front, GV Deku Scrub Grotto Rear
- **GoronCity / beehive** (no remake counterpart in group): GC Grotto Beehive
- **GoronCity / chest** (ambiguous: 4 oot vs 6 remake candidates): GC Maze Left Chest, GC Maze Center Chest, GC Maze Right Chest
- **GoronCity / crate** (no remake counterpart in group): GC Boulder Maze Crate
- **GoronCity / freestanding** (no remake counterpart in group): GC Spinning Pot Bomb Drop 1, GC Spinning Pot Bomb Drop 2, GC Spinning Pot Bomb Drop 3, GC Spinning Pot Rupee Drop 1, GC Spinning Pot Rupee Drop 2, GC Spinning Pot Rupee Drop 3, GC Spinning Pot PoH Drop Rupee 1, GC Spinning Pot PoH Drop Rupee 2
- **GoronCity / heart** (ambiguous: 4 oot vs 6 remake candidates): GC Pot Freestanding PoH
- **GoronCity / pot** (ambiguous: 2 oot vs 1 remake candidates of the same item class): GC Darunia Pot 2, GC Upper Staircase Pot 2
- **GoronCity / pot** (ambiguous: 3 oot vs 1 remake candidates of the same item class): GC Darunia Pot 1, GC Darunia Pot 3, GC Lower Staircase Pot 1
- **GoronCity / pot** (no remake counterpart in group): GC Medigoron Pot, GC Lower Staircase Pot 2, GC Upper Staircase Pot 1, GC Upper Staircase Pot 3
- **GoronCity / scrub** (area has no remake counterpart in dump): GC Deku Scrub Grotto Left, GC Deku Scrub Grotto Center, GC Deku Scrub Grotto Right
- **Graveyard / chest** (ambiguous: 4 oot vs 2 remake candidates): Graveyard Heart Piece Grave Chest, Graveyard Royal Familys Tomb Chest
- **Graveyard / crate** (no remake counterpart in group): Graveyard Freestanding PoH Crate
- **Graveyard / heart** (ambiguous: 4 oot vs 2 remake candidates): Graveyard Freestanding PoH, Graveyard Dampe Gravedigging Tour
- **Graveyard / pot** (no remake counterpart in group): Graveyard Dampe Pot 1, Graveyard Dampe Pot 2, Graveyard Dampe Pot 3, Graveyard Dampe Pot 4, Graveyard Dampe Pot 5, Graveyard Dampe Pot 6
- **HountedWasteland / crate** (no remake counterpart in group): Wasteland Crate Before Quicksand, Wasteland Crate After Quicksand 1, Wasteland Crate After Quicksand 2, Wasteland Crate After Quicksand 3, Wasteland Crate Near Colossus
- **HountedWasteland / pot** (no remake counterpart in group): Wasteland Near GS Pot 1, Wasteland Near GS Pot 2, Wasteland Near GS Pot 3, Wasteland Near GS Pot 4
- **HyruleCastle / pot** (no remake counterpart in group): HC Storms Grotto Pot 1, HC Storms Grotto Pot 2, HC Storms Grotto Pot 3, HC Storms Grotto Pot 4
- **HyruleField / beehive** (no remake counterpart in group): HF Near Market Grotto Beehive 1, HF Near Market Grotto Beehive 2, HF Open Grotto Beehive 1, HF Open Grotto Beehive 2, HF Southeast Grotto Beehive 1, HF Southeast Grotto Beehive 2, HF Inside Fence Grotto Beehive, LLR Grotto Beehive, LH Grotto Beehive
- **HyruleField / chest** (ambiguous: 3 oot vs 6 remake candidates): HF Near Market Grotto Chest, HF Southeast Grotto Chest, HF Open Grotto Chest
- **HyruleField / cow** (ambiguous: 5 oot vs 3 remake candidates): HF Cow Grotto Cow, LLR Stables Left Cow, LLR Stables Right Cow, LLR Tower Left Cow, LLR Tower Right Cow
- **HyruleField / crate** (ambiguous: 1 oot vs 3 remake candidates of the same item class): LLR Child Crate
- **HyruleField / freestanding** (ambiguous: 3 oot vs 17 remake candidates of the same item class): LH Underwater Near Shore Green Rupee, LH Underwater Green Rupee 1, LH Underwater Green Rupee 2
- **HyruleField / freestanding** (ambiguous: 3 oot vs 2 remake candidates of the same item class): LH Lab Dive Red Rupee 1, LH Lab Dive Red Rupee 2, LH Lab Dive Red Rupee 3
- **HyruleField / pot** (ambiguous: 4 oot vs 6 remake candidates of the same item class): HF Cow Grotto Pot 2, LLR Talons House Pot 1, LLR Talons House Pot 2, LLR Talons House Pot 3
- **HyruleField / pot** (no remake counterpart in group): HF Cow Grotto Pot 1, LLR Front Pot 1, LLR Front Pot 2, LLR Front Pot 3, LLR Front Pot 4, LLR Rain Shed Pot 1, LLR Rain Shed Pot 2, LLR Rain Shed Pot 3
- **HyruleField / scrub** (ambiguous: 6 oot vs 3 remake candidates): LLR Deku Scrub Grotto Left, LLR Deku Scrub Grotto Center, LLR Deku Scrub Grotto Right, LH Deku Scrub Grotto Left, LH Deku Scrub Grotto Center, LH Deku Scrub Grotto Right
- **IceCavern / flying_pot** (no remake counterpart in group): Ice Cavern Spinning Scythe Flying Pot
- **IceCavern / freestanding** (no remake counterpart in group): Ice Cavern Frozen Blue Rupee, Ice Cavern Map Room Recovery Heart 1, Ice Cavern Map Room Recovery Heart 2, Ice Cavern Map Room Recovery Heart 3, Ice Cavern Push Block Room Red Rupee 1, Ice Cavern Push Block Room Red Rupee 2, Ice Cavern Push Block Room Red Rupee 3
- **IceCavern / heart** (item class conflict, forced only by counts): Ice Cavern Freestanding PoH
- **IceCavern / pot** (ambiguous: 1 oot vs 3 remake candidates of the same item class): Ice Cavern Spinning Scythe Pot 2
- **IceCavern / pot** (no remake counterpart in group): Ice Cavern Hall Pot 1, Ice Cavern Hall Pot 2, Ice Cavern Spinning Scythe Pot 1, Ice Cavern Spinning Scythe Pot 3, Ice Cavern Near End Pot 1, Ice Cavern Near End Pot 2, Ice Cavern Frozen Pot
- **InsideGanonCastle / chest** (ambiguous: 13 oot vs 9 remake candidates): Ganons Castle Forest Trial Chest, Ganons Castle Water Trial Left Chest, Ganons Castle Water Trial Right Chest, Ganons Castle Shadow Trial Front Chest, Ganons Castle Light Trial First Left Chest, Ganons Castle Light Trial Second Left Chest, Ganons Castle Light Trial Third Left Chest, Ganons Castle Light Trial First Right Chest, Ganons Castle Light Trial Second Right Chest, Ganons Castle Light Trial Third Right Chest, Ganons Castle Light Trial Lullaby Chest, Ganons Castle Spirit Trial Crystal Switch Chest, Ganons Castle Spirit Trial Invisible Chest
- **InsideGanonCastle / freestanding** (no remake counterpart in group): Ganons Castle Shadow Trial Recovery Heart 1, Ganons Castle Shadow Trial Recovery Heart 2, Ganons Castle Shadow Trial Recovery Heart 3, Ganons Castle Fire Trial Recovery Heart, Ganons Castle Spirit Trial Recovery Heart
- **InsideGanonCastle / pot** (no remake counterpart in group): 33 locations, first: Ganons Castle Water Trial Pot 1, Ganons Castle Water Trial Pot 2, Ganons Castle Forest Trial Pot 1, Ganons Castle Forest Trial Pot 2, Ganons Castle Light Trial Boulder Pot, Ganons Castle Light Trial Pot 1
- **InsideGanonCastle / scrub** (area has no remake counterpart in dump): Ganons Castle Deku Scrub Left, Ganons Castle Deku Scrub Center-Left, Ganons Castle Deku Scrub Center-Right, Ganons Castle Deku Scrub Right
- **InsideJabuJabuBelly / crate** (no remake counterpart in group): Jabu Jabus Belly Small Wooden Crate 1, Jabu Jabus Belly Small Wooden Crate 2
- **InsideJabuJabuBelly / pot** (no remake counterpart in group): Jabu Jabus Belly Above Big Octo Pot 1, Jabu Jabus Belly Above Big Octo Pot 2, Jabu Jabus Belly Above Big Octo Pot 3, Jabu Jabus Belly Basement 2 Octoroks Pot 5, Jabu Jabus Belly Basement 2 Octoroks Pot 1, Jabu Jabus Belly Basement 2 Octoroks Pot 2, Jabu Jabus Belly Basement 2 Octoroks Pot 3, Jabu Jabus Belly Basement 2 Octoroks Pot 4, Jabu Jabus Belly Basement Switch Room Pot 1, Jabu Jabus Belly Basement Switch Room Pot 2, Jabu Jabus Belly Basement Switch Room Pot 3
- **InsideTheDekuTree / freestanding** (ambiguous: 5 oot vs 9 remake candidates of the same item class): Deku Tree Lower Lobby Recovery Heart, Deku Tree Upper Lobby Recovery Heart, Deku Tree Basement Recovery Heart 1, Deku Tree Basement Recovery Heart 2, Deku Tree Basement Recovery Heart 3
- **KakarikoVillage / beehive** (ambiguous: 1 oot vs 3 remake candidates of the same item class): Kak Open Grotto Beehive 1
- **KakarikoVillage / beehive** (no remake counterpart in group): Kak Open Grotto Beehive 2
- **KakarikoVillage / chest** (ambiguous: 2 oot vs 3 remake candidates): Kak Open Grotto Chest, Kak Redead Grotto Chest
- **KakarikoVillage / crate** (no remake counterpart in group): 18 locations, first: Kak Adult Crate 1, Kak Adult Crate 2, Kak Adult Arrows Crate, Kak Adult Crate 3, Kak Adult Crate 4, Kak Adult Crate 5
- **KakarikoVillage / pot** (no remake counterpart in group): Kak Near Potion Shop Pot 1, Kak Near Potion Shop Pot 2, Kak Near Potion Shop Pot 3, Kak Near Impas House Pot 1, Kak Near Impas House Pot 2, Kak Near Impas House Pot 3, Kak Near Guards House Pot 1, Kak Near Guards House Pot 2, Kak Near Guards House Pot 3, Kak Near Odd Medicine Building Pot 1, Kak Near Odd Medicine Building Pot 2
- **KokiriForest / beehive** (no remake counterpart in group): KF Storms Grotto Beehive 1, KF Storms Grotto Beehive 2
- **KokiriForest / chest** (ambiguous: 1 oot vs 2 remake candidates): KF Storms Grotto Chest
- **KokiriForest / freestanding** (ambiguous: 4 oot vs 7 remake candidates of the same item class): KF Behind Midos Blue Rupee, KF Boulder Maze Blue Rupee 1, KF Boulder Maze Blue Rupee 2, KF End of Bridge Blue Rupee
- **KokiriForest / freestanding** (ambiguous: 4 oot vs 9 remake candidates of the same item class): KF Grass Near Ramp Green Rupee 1, KF Grass Near Ramp Green Rupee 2, KF Grass Near Midos Green Rupee 1, KF Grass Near Midos Green Rupee 2
- **KokiriForest / pot** (no remake counterpart in group): KF Links House Pot
- **LostWood / beehive** (no remake counterpart in group): LW Near Shortcuts Grotto Beehive 1, LW Near Shortcuts Grotto Beehive 2, LW Scrubs Grotto Beehive
- **LostWood / freestanding** (no remake counterpart in group): LW Under Boulder Blue Rupee, LW Underwater Green Rupee 1, LW Underwater Green Rupee 2, LW Underwater Green Rupee 3, LW Underwater Green Rupee 4, LW Underwater Green Rupee 5, LW Underwater Green Rupee 6, LW Underwater Green Rupee 7, LW Underwater Shortcut Green Rupee
- **LostWood / gs** (item class conflict, forced only by counts): LW GS Above Theater
- **LostWood / scrub** (ambiguous: 2 oot vs 4 remake candidates): LW Deku Scrub Near Deku Theater Left, LW Deku Scrub Near Deku Theater Right
- **Market / crate** (ambiguous: 4 oot vs 1 remake candidates of the same item class): Market Night Green Rupee Crate 1, Market Night Green Rupee Crate 2, Market Night Green Rupee Crate 3, Market Guard House Child Crate 4
- **Market / crate** (no remake counterpart in group): Market Night Red Rupee Crate, Market Day Crate 1, Market Day Crate 2, Market Day Crate 3, Market Day Crate 4, Market Dog Lady House Crate, Market Guard House Child Crate 1, Market Guard House Child Crate 2, Market Guard House Child Crate 3
- **Market / pot** (ambiguous: 41 oot vs 16 remake candidates of the same item class): 41 locations, first: Market Guard House Child Pot 1, Market Guard House Child Pot 2, Market Guard House Child Pot 3, Market Guard House Child Pot 4, Market Guard House Child Pot 6, Market Guard House Child Pot 7
- **Market / pot** (no remake counterpart in group): 17 locations, first: Market Guard House Child Pot 5, Market Guard House Child Pot 8, Market Guard House Child Pot 10, Market Guard House Child Pot 12, Market Guard House Child Pot 13, Market Guard House Child Pot 15
- **SacredForestMeadow / beehive** (no remake counterpart in group): SFM Storms Grotto Beehive
- **SacredForestMeadow / chest** (ambiguous: 1 oot vs 2 remake candidates): SFM Wolfos Grotto Chest
- **SacredForestMeadow / scrub** (area has no remake counterpart in dump): SFM Deku Scrub Grotto Front, SFM Deku Scrub Grotto Rear
- **ShadowTemple / chest** (ambiguous: 8 oot vs 6 remake candidates): Shadow Temple Invisible Blades Visible Chest, Shadow Temple Invisible Blades Invisible Chest, Shadow Temple Falling Spikes Lower Chest, Shadow Temple Falling Spikes Upper Chest, Shadow Temple Invisible Spikes Chest, Shadow Temple Wind Hint Chest, Shadow Temple After Wind Enemy Chest, Shadow Temple Spike Walls Left Chest
- **ShadowTemple / flying_pot** (no remake counterpart in group): Shadow Temple Whispering Walls Flying Pot, Shadow Temple After Wind Flying Pot 1, Shadow Temple After Wind Flying Pot 2
- **ShadowTemple / freestanding** (no remake counterpart in group): 16 locations, first: Shadow Temple Invisible Blades Recovery Heart 1, Shadow Temple Invisible Blades Recovery Heart 2, Shadow Temple Before Boat Recovery Heart 1, Shadow Temple Before Boat Recovery Heart 2, Shadow Temple After Boat Upper Recovery Heart 1, Shadow Temple After Boat Upper Recovery Heart 2
- **ShadowTemple / pot** (no remake counterpart in group): 15 locations, first: Shadow Temple Whispering Walls Left Pot 2, Shadow Temple Whispering Walls Front Pot 1, Shadow Temple Whispering Walls Front Pot 2, Shadow Temple Map Chest Room Pot 1, Shadow Temple Map Chest Room Pot 2, Shadow Temple Falling Spikes Lower Pot 1
- **SpiritTemple / chest** (ambiguous: 7 oot vs 5 remake candidates): Spirit Temple Child Bridge Chest, Spirit Temple Child Climb North Chest, Spirit Temple Child Climb East Chest, Spirit Temple First Mirror Left Chest, Spirit Temple First Mirror Right Chest, Spirit Temple Statue Room Northeast Chest, Spirit Temple Topmost Chest
- **SpiritTemple / crate** (no remake counterpart in group): Spirit Temple Before Child Climb Small Wooden Crate 1, Spirit Temple Before Child Climb Small Wooden Crate 2
- **SpiritTemple / flying_pot** (no remake counterpart in group): Spirit Temple Lobby Flying Pot 1, Spirit Temple Lobby Flying Pot 2, Spirit Temple Child Bridge Flying Pot 1, Spirit Temple Child Bridge Flying Pot 2, Spirit Temple Central Chamber Flying Pot 1, Spirit Temple Central Chamber Flying Pot 2, Spirit Temple Adult Climb Flying Pot 1, Spirit Temple Adult Climb Flying Pot 2, Spirit Temple Big Mirror Flying Pot 1, Spirit Temple Big Mirror Flying Pot 2, Spirit Temple Big Mirror Flying Pot 3, Spirit Temple Big Mirror Flying Pot 4, Spirit Temple Big Mirror Flying Pot 5, Spirit Temple Big Mirror Flying Pot 6
- **SpiritTemple / freestanding** (no remake counterpart in group): Spirit Temple Shifting Wall Recovery Heart 1, Spirit Temple Shifting Wall Recovery Heart 2
- **SpiritTemple / pot** (no remake counterpart in group): 16 locations, first: Spirit Temple Lobby Pot 1, Spirit Temple Lobby Pot 2, Spirit Temple Child Climb Pot, Spirit Temple Hall After Sun Block Room Pot 1, Spirit Temple Hall After Sun Block Room Pot 2, Spirit Temple Beamos Hall Pot
- **WaterTemple / chest** (ambiguous: 6 oot vs 5 remake candidates): Water Temple Cracked Wall Chest, Water Temple Torches Chest, Water Temple Central Pillar Chest, Water Temple Central Bow Target Chest, Water Temple River Chest, Water Temple Dragon Chest
- **WaterTemple / freestanding** (no remake counterpart in group): Water Temple River Recovery Heart 1, Water Temple River Recovery Heart 2, Water Temple River Recovery Heart 3, Water Temple River Recovery Heart 4
- **WaterTemple / pot** (ambiguous: 5 oot vs 1 remake candidates of the same item class): Water Temple Near Boss Pot 1, Water Temple Near Boss Pot 2, Water Temple Boss Key Pot 1, Water Temple Boss Key Pot 2, Water Temple River Pot 2
- **WaterTemple / pot** (no remake counterpart in group): 18 locations, first: Water Temple Main Room L2 Pot 1, Water Temple Main Room L2 Pot 2, Water Temple Behind Gate Pot 1, Water Temple Behind Gate Pot 2, Water Temple Behind Gate Pot 3, Water Temple Behind Gate Pot 4
- **ZoraDomain / beehive** (no remake counterpart in group): ZD In Front of King Zora Beehive 1, ZD In Front of King Zora Beehive 2, ZD Behind King Zora Beehive
- **ZoraDomain / pot** (no remake counterpart in group): ZD Pot 1, ZD Pot 2, ZD Pot 3, ZD Pot 4, ZD Pot 5
- **ZoraFountain / freestanding** (ambiguous: 18 oot vs 16 remake candidates of the same item class): 18 locations, first: ZF Bottom Green Rupee 1, ZF Bottom Green Rupee 2, ZF Bottom Green Rupee 3, ZF Bottom Green Rupee 4, ZF Bottom Green Rupee 5, ZF Bottom Green Rupee 6
- **ZoraFountain / pot** (no remake counterpart in group): ZF Hidden Cave Pot 1, ZF Hidden Cave Pot 2, ZF Hidden Cave Pot 3, ZF Near Jabu Pot 1, ZF Near Jabu Pot 2, ZF Near Jabu Pot 3, ZF Near Jabu Pot 4
- **ZoraRiver / beehive** (no remake counterpart in group): ZR Open Grotto Beehive 1, ZR Open Grotto Beehive 2, ZR Storms Grotto Beehive
- **ZoraRiver / freestanding** (no remake counterpart in group): ZR Waterfall Red Rupee 1, ZR Waterfall Red Rupee 2, ZR Waterfall Red Rupee 3, ZR Waterfall Red Rupee 4
- **ZoraRiver / gs** (item class conflict, forced only by counts): ZR GS Tree
- **ZoraRiver / scrub** (area has no remake counterpart in dump): ZR Deku Scrub Grotto Front, ZR Deku Scrub Grotto Rear

Every entry above with fewer than 15 names is listed in full. Longer groups are truncated here and listed in full, one by one with its reason, in `unmatched_oot` in `data/locations.json`.

## Unmatched remake actors and reasons

| Folder | Kind | Reason | Count | Items |
|---|---|---|---|---|
| BottomOfTheWell | pot | ambiguous: 7 oot vs 1 remake candidates of the same item class | 1 | Obj_Rupee_Blue x1 |
| BottomOfTheWell | scrub | no oot location of this kind in area | 1 | Enemy_Scrub_Business x1 |
| DeathMountainCrater | chest | ambiguous: 1 oot vs 2 remake candidates | 2 | Item_ScrubBusinessSeed x1, TBox_Rupee_Purple x1 |
| DeathMountainCrater | crate | no oot counterpart in group | 1 | Obj_Rupee_Green x1 |
| DeathMountainCrater | freestanding | ambiguous: 2 oot vs 1 remake candidates of the same item class | 1 | Obj_Rupee_Red x1 |
| DeathMountainCrater | freestanding | no oot counterpart in group | 9 | Obj_Rupee_Green x8, Obj_Rupee_Purple x1 |
| DeathMountainCrater | scrub | ambiguous: 4 oot vs 1 remake candidates | 1 | Enemy_Scrub_Business x1 |
| DeathMountainTrail | chest | ambiguous: 2 oot vs 1 remake candidates | 1 | Item_ScrubBusinessSeed x1 |
| DeathMountainTrail | freestanding | ambiguous: 1 oot vs 2 remake candidates of the same item class | 2 | Obj_Rupee_Blue x2 |
| DeathMountainTrail | freestanding | no oot counterpart in group | 3 | Obj_Rupee_Green x3 |
| DesertColossus | chest | ambiguous: 7 oot vs 5 remake candidates | 1 | TBox_Rupee_Purple x1 |
| DesertColossus | gs | ambiguous pool with class conflicts | 2 | Enemy_Skulltula_Gold x2 |
| DesertColossus | other_drop | no oot location of this kind in area | 7 | Obj_Rupee_Blue x3, Obj_Rupee_Green x3, Obj_Rupee_Red x1 |
| DodongoCavern | chest | ambiguous: 3 oot vs 6 remake candidates | 6 | Item_ScrubBusinessSeed x2, TBox_Rupee_Blue x1, TBox_Rupee_Red x1, TBox_Shield_Deku x2 |
| DodongoCavern | pot | ambiguous: 4 oot vs 1 remake candidates of the same item class | 1 | Obj_Rupee_Green x1 |
| DodongoCavern | pot | ambiguous: 8 oot vs 2 remake candidates of the same item class | 2 | Obj_Rupee_Blue x2 |
| DodongoCavern | scrub | ambiguous: 4 oot vs 1 remake candidates | 1 | Enemy_Scrub_Business x1 |
| DreamWorld | chest | no oot location of this kind in area | 10 | TBox_Rupee_Gold x10 |
| DreamWorld | other_drop | no oot location of this kind in area | 7 | Obj_DekuSeed x2, Obj_Heart x5 |
| DreamWorld | pot | no oot counterpart in group | 8 | Obj_Fairy x1, Obj_Heart x2, Obj_MagicJar_Small x5 |
| FireTemple | chest | ambiguous: 2 oot vs 4 remake candidates | 4 | TBox_Rupee_Purple x1, TBox_Rupee_Red x3 |
| FireTemple | freestanding | ambiguous: 9 oot vs 2 remake candidates of the same item class | 2 | Obj_Heart x2 |
| FireTemple | pot | no oot counterpart in group | 3 | Obj_Rupee_Blue x2, Obj_Rupee_Red x1 |
| ForestTemple | chest | ambiguous: 4 oot vs 2 remake candidates | 2 | TBox_ArrowBundle_Large x1, TBox_Rupee_Purple x1 |
| GanonCastle | pot | no oot counterpart in group | 9 | Obj_ArrowBundle_Small x1, Obj_Bomb x1, Obj_Heart x4, Obj_MagicJar_Small x3 |
| GerudoFortress | chest | no oot counterpart in group | 1 | TBox_Rupee_Red x1 |
| GerudoFortress | cow | no oot location of this kind in area | 1 | Animal_Cow_GerudoFortress x1 |
| GerudoFortress | crate | no oot counterpart in group | 1 | Obj_Rupee_Blue x1 |
| GerudoFortress | flying_pot | no oot counterpart in group | 3 | Obj_Rupee_Blue x3 |
| GerudoTrainingGround | chest | ambiguous: 12 oot vs 10 remake candidates | 10 | Item_BombBundle_Small x1, Item_BombchuBundle_Small x1, Item_ScrubBusinessSeed x1, TBox_Rupee_Blue x2, TBox_Rupee_Purple x1 |
| GerudoTrainingGround | other_drop | no oot location of this kind in area | 1 | Obj_Rupee_Red x1 |
| GerudoValley | chest | ambiguous: 1 oot vs 5 remake candidates | 5 | Item_ScrubBusinessSeed x1, TBox_Rupee_Green x1, TBox_Rupee_Red x3 |
| GerudoValley | pot | no oot counterpart in group | 4 | Obj_Rupee_Green x4 |
| GoronCity | chest | ambiguous: 4 oot vs 6 remake candidates | 6 | TBox_Rupee_Blue x2, TBox_Rupee_Green x1, TBox_Rupee_Purple x2, TBox_Rupee_Red x1 |
| GoronCity | pot | ambiguous: 2 oot vs 1 remake candidates of the same item class | 1 | Obj_Rupee_Green x1 |
| GoronCity | pot | ambiguous: 3 oot vs 1 remake candidates of the same item class | 1 | Obj_DekuStick x1 |
| Graveyard | chest | ambiguous: 4 oot vs 2 remake candidates | 1 | Obj_Heart_Piece x1 |
| Graveyard | chest | no oot counterpart in group | 2 | TBox_Rupee_Blue x1, TBox_Rupee_Red x1 |
| Graveyard | heart | ambiguous: 4 oot vs 2 remake candidates | 1 | Obj_Heart_Piece x1 |
| HyruleField | chest | ambiguous: 3 oot vs 6 remake candidates | 6 | Item_ScrubBusinessSeed x1, TBox_Rupee_Purple x1, TBox_Rupee_Red x4 |
| HyruleField | cow | ambiguous: 5 oot vs 3 remake candidates | 3 | Animal_Cow x3 |
| HyruleField | crate | ambiguous: 1 oot vs 3 remake candidates of the same item class | 3 | Obj_Rupee_Green x3 |
| HyruleField | crate | no oot counterpart in group | 2 | Obj_Rupee_Red x2 |
| HyruleField | freestanding | ambiguous: 3 oot vs 17 remake candidates of the same item class | 17 | Obj_Rupee_Green x17 |
| HyruleField | freestanding | ambiguous: 3 oot vs 2 remake candidates of the same item class | 2 | Obj_Rupee_Red x2 |
| HyruleField | freestanding | no oot counterpart in group | 7 | Obj_Rupee_Blue x7 |
| HyruleField | other_drop | no oot location of this kind in area | 16 | Animal_Insect x13, Obj_Rupee_Blue x3 |
| HyruleField | pot | ambiguous: 4 oot vs 6 remake candidates of the same item class | 6 | Obj_Rupee_Blue x6 |
| HyruleField | pot | no oot counterpart in group | 1 | Obj_Fairy x1 |
| HyruleField | scrub | ambiguous: 6 oot vs 3 remake candidates | 3 | Enemy_Scrub_Business x3 |
| IceCavern | chest | item class conflict, forced only by counts | 1 | TBox_Rupee_Red x1 |
| IceCavern | pot | ambiguous: 1 oot vs 3 remake candidates of the same item class | 3 | Obj_Rupee_Blue x3 |
| IceCavern | pot | no oot counterpart in group | 3 | Obj_Fairy x1, Obj_MagicJar_Small x1, Obj_Rupee_Red x1 |
| InsideGanonCastle | chest | ambiguous: 13 oot vs 9 remake candidates | 9 | Item_BombchuBundle_Small x1, Item_ScrubBusinessSeed x1, TBox_ArrowBundle_Medium x2, TBox_Rupee_Blue x3, TBox_Rupee_Green x1 |
| InsideGanonCastle | pot | no oot counterpart in group | 11 | Obj_MagicJar_Small x11 |
| InsideJabuJabuBelly | chest | no oot counterpart in group | 1 | TBox_Rupee_Red x1 |
| InsideTheDekuTree | freestanding | ambiguous: 5 oot vs 9 remake candidates of the same item class | 9 | Obj_Heart x6, Obj_Heart_InWater x3 |
| InsideTheDekuTree | freestanding | no oot counterpart in group | 3 | Obj_Rupee_Green x3 |
| InsideTheDekuTree | other_drop | no oot location of this kind in area | 9 | Obj_DekuSeed x4, Obj_Heart x4, Obj_Rupee_Green x1 |
| KakarikoVillage | beehive | ambiguous: 1 oot vs 3 remake candidates of the same item class | 3 | Obj_Rupee_Blue x3 |
| KakarikoVillage | chest | ambiguous: 2 oot vs 3 remake candidates | 3 | TBox_Rupee_Blue x1, TBox_Rupee_Purple x2 |
| KokiriForest | chest | ambiguous: 1 oot vs 2 remake candidates | 2 | TBox_Rupee_Green x1, TBox_Rupee_Purple x1 |
| KokiriForest | freestanding | ambiguous: 4 oot vs 7 remake candidates of the same item class | 7 | Obj_Rupee_Blue x7 |
| KokiriForest | freestanding | ambiguous: 4 oot vs 9 remake candidates of the same item class | 9 | Obj_Rupee_Green x9 |
| KokiriForest | freestanding | no oot counterpart in group | 1 | Obj_Rupee_Purple x1 |
| KokiriForest | scrub | no oot location of this kind in area | 1 | Enemy_Scrub_Business x1 |
| LostWood | gs | item class conflict, forced only by counts | 1 | Enemy_Skulltula_Gold x1 |
| LostWood | other_drop | no oot location of this kind in area | 9 | Animal_Insect x9 |
| LostWood | scrub | ambiguous: 2 oot vs 4 remake candidates | 4 | Enemy_Scrub_Business x4 |
| Market | chest | no oot counterpart in group | 1 | TBox_Rupee_Red x1 |
| Market | crate | ambiguous: 4 oot vs 1 remake candidates of the same item class | 1 | Obj_Rupee_Green x1 |
| Market | pot | ambiguous: 41 oot vs 16 remake candidates of the same item class | 16 | Obj_Rupee_Green x16 |
| SacredForestMeadow | chest | ambiguous: 1 oot vs 2 remake candidates | 2 | Item_ScrubBusinessSeed x1, TBox_Rupee_Purple x1 |
| ShadowTemple | chest | ambiguous: 8 oot vs 6 remake candidates | 6 | Item_ScrubBusinessSeed x1, TBox_Rupee_Green x1, TBox_Rupee_Red x3, Tbox_MagicJar_Large x1 |
| ShadowTemple | pot | no oot counterpart in group | 1 | Obj_Fairy x1 |
| ShadowTemple | scrub | no oot location of this kind in area | 1 | Enemy_Scrub_Business x1 |
| SpiritTemple | chest | ambiguous: 7 oot vs 5 remake candidates | 4 | Item_BombBundle_Small x2, Item_BombchuBundle_Small x1, TBox_Shield_Deku x1 |
| SpiritTemple | other_drop | no oot location of this kind in area | 13 | Obj_Heart x13 |
| WaterTemple | chest | ambiguous: 6 oot vs 5 remake candidates | 5 | Item_Dungeon_SmallKey x4, TBox_Rupee_Red x1 |
| WaterTemple | pot | ambiguous: 5 oot vs 1 remake candidates of the same item class | 1 | Obj_Fairy x1 |
| ZoraDomain | freestanding | no oot counterpart in group | 6 | Obj_Rupee_Blue x6 |
| ZoraFountain | chest | no oot counterpart in group | 1 | TBox_Rupee_Purple x1 |
| ZoraFountain | freestanding | ambiguous: 18 oot vs 16 remake candidates of the same item class | 16 | Obj_Rupee_Green x16 |
| ZoraRiver | freestanding | no oot counterpart in group | 20 | Obj_Rupee_Blue x3, Obj_Rupee_Green x17 |
| ZoraRiver | gs | item class conflict, forced only by counts | 1 | Enemy_Skulltula_Gold x1 |
| ZoraRiver | other_drop | no oot location of this kind in area | 1 | Animal_Insect x1 |

Full per-actor entries, with hashes and files, are in `unmatched_remake` in `data/locations.json`.

## Revision 2 matching, what was tried

- Age feasibility is a hard constraint. A remake actor that exists only in the Adult file cannot hold an OoTR location that no adult can reach, and the same for Child. Checked on the 99 revision 1 anchors: 28 have a single age remake actor and 0 contradict the constraint. VERIFIED.
- A nocturnal remake Gold Skulltula can only hold an OoTR night location. 5 of 5 anchors agree. The reverse does not hold, 2 of 7 night locations have a remake skulltula that is not nocturnal, so that direction is only a tie break. VERIFIED on a small sample.
- Region to room: OoTR regions are not rooms. Of 8 regions with two or more anchors, 0 have all their anchors in one remake room. Not used.
- Chest flag number order against remake room order: monotone in only 3 of 12 dungeons. Not used.
- OoTR `LocationList` order against remake room order: 61 of 73 anchor pairs agree. Weak, used only to order members inside a pool.
- Developer room labels exist for Shadow Temple and Bottom of the Well only, the other dungeons use plain numbers. They decide the pins coded `lbl`.
- Named neighbor actors decide the pins coded `nbr` and `lm`. The remake places landmark actors (scarecrow markers, enemy names, tower and windmill models, entrance actors) next to many targets. Each such pin names the actor token it rests on and was checked against the neighbor list in the dump.
- The Treasure Chest Game rooms are a row of 5 with the pinned last room at one end, which fixes the order of the other four (`lin`, `anchor`).
- Every pin still needs a play-through to be fully trusted. `ootrr calibrate <romfs> <load_dir> <mod_name> <area>` writes a calibration mod for that.

## Findings about the remake data

- Gold Skulltula count is exactly 100 distinct positions, the same as OoTR, and per-area counts agree (HyruleField folder = Hyrule Field 2 + Lake Hylia 5 + Lon Lon Ranch 4). VERIFIED by position clustering of Adult and Child files.
- Chest contents do not follow N64 vanilla. Maps, compasses, boss keys, small-key counts and major items agree with OoTR in most dungeons, but filler chests (hearts, nuts, sticks) were replaced by rupees, seeds and bundles, and some areas have more or fewer chests than OoTR (Dodongo 9 vs 6, Gerudo Valley 6 vs 1, Water Temple small keys 4 vs 6).
- Adult and Child files share hashes for chests, but not for most other actors. A cow, skulltula, scrub or heart piece usually has a different hash in each age file and sometimes a slightly different position, so those locations list several refs.
- Child and Adult copies of the same breakable can hold different `Drop__DropActor` values, so they are separate candidates.
- Fire Temple: the chest with `GetCompass` as its event name holds the map item and the one with `GetMap` holds the compass item. The catalog follows the item actor, not the event name.
- Heart pieces are `Obj_Heart_Piece` actors referenced by a `HeartPieceAnchor` whose `HeartPieceAnchor_GameDataPath` names the source. Minigame, NPC and scrub sources were excluded as event rewards.
- Thieves Hideout carpenter keys are `Obj_Dungeon_SmallKey_GerudoFortress_01..04`, spawned by the guard actors (`Enemy_MiddleBoss_GerudoThief*`, `Drop__DropActor` set). The number to torch count mapping is a GUESS.
- Several overworld chests hold `Item_ScrubBusinessSeed`, which is not an OoTR vanilla item for those spots.
- Mido's House chests share the event `TreasureBox_NaviChatOff` and the Treasure Chest Game chests carry `ChoiceTreasure_Open*` events, which were used to split pools.
