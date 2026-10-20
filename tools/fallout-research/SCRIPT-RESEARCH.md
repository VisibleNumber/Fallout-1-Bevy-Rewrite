# Script reconstruction progress

The local Steam installation structurally decodes to 952 scripts and 757,163 instructions across bootstrap and code regions. Opcode names are resolved from 340 handler registrations in the optional reference source. These are static checks; they do not demonstrate campaign or gameplay parity. Generated disassembly remains excluded from Git.

## Opening cave findings

The entrance map header resolves to the cave controller script. Its map-entry procedure contains direct calls to helpers for base inventory, tagged-skill inventory, darkness and party handling, as well as a conditional endgame helper. Entry/update/exit procedures are separate. A reconstructed map therefore needs the script lifecycle in addition to object placement. Treat these as possible paths through the code, not actions that always execute on entry.

There are 24 persisted script records and six unique script dependencies including the map controller. Two records are spatial triggers. Object/inventory traversal exposes 1,496 instances, of which 1,494 are top-level placements. All six dependencies resolve to effective assets in the local installation. These counts describe the development edition and can change with patches.

## Important representation details

- The map-header script index is one-based. Script record indices use their low 24 bits as a zero-based script-list index.
- INT files contain executable bootstrap instructions before their metadata tables and executable code after them. Do not decode the tables as instructions.
- Typed constants consume six bytes; the inspected opcode handlers otherwise dispatch on the low ten opcode bits.
- String-tagged constants can refer to either static text or identifier tables, depending on the consuming operation. Keep the numeric offset and both resolvable candidates until consumer semantics are known.
- The script VM data stack, return stack, program globals and game quest globals are distinct. An operation named fetch_global in the interpreter does not automatically mean a world quest variable.
- A resolved call in this tool requires an adjacent pushed integer. More complicated stack expressions and indirect calls remain unresolved.

## Next implementation boundaries

1. Reproduce typed stack operations, control flow, local frames, calls and returns with synthetic programs.
2. Define host bindings for world objects, inventory, variables, timers and messages without connecting them to rendering.
3. Trace the cave controller's map-enter procedure under recorded original-game conditions; document each state change.
4. Add map-update/map-exit and spatial/object trigger dispatch, then compare save/load transitions.
5. Feed the same state layer to Bevy presentation once the original behavior is reproducible.

## Integer execution subset

The tool now includes a bounded integer VM subset with independent synthetic tests for operand ordering, signed division, branches, internal calls, local/program-global stack slots, return-stack transfers, frame operations and execution limits. Strings, floats, pointer values, external calls and the engine scheduler remain unsupported. Arithmetic wrapping represents a deliberate 32-bit model; edge cases still need original executable comparison.

Bootstrap probing with no host context halts 19 scripts and blocks 933 at the first unimplemented dependency in the development installation. These are bootstrap outcomes only, not completed game-script runs. The most common first dependencies are script action and the current script owner object.

The cave's map-enter procedure first queries whether the map is newly entered. With no context, the probe stops at that query. An explicit scalar map-flags context lets it evaluate the condition and reach the request to override the player's map entry location. The tool stops there because placement requires real object/world semantics. It does not pretend to apply a movement command or execute the rest of the opening.

The optional scalar host distinguishes game globals, map globals and per-script locals, requires supplied values, and records variable writes. It implements selected read-only metarule queries. This is useful for controlled probes but is not a model of the complete world.

## Opening cave startup completion

The newer cave-run command replaces the earlier scalar-only blocker experiment with a local map/object model and imported player GCD. It executes bootstrap initialization, exported variables and the map-entry controller path for each original premade character. The controller completes for all three records with no unsupported operation on these paths. The player's initial inventory is produced by script execution, with prototype-backed weapon ammunition initialized. This does not execute every other procedure or the map's object/spatial scripts.

The VM now supports tagged object pointers and resolved identifier constants. The structural world model supplies object lookup, selected prototype data, creation and inventory transfers, player placement and ambient light. Initial skill/trait calculations are reference-derived and require the relevant clock/difficulty context. Night Person preserves the literal reference threshold, including its midnight behavior, rather than applying an assumed day/night schedule.

No full script VM, original-engine runtime trace, high-level decompilation or verified Rust rules implementation is claimed by the current tool.
