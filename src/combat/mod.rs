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

pub const ARENA_SIZE: usize = 9;

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
}

impl BattleTile {
    pub fn is_walkable(self) -> bool {
        !matches!(self, BattleTile::Obstacle)
    }

    pub fn blocks_los(self) -> bool {
        matches!(self, BattleTile::Obstacle | BattleTile::Steam)
    }

    pub fn extra_move_cost(self) -> i32 {
        match self {
            BattleTile::Water | BattleTile::BrokenGround => 1,
            _ => 0,
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
}

impl TacticalArena {
    pub fn new(width: usize, height: usize) -> Self {
        let count = width * height;
        Self {
            width,
            height,
            tiles: vec![BattleTile::Open; count],
            steam_timers: vec![0; count],
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
    /// 5-7=1.3, 8-11=1.5, 12+=2.0.
    pub fn combo_multiplier(&self) -> f64 {
        match self.combo_streak {
            0 => 1.0,
            1..=2 => 1.1,
            3..=4 => 1.2,
            5..=7 => 1.3,
            8..=11 => 1.5,
            _ => 2.0,
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
