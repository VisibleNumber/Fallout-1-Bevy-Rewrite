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
