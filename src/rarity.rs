use crate::player::EquipSlot;

// ---------------------------------------------------------------------------
// Rarity tiers
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemRarity {
    Normal,
    Magic,
    Rare,
    Unique,
}

impl ItemRarity {
    pub fn color(&self) -> &'static str {
        match self {
            ItemRarity::Normal => "#cccccc",
            ItemRarity::Magic  => "#4488ff",
            ItemRarity::Rare   => "#ffdd44",
            ItemRarity::Unique => "#ff8800",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ItemRarity::Normal => "Normal",
            ItemRarity::Magic  => "Magic",
            ItemRarity::Rare   => "Rare",
            ItemRarity::Unique => "Unique",
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_json(&self) -> String {
        format!("\"{}\"", self.label())
    }

    pub fn from_json(s: &str) -> Self {
        let trimmed = s.trim().trim_matches('"');
        match trimmed {
            "Magic"  => ItemRarity::Magic,
            "Rare"   => ItemRarity::Rare,
            "Unique" => ItemRarity::Unique,
            _        => ItemRarity::Normal,
        }
    }
}

// ---------------------------------------------------------------------------
// Affix system
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AffixEffect {
    BonusDamage(i32),
    BonusArmor(i32),
    MaxHp(i32),
    SpellPower(i32),
    CritChance(i32),
    LifeSteal(i32),
    FocusRegen(i32),
    DodgeChance(i32),
    GoldFind(i32),
    RadicalFind(i32),
    HardAnswerDamage(i32),
    DamageReduction(i32),
    MovementBonus(i32),
}

#[derive(Clone, Copy, Debug)]
pub struct Affix {
    pub name: &'static str,
    pub effect: AffixEffect,
    pub is_prefix: bool,
}

/// An affix instance rolled onto a piece of equipment.
#[derive(Clone, Debug)]
pub struct RolledAffix {
    pub affix: &'static Affix,
}

// ---------------------------------------------------------------------------
// Static affix pools
// ---------------------------------------------------------------------------

pub static PREFIX_POOL: &[Affix] = &[
    Affix { name: "Sharpened",  effect: AffixEffect::BonusDamage(1),    is_prefix: true },
    Affix { name: "Tempered",   effect: AffixEffect::BonusDamage(2),    is_prefix: true },
    Affix { name: "Deadly",     effect: AffixEffect::BonusDamage(3),    is_prefix: true },
    Affix { name: "Hardened",   effect: AffixEffect::BonusArmor(1),     is_prefix: true },
    Affix { name: "Fortified",  effect: AffixEffect::BonusArmor(2),     is_prefix: true },
    Affix { name: "Sturdy",     effect: AffixEffect::MaxHp(3),          is_prefix: true },
    Affix { name: "Robust",     effect: AffixEffect::MaxHp(5),          is_prefix: true },
    Affix { name: "Swift",      effect: AffixEffect::MovementBonus(1),  is_prefix: true },
    Affix { name: "Learned",    effect: AffixEffect::SpellPower(1),     is_prefix: true },
    Affix { name: "Brilliant",  effect: AffixEffect::SpellPower(2),     is_prefix: true },
    Affix { name: "Lucky",      effect: AffixEffect::GoldFind(15),      is_prefix: true },
    Affix { name: "Prosperous", effect: AffixEffect::GoldFind(25),      is_prefix: true },
    Affix { name: "Precise",   effect: AffixEffect::CritChance(8),       is_prefix: true },
    Affix { name: "Resonant",  effect: AffixEffect::SpellPower(3),       is_prefix: true },
    Affix { name: "Vampiric",  effect: AffixEffect::LifeSteal(1),        is_prefix: true },
    Affix { name: "Armored",   effect: AffixEffect::DamageReduction(1),  is_prefix: true },
];

pub static SUFFIX_POOL: &[Affix] = &[
    Affix { name: "of Precision",     effect: AffixEffect::CritChance(10),       is_prefix: false },
    Affix { name: "of Destruction",   effect: AffixEffect::CritChance(15),       is_prefix: false },
    Affix { name: "of Draining",      effect: AffixEffect::LifeSteal(1),         is_prefix: false },
    Affix { name: "of Vampirism",     effect: AffixEffect::LifeSteal(2),         is_prefix: false },
    Affix { name: "of Focus",         effect: AffixEffect::FocusRegen(1),        is_prefix: false },
    Affix { name: "of Concentration", effect: AffixEffect::FocusRegen(2),        is_prefix: false },
    Affix { name: "of Evasion",       effect: AffixEffect::DodgeChance(5),       is_prefix: false },
    Affix { name: "of Agility",       effect: AffixEffect::DodgeChance(10),      is_prefix: false },
    Affix { name: "of the Scholar",   effect: AffixEffect::HardAnswerDamage(1),  is_prefix: false },
    Affix { name: "of Mastery",       effect: AffixEffect::HardAnswerDamage(2),  is_prefix: false },
    Affix { name: "of Warding",       effect: AffixEffect::DamageReduction(1),   is_prefix: false },
    Affix { name: "of Shielding",     effect: AffixEffect::DamageReduction(2),   is_prefix: false },
    Affix { name: "of Radicals",      effect: AffixEffect::RadicalFind(15),      is_prefix: false },
    Affix { name: "of Discovery",     effect: AffixEffect::RadicalFind(25),      is_prefix: false },
    Affix { name: "of Power",        effect: AffixEffect::BonusDamage(2),    is_prefix: false },
    Affix { name: "of the Bulwark",  effect: AffixEffect::BonusArmor(2),     is_prefix: false },
    Affix { name: "of Vitality",     effect: AffixEffect::MaxHp(7),          is_prefix: false },
    Affix { name: "of Swiftness",    effect: AffixEffect::MovementBonus(1),  is_prefix: false },
];

// ---------------------------------------------------------------------------
// Simple splitmix64-style PRNG helpers (deterministic, no external deps)
// ---------------------------------------------------------------------------

fn splitmix(seed: u64) -> (u64, u64) {
    let s = seed.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = s;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z = z ^ (z >> 31);
    (z, s)
}

fn pick_index(seed: u64, len: usize) -> usize {
    (seed as usize) % len
}

// ---------------------------------------------------------------------------
// Rolling functions
// ---------------------------------------------------------------------------

/// Roll affixes for the given rarity using a simple PRNG seed.
pub fn roll_affixes(rarity: ItemRarity, rng: u64) -> Vec<RolledAffix> {
    match rarity {
        ItemRarity::Normal | ItemRarity::Unique => vec![],

        ItemRarity::Magic => {
            // 1-2 affixes: always at least one, 50 % chance of a second
            let (r1, s1) = splitmix(rng);
            let (r2, s2) = splitmix(s1);
            let (r3, _s3) = splitmix(s2);

            let has_two = r1 % 2 == 0;
            let mut out = Vec::with_capacity(2);

            // First affix — coin flip prefix vs suffix
            if r2 % 2 == 0 {
                let idx = pick_index(r2, PREFIX_POOL.len());
                out.push(RolledAffix { affix: &PREFIX_POOL[idx] });
            } else {
                let idx = pick_index(r2, SUFFIX_POOL.len());
                out.push(RolledAffix { affix: &SUFFIX_POOL[idx] });
            }

            if has_two {
                // Second affix — opposite pool from first to avoid same-pool dupes
                if out[0].affix.is_prefix {
                    let idx = pick_index(r3, SUFFIX_POOL.len());
                    out.push(RolledAffix { affix: &SUFFIX_POOL[idx] });
                } else {
                    let idx = pick_index(r3, PREFIX_POOL.len());
                    out.push(RolledAffix { affix: &PREFIX_POOL[idx] });
                }
            }
            out
        }

        ItemRarity::Rare => {
            // 3-4 affixes: 1-2 prefixes + 1-2 suffixes, total 3 or 4
            let (r0, s0) = splitmix(rng);
            let (r1, s1) = splitmix(s0);
            let (r2, s2) = splitmix(s1);
            let (r3, s3) = splitmix(s2);
            let (r4, _s4) = splitmix(s3);

            let total = if r0 % 2 == 0 { 4 } else { 3 };
            let n_prefix = if r1 % 2 == 0 { 2 } else { 1 };
            let n_suffix = total - n_prefix;

            let mut out = Vec::with_capacity(total);

            // Pick distinct prefixes
            let idx1 = pick_index(r2, PREFIX_POOL.len());
            out.push(RolledAffix { affix: &PREFIX_POOL[idx1] });
            if n_prefix == 2 {
                let mut idx2 = pick_index(r3, PREFIX_POOL.len());
                if idx2 == idx1 {
                    idx2 = (idx2 + 1) % PREFIX_POOL.len();
                }
                out.push(RolledAffix { affix: &PREFIX_POOL[idx2] });
            }

            // Pick distinct suffixes
            let suffix_seed = if n_prefix == 2 { r4 } else { r3 };
            let (rs1, ss1) = splitmix(suffix_seed);
            let sidx1 = pick_index(rs1, SUFFIX_POOL.len());
            out.push(RolledAffix { affix: &SUFFIX_POOL[sidx1] });
            if n_suffix >= 2 {
                let (rs2, _) = splitmix(ss1);
                let mut sidx2 = pick_index(rs2, SUFFIX_POOL.len());
                if sidx2 == sidx1 {
                    sidx2 = (sidx2 + 1) % SUFFIX_POOL.len();
                }
                out.push(RolledAffix { affix: &SUFFIX_POOL[sidx2] });
            }
            out
        }
    }
}

/// Determine rarity for a drop based on floor depth and luck modifier.
pub fn roll_rarity(floor: i32, luck_bonus: i32, rng: u64) -> ItemRarity {
    let roll = (rng % 1000) as i32;
    let unique_threshold = 5 + luck_bonus;
    let rare_threshold = 50 + floor * 3 + luck_bonus * 2;
    let magic_threshold = 200 + floor * 5 + luck_bonus * 3;

    if roll < unique_threshold {
        ItemRarity::Unique
    } else if roll < rare_threshold {
        ItemRarity::Rare
    } else if roll < magic_threshold {
        ItemRarity::Magic
    } else {
        ItemRarity::Normal
    }
}

/// Generate a display name for equipment with rarity and rolled affixes.
pub fn rarity_name(base_name: &str, rarity: ItemRarity, affixes: &[RolledAffix]) -> String {
    match rarity {
        ItemRarity::Normal => base_name.to_string(),
        ItemRarity::Unique => base_name.to_string(), // uniques use their own name
        ItemRarity::Magic => {
            let prefix_part = affixes.iter().find(|a| a.affix.is_prefix).map(|a| a.affix.name);
            let suffix_part = affixes.iter().find(|a| !a.affix.is_prefix).map(|a| a.affix.name);
            match (prefix_part, suffix_part) {
                (Some(p), Some(s)) => format!("{} {} {}", p, base_name, s),
                (Some(p), None)    => format!("{} {}", p, base_name),
                (None, Some(s))    => format!("{} {}", base_name, s),
                (None, None)       => base_name.to_string(),
            }
        }
        ItemRarity::Rare => {
            let prefix_part = affixes.iter().find(|a| a.affix.is_prefix).map(|a| a.affix.name);
            let suffix_part = affixes.iter().find(|a| !a.affix.is_prefix).map(|a| a.affix.name);
            match (prefix_part, suffix_part) {
                (Some(p), Some(s)) => format!("{} {} {}", p, base_name, s),
                (Some(p), None)    => format!("{} {}", p, base_name),
                (None, Some(s))    => format!("{} {}", base_name, s),
                (None, None)       => base_name.to_string(),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Unique equipment
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UniqueEffect {
    BonusDamage(i32),
    BonusArmor(i32),
    MaxHp(i32),
    SpellPower(i32),
    DamageScalesWithMissingHp,
    AllAnswersHard,
    StealBuffOnKill,
    DamagePerCodexEntry(i32),
    DoubleComboGain,
    HealOnWrongAnswer(i32),
    GoldOnDamageTaken(i32),
    SpellsAreFree,
    IgnoreArmor,
    DoubleRadicalDrops,
    EnemiesBurnOnSight,
    CritAlwaysOnHardAnswer,
    FirstStrikeBonusDamage(i32),  // bonus damage on first attack each combat
    HealOnCombo(i32),             // heal N HP when combo reaches multiple of 5
    DamagePerFloor,               // +1 damage per 5 floors cleared
    SpellEcho,                    // 25% chance spells fire twice
}

pub struct UniqueEquipment {
    pub name: &'static str,
    pub base_slot: EquipSlot,
    #[allow(dead_code)]
    pub lore: &'static str,
    #[allow(dead_code)]
    pub effects: &'static [UniqueEffect],
}

pub static UNIQUE_POOL: &[UniqueEquipment] = &[
    UniqueEquipment {
        name: "Quantum Paradox Blade",
        base_slot: EquipSlot::Weapon,
        lore: "Forged in a collapsing star. Damage increases as your HP decreases.",
        effects: &[UniqueEffect::DamageScalesWithMissingHp, UniqueEffect::BonusDamage(1)],
    },
    UniqueEquipment {
        name: "Scholar's Infinite Codex",
        base_slot: EquipSlot::Charm,
        lore: "Contains every word ever written. All answers are treated as hard.",
        effects: &[UniqueEffect::AllAnswersHard, UniqueEffect::SpellPower(1)],
    },
    UniqueEquipment {
        name: "Drift Leviathan's Fang",
        base_slot: EquipSlot::Weapon,
        lore: "Torn from the final boss. Steals enemy power on kill.",
        effects: &[UniqueEffect::StealBuffOnKill, UniqueEffect::BonusDamage(2)],
    },
    UniqueEquipment {
        name: "The Last Word",
        base_slot: EquipSlot::Weapon,
        lore: "Grows stronger with knowledge. +1 damage per 10 codex entries.",
        effects: &[UniqueEffect::DamagePerCodexEntry(1)],
    },
    UniqueEquipment {
        name: "Echo Chamber",
        base_slot: EquipSlot::Charm,
        lore: "Amplifies linguistic resonance. Combo builds twice as fast.",
        effects: &[UniqueEffect::DoubleComboGain],
    },
    UniqueEquipment {
        name: "Fool's Crown",
        base_slot: EquipSlot::Armor,
        lore: "The wise learn from mistakes. Heal 2 HP on wrong answers but deal no damage.",
        effects: &[UniqueEffect::HealOnWrongAnswer(2), UniqueEffect::MaxHp(5)],
    },
    UniqueEquipment {
        name: "Merchant's Misfortune",
        base_slot: EquipSlot::Armor,
        lore: "Pain is profit. Gain 3 gold each time you take damage.",
        effects: &[UniqueEffect::GoldOnDamageTaken(3), UniqueEffect::BonusArmor(-1)],
    },
    UniqueEquipment {
        name: "Void Conduit",
        base_slot: EquipSlot::Charm,
        lore: "Channels the void. Spells are free but cost 1 HP each.",
        effects: &[UniqueEffect::SpellsAreFree, UniqueEffect::SpellPower(2)],
    },
    UniqueEquipment {
        name: "Phase Disruptor",
        base_slot: EquipSlot::Weapon,
        lore: "Shifts between dimensions. Attacks ignore all enemy armor.",
        effects: &[UniqueEffect::IgnoreArmor],
    },
    UniqueEquipment {
        name: "Radical Harvester",
        base_slot: EquipSlot::Charm,
        lore: "A magnetic array tuned to hanzi radicals. Always double radical drops.",
        effects: &[UniqueEffect::DoubleRadicalDrops],
    },
    UniqueEquipment {
        name: "Pyromaniac's Lens",
        base_slot: EquipSlot::Charm,
        lore: "Everything burns. Enemies take burn damage when first spotted.",
        effects: &[UniqueEffect::EnemiesBurnOnSight],
    },
    UniqueEquipment {
        name: "Sage's Judgment",
        base_slot: EquipSlot::Weapon,
        lore: "Rewards deep knowledge. Hard answers always crit.",
        effects: &[UniqueEffect::CritAlwaysOnHardAnswer, UniqueEffect::BonusDamage(1)],
    },
    UniqueEquipment {
        name: "Chrono Disruptor",
        base_slot: EquipSlot::Weapon,
        lore: "Strikes before the enemy can react. Devastating opening blow.",
        effects: &[UniqueEffect::FirstStrikeBonusDamage(4), UniqueEffect::BonusDamage(1)],
    },
    UniqueEquipment {
        name: "Harmony Matrix",
        base_slot: EquipSlot::Charm,
        lore: "Resonates with linguistic mastery. Heals through correct answers.",
        effects: &[UniqueEffect::HealOnCombo(3), UniqueEffect::SpellPower(1)],
    },
    UniqueEquipment {
        name: "Abyssal Depth Gauge",
        base_slot: EquipSlot::Weapon,
        lore: "Grows stronger the deeper you venture. A weapon for true explorers.",
        effects: &[UniqueEffect::DamagePerFloor, UniqueEffect::MaxHp(5)],
    },
    UniqueEquipment {
        name: "Resonance Engine",
        base_slot: EquipSlot::Charm,
        lore: "Echoes of ancient knowledge sometimes ripple outward twice.",
        effects: &[UniqueEffect::SpellEcho, UniqueEffect::SpellPower(2)],
    },
];

/// Look up a unique equipment definition by name.
#[allow(dead_code)]
pub fn find_unique(name: &str) -> Option<&'static UniqueEquipment> {
    UNIQUE_POOL.iter().find(|u| u.name == name)
}

/// Pick a random unique from the pool.
pub fn roll_unique(rng: u64) -> &'static UniqueEquipment {
    let idx = pick_index(rng, UNIQUE_POOL.len());
    &UNIQUE_POOL[idx]
}

// ---------------------------------------------------------------------------
// Aggregate helpers — sum specific affix effects
// ---------------------------------------------------------------------------

fn sum_affix(affixes: &[RolledAffix], f: impl Fn(&AffixEffect) -> i32) -> i32 {
    affixes.iter().map(|a| f(&a.affix.effect)).sum()
}

pub fn total_affix_damage(affixes: &[RolledAffix]) -> i32 {
    sum_affix(affixes, |e| match e { AffixEffect::BonusDamage(v) => *v, _ => 0 })
}

pub fn total_affix_armor(affixes: &[RolledAffix]) -> i32 {
    sum_affix(affixes, |e| match e { AffixEffect::BonusArmor(v) => *v, _ => 0 })
}

pub fn total_affix_max_hp(affixes: &[RolledAffix]) -> i32 {
    sum_affix(affixes, |e| match e { AffixEffect::MaxHp(v) => *v, _ => 0 })
}

pub fn total_affix_crit(affixes: &[RolledAffix]) -> i32 {
    sum_affix(affixes, |e| match e { AffixEffect::CritChance(v) => *v, _ => 0 })
}

#[allow(dead_code)]
pub fn total_affix_spell_power(affixes: &[RolledAffix]) -> i32 {
    sum_affix(affixes, |e| match e { AffixEffect::SpellPower(v) => *v, _ => 0 })
}

#[allow(dead_code)]
pub fn total_affix_lifesteal(affixes: &[RolledAffix]) -> i32 {
    sum_affix(affixes, |e| match e { AffixEffect::LifeSteal(v) => *v, _ => 0 })
}

#[allow(dead_code)]
pub fn total_affix_focus_regen(affixes: &[RolledAffix]) -> i32 {
    sum_affix(affixes, |e| match e { AffixEffect::FocusRegen(v) => *v, _ => 0 })
}

pub fn total_affix_dodge(affixes: &[RolledAffix]) -> i32 {
    sum_affix(affixes, |e| match e { AffixEffect::DodgeChance(v) => *v, _ => 0 })
}

pub fn total_affix_gold_find(affixes: &[RolledAffix]) -> i32 {
    sum_affix(affixes, |e| match e { AffixEffect::GoldFind(v) => *v, _ => 0 })
}

pub fn total_affix_radical_find(affixes: &[RolledAffix]) -> i32 {
    sum_affix(affixes, |e| match e { AffixEffect::RadicalFind(v) => *v, _ => 0 })
}

pub fn total_affix_hard_answer_damage(affixes: &[RolledAffix]) -> i32 {
    sum_affix(affixes, |e| match e { AffixEffect::HardAnswerDamage(v) => *v, _ => 0 })
}

pub fn total_affix_damage_reduction(affixes: &[RolledAffix]) -> i32 {
    sum_affix(affixes, |e| match e { AffixEffect::DamageReduction(v) => *v, _ => 0 })
}

// ---------------------------------------------------------------------------
// JSON serialization (manual, matching project convention)
// ---------------------------------------------------------------------------

impl AffixEffect {
    /// Human-readable description of the affix effect (e.g. "+2 damage").
    pub fn describe(&self) -> String {
        match self {
            AffixEffect::BonusDamage(v)      => format!("+{} damage", v),
            AffixEffect::BonusArmor(v)       => format!("+{} armor", v),
            AffixEffect::MaxHp(v)            => format!("+{} max HP", v),
            AffixEffect::SpellPower(v)       => format!("+{} spell power", v),
            AffixEffect::CritChance(v)       => format!("+{}% crit", v),
            AffixEffect::LifeSteal(v)        => format!("+{} life steal", v),
            AffixEffect::FocusRegen(v)       => format!("+{} focus regen", v),
            AffixEffect::DodgeChance(v)      => format!("+{}% dodge", v),
            AffixEffect::GoldFind(v)         => format!("+{}% gold find", v),
            AffixEffect::RadicalFind(v)      => format!("+{}% radical find", v),
            AffixEffect::HardAnswerDamage(v) => format!("+{} hard answer dmg", v),
            AffixEffect::DamageReduction(v)  => format!("-{} damage taken", v),
            AffixEffect::MovementBonus(v)    => format!("+{} movement", v),
        }
    }

    fn variant_name(&self) -> &'static str {
        match self {
            AffixEffect::BonusDamage(_)      => "BonusDamage",
            AffixEffect::BonusArmor(_)       => "BonusArmor",
            AffixEffect::MaxHp(_)            => "MaxHp",
            AffixEffect::SpellPower(_)       => "SpellPower",
            AffixEffect::CritChance(_)       => "CritChance",
            AffixEffect::LifeSteal(_)        => "LifeSteal",
            AffixEffect::FocusRegen(_)       => "FocusRegen",
            AffixEffect::DodgeChance(_)      => "DodgeChance",
            AffixEffect::GoldFind(_)         => "GoldFind",
            AffixEffect::RadicalFind(_)      => "RadicalFind",
            AffixEffect::HardAnswerDamage(_) => "HardAnswerDamage",
            AffixEffect::DamageReduction(_)  => "DamageReduction",
            AffixEffect::MovementBonus(_)    => "MovementBonus",
        }
    }

    fn value(&self) -> i32 {
        match self {
            AffixEffect::BonusDamage(v)
            | AffixEffect::BonusArmor(v)
            | AffixEffect::MaxHp(v)
            | AffixEffect::SpellPower(v)
            | AffixEffect::CritChance(v)
            | AffixEffect::LifeSteal(v)
            | AffixEffect::FocusRegen(v)
            | AffixEffect::DodgeChance(v)
            | AffixEffect::GoldFind(v)
            | AffixEffect::RadicalFind(v)
            | AffixEffect::HardAnswerDamage(v)
            | AffixEffect::DamageReduction(v)
            | AffixEffect::MovementBonus(v) => *v,
        }
    }
}

/// Find a static Affix reference by name in PREFIX_POOL or SUFFIX_POOL.
fn find_affix_by_name(name: &str) -> Option<&'static Affix> {
    PREFIX_POOL
        .iter()
        .chain(SUFFIX_POOL.iter())
        .find(|a| a.name == name)
}

pub fn affixes_to_json(affixes: &[RolledAffix]) -> String {
    let mut s = String::from("[");
    for (i, ra) in affixes.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "{{\"name\":\"{}\",\"effect\":\"{}\",\"value\":{}}}",
            ra.affix.name,
            ra.affix.effect.variant_name(),
            ra.affix.effect.value(),
        ));
    }
    s.push(']');
    s
}

pub fn affixes_from_json(json: &str) -> Vec<RolledAffix> {
    let trimmed = json.trim();
    if trimmed == "[]" || trimmed.is_empty() {
        return vec![];
    }
    let inner = &trimmed[1..trimmed.len() - 1]; // strip outer []
    let mut result = Vec::new();
    // Split on },{ boundaries
    for obj_str in split_json_objects(inner) {
        if let Some(name) = extract_json_string_field(obj_str, "name") {
            if let Some(affix) = find_affix_by_name(&name) {
                result.push(RolledAffix { affix });
            }
        }
    }
    result
}

/// Split a string of JSON objects (without outer brackets) into individual object strings.
fn split_json_objects(s: &str) -> Vec<&str> {
    let mut objects = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '{' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    objects.push(&s[start..=i]);
                }
            }
            _ => {}
        }
    }
    objects
}

/// Extract a string-valued field from a JSON object string.
fn extract_json_string_field(obj: &str, field: &str) -> Option<String> {
    let key = format!("\"{}\":\"", field);
    if let Some(start) = obj.find(&key) {
        let val_start = start + key.len();
        if let Some(end) = obj[val_start..].find('"') {
            return Some(obj[val_start..val_start + end].to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rarity_distribution_normal_most_common() {
        // Floor 1, no luck — Normal should dominate
        let mut counts = [0u32; 4];
        for seed in 0..1000u64 {
            match roll_rarity(1, 0, seed) {
                ItemRarity::Normal => counts[0] += 1,
                ItemRarity::Magic  => counts[1] += 1,
                ItemRarity::Rare   => counts[2] += 1,
                ItemRarity::Unique => counts[3] += 1,
            }
        }
        // Normal should be most common
        assert!(counts[0] > counts[1], "Normal ({}) should exceed Magic ({})", counts[0], counts[1]);
        assert!(counts[1] > counts[2], "Magic ({}) should exceed Rare ({})", counts[1], counts[2]);
        assert!(counts[2] > counts[3], "Rare ({}) should exceed Unique ({})", counts[2], counts[3]);
    }

    #[test]
    fn rarity_unique_is_rarest() {
        let mut unique_count = 0u32;
        for seed in 0..10_000u64 {
            if roll_rarity(1, 0, seed) == ItemRarity::Unique {
                unique_count += 1;
            }
        }
        // With threshold 5+0 = 5 out of 1000, expect ~0.5%
        assert!(unique_count < 200, "Unique should be rare, got {}", unique_count);
    }

    #[test]
    fn normal_has_no_affixes() {
        let affixes = roll_affixes(ItemRarity::Normal, 12345);
        assert!(affixes.is_empty());
    }

    #[test]
    fn unique_has_no_random_affixes() {
        let affixes = roll_affixes(ItemRarity::Unique, 12345);
        assert!(affixes.is_empty());
    }

    #[test]
    fn magic_has_one_or_two_affixes() {
        for seed in 0..100u64 {
            let affixes = roll_affixes(ItemRarity::Magic, seed);
            assert!(
                affixes.len() == 1 || affixes.len() == 2,
                "Magic item should have 1-2 affixes, got {}",
                affixes.len()
            );
        }
    }

    #[test]
    fn rare_has_three_or_four_affixes() {
        for seed in 0..100u64 {
            let affixes = roll_affixes(ItemRarity::Rare, seed);
            assert!(
                affixes.len() == 3 || affixes.len() == 4,
                "Rare item should have 3-4 affixes, got {}",
                affixes.len()
            );
        }
    }

    #[test]
    fn name_generation_normal() {
        let name = rarity_name("Laser Pistol", ItemRarity::Normal, &[]);
        assert_eq!(name, "Laser Pistol");
    }

    #[test]
    fn name_generation_magic_prefix() {
        let affix = RolledAffix { affix: &PREFIX_POOL[2] }; // "Deadly"
        let name = rarity_name("Laser Pistol", ItemRarity::Magic, &[affix]);
        assert_eq!(name, "Deadly Laser Pistol");
    }

    #[test]
    fn name_generation_magic_suffix() {
        let affix = RolledAffix { affix: &SUFFIX_POOL[3] }; // "of Vampirism"
        let name = rarity_name("Laser Pistol", ItemRarity::Magic, &[affix]);
        assert_eq!(name, "Laser Pistol of Vampirism");
    }

    #[test]
    fn name_generation_rare_full() {
        let affixes = vec![
            RolledAffix { affix: &PREFIX_POOL[2] },  // "Deadly"
            RolledAffix { affix: &SUFFIX_POOL[3] },  // "of Vampirism"
            RolledAffix { affix: &SUFFIX_POOL[0] },  // "of Precision"
        ];
        let name = rarity_name("Laser Pistol", ItemRarity::Rare, &affixes);
        assert_eq!(name, "Deadly Laser Pistol of Vampirism");
    }

    #[test]
    fn unique_lookup() {
        let u = find_unique("Quantum Paradox Blade");
        assert!(u.is_some());
        let u = u.unwrap();
        assert_eq!(u.base_slot, EquipSlot::Weapon);
        assert!(u.effects.contains(&UniqueEffect::DamageScalesWithMissingHp));
    }

    #[test]
    fn unique_lookup_missing() {
        assert!(find_unique("Nonexistent Sword").is_none());
    }

    #[test]
    fn unique_pool_has_sixteen_items() {
        assert_eq!(UNIQUE_POOL.len(), 16);
    }

    #[test]
    fn aggregate_damage() {
        let affixes = vec![
            RolledAffix { affix: &PREFIX_POOL[0] },  // BonusDamage(1)
            RolledAffix { affix: &PREFIX_POOL[2] },  // BonusDamage(3)
            RolledAffix { affix: &SUFFIX_POOL[0] },  // CritChance(10)
        ];
        assert_eq!(total_affix_damage(&affixes), 4);
        assert_eq!(total_affix_crit(&affixes), 10);
        assert_eq!(total_affix_armor(&affixes), 0);
    }

    #[test]
    fn aggregate_mixed() {
        let affixes = vec![
            RolledAffix { affix: &PREFIX_POOL[3] },  // BonusArmor(1)
            RolledAffix { affix: &PREFIX_POOL[5] },  // MaxHp(3)
            RolledAffix { affix: &SUFFIX_POOL[2] },  // LifeSteal(1)
            RolledAffix { affix: &SUFFIX_POOL[6] },  // DodgeChance(5)
        ];
        assert_eq!(total_affix_armor(&affixes), 1);
        assert_eq!(total_affix_max_hp(&affixes), 3);
        assert_eq!(total_affix_lifesteal(&affixes), 1);
        assert_eq!(total_affix_dodge(&affixes), 5);
        assert_eq!(total_affix_damage(&affixes), 0);
    }

    #[test]
    fn json_roundtrip_rarity() {
        for rarity in &[ItemRarity::Normal, ItemRarity::Magic, ItemRarity::Rare, ItemRarity::Unique] {
            let json = rarity.to_json();
            let parsed = ItemRarity::from_json(&json);
            assert_eq!(*rarity, parsed);
        }
    }

    #[test]
    fn json_roundtrip_affixes() {
        let affixes = vec![
            RolledAffix { affix: &PREFIX_POOL[0] },
            RolledAffix { affix: &SUFFIX_POOL[1] },
        ];
        let json = affixes_to_json(&affixes);
        let parsed = affixes_from_json(&json);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].affix.name, "Sharpened");
        assert_eq!(parsed[1].affix.name, "of Destruction");
    }

    #[test]
    fn json_roundtrip_empty_affixes() {
        let json = affixes_to_json(&[]);
        assert_eq!(json, "[]");
        let parsed = affixes_from_json(&json);
        assert!(parsed.is_empty());
    }

    #[test]
    fn rarity_colors() {
        assert_eq!(ItemRarity::Normal.color(), "#cccccc");
        assert_eq!(ItemRarity::Magic.color(), "#4488ff");
        assert_eq!(ItemRarity::Rare.color(), "#ffdd44");
        assert_eq!(ItemRarity::Unique.color(), "#ff8800");
    }

    #[test]
    fn rarity_labels() {
        assert_eq!(ItemRarity::Normal.label(), "Normal");
        assert_eq!(ItemRarity::Magic.label(), "Magic");
        assert_eq!(ItemRarity::Rare.label(), "Rare");
        assert_eq!(ItemRarity::Unique.label(), "Unique");
    }

    #[test]
    fn rarity_to_json_format() {
        assert_eq!(ItemRarity::Normal.to_json(), "\"Normal\"");
        assert_eq!(ItemRarity::Magic.to_json(), "\"Magic\"");
        assert_eq!(ItemRarity::Rare.to_json(), "\"Rare\"");
        assert_eq!(ItemRarity::Unique.to_json(), "\"Unique\"");
    }

    #[test]
    fn rarity_from_json_all_variants() {
        assert_eq!(ItemRarity::from_json("\"Normal\""), ItemRarity::Normal);
        assert_eq!(ItemRarity::from_json("\"Magic\""), ItemRarity::Magic);
        assert_eq!(ItemRarity::from_json("\"Rare\""), ItemRarity::Rare);
        assert_eq!(ItemRarity::from_json("\"Unique\""), ItemRarity::Unique);
    }

    #[test]
    fn rarity_from_json_unknown_defaults_to_normal() {
        assert_eq!(ItemRarity::from_json("\"Unknown\""), ItemRarity::Normal);
        assert_eq!(ItemRarity::from_json("\"\""), ItemRarity::Normal);
        assert_eq!(ItemRarity::from_json("garbage"), ItemRarity::Normal);
    }

    #[test]
    fn rarity_from_json_with_whitespace() {
        assert_eq!(ItemRarity::from_json("  \"Magic\"  "), ItemRarity::Magic);
    }

    #[test]
    fn roll_unique_returns_valid_item() {
        for seed in 0..20u64 {
            let u = roll_unique(seed);
            assert!(!u.name.is_empty());
            assert!(!u.effects.is_empty());
        }
    }

    // ── AffixEffect::describe ─────────────────────────────────────

    #[test]
    fn affix_effect_describe_all_variants() {
        assert_eq!(AffixEffect::BonusDamage(2).describe(), "+2 damage");
        assert_eq!(AffixEffect::BonusArmor(3).describe(), "+3 armor");
        assert_eq!(AffixEffect::MaxHp(5).describe(), "+5 max HP");
        assert_eq!(AffixEffect::SpellPower(1).describe(), "+1 spell power");
        assert_eq!(AffixEffect::CritChance(10).describe(), "+10% crit");
        assert_eq!(AffixEffect::LifeSteal(2).describe(), "+2 life steal");
        assert_eq!(AffixEffect::FocusRegen(1).describe(), "+1 focus regen");
        assert_eq!(AffixEffect::DodgeChance(5).describe(), "+5% dodge");
        assert_eq!(AffixEffect::GoldFind(15).describe(), "+15% gold find");
        assert_eq!(AffixEffect::RadicalFind(25).describe(), "+25% radical find");
        assert_eq!(AffixEffect::HardAnswerDamage(1).describe(), "+1 hard answer dmg");
        assert_eq!(AffixEffect::DamageReduction(2).describe(), "-2 damage taken");
        assert_eq!(AffixEffect::MovementBonus(1).describe(), "+1 movement");
    }

    // ── Affix pools ───────────────────────────────────────────────

    #[test]
    fn prefix_pool_count() {
        assert_eq!(PREFIX_POOL.len(), 16);
    }

    #[test]
    fn suffix_pool_count() {
        assert_eq!(SUFFIX_POOL.len(), 18);
    }

    #[test]
    fn all_prefixes_have_is_prefix_true() {
        for affix in PREFIX_POOL {
            assert!(affix.is_prefix, "prefix {} has is_prefix=false", affix.name);
        }
    }

    #[test]
    fn all_suffixes_have_is_prefix_false() {
        for affix in SUFFIX_POOL {
            assert!(!affix.is_prefix, "suffix {} has is_prefix=true", affix.name);
        }
    }

    #[test]
    fn affix_names_are_non_empty() {
        for affix in PREFIX_POOL.iter().chain(SUFFIX_POOL.iter()) {
            assert!(!affix.name.is_empty());
        }
    }

    // ── roll_rarity thresholds ────────────────────────────────────

    #[test]
    fn roll_rarity_very_low_roll_is_unique() {
        // rng % 1000 = 0 should be < unique_threshold (5+luck)
        assert_eq!(roll_rarity(1, 0, 0), ItemRarity::Unique);
        assert_eq!(roll_rarity(1, 0, 1000), ItemRarity::Unique);
    }

    #[test]
    fn roll_rarity_high_luck_increases_unique_chance() {
        // unique_threshold = 5 + luck_bonus
        // With luck=100, threshold is 105. rng=104*something -> roll=104 should be unique
        let roll_104 = 104u64; // rng%1000 = 104
        assert_eq!(roll_rarity(1, 100, roll_104), ItemRarity::Unique);
    }

    #[test]
    fn roll_rarity_high_roll_is_normal() {
        // rng % 1000 = 999 should always be Normal
        assert_eq!(roll_rarity(1, 0, 999), ItemRarity::Normal);
    }

    #[test]
    fn roll_rarity_high_floor_increases_magic_threshold() {
        // magic_threshold = 200 + floor*5 + luck*3
        // Floor 50: magic_threshold = 200+250+0 = 450
        // rng%1000 = 400 should be Magic
        assert_eq!(roll_rarity(50, 0, 400), ItemRarity::Magic);
        // At floor 1: magic_threshold = 205, rng%1000=400 should be Normal
        assert_eq!(roll_rarity(1, 0, 400), ItemRarity::Normal);
    }

    // ── roll_affixes ──────────────────────────────────────────────

    #[test]
    fn magic_affixes_contain_at_least_one_prefix_or_suffix() {
        for seed in 0..50u64 {
            let affixes = roll_affixes(ItemRarity::Magic, seed);
            let has_prefix = affixes.iter().any(|a| a.affix.is_prefix);
            let has_suffix = affixes.iter().any(|a| !a.affix.is_prefix);
            assert!(
                has_prefix || has_suffix,
                "Magic item should have at least one prefix or suffix"
            );
        }
    }

    #[test]
    fn rare_affixes_have_both_prefix_and_suffix() {
        for seed in 0..50u64 {
            let affixes = roll_affixes(ItemRarity::Rare, seed);
            let has_prefix = affixes.iter().any(|a| a.affix.is_prefix);
            let has_suffix = affixes.iter().any(|a| !a.affix.is_prefix);
            assert!(
                has_prefix && has_suffix,
                "Rare item seed {} should have both prefix and suffix, got {:?}",
                seed,
                affixes.iter().map(|a| (a.affix.name, a.affix.is_prefix)).collect::<Vec<_>>()
            );
        }
    }

    // ── rarity_name edge cases ────────────────────────────────────

    #[test]
    fn rarity_name_unique_uses_base_name() {
        let name = rarity_name("Star Destroyer", ItemRarity::Unique, &[]);
        assert_eq!(name, "Star Destroyer");
    }

    #[test]
    fn rarity_name_magic_no_affixes() {
        let name = rarity_name("Blade", ItemRarity::Magic, &[]);
        assert_eq!(name, "Blade");
    }

    #[test]
    fn rarity_name_rare_no_affixes() {
        let name = rarity_name("Blade", ItemRarity::Rare, &[]);
        assert_eq!(name, "Blade");
    }

    #[test]
    fn rarity_name_magic_prefix_and_suffix() {
        let affixes = vec![
            RolledAffix { affix: &PREFIX_POOL[0] }, // Sharpened
            RolledAffix { affix: &SUFFIX_POOL[0] }, // of Precision
        ];
        let name = rarity_name("Blade", ItemRarity::Magic, &affixes);
        assert_eq!(name, "Sharpened Blade of Precision");
    }

    // ── aggregate functions ───────────────────────────────────────

    #[test]
    fn total_affix_spell_power_sums_correctly() {
        let affixes = vec![
            RolledAffix { affix: &PREFIX_POOL[8] },  // SpellPower(1) - Learned
            RolledAffix { affix: &PREFIX_POOL[9] },  // SpellPower(2) - Brilliant
        ];
        assert_eq!(total_affix_spell_power(&affixes), 3);
    }

    #[test]
    fn total_affix_focus_regen_sums_correctly() {
        let affixes = vec![
            RolledAffix { affix: &SUFFIX_POOL[4] },  // FocusRegen(1) - of Focus
            RolledAffix { affix: &SUFFIX_POOL[5] },  // FocusRegen(2) - of Concentration
        ];
        assert_eq!(total_affix_focus_regen(&affixes), 3);
    }

    #[test]
    fn total_affix_gold_find_sums_correctly() {
        let affixes = vec![
            RolledAffix { affix: &PREFIX_POOL[10] }, // GoldFind(15) - Lucky
            RolledAffix { affix: &PREFIX_POOL[11] }, // GoldFind(25) - Prosperous
        ];
        assert_eq!(total_affix_gold_find(&affixes), 40);
    }

    #[test]
    fn total_affix_radical_find_sums_correctly() {
        let affixes = vec![
            RolledAffix { affix: &SUFFIX_POOL[12] }, // RadicalFind(15) - of Radicals
            RolledAffix { affix: &SUFFIX_POOL[13] }, // RadicalFind(25) - of Discovery
        ];
        assert_eq!(total_affix_radical_find(&affixes), 40);
    }

    #[test]
    fn total_affix_hard_answer_damage_sums() {
        let affixes = vec![
            RolledAffix { affix: &SUFFIX_POOL[8] },  // HardAnswerDamage(1)
            RolledAffix { affix: &SUFFIX_POOL[9] },  // HardAnswerDamage(2)
        ];
        assert_eq!(total_affix_hard_answer_damage(&affixes), 3);
    }

    #[test]
    fn total_affix_damage_reduction_sums() {
        let affixes = vec![
            RolledAffix { affix: &SUFFIX_POOL[10] }, // DamageReduction(1)
            RolledAffix { affix: &SUFFIX_POOL[11] }, // DamageReduction(2)
        ];
        assert_eq!(total_affix_damage_reduction(&affixes), 3);
    }

    #[test]
    fn aggregate_empty_affixes_returns_zero() {
        assert_eq!(total_affix_damage(&[]), 0);
        assert_eq!(total_affix_armor(&[]), 0);
        assert_eq!(total_affix_max_hp(&[]), 0);
        assert_eq!(total_affix_crit(&[]), 0);
        assert_eq!(total_affix_spell_power(&[]), 0);
        assert_eq!(total_affix_lifesteal(&[]), 0);
        assert_eq!(total_affix_focus_regen(&[]), 0);
        assert_eq!(total_affix_dodge(&[]), 0);
        assert_eq!(total_affix_gold_find(&[]), 0);
        assert_eq!(total_affix_radical_find(&[]), 0);
        assert_eq!(total_affix_hard_answer_damage(&[]), 0);
        assert_eq!(total_affix_damage_reduction(&[]), 0);
    }

    // ── JSON serialization ────────────────────────────────────────

    #[test]
    fn affixes_to_json_single_affix() {
        let affixes = vec![RolledAffix { affix: &PREFIX_POOL[0] }];
        let json = affixes_to_json(&affixes);
        assert!(json.contains("Sharpened"));
        assert!(json.contains("BonusDamage"));
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
    }

    #[test]
    fn affixes_from_json_empty_string() {
        let parsed = affixes_from_json("");
        assert!(parsed.is_empty());
    }

    #[test]
    fn affixes_from_json_unknown_affix_skipped() {
        let json = r#"[{"name":"NonexistentAffix","effect":"BonusDamage","value":99}]"#;
        let parsed = affixes_from_json(json);
        assert!(parsed.is_empty());
    }

    #[test]
    fn affixes_json_roundtrip_all_pools() {
        // Test with one prefix and one suffix
        for i in 0..PREFIX_POOL.len().min(5) {
            let affixes = vec![RolledAffix { affix: &PREFIX_POOL[i] }];
            let json = affixes_to_json(&affixes);
            let parsed = affixes_from_json(&json);
            assert_eq!(parsed.len(), 1);
            assert_eq!(parsed[0].affix.name, PREFIX_POOL[i].name);
        }
        for i in 0..SUFFIX_POOL.len().min(5) {
            let affixes = vec![RolledAffix { affix: &SUFFIX_POOL[i] }];
            let json = affixes_to_json(&affixes);
            let parsed = affixes_from_json(&json);
            assert_eq!(parsed.len(), 1);
            assert_eq!(parsed[0].affix.name, SUFFIX_POOL[i].name);
        }
    }

    // ── find_unique ───────────────────────────────────────────────

    #[test]
    fn find_unique_all_names() {
        for u in UNIQUE_POOL {
            assert!(find_unique(u.name).is_some(), "unique '{}' not found", u.name);
        }
    }

    #[test]
    fn find_unique_returns_correct_slot() {
        let u = find_unique("Void Conduit").unwrap();
        assert_eq!(u.base_slot, EquipSlot::Charm);
    }
}
