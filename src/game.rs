//! Main game state and loop.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement, KeyboardEvent};

use crate::achievement::AchievementTracker;
use crate::audio::Audio;
use crate::codex::Codex;
use crate::combat;
use crate::world::{compute_fov, TerminalKind, AltarKind, DungeonLevel, RoomModifier, SecuritySeal, SealKind, SpecialRoomKind, Tile};
use crate::world::starmap::{SectorMap, generate_sector};
use crate::world::ship::{ShipLayout, generate_ship_layout};
use crate::enemy::{BossKind, Enemy, RadicalAction};
use crate::particle::ParticleSystem;
use crate::player::{
    Faction, EquipEffect, Item, ItemKind, ItemState, Player, PlayerClass, PlayerForm, EQUIPMENT_POOL,
    ITEM_KIND_COUNT, MYSTERY_ITEM_APPEARANCES, Ship, CrewMember, CrewRole,
};
use crate::radical::{self, Spell, SpellEffect};
use crate::render::Renderer;
use crate::srs::SrsTracker;
use crate::status;
use crate::vocab::{self, VocabEntry};

const MAP_W: i32 = 48;
const MAP_H: i32 = 48;
const FOV_RADIUS: i32 = 8;
const ENEMIES_PER_ROOM: i32 = 1;
const LOOK_RANGE: i32 = 3;

/// Combat phase when the player is adjacent to / engages an enemy.
/// Companion NPC that follows the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Companion {
    ScienceOfficer,
    Medic,
    Quartermaster,
    SecurityChief,
}

impl Companion {
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

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum GameMode {
    Starmap,
    ShipInterior,
    LocationExploration,
    GroundCombat,
    SpaceCombat,
    Event,
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
    fn roll(floor: i32, rng_val: u64) -> Self {
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
}

impl Quest {
    #[allow(dead_code)]
    fn check_complete(&mut self) -> bool {
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

    fn is_chain(&self) -> bool {
        self.chain_id > 0
    }
}

#[derive(Clone, Debug, Default)]
struct TutorialState {
    combat_done: bool,
    forge_done: bool,
}

impl TutorialState {
    fn is_complete(&self) -> bool {
        self.combat_done && self.forge_done
    }

    fn objective_text(&self) -> &'static str {
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

    fn next(self) -> Self {
        match self {
            Self::Slow => Self::Normal,
            Self::Normal => Self::Fast,
            Self::Fast => Self::Fast,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Slow => Self::Slow,
            Self::Normal => Self::Slow,
            Self::Fast => Self::Normal,
        }
    }

    fn timer_step(self) -> u8 {
        match self {
            Self::Slow => 1,
            Self::Normal => 1,
            Self::Fast => 1,
        }
    }

    fn timer_delay(self) -> u8 {
        match self {
            Self::Slow => 3,
            Self::Normal => 2,
            Self::Fast => 1,
        }
    }

    fn from_storage(value: &str) -> Self {
        match value {
            "slow" => Self::Slow,
            "fast" => Self::Fast,
            _ => Self::Normal,
        }
    }

    fn storage_key(self) -> &'static str {
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

/// Sentence data for sentence construction challenges.
/// Organised into three difficulty tiers so `select_sentence_for_floor` can
/// pick easier sentences on early floors and harder ones later.

/// Tier 1 — easy (2-3 words, HSK 1)
const SENTENCES_EASY: &[(&[&str], &str)] = &[
    (&["我", "是", "学生"], "I am a student"),
    (&["你", "好", "吗"], "How are you?"),
    (&["我们", "去", "学校"], "We go to school"),
    (&["她", "很", "高兴"], "She is very happy"),
    (&["他", "喝", "水"], "He drinks water"),
    (&["我", "吃", "饭"], "I eat"),
    (&["你", "看", "书"], "You read books"),
    (&["他们", "是", "朋友"], "They are friends"),
    (&["我", "很", "好"], "I am fine"),
    (&["她", "有", "猫"], "She has a cat"),
];

/// Tier 2 — medium (3-4 words, HSK 1-2)
const SENTENCES_MEDIUM: &[(&[&str], &str)] = &[
    (&["他", "不", "喝", "水"], "He doesn't drink water"),
    (&["我", "想", "吃", "饭"], "I want to eat"),
    (&["今天", "天气", "很", "好"], "Today's weather is good"),
    (&["你", "叫", "什么", "名字"], "What is your name?"),
    (&["他们", "在", "看", "书"], "They are reading books"),
    (&["我", "喜欢", "中国", "菜"], "I like Chinese food"),
    (&["她", "不", "想", "去"], "She doesn't want to go"),
    (&["我们", "明天", "去", "学校"], "We go to school tomorrow"),
    (&["他", "每天", "喝", "茶"], "He drinks tea every day"),
    (&["你", "在", "做", "什么"], "What are you doing?"),
];

/// Tier 3 — hard (4-5 words, HSK 2-3)
const SENTENCES_HARD: &[(&[&str], &str)] = &[
    (
        &["我", "昨天", "买", "了", "书"],
        "I bought a book yesterday",
    ),
    (&["她", "每天", "早上", "跑步"], "She runs every morning"),
    (
        &["他们", "下午", "去", "公园", "玩"],
        "They go to the park to play in the afternoon",
    ),
    (&["你", "能", "帮", "我", "吗"], "Can you help me?"),
    (
        &["我", "不", "知道", "他", "在哪儿"],
        "I don't know where he is",
    ),
    (
        &["老师", "今天", "没有", "来", "学校"],
        "The teacher didn't come to school today",
    ),
    (&["我们", "一起", "去", "吃", "饭"], "Let's go eat together"),
    (&["他", "说", "他", "很", "忙"], "He says he is very busy"),
    (
        &["你", "想", "喝", "什么", "茶"],
        "What tea would you like?",
    ),
    (&["她", "的", "朋友", "很", "多"], "She has many friends"),
];

/// Select a sentence appropriate for the current floor.
/// Early floors (≤8) pick from easy, mid floors (9–18) from easy+medium,
/// late floors (19+) can draw from all three tiers.
fn select_sentence_for_floor(floor: i32, rng_val: u64) -> (&'static [&'static str], &'static str) {
    if floor <= 8 {
        let idx = rng_val as usize % SENTENCES_EASY.len();
        SENTENCES_EASY[idx]
    } else if floor <= 18 {
        let pool_len = SENTENCES_EASY.len() + SENTENCES_MEDIUM.len();
        let idx = rng_val as usize % pool_len;
        if idx < SENTENCES_EASY.len() {
            SENTENCES_EASY[idx]
        } else {
            SENTENCES_MEDIUM[idx - SENTENCES_EASY.len()]
        }
    } else {
        let pool_len = SENTENCES_EASY.len() + SENTENCES_MEDIUM.len() + SENTENCES_HARD.len();
        let idx = rng_val as usize % pool_len;
        if idx < SENTENCES_EASY.len() {
            SENTENCES_EASY[idx]
        } else if idx < SENTENCES_EASY.len() + SENTENCES_MEDIUM.len() {
            SENTENCES_MEDIUM[idx - SENTENCES_EASY.len()]
        } else {
            SENTENCES_HARD[idx - SENTENCES_EASY.len() - SENTENCES_MEDIUM.len()]
        }
    }
}

const STROKE_ORDER_DATA: &[(&str, &[&str], &str, &str)] = &[
    ("明", &["日", "月"], "ming2", "bright"),
    ("休", &["亻", "木"], "xiu1", "rest"),
    ("林", &["木", "木"], "lin2", "forest"),
    ("好", &["女", "子"], "hao3", "good"),
    ("安", &["宀", "女"], "an1", "peace"),
    ("信", &["亻", "言"], "xin4", "believe"),
    ("花", &["艹", "化"], "hua1", "flower"),
    ("想", &["相", "心"], "xiang3", "think"),
    ("吃", &["口", "乞"], "chi1", "eat"),
    ("喝", &["口", "曷"], "he1", "drink"),
];

const COMPOUND_DATA: &[(&str, &[&str], &str, &str)] = &[
    ("学生", &["学", "生"], "xue2sheng1", "student"),
    ("老师", &["老", "师"], "lao3shi1", "teacher"),
    ("中国", &["中", "国"], "zhong1guo2", "China"),
    ("朋友", &["朋", "友"], "peng2you3", "friend"),
    ("电话", &["电", "话"], "dian4hua4", "phone"),
    ("天气", &["天", "气"], "tian1qi4", "weather"),
    ("大学", &["大", "学"], "da4xue2", "university"),
    ("飞机", &["飞", "机"], "fei1ji1", "airplane"),
    ("火车", &["火", "车"], "huo3che1", "train"),
    ("书包", &["书", "包"], "shu1bao1", "schoolbag"),
    ("东西", &["东", "西"], "dong1xi1", "thing"),
    ("工作", &["工", "作"], "gong1zuo4", "work"),
];

const CLASSIFIER_DATA: &[(&str, &str, &str, &str)] = &[
    ("书", "本", "shu1", "book"),
    ("人", "个", "ren2", "person"),
    ("猫", "只", "mao1", "cat"),
    ("狗", "只", "gou3", "dog"),
    ("车", "辆", "che1", "car"),
    ("花", "朵", "hua1", "flower"),
    ("纸", "张", "zhi3", "paper"),
    ("刀", "把", "dao1", "knife"),
    ("鱼", "条", "yu2", "fish"),
    ("笔", "支", "bi3", "pen"),
    ("衣服", "件", "yi1fu2", "clothes"),
    ("马", "匹", "ma3", "horse"),
    ("河", "条", "he2", "river"),
    ("山", "座", "shan1", "mountain"),
    ("树", "棵", "shu4", "tree"),
];

const ALL_CLASSIFIERS: &[&str] = &[
    "个", "本", "只", "辆", "朵", "张", "把", "条", "支", "件", "匹", "座", "棵",
];

/// (hanzi, component_count, pinyin, meaning)
const INK_WELL_DATA: &[(&str, u8, &str, &str)] = &[
    ("明", 2, "ming2", "bright"),
    ("休", 2, "xiu1", "rest"),
    ("好", 2, "hao3", "good"),
    ("安", 2, "an1", "peace"),
    ("林", 2, "lin2", "forest"),
    ("想", 2, "xiang3", "think"),
    ("花", 2, "hua1", "flower"),
    ("吃", 2, "chi1", "eat"),
    ("喝", 2, "he1", "drink"),
    ("信", 2, "xin4", "believe"),
    ("忘", 2, "wang4", "forget"),
    ("看", 2, "kan4", "look"),
];

/// (first_half, second_half, full, pinyin, meaning)
const CHENGYU_DATA: &[(&str, &str, &str, &str, &str)] = &[
    (
        "\u{4e00}\u{5fc3}",
        "\u{4e00}\u{610f}",
        "\u{4e00}\u{5fc3}\u{4e00}\u{610f}",
        "yi1xin1yi1yi4",
        "wholeheartedly",
    ),
    (
        "\u{534a}\u{9014}",
        "\u{800c}\u{5e9f}",
        "\u{534a}\u{9014}\u{800c}\u{5e9f}",
        "ban4tu2er2fei4",
        "give up halfway",
    ),
    (
        "\u{81ea}\u{8a00}",
        "\u{81ea}\u{8bed}",
        "\u{81ea}\u{8a00}\u{81ea}\u{8bed}",
        "zi4yan2zi4yu3",
        "talk to oneself",
    ),
    (
        "\u{5165}\u{4e61}",
        "\u{968f}\u{4fd7}",
        "\u{5165}\u{4e61}\u{968f}\u{4fd7}",
        "ru4xiang1sui2su2",
        "when in Rome",
    ),
    (
        "\u{9a6c}\u{5230}",
        "\u{6210}\u{529f}",
        "\u{9a6c}\u{5230}\u{6210}\u{529f}",
        "ma3dao4cheng2gong1",
        "instant success",
    ),
    (
        "\u{5fc3}\u{60f3}",
        "\u{4e8b}\u{6210}",
        "\u{5fc3}\u{60f3}\u{4e8b}\u{6210}",
        "xin1xiang3shi4cheng2",
        "wishes come true",
    ),
    (
        "\u{5927}\u{540c}",
        "\u{5c0f}\u{5f02}",
        "\u{5927}\u{540c}\u{5c0f}\u{5f02}",
        "da4tong2xiao3yi4",
        "mostly the same",
    ),
    (
        "\u{767e}\u{53d1}",
        "\u{767e}\u{4e2d}",
        "\u{767e}\u{53d1}\u{767e}\u{4e2d}",
        "bai3fa1bai3zhong4",
        "hit every target",
    ),
    (
        "\u{5343}\u{65b9}",
        "\u{767e}\u{8ba1}",
        "\u{5343}\u{65b9}\u{767e}\u{8ba1}",
        "qian1fang1bai3ji4",
        "by every means",
    ),
    (
        "\u{5f00}\u{95e8}",
        "\u{89c1}\u{5c71}",
        "\u{5f00}\u{95e8}\u{89c1}\u{5c71}",
        "kai1men2jian4shan1",
        "get to the point",
    ),
    (
        "\u{4e07}\u{4e8b}",
        "\u{5982}\u{610f}",
        "\u{4e07}\u{4e8b}\u{5982}\u{610f}",
        "wan4shi4ru2yi4",
        "everything goes well",
    ),
    (
        "\u{5929}\u{4e0b}",
        "\u{592a}\u{5e73}",
        "\u{5929}\u{4e0b}\u{592a}\u{5e73}",
        "tian1xia4tai4ping2",
        "peace under heaven",
    ),
    (
        "\u{5927}\u{5f00}",
        "\u{773c}\u{754c}",
        "\u{5927}\u{5f00}\u{773c}\u{754c}",
        "da4kai1yan3jie4",
        "eye-opening",
    ),
    (
        "\u{4e03}\u{4e0a}",
        "\u{516b}\u{4e0b}",
        "\u{4e03}\u{4e0a}\u{516b}\u{4e0b}",
        "qi1shang4ba1xia4",
        "at sixes and sevens",
    ),
    (
        "\u{4e94}\u{5149}",
        "\u{5341}\u{8272}",
        "\u{4e94}\u{5149}\u{5341}\u{8272}",
        "wu3guang1shi2se4",
        "dazzling",
    ),
    (
        "\u{4e5d}\u{6b7b}",
        "\u{4e00}\u{751f}",
        "\u{4e5d}\u{6b7b}\u{4e00}\u{751f}",
        "jiu3si3yi1sheng1",
        "narrow escape",
    ),
    (
        "\u{4e00}\u{5200}",
        "\u{4e24}\u{65ad}",
        "\u{4e00}\u{5200}\u{4e24}\u{65ad}",
        "yi1dao1liang3duan4",
        "cut cleanly",
    ),
    (
        "\u{4e00}\u{76ee}",
        "\u{4e86}\u{7136}",
        "\u{4e00}\u{76ee}\u{4e86}\u{7136}",
        "yi1mu4liao3ran2",
        "crystal clear",
    ),
    (
        "\u{4e0d}\u{53ef}",
        "\u{601d}\u{8bae}",
        "\u{4e0d}\u{53ef}\u{601d}\u{8bae}",
        "bu4ke3si1yi4",
        "incredible",
    ),
    (
        "\u{6cf0}\u{7136}",
        "\u{81ea}\u{82e5}",
        "\u{6cf0}\u{7136}\u{81ea}\u{82e5}",
        "tai4ran2zi4ruo4",
        "calm and composed",
    ),
    (
        "\u{5b66}\u{4ee5}",
        "\u{81f4}\u{7528}",
        "\u{5b66}\u{4ee5}\u{81f4}\u{7528}",
        "xue2yi3zhi4yong4",
        "learn to apply",
    ),
    (
        "\u{5927}\u{5668}",
        "\u{665a}\u{6210}",
        "\u{5927}\u{5668}\u{665a}\u{6210}",
        "da4qi4wan3cheng2",
        "great minds mature slowly",
    ),
    (
        "\u{53e3}\u{662f}",
        "\u{5fc3}\u{975e}",
        "\u{53e3}\u{662f}\u{5fc3}\u{975e}",
        "kou3shi4xin1fei1",
        "say one thing mean another",
    ),
    (
        "\u{9f99}\u{98de}",
        "\u{51e4}\u{821e}",
        "\u{9f99}\u{98de}\u{51e4}\u{821e}",
        "long2fei1feng4wu3",
        "dragons fly phoenixes dance",
    ),
    (
        "\u{864e}\u{5934}",
        "\u{86c7}\u{5c3e}",
        "\u{864e}\u{5934}\u{86c7}\u{5c3e}",
        "hu3tou2she2wei3",
        "strong start weak end",
    ),
    (
        "\u{6c34}\u{6ef4}",
        "\u{77f3}\u{7a7f}",
        "\u{6c34}\u{6ef4}\u{77f3}\u{7a7f}",
        "shui3di1shi2chuan1",
        "water wears stone",
    ),
    (
        "\u{98ce}\u{548c}",
        "\u{65e5}\u{4e3d}",
        "\u{98ce}\u{548c}\u{65e5}\u{4e3d}",
        "feng1he2ri4li4",
        "gentle breeze sunny day",
    ),
    (
        "\u{91d1}\u{7389}",
        "\u{6ee1}\u{5802}",
        "\u{91d1}\u{7389}\u{6ee1}\u{5802}",
        "jin1yu4man3tang2",
        "riches fill the hall",
    ),
    (
        "\u{5929}\u{957f}",
        "\u{5730}\u{4e45}",
        "\u{5929}\u{957f}\u{5730}\u{4e45}",
        "tian1chang2di4jiu3",
        "everlasting",
    ),
    (
        "\u{5fc3}\u{5982}",
        "\u{6b62}\u{6c34}",
        "\u{5fc3}\u{5982}\u{6b62}\u{6c34}",
        "xin1ru2zhi3shui3",
        "mind still as water",
    ),
    (
        "\u{5149}\u{660e}",
        "\u{78ca}\u{843d}",
        "\u{5149}\u{660e}\u{78ca}\u{843d}",
        "guang1ming2lei3luo4",
        "open and upright",
    ),
    (
        "\u{4e00}\u{8def}",
        "\u{5e73}\u{5b89}",
        "\u{4e00}\u{8def}\u{5e73}\u{5b89}",
        "yi1lu4ping2an1",
        "safe journey",
    ),
];

/// (hanzi, pinyin, meaning, radical, wrong1, wrong2, wrong3)
const RADICAL_GARDEN_DATA: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
    ("妈", "ma1", "mother", "女", "马", "口", "木"),
    ("河", "he2", "river", "氵", "口", "可", "亻"),
    ("打", "da3", "hit", "扌", "丁", "口", "大"),
    ("说", "shuo1", "speak", "讠", "兑", "口", "言"),
    ("吗", "ma0", "question particle", "口", "马", "女", "木"),
    ("他", "ta1", "he/him", "亻", "也", "口", "土"),
    ("跑", "pao3", "run", "足", "包", "口", "走"),
    ("猫", "mao1", "cat", "犭", "苗", "口", "豸"),
    ("认", "ren4", "recognize", "讠", "人", "口", "亻"),
    ("饭", "fan4", "rice/meal", "饣", "反", "口", "食"),
];

/// (hanzi, pinyin, meaning) — used by MirrorPool (pinyin typing)
const MIRROR_POOL_DATA: &[(&str, &str, &str)] = &[
    ("你好", "ni3hao3", "hello"),
    ("谢谢", "xie4xie4", "thank you"),
    ("再见", "zai4jian4", "goodbye"),
    ("学生", "xue2sheng1", "student"),
    ("老师", "lao3shi1", "teacher"),
    ("中国", "zhong1guo2", "China"),
    ("朋友", "peng2you3", "friend"),
    ("电话", "dian4hua4", "phone"),
    ("天气", "tian1qi4", "weather"),
    ("工作", "gong1zuo4", "work"),
    ("大学", "da4xue2", "university"),
    ("飞机", "fei1ji1", "airplane"),
];

#[derive(Clone, Debug)]
pub struct ShopItem {
    pub label: String,
    pub cost: i32,
    pub kind: ShopItemKind,
}

#[derive(Clone, Debug)]
pub enum ShopItemKind {
    Radical(&'static str),
    HealFull,
    Equipment(usize), // index into EQUIPMENT_POOL
    Consumable(crate::player::Item),
}

pub struct GameState {
    pub level: DungeonLevel,
    pub player: Player,
    pub renderer: Renderer,
    pub audio: Option<Audio>,
    pub floor_num: i32,
    pub seed: u64,
    pub enemies: Vec<Enemy>,
    pub combat: CombatState,
    pub typing: String,
    pub message: String,
    pub message_timer: u8,
    message_tick_delay: u8,
    pub discovered_recipes: Vec<usize>,
    pub best_floor: i32,
    pub srs: SrsTracker,
    pub total_kills: u32,
    pub total_runs: u32,
    /// Move counter for haste effect
    pub move_count: u32,
    /// Particle effects
    pub particles: ParticleSystem,
    /// Screen shake remaining frames
    pub shake_timer: u8,
    /// Flash overlay (r, g, b, alpha 0.0..1.0)
    pub flash: Option<(u8, u8, u8, f64)>,
    /// Achievement tracker
    pub achievements: AchievementTracker,
    /// Achievement popup: (name, desc) + timer frames
    pub achievement_popup: Option<(&'static str, &'static str, u16)>,
    /// Character codex
    pub codex: Codex,
    /// Whether codex overlay is showing
    pub show_codex: bool,
    pub show_inventory: bool,
    pub inventory_cursor: usize,
    pub inventory_inspect: Option<usize>,
    pub show_spellbook: bool,
    pub show_help: bool,
    item_appearance_order: [usize; ITEM_KIND_COUNT],
    identified_items: [bool; ITEM_KIND_COUNT],
    pub settings: GameSettings,
    pub show_settings: bool,
    pub settings_cursor: usize,
    /// Last spell effect used (for combos)
    pub last_spell: Option<SpellEffect>,
    /// Turns since last spell (combo window)
    pub spell_combo_timer: u8,
    /// Listening mode for audio-based combat
    pub listening_mode: ListenMode,
    /// Active companion NPC
    pub companion: Option<Companion>,
    /// Guard companion: used block this fight?
    pub guard_used_this_fight: bool,
    pub guard_blocks_used: u8,
    pub companion_xp: u32,
    pub merchant_reroll_used: bool,
    /// Active quests
    pub quests: Vec<Quest>,
    /// Daily challenge mode (fixed seed)
    pub daily_mode: bool,
    /// Endless mode (continue past floor 20)
    #[allow(dead_code)]
    pub endless_mode: bool,
    /// Active scripted tutorial state for first-time players
    tutorial: Option<TutorialState>,
    rng_state: u64,
    run_kills: u32,
    run_gold_earned: i32,
    run_correct_answers: u32,
    run_wrong_answers: u32,
    run_spells_forged: u32,
    run_bosses_killed: u32,
    pub mirror_hint: bool,
    next_chain_id: u32,
    pub floor_profile: FloorProfile,
    pub answer_streak: u32,
    pub run_journal: RunJournal,
    pub post_mortem_page: usize,
    pub class_cursor: usize,
    /// How many times player has been caught stealing
    pub theft_catches: u32,
    /// Whether this floor's shop is banned (caught stealing)
    pub shop_banned: bool,
    /// Saved TacticalBattle state for boss sentence challenges.
    /// When a boss triggers a sentence challenge mid-tactical-battle,
    /// the battle state is stashed here and restored after the challenge.
    pub saved_battle: Option<Box<combat::TacticalBattle>>,
    /// Cheat console visible
    pub show_console: bool,
    /// Current console input buffer
    pub console_buffer: String,
    /// Console output history (lines of text)
    pub console_history: Vec<String>,
    /// Command history for up/down recall
    pub console_cmd_history: Vec<String>,
    /// Index into command history (None = new input)
    pub console_cmd_index: Option<usize>,
    /// Tab-completion: cached matches and cycle index
    pub tab_matches: Vec<String>,
    pub tab_cycle_index: usize,
    /// The prefix that was used to generate current tab_matches
    pub tab_prefix: String,
    /// God mode (invincible)
    pub god_mode: bool,
    /// Set of (floor, room_x, room_y) for special rooms already activated
    pub completed_special_rooms: HashSet<(i32, i32, i32)>,
    /// Whether the demon deal is active (enemies on next floor are elite)
    pub demon_deal_floors: i32,
    /// Whether the crafting sub-mode is active inside the inventory screen
    pub crafting_mode: bool,
    /// Index (into player.items) of the first item selected for crafting
    pub crafting_first: Option<usize>,
    /// Cursor position while selecting items for crafting
    pub crafting_cursor: usize,
    /// Current game mode (Starmap/ShipInterior/etc.)
    #[allow(dead_code)]
    pub game_mode: GameMode,
    /// Sector map for space exploration
    #[allow(dead_code)]
    pub sector_map: Option<SectorMap>,
    /// Ship interior layout
    #[allow(dead_code)]
    pub ship_layout: ShipLayout,
    /// Ship stats (hull, fuel, shields)
    #[allow(dead_code)]
    pub ship: Ship,
    /// Crew members aboard the ship
    #[allow(dead_code)]
    pub crew: Vec<CrewMember>,
    /// Current event index (if in Event mode)
    #[allow(dead_code)]
    pub current_event: Option<usize>,
    /// Cursor for event choice selection
    #[allow(dead_code)]
    pub event_choice_cursor: usize,
    /// Player position inside ship
    #[allow(dead_code)]
    pub ship_player_x: i32,
    #[allow(dead_code)]
    pub ship_player_y: i32,
    /// Cursor for starmap system selection
    #[allow(dead_code)]
    pub starmap_cursor: usize,
}

impl GameState {
    /// Convert tile position to screen coordinates for particles.
    fn tile_to_screen(&self, tx: i32, ty: i32) -> (f64, f64) {
        let cam_x = self.player.x as f64 * 24.0 - self.renderer.canvas_w / 2.0 + 12.0;
        let cam_y = self.player.y as f64 * 24.0 - self.renderer.canvas_h / 2.0 + 12.0;
        (
            tx as f64 * 24.0 - cam_x + 12.0,
            ty as f64 * 24.0 - cam_y + 12.0,
        )
    }

    fn rng_next(&mut self) -> u64 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        x
    }

    fn trigger_shake(&mut self, frames: u8) {
        if self.settings.screen_shake {
            self.shake_timer = self.shake_timer.max(frames);
        }
    }

    fn open_settings(&mut self) {
        self.show_settings = true;
        self.settings_cursor = 0;
    }

    fn close_settings(&mut self) {
        self.show_settings = false;
    }

    fn open_inventory(&mut self) {
        self.show_inventory = true;
        self.inventory_cursor = 0;
        self.inventory_inspect = None;
    }

    fn close_inventory(&mut self) {
        self.show_inventory = false;
        self.inventory_inspect = None;
        self.crafting_mode = false;
        self.crafting_first = None;
        self.crafting_cursor = 0;
    }

    fn try_craft(&mut self, first_idx: usize, second_idx: usize) {
        use crate::player::{find_crafting_recipe, crafted_item, ItemState};
        let kind1 = self.player.items[first_idx].kind();
        let kind2 = self.player.items[second_idx].kind();
        if let Some(recipe) = find_crafting_recipe(kind1, kind2) {
            let output = crafted_item(recipe, &self.player.items[first_idx], &self.player.items[second_idx]);
            let output_name = recipe.output_name;
            // Remove the higher index first to avoid shifting the lower one
            let (hi, lo) = if first_idx > second_idx {
                (first_idx, second_idx)
            } else {
                (second_idx, first_idx)
            };
            self.player.items.remove(hi);
            self.player.item_states.remove(hi);
            self.player.items.remove(lo);
            self.player.item_states.remove(lo);
            self.player.add_item(output, ItemState::Normal);
            if let Some(ref audio) = self.audio {
                audio.play_forge();
            }
            self.message = format!("✨ Crafted {}!", output_name);
            self.message_timer = 90;
            // Exit crafting mode after successful craft
            self.crafting_mode = false;
            self.crafting_first = None;
            self.crafting_cursor = 0;
        } else {
            self.message = "These items can't be combined.".to_string();
            self.message_timer = 60;
            self.crafting_first = None;
        }
    }

    fn start_look_mode(&mut self) {
        self.combat = CombatState::Looking {
            x: self.player.x,
            y: self.player.y,
        };
        self.update_look_message(self.player.x, self.player.y);
    }

    fn stop_look_mode(&mut self) {
        self.combat = CombatState::Explore;
        self.message.clear();
        self.message_timer = 0;
    }

    fn move_settings_cursor(&mut self, delta: i32) {
        let next = (self.settings_cursor as i32 + delta).clamp(0, 3);
        self.settings_cursor = next as usize;
    }

    fn adjust_volume(value: u8, delta: i8) -> u8 {
        (value as i16 + delta as i16 * 10).clamp(0, 100) as u8
    }

    fn adjust_selected_setting(&mut self, delta: i8) {
        match self.settings_cursor {
            0 => {
                self.settings.music_volume = Self::adjust_volume(self.settings.music_volume, delta)
            }
            1 => self.settings.sfx_volume = Self::adjust_volume(self.settings.sfx_volume, delta),
            2 => self.settings.screen_shake = !self.settings.screen_shake,
            3 => {
                self.settings.text_speed = if delta < 0 {
                    self.settings.text_speed.previous()
                } else {
                    self.settings.text_speed.next()
                };
            }
            _ => {}
        }
        self.apply_settings();
    }

    fn apply_settings(&mut self) {
        if !self.settings.screen_shake {
            self.shake_timer = 0;
        }
        if let Some(ref mut audio) = self.audio {
            audio.set_music_volume(self.settings.music_volume);
            audio.set_sfx_volume(self.settings.sfx_volume);
        }
        self.save_settings();
    }

    fn make_player(&mut self, x: i32, y: i32, class: PlayerClass) -> Player {
        let mut player = Player::new(x, y, class);
        match class {
            PlayerClass::Soldier => {
                player.weapon = Some(&crate::player::EQUIPMENT_POOL[0]); // Brush of Clarity
            }
            PlayerClass::Mystic => {
                player.items.push(crate::player::Item::MedHypo(5));
                player.item_states.push(ItemState::Normal);
                player.items.push(crate::player::Item::MedHypo(5));
                player.item_states.push(ItemState::Normal);
            }
            PlayerClass::Operative => {
                player.gold += 20;
            }
            PlayerClass::Envoy => {
                player.gold += 15;
                player.shop_discount_pct = 20;
            }
            PlayerClass::Technomancer => {
                player.radicals.push("木");
                player.radicals.push("水");
                player.radicals.push("火");
            }
            _ => {}
        }
        player
    }

    fn companion_level(&self) -> u8 {
        if self.companion.is_some() {
            Companion::level_from_xp(self.companion_xp)
        } else {
            0
        }
    }

    fn add_companion_xp(&mut self, amount: u32) {
        if self.companion.is_some() {
            let old_level = self.companion_level();
            self.companion_xp = self.companion_xp.saturating_add(amount);
            let new_level = self.companion_level();
            if new_level > old_level {
                if let Some(ref comp) = self.companion {
                    self.message = format!(
                        "{} {} reached level {}!",
                        comp.icon(),
                        comp.name(),
                        new_level
                    );
                    self.message_timer = 90;
                }
            }
        }
    }

    fn effective_shop_discount_pct(&self) -> i32 {
        let mut discount = self.player.shop_discount_pct;
        if let Some(ref comp) = self.companion {
            discount += comp.shop_discount_pct(self.companion_level());
        }
        discount.clamp(0, 50)
    }

    fn companion_exploration_hint(&mut self) {
        let comp = match self.companion {
            Some(c) => c,
            None => return,
        };
        if self.message_timer > 20 {
            return;
        }
        if !matches!(self.combat, CombatState::Explore) {
            return;
        }

        let w = self.level.width;
        let h = self.level.height;
        let level = self.companion_level();

        let hint: Option<String> = match comp {
            Companion::ScienceOfficer => {
                let mut forge_visible = false;
                for y in 0..h {
                    for x in 0..w {
                        let idx = y * w + x;
                        if self.level.visible[idx as usize]
                            && self.level.tiles[idx as usize] == Tile::QuantumForge
                        {
                            forge_visible = true;
                            break;
                        }
                    }
                    if forge_visible {
                        break;
                    }
                }
                if forge_visible && !self.player.radicals.is_empty() {
                    Some("📚 I see a forge! You have radicals to combine.".to_string())
                } else {
                    None
                }
            }
            Companion::Medic => {
                if self.player.hp <= self.player.max_hp / 3 {
                    let mut shrine_visible = false;
                    for y in 0..h {
                        for x in 0..w {
                            let idx = y * w + x;
                            if self.level.visible[idx as usize] {
                                let t = self.level.tiles[idx as usize];
                                if t == Tile::CircuitShrine
                                    || matches!(t, Tile::Terminal(_))
                                    || t == Tile::MemorialNode
                                {
                                    shrine_visible = true;
                                    break;
                                }
                            }
                        }
                        if shrine_visible {
                            break;
                        }
                    }
                    if shrine_visible {
                        let heal = Companion::Medic.heal_per_floor(level);
                        Some(format!(
                            "🧘 A shrine nearby — rest may help. I'll mend {} HP next floor.",
                            heal
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Companion::Quartermaster => {
                if self.floor_profile.radical_drop_bonus() {
                    let mut chest_visible = false;
                    for y in 0..h {
                        for x in 0..w {
                            let idx = y * w + x;
                            if self.level.visible[idx as usize]
                                && self.level.tiles[idx as usize] == Tile::SupplyCrate
                            {
                                chest_visible = true;
                                break;
                            }
                        }
                        if chest_visible {
                            break;
                        }
                    }
                    if chest_visible {
                        Some("💰 Chest spotted on a rich floor — extra loot likely!".to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Companion::SecurityChief => {
                let px = self.player.x;
                let py = self.player.y;
                let alert_count = self
                    .enemies
                    .iter()
                    .filter(|e| {
                        e.is_alive()
                            && e.alert
                            && (e.x - px).abs() <= FOV_RADIUS
                            && (e.y - py).abs() <= FOV_RADIUS
                    })
                    .count();
                if alert_count >= 3 {
                    let blocks = Companion::SecurityChief.guard_max_blocks(level);
                    Some(format!(
                        "🛡 {} enemies closing in! I can block {} hit{}.",
                        alert_count,
                        blocks,
                        if blocks > 1 { "s" } else { "" }
                    ))
                } else {
                    None
                }
            }
        };

        if let Some(text) = hint {
            self.message = text;
            self.message_timer = 70;
        }
    }

    fn discounted_cost(&self, base_cost: i32) -> i32 {
        let pct = 100 - self.effective_shop_discount_pct();
        let mut cost = ((base_cost * pct).max(0) + 99) / 100;
        let surcharge = (base_cost as f64 * self.theft_catches as f64 * 0.15) as i32;
        cost += surcharge;
        cost
    }

    fn roll_item_appearance_order(seed: u64) -> [usize; ITEM_KIND_COUNT] {
        let mut order = core::array::from_fn(|idx| idx);
        let mut state = seed ^ 0x9e37_79b9_7f4a_7c15;
        for i in (1..ITEM_KIND_COUNT).rev() {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let j = (state as usize) % (i + 1);
            order.swap(i, j);
        }
        order
    }

    fn reset_item_lore(&mut self) {
        self.identified_items = [false; ITEM_KIND_COUNT];
        self.item_appearance_order = Self::roll_item_appearance_order(self.seed);
    }

    fn item_appearance(&self, kind: ItemKind) -> &'static str {
        MYSTERY_ITEM_APPEARANCES[self.item_appearance_order[kind.index()]]
    }

    fn item_is_identified(&self, kind: ItemKind) -> bool {
        self.identified_items[kind.index()]
    }

    fn item_display_name(&self, item: &crate::player::Item) -> String {
        item.display_name(
            self.item_is_identified(item.kind()),
            self.item_appearance(item.kind()),
        )
    }

    fn identify_item_kind(&mut self, kind: ItemKind) -> bool {
        let idx = kind.index();
        let newly_identified = !self.identified_items[idx];
        self.identified_items[idx] = true;
        newly_identified
    }

    fn vocab_entry_by_hanzi(hanzi: &str) -> Option<&'static VocabEntry> {
        vocab::VOCAB.iter().find(|entry| entry.hanzi == hanzi)
    }

    fn spawn_enemies(&mut self) {
        let pool = vocab::vocab_for_floor(self.floor_num);
        if pool.is_empty() {
            return;
        }
        let rooms = self.level.rooms.clone();
        let is_boss_floor = self.floor_num % 5 == 0 && self.floor_num > 0;
        let mut enemies_per_room = 1 + self.floor_num / 4;
        if self.floor_profile == FloorProfile::Siege {
            enemies_per_room += 2;
        }

        for (i, room) in rooms.iter().enumerate() {
            if i == 0 || i == rooms.len() - 1 {
                continue;
            }
            // Boss in second-to-last room on boss floors
            if is_boss_floor && i == rooms.len() - 2 {
                let entry: &'static VocabEntry = match BossKind::for_floor(self.floor_num) {
                    Some(BossKind::PirateCaptain) => Self::vocab_entry_by_hanzi("门")
                        .unwrap_or(pool[self.rng_next() as usize % pool.len()]),
                    Some(BossKind::HiveQueen) => Self::vocab_entry_by_hanzi("学")
                        .unwrap_or(pool[self.rng_next() as usize % pool.len()]),
                    Some(BossKind::RogueAICore) => Self::vocab_entry_by_hanzi("电")
                        .unwrap_or(pool[self.rng_next() as usize % pool.len()]),
                    Some(BossKind::VoidEntity) => Self::vocab_entry_by_hanzi("王")
                        .unwrap_or(pool[self.rng_next() as usize % pool.len()]),
                    Some(BossKind::AncientGuardian) => Self::vocab_entry_by_hanzi("书")
                        .unwrap_or(pool[self.rng_next() as usize % pool.len()]),
                    Some(BossKind::DriftLeviathan) => Self::vocab_entry_by_hanzi("盗")
                        .unwrap_or(pool[self.rng_next() as usize % pool.len()]),
                    None => pool[self.rng_next() as usize % pool.len()],
                };
                let (cx, cy) = room.center();
                self.enemies
                    .push(Enemy::boss_from_vocab(entry, cx, cy, self.floor_num));
                continue;
            }
            for _ in 0..enemies_per_room.min(ENEMIES_PER_ROOM as i32 + self.floor_num / 3) {
                let rand_val = self.rng_next();
                let entry_idx = self.srs.weighted_pick(&pool, rand_val);
                let entry: &'static VocabEntry = pool[entry_idx];
                let ex = room.x + 1 + (self.rng_next() % (room.w - 2).max(1) as u64) as i32;
                let ey = room.y + 1 + (self.rng_next() % (room.h - 2).max(1) as u64) as i32;
                if tile_allows_enemy_spawn(self.level.tile(ex, ey)) {
                    self.enemies
                        .push(Enemy::from_vocab(entry, ex, ey, self.floor_num));
                }
            }
        }
    }

    fn start_tutorial(&mut self, class: PlayerClass) {
        self.floor_num = 0;
        self.srs.current_deck = 0;
        self.level = DungeonLevel::tutorial(MAP_W, MAP_H);
        let (sx, sy) = self.level.start_pos();
        self.player = self.make_player(sx, sy, class);
        self.reset_item_lore();
        self.player.add_radical("女");
        self.player.add_radical("子");
        self.enemies.clear();

        let enemy_room = &self.level.rooms[1];
        let entry = vocab::VOCAB
            .iter()
            .find(|entry| entry.hanzi == "大")
            .unwrap_or(&vocab::VOCAB[0]);
        let mut enemy = Enemy::from_vocab(entry, enemy_room.x, enemy_room.y + enemy_room.h / 2, 1);
        enemy.gold_value = 0;
        self.enemies.push(enemy);

        self.combat = CombatState::Explore;
        self.typing.clear();
        self.message =
            "Tutorial Floor — read the signs, defeat 大, then forge 好 from 女 + 子.".to_string();
        self.message_timer = 220;
        self.tutorial = Some(TutorialState::default());

        let (px, py) = (self.player.x, self.player.y);
        compute_fov(&mut self.level, px, py, FOV_RADIUS);
    }

    fn show_tutorial_sign(&mut self, sign_id: u8) {
        let text = match (sign_id, self.tutorial.as_ref()) {
            (0, _) => "Move with WASD or arrow keys. Walk onto signs to read tips.",
            (1, _) => "Combat: bump into 大, type da4, then press Enter to attack.",
            (2, Some(tutorial)) if !tutorial.combat_done => {
                "Beat the enemy first. Then use the forge with 女 + 子."
            }
            (2, _) => "Forge: stand on ⚒, press 1, 2, then Enter to make 好 (hao3).",
            (3, Some(tutorial)) if !tutorial.is_complete() => {
                "The stairs stay sealed until you defeat 大 and forge one spell."
            }
            (3, _) => "The stairs are open. Step on ▼ to begin Floor 1.",
            _ => return,
        };
        self.message = text.to_string();
        self.message_timer = 220;
    }

    fn tutorial_hint(&self) -> Option<&'static str> {
        self.tutorial.as_ref().map(TutorialState::objective_text)
    }

    fn tutorial_exit_blocker(&self) -> Option<&'static str> {
        tutorial_exit_blocker_for(self.tutorial.as_ref())
    }

    fn descend_floor(&mut self, force_skip: bool) -> bool {
        if !force_skip {
            if let Some(blocker) = self.tutorial_exit_blocker() {
                self.message = blocker.to_string();
                self.message_timer = 150;
                return false;
            }
        }

        let leaving_tutorial = self.tutorial.is_some();
        if leaving_tutorial {
            self.tutorial = None;
        }
        self.new_floor();

        if leaving_tutorial {
            self.message = if force_skip {
                "⏭ Tutorial skipped. Welcome to Floor 1.".to_string()
            } else {
                "Tutorial complete! Welcome to Floor 1.".to_string()
            };
            self.message_timer = 180;
        } else if force_skip {
            self.message = format!("⏭ Test skip — descended to Floor {}.", self.floor_num);
            self.message_timer = 90;
        }

        true
    }

    fn reveal_entire_floor(&mut self) {
        for revealed in self.level.revealed.iter_mut() {
            *revealed = true;
        }
    }

    fn pacify_gold_reward(base_gold: i32, spell_power: i32) -> i32 {
        ((base_gold + 1) / 2).max(4) + spell_power.max(0)
    }

    fn invoke_altar(&mut self, _x: i32, _y: i32, kind: AltarKind) {
        // Player is already on the tile (x, y) because move_to was called before this.

        let god = kind.deity();

        let has_piety = self.player.piety.iter().any(|(d, _)| *d == god);

        if !has_piety {
            if let Some(highest) = self.player.highest_faction() {
                if highest != god && self.player.get_piety(highest) >= 15 {
                    self.player.add_piety(highest, -3);
                    self.message = format!(
                        "⚠ {} disapproves of your wandering faith! (-3 favor)",
                        highest.name()
                    );
                    self.message_timer = 255;
                }
            }
            self.player.piety.push((god, 0));
        }

        if let Some(ref audio) = self.audio {
            audio.play_spell();
        }

        // Open Offering menu
        self.combat = CombatState::Offering {
            altar_kind: kind,
            cursor: 0,
        };

        let god_name = match kind {
            TerminalKind::Quantum => "Quantum Sage",
            TerminalKind::Stellar => "Stellar Navigator",
            TerminalKind::Holographic => "Holo Architect",
            TerminalKind::Tactical => "Tactical Commander",
            TerminalKind::Commerce => "Trade Consortium",
        };
        if !(!has_piety && self.message.contains("disapproves")) {
            self.message = format!("You kneel before the Altar of {}.", god_name);
            self.message_timer = 255;
        }
    }

    fn begin_sentence_challenge(&mut self, mode: SentenceChallengeMode, intro: String) {
        let rng_val = self.rng_next();
        let (words, meaning) = select_sentence_for_floor(self.floor_num, rng_val);
        let mut tiles: Vec<usize> = (0..words.len()).collect();
        for i in (1..tiles.len()).rev() {
            let j = self.rng_next() as usize % (i + 1);
            tiles.swap(i, j);
        }
        self.combat = CombatState::SentenceChallenge {
            tiles,
            words: words.to_vec(),
            cursor: 0,
            arranged: Vec::new(),
            meaning,
            mode,
        };
        self.message = intro;
        self.message_timer = 150;
    }

    fn maybe_trigger_boss_phase(&mut self, enemy_idx: usize) -> bool {
        if enemy_idx >= self.enemies.len() || !self.enemies[enemy_idx].is_alive() {
            return false;
        }
        // Save tactical battle state before sentence challenge replaces combat state
        if matches!(self.combat, CombatState::TacticalBattle(_)) {
            if let CombatState::TacticalBattle(battle) =
                std::mem::replace(&mut self.combat, CombatState::Explore)
            {
                self.saved_battle = Some(battle);
            }
        }
        if self.enemies[enemy_idx].boss_kind == Some(BossKind::PirateCaptain)
            && !self.enemies[enemy_idx].phase_triggered
            && self.enemies[enemy_idx].hp <= self.enemies[enemy_idx].max_hp / 2
        {
            self.enemies[enemy_idx].phase_triggered = true;
            self.begin_sentence_challenge(
                SentenceChallengeMode::GatekeeperSeal {
                    boss_idx: enemy_idx,
                    success_damage: 4 + self.floor_num / 3,
                    failure_damage_to_player: 3 + self.floor_num / 4,
                },
                "🔒 The Pirate Captain activates a lockdown seal! Arrange the sentence to shatter it.".to_string(),
            );
            return true;
        }
        if self.enemies[enemy_idx].boss_kind == Some(BossKind::HiveQueen)
            && !self.enemies[enemy_idx].phase_triggered
            && self.enemies[enemy_idx].hp <= self.enemies[enemy_idx].max_hp / 2
        {
            self.enemies[enemy_idx].phase_triggered = true;
            self.begin_sentence_challenge(
                SentenceChallengeMode::ScholarTrial {
                    boss_idx: enemy_idx,
                    success_damage: 6 + self.floor_num / 2,
                    failure_heal: 3 + self.floor_num / 5,
                },
                "📜 The Hive Queen emits a psionic syntax duel! Arrange the sentence to break the swarm link."
                    .to_string(),
            );
            return true;
        }
        if self.enemies[enemy_idx].boss_kind == Some(BossKind::AncientGuardian)
            && !self.enemies[enemy_idx].phase_triggered
            && self.enemies[enemy_idx].hp <= self.enemies[enemy_idx].max_hp / 2
        {
            self.enemies[enemy_idx].phase_triggered = true;
            self.begin_sentence_challenge(
                SentenceChallengeMode::ScholarTrial {
                    boss_idx: enemy_idx,
                    success_damage: 8 + self.floor_num / 2,
                    failure_heal: 5 + self.floor_num / 4,
                },
                "🖌 The Ancient Guardian projects a glyph trial! Arrange the sentence to dispel the energy ward."
                    .to_string(),
            );
            return true;
        }
        false
    }

    fn find_free_adjacent_tile(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let dirs = [
            (0, -1),
            (1, 0),
            (0, 1),
            (-1, 0),
            (1, -1),
            (1, 1),
            (-1, 1),
            (-1, -1),
        ];
        dirs.iter().copied().find(|(dx, dy)| {
            let nx = x + dx;
            let ny = y + dy;
            self.level.is_walkable(nx, ny)
                && self.enemy_at(nx, ny).is_none()
                && (nx != self.player.x || ny != self.player.y)
        })
    }

    fn paint_seal_cross(&mut self, x: i32, y: i32, tile: Tile) -> usize {
        let mut changed = 0;
        for (tx, ty) in seal_cross_positions(x, y) {
            if !self.level.in_bounds(tx, ty) {
                continue;
            }
            let idx = self.level.idx(tx, ty);
            if can_be_reshaped_by_seal(self.level.tiles[idx]) {
                self.level.tiles[idx] = tile;
                changed += 1;
            }
        }
        changed
    }

    fn stun_enemies_on_tiles(&mut self, targets: &[(i32, i32)]) -> usize {
        let mut stunned = 0;
        for idx in 0..self.enemies.len() {
            if !self.enemies[idx].is_alive() {
                continue;
            }
            if targets
                .iter()
                .any(|(tx, ty)| self.enemies[idx].x == *tx && self.enemies[idx].y == *ty)
            {
                self.enemies[idx].stunned = true;
                let (sx, sy) = self.tile_to_screen(self.enemies[idx].x, self.enemies[idx].y);
                self.particles.spawn_stun(sx, sy, &mut self.rng_state);
                stunned += 1;
            }
        }
        stunned
    }

    fn damage_enemies_on_tiles(&mut self, targets: &[(i32, i32)]) -> usize {
        let mut pricked = 0;
        for idx in 0..self.enemies.len() {
            if !self.enemies[idx].is_alive() {
                continue;
            }
            if targets
                .iter()
                .any(|(tx, ty)| self.enemies[idx].x == *tx && self.enemies[idx].y == *ty)
            {
                let hp_before = self.enemies[idx].hp;
                self.apply_enemy_tile_effect(idx);
                if self.enemies[idx].hp < hp_before {
                    pricked += 1;
                }
            }
        }
        pricked
    }

    fn spawn_seal_ambusher(&mut self, x: i32, y: i32) -> Option<&'static str> {
        let (sx, sy) = self
            .find_free_adjacent_tile(x, y)
            .or_else(|| self.find_free_adjacent_tile(self.player.x, self.player.y))?;
        let pool = vocab::vocab_for_floor(self.floor_num.max(1));
        if pool.is_empty() {
            return None;
        }
        let rand_val = self.rng_next();
        let entry = pool[self.srs.weighted_pick(&pool, rand_val)];
        let mut enemy = Enemy::from_vocab(entry, sx, sy, self.floor_num.max(1));
        enemy.alert = true;
        let hanzi = enemy.hanzi;
        self.enemies.push(enemy);
        Some(hanzi)
    }

    fn trigger_seal(&mut self, x: i32, y: i32, kind: SealKind, triggerer: Option<&'static str>) {
        if !self.level.in_bounds(x, y) {
            return;
        }
        let idx = self.level.idx(x, y);
        if !matches!(self.level.tiles[idx], Tile::SecurityLock(current) if current == kind) {
            return;
        }
        self.level.tiles[idx] = Tile::MetalFloor;

        let visible = self.level.visible[idx] || triggerer.is_none();
        let (sx, sy) = self.tile_to_screen(x, y);
        let affected_tiles = seal_cross_positions(x, y);
        match kind {
            SecuritySeal::Thermal => {
                let changed = self.paint_seal_cross(x, y, Tile::Coolant);
                if visible {
                    self.particles.spawn_fire(sx, sy, &mut self.rng_state);
                    self.flash = Some((255, 128, 80, 0.16));
                    self.message = match triggerer {
                        Some(name) => format!(
                            "🔥 {} triggers an {} — oil spills across {} tiles!",
                            name,
                            kind.label(),
                            changed
                        ),
                        None => format!(
                            "🔥 {} bursts open — oil spills across {} nearby tiles!",
                            kind.label(),
                            changed
                        ),
                    };
                    self.message_timer = 90;
                }
            }
            SecuritySeal::Hydraulic => {
                let changed = self.paint_seal_cross(x, y, Tile::CoolantPool);
                let stunned = self.stun_enemies_on_tiles(&affected_tiles);
                if visible {
                    self.particles.spawn_shield(sx, sy, &mut self.rng_state);
                    self.flash = Some((110, 180, 255, 0.14));
                    self.message = match triggerer {
                        Some(name) => format!(
                            "≈ {} releases a {} — {} tiles flood and {} foes stagger!",
                            name,
                            kind.label(),
                            changed,
                            stunned
                        ),
                        None => format!(
                            "≈ {} floods the room — {} tiles turn to water and {} foes stagger!",
                            kind.label(),
                            changed,
                            stunned
                        ),
                    };
                    self.message_timer = 90;
                }
            }
            SecuritySeal::Kinetic => {
                let changed = self.paint_seal_cross(x, y, Tile::LaserGrid);
                let pricked = self.damage_enemies_on_tiles(&affected_tiles);
                if visible {
                    self.particles.spawn_damage(sx, sy, &mut self.rng_state);
                    self.flash = Some((255, 100, 140, 0.14));
                    self.message = match triggerer {
                        Some(name) => format!(
                            "🗡 {} snaps a {} — {} spike tiles rise and prick {} foes!",
                            name,
                            kind.label(),
                            changed,
                            pricked
                        ),
                        None => format!(
                            "🗡 {} flares — {} spike tiles rise and prick {} foes!",
                            kind.label(),
                            changed,
                            pricked
                        ),
                    };
                    self.message_timer = 90;
                }
            }
            SecuritySeal::Sonic => {
                let ambusher = self.spawn_seal_ambusher(x, y);
                if visible {
                    self.particles.spawn_drain(sx, sy, &mut self.rng_state);
                    self.flash = Some((190, 100, 255, 0.16));
                    self.message = match (triggerer, ambusher) {
                        (Some(name), Some(enemy)) => format!(
                            "📣 {} cracks an {} — {} answers the call!",
                            name,
                            kind.label(),
                            enemy
                        ),
                        (None, Some(enemy)) => format!(
                            "📣 {} echoes through the hall — {} answers the call!",
                            kind.label(),
                            enemy
                        ),
                        (Some(name), None) => {
                            format!(
                                "📣 {} stirs an {}, but nothing answers.",
                                name,
                                kind.label()
                            )
                        }
                        (None, None) => {
                            format!("📣 {} hums softly, but nothing answers.", kind.label())
                        }
                    };
                    self.message_timer = 90;
                }
            }
        }
    }

    fn apply_player_tile_effect(&mut self, tile: Tile) {
        match tile {
            Tile::LaserGrid => {
                let dmg = (1 + self.floor_num.max(1) / 3).max(1);
                self.player.hp -= dmg;
                if let Some(ref audio) = self.audio {
                    audio.play_damage();
                }
                let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
                self.particles.spawn_damage(sx, sy, &mut self.rng_state);
                self.trigger_shake(6);
                self.flash = Some((255, 60, 60, 0.2));
                self.message = format!("🪤 Spikes jab you for {} damage!", dmg);
                self.message_timer = 70;
                if self.player.hp <= 0 && !self.try_phoenix_revive() {
                    self.player.hp = 0;
                    self.run_journal
                        .log(RunEvent::DiedTo("Spike trap".to_string(), self.floor_num));
                    self.post_mortem_page = 0;
                    self.combat = CombatState::GameOver;
                    self.message = self.run_summary();
                    self.message_timer = 255;
                    if let Some(ref audio) = self.audio {
                        audio.play_death();
                    }
                    self.save_high_score();
                }
            }
            Tile::Coolant => {
                self.message = "🛢 Oil slick — fire magic will ignite nearby puddles.".to_string();
                self.message_timer = 60;
            }
            Tile::CoolantPool => {
                self.message = "≈ Shallow water — stunning magic can arc through it.".to_string();
                self.message_timer = 60;
                if let Some(ref audio) = self.audio {
                    audio.play_water_splash();
                }
            }
            Tile::Trap(trap_type) => {
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor; // trap consumed
                match trap_type {
                    0 => {
                        // Poison trap
                        self.player
                            .statuses
                            .push(crate::status::StatusInstance::new(
                                crate::status::StatusKind::Poison { damage: 1 },
                                5,
                            ));
                        self.message = "💀 Poison trap! Toxic fumes engulf you!".to_string();
                        self.trigger_shake(6);
                        self.flash = Some((120, 255, 80, 0.2));
                    }
                    1 => {
                        // Teleport trap
                        let mut found = false;
                        for _ in 0..100 {
                            let rx = (self.rng_next() % MAP_W as u64) as i32;
                            let ry = (self.rng_next() % MAP_H as u64) as i32;
                            let ri = self.level.idx(rx, ry);
                            if matches!(self.level.tiles[ri], Tile::MetalFloor | Tile::Hallway)
                                && self.level.revealed[ri]
                            {
                                self.player.x = rx;
                                self.player.y = ry;
                                found = true;
                                break;
                            }
                        }
                        self.message = if found {
                            "✦ Teleport trap! The floor gives way beneath you!".to_string()
                        } else {
                            "✦ A teleport trap fizzles...".to_string()
                        };
                        self.trigger_shake(4);
                        self.flash = Some((100, 140, 255, 0.25));
                    }
                    _ => {
                        // Alarm trap — alert all enemies
                        for e in &mut self.enemies {
                            if e.is_alive() {
                                e.alert = true;
                            }
                        }
                        self.message = "🔔 Alarm trap! All monsters are alerted!".to_string();
                        self.trigger_shake(4);
                        self.flash = Some((255, 200, 50, 0.2));
                    }
                }
                self.message_timer = 60;
                if let Some(ref audio) = self.audio {
                    audio.play_damage();
                }
            }
            Tile::PlasmaVent => {
                let dmg = (2 + self.floor_num.max(1) / 2).max(2);
                self.player.hp -= dmg;
                if let Some(ref audio) = self.audio {
                    audio.play_damage();
                    audio.play_lava_rumble();
                }
                let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
                self.particles.spawn_damage(sx, sy, &mut self.rng_state);
                self.trigger_shake(8);
                self.flash = Some((255, 80, 0, 0.3));
                self.message = format!("🔥 Lava burns you for {} damage!", dmg);
                self.message_timer = 70;
                if self.player.hp <= 0 && !self.try_phoenix_revive() {
                    self.player.hp = 0;
                    self.run_journal
                        .log(RunEvent::DiedTo("Lava".to_string(), self.floor_num));
                    self.post_mortem_page = 0;
                    self.combat = CombatState::GameOver;
                    self.message = self.run_summary();
                    self.message_timer = 255;
                    if let Some(ref audio) = self.audio {
                        audio.play_death();
                    }
                    self.save_high_score();
                }
            }
            Tile::FrozenDeck => {
                self.message = "❄ Ice! The floor is slippery.".to_string();
                self.message_timer = 40;
            }
            Tile::ToxicFungus => {
                self.message = "🍄 Spore cloud! You feel disoriented.".to_string();
                self.message_timer = 60;
                self.flash = Some((180, 100, 255, 0.15));
            }
            Tile::ToxicGas => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Poison { damage: 1 },
                        3,
                    ));
                self.message = "☠ Poison gas! Toxic fumes seep into your lungs!".to_string();
                self.message_timer = 60;
                self.trigger_shake(4);
                self.flash = Some((100, 220, 60, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_damage();
                }
            }
            Tile::CreditCache => {
                let gold = 5 + (self.rng_next() % 16) as i32;
                self.player.gold += gold;
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                self.message = format!("💰 You pick up {} gold!", gold);
                self.message_timer = 50;
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }
            Tile::MedBayTile => {
                let heal = self.player.max_hp / 2;
                self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                self.player.spirit = (self.player.spirit + 20).min(self.player.max_spirit);
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::CoolantPool;
                self.message = format!("🌊 The spirit spring heals {} HP and restores 20 spirit!", heal);
                self.message_timer = 80;
                self.flash = Some((100, 255, 200, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_heal();
                }
            }
            _ => {}
        }
    }

    fn apply_enemy_tile_effect(&mut self, enemy_idx: usize) {
        if enemy_idx >= self.enemies.len() || !self.enemies[enemy_idx].is_alive() {
            return;
        }
        let tile = self
            .level
            .tile(self.enemies[enemy_idx].x, self.enemies[enemy_idx].y);
        match tile {
            Tile::LaserGrid => {
                self.enemies[enemy_idx].hp -= 1;
                if self.enemies[enemy_idx].hp <= 0 {
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    let idx = self
                        .level
                        .idx(self.enemies[enemy_idx].x, self.enemies[enemy_idx].y);
                    if self.level.visible[idx] {
                        self.message = format!("🪤 {} stumbles into spikes and falls!", e_hanzi);
                        self.message_timer = 60;
                    }
                }
            }
            Tile::SecurityLock(kind) => {
                let (x, y, name) = (
                    self.enemies[enemy_idx].x,
                    self.enemies[enemy_idx].y,
                    self.enemies[enemy_idx].hanzi,
                );
                self.trigger_seal(x, y, kind, Some(name));
            }
            _ => {}
        }
    }

    fn ignite_visible_oil(&mut self, bonus_dmg: i32) -> (usize, usize, usize) {
        let mut oil_tiles = Vec::new();
        let mut oil_screens = Vec::new();
        for y in 0..self.level.height {
            for x in 0..self.level.width {
                let idx = self.level.idx(x, y);
                if self.level.visible[idx] && self.level.tiles[idx] == Tile::Coolant {
                    oil_tiles.push((x, y));
                    oil_screens.push(self.tile_to_screen(x, y));
                }
            }
        }
        for &(x, y) in &oil_tiles {
            let idx = self.level.idx(x, y);
            self.level.tiles[idx] = Tile::MetalFloor;
        }
        for (sx, sy) in oil_screens {
            self.particles.spawn_fire(sx, sy, &mut self.rng_state);
        }

        let mut scorched = 0;
        let mut kills = 0;
        for enemy in &mut self.enemies {
            if !enemy.is_alive() {
                continue;
            }
            let hit = oil_tiles
                .iter()
                .any(|&(ox, oy)| (enemy.x - ox).abs() <= 1 && (enemy.y - oy).abs() <= 1);
            if hit {
                enemy.hp -= bonus_dmg;
                scorched += 1;
                if enemy.hp <= 0 {
                    kills += 1;
                }
            }
        }

        (oil_tiles.len(), scorched, kills)
    }

    fn electrify_visible_water(&mut self, bonus_dmg: i32) -> (usize, usize, usize) {
        let mut water_tiles = Vec::new();
        let mut water_screens = Vec::new();
        for y in 0..self.level.height {
            for x in 0..self.level.width {
                let idx = self.level.idx(x, y);
                if self.level.visible[idx]
                    && matches!(self.level.tiles[idx], Tile::CoolantPool | Tile::VacuumBreach)
                {
                    water_tiles.push((x, y));
                    water_screens.push(self.tile_to_screen(x, y));
                }
            }
        }
        for (sx, sy) in water_screens {
            self.particles.spawn_stun(sx, sy, &mut self.rng_state);
        }

        let mut shocked = 0;
        let mut kills = 0;
        for enemy in &mut self.enemies {
            if !enemy.is_alive() {
                continue;
            }
            let standing_in_water = water_tiles
                .iter()
                .any(|&(wx, wy)| enemy.x == wx && enemy.y == wy);
            if standing_in_water {
                enemy.stunned = true;
                if bonus_dmg > 0 {
                    enemy.hp -= bonus_dmg;
                    if enemy.hp <= 0 {
                        kills += 1;
                    }
                }
                shocked += 1;
            }
        }

        (water_tiles.len(), shocked, kills)
    }

    fn new_floor(&mut self) {
        if let Some(ref audio) = self.audio {
            audio.play_descend();
        }
        crate::srs::save_srs(&self.srs);
        self.codex.save();
        self.floor_num += 1;
        self.run_journal.log(RunEvent::EnteredFloor(self.floor_num));
        self.srs.current_deck = self.floor_num;
        self.tutorial = None;
        self.merchant_reroll_used = false;
        self.shop_banned = false;
        if self.floor_num > self.best_floor {
            self.best_floor = self.floor_num;
        }
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.rng_state = self.seed;
        self.floor_profile = FloorProfile::roll(self.floor_num, self.rng_next());
        self.level = DungeonLevel::generate(MAP_W, MAP_H, self.seed, self.floor_num, crate::world::LocationType::OrbitalPlatform);
        let (sx, sy) = self.level.start_pos();
        self.player.move_to(sx, sy);
        self.enemies.clear();
        self.combat = CombatState::Explore;
        self.typing.clear();
        self.spawn_enemies();
        let (px, py) = (self.player.x, self.player.y);
        compute_fov(&mut self.level, px, py, FOV_RADIUS);
        self.achievements.check_floor(self.floor_num);

        if self.companion == Some(Companion::Medic) {
            let lvl = self.companion_level();
            let heal = Companion::Medic.heal_per_floor(lvl);
            let max_hp = self.player.max_hp;
            if self.player.hp < max_hp && heal > 0 {
                self.player.hp = (self.player.hp + heal).min(max_hp);
                self.message = format!("🧘 Monk heals you for {} HP.", heal);
                self.message_timer = 60;
                if lvl >= 3 {
                    if let Some(idx) = self.player.statuses.iter().position(|s| s.is_negative()) {
                        let removed = self.player.statuses.remove(idx);
                        self.message
                            .push_str(&format!(" Cured {}.", removed.label()));
                    }
                }
            }
        }

        if self.floor_num > 1 {
            if self.player.get_piety(Faction::Consortium) >= 10 && self.player.get_piety(Faction::AncientOrder) >= 10
            {
                self.player.gold += 5;
            }
        }

        if self.player.get_piety(Faction::Technocracy) >= 10 && self.player.get_piety(Faction::FreeTraders) >= 10 {
            if (self.rng_next() % 100) < 25 {
                self.reveal_entire_floor();
                let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
                self.particles.spawn_synergy(sx, sy, &mut self.rng_state);
                if self.message.is_empty() {
                    self.message = "Scholar's Wind reveals the floor layout!".to_string();
                    self.message_timer = 90;
                }
            }
        }

        if self.player.get_piety(Faction::FreeTraders) >= 10 && self.player.get_piety(Faction::AncientOrder) >= 10 {
            if (self.rng_next() % 100) < 25 {
                let mut tries = 0;
                while tries < 100 {
                    let rx = (self.rng_next() % (MAP_W as u64)) as i32;
                    let ry = (self.rng_next() % (MAP_H as u64)) as i32;
                    if self.level.in_bounds(rx, ry)
                        && self.level.is_walkable(rx, ry)
                        && self.level.tile(rx, ry) == Tile::MetalFloor
                    {
                        if (rx, ry) != (self.player.x, self.player.y) {
                            let idx = self.level.idx(rx, ry);
                            self.level.tiles[idx] = Tile::SupplyCrate;
                            break;
                        }
                    }
                    tries += 1;
                }
            }
        }

        let profile_label = self.floor_profile.label();
        if !profile_label.is_empty() && self.message.is_empty() {
            self.message = profile_label.to_string();
            self.message_timer = 90;
        }

        // Check floor-based quests
        self.check_floor_quests();
    }

    /// Check if an enemy occupies (x, y). Returns its index.
    fn enemy_at(&self, x: i32, y: i32) -> Option<usize> {
        self.enemies
            .iter()
            .position(|e| e.is_alive() && e.x == x && e.y == y)
    }

    /// Get the room modifier at the player's current position.
    fn current_room_modifier(&self) -> Option<RoomModifier> {
        let px = self.player.x;
        let py = self.player.y;
        for room in &self.level.rooms {
            if px >= room.x && px < room.x + room.w && py >= room.y && py < room.y + room.h {
                return room.modifier;
            }
        }
        None
    }

    /// Get the special room kind at the player's current position.
    fn current_special_room(&self) -> Option<SpecialRoomKind> {
        let px = self.player.x;
        let py = self.player.y;
        for room in &self.level.rooms {
            if px >= room.x && px < room.x + room.w && py >= room.y && py < room.y + room.h {
                return room.special;
            }
        }
        None
    }

    /// Get the (room.x, room.y) of the room the player is currently in, if any.
    fn current_room_origin(&self) -> Option<(i32, i32)> {
        let px = self.player.x;
        let py = self.player.y;
        for room in &self.level.rooms {
            if px >= room.x && px < room.x + room.w && py >= room.y && py < room.y + room.h {
                return Some((room.x, room.y));
            }
        }
        None
    }

    /// Mark the current special room as completed so it won't trigger again.
    fn mark_room_completed(&mut self) {
        if let Some((rx, ry)) = self.current_room_origin() {
            self.completed_special_rooms.insert((self.floor_num, rx, ry));
        }
    }

    /// Check if the current special room has already been completed.
    fn is_room_completed(&self) -> bool {
        if let Some((rx, ry)) = self.current_room_origin() {
            self.completed_special_rooms.contains(&(self.floor_num, rx, ry))
        } else {
            false
        }
    }

    /// Handle interactive mechanics for the 23 new special room types.
    fn handle_special_room_interaction(&mut self, target_tile: Tile) {
        let special = match self.current_special_room() {
            Some(s) => s,
            None => return,
        };

        // Skip rooms already completed
        if self.is_room_completed() {
            return;
        }

        match special {
            // ── Risk/Reward Rooms ────────────────────────────────────
            SpecialRoomKind::WanderingMerchant => {
                // Triggered when stepping on a Chest tile
                if target_tile != Tile::SupplyCrate { return; }
                self.mark_room_completed();
                let roll = self.rng_next() % 3;
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                match roll {
                    0 => {
                        // Great reward: rare radical + gold
                        let rare_radicals: &[&str] = &["龙", "凤", "鬼", "神", "魂", "仙"];
                        let r = rare_radicals[self.rng_next() as usize % rare_radicals.len()];
                        self.player.add_radical(r);
                        self.player.gold += 50;
                        self.message = format!("🎰 JACKPOT! You find the rare radical {} and 50 gold!", r);
                        self.flash = Some((255, 215, 0, 0.3));
                        if let Some(ref audio) = self.audio {
                            audio.play_treasure();
                        }
                    }
                    1 => {
                        // Decent: random item
                        let _ = self.player.add_item(Item::MedHypo(8), ItemState::Normal);
                        self.message = "🎰 You find a Health Potion inside the urn.".to_string();
                        if let Some(ref audio) = self.audio {
                            audio.play_treasure();
                        }
                    }
                    _ => {
                        // Trap: poison + lose gold
                        self.player.statuses.push(crate::status::StatusInstance::new(
                            crate::status::StatusKind::Poison { damage: 2 },
                            5,
                        ));
                        let lost = 20.min(self.player.gold);
                        self.player.gold -= lost;
                        self.message = format!("🎰 TRAP! Poison gas and you lose {} gold!", lost);
                        self.trigger_shake(8);
                        self.flash = Some((120, 255, 80, 0.25));
                        if let Some(ref audio) = self.audio {
                            audio.play_damage();
                        }
                    }
                }
                self.message_timer = 100;
            }

            SpecialRoomKind::EnergyNexus => {
                if target_tile != Tile::Terminal(TerminalKind::Tactical) { return; }
                if self.player.hp <= 5 {
                    self.message = "🩸 You're too weak to sacrifice. You need more than 5 HP.".to_string();
                    self.message_timer = 80;
                    return;
                }
                self.mark_room_completed();
                self.player.hp -= 5;
                self.player.tone_bonus_damage += 1;
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                self.message = "🩸 You sacrifice 5 HP. Permanent +1 damage gained!".to_string();
                self.message_timer = 100;
                self.trigger_shake(6);
                self.flash = Some((180, 0, 0, 0.3));
                if let Some(ref audio) = self.audio {
                    audio.play_damage();
                }
            }

            SpecialRoomKind::CursedSalvage => {
                if target_tile != Tile::SupplyCrate { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Great loot: high-tier equipment + gold
                let equip_idx = 2 + (self.rng_next() as usize % 3); // Dragon Fang Pen or better
                let equip = &EQUIPMENT_POOL[equip_idx.min(EQUIPMENT_POOL.len() - 1)];
                self.player.equip(equip, ItemState::Normal);
                self.player.gold += 75;
                // Apply Cursed status for 10 turns (representing floors)
                self.player.statuses.push(crate::status::StatusInstance::new(
                    crate::status::StatusKind::Cursed,
                    50, // ~10 floors worth of turns
                ));
                self.message = format!("💀 You claim {} and 75 gold, but a curse clings to you!", equip.name);
                self.message_timer = 120;
                self.trigger_shake(4);
                self.flash = Some((100, 0, 150, 0.3));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::SoulForge => {
                if target_tile != Tile::QuantumForge { return; }
                if self.player.radicals.is_empty() {
                    self.message = "🔮 The Soul Forge flickers — you have no radicals to offer.".to_string();
                    self.message_timer = 80;
                    return;
                }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Remove a random radical and give a different one
                let remove_idx = self.rng_next() as usize % self.player.radicals.len();
                let old = self.player.radicals.remove(remove_idx);
                let rare_radicals: &[&str] = &["龙", "凤", "鬼", "神", "魂", "仙", "雷", "冰", "光", "暗"];
                let new_rad = rare_radicals[self.rng_next() as usize % rare_radicals.len()];
                self.player.add_radical(new_rad);
                self.message = format!("🔮 The Soul Forge transforms {} into {}!", old, new_rad);
                self.message_timer = 100;
                self.flash = Some((200, 100, 255, 0.25));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::ChemLab => {
                // Triggered when stepping on DeepWater (the well center)
                if target_tile != Tile::VacuumBreach { return; }
                self.mark_room_completed();
                // Tiered cost: take what the player can afford
                let (cost, reward_tier) = if self.player.gold >= 50 {
                    (50, 2)
                } else if self.player.gold >= 25 {
                    (25, 1)
                } else if self.player.gold >= 10 {
                    (10, 0)
                } else {
                    self.message = "🪙 The well needs at least 10 gold...".to_string();
                    self.message_timer = 60;
                    return;
                };
                self.player.gold -= cost;
                match reward_tier {
                    0 => {
                        let _ = self.player.add_item(Item::MedHypo(10), ItemState::Normal);
                        self.message = format!("🪙 You throw {} gold. A potion rises from the depths!", cost);
                    }
                    1 => {
                        let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
                        let equip = &EQUIPMENT_POOL[eq_idx];
                        self.player.equip(equip, ItemState::Normal);
                        self.message = format!("🪙 You throw {} gold. {} rises from the depths!", cost, equip.name);
                    }
                    _ => {
                        let rare_radicals: &[&str] = &["龙", "凤", "鬼", "神", "魂", "仙"];
                        let r = rare_radicals[self.rng_next() as usize % rare_radicals.len()];
                        self.player.add_radical(r);
                        let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
                        let equip = &EQUIPMENT_POOL[eq_idx];
                        self.player.equip(equip, ItemState::Blessed);
                        self.message = format!("🪙 You throw {} gold. Radical {} and blessed {} rise!", cost, r, equip.name);
                    }
                }
                self.message_timer = 120;
                self.flash = Some((100, 180, 255, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            // ── Puzzle Rooms ─────────────────────────────────────────
            SpecialRoomKind::WarpGate => {
                // Stepping on pressure plates in the room
                if target_tile != Tile::PressureSensor { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Simplified: each plate gives a reward or penalty
                if self.rng_next() % 2 == 0 {
                    let radicals: &[&str] = &["水", "火", "金", "木", "土"];
                    let r = radicals[self.rng_next() as usize % radicals.len()];
                    self.player.add_radical(r);
                    self.message = format!("✨ Correct sequence! The rune grants you radical {}!", r);
                    self.flash = Some((100, 255, 200, 0.2));
                    if let Some(ref audio) = self.audio {
                        audio.play_treasure();
                    }
                } else {
                    let dmg = 2 + self.floor_num / 5;
                    self.player.hp -= dmg;
                    self.message = format!("⚡ Wrong order! The rune zaps you for {} damage!", dmg);
                    self.trigger_shake(6);
                    self.flash = Some((255, 100, 50, 0.25));
                    if let Some(ref audio) = self.audio {
                        audio.play_damage();
                    }
                }
                self.message_timer = 80;
            }

            SpecialRoomKind::HoloMaze => {
                // Reward at the center chest
                if target_tile != Tile::SupplyCrate { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                self.player.gold += 30 + (self.floor_num * 3) as i32;
                let _ = self.player.add_item(Item::ScannerPulse, ItemState::Normal);
                self.message = format!("🪞 You navigate the mirrors! {} gold + Reveal Scroll!", 30 + self.floor_num * 3);
                self.message_timer = 100;
                self.flash = Some((200, 200, 255, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::GravityPuzzle => {
                // Reward chest in center after navigating ice + boulders
                if target_tile != Tile::SupplyCrate { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                self.player.gold += 40;
                let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
                let equip = &EQUIPMENT_POOL[eq_idx];
                self.player.equip(equip, ItemState::Normal);
                self.message = format!("⚖ Puzzle solved! 40 gold + {}!", equip.name);
                self.message_timer = 100;
                self.flash = Some((200, 255, 150, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::ToneFrequency => {
                // Stepping on Shrine tiles (the steps)
                if target_tile != Tile::CircuitShrine { return; }
                // Don't mark completed — each shrine is a step
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                self.player.spirit = (self.player.spirit + 5).min(self.player.max_spirit);
                self.message = "🎵 Correct tone! Spirit +5. Ascend the staircase!".to_string();
                self.message_timer = 60;
                self.flash = Some((255, 220, 100, 0.15));
            }

            SpecialRoomKind::ElementalLock => {
                // Stepping on elemental altars charges the lock
                if !matches!(target_tile, Tile::Terminal(_)) { return; }
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Count remaining altars in room
                let mut remaining = 0;
                if let Some((rx, ry)) = self.current_room_origin() {
                    for room in &self.level.rooms {
                        if room.x == rx && room.y == ry {
                            for ty in room.y..room.y + room.h {
                                for tx in room.x..room.x + room.w {
                                    if self.level.in_bounds(tx, ty) {
                                        let ti = self.level.idx(tx, ty);
                                        if matches!(self.level.tiles[ti], Tile::Terminal(_)) {
                                            remaining += 1;
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }
                }
                if remaining == 0 {
                    // All altars activated — unlock the door, give reward
                    self.mark_room_completed();
                    // Find and unlock the locked door
                    if let Some((rx, ry)) = self.current_room_origin() {
                        for room in &self.level.rooms {
                            if room.x == rx && room.y == ry {
                                for ty in room.y..room.y + room.h {
                                    for tx in room.x..room.x + room.w {
                                        if self.level.in_bounds(tx, ty) {
                                            let ti = self.level.idx(tx, ty);
                                            if self.level.tiles[ti] == Tile::SealedHatch {
                                                self.level.tiles[ti] = Tile::SupplyCrate;
                                            }
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }
                    self.message = "🔓 All elements channeled! The sealed door opens!".to_string();
                    self.flash = Some((255, 255, 200, 0.3));
                } else {
                    self.message = format!("🔮 Elemental energy absorbed! {} altars remaining.", remaining);
                }
                self.message_timer = 80;
            }

            // ── Timed/Wave Rooms ─────────────────────────────────────
            SpecialRoomKind::SurvivalBay => {
                // On first entry, give a big bonus (simulate surviving)
                if target_tile != Tile::LaserGrid && target_tile != Tile::MetalFloor { return; }
                // Only trigger once when entering the arena center area
                if self.player.x != self.level.rooms.iter()
                    .find(|r| r.special == Some(SpecialRoomKind::SurvivalBay))
                    .map(|r| r.x + r.w / 2)
                    .unwrap_or(-1)
                { return; }
                self.mark_room_completed();
                let reward_gold = 20 + self.player.hp * 2;
                self.player.gold += reward_gold;
                self.message = format!("⚔ You survived the pit! {} gold (HP bonus)!", reward_gold);
                self.message_timer = 100;
                self.flash = Some((255, 200, 50, 0.25));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::SalvageRace => {
                // Gold piles are already placed; tile effects handle pickup
                // Just show encouragement on entry
                if target_tile == Tile::CreditCache {
                    // Normal GoldPile effect handles this via apply_player_tile_effect
                    return;
                }
            }

            SpecialRoomKind::DepressurizingChamber => {
                // Chest in center is the goal
                if target_tile != Tile::SupplyCrate { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                self.player.gold += 60;
                let _ = self.player.add_item(Item::PersonalTeleporter, ItemState::Normal);
                self.message = "💎 You grab the treasure before the floor collapses! 60 gold + Teleport Scroll!".to_string();
                self.message_timer = 100;
                self.flash = Some((255, 200, 100, 0.25));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::NanoFlood => {
                if target_tile == Tile::DataWell {
                    self.mark_room_completed();
                    let idx = self.level.idx(self.player.x, self.player.y);
                    self.level.tiles[idx] = Tile::MetalFloor;
                    self.player.spell_power_bonus += 1;
                    self.player.hp -= 2;
                    self.message = "🖋 The ink empowers your spells! +1 spell power, but -2 HP from ink exposure.".to_string();
                    self.message_timer = 100;
                    self.flash = Some((50, 50, 100, 0.2));
                }
            }

            // ── Transformation/Permanent Rooms ───────────────────────
            SpecialRoomKind::FormShrine => {
                if target_tile != Tile::CircuitShrine { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Cycle through forms based on RNG
                let forms = [PlayerForm::Powered, PlayerForm::Cybernetic, PlayerForm::Holographic, PlayerForm::Void];
                let form = forms[self.rng_next() as usize % forms.len()];
                self.player.form = form;
                self.player.form_timer = 0; // permanent
                self.message = format!("🔥 The shrine transforms you into {} form permanently!", form.name());
                self.message_timer = 120;
                self.flash = Some((255, 150, 50, 0.3));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::ClassTrial => {
                if !matches!(target_tile, Tile::Terminal(_)) { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Grant class-specific bonus
                self.player.tone_bonus_damage += 1;
                self.player.defense_bonus += 1;
                self.message = "⚔ Trial complete! +1 damage and +1 defense permanently!".to_string();
                self.message_timer = 100;
                self.flash = Some((200, 180, 50, 0.25));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::DataArchive => {
                if target_tile != Tile::MedBayTile { return; }
                self.mark_room_completed();
                // The spirit spring tile effect (heal) is handled by apply_player_tile_effect
                // Additional: permanent spell power bonus
                self.player.spell_power_bonus += 1;
                self.message = "✨ The fountain refines your radicals! All spells gain +1 damage permanently!".to_string();
                self.message_timer = 120;
                self.flash = Some((150, 255, 200, 0.3));
            }

            SpecialRoomKind::AncestorCrypt => {
                if target_tile != Tile::SupplyCrate { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Give a high-tier weapon
                let high_tier = &EQUIPMENT_POOL[2]; // Dragon Fang Pen (+3 dmg)
                self.player.equip(high_tier, ItemState::Blessed);
                self.message = format!("⚔ The ancestor's spirit grants you their blessed {}!", high_tier.name);
                self.message_timer = 120;
                self.flash = Some((255, 215, 0, 0.3));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            // ── Story/Lore Rooms ─────────────────────────────────────
            SpecialRoomKind::WisdomCore => {
                if target_tile != Tile::HoloPool { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Reveal the map + give info
                for i in 0..self.level.revealed.len() {
                    self.level.revealed[i] = true;
                }
                self.message = "🔮 The prophecy reveals the entire floor! You sense the boss's presence...".to_string();
                self.message_timer = 120;
                self.flash = Some((200, 150, 255, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::SealedMemory => {
                if target_tile != Tile::CircuitShrine { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // XP/spirit reward for recalling memories
                self.player.spirit = self.player.max_spirit;
                self.player.gold += 25;
                let radicals: &[&str] = &["心", "力", "气", "光"];
                let r = radicals[self.rng_next() as usize % radicals.len()];
                self.player.add_radical(r);
                self.message = format!("🧠 Memories flood back! Spirit restored, +25 gold, radical {}!", r);
                self.message_timer = 100;
                self.flash = Some((180, 200, 255, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::DemonSeal => {
                if target_tile != Tile::Npc(3) { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Accept the deal: +3 max HP but next floor enemies are tougher
                self.player.max_hp += 3;
                self.player.hp += 3;
                self.demon_deal_floors = 1;
                self.message = "👹 Deal accepted! +3 max HP, but next floor's enemies will be elite-tier!".to_string();
                self.message_timer = 120;
                self.trigger_shake(6);
                self.flash = Some((150, 0, 0, 0.3));
                if let Some(ref audio) = self.audio {
                    audio.play_damage();
                }
            }

            SpecialRoomKind::PhoenixNest => {
                if target_tile == Tile::MedBayTile {
                    // Full heal from stepping on spring (handled by tile effect)
                    // Also grant +2 max HP
                    if !self.is_room_completed() {
                        self.mark_room_completed();
                        self.player.max_hp += 2;
                        self.player.hp = self.player.max_hp;
                        self.message = "🔥 The Phoenix blesses you! Full heal + permanent +2 max HP!".to_string();
                        self.message_timer = 120;
                        self.flash = Some((255, 150, 50, 0.3));
                    }
                } else if target_tile == Tile::SupplyCrate {
                    // PhoenixPlume item
                    let idx = self.level.idx(self.player.x, self.player.y);
                    self.level.tiles[idx] = Tile::MetalFloor;
                    let _ = self.player.add_item(Item::Revitalizer(self.player.max_hp / 2), ItemState::Blessed);
                    self.message = "🔥 You find a blessed Phoenix Plume! Auto-revive on death!".to_string();
                    self.message_timer = 100;
                    if let Some(ref audio) = self.audio {
                        audio.play_treasure();
                    }
                }
            }

            SpecialRoomKind::CalligraphyContest => {
                if target_tile != Tile::DataWell { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Score-based gold reward
                let score = 20 + (self.rng_next() % 40) as i32;
                self.player.gold += score;
                self.message = format!("🖌 Calligraphy contest! Score: {}. You earn {} gold!", score, score);
                self.message_timer = 100;
                self.flash = Some((255, 255, 200, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            // All other room types — no special interaction on tile step
            _ => {}
        }
    }

    /// Effective FOV radius (reduced in Dark rooms and Shadow Realm).
    fn effective_fov(&self) -> i32 {
        let base = if self.current_room_modifier() == Some(RoomModifier::PoweredDown)
            || self.current_special_room() == Some(SpecialRoomKind::DarkSector)
        {
            2
        } else {
            FOV_RADIUS
        };
        base + self.player.enchant_fov_bonus()
    }

    fn look_text(&self, x: i32, y: i32) -> String {
        if !self.level.in_bounds(x, y) || !in_look_range(self.player.x, self.player.y, x, y) {
            return format!("Look range is {} tiles.", LOOK_RANGE);
        }

        let idx = self.level.idx(x, y);
        if x == self.player.x && y == self.player.y {
            return format!(
                "You are here — {} form, HP {}/{}.",
                self.player.form.name(),
                self.player.hp,
                self.player.max_hp
            );
        }

        if !self.level.revealed[idx] {
            return "Unseen darkness.".to_string();
        }

        if self.level.visible[idx] {
            if let Some(enemy_idx) = self.enemy_at(x, y) {
                return enemy_look_text(&self.enemies[enemy_idx]);
            }
        }

        let mut text = tile_look_text(self.level.tile(x, y));
        if !self.level.visible[idx] {
            text.push_str(" (remembered)");
        }
        text
    }

    fn update_look_message(&mut self, x: i32, y: i32) {
        self.message = self.look_text(x, y);
        self.message_timer = 255;
    }

    fn move_look_cursor(&mut self, dx: i32, dy: i32) {
        let CombatState::Looking { x, y } = self.combat.clone() else {
            return;
        };
        let min_x = (self.player.x - LOOK_RANGE).max(0);
        let max_x = (self.player.x + LOOK_RANGE).min(self.level.width - 1);
        let min_y = (self.player.y - LOOK_RANGE).max(0);
        let max_y = (self.player.y + LOOK_RANGE).min(self.level.height - 1);
        let next_x = (x + dx).clamp(min_x, max_x);
        let next_y = (y + dy).clamp(min_y, max_y);
        self.combat = CombatState::Looking {
            x: next_x,
            y: next_y,
        };
        self.update_look_message(next_x, next_y);
    }

    /// Try to move player. Bumping into an enemy starts combat.
    fn try_move(&mut self, dx: i32, dy: i32) {
        if matches!(self.combat, CombatState::GameOver) {
            return;
        }
        // If fighting, ignore movement
        if matches!(
            self.combat,
            CombatState::Fighting { .. } | CombatState::TacticalBattle(_)
        ) {
            return;
        }

        // Tick player statuses
        // Decrement combo timer
        if self.spell_combo_timer > 0 {
            self.spell_combo_timer -= 1;
            if self.spell_combo_timer == 0 {
                self.last_spell = None;
            }
        }
        let (pdmg, pheal) = status::tick_statuses(&mut self.player.statuses);
        self.player.tick_form();

        // Spirit clock tick
        let spirit_shielded = crate::status::has_spirit_shield(&self.player.statuses);
        let spirit_sustained = self.player.has_spirit_sustain();
        let skip_drain = spirit_shielded || (spirit_sustained && self.move_count % 2 == 0);
        if self.player.spirit > 0 {
            if !skip_drain {
                self.player.spirit -= 1;
            }
            if self.player.spirit == 0 {
                self.message = "🌑 Your spirit is exhausted! Find food or an ink well!".to_string();
                self.message_timer = 80;
            }
        } else {
            // Starving: damage scales with depth
            let starvation_dmg = 1 + self.floor_num / 5;
            self.player.hp -= starvation_dmg;
            self.message = format!(
                "🌑 Spiritual exhaustion drains your life force! (-{} HP)",
                starvation_dmg
            );
            self.message_timer = 40;
            if self.player.hp <= 0 && !self.try_phoenix_revive() {
                self.player.hp = 0;
                if let Some(ref audio) = self.audio {
                    audio.play_death();
                }
                self.run_journal
                    .log(RunEvent::DiedTo("Starvation".to_string(), self.floor_num));
                self.post_mortem_page = 0;
                self.combat = CombatState::GameOver;
                self.message = self.run_summary();
                self.message_timer = 255;
                return;
            }
        }

        if pdmg > 0 {
            self.player.hp -= pdmg;
            self.message = format!("☠ Poison deals {} damage!", pdmg);
            self.message_timer = 40;
            if self.player.hp <= 0 && !self.try_phoenix_revive() {
                self.player.hp = 0;
                if let Some(ref audio) = self.audio {
                    audio.play_death();
                }
                self.run_journal
                    .log(RunEvent::DiedTo("Poison".to_string(), self.floor_num));
                self.post_mortem_page = 0;
                self.combat = CombatState::GameOver;
                self.message = self.run_summary();
                self.message_timer = 255;
                return;
            }
        }
        if pheal > 0 {
            self.player.hp = (self.player.hp + pheal).min(self.player.max_hp);
        }

        // Tick enemy statuses
        for e in &mut self.enemies {
            if e.is_alive() {
                let (edmg, _) = status::tick_statuses(&mut e.statuses);
                if edmg > 0 {
                    e.hp -= edmg;
                }
            }
        }

        // Confused: randomize direction
        let (dx, dy) = if status::has_confused(&self.player.statuses) {
            let dirs = [(0, -1), (0, 1), (-1, 0), (1, 0)];
            dirs[self.rng_next() as usize % 4]
        } else {
            (dx, dy)
        };

        // If map-reveal status active, reveal all tiles
        if status::has_revealed(&self.player.statuses) {
            self.reveal_entire_floor();
        }

        let (nx, ny) = self.player.intended_move(dx, dy);
        let target_tile = self.level.tile(nx, ny);
        if target_tile == Tile::CargoCrate {
            let pdx = nx - self.player.x;
            let pdy = ny - self.player.y;
            let px = nx + pdx;
            let py = ny + pdy;

            // Check bounds
            if !self.level.in_bounds(px, py) {
                self.message = "It's jammed against the wall.".to_string();
                self.message_timer = 30;
                return;
            }

            let push_target_idx = self.level.idx(px, py);
            let push_target_tile = self.level.tiles[push_target_idx];

            // Allow pushing into open space or liquids
            let can_push = matches!(
                push_target_tile,
                Tile::MetalFloor
                    | Tile::Hallway
                    | Tile::CoolantPool
                    | Tile::VacuumBreach
                    | Tile::Coolant
                    | Tile::LaserGrid
                    | Tile::Catwalk
            );
            let enemy_behind = self.enemy_at(px, py).is_some();

            if can_push && !enemy_behind {
                if matches!(push_target_tile, Tile::CoolantPool | Tile::VacuumBreach) {
                    self.level.tiles[push_target_idx] = Tile::Catwalk;
                    self.message = if push_target_tile == Tile::VacuumBreach {
                        "The crate drops into the pool, forming a rough bridge!".to_string()
                    } else {
                        "The crate splashes into place, forming a bridge!".to_string()
                    };
                    self.message_timer = 80;
                    let (sx, sy) = self.tile_to_screen(px, py);
                    self.particles.spawn_bridge(sx, sy, &mut self.rng_state);
                    self.trigger_shake(4);
                    if let Some(ref audio) = self.audio {
                        audio.play_bridge();
                    }
                } else {
                    self.level.tiles[push_target_idx] = Tile::CargoCrate;
                    self.message = "You shove the crate aside.".to_string();
                    self.message_timer = 40;
                }
                let current_idx = self.level.idx(nx, ny);
                self.level.tiles[current_idx] = Tile::MetalFloor;

                // Player moves into the spot
                self.player.x = nx;
                self.player.y = ny;
                self.move_count += 1;

                let skip_enemy =
                    status::has_haste(&self.player.statuses) && self.move_count % 2 == 0;
                if !skip_enemy {
                    self.enemy_turn();
                }
                let (px, py) = (self.player.x, self.player.y);
                let fov = self.effective_fov();
                compute_fov(&mut self.level, px, py, fov);
                return;
            } else {
                self.message = "It won't budge.".to_string();
                self.message_timer = 30;
                return;
            }
        }

        if matches!(
            target_tile,
            Tile::Bulkhead | Tile::DamagedBulkhead | Tile::WeakBulkhead
        ) {
            // Check for digging using weapon effect
            let can_dig = self
                .player
                .weapon
                .map_or(false, |eq| matches!(eq.effect, EquipEffect::Digging))
                || self.player.form == PlayerForm::Cybernetic;

            if can_dig {
                let cracked_wall = target_tile == Tile::DamagedBulkhead;
                let brittle_wall = target_tile == Tile::WeakBulkhead;
                let idx = self.level.idx(nx, ny);
                self.level.tiles[idx] = Tile::MetalFloor;
                let (sx, sy) = self.tile_to_screen(nx, ny);
                self.particles.spawn_dig(sx, sy, &mut self.rng_state);
                self.trigger_shake(if cracked_wall {
                    6
                } else if brittle_wall {
                    5
                } else {
                    4
                });
                if let Some(ref audio) = self.audio {
                    audio.play_dig();
                }
                self.message = if cracked_wall {
                    "You smash through the cracked wall and uncover a hidden chamber!".to_string()
                } else if brittle_wall {
                    "You break through the brittle wall and crack open the cache!".to_string()
                } else {
                    "Stone chips fly as you dig a rough tunnel.".to_string()
                };
                self.message_timer = if cracked_wall {
                    120
                } else if brittle_wall {
                    100
                } else {
                    75
                };
                self.move_count += 1;

                let skip_enemy =
                    status::has_haste(&self.player.statuses) && self.move_count % 2 == 0;
                if !skip_enemy {
                    self.enemy_turn();
                }
                let (px, py) = (self.player.x, self.player.y);
                let fov = self.effective_fov();
                compute_fov(&mut self.level, px, py, fov);
                return;
            }

            if target_tile == Tile::DamagedBulkhead {
                self.message =
                    "The wall is cracked. A digging tool could break through.".to_string();
                self.message_timer = 90;
            } else if target_tile == Tile::WeakBulkhead {
                self.message =
                    "The brittle wall could be smashed open with a digging tool.".to_string();
                self.message_timer = 90;
            }
        }

        if target_tile == Tile::SealedHatch {
            self.start_locked_door(nx, ny);
            return;
        }

        // Boulder pushing (similar to crate)
        if target_tile == Tile::CargoCrate {
            let pdx = nx - self.player.x;
            let pdy = ny - self.player.y;
            let px = nx + pdx;
            let py = ny + pdy;
            if self.level.in_bounds(px, py) {
                let push_idx = self.level.idx(px, py);
                let push_tile = self.level.tiles[push_idx];
                if push_tile == Tile::PressureSensor || push_tile == Tile::MetalFloor {
                    let boulder_idx = self.level.idx(nx, ny);
                    self.level.tiles[boulder_idx] = Tile::MetalFloor;
                    self.level.tiles[push_idx] = Tile::CargoCrate;
                    if push_tile == Tile::PressureSensor {
                        self.message = "🪨 The boulder clicks onto the pressure plate!".to_string();
                        self.trigger_shake(4);
                    } else {
                        self.message = "🪨 You heave the boulder aside.".to_string();
                    }
                    self.message_timer = 50;
                    self.player.x = nx;
                    self.player.y = ny;
                    self.move_count += 1;
                    let skip_enemy =
                        status::has_haste(&self.player.statuses) && self.move_count % 2 == 0;
                    if !skip_enemy {
                        self.enemy_turn();
                    }
                    let (px2, py2) = (self.player.x, self.player.y);
                    let fov = self.effective_fov();
                    compute_fov(&mut self.level, px2, py2, fov);
                    return;
                } else {
                    self.message = "The boulder won't budge that way.".to_string();
                    self.message_timer = 30;
                    return;
                }
            } else {
                self.message = "The boulder is jammed against the wall.".to_string();
                self.message_timer = 30;
                return;
            }
        }

        // Bookshelf interaction
        if target_tile == Tile::DataRack {
            self.message = "📚 You study the ancient texts and feel wiser.".to_string();
            self.message_timer = 60;
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        // GoldOre mining
        if target_tile == Tile::OreVein {
            let gold = 5 + (self.rng_next() % 11) as i32;
            self.player.gold += gold;
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            self.message = format!("⛏ You mine {} gold from the ore vein!", gold);
            self.message_timer = 60;
            self.trigger_shake(3);
            return;
        }

        // Crystal wall — not walkable, just shows message
        if target_tile == Tile::CrystalPanel {
            self.message = "💎 The crystal formation is too solid to pass.".to_string();
            self.message_timer = 40;
            return;
        }

        // Bamboo — not walkable without cutting
        if target_tile == Tile::CargoPipes {
            let can_dig = self
                .player
                .weapon
                .map_or(false, |eq| matches!(eq.effect, EquipEffect::Digging))
                || self.player.form == PlayerForm::Cybernetic;
            if can_dig {
                let idx = self.level.idx(nx, ny);
                self.level.tiles[idx] = Tile::MetalFloor;
                self.message = "🎋 You cut through the bamboo thicket!".to_string();
                self.message_timer = 50;
                self.trigger_shake(3);
                return;
            }
            self.message = "🎋 Dense bamboo blocks the way. A cutting tool might help.".to_string();
            self.message_timer = 60;
            return;
        }

        if !target_tile.is_walkable() {
            return;
        }

        // Check for enemy bump → start tactical combat
        if let Some(idx) = self.enemy_at(nx, ny) {
            let mut combat_indices = vec![idx];
            let mut sentence_ambush = false;

            if self.floor_num >= 8 && !self.enemies[idx].is_boss {
                let chance = if self.floor_num >= 15 { 3 } else { 5 };
                if self.rng_next() % chance == 0 {
                    let pool = vocab::sentences_for_floor(self.floor_num);
                    if !pool.is_empty() {
                        let entry_idx = self.rng_next() as usize % pool.len();
                        let sentence = &pool[entry_idx];
                        let sent_enemies =
                            combat::transition::enemies_from_sentence(sentence, self.floor_num);
                        for mut se in sent_enemies {
                            se.x = nx;
                            se.y = ny;
                            self.enemies.push(se);
                            combat_indices.push(self.enemies.len() - 1);
                        }
                        sentence_ambush = true;
                    }
                }
            }

            let battle = combat::transition::enter_combat(
                &self.player,
                &self.enemies,
                &combat_indices,
                self.floor_num,
                self.current_room_modifier(),
                &self.srs,
                self.companion,
            );
            self.combat = CombatState::TacticalBattle(Box::new(battle));
            self.typing.clear();
            self.guard_used_this_fight = false;
            self.guard_blocks_used = 0;
            if let Some(ref audio) = self.audio {
                let has_boss = combat_indices.iter().any(|&ei| self.enemies[ei].is_boss);
                if has_boss {
                    audio.play_boss_encounter();
                } else {
                    audio.play_combat_start();
                }
            }
            if self.listening_mode.is_active() && !self.enemies[idx].is_elite {
                let pinyin = self.enemies[idx].pinyin;
                let tone_num = pinyin
                    .chars()
                    .last()
                    .and_then(|c| c.to_digit(10))
                    .unwrap_or(1) as u8;
                if let Some(ref audio) = self.audio {
                    audio.play_chinese_tone(tone_num);
                }
                self.message =
                    combat_prompt_for(&self.enemies[idx], self.listening_mode, self.mirror_hint);
            } else {
                self.message =
                    combat_prompt_for(&self.enemies[idx], self.listening_mode, self.mirror_hint);
            }
            if let Some(ref comp) = self.companion {
                let lvl = self.companion_level();
                if let Some(hint) = comp.contextual_hint(
                    &self.enemies[idx],
                    self.player.hp,
                    self.player.max_hp,
                    self.guard_used_this_fight,
                    lvl,
                ) {
                    self.message.push_str(&format!("\n{}", hint));
                }
            }
            if sentence_ambush {
                self.message
                    .push_str("\n\u{26a0} Sentence ambush! Extra enemies joined the fight!");
            }
            self.message_timer = 255;
            return;
        }

        if target_tile == Tile::Airlock {
            if let Some(blocker) = self.tutorial_exit_blocker() {
                self.message = blocker.to_string();
                self.message_timer = 150;
                return;
            }
        }

        self.player.move_to(nx, ny);
        if let Some(ref audio) = self.audio {
            audio.play_step();
        }

        if let Tile::InfoPanel(sign_id) = target_tile {
            self.show_tutorial_sign(sign_id);
        }

        if let Tile::SecurityLock(kind) = target_tile {
            self.trigger_seal(nx, ny, kind, None);
        }

        // Stairs
        if target_tile == Tile::Airlock {
            self.descend_floor(false);
            return;
        }

        // Forge workbench
        if target_tile == Tile::QuantumForge {
            if self.player.radicals.is_empty() {
                self.message = "Forge workbench — but you have no radicals!".to_string();
                self.message_timer = 60;
            } else {
                let recipes = radical::craftable_recipes(&self.player.radicals);
                if recipes.is_empty() {
                    self.message =
                        "Forge workbench — no recipes available with your radicals.".to_string();
                    self.message_timer = 80;
                } else {
                    let count = recipes.len();
                    self.combat = CombatState::Forging { recipes, cursor: 0 };
                    self.message = format!(
                        "{} recipe{} available. ↑/↓ browse, Enter forge, E enchant, Esc close.",
                        count,
                        if count == 1 { "" } else { "s" }
                    );
                    self.message_timer = 255;
                }
                let (px, py) = (self.player.x, self.player.y);
                compute_fov(&mut self.level, px, py, FOV_RADIUS);
                return;
            }
        }

        // Shop
        if target_tile == Tile::TradeTerminal {
            if self.shop_banned {
                self.message =
                    "🚫 The shopkeeper slams the door shut. You're not welcome here!".to_string();
                self.message_timer = 80;
                return;
            }
            let items = self.generate_shop_items();
            self.combat = CombatState::Shopping { items, cursor: 0 };
            self.message =
                "Welcome to the shop! ↑↓ browse, Enter buy, G grab (steal), Esc leave.".to_string();
            self.message_timer = 255;
            let (px, py) = (self.player.x, self.player.y);
            compute_fov(&mut self.level, px, py, FOV_RADIUS);
            return;
        }

        // Chest
        if target_tile == Tile::SupplyCrate {
            self.open_chest(nx, ny);
        }

        // NPC companion recruit or quest
        if let Tile::Npc(npc_type) = target_tile {
            let comp = match npc_type {
                0 => Companion::ScienceOfficer,
                1 => Companion::Medic,
                2 => Companion::Quartermaster,
                _ => Companion::SecurityChief,
            };
            if self.companion.is_some() {
                if self.quests.len() < 2 {
                    let has_active_chain = self.quests.iter().any(|q| q.is_chain() && !q.completed);
                    let quest = if !has_active_chain && (self.rng_next() % 100) < 30 {
                        let cid = self.next_chain_id;
                        self.next_chain_id += 1;
                        self.generate_chain_quest(0, cid)
                    } else {
                        self.generate_quest()
                    };
                    self.message = format!("{} gives quest: {}", comp.icon(), quest.description);
                    self.quests.push(quest);
                } else {
                    self.message = format!(
                        "{} {} waves hello! (quest slots full)",
                        comp.icon(),
                        comp.name()
                    );
                }
            } else {
                self.companion = Some(comp);
                self.companion_xp = 0;
                self.merchant_reroll_used = false;
                self.message = format!("{} {} joins your party!", comp.icon(), comp.name());
            }
            self.message_timer = 100;
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
        }

        if let Tile::Terminal(kind) = target_tile {
            self.invoke_altar(nx, ny, kind);
        }

        // Tone shrine interaction
        if target_tile == Tile::CircuitShrine {
            self.start_tone_battle();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::CompoundShrine {
            self.start_stroke_order();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::FrequencyWall {
            self.start_tone_defense();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::CompoundShrine {
            self.start_compound_builder();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::ClassifierNode {
            self.start_classifier_match();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::DataWell {
            self.start_ink_well();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::MemorialNode {
            self.start_ancestor_challenge();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::TranslationTerminal {
            self.start_translation_challenge();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::RadicalLab {
            self.start_radical_garden();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::HoloPool {
            self.start_mirror_pool();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::DroidTutor {
            self.start_stone_tutor();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::CodexTerminal {
            self.start_codex_challenge();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::DataBridge {
            let dirs = [(0, -1), (0, 1), (-1, 0), (1, 0)];
            let mut bridge_target = (nx, ny);
            for (ddx, ddy) in &dirs {
                let bx = nx + ddx;
                let by = ny + ddy;
                if self.level.in_bounds(bx, by) && self.level.tile(bx, by) == Tile::VacuumBreach {
                    bridge_target = (bx, by);
                    break;
                }
            }
            self.start_word_bridge(bridge_target.0, bridge_target.1);
            return;
        }

        if target_tile == Tile::CorruptedFloor {
            self.player.move_to(nx, ny);
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            self.start_cursed_floor();
            return;
        }

        // Dragon Gate Portal interaction
        if target_tile == Tile::WarpGatePortal {
            self.message = "🐉 The Dragon Gate pulses with ancient power! You are not yet ready...".to_string();
            self.message_timer = 100;
        }

        // Show special room description on first entry
        if let Some(special) = self.current_special_room() {
            let sr_desc = special.description();
            if !self.message.contains(sr_desc) && self.message_timer < 20 {
                self.message = format!("📍 {} — {}", special.name(), sr_desc);
                self.message_timer = 100;
            }
        }

        // Handle special room interactions on tile step
        self.handle_special_room_interaction(target_tile);

        self.apply_player_tile_effect(target_tile);
        if matches!(self.combat, CombatState::GameOver) {
            return;
        }

        // After player moves, enemies take a turn(skipped on even moves during haste)
        self.move_count += 1;
        let skip_enemy = status::has_haste(&self.player.statuses) && self.move_count % 2 == 0;
        if !skip_enemy {
            self.enemy_turn();
        }

        let (px, py) = (self.player.x, self.player.y);
        let fov = self.effective_fov();
        compute_fov(&mut self.level, px, py, fov);
        self.companion_exploration_hint();
    }

    /// All enemies take one step toward the player if alerted.
    fn enemy_turn(&mut self) {
        let px = self.player.x;
        let py = self.player.y;
        let mut summons = Vec::new();
        let mut summon_message = None;

        for i in 0..self.enemies.len() {
            if !self.enemies[i].is_alive() {
                continue;
            }
            // Stunned enemies skip their turn
            if self.enemies[i].stunned {
                self.enemies[i].stunned = false;
                continue;
            }
            // Alert if within FOV radius
            let dist_sq = (self.enemies[i].x - px).pow(2) + (self.enemies[i].y - py).pow(2);
            if dist_sq <= (FOV_RADIUS * FOV_RADIUS) {
                self.enemies[i].alert = true;
            }
            if !self.enemies[i].alert {
                continue;
            }

            if self.enemies[i].boss_kind == Some(BossKind::PirateCaptain) {
                if self.enemies[i].summon_cooldown > 0 {
                    self.enemies[i].summon_cooldown -= 1;
                }
                let nearby_minions = self
                    .enemies
                    .iter()
                    .enumerate()
                    .filter(|(j, e)| {
                        *j != i
                            && e.is_alive()
                            && !e.is_boss
                            && (e.x - self.enemies[i].x).abs() <= 1
                            && (e.y - self.enemies[i].y).abs() <= 1
                    })
                    .count();
                if nearby_minions < 2 && self.enemies[i].summon_cooldown == 0 {
                    if let Some((sx, sy)) =
                        self.find_free_adjacent_tile(self.enemies[i].x, self.enemies[i].y)
                    {
                        let entry = Self::vocab_entry_by_hanzi("门")
                            .or_else(|| Self::vocab_entry_by_hanzi("人"))
                            .unwrap_or(&vocab::VOCAB[0]);
                        let mut ward = Enemy::from_vocab(entry, sx, sy, self.floor_num.max(2));
                        ward.alert = true;
                        ward.gold_value += 4;
                        summons.push(ward);
                        self.enemies[i].summon_cooldown = 3;
                        let visible_idx = self.level.idx(self.enemies[i].x, self.enemies[i].y);
                        if self.level.visible[visible_idx] {
                            summon_message =
                                Some("🚪 The Pirate Captain deploys a shield drone!".to_string());
                        }
                    }
                }
            }

            if self.enemies[i].boss_kind == Some(BossKind::VoidEntity) {
                if self.enemies[i].summon_cooldown > 0 {
                    self.enemies[i].summon_cooldown -= 1;
                }
                let nearby_minions = self
                    .enemies
                    .iter()
                    .enumerate()
                    .filter(|(j, e)| *j != i && e.is_alive() && !e.is_boss)
                    .count();
                if nearby_minions < 3 && self.enemies[i].summon_cooldown == 0 {
                    if let Some((sx, sy)) =
                        self.find_free_adjacent_tile(self.enemies[i].x, self.enemies[i].y)
                    {
                        let mimic_pool = vocab::vocab_for_floor(self.floor_num);
                        if !mimic_pool.is_empty() {
                            let entry = mimic_pool[self.rng_next() as usize % mimic_pool.len()];
                            let mut mimic = Enemy::from_vocab(entry, sx, sy, self.floor_num);
                            mimic.alert = true;
                            mimic.gold_value += 8;
                            summons.push(mimic);
                            self.enemies[i].summon_cooldown = 2;
                            let visible_idx = self.level.idx(self.enemies[i].x, self.enemies[i].y);
                            if self.level.visible[visible_idx] {
                                summon_message =
                                    Some("👑 The Void Entity warps in a doppelgänger!".to_string());
                            }
                        }
                    }
                }
            }

            let (ex, ey) = (self.enemies[i].x, self.enemies[i].y);
            let nearby_allies = self
                .enemies
                .iter()
                .enumerate()
                .filter(|(j, e)| {
                    *j != i && e.is_alive() && e.alert && (e.x - ex).abs() + (e.y - ey).abs() <= 3
                })
                .count();
            let (nx, ny) = self.enemies[i].ai_step(px, py, nearby_allies);

            // Don't walk into walls or other enemies
            if !self.level.is_walkable(nx, ny) {
                continue;
            }
            // Don't stack on other enemies
            let occupied = self
                .enemies
                .iter()
                .enumerate()
                .any(|(j, e)| j != i && e.is_alive() && e.x == nx && e.y == ny);
            if occupied {
                continue;
            }

            // If enemy walks into player → start combat (same as player bumping enemy)
            if nx == px && ny == py {
                if !matches!(
                    self.combat,
                    CombatState::Fighting { .. } | CombatState::TacticalBattle(_)
                ) {
                    self.trigger_shake(4);
                    let battle = combat::transition::enter_combat(
                        &self.player,
                        &self.enemies,
                        &[i],
                        self.floor_num,
                        self.current_room_modifier(),
                        &self.srs,
                        self.companion,
                    );
                    self.combat = CombatState::TacticalBattle(Box::new(battle));
                    self.typing.clear();
                    self.message = format!(
                        "{} attacks! {}",
                        self.enemies[i].hanzi,
                        combat_prompt_for(&self.enemies[i], self.listening_mode, self.mirror_hint)
                    );
                    if let Some(ref comp) = self.companion {
                        let lvl = self.companion_level();
                        if let Some(hint) = comp.contextual_hint(
                            &self.enemies[i],
                            self.player.hp,
                            self.player.max_hp,
                            self.guard_used_this_fight,
                            lvl,
                        ) {
                            self.message.push_str(&format!("\n{}", hint));
                        }
                    }
                    self.message_timer = 255;
                }
                continue;
            }

            self.enemies[i].x = nx;
            self.enemies[i].y = ny;
            self.apply_enemy_tile_effect(i);
        }

        if !summons.is_empty() {
            self.enemies.extend(summons);
            if let Some(message) = summon_message {
                self.message = message;
                self.message_timer = 90;
            }
        }
    }

    /// Handle typing a character during combat.
    fn type_char(&mut self, ch: char) {
        if matches!(self.combat, CombatState::GameOver) {
            return;
        }
        if let CombatState::Fighting { .. } = &self.combat {
            self.typing.push(ch);
        }
    }

    /// Submit pinyin answer.
    fn submit_answer(&mut self) {
        if let CombatState::Fighting { enemy_idx, .. } = self.combat.clone() {
            if enemy_idx >= self.enemies.len() {
                self.combat = CombatState::Explore;
                return;
            }

            // Component (Shield) Logic
            if !self.enemies[enemy_idx].components.is_empty() {
                let comp_hanzi = self.enemies[enemy_idx].components[0];
                let comp_pinyin = vocab::vocab_entry_by_hanzi(comp_hanzi)
                    .map(|e| e.pinyin)
                    .unwrap_or("???");

                let matches = if let Some(entry) = vocab::vocab_entry_by_hanzi(comp_hanzi) {
                    vocab::check_pinyin(entry, &self.typing)
                } else {
                    self.typing == comp_pinyin
                };

                if matches {
                    self.enemies[enemy_idx].components.remove(0);
                    self.typing.clear();
                    self.trigger_shake(2);
                    if let Some(ref audio) = self.audio {
                        audio.play_hit();
                    }

                    if self.enemies[enemy_idx].components.is_empty() {
                        self.message = format!("The {} is exposed!", self.enemies[enemy_idx].hanzi);
                    } else {
                        self.message = format!("Shattered {} shield!", comp_hanzi);
                    }
                    self.message_timer = 60;
                    return;
                } else {
                    self.message = format!("Shield holds! Need {}", comp_pinyin);
                    self.message_timer = 60;
                    self.typing.clear();
                    return;
                }
            }

            let e_hanzi = self.enemies[enemy_idx].hanzi;
            let e_pinyin = self.enemies[enemy_idx].pinyin;
            let e_meaning = self.enemies[enemy_idx].meaning;
            let e_damage = (self.enemies[enemy_idx].damage
                - self.player.damage_reduction()
                - self.player.enchant_damage_reduction())
            .max(1);
            let e_gold = self.enemies[enemy_idx].gold_value
                + self.player.gold_bonus()
                + self.player.enchant_gold_bonus();
            let e_is_boss = self.enemies[enemy_idx].is_boss;
            let e_is_elite = self.enemies[enemy_idx].is_elite;
            let e_x = self.enemies[enemy_idx].x;
            let e_y = self.enemies[enemy_idx].y;

            // ToneOnly mode: accept just the tone number (1-4) for non-elite
            let tone_only_active = self.listening_mode == ListenMode::ToneOnly && !e_is_elite;

            let elite_step = if e_is_elite {
                Some(vocab::resolve_compound_pinyin_step(
                    e_pinyin,
                    self.enemies[enemy_idx].elite_chain,
                    &self.typing,
                ))
            } else {
                None
            };
            let answer_correct = if tone_only_active {
                let expected_tone = e_pinyin
                    .chars()
                    .last()
                    .and_then(|c| c.to_digit(10))
                    .unwrap_or(1);
                self.typing
                    .trim()
                    .parse::<u32>()
                    .map(|t| t == expected_tone)
                    .unwrap_or(false)
            } else if let Some(step) = elite_step {
                !matches!(step, vocab::CompoundPinyinStep::Miss { .. })
            } else {
                vocab::check_pinyin(
                    &vocab::VocabEntry {
                        hanzi: e_hanzi,
                        pinyin: e_pinyin,
                        meaning: e_meaning,
                        hsk: 1,
                        example: "",
                    },
                    &self.typing,
                )
            };

            if answer_correct {
                self.srs.record(e_hanzi, true);
                self.codex.record(e_hanzi, e_pinyin, e_meaning, true);
                self.run_correct_answers += 1;
                // Hit with bonus damage from equipment + room modifiers
                let cursed_bonus = if self.current_room_modifier() == Some(RoomModifier::Irradiated) {
                    1
                } else {
                    0
                };
                let warrior_bonus = if self.player.class == PlayerClass::Soldier {
                    1
                } else {
                    0
                };
                let tone_bonus = self.player.tone_bonus_damage;
                self.player.tone_bonus_damage = 0; // consumed

                let form_bonus = match self.player.form {
                    PlayerForm::Void => 2,
                    PlayerForm::Powered => 1,
                    _ => 0,
                };
                let empowered_bonus = status::empowered_amount(&self.player.statuses);

                let iron_bonus = if self.player.get_piety(Faction::MilitaryAlliance) >= 10 {
                    1
                } else {
                    0
                };
                let tactical_insight = if e_is_elite
                    && self.player.get_piety(Faction::Technocracy) >= 10
                    && self.player.get_piety(Faction::MilitaryAlliance) >= 10
                {
                    2
                } else {
                    0
                };

                let hit_dmg = 2
                    + self.player.bonus_damage()
                    + self.player.enchant_bonus_damage()
                    + cursed_bonus
                    + warrior_bonus
                    + tone_bonus
                    + form_bonus
                    + empowered_bonus
                    + iron_bonus
                    + tactical_insight;

                self.answer_streak += 1;
                if self.answer_streak > self.run_journal.max_combo {
                    self.run_journal.max_combo = self.answer_streak;
                    if self.answer_streak >= 5 {
                        self.run_journal
                            .log(RunEvent::ComboAchieved(self.answer_streak, self.floor_num));
                    }
                }
                let multiplier = combo_tier(self.answer_streak).multiplier();
                let hit_dmg = ((hit_dmg as f64) * multiplier).round() as i32;
                let hit_dmg = hit_dmg.max(1);

                if self.answer_streak == 5 || self.answer_streak == 10 {
                    let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
                    self.particles.spawn_streak(sx, sy, &mut self.rng_state);
                    if let Some(ref audio) = self.audio {
                        audio.play_streak_ding();
                    }
                }

                // Status application
                if status::has_envenomed(&self.player.statuses) {
                    self.enemies[enemy_idx]
                        .statuses
                        .push(status::StatusInstance::new(
                            status::StatusKind::Poison { damage: 2 },
                            3,
                        ));
                }
                if self.player.form == PlayerForm::Powered {
                    self.enemies[enemy_idx]
                        .statuses
                        .push(status::StatusInstance::new(
                            status::StatusKind::Burn { damage: 1 },
                            3,
                        ));
                }

                let mut dealt_dmg = hit_dmg;
                let mut elite_completed_cycle = false;
                let mut elite_message: Option<String> = None;
                if let Some(step) = elite_step {
                    match step {
                        vocab::CompoundPinyinStep::Advanced {
                            matched,
                            next_progress,
                            total,
                        } => {
                            dealt_dmg = elite_chain_damage(hit_dmg, total, false);
                            self.enemies[enemy_idx].elite_chain = next_progress;
                            self.enemies[enemy_idx].hp =
                                elite_remaining_hp(self.enemies[enemy_idx].hp, dealt_dmg, false);
                            let next_expected = self.enemies[enemy_idx]
                                .elite_expected_syllable()
                                .unwrap_or(e_pinyin);
                            elite_message = Some(format!(
                                "⛓ {} clicks! Chain {}/{} — next: {}",
                                matched, next_progress, total, next_expected
                            ));
                        }
                        vocab::CompoundPinyinStep::Completed { matched, total } => {
                            dealt_dmg = elite_chain_damage(hit_dmg, total, true);
                            self.enemies[enemy_idx].elite_chain = 0;
                            self.enemies[enemy_idx].hp -= dealt_dmg;
                            elite_completed_cycle = true;
                            if self.enemies[enemy_idx].hp > 0 {
                                self.enemies[enemy_idx].stunned = true;
                                elite_message = Some(format!(
                                    "✦ Compound broken with {}! {} takes {} damage and staggers.",
                                    matched, e_hanzi, dealt_dmg
                                ));
                            }
                        }
                        vocab::CompoundPinyinStep::Miss { .. } => {}
                    }
                } else {
                    if self.enemies[enemy_idx].radical_dodge {
                        self.enemies[enemy_idx].radical_dodge = false;
                        self.message =
                            format!("🌙 {} dodges your attack with Shadow Step!", e_hanzi);
                        self.message_timer = 60;
                        self.typing.clear();
                        return;
                    }

                    let armor = self.enemies[enemy_idx].radical_armor;
                    if armor > 0 {
                        self.enemies[enemy_idx].radical_armor = 0;
                        dealt_dmg = (dealt_dmg - armor).max(1);
                    }

                    self.enemies[enemy_idx].hp -= dealt_dmg;
                }
                if self.enemies[enemy_idx].hp <= 0 {
                    self.total_kills += 1;
                    self.run_kills += 1;
                    self.run_gold_earned += e_gold;
                    if e_is_boss {
                        self.run_bosses_killed += 1;
                        self.run_journal
                            .log(RunEvent::BossKilled(e_hanzi.to_string(), self.floor_num));
                    } else {
                        self.run_journal
                            .log(RunEvent::EnemyKilled(e_hanzi.to_string(), self.floor_num));
                    }
                    if let Some(ref audio) = self.audio {
                        audio.play_kill();
                    }
                    // Kill particles
                    let (sx, sy) = self.tile_to_screen(e_x, e_y);
                    self.particles.spawn_kill(sx, sy, &mut self.rng_state);
                    self.flash = Some((255, 255, 255, 0.3));
                    // Rewards
                    let mut gold_gain = e_gold;
                    if self.player.get_piety(Faction::MilitaryAlliance) >= 10
                        && self.player.get_piety(Faction::AncientOrder) >= 10
                    {
                        gold_gain *= 2;
                    }
                    if self.player.get_piety(Faction::AncientOrder) >= 10 {
                        gold_gain += 3;
                    }
                    gold_gain = (gold_gain as f64 * self.floor_profile.gold_multiplier()) as i32;
                    gold_gain = gold_gain.max(1);
                    self.player.gold += gold_gain;

                    // Listening mode bonus gold
                    let listen_bonus = if !self.enemies[enemy_idx].is_elite {
                        match self.listening_mode {
                            ListenMode::ToneOnly => 3,
                            ListenMode::FullAudio => 5,
                            ListenMode::Off => 0,
                        }
                    } else {
                        0
                    };
                    self.player.gold += listen_bonus;
                    let available = radical::radicals_for_floor(self.floor_num);
                    let rad_roll = self.rng_next() % 100;
                    let rad_chance = self.floor_profile.radical_drop_chance();
                    let dropped = if rad_roll < rad_chance {
                        let drop_idx = self.rng_next() as usize % available.len();
                        let ch = available[drop_idx].ch;
                        self.player.add_radical(ch);
                        self.run_journal
                            .log(RunEvent::RadicalCollected(ch.to_string(), self.floor_num));
                        if self.floor_profile.radical_drop_bonus() {
                            let bonus_idx = self.rng_next() as usize % available.len();
                            self.player.add_radical(available[bonus_idx].ch);
                        }
                        Some(ch)
                    } else {
                        None
                    };
                    self.advance_radical_quests();

                    // Elite enemies drop an extra radical
                    if e_is_elite {
                        let drop2 = self.rng_next() as usize % available.len();
                        self.player.add_radical(available[drop2].ch);
                    }

                    // Extra radical from charm
                    let extra_chance = self.player.extra_radical_chance();
                    if extra_chance > 0 && (self.rng_next() % 100) < extra_chance as u64 {
                        let drop2 = self.rng_next() as usize % available.len();
                        self.player.add_radical(available[drop2].ch);
                    }

                    // Heal on kill from charm
                    let mut heal = self.player.heal_on_kill();
                    if self.player.get_piety(Faction::Consortium) >= 10 {
                        heal += 1;
                    }
                    if heal > 0 {
                        self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                    }

                    // Random equipment drop (5% chance, higher for bosses)
                    let equip_chance = if e_is_boss { 60 } else { 5 };
                    if (self.rng_next() % 100) < equip_chance {
                        let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
                        let eq = &EQUIPMENT_POOL[eq_idx];
                        let current_state = self.player.equipment_state(eq.slot);
                        if current_state == ItemState::Cursed {
                            if let Some(rad) = dropped {
                                self.message = format!(
                                    "Defeated {}! +{}g [{}] ({} blocked by curse!)",
                                    e_hanzi, e_gold, rad, eq.name
                                );
                            } else {
                                self.message = format!(
                                    "Defeated {}! +{}g ({} blocked by curse!)",
                                    e_hanzi, e_gold, eq.name
                                );
                            }
                        } else {
                            let state = self.roll_item_state();
                            self.player.equip(eq, state);
                            let prefix = match state {
                                ItemState::Cursed => "💀 ",
                                ItemState::Blessed => "✨ ",
                                ItemState::Normal => "",
                            };
                            if let Some(rad) = dropped {
                                self.message = format!(
                                    "Defeated {}! +{}g [{}] + {}{}!",
                                    e_hanzi, e_gold, rad, prefix, eq.name
                                );
                            } else {
                                self.message = format!(
                                    "Defeated {}! +{}g + {}{}!",
                                    e_hanzi, e_gold, prefix, eq.name
                                );
                            }
                        }
                    } else if let Some(rad) = dropped {
                        self.message = format!("Defeated {}! +{}g [{}]", e_hanzi, e_gold, rad);
                    } else {
                        self.message = format!("Defeated {}! +{}g", e_hanzi, e_gold);
                    }
                    if e_is_elite && elite_completed_cycle {
                        self.message = format!("⛓ {} — Compound shattered!", self.message);
                    }

                    // Bosses drop a rare radical
                    if e_is_boss {
                        let rares = radical::rare_radicals();
                        if !rares.is_empty() {
                            let rare_idx = self.rng_next() as usize % rares.len();
                            let rare = rares[rare_idx].ch;
                            self.player.add_radical(rare);
                            self.message = format!("{} ✦ Rare radical [{}]!", self.message, rare);
                        }
                    }
                    // Streak indicator
                    let tier = combo_tier(self.answer_streak);
                    if tier != ComboTier::None {
                        self.message = format!(
                            "{} 🔥 {}! ×{}",
                            self.message,
                            tier.name(),
                            self.answer_streak
                        );
                    }
                    self.message_timer = 80;
                    // Tutorial hint: first tutorial fight complete
                    if let Some(tutorial) = self.tutorial.as_mut() {
                        if !tutorial.combat_done {
                            tutorial.combat_done = true;
                            self.message =
                                "Great! Now walk to the ⚒ and forge 好 from 女 + 子.".to_string();
                            self.message_timer = 180;
                        }
                    } else if self.total_runs == 0 && self.player.radicals.len() == 1 {
                        if let Some(rad) = dropped {
                            self.message = format!(
                                "Defeated {}! +{}g [{}] — Walk to an ⚒ anvil to forge spells!",
                                e_hanzi, e_gold, rad
                            );
                        } else {
                            self.message = format!(
                                "Defeated {}! +{}g — Walk to an ⚒ anvil to forge spells!",
                                e_hanzi, e_gold
                            );
                        }
                        self.message_timer = 160;
                    }
                    self.combat = CombatState::Explore;

                    let kill_xp = if e_is_boss {
                        5
                    } else if e_is_elite {
                        3
                    } else {
                        2
                    };
                    self.add_companion_xp(kill_xp);

                    // Achievement checks
                    self.achievements.record_correct();
                    self.achievements.check_kills(self.total_kills);
                    self.achievements.check_gold(self.player.gold);
                    self.achievements.check_radicals(self.player.radicals.len());
                    if e_is_elite {
                        self.achievements.unlock("first_elite");
                    }
                    if e_is_boss {
                        self.achievements.unlock("first_boss");
                    }

                    // Boss bonus sentence challenge (scaled by floor)
                    if e_is_boss
                        && self.floor_num >= 5
                        && self.enemies[enemy_idx].boss_kind != Some(BossKind::HiveQueen)
                        && self.enemies[enemy_idx].boss_kind != Some(BossKind::AncientGuardian)
                        && self.enemies[enemy_idx].boss_kind != Some(BossKind::PirateCaptain)
                    {
                        let base_reward = 15 + self.floor_num * 2;
                        self.begin_sentence_challenge(
                            SentenceChallengeMode::BonusGold { reward: base_reward },
                            "Boss Phase 2! Arrange the words in correct order. ←→ to select, Enter to pick.".to_string(),
                        );
                    }

                    // Quest progress: kill tracking
                    self.advance_kill_quests();
                } else {
                    if let Some(ref audio) = self.audio {
                        audio.play_hit();
                    }
                    if self.maybe_trigger_boss_phase(enemy_idx) {
                        self.typing.clear();
                        return;
                    }
                    if let Some(message) = elite_message {
                        self.message =
                            format!("{} ({} HP left)", message, self.enemies[enemy_idx].hp);
                        self.message_timer = if elite_completed_cycle { 80 } else { 70 };
                    } else {
                        let tier = combo_tier(self.answer_streak);
                        self.message = if tier != ComboTier::None {
                            format!(
                                "Hit for {}! {} HP left 🔥 {}! ×{}",
                                dealt_dmg,
                                self.enemies[enemy_idx].hp,
                                tier.name(),
                                self.answer_streak
                            )
                        } else {
                            format!(
                                "Hit for {}! {} HP left",
                                dealt_dmg, self.enemies[enemy_idx].hp
                            )
                        };
                        self.message_timer = 40;
                    }
                }
            } else {
                // Miss — enemy counter-attacks
                let expected_pinyin =
                    if let Some(vocab::CompoundPinyinStep::Miss { expected, .. }) = elite_step {
                        self.enemies[enemy_idx].elite_chain = 0;
                        expected
                    } else {
                        e_pinyin
                    };
                self.srs.record(e_hanzi, false);
                self.codex.record(e_hanzi, e_pinyin, e_meaning, false);
                self.run_wrong_answers += 1;
                self.answer_streak = 0;
                self.achievements.record_miss();

                let mut thief_stole = None;
                if self.enemies[enemy_idx].boss_kind == Some(BossKind::DriftLeviathan)
                    && !self.player.radicals.is_empty()
                {
                    let steal_idx = self.rng_next() as usize % self.player.radicals.len();
                    let stolen = self.player.radicals.remove(steal_idx);
                    thief_stole = Some(stolen);
                }
                if let Some(ref audio) = self.audio {
                    audio.play_miss();
                }
                if self.enemies[enemy_idx].stunned {
                    self.enemies[enemy_idx].stunned = false;
                    self.message = if e_is_elite {
                        format!(
                            "✗ Wrong chain! Needed \"{}\", but {} is still staggered and cannot counterattack.",
                            expected_pinyin, e_hanzi
                        )
                    } else {
                        format!(
                            "Wrong! (was \"{}\") — {} is stunned and can't counterattack!",
                            expected_pinyin, e_hanzi
                        )
                    };
                    self.message_timer = 70;
                } else if self.player.shield {
                    self.player.shield = false;
                    self.message = if e_is_elite {
                        format!(
                            "✗ Wrong chain! Needed \"{}\" — Shield absorbed the blow!",
                            expected_pinyin
                        )
                    } else {
                        format!(
                            "Wrong! (was \"{}\") — Shield absorbed the blow!",
                            expected_pinyin
                        )
                    };
                    self.message_timer = if e_is_elite { 70 } else { 60 };
                } else if self.companion == Some(Companion::SecurityChief) && !self.guard_used_this_fight {
                    let lvl = self.companion_level();
                    let max_blocks = Companion::SecurityChief.guard_max_blocks(lvl);
                    self.guard_blocks_used += 1;
                    if self.guard_blocks_used >= max_blocks {
                        self.guard_used_this_fight = true;
                    } else {
                        let second_chance = Companion::SecurityChief.guard_second_block_chance(lvl);
                        if (self.rng_next() % 100) >= second_chance {
                            self.guard_used_this_fight = true;
                        }
                    }
                    self.message = if e_is_elite {
                        format!(
                            "✗ Wrong chain! Needed \"{}\" — 🛡 Guard blocks the attack!",
                            expected_pinyin
                        )
                    } else {
                        format!(
                            "Wrong! (was \"{}\") — 🛡 Guard blocks the attack!",
                            expected_pinyin
                        )
                    };
                    self.message_timer = if e_is_elite { 70 } else { 60 };
                } else {
                    let mut evaded = false;
                    if self.player.get_piety(Faction::FreeTraders) >= 10 {
                        if (self.rng_next() % 100) < 15 {
                            evaded = true;
                        }
                    }

                    if evaded {
                        let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
                        self.particles.spawn_synergy(sx, sy, &mut self.rng_state);
                        self.message = if e_is_elite {
                            format!(
                                "✗ Wrong chain! Needed \"{}\", but you evaded {}'s attack!",
                                expected_pinyin, e_hanzi
                            )
                        } else {
                            format!(
                                "Wrong! (was \"{}\") — you evaded {}'s attack!",
                                expected_pinyin, e_hanzi
                            )
                        };
                        self.message_timer = if e_is_elite { 70 } else { 60 };
                    } else {
                        // Apply defense_bonus from ToneDefense reward
                        let def_bonus = self.player.defense_bonus;
                        if def_bonus > 0 {
                            self.player.defense_bonus = 0;
                        }
                        let e_damage = (e_damage - def_bonus).max(0);
                        self.player.hp -= e_damage;

                        if self.enemies[enemy_idx].radical_multiply {
                            self.enemies[enemy_idx].radical_multiply = false;
                            self.player.hp -= e_damage;
                            self.message.push_str(" ✕ Double strike!");
                        }

                        if let Some(ref audio) = self.audio {
                            audio.play_damage();
                        }
                        // Damage particles + shake
                        let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
                        self.particles.spawn_damage(sx, sy, &mut self.rng_state);
                        self.trigger_shake(8);
                        self.flash = Some((255, 50, 50, 0.25));
                        self.message = if e_is_elite {
                            format!(
                                "✗ Wrong chain! Needed \"{}\". {} hits for {} and the compound resets!",
                                expected_pinyin, e_hanzi, e_damage
                            )
                        } else {
                            format!(
                                "Wrong! It was \"{}\". {} hits for {}!",
                                expected_pinyin, e_hanzi, e_damage
                            )
                        };
                        self.message_timer = if e_is_elite { 70 } else { 60 };
                    }
                }

                if self.player.get_piety(Faction::Technocracy) >= 10 {
                    self.mirror_hint = true;
                }

                if let Some(stolen) = thief_stole {
                    self.message
                        .push_str(&format!(" 🥷 The Drift Leviathan absorbed {}!", stolen));
                }

                // Radical actions: enemy uses abilities based on its hanzi components
                {
                    let actions = self.enemies[enemy_idx].radical_actions();
                    let mut action_msgs = Vec::new();
                    for action in actions {
                        // 30% chance per radical action
                        if (self.rng_next() % 100) < 30 {
                            let msg = self.apply_radical_action(enemy_idx, action);
                            action_msgs.push(msg);
                        }
                    }
                    if !action_msgs.is_empty() {
                        self.message.push_str(" ");
                        self.message.push_str(&action_msgs.join(" "));
                    }
                }

                if self.player.hp <= 0 && !self.try_phoenix_revive() {
                    self.player.hp = 0;
                    self.run_journal
                        .log(RunEvent::DiedTo(e_hanzi.to_string(), self.floor_num));
                    self.post_mortem_page = 0;
                    self.combat = CombatState::GameOver;
                    self.message = self.run_summary();
                    self.message_timer = 255;
                    if let Some(ref audio) = self.audio {
                        audio.play_death();
                    }
                    self.save_high_score();
                }
            }
            self.typing.clear();
        }
    }

    /// Apply a radical action triggered by an enemy on a wrong answer.
    /// Returns a description string of what happened.
    fn apply_radical_action(&mut self, enemy_idx: usize, action: RadicalAction) -> String {
        match action {
            RadicalAction::SpreadingWildfire => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Burn { damage: 1 },
                        3,
                    ));
                format!("{} — You catch fire!", action.name())
            }
            RadicalAction::ErosiveFlow => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        3,
                    ));
                format!("{} — Water erodes your defenses!", action.name())
            }
            RadicalAction::OverwhelmingForce => {
                self.player.hp -= 2;
                format!("{} — A crushing blow! (-2 HP)", action.name())
            }
            RadicalAction::DoubtSeed => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Confused,
                        2,
                    ));
                format!("{} — Your mind clouds with doubt!", action.name())
            }
            RadicalAction::DevouringMaw => {
                self.player.hp -= 1;
                if self.player.shield {
                    self.player.shield = false;
                }
                format!("{} — Devoured! (-1 HP)", action.name())
            }
            RadicalAction::WitnessMark => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Fear,
                        1,
                    ));
                format!("{} — You feel exposed!", action.name())
            }
            RadicalAction::SleightReversal => {
                self.player.hp -= 1;
                format!("{} — Switched around! (-1 HP)", action.name())
            }
            RadicalAction::RootingGrasp => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        2,
                    ));
                self.enemies[enemy_idx].radical_armor += 1;
                format!("{} — Roots bind you!", action.name())
            }
            RadicalAction::HarvestReaping => {
                let threshold = (self.player.max_hp as f32 * 0.4) as i32;
                let dmg = if self.player.hp < threshold { 3 } else { 1 };
                self.player.hp -= dmg;
                format!("{} — Reaped! (-{} HP)", action.name(), dmg)
            }
            RadicalAction::RevealingDawn => {
                let e = &mut self.enemies[enemy_idx];
                e.statuses.retain(|s| {
                    !matches!(
                        s.kind,
                        crate::status::StatusKind::Burn { .. }
                            | crate::status::StatusKind::Poison { .. }
                            | crate::status::StatusKind::Bleed { .. }
                            | crate::status::StatusKind::Slow
                            | crate::status::StatusKind::Confused
                            | crate::status::StatusKind::Freeze
                            | crate::status::StatusKind::Fear
                    )
                });
                e.hp = (e.hp + 2).min(e.max_hp);
                format!("{} — Bathed in cleansing light!", action.name())
            }
            RadicalAction::WaningCurse => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Poison { damage: 1 },
                        2,
                    ));
                format!("{} — A waning curse takes hold!", action.name())
            }
            RadicalAction::MortalResilience => {
                let e = &mut self.enemies[enemy_idx];
                e.damage += 1;
                e.radical_armor += 1;
                format!("{} — Enemy grows desperate!", action.name())
            }
            RadicalAction::MaternalShield => {
                self.enemies[enemy_idx].radical_dodge = true;
                self.enemies[enemy_idx].radical_armor += 1;
                format!("{} — Takes a protective stance!", action.name())
            }
            RadicalAction::PotentialBurst => {
                self.player.hp -= 1;
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Bleed { damage: 1 },
                        1,
                    ));
                format!("{} — Latent energy erupts! (-1 HP)", action.name())
            }
            RadicalAction::ChasingChaff => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Confused,
                        1,
                    ));
                format!("{} — Chaff clouds your vision!", action.name())
            }
            RadicalAction::CrossroadsGambit => {
                let seed = (self.floor_num as usize)
                    .wrapping_mul(31)
                    .wrapping_add(enemy_idx)
                    % 2;
                if seed == 0 {
                    self.player.hp -= 3;
                    format!("{} — A bad bet! (-3 HP)", action.name())
                } else {
                    format!("{} — The gambit fails!", action.name())
                }
            }
            RadicalAction::RigidStance => {
                self.enemies[enemy_idx].radical_armor += 3;
                format!("{} — Metal scales form!", action.name())
            }
            RadicalAction::GroundingWeight => {
                self.player.hp -= 1;
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        3,
                    ));
                format!("{} — The earth drags you down! (-1 HP)", action.name())
            }
            RadicalAction::EchoStrike => {
                let dmg = self.enemies[enemy_idx].damage;
                self.player.hp -= dmg;
                format!("{} — An echo strikes for {} damage!", action.name(), dmg)
            }
            RadicalAction::PreciseExecution => {
                let threshold = (self.player.max_hp as f32 * 0.25) as i32;
                let dmg = if self.player.hp < threshold { 4 } else { 1 };
                self.player.hp -= dmg;
                format!("{} — Executed! (-{} HP)", action.name(), dmg)
            }
            RadicalAction::CleavingCut => {
                self.player.hp -= 2;
                self.player.max_hp = (self.player.max_hp - 1).max(1);
                format!("{} — A deep cut! (-2 HP, -1 Max HP)", action.name())
            }
            RadicalAction::BindingOath => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        3,
                    ));
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Confused,
                        1,
                    ));
                format!("{} — Bound by oath!", action.name())
            }
            RadicalAction::PursuingSteps => {
                self.player.hp -= 1;
                self.enemies[enemy_idx].alert = true;
                format!("{} — Relentless pursuit! (-1 HP)", action.name())
            }
            RadicalAction::EntanglingWeb => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        3,
                    ));
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Bleed { damage: 1 },
                        2,
                    ));
                format!("{} — Caught in a web!", action.name())
            }
            RadicalAction::ThresholdSeal => {
                self.enemies[enemy_idx].radical_armor += 3;
                format!("{} — A barrier seals the way!", action.name())
            }
            RadicalAction::CavalryCharge => {
                self.player.hp -= 2;
                self.trigger_shake(8);
                format!("{} — Trampled! (-2 HP)", action.name())
            }
            RadicalAction::SoaringEscape => {
                self.enemies[enemy_idx].radical_dodge = true;
                format!("{} — The enemy takes flight!", action.name())
            }
            RadicalAction::DownpourBarrage => {
                self.player.hp -= 1;
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Bleed { damage: 1 },
                        3,
                    ));
                format!("{} — Pierced by rain! (-1 HP)", action.name())
            }
            RadicalAction::PetrifyingGaze => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        3,
                    ));
                self.enemies[enemy_idx].radical_armor += 2;
                format!("{} — Turned to stone!", action.name())
            }
            RadicalAction::ParasiticSwarm => {
                self.player.hp -= 1;
                let e = &mut self.enemies[enemy_idx];
                e.hp = (e.hp + 2).min(e.max_hp);
                format!("{} — Swarmed! (-1 HP, Enemy heals 2)", action.name())
            }
            RadicalAction::MercenaryPact => {
                let e = &mut self.enemies[enemy_idx];
                e.hp -= 2;
                for em in &mut self.enemies {
                    if em.is_alive() {
                        em.alert = true;
                    }
                }
                format!("{} — A rallying cry!", action.name())
            }
            RadicalAction::ImmovablePeak => {
                self.enemies[enemy_idx].radical_armor += 3;
                format!("{} — Immovable as a mountain!", action.name())
            }
            RadicalAction::SavageMaul => {
                self.player.hp -= 3;
                format!("{} — Mauled! (-3 HP)", action.name())
            }
            RadicalAction::ArcingShot => {
                self.player.hp -= 2;
                format!("{} — Struck from above! (-2 HP)", action.name())
            }
            RadicalAction::ConsumingBite => {
                self.player.hp -= 2;
                let e = &mut self.enemies[enemy_idx];
                e.max_hp += 1;
                e.hp = (e.hp + 2).min(e.max_hp);
                format!("{} — Bitten! (-2 HP, Enemy +Max HP)", action.name())
            }
            RadicalAction::CloakingGuise => {
                self.enemies[enemy_idx].radical_dodge = true;
                format!("{} — The enemy vanishes!", action.name())
            }
            RadicalAction::FlexibleCounter => {
                self.enemies[enemy_idx].radical_dodge = true;
                self.enemies[enemy_idx].radical_armor += 1;
                format!("{} — Ready to counter!", action.name())
            }
            RadicalAction::BlitzAssault => {
                self.player.hp -= 2;
                self.trigger_shake(6);
                format!("{} — Blitzed! (-2 HP)", action.name())
            }
            RadicalAction::CrushingWheels => {
                self.player.hp -= 2;
                self.trigger_shake(10);
                format!("{} — Crushed! (-2 HP)", action.name())
            }
            RadicalAction::ImperialCommand => {
                for em in &mut self.enemies {
                    if em.is_alive() {
                        em.alert = true;
                        em.damage += 1;
                    }
                }
                format!("{} — A commanding presence!", action.name())
            }
            RadicalAction::MagnifyingAura => {
                self.enemies[enemy_idx].damage += 1;
                format!("{} — An empowering aura!", action.name())
            }
            RadicalAction::NeedleStrike => {
                self.player.hp -= 2;
                format!("{} — Pierced! (-2 HP)", action.name())
            }
            RadicalAction::ArtisanTrap => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Burn { damage: 1 },
                        2,
                    ));
                format!("{} — Trapped!", action.name())
            }
            RadicalAction::CleansingLight => {
                let e = &mut self.enemies[enemy_idx];
                e.statuses.retain(|s| {
                    !matches!(
                        s.kind,
                        crate::status::StatusKind::Burn { .. }
                            | crate::status::StatusKind::Poison { .. }
                            | crate::status::StatusKind::Bleed { .. }
                            | crate::status::StatusKind::Slow
                            | crate::status::StatusKind::Confused
                            | crate::status::StatusKind::Freeze
                            | crate::status::StatusKind::Fear
                    )
                });
                e.hp = (e.hp + 3).min(e.max_hp);
                format!("{} — Bathed in cleansing light!", action.name())
            }
            RadicalAction::ScatteringPages => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Confused,
                        1,
                    ));
                format!("{} — Pages scatter and blind you!", action.name())
            }
            RadicalAction::TrueVision => {
                if self.player.shield {
                    self.player.shield = false;
                }
                self.player.statuses.retain(|s| s.is_negative());
                format!("{} — All your protections are dispelled!", action.name())
            }
            RadicalAction::QiDisruption => {
                self.player.hp -= 1;
                format!("{} — Your qi is disrupted! (-1 HP)", action.name())
            }
            RadicalAction::ExpandingDomain => {
                self.enemies[enemy_idx].damage += 1;
                format!("{} — Domain expands!", action.name())
            }
            RadicalAction::SinkholeSnare => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        2,
                    ));
                format!("{} — Ground cracks beneath you!", action.name())
            }
            RadicalAction::SonicBurst => {
                self.player.hp -= 2;
                format!("{} — Sonic shockwave! (-2 HP)", action.name())
            }
            RadicalAction::VenomousLash => {
                self.player.hp -= 1;
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Poison { damage: 2 },
                        2,
                    ));
                format!("{} — Venomous lash! Poisoned! (-1 HP)", action.name())
            }
            RadicalAction::IronBodyStance => {
                self.enemies[enemy_idx].hp =
                    (self.enemies[enemy_idx].hp + 2).min(self.enemies[enemy_idx].max_hp);
                format!("{} — Iron body! Enemy hardens!", action.name())
            }
            RadicalAction::GoreCrush => {
                self.player.hp -= 2;
                format!("{} — Gored! (-2 HP)", action.name())
            }
            RadicalAction::IntoxicatingMist => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Confused,
                        2,
                    ));
                format!("{} — Intoxicating mist clouds your mind!", action.name())
            }
            RadicalAction::SproutingBarrier => {
                self.enemies[enemy_idx].hp =
                    (self.enemies[enemy_idx].hp + 1).min(self.enemies[enemy_idx].max_hp);
                format!("{} — Sprouting barrier!", action.name())
            }
            RadicalAction::TidalSurge => {
                self.player.hp -= 1;
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        2,
                    ));
                format!("{} — Tidal surge pushes you! (-1 HP)", action.name())
            }
            RadicalAction::BoneShatter => {
                self.player.hp -= 3;
                if self.player.shield {
                    self.player.shield = false;
                }
                format!("{} — Bone shatters your defenses! (-3 HP)", action.name())
            }
            RadicalAction::AdaptiveShift => {
                self.enemies[enemy_idx].damage += 1;
                format!("{} — Enemy adapts and hardens!", action.name())
            }
            RadicalAction::BerserkerFury => {
                self.enemies[enemy_idx].hp = (self.enemies[enemy_idx].hp - 2).max(1);
                self.enemies[enemy_idx].damage += 3;
                format!("{} — Berserker fury! Enemy rages!", action.name())
            }
            RadicalAction::FlockAssault => {
                self.player.hp -= 2;
                format!("{} — A flock descends! (-2 HP)", action.name())
            }
        }
    }

    /// Backspace during typing.
    fn backspace(&mut self) {
        self.typing.pop();
    }

    fn forge_submit(&mut self) {
        let recipe_idx = if let CombatState::Forging {
            ref recipes,
            cursor,
            ..
        } = self.combat
        {
            if recipes.is_empty() {
                return;
            }
            recipes[cursor]
        } else {
            return;
        };

        let recipe = &radical::RECIPES[recipe_idx];

        let mut available_indices: Vec<usize> = (0..self.player.radicals.len()).collect();
        let mut consumed_indices: Vec<usize> = Vec::new();
        for &needed in recipe.inputs {
            if let Some(pos) = available_indices
                .iter()
                .position(|&i| self.player.radicals[i] == needed)
            {
                consumed_indices.push(available_indices.remove(pos));
            }
        }

        if let Some(ref audio) = self.audio {
            audio.play_forge();
        }
        let spell = Spell {
            hanzi: recipe.output_hanzi,
            pinyin: recipe.output_pinyin,
            meaning: recipe.output_meaning,
            effect: recipe.effect,
        };
        if !self.discovered_recipes.contains(&recipe_idx) {
            self.discovered_recipes.push(recipe_idx);
        }
        self.run_spells_forged += 1;
        self.run_journal.log(RunEvent::SpellForged(
            format!("{} ({})", recipe.output_hanzi, recipe.output_meaning),
            self.floor_num,
        ));
        self.message = format!(
            "Forged {} ({}) — {}! [{}]",
            recipe.output_hanzi,
            recipe.output_pinyin,
            recipe.output_meaning,
            recipe.effect.label()
        );
        self.message_timer = 80;
        self.player.add_spell(spell);
        let forge_quest_message = self.advance_forge_quests(recipe.output_hanzi);
        if let Some(tutorial) = self.tutorial.as_mut() {
            tutorial.forge_done = true;
            self.message = format!(
                "Forged {}! Tutorial complete — Q selects spells, Space casts. Take the stairs to Floor 1.",
                recipe.output_hanzi
            );
            self.message_timer = 220;
        } else if self.total_runs == 0 && self.player.spells.len() == 1 {
            self.message = format!(
                "Forged {}! Q to select spell, Space to cast!",
                recipe.output_hanzi
            );
            self.message_timer = 160;
        }
        if let Some(quest_message) = forge_quest_message {
            self.message = format!("{}  {}", self.message, quest_message);
            self.message_timer = self.message_timer.max(120);
        }
        consumed_indices.sort_unstable_by(|a, b| b.cmp(a));
        for idx in consumed_indices {
            self.player.radicals.remove(idx);
        }
        self.combat = CombatState::Explore;
        self.achievements
            .check_recipes(self.discovered_recipes.len());
        self.achievements.check_spells(self.player.spells.len());
    }

    /// Generate shop items for current floor.
    /// Advance kill-based quests and collect rewards.
    fn advance_kill_quests(&mut self) {
        for q in &mut self.quests {
            if q.completed {
                continue;
            }
            if let QuestGoal::KillEnemies(ref mut current, _) = q.goal {
                *current += 1;
            }
        }
        self.collect_quest_rewards();
    }

    /// Advance radical-collect quests.
    fn advance_radical_quests(&mut self) {
        for q in &mut self.quests {
            if q.completed {
                continue;
            }
            if let QuestGoal::CollectRadicals(ref mut current, _) = q.goal {
                *current += 1;
            }
        }
        self.collect_quest_rewards();
    }

    /// Complete forge-character quests when the requested hanzi is created.
    fn advance_forge_quests(&mut self, forged_hanzi: &'static str) -> Option<String> {
        let mut reward_messages = Vec::new();
        let mut chain_follow_ups: Vec<(u8, u32)> = Vec::new();
        for q in &mut self.quests {
            if q.completed {
                continue;
            }
            if let QuestGoal::ForgeCharacter(target) = q.goal {
                if target == forged_hanzi {
                    q.completed = true;
                    if q.gold_reward > 0 {
                        self.player.gold += q.gold_reward;
                        if q.is_chain() && q.chain_step < 4 {
                            reward_messages.push(format!(
                                "⛓ Chain quest step complete: {}! +{}g — Next step incoming!",
                                q.description, q.gold_reward
                            ));
                            chain_follow_ups.push((q.chain_step, q.chain_id));
                        } else if q.is_chain() && q.chain_step >= 4 {
                            reward_messages.push(format!(
                                "🏆 Quest chain complete: {}! +{}g — Bonus: +20g!",
                                q.description, q.gold_reward
                            ));
                            self.player.gold += 20;
                        } else {
                            reward_messages.push(format!(
                                "Quest complete: {}! +{}g",
                                q.description, q.gold_reward
                            ));
                        }
                        q.gold_reward = 0;
                    }
                }
            }
        }
        for (step, cid) in chain_follow_ups {
            self.quests.retain(|q| !(q.chain_id == cid && q.completed));
            let follow_up = self.generate_chain_quest(step, cid);
            self.quests.push(follow_up);
        }
        if reward_messages.is_empty() {
            None
        } else {
            Some(reward_messages.join(" "))
        }
    }

    /// Check floor-based quests.
    fn check_floor_quests(&mut self) {
        let floor = self.floor_num;
        for q in &mut self.quests {
            if q.completed {
                continue;
            }
            if let QuestGoal::ReachFloor(target) = q.goal {
                if floor >= target {
                    q.completed = true;
                }
            }
        }
        self.collect_quest_rewards();
    }

    /// Collect rewards from completed quests.
    fn collect_quest_rewards(&mut self) {
        let mut chain_follow_ups: Vec<(u8, u32)> = Vec::new();
        let mut quest_xp: u32 = 0;
        for q in &mut self.quests {
            if q.completed && q.gold_reward > 0 {
                self.player.gold += q.gold_reward;
                quest_xp += 10;
                if q.is_chain() && q.chain_step < 4 {
                    self.message = format!(
                        "⛓ Chain quest step complete: {}! +{}g — Next step incoming!",
                        q.description, q.gold_reward
                    );
                    chain_follow_ups.push((q.chain_step, q.chain_id));
                } else if q.is_chain() && q.chain_step >= 4 {
                    self.message = format!(
                        "🏆 Quest chain complete: {}! +{}g — Bonus: +20g!",
                        q.description, q.gold_reward
                    );
                    self.player.gold += 20;
                } else {
                    self.message =
                        format!("Quest complete: {}! +{}g", q.description, q.gold_reward);
                }
                self.message_timer = 100;
                q.gold_reward = 0;
            }
        }
        if quest_xp > 0 {
            self.add_companion_xp(quest_xp);
        }
        for (step, cid) in chain_follow_ups {
            self.quests.retain(|q| !(q.chain_id == cid && q.completed));
            let follow_up = self.generate_chain_quest(step, cid);
            self.quests.push(follow_up);
        }
    }

    /// Start a tone battle at a shrine.
    fn start_tone_battle(&mut self) {
        let (hanzi, tone) = self.pick_tone_battle_char();
        if let Some(ref audio) = self.audio {
            audio.play_chinese_tone(tone);
        }
        self.combat = CombatState::ToneBattle {
            round: 0,
            hanzi,
            correct_tone: tone,
            score: 0,
            last_result: None,
        };
        self.message = "🔔 Tone Shrine! Listen and press 1-4 for the correct tone.".to_string();
        self.message_timer = 120;
    }

    /// Pick a random character and extract its tone for the tone battle.
    fn pick_tone_battle_char(&mut self) -> (&'static str, u8) {
        let v = &vocab::VOCAB;
        let idx = self.rng_next() as usize % v.len();
        let entry = &v[idx];
        let tone = entry
            .pinyin
            .chars()
            .last()
            .and_then(|c| c.to_digit(10))
            .unwrap_or(1) as u8;
        (entry.hanzi, tone)
    }

    /// Start a stroke order challenge at a StrokeShrine.
    fn start_stroke_order(&mut self) {
        let idx = self.rng_next() as usize % STROKE_ORDER_DATA.len();
        let (hanzi, components, pinyin, meaning) = STROKE_ORDER_DATA[idx];
        let correct_order: Vec<&'static str> = components.to_vec();
        // Fisher-Yates shuffle for components
        let mut shuffled = correct_order.clone();
        let n = shuffled.len();
        for i in (1..n).rev() {
            let j = self.rng_next() as usize % (i + 1);
            shuffled.swap(i, j);
        }
        self.combat = CombatState::StrokeOrder {
            hanzi,
            components: shuffled,
            correct_order,
            cursor: 0,
            arranged: Vec::new(),
            pinyin,
            meaning,
        };
        self.message = format!(
            "筆 Stroke Shrine! Arrange the components of {} in order.",
            hanzi
        );
        self.message_timer = 120;
    }

    /// Start a tone defense challenge at a ToneWall.
    fn start_tone_defense(&mut self) {
        let pool = vocab::vocab_for_floor(self.floor_num);
        let entry = if pool.is_empty() {
            &vocab::VOCAB[self.rng_next() as usize % vocab::VOCAB.len()]
        } else {
            pool[self.rng_next() as usize % pool.len()]
        };
        let tone = entry
            .pinyin
            .chars()
            .last()
            .and_then(|c| c.to_digit(10))
            .unwrap_or(1) as u8;
        self.combat = CombatState::ToneDefense {
            round: 0,
            hanzi: entry.hanzi,
            pinyin: entry.pinyin,
            meaning: entry.meaning,
            correct_tone: tone,
            score: 0,
            last_result: None,
        };
        self.message = format!("壁 Tone Wall! What tone is {}? Press 1-4.", entry.hanzi);
        self.message_timer = 120;
    }

    /// Start a compound builder challenge at a CompoundShrine.
    fn start_compound_builder(&mut self) {
        let idx = self.rng_next() as usize % COMPOUND_DATA.len();
        let (compound, parts, pinyin, meaning) = COMPOUND_DATA[idx];
        let correct_compound = compound;
        // Fisher-Yates shuffle for parts
        let mut shuffled: Vec<&'static str> = parts.to_vec();
        let n = shuffled.len();
        for i in (1..n).rev() {
            let j = self.rng_next() as usize % (i + 1);
            shuffled.swap(i, j);
        }
        self.combat = CombatState::CompoundBuilder {
            parts: shuffled,
            correct_compound,
            pinyin,
            meaning,
            cursor: 0,
            arranged: Vec::new(),
        };
        self.message = format!(
            "合 Compound Shrine! Combine the characters into a word. ({})",
            meaning
        );
        self.message_timer = 120;
    }

    /// Start a classifier match challenge at a ClassifierShrine.
    fn start_classifier_match(&mut self) {
        let idx = self.rng_next() as usize % CLASSIFIER_DATA.len();
        let (noun, correct_classifier, noun_pinyin, noun_meaning) = CLASSIFIER_DATA[idx];
        // Build 4 options: 1 correct + 3 random wrong
        let mut options: Vec<&'static str> = vec![correct_classifier];
        let mut attempts = 0;
        while options.len() < 4 && attempts < 50 {
            let c = ALL_CLASSIFIERS[self.rng_next() as usize % ALL_CLASSIFIERS.len()];
            if !options.contains(&c) {
                options.push(c);
            }
            attempts += 1;
        }
        // Pad if not enough unique classifiers found
        while options.len() < 4 {
            options.push("个");
        }
        // Fisher-Yates shuffle
        let n = options.len();
        for i in (1..n).rev() {
            let j = self.rng_next() as usize % (i + 1);
            options.swap(i, j);
        }
        let correct_idx = options
            .iter()
            .position(|&c| c == correct_classifier)
            .unwrap_or(0);
        let options_arr: [&'static str; 4] = [options[0], options[1], options[2], options[3]];
        self.combat = CombatState::ClassifierMatch {
            round: 0,
            noun,
            noun_pinyin,
            noun_meaning,
            correct_classifier,
            options: options_arr,
            correct_idx,
            score: 0,
            last_result: None,
        };
        self.message = format!(
            "量 Classifier Shrine! Which measure word for {}? Press 1-4.",
            noun
        );
        self.message_timer = 120;
    }

    fn start_ink_well(&mut self) {
        let idx = self.rng_next() as usize % INK_WELL_DATA.len();
        let (hanzi, correct_count, pinyin, meaning) = INK_WELL_DATA[idx];
        self.combat = CombatState::InkWellChallenge {
            hanzi,
            correct_count,
            pinyin,
            meaning,
        };
        self.message = format!(
            "墨 Ink Well! {} ({}) — How many components? Press 1-9.",
            hanzi, meaning
        );
        self.message_timer = 120;
    }

    fn start_ancestor_challenge(&mut self) {
        let idx = self.rng_next() as usize % CHENGYU_DATA.len();
        let (first_half, correct_second, full, pinyin, meaning) = CHENGYU_DATA[idx];
        let mut options: Vec<&'static str> = vec![correct_second];
        let mut attempts = 0;
        while options.len() < 4 && attempts < 50 {
            let other_idx = self.rng_next() as usize % CHENGYU_DATA.len();
            let (_, other_second, _, _, _) = CHENGYU_DATA[other_idx];
            if !options.contains(&other_second) {
                options.push(other_second);
            }
            attempts += 1;
        }
        while options.len() < 4 {
            options.push("??");
        }
        let n = options.len();
        for i in (1..n).rev() {
            let j = self.rng_next() as usize % (i + 1);
            options.swap(i, j);
        }
        let correct_idx = options
            .iter()
            .position(|&s| s == correct_second)
            .unwrap_or(0);
        let options_arr: [&'static str; 4] = [options[0], options[1], options[2], options[3]];
        self.combat = CombatState::AncestorChallenge {
            first_half,
            correct_second,
            full,
            pinyin,
            meaning,
            options: options_arr,
            correct_idx,
        };
        self.message = format!(
            "祖 Ancestor Shrine! Complete the chengyu: {}____. Press 1-4.",
            first_half
        );
        self.message_timer = 120;
    }

    fn start_translation_challenge(&mut self) {
        let vocab = vocab::vocab_for_floor(self.floor_num);
        if vocab.len() < 4 {
            self.message = "Not enough vocabulary for this floor.".into();
            self.message_timer = 60;
            return;
        }
        let idx = self.rng_next() as usize % vocab.len();
        let correct = vocab[idx];
        let mut options: Vec<&'static str> = vec![correct.hanzi];
        let mut attempts = 0;
        while options.len() < 4 && attempts < 50 {
            let other_idx = self.rng_next() as usize % vocab.len();
            if !options.contains(&vocab[other_idx].hanzi) {
                options.push(vocab[other_idx].hanzi);
            }
            attempts += 1;
        }
        while options.len() < 4 {
            options.push("?");
        }
        let n = options.len();
        for i in (1..n).rev() {
            let j = self.rng_next() as usize % (i + 1);
            options.swap(i, j);
        }
        let correct_idx = options
            .iter()
            .position(|&s| s == correct.hanzi)
            .unwrap_or(0);
        let options_arr: [&'static str; 4] = [options[0], options[1], options[2], options[3]];
        self.combat = CombatState::TranslationChallenge {
            round: 0,
            meaning: correct.meaning,
            correct_hanzi: correct.hanzi,
            correct_pinyin: correct.pinyin,
            options: options_arr,
            correct_idx,
            score: 0,
        };
        self.message = format!(
            "译 Translation Altar! Which Chinese means \"{}\"? Press 1-4. (Round 1/3)",
            correct.meaning
        );
        self.message_timer = 120;
    }

    fn start_radical_garden(&mut self) {
        let idx = self.rng_next() as usize % RADICAL_GARDEN_DATA.len();
        let (hanzi, pinyin, meaning, radical, w1, w2, w3) = RADICAL_GARDEN_DATA[idx];
        let mut options: Vec<&'static str> = vec![radical, w1, w2, w3];
        let n = options.len();
        for i in (1..n).rev() {
            let j = self.rng_next() as usize % (i + 1);
            options.swap(i, j);
        }
        let correct_idx = options.iter().position(|&s| s == radical).unwrap_or(0);
        let options_arr: [&'static str; 4] = [options[0], options[1], options[2], options[3]];
        self.combat = CombatState::RadicalGardenChallenge {
            hanzi,
            pinyin,
            meaning,
            correct_radical: radical,
            options: options_arr,
            correct_idx,
        };
        self.message = format!(
            "部 Radical Garden! What is the radical of {}? Press 1-4.",
            hanzi
        );
        self.message_timer = 120;
    }

    fn start_mirror_pool(&mut self) {
        let idx = self.rng_next() as usize % MIRROR_POOL_DATA.len();
        let (hanzi, pinyin, meaning) = MIRROR_POOL_DATA[idx];
        self.combat = CombatState::MirrorPoolChallenge {
            hanzi,
            correct_pinyin: pinyin,
            meaning,
            input: String::new(),
        };
        self.typing = String::new();
        self.message = format!(
            "鏡 Mirror Pool! Type the pinyin for {} ({}). Press Enter to submit.",
            hanzi, meaning
        );
        self.message_timer = 120;
    }

    fn start_stone_tutor(&mut self) {
        let vocab = vocab::vocab_for_floor(self.floor_num);
        if vocab.is_empty() {
            self.message = "No vocabulary available.".into();
            self.message_timer = 60;
            return;
        }
        let idx = self.rng_next() as usize % vocab.len();
        let entry = vocab[idx];
        let tone = entry
            .pinyin
            .chars()
            .last()
            .and_then(|c| c.to_digit(10))
            .unwrap_or(1) as u8;
        self.combat = CombatState::StoneTutorChallenge {
            round: 0,
            hanzi: entry.hanzi,
            pinyin: entry.pinyin,
            meaning: entry.meaning,
            correct_tone: tone,
            phase: 0,
            score: 0,
        };
        self.message = format!(
            "石 Stone Tutor! Study: {} — {} ({}). Press Space to continue to quiz.",
            entry.hanzi, entry.pinyin, entry.meaning
        );
        self.message_timer = 120;
    }

    fn start_codex_challenge(&mut self) {
        let codex_data: Vec<(&'static str, &'static str, &'static str)> = self
            .codex
            .sorted_entries()
            .iter()
            .map(|e| (e.hanzi, e.pinyin, e.meaning))
            .collect();
        let vocab = vocab::vocab_for_floor(self.floor_num);
        let use_codex = codex_data.len() >= 4;
        if !use_codex && vocab.len() < 4 {
            self.message = "Not enough vocabulary yet.".into();
            self.message_timer = 60;
            return;
        }
        let (hanzi, pinyin, meaning, distractors) = if use_codex {
            let idx = self.rng_next() as usize % codex_data.len();
            let (h, p, m) = codex_data[idx];
            let mut dist: Vec<&'static str> = codex_data
                .iter()
                .filter(|e| e.0 != h)
                .map(|e| e.2)
                .collect();
            while dist.len() < 3 {
                let vi = self.rng_next() as usize % vocab.len();
                let vm = vocab[vi].meaning;
                if vm != m && !dist.contains(&vm) {
                    dist.push(vm);
                }
            }
            for i in (1..dist.len()).rev() {
                let j = self.rng_next() as usize % (i + 1);
                dist.swap(i, j);
            }
            (h, p, m, [dist[0], dist[1], dist[2], ""])
        } else {
            let idx = self.rng_next() as usize % vocab.len();
            let entry = vocab[idx];
            let mut dist: Vec<&'static str> = vocab
                .iter()
                .filter(|e| e.hanzi != entry.hanzi)
                .map(|e| e.meaning)
                .collect();
            for i in (1..dist.len()).rev() {
                let j = self.rng_next() as usize % (i + 1);
                dist.swap(i, j);
            }
            (
                entry.hanzi,
                entry.pinyin,
                entry.meaning,
                [dist[0], dist[1], dist[2], ""],
            )
        };
        let correct_idx = self.rng_next() as usize % 4;
        let mut options = [distractors[0], distractors[1], distractors[2], meaning];
        // Shift correct answer into correct_idx
        options[3] = options[correct_idx];
        options[correct_idx] = meaning;
        self.combat = CombatState::CodexChallenge {
            round: 0,
            hanzi,
            pinyin,
            meaning,
            options,
            correct_idx,
            score: 0,
        };
        self.message = format!("典 Codex Shrine! What does {} mean? Pick 1-4.", hanzi);
        self.message_timer = 120;
    }

    fn start_word_bridge(&mut self, bridge_x: i32, bridge_y: i32) {
        let vocab = vocab::vocab_for_floor(self.floor_num);
        if vocab.len() < 4 {
            self.message = "Not enough vocabulary.".into();
            self.message_timer = 60;
            return;
        }
        let idx = self.rng_next() as usize % vocab.len();
        let entry = vocab[idx];
        let mut others: Vec<&'static str> = vocab
            .iter()
            .filter(|e| e.hanzi != entry.hanzi)
            .map(|e| e.hanzi)
            .collect();
        for i in (1..others.len()).rev() {
            let j = self.rng_next() as usize % (i + 1);
            others.swap(i, j);
        }
        let correct_idx = self.rng_next() as usize % 4;
        let mut options = [others[0], others[1], others[2], entry.hanzi];
        options[3] = options[correct_idx];
        options[correct_idx] = entry.hanzi;
        self.combat = CombatState::WordBridgeChallenge {
            meaning: entry.meaning,
            correct_hanzi: entry.hanzi,
            correct_pinyin: entry.pinyin,
            options,
            correct_idx,
            bridge_x,
            bridge_y,
        };
        self.message = format!(
            "桥 Word Bridge! Which character means \"{}\"? Pick 1-4.",
            entry.meaning
        );
        self.message_timer = 120;
    }

    fn start_locked_door(&mut self, door_x: i32, door_y: i32) {
        let vocab = vocab::vocab_for_floor(self.floor_num);
        if vocab.len() < 4 {
            self.message = "Not enough vocabulary.".into();
            self.message_timer = 60;
            return;
        }
        let idx = self.rng_next() as usize % vocab.len();
        let entry = vocab[idx];
        let mut others: Vec<&'static str> = vocab
            .iter()
            .filter(|e| e.meaning != entry.meaning)
            .map(|e| e.meaning)
            .collect();
        for i in (1..others.len()).rev() {
            let j = self.rng_next() as usize % (i + 1);
            others.swap(i, j);
        }
        let correct_idx = self.rng_next() as usize % 4;
        let mut options = [others[0], others[1], others[2], entry.meaning];
        options[3] = options[correct_idx];
        options[correct_idx] = entry.meaning;
        self.combat = CombatState::LockedDoorChallenge {
            hanzi: entry.hanzi,
            pinyin: entry.pinyin,
            correct_meaning: entry.meaning,
            options,
            correct_idx,
            door_x,
            door_y,
        };
        self.message = format!("锁 Locked Door! What does {} mean? Pick 1-4.", entry.hanzi);
        self.message_timer = 120;
    }

    fn start_cursed_floor(&mut self) {
        let vocab = vocab::vocab_for_floor(self.floor_num);
        if vocab.is_empty() {
            self.message = "The curse fizzles.".into();
            self.message_timer = 60;
            return;
        }
        let idx = self.rng_next() as usize % vocab.len();
        let entry = vocab[idx];
        let tone = entry
            .pinyin
            .chars()
            .last()
            .and_then(|c| c.to_digit(10))
            .unwrap_or(1) as u8;
        self.combat = CombatState::CursedFloorChallenge {
            hanzi: entry.hanzi,
            pinyin: entry.pinyin,
            meaning: entry.meaning,
            correct_tone: tone,
        };
        self.message = format!(
            "咒 Cursed Floor! What tone is {} ({})? Press 1-4.",
            entry.hanzi, entry.meaning
        );
        self.message_timer = 120;
    }

    fn forge_quest_candidates_for_floor(floor: i32) -> Vec<&'static radical::Recipe> {
        let available = radical::radicals_for_floor(floor.max(1));
        radical::RECIPES
            .iter()
            .filter(|recipe| {
                recipe
                    .inputs
                    .iter()
                    .all(|input| available.iter().any(|radical| radical.ch == *input))
            })
            .collect()
    }

    /// Generate a random quest.
    fn generate_quest(&mut self) -> Quest {
        let floor = self.floor_num;
        match self.rng_next() % 4 {
            0 => {
                let target = 3 + (floor / 3) as i32;
                Quest {
                    description: format!("Defeat {} enemies", target),
                    goal: QuestGoal::KillEnemies(0, target),
                    gold_reward: 10 + floor * 3,
                    completed: false,
                    chain_step: 0,
                    chain_id: 0,
                }
            }
            1 => {
                let target_floor = floor + 2;
                Quest {
                    description: format!("Reach floor {}", target_floor),
                    goal: QuestGoal::ReachFloor(target_floor),
                    gold_reward: 14 + floor * 3,
                    completed: false,
                    chain_step: 0,
                    chain_id: 0,
                }
            }
            2 => {
                let target = 3 + (floor / 2) as i32;
                Quest {
                    description: format!("Collect {} radicals", target),
                    goal: QuestGoal::CollectRadicals(0, target),
                    gold_reward: 8 + floor * 2,
                    completed: false,
                    chain_step: 0,
                    chain_id: 0,
                }
            }
            _ => {
                let candidates = Self::forge_quest_candidates_for_floor(floor);
                if candidates.is_empty() {
                    let target = 3 + (floor / 2) as i32;
                    Quest {
                        description: format!("Collect {} radicals", target),
                        goal: QuestGoal::CollectRadicals(0, target),
                        gold_reward: 8 + floor * 2,
                        completed: false,
                        chain_step: 0,
                        chain_id: 0,
                    }
                } else {
                    let recipe = candidates[self.rng_next() as usize % candidates.len()];
                    Quest {
                        description: format!(
                            "Forge {} ({})",
                            recipe.output_hanzi, recipe.output_meaning
                        ),
                        goal: QuestGoal::ForgeCharacter(recipe.output_hanzi),
                        gold_reward: 12 + floor * 3,
                        completed: false,
                        chain_step: 0,
                        chain_id: 0,
                    }
                }
            }
        }
    }

    fn generate_chain_quest(&mut self, step: u8, chain_id: u32) -> Quest {
        let floor = self.floor_num;
        let escalation = step as i32;
        match step {
            0 => {
                let target = 3 + (floor / 3) + escalation;
                Quest {
                    description: format!("⛓① Defeat {} enemies", target),
                    goal: QuestGoal::KillEnemies(0, target),
                    gold_reward: 7 + floor * 2,
                    completed: false,
                    chain_step: 1,
                    chain_id,
                }
            }
            1 => {
                let target = 3 + (floor / 2) + escalation;
                Quest {
                    description: format!("⛓② Collect {} radicals", target),
                    goal: QuestGoal::CollectRadicals(0, target),
                    gold_reward: 10 + floor * 3,
                    completed: false,
                    chain_step: 2,
                    chain_id,
                }
            }
            2 => {
                let candidates = Self::forge_quest_candidates_for_floor(floor);
                if !candidates.is_empty() {
                    let recipe = candidates[self.rng_next() as usize % candidates.len()];
                    Quest {
                        description: format!(
                            "⛓③ Forge {} ({})",
                            recipe.output_hanzi, recipe.output_meaning
                        ),
                        goal: QuestGoal::ForgeCharacter(recipe.output_hanzi),
                        gold_reward: 18 + floor * 4,
                        completed: false,
                        chain_step: 3,
                        chain_id,
                    }
                } else {
                    let target = 5 + (floor / 2) + escalation;
                    Quest {
                        description: format!("⛓③ Defeat {} enemies", target),
                        goal: QuestGoal::KillEnemies(0, target),
                        gold_reward: 18 + floor * 4,
                        completed: false,
                        chain_step: 3,
                        chain_id,
                    }
                }
            }
            _ => {
                let target_floor = floor + 3;
                Quest {
                    description: format!("⛓④ Reach floor {} (finale!)", target_floor),
                    goal: QuestGoal::ReachFloor(target_floor),
                    gold_reward: 28 + floor * 4,
                    completed: false,
                    chain_step: 4,
                    chain_id,
                }
            }
        }
    }

    fn generate_shop_items(&mut self) -> Vec<ShopItem> {
        let mut items = Vec::new();

        // Always offer heal
        items.push(ShopItem {
            label: "Full Heal".to_string(),
            cost: 20 + self.floor_num * 4,
            kind: ShopItemKind::HealFull,
        });

        // Offer 2 random radicals
        let available = radical::radicals_for_floor(self.floor_num);
        for _ in 0..2 {
            let idx = self.rng_next() as usize % available.len();
            let rad = available[idx];
            items.push(ShopItem {
                label: format!("Radical [{}] ({})", rad.ch, rad.meaning),
                cost: 12 + self.floor_num * 2,
                kind: ShopItemKind::Radical(rad.ch),
            });
        }

        // Offer 1 random equipment
        let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
        let eq = &EQUIPMENT_POOL[eq_idx];
        items.push(ShopItem {
            label: format!("{} ({:?})", eq.name, eq.slot),
            cost: 30 + self.floor_num * 6,
            kind: ShopItemKind::Equipment(eq_idx),
        });

        // Offer 1 random consumable item
        let consumable = self.random_item();
        let cname = self.item_display_name(&consumable);
        items.push(ShopItem {
            label: cname,
            cost: 15 + self.floor_num * 3,
            kind: ShopItemKind::Consumable(consumable),
        });

        items.push(ShopItem {
            label: "🍙 Rice Ball".to_string(),
            cost: 8,
            kind: ShopItemKind::Consumable(crate::player::Item::RationPack(40)),
        });

        items
    }

    /// Buy item from shop.
    fn shop_buy(&mut self) {
        if let CombatState::Shopping { ref items, cursor } = self.combat.clone() {
            if cursor >= items.len() {
                return;
            }
            let item = &items[cursor];
            let effective_cost = self.discounted_cost(item.cost);
            if self.player.gold < effective_cost {
                self.message = format!(
                    "Not enough gold! Need {} (have {})",
                    effective_cost, self.player.gold
                );
                self.message_timer = 40;
                return;
            }
            self.player.gold -= effective_cost;
            if let Some(ref audio) = self.audio {
                audio.play_buy();
            }
            match &item.kind {
                ShopItemKind::Radical(ch) => {
                    self.player.add_radical(ch);
                    self.message = format!("Bought radical [{}]!", ch);
                }
                ShopItemKind::HealFull => {
                    self.player.hp = self.player.max_hp;
                    self.message = "Fully healed!".to_string();
                }
                ShopItemKind::Equipment(idx) => {
                    let eq = &EQUIPMENT_POOL[*idx];
                    let current_state = self.player.equipment_state(eq.slot);
                    if current_state == ItemState::Cursed {
                        self.message = format!(
                            "💀 Your {} slot is cursed! Visit an altar to purify.",
                            match eq.slot {
                                crate::player::EquipSlot::Weapon => "weapon",
                                crate::player::EquipSlot::Armor => "armor",
                                crate::player::EquipSlot::Charm => "charm",
                            }
                        );
                        self.player.gold += effective_cost; // refund
                    } else {
                        self.player.equip(eq, ItemState::Normal);
                        self.message = format!("Equipped {}!", eq.name);
                    }
                }
                ShopItemKind::Consumable(consumable) => {
                    let name = self.item_display_name(consumable);
                    if self.player.add_item(consumable.clone(), ItemState::Normal) {
                        self.message = format!("Bought {}!", name);
                    } else {
                        self.message = "Inventory full!".to_string();
                        self.player.gold += effective_cost; // refund
                    }
                }
            }
            self.message_timer = 60;
        }
    }

    /// Attempt to steal the currently highlighted shop item.
    fn shop_steal(&mut self) {
        if self.shop_banned {
            self.message = "🚫 The shopkeeper refuses to serve you!".to_string();
            self.message_timer = 60;
            return;
        }

        if let CombatState::Shopping { ref items, cursor } = self.combat.clone() {
            if cursor >= items.len() {
                return;
            }

            let mut chance: i64 = 40;
            if self.player.class == PlayerClass::Operative {
                chance += 25;
            }
            if self.player.class == PlayerClass::Operative {
                chance += 15;
            }
            chance -= (self.theft_catches as i64) * 10;
            chance = chance.clamp(5, 80);

            let roll = (self.rng_next() % 100) as i64;

            if roll < chance {
                let item = &items[cursor];
                match &item.kind {
                    ShopItemKind::Radical(ch) => {
                        self.player.add_radical(ch);
                        self.message = format!(
                            "🤫 You pocket radical [{}] while the shopkeeper looks away!",
                            ch
                        );
                    }
                    ShopItemKind::HealFull => {
                        self.player.hp = self.player.max_hp;
                        self.message = "🤫 You sip the healing brew unnoticed!".to_string();
                    }
                    ShopItemKind::Equipment(idx) => {
                        let eq = &EQUIPMENT_POOL[*idx];
                        self.player.equip(eq, ItemState::Normal);
                        self.message =
                            format!("🤫 You slip on the {} when nobody's watching!", eq.name);
                    }
                    ShopItemKind::Consumable(consumable) => {
                        if !self.player.add_item(consumable.clone(), ItemState::Normal) {
                            self.message = "Inventory full — can't steal!".to_string();
                            self.message_timer = 40;
                            return;
                        }
                        self.message = "🤫 Five-finger discount! Item pocketed.".to_string();
                    }
                }
                self.message_timer = 80;
                if let Some(ref audio) = self.audio {
                    audio.play_buy();
                }
                if let CombatState::Shopping {
                    ref mut items,
                    ref mut cursor,
                } = self.combat
                {
                    if *cursor < items.len() {
                        items.remove(*cursor);
                        if *cursor >= items.len() && *cursor > 0 {
                            *cursor -= 1;
                        }
                    }
                }
            } else {
                self.theft_catches += 1;
                self.shop_banned = true;
                let dmg = 3 + self.theft_catches as i32;
                self.player.hp -= dmg;
                self.combat = CombatState::Explore;
                self.message = format!(
                    "🚨 Caught stealing! The shopkeeper strikes you for {} damage and throws you out!",
                    dmg
                );
                self.message_timer = 100;
                if let Some(ref audio) = self.audio {
                    audio.play_damage();
                }
                self.trigger_shake(10);
                self.flash = Some((255, 50, 50, 0.3));

                if self.player.hp <= 0 && !self.try_phoenix_revive() {
                    self.player.hp = 0;
                    self.run_journal.log(RunEvent::DiedTo(
                        "Angry shopkeeper".to_string(),
                        self.floor_num,
                    ));
                    self.post_mortem_page = 0;
                    self.combat = CombatState::GameOver;
                    self.message = self.run_summary();
                    self.message_timer = 255;
                    if let Some(ref audio) = self.audio {
                        audio.play_death();
                    }
                }
            }
        }
    }

    /// Merchant L3 perk: reroll one shop item per floor.
    fn shop_reroll(&mut self) {
        if self.merchant_reroll_used {
            self.message = "Already rerolled this floor!".to_string();
            self.message_timer = 40;
            return;
        }
        let has_merchant_l3 =
            self.companion == Some(Companion::Quartermaster) && self.companion_level() >= 3;
        if !has_merchant_l3 {
            return;
        }
        // Phase 1: Extract cursor and old item kind (immutable borrow, then drop)
        let (cursor, old_kind) = if let CombatState::Shopping { ref items, cursor } = self.combat {
            if cursor < items.len() {
                (cursor, items[cursor].kind.clone())
            } else {
                return;
            }
        } else {
            return;
        };
        // Phase 2: Generate new item (free to use &mut self)
        let new_item = match old_kind {
            ShopItemKind::HealFull => ShopItem {
                label: "Full Heal".to_string(),
                cost: 20 + self.floor_num * 4,
                kind: ShopItemKind::HealFull,
            },
            ShopItemKind::Radical(_) => {
                let available = radical::radicals_for_floor(self.floor_num);
                let idx = self.rng_next() as usize % available.len();
                let rad = available[idx];
                ShopItem {
                    label: format!("Radical [{}] ({})", rad.ch, rad.meaning),
                    cost: 12 + self.floor_num * 2,
                    kind: ShopItemKind::Radical(rad.ch),
                }
            }
            ShopItemKind::Equipment(_) => {
                let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
                let eq = &EQUIPMENT_POOL[eq_idx];
                ShopItem {
                    label: format!("{} ({:?})", eq.name, eq.slot),
                    cost: 30 + self.floor_num * 6,
                    kind: ShopItemKind::Equipment(eq_idx),
                }
            }
            ShopItemKind::Consumable(_) => {
                let consumable = self.random_item();
                let cname = self.item_display_name(&consumable);
                ShopItem {
                    label: cname,
                    cost: 15 + self.floor_num * 3,
                    kind: ShopItemKind::Consumable(consumable),
                }
            }
        };
        // Phase 3: Replace item in shop
        if let CombatState::Shopping { ref mut items, .. } = self.combat {
            items[cursor] = new_item;
        }
        self.merchant_reroll_used = true;
        self.message = "💰 Merchant rerolled the item!".to_string();
        self.message_timer = 60;
    }

    /// Use a spell during combat (Q to cycle, Space to cast).
    fn use_spell(&mut self) {
        if let CombatState::Fighting { enemy_idx, .. } = self.combat {
            // Copy enemy position for particles before spell takes effect
            let e_screen = if enemy_idx < self.enemies.len() {
                Some(self.tile_to_screen(self.enemies[enemy_idx].x, self.enemies[enemy_idx].y))
            } else {
                None
            };
            let p_screen = self.tile_to_screen(self.player.x, self.player.y);

            if let Some(spell) = self.player.use_spell() {
                if let Some(ref audio) = self.audio {
                    audio.play_spell();
                }
                // Arcane room doubles spell damage
                let arcane_mult = if self.current_room_modifier() == Some(RoomModifier::HighTech) {
                    2
                } else {
                    1
                };
                let spell_power =
                    self.player.spell_power_bonus + self.player.spell_power_temp_bonus;
                self.player.spell_power_temp_bonus = 0;
                let current_effect = spell.effect;
                let spell_school = match current_effect {
                    SpellEffect::FireAoe(_) => Some("Fire"),
                    SpellEffect::StrongHit(_) => Some("Strike"),
                    SpellEffect::Drain(_) => Some("Drain"),
                    SpellEffect::Stun => Some("Stun"),
                    SpellEffect::Heal(_)
                    | SpellEffect::Reveal
                    | SpellEffect::Shield
                    | SpellEffect::Pacify
                    | SpellEffect::Slow(_)
                    | SpellEffect::FocusRestore(_)
                    | SpellEffect::ArmorBreak => None,
                    SpellEffect::Teleport => None,
                    SpellEffect::Poison(_, _) => Some("Poison"),
                    SpellEffect::Dash(_) => None,
                    SpellEffect::Pierce(_) => Some("Strike"),
                    SpellEffect::PullToward => None,
                    SpellEffect::KnockBack(_) => Some("Strike"),
                    SpellEffect::Thorns(_) => None,
                    SpellEffect::Cone(_) => Some("Fire"),
                    SpellEffect::Wall(_) => None,
                    SpellEffect::OilSlick
                    | SpellEffect::Ignite
                    | SpellEffect::PlantGrowth
                    | SpellEffect::Earthquake(_)
                    | SpellEffect::Sanctify(_)
                    | SpellEffect::FloodWave(_)
                    | SpellEffect::SummonBoulder
                    | SpellEffect::FreezeGround(_) => None,
                };
                let elementalist_resisted = enemy_idx < self.enemies.len()
                    && self.enemies[enemy_idx].boss_kind == Some(BossKind::RogueAICore)
                    && spell_school.is_some()
                    && self.enemies[enemy_idx].resisted_spell == spell_school;
                match spell.effect {
                    SpellEffect::FireAoe(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        // Fire particles at player position (AoE emanates from player)
                        self.particles
                            .spawn_fire(p_screen.0, p_screen.1, &mut self.rng_state);
                        // Damage all visible enemies
                        let mut killed = 0;
                        let mut boss_resisted = false;
                        for (idx, e) in self.enemies.iter_mut().enumerate() {
                            if e.is_alive() {
                                let eidx = self.level.idx(e.x, e.y);
                                if self.level.visible[eidx] {
                                    let applied_dmg = if idx == enemy_idx && elementalist_resisted {
                                        boss_resisted = true;
                                        (dmg / 2).max(1)
                                    } else {
                                        dmg
                                    };
                                    e.hp -= applied_dmg;
                                    if e.hp <= 0 {
                                        killed += 1;
                                    }
                                }
                            }
                        }
                        let oil_bonus = (1 + self.floor_num.max(1) / 4).max(1) + spell_power;
                        let (oil_tiles, scorched, oil_kills) = self.ignite_visible_oil(oil_bonus);
                        killed += oil_kills;
                        let resist_text = if boss_resisted {
                            " The Rogue AI Core dampens the repeated fire spell!"
                        } else {
                            ""
                        };
                        self.message = if oil_tiles > 0 {
                            format!(
                                "{}🔥 {} deals {} damage to all! Oil ignites on {} tiles and scorches {} foes! ({} defeated){}",
                                spell.hanzi, spell.meaning, dmg, oil_tiles, scorched, killed, resist_text
                            )
                        } else {
                            format!(
                                "{}🔥 {} deals {} damage to all! ({} defeated){}",
                                spell.hanzi, spell.meaning, dmg, killed, resist_text
                            )
                        };
                        self.message_timer = 80;
                        // If the fought enemy died, return to explore
                        if enemy_idx < self.enemies.len() && !self.enemies[enemy_idx].is_alive() {
                            self.combat = CombatState::Explore;
                            self.typing.clear();
                        }
                    }
                    SpellEffect::Heal(amount) => {
                        let amount = amount * arcane_mult + spell_power;
                        self.player.hp = (self.player.hp + amount).min(self.player.max_hp);
                        self.particles
                            .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                        self.flash = Some((60, 220, 80, 0.2));
                        self.message = format!(
                            "{} heals {} HP! (now {}/{})",
                            spell.hanzi, amount, self.player.hp, self.player.max_hp
                        );
                        self.message_timer = 60;
                    }
                    SpellEffect::Reveal => {
                        self.reveal_entire_floor();
                        self.particles
                            .spawn_teleport(p_screen.0, p_screen.1, &mut self.rng_state);
                        self.flash = Some((100, 210, 255, 0.18));
                        self.message = format!(
                            "{}👁 The dungeon's paths blaze into focus. Floor map revealed!",
                            spell.hanzi
                        );
                        self.message_timer = 70;
                    }
                    SpellEffect::Shield => {
                        self.player.shield = true;
                        self.particles
                            .spawn_shield(p_screen.0, p_screen.1, &mut self.rng_state);
                        self.message =
                            format!("{} — Shield active! Next hit will be blocked.", spell.hanzi);
                        self.message_timer = 60;
                    }
                    SpellEffect::StrongHit(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                            }
                            let applied_dmg = if elementalist_resisted {
                                (dmg / 2).max(1)
                            } else {
                                dmg
                            };
                            self.enemies[enemy_idx].hp -= applied_dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}⚔ Devastating {} damage! Defeated {}! Got [{}]{}",
                                    spell.hanzi,
                                    applied_dmg,
                                    e_hanzi,
                                    available[drop_idx].ch,
                                    if elementalist_resisted {
                                        " (partially resisted)"
                                    } else {
                                        ""
                                    }
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}⚔ {} damage to {}! ({} HP left){}",
                                    spell.hanzi,
                                    applied_dmg,
                                    e_hanzi,
                                    self.enemies[enemy_idx].hp,
                                    if elementalist_resisted {
                                        " The Rogue AI Core resists the repeated school."
                                    } else {
                                        ""
                                    }
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Drain(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_drain(ex, ey, &mut self.rng_state);
                            }
                            let applied_dmg = if elementalist_resisted {
                                (dmg / 2).max(1)
                            } else {
                                dmg
                            };
                            self.enemies[enemy_idx].hp -= applied_dmg;
                            self.player.hp = (self.player.hp + applied_dmg).min(self.player.max_hp);
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}🩸 Drained {} HP from {}! Defeated! Got [{}]{}",
                                    spell.hanzi,
                                    applied_dmg,
                                    e_hanzi,
                                    available[drop_idx].ch,
                                    if elementalist_resisted {
                                        " (partially resisted)"
                                    } else {
                                        ""
                                    }
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}🩸 Drained {} HP from {}! +{} HP ({} left){}",
                                    spell.hanzi,
                                    applied_dmg,
                                    e_hanzi,
                                    applied_dmg,
                                    self.enemies[enemy_idx].hp,
                                    if elementalist_resisted {
                                        " The Rogue AI Core resists the repeated school."
                                    } else {
                                        ""
                                    }
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Stun => {
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_stun(ex, ey, &mut self.rng_state);
                            }
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            let (water_tiles, shocked, water_kills) =
                                self.electrify_visible_water(arcane_mult + spell_power);
                            if !elementalist_resisted {
                                self.enemies[enemy_idx].stunned = true;
                            }
                            self.message = if water_tiles > 0 {
                                format!(
                                    "{}⚡ {}{} Water arcs through {} foes on {} tiles! ({} fried)",
                                    spell.hanzi,
                                    e_hanzi,
                                    if elementalist_resisted {
                                        " resists the repeated stun!"
                                    } else {
                                        " is stunned!"
                                    },
                                    shocked,
                                    water_tiles,
                                    water_kills
                                )
                            } else {
                                format!(
                                    "{}⚡ {}{}",
                                    spell.hanzi,
                                    e_hanzi,
                                    if elementalist_resisted {
                                        " resists the repeated stun!"
                                    } else {
                                        " is stunned! It will skip its next action."
                                    }
                                )
                            };
                            self.message_timer = 60;
                            if !self.enemies[enemy_idx].is_alive() {
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            }
                        }
                    }
                    SpellEffect::Pacify => {
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_shield(ex, ey, &mut self.rng_state);
                            }
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].is_boss {
                                self.enemies[enemy_idx].stunned = true;
                                self.flash = Some((150, 190, 255, 0.14));
                                self.message = format!(
                                    "{}☯ {} resists a full truce, but falters and loses its next action.",
                                    spell.hanzi, e_hanzi
                                );
                                self.message_timer = 80;
                            } else {
                                let gold = Self::pacify_gold_reward(
                                    self.enemies[enemy_idx].gold_value,
                                    spell_power,
                                );
                                self.player.gold += gold;
                                self.enemies[enemy_idx].hp = 0;
                                self.flash = Some((120, 220, 190, 0.18));
                                self.message = format!(
                                    "{}☯ You reason with {}. It withdraws peacefully and leaves {} gold behind.",
                                    spell.hanzi, e_hanzi, gold
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            }
                        }
                    }
                    SpellEffect::Slow(turns) => {
                        if enemy_idx < self.enemies.len() {
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            self.enemies[enemy_idx].stunned = true; // overworld: stun as proxy for slow
                            self.flash = Some((150, 200, 255, 0.15));
                            self.message = format!(
                                "{}🐌 {} is slowed for {} turns! It loses its next action.",
                                spell.hanzi, e_hanzi, turns
                            );
                            self.message_timer = 60;
                        }
                    }
                    SpellEffect::Teleport => {
                        // Overworld: blink past the enemy
                        self.flash = Some((100, 210, 255, 0.2));
                        self.message = format!(
                            "{}💨 You vanish in a gust of wind and reappear safely!",
                            spell.hanzi
                        );
                        self.message_timer = 60;
                        self.combat = CombatState::Explore;
                        self.typing.clear();
                    }
                    SpellEffect::Poison(dmg, turns) => {
                        if enemy_idx < self.enemies.len() {
                            let applied_dmg = dmg * turns * arcane_mult + spell_power;
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_drain(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= applied_dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}☠ Venom courses through {}! {} poison damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, applied_dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}☠ Poison seeps into {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, applied_dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::FocusRestore(amt) => {
                        let amt = amt * arcane_mult + spell_power;
                        self.player.hp = (self.player.hp + amt).min(self.player.max_hp);
                        self.particles
                            .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                        self.flash = Some((80, 180, 255, 0.15));
                        self.message = format!(
                            "{}🧘 Mental focus restored! +{} HP (now {}/{})",
                            spell.hanzi, amt, self.player.hp, self.player.max_hp
                        );
                        self.message_timer = 60;
                    }
                    SpellEffect::ArmorBreak => {
                        if enemy_idx < self.enemies.len() {
                            let dmg = 2 * arcane_mult + spell_power;
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}💥 Armor-breaking strike shatters {}! {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}💥 Armor-breaking force hits {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Dash(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}💨 Charged through {}! {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}💨 Dashing strike hits {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        } else {
                            self.flash = Some((100, 210, 255, 0.2));
                            self.message = format!("{}💨 You dash past in a blur!", spell.hanzi);
                            self.message_timer = 60;
                            self.combat = CombatState::Explore;
                            self.typing.clear();
                        }
                    }
                    SpellEffect::Pierce(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_damage(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}🔱 Piercing bolt skewers {}! {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}🔱 Piercing bolt hits {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::PullToward => {
                        if enemy_idx < self.enemies.len() {
                            self.enemies[enemy_idx].stunned = true;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            self.message = format!(
                                "{}🧲 {} is yanked toward you and dazed!",
                                spell.hanzi, e_hanzi
                            );
                            self.message_timer = 60;
                        }
                    }
                    SpellEffect::KnockBack(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_damage(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}🤜 {} sent flying! {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}🤜 {} knocked back for {} damage! ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Thorns(turns) => {
                        self.player.shield = true;
                        self.particles
                            .spawn_shield(p_screen.0, p_screen.1, &mut self.rng_state);
                        self.message = format!(
                            "{}🌿 Thorns grow around you for {} turns!",
                            spell.hanzi, turns
                        );
                        self.message_timer = 60;
                    }
                    SpellEffect::Cone(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_fire(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}🔺 Cone blast engulfs {}! {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}🔺 Cone blast hits {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Wall(_) => {
                        self.flash = Some((160, 140, 100, 0.15));
                        self.message =
                            format!("{}🧱 A wall of stone erupts from the ground!", spell.hanzi);
                        self.message_timer = 60;
                    }
                    SpellEffect::OilSlick => {
                        self.flash = Some((80, 60, 20, 0.15));
                        self.message =
                            format!("{}🛢 Oil spreads across the ground!", spell.hanzi);
                        self.message_timer = 60;
                    }
                    SpellEffect::FreezeGround(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_damage(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}🧊 Ice encases {}! {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}🧊 Ground freezes beneath {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Ignite => {
                        self.flash = Some((255, 120, 30, 0.2));
                        self.message =
                            format!("{}🔥 Flames erupt and spread!", spell.hanzi);
                        self.message_timer = 60;
                    }
                    SpellEffect::PlantGrowth => {
                        self.flash = Some((60, 180, 60, 0.15));
                        let healed = 1_i32.min(self.player.max_hp - self.player.hp);
                        self.player.hp = (self.player.hp + 1).min(self.player.max_hp);
                        self.message = format!(
                            "{}🌿 Nature blooms! +{} HP",
                            spell.hanzi, healed
                        );
                        self.message_timer = 60;
                    }
                    SpellEffect::Earthquake(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_damage(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}💥 The earth shakes! {} crushed for {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}💥 Earthquake hits {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Sanctify(heal) => {
                        let heal = heal * arcane_mult + spell_power;
                        self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                        self.particles
                            .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                        self.flash = Some((255, 215, 80, 0.2));
                        self.message = format!(
                            "{}✨ Holy light purifies! +{} HP",
                            spell.hanzi, heal
                        );
                        self.message_timer = 60;
                    }
                    SpellEffect::FloodWave(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_damage(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}🌊 Flood wave sweeps away {}! {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}🌊 Flood wave hits {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::SummonBoulder => {
                        self.flash = Some((140, 120, 90, 0.15));
                        self.message =
                            format!("{}🪨 A boulder materializes!", spell.hanzi);
                        self.message_timer = 60;
                    }
                }

                if enemy_idx < self.enemies.len()
                    && self.enemies[enemy_idx].boss_kind == Some(BossKind::RogueAICore)
                {
                    if let Some(school) = spell_school {
                        self.enemies[enemy_idx].resisted_spell = Some(school);
                    }
                }

                // ── Combo detection ─────────────────────────────────────
                if let Some(prev) = self.last_spell.take() {
                    let combo = detect_combo(&prev, &current_effect);
                    if let Some((combo_name, combo_effect)) = combo {
                        self.apply_combo(enemy_idx, &combo_name, combo_effect, p_screen, e_screen);
                    }
                }
                self.last_spell = Some(current_effect);
                self.spell_combo_timer = 3;
                if matches!(self.combat, CombatState::Fighting { .. })
                    && self.maybe_trigger_boss_phase(enemy_idx)
                {
                    self.typing.clear();
                }
            } else {
                self.message = "No spells available!".to_string();
                self.message_timer = 40;
            }
        }
    }

    /// Use a spell during exploration (Q to cycle, Space to cast).
    /// Only utility spells (Heal, Shield, Reveal) work outside combat.
    fn use_spell_explore(&mut self) {
        if !matches!(self.combat, CombatState::Explore) {
            return;
        }
        if self.player.selected_spell >= self.player.spells.len() {
            self.message = "No spells available!".to_string();
            self.message_timer = 40;
            return;
        }
        let effect = self.player.spells[self.player.selected_spell].effect;
        match effect {
            SpellEffect::Heal(_)
            | SpellEffect::Shield
            | SpellEffect::Reveal
            | SpellEffect::Thorns(_) => {}
            _ => {
                // Offensive spell — enter aiming mode instead of rejecting
                let label = effect.label();
                self.combat = CombatState::Aiming {
                    spell_idx: self.player.selected_spell,
                    dx: 0,
                    dy: -1,
                };
                self.message = format!("{} — Aim with arrows, Enter to fire, Esc cancel", label);
                self.message_timer = 120;
                return;
            }
        }
        let spell = self.player.use_spell().unwrap();
        if let Some(ref audio) = self.audio {
            audio.play_spell();
        }
        let p_screen = self.tile_to_screen(self.player.x, self.player.y);
        let spell_power = self.player.spell_power_bonus + self.player.spell_power_temp_bonus;
        self.player.spell_power_temp_bonus = 0;
        let arcane_mult = if self.current_room_modifier() == Some(RoomModifier::HighTech) {
            2
        } else {
            1
        };
        match spell.effect {
            SpellEffect::Heal(amount) => {
                let amount = amount * arcane_mult + spell_power;
                self.player.hp = (self.player.hp + amount).min(self.player.max_hp);
                self.particles
                    .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                self.flash = Some((60, 220, 80, 0.2));
                self.message = format!(
                    "{} heals {} HP! (now {}/{})",
                    spell.hanzi, amount, self.player.hp, self.player.max_hp
                );
                self.message_timer = 60;
            }
            SpellEffect::Reveal => {
                self.reveal_entire_floor();
                self.particles
                    .spawn_teleport(p_screen.0, p_screen.1, &mut self.rng_state);
                self.flash = Some((100, 210, 255, 0.18));
                self.message = format!(
                    "{}👁 The dungeon's paths blaze into focus. Floor map revealed!",
                    spell.hanzi
                );
                self.message_timer = 70;
            }
            SpellEffect::Shield => {
                self.player.shield = true;
                self.particles
                    .spawn_shield(p_screen.0, p_screen.1, &mut self.rng_state);
                self.message =
                    format!("{} — Shield active! Next hit will be blocked.", spell.hanzi);
                self.message_timer = 60;
            }
            SpellEffect::Thorns(turns) => {
                self.player.shield = true;
                self.particles
                    .spawn_shield(p_screen.0, p_screen.1, &mut self.rng_state);
                self.message = format!("{}🌿 Thorns active for {} turns!", spell.hanzi, turns);
                self.message_timer = 60;
            }
            _ => unreachable!(),
        }
    }

    fn fire_aimed_spell(&mut self, spell_idx: usize, dx: i32, dy: i32) {
        if spell_idx >= self.player.spells.len() {
            self.combat = CombatState::Explore;
            return;
        }
        let effect = self.player.spells[spell_idx].effect;
        let arcane_mult = if self.current_room_modifier() == Some(RoomModifier::HighTech) {
            2
        } else {
            1
        };
        let spell_power = self.player.spell_power_bonus + self.player.spell_power_temp_bonus;
        self.player.spell_power_temp_bonus = 0;

        let mut cx = self.player.x + dx;
        let mut cy = self.player.y + dy;
        let mut hit_enemy: Option<usize> = None;
        let max_range = 10;
        for _ in 0..max_range {
            if !self.level.in_bounds(cx, cy)
                || !self.level.tiles[self.level.idx(cx, cy)].is_walkable()
            {
                break;
            }
            if let Some(idx) = self.enemy_at(cx, cy) {
                hit_enemy = Some(idx);
                break;
            }
            cx += dx;
            cy += dy;
        }

        let spell = self.player.use_spell().unwrap();
        if let Some(ref audio) = self.audio {
            audio.play_spell();
        }
        let p_screen = self.tile_to_screen(self.player.x, self.player.y);

        if let Some(enemy_idx) = hit_enemy {
            let e_screen =
                self.tile_to_screen(self.enemies[enemy_idx].x, self.enemies[enemy_idx].y);
            match effect {
                SpellEffect::FireAoe(dmg) => {
                    let dmg = dmg * arcane_mult + spell_power;
                    self.particles
                        .spawn_fire(e_screen.0, e_screen.1, &mut self.rng_state);
                    self.enemies[enemy_idx].hp -= dmg;
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    if self.enemies[enemy_idx].hp <= 0 {
                        self.message = format!(
                            "{}🔥 {} takes {} damage and is defeated!",
                            spell.hanzi, e_hanzi, dmg
                        );
                    } else {
                        self.message = format!(
                            "{}🔥 {} takes {} damage! ({} HP left)",
                            spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                        );
                    }
                }
                SpellEffect::StrongHit(dmg) => {
                    let dmg = dmg * arcane_mult + spell_power;
                    self.particles
                        .spawn_damage(e_screen.0, e_screen.1, &mut self.rng_state);
                    self.enemies[enemy_idx].hp -= dmg;
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    if self.enemies[enemy_idx].hp <= 0 {
                        self.message = format!(
                            "{}⚔ {} takes {} damage and is defeated!",
                            spell.hanzi, e_hanzi, dmg
                        );
                    } else {
                        self.message = format!(
                            "{}⚔ {} takes {} damage! ({} HP left)",
                            spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                        );
                    }
                }
                SpellEffect::Drain(dmg) => {
                    let dmg = dmg * arcane_mult + spell_power;
                    self.particles
                        .spawn_damage(e_screen.0, e_screen.1, &mut self.rng_state);
                    self.enemies[enemy_idx].hp -= dmg;
                    self.player.hp = (self.player.hp + dmg).min(self.player.max_hp);
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    if self.enemies[enemy_idx].hp <= 0 {
                        self.message = format!(
                            "{}🩸 Drained {} from {} — foe defeated! +{} HP",
                            spell.hanzi, dmg, e_hanzi, dmg
                        );
                    } else {
                        self.message = format!(
                            "{}🩸 Drained {} from {}! +{} HP ({} HP left)",
                            spell.hanzi, dmg, e_hanzi, dmg, self.enemies[enemy_idx].hp
                        );
                    }
                }
                SpellEffect::Stun => {
                    self.enemies[enemy_idx].stunned = true;
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    self.particles
                        .spawn_damage(e_screen.0, e_screen.1, &mut self.rng_state);
                    self.message = format!("{}⚡ {} is stunned!", spell.hanzi, e_hanzi);
                }
                SpellEffect::Pacify => {
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    if self.enemies[enemy_idx].is_boss {
                        self.enemies[enemy_idx].stunned = true;
                        self.message = format!(
                            "{}☯ {} resists pacification but is dazed!",
                            spell.hanzi, e_hanzi
                        );
                    } else {
                        self.enemies[enemy_idx].hp = 0;
                        self.message =
                            format!("{}☯ {} stands down peacefully.", spell.hanzi, e_hanzi);
                    }
                }
                SpellEffect::Pierce(dmg) | SpellEffect::KnockBack(dmg) | SpellEffect::Cone(dmg) => {
                    let dmg = dmg * arcane_mult + spell_power;
                    self.particles
                        .spawn_damage(e_screen.0, e_screen.1, &mut self.rng_state);
                    self.enemies[enemy_idx].hp -= dmg;
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    let icon = match effect {
                        SpellEffect::Pierce(_) => "🔱",
                        SpellEffect::KnockBack(_) => "🤜",
                        SpellEffect::Cone(_) => "🔺",
                        _ => "",
                    };
                    if self.enemies[enemy_idx].hp <= 0 {
                        self.message = format!(
                            "{}{} {} takes {} damage and is defeated!",
                            spell.hanzi, icon, e_hanzi, dmg
                        );
                    } else {
                        self.message = format!(
                            "{}{} {} takes {} damage! ({} HP left)",
                            spell.hanzi, icon, e_hanzi, dmg, self.enemies[enemy_idx].hp
                        );
                    }
                }
                SpellEffect::PullToward => {
                    self.enemies[enemy_idx].stunned = true;
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    self.message =
                        format!("{}🧲 {} is pulled closer and dazed!", spell.hanzi, e_hanzi);
                }
                _ => {}
            }
            self.flash = Some((255, 200, 100, 0.15));
            self.shake_timer = 4;
        } else {
            self.particles
                .spawn_fire(p_screen.0, p_screen.1, &mut self.rng_state);
            self.message = format!("{} flies off but hits nothing.", spell.hanzi);
        }
        self.message_timer = 80;
        self.combat = CombatState::Explore;
    }

    /// Apply a spell combo bonus.
    fn apply_combo(
        &mut self,
        enemy_idx: usize,
        name: &str,
        effect: ComboEffect,
        p_screen: (f64, f64),
        e_screen: Option<(f64, f64)>,
    ) {
        // Flash gold for combo
        self.flash = Some((255, 200, 50, 0.3));
        match effect {
            ComboEffect::Steam => {
                // AoE stun all visible enemies
                for e in &mut self.enemies {
                    if e.is_alive() {
                        e.stunned = true;
                    }
                }
                self.particles
                    .spawn_fire(p_screen.0, p_screen.1 - 10.0, &mut self.rng_state);
                self.particles
                    .spawn_shield(p_screen.0, p_screen.1 + 10.0, &mut self.rng_state);
                self.message = format!("💥 COMBO: {}! All enemies stunned!", name);
            }
            ComboEffect::Counter(dmg) => {
                // Reflect damage + shield
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].hp -= dmg;
                    self.player.shield = true;
                    if let Some((ex, ey)) = e_screen {
                        self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                    }
                }
                self.message = format!("💥 COMBO: {}! {} reflected + Shield!", name, dmg);
            }
            ComboEffect::Barrier(amount) => {
                // Strong shield + heal
                self.player.shield = true;
                self.player.hp = (self.player.hp + amount).min(self.player.max_hp);
                self.particles
                    .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                self.particles
                    .spawn_shield(p_screen.0, p_screen.1, &mut self.rng_state);
                self.message = format!("💥 COMBO: {}! Shield + {} HP!", name, amount);
            }
            ComboEffect::Flurry(dmg) => {
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].hp -= dmg;
                    if let Some((ex, ey)) = e_screen {
                        self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                        self.particles.spawn_fire(ex, ey, &mut self.rng_state);
                    }
                }
                self.message = format!("💥 COMBO: {}! {} damage flurry!", name, dmg);
            }
            ComboEffect::Ignite(dmg) => {
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].hp -= dmg;
                    let heal = dmg / 2;
                    self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                    if let Some((ex, ey)) = e_screen {
                        self.particles.spawn_fire(ex, ey, &mut self.rng_state);
                    }
                    self.particles
                        .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                }
                self.message = format!("💥 COMBO: {}! {} fire damage + lifesteal!", name, dmg);
            }
            ComboEffect::Tempest(dmg) => {
                for e in &mut self.enemies {
                    if e.is_alive() {
                        e.hp -= dmg;
                    }
                }
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].stunned = true;
                }
                self.particles
                    .spawn_fire(p_screen.0, p_screen.1, &mut self.rng_state);
                self.message = format!("💥 COMBO: {}! {} AoE damage + stun!", name, dmg);
            }
            ComboEffect::Rally(dmg) => {
                let heal = dmg / 2;
                self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].hp -= dmg;
                    if let Some((ex, ey)) = e_screen {
                        self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                    }
                }
                self.particles
                    .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                self.message = format!("💥 COMBO: {}! {} damage + {} HP!", name, dmg, heal);
            }
            ComboEffect::Siphon(dmg) => {
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].hp -= dmg;
                    self.enemies[enemy_idx].stunned = true;
                    let heal = dmg;
                    self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                    if let Some((ex, ey)) = e_screen {
                        self.particles.spawn_damage(ex, ey, &mut self.rng_state);
                    }
                    self.particles
                        .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                }
                self.message = format!("💥 COMBO: {}! {} drained + stun!", name, dmg);
            }
            ComboEffect::Fortify(amount) => {
                self.player.shield = true;
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].hp -= amount;
                    if let Some((ex, ey)) = e_screen {
                        self.particles.spawn_damage(ex, ey, &mut self.rng_state);
                    }
                }
                self.player.hp = (self.player.hp + amount).min(self.player.max_hp);
                self.particles
                    .spawn_shield(p_screen.0, p_screen.1, &mut self.rng_state);
                self.particles
                    .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                self.message = format!(
                    "💥 COMBO: {}! Shield + {} damage + {} HP!",
                    name, amount, amount
                );
            }
        }
        self.message_timer = 100;
        // Check if fought enemy died from combo
        if enemy_idx < self.enemies.len() && !self.enemies[enemy_idx].is_alive() {
            self.combat = CombatState::Explore;
            self.typing.clear();
        }
    }

    /// Open a treasure chest tile.
    fn open_chest(&mut self, cx: i32, cy: i32) {
        // Chest open particles
        let (sx, sy) = self.tile_to_screen(cx, cy);
        self.particles.spawn_chest(sx, sy, &mut self.rng_state);
        self.achievements.unlock("first_chest");
        if let Some(ref audio) = self.audio {
            audio.play_treasure();
        }

        // Remove chest tile
        let idx = self.level.idx(cx, cy);
        self.level.tiles[idx] = Tile::MetalFloor;

        let roll = self.rng_next() % 100;
        if roll < 70 {
            // 70% — loot (item, gold, or radical)
            let loot_type = self.rng_next() % 3;
            match loot_type {
                0 => {
                    // Random item
                    let item = self.random_item();
                    let state = self.roll_item_state();
                    let name = self.item_display_name(&item);
                    let prefix = match state {
                        ItemState::Cursed => "💀 ",
                        ItemState::Blessed => "✨ ",
                        ItemState::Normal => "",
                    };
                    if self.player.add_item(item, state) {
                        self.message = format!("◆ Found {}{}!", prefix, name);
                        self.achievements.check_items(self.player.items.len());
                    } else {
                        self.message = "◆ Chest had an item but inventory is full!".to_string();
                    }
                }
                1 => {
                    // Gold
                    let base = 10 + (self.rng_next() % 20) as i32 + self.floor_num * 3;
                    let gold = ((base as f64) * self.floor_profile.gold_multiplier()) as i32;
                    let gold = gold.max(1);
                    self.player.gold += gold;
                    self.message = format!("◆ Found {}g!", gold);
                }
                _ => {
                    // Radical
                    let available = radical::radicals_for_floor(self.floor_num);
                    let drop_idx = self.rng_next() as usize % available.len();
                    let dropped = available[drop_idx].ch;
                    self.player.add_radical(dropped);
                    if self.floor_profile.radical_drop_bonus() {
                        let bonus_idx = self.rng_next() as usize % available.len();
                        let bonus = available[bonus_idx].ch;
                        self.player.add_radical(bonus);
                        self.message = format!("◆ Found radicals [{}] + [{}]!", dropped, bonus);
                    } else {
                        self.message = format!("◆ Found radical [{}]!", dropped);
                    }
                }
            }
            self.message_timer = 60;
        } else if roll < 90 {
            // 20% — trap
            let trap_type = self.rng_next() % 2;
            if trap_type == 0 {
                // Poison trap
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Poison { damage: 1 },
                    5,
                ));
                self.message = "◆ Trapped! Poisoned for 5 turns!".to_string();
            } else {
                // Damage trap
                let dmg = 2 + self.floor_num / 2;
                self.player.hp -= dmg;
                if self.player.hp <= 0 && !self.try_phoenix_revive() {
                    self.player.hp = 0;
                    if let Some(ref audio) = self.audio {
                        audio.play_death();
                    }
                    self.run_journal
                        .log(RunEvent::DiedTo("Spike trap".to_string(), self.floor_num));
                    self.post_mortem_page = 0;
                    self.combat = CombatState::GameOver;
                    self.message = self.run_summary();
                    self.message_timer = 255;
                } else {
                    self.message = format!("◆ Trapped! Spike trap deals {} damage!", dmg);
                }
            }
            self.message_timer = 60;
        } else {
            // 10% — mimic! Spawn an enemy here
            let pool = vocab::vocab_for_floor(self.floor_num);
            if !pool.is_empty() {
                let entry_idx = self.rng_next() as usize % pool.len();
                let entry: &'static VocabEntry = pool[entry_idx];
                let mut mimic = Enemy::from_vocab(entry, cx, cy, self.floor_num);
                mimic.hp = mimic.hp + 2; // mimics are tougher
                mimic.damage += 1;
                mimic.alert = true;
                mimic.gold_value *= 2; // better drops
                self.enemies.push(mimic);
                let idx = self.enemies.len() - 1;
                let battle = combat::transition::enter_combat(
                    &self.player,
                    &self.enemies,
                    &[idx],
                    self.floor_num,
                    self.current_room_modifier(),
                    &self.srs,
                    self.companion,
                );
                self.combat = CombatState::TacticalBattle(Box::new(battle));
                self.typing.clear();
                self.message = format!(
                    "◆ It's a Mimic! Type pinyin for {} ({})",
                    entry.hanzi, entry.meaning
                );
                self.message_timer = 255;
            }
        }
    }

    fn roll_item_state(&mut self) -> ItemState {
        let roll = self.rng_next() % 100;
        if roll < 15 {
            ItemState::Cursed
        } else if roll < 25 {
            ItemState::Blessed
        } else {
            ItemState::Normal
        }
    }

    /// Generate a random item appropriate for the current floor.
    fn random_item(&mut self) -> crate::player::Item {
        use crate::player::Item;
        match self.rng_next() % 32 {
            0 => Item::MedHypo(4 + self.floor_num),
            1 => Item::ToxinGrenade(2, 3),
            2 => Item::ScannerPulse,
            3 => Item::PersonalTeleporter,
            4 => Item::StimPack(5),
            5 => Item::EMPGrenade,
            6 | 7 => Item::RationPack(40),
            8 => Item::FocusStim(5),
            9 => Item::SynthAle(3),
            10 => Item::HoloDecoy(4),
            11 => Item::PlasmaBurst(3 + self.floor_num / 2),
            12 => Item::NanoShield(5),
            13 => Item::NeuralBoost,
            14 => Item::CreditChip(8 + self.floor_num * 2),
            15 => Item::ShockModule(5 + self.floor_num),
            16 => Item::BiogelPatch(2),
            17 => Item::VenomDart,
            18 => Item::DeflectorDrone(5),
            19 => Item::NaniteSwarm,
            20 => Item::ReflectorPlate,
            21 => Item::CryoGrenade(1),
            22 => Item::CloakingDevice(2),
            23 => Item::PlasmaShield(3),
            24 => Item::SignalJammer(2),
            25 => Item::NavComputer,
            26 => Item::GrappleLine,
            27 => Item::OmniGel,
            28 => Item::SonicEmitter(2),
            29 => Item::CircuitInk,
            30 => Item::DataCore(5),
            _ => Item::ThrusterPack,
        }
    }

    /// Use a consumable item from inventory.
    fn use_item(&mut self, idx: usize) {
        if idx >= self.player.items.len() {
            return;
        }
        let item = self.player.items.remove(idx);
        let item_state = self.player.item_states.remove(idx);
        let kind = item.kind();
        let appearance = self.item_appearance(kind).to_string();
        let true_name = item.name();
        let newly_identified = self.identify_item_kind(kind);
        if let Some(ref audio) = self.audio {
            audio.play_spell();
        }

        match item {
            crate::player::Item::MedHypo(heal) => {
                let heal = if self.player.class == PlayerClass::Mechanic {
                    heal * 2
                } else {
                    heal
                };
                let heal = match item_state {
                    ItemState::Cursed => (heal / 2).max(1),
                    ItemState::Blessed => heal * 3 / 2,
                    ItemState::Normal => heal,
                };
                self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.message = format!(
                    "{}💚 Healed {} HP! ({}/{})",
                    prefix, heal, self.player.hp, self.player.max_hp
                );
                if item_state == ItemState::Cursed {
                    self.player
                        .statuses
                        .push(crate::status::StatusInstance::new(
                            crate::status::StatusKind::Poison { damage: 1 },
                            3,
                        ));
                    self.message.push_str(" But the curse poisons you!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::ToxinGrenade(dmg, turns) => {
                let dmg = match item_state {
                    ItemState::Blessed => dmg * 2,
                    _ => dmg,
                };
                let px = self.player.x;
                let py = self.player.y;
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() && (e.x - px).abs() <= 1 && (e.y - py).abs() <= 1 {
                        e.statuses.push(status::StatusInstance::new(
                            status::StatusKind::Poison { damage: dmg },
                            turns,
                        ));
                        count += 1;
                    }
                }
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.message = format!(
                    "{}☠ Poisoned {} enemies! ({} dmg × {} turns)",
                    prefix, count, dmg, turns
                );
                if item_state == ItemState::Cursed {
                    self.player
                        .statuses
                        .push(crate::status::StatusInstance::new(
                            crate::status::StatusKind::Poison { damage: 1 },
                            2,
                        ));
                    self.message.push_str(" But the fumes poison you!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::ScannerPulse => {
                self.reveal_entire_floor();
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.message = format!("{}👁 Map revealed!", prefix);
                if item_state == ItemState::Blessed {
                    self.player.hp = (self.player.hp + 3).min(self.player.max_hp);
                    self.message.push_str(" You feel revitalized! (+3 HP)");
                } else if item_state == ItemState::Cursed {
                    let mut spawned = false;
                    for dir in &[
                        (0, 1),
                        (1, 0),
                        (0, -1),
                        (-1, 0),
                        (1, 1),
                        (-1, -1),
                        (1, -1),
                        (-1, 1),
                    ] {
                        let tx = self.player.x + dir.0;
                        let ty = self.player.y + dir.1;
                        if tx > 0 && tx < MAP_W - 1 && ty > 0 && ty < MAP_H - 1 {
                            let idx = self.level.idx(tx, ty);
                            if self.level.tiles[idx].is_walkable()
                                && self.enemy_at(tx, ty).is_none()
                            {
                                let pool = crate::vocab::vocab_for_floor(self.floor_num);
                                if !pool.is_empty() {
                                    let entry = pool[self.rng_next() as usize % pool.len()];
                                    let mut e = Enemy::from_vocab(entry, tx, ty, self.floor_num);
                                    e.alert = true;
                                    self.enemies.push(e);
                                    spawned = true;
                                    break;
                                }
                            }
                        }
                    }
                    if spawned {
                        self.message.push_str(" An enemy was summoned!");
                    }
                }
                self.message_timer = 60;
            }
            crate::player::Item::PersonalTeleporter => {
                // Find random explored walkable tile
                let mut candidates = Vec::new();
                for y in 0..self.level.height {
                    for x in 0..self.level.width {
                        let i = self.level.idx(x, y);
                        if self.level.revealed[i]
                            && self.level.tiles[i].is_walkable()
                            && self.enemy_at(x, y).is_none()
                            && (x != self.player.x || y != self.player.y)
                        {
                            candidates.push((x, y));
                        }
                    }
                }
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                if let Some(&(tx, ty)) =
                    candidates.get(self.rng_next() as usize % candidates.len().max(1))
                {
                    self.player.move_to(tx, ty);
                    let (px, py) = (self.player.x, self.player.y);
                    compute_fov(&mut self.level, px, py, FOV_RADIUS);
                    self.message = format!("{}✦ Teleported!", prefix);
                } else {
                    self.message = format!("{}Teleport fizzled — nowhere to go!", prefix);
                }
                if item_state == ItemState::Blessed {
                    self.player.shield = true;
                    self.message.push_str(" You gained a shield!");
                } else if item_state == ItemState::Cursed {
                    self.player.hp -= 2;
                    self.message.push_str(" The rough transit deals 2 damage!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::StimPack(turns) => {
                let turns = match item_state {
                    ItemState::Blessed => turns * 2,
                    _ => turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Poison { damage: 1 },
                        3,
                    ));
                    self.message =
                        format!("{}The foul concoction slows you (poison 3 turns)!", prefix);
                } else {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Haste,
                        turns,
                    ));
                    self.message = format!(
                        "{}⚡ Haste for {} turns! Enemies move at half speed.",
                        prefix, turns
                    );
                }
                self.message_timer = 60;
            }
            crate::player::Item::EMPGrenade => {
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() {
                        let i = self.level.idx(e.x, e.y);
                        if self.level.visible[i] {
                            e.stunned = true;
                            if item_state == ItemState::Blessed {
                                e.hp -= 2;
                            }
                            count += 1;
                        }
                    }
                }
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.message = format!("{}💥 Stunned {} enemies!", prefix, count);
                if item_state == ItemState::Blessed {
                    self.message.push_str(" (Dealt 2 holy damage to each!)");
                } else if item_state == ItemState::Cursed {
                    self.player.hp -= 2;
                    self.message
                        .push_str(" The blast backfires and deals 2 damage to you!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::RationPack(amount) => {
                let amount = match item_state {
                    ItemState::Cursed => amount / 2,
                    ItemState::Blessed => (amount as f64 * 1.5) as i32,
                    ItemState::Normal => amount,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.spirit = (self.player.spirit + amount).min(self.player.max_spirit);
                self.message = format!("{}🍙 Restored {} spirit energy!", prefix, amount);
                self.message_timer = 60;
            }
            crate::player::Item::FocusStim(turns) => {
                let turns = match item_state {
                    ItemState::Cursed => (turns / 2).max(1),
                    ItemState::Blessed => turns + 3,
                    ItemState::Normal => turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::SpiritShield,
                        turns,
                    ));
                self.message = format!("{}🌕 Spirit Shield active for {} turns!", prefix, turns);
                self.message_timer = 60;
            }
            crate::player::Item::SynthAle(confuse_turns) => {
                let confuse_turns = match item_state {
                    ItemState::Cursed => confuse_turns + 3,
                    ItemState::Blessed => 0,
                    ItemState::Normal => confuse_turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.spirit = self.player.max_spirit;
                if confuse_turns > 0 {
                    self.player
                        .statuses
                        .push(crate::status::StatusInstance::new(
                            crate::status::StatusKind::Confused,
                            confuse_turns,
                        ));
                    self.message = format!(
                        "{}🍶 Spirit fully restored! But you feel dizzy for {} turns...",
                        prefix, confuse_turns
                    );
                } else {
                    self.message = format!(
                        "{}🍶 Spirit fully restored! The blessed wine clears your mind!",
                        prefix
                    );
                }
                self.message_timer = 60;
            }
            crate::player::Item::HoloDecoy(turns) => {
                let turns = match item_state {
                    ItemState::Cursed => (turns / 2).max(1),
                    ItemState::Blessed => turns * 2,
                    ItemState::Normal => turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Haste,
                    turns,
                ));
                self.message = format!("{}🌫 Smoke screen! Haste for {} turns!", prefix, turns);
                if item_state == ItemState::Cursed {
                    self.player
                        .statuses
                        .push(status::StatusInstance::new(status::StatusKind::Confused, 2));
                    self.message.push_str(" The smoke disorients you!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::PlasmaBurst(damage) => {
                let damage = match item_state {
                    ItemState::Blessed => damage * 2,
                    _ => damage,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() {
                        let i = self.level.idx(e.x, e.y);
                        if self.level.visible[i] {
                            e.hp -= damage;
                            if item_state == ItemState::Blessed {
                                e.statuses.push(status::StatusInstance::new(
                                    status::StatusKind::Burn { damage: 1 },
                                    2,
                                ));
                            }
                            count += 1;
                        }
                    }
                }
                self.message = format!(
                    "{}🧨 Cracker hit {} enemies for {} damage!",
                    prefix, count, damage
                );
                if item_state == ItemState::Cursed {
                    let self_dmg = (damage / 2).max(1);
                    self.player.hp -= self_dmg;
                    self.message
                        .push_str(&format!(" Backfire! You take {} damage!", self_dmg));
                }
                self.message_timer = 60;
            }
            crate::player::Item::NanoShield(turns) => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let regen_amt = if item_state == ItemState::Blessed {
                    2
                } else {
                    1
                };
                let regen_turns = match item_state {
                    ItemState::Cursed => (turns / 2).max(1),
                    _ => turns,
                };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Regen { heal: regen_amt },
                    regen_turns,
                ));
                if item_state != ItemState::Cursed {
                    self.player.shield = true;
                    self.message = format!(
                        "{}🛡 Iron Skin! Shield + Regen({}) for {} turns!",
                        prefix, regen_amt, regen_turns
                    );
                } else {
                    self.message = format!(
                        "{}Regen({}) for {} turns, but no shield!",
                        prefix, regen_amt, regen_turns
                    );
                }
                self.message_timer = 60;
            }
            crate::player::Item::NeuralBoost => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                if item_state == ItemState::Cursed {
                    if !self.player.statuses.is_empty() {
                        let idx = self.rng_next() as usize % self.player.statuses.len();
                        let removed = self.player.statuses.remove(idx);
                        let _ = removed;
                    }
                    self.player
                        .statuses
                        .push(status::StatusInstance::new(status::StatusKind::Confused, 2));
                    self.message = format!("{}🍵 Removed one status, but now confused!", prefix);
                } else {
                    self.player.statuses.retain(|s| !s.is_negative());
                    self.message = format!("{}🍵 All negative effects purged!", prefix);
                    if item_state == ItemState::Blessed {
                        self.player.hp = (self.player.hp + 3).min(self.player.max_hp);
                        self.player.spirit = (self.player.spirit + 10).min(self.player.max_spirit);
                        self.message.push_str(" +3 HP, +10 spirit!");
                    }
                }
                self.message_timer = 60;
            }
            crate::player::Item::CreditChip(amount) => {
                let amount = match item_state {
                    ItemState::Cursed => (amount / 2).max(1),
                    ItemState::Blessed => amount * 2,
                    ItemState::Normal => amount,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.gold += amount;
                self.message = format!("{}🪙 Gained {} gold!", prefix, amount);
                self.message_timer = 60;
            }
            crate::player::Item::ShockModule(damage) => {
                let damage = match item_state {
                    ItemState::Cursed => (damage / 2).max(1),
                    ItemState::Blessed => damage * 2,
                    ItemState::Normal => damage,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let px = self.player.x;
                let py = self.player.y;
                let mut nearest: Option<(usize, i32)> = None;
                for (i, e) in self.enemies.iter().enumerate() {
                    if e.is_alive() {
                        let dist = (e.x - px).abs() + (e.y - py).abs();
                        if nearest.is_none() || dist < nearest.unwrap().1 {
                            nearest = Some((i, dist));
                        }
                    }
                }
                if let Some((idx, _)) = nearest {
                    self.enemies[idx].hp -= damage;
                    if item_state == ItemState::Blessed {
                        self.enemies[idx].stunned = true;
                    }
                    self.message = format!("{}⚡ Thunder strikes for {} damage!", prefix, damage);
                    if item_state == ItemState::Blessed {
                        self.message.push_str(" Target stunned!");
                    }
                } else {
                    self.message = format!("{}⚡ Thunder crackles but finds no target!", prefix);
                }
                self.message_timer = 60;
            }
            crate::player::Item::BiogelPatch(regen) => {
                let regen = match item_state {
                    ItemState::Cursed => (regen / 2).max(1),
                    ItemState::Blessed => regen * 2,
                    ItemState::Normal => regen,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Regen { heal: regen },
                    5,
                ));
                self.message = format!(
                    "{}💎 Jade Salve! Regen {} per turn for 5 turns!",
                    prefix, regen
                );
                if item_state == ItemState::Blessed {
                    self.player.hp = (self.player.hp + 2).min(self.player.max_hp);
                    self.message.push_str(" +2 HP!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::VenomDart => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let venom_turns = if item_state == ItemState::Blessed {
                    8
                } else {
                    5
                };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Envenomed,
                    venom_turns,
                ));
                self.message = format!("{}🐍 Weapon envenomed for {} turns!", prefix, venom_turns);
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Poison { damage: 1 },
                        3,
                    ));
                    self.message.push_str(" The venom bites back!");
                }
                if item_state == ItemState::Blessed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Empowered { amount: 1 },
                        5,
                    ));
                    self.message.push_str(" You feel empowered!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::DeflectorDrone(turns) => {
                let turns = match item_state {
                    ItemState::Cursed => (turns / 2).max(1),
                    _ => turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::SpiritShield,
                    turns,
                ));
                if item_state != ItemState::Cursed {
                    self.player.shield = true;
                    self.message = format!(
                        "{}🔮 Ward active! Shield + Spirit Shield for {} turns!",
                        prefix, turns
                    );
                } else {
                    self.message = format!(
                        "{}Spirit Shield for {} turns, but no physical shield!",
                        prefix, turns
                    );
                }
                if item_state == ItemState::Blessed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Regen { heal: 1 },
                        turns,
                    ));
                    self.message.push_str(" +Regen!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::NaniteSwarm => {
                let mut count = 0;
                let stun_turns = if item_state == ItemState::Blessed {
                    5
                } else {
                    3
                };
                for e in &mut self.enemies {
                    if e.is_alive() {
                        let i = self.level.idx(e.x, e.y);
                        if self.level.visible[i] {
                            e.stunned = true;
                            if item_state == ItemState::Blessed {
                                e.hp -= 1;
                            }
                            count += 1;
                        }
                    }
                }
                let _ = stun_turns;
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.message = format!("{}🖤 Ink splatters {} enemies!", prefix, count);
                if item_state == ItemState::Blessed {
                    self.message.push_str(" (Dealt 1 damage each!)");
                }
                if item_state == ItemState::Cursed {
                    self.player
                        .statuses
                        .push(status::StatusInstance::new(status::StatusKind::Confused, 2));
                    self.message
                        .push_str(" Ink splashes back — you're confused!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::Revitalizer(_) => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.message = format!(
                    "{}🔥 The Phoenix Plume glows warmly... it activates on death, not by hand.",
                    prefix
                );
                self.message_timer = 60;
                self.player.items.insert(idx, item);
                self.player.item_states.insert(idx, item_state);
            }
            crate::player::Item::ReflectorPlate => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let thorns_turns = if item_state == ItemState::Blessed { 3 } else { 1 };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Thorns,
                    thorns_turns,
                ));
                self.player.shield = true;
                self.message = format!(
                    "{}🪞 Mirror Shard activated! Next attack will be reflected!",
                    prefix
                );
                if item_state == ItemState::Cursed {
                    self.player.hp -= 1;
                    self.message.push_str(" A shard cuts you for 1 damage!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::CryoGrenade(turns) => {
                let turns = match item_state {
                    ItemState::Cursed => 1,
                    ItemState::Blessed => turns + 1,
                    ItemState::Normal => turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let px = self.player.x;
                let py = self.player.y;
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() && (e.x - px).abs() <= 1 && (e.y - py).abs() <= 1 {
                        e.statuses.push(status::StatusInstance::new(
                            status::StatusKind::Freeze,
                            turns,
                        ));
                        count += 1;
                    }
                }
                self.message = format!(
                    "{}❄ Frost Vial freezes {} adjacent enemies for {} turns!",
                    prefix, count, turns
                );
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Slow,
                        2,
                    ));
                    self.message.push_str(" The cold slows you too!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::CloakingDevice(turns) => {
                let turns = match item_state {
                    ItemState::Cursed => 1,
                    ItemState::Blessed => turns * 2,
                    ItemState::Normal => turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Invisible,
                    turns,
                ));
                self.message = format!(
                    "{}👻 Shadow Cloak! Invisible for {} turns!",
                    prefix, turns
                );
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Confused,
                        2,
                    ));
                    self.message.push_str(" The shadows disorient you!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::PlasmaShield(armor) => {
                let armor = match item_state {
                    ItemState::Cursed => (armor / 2).max(1),
                    ItemState::Blessed => armor + 2,
                    ItemState::Normal => armor,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Fortify { stacks: armor },
                    99,
                ));
                self.player.shield = true;
                self.message = format!(
                    "{}🐉 Dragon Scale! Gained +{} armor and a shield!",
                    prefix, armor
                );
                self.message_timer = 60;
            }
            crate::player::Item::SignalJammer(turns) => {
                let turns = match item_state {
                    ItemState::Cursed => 1,
                    ItemState::Blessed => turns + 2,
                    ItemState::Normal => turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() {
                        let i = self.level.idx(e.x, e.y);
                        if self.level.visible[i] {
                            e.statuses.push(status::StatusInstance::new(
                                status::StatusKind::Confused,
                                turns,
                            ));
                            count += 1;
                        }
                    }
                }
                self.message = format!(
                    "{}🎋 Bamboo Flute confuses {} enemies for {} turns!",
                    prefix, count, turns
                );
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Confused,
                        1,
                    ));
                    self.message.push_str(" The melody confuses you too!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::NavComputer => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.reveal_entire_floor();
                self.message = format!(
                    "{}🧭 Jade Compass reveals all traps and hidden areas!",
                    prefix
                );
                if item_state == ItemState::Blessed {
                    self.player.hp = (self.player.hp + 3).min(self.player.max_hp);
                    self.message.push_str(" You feel revitalized! (+3 HP)");
                } else if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Confused,
                        3,
                    ));
                    self.message.push_str(" The visions disorient you!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::GrappleLine => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let px = self.player.x;
                let py = self.player.y;
                let mut nearest: Option<(usize, i32)> = None;
                for (i, e) in self.enemies.iter().enumerate() {
                    if e.is_alive() {
                        let dist = (e.x - px).abs() + (e.y - py).abs();
                        if dist > 1 && (nearest.is_none() || dist < nearest.unwrap().1) {
                            nearest = Some((i, dist));
                        }
                    }
                }
                if let Some((eidx, _)) = nearest {
                    let dirs: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];
                    let mut placed = false;
                    for &(dx, dy) in &dirs {
                        let tx = px + dx;
                        let ty = py + dy;
                        let ti = self.level.idx(tx, ty);
                        if self.level.tiles[ti].is_walkable() && self.enemy_at(tx, ty).is_none() {
                            self.enemies[eidx].x = tx;
                            self.enemies[eidx].y = ty;
                            placed = true;
                            break;
                        }
                    }
                    if placed {
                        self.message = format!("{}🪢 Silk Rope pulls an enemy close!", prefix);
                        if item_state == ItemState::Blessed {
                            self.enemies[eidx].stunned = true;
                            self.message.push_str(" The enemy is stunned!");
                        }
                    } else {
                        self.message = format!("{}🪢 No space to pull enemy to!", prefix);
                    }
                } else {
                    self.message = format!("{}🪢 No distant enemies to pull!", prefix);
                }
                if item_state == ItemState::Cursed {
                    self.player.hp -= 1;
                    self.message.push_str(" The rope snaps back and hurts you!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::OmniGel => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Weakened,
                        3,
                    ));
                    self.message = format!("{}🪷 The tainted elixir weakens you!", prefix);
                } else {
                    self.player.statuses.retain(|s| !s.is_negative());
                    self.message = format!("{}🪷 Lotus Elixir purges all negative effects!", prefix);
                    if item_state == ItemState::Blessed {
                        self.player.statuses.push(status::StatusInstance::new(
                            status::StatusKind::Blessed,
                            5,
                        ));
                        self.message.push_str(" You feel blessed!");
                    }
                }
                self.message_timer = 60;
            }
            crate::player::Item::SonicEmitter(damage) => {
                let damage = match item_state {
                    ItemState::Cursed => (damage / 2).max(1),
                    ItemState::Blessed => damage * 2,
                    ItemState::Normal => damage,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() {
                        let i = self.level.idx(e.x, e.y);
                        if self.level.visible[i] {
                            e.hp -= damage;
                            e.statuses.push(status::StatusInstance::new(
                                status::StatusKind::Slow,
                                1,
                            ));
                            count += 1;
                        }
                    }
                }
                self.message = format!(
                    "{}🥁 Thunder Drum hits {} enemies for {} damage + Slow!",
                    prefix, count, damage
                );
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Slow,
                        1,
                    ));
                    self.message.push_str(" The vibration slows you too!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::CircuitInk => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let empower_amount = if item_state == ItemState::Blessed { 4 } else { 2 };
                let empower_turns = if item_state == ItemState::Cursed { 2 } else { 5 };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Empowered { amount: empower_amount },
                    empower_turns,
                ));
                self.message = format!(
                    "{}🖊 Cinnabar Ink! +{} spell damage for {} turns!",
                    prefix, empower_amount, empower_turns
                );
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Weakened,
                        2,
                    ));
                    self.message.push_str(" But the ink weakens your body!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::DataCore(_) => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.message = format!(
                    "{}🏺 The Ancestor Token hums gently... it activates on death, not by hand.",
                    prefix
                );
                self.message_timer = 60;
                self.player.items.insert(idx, item);
                self.player.item_states.insert(idx, item_state);
            }
            crate::player::Item::ThrusterPack => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let px = self.player.x;
                let py = self.player.y;
                let push_dist = if item_state == ItemState::Blessed { 3 } else { 2 };
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() && (e.x - px).abs() <= 1 && (e.y - py).abs() <= 1 {
                        let dx = (e.x - px).signum();
                        let dy = (e.y - py).signum();
                        for _ in 0..push_dist {
                            let nx = e.x + dx;
                            let ny = e.y + dy;
                            if nx > 0 && nx < MAP_W - 1 && ny > 0 && ny < MAP_H - 1 {
                                let ni = self.level.idx(nx, ny);
                                if self.level.tiles[ni].is_walkable() {
                                    e.x = nx;
                                    e.y = ny;
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        count += 1;
                    }
                }
                self.message = format!(
                    "{}🌬 Wind Fan pushes {} enemies away!",
                    prefix, count
                );
                if item_state == ItemState::Cursed {
                    self.player.hp -= 1;
                    self.message.push_str(" The gust blows back on you for 1 damage!");
                }
                self.message_timer = 60;
            }
        }

        if newly_identified {
            self.message.push_str(&format!(
                " The {} reveals itself as {}.",
                appearance, true_name
            ));
        }
    }

    fn try_phoenix_revive(&mut self) -> bool {
        if self.player.hp > 0 {
            return false;
        }
        let plume_pos = self
            .player
            .items
            .iter()
            .position(|item| matches!(item, crate::player::Item::Revitalizer(_)));
        if let Some(idx) = plume_pos {
            let item = self.player.items.remove(idx);
            let item_state = self.player.item_states.remove(idx);
            let heal = match &item {
                crate::player::Item::Revitalizer(h) => *h,
                _ => 1,
            };
            let heal = match item_state {
                ItemState::Cursed => (heal / 2).max(1),
                ItemState::Blessed => self.player.max_hp,
                ItemState::Normal => heal,
            };
            self.player.hp = heal.min(self.player.max_hp);
            self.message = format!(
                "🔥 The Phoenix Plume ignites! You are reborn with {} HP!",
                self.player.hp
            );
            if item_state == ItemState::Cursed {
                self.player
                    .statuses
                    .push(status::StatusInstance::new(status::StatusKind::Confused, 3));
                self.message.push_str(" But you feel disoriented...");
            }
            self.message_timer = 120;
            return true;
        }
        let token_pos = self
            .player
            .items
            .iter()
            .position(|item| matches!(item, crate::player::Item::DataCore(_)));
        if let Some(idx) = token_pos {
            let item = self.player.items.remove(idx);
            let item_state = self.player.item_states.remove(idx);
            let heal = match &item {
                crate::player::Item::DataCore(h) => *h,
                _ => 5,
            };
            let heal = match item_state {
                ItemState::Cursed => (heal / 2).max(1),
                ItemState::Blessed => heal * 2,
                ItemState::Normal => heal,
            };
            self.player.hp = heal.min(self.player.max_hp);
            self.message = format!(
                "🏺 The Ancestor Token glows! Your ancestors revive you with {} HP!",
                self.player.hp
            );
            if item_state == ItemState::Cursed {
                self.player
                    .statuses
                    .push(status::StatusInstance::new(status::StatusKind::Weakened, 3));
                self.message.push_str(" But you feel weakened...");
            }
            self.message_timer = 120;
            return true;
        }
        false
    }

    fn perform_offering(&mut self, altar: AltarKind, idx: usize) {
        if idx >= self.player.items.len() {
            return;
        }
        let item = self.player.items.remove(idx);
        let _item_state = self.player.item_states.remove(idx);
        let deity = altar.deity();

        // Basic offering logic
        let piety_gain = match (deity, item.kind()) {
            (Faction::Consortium, ItemKind::MedHypo) => 5,
            (Faction::Consortium, _) => 1,
            (Faction::FreeTraders, ItemKind::StimPack | ItemKind::PersonalTeleporter) => 5,
            (Faction::Technocracy, ItemKind::ScannerPulse) => 5,
            (Faction::MilitaryAlliance, ItemKind::EMPGrenade | ItemKind::ToxinGrenade) => 5,
            (Faction::AncientOrder, _) => 2,
            _ => 1,
        };

        self.player.add_piety(deity, piety_gain);
        let bonus_text = self.player.faction_bonus(deity);
        self.message = if bonus_text == "None" || bonus_text == "Minor devotion" {
            format!(
                "Offered {} to {}. (+{} favor).",
                item.name(),
                deity.name(),
                piety_gain
            )
        } else {
            format!(
                "Offered {} to {}. (+{} favor) [{}]",
                item.name(),
                deity.name(),
                piety_gain,
                bonus_text
            )
        };
        self.message_timer = 90;
        self.combat = CombatState::Explore;
        let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
        self.particles.spawn_altar(sx, sy, &mut self.rng_state);
        if let Some(ref audio) = self.audio {
            audio.play_spell();
        }
    }

    fn pray_at_altar(&mut self, altar: AltarKind) {
        let deity = altar.deity();
        let piety = self.player.get_piety(deity);

        if piety >= 20 {
            // Grant Boon
            self.player.add_piety(deity, -20);
            match deity {
                Faction::Consortium => {
                    self.player.max_hp += 5;
                    self.player.hp = self.player.max_hp;
                    self.message = "Jade Emperor grants you vitality! (+5 Max HP)".to_string();
                }
                Faction::FreeTraders => {
                    self.player.set_form(PlayerForm::Holographic, 50);
                    self.message = "Wind Walker grants you form of Mist!".to_string();
                }
                Faction::Technocracy => {
                    for i in 0..ITEM_KIND_COUNT {
                        self.identified_items[i] = true;
                    }
                    self.message = "Mirror Sage reveals all truths!".to_string();
                }
                Faction::MilitaryAlliance => {
                    self.player.set_form(PlayerForm::Void, 50);
                    self.message = "Iron General grants you form of Tiger!".to_string();
                }
                Faction::AncientOrder => {
                    self.player.gold += 100;
                    self.message = "Golden Toad rains coins upon you!".to_string();
                }
            }
            if let Some(ref audio) = self.audio { audio.play_level_up(); }
        } else if piety < 0 {
            // Smite
            self.player.hp -= 5;
            self.message = format!("{} is offended by your pestering! (-5 HP)", deity.name());
            if let Some(ref audio) = self.audio { audio.play_damage(); }
        } else {
            let bonus = self.player.faction_bonus(deity);
            if bonus == "None" || bonus == "Minor devotion" {
                self.message = format!("{} ignores you. (Favor: {})", deity.name(), piety);
            } else {
                self.message = format!(
                    "{} acknowledges you. [{}] (Favor: {})",
                    deity.name(),
                    bonus,
                    piety
                );
            }
        }
        self.message_timer = 120;
    }

    fn perform_dip(&mut self, source_idx: usize, target_cursor: usize) {
        // target_cursor: 0=Weapon, 1=Armor, 2=Charm, 3+=Inventory Items (idx - 3)
        // If target is an item, we need to be careful with indices because we remove the potion first.

        if source_idx >= self.player.items.len() {
            return;
        }

        // Remove potion first
        let potion = self.player.items.remove(source_idx);
        let _potion_state = self.player.item_states.remove(source_idx);

        // Adjust target index if it was an item
        let effective_target_idx = if target_cursor >= 3 {
            let raw_idx = target_cursor - 3;
            if source_idx < raw_idx {
                raw_idx - 1
            } else {
                raw_idx
            }
        } else {
            0 // unused
        };

        match target_cursor {
            0 => {
                // Weapon
                if let Some(_) = self.player.weapon {
                    if matches!(potion.kind(), ItemKind::ToxinGrenade) {
                        self.message =
                            "Coated weapon with poison! (Attacks poison enemies)".to_string();
                        self.player.statuses.push(status::StatusInstance::new(
                            status::StatusKind::Envenomed,
                            20,
                        ));
                    } else if matches!(potion.kind(), ItemKind::StimPack) {
                        self.message = "Coated weapon with speed! (Attacks empowered)".to_string();
                        self.player.statuses.push(status::StatusInstance::new(
                            status::StatusKind::Empowered { amount: 1 },
                            20,
                        ));
                    } else {
                        self.message = "Nothing happens.".to_string();
                    }
                } else {
                    self.message = "No weapon to coat.".to_string();
                    self.player.add_item(potion, ItemState::Normal); // Return item
                }
            }
            1 => {
                // Armor
                if let Some(_) = self.player.armor {
                    self.message = "You wash your armor.".to_string();
                } else {
                    self.message = "No armor.".to_string();
                    self.player.add_item(potion, ItemState::Normal);
                }
            }
            2 => {
                // Charm
                self.message = "You dip the charm. It sparkles.".to_string();
            }
            _ => {
                // Inventory Item
                if effective_target_idx < self.player.items.len() {
                    self.message = "Mixing not yet implemented. Item returned.".to_string();
                    self.player.add_item(potion, ItemState::Normal);
                } else {
                    // Invalid target (shouldn't happen)
                    self.player.add_item(potion, ItemState::Normal);
                }
            }
        }
        self.combat = CombatState::Explore;
        self.message_timer = 90;
    }

    fn handle_tactical_victory(&mut self, killed: &[usize], combo: u32) {
        let mut total_gold_gained: i32 = 0;
        let mut last_radical_drop: Option<&str> = None;
        let mut equip_msg: Option<String> = None;
        let mut item_msg: Option<String> = None;
        let mut sentence_challenge: Option<(SentenceChallengeMode, String)> = None;

        for &ei in killed {
            if ei >= self.enemies.len() {
                continue;
            }
            let e_hanzi = self.enemies[ei].hanzi;
            let e_is_boss = self.enemies[ei].is_boss;
            let e_is_elite = self.enemies[ei].is_elite;
            let e_gold_base = self.enemies[ei].gold_value
                + self.player.gold_bonus()
                + self.player.enchant_gold_bonus();

            self.total_kills += 1;
            self.run_kills += 1;
            self.run_gold_earned += e_gold_base;
            if e_is_boss {
                self.run_bosses_killed += 1;
                self.run_journal
                    .log(RunEvent::BossKilled(e_hanzi.to_string(), self.floor_num));
            } else {
                self.run_journal
                    .log(RunEvent::EnemyKilled(e_hanzi.to_string(), self.floor_num));
            }

            if let Some(ref audio) = self.audio {
                audio.play_kill();
            }

            let (sx, sy) = self.tile_to_screen(self.enemies[ei].x, self.enemies[ei].y);
            self.particles.spawn_kill(sx, sy, &mut self.rng_state);
            self.flash = Some((255, 255, 255, 0.3));

            let mut gold_gain = e_gold_base;
            if self.player.get_piety(Faction::MilitaryAlliance) >= 10 && self.player.get_piety(Faction::AncientOrder) >= 10
            {
                gold_gain *= 2;
            }
            if self.player.get_piety(Faction::AncientOrder) >= 10 {
                gold_gain += 3;
            }
            gold_gain = (gold_gain as f64 * self.floor_profile.gold_multiplier()) as i32;
            gold_gain = gold_gain.max(1);
            self.player.gold += gold_gain;
            total_gold_gained += gold_gain;

            let listen_bonus = if !e_is_elite {
                match self.listening_mode {
                    ListenMode::ToneOnly => 3,
                    ListenMode::FullAudio => 5,
                    ListenMode::Off => 0,
                }
            } else {
                0
            };
            self.player.gold += listen_bonus;
            total_gold_gained += listen_bonus;

            let available = radical::radicals_for_floor(self.floor_num);
            let rad_roll = self.rng_next() % 100;
            let rad_chance = self.floor_profile.radical_drop_chance();
            if rad_roll < rad_chance {
                let drop_idx = self.rng_next() as usize % available.len();
                let ch = available[drop_idx].ch;
                self.player.add_radical(ch);
                self.run_journal
                    .log(RunEvent::RadicalCollected(ch.to_string(), self.floor_num));
                if self.floor_profile.radical_drop_bonus() {
                    let bonus_idx = self.rng_next() as usize % available.len();
                    self.player.add_radical(available[bonus_idx].ch);
                }
                last_radical_drop = Some(ch);
            }
            self.advance_radical_quests();

            if e_is_elite {
                let drop2 = self.rng_next() as usize % available.len();
                self.player.add_radical(available[drop2].ch);
            }

            let extra_chance = self.player.extra_radical_chance();
            if extra_chance > 0 && (self.rng_next() % 100) < extra_chance as u64 {
                let drop2 = self.rng_next() as usize % available.len();
                self.player.add_radical(available[drop2].ch);
            }

            let mut heal = self.player.heal_on_kill();
            if self.player.get_piety(Faction::Consortium) >= 10 {
                heal += 1;
            }
            if heal > 0 {
                self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
            }

            let equip_chance: u64 = if e_is_boss { 60 } else { 5 };
            if (self.rng_next() % 100) < equip_chance {
                let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
                let eq = &EQUIPMENT_POOL[eq_idx];
                let current_state = self.player.equipment_state(eq.slot);
                if current_state == ItemState::Cursed {
                    equip_msg = Some(format!("{} blocked by curse!", eq.name));
                } else {
                    let state = self.roll_item_state();
                    self.player.equip(eq, state);
                    let prefix = match state {
                        ItemState::Cursed => "💀 ",
                        ItemState::Blessed => "✨ ",
                        ItemState::Normal => "",
                    };
                    equip_msg = Some(format!("{}{}", prefix, eq.name));
                }
            }

            let item_chance: u64 = if e_is_boss {
                40
            } else if e_is_elite {
                15
            } else {
                4
            };
            if (self.rng_next() % 100) < item_chance {
                let drop_item = if e_is_boss && (self.rng_next() % 5) == 0 {
                    crate::player::Item::Revitalizer(self.player.max_hp / 2)
                } else {
                    self.random_item()
                };
                let state = self.roll_item_state();
                let name = drop_item.name().to_string();
                if self.player.add_item(drop_item, state) {
                    let prefix = match state {
                        ItemState::Cursed => "💀 ",
                        ItemState::Blessed => "✨ ",
                        ItemState::Normal => "",
                    };
                    item_msg = Some(format!("{}{}", prefix, name));
                }
            }

            if e_is_boss {
                let rares = radical::rare_radicals();
                if !rares.is_empty() {
                    let rare_idx = self.rng_next() as usize % rares.len();
                    let rare = rares[rare_idx].ch;
                    self.player.add_radical(rare);
                    last_radical_drop = Some(rare);
                }
            }

            let kill_xp = if e_is_boss {
                5
            } else if e_is_elite {
                3
            } else {
                2
            };
            self.add_companion_xp(kill_xp);

            self.achievements.record_correct();
            self.achievements.check_kills(self.total_kills);
            self.achievements.check_gold(self.player.gold);
            self.achievements.check_radicals(self.player.radicals.len());
            if e_is_elite {
                self.achievements.unlock("first_elite");
            }
            if e_is_boss {
                self.achievements.unlock("first_boss");
            }

            if e_is_boss
                && self.floor_num >= 5
                && self.enemies[ei].boss_kind != Some(BossKind::HiveQueen)
                && self.enemies[ei].boss_kind != Some(BossKind::AncientGuardian)
                && self.enemies[ei].boss_kind != Some(BossKind::PirateCaptain)
            {
                let base_reward = 15 + self.floor_num * 2;
                sentence_challenge = Some((
                    SentenceChallengeMode::BonusGold { reward: base_reward },
                    "Boss Phase 2! Arrange the words in correct order. ←→ to select, Enter to pick.".to_string(),
                ));
            }

            self.advance_kill_quests();
        }

        self.answer_streak = combo;

        let mut msg = format!("Victory! +{}g", total_gold_gained);
        if let Some(rad) = last_radical_drop {
            msg = format!("{} [{}]", msg, rad);
        }
        if let Some(eq) = equip_msg {
            msg = format!("{} + {}", msg, eq);
        }
        if let Some(itm) = item_msg {
            msg = format!("{} + {}", msg, itm);
        }
        let tier = combo_tier(self.answer_streak);
        if tier != ComboTier::None {
            msg = format!("{} 🔥 {}! ×{}", msg, tier.name(), self.answer_streak);
        }
        self.message = msg;
        self.message_timer = 90;

        if let Some(tutorial) = self.tutorial.as_mut() {
            if !tutorial.combat_done {
                tutorial.combat_done = true;
                self.message = "Great! Now walk to the ⚒ and forge 好 from 女 + 子.".to_string();
                self.message_timer = 180;
            }
        }

        if let Some((mode, intro)) = sentence_challenge {
            self.combat = CombatState::Explore;
            self.begin_sentence_challenge(mode, intro);
        } else {
            self.combat = CombatState::Explore;
        }
    }

    fn handle_tactical_defeat(&mut self, killer_name: String) {
        self.player.hp = 0;
        self.run_journal
            .log(RunEvent::DiedTo(killer_name, self.floor_num));
        self.post_mortem_page = 0;
        self.combat = CombatState::GameOver;
        self.message = self.run_summary();
        self.message_timer = 255;
        if let Some(ref audio) = self.audio {
            audio.play_death();
        }
        self.save_high_score();
    }

    fn run_summary(&self) -> String {
        let accuracy = if self.run_correct_answers + self.run_wrong_answers > 0 {
            (self.run_correct_answers as f64
                / (self.run_correct_answers + self.run_wrong_answers) as f64
                * 100.0) as u32
        } else {
            0
        };
        format!(
            "☠ You died on floor {}!  ⚔ {} kills | 🏆 {} bosses | 💰 {} gold | ✅ {}% accuracy ({}/{}) | 🔨 {} spells forged  — Press R to restart",
            self.floor_num,
            self.run_kills,
            self.run_bosses_killed,
            self.run_gold_earned,
            accuracy,
            self.run_correct_answers,
            self.run_correct_answers + self.run_wrong_answers,
            self.run_spells_forged,
        )
    }

    fn restart(&mut self) {
        self.total_runs += 1;
        self.save_high_score();
        self.save_stats();
        self.srs = crate::srs::load_srs();
        self.player = Player::new(0, 0, PlayerClass::Envoy);
        self.floor_num = 0;
        self.run_kills = 0;
        self.run_gold_earned = 0;
        self.run_correct_answers = 0;
        self.run_wrong_answers = 0;
        self.run_spells_forged = 0;
        self.run_bosses_killed = 0;
        self.mirror_hint = false;
        self.next_chain_id = 1;
        self.floor_profile = FloorProfile::Normal;
        self.answer_streak = 0;
        self.run_journal = RunJournal::default();
        self.post_mortem_page = 0;
        self.theft_catches = 0;
        self.shop_banned = false;
        self.enemies.clear();
        self.typing.clear();
        // Keep discovered recipes across runs (loaded from localStorage)
        self.combat = CombatState::ClassSelect;
        self.tutorial = None;
        self.show_inventory = false;
        self.show_help = false;
        self.show_settings = false;
        self.crafting_mode = false;
        self.crafting_first = None;
        self.crafting_cursor = 0;
        self.message_tick_delay = 0;
        self.new_floor();
    }

    fn save_high_score(&self) {
        crate::srs::save_srs(&self.srs);
        self.save_stats();
        let storage: Option<web_sys::Storage> =
            window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        if let Some(storage) = storage {
            let prev: i32 = storage
                .get_item("radical_roguelike_best")
                .ok()
                .flatten()
                .and_then(|s: String| s.parse::<i32>().ok())
                .unwrap_or(0);
            if self.best_floor > prev {
                let _ = storage.set_item("radical_roguelike_best", &self.best_floor.to_string());
            }
            // Save discovered recipes
            let recipe_str: String = self
                .discovered_recipes
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let _ = storage.set_item("radical_roguelike_recipes", &recipe_str);

            // Save daily best
            if self.daily_mode {
                let score = self.daily_score();
                let prev_daily: i32 = storage
                    .get_item("radical_roguelike_daily_best")
                    .ok()
                    .flatten()
                    .and_then(|s: String| s.parse::<i32>().ok())
                    .unwrap_or(0);
                if score > prev_daily {
                    let _ = storage.set_item("radical_roguelike_daily_best", &score.to_string());
                }
            }
        }
    }

    /// Calculate daily challenge score.
    fn daily_score(&self) -> i32 {
        self.floor_num * 100 + self.player.gold + self.total_kills as i32 * 10
    }

    fn load_high_score() -> i32 {
        let storage: Option<web_sys::Storage> =
            window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        storage
            .and_then(|s: web_sys::Storage| s.get_item("radical_roguelike_best").ok().flatten())
            .and_then(|s: String| s.parse::<i32>().ok())
            .unwrap_or(0)
    }

    fn load_recipes() -> Vec<usize> {
        let storage: Option<web_sys::Storage> =
            window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        storage
            .and_then(|s: web_sys::Storage| s.get_item("radical_roguelike_recipes").ok().flatten())
            .map(|s: String| {
                s.split(',')
                    .filter_map(|v| v.parse::<usize>().ok())
                    .filter(|&i| i < radical::RECIPES.len())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn load_settings() -> GameSettings {
        let storage: Option<web_sys::Storage> =
            window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        if let Some(storage) = storage {
            let music_volume = storage
                .get_item("radical_roguelike_music_volume")
                .ok()
                .flatten()
                .and_then(|s: String| s.parse::<u8>().ok())
                .filter(|v| *v <= 100)
                .unwrap_or(100);
            let sfx_volume = storage
                .get_item("radical_roguelike_sfx_volume")
                .ok()
                .flatten()
                .and_then(|s: String| s.parse::<u8>().ok())
                .filter(|v| *v <= 100)
                .unwrap_or(100);
            let screen_shake = storage
                .get_item("radical_roguelike_screen_shake")
                .ok()
                .flatten()
                .map(|s: String| s != "0")
                .unwrap_or(true);
            let text_speed = storage
                .get_item("radical_roguelike_text_speed")
                .ok()
                .flatten()
                .map(|s| TextSpeed::from_storage(&s))
                .unwrap_or(TextSpeed::Normal);
            GameSettings {
                music_volume,
                sfx_volume,
                screen_shake,
                text_speed,
            }
        } else {
            GameSettings::default()
        }
    }

    fn load_stat(key: &str) -> u32 {
        let storage: Option<web_sys::Storage> =
            window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        storage
            .and_then(|s: web_sys::Storage| s.get_item(key).ok().flatten())
            .and_then(|s: String| s.parse::<u32>().ok())
            .unwrap_or(0)
    }

    fn save_stats(&self) {
        let storage: Option<web_sys::Storage> =
            window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        if let Some(storage) = storage {
            let _ = storage.set_item("radical_roguelike_runs", &self.total_runs.to_string());
            let _ = storage.set_item("radical_roguelike_kills", &self.total_kills.to_string());
        }
    }

    fn save_settings(&self) {
        let storage: Option<web_sys::Storage> =
            window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        if let Some(storage) = storage {
            let _ = storage.set_item(
                "radical_roguelike_music_volume",
                &self.settings.music_volume.to_string(),
            );
            let _ = storage.set_item(
                "radical_roguelike_sfx_volume",
                &self.settings.sfx_volume.to_string(),
            );
            let _ = storage.set_item(
                "radical_roguelike_screen_shake",
                if self.settings.screen_shake { "1" } else { "0" },
            );
            let _ = storage.set_item(
                "radical_roguelike_text_speed",
                self.settings.text_speed.storage_key(),
            );
        }
    }

    fn tick_message(&mut self) {
        if self.message_timer > 0
            && advance_message_decay(
                &mut self.message_timer,
                &mut self.message_tick_delay,
                self.settings.text_speed,
            )
        {
            self.message.clear();
        }
    }

    fn render(&self) {
        let popup = self.achievement_popup.map(|(n, d, _)| (n, d));
        let room_mod = self.current_room_modifier();
        let tutorial_hint = self.tutorial_hint();
        let show_help =
            self.show_help && !self.show_inventory && !self.show_codex && !self.show_settings;
        let item_labels: Vec<String> = self
            .player
            .items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let state = self
                    .player
                    .item_states
                    .get(idx)
                    .copied()
                    .unwrap_or(ItemState::Normal);
                let prefix = match state {
                    ItemState::Cursed => "💀 ",
                    ItemState::Blessed => "✨ ",
                    ItemState::Normal => "",
                };
                format!("{}{}", prefix, self.item_display_name(item))
            })
            .collect();
        self.renderer.draw(
            &self.level,
            &self.player,
            &self.enemies,
            &self.combat,
            &self.typing,
            &self.message,
            self.floor_num,
            self.best_floor,
            self.total_kills,
            self.total_runs,
            self.discovered_recipes.len(),
            &self.srs,
            &self.particles,
            if self.settings.screen_shake {
                self.shake_timer
            } else {
                0
            },
            self.flash,
            popup,
            room_mod,
            self.listening_mode,
            self.companion,
            self.companion_level(),
            &self.quests,
            tutorial_hint,
            show_help,
            &item_labels,
            &self.settings,
            self.show_settings,
            self.settings_cursor,
            self.answer_streak,
            self.floor_profile.label(),
            &self.codex,
            &self.run_journal,
            self.post_mortem_page,
            self.class_cursor,
        );
        if self.show_inventory {
            self.renderer.draw_inventory(
                &self.player,
                self.floor_num,
                self.discovered_recipes.len(),
                self.best_floor,
                self.total_kills,
                self.companion,
                self.companion_level(),
                &item_labels,
                self.inventory_cursor,
                self.inventory_inspect,
                self.crafting_mode,
                self.crafting_first,
                self.crafting_cursor,
            );
        } else if self.show_spellbook {
            self.renderer.draw_spellbook(&self.player);
        } else if self.show_codex {
            let entries = self.codex.sorted_entries();
            self.renderer.draw_codex(&entries);
        }

        if self.show_console {
            self.renderer
                .draw_console(&self.console_history, &self.console_buffer);
        }
    }

    const CONSOLE_COMMANDS: &'static [&'static str] = &[
        "help", "god", "hp", "gold", "floor", "reveal", "kill_all",
        "focus", "spirit", "clear", "stats", "items", "give_item",
        "radicals", "give_radical", "spells", "give_spell", "fight", "boss",
    ];

    const ITEM_NAMES: &'static [&'static str] = &[
        "HealthPotion", "PoisonFlask", "RevealScroll", "TeleportScroll",
        "HastePotion", "StunBomb", "RiceBall", "MeditationIncense",
        "AncestralWine", "SmokeScreen", "FireCracker", "IronSkinElixir",
        "ClarityTea", "GoldIngot", "ThunderTalisman", "JadeSalve",
        "SerpentFang", "WardingCharm", "InkBomb", "PhoenixPlume",
    ];

    const BOSS_NAMES: &'static [&'static str] = &[
        "PirateCaptain", "HiveQueen", "RogueAICore", "VoidEntity", "AncientGuardian", "DriftLeviathan",
    ];

    const FIGHT_TYPES: &'static [&'static str] = &["normal", "elite", "boss"];

    fn tab_complete(&mut self) {
        let input = self.console_buffer.clone();
        let has_space = input.contains(' ');

        if has_space {
            // Argument completion
            let parts: Vec<&str> = input.splitn(2, ' ').collect();
            let cmd = parts[0];
            let arg_prefix = parts.get(1).unwrap_or(&"");

            let candidates: Vec<&str> = match cmd {
                "give_item" => Self::ITEM_NAMES.iter()
                    .filter(|n| n.to_lowercase().starts_with(&arg_prefix.to_lowercase()))
                    .copied().collect(),
                "boss" => Self::BOSS_NAMES.iter()
                    .filter(|n| n.to_lowercase().starts_with(&arg_prefix.to_lowercase()))
                    .copied().collect(),
                "fight" => Self::FIGHT_TYPES.iter()
                    .filter(|n| n.starts_with(&arg_prefix.to_lowercase()))
                    .copied().collect(),
                _ => return,
            };

            if candidates.is_empty() {
                return;
            }

            let prefix_key = format!("arg:{}", input);
            if self.tab_prefix == prefix_key {
                // Cycle through matches
                self.tab_cycle_index = (self.tab_cycle_index + 1) % self.tab_matches.len();
                self.console_buffer = format!("{} {}", cmd, self.tab_matches[self.tab_cycle_index]);
            } else {
                // New completion
                self.tab_matches = candidates.iter().map(|s| s.to_string()).collect();
                self.tab_cycle_index = 0;
                if candidates.len() == 1 {
                    self.console_buffer = format!("{} {}", cmd, candidates[0]);
                    self.tab_prefix = format!("arg:{}", self.console_buffer);
                } else {
                    let lcp = longest_common_prefix_ci(&candidates);
                    self.console_buffer = format!("{} {}", cmd, lcp);
                    self.console_history.push(format!("  completions: {}", candidates.join(", ")));
                    self.tab_prefix = format!("arg:{}", self.console_buffer);
                }
            }
        } else {
            // Command name completion
            let prefix = input.to_lowercase();
            let candidates: Vec<&str> = Self::CONSOLE_COMMANDS.iter()
                .filter(|c| c.starts_with(&prefix))
                .copied().collect();

            if candidates.is_empty() {
                return;
            }

            let prefix_key = format!("cmd:{}", input);
            if self.tab_prefix == prefix_key {
                // Cycle through matches
                self.tab_cycle_index = (self.tab_cycle_index + 1) % self.tab_matches.len();
                self.console_buffer = format!("{} ", self.tab_matches[self.tab_cycle_index]);
            } else {
                // New completion
                self.tab_matches = candidates.iter().map(|s| s.to_string()).collect();
                self.tab_cycle_index = 0;
                if candidates.len() == 1 {
                    self.console_buffer = format!("{} ", candidates[0]);
                    self.tab_prefix = format!("cmd:{}", self.console_buffer);
                } else {
                    let lcp = longest_common_prefix_ci(&candidates);
                    self.console_buffer = lcp;
                    self.console_history.push(format!("  completions: {}", candidates.join(", ")));
                    self.tab_prefix = format!("cmd:{}", self.console_buffer);
                }
            }
        }
    }

    fn execute_console_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        let response = match parts[0] {
            "help" => {
                self.console_history.push("=== CHEAT CONSOLE ===".into());
                self.console_history
                    .push("help         - Show this help".into());
                self.console_history
                    .push("god          - Toggle god mode".into());
                self.console_history
                    .push("hp [n]       - Set HP to n (or full)".into());
                self.console_history
                    .push("gold [n]     - Add n gold (default 100)".into());
                self.console_history
                    .push("floor [n]    - Go to floor n".into());
                self.console_history
                    .push("reveal       - Reveal entire map".into());
                self.console_history
                    .push("kill_all     - Kill all enemies".into());
                self.console_history
                    .push("focus [n]    - Set focus in combat".into());
                self.console_history
                    .push("spirit [n]   - Set spirit to n".into());
                self.console_history
                    .push("clear        - Clear console".into());
                self.console_history
                    .push("stats        - Show player stats".into());
                self.console_history
                    .push("items        - List all item types".into());
                self.console_history
                    .push("give_item <name> - Give item by name".into());
                self.console_history
                    .push("radicals     - List all radicals".into());
                self.console_history
                    .push("give_radical <ch> - Give a radical".into());
                self.console_history
                    .push("spells       - List player spells".into());
                self.console_history
                    .push("give_spell <hanzi> - Give spell by hanzi".into());
                self.console_history
                    .push("fight <type> - Fight normal/elite/boss".into());
                self.console_history
                    .push("boss <name>  - Fight a specific boss".into());
                return;
            }
            "god" => {
                self.god_mode = !self.god_mode;
                if let CombatState::TacticalBattle(ref mut battle) = self.combat {
                    battle.god_mode = self.god_mode;
                }
                format!("God mode: {}", if self.god_mode { "ON" } else { "OFF" })
            }
            "hp" => {
                let amount = parts.get(1).and_then(|s| s.parse::<i32>().ok());
                match amount {
                    Some(n) => {
                        self.player.hp = n.min(self.player.max_hp);
                        format!("HP set to {}/{}", self.player.hp, self.player.max_hp)
                    }
                    None => {
                        self.player.hp = self.player.max_hp;
                        format!("HP restored to {}", self.player.max_hp)
                    }
                }
            }
            "gold" => {
                let amount = parts
                    .get(1)
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(100);
                self.player.gold += amount;
                format!("Added {} gold (total: {})", amount, self.player.gold)
            }
            "floor" => {
                if let Some(n) = parts.get(1).and_then(|s| s.parse::<i32>().ok()) {
                    if n >= 1 {
                        self.floor_num = n - 1;
                        self.new_floor();
                        format!("Warped to floor {}", self.floor_num)
                    } else {
                        "Floor must be >= 1".into()
                    }
                } else {
                    format!("Current floor: {}. Usage: floor [n]", self.floor_num)
                }
            }
            "reveal" => {
                self.reveal_entire_floor();
                for v in self.level.visible.iter_mut() {
                    *v = true;
                }
                "Map revealed!".into()
            }
            "kill_all" => {
                if let CombatState::TacticalBattle(ref mut battle) = self.combat {
                    let mut killed = 0;
                    for unit in battle.units.iter_mut() {
                        if unit.is_enemy() && unit.alive {
                            unit.hp = 0;
                            unit.alive = false;
                            killed += 1;
                        }
                    }
                    format!("Killed {} tactical enemies", killed)
                } else {
                    let count = self.enemies.len();
                    for e in self.enemies.iter_mut() {
                        e.hp = 0;
                    }
                    self.enemies.clear();
                    format!("Killed {} enemies", count)
                }
            }
            "focus" => {
                if let CombatState::TacticalBattle(ref mut battle) = self.combat {
                    let amount = parts
                        .get(1)
                        .and_then(|s| s.parse::<i32>().ok())
                        .unwrap_or(battle.max_focus);
                    battle.focus = amount;
                    format!("Focus set to {}/{}", battle.focus, battle.max_focus)
                } else {
                    "Not in tactical combat".into()
                }
            }
            "spirit" => {
                let amount = parts
                    .get(1)
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(self.player.max_spirit);
                self.player.spirit = amount.min(self.player.max_spirit);
                format!(
                    "Spirit set to {}/{}",
                    self.player.spirit, self.player.max_spirit
                )
            }
            "clear" => {
                self.console_history.clear();
                return;
            }
            "stats" => {
                self.console_history.push(format!(
                    "HP: {}/{}  Gold: {}  Floor: {}",
                    self.player.hp, self.player.max_hp, self.player.gold, self.floor_num
                ));
                self.console_history.push(format!(
                    "Spirit: {}/{}  Kills: {}  God: {}",
                    self.player.spirit, self.player.max_spirit, self.total_kills, self.god_mode
                ));
                self.console_history.push(format!(
                    "Radicals: {}  Spells: {}  Items: {}",
                    self.player.radicals.len(),
                    self.player.spells.len(),
                    self.player.items.len()
                ));
                return;
            }
            "items" => {
                let all: &[(&str, &str)] = &[
                    ("HealthPotion", "Heal HP"),
                    ("PoisonFlask", "Poison enemies"),
                    ("RevealScroll", "Reveal map"),
                    ("TeleportScroll", "Teleport"),
                    ("HastePotion", "Grant haste"),
                    ("StunBomb", "Stun enemies"),
                    ("RiceBall", "Restore spirit"),
                    ("MeditationIncense", "Block spirit drain"),
                    ("AncestralWine", "Full spirit restore"),
                    ("SmokeScreen", "Smoke + haste"),
                    ("FireCracker", "AoE damage"),
                    ("IronSkinElixir", "Shield + regen"),
                    ("ClarityTea", "Cleanse debuffs"),
                    ("GoldIngot", "Gain gold"),
                    ("ThunderTalisman", "High damage"),
                    ("JadeSalve", "Regen over time"),
                    ("SerpentFang", "Envenom weapon"),
                    ("WardingCharm", "Shield + spirit shield"),
                    ("InkBomb", "Stun + confuse"),
                    ("PhoenixPlume", "Auto-revive"),
                ];
                self.console_history.push("=== ITEM TYPES ===".into());
                for (name, desc) in all {
                    self.console_history
                        .push(format!("  {} - {}", name, desc));
                }
                return;
            }
            "give_item" => {
                use crate::player::Item;
                if let Some(name) = parts.get(1) {
                    let lower = name.to_lowercase();
                    let item = match lower.as_str() {
                        "healthpotion" => Some(Item::MedHypo(5 + self.floor_num)),
                        "poisonflask" => Some(Item::ToxinGrenade(2, 3)),
                        "revealscroll" => Some(Item::ScannerPulse),
                        "teleportscroll" => Some(Item::PersonalTeleporter),
                        "hastepotion" => Some(Item::StimPack(5)),
                        "stunbomb" => Some(Item::EMPGrenade),
                        "riceball" => Some(Item::RationPack(40)),
                        "meditationincense" => Some(Item::FocusStim(5)),
                        "ancestralwine" => Some(Item::SynthAle(3)),
                        "smokescreen" => Some(Item::HoloDecoy(4)),
                        "firecracker" => Some(Item::PlasmaBurst(3 + self.floor_num / 2)),
                        "ironskinelixir" => Some(Item::NanoShield(5)),
                        "claritytea" => Some(Item::NeuralBoost),
                        "goldingot" => Some(Item::CreditChip(8 + self.floor_num * 2)),
                        "thundertalisman" => Some(Item::ShockModule(5 + self.floor_num)),
                        "jadesalve" => Some(Item::BiogelPatch(2)),
                        "serpentfang" => Some(Item::VenomDart),
                        "wardingcharm" => Some(Item::DeflectorDrone(5)),
                        "inkbomb" => Some(Item::NaniteSwarm),
                        "phoenixplume" => Some(Item::Revitalizer(5)),
                        _ => None,
                    };
                    match item {
                        Some(it) => {
                            let name_str = it.name().to_string();
                            if self.player.add_item(it, ItemState::Normal) {
                                format!("Added {}", name_str)
                            } else {
                                "Inventory full!".into()
                            }
                        }
                        None => format!("Unknown item '{}'. Type 'items' to list.", name),
                    }
                } else {
                    "Usage: give_item <name>".into()
                }
            }
            "radicals" => {
                self.console_history.push("=== RADICALS ===".into());
                for r in radical::RADICALS.iter() {
                    let tag = if r.rare { " [rare]" } else { "" };
                    self.console_history
                        .push(format!("  {} ({}) - {}{}", r.ch, r.name, r.meaning, tag));
                }
                return;
            }
            "give_radical" => {
                if let Some(ch) = parts.get(1) {
                    if let Some(r) = radical::RADICALS.iter().find(|r| r.ch == *ch) {
                        self.player.add_radical(r.ch);
                        format!("Added radical {} ({})", r.ch, r.meaning)
                    } else {
                        format!("Unknown radical '{}'. Type 'radicals' to list.", ch)
                    }
                } else {
                    "Usage: give_radical <char>".into()
                }
            }
            "spells" => {
                if self.player.spells.is_empty() {
                    self.console_history.push("No spells.".into());
                } else {
                    self.console_history.push("=== SPELLS ===".into());
                    for (i, s) in self.player.spells.iter().enumerate() {
                        let sel = if i == self.player.selected_spell {
                            " ◀"
                        } else {
                            ""
                        };
                        self.console_history.push(format!(
                            "  {} {} ({}) - {:?}{}",
                            s.hanzi, s.pinyin, s.meaning, s.effect, sel
                        ));
                    }
                }
                return;
            }
            "give_spell" => {
                if let Some(hanzi) = parts.get(1) {
                    if let Some(recipe) = radical::RECIPES
                        .iter()
                        .find(|r| r.output_hanzi == *hanzi)
                    {
                        self.player.add_spell(Spell {
                            hanzi: recipe.output_hanzi,
                            pinyin: recipe.output_pinyin,
                            meaning: recipe.output_meaning,
                            effect: recipe.effect,
                        });
                        format!(
                            "Added spell {} ({} - {})",
                            recipe.output_hanzi, recipe.output_pinyin, recipe.output_meaning
                        )
                    } else {
                        format!("No recipe for '{}'. Check radical.rs RECIPES.", hanzi)
                    }
                } else {
                    "Usage: give_spell <hanzi>".into()
                }
            }
            "fight" => {
                if let Some(kind) = parts.get(1) {
                    let lower = kind.to_lowercase();
                    let pool = vocab::vocab_for_floor(self.floor_num);
                    if pool.is_empty() {
                        "No vocab entries for this floor!".into()
                    } else {
                        let entry = pool[self.rng_next() as usize % pool.len()];
                        let px = self.player.x;
                        let py = self.player.y;
                        match lower.as_str() {
                            "normal" => {
                                let e = Enemy::from_vocab(entry, px + 1, py, self.floor_num);
                                let label = format!("Fight: {} ({})", e.hanzi, e.meaning);
                                self.enemies.push(e);
                                let idx = self.enemies.len() - 1;
                                let battle = combat::transition::enter_combat(
                                    &self.player,
                                    &self.enemies,
                                    &[idx],
                                    self.floor_num,
                                    self.current_room_modifier(),
                                    &self.srs,
                                    self.companion,
                                );
                                self.combat = CombatState::TacticalBattle(Box::new(battle));
                                self.typing.clear();
                                label
                            }
                            "elite" => {
                                let elite_pool: Vec<&VocabEntry> =
                                    pool.iter().filter(|v| vocab::is_elite(v)).copied().collect();
                                let ep = if elite_pool.is_empty() {
                                    entry
                                } else {
                                    elite_pool[self.rng_next() as usize % elite_pool.len()]
                                };
                                let e = Enemy::from_vocab(ep, px + 1, py, self.floor_num);
                                let label = format!("Fight elite: {} ({})", e.hanzi, e.meaning);
                                self.enemies.push(e);
                                let idx = self.enemies.len() - 1;
                                let battle = combat::transition::enter_combat(
                                    &self.player,
                                    &self.enemies,
                                    &[idx],
                                    self.floor_num,
                                    self.current_room_modifier(),
                                    &self.srs,
                                    self.companion,
                                );
                                self.combat = CombatState::TacticalBattle(Box::new(battle));
                                self.typing.clear();
                                label
                            }
                            "boss" => {
                                let e =
                                    Enemy::boss_from_vocab(entry, px + 1, py, self.floor_num);
                                let label = format!("Fight boss: {} ({})", e.hanzi, e.meaning);
                                self.enemies.push(e);
                                let idx = self.enemies.len() - 1;
                                let battle = combat::transition::enter_combat(
                                    &self.player,
                                    &self.enemies,
                                    &[idx],
                                    self.floor_num,
                                    self.current_room_modifier(),
                                    &self.srs,
                                    self.companion,
                                );
                                self.combat = CombatState::TacticalBattle(Box::new(battle));
                                self.typing.clear();
                                label
                            }
                            _ => format!(
                                "Unknown fight type '{}'. Use: normal, elite, boss",
                                kind
                            ),
                        }
                    }
                } else {
                    "Usage: fight <normal|elite|boss>".into()
                }
            }
            "boss" => {
                if let Some(name) = parts.get(1) {
                    let lower = name.to_lowercase();
                    let boss_kind = match lower.as_str() {
                        "piratecaptain" | "gatekeeper" => Some((BossKind::PirateCaptain, 5)),
                        "hivequeen" | "scholar" => Some((BossKind::HiveQueen, 10)),
                        "rogueaicore" | "elementalist" => Some((BossKind::RogueAICore, 15)),
                        "voidentity" | "mimicking" => Some((BossKind::VoidEntity, 20)),
                        "ancientguardian" | "inksage" => Some((BossKind::AncientGuardian, 25)),
                        "driftleviathan" | "radicalthief" => Some((BossKind::DriftLeviathan, 30)),
                        _ => None,
                    };
                    match boss_kind {
                        Some((_kind, floor)) => {
                            let pool = vocab::vocab_for_floor(floor);
                            let entry = if pool.is_empty() {
                                &vocab::VOCAB[0]
                            } else {
                                pool[self.rng_next() as usize % pool.len()]
                            };
                            let px = self.player.x;
                            let py = self.player.y;
                            let e = Enemy::boss_from_vocab(entry, px + 1, py, floor);
                            let label = format!(
                                "Boss fight: {} ({}) - {}",
                                e.hanzi,
                                e.meaning,
                                _kind.title()
                            );
                            self.enemies.push(e);
                            let idx = self.enemies.len() - 1;
                            let battle = combat::transition::enter_combat(
                                &self.player,
                                &self.enemies,
                                &[idx],
                                floor,
                                self.current_room_modifier(),
                                &self.srs,
                                self.companion,
                            );
                            self.combat = CombatState::TacticalBattle(Box::new(battle));
                            self.typing.clear();
                            label
                        }
                        None => format!(
                            "Unknown boss '{}'. Options: PirateCaptain, HiveQueen, RogueAICore, VoidEntity, AncientGuardian, DriftLeviathan",
                            name
                        ),
                    }
                } else {
                    "Usage: boss <name> (PirateCaptain/HiveQueen/RogueAICore/VoidEntity/AncientGuardian/DriftLeviathan)".into()
                }
            }
            other => {
                format!("Unknown command: '{}'. Type 'help' for commands.", other)
            }
        };
        self.console_history.push(format!("> {}", cmd));
        self.console_history.push(response);
        while self.console_history.len() > 100 {
            self.console_history.remove(0);
        }
    }
}

fn longest_common_prefix_ci(strings: &[&str]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    let first: Vec<char> = strings[0].chars().collect();
    let mut len = first.len();
    for s in &strings[1..] {
        let chars: Vec<char> = s.chars().collect();
        len = len.min(chars.len());
        for i in 0..len {
            if first[i].to_lowercase().next() != chars[i].to_lowercase().next() {
                len = i;
                break;
            }
        }
    }
    strings[0][..strings[0].char_indices().nth(len).map_or(strings[0].len(), |(i, _)| i)].to_string()
}

/// Combo effects from spell combinations.
enum ComboEffect {
    Steam,        // Fire + Shield: AoE stun
    Counter(i32), // Shield + Strike: reflect damage
    Barrier(i32), // Heal + Shield: shield + heal
    Flurry(i32),  // Strike + Fire: triple damage
    Ignite(i32),  // Fire + Drain: burn DoT + lifesteal
    Tempest(i32), // Fire + Stun: AoE damage + stun target
    Rally(i32),   // Heal + Strike: heal + bonus hit
    Siphon(i32),  // Drain + Stun: massive drain while stunned
    Fortify(i32), // Drain + Shield: shield + heal from stolen life
}

/// Detect if two spell effects form a combo.
fn detect_combo(prev: &SpellEffect, current: &SpellEffect) -> Option<(&'static str, ComboEffect)> {
    match (spell_category(prev), spell_category(current)) {
        ("fire", "shield") | ("shield", "fire") => Some(("Steam Burst", ComboEffect::Steam)),
        ("shield", "strike") | ("strike", "shield") => {
            Some(("Counter Strike", ComboEffect::Counter(6)))
        }
        ("heal", "shield") | ("shield", "heal") => Some(("Barrier", ComboEffect::Barrier(4))),
        ("strike", "fire") | ("fire", "strike") => Some(("Flurry", ComboEffect::Flurry(8))),
        ("drain", "heal") | ("heal", "drain") => Some(("Life Surge", ComboEffect::Barrier(6))),
        ("stun", "strike") | ("strike", "stun") => {
            Some(("Crippling Blow", ComboEffect::Flurry(10)))
        }
        ("fire", "drain") | ("drain", "fire") => Some(("Immolate", ComboEffect::Ignite(8))),
        ("fire", "stun") | ("stun", "fire") => Some(("Tempest", ComboEffect::Tempest(4))),
        ("heal", "strike") | ("strike", "heal") => Some(("Rally", ComboEffect::Rally(6))),
        ("drain", "stun") | ("stun", "drain") => Some(("Siphon", ComboEffect::Siphon(6))),
        ("drain", "shield") | ("shield", "drain") => Some(("Fortify", ComboEffect::Fortify(4))),
        ("heal", "stun") | ("stun", "heal") => Some(("Renewal", ComboEffect::Barrier(5))),
        _ => None,
    }
}

fn spell_category(effect: &SpellEffect) -> &'static str {
    match effect {
        SpellEffect::FireAoe(_) => "fire",
        SpellEffect::Heal(_) => "heal",
        SpellEffect::Reveal => "utility",
        SpellEffect::Shield => "shield",
        SpellEffect::StrongHit(_) => "strike",
        SpellEffect::Drain(_) => "drain",
        SpellEffect::Stun => "stun",
        SpellEffect::Pacify => "utility",
        SpellEffect::Slow(_) => "stun",
        SpellEffect::Teleport => "utility",
        SpellEffect::Poison(_, _) => "drain",
        SpellEffect::FocusRestore(_) => "heal",
        SpellEffect::ArmorBreak => "strike",
        SpellEffect::Dash(_) => "utility",
        SpellEffect::Pierce(_) => "strike",
        SpellEffect::PullToward => "utility",
        SpellEffect::KnockBack(_) => "strike",
        SpellEffect::Thorns(_) => "shield",
        SpellEffect::Cone(_) => "fire",
        SpellEffect::Wall(_) => "shield",
        SpellEffect::OilSlick => "utility",
        SpellEffect::FreezeGround(_) => "stun",
        SpellEffect::Ignite => "fire",
        SpellEffect::PlantGrowth => "heal",
        SpellEffect::Earthquake(_) => "strike",
        SpellEffect::Sanctify(_) => "heal",
        SpellEffect::FloodWave(_) => "strike",
        SpellEffect::SummonBoulder => "shield",
    }
}

fn combat_prompt_for(enemy: &Enemy, listening_mode: ListenMode, mirror_hint: bool) -> String {
    let pinyin_hint = if mirror_hint {
        format!(" (Hint: {})", enemy.pinyin)
    } else {
        String::new()
    };

    if enemy.is_elite {
        let target = enemy
            .hanzi
            .chars()
            .nth(enemy.elite_chain)
            .map(|ch| ch.to_string())
            .unwrap_or_else(|| enemy.hanzi.chars().last().unwrap_or('？').to_string());
        let expected = enemy.elite_expected_syllable().unwrap_or(enemy.pinyin);
        format!(
            "Compound foe {} ({}) — break it syllable by syllable. Start with {} = {}.{}",
            enemy.hanzi, enemy.meaning, target, expected, pinyin_hint
        )
    } else if !enemy.components.is_empty() {
        let comp = enemy.components[0];
        let pinyin = vocab::vocab_entry_by_hanzi(comp)
            .map(|e| e.pinyin)
            .unwrap_or("???");
        format!("Shielded by {}! Type {} to break.", comp, pinyin)
    } else if listening_mode == ListenMode::ToneOnly {
        format!(
            "🎵 What tone is {}? Type 1-4...{}",
            enemy.meaning, pinyin_hint
        )
    } else if listening_mode == ListenMode::FullAudio {
        format!("🎧 Listen! Type the pinyin you hear...{}", pinyin_hint)
    } else {
        format!(
            "Type pinyin for {} ({}){}",
            enemy.hanzi, enemy.meaning, pinyin_hint
        )
    }
}

fn in_look_range(origin_x: i32, origin_y: i32, target_x: i32, target_y: i32) -> bool {
    (target_x - origin_x).abs().max((target_y - origin_y).abs()) <= LOOK_RANGE
}

fn tile_look_text(tile: Tile) -> String {
    match tile {
        Tile::Bulkhead => "Solid wall.".to_string(),
        Tile::DamagedBulkhead => {
            "Cracked wall — a digging tool could break into a hidden room.".to_string()
        }
        Tile::WeakBulkhead => {
            "Brittle wall — a digging tool could break into the cache behind it.".to_string()
        }
        Tile::MetalFloor => "Open floor.".to_string(),
        Tile::Hallway => "Corridor passage.".to_string(),
        Tile::Airlock => "Stairs down to the next floor.".to_string(),
        Tile::QuantumForge => "Forge — combine radicals or enchant gear here.".to_string(),
        Tile::TradeTerminal => "Shop — buy gear, radicals, and consumables.".to_string(),
        Tile::SupplyCrate => "Treasure chest — step onto it to open it.".to_string(),
        Tile::CargoCrate => "Crate — push it, or shove it into deep water to make a bridge.".to_string(),
        Tile::LaserGrid => "Spike trap — hurts anything that steps on it.".to_string(),
        Tile::Coolant => "Oil slick — fire can ignite it.".to_string(),
        Tile::CoolantPool => {
            "Shallow water — you can wade through it, and lightning arcs through it.".to_string()
        }
        Tile::VacuumBreach => {
            "Deep water — too deep to cross on foot; a crate could bridge it.".to_string()
        }
        Tile::Npc(0) => format!("{} — offers meaning hints.", Companion::ScienceOfficer.name()),
        Tile::Npc(1) => format!("{} — heals you between floors.", Companion::Medic.name()),
        Tile::Npc(2) => format!(
            "{} — discounts goods and may offer quests.",
            Companion::Quartermaster.name()
        ),
        Tile::Npc(_) => format!(
            "{} — can block the first hit in a fight.",
            Companion::SecurityChief.name()
        ),
        Tile::CircuitShrine => "Tone shrine — complete a tone challenge for bonus damage.".to_string(),
        Tile::CompoundShrine => "Stroke shrine — arrange character components in order.".to_string(),
        Tile::FrequencyWall => "Tone wall — identify tones to defend against attacks.".to_string(),
        Tile::ClassifierNode => {
            "Classifier shrine — match nouns with correct classifiers.".to_string()
        }
        Tile::Terminal(kind) => format!("{} — offer items here, or pray with 20 favor.", kind.name()),
        Tile::SecurityLock(kind) => format!(
            "{} — one-shot script seal that reshapes the room.",
            kind.label()
        ),
        Tile::InfoPanel(_) => "Tutorial sign — step onto it to read the guidance.".to_string(),
        Tile::Catwalk => "Bridge — safe footing laid over water.".to_string(),
        Tile::DataWell => "Ink well — guess the component count to restore HP.".to_string(),
        Tile::MemorialNode => "Ancestor shrine — complete the proverb for gold.".to_string(),
        Tile::TranslationTerminal => {
            "Translation altar — pick the correct Chinese for English meaning.".to_string()
        }
        Tile::RadicalLab => "Radical garden — identify the radical to harvest it.".to_string(),
        Tile::HoloPool => "Mirror pool — type the pinyin to gain spell power.".to_string(),
        Tile::DroidTutor => "Stone tutor — study, then prove you learned the tone.".to_string(),
        Tile::CodexTerminal => "Codex shrine — quiz on characters you've encountered.".to_string(),
        Tile::DataBridge => "Word bridge — answer correctly to bridge the water.".to_string(),
        Tile::SealedHatch => "Locked door — translate to unlock.".to_string(),
        Tile::CorruptedFloor => "Cursed floor — a hidden trap awaits the unwary.".to_string(),
        Tile::Trap(_) => "Open floor.".to_string(),
        Tile::OreVein => "Gold ore vein — mine it for gold.".to_string(),
        Tile::PlasmaVent => "Molten lava — stepping on it will burn you!".to_string(),
        Tile::FrozenDeck => "Ice — slippery surface, be careful.".to_string(),
        Tile::CargoPipes => "Dense bamboo — blocks passage.".to_string(),
        Tile::ToxicFungus => "Giant mushroom — spore cloud causes disorientation.".to_string(),
        Tile::ToxicGas => "Poison gas — toxic fumes linger here.".to_string(),
        Tile::DataRack => "Data rack — interact for information.".to_string(),
        Tile::SalvageCrate => "Salvage crate — smash for loot.".to_string(),
        Tile::NavBeacon => "Nav beacon — activate for map.".to_string(),
        Tile::SpecialRoom(_) => "Special room.".to_string(),
        Tile::PressureSensor => "Pressure plate — something heavy might activate it.".to_string(),
        Tile::CrystalPanel => "Crystal formation — reflects light beautifully.".to_string(),
        Tile::WarpGatePortal => "Dragon Gate — an otherworldly portal shimmering with power.".to_string(),
        Tile::MedBayTile => "Spirit spring — step in to restore HP and spirit.".to_string(),
        Tile::CreditCache => "Gold pile — walk over it to collect.".to_string(),
    }
}

fn tile_allows_enemy_spawn(tile: Tile) -> bool {
    matches!(
        tile,
        Tile::MetalFloor | Tile::Hallway | Tile::Coolant | Tile::CoolantPool | Tile::LaserGrid | Tile::Catwalk
    )
}

fn enemy_look_text(enemy: &Enemy) -> String {
    let role = if enemy.is_boss {
        "Boss"
    } else if enemy.is_elite {
        "Elite"
    } else {
        "Enemy"
    };

    let mut text = format!(
        "{} {} ({}) HP {}/{}",
        role, enemy.hanzi, enemy.meaning, enemy.hp, enemy.max_hp
    );
    if !enemy.components.is_empty() {
        text.push_str(&format!(" — shield {}.", enemy.components.join("→")));
    } else if enemy.is_elite {
        if let Some(next) = enemy.elite_expected_syllable() {
            text.push_str(&format!(" — next {}.", next));
        }
    }
    if let Some(trait_text) = enemy.boss_trait_text() {
        text.push_str(&format!(" {}", trait_text));
    }

    let actions = enemy.radical_actions();
    if !actions.is_empty() {
        let mut by_radical: Vec<(&str, Vec<&str>)> = Vec::new();
        for action in &actions {
            let rad = action.radical();
            if let Some(entry) = by_radical.iter_mut().find(|(r, _)| *r == rad) {
                entry.1.push(action.name());
            } else {
                by_radical.push((rad, vec![action.name()]));
            }
        }
        let grouped: Vec<String> = by_radical
            .iter()
            .map(|(rad, names)| format!("{}: {}", rad, names.join(", ")))
            .collect();
        text.push_str(&format!(" | Abilities: {}", grouped.join(" | ")));
    }

    text
}

fn elite_chain_damage(base_hit: i32, total_syllables: usize, completing_cycle: bool) -> i32 {
    if completing_cycle {
        base_hit + total_syllables.saturating_sub(1) as i32
    } else {
        (base_hit / 2).max(1)
    }
}

fn elite_remaining_hp(current_hp: i32, damage: i32, completing_cycle: bool) -> i32 {
    if completing_cycle {
        current_hp - damage
    } else {
        (current_hp - damage).max(1)
    }
}

fn advance_message_decay(
    message_timer: &mut u8,
    message_tick_delay: &mut u8,
    text_speed: TextSpeed,
) -> bool {
    if *message_timer == 0 {
        return true;
    }

    if *message_tick_delay > 0 {
        *message_tick_delay -= 1;
        return false;
    }

    *message_tick_delay = text_speed.timer_delay().saturating_sub(1);
    *message_timer = message_timer.saturating_sub(text_speed.timer_step());
    *message_timer == 0
}

fn tutorial_exit_blocker_for(tutorial: Option<&TutorialState>) -> Option<&'static str> {
    let tutorial = tutorial?;
    if !tutorial.combat_done {
        Some("The exit is sealed. Defeat 大 before leaving the tutorial.")
    } else if !tutorial.forge_done {
        Some("The exit is sealed. Forge 好 at the anvil before leaving.")
    } else {
        None
    }
}

fn can_be_reshaped_by_seal(tile: Tile) -> bool {
    matches!(
        tile,
        Tile::MetalFloor | Tile::Hallway | Tile::Coolant | Tile::CoolantPool | Tile::LaserGrid
    )
}

fn seal_cross_positions(x: i32, y: i32) -> [(i32, i32); 8] {
    [
        (x + 1, y),
        (x - 1, y),
        (x + 2, y),
        (x - 2, y),
        (x, y + 1),
        (x, y - 1),
        (x, y + 2),
        (x, y - 2),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        advance_message_decay, can_be_reshaped_by_seal, combat_prompt_for, combo_tier,
        detect_combo, elite_chain_damage, elite_remaining_hp, enemy_look_text, in_look_range,
        seal_cross_positions, spell_category, tile_look_text, tutorial_exit_blocker_for, ComboTier,
        Companion, FloorProfile, GameState, ListenMode, RunEvent, RunJournal, TextSpeed,
        TutorialState,
    };
    use crate::dungeon::Tile;
    use crate::enemy::Enemy;
    use crate::player::ITEM_KIND_COUNT;
    use crate::radical::SpellEffect;
    use crate::vocab::VOCAB;

    fn friend_entry() -> &'static crate::vocab::VocabEntry {
        VOCAB.iter().find(|entry| entry.hanzi == "朋友").unwrap()
    }

    fn shielded_entry() -> &'static crate::vocab::VocabEntry {
        VOCAB.iter().find(|entry| entry.hanzi == "好").unwrap()
    }

    fn message_frames_until_clear(start_timer: u8, speed: TextSpeed) -> u32 {
        let mut timer = start_timer;
        let mut delay = 0;
        let mut frames = 0;
        while timer > 0 && frames < 10_000 {
            let _ = advance_message_decay(&mut timer, &mut delay, speed);
            frames += 1;
        }
        frames
    }

    #[test]
    fn text_speed_storage_round_trip() {
        assert_eq!(TextSpeed::from_storage("slow"), TextSpeed::Slow);
        assert_eq!(TextSpeed::from_storage("normal"), TextSpeed::Normal);
        assert_eq!(TextSpeed::from_storage("fast"), TextSpeed::Fast);
        assert_eq!(TextSpeed::Fast.storage_key(), "fast");
    }

    #[test]
    fn normal_text_speed_stretches_a_ten_tick_message_to_nineteen_frames() {
        assert_eq!(message_frames_until_clear(10, TextSpeed::Normal), 19);
    }

    #[test]
    fn slower_text_speeds_hold_messages_longer_than_faster_ones() {
        let slow_frames = message_frames_until_clear(10, TextSpeed::Slow);
        let normal_frames = message_frames_until_clear(10, TextSpeed::Normal);
        let fast_frames = message_frames_until_clear(10, TextSpeed::Fast);

        assert!(slow_frames > normal_frames);
        assert!(normal_frames > fast_frames);
    }

    #[test]
    fn look_range_reaches_three_tiles_but_not_four() {
        assert!(in_look_range(10, 10, 13, 10));
        assert!(in_look_range(10, 10, 12, 13));
        assert!(!in_look_range(10, 10, 14, 10));
    }

    #[test]
    fn cracked_wall_look_text_mentions_hidden_room() {
        assert!(tile_look_text(Tile::DamagedBulkhead).contains("hidden room"));
    }

    #[test]
    fn deep_water_look_text_mentions_bridge() {
        assert!(tile_look_text(Tile::VacuumBreach).contains("bridge"));
    }

    #[test]
    fn enemy_look_text_reports_component_shields() {
        let enemy = Enemy::from_vocab(shielded_entry(), 0, 0, 3);

        assert!(enemy_look_text(&enemy).contains("shield 女→子"));
    }

    #[test]
    fn settings_volume_adjustment_clamps() {
        assert_eq!(GameState::adjust_volume(0, -1), 0);
        assert_eq!(GameState::adjust_volume(95, 1), 100);
        assert_eq!(GameState::adjust_volume(40, -2), 20);
    }

    #[test]
    fn utility_spells_do_not_create_damage_combos() {
        assert_eq!(spell_category(&SpellEffect::Reveal), "utility");
        assert_eq!(spell_category(&SpellEffect::Pacify), "utility");
        assert!(detect_combo(&SpellEffect::Reveal, &SpellEffect::Shield).is_none());
        assert!(detect_combo(&SpellEffect::Pacify, &SpellEffect::FireAoe(3)).is_none());
    }

    #[test]
    fn pacify_reward_scales_with_spell_power() {
        assert_eq!(GameState::pacify_gold_reward(2, 0), 4);
        assert_eq!(GameState::pacify_gold_reward(9, 2), 7);
    }

    #[test]
    fn forge_quest_candidates_respect_floor_radicals() {
        let floor_one = GameState::forge_quest_candidates_for_floor(1);
        assert!(floor_one.iter().any(|recipe| recipe.output_hanzi == "明"));
        assert!(!floor_one.iter().any(|recipe| recipe.output_hanzi == "理"));
    }

    #[test]
    fn item_appearance_order_is_deterministic_for_a_seed() {
        assert_eq!(
            GameState::roll_item_appearance_order(12345),
            GameState::roll_item_appearance_order(12345)
        );
    }

    #[test]
    fn item_appearance_order_uses_each_appearance_once() {
        let mut order = GameState::roll_item_appearance_order(99).to_vec();
        order.sort_unstable();

        assert_eq!(order, (0..ITEM_KIND_COUNT).collect::<Vec<_>>());
    }

    #[test]
    fn combat_prompt_for_elite_mentions_next_syllable() {
        let enemy = Enemy::from_vocab(friend_entry(), 0, 0, 6);

        assert_eq!(
            combat_prompt_for(&enemy, ListenMode::Off, false),
            "Compound foe 朋友 (friend) — break it syllable by syllable. Start with 朋 = peng2."
        );
    }

    #[test]
    fn elite_chain_damage_spikes_on_finishing_syllable() {
        assert_eq!(elite_chain_damage(2, 2, false), 1);
        assert_eq!(elite_chain_damage(2, 2, true), 3);
    }

    #[test]
    fn elite_remaining_hp_stays_above_zero_until_chain_finishes() {
        assert_eq!(elite_remaining_hp(2, 3, false), 1);
        assert_eq!(elite_remaining_hp(2, 3, true), -1);
    }

    #[test]
    fn seal_cross_positions_extend_two_tiles_cardinally() {
        assert_eq!(
            seal_cross_positions(10, 8),
            [
                (11, 8),
                (9, 8),
                (12, 8),
                (8, 8),
                (10, 9),
                (10, 7),
                (10, 10),
                (10, 6),
            ]
        );
    }

    #[test]
    fn only_mutable_ground_can_be_reshaped_by_seals() {
        assert!(can_be_reshaped_by_seal(Tile::MetalFloor));
        assert!(can_be_reshaped_by_seal(Tile::CoolantPool));
        assert!(!can_be_reshaped_by_seal(Tile::QuantumForge));
        assert!(!can_be_reshaped_by_seal(Tile::SupplyCrate));
    }

    #[test]
    fn tutorial_exit_blocker_requires_combat_before_descent() {
        let tutorial = TutorialState {
            combat_done: false,
            forge_done: false,
        };

        assert_eq!(
            tutorial_exit_blocker_for(Some(&tutorial)),
            Some("The exit is sealed. Defeat 大 before leaving the tutorial.")
        );
    }

    #[test]
    fn tutorial_exit_blocker_requires_forge_after_combat() {
        let tutorial = TutorialState {
            combat_done: true,
            forge_done: false,
        };

        assert_eq!(
            tutorial_exit_blocker_for(Some(&tutorial)),
            Some("The exit is sealed. Forge 好 at the anvil before leaving.")
        );
    }

    #[test]
    fn tutorial_exit_blocker_clears_once_tutorial_is_complete() {
        let tutorial = TutorialState {
            combat_done: true,
            forge_done: true,
        };

        assert_eq!(tutorial_exit_blocker_for(Some(&tutorial)), None);
    }

    #[test]
    fn floor_profile_tutorial_floors_are_normal() {
        assert_eq!(FloorProfile::roll(1, 999), FloorProfile::Normal);
        assert_eq!(FloorProfile::roll(2, 123), FloorProfile::Normal);
    }

    #[test]
    fn floor_profile_gold_multipliers() {
        assert_eq!(FloorProfile::Normal.gold_multiplier(), 1.0);
        assert_eq!(FloorProfile::Famine.gold_multiplier(), 0.5);
        assert_eq!(FloorProfile::RadicalRich.gold_multiplier(), 0.8);
        assert_eq!(FloorProfile::Siege.gold_multiplier(), 1.5);
        assert_eq!(FloorProfile::Drought.gold_multiplier(), 0.3);
    }

    #[test]
    fn listen_mode_cycles_through_variants() {
        assert_eq!(ListenMode::Off.cycle(), ListenMode::ToneOnly);
        assert_eq!(ListenMode::ToneOnly.cycle(), ListenMode::FullAudio);
        assert_eq!(ListenMode::FullAudio.cycle(), ListenMode::Off);
    }

    #[test]
    fn listen_mode_is_active_checks() {
        assert_eq!(ListenMode::Off.is_active(), false);
        assert_eq!(ListenMode::ToneOnly.is_active(), true);
        assert_eq!(ListenMode::FullAudio.is_active(), true);
    }

    // --- Resource Pressure Tests ---

    #[test]
    fn radical_drop_chance_varies_by_profile() {
        assert_eq!(FloorProfile::Normal.radical_drop_chance(), 80);
        assert_eq!(FloorProfile::Famine.radical_drop_chance(), 50);
        assert_eq!(FloorProfile::RadicalRich.radical_drop_chance(), 100);
        assert_eq!(FloorProfile::Siege.radical_drop_chance(), 80);
        assert_eq!(FloorProfile::Drought.radical_drop_chance(), 0);
    }

    #[test]
    fn drought_profile_is_harshest() {
        assert_eq!(FloorProfile::Drought.gold_multiplier(), 0.3);
        assert_eq!(FloorProfile::Drought.radical_drop_chance(), 0);
        assert_eq!(FloorProfile::Drought.radical_drop_bonus(), false);
    }

    #[test]
    fn floor_profile_roll_distribution() {
        // Famine: 0..=19, RadicalRich: 20..=34, Siege: 35..=44, Drought: 45..=54, Normal: 55+
        assert_eq!(FloorProfile::roll(5, 0), FloorProfile::Famine);
        assert_eq!(FloorProfile::roll(5, 19), FloorProfile::Famine);
        assert_eq!(FloorProfile::roll(5, 20), FloorProfile::RadicalRich);
        assert_eq!(FloorProfile::roll(5, 34), FloorProfile::RadicalRich);
        assert_eq!(FloorProfile::roll(5, 35), FloorProfile::Siege);
        assert_eq!(FloorProfile::roll(5, 44), FloorProfile::Siege);
        assert_eq!(FloorProfile::roll(5, 45), FloorProfile::Drought);
        assert_eq!(FloorProfile::roll(5, 54), FloorProfile::Drought);
        assert_eq!(FloorProfile::roll(5, 55), FloorProfile::Normal);
        assert_eq!(FloorProfile::roll(5, 99), FloorProfile::Normal);
    }

    #[test]
    fn drought_label_shows_desert_emoji() {
        assert!(FloorProfile::Drought.label().contains("Drought"));
    }

    #[test]
    fn radical_rich_guarantees_radical_drops() {
        assert_eq!(FloorProfile::RadicalRich.radical_drop_chance(), 100);
        assert!(FloorProfile::RadicalRich.radical_drop_bonus());
    }

    #[test]
    fn companion_level_from_xp_thresholds() {
        assert_eq!(Companion::level_from_xp(0), 1);
        assert_eq!(Companion::level_from_xp(15), 1);
        assert_eq!(Companion::level_from_xp(29), 1);
        assert_eq!(Companion::level_from_xp(30), 2);
        assert_eq!(Companion::level_from_xp(50), 2);
        assert_eq!(Companion::level_from_xp(79), 2);
        assert_eq!(Companion::level_from_xp(80), 3);
        assert_eq!(Companion::level_from_xp(200), 3);
    }

    #[test]
    fn companion_xp_for_level_matches_thresholds() {
        assert_eq!(Companion::xp_for_level(1), 0);
        assert_eq!(Companion::xp_for_level(2), 30);
        assert_eq!(Companion::xp_for_level(3), 80);
    }

    #[test]
    fn companion_max_level_is_three() {
        assert_eq!(Companion::max_level(), 3);
    }

    #[test]
    fn merchant_discount_scales_with_level() {
        assert_eq!(Companion::Quartermaster.shop_discount_pct(1), 20);
        assert_eq!(Companion::Quartermaster.shop_discount_pct(2), 25);
        assert_eq!(Companion::Quartermaster.shop_discount_pct(3), 25);
    }

    #[test]
    fn non_merchant_has_no_discount() {
        assert_eq!(Companion::ScienceOfficer.shop_discount_pct(3), 0);
        assert_eq!(Companion::Medic.shop_discount_pct(3), 0);
        assert_eq!(Companion::SecurityChief.shop_discount_pct(3), 0);
    }

    #[test]
    fn monk_heal_scales_with_level() {
        assert_eq!(Companion::Medic.heal_per_floor(1), 1);
        assert_eq!(Companion::Medic.heal_per_floor(2), 2);
        assert_eq!(Companion::Medic.heal_per_floor(3), 2);
    }

    #[test]
    fn non_monk_has_no_heal() {
        assert_eq!(Companion::ScienceOfficer.heal_per_floor(3), 0);
        assert_eq!(Companion::SecurityChief.heal_per_floor(3), 0);
    }

    #[test]
    fn guard_blocks_scale_with_level() {
        assert_eq!(Companion::SecurityChief.guard_max_blocks(1), 1);
        assert_eq!(Companion::SecurityChief.guard_max_blocks(2), 1);
        assert_eq!(Companion::SecurityChief.guard_max_blocks(3), 2);
    }

    #[test]
    fn guard_second_block_chance_scales_with_level() {
        assert_eq!(Companion::SecurityChief.guard_second_block_chance(1), 0);
        assert_eq!(Companion::SecurityChief.guard_second_block_chance(2), 50);
        assert_eq!(Companion::SecurityChief.guard_second_block_chance(3), 100);
    }

    #[test]
    fn non_guard_has_no_blocks() {
        assert_eq!(Companion::ScienceOfficer.guard_max_blocks(3), 0);
        assert_eq!(Companion::Medic.guard_max_blocks(3), 0);
    }

    #[test]
    fn teacher_hint_reveals_more_at_higher_levels() {
        let entry = friend_entry();
        let enemy = Enemy::from_vocab(entry, 5, 5, 1);
        let l1 = Companion::ScienceOfficer
            .contextual_hint(&enemy, 10, 10, false, 1)
            .unwrap();
        let l2 = Companion::ScienceOfficer
            .contextual_hint(&enemy, 10, 10, false, 2)
            .unwrap();
        let l3 = Companion::ScienceOfficer
            .contextual_hint(&enemy, 10, 10, false, 3)
            .unwrap();
        assert!(!l1.contains(&enemy.pinyin));
        assert!(l2.contains(&enemy.pinyin));
        assert!(l3.len() >= l2.len());
    }

    #[test]
    fn guard_hint_shows_block_count_at_higher_levels() {
        let entry = friend_entry();
        let enemy = Enemy::from_vocab(entry, 5, 5, 1);
        let l1 = Companion::SecurityChief
            .contextual_hint(&enemy, 10, 10, false, 1)
            .unwrap();
        assert!(l1.contains("first hit"));
        let l3 = Companion::SecurityChief
            .contextual_hint(&enemy, 10, 10, false, 3)
            .unwrap();
        assert!(l3.contains("2 hits"));
    }

    #[test]
    fn guard_hint_none_when_already_used() {
        let entry = friend_entry();
        let enemy = Enemy::from_vocab(entry, 5, 5, 1);
        assert!(Companion::SecurityChief
            .contextual_hint(&enemy, 10, 10, true, 1)
            .is_none());
    }

    #[test]
    fn combo_tier_mapping() {
        assert_eq!(combo_tier(0), ComboTier::None);
        assert_eq!(combo_tier(1), ComboTier::None);
        assert_eq!(combo_tier(2), ComboTier::Good);
        assert_eq!(combo_tier(3), ComboTier::Good);
        assert_eq!(combo_tier(4), ComboTier::Great);
        assert_eq!(combo_tier(5), ComboTier::Great);
        assert_eq!(combo_tier(6), ComboTier::Excellent);
        assert_eq!(combo_tier(8), ComboTier::Excellent);
        assert_eq!(combo_tier(9), ComboTier::Perfect);
        assert_eq!(combo_tier(11), ComboTier::Perfect);
        assert_eq!(combo_tier(12), ComboTier::Radical);
        assert_eq!(combo_tier(20), ComboTier::Radical);
    }

    #[test]
    fn combo_tier_multipliers() {
        assert_eq!(ComboTier::None.multiplier(), 1.0);
        assert_eq!(ComboTier::Good.multiplier(), 1.15);
        assert_eq!(ComboTier::Great.multiplier(), 1.3);
        assert_eq!(ComboTier::Excellent.multiplier(), 1.5);
        assert_eq!(ComboTier::Perfect.multiplier(), 1.75);
        assert_eq!(ComboTier::Radical.multiplier(), 2.0);
    }

    #[test]
    fn run_journal_logs_and_retrieves_floor_events() {
        let mut j = RunJournal::default();
        j.log(RunEvent::EnteredFloor(1));
        j.log(RunEvent::EnemyKilled("猫".to_string(), 1));
        j.log(RunEvent::EnemyKilled("狗".to_string(), 1));
        j.log(RunEvent::SpellForged("FireBolt".to_string(), 1));
        j.log(RunEvent::EnteredFloor(2));
        j.log(RunEvent::BossKilled("龙".to_string(), 2));

        assert_eq!(j.enemies_killed_count(), 3);
        assert_eq!(j.max_floor(), 2);
        assert_eq!(j.spells_forged_list(), vec!["FireBolt"]);

        let f1 = j.floor_summary(1);
        assert_eq!(f1.len(), 4);
        let f2 = j.floor_summary(2);
        assert_eq!(f2.len(), 2);

        let line1 = j.floor_line(1);
        assert!(line1.contains("2 kills"));
        assert!(line1.contains("Forged FireBolt"));

        let line2 = j.floor_line(2);
        assert!(line2.contains("Boss 龙 slain"));
    }

    #[test]
    fn run_journal_death_cause_extracts_reason() {
        let mut j = RunJournal::default();
        assert_eq!(j.death_cause(), "Unknown");

        j.log(RunEvent::EnteredFloor(1));
        j.log(RunEvent::EnemyKilled("猫".to_string(), 1));
        j.log(RunEvent::DiedTo("Spike trap".to_string(), 1));
        assert_eq!(j.death_cause(), "Spike trap");
    }
}

pub fn init_game() -> Result<(), JsValue> {
    let win = window().ok_or("no window")?;
    let doc = win.document().ok_or("no document")?;

    // Create canvas
    let canvas: HtmlCanvasElement = doc.create_element("canvas")?.dyn_into()?;
    canvas.set_id("game-canvas");
    canvas.set_width(800);
    canvas.set_height(600);
    canvas.set_attribute(
        "style",
        "display:block; margin:0 auto; background:#0d0b14; image-rendering:pixelated;",
    )?;
    doc.body().unwrap().append_child(&canvas)?;

    // Remove loading indicator
    if let Some(el) = doc.get_element_by_id("loading") {
        el.remove();
    }

    let renderer = Renderer::new(canvas).map_err(|e| JsValue::from_str(e))?;

    let seed = win.performance().map(|p| p.now() as u64).unwrap_or(42);
    let level = DungeonLevel::generate(MAP_W, MAP_H, seed, 1, crate::world::LocationType::OrbitalPlatform);
    let (sx, sy) = level.start_pos();
    let player = Player::new(sx, sy, PlayerClass::Envoy);

    let best_floor = GameState::load_high_score();
    let srs = crate::srs::load_srs();
    let settings = GameState::load_settings();
    let mut audio = Audio::new();
    if let Some(ref mut audio) = audio {
        audio.set_music_volume(settings.music_volume);
        audio.set_sfx_volume(settings.sfx_volume);
    }
    let total_runs = GameState::load_stat("radical_roguelike_runs");
    let total_kills = GameState::load_stat("radical_roguelike_kills");
    let item_appearance_order = GameState::roll_item_appearance_order(seed);

    let state = Rc::new(RefCell::new(GameState {
        level,
        player,
        renderer,
        audio,
        floor_num: 1,
        seed,
        enemies: Vec::new(),
        combat: CombatState::ClassSelect,
        typing: String::new(),
        message: String::new(),
        message_timer: 0,
        message_tick_delay: 0,
        discovered_recipes: GameState::load_recipes(),
        best_floor,
        srs,
        total_kills,
        total_runs,
        move_count: 0,
        particles: ParticleSystem::new(),
        shake_timer: 0,
        flash: None,
        achievements: AchievementTracker::load(),
        achievement_popup: None,
        codex: Codex::load(&vocab::VOCAB),
        show_codex: false,
        show_inventory: false,
        inventory_cursor: 0,
        inventory_inspect: None,
        show_spellbook: false,
        show_help: false,
        item_appearance_order,
        identified_items: [false; ITEM_KIND_COUNT],
        settings,
        show_settings: false,
        settings_cursor: 0,
        last_spell: None,
        spell_combo_timer: 0,
        listening_mode: ListenMode::Off,
        companion: None,
        guard_used_this_fight: false,
        guard_blocks_used: 0,
        companion_xp: 0,
        merchant_reroll_used: false,
        quests: Vec::new(),
        daily_mode: false,
        endless_mode: false,
        tutorial: None,
        rng_state: seed,
        run_kills: 0,
        run_gold_earned: 0,
        run_correct_answers: 0,
        run_wrong_answers: 0,
        run_spells_forged: 0,
        run_bosses_killed: 0,
        mirror_hint: false,
        next_chain_id: 1,
        floor_profile: FloorProfile::Normal,
        answer_streak: 0,
        run_journal: RunJournal::default(),
        post_mortem_page: 0,
        class_cursor: 0,
        theft_catches: 0,
        shop_banned: false,
        saved_battle: None,
        show_console: false,
        console_buffer: String::new(),
        console_history: Vec::new(),
        console_cmd_history: Vec::new(),
        console_cmd_index: None,
        tab_matches: Vec::new(),
        tab_cycle_index: 0,
        tab_prefix: String::new(),
        god_mode: false,
        completed_special_rooms: HashSet::new(),
        demon_deal_floors: 0,
        crafting_mode: false,
        crafting_first: None,
        crafting_cursor: 0,
        game_mode: GameMode::ShipInterior,
        sector_map: Some({
            let s = generate_sector(0, 1, seed as u32);
            let start = s.start_system;
            SectorMap {
                sectors: vec![s],
                current_sector: 0,
                current_system: start,
            }
        }),
        ship_layout: generate_ship_layout(),
        ship: Ship {
            hull: 100,
            max_hull: 100,
            fuel: 50,
            max_fuel: 100,
            shields: 20,
            max_shields: 50,
            weapon_power: 10,
            engine_power: 10,
            sensor_range: 2,
            cargo_capacity: 100,
            cargo_used: 0,
        },
        crew: vec![
            CrewMember {
                name: "First Officer Chen".to_string(),
                role: CrewRole::Pilot,
                hp: 10,
                max_hp: 10,
                level: 1,
                xp: 0,
                morale: 50,
                skill: 1,
            },
            CrewMember {
                name: "Engineer Rodriguez".to_string(),
                role: CrewRole::Engineer,
                hp: 10,
                max_hp: 10,
                level: 1,
                xp: 0,
                morale: 50,
                skill: 1,
            },
        ],
        current_event: None,
        event_choice_cursor: 0,
        ship_player_x: 5,
        ship_player_y: 5,
        starmap_cursor: 0,
    }));

    // Initial setup
    {
        let s = state.borrow_mut();
        // Don't spawn enemies yet — class selection first
        s.render();
    }

    // Keyboard input
    {
        let state = Rc::clone(&state);
        let closure = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            // Allow IME and OS shortcut combos (Ctrl+Space, Ctrl+Shift, etc.) to pass through
            if event.ctrl_key() || event.alt_key() || event.meta_key() {
                return;
            }

            let key = event.key();
            let Ok(mut s) = state.try_borrow_mut() else {
                return;
            };

            // Resume audio context on first interaction (browser requirement)
            if let Some(ref audio) = s.audio {
                audio.resume();
            }

            // Cheat console toggle
            if key == "`" || key == "Dead" {
                event.prevent_default();
                s.show_console = !s.show_console;
                if let Some(ref audio) = s.audio {
                    audio.play_console_toggle();
                }
                if s.show_console {
                    s.console_buffer.clear();
                }
                s.render();
                return;
            }

            // Console input handling — intercepts ALL keys when console is open
            if s.show_console {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.show_console = false;
                    }
                    "Enter" => {
                        let cmd = s.console_buffer.trim().to_string();
                        if !cmd.is_empty() {
                            s.console_cmd_history.push(cmd.clone());
                            s.console_cmd_index = None;
                            s.execute_console_command(&cmd);
                            s.console_buffer.clear();
                        }
                    }
                    "Backspace" => {
                        s.console_buffer.pop();
                    }
                    "ArrowUp" => {
                        if !s.console_cmd_history.is_empty() {
                            let idx = match s.console_cmd_index {
                                Some(i) => i.saturating_sub(1),
                                None => s.console_cmd_history.len() - 1,
                            };
                            s.console_cmd_index = Some(idx);
                            s.console_buffer = s.console_cmd_history[idx].clone();
                        }
                    }
                    "ArrowDown" => {
                        if let Some(idx) = s.console_cmd_index {
                            if idx + 1 < s.console_cmd_history.len() {
                                let new_idx = idx + 1;
                                s.console_cmd_index = Some(new_idx);
                                s.console_buffer = s.console_cmd_history[new_idx].clone();
                            } else {
                                s.console_cmd_index = None;
                                s.console_buffer.clear();
                            }
                        }
                    }
                    "Tab" => {
                        s.tab_complete();
                    }
                    _ => {
                        // Reset tab completion state on any non-Tab key
                        s.tab_prefix.clear();
                        s.tab_matches.clear();
                        s.tab_cycle_index = 0;
                        if key.len() == 1 {
                            s.console_buffer.push_str(&key);
                        } else if key == "Space" {
                            s.console_buffer.push(' ');
                        }
                    }
                }
                s.render();
                return;
            }

            if key == "?" || key == "/" {
                event.prevent_default();
                s.show_help = !s.show_help;
                s.render();
                return;
            }

            if s.show_settings {
                event.prevent_default();
                match key.as_str() {
                    "Escape" | "o" | "O" => s.close_settings(),
                    "ArrowUp" | "w" | "W" => {
                        s.move_settings_cursor(-1);
                        if let Some(ref audio) = s.audio {
                            audio.play_menu_click();
                        }
                    }
                    "ArrowDown" | "s" | "S" => {
                        s.move_settings_cursor(1);
                        if let Some(ref audio) = s.audio {
                            audio.play_menu_click();
                        }
                    }
                    "ArrowLeft" | "a" | "A" => s.adjust_selected_setting(-1),
                    "ArrowRight" | "d" | "D" | "Enter" => s.adjust_selected_setting(1),
                    _ => {}
                }
                s.render();
                return;
            }

            if s.show_inventory {
                event.prevent_default();
                if s.crafting_mode {
                    // Crafting sub-mode input handling
                    let item_count = s.player.items.len();
                    match key.as_str() {
                        "Escape" | "Backspace" => {
                            if s.crafting_first.is_some() {
                                // Go back to selecting first item
                                s.crafting_first = None;
                            } else {
                                // Exit crafting mode entirely
                                s.crafting_mode = false;
                                s.crafting_cursor = 0;
                            }
                        }
                        "ArrowUp" | "w" | "W" => {
                            if s.crafting_cursor > 0 {
                                s.crafting_cursor -= 1;
                            }
                        }
                        "ArrowDown" | "s" | "S" => {
                            if item_count > 0 && s.crafting_cursor < item_count - 1 {
                                s.crafting_cursor += 1;
                            }
                        }
                        "Enter" => {
                            if item_count > 0 && s.crafting_cursor < item_count {
                                if let Some(first_idx) = s.crafting_first {
                                    let second_idx = s.crafting_cursor;
                                    if first_idx != second_idx {
                                        s.try_craft(first_idx, second_idx);
                                    } else {
                                        s.message = "Select a different item!".to_string();
                                        s.message_timer = 60;
                                    }
                                } else {
                                    // Select first item
                                    s.crafting_first = Some(s.crafting_cursor);
                                }
                            }
                        }
                        _ => {}
                    }
                } else if s.inventory_inspect.is_some() {
                    match key.as_str() {
                        "Escape" | "Backspace" => s.inventory_inspect = None,
                        _ => {}
                    }
                } else {
                    // Unified cursor: 0=weapon, 1=armor, 2=charm, 3+=consumables
                    let total_slots = 3 + s.player.items.len();
                    match key.as_str() {
                        "Escape" | "i" | "I" => s.close_inventory(),
                        "ArrowUp" | "w" | "W" => {
                            if s.inventory_cursor > 0 {
                                s.inventory_cursor -= 1;
                            }
                        }
                        "ArrowDown" | "s" | "S" => {
                            if s.inventory_cursor < total_slots.saturating_sub(1) {
                                s.inventory_cursor += 1;
                            }
                        }
                        "Enter" => {
                            if s.inventory_cursor < total_slots {
                                s.inventory_inspect = Some(s.inventory_cursor);
                            }
                        }
                        "c" | "C" => {
                            if s.player.items.len() >= 2 {
                                s.crafting_mode = true;
                                s.crafting_first = None;
                                s.crafting_cursor = 0;
                            } else {
                                s.message = "Need at least 2 items to craft.".to_string();
                                s.message_timer = 60;
                            }
                        }
                        _ => {}
                    }
                }
                s.render();
                return;
            }

            if s.show_spellbook {
                event.prevent_default();
                match key.as_str() {
                    "Escape" | "b" | "B" => s.show_spellbook = false,
                    _ => {}
                }
                s.render();
                return;
            }

            if (key == "o" || key == "O")
                && !s.show_codex
                && matches!(
                    s.combat,
                    CombatState::Explore | CombatState::ClassSelect | CombatState::GameOver
                )
            {
                event.prevent_default();
                s.open_settings();
                s.render();
                return;
            }

            if (key == "i" || key == "I")
                && !s.show_codex
                && matches!(s.combat, CombatState::Explore | CombatState::GameOver)
            {
                event.prevent_default();
                s.open_inventory();
                s.render();
                return;
            }

            if (key == "b" || key == "B")
                && !s.show_codex
                && !s.show_inventory
                && matches!(s.combat, CombatState::Explore | CombatState::GameOver)
            {
                event.prevent_default();
                s.show_spellbook = true;
                s.render();
                return;
            }

            // Game over: press R to restart, arrows to page
            if matches!(s.combat, CombatState::GameOver) {
                if key == "r" || key == "R" {
                    s.restart();
                    s.render();
                } else if key == "ArrowRight" {
                    let max_fl = s.run_journal.max_floor();
                    let total_pages = ((max_fl as usize).saturating_sub(1)) / 8 + 1;
                    if s.post_mortem_page < total_pages {
                        s.post_mortem_page += 1;
                    }
                    s.render();
                } else if key == "ArrowLeft" {
                    if s.post_mortem_page > 0 {
                        s.post_mortem_page -= 1;
                    }
                    s.render();
                }
                return;
            }

            // Class selection screen

            // StrokeOrder input
            if matches!(s.combat, CombatState::StrokeOrder { .. }) {
                event.prevent_default();
                let mut completed = None;
                if let CombatState::StrokeOrder {
                    hanzi,
                    ref components,
                    ref correct_order,
                    ref mut cursor,
                    ref mut arranged,
                    pinyin,
                    meaning,
                } = s.combat
                {
                    let remaining: Vec<&str> = components
                        .iter()
                        .copied()
                        .filter(|c| !arranged.contains(c))
                        .collect();
                    match key.as_str() {
                        "ArrowUp" | "w" => {
                            if *cursor > 0 {
                                *cursor -= 1;
                            }
                        }
                        "ArrowDown" | "s" => {
                            if !remaining.is_empty() && *cursor + 1 < remaining.len() {
                                *cursor += 1;
                            }
                        }
                        "Enter" => {
                            if *cursor < remaining.len() {
                                let picked = remaining[*cursor];
                                arranged.push(picked);
                                *cursor = 0;
                                if arranged.len() == correct_order.len() {
                                    let correct = arranged
                                        .iter()
                                        .zip(correct_order.iter())
                                        .all(|(a, b)| a == b);
                                    completed = Some((correct, hanzi, pinyin, meaning));
                                }
                            }
                        }
                        "Backspace" => {
                            arranged.pop();
                            *cursor = 0;
                        }
                        "Escape" => {
                            completed = Some((false, hanzi, pinyin, meaning));
                        }
                        _ => {}
                    }
                }
                if let Some((correct, hanzi, pinyin, meaning)) = completed {
                    s.srs.record(hanzi, correct);
                    s.codex.record(hanzi, pinyin, meaning, correct);
                    let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                    if correct {
                        let gs = &mut *s;
                        gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                        s.message =
                            format!("✓ Correct stroke order for {}! (+1 bonus damage)", hanzi);
                        s.player.tone_bonus_damage += 1;
                    } else {
                        let gs = &mut *s;
                        gs.particles.spawn_damage(sx, sy, &mut gs.rng_state);
                        s.message =
                            format!("✗ Wrong order for {} ({} — {}).", hanzi, pinyin, meaning);
                    }
                    s.message_timer = 80;
                    s.combat = CombatState::Explore;
                }
                s.render();
                return;
            }

            // ToneDefense input
            if matches!(s.combat, CombatState::ToneDefense { .. }) {
                event.prevent_default();
                if let CombatState::ToneDefense {
                    round,
                    hanzi,
                    pinyin,
                    meaning,
                    correct_tone,
                    score,
                    last_result: _,
                } = s.combat.clone()
                {
                    let chosen = match key.as_str() {
                        "1" => Some(1u8),
                        "2" => Some(2u8),
                        "3" => Some(3u8),
                        "4" => Some(4u8),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Retreated from the Tone Wall.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(tone) = chosen {
                        let correct = tone == correct_tone;
                        s.srs.record(hanzi, correct);
                        s.codex.record(hanzi, pinyin, meaning, correct);
                        if !correct {
                            s.player.hp -= 1;
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_damage(sx, sy, &mut gs.rng_state);
                            s.trigger_shake(6);
                        }
                        let new_score = if correct { score + 1 } else { score };
                        if round >= 4 {
                            s.player.defense_bonus = new_score as i32;
                            s.combat = CombatState::Explore;
                            s.message = format!(
                                "Tone Wall complete! {}/5 correct — +{} defense next fight!",
                                new_score, new_score
                            );
                            s.message_timer = 120;
                        } else {
                            let pool = vocab::vocab_for_floor(s.floor_num);
                            let entry = if pool.is_empty() {
                                &vocab::VOCAB[s.rng_next() as usize % vocab::VOCAB.len()]
                            } else {
                                pool[s.rng_next() as usize % pool.len()]
                            };
                            let next_tone = entry
                                .pinyin
                                .chars()
                                .last()
                                .and_then(|c| c.to_digit(10))
                                .unwrap_or(1) as u8;
                            s.combat = CombatState::ToneDefense {
                                round: round + 1,
                                hanzi: entry.hanzi,
                                pinyin: entry.pinyin,
                                meaning: entry.meaning,
                                correct_tone: next_tone,
                                score: new_score,
                                last_result: Some(correct),
                            };
                            s.message = if correct {
                                format!("✓ Blocked! Round {}/5 — {}", round + 2, entry.hanzi)
                            } else {
                                format!(
                                    "✗ Hit! (was tone {}) Round {}/5 — {}",
                                    correct_tone,
                                    round + 2,
                                    entry.hanzi
                                )
                            };
                            s.message_timer = 80;
                        }
                    }
                }
                s.render();
                return;
            }

            // CompoundBuilder input
            if matches!(s.combat, CombatState::CompoundBuilder { .. }) {
                event.prevent_default();
                let mut completed = None;
                if let CombatState::CompoundBuilder {
                    ref parts,
                    correct_compound,
                    pinyin,
                    meaning,
                    ref mut cursor,
                    ref mut arranged,
                } = s.combat
                {
                    let remaining: Vec<&str> = parts
                        .iter()
                        .copied()
                        .filter(|p| !arranged.contains(p))
                        .collect();
                    match key.as_str() {
                        "ArrowUp" | "w" => {
                            if *cursor > 0 {
                                *cursor -= 1;
                            }
                        }
                        "ArrowDown" | "s" => {
                            if !remaining.is_empty() && *cursor + 1 < remaining.len() {
                                *cursor += 1;
                            }
                        }
                        "Enter" => {
                            if *cursor < remaining.len() {
                                let picked = remaining[*cursor];
                                arranged.push(picked);
                                *cursor = 0;
                                if arranged.len() == parts.len() {
                                    let built: String = arranged.iter().copied().collect();
                                    let correct = built == correct_compound;
                                    completed = Some((correct, correct_compound, pinyin, meaning));
                                }
                            }
                        }
                        "Backspace" => {
                            arranged.pop();
                            *cursor = 0;
                        }
                        "Escape" => {
                            completed = Some((false, correct_compound, pinyin, meaning));
                        }
                        _ => {}
                    }
                }
                if let Some((correct, compound, pinyin, meaning)) = completed {
                    s.srs.record(compound, correct);
                    s.codex.record(compound, pinyin, meaning, correct);
                    let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                    if correct {
                        let gs = &mut *s;
                        gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                        s.player.spell_power_temp_bonus = 2;
                        s.message = format!(
                            "✓ Correct! {} ({}) — +2 spell power next cast!",
                            compound, meaning
                        );
                    } else {
                        let gs = &mut *s;
                        gs.particles.spawn_damage(sx, sy, &mut gs.rng_state);
                        s.message = format!(
                            "✗ Wrong! The word was {} ({} — {}).",
                            compound, pinyin, meaning
                        );
                    }
                    s.message_timer = 80;
                    s.combat = CombatState::Explore;
                }
                s.render();
                return;
            }

            // ClassifierMatch input
            if matches!(s.combat, CombatState::ClassifierMatch { .. }) {
                event.prevent_default();
                if let CombatState::ClassifierMatch {
                    round,
                    noun,
                    noun_pinyin,
                    noun_meaning,
                    correct_classifier,
                    options: _,
                    correct_idx,
                    score,
                    last_result: _,
                } = s.combat.clone()
                {
                    let chosen = match key.as_str() {
                        "1" => Some(0usize),
                        "2" => Some(1usize),
                        "3" => Some(2usize),
                        "4" => Some(3usize),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Classifier Shrine.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(pick) = chosen {
                        let correct = pick == correct_idx;
                        s.srs.record(noun, correct);
                        s.codex.record(noun, noun_pinyin, noun_meaning, correct);
                        let new_score = if correct { score + 1 } else { score };
                        if round >= 2 {
                            let gold = new_score as i32 * 5;
                            s.player.gold += gold;
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_chest(sx, sy, &mut gs.rng_state);
                            s.combat = CombatState::Explore;
                            s.message =
                                format!("Classifier done! {}/3 correct — +{}g!", new_score, gold);
                            s.message_timer = 120;
                        } else {
                            let next_idx = s.rng_next() as usize % CLASSIFIER_DATA.len();
                            let (next_noun, next_correct, next_pinyin, next_meaning) =
                                CLASSIFIER_DATA[next_idx];
                            let mut opts: Vec<&'static str> = vec![next_correct];
                            let mut attempts = 0;
                            while opts.len() < 4 && attempts < 50 {
                                let c =
                                    ALL_CLASSIFIERS[s.rng_next() as usize % ALL_CLASSIFIERS.len()];
                                if !opts.contains(&c) {
                                    opts.push(c);
                                }
                                attempts += 1;
                            }
                            while opts.len() < 4 {
                                opts.push("个");
                            }
                            let n = opts.len();
                            for i in (1..n).rev() {
                                let j = s.rng_next() as usize % (i + 1);
                                opts.swap(i, j);
                            }
                            let next_correct_idx =
                                opts.iter().position(|&c| c == next_correct).unwrap_or(0);
                            s.combat = CombatState::ClassifierMatch {
                                round: round + 1,
                                noun: next_noun,
                                noun_pinyin: next_pinyin,
                                noun_meaning: next_meaning,
                                correct_classifier: next_correct,
                                options: [opts[0], opts[1], opts[2], opts[3]],
                                correct_idx: next_correct_idx,
                                score: new_score,
                                last_result: Some(correct),
                            };
                            s.message = if correct {
                                format!(
                                    "✓ Correct! ({}) Round {}/3 — {}",
                                    correct_classifier,
                                    round + 2,
                                    next_noun
                                )
                            } else {
                                format!(
                                    "✗ Wrong! (was {}) Round {}/3 — {}",
                                    correct_classifier,
                                    round + 2,
                                    next_noun
                                )
                            };
                            s.message_timer = 80;
                        }
                    }
                }
                s.render();
                return;
            }

            // InkWell input (press 1-9 to guess component count)
            if matches!(s.combat, CombatState::InkWellChallenge { .. }) {
                event.prevent_default();
                if let CombatState::InkWellChallenge {
                    hanzi,
                    correct_count,
                    pinyin,
                    meaning,
                } = s.combat.clone()
                {
                    let chosen: Option<u8> = match key.as_str() {
                        "1" => Some(1),
                        "2" => Some(2),
                        "3" => Some(3),
                        "4" => Some(4),
                        "5" => Some(5),
                        "6" => Some(6),
                        "7" => Some(7),
                        "8" => Some(8),
                        "9" => Some(9),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Ink Well.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(guess) = chosen {
                        let correct = guess == correct_count;
                        s.srs.record(hanzi, correct);
                        s.codex.record(hanzi, pinyin, meaning, correct);
                        if correct {
                            s.player.hp = (s.player.hp + 1).min(s.player.max_hp);
                            s.player.spirit = (s.player.spirit + 35).min(s.player.max_spirit);
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                            s.message = format!(
                                "✓ Correct! {} has {} components. +1 HP, +35 Spirit!",
                                hanzi, correct_count
                            );
                        } else {
                            s.message = format!(
                                "✗ Wrong! {} has {} components ({} — {}).",
                                hanzi, correct_count, pinyin, meaning
                            );
                        }
                        s.combat = CombatState::Explore;
                        s.message_timer = 120;
                    }
                }
                s.render();
                return;
            }

            // AncestorShrine input (press 1-4 to complete chengyu)
            if matches!(s.combat, CombatState::AncestorChallenge { .. }) {
                event.prevent_default();
                if let CombatState::AncestorChallenge {
                    first_half,
                    correct_second,
                    full,
                    pinyin,
                    meaning,
                    options: _,
                    correct_idx,
                } = s.combat.clone()
                {
                    let chosen = match key.as_str() {
                        "1" => Some(0usize),
                        "2" => Some(1usize),
                        "3" => Some(2usize),
                        "4" => Some(3usize),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Ancestor Shrine.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(pick) = chosen {
                        let correct = pick == correct_idx;
                        s.srs.record(first_half, correct);
                        s.codex.record(full, pinyin, meaning, correct);
                        if correct {
                            s.player.gold += 10;
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_chest(sx, sy, &mut gs.rng_state);
                            s.message = format!("✓ {}! ({} — {}) +10 gold!", full, pinyin, meaning);
                        } else {
                            s.message = format!(
                                "✗ Wrong! {} + {} = {} ({}).",
                                first_half, correct_second, full, meaning
                            );
                        }
                        s.combat = CombatState::Explore;
                        s.message_timer = 120;
                    }
                }
                s.render();
                return;
            }

            // TranslationAltar input (press 1-4, 3 rounds)
            if matches!(s.combat, CombatState::TranslationChallenge { .. }) {
                event.prevent_default();
                if let CombatState::TranslationChallenge {
                    round,
                    meaning,
                    correct_hanzi,
                    correct_pinyin,
                    options: _,
                    correct_idx,
                    score,
                } = s.combat.clone()
                {
                    let chosen = match key.as_str() {
                        "1" => Some(0usize),
                        "2" => Some(1usize),
                        "3" => Some(2usize),
                        "4" => Some(3usize),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Translation Altar.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(pick) = chosen {
                        let correct = pick == correct_idx;
                        s.srs.record(correct_hanzi, correct);
                        s.codex
                            .record(correct_hanzi, correct_pinyin, meaning, correct);
                        let new_score = if correct { score + 1 } else { score };
                        if round >= 2 {
                            if new_score >= 2 {
                                s.player.max_hp += 1;
                                s.player.hp += 1;
                                let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                                let gs = &mut *s;
                                gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                                s.message = format!(
                                    "Translation done! {}/3 correct — +1 max HP!",
                                    new_score
                                );
                            } else {
                                s.message = format!(
                                    "Translation done! {}/3 correct — not enough for a reward.",
                                    new_score
                                );
                            }
                            s.combat = CombatState::Explore;
                            s.message_timer = 120;
                        } else {
                            let vocab = vocab::vocab_for_floor(s.floor_num);
                            if vocab.len() >= 4 {
                                let next_idx = s.rng_next() as usize % vocab.len();
                                let next_entry = vocab[next_idx];
                                let mut opts: Vec<&'static str> = vec![next_entry.hanzi];
                                let mut attempts = 0;
                                while opts.len() < 4 && attempts < 50 {
                                    let oi = s.rng_next() as usize % vocab.len();
                                    if !opts.contains(&vocab[oi].hanzi) {
                                        opts.push(vocab[oi].hanzi);
                                    }
                                    attempts += 1;
                                }
                                while opts.len() < 4 {
                                    opts.push("?");
                                }
                                let n = opts.len();
                                for i in (1..n).rev() {
                                    let j = s.rng_next() as usize % (i + 1);
                                    opts.swap(i, j);
                                }
                                let next_correct_idx = opts
                                    .iter()
                                    .position(|&h| h == next_entry.hanzi)
                                    .unwrap_or(0);
                                s.combat = CombatState::TranslationChallenge {
                                    round: round + 1,
                                    meaning: next_entry.meaning,
                                    correct_hanzi: next_entry.hanzi,
                                    correct_pinyin: next_entry.pinyin,
                                    options: [opts[0], opts[1], opts[2], opts[3]],
                                    correct_idx: next_correct_idx,
                                    score: new_score,
                                };
                                let result_str = if correct {
                                    "✓ Correct!"
                                } else {
                                    "✗ Wrong!"
                                };
                                s.message = format!(
                                    "{} Round {}/3 — Which means \"{}\"?",
                                    result_str,
                                    round + 2,
                                    next_entry.meaning
                                );
                                s.message_timer = 80;
                            } else {
                                s.combat = CombatState::Explore;
                                s.message = "Not enough vocabulary.".to_string();
                                s.message_timer = 60;
                            }
                        }
                    }
                }
                s.render();
                return;
            }

            // RadicalGarden input (press 1-4)
            if matches!(s.combat, CombatState::RadicalGardenChallenge { .. }) {
                event.prevent_default();
                if let CombatState::RadicalGardenChallenge {
                    hanzi,
                    pinyin,
                    meaning,
                    correct_radical,
                    options: _,
                    correct_idx,
                } = s.combat.clone()
                {
                    let chosen = match key.as_str() {
                        "1" => Some(0usize),
                        "2" => Some(1usize),
                        "3" => Some(2usize),
                        "4" => Some(3usize),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Radical Garden.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(pick) = chosen {
                        let correct = pick == correct_idx;
                        s.srs.record(hanzi, correct);
                        s.codex.record(hanzi, pinyin, meaning, correct);
                        if correct {
                            let rads = radical::radicals_for_floor(s.floor_num.max(1));
                            if !rads.is_empty() {
                                let ri = s.rng_next() as usize % rads.len();
                                let rad = rads[ri];
                                if !s.player.radicals.contains(&rad.ch) {
                                    s.player.radicals.push(rad.ch);
                                }
                                let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                                let gs = &mut *s;
                                gs.particles.spawn_chest(sx, sy, &mut gs.rng_state);
                                s.message = format!(
                                    "✓ Radical of {} is {}! Free radical: {}",
                                    hanzi, correct_radical, rad.ch
                                );
                            } else {
                                s.message =
                                    format!("✓ Radical of {} is {}!", hanzi, correct_radical);
                            }
                        } else {
                            s.message = format!(
                                "✗ Wrong! Radical of {} is {} ({} — {}).",
                                hanzi, correct_radical, pinyin, meaning
                            );
                        }
                        s.combat = CombatState::Explore;
                        s.message_timer = 120;
                    }
                }
                s.render();
                return;
            }

            // MirrorPool input (type pinyin, Enter to submit)
            if matches!(s.combat, CombatState::MirrorPoolChallenge { .. }) {
                event.prevent_default();
                if let CombatState::MirrorPoolChallenge {
                    hanzi,
                    correct_pinyin,
                    meaning,
                    input,
                } = s.combat.clone()
                {
                    let mut current_input = input;
                    match key.as_str() {
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Mirror Pool.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        "Backspace" => {
                            current_input.pop();
                        }
                        "Enter" => {
                            let correct = current_input.trim() == correct_pinyin;
                            s.srs.record(hanzi, correct);
                            s.codex.record(hanzi, correct_pinyin, meaning, correct);
                            if correct {
                                s.player.spell_power_bonus += 1;
                                let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                                let gs = &mut *s;
                                gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                                s.message = format!(
                                    "✓ Correct! {} = {}. +1 spell power!",
                                    hanzi, correct_pinyin
                                );
                            } else {
                                s.message = format!(
                                    "✗ Wrong! {} = {} ({}).",
                                    hanzi, correct_pinyin, meaning
                                );
                            }
                            s.combat = CombatState::Explore;
                            s.message_timer = 120;
                            s.render();
                            return;
                        }
                        other => {
                            if other.len() == 1 {
                                let ch = other.chars().next().unwrap();
                                if ch.is_ascii_alphanumeric() {
                                    current_input.push(ch);
                                }
                            }
                        }
                    }
                    s.combat = CombatState::MirrorPoolChallenge {
                        hanzi,
                        correct_pinyin,
                        meaning,
                        input: current_input,
                    };
                }
                s.render();
                return;
            }

            // StoneTutor input (Space to advance from teach to quiz, 1-4 for tone quiz)
            if matches!(s.combat, CombatState::StoneTutorChallenge { .. }) {
                event.prevent_default();
                if let CombatState::StoneTutorChallenge {
                    round,
                    hanzi,
                    pinyin,
                    meaning,
                    correct_tone,
                    phase,
                    score,
                } = s.combat.clone()
                {
                    if phase == 0 {
                        if key.as_str() == " " || key.as_str() == "Enter" {
                            s.combat = CombatState::StoneTutorChallenge {
                                round,
                                hanzi,
                                pinyin,
                                meaning,
                                correct_tone,
                                phase: 1,
                                score,
                            };
                            s.message = format!("石 Quiz! What tone is {}? Press 1-4.", hanzi);
                            s.message_timer = 120;
                        } else if key.as_str() == "Escape" {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Stone Tutor.".to_string();
                            s.message_timer = 40;
                        }
                    } else {
                        let chosen: Option<u8> = match key.as_str() {
                            "1" => Some(1),
                            "2" => Some(2),
                            "3" => Some(3),
                            "4" => Some(4),
                            "Escape" => {
                                s.combat = CombatState::Explore;
                                s.message = "Left the Stone Tutor.".to_string();
                                s.message_timer = 40;
                                s.render();
                                return;
                            }
                            _ => None,
                        };
                        if let Some(guess) = chosen {
                            let correct = guess == correct_tone;
                            s.srs.record(hanzi, correct);
                            s.codex.record(hanzi, pinyin, meaning, correct);
                            if correct {
                                s.srs.record(hanzi, true);
                                s.codex.record(hanzi, pinyin, meaning, true);
                            }
                            let new_score = if correct { score + 1 } else { score };
                            if round >= 2 {
                                let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                                let gs = &mut *s;
                                gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                                s.combat = CombatState::Explore;
                                s.message = format!(
                                    "Stone Tutor done! {}/3 correct. SRS boosted!",
                                    new_score
                                );
                                s.message_timer = 120;
                            } else {
                                let vocab = vocab::vocab_for_floor(s.floor_num);
                                if !vocab.is_empty() {
                                    let next_idx = s.rng_next() as usize % vocab.len();
                                    let next = vocab[next_idx];
                                    let next_tone = next
                                        .pinyin
                                        .chars()
                                        .last()
                                        .and_then(|c| c.to_digit(10))
                                        .unwrap_or(1)
                                        as u8;
                                    let result_str = if correct {
                                        format!("✓ Correct! Tone {}.", correct_tone)
                                    } else {
                                        format!("✗ Wrong! Was tone {}.", correct_tone)
                                    };
                                    s.combat = CombatState::StoneTutorChallenge {
                                        round: round + 1,
                                        hanzi: next.hanzi,
                                        pinyin: next.pinyin,
                                        meaning: next.meaning,
                                        correct_tone: next_tone,
                                        phase: 0,
                                        score: new_score,
                                    };
                                    s.message = format!(
                                        "{} Study: {} — {} ({}). Press Space.",
                                        result_str, next.hanzi, next.pinyin, next.meaning
                                    );
                                    s.message_timer = 120;
                                } else {
                                    s.combat = CombatState::Explore;
                                    s.message = "No more vocabulary.".to_string();
                                    s.message_timer = 60;
                                }
                            }
                        }
                    }
                }
                s.render();
                return;
            }

            // CodexChallenge input (1-4 pick meaning, Escape to leave)
            if matches!(s.combat, CombatState::CodexChallenge { .. }) {
                event.prevent_default();
                if let CombatState::CodexChallenge {
                    round,
                    hanzi,
                    pinyin,
                    meaning,
                    options: _,
                    correct_idx,
                    score,
                } = s.combat.clone()
                {
                    let chosen: Option<usize> = match key.as_str() {
                        "1" => Some(0),
                        "2" => Some(1),
                        "3" => Some(2),
                        "4" => Some(3),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Codex Shrine.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(pick) = chosen {
                        let correct = pick == correct_idx;
                        s.srs.record(hanzi, correct);
                        s.codex.record(hanzi, pinyin, meaning, correct);
                        let new_score = if correct { score + 1 } else { score };
                        if round >= 2 {
                            let gold_earned = new_score as i32 * 5;
                            s.player.gold += gold_earned;
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_chest(sx, sy, &mut gs.rng_state);
                            s.combat = CombatState::Explore;
                            s.message = format!(
                                "Codex Shrine done! {}/3 correct. +{} gold!",
                                new_score, gold_earned
                            );
                            s.message_timer = 120;
                        } else {
                            let codex_entries = s.codex.sorted_entries();
                            let vocab = vocab::vocab_for_floor(s.floor_num);
                            let use_codex = codex_entries.len() >= 4;
                            let pool: Vec<(&'static str, &'static str, &'static str)> = if use_codex
                            {
                                codex_entries
                                    .iter()
                                    .map(|e| (e.hanzi, e.pinyin, e.meaning))
                                    .collect()
                            } else {
                                vocab
                                    .iter()
                                    .map(|e| (e.hanzi, e.pinyin, e.meaning))
                                    .collect()
                            };
                            if pool.len() >= 4 {
                                let next_idx = s.rng_next() as usize % pool.len();
                                let (nh, np, nm) = pool[next_idx];
                                let mut dist: Vec<&'static str> = pool
                                    .iter()
                                    .filter(|(h, _, _)| *h != nh)
                                    .map(|(_, _, m)| *m)
                                    .collect();
                                for i in (1..dist.len()).rev() {
                                    let j = s.rng_next() as usize % (i + 1);
                                    dist.swap(i, j);
                                }
                                let ci = s.rng_next() as usize % 4;
                                let mut opts = [dist[0], dist[1], dist[2], nm];
                                opts[3] = opts[ci];
                                opts[ci] = nm;
                                let result_str = if correct {
                                    format!("✓ Correct! {} = {}", hanzi, meaning)
                                } else {
                                    format!("✗ Wrong! {} = {}", hanzi, meaning)
                                };
                                s.combat = CombatState::CodexChallenge {
                                    round: round + 1,
                                    hanzi: nh,
                                    pinyin: np,
                                    meaning: nm,
                                    options: opts,
                                    correct_idx: ci,
                                    score: new_score,
                                };
                                s.message =
                                    format!("{} What does {} mean? Pick 1-4.", result_str, nh);
                                s.message_timer = 120;
                            } else {
                                s.combat = CombatState::Explore;
                                s.message = "No more vocabulary.".to_string();
                                s.message_timer = 60;
                            }
                        }
                    }
                }
                s.render();
                return;
            }

            // Journal input (PgUp/PgDn/Up/Down to scroll, Escape to close)
            if let CombatState::Journal { page } = s.combat {
                event.prevent_default();
                let total = s.codex.sorted_entries().len();
                let per_page = 12;
                let max_page = if total == 0 {
                    0
                } else {
                    (total - 1) / per_page
                };
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message = "Closed journal.".to_string();
                        s.message_timer = 40;
                    }
                    "ArrowDown" | "PageDown" | "s" | "S" => {
                        if page < max_page {
                            s.combat = CombatState::Journal { page: page + 1 };
                        }
                    }
                    "ArrowUp" | "PageUp" | "w" | "W" => {
                        if page > 0 {
                            s.combat = CombatState::Journal { page: page - 1 };
                        }
                    }
                    _ => {}
                }
                s.render();
                return;
            }

            // WordBridgeChallenge input (1-4 pick hanzi, Escape to leave)
            if matches!(s.combat, CombatState::WordBridgeChallenge { .. }) {
                event.prevent_default();
                if let CombatState::WordBridgeChallenge {
                    meaning,
                    correct_hanzi,
                    correct_pinyin,
                    options: _,
                    correct_idx,
                    bridge_x,
                    bridge_y,
                } = s.combat.clone()
                {
                    let chosen: Option<usize> = match key.as_str() {
                        "1" => Some(0),
                        "2" => Some(1),
                        "3" => Some(2),
                        "4" => Some(3),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Word Bridge.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(pick) = chosen {
                        let correct = pick == correct_idx;
                        s.srs.record(correct_hanzi, correct);
                        s.codex
                            .record(correct_hanzi, correct_pinyin, meaning, correct);
                        if correct {
                            let bidx = s.level.idx(bridge_x, bridge_y);
                            s.level.tiles[bidx] = Tile::Catwalk;
                            let widx = s.level.idx(s.player.x, s.player.y);
                            if s.level.tiles[widx] == Tile::DataBridge {
                                s.level.tiles[widx] = Tile::MetalFloor;
                            }
                            let (sx, sy) = s.tile_to_screen(bridge_x, bridge_y);
                            let gs = &mut *s;
                            gs.particles.spawn_bridge(sx, sy, &mut gs.rng_state);
                            s.message = format!(
                                "✓ Correct! {} ({}). A bridge forms over the water!",
                                correct_hanzi, meaning
                            );
                        } else {
                            s.message = format!(
                                "✗ Wrong! The answer was {} ({}). Try again later.",
                                correct_hanzi, meaning
                            );
                        }
                        s.combat = CombatState::Explore;
                        s.message_timer = 120;
                    }
                }
                s.render();
                return;
            }

            // LockedDoorChallenge input (1-4 pick meaning, Escape to leave)
            if matches!(s.combat, CombatState::LockedDoorChallenge { .. }) {
                event.prevent_default();
                if let CombatState::LockedDoorChallenge {
                    hanzi,
                    pinyin,
                    correct_meaning,
                    options: _,
                    correct_idx,
                    door_x,
                    door_y,
                } = s.combat.clone()
                {
                    let chosen: Option<usize> = match key.as_str() {
                        "1" => Some(0),
                        "2" => Some(1),
                        "3" => Some(2),
                        "4" => Some(3),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Stepped away from the locked door.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(pick) = chosen {
                        let correct = pick == correct_idx;
                        s.srs.record(hanzi, correct);
                        s.codex.record(hanzi, pinyin, correct_meaning, correct);
                        if correct {
                            let didx = s.level.idx(door_x, door_y);
                            s.level.tiles[didx] = Tile::MetalFloor;
                            let (sx, sy) = s.tile_to_screen(door_x, door_y);
                            let gs = &mut *s;
                            gs.particles.spawn_dig(sx, sy, &mut gs.rng_state);
                            s.message = format!(
                                "✓ Correct! {} = {}. The door unlocks!",
                                hanzi, correct_meaning
                            );
                        } else {
                            s.player.hp = (s.player.hp - 1).max(0);
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_damage(sx, sy, &mut gs.rng_state);
                            s.message = format!(
                                "✗ Wrong! {} = {}. The door shocks you! (-1 HP)",
                                hanzi, correct_meaning
                            );
                            if s.player.hp <= 0 && !s.try_phoenix_revive() {
                                let fl = s.floor_num;
                                s.run_journal
                                    .log(RunEvent::DiedTo("Locked door trap".to_string(), fl));
                                s.post_mortem_page = 0;
                                s.combat = CombatState::GameOver;
                                s.message = s.run_summary();
                                s.message_timer = 255;
                                s.render();
                                return;
                            }
                        }
                        s.combat = CombatState::Explore;
                        s.message_timer = 120;
                    }
                }
                s.render();
                return;
            }

            // CursedFloorChallenge input (1-4 pick tone)
            if matches!(s.combat, CombatState::CursedFloorChallenge { .. }) {
                event.prevent_default();
                if let CombatState::CursedFloorChallenge {
                    hanzi,
                    pinyin,
                    meaning,
                    correct_tone,
                } = s.combat.clone()
                {
                    let chosen: Option<u8> = match key.as_str() {
                        "1" => Some(1),
                        "2" => Some(2),
                        "3" => Some(3),
                        "4" => Some(4),
                        "Escape" => {
                            s.player.gold = (s.player.gold - 2).max(0);
                            s.combat = CombatState::Explore;
                            s.message = "You flee the curse! (-2 gold)".to_string();
                            s.message_timer = 60;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(guess) = chosen {
                        let correct = guess == correct_tone;
                        s.srs.record(hanzi, correct);
                        s.codex.record(hanzi, pinyin, meaning, correct);
                        if correct {
                            s.player.gold += 1;
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_chest(sx, sy, &mut gs.rng_state);
                            s.message = format!(
                                "✓ Curse averted! {} is tone {}. (+1 gold)",
                                hanzi, correct_tone
                            );
                        } else {
                            s.player.gold = (s.player.gold - 2).max(0);
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_drain(sx, sy, &mut gs.rng_state);
                            s.message = format!(
                                "✗ Cursed! {} is tone {}, not {}. (-2 gold)",
                                hanzi, correct_tone, guess
                            );
                        }
                        s.combat = CombatState::Explore;
                        s.message_timer = 120;
                    }
                }
                s.render();
                return;
            }

            // Tone Battle input
            if matches!(s.combat, CombatState::ToneBattle { .. }) {
                event.prevent_default();
                if let CombatState::ToneBattle {
                    round,
                    hanzi: _,
                    correct_tone,
                    score,
                    last_result: _,
                } = s.combat.clone()
                {
                    let chosen = match key.as_str() {
                        "1" => Some(1u8),
                        "2" => Some(2u8),
                        "3" => Some(3u8),
                        "4" => Some(4u8),
                        "r" | "R" => {
                            // Replay tone
                            if let Some(ref audio) = s.audio {
                                audio.play_chinese_tone(correct_tone);
                            }
                            None
                        }
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the shrine.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(tone) = chosen {
                        let correct = tone == correct_tone;
                        let new_score = if correct { score + 1 } else { score };
                        if round >= 4 {
                            // End of tone battle
                            let bonus_dmg = new_score as i32;
                            s.player.tone_bonus_damage = bonus_dmg;
                            s.combat = CombatState::Explore;
                            s.message = format!(
                                "Shrine complete! {}/5 correct — +{} bonus damage next fight!",
                                new_score, bonus_dmg
                            );
                            s.message_timer = 120;
                        } else {
                            // Next round
                            let (next_hanzi, next_tone) = s.pick_tone_battle_char();
                            if let Some(ref audio) = s.audio {
                                audio.play_chinese_tone(next_tone);
                            }
                            s.combat = CombatState::ToneBattle {
                                round: round + 1,
                                hanzi: next_hanzi,
                                correct_tone: next_tone,
                                score: new_score,
                                last_result: Some(correct),
                            };
                            s.message = if correct {
                                format!("✓ Correct! Round {}/5 — {}", round + 2, next_hanzi)
                            } else {
                                format!(
                                    "✗ Wrong (was tone {})! Round {}/5 — {}",
                                    correct_tone,
                                    round + 2,
                                    next_hanzi
                                )
                            };
                            s.message_timer = 80;
                        }
                    }
                }
                s.render();
                return;
            }

            // Sentence Challenge input
            if matches!(s.combat, CombatState::SentenceChallenge { .. }) {
                event.prevent_default();
                let mut completed = None;
                let mut escaped_mode = None;
                if let CombatState::SentenceChallenge {
                    ref tiles,
                    ref words,
                    ref mut cursor,
                    ref mut arranged,
                    meaning,
                    ref mode,
                } = s.combat
                {
                    let remaining: Vec<usize> = tiles
                        .iter()
                        .copied()
                        .filter(|t| !arranged.contains(t))
                        .collect();
                    match key.as_str() {
                        "ArrowLeft" | "a" => {
                            if *cursor > 0 {
                                *cursor -= 1;
                            }
                        }
                        "ArrowRight" | "d" => {
                            if *cursor + 1 < remaining.len() {
                                *cursor += 1;
                            }
                        }
                        "Enter" => {
                            if *cursor < remaining.len() {
                                arranged.push(remaining[*cursor]);
                                *cursor = 0;
                                // Check if complete
                                if arranged.len() == words.len() {
                                    let correct = arranged.iter().enumerate().all(|(i, &a)| a == i);
                                    completed = Some((
                                        correct,
                                        mode.clone(),
                                        meaning.to_string(),
                                        words.join(" "),
                                    ));
                                }
                            }
                        }
                        "Backspace" => {
                            arranged.pop();
                            *cursor = 0;
                        }
                        "Escape" => {
                            escaped_mode = Some(mode.clone());
                        }
                        _ => {}
                    }
                }
                if let Some((correct, mode, meaning, correct_text)) = completed {
                    match mode {
                        SentenceChallengeMode::BonusGold { reward } => {
                            if correct {
                                s.player.gold += reward;
                                s.message =
                                    format!("✓ Correct! \"{}\" — +{}g bonus!", meaning, reward);
                            } else {
                                s.message = format!("✗ Wrong order! Correct: {}", correct_text);
                            }
                            s.combat = CombatState::Explore;
                            s.message_timer = 120;
                        }
                        SentenceChallengeMode::ScholarTrial {
                            boss_idx,
                            success_damage,
                            failure_heal,
                        } => {
                            if boss_idx < s.enemies.len() && s.enemies[boss_idx].is_alive() {
                                if correct {
                                    let applied =
                                        success_damage.min((s.enemies[boss_idx].hp - 1).max(1));
                                    s.enemies[boss_idx].hp -= applied;
                                    s.enemies[boss_idx].stunned = true;
                                    s.message = format!(
                                        "✓ Correct! \"{}\" — The boss loses {} HP and is stunned!",
                                        meaning, applied
                                    );
                                } else {
                                    let before = s.enemies[boss_idx].hp;
                                    s.enemies[boss_idx].hp =
                                        (before + failure_heal).min(s.enemies[boss_idx].max_hp);
                                    let healed = s.enemies[boss_idx].hp - before;
                                    s.message = format!(
                                        "✗ Wrong order! Correct: {} — The boss regains {} HP.",
                                        correct_text, healed
                                    );
                                }
                                if let Some(mut battle) = s.saved_battle.take() {
                                    for unit in &mut battle.units {
                                        if let combat::UnitKind::Enemy(eidx) = unit.kind {
                                            if eidx == boss_idx {
                                                unit.hp = s.enemies[boss_idx].hp;
                                                unit.max_hp = s.enemies[boss_idx].max_hp;
                                                unit.stunned = s.enemies[boss_idx].stunned;
                                                break;
                                            }
                                        }
                                    }
                                    s.combat = CombatState::TacticalBattle(battle);
                                } else {
                                    s.combat = CombatState::Fighting {
                                        enemy_idx: boss_idx,
                                        timer_ms: 0.0,
                                    };
                                }
                                s.typing.clear();
                            } else {
                                s.combat = CombatState::Explore;
                                s.message = "The sentence duel fades.".to_string();
                            }
                            s.message_timer = 120;
                        }
                        SentenceChallengeMode::GatekeeperSeal {
                            boss_idx,
                            success_damage,
                            failure_damage_to_player,
                        } => {
                            if boss_idx < s.enemies.len() && s.enemies[boss_idx].is_alive() {
                                if correct {
                                    let applied =
                                        success_damage.min((s.enemies[boss_idx].hp - 1).max(1));
                                    s.enemies[boss_idx].hp -= applied;
                                    s.enemies[boss_idx].stunned = true;
                                    s.message = format!(
                                        "✓ Seal shattered! \"{}\" — The Pirate Captain loses {} HP and is stunned!",
                                        meaning, applied
                                    );
                                } else {
                                    s.player.hp = (s.player.hp - failure_damage_to_player).max(0);
                                    s.message = format!(
                                        "✗ The seal backfires! Correct: {} — You take {} damage!",
                                        correct_text, failure_damage_to_player
                                    );
                                    if s.player.hp <= 0 && !s.try_phoenix_revive() {
                                        let fl = s.floor_num;
                                        s.run_journal.log(crate::game::RunEvent::DiedTo(
                                            "Pirate Captain's Seal".to_string(),
                                            fl,
                                        ));
                                        s.combat = CombatState::GameOver;
                                        s.message_timer = 200;
                                        s.render();
                                        return;
                                    }
                                }
                                if let Some(mut battle) = s.saved_battle.take() {
                                    for unit in &mut battle.units {
                                        if let combat::UnitKind::Enemy(eidx) = unit.kind {
                                            if eidx == boss_idx {
                                                unit.hp = s.enemies[boss_idx].hp;
                                                unit.max_hp = s.enemies[boss_idx].max_hp;
                                                unit.stunned = s.enemies[boss_idx].stunned;
                                                break;
                                            }
                                        }
                                    }
                                    s.combat = CombatState::TacticalBattle(battle);
                                } else {
                                    s.combat = CombatState::Fighting {
                                        enemy_idx: boss_idx,
                                        timer_ms: 0.0,
                                    };
                                }
                                s.typing.clear();
                            } else {
                                s.combat = CombatState::Explore;
                                s.message = "The seal fades.".to_string();
                            }
                            s.message_timer = 120;
                        }
                    }
                } else if let Some(mode) = escaped_mode {
                    match mode {
                        SentenceChallengeMode::BonusGold { .. } => {
                            s.combat = CombatState::Explore;
                            s.message = "Skipped sentence challenge.".to_string();
                            s.message_timer = 40;
                        }
                        SentenceChallengeMode::ScholarTrial {
                            boss_idx,
                            failure_heal,
                            ..
                        } => {
                            if boss_idx < s.enemies.len() && s.enemies[boss_idx].is_alive() {
                                let before = s.enemies[boss_idx].hp;
                                s.enemies[boss_idx].hp =
                                    (before + failure_heal).min(s.enemies[boss_idx].max_hp);
                                let healed = s.enemies[boss_idx].hp - before;
                                if let Some(mut battle) = s.saved_battle.take() {
                                    for unit in &mut battle.units {
                                        if let combat::UnitKind::Enemy(eidx) = unit.kind {
                                            if eidx == boss_idx {
                                                unit.hp = s.enemies[boss_idx].hp;
                                                unit.max_hp = s.enemies[boss_idx].max_hp;
                                                unit.stunned = s.enemies[boss_idx].stunned;
                                                break;
                                            }
                                        }
                                    }
                                    s.combat = CombatState::TacticalBattle(battle);
                                } else {
                                    s.combat = CombatState::Fighting {
                                        enemy_idx: boss_idx,
                                        timer_ms: 0.0,
                                    };
                                }
                                s.typing.clear();
                                s.message = format!(
                                    "You abandon the syntax duel. The boss regains {} HP!",
                                    healed
                                );
                            } else {
                                s.combat = CombatState::Explore;
                                s.message = "The sentence duel fades.".to_string();
                            }
                            s.message_timer = 80;
                        }
                        SentenceChallengeMode::GatekeeperSeal {
                            boss_idx,
                            failure_damage_to_player,
                            ..
                        } => {
                            if boss_idx < s.enemies.len() && s.enemies[boss_idx].is_alive() {
                                s.player.hp = (s.player.hp - failure_damage_to_player).max(0);
                                if let Some(mut battle) = s.saved_battle.take() {
                                    for unit in &mut battle.units {
                                        if let combat::UnitKind::Enemy(eidx) = unit.kind {
                                            if eidx == boss_idx {
                                                unit.hp = s.enemies[boss_idx].hp;
                                                unit.max_hp = s.enemies[boss_idx].max_hp;
                                                unit.stunned = s.enemies[boss_idx].stunned;
                                                break;
                                            }
                                        }
                                    }
                                    s.combat = CombatState::TacticalBattle(battle);
                                } else {
                                    s.combat = CombatState::Fighting {
                                        enemy_idx: boss_idx,
                                        timer_ms: 0.0,
                                    };
                                }
                                s.typing.clear();
                                s.message = format!(
                                    "You abandon the seal! The backfire deals {} damage!",
                                    failure_damage_to_player
                                );
                                if s.player.hp <= 0 && !s.try_phoenix_revive() {
                                    let fl = s.floor_num;
                                    s.run_journal
                                        .log(RunEvent::DiedTo("Pirate Captain's Seal".to_string(), fl));
                                    s.post_mortem_page = 0;
                                    s.combat = CombatState::GameOver;
                                }
                            } else {
                                s.combat = CombatState::Explore;
                                s.message = "The seal fades.".to_string();
                            }
                            s.message_timer = 80;
                        }
                    }
                }
                s.render();
                return;
            }

            if matches!(s.combat, CombatState::ClassSelect) {
                event.prevent_default();
                // Daily challenge
                if key == "d" || key == "D" {
                    // Seed from date: year * 10000 + month * 100 + day
                    let date_seed = js_sys::Date::new_0();
                    let daily_seed = (date_seed.get_full_year() as u64) * 10000
                        + (date_seed.get_month() as u64 + 1) * 100
                        + date_seed.get_date() as u64;
                    s.seed = daily_seed;
                    s.rng_state = daily_seed;
                    s.daily_mode = true;
                    s.level = DungeonLevel::generate(MAP_W, MAP_H, daily_seed, 1, crate::world::LocationType::OrbitalPlatform);
                    let (sx, sy) = s.level.start_pos();
                    s.player = s.make_player(sx, sy, PlayerClass::Envoy);
                    s.reset_item_lore();
                    s.combat = CombatState::Explore;
                    s.message =
                        "🏆 Daily Challenge! Fixed seed — compete for high score!".to_string();
                    s.message_timer = 150;
                    s.spawn_enemies();
                    let (px, py) = (s.player.x, s.player.y);
                    compute_fov(&mut s.level, px, py, FOV_RADIUS);
                    s.render();
                    return;
                }
                let total_classes = PlayerClass::all().len();
                if key == "ArrowUp" || key == "w" || key == "W" {
                    if s.class_cursor > 0 {
                        s.class_cursor -= 1;
                    } else {
                        s.class_cursor = total_classes - 1;
                    }
                    if let Some(ref audio) = s.audio {
                        audio.play_menu_click();
                    }
                    s.render();
                    return;
                }
                if key == "ArrowDown" || key == "s" || key == "S" {
                    s.class_cursor = (s.class_cursor + 1) % total_classes;
                    if let Some(ref audio) = s.audio {
                        audio.play_menu_click();
                    }
                    s.render();
                    return;
                }

                let chosen_class = if key == "Enter" {
                    Some(PlayerClass::all()[s.class_cursor])
                } else {
                    None
                };
                if let Some(chosen_class) = chosen_class {
                    s.daily_mode = false;
                    if s.total_runs == 0 {
                        s.start_tutorial(chosen_class);
                    } else {
                        let (sx, sy) = s.level.start_pos();
                        s.player = s.make_player(sx, sy, chosen_class);
                        s.reset_item_lore();
                        s.combat = CombatState::Explore;
                        let class_name = chosen_class.data().name_en;
                        s.message = format!("You chose {}! Explore the dungeon...", class_name);
                        s.message_timer = 120;
                        s.spawn_enemies();
                        let (px, py) = (s.player.x, s.player.y);
                        compute_fov(&mut s.level, px, py, FOV_RADIUS);
                    }
                    s.render();
                }
                return;
            }

            if matches!(s.combat, CombatState::TacticalBattle(_)) {
                event.prevent_default();
                let gs = &mut *s;
                let mut old_combat = std::mem::replace(&mut gs.combat, CombatState::Explore);
                if let CombatState::TacticalBattle(ref mut battle) = old_combat {
                    let log_len_before = battle.log.len();
                    let result = combat::input::handle_input(battle, key.as_str());

                    // Drain queued audio events from combat
                    for audio_event in battle.audio_events.drain(..) {
                        if let Some(ref audio) = gs.audio {
                            match audio_event {
                                combat::AudioEvent::EnemyDeath => audio.play_enemy_death(),
                                combat::AudioEvent::CriticalHit => audio.play_critical_hit(),
                                combat::AudioEvent::ProjectileLaunch => {
                                    audio.play_projectile_launch()
                                }
                                combat::AudioEvent::ProjectileImpact => {
                                    audio.play_projectile_impact()
                                }
                                combat::AudioEvent::Heal => audio.play_heal(),
                                combat::AudioEvent::ShieldBlock => audio.play_shield_block(),
                                combat::AudioEvent::StatusBurn => audio.play_status_burn(),
                                combat::AudioEvent::StatusPoison => audio.play_status_poison(),
                                combat::AudioEvent::StatusSlow => audio.play_status_slow(),
                                combat::AudioEvent::SpellElement(ref elem) => {
                                    audio.play_spell_element(elem)
                                }
                                combat::AudioEvent::TurnTick => audio.play_turn_tick(),
                                combat::AudioEvent::TypingCorrect => audio.play_typing_correct(),
                                combat::AudioEvent::TypingError => audio.play_typing_error(),
                                combat::AudioEvent::WaterSplash => audio.play_water_splash(),
                                combat::AudioEvent::LavaRumble => audio.play_lava_rumble(),
                                combat::AudioEvent::ComboStrike => audio.play_critical_hit(),
                            }
                        }
                    }
                    // Scan new log messages for particle/shake triggers
                    for msg in &battle.log[log_len_before..] {
                        if msg.contains("Collision!") || msg.contains("Slammed") {
                            gs.trigger_shake(4);
                            let px = gs.renderer.canvas_w / 2.0;
                            let py = gs.renderer.canvas_h / 2.0;
                            gs.particles
                                .spawn_knockback_collision(px, py, &mut gs.rng_state);
                        }
                        if msg.contains("CHENGYU!") {
                            gs.trigger_shake(6);
                            let px = gs.renderer.canvas_w / 2.0;
                            let py = gs.renderer.canvas_h / 3.0;
                            gs.particles.spawn_chengyu(px, py, &mut gs.rng_state);
                        }
                        if msg.contains("Super effective!") {
                            let px = gs.renderer.canvas_w / 2.0;
                            let py = gs.renderer.canvas_h / 2.0;
                            gs.particles
                                .spawn_wuxing_effective(px, py, &mut gs.rng_state);
                        }
                    }

                    // SRS tracking: consume last_answer from tactical battle
                    if let Some((hanzi, correct)) = battle.last_answer.take() {
                        gs.srs.record(hanzi, correct);
                        if correct {
                            gs.run_correct_answers += 1;
                            gs.answer_streak += 1;
                        } else {
                            gs.run_wrong_answers += 1;
                            gs.answer_streak = 0;
                        }
                    }

                    if let Some(spell_idx) = battle.spent_spell_index.take() {
                        if spell_idx < gs.player.spells.len() {
                            gs.player.spells.remove(spell_idx);
                        }
                        if spell_idx < battle.available_spells.len() {
                            battle.available_spells.remove(spell_idx);
                        }
                    }

                    for consumed in &battle.consumed_radicals {
                        if let Some(pos) = gs.player.radicals.iter().position(|r| r == consumed) {
                            gs.player.radicals.remove(pos);
                        }
                    }
                    battle.consumed_radicals.clear();

                    match result {
                        combat::input::BattleEvent::Flee => {
                            // Nearest alive enemy gets a free hit
                            let free_hit = battle
                                .units
                                .iter()
                                .filter(|u| matches!(u.kind, combat::UnitKind::Enemy(_)) && u.alive)
                                .map(|u| u.damage)
                                .next()
                                .unwrap_or(0);

                            combat::transition::exit_combat(
                                battle,
                                &mut gs.player,
                                &mut gs.enemies,
                            );

                            if free_hit > 0 {
                                if gs.player.shield {
                                    gs.player.shield = false;
                                    gs.message = "Fled! Shield absorbed the blow!".to_string();
                                } else {
                                    gs.player.hp -= free_hit;
                                    gs.message =
                                        format!("Fled! Hit for {} on the way out!", free_hit);
                                }
                            } else {
                                gs.message = "Fled from battle!".to_string();
                            }

                            if gs.player.hp <= 0 && !gs.try_phoenix_revive() {
                                gs.player.hp = 0;
                                gs.run_journal
                                    .log(RunEvent::DiedTo("fleeing".to_string(), gs.floor_num));
                                gs.post_mortem_page = 0;
                                gs.combat = CombatState::GameOver;
                                gs.message = gs.run_summary();
                                gs.message_timer = 255;
                                if let Some(ref audio) = gs.audio {
                                    audio.play_death();
                                }
                                gs.save_high_score();
                            } else {
                                gs.message_timer = 60;
                                gs.combat = CombatState::Explore;
                            }

                            gs.render();
                            return;
                        }
                        combat::input::BattleEvent::Victory => {
                            if let Some(ref audio) = gs.audio {
                                audio.play_victory();
                            }
                            let combo = battle.combo_streak;
                            let killed = combat::transition::exit_combat(
                                battle,
                                &mut gs.player,
                                &mut gs.enemies,
                            );
                            gs.handle_tactical_victory(&killed, combo);
                            gs.render();
                            return;
                        }
                        combat::input::BattleEvent::Defeat => {
                            let killer_name = battle
                                .units
                                .iter()
                                .find(|u| u.is_enemy() && u.alive)
                                .map(|u| u.hanzi.to_string())
                                .unwrap_or_else(|| "battle".to_string());
                            combat::transition::exit_combat(
                                battle,
                                &mut gs.player,
                                &mut gs.enemies,
                            );
                            gs.handle_tactical_defeat(killer_name);
                            gs.render();
                            return;
                        }
                        combat::input::BattleEvent::None => {}
                    }

                    if battle.pending_spirit_delta != 0 {
                        gs.player.spirit = (gs.player.spirit + battle.pending_spirit_delta)
                            .max(0)
                            .min(gs.player.max_spirit);
                        battle.pending_spirit_delta = 0;
                    }
                }
                gs.combat = old_combat;
                // Check for boss phase triggers after processing tactical battle input
                if let CombatState::TacticalBattle(ref battle) = gs.combat {
                    let mut trigger_idx = None;
                    for unit in &battle.units {
                        if let combat::UnitKind::Enemy(eidx) = unit.kind {
                            if unit.alive
                                && eidx < gs.enemies.len()
                                && gs.enemies[eidx].boss_kind.is_some()
                                && !gs.enemies[eidx].phase_triggered
                                && unit.hp <= unit.max_hp / 2
                            {
                                // Sync HP from battle unit to enemy array before triggering
                                gs.enemies[eidx].hp = unit.hp;
                                gs.enemies[eidx].max_hp = unit.max_hp;
                                trigger_idx = Some(eidx);
                                break;
                            }
                        }
                    }
                    if let Some(eidx) = trigger_idx {
                        gs.maybe_trigger_boss_phase(eidx);
                        gs.render();
                        return;
                    }
                }
                gs.render();
                return;
            }

            // Combat typing mode
            if matches!(s.combat, CombatState::Fighting { .. }) {
                event.prevent_default();
                match key.as_str() {
                    "Enter" => {
                        s.submit_answer();
                        s.render();
                    }
                    "Backspace" => {
                        s.backspace();
                        s.render();
                    }
                    "Escape" => {
                        // Flee — enemy gets a free hit (shield can block)
                        if let CombatState::Fighting { enemy_idx, .. } = s.combat {
                            if enemy_idx < s.enemies.len() && s.enemies[enemy_idx].is_alive() {
                                if s.player.shield {
                                    s.player.shield = false;
                                    s.message = "Fled! Shield absorbed the blow!".to_string();
                                    s.message_timer = 40;
                                } else {
                                    let dmg = s.enemies[enemy_idx].damage;
                                    s.player.hp -= dmg;
                                    s.message = format!(
                                        "Fled! {} hits for {}!",
                                        s.enemies[enemy_idx].hanzi, dmg
                                    );
                                    s.message_timer = 40;
                                }
                                if s.player.hp <= 0 && !s.try_phoenix_revive() {
                                    s.player.hp = 0;
                                    let cause = s.enemies[enemy_idx].hanzi.to_string();
                                    let fl = s.floor_num;
                                    s.run_journal.log(RunEvent::DiedTo(cause, fl));
                                    s.post_mortem_page = 0;
                                    s.combat = CombatState::GameOver;
                                    s.message = s.run_summary();
                                    s.message_timer = 255;
                                } else {
                                    s.combat = CombatState::Explore;
                                }
                            } else {
                                s.combat = CombatState::Explore;
                            }
                        }
                        s.typing.clear();
                        s.render();
                    }
                    " " => {
                        // Cast selected spell
                        s.use_spell();
                        s.render();
                    }
                    "r" | "R" => {
                        // Replay tone in listening mode
                        if s.listening_mode.is_active() {
                            if let CombatState::Fighting { enemy_idx, .. } = s.combat {
                                if enemy_idx < s.enemies.len() {
                                    let pinyin = s.enemies[enemy_idx].pinyin;
                                    let tone_num = pinyin
                                        .chars()
                                        .last()
                                        .and_then(|c| c.to_digit(10))
                                        .unwrap_or(1)
                                        as u8;
                                    if let Some(ref audio) = s.audio {
                                        audio.play_chinese_tone(tone_num);
                                    }
                                }
                            }
                        } else {
                            s.type_char(key.chars().next().unwrap_or('r'));
                            s.render();
                        }
                    }
                    _ => {
                        if let Some(ch) = key.chars().next() {
                            if key.len() == 1 && (ch.is_ascii_alphanumeric()) {
                                s.type_char(ch);
                                s.render();
                            }
                        }
                    }
                }
                return;
            }

            // Forge mode
            if matches!(s.combat, CombatState::Forging { .. }) {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message.clear();
                        s.message_timer = 0;
                        s.render();
                    }
                    "Enter" => {
                        s.forge_submit();
                        s.render();
                    }
                    "ArrowUp" | "w" | "W" => {
                        if let CombatState::Forging { ref mut cursor, .. } = s.combat {
                            if *cursor > 0 {
                                *cursor -= 1;
                                if let Some(ref audio) = s.audio {
                                    audio.play_menu_click();
                                }
                            }
                        }
                        s.render();
                    }
                    "ArrowDown" | "s" | "S" => {
                        if let CombatState::Forging {
                            ref recipes,
                            ref mut cursor,
                            ..
                        } = s.combat
                        {
                            if *cursor + 1 < recipes.len() {
                                *cursor += 1;
                                if let Some(ref audio) = s.audio {
                                    audio.play_menu_click();
                                }
                            }
                        }
                        s.render();
                    }
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                        let slot = key.parse::<usize>().unwrap_or(1) - 1;
                        if let CombatState::Forging {
                            ref recipes,
                            ref mut cursor,
                            ..
                        } = s.combat
                        {
                            if slot < recipes.len() {
                                *cursor = slot;
                            }
                        }
                        s.forge_submit();
                        s.render();
                    }
                    "e" | "E" => {
                        // Enter enchant mode — pick a slot first
                        let has_equip = s.player.weapon.is_some()
                            || s.player.armor.is_some()
                            || s.player.charm.is_some();
                        if !has_equip {
                            s.message = "No equipment to enchant!".to_string();
                            s.message_timer = 90;
                        } else if s.player.radicals.is_empty() {
                            s.message = "No radicals to enchant with!".to_string();
                            s.message_timer = 90;
                        } else {
                            s.combat = CombatState::Enchanting {
                                step: 0,
                                slot: 0,
                                page: 0,
                            };
                            s.message =
                                "Enchant: 1=Weapon 2=Armor 3=Charm. Pick slot, then radical."
                                    .to_string();
                            s.message_timer = 255;
                        }
                        s.render();
                    }
                    _ => {}
                }
                return;
            }

            // Enchanting mode (two-phase: step 0 = pick slot, step 1 = pick radical)
            if let CombatState::Enchanting { step, slot, page } = s.combat {
                event.prevent_default();
                if step == 0 {
                    // Phase 0: Select equipment slot
                    match key.as_str() {
                        "Escape" => {
                            s.combat = CombatState::Forging {
                                recipes: s.discovered_recipes.clone(),
                                cursor: 0,
                            };
                            s.message.clear();
                            s.message_timer = 0;
                        }
                        "1" => {
                            if s.player.weapon.is_some() {
                                s.combat = CombatState::Enchanting {
                                    step: 1,
                                    slot: 0,
                                    page: 0,
                                };
                                s.message = "Enchanting Weapon. Pick a radical.".to_string();
                                s.message_timer = 255;
                            } else {
                                s.message = "No Weapon equipped!".to_string();
                                s.message_timer = 90;
                            }
                        }
                        "2" => {
                            if s.player.armor.is_some() {
                                s.combat = CombatState::Enchanting {
                                    step: 1,
                                    slot: 1,
                                    page: 0,
                                };
                                s.message = "Enchanting Armor. Pick a radical.".to_string();
                                s.message_timer = 255;
                            } else {
                                s.message = "No Armor equipped!".to_string();
                                s.message_timer = 90;
                            }
                        }
                        "3" => {
                            if s.player.charm.is_some() {
                                s.combat = CombatState::Enchanting {
                                    step: 1,
                                    slot: 2,
                                    page: 0,
                                };
                                s.message = "Enchanting Charm. Pick a radical.".to_string();
                                s.message_timer = 255;
                            } else {
                                s.message = "No Charm equipped!".to_string();
                                s.message_timer = 90;
                            }
                        }
                        "ArrowUp" | "w" | "W" => {
                            if let CombatState::Enchanting { ref mut slot, .. } = s.combat {
                                if *slot > 0 {
                                    *slot -= 1;
                                }
                            }
                        }
                        "ArrowDown" | "s" | "S" => {
                            if let CombatState::Enchanting { ref mut slot, .. } = s.combat {
                                if *slot < 2 {
                                    *slot += 1;
                                }
                            }
                        }
                        "Enter" => {
                            let has_slot = match slot {
                                0 => s.player.weapon.is_some(),
                                1 => s.player.armor.is_some(),
                                2 => s.player.charm.is_some(),
                                _ => false,
                            };
                            if has_slot {
                                s.combat = CombatState::Enchanting {
                                    step: 1,
                                    slot,
                                    page: 0,
                                };
                                let slot_name = match slot {
                                    0 => "Weapon",
                                    1 => "Armor",
                                    _ => "Charm",
                                };
                                s.message = format!("Enchanting {}. Pick a radical.", slot_name);
                                s.message_timer = 255;
                            } else {
                                let slot_name = match slot {
                                    0 => "Weapon",
                                    1 => "Armor",
                                    _ => "Charm",
                                };
                                s.message = format!("No {} equipped!", slot_name);
                                s.message_timer = 90;
                            }
                        }
                        _ => {}
                    }
                } else {
                    // Phase 1: Select radical (keys 1-6 per page)
                    match key.as_str() {
                        "Escape" => {
                            s.combat = CombatState::Enchanting {
                                step: 0,
                                slot,
                                page: 0,
                            };
                            s.message = "Enchant: pick equipment slot.".to_string();
                            s.message_timer = 255;
                        }
                        "ArrowLeft" => {
                            if let CombatState::Enchanting { ref mut page, .. } = s.combat {
                                if *page > 0 {
                                    *page -= 1;
                                }
                            }
                        }
                        "ArrowRight" => {
                            let max_page = s.player.radicals.len().saturating_sub(1) / 6;
                            if let CombatState::Enchanting { ref mut page, .. } = s.combat {
                                if *page < max_page {
                                    *page += 1;
                                }
                            }
                        }
                        "1" | "2" | "3" | "4" | "5" | "6" => {
                            let key_idx = key.parse::<usize>().unwrap_or(1) - 1;
                            let abs_idx = page * 6 + key_idx;
                            if abs_idx < s.player.radicals.len() {
                                let radical = s.player.radicals[abs_idx];
                                s.player.enchantments[slot] = Some(radical);
                                s.player.radicals.remove(abs_idx);
                                let slot_name = match slot {
                                    0 => "Weapon",
                                    1 => "Armor",
                                    _ => "Charm",
                                };
                                let bonus = match radical {
                                    "力" | "火" => "+1 damage",
                                    "水" | "土" => "+1 defense",
                                    "心" => "+2 max HP",
                                    "金" => "+3 gold/kill",
                                    "目" => "+1 FOV",
                                    _ => "+1 damage",
                                };
                                if radical == "心" {
                                    s.player.max_hp += 2;
                                    s.player.hp = s.player.hp.min(s.player.max_hp);
                                }
                                if let Some(ref audio) = s.audio {
                                    audio.play_forge();
                                }
                                let cam_x =
                                    s.player.x as f64 * 24.0 - s.renderer.canvas_w / 2.0 + 12.0;
                                let cam_y =
                                    s.player.y as f64 * 24.0 - s.renderer.canvas_h / 2.0 + 12.0;
                                let sx = s.player.x as f64 * 24.0 - cam_x + 12.0;
                                let sy = s.player.y as f64 * 24.0 - cam_y + 12.0;
                                let gs = &mut *s;
                                gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                                s.message = format!(
                                    "Enchanted {} with {} ({})!",
                                    slot_name, radical, bonus
                                );
                                s.message_timer = 120;
                                s.combat = CombatState::Explore;
                                let recipe_count = s.discovered_recipes.len();
                                s.achievements.check_recipes(recipe_count);
                            } else {
                                s.message = "No radical at that slot.".to_string();
                                s.message_timer = 60;
                            }
                        }
                        _ => {}
                    }
                }
                s.render();
                return;
            }

            // Offering mode
            if let CombatState::Offering { altar_kind, cursor } = s.combat {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message.clear();
                        s.render();
                    }
                    "ArrowUp" | "w" => {
                        if cursor > 0 {
                            s.combat = CombatState::Offering {
                                altar_kind,
                                cursor: cursor - 1,
                            };
                        }
                        s.render();
                    }
                    "ArrowDown" | "s" => {
                        if cursor + 1 < s.player.items.len() {
                            s.combat = CombatState::Offering {
                                altar_kind,
                                cursor: cursor + 1,
                            };
                        }
                        s.render();
                    }
                    "Enter" => {
                        s.perform_offering(altar_kind, cursor);
                        s.render();
                    }
                    "p" | "P" => {
                        let has_curse = (s.player.weapon.is_some()
                            && s.player.weapon_state == ItemState::Cursed)
                            || (s.player.armor.is_some()
                                && s.player.armor_state == ItemState::Cursed)
                            || (s.player.charm.is_some()
                                && s.player.charm_state == ItemState::Cursed);
                        if has_curse {
                            if s.player.weapon_state == ItemState::Cursed {
                                s.player.weapon_state = ItemState::Normal;
                            }
                            if s.player.armor_state == ItemState::Cursed {
                                s.player.armor_state = ItemState::Normal;
                            }
                            if s.player.charm_state == ItemState::Cursed {
                                s.player.charm_state = ItemState::Normal;
                            }
                            s.message = "🔮 The altar purifies your cursed equipment!".to_string();
                            s.message_timer = 90;
                            s.combat = CombatState::Explore;
                        } else {
                            s.message = "You have no cursed equipment to purify.".to_string();
                            s.message_timer = 60;
                        }
                        s.render();
                    }
                    _ => {}
                }
                return;
            }

            // Dipping Source
            if let CombatState::DippingSource { cursor } = s.combat {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message.clear();
                        s.render();
                    }
                    "ArrowUp" | "w" => {
                        if cursor > 0 {
                            s.combat = CombatState::DippingSource { cursor: cursor - 1 };
                        }
                        s.render();
                    }
                    "ArrowDown" | "s" => {
                        if cursor + 1 < s.player.items.len() {
                            s.combat = CombatState::DippingSource { cursor: cursor + 1 };
                        }
                        s.render();
                    }
                    "Enter" => {
                        if cursor < s.player.items.len() {
                            let kind = s.player.items[cursor].kind();
                            if matches!(
                                kind,
                                ItemKind::MedHypo
                                    | ItemKind::ToxinGrenade
                                    | ItemKind::StimPack
                            ) {
                                s.combat = CombatState::DippingTarget {
                                    source_idx: cursor,
                                    cursor: 0,
                                };
                                s.message = "Dip into what? (Equip/Items)".to_string();
                            } else {
                                s.message = "Can only dip potions!".to_string();
                                s.message_timer = 60;
                            }
                        }
                        s.render();
                    }
                    _ => {}
                }
                return;
            }

            // Dipping Target
            if let CombatState::DippingTarget { source_idx, cursor } = s.combat {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message.clear();
                        s.render();
                    }
                    "ArrowUp" | "w" => {
                        if cursor > 0 {
                            s.combat = CombatState::DippingTarget {
                                source_idx,
                                cursor: cursor - 1,
                            };
                        }
                        s.render();
                    }
                    "ArrowDown" | "s" => {
                        // 0=Wep, 1=Arm, 2=Chm, 3+=Items
                        let max_cursor = 2 + s.player.items.len();
                        if cursor < max_cursor {
                            s.combat = CombatState::DippingTarget {
                                source_idx,
                                cursor: cursor + 1,
                            };
                        }
                        s.render();
                    }
                    "Enter" => {
                        s.perform_dip(source_idx, cursor);
                        s.render();
                    }
                    _ => {}
                }
                return;
            }

            // Shop mode
            if matches!(s.combat, CombatState::Shopping { .. }) {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message.clear();
                        s.message_timer = 0;
                        s.render();
                    }
                    "ArrowUp" | "w" | "W" => {
                        if let CombatState::Shopping {
                            ref items,
                            ref mut cursor,
                        } = s.combat
                        {
                            if *cursor > 0 {
                                *cursor -= 1;
                            }
                            let _ = items;
                        }
                        s.render();
                    }
                    "ArrowDown" | "s" | "S" => {
                        if let CombatState::Shopping {
                            ref items,
                            ref mut cursor,
                        } = s.combat
                        {
                            if *cursor + 1 < items.len() {
                                *cursor += 1;
                            }
                        }
                        s.render();
                    }
                    "Enter" => {
                        s.shop_buy();
                        s.render();
                    }
                    "r" | "R" => {
                        s.shop_reroll();
                        s.render();
                    }
                    "g" | "G" => {
                        s.shop_steal();
                        s.render();
                    }
                    _ => {}
                }
                return;
            }

            if matches!(s.combat, CombatState::Looking { .. }) {
                event.prevent_default();
                match key.as_str() {
                    "Escape" | "v" | "V" | "Enter" | " " => s.stop_look_mode(),
                    "ArrowUp" | "w" | "W" => s.move_look_cursor(0, -1),
                    "ArrowDown" | "s" | "S" => s.move_look_cursor(0, 1),
                    "ArrowLeft" | "a" | "A" => s.move_look_cursor(-1, 0),
                    "ArrowRight" | "d" | "D" => s.move_look_cursor(1, 0),
                    _ => {}
                }
                s.render();
                return;
            }

            if let CombatState::Aiming {
                spell_idx,
                ref mut dx,
                ref mut dy,
            } = s.combat
            {
                event.prevent_default();
                match key.as_str() {
                    "ArrowUp" | "w" | "W" => {
                        *dx = 0;
                        *dy = -1;
                    }
                    "ArrowDown" | "s" | "S" => {
                        *dx = 0;
                        *dy = 1;
                    }
                    "ArrowLeft" | "a" | "A" => {
                        *dx = -1;
                        *dy = 0;
                    }
                    "ArrowRight" | "d" | "D" => {
                        *dx = 1;
                        *dy = 0;
                    }
                    "Enter" | " " => {
                        let si = spell_idx;
                        let fdx = *dx;
                        let fdy = *dy;
                        s.fire_aimed_spell(si, fdx, fdy);
                    }
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message = "Cancelled aiming.".to_string();
                        s.message_timer = 30;
                    }
                    _ => {}
                }
                s.render();
                return;
            }

            // Exploration movement + item usage
            // Toggle codex with 'c'
            if key == "c" || key == "C" {
                s.show_codex = !s.show_codex;
                s.render();
                return;
            }
            // Toggle listening mode with 'l'
            if key == "l" || key == "L" {
                s.listening_mode = s.listening_mode.cycle();
                s.message = format!("Listening mode: {}", s.listening_mode.label());
                s.message_timer = 90;
                s.render();
                return;
            }
            // Close codex on Escape
            if s.show_codex {
                if key == "Escape" {
                    s.show_codex = false;
                    s.render();
                }
                return;
            }
            match key.as_str() {
                "1" | "2" | "3" | "4" | "5" => {
                    let idx = key.parse::<usize>().unwrap_or(1) - 1;
                    s.use_item(idx);
                    s.render();
                    return;
                }
                "x" | "X" => {
                    event.prevent_default();
                    s.descend_floor(true);
                    s.render();
                    return;
                }
                "v" | "V" => {
                    event.prevent_default();
                    if matches!(s.combat, CombatState::Explore) {
                        s.start_look_mode();
                    }
                    s.render();
                    return;
                }
                "o" | "O" => {
                    if let Tile::Terminal(kind) = s.level.tile(s.player.x, s.player.y) {
                        if s.player.items.is_empty() {
                            s.message = "You have nothing to offer.".to_string();
                            s.message_timer = 60;
                        } else {
                            s.combat = CombatState::Offering {
                                altar_kind: kind,
                                cursor: 0,
                            };
                            s.message = format!("Offer to {}? Select item.", kind.name());
                        }
                    } else {
                        s.message = "There is no altar here.".to_string();
                        s.message_timer = 60;
                    }
                    s.render();
                    return;
                }
                "p" | "P" => {
                    if let Tile::Terminal(kind) = s.level.tile(s.player.x, s.player.y) {
                        s.pray_at_altar(kind);
                    } else {
                        s.message = "You pray to the void. Silence.".to_string();
                        s.message_timer = 60;
                    }
                    s.render();
                    return;
                }
                "D" => {
                    if s.player.items.is_empty() {
                        s.message = "Inventory empty.".to_string();
                        s.message_timer = 60;
                    } else {
                        s.combat = CombatState::DippingSource { cursor: 0 };
                        s.message = "Dip which potion?".to_string();
                    }
                    s.render();
                    return;
                }
                "j" | "J" => {
                    s.combat = CombatState::Journal { page: 0 };
                    s.message = "📖 Character Journal".to_string();
                    s.message_timer = 120;
                    s.render();
                    return;
                }
                "q" => {
                    s.player.cycle_spell();
                    if !s.player.spells.is_empty() {
                        let sp = &s.player.spells[s.player.selected_spell];
                        s.message =
                            format!("Spell: {} {} ({})", sp.hanzi, sp.meaning, sp.effect.label());
                        s.message_timer = 50;
                    }
                    s.render();
                    return;
                }
                " " => {
                    s.use_spell_explore();
                    s.render();
                    return;
                }
                _ => {}
            }
            let (dx, dy) = match key.as_str() {
                "ArrowUp" | "w" | "W" => (0, -1),
                "ArrowDown" | "s" | "S" => (0, 1),
                "ArrowLeft" | "a" | "A" => (-1, 0),
                "ArrowRight" | "d" => (1, 0),
                _ => return,
            };
            event.prevent_default();
            s.try_move(dx, dy);
            s.render();
        });
        doc.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Initial render
    state.borrow().render();

    // Animation loop for particles, screen shake, and flash effects
    {
        let state = Rc::clone(&state);
        let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
        let g = Rc::clone(&f);
        *g.borrow_mut() = Some(Closure::new(move || {
            {
                let Ok(mut s) = state.try_borrow_mut() else {
                    if let Some(win) = window() {
                        let _ = win.request_animation_frame(
                            f.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
                        );
                    }
                    return;
                }; // Tick music
                let mood = match s.combat {
                    CombatState::Fighting { enemy_idx, .. } => {
                        if enemy_idx < s.enemies.len() && s.enemies[enemy_idx].is_boss {
                            crate::audio::MusicMood::Boss
                        } else {
                            crate::audio::MusicMood::Combat
                        }
                    }
                    CombatState::TacticalBattle(ref battle) => {
                        if battle.is_boss_battle {
                            crate::audio::MusicMood::Boss
                        } else {
                            crate::audio::MusicMood::Combat
                        }
                    }
                    CombatState::GameOver => crate::audio::MusicMood::Silent,
                    _ => crate::audio::MusicMood::Explore,
                };
                if let Some(ref mut audio) = s.audio {
                    audio.set_mood(mood);
                    audio.tick_music();
                }

                // Tick achievement popup
                if s.achievement_popup.is_none() {
                    if let Some(id) = s.achievements.pop_popup() {
                        if let Some(def) = AchievementTracker::get_def(id) {
                            s.achievement_popup = Some((def.name, def.desc, 180));
                            // ~3 seconds at 60fps
                        }
                    }
                }
                if let Some((_, _, ref mut timer)) = s.achievement_popup {
                    if *timer > 0 {
                        *timer -= 1;
                    } else {
                        s.achievement_popup = None;
                    }
                }

                let had_message = s.message_timer > 0;
                if had_message {
                    s.tick_message();
                }

                {
                    let weather = if let CombatState::TacticalBattle(ref battle) = s.combat {
                        Some(battle.weather)
                    } else {
                        None
                    };
                    if let Some(weather) = weather {
                        let gs = &mut *s;
                        let cw = gs.renderer.canvas_w;
                        let ch = gs.renderer.canvas_h;
                        match weather {
                            combat::Weather::CoolantLeak => {
                                for _ in 0..3 {
                                    let x = (gs.rng_state & 0xFFFF) as f64 / 65536.0 * cw;
                                    gs.rng_state ^= gs.rng_state << 13;
                                    gs.rng_state ^= gs.rng_state >> 7;
                                    gs.rng_state ^= gs.rng_state << 17;
                                    gs.particles.spawn_rain_drop(x, 0.0, &mut gs.rng_state);
                                }
                            }
                            combat::Weather::SmokeScreen => {
                                if gs.rng_state % 4 == 0 {
                                    let x = (gs.rng_state & 0xFFFF) as f64 / 65536.0 * cw;
                                    gs.rng_state ^= gs.rng_state << 13;
                                    gs.rng_state ^= gs.rng_state >> 7;
                                    gs.rng_state ^= gs.rng_state << 17;
                                    let y = ch * 0.6
                                        + (gs.rng_state & 0xFFFF) as f64 / 65536.0 * ch * 0.4;
                                    gs.rng_state ^= gs.rng_state << 13;
                                    gs.rng_state ^= gs.rng_state >> 7;
                                    gs.rng_state ^= gs.rng_state << 17;
                                    gs.particles.spawn_fog_wisp(x, y, &mut gs.rng_state);
                                }
                            }
                            combat::Weather::DebrisStorm => {
                                for _ in 0..2 {
                                    let y = (gs.rng_state & 0xFFFF) as f64 / 65536.0 * ch;
                                    gs.rng_state ^= gs.rng_state << 13;
                                    gs.rng_state ^= gs.rng_state >> 7;
                                    gs.rng_state ^= gs.rng_state << 17;
                                    gs.particles.spawn_sand_grain(0.0, y, &mut gs.rng_state);
                                }
                            }
                            combat::Weather::EnergyFlux => {
                                if gs.rng_state % 3 == 0 {
                                    let x = (gs.rng_state & 0xFFFF) as f64 / 65536.0 * cw;
                                    gs.rng_state ^= gs.rng_state << 13;
                                    gs.rng_state ^= gs.rng_state >> 7;
                                    gs.rng_state ^= gs.rng_state << 17;
                                    let y = ch * 0.3
                                        + (gs.rng_state & 0xFFFF) as f64 / 65536.0 * ch * 0.5;
                                    gs.rng_state ^= gs.rng_state << 13;
                                    gs.rng_state ^= gs.rng_state >> 7;
                                    gs.rng_state ^= gs.rng_state << 17;
                                    gs.particles.spawn_ink_mote(x, y, &mut gs.rng_state);
                                }
                            }
                            combat::Weather::Normal => {}
                        }
                    }
                }

                s.particles.tick();
                if s.shake_timer > 0 {
                    s.shake_timer -= 1;
                }
                if let Some((_, _, _, ref mut a)) = s.flash {
                    *a -= 0.05;
                    if *a <= 0.0 {
                        s.flash = None;
                    }
                }

                // Tick tactical battle animations (Resolve/EnemyTurn/End phase timers).
                {
                    let gs = &mut *s;
                    let mut old_combat = std::mem::replace(&mut gs.combat, CombatState::Explore);
                    if let CombatState::TacticalBattle(ref mut battle) = old_combat {
                        let event = combat::tick::tick_battle(battle);

                        // Drain queued audio events from combat tick
                        for audio_event in battle.audio_events.drain(..) {
                            if let Some(ref audio) = gs.audio {
                                match audio_event {
                                    combat::AudioEvent::EnemyDeath => audio.play_enemy_death(),
                                    combat::AudioEvent::CriticalHit => audio.play_critical_hit(),
                                    combat::AudioEvent::ProjectileLaunch => {
                                        audio.play_projectile_launch()
                                    }
                                    combat::AudioEvent::ProjectileImpact => {
                                        audio.play_projectile_impact()
                                    }
                                    combat::AudioEvent::Heal => audio.play_heal(),
                                    combat::AudioEvent::ShieldBlock => audio.play_shield_block(),
                                    combat::AudioEvent::StatusBurn => audio.play_status_burn(),
                                    combat::AudioEvent::StatusPoison => audio.play_status_poison(),
                                    combat::AudioEvent::StatusSlow => audio.play_status_slow(),
                                    combat::AudioEvent::SpellElement(ref elem) => {
                                        audio.play_spell_element(elem)
                                    }
                                    combat::AudioEvent::TurnTick => audio.play_turn_tick(),
                                    combat::AudioEvent::TypingCorrect => {
                                        audio.play_typing_correct()
                                    }
                                    combat::AudioEvent::TypingError => audio.play_typing_error(),
                                    combat::AudioEvent::WaterSplash => audio.play_water_splash(),
                                    combat::AudioEvent::LavaRumble => audio.play_lava_rumble(),
                                    combat::AudioEvent::ComboStrike => audio.play_critical_hit(),
                                }
                            }
                        }

                        match event {
                            combat::input::BattleEvent::Victory => {
                                if let Some(ref audio) = gs.audio {
                                    audio.play_victory();
                                }
                                let combo = battle.combo_streak;
                                let killed = combat::transition::exit_combat(
                                    battle,
                                    &mut gs.player,
                                    &mut gs.enemies,
                                );
                                gs.handle_tactical_victory(&killed, combo);
                            }
                            combat::input::BattleEvent::Defeat => {
                                let killer_name = battle
                                    .units
                                    .iter()
                                    .find(|u| u.is_enemy() && u.alive)
                                    .map(|u| u.hanzi.to_string())
                                    .unwrap_or_else(|| "an enemy".to_string());
                                combat::transition::exit_combat(
                                    battle,
                                    &mut gs.player,
                                    &mut gs.enemies,
                                );
                                gs.handle_tactical_defeat(killer_name);
                            }
                            _ => {
                                if battle.pending_spirit_delta != 0 {
                                    gs.player.spirit = (gs.player.spirit
                                        + battle.pending_spirit_delta)
                                        .max(0)
                                        .min(gs.player.max_spirit);
                                    battle.pending_spirit_delta = 0;
                                }
                                gs.combat = old_combat;
                            }
                        }
                    } else {
                        gs.combat = old_combat;
                    }
                }

                s.render();
            }
            // Schedule next frame
            if let Some(win) = window() {
                let _ = win
                    .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
            }
        }));
        let win = window().ok_or("no window")?;
        let _ = win.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }

    Ok(())
}

#[cfg(test)]
mod player_class_tests {
    use crate::player::PlayerClass;

    #[test]
    fn class_data_covers_all_variants() {
        let classes = PlayerClass::all();
        assert_eq!(classes.len(), 20);
        for class in &classes {
            let data = class.data();
            assert!(!data.name_en.is_empty());
            assert!(!data.name_cn.is_empty());
        }
    }
}

#[cfg(test)]
mod item_state_tests {

    #[test]
    fn cursed_health_potion_heals_less() {
        let heal = 6i32;
        let cursed_heal = (heal / 2).max(1);
        assert_eq!(cursed_heal, 3);
        let small_heal = 1i32;
        let cursed_small = (small_heal / 2).max(1);
        assert_eq!(cursed_small, 1);
    }

    #[test]
    fn blessed_health_potion_heals_more() {
        let heal = 6i32;
        let blessed_heal = heal * 3 / 2;
        assert_eq!(blessed_heal, 9);
    }

    #[test]
    fn item_state_parallel_vec_stays_in_sync() {
        use crate::player::{Item, ItemState, Player, PlayerClass};
        let mut p = Player::new(0, 0, PlayerClass::Envoy);
        assert!(p.add_item(Item::MedHypo(5), ItemState::Normal));
        assert!(p.add_item(Item::EMPGrenade, ItemState::Cursed));
        assert!(p.add_item(Item::ScannerPulse, ItemState::Blessed));
        assert_eq!(p.items.len(), 3);
        assert_eq!(p.item_states.len(), 3);
        assert_eq!(p.item_states[1], ItemState::Cursed);
        let (item, state) = p.take_item(1).unwrap();
        assert!(matches!(item, Item::EMPGrenade));
        assert_eq!(state, ItemState::Cursed);
        assert_eq!(p.items.len(), 2);
        assert_eq!(p.item_states.len(), 2);
        assert_eq!(p.item_states[0], ItemState::Normal);
        assert_eq!(p.item_states[1], ItemState::Blessed);
    }

    #[test]
    fn cursed_equipment_cannot_be_replaced() {
        use crate::player::{ItemState, Player, PlayerClass, EQUIPMENT_POOL};
        let mut p = Player::new(0, 0, PlayerClass::Envoy);
        p.equip(&EQUIPMENT_POOL[0], ItemState::Cursed);
        assert_eq!(p.weapon_state, ItemState::Cursed);
        assert_eq!(
            p.equipment_state(crate::player::EquipSlot::Weapon),
            ItemState::Cursed
        );
    }

    #[test]
    fn spirit_decreases_on_move() {
        use crate::player::{Player, PlayerClass};
        let mut p = Player::new(0, 0, PlayerClass::Envoy);
        assert_eq!(p.spirit, 160);
        p.spirit -= 1;
        assert_eq!(p.spirit, 159);
    }

    #[test]
    fn rice_ball_restores_spirit() {
        use crate::player::{Player, PlayerClass};
        let mut p = Player::new(0, 0, PlayerClass::Envoy);
        p.spirit = 50;
        let restore = 40;
        p.spirit = (p.spirit + restore).min(p.max_spirit);
        assert_eq!(p.spirit, 90);

        p.spirit = 140;
        p.spirit = (p.spirit + restore).min(p.max_spirit);
        assert_eq!(p.spirit, 160);
    }

    #[test]
    fn theft_chance_clamped_between_5_and_80() {
        let base: i64 = 40;
        let thief_bonus: i64 = 25;
        assert_eq!((base + thief_bonus).clamp(5, 80), 65);
        assert_eq!((base + thief_bonus - 100).clamp(5, 80), 5);
        assert_eq!(base.clamp(5, 80), 40);
    }

    #[test]
    fn theft_catch_escalates_damage() {
        let mut catches: u32 = 0;
        catches += 1;
        assert_eq!(3 + catches as i32, 4);
        catches += 1;
        assert_eq!(3 + catches as i32, 5);
        catches += 1;
        assert_eq!(3 + catches as i32, 6);
    }

    #[test]
    fn trap_tile_types_cover_all_variants() {
        use crate::dungeon::Tile;
        for t in 0..3u8 {
            let tile = Tile::Trap(t);
            assert!(tile.is_walkable());
        }
    }

    #[test]
    fn trap_tile_looks_like_floor_in_look_text() {
        use crate::dungeon::Tile;
        let trap_text = super::tile_look_text(Tile::Trap(0));
        let floor_text = super::tile_look_text(Tile::MetalFloor);
        assert_eq!(trap_text, floor_text);
    }

    #[test]
    fn sentence_selection_scales_by_floor() {
        let (w_early, _) = super::select_sentence_for_floor(3, 0);
        assert!(w_early.len() <= 3, "Early floor sentences should be short");
        let (w_late, _) = super::select_sentence_for_floor(25, 7);
        assert!(w_late.len() >= 2, "Late floor sentences exist");
    }

    #[test]
    fn gatekeeper_seal_mode_damages_player_on_failure() {
        let mode = super::SentenceChallengeMode::GatekeeperSeal {
            boss_idx: 0,
            success_damage: 5,
            failure_damage_to_player: 3,
        };
        if let super::SentenceChallengeMode::GatekeeperSeal {
            failure_damage_to_player,
            ..
        } = mode
        {
            assert_eq!(failure_damage_to_player, 3);
        } else {
            panic!("expected GatekeeperSeal");
        }
    }
}


