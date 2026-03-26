use super::*;
use crate::combat::{ArenaBiome, BattleTile, UnitKind, Weather};
use crate::combat::test_helpers::{make_test_battle, make_test_unit};

#[test]
fn manhattan_same_point_is_zero() {
    assert_eq!(manhattan(3, 4, 3, 4), 0);
}

#[test]
fn manhattan_basic_distances() {
    assert_eq!(manhattan(0, 0, 3, 4), 7);
    assert_eq!(manhattan(0, 0, 1, 0), 1);
    assert_eq!(manhattan(2, 3, 5, 7), 7);
}

#[test]
fn manhattan_is_symmetric() {
    assert_eq!(manhattan(1, 2, 5, 7), manhattan(5, 7, 1, 2));
    assert_eq!(manhattan(-2, 3, 4, -1), manhattan(4, -1, -2, 3));
}

#[test]
fn manhattan_with_negative_coordinates() {
    assert_eq!(manhattan(-2, -3, 2, 3), 10);
}

#[test]
fn weather_adjusted_range_normal_unchanged() {
    assert_eq!(weather_adjusted_range(5, Weather::Normal), 5);
    assert_eq!(weather_adjusted_range(5, Weather::CoolantLeak), 5);
    assert_eq!(weather_adjusted_range(5, Weather::DebrisStorm), 5);
    assert_eq!(weather_adjusted_range(5, Weather::EnergyFlux), 5);
}

#[test]
fn weather_adjusted_range_smoke_screen_reduces_by_two() {
    assert_eq!(weather_adjusted_range(5, Weather::SmokeScreen), 3);
    assert_eq!(weather_adjusted_range(4, Weather::SmokeScreen), 2);
}

#[test]
fn weather_adjusted_range_smoke_screen_clamps_to_one() {
    assert_eq!(weather_adjusted_range(2, Weather::SmokeScreen), 1);
    assert_eq!(weather_adjusted_range(1, Weather::SmokeScreen), 1);
}

#[test]
fn line_of_sight_clear_arena() {
    let arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
    assert!(has_line_of_sight(&arena, 0, 0, 6, 6));
    assert!(has_line_of_sight(&arena, 0, 0, 0, 6));
    assert!(has_line_of_sight(&arena, 3, 3, 6, 0));
}

#[test]
fn line_of_sight_same_tile_always_true() {
    let arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
    assert!(has_line_of_sight(&arena, 3, 3, 3, 3));
}

#[test]
fn line_of_sight_blocked_by_barrier() {
    let mut arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
    arena.set_tile(3, 3, BattleTile::CoverBarrier);
    // Straight line through the barrier
    assert!(!has_line_of_sight(&arena, 0, 3, 6, 3));
    assert!(!has_line_of_sight(&arena, 3, 0, 3, 6));
}

#[test]
fn tiles_in_range_with_los_respects_distance_and_barriers() {
    let mut arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
    arena.set_tile(2, 0, BattleTile::CoverBarrier);

    let tiles = tiles_in_range_with_los(&arena, 0, 0, 3);

    // Origin is in range
    assert!(tiles.contains(&(0, 0)));
    // Adjacent tiles visible
    assert!(tiles.contains(&(1, 0)));
    assert!(tiles.contains(&(0, 1)));
    // (3,0) is behind the barrier at (2,0) — blocked
    assert!(!tiles.contains(&(3, 0)));
    // (4,0) is out of range (distance 4 > 3) — excluded
    assert!(!tiles.contains(&(4, 0)));
}

// ── Additional manhattan tests ────────────────────────────────────

#[test]
fn manhattan_adjacent_tiles() {
    assert_eq!(manhattan(3, 3, 4, 3), 1);
    assert_eq!(manhattan(3, 3, 3, 4), 1);
    assert_eq!(manhattan(3, 3, 2, 3), 1);
    assert_eq!(manhattan(3, 3, 3, 2), 1);
}

#[test]
fn manhattan_diagonal_is_two() {
    assert_eq!(manhattan(0, 0, 1, 1), 2);
    assert_eq!(manhattan(3, 3, 4, 4), 2);
}

// ── Additional weather_adjusted_range ─────────────────────────────

#[test]
fn weather_adjusted_range_large_range_smoke() {
    assert_eq!(weather_adjusted_range(10, Weather::SmokeScreen), 8);
}

// ── Additional LOS tests ─────────────────────────────────────────

#[test]
fn line_of_sight_adjacent_to_barrier_is_visible() {
    let mut arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
    arena.set_tile(3, 3, BattleTile::CoverBarrier);
    assert!(has_line_of_sight(&arena, 2, 3, 3, 3));
    assert!(has_line_of_sight(&arena, 3, 3, 4, 3));
}

#[test]
fn line_of_sight_blocked_by_pipe_tangle() {
    let mut arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
    arena.set_tile(3, 3, BattleTile::PipeTangle);
    assert!(!has_line_of_sight(&arena, 0, 3, 6, 3));
}

#[test]
fn line_of_sight_blocked_by_vent_steam() {
    let mut arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
    arena.set_tile(3, 3, BattleTile::VentSteam);
    assert!(!has_line_of_sight(&arena, 0, 3, 6, 3));
}

#[test]
fn line_of_sight_not_blocked_by_walkable_tiles() {
    let mut arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
    arena.set_tile(3, 3, BattleTile::CoolantPool);
    assert!(has_line_of_sight(&arena, 0, 3, 6, 3));
    arena.set_tile(3, 3, BattleTile::OilSlick);
    assert!(has_line_of_sight(&arena, 0, 3, 6, 3));
}

#[test]
fn line_of_sight_diagonal_blocked_by_barrier() {
    let mut arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
    arena.set_tile(2, 2, BattleTile::CoverBarrier);
    assert!(!has_line_of_sight(&arena, 0, 0, 4, 4));
}

#[test]
fn line_of_sight_diagonal_clear() {
    let arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
    assert!(has_line_of_sight(&arena, 0, 0, 6, 6));
    assert!(has_line_of_sight(&arena, 6, 0, 0, 6));
}

#[test]
fn line_of_sight_around_barrier() {
    let mut arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
    arena.set_tile(3, 3, BattleTile::CoverBarrier);
    // Can see around the barrier at an angle that misses (3,3)
    assert!(has_line_of_sight(&arena, 0, 0, 6, 4));
}

// ── Additional tiles_in_range_with_los ────────────────────────────

#[test]
fn tiles_in_range_with_los_range_zero_returns_only_origin() {
    let arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
    let tiles = tiles_in_range_with_los(&arena, 3, 3, 0);
    assert_eq!(tiles, vec![(3, 3)]);
}

#[test]
fn tiles_in_range_with_los_range_one_returns_adjacent() {
    let arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
    let tiles = tiles_in_range_with_los(&arena, 3, 3, 1);
    assert!(tiles.contains(&(3, 3)));
    assert!(tiles.contains(&(2, 3)));
    assert!(tiles.contains(&(4, 3)));
    assert!(tiles.contains(&(3, 2)));
    assert!(tiles.contains(&(3, 4)));
    assert_eq!(tiles.len(), 5);
}

#[test]
fn tiles_in_range_with_los_wall_blocks_tiles_behind() {
    let mut arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
    for x in 0..7 {
        arena.set_tile(x, 2, BattleTile::CoverBarrier);
    }
    let tiles = tiles_in_range_with_los(&arena, 3, 0, 5);
    for &(_, ty) in &tiles {
        assert!(ty < 3, "should not see past wall at y=2, got tile at y={}", ty);
    }
}

// ── reachable_tiles ───────────────────────────────────────────────

#[test]
fn reachable_tiles_open_arena_movement_1() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let battle = make_test_battle(vec![player]);
    let tiles = reachable_tiles(&battle, 3, 3, 1);
    assert!(tiles.contains(&(2, 3)));
    assert!(tiles.contains(&(4, 3)));
    assert!(tiles.contains(&(3, 2)));
    assert!(tiles.contains(&(3, 4)));
    assert_eq!(tiles.len(), 4);
}

#[test]
fn reachable_tiles_open_arena_movement_2() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let battle = make_test_battle(vec![player]);
    let tiles = reachable_tiles(&battle, 3, 3, 2);
    for &(x, y) in &tiles {
        assert!(manhattan(3, 3, x, y) <= 2);
    }
    assert!(!tiles.contains(&(3, 3)));
    assert!(tiles.contains(&(2, 2)));
    assert!(tiles.contains(&(4, 4)));
}

#[test]
fn reachable_tiles_blocked_by_barrier() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(2, 3, BattleTile::CoverBarrier);
    battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);
    battle.arena.set_tile(3, 2, BattleTile::CoverBarrier);
    let tiles = reachable_tiles(&battle, 3, 3, 3);
    assert!(!tiles.contains(&(2, 3)));
    assert!(!tiles.contains(&(4, 3)));
    assert!(!tiles.contains(&(3, 2)));
    assert!(tiles.contains(&(3, 4)));
}

#[test]
fn reachable_tiles_does_not_include_start() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let battle = make_test_battle(vec![player]);
    let tiles = reachable_tiles(&battle, 3, 3, 5);
    assert!(!tiles.contains(&(3, 3)));
}

#[test]
fn reachable_tiles_blocked_by_occupied_tile() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    let battle = make_test_battle(vec![player, enemy]);
    let tiles = reachable_tiles(&battle, 3, 3, 1);
    assert!(!tiles.contains(&(4, 3)));
    assert!(tiles.contains(&(2, 3)));
}

#[test]
fn reachable_tiles_movement_zero_returns_empty() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let battle = make_test_battle(vec![player]);
    let tiles = reachable_tiles(&battle, 3, 3, 0);
    assert!(tiles.is_empty());
}

#[test]
fn reachable_tiles_corner_has_fewer_tiles() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let battle = make_test_battle(vec![player]);
    let tiles = reachable_tiles(&battle, 0, 0, 1);
    assert_eq!(tiles.len(), 2);
    assert!(tiles.contains(&(1, 0)));
    assert!(tiles.contains(&(0, 1)));
}

#[test]
fn reachable_tiles_extra_move_cost_reduces_reach() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(4, 3, BattleTile::CoolantPool);
    let tiles_with_pool = reachable_tiles(&battle, 3, 3, 1);
    assert!(!tiles_with_pool.contains(&(4, 3)));

    let tiles_with_2 = reachable_tiles(&battle, 3, 3, 2);
    assert!(tiles_with_2.contains(&(4, 3)));
}

#[test]
fn reachable_tiles_debris_storm_increases_cost() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);
    battle.weather = Weather::DebrisStorm;
    let tiles = reachable_tiles(&battle, 3, 3, 1);
    assert!(tiles.is_empty());

    let tiles2 = reachable_tiles(&battle, 3, 3, 2);
    assert_eq!(tiles2.len(), 4);
}

#[test]
fn reachable_tiles_phase_walk_through_barrier() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);
    battle.phase_walk_available = true;
    let tiles = reachable_tiles(&battle, 3, 3, 1);
    assert!(tiles.contains(&(4, 3)));
}

#[test]
fn reachable_tiles_phase_walk_disabled() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);
    battle.phase_walk_available = false;
    let tiles = reachable_tiles(&battle, 3, 3, 1);
    assert!(!tiles.contains(&(4, 3)));
}

#[test]
fn reachable_tiles_phase_walk_not_onto_occupied_barrier() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);
    battle.phase_walk_available = true;
    let tiles = reachable_tiles(&battle, 3, 3, 1);
    assert!(!tiles.contains(&(4, 3)));
}

#[test]
fn reachable_tiles_large_movement_covers_whole_arena() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let battle = make_test_battle(vec![player]);
    let tiles = reachable_tiles(&battle, 3, 3, 20);
    assert_eq!(tiles.len(), 48);
}

