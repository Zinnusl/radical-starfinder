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
    /// Allows digging through walls
    Digging,
}

impl EquipEffect {
    pub fn description(&self) -> String {
        match self {
            EquipEffect::BonusDamage(n) => {
                format!("Deals +{} bonus damage on correct combat answers.", n)
            }
            EquipEffect::DamageReduction(n) => format!("Reduces incoming damage by {} per hit.", n),
            EquipEffect::ExtraRadicalDrop(n) => format!(
                "{}% extra chance to find radical drops from defeated enemies.",
                n
            ),
            EquipEffect::HealOnKill(n) => {
                format!("Restores {} HP whenever you defeat an enemy.", n)
            }
            EquipEffect::GoldBonus(n) => {
                format!("Earn +{} bonus gold from each enemy defeated.", n)
            }
            EquipEffect::Digging => {
                "Allows you to dig through dungeon walls by walking into them.".to_string()
            }
        }
    }
}

impl Equipment {
    pub fn description(&self) -> String {
        let slot = match self.slot {
            EquipSlot::Weapon => "Weapon",
            EquipSlot::Armor => "Armor",
            EquipSlot::Charm => "Charm",
        };
        format!("[{}] {}", slot, self.effect.description())
    }
}

#[allow(dead_code)]
pub const MAX_ITEMS: usize = 5;
pub const ITEM_KIND_COUNT: usize = 6;
pub const MYSTERY_ITEM_APPEARANCES: [&str; ITEM_KIND_COUNT] = [
    "Vermilion Seal 朱符",
    "Jade Seal 玉符",
    "Cloud Seal 云符",
    "Ink Seal 墨符",
    "Mirror Seal 镜符",
    "Storm Seal 雷符",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemKind {
    HealthPotion,
    PoisonFlask,
    RevealScroll,
    TeleportScroll,
    HastePotion,
    StunBomb,
}

impl ItemKind {
    pub fn index(self) -> usize {
        match self {
            ItemKind::HealthPotion => 0,
            ItemKind::PoisonFlask => 1,
            ItemKind::RevealScroll => 2,
            ItemKind::TeleportScroll => 3,
            ItemKind::HastePotion => 4,
            ItemKind::StunBomb => 5,
        }
    }
}

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
    pub fn kind(&self) -> ItemKind {
        match self {
            Item::HealthPotion(_) => ItemKind::HealthPotion,
            Item::PoisonFlask(_, _) => ItemKind::PoisonFlask,
            Item::RevealScroll => ItemKind::RevealScroll,
            Item::TeleportScroll => ItemKind::TeleportScroll,
            Item::HastePotion(_) => ItemKind::HastePotion,
            Item::StunBomb => ItemKind::StunBomb,
        }
    }

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

    #[allow(dead_code)]
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

    pub fn display_name(&self, identified: bool, appearance: &'static str) -> String {
        if identified {
            self.name().to_string()
        } else {
            format!("? {}", appearance)
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Item::HealthPotion(_) => "Restores HP instantly when used. Drink during exploration or combat to heal wounds.",
            Item::PoisonFlask(_, _) => "Throw at an adjacent enemy to inflict poison damage over several turns.",
            Item::RevealScroll => "Reveals the entire floor map, showing all rooms, corridors, and hidden areas.",
            Item::TeleportScroll => "Instantly teleport to a random explored tile. Useful for escaping danger.",
            Item::HastePotion(_) => "Grants Haste, letting you take extra actions each turn for a short duration.",
            Item::StunBomb => "Stuns all visible enemies for several turns, preventing them from acting.",
        }
    }
}

pub const EQUIPMENT_POOL: &[Equipment] = &[
    Equipment {
        name: "Brush of Clarity",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::BonusDamage(1),
    },
    Equipment {
        name: "Scholar's Quill",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::BonusDamage(2),
    },
    Equipment {
        name: "Dragon Fang Pen",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::BonusDamage(3),
    },
    Equipment {
        name: "Jade Vest",
        slot: EquipSlot::Armor,
        effect: EquipEffect::DamageReduction(1),
    },
    Equipment {
        name: "Iron Silk Robe",
        slot: EquipSlot::Armor,
        effect: EquipEffect::DamageReduction(2),
    },
    Equipment {
        name: "Phoenix Mantle",
        slot: EquipSlot::Armor,
        effect: EquipEffect::DamageReduction(3),
    },
    Equipment {
        name: "Radical Magnet",
        slot: EquipSlot::Charm,
        effect: EquipEffect::ExtraRadicalDrop(50),
    },
    Equipment {
        name: "Life Jade",
        slot: EquipSlot::Charm,
        effect: EquipEffect::HealOnKill(2),
    },
    Equipment {
        name: "Gold Toad",
        slot: EquipSlot::Charm,
        effect: EquipEffect::GoldBonus(10),
    },
    Equipment {
        name: "Phoenix Feather",
        slot: EquipSlot::Charm,
        effect: EquipEffect::HealOnKill(3),
    },
    Equipment {
        name: "Iron Pickaxe",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::Digging,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Deity {
    Jade,   // Life
    Gale,   // Travel
    Mirror, // Knowledge
    Iron,   // War
    Gold,   // Wealth
}

impl Deity {
    pub fn name(&self) -> &'static str {
        match self {
            Deity::Jade => "Jade Emperor (Life)",
            Deity::Gale => "Wind Walker (Travel)",
            Deity::Mirror => "Mirror Sage (Knowledge)",
            Deity::Iron => "Iron General (War)",
            Deity::Gold => "Golden Toad (Wealth)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerForm {
    Human,
    Flame, // Immune to fire, burn on touch
    #[allow(dead_code)]
    Stone, // High Def, slow
    Mist,  // High Evasion, weak atk
    Tiger, // High Atk, fast
}

impl PlayerForm {
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            PlayerForm::Human => "Human",
            PlayerForm::Flame => "Flame Avatar",
            PlayerForm::Stone => "Stone Golem",
            PlayerForm::Mist => "Mist Spirit",
            PlayerForm::Tiger => "Tiger Demon",
        }
    }

    pub fn glyph(&self) -> &'static str {
        match self {
            PlayerForm::Human => "@",
            PlayerForm::Flame => "火",
            PlayerForm::Stone => "石",
            PlayerForm::Mist => "气",
            PlayerForm::Tiger => "虎",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            PlayerForm::Human => "#ffffff",
            PlayerForm::Flame => "#ff5500",
            PlayerForm::Stone => "#888888",
            PlayerForm::Mist => "#aaddff",
            PlayerForm::Tiger => "#ffaa00",
        }
    }
}

/// Player class specialization chosen at game start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerClass {
    /// Balanced: no bonuses
    Scholar,
    /// +3 HP, +1 damage, -1 item slot
    Warrior,
    /// 2x item effectiveness, +1 item slot
    Alchemist,
}

#[derive(Clone)]
pub struct Player {
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    /// Player class
    pub class: PlayerClass,
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
    /// Bonus damage from tone shrine (used once, then reset)
    pub tone_bonus_damage: i32,
    /// Bonus defense from tone defense wall (used once, then reset)
    pub defense_bonus: i32,
    /// Bonus spell power from compound builder (used once, then reset)
    pub spell_power_temp_bonus: i32,
    /// Permanent shop discount from meta progression (percentage)
    pub shop_discount_pct: i32,
    /// Permanent spell potency bonus from meta progression
    pub spell_power_bonus: i32,
    /// Active god favor (piety)
    pub piety: Vec<(Deity, i32)>,
    /// Current physical form
    pub form: PlayerForm,
    /// Turns remaining in current form (0 = permanent/human)
    pub form_timer: i32,
}

impl Player {
    pub fn new(x: i32, y: i32, class: PlayerClass) -> Self {
        let (hp, max_hp) = match class {
            PlayerClass::Warrior => (13, 13),
            _ => (10, 10),
        };
        Self {
            x,
            y,
            hp,
            max_hp,
            gold: 0,
            class,
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
            tone_bonus_damage: 0,
            defense_bonus: 0,
            spell_power_temp_bonus: 0,
            shop_discount_pct: 0,
            spell_power_bonus: 0,
            piety: Vec::new(),
            form: PlayerForm::Human,
            form_timer: 0,
        }
    }

    pub fn get_piety(&self, deity: Deity) -> i32 {
        self.piety
            .iter()
            .find(|(d, _)| *d == deity)
            .map(|(_, p)| *p)
            .unwrap_or(0)
    }

    pub fn add_piety(&mut self, deity: Deity, amount: i32) {
        if let Some((_, p)) = self.piety.iter_mut().find(|(d, _)| *d == deity) {
            *p += amount;
        } else {
            self.piety.push((deity, amount));
        }
    }

    pub fn highest_deity(&self) -> Option<Deity> {
        self.piety
            .iter()
            .filter(|&&(_, p)| p > 0)
            .max_by_key(|&&(_, p)| p)
            .map(|&(d, _)| d)
    }

    pub fn devotion_bonus(&self, deity: Deity) -> &'static str {
        let p = self.get_piety(deity);
        if p >= 15 {
            match deity {
                Deity::Jade => "Major: +1 HP on kill",
                Deity::Iron => "Major: +1 bonus damage",
                Deity::Gold => "Major: +3 bonus gold on kill",
                Deity::Gale => "Major: 15% evade on wrong answer",
                Deity::Mirror => "Major: Show pinyin on wrong answer",
            }
        } else if p >= 10 {
            match deity {
                Deity::Jade => "Moderate: +1 HP on kill",
                Deity::Iron => "Moderate: +1 bonus damage",
                Deity::Gold => "Moderate: +3 bonus gold on kill",
                Deity::Gale => "Moderate: 15% evade on wrong answer",
                Deity::Mirror => "Moderate: Show pinyin on wrong answer",
            }
        } else if p >= 5 {
            "Minor devotion"
        } else {
            "None"
        }
    }

    pub fn deity_synergy(&self) -> Option<(&'static str, &'static str)> {
        let p = |d| self.get_piety(d) >= 10;
        if p(Deity::Jade) && p(Deity::Iron) {
            Some(("Paladin's Vigor", "Heal 1 HP per kill AND +1 damage"))
        } else if p(Deity::Jade) && p(Deity::Gold) {
            Some(("Merchant's Blessing", "+5 gold per floor cleared"))
        } else if p(Deity::Mirror) && p(Deity::Gale) {
            Some(("Scholar's Wind", "Reveal map on floor entry (25% chance)"))
        } else if p(Deity::Iron) && p(Deity::Gold) {
            Some(("Warlord's Tithe", "Enemies drop double gold"))
        } else if p(Deity::Mirror) && p(Deity::Iron) {
            Some(("Tactical Insight", "+2 bonus damage to elites"))
        } else if p(Deity::Gale) && p(Deity::Gold) {
            Some(("Fortune's Breeze", "25% chance for extra item on floor"))
        } else {
            None
        }
    }

    pub fn set_form(&mut self, form: PlayerForm, duration: i32) {
        self.form = form;
        self.form_timer = duration;
    }

    pub fn tick_form(&mut self) {
        if self.form_timer > 0 {
            self.form_timer -= 1;
            if self.form_timer == 0 {
                self.form = PlayerForm::Human;
            }
        }
    }

    pub fn apply_meta_progression(
        &mut self,
        starting_hp_bonus: i32,
        shop_discount_pct: i32,
        spell_power_bonus: i32,
    ) {
        if starting_hp_bonus > 0 {
            self.max_hp += starting_hp_bonus;
            self.hp += starting_hp_bonus;
        }
        self.shop_discount_pct = shop_discount_pct.max(0);
        self.spell_power_bonus = spell_power_bonus.max(0);
    }

    /// Max items depends on class
    pub fn max_items(&self) -> usize {
        match self.class {
            PlayerClass::Alchemist => 7,
            PlayerClass::Warrior => 4,
            PlayerClass::Scholar => 5,
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
        if self.items.len() < self.max_items() {
            self.items.push(item);
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
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
        self.enchantments
            .iter()
            .filter_map(|e| *e)
            .map(|r| match r {
                "力" | "火" => 1,
                _ => 0,
            })
            .sum()
    }

    /// Bonus damage reduction from enchantments (水=+1, 土=+1)
    pub fn enchant_damage_reduction(&self) -> i32 {
        self.enchantments
            .iter()
            .filter_map(|e| *e)
            .map(|r| match r {
                "水" | "土" => 1,
                _ => 0,
            })
            .sum()
    }

    /// Bonus max HP from enchantments (心=+2)
    #[allow(dead_code)]
    pub fn enchant_max_hp_bonus(&self) -> i32 {
        self.enchantments
            .iter()
            .filter_map(|e| *e)
            .map(|r| match r {
                "心" => 2,
                _ => 0,
            })
            .sum()
    }

    /// Bonus gold from enchantments (金=+3)
    pub fn enchant_gold_bonus(&self) -> i32 {
        self.enchantments
            .iter()
            .filter_map(|e| *e)
            .map(|r| match r {
                "金" => 3,
                _ => 0,
            })
            .sum()
    }

    /// Bonus FOV from enchantments (目=+1)
    pub fn enchant_fov_bonus(&self) -> i32 {
        self.enchantments
            .iter()
            .filter_map(|e| *e)
            .map(|r| match r {
                "目" => 1,
                _ => 0,
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::{Deity, Item, ItemKind, Player, PlayerClass};

    #[test]
    fn item_kind_matches_variant() {
        assert_eq!(Item::HealthPotion(5).kind(), ItemKind::HealthPotion);
        assert_eq!(Item::TeleportScroll.kind(), ItemKind::TeleportScroll);
    }

    #[test]
    fn item_display_name_uses_mystery_label_until_identified() {
        let item = Item::RevealScroll;

        assert_eq!(
            item.display_name(false, "Cloud Seal 云符"),
            "? Cloud Seal 云符"
        );
        assert_eq!(
            item.display_name(true, "Cloud Seal 云符"),
            "👁 Reveal Scroll"
        );
    }

    #[test]
    fn deity_synergy_requires_dual_devotion() {
        let mut player = Player::new(0, 0, PlayerClass::Warrior);
        player.add_piety(Deity::Jade, 10);
        player.add_piety(Deity::Iron, 10);
        assert_eq!(
            player.deity_synergy(),
            Some(("Paladin's Vigor", "Heal 1 HP per kill AND +1 damage"))
        );
    }

    #[test]
    fn deity_synergy_returns_none_without_threshold() {
        let mut player = Player::new(0, 0, PlayerClass::Warrior);
        player.add_piety(Deity::Jade, 9);
        player.add_piety(Deity::Iron, 10);
        assert_eq!(player.deity_synergy(), None);
    }

    #[test]
    fn devotion_bonus_tiers() {
        let mut player = Player::new(0, 0, PlayerClass::Warrior);
        assert_eq!(player.devotion_bonus(Deity::Jade), "None");

        player.add_piety(Deity::Jade, 5);
        assert_eq!(player.devotion_bonus(Deity::Jade), "Minor devotion");

        player.add_piety(Deity::Jade, 5);
        assert_eq!(
            player.devotion_bonus(Deity::Jade),
            "Moderate: +1 HP on kill"
        );

        player.add_piety(Deity::Jade, 5);
        assert_eq!(player.devotion_bonus(Deity::Jade), "Major: +1 HP on kill");
    }
}
