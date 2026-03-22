//! BSP-based dungeon generation.
//!
//! Splits a rectangular area recursively, places rooms in leaves,
//! then connects sibling rooms with corridors.

use crate::player::Deity;
use crate::world::{Tile, AltarKind, SealKind, SpecialRoomKind, RoomModifier, SecuritySeal, TerminalKind, LocationType, Rng};

// ── Tile types ──────────────────────────────────────────────────────────────

/* Enum AltarKind removed */


#[derive(Clone)]
pub struct Room {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub modifier: Option<RoomModifier>,
    pub special: Option<SpecialRoomKind>,
}

impl Room {
    pub fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }

    #[allow(dead_code)]
    pub fn intersects(&self, other: &Room) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }
}

// ── BSP node ────────────────────────────────────────────────────────────────

struct BspNode {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    left: Option<Box<BspNode>>,
    right: Option<Box<BspNode>>,
    room: Option<Room>,
}

const MIN_LEAF: i32 = 7;
const MIN_ROOM: i32 = 4;

impl BspNode {
    fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            x,
            y,
            w,
            h,
            left: None,
            right: None,
            room: None,
        }
    }

    fn split(&mut self, rng: &mut Rng) -> bool {
        if self.left.is_some() {
            return false; // already split
        }
        // Decide split direction: prefer splitting the longer axis
        let split_h = if self.w > self.h && (self.w as f64 / self.h as f64) >= 1.25 {
            false // split vertically
        } else if self.h > self.w && (self.h as f64 / self.w as f64) >= 1.25 {
            true // split horizontally
        } else {
            rng.next_u64() % 2 == 0
        };

        let max = if split_h { self.h } else { self.w } - MIN_LEAF;
        if max < MIN_LEAF {
            return false; // too small
        }

        let split = rng.range(MIN_LEAF, max + 1);

        if split_h {
            self.left = Some(Box::new(BspNode::new(self.x, self.y, self.w, split)));
            self.right = Some(Box::new(BspNode::new(
                self.x,
                self.y + split,
                self.w,
                self.h - split,
            )));
        } else {
            self.left = Some(Box::new(BspNode::new(self.x, self.y, split, self.h)));
            self.right = Some(Box::new(BspNode::new(
                self.x + split,
                self.y,
                self.w - split,
                self.h,
            )));
        }
        true
    }

    fn create_rooms(&mut self, rng: &mut Rng) {
        if let (Some(ref mut l), Some(ref mut r)) = (&mut self.left, &mut self.right) {
            l.create_rooms(rng);
            r.create_rooms(rng);
        } else {
            // Leaf node — place a room
            let w = rng.range(MIN_ROOM, self.w - 1);
            let h = rng.range(MIN_ROOM, self.h - 1);
            let x = self.x + rng.range(1, self.w - w);
            let y = self.y + rng.range(1, self.h - h);
            self.room = Some(Room {
                x,
                y,
                w,
                h,
                modifier: None,
                special: None,
            });
        }
    }

    fn get_room(&self) -> Option<&Room> {
        if self.room.is_some() {
            return self.room.as_ref();
        }
        // Search children for any room (pick left-first)
        if let Some(ref l) = self.left {
            if let Some(r) = l.get_room() {
                return Some(r);
            }
        }
        if let Some(ref r) = self.right {
            return r.get_room();
        }
        None
    }

    fn collect_corridors(&self, corridors: &mut Vec<((i32, i32), (i32, i32))>) {
        if let (Some(ref l), Some(ref r)) = (&self.left, &self.right) {
            l.collect_corridors(corridors);
            r.collect_corridors(corridors);
            // Connect a room from each child
            if let (Some(lr), Some(rr)) = (l.get_room(), r.get_room()) {
                corridors.push((lr.center(), rr.center()));
            }
        }
    }
}

// ── DungeonLevel ────────────────────────────────────────────────────────────

pub struct DungeonLevel {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Tile>,
    pub rooms: Vec<Room>,
    pub visible: Vec<bool>,
    pub revealed: Vec<bool>,
}

impl DungeonLevel {
    pub fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.width + x) as usize
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }

    pub fn tile(&self, x: i32, y: i32) -> Tile {
        if self.in_bounds(x, y) {
            self.tiles[self.idx(x, y)]
        } else {
            Tile::Bulkhead
        }
    }

    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && self.tiles[self.idx(x, y)].is_walkable()
    }

    fn area_is_solid_wall(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        if x < 1 || y < 1 || x + w >= self.width - 1 || y + h >= self.height - 1 {
            return false;
        }

        for ty in y..y + h {
            for tx in x..x + w {
                if self.tile(tx, ty) != Tile::Bulkhead {
                    return false;
                }
            }
        }

        true
    }

    fn carve_rect(&mut self, x: i32, y: i32, w: i32, h: i32, tile: Tile) {
        for ty in y..y + h {
            for tx in x..x + w {
                let idx = self.idx(tx, ty);
                self.tiles[idx] = tile;
            }
        }
    }

    fn place_secret_room_feature(
        &mut self,
        room_x: i32,
        room_y: i32,
        room_w: i32,
        room_h: i32,
        rng: &mut Rng,
    ) {
        let cx = room_x + room_w / 2;
        let cy = room_y + room_h / 2;

        match rng.next_u64() % 4 {
            0 => {
                for &(dx, dy) in &[(0, 0), (-1, 0), (1, 0)] {
                    let tx = cx + dx;
                    let ty = cy + dy;
                    if self.in_bounds(tx, ty) && self.tile(tx, ty) == Tile::MetalFloor {
                        let idx = self.idx(tx, ty);
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
            }
            1 => {
                let idx = self.idx(cx, cy);
                self.tiles[idx] = Tile::Terminal(AltarKind::random(rng));
            }
            2 => {
                let idx = self.idx(cx, cy);
                self.tiles[idx] = Tile::CircuitShrine;
            }
            _ => {
                let idx = self.idx(cx, cy);
                self.tiles[idx] = Tile::QuantumForge;
            }
        }
    }

    fn try_place_secret_room_candidate(
        &mut self,
        secret_x: i32,
        secret_y: i32,
        secret_w: i32,
        secret_h: i32,
        door_x: i32,
        door_y: i32,
        rng: &mut Rng,
    ) -> bool {
        if secret_x < 1
            || secret_y < 1
            || secret_x + secret_w >= self.width - 1
            || secret_y + secret_h >= self.height - 1
        {
            return false;
        }

        if !self.area_is_solid_wall(secret_x - 1, secret_y - 1, secret_w + 2, secret_h + 2) {
            return false;
        }

        self.carve_rect(secret_x, secret_y, secret_w, secret_h, Tile::MetalFloor);
        let door_idx = self.idx(door_x, door_y);
        self.tiles[door_idx] = Tile::DamagedBulkhead;
        self.place_secret_room_feature(secret_x, secret_y, secret_w, secret_h, rng);
        true
    }

    fn place_secret_room(&mut self, rng: &mut Rng) {
        if self.rooms.len() < 4 {
            return;
        }

        for _ in 0..24 {
            let room_idx = 1 + (rng.next_u64() as usize % (self.rooms.len() - 2));
            let room = self.rooms[room_idx].clone();
            let start_dir = (rng.next_u64() % 4) as usize;

            for dir_offset in 0..4 {
                let secret_w = 4 + (rng.next_u64() % 3) as i32;
                let secret_h = 4 + (rng.next_u64() % 3) as i32;
                let max_x = (self.width - secret_w - 1).max(1);
                let max_y = (self.height - secret_h - 1).max(1);
                let dir = (start_dir + dir_offset) % 4;

                let candidate = match dir {
                    0 => {
                        let door_x = rng.range(room.x + 1, room.x + room.w - 1);
                        let door_y = room.y - 1;
                        let secret_x = (door_x - secret_w / 2).clamp(1, max_x);
                        let secret_y = door_y - secret_h;
                        if secret_y < 1 || door_x < secret_x || door_x >= secret_x + secret_w {
                            None
                        } else {
                            Some((secret_x, secret_y, door_x, door_y))
                        }
                    }
                    1 => {
                        let door_x = rng.range(room.x + 1, room.x + room.w - 1);
                        let door_y = room.y + room.h;
                        let secret_x = (door_x - secret_w / 2).clamp(1, max_x);
                        let secret_y = door_y + 1;
                        if secret_y + secret_h >= self.height - 1
                            || door_x < secret_x
                            || door_x >= secret_x + secret_w
                        {
                            None
                        } else {
                            Some((secret_x, secret_y, door_x, door_y))
                        }
                    }
                    2 => {
                        let door_x = room.x - 1;
                        let door_y = rng.range(room.y + 1, room.y + room.h - 1);
                        let secret_x = door_x - secret_w;
                        let secret_y = (door_y - secret_h / 2).clamp(1, max_y);
                        if secret_x < 1 || door_y < secret_y || door_y >= secret_y + secret_h {
                            None
                        } else {
                            Some((secret_x, secret_y, door_x, door_y))
                        }
                    }
                    _ => {
                        let door_x = room.x + room.w;
                        let door_y = rng.range(room.y + 1, room.y + room.h - 1);
                        let secret_x = door_x + 1;
                        let secret_y = (door_y - secret_h / 2).clamp(1, max_y);
                        if secret_x + secret_w >= self.width - 1
                            || door_y < secret_y
                            || door_y >= secret_y + secret_h
                        {
                            None
                        } else {
                            Some((secret_x, secret_y, door_x, door_y))
                        }
                    }
                };

                let Some((secret_x, secret_y, door_x, door_y)) = candidate else {
                    continue;
                };

                if self.try_place_secret_room_candidate(
                    secret_x, secret_y, secret_w, secret_h, door_x, door_y, rng,
                ) {
                    return;
                }
            }
        }

        for room_idx in 1..self.rooms.len() - 1 {
            let room = self.rooms[room_idx].clone();
            for &(secret_w, secret_h) in &[(4, 4), (5, 4), (4, 5), (5, 5)] {
                for door_x in room.x + 1..room.x + room.w - 1 {
                    let secret_x = door_x - secret_w / 2;
                    let top_y = room.y - 1 - secret_h;
                    if door_x >= secret_x
                        && door_x < secret_x + secret_w
                        && self.try_place_secret_room_candidate(
                            secret_x,
                            top_y,
                            secret_w,
                            secret_h,
                            door_x,
                            room.y - 1,
                            rng,
                        )
                    {
                        return;
                    }

                    let bottom_y = room.y + room.h + 1;
                    if door_x >= secret_x
                        && door_x < secret_x + secret_w
                        && self.try_place_secret_room_candidate(
                            secret_x,
                            bottom_y,
                            secret_w,
                            secret_h,
                            door_x,
                            room.y + room.h,
                            rng,
                        )
                    {
                        return;
                    }
                }

                for door_y in room.y + 1..room.y + room.h - 1 {
                    let secret_y = door_y - secret_h / 2;
                    let left_x = room.x - 1 - secret_w;
                    if door_y >= secret_y
                        && door_y < secret_y + secret_h
                        && self.try_place_secret_room_candidate(
                            left_x,
                            secret_y,
                            secret_w,
                            secret_h,
                            room.x - 1,
                            door_y,
                            rng,
                        )
                    {
                        return;
                    }

                    let right_x = room.x + room.w + 1;
                    if door_y >= secret_y
                        && door_y < secret_y + secret_h
                        && self.try_place_secret_room_candidate(
                            right_x,
                            secret_y,
                            secret_w,
                            secret_h,
                            room.x + room.w,
                            door_y,
                            rng,
                        )
                    {
                        return;
                    }
                }
            }
        }
    }

    fn place_bridge_setup_in_room(&mut self, room: &Room, rng: &mut Rng) -> bool {
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

    fn place_bridge_setup(&mut self, rng: &mut Rng) -> bool {
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

    fn room_is_plain(&self, room: &Room) -> bool {
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

    fn try_place_puzzle_niche(
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

    fn try_place_brittle_vault(&mut self, room: &Room, rng: &mut Rng) -> bool {
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

    fn try_place_deep_water_cache(&mut self, room: &Room, rng: &mut Rng) -> bool {
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
            if self.try_place_puzzle_niche(x, y, dx, dy, Tile::Coolant, true) {
                return true;
            }
        }

        false
    }

    /// Spike bridge: 3-wide spike corridor with a chest on the far side.
    fn try_place_spike_bridge(&mut self, room: &Room, rng: &mut Rng) -> bool {
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
            self.tiles[idx] = Tile::Trap(0);
        }
        let idx = self.idx(reward_x, cy);
        self.tiles[idx] = Tile::SupplyCrate;
        true
    }

    /// Oil-fire trap: oil slick leading to a chest, ignitable by fire spell.
    fn try_place_oil_fire_trap(&mut self, room: &Room, rng: &mut Rng) -> bool {
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
            self.tiles[idx] = Tile::Trap(1);
        }
        let idx = self.idx(sx + 2, cy);
        self.tiles[idx] = Tile::SupplyCrate;
        true
    }

    /// Seal chain: two seals placed near each other for cascading reshaping.
    fn try_place_seal_chain(&mut self, room: &Room, rng: &mut Rng) -> bool {
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

    fn place_puzzle_room_in_room(&mut self, room: &Room, rng: &mut Rng) -> bool {
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

    pub fn place_puzzle_rooms(&mut self, rng: &mut Rng) {
        if self.rooms.len() < 4 {
            return;
        }

        let room_count = self.rooms.len().saturating_sub(2);
        if room_count == 0 {
            return;
        }

        let desired = if room_count >= 4 && rng.next_u64() % 100 < 50 {
            2
        } else {
            1
        };
        let start = (rng.next_u64() as usize) % room_count;
        let mut placed = 0;
        for offset in 0..room_count {
            let room_idx = 1 + (start + offset) % room_count;
            let room = self.rooms[room_idx].clone();
            if !self.room_is_plain(&room) {
                continue;
            }
            if self.place_puzzle_room_in_room(&room, rng) {
                placed += 1;
                if placed >= desired {
                    break;
                }
            }
        }
    }

    /// Place stairs down in the last room.
    pub fn place_stairs(&mut self) {
        if let Some(room) = self.rooms.last() {
            let (cx, cy) = room.center();
            let idx = self.idx(cx, cy);
            self.tiles[idx] = Tile::Airlock;
        }
    }

    /// Place forge workbenches in 1-2 middle rooms.
    pub fn place_forges(&mut self, rng: &mut Rng) {
        if self.rooms.len() < 3 {
            return;
        }
        // Pick 1-2 rooms (not first or last)
        let candidates: Vec<usize> = (1..self.rooms.len() - 1).collect();
        let count = if candidates.len() >= 3 { 2 } else { 1 };
        let mut placed = 0;
        let mut used = Vec::new();
        while placed < count {
            let pick = rng.range(0, candidates.len() as i32) as usize;
            if used.contains(&pick) {
                // Avoid infinite loop if few candidates
                if used.len() >= candidates.len() {
                    break;
                }
                continue;
            }
            used.push(pick);
            let room = &self.rooms[candidates[pick]];
            // Place forge at an offset from center so it doesn't overlap stairs
            let fx = room.x + 1;
            let fy = room.y + 1;
            if self.in_bounds(fx, fy) {
                let idx = self.idx(fx, fy);
                if self.tiles[idx] == Tile::MetalFloor {
                    self.tiles[idx] = Tile::QuantumForge;
                    placed += 1;
                }
            }
        }
    }

    /// Place a shop in one middle room (if enough rooms).
    pub fn place_shop(&mut self, rng: &mut Rng) {
        if self.rooms.len() < 4 {
            return;
        }
        // Pick a room that isn't first, last, or already has a forge
        for _ in 0..10 {
            let pick = rng.range(1, self.rooms.len() as i32 - 1) as usize;
            let room = &self.rooms[pick];
            let fx = room.x + room.w - 2;
            let fy = room.y + 1;
            if self.in_bounds(fx, fy) {
                let idx = self.idx(fx, fy);
                if self.tiles[idx] == Tile::MetalFloor {
                    self.tiles[idx] = Tile::TradeTerminal;
                    return;
                }
            }
        }
    }

    /// Place treasure chests in one room (2-3 chests).
    pub fn place_chests(&mut self, rng: &mut Rng) {
        if self.rooms.len() < 5 {
            return;
        }
        // Pick a middle room that doesn't have forge/shop/stairs
        for _ in 0..20 {
            let pick = rng.range(1, self.rooms.len() as i32 - 1) as usize;
            let room = &self.rooms[pick];
            // Check room doesn't already have special tiles
            let has_special = (room.y..room.y + room.h).any(|ry| {
                (room.x..room.x + room.w).any(|rx| {
                    if rx >= 0 && ry >= 0 && rx < self.width && ry < self.height {
                        let idx = (ry * self.width + rx) as usize;
                        matches!(self.tiles[idx], Tile::QuantumForge | Tile::TradeTerminal | Tile::Airlock)
                    } else {
                        false
                    }
                })
            });
            if has_special {
                continue;
            }

            let chest_count = rng.range(2, 4); // 2-3 chests
            let mut placed = 0;
            for _ in 0..10 {
                let cx = rng.range(room.x + 1, room.x + room.w - 1);
                let cy = rng.range(room.y + 1, room.y + room.h - 1);
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                        placed += 1;
                        if placed >= chest_count {
                            break;
                        }
                    }
                }
            }
            if placed > 0 {
                return;
            }
        }
    }

    /// Place hazard tiles inspired by classic roguelikes.
    pub fn place_hazards(&mut self, rng: &mut Rng) {
        if self.rooms.len() < 4 {
            return;
        }

        let hazard_rooms = 1 + (rng.next_u64() % 2) as usize;
        let mut guaranteed_water = false;
        for _ in 0..hazard_rooms {
            for _ in 0..20 {
                let pick = rng.range(1, self.rooms.len() as i32 - 1) as usize;
                let room = &self.rooms[pick];
                let has_special = (room.y..room.y + room.h).any(|ry| {
                    (room.x..room.x + room.w).any(|rx| {
                        if rx >= 0 && ry >= 0 && rx < self.width && ry < self.height {
                            let idx = (ry * self.width + rx) as usize;
                            matches!(
                                self.tiles[idx],
                                Tile::QuantumForge
                                    | Tile::TradeTerminal
                                    | Tile::Airlock
                                    | Tile::SupplyCrate
                                    | Tile::Npc(_)
                                    | Tile::CircuitShrine
                                    | Tile::Terminal(_)
                                    | Tile::SecurityLock(_)
                                    | Tile::InfoPanel(_)
                            )
                        } else {
                            false
                        }
                    })
                });
                if has_special {
                    continue;
                }

                let hazard_count = rng.range(2, 5);
                let mut placed = 0;
                for _ in 0..16 {
                    let hx = rng.range(room.x + 1, room.x + room.w - 1);
                    let hy = rng.range(room.y + 1, room.y + room.h - 1);
                    if !self.in_bounds(hx, hy) {
                        continue;
                    }
                    let idx = self.idx(hx, hy);
                    if self.tiles[idx] != Tile::MetalFloor {
                        continue;
                    }
                    self.tiles[idx] = if !guaranteed_water {
                        guaranteed_water = true;
                        Tile::CoolantPool
                    } else {
                        match rng.next_u64() % 3 {
                            0 => Tile::Trap(0),
                            1 => Tile::Trap(1),
                            _ => Tile::CoolantPool,
                        }
                    };
                    placed += 1;
                    if placed >= hazard_count {
                        break;
                    }
                }
                if placed > 0 {
                    break;
                }
            }
        }
    }

    /// Place smashable crates with supplies or trap gas.
    pub fn place_crates(&mut self, rng: &mut Rng) {
        if self.rooms.len() < 4 {
            return;
        }

        let _ = self.place_bridge_setup(rng);
        let crate_rooms = 1 + (rng.next_u64() % 2) as usize;
        for _ in 0..crate_rooms {
            for _ in 0..20 {
                let pick = rng.range(1, self.rooms.len() as i32 - 1) as usize;
                let room = &self.rooms[pick];
                let has_special = (room.y..room.y + room.h).any(|ry| {
                    (room.x..room.x + room.w).any(|rx| {
                        if rx >= 0 && ry >= 0 && rx < self.width && ry < self.height {
                            let idx = (ry * self.width + rx) as usize;
                            matches!(
                                self.tiles[idx],
                                Tile::QuantumForge
                                    | Tile::TradeTerminal
                                    | Tile::Airlock
                                    | Tile::SupplyCrate
                                    | Tile::Npc(_)
                                    | Tile::CircuitShrine
                                    | Tile::Terminal(_)
                                    | Tile::SecurityLock(_)
                                    | Tile::InfoPanel(_)
                            )
                        } else {
                            false
                        }
                    })
                });
                if has_special {
                    continue;
                }

                let crate_count = rng.range(1, 3);
                let mut placed = 0;
                for _ in 0..12 {
                    let cx = rng.range(room.x + 1, room.x + room.w - 1);
                    let cy = rng.range(room.y + 1, room.y + room.h - 1);
                    if !self.in_bounds(cx, cy) {
                        continue;
                    }
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] != Tile::MetalFloor {
                        continue;
                    }
                    self.tiles[idx] = Tile::SalvageCrate;
                    placed += 1;
                    if placed >= crate_count {
                        break;
                    }
                }
                if placed > 0 {
                    break;
                }
            }
        }
    }

    /// Get player start position (center of first room).
    pub fn start_pos(&self) -> (i32, i32) {
        self.rooms.first().map(|r| r.center()).unwrap_or((1, 1))
    }

    /// Scripted tutorial floor used on the first run.
    pub fn tutorial(width: i32, height: i32) -> Self {
        debug_assert!(
            width >= 44 && height >= 30,
            "tutorial floor expects the default map size"
        );

        let size = (width * height) as usize;
        let mut level = DungeonLevel {
            width,
            height,
            tiles: vec![Tile::Bulkhead; size],
            rooms: vec![
                Room {
                    x: 4,
                    y: 18,
                    w: 9,
                    h: 9,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 16,
                    y: 18,
                    w: 9,
                    h: 9,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 29,
                    y: 18,
                    w: 13,
                    h: 9,
                    modifier: None,
                    special: None,
                },
            ],
            visible: vec![false; size],
            revealed: vec![false; size],
        };

        fn carve_room(level: &mut DungeonLevel, room: &Room) {
            for ry in room.y..room.y + room.h {
                for rx in room.x..room.x + room.w {
                    let idx = level.idx(rx, ry);
                    level.tiles[idx] = Tile::MetalFloor;
                }
            }
        }

        fn carve_h_corridor(level: &mut DungeonLevel, x1: i32, x2: i32, y: i32) {
            for x in x1.min(x2)..=x1.max(x2) {
                let idx = level.idx(x, y);
                if level.tiles[idx] == Tile::Bulkhead {
                    level.tiles[idx] = Tile::Hallway;
                }
            }
        }

        let rooms = level.rooms.clone();
        for room in &rooms {
            carve_room(&mut level, room);
        }

        carve_h_corridor(&mut level, 13, 15, 22);
        carve_h_corridor(&mut level, 25, 28, 22);

        let sign0 = level.idx(8, 20);
        let sign1 = level.idx(13, 22);
        let sign2 = level.idx(30, 20);
        let sign3 = level.idx(38, 20);
        let forge = level.idx(32, 22);
        let stairs = level.idx(39, 22);
        level.tiles[sign0] = Tile::InfoPanel(0);
        level.tiles[sign1] = Tile::InfoPanel(1);
        level.tiles[sign2] = Tile::InfoPanel(2);
        level.tiles[sign3] = Tile::InfoPanel(3);
        level.tiles[forge] = Tile::QuantumForge;
        level.tiles[stairs] = Tile::Airlock;

        level
    }

    pub fn generate(width: i32, height: i32, seed: u64, floor: i32) -> Self {
        let mut rng = Rng::new(seed);
        let size = (width * height) as usize;
        let mut tiles = vec![Tile::Bulkhead; size];
        let mut root = BspNode::new(0, 0, width, height);

        // Recursive split
        let mut leaves = vec![&mut root as *mut BspNode];
        let mut did_split = true;
        while did_split {
            did_split = false;
            let mut new_leaves = Vec::new();
            for ptr in &leaves {
                let node = unsafe { &mut **ptr };
                if node.left.is_none() && node.split(&mut rng) {
                    new_leaves.push(node.left.as_mut().unwrap().as_mut() as *mut BspNode);
                    new_leaves.push(node.right.as_mut().unwrap().as_mut() as *mut BspNode);
                    did_split = true;
                } else if node.left.is_none() {
                    new_leaves.push(*ptr);
                }
            }
            if did_split {
                leaves = new_leaves;
            }
        }

        // Create rooms in leaves
        root.create_rooms(&mut rng);

        // Collect rooms
        let mut rooms = Vec::new();
        fn collect_rooms(node: &BspNode, out: &mut Vec<Room>) {
            if let Some(ref r) = node.room {
                out.push(r.clone());
            }
            if let Some(ref l) = node.left {
                collect_rooms(l, out);
            }
            if let Some(ref r) = node.right {
                collect_rooms(r, out);
            }
        }
        collect_rooms(&root, &mut rooms);

        // Carve rooms
        for room in &rooms {
            for ry in room.y..room.y + room.h {
                for rx in room.x..room.x + room.w {
                    if rx >= 0 && ry >= 0 && rx < width && ry < height {
                        tiles[(ry * width + rx) as usize] = Tile::MetalFloor;
                    }
                }
            }
        }

        // Collect and carve corridors (L-shaped)
        let mut corridors = Vec::new();
        root.collect_corridors(&mut corridors);
        for ((x1, y1), (x2, y2)) in &corridors {
            // Horizontal then vertical
            let (x1, y1, x2, y2) = (*x1, *y1, *x2, *y2);
            let xmin = x1.min(x2);
            let xmax = x1.max(x2);
            for x in xmin..=xmax {
                if x >= 0 && y1 >= 0 && x < width && y1 < height {
                    let i = (y1 * width + x) as usize;
                    if tiles[i] == Tile::Bulkhead {
                        tiles[i] = Tile::Hallway;
                    }
                }
            }
            let ymin = y1.min(y2);
            let ymax = y1.max(y2);
            for y in ymin..=ymax {
                if x2 >= 0 && y >= 0 && x2 < width && y < height {
                    let i = (y * width + x2) as usize;
                    if tiles[i] == Tile::Bulkhead {
                        tiles[i] = Tile::Hallway;
                    }
                }
            }
        }

        let visible = vec![false; size];
        let revealed = vec![false; size];

        let mut level = DungeonLevel {
            width,
            height,
            tiles,
            rooms,
            visible,
            revealed,
        };
        level.place_special_rooms(&mut rng, floor);
        level.place_stairs();
        level.place_forges(&mut rng);
        level.place_shop(&mut rng);
        level.place_chests(&mut rng);
        level.assign_room_modifiers(&mut rng);
        level.place_npcs(&mut rng);
        level.place_shrines(&mut rng);
        level.place_stroke_shrines(&mut rng);
        level.place_tone_walls(&mut rng);
        level.place_compound_shrines(&mut rng);
        level.place_classifier_shrines(&mut rng);
        level.place_ink_wells(&mut rng);
        level.place_ancestor_shrines(&mut rng);
        level.place_translation_altars(&mut rng);
        level.place_radical_gardens(&mut rng);
        level.place_mirror_pools(&mut rng);
        level.place_stone_tutors(&mut rng);
        level.place_codex_shrines(&mut rng);
        level.place_word_bridges(&mut rng);
        level.place_locked_doors(&mut rng);
        level.place_cursed_floors(&mut rng);
        level.place_altars(&mut rng);
        level.place_seals(&mut rng);
        level.place_hazards(&mut rng);
        level.place_crates(&mut rng);
        level.place_secret_room(&mut rng);
        level.place_puzzle_rooms(&mut rng);

        // Place trap tiles on deeper floors
        if floor >= 2 {
            let trap_count = 2 + floor / 3;
            let mut placed = 0;
            for _ in 0..trap_count * 10 {
                let x = (rng.next_u64() % width as u64) as i32;
                let y = (rng.next_u64() % height as u64) as i32;
                let idx = (y * width + x) as usize;
                if idx < level.tiles.len() && level.tiles[idx] == Tile::MetalFloor {
                    let trap_type = (rng.next_u64() % 4) as u8;
                    level.tiles[idx] = Tile::Trap(trap_type);
                    placed += 1;
                    if placed >= trap_count {
                        break;
                    }
                }
            }
        }

        level
    }

    /// Assign 2-4 rooms per floor as special rooms with unique layouts.
    fn place_special_rooms(&mut self, rng: &mut Rng, floor: i32) {
        let n = self.rooms.len();
        if n <= 4 {
            return;
        }
        // 2-4 special rooms per floor, scaling with room count
        let desired = 2 + (rng.next_u64() % 3.min(((n - 2) / 3) as u64)) as usize;
        let desired = desired.min(n - 2);
        let mut used = Vec::new();
        let mut placed = 0;
        for _ in 0..desired * 8 {
            if placed >= desired {
                break;
            }
            let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
            if used.contains(&room_idx) {
                continue;
            }
            // Skip rooms that already have special content
            let room = &self.rooms[room_idx];
            if room.special.is_some() {
                continue;
            }
            let kind = SpecialRoomKind::random_for_floor(rng, floor);
            let room = self.rooms[room_idx].clone();
            self.generate_special_room(kind, &room, rng);
            self.rooms[room_idx].special = Some(kind);
            used.push(room_idx);
            placed += 1;
        }
    }

    /// Generate tile layout for a special room.
    fn generate_special_room(&mut self, kind: SpecialRoomKind, room: &Room, rng: &mut Rng) {
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2;
        match kind {
            SpecialRoomKind::PirateStash => {
                // Gold piles scattered around the vault
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 7 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CreditCache;
                            }
                        }
                    }
                }
                // Water moat around center treasure area
                for &(dx, dy) in &[(-2,-1),(-2,0),(-2,1),(2,-1),(2,0),(2,1),(-1,-2),(0,-2),(1,-2),(-1,2),(0,2),(1,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::CreditCache) {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Crate barriers flanking the vault
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1),(3,-1),(3,0),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::CreditCache) {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // 3 chests in center
                for dx in -1..=1 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
                // Locked door at room edge
                if self.in_bounds(room.x, cy) {
                    let idx = self.idx(room.x, cy);
                    if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Hallway) {
                        self.tiles[idx] = Tile::SealedHatch;
                    }
                }
            }
            SpecialRoomKind::OreDeposit => {
                // Dense gold ore on walls (higher coverage)
                for x in room.x..room.x + room.w {
                    for y in [room.y, room.y + room.h - 1] {
                        if self.in_bounds(x, y) && rng.next_u64() % 2 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::OreVein;
                            }
                        }
                    }
                }
                for y in room.y..room.y + room.h {
                    for x in [room.x, room.x + room.w - 1] {
                        if self.in_bounds(x, y) && rng.next_u64() % 2 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::OreVein;
                            }
                        }
                    }
                }
                // Boulder support pillars
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2),(0,-2),(0,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Gold piles near veins
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 8 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CreditCache;
                            }
                        }
                    }
                }
                // Crate (mine cart) at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SalvageCrate;
                    }
                }
            }
            SpecialRoomKind::MemorialShrine => {
                // Crystal candles along walls
                for y in room.y + 1..room.y + room.h - 1 {
                    if (y - room.y) % 3 == 1 {
                        for &x in &[room.x + 1, room.x + room.w - 2] {
                            if self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::MetalFloor {
                                    self.tiles[idx] = Tile::CrystalPanel;
                                }
                            }
                        }
                    }
                }
                // Bookshelf pew rows on both sides of center aisle
                for y in room.y + 1..room.y + room.h - 1 {
                    if y != cy && (y - room.y) % 2 == 0 {
                        for &x_off in &[2, 3] {
                            let x_left = room.x + x_off;
                            let x_right = room.x + room.w - 1 - x_off;
                            if self.in_bounds(x_left, y) {
                                let idx = self.idx(x_left, y);
                                if self.tiles[idx] == Tile::MetalFloor {
                                    self.tiles[idx] = Tile::DataRack;
                                }
                            }
                            if self.in_bounds(x_right, y) {
                                let idx = self.idx(x_right, y);
                                if self.tiles[idx] == Tile::MetalFloor {
                                    self.tiles[idx] = Tile::DataRack;
                                }
                            }
                        }
                    }
                }
                // Water basin behind altar
                for dx in -1..=1 {
                    let x = cx + dx;
                    let y = cy - 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Jade altar at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Terminal(TerminalKind::Quantum);
                }
            }
            SpecialRoomKind::HiddenCache => {
                // Oil (dust) and mushroom (cobwebs) in neglected corners
                for &(dx, dy) in &[(1,1),(1,-1),(-1,1),(-1,-1)] {
                    let x = cx + dx * 2;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                for &(dx, _dy) in &[(1,0),(-1,0),(2,0),(-2,0)] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Trap(1);
                        }
                    }
                }
                // Crate (old supplies)
                if self.in_bounds(cx + 1, cy - 1) {
                    let idx = self.idx(cx + 1, cy - 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SalvageCrate;
                    }
                }
                // Chest at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
                // Cracked wall entrance
                let wall_x = room.x;
                if self.in_bounds(wall_x, cy) {
                    let idx = self.idx(wall_x, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::DamagedBulkhead;
                    }
                }
                // More cracked walls for atmosphere
                if self.in_bounds(wall_x, cy - 1) {
                    let idx = self.idx(wall_x, cy - 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::DamagedBulkhead;
                    }
                }
            }
            SpecialRoomKind::OfferingTerminal => {
                // Water purification basin
                for dx in -1..=1 {
                    let x = cx + dx;
                    let y = cy + 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Crystal candles at diagonal positions
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Mushroom incense at far positions
                for &(dx, dy) in &[(-2,-1),(2,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Gold pile offerings at cardinal positions
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CreditCache;
                        }
                    }
                }
                // Central altar
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Terminal(TerminalKind::Commerce);
                }
            }
            SpecialRoomKind::HydroponicsGarden => {
                // Bamboo border along room edges
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) && x != cx {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CargoPipes;
                            }
                        }
                    }
                }
                for y in room.y + 2..room.y + room.h - 2 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) && y != cy {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CargoPipes;
                            }
                        }
                    }
                }
                // Water pond around center (2-tile radius)
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        if dx == 0 && dy == 0 { continue; }
                        if dx.abs() + dy.abs() > 3 { continue; }
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CoolantPool;
                            }
                        }
                    }
                }
                // Mushroom (lotus plants) near water
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Spirit spring at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::MedBayTile;
                }
            }
            SpecialRoomKind::PirateStash => {
                // Gold ore on walls
                for x in room.x..room.x + room.w {
                    for y in [room.y, room.y + room.h - 1] {
                        if self.in_bounds(x, y) && rng.next_u64() % 2 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::OreVein;
                            }
                        }
                    }
                }
                // Dense gold piles throughout
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 3 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CreditCache;
                            }
                        }
                    }
                }
                // Crystal gems scattered
                for &(dx, dy) in &[(-2,0),(2,0),(0,-2),(0,2),(-1,-1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Crate treasure boxes at corners
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::CreditCache) {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
            }
            SpecialRoomKind::XenoCrystalVault => {
                // Crystal walls along full inner perimeter
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CrystalPanel;
                            }
                        }
                    }
                }
                for y in room.y + 2..room.y + room.h - 2 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CrystalPanel;
                            }
                        }
                    }
                }
                // Gold piles near chest
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CreditCache;
                        }
                    }
                }
                // Jade altar behind chest
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    self.tiles[idx] = Tile::Terminal(TerminalKind::Quantum);
                }
                // Chest in center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
                // Locked door at entry
                if self.in_bounds(room.x, cy) {
                    let idx = self.idx(room.x, cy);
                    if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Hallway | Tile::CrystalPanel) {
                        self.tiles[idx] = Tile::SealedHatch;
                    }
                }
            }
            SpecialRoomKind::RelicChamber => {
                // Bookshelves (lore) on walls
                for y in room.y + 1..room.y + room.h - 1 {
                    if (y - room.y) % 2 == 1 {
                        for &x in &[room.x + 1, room.x + room.w - 2] {
                            if self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::MetalFloor {
                                    self.tiles[idx] = Tile::DataRack;
                                }
                            }
                        }
                    }
                }
                // Water purification pool at entry
                for dx in -1..=1 {
                    let x = cx + dx;
                    let y = cy + 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Crystal lighting flanks
                for &(dx, dy) in &[(-2, 0), (2, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Mirror altar backdrop
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    self.tiles[idx] = Tile::Terminal(TerminalKind::Holographic);
                }
                // Chest (relic) at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::MedBay => {
                // Bamboo accents at room edges
                for &(dx, dy) in &[(-3,-2),(-3,2),(3,-2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoPipes;
                        }
                    }
                }
                // Water ring (2-wide)
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        if dx == 0 && dy == 0 { continue; }
                        let dist = dx.abs().max(dy.abs());
                        if dist >= 1 && dist <= 2 {
                            let x = cx + dx;
                            let y = cy + dy;
                            if self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::MetalFloor {
                                    self.tiles[idx] = Tile::CoolantPool;
                                }
                            }
                        }
                    }
                }
                // Crystal accents at compass points
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Mushroom herbs near water
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::CoolantPool) {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Spirit spring at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::MedBayTile;
                }
            }
            SpecialRoomKind::ArenaChallenge => {
                // Oil (sand/blood) scattered in center area
                for y in room.y + 2..room.y + room.h - 2 {
                    for x in room.x + 2..room.x + room.w - 2 {
                        if self.in_bounds(x, y) && rng.next_u64() % 5 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(1);
                            }
                        }
                    }
                }
                // Spikes inner border
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Trap(1)) {
                                self.tiles[idx] = Tile::Trap(0);
                            }
                        }
                    }
                }
                for y in room.y + 1..room.y + room.h - 1 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Trap(1)) {
                                self.tiles[idx] = Tile::Trap(0);
                            }
                        }
                    }
                }
                // Boulder pillars at corners
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Trap(1)) {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Pressure plate arena markers
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx * 2;
                    let y = cy + dy * 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Trap(1)) {
                            self.tiles[idx] = Tile::PressureSensor;
                        }
                    }
                }
            }
            SpecialRoomKind::SecurityCheckpoint => {
                // Spikes (caltrops) near entrance
                for dx in [-2, -1, 1, 2] {
                    let x = cx + dx;
                    let y = cy - 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Trap(0);
                        }
                    }
                }
                // Crate barricades on sides
                for &(dx, dy) in &[(-2,-1),(-2,1),(2,-1),(2,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Boulder obstacles
                for &(dx, dy) in &[(-3, 0), (3, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Pressure plates alarm strip
                for x in room.x + 1..room.x + room.w - 1 {
                    if self.in_bounds(x, cy) && x != cx {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PressureSensor;
                        }
                    }
                }
            }
            SpecialRoomKind::SecurityZone => {
                // Oil slicks between spike rows
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x + y) % 3 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(1);
                            }
                        }
                    }
                }
                // Dense spike checkerboard
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x + y) % 2 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(0);
                            }
                        }
                    }
                }
                // Boulder safe spots for cover
                for &(dx, dy) in &[(0,-1),(0,1),(-2,0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::CargoCrate;
                    }
                }
                // Pressure plate triggers
                for &(dx, dy) in &[(-1,-2),(1,-2),(-1,2),(1,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::PressureSensor;
                    }
                }
                // Chest reward at far end
                let rx = room.x + room.w - 2;
                if self.in_bounds(rx, cy) {
                    let idx = self.idx(rx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::ToxicMaze => {
                // Gas walls forming corridors (structured pattern)
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let rel_x = x - room.x;
                            let rel_y = y - room.y;
                            let is_wall = (rel_x % 3 != 0) && (rel_y % 3 != 0);
                            if is_wall {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::MetalFloor {
                                    self.tiles[idx] = Tile::ToxicGas;
                                }
                            }
                        }
                    }
                }
                // Mushroom patches in gas corridors
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 8 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::ToxicGas {
                                self.tiles[idx] = Tile::ToxicFungus;
                            }
                        }
                    }
                }
                // Water safe zones at corners
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::ToxicGas {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Chest reward at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::GravityPuzzle => {
                // Pressure plates at puzzle positions
                let plate_positions = [
                    (cx - 1, cy - 1), (cx + 1, cy - 1),
                    (cx - 1, cy + 1), (cx + 1, cy + 1),
                ];
                let boulder_positions = [
                    (cx - 1, cy), (cx + 1, cy),
                    (cx, cy - 1), (cx, cy + 1),
                ];
                for &(px, py) in &plate_positions {
                    if self.in_bounds(px, py) {
                        let idx = self.idx(px, py);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PressureSensor;
                        }
                    }
                }
                for &(bx, by) in &boulder_positions {
                    if self.in_bounds(bx, by) {
                        let idx = self.idx(bx, by);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Crystal markers near plates
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Cracked wall hints
                for &x in &[room.x + 1, room.x + room.w - 2] {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DamagedBulkhead;
                        }
                    }
                }
                // Oil ground texture
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 6 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(1);
                            }
                        }
                    }
                }
                // Chest reward at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::HolographicRoom => {
                // Ice reflective floor sections
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x + y) % 3 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::FrozenDeck;
                            }
                        }
                    }
                }
                // Water accents at edges
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::FrozenDeck) {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Crystal walls forming diamond pattern
                for &(dx, dy) in &[(-2,0),(2,0),(0,-2),(0,2),(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::CrystalPanel;
                    }
                }
                // Mirror pool at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::HoloPool;
                }
            }
            SpecialRoomKind::EnergyNexus => {
                // Elemental tiles around each altar
                // Water around Gale (west)
                for &(dx, dy) in &[(-3,-1),(-3,1),(-1,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Lava around Iron (east)
                for &(dx, dy) in &[(3,-1),(3,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PlasmaVent;
                        }
                    }
                }
                // Ice around Mirror (north)
                for &(dx, dy) in &[(-1,-3),(1,-3),(-1,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::FrozenDeck;
                        }
                    }
                }
                // Crystal around Gold (south)
                for &(dx, dy) in &[(-1,3),(1,3),(1,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Bamboo around Jade (center)
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoPipes;
                        }
                    }
                }
                // 5 elemental altars in cross pattern
                let positions = [
                    (cx, cy, TerminalKind::Quantum),
                    (cx - 2, cy, TerminalKind::Stellar),
                    (cx + 2, cy, TerminalKind::Tactical),
                    (cx, cy - 2, TerminalKind::Holographic),
                    (cx, cy + 2, TerminalKind::Commerce),
                ];
                for &(x, y, kind) in &positions {
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::Terminal(kind);
                    }
                }
            }
            SpecialRoomKind::SpaceHulkRise => {
                // Grid of spikes (tombstones) in rows
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x - room.x) % 2 == 1 && (y - room.y) % 2 == 1 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(0);
                            }
                        }
                    }
                }
                // Oil (disturbed earth) between tombstones
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 5 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(1);
                            }
                        }
                    }
                }
                // Mushroom (dead flowers) near graves
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2),(0,0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // CursedFloor path through cemetery
                for x in room.x + 1..room.x + room.w - 1 {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CorruptedFloor;
                        }
                    }
                }
            }
            SpecialRoomKind::PirateAmbush => {
                // Oil (escape routes / slippery floor)
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 6 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(1);
                            }
                        }
                    }
                }
                // Crate hiding spots forming ambush positions
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2),(-3,0),(3,0),(0,-2),(0,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Trap(1)) {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Hidden traps near gold
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        let trap_type = (rng.next_u64() % 3) as u8;
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Trap(1)) {
                            self.tiles[idx] = Tile::Trap(trap_type);
                        }
                    }
                }
                // Gold bait in center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::CreditCache;
                }
            }
            SpecialRoomKind::DuelArena => {
                // Oil (sand pit) inside circle
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(1);
                            }
                        }
                    }
                }
                // Spikes outer boundary
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3),(-3,-1),(-3,1),(3,-1),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Trap(0);
                        }
                    }
                }
                // Boulder pillars at far corners
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Pressure plate ring
                for &(dx, dy) in &[
                    (-2,0),(2,0),(0,-2),(0,2),
                    (-1,-1),(1,-1),(-1,1),(1,1),
                ] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::PressureSensor;
                    }
                }
            }
            SpecialRoomKind::DataArchive => {
                // Rows of bookshelves with center aisle
                for y in room.y + 1..room.y + room.h - 1 {
                    if (y - room.y) % 2 == 0 {
                        for x in room.x + 1..room.x + room.w - 1 {
                            if x != cx && self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::MetalFloor {
                                    self.tiles[idx] = Tile::DataRack;
                                }
                            }
                        }
                    }
                }
                // Crystal lanterns at end of bookshelf rows
                for y in room.y + 1..room.y + room.h - 1 {
                    if (y - room.y) % 2 == 0 {
                        for &x in &[room.x + 1, room.x + room.w - 2] {
                            if self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::DataRack {
                                    self.tiles[idx] = Tile::CrystalPanel;
                                }
                            }
                        }
                    }
                }
                // Crate (reading desks) in aisle
                for y in room.y + 2..room.y + room.h - 2 {
                    if (y - room.y) % 4 == 1 && self.in_bounds(cx, y) {
                        let idx = self.idx(cx, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // InkWell at study positions
                for &(dx, dy) in &[(-1, 1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataWell;
                        }
                    }
                }
            }
            SpecialRoomKind::SignalHall => {
                // Crystal lighting overhead
                for &x in &[room.x + 1, room.x + room.w - 2] {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Crate storage at corners
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Writing stations: bookshelf + inkwell pairs
                for dx in [-2, 0, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                    if self.in_bounds(x, cy + 1) {
                        let idx = self.idx(x, cy + 1);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataWell;
                        }
                    }
                }
                // Additional bookshelf rows
                for dx in [-2, 0, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy - 1) {
                        let idx = self.idx(x, cy - 1);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
            }
            SpecialRoomKind::ResearcherStudy => {
                // Bookshelves on north and south walls
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::DataRack;
                            }
                        }
                    }
                }
                // Crystal lamp
                if self.in_bounds(cx + 1, cy) {
                    let idx = self.idx(cx + 1, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::CrystalPanel;
                    }
                }
                // InkWell at desk
                if self.in_bounds(cx - 1, cy) {
                    let idx = self.idx(cx - 1, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::DataWell;
                    }
                }
                // Crate (desk)
                if self.in_bounds(cx, cy + 1) {
                    let idx = self.idx(cx, cy + 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SalvageCrate;
                    }
                }
                // Codex shrine at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::CodexTerminal;
                }
            }
            SpecialRoomKind::SensorArray => {
                // Water border accents
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Bamboo curtains at edges
                for &(dx, dy) in &[(-3,-2),(-3,2),(3,-2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoPipes;
                        }
                    }
                }
                // Mushroom (incense) at corners
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Crystal ring around mirror pool
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1),(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::CrystalPanel;
                    }
                }
                // Mirror pool at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::HoloPool;
                }
            }
            SpecialRoomKind::InscriptionWall => {
                // Full bookshelf walls on left and right
                for y in room.y + 1..room.y + room.h - 1 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::DataRack;
                            }
                        }
                    }
                }
                // Crystal lighting
                for &(dx, dy) in &[(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // InkWell for practice
                if self.in_bounds(cx + 1, cy) {
                    let idx = self.idx(cx + 1, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::DataWell;
                    }
                }
                // Radical garden at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::RadicalLab;
                }
            }
            SpecialRoomKind::TrainingSimulator => {
                // Oil (training mat) in center area
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(1);
                            }
                        }
                    }
                }
                // Crate (weapon rack) at side
                for &(dx, dy) in &[(-3, -1), (-3, 0), (-3, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Boulder (heavy bag) obstacles
                for &(dx, dy) in &[(3, -1), (3, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Pressure plates (training dummies)
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::PressureSensor;
                    }
                }
            }
            SpecialRoomKind::ZenChamber => {
                // Bamboo border accents
                for &(dx, dy) in &[(-3,-2),(-3,2),(3,-2),(3,2),(-3,0),(3,0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoPipes;
                        }
                    }
                }
                // Mushroom (incense) at corners
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Water at cardinal positions
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Crystal candles at diagonal positions
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Spirit spring at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::MedBayTile;
                }
            }
            SpecialRoomKind::TranslationChallenge => {
                // Bookshelves (reference texts) behind altars
                for dx in -2..=2 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy - 1) {
                        let idx = self.idx(x, cy - 1);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
                // Crystal lighting
                for &(dx, dy) in &[(-2, 0), (2, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // InkWell at translation stations
                for dx in -1..=1 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy + 1) {
                        let idx = self.idx(x, cy + 1);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataWell;
                        }
                    }
                }
                // Translation altars in a row
                for dx in -1..=1 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        self.tiles[idx] = Tile::TranslationTerminal;
                    }
                }
            }
            SpecialRoomKind::AncientDatapad => {
                // Mushroom (cobwebs) in corners
                for &(dx, dy) in &[(-2,-1),(2,-1),(-2,1),(2,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Crystal lighting
                for &(dx, dy) in &[(-1, 0), (1, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Cracked wall (age damage)
                if self.in_bounds(room.x + 1, cy + 1) {
                    let idx = self.idx(room.x + 1, cy + 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::DamagedBulkhead;
                    }
                }
                // Bookshelf alcove (expanded)
                for dx in -1..=1 {
                    let x = cx + dx;
                    for &y in &[cy - 1, cy - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::DataRack;
                            }
                        }
                    }
                }
                // Chest (scroll) at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::WisdomCore => {
                // Bookshelf (wisdom texts) at walls
                for &(dx, dy) in &[(-3, -1), (-3, 0), (-3, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
                // Crystal markers around well
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Water ring around well
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1),(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Deep water well at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Coolant;
                }
                // Stone tutor nearby
                if self.in_bounds(cx + 2, cy) {
                    let idx = self.idx(cx + 2, cy);
                    self.tiles[idx] = Tile::DroidTutor;
                }
            }
            SpecialRoomKind::FloodedCompartment => {
                // Most floor tiles become water, some deep water
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = if rng.next_u64() % 5 == 0 {
                                    Tile::Coolant
                                } else {
                                    Tile::CoolantPool
                                };
                            }
                        }
                    }
                }
                // Safe path through center row
                for x in room.x..room.x + room.w {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if matches!(self.tiles[idx], Tile::CoolantPool | Tile::Coolant) {
                            self.tiles[idx] = Tile::MetalFloor;
                        }
                    }
                }
                // Boulder stepping stones
                for &(dx, dy) in &[(-2,-2),(0,-2),(2,-2),(-2,2),(0,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::CoolantPool | Tile::Coolant) {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Crystal cave formations
                for &(dx, dy) in &[(-3, 0), (3, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::CoolantPool | Tile::Coolant) {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Mushroom (algae) patches
                for _ in 0..3 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::CoolantPool {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
            }
            SpecialRoomKind::CryogenicBay => {
                // Ice tiles everywhere
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::FrozenDeck;
                            }
                        }
                    }
                }
                // Crystal pillars
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1), (0, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::FrozenDeck {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Boulder (frozen rocks)
                for &(dx, dy) in &[(-3, 0), (3, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::FrozenDeck {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Water (frozen puddle edges)
                for &(dx, dy) in &[(-1,-2),(1,-2),(-1,2),(1,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::FrozenDeck {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
            }
            SpecialRoomKind::PlasmaCrossing => {
                // Lava floor
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::PlasmaVent;
                            }
                        }
                    }
                }
                // Safe cross-shaped path
                for x in room.x..room.x + room.w {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::PlasmaVent {
                            self.tiles[idx] = Tile::MetalFloor;
                        }
                    }
                }
                for y in room.y..room.y + room.h {
                    if self.in_bounds(cx, y) {
                        let idx = self.idx(cx, y);
                        if self.tiles[idx] == Tile::PlasmaVent {
                            self.tiles[idx] = Tile::MetalFloor;
                        }
                    }
                }
                // Boulder (heat-resistant rocks) on safe paths
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Gold ore (volcanic deposits) on walls
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::PlasmaVent {
                            self.tiles[idx] = Tile::OreVein;
                        }
                    }
                }
                // Chest reward at far corner
                let rx = room.x + room.w - 2;
                let ry = room.y + 1;
                if self.in_bounds(rx, ry) {
                    let idx = self.idx(rx, ry);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::PipeForest => {
                // Dense bamboo with gaps for navigation
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 3 != 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CargoPipes;
                            }
                        }
                    }
                }
                // Ensure center is clear
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::CargoPipes {
                                self.tiles[idx] = Tile::MetalFloor;
                            }
                        }
                    }
                }
                // Ensure paths from edges to center
                for x in room.x..room.x + room.w {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::CargoPipes {
                            self.tiles[idx] = Tile::MetalFloor;
                        }
                    }
                }
                // Water stream along one axis
                for y in room.y + 2..room.y + room.h - 2 {
                    let x = cx - 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::CargoPipes {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Mushroom undergrowth
                for &(dx, dy) in &[(-1,-2),(1,-2),(-1,2),(1,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::CargoPipes {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Crystal (lanterns/fireflies)
                for &(dx, dy) in &[(0,-2),(0,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::CargoPipes) {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
            }
            SpecialRoomKind::VentTunnel => {
                // Oil (wind streaks) along center
                for x in room.x + 1..room.x + room.w - 1 {
                    if self.in_bounds(x, cy) && x % 2 == 0 {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Trap(1);
                        }
                    }
                }
                // Spike barriers on sides
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[cy - 1, cy + 1] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(0);
                            }
                        }
                    }
                }
                // Pressure plate wind triggers
                for &dx in &[-2, 0, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Trap(1)) {
                            self.tiles[idx] = Tile::PressureSensor;
                        }
                    }
                }
                // Crystal wind chimes at walls
                for &(dx, dy) in &[(-3,-1),(3,-1),(-3,1),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Trap(0)) {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
            }
            SpecialRoomKind::CrystalCave => {
                // Water (cave pools) scattered
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 8 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CoolantPool;
                            }
                        }
                    }
                }
                // Ice (cold spots)
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::FrozenDeck;
                        }
                    }
                }
                // Mushroom (cave growth)
                for _ in 0..3 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Crystal formations (denser, with pattern)
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 5 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CrystalPanel;
                            }
                        }
                    }
                }
            }
            SpecialRoomKind::FungalGrotto => {
                // Mushroom clusters scattered
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 4 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::ToxicFungus;
                            }
                        }
                    }
                }
                // Water puddles
                for _ in 0..4 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Poison gas spore clouds near mushroom clusters
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 10 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::ToxicGas;
                            }
                        }
                    }
                }
                // Crystal bioluminescence
                for &(dx, dy) in &[(-2,0),(2,0),(0,-2),(0,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::ToxicFungus) {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
            }
            SpecialRoomKind::CoolantRiver => {
                // River of water/deep water running through middle
                for x in room.x..room.x + room.w {
                    for dy in -1..=1 {
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = if dy == 0 {
                                    Tile::Coolant
                                } else {
                                    Tile::CoolantPool
                                };
                            }
                        }
                    }
                }
                // Boulder (river rocks) in shallows
                for &(dx, dy) in &[(-3, -1), (-1, 1), (2, -1), (3, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::CoolantPool | Tile::Coolant) {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Crystal cave formations on banks
                for &(dx, dy) in &[(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Mushroom (riverside growth)
                for &(dx, dy) in &[(-1, -2), (1, -2), (-1, 2), (1, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Bridge crossing at center
                for dy in -1..=1 {
                    if self.in_bounds(cx, cy + dy) {
                        let idx = self.idx(cx, cy + dy);
                        self.tiles[idx] = Tile::DataBridge;
                    }
                }
            }
            SpecialRoomKind::EchoingHull => {
                // Water (sound-amplifying pools)
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Boulder (echo-generating rock formations)
                for &(dx, dy) in &[(-1, -2), (1, -2), (-1, 2), (1, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Ice patches (frozen condensation)
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::FrozenDeck;
                        }
                    }
                }
                // Crystal echo points at compass positions
                for &(dx, dy) in &[(-3, 0), (3, 0), (0, -3), (0, 3)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::CrystalPanel;
                    }
                }
            }
            SpecialRoomKind::DarkSector => {
                // Oil (shadow puddles)
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 4 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(1);
                            }
                        }
                    }
                }
                // CursedFloor patches
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1),(0,0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Trap(1)) {
                            self.tiles[idx] = Tile::CorruptedFloor;
                        }
                    }
                }
                // Crystal (dim lights at far edges)
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Trap(1)) {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Hidden traps
                for _ in 0..5 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Trap(1)) {
                            let trap_type = (rng.next_u64() % 3) as u8;
                            self.tiles[idx] = Tile::Trap(trap_type);
                        }
                    }
                }
            }
            SpecialRoomKind::WanderingMerchant => {
                // Bookshelf (wares display) at walls
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1),(3,-1),(3,0),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
                // Gold pile (display items)
                for &(dx, dy) in &[(-1, 1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CreditCache;
                        }
                    }
                }
                // Crate inventory around shop
                for &(dx, dy) in &[(-1, -1), (1, -1), (-2, 0), (2, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Shop at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::TradeTerminal;
                }
            }
            SpecialRoomKind::HermitSage => {
                // Crystal lantern
                if self.in_bounds(cx + 2, cy) {
                    let idx = self.idx(cx + 2, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::CrystalPanel;
                    }
                }
                // Mushroom (herb garden)
                for &(dx, dy) in &[(1, 1), (2, 1), (1, -1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Water (tea/medicine)
                if self.in_bounds(cx - 1, cy + 1) {
                    let idx = self.idx(cx - 1, cy + 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::CoolantPool;
                    }
                }
                // Bookshelves (study)
                for dx in -1..=1 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy - 1) {
                        let idx = self.idx(x, cy - 1);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
                // Crate (storage)
                if self.in_bounds(cx - 2, cy) {
                    let idx = self.idx(cx - 2, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SalvageCrate;
                    }
                }
                // Hermit NPC
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Npc(1);
                }
            }
            SpecialRoomKind::DetentionCell => {
                // Spikes (chains on walls)
                for y in room.y + 1..room.y + room.h - 1 {
                    if self.in_bounds(room.x + room.w - 2, y) && y != cy {
                        let idx = self.idx(room.x + room.w - 2, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Trap(0);
                        }
                    }
                }
                // Water puddle
                if self.in_bounds(cx + 1, cy + 1) {
                    let idx = self.idx(cx + 1, cy + 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::CoolantPool;
                    }
                }
                // Crate (bed/bench)
                if self.in_bounds(cx - 1, cy + 1) {
                    let idx = self.idx(cx - 1, cy + 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SalvageCrate;
                    }
                }
                // Iron bars (brittle walls)
                for y in room.y + 1..room.y + room.h - 1 {
                    if y != cy && self.in_bounds(room.x, y) {
                        let idx = self.idx(room.x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::WeakBulkhead;
                        }
                    }
                }
                // NPC prisoner
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Npc(3);
                }
                // Locked door
                if self.in_bounds(room.x, cy) {
                    let idx = self.idx(room.x, cy);
                    if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Hallway) {
                        self.tiles[idx] = Tile::SealedHatch;
                    }
                }
            }
            SpecialRoomKind::MemorialShrine => {
                // Bamboo sacred grove at edges
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1),(3,-1),(3,0),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoPipes;
                        }
                    }
                }
                // Crystal votive candles
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Water blessing pool
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Shrine at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::CircuitShrine;
                }
            }
            SpecialRoomKind::CantinaBay => {
                // Bamboo decoration at corners
                for &(dx, dy) in &[(-3,-2),(-3,2),(3,-2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoPipes;
                        }
                    }
                }
                // Water ornamental feature
                for &(dx, dy) in &[(0, 2), (-1, 2), (1, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Crate (tea tables)
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Bookshelves (menu/scrolls) on sides
                for &dx in &[-2, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
                // Spirit spring (tea pot warmth)
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    self.tiles[idx] = Tile::MedBayTile;
                }
                // Tea house keeper NPC
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Npc(2);
                }
            }
            SpecialRoomKind::EngineerWorkshop => {
                // Gold ore (raw materials) at walls
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::OreVein;
                        }
                    }
                }
                // Lava heat source (expanded)
                for &(dx, dy) in &[(0, 1), (-1, 1), (1, 1), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PlasmaVent;
                        }
                    }
                }
                // Crate (tools) on sides
                for &(dx, dy) in &[(-2, 0), (2, 0), (2, -1), (-2, -1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Boulder (anvil)
                if self.in_bounds(cx + 1, cy) {
                    let idx = self.idx(cx + 1, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::CargoCrate;
                    }
                }
                // Forge at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::QuantumForge;
                }
            }
            SpecialRoomKind::ChemLab => {
                // Water (vials/solutions) scattered
                for &(dx, dy) in &[(0, -1), (1, -1), (-1, -1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Bookshelves (recipe books)
                for &(dx, dy) in &[(-2, -1), (-2, 0), (-2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
                // Mushroom reagents
                for &(dx, dy) in &[(2, -1), (2, 0), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Crystal (reagent containers)
                for &(dx, dy) in &[(-1, 1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Poison gas (fume hood exhaust)
                if self.in_bounds(cx, cy + 2) {
                    let idx = self.idx(cx, cy + 2);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::ToxicGas;
                    }
                }
                // Forge (cauldron) at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::QuantumForge;
                }
            }
            SpecialRoomKind::FortuneTeller => {
                // Bamboo curtains
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1),(3,-1),(3,0),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoPipes;
                        }
                    }
                }
                // Oil (mysterious aura)
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Trap(1);
                        }
                    }
                }
                // Mushroom (incense)
                for &(dx, dy) in &[(-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Crystal ball
                if self.in_bounds(cx, cy + 1) {
                    let idx = self.idx(cx, cy + 1);
                    self.tiles[idx] = Tile::CrystalPanel;
                }
                // Mirror pool (scrying)
                if self.in_bounds(cx - 1, cy) {
                    let idx = self.idx(cx - 1, cy);
                    self.tiles[idx] = Tile::HoloPool;
                }
                // Fortune teller NPC
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Npc(0);
                }
            }
            SpecialRoomKind::RefugeeBay => {
                // Oil (campfire residue)
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Trap(1);
                        }
                    }
                }
                // Water (well/water source)
                if self.in_bounds(cx + 2, cy + 1) {
                    let idx = self.idx(cx + 2, cy + 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::CoolantPool;
                    }
                }
                // Crate (supplies, expanded)
                for &(dx, dy) in &[(-1, 1), (0, -1), (2, -1), (-2, -1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Trap(1)) {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Spirit spring (campfire warmth)
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::MedBayTile;
                }
                // NPCs (refugees)
                for &(dx, dy) in &[(-2, 0), (1, -1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Trap(1)) {
                            let npc_type = (rng.next_u64() % 4) as u8;
                            self.tiles[idx] = Tile::Npc(npc_type);
                        }
                    }
                }
            }
            SpecialRoomKind::WarpGate => {
                // Gold pile offerings around approach
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3),(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CreditCache;
                        }
                    }
                }
                // Lava moat
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1),(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::CreditCache) {
                            self.tiles[idx] = Tile::PlasmaVent;
                        }
                    }
                }
                // Crystal power conduits
                for &(dx, dy) in &[(-2,-1),(2,-1),(-2,1),(2,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::CreditCache) {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Safe approach paths (clear lava for cardinal approaches)
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::MetalFloor;
                    }
                    let lx = cx + dx / 2;
                    let ly = cy + dy / 2;
                    if self.in_bounds(lx, ly) {
                        let idx = self.idx(lx, ly);
                        if self.tiles[idx] == Tile::PlasmaVent {
                            self.tiles[idx] = Tile::MetalFloor;
                        }
                    }
                }
                // Dragon gate portal at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::WarpGatePortal;
                }
            }
            SpecialRoomKind::GamblingDen => {
                // 3 urns (chests) in a row, player picks one
                for dx in -1..=1 {
                    let x = cx + dx * 2;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SupplyCrate;
                        }
                    }
                }
                // Gold decoration around the den
                for &(dx, dy) in &[(-3, 0), (3, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CreditCache;
                        }
                    }
                }
            }
            SpecialRoomKind::BloodTerminal => {
                // Central altar with blood pools (lava-styled)
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::Terminal(TerminalKind::Tactical);
                    }
                }
                for &(dx, dy) in &[(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Trap(1);
                        }
                    }
                }
            }
            SpecialRoomKind::CursedSalvage => {
                // Ominous chest surrounded by cursed floor
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CorruptedFloor;
                            }
                        }
                    }
                }
            }
            SpecialRoomKind::SoulForge => {
                // Ethereal forge with crystal pillars
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::QuantumForge;
                    }
                }
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Spirit spring fuel
                if self.in_bounds(cx, cy + 1) {
                    let idx = self.idx(cx, cy + 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::PlasmaVent;
                    }
                }
            }
            SpecialRoomKind::WishingReactor => {
                // Deep water well with water ring
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::Coolant;
                    }
                }
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Gold offerings around well
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CreditCache;
                        }
                    }
                }
            }
            SpecialRoomKind::CipherGate => {
                // 4 pressure plates in cardinal directions (rune positions)
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PressureSensor;
                        }
                    }
                }
                // Locked treasure in center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SealedHatch;
                    }
                }
            }
            SpecialRoomKind::HoloMaze => {
                // Grid of crystals (mirrors) with walkable paths
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x + y) % 3 == 0 && !(x == cx && y == cy) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CrystalPanel;
                            }
                        }
                    }
                }
                // Reward in center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
            }
            SpecialRoomKind::GravityPlate => {
                // 3 pressure plates in a triangle, 3 boulders nearby, ice floor
                let plates = [(cx - 2, cy), (cx + 2, cy), (cx, cy - 2)];
                let boulders = [(cx - 2, cy + 1), (cx + 2, cy + 1), (cx, cy + 1)];
                for &(px, py) in &plates {
                    if self.in_bounds(px, py) {
                        let idx = self.idx(px, py);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PressureSensor;
                        }
                    }
                }
                for &(bx, by) in &boulders {
                    if self.in_bounds(bx, by) {
                        let idx = self.idx(bx, by);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Ice floor for sliding
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::FrozenDeck;
                            }
                        }
                    }
                }
                // Locked chest reward
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::ToneFrequency => {
                // 4 ascending shrines in a line
                for i in 0..4 {
                    let x = cx - 1 + i;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CircuitShrine;
                        }
                    }
                }
                // Reward chest at the top
                if self.in_bounds(cx + 3, cy) {
                    let idx = self.idx(cx + 3, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
            }
            SpecialRoomKind::ElementalLock => {
                // Locked door with 5 elemental altars
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SealedHatch;
                    }
                }
                let altars = [
                    (cx - 2, cy - 1, TerminalKind::Quantum),
                    (cx + 2, cy - 1, TerminalKind::Stellar),
                    (cx - 2, cy + 1, TerminalKind::Tactical),
                    (cx + 2, cy + 1, TerminalKind::Commerce),
                    (cx, cy + 2, TerminalKind::Holographic),
                ];
                for &(ax, ay, akind) in &altars {
                    if self.in_bounds(ax, ay) {
                        let idx = self.idx(ax, ay);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Terminal(akind);
                        }
                    }
                }
                // Chest behind the locked door
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
            }
            SpecialRoomKind::SurvivalBay => {
                // Open arena with spikes border, central floor
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(0);
                            }
                        }
                    }
                }
                for y in room.y + 1..room.y + room.h - 1 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(0);
                            }
                        }
                    }
                }
            }
            SpecialRoomKind::SalvageRace => {
                // Open room, gold piles scattered (collected on step)
                for _ in 0..6 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CreditCache;
                        }
                    }
                }
            }
            SpecialRoomKind::DepressurizingChamber => {
                // Spikes on the outer ring, treasure chest in center
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(0);
                            }
                        }
                    }
                }
                for y in room.y + 2..room.y + room.h - 2 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(0);
                            }
                        }
                    }
                }
                // Treasure at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
            }
            SpecialRoomKind::NanoFlood => {
                // Ink (oil) flooding most of the room
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 3 != 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Trap(1);
                            }
                        }
                    }
                }
                // InkWell at center for bonus
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::DataWell;
                }
                // Chest as reward near edge
                let rx = room.x + room.w - 2;
                if self.in_bounds(rx, cy) {
                    let idx = self.idx(rx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::FormShrine => {
                // Central shrine with 4 elemental markers
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::CircuitShrine;
                    }
                }
                let markers = [
                    (cx - 2, cy, Tile::PlasmaVent),     // Flame
                    (cx + 2, cy, Tile::CargoCrate),   // Stone
                    (cx, cy - 2, Tile::CoolantPool),     // Mist
                    (cx, cy + 2, Tile::PressureSensor), // Tiger
                ];
                for &(mx, my, tile) in &markers {
                    if self.in_bounds(mx, my) {
                        let idx = self.idx(mx, my);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = tile;
                        }
                    }
                }
            }
            SpecialRoomKind::ClassTrial => {
                // Training dummies (pressure plates) with obstacle course
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PressureSensor;
                        }
                    }
                }
                // Reward altar at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::Terminal(TerminalKind::Tactical);
                    }
                }
            }
            SpecialRoomKind::RadicalReactor => {
                // Spirit spring fountain with water and crystals
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::MedBayTile;
                    }
                }
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CoolantPool;
                            }
                        }
                    }
                }
                for &(dx, dy) in &[(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
            }
            SpecialRoomKind::AncestorCrypt => {
                // Tomb with bookshelf walls and central chest (weapon)
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::Terminal(TerminalKind::Quantum);
                    }
                }
                for dx in -1..=1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        let x = cx + dx;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::DataRack;
                            }
                        }
                    }
                }
            }
            SpecialRoomKind::ProphecyRoom => {
                // Murals (bookshelves) on all walls, crystal ball in center
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::DataRack;
                            }
                        }
                    }
                }
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::HoloPool;
                    }
                }
            }
            SpecialRoomKind::SealedMemory => {
                // Meditation space with shrine and water
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::CircuitShrine;
                    }
                }
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Bookshelves as memory records
                for dx in -2..=2 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy - 2) {
                        let idx = self.idx(x, cy - 2);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
            }
            SpecialRoomKind::DemonSeal => {
                // Sealed demon NPC with lava barrier
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::Npc(3);
                    }
                }
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::PlasmaVent;
                            }
                        }
                    }
                }
                // Safe approach from south
                if self.in_bounds(cx, cy + 1) {
                    let idx = self.idx(cx, cy + 1);
                    self.tiles[idx] = Tile::MetalFloor;
                }
                if self.in_bounds(cx, cy + 2) {
                    let idx = self.idx(cx, cy + 2);
                    self.tiles[idx] = Tile::MetalFloor;
                }
            }
            SpecialRoomKind::PhoenixNest => {
                // Central spirit spring with lava nest ring
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::MedBayTile;
                    }
                }
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PlasmaVent;
                        }
                    }
                }
                // Chest with phoenix plume
                if self.in_bounds(cx + 2, cy) {
                    let idx = self.idx(cx + 2, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
                // Safe paths
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] != Tile::SupplyCrate {
                            self.tiles[idx] = Tile::MetalFloor;
                        }
                    }
                }
            }
            SpecialRoomKind::CalligraphyContest => {
                // Writing stations with ink wells
                for dx in [-2, 0, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataWell;
                        }
                    }
                }
                // NPC judge
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::Npc(0);
                    }
                }
            }
        }
    }

    /// Assign random modifiers to some rooms (not first or last).
    fn assign_room_modifiers(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 2 {
            return;
        }
        for i in 1..n - 1 {
            if rng.next_u64() % 100 < 30 {
                self.rooms[i].modifier = Some(match rng.next_u64() % 6 {
                    0 => RoomModifier::PoweredDown,
                    1 => RoomModifier::HighTech,
                    2 => RoomModifier::Irradiated,
                    3 => RoomModifier::Hydroponics,
                    4 => RoomModifier::Cryogenic,
                    _ => RoomModifier::OverheatedReactor,
                });
            }
        }
    }

    /// Place a companion NPC in a random middle room (~40% chance per floor).
    fn place_npcs(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 40 {
            return;
        }
        // Pick a random middle room (not first, last, or second-to-last)
        let room_idx = 1 + (rng.next_u64() as usize % (n - 3));
        let room = &self.rooms[room_idx];
        let npc_type = (rng.next_u64() % 4) as u8;
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2 + 1; // offset from center
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::Npc(npc_type);
        }
    }

    /// Place a tone shrine in a random middle room (~30% chance).
    fn place_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 30 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 1;
        let cy = room.y + room.h / 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::CircuitShrine;
        }
    }

    fn place_stroke_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 20 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 + 1;
        let cy = room.y + room.h / 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::RadicalLab;
        }
    }

    fn place_tone_walls(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 20 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2 + 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::FrequencyWall;
        }
    }

    fn place_compound_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 20 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 1;
        let cy = room.y + room.h / 2 + 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::CompoundShrine;
        }
    }

    fn place_classifier_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 20 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 + 1;
        let cy = room.y + room.h / 2 + 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::ClassifierNode;
        }
    }

    fn place_ink_wells(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 1;
        let cy = room.y + room.h / 2 - 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::DataWell;
        }
    }

    fn place_ancestor_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 + 1;
        let cy = room.y + room.h / 2 - 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::MemorialNode;
        }
    }

    fn place_translation_altars(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2 - 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::TranslationTerminal;
        }
    }

    fn place_radical_gardens(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 2;
        let cy = room.y + room.h / 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::RadicalLab;
        }
    }

    fn place_mirror_pools(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 15 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 + 2;
        let cy = room.y + room.h / 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::HoloPool;
        }
    }

    fn place_stone_tutors(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2 + 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::DroidTutor;
        }
    }

    fn place_codex_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 15 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 1;
        let cy = room.y + room.h / 2 + 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::CodexTerminal;
        }
    }

    fn place_word_bridges(&mut self, rng: &mut Rng) {
        for i in 0..self.tiles.len() {
            if self.tiles[i] == Tile::Coolant {
                let x = (i % self.width as usize) as i32;
                let y = (i / self.width as usize) as i32;
                for &(dx, dy) in &[(0, -1), (0, 1), (-1, 0), (1, 0)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if self.in_bounds(nx, ny) {
                        let ni = self.idx(nx, ny);
                        if self.tiles[ni] == Tile::MetalFloor && rng.next_u64() % 100 < 8 {
                            self.tiles[ni] = Tile::DataBridge;
                            break;
                        }
                    }
                }
            }
        }
    }

    fn place_locked_doors(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 4 {
            return;
        }
        if rng.next_u64() % 100 >= 12 {
            return;
        }
        let room_idx = 2 + (rng.next_u64() as usize % (n - 3));
        let room = &self.rooms[room_idx];
        let doorways = [
            (room.x + room.w / 2, room.y),
            (room.x + room.w / 2, room.y + room.h - 1),
            (room.x, room.y + room.h / 2),
            (room.x + room.w - 1, room.y + room.h / 2),
        ];
        for (dx, dy) in doorways {
            if self.in_bounds(dx, dy) {
                let di = self.idx(dx, dy);
                if self.tiles[di] == Tile::MetalFloor || self.tiles[di] == Tile::Hallway {
                    self.tiles[di] = Tile::SealedHatch;
                    return;
                }
            }
        }
    }

    fn place_cursed_floors(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        let count = 1 + (rng.next_u64() as usize % 3);
        for _ in 0..count {
            let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
            let room = &self.rooms[room_idx];
            let rx = room.x + 1 + (rng.next_u64() as i32 % (room.w - 2).max(1));
            let ry = room.y + 1 + (rng.next_u64() as i32 % (room.h - 2).max(1));
            if self.in_bounds(rx, ry) {
                let ri = self.idx(rx, ry);
                if self.tiles[ri] == Tile::MetalFloor {
                    self.tiles[ri] = Tile::CorruptedFloor;
                }
            }
        }
    }

    /// Place a blessing altar in a quiet side room (~35% chance).
    fn place_altars(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 35 {
            return;
        }
        for _ in 0..12 {
            let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
            let room = &self.rooms[room_idx];
            let has_special = (room.y..room.y + room.h).any(|ry| {
                (room.x..room.x + room.w).any(|rx| {
                    if rx >= 0 && ry >= 0 && rx < self.width && ry < self.height {
                        let idx = (ry * self.width + rx) as usize;
                        matches!(
                            self.tiles[idx],
                            Tile::QuantumForge
                                | Tile::TradeTerminal
                                | Tile::Airlock
                                | Tile::SupplyCrate
                                | Tile::Npc(_)
                                | Tile::CircuitShrine
                                | Tile::Terminal(_)
                                | Tile::SecurityLock(_)
                                | Tile::InfoPanel(_)
                        )
                    } else {
                        false
                    }
                })
            });
            if has_special {
                continue;
            }

            let ax = room.x + room.w / 2;
            let ay = room.y + room.h / 2;
            if !self.in_bounds(ax, ay) {
                continue;
            }
            let idx = self.idx(ax, ay);
            if self.tiles[idx] == Tile::MetalFloor {
                self.tiles[idx] = Tile::Terminal(AltarKind::random(rng));
                return;
            }
        }
    }

    /// Place 1-2 script seals that reshape rooms when stepped on.
    fn place_seals(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }

        let seal_count = 1 + (rng.next_u64() % 2) as usize;
        let mut used_rooms = Vec::new();
        let mut placed = 0;
        for _ in 0..20 {
            if placed >= seal_count {
                break;
            }

            let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
            if used_rooms.contains(&room_idx) {
                continue;
            }
            let room = &self.rooms[room_idx];
            let has_special = (room.y..room.y + room.h).any(|ry| {
                (room.x..room.x + room.w).any(|rx| {
                    if rx >= 0 && ry >= 0 && rx < self.width && ry < self.height {
                        let idx = (ry * self.width + rx) as usize;
                        matches!(
                            self.tiles[idx],
                            Tile::QuantumForge
                                | Tile::TradeTerminal
                                | Tile::Airlock
                                | Tile::SupplyCrate
                                | Tile::Npc(_)
                                | Tile::CircuitShrine
                                | Tile::Terminal(_)
                                | Tile::SecurityLock(_)
                                | Tile::InfoPanel(_)
                        )
                    } else {
                        false
                    }
                })
            });
            if has_special {
                continue;
            }

            let sx = room.x + room.w / 2;
            let sy = room.y + room.h / 2;
            if !self.in_bounds(sx, sy) {
                continue;
            }
            let idx = self.idx(sx, sy);
            if self.tiles[idx] != Tile::MetalFloor {
                continue;
            }

            self.tiles[idx] = Tile::SecurityLock(SealKind::random(rng));
            used_rooms.push(room_idx);
            placed += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AltarKind, DungeonLevel, Rng, Room, SealKind, Tile};

    fn make_clean_test_level() -> DungeonLevel {
        let width = 24;
        let height = 24;
        let mut level = DungeonLevel {
            width,
            height,
            tiles: vec![Tile::Bulkhead; (width * height) as usize],
            rooms: vec![
                Room {
                    x: 1,
                    y: 1,
                    w: 5,
                    h: 5,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 8,
                    y: 1,
                    w: 5,
                    h: 5,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 15,
                    y: 1,
                    w: 5,
                    h: 5,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 1,
                    y: 10,
                    w: 5,
                    h: 5,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 8,
                    y: 10,
                    w: 5,
                    h: 5,
                    modifier: None,
                    special: None,
                },
            ],
            visible: vec![false; (width * height) as usize],
            revealed: vec![false; (width * height) as usize],
        };
        for room in level.rooms.clone() {
            for y in room.y..room.y + room.h {
                for x in room.x..room.x + room.w {
                    let idx = level.idx(x, y);
                    level.tiles[idx] = Tile::MetalFloor;
                }
            }
        }
        level
    }

    fn make_spacious_test_level() -> DungeonLevel {
        let width = 40;
        let height = 28;
        let mut level = DungeonLevel {
            width,
            height,
            tiles: vec![Tile::Bulkhead; (width * height) as usize],
            rooms: vec![
                Room {
                    x: 1,
                    y: 1,
                    w: 8,
                    h: 8,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 11,
                    y: 1,
                    w: 8,
                    h: 8,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 21,
                    y: 1,
                    w: 8,
                    h: 8,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 1,
                    y: 12,
                    w: 8,
                    h: 8,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 11,
                    y: 12,
                    w: 8,
                    h: 8,
                    modifier: None,
                    special: None,
                },
            ],
            visible: vec![false; (width * height) as usize],
            revealed: vec![false; (width * height) as usize],
        };
        for room in level.rooms.clone() {
            for y in room.y..room.y + room.h {
                for x in room.x..room.x + room.w {
                    let idx = level.idx(x, y);
                    level.tiles[idx] = Tile::MetalFloor;
                }
            }
        }
        level
    }

    fn has_pushable_bridge_setup(level: &DungeonLevel) -> bool {
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for y in 0..level.height {
            for x in 0..level.width {
                if level.tile(x, y) != Tile::CoolantPool {
                    continue;
                }

                for (dx, dy) in dirs {
                    let crate_x = x - dx;
                    let crate_y = y - dy;
                    let stand_x = crate_x - dx;
                    let stand_y = crate_y - dy;
                    if !level.in_bounds(crate_x, crate_y) || !level.in_bounds(stand_x, stand_y) {
                        continue;
                    }

                    if level.tile(crate_x, crate_y) == Tile::SalvageCrate {
                        let stand_tile = level.tile(stand_x, stand_y);
                        if stand_tile.is_walkable() && stand_tile != Tile::CoolantPool {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn has_brittle_vault(level: &DungeonLevel) -> bool {
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for y in 0..level.height {
            for x in 0..level.width {
                if level.tile(x, y) != Tile::WeakBulkhead {
                    continue;
                }

                for (dx, dy) in dirs {
                    let chest_x = x + dx;
                    let chest_y = y + dy;
                    let back_x = chest_x + dx;
                    let back_y = chest_y + dy;
                    let side_a = (-dy, dx);
                    let side_b = (dy, -dx);
                    if !level.in_bounds(chest_x, chest_y) || !level.in_bounds(back_x, back_y) {
                        continue;
                    }

                    if level.tile(chest_x, chest_y) == Tile::SupplyCrate
                        && level.tile(back_x, back_y) == Tile::Bulkhead
                        && level.tile(x + side_a.0, y + side_a.1) == Tile::Bulkhead
                        && level.tile(chest_x + side_a.0, chest_y + side_a.1) == Tile::Bulkhead
                        && level.tile(x + side_b.0, y + side_b.1) == Tile::Bulkhead
                        && level.tile(chest_x + side_b.0, chest_y + side_b.1) == Tile::Bulkhead
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn has_deep_water_cache(level: &DungeonLevel) -> bool {
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for y in 0..level.height {
            for x in 0..level.width {
                if level.tile(x, y) != Tile::Coolant {
                    continue;
                }

                for (dx, dy) in dirs {
                    let crate_x = x - dx;
                    let crate_y = y - dy;
                    let stand_x = crate_x - dx;
                    let stand_y = crate_y - dy;
                    let chest_x = x + dx;
                    let chest_y = y + dy;
                    let back_x = chest_x + dx;
                    let back_y = chest_y + dy;
                    let side_a = (-dy, dx);
                    let side_b = (dy, -dx);
                    if !level.in_bounds(crate_x, crate_y)
                        || !level.in_bounds(stand_x, stand_y)
                        || !level.in_bounds(chest_x, chest_y)
                        || !level.in_bounds(back_x, back_y)
                    {
                        continue;
                    }

                    if level.tile(crate_x, crate_y) == Tile::SalvageCrate
                        && level.tile(stand_x, stand_y) == Tile::MetalFloor
                        && level.tile(chest_x, chest_y) == Tile::SupplyCrate
                        && level.tile(back_x, back_y) == Tile::Bulkhead
                        && level.tile(x + side_a.0, y + side_a.1) == Tile::Bulkhead
                        && level.tile(chest_x + side_a.0, chest_y + side_a.1) == Tile::Bulkhead
                        && level.tile(x + side_b.0, y + side_b.1) == Tile::Bulkhead
                        && level.tile(chest_x + side_b.0, chest_y + side_b.1) == Tile::Bulkhead
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn has_spike_bridge(level: &DungeonLevel) -> bool {
        for y in 0..level.height {
            for x in 0..level.width - 3 {
                if level.tile(x, y) == Tile::Trap(0)
                    && level.tile(x + 1, y) == Tile::Trap(0)
                    && level.tile(x + 2, y) == Tile::Trap(0)
                    && level.tile(x + 3, y) == Tile::SupplyCrate
                {
                    return true;
                }
            }
        }
        false
    }

    fn has_oil_fire_trap(level: &DungeonLevel) -> bool {
        for y in 0..level.height {
            for x in 0..level.width - 2 {
                if level.tile(x, y) == Tile::Trap(1)
                    && level.tile(x + 1, y) == Tile::Trap(1)
                    && level.tile(x + 2, y) == Tile::SupplyCrate
                {
                    return true;
                }
            }
        }
        false
    }

    fn has_seal_chain(level: &DungeonLevel) -> bool {
        for y in 0..level.height {
            for x in 0..level.width - 2 {
                if matches!(level.tile(x, y), Tile::SecurityLock(_))
                    && matches!(level.tile(x + 2, y), Tile::SecurityLock(_))
                {
                    return true;
                }
            }
        }
        false
    }

    fn has_any_puzzle_room(level: &DungeonLevel) -> bool {
        has_brittle_vault(level)
            || has_deep_water_cache(level)
            || has_spike_bridge(level)
            || has_oil_fire_trap(level)
            || has_seal_chain(level)
    }

    #[test]
    fn hazards_and_altars_are_walkable_but_crates_block() {
        assert!(Tile::Trap(0).is_walkable());
        assert!(Tile::Trap(1).is_walkable());
        assert!(Tile::CoolantPool.is_walkable());
        assert!(Tile::Terminal(TerminalKind::Quantum).is_walkable());
        assert!(Tile::SecurityLock(SealKind::Ember).is_walkable());
        assert!(!Tile::SalvageCrate.is_walkable());
        assert!(!Tile::DamagedBulkhead.is_walkable());
        assert!(!Tile::WeakBulkhead.is_walkable());
        assert!(!Tile::Coolant.is_walkable());
    }

    #[test]
    fn place_altars_adds_a_blessing_site_to_clean_levels() {
        let mut level = make_clean_test_level();
        let mut rng = Rng::new(2);

        level.place_altars(&mut rng);

        assert!(level
            .tiles
            .iter()
            .any(|tile| matches!(tile, Tile::Terminal(_))));
    }

    #[test]
    fn place_seals_adds_script_seals_to_clean_levels() {
        let mut level = make_clean_test_level();
        let mut rng = Rng::new(7);

        level.place_seals(&mut rng);

        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::SecurityLock(_))));
    }

    #[test]
    fn place_secret_room_carves_hidden_chamber_with_cracked_entrance() {
        let mut level = make_clean_test_level();
        let mut rng = Rng::new(11);
        let original_open_tiles = level
            .tiles
            .iter()
            .filter(|tile| !matches!(tile, Tile::Bulkhead))
            .count();

        level.place_secret_room(&mut rng);

        let new_open_tiles = level
            .tiles
            .iter()
            .filter(|tile| !matches!(tile, Tile::Bulkhead))
            .count();
        assert!(level
            .tiles
            .iter()
            .any(|tile| matches!(tile, Tile::DamagedBulkhead)));
        assert!(new_open_tiles > original_open_tiles);
    }

    #[test]
    fn place_secret_room_adds_hidden_point_of_interest() {
        let mut level = make_clean_test_level();
        let mut rng = Rng::new(11);

        level.place_secret_room(&mut rng);

        assert!(level.tiles.iter().any(|tile| {
            matches!(
                tile,
                Tile::SupplyCrate | Tile::QuantumForge | Tile::CircuitShrine | Tile::Terminal(_)
            )
        }));
    }

    #[test]
    fn generated_levels_hide_secret_rooms_on_most_runs() {
        let mut secret_count = 0;
        for seed in 1..=24 {
            let level = DungeonLevel::generate(48, 48, seed, 1);
            if level
                .tiles
                .iter()
                .any(|tile| matches!(tile, Tile::DamagedBulkhead))
            {
                secret_count += 1;
            }
        }

        assert!(
            secret_count >= 18,
            "expected secret-room entrances on most sample floors, found {secret_count}"
        );
    }

    #[test]
    fn generated_levels_regularly_offer_bridge_building_setups() {
        let mut bridge_count = 0;
        for seed in 1..=24 {
            let level = DungeonLevel::generate(48, 48, seed, 1);
            if has_pushable_bridge_setup(&level) {
                bridge_count += 1;
            }
        }

        assert!(
            bridge_count >= 10,
            "expected bridge setups across the sample set, found {bridge_count}"
        );
    }

    #[test]
    fn place_puzzle_rooms_adds_visible_environmental_niches() {
        let mut level = make_spacious_test_level();
        let mut rng = Rng::new(19);

        level.place_puzzle_rooms(&mut rng);

        assert!(has_any_puzzle_room(&level));
    }

    #[test]
    fn generated_levels_regularly_offer_puzzle_rooms() {
        let mut puzzle_count = 0;
        let mut brittle_count = 0;
        let mut deep_water_count = 0;
        let mut spike_count = 0;
        let mut oil_count = 0;
        let mut seal_count = 0;
        for seed in 1..=24 {
            let level = DungeonLevel::generate(48, 48, seed, 1);
            if has_any_puzzle_room(&level) {
                puzzle_count += 1;
            }
            if has_brittle_vault(&level) {
                brittle_count += 1;
            }
            if has_deep_water_cache(&level) {
                deep_water_count += 1;
            }
            if has_spike_bridge(&level) {
                spike_count += 1;
            }
            if has_oil_fire_trap(&level) {
                oil_count += 1;
            }
            if has_seal_chain(&level) {
                seal_count += 1;
            }
        }

        assert!(
            puzzle_count >= 16,
            "expected puzzle rooms on most sample floors, found {puzzle_count}"
        );
        let variant_types = [
            brittle_count > 0,
            deep_water_count > 0,
            spike_count > 0,
            oil_count > 0,
            seal_count > 0,
        ];
        let variants_seen = variant_types.iter().filter(|&&v| v).count();
        assert!(
            variants_seen >= 2,
            "expected at least 2 puzzle room variants across 24 seeds, saw {variants_seen}"
        );
    }

    #[test]
    fn tutorial_floor_has_required_landmarks() {
        let level = DungeonLevel::tutorial(48, 48);

        assert_eq!(level.start_pos(), (8, 22));
        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::InfoPanel(0))));
        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::InfoPanel(1))));
        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::InfoPanel(2))));
        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::InfoPanel(3))));
        assert!(level.tiles.iter().any(|tile| *tile == Tile::QuantumForge));
        assert!(level.tiles.iter().any(|tile| *tile == Tile::Airlock));
        assert!(level.is_walkable(8, 20));
    }
}







