//! Enemy entities that live on the dungeon floor.

use crate::vocab::VocabEntry;

#[derive(Clone)]
pub struct Enemy {
    pub x: i32,
    pub y: i32,
    pub hanzi: &'static str,
    pub pinyin: &'static str,
    pub meaning: &'static str,
    pub hp: i32,
    pub max_hp: i32,
    pub damage: i32,
    /// Set when the enemy is alerted (player in same room / nearby)
    pub alert: bool,
}

impl Enemy {
    pub fn from_vocab(entry: &'static VocabEntry, x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            hanzi: entry.hanzi,
            pinyin: entry.pinyin,
            meaning: entry.meaning,
            hp: 2,
            max_hp: 2,
            damage: 2,
            alert: false,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// Simple chase AI: move one step toward (tx, ty) if possible.
    /// Returns desired (nx, ny). Caller checks walkability & occupancy.
    pub fn step_toward(&self, tx: i32, ty: i32) -> (i32, i32) {
        let dx = (tx - self.x).signum();
        let dy = (ty - self.y).signum();
        // Prefer axis with larger distance
        if (tx - self.x).abs() >= (ty - self.y).abs() {
            if dx != 0 {
                return (self.x + dx, self.y);
            }
            (self.x, self.y + dy)
        } else {
            if dy != 0 {
                return (self.x, self.y + dy);
            }
            (self.x + dx, self.y)
        }
    }
}
