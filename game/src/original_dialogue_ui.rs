//! Original OVER.INT conversation UI. Y is a temporary access point until Vault 13 is traversable.
use crate::{
    Motion, SceneData,
    combat_ui::Fight,
    inventory_ui::Bag,
    original_dialogue::{self, State, View},
};
use bevy::prelude::*;
use serde_json::{Value, json};
use std::sync::{Mutex, mpsc};
#[derive(Resource, Default)]
pub struct Job(Option<Mutex<mpsc::Receiver<Result<Value, String>>>>);
#[derive(Component)]
pub(crate) struct Panel;
#[derive(Component)]
pub(crate) struct Choice {
    index: usize,
    step: u64,
}
fn launch(job: &mut Job, state: &mut State, request: Value) {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(original_dialogue::run(request));
    });
    job.0 = Some(Mutex::new(rx));
    state.open = true;
    state.loading = true;
    state.error = None;
}
pub fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut fight: ResMut<Fight>,
    mut bag: ResMut<Bag>,
    mut motion: ResMut<Motion>,
    buttons: Query<(&Interaction, &Choice), Changed<Interaction>>,
    mut job: ResMut<Job>,
) {
    if let Some(result) = job
        .0
        .as_ref()
        .and_then(|rx| rx.lock().ok()?.try_recv().ok())
    {
        job.0 = None;
        fight.original.loading = false;
        let applied = result.and_then(|response| {
            let items = original_dialogue::apply_inventory(&bag.inventory.items, &response)?;
            let session = response
                .get("session")
                .cloned()
                .ok_or("Missing script session")?;
            let view: View =
                serde_json::from_value(session["view"].clone()).map_err(|e| e.to_string())?;
            let next = State {
                open: response["status"] == "dialogue",
                loading: false,
                session: Some(session),
                view: Some(view),
                error: None,
            };
            next.validate()?;
            Ok((items, next))
        });
        match applied {
            Ok((items, state)) => {
                for (before, after) in bag.inventory.items.iter().zip(&items) {
                    let added = (after.quantity - before.quantity).max(0);
                    if added > 0 {
                        *fight.script_awards.entry(after.id).or_default() += added;
                    }
                }
                bag.inventory.items = items;
                bag.dirty = true;
                fight.original = state;
                bag.block_frame = true;
                motion.message = "Original Overseer script executed".into();
            }
            Err(error) => {
                fight.original.error = Some(error.clone());
                motion.message = error;
            }
        }
    }
    if fight.original.open {
        if keys.just_pressed(KeyCode::Escape) {
            job.0 = None;
            fight.original.loading = false;
            fight.original.open = false;
            bag.block_frame = true;
            return;
        }
        if fight.original.loading {
            return;
        }
        let step = fight
            .original
            .session
            .as_ref()
            .and_then(|s| s["step"].as_u64())
            .unwrap_or(0);
        let selected = buttons
            .iter()
            .find(|(i, c)| **i == Interaction::Pressed && c.step == step)
            .map(|(_, c)| c.index)
            .or_else(|| {
                (keys.just_pressed(KeyCode::Enter)
                    && fight
                        .original
                        .view
                        .as_ref()
                        .is_some_and(|v| !v.options.is_empty()))
                .then_some(0)
            });
        if let Some(option) = selected {
            let request = json!({"action":"choose","session":fight.original.session,"option":option,"expected_step":step});
            launch(&mut job, &mut fight.original, request);
            bag.block_frame = true;
        }
        return;
    }
    if keys.just_pressed(KeyCode::KeyY)
        && fight.story.active.is_none()
        && !bag.open
        && motion.progress == 0.
    {
        if fight.active || fight.player.hp == 0 || fight.attack_time > 0. {
            motion.message = "Finish combat before talking".into();
            return;
        }
        motion.path.clear();
        bag.block_frame = true;
        if fight
            .original
            .session
            .as_ref()
            .is_some_and(|s| s["phase"] != "closed")
        {
            fight.original.open = true;
            return;
        }
        let request = if let Some(session) = &fight.original.session {
            json!({"action":"reopen","session":session,"inventory":bag.inventory.items,"hp":fight.player.hp})
        } else {
            json!({"action":"start","inventory":bag.inventory.items,"hp":fight.player.hp})
        };
        launch(&mut job, &mut fight.original, request);
    }
}
pub fn refresh(
    mut commands: Commands,
    data: Res<SceneData>,
    fight: Res<Fight>,
    assets: Res<AssetServer>,
    panels: Query<Entity, With<Panel>>,
    mut previous: Local<String>,
) {
    let state = &fight.original;
    let fingerprint =
        serde_json::to_string(&(state.open, state.loading, &state.view, &state.error)).unwrap();
    if *previous == fingerprint {
        return;
    }
    *previous = fingerprint;
    for panel in &panels {
        commands.entity(panel).despawn();
    }
    if !state.open {
        return;
    }
    let reply = if state.loading {
        "Running original dialogue script...".to_string()
    } else if let Some(error) = &state.error {
        format!("Script stopped: {error}")
    } else {
        state
            .view
            .as_ref()
            .and_then(|v| v.reply.as_ref())
            .map(|r| r.text.clone())
            .unwrap_or_default()
    };
    let step = state
        .session
        .as_ref()
        .and_then(|s| s["step"].as_u64())
        .unwrap_or(0);
    commands
        .spawn((
            Panel,
            GlobalZIndex(160),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.92)),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(720),
                        height: px(660),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.035, 0.045, 0.025)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        ImageNode::new(assets.load(data.dialogue_portrait.file.clone())),
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(223),
                            top: px(10),
                            width: px(274),
                            height: px(data.dialogue_portrait.h),
                            ..default()
                        },
                    ));
                    panel.spawn((
                        Text::new("Overseer | original script"),
                        TextFont {
                            font_size: 18.,
                            ..default()
                        },
                        TextColor(Color::srgb(0.5, 1., 0.4)),
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(30),
                            top: px(220),
                            ..default()
                        },
                    ));
                    panel.spawn((
                        Text::new(reply),
                        TextFont {
                            font_size: 16.,
                            ..default()
                        },
                        TextColor(Color::srgb(0.5, 1., 0.4)),
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(30),
                            top: px(250),
                            width: px(660),
                            height: px(160),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                    ));
                    if !state.loading && state.error.is_none() {
                        if let Some(view) = &state.view {
                            for (index, choice) in view.options.iter().enumerate() {
                                panel
                                    .spawn((
                                        Button,
                                        Choice { index, step },
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: px(30),
                                            top: px(420 + index * 65),
                                            width: px(660),
                                            height: px(58),
                                            padding: UiRect::all(px(6)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.08, 0.16, 0.065)),
                                    ))
                                    .with_children(|button| {
                                        button.spawn((
                                            Text::new(choice.text.as_deref().unwrap_or("Continue")),
                                            TextFont {
                                                font_size: 15.,
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.5, 1., 0.4)),
                                        ));
                                    });
                            }
                        }
                    }
                    panel.spawn((
                        Text::new(
                            "Enter: first response | Esc: pause | Y: resume (temporary NPC access)",
                        ),
                        TextFont {
                            font_size: 12.,
                            ..default()
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(30),
                            bottom: px(12),
                            ..default()
                        },
                    ));
                });
        });
}

/// Runs the real worker through the same asynchronous UI controls, then checkpoints it.
pub fn smoke(
    time: Res<Time>,
    data: Res<SceneData>,
    mut fight: ResMut<Fight>,
    mut bag: ResMut<Bag>,
    mut motion: ResMut<Motion>,
    mut job: ResMut<Job>,
    mut buttons: Query<(&Choice, &mut Interaction)>,
    mut stage: Local<u8>,
    mut due: Local<f32>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    if !std::env::args().any(|a| a == "--original-dialogue-smoke") {
        return;
    }
    let now = time.elapsed_secs();
    let file = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/original-dialogue-smoke-save.json"
    );
    if let Some(error) = &fight.original.error {
        panic!("Original dialogue smoke: {error}");
    }
    if *stage == 0 && now > 3. {
        let request =
            json!({"action":"start","inventory":bag.inventory.items,"hp":fight.player.hp});
        launch(&mut job, &mut fight.original, request);
        *stage = 1;
    }
    if *stage == 1 && !fight.original.loading && fight.original.view.is_some() {
        assert!(fight.original.open);
        assert!(
            fight
                .original
                .view
                .as_ref()
                .unwrap()
                .reply
                .as_ref()
                .unwrap()
                .text
                .contains("one piece")
        );
        let expected = fight.original.clone();
        crate::combat_ui::save_checkpoint(file, &data, &fight, &bag, &motion).unwrap();
        fight.original = State::default();
        crate::combat_ui::load_checkpoint(file, &data, &mut fight, &mut bag, &mut motion).unwrap();
        assert!(
            fight.original == expected,
            "Actual VM session must survive saving"
        );
        *stage = 2;
        *due = now + 1.;
    }
    if *stage == 2 && now > *due {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/original-dialogue-smoke.png"
            )));
        for (choice, mut interaction) in &mut buttons {
            if choice.index == 0 {
                *interaction = Interaction::Pressed;
            }
        }
        *stage = 3;
    }
    if *stage == 3
        && !fight.original.loading
        && fight
            .original
            .session
            .as_ref()
            .is_some_and(|s| s["step"].as_u64().unwrap_or(0) >= 2)
    {
        assert_eq!(
            fight.original.view.as_ref().unwrap().options[0]
                .procedure
                .as_deref(),
            Some("overend")
        );
        *stage = 4;
        *due = now + 0.5;
    }
    if *stage == 4 && now > *due {
        for (choice, mut interaction) in &mut buttons {
            if choice.index == 0 {
                *interaction = Interaction::Pressed;
            }
        }
        *stage = 5;
    }
    if *stage == 5 && !fight.original.loading && !fight.original.open {
        assert!(
            motion.path.is_empty(),
            "Dialogue must not issue a ground order"
        );
        let expected = fight.original.clone();
        crate::combat_ui::save_checkpoint(file, &data, &fight, &bag, &motion).unwrap();
        fight.original = State::default();
        crate::combat_ui::load_checkpoint(file, &data, &mut fight, &mut bag, &mut motion).unwrap();
        assert!(fight.original == expected);
        let request = json!({"action":"reopen","session":fight.original.session,"inventory":bag.inventory.items,"hp":fight.player.hp});
        launch(&mut job, &mut fight.original, request);
        *stage = 6;
    }
    if *stage == 6 && !fight.original.loading {
        assert!(
            fight
                .original
                .view
                .as_ref()
                .unwrap()
                .reply
                .as_ref()
                .unwrap()
                .text
                .contains("have the chip")
        );
        *stage = 7;
        *due = now + 1.;
    }
    if *stage == 7 && now > *due {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/original-dialogue-return-smoke.png"
            )));
        *stage = 8;
        *due = now + 2.;
    }
    if *stage == 8 && now > *due {
        println!("Original dialogue UI, callbacks, inventory, save/load and return visit passed");
        exit.write(AppExit::Success);
    }
    assert!(
        now < 50.,
        "Original dialogue UI smoke timed out at stage {}",
        *stage
    );
}
