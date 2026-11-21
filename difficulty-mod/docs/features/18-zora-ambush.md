# Zora's Domain ambush

Setting `zora_ambush` (number 127), checkbox. Code: `crates/core/src/difficulty/ambush.rs`. Built by a sub-agent on `feature/zora-ambush` (`f098307`).

## Request
"in zora's domain, there's a hole that has absolutely nothing in it. let's make that more fun. put a wall skulltulla on each of the far walls, and when you go into the wall area, spawn a ceiling skultylla behind you, so it catches you on the way out. heheh. fun"

## Interpretation
Find the empty hidden hole in Zora's Domain, put a wall-crawling Skulltula on each far wall, and make a hanging Skulltula appear near the way out once you have walked in.

## Investigation
- Zora's Domain has exactly one hidden hole. Its cover is near (17, 100, -18) and the cave is about 40 m below. It holds only six blue rupees. Your later map link points right at it.
- The cave is the piece `Land_HidingHole_Base_D_01`: a central chamber with three side caves. The same piece is used in the Hyrule Castle hole, where the side caves are sealed by bombable walls; here they are open and empty. "Wall area" was read as those side caves.
- **Appear on entry already exists in the game:** the first room of the Forest Temple has a four-node logic graph where an area sensor latches and creates a Skulltula. It can be reused as it is.
- Wall and ceiling heights came from the cave's visible model, not its collision shape.

## Implementation
In all three Zora's Domain maps (child, adult, adult alternative):
- **3 wall Skulltulas**, one on the back wall of each side cave, about 3 m up. Two are exact transplants of the vanilla wall Skulltulas in the Hyrule Castle hole. The third cave has no vanilla example; its spot was picked from the model.
- **3 hanging Skulltulas**, one in each side cave's doorway, 3.8 m above the floor.
- **3 trigger areas** covering the inner part of each side cave. Each is tied to its Skulltula through a new group that points at the Forest Temple graph.

## How it was tested
- 22 difficulty tests; one checks the code reproduces the vanilla castle coordinates exactly, one checks all three real maps and that Gold Skulltulas are untouched.
- Builds with the toggle on and off differ only in the three map files and the size table.
- Positions are plotted in `<builds>\rc-4\entity-maps\ZoraDomain_Child_zoom.png`.

## In the game
Never played. To test:
- Turn this on alone and drop into the hole in Zora's Domain.
- Pass: a wall Skulltula on the back wall of each side cave; walking to the back of a side cave makes a Skulltula drop in its doorway.
- Report per cave (A is toward +x, B toward -x and -z, C toward +z): Skulltula floating off the wall or inside it, ceiling one too high or low, or no ambush at all.
- If you meant a different spot, say where.

## Notes
- The third wall Skulltula is the most likely to look wrong.
- Whether the Forest Temple graph runs in another map is untested.
- The object map site shows this cave's real collision; with it the positions could be corrected precisely.
- Reusing a vanilla logic graph by pointing a new group at it is a cheap way to add scripted behaviour to a map without editing any graph.

## After the first playtest (19 Nov, branch `feature/polish-r5`, `10e87c3`)
You: "the ambush is almost perfect. the ones on the walls are great, the positions of the ones falling down is almost-perfect (they could be a bit closer to the center and further from the wall) the problem is the triggering. they trigger when you're underneath them. they shouldn't trigger 'till you're all the way in towards the wall one".
- Positions: each hanging Skulltula moved 1.25 units straight toward the middle of the hole (the DestinationAnchor, local origin). Wall Skulltulas untouched.
- Trigger: the Forest Temple graph is gone from this feature. Each side cave now has its own private logic graph built with the enemy gates construct (`gates::wire`: player-sensor Area, hit check, AND, and a node that creates the Skulltula). The Skulltula is created only when you stand in a trigger box at the back, by the wall Skulltula (1.5 in front of it, 4 deep, as wide as the cave). It then appears in the doorway behind you and drops by its own behaviour when you pass under it. No enemy counter or barrier is used.
- One box and one Skulltula per cave, not all three at once: each box creates its own cave's Skulltula.
- Tests: new ones for the nearer positions, for the box lying past every dropper measured from the middle of the hole, and for the private graphs; the real-map test checks the box distance in all three maps and that the three graph files are staged.
- Open: not seen in the game. Unknown whether a created Skulltula drops at once or waits for you to pass under it.

## rc 13 scripted check (19 Nov)
- Zora's Domain loads with the mod. The three hanging `Enemy_Skulltula` are already in the actor list on arrival (13.3, 63.6, -22.7), (22.0, 63.6, -18.2), (15.6, 63.6, -13.2), next to the 3 wall ones, before Link goes near; the same six were listed with Link in the room at (19.0, 59.8, -18.3). The trigger is not confirmed (the list cannot tell created from waiting actors). Cave walk NOT TESTED. Details: `playtest
c13\RESULTS.md`.


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 18):** Zora's Domain ambush
- **State:** Seen, fix built
- **What was seen in the game:** Seen in game: wall Skulltulas great, droppers nearly right but triggered underneath. Fixed in polish-r5 (trigger box at the back, droppers 1.25 nearer the middle) rc13 (19 Nov): Zora's Domain loads. The three hanging Skulltulas are already listed at map load (before any trigger), so the trigger is not confirmed; cave walk NOT TESTED.
- **Next step:** Retest: nothing drops until you reach the back of a side cave

## Cave C "never fires" (21 Nov, branch `feature/zora-bottle-fixes`)
Not a code bug. The earlier scripted check stood at the back of cave C and waited; the dropper there never falls from that spot because it only drops when Link passes within about 3 units, and in cave C it is 8.8 units from the trigger box (4.3 to 4.7 in A and B).
Seen in the game (child, sword, more enemies):
- Warp straight under cave C's dropper without visiting its box: it stays up (the trigger gates it).
- Warp to the box (11.8, -5.4), then warp or walk out past the dropper (15.6, -13.2): it drops (y 63.6 to 61.0), Link takes a bite (80 to 76 quarter hearts).
- Same walk with no sword: nothing drops. Cave A: the same, drops when Link walks out past it.
- The trigger box, wall spider and dropper positions are unchanged.
Added a test: each dropper lies on the line from its box to the middle of the hole (within 1.5), between the two, and 3 to 10 units from the box, so a check that only stands in the box is known not to see it fall.
To test by script: after the box, walk out past the dropper, do not wait at the back.
