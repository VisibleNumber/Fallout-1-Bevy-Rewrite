use serde::{Deserialize, Serialize};

/// Combat state for a small deterministic turn-based prototype.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Combatant {
    pub id: i32,
    pub tile: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub ap: i32,
    pub max_ap: i32,
    pub armor_class: i32,
}

/// The subset of weapon data needed by this combat model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Weapon {
    pub min_damage: i32,
    pub max_damage: i32,
    pub range: i32,
    pub ap_cost: i32,
    /// Base skill percentage used by the simplified hit calculation.
    pub skill: i32,
    pub ammo: i32,
    pub capacity: i32,
    pub ranged: bool,
    /// Attacker perception used by the reference ranged-distance modifier.
    pub perception: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackOutcome {
    pub hit: bool,
    /// Damage actually removed from the target's current hit points.
    pub damage: i32,
    pub killed: bool,
}

fn validate_combatant(combatant: &Combatant, role: &str) -> Result<(), String> {
    if combatant.max_hp < 1 || combatant.hp < 0 || combatant.hp > combatant.max_hp {
        return Err(format!("Invalid {role} hit points"));
    }
    if combatant.max_ap < 0 || combatant.ap < 0 || combatant.ap > combatant.max_ap {
        return Err(format!("Invalid {role} action points"));
    }
    if combatant.armor_class < 0 {
        return Err(format!("Invalid {role} armor class"));
    }
    Ok(())
}

/// Resolve one deterministic attack.
///
/// The ordinary ranged accuracy modifier follows Fallout CE's basic
/// `-4 * (distance - 2 * perception)` slice and the 95% upper cap. The model
/// omits visibility, hit locations, traits, perks, criticals, difficulty,
/// damage resistance/threshold, burst attacks, and combat animation. `roll`
/// and `damage_roll` are supplied by the caller, so this function uses no RNG.
/// Melee attacks have no distance accuracy modifier in this slice. A weapon
/// with zero capacity is treated as not using tracked ammunition.
pub fn attack(
    attacker: &mut Combatant,
    target: &mut Combatant,
    weapon: &mut Weapon,
    distance: i32,
    roll: i32,
    damage_roll: i32,
) -> Result<AttackOutcome, String> {
    validate_combatant(attacker, "attacker")?;
    validate_combatant(target, "target")?;
    if attacker.hp == 0 {
        return Err("Attacker is dead".to_string());
    }
    if target.hp == 0 {
        return Err("Target is already dead".to_string());
    }
    if distance < 0 {
        return Err("Invalid attack distance".to_string());
    }
    if weapon.range < 0 || distance > weapon.range {
        return Err("Target is out of weapon range".to_string());
    }
    if weapon.ap_cost < 1 {
        return Err("Invalid weapon action point cost".to_string());
    }
    if attacker.ap < weapon.ap_cost {
        return Err("Not enough action points".to_string());
    }
    if weapon.skill < 0 || weapon.skill > 200 {
        return Err("Invalid weapon skill".to_string());
    }
    if weapon.min_damage < 0 || weapon.max_damage < weapon.min_damage {
        return Err("Invalid weapon damage range".to_string());
    }
    if weapon.capacity < 0 || weapon.ammo < 0 || weapon.ammo > weapon.capacity {
        return Err("Invalid weapon ammunition".to_string());
    }
    if weapon.capacity > 0 && weapon.ammo == 0 {
        return Err("Weapon is out of ammunition".to_string());
    }
    if weapon.ranged && weapon.perception < 0 {
        return Err("Invalid attacker perception".to_string());
    }
    if !(1..=100).contains(&roll) {
        return Err("Attack roll must be between 1 and 100".to_string());
    }
    if !(weapon.min_damage..=weapon.max_damage).contains(&damage_roll) {
        return Err("Damage roll is outside weapon damage range".to_string());
    }

    let distance_modifier = if weapon.ranged {
        let perception = i64::from(weapon.perception);
        let adjusted_distance = (i64::from(distance) - 2 * perception).max(-2 * perception);
        -4 * adjusted_distance
    } else {
        0_i64
    };
    let hit_chance =
        (i64::from(weapon.skill) - i64::from(target.armor_class) + distance_modifier).min(95);
    let hit = i64::from(roll) <= hit_chance;
    let damage = if hit { damage_roll.min(target.hp) } else { 0 };

    // Mutations happen only after every input and precondition has passed.
    attacker.ap -= weapon.ap_cost;
    if weapon.capacity > 0 {
        weapon.ammo -= 1;
    }
    target.hp -= damage;

    Ok(AttackOutcome {
        hit,
        damage,
        killed: target.hp == 0,
    })
}

/// Refresh action points at the start of the next player turn.
pub fn end_turn(player: &mut Combatant) {
    player.ap = player.max_ap;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combatant(id: i32, hp: i32, ap: i32, armor_class: i32) -> Combatant {
        Combatant {
            id,
            tile: 1000 + id,
            hp,
            max_hp: hp.max(1),
            ap,
            max_ap: ap,
            armor_class,
        }
    }

    fn rifle() -> Weapon {
        Weapon {
            min_damage: 4,
            max_damage: 9,
            range: 8,
            ap_cost: 4,
            skill: 75,
            ammo: 5,
            capacity: 5,
            ranged: true,
            perception: 5,
        }
    }

    #[test]
    fn ranged_hit_spends_ap_and_ammo_and_applies_damage() {
        let mut attacker = combatant(1, 30, 8, 0);
        let mut target = combatant(2, 20, 6, 10);
        let mut weapon = rifle();
        let outcome = attack(&mut attacker, &mut target, &mut weapon, 4, 60, 7).unwrap();
        assert_eq!(
            outcome,
            AttackOutcome {
                hit: true,
                damage: 7,
                killed: false
            }
        );
        assert_eq!(attacker.ap, 4);
        assert_eq!(target.hp, 13);
        assert_eq!(weapon.ammo, 4);
    }

    #[test]
    fn ranged_accuracy_uses_perception_and_has_ninety_five_cap() {
        let mut attacker = combatant(1, 30, 8, 0);
        let mut target = combatant(2, 20, 6, 10);
        let mut weapon = rifle();
        // 75 skill - 10 armor - 4*(8 - 2*5) = 73%.
        let outcome = attack(&mut attacker, &mut target, &mut weapon, 8, 73, 4).unwrap();
        assert!(outcome.hit);
        assert_eq!(outcome.damage, 4);
        let mut attacker = combatant(1, 30, 8, 0);
        let mut target = combatant(2, 20, 6, 10);
        let mut weapon = rifle();
        let outcome = attack(&mut attacker, &mut target, &mut weapon, 8, 74, 4).unwrap();
        assert!(!outcome.hit);

        let mut attacker = combatant(1, 30, 8, 0);
        let mut target = combatant(2, 20, 6, 0);
        let mut weapon = rifle();
        let outcome = attack(&mut attacker, &mut target, &mut weapon, 0, 95, 4).unwrap();
        assert!(outcome.hit, "chance caps at 95 rather than reaching 100");
        let mut attacker = combatant(1, 30, 8, 0);
        let mut target = combatant(2, 20, 6, 0);
        let mut weapon = rifle();
        let outcome = attack(&mut attacker, &mut target, &mut weapon, 0, 96, 4).unwrap();
        assert!(!outcome.hit);
    }

    #[test]
    fn fallout_skill_values_above_one_hundred_are_valid_but_capped() {
        let mut attacker = combatant(1, 30, 8, 0);
        let mut target = combatant(2, 20, 6, 0);
        let mut weapon = rifle();
        weapon.skill = 200;
        let outcome = attack(&mut attacker, &mut target, &mut weapon, 0, 95, 4).unwrap();
        assert!(outcome.hit);

        let mut attacker = combatant(1, 30, 8, 0);
        let mut target = combatant(2, 20, 6, 0);
        let mut weapon = rifle();
        weapon.skill = 101;
        let outcome = attack(&mut attacker, &mut target, &mut weapon, 0, 96, 4).unwrap();
        assert!(!outcome.hit);
    }

    #[test]
    fn misses_consume_attack_resources_without_damaging_target() {
        let mut attacker = combatant(1, 30, 8, 0);
        let mut target = combatant(2, 20, 6, 10);
        let mut weapon = rifle();
        let outcome = attack(&mut attacker, &mut target, &mut weapon, 8, 86, 9).unwrap();
        assert_eq!(
            outcome,
            AttackOutcome {
                hit: false,
                damage: 0,
                killed: false
            }
        );
        assert_eq!(attacker.ap, 4);
        assert_eq!(target.hp, 20);
        assert_eq!(weapon.ammo, 4);
    }

    #[test]
    fn melee_attack_skips_ammo_and_distance_accuracy_modifier() {
        let mut attacker = combatant(1, 20, 6, 0);
        let mut target = combatant(2, 12, 6, 20);
        let mut weapon = Weapon {
            min_damage: 2,
            max_damage: 6,
            range: 1,
            ap_cost: 3,
            skill: 50,
            ammo: 0,
            capacity: 0,
            ranged: false,
            perception: 0,
        };
        let outcome = attack(&mut attacker, &mut target, &mut weapon, 1, 30, 5).unwrap();
        assert!(outcome.hit);
        assert_eq!(outcome.damage, 5);
        assert_eq!(attacker.ap, 3);
        assert_eq!(weapon.ammo, 0);
    }

    #[test]
    fn lethal_damage_clamps_to_remaining_hit_points() {
        let mut attacker = combatant(1, 20, 6, 0);
        let mut target = combatant(2, 3, 6, 0);
        let mut weapon = rifle();
        let outcome = attack(&mut attacker, &mut target, &mut weapon, 2, 1, 9).unwrap();
        assert_eq!(outcome.damage, 3);
        assert!(outcome.killed);
        assert_eq!(target.hp, 0);
    }

    #[test]
    fn invalid_preconditions_leave_all_combat_state_unchanged() {
        let cases = [
            ("range", 9, 1, 4, 5),
            ("ap", 2, 1, 4, 5),
            ("ammo", 2, 8, 0, 5),
        ];
        for (case, distance, ap, ammo, roll) in cases {
            let mut attacker = combatant(1, 20, ap, 0);
            let mut target = combatant(2, 20, 6, 0);
            let mut weapon = rifle();
            weapon.ammo = ammo;
            let before = (attacker.clone(), target.clone(), weapon.clone());
            assert!(
                attack(&mut attacker, &mut target, &mut weapon, distance, roll, 5).is_err(),
                "{case}"
            );
            assert_eq!((attacker, target, weapon), before, "{case} mutated state");
        }

        let mut attacker = combatant(1, 20, 8, 0);
        let mut target = combatant(2, 20, 6, 0);
        let mut weapon = rifle();
        let before = (attacker.clone(), target.clone(), weapon.clone());
        assert!(attack(&mut attacker, &mut target, &mut weapon, 2, 101, 5).is_err());
        assert_eq!((attacker, target, weapon), before);
    }

    #[test]
    fn dead_combatants_cannot_attack_or_be_attacked() {
        let mut attacker = combatant(1, 0, 8, 0);
        let mut target = combatant(2, 20, 6, 0);
        let mut weapon = rifle();
        assert!(
            attack(&mut attacker, &mut target, &mut weapon, 1, 1, 5)
                .unwrap_err()
                .contains("Attacker")
        );
        attacker.hp = 20;
        attacker.max_hp = 20;
        target.hp = 0;
        assert!(
            attack(&mut attacker, &mut target, &mut weapon, 1, 1, 5)
                .unwrap_err()
                .contains("Target")
        );
    }

    #[test]
    fn turn_refresh_restores_action_points() {
        let mut player = combatant(1, 20, 2, 0);
        player.max_ap = 7;
        end_turn(&mut player);
        assert_eq!(player.ap, 7);
    }
}
