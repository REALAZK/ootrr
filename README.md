# Ocarina of Time Remake Randomizer (OOTRR)

A local rando for the Ocarina of Time remaake, in early dev. repo: https://github.com/realazk/ootrr

## How it works

Generate: settings and a seed it will create a seed file.

Build: a seed file plus your own extracted game folder produce a mod folder for your emulator.
(more detailed tutorial will come soon.)

## Commands

Run `ootrr` with no arguments for the full list. The common ones:

```
ootrr serve [--port N]
ootrr generate <preset_or_settings_string> <seed> <out.ootrr> [--spoiler <file>]
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
