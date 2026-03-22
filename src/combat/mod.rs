pub mod action;
pub mod ai;
pub mod boss;
pub mod grid;
pub mod input;
pub mod radical;
pub mod synergy;
pub mod terrain;
pub mod tick;
pub mod transition;
pub mod turn;

use crate::enemy::{AiBehavior, RadicalAction};
use crate::radical::SpellEffect;
use crate::status::StatusInstance;

/// Audio events queued during combat, drained by `game.rs` each frame.
#[derive(Clone, Debug)]
pub enum AudioEvent {
    EnemyDeath,
    CriticalHit,
    ProjectileLaunch,
    ProjectileImpact,
    Heal,
    ShieldBlock,
    StatusBurn,
    StatusPoison,
    StatusSlow,
    SpellElement(String),
    TurnTick,
    TypingCorrect,
    TypingError,
    WaterSplash,
    LavaRumble,
    ComboStrike,
}

// ── Player Combat Stances ────────────────────────────────────────────────────

/// Combat stance the player can cycle during the Command phase.
/// Each stance provides stat modifiers with meaningful tradeoffs.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PlayerStance {
    /// Default — no modifiers.
    Balanced,
    /// +2 damage, -1 armor, -1 movement.
    Aggressive,
    /// +2 armor, -1 damage, +0 movement.
    Defensive,
    /// +2 movement, -1 damage, can't cast spells.
    Mobile,
    /// +1 spell power, +1 spell range, -1 movement, -1 damage.
    Focused,
}

impl PlayerStance {
    pub fn damage_mod(&self) -> i32 {
        match self {
            Self::Balanced => 0,
            Self::Aggressive => 2,
            Self::Defensive => -1,
            Self::Mobile => -1,
            Self::Focused => -1,
        }
    }

    pub fn armor_mod(&self) -> i32 {
        match self {
            Self::Balanced => 0,
            Self::Aggressive => -1,
            Self::Defensive => 2,
            Self::Mobile => 0,
            Self::Focused => 0,
        }
    }

    pub fn movement_mod(&self) -> i32 {
        match self {
            Self::Balanced => 0,
            Self::Aggressive => -1,
            Self::Defensive => 0,
            Self::Mobile => 2,
            Self::Focused => -1,
        }
    }

    pub fn spell_power_mod(&self) -> i32 {
        match self {
            Self::Focused => 1,
            _ => 0,
        }
    }

    pub fn spell_range_mod(&self) -> i32 {
        match self {
            Self::Focused => 1,
            _ => 0,
        }
    }

    pub fn can_cast_spells(&self) -> bool {
        !matches!(self, Self::Mobile)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Balanced => "Balanced",
            Self::Aggressive => "Aggressive",
            Self::Defensive => "Defensive",
            Self::Mobile => "Mobile",
            Self::Focused => "Focused",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Balanced => "⚖",
            Self::Aggressive => "⚔",
            Self::Defensive => "🛡",
            Self::Mobile => "🏃",
            Self::Focused => "🧘",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::Balanced => "#cccccc",
            Self::Aggressive => "#ff4444",
            Self::Defensive => "#4488ff",
            Self::Mobile => "#44cc44",
            Self::Focused => "#bb66ff",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Balanced => "No modifiers",
            Self::Aggressive => "+2 dmg, -1 armor, -1 move",
            Self::Defensive => "+2 armor, -1 dmg",
            Self::Mobile => "+2 move, -1 dmg, no spells",
            Self::Focused => "+1 spell pwr/range, -1 move/dmg",
        }
    }

    /// Cycle to the next stance.
    pub fn next(&self) -> Self {
        match self {
            Self::Balanced => Self::Aggressive,
            Self::Aggressive => Self::Defensive,
            Self::Defensive => Self::Mobile,
            Self::Mobile => Self::Focused,
            Self::Focused => Self::Balanced,
        }
    }
}

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
            "水" | "雨" => Some(Self::Water),
            "火" => Some(Self::Fire),
            "金" | "刀" => Some(Self::Metal),
            "木" | "竹" => Some(Self::Wood),
            "土" | "石" | "山" => Some(Self::Earth),
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

    /// Short label with Chinese character for display.
    pub fn label(self) -> &'static str {
        match self {
            Self::Water => "水 Water",
            Self::Fire => "火 Fire",
            Self::Metal => "金 Metal",
            Self::Wood => "木 Wood",
            Self::Earth => "土 Earth",
        }
    }
}

// ── Arena Events ─────────────────────────────────────────────────────────────

/// Dynamic environmental events that trigger periodically during combat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArenaEvent {
    /// Water tiles expand to adjacent Open tiles.
    RisingWater,
    /// 3-5 random tiles become CrumblingFloor/CrackedFloor.
    EarthTremor,
    /// 2-3 InkPool tiles appear randomly.
    SpiritSurge,
    /// All units pushed 1 tile in random cardinal direction.
    WindGust,
    /// Random tile hit with 3 damage + chain to Water.
    LightningStrike,
    /// 2-3 Lava tiles appear at arena edges, spread inward.
    LavaFlow,
    /// All units heal 2 HP.
    HealingMist,
    /// All Water tiles freeze to Ice, Wet units get Slow.
    FrostSnap,
    /// 4-6 random tiles become Sand, all units lose 1 movement this round.
    SandstormBurst,
    /// All status effect durations extended by 1 turn.
    SpiritualEcho,
    /// Grass tiles expand, some upgrade to BambooThicket.
    WildGrowth,
    /// Single tile becomes Lava + ExplosiveBarrel spawns adjacent.
    VolcanicVent,
}

impl ArenaEvent {
    pub fn name(self) -> &'static str {
        match self {
            Self::RisingWater => "Rising Water",
            Self::EarthTremor => "Earth Tremor",
            Self::SpiritSurge => "Spirit Surge",
            Self::WindGust => "Wind Gust",
            Self::LightningStrike => "Lightning Strike",
            Self::LavaFlow => "Lava Flow",
            Self::HealingMist => "Healing Mist",
            Self::FrostSnap => "Frost Snap",
            Self::SandstormBurst => "Sandstorm Burst",
            Self::SpiritualEcho => "Spiritual Echo",
            Self::WildGrowth => "Wild Growth",
            Self::VolcanicVent => "Volcanic Vent",
        }
    }

    /// Danger category for color-coding: "damaging", "environmental", "beneficial".
    pub fn danger_level(self) -> &'static str {
        match self {
            Self::LightningStrike | Self::LavaFlow | Self::VolcanicVent => "damaging",
            Self::HealingMist | Self::SpiritualEcho | Self::WildGrowth => "beneficial",
            _ => "environmental",
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
    #[allow(dead_code)]
    RadicalAbility { name: &'static str },
    /// Will retreat / move away.
    Retreat,
    /// Will wait / do nothing.
    Idle,
    /// Will use a self-buff.
    Buff,
    /// Will heal self.
    Heal,
    /// Will use a ranged radical action.
    RangedAttack,
    /// Pack behavior, moving to surround player.
    Surround,
}

impl EnemyIntent {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Attack => "Attacking",
            Self::Approach => "Approaching",
            Self::RadicalAbility { name } => name,
            Self::Retreat => "Retreating",
            Self::Idle => "Idle",
            Self::Buff => "Buffing",
            Self::Heal => "Healing",
            Self::RangedAttack => "Ranged Atk",
            Self::Surround => "Surrounding",
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
    /// Pushable rock. Blocks movement. Slides when hit, damages entities it collides with.
    Boulder,
    /// Flowing water — pushes units 1 tile at end of each round.
    FlowNorth,
    FlowSouth,
    FlowEast,
    FlowWest,
    /// Explodes when hit. 3 damage to adjacent units, chain-reacts with other barrels.
    ExplosiveBarrel,
    /// Floor that cracks on first step. Walkable until it collapses.
    CrumblingFloor,
    /// Cracked floor — collapses into Pit next time it is stepped on or at end of round.
    CrackedFloor,
    /// Collapsed pit. Impassable.
    Pit,
    /// Hidden spike trap. Deals 2 damage + Slow on trigger.
    TrapTile,
    /// Revealed spike trap. Permanent hazard: 2 damage + Slow on entry.
    TrapTileRevealed,
    /// Slippery oil. Flammable: fire turns it into Scorched + AoE damage.
    Oil,
    /// Holy ground. Heals units at start of turn. Timed (uses steam_timers).
    HolyGround,
    /// Elevated terrain. +1 damage attacking downhill, -1 damage received from below.
    HighGround,
}

impl BattleTile {
    pub fn is_walkable(self) -> bool {
        !matches!(
            self,
            BattleTile::Obstacle
                | BattleTile::BambooThicket
                | BattleTile::Boulder
                | BattleTile::ExplosiveBarrel
                | BattleTile::Pit
        )
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
            BattleTile::FlowNorth
            | BattleTile::FlowSouth
            | BattleTile::FlowEast
            | BattleTile::FlowWest => 1,
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
            BattleTile::Boulder => "Boulder. Pushable when attacked. Damages what it hits.",
            BattleTile::FlowNorth => "Flowing Water (↑). Pushes units north each round.",
            BattleTile::FlowSouth => "Flowing Water (↓). Pushes units south each round.",
            BattleTile::FlowEast => "Flowing Water (→). Pushes units east each round.",
            BattleTile::FlowWest => "Flowing Water (←). Pushes units west each round.",
            BattleTile::ExplosiveBarrel => "Explosive Barrel. Explodes when hit, 3 damage to adjacent.",
            BattleTile::CrumblingFloor => "Crumbling Floor. Will crack when stepped on.",
            BattleTile::CrackedFloor => "Cracked Floor. Will collapse into a pit!",
            BattleTile::Pit => "Pit. Impassable.",
            BattleTile::TrapTile => "Open ground. No special effects.",
            BattleTile::TrapTileRevealed => "Spike Trap. 2 damage + Slow on entry.",
            BattleTile::Oil => "Oil. Slippery (slide 1 extra tile). Flammable!",
            BattleTile::HolyGround => "Holy Ground. Heals units at start of turn.",
            BattleTile::HighGround => "High Ground. +1 damage attacking down, -1 damage from below.",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            BattleTile::Open | BattleTile::TrapTile => "Open Ground",
            BattleTile::Obstacle => "Obstacle",
            BattleTile::Grass => "Grass",
            BattleTile::Water => "Water",
            BattleTile::Ice => "Ice",
            BattleTile::Scorched => "Scorched",
            BattleTile::InkPool => "Ink Pool",
            BattleTile::BrokenGround => "Broken Ground",
            BattleTile::Steam => "Steam",
            BattleTile::Lava => "Lava",
            BattleTile::Thorns => "Thorns",
            BattleTile::ArcaneGlyph => "Arcane Glyph",
            BattleTile::Sand => "Sand",
            BattleTile::BambooThicket => "Bamboo Thicket",
            BattleTile::FrozenGround => "Frozen Ground",
            BattleTile::SpiritWell => "Spirit Well",
            BattleTile::SpiritDrain => "Spirit Drain",
            BattleTile::MeditationStone => "Meditation Stone",
            BattleTile::SoulTrap => "Soul Trap",
            BattleTile::Boulder => "Boulder",
            BattleTile::FlowNorth => "Flow ↑",
            BattleTile::FlowSouth => "Flow ↓",
            BattleTile::FlowEast => "Flow →",
            BattleTile::FlowWest => "Flow ←",
            BattleTile::ExplosiveBarrel => "Explosive Barrel",
            BattleTile::CrumblingFloor => "Crumbling Floor",
            BattleTile::CrackedFloor => "Cracked Floor",
            BattleTile::Pit => "Pit",
            BattleTile::TrapTileRevealed => "Spike Trap",
            BattleTile::Oil => "Oil",
            BattleTile::HolyGround => "Holy Ground",
            BattleTile::HighGround => "High Ground",
        }
    }

    pub fn special_effects(self) -> Option<&'static str> {
        match self {
            BattleTile::Scorched => Some("1 damage/turn"),
            BattleTile::Lava => Some("2 damage/turn"),
            BattleTile::Thorns => Some("1 damage on entry"),
            BattleTile::InkPool => Some("Spells +1 damage"),
            BattleTile::ArcaneGlyph => Some("Spells +2 damage"),
            BattleTile::Ice => Some("Slippery surface"),
            BattleTile::Steam => Some("Blocks LOS"),
            BattleTile::BambooThicket => Some("Blocks LOS"),
            BattleTile::SpiritWell => Some("+15 spirit (one-time)"),
            BattleTile::SpiritDrain => Some("-3 spirit/turn"),
            BattleTile::MeditationStone => Some("Wait to restore 10 spr"),
            BattleTile::SoulTrap => Some("Kill here: +10 spirit"),
            BattleTile::Boulder => Some("Pushable, damages on hit"),
            BattleTile::FlowNorth | BattleTile::FlowSouth | BattleTile::FlowEast | BattleTile::FlowWest => Some("Pushes units each round"),
            BattleTile::ExplosiveBarrel => Some("Explodes: 3 dmg AoE"),
            BattleTile::CrumblingFloor => Some("Cracks when stepped on"),
            BattleTile::CrackedFloor => Some("Collapses into pit!"),
            BattleTile::TrapTileRevealed => Some("2 dmg + Slow on entry"),
            BattleTile::Oil => Some("Slippery + Flammable"),
            BattleTile::HolyGround => Some("Heals at start of turn"),
            BattleTile::HighGround => Some("+1 dmg down, -1 dmg up"),
            _ => None,
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
    /// Per-tile turn countdown for HolyGround decay (0 = no timer).
    pub holy_timers: Vec<u8>,
    /// Per-tile age counter for Lava cooling (0 = fresh or non-lava).
    pub lava_timers: Vec<u8>,
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
            holy_timers: vec![0; count],
            lava_timers: vec![0; count],
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

    pub fn set_holy(&mut self, x: i32, y: i32, turns: u8) {
        if let Some(i) = self.idx(x, y) {
            self.tiles[i] = BattleTile::HolyGround;
            self.holy_timers[i] = turns;
        }
    }

    pub fn tick_holy(&mut self) {
        for i in 0..self.tiles.len() {
            if self.tiles[i] == BattleTile::HolyGround && self.holy_timers[i] > 0 {
                self.holy_timers[i] -= 1;
                if self.holy_timers[i] == 0 {
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

/// Identifies whether a unit is the player, an enemy, or a companion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Player,
    /// Index into `GameState.enemies`.
    Enemy(usize),
    /// Allied companion unit.
    Companion,
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
    /// Counter stance: reflect 2 damage to next attacker.
    pub radical_counter: bool,
    /// Extra damage marked on this unit (from WeakPoint). Reset after being hit.
    pub marked_extra_damage: i32,
    /// Thorn armor: attackers take 1 damage. Turns remaining.
    pub thorn_armor_turns: i32,
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
    /// Temporary damage bonus from enemy synergies (reset each round).
    pub synergy_damage_bonus: i32,
    /// Whether this unit has elemental resonance active (display flag).
    pub elemental_resonance: bool,
    /// Bonus damage from ally sacrifice (+2 for 2 turns).
    pub sacrifice_bonus_damage: i32,
    /// Turns remaining for sacrifice damage bonus.
    pub sacrifice_bonus_turns: i32,
    /// Movement momentum (0-3). Builds with straight-line movement.
    pub momentum: i32,
    /// Direction of last movement (for momentum tracking).
    pub last_move_dir: Option<Direction>,
}

impl BattleUnit {
    pub fn is_player(&self) -> bool {
        matches!(self.kind, UnitKind::Player)
    }

    pub fn is_enemy(&self) -> bool {
        matches!(self.kind, UnitKind::Enemy(_))
    }

    pub fn is_companion(&self) -> bool {
        matches!(self.kind, UnitKind::Companion)
    }

    /// Effective movement points this turn (base + stored).
    pub fn effective_movement(&self) -> i32 {
        let base = self.movement + self.stored_movement;
        if self
            .statuses
            .iter()
            .any(|s| matches!(s.kind, crate::status::StatusKind::Slow))
        {
            (base / 2).max(1)
        } else {
            base
        }
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
    #[allow(dead_code)]
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

// ── Projectile System ────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum ProjectileEffect {
    Damage(i32),
    /// Damage that bypasses armor/defense (e.g. NeedleStrike).
    PiercingDamage(i32),
    SpellHit(SpellEffect),
}

#[derive(Clone, Debug)]
pub struct Projectile {
    pub from_x: f64,
    pub from_y: f64,
    pub to_x: i32,
    pub to_y: i32,
    pub progress: f64,
    pub speed: f64,
    pub arc_height: f64,
    pub effect: ProjectileEffect,
    pub owner_idx: usize,
    pub glyph: &'static str,
    pub color: &'static str,
    pub done: bool,
}

impl Projectile {
    pub fn current_pos(&self) -> (f64, f64) {
        let t = self.progress;
        let x = self.from_x + (self.to_x as f64 - self.from_x) * t;
        let y_base = self.from_y + (self.to_y as f64 - self.from_y) * t;
        let arc = -4.0 * self.arc_height * t * (t - 1.0);
        (x, y_base - arc)
    }
}

#[derive(Clone, Debug)]
pub struct ArcingProjectile {
    pub target_x: i32,
    pub target_y: i32,
    pub turns_remaining: i32,
    pub effect: ProjectileEffect,
    pub glyph: &'static str,
    pub color: &'static str,
    pub owner_is_player: bool,
    /// Skip the first tick so projectiles don't resolve on the round they're spawned
    pub fresh: bool,
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
        mode: TargetMode,
        cursor_x: i32,
        cursor_y: i32,
        valid_targets: Vec<(i32, i32)>,
        aoe_preview: Vec<(i32, i32)>,
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
        victory: bool,
        timer: u8,
    },

    ProjectileAnimation {
        message: String,
        end_turn: bool,
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
    #[allow(dead_code)]
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
    /// Current combat stance (free action to switch during Command phase).
    pub player_stance: PlayerStance,
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
    /// Last spell element (Wuxing) used by the player (for spell combo chains).
    pub last_spell_element: Option<WuxingElement>,
    /// Turn number when the last spell was cast (for combo window check).
    pub last_spell_turn: u32,
    /// Combo notification message (e.g. "⚡ COMBO: Lightning Storm!").
    pub combo_message: Option<String>,
    /// Fade timer (frames) for combo notification overlay.
    pub combo_message_timer: u16,
    /// Temporary armor bonus from spell combos (stacks cleared after N turns).
    pub combo_armor_bonus: i32,
    /// Turns remaining for combo armor bonus.
    pub combo_armor_turns: i32,
    /// Bonus damage on next N basic attacks from Frozen Edge combo.
    pub frozen_edge_charges: i32,
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
    /// Terrain evolution tick counter (increments each round).
    pub terrain_tick_count: u32,
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
    /// Player radical abilities available this combat.
    pub player_radical_abilities: Vec<(&'static str, crate::enemy::PlayerRadicalAbility)>,
    /// Radicals consumed during this battle (the radical char strings).
    pub consumed_radicals: Vec<&'static str>,
    /// Currently selected radical ability for the next attack (index into player_radical_abilities).
    pub selected_radical_ability: Option<usize>,
    /// Whether the radical picker menu is open.
    pub radical_picker_open: bool,
    /// Cursor position in radical picker (0 = normal attack, 1+ = abilities).
    pub radical_picker_cursor: usize,
    pub projectiles: Vec<Projectile>,
    pub arcing_projectiles: Vec<ArcingProjectile>,
    pub god_mode: bool,
    /// Audio events queued during combat logic, drained by `game.rs`.
    pub audio_events: Vec<AudioEvent>,
    /// Companion type for passive/active ability checks.
    pub companion_kind: Option<crate::game::Companion>,
    /// Player equipment effects copied at combat start for synergy checks.
    pub player_equip_effects: Vec<crate::player::EquipEffect>,
    /// Counts enemy attacks on the player this round for coordinated-attack synergy.
    pub attacks_on_player_this_round: u32,
    /// Countdown to next arena event (event fires when this reaches 0).
    pub arena_event_timer: u32,
    /// Warning shown 1 turn before the event fires.
    pub pending_event: Option<ArenaEvent>,
    /// Display message when event triggers.
    pub event_message: Option<String>,
    /// Fade timer for event message (frames).
    pub event_message_timer: u16,
}

impl TacticalBattle {
    /// Index of the unit whose turn it currently is.
    pub fn current_unit_idx(&self) -> usize {
        self.turn_queue[self.turn_queue_pos]
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
    /// Teacher companion: +1 effective streak for combo tier calculation.
    pub fn combo_multiplier(&self) -> f64 {
        let effective_streak = if self.companion_kind == Some(crate::game::Companion::Teacher) {
            self.combo_streak + 1
        } else {
            self.combo_streak
        };
        match effective_streak {
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
