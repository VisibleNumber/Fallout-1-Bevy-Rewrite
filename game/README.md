# Original-art Bevy renderer

First rendering slice of the rewrite, loading local original artwork and the opening cave map. Not a complete game or a 1:1 parity claim yet.

From the repository root:

```powershell
$env:FALLOUT_GAME_PATH = 'C:\path\to\Fallout'
node game/prepare.mjs
cargo run --manifest-path game/Cargo.toml
```

WASD pans the view, arrow keys move the player sprite, R resets. Movement has no collision/pathfinding yet. Static object frames and palette are original; animation, roof handling, accurate draw ordering and original lighting remain incomplete. Combat, interactions, original UI and audio are not connected.

Assets are generated privately in game/assets and ignored by Git. No original game data is shipped. The source 3D scene is separate and is not used here.
