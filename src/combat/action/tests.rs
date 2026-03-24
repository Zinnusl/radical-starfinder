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

