# Fallout 1 Bevy Rewrite

Research tooling and planning for a future Fallout 1 recreation in Bevy, with a full 3D presentation. No playable Bevy port is included yet.

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

## Publishing policy

Commit only tool source, synthetic tests and authored documentation. Keep generated reports, extracted text, textures, maps, scripts, prototypes, archives, audio, cinematics, executables and saves out of Git. `.gitignore` provides additional protection; review staged files before every push.
