//! Type definitions for the game module.

use crate::combat;
use crate::player::ItemKind;
use crate::radical;
use crate::world::{AltarKind, SealKind};

use super::ShopItem;


/// Combat phase when the player is adjacent to / engages an enemy.
/// Companion NPC that follows the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Companion {
    ScienceOfficer,
    Medic,
    Quartermaster,
    SecurityChief,
}

/// Number of companions (used for fixed-size bond arrays).
pub const COMPANION_COUNT: usize = 4;

impl Companion {
    /// Stable index for array-based bond tracking.
    pub fn index(&self) -> usize {
        match self {
            Companion::ScienceOfficer => 0,
            Companion::Medic => 1,
            Companion::Quartermaster => 2,
            Companion::SecurityChief => 3,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Companion::ScienceOfficer => "Science Officer 研",
            Companion::Medic => "Medic 医",
            Companion::Quartermaster => "Quartermaster 商",
            Companion::SecurityChief => "Security Chief 卫",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Companion::ScienceOfficer => "🔬",
            Companion::Medic => "💊",
            Companion::Quartermaster => "📦",
            Companion::SecurityChief => "🛡",
        }
    }

    #[allow(dead_code)]
    pub fn xp_for_level(level: u8) -> u32 {
        match level {
            0 | 1 => 0,
            2 => 30,
            _ => 80,
        }
    }

    pub fn level_from_xp(xp: u32) -> u8 {
        if xp >= 80 {
            3
        } else if xp >= 30 {
            2
        } else {
            1
        }
    }

    #[allow(dead_code)]
    pub fn max_level() -> u8 {
        3
    }

    pub fn shop_discount_pct(&self, level: u8) -> i32 {
        match self {
            Companion::Quartermaster => {
                if level >= 2 {
                    25
                } else {
                    20
                }
            }
            _ => 0,
        }
    }

    pub fn heal_per_floor(&self, level: u8) -> i32 {
        match self {
            Companion::Medic => {
                if level >= 2 {
                    2
                } else {
                    1
                }
            }
            _ => 0,
        }
    }

    pub fn guard_max_blocks(&self, level: u8) -> u8 {
        match self {
            Companion::SecurityChief => {
                if level >= 3 {
                    2
                } else {
                    1
                }
            }
            _ => 0,
        }
    }

    pub fn guard_second_block_chance(&self, level: u8) -> u64 {
        match self {
            Companion::SecurityChief => {
                if level >= 3 {
                    100
                } else if level >= 2 {
                    50
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    pub fn contextual_hint(
        &self,
        enemy: &crate::enemy::Enemy,
        player_hp: i32,
        player_max_hp: i32,
        guard_used: bool,
        level: u8,
    ) -> Option<String> {
        match self {
            Companion::ScienceOfficer => {
                let first_char = enemy
                    .meaning
                    .split_whitespace()
                    .next()
                    .unwrap_or(enemy.meaning);
                let mut hint = format!("🔬 Hint: {} means \"{}\"", enemy.hanzi, first_char);
                if level >= 2 {
                    hint.push_str(&format!(" ({})", enemy.pinyin));
                }
                if level >= 3 {
                    let radicals: Vec<String> =
                        enemy.hanzi.chars().map(|c| c.to_string()).collect();
                    if radicals.len() > 1 {
                        hint.push_str(&format!(" [{}]", radicals.join("+")));
                    }
                }
                Some(hint)
            }
            Companion::Medic => {
                if player_hp <= player_max_hp / 3 {
                    let heal = self.heal_per_floor(level);
                    Some(format!(
                        "💊 Stay focused. I'll mend {} HP next deck.",
                        heal
                    ))
                } else {
                    None
                }
            }
            Companion::Quartermaster => {
                if enemy.gold_value >= 20 {
                    Some(format!("📦 That one's worth {} credits!", enemy.gold_value))
                } else {
                    None
                }
            }
            Companion::SecurityChief => {
                if !guard_used {
                    let blocks = self.guard_max_blocks(level);
                    if blocks > 1 {
                        Some(format!("🛡 I'll block up to {} hits for you.", blocks))
                    } else {
                        Some("🛡 I'll block the first hit for you.".to_string())
                    }
                } else {
                    None
                }
            }
        }
    }
}

// ── Companion Bond / Synergy ──────────────────────────────────────

/// Tracks the relationship between the player and a companion over time.
/// As `floors_together` increases, `synergy_level` unlocks new effects.
#[derive(Clone, Copy, Debug, Default)]
pub struct CompanionBond {
    pub floors_together: u32,
    /// 0 = no synergy, 1 = callouts, 2 = passive bonus, 3 = combo ability
    pub synergy_level: u8,
}

impl CompanionBond {
    /// Advance bond by one floor and recalculate synergy level.
    pub fn advance_floor(&mut self) {
        self.floors_together += 1;
        self.synergy_level = Self::level_for_floors(self.floors_together);
    }

    pub fn level_for_floors(floors: u32) -> u8 {
        match floors {
            0..=4 => 0,
            5..=9 => 1,
            10..=14 => 2,
            _ => 3,
        }
    }
}

impl Companion {
    /// Random combat callout (synergy level 1+). Returns a flavour message.
    pub fn synergy_callout(&self, rng_val: u64) -> &'static str {
        let pool: &[&str] = match self {
            Companion::ScienceOfficer => &[
                "🔬 Officer: That enemy is weak to fire!",
                "🔬 Officer: I'm detecting a structural weakness!",
                "🔬 Officer: Scans show reduced defenses!",
            ],
            Companion::Medic => &[
                "💊 Medic: Stay steady, I've got your back!",
                "💊 Medic: That one's venom glands are exposed!",
                "💊 Medic: Watch the counterattack pattern!",
            ],
            Companion::Quartermaster => &[
                "📦 QM: That one's carrying valuable loot!",
                "📦 QM: I've seen this type — hit it hard!",
                "📦 QM: Careful, these drop rare components!",
            ],
            Companion::SecurityChief => &[
                "🛡 Chief: Flanking position — strike now!",
                "🛡 Chief: I see an opening in its guard!",
                "🛡 Chief: Cover me, setting up a crossfire!",
            ],
        };
        pool[(rng_val as usize) % pool.len()]
    }

    /// Synergy level 2 passive bonus: extra damage on correct answers.
    pub fn synergy_damage_bonus(&self) -> i32 {
        match self {
            Companion::ScienceOfficer => 1,
            Companion::SecurityChief => 1,
            _ => 0,
        }
    }

    /// Synergy level 2 passive bonus: extra gold percentage from kills.
    pub fn synergy_gold_pct(&self) -> i32 {
        match self {
            Companion::Quartermaster => 15,
            _ => 0,
        }
    }

    /// Synergy level 2 passive bonus: extra heal per floor.
    pub fn synergy_heal_bonus(&self) -> i32 {
        match self {
            Companion::Medic => 1,
            _ => 0,
        }
    }

    /// Name of the level-3 combo ability.
    pub fn combo_ability_name(&self) -> &'static str {
        match self {
            Companion::ScienceOfficer => "Nanite Surge",
            Companion::Medic => "Vital Strike",
            Companion::Quartermaster => "Supply Drop",
            Companion::SecurityChief => "Fortified Stance",
        }
    }

    /// Trigger message for the level-3 combo ability.
    pub fn combo_ability_message(&self) -> &'static str {
        match self {
            Companion::ScienceOfficer => "🔬 Nanite Surge! Healing nanites + weakness scan!",
            Companion::Medic => "💊 Vital Strike! Heal on correct answer!",
            Companion::Quartermaster => "📦 Supply Drop! Double gold from this kill!",
            Companion::SecurityChief => "🛡 Fortified Stance! Damage negated!",
        }
    }
}

// ── Run Journal ────────────────────────────────────────────────────
#[derive(Clone, Debug)]
pub enum RunEvent {
    EnteredFloor(i32),
    EnemyKilled(#[allow(dead_code)] String, i32),
    BossKilled(String, i32),
    SpellForged(String, i32),
    RadicalCollected(String, i32),
    ComboAchieved(u32, i32),
    DiedTo(String, i32),
}

#[derive(Clone, Debug, Default)]
pub struct RunJournal {
    pub events: Vec<RunEvent>,
    pub max_combo: u32,
}

impl RunJournal {
    pub fn log(&mut self, event: RunEvent) {
        self.events.push(event);
    }

    pub fn floor_summary(&self, floor: i32) -> Vec<&RunEvent> {
        self.events
            .iter()
            .filter(|e| match e {
                RunEvent::EnteredFloor(f)
                | RunEvent::EnemyKilled(_, f)
                | RunEvent::BossKilled(_, f)
                | RunEvent::SpellForged(_, f)
                | RunEvent::RadicalCollected(_, f)
                | RunEvent::ComboAchieved(_, f)
                | RunEvent::DiedTo(_, f) => *f == floor,
            })
            .collect()
    }

    pub fn death_cause(&self) -> &str {
        for e in self.events.iter().rev() {
            if let RunEvent::DiedTo(cause, _) = e {
                return cause;
            }
        }
        "Unknown"
    }

    pub fn enemies_killed_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e, RunEvent::EnemyKilled(_, _) | RunEvent::BossKilled(_, _)))
            .count()
    }

    #[allow(dead_code)]
    pub fn spells_forged_list(&self) -> Vec<&str> {
        self.events
            .iter()
            .filter_map(|e| {
                if let RunEvent::SpellForged(name, _) = e {
                    Some(name.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn max_floor(&self) -> i32 {
        self.events
            .iter()
            .filter_map(|e| {
                if let RunEvent::EnteredFloor(f) = e {
                    Some(*f)
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(1)
    }

    /// One-line summary for a given floor
    pub fn floor_line(&self, floor: i32) -> String {
        let evts = self.floor_summary(floor);
        let mut parts: Vec<String> = Vec::new();
        let mut kills = 0u32;
        let mut boss: Option<&str> = None;
        for e in &evts {
            match e {
                RunEvent::EnemyKilled(_, _) => kills += 1,
                RunEvent::BossKilled(h, _) => {
                    boss = Some(h);
                    kills += 1;
                }
                RunEvent::SpellForged(name, _) => parts.push(format!("Forged {}", name)),
                RunEvent::RadicalCollected(ch, _) => parts.push(format!("+[{}]", ch)),

                RunEvent::ComboAchieved(n, _) => parts.push(format!("{}× combo", n)),
                RunEvent::DiedTo(cause, _) => parts.push(format!("☠ {}", cause)),
                RunEvent::EnteredFloor(_) => {}
            }
        }
        if let Some(b) = boss {
            parts.insert(0, format!("Boss {} slain", b));
        } else if kills > 0 {
            parts.insert(0, format!("{} kills", kills));
        }
        if parts.is_empty() {
            "Explored".to_string()
        } else {
            parts.join(", ")
        }
    }
}

// ── Combo tiers ────────────────────────────────────────────────────
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ComboTier {
    None,
    Good,
    Great,
    Excellent,
    Perfect,
    Radical,
}

impl ComboTier {
    pub fn name(&self) -> &'static str {
        match self {
            ComboTier::None => "",
            ComboTier::Good => "GOOD",
            ComboTier::Great => "GREAT",
            ComboTier::Excellent => "EXCELLENT",
            ComboTier::Perfect => "PERFECT",
            ComboTier::Radical => "RADICAL",
        }
    }

    pub fn multiplier(&self) -> f64 {
        match self {
            ComboTier::None => 1.0,
            ComboTier::Good => 1.15,
            ComboTier::Great => 1.3,
            ComboTier::Excellent => 1.5,
            ComboTier::Perfect => 1.75,
            ComboTier::Radical => 2.0,
        }
    }
}

pub fn combo_tier(streak: u32) -> ComboTier {
    match streak {
        0..=1 => ComboTier::None,
        2..=3 => ComboTier::Good,
        4..=5 => ComboTier::Great,
        6..=8 => ComboTier::Excellent,
        9..=11 => ComboTier::Perfect,
        _ => ComboTier::Radical,
    }
}

// ── Event Memory ───────────────────────────────────────────────────
/// Tracks persistent consequences of event choices across a run.
#[derive(Clone, Debug, Default)]
pub struct EventMemory {
    /// Accumulated crew morale modifier from social choices
    pub crew_morale: i32,
    /// Standing with alien/pirate factions (negative = hostile)
    pub faction_standing: i32,
    /// Keys recording past choices (e.g. "helped_stowaway", "raided_pirates")
    pub past_choices: Vec<String>,
}

impl EventMemory {
    pub fn record_choice(&mut self, key: &str) {
        if !self.past_choices.iter().any(|c| c == key) {
            self.past_choices.push(key.to_string());
        }
    }

    pub fn has_choice(&self, key: &str) -> bool {
        self.past_choices.iter().any(|c| c == key)
    }
}

/// Listening mode variants for audio-based combat challenges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ListenMode {
    /// Normal mode: show hanzi + meaning
    Off,
    /// Tone-only mode: play tone contour, show meaning, hide pinyin — identify tone number
    ToneOnly,
    /// Full audio mode: play tone, hide hanzi — type full pinyin by ear
    FullAudio,
}

impl ListenMode {
    pub fn is_active(self) -> bool {
        self != ListenMode::Off
    }

    pub fn cycle(self) -> Self {
        match self {
            ListenMode::Off => ListenMode::ToneOnly,
            ListenMode::ToneOnly => ListenMode::FullAudio,
            ListenMode::FullAudio => ListenMode::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ListenMode::Off => "OFF",
            ListenMode::ToneOnly => "🎵 Tone-Only",
            ListenMode::FullAudio => "🎧 Full Audio",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FloorProfile {
    Normal,
    Famine,
    RadicalRich,
    Siege,
    Drought,
}

impl FloorProfile {
    pub(super) fn roll(floor: i32, rng_val: u64) -> Self {
        if floor <= 2 {
            return FloorProfile::Normal;
        }
        match rng_val % 100 {
            0..=19 => FloorProfile::Famine,
            20..=34 => FloorProfile::RadicalRich,
            35..=44 => FloorProfile::Siege,
            45..=54 => FloorProfile::Drought,
            _ => FloorProfile::Normal,
        }
    }

    pub fn gold_multiplier(self) -> f64 {
        match self {
            FloorProfile::Normal => 1.0,
            FloorProfile::Famine => 0.5,
            FloorProfile::RadicalRich => 0.8,
            FloorProfile::Siege => 1.5,
            FloorProfile::Drought => 0.3,
        }
    }

    pub fn radical_drop_bonus(self) -> bool {
        matches!(self, FloorProfile::RadicalRich)
    }

    /// Chance (0-100) that killing an enemy drops a radical.
    pub fn radical_drop_chance(self) -> u64 {
        match self {
            FloorProfile::Normal => 80,
            FloorProfile::Famine => 50,
            FloorProfile::RadicalRich => 100,
            FloorProfile::Siege => 80,
            FloorProfile::Drought => 0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FloorProfile::Normal => "",
            FloorProfile::Famine => "⚠ Famine Floor",
            FloorProfile::RadicalRich => "📜 Radical-Rich Floor",
            FloorProfile::Siege => "⚔ Siege Floor",
            FloorProfile::Drought => "🏜 Drought Floor",
        }
    }
}

/// Quest condition for procedural quests.
#[derive(Clone, Debug)]
pub enum QuestGoal {
    /// Kill N enemies on this floor
    KillEnemies(i32, i32),
    /// Forge a specific character
    ForgeCharacter(&'static str),
    /// Reach floor N
    ReachFloor(i32),
    /// Collect N radicals
    CollectRadicals(i32, i32),
}

/// Active quest given by an NPC.
#[derive(Clone, Debug)]
pub struct Quest {
    pub description: String,
    pub goal: QuestGoal,
    pub gold_reward: i32,
    pub completed: bool,
    /// Chain step: 0 = standalone, 1+ = chain step number
    pub chain_step: u8,
    /// Chain ID to group related quests (0 = not chained)
    pub chain_id: u32,
    /// NPC questgiver name (empty for procedural quests)
    pub giver_name: &'static str,
    /// NPC questgiver title/division (empty for procedural quests)
    pub giver_title: &'static str,
    /// Flavor text when quest is given (empty for procedural quests)
    #[allow(dead_code)]
    pub intro_text: &'static str,
    /// Flavor text on completion (empty for procedural quests)
    pub completion_text: &'static str,
}

impl Quest {
    /// Create a procedural (non-narrative) quest with empty flavor fields.
    pub(super) fn procedural(
        description: String,
        goal: QuestGoal,
        gold_reward: i32,
        chain_step: u8,
        chain_id: u32,
    ) -> Self {
        Self {
            description,
            goal,
            gold_reward,
            completed: false,
            chain_step,
            chain_id,
            giver_name: "",
            giver_title: "",
            intro_text: "",
            completion_text: "",
        }
    }

    #[allow(dead_code)]
    pub(super) fn check_complete(&mut self) -> bool {
        if self.completed {
            return false;
        }
        let done = match &self.goal {
            QuestGoal::KillEnemies(current, target) => current >= target,
            QuestGoal::ForgeCharacter(_) => false,
            QuestGoal::ReachFloor(_) => false,
            QuestGoal::CollectRadicals(current, target) => current >= target,
        };
        if done {
            self.completed = true;
        }
        done
    }

    pub(super) fn is_chain(&self) -> bool {
        self.chain_id > 0
    }

    /// Whether this quest has narrative flavor text.
    pub(super) fn is_narrative(&self) -> bool {
        !self.giver_name.is_empty()
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct TutorialState {
    pub(super) combat_done: bool,
    pub(super) forge_done: bool,
}

impl TutorialState {
    pub(super) fn is_complete(&self) -> bool {
        self.combat_done && self.forge_done
    }

    pub(super) fn objective_text(&self) -> &'static str {
        if !self.combat_done {
            "Tutorial: defeat 大 by typing da4."
        } else if !self.forge_done {
            "Tutorial: forge 好 from 女 + 子 at the anvil."
        } else {
            "Tutorial complete: take the stairs to Floor 1."
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextSpeed {
    Slow,
    Normal,
    Fast,
}

impl TextSpeed {
    pub fn label(self) -> &'static str {
        match self {
            Self::Slow => "Slow",
            Self::Normal => "Normal",
            Self::Fast => "Fast",
        }
    }

    pub(super) fn next(self) -> Self {
        match self {
            Self::Slow => Self::Normal,
            Self::Normal => Self::Fast,
            Self::Fast => Self::Fast,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::Slow => Self::Slow,
            Self::Normal => Self::Slow,
            Self::Fast => Self::Normal,
        }
    }

    pub(super) fn timer_step(self) -> u8 {
        match self {
            Self::Slow => 1,
            Self::Normal => 1,
            Self::Fast => 1,
        }
    }

    pub(super) fn timer_delay(self) -> u8 {
        match self {
            Self::Slow => 3,
            Self::Normal => 2,
            Self::Fast => 1,
        }
    }

    pub(super) fn from_storage(value: &str) -> Self {
        match value {
            "slow" => Self::Slow,
            "fast" => Self::Fast,
            _ => Self::Normal,
        }
    }

    pub(super) fn storage_key(self) -> &'static str {
        match self {
            Self::Slow => "slow",
            Self::Normal => "normal",
            Self::Fast => "fast",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameSettings {
    pub music_volume: u8,
    pub sfx_volume: u8,
    pub screen_shake: bool,
    pub text_speed: TextSpeed,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            music_volume: 100,
            sfx_volume: 100,
            screen_shake: true,
            text_speed: TextSpeed::Normal,
        }
    }
}

#[derive(Clone, Debug)]
pub enum SentenceChallengeMode {
    BonusGold {
        reward: i32,
    },
    ScholarTrial {
        boss_idx: usize,
        success_damage: i32,
        failure_heal: i32,
    },
    GatekeeperSeal {
        boss_idx: usize,
        success_damage: i32,
        failure_damage_to_player: i32,
    },
}

#[derive(Clone, Debug)]
pub enum CombatState {
    /// Normal exploration — no active fight
    Explore,
    /// Cursor-based inspection mode
    Looking { x: i32, y: i32 },
    /// Fighting an enemy: index into `enemies` vec
    Fighting {
        enemy_idx: usize,
        #[allow(dead_code)]
        timer_ms: f64,
    },
    /// Player is at a forge workbench, browsing craftable recipes
    Forging { recipes: Vec<usize>, cursor: usize },
    /// Player is at a shop, browsing items
    Shopping {
        /// Items for sale: (description, cost, action)
        items: Vec<ShopItem>,
        cursor: usize,
    },
    /// Player is enchanting equipment at a forge
    Enchanting {
        /// 0 = selecting slot, 1 = selecting radical
        step: u8,
        /// 0=weapon, 1=armor, 2=charm
        slot: usize,
        /// Which page of radicals to show
        page: usize,
    },
    /// Player is dead
    GameOver,
    /// Class selection screen at game start
    ClassSelect,
    /// Tone battle mini-game at a shrine
    ToneBattle {
        /// Current round (0-4, 5 rounds per shrine)
        round: usize,
        /// The character shown
        hanzi: &'static str,
        /// Correct tone (1-4)
        correct_tone: u8,
        /// Score so far (correct answers)
        score: usize,
        /// Result of last answer (None=waiting, Some(true)=correct, Some(false)=wrong)
        last_result: Option<bool>,
    },
    /// Sentence construction challenge (boss phase 2)
    SentenceChallenge {
        /// Scrambled word tiles (indices into correct order)
        tiles: Vec<usize>,
        /// The words in correct order
        words: Vec<&'static str>,
        /// Current cursor position
        cursor: usize,
        /// Player's arranged order so far
        arranged: Vec<usize>,
        /// Translation/meaning
        meaning: &'static str,
        /// What this challenge resolves into
        mode: SentenceChallengeMode,
    },
    /// Player is offering an item at an altar
    Offering {
        altar_kind: AltarKind,
        cursor: usize,
    },
    /// Player is selecting a potion to dip
    DippingSource { cursor: usize },
    /// Player is selecting a target for the dipped potion
    DippingTarget {
        source_idx: usize,
        cursor: usize, // 0..items.len() + 3 (equip slots)
    },
    /// Player is aiming a spell during exploration
    Aiming { spell_idx: usize, dx: i32, dy: i32 },
    /// Stroke order challenge at a shrine
    StrokeOrder {
        hanzi: &'static str,
        components: Vec<&'static str>,
        correct_order: Vec<&'static str>,
        cursor: usize,
        arranged: Vec<&'static str>,
        pinyin: &'static str,
        meaning: &'static str,
    },
    /// Tone defense wall challenge
    ToneDefense {
        round: usize,
        hanzi: &'static str,
        pinyin: &'static str,
        meaning: &'static str,
        correct_tone: u8,
        score: usize,
        last_result: Option<bool>,
    },
    /// Compound word builder challenge
    CompoundBuilder {
        parts: Vec<&'static str>,
        correct_compound: &'static str,
        pinyin: &'static str,
        meaning: &'static str,
        cursor: usize,
        arranged: Vec<&'static str>,
    },
    /// Classifier matching challenge
    ClassifierMatch {
        round: usize,
        noun: &'static str,
        noun_pinyin: &'static str,
        noun_meaning: &'static str,
        correct_classifier: &'static str,
        options: [&'static str; 4],
        correct_idx: usize,
        score: usize,
        last_result: Option<bool>,
    },
    /// InkWell: guess component count of a hanzi
    InkWellChallenge {
        hanzi: &'static str,
        correct_count: u8,
        pinyin: &'static str,
        meaning: &'static str,
    },
    /// AncestorShrine: complete a chengyu (4-char idiom)
    AncestorChallenge {
        first_half: &'static str,
        correct_second: &'static str,
        full: &'static str,
        pinyin: &'static str,
        meaning: &'static str,
        options: [&'static str; 4],
        correct_idx: usize,
    },
    /// TranslationAltar: pick correct Chinese for English meaning (3 rounds)
    TranslationChallenge {
        round: usize,
        meaning: &'static str,
        correct_hanzi: &'static str,
        correct_pinyin: &'static str,
        options: [&'static str; 4],
        correct_idx: usize,
        score: usize,
    },
    /// RadicalGarden: identify the radical of a hanzi
    RadicalGardenChallenge {
        hanzi: &'static str,
        pinyin: &'static str,
        meaning: &'static str,
        correct_radical: &'static str,
        options: [&'static str; 4],
        correct_idx: usize,
    },
    /// MirrorPool: type pinyin for a hanzi (text input)
    MirrorPoolChallenge {
        hanzi: &'static str,
        correct_pinyin: &'static str,
        meaning: &'static str,
        input: String,
    },
    /// StoneTutor: teaching phase then tone quiz (3 rounds)
    StoneTutorChallenge {
        round: usize,
        hanzi: &'static str,
        pinyin: &'static str,
        meaning: &'static str,
        correct_tone: u8,
        /// 0 = teaching phase, 1 = quiz phase
        phase: u8,
        score: usize,
    },
    /// CodexShrine: quiz on previously encountered characters (3 rounds)
    CodexChallenge {
        round: usize,
        hanzi: &'static str,
        pinyin: &'static str,
        meaning: &'static str,
        options: [&'static str; 4],
        correct_idx: usize,
        score: usize,
    },
    /// Character Journal: browse codex entries with pagination
    Journal { page: usize },
    /// WordBridge: answer vocab question to create bridge over deep water
    WordBridgeChallenge {
        meaning: &'static str,
        correct_hanzi: &'static str,
        correct_pinyin: &'static str,
        options: [&'static str; 4],
        correct_idx: usize,
        bridge_x: i32,
        bridge_y: i32,
    },
    /// LockedDoor: translation question to unlock
    LockedDoorChallenge {
        hanzi: &'static str,
        pinyin: &'static str,
        correct_meaning: &'static str,
        options: [&'static str; 4],
        correct_idx: usize,
        door_x: i32,
        door_y: i32,
    },
    /// CursedFloor: quick tone quiz trap
    CursedFloorChallenge {
        hanzi: &'static str,
        pinyin: &'static str,
        meaning: &'static str,
        correct_tone: u8,
    },
    /// Tactical grid-based combat (new system).
    TacticalBattle(Box<combat::TacticalBattle>),
}
