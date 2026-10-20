//! Bevy presentation of the independent dialogue rules. The opening briefing is
//! a data-driven prototype presentation, not execution of the original OVER.INT.
use crate::{Motion, SceneData, combat_ui::Fight, dialogue::Story, inventory_ui::Bag};
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct DialoguePanel;
#[derive(Component)]
pub(crate) struct DialogueChoice {
    pub node: String,
    pub choice: String,
}
pub fn in_game(fight: Res<Fight>) -> bool {
    fight.story.active.is_none()
}

pub fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    data: Res<SceneData>,
    mut fight: ResMut<Fight>,
    mut bag: ResMut<Bag>,
    mut motion: ResMut<Motion>,
    buttons: Query<(&Interaction, &DialogueChoice), Changed<Interaction>>,
) {
    if fight.story.active.is_none() {
        if keys.just_pressed(KeyCode::KeyT) && !bag.open && motion.progress == 0. {
            if fight.active || fight.player.hp == 0 || fight.attack_time > 0. {
                motion.message = "Finish combat before opening the briefing".into();
                return;
            }
            motion.path.clear();
            match fight.story.begin(&data.dialogue_definition) {
                Ok(()) => bag.block_frame = true,
                Err(e) => motion.message = e,
            }
        }
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        fight.story.active = None;
        bag.block_frame = true;
        motion.message = "Briefing closed".into();
        return;
    }
    let mut selected = buttons
        .iter()
        .find(|(interaction, _)| **interaction == Interaction::Pressed)
        .map(|(_, button)| (button.node.clone(), button.choice.clone()));
    if selected.is_none() && keys.just_pressed(KeyCode::Enter) {
        if let Ok(choices) = fight.story.available_choices(&data.dialogue_definition) {
            if let Some(choice) = choices.first() {
                selected = Some((fight.story.active.clone().unwrap(), choice.id.clone()));
            }
        }
    }
    if let Some((node, choice)) = selected {
        match fight
            .story
            .choose(&data.dialogue_definition, &node, &choice)
        {
            Ok(()) => {
                // Consume the closing click so it cannot become a ground order.
                bag.block_frame = true;
                if fight.story.active.is_none() {
                    motion.message = "Water chip search started".into();
                }
            }
            Err(e) => motion.message = e,
        }
    }
}
pub fn refresh(
    mut commands: Commands,
    data: Res<SceneData>,
    fight: Res<Fight>,
    assets: Res<AssetServer>,
    panels: Query<Entity, With<DialoguePanel>>,
    mut previous: Local<Option<Story>>,
) {
    if previous.as_ref() == Some(&fight.story) {
        return;
    }
    *previous = Some(fight.story.clone());
    for panel in &panels {
        commands.entity(panel).despawn();
    }
    let Some(node_id) = fight.story.active.as_ref() else {
        return;
    };
    let Some(node) = data.dialogue_definition.nodes.get(node_id) else {
        return;
    };
    let choices = fight
        .story
        .available_choices(&data.dialogue_definition)
        .unwrap_or_default();
    commands
        .spawn((
            DialoguePanel,
            GlobalZIndex(150),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.9)),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(640),
                        height: px(480),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.035, 0.045, 0.025)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        ImageNode::new(assets.load(data.dialogue_portrait.file.clone())),
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(183),
                            top: px(12),
                            width: px(274),
                            height: px(data.dialogue_portrait.h),
                            ..default()
                        },
                    ));
                    panel.spawn((
                        Text::new(&data.dialogue_definition.speaker),
                        TextFont {
                            font_size: 18.,
                            ..default()
                        },
                        TextColor(Color::srgb(0.5, 1., 0.4)),
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(40),
                            top: px(213),
                            ..default()
                        },
                    ));
                    panel.spawn((
                        Text::new(&node.text),
                        TextFont {
                            font_size: 15.,
                            ..default()
                        },
                        TextColor(Color::srgb(0.45, 0.95, 0.4)),
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(40),
                            top: px(242),
                            width: px(560),
                            height: px(120),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                    ));
                    for (i, choice) in choices.iter().enumerate() {
                        panel
                            .spawn((
                                Button,
                                DialogueChoice {
                                    node: node_id.clone(),
                                    choice: choice.id.clone(),
                                },
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: px(40),
                                    top: px(375 + i * 40),
                                    width: px(560),
                                    height: px(34),
                                    padding: UiRect::all(px(7)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.08, 0.16, 0.065)),
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Text::new(&choice.label),
                                    TextFont {
                                        font_size: 15.,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.5, 1., 0.4)),
                                ));
                            });
                    }
                    panel.spawn((
                        Text::new("Enter continues | Esc closes"),
                        TextFont {
                            font_size: 12.,
                            ..default()
                        },
                        TextColor(Color::srgb(0.3, 0.6, 0.3)),
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(40),
                            bottom: px(12),
                            ..default()
                        },
                    ));
                });
        });
}
