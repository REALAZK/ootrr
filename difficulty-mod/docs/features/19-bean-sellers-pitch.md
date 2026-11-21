# Bean Seller's sales pitch

Setting `bean_pitch` (number 128), checkbox, in the "Just for fun" section. Code: `crates/core/src/difficulty/bean.rs`. Built by a sub-agent on `feature/bean-pitch` (`e74c8e4`).

## Request
"also upgrade the Bean Seller's dialog so when you talk to him there's a 1-in-5 chance he'll say 'Chomp Chomp... would you like some Difficulty Mods' instead of his usual dialog."

## Interpretation
One talk in five he says the new line. His normal sale must stay reachable, so the line is added in front of his usual talk and does not replace the sale.

## Investigation
- His flow is `Event/EventFlow/Npc_Hylia_047_Child.bfevfl.zs`, entry `Talk`.
- The node list has `EventQueryRandomChoice5`, an exact 1-in-5 roll. No shipped flow uses it; the 2, 3, 4, 6 and 8-way versions are used.
- Text lives in one archive per language. The repo's Navi return feature already adds new message labels to every language, which gave the method.
- The randomizer edits this same flow for bean prices, so the edits have to combine.

## Implementation
- After his greeting step, a 5-way roll: case 0 shows the new line, cases 1 to 4 go straight to his usual talk.
- The new line is a copy of one of his own chewing lines, pointing at a new label `OOTRR_BeanPitch`, added to all 16 language archives in English.
- Text: "Chomp Chomp..." (pause, new line) "would you like some Difficulty Mods?" with "Difficulty Mods" highlighted like "magic beans". The question mark and line break were added.
- After the line the conversation continues into "You a buyer?" and the sale.

## How it was tested
- 21 difficulty tests, including one that applies the pitch on top of the bean price edit and checks each language archive changes only that one file.
- A real build; `flow-loads` clean; all 16 archives hold both the bean line and the Navi return texts.

## In the game
Never played. To test:
- Turn this on alone, go to the bean seller in Zora's River, and talk to him 15 to 20 times.
- Pass: the line appears roughly one time in five, then his normal sale follows.
- Report: the line never appears (the 5-way roll may not work), the game hangs at the roll, the text overflows the box, or his voice and animation look wrong on the new line.

## Notes
- If the 5-way roll does not work, the fallback is the shipped 6-way roll with the sixth case rolling again.
- This agent also fixed a build-order problem that affects every feature: two late build steps editing the same file could overwrite each other. The fix is in the combined branch.
- Pattern to reuse for any NPC line: copy one of the NPC's own talk events, point it at a new label, add the label to every language.


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 19):** Bean Seller's pitch
- **State:** Done
- **What was seen in the game:** User (18 Nov): the dialogue works
- **Next step:** None
