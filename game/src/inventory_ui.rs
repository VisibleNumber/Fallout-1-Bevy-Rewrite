use crate::{SceneData, inventory::Inventory};
use bevy::prelude::*;

#[derive(Resource)]
pub struct Bag {
    pub inventory: Inventory,
    /// First positive-quantity catalog row shown in the six-slot viewport.
    pub row_offset: usize,
    pub open: bool,
    pub block_frame: bool,
    pub dirty: bool,
    pub notice: String,
}
impl Bag {
    pub fn new(items: Vec<crate::inventory::Item>) -> Self {
        Self {
            inventory: Inventory::new(items).expect("valid imported inventory"),
            row_offset: 0,
            open: false,
            block_frame: false,
            dirty: true,
            notice: String::new(),
        }
    }
}

const VISIBLE_ROWS: usize = 6;

fn positive_indices(bag: &Bag) -> Vec<usize> {
    bag.inventory
        .items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| (item.quantity > 0).then_some(index))
        .collect()
}

fn keep_selection_visible(bag: &mut Bag) {
    let indices = positive_indices(bag);
    if indices.is_empty() {
        bag.row_offset = 0;
        return;
    }
    if !indices.contains(&bag.inventory.selected) {
        bag.inventory.selected = indices[0];
    }
    let selected_row = indices
        .iter()
        .position(|&index| index == bag.inventory.selected)
        .expect("selected positive inventory row");
    if selected_row < bag.row_offset {
        bag.row_offset = selected_row;
    } else if selected_row >= bag.row_offset + VISIBLE_ROWS {
        bag.row_offset = selected_row + 1 - VISIBLE_ROWS;
    }
    bag.row_offset = bag
        .row_offset
        .min(indices.len().saturating_sub(VISIBLE_ROWS));
}
#[derive(Component)]
pub struct InventoryPanel;
#[derive(Component)]
pub struct EquippedIcon;

pub fn actions(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    mut bag: ResMut<Bag>,
    mut fight: ResMut<crate::combat_ui::Fight>,
    mut motion: ResMut<crate::Motion>,
) {
    let was_open = bag.open;
    bag.block_frame = false;
    if keys.just_pressed(KeyCode::KeyI) {
        bag.open = !bag.open;
        bag.dirty = true;
        bag.block_frame = true;
    }
    if keys.just_pressed(KeyCode::Escape) && bag.open {
        bag.open = false;
        bag.dirty = true;
        bag.block_frame = true;
    }
    let clicked = buttons.just_pressed(MouseButton::Left);
    if clicked {
        if let Some(p) = window.cursor_position() {
            if bag.open {
                bag.block_frame = true;
                let x = p.x - (window.width() - 499.) / 2.;
                let y = p.y - (window.height() - 377.) / 2.;
                if (429. ..465.).contains(&x) && (320. ..356.).contains(&y) {
                    bag.open = false;
                    bag.dirty = true;
                } else if (35. ..122.).contains(&x) && (32. ..320.).contains(&y) {
                    let row = ((y - 32.) / 48.) as usize;
                    let indices = positive_indices(&bag);
                    let visible_row = bag.row_offset + row;
                    if row < VISIBLE_ROWS
                        && let Some(&index) = indices.get(visible_row)
                        && bag.inventory.select(index)
                    {
                        bag.dirty = true;
                    }
                } else if (245. ..335.).contains(&x) && (286. ..347.).contains(&y) {
                    equip(&mut bag);
                }
            } else {
                let x = p.x - (window.width() - 640.) / 2.;
                let y = p.y - (window.height() - 100.);
                if (211. ..243.).contains(&x) && (41. ..62.).contains(&y) {
                    bag.open = true;
                    bag.dirty = true;
                    bag.block_frame = true;
                }
            }
        }
    }
    if !was_open && bag.open {
        if fight.player.hp == 0
            || motion.progress > 0.
            || fight.attack_time > 0.
            || (fight.active && fight.player.ap < 4)
        {
            bag.open = false;
            bag.dirty = true;
            motion.message = "Cannot open inventory now (requires 4 AP in combat)".into();
        } else if fight.active {
            fight.player.ap -= 4;
        }
    }
    if bag.open {
        keep_selection_visible(&mut bag);
        let indices = positive_indices(&bag);
        if keys.just_pressed(KeyCode::ArrowDown)
            && let Some(row) = indices.iter().position(|&i| i == bag.inventory.selected)
            && let Some(&next) = indices.get(row + 1)
        {
            bag.inventory.select(next);
            bag.dirty = true;
        }
        if keys.just_pressed(KeyCode::ArrowUp)
            && let Some(row) = indices.iter().position(|&i| i == bag.inventory.selected)
            && row > 0
        {
            bag.inventory.select(indices[row - 1]);
            bag.dirty = true;
        }
        keep_selection_visible(&mut bag);
        if keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::Enter) {
            equip(&mut bag);
        }
    }
}
fn equip(bag: &mut Bag) {
    bag.notice = match bag.inventory.equip_selected() {
        Ok(()) => format!(
            "{} selected for right hand",
            bag.inventory.items[bag.inventory.selected].name
        ),
        Err(e) => e,
    };
    bag.dirty = true;
}

pub fn refresh(
    mut commands: Commands,
    data: Res<SceneData>,
    assets: Res<AssetServer>,
    mut bag: ResMut<Bag>,
    panels: Query<Entity, With<InventoryPanel>>,
    mut equipped: Query<(&mut ImageNode, &mut Visibility), With<EquippedIcon>>,
) {
    if !bag.dirty {
        return;
    }
    bag.dirty = false;
    for (mut image, mut visible) in &mut equipped {
        if let Some(item) = bag
            .inventory
            .items
            .iter()
            .find(|i| Some(i.id) == bag.inventory.equipped)
        {
            image.image = assets.load(item.file.clone());
            *visible = Visibility::Visible;
        } else {
            *visible = Visibility::Hidden;
        }
    }
    for entity in &panels {
        commands.entity(entity).despawn();
    }
    if !bag.open {
        return;
    }
    keep_selection_visible(&mut bag);
    let positive = positive_indices(&bag);
    let visible = positive
        .iter()
        .skip(bag.row_offset)
        .take(VISIBLE_ROWS)
        .copied()
        .collect::<Vec<_>>();
    commands
        .spawn((
            InventoryPanel,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.6)),
            GlobalZIndex(100),
        ))
        .with_children(|overlay| {
            overlay
                .spawn(Node {
                    width: px(499),
                    height: px(377),
                    ..default()
                })
                .with_children(|panel| {
                    panel.spawn((
                        ImageNode::new(assets.load(data.inventory_background.file.clone())),
                        Node {
                            position_type: PositionType::Absolute,
                            width: px(499),
                            height: px(377),
                            ..default()
                        },
                    ));
                    for (row, &index) in visible.iter().enumerate() {
                        let item = &bag.inventory.items[index];
                        let y = 34. + row as f32 * 48.;
                        panel
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: px(37),
                                    top: px(y),
                                    width: px(78),
                                    height: px(47),
                                    ..default()
                                },
                                BackgroundColor(if index == bag.inventory.selected {
                                    Color::srgba(0.1, 0.6, 0.1, 0.24)
                                } else {
                                    Color::NONE
                                }),
                            ))
                            .with_children(|slot| {
                                slot.spawn((
                                    ImageNode::new(assets.load(item.file.clone())),
                                    Node {
                                        width: px(64),
                                        height: px(40),
                                        ..default()
                                    },
                                ));
                                slot.spawn((
                                    Text::new(format!("{}", item.quantity)),
                                    TextFont {
                                        font_size: 11.,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.3, 1., 0.3)),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        right: px(1),
                                        bottom: px(0),
                                        ..default()
                                    },
                                ));
                            });
                    }
                    if let Some(item) = bag.inventory.items.get(bag.inventory.selected) {
                        let text = format!(
                            "{}\nQty: {} | {} lb each{}\n\n{}",
                            item.name,
                            item.quantity,
                            item.weight,
                            if item.ammo_capacity > 0 {
                                format!("\nAmmo: {}/{}", item.ammo_quantity, item.ammo_capacity)
                            } else {
                                String::new()
                            },
                            item.description
                        );
                        panel.spawn((
                            Text::new(text),
                            TextFont {
                                font_size: 11.,
                                ..default()
                            },
                            TextColor(Color::srgb(0.25, 0.9, 0.25)),
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(298),
                                top: px(40),
                                width: px(150),
                                max_height: px(185),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                        ));
                    }
                    let f = &data.player[2];
                    panel.spawn((
                        ImageNode::new(assets.load(f.file.clone())),
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(180),
                            top: px(55),
                            width: px(40),
                            height: px(70),
                            ..default()
                        },
                    ));
                    if let Some(item) = bag
                        .inventory
                        .items
                        .iter()
                        .find(|i| Some(i.id) == bag.inventory.equipped)
                    {
                        panel.spawn((
                            ImageNode::new(assets.load(item.file.clone())),
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(249),
                                top: px(294),
                                width: px(80),
                                height: px(45),
                                ..default()
                            },
                        ));
                    }
                    panel.spawn((
                        Text::new(format!(
                            "{} | {} lb\nI/Esc close | E equip\nU use | R load | X unload | arrows scroll\n{}",
                            data.player_name,
                            bag.inventory.weight(),
                            bag.notice
                        )),
                        TextFont {
                            font_size: 11.,
                            ..default()
                        },
                        TextColor(Color::srgb(0.3, 1., 0.3)),
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(136),
                            top: px(232),
                            width: px(305),
                            max_height: px(60),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0., 0., 0., 0.72)),
                    ));
                });
        });
}
