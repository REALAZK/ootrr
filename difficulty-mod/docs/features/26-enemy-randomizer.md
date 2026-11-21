# Enemy randomizer

Setting `enemy_randomizer` (number 150, was 138 before rc 19), checkbox on the Difficulty page, Enemies section, after "More enemies". Default off. Code: `randomizer/core/src/difficulty/swap.rs`. Branch `feature/enemy-randomizer` from `playtest/next`, worktree `worktree enemyrando`. Exe: `<playtest folder>\enemyrando\ootrr.exe`. Version 2, built 20 Nov (after your v1 play) by an Opus agent with no game run (the user was playing). Load-test plan: `<playtest folder>\enemyrando\PLAN.md`.

## Request
20 Nov: "new feature: enemy randomizer", like Ship of Harkinian's. The design is "Our design proposal" in `docs\research-soh.md`.

Correction from another modder, endorsed by you (20 Nov): in this remake a Deku Stick breaks after one hit, so a child may have no sword and zero or one stick. Sticks are never counted on to kill anything; for child maps "killable" means killable with the Kokiri Sword alone. Any room or flag that needs enemies killed (room-clear counters, enemy groups that open doors or chests, events waiting on kills) keeps its original enemies untouched, and a game-file test lists the counted actors of each child dungeon and checks none changed.

## Interpretation
- Like SoH's "Random (Seeded)": each placed ordinary enemy may become another kind, fixed by the seed. We edit the map placement files at build time (data only), so it works on every runtime.
- Version 1 only used kinds that the same map already places, so the kind's actor pack was certainly loaded there.
- Your feedback on v1 (20 Nov): "the enemies don't feel particularly randomised" (Fire Temple mostly swapped fire Keese and Keese; adult Hyrule Field Guays and Poes traded places). Version 2 draws from every map, changes about four in five of the enemies that may change, and spreads the kinds so a room gets a mix. The safety rules are the same, plus two new locks (below).

## What makes a kind appear outside its home map (from the game files)
- There is no per-map resource or preload list to extend. A map's Banc file holds `Actors`, `AiGroups`, `FarDeleteGroups`, `SimultaneousGroups`, `Rails`, `PlgGdTable`, `PhiveHeader` and `FilePath`, nothing that names actor packs. `Banc/GameBancParam` and `Scene/Component/DungeonParam` hold room and map settings, not actor lists.
- Nothing enemy-like is resident. `Scene/Component/ResidentActorMgrParam/Default` lists only the player, cameras and system actors; `Pack/ResidentCommon.pack.zs` (145 files) holds no actor pack. Every `Enemy_*` kind has its own `Pack/Actor/<kind>.pack.zs`.
- So a map's actor list is its load list: whatever `Gyaml` a placement names is loaded with the map. This is exactly how the horde makes kinds that "never live in the area" appear at 5x: it adds a placement copied (as a template) from any map where the kind lives, and nothing else. A swap does the same by renaming an existing placement. Seen working in the game: Like Likes, Dead Hand and Iron Knuckles added by the horde in Hyrule Field, and the silver Iron Knuckle placed by special encounters in Hyrule Field.
- Kinds created at run time (spawn combo, backup calls) are a different path. Created in child Hyrule Field, 38 of the spawn combo's 43 kinds appeared; blue Bubble, Gibdo, fire Keese, big Poe and the silver Iron Knuckle did not. In today's Forest Temple boss room Moblin, Wolfos, Skulltula, Poe, Octorok, Dinolfos, ReDead, Stalfos, Deku Baba and Stalchild appeared, blue Bubble did not. The packs of the failing kinds have the same structure as the working ones (compared Keese / fire Keese, red / blue Bubble, ReDead / Gibdo, Poe / big Poe), so the files do not say why. The silver Iron Knuckle failed when created but works when placed, so placement is at least as strong.
- **Safe anywhere** (placed, and also seen appearing when created away from home): Wolfos, Floormaster, Dodongo, Like Like, ReDead, Baby Dodongo, Torch Slug, Deku Baba, red Tektite, Keese, Ice Keese, Guay, Poe, green and red Bubble, Shabom, Biri, Skulltula, Wallmaster.
- **Only where the map already has it**: fire Keese (never seen away from home; the horde's `HOME_ONLY` rule). Blue Bubble, Gibdo and big Poe stay out of the pool altogether.

## How it works
For each map (both its `_Static` and `_Dynamic` files), seeded by `seed | enemy-randomizer | map`:
1. **Candidates.** A vanilla enemy whose placement has only the plain keys (`Dynamic`, `Gyaml`, `Hash`, `Phive`, `Rotate`, `SRTHash`, `Translate`; so no `Links`, `Rails`, `Name`, `Presence`, `Scale`...), whose `Dynamic` holds no event, game-data or drop key (`extras::tied`), whose hash appears exactly twice in all map files (its own `Hash` and `Phive` id: nothing else, no counter, AI group, link or rail, points at it; the same test the 2x copies use), and whose kind is in the pool below.
2. **Pool.** `horde::KINDS` minus the miniboss class (Stalfos, Dinolfos, silver Iron Knuckle, white Wolfos), minus wall Skulltulas (their placement is turned onto a wall), blue Tektites (they stand on water) and Armos (statues that may sit on a puzzle spot; SoH keeps one in the Spirit Temple; dropped after the first count showed 3 Armos becoming Wolfos in the Spirit Temple and Ganon's Castle). A replacement must be in the same ground class: flat (Wolfos, Floormaster, Dodongo), gentle slope (Like Like, ReDead, Baby Dodongo, Torch Slug, Deku Baba), any ground (red Tektite), fliers (Keese, ice and fire Keese, Guay, Poe, green and red Bubble, Shabom, Biri), ceiling (Skulltula, Wallmaster). Water (Octorok) and sand (Leever) only have one kind, so they never change. The original kind is not a choice.
3. **From any map (v2).** The kinds offered are every pool kind with a parameter shape anywhere in the game (next point), of the same class, that fits the map (`fits`):
   - fire Keese only in a map that already places one (see "What makes a kind appear");
   - ceiling kinds (Skulltula, Wallmaster) only in dungeons, Guays only outdoors (the horde's rules);
   - Keese of any kind only replace Keese of any kind: Keese hang from a surface and a Keese in open air has no vanilla example, so a Guay, Poe, Bubble or Biri spot never becomes a Keese;
   - Dodongos and Baby Dodongos are not brought into a map with a bomb wall (they blast it) unless they already live there, and never in Dodongo's Cavern;
   - in the Water Temple (rising water) only fliers or kinds already there.
4. **How many and which (v2).** Each candidate with at least one option changes with chance 80 in 100 (`SHARE`), so about four in five change and a few originals stay. The new kind is drawn with weights that favour kinds the room does not have yet: weight 12 / (1 + 2 x that kind's count in the room), at least 1. "Room" is the dungeon room index, outdoors a 40-unit square. Counts start from the vanilla enemies there and follow each swap.
5. **Parameters.** The new enemy gets the `Dynamic` of a placement of the new kind: the map's own if the kind lives there, otherwise the best one from any map (plain-keyed ones only, preferring one that nothing references, then the one with fewest keys), with event, game-data, drop and room keys removed. Parameters taken from another map also lose keys that only mean something where they came from (`PLACE_KEYS`: the ReDead `IsBottomOfTheWell`, named perception ranges, the Keese jump-out, the Bubble `ApperMinHeightLimit`). Its original room index is put back. Its own old kind-specific keys are dropped. Hash, physics id, rotation and position stay; fliers and ceiling kinds are raised or lowered by the difference of their usual height above the floor (`horde` lift values, e.g. Keese 3.5 to Bubble 1.5 moves it down 2).
6. **Never touched.**
   - anything a counter, AI group, link, rail, name or event refers to (point 1), so a room that opens when enemies are beaten still has the very enemies it counts;
   - bosses and minibosses (not in the pool), every enemy in a dungeon room that holds a `Enemy_DungeonBoss*` or `Enemy_MiddleBoss*`, and outdoors anything within 40 units of a boss or 25 of a miniboss;
   - Gold Skulltulas, scrubs, Poe sisters and everything on the horde's "never placed" list (not in the pool);
   - the maps the horde skips (towns, Zora's Domain, Goron City, Gerudo Fortress, Hyrule Castle, Gerudo Training Ground (timed rooms), the collapse, the Sage room, the final battle, Haunted Wasteland child, the peaceful places: child Kokiri Forest, Kakariko), `Demo_` stages and the dream world;
   - the SoH special cases, by map and kind (`KEEP`): Dodongo's Cavern Lizalfos, Sacred Forest Meadow Wolfos, Forest Temple and Shadow Temple Stalfos, Water Temple Shell Blades and Spikes, Spirit Temple Iron Knuckles, Lost Woods Skull Kids; and in Dodongo's Cavern every Dodongo and Baby Dodongo (bomb-wall helpers), which are also never offered as a replacement there. Most of these are already out of the pool or referenced; the list is a second lock.
   - enemies other difficulty features added (only hashes present in the vanilla file are touched);
   - (v2) Deku Babas on child maps: they are the child's source of Deku Sticks and Nuts (torches, webs). They can still be a replacement there.
7. **Kill rule (beatability).** Each kind is tagged "Kokiri Sword alone is enough" or "needs more" (`HARD`: Armos, Wallmaster, Octorok, Freezard, Beamos, Shell Blade, Spike, Anubis; only the Wallmaster can actually come up now). Deku Sticks are never counted on: one hit breaks them. On a child map (any `_Child` map, the Deku Tree, Dodongo's Cavern, Jabu-Jabu, Bottom of the Well, the Spirit Temple) every replacement must be sword-only killable. In an adult dungeon, a room that holds an enemy counter or any enemy something refers to only takes sword-only kinds; outdoors the same within 30 units of one.
8. **Logic stays safe.** The item logic is not changed. A swapped enemy is never one a counter, group, link or event needs, so no door, chest or flag waits on it; the player can always walk past it or leave.

### Order
It runs after the 2x copies, boss minibosses and minions, the Zora ambush, special encounters and surprises, Cucco and gates, and just before the horde fill (above 2x). Reasons: the copy step keeps its kind list and random stream, so 1.5x and 2x stay byte for byte as before; any link a later-added feature makes to a vanilla enemy (the gates guards, encounters) is already in the files when the reference check runs, so such an enemy is left alone; and the horde sees the final kinds when it picks "home" kinds near existing enemies. A copy made by the 2x step keeps the original kind (it is not vanilla), so with both on a swapped enemy can stand next to a copy of what it used to be.

### Build report
With the setting on, the report has the feature note and one line: `Enemy randomizer swapped N enemies in M maps: <map> <count>, ...`. (`difficulty::worked` now returns notes; `slice/edit.rs` appends them.)

### Numbers (seed "game")
**v2:** 91 enemies swapped in 16 maps, of 117 candidates that have at least one option (78%). Kinds now used as replacements (15): Deku Baba 12, Poe 11, Like Like 11, Shabom 8, green Bubble 8, red Bubble 8, Biri 7, Torch Slug 6, ReDead 5, Guay 4, Ice Keese 4, Baby Dodongo 3, Keese 2, Wolfos 1, fire Keese 1.

| Map | v1 swaps | v2 swaps / candidates |
|---|---|---|
| Fire Temple | 22 | 18 / 22 |
| Dodongo's Cavern | 8 | 7 / 8 |
| Jabu-Jabu | 8 | 7 / 8 |
| Bottom of the Well | 8 | 6 / 7 (its Deku Baba is now kept) |
| Ice Cavern | 6 | 6 / 6 |
| Spirit Temple | 5 | 4 / 5 |
| Ganon's Castle | 5 | 3 / 5 |
| Shadow Temple | 2 | 3 / 4 |
| Forest Temple | 0 | 2 / 2 |
| Water Temple | 0 | 1 / 2 |
| Hyrule Field adult | 14 | 10 / 14 |
| Market adult | 13 | 9 / 13 |
| Graveyard adult | 3 | 8 / 11 |
| Graveyard child | 0 | 4 / 4 |
| Hyrule Field child | 0 | 2 / 3 |
| Desert Colossus adult | 0 | 1 / 1 |
| Desert Colossus child | 0 | 0 / 1 |
| Total | 94 in 11 maps | 91 in 16 maps / 117 |

v1 swapped every candidate that had another kind of its class in its own map, which was mostly the same few pairs (fire Keese to Keese 14, ReDead to Deku Baba 12, Guay to Poe 9, Biri to Shabom 8, Torch Slug to Like Like 8). v2 has about the same count but each map now gets a mix: the Fire Temple's 18 go to 9 different kinds (Poe, Ice Keese, Biri, Shabom, red Bubble, Deku Baba, ReDead, Like Like, fire Keese), adult Hyrule Field's 10 to Poe, Guay, Biri, Shabom and red Bubble.

**Why the count did not go up.** The pool is no longer the limit; the v1 candidate rules are. Of the 411 vanilla enemies of pool kinds in the maps we edit, 187 are free; the rest: 102 are referenced by an AI group (most also by the streaming groups `FarDeleteGroups` / `SimultaneousGroups`), 7 by another actor, 36 carry an event, game-data or drop key, 79 carry extra placement keys (`Name` 39, `Links` 34, `Presence` 26, `IsPhysicsStable` 9, `IsForceActive` 8). Of the 187 free ones, 29 have no other kind in their class (26 Octoroks, 3 red Tektites), 18 Skulltulas have none (they are on child maps or in counted rooms, where the Wallmaster, the only other ceiling kind, is not allowed), and the rest are in boss rooms, near minibosses, or child Deku Babas. Loosening any of these is a question for you (`suggested-improvements.md`): allowing `Presence` (22 enemies with only that extra key) and `IsPhysicsStable` (9) would add about 30, all still unreferenced.

Every swapped spot (map, kinds, room, position) is in `PLAN.md`.

Child dungeons, what is kept because something needs it (from the game-file test): Deku Tree (56 AI groups, no counter actor) 3 Deku Babas, 3 Skulltulas, 2 wall Skulltulas; Dodongo's Cavern (84 groups, 4 `Obj_EnemyCounter_A_09`) 3 Beamos, 2 Keese, 4 Lizalfos; Jabu-Jabu (96 groups, 4 `Obj_EnemyCounter_A_05`) 4 Bari, 10 Biri, 13 Shabom, 6 Stingers, 3 Tailpasaran, 2 Octoroks, Big Octo, 3 tentacles; Bottom of the Well (49 groups, 1 enemy counter, 1 silver rupee counter) 8 Keese, 2 Gibdos, 2 big green Bubbles, Dead Hand and hands, 1 Skulltula, 1 Wallmaster.

## Version 3 addition (20 Nov, Q14: yes)
Enemies whose placement also carries `Presence` or `IsPhysicsStable` (nothing else extra) may now be swapped, under the same rule as the others: nothing links to the placement (hash seen exactly twice, no event, game-data or drop key). Only the `Gyaml` and parameters change; the extra keys are kept as they were. Per-map load-list rule unchanged. Result for seed "game": 98 swaps instead of 91, 7 added: Bottom of the Well +4 (6 to 10), Dodongo's Cavern +1, Ice Cavern +1, Spirit Temple (child) +1. Other maps unchanged. Far fewer than the 30 expected: most of those placements are kinds the pool never touches (kept, boss-room, counted or hard kinds). Child maps still get only sword-only kinds and keep their Deku Babas and counted enemies. Left out: none by map; every swapped map passes the game-file tests (class, fit, no referenced hash, counted enemies of the child dungeons unchanged). Not run in the game: load-test Bottom of the Well, Dodongo's Cavern (black screen history), Ice Cavern and Spirit Temple child first. Exe: `<playtest folder>\enemy-rando\ootrr.exe`. Tests: `difficulty` 215, `settings` 106 passed.

## How it was tested
- v2 unit tests (`swap.rs`): kinds from other maps are offered and a room of 12 Keese becomes at least 3 kinds; fire Keese never leave home, Guays stay outdoors, ceiling kinds indoors, a Poe never becomes a Keese; blasters stay away from bomb walls and the Water Temple gets no new walkers; parameters from another map lose their place keys; about four in five change (280 to 360 of 400) and with share 0 nothing changes; child Deku Babas stay and can still be a replacement.
- v1 unit tests, kept: the class is kept and the new kind's parameters are used; linked, counted, evented and drop-carrying enemies are never swapped; boss rooms and miniboss surroundings are left alone; child maps and counted rooms only get sword-only kinds (adult dungeons without a counter can get a Wallmaster); same seed same swaps, another seed differs; no boss, miniboss or never-placed kind in the pool.
- Game-file tests over all maps: only free vanilla enemies change, nothing else in any file; nothing referenced changed; class kept; Octoroks and Leevers never change; the new kind fits the map (`fits`), is not on the never-placed list, and fire Keese only where they live; Keese only replace Keese; parameters from another map carry no place key; child maps keep their Deku Babas; between 70 and 90 per cent of the candidates change and at least 12 kinds are used; child maps only sword-only kinds; room and rotation kept, height within 7; no event, game-data or drop key added; deterministic; another seed differs; with the setting off the difficulty stage writes nothing; the stage's report line matches. A second test lists the counted actors of the four child dungeons (above) and checks none changed.
- v2: `cargo test -p ootrr-core difficulty` 215 passed, `settings` 106 passed (includes the default settings string), release build copied to the exe above. Not run in the game at all.

## How to test it in the game
Turn on "Enemy randomizer" only (with `skip_intro`, `skip_cutscenes` all, `faster_chests`), seed "game". Load every map in `PLAN.md` (dungeons first) and look at the swapped spots listed there: does the new kind appear, stand on the ground (or fly) where expected, fight and die? Check that the counted rooms in the child dungeons still open. Then play the Fire Temple and Hyrule Field adult a while.

## Risks not settled from the files
- (v2, main risk) Kinds new to a map. The files show no load list other than the placements, and the horde's new kinds were seen in Hyrule Field, but no v2 swap has been seen in the game. Fire Temple, Dodongo's Cavern and Jabu-Jabu get the most kinds that never lived there. Load-test first (PLAN.md).
- (v2) Shabom (`Enemy_Shabom_AlwaysRespawn`), Biri and Bubbles now float outdoors (Hyrule Field, Graveyard, Desert Colossus) and Poes in fire and water dungeons: odd-looking but no rule says they cannot.
- (v2) Swapped enemies keep their hash, so they stay in the map's streaming groups (`FarDeleteGroups`, `SimultaneousGroups`) exactly as before; only unreferenced placements are touched, so no group membership changes.
- A donor's remaining `Dynamic` keys may mean something only where the donor stands (e.g. a Keese `IsCreateOnSurface` or Bubble ellipse radii): copied as they are; from another map the place keys listed above are removed.
- Whether the remake loads an actor pack per map or per room: a kind that lives only in one room of a dungeon may not be loaded in another room.
- Fliers that perched on walls or hung from ceilings keep that spot with a new kind (v2: a non-Keese spot never becomes a Keese, but a Keese spot can become a Bubble, Poe, Biri or Shabom).
- SoH's timed room in Jabu-Jabu is not identified in our data (Jabu is child, so only sword-only kinds land there; v2: 7 Biri become Shaboms, Bubbles and a Poe).
- Market adult (v2: 9, ReDeads become Torch Slugs, Deku Babas, Like Likes and a Baby Dodongo) is a story area; Graveyard adult is close to the Shadow Temple entrance.


## Moved from STATUS.md (20 Nov): randomizer v1 and v2

- **Hands-on session 4 (20 Nov late afternoon), seen by the user:** gates without the sword (absent), with the sword (stay when walked up to: the fix works), the guard kill drops the barrier; the Zora ambush works with the fix; the spawn combo; lose heart on death. Enemy randomizer v1: works but is too subtle ("boring"). v2 built (``b461862``, not played): 91 swaps in 16 maps, into 15 kinds; 78% of the 117 enemies the safety rules allow; the Fire Temple's 18 go to 9 kinds. The count is limited by the safety rules, not the pool.

  - **Enemy randomizer v2:** all 16 swapped maps load, Dodongo's Cavern included, and the new kinds are present at every planned spot checked. Movement and walls not judged.

## Dodongo's Cavern (21 Nov)
Seed handson9 swapped 8 enemies in Dodongo's Cavern, all Keese (Fire Keese and Keese turned into Bubbles, Biri, Poes and similar). The 12 Baby Dodongos, 5 Dodongos and 5 Armos were left, because Dodongo kinds are kept in that dungeon. Baby Dodongos are now allowed to be swapped out (never swapped in), which brings the seed to 15 swaps there. Dodongos and Armos still keep their kind.
