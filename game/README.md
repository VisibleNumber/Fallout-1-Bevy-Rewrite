# Original-art Bevy renderer

First rendering slice of the rewrite, loading local original artwork and the opening cave map. Not a complete game or a 1:1 parity claim yet.

From the repository root:

```powershell
$env:FALLOUT_GAME_PATH = 'C:\path\to\Fallout'
node game/prepare.mjs
cargo run --manifest-path game/Cargo.toml
```

Left-click ground to walk, WASD pans the view, and R resets. Walking follows the original hex adjacency with deterministic shortest-path navigation and original walking frames in all six directions. Visible critter/scenery/wall flags and multihex flags contribute to blocking. The floor footprint is currently approximated from nonblank square tiles. Exact original path tie-breaking, FRM motion offsets, timing, roof handling, lighting and object ordering still need verification. Combat, interactions, original UI and audio are not connected. A new click is accepted between steps; it does not interrupt an active step.

Run `cargo test --manifest-path game/Cargo.toml` from the repository root for synthetic navigation checks. Run the executable with `--smoke` for an automated local route traversal, screenshot and exit. That check requires prepared local assets and a graphics-capable desktop.

Assets are generated privately in game/assets and ignored by Git. No original game data is shipped. The source 3D scene is separate and is not used here.

The original bottom interface artwork is displayed at its native 640-by-100 size, centered at the bottom of the window. The inventory button works; other buttons remain visual only. HP and AC now show imported initial character values, covering the example numbers baked into the background. Combat does not update those values yet. Interface clicks do not issue walking orders. Flat map objects use a separate ground rendering layer. Exact wall occlusion remains incomplete. Clicking during a walk replaces the remaining route after the active step completes.

## Inventory and weapon selection

I or the inventory button opens the original inventory background. Click an item or use Up/Down to inspect original names, descriptions, quantities and icons. E/Enter or the right-hand slot selects a weapon; I/Esc or Done closes the screen. Weapon selection changes the bar icon and the character's original standing/walking weapon sprites. Inventory pauses movement. Original drag/drop, consumable use, ammunition operations, combat and equipment effects are not yet implemented. Presentation groups identical starting entries for display; this is not a complete implementation of original stacking rules.

Starting items come from executing the effective opening controller script for Max Stone during asset preparation. This currently requires a local reference source checkout for opcode registration names: set FALLOUT_REFERENCE_PATH as described in the repository README. Preparation invokes the existing startup simulation, keeping generated data private. Initial skill/stat calculations have the documented simulation limits.

## Quicksave

F5 stores position, facing and selected weapon; F9 restores them. Saves are validated against current walkable tiles and known weapons. Saving waits for the active movement step to finish. The local file is game/assets/quicksave.json, ignored by Git. This is a small rewrite-specific save, not original Fallout save compatibility and not a complete world/inventory snapshot. Inventory quantities cannot yet change in gameplay.

`--inventory-smoke` opens inventory, selects the pistol, captures open and closed views, verifies a save-file round trip and exits. `cargo test` covers the inventory model and save validation alongside navigation. Generated screenshots and save files stay local.
