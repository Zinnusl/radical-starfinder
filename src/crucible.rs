//! Crucible skill trees for equipment — dynamically generated per-item trees.
//!
//! Each piece of equipment develops a unique passive tree as the player uses it.
//! Trees are procedurally generated based on equipment slot, rarity, and a seed.
//! Nodes are unlocked by spending accumulated XP.

use crate::player::EquipSlot;
use crate::rarity::ItemRarity;

// ── Effect Enum ──────────────────────────────────────────────────────────────

/// Passive bonus granted by an unlocked crucible node.
#[derive(Clone, Copy, Debug)]
pub enum CrucibleEffect {
    // Stat bonuses
    BonusDamage(i32),
    BonusArmor(i32),
    MaxHp(i32),
    CritChance(i32),
    LifeSteal(i32),
    SpellPower(i32),
    FocusRegen(i32),
    GoldFind(i32),
    RadicalFind(i32),
    DodgeChance(i32),
    MovementBonus(i32),
    // On-hit procs
    BurnOnHit { damage: i32, turns: i32 },
    #[allow(dead_code)]
    PoisonOnHit { damage: i32, turns: i32 },
    // On-kill procs
    ShieldOnKill,
    HealOnKill(i32),
    FocusOnKill(i32),
    // Hard-answer synergy
    HardAnswerDamage(i32),
    HardAnswerHeal(i32),
    // Special
    ComboExtender,      // wrong answer only drops streak by half instead of zero
    DoubleStrike(i32),  // % chance to hit twice on correct answer
    ArmorPierce(i32),   // ignore N points of enemy armor
    OverchargeProc,     // 15% chance for +50% damage on attack
    EmergencyRepair(i32), // auto-heal N HP once per fight when below 25% max
    KineticAbsorber,    // store 50% damage taken, add to next attack
    NeuralSync,         // +50% SRS accuracy recording (counts correct twice)
    TemporalFlux,       // 10% chance for bonus action after kill
}

impl CrucibleEffect {
    #[allow(dead_code)]
    pub fn short_label(&self) -> &'static str {
        match self {
            Self::BonusDamage(_) => "+Dmg",
            Self::BonusArmor(_) => "+Armor",
            Self::MaxHp(_) => "+HP",
            Self::CritChance(_) => "+Crit%",
            Self::LifeSteal(_) => "Drain",
            Self::SpellPower(_) => "+Spell",
            Self::FocusRegen(_) => "+Focus",
            Self::GoldFind(_) => "+Gold",
            Self::RadicalFind(_) => "+Rad",
            Self::DodgeChance(_) => "+Dodge",
            Self::MovementBonus(_) => "+Move",
            Self::BurnOnHit { .. } => "🔥Hit",
            Self::PoisonOnHit { .. } => "☠Hit",
            Self::ShieldOnKill => "🛡Kill",
            Self::HealOnKill(_) => "❤Kill",
            Self::FocusOnKill(_) => "⚡Kill",
            Self::HardAnswerDamage(_) => "📖Dmg",
            Self::HardAnswerHeal(_) => "📖Heal",
            Self::ComboExtender => "Combo+",
            Self::DoubleStrike(_) => "2xHit",
            Self::ArmorPierce(_) => "Pierce",
            Self::OverchargeProc => "Overch",
            Self::EmergencyRepair(_) => "EmRep",
            Self::KineticAbsorber => "Absorb",
            Self::NeuralSync => "SRS+",
            Self::TemporalFlux => "Tempo",
        }
    }
}

// ── Effect Serialization ─────────────────────────────────────────────────────

impl CrucibleEffect {
    pub fn effect_to_tag(&self) -> String {
        match self {
            Self::BonusDamage(n) => format!("BonusDamage:{}", n),
            Self::BonusArmor(n) => format!("BonusArmor:{}", n),
            Self::MaxHp(n) => format!("MaxHp:{}", n),
            Self::CritChance(n) => format!("CritChance:{}", n),
            Self::LifeSteal(n) => format!("LifeSteal:{}", n),
            Self::SpellPower(n) => format!("SpellPower:{}", n),
            Self::FocusRegen(n) => format!("FocusRegen:{}", n),
            Self::GoldFind(n) => format!("GoldFind:{}", n),
            Self::RadicalFind(n) => format!("RadicalFind:{}", n),
            Self::DodgeChance(n) => format!("DodgeChance:{}", n),
            Self::MovementBonus(n) => format!("MovementBonus:{}", n),
            Self::BurnOnHit { damage, turns } => format!("BurnOnHit:{},{}", damage, turns),
            Self::PoisonOnHit { damage, turns } => format!("PoisonOnHit:{},{}", damage, turns),
            Self::ShieldOnKill => "ShieldOnKill".to_string(),
            Self::HealOnKill(n) => format!("HealOnKill:{}", n),
            Self::FocusOnKill(n) => format!("FocusOnKill:{}", n),
            Self::HardAnswerDamage(n) => format!("HardAnswerDamage:{}", n),
            Self::HardAnswerHeal(n) => format!("HardAnswerHeal:{}", n),
            Self::ComboExtender => "ComboExtender".to_string(),
            Self::DoubleStrike(n) => format!("DoubleStrike:{}", n),
            Self::ArmorPierce(n) => format!("ArmorPierce:{}", n),
            Self::OverchargeProc => "OverchargeProc".to_string(),
            Self::EmergencyRepair(n) => format!("EmergencyRepair:{}", n),
            Self::KineticAbsorber => "KineticAbsorber".to_string(),
            Self::NeuralSync => "NeuralSync".to_string(),
            Self::TemporalFlux => "TemporalFlux".to_string(),
        }
    }

    pub fn effect_from_tag(tag: &str) -> CrucibleEffect {
        let (name, vals) = match tag.find(':') {
            Some(pos) => (&tag[..pos], &tag[pos + 1..]),
            None => (tag, ""),
        };
        let parse_i32 = |s: &str| -> i32 { s.trim().parse().unwrap_or(0) };
        match name {
            "BonusDamage" => CrucibleEffect::BonusDamage(parse_i32(vals)),
            "BonusArmor" => CrucibleEffect::BonusArmor(parse_i32(vals)),
            "MaxHp" => CrucibleEffect::MaxHp(parse_i32(vals)),
            "CritChance" => CrucibleEffect::CritChance(parse_i32(vals)),
            "LifeSteal" => CrucibleEffect::LifeSteal(parse_i32(vals)),
            "SpellPower" => CrucibleEffect::SpellPower(parse_i32(vals)),
            "FocusRegen" => CrucibleEffect::FocusRegen(parse_i32(vals)),
            "GoldFind" => CrucibleEffect::GoldFind(parse_i32(vals)),
            "RadicalFind" => CrucibleEffect::RadicalFind(parse_i32(vals)),
            "DodgeChance" => CrucibleEffect::DodgeChance(parse_i32(vals)),
            "MovementBonus" => CrucibleEffect::MovementBonus(parse_i32(vals)),
            "BurnOnHit" => {
                let parts: Vec<&str> = vals.split(',').collect();
                if parts.len() >= 2 {
                    CrucibleEffect::BurnOnHit {
                        damage: parse_i32(parts[0]),
                        turns: parse_i32(parts[1]),
                    }
                } else {
                    CrucibleEffect::BurnOnHit { damage: 1, turns: 3 }
                }
            }
            "PoisonOnHit" => {
                let parts: Vec<&str> = vals.split(',').collect();
                if parts.len() >= 2 {
                    CrucibleEffect::PoisonOnHit {
                        damage: parse_i32(parts[0]),
                        turns: parse_i32(parts[1]),
                    }
                } else {
                    CrucibleEffect::PoisonOnHit { damage: 1, turns: 3 }
                }
            }
            "ShieldOnKill" => CrucibleEffect::ShieldOnKill,
            "HealOnKill" => CrucibleEffect::HealOnKill(parse_i32(vals)),
            "FocusOnKill" => CrucibleEffect::FocusOnKill(parse_i32(vals)),
            "HardAnswerDamage" => CrucibleEffect::HardAnswerDamage(parse_i32(vals)),
            "HardAnswerHeal" => CrucibleEffect::HardAnswerHeal(parse_i32(vals)),
            "ComboExtender" => CrucibleEffect::ComboExtender,
            "DoubleStrike" => CrucibleEffect::DoubleStrike(parse_i32(vals)),
            "ArmorPierce" => CrucibleEffect::ArmorPierce(parse_i32(vals)),
            "OverchargeProc" => CrucibleEffect::OverchargeProc,
            "EmergencyRepair" => CrucibleEffect::EmergencyRepair(parse_i32(vals)),
            "KineticAbsorber" => CrucibleEffect::KineticAbsorber,
            "NeuralSync" => CrucibleEffect::NeuralSync,
            "TemporalFlux" => CrucibleEffect::TemporalFlux,
            _ => CrucibleEffect::BonusDamage(0),
        }
    }
}

/// Standalone alias for `CrucibleEffect::effect_to_tag`.
pub fn effect_tag(e: &CrucibleEffect) -> String {
    e.effect_to_tag()
}

/// Standalone alias for `CrucibleEffect::effect_from_tag`.
pub fn effect_from_tag(tag: &str) -> CrucibleEffect {
    CrucibleEffect::effect_from_tag(tag)
}

// ── Dynamic Node ─────────────────────────────────────────────────────────────

/// A dynamically generated crucible node with owned data.
#[derive(Clone, Debug)]
pub struct CrucibleNodeDyn {
    pub name: String,
    pub description: String,
    pub effect: CrucibleEffect,
    pub xp_cost: u32,
    /// Position for rendering in tree view (world-space).
    pub pos: (f64, f64),
}

// ── PRNG ─────────────────────────────────────────────────────────────────────

fn splitmix(seed: u64) -> (u64, u64) {
    let s = seed.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = s;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z = z ^ (z >> 31);
    (z, s)
}

// ── Name Pools ───────────────────────────────────────────────────────────────

const WEAPON_PREFIXES: &[&str] = &[
    "Calibrated", "Overcharged", "Kinetic", "Plasma", "Focused",
    "Lethal", "Precision", "Assault", "Vampiric", "Piercing",
];
const WEAPON_SUFFIXES: &[&str] = &["Core", "Module", "Circuit", "Array", "Chamber"];

const ARMOR_PREFIXES: &[&str] = &[
    "Reinforced", "Reactive", "Hardened", "Adaptive", "Kinetic",
    "Nano", "Ablative", "Emergency",
];
const ARMOR_SUFFIXES: &[&str] = &["Layer", "Plating", "Shell", "Matrix", "Weave"];

const CHARM_PREFIXES: &[&str] = &[
    "Quantum", "Neural", "Resonance", "Salvage", "Discovery",
    "Focus", "Temporal", "Harmonic",
];
const CHARM_SUFFIXES: &[&str] = &["Link", "Chip", "Crystal", "Node", "Relay"];

fn name_pools(slot: EquipSlot) -> (&'static [&'static str], &'static [&'static str]) {
    match slot {
        EquipSlot::Weapon => (WEAPON_PREFIXES, WEAPON_SUFFIXES),
        EquipSlot::Armor => (ARMOR_PREFIXES, ARMOR_SUFFIXES),
        EquipSlot::Charm => (CHARM_PREFIXES, CHARM_SUFFIXES),
    }
}

fn gen_name(prefixes: &[&str], suffixes: &[&str], seed: u64) -> String {
    let (r1, s2) = splitmix(seed);
    let prefix = prefixes[(r1 as usize) % prefixes.len()];
    let (r2, _) = splitmix(s2);
    let suffix = suffixes[(r2 as usize) % suffixes.len()];
    format!("{} {}", prefix, suffix)
}

// ── Effect Generation ────────────────────────────────────────────────────────

const WEAPON_POOL_SIZE: usize = 10;
const ARMOR_POOL_SIZE: usize = 8;
const CHARM_POOL_SIZE: usize = 9;

fn pool_size(slot: EquipSlot) -> usize {
    match slot {
        EquipSlot::Weapon => WEAPON_POOL_SIZE,
        EquipSlot::Armor => ARMOR_POOL_SIZE,
        EquipSlot::Charm => CHARM_POOL_SIZE,
    }
}

/// Create an effect from the slot-specific pool, using seed for value randomization.
fn generate_effect(slot: EquipSlot, index: usize, seed: u64) -> CrucibleEffect {
    let v = 1 + (seed % 3) as i32;
    let pct = 5 + (seed % 11) as i32;
    match slot {
        EquipSlot::Weapon => match index % WEAPON_POOL_SIZE {
            0 => CrucibleEffect::BonusDamage(v),
            1 => CrucibleEffect::CritChance(pct),
            2 => CrucibleEffect::LifeSteal(v),
            3 => CrucibleEffect::SpellPower(v),
            4 => CrucibleEffect::DoubleStrike(pct),
            5 => CrucibleEffect::ArmorPierce(v),
            6 => CrucibleEffect::OverchargeProc,
            7 => CrucibleEffect::BurnOnHit { damage: v, turns: 2 + (seed % 2) as i32 },
            8 => CrucibleEffect::HardAnswerDamage(v),
            _ => CrucibleEffect::ComboExtender,
        },
        EquipSlot::Armor => match index % ARMOR_POOL_SIZE {
            0 => CrucibleEffect::BonusArmor(v),
            1 => CrucibleEffect::MaxHp(2 + (seed % 4) as i32),
            2 => CrucibleEffect::DodgeChance(pct),
            3 => CrucibleEffect::MovementBonus(v),
            4 => CrucibleEffect::EmergencyRepair(2 + (seed % 3) as i32),
            5 => CrucibleEffect::KineticAbsorber,
            6 => CrucibleEffect::ShieldOnKill,
            _ => CrucibleEffect::HardAnswerHeal(v),
        },
        EquipSlot::Charm => match index % CHARM_POOL_SIZE {
            0 => CrucibleEffect::GoldFind(10 + (seed % 11) as i32),
            1 => CrucibleEffect::RadicalFind(10 + (seed % 11) as i32),
            2 => CrucibleEffect::FocusRegen(v),
            3 => CrucibleEffect::SpellPower(v),
            4 => CrucibleEffect::NeuralSync,
            5 => CrucibleEffect::TemporalFlux,
            6 => CrucibleEffect::FocusOnKill(v),
            7 => CrucibleEffect::HealOnKill(v),
            _ => CrucibleEffect::HardAnswerDamage(v),
        },
    }
}

fn effect_description(effect: &CrucibleEffect) -> String {
    match effect {
        CrucibleEffect::BonusDamage(n) => format!("+{} damage on all attacks", n),
        CrucibleEffect::BonusArmor(n) => format!("+{} armor", n),
        CrucibleEffect::MaxHp(n) => format!("+{} max HP", n),
        CrucibleEffect::CritChance(n) => format!("{}% critical hit chance", n),
        CrucibleEffect::LifeSteal(n) => format!("+{} lifesteal on attacks", n),
        CrucibleEffect::SpellPower(n) => format!("+{} spell power", n),
        CrucibleEffect::FocusRegen(n) => format!("+{} focus regen per turn", n),
        CrucibleEffect::GoldFind(n) => format!("+{}% gold find", n),
        CrucibleEffect::RadicalFind(n) => format!("+{}% radical find", n),
        CrucibleEffect::DodgeChance(n) => format!("{}% dodge chance", n),
        CrucibleEffect::MovementBonus(n) => format!("+{} movement speed", n),
        CrucibleEffect::BurnOnHit { damage, turns } =>
            format!("Burn ({} dmg/turn, {} turns)", damage, turns),
        CrucibleEffect::PoisonOnHit { damage, turns } =>
            format!("Poison ({} dmg/turn, {} turns)", damage, turns),
        CrucibleEffect::ShieldOnKill => "Gain a shield after each kill".to_string(),
        CrucibleEffect::HealOnKill(n) => format!("Heal {} HP per kill", n),
        CrucibleEffect::FocusOnKill(n) => format!("+{} focus on kill", n),
        CrucibleEffect::HardAnswerDamage(n) => format!("+{} damage on hard answers", n),
        CrucibleEffect::HardAnswerHeal(n) => format!("+{} heal on hard answers", n),
        CrucibleEffect::ComboExtender => "Wrong answers only halve combo".to_string(),
        CrucibleEffect::DoubleStrike(n) => format!("{}% chance to strike twice", n),
        CrucibleEffect::ArmorPierce(n) => format!("Ignore {} enemy armor", n),
        CrucibleEffect::OverchargeProc => "15% chance for +50% damage".to_string(),
        CrucibleEffect::EmergencyRepair(n) =>
            format!("Auto-heal {} HP once per fight below 25%", n),
        CrucibleEffect::KineticAbsorber =>
            "Store 50% damage taken, add to next attack".to_string(),
        CrucibleEffect::NeuralSync => "+50% SRS accuracy recording".to_string(),
        CrucibleEffect::TemporalFlux => "10% chance for bonus action after kill".to_string(),
    }
}

// ── Ring Costs ───────────────────────────────────────────────────────────────

const RING_COSTS: [u32; 5] = [0, 10, 25, 50, 80];

fn ring_cost(ring: usize) -> u32 {
    if ring < RING_COSTS.len() { RING_COSTS[ring] } else { 80 }
}

// ── JSON Helpers ─────────────────────────────────────────────────────────────

fn json_push_escaped(out: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
}

fn extract_u32(s: &str, key: &str) -> Option<u32> {
    let needle = format!("\"{}\":", key);
    let start = s.find(&needle)? + needle.len();
    let rest = &s[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn extract_string(s: &str, key: &str) -> String {
    let needle = format!("\"{}\":\"", key);
    let start = match s.find(&needle) {
        Some(pos) => pos + needle.len(),
        None => return String::new(),
    };
    let rest = &s[start..];
    let bytes = rest.as_bytes();
    let mut end = 0;
    let mut escape = false;
    while end < bytes.len() {
        if escape {
            escape = false;
            end += 1;
            continue;
        }
        match bytes[end] {
            b'\\' => {
                escape = true;
                end += 1;
            }
            b'"' => break,
            _ => {
                end += 1;
            }
        }
    }
    rest[..end].replace("\\\"", "\"").replace("\\\\", "\\")
}

fn extract_u32_array(s: &str, key: &str) -> Vec<u32> {
    let needle = format!("\"{}\":[", key);
    let start = match s.find(&needle) {
        Some(pos) => pos + needle.len(),
        None => return vec![],
    };
    let end = match s[start..].find(']') {
        Some(pos) => start + pos,
        None => return vec![],
    };
    let slice = &s[start..end];
    if slice.trim().is_empty() {
        return vec![];
    }
    slice
        .split(',')
        .filter_map(|tok| tok.trim().parse::<u32>().ok())
        .collect()
}

fn extract_f64_array(s: &str, key: &str) -> Vec<f64> {
    let needle = format!("\"{}\":[", key);
    let start = match s.find(&needle) {
        Some(pos) => pos + needle.len(),
        None => return vec![],
    };
    let end = match s[start..].find(']') {
        Some(pos) => start + pos,
        None => return vec![],
    };
    let slice = &s[start..end];
    if slice.trim().is_empty() {
        return vec![];
    }
    slice
        .split(',')
        .filter_map(|tok| tok.trim().parse::<f64>().ok())
        .collect()
}

fn extract_string_array(s: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{}\":[", key);
    let start = match s.find(&needle) {
        Some(pos) => pos + needle.len(),
        None => return vec![],
    };
    let bytes = s.as_bytes();
    let mut depth: i32 = 1;
    let mut pos = start;
    let mut in_str = false;
    let mut esc = false;
    while pos < bytes.len() && depth > 0 {
        if esc {
            esc = false;
            pos += 1;
            continue;
        }
        match bytes[pos] {
            b'\\' if in_str => {
                esc = true;
            }
            b'"' => {
                in_str = !in_str;
            }
            b'[' if !in_str => {
                depth += 1;
            }
            b']' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        pos += 1;
    }
    let end = pos;
    let inner = &s[start..end];
    let mut result = vec![];
    let mut p = 0;
    let ibytes = inner.as_bytes();
    while p < ibytes.len() {
        match inner[p..].find('"') {
            Some(q) => p += q + 1,
            None => break,
        }
        let mut ep = p;
        let mut esc2 = false;
        while ep < ibytes.len() {
            if esc2 {
                esc2 = false;
                ep += 1;
                continue;
            }
            match ibytes[ep] {
                b'\\' => {
                    esc2 = true;
                    ep += 1;
                }
                b'"' => break,
                _ => {
                    ep += 1;
                }
            }
        }
        let raw = &inner[p..ep];
        let val = raw.replace("\\\"", "\"").replace("\\\\", "\\");
        result.push(val);
        p = ep + 1;
    }
    result
}

// ── Runtime State ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct CrucibleState {
    /// The nodes in this item's crucible tree.
    pub nodes: Vec<CrucibleNodeDyn>,
    /// Adjacency list: edges[i] lists indices of nodes connected to node i.
    pub edges: Vec<Vec<usize>>,
    /// Which nodes are allocated (unlocked).
    pub allocated: Vec<bool>,
    /// Accumulated XP on this equipment piece.
    pub xp: u32,
    /// Per-fight state.
    pub emergency_used: bool,
    pub kinetic_stored: i32,
}

impl CrucibleState {
    /// An empty/inactive state (no tree, no nodes).
    pub fn empty() -> Self {
        Self {
            nodes: vec![],
            edges: vec![],
            allocated: vec![],
            xp: 0,
            emergency_used: false,
            kinetic_stored: 0,
        }
    }

    /// Procedurally generate a crucible tree for a given slot, rarity, and seed.
    pub fn generate(slot: EquipSlot, rarity: ItemRarity, seed: u64) -> Self {
        let (count_min, count_max) = match rarity {
            ItemRarity::Normal => (3, 5),
            ItemRarity::Magic => (6, 9),
            ItemRarity::Rare => (10, 14),
            ItemRarity::Unique => (15, 20),
        };

        let (r, mut s) = splitmix(seed);
        let count = count_min + (r as usize % (count_max - count_min + 1));

        let ps = pool_size(slot);
        let (prefixes, suffixes) = name_pools(slot);

        let mut nodes = Vec::with_capacity(count);
        let mut edges: Vec<Vec<usize>> = Vec::with_capacity(count);
        let mut allocated = Vec::with_capacity(count);
        let mut rings: Vec<usize> = Vec::with_capacity(count);

        // Node 0: root at (0,0), ring 0, pre-allocated
        let (r_e, ns) = splitmix(s);
        s = ns;
        let eidx = (r_e as usize) % ps;
        let (r2, ns) = splitmix(s);
        s = ns;
        let effect = generate_effect(slot, eidx, r2);
        let (r3, ns) = splitmix(s);
        s = ns;
        let name = gen_name(prefixes, suffixes, r3);

        nodes.push(CrucibleNodeDyn {
            name,
            description: effect_description(&effect),
            effect,
            xp_cost: 0,
            pos: (0.0, 0.0),
        });
        edges.push(vec![]);
        allocated.push(true);
        rings.push(0);

        let mut current_ring: usize = 0;
        let tau = std::f64::consts::TAU;

        while nodes.len() < count {
            let ring_nodes: Vec<usize> = (0..nodes.len())
                .filter(|&i| rings[i] == current_ring)
                .collect();
            if ring_nodes.is_empty() {
                break;
            }

            let next_ring = current_ring + 1;
            let cost = ring_cost(next_ring);
            let radius = next_ring as f64;

            if current_ring == 0 {
                // Ring 1: distribute children evenly around center
                let remaining = count - nodes.len();
                let (rc, ns) = splitmix(s);
                s = ns;
                let num_children = (1 + (rc as usize % 3)).min(remaining);

                let angle_step = tau / num_children as f64;
                let (r_rot, ns) = splitmix(s);
                s = ns;
                let base_angle = (r_rot % 1000) as f64 / 1000.0 * tau;

                for c in 0..num_children {
                    let child_idx = nodes.len();
                    let angle = base_angle + c as f64 * angle_step;
                    let px = angle.cos() * radius;
                    let py = angle.sin() * radius;

                    let (re, ns) = splitmix(s);
                    s = ns;
                    let ei = (re as usize) % ps;
                    let (rv, ns) = splitmix(s);
                    s = ns;
                    let eff = generate_effect(slot, ei, rv);
                    let (rn, ns) = splitmix(s);
                    s = ns;
                    let nm = gen_name(prefixes, suffixes, rn);

                    nodes.push(CrucibleNodeDyn {
                        name: nm,
                        description: effect_description(&eff),
                        effect: eff,
                        xp_cost: cost,
                        pos: (px, py),
                    });
                    edges.push(vec![0]);
                    edges[0].push(child_idx);
                    allocated.push(false);
                    rings.push(next_ring);
                }
            } else {
                // Ring N+1: add children to each node in current ring
                for &parent in &ring_nodes {
                    if nodes.len() >= count {
                        break;
                    }
                    let remaining = count - nodes.len();
                    let (rc, ns) = splitmix(s);
                    s = ns;
                    let num_children = (1 + (rc as usize % 2)).min(remaining);

                    let (ppx, ppy) = nodes[parent].pos;
                    let parent_angle = ppy.atan2(ppx);
                    let fan_spread = (std::f64::consts::PI
                        / (1u32 << current_ring.min(30) as u32) as f64)
                        .max(0.3);

                    for c in 0..num_children {
                        let child_idx = nodes.len();
                        let offset = if num_children == 1 {
                            0.0
                        } else {
                            (c as f64 / (num_children - 1) as f64 - 0.5) * fan_spread
                        };
                        let angle = parent_angle + offset;
                        let px = angle.cos() * radius;
                        let py = angle.sin() * radius;

                        let (re, ns) = splitmix(s);
                        s = ns;
                        let ei = (re as usize) % ps;
                        let (rv, ns) = splitmix(s);
                        s = ns;
                        let eff = generate_effect(slot, ei, rv);
                        let (rn, ns) = splitmix(s);
                        s = ns;
                        let nm = gen_name(prefixes, suffixes, rn);

                        nodes.push(CrucibleNodeDyn {
                            name: nm,
                            description: effect_description(&eff),
                            effect: eff,
                            xp_cost: cost,
                            pos: (px, py),
                        });
                        edges.push(vec![parent]);
                        edges[parent].push(child_idx);
                        allocated.push(false);
                        rings.push(next_ring);
                    }
                }
            }
            current_ring = next_ring;
        }

        CrucibleState {
            nodes,
            edges,
            allocated,
            xp: 0,
            emergency_used: false,
            kinetic_stored: 0,
        }
    }

    /// Create a crucible state for a specific equipment piece (defaults to Normal rarity).
    pub fn for_equipment(equipment: &crate::player::Equipment) -> Self {
        let hash = equipment
            .name
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        Self::generate(equipment.slot, ItemRarity::Normal, hash)
    }

    /// True if node `idx` can be allocated: exists, not yet allocated,
    /// player has enough XP, and at least one neighbor is already allocated.
    pub fn can_allocate(&self, idx: usize) -> bool {
        if idx >= self.nodes.len() {
            return false;
        }
        if self.allocated[idx] {
            return false;
        }
        if self.xp < self.nodes[idx].xp_cost {
            return false;
        }
        self.edges[idx].iter().any(|&j| self.allocated[j])
    }

    /// Spend XP and allocate the node. Returns true on success.
    pub fn allocate(&mut self, idx: usize) -> bool {
        if !self.can_allocate(idx) {
            return false;
        }
        self.xp -= self.nodes[idx].xp_cost;
        self.allocated[idx] = true;
        true
    }

    /// Add XP to the pool (no auto-unlock).
    pub fn gain_xp(&mut self, amount: u32) {
        self.xp += amount;
    }

    /// Collect all active effects from allocated nodes.
    pub fn active_effects(&self) -> Vec<CrucibleEffect> {
        self.allocated
            .iter()
            .enumerate()
            .filter(|(_, &a)| a)
            .map(|(i, _)| self.nodes[i].effect)
            .collect()
    }

    /// Number of allocated nodes.
    #[allow(dead_code)]
    pub fn unlocked_count(&self) -> usize {
        self.allocated.iter().filter(|&&a| a).count()
    }

    /// Reset per-fight state (emergency repair, kinetic absorber).
    #[allow(dead_code)]
    pub fn reset_fight(&mut self) {
        self.emergency_used = false;
        self.kinetic_stored = 0;
    }

    /// XP needed for the cheapest allocatable (adjacent) node, or `None` if
    /// no node is currently allocatable.
    pub fn xp_to_next(&self) -> Option<u32> {
        let mut cheapest: Option<u32> = None;
        for i in 0..self.nodes.len() {
            if self.allocated[i] {
                continue;
            }
            let adjacent = self.edges[i].iter().any(|&j| self.allocated[j]);
            if !adjacent {
                continue;
            }
            let needed = self.nodes[i].xp_cost.saturating_sub(self.xp);
            match cheapest {
                None => cheapest = Some(needed),
                Some(prev) if needed < prev => cheapest = Some(needed),
                _ => {}
            }
        }
        cheapest
    }

    /// Legacy method — always returns false in the dynamic tree system.
    pub fn pending_branch(&self) -> bool {
        false
    }

    /// Legacy method — no-op in the dynamic tree system.
    pub fn choose_branch(&mut self, _left: bool) {}

    /// Serialize to a JSON string for localStorage.
    pub fn to_json(&self) -> String {
        let nc = self.nodes.len();
        let mut out = String::from("{\"nc\":");
        out.push_str(&nc.to_string());

        // Node names
        out.push_str(",\"nm\":[");
        for (i, node) in self.nodes.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            json_push_escaped(&mut out, &node.name);
            out.push('"');
        }
        out.push(']');

        // Descriptions
        out.push_str(",\"ds\":[");
        for (i, node) in self.nodes.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            json_push_escaped(&mut out, &node.description);
            out.push('"');
        }
        out.push(']');

        // Effect tags
        out.push_str(",\"fx\":[");
        for (i, node) in self.nodes.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&node.effect.effect_to_tag());
            out.push('"');
        }
        out.push(']');

        // XP costs
        out.push_str(",\"xc\":[");
        for (i, node) in self.nodes.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&node.xp_cost.to_string());
        }
        out.push(']');

        // Positions X
        out.push_str(",\"px\":[");
        for (i, node) in self.nodes.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!("{:.2}", node.pos.0));
        }
        out.push(']');

        // Positions Y
        out.push_str(",\"py\":[");
        for (i, node) in self.nodes.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!("{:.2}", node.pos.1));
        }
        out.push(']');

        // Edges as semicolon-separated adjacency lists
        out.push_str(",\"eg\":\"");
        for (i, neighbors) in self.edges.iter().enumerate() {
            if i > 0 {
                out.push(';');
            }
            for (j, &n) in neighbors.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                out.push_str(&n.to_string());
            }
        }
        out.push('"');

        // Allocated flags
        out.push_str(",\"al\":[");
        for (i, &a) in self.allocated.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push(if a { '1' } else { '0' });
        }
        out.push(']');

        // XP balance
        out.push_str(",\"xp\":");
        out.push_str(&self.xp.to_string());

        out.push('}');
        out
    }

    /// Deserialize from JSON string. Returns `empty()` on any parse failure.
    pub fn from_json(json: &str) -> Self {
        let nc = match extract_u32(json, "nc") {
            Some(n) => n as usize,
            None => return Self::empty(),
        };
        if nc == 0 {
            return Self::empty();
        }

        let names = extract_string_array(json, "nm");
        let descs = extract_string_array(json, "ds");
        let fxs = extract_string_array(json, "fx");
        let xcs = extract_u32_array(json, "xc");
        let pxs = extract_f64_array(json, "px");
        let pys = extract_f64_array(json, "py");
        let edges_str = extract_string(json, "eg");
        let als = extract_u32_array(json, "al");
        let xp = extract_u32(json, "xp").unwrap_or(0);

        if names.len() != nc
            || descs.len() != nc
            || fxs.len() != nc
            || xcs.len() != nc
            || pxs.len() != nc
            || pys.len() != nc
            || als.len() != nc
        {
            return Self::empty();
        }

        let mut nodes = Vec::with_capacity(nc);
        for i in 0..nc {
            nodes.push(CrucibleNodeDyn {
                name: names[i].clone(),
                description: descs[i].clone(),
                effect: CrucibleEffect::effect_from_tag(&fxs[i]),
                xp_cost: xcs[i],
                pos: (pxs[i], pys[i]),
            });
        }

        let mut edge_lists = vec![vec![]; nc];
        if !edges_str.is_empty() {
            for (i, group) in edges_str.split(';').enumerate() {
                if i >= nc {
                    break;
                }
                if group.is_empty() {
                    continue;
                }
                for tok in group.split(',') {
                    if let Ok(j) = tok.trim().parse::<usize>() {
                        if j < nc {
                            edge_lists[i].push(j);
                        }
                    }
                }
            }
        }

        let allocated_vec: Vec<bool> = als.iter().map(|&v| v != 0).collect();

        CrucibleState {
            nodes,
            edges: edge_lists,
            allocated: allocated_vec,
            xp,
            emergency_used: false,
            kinetic_stored: 0,
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Deterministically hash equipment slot and name to a usize value.
/// Kept for backwards compatibility; callers should prefer `CrucibleState::generate`.
pub fn tree_for_equipment(slot: EquipSlot, name: &str) -> usize {
    let offset: u32 = match slot {
        EquipSlot::Weapon => 0,
        EquipSlot::Armor => 3,
        EquipSlot::Charm => 6,
    };
    let hash = name
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    (offset + (hash % 3)) as usize
}

// ── Aggregate helpers (used by combat systems) ───────────────────────────────

/// Sum a numeric effect across multiple crucible states.
pub fn aggregate_bonus_damage(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::BonusDamage(n) => Some(n),
            _ => None,
        })
        .sum()
}

pub fn aggregate_bonus_armor(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::BonusArmor(n) => Some(n),
            _ => None,
        })
        .sum()
}

pub fn aggregate_max_hp(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::MaxHp(n) => Some(n),
            _ => None,
        })
        .sum()
}

pub fn aggregate_crit_chance(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::CritChance(n) => Some(n),
            _ => None,
        })
        .sum()
}

#[allow(dead_code)]
pub fn aggregate_spell_power(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::SpellPower(n) => Some(n),
            _ => None,
        })
        .sum()
}

pub fn aggregate_dodge_chance(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::DodgeChance(n) => Some(n),
            _ => None,
        })
        .sum()
}

pub fn aggregate_gold_find(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::GoldFind(n) => Some(n),
            _ => None,
        })
        .sum()
}

pub fn aggregate_radical_find(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::RadicalFind(n) => Some(n),
            _ => None,
        })
        .sum()
}

#[allow(dead_code)]
pub fn aggregate_focus_regen(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::FocusRegen(n) => Some(n),
            _ => None,
        })
        .sum()
}

pub fn aggregate_lifesteal(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::LifeSteal(n) => Some(n),
            _ => None,
        })
        .sum()
}

pub fn aggregate_heal_on_kill(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::HealOnKill(n) => Some(n),
            _ => None,
        })
        .sum()
}

pub fn aggregate_armor_pierce(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::ArmorPierce(n) => Some(n),
            _ => None,
        })
        .sum()
}

pub fn aggregate_hard_answer_damage(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::HardAnswerDamage(n) => Some(n),
            _ => None,
        })
        .sum()
}

pub fn aggregate_hard_answer_heal(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::HardAnswerHeal(n) => Some(n),
            _ => None,
        })
        .sum()
}

pub fn aggregate_double_strike(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::DoubleStrike(n) => Some(n),
            _ => None,
        })
        .sum()
}

#[allow(dead_code)]
pub fn aggregate_movement_bonus(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::MovementBonus(n) => Some(n),
            _ => None,
        })
        .sum()
}

#[allow(dead_code)]
pub fn aggregate_focus_on_kill(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::FocusOnKill(n) => Some(n),
            _ => None,
        })
        .sum()
}

/// Check if any crucible has a specific flag-type effect.
#[allow(dead_code)]
pub fn has_effect(states: &[&CrucibleState], check: fn(&CrucibleEffect) -> bool) -> bool {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .any(|e| check(&e))
}

pub fn has_combo_extender(states: &[&CrucibleState]) -> bool {
    has_effect(states, |e| matches!(e, CrucibleEffect::ComboExtender))
}

pub fn has_overcharge_proc(states: &[&CrucibleState]) -> bool {
    has_effect(states, |e| matches!(e, CrucibleEffect::OverchargeProc))
}

pub fn has_shield_on_kill(states: &[&CrucibleState]) -> bool {
    has_effect(states, |e| matches!(e, CrucibleEffect::ShieldOnKill))
}

#[allow(dead_code)]
pub fn has_neural_sync(states: &[&CrucibleState]) -> bool {
    has_effect(states, |e| matches!(e, CrucibleEffect::NeuralSync))
}

#[allow(dead_code)]
pub fn has_temporal_flux(states: &[&CrucibleState]) -> bool {
    has_effect(states, |e| matches!(e, CrucibleEffect::TemporalFlux))
}

#[allow(dead_code)]
pub fn has_kinetic_absorber(states: &[&CrucibleState]) -> bool {
    has_effect(states, |e| matches!(e, CrucibleEffect::KineticAbsorber))
}

/// Get burn-on-hit params if any crucible has it.
pub fn burn_on_hit(states: &[&CrucibleState]) -> Option<(i32, i32)> {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .find_map(|e| match e {
            CrucibleEffect::BurnOnHit { damage, turns } => Some((damage, turns)),
            _ => None,
        })
}

/// Get poison-on-hit params if any crucible has it.
#[allow(dead_code)]
pub fn poison_on_hit(states: &[&CrucibleState]) -> Option<(i32, i32)> {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .find_map(|e| match e {
            CrucibleEffect::PoisonOnHit { damage, turns } => Some((damage, turns)),
            _ => None,
        })
}

/// Get emergency repair threshold if any crucible has it (returns heal amount).
pub fn emergency_repair_amount(states: &[&CrucibleState]) -> i32 {
    states
        .iter()
        .flat_map(|s| s.active_effects())
        .filter_map(|e| match e {
            CrucibleEffect::EmergencyRepair(n) => Some(n),
            _ => None,
        })
        .sum()
}


#[cfg(test)]
mod tests {
    use super::*;

    // ── short_label tests ──

    #[test]
    fn short_label_all_variants() {
        assert_eq!(CrucibleEffect::BonusDamage(1).short_label(), "+Dmg");
        assert_eq!(CrucibleEffect::BonusArmor(1).short_label(), "+Armor");
        assert_eq!(CrucibleEffect::MaxHp(5).short_label(), "+HP");
        assert_eq!(CrucibleEffect::CritChance(10).short_label(), "+Crit%");
        assert_eq!(CrucibleEffect::LifeSteal(1).short_label(), "Drain");
        assert_eq!(CrucibleEffect::SpellPower(1).short_label(), "+Spell");
        assert_eq!(CrucibleEffect::FocusRegen(1).short_label(), "+Focus");
        assert_eq!(CrucibleEffect::GoldFind(15).short_label(), "+Gold");
        assert_eq!(CrucibleEffect::RadicalFind(15).short_label(), "+Rad");
        assert_eq!(CrucibleEffect::DodgeChance(5).short_label(), "+Dodge");
        assert_eq!(CrucibleEffect::MovementBonus(1).short_label(), "+Move");
        assert_eq!(
            CrucibleEffect::BurnOnHit { damage: 1, turns: 3 }.short_label(),
            "🔥Hit"
        );
        assert_eq!(
            CrucibleEffect::PoisonOnHit { damage: 2, turns: 4 }.short_label(),
            "☠Hit"
        );
        assert_eq!(CrucibleEffect::ShieldOnKill.short_label(), "🛡Kill");
        assert_eq!(CrucibleEffect::HealOnKill(1).short_label(), "❤Kill");
        assert_eq!(CrucibleEffect::FocusOnKill(3).short_label(), "⚡Kill");
        assert_eq!(CrucibleEffect::HardAnswerDamage(2).short_label(), "📖Dmg");
        assert_eq!(CrucibleEffect::HardAnswerHeal(2).short_label(), "📖Heal");
        assert_eq!(CrucibleEffect::ComboExtender.short_label(), "Combo+");
        assert_eq!(CrucibleEffect::DoubleStrike(10).short_label(), "2xHit");
        assert_eq!(CrucibleEffect::ArmorPierce(2).short_label(), "Pierce");
        assert_eq!(CrucibleEffect::OverchargeProc.short_label(), "Overch");
        assert_eq!(CrucibleEffect::EmergencyRepair(3).short_label(), "EmRep");
        assert_eq!(CrucibleEffect::KineticAbsorber.short_label(), "Absorb");
        assert_eq!(CrucibleEffect::NeuralSync.short_label(), "SRS+");
        assert_eq!(CrucibleEffect::TemporalFlux.short_label(), "Tempo");
    }

    // ── generate: node count ranges per rarity ──

    #[test]
    fn generate_normal_count() {
        for seed in 0..50u64 {
            let s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, seed);
            assert!(
                s.nodes.len() >= 3 && s.nodes.len() <= 5,
                "Normal tree had {} nodes (seed={})",
                s.nodes.len(),
                seed
            );
        }
    }

    #[test]
    fn generate_magic_count() {
        for seed in 0..50u64 {
            let s = CrucibleState::generate(EquipSlot::Armor, ItemRarity::Magic, seed);
            assert!(
                s.nodes.len() >= 6 && s.nodes.len() <= 9,
                "Magic tree had {} nodes (seed={})",
                s.nodes.len(),
                seed
            );
        }
    }

    #[test]
    fn generate_rare_count() {
        for seed in 0..50u64 {
            let s = CrucibleState::generate(EquipSlot::Charm, ItemRarity::Rare, seed);
            assert!(
                s.nodes.len() >= 10 && s.nodes.len() <= 14,
                "Rare tree had {} nodes (seed={})",
                s.nodes.len(),
                seed
            );
        }
    }

    #[test]
    fn generate_unique_count() {
        for seed in 0..50u64 {
            let s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Unique, seed);
            assert!(
                s.nodes.len() >= 15 && s.nodes.len() <= 20,
                "Unique tree had {} nodes (seed={})",
                s.nodes.len(),
                seed
            );
        }
    }

    #[test]
    fn generate_root_pre_allocated() {
        let s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        assert!(s.allocated[0], "Node 0 should be pre-allocated");
        assert_eq!(s.nodes[0].xp_cost, 0, "Root node should be free");
        assert_eq!(s.nodes[0].pos, (0.0, 0.0), "Root node at origin");
    }

    #[test]
    fn generate_deterministic() {
        let a = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Rare, 12345);
        let b = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Rare, 12345);
        assert_eq!(a.nodes.len(), b.nodes.len());
        for i in 0..a.nodes.len() {
            assert_eq!(a.nodes[i].name, b.nodes[i].name);
            assert_eq!(a.nodes[i].xp_cost, b.nodes[i].xp_cost);
        }
    }

    #[test]
    fn generate_tree_connected() {
        let s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Unique, 99);
        let n = s.nodes.len();
        let mut visited = vec![false; n];
        let mut stack = vec![0usize];
        while let Some(node) = stack.pop() {
            if visited[node] {
                continue;
            }
            visited[node] = true;
            for &neighbor in &s.edges[node] {
                if !visited[neighbor] {
                    stack.push(neighbor);
                }
            }
        }
        assert!(
            visited.iter().all(|&v| v),
            "All nodes must be reachable from root"
        );
    }

    // ── can_allocate / allocate ──

    #[test]
    fn can_allocate_root_already_allocated() {
        let s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        assert!(!s.can_allocate(0), "Root is already allocated");
    }

    #[test]
    fn can_allocate_neighbor_of_root() {
        let mut s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        s.xp = 1000;
        let neighbor = s.edges[0][0];
        assert!(s.can_allocate(neighbor));
    }

    #[test]
    fn can_allocate_requires_xp() {
        let s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        let neighbor = s.edges[0][0];
        assert!(!s.can_allocate(neighbor));
    }

    #[test]
    fn can_allocate_requires_adjacency() {
        let mut s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Rare, 42);
        s.xp = 10000;
        let non_adj = (1..s.nodes.len()).find(|&i| {
            !s.edges[i].iter().any(|&j| s.allocated[j])
        });
        if let Some(idx) = non_adj {
            assert!(
                !s.can_allocate(idx),
                "Node {} should not be allocatable without adjacent allocated",
                idx
            );
        }
    }

    #[test]
    fn allocate_spends_xp() {
        let mut s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        s.xp = 100;
        let neighbor = s.edges[0][0];
        let cost = s.nodes[neighbor].xp_cost;
        assert!(s.allocate(neighbor));
        assert_eq!(s.xp, 100 - cost);
        assert!(s.allocated[neighbor]);
    }

    #[test]
    fn allocate_fails_when_cannot() {
        let mut s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        assert!(!s.allocate(0)); // already allocated
    }

    #[test]
    fn allocate_out_of_bounds() {
        let mut s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        assert!(!s.can_allocate(9999));
        assert!(!s.allocate(9999));
    }

    // ── gain_xp ──

    #[test]
    fn gain_xp_accumulates() {
        let mut s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        assert_eq!(s.xp, 0);
        s.gain_xp(10);
        assert_eq!(s.xp, 10);
        s.gain_xp(5);
        assert_eq!(s.xp, 15);
    }

    #[test]
    fn gain_xp_no_auto_unlock() {
        let mut s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        s.gain_xp(10000);
        assert_eq!(s.unlocked_count(), 1);
    }

    // ── active_effects ──

    #[test]
    fn active_effects_only_root() {
        let s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        assert_eq!(s.active_effects().len(), 1);
    }

    #[test]
    fn active_effects_after_allocate() {
        let mut s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        s.xp = 1000;
        let neighbor = s.edges[0][0];
        s.allocate(neighbor);
        assert_eq!(s.active_effects().len(), 2);
    }

    #[test]
    fn active_effects_empty_state() {
        let s = CrucibleState::empty();
        assert!(s.active_effects().is_empty());
    }

    // ── to_json / from_json roundtrip ──

    #[test]
    fn json_roundtrip() {
        let mut s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Rare, 42);
        s.xp = 1000;
        let neighbor = s.edges[0][0];
        s.allocate(neighbor);

        let json = s.to_json();
        let s2 = CrucibleState::from_json(&json);

        assert_eq!(s2.nodes.len(), s.nodes.len());
        assert_eq!(s2.xp, s.xp);
        assert_eq!(s2.allocated, s.allocated);
        for i in 0..s.nodes.len() {
            assert_eq!(s2.nodes[i].name, s.nodes[i].name);
            assert_eq!(s2.nodes[i].description, s.nodes[i].description);
            assert_eq!(s2.nodes[i].xp_cost, s.nodes[i].xp_cost);
            assert_eq!(
                s2.nodes[i].effect.short_label(),
                s.nodes[i].effect.short_label()
            );
            assert_eq!(s2.edges[i].len(), s.edges[i].len());
        }
    }

    #[test]
    fn json_roundtrip_empty() {
        let s = CrucibleState::empty();
        let json = s.to_json();
        let s2 = CrucibleState::from_json(&json);
        assert_eq!(s2.nodes.len(), 0);
        assert_eq!(s2.xp, 0);
    }

    #[test]
    fn from_json_invalid_returns_empty() {
        let s = CrucibleState::from_json("not valid json at all");
        assert_eq!(s.nodes.len(), 0);
        assert_eq!(s.xp, 0);
    }

    #[test]
    fn from_json_empty_string() {
        let s = CrucibleState::from_json("");
        assert_eq!(s.nodes.len(), 0);
    }

    #[test]
    fn from_json_old_format_returns_empty() {
        let s = CrucibleState::from_json(r#"{"t":0,"x":42,"u":[1,1,0,0,0],"b":null}"#);
        assert_eq!(s.nodes.len(), 0);
    }

    // ── unlocked_count ──

    #[test]
    fn unlocked_count_initial() {
        let s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        assert_eq!(s.unlocked_count(), 1);
    }

    #[test]
    fn unlocked_count_after_allocate() {
        let mut s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        s.xp = 1000;
        let neighbor = s.edges[0][0];
        s.allocate(neighbor);
        assert_eq!(s.unlocked_count(), 2);
    }

    #[test]
    fn unlocked_count_empty() {
        let s = CrucibleState::empty();
        assert_eq!(s.unlocked_count(), 0);
    }

    // ── reset_fight ──

    #[test]
    fn reset_fight_clears_state() {
        let mut s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        s.emergency_used = true;
        s.kinetic_stored = 42;
        s.reset_fight();
        assert!(!s.emergency_used);
        assert_eq!(s.kinetic_stored, 0);
    }

    #[test]
    fn reset_fight_preserves_tree() {
        let mut s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        s.xp = 1000;
        let neighbor = s.edges[0][0];
        s.allocate(neighbor);
        let saved_xp = s.xp;
        s.emergency_used = true;
        s.kinetic_stored = 10;
        s.reset_fight();
        assert_eq!(s.unlocked_count(), 2);
        assert_eq!(s.xp, saved_xp);
    }

    // ── xp_to_next ──

    #[test]
    fn xp_to_next_shows_needed() {
        let s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        assert_eq!(s.xp_to_next(), Some(10));
    }

    #[test]
    fn xp_to_next_with_enough_xp() {
        let mut s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        s.xp = 100;
        assert_eq!(s.xp_to_next(), Some(0));
    }

    #[test]
    fn xp_to_next_empty() {
        let s = CrucibleState::empty();
        assert_eq!(s.xp_to_next(), None);
    }

    // ── legacy methods ──

    #[test]
    fn pending_branch_always_false() {
        let s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        assert!(!s.pending_branch());
    }

    // ── tree_for_equipment ──

    #[test]
    fn tree_for_equipment_deterministic() {
        let t1 = tree_for_equipment(EquipSlot::Weapon, "Laser Pistol");
        let t2 = tree_for_equipment(EquipSlot::Weapon, "Laser Pistol");
        assert_eq!(t1, t2);
    }

    #[test]
    fn tree_for_equipment_slot_ranges() {
        let w = tree_for_equipment(EquipSlot::Weapon, "Test");
        assert!(w < 3);
        let a = tree_for_equipment(EquipSlot::Armor, "Test");
        assert!((3..6).contains(&a));
        let c = tree_for_equipment(EquipSlot::Charm, "Test");
        assert!((6..9).contains(&c));
    }

    // ── aggregate functions ──

    #[test]
    fn aggregate_empty_returns_zero() {
        let empty: &[&CrucibleState] = &[];
        assert_eq!(aggregate_bonus_damage(empty), 0);
        assert_eq!(aggregate_bonus_armor(empty), 0);
        assert_eq!(aggregate_max_hp(empty), 0);
        assert_eq!(aggregate_crit_chance(empty), 0);
        assert_eq!(aggregate_dodge_chance(empty), 0);
        assert_eq!(aggregate_gold_find(empty), 0);
        assert_eq!(aggregate_radical_find(empty), 0);
        assert_eq!(aggregate_lifesteal(empty), 0);
        assert_eq!(aggregate_heal_on_kill(empty), 0);
        assert_eq!(aggregate_armor_pierce(empty), 0);
        assert_eq!(aggregate_hard_answer_damage(empty), 0);
        assert_eq!(aggregate_hard_answer_heal(empty), 0);
        assert_eq!(aggregate_double_strike(empty), 0);
        assert_eq!(emergency_repair_amount(empty), 0);
        assert!(!has_combo_extender(empty));
        assert!(!has_overcharge_proc(empty));
        assert!(!has_shield_on_kill(empty));
        assert_eq!(burn_on_hit(empty), None);
        assert_eq!(poison_on_hit(empty), None);
    }

    #[test]
    fn aggregate_with_generated_state() {
        let s = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Normal, 42);
        let states: &[&CrucibleState] = &[&s];
        let _ = aggregate_bonus_damage(states);
        let _ = aggregate_bonus_armor(states);
        let _ = aggregate_max_hp(states);
        let _ = aggregate_crit_chance(states);
        let _ = aggregate_dodge_chance(states);
        let _ = aggregate_gold_find(states);
        let _ = aggregate_radical_find(states);
        let _ = aggregate_lifesteal(states);
        let _ = aggregate_heal_on_kill(states);
        let _ = aggregate_armor_pierce(states);
        let _ = aggregate_hard_answer_damage(states);
        let _ = aggregate_hard_answer_heal(states);
        let _ = aggregate_double_strike(states);
        let _ = emergency_repair_amount(states);
        let _ = has_combo_extender(states);
        let _ = has_overcharge_proc(states);
        let _ = has_shield_on_kill(states);
        let _ = burn_on_hit(states);
        let _ = poison_on_hit(states);
    }

    // ── effect tag roundtrip ──

    #[test]
    fn effect_tag_roundtrip() {
        let effects = vec![
            CrucibleEffect::BonusDamage(3),
            CrucibleEffect::BonusArmor(2),
            CrucibleEffect::MaxHp(5),
            CrucibleEffect::CritChance(10),
            CrucibleEffect::LifeSteal(1),
            CrucibleEffect::SpellPower(2),
            CrucibleEffect::FocusRegen(1),
            CrucibleEffect::GoldFind(15),
            CrucibleEffect::RadicalFind(15),
            CrucibleEffect::DodgeChance(5),
            CrucibleEffect::MovementBonus(1),
            CrucibleEffect::BurnOnHit { damage: 2, turns: 3 },
            CrucibleEffect::ShieldOnKill,
            CrucibleEffect::HealOnKill(2),
            CrucibleEffect::FocusOnKill(3),
            CrucibleEffect::HardAnswerDamage(2),
            CrucibleEffect::HardAnswerHeal(1),
            CrucibleEffect::ComboExtender,
            CrucibleEffect::DoubleStrike(10),
            CrucibleEffect::ArmorPierce(2),
            CrucibleEffect::OverchargeProc,
            CrucibleEffect::EmergencyRepair(3),
            CrucibleEffect::KineticAbsorber,
            CrucibleEffect::NeuralSync,
            CrucibleEffect::TemporalFlux,
        ];
        for e in &effects {
            let tag = effect_tag(e);
            let recovered = effect_from_tag(&tag);
            assert_eq!(
                recovered.short_label(),
                e.short_label(),
                "Tag '{}' didn't roundtrip for {:?}",
                tag,
                e
            );
        }
    }

    // ── slot affects pool ──

    #[test]
    fn slot_affects_effect_pool() {
        let s_weapon = CrucibleState::generate(EquipSlot::Weapon, ItemRarity::Rare, 42);
        let s_armor = CrucibleState::generate(EquipSlot::Armor, ItemRarity::Rare, 42);
        let w_labels: Vec<_> = s_weapon.nodes.iter().map(|n| n.name.clone()).collect();
        let a_labels: Vec<_> = s_armor.nodes.iter().map(|n| n.name.clone()).collect();
        assert_ne!(w_labels, a_labels);
    }
}
