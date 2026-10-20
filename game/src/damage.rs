/// Inputs needed by Fallout 1 CE's ordinary, non-critical damage calculation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DamageProfile {
    pub threshold: i32,
    /// Resistance after `stat_level` clamping and any attacker trait adjustment.
    /// Fallout 1 CE can reach 130 when Finesse adds 30 to a 100 stat value.
    pub resistance: i32,
    /// Retained for compatibility with ammo prototype data. Fallout 1 CE's
    /// ordinary `compute_damage` does not read this field.
    pub ammo_resistance_modifier: i32,
    /// Retained for compatibility with ammo prototype data. Fallout 1 CE's
    /// ordinary `compute_damage` does not read either ammo damage field.
    pub ammo_multiplier: i32,
    /// Retained for compatibility with ammo prototype data. Fallout 1 CE's
    /// ordinary `compute_damage` does not read either ammo damage field.
    pub ammo_divisor: i32,
    /// CE supplies 2 for ordinary attacks; critical-hit processing can supply
    /// other multipliers but is outside this function's scope.
    pub damage_multiplier: i32,
    /// Additive damage before the ordinary multiplier (for example, ranged
    /// damage perk bonus in `compute_damage`).
    pub bonus_damage: i32,
    /// CE combat difficulty percentage: typically 75, 100, or 125.
    pub difficulty_percent: i32,
}

/// Apply Fallout 1 CE's one-round ordinary damage pipeline to an already-rolled
/// weapon damage value. No RNG, critical effect, armor bypass, or multi-round
/// accumulation is performed. Arithmetic follows the source's integer operation
/// order and returns an error instead of relying on signed-overflow behavior.
pub fn ordinary_damage(rolled: i32, profile: &DamageProfile) -> Result<i32, String> {
    if profile.threshold < 0 {
        return Err("Damage threshold cannot be negative".to_string());
    }
    if !(0..=130).contains(&profile.resistance) {
        return Err("Resistance must be between 0 and 130".to_string());
    }
    if profile.damage_multiplier < 0 {
        return Err("Damage multiplier cannot be negative".to_string());
    }
    if profile.bonus_damage < 0 {
        return Err("Bonus damage cannot be negative".to_string());
    }
    if profile.difficulty_percent < 0 {
        return Err("Difficulty percentage cannot be negative".to_string());
    }

    // Reference: combat.cc compute_damage, the per-round sequence beginning
    // with item_w_damage + bonus_ranged_damage. C integer division truncates
    // toward zero, as Rust's signed integer division does.
    let mut damage = rolled
        .checked_add(profile.bonus_damage)
        .ok_or_else(|| "Damage addition overflow".to_string())?;
    damage = damage
        .checked_mul(profile.damage_multiplier)
        .ok_or_else(|| "Damage multiplier overflow".to_string())?
        / 2;
    damage = damage
        .checked_mul(profile.difficulty_percent)
        .ok_or_else(|| "Difficulty scaling overflow".to_string())?
        / 100;
    damage = damage
        .checked_sub(profile.threshold)
        .ok_or_else(|| "Damage threshold subtraction overflow".to_string())?;

    if damage <= 0 {
        return Ok(0);
    }

    // CE does not clamp the resistance after the optional Finesse +30 modifier.
    damage -= damage
        .checked_mul(profile.resistance)
        .ok_or_else(|| "Resistance scaling overflow".to_string())?
        / 100;
    Ok(damage.max(0))
}

#[cfg(test)]
mod tests {
    use super::{DamageProfile, ordinary_damage};

    fn profile() -> DamageProfile {
        DamageProfile {
            threshold: 2,
            resistance: 25,
            ammo_resistance_modifier: 0,
            ammo_multiplier: 1,
            ammo_divisor: 1,
            damage_multiplier: 2,
            bonus_damage: 1,
            difficulty_percent: 100,
        }
    }

    #[test]
    fn applies_ordinary_steps_in_reference_order() {
        // (9 + 1) * 2 / 2 * 100 / 100 - 2 = 8; then 8 - 8*25/100 = 6.
        assert_eq!(ordinary_damage(9, &profile()).unwrap(), 6);
    }

    #[test]
    fn threshold_is_applied_before_resistance_and_damage_floors_at_zero() {
        let mut p = profile();
        p.bonus_damage = 0;
        p.threshold = 3;
        p.resistance = 50;
        assert_eq!(ordinary_damage(10, &p).unwrap(), 4);

        p.threshold = 20;
        assert_eq!(ordinary_damage(10, &p).unwrap(), 0);

        p.threshold = 0;
        p.resistance = 130;
        assert_eq!(ordinary_damage(10, &p).unwrap(), 0);
    }

    #[test]
    fn integer_divisions_keep_the_reference_truncation_order() {
        let mut p = profile();
        p.threshold = 0;
        p.resistance = 50;
        p.bonus_damage = 0;
        p.difficulty_percent = 75;
        // 3*2/2=3, then 3*75/100=2, then resistance removes 1.
        assert_eq!(ordinary_damage(3, &p).unwrap(), 1);

        // Difficulty division occurs before threshold; this truncates to zero.
        assert_eq!(ordinary_damage(1, &p).unwrap(), 0);
    }

    #[test]
    fn fallout_one_ignores_ammunition_damage_modifiers_in_this_pipeline() {
        let baseline = profile();
        let expected = ordinary_damage(9, &baseline).unwrap();
        let mut ammo = baseline;
        ammo.ammo_resistance_modifier = -100;
        ammo.ammo_multiplier = 3;
        ammo.ammo_divisor = 0;
        assert_eq!(ordinary_damage(9, &ammo).unwrap(), expected);
    }

    #[test]
    fn reports_invalid_profiles_and_overflow() {
        let mut p = profile();
        p.resistance = 131;
        assert!(
            ordinary_damage(10, &p)
                .unwrap_err()
                .contains("between 0 and 130")
        );
        p = profile();
        p.damage_multiplier = i32::MAX;
        assert!(ordinary_damage(2, &p).unwrap_err().contains("overflow"));
    }
}
