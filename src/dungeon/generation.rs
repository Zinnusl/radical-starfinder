//! BSP-based dungeon generation.
//!
//! Splits a rectangular area recursively, places rooms in leaves,
//! then connects sibling rooms with corridors.

use crate::player::Deity;

/// Simple PRNG (xorshift64) so we don't need an external crate.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 { 1 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Random i32 in [lo, hi) (hi > lo).
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo) as u64) as i32
    }
}

// ── Tile types ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum AltarKind {
    Jade,
    Gale,
    Mirror,
    Iron,
    Gold,
}

impl AltarKind {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Jade => "☯",   // Yin-Yang (Balance/Life)
            Self::Gale => "✦",   // Sparkle/Wind
            Self::Mirror => "◈", // Diamond/Reflection
            Self::Iron => "⚔",   // Crossed Swords (War)
            Self::Gold => "¥",   // Yen/Yuan (Wealth)
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Self::Jade => "#66dd99",   // Green
            Self::Gale => "#88ccff",   // Sky Blue
            Self::Mirror => "#ddb8ff", // Purple
            Self::Iron => "#ff5555",   // Red
            Self::Gold => "#ffd700",   // Gold
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Jade => "Jade Altar",
            Self::Gale => "Gale Altar",
            Self::Mirror => "Mirror Altar",
            Self::Iron => "Iron Altar",
            Self::Gold => "Gold Altar",
        }
    }

    pub fn deity(self) -> Deity {
        match self {
            Self::Jade => Deity::Jade,
            Self::Gale => Deity::Gale,
            Self::Mirror => Deity::Mirror,
            Self::Iron => Deity::Iron,
            Self::Gold => Deity::Gold,
        }
    }

    fn random(rng: &mut Rng) -> Self {
        match rng.next_u64() % 5 {
            0 => Self::Jade,
            1 => Self::Gale,
            2 => Self::Mirror,
            3 => Self::Iron,
            _ => Self::Gold,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SealKind {
    Ember,
    Tide,
    Thorn,
    Echo,
}

impl SealKind {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Ember => "火",
            Self::Tide => "水",
            Self::Thorn => "刃",
            Self::Echo => "回",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Self::Ember => "#ff9b73",
            Self::Tide => "#90c9ff",
            Self::Thorn => "#ff9eb8",
            Self::Echo => "#d4a4ff",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ember => "Ember seal",
            Self::Tide => "Tide seal",
            Self::Thorn => "Thorn seal",
            Self::Echo => "Echo seal",
        }
    }

    fn random(rng: &mut Rng) -> Self {
        match rng.next_u64() % 4 {
            0 => Self::Ember,
            1 => Self::Tide,
            2 => Self::Thorn,
            _ => Self::Echo,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    Wall,
    CrackedWall,
    /// A visible weak barrier for optional vault-style puzzle niches
    BrittleWall,
    Floor,
    Corridor,
    StairsDown,
    Forge,
    Shop,
    Chest,
    /// Smashable crate with supplies or traps
    Crate,
    /// NetHack-style spike trap
    Spikes,
    /// Slick oil that can be ignited by fire magic
    Oil,
    /// Water that conducts stunning spells
    Water,
    /// Too deep to wade through, but a crate can span it
    DeepWater,
    /// NPC companion (0=Teacher, 1=Monk, 2=Merchant, 3=Guard)
    Npc(u8),
    /// Tone shrine for tone battle mini-game
    Shrine,
    /// One-shot altar that grants a blessing
    Altar(AltarKind),
    /// One-shot script seal that reshapes the room when stepped on
    Seal(SealKind),
    /// Tutorial signpost with a scripted message
    Sign(u8),
    /// Bridge created by pushing a crate into water
    Bridge,
}

impl Tile {
    pub fn is_walkable(self) -> bool {
        matches!(
            self,
            Tile::Floor
                | Tile::Corridor
                | Tile::StairsDown
                | Tile::Forge
                | Tile::Shop
                | Tile::Chest
                | Tile::Spikes
                | Tile::Oil
                | Tile::Water
                | Tile::Npc(_)
                | Tile::Shrine
                | Tile::Altar(_)
                | Tile::Seal(_)
                | Tile::Sign(_)
                | Tile::Bridge
        )
    }
}

// ── Room descriptor ─────────────────────────────────────────────────────────

/// Room environment modifier affecting gameplay.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RoomModifier {
    /// Reduced FOV to 2 tiles
    Dark,
    /// Spells deal 2x damage
    Arcane,
    /// Enemies take 1 extra damage per hit
    Cursed,
}

#[derive(Clone)]
pub struct Room {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub modifier: Option<RoomModifier>,
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
            self.room = Some(Room { x, y, w, h, modifier: None });
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
            Tile::Wall
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
                if self.tile(tx, ty) != Tile::Wall {
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

    fn place_secret_room_feature(&mut self, room_x: i32, room_y: i32, room_w: i32, room_h: i32, rng: &mut Rng) {
        let cx = room_x + room_w / 2;
        let cy = room_y + room_h / 2;

        match rng.next_u64() % 4 {
            0 => {
                for &(dx, dy) in &[(0, 0), (-1, 0), (1, 0)] {
                    let tx = cx + dx;
                    let ty = cy + dy;
                    if self.in_bounds(tx, ty) && self.tile(tx, ty) == Tile::Floor {
                        let idx = self.idx(tx, ty);
                        self.tiles[idx] = Tile::Chest;
                    }
                }
            }
            1 => {
                let idx = self.idx(cx, cy);
                self.tiles[idx] = Tile::Altar(AltarKind::random(rng));
            }
            2 => {
                let idx = self.idx(cx, cy);
                self.tiles[idx] = Tile::Shrine;
            }
            _ => {
                let idx = self.idx(cx, cy);
                self.tiles[idx] = Tile::Forge;
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

        self.carve_rect(secret_x, secret_y, secret_w, secret_h, Tile::Floor);
        let door_idx = self.idx(door_x, door_y);
        self.tiles[door_idx] = Tile::CrackedWall;
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
                if self.tile(x, y) == Tile::Water {
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

                if self.tile(crate_x, crate_y) != Tile::Floor {
                    continue;
                }

                let stand_tile = self.tile(stand_x, stand_y);
                if !stand_tile.is_walkable() || stand_tile == Tile::Water {
                    continue;
                }

                let idx = self.idx(crate_x, crate_y);
                self.tiles[idx] = Tile::Crate;
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
                if !matches!(self.tile(x, y), Tile::Floor | Tile::Corridor) {
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
            .any(|&(x, y)| !self.in_bounds(x, y) || self.tile(x, y) != Tile::Floor)
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
            self.tiles[idx] = Tile::Wall;
        }

        let barrier_idx = self.idx(barrier_x, barrier_y);
        self.tiles[barrier_idx] = barrier_tile;

        let reward_idx = self.idx(reward_x, reward_y);
        self.tiles[reward_idx] = Tile::Chest;

        if with_crate {
            let crate_idx = self.idx(approach_x, approach_y);
            self.tiles[crate_idx] = Tile::Crate;
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
            if self.try_place_puzzle_niche(x, y, dx, dy, Tile::BrittleWall, false) {
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
            if self.try_place_puzzle_niche(x, y, dx, dy, Tile::DeepWater, true) {
                return true;
            }
        }

        false
    }

    fn place_puzzle_room_in_room(&mut self, room: &Room, rng: &mut Rng) -> bool {
        let start = (rng.next_u64() % 2) as usize;
        for offset in 0..2 {
            let placed = match (start + offset) % 2 {
                0 => self.try_place_brittle_vault(room, rng),
                _ => self.try_place_deep_water_cache(room, rng),
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

        let desired = if room_count >= 4 && rng.next_u64() % 100 < 50 { 2 } else { 1 };
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
            self.tiles[idx] = Tile::StairsDown;
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
                if used.len() >= candidates.len() { break; }
                continue;
            }
            used.push(pick);
            let room = &self.rooms[candidates[pick]];
            // Place forge at an offset from center so it doesn't overlap stairs
            let fx = room.x + 1;
            let fy = room.y + 1;
            if self.in_bounds(fx, fy) {
                let idx = self.idx(fx, fy);
                if self.tiles[idx] == Tile::Floor {
                    self.tiles[idx] = Tile::Forge;
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
                if self.tiles[idx] == Tile::Floor {
                    self.tiles[idx] = Tile::Shop;
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
                        matches!(self.tiles[idx], Tile::Forge | Tile::Shop | Tile::StairsDown)
                    } else {
                        false
                    }
                })
            });
            if has_special { continue; }

            let chest_count = rng.range(2, 4); // 2-3 chests
            let mut placed = 0;
            for _ in 0..10 {
                let cx = rng.range(room.x + 1, room.x + room.w - 1);
                let cy = rng.range(room.y + 1, room.y + room.h - 1);
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Chest;
                        placed += 1;
                        if placed >= chest_count { break; }
                    }
                }
            }
            if placed > 0 { return; }
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
                                Tile::Forge
                                    | Tile::Shop
                                    | Tile::StairsDown
                                    | Tile::Chest
                                    | Tile::Npc(_)
                                    | Tile::Shrine
                                    | Tile::Altar(_)
                                    | Tile::Seal(_)
                                    | Tile::Sign(_)
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
                    if self.tiles[idx] != Tile::Floor {
                        continue;
                    }
                    self.tiles[idx] = if !guaranteed_water {
                        guaranteed_water = true;
                        Tile::Water
                    } else {
                        match rng.next_u64() % 3 {
                            0 => Tile::Spikes,
                            1 => Tile::Oil,
                            _ => Tile::Water,
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
                                Tile::Forge
                                    | Tile::Shop
                                    | Tile::StairsDown
                                    | Tile::Chest
                                    | Tile::Npc(_)
                                    | Tile::Shrine
                                    | Tile::Altar(_)
                                    | Tile::Seal(_)
                                    | Tile::Sign(_)
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
                    if self.tiles[idx] != Tile::Floor {
                        continue;
                    }
                    self.tiles[idx] = Tile::Crate;
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
        self.rooms
            .first()
            .map(|r| r.center())
            .unwrap_or((1, 1))
    }

    /// Scripted tutorial floor used on the first run.
    pub fn tutorial(width: i32, height: i32) -> Self {
        debug_assert!(width >= 44 && height >= 30, "tutorial floor expects the default map size");

        let size = (width * height) as usize;
        let mut level = DungeonLevel {
            width,
            height,
            tiles: vec![Tile::Wall; size],
            rooms: vec![
                Room { x: 4, y: 18, w: 9, h: 9, modifier: None },
                Room { x: 16, y: 18, w: 9, h: 9, modifier: None },
                Room { x: 29, y: 18, w: 13, h: 9, modifier: None },
            ],
            visible: vec![false; size],
            revealed: vec![false; size],
        };

        fn carve_room(level: &mut DungeonLevel, room: &Room) {
            for ry in room.y..room.y + room.h {
                for rx in room.x..room.x + room.w {
                    let idx = level.idx(rx, ry);
                    level.tiles[idx] = Tile::Floor;
                }
            }
        }

        fn carve_h_corridor(level: &mut DungeonLevel, x1: i32, x2: i32, y: i32) {
            for x in x1.min(x2)..=x1.max(x2) {
                let idx = level.idx(x, y);
                if level.tiles[idx] == Tile::Wall {
                    level.tiles[idx] = Tile::Corridor;
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
        level.tiles[sign0] = Tile::Sign(0);
        level.tiles[sign1] = Tile::Sign(1);
        level.tiles[sign2] = Tile::Sign(2);
        level.tiles[sign3] = Tile::Sign(3);
        level.tiles[forge] = Tile::Forge;
        level.tiles[stairs] = Tile::StairsDown;

        level
    }

    pub fn generate(width: i32, height: i32, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let size = (width * height) as usize;
        let mut tiles = vec![Tile::Wall; size];
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
                        tiles[(ry * width + rx) as usize] = Tile::Floor;
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
                    if tiles[i] == Tile::Wall {
                        tiles[i] = Tile::Corridor;
                    }
                }
            }
            let ymin = y1.min(y2);
            let ymax = y1.max(y2);
            for y in ymin..=ymax {
                if x2 >= 0 && y >= 0 && x2 < width && y < height {
                    let i = (y * width + x2) as usize;
                    if tiles[i] == Tile::Wall {
                        tiles[i] = Tile::Corridor;
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
        level.place_stairs();
        level.place_forges(&mut rng);
        level.place_shop(&mut rng);
        level.place_chests(&mut rng);
        level.assign_room_modifiers(&mut rng);
        level.place_npcs(&mut rng);
        level.place_shrines(&mut rng);
        level.place_altars(&mut rng);
        level.place_seals(&mut rng);
        level.place_hazards(&mut rng);
        level.place_crates(&mut rng);
        level.place_secret_room(&mut rng);
        level.place_puzzle_rooms(&mut rng);
        level
    }

    /// Assign random modifiers to some rooms (not first or last).
    fn assign_room_modifiers(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 2 { return; }
        for i in 1..n - 1 {
            if rng.next_u64() % 100 < 30 {
                self.rooms[i].modifier = Some(match rng.next_u64() % 3 {
                    0 => RoomModifier::Dark,
                    1 => RoomModifier::Arcane,
                    _ => RoomModifier::Cursed,
                });
            }
        }
    }

    /// Place a companion NPC in a random middle room (~40% chance per floor).
    fn place_npcs(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 { return; }
        if rng.next_u64() % 100 >= 40 { return; }
        // Pick a random middle room (not first, last, or second-to-last)
        let room_idx = 1 + (rng.next_u64() as usize % (n - 3));
        let room = &self.rooms[room_idx];
        let npc_type = (rng.next_u64() % 4) as u8;
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2 + 1; // offset from center
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::Npc(npc_type);
        }
    }

    /// Place a tone shrine in a random middle room (~30% chance).
    fn place_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 { return; }
        if rng.next_u64() % 100 >= 30 { return; }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 1;
        let cy = room.y + room.h / 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::Shrine;
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
                            Tile::Forge
                                | Tile::Shop
                                | Tile::StairsDown
                                | Tile::Chest
                                | Tile::Npc(_)
                                | Tile::Shrine
                                | Tile::Altar(_)
                                | Tile::Seal(_)
                                | Tile::Sign(_)
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
            if self.tiles[idx] == Tile::Floor {
                self.tiles[idx] = Tile::Altar(AltarKind::random(rng));
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
                            Tile::Forge
                                | Tile::Shop
                                | Tile::StairsDown
                                | Tile::Chest
                                | Tile::Npc(_)
                                | Tile::Shrine
                                | Tile::Altar(_)
                                | Tile::Seal(_)
                                | Tile::Sign(_)
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
            if self.tiles[idx] != Tile::Floor {
                continue;
            }

            self.tiles[idx] = Tile::Seal(SealKind::random(rng));
            used_rooms.push(room_idx);
            placed += 1;
        }
    }
}


#[cfg(test)]
mod tests {
    use super::{AltarKind, DungeonLevel, Room, Rng, SealKind, Tile};

    fn make_clean_test_level() -> DungeonLevel {
        let width = 24;
        let height = 24;
        let mut level = DungeonLevel {
            width,
            height,
            tiles: vec![Tile::Wall; (width * height) as usize],
            rooms: vec![
                Room { x: 1, y: 1, w: 5, h: 5, modifier: None },
                Room { x: 8, y: 1, w: 5, h: 5, modifier: None },
                Room { x: 15, y: 1, w: 5, h: 5, modifier: None },
                Room { x: 1, y: 10, w: 5, h: 5, modifier: None },
                Room { x: 8, y: 10, w: 5, h: 5, modifier: None },
            ],
            visible: vec![false; (width * height) as usize],
            revealed: vec![false; (width * height) as usize],
        };
        for room in level.rooms.clone() {
            for y in room.y..room.y + room.h {
                for x in room.x..room.x + room.w {
                    let idx = level.idx(x, y);
                    level.tiles[idx] = Tile::Floor;
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
            tiles: vec![Tile::Wall; (width * height) as usize],
            rooms: vec![
                Room { x: 1, y: 1, w: 8, h: 8, modifier: None },
                Room { x: 11, y: 1, w: 8, h: 8, modifier: None },
                Room { x: 21, y: 1, w: 8, h: 8, modifier: None },
                Room { x: 1, y: 12, w: 8, h: 8, modifier: None },
                Room { x: 11, y: 12, w: 8, h: 8, modifier: None },
            ],
            visible: vec![false; (width * height) as usize],
            revealed: vec![false; (width * height) as usize],
        };
        for room in level.rooms.clone() {
            for y in room.y..room.y + room.h {
                for x in room.x..room.x + room.w {
                    let idx = level.idx(x, y);
                    level.tiles[idx] = Tile::Floor;
                }
            }
        }
        level
    }

    fn has_pushable_bridge_setup(level: &DungeonLevel) -> bool {
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for y in 0..level.height {
            for x in 0..level.width {
                if level.tile(x, y) != Tile::Water {
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

                    if level.tile(crate_x, crate_y) == Tile::Crate {
                        let stand_tile = level.tile(stand_x, stand_y);
                        if stand_tile.is_walkable() && stand_tile != Tile::Water {
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
                if level.tile(x, y) != Tile::BrittleWall {
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

                    if level.tile(chest_x, chest_y) == Tile::Chest
                        && level.tile(back_x, back_y) == Tile::Wall
                        && level.tile(x + side_a.0, y + side_a.1) == Tile::Wall
                        && level.tile(chest_x + side_a.0, chest_y + side_a.1) == Tile::Wall
                        && level.tile(x + side_b.0, y + side_b.1) == Tile::Wall
                        && level.tile(chest_x + side_b.0, chest_y + side_b.1) == Tile::Wall
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
                if level.tile(x, y) != Tile::DeepWater {
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

                    if level.tile(crate_x, crate_y) == Tile::Crate
                        && level.tile(stand_x, stand_y) == Tile::Floor
                        && level.tile(chest_x, chest_y) == Tile::Chest
                        && level.tile(back_x, back_y) == Tile::Wall
                        && level.tile(x + side_a.0, y + side_a.1) == Tile::Wall
                        && level.tile(chest_x + side_a.0, chest_y + side_a.1) == Tile::Wall
                        && level.tile(x + side_b.0, y + side_b.1) == Tile::Wall
                        && level.tile(chest_x + side_b.0, chest_y + side_b.1) == Tile::Wall
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    #[test]
    fn hazards_and_altars_are_walkable_but_crates_block() {
        assert!(Tile::Spikes.is_walkable());
        assert!(Tile::Oil.is_walkable());
        assert!(Tile::Water.is_walkable());
        assert!(Tile::Altar(AltarKind::Jade).is_walkable());
        assert!(Tile::Seal(SealKind::Ember).is_walkable());
        assert!(!Tile::Crate.is_walkable());
        assert!(!Tile::CrackedWall.is_walkable());
        assert!(!Tile::BrittleWall.is_walkable());
        assert!(!Tile::DeepWater.is_walkable());
    }

    #[test]
    fn place_altars_adds_a_blessing_site_to_clean_levels() {
        let mut level = make_clean_test_level();
        let mut rng = Rng::new(2);

        level.place_altars(&mut rng);

        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::Altar(_))));
    }

    #[test]
    fn place_seals_adds_script_seals_to_clean_levels() {
        let mut level = make_clean_test_level();
        let mut rng = Rng::new(7);

        level.place_seals(&mut rng);

        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::Seal(_))));
    }

    #[test]
    fn place_secret_room_carves_hidden_chamber_with_cracked_entrance() {
        let mut level = make_clean_test_level();
        let mut rng = Rng::new(11);
        let original_open_tiles = level.tiles.iter().filter(|tile| !matches!(tile, Tile::Wall)).count();

        level.place_secret_room(&mut rng);

        let new_open_tiles = level.tiles.iter().filter(|tile| !matches!(tile, Tile::Wall)).count();
        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::CrackedWall)));
        assert!(new_open_tiles > original_open_tiles);
    }

    #[test]
    fn place_secret_room_adds_hidden_point_of_interest() {
        let mut level = make_clean_test_level();
        let mut rng = Rng::new(11);

        level.place_secret_room(&mut rng);

        assert!(level.tiles.iter().any(|tile| {
            matches!(tile, Tile::Chest | Tile::Forge | Tile::Shrine | Tile::Altar(_))
        }));
    }

    #[test]
    fn generated_levels_hide_secret_rooms_on_most_runs() {
        let mut secret_count = 0;
        for seed in 1..=24 {
            let level = DungeonLevel::generate(48, 48, seed);
            if level.tiles.iter().any(|tile| matches!(tile, Tile::CrackedWall)) {
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
            let level = DungeonLevel::generate(48, 48, seed);
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

        assert!(has_brittle_vault(&level) || has_deep_water_cache(&level));
    }

    #[test]
    fn generated_levels_regularly_offer_puzzle_rooms() {
        let mut puzzle_count = 0;
        let mut brittle_count = 0;
        let mut deep_water_count = 0;
        for seed in 1..=24 {
            let level = DungeonLevel::generate(48, 48, seed);
            let has_brittle = has_brittle_vault(&level);
            let has_deep = has_deep_water_cache(&level);
            if has_brittle || has_deep {
                puzzle_count += 1;
            }
            if has_brittle {
                brittle_count += 1;
            }
            if has_deep {
                deep_water_count += 1;
            }
        }

        assert!(
            puzzle_count >= 16,
            "expected puzzle rooms on most sample floors, found {puzzle_count}"
        );
        assert!(brittle_count > 0, "expected at least one brittle-wall vault in the sample set");
        assert!(deep_water_count > 0, "expected at least one deep-water cache in the sample set");
    }

    #[test]
    fn tutorial_floor_has_required_landmarks() {
        let level = DungeonLevel::tutorial(48, 48);

        assert_eq!(level.start_pos(), (8, 22));
        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::Sign(0))));
        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::Sign(1))));
        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::Sign(2))));
        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::Sign(3))));
        assert!(level.tiles.iter().any(|tile| *tile == Tile::Forge));
        assert!(level.tiles.iter().any(|tile| *tile == Tile::StairsDown));
        assert!(level.is_walkable(8, 20));
    }
}
