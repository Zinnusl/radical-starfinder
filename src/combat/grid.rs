use crate::combat::{TacticalArena, TacticalBattle, Weather};
use std::collections::VecDeque;

pub fn manhattan(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    (x1 - x2).abs() + (y1 - y2).abs()
}

/// Flood-fill reachable tiles from (start_x, start_y) with `movement` points.
/// Returns list of (x, y, cost) that can be reached. Accounts for terrain
/// extra_move_cost and obstacles. Does NOT allow walking through occupied tiles
/// (except the start tile).
pub fn reachable_tiles(
    battle: &TacticalBattle,
    start_x: i32,
    start_y: i32,
    movement: i32,
) -> Vec<(i32, i32)> {
    let arena = &battle.arena;
    let w = arena.width as i32;
    let h = arena.height as i32;

    let mut cost_map = vec![i32::MAX; arena.width * arena.height];
    let start_idx = (start_y * w + start_x) as usize;
    cost_map[start_idx] = 0;

    let mut queue = VecDeque::new();
    queue.push_back((start_x, start_y, 0i32));

    let deltas: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

    while let Some((cx, cy, cur_cost)) = queue.pop_front() {
        for (dx, dy) in &deltas {
            let nx = cx + dx;
            let ny = cy + dy;
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            let tile = arena.tiles[(ny * w + nx) as usize];
            if !tile.is_walkable() {
                continue;
            }
            // Can't move through other units (but can be on start tile).
            if (nx != start_x || ny != start_y) && battle.unit_at(nx, ny).is_some() {
                continue;
            }
            let mut step_cost = 1 + tile.extra_move_cost();
            if battle.weather == Weather::DebrisStorm {
                step_cost += 1;
            }
            let new_cost = cur_cost + step_cost;
            if new_cost > movement {
                continue;
            }
            let n_idx = (ny * w + nx) as usize;
            if new_cost < cost_map[n_idx] {
                cost_map[n_idx] = new_cost;
                queue.push_back((nx, ny, new_cost));
            }
        }
    }

    let mut result = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if cost_map[idx] <= movement && (x != start_x || y != start_y) {
                // Only include if tile is not occupied by another unit.
                if battle.unit_at(x, y).is_none() {
                    result.push((x, y));
                }
            }
        }
    }
    result
}

/// Bresenham line-of-sight check between two tiles.
pub fn has_line_of_sight(arena: &TacticalArena, x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
    let dx = (x2 - x1).abs();
    let dy = -(y2 - y1).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x1;
    let mut y = y1;

    loop {
        if x == x2 && y == y2 {
            return true;
        }
        // Check intermediate tiles (not start/end).
        if (x != x1 || y != y1) && (x != x2 || y != y2) {
            if let Some(tile) = arena.tile(x, y) {
                if tile.blocks_los() {
                    return false;
                }
            } else {
                return false;
            }
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Get all tiles within range (Manhattan distance) that have line of sight.
pub fn tiles_in_range_with_los(
    arena: &TacticalArena,
    origin_x: i32,
    origin_y: i32,
    range: i32,
) -> Vec<(i32, i32)> {
    let mut result = Vec::new();
    let w = arena.width as i32;
    let h = arena.height as i32;
    for y in 0..h {
        for x in 0..w {
            if manhattan(origin_x, origin_y, x, y) <= range
                && has_line_of_sight(arena, origin_x, origin_y, x, y)
            {
                result.push((x, y));
            }
        }
    }
    result
}

/// Adjust range based on weather conditions (e.g., Fog reduces range).
pub fn weather_adjusted_range(base_range: i32, weather: Weather) -> i32 {
    match weather {
        Weather::SmokeScreen => (base_range - 2).max(1),
        _ => base_range,
    }
}

#[cfg(test)]
mod tests {
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
}
