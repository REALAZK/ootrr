# Difficulty settings: what does not work yet

Newest state first. "Seen" means seen in the running game. Items move to the changelog when they work.

## Broken or unsafe (rc 19)

- **Random enemy sizes (145, 146): not working.** The game ignores placement Scale. Labelled "not working yet".
- **Volvagia in the field (143): Functional WIP.** Freezes at death, no regen. Wanted: red rupee, then she flees.
- **Surprises did not show in field grass** (6% per surprise).
- **Gates 14, 19, 21, 22 are easy to walk round.** Gates 15 and 16 are in the wrong place and are to be removed.
- **Backup ladder** needs a proper difficulty order derived from the game files. Mega Like Likes still call small Floormasters.
- **Night Stalchild doubling has no effect.**
- **Desert Colossus storm: switched off** (Link fell through the floor).
- **Fixes from hands-on 10** are not in rc 19: they come as small separate pull requests.

## To confirm by hand

| What | State | What to do |
|---|---|---|
| More enemies slider, rc 15 rules | Played at 10x in rc 14; the rules that came out of it (peaceful places, perched Keese, big enemies apart, Iron Knuckle cap, nothing in lava, no added Dodongos in Dodongo's Cavern) are built, not all seen | Play child Hyrule Field, Kokiri Forest, Kakariko and Dodongo's Cavern at 10x; then 2x and 5x; a room whose door waits for its enemies; die in a dungeon and check the retry is safe |
| Enemies call for backup, lesser kind | Built (rc 15), not played | Fight a Stalfos (a Stalchild should come), a Wolfos (two Keese), a Lizalfos (a Keese). Eight callers called nobody in an open-field script (Stalchild, Freezard, Skulltula, Wallmaster, Armos, Spike, Tailpasaran, Biri): check some in their homes |
| Does a kill free one of the 12 spawn slots? | Unknown | Spawn 12, kill some, spawn again |
| Boss adds in the other rooms | Seen with Gohma and Phantom Ganon (by hand) and Twinrova (by script) | Barinade, Volvagia, Morpha, Bongo Bongo: walk in, the adds come with the fight |
| Shadow Temple with "Bosses spawn minions" | Black screen before the rc 16 fix; the fix is not loaded in the game yet | Load the Shadow Temple with the setting on, then fight Bongo Bongo |
| Fireball storms, other stretches | Middle road and castle garden seen | Hyrule Field south-central and west road, child and adult |
| Gates with dungeon entrance shuffle | Gates 9 present and 2 absent (script) | Gates 1 and 6 absent |
| Jabu-Jabu water spawns | Positions fine (script) | Look at the scrub room pool at 10x: no Octorok under a ceiling |
| Zora's Domain ambush | Map loads, Skulltulas present | Walk the cave |
| The held-back gate 7 | Never tried | Walk the Lost Woods east passage, kill the guard, see the barrier drop |
| Dead Hand in the Hyrule Field hole | Rises near the entrance; left as is by decision | None |
| Fake pots | Never seen | Break pots until one flies |
| Lake Hylia Octoroks and Mad Scrubs (rc 17) | Octoroks by script; Mad Scrub kill seen by hand | Swim at the bridges (3 to 4 at once took 15 hearts, frame rate 22 to 29); look at the adult field scrubs |
| Volvagia in Hyrule Field (rc 17, Q17) | Fought by hand: freezes at death (WIP) | After the rupee-and-flee change: fight her again; win the real Fire Temple fight with the setting on and see her gone; Q17 harshness |
| Gates 14 to 23 (rc 17) | Gate 23 seen by hand; the rest by script, and gates 9, 14, 20, 21, 22 changed after the hands-on | Real sword kills at each, gate 20 (3 Stalfos) by day and night, Nocturne arrival at the Graveyard, gate 22 with the lake drained, gate 15 after the guard dies |
| Enemy randomizer v3 (rc 17) | Not played | Load Bottom of the Well, Dodongo's Cavern, Ice Cavern, child Spirit Temple; judge movement and walls |
| Random enemy sizes (rc 18) | Not working (gallery showed no size change) | Fix first, then Mild, walls, the mix with other settings |
| Shorter timers (rc 18) | Switch and torch times by script | Win the Dodongo's Cavern torch room at 50%; the eye switches |
| Damage taken (rc 18) | Played by script incl. fairy revive | Pit and lava respawn, grabs, one hit kills by hand |
| Bottle refusal notice (rc 18) | Seen by script | Look at the notice wording in a fight |

## Not built yet (asked for)

- Volvagia: red rupee, then she flees.
- Fireball storms: the erupting Death Mountain backdrop during a storm; the desert storm back without the fall through the floor.
- Gate editor with a live, real barrier tied to a dummy guard (Longshot tool).
- Panel: a teleport that switches age, or refuses.
- Gates 15 and 16 higher up the Death Mountain Trail (waiting for the user's screenshot).
- Enemy sizes: the real fix.

- The Gerudo fighter as a placed enemy (she plays a fortress scene when beaten), and Volvagia or other bosses outside their rooms.
- Enemy speed: most enemies ignore the setting (see enemy-speed.md). Making the others follow it is open.
- Mido as a blocker; lose-heart surviving a quit.

## Gates

Eleven of the first twelve were played by hand in earlier sessions. Gate 23 (Dark Link) was seen by hand on 21 Nov; gates 14 to 22 passed by script only. Gate 7 (Lost Woods, east passage) is held back until someone tries it.

## Works, with a known limit

- **Lose heart on death:** the loss lasts for the play session; quitting and continuing restores the hearts.
- **Limited ammo with shuffled shops:** a seed can end up with no source of arrows; boss fights keep their ammo (round 2, rc 17, not played).
- **Dead Hand in the Hyrule Field hole** rises right by the entrance, although he is placed at the back of the room.
- **Pots in the Market guard house** give no surprises.
- **Bosses spawn minibosses:** King Dodongo is left out; adding a miniboss there hung his dungeon.
- **Fireball storms:** child Hyrule Field ran at about 30 to 37 frames a second with 5x more enemies plus the storm.

## Never seen in the game

Enemy speed by hand (measured by script: most enemies ignore it, see enemy-speed.md), enemies respawn sooner, Zora's Domain ambush, the ten Like Likes at Lake Hylia, fake pots, random enemy sizes (they do not work).

## Ideas saved for later

- Grass that normally gives bugs joins the surprise pool.
- The Gerudo fighter as a gate guard.
- Mido as a visible blocker.
- A bomb-proof, glowing wall was replaced by the purple barrier; nothing further planned.

- Helper fairy (user, 21 Nov): a fairy that keeps 1 unit from Link, pointing at the nearest pot, grass or other target. Needs bridge commands `props <name>` (list placed actors with positions) and `spawn --at x y z`. Skill: .claude\skills\helper-fairy\SKILL.md.
- Panel: use the bridge's `magic` and `bottle` commands for magic refill and for filling an empty bottle (give is unreliable).

- Gate editor v2 (user design, 21 Nov), Longshot tool, first after #52:
  - Off: everything is normal.
  - On (F8): the nearest gate is temporarily replaced by a WIP gate whose dummy guard is a Gold Skulltula. Link gets a slingshot or bow with full ammo.
  - Where a shot lands marks the gate's left end, then its right end; a real barrier is spawned between them.
  - Killing the Skulltula (one hit) tears the WIP gate down, to try again.
  - Leaving the mode with the Skulltula alive saves the gate (gates.json plus the export); with it dead, nothing changes.
  - How: the editor tracks the arrow or seed actor and takes its last position as the impact. The editor itself watches the Skulltula's health and removes the barrier when it dies, which sidesteps the never-found barrier-to-guard link. Real barrier pieces are spawned at the posts so the blocking is exact.
