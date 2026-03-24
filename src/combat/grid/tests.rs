use super::*;
use crate::combat::{ArenaBiome, BattleTile, Weather};

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

