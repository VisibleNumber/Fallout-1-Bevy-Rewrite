# Initial rules extracted from pinned reference source

Confidence: reference implementation only. Original-game behavioral comparison is still required. Source commit: 0609bcfd0ec40ff0571d0f57fab2821eb461dc8b. The released playable engine and this source checkout are separately versioned.

## Derived character statistics

Source: reference/fallout1-ce/src/game/stat.cc, stat_recalc_derived (line 398).

The routine writes base derived statistics. Other layers, traits, perks, equipment, conditions and bonuses may modify final displayed values.

| Derived base value | Reference rule |
|---|---|
| Maximum HP | base Strength + 2 * base Endurance + 15 |
| AP | integer Agility / 2 + 5 |
| Armor class | Agility |
| Melee damage | max(Strength - 5, 1) |
| Carry weight | 25 * Strength + 25 |
| Sequence | 2 * Perception |
| Healing rate | max(integer Endurance / 3, 1) |
| Critical chance | Luck |
| Radiation resistance | 2 * Endurance |
| Poison resistance | 5 * Endurance |

HP explicitly uses stat_get_base, whereas several other values use stat_level. Preserve that distinction when porting.

## Attack AP cost

Source: reference/fallout1-ce/src/game/item.cc, item_w_mp_cost (line 1376).

Reload returns 2 immediately. Punch, kick or no weapon starts at 3. Weapon attacks use the primary or secondary prototype AP field. For the player, Fast Shot subtracts 1 inside the weapon branch. Bonus HTH Attacks subtracts 1 for melee/unarmed; Bonus Rate of Fire subtracts 1 for ranged. Aiming adds 1. Final attack cost is at least 1. Reload's early return bypasses these later adjustments.

## Generic percentage checks

Source: reference/fallout1-ce/src/game/roll.cc, roll_check and roll_check_critical (lines 66 and 82).

Compute delta = difficulty - random integer from 1 through 100. Nonnegative delta succeeds. Before the second game day, this helper does not upgrade the result to a critical. Thereafter it makes an additional percentage roll: failure upgrades when roll <= integer(-delta / 10); success upgrades when roll <= integer(delta / 10) + criticalSuccessModifier. This is the generic helper, not a complete account of combat critical handling.

## Next observations

Compare premade character sheets, then attack/reload/aim costs in the cave. Record full input state, game time, traits, perks, weapon prototype and RNG context. Store actual observations separately from these source notes.
