use super::*;

fn frame() -> Frame {
    Frame {
        file: "synthetic.frm".into(),
        h: 32.,
        dx: 0.,
        dy: 0.,
    }
}

fn item(id: i32, pid: i32, name: &str, item_type: i32, quantity: i32) -> crate::inventory::Item {
    crate::inventory::Item {
        id,
        pid,
        name: name.into(),
        description: format!("Synthetic {name}"),
        file: format!("{name}.frm"),
        quantity,
        weight: 1,
        item_type,
        ammo_quantity: 0,
        ammo_capacity: 0,
        ammo_pid: 0,
        caliber: 0,
        ammo_pack_size: 0,
        heal_min: 0,
        heal_max: 0,
    }
}

fn scene() -> SceneData {
    let mut weapon = item(10, 110, "pistol", 3, 1);
    weapon.ammo_quantity = 2;
    weapon.ammo_capacity = 6;
    weapon.caliber = 45;
    weapon.ammo_pid = 111;

    let mut ammo = item(11, 111, "rounds", 4, 2);
    ammo.ammo_quantity = 4;
    ammo.ammo_pack_size = 5;
    ammo.caliber = 45;

    let mut healing = item(12, 112, "stim", 0, 2);
    healing.heal_min = 3;
    healing.heal_max = 8;

    SceneData {
        dialogue_definition:serde_json::from_str(r#"{"id":"briefing","speaker":"NPC","start":"hello","flags":["water_chip_search"],"nodes":{"hello":{"text":"Synthetic dialogue","choices":[{"id":"end","label":"Done","next":null,"effects":[{"flag":"water_chip_search","value":1}]}]}}}"#).unwrap(),
        dialogue_portrait:frame(),
        examinables: Vec::new(),
        ap_light: frame(),
        carry_weight: 100,
        loot_containers: vec![LootContainer {
            id: 30,
            tile: 101,
            name: "synthetic crate".into(),
            contents: vec![LootSlot {
                item_id: 13,
                quantity: 2,
            }],
        }],
        unarmed_attacking: vec![vec![frame()]; 6],
        shot_blocked: vec![false; 40_000],
        actors: vec![ActorData {
            id: 20,
            tile: 101,
            hp: 10,
            max_hp: 10,
            ap: 4,
            armor_class: 0,
            thresholds: vec![0; 7],
            resistances: vec![0; 7],
            melee_damage: 2,
            skill: 65,
            rotation: 0,
            standing: vec![frame()],
            dead: vec![frame()],
        }],
        weapons: vec![WeaponData {
            pid: 110,
            damage_type: 0,
            min_damage: 1,
            max_damage: 4,
            range: 12,
            ap_cost: 5,
            skill: 50,
            ranged: true,
            perception: 5,
        }],
        unarmed: WeaponData {
            pid: 0,
            damage_type: 0,
            min_damage: 1,
            max_damage: 2,
            range: 1,
            ap_cost: 3,
            skill: 50,
            ranged: false,
            perception: 5,
        },
        player_combat: crate::combat::Combatant {
            id: 1,
            tile: 100,
            hp: 8,
            max_hp: 10,
            ap: 7,
            max_ap: 7,
            armor_class: 0,
        },
        sprites: Vec::new(),
        ui: Vec::new(),
        inventory: vec![weapon, ammo, healing, item(13, 113, "relic", 0, 0)],
        weapon_art: Vec::new(),
        inventory_background: frame(),
        player_name: "Synthetic player".into(),
        hit_points: 8,
        armor_class: 0,
        player: vec![frame(); 6],
        walking: vec![vec![frame()]; 6],
        walkable: vec![true; 40_000],
        spawn_tile: 100,
    }
}

fn setup(data: &SceneData) -> (Fight, Bag, Motion) {
    let mut fight = Fight::new(data);
    fight.player.hp = 8;
    let mut bag = Bag::new(data.inventory.iter().map(clone_item).collect());
    bag.inventory.equipped = Some(10);
    bag.inventory.select(2);
    let motion = Motion {
        tile: 100,
        from: 100,
        path: Default::default(),
        progress: 0.,
        facing: 2,
        clock: 0.,
        message: String::new(),
    };
    (fight, bag, motion)
}

fn clone_item(item: &crate::inventory::Item) -> crate::inventory::Item {
    item.clone()
}

fn temp_file() -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-checkpoints");
    std::fs::create_dir_all(&root).unwrap();
    root.join(format!(
        "fallout-checkpoint-test-{}-{}.json",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
    .to_string_lossy()
    .into_owned()
}

#[test]
fn checkpoint_roundtrip_restores_reload_healing_loot_and_position() {
    let data = scene();
    let (mut fight, mut bag, mut motion) = setup(&data);
    assert_eq!(reload(&mut fight, &mut bag).unwrap(), 4);
    assert_eq!(bag.inventory.items[0].ammo_quantity, 6);
    assert_eq!(bag.inventory.items[1].quantity, 1);
    assert_eq!(bag.inventory.items[1].ammo_quantity, 5);
    assert!(heal(&mut fight, &mut bag).unwrap() > 0);
    assert_eq!(bag.inventory.items[2].quantity, 1);
    take_loot(&data, &mut fight, &mut bag, motion.tile).unwrap();
    assert_eq!(bag.inventory.items[3].quantity, 2);
    assert_eq!(fight.looted, vec![30]);
    motion.tile = 102;
    fight.player.tile = 102;

    let expected_seed = fight.seed;
    let expected_player = fight.player.clone();
    let expected_items = bag.inventory.items.clone();
    let path = temp_file();
    save_checkpoint(&path, &data, &fight, &bag, &motion).unwrap();

    fight.seed = 1;
    fight.player.hp = 1;
    fight.looted.clear();
    bag.inventory.items[0].ammo_quantity = 0;
    bag.inventory.items[1].quantity = 0;
    bag.inventory.items[2].quantity = 0;
    bag.inventory.items[3].quantity = 0;
    motion.tile = 100;
    motion.facing = 0;
    load_checkpoint(&path, &data, &mut fight, &mut bag, &mut motion).unwrap();
    let _ = std::fs::remove_file(&path);

    assert_eq!(fight.seed, expected_seed);
    assert_eq!(fight.player, expected_player);
    assert_eq!(bag.inventory.items, expected_items);
    assert_eq!(fight.looted, vec![30]);
    assert_eq!(motion.tile, 102);
    assert_eq!(motion.facing, 2);
}

#[test]
fn validator_rejects_inventory_identity_quantity_and_ammo_errors() {
    let data = scene();
    let (_, bag, motion) = setup(&data);
    let mut base = checkpoint(&data, &Fight::new(&data), &bag, &motion).unwrap();

    let mut unknown = clone_checkpoint(&base);
    unknown.inventory[0].id = 999;
    assert!(validate_checkpoint(&unknown, &data, &bag).is_err());

    let mut missing = clone_checkpoint(&base);
    missing.inventory.pop();
    assert!(validate_checkpoint(&missing, &data, &bag).is_err());

    let mut reordered = clone_checkpoint(&base);
    reordered.inventory.swap(0, 1);
    assert!(validate_checkpoint(&reordered, &data, &bag).is_err());

    let mut overcount = clone_checkpoint(&base);
    overcount.inventory[0].quantity = 2;
    assert!(validate_checkpoint(&overcount, &data, &bag).is_err());

    let mut invalid_ammo = clone_checkpoint(&base);
    invalid_ammo.inventory[1].ammo = 6;
    assert!(validate_checkpoint(&invalid_ammo, &data, &bag).is_err());

    let mut inconsistent_player = clone_checkpoint(&base);
    inconsistent_player.position.tile += 1;
    assert!(validate_checkpoint(&inconsistent_player, &data, &bag).is_err());

    let mut duplicate_loot = clone_checkpoint(&base);
    duplicate_loot.looted = vec![30, 30];
    assert!(validate_checkpoint(&duplicate_loot, &data, &bag).is_err());

    let mut unknown_loot = clone_checkpoint(&base);
    unknown_loot.looted = vec![999];
    assert!(validate_checkpoint(&unknown_loot, &data, &bag).is_err());

    let mut depleted_equipped = clone_checkpoint(&base);
    depleted_equipped.inventory[0].quantity = 0;
    assert!(validate_checkpoint(&depleted_equipped, &data, &bag).is_err());

    // Make sure the untouched base remains a valid synthetic checkpoint.
    base.position.tile = motion.tile;
    assert!(validate_checkpoint(&base, &data, &bag).is_ok());
}

fn clone_checkpoint(checkpoint: &Checkpoint) -> Checkpoint {
    serde_json::from_str(&serde_json::to_string(checkpoint).unwrap()).unwrap()
}

#[test]
fn invalid_file_load_leaves_all_runtime_state_unchanged() {
    let data = scene();
    let (mut fight, mut bag, mut motion) = setup(&data);
    let mut invalid = checkpoint(&data, &fight, &bag, &motion).unwrap();
    invalid.inventory[0].id = 999;
    let path = temp_file();
    std::fs::write(&path, serde_json::to_string(&invalid).unwrap()).unwrap();

    let player_before = fight.player.clone();
    let rats_before = fight.rats.clone();
    let active_before = fight.active;
    let touched_before = fight.touched;
    let looted_before = fight.looted.clone();
    let items_before = bag.inventory.items.clone();
    let equipped_before = bag.inventory.equipped;
    let motion_before = (motion.tile, motion.from, motion.facing, motion.progress);
    assert!(load_checkpoint(&path, &data, &mut fight, &mut bag, &mut motion).is_err());
    let _ = std::fs::remove_file(&path);

    assert_eq!(fight.player, player_before);
    assert_eq!(fight.rats, rats_before);
    assert_eq!(fight.active, active_before);
    assert_eq!(fight.touched, touched_before);
    assert_eq!(fight.looted, looted_before);
    assert_eq!(bag.inventory.items, items_before);
    assert_eq!(bag.inventory.equipped, equipped_before);
    assert_eq!(
        (motion.tile, motion.from, motion.facing, motion.progress),
        motion_before
    );
}

#[test]
fn encounter_ends_after_participants_die_with_untouched_rats_still_alive() {
    let data = scene();
    let mut fight = Fight::new(&data);
    let mut untouched = fight.rats[0].clone();
    untouched.id = 21;
    fight.rats.push(untouched);
    fight.active = true;
    fight.participants = vec![20];
    fight.player.ap = 0;
    finish_encounter(&mut fight);
    assert!(fight.active);
    fight.rats[0].hp = 0;
    finish_encounter(&mut fight);
    assert!(!fight.active);
    assert!(fight.participants.is_empty());
    assert_eq!(fight.player.ap, fight.player.max_ap);
    assert!(fight.rats[1].hp > 0);
}
#[test]
fn checkpoint_preserves_and_validates_encounter_participants() {
    let data = scene();
    let (mut fight, mut bag, mut motion) = setup(&data);
    fight.active = true;
    fight.participants = vec![20];
    let file = temp_file();
    save_checkpoint(&file, &data, &fight, &bag, &motion).unwrap();
    fight.participants.clear();
    fight.active = false;
    load_checkpoint(&file, &data, &mut fight, &mut bag, &mut motion).unwrap();
    assert_eq!(fight.participants, vec![20]);
    assert!(fight.active);
    let mut c = checkpoint(&data, &fight, &bag, &motion).unwrap();
    c.participants.push(20);
    assert!(validate_checkpoint(&c, &data, &bag).is_err());
    c.participants = vec![999];
    assert!(validate_checkpoint(&c, &data, &bag).is_err());
    std::fs::remove_file(file).unwrap();
}

#[test]
fn attacked_distant_rat_still_takes_its_turn() {
    let data = scene();
    let (mut fight, _bag, mut motion) = setup(&data);
    fight.active = true;
    fight.participants = vec![20];
    fight.rats[0].tile = 115;
    assert!(distance(fight.rats[0].tile, motion.tile) > 12);
    rats_turn(&data, &mut fight, &mut motion);
    assert_ne!(fight.rats[0].tile, 115);
    assert!(fight.active);
}

#[test]
fn checkpoints_restore_mid_conversation_and_completed_quest_without_rendering() {
    let data = scene();
    let (mut fight, mut bag, mut motion) = setup(&data);
    fight.story.begin(&data.dialogue_definition).unwrap();
    let mid = fight.story.clone();
    let file = temp_file();
    save_checkpoint(&file, &data, &fight, &bag, &motion).unwrap();
    fight
        .story
        .choose(&data.dialogue_definition, "hello", "end")
        .unwrap();
    assert_eq!(fight.story.flags.get("water_chip_search"), Some(&1));
    load_checkpoint(&file, &data, &mut fight, &mut bag, &mut motion).unwrap();
    assert_eq!(fight.story, mid);
    fight
        .story
        .choose(&data.dialogue_definition, "hello", "end")
        .unwrap();
    let completed = fight.story.clone();
    save_checkpoint(&file, &data, &fight, &bag, &motion).unwrap();
    fight.story = crate::dialogue::Story::default();
    load_checkpoint(&file, &data, &mut fight, &mut bag, &mut motion).unwrap();
    assert_eq!(fight.story, completed);
    let mut invalid = checkpoint(&data, &fight, &bag, &motion).unwrap();
    invalid.story.flags.insert("unknown_quest".into(), 1);
    assert!(validate_checkpoint(&invalid, &data, &bag).is_err());
    assert_eq!(fight.story, completed);
    std::fs::remove_file(file).unwrap();
}
