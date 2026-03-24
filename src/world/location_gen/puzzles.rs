//! Puzzle and secret room placement.

use super::super::*;

impl LocationLevel {
    pub(super) fn place_bridge_setup_in_room(&mut self, room: &Room, rng: &mut Rng) -> bool {
        let mut water_tiles = Vec::new();
        for y in room.y + 1..room.y + room.h - 1 {
            for x in room.x + 1..room.x + room.w - 1 {
                if self.tile(x, y) == Tile::CoolantPool {
                    water_tiles.push((x, y));
                }
            }
        }

        if water_tiles.is_empty() {
            return false;
        }

        let water_start = (rng.next_u64() as usize) % water_tiles.len();
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for water_offset in 0..water_tiles.len() {
            let (wx, wy) = water_tiles[(water_start + water_offset) % water_tiles.len()];
            let dir_start = (rng.next_u64() % 4) as usize;
            for dir_offset in 0..4 {
                let (dx, dy) = dirs[(dir_start + dir_offset) % 4];
                let crate_x = wx - dx;
                let crate_y = wy - dy;
                let stand_x = crate_x - dx;
                let stand_y = crate_y - dy;

                if !self.in_bounds(crate_x, crate_y) || !self.in_bounds(stand_x, stand_y) {
                    continue;
                }

                if self.tile(crate_x, crate_y) != Tile::MetalFloor {
                    continue;
                }

                let stand_tile = self.tile(stand_x, stand_y);
                if !stand_tile.is_walkable() || stand_tile == Tile::CoolantPool {
                    continue;
                }

                let idx = self.idx(crate_x, crate_y);
                self.tiles[idx] = Tile::SalvageCrate;
                return true;
            }
        }

        false
    }

    pub(super) fn place_bridge_setup(&mut self, rng: &mut Rng) -> bool {
        if self.rooms.len() < 4 {
            return false;
        }

        let room_count = self.rooms.len().saturating_sub(2);
        if room_count == 0 {
            return false;
        }

        let start = (rng.next_u64() as usize) % room_count;
        for offset in 0..room_count {
            let room_idx = 1 + (start + offset) % room_count;
            let room = self.rooms[room_idx].clone();
            if self.place_bridge_setup_in_room(&room, rng) {
                return true;
            }
        }

        false
    }

    pub(super) fn room_is_plain(&self, room: &Room) -> bool {
        for y in room.y..room.y + room.h {
            for x in room.x..room.x + room.w {
                if !self.in_bounds(x, y) {
                    return false;
                }
                if !matches!(self.tile(x, y), Tile::MetalFloor | Tile::Hallway) {
                    return false;
                }
            }
        }
        true
    }

    pub(super) fn try_place_puzzle_niche(
        &mut self,
        barrier_x: i32,
        barrier_y: i32,
        dx: i32,
        dy: i32,
        barrier_tile: Tile,
        with_crate: bool,
    ) -> bool {
        let reward_x = barrier_x + dx;
        let reward_y = barrier_y + dy;
        let back_x = reward_x + dx;
        let back_y = reward_y + dy;
        let approach_x = barrier_x - dx;
        let approach_y = barrier_y - dy;
        let stand_x = approach_x - dx;
        let stand_y = approach_y - dy;
        let side_a = (-dy, dx);
        let side_b = (dy, -dx);

        let mut floor_positions = vec![
            (barrier_x, barrier_y),
            (reward_x, reward_y),
            (back_x, back_y),
            (approach_x, approach_y),
            (barrier_x + side_a.0, barrier_y + side_a.1),
            (reward_x + side_a.0, reward_y + side_a.1),
            (back_x + side_a.0, back_y + side_a.1),
            (barrier_x + side_b.0, barrier_y + side_b.1),
            (reward_x + side_b.0, reward_y + side_b.1),
            (back_x + side_b.0, back_y + side_b.1),
        ];
        if with_crate {
            floor_positions.push((stand_x, stand_y));
        }

        if floor_positions
            .iter()
            .any(|&(x, y)| !self.in_bounds(x, y) || self.tile(x, y) != Tile::MetalFloor)
        {
            return false;
        }

        let wall_positions = [
            (back_x, back_y),
            (barrier_x + side_a.0, barrier_y + side_a.1),
            (reward_x + side_a.0, reward_y + side_a.1),
            (back_x + side_a.0, back_y + side_a.1),
            (barrier_x + side_b.0, barrier_y + side_b.1),
            (reward_x + side_b.0, reward_y + side_b.1),
            (back_x + side_b.0, back_y + side_b.1),
        ];

        for (x, y) in wall_positions {
            let idx = self.idx(x, y);
            self.tiles[idx] = Tile::Bulkhead;
        }

        let barrier_idx = self.idx(barrier_x, barrier_y);
        self.tiles[barrier_idx] = barrier_tile;

        let reward_idx = self.idx(reward_x, reward_y);
        self.tiles[reward_idx] = Tile::SupplyCrate;

        if with_crate {
            let crate_idx = self.idx(approach_x, approach_y);
            self.tiles[crate_idx] = Tile::SalvageCrate;
        }

        true
    }

    pub(super) fn try_place_brittle_vault(&mut self, room: &Room, rng: &mut Rng) -> bool {
        let mut candidates = Vec::new();
        if room.w >= 6 && room.h >= 5 {
            let y = rng.range(room.y + 2, room.y + room.h - 2);
            candidates.push((room.x + room.w - 4, y, 1, 0));
            candidates.push((room.x + 3, y, -1, 0));
        }
        if room.h >= 6 && room.w >= 5 {
            let x = rng.range(room.x + 2, room.x + room.w - 2);
            candidates.push((x, room.y + room.h - 4, 0, 1));
            candidates.push((x, room.y + 3, 0, -1));
        }

        if candidates.is_empty() {
            return false;
        }

        let start = (rng.next_u64() as usize) % candidates.len();
        for offset in 0..candidates.len() {
            let (x, y, dx, dy) = candidates[(start + offset) % candidates.len()];
            if self.try_place_puzzle_niche(x, y, dx, dy, Tile::WeakBulkhead, false) {
                return true;
            }
        }

        false
    }

    pub(super) fn try_place_deep_water_cache(&mut self, room: &Room, rng: &mut Rng) -> bool {
        let mut candidates = Vec::new();
        if room.w >= 7 && room.h >= 5 {
            let y = rng.range(room.y + 2, room.y + room.h - 2);
            candidates.push((room.x + room.w - 4, y, 1, 0));
            candidates.push((room.x + 3, y, -1, 0));
        }
        if room.h >= 7 && room.w >= 5 {
            let x = rng.range(room.x + 2, room.x + room.w - 2);
            candidates.push((x, room.y + room.h - 4, 0, 1));
            candidates.push((x, room.y + 3, 0, -1));
        }

        if candidates.is_empty() {
            return false;
        }

        let start = (rng.next_u64() as usize) % candidates.len();
        for offset in 0..candidates.len() {
            let (x, y, dx, dy) = candidates[(start + offset) % candidates.len()];
            if self.try_place_puzzle_niche(x, y, dx, dy, Tile::VacuumBreach, true) {
                return true;
            }
        }

        false
    }

    /// Laser bridge: 3-wide laser grid corridor with a supply crate on the far side.
    pub(super) fn try_place_spike_bridge(&mut self, room: &Room, rng: &mut Rng) -> bool {
        if room.w < 7 || room.h < 5 {
            return false;
        }
        let cy = rng.range(room.y + 1, room.y + room.h - 1);
        let sx = room.x + 2;
        for dx in 0..3 {
            let x = sx + dx;
            if !self.in_bounds(x, cy) || self.tile(x, cy) != Tile::MetalFloor {
                return false;
            }
        }
        let reward_x = sx + 3;
        if !self.in_bounds(reward_x, cy) || self.tile(reward_x, cy) != Tile::MetalFloor {
            return false;
        }
        for dx in 0..3 {
            let idx = self.idx(sx + dx, cy);
            self.tiles[idx] = Tile::LaserGrid;
        }
        let idx = self.idx(reward_x, cy);
        self.tiles[idx] = Tile::SupplyCrate;
        true
    }

    /// Coolant-fire trap: coolant slick leading to a supply crate, ignitable by energy weapons.
    pub(super) fn try_place_oil_fire_trap(&mut self, room: &Room, rng: &mut Rng) -> bool {
        if room.w < 6 || room.h < 5 {
            return false;
        }
        let cy = rng.range(room.y + 1, room.y + room.h - 1);
        let sx = room.x + 2;
        for dx in 0..3 {
            let x = sx + dx;
            if !self.in_bounds(x, cy) || self.tile(x, cy) != Tile::MetalFloor {
                return false;
            }
        }
        for dx in 0..2 {
            let idx = self.idx(sx + dx, cy);
            self.tiles[idx] = Tile::Coolant;
        }
        let idx = self.idx(sx + 2, cy);
        self.tiles[idx] = Tile::SupplyCrate;
        true
    }

    /// Seal chain: two seals placed near each other for cascading reshaping.
    pub(super) fn try_place_seal_chain(&mut self, room: &Room, rng: &mut Rng) -> bool {
        if room.w < 6 || room.h < 5 {
            return false;
        }
        let cx = rng.range(room.x + 1, room.x + room.w - 3);
        let cy = rng.range(room.y + 1, room.y + room.h - 1);
        if !self.in_bounds(cx, cy) || self.tile(cx, cy) != Tile::MetalFloor {
            return false;
        }
        if !self.in_bounds(cx + 2, cy) || self.tile(cx + 2, cy) != Tile::MetalFloor {
            return false;
        }
        let kind_a = SealKind::random(&mut Rng::new(rng.next_u64()));
        let kind_b = SealKind::random(&mut Rng::new(rng.next_u64()));
        let idx_a = self.idx(cx, cy);
        self.tiles[idx_a] = Tile::SecurityLock(kind_a);
        let idx_b = self.idx(cx + 2, cy);
        self.tiles[idx_b] = Tile::SecurityLock(kind_b);
        true
    }

    pub(super) fn place_puzzle_room_in_room(&mut self, room: &Room, rng: &mut Rng) -> bool {
        let variant_count = 5;
        let start = (rng.next_u64() % variant_count) as usize;
        for offset in 0..variant_count as usize {
            let placed = match (start + offset) % variant_count as usize {
                0 => self.try_place_brittle_vault(room, rng),
                1 => self.try_place_deep_water_cache(room, rng),
                2 => self.try_place_spike_bridge(room, rng),
                3 => self.try_place_oil_fire_trap(room, rng),
                _ => self.try_place_seal_chain(room, rng),
            };
            if placed {
                return true;
            }
        }

        false
    }
}
