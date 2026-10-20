use crate::{Motion, combat_ui::Fight};
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct ApLight(usize);

#[derive(Component)]
pub(crate) struct MessageLog;

/// Spawn the ten action-point pips and the message-log text inside a UI parent.
pub fn spawn(parent: &mut ChildSpawnerCommands, assets: &AssetServer, ap_file: &str) {
    for index in 0..10 {
        parent.spawn((
            ApLight(index),
            ImageNode {
                image: assets.load(ap_file.to_owned()),
                color: Color::WHITE,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: px(316 + 9 * index as i32),
                top: px(14),
                width: px(5),
                height: px(5),
                ..default()
            },
        ));
    }

    parent.spawn((
        MessageLog,
        Text::new(""),
        TextFont {
            font_size: 10.,
            ..default()
        },
        TextColor(Color::srgb(0.3, 1., 0.3)),
        Node {
            position_type: PositionType::Absolute,
            left: px(23),
            top: px(25),
            width: px(166),
            height: px(60),
            overflow: Overflow::clip(),
            ..default()
        },
    ));
}

/// Refresh AP pip tints and the message log from the current game resources.
pub fn update(
    fight: Res<Fight>,
    motion: Res<Motion>,
    mut lights: Query<(&ApLight, &mut ImageNode)>,
    mut log: Query<&mut Text, With<MessageLog>>,
) {
    let ap = if fight.active {
        fight.player.ap
    } else {
        fight.player.max_ap
    };
    for (light, mut image) in &mut lights {
        image.color = if (light.0 as i32) < ap {
            Color::WHITE
        } else {
            Color::srgba(0.12, 0.12, 0.12, 1.)
        };
    }
    for mut text in &mut log {
        *text = Text::new(motion.message.clone());
    }
}
