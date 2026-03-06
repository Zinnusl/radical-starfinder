//! Player state and movement.

use crate::radical::Spell;
use crate::status::StatusInstance;

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

pub const MAX_ITEMS: usize = 5;

/// Consumable items the player can carry and use.
#[derive(Clone, Debug)]
pub enum Item {
    /// Heal N HP instantly
    HealthPotion(i32),
    /// Apply poison (dmg, turns) to adjacent enemies
    PoisonFlask(i32, i32),
    /// Reveal entire floor map
    RevealScroll,
    /// Teleport to random explored walkable tile
    TeleportScroll,
    /// Grant haste for N turns
    HastePotion(i32),
    /// Stun all visible enemies
    StunBomb,
}

impl Item {
    pub fn name(&self) -> &'static str {
        match self {
            Item::HealthPotion(_) => "💚 Health Potion",
            Item::PoisonFlask(_, _) => "☠ Poison Flask",
            Item::RevealScroll => "👁 Reveal Scroll",
            Item::TeleportScroll => "✦ Teleport Scroll",
            Item::HastePotion(_) => "⚡ Haste Potion",
            Item::StunBomb => "💥 Stun Bomb",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Item::HealthPotion(_) => "HP Pot",
            Item::PoisonFlask(_, _) => "Poison",
            Item::RevealScroll => "Reveal",
            Item::TeleportScroll => "Teleport",
            Item::HastePotion(_) => "Haste",
            Item::StunBomb => "Stun",
        }
    }
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
    /// Active status effects
    pub statuses: Vec<StatusInstance>,
    /// Consumable items (max MAX_ITEMS)
    pub items: Vec<Item>,
    /// Equipped items (up to 3: weapon, armor, charm)
    pub weapon: Option<&'static Equipment>,
    pub armor: Option<&'static Equipment>,
    pub charm: Option<&'static Equipment>,
    /// Enchantments on equipment slots: [weapon, armor, charm]
    pub enchantments: [Option<&'static str>; 3],
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
            statuses: Vec::new(),
            items: Vec::new(),
            weapon: None,
            armor: None,
            charm: None,
            enchantments: [None; 3],
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

    pub fn add_item(&mut self, item: Item) -> bool {
        if self.items.len() < MAX_ITEMS {
            self.items.push(item);
            true
        } else {
            false
        }
    }

    pub fn take_item(&mut self, idx: usize) -> Option<Item> {
        if idx < self.items.len() {
            Some(self.items.remove(idx))
        } else {
            None
        }
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

    /// Bonus damage from enchantments (力=+1, 火=+1)
    pub fn enchant_bonus_damage(&self) -> i32 {
        self.enchantments.iter().filter_map(|e| *e).map(|r| match r {
            "力" | "火" => 1,
            _ => 0,
        }).sum()
    }

    /// Bonus damage reduction from enchantments (水=+1, 土=+1)
    pub fn enchant_damage_reduction(&self) -> i32 {
        self.enchantments.iter().filter_map(|e| *e).map(|r| match r {
            "水" | "土" => 1,
            _ => 0,
        }).sum()
    }

    /// Bonus max HP from enchantments (心=+2)
    pub fn enchant_max_hp_bonus(&self) -> i32 {
        self.enchantments.iter().filter_map(|e| *e).map(|r| match r {
            "心" => 2,
            _ => 0,
        }).sum()
    }

    /// Bonus gold from enchantments (金=+3)
    pub fn enchant_gold_bonus(&self) -> i32 {
        self.enchantments.iter().filter_map(|e| *e).map(|r| match r {
            "金" => 3,
            _ => 0,
        }).sum()
    }

    /// Bonus FOV from enchantments (目=+1)
    pub fn enchant_fov_bonus(&self) -> i32 {
        self.enchantments.iter().filter_map(|e| *e).map(|r| match r {
            "目" => 1,
            _ => 0,
        }).sum()
    }
}
