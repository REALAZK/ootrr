# No dupe items

Setting `no_dupe_items` (number 122), checkbox. Code: `crates/core/src/difficulty/dupes.rs`. Built by a sub-agent on `feature/no-dupe-items` (`6021c95`).

## Request
"no dupe items (won't carry you get a 2nd red potion, 2nd fairy, etc)"

## Interpretation
You cannot hold two bottles with the same contents. Empty bottles are unaffected, and different contents in different bottles are fine.

## Investigation
- The game has a flow query "is this bottle content already in the pouch" (`EventQueryIsExistBottleTypeInPouch`). The game over flow uses it to look for a fairy.
- Bottle content codes were read from the flows: 0 Red Potion, 2 Blue Potion, 3 Blue Fire, 4 Fish, 5 Letter, 6 Big Poe, 7 Poe, 8 Bugs, 9 Fairy, 10 Milk. Green Potion (1) and half Milk (11) are inferred.
- Three ways to fill a bottle were found: catching with it, being handed something in an event, and buying from a shop shelf.
- This agent also noticed that true is case 0 in flow queries, which exposed the inverted bottle bug in rc 1.

## Implementation
The "already held?" check is added in five flows:
- **Catching:** the insert-bottle flow only runs if you do not already hold that content. Otherwise a refusal runs that opens and closes the way the vanilla catch does: switch to the player camera, start the player event, the `EvtTalkNo` head shake, then put the bottle away (`EventPlayerDrawnShieldAfterUseBottle`, copied from the flow with its own parameters). The vanilla flow starts with the camera switch and the event start and ends with that put-away, so Link is left in a normal state. The refusal skips the selected-item stop/start pair, the camera save point and the get-item effect, because the vanilla path only runs those after the guard point. The flow never hides or removes the caught thing (no delete or offstage action in it), so a refused catch does not consume it in this flow; whether the swing code removes it earlier is not visible in the files.
- **Hand-overs:** the five bottle-filling entries of the event item-get flow get the same guard. These entries are sub-flows called from an NPC's event, which already started it. Vanilla ends each of them by playing `EvtTalk` to put Link back in the talk pose (and warping Navi home and an action guide, which only follow the demo the refusal skips). The refusal plays the `EvtTalkNo` head shake, then the same `EvtTalk`, then ends, which returns to the caller exactly as a normal completion does. Nothing is given and no rupees are taken, since payment comes after the guard.
- **Shop shelves:** the "do you have an empty bottle" check now also fails if you hold what you selected, so the shop refuses as if you had no bottle.
- **Talon's Milk and the Kakariko Blue Potion:** the same wrap, because Talon takes payment before the hand-over.

## How it was tested
- Unit tests for each refusal: the chain of actions (catch: camera, start, shake, put away; hand-over: shake, talk) and that it ends; the game-file test counts the added events in each real flow (5, 15, 22, 3 and 1).
- A real build; `flow-loads` clean.

## In the game
Never played. To test, with two empty bottles:
- Catch a fairy, then try to catch a second. Pass: the second is refused and you can move.
- Buy a Red Potion, then try to buy another. Pass: refused, no rupees taken.
- Check that a different content still goes into the second bottle.
- Report whether a refused catch still uses up the fairy or fish. The files do not say.
- Check Link moves normally after a refused catch and after a refused hand-over, and that the head shake plays.

## Notes
- Not blocked: bottle contents that arrive through the normal item-get path, such as a shuffled item in a chest.
- Milk and half Milk count as different.
- Deku Scrub potion sellers, cow Milk and the Market potion gift: the bottle is not filled and nothing is charged, but the NPC's lines will not match.
- Ending the catch flow at its entry may leave Link frozen, as the bottle-use flows did in rc 1; both guarded flows now end with a proper refusal as described above, not yet played.


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 13):** No dupe items
- **State:** Done (bottled fairy)
- **What was seen in the game:** Confirmed by the user (18 Nov): a second fairy could not be bottled while holding one, and the refusal animation is right
- **Next step:** Other duplicate cases (gifts, other bottle contents) not tried
