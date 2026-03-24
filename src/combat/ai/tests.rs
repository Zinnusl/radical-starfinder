use super::*;
use crate::combat::test_helpers::{make_test_battle, make_test_unit};
use crate::combat::{BattleTile, UnitKind};

#[test]
fn step_toward_moves_closer_to_target() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 5, 5);
    let battle = make_test_battle(vec![player, enemy]);

    let result = step_toward(&battle, 3, 3, 6, 6);
    assert!(result.is_some());
    let (nx, ny) = result.unwrap();
    // Should move closer to (6,6)
    let old_dist = manhattan(3, 3, 6, 6);
    let new_dist = manhattan(nx, ny, 6, 6);
    assert!(new_dist < old_dist);
}

#[test]
fn step_toward_avoids_occupied_tiles() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    // Place a blocking unit to the east
    let blocker = make_test_unit(UnitKind::Enemy(0), 4, 3);
    let battle = make_test_battle(vec![player, blocker]);

    let result = step_toward(&battle, 3, 3, 6, 3);
    // Can't step east (occupied), should pick another direction
    match result {
        Some((x, y)) => assert!(!(x == 4 && y == 3)), // not into blocker
        None => {} // acceptable if all adjacent are occupied/blocked
    }
}

#[test]
fn step_toward_avoids_unwalkable_tiles() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    // Block east with a barrier
    battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);

    let result = step_toward(&battle, 3, 3, 6, 3);
    match result {
        Some((x, y)) => assert!(!(x == 4 && y == 3)),
        None => {}
    }
}

#[test]
fn step_away_moves_farther_from_threat() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let battle = make_test_battle(vec![player]);

    let result = step_away(&battle, 3, 3, 2, 3);
    assert!(result.is_some());
    let (nx, ny) = result.unwrap();
    let old_dist = manhattan(3, 3, 2, 3);
    let new_dist = manhattan(nx, ny, 2, 3);
    assert!(new_dist > old_dist);
}

#[test]
fn step_away_respects_bounds() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let battle = make_test_battle(vec![player]);

    // Try to step away from center — should stay in bounds
    let result = step_away(&battle, 0, 0, 3, 3);
    if let Some((x, y)) = result {
        assert!(battle.arena.in_bounds(x, y));
    }
}

#[test]
fn count_allies_near_counts_same_faction() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let enemy2 = make_test_unit(UnitKind::Enemy(1), 4, 3); // within 1 of enemy1
    let enemy3 = make_test_unit(UnitKind::Enemy(2), 6, 6); // far from enemy1

    let battle = make_test_battle(vec![player, enemy1, enemy2, enemy3]);
    let count = count_allies_near(&battle, 1, 2); // radius 2
    assert_eq!(count, 1); // only enemy2 is within range
}

#[test]
fn get_radical_role_classifies_correctly() {
    assert!(matches!(get_radical_role(&RadicalAction::SpreadingWildfire), TacticalRole::Offensive));
    assert!(matches!(get_radical_role(&RadicalAction::MortalResilience), TacticalRole::Defensive));
    assert!(matches!(get_radical_role(&RadicalAction::ErosiveFlow), TacticalRole::Debuff));
    assert!(matches!(get_radical_role(&RadicalAction::RevealingDawn), TacticalRole::Support));
}

