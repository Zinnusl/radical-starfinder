//! BSP-based procedural location generation for space facilities.
//!
//! Splits a rectangular area recursively, places modules in leaves,
//! then connects sibling compartments with hallways.

use super::*;


mod bsp;
mod special_rooms;
mod features;
mod puzzles;
use bsp::BspNode;

impl LocationLevel {
    pub(super) fn area_is_solid_wall(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
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

    pub(super) fn carve_rect(&mut self, x: i32, y: i32, w: i32, h: i32, tile: Tile) {
        for ty in y..y + h {
            for tx in x..x + w {
                let idx = self.idx(tx, ty);
                self.tiles[idx] = tile;
            }
        }
    }

    pub(super) fn place_secret_room_feature(
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
                self.tiles[idx] = Tile::NavBeacon;
            }
            _ => {
                let idx = self.idx(cx, cy);
                self.tiles[idx] = Tile::QuantumForge;
            }
        }
    }

    pub(super) fn try_place_secret_room_candidate(
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

    pub(super) fn place_secret_room(&mut self, rng: &mut Rng) {
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

    /// Place airlock in the last module.
    pub fn place_stairs(&mut self) {
        if let Some(room) = self.rooms.last() {
            let (cx, cy) = room.center();
            let idx = self.idx(cx, cy);
            self.tiles[idx] = Tile::Airlock;
        }
    }

    /// Place quantum forges in 1-2 middle modules.
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

    /// Place a trade terminal in one middle module (if enough modules).
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

    /// Place supply crates in one module (2-3 crates).
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

    /// Place environmental hazard tiles.
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
                                    | Tile::NavBeacon
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
                            0 => Tile::LaserGrid,
                            1 => Tile::Coolant,
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

    /// Place smashable containers with supplies or hazardous gas.
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
                                    | Tile::NavBeacon
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

    /// Get player start position (center of first module).
    pub fn start_pos(&self) -> (i32, i32) {
        self.rooms.first().map(|r| r.center()).unwrap_or((1, 1))
    }

    /// Scripted tutorial deck used on the first run.
    pub fn tutorial(width: i32, height: i32) -> Self {
        debug_assert!(
            width >= 44 && height >= 30,
            "tutorial deck expects the default map size"
        );

        let size = (width * height) as usize;
        let mut level = LocationLevel {
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

        fn carve_room(level: &mut LocationLevel, room: &Room) {
            for ry in room.y..room.y + room.h {
                for rx in room.x..room.x + room.w {
                    let idx = level.idx(rx, ry);
                    level.tiles[idx] = Tile::MetalFloor;
                }
            }
        }

        fn carve_h_corridor(level: &mut LocationLevel, x1: i32, x2: i32, y: i32) {
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

    pub fn generate(width: i32, height: i32, seed: u64, floor: i32, location_type: LocationType) -> Self {
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

        let mut level = LocationLevel {
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

        // Place trap tiles on deeper decks
        if floor >= 2 {
            let trap_count = 2 + floor / 3;
            let mut placed = 0;
            for _ in 0..trap_count * 10 {
                let x = (rng.next_u64() % width as u64) as i32;
                let y = (rng.next_u64() % height as u64) as i32;
                let idx = (y * width + x) as usize;
                if idx < level.tiles.len() && level.tiles[idx] == Tile::MetalFloor {
                    let trap_type = (rng.next_u64() % 3) as u8;
                    level.tiles[idx] = Tile::Trap(trap_type);
                    placed += 1;
                    if placed >= trap_count {
                        break;
                    }
                }
            }
        }


        // Location-type-specific adjustments
        match location_type {
            LocationType::SpaceStation => {
                level.place_shop(&mut rng);
            }
            LocationType::AsteroidBase => {
                for _ in 0..8 {
                    let x = rng.range(1, width - 1);
                    let y = rng.range(1, height - 1);
                    let idx = (y * width + x) as usize;
                    if idx < level.tiles.len() && level.tiles[idx] == Tile::Bulkhead {
                        let mut near_hallway = false;
                        for &(dx, dy) in &[(-1i32,0),(1,0),(0,-1),(0,1)] {
                            let nx = x + dx;
                            let ny = y + dy;
                            if level.in_bounds(nx, ny) && level.tiles[level.idx(nx, ny)] == Tile::Hallway {
                                near_hallway = true;
                                break;
                            }
                        }
                        if near_hallway {
                            level.tiles[idx] = Tile::OreVein;
                        }
                    }
                }
            }
            LocationType::DerelictShip => {
                let breach_count = 3 + floor / 2;
                let mut placed = 0;
                for _ in 0..breach_count * 10 {
                    let x = rng.range(1, width - 1);
                    let y = rng.range(1, height - 1);
                    let idx = (y * width + x) as usize;
                    if idx < level.tiles.len() && level.tiles[idx] == Tile::MetalFloor {
                        level.tiles[idx] = Tile::VacuumBreach;
                        placed += 1;
                        if placed >= breach_count {
                            break;
                        }
                    }
                }
            }
            LocationType::AlienRuins => {
                for _ in 0..12 {
                    let x = rng.range(1, width - 1);
                    let y = rng.range(1, height - 1);
                    let idx = (y * width + x) as usize;
                    if idx < level.tiles.len() && level.tiles[idx] == Tile::MetalFloor {
                        level.tiles[idx] = Tile::CrystalPanel;
                    }
                }
            }
            LocationType::TradingPost => {
                level.place_shop(&mut rng);
                level.place_shop(&mut rng);
            }
            _ => {}
        }

        level
    }

    /// Assign 2-4 modules per deck as special compartments with unique layouts.
    pub(super) fn place_special_rooms(&mut self, rng: &mut Rng, floor: i32) {
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

}


#[cfg(test)]
mod tests;
