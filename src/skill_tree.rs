// skill_tree.rs — POE1-style passive skill tree for Radical Starfinder
//
// 41 nodes: 1 start + 4 clusters × (5 main path + 4 branch = 10 nodes each).
// The player earns XP from combat and correct answers, levels up (triangular
// formula), and spends skill points to allocate nodes adjacent to already-
// allocated ones.

// ---------------------------------------------------------------------------
// Enums & core types
// ---------------------------------------------------------------------------

/// Percentage-based bonuses use integer basis points where 10 means 10%.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SkillEffect {
    // Flat stat bonuses
    BonusDamage(i32),
    BonusArmor(i32),
    MaxHp(i32),
    CritChance(i32),       // percent
    SpellPower(i32),
    MaxFocus(i32),
    FocusRegen(i32),
    DodgeChance(i32),      // percent
    GoldFind(i32),         // percent
    RadicalFind(i32),      // percent
    ItemRarityBonus(i32),  // percent

    // Notable keystones
    Berserker,       // +3 damage, −1 armor
    Executioner,     // +50% damage to enemies below 30% HP
    Polyglot,        // all correct answers count as "hard"
    LinguistsFury,   // combo multiplier starts at 1.15×
    Undying,         // auto-revive once per run with 5 HP
    IronWill,        // −1 damage taken from all sources (min 1)
    MidasTouch,      // gold drops doubled
    RadicalMagnet,   // always drop a radical on enemy kill
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cluster {
    Start,
    Combat,
    Scholar,
    Survival,
    Fortune,
}

impl Cluster {
    pub fn color(&self) -> &'static str {
        match self {
            Cluster::Start    => "#ffffff",
            Cluster::Combat   => "#ff4444",
            Cluster::Scholar  => "#4488ff",
            Cluster::Survival => "#44cc44",
            Cluster::Fortune  => "#ffaa00",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Cluster::Start    => "Start",
            Cluster::Combat   => "Combat",
            Cluster::Scholar  => "Scholar",
            Cluster::Survival => "Survival",
            Cluster::Fortune  => "Fortune",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SkillNode {
    pub name: &'static str,
    pub description: &'static str,
    pub effect: SkillEffect,
    pub is_notable: bool,
    pub cluster: Cluster,
    /// Grid position for rendering (x, y) relative to center.
    pub pos: (i32, i32),
}

pub struct SkillTree {
    pub nodes: &'static [SkillNode],
    /// `edges[i]` lists every node index adjacent to node `i`.
    pub edges: &'static [&'static [usize]],
}

// ---------------------------------------------------------------------------
// XP constants
// ---------------------------------------------------------------------------

pub const XP_NORMAL_KILL: u32 = 10;
pub const XP_ELITE_KILL: u32 = 25;
pub const XP_BOSS_KILL: u32 = 100;
pub const XP_CORRECT_ANSWER: u32 = 5;
pub const XP_HARD_BONUS: u32 = 5;

// ---------------------------------------------------------------------------
// Static tree data — 41 nodes, 0-indexed
// ---------------------------------------------------------------------------
//
// Layout (conceptual):
//   Combat  → extends to the right  (+x)
//   Scholar → extends upward        (−y)
//   Survival→ extends to the left   (−x)
//   Fortune → extends downward      (+y)
//
// Main paths follow the axis; branches veer diagonally.

static NODES: [SkillNode; 41] = [
    // 0 — Start
    SkillNode {
        name: "Origin",
        description: "The center of your potential.",
        effect: SkillEffect::MaxHp(0),
        is_notable: false,
        cluster: Cluster::Start,
        pos: (0, 0),
    },

    // -----------------------------------------------------------------------
    // COMBAT cluster (indices 1–10), extends right (+x)
    // Main path: 0 → 1 → 2 → 3 → 4 → 5
    // Branch:    3 → 6 → 7 → 8 → 9  (diagonal: +x, −y)
    // -----------------------------------------------------------------------
    // 1
    SkillNode {
        name: "Sharpened Edge",
        description: "+1 damage",
        effect: SkillEffect::BonusDamage(1),
        is_notable: false,
        cluster: Cluster::Combat,
        pos: (1, 0),
    },
    // 2
    SkillNode {
        name: "Honed Blade",
        description: "+1 damage",
        effect: SkillEffect::BonusDamage(1),
        is_notable: false,
        cluster: Cluster::Combat,
        pos: (2, 0),
    },
    // 3
    SkillNode {
        name: "Keen Eye",
        description: "+5% critical strike chance",
        effect: SkillEffect::CritChance(5),
        is_notable: false,
        cluster: Cluster::Combat,
        pos: (3, 0),
    },
    // 4
    SkillNode {
        name: "Brutal Force",
        description: "+1 damage",
        effect: SkillEffect::BonusDamage(1),
        is_notable: false,
        cluster: Cluster::Combat,
        pos: (4, 0),
    },
    // 5 — Notable
    SkillNode {
        name: "Berserker",
        description: "+3 damage, −1 armor permanently",
        effect: SkillEffect::Berserker,
        is_notable: true,
        cluster: Cluster::Combat,
        pos: (5, 0),
    },
    // 6  (branch from 3)
    SkillNode {
        name: "Precision",
        description: "+10% critical strike chance",
        effect: SkillEffect::CritChance(10),
        is_notable: false,
        cluster: Cluster::Combat,
        pos: (4, -1),
    },
    // 7
    SkillNode {
        name: "Sharp Senses",
        description: "+5% critical strike chance",
        effect: SkillEffect::CritChance(5),
        is_notable: false,
        cluster: Cluster::Combat,
        pos: (5, -2),
    },
    // 8
    SkillNode {
        name: "Lethal Focus",
        description: "+10% critical strike chance",
        effect: SkillEffect::CritChance(10),
        is_notable: false,
        cluster: Cluster::Combat,
        pos: (6, -3),
    },
    // 9 — Notable
    SkillNode {
        name: "Executioner",
        description: "+50% damage to enemies below 30% HP",
        effect: SkillEffect::Executioner,
        is_notable: true,
        cluster: Cluster::Combat,
        pos: (7, -4),
    },
    // 10 — placeholder (unused, keeps cluster at 10 nodes)
    // Actually we need exactly 10 nodes per cluster. Let's reconsider:
    // Main path 5 nodes (1-5) + branch 4 nodes (6-9) = 9 per cluster.
    // But the spec says indices 1-10 for combat. That's 10 nodes.
    // Re-reading: "5 nodes (4 small + 1 notable) plus a branch of 4 nodes
    // (3 small + 1 notable)" = 9 per cluster, total 4×9 + 1 start = 37.
    // The spec says ~41. Let's add a connector node (index 10) between
    // start and each cluster to reach 41 = 1 + 4×10.
    // Actually the spec says indices 1-10 for combat (10 nodes).
    // Let me just add one extra small node to each main path to get 10.
    // Re-reading more carefully:
    //   Main path: start → small → small → small → small → Notable = 5 nodes
    //   Branch at node 3: → small → small → small → Notable = 4 nodes
    //   Total per cluster = 9, so 4×9+1=37, but spec says ~41.
    //   With 10 per cluster: 4×10+1=41. Need one more node per cluster.
    // I'll insert an extra small node in each branch to make 5+5=10.
    //
    // Revised Combat branch: 3 → 6 → 7 → 8 → 9 → 10(Notable)
    // That makes branch 5 nodes. But spec says branch = 3 small + 1 notable.
    // The simplest fix: use index 10 as an extra small on the main path.
    // New main: 0 → 1 → 2 → 3 → 4 → 10 → 5(Notable) — 6 main nodes,
    // but that doesn't match the spec topology either.
    //
    // Let me just follow the spec literally and add a bridge node between
    // the start and each cluster's first node. That gives exactly 41.
    //
    // Actually, let me just recount. We want indices 1–10 (10 nodes).
    // I'll do:  main path 6 nodes (5 small + 1 notable),
    //           branch    4 nodes (3 small + 1 notable) = 10.
    // Adjusted main path: start → 1(+1d) → 2(+1d) → 3(+5%c) → 4(+1d) → 5(+1d) → 6(Berserker)
    // Wait that changes the spec. Let me just keep it as specified and
    // not worry about the index range being 1-9 vs 1-10.
    //
    // Simplest: re-purpose index 10 as an extra bridge node.
    SkillNode {
        name: "Warrior's Path",
        description: "+1 damage",
        effect: SkillEffect::BonusDamage(1),
        is_notable: false,
        cluster: Cluster::Combat,
        pos: (6, -1),
    },

    // -----------------------------------------------------------------------
    // SCHOLAR cluster (indices 11–20), extends upward (−y)
    // Main path: 0 → 11 → 12 → 13 → 14 → 15
    // Branch:    13 → 16 → 17 → 18 → 19
    // Extra:     20 (bridge / extra small)
    // -----------------------------------------------------------------------
    // 11
    SkillNode {
        name: "Arcane Initiate",
        description: "+1 spell power",
        effect: SkillEffect::SpellPower(1),
        is_notable: false,
        cluster: Cluster::Scholar,
        pos: (0, -1),
    },
    // 12
    SkillNode {
        name: "Mental Fortitude",
        description: "+5 max focus",
        effect: SkillEffect::MaxFocus(5),
        is_notable: false,
        cluster: Cluster::Scholar,
        pos: (0, -2),
    },
    // 13
    SkillNode {
        name: "Deeper Study",
        description: "+1 spell power",
        effect: SkillEffect::SpellPower(1),
        is_notable: false,
        cluster: Cluster::Scholar,
        pos: (0, -3),
    },
    // 14
    SkillNode {
        name: "Inner Flow",
        description: "+1 focus regen",
        effect: SkillEffect::FocusRegen(1),
        is_notable: false,
        cluster: Cluster::Scholar,
        pos: (0, -4),
    },
    // 15 — Notable
    SkillNode {
        name: "Polyglot",
        description: "All correct answers count as \"hard\" for riposte/bonus purposes",
        effect: SkillEffect::Polyglot,
        is_notable: true,
        cluster: Cluster::Scholar,
        pos: (0, -5),
    },
    // 16 (branch from 13)
    SkillNode {
        name: "Glyph Adept",
        description: "+1 spell power",
        effect: SkillEffect::SpellPower(1),
        is_notable: false,
        cluster: Cluster::Scholar,
        pos: (1, -4),
    },
    // 17
    SkillNode {
        name: "Expanded Mind",
        description: "+5 max focus",
        effect: SkillEffect::MaxFocus(5),
        is_notable: false,
        cluster: Cluster::Scholar,
        pos: (2, -5),
    },
    // 18
    SkillNode {
        name: "Regenerative Thought",
        description: "+1 focus regen",
        effect: SkillEffect::FocusRegen(1),
        is_notable: false,
        cluster: Cluster::Scholar,
        pos: (3, -6),
    },
    // 19 — Notable
    SkillNode {
        name: "Linguist's Fury",
        description: "Combo multiplier starts at 1.15× instead of 1.0×",
        effect: SkillEffect::LinguistsFury,
        is_notable: true,
        cluster: Cluster::Scholar,
        pos: (4, -7),
    },
    // 20 — extra small
    SkillNode {
        name: "Scholar's Insight",
        description: "+1 spell power",
        effect: SkillEffect::SpellPower(1),
        is_notable: false,
        cluster: Cluster::Scholar,
        pos: (3, -4),
    },

    // -----------------------------------------------------------------------
    // SURVIVAL cluster (indices 21–30), extends left (−x)
    // Main path: 0 → 21 → 22 → 23 → 24 → 25
    // Branch:    23 → 26 → 27 → 28 → 29
    // Extra:     30
    // -----------------------------------------------------------------------
    // 21
    SkillNode {
        name: "Tough Skin",
        description: "+3 max HP",
        effect: SkillEffect::MaxHp(3),
        is_notable: false,
        cluster: Cluster::Survival,
        pos: (-1, 0),
    },
    // 22
    SkillNode {
        name: "Hardened Shell",
        description: "+1 armor",
        effect: SkillEffect::BonusArmor(1),
        is_notable: false,
        cluster: Cluster::Survival,
        pos: (-2, 0),
    },
    // 23
    SkillNode {
        name: "Vital Reserves",
        description: "+3 max HP",
        effect: SkillEffect::MaxHp(3),
        is_notable: false,
        cluster: Cluster::Survival,
        pos: (-3, 0),
    },
    // 24
    SkillNode {
        name: "Nimble Feet",
        description: "+5% dodge chance",
        effect: SkillEffect::DodgeChance(5),
        is_notable: false,
        cluster: Cluster::Survival,
        pos: (-4, 0),
    },
    // 25 — Notable
    SkillNode {
        name: "Undying",
        description: "Auto-revive once per run with 5 HP when you would die",
        effect: SkillEffect::Undying,
        is_notable: true,
        cluster: Cluster::Survival,
        pos: (-5, 0),
    },
    // 26 (branch from 23)
    SkillNode {
        name: "Endurance",
        description: "+3 max HP",
        effect: SkillEffect::MaxHp(3),
        is_notable: false,
        cluster: Cluster::Survival,
        pos: (-4, 1),
    },
    // 27
    SkillNode {
        name: "Thick Plating",
        description: "+1 armor",
        effect: SkillEffect::BonusArmor(1),
        is_notable: false,
        cluster: Cluster::Survival,
        pos: (-5, 2),
    },
    // 28
    SkillNode {
        name: "Evasive Maneuvers",
        description: "+5% dodge chance",
        effect: SkillEffect::DodgeChance(5),
        is_notable: false,
        cluster: Cluster::Survival,
        pos: (-6, 3),
    },
    // 29 — Notable
    SkillNode {
        name: "Iron Will",
        description: "Take 1 less damage from all sources (minimum 1)",
        effect: SkillEffect::IronWill,
        is_notable: true,
        cluster: Cluster::Survival,
        pos: (-7, 4),
    },
    // 30 — extra small
    SkillNode {
        name: "Survivor's Grit",
        description: "+3 max HP",
        effect: SkillEffect::MaxHp(3),
        is_notable: false,
        cluster: Cluster::Survival,
        pos: (-6, 1),
    },

    // -----------------------------------------------------------------------
    // FORTUNE cluster (indices 31–40), extends downward (+y)
    // Main path: 0 → 31 → 32 → 33 → 34 → 35
    // Branch:    33 → 36 → 37 → 38 → 39
    // Extra:     40
    // -----------------------------------------------------------------------
    // 31
    SkillNode {
        name: "Lucky Find",
        description: "+10% gold find",
        effect: SkillEffect::GoldFind(10),
        is_notable: false,
        cluster: Cluster::Fortune,
        pos: (0, 1),
    },
    // 32
    SkillNode {
        name: "Radical Sense",
        description: "+10% radical drop rate",
        effect: SkillEffect::RadicalFind(10),
        is_notable: false,
        cluster: Cluster::Fortune,
        pos: (0, 2),
    },
    // 33
    SkillNode {
        name: "Treasure Hunter",
        description: "+10% gold find",
        effect: SkillEffect::GoldFind(10),
        is_notable: false,
        cluster: Cluster::Fortune,
        pos: (0, 3),
    },
    // 34
    SkillNode {
        name: "Connoisseur",
        description: "+5% better item rarity",
        effect: SkillEffect::ItemRarityBonus(5),
        is_notable: false,
        cluster: Cluster::Fortune,
        pos: (0, 4),
    },
    // 35 — Notable
    SkillNode {
        name: "Midas Touch",
        description: "Gold drops doubled",
        effect: SkillEffect::MidasTouch,
        is_notable: true,
        cluster: Cluster::Fortune,
        pos: (0, 5),
    },
    // 36 (branch from 33)
    SkillNode {
        name: "Prospector",
        description: "+10% gold find",
        effect: SkillEffect::GoldFind(10),
        is_notable: false,
        cluster: Cluster::Fortune,
        pos: (-1, 4),
    },
    // 37
    SkillNode {
        name: "Glyph Magnet",
        description: "+10% radical drop rate",
        effect: SkillEffect::RadicalFind(10),
        is_notable: false,
        cluster: Cluster::Fortune,
        pos: (-2, 5),
    },
    // 38
    SkillNode {
        name: "Rare Collector",
        description: "+5% better item rarity",
        effect: SkillEffect::ItemRarityBonus(5),
        is_notable: false,
        cluster: Cluster::Fortune,
        pos: (-3, 6),
    },
    // 39 — Notable
    SkillNode {
        name: "Radical Magnet",
        description: "Always drop a radical on enemy kill",
        effect: SkillEffect::RadicalMagnet,
        is_notable: true,
        cluster: Cluster::Fortune,
        pos: (-4, 7),
    },
    // 40 — extra small
    SkillNode {
        name: "Fortune's Favor",
        description: "+10% gold find",
        effect: SkillEffect::GoldFind(10),
        is_notable: false,
        cluster: Cluster::Fortune,
        pos: (-3, 4),
    },
];

// ---------------------------------------------------------------------------
// Edge list (adjacency). Symmetric — if A connects to B, B connects to A.
// ---------------------------------------------------------------------------

static EDGES: [&[usize]; 41] = [
    // 0  Start — connects to first node of each cluster
    &[1, 11, 21, 31],
    // Combat main path
    &[0, 2],          // 1
    &[1, 3],          // 2
    &[2, 4, 6],       // 3  (branch point → 6)
    &[3, 5],          // 4
    &[4],             // 5  Berserker (terminal)
    // Combat branch
    &[3, 7],          // 6
    &[6, 8],          // 7
    &[7, 9, 10],      // 8
    &[8],             // 9  Executioner (terminal)
    &[8],             // 10 extra small (terminal off 8)
    // Scholar main path
    &[0, 12],         // 11
    &[11, 13],        // 12
    &[12, 14, 16],    // 13 (branch point → 16)
    &[13, 15],        // 14
    &[14],            // 15 Polyglot (terminal)
    // Scholar branch
    &[13, 17],        // 16
    &[16, 18, 20],    // 17
    &[17, 19],        // 18
    &[18],            // 19 Linguist's Fury (terminal)
    &[17],            // 20 extra small (terminal off 17)
    // Survival main path
    &[0, 22],         // 21
    &[21, 23],        // 22
    &[22, 24, 26],    // 23 (branch point → 26)
    &[23, 25],        // 24
    &[24],            // 25 Undying (terminal)
    // Survival branch
    &[23, 27],        // 26
    &[26, 28, 30],    // 27
    &[27, 29],        // 28
    &[28],            // 29 Iron Will (terminal)
    &[27],            // 30 extra small (terminal off 27)
    // Fortune main path
    &[0, 32],         // 31
    &[31, 33],        // 32
    &[32, 34, 36],    // 33 (branch point → 36)
    &[33, 35],        // 34
    &[34],            // 35 Midas Touch (terminal)
    // Fortune branch
    &[33, 37],        // 36
    &[36, 38, 40],    // 37
    &[37, 39],        // 38
    &[38],            // 39 Radical Magnet (terminal)
    &[37],            // 40 extra small (terminal off 37)
];

pub static SKILL_TREE: SkillTree = SkillTree {
    nodes: &NODES,
    edges: &EDGES,
};

// ---------------------------------------------------------------------------
// Runtime state
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SkillTreeState {
    pub xp: u32,
    pub level: u32,
    pub skill_points: u32,
    /// `allocated[i]` is `true` when node `i` has been taken.
    pub allocated: Vec<bool>,
    /// Whether the Undying auto-revive has already fired this run.
    pub undying_used: bool,
}

impl SkillTreeState {
    /// Create a fresh state with only the start node (index 0) allocated.
    pub fn new() -> Self {
        let mut allocated = vec![false; SKILL_TREE.nodes.len()];
        allocated[0] = true;
        Self {
            xp: 0,
            level: 0,
            skill_points: 0,
            allocated,
            undying_used: false,
        }
    }

    // -- XP / leveling -----------------------------------------------------

    /// Cumulative XP required to reach level `n` (triangular: n*(n+1)/2 * 100).
    /// Level 1 → 100, level 2 → 300, level 3 → 600, …
    fn cumulative_xp_for_level(n: u32) -> u32 {
        n * (n + 1) / 2 * 100
    }

    /// XP still needed to reach the *next* level.
    pub fn xp_for_next_level(&self) -> u32 {
        let required = Self::cumulative_xp_for_level(self.level + 1);
        required.saturating_sub(self.xp)
    }

    /// Grant `amount` XP and automatically level up as many times as possible.
    pub fn gain_xp(&mut self, amount: u32) {
        self.xp += amount;
        loop {
            let next = Self::cumulative_xp_for_level(self.level + 1);
            if self.xp >= next {
                self.level += 1;
                self.skill_points += 1;
            } else {
                break;
            }
        }
    }

    // -- Allocation --------------------------------------------------------

    /// A node can be allocated when:
    /// 1. It exists and is not already allocated.
    /// 2. The player has at least one skill point.
    /// 3. At least one neighbor of the node is already allocated.
    pub fn can_allocate(&self, node_idx: usize) -> bool {
        let tree = &SKILL_TREE;
        if node_idx >= tree.nodes.len() {
            return false;
        }
        if self.allocated[node_idx] {
            return false;
        }
        if self.skill_points == 0 {
            return false;
        }
        tree.edges[node_idx]
            .iter()
            .any(|&neighbor| self.allocated[neighbor])
    }

    /// Spend a skill point and allocate `node_idx`. Returns `true` on success.
    pub fn allocate(&mut self, node_idx: usize) -> bool {
        if !self.can_allocate(node_idx) {
            return false;
        }
        self.skill_points -= 1;
        self.allocated[node_idx] = true;
        true
    }

    // -- Notable helpers ---------------------------------------------------

    /// Returns `true` if any allocated node's effect satisfies `check`.
    pub fn has_notable(&self, check: fn(&SkillEffect) -> bool) -> bool {
        self.allocated
            .iter()
            .enumerate()
            .any(|(i, &alloc)| alloc && check(&SKILL_TREE.nodes[i].effect))
    }

    pub fn has_berserker(&self) -> bool {
        self.has_notable(|e| matches!(e, SkillEffect::Berserker))
    }
    pub fn has_executioner(&self) -> bool {
        self.has_notable(|e| matches!(e, SkillEffect::Executioner))
    }
    pub fn has_polyglot(&self) -> bool {
        self.has_notable(|e| matches!(e, SkillEffect::Polyglot))
    }
    pub fn has_linguists_fury(&self) -> bool {
        self.has_notable(|e| matches!(e, SkillEffect::LinguistsFury))
    }
    pub fn has_undying(&self) -> bool {
        self.has_notable(|e| matches!(e, SkillEffect::Undying))
    }
    pub fn has_iron_will(&self) -> bool {
        self.has_notable(|e| matches!(e, SkillEffect::IronWill))
    }
    pub fn has_midas_touch(&self) -> bool {
        self.has_notable(|e| matches!(e, SkillEffect::MidasTouch))
    }
    pub fn has_radical_magnet(&self) -> bool {
        self.has_notable(|e| matches!(e, SkillEffect::RadicalMagnet))
    }

    // -- Aggregate stat helpers --------------------------------------------

    fn sum_effect<F: Fn(&SkillEffect) -> i32>(&self, f: F) -> i32 {
        self.allocated
            .iter()
            .enumerate()
            .filter(|(_, &alloc)| alloc)
            .map(|(i, _)| f(&SKILL_TREE.nodes[i].effect))
            .sum()
    }

    pub fn total_bonus_damage(&self) -> i32 {
        let mut total = self.sum_effect(|e| match e {
            SkillEffect::BonusDamage(v) => *v,
            _ => 0,
        });
        if self.has_berserker() {
            total += 3;
        }
        total
    }

    pub fn total_bonus_armor(&self) -> i32 {
        let mut total = self.sum_effect(|e| match e {
            SkillEffect::BonusArmor(v) => *v,
            _ => 0,
        });
        if self.has_berserker() {
            total -= 1;
        }
        total
    }

    pub fn total_max_hp(&self) -> i32 {
        self.sum_effect(|e| match e {
            SkillEffect::MaxHp(v) => *v,
            _ => 0,
        })
    }

    pub fn total_crit_chance(&self) -> i32 {
        self.sum_effect(|e| match e {
            SkillEffect::CritChance(v) => *v,
            _ => 0,
        })
    }

    pub fn total_spell_power(&self) -> i32 {
        self.sum_effect(|e| match e {
            SkillEffect::SpellPower(v) => *v,
            _ => 0,
        })
    }

    pub fn total_max_focus(&self) -> i32 {
        self.sum_effect(|e| match e {
            SkillEffect::MaxFocus(v) => *v,
            _ => 0,
        })
    }

    pub fn total_focus_regen(&self) -> i32 {
        self.sum_effect(|e| match e {
            SkillEffect::FocusRegen(v) => *v,
            _ => 0,
        })
    }

    pub fn total_dodge_chance(&self) -> i32 {
        self.sum_effect(|e| match e {
            SkillEffect::DodgeChance(v) => *v,
            _ => 0,
        })
    }

    pub fn total_gold_find(&self) -> i32 {
        self.sum_effect(|e| match e {
            SkillEffect::GoldFind(v) => *v,
            _ => 0,
        })
    }

    pub fn total_radical_find(&self) -> i32 {
        self.sum_effect(|e| match e {
            SkillEffect::RadicalFind(v) => *v,
            _ => 0,
        })
    }

    pub fn total_item_rarity_bonus(&self) -> i32 {
        self.sum_effect(|e| match e {
            SkillEffect::ItemRarityBonus(v) => *v,
            _ => 0,
        })
    }

    // -- Run lifecycle -----------------------------------------------------

    /// Reset per-run flags (call at the start of a new dungeon run).
    pub fn reset_run(&mut self) {
        self.undying_used = false;
    }

    // -- Serialization (simple JSON for localStorage) ----------------------

    /// Serialize to a JSON string. Format:
    /// `{"xp":N,"level":N,"skill_points":N,"allocated":[0,3,5,...],"undying_used":false}`
    pub fn to_json(&self) -> String {
        let allocated_indices: Vec<usize> = self
            .allocated
            .iter()
            .enumerate()
            .filter(|(_, &a)| a)
            .map(|(i, _)| i)
            .collect();
        let indices_str: Vec<String> = allocated_indices.iter().map(|i| i.to_string()).collect();
        format!(
            "{{\"xp\":{},\"level\":{},\"skill_points\":{},\"allocated\":[{}],\"undying_used\":{}}}",
            self.xp,
            self.level,
            self.skill_points,
            indices_str.join(","),
            self.undying_used,
        )
    }

    /// Deserialize from a JSON string produced by `to_json`. Returns `None`
    /// on any parse failure.
    pub fn from_json(json: &str) -> Option<Self> {
        // Minimal hand-rolled parser — no serde dependency.
        let json = json.trim();
        if !json.starts_with('{') || !json.ends_with('}') {
            return None;
        }
        let inner = &json[1..json.len() - 1];

        fn extract_u32(s: &str, key: &str) -> Option<u32> {
            let needle = format!("\"{}\":", key);
            let start = s.find(&needle)? + needle.len();
            let rest = &s[start..];
            let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
            rest[..end].parse().ok()
        }

        fn extract_bool(s: &str, key: &str) -> Option<bool> {
            let needle = format!("\"{}\":", key);
            let start = s.find(&needle)? + needle.len();
            let rest = s[start..].trim_start();
            if rest.starts_with("true") {
                Some(true)
            } else if rest.starts_with("false") {
                Some(false)
            } else {
                None
            }
        }

        fn extract_usize_array(s: &str, key: &str) -> Option<Vec<usize>> {
            let needle = format!("\"{}\":[", key);
            let start = s.find(&needle)? + needle.len();
            let end = s[start..].find(']')? + start;
            let slice = &s[start..end];
            if slice.trim().is_empty() {
                return Some(vec![]);
            }
            slice
                .split(',')
                .map(|tok| tok.trim().parse::<usize>().ok())
                .collect()
        }

        let xp = extract_u32(inner, "xp")?;
        let level = extract_u32(inner, "level")?;
        let skill_points = extract_u32(inner, "skill_points")?;
        let undying_used = extract_bool(inner, "undying_used")?;
        let indices = extract_usize_array(inner, "allocated")?;

        let node_count = SKILL_TREE.nodes.len();
        let mut allocated = vec![false; node_count];
        for idx in indices {
            if idx >= node_count {
                return None;
            }
            allocated[idx] = true;
        }

        Some(Self {
            xp,
            level,
            skill_points,
            allocated,
            undying_used,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Tree structure sanity ---------------------------------------------

    #[test]
    fn tree_has_41_nodes() {
        assert_eq!(SKILL_TREE.nodes.len(), 41);
        assert_eq!(SKILL_TREE.edges.len(), 41);
    }

    #[test]
    fn start_node_is_at_origin() {
        let n = &SKILL_TREE.nodes[0];
        assert_eq!(n.cluster, Cluster::Start);
        assert_eq!(n.pos, (0, 0));
    }

    #[test]
    fn edges_are_symmetric() {
        for (i, neighbors) in SKILL_TREE.edges.iter().enumerate() {
            for &j in *neighbors {
                assert!(
                    SKILL_TREE.edges[j].contains(&i),
                    "Edge {i}→{j} exists but {j}→{i} does not"
                );
            }
        }
    }

    #[test]
    fn all_edge_indices_in_range() {
        let n = SKILL_TREE.nodes.len();
        for (i, neighbors) in SKILL_TREE.edges.iter().enumerate() {
            for &j in *neighbors {
                assert!(j < n, "Node {i} has out-of-range neighbor {j}");
            }
        }
    }

    #[test]
    fn notables_are_marked() {
        let notable_indices = [5, 9, 15, 19, 25, 29, 35, 39];
        for &i in &notable_indices {
            assert!(
                SKILL_TREE.nodes[i].is_notable,
                "Node {i} should be notable"
            );
        }
    }

    #[test]
    fn cluster_colors_and_names() {
        assert_eq!(Cluster::Start.color(), "#ffffff");
        assert_eq!(Cluster::Combat.color(), "#ff4444");
        assert_eq!(Cluster::Scholar.color(), "#4488ff");
        assert_eq!(Cluster::Survival.color(), "#44cc44");
        assert_eq!(Cluster::Fortune.color(), "#ffaa00");

        assert_eq!(Cluster::Start.name(), "Start");
        assert_eq!(Cluster::Combat.name(), "Combat");
        assert_eq!(Cluster::Scholar.name(), "Scholar");
        assert_eq!(Cluster::Survival.name(), "Survival");
        assert_eq!(Cluster::Fortune.name(), "Fortune");
    }

    // -- XP / leveling -----------------------------------------------------

    #[test]
    fn cumulative_xp_formula() {
        // Level 1: 1*2/2*100 = 100
        assert_eq!(SkillTreeState::cumulative_xp_for_level(1), 100);
        // Level 2: 2*3/2*100 = 300
        assert_eq!(SkillTreeState::cumulative_xp_for_level(2), 300);
        // Level 3: 3*4/2*100 = 600
        assert_eq!(SkillTreeState::cumulative_xp_for_level(3), 600);
        // Level 10: 10*11/2*100 = 5500
        assert_eq!(SkillTreeState::cumulative_xp_for_level(10), 5500);
    }

    #[test]
    fn gain_xp_single_level() {
        let mut s = SkillTreeState::new();
        assert_eq!(s.level, 0);
        assert_eq!(s.skill_points, 0);
        s.gain_xp(100);
        assert_eq!(s.level, 1);
        assert_eq!(s.skill_points, 1);
    }

    #[test]
    fn gain_xp_multiple_levels_at_once() {
        let mut s = SkillTreeState::new();
        s.gain_xp(600); // enough for level 3
        assert_eq!(s.level, 3);
        assert_eq!(s.skill_points, 3);
    }

    #[test]
    fn gain_xp_partial() {
        let mut s = SkillTreeState::new();
        s.gain_xp(50);
        assert_eq!(s.level, 0);
        assert_eq!(s.xp_for_next_level(), 50);
        s.gain_xp(50);
        assert_eq!(s.level, 1);
        // Next level at 300 cumulative, we have 100 → need 200
        assert_eq!(s.xp_for_next_level(), 200);
    }

    #[test]
    fn xp_for_next_level_at_zero() {
        let s = SkillTreeState::new();
        assert_eq!(s.xp_for_next_level(), 100);
    }

    // -- Allocation --------------------------------------------------------

    #[test]
    fn start_node_always_allocated() {
        let s = SkillTreeState::new();
        assert!(s.allocated[0]);
    }

    #[test]
    fn can_allocate_adjacent() {
        let mut s = SkillTreeState::new();
        s.skill_points = 1;
        // Node 1 is adjacent to start
        assert!(s.can_allocate(1));
        // Node 2 is NOT adjacent to any allocated node
        assert!(!s.can_allocate(2));
    }

    #[test]
    fn cannot_allocate_without_points() {
        let s = SkillTreeState::new();
        assert_eq!(s.skill_points, 0);
        assert!(!s.can_allocate(1));
    }

    #[test]
    fn cannot_allocate_already_allocated() {
        let mut s = SkillTreeState::new();
        s.skill_points = 1;
        assert!(!s.can_allocate(0)); // start is already allocated
    }

    #[test]
    fn allocate_spends_point() {
        let mut s = SkillTreeState::new();
        s.skill_points = 2;
        assert!(s.allocate(1));
        assert_eq!(s.skill_points, 1);
        assert!(s.allocated[1]);
        // Now node 2 is adjacent
        assert!(s.allocate(2));
        assert_eq!(s.skill_points, 0);
    }

    #[test]
    fn allocate_returns_false_on_failure() {
        let mut s = SkillTreeState::new();
        assert!(!s.allocate(1)); // no points
        s.skill_points = 1;
        assert!(!s.allocate(5)); // not adjacent
    }

    #[test]
    fn cannot_allocate_out_of_range() {
        let mut s = SkillTreeState::new();
        s.skill_points = 10;
        assert!(!s.can_allocate(999));
    }

    // -- Stat aggregation --------------------------------------------------

    #[test]
    fn total_damage_from_combat_path() {
        let mut s = SkillTreeState::new();
        s.skill_points = 10;
        // Allocate combat main path: 1(+1d), 2(+1d), 3(+5%c), 4(+1d)
        s.allocate(1);
        s.allocate(2);
        s.allocate(3);
        s.allocate(4);
        assert_eq!(s.total_bonus_damage(), 3); // 1+1+1
        assert_eq!(s.total_crit_chance(), 5);
    }

    #[test]
    fn berserker_modifies_damage_and_armor() {
        let mut s = SkillTreeState::new();
        s.skill_points = 20;
        for i in 1..=5 {
            s.allocate(i);
        }
        assert!(s.has_berserker());
        // Damage: nodes 1,2,4 = 3 flat + 3 berserker = 6
        assert_eq!(s.total_bonus_damage(), 6);
        // Armor: 0 flat − 1 berserker = −1
        assert_eq!(s.total_bonus_armor(), -1);
    }

    #[test]
    fn survival_path_hp_and_armor() {
        let mut s = SkillTreeState::new();
        s.skill_points = 10;
        // Main: 21(+3hp), 22(+1arm), 23(+3hp), 24(+5%dodge)
        s.allocate(21);
        s.allocate(22);
        s.allocate(23);
        s.allocate(24);
        assert_eq!(s.total_max_hp(), 6);
        assert_eq!(s.total_bonus_armor(), 1);
        assert_eq!(s.total_dodge_chance(), 5);
    }

    #[test]
    fn fortune_gold_and_radical() {
        let mut s = SkillTreeState::new();
        s.skill_points = 10;
        s.allocate(31);
        s.allocate(32);
        s.allocate(33);
        assert_eq!(s.total_gold_find(), 20);     // 10 + 10
        assert_eq!(s.total_radical_find(), 10);   // 10
    }

    #[test]
    fn scholar_spell_power_and_focus() {
        let mut s = SkillTreeState::new();
        s.skill_points = 10;
        s.allocate(11);
        s.allocate(12);
        s.allocate(13);
        s.allocate(14);
        assert_eq!(s.total_spell_power(), 2); // 1 + 1
        assert_eq!(s.total_max_focus(), 5);
        assert_eq!(s.total_focus_regen(), 1);
    }

    // -- Notable checks ----------------------------------------------------

    #[test]
    fn notable_checks_default_false() {
        let s = SkillTreeState::new();
        assert!(!s.has_berserker());
        assert!(!s.has_executioner());
        assert!(!s.has_polyglot());
        assert!(!s.has_linguists_fury());
        assert!(!s.has_undying());
        assert!(!s.has_iron_will());
        assert!(!s.has_midas_touch());
        assert!(!s.has_radical_magnet());
    }

    #[test]
    fn notable_polyglot_after_allocation() {
        let mut s = SkillTreeState::new();
        s.skill_points = 20;
        // Path: 0 → 11 → 12 → 13 → 14 → 15(Polyglot)
        for i in 11..=15 {
            assert!(s.allocate(i));
        }
        assert!(s.has_polyglot());
    }

    #[test]
    fn notable_radical_magnet() {
        let mut s = SkillTreeState::new();
        s.skill_points = 20;
        // Path: 0 → 31 → 32 → 33 → 36 → 37 → 38 → 39
        for i in [31, 32, 33, 36, 37, 38, 39] {
            assert!(s.allocate(i), "Failed to allocate node {i}");
        }
        assert!(s.has_radical_magnet());
    }

    // -- Reset run ---------------------------------------------------------

    #[test]
    fn reset_run_clears_undying() {
        let mut s = SkillTreeState::new();
        s.undying_used = true;
        s.reset_run();
        assert!(!s.undying_used);
    }

    // -- JSON round-trip ---------------------------------------------------

    #[test]
    fn json_round_trip_empty() {
        let s = SkillTreeState::new();
        let json = s.to_json();
        let s2 = SkillTreeState::from_json(&json).expect("parse failed");
        assert_eq!(s2.xp, 0);
        assert_eq!(s2.level, 0);
        assert_eq!(s2.skill_points, 0);
        assert!(s2.allocated[0]);
        assert!(!s2.undying_used);
    }

    #[test]
    fn json_round_trip_with_progress() {
        let mut s = SkillTreeState::new();
        s.gain_xp(350);
        s.allocate(1);
        s.allocate(11);
        s.undying_used = true;

        let json = s.to_json();
        let s2 = SkillTreeState::from_json(&json).expect("parse failed");
        assert_eq!(s2.xp, s.xp);
        assert_eq!(s2.level, s.level);
        assert_eq!(s2.skill_points, s.skill_points);
        assert_eq!(s2.allocated, s.allocated);
        assert_eq!(s2.undying_used, true);
    }

    #[test]
    fn json_invalid_returns_none() {
        assert!(SkillTreeState::from_json("").is_none());
        assert!(SkillTreeState::from_json("not json").is_none());
        assert!(SkillTreeState::from_json("{}").is_none());
    }

    #[test]
    fn json_out_of_range_index_returns_none() {
        let bad = "{\"xp\":0,\"level\":0,\"skill_points\":0,\"allocated\":[9999],\"undying_used\":false}";
        assert!(SkillTreeState::from_json(bad).is_none());
    }

    // -- Integration: full play-through ------------------------------------

    #[test]
    fn integration_play_through() {
        let mut s = SkillTreeState::new();

        // Earn XP from a few encounters
        s.gain_xp(XP_BOSS_KILL);                       // 100 → level 1
        s.gain_xp(XP_ELITE_KILL);                      // 125
        s.gain_xp(XP_CORRECT_ANSWER + XP_HARD_BONUS);  // 135
        s.gain_xp(XP_NORMAL_KILL * 10);                 // 235
        s.gain_xp(XP_BOSS_KILL);                        // 335 → level 2
        assert_eq!(s.level, 2);
        assert_eq!(s.skill_points, 2);

        // Allocate two combat nodes
        assert!(s.allocate(1));
        assert!(s.allocate(2));
        assert_eq!(s.total_bonus_damage(), 2);
        assert_eq!(s.skill_points, 0);

        // Earn more XP, reach level 3 (need 600 cumulative, have 335)
        s.gain_xp(265); // 600
        assert_eq!(s.level, 3);
        assert!(s.allocate(3));

        // Serialize, deserialize, verify
        let json = s.to_json();
        let s2 = SkillTreeState::from_json(&json).unwrap();
        assert_eq!(s2.level, 3);
        assert_eq!(s2.total_bonus_damage(), 2);
        assert_eq!(s2.total_crit_chance(), 5);
    }

    // -- Item rarity bonus -------------------------------------------------

    #[test]
    fn item_rarity_from_fortune_branch() {
        let mut s = SkillTreeState::new();
        s.skill_points = 20;
        // 31 → 32 → 33 → 34(+5% rarity)
        s.allocate(31);
        s.allocate(32);
        s.allocate(33);
        s.allocate(34);
        assert_eq!(s.total_item_rarity_bonus(), 5);
        // Branch: 33 → 36 → 37 → 38(+5% rarity)
        s.allocate(36);
        s.allocate(37);
        s.allocate(38);
        assert_eq!(s.total_item_rarity_bonus(), 10);
    }
}
