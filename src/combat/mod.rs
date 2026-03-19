pub mod action;
pub mod ai;
pub mod boss;
pub mod grid;
pub mod input;
pub mod radical;
pub mod terrain;
pub mod tick;
pub mod transition;
pub mod turn;

use crate::enemy::{AiBehavior, RadicalAction};
use crate::radical::SpellEffect;
use crate::status::StatusInstance;

// ── Wuxing (五行) Elemental Cycle ────────────────────────────────────────────

/// The five Chinese elements. Cycle: Water > Fire > Metal > Wood > Earth > Water.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WuxingElement {
    Water, // 水
    Fire,  // 火
    Metal, // 金
    Wood,  // 木
    Earth, // 土
}

impl WuxingElement {
    /// Derive element from radical, if it matches one of the five.
    pub fn from_radical(radical: &str) -> Option<Self> {
        match radical {
            "水" => Some(Self::Water),
            "火" => Some(Self::Fire),
            "金" => Some(Self::Metal),
            "木" => Some(Self::Wood),
            "土" => Some(Self::Earth),
            _ => None,
        }
    }

    /// Returns true if `self` beats `other` in the destructive cycle.
    pub fn beats(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Water, Self::Fire)
                | (Self::Fire, Self::Metal)
                | (Self::Metal, Self::Wood)
                | (Self::Wood, Self::Earth)
                | (Self::Earth, Self::Water)
        )
    }

    /// Damage multiplier: 1.5× advantage, 0.75× disadvantage, 1.0× neutral.
    pub fn multiplier(attacker: Option<Self>, defender: Option<Self>) -> f64 {
        match (attacker, defender) {
            (Some(a), Some(d)) if a.beats(d) => 1.5,
            (Some(a), Some(d)) if d.beats(a) => 0.75,
            _ => 1.0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Water => "水 Water",
            Self::Fire => "火 Fire",
            Self::Metal => "金 Metal",
            Self::Wood => "木 Wood",
            Self::Earth => "土 Earth",
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            Self::Water => "💧",
            Self::Fire => "🔥",
            Self::Metal => "⚔",
            Self::Wood => "🌿",
            Self::Earth => "🪨",
        }
    }
}

// ── Weather System ───────────────────────────────────────────────────────────

/// Arena-wide weather effect that modifies combat rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weather {
    /// No weather — baseline.
    Clear,
    /// Rain: Water tiles spread, Fire damage -1, Lightning chains +1 tile.
    Rain,
    /// Fog: Line of sight reduced by 2, ranged spell range -1.
    Fog,
    /// Sandstorm: Movement costs +1, accuracy reduced (miss chance +10%).
    Sandstorm,
    /// Spiritual Ink: Spell power +1, focus regen +1 per turn.
    SpiritualInk,
}

impl Weather {
    pub fn name(self) -> &'static str {
        match self {
            Self::Clear => "Clear",
            Self::Rain => "Rain",
            Self::Fog => "Fog",
            Self::Sandstorm => "Sandstorm",
            Self::SpiritualInk => "Spiritual Ink",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Clear => "Normal conditions.",
            Self::Rain => "Water spreads. Fire weakened. Lightning chains further.",
            Self::Fog => "Reduced visibility. Spell range shortened.",
            Self::Sandstorm => "Movement costs +1. Attacks may miss.",
            Self::SpiritualInk => "Spell power +1. Focus regenerates faster.",
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            Self::Clear => "☀",
            Self::Rain => "🌧",
            Self::Fog => "🌫",
            Self::Sandstorm => "🏜",
            Self::SpiritualInk => "🖋",
        }
    }
}

// ── Enemy Intent (Telegraphed Attacks) ───────────────────────────────────────

/// What an enemy intends to do on its next turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnemyIntent {
    /// Will attack the player.
    Attack,
    /// Will move toward the player.
    Approach,
    /// Will use a radical ability.
    RadicalAbility { name: &'static str },
    /// Will retreat / move away.
    Retreat,
    /// Will wait / do nothing.
    Idle,
}

impl EnemyIntent {
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Attack => "⚔",
            Self::Approach => "➡",
            Self::RadicalAbility { .. } => "✦",
            Self::Retreat => "←",
            Self::Idle => "💤",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Attack => "Attacking",
            Self::Approach => "Approaching",
            Self::RadicalAbility { name } => name,
            Self::Retreat => "Retreating",
            Self::Idle => "Idle",
        }
    }
}

/// Determine arena size based on encounter type.
/// Normal = 7×7, Elite = 9×9, Boss = 11×11.
pub fn arena_size_for_encounter(has_elite: bool, has_boss: bool) -> usize {
    if has_boss {
        11
    } else if has_elite {
        9
    } else {
        7
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

impl Direction {
    pub fn dx(self) -> i32 {
        match self {
            Direction::East => 1,
            Direction::West => -1,
            _ => 0,
        }
    }
    pub fn dy(self) -> i32 {
        match self {
            Direction::North => -1,
            Direction::South => 1,
            _ => 0,
        }
    }

    pub fn opposite(self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }

    pub fn rotate_cw(self) -> Direction {
        match self {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
        }
    }

    pub fn from_delta(dx: i32, dy: i32) -> Option<Direction> {
        if dx.abs() >= dy.abs() {
            if dx > 0 {
                Some(Direction::East)
            } else if dx < 0 {
                Some(Direction::West)
            } else {
                None
            }
        } else {
            if dy > 0 {
                Some(Direction::South)
            } else if dy < 0 {
                Some(Direction::North)
            } else {
                None
            }
        }
    }
}

/// Arena biome — determines tileset and terrain mix.
/// Derived from the dungeon `RoomModifier` of the room where combat starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArenaBiome {
    /// Default stone dungeon.
    Stone,
    /// Shadow / reduced visibility rooms.
    Dark,
    /// Arcane / magical rooms.
    Arcane,
    /// Cursed / corrupted rooms.
    Cursed,
    /// Overgrown garden with bamboo and grass.
    Garden,
    /// Frozen tundra with ice and snow.
    Frozen,
    /// Volcanic inferno with lava and fire.
    Infernal,
}

impl ArenaBiome {
    pub fn from_room_modifier(m: Option<crate::dungeon::RoomModifier>) -> Self {
        match m {
            Some(crate::dungeon::RoomModifier::Dark) => ArenaBiome::Dark,
            Some(crate::dungeon::RoomModifier::Arcane) => ArenaBiome::Arcane,
            Some(crate::dungeon::RoomModifier::Cursed) => ArenaBiome::Cursed,
            Some(crate::dungeon::RoomModifier::Garden) => ArenaBiome::Garden,
            Some(crate::dungeon::RoomModifier::Frozen) => ArenaBiome::Frozen,
            Some(crate::dungeon::RoomModifier::Infernal) => ArenaBiome::Infernal,
            None => ArenaBiome::Stone,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleTile {
    Open,
    /// Impassable — blocks movement and line of sight.
    Obstacle,
    Grass,
    /// Costs 2 movement to enter.
    Water,
    Ice,
    Scorched,
    /// +1 spell power for units standing in it.
    InkPool,
    /// Costs 2 movement to enter.
    BrokenGround,
    /// Blocks line of sight, decays after N turns. Walkable.
    Steam,
    /// Deals 2 damage per turn to units standing on it. Costs 2 movement.
    Lava,
    /// Deals 1 damage on entry.
    Thorns,
    /// +2 spell power for units standing on it (stronger InkPool).
    ArcaneGlyph,
    /// Costs 2 movement to enter (like BrokenGround but thematic).
    Sand,
    /// Blocks movement, blocks LOS. Bamboo thicket (Garden biome).
    BambooThicket,
    /// Slows movement (+1 cost). Frozen ground (Frozen biome).
    FrozenGround,
    /// One-time spirit restore (+15). Becomes Open after use.
    SpiritWell,
    /// Drains 3 spirit per turn while standing on it.
    SpiritDrain,
    /// Wait on this tile to restore 10 spirit.
    MeditationStone,
    /// When an enemy dies on this tile, player gains 10 spirit.
    SoulTrap,
}

impl BattleTile {
    pub fn is_walkable(self) -> bool {
        !matches!(self, BattleTile::Obstacle | BattleTile::BambooThicket)
    }

    pub fn blocks_los(self) -> bool {
        matches!(
            self,
            BattleTile::Obstacle | BattleTile::Steam | BattleTile::BambooThicket
        )
    }

    pub fn extra_move_cost(self) -> i32 {
        match self {
            BattleTile::Water | BattleTile::BrokenGround | BattleTile::Lava | BattleTile::Sand => 1,
            BattleTile::FrozenGround => 1,
            _ => 0,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            BattleTile::Open => "Open ground. No special effects.",
            BattleTile::Obstacle => "Obstacle. Impassable.",
            BattleTile::Grass => "Grass. No special effects.",
            BattleTile::Water => "Water. Costs 2 movement.",
            BattleTile::Ice => "Ice. Slippery surface.",
            BattleTile::Scorched => "Scorched. 1 damage/turn.",
            BattleTile::InkPool => "Ink Pool. Spells +1 damage.",
            BattleTile::BrokenGround => "Broken ground. Costs 2 movement.",
            BattleTile::Steam => "Steam. Blocks line of sight.",
            BattleTile::Lava => "Lava. 2 damage/turn. Costs 2 movement.",
            BattleTile::Thorns => "Thorns. 1 damage on entry.",
            BattleTile::ArcaneGlyph => "Arcane Glyph. Spells +2 damage.",
            BattleTile::Sand => "Sand. Costs 2 movement.",
            BattleTile::BambooThicket => "Bamboo Thicket. Impassable, blocks sight.",
            BattleTile::FrozenGround => "Frozen Ground. Costs 2 movement.",
            BattleTile::SpiritWell => "Spirit Well. +15 spirit (one-time).",
            BattleTile::SpiritDrain => "Spirit Drain. -3 spirit/turn.",
            BattleTile::MeditationStone => "Meditation Stone. Wait to restore 10 spirit.",
            BattleTile::SoulTrap => "Soul Trap. Enemy death here grants +10 spirit.",
        }
    }
}

#[derive(Clone, Debug)]
pub struct TacticalArena {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<BattleTile>,
    /// Per-tile turn countdown for Steam decay (0 = no timer).
    pub steam_timers: Vec<u8>,
    pub biome: ArenaBiome,
}

impl TacticalArena {
    pub fn new(width: usize, height: usize, biome: ArenaBiome) -> Self {
        let count = width * height;
        Self {
            width,
            height,
            tiles: vec![BattleTile::Open; count],
            steam_timers: vec![0; count],
            biome,
        }
    }

    /// Convert (x, y) to a flat index. Returns `None` if out of bounds.
    pub fn idx(&self, x: i32, y: i32) -> Option<usize> {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            Some(y as usize * self.width + x as usize)
        } else {
            None
        }
    }

    /// Get the tile at (x, y).
    pub fn tile(&self, x: i32, y: i32) -> Option<BattleTile> {
        self.idx(x, y).map(|i| self.tiles[i])
    }

    /// Set the tile at (x, y).
    pub fn set_tile(&mut self, x: i32, y: i32, tile: BattleTile) {
        if let Some(i) = self.idx(x, y) {
            self.tiles[i] = tile;
        }
    }

    pub fn set_steam(&mut self, x: i32, y: i32, turns: u8) {
        if let Some(i) = self.idx(x, y) {
            self.tiles[i] = BattleTile::Steam;
            self.steam_timers[i] = turns;
        }
    }

    pub fn tick_steam(&mut self) {
        for i in 0..self.tiles.len() {
            if self.tiles[i] == BattleTile::Steam && self.steam_timers[i] > 0 {
                self.steam_timers[i] -= 1;
                if self.steam_timers[i] == 0 {
                    self.tiles[i] = BattleTile::Open;
                }
            }
        }
    }

    /// Whether (x, y) is in-bounds.
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height
    }
}

// ── Units ────────────────────────────────────────────────────────────────────

/// Identifies whether a unit is the player or an enemy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Player,
    /// Index into `GameState.enemies`.
    Enemy(usize),
}

/// A unit on the tactical battle grid.
#[derive(Clone, Debug)]
pub struct BattleUnit {
    pub kind: UnitKind,
    pub x: i32,
    pub y: i32,
    pub facing: Direction,
    pub hanzi: &'static str,
    pub pinyin: &'static str,
    /// Speed determines turn order (higher = earlier).
    pub speed: i32,
    /// Movement points per turn (base).
    pub movement: i32,
    /// Stored bonus movement from Wait action (max +2).
    pub stored_movement: i32,
    pub hp: i32,
    pub max_hp: i32,
    /// Damage value (base).
    pub damage: i32,
    /// Currently defending (50% damage reduction until next turn).
    pub defending: bool,
    /// This unit is alive and active.
    pub alive: bool,
    /// AI behavior (only meaningful for enemies).
    pub ai: AiBehavior,
    /// Radical actions available to this enemy.
    pub radical_actions: Vec<RadicalAction>,
    /// Active status effects.
    pub statuses: Vec<StatusInstance>,
    /// Stunned: skip next turn.
    pub stunned: bool,
    /// Temporary armor from radical action (reduces next player hit).
    pub radical_armor: i32,
    /// Will dodge next attack (ShadowStep).
    pub radical_dodge: bool,
    /// Next attack hits twice (Multiply).
    pub radical_multiply: bool,
    /// Fortify stacks (permanent +damage this battle).
    pub fortify_stacks: i32,
    /// Boss kind (only set for boss enemies).
    pub boss_kind: Option<crate::enemy::BossKind>,
    /// Whether this unit is a decoy (MimicKing clones).
    pub is_decoy: bool,
    /// Word group ID — units from the same multi-char word share this value.
    pub word_group: Option<usize>,
    /// Position within the word (0 = first char, 1 = second, etc.).
    pub word_group_order: u8,
    /// Wuxing element derived from radical (if any).
    pub wuxing_element: Option<WuxingElement>,
    /// Telegraphed intent for this enemy's next turn.
    pub intent: Option<EnemyIntent>,
    /// SRS mastery tier: 0=unknown, 1=learning, 2=familiar, 3=mastered.
    pub mastery_tier: u8,
    /// Charge-cast: turns remaining before complex character attack fires.
    /// None = not charging. Some(0) = ready to fire.
    pub charge_remaining: Option<u8>,
}

impl BattleUnit {
    pub fn is_player(&self) -> bool {
        matches!(self.kind, UnitKind::Player)
    }

    pub fn is_enemy(&self) -> bool {
        matches!(self.kind, UnitKind::Enemy(_))
    }

    /// Effective movement points this turn (base + stored).
    pub fn effective_movement(&self) -> i32 {
        self.movement + self.stored_movement
    }
}

// ── Turn phases ──────────────────────────────────────────────────────────────

/// What kind of typing action the player is performing.
#[derive(Clone, Debug)]
pub enum TypingAction {
    /// Attacking an enemy — must type the enemy's pinyin.
    BasicAttack { target_unit: usize },
    /// Casting a spell — must type the spell's pinyin.
    SpellCast {
        spell_idx: usize,
        target_x: i32,
        target_y: i32,
        effect: SpellEffect,
    },
    /// Breaking an enemy's component shield.
    ShieldBreak {
        target_unit: usize,
        component: &'static str,
    },
    /// Elite chain attack — multi-syllable pinyin typed one syllable at a time.
    EliteChain {
        target_unit: usize,
        syllable_progress: usize,
        total_syllables: usize,
        damage_per_syllable: i32,
        damage_dealt: i32,
    },
}

/// Collapsed tactical phases per Oracle review (~5 core states).
///
/// Transient UI state (cursor position, valid tiles, etc.) is stored
/// as fields rather than encoded as additional enum variants.
#[derive(Clone, Debug)]
pub enum TacticalPhase {
    /// Player is choosing an action (Move / Attack / Spell / Item / Defend / Wait / Flee).
    Command,

    /// Player is selecting a target tile or unit.
    /// `mode` determines what happens after selection.
    Targeting {
        /// What are we targeting for?
        mode: TargetMode,
        /// Current cursor position on the grid.
        cursor_x: i32,
        cursor_y: i32,
        /// Set of valid target positions.
        valid_targets: Vec<(i32, i32)>,
    },

    /// An action is being resolved (animation / result display).
    Resolve {
        /// Brief description for the battle log.
        message: String,
        /// Countdown timer in frames (~60fps, so 30 = 500ms).
        timer: u8,
        /// When true, advance to the next unit after timer expires.
        /// When false, return to Command (player can still act).
        end_turn: bool,
    },

    /// An enemy unit is executing its turn.
    EnemyTurn {
        /// Index into `units` of the acting enemy.
        unit_idx: usize,
        /// Countdown timer in frames (~60fps).
        timer: u8,
        /// Whether the enemy action has been executed yet.
        acted: bool,
    },

    /// Player is inspecting the arena (free-look cursor).
    Look {
        /// Current look-cursor position on the grid.
        cursor_x: i32,
        cursor_y: i32,
    },

    /// Player chooses starting position before combat begins.
    Deployment {
        cursor_x: i32,
        cursor_y: i32,
        valid_tiles: Vec<(i32, i32)>,
    },

    /// Battle is over — showing results before returning to exploration.
    End {
        /// Did the player win?
        victory: bool,
        /// Timer in frames before allowing keypress transition.
        timer: u8,
    },
}

/// What the targeting phase is selecting for.
#[derive(Clone, Debug)]
pub enum TargetMode {
    /// Selecting a movement destination.
    Move,
    /// Selecting an adjacent enemy to attack (transitions to typing).
    Attack,
    /// Selecting a spell target tile/unit (transitions to typing).
    Spell { spell_idx: usize },
    /// Selecting an enemy to break a component shield (transitions to typing).
    ShieldBreak,
}

// ── Top-level battle state ───────────────────────────────────────────────────

/// The full state of a tactical battle. Owned by `GameState` during combat.
///
/// `game.rs` holds a `TacticalBattle` and delegates to methods on it;
/// the combat module owns all tactical logic.
#[derive(Clone, Debug)]
pub struct TacticalBattle {
    /// The battle arena grid.
    pub arena: TacticalArena,
    /// All units participating in this battle (index 0 = player).
    pub units: Vec<BattleUnit>,
    /// Turn order: indices into `units`, sorted by speed descending.
    pub turn_queue: Vec<usize>,
    /// Current position in `turn_queue`.
    pub turn_queue_pos: usize,
    /// Current tactical phase.
    pub phase: TacticalPhase,
    /// Battle-wide turn counter (increments when the queue wraps).
    pub turn_number: u32,
    /// Combo streak from consecutive correct pinyin answers.
    pub combo_streak: u32,
    /// Player has already moved this turn.
    pub player_moved: bool,
    /// Player has already used their action this turn.
    pub player_acted: bool,
    /// Text the player is currently typing (pinyin input buffer).
    pub typing_buffer: String,
    /// Active typing action (what the buffer is for).
    pub typing_action: Option<TypingAction>,
    /// Battle log messages.
    pub log: Vec<String>,
    /// Last answer result for SRS tracking: (hanzi, correct).
    /// Set by `resolve_basic_attack` / spell typing; consumed by `game.rs`.
    pub last_answer: Option<(&'static str, bool)>,
    /// Boss battles have delayed exhaustion (starts at turn 15 instead of 10).
    pub is_boss_battle: bool,
    /// Player spells: (hanzi, pinyin, effect).
    pub available_spells: Vec<(&'static str, &'static str, SpellEffect)>,
    pub spell_cursor: usize,
    pub spell_menu_open: bool,
    /// Index of a spell that was just consumed (cast successfully).
    /// Consumed by `game.rs` to remove from `player.spells`.
    pub spent_spell_index: Option<usize>,
    /// Ward tile positions placed by Gatekeeper boss.
    pub ward_tiles: Vec<(i32, i32)>,
    /// Last spell school used by the player (for Elementalist resistance).
    pub last_spell_school: Option<&'static str>,
    /// Stolen spell pickups on the grid (RadicalThief).
    /// Each entry: (x, y, hanzi, pinyin, effect).
    pub stolen_spells: Vec<(
        i32,
        i32,
        &'static str,
        &'static str,
        crate::radical::SpellEffect,
    )>,
    pub player_class: Option<crate::player::PlayerClass>,
    pub available_items: Vec<(usize, crate::player::Item)>,
    pub used_item_indices: Vec<usize>,
    pub item_menu_open: bool,
    pub item_cursor: usize,
    /// Arena weather effect.
    pub weather: Weather,
    /// Mental focus resource. Complex chars cost more focus to attack.
    pub focus: i32,
    pub max_focus: i32,
    /// Radical synergy tracking: (last radical killed, consecutive streak).
    pub radical_synergy_radical: Option<&'static str>,
    pub radical_synergy_streak: u32,
    /// Kill history for chengyu (成语) detection — last 4 hanzi killed.
    pub chengyu_history: Vec<String>,
    /// Enemy intents calculated at start of each round.
    pub intents_calculated: bool,
    /// Accumulated spirit delta from tile effects (applied by game.rs each tick).
    pub pending_spirit_delta: i32,
}

impl TacticalBattle {
    /// Index of the unit whose turn it currently is.
    pub fn current_unit_idx(&self) -> usize {
        self.turn_queue[self.turn_queue_pos]
    }

    /// Reference to the unit whose turn it currently is.
    pub fn current_unit(&self) -> &BattleUnit {
        &self.units[self.current_unit_idx()]
    }

    /// Is it the player's turn?
    pub fn is_player_turn(&self) -> bool {
        self.current_unit().is_player()
    }

    /// Push a message to the battle log.
    pub fn log_message(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        self.log.push(msg);
        // Keep the log from growing unbounded.
        if self.log.len() > 50 {
            self.log.remove(0);
        }
    }

    /// Check if all enemies are dead.
    pub fn all_enemies_dead(&self) -> bool {
        self.units.iter().filter(|u| u.is_enemy()).all(|u| !u.alive)
    }

    /// Check if the player unit is dead.
    pub fn player_dead(&self) -> bool {
        self.units.first().map(|u| !u.alive).unwrap_or(true)
    }

    /// Find a unit at position (x, y) that is alive.
    pub fn unit_at(&self, x: i32, y: i32) -> Option<usize> {
        self.units
            .iter()
            .position(|u| u.alive && u.x == x && u.y == y)
    }

    /// Find all alive enemy unit indices adjacent to position (x, y).
    pub fn adjacent_enemies(&self, x: i32, y: i32) -> Vec<usize> {
        let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        let mut result = Vec::new();
        for (dx, dy) in &deltas {
            let nx = x + dx;
            let ny = y + dy;
            if let Some(idx) = self.unit_at(nx, ny) {
                if self.units[idx].is_enemy() {
                    result.push(idx);
                }
            }
        }
        result
    }

    /// Get combo damage multiplier based on current streak.
    /// Same 6 tiers as existing system: 0=1.0, 1-2=1.1, 3-4=1.2,
    /// 5-7=1.3, 8-11=1.5, 12+=1.75.
    pub fn combo_multiplier(&self) -> f64 {
        match self.combo_streak {
            0 => 1.0,
            1..=2 => 1.1,
            3..=4 => 1.2,
            5..=7 => 1.3,
            8..=11 => 1.5,
            _ => 1.75,
        }
    }

    /// Combo tier name for display.
    pub fn combo_tier_name(&self) -> &'static str {
        match self.combo_streak {
            0 => "",
            1..=2 => "Good",
            3..=4 => "Great",
            5..=7 => "Excellent",
            8..=11 => "Amazing",
            _ => "RADICAL!",
        }
    }
}
