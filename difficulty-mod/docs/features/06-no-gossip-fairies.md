# No gossip fairies

Setting `no_gossip_fairies` (number 114), checkbox. Code: `crates/core/src/difficulty/flows.rs` (`no_gossip`).

## Request
"no gossip fairies (checkbox)"

## Interpretation
Playing a song to a gossip stone no longer produces the fairy that heals you.

## Investigation
- Searching file names for "Gossip" found the actor `GossipStone_FairyGenerator` and the flow `Event/EventFlow/GossipStone.bfevfl.zs`.
- In this remake the stone produces a "fairy spirit" that heals 8 hearts and fills magic.
- The flow entry `BlewOcarinaByPlayer` tells the generator "the ocarina was played", then checks a flag `GossipStone_FairySpiritAppearOk` to decide whether the fairy scene runs. If that check fails, the flow simply ends: that is the game's own "no fairy this time" path.

## Implementation
Two small edits to the flow, both steering it down the game's own no-fairy path:
- The "ocarina was played" message to the generator is changed to "not played".
- The check that starts the fairy scene is given a case value that can never match.

No events are added, so the file does not grow.

## How I tested it
- Unit test on a made-up flow.
- The game-file test stages the real flow and confirms exactly one "played" message and one gate were found and changed.
- `flow-loads` on the built mod is clean.

## In the game
Never played. To test:
- Turn this on alone, take some damage, play Zelda's Lullaby (or any song) at a gossip stone.
- Pass: the song plays and nothing else happens; you can move again right away.
- Fail cases to report: a fairy spirit still appears, Link is stuck after the song, or the stone stops talking with the Mask of Truth.
- Also hit the stone with a sword and talk to it, to check its other reactions still work.

## Notes
- Sun's Song and Song of Storms fairy spots are a separate mechanism (`FairySpirit_AppearPoint`) and are untouched.
- Good pattern to reuse: when the game already has a "nothing happens" branch, route to it. That is safer than cutting events out.

## Seen in the game
Not tested (batch 1 stopped early; no song-playing routine exists).


## Moved from STATUS.md (20 Nov): feature table row

- **Feature (old table row 06):** No gossip fairies
- **State:** Done
- **What was seen in the game:** User (18 Nov): none of the six songs produced a fairy at a gossip stone
- **Next step:** None
