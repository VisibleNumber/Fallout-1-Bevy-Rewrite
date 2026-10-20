use serde::{Deserialize, Serialize};

const TILE_COUNT: usize = 40_000;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct SaveGame {
    pub schema: u32,
    pub tile: i32,
    pub facing: usize,
    pub equipped: Option<i32>,
}

impl SaveGame {
    /// Validate the save against the current cave and known weapon IDs.
    pub fn validate(&self, walkable: &[bool], weapon_ids: &[i32]) -> Result<(), String> {
        if self.schema != 1 {
            return Err(format!("Unsupported save schema {}", self.schema));
        }
        if walkable.len() != TILE_COUNT {
            return Err(format!(
                "Expected {TILE_COUNT} walkability entries, got {}",
                walkable.len()
            ));
        }
        if !(0..TILE_COUNT as i32).contains(&self.tile) {
            return Err(format!("Tile {} is out of bounds", self.tile));
        }
        if !walkable[self.tile as usize] {
            return Err(format!("Tile {} is not walkable", self.tile));
        }
        if self.facing >= 6 {
            return Err(format!("Facing {} is out of bounds", self.facing));
        }
        if let Some(id) = self.equipped {
            if !weapon_ids.contains(&id) {
                return Err(format!("Unknown equipped weapon ID {id}"));
            }
        }
        Ok(())
    }

    /// Encode a structurally valid save record as JSON text.
    pub fn encode(&self) -> Result<String, String> {
        if self.schema != 1 {
            return Err(format!("Unsupported save schema {}", self.schema));
        }
        if !(0..TILE_COUNT as i32).contains(&self.tile) {
            return Err(format!("Tile {} is out of bounds", self.tile));
        }
        if self.facing >= 6 {
            return Err(format!("Facing {} is out of bounds", self.facing));
        }
        if self.equipped.is_some_and(|id| id < 0) {
            return Err("Equipped weapon ID must be nonnegative".to_string());
        }
        serde_json::to_string(self).map_err(|error| error.to_string())
    }

    /// Decode and validate JSON text against the current map and weapon list.
    pub fn decode(text: &str, walkable: &[bool], weapon_ids: &[i32]) -> Result<Self, String> {
        let save: Self = serde_json::from_str(text).map_err(|error| error.to_string())?;
        save.validate(walkable, weapon_ids)?;
        Ok(save)
    }
}

/// Combat-only checkpoint data. Position and combat stats are restored while
/// static world data and weapon capacities continue to come from the game.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CombatSnapshot {
    pub player: crate::combat::Combatant,
    pub rats: Vec<crate::combat::Combatant>,
    pub active: bool,
    /// `(weapon_id, current_ammo)` entries for ranged weapons.
    pub ammo: Vec<(i32, i32)>,
}

impl CombatSnapshot {
    /// Check that a checkpoint can safely be applied to the starting encounter.
    ///
    /// Tiles are checked against map bounds only, since combatants may move to
    /// occupied or initially blocked hexes during a fight.
    pub fn validate(
        &self,
        initial_player: &crate::combat::Combatant,
        initial_rats: &[crate::combat::Combatant],
        ammo_caps: &[(i32, i32)],
    ) -> Result<(), String> {
        validate_combat_snapshot_unit(&self.player, initial_player, "player")?;
        if self.rats.len() != initial_rats.len() {
            return Err("Combat snapshot rat count does not match encounter".to_string());
        }

        let mut initial_by_id = std::collections::HashMap::new();
        initial_by_id.insert(initial_player.id, initial_player);
        for rat in initial_rats {
            if initial_by_id.insert(rat.id, rat).is_some() {
                return Err("Duplicate combatant ID in initial encounter".to_string());
            }
        }
        if self.player.id != initial_player.id {
            return Err("Combat snapshot player ID does not match encounter".to_string());
        }

        let mut seen_ids = std::collections::HashSet::new();
        seen_ids.insert(self.player.id);
        for rat in &self.rats {
            if !seen_ids.insert(rat.id) {
                return Err("Duplicate combatant ID in snapshot".to_string());
            }
            let initial = initial_by_id
                .get(&rat.id)
                .filter(|initial| initial.id != initial_player.id)
                .ok_or_else(|| format!("Unknown rat ID {} in combat snapshot", rat.id))?;
            validate_combat_snapshot_unit(rat, initial, "rat")?;
        }

        let mut expected_caps = std::collections::HashMap::new();
        for &(id, capacity) in ammo_caps {
            if id < 0 || capacity < 0 {
                return Err("Invalid ranged weapon ID or ammo capacity".to_string());
            }
            if expected_caps.insert(id, capacity).is_some() {
                return Err(format!("Duplicate ranged weapon ID {id}"));
            }
        }
        if self.ammo.len() != expected_caps.len() {
            return Err("Combat snapshot ammo count does not match weapons".to_string());
        }
        let mut seen_ammo = std::collections::HashSet::new();
        for &(id, quantity) in &self.ammo {
            if !seen_ammo.insert(id) {
                return Err(format!("Duplicate ammo entry for weapon {id}"));
            }
            let capacity = expected_caps
                .get(&id)
                .ok_or_else(|| format!("Unknown ammo weapon ID {id}"))?;
            if quantity < 0 || quantity > *capacity {
                return Err(format!("Invalid ammo quantity for weapon {id}"));
            }
        }
        Ok(())
    }
}

fn validate_combat_snapshot_unit(
    current: &crate::combat::Combatant,
    initial: &crate::combat::Combatant,
    role: &str,
) -> Result<(), String> {
    if current.id != initial.id {
        return Err(format!(
            "Combat snapshot {role} ID does not match encounter"
        ));
    }
    if !(0..TILE_COUNT as i32).contains(&current.tile) {
        return Err(format!("Combat snapshot {role} tile is out of bounds"));
    }
    if current.max_hp != initial.max_hp
        || current.max_ap != initial.max_ap
        || current.armor_class != initial.armor_class
    {
        return Err(format!(
            "Combat snapshot {role} maximum stats were modified"
        ));
    }
    if current.max_hp < 1 || current.hp < 0 || current.hp > current.max_hp {
        return Err(format!("Combat snapshot {role} hit points are invalid"));
    }
    if current.max_ap < 0 || current.ap < 0 || current.ap > current.max_ap {
        return Err(format!("Combat snapshot {role} action points are invalid"));
    }
    if current.armor_class < 0 {
        return Err(format!("Combat snapshot {role} armor class is invalid"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::Combatant;

    fn world() -> Vec<bool> {
        let mut walkable = vec![false; TILE_COUNT];
        walkable[1234] = true;
        walkable[2345] = true;
        walkable
    }

    fn valid_save() -> SaveGame {
        SaveGame {
            schema: 1,
            tile: 1234,
            facing: 3,
            equipped: Some(42),
        }
    }

    fn combatant(id: i32, tile: i32) -> Combatant {
        Combatant {
            id,
            tile,
            hp: 20,
            max_hp: 20,
            ap: 6,
            max_ap: 8,
            armor_class: 10,
        }
    }

    fn valid_combat_snapshot() -> (CombatSnapshot, Combatant, Vec<Combatant>, Vec<(i32, i32)>) {
        let player = combatant(1, 100);
        let rats = vec![combatant(2, 200), combatant(3, 300)];
        let caps = vec![(40, 6), (41, 12)];
        let mut moved_rat = rats[0].clone();
        moved_rat.tile = 201;
        moved_rat.hp = 7;
        let mut player_state = player.clone();
        player_state.tile = 101;
        player_state.ap = 2;
        let snapshot = CombatSnapshot {
            player: player_state,
            rats: vec![moved_rat, rats[1].clone()],
            active: true,
            ammo: vec![(40, 3), (41, 0)],
        };
        (snapshot, player, rats, caps)
    }

    #[test]
    fn encode_decode_round_trip_validated_save() {
        let walkable = world();
        let weapons = [42, 99];
        let save = valid_save();
        save.validate(&walkable, &weapons).unwrap();
        let text = save.encode().unwrap();
        assert!(text.contains("\"schema\":1"));
        assert_eq!(SaveGame::decode(&text, &walkable, &weapons).unwrap(), save);
    }

    #[test]
    fn validation_rejects_wrong_schema_and_bad_map_tiles() {
        let walkable = world();
        let weapons = [42];
        let mut save = valid_save();
        save.schema = 2;
        assert!(
            save.validate(&walkable, &weapons)
                .unwrap_err()
                .contains("schema")
        );
        save.schema = 1;
        save.tile = -1;
        assert!(
            save.validate(&walkable, &weapons)
                .unwrap_err()
                .contains("bounds")
        );
        save.tile = 40_000;
        assert!(
            save.validate(&walkable, &weapons)
                .unwrap_err()
                .contains("bounds")
        );
        save.tile = 1235;
        assert!(
            save.validate(&walkable, &weapons)
                .unwrap_err()
                .contains("walkable")
        );
        assert!(
            save.validate(&[], &weapons)
                .unwrap_err()
                .contains("walkability")
        );
    }

    #[test]
    fn validation_rejects_invalid_facing_and_unknown_weapon() {
        let walkable = world();
        let mut save = valid_save();
        save.facing = 6;
        assert!(
            save.validate(&walkable, &[42])
                .unwrap_err()
                .contains("Facing")
        );
        save.facing = 0;
        save.equipped = Some(77);
        assert!(
            save.validate(&walkable, &[42])
                .unwrap_err()
                .contains("weapon")
        );
    }

    #[test]
    fn decode_rejects_bad_json_and_invalid_records() {
        let walkable = world();
        let weapons = [42];
        assert!(SaveGame::decode("not json", &walkable, &weapons).is_err());
        let wrong_schema = r#"{"schema":2,"tile":1234,"facing":0,"equipped":null}"#;
        assert!(
            SaveGame::decode(wrong_schema, &walkable, &weapons)
                .unwrap_err()
                .contains("schema")
        );
        let unknown_weapon = r#"{"schema":1,"tile":1234,"facing":0,"equipped":77}"#;
        assert!(
            SaveGame::decode(unknown_weapon, &walkable, &weapons)
                .unwrap_err()
                .contains("weapon")
        );
    }

    #[test]
    fn encode_rejects_structural_values_outside_supported_ranges() {
        let mut save = valid_save();
        save.schema = 9;
        assert!(save.encode().is_err());
        save.schema = 1;
        save.facing = 9;
        assert!(save.encode().is_err());
        save.facing = 0;
        save.equipped = Some(-1);
        assert!(save.encode().is_err());
    }

    #[test]
    fn combat_snapshot_validates_and_round_trips() {
        let (snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.validate(&player, &rats, &caps).unwrap();
        let text = serde_json::to_string(&snapshot).unwrap();
        let decoded: CombatSnapshot = serde_json::from_str(&text).unwrap();
        assert_eq!(decoded, snapshot);
        decoded.validate(&player, &rats, &caps).unwrap();
    }

    #[test]
    fn combat_snapshot_allows_in_bounds_tiles_even_if_blocked() {
        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        // This validator has no walkability input: a moved unit may occupy a
        // tile that was blocked in the original static map.
        snapshot.rats[0].tile = 39_999;
        snapshot.validate(&player, &rats, &caps).unwrap();
    }

    #[test]
    fn combat_snapshot_rejects_mismatched_or_duplicate_combatants() {
        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.rats.pop();
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("count")
        );

        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.rats[0].id = 99;
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("rat ID")
        );

        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.rats[1].id = snapshot.rats[0].id;
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("Duplicate")
        );

        let (snapshot, player, mut rats, caps) = valid_combat_snapshot();
        rats[1].id = rats[0].id;
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("Duplicate")
        );
    }

    #[test]
    fn combat_snapshot_rejects_invalid_state_and_modified_maximum_stats() {
        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.player.tile = 40_000;
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("tile")
        );

        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.player.hp = -1;
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("hit points")
        );

        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.rats[0].ap = snapshot.rats[0].max_ap + 1;
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("action points")
        );

        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.rats[0].max_hp += 1;
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("maximum stats")
        );

        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.player.max_ap -= 1;
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("maximum stats")
        );

        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.player.armor_class += 1;
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("maximum stats")
        );
    }

    #[test]
    fn combat_snapshot_checks_complete_unique_ammo_and_capacities() {
        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.ammo[0].1 = -1;
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("ammo quantity")
        );

        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.ammo[0].1 = 7;
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("ammo quantity")
        );

        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.ammo[1].0 = 40;
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("Duplicate ammo")
        );

        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.ammo.pop();
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("ammo count")
        );

        let (mut snapshot, player, rats, caps) = valid_combat_snapshot();
        snapshot.ammo[0].0 = 999;
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("Unknown ammo")
        );

        let (snapshot, player, rats, mut caps) = valid_combat_snapshot();
        caps[1].0 = caps[0].0;
        assert!(
            snapshot
                .validate(&player, &rats, &caps)
                .unwrap_err()
                .contains("Duplicate ranged weapon")
        );
    }
}
