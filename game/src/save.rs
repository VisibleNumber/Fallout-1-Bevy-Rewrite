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

#[cfg(test)]
mod tests {
    use super::*;

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
}
