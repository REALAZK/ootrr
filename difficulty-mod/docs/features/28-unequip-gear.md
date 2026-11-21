# 28. Unequip sword and shield from the pause menu

Status: NOT POSSIBLE with data edits (studied 20 Nov, no code written, no branch made). Nothing was launched in the game.

## Requested

User, 20 Nov: "make the shields and swords removable by going into the menu and deselecting them". In the Equipment tab, choosing a sword or shield equips it. Choosing the equipped one again should unequip it, so Link has no sword or no shield (challenge play, testing the gates).

## Finding

The menu's equip and unequip decision is made in native code. No data file the menu reads has a toggle, a "none" entry or a hook that runs a flow or graph.

What was checked (all in the game's romfs, read only):

- `UI/Pouch/PouchTable/PouchTable.game__ui__PouchTable.bgyml`: the Equipment tab's table. `EquipmentArray` lists, per button, the actor names (`Weapon_Sword_Biggoron/Giant/BrokenGiant`, `Weapon_Sword_Master`, `Weapon_Sword_Kokiri`, `Weapon_Shield_Mirror/Hylian/Deku`, tunics, boots), the button pane (`BtnName` `Pa_Epuipment_00` to `08`, `Pa_FootEquip_*`, `Pa_HeadEquip_*`), `IsUsableAdult`, `IsUsableChild`, `IsChangeMenuIfInputLeftRight`, `RightBtnParts`, `SpecialDeal`. The only per-entry behaviour flag, `IsChangeActorEquipState` (5 uses), belongs to the preview model. There is no "can unequip" or toggle field.
- `Pack/Actor/PouchSystemActor.pack.zs`: its graph `PouchSystemActor.event.root.ainb` holds only event nodes for flows (add or remove pouch content, bottles, `EventTriggerRemoveWeaponKeepSheath`, `EventQueryIsExistInPouch`). Nothing handles a menu selection. `IsNoSelectIfSameInstance` is a property of those event nodes, not of the menu.
- `Pack/Actor/PouchPlayer_Adult.pack.zs`: the menu's Link model. Its graph has only `PouchWait`, `PouchTurn`, `QueryGetCurrentMenu`, camera and rotation. `PouchPlayerParam` and `MenuActorParam` are camera numbers only.
- Weapon and shield packs (`PouchContentParam`, `WeaponParam`, `ShieldParam`): category and get-type values; no equip rule.
- `UI/Layout2Archive/Common.Product.100...blarc.zs`: the equipment panes and animations (`PaEquipmentMenu_00`, `Pa_Epuipment_00` to `08`, `IsEquip` animation tag for the equipped mark, `SelectEquipBase_*`). Layout is drawing and animation only. It cannot start an event.
- Save fields: `PlayerStatus.EquipWeapon`, `PlayerStatus.EquipShield` (String64, default empty), plus `AnotherEraEquipWeapon`, `AnotherEraEquipShield`, `EquipArmor`, `EquipBoot`, `EquipHeadArmor` in `data/flags.json`. An empty string means nothing equipped.

The blocking fact: which actor name is written into `PlayerStatus.Equip*` when the player presses A on an Equipment-tab button is done by the native pouch/menu code. That code reads `PouchTable` (an item always selects an item of the table, never "none") and the layout panes, and neither carries a rule or a call into a flow or graph. A data edit can only change which items the table lists and where they are drawn.

## What the game already does in data (useful, but not a menu trigger)

- The empty state is real and handled. `Event/EventFlow/ReplaceEquipmentForEraChange.bfevfl.zs` (the child/adult switch) sets `PlayerStatus.AnotherEraEquipShield` to `""` when the other era's shield is not owned (events 18 to 21), and copies the two strings across with `EventTriggerCopyGameDataString64`. So a slot left empty is a state the game itself reaches.
- Event/AI primitives exist that act on Link's gear: `EventPlayerUnequip`, `EventUnequipWeapon`, `EventTriggerUnequipEquipment`, `EventTriggerRequestDeletePlayerEquipmentFromPouch`, `EventTriggerSyncPlayerEquipmentAndPouch`, `OneShotPlayerRequestAllEquipmentSheathed`, and the `Player_Event` actor members `EquipmentUser_Weapon` and `EquipmentUser_Shield` (with their `_EquipmentState`). Setting the string to `""` and then syncing equipment to the pouch is the likely shape of an unequip event. This is untested.
- What is missing is only the trigger: no flow, graph or table runs when a menu button is pressed.

## Data-only routes that were rejected

1. A "none" entry in the Equipment tab. There are nine equipment buttons (`Pa_Epuipment_00` to `08`) and all are used. A tenth needs new layout panes in the blarc (large, binary, and untestable without the game running). Its actor would also have to be a real item: selecting it equips a real (invisible) sword or shield, so Link still draws and swings, and the gate check "sword owned" is unchanged. It is not "no sword".
2. Removing the sword from the pouch (`OneShotRemoveFromPouchWeaponKeepSheath`). That loses the item, which is not unequipping and breaks gates that check ownership.
3. A pad-driven trigger during play (for example a chord that runs the unequip flow). Flows run only when an event starts, and pad queries are not exposed to flows (only `EventWaitForKeyInput` waits and virtual-key nodes inside the Player graph). It would mean editing the Player AI graph, which the repo's parser cannot round-trip with attachments, and it would not be the menu anyway. Not attempted.

## Nearest workable alternatives

- For our own testing only (allowed as a test tool, never part of the mod): a Longshot-side command that writes `PlayerStatus.EquipWeapon` or `EquipShield` to the empty string and forces the equipment sync. The control bridge in v3.0.0-rc.3 reads flags only (`docs\control.md`), so this needs a Longshot change (our `ootrr-tools` branch). Not built.
- A mod-side, data-only choice that works on every runtime but is not the menu: an interactive spot (an NPC talk or a prompt, as the Temple of Time pedestal prompt does) that runs a flow which sets the string to `""` and syncs. Needs a decision from the user on where such a spot should live; not built, because it is a different feature from the request.
- For testing the gates: they check ownership, so the existing `gate-sword` work can be tested by not owning the sword (new game start before the sword is found) without any unequip.

## How to confirm the blocking fact in the game (hands-on, NOT TESTED)

Open the Equipment tab, press A on the equipped sword, and see that nothing changes. That is the vanilla behaviour this study found no data lever for.
