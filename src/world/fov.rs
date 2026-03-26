//! Field-of-view using recursive shadowcasting.
//!
//! Reveals tiles visible from a point within a given radius.
//! Uses the classic 8-octant symmetric shadowcasting algorithm.

use super::DungeonLevel;

/// Compute FOV from (ox, oy) with given radius. Updates `visible` and `revealed`.
pub fn compute_fov(level: &mut DungeonLevel, ox: i32, oy: i32, radius: i32) {
    let size = (level.width * level.height) as usize;
    // Clear previous frame visibility
    level.visible = vec![false; size];

    // Origin is always visible
    if level.in_bounds(ox, oy) {
        let idx = level.idx(ox, oy);
        level.visible[idx] = true;
        level.revealed[idx] = true;
    }

    for octant in 0..8 {
        cast_light(level, ox, oy, radius, 1, 1.0, 0.0, octant);
    }
}

/// Multipliers for the 8 octants.
const MULT: [[i32; 8]; 4] = [
    [1, 0, 0, -1, -1, 0, 0, 1],
    [0, 1, -1, 0, 0, -1, 1, 0],
    [0, 1, 1, 0, 0, -1, -1, 0],
    [1, 0, 0, 1, -1, 0, 0, -1],
];

#[allow(clippy::too_many_arguments)]
fn cast_light(
    level: &mut DungeonLevel,
    ox: i32,
    oy: i32,
    radius: i32,
    row: i32,
    mut start_slope: f64,
    end_slope: f64,
    octant: usize,
) {
    if start_slope < end_slope {
        return;
    }

    let mut next_start_slope = start_slope;

    for j in row..=radius {
        let mut blocked = false;

        let dy = -j;
        for dx in -j..=0 {
            let l_slope = (dx as f64 - 0.5) / (dy as f64 + 0.5);
            let r_slope = (dx as f64 + 0.5) / (dy as f64 - 0.5);

            if start_slope < r_slope {
                continue;
            }
            if end_slope > l_slope {
                break;
            }

            let map_x = ox + dx * MULT[0][octant] + dy * MULT[1][octant];
            let map_y = oy + dx * MULT[2][octant] + dy * MULT[3][octant];

            // Distance check (circular FOV)
            if dx * dx + dy * dy <= radius * radius && level.in_bounds(map_x, map_y) {
                    let idx = level.idx(map_x, map_y);
                    level.visible[idx] = true;
                    level.revealed[idx] = true;
            }

            let is_wall = !level.in_bounds(map_x, map_y) || !level.tile(map_x, map_y).is_walkable();

            if blocked {
                if is_wall {
                    next_start_slope = r_slope;
                } else {
                    blocked = false;
                    start_slope = next_start_slope;
                }
            } else if is_wall && j < radius {
                blocked = true;
                cast_light(level, ox, oy, radius, j + 1, start_slope, l_slope, octant);
                next_start_slope = r_slope;
            }
        }

        if blocked {
            break;
        }
    }
}
