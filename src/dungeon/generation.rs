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
pub enum Tile {
    Wall,
    Floor,
    Corridor,
    StairsDown,
    Forge,
}

impl Tile {
    pub fn is_walkable(self) -> bool {
        matches!(self, Tile::Floor | Tile::Corridor | Tile::StairsDown | Tile::Forge)
    }
}

// ── Room descriptor ─────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Room {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
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
            self.room = Some(Room { x, y, w, h });
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

    /// Get player start position (center of first room).
    pub fn start_pos(&self) -> (i32, i32) {
        self.rooms
            .first()
            .map(|r| r.center())
            .unwrap_or((1, 1))
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
        level
    }
}
