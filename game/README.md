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

## Opening briefing and shared rules

T opens the Overseer briefing with the original local portrait and message text. Enter or the visible Continue button advances; the final Enter the cave choice starts the prototype water-chip-search flag. Esc closes the panel. This is an authored presentation of original opening content, not execution of OVER.INT or an NPC placed inside the cave. The cave has no conversational NPC, so none is invented.

The content manifest in `game/content/overseer-briefing.json` contains message IDs and rule effects only. Original text and images are resolved locally during preparation and remain ignored. `dialogue.rs` has no Bevy dependency: conditions, choices, stale-menu rejection, flag changes, and conversation completion are separate from `dialogue_ui.rs`. Saves preserve an active dialogue node and its story flags; F5/F9 also work while the dialogue panel is open. Movement, inventory controls and combat actions pause under the modal. Its closing click is consumed.

## Controls

- Left-click ground to walk; WASD pans the camera. Home returns to the starting position before combat has been used.
- I or the inventory button opens inventory. Click an item or use Up/Down; E/Enter or the right-hand slot equips it. I/Esc or Done closes inventory.
- Equip the pistol or knife, close inventory, then click a rat to attack. An empty hand punches. The flare has no supported primary attack.
- R reloads the equipped gun. In inventory, select an ammo row first to load that type; X unloads the gun. Different ammo types cannot mix in a partly loaded gun.
- U in inventory uses the selected Stimpak, rolling its original 10–20 healing range. A Stimpak is consumed even at full health, matching the reference.
- G searches the nearest original container when adjacent and takes its contents, subject to carrying capacity. The opening corpse contains the original knife and 10mm AP ammunition; it can only be emptied once.
- E or right-click examines an object under the cursor using its original name and description.
- Space ends your turn. Walking in combat costs one action point per hex. Attacks use the imported primary action-point cost; the starting pistol uses five AP and one round. Rats move and bite during their turn. The green overlay reports health, AP, ammunition and results.
- F5 saves the campaign; F9 restores its current location and all visited locations, including health, AP, inventory, ammunition, loot, dialogue and encounter state. Finish the current movement step first. The file is `game/assets/campaign-save.json`.

Original inventory names, descriptions, quantities and icons are loaded locally. Weapon selection changes the original standing/walking sprites and bottom-bar icon. Player attacks use original punch, pistol and knife frames. Dead rats use the final original fall-back frame. Inventory pauses walking and attack animation and costs four AP to open in combat (no Quick Pockets perk yet). Reloading outside inventory costs two AP in combat; inventory reload/use has no further AP charge. HP in the bottom interface updates after damage; action-point lights and the original message area now update too. Other interface buttons remain visual only.

## Implemented combat slice and remaining differences

Ordinary attacks validate range, health, AP and loaded rounds before changing state. Hits use imported skills, target armor class, the basic ranged perception/distance modifier, and a 95% cap. Ordinary damage follows the reference integer operation order for damage threshold and resistance; target defenses are imported by damage type. Misses still spend AP and ammunition. Melee maximum damage includes the initial character's melee damage stat. Rat health/AP/armor and melee damage come from the original map/prototypes. Wall/scenery shoot-through flags contribute to a provisional geometric line-of-fire check.

This is deliberately incomplete: original criticals, aimed shots, lighting/cover accuracy, perks, burst fire, secondary attacks, equipment armor, non-healing drugs, flare use, XP and combat scripts are not connected. The local Fallout 1 CE ordinary-damage path parses but does not apply ammunition damage modifiers; the rewrite preserves that behavior instead of substituting Fallout 2 rules. Rat turn decisions, activation and instantaneous movement are provisional, not the original AI. Combat begins when an attack succeeds validation; it does not yet reproduce original automatic enemy detection. Combat ends when the current encounter participants are defeated; untouched rats elsewhere do not keep it active. Directly attacked rats remain participants even beyond the provisional nearby-rat radius. The participant roster is saved. Older active schema-3 checkpoints without a roster retain their legacy all-living-rats encounter until it ends. Player death stops walking and attacks; restart or load a checkpoint to resume.

Navigation uses the original six-neighbor hex adjacency and shortest-path search. Floor coverage, path tie-breaking, animation motion/timing, roof handling, wall occlusion, object sorting and lighting still need comparison against the original. Geometric hex distance and line interpolation are tested but do not claim identical original ray tie-breaking. Map transitions, spatial NPC interaction, full quests, sound, character creation and the remaining campaign are not playable yet. Original Overseer dialogue execution is available through the temporary Y access point described below. The briefing/quest flag is the first shared dialogue-system slice.

Campaign files use rewrite schema 1 containing validated schema-3 location snapshots; they are not original Fallout saves. Global character, inventory, story, original Overseer VM state and loot records are consistent across those snapshots. Each visited location retains its positions, encounter roster and RNG state. Loading restores the saved location automatically after validating every saved map, including inactive maps. The file is written through a temporary file before replacement. Older single-map quicksaves are left untouched; the F5/F9 controls now use the campaign file. Campaign time is not yet simulated.

## Verification

`cargo test --manifest-path game/Cargo.toml` runs synthetic combat, inventory, save, navigation and hex geometry tests without original assets. After preparation, these executable options run local graphical checks and exit:

- `--smoke`: walks a reachable route and captures a screenshot.
- `--inventory-smoke`: opens inventory, equips the pistol, captures both views and checks the position-record codec.
- `--journey-smoke`: advances the briefing through actual Bevy button interactions, checks that dialogue closure consumes its click, equips the pistol, attacks a rat, and verifies save/load across conversation, inventory and combat. Captures both briefing and cave views.
- `--items-smoke`: reloads from reserves, uses a Stimpak, takes original corpse loot, verifies repeat looting is blocked, then saves and restores quantities, ammunition and health.
- `--combat-smoke`: fires at a visible rat, verifies AP/ammo spending, runs an enemy turn and checks a complete combat checkpoint round trip.

Generated screenshots and checkpoint fixtures remain private under `game/assets`.

## Original Overseer script integration

Press **Y** outside combat to open the actual installed `OVER.INT` conversation. This is temporary access from the cave while Vault 13 traversal is unfinished; there is no invented Overseer NPC in the cave. **T** still opens the separate authored opening-briefing presentation. Enter chooses the first response; click another response to branch. Esc pauses the conversation; Y resumes it.

The Node worker boots the original Vault 13 map script to initialize shared exports, then executes the original Overseer script. Replies and choices come from local message files; Intelligence conditions and callbacks execute in the VM. Program variables persist across responses and return visits. F5/F9 preserve the original session. Saving/loading waits for pending script execution. Inventory reconciliation is atomic and conserves loaded and reserve ammunition; unknown item prototypes stop the transaction. Unsupported script calls report an error and leave the previous accepted state intact.

This binds one NPC script, not the campaign. The original script's isolated Vault 13 world is not yet the rendered cave world; spatial effects, animation/speech, general NPC scheduling, XP and campaign integration remain unfinished. No game content is embedded in the source repository. The local worker requires Node, the installed game and the existing local `fallout1-ce` reference checkout used by the research tools to resolve opcodes; `FALLOUT_GAME_PATH` and `FALLOUT_REFERENCE_PATH` override their local defaults.

Verification:

- `node game/script-dialogue-host.test.mjs`: synthetic dialogue opcode contracts.
- `node game/script-runtime.test.mjs`: requires local game/reference; exercises real original replies, Intelligence filtering, callbacks, JSON persistence, repeat visits and stale-choice rejection.
- `--original-dialogue-smoke`: runs the real script asynchronously through Bevy response buttons; tests inventory bridging, checkpoints mid-conversation and after closing, click consumption, and a changed reply on a return visit. Captures private screenshots under `game/assets`.

## Connected location prototype

Prepare all four supported maps with `node game/prepare-world.mjs` (the same local game/reference environment variables apply). Individual preparation accepts `FALLOUT_MAP=v13ent`, `vault13`, `shadyw`, or `shadye`. Each writes a separate local scene and uniquely named artwork; preparing another location does not replace the default cave scene.

Press **M** outside combat to open the travel menu and select the Vault 13 cave, Vault 13 entrance level, or either Shady Sands section. Imported exit grids also connect supported destinations; world-map exit grids open the same menu. Unsupported maps and upper elevations report that they are not connected. The menu is a traversal prototype: encounters, discovery, elapsed time, original travel scheduling and the original world-map controls are still missing.

Health, inventory, equipped items, briefing/story flags, original Overseer session and looted cave contents follow the character between locations. Returning during the same run restores the location's encounter state. Settlement NPCs render in their imported positions and are neutral; their dialogue, schedules and quests are not yet connected. When the installation lacks an NPC's standing art, that record stays in the world but is not drawn using unrelated called-shot art.

F5/F9 now save and restore the entire visited-location collection across restarts. A load validates every saved map and consistent shared character/inventory/story data before replacing live state. Travel remains blocked during dialogue, pending script work and combat.

`--travel-smoke` visits all four maps, checks health/inventory/story and cave encounter state on return, saves and restores at Shady Sands, then selects a destination through the actual world-menu button handler. It also writes a campaign save, clears in-memory visits to simulate a fresh session, reloads all maps and confirms an inactive-map corruption rejects atomically. Private screenshots are `shady-sands-smoke.png` and `world-travel-smoke.png` under `game/assets/`.

`--campaign-load-smoke` must run after `--travel-smoke` in a separate process. It starts in the default cave, restores the saved Vault 13 campaign and returns to the cave to verify defeated rats, story and consumed inventory survived the process restart.

## Original-art first/third-person prototype

Prepare local assets with `node game/prepare-world.mjs`, then start `cargo run --manifest-path game/Cargo.toml -- --3d`. The default launch remains the 2D rewrite. The 3D mode loads one Shady Sands west block, with WASD movement, right-mouse drag to look, Tab to switch cameras, Shift to run, I for the shared inventory, and E to identify a nearby NPC.

The ground mesh maps the original isometric floor diamonds into 3D. Solid wall sections occupy imported wall positions, and roof surfaces use the original roof-tile grid. Buildings now default to a geometry-first blockout: plain lit materials, chamfered wall edges, solid roof slabs, boundary roof overhangs and framed door placeholders. H toggles roofs so the layout can be inspected. Original floor and sprite artwork retain nearest-neighbor filtering. Wall texture sampling is parked until the shapes are approved; the optional adapter flag `FALLOUT_TEXTURE_WALLS=1` retains the previous UV-region preparation. Player movement checks both the original walkability grid and the solid wall footprint; the follow camera retracts before passing through a wall.

This is an early geometry conversion, not a finished 3D recreation. Wall thickness/height, the hex-based perimeter and door orientation remain provisional; closed door placeholders are not interactive, and roof/camera behavior needs refinement. Shape changes are allowed to improve realism while preserving the Fallout style. NPCs, the avatar and furniture remain directional/billboard sprites. NPC dialogue, combat, 3D campaign saving, map travel and full streaming are not connected in this separate view. The rendered block is limited to roughly 25 map units around the starting point.

`--3d-smoke` renders deterministic third- and first-person views and exits; screenshots remain private under `game/assets/third-person-smoke.png` and `first-person-smoke.png`. Rust tests cover the original hex/3D coordinate conversion, wall collision at faces/corners and closed wall mesh texture bounds.

Original-behavior reference: [Fallout Reference Edition](https://github.com/alexbatalov/fallout1-re), which aims to preserve the original engine behavior. The custom 3D geometry remains part of this Bevy project; reference-engine source is not vendored.

## Local New Vegas conversion samples

`game/nv-assets.mjs` reads version-104 BSA directories without modifying the install. `node game/nv-assets.mjs list <filter>` lists matching assets; `extract meshes/...nif` or `extract textures/...dds` writes only under ignored `game/assets/nv/raw`. `FALLOUT_NV_PATH` selects the installed New Vegas folder. The default also checks Steam's download staging area; partial/locked archives may fail and should be retried after downloading completes.

The static converter uses the open-source [PyFFI](https://github.com/niftools/pyffi) parser and Pillow. Install `pyffi==2.2.3` into `game/target/conversion-tools` with Python pip, and ensure Pillow is available. Run `python game/convert-nv.py meshes/furniture/table01.nif`. It resolves diffuse texture references, converts DDS to PNG, transforms NIF coordinates to glTF coordinates and writes glTF/binary data under ignored `game/assets/nv/converted`. Skinned meshes are rejected. Animation, Havok collision, normal maps, special shaders, alpha handling and placement require further work.

Converted local samples `table01`, `nv_shackvictor`, and `nv_prospectorsaloon_door` appear near the 3D starting point when present. These are import demonstrations, not integrated gameplay objects; building elevation and material tuning remain provisional. The source repository contains the converter only, with no Bethesda assets or bundled third-party parser. All conversion dependencies remain in ignored `target`.
