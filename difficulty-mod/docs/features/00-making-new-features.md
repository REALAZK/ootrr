# Making new features: what we learned

Notes that apply to every feature in this folder. Read this first; the per-feature files only add what is specific to them.

## Where things are

- **Source:** `<repo>` (a clone of `REALAZK/ootrr`). Feature worktrees are under `<workspace>\wt\`. The combined branch is `playtest/difficulty-all` in `worktree playtest-all`. Nothing is pushed.
- **Builds:** `<workspace>\builds\<number>_...\` with `ootrr.exe` and `CHANGELOG.md`. Quote the build number in feedback.
- **Game files (read only):** `<game romfs>`. Set `OOTRR_ROMFS` to that path to run the tests that read the game.

## How a difficulty feature is wired

1. **Setting:** declared in `fn difficulty()` in `crates/core/src/settings/list.rs`. Its id and number are constants in `crates/core/src/difficulty.rs`. Numbers 110 to 129 are used; take 130 next.
2. **Menu:** placed on the `difficulty` page in `crates/core/src/settings/menu.rs`.
3. **Read:** `difficulty::read` turns settings into `difficulty::Opts`.
4. **Apply:** `difficulty::stage` runs in the late stage of the build (`slice/edit.rs` calls `difficulty::worked`). Each step reads the previous step's output, so two features that edit the same file combine instead of overwriting.
5. **Report:** `Opts::notes()` adds a line to the build report for each active feature.

Every new setting must default to off, and the default settings string must not change.

## Where the game keeps things

| What | Where |
|---|---|
| Per-actor numbers (health, attack, movement, drop tables, lock-on) | `.bgyml` files inside `Pack/Actor/<Actor>.pack.zs` |
| Enemy AI | `.ainb` graphs inside actor packs and `Pack/AI.Global.Product.111.pack.zs` |
| Scripted scenes (game over, bottles, NPC talk) | `Event/EventFlow/<Name>.bfevfl.zs` |
| What is placed in each map | `Banc/<Map>/<Map>_{Static,Dynamic}.bcett.byml.zs` |
| Room logic (create on entry, delete on signal) | `Logic/*.ainb`, tied to map actors through the map's `AiGroups` |
| Global tuning | loose files under `GameParameter/`, `Life/`, `System/` |
| Text | `.msbt` files inside `Mals/*.sarc.zs`, one archive per language |
| Save flags and their names | `data/flags.json` in the repo |

## Facts that cost time to learn

- **True is case 0.** A true/false query in an event flow answers switch case 0 for true and case 1 for false. Integer compare answers 0 for less, 1 for equal, 2 for greater. Getting this backwards inverted the bottle feature in rc 1.
- **Flow booleans:** a true parameter is `0x8000_0001`, false is `0`.
- **Enum values are hashes.** Many `.bgyml` fields hold murmur3 hashes of a name (`murmur3_32(name, 0)` in `crates/core/src/murmur.rs`). Guessing the name and hashing it identifies them: "Bomb" is 3237216321.
- **The same file sits in many packs.** A shared parameter file is copied into every pack that uses it. Edit every copy identically.
- **Growing a file needs a size table entry.** If a file inside a pack grows, both the pack's entry and the inner file's entry in the resource size table must grow, or the game crashes while loading. The late stage does this for you.
- **A flow must end the way vanilla ends it.** Ending a bottle flow at its entry froze Link. Every vanilla path through those flows ends with the same "put away" action, so a refusal path has to run it too.
- **Retry reloads the last save.** Anything written to game data during the game over scene is lost unless `Sequence_RequestLastGameOverSave` is set first.
- **Global tables can override per-actor tables.** Drop tables looked complete, but `GameParameter/DropGlobalParam` adds drop chance when you are low on health or ammo. Always search for a global file with the same subject.
- **Some files the repo cannot parse.** BYML files with hash-keyed maps (node type 0x20) and AI graphs with "attachments" fail to parse. `difficulty/lottery.rs` shows how to patch such a BYML in place without parsing it fully.
- **Room clears count linked enemies only.** Rooms open on explicitly linked enemies, not on everything in the room, so an added enemy with no links should never block a door.
- **Added map actors have no save flag.** They are expected to come back on every reload.

## Research tools

- **Flows:** `ootrr bfevfl-dump <romfs> Event/EventFlow/<Name>.bfevfl.zs` prints every event with its parameters.
- **Loose AI graphs:** `ootrr ainb-dump <romfs> <path>`.
- **Node list:** the AI and event node names with their inputs are in `AI/NodeDefinition/Node.Product.111.aidefn.byml.zs`. Searching the names is the fastest way to learn what the engine can do.
- **Vanilla precedent:** before inventing a mechanism, find where the game already does the same thing and copy its shape. Almost every feature here works that way.
- **Object map:** https://objmap-oot.zeldamods.org shows every placed object and the real collision geometry.
- **Check a built mod:** `ootrr flow-loads <romfs> <mod romfs folder>` must report no missing loads after any flow edit.
- Never run `ootrr` with no arguments from a script: it starts the web page.

## Longshot's control bridge (found 18 Nov, not used yet)

Longshot has a local control socket that lets a script drive a running game and read its state. Your installed `longshot.exe` (v3.0.0-rc.3) contains it. Reference copies of its docs and client are in `<Longshot checkout>` (`docs\control.md`, `scripts\longshot_ctl.py`).

- **Turn on:** start the game with the environment variable `HOS_CONTROL` set to a pipe name. It is off otherwise, local only, and only for your Windows user.
- **Read:** current map, Link's position, age, health and max health in quarter hearts, rupees, the music playing, whether an event is running, and any named save flag.
- **Do:** press buttons and move sticks for a set number of frames, take screenshots, teleport to any map, switch age, list saves, quit.
- **Memory:** reading or writing game memory is a second, separate opt-in (`HOS_CONTROL_MEM`).

What this could verify without you playing: lose heart on death (read max health before and after), the battle music name the bottle check depends on, whether a map with added enemies loads at all, and screenshots of the Zora hole and boss rooms. It would run a second copy of the game with its own save folder.

## Testing in the game efficiently

- **Change one thing per build of the mod.** Turn on one or two settings, generate, build, play. With everything on, a crash tells you nothing.
- **Pick the shortest route to the thing.** The repo's `ootrr save-unlock` and `save-set` commands make test saves with chosen items, hearts and start position (see CONTRIBUTING.md). A save standing next to the enemy or NPC you need saves most of the time.
- **Report three things:** the build number, which settings were on, and exactly what you saw (crash on load, nothing happened, wrong thing happened).
- **A crash while loading a map** usually means a size table or malformed file problem in that map's files. **A frozen Link** usually means an event flow ended in a state the game does not expect.

## Status of every feature

Only enemy health scaling has been seen working in the game (rc 1). Bottles, drops and lose heart were seen failing in rc 1 and were fixed in rc 2, but the fixes have not been played. Everything else has never been played.

## Learned on 20 Nov

- **Flags:** we can read the game's own saved flags in AI and event graphs (`HoldGameDataGetBoolSimple`), for example `IsGet.Weapon_Sword_Kokiri` or `HavePlayedEvent.Dm_<dungeon>_BossIn`. We cannot add new saved flags, so any "remember this" feature has to reuse a game flag or be stateless.
- **Fight start:** a boss's fight-start marker is the game's own `HavePlayedEvent.<intro>` flag. A door-shut scene is a poor trigger, because a warp skips it.
- **Effects:** `OneShotXLinkSearchAndEmit` only finds keys under the running actor's own user in the shared `ELink2` file. An emitter in the `static` bundle shows on every map. The Deku Nut stun flash key is shared by 42 enemy kinds.
- **Cloning an actor:** a new actor name works like the game's `Dm_` actors: an ActorParam whose parent is the original, plus `RSDB/ActorInfo`, `GameActorInfo` and `Tag` rows and size table entries, as in `cow_clone` and `difficulty/volvagia.rs`.
- **Drops:** boss supply comes through named drop tables (`DropTable_Enemy_RangedWeapon_02`, `Drop_ResourceSupply_BeforeBoss*`, `DropTable_ResourceSupply_FinalBattle`). Spawned pots carry a fixed rupee drop, so test drop changes on pots the map places.
- **Testing:** the bridge's `enemies` list includes actors a map holds but has not created, so "listed" is not "present".

## Learned on 21 Nov
- **A new loose file needs a realistic size-table entry.** The Volvagia clone's own death table got an entry of its length plus 1 KB. The game needs 5 to 9 KB more for such tables, so it silently failed to load and the actor was never created. Size new entries from the game's own entries for the same file type.
- **Keep an actor's own death table when cloning.** Swapping a boss clone's DeathRef to a generic boss table dropped its death rules, and it could no longer be hurt. Copy its own table and change only the cells you need (destroy the body, respawn flags off).
- **The bridge's kill may skip the normal death path** (drops, death table). To test drops or death-driven logic, bring the enemy to 0 hp with damage.
