# Opening-cave player journey

Scope: one original cave, the original opening Overseer briefing presented as a separate modal, one rat encounter, inventory, and rewrite saves. No living NPC is invented in the cave. The briefing graph is authored data referencing local original message IDs; original NPC scripting and full campaign quest behavior remain outside this slice.

## Manual reproduction

1. Prepare local assets and launch the rewrite. The player starts on cave hex 17690.
2. Press T. Confirm the Overseer portrait and original opening text appear. Ground clicks, inventory and combat must not operate behind the dialogue.
3. Advance one page, press F5, then advance another page and press F9. Confirm the earlier page returns.
4. Continue through the briefing. Confirm the last choice closes the panel, reports that the water-chip search started, and does not move the player.
5. Open inventory with I. Select the pistol, press E, and close inventory. Confirm the character and hand icon change.
6. Click a visible rat. Confirm one round is consumed and AP decreases, or exploration resumes if that attack ends the encounter. Space runs the rat turn when combat remains active.
7. Press F5. Change position, weapon or ammunition, then F9. Confirm position, health, ammunition, rats and briefing completion restore together.
8. Reload with R. In inventory, X unloads; select an ammo row and press R to choose its type. A partly loaded gun must reject a different ammo type.
9. After taking damage, select a Stimpak and press U. Check its quantity and current HP. Close inventory, approach the original corpse at hex 17488 and press G; repeat looting must not duplicate its contents.

The reference screenshot window may be scaled by Windows DPI. Use imported hex coordinates for world placement rather than treating screenshot pixels or camera height as actor positions.

## Automated whole-journey check

Run the built executable with `--journey-smoke` from `game/` after preparing assets. It drives the same dialogue button interaction handler, checks close-frame input consumption, exercises inventory/attack rules, saves mid-conversation, and restores story, inventory and combat state. It must exit successfully within 20 seconds; a stalled node or assertion fails the run.

Local screenshots:

- `game/assets/briefing-smoke.png`: first briefing page and portrait.
- `game/assets/journey-smoke.png`: cave after conversation, combat and restore.

Local save fixtures are `briefing-save-smoke.json` and `journey-save-smoke.json` under `game/assets/`. They and all screenshots are ignored by Git. Synthetic unit tests separately cover hidden choices, stale menu actions, atomic failures, conditions, and persistence of conversation/quest state. The journey check complements these tests; a successful compile is insufficient.

## Original script conversation

Outside combat, press Y to access the actual Overseer script. This temporary access does not place an NPC in the cave. Select a response; verify that the next reply comes from the original script, then finish the conversation. Open it again and confirm the Overseer now asks a different question. Save mid-conversation with F5, advance, then F9: the original reply, choices and VM variables should restore together. Esc pauses; Y resumes. Saving/loading while a worker request is pending must wait.

`--original-dialogue-smoke` exercises this complete asynchronous UI path with the actual installed `OVER.INT`, including native inventory reconciliation, save/load both mid-conversation and after completion, and a changed original reply on return. Private captures are `original-dialogue-smoke.png` and `original-dialogue-return-smoke.png`; its checkpoint is `original-dialogue-smoke-save.json` under `game/assets/`.
