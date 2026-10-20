//! Map traversal prototype. Original map-controller and world-travel scheduling
//! are not yet connected; transitions preserve the current character and inventory.
use crate::{
    Motion, SceneData,
    combat_ui::{self, Fight},
    inventory_ui::Bag,
    point,
};
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Deserialize)]
pub struct ExitGrid {
    pub tile: i32,
    pub map: i32,
    pub destination_tile: i32,
    pub elevation: i32,
    pub rotation: usize,
}
#[derive(Component)]
pub struct MapEntity;
#[derive(Component)]
pub struct WorldPanel;
#[derive(Component)]
pub struct Destination(&'static str);
#[derive(Clone)]
struct Visit {
    fight: Fight,
    motion: Motion,
}
#[derive(Default, Resource)]
pub struct Travel {
    pub open: bool,
    request: Option<(String, Option<(i32, usize)>)>,
    visited: HashMap<String, Visit>,
    suppressed: Option<(String, i32)>,
    pending: Option<Prepared>,
}
struct Prepared {
    data: SceneData,
    fight: Fight,
    bag: Bag,
    motion: Motion,
    visited: HashMap<String, Visit>,
}
pub fn in_map(travel: Res<Travel>) -> bool {
    !travel.open
}
fn map_name(number: i32) -> Option<&'static str> {
    match number {
        35 => Some("v13ent"),
        6 => Some("vault13"),
        25 => Some("shadye"),
        26 => Some("shadyw"),
        _ => None,
    }
}
fn read_scene(name: &str) -> Result<SceneData, String> {
    if !["v13ent", "vault13", "shadyw", "shadye"].contains(&name) {
        return Err("This destination is not prepared yet".into());
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(format!("scene-{name}.json"));
    let text = std::fs::read_to_string(path)
        .map_err(|_| format!("Prepare the {name} map before travelling"))?;
    let data: SceneData = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if map_name(data.map_number) != Some(name)
        || data.map_name != name
        || data.walkable.len() != 40000
        || data.spawn_tile < 0
        || data.spawn_tile >= 40000
        || !data.walkable[data.spawn_tile as usize]
    {
        return Err("Invalid destination map".into());
    }
    Ok(data)
}
fn world_panel(commands: &mut Commands, data: &SceneData, assets: &AssetServer) {
    commands.spawn((WorldPanel,GlobalZIndex(200),Node {position_type:PositionType::Absolute,
        width:percent(100),height:percent(100),align_items:AlignItems::Center,justify_content:JustifyContent::Center,..default()},
        BackgroundColor(Color::BLACK))).with_children(|root| {
        root.spawn((Node {width:px(640),height:px(480),..default()},BackgroundColor(Color::srgb(0.06,0.07,0.055))))
            .with_children(|panel| {
                panel.spawn(Node {position_type:PositionType::Absolute,left:px(15),top:px(25),width:px(450),height:px(440),overflow:Overflow::clip(),..default()})
                    .with_children(|view| {view.spawn((ImageNode::new(assets.load(data.world_map.file.clone())),Node {
                        position_type:PositionType::Absolute,left:px(-650),top:px(0),width:px(1400),height:px(data.world_map.h),..default()}));});
                panel.spawn((Text::new("WASTELAND"),TextFont {font_size:16.,..default()},TextColor(Color::srgb(0.3,1.,0.3)),
                    Node {position_type:PositionType::Absolute,left:px(20),top:px(3),..default()}));
                for (i,(name,label)) in [("v13ent","Vault 13 cave"),("vault13","Vault 13 interior"),("shadyw","Shady Sands west"),("shadye","Shady Sands east")].into_iter().enumerate() {
                    panel.spawn((Button,Destination(name),Node {position_type:PositionType::Absolute,left:px(480),top:px(50+i*65),width:px(150),height:px(48),padding:UiRect::all(px(7)),..default()},
                        BackgroundColor(Color::srgb(0.12,0.16,0.1)))).with_children(|button| {button.spawn((Text::new(label),TextFont {font_size:14.,..default()},TextColor(Color::srgb(0.4,1.,0.35))));});
                }
                panel.spawn((Text::new("Travel prototype\nNo encounters or elapsed travel time yet.\n\nEsc returns to your current map."),TextFont {font_size:12.,..default()},TextColor(Color::srgb(0.45,0.8,0.4)),
                    Node {position_type:PositionType::Absolute,left:px(480),top:px(330),width:px(150),..default()}));
            });
    });
}
pub fn buttons(
    mut travel: ResMut<Travel>,
    buttons: Query<(&Interaction, &Destination), Changed<Interaction>>,
) {
    if !travel.open {
        return;
    }
    for (interaction, destination) in &buttons {
        if *interaction == Interaction::Pressed {
            travel.request = Some((destination.0.into(), None));
        }
    }
}
pub fn update(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut travel: ResMut<Travel>,
    mut data: ResMut<SceneData>,
    mut fight: ResMut<Fight>,
    mut bag: ResMut<Bag>,
    mut motion: ResMut<Motion>,
    assets: Res<AssetServer>,
    entities: Query<Entity, With<MapEntity>>,
    panels: Query<Entity, With<WorldPanel>>,
    mut camera: Single<&mut Transform, With<Camera>>,
) {
    if let Some(saved) = travel.pending.take() {
        for entity in &entities {
            commands.entity(entity).despawn();
        }
        for entity in &panels {
            commands.entity(entity).despawn();
        }
        for sprite in &saved.data.sprites {
            commands.spawn((
                MapEntity,
                Sprite::from_image(assets.load(sprite.file.clone())),
                Transform::from_xyz(sprite.x, -sprite.y, sprite.z),
            ));
        }
        combat_ui::spawn(&mut commands, &saved.data, &assets);
        *data = saved.data;
        *fight = saved.fight;
        *bag = saved.bag;
        *motion = saved.motion;
        motion.message = "Campaign loaded".into();
        bag.block_frame = true;
        bag.open = false;
        travel.visited = saved.visited;
        travel.open = false;
        travel.request = None;
        travel.suppressed = Some((data.map_name.clone(), motion.tile));
        let p = point(motion.tile);
        camera.translation.x = p.x;
        camera.translation.y = p.y;
        return;
    }
    if fight.original.open || fight.original.loading || fight.story.active.is_some() {
        return;
    }
    if travel.open && keys.just_pressed(KeyCode::Escape) {
        travel.open = false;
        for e in &panels {
            commands.entity(e).despawn();
        }
        travel.suppressed = Some((data.map_name.clone(), motion.tile));
        return;
    }
    // M is an explicit traversal-prototype shortcut until original travel scripts work.
    if !travel.open && keys.just_pressed(KeyCode::KeyM) && !bag.open && motion.progress == 0. {
        if fight.active {
            motion.message = "Finish combat before travelling".into();
        } else {
            travel.open = true;
            motion.path.clear();
            world_panel(&mut commands, &data, &assets);
        }
    }
    if !travel.open && motion.progress == 0. && travel.request.is_none() {
        let here = (data.map_name.clone(), motion.tile);
        if travel.suppressed.as_ref() != Some(&here) {
            travel.suppressed = None;
            if let Some(exit) = data.exit_grids.iter().find(|e| e.tile == motion.tile) {
                motion.path.clear();
                if fight.active {
                    motion.message = "Finish combat before travelling".into();
                    travel.suppressed = Some(here);
                } else if exit.elevation != 0 {
                    motion.message = "This elevation is not connected yet".into();
                    travel.suppressed = Some(here);
                } else if exit.map == -2 {
                    travel.open = true;
                    world_panel(&mut commands, &data, &assets);
                } else if let Some(name) = map_name(exit.map) {
                    travel.request =
                        Some((name.into(), Some((exit.destination_tile, exit.rotation))));
                } else {
                    motion.message = format!("Destination map {} is not connected yet", exit.map);
                    travel.suppressed = Some(here);
                }
            }
        }
    }
    let Some((name, entrance)) = travel.request.take() else {
        return;
    };
    let next = match read_scene(&name) {
        Ok(scene) => scene,
        Err(e) => {
            motion.message = e;
            return;
        }
    };
    if fight.active {
        motion.message = "Finish combat before travelling".into();
        return;
    }
    let player = fight.player.clone();
    travel.visited.insert(
        data.map_name.clone(),
        Visit {
            fight: fight.clone(),
            motion: motion.clone(),
        },
    );
    let restored = travel.visited.get(&name).cloned();
    let mut next_fight = restored
        .as_ref()
        .map(|v| v.fight.clone())
        .unwrap_or_else(|| Fight::new(&next));
    let tile = entrance
        .map(|e| e.0)
        .filter(|t| (0..40000).contains(t) && next.walkable[*t as usize])
        .or_else(|| restored.as_ref().map(|v| v.motion.tile))
        .unwrap_or(next.spawn_tile);
    next_fight.story = fight.story.clone();
    next_fight.original = fight.original.clone();
    next_fight.script_awards = fight.script_awards.clone();
    next_fight.looted = fight.looted.clone();
    next_fight.player = player;
    next_fight.player.tile = tile;
    next_fight.active = false;
    next_fight.participants.clear();
    next_fight.attack_time = 0.;
    *fight = next_fight;
    motion.tile = tile;
    motion.from = tile;
    motion.progress = 0.;
    motion.path.clear();
    motion.facing = entrance
        .map(|e| e.1)
        .filter(|r| *r < 6)
        .or_else(|| restored.as_ref().map(|v| v.motion.facing))
        .unwrap_or(2);
    motion.message = format!("Entered {name}");
    bag.block_frame = true;
    bag.open = false;
    bag.dirty = true;
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    for entity in &panels {
        commands.entity(entity).despawn();
    }
    for sprite in &next.sprites {
        commands.spawn((
            MapEntity,
            Sprite::from_image(assets.load(sprite.file.clone())),
            Transform::from_xyz(sprite.x, -sprite.y, sprite.z),
        ));
    }
    combat_ui::spawn(&mut commands, &next, &assets);
    *data = next;
    let p = point(tile);
    camera.translation.x = p.x;
    camera.translation.y = p.y;
    travel.open = false;
    travel.suppressed = Some((data.map_name.clone(), tile));
}

pub fn smoke(
    time: Res<Time>,
    mut travel: ResMut<Travel>,
    data: Res<SceneData>,
    mut fight: ResMut<Fight>,
    mut bag: ResMut<Bag>,
    mut motion: ResMut<Motion>,
    mut stage: Local<u8>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut choices: Query<(&Destination, &mut Interaction)>,
) {
    if std::env::args().any(|a| a == "--campaign-load-smoke") {
        let now = time.elapsed_secs();
        if *stage == 0 && now > 3. {
            load_campaign(
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/campaign-smoke-save.json"
                ),
                &mut travel,
            )
            .unwrap();
            *stage = 1;
        }
        if *stage == 1 && travel.pending.is_none() {
            assert_eq!(data.map_name, "vault13");
            assert_eq!(travel.visited.len(), 3);
            assert_eq!(fight.player.hp, data.player_combat.hp - 1);
            travel.request = Some(("v13ent".into(), None));
            *stage = 2;
        }
        if *stage == 2 && data.map_name == "v13ent" {
            assert_eq!(fight.rats[0].hp, 0);
            assert_eq!(fight.story.flags.get("water_chip_search"), Some(&1));
            let stim = bag.inventory.items.iter().find(|i| i.heal_min > 0).unwrap();
            assert_eq!(
                stim.quantity,
                data.inventory
                    .iter()
                    .find(|i| i.id == stim.id)
                    .unwrap()
                    .quantity
                    - 1
            );
            println!("Separate-process campaign restore and return visit passed");
            exit.write(AppExit::Success);
        }
        assert!(now < 20., "Fresh-process campaign restore timed out");
        return;
    }
    if !std::env::args().any(|a| a == "--travel-smoke") {
        return;
    }
    let now = time.elapsed_secs();
    if *stage == 0 && now > 3. {
        fight.rats[0].hp = 0;
        fight.player.hp -= 1;
        fight.story.flags.insert("water_chip_search".into(), 1);
        bag.inventory
            .items
            .iter_mut()
            .find(|i| i.heal_min > 0)
            .unwrap()
            .quantity -= 1;
        travel.request = Some(("vault13".into(), None));
        *stage = 1;
    }
    if *stage == 1 && data.map_name == "vault13" && now > 5. {
        assert_eq!(fight.player.hp, data.player_combat.hp - 1);
        travel.request = Some(("shadyw".into(), None));
        *stage = 2;
    }
    if *stage == 2 && data.map_name == "shadyw" && now > 7. {
        assert!(!data.actors.is_empty());
        assert!(data.actors.iter().all(|a| !a.hostile));
        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/travel-smoke-save.json");
        combat_ui::save_checkpoint(file, &data, &fight, &bag, &motion).unwrap();
        let hp = fight.player.hp;
        fight.player.hp = 1;
        combat_ui::load_checkpoint(file, &data, &mut fight, &mut bag, &mut motion).unwrap();
        assert_eq!(fight.player.hp, hp);
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/shady-sands-smoke.png"
            )));
        *stage = 3;
    }
    if *stage == 3 && now > 8. {
        travel.request = Some(("shadye".into(), None));
        *stage = 4;
    }
    if *stage == 4 && data.map_name == "shadye" && now > 10. {
        assert!(data.actors.iter().all(|a| !a.hostile));
        travel.request = Some(("v13ent".into(), None));
        *stage = 5;
    }
    if *stage == 5 && data.map_name == "v13ent" && now > 12. {
        assert_eq!(fight.rats[0].hp, 0);
        assert_eq!(fight.player.hp, data.player_combat.hp - 1);
        assert_eq!(fight.story.flags.get("water_chip_search"), Some(&1));
        let stim = bag.inventory.items.iter().find(|i| i.heal_min > 0).unwrap();
        assert_eq!(
            stim.quantity,
            data.inventory
                .iter()
                .find(|i| i.id == stim.id)
                .unwrap()
                .quantity
                - 1
        );
        println!(
            "Four-map travel, neutral NPCs, player/inventory/story/encounter persistence and destination checkpoint passed"
        );
        travel.open = true;
        world_panel(&mut commands, &data, &assets);
        *stage = 6;
    }
    if *stage == 6 && now > 14. {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/world-travel-smoke.png"
            )));
        *stage = 7;
    }
    if *stage == 7 && now > 15. {
        for (destination, mut interaction) in &mut choices {
            if destination.0 == "vault13" {
                *interaction = Interaction::Pressed;
            }
        }
        *stage = 8;
    }
    if *stage == 8 && data.map_name == "vault13" {
        assert!(!travel.open);
        println!("World menu destination button passed");
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/campaign-smoke-save.json"
        );
        save_campaign(path, &travel, &data, &fight, &bag, &motion).unwrap();
        let mut invalid: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        invalid["maps"]["shadyw"]["combat"]["rats"][0]["hp"] = serde_json::json!(-1);
        let bad = format!("{path}.invalid");
        std::fs::write(&bad, serde_json::to_vec(&invalid).unwrap()).unwrap();
        let count = travel.visited.len();
        assert!(load_campaign(&bad, &mut travel).is_err());
        assert_eq!(travel.visited.len(), count);
        assert!(travel.pending.is_none());
        *travel = Travel::default();
        fight.player.hp = 1;
        load_campaign(path, &mut travel).unwrap();
        *stage = 9;
    }
    if *stage == 9 && now > 17. && travel.pending.is_none() {
        assert_eq!(fight.player.hp, data.player_combat.hp - 1);
        assert_eq!(travel.visited.len(), 3);
        travel.request = Some(("v13ent".into(), None));
        *stage = 10;
    }
    if *stage == 10 && data.map_name == "v13ent" {
        assert_eq!(fight.rats[0].hp, 0);
        assert_eq!(fight.story.flags.get("water_chip_search"), Some(&1));
        println!(
            "Campaign restart restore, all visited maps and atomic corrupt-save rejection passed"
        );
        exit.write(AppExit::Success);
    }
    assert!(
        now < 35.,
        "Travel smoke timed out: {} stage {}",
        data.map_name,
        *stage
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_map_numbers_resolve_supported_destinations() {
        assert_eq!(map_name(35), Some("v13ent"));
        assert_eq!(map_name(6), Some("vault13"));
        assert_eq!(map_name(25), Some("shadye"));
        assert_eq!(map_name(26), Some("shadyw"));
        assert_eq!(map_name(-2), None);
    }
    #[test]
    fn destination_names_are_restricted_before_file_access() {
        assert!(read_scene("../other").is_err());
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Campaign {
    schema: u32,
    current: String,
    maps: std::collections::BTreeMap<String, serde_json::Value>,
}
fn initial_motion(data: &SceneData) -> Motion {
    Motion {
        tile: data.spawn_tile,
        from: data.spawn_tile,
        path: Default::default(),
        progress: 0.,
        facing: 2,
        clock: 0.,
        message: String::new(),
    }
}
fn campaign(
    travel: &Travel,
    data: &SceneData,
    fight: &Fight,
    bag: &Bag,
    motion: &Motion,
) -> Result<Campaign, String> {
    if fight.original.loading || motion.progress > 0. || travel.pending.is_some() {
        return Err("Finish the pending action before saving".into());
    }
    let mut maps = std::collections::BTreeMap::new();
    for (name, visit) in &travel.visited {
        if name == &data.map_name {
            continue;
        }
        let scene = read_scene(name)?;
        let mut local = visit.fight.clone();
        local.story = fight.story.clone();
        local.original = fight.original.clone();
        local.looted = fight.looted.clone();
        local.script_awards = fight.script_awards.clone();
        local.player = fight.player.clone();
        local.player.tile = visit.motion.tile;
        maps.insert(
            name.clone(),
            combat_ui::snapshot(&scene, &local, bag, &visit.motion)?,
        );
    }
    maps.insert(
        data.map_name.clone(),
        combat_ui::snapshot(data, fight, bag, motion)?,
    );
    Ok(Campaign {
        schema: 1,
        current: data.map_name.clone(),
        maps,
    })
}
pub fn save_campaign(
    path: &str,
    travel: &Travel,
    data: &SceneData,
    fight: &Fight,
    bag: &Bag,
    motion: &Motion,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(&campaign(travel, data, fight, bag, motion)?)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 80 * 1024 * 1024 {
        return Err("Campaign exceeds save size limit".into());
    }
    let temp = format!("{path}.tmp");
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, path).map_err(|e| e.to_string())
}
fn prepare_campaign(saved: Campaign) -> Result<Prepared, String> {
    if saved.schema != 1 || saved.maps.is_empty() || saved.maps.len() > 4 {
        return Err("Unsupported campaign save".into());
    }
    let current = saved
        .maps
        .get(&saved.current)
        .ok_or("Campaign current map is missing")?;
    let shared = ["inventory", "story", "original", "script_awards", "looted"];
    let mut visited = HashMap::new();
    let mut selected = None;
    for (name, snapshot) in &saved.maps {
        for key in shared {
            if snapshot.get(key) != current.get(key) {
                return Err(format!("Campaign shared state differs in {name}: {key}"));
            }
        }
        let scene = read_scene(name)?;
        let mut fight = Fight::new(&scene);
        let mut bag = Bag::new(scene.inventory.clone());
        let mut motion = initial_motion(&scene);
        combat_ui::restore_snapshot(snapshot.clone(), &scene, &mut fight, &mut bag, &mut motion)?;
        // Character health, AP and base stats are global; only placement differs per map.
        let mut player = snapshot["combat"]["player"].clone();
        let mut canonical = current["combat"]["player"].clone();
        player
            .as_object_mut()
            .ok_or("Invalid player record")?
            .remove("tile");
        canonical
            .as_object_mut()
            .ok_or("Invalid current player record")?
            .remove("tile");
        if player != canonical {
            return Err("Campaign character state differs between maps".into());
        }
        if name == &saved.current {
            selected = Some((scene, fight, bag, motion));
        } else {
            visited.insert(name.clone(), Visit { fight, motion });
        }
    }
    let (data, fight, bag, motion) = selected.ok_or("Missing current location")?;
    Ok(Prepared {
        data,
        fight,
        bag,
        motion,
        visited,
    })
}
pub fn load_campaign(path: &str, travel: &mut Travel) -> Result<(), String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.len() > 80 * 1024 * 1024 {
        return Err("Campaign exceeds save size limit".into());
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let saved = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    // Nothing in the live game changes until every location validates.
    travel.pending = Some(prepare_campaign(saved)?);
    Ok(())
}

#[cfg(test)]
mod campaign_tests {
    use super::*;
    #[test]
    fn campaign_rejects_version_empty_missing_current_and_unknown_paths() {
        assert!(
            prepare_campaign(Campaign {
                schema: 2,
                current: "v13ent".into(),
                maps: Default::default()
            })
            .is_err()
        );
        assert!(
            prepare_campaign(Campaign {
                schema: 1,
                current: "v13ent".into(),
                maps: Default::default()
            })
            .is_err()
        );
        let mut maps = std::collections::BTreeMap::new();
        maps.insert("../invalid".into(), serde_json::Value::Null);
        assert!(
            prepare_campaign(Campaign {
                schema: 1,
                current: "v13ent".into(),
                maps: maps.clone()
            })
            .is_err()
        );
        assert!(
            prepare_campaign(Campaign {
                schema: 1,
                current: "../invalid".into(),
                maps
            })
            .is_err()
        );
    }
}
