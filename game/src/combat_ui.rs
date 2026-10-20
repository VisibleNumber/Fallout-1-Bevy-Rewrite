//! First cave combat integration. Rat decisions are provisional, not the original AI.
use crate::{
    Frame, Motion, Player, SceneData,
    combat::{self, Combatant, Weapon},
    inventory_ui::Bag,
    navigation, point,
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ActorData {
    pub id: i32,
    pub tile: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub ap: i32,
    pub armor_class: i32,
    pub thresholds: Vec<i32>,
    pub resistances: Vec<i32>,
    pub melee_damage: i32,
    pub skill: i32,
    pub rotation: usize,
    pub standing: Vec<Frame>,
    pub dead: Vec<Frame>,
}
#[derive(Deserialize)]
pub struct WeaponData {
    pub pid: i32,
    pub damage_type: usize,
    pub min_damage: i32,
    pub max_damage: i32,
    pub range: i32,
    pub ap_cost: i32,
    pub skill: i32,
    pub ranged: bool,
    pub perception: i32,
}
impl WeaponData {
    fn weapon(&self, ammo: i32, capacity: i32) -> Weapon {
        Weapon {
            min_damage: self.min_damage,
            max_damage: self.max_damage,
            range: self.range,
            ap_cost: self.ap_cost,
            skill: self.skill,
            ammo,
            capacity,
            ranged: self.ranged,
            perception: self.perception,
        }
    }
}
#[derive(Resource)]
pub struct Fight {
    pub player: Combatant,
    pub rats: Vec<Combatant>,
    pub active: bool,
    pub touched: bool,
    pub attack_time: f32,
    pub looted: Vec<i32>,
    pub participants: Vec<i32>,
    pub story: crate::dialogue::Story,
    pub original: crate::original_dialogue::State,
    pub script_awards: std::collections::BTreeMap<i32, i32>,
    seed: u64,
}
impl Fight {
    pub fn new(data: &SceneData) -> Self {
        Self {
            player: data.player_combat.clone(),
            rats: data
                .actors
                .iter()
                .map(|a| Combatant {
                    id: a.id,
                    tile: a.tile,
                    hp: a.hp,
                    max_hp: a.max_hp,
                    ap: a.ap,
                    max_ap: a.ap,
                    armor_class: a.armor_class,
                })
                .collect(),
            active: false,
            touched: false,
            attack_time: 0.,
            looted: Vec::new(),
            participants: Vec::new(),
            story: crate::dialogue::Story::default(),
            original: crate::original_dialogue::State::default(),
            script_awards: std::collections::BTreeMap::new(),
            seed: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64
                | 1,
        }
    }
    fn roll(&mut self, min: i32, max: i32) -> i32 {
        // Local random source; not compatible with original saved RNG state.
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        min + (self.seed % ((max - min + 1) as u64)) as i32
    }
    pub fn status(&self, bag: &Bag) -> String {
        let ammo = bag
            .inventory
            .items
            .iter()
            .find(|i| Some(i.id) == bag.inventory.equipped)
            .filter(|i| i.ammo_capacity > 0)
            .map(|i| format!(" | Ammo {}/{}", i.ammo_quantity, i.ammo_capacity))
            .unwrap_or_default();
        if self.player.hp == 0 {
            return "You have died. Restart the rewrite to try again.".into();
        }
        format!(
            "HP {}/{} | AP {}/{}{} | {} | Click rat: attack | Space: end turn",
            self.player.hp,
            self.player.max_hp,
            self.player.ap,
            self.player.max_ap,
            ammo,
            if self.active { "COMBAT" } else { "Exploring" }
        )
    }
}
#[derive(Component)]
pub struct Rat(usize);
pub fn spawn(commands: &mut Commands, data: &SceneData, assets: &AssetServer) {
    for (i, a) in data.actors.iter().enumerate() {
        let f = &a.standing[a.rotation];
        let p = point(a.tile);
        commands.spawn((
            Rat(i),
            Sprite::from_image(assets.load(f.file.clone())),
            Transform::from_xyz(p.x + f.dx, p.y + f.h / 2. - f.dy, 10. - (p.y + 8.) / 10000.),
        ));
    }
}
/// Shortest geometric hex distance, independent of blocked terrain.
pub fn distance(a: i32, b: i32) -> i32 {
    crate::distance::hex_distance(a, b)
}
fn shoot(index: usize, data: &SceneData, fight: &mut Fight, bag: &mut Bag, motion: &mut Motion) {
    if motion.progress > 0. {
        motion.message = "Finish moving before attacking".into();
        return;
    }
    let item_index = bag
        .inventory
        .items
        .iter()
        .position(|i| Some(i.id) == bag.inventory.equipped);
    let spec = item_index
        .and_then(|i| {
            data.weapons
                .iter()
                .find(|w| w.pid == bag.inventory.items[i].pid)
        })
        .unwrap_or(&data.unarmed);
    let (ammo, capacity) = item_index
        .map(|i| {
            (
                bag.inventory.items[i].ammo_quantity,
                bag.inventory.items[i].ammo_capacity,
            )
        })
        .unwrap_or((0, 0));
    if spec.ap_cost < 1 {
        motion.message = "This item has no supported primary attack yet".into();
        return;
    }
    let mut weapon = spec.weapon(ammo, capacity);
    if crate::distance::hex_line(motion.tile, fight.rats[index].tile)
        .iter()
        .skip(1)
        .take_while(|t| **t != fight.rats[index].tile)
        .any(|t| data.shot_blocked[*t as usize])
    {
        motion.message = "A wall or scenery blocks the attack".into();
        return;
    }
    let d = distance(motion.tile, fight.rats[index].tile);
    let roll = fight.roll(1, 100);
    let damage = fight.roll(weapon.min_damage, weapon.max_damage);
    let profile = crate::damage::DamageProfile {
        threshold: data.actors[index].thresholds[spec.damage_type],
        resistance: data.actors[index].resistances[spec.damage_type],
        ammo_resistance_modifier: 0,
        ammo_multiplier: 1,
        ammo_divisor: 1,
        damage_multiplier: 2,
        bonus_damage: 0,
        difficulty_percent: 100,
    };
    match combat::attack_with_profile(
        &mut fight.player,
        &mut fight.rats[index],
        &mut weapon,
        d,
        roll,
        damage,
        Some(&profile),
    ) {
        Ok(result) => {
            fight.attack_time = 0.8;
            let toward = point(fight.rats[index].tile) - point(motion.tile);
            motion.facing = navigation::neighbors(motion.tile)
                .into_iter()
                .max_by(|(a, _), (b, _)| {
                    (point(*a) - point(motion.tile))
                        .normalize()
                        .dot(toward)
                        .total_cmp(&(point(*b) - point(motion.tile)).normalize().dot(toward))
                })
                .map(|(_, r)| r)
                .unwrap_or(motion.facing);
            fight.active = true;
            let id = fight.rats[index].id;
            if !fight.participants.contains(&id) {
                fight.participants.push(id);
            }
            fight.touched = true;
            motion.path.clear();
            if let Some(i) = item_index {
                bag.inventory.items[i].ammo_quantity = weapon.ammo;
                bag.dirty = true;
            }
            motion.message = if result.killed {
                format!("Rat killed ({} damage)", result.damage)
            } else if result.hit {
                format!(
                    "Rat hit for {} damage; {} HP left",
                    result.damage, fight.rats[index].hp
                )
            } else {
                "You missed the rat".into()
            };
            finish_encounter(fight);
        }
        Err(e) => motion.message = e,
    }
}
fn finish_encounter(fight: &mut Fight) {
    let opponent_alive = fight
        .rats
        .iter()
        .any(|r| r.hp > 0 && fight.participants.contains(&r.id));
    if !opponent_alive || fight.player.hp == 0 {
        fight.active = false;
        fight.participants.clear();
        if fight.player.hp > 0 {
            combat::end_turn(&mut fight.player);
        }
    }
}
fn rats_turn(data: &SceneData, fight: &mut Fight, motion: &mut Motion) {
    if !fight.active {
        return;
    }
    let mut messages = Vec::new();
    for i in 0..fight.rats.len() {
        if fight.rats[i].hp == 0 || fight.player.hp == 0 {
            continue;
        }
        // Provisional activation radius; original perception/script/AI scheduling is pending.
        if distance(fight.rats[i].tile, motion.tile) > 12
            && !fight.participants.contains(&fight.rats[i].id)
        {
            continue;
        }
        let id = fight.rats[i].id;
        if !fight.participants.contains(&id) {
            fight.participants.push(id);
        }
        fight.rats[i].ap = fight.rats[i].max_ap;
        let mut grid = data.walkable.clone();
        for a in &data.actors {
            grid[a.tile as usize] = true;
        }
        for r in &fight.rats {
            grid[r.tile as usize] = r.hp == 0;
        }
        grid[fight.rats[i].tile as usize] = true;
        let path = navigation::neighbors(motion.tile)
            .into_iter()
            .filter(|(t, _)| grid[*t as usize])
            .map(|(t, _)| navigation::route(fight.rats[i].tile, t, &grid))
            .filter(|r| !r.is_empty())
            .min_by_key(|r| r.len())
            .unwrap_or_default();
        for next in path {
            if distance(fight.rats[i].tile, motion.tile) <= 1 || fight.rats[i].ap == 0 {
                break;
            }
            fight.rats[i].tile = next;
            fight.rats[i].ap -= 1;
        }
        while distance(fight.rats[i].tile, motion.tile) <= 1
            && fight.rats[i].ap >= 3
            && fight.player.hp > 0
        {
            let a = &data.actors[i];
            let mut weapon = Weapon {
                min_damage: 1,
                max_damage: a.melee_damage + 2,
                range: 1,
                ap_cost: 3,
                skill: a.skill,
                ammo: 0,
                capacity: 0,
                ranged: false,
                perception: 1,
            };
            let roll = fight.roll(1, 100);
            let damage = fight.roll(1, weapon.max_damage);
            if let Ok(out) = combat::attack(
                &mut fight.rats[i],
                &mut fight.player,
                &mut weapon,
                1,
                roll,
                damage,
            ) {
                messages.push(if out.hit {
                    format!("Rat bites for {}", out.damage)
                } else {
                    "Rat misses".into()
                });
            } else {
                break;
            }
        }
    }
    finish_encounter(fight);
    combat::end_turn(&mut fight.player);
    motion.message = if messages.is_empty() {
        "Your turn".into()
    } else {
        messages.join("; ")
    };
}
pub fn actions(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    data: Res<SceneData>,
    mut fight: ResMut<Fight>,
    mut bag: ResMut<Bag>,
    mut motion: ResMut<Motion>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), Without<Player>>,
) {
    if bag.open || bag.block_frame || fight.player.hp == 0 || fight.attack_time > 0. {
        return;
    }
    if keys.just_pressed(KeyCode::Space) && fight.active && motion.progress == 0. {
        motion.path.clear();
        rats_turn(&data, &mut fight, &mut motion);
        bag.block_frame = true;
    }
    if buttons.just_pressed(MouseButton::Left) {
        if let Some(cursor) = window.cursor_position() {
            if cursor.y < 105. || cursor.y > window.height() - 100. {
                return;
            }
            if let Ok(p) = camera.0.viewport_to_world_2d(camera.1, cursor) {
                let target = fight
                    .rats
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.hp > 0)
                    .filter_map(|(i, r)| {
                        let f = &data.actors[i].standing[data.actors[i].rotation];
                        let center = point(r.tile) + Vec2::new(f.dx, f.h / 2. - f.dy);
                        (p.x - center.x).abs().le(&24.).then_some((i, center))
                    })
                    .filter(|(_, center)| (p.y - center.y).abs() < 24.)
                    .min_by(|(_, a), (_, b)| {
                        a.distance_squared(p).total_cmp(&b.distance_squared(p))
                    })
                    .map(|(i, _)| i);
                if let Some(i) = target {
                    bag.block_frame = true;
                    shoot(i, &data, &mut fight, &mut bag, &mut motion);
                }
            }
        }
    }
}
pub fn render(
    data: Res<SceneData>,
    fight: Res<Fight>,
    assets: Res<AssetServer>,
    mut rats: Query<(&Rat, &mut Sprite, &mut Transform)>,
    mut hp: Single<&mut Text, With<crate::HitPoints>>,
) {
    **hp = Text::new(format!("{:03}", fight.player.hp));
    for (Rat(i), mut sprite, mut transform) in &mut rats {
        let actor = &data.actors[*i];
        let state = &fight.rats[*i];
        let f = if state.hp == 0 {
            &actor.dead[actor.rotation]
        } else {
            &actor.standing[actor.rotation]
        };
        let p = point(state.tile);
        sprite.image = assets.load(f.file.clone());
        transform.translation = Vec3::new(
            p.x + f.dx,
            p.y + f.h / 2. - f.dy,
            if state.hp == 0 {
                1.
            } else {
                10. - (p.y + 8.) / 10000.
            },
        );
    }
}
pub fn smoke(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<SceneData>,
    mut fight: ResMut<Fight>,
    mut bag: ResMut<Bag>,
    mut motion: ResMut<Motion>,
    mut stage: Local<u8>,
    mut exit: MessageWriter<AppExit>,
) {
    if !std::env::args().any(|a| a == "--combat-smoke") {
        return;
    }
    if *stage == 0 && time.elapsed_secs() > 3. {
        let i = bag
            .inventory
            .items
            .iter()
            .position(|i| i.ammo_capacity > 0)
            .unwrap();
        bag.inventory.select(i);
        bag.inventory.equip_selected().unwrap();
        bag.dirty = true;
        let target = fight
            .rats
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                distance(motion.tile, r.tile) <= 25
                    && crate::distance::hex_line(motion.tile, r.tile)
                        .iter()
                        .skip(1)
                        .take_while(|t| **t != r.tile)
                        .all(|t| !data.shot_blocked[*t as usize])
            })
            .min_by_key(|(_, r)| distance(motion.tile, r.tile))
            .unwrap()
            .0;
        let ammo = bag.inventory.items[i].ammo_quantity;
        let ap = fight.player.ap;
        shoot(target, &data, &mut fight, &mut bag, &mut motion);
        assert_eq!(
            fight.player.ap,
            if fight.active {
                ap - 5
            } else {
                fight.player.max_ap
            }
        );
        assert_eq!(bag.inventory.items[i].ammo_quantity, ammo - 1);
        rats_turn(&data, &mut fight, &mut motion);
        assert_eq!(fight.player.ap, fight.player.max_ap);
        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/combat-save-smoke.json");
        save_checkpoint(file, &data, &fight, &bag, &motion).unwrap();
        let saved_player = fight.player.clone();
        let saved_rats = fight.rats.clone();
        let saved_ammo = bag.inventory.items[i].ammo_quantity;
        fight.player.hp = 1;
        fight.rats[0].hp = 0;
        bag.inventory.items[i].ammo_quantity = 0;
        load_checkpoint(file, &data, &mut fight, &mut bag, &mut motion).unwrap();
        assert_eq!(fight.player, saved_player);
        assert_eq!(fight.rats, saved_rats);
        assert_eq!(bag.inventory.items[i].ammo_quantity, saved_ammo);
        *stage = 1;
    }
    if *stage == 1 && time.elapsed_secs() > 6. {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/combat-smoke.png"
            )));
        *stage = 2;
    }
    if time.elapsed_secs() > 9. {
        exit.write(AppExit::Success);
    }
}

#[derive(Deserialize, Serialize)]
struct Checkpoint {
    schema: u32,
    #[serde(default)]
    story: crate::dialogue::Story,
    #[serde(default)]
    original: crate::original_dialogue::State,
    #[serde(default)]
    script_awards: std::collections::BTreeMap<i32, i32>,
    #[serde(default)]
    participants: Vec<i32>,
    seed: u64,
    inventory: Vec<ItemState>,
    looted: Vec<i32>,
    #[serde(default)]
    touched: bool,
    position: crate::save::SaveGame,
    combat: crate::save::CombatSnapshot,
}
fn checkpoint(
    data: &SceneData,
    fight: &Fight,
    bag: &Bag,
    motion: &Motion,
) -> Result<Checkpoint, String> {
    let position = crate::save::SaveGame {
        schema: 1,
        tile: motion.tile,
        facing: motion.facing,
        equipped: bag.inventory.equipped,
    };
    let combat = crate::save::CombatSnapshot {
        player: fight.player.clone(),
        rats: fight.rats.clone(),
        active: fight.active,
        ammo: bag
            .inventory
            .items
            .iter()
            .filter(|i| i.ammo_capacity > 0)
            .map(|i| (i.id, i.ammo_quantity))
            .collect(),
    };
    let checkpoint = Checkpoint {
        schema: 3,
        story: fight.story.clone(),
        original: fight.original.clone(),
        script_awards: fight.script_awards.clone(),
        participants: fight.participants.clone(),
        seed: fight.seed,
        inventory: bag
            .inventory
            .items
            .iter()
            .map(|i| ItemState {
                id: i.id,
                quantity: i.quantity,
                ammo: i.ammo_quantity,
                ammo_pid: i.ammo_pid,
            })
            .collect(),
        looted: fight.looted.clone(),
        touched: fight.touched,
        position,
        combat,
    };
    validate_checkpoint(&checkpoint, data, bag)?;
    Ok(checkpoint)
}
fn validate_checkpoint(c: &Checkpoint, data: &SceneData, bag: &Bag) -> Result<(), String> {
    c.story.validate(&data.dialogue_definition)?;
    c.original.validate()?;
    if c.original.loading {
        return Err("Cannot save while the original dialogue worker is loading".into());
    }
    for (item_id, quantity) in &c.script_awards {
        if !data.inventory.iter().any(|item| item.id == *item_id)
            || *quantity < 0
            || *quantity > 1_000_000
        {
            return Err("Invalid original-script inventory allowance".into());
        }
    }
    if c.schema != 3 {
        return Err("Unsupported checkpoint version".into());
    }
    if c.seed == 0 {
        return Err("Invalid random state".into());
    }
    let mut participant_ids = std::collections::HashSet::new();
    for id in &c.participants {
        if !participant_ids.insert(*id) || !data.actors.iter().any(|a| a.id == *id) {
            return Err("Invalid encounter participant".into());
        }
    }
    let initial = Fight::new(data);
    let caps = bag
        .inventory
        .items
        .iter()
        .filter(|i| i.ammo_capacity > 0)
        .map(|i| (i.id, i.ammo_capacity))
        .collect::<Vec<_>>();
    c.combat.validate(&initial.player, &initial.rats, &caps)?;
    if c.inventory.len() != data.inventory.len() {
        return Err("Inventory roster mismatch".into());
    }
    let mut seen = std::collections::HashSet::new();
    for id in &c.looted {
        if !seen.insert(*id) || !data.loot_containers.iter().any(|x| x.id == *id) {
            return Err("Invalid looted container".into());
        }
    }
    for (state, item) in c.inventory.iter().zip(&data.inventory) {
        let gained = data
            .loot_containers
            .iter()
            .filter(|x| c.looted.contains(&x.id))
            .flat_map(|x| &x.contents)
            .filter(|x| x.item_id == item.id)
            .map(|x| i64::from(x.quantity))
            .sum::<i64>();
        let script_award = i64::from(c.script_awards.get(&item.id).copied().unwrap_or(0));
        if state.id != item.id
            || state.quantity < 0
            || i64::from(state.quantity) > i64::from(item.quantity) + gained + script_award
        {
            return Err("Invalid inventory quantity or identity".into());
        }
        let cap = if item.item_type == 4 {
            item.ammo_pack_size
        } else {
            item.ammo_capacity
        };
        if state.ammo < 0
            || state.ammo > cap
            || (item.item_type == 4 && state.quantity > 0 && state.ammo == 0)
        {
            return Err("Invalid ammunition packet".into());
        }
        if item.ammo_capacity > 0
            && c.combat
                .ammo
                .iter()
                .find(|(id, _)| *id == item.id)
                .map(|(_, a)| *a)
                != Some(state.ammo)
        {
            return Err("Ammunition records disagree".into());
        }
        if item.ammo_capacity > 0
            && state.ammo > 0
            && !data
                .inventory
                .iter()
                .any(|a| a.item_type == 4 && a.pid == state.ammo_pid && a.caliber == item.caliber)
        {
            return Err("Invalid loaded ammunition type".into());
        }
        if c.position.equipped == Some(item.id) && state.quantity == 0 {
            return Err("Equipped item is depleted".into());
        }
    }
    if c.combat
        .rats
        .iter()
        .zip(&initial.rats)
        .any(|(a, b)| a.id != b.id)
    {
        return Err("Rat order does not match this map".into());
    }
    if c.position.tile != c.combat.player.tile {
        return Err("Player positions disagree".into());
    }
    let mut grid = data.walkable.clone();
    for r in &initial.rats {
        grid[r.tile as usize] = true;
    }
    let ids = bag
        .inventory
        .items
        .iter()
        .filter(|i| i.item_type == 3)
        .map(|i| i.id)
        .collect::<Vec<_>>();
    c.position.validate(&grid, &ids)?;
    if c.combat
        .rats
        .iter()
        .any(|r| r.hp > 0 && r.tile == c.position.tile)
    {
        return Err("Player overlaps a living rat".into());
    }
    Ok(())
}
pub fn save_checkpoint(
    path: &str,
    data: &SceneData,
    fight: &Fight,
    bag: &Bag,
    motion: &Motion,
) -> Result<(), String> {
    let c = checkpoint(data, fight, bag, motion)?;
    let temp = format!("{path}.tmp");
    std::fs::write(&temp, serde_json::to_string(&c).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    std::fs::rename(&temp, path).map_err(|e| e.to_string())
}
pub fn load_checkpoint(
    path: &str,
    data: &SceneData,
    fight: &mut Fight,
    bag: &mut Bag,
    motion: &mut Motion,
) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let c: Checkpoint = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    validate_checkpoint(&c, data, bag)?;
    // Apply only after the complete snapshot passes validation.
    fight.story = c.story;
    fight.original = c.original;
    fight.script_awards = c.script_awards;
    fight.seed = c.seed;
    fight.attack_time = 0.;
    fight.player = c.combat.player;
    fight.rats = c.combat.rats;
    fight.active = c.combat.active;
    fight.participants = if fight.active && c.participants.is_empty() {
        fight
            .rats
            .iter()
            .filter(|r| r.hp > 0)
            .map(|r| r.id)
            .collect()
    } else {
        c.participants
    };
    fight.touched = c.touched;
    fight.looted = c.looted;
    for (item, state) in bag.inventory.items.iter_mut().zip(c.inventory) {
        item.quantity = state.quantity;
        item.ammo_quantity = state.ammo;
        item.ammo_pid = state.ammo_pid;
    }
    for (id, ammo) in c.combat.ammo {
        bag.inventory
            .items
            .iter_mut()
            .find(|i| i.id == id)
            .unwrap()
            .ammo_quantity = ammo;
    }
    bag.inventory.equipped = c.position.equipped;
    bag.dirty = true;
    motion.tile = c.position.tile;
    motion.from = motion.tile;
    motion.facing = c.position.facing;
    motion.progress = 0.;
    motion.path.clear();
    Ok(())
}

#[derive(Deserialize)]
pub struct LootContainer {
    pub id: i32,
    pub tile: i32,
    pub name: String,
    pub contents: Vec<LootSlot>,
}
#[derive(Deserialize)]
pub struct LootSlot {
    pub item_id: i32,
    pub quantity: i32,
}
fn reload(fight: &mut Fight, bag: &mut Bag) -> Result<i32, String> {
    let cost = if fight.active && !bag.open { 2 } else { 0 };
    if fight.player.ap < cost {
        return Err("Reloading requires 2 AP".into());
    }
    let selected = bag.inventory.selected;
    let rounds = if bag.open
        && bag
            .inventory
            .items
            .get(selected)
            .is_some_and(|i| i.item_type == 4)
    {
        bag.inventory.reload_from(selected)?
    } else {
        bag.inventory.reload_equipped()?
    };
    fight.player.ap -= cost;
    fight.touched = true;
    bag.dirty = true;
    Ok(rounds)
}
fn heal(fight: &mut Fight, bag: &mut Bag) -> Result<i32, String> {
    let i = bag.inventory.selected;
    let item = bag.inventory.items.get(i).ok_or("No selected item")?;
    if item.heal_max <= 0 {
        return Err("Selected item cannot be used as healing".into());
    }
    let roll = fight.roll(item.heal_min, item.heal_max);
    let restored = bag
        .inventory
        .consume_healing(i, roll, fight.player.hp, fight.player.max_hp)?;
    fight.player.hp += restored;
    fight.touched = true;
    bag.dirty = true;
    Ok(restored)
}
fn take_loot(
    data: &SceneData,
    fight: &mut Fight,
    bag: &mut Bag,
    tile: i32,
) -> Result<String, String> {
    let c = data
        .loot_containers
        .iter()
        .min_by_key(|c| distance(tile, c.tile))
        .ok_or("No container here")?;
    if distance(tile, c.tile) > 1
        && fight
            .rats
            .iter()
            .any(|r| r.hp == 0 && distance(tile, r.tile) <= 1)
    {
        return Err("The rat has nothing to take".into());
    }
    if distance(tile, c.tile) > 1 {
        return Err(format!("Move next to {} to search it", c.name));
    }
    if fight.looted.contains(&c.id) {
        return Err(format!("{} is empty", c.name));
    }
    let weight = c
        .contents
        .iter()
        .map(|s| {
            bag.inventory
                .items
                .iter()
                .find(|i| i.id == s.item_id)
                .unwrap()
                .weight
                * s.quantity
        })
        .sum::<i32>();
    if bag.inventory.weight() + weight > data.carry_weight {
        return Err("You cannot carry that much".into());
    }
    let mut names = Vec::new();
    for slot in &c.contents {
        let item = bag
            .inventory
            .items
            .iter_mut()
            .find(|i| i.id == slot.item_id)
            .unwrap();
        if item.quantity == 0 && item.item_type == 4 {
            item.ammo_quantity = item.ammo_pack_size;
        }
        item.quantity += slot.quantity;
        names.push(format!("{} x{}", item.name, slot.quantity));
    }
    fight.looted.push(c.id);
    fight.touched = true;
    bag.dirty = true;
    Ok(format!("Found {}", names.join(", ")))
}
pub fn item_actions(
    keys: Res<ButtonInput<KeyCode>>,
    data: Res<SceneData>,
    mut fight: ResMut<Fight>,
    mut bag: ResMut<Bag>,
    mut motion: ResMut<Motion>,
) {
    if fight.player.hp == 0 || motion.progress > 0. || fight.attack_time > 0. {
        return;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        motion.message = match reload(&mut fight, &mut bag) {
            Ok(n) => format!("Reloaded {n} rounds"),
            Err(e) => e,
        };
        bag.notice = motion.message.clone();
    }
    if bag.open && keys.just_pressed(KeyCode::KeyX) {
        motion.message = match bag.inventory.unload_equipped() {
            Ok(n) => {
                bag.dirty = true;
                fight.touched = true;
                format!("Unloaded {n} rounds")
            }
            Err(e) => e,
        };
        bag.notice = motion.message.clone();
    }
    if bag.open && keys.just_pressed(KeyCode::KeyU) {
        motion.message = match heal(&mut fight, &mut bag) {
            Ok(n) => format!("Recovered {n} HP"),
            Err(e) => e,
        };
        bag.notice = motion.message.clone();
        bag.dirty = true;
    }
    if !bag.open && keys.just_pressed(KeyCode::KeyG) {
        motion.message = match take_loot(&data, &mut fight, &mut bag, motion.tile) {
            Ok(text) => text,
            Err(e) => e,
        };
    }
}

pub fn items_smoke(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<SceneData>,
    mut fight: ResMut<Fight>,
    mut bag: ResMut<Bag>,
    mut motion: ResMut<Motion>,
    mut stage: Local<u8>,
    mut exit: MessageWriter<AppExit>,
) {
    if !std::env::args().any(|a| a == "--items-smoke") {
        return;
    }
    if *stage == 0 && time.elapsed_secs() > 3. {
        let gun = bag
            .inventory
            .items
            .iter()
            .position(|i| i.ammo_capacity > 0)
            .unwrap();
        bag.inventory.select(gun);
        bag.inventory.equip_selected().unwrap();
        bag.inventory.items[gun].ammo_quantity = 0;
        let ammo = bag
            .inventory
            .items
            .iter()
            .position(|i| i.item_type == 4 && i.quantity > 0)
            .unwrap();
        let before = bag.inventory.items[ammo].ammo_quantity;
        fight.active = true;
        fight.player.ap = fight.player.max_ap;
        assert_eq!(reload(&mut fight, &mut bag).unwrap(), 12);
        assert_eq!(fight.player.ap, fight.player.max_ap - 2);
        assert_eq!(bag.inventory.items[ammo].ammo_quantity, before - 12);
        fight.player.hp = fight.player.max_hp - 15;
        let stim = bag
            .inventory
            .items
            .iter()
            .position(|i| i.heal_max > 0)
            .unwrap();
        bag.inventory.select(stim);
        let count = bag.inventory.items[stim].quantity;
        let restored = heal(&mut fight, &mut bag).unwrap();
        assert!((10..=15).contains(&restored));
        assert_eq!(bag.inventory.items[stim].quantity, count - 1);
        let c = &data.loot_containers[0];
        let tile = navigation::neighbors(c.tile)
            .into_iter()
            .find(|(t, _)| data.walkable[*t as usize])
            .unwrap()
            .0;
        motion.tile = tile;
        motion.from = tile;
        fight.player.tile = tile;
        take_loot(&data, &mut fight, &mut bag, tile).unwrap();
        assert!(
            take_loot(&data, &mut fight, &mut bag, tile)
                .unwrap_err()
                .contains("empty")
        );
        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/items-save-smoke.json");
        let expected = bag.inventory.items.clone();
        let health = fight.player.hp;
        save_checkpoint(file, &data, &fight, &bag, &motion).unwrap();
        bag.inventory.items[stim].quantity = 0;
        fight.looted.clear();
        fight.player.hp = 1;
        load_checkpoint(file, &data, &mut fight, &mut bag, &mut motion).unwrap();
        assert_eq!(bag.inventory.items, expected);
        assert_eq!(fight.player.hp, health);
        assert_eq!(fight.looted, vec![c.id]);
        bag.open = true;
        bag.dirty = true;
        bag.notice = "Reloaded, healed, looted and restored checkpoint".into();
        *stage = 1;
    }
    if *stage == 1 && time.elapsed_secs() > 6. {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/items-smoke.png"
            )));
        *stage = 2;
    }
    if time.elapsed_secs() > 9. {
        exit.write(AppExit::Success);
    }
}

#[derive(Serialize, Deserialize)]
struct ItemState {
    id: i32,
    quantity: i32,
    ammo: i32,
    ammo_pid: i32,
}

#[cfg(test)]
#[path = "checkpoint_tests.rs"]
mod checkpoint_tests;

/// Full local player-journey check. Dialogue advances through the same Bevy
/// button interactions as normal play; state assertions cover system boundaries.
pub fn journey_smoke(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<SceneData>,
    mut fight: ResMut<Fight>,
    mut bag: ResMut<Bag>,
    mut motion: ResMut<Motion>,
    mut stage: Local<u8>,
    mut last_click: Local<f32>,
    mut choices: Query<(&crate::dialogue_ui::DialogueChoice, &mut Interaction)>,
    mut exit: MessageWriter<AppExit>,
) {
    if !std::env::args().any(|a| a == "--journey-smoke") {
        return;
    }
    let t = time.elapsed_secs();
    if *stage == 0 && t > 3. {
        fight.story.begin(&data.dialogue_definition).unwrap();
        motion.path.clear();
        *stage = 1;
    }
    if *stage == 1 && t > 4. {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/briefing-smoke.png"
            )));
        let file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/briefing-save-smoke.json"
        );
        let expected = fight.story.clone();
        save_checkpoint(file, &data, &fight, &bag, &motion).unwrap();
        fight.story = crate::dialogue::Story::default();
        load_checkpoint(file, &data, &mut fight, &mut bag, &mut motion).unwrap();
        assert_eq!(fight.story, expected);
        *stage = 2;
    }
    if *stage == 2 {
        if fight.story.active.is_some() {
            if t - *last_click <= 0.25 {
                return;
            }
            if let Some((choice, mut interaction)) = choices
                .iter_mut()
                .find(|(c, _)| Some(&c.node) == fight.story.active.as_ref())
            {
                assert!(!choice.choice.is_empty());
                *interaction = Interaction::Pressed;
                *last_click = t;
            }
        } else {
            assert_eq!(fight.story.flags.get("water_chip_search"), Some(&1));
            assert!(fight.story.completed.contains(&data.dialogue_definition.id));
            assert!(bag.block_frame, "Closing dialogue consumes its final click");
            assert!(motion.path.is_empty());
            assert_eq!(motion.progress, 0.);
            *stage = 3;
        }
    }
    if *stage == 3 {
        let gun = bag
            .inventory
            .items
            .iter()
            .position(|i| i.ammo_capacity > 0)
            .unwrap();
        bag.open = true;
        bag.inventory.select(gun);
        bag.inventory.equip_selected().unwrap();
        bag.dirty = true;
        bag.open = false;
        let target = fight
            .rats
            .iter()
            .enumerate()
            .filter(|(_, r)| r.hp > 0 && distance(motion.tile, r.tile) <= 25)
            .filter(|(_, r)| {
                crate::distance::hex_line(motion.tile, r.tile)
                    .iter()
                    .skip(1)
                    .take_while(|t| **t != r.tile)
                    .all(|t| !data.shot_blocked[*t as usize])
            })
            .min_by_key(|(_, r)| distance(motion.tile, r.tile))
            .unwrap()
            .0;
        let rounds = bag.inventory.items[gun].ammo_quantity;
        shoot(target, &data, &mut fight, &mut bag, &mut motion);
        assert_eq!(bag.inventory.items[gun].ammo_quantity, rounds - 1);
        rats_turn(&data, &mut fight, &mut motion);
        let file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/journey-save-smoke.json"
        );
        let story = fight.story.clone();
        let rats = fight.rats.clone();
        let hp = fight.player.hp;
        save_checkpoint(file, &data, &fight, &bag, &motion).unwrap();
        fight.story = crate::dialogue::Story::default();
        fight.player.hp = 1;
        bag.inventory.items[gun].ammo_quantity = 0;
        load_checkpoint(file, &data, &mut fight, &mut bag, &mut motion).unwrap();
        assert_eq!(fight.story, story);
        assert_eq!(fight.rats, rats);
        assert_eq!(fight.player.hp, hp);
        assert_eq!(bag.inventory.items[gun].ammo_quantity, rounds - 1);
        *stage = 4;
    }
    if *stage == 4 && t > 9. {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/journey-smoke.png"
            )));
        *stage = 5;
    }
    if *stage == 5 && t > 12. {
        exit.write(AppExit::Success);
    }
    if t > 20. {
        panic!(
            "Journey stalled at stage {} node {:?}",
            *stage, fight.story.active
        );
    }
}
