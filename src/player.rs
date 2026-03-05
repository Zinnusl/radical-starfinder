//! Player state and movement.

use crate::radical::Spell;

/// Equipment slot types
#[derive(Clone, Debug)]
pub struct Equipment {
    pub name: &'static str,
    pub slot: EquipSlot,
    pub effect: EquipEffect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipSlot {
    Weapon,
    Armor,
    Charm,
}

#[derive(Clone, Copy, Debug)]
pub enum EquipEffect {
    /// Extra damage on correct answer
    BonusDamage(i32),
    /// Reduce incoming damage
    DamageReduction(i32),
    /// Extra radical drop chance (percentage 0-100)
    ExtraRadicalDrop(i32),
    /// Heal on kill
    HealOnKill(i32),
    /// Extra gold on kill
    GoldBonus(i32),
}

pub const EQUIPMENT_POOL: &[Equipment] = &[
    Equipment { name: "Brush of Clarity", slot: EquipSlot::Weapon, effect: EquipEffect::BonusDamage(1) },
    Equipment { name: "Scholar's Quill", slot: EquipSlot::Weapon, effect: EquipEffect::BonusDamage(2) },
    Equipment { name: "Dragon Fang Pen", slot: EquipSlot::Weapon, effect: EquipEffect::BonusDamage(3) },
    Equipment { name: "Jade Vest", slot: EquipSlot::Armor, effect: EquipEffect::DamageReduction(1) },
    Equipment { name: "Iron Silk Robe", slot: EquipSlot::Armor, effect: EquipEffect::DamageReduction(2) },
    Equipment { name: "Phoenix Mantle", slot: EquipSlot::Armor, effect: EquipEffect::DamageReduction(3) },
    Equipment { name: "Radical Magnet", slot: EquipSlot::Charm, effect: EquipEffect::ExtraRadicalDrop(50) },
    Equipment { name: "Life Jade", slot: EquipSlot::Charm, effect: EquipEffect::HealOnKill(2) },
    Equipment { name: "Gold Toad", slot: EquipSlot::Charm, effect: EquipEffect::GoldBonus(10) },
    Equipment { name: "Phoenix Feather", slot: EquipSlot::Charm, effect: EquipEffect::HealOnKill(3) },
];

#[derive(Clone)]
pub struct Player {
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    /// Collected radicals (stored as their &str character)
    pub radicals: Vec<&'static str>,
    /// Forged spells ready to use in combat
    pub spells: Vec<Spell>,
    /// Index of currently selected spell (for combat use)
    pub selected_spell: usize,
    /// Shield active (blocks next hit)
    pub shield: bool,
    /// Equipped items (up to 3: weapon, armor, charm)
    pub weapon: Option<&'static Equipment>,
    pub armor: Option<&'static Equipment>,
    pub charm: Option<&'static Equipment>,
}

impl Player {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            hp: 10,
            max_hp: 10,
            gold: 0,
            radicals: Vec::new(),
            spells: Vec::new(),
            selected_spell: 0,
            shield: false,
            weapon: None,
            armor: None,
            charm: None,
        }
    }

    /// Attempt to move by (dx, dy). Returns the target position.
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

    pub fn add_spell(&mut self, spell: Spell) {
        self.spells.push(spell);
    }

    pub fn cycle_spell(&mut self) -> bool {
        if self.spells.is_empty() {
            return false;
        }
        self.selected_spell = (self.selected_spell + 1) % self.spells.len();
        true
    }

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

    /// Get bonus attack damage from equipment
    pub fn bonus_damage(&self) -> i32 {
        match self.weapon {
            Some(eq) => match eq.effect {
                EquipEffect::BonusDamage(d) => d,
                _ => 0,
            },
            None => 0,
        }
    }

    /// Get damage reduction from armor
    pub fn damage_reduction(&self) -> i32 {
        match self.armor {
            Some(eq) => match eq.effect {
                EquipEffect::DamageReduction(d) => d,
                _ => 0,
            },
            None => 0,
        }
    }

    /// Check extra radical drop chance (percentage)
    pub fn extra_radical_chance(&self) -> i32 {
        match self.charm {
            Some(eq) => match eq.effect {
                EquipEffect::ExtraRadicalDrop(pct) => pct,
                _ => 0,
            },
            None => 0,
        }
    }

    /// Get heal-on-kill amount
    pub fn heal_on_kill(&self) -> i32 {
        match self.charm {
            Some(eq) => match eq.effect {
                EquipEffect::HealOnKill(amt) => amt,
                _ => 0,
            },
            None => 0,
        }
    }

    /// Get gold bonus
    pub fn gold_bonus(&self) -> i32 {
        match self.charm {
            Some(eq) => match eq.effect {
                EquipEffect::GoldBonus(amt) => amt,
                _ => 0,
            },
            None => 0,
        }
    }

    pub fn equip(&mut self, equipment: &'static Equipment) {
        match equipment.slot {
            EquipSlot::Weapon => self.weapon = Some(equipment),
            EquipSlot::Armor => self.armor = Some(equipment),
            EquipSlot::Charm => self.charm = Some(equipment),
        }
    }
}
