# Ocarina of Time Remake Randomizer (OOTRR)

[![Join the OOTRR Discord](https://img.shields.io/badge/Discord-Join%20the%20OOTRR%20server-5865F2?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/jJ5V3NA2GM)

A local rando for the Ocarina of Time remaake, in early dev. repo: https://github.com/realazk/ootrr

## Community

Questions, bug reports, seeds and playtesting happen on the [OOTRR Discord](https://discord.gg/jJ5V3NA2GM). Come say hi, share a seed or help test a build.

## How it works

Generate: settings and a seed it will create a seed file.

Build: a seed file plus your own extracted game folder produce a mod folder. Lichii and Eden take `load/<title id>/<name>/romfs`. Hookshot and NX2navi take the game's `mods` folder, and the mod is `mods/<name>/romfs`. Choose the game on the Setup page, or pass `--host hookshot` or `--host nx2navi` to `ootrr build`. A Hookshot mod also writes `coop-seed.txt` and keeps the larger resource-size entry from every other mod that is on. Name that mod `randomizer-<seed>` so Hookshot co-op can tell the seeds apart.

Starting items can include every weapon, tool, upgrade tier, bottle, boot, tunic, shield, the Gerudo card, magic, songs and the starting hearts. A new game starts with them and one copy of each leaves the item pool. Zelda's Letter is refused because it shares the child trade slot. The full Business Scrubs setting is still not applied. The one time and menu scrub sales have their own settings under Story and NPC rewards.

New here? Follow the [full tutorial with screenshots](https://docs.google.com/document/d/19bxnx9MBVK9WNcX4B0JpQdvWd249YmhNvEfIP1zRMug/edit?usp=sharing), from installing to playing in Lichii.

## Your game folder

The mod build needs your own extracted copy of the game with the 1.1.1 update installed. Pick a folder of the game with the 1.1.1 update applied in the Setup screen. The folder must hold the Pack, Banc, Event and System folders. Setup checks for the 1.1.1 files and tells you if they are missing. A folder from the base game without the update fails the build.

## What the settings change

Everything is optional and off unless noted. All of these are experimental and some have not been played in the game yet.

- Chest shuffle, dungeon item placement and progressive upgrades (Hookshot, Strength, bags, wallets, scales and more).
- Story and NPC rewards (on by default, experimental and untested in the game). The Spiritual Stones and Medallions stay with the bosses and Rauru unless Stones and Medallions is also on.
- Boss heart containers as chests, and songs shuffled among the song teachers or anywhere.
- Gossip stones that tell you where an important item lies, without the Mask of Truth.
- Start with one random Spiritual Stone or Medallion.
- Faster chest and item pickup scenes.
- Skip the opening cutscenes (on by default): a new game goes straight to the first moment you control Link, and the first-visit scenes of eighteen areas that only show the place are marked as played. The arrival scenes that advance a quest step (the Deku Tree, Zora's Domain, the Lost Woods, the Desert Colossus) and the Water Temple's are left alone.
- Small keys in any dungeon or anywhere, counted for the dungeon each key belongs to (experimental).
- Kokiri Forest can start open (open by default, or open exit with the Deku Tree still closed). An open Kakariko gate needs the forest exit open: its story state takes Mido away while the exit wall would stay, so with a closed forest the gate setting falls back to Zelda's Letter, and a seed that starts with Zelda's Letter or her Lullaby next to an open gate is refused.
- The Kakariko gate can start open (on by default), which also skips the castle visit: a new game sets the castle, Door of Time and Fire Stone quests to the state after Impa's send off and the guard, marks Saria's house scenes, the owl and Talon's castle scene as played, opens the Threads of Time menu, and gives the items of Zelda's Letter and the Song from Impa. Malon's egg can no longer be collected and never holds an important item. These writes go into the opening event flow, next to the game's own first quest step. Saria's bridge scene normally marks Hyrule Field as first visited, which fixes its daylight and stops the clock until Impa's send off; with the send off skipped that write is turned into a clear, so the day and night cycle runs from the first visit. The field's clock also waits for Zelda's courtyard talk, so its markers are written too, and Navi's monster lesson from the Deku Tree is marked as seen, so the D-pad calls Navi and gives enemy information from the start.
- The Door of Time can start open (on by default): the Temple of Time logic modules are edited so the door is always open and the Master Sword can be drawn without the Door of Time quest, a new game sets the door's opened flag, and the logic places items accordingly. The quest is left alone so Zelda's escape and the Song of Time stay checks, and drawing the sword before the quest reached the door no longer completes the quest, since a completed quest would hide those two checks. The adult pedestal also works before the Forest Sage is awakened, so you can return to childhood at any time; on the way back the child is given the Kokiri Sword instead of the Master Sword, and Sheik stands a little off the centre line so the pedestal prompt is easy to reach. `ootrr door-open` writes a test mod with only those edits.
- Zora's Fountain can start open: a new game sets the Water Stone quest to the step after Ruto's Letter was shown (King Zora stands aside) and marks his moving scene as played, which removes the wall in front of the passage. Ruto's Letter can still be found at the lake and stays a bottle. "Open as adult" only tells the logic; the adult domain has no blocker in the game data.
- Bombchus in logic (experimental, off by default): the seed logic counts Bombchus as explosives without a Bomb Bag, as OoTR does. Only the logic is changed. In our test the game only let Bombchus work once a Bomb Bag and its ammo maximum were set, so a seed with this ticked may expect Bombchus where the game will not let you use them.
- A plandomizer on the Generate page, or --plando Location=Item on the command line, to fix chosen items at chosen locations. The seed is only made if it can still be beaten.

## Troubleshooting

- The build stops with "the game folder has no ..." Your folder is missing a file of version 1.1.1, so it is an older version of the game. Use a folder with the update applied.
- Setup shows the 1.1.1 check with a cross: same cause.
- The emulator ignores the mod: copy the whole mod folder into the emulator's load folder for the game and remove other mods that change the same files.

## Install

Download the file for your system from the [releases page](https://github.com/realazk/ootrr/releases) and check it against `SHA256SUMS.txt`.

| System | File | What to do |
|---|---|---|
| Windows | `OOTRR-<version>-windows-x64-setup.exe` | Run it and follow the wizard. A portable `.zip` is there too. |
| macOS | `OOTRR-<version>-macos-universal.pkg` | Open it and follow the installer. It runs on Apple silicon and Intel. |
| Linux | `OOTRR-<version>-linux-amd64.deb` | Install it with your package manager. Or unpack `OOTRR-<version>-linux-x86_64.tar.gz` and run `./install.sh`. |

The installers are not signed. Windows SmartScreen and macOS Gatekeeper warn you the first time, and you can allow it. Start OOTRR from the menu or run `ootrr serve`, then open the link it prints. OOTRR contains no game files. You provide your own extracted copy.

## Commands

Running `ootrr` with no arguments (for example by double clicking the exe) starts the web page, the same as `ootrr serve`. Run `ootrr help` for the full list. The common ones:

```
ootrr serve [--port N]
ootrr generate <preset_or_settings_string> <seed> <out.ootrr> [--spoiler <file>] [--plando Location=Item]...
ootrr build <seed.ootrr> <romfs_root> <load_dir> <mod_name>
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

The workflow in `.github/workflows/release.yml` runs the tests, builds with the build paths stripped, audits each binary with `tools/audit.py`, makes the installers in `packaging/` and publishes a prerelease with `SHA256SUMS.txt`. To try the build without publishing, start the workflow by hand from the Actions tab. That builds everything and keeps the files as artifacts.

## License

OOTRR is free software under the GNU General Public License, version 3 or any later version (see `LICENSE`). Releases up to and including 0.4.0 were published under the MIT License, kept in `LICENSE-MIT`. Credits for the ported OoT Randomizer logic and the embedded fonts are in `NOTICE`. No game files are included.
