# OOTRR 0.8.2

A fix-up release. Beating a temple, or reaching a later part of its story, before doing an earlier step no longer locks you out of checks. The biggest change is the Gerudo Fortress, which now follows the Gerudo Membership Card instead of the Spirit Temple.

## Gerudo Fortress

- **The fortress follows the Gerudo Membership Card.** Without the card, the fortress is hostile. With the card in your inventory, the guards are friendly, the fortress looks cleared, the bridge in Gerudo Valley is repaired and horseback archery is open. Beating the Spirit Temple, or reaching the Desert Colossus or the Spirit Temple first, no longer changes any of this.
- **The carpenters can always be rescued.** Before, reaching the Spirit Temple first left the carpenters in their cells talking as if they were already free, and the thieves who guard them never appeared. Now they stay jailed until you free them, each one still calls his thief, and freeing all four still plays the Membership Card scene with its randomized item. This works with or without the card: a card found somewhere else does not empty the cells.
- **Open fortress:** the guard who gives the Membership Card check now offers it when it is still owed. Before, she would only replay the scene after it had already played.

## Checks that stay available

- **Link the Goron (Goron City, adult):** his gift is no longer lost if you go to the Death Mountain Crater first or beat the Fire Temple first. In that case he no longer rolls around; talk to him and he gives you his item. He is now also there when you enter Goron City from the Crater.
- **King Zora's thaw gift (Zora's Domain, adult):** if you beat the Water Temple before melting the red ice, King Zora is free when you return. Talk to him and he gives you the check, whether or not you already own a Zora Tunic.

## Quests no longer go backwards

Some places in the game moved a story quest back to an earlier step when you visited them late, which could undo progress in a temple. They now leave the quest where it is:

- **Spirit Temple:** talking to the carpenter boss in Gerudo Valley or the guard at the Haunted Wasteland gate, arriving at the Desert Colossus through the Haunted Wasteland, and getting Requiem of Spirit after you have already made progress in the Spirit Temple.
- **Fire Temple:** Link the Goron's first scene, stopping him and receiving his gift, if the Fire Temple quest is already further along.

## Known limits

- The logic has not changed. It still expects the Gerudo Valley bridge to be repaired once the carpenters are rescued; in the game it is now repaired once you hold the card. This only matters for walking back across the valley.
- The new Goron and Zora gifts are extra ways to get those checks. The logic still asks for the usual items (something to stop Link the Goron, blue fire for King Zora).
