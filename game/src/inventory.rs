use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Item {
    pub id: i32,
    pub pid: i32,
    pub name: String,
    pub description: String,
    pub file: String,
    pub quantity: i32,
    pub weight: i32,
    pub item_type: i32,
    pub ammo_quantity: i32,
    pub ammo_capacity: i32,
    #[serde(default)]
    pub caliber: i32,
    #[serde(default)]
    pub ammo_pid: i32,
    #[serde(default)]
    pub ammo_pack_size: i32,
    #[serde(default)]
    pub heal_min: i32,
    #[serde(default)]
    pub heal_max: i32,
}

/// A small inventory model for the current game prototype.
///
/// It tracks item records, selection, equipped weapon identity, carried
/// weight, basic ammunition transfer, and healing item use. It does not model
/// drops, combat, or general item stacking rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventory {
    pub items: Vec<Item>,
    pub selected: usize,
    pub equipped: Option<i32>,
}

impl Inventory {
    pub fn new(items: Vec<Item>) -> Result<Self, String> {
        for item in &items {
            if item.quantity < 0 {
                return Err(format!("Item {} has invalid quantity", item.id));
            }
            if item.weight < 0 {
                return Err(format!("Item {} has invalid weight", item.id));
            }
        }
        Ok(Self {
            items,
            selected: 0,
            equipped: None,
        })
    }

    /// Select an item by its position. Invalid indexes leave selection intact.
    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.items.len() {
            return false;
        }
        self.selected = index;
        true
    }

    /// Equip the selected item when it is a weapon (Fallout item type 3).
    pub fn equip_selected(&mut self) -> Result<(), String> {
        let item = self
            .items
            .get(self.selected)
            .ok_or_else(|| "No selected inventory item".to_string())?;
        if item.item_type != 3 || item.quantity == 0 {
            return Err(format!("Item {} is not a weapon", item.id));
        }
        self.equipped = Some(item.id);
        Ok(())
    }

    /// Return carried weight, saturating at the largest representable value.
    pub fn weight(&self) -> i32 {
        self.items.iter().fold(0_i32, |total, item| {
            total.saturating_add(item.quantity.saturating_mul(item.weight))
        })
    }

    /// Load compatible ammunition packets into the equipped weapon.
    ///
    /// Ammo stacks keep a stable item record when depleted. `ammo_quantity`
    /// stores the rounds in the leading packet; zero is treated as a full
    /// packet for legacy inventory data. Empty weapons pick the first available
    /// compatible ammo PID; loaded weapons stay with their current PID.
    pub fn reload_equipped(&mut self) -> Result<i32, String> {
        let weapon_index = self
            .items
            .iter()
            .position(|item| Some(item.id) == self.equipped)
            .ok_or_else(|| "No equipped weapon".to_string())?;
        let weapon = &self.items[weapon_index];
        let chosen_ammo_pid = if weapon.ammo_quantity > 0 && weapon.ammo_pid > 0 {
            weapon.ammo_pid
        } else {
            self.items
                .iter()
                .find(|item| {
                    item.item_type == 4
                        && item.caliber == weapon.caliber
                        && item.quantity > 0
                        && item.pid > 0
                })
                .map(|item| item.pid)
                .ok_or_else(|| "No compatible ammunition available".to_string())?
        };
        self.reload_weapon_from_pid(weapon_index, chosen_ammo_pid)
    }

    /// Reload from the selected ammo row, consuming only stacks with its PID.
    pub fn reload_from(&mut self, ammo_index: usize) -> Result<i32, String> {
        let weapon_index = self
            .items
            .iter()
            .position(|item| Some(item.id) == self.equipped)
            .ok_or_else(|| "No equipped weapon".to_string())?;
        let ammo = self
            .items
            .get(ammo_index)
            .ok_or_else(|| "No selected ammunition item".to_string())?;
        if ammo.item_type != 4 || ammo.quantity <= 0 || ammo.pid <= 0 {
            return Err("Selected item has no known ammunition to load".to_string());
        }
        let weapon = &self.items[weapon_index];
        if weapon.item_type != 3 || weapon.ammo_capacity <= 0 {
            return Err("Equipped weapon cannot be reloaded".to_string());
        }
        if ammo.caliber != weapon.caliber {
            return Err("Selected ammunition is not compatible with the weapon".to_string());
        }
        if weapon.ammo_quantity > 0 && weapon.ammo_pid > 0 && weapon.ammo_pid != ammo.pid {
            return Err("A loaded weapon cannot mix ammunition types".to_string());
        }
        self.reload_weapon_from_pid(weapon_index, ammo.pid)
    }

    fn reload_weapon_from_pid(
        &mut self,
        weapon_index: usize,
        chosen_ammo_pid: i32,
    ) -> Result<i32, String> {
        let weapon = &self.items[weapon_index];
        if weapon.item_type != 3 || weapon.ammo_capacity <= 0 {
            return Err("Equipped weapon cannot be reloaded".to_string());
        }
        if weapon.ammo_quantity < 0 || weapon.ammo_quantity > weapon.ammo_capacity {
            return Err(format!(
                "Weapon {} has invalid loaded ammunition",
                weapon.id
            ));
        }
        let room = weapon.ammo_capacity - weapon.ammo_quantity;
        if room == 0 {
            return Err("Equipped weapon is already full".to_string());
        }
        let weapon_caliber = weapon.caliber;
        let mut working: Vec<(usize, i32, i32)> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.item_type == 4 && item.caliber == weapon_caliber && item.pid == chosen_ammo_pid
            })
            .map(|(index, item)| (index, item.quantity, item.ammo_quantity))
            .collect();
        if working.is_empty() {
            return Err("No compatible ammunition".to_string());
        }

        let mut remaining = room;
        let mut loaded = 0;
        for (index, quantity, lead_rounds) in &mut working {
            if remaining == 0 || *quantity == 0 {
                continue;
            }
            let pack_size = self.items[*index].ammo_pack_size;
            if pack_size < 1 || *lead_rounds < 0 || *lead_rounds > pack_size {
                return Err(format!(
                    "Ammunition {} has invalid packet data",
                    self.items[*index].id
                ));
            }
            let mut packet_rounds = if *lead_rounds == 0 {
                pack_size
            } else {
                *lead_rounds
            };
            while remaining > 0 && *quantity > 0 {
                let transfer = remaining.min(packet_rounds);
                remaining -= transfer;
                loaded += transfer;
                packet_rounds -= transfer;
                if packet_rounds == 0 {
                    *quantity -= 1;
                    if *quantity > 0 {
                        packet_rounds = pack_size;
                    }
                }
            }
            *lead_rounds = if *quantity == 0 { 0 } else { packet_rounds };
        }
        if loaded == 0 {
            return Err("No compatible ammunition available".to_string());
        }

        self.items[weapon_index].ammo_quantity += loaded;
        self.items[weapon_index].ammo_pid = chosen_ammo_pid;
        for (index, quantity, lead_rounds) in working {
            self.items[index].quantity = quantity;
            self.items[index].ammo_quantity = lead_rounds;
        }
        Ok(loaded)
    }

    /// Unload all rounds into an existing catalog row for the same ammo PID.
    pub fn unload_equipped(&mut self) -> Result<i32, String> {
        let weapon_index = self
            .items
            .iter()
            .position(|item| Some(item.id) == self.equipped)
            .ok_or_else(|| "No equipped weapon".to_string())?;
        let weapon = &self.items[weapon_index];
        if weapon.item_type != 3 || weapon.ammo_capacity <= 0 {
            return Err("Equipped weapon has no tracked ammunition".to_string());
        }
        if weapon.ammo_quantity <= 0 || weapon.ammo_quantity > weapon.ammo_capacity {
            return Err("Equipped weapon has no valid ammunition to unload".to_string());
        }
        if weapon.ammo_pid <= 0 {
            return Err("Loaded ammunition type is unknown".to_string());
        }
        let ammo_index = self
            .items
            .iter()
            .position(|item| {
                item.item_type == 4 && item.pid == weapon.ammo_pid && item.caliber == weapon.caliber
            })
            .ok_or_else(|| "No matching ammunition stack in inventory".to_string())?;
        let ammo = &self.items[ammo_index];
        let pack_size = ammo.ammo_pack_size;
        if pack_size <= 0
            || ammo.quantity < 0
            || ammo.ammo_quantity < 0
            || (ammo.quantity == 0 && ammo.ammo_quantity != 0)
            || (ammo.quantity > 0 && ammo.ammo_quantity > pack_size)
        {
            return Err("Ammunition stack has invalid packet data".to_string());
        }
        let leading = if ammo.quantity == 0 {
            0
        } else if ammo.ammo_quantity == 0 {
            pack_size
        } else {
            ammo.ammo_quantity
        };
        let existing = ammo
            .quantity
            .checked_sub(if ammo.quantity > 0 { 1 } else { 0 })
            .and_then(|n| n.checked_mul(pack_size))
            .and_then(|n| n.checked_add(leading))
            .ok_or_else(|| "Ammunition stack is too large".to_string())?;
        let unloaded = weapon.ammo_quantity;
        let total = existing
            .checked_add(unloaded)
            .ok_or_else(|| "Ammunition stack is too large".to_string())?;
        let new_quantity = total
            .checked_add(pack_size - 1)
            .ok_or_else(|| "Ammunition stack is too large".to_string())?
            / pack_size;
        let new_leading = total - (new_quantity - 1) * pack_size;

        self.items[ammo_index].quantity = new_quantity;
        self.items[ammo_index].ammo_quantity = new_leading;
        self.items[weapon_index].ammo_quantity = 0;
        self.items[weapon_index].ammo_pid = 0;
        Ok(unloaded)
    }

    /// Consume one healing item and return the actual hit points restored.
    pub fn consume_healing(
        &mut self,
        index: usize,
        heal_roll: i32,
        hp: i32,
        max_hp: i32,
    ) -> Result<i32, String> {
        if max_hp <= 0 || hp < 0 || hp > max_hp {
            return Err("Invalid hit point state".to_string());
        }
        if hp == 0 {
            return Err("Cannot use healing items while dead".to_string());
        }
        let item = self
            .items
            .get(index)
            .ok_or_else(|| "No selected inventory item".to_string())?;
        if item.heal_max <= 0 || item.heal_min < 0 || item.heal_min > item.heal_max {
            return Err(format!("Item {} is not a healing item", item.id));
        }
        if item.quantity <= 0 {
            return Err(format!("Item {} is depleted", item.id));
        }
        if heal_roll < item.heal_min || heal_roll > item.heal_max {
            return Err("Healing roll is outside the item's range".to_string());
        }
        let restored = heal_roll.min(max_hp - hp);
        self.items[index].quantity -= 1;
        Ok(restored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: i32, name: &str, item_type: i32, quantity: i32, weight: i32) -> Item {
        Item {
            id,
            pid: id + 1000,
            name: name.to_string(),
            description: format!("Synthetic {name}"),
            file: format!("art/{name}.png"),
            quantity,
            weight,
            item_type,
            ammo_quantity: 0,
            ammo_capacity: 0,
            caliber: 0,
            ammo_pid: 0,
            ammo_pack_size: 0,
            heal_min: 0,
            heal_max: 0,
        }
    }

    #[test]
    fn counts_items_as_entries_and_weights_each_quantity() {
        let inventory = Inventory::new(vec![
            item(1, "stimpack", 0, 3, 2),
            item(2, "another_stimpack", 0, 1, 2),
            item(3, "pistol", 3, 1, 5),
        ])
        .unwrap();
        assert_eq!(inventory.items.len(), 3);
        assert_eq!(inventory.weight(), 13);
        assert_eq!(inventory.selected, 0);
        assert_eq!(inventory.equipped, None);
    }

    #[test]
    fn selection_changes_only_for_existing_indexes() {
        let mut inventory =
            Inventory::new(vec![item(1, "knife", 3, 1, 2), item(2, "pistol", 3, 1, 5)]).unwrap();
        assert!(inventory.select(1));
        assert_eq!(inventory.selected, 1);
        assert!(!inventory.select(2));
        assert_eq!(inventory.selected, 1);
        assert!(!Inventory::new(Vec::new()).unwrap().select(0));
    }

    #[test]
    fn only_type_three_items_can_be_equipped() {
        let mut inventory = Inventory::new(vec![
            item(1, "stimpack", 0, 1, 2),
            item(2, "pistol", 3, 1, 5),
        ])
        .unwrap();
        assert!(inventory.equip_selected().is_err());
        assert_eq!(inventory.equipped, None);
        assert!(inventory.select(1));
        inventory.equip_selected().unwrap();
        assert_eq!(inventory.equipped, Some(2));
        assert!(
            Inventory::new(Vec::new())
                .unwrap()
                .equip_selected()
                .is_err()
        );
    }

    #[test]
    fn constructor_allows_depleted_rows_but_rejects_negative_quantity_and_weight() {
        assert!(Inventory::new(vec![item(1, "empty", 0, 0, 1)]).is_ok());
        assert!(Inventory::new(vec![item(1, "bad", 0, -1, 1)]).is_err());
        assert!(Inventory::new(vec![item(1, "bad", 0, 1, -1)]).is_err());
    }

    #[test]
    fn weight_saturates_instead_of_overflowing() {
        let inventory = Inventory::new(vec![
            item(1, "heavy", 0, i32::MAX, i32::MAX),
            item(2, "also_heavy", 0, 1, 1),
        ])
        .unwrap();
        assert_eq!(inventory.weight(), i32::MAX);
    }

    fn weapon(id: i32, loaded: i32, capacity: i32, caliber: i32) -> Item {
        let mut value = item(id, "weapon", 3, 1, 5);
        value.ammo_quantity = loaded;
        value.ammo_capacity = capacity;
        value.caliber = caliber;
        value.ammo_pid = 0;
        value
    }

    #[test]
    fn reload_keeps_ammunition_pid_consistent_for_loaded_and_empty_weapons() {
        let mut gun = weapon(1, 2, 10, 45);
        gun.ammo_pid = 7;
        let mut matching = ammo(2, 2, 0, 4, 45);
        matching.pid = 7;
        let mut different = ammo(3, 2, 0, 4, 45);
        different.pid = 8;
        let mut loaded = Inventory::new(vec![gun, matching, different]).unwrap();
        loaded.equipped = Some(1);
        assert_eq!(loaded.reload_equipped().unwrap(), 8);
        assert_eq!(loaded.items[0].ammo_quantity, 10);
        assert_eq!(loaded.items[0].ammo_pid, 7);
        assert_eq!(loaded.items[1].quantity, 0);
        assert_eq!(loaded.items[2].quantity, 2);

        let mut first = ammo(12, 1, 0, 3, 45);
        first.pid = 7;
        let mut second = ammo(13, 2, 0, 3, 45);
        second.pid = 7;
        let mut other = ammo(14, 2, 0, 3, 45);
        other.pid = 8;
        let mut empty = Inventory::new(vec![weapon(11, 0, 6, 45), first, second, other]).unwrap();
        empty.equipped = Some(11);
        assert_eq!(empty.reload_equipped().unwrap(), 6);
        assert_eq!(empty.items[0].ammo_pid, 7);
        assert_eq!(empty.items[1].quantity, 0);
        assert_eq!(empty.items[2].quantity, 1);
        assert_eq!(empty.items[2].ammo_quantity, 3);
        assert_eq!(empty.items[3].quantity, 2);
    }

    fn ammo(id: i32, quantity: i32, lead_rounds: i32, pack_size: i32, caliber: i32) -> Item {
        let mut value = item(id, "ammo", 4, quantity, 1);
        value.ammo_quantity = lead_rounds;
        value.ammo_pack_size = pack_size;
        value.caliber = caliber;
        value
    }

    #[test]
    fn reload_uses_partial_leading_packet_then_full_packets() {
        let mut inventory =
            Inventory::new(vec![weapon(1, 2, 10, 45), ammo(2, 2, 2, 6, 45)]).unwrap();
        inventory.equipped = Some(1);

        assert_eq!(inventory.reload_equipped().unwrap(), 8);
        assert_eq!(inventory.items[0].ammo_quantity, 10);
        assert_eq!(inventory.items[1].quantity, 0);
        assert_eq!(inventory.items[1].ammo_quantity, 0);
    }

    #[test]
    fn reload_preserves_rounds_left_in_a_packet() {
        let mut inventory =
            Inventory::new(vec![weapon(1, 0, 5, 10), ammo(2, 3, 0, 4, 10)]).unwrap();
        inventory.equipped = Some(1);

        assert_eq!(inventory.reload_equipped().unwrap(), 5);
        assert_eq!(inventory.items[0].ammo_quantity, 5);
        assert_eq!(inventory.items[1].quantity, 2);
        assert_eq!(inventory.items[1].ammo_quantity, 3);
    }

    #[test]
    fn selected_reload_uses_only_selected_ammo_pid() {
        let gun = weapon(1, 0, 4, 45);
        let mut ammo_a = ammo(2, 2, 0, 4, 45);
        ammo_a.pid = 7;
        let mut jhp = ammo(3, 2, 0, 4, 45);
        jhp.pid = 8;
        let mut inventory = Inventory::new(vec![gun.clone(), ammo_a, jhp]).unwrap();
        inventory.equipped = Some(gun.id);

        assert_eq!(inventory.reload_from(1).unwrap(), 4);
        assert_eq!(inventory.items[0].ammo_pid, 7);
        assert_eq!(inventory.items[1].quantity, 1);
        assert_eq!(inventory.items[2].quantity, 2);
    }

    #[test]
    fn selected_reload_rejects_mixing_a_partial_magazine_atomically() {
        let mut gun = weapon(1, 2, 8, 45);
        gun.ammo_pid = 7;
        let mut ammo_a = ammo(2, 1, 0, 4, 45);
        ammo_a.pid = 7;
        let mut jhp = ammo(3, 1, 0, 4, 45);
        jhp.pid = 8;
        let mut inventory = Inventory::new(vec![gun, ammo_a, jhp]).unwrap();
        inventory.equipped = Some(1);
        let before = inventory.clone();

        assert!(inventory.reload_from(2).is_err());
        assert_eq!(inventory, before);
    }

    #[test]
    fn unload_conserves_rounds_and_preserves_partial_leading_packet() {
        let mut gun = weapon(1, 4, 10, 45);
        gun.ammo_pid = 7;
        let mut ammo_row = ammo(2, 2, 3, 5, 45);
        ammo_row.pid = 7;
        let mut inventory = Inventory::new(vec![gun, ammo_row]).unwrap();
        inventory.equipped = Some(1);

        assert_eq!(inventory.unload_equipped().unwrap(), 4);
        assert_eq!(inventory.items[0].ammo_quantity, 0);
        assert_eq!(inventory.items[0].ammo_pid, 0);
        assert_eq!(inventory.items[1].quantity, 3);
        assert_eq!(inventory.items[1].ammo_quantity, 2);
        assert_eq!((3 - 1) * 5 + 2, 12);
    }

    #[test]
    fn incompatible_or_malformed_ammo_fails_without_changes() {
        let mut inventory = Inventory::new(vec![weapon(1, 1, 6, 45), ammo(2, 1, 0, 6, 9)]).unwrap();
        inventory.equipped = Some(1);
        let before = inventory.clone();
        assert!(inventory.reload_equipped().is_err());
        assert_eq!(inventory, before);

        inventory.items[1].caliber = 45;
        inventory.items[1].ammo_pack_size = 0;
        let before = inventory.clone();
        assert!(inventory.reload_equipped().is_err());
        assert_eq!(inventory, before);
    }

    #[test]
    fn healing_caps_at_maximum_and_consumes_only_after_valid_checks() {
        let mut stim = item(1, "stim", 0, 1, 1);
        stim.heal_min = 4;
        stim.heal_max = 8;
        let mut inventory = Inventory::new(vec![stim]).unwrap();

        assert_eq!(inventory.consume_healing(0, 6, 19, 20).unwrap(), 1);
        assert_eq!(inventory.items[0].quantity, 0);
        assert!(inventory.consume_healing(0, 6, 10, 20).is_err());
        assert_eq!(inventory.items[0].quantity, 0);
        assert!(inventory.consume_healing(0, 3, 10, 20).is_err());
        assert!(inventory.consume_healing(0, 6, 0, 20).is_err());
        let mut full_hp_stim = item(2, "stim", 0, 1, 1);
        full_hp_stim.heal_min = 4;
        full_hp_stim.heal_max = 8;
        let mut full_hp = Inventory::new(vec![full_hp_stim]).unwrap();
        assert_eq!(full_hp.consume_healing(0, 6, 10, 10).unwrap(), 0);
        assert_eq!(full_hp.items[0].quantity, 0);
    }

    #[test]
    fn old_item_json_defaults_new_combat_fields() {
        let old = r#"{"id":1,"pid":2,"name":"old","description":"old item","file":"old.frm","quantity":1,"weight":1,"item_type":0,"ammo_quantity":0,"ammo_capacity":0}"#;
        let parsed: Item = serde_json::from_str(old).unwrap();
        assert_eq!(parsed.caliber, 0);
        assert_eq!(parsed.ammo_pid, 0);
        assert_eq!(parsed.ammo_pack_size, 0);
        assert_eq!(parsed.heal_min, 0);
        assert_eq!(parsed.heal_max, 0);
    }
}
