# Ocarina of Time Remake Randomizer (OOTRR)

A local rando for the Ocarina of Time remaake, in early dev. repo: https://github.com/realazk/ootrr

## How it works

Generate: settings and a seed it will create a seed file.

Build: a seed file plus your own extracted game folder produce a mod folder for your emulator.
(more detailed tutorial will come soon.)

## Install

Download the file for your system from the [releases page](https://github.com/realazk/ootrr/releases) and check it against `SHA256SUMS.txt`.

| System | File | What to do |
|---|---|---|
| Windows | `OOTRR-<version>-windows-x64-setup.exe` | Run it and follow the wizard. A portable `.zip` is there too. |
| macOS | `OOTRR-<version>-macos-universal.pkg` | Open it and follow the installer. It runs on Apple silicon and Intel. |
| Linux | `OOTRR-<version>-linux-amd64.deb` | Install it with your package manager. Or unpack `OOTRR-<version>-linux-x86_64.tar.gz` and run `./install.sh`. |

The installers are not signed. Windows SmartScreen and macOS Gatekeeper warn you the first time, and you can allow it. Start OOTRR from the menu or run `ootrr serve`, then open the link it prints. OOTRR contains no game files. You provide your own extracted copy.

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

## Making a release

Releases are built by GitHub Actions on Windows, macOS and Linux machines, so nothing needs to be installed locally. Set the version in `Cargo.toml`, commit, then push a tag that matches it:

```
git tag v0.1.0
git push origin v0.1.0
```

The workflow in `.github/workflows/release.yml` runs the tests, builds with the build paths stripped, audits each binary with `tools/audit.py`, makes the installers in `packaging/` and publishes a prerelease with `SHA256SUMS.txt`. To try the build without publishing, start the workflow by hand from the Actions tab. That builds everything and keeps the files as artifacts.
