//! Player state and movement.

use crate::radical::Spell;

#[derive(Clone)]
pub struct Player {
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub max_hp: i32,
    /// Collected radicals (stored as their &str character)
    pub radicals: Vec<&'static str>,
    /// Forged spells ready to use in combat
    pub spells: Vec<Spell>,
    /// Index of currently selected spell (for combat use)
    pub selected_spell: usize,
    /// Shield active (blocks next hit)
    pub shield: bool,
}

impl Player {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            hp: 10,
            max_hp: 10,
            radicals: Vec::new(),
            spells: Vec::new(),
            selected_spell: 0,
            shield: false,
        }
    }

    /// Attempt to move by (dx, dy). Returns the target position.
    /// Caller is responsible for collision checking.
    pub fn intended_move(&self, dx: i32, dy: i32) -> (i32, i32) {
        (self.x + dx, self.y + dy)
    }

    pub fn move_to(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn add_radical(&mut self, ch: &'static str) {
        self.radicals.push(ch);
    }

    pub fn has_radical(&self, ch: &str) -> bool {
        self.radicals.contains(&ch)
    }

    /// Remove specific radicals by their characters (consumes them in forging).
    pub fn remove_radicals(&mut self, chars: &[&str]) {
        for ch in chars {
            if let Some(pos) = self.radicals.iter().position(|r| r == ch) {
                self.radicals.remove(pos);
            }
        }
    }

    pub fn add_spell(&mut self, spell: Spell) {
        self.spells.push(spell);
    }

    /// Cycle to next spell. Returns true if there are spells.
    pub fn cycle_spell(&mut self) -> bool {
        if self.spells.is_empty() {
            return false;
        }
        self.selected_spell = (self.selected_spell + 1) % self.spells.len();
        true
    }

    /// Use the currently selected spell, removing it from inventory.
    pub fn use_spell(&mut self) -> Option<Spell> {
        if self.spells.is_empty() {
            return None;
        }
        let idx = self.selected_spell.min(self.spells.len() - 1);
        let spell = self.spells.remove(idx);
        if self.selected_spell >= self.spells.len() && !self.spells.is_empty() {
            self.selected_spell = 0;
        }
        Some(spell)
    }
}
