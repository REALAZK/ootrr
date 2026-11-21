# Random enemy sizes (optionally with health scaled to size)

Settings `enemy_sizes` (number 145: off / mild / wild) and `enemy_sizes_health` (number 146, on/off). Both default off. Code: `randomizer/core/src/difficulty/sizes.rs`. Branch `feature/enemy-sizes` (worktree `wt\sizes`).

## Request
Next item in your order: "random enemy sizes (optionally with health scaled to size)". Off / mild / wild; mild 0.75x to 1.5x, wild 0.5x to 2.5x; a separate on/off "health follows size". Roll per placed enemy from the seed. Leave bosses, minibosses that rooms count and gate guards alone, keep big rolls out of narrow spots, stack sensibly with enemy health, the enemy randomizer and more enemies.

## How it works
- **Size:** each eligible placed enemy gets a `Scale` (the same number on all three axes) in its map file (`Banc/*/*.bcett.byml.zs`). The mega Like Like and the Beamos already showed the game honours a placement `Scale`.
- **Roll:** from `seed|enemy-sizes|<placement hash>`, so a seed is repeatable and one enemy's size does not depend on another's. The roll is log-uniform between the bounds, so small sizes are commoner than the biggest (wild averages about 1.2x, not 1.5x). A roll within 4 percent of 1.0 is left alone.
- **Runs last:** the step is the final one in `difficulty::stage`, after the enemy randomizer and more enemies, so swapped and added enemies are sized too. It reads the files earlier steps wrote.
- **Who is eligible** (`eligible`): an `Enemy_*` placement whose hash no `Links` entry anywhere mentions and that has no `Links`, `Rails` or `Scale` of its own; no `Event`, `RecordingDeath`, `FenceID`, `IsCleared` or (other than attention) `GameData` key in its Dynamic block; not near a boss or miniboss (`swap::guard`), not in a room that holds an enemy counter or any linked enemy, not within 30 units of a counter outdoors. This is the same test the enemy randomizer uses, except that it looks only at `Links`: the AI group and delete-group lists every field enemy sits in do not count (counting them left only 200 enemies; now about 360).
- **Kinds never resized** (`KEEP_KINDS`): bosses, minibosses, tentacles, Tricky doors, Gohma eggs, floor tiles, Dead Hand, Beamos, Anubis, Armos, Wall Skulltulas, every Leever (the gate guards), the Poe sisters, Flat and Sharp Poes and Big Poes. Maps `SageRoom` and `FinalBattleField` and `Demo_` maps are skipped.
- **Caps for tight spots:** there is no navmesh or ceiling check in the data we can read, so caps by place: 1.5x indoors (any placement with a dungeon room index), 1.5x in narrow-lane maps (Lost Woods, Zora's River, Death Mountain Trail and Crater, Gerudo Valley, Graveyard, Kakariko, Market, Sacred Forest Meadow, Kokiri Forest), 2.0x outdoors for the already big kinds (`horde::big`) and 1.25x for them indoors. A capped roll is drawn again inside the lower range, so sizes do not pile up at the cap.
- **Health follows size:** health is a per-actor number, not a per-placement one, so each size step is its own copy of the enemy. With the second setting on, rolls snap to steps (mild 0.75, 1.0, 1.25, 1.5; wild 0.5, 0.75, 1.0, 1.5, 2.0, 2.5) and the placement is renamed `Enemy_<Kind>_Size<percent>` (for example `Enemy_ReDead_Size150`). The copy is the kind's own pack (taken after the enemy health, damage, regen and other pack edits) plus three new files: an ActorParam whose `$parent` is the original and that points `LifeRef` at a new life component, that component, and a life file with `MaxLife` = round(old x size), at least 1, at most 99. It gets ActorInfo, GameActorInfo and Tag rows copied from the original (so effects, sounds and model stay the original's), a pack size and loose-file size entries from the kind's own peers, as for the mega Like Like and Leever gate clones (`write` in `slice/edit.rs`, after those). Only the (kind, step) pairs actually placed are made: the build scans the written map files for names ending `_Size<digits>`.
- **Kinds without a copy:** a kind whose life number lives in a shared or scripted file (value 100 or more, or no `LifeRef` in its own pack) is not resized at all in health mode, so the promise "health follows size" always holds.
- **Stacking:** with `enemy_health` the copies are made from the already scaled pack, so the two multiply (capped at 99 as that setting already is). With the enemy randomizer or more enemies the new and swapped enemies are sized like any other. Without the health setting the sizes need no extra files and any kind is resized.
- **Cost:** about 70 copies (wild plus health, this seed) add about 27 MB to the mod; the size-only build adds nothing.

## UI text
- Random enemy sizes (Off / Mild / Wild): "Every ordinary enemy gets its own size, the same for the same seed. Bosses, minibosses and the enemies a door or puzzle counts keep theirs, and indoors the biggest sizes are capped so nothing is wedged into a wall. Experimental, untested by hand."
- Health follows size: "With random enemy sizes on, an enemy's health is multiplied by its size, so a double-size Stalfos has twice the health and a half-size one half, never less than one. Sizes then come in a few steps. It stacks with the enemy health setting."
- The build report says how many enemies in how many maps were resized, and how many sized copies were added.

## Choices I made (change them if you disagree)
- Log-uniform rolls (small sizes commoner), per-placement seeding, 4 percent dead zone.
- The caps above, including 1.5x as the indoor maximum even in wild.
- Octoroks, Tektites, Peahats and fliers are resized like the rest; Peahat larvae ("independent") too.
- Health mode snaps to steps rather than copying every size.
- Spawned enemies that are not placed in a map (night Stalchildren in the field, enemies the spawn combo or boss adds create) are not resized: they have no placement.
- The first and last roll of a kind in health mode can be the same number: a half-size Keese (1 hp) keeps 1 hp.

## How it was tested
- **Unit tests** (`cargo test -p ootrr-core sizes`, no game files needed): ranges and mean for mild and wild; same seed same sizes and another seed other sizes; bosses, scripted, tied, linked, wall and counted-room enemies untouched; caps indoors, in lanes and for big kinds; health mode renames only known kinds and snaps to steps; life rounding (18 x 0.5 = 9, 1 x 0.5 = 1, 90 x 2.5 = 99); class names round trip; the settings read (`difficulty::tests::the_size_settings_are_read_and_default_off`).
- **Game-file tests** (`OOTRR_ROMFS` set): every map gets mixed sizes for mild, wild and wild+health (about 340 / 360 / 250 enemies in 30 maps, 33 kinds), nothing in `KEEP_KINDS` changes, bounds hold, the same seed gives the same files; every kind used in health mode builds its copies (3 new files, scaled `MaxLife`, parent and life references correct, size table entries found for every new file). `cargo test -p ootrr-core difficulty` (271 passed) and `settings` (112 passed).
- **Default settings string and default seed unchanged:** `ootrr generate` of an empty preset gives byte-identical files before and after (checked against `playtest/next`).
- **In the game** (Longshot source build, windowed, sound off, bridge scripted; `<playtest folder>\sizes`):
  - *Size only, wild:* Kokiri Forest, Hyrule Field (child and adult), the Deku Tree and Dodongo's Cavern all load and play with no black screen. In Kokiri Forest the withered Deku Babas on the Deku Tree path are visibly different sizes (shot `shots\kokiri_c.png`: a big and a small one side by side); the 0.53x and 1.11x ones died to real Kokiri Sword hits (hp 1 to 0, gone from the list).
  - *Health follows size, wild:* the same maps load with the copies in them. Ruined Market ReDeads: base 12/12, `_Size75` 9/9, `_Size150` 18/18; Skulltulas 1, 2 and 3; Baby Dodongos and Keese 1 or 2. A 0.75x ReDead (9 hp) died to 5 real Master Sword hits (2 hp each); a 1.5x one (18 hp) went 18 to 2 over 9 hits, and Link died to the ReDead crowd before the last hit (ReDeads grab and drain, with or without this setting). Shot `shots\market_redeads.png` shows a big and a small ReDead.
  - *Frame rate:* the overlay and `status` read 59 to 61 fps in every map with the setting on (Field, Deku Tree, Dodongo's, Market) while another game copy was running; there was no baseline run with the setting off and no timed measurement, so "unchanged" is only "the same cap as vanilla".
  - *Walls:* nothing was seen stuck or clipped in the maps visited. Only the places named above were looked at, no systematic walk; the caps are a guess, not a collision check.
  - *Not tested:* mild; Kokiri Forest adult; the other 25 maps; interactions with `enemy_health`, the randomizer and more enemies in the game (only unit and game-file tests); a hand session.

## How to test it by hand
1. Preset: `"enemy_sizes": "wild"` (add `"enemy_sizes_health": true` for health), plus the usual time savers (`skip_intro`, `skip_cutscenes` all, `faster_chests`, `open_forest`). `ootrr generate`, `ootrr build ... --host longshot`, restart the game.
2. Child, Kokiri Forest: walk to the Deku Tree path (east of Mido's house) and look at the withered Deku Babas: one is tall, one short.
3. Adult, ruined Market (Hyrule Castle Town after the Door of Time): ReDeads of three sizes; with health on a big one takes about twice the hits of a small one. Dodongo's Cavern: Baby Dodongos at 0.5x to 1.5x near the first big room.
4. Hyrule Field: Poes, Tektites and Guays of different sizes. Look for anything wedged in a wall or floating.
5. Finding where an enemy was sized: `SIZE_MAPS=Market_Adult SIZE_HEALTH=1 cargo test -p ootrr-core sizes::where_tests -- --ignored --nocapture` prints `SIZED map kind percent x y z` lines for the maps you name (the seed is `sizes-seed`).
6. Try the same seed twice: sizes must repeat. Turn the settings off: nothing changes.

## Notes
- The mild range also has a 1.25 step in health mode so mild has three copies per kind, wild five.
- Open question for you: should wild be allowed above 1.5x indoors in big rooms (Dodongo's, Forest Temple main room)? Default: no.

## Not working yet (21 Nov, hands-on)
Settings 145 (Random enemy sizes) and 146 (Health follows size) are labelled "(not working yet)" in the settings page. The game ignores the placement Scale for these enemies, so they keep their usual size. The code is kept. Needs a different way to scale (a pack value rather than placement Scale).

Hands-on 9 (21 Nov): the size gallery in child Hyrule Field (Like Likes, Stalfos) showed every size row identical, so the settings really do nothing in the game. Status: NOT WORKING. They stay in rc 18 and on `playtest/next` labelled "not working yet"; the real fix (a pack value or model scale instead of placement Scale; find what made the mega Like Like and the temple Beamos bigger) is open.
