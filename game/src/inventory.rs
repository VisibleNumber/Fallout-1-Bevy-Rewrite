use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
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
}

/// A small inventory model for the current game prototype.
///
/// It tracks item records, selection, equipped weapon identity, and carried
/// weight. It does not model use, drops, combat, ammunition changes, or item
/// stacking rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventory {
    pub items: Vec<Item>,
    pub selected: usize,
    pub equipped: Option<i32>,
}

impl Inventory {
    pub fn new(items: Vec<Item>) -> Result<Self, String> {
        for item in &items {
            if item.quantity < 1 {
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
        if item.item_type != 3 {
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
    fn constructor_rejects_invalid_quantity_and_weight() {
        assert!(Inventory::new(vec![item(1, "bad", 0, 0, 1)]).is_err());
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
}
