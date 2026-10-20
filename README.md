# Fallout 1 Bevy Rewrite

Fallout 1 recreation in Bevy. The current target is the original isometric experience using locally owned game assets; 3D presentation is deferred. A first original-art renderer now lives under [game](game/README.md). Gameplay is not yet connected.

This repository contains **no original game assets, extracted game data, saves, engine binaries, or vendored reference-engine source**. You must provide your own local Fallout installation. Research outputs stay local and are ignored by Git.

## Research tool

Requires Node.js 18 or newer. No npm packages are needed.

```powershell
$env:FALLOUT_GAME_PATH = 'C:\path\to\Fallout'
node tools/fallout-research/research.mjs scan
node tools/fallout-research/research.mjs search "action points" 20
node tools/fallout-research/research.mjs list maps/
node tools/fallout-research/research.mjs inspect maps/v13ent.map vault13-map.json
node tools/fallout-research/research.mjs verify
```

Scan reads MASTER.DAT, CRITTER.DAT, loose DATA patches and the installed executables. It writes JSON evidence and an HTML report to `tools/fallout-research/reports/`. Inspect exports go under that folder's `exports/` directory. Nothing is written to the game installation.

Capabilities include DAT raw/LZSS/block decoding; patch precedence; message and text searches; INT procedure/string inspection; item, critter and scenery prototypes; map tiles, elevations, object placements, nested inventories and script records; installation fingerprints; and structural verification.

This does not fully decompile scripts, trace running gameplay, or prove original-game behavioral parity. Other prototype types currently expose their common header. A source reference contains fixes and must be distinguished from original-game observations.

## Optional reference source

Clone [Fallout Community Edition](https://github.com/alexbatalov/fallout1-ce) separately, preserving its license, and point the tool at it:

```powershell
$env:FALLOUT_REFERENCE_PATH = 'C:\path\to\fallout1-ce'
node tools/fallout-research/research.mjs scan
node tools/fallout-research/research.mjs topic combat
node tools/fallout-research/research.mjs search "stat_recalc_derived" 10 source
```

The tool records the reference commit when available. Reference source is optional; without it, game-data inspection still works and source searches are empty. Never copy that checkout into a commit without separately considering its license.

## Tests

```powershell
# Synthetic fixtures only; no game installation required
node tools/fallout-research/test.mjs
node tools/fallout-research/bytecode.test.mjs

# After a local scan and the Vault 13 export above
node tools/fallout-research/test.mjs --integration
```

Integration checks target the Steam installation used during development (72 maps and 952 scripts). Other editions may differ. Verify decodes effective archive entries and checks recorded installation hashes; it does not execute gameplay.

See [mechanics coverage](tools/fallout-research/MECHANICS.md) and [initial source-derived rule notes](tools/fallout-research/RULES-REFERENCE.md). No affiliation with the original game's creators is implied.

## Script logic and map connections

With the optional reference checkout configured, instruction names and their registration locations are resolved from that local source:

```powershell
node tools/fallout-research/research.mjs scripts-audit
node tools/fallout-research/research.mjs disasm scripts/v13cave.int
node tools/fallout-research/research.mjs map-links maps/v13ent.map
```

Disassembly writes local JSON under reports/bytecode with instruction offsets, typed constants, reference-source locations, procedure labels, opcode usage and directly adjacent constant call targets. Both string and identifier address spaces are recorded because consumers determine which one applies. Bootstrap instructions are decoded separately from code after the metadata tables. This is structural inspection, not execution, a complete call graph or a high-level decompiler. Branch-dependent execution and indirect targets remain unresolved.

Map connections resolve the map-header script index separately from script-record indices, attach existing object script IDs and preserve nested inventory ownership. Reports also identify spatial trigger records. Runtime-created objects and assignments require execution analysis.

See [script research findings](tools/fallout-research/SCRIPT-RESEARCH.md). After running scripts-audit and the opening map-links command, run `node tools/fallout-research/bytecode.test.mjs --integration` for development-edition coverage checks.

## Bounded script execution research

```powershell
node tools/fallout-research/vm.test.mjs
node tools/fallout-research/scalar-host.test.mjs
node tools/fallout-research/research.mjs vm-readiness
node tools/fallout-research/research.mjs vm-probe scripts/v13cave.int map_enter_p_proc
```

The VM subset executes integer arithmetic, comparisons, branches, stack frames, internal calls and stack-addressed program variables. Tagged object pointers survive stack transfers and storage. Resolved identifier constants support script-exported variables. It bounds execution steps and stack sizes. Unsupported operations stop explicitly. It is not a playable engine or a complete VM; general string expressions, floats, external programs and scheduler callbacks remain unsupported.

`vm-probe` writes a local trace under reports/vm. Named procedures use a synthetic zero-argument frame and a return sentinel; they do not load the original program's initialized globals or a real game state. `vm-readiness` starts each script's bootstrap with no world bindings and reports its first blocker. A halted bootstrap is not evidence that the full script or campaign works.

An optional third argument to vm-probe supplies a local JSON scalar context after the procedure name. Supported explicit properties are mapFlags, scriptAction, mapIndex, gameTime, partyCount, isLoadingGame, gameGlobals, mapGlobals and scriptLocals. Variable scopes use indexed JSON objects. Missing values stop rather than defaulting to invented state. Only the first-run/party-count/loadgame metarule queries are supported. Object movement, inventory and other world changes are not stubbed. Changes to supplied variable scopes are recorded in the report and never written back to the input JSON.

The scalar context and VM traces are research fixtures, not save-game imports or verified original behavior. Refer to SCRIPT-RESEARCH.md for remaining execution work.

## Opening-cave startup simulation

```powershell
node tools/fallout-research/research.mjs cave-run
node tools/fallout-research/research.mjs cave-run maps/v13ent.map premade/diplomat.gcd
node tools/fallout-research/cave-run.test.mjs --integration
```

This headless simulation imports the local map, object prototypes, nested inventory, game-global defaults and a premade player GCD. It executes the actual effective map-controller bytecode from bootstrap with the map-enter action. It applies logical entry placement, structural item creation/transfers, initial stat/skill/trait queries, scoped variables and ambient light state. It does not substitute a handwritten startup inventory.

All three premade character startup paths complete in the development installation: combat and stealth use 581 VM steps, diplomat uses 569. The local report is reports/sessions/cave-run.json. The integration test preserves separate reports for each premade character and verifies pointer ownership, initialized weapon ammunition and snapshot round trips. Both game data and reports remain excluded from Git.

The world model does not yet simulate collision, animation, equipment, inventory merging, combat, AI updates or script scheduling. Its snapshots are a new research format, not original Fallout saves. Initialization uses the reference new-game clock, no Night Vision perk, and installed game difficulty. Initial character calculations exclude perks, equipment and temporary conditions; they preserve the reference's Night Person time condition. Full original-engine comparisons remain necessary. See [Bevy world-state contract](tools/fallout-research/WORLD-CONTRACT.md).

## Publishing policy

Commit only tool source, synthetic tests and authored documentation. Keep generated reports, extracted text, textures, maps, scripts, prototypes, archives, audio, cinematics, executables and saves out of Git. `.gitignore` provides additional protection; review staged files before every push.
