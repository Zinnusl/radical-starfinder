use super::*;
use crate::combat::test_helpers::{make_test_battle, make_test_unit};
use crate::combat::{Direction, PlayerStance, UnitKind};
use crate::status::StatusInstance;

#[test]
fn player_base_speed_fast_classes() {
    assert_eq!(player_base_speed(PlayerClass::Operative), 5);
    assert_eq!(player_base_speed(PlayerClass::Solarian), 5);
}

#[test]
fn player_base_speed_slow_classes() {
    assert_eq!(player_base_speed(PlayerClass::Soldier), 3);
    assert_eq!(player_base_speed(PlayerClass::Mechanic), 3);
}

#[test]
fn player_base_speed_normal_classes() {
    assert_eq!(player_base_speed(PlayerClass::Envoy), 4);
    assert_eq!(player_base_speed(PlayerClass::Mystic), 4);
    assert_eq!(player_base_speed(PlayerClass::Technomancer), 4);
}

#[test]
fn player_speed_haste_adds_two() {
    let statuses = vec![StatusInstance::new(crate::status::StatusKind::Haste, 2)];
    let speed = player_speed(PlayerClass::Envoy, PlayerForm::Human, &statuses);
    assert_eq!(speed, 6); // base 4 + 2 haste
}

#[test]
fn player_speed_void_form_adds_two() {
    let speed = player_speed(PlayerClass::Envoy, PlayerForm::Void, &[]);
    assert_eq!(speed, 6); // base 4 + 2 void
}

#[test]
fn player_speed_cybernetic_form_subtracts_one() {
    let speed = player_speed(PlayerClass::Envoy, PlayerForm::Cybernetic, &[]);
    assert_eq!(speed, 3); // base 4 - 1 cybernetic
}

#[test]
fn enemy_base_speed_boss_and_elite_are_four() {
    assert_eq!(enemy_base_speed(false, true), 4);
    assert_eq!(enemy_base_speed(true, false), 4);
}

#[test]
fn enemy_base_speed_normal_is_three() {
    assert_eq!(enemy_base_speed(false, false), 3);
}

#[test]
fn enemy_base_movement_elite_or_boss_is_three() {
    assert_eq!(enemy_base_movement(true, false), 3);
    assert_eq!(enemy_base_movement(false, true), 3);
    assert_eq!(enemy_base_movement(true, true), 3);
}

#[test]
fn enemy_base_movement_normal_is_two() {
    assert_eq!(enemy_base_movement(false, false), 2);
}

#[test]
fn player_movement_base_is_three() {
    assert_eq!(player_base_movement(), 3);
}

#[test]
fn player_movement_mobile_stance_adds_two() {
    let mv = player_movement(PlayerForm::Human, &[], PlayerStance::Mobile);
    assert_eq!(mv, 5); // base 3 + 2
}

#[test]
fn player_movement_aggressive_stance_subtracts_one() {
    let mv = player_movement(PlayerForm::Human, &[], PlayerStance::Aggressive);
    assert_eq!(mv, 2); // base 3 - 1
}

#[test]
fn player_movement_minimum_is_one() {
    // Focused stance: -1 movement = 2, still above 1
    let mv = player_movement(PlayerForm::Human, &[], PlayerStance::Focused);
    assert_eq!(mv, 2);
}

#[test]
fn build_turn_queue_sorts_by_speed_descending() {
    let mut player = make_test_unit(UnitKind::Player, 0, 0);
    player.speed = 3;
    let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 2, 0);
    enemy1.speed = 5;
    let mut enemy2 = make_test_unit(UnitKind::Enemy(1), 4, 0);
    enemy2.speed = 2;

    let queue = build_turn_queue(&[player, enemy1, enemy2]);
    // enemy1 (speed 5) first, player (speed 3) second, enemy2 (speed 2) last
    assert_eq!(queue, vec![1, 0, 2]);
}

#[test]
fn build_turn_queue_player_wins_ties() {
    let mut player = make_test_unit(UnitKind::Player, 0, 0);
    player.speed = 4;
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 2, 0);
    enemy.speed = 4;

    let queue = build_turn_queue(&[player, enemy]);
    // Same speed → player goes first
    assert_eq!(queue[0], 0);
    assert_eq!(queue[1], 1);
}

#[test]
fn build_turn_queue_excludes_dead_units() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut dead_enemy = make_test_unit(UnitKind::Enemy(0), 2, 0);
    dead_enemy.alive = false;
    let alive_enemy = make_test_unit(UnitKind::Enemy(1), 4, 0);

    let queue = build_turn_queue(&[player, dead_enemy, alive_enemy]);
    assert_eq!(queue.len(), 2);
    assert!(!queue.contains(&1)); // dead enemy excluded
}

#[test]
fn advance_turn_wraps_and_increments_turn_number() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.turn_queue = vec![0, 1];
    battle.turn_queue_pos = 0;
    battle.turn_number = 1;

    // Advance past unit 0 → unit 1
    let wrapped = advance_turn(&mut battle);
    assert!(!wrapped);

    // Advance past unit 1 → wraps to new round
    let wrapped = advance_turn(&mut battle);
    assert!(wrapped);
    assert_eq!(battle.turn_number, 2);
}

#[test]
fn advance_turn_skips_dead_units() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut dead = make_test_unit(UnitKind::Enemy(0), 2, 2);
    dead.alive = false;
    let alive = make_test_unit(UnitKind::Enemy(1), 4, 4);
    let mut battle = make_test_battle(vec![player, dead, alive]);
    battle.turn_queue = vec![0, 1, 2];
    battle.turn_queue_pos = 0;

    let wrapped = advance_turn(&mut battle);
    assert!(!wrapped);
    // Should skip dead unit (index 1) and land on alive unit (index 2)
    assert_eq!(battle.turn_queue[battle.turn_queue_pos], 2);
}

