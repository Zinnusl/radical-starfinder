//! BSP tree for recursive spatial subdivision.

use super::super::*;

// ── BSP node ────────────────────────────────────────────────────────────────

pub(super) struct BspNode {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
    pub(super) left: Option<Box<BspNode>>,
    pub(super) right: Option<Box<BspNode>>,
    pub(super) room: Option<Room>,
}

pub(super) const MIN_LEAF: i32 = 7;
pub(super) const MIN_ROOM: i32 = 4;

impl BspNode {
    pub(super) fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
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

    pub(super) fn split(&mut self, rng: &mut Rng) -> bool {
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

    pub(super) fn create_rooms(&mut self, rng: &mut Rng) {
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

    pub(super) fn get_room(&self) -> Option<&Room> {
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

    pub(super) fn collect_corridors(&self, corridors: &mut Vec<((i32, i32), (i32, i32))>) {
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
