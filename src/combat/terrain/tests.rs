use super::*;
use crate::combat::test_helpers::{make_test_battle, make_test_unit};
use crate::combat::{BattleTile, UnitKind};

#[test]
fn step_on_weakened_plating_becomes_damaged() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(2, 2, BattleTile::WeakenedPlating);

    let msgs = step_on_crumbling(&mut battle, 2, 2);
    assert_eq!(battle.arena.tile(2, 2), Some(BattleTile::DamagedFloor));
    assert!(!msgs.is_empty());
}

#[test]
fn step_on_damaged_floor_becomes_breached() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(2, 2, BattleTile::DamagedFloor);

    let msgs = step_on_crumbling(&mut battle, 2, 2);
    assert_eq!(battle.arena.tile(2, 2), Some(BattleTile::BreachedFloor));
    assert!(!msgs.is_empty());
}

#[test]
fn step_on_normal_floor_does_nothing() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);

    let msgs = step_on_crumbling(&mut battle, 2, 2);
    assert_eq!(battle.arena.tile(2, 2), Some(BattleTile::MetalFloor));
    assert!(msgs.is_empty());
}

#[test]
fn decay_cracked_floors_collapses_unoccupied() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::DamagedFloor);
    battle.arena.set_tile(4, 4, BattleTile::DamagedFloor);

    let msgs = decay_cracked_floors(&mut battle);
    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::BreachedFloor));
    assert_eq!(battle.arena.tile(4, 4), Some(BattleTile::BreachedFloor));
    assert_eq!(msgs.len(), 2);
}

#[test]
fn decay_cracked_floors_spares_occupied_tiles() {
    // Player is standing on the damaged floor
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::DamagedFloor);

    let msgs = decay_cracked_floors(&mut battle);
    // Should not collapse because player is standing there
    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::DamagedFloor));
    assert!(msgs.is_empty());
}

#[test]
fn trigger_trap_on_mine_deals_damage_and_slows() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.hp = 10;
    player.max_hp = 10;
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::MineTile);

    let msgs = trigger_trap(&mut battle, 0, 3, 3);
    assert!(!msgs.is_empty());
    // Took damage
    assert!(battle.units[0].hp < 10);
    // Got Slow status
    let has_slow = battle.units[0].statuses.iter().any(|s| matches!(s.kind, StatusKind::Slow));
    assert!(has_slow);
    // Mine revealed
    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::MineTileRevealed));
}

#[test]
fn trigger_trap_on_normal_floor_does_nothing() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.hp = 10;
    let mut battle = make_test_battle(vec![player]);

    let msgs = trigger_trap(&mut battle, 0, 3, 3);
    assert!(msgs.is_empty());
    assert_eq!(battle.units[0].hp, 10);
}

#[test]
fn terrain_roll_deterministic() {
    // Same inputs always produce same result
    let r1 = terrain_roll(5, 10, 50);
    let r2 = terrain_roll(5, 10, 50);
    assert_eq!(r1, r2);

    // Different inputs can produce different results
    let r3 = terrain_roll(5, 10, 50);
    let r4 = terrain_roll(6, 10, 50);
    // These might be different (probabilistic, but deterministic)
    let _ = (r3, r4); // just ensure no panic

    // Chance 0 always false, chance 100 always true
    assert!(!terrain_roll(1, 1, 0));
    assert!(terrain_roll(1, 1, 100));
}

