//! Crucible skill trees for equipment — inspired by POE 1's Crucible league.
//!
//! Each piece of equipment develops a small passive tree as the player uses it.
//! Weapon trees are flavored as "Attachments", armor as "Plating", modules as
//! "Firmware", and alien artifacts as "Resonance Patterns".
//!
//! Tree layout (5 nodes each):
//! ```text
//!   [0] ─── [1] ─── [2]  (linear trunk, auto-unlock)
//!                     ├── [3]  (left branch — player chooses)
//!                     └── [4]  (right branch — mutually exclusive)
//! ```
//!
//! Equipment gains XP from correct answers while equipped.
//! Trunk nodes auto-unlock; branch nodes require a choice at a Forge workbench.

use crate::player::EquipSlot;

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

// ── Node / Template ──────────────────────────────────────────────────────────

/// A single node in a crucible tree.
#[derive(Clone, Copy, Debug)]
pub struct CrucibleNode {
    pub name: &'static str,
    pub description: &'static str,
    pub effect: CrucibleEffect,
    /// Cumulative XP required to unlock this node.
    pub xp_cost: u32,
}

/// A 5-node tree template.
pub struct CrucibleTemplate {
    pub name: &'static str,
    /// Flavor label: "Attachment", "Plating", "Firmware", "Resonance"
    #[allow(dead_code)]
    pub flavor: &'static str,
    pub nodes: [CrucibleNode; 5],
}

// ── Static Templates ─────────────────────────────────────────────────────────

/// 9 templates: indices 0–2 for weapons, 3–5 for armor, 6–8 for charms.
pub static CRUCIBLE_TEMPLATES: &[CrucibleTemplate] = &[
    // ─── Weapon 0: Assault Configuration ─────────────────────────────────────
    CrucibleTemplate {
        name: "Assault Configuration",
        flavor: "Attachment",
        nodes: [
            CrucibleNode {
                name: "Calibrated Barrel",
                description: "+1 damage on all attacks",
                effect: CrucibleEffect::BonusDamage(1),
                xp_cost: 5,
            },
            CrucibleNode {
                name: "Accelerator Coil",
                description: "10% critical hit chance",
                effect: CrucibleEffect::CritChance(10),
                xp_cost: 15,
            },
            CrucibleNode {
                name: "Overcharge Port",
                description: "15% chance for +50% damage",
                effect: CrucibleEffect::OverchargeProc,
                xp_cost: 30,
            },
            CrucibleNode {
                name: "Incendiary Rounds",
                description: "Attacks burn enemies (1 dmg/turn, 3 turns)",
                effect: CrucibleEffect::BurnOnHit { damage: 1, turns: 3 },
                xp_cost: 50,
            },
            CrucibleNode {
                name: "Armor-Piercing Tips",
                description: "Attacks ignore 2 points of enemy armor",
                effect: CrucibleEffect::ArmorPierce(2),
                xp_cost: 50,
            },
        ],
    },
    // ─── Weapon 1: Precision Configuration ───────────────────────────────────
    CrucibleTemplate {
        name: "Precision Configuration",
        flavor: "Attachment",
        nodes: [
            CrucibleNode {
                name: "Tactical Scope",
                description: "+1 spell power",
                effect: CrucibleEffect::SpellPower(1),
                xp_cost: 5,
            },
            CrucibleNode {
                name: "Stabilizer Mod",
                description: "+1 focus regen per turn",
                effect: CrucibleEffect::FocusRegen(1),
                xp_cost: 15,
            },
            CrucibleNode {
                name: "Neural Interface",
                description: "+2 bonus damage on hard answers",
                effect: CrucibleEffect::HardAnswerDamage(2),
                xp_cost: 30,
            },
            CrucibleNode {
                name: "Double-Tap Module",
                description: "10% chance to strike twice",
                effect: CrucibleEffect::DoubleStrike(10),
                xp_cost: 50,
            },
            CrucibleNode {
                name: "Momentum Cache",
                description: "Wrong answers only halve combo instead of resetting",
                effect: CrucibleEffect::ComboExtender,
                xp_cost: 50,
            },
        ],
    },
    // ─── Weapon 2: Sustain Configuration ─────────────────────────────────────
    CrucibleTemplate {
        name: "Sustain Configuration",
        flavor: "Attachment",
        nodes: [
            CrucibleNode {
                name: "Vampiric Edge",
                description: "Heal 1 HP per kill",
                effect: CrucibleEffect::HealOnKill(1),
                xp_cost: 5,
            },
            CrucibleNode {
                name: "Bio-Feedback Loop",
                description: "+1 lifesteal on attacks",
                effect: CrucibleEffect::LifeSteal(1),
                xp_cost: 15,
            },
            CrucibleNode {
                name: "Nano-Recovery Shell",
                description: "+3 max HP",
                effect: CrucibleEffect::MaxHp(3),
                xp_cost: 30,
            },
            CrucibleNode {
                name: "Emergency Repair Protocol",
                description: "Auto-heal 3 HP once per fight when below 25%",
                effect: CrucibleEffect::EmergencyRepair(3),
                xp_cost: 50,
            },
            CrucibleNode {
                name: "Kill Shield",
                description: "Gain a shield after each kill",
                effect: CrucibleEffect::ShieldOnKill,
                xp_cost: 50,
            },
        ],
    },
    // ─── Armor 0: Bulwark Plating ────────────────────────────────────────────
    CrucibleTemplate {
        name: "Bulwark Plating",
        flavor: "Plating",
        nodes: [
            CrucibleNode {
                name: "Reinforced Layer",
                description: "+1 armor",
                effect: CrucibleEffect::BonusArmor(1),
                xp_cost: 5,
            },
            CrucibleNode {
                name: "Impact Absorber",
                description: "+3 max HP",
                effect: CrucibleEffect::MaxHp(3),
                xp_cost: 15,
            },
            CrucibleNode {
                name: "Hardened Core",
                description: "5% dodge chance",
                effect: CrucibleEffect::DodgeChance(5),
                xp_cost: 30,
            },
            CrucibleNode {
                name: "Kinetic Battery",
                description: "Store 50% damage taken, add to next attack",
                effect: CrucibleEffect::KineticAbsorber,
                xp_cost: 50,
            },
            CrucibleNode {
                name: "Emergency Vent",
                description: "Auto-heal 4 HP once per fight when below 25%",
                effect: CrucibleEffect::EmergencyRepair(4),
                xp_cost: 50,
            },
        ],
    },
    // ─── Armor 1: Reactive Weave ─────────────────────────────────────────────
    CrucibleTemplate {
        name: "Reactive Weave",
        flavor: "Plating",
        nodes: [
            CrucibleNode {
                name: "Shock Mesh",
                description: "Attackers are burned (1 dmg/turn, 2 turns)",
                effect: CrucibleEffect::BurnOnHit { damage: 1, turns: 2 },
                xp_cost: 5,
            },
            CrucibleNode {
                name: "Counter Pulse",
                description: "+1 bonus damage",
                effect: CrucibleEffect::BonusDamage(1),
                xp_cost: 15,
            },
            CrucibleNode {
                name: "Adaptive Layer",
                description: "5% dodge chance",
                effect: CrucibleEffect::DodgeChance(5),
                xp_cost: 30,
            },
            CrucibleNode {
                name: "Temporal Shift",
                description: "10% chance for bonus action after kill",
                effect: CrucibleEffect::TemporalFlux,
                xp_cost: 50,
            },
            CrucibleNode {
                name: "Scholar's Mend",
                description: "Heal 2 HP on hard answers",
                effect: CrucibleEffect::HardAnswerHeal(2),
                xp_cost: 50,
            },
        ],
    },
    // ─── Armor 2: Mobility Frame ─────────────────────────────────────────────
    CrucibleTemplate {
        name: "Mobility Frame",
        flavor: "Plating",
        nodes: [
            CrucibleNode {
                name: "Servo Assist",
                description: "+1 movement in combat",
                effect: CrucibleEffect::MovementBonus(1),
                xp_cost: 5,
            },
            CrucibleNode {
                name: "Reflex Boost",
                description: "5% dodge chance",
                effect: CrucibleEffect::DodgeChance(5),
                xp_cost: 15,
            },
            CrucibleNode {
                name: "Neural Accelerator",
                description: "+1 focus regen per turn",
                effect: CrucibleEffect::FocusRegen(1),
                xp_cost: 30,
            },
            CrucibleNode {
                name: "Flow State",
                description: "Wrong answers only halve combo instead of resetting",
                effect: CrucibleEffect::ComboExtender,
                xp_cost: 50,
            },
            CrucibleNode {
                name: "Scholar's Edge",
                description: "+2 bonus damage on hard answers",
                effect: CrucibleEffect::HardAnswerDamage(2),
                xp_cost: 50,
            },
        ],
    },
    // ─── Module 0: Combat Firmware ───────────────────────────────────────────
    CrucibleTemplate {
        name: "Combat Firmware",
        flavor: "Firmware",
        nodes: [
            CrucibleNode {
                name: "Power Optimizer",
                description: "+1 spell power",
                effect: CrucibleEffect::SpellPower(1),
                xp_cost: 5,
            },
            CrucibleNode {
                name: "Focus Capacitor",
                description: "+1 focus regen per turn",
                effect: CrucibleEffect::FocusRegen(1),
                xp_cost: 15,
            },
            CrucibleNode {
                name: "Resonance Amp",
                description: "10% critical hit chance",
                effect: CrucibleEffect::CritChance(10),
                xp_cost: 30,
            },
            CrucibleNode {
                name: "Neural Sync",
                description: "SRS records correct answers twice (learn faster)",
                effect: CrucibleEffect::NeuralSync,
                xp_cost: 50,
            },
            CrucibleNode {
                name: "Surge Capacitor",
                description: "15% chance for +50% damage",
                effect: CrucibleEffect::OverchargeProc,
                xp_cost: 50,
            },
        ],
    },
    // ─── Module 1: Salvage Firmware ──────────────────────────────────────────
    CrucibleTemplate {
        name: "Salvage Firmware",
        flavor: "Firmware",
        nodes: [
            CrucibleNode {
                name: "Loot Scanner",
                description: "+15% gold from kills",
                effect: CrucibleEffect::GoldFind(15),
                xp_cost: 5,
            },
            CrucibleNode {
                name: "Radical Detector",
                description: "+15% radical drop chance",
                effect: CrucibleEffect::RadicalFind(15),
                xp_cost: 15,
            },
            CrucibleNode {
                name: "Economy Core",
                description: "Heal 1 HP per kill",
                effect: CrucibleEffect::HealOnKill(1),
                xp_cost: 30,
            },
            CrucibleNode {
                name: "Double-Tap Circuit",
                description: "10% chance to strike twice",
                effect: CrucibleEffect::DoubleStrike(10),
                xp_cost: 50,
            },
            CrucibleNode {
                name: "Focus Siphon",
                description: "Restore 3 focus per kill",
                effect: CrucibleEffect::FocusOnKill(3),
                xp_cost: 50,
            },
        ],
    },
    // ─── Module 2: Survival Firmware ─────────────────────────────────────────
    CrucibleTemplate {
        name: "Survival Firmware",
        flavor: "Firmware",
        nodes: [
            CrucibleNode {
                name: "Shield Capacitor",
                description: "+1 armor",
                effect: CrucibleEffect::BonusArmor(1),
                xp_cost: 5,
            },
            CrucibleNode {
                name: "Repair Nanites",
                description: "+2 max HP",
                effect: CrucibleEffect::MaxHp(2),
                xp_cost: 15,
            },
            CrucibleNode {
                name: "Defense Matrix",
                description: "Heal 1 HP on hard answers",
                effect: CrucibleEffect::HardAnswerHeal(1),
                xp_cost: 30,
            },
            CrucibleNode {
                name: "Last Stand Protocol",
                description: "Auto-heal 3 HP once per fight when below 25%",
                effect: CrucibleEffect::EmergencyRepair(3),
                xp_cost: 50,
            },
            CrucibleNode {
                name: "Victory Shield",
                description: "Gain a shield after each kill",
                effect: CrucibleEffect::ShieldOnKill,
                xp_cost: 50,
            },
        ],
    },
];

// ── Runtime State ────────────────────────────────────────────────────────────

/// Per-equipment crucible tree state.
#[derive(Clone, Debug)]
pub struct CrucibleState {
    /// Index into `CRUCIBLE_TEMPLATES`.
    pub tree_idx: usize,
    /// Accumulated XP on this equipment piece.
    pub xp: u32,
    /// Which of the 5 nodes are unlocked.
    pub unlocked: [bool; 5],
    /// Branch choice: `None` = not chosen yet, `Some(true)` = node 3,
    /// `Some(false)` = node 4.
    pub branch_chosen: Option<bool>,
    /// Whether emergency repair has fired this fight.
    #[allow(dead_code)]
    pub emergency_used: bool,
    /// Kinetic absorber stored damage.
    #[allow(dead_code)]
    pub kinetic_stored: i32,
}

impl CrucibleState {
    pub fn new(tree_idx: usize) -> Self {
        Self {
            tree_idx: tree_idx.min(CRUCIBLE_TEMPLATES.len() - 1),
            xp: 0,
            unlocked: [false; 5],
            branch_chosen: None,
            emergency_used: false,
            kinetic_stored: 0,
        }
    }

    /// An empty/inactive state (no tree assigned).
    pub fn empty() -> Self {
        Self::new(0)
    }

    /// Create a crucible state for a specific equipment piece.
    pub fn for_equipment(equipment: &crate::player::Equipment) -> Self {
        let idx = tree_for_equipment(equipment.slot, equipment.name);
        Self::new(idx)
    }

    pub fn template(&self) -> &'static CrucibleTemplate {
        &CRUCIBLE_TEMPLATES[self.tree_idx]
    }

    /// Add XP and auto-unlock linear trunk nodes (0, 1, 2).
    pub fn gain_xp(&mut self, amount: u32) {
        self.xp += amount;
        let t = self.template();
        for i in 0..3 {
            if !self.unlocked[i] && self.xp >= t.nodes[i].xp_cost {
                self.unlocked[i] = true;
            }
        }
        // Branch nodes require explicit choice — don't auto-unlock.
    }

    /// True if the branch is available but not yet chosen.
    pub fn pending_branch(&self) -> bool {
        self.unlocked[2]
            && self.branch_chosen.is_none()
            && self.xp >= self.template().nodes[3].xp_cost
    }

    /// Choose left (node 3) or right (node 4) branch. Irreversible.
    pub fn choose_branch(&mut self, left: bool) {
        if self.branch_chosen.is_some() {
            return;
        }
        self.branch_chosen = Some(left);
        if left {
            self.unlocked[3] = true;
        } else {
            self.unlocked[4] = true;
        }
    }

    /// Number of unlocked nodes.
    #[allow(dead_code)]
    pub fn unlocked_count(&self) -> usize {
        self.unlocked.iter().filter(|&&u| u).count()
    }

    /// XP needed for the next locked trunk node, or the branch if trunk is done.
    pub fn xp_to_next(&self) -> Option<u32> {
        let t = self.template();
        for i in 0..3 {
            if !self.unlocked[i] {
                return Some(t.nodes[i].xp_cost.saturating_sub(self.xp));
            }
        }
        if self.branch_chosen.is_none() && self.xp < t.nodes[3].xp_cost {
            return Some(t.nodes[3].xp_cost.saturating_sub(self.xp));
        }
        None // fully unlocked
    }

    /// Collect all active effects from unlocked nodes.
    pub fn active_effects(&self) -> Vec<CrucibleEffect> {
        let t = self.template();
        self.unlocked
            .iter()
            .enumerate()
            .filter(|(_, &u)| u)
            .map(|(i, _)| t.nodes[i].effect)
            .collect()
    }

    /// Reset per-fight state (emergency repair, kinetic absorber).
    #[allow(dead_code)]
    pub fn reset_fight(&mut self) {
        self.emergency_used = false;
        self.kinetic_stored = 0;
    }

    /// Serialize to a compact JSON string for localStorage.
    pub fn to_json(&self) -> String {
        let mut s = String::from("{\"t\":");
        s.push_str(&self.tree_idx.to_string());
        s.push_str(",\"x\":");
        s.push_str(&self.xp.to_string());
        s.push_str(",\"u\":[");
        for (i, &u) in self.unlocked.iter().enumerate() {
            if i > 0 { s.push(','); }
            s.push(if u { '1' } else { '0' });
        }
        s.push_str("],\"b\":");
        match self.branch_chosen {
            None => s.push_str("null"),
            Some(true) => s.push_str("true"),
            Some(false) => s.push_str("false"),
        }
        s.push('}');
        s
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Self {
        let mut state = Self::empty();
        if let Some(t_pos) = json.find("\"t\":") {
            let rest = &json[t_pos + 4..];
            let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
            state.tree_idx = rest[..end].parse().unwrap_or(0);
            if state.tree_idx >= CRUCIBLE_TEMPLATES.len() {
                state.tree_idx = 0;
            }
        }
        if let Some(x_pos) = json.find("\"x\":") {
            let rest = &json[x_pos + 4..];
            let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
            state.xp = rest[..end].parse().unwrap_or(0);
        }
        if let Some(u_pos) = json.find("\"u\":[") {
            let rest = &json[u_pos + 5..];
            if let Some(end) = rest.find(']') {
                let nums: Vec<&str> = rest[..end].split(',').collect();
                for (i, n) in nums.iter().enumerate().take(5) {
                    state.unlocked[i] = n.trim() == "1" || n.trim() == "true";
                }
            }
        }
        if let Some(b_pos) = json.find("\"b\":") {
            let rest = &json[b_pos + 4..];
            if rest.starts_with("true") {
                state.branch_chosen = Some(true);
            } else if rest.starts_with("false") {
                state.branch_chosen = Some(false);
            }
        }
        state
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Deterministically assign a tree template index based on equipment slot and name.
pub fn tree_for_equipment(slot: EquipSlot, name: &str) -> usize {
    let offset = match slot {
        EquipSlot::Weapon => 0,
        EquipSlot::Armor => 3,
        EquipSlot::Charm => 6,
    };
    let hash = name.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    offset + (hash as usize % 3)
}

// ── Aggregate helpers (used by combat systems) ───────────────────────────────

/// Sum a numeric effect across multiple crucible states.
pub fn aggregate_bonus_damage(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::BonusDamage(n) => Some(n),
        _ => None,
    }).sum()
}

pub fn aggregate_bonus_armor(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::BonusArmor(n) => Some(n),
        _ => None,
    }).sum()
}

pub fn aggregate_max_hp(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::MaxHp(n) => Some(n),
        _ => None,
    }).sum()
}

pub fn aggregate_crit_chance(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::CritChance(n) => Some(n),
        _ => None,
    }).sum()
}

#[allow(dead_code)]
pub fn aggregate_spell_power(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::SpellPower(n) => Some(n),
        _ => None,
    }).sum()
}

pub fn aggregate_dodge_chance(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::DodgeChance(n) => Some(n),
        _ => None,
    }).sum()
}

pub fn aggregate_gold_find(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::GoldFind(n) => Some(n),
        _ => None,
    }).sum()
}

pub fn aggregate_radical_find(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::RadicalFind(n) => Some(n),
        _ => None,
    }).sum()
}

#[allow(dead_code)]
pub fn aggregate_focus_regen(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::FocusRegen(n) => Some(n),
        _ => None,
    }).sum()
}

pub fn aggregate_lifesteal(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::LifeSteal(n) => Some(n),
        _ => None,
    }).sum()
}

pub fn aggregate_heal_on_kill(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::HealOnKill(n) => Some(n),
        _ => None,
    }).sum()
}

pub fn aggregate_armor_pierce(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::ArmorPierce(n) => Some(n),
        _ => None,
    }).sum()
}

pub fn aggregate_hard_answer_damage(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::HardAnswerDamage(n) => Some(n),
        _ => None,
    }).sum()
}

pub fn aggregate_hard_answer_heal(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::HardAnswerHeal(n) => Some(n),
        _ => None,
    }).sum()
}

pub fn aggregate_double_strike(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::DoubleStrike(n) => Some(n),
        _ => None,
    }).sum()
}

#[allow(dead_code)]
pub fn aggregate_movement_bonus(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::MovementBonus(n) => Some(n),
        _ => None,
    }).sum()
}

#[allow(dead_code)]
pub fn aggregate_focus_on_kill(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::FocusOnKill(n) => Some(n),
        _ => None,
    }).sum()
}

/// Check if any crucible has a specific flag-type effect.
#[allow(dead_code)]
pub fn has_effect(states: &[&CrucibleState], check: fn(&CrucibleEffect) -> bool) -> bool {
    states.iter().flat_map(|s| s.active_effects()).any(|e| check(&e))
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
    states.iter().flat_map(|s| s.active_effects()).find_map(|e| match e {
        CrucibleEffect::BurnOnHit { damage, turns } => Some((damage, turns)),
        _ => None,
    })
}

/// Get poison-on-hit params if any crucible has it.
#[allow(dead_code)]
pub fn poison_on_hit(states: &[&CrucibleState]) -> Option<(i32, i32)> {
    states.iter().flat_map(|s| s.active_effects()).find_map(|e| match e {
        CrucibleEffect::PoisonOnHit { damage, turns } => Some((damage, turns)),
        _ => None,
    })
}

/// Get emergency repair threshold if any crucible has it (returns heal amount).
pub fn emergency_repair_amount(states: &[&CrucibleState]) -> i32 {
    states.iter().flat_map(|s| s.active_effects()).filter_map(|e| match e {
        CrucibleEffect::EmergencyRepair(n) => Some(n),
        _ => None,
    }).sum()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_assignment_deterministic() {
        let t1 = tree_for_equipment(EquipSlot::Weapon, "Laser Pistol");
        let t2 = tree_for_equipment(EquipSlot::Weapon, "Laser Pistol");
        assert_eq!(t1, t2);
        assert!(t1 < 3); // weapon trees are 0-2
    }

    #[test]
    fn tree_assignment_slot_ranges() {
        let w = tree_for_equipment(EquipSlot::Weapon, "Test");
        assert!(w < 3);
        let a = tree_for_equipment(EquipSlot::Armor, "Test");
        assert!((3..6).contains(&a));
        let c = tree_for_equipment(EquipSlot::Charm, "Test");
        assert!((6..9).contains(&c));
    }

    #[test]
    fn xp_auto_unlocks_trunk() {
        let mut s = CrucibleState::new(0);
        assert!(!s.unlocked[0]);
        s.gain_xp(5);
        assert!(s.unlocked[0]);
        assert!(!s.unlocked[1]);
        s.gain_xp(10);
        assert!(s.unlocked[1]);
        s.gain_xp(15);
        assert!(s.unlocked[2]);
        // Branch not auto-unlocked
        assert!(!s.unlocked[3]);
        assert!(!s.unlocked[4]);
    }

    #[test]
    fn branch_choice_is_exclusive() {
        let mut s = CrucibleState::new(0);
        s.xp = 50;
        for i in 0..3 { s.unlocked[i] = true; }
        assert!(s.pending_branch());
        s.choose_branch(true);
        assert!(s.unlocked[3]);
        assert!(!s.unlocked[4]);
        assert!(!s.pending_branch());
    }

    #[test]
    fn serialization_roundtrip() {
        let mut s = CrucibleState::new(2);
        s.xp = 42;
        s.unlocked = [true, true, false, false, false];
        s.branch_chosen = Some(false);
        let json = s.to_json();
        let s2 = CrucibleState::from_json(&json);
        assert_eq!(s2.tree_idx, 2);
        assert_eq!(s2.xp, 42);
        assert_eq!(s2.unlocked, [true, true, false, false, false]);
        assert_eq!(s2.branch_chosen, Some(false));
    }
}
