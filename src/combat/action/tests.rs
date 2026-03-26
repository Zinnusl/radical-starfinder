use super::*;
use crate::combat::test_helpers::{make_test_battle, make_test_unit};
use crate::combat::{BattleTile, Direction, PlayerStance, UnitKind, WuxingElement};
use crate::status::{StatusInstance, StatusKind};

#[test]
fn flank_bonus_backstab_from_behind() {
    let player = make_test_unit(UnitKind::Player, 3, 4); // south of target
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.facing = Direction::North;
    // Player is south, target faces north → player is behind

    let battle = make_test_battle(vec![player, target]);
    let bonus = flank_bonus(&battle, 0, 1);
    assert!((bonus - 0.50).abs() < f64::EPSILON);
}

#[test]
fn flank_bonus_side_attack() {
    let player = make_test_unit(UnitKind::Player, 4, 3); // east of target
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.facing = Direction::North;
    // Player is east, target faces north → side attack

    let battle = make_test_battle(vec![player, target]);
    let bonus = flank_bonus(&battle, 0, 1);
    assert!((bonus - 0.25).abs() < f64::EPSILON);
}

#[test]
fn flank_bonus_frontal_attack_is_zero() {
    let player = make_test_unit(UnitKind::Player, 3, 2); // north of target
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.facing = Direction::North;
    // Player is north, target faces north → frontal

    let battle = make_test_battle(vec![player, target]);
    let bonus = flank_bonus(&battle, 0, 1);
    assert!((bonus - 0.0).abs() < f64::EPSILON);
}

#[test]
fn is_cornered_at_corner_of_map() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let battle = make_test_battle(vec![player]);
    // (0,0) is corner: north (-1) and west (-1) are out of bounds = 2 walls
    assert!(is_cornered(&battle, 0));
}

#[test]
fn is_cornered_false_in_open_area() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let battle = make_test_battle(vec![player]);
    assert!(!is_cornered(&battle, 0));
}

#[test]
fn is_cornered_next_to_barrier_and_edge() {
    let player = make_test_unit(UnitKind::Player, 0, 3);
    let mut battle = make_test_battle(vec![player]);
    // Place barrier to the south
    battle.arena.set_tile(0, 4, BattleTile::CoverBarrier);
    // West is out of bounds, south is barrier → 2 walls
    assert!(is_cornered(&battle, 0));
}

#[test]
fn deal_damage_basic_reduces_hp() {
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.hp = 10;
    target.max_hp = 10;
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player, target]);

    let actual = deal_damage(&mut battle, 1, 3);
    assert!(actual >= 1); // minimum damage is 1
    assert!(battle.units[1].hp < 10);
}

#[test]
fn deal_damage_kills_unit_at_zero_hp() {
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.hp = 1;
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player, target]);

    deal_damage(&mut battle, 1, 10);
    assert_eq!(battle.units[1].hp, 0);
    assert!(!battle.units[1].alive);
}

#[test]
fn deal_damage_defending_halves_damage() {
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.hp = 20;
    target.max_hp = 20;
    target.defending = true;
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player, target]);

    let actual = deal_damage(&mut battle, 1, 6);
    // 6 / 2 = 3 damage (defending halves)
    assert_eq!(actual, 3);
    assert_eq!(battle.units[1].hp, 17);
}

#[test]
fn deal_damage_from_applies_element_multiplier() {
    let mut attacker = make_test_unit(UnitKind::Player, 0, 0);
    attacker.wuxing_element = Some(WuxingElement::Water);
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.wuxing_element = Some(WuxingElement::Fire);
    target.hp = 20;
    target.max_hp = 20;
    let mut battle = make_test_battle(vec![attacker, target]);

    let (actual, label) = deal_damage_from(&mut battle, 0, 1, 4);
    // Water beats Fire → 1.5x → ceil(6) = 6 raw, min 1 after armor = 6
    assert!(actual >= 5); // at least 5 damage after the 1.5x multiplier
    assert_eq!(label, Some("Super effective!"));
}

#[test]
fn check_status_combos_slow_and_haste_cancel() {
    let mut unit = make_test_unit(UnitKind::Enemy(0), 3, 3);
    unit.statuses.push(StatusInstance::new(StatusKind::Slow, 2));
    unit.statuses.push(StatusInstance::new(StatusKind::Haste, 2));
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player, unit]);

    let msgs = check_status_combos(&mut battle, 1);
    assert!(!msgs.is_empty());
    // Both statuses should be removed
    let has_slow = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Slow));
    let has_haste = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Haste));
    assert!(!has_slow);
    assert!(!has_haste);
}

#[test]
fn check_status_combos_fortify_and_weakened_cancel() {
    let mut unit = make_test_unit(UnitKind::Enemy(0), 3, 3);
    unit.statuses.push(StatusInstance::new(StatusKind::Fortify { stacks: 1 }, 3));
    unit.statuses.push(StatusInstance::new(StatusKind::Weakened, 2));
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player, unit]);

    let msgs = check_status_combos(&mut battle, 1);
    assert!(!msgs.is_empty());
    let has_fortify = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Fortify { .. }));
    let has_weakened = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Weakened));
    assert!(!has_fortify);
    assert!(!has_weakened);
}

// ── move_unit: momentum ────────────────────────────────────────────────

#[test]
fn move_unit_builds_momentum_on_same_direction() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 0, 0);
    let mut battle = make_test_battle(vec![player, enemy]);

    // Move east 3 times in a row
    move_unit(&mut battle, 0, 4, 3);
    assert_eq!(battle.units[0].momentum, 1);
    move_unit(&mut battle, 0, 5, 3);
    assert_eq!(battle.units[0].momentum, 2);
    move_unit(&mut battle, 0, 6, 3);
    assert_eq!(battle.units[0].momentum, 3);
}

#[test]
fn move_unit_resets_momentum_on_direction_change() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 0, 0);
    let mut battle = make_test_battle(vec![player, enemy]);

    move_unit(&mut battle, 0, 4, 3); // east
    assert_eq!(battle.units[0].momentum, 1);
    move_unit(&mut battle, 0, 4, 4); // south (direction change)
    assert_eq!(battle.units[0].momentum, 1); // reset to 1
}

#[test]
fn move_unit_charge_attack_at_momentum_3() {
    let mut player = make_test_unit(UnitKind::Player, 2, 3);
    player.momentum = 2;
    player.last_move_dir = Some(Direction::East);
    let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    let mut battle = make_test_battle(vec![player, enemy]);

    let msgs = move_unit(&mut battle, 0, 3, 3);

    // Player at (3,3), momentum now 3, enemy at (4,3) is adjacent
    assert!(msgs.iter().any(|m| m.contains("charge attack")));
    assert!(battle.units[1].hp < 10); // enemy took damage
    assert_eq!(battle.units[0].momentum, 0); // reset after charge
}

#[test]
fn move_unit_momentum_caps_at_3() {
    let mut player = make_test_unit(UnitKind::Player, 1, 3);
    player.momentum = 3;
    player.last_move_dir = Some(Direction::East);
    let mut battle = make_test_battle(vec![player]);

    move_unit(&mut battle, 0, 2, 3); // east again
    assert_eq!(battle.units[0].momentum, 3); // capped at 3
}

// ── move_unit: terrain effects ─────────────────────────────────────────

#[test]
fn move_unit_coolant_pool_applies_wet() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 0, 0);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::CoolantPool);

    let msgs = move_unit(&mut battle, 0, 4, 3);

    let has_wet = battle.units[0].statuses.iter().any(|s| matches!(s.kind, StatusKind::Wet));
    assert!(has_wet);
    assert!(msgs.iter().any(|m| m.contains("soaked")));
}

#[test]
fn move_unit_coolant_pool_no_double_wet() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.statuses.push(StatusInstance::new(StatusKind::Wet, 3));
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(4, 3, BattleTile::CoolantPool);

    move_unit(&mut battle, 0, 4, 3);

    let wet_count = battle.units[0].statuses.iter().filter(|s| matches!(s.kind, StatusKind::Wet)).count();
    assert_eq!(wet_count, 1); // no duplicate
}

#[test]
fn move_unit_energy_node_heals_player_and_converts_tile() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.hp = 5;
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(4, 3, BattleTile::EnergyNode);

    let msgs = move_unit(&mut battle, 0, 4, 3);

    assert_eq!(battle.units[0].hp, 8); // 5 + 3
    assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::MetalFloor));
    assert!(msgs.iter().any(|m| m.contains("Energy Node")));
}

#[test]
fn move_unit_energy_node_caps_at_max_hp() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.hp = 9;
    player.max_hp = 10;
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(4, 3, BattleTile::EnergyNode);

    move_unit(&mut battle, 0, 4, 3);

    assert_eq!(battle.units[0].hp, 10);
}

#[test]
fn move_unit_energy_node_ignored_for_enemies() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 5;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::EnergyNode);

    move_unit(&mut battle, 1, 4, 3);

    // Enemy should not be healed by EnergyNode (player-only)
    assert_eq!(battle.units[1].hp, 5);
}

#[test]
fn move_unit_phase_walk_converts_non_walkable_tile() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);
    battle.phase_walk_available = true;

    let msgs = move_unit(&mut battle, 0, 4, 3);

    assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::MetalFloor));
    assert!(!battle.phase_walk_available); // consumed
    assert!(msgs.iter().any(|m| m.contains("Phase Walk")));
}

#[test]
fn move_unit_electrified_wire_damages() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.hp = 10;
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(4, 3, BattleTile::ElectrifiedWire);

    let msgs = move_unit(&mut battle, 0, 4, 3);

    assert!(battle.units[0].hp < 10);
    assert!(msgs.iter().any(|m| m.contains("thorns")));
}

// ── defend ─────────────────────────────────────────────────────────────

#[test]
fn defend_sets_defending_flag() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);

    defend(&mut battle, 0);

    assert!(battle.units[0].defending);
}

#[test]
fn defend_resets_momentum() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.momentum = 2;
    player.last_move_dir = Some(Direction::East);
    let mut battle = make_test_battle(vec![player]);

    defend(&mut battle, 0);

    assert_eq!(battle.units[0].momentum, 0);
    assert_eq!(battle.units[0].last_move_dir, None);
}

#[test]
fn defend_logs_for_player() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);

    defend(&mut battle, 0);

    assert!(battle.player_acted);
    assert!(battle.log.iter().any(|m| m.contains("brace")));
}

#[test]
fn defend_no_log_for_enemy() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);

    defend(&mut battle, 1);

    assert!(battle.units[1].defending);
    assert!(battle.log.is_empty());
}

// ── wait ───────────────────────────────────────────────────────────────

#[test]
fn wait_increments_stored_movement() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);

    wait(&mut battle, 0);
    assert_eq!(battle.units[0].stored_movement, 1);

    wait(&mut battle, 0);
    assert_eq!(battle.units[0].stored_movement, 2);
}

#[test]
fn wait_caps_stored_movement_at_2() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.stored_movement = 2;
    let mut battle = make_test_battle(vec![player]);

    wait(&mut battle, 0);

    assert_eq!(battle.units[0].stored_movement, 2); // capped
}

#[test]
fn wait_resets_momentum() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.momentum = 2;
    player.last_move_dir = Some(Direction::East);
    let mut battle = make_test_battle(vec![player]);

    wait(&mut battle, 0);

    assert_eq!(battle.units[0].momentum, 0);
    assert_eq!(battle.units[0].last_move_dir, None);
}

#[test]
fn wait_on_charging_pad_grants_focus() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::ChargingPad);
    battle.focus = 5;
    battle.max_focus = 10;

    wait(&mut battle, 0);

    assert_eq!(battle.focus, 8); // 5 + 3
    assert!(battle.log.iter().any(|m| m.contains("focus")));
}

#[test]
fn wait_on_charging_pad_focus_caps_at_max() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::ChargingPad);
    battle.focus = 9;
    battle.max_focus = 10;

    wait(&mut battle, 0);

    assert_eq!(battle.focus, 10);
}

#[test]
fn wait_on_normal_tile_logs_momentum_message() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);

    wait(&mut battle, 0);

    assert!(battle.player_acted);
    assert!(battle.log.iter().any(|m| m.contains("momentum")));
}

// ── element_multiplier (via deal_damage_from) ──────────────────────────

#[test]
fn element_water_beats_fire() {
    let mut attacker = make_test_unit(UnitKind::Player, 0, 0);
    attacker.wuxing_element = Some(WuxingElement::Water);
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.wuxing_element = Some(WuxingElement::Fire);
    target.hp = 50;
    target.max_hp = 50;
    let mut battle = make_test_battle(vec![attacker, target]);

    let (_, label) = deal_damage_from(&mut battle, 0, 1, 4);
    assert_eq!(label, Some("Super effective!"));
}

#[test]
fn element_fire_beats_metal() {
    let mut attacker = make_test_unit(UnitKind::Player, 0, 0);
    attacker.wuxing_element = Some(WuxingElement::Fire);
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.wuxing_element = Some(WuxingElement::Metal);
    target.hp = 50;
    target.max_hp = 50;
    let mut battle = make_test_battle(vec![attacker, target]);

    let (_, label) = deal_damage_from(&mut battle, 0, 1, 4);
    assert_eq!(label, Some("Super effective!"));
}

#[test]
fn element_metal_beats_wood() {
    let mut attacker = make_test_unit(UnitKind::Player, 0, 0);
    attacker.wuxing_element = Some(WuxingElement::Metal);
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.wuxing_element = Some(WuxingElement::Wood);
    target.hp = 50;
    target.max_hp = 50;
    let mut battle = make_test_battle(vec![attacker, target]);

    let (_, label) = deal_damage_from(&mut battle, 0, 1, 4);
    assert_eq!(label, Some("Super effective!"));
}

#[test]
fn element_wood_beats_earth() {
    let mut attacker = make_test_unit(UnitKind::Player, 0, 0);
    attacker.wuxing_element = Some(WuxingElement::Wood);
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.wuxing_element = Some(WuxingElement::Earth);
    target.hp = 50;
    target.max_hp = 50;
    let mut battle = make_test_battle(vec![attacker, target]);

    let (_, label) = deal_damage_from(&mut battle, 0, 1, 4);
    assert_eq!(label, Some("Super effective!"));
}

#[test]
fn element_earth_beats_water() {
    let mut attacker = make_test_unit(UnitKind::Player, 0, 0);
    attacker.wuxing_element = Some(WuxingElement::Earth);
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.wuxing_element = Some(WuxingElement::Water);
    target.hp = 50;
    target.max_hp = 50;
    let mut battle = make_test_battle(vec![attacker, target]);

    let (_, label) = deal_damage_from(&mut battle, 0, 1, 4);
    assert_eq!(label, Some("Super effective!"));
}

#[test]
fn element_disadvantage_returns_not_effective() {
    let mut attacker = make_test_unit(UnitKind::Player, 0, 0);
    attacker.wuxing_element = Some(WuxingElement::Fire);
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.wuxing_element = Some(WuxingElement::Water);
    target.hp = 50;
    target.max_hp = 50;
    let mut battle = make_test_battle(vec![attacker, target]);

    let (_, label) = deal_damage_from(&mut battle, 0, 1, 4);
    assert_eq!(label, Some("Not very effective..."));
}

#[test]
fn element_neutral_returns_none() {
    let mut attacker = make_test_unit(UnitKind::Player, 0, 0);
    attacker.wuxing_element = Some(WuxingElement::Fire);
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.wuxing_element = Some(WuxingElement::Fire); // same element, neutral
    target.hp = 50;
    target.max_hp = 50;
    let mut battle = make_test_battle(vec![attacker, target]);

    let (_, label) = deal_damage_from(&mut battle, 0, 1, 4);
    assert_eq!(label, None);
}

#[test]
fn element_none_attacker_is_neutral() {
    let attacker = make_test_unit(UnitKind::Player, 0, 0);
    let mut target = make_test_unit(UnitKind::Enemy(0), 3, 3);
    target.wuxing_element = Some(WuxingElement::Fire);
    target.hp = 50;
    target.max_hp = 50;
    let mut battle = make_test_battle(vec![attacker, target]);

    let (_, label) = deal_damage_from(&mut battle, 0, 1, 4);
    assert_eq!(label, None);
}

// ── check_status_combos: Wet + Burn ────────────────────────────────────

#[test]
fn check_status_combos_wet_and_burn_creates_steam() {
    let mut unit = make_test_unit(UnitKind::Enemy(0), 3, 3);
    unit.statuses.push(StatusInstance::new(StatusKind::Wet, 3));
    unit.statuses.push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 3));
    unit.hp = 10;
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player, unit]);

    let msgs = check_status_combos(&mut battle, 1);

    assert!(msgs.iter().any(|m| m.contains("Steam")));
    // Both Wet and Burn should be removed
    let has_wet = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Wet));
    let has_burn = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Burn { .. }));
    assert!(!has_wet);
    assert!(!has_burn);
    // Steam tile should be placed
    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::VentSteam));
    // 1 damage dealt
    assert!(battle.units[1].hp < 10);
}

// ── check_status_combos: Poison + Burn ─────────────────────────────────

#[test]
fn check_status_combos_poison_and_burn_creates_toxic_fumes() {
    let mut unit = make_test_unit(UnitKind::Enemy(0), 3, 3);
    unit.statuses.push(StatusInstance::new(StatusKind::Poison { damage: 1 }, 3));
    unit.statuses.push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 3));
    let player = make_test_unit(UnitKind::Player, 4, 3); // within 2 tiles
    let mut battle = make_test_battle(vec![player, unit]);

    let msgs = check_status_combos(&mut battle, 1);

    assert!(msgs.iter().any(|m| m.contains("Toxic Fumes")));
    // Both Poison and Burn should be removed
    let has_poison = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Poison { .. }));
    let has_burn = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Burn { .. }));
    assert!(!has_poison);
    assert!(!has_burn);
    // Player within 2 tiles gets damage + Confused
    assert!(battle.units[0].hp < 10);
    let has_confused = battle.units[0].statuses.iter().any(|s| matches!(s.kind, StatusKind::Confused));
    assert!(has_confused);
}

// ── check_status_combos: Blessed + Cursed ──────────────────────────────

#[test]
fn check_status_combos_blessed_and_cursed_cancel() {
    let mut unit = make_test_unit(UnitKind::Enemy(0), 3, 3);
    unit.statuses.push(StatusInstance::new(StatusKind::Blessed, 3));
    unit.statuses.push(StatusInstance::new(StatusKind::Cursed, 3));
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player, unit]);

    let msgs = check_status_combos(&mut battle, 1);

    assert!(!msgs.is_empty());
    let has_blessed = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Blessed));
    let has_cursed = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Cursed));
    assert!(!has_blessed);
    assert!(!has_cursed);
}

// ── move_unit sets player_moved ────────────────────────────────────────

#[test]
fn move_unit_sets_player_moved() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);

    move_unit(&mut battle, 0, 4, 3);

    assert!(battle.player_moved);
}

#[test]
fn move_unit_clears_stored_movement() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.stored_movement = 2;
    let mut battle = make_test_battle(vec![player]);

    move_unit(&mut battle, 0, 4, 3);

    assert_eq!(battle.units[0].stored_movement, 0);
}

