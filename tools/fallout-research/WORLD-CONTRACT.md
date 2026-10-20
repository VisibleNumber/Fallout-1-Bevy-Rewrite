# World state and Bevy boundary

The research simulator now produces an initialized opening-cave state. This is a headless startup milestone, not a playable engine or a claim of original-game parity. Read local reports/sessions/cave-run.json after running cave-run.

## Snapshot schema 1

`world` contains schema, playerId, nextId, prototypes and objects. The object and prototype collections are arrays in JSON, and Maps inside WorldState. Objects have stable numeric id, pid, tile, elevation, rotation, ownerId, inventory and type-specific data. Inventory slots are `{ itemId, quantity }`; owned objects remain in the flat object collection. A null ownerId denotes an unowned object. Player character data comes from the selected GCD.

Original object IDs are retained for imported map instances. Runtime-created objects receive monotonically increasing IDs. Script VM pointers are tagged values `{ type: 'pointer', value: id }`; pointer value zero denotes null and is distinct from ordinary integer operands. Do not use renderer entity indexes as these IDs.

Loaded object data is preserved. Created items initialize prototype-backed weapon ammo, ammo package quantity, charges or key code where applicable. Structural transfers currently keep separate entries; original item merging, currency/ammo exceptions and equipment flags are not yet reproduced.

## Coordinates

Object tile numbers refer to the 200-by-200 hex grid; retain the original integer tile, elevation and rotation even when adding a 3D transform. Elevations range from zero to two; rotations from zero to five. The map's floor/roof tile layer uses a different 100-by-100 square grid. Do not equate those cell indices or infer a physical height scale from elevation alone.

The cave startup's placement operation changes logical tile/elevation/rotation only. It does not perform collision resolution, camera changes, animation or the original engine's combat/visibility updates. A Bevy adapter must implement those systems and explicitly document coordinate conversion and scale.

## Effects and variables

Reports record item creation, inventory transfers, entry placement, ambient-light changes and scoped variable writes in execution order with instruction offsets. VM program-global slots, exported variables, game quest globals, map globals and script locals are separate namespaces.

Ambient light is a numeric state value; no renderer output is produced. The reference's special midpoint lighting formula is preserved instead of replacing it with a linear percentage. Character skills require installed difficulty; time-sensitive traits use the supplied game clock.

## Completed local startup checks

The three original premade player records each complete the map controller's bootstrap and map-enter path. Regression checks verify normal termination, no unresolved opcode, placement/light effects, player inventory ownership, known prototypes, initialized weapon ammunition and a world snapshot round trip. The object-trigger scripts, update loop, spatial dispatch, combat and save compatibility remain outside this milestone.

## Asset separation

Generated snapshots contain locally extracted game data. They are ignored and must remain local. Commit only schemas, authored code and synthetic fixtures. The Bevy project should load or generate private snapshots from the user's own game installation.
