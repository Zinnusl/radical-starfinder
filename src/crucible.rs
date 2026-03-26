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
                description: "+2 bonus damage",
                effect: CrucibleEffect::BonusDamage(2),
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
                description: "Heal 2 HP per kill",
                effect: CrucibleEffect::HealOnKill(2),
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
                description: "+3 max HP",
                effect: CrucibleEffect::MaxHp(3),
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

    // ── Helper: build a state with specific tree and unlocked trunk + optional branch ──

    fn state_with_trunk(tree_idx: usize) -> CrucibleState {
        let mut s = CrucibleState::new(tree_idx);
        s.xp = 50;
        s.unlocked = [true, true, true, false, false];
        s
    }

    fn state_with_left_branch(tree_idx: usize) -> CrucibleState {
        let mut s = state_with_trunk(tree_idx);
        s.choose_branch(true);
        s
    }

    fn state_with_right_branch(tree_idx: usize) -> CrucibleState {
        let mut s = state_with_trunk(tree_idx);
        s.choose_branch(false);
        s
    }

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
        assert_eq!(CrucibleEffect::BurnOnHit { damage: 1, turns: 3 }.short_label(), "🔥Hit");
        assert_eq!(CrucibleEffect::PoisonOnHit { damage: 2, turns: 4 }.short_label(), "☠Hit");
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

    // ── unlocked_count tests ──

    #[test]
    fn unlocked_count_none() {
        let s = CrucibleState::new(0);
        assert_eq!(s.unlocked_count(), 0);
    }

    #[test]
    fn unlocked_count_trunk_only() {
        let s = state_with_trunk(0);
        assert_eq!(s.unlocked_count(), 3);
    }

    #[test]
    fn unlocked_count_with_branch() {
        let s = state_with_left_branch(0);
        assert_eq!(s.unlocked_count(), 4);
    }

    // ── xp_to_next tests ──

    #[test]
    fn xp_to_next_fresh_state() {
        let s = CrucibleState::new(0);
        // Tree 0 node 0 costs 5 XP, we have 0
        assert_eq!(s.xp_to_next(), Some(5));
    }

    #[test]
    fn xp_to_next_after_first_unlock() {
        let mut s = CrucibleState::new(0);
        s.gain_xp(5);
        // Node 1 costs 15 XP, we have 5
        assert_eq!(s.xp_to_next(), Some(10));
    }

    #[test]
    fn xp_to_next_trunk_done_branch_not_ready() {
        let mut s = CrucibleState::new(0);
        s.xp = 35;
        s.unlocked = [true, true, true, false, false];
        // Branch costs 50, we have 35
        assert_eq!(s.xp_to_next(), Some(15));
    }

    #[test]
    fn xp_to_next_fully_unlocked() {
        let s = state_with_left_branch(0);
        assert_eq!(s.xp_to_next(), None);
    }

    #[test]
    fn xp_to_next_branch_chosen_returns_none() {
        let mut s = state_with_trunk(0);
        s.branch_chosen = Some(true);
        s.unlocked[3] = true;
        assert_eq!(s.xp_to_next(), None);
    }

    // ── active_effects tests ──

    #[test]
    fn active_effects_empty_when_nothing_unlocked() {
        let s = CrucibleState::new(0);
        assert!(s.active_effects().is_empty());
    }

    #[test]
    fn active_effects_trunk_only() {
        let s = state_with_trunk(0);
        let effects = s.active_effects();
        assert_eq!(effects.len(), 3);
    }

    #[test]
    fn active_effects_with_branch() {
        let s = state_with_left_branch(0);
        let effects = s.active_effects();
        assert_eq!(effects.len(), 4);
    }

    #[test]
    fn active_effects_single_node() {
        let mut s = CrucibleState::new(0);
        s.unlocked[0] = true;
        let effects = s.active_effects();
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].short_label(), "+Dmg");
    }

    // ── reset_fight tests ──

    #[test]
    fn reset_fight_clears_emergency_and_kinetic() {
        let mut s = CrucibleState::new(0);
        s.emergency_used = true;
        s.kinetic_stored = 42;
        s.reset_fight();
        assert!(!s.emergency_used);
        assert_eq!(s.kinetic_stored, 0);
    }

    #[test]
    fn reset_fight_preserves_other_state() {
        let mut s = state_with_left_branch(0);
        s.emergency_used = true;
        s.kinetic_stored = 10;
        s.reset_fight();
        assert_eq!(s.unlocked_count(), 4);
        assert_eq!(s.xp, 50);
        assert_eq!(s.branch_chosen, Some(true));
    }

    // ── choose_branch edge cases ──

    #[test]
    fn choose_branch_right_unlocks_node_4() {
        let s = state_with_right_branch(0);
        assert!(!s.unlocked[3]);
        assert!(s.unlocked[4]);
        assert_eq!(s.branch_chosen, Some(false));
    }

    #[test]
    fn choose_branch_second_call_ignored() {
        let mut s = state_with_left_branch(0);
        assert!(s.unlocked[3]);
        assert!(!s.unlocked[4]);
        // Try choosing right — should be ignored
        s.choose_branch(false);
        assert!(s.unlocked[3]);
        assert!(!s.unlocked[4]);
        assert_eq!(s.branch_chosen, Some(true));
    }

    // ── from_json edge cases ──

    #[test]
    fn from_json_invalid_json_returns_empty() {
        let s = CrucibleState::from_json("not valid json at all");
        assert_eq!(s.tree_idx, 0);
        assert_eq!(s.xp, 0);
        assert_eq!(s.unlocked, [false; 5]);
        assert_eq!(s.branch_chosen, None);
    }

    #[test]
    fn from_json_empty_string() {
        let s = CrucibleState::from_json("");
        assert_eq!(s.tree_idx, 0);
        assert_eq!(s.xp, 0);
    }

    #[test]
    fn from_json_out_of_range_tree_clamps() {
        let json = r#"{"t":999,"x":10,"u":[1,0,0,0,0],"b":null}"#;
        let s = CrucibleState::from_json(json);
        assert_eq!(s.tree_idx, 0); // clamped to 0 for out-of-range
    }

    #[test]
    fn from_json_partial_fields() {
        let json = r#"{"t":1,"x":25}"#;
        let s = CrucibleState::from_json(json);
        assert_eq!(s.tree_idx, 1);
        assert_eq!(s.xp, 25);
        assert_eq!(s.unlocked, [false; 5]);
        assert_eq!(s.branch_chosen, None);
    }

    #[test]
    fn from_json_branch_true_false_null() {
        let json_true = r#"{"t":0,"x":0,"u":[0,0,0,0,0],"b":true}"#;
        assert_eq!(CrucibleState::from_json(json_true).branch_chosen, Some(true));

        let json_false = r#"{"t":0,"x":0,"u":[0,0,0,0,0],"b":false}"#;
        assert_eq!(CrucibleState::from_json(json_false).branch_chosen, Some(false));

        let json_null = r#"{"t":0,"x":0,"u":[0,0,0,0,0],"b":null}"#;
        assert_eq!(CrucibleState::from_json(json_null).branch_chosen, None);
    }

    // ── aggregate function tests ──

    #[test]
    fn aggregate_bonus_damage_sums_across_states() {
        // Tree 0 node 0: BonusDamage(1), Tree 4 node 1: BonusDamage(2)
        let s0 = state_with_trunk(0);
        let s4 = state_with_trunk(4);
        assert_eq!(aggregate_bonus_damage(&[&s0, &s4]), 1 + 2);
    }

    #[test]
    fn aggregate_bonus_damage_empty() {
        assert_eq!(aggregate_bonus_damage(&[]), 0);
    }

    #[test]
    fn aggregate_bonus_armor_sums() {
        // Tree 3 node 0: BonusArmor(1), Tree 8 node 0: BonusArmor(1)
        let s3 = state_with_trunk(3);
        let s8 = state_with_trunk(8);
        assert_eq!(aggregate_bonus_armor(&[&s3, &s8]), 2);
    }

    #[test]
    fn aggregate_bonus_armor_empty() {
        assert_eq!(aggregate_bonus_armor(&[]), 0);
    }

    #[test]
    fn aggregate_max_hp_sums() {
        // Tree 2 node 2: MaxHp(3), Tree 3 node 1: MaxHp(3), Tree 8 node 1: MaxHp(3)
        let s2 = state_with_trunk(2);
        let s3 = state_with_trunk(3);
        let s8 = state_with_trunk(8);
        assert_eq!(aggregate_max_hp(&[&s2, &s3, &s8]), 9);
    }

    #[test]
    fn aggregate_max_hp_empty() {
        assert_eq!(aggregate_max_hp(&[]), 0);
    }

    #[test]
    fn aggregate_crit_chance_sums() {
        // Tree 0 node 1: CritChance(10), Tree 6 node 2: CritChance(10)
        let s0 = state_with_trunk(0);
        let s6 = state_with_trunk(6);
        assert_eq!(aggregate_crit_chance(&[&s0, &s6]), 20);
    }

    #[test]
    fn aggregate_crit_chance_empty() {
        assert_eq!(aggregate_crit_chance(&[]), 0);
    }

    #[test]
    fn aggregate_spell_power_sums() {
        // Tree 1 node 0: SpellPower(1), Tree 6 node 0: SpellPower(1)
        let s1 = state_with_trunk(1);
        let s6 = state_with_trunk(6);
        assert_eq!(aggregate_spell_power(&[&s1, &s6]), 2);
    }

    #[test]
    fn aggregate_dodge_chance_sums() {
        // Tree 3 node 2: DodgeChance(5), Tree 4 node 2: DodgeChance(5), Tree 5 node 1: DodgeChance(5)
        let s3 = state_with_trunk(3);
        let s4 = state_with_trunk(4);
        let s5 = state_with_trunk(5);
        assert_eq!(aggregate_dodge_chance(&[&s3, &s4, &s5]), 15);
    }

    #[test]
    fn aggregate_gold_find_sums() {
        // Tree 7 node 0: GoldFind(15)
        let s7 = state_with_trunk(7);
        assert_eq!(aggregate_gold_find(&[&s7]), 15);
    }

    #[test]
    fn aggregate_gold_find_no_match() {
        let s0 = state_with_trunk(0);
        assert_eq!(aggregate_gold_find(&[&s0]), 0);
    }

    #[test]
    fn aggregate_radical_find_sums() {
        // Tree 7 node 1: RadicalFind(15)
        let s7 = state_with_trunk(7);
        assert_eq!(aggregate_radical_find(&[&s7]), 15);
    }

    #[test]
    fn aggregate_focus_regen_sums() {
        // Tree 1 node 1: FocusRegen(1), Tree 5 node 2: FocusRegen(1), Tree 6 node 1: FocusRegen(1)
        let s1 = state_with_trunk(1);
        let s5 = state_with_trunk(5);
        let s6 = state_with_trunk(6);
        assert_eq!(aggregate_focus_regen(&[&s1, &s5, &s6]), 3);
    }

    #[test]
    fn aggregate_lifesteal_sums() {
        // Tree 2 node 1: LifeSteal(1)
        let s2 = state_with_trunk(2);
        assert_eq!(aggregate_lifesteal(&[&s2]), 1);
    }

    #[test]
    fn aggregate_heal_on_kill_sums() {
        // Tree 2 node 0: HealOnKill(1), Tree 7 node 2: HealOnKill(2)
        let s2 = state_with_trunk(2);
        let s7 = state_with_trunk(7);
        assert_eq!(aggregate_heal_on_kill(&[&s2, &s7]), 3);
    }

    #[test]
    fn aggregate_armor_pierce_from_branch() {
        // Tree 0 node 4 (right branch): ArmorPierce(2)
        let s0 = state_with_right_branch(0);
        assert_eq!(aggregate_armor_pierce(&[&s0]), 2);
    }

    #[test]
    fn aggregate_armor_pierce_trunk_only_zero() {
        let s0 = state_with_trunk(0);
        assert_eq!(aggregate_armor_pierce(&[&s0]), 0);
    }

    #[test]
    fn aggregate_hard_answer_damage_sums() {
        // Tree 1 node 2: HardAnswerDamage(2), Tree 5 node 4 (right branch): HardAnswerDamage(2)
        let s1 = state_with_trunk(1);
        let s5 = state_with_right_branch(5);
        assert_eq!(aggregate_hard_answer_damage(&[&s1, &s5]), 4);
    }

    #[test]
    fn aggregate_hard_answer_heal_sums() {
        // Tree 4 node 4 (right branch): HardAnswerHeal(2), Tree 8 node 2: HardAnswerHeal(1)
        let s4 = state_with_right_branch(4);
        let s8 = state_with_trunk(8);
        assert_eq!(aggregate_hard_answer_heal(&[&s4, &s8]), 3);
    }

    #[test]
    fn aggregate_double_strike_sums() {
        // Tree 1 node 3 (left branch): DoubleStrike(10), Tree 7 node 3 (left branch): DoubleStrike(10)
        let s1 = state_with_left_branch(1);
        let s7 = state_with_left_branch(7);
        assert_eq!(aggregate_double_strike(&[&s1, &s7]), 20);
    }

    #[test]
    fn aggregate_movement_bonus_sums() {
        // Tree 5 node 0: MovementBonus(1)
        let s5 = state_with_trunk(5);
        assert_eq!(aggregate_movement_bonus(&[&s5]), 1);
    }

    #[test]
    fn aggregate_focus_on_kill_sums() {
        // Tree 7 node 4 (right branch): FocusOnKill(3)
        let s7 = state_with_right_branch(7);
        assert_eq!(aggregate_focus_on_kill(&[&s7]), 3);
    }

    #[test]
    fn aggregate_focus_on_kill_no_branch_zero() {
        let s7 = state_with_trunk(7);
        assert_eq!(aggregate_focus_on_kill(&[&s7]), 0);
    }

    // ── has_* function tests ──

    #[test]
    fn has_combo_extender_true() {
        // Tree 1 node 4 (right branch): ComboExtender
        let s = state_with_right_branch(1);
        assert!(has_combo_extender(&[&s]));
    }

    #[test]
    fn has_combo_extender_false_trunk_only() {
        let s = state_with_trunk(1);
        assert!(!has_combo_extender(&[&s]));
    }

    #[test]
    fn has_combo_extender_tree5_left_branch() {
        // Tree 5 node 3 (left branch): ComboExtender
        let s = state_with_left_branch(5);
        assert!(has_combo_extender(&[&s]));
    }

    #[test]
    fn has_overcharge_proc_true() {
        // Tree 0 node 2: OverchargeProc
        let s = state_with_trunk(0);
        assert!(has_overcharge_proc(&[&s]));
    }

    #[test]
    fn has_overcharge_proc_false() {
        let s = state_with_trunk(1);
        assert!(!has_overcharge_proc(&[&s]));
    }

    #[test]
    fn has_overcharge_proc_tree6_right_branch() {
        // Tree 6 node 4 (right branch): OverchargeProc
        let s = state_with_right_branch(6);
        assert!(has_overcharge_proc(&[&s]));
    }

    #[test]
    fn has_shield_on_kill_true() {
        // Tree 2 node 4 (right branch): ShieldOnKill
        let s = state_with_right_branch(2);
        assert!(has_shield_on_kill(&[&s]));
    }

    #[test]
    fn has_shield_on_kill_false() {
        let s = state_with_trunk(0);
        assert!(!has_shield_on_kill(&[&s]));
    }

    #[test]
    fn has_shield_on_kill_tree8_right_branch() {
        // Tree 8 node 4 (right branch): ShieldOnKill
        let s = state_with_right_branch(8);
        assert!(has_shield_on_kill(&[&s]));
    }

    #[test]
    fn has_neural_sync_true() {
        // Tree 6 node 3 (left branch): NeuralSync
        let s = state_with_left_branch(6);
        assert!(has_neural_sync(&[&s]));
    }

    #[test]
    fn has_neural_sync_false() {
        let s = state_with_trunk(6);
        assert!(!has_neural_sync(&[&s]));
    }

    #[test]
    fn has_temporal_flux_true() {
        // Tree 4 node 3 (left branch): TemporalFlux
        let s = state_with_left_branch(4);
        assert!(has_temporal_flux(&[&s]));
    }

    #[test]
    fn has_temporal_flux_false() {
        let s = state_with_trunk(4);
        assert!(!has_temporal_flux(&[&s]));
    }

    #[test]
    fn has_kinetic_absorber_true() {
        // Tree 3 node 3 (left branch): KineticAbsorber
        let s = state_with_left_branch(3);
        assert!(has_kinetic_absorber(&[&s]));
    }

    #[test]
    fn has_kinetic_absorber_false() {
        let s = state_with_trunk(3);
        assert!(!has_kinetic_absorber(&[&s]));
    }

    #[test]
    fn has_functions_empty_states() {
        let empty: &[&CrucibleState] = &[];
        assert!(!has_combo_extender(empty));
        assert!(!has_overcharge_proc(empty));
        assert!(!has_shield_on_kill(empty));
        assert!(!has_neural_sync(empty));
        assert!(!has_temporal_flux(empty));
        assert!(!has_kinetic_absorber(empty));
    }

    // ── burn_on_hit tests ──

    #[test]
    fn burn_on_hit_from_tree0_left_branch() {
        // Tree 0 node 3 (left branch): BurnOnHit { damage: 1, turns: 3 }
        let s = state_with_left_branch(0);
        assert_eq!(burn_on_hit(&[&s]), Some((1, 3)));
    }

    #[test]
    fn burn_on_hit_from_tree4_trunk() {
        // Tree 4 node 0: BurnOnHit { damage: 1, turns: 2 }
        let s = state_with_trunk(4);
        assert_eq!(burn_on_hit(&[&s]), Some((1, 2)));
    }

    #[test]
    fn burn_on_hit_none_when_no_effect() {
        let s = state_with_trunk(1);
        assert_eq!(burn_on_hit(&[&s]), None);
    }

    #[test]
    fn burn_on_hit_empty() {
        assert_eq!(burn_on_hit(&[]), None);
    }

    // ── poison_on_hit tests ──

    #[test]
    fn poison_on_hit_none_for_all_trees() {
        // No tree has PoisonOnHit in its nodes
        for i in 0..9 {
            let s = state_with_left_branch(i);
            assert_eq!(poison_on_hit(&[&s]), None, "tree {} left branch", i);
            let s = state_with_right_branch(i);
            assert_eq!(poison_on_hit(&[&s]), None, "tree {} right branch", i);
        }
    }

    #[test]
    fn poison_on_hit_empty() {
        assert_eq!(poison_on_hit(&[]), None);
    }

    // ── emergency_repair_amount tests ──

    #[test]
    fn emergency_repair_amount_from_tree2_left_branch() {
        // Tree 2 node 3 (left branch): EmergencyRepair(3)
        let s = state_with_left_branch(2);
        assert_eq!(emergency_repair_amount(&[&s]), 3);
    }

    #[test]
    fn emergency_repair_amount_from_tree3_right_branch() {
        // Tree 3 node 4 (right branch): EmergencyRepair(4)
        let s = state_with_right_branch(3);
        assert_eq!(emergency_repair_amount(&[&s]), 4);
    }

    #[test]
    fn emergency_repair_amount_from_tree8_left_branch() {
        // Tree 8 node 3 (left branch): EmergencyRepair(3)
        let s = state_with_left_branch(8);
        assert_eq!(emergency_repair_amount(&[&s]), 3);
    }

    #[test]
    fn emergency_repair_amount_stacks_across_states() {
        let s2 = state_with_left_branch(2);
        let s8 = state_with_left_branch(8);
        assert_eq!(emergency_repair_amount(&[&s2, &s8]), 6);
    }

    #[test]
    fn emergency_repair_amount_zero_when_no_effect() {
        let s = state_with_trunk(0);
        assert_eq!(emergency_repair_amount(&[&s]), 0);
    }

    #[test]
    fn emergency_repair_amount_empty() {
        assert_eq!(emergency_repair_amount(&[]), 0);
    }

    // ── CrucibleState::new edge cases ──

    #[test]
    fn new_clamps_tree_idx_to_max() {
        let s = CrucibleState::new(100);
        assert!(s.tree_idx < CRUCIBLE_TEMPLATES.len());
    }

    #[test]
    fn empty_is_tree_zero() {
        let s = CrucibleState::empty();
        assert_eq!(s.tree_idx, 0);
        assert_eq!(s.xp, 0);
        assert_eq!(s.unlocked, [false; 5]);
    }

    // ── pending_branch tests ──

    #[test]
    fn pending_branch_false_trunk_not_done() {
        let mut s = CrucibleState::new(0);
        s.gain_xp(15);
        assert!(!s.pending_branch());
    }

    #[test]
    fn pending_branch_false_insufficient_xp() {
        let mut s = CrucibleState::new(0);
        s.xp = 40;
        s.unlocked = [true, true, true, false, false];
        assert!(!s.pending_branch());
    }

    #[test]
    fn pending_branch_true_when_ready() {
        let s = state_with_trunk(0);
        assert!(s.pending_branch());
    }

    #[test]
    fn pending_branch_false_after_choice() {
        let s = state_with_left_branch(0);
        assert!(!s.pending_branch());
    }

    // ── template tests ──

    #[test]
    fn template_returns_correct_tree() {
        let s = CrucibleState::new(0);
        assert_eq!(s.template().name, "Assault Configuration");

        let s1 = CrucibleState::new(1);
        assert_eq!(s1.template().name, "Precision Configuration");

        let s7 = CrucibleState::new(7);
        assert_eq!(s7.template().name, "Salvage Firmware");
    }

    // ── gain_xp does not auto-unlock branches ──

    #[test]
    fn gain_xp_large_amount_still_no_branch() {
        let mut s = CrucibleState::new(0);
        s.gain_xp(1000);
        assert!(s.unlocked[0]);
        assert!(s.unlocked[1]);
        assert!(s.unlocked[2]);
        assert!(!s.unlocked[3]);
        assert!(!s.unlocked[4]);
        assert!(s.pending_branch());
    }

    // ── Multiple states aggregation ──

    #[test]
    fn aggregate_with_unlocked_state_contributes_nothing() {
        let s = CrucibleState::new(0); // nothing unlocked
        assert_eq!(aggregate_bonus_damage(&[&s]), 0);
    }
}
