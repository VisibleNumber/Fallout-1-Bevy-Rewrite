use crate::{Motion, SceneData, inventory_ui::Bag};
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Examinable {
    pub id: i32,
    pub tile: i32,
    pub name: String,
    pub description: String,
}

/// Examine the nearest authored object under the cursor with E or right-click.
pub fn examine(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    bag: Res<Bag>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), Without<crate::Player>>,
    data: Res<SceneData>,
    fight: Res<crate::combat_ui::Fight>,
    mut motion: ResMut<Motion>,
) {
    if bag.open
        || bag.block_frame
        || (!keys.just_pressed(KeyCode::KeyE) && !buttons.just_pressed(MouseButton::Right))
    {
        return;
    }

    let Some(cursor) = window.cursor_position() else {
        motion.message = "Nothing to examine".into();
        return;
    };
    if cursor.y < 105. || cursor.y > window.height() - 100. {
        return;
    }
    let (camera, camera_transform) = *camera;
    let Some(world_cursor) = camera.viewport_to_world_2d(camera_transform, cursor).ok() else {
        motion.message = "Nothing to examine".into();
        return;
    };

    let nearest = data
        .examinables
        .iter()
        .map(|item| {
            let tile = fight
                .rats
                .iter()
                .find(|r| r.id == item.id)
                .map(|r| r.tile)
                .unwrap_or(item.tile);
            (item, crate::point(tile).distance(world_cursor))
        })
        .filter(|(_, distance)| *distance <= 40.)
        .min_by(|a, b| a.1.total_cmp(&b.1));

    motion.message = nearest
        .map(|(item, _)| format!("{}\n{}", item.name, item.description))
        .unwrap_or_else(|| "Nothing to examine".into());
}
