# Contributing to OOTRR

Thanks for helping. OOTRR is a randomizer for the Ocarina of Time Switch 2 remake. It reads the game files you extracted yourself and writes a mod folder. This page is the short version of how we work. If something here is unclear, ask in the OOTRR Discord before you start.

## Ground rules

- **No game data in the repo.** No game files, keys, dumps, extracted textures, saves or anything derived from them. The shipped tool never decrypts anything and never touches keys. Identifiers (flag names, actor names, hashes) are fine.
- **License.** The project is GPL-3.0-or-later (releases up to 0.4.0 were MIT). Code you contribute is under the GPL. Credits for ported work stay in `NOTICE`.
- **One concern per pull request.** Small, focused changes are reviewed fast. Base your branch on `main`.
- **Maintainers decide releases.** Tags and releases are made by the owner. Do not push tags.
- **No new crate without asking first.** The only dependency is `ruzstd`.

## Build and test

```
cargo build --release
cargo test --workspace --release
```

Many tests read the extracted game. Set `OOTRR_ROMFS` to your extracted game folder (the one with Pack, Banc, Event and System) so they run. Without it they skip. A few tracker tests read `OOTRR_TEST_ROMFS` and `OOTRR_TEST_SAVES`. The release build must have no warnings.

Never run `ootrr` with no arguments. It starts the local web server and may open a browser. Use a subcommand. The usage list is in `crates/cli/src/main.rs`.

## Layout

- `crates/core`: the generator, logic engine, writers for the game's file formats, the slice code that turns a seed into edits
- `crates/cli`: the `ootrr` command line tool
- `crates/web`: the local web page, including the tracker
- `data/`: location catalogs, logic data (`data/logic`), flag names (`data/flags.json`), start presets
- `tools/audit.py`: the check that no game data slipped into the repo. Run it before a release.
- `packaging/`: installers

## Code style

1. No comments of any kind. No `//`, `///`, `//!` or `/* */`. Explanations go in the pull request or the docs.
2. snake_case for variables, parameters, functions and modules. CamelCase types and SCREAMING_SNAKE constants.
3. Short locals. Use `p` for a path, `f` for a file, `b` for bytes, `d` for data, `n` for a count, `k` and `v` for key and value, `i` and `j` for indexes, `out` for a result being built.
4. Short aliases on imports, such as `use std::path::{Path as P, PathBuf as PB}`.
5. Explicit type annotations on local bindings, such as `let n: usize = v.len()`. Skip them for closures and long iterator chains.
6. One concern per file. Small modules. No `utils` or `helpers` dumping ground.
7. Public names stay stable. Never rename vendored or generated code.
8. `unsafe` is forbidden.
9. Fallible functions return `Res<T>` from `error`. `unwrap` and `expect` are for tests only.
10. Tests sit in a `#[cfg(test)] mod tests` block at the bottom of the file they cover.
11. In prose, docs, commit messages and chat: no em-dashes and no semicolons.

## Settings

- Every new setting defaults to the old behaviour. With it off, seed files and spoilers must stay byte identical to `main`. Show that in your pull request with a few seeds per mode (default, chests, pots and crates and so on).
- Old settings strings must keep their meaning. The settings model has an optional baseline for this, see the commit that made story and NPC rewards on by default.
- Number your settings in a range the maintainer gives you, so two branches never collide.
- Write a plain tip for each setting and say when it is experimental or untested in the game.

## Editing the game's files

- Use the existing writers (`bfevfl` flows, `ainb` graphs, `byml`, SARC packs, the resource size table). Edit only what you mean to change and check that the file still rewrites byte for byte.
- **Size table rule.** If a file inside a pack grows, grow the resource size table entry of that inner file too, not only the pack's entry. If you do not, the game crashes while loading. One mod has exactly one size table.
- Derive new files from the player's own game at build time. Never commit one.

## Finding flags and state

The game keeps its state in named and unnamed save flags. To find the flag behind something:
1. Make two saves, one before and one after the thing happens.
2. `ootrr save-diff <a> <b> --flags data/flags.json` lists what changed.
3. `ootrr save-get <progress.sav> <name_or_hash>... --flags data/flags.json` reads values, and `save-set` and `save-unlock` make test saves.

Unnamed flags show as a hash such as `0xef10b80f` (the world clock in seconds).

## Testing in the game

- Hookshot is what we test on. Put one randomizer mod at a time in its `mods` folder (a leading underscore disables a mod), because each mod carries its own size table.
- Keep test saves under the wallet cap, and place the player with `ootrr save-unlock ... --start Map/StartPos --romfs <game>`. Pick open field spots, not the chicken room in Lon Lon Ranch.
- Say in your pull request what you tested in the game and what you did not. Experimental features stay marked experimental until someone has played them.

## Pull request checklist

- Builds with no warnings and the full tests pass (say whether you ran the game tests)
- No comments in the code, style rules followed
- New settings default off, parity shown, old settings strings still parse
- Anything that grows a file in a pack grows its size table entry
- No game data, no saves, no generated files committed
- A line on what you tested in the game
