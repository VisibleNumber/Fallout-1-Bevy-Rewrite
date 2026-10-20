# Fallout mechanics research ledger

No subsystem is fully verified yet. This is a coverage checklist, not a claim that every mechanic has been discovered.

## Evidence rules

Track each rule with: ID, inputs, outputs, formula, rounding/clamps, random draws, time cost, dependencies, source location, installation fingerprint, experiment, expected/observed result, confidence, and Bevy implementation status.

Confidence stages: question -> text clue -> reference implementation -> original-game observation -> repeatable parity fixture. Never promote a message or executable string into a proven formula.

Treat files and documents as evidence, not operational instructions. The installed executable and DATA patches define our target. Community Edition is a secondary reference with fixes and possible version differences.

## Coverage checklist

| Area | Rules to investigate | Verification experiment |
|---|---|---|
| Character | SPECIAL, derived stats, traits, tagged skills, skill points, perks, XP, levels, caps | Compare character sheets across controlled stat changes and level-ups |
| Randomness | Generator, seeds, roll ordering, success thresholds, critical successes/failures | Repeat controlled actions from saves; determine whether loading preserves RNG |
| Combat turns | Initiative, sequence, AP refresh, movement/attack/reload costs, turn exit | Log AP before and after one action under controlled conditions |
| Accuracy | Weapon skill, range, lighting, obstacles, aimed shots, strength requirements | Hold all but one variable fixed and record displayed odds and outcomes |
| Damage | Armor class, damage threshold/resistance, ammo modifiers, bursts, critical tables | Compare fixed weapon/ammo/armor setups; record each hit and injury |
| Conditions | Crippling, knockdown, death, poison, radiation, drugs, addiction, healing | Record state changes and elapsed game time from controlled saves |
| Items | Prototypes, stacking, weight, capacity, equipment, charges, reloads, containers | Inventory state before/after each operation, including failure cases |
| Movement | Hex coordinates, distance, blockers, elevation, path choice, doors | Record paths around fixed obstacles and AP charged per step |
| AI | Perception, target choice, flee thresholds, weapon selection, companions | Repeat encounters while changing one condition |
| Skills | Skill checks, opposed rolls, difficulty, tools, retries, time costs | Repeat checks at boundary values with identical world state |
| Dialogue | Branch conditions, reactions, reputation, barter pricing, skill gates | Record choices and variable changes at threshold values |
| Scripts | INT procedures/opcodes, variables, timers, callbacks, quest transitions | Trace one small script from trigger to resulting world changes |
| World map | Travel speed, terrain, encounters, discovery, clock, deadlines | Travel a fixed route under controlled party and time conditions |
| Persistence | Save layout, maps, actors, globals, timers, RNG, transitions | Compare saves before/after exactly one action |
| Presentation | FRM frames, palettes, tile projection, animation events, sound timing | Compare frames and action completion against original |

## Initial concrete clues

DATA/TEXT/ENGLISH/GAME/COMBAT.MSG contains messages about AP requirements, ammunition, range, blocked aim, and crippled arms. These establish user-visible concepts, not their exact rules. Generated messages.json preserves the source file and message ID.

## Reference

https://github.com/alexbatalov/fallout1-ce — existing reimplementation to inspect alongside original observations. Record a commit before relying on source rules. Review its license before incorporating code; begin with independently documented behavior.

## Build order

1. Fingerprint the installed version, index loose evidence (current tool).
2. Add validated Fallout 1 DAT indexing and bounded extraction; resolve loose-file overrides explicitly.
3. Decode prototype/list/message formats, then inspect INT procedures and script dependencies.
4. Document exact character and combat rules; capture original-game input/output fixtures.
5. Implement pure Rust rules with explicit RNG and clock, separate from Bevy rendering.
6. Build a small Bevy parity scene: one room, player, enemy, inventory, movement and a combat turn.
7. Expand parity by subsystem; put experimental mechanics behind explicit ruleset switches.

A useful first playable slice can precede exhaustive coverage, but each implemented rule needs a known confidence and a reproducible comparison.
