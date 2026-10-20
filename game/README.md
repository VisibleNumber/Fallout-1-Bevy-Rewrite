# Fallout 1 Bevy rewrite: opening cave

This is the original-art, isometric rewrite. It loads the installed game's Vault 13 cave, original floor and object artwork, player sprites, rat sprites, inventory art, and bottom interface. It is a playable opening-cave prototype, **not the complete game or 1:1 behavioral parity**. The separate experimental 3D scene is not used here.

From the repository root:

```powershell
$env:FALLOUT_GAME_PATH = 'C:\path\to\Fallout'
# Optional if the local reference checkout is not in the development default location:
$env:FALLOUT_REFERENCE_PATH = 'C:\path\to\fallout1-ce'
node game/prepare.mjs
cargo run --manifest-path game/Cargo.toml
```

All extracted artwork, map records, screenshots and saves stay in ignored local folders. The repository ships source and synthetic tests only. Preparation executes the opening controller script for Max Stone to obtain the starting inventory; the reference checkout supplies opcode registration names. See the root README for that checkout's setup and license.

## Controls

- Left-click ground to walk; WASD pans the camera. Home returns to the starting position before combat has been used.
- I or the inventory button opens inventory. Click an item or use Up/Down; E/Enter or the right-hand slot equips it. I/Esc or Done closes inventory.
- Equip the pistol or knife, close inventory, then click a rat to attack. An empty hand punches. The flare has no supported primary attack.
- R reloads the equipped gun. In inventory, select an ammo row first to load that type; X unloads the gun. Different ammo types cannot mix in a partly loaded gun.
- U in inventory uses the selected Stimpak, rolling its original 10–20 healing range. A Stimpak is consumed even at full health, matching the reference.
- G searches the nearest original container when adjacent and takes its contents, subject to carrying capacity. The opening corpse contains the original knife and 10mm AP ammunition; it can only be emptied once.
- E or right-click examines an object under the cursor using its original name and description.
- Space ends your turn. Walking in combat costs one action point per hex. Attacks use the imported primary action-point cost; the starting pistol uses five AP and one round. Rats move and bite during their turn. The green overlay reports health, AP, ammunition and results.
- F5 saves; F9 loads the cave checkpoint, including player/rat health and positions, AP, equipped weapon, ammunition type, reserve packets, consumed items, looted containers, and the rewrite RNG state. Finish the current movement step first. The file is `game/assets/quicksave.json`.

Original inventory names, descriptions, quantities and icons are loaded locally. Weapon selection changes the original standing/walking sprites and bottom-bar icon. Player attacks use original punch, pistol and knife frames. Dead rats use the final original fall-back frame. Inventory pauses walking and attack animation and costs four AP to open in combat (no Quick Pockets perk yet). Reloading outside inventory costs two AP in combat; inventory reload/use has no further AP charge. HP in the bottom interface updates after damage; action-point lights and the original message area now update too. Other interface buttons remain visual only.

## Implemented combat slice and remaining differences

Ordinary attacks validate range, health, AP and loaded rounds before changing state. Hits use imported skills, target armor class, the basic ranged perception/distance modifier, and a 95% cap. Ordinary damage follows the reference integer operation order for damage threshold and resistance; target defenses are imported by damage type. Misses still spend AP and ammunition. Melee maximum damage includes the initial character's melee damage stat. Rat health/AP/armor and melee damage come from the original map/prototypes. Wall/scenery shoot-through flags contribute to a provisional geometric line-of-fire check.

This is deliberately incomplete: original criticals, aimed shots, lighting/cover accuracy, perks, burst fire, secondary attacks, equipment armor, non-healing drugs, flare use, XP and combat scripts are not connected. The local Fallout 1 CE ordinary-damage path parses but does not apply ammunition damage modifiers; the rewrite preserves that behavior instead of substituting Fallout 2 rules. Rat turn decisions, activation and instantaneous movement are provisional, not the original AI. Combat begins when an attack succeeds validation; it does not yet reproduce original automatic enemy detection. Combat ends when the current encounter participants are defeated; untouched rats elsewhere do not keep it active. Directly attacked rats remain participants even beyond the provisional nearby-rat radius. The participant roster is saved. Older active schema-3 checkpoints without a roster retain their legacy all-living-rats encounter until it ends. Player death stops walking and attacks; restart or load a checkpoint to resume.

Navigation uses the original six-neighbor hex adjacency and shortest-path search. Floor coverage, path tie-breaking, animation motion/timing, roof handling, wall occlusion, object sorting and lighting still need comparison against the original. Geometric hex distance and line interpolation are tested but do not claim identical original ray tie-breaking. Map transitions, dialogue, quests, sound, character creation and the remaining campaign are not playable yet.

Checkpoints use the rewrite's schema 3, not original Fallout saves. Older quicksaves are rejected; F5 creates a new checkpoint. They save the current cave inventory and the rewrite RNG state, but do not yet save original scripts, world time, or other maps. Save output is written to a temporary file before replacing the checkpoint. Validation rejects incompatible encounter identities, invalid health/AP/ammo, unknown weapons and inconsistent player position before applying any state.

## Verification

`cargo test --manifest-path game/Cargo.toml` runs synthetic combat, inventory, save, navigation and hex geometry tests without original assets. After preparation, these executable options run local graphical checks and exit:

- `--smoke`: walks a reachable route and captures a screenshot.
- `--inventory-smoke`: opens inventory, equips the pistol, captures both views and checks the position-record codec.
- `--items-smoke`: reloads from reserves, uses a Stimpak, takes original corpse loot, verifies repeat looting is blocked, then saves and restores quantities, ammunition and health.
- `--combat-smoke`: fires at a visible rat, verifies AP/ammo spending, runs an enemy turn and checks a complete combat checkpoint round trip.

Generated screenshots and checkpoint fixtures remain private under `game/assets`.
