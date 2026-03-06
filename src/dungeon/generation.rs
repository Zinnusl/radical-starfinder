//! BSP-based dungeon generation.
//!
//! Splits a rectangular area recursively, places rooms in leaves,
//! then connects sibling rooms with corridors.

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AltarKind {
    Jade,
    Gale,
    Mirror,
}

impl AltarKind {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Jade => "☯",
            Self::Gale => "✦",
            Self::Mirror => "◈",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Self::Jade => "#66dd99",
            Self::Gale => "#88ccff",
            Self::Mirror => "#ddb8ff",
        }
    }

    fn random(rng: &mut Rng) -> Self {
        match rng.next_u64() % 3 {
            0 => Self::Jade,
            1 => Self::Gale,
            _ => Self::Mirror,
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
                    self.tiles[idx] = match rng.next_u64() % 3 {
                        0 => Tile::Spikes,
                        1 => Tile::Oil,
                        _ => Tile::Water,
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

    #[test]
    fn hazards_and_altars_are_walkable_but_crates_block() {
        assert!(Tile::Spikes.is_walkable());
        assert!(Tile::Oil.is_walkable());
        assert!(Tile::Water.is_walkable());
        assert!(Tile::Altar(AltarKind::Jade).is_walkable());
        assert!(Tile::Seal(SealKind::Ember).is_walkable());
        assert!(!Tile::Crate.is_walkable());
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
