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

    // PhaseWalk: allow stepping onto one adjacent non-walkable tile.
    if battle.phase_walk_available {
        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                if cost_map[idx] >= movement || cost_map[idx] == i32::MAX {
                    continue;
                }
                for (dx, dy) in &deltas {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    let ntile = arena.tiles[(ny * w + nx) as usize];
                    if !ntile.is_walkable() && battle.unit_at(nx, ny).is_none() && !result.contains(&(nx, ny)) {
                        result.push((nx, ny));
                    }
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
mod tests;
