use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

const MAX_JSON_BYTES: usize = 16 * 1024 * 1024;
const MAX_WORLD_OBJECTS: usize = 100_000;

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct State {
    pub open: bool,
    #[serde(skip)]
    pub loading: bool,
    pub session: Option<Value>,
    pub view: Option<View>,
    pub error: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct View {
    pub reply: Option<Reply>,
    #[serde(default)]
    pub options: Vec<Choice>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reply {
    #[serde(rename = "listId")]
    pub list_id: i32,
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Choice {
    pub text: Option<String>,
    pub procedure: Option<String>,
    pub reaction: i32,
    #[serde(default, rename = "continue")]
    pub continuing: bool,
}

impl State {
    pub fn validate(&self) -> Result<(), String> {
        if serde_json::to_vec(self)
            .map_err(|error| error.to_string())?
            .len()
            > MAX_JSON_BYTES
        {
            return Err("dialogue state exceeds the 16 MiB limit".into());
        }
        let Some(session) = &self.session else {
            if !self.loading && (self.view.is_some() || self.open) {
                return Err("dialogue state has UI data without a session".into());
            }
            return Ok(());
        };
        let blob = serde_json::to_vec(session).map_err(|error| error.to_string())?;
        if blob.len() > MAX_JSON_BYTES {
            return Err("dialogue session exceeds the 16 MiB limit".into());
        }
        if session.get("script").and_then(Value::as_str) != Some("over") {
            return Err("dialogue session is not the original Overseer script".into());
        }
        if session
            .get("step")
            .and_then(Value::as_u64)
            .filter(|step| *step >= 1)
            .is_none()
        {
            return Err("dialogue session has an invalid step".into());
        }
        let session_view = match session.get("view") {
            None | Some(Value::Null) => None,
            Some(value) => Some(
                serde_json::from_value::<View>(value.clone())
                    .map_err(|error| format!("invalid dialogue view: {error}"))?,
            ),
        };
        if session_view != self.view {
            return Err("dialogue view does not match the session snapshot".into());
        }
        let world = session
            .get("world")
            .ok_or_else(|| "dialogue session has no world snapshot".to_string())?;
        let objects = world
            .get("objects")
            .and_then(Value::as_array)
            .ok_or_else(|| "dialogue session has an invalid object list".to_string())?;
        if objects.len() > MAX_WORLD_OBJECTS {
            return Err("dialogue world has too many objects".into());
        }
        let player_id = world
            .get("playerId")
            .and_then(Value::as_i64)
            .ok_or_else(|| "dialogue world has no valid player id".to_string())?;
        if !objects
            .iter()
            .any(|object| object.get("id").and_then(Value::as_i64) == Some(player_id))
        {
            return Err("dialogue world player is missing from its objects".into());
        }
        Ok(())
    }
}

/// Runs one original-script dialogue request through the local Node worker.
/// The worker path is fixed at build time; request data cannot select a file.
pub fn run(request: Value) -> Result<Value, String> {
    let request_bytes = serde_json::to_vec(&request).map_err(|error| error.to_string())?;
    if request_bytes.len() > MAX_JSON_BYTES {
        return Err("dialogue request exceeds the 16 MiB limit".into());
    }
    if let Some(character_path) = request.get("characterPath") {
        let path = character_path
            .as_str()
            .ok_or_else(|| "invalid character path".to_string())?;
        if ![
            "premade/combat.gcd",
            "premade/diplomat.gcd",
            "premade/stealth.gcd",
        ]
        .contains(&path)
        {
            return Err("unsupported player character".into());
        }
    }
    if let Some(action) = request.get("action") {
        if !matches!(action.as_str(), Some("start" | "reopen" | "choose")) {
            return Err("unsupported dialogue action".into());
        }
    }

    let worker = concat!(env!("CARGO_MANIFEST_DIR"), "/script-runtime.mjs");
    let mut child = Command::new("node")
        .arg(worker)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not start dialogue worker: {error}"))?;
    child
        .stdin
        .take()
        .ok_or_else(|| "dialogue worker stdin unavailable".to_string())?
        .write_all(&request_bytes)
        .map_err(|error| format!("could not send dialogue request: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("could not read dialogue worker output: {error}"))?;
    if output.stdout.len() > MAX_JSON_BYTES {
        return Err("dialogue response exceeds the 16 MiB limit".into());
    }
    let response: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        format!(
            "dialogue worker returned invalid JSON: {error}; {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })?;
    let status = response
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    if !output.status.success() || !matches!(status, "dialogue" | "halted") {
        let message = response
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("dialogue script did not complete");
        let blocked = response
            .get("blockedInstruction")
            .or_else(|| response.get("blocked_instruction"))
            .and_then(Value::as_str);
        return Err(match blocked {
            Some(opcode) => format!("{message} (blocked at {opcode})"),
            None => message.to_string(),
        });
    }
    let dialogue = response
        .get("dialogue")
        .cloned()
        .ok_or_else(|| "dialogue worker omitted its view".to_string())?;
    let state = State {
        open: dialogue
            .get("open")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        loading: false,
        session: response.get("session").cloned(),
        view: Some(
            serde_json::from_value(dialogue)
                .map_err(|error| format!("dialogue worker returned an invalid view: {error}"))?,
        ),
        error: None,
    };
    state.validate()?;
    Ok(response)
}

/// Rebuilds catalog-backed inventory rows from the worker's structural world snapshot.
/// The input catalog is never mutated; any unknown item aborts the whole conversion.
pub fn apply_inventory(
    items: &[crate::inventory::Item],
    response: &Value,
) -> Result<Vec<crate::inventory::Item>, String> {
    if serde_json::to_vec(response)
        .map_err(|error| error.to_string())?
        .len()
        > MAX_JSON_BYTES
    {
        return Err("dialogue response exceeds the 16 MiB limit".into());
    }
    if items.len() > 10_000 {
        return Err("inventory catalog exceeds the 10000 item limit".into());
    }
    let world = response
        .get("session")
        .and_then(|session| session.get("world"))
        .ok_or_else(|| "dialogue response has no world snapshot".to_string())?;
    let player_id = world
        .get("playerId")
        .and_then(Value::as_i64)
        .ok_or_else(|| "dialogue world has no valid player id".to_string())?;
    let objects = world
        .get("objects")
        .and_then(Value::as_array)
        .ok_or_else(|| "dialogue world has an invalid object list".to_string())?;
    if objects.len() > MAX_WORLD_OBJECTS {
        return Err("dialogue world has too many objects".into());
    }
    let object_by_id: std::collections::HashMap<i64, &Value> = objects
        .iter()
        .filter_map(|object| Some((object.get("id")?.as_i64()?, object)))
        .collect();
    let player = object_by_id
        .get(&player_id)
        .ok_or_else(|| "dialogue world player is missing from its objects".to_string())?;
    let slots = player
        .get("inventory")
        .and_then(Value::as_array)
        .ok_or_else(|| "dialogue player has an invalid inventory".to_string())?;
    if slots.len() > MAX_WORLD_OBJECTS {
        return Err("dialogue player inventory has too many slots".into());
    }

    let mut result = items.to_vec();
    let mut index_by_id = std::collections::HashMap::new();
    let mut first_index_by_pid = std::collections::HashMap::new();
    for (index, item) in result.iter_mut().enumerate() {
        if item.id < 0
            || item.pid < 0
            || item.quantity < 0
            || item.ammo_capacity < 0
            || item.ammo_quantity < 0
        {
            return Err(format!("catalog item {} has invalid bounds", item.id));
        }
        if index_by_id.insert(item.id, index).is_some() {
            return Err(format!("duplicate catalog bridge id {}", item.id));
        }
        first_index_by_pid.entry(item.pid).or_insert(index);
        item.quantity = 0;
        item.ammo_quantity = 0;
        if item.item_type == 4 {
            item.ammo_pid = 0;
        }
    }
    let mut ammo_rounds: std::collections::HashMap<usize, i64> = std::collections::HashMap::new();

    for slot in slots {
        let object_id = slot
            .get("itemId")
            .and_then(Value::as_i64)
            .ok_or_else(|| "inventory slot has an invalid item id".to_string())?;
        let stack_quantity = slot
            .get("quantity")
            .and_then(Value::as_i64)
            .filter(|quantity| *quantity > 0 && *quantity <= i32::MAX as i64)
            .ok_or_else(|| "inventory slot has an invalid quantity".to_string())?;
        let object = object_by_id
            .get(&object_id)
            .ok_or_else(|| format!("inventory item object {object_id} is missing"))?;
        let pid = object
            .get("pid")
            .and_then(Value::as_i64)
            .filter(|pid| *pid >= 0 && *pid <= i32::MAX as i64)
            .ok_or_else(|| format!("inventory object {object_id} has an invalid prototype"))?
            as i32;
        let index = if let Some(bridge_id) = object.get("bridgeId").and_then(Value::as_i64) {
            if bridge_id < 0 || bridge_id > i32::MAX as i64 {
                return Err(format!("inventory bridge id {bridge_id} is invalid"));
            }
            let index = *index_by_id
                .get(&(bridge_id as i32))
                .ok_or_else(|| format!("inventory bridge id {bridge_id} is unknown"))?;
            if result[index].pid != pid {
                return Err(format!(
                    "inventory bridge id {bridge_id} does not match prototype {pid}"
                ));
            }
            index
        } else {
            *first_index_by_pid
                .get(&pid)
                .ok_or_else(|| format!("unknown inventory prototype {pid}"))?
        };
        let item = &mut result[index];
        if item.item_type == 3 {
            let data = object
                .get("data")
                .and_then(Value::as_object)
                .ok_or_else(|| format!("weapon object {object_id} has invalid data"))?;
            let loaded = data
                .get("ammoQuantity")
                .and_then(Value::as_i64)
                .ok_or_else(|| format!("weapon object {object_id} has no ammo quantity"))?;
            let ammo_pid = data.get("ammoTypePid").and_then(Value::as_i64).unwrap_or(0);
            if loaded < 0
                || loaded > item.ammo_capacity as i64
                || (ammo_pid < 0 && !(ammo_pid == -1 && item.ammo_capacity == 0 && loaded == 0))
                || ammo_pid > i32::MAX as i64
            {
                return Err(format!("weapon object {object_id} has invalid ammunition"));
            }
            if item.quantity > 0
                && (item.ammo_quantity != loaded as i32 || item.ammo_pid != ammo_pid as i32)
            {
                return Err(format!(
                    "multiple weapons for item {} have incompatible loaded ammunition",
                    item.id
                ));
            }
            item.quantity = item
                .quantity
                .checked_add(stack_quantity as i32)
                .ok_or_else(|| "inventory quantity overflow".to_string())?;
            item.ammo_quantity = loaded as i32;
            item.ammo_pid = ammo_pid as i32;
        } else if item.item_type == 4 {
            let data = object
                .get("data")
                .and_then(Value::as_object)
                .ok_or_else(|| format!("ammunition object {object_id} has invalid data"))?;
            let first_packet = data
                .get("quantity")
                .and_then(Value::as_i64)
                .ok_or_else(|| format!("ammunition object {object_id} has no packet quantity"))?;
            let pack_size = item.ammo_pack_size;
            if pack_size <= 0 || first_packet < 0 || first_packet > pack_size as i64 {
                return Err(format!(
                    "ammunition object {object_id} has invalid packet quantity"
                ));
            }
            let rounds = first_packet
                .checked_add(
                    (stack_quantity - 1)
                        .checked_mul(pack_size as i64)
                        .ok_or_else(|| "ammunition quantity overflow".to_string())?,
                )
                .ok_or_else(|| "ammunition quantity overflow".to_string())?;
            let total = ammo_rounds
                .get(&index)
                .copied()
                .unwrap_or(0)
                .checked_add(rounds)
                .ok_or_else(|| "ammunition quantity overflow".to_string())?;
            ammo_rounds.insert(index, total);
        } else {
            item.quantity = item
                .quantity
                .checked_add(stack_quantity as i32)
                .ok_or_else(|| "inventory quantity overflow".to_string())?;
        }
    }

    for (index, rounds) in ammo_rounds {
        let item = &mut result[index];
        let pack_size = item.ammo_pack_size as i64;
        let quantity = if rounds == 0 {
            0
        } else {
            (rounds + pack_size - 1) / pack_size
        };
        if quantity > i32::MAX as i64 || rounds > i32::MAX as i64 {
            return Err("ammunition quantity overflow".into());
        }
        item.quantity = quantity as i32;
        item.ammo_quantity = if quantity == 0 {
            0
        } else {
            (rounds - (quantity - 1) * pack_size) as i32
        };
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::Item;
    use serde_json::json;

    fn item(
        id: i32,
        pid: i32,
        item_type: i32,
        quantity: i32,
        ammo_quantity: i32,
        ammo_pack_size: i32,
    ) -> Item {
        Item {
            id,
            pid,
            name: format!("item-{pid}"),
            description: "catalog description".into(),
            file: "item.png".into(),
            quantity,
            weight: 1,
            item_type,
            ammo_quantity,
            ammo_capacity: 12,
            caliber: 1,
            ammo_pid: 0,
            ammo_pack_size,
            heal_min: 0,
            heal_max: 0,
        }
    }

    fn response(objects: Value, player_inventory: Value) -> Value {
        let mut all_objects = vec![json!({"id":1,"pid":16777216,"inventory":player_inventory})];
        all_objects.extend(objects.as_array().unwrap().iter().cloned());
        json!({"status":"dialogue","dialogue":{"open":true,"reply":null,"options":[]},"session":{
            "script":"over","step":1,"view":{"open":true,"reply":null,"options":[]},
            "world":{"schema":1,"playerId":1,"objects":all_objects}
        }})
    }

    #[test]
    fn non_ammunition_weapons_preserve_original_minus_one_sentinel() {
        let mut knife = item(10, 4, 3, 1, 0, 0);
        knife.ammo_capacity = 0;
        knife.ammo_pid = -1;
        let snapshot = response(
            json!([{ "id":2,"pid":4,"bridgeId":10,"data":{"ammoQuantity":0,"ammoTypePid":-1} }]),
            json!([{ "itemId":2,"quantity":1 }]),
        );
        let updated = apply_inventory(&[knife.clone()], &snapshot).unwrap();
        assert_eq!(updated, vec![knife]);
        let mut gun = item(10, 4, 3, 1, 0, 0);
        gun.ammo_capacity = 12;
        assert!(apply_inventory(&[gun], &snapshot).is_err());
    }

    #[test]
    fn state_validates_matching_original_session_and_rejects_bad_player() {
        let session = response(json!([]), json!([]))["session"].clone();
        let view: View = serde_json::from_value(session["view"].clone()).unwrap();
        State {
            open: true,
            session: Some(session),
            view: Some(view),
            ..State::default()
        }
        .validate()
        .unwrap();
        let mut bad = response(json!([]), json!([]))["session"].clone();
        bad["script"] = json!("other");
        assert!(
            State {
                open: true,
                session: Some(bad),
                view: None,
                ..State::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            State {
                open: true,
                ..State::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            State {
                open: true,
                loading: true,
                ..State::default()
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn inventory_reconciles_bridge_items_and_known_pid_gifts() {
        let catalog = vec![item(10, 100, 0, 4, 0, 0), item(11, 200, 0, 1, 0, 0)];
        let result = response(
            json!([
                {"id":2,"pid":100,"bridgeId":10,"data":{}},
                {"id":3,"pid":200,"data":{}}
            ]),
            json!([{ "itemId":2,"quantity":2 },{ "itemId":3,"quantity":1 } ]),
        );
        let updated = apply_inventory(&catalog, &result).unwrap();
        assert_eq!(updated[0].quantity, 2);
        assert_eq!(updated[1].quantity, 1);
        assert_eq!(updated[0].name, catalog[0].name);
        assert!(
            apply_inventory(
                &catalog,
                &response(
                    json!([{"id":4,"pid":999,"data":{}}]),
                    json!([{ "itemId":4,"quantity":1 }])
                )
            )
            .is_err()
        );
        assert_eq!(catalog[0].quantity, 4);
    }

    #[test]
    fn ammo_rounds_conserve_across_partial_packets_and_weapon_load() {
        let catalog = vec![item(10, 100, 4, 3, 10, 10), item(11, 200, 3, 1, 5, 0)];
        let result = response(
            json!([
                {"id":2,"pid":100,"bridgeId":10,"data":{"quantity":4}},
                {"id":3,"pid":100,"data":{"quantity":7}},
                {"id":4,"pid":200,"bridgeId":11,"data":{"ammoQuantity":6,"ammoTypePid":100}}
            ]),
            json!([{ "itemId":2,"quantity":2 },{ "itemId":3,"quantity":1 },{ "itemId":4,"quantity":1 } ]),
        );
        let updated = apply_inventory(&catalog, &result).unwrap();
        assert_eq!(
            updated[0].quantity * 10 - (10 - updated[0].ammo_quantity),
            21
        );
        assert_eq!(updated[0].quantity, 3);
        assert_eq!(updated[0].ammo_quantity, 1);
        assert_eq!(updated[1].ammo_quantity, 6);
        assert_eq!(updated[1].ammo_pid, 100);
    }
}
