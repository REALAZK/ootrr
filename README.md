# Ocarina of Time Remake Randomizer (OOTRR)

[![Join the OOTRR Discord](https://img.shields.io/badge/Discord-Join%20the%20OOTRR%20server-5865F2?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/jJ5V3NA2GM)

A local rando for the Ocarina of Time remaake, in early dev. repo: https://github.com/ootrando/ootrr

## Community

Questions, bug reports, seeds and playtesting happen on the [OOTRR Discord](https://discord.gg/jJ5V3NA2GM). Come say hi, share a seed or help test a build.

## How it works

Generate: settings and a seed it will create a seed file.

Build: a seed file plus your own extracted game folder (version 1.1.1) produce a mod folder. A version 1.0.0 folder works only together with the 1.1.1 update's folder, which the Configuration page asks for when it finds 1.0.0. The two are combined with links in the app's data folder, never copied, and the game folders are only read. The game can also be a `.nup` or `.nx` package (Package… next to Browse), or a package can be the update: only the files the build reads, about 370 MB (about 2.3 GB with music or sound effect shuffle), are unpacked into the app's data folder for each build and deleted afterwards. Lichii and Eden take `load/<title id>/<name>/romfs`. Hookshot and NX2navi take the game's `mods` folder, and the mod is `mods/<name>/romfs`. Choose the game on the Configuration page, or pass `--host hookshot` or `--host nx2navi` to `ootrr build`. A Hookshot mod also writes `coop-seed.txt` and keeps the larger resource-size entry from every other mod that is on. Name that mod `randomizer-<seed>` so Hookshot co-op can tell the seeds apart.

Longshot, the successor to Hookshot, also loads `mods/<name>/romfs` from the game folder (beside `bin\longshot.exe`). Pick Longshot on the Configuration page, or pass `--host longshot`. Like Hookshot, Longshot uses the size table from just one mod, the enabled mod whose folder name sorts last. So the build copies the larger entry from every other enabled mod, including mods nested as `mods/contents/<title id>/<name>`. If another mod with a size table would still sort after this one, the build report names it, so you can rename this mod (for example `zz-<name>`). Longshot has no co-op seed file, so none is written. The tracker's Longshot button finds the saves in `%USERPROFILE%\Games\Longshot\user\save` or `~/.local/share/longshot/save`.

Starting items can include every weapon, tool, upgrade tier, bottle, boot, tunic, shield, the Gerudo card, magic, songs and the starting hearts. A new game starts with them and one copy of each leaves the item pool. Zelda's Letter is refused because it shares the child trade slot. The full Business Scrubs setting is still not applied. The one time and menu scrub sales have their own settings under Story and NPC rewards.

New here? Follow the [full tutorial with screenshots](https://docs.google.com/document/d/19bxnx9MBVK9WNcX4B0JpQdvWd249YmhNvEfIP1zRMug/edit?usp=sharing), from installing to playing in Lichii.

## Your game folder

The mod build needs your own extracted copy of the game with the 1.1.1 update installed. Pick a folder of the game with the 1.1.1 update applied on the Configuration page. The folder must hold the Pack, Banc, Event and System folders. The page checks for the 1.1.1 files and tells you if they are missing. A folder from the base game without the update fails the build.

## What the settings change

Everything is optional and off unless noted. All of these are experimental and some have not been played in the game yet.

- Chest shuffle, dungeon item placement and progressive upgrades (Hookshot, Strength, bags, wallets, scales and more).
- Story and NPC rewards (on by default, experimental and untested in the game). The Spiritual Stones and Medallions stay with the bosses and Rauru unless Dungeon Rewards is set to End of dungeons (shuffled only among the bosses and Rauru) or Anywhere.
- Boss heart containers as chests, and songs shuffled among the song teachers or anywhere.
- Gossip stones that tell you where an important item lies, without the Mask of Truth.
- Start with one random Spiritual Stone or Medallion.
- Skip first pickup text (off by default), next to the cutscene options: no "You got X!" box the first time Link picks up a rupee, a recovery heart, a magic jar, arrows or Deku seeds. Chests, quest items and equipment keep theirs. It marks those pickups' get events as already played at new game.
- Skip area intros (off by default), next to the cutscene options: the short scene that shows off an area the first time you enter it is skipped, for the overworld areas and the dungeon entrances that only show the place. It also marks each area as visited so its map is available. Arrival scenes that start or advance a quest (the Deku Tree, Zora's Domain, the Lost Woods, the Desert Colossus, the Forest and Water Temples, Hyrule Field and Kokiri Forest, Lon Lon Ranch) keep playing.
- Faster chest and item pickup scenes. Big chests also open with the short small chest scene: the big chest flow hands over to a copy of the small chest flow that lifts the big lid, then gives the dungeon map, compass and boss key credit the big chest flow would. `ootrr qol-mod <romfs> <load> <name> big-chest-anim` writes a test mod with only that change.
- Chest size matches contents (experimental, untested in the game): plain wooden chests holding major items become big chests, the rest become small ones. Major means progression items, songs, stones and medallions, boss keys, small keys / key rings / silver rupees found outside their own dungeon, and shields or capacity upgrades you do not start with. Boss heart chests and summoned chests (Jabu Boomerang, Iron Boots, Ice Arrows, Silver Gauntlets, the Dampé race chests) follow the same rule; the two Dampé rupee chests only change size together. Chests that run another story scene, invisible chests and the treasure chest game keep their model.
- Skip the opening cutscenes (on by default): a new game goes straight to the first moment you control Link, and the first-visit scenes of eighteen areas that only show the place are marked as played. The arrival scenes that advance a quest step (the Deku Tree, Zora's Domain, the Lost Woods, the Desert Colossus) and the Water Temple's are left alone.
- Small keys in any dungeon or anywhere, counted for the dungeon each key belongs to (experimental).
- Kokiri Forest can start open (open, or open exit with the Deku Tree still closed). An open Kakariko gate needs the forest exit open: its story state takes Mido away while the exit wall would stay, so with a closed forest the gate setting falls back to Zelda's Letter, and a seed that starts with Zelda's Letter or her Lullaby next to an open gate is refused.
- The Kakariko gate can start open, which also skips the castle visit: a new game sets the castle, Door of Time and Fire Stone quests to the state after Impa's send off and the guard, marks Saria's house scenes, the owl and Talon's castle scene as played, opens the Threads of Time menu, and gives the items of Zelda's Letter and the Song from Impa. Malon's egg can no longer be collected and never holds an important item. These writes go into the opening event flow, next to the game's own first quest step. Saria's bridge scene normally marks Hyrule Field as first visited, which fixes its daylight and stops the clock until Impa's send off; with the send off skipped that write is turned into a clear, so the day and night cycle runs from the first visit. The field's clock also waits for Zelda's courtyard talk, so its markers are written too, and Navi's monster lesson from the Deku Tree is marked as seen, so the D-pad calls Navi and gives enemy information from the start.
- The Door of Time can start open: the Temple of Time logic modules are edited so the door is always open and the Master Sword can be drawn without the Door of Time quest, a new game sets the door's opened flag, and the logic places items accordingly. The quest is left alone so Zelda's escape and the Song of Time stay checks, and drawing the sword before the quest reached the door no longer completes the quest, since a completed quest would hide those two checks. The adult pedestal also works before the Forest Sage is awakened, so you can return to childhood at any time; on the way back the child is given the Kokiri Sword instead of the Master Sword, and Sheik stands a little off the centre line so the pedestal prompt is easy to reach. `ootrr door-open` writes a test mod with only those edits.
- Zora's Fountain can start open: King Zora starts moved aside, so the fountain needs no Ruto's Letter, and Ruto's Letter stays in the item pool. Any event that would move the Zora quest back to before King Zora moved (the diving game, getting or reading the letter, the arrival scenes) leaves the quest where it is.
- Return with Navi (experimental, untested in the game): calling Navi with the D-pad asks whether to head back, to the dungeon entrance inside a dungeon, and with "Dungeons and overworld" also to Kokiri Forest as a child or the Temple of Time as an adult. The D-pad calls Navi from the start of a new game. Her room tutorial hints are replaced by the question; enemy hints while targeting stay. Boss rooms, miniboss rooms, the boss rush and the final battle never offer it, since leaving a fight that way left the save button disabled.
- Bunny Hood effect (experimental): while child Link wears the Bunny Hood he runs 1.5 times as fast, and with "Faster running and longer jumps" he also jumps further. Done in the player's AI graphs, with no code patch.
- Character hints (experimental, untested in the game): the Temple of Time altar, Dampé's diary, the House of Skulltula, the Zora's River frogs, the Great Fairies, Sheik, Malon, Mido and Saria tell you where an item lies, in their own words. Only the text changes.
- Skip Epona's race (on by default, experimental, untested in the game): learning Epona's Song counts as having won Epona from Ingo, so an adult with the song can call Epona without the ranch race. The logic follows it: with it on the song is Epona, with it off the race is needed. Child Malon's song lesson still works.
- Skip the mask trading quest: the Happy Mask Shop quest starts complete, so once the shop opens every mask can be borrowed right away, including the Mask of Truth.
- Magic beans planted: every soft soil patch starts planted, so the adult bean platforms are there from the start. The Magic Bean leaves the item pool, and the bean salesman in Zora's River has one sale left, for 60 rupees, which holds a random item when NPC rewards are shuffled.
- Gerudo Valley bridge repaired: the broken bridge is fixed from the start, so adult Link can reach Gerudo Fortress without Epona or the Longshot. The carpenters are not rescued, so the guards, the rescue and the Gerudo Membership Card stay as they are.
- Difficulty (experimental, all off by default), on its own page of the settings menu: enemy health and damage from 0.5 to 5 times, enemy speed from 0.75 to 2 times (most enemies ignore it so far), no heart drops, limited ammo, no gossip stone fairies, lose a heart container per game over down to three, no bottles during combat, no Z-targeting of enemies, no duplicate bottle contents, more enemies (up to ten times, with a few special encounters), enemies that come back sooner, extra enemies in boss fights, enemies that heal (with a green glow), fairies that heal enemies too, enemies that call for backup, enemies that block the way, Cuccos that get angry at the first hit, fireball storms in parts of Hyrule Field and the castle garden, Bombchu Bowling grass that cannot be broken (an idea by Sabriel), a Volvagia that roams adult Hyrule Field, mega Like Likes that eat your shields, random bomb and Bombchu fuses, an enemy randomizer, random enemy sizes, shorter timers on timed switches and torches, and damage taken up to one hit kills. It is all done by editing game data, with no code patch. Each setting's note in the menu says what it does and whether it has been tried in the game; `difficulty-mod/docs/CHANGELOG.md` and `difficulty-mod/docs/TODO.md` say what was played and what is known not to work.
- Spawn enemies (off by default), on the Difficulty page: hold both sticks pressed in and press D-pad Down to spawn a random enemy in front of Link, or D-pad Up for the same kind again. Enemies the area has not loaded will not appear.
- Bombchus in logic (experimental, off by default): the seed logic counts Bombchus as explosives without a Bomb Bag, as OoTR does. Only the logic is changed. In our test the game only let Bombchus work once a Bomb Bag and its ammo maximum were set, so a seed with this ticked may expect Bombchus where the game will not let you use them.
- A plandomizer on the Generate page, or --plando Location=Item on the command line, to fix chosen items at chosen locations. The seed is only made if it can still be beaten.

## Troubleshooting

- The build stops with "the game folder has no ..." Your folder is missing a file of version 1.1.1, so it is an older version of the game. Use a folder with the update applied.
- The Configuration page shows the 1.1.1 check with a cross: same cause.
- The emulator ignores the mod: copy the whole mod folder into the emulator's load folder for the game and remove other mods that change the same files.

## Install

Download the file for your system from the [releases page](https://github.com/ootrando/ootrr/releases) and check it against `SHA256SUMS.txt`.

| System | File | What to do |
|---|---|---|
| Windows | `OOTRR-<version>-windows-x64-setup.exe` | Run it and follow the wizard. Or take `OOTRR-<version>-windows-x64-portable.zip`: unpack it anywhere and double-click `ootrr.exe`. It installs nothing and keeps its data in its own folder. |
| macOS | `OOTRR-<version>-macos-universal.pkg` | Open it and follow the installer. It runs on Apple silicon and Intel. |
| Linux | `OOTRR-<version>-linux-amd64.deb` | Install it with your package manager. Or unpack `OOTRR-<version>-linux-x86_64.tar.gz` and run `./install.sh`. |

The files are not code-signed yet. Windows SmartScreen and macOS Gatekeeper warn you the first time, and you can allow it. Each file is built by GitHub Actions from the tagged source and has a build attestation: `gh attestation verify <file> --repo ootrando/ootrr` shows which workflow and commit made it. Start OOTRR from the menu or run `ootrr serve`, then open the link it prints. OOTRR contains no game files. You provide your own extracted copy.

## Commands

Running `ootrr` with no arguments (for example by double clicking the exe) starts the web page, the same as `ootrr serve`. Run `ootrr help` for the full list. The common ones:

```
ootrr serve [--port N]
ootrr generate <preset_or_settings_string> <seed> <out.ootrr> [--spoiler <file>] [--plando Location=Item]...
ootrr build <seed.ootrr> <game> <load_dir> <mod_name> [--update <folder|.nup|.nx>] [--include-update] [--host eden|hookshot|longshot|nx2navi]
ootrr swap-entrance <romfs_root> <load_dir> <mod_name> <entrance_a> <entrance_b>
ootrr set-drop <romfs_root> <load_dir> <mod_name> <actor_hash> <item_name> [no-event]
ootrr check-byml <romfs_root> <subfolder> <suffix>
```

## Build and test

```
cargo test
powershell -File tools\build.ps1
python -I tools\audit.py target\release\ootrr.exe
```

## Making a release

Releases are built by GitHub Actions on Windows, macOS and Linux machines, so nothing needs to be installed locally. Set the version in `Cargo.toml`, commit, then push a tag that matches it:

```
git tag v0.1.0
git push origin v0.1.0
```

The workflow in `.github/workflows/release.yml` runs the tests, builds with the build paths stripped, audits each binary with `tools/audit.py`, makes the installers in `packaging/` and publishes a release with `SHA256SUMS.txt`. To try the build without publishing, start the workflow by hand from the Actions tab. That builds everything and keeps the files as artifacts.

## License

OOTRR is free software under the GNU General Public License, version 3 or any later version (see `LICENSE`). Releases up to and including 0.4.0 were published under the MIT License, kept in `LICENSE-MIT`. Credits for the ported OoT Randomizer logic and the embedded fonts are in `NOTICE`. No game files are included.
