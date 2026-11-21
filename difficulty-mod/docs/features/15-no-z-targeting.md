# No Z-targeting

Setting `no_z_targeting` (number 124), checkbox. Code: `crates/core/src/difficulty/target.rs`. Built by a sub-agent on `feature/no-z-targeting` (`eb28394`).

## Request
"no z-targetting"

## Interpretation
You cannot lock on to enemies. Talking to people, reading signs and other interactions keep working. Aim assist for the bow, slingshot and hookshot is left alone.

## Investigation
- Every lock-on target is an "attention client" file inside the actor's pack, with an attention type and an action type. Both are hashed names.
- Enemies use type "LockOn" with action "None". People and signs use "Talk" or "Check".
- The game itself ships a client named "LockOn" whose type is "LookOnly" (a Deku Scrub in the forest stage). That is the precedent: Link looks at it but cannot lock on.
- Aim assist is a separate client file with its own type.

## Implementation
- Enemy client files with type "LockOn" and action "None" have their type changed to "LookOnly". Seventeen root files cover everything, because per-enemy files inherit from them.
- "Enemy" is decided by file name: starts with `EnemyCommon_`, `Enemy_`, `DungeonBoss` or `Ganon_`.
- The value is the same size, so no file grows.

## How it was tested
- 19 difficulty tests. The game-file test sweeps every actor pack and asserts only enemy lock-on clients change, every other client is untouched, and no player or NPC pack is written.
- A real build: the shared enemy client is LookOnly, aim assist and the training rock client are unchanged.

## In the game
Never played. To test:
- Turn this on alone. Stand near a Deku Baba or Keese and press the lock-on button.
- Pass: no lock-on, Link may turn his head toward it. Talking to an NPC and reading a sign still work.
- Check the Kokiri Forest lock-on training rocks still work.
- Then try a fight that leans on lock-on: Big Octo's back, Morpha's nucleus, Dark Link, Ganondorf's energy balls.

## Notes
- That LookOnly clients cannot be locked on to is inferred from the name and the one vanilla example.
- Some bosses request a scripted camera focus on themselves (Dead Hand, Ganon, Twinrova); that is separate and may remain.
- A few enemies ask "am I locked on?" in their AI and will always get "no", which may change how they behave.
- Fights that could become much harder or stall are listed above. None is confirmed beatable without lock-on.


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 15):** No Z-targeting
- **State:** Done
- **What was seen in the game:** Confirmed by the user at the controls (18 Nov): enemies cannot be locked on to
- **Next step:** None
