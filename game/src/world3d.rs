//! First/third-person view of a local original-art Shady Sands block.
//! Ground is a 3D mesh; vertical artwork remains 2.5D sprite cutouts.
use crate::{Motion, SceneData, combat_ui, inventory_ui, navigation};
use bevy::{
    asset::RenderAssetUsages, input::mouse::AccumulatedMouseMotion, mesh::Indices, prelude::*,
    render::render_resource::PrimitiveTopology,
};
use serde::Deserialize;
use std::collections::HashMap;
#[derive(Deserialize, Resource)]
struct Art {
    floors: Vec<Floor>,
    #[serde(default)]
    roofs: Vec<Floor>,
    props: Vec<Prop>,
}
#[derive(Deserialize)]
struct Floor {
    file: String,
    x: f32,
    z: f32,
}
#[derive(Deserialize)]
struct Prop {
    file: String,
    w: f32,
    h: f32,
    x: f32,
    z: f32,
    kind: i32,
    #[serde(default)]
    solid: bool,
    #[serde(default)]
    door: bool,
    #[serde(default = "full_uv", rename = "uv")]
    _uv: [f32; 4],
}
fn full_uv() -> [f32; 4] {
    [0., 0., 1., 1.]
}
#[derive(Resource, Default)]
struct Walls(Vec<Vec2>);
const WALL_WIDTH: f32 = 1.15;
const WALL_HEIGHT: f32 = 2.7;
fn touches_wall(p: Vec2, radius: f32, walls: &Walls) -> bool {
    walls.0.iter().any(|c| {
        let d = (p - *c).abs() - Vec2::splat(WALL_WIDTH / 2.);
        d.max(Vec2::ZERO).length_squared() < radius * radius || (d.x <= 0. && d.y <= 0.)
    })
}
fn solid_wall_mesh(uv: [f32; 4]) -> Mesh {
    // Chamfered adobe footprint: closed volume with softened vertical edges.
    let half = WALL_WIDTH / 2.;
    let bevel = 0.08;
    let outline = [
        [-half + bevel, -half],
        [half - bevel, -half],
        [half, -half + bevel],
        [half, half - bevel],
        [half - bevel, half],
        [-half + bevel, half],
        [-half, half - bevel],
        [-half, -half + bevel],
    ];
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut tex = Vec::new();
    let mut indices = Vec::new();
    for i in 0..8 {
        let a = outline[i];
        let b = outline[(i + 1) % 8];
        let start = positions.len() as u32;
        positions.extend([
            [a[0], 0., a[1]],
            [b[0], 0., b[1]],
            [b[0], WALL_HEIGHT, b[1]],
            [a[0], WALL_HEIGHT, a[1]],
        ]);
        let n = Vec3::new(b[1] - a[1], 0., a[0] - b[0])
            .normalize()
            .to_array();
        normals.extend([n; 4]);
        tex.extend([
            [uv[0], uv[3]],
            [uv[2], uv[3]],
            [uv[2], uv[1]],
            [uv[0], uv[1]],
        ]);
        indices.extend([start, start + 2, start + 1, start, start + 3, start + 2]);
    }
    for (height, normal, top) in [
        (WALL_HEIGHT, [0., 1., 0.], true),
        (0., [0., -1., 0.], false),
    ] {
        let start = positions.len() as u32;
        for p in outline {
            positions.push([p[0], height, p[1]]);
            normals.push(normal);
            tex.push([
                uv[0] + (p[0] / WALL_WIDTH + 0.5) * (uv[2] - uv[0]),
                uv[1] + (p[1] / WALL_WIDTH + 0.5) * (uv[3] - uv[1]),
            ]);
        }
        for i in 1..7 {
            if top {
                indices.extend([start, start + i + 1, start + i]);
            } else {
                indices.extend([start, start + i, start + i + 1]);
            }
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, tex)
    .with_inserted_indices(Indices::U32(indices))
}
#[derive(Resource)]
struct Controller {
    position: Vec2,
    yaw: f32,
    pitch: f32,
    first: bool,
    moving: bool,
    roofs: bool,
    clock: f32,
}
#[derive(Component)]
struct Card;
#[derive(Component)]
struct Avatar;
#[derive(Component)]
struct Npc(usize);
#[derive(Component)]
struct Status;
#[derive(Component)]
struct Roof;
fn ground(tile: i32) -> Vec2 {
    let (x, y) = navigation::screen(tile);
    Vec2::new((12. * x + 16. * y) / 384., (6. * x - 24. * y) / 384.)
}
fn tile_at(p: Vec2) -> Option<i32> {
    let c = (p.x - 1. / 6.).round() as i32;
    let r = (p.y - 0.75 - 0.5 * c.rem_euclid(2) as f32).round() as i32;
    if !(0..200).contains(&c) || !(0..200).contains(&r) {
        return None;
    }
    Some(r * 200 + 199 - c)
}
pub fn run() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/scene-shadyw.json"
    ))
    .expect("Run node game/prepare-world.mjs first");
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let art: Art = serde_json::from_value(value["view3d"].clone())
        .expect("Reprepare Shady Sands for the 3D view");
    let data: SceneData = serde_json::from_value(value).unwrap();
    let tile = data.spawn_tile;
    let bag = inventory_ui::Bag::new(data.inventory.clone());
    let fight = combat_ui::Fight::new(&data);
    App::new()
        .insert_resource(art)
        .insert_resource(data)
        .insert_resource(bag)
        .insert_resource(fight)
        .insert_resource(Motion {
            tile,
            from: tile,
            path: Default::default(),
            progress: 0.,
            facing: 2,
            clock: 0.,
            message: "Original artwork | Shady Sands west".into(),
        })
        .insert_resource(Controller {
            position: ground(tile),
            yaw: -2.5,
            pitch: -0.12,
            first: false,
            moving: false,
            roofs: true,
            clock: 0.,
        })
        .insert_resource(ClearColor(Color::srgb(0.28, 0.23, 0.16)))
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").into(),
                    ..default()
                })
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Fallout Rewrite | Original-art 3D Shady Sands".into(),
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
                controls,
                combat_ui::item_actions,
                inventory_ui::refresh,
                draw,
                smoke,
            )
                .chain(),
        )
        .run();
}
fn floor_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[0., 0., 0.], [2., 0., 0.], [2., 0., 2.], [0., 0., 2.]],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0., 1. / 3.], [0.6, 0.], [1., 2. / 3.], [0.4, 1.]],
    )
    .with_inserted_indices(Indices::U32(vec![0, 3, 2, 0, 2, 1]))
}
fn material(
    file: &str,
    assets: &AssetServer,
    materials: &mut Assets<StandardMaterial>,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color_texture: Some(assets.load(file.to_string())),
        unlit: true,
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: None,
        ..default()
    })
}
fn setup(
    mut commands: Commands,
    data: Res<SceneData>,
    art: Res<Art>,
    control: Res<Controller>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ground_mesh = meshes.add(floor_mesh());
    let card = meshes.add(Rectangle::new(1., 1.));
    let mut cache = HashMap::new();
    let wall_mesh = meshes.add(solid_wall_mesh(full_uv()));
    let roof_mesh = meshes.add(Cuboid::new(2., 0.14, 2.));
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.66, 0.63, 0.56),
        perceptual_roughness: 1.,
        ..default()
    });
    let roof_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.43, 0.42),
        perceptual_roughness: 1.,
        ..default()
    });
    commands.spawn((
        DirectionalLight {
            illuminance: 9000.,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.5, 0.)),
    ));
    let door_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.29, 0.21),
        perceptual_roughness: 1.,
        ..default()
    });
    let post_mesh = meshes.add(Cuboid::new(0.16, 2.25, 0.3));
    let lintel_mesh = meshes.add(Cuboid::new(1.28, 0.18, 0.3));
    let leaf_mesh = meshes.add(Cuboid::new(0.96, 2.1, 0.12));
    let mut walls = Walls::default();
    for f in &art.floors {
        if Vec2::new(f.x + 1., f.z + 1.).distance(control.position) > 25. {
            continue;
        }
        let texture = cache
            .entry(f.file.clone())
            .or_insert_with(|| material(&f.file, &assets, &mut materials))
            .clone();
        commands.spawn((
            Mesh3d(ground_mesh.clone()),
            MeshMaterial3d(texture),
            Transform::from_xyz(f.x, 0., f.z),
        ));
    }
    let roof_cells: std::collections::HashSet<(i32, i32)> = art
        .roofs
        .iter()
        .map(|f| ((f.x * 4.).round() as i32, (f.z * 4.).round() as i32))
        .collect();
    let edge_x = meshes.add(Cuboid::new(2.08, 0.18, 0.16));
    let edge_z = meshes.add(Cuboid::new(0.16, 0.18, 2.08));
    for f in &art.roofs {
        if Vec2::new(f.x + 1., f.z + 1.).distance(control.position) > 25. {
            continue;
        }
        commands.spawn((
            Roof,
            Mesh3d(roof_mesh.clone()),
            MeshMaterial3d(roof_material.clone()),
            Transform::from_xyz(f.x + 1., WALL_HEIGHT + 0.07, f.z + 1.),
        ));
        let key = ((f.x * 4.).round() as i32, (f.z * 4.).round() as i32);
        for (dx, dz) in [(8, 0), (-8, 0), (0, 8), (0, -8)] {
            if !roof_cells.contains(&(key.0 + dx, key.1 + dz)) {
                commands.spawn((
                    Roof,
                    Mesh3d(if dx == 0 {
                        edge_x.clone()
                    } else {
                        edge_z.clone()
                    }),
                    MeshMaterial3d(roof_material.clone()),
                    Transform::from_xyz(
                        f.x + 1. + dx as f32 / 8.,
                        WALL_HEIGHT - 0.02,
                        f.z + 1. + dz as f32 / 8.,
                    ),
                ));
            }
        }
    }
    for f in &art.props {
        if Vec2::new(f.x, f.z).distance(control.position) > 23. {
            continue;
        }
        if f.door {
            let mut neighbors: Vec<&Prop> = art
                .props
                .iter()
                .filter(|p| p.solid && Vec2::new(p.x - f.x, p.z - f.z).length() < 2.5)
                .collect();
            neighbors.sort_by(|a, b| {
                Vec2::new(a.x - f.x, a.z - f.z)
                    .length_squared()
                    .total_cmp(&Vec2::new(b.x - f.x, b.z - f.z).length_squared())
            });
            let axis = if neighbors.len() >= 2 {
                Vec2::new(
                    neighbors[1].x - neighbors[0].x,
                    neighbors[1].z - neighbors[0].z,
                )
                .normalize_or_zero()
            } else {
                Vec2::X
            };
            let rotation = Quat::from_rotation_y((-axis.y).atan2(axis.x));
            for side in [-1., 1.] {
                commands.spawn((
                    Mesh3d(post_mesh.clone()),
                    MeshMaterial3d(wall_material.clone()),
                    Transform::from_xyz(
                        f.x + axis.x * 0.56 * side,
                        1.125,
                        f.z + axis.y * 0.56 * side,
                    )
                    .with_rotation(rotation),
                ));
            }
            commands.spawn((
                Mesh3d(lintel_mesh.clone()),
                MeshMaterial3d(wall_material.clone()),
                Transform::from_xyz(f.x, 2.25, f.z).with_rotation(rotation),
            ));
            commands.spawn((
                Mesh3d(leaf_mesh.clone()),
                MeshMaterial3d(door_material.clone()),
                Transform::from_xyz(f.x, 1.05, f.z).with_rotation(rotation),
            ));
            walls.0.push(Vec2::new(f.x, f.z));
            continue;
        }
        if f.solid {
            commands.spawn((
                Mesh3d(wall_mesh.clone()),
                MeshMaterial3d(wall_material.clone()),
                Transform::from_xyz(f.x, WALL_HEIGHT / 2., f.z),
            ));
            walls.0.push(Vec2::new(f.x, f.z));
            continue;
        }
        let texture = cache
            .entry(f.file.clone())
            .or_insert_with(|| material(&f.file, &assets, &mut materials))
            .clone();
        let scale = if f.kind == 3 { 28. } else { 32. };
        commands.spawn((
            Card,
            Mesh3d(card.clone()),
            MeshMaterial3d(texture),
            Transform::from_xyz(f.x, f.h / scale / 2., f.z).with_scale(Vec3::new(
                f.w / scale,
                f.h / scale,
                1.,
            )),
        ));
    }
    commands.insert_resource(walls);
    for (i, npc) in data.actors.iter().enumerate().filter(|(_, a)| a.visible) {
        let pos = ground(npc.tile);
        if pos.distance(control.position) > 23. {
            continue;
        }
        let f = &npc.standing[npc.rotation];
        // NPC materials are independent because each selects its own facing.
        commands.spawn((
            Card,
            Npc(i),
            Mesh3d(card.clone()),
            MeshMaterial3d(material(&f.file, &assets, &mut materials)),
            Transform::from_xyz(pos.x, f.h / 64., pos.y).with_scale(Vec3::new(0.9, f.h / 32., 1.)),
        ));
    }
    // Locally converted New Vegas examples; never distributed with source.
    for (name, offset, lift) in [
        ("table01", Vec2::new(1.5, -2.), 0.350426),
        ("nv_shackvictor", Vec2::new(5., -9.), 0.),
        ("nv_prospectorsaloon_door", Vec2::new(-2., -3.), 0.),
    ] {
        let file = format!("nv/converted/{name}/model.gltf");
        if std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(&file)
            .exists()
        {
            commands.spawn((
                SceneRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(file))),
                Transform::from_xyz(
                    control.position.x + offset.x,
                    lift,
                    control.position.y + offset.y,
                ),
            ));
        }
    }
    let f = &data.player[2];
    commands.spawn((
        Card,
        Avatar,
        Mesh3d(card),
        MeshMaterial3d(material(&f.file, &assets, &mut materials)),
        Transform::from_xyz(control.position.x, f.h / 64., control.position.y)
            .with_scale(Vec3::new(0.9, f.h / 32., 1.)),
    ));
    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Transform::default(),
    ));
    commands.spawn((
        Status,
        Text::new(""),
        TextFont {
            font_size: 16.,
            ..default()
        },
        TextColor(Color::srgb(0.5, 1., 0.4)),
        BackgroundColor(Color::srgba(0., 0., 0., 0.85)),
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(12),
            padding: UiRect::all(px(10)),
            ..default()
        },
    ));
}
fn allowed(p: Vec2, data: &SceneData, walls: &Walls) -> bool {
    if touches_wall(p, 0.18, walls) {
        return false;
    }
    [
        Vec2::ZERO,
        Vec2::X * 0.18,
        -Vec2::X * 0.18,
        Vec2::Y * 0.18,
        -Vec2::Y * 0.18,
    ]
    .iter()
    .all(|d| tile_at(p + *d).is_some_and(|t| data.walkable[t as usize]))
        && data.actors.iter().all(|a| ground(a.tile).distance(p) > 0.4)
}
fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mouse: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
    data: Res<SceneData>,
    bag: Res<inventory_ui::Bag>,
    walls: Res<Walls>,
    mut control: ResMut<Controller>,
    mut motion: ResMut<Motion>,
    mut fight: ResMut<combat_ui::Fight>,
) {
    control.clock += time.delta_secs();
    if std::env::args().any(|a| a == "--3d-smoke") {
        return;
    }
    control.moving = false;
    if bag.open || bag.block_frame {
        return;
    }
    if keys.just_pressed(KeyCode::KeyH) {
        control.roofs = !control.roofs;
    }
    if keys.just_pressed(KeyCode::Tab) {
        control.first = !control.first;
    }
    if buttons.pressed(MouseButton::Right) {
        control.yaw -= mouse.delta.x * 0.004;
        control.pitch = (control.pitch - mouse.delta.y * 0.003).clamp(-1.1, 0.8);
    }
    let forward = Vec2::new(-control.yaw.sin(), -control.yaw.cos());
    let right = Vec2::new(control.yaw.cos(), -control.yaw.sin());
    let mut movement = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        movement += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        movement -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        movement += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        movement -= right;
    }
    let delta = movement.normalize_or_zero()
        * time.delta_secs().min(0.05)
        * if keys.pressed(KeyCode::ShiftLeft) {
            3.5
        } else {
            2.
        };
    for axis in [Vec2::new(delta.x, 0.), Vec2::new(0., delta.y)] {
        let next = control.position + axis;
        if allowed(next, &data, &walls) {
            control.position = next;
            control.moving |= axis.length_squared() > 0.;
        }
    }
    motion.tile = tile_at(control.position).unwrap();
    fight.player.tile = motion.tile;
    if keys.just_pressed(KeyCode::KeyE) {
        if let Some(npc) = data
            .actors
            .iter()
            .filter(|a| a.visible)
            .filter(|a| ground(a.tile).distance(control.position) < 3.)
            .min_by(|a, b| {
                ground(a.tile)
                    .distance_squared(control.position)
                    .total_cmp(&ground(b.tile).distance_squared(control.position))
            })
        {
            motion.message = format!("{} | Conversation not connected in this view yet", npc.name);
        } else {
            motion.message = "No NPC within reach".into();
        }
    }
}
fn draw(
    data: Res<SceneData>,
    control: Res<Controller>,
    walls: Res<Walls>,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cards: Query<
        (
            &mut Transform,
            &MeshMaterial3d<StandardMaterial>,
            Option<&Npc>,
            Option<&Avatar>,
            &mut Visibility,
        ),
        (With<Card>, Without<Camera3d>),
    >,
    mut camera: Single<&mut Transform, (With<Camera3d>, Without<Card>)>,
    mut status: Single<&mut Text, With<Status>>,
    mut roofs: Query<&mut Visibility, (With<Roof>, Without<Card>)>,
    motion: Res<Motion>,
    fight: Res<combat_ui::Fight>,
) {
    for mut roof in &mut roofs {
        *roof = if control.roofs {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let direction = Vec3::new(
        -control.yaw.sin() * control.pitch.cos(),
        control.pitch.sin(),
        -control.yaw.cos() * control.pitch.cos(),
    );
    let eye = Vec3::new(control.position.x, 1.55, control.position.y);
    **camera = if control.first {
        Transform::from_translation(eye).looking_to(direction, Vec3::Y)
    } else {
        let desired = eye - direction * 5. + Vec3::Y * 1.4;
        let mut position = eye;
        for step in 1..=60 {
            let candidate = eye.lerp(desired, step as f32 / 60.);
            if candidate.y < WALL_HEIGHT + 0.15
                && touches_wall(Vec2::new(candidate.x, candidate.z), 0.15, &walls)
            {
                break;
            }
            position = candidate;
        }
        Transform::from_translation(position).looking_at(eye, Vec3::Y)
    };
    let facing = ((control.yaw / std::f32::consts::TAU * 6.).round() as i32).rem_euclid(6) as usize;
    for (mut transform, handle, npc, avatar, mut visible) in &mut cards {
        transform.rotation = Quat::from_rotation_y(control.yaw);
        let frame = if avatar.is_some() {
            *visible = if control.first {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
            let f = if control.moving {
                let frames = &data.walking[facing];
                &frames[(control.clock * 10.) as usize % frames.len()]
            } else {
                &data.player[facing]
            };
            transform.translation = Vec3::new(control.position.x, f.h / 64., control.position.y);
            Some(f)
        } else if let Some(npc) = npc {
            Some(&data.actors[npc.0].standing[(data.actors[npc.0].rotation + facing) % 6])
        } else {
            None
        };
        if let Some(f) = frame {
            if let Some(mat) = materials.get_mut(&handle.0) {
                mat.base_color_texture = Some(assets.load(f.file.clone()));
            }
            transform.scale.y = f.h / 32.;
        }
    }
    **status = Text::new(format!(
        "FALLOUT | Original-art 3D Shady Sands | {} person\nWASD move | Hold right mouse: look | Tab: camera | H: roofs | I: inventory\nHP {}/{} | Hex {} | {}\nGEOMETRY BLOCKOUT | Plain walls and roofs | H toggles roofs for layout review",
        if control.first { "First" } else { "Third" },
        fight.player.hp,
        fight.player.max_hp,
        motion.tile,
        motion.message
    ));
}
fn smoke(
    time: Res<Time>,
    mut control: ResMut<Controller>,
    mut stage: Local<u8>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    if !std::env::args().any(|a| a == "--3d-smoke") {
        return;
    }
    if time.elapsed_secs() < 1. {
        control.yaw = -0.45;
        control.pitch = -0.06;
    }
    if *stage == 0 && time.elapsed_secs() > 5. {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/third-person-smoke.png"
            )));
        *stage = 1;
    }
    if *stage == 1 && time.elapsed_secs() > 6. {
        control.first = true;
        *stage = 2;
    }
    if *stage == 2 && time.elapsed_secs() > 8. {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/first-person-smoke.png"
            )));
        *stage = 3;
    }
    if time.elapsed_secs() > 10. {
        exit.write(AppExit::Success);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn solid_wall_blocks_player_radius_including_corners() {
        let walls = Walls(vec![Vec2::ZERO]);
        assert!(touches_wall(Vec2::ZERO, 0.18, &walls));
        assert!(touches_wall(Vec2::new(0.7, 0.), 0.18, &walls));
        assert!(touches_wall(Vec2::new(0.7, 0.7), 0.18, &walls));
        assert!(!touches_wall(Vec2::new(0.8, 0.), 0.18, &walls));
        assert!(!touches_wall(Vec2::ZERO, 0.18, &Walls::default()));
    }
    #[test]
    fn solid_wall_uses_closed_mesh_and_only_selected_original_texture_region() {
        let mesh = solid_wall_mesh([0.2, 0.3, 0.7, 0.8]);
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uv)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("missing texture coordinates")
        };
        assert!(
            uv.iter()
                .all(|p| (0.2..=0.7).contains(&p[0]) && (0.3..=0.8).contains(&p[1]))
        );
        assert_eq!(mesh.indices().unwrap().len(), 84);
    }
    #[test]
    fn original_hex_centers_round_trip_through_3d_ground() {
        for tile in 0..40000 {
            assert_eq!(tile_at(ground(tile)), Some(tile));
        }
        assert_eq!(tile_at(Vec2::new(-5., -5.)), None);
    }
}
