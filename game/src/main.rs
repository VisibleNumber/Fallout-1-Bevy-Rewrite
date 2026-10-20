use bevy::prelude::*;
use serde::Deserialize;
use std::collections::VecDeque;
mod inventory;
mod inventory_ui;
mod navigation;
mod save;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
#[derive(Deserialize)]
struct Frame {
    file: String,
    h: f32,
    dx: f32,
    dy: f32,
}
#[derive(Deserialize)]
struct UiArt {
    file: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}
#[derive(Deserialize)]
struct WeaponArt {
    pid: i32,
    standing: Vec<Frame>,
    walking: Vec<Vec<Frame>>,
}
#[derive(Deserialize)]
struct Placed {
    file: String,
    x: f32,
    y: f32,
    z: f32,
}
#[derive(Deserialize, Resource)]
struct SceneData {
    sprites: Vec<Placed>,
    ui: Vec<UiArt>,
    inventory: Vec<inventory::Item>,
    weapon_art: Vec<WeaponArt>,
    inventory_background: Frame,
    player_name: String,
    hit_points: i32,
    armor_class: i32,
    player: Vec<Frame>,
    walking: Vec<Vec<Frame>>,
    walkable: Vec<bool>,
    spawn_tile: i32,
}
#[derive(Component)]
struct Player;
#[derive(Component)]
struct Hud;
#[derive(Resource)]
struct Motion {
    tile: i32,
    from: i32,
    path: VecDeque<i32>,
    progress: f32,
    facing: usize,
    clock: f32,
    message: String,
}
fn point(tile: i32) -> Vec2 {
    let (x, y) = navigation::screen(tile);
    Vec2::new(x, y)
}
fn main() {
    let raw = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/scene.json"))
        .expect("Run node game/prepare.mjs first");
    let data: SceneData = serde_json::from_str(&raw).unwrap();
    let motion = Motion {
        tile: data.spawn_tile,
        from: data.spawn_tile,
        path: VecDeque::new(),
        progress: 0.,
        facing: 2,
        clock: 0.,
        message: String::new(),
    };
    let bag = inventory_ui::Bag::new(data.inventory.clone());
    App::new()
        .insert_resource(bag)
        .insert_resource(data)
        .insert_resource(motion)
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").into(),
                    ..default()
                })
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Fallout Rewrite | Vault 13".into(),
                        resolution: (1280, 960).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                inventory_ui::actions,
                save_input,
                input,
                walk,
                inventory_ui::refresh,
                inventory_smoke,
                smoke,
            )
                .chain(),
        )
        .run();
}
fn setup(mut commands: Commands, data: Res<SceneData>, assets: Res<AssetServer>) {
    let at = point(data.spawn_tile);
    commands.spawn((Camera2d, Transform::from_xyz(at.x, at.y, 1000.)));
    for s in &data.sprites {
        commands.spawn((
            Sprite::from_image(assets.load(s.file.clone())),
            Transform::from_xyz(s.x, -s.y, s.z),
        ));
    }
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: percent(50),
                margin: UiRect {
                    left: px(-320.),
                    ..default()
                },
                width: px(640),
                height: px(100),
                ..default()
            },
            GlobalZIndex(10),
        ))
        .with_children(|parent| {
            for art in &data.ui {
                parent.spawn((
                    ImageNode::new(assets.load(art.file.clone())),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(art.x),
                        top: px(art.y),
                        width: px(art.w),
                        height: px(art.h),
                        ..default()
                    },
                ));
            }
            for (y, value) in [(40., data.hit_points), (75., data.armor_class)] {
                parent
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(471),
                            top: px(y - 1.),
                            width: px(41),
                            height: px(20),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.04, 0.045, 0.035)),
                    ))
                    .with_children(|n| {
                        n.spawn((
                            Text::new(format!("{:03}", value)),
                            TextFont {
                                font_size: 15.,
                                ..default()
                            },
                            TextColor(Color::srgb(0.3, 1., 0.3)),
                        ));
                    });
            }
            parent.spawn((
                inventory_ui::EquippedIcon,
                ImageNode::default(),
                Visibility::Hidden,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(290),
                    top: px(28),
                    width: px(140),
                    height: px(58),
                    ..default()
                },
            ));
        });
    let f = &data.player[2];
    commands.spawn((
        Player,
        Sprite::from_image(assets.load(f.file.clone())),
        Transform::from_xyz(
            at.x + f.dx,
            at.y + f.h / 2. - f.dy,
            10. - (at.y + 8.) / 10000.,
        ),
    ));
    commands.spawn((
        Hud,
        Text::default(),
        TextFont {
            font_size: 16.,
            ..default()
        },
        TextColor(Color::srgb(0.3, 1., 0.3)),
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(12),
            padding: UiRect::all(px(8)),
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.85)),
    ));
}
fn input(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    data: Res<SceneData>,
    mut motion: ResMut<Motion>,
    bag: Res<inventory_ui::Bag>,
    window: Single<&Window>,
    mut camera: Single<(&Camera, &GlobalTransform, &mut Transform), Without<Player>>,
) {
    let dt = time.delta_secs().min(0.05);
    let (c, global, t) = &mut *camera;
    let mut d = Vec2::ZERO;
    for (key, delta) in [
        (KeyCode::KeyW, Vec2::Y),
        (KeyCode::KeyS, Vec2::NEG_Y),
        (KeyCode::KeyA, Vec2::NEG_X),
        (KeyCode::KeyD, Vec2::X),
    ] {
        if keys.pressed(key) {
            d += delta;
        }
    }
    t.translation += d.extend(0.) * dt * 300.;
    if keys.just_pressed(KeyCode::KeyR) {
        motion.tile = data.spawn_tile;
        motion.from = motion.tile;
        motion.path.clear();
        motion.progress = 0.;
        let p = point(motion.tile);
        t.translation.x = p.x;
        t.translation.y = p.y;
    }
    if bag.open || bag.block_frame {
        return;
    }
    if buttons.just_pressed(MouseButton::Left) {
        if let Some(cursor) = window.cursor_position() {
            if cursor.y < 105. || cursor.y > window.height() - 100. {
                return;
            }
            if let Ok(p) = c.viewport_to_world_2d(global, cursor) {
                let target = (0..40000)
                    .min_by(|a, b| {
                        point(*a)
                            .distance_squared(p)
                            .total_cmp(&point(*b).distance_squared(p))
                    })
                    .unwrap();
                let route = navigation::route(motion.tile, target, &data.walkable);
                motion.message = if route.is_empty() && target != motion.tile {
                    "No route".into()
                } else {
                    format!("Destination hex {}", target)
                };
                motion.path = route.into();
            }
        }
    }
}
fn walk(
    time: Res<Time>,
    data: Res<SceneData>,
    assets: Res<AssetServer>,
    mut m: ResMut<Motion>,
    mut player: Single<(&mut Transform, &mut Sprite), With<Player>>,
    mut hud: Single<&mut Text, With<Hud>>,
    bag: Res<inventory_ui::Bag>,
) {
    let dt = if bag.open {
        0.
    } else {
        time.delta_secs().min(0.05)
    };
    if m.progress == 0. && !bag.open {
        if let Some(next) = m.path.pop_front() {
            m.from = m.tile;
            m.facing = navigation::neighbors(m.tile)
                .iter()
                .find(|(tile, _)| *tile == next)
                .map(|(_, r)| *r)
                .unwrap_or(m.facing);
            m.tile = next;
            m.progress = 0.0001;
        }
    }
    let moving = m.progress > 0.;
    if moving {
        m.clock += dt;
        m.progress = (m.progress + dt / 0.3).min(1.);
    }
    let p = if moving {
        point(m.from).lerp(point(m.tile), m.progress)
    } else {
        point(m.tile)
    };
    let equipped = bag
        .inventory
        .items
        .iter()
        .find(|i| Some(i.id) == bag.inventory.equipped)
        .and_then(|i| data.weapon_art.iter().find(|a| a.pid == i.pid));
    let frames = equipped
        .map(|a| &a.walking[m.facing])
        .unwrap_or(&data.walking[m.facing]);
    let f = if moving {
        &frames[((m.clock * 10.) as usize) % frames.len()]
    } else {
        equipped
            .map(|a| &a.standing[m.facing])
            .unwrap_or(&data.player[m.facing])
    };
    let (t, s) = &mut *player;
    t.translation = Vec3::new(p.x + f.dx, p.y + f.h / 2. - f.dy, 10. - (p.y + 8.) / 10000.);
    s.image = assets.load(f.file.clone());
    if m.progress >= 1. {
        m.progress = 0.;
    }
    **hud = Text::new(format!(
        "FALLOUT REWRITE | Original artwork\nClick ground | WASD pan | I inventory | F5 save / F9 load\nHex {} | {} steps remaining | {}\nCombat and interactions not connected",
        m.tile,
        m.path.len(),
        m.message
    ));
}

fn smoke(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<SceneData>,
    mut motion: ResMut<Motion>,
    mut stage: Local<u8>,
    mut exit: MessageWriter<AppExit>,
) {
    if !std::env::args().any(|a| a == "--smoke") {
        return;
    }
    if *stage == 0 && time.elapsed_secs() > 2. {
        let target = (0..40000)
            .filter(|t| data.walkable[*t as usize])
            .find(|t| {
                let r = navigation::route(motion.tile, *t, &data.walkable);
                r.len() >= 6 && r.len() <= 9
            })
            .expect("reachable cave route");
        motion.path = navigation::route(motion.tile, target, &data.walkable).into();
        *stage = 1;
    }
    if *stage == 1 && time.elapsed_secs() > 9. {
        assert_eq!(motion.progress, 0.);
        assert!(motion.path.is_empty());
        assert_ne!(motion.tile, data.spawn_tile);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/movement-smoke.png"
            )));
        *stage = 2;
    }
    if time.elapsed_secs() > 12. {
        exit.write(AppExit::Success);
    }
}

fn inventory_smoke(
    mut commands: Commands,
    time: Res<Time>,
    mut bag: ResMut<inventory_ui::Bag>,
    motion: Res<Motion>,
    data: Res<SceneData>,
    mut stage: Local<u8>,
    mut exit: MessageWriter<AppExit>,
) {
    if !std::env::args().any(|a| a == "--inventory-smoke") {
        return;
    }
    if *stage == 0 && time.elapsed_secs() > 3. {
        bag.open = true;
        bag.dirty = true;
        let index = bag
            .inventory
            .items
            .iter()
            .position(|i| i.ammo_capacity > 0)
            .expect("starting pistol");
        bag.inventory.select(index);
        bag.inventory.equip_selected().unwrap();
        *stage = 1;
    }
    if *stage == 1 && time.elapsed_secs() > 7. {
        assert!(bag.open);
        assert!(bag.inventory.equipped.is_some());
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/inventory-smoke.png"
            )));
        *stage = 2;
    }
    if *stage == 2 && time.elapsed_secs() > 10. {
        let ids: Vec<i32> = bag
            .inventory
            .items
            .iter()
            .filter(|i| i.item_type == 3)
            .map(|i| i.id)
            .collect();
        let saved = save::SaveGame {
            schema: 1,
            tile: motion.tile,
            facing: motion.facing,
            equipped: bag.inventory.equipped,
        };
        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/save-smoke.json");
        std::fs::write(file, saved.encode().unwrap()).unwrap();
        let loaded = save::SaveGame::decode(
            &std::fs::read_to_string(file).unwrap(),
            &data.walkable,
            &ids,
        )
        .unwrap();
        assert_eq!(saved, loaded);
        bag.open = false;
        bag.dirty = true;
        *stage = 3;
    }
    if *stage == 3 && time.elapsed_secs() > 12. {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/equipped-smoke.png"
            )));
        *stage = 4;
    }
    if time.elapsed_secs() > 15. {
        exit.write(AppExit::Success);
    }
}

fn save_input(
    keys: Res<ButtonInput<KeyCode>>,
    data: Res<SceneData>,
    mut motion: ResMut<Motion>,
    mut bag: ResMut<inventory_ui::Bag>,
) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/quicksave.json");
    let ids: Vec<i32> = bag
        .inventory
        .items
        .iter()
        .filter(|i| i.item_type == 3)
        .map(|i| i.id)
        .collect();
    if keys.just_pressed(KeyCode::F5) {
        if motion.progress > 0. {
            motion.message = "Wait until walking finishes before saving".into();
            return;
        }
        let s = save::SaveGame {
            schema: 1,
            tile: motion.tile,
            facing: motion.facing,
            equipped: bag.inventory.equipped,
        };
        motion.message = match s
            .validate(&data.walkable, &ids)
            .and_then(|_| s.encode())
            .and_then(|text| std::fs::write(path, text).map_err(|e| e.to_string()))
        {
            Ok(()) => "Position and weapon saved".into(),
            Err(e) => format!("Save failed: {}", e),
        };
    }
    if keys.just_pressed(KeyCode::F9) {
        match std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|text| save::SaveGame::decode(&text, &data.walkable, &ids))
        {
            Ok(s) => {
                motion.tile = s.tile;
                motion.from = s.tile;
                motion.facing = s.facing;
                motion.progress = 0.;
                motion.path.clear();
                bag.inventory.equipped = s.equipped;
                bag.dirty = true;
                motion.message = "Position and weapon loaded".into();
            }
            Err(e) => motion.message = format!("Load failed: {}", e),
        }
    }
}
