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
    GravityPull,
    SteamVent,
    OilIgnition,
    CratePush,
    CrateCrush,
    ConveyorMove,
    ChainExplosion,
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
    /// +2 movement, -1 damage, can't use abilities.
    Mobile,
    /// +1 ability power, +1 ability range, -1 movement, -1 damage.
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
            Self::Focused => "+1 ability pwr/range, -1 move/dmg",
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
    /// Coolant tiles expand to adjacent MetalFloor tiles.
    CoolantFlood,
    /// 3-5 random tiles become WeakenedPlating/DamagedFloor.
    HullBreach,
    /// 2-3 OilSlick tiles appear randomly.
    PowerSurge,
    /// All units pushed 1 tile in random cardinal direction.
    VentBlast,
    /// Random tile hit with 3 damage + chain to CoolantPool.
    ArcDischarge,
    /// 2-3 PlasmaPool tiles appear at arena edges, spread inward.
    PlasmaLeak,
    /// All units heal 2 HP.
    MediGas,
    /// All CoolantPool tiles freeze to FrozenCoolant, Wet units get Slow.
    CryoVent,
    /// 4-6 random tiles become Debris, all units lose 1 movement this round.
    DebrisBurst,
    /// All status effect durations extended by 1 turn.
    SystemGlitch,
    /// WiringPanel tiles expand, some upgrade to PipeTangle.
    NaniteSpread,
    /// Single tile becomes PlasmaPool + FuelCanister spawns adjacent.
    ReactorBlowout,
}

impl ArenaEvent {
    pub fn name(self) -> &'static str {
        match self {
            Self::CoolantFlood => "Coolant Flood",
            Self::HullBreach => "Hull Breach",
            Self::PowerSurge => "Power Surge",
            Self::VentBlast => "Vent Blast",
            Self::ArcDischarge => "Arc Discharge",
            Self::PlasmaLeak => "Plasma Leak",
            Self::MediGas => "Medi-Gas",
            Self::CryoVent => "Cryo Vent",
            Self::DebrisBurst => "Debris Burst",
            Self::SystemGlitch => "System Glitch",
            Self::NaniteSpread => "Nanite Spread",
            Self::ReactorBlowout => "Reactor Blowout",
        }
    }

    /// Danger category for color-coding: "damaging", "environmental", "beneficial".
    pub fn danger_level(self) -> &'static str {
        match self {
            Self::ArcDischarge | Self::PlasmaLeak | Self::ReactorBlowout => "damaging",
            Self::MediGas | Self::SystemGlitch | Self::NaniteSpread => "beneficial",
            _ => "environmental",
        }
    }
}

// ── Weather System ───────────────────────────────────────────────────────────

/// Arena-wide environmental effect that modifies combat rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weather {
    /// No environmental effect — baseline.
    Normal,
    /// Coolant leak: Coolant tiles spread, Plasma damage -1, Arc discharge chains +1 tile.
    CoolantLeak,
    /// Smoke screen: Line of sight reduced by 2, ranged ability range -1.
    SmokeScreen,
    /// Debris storm: Movement costs +1, accuracy reduced (miss chance +10%).
    DebrisStorm,
    /// Energy flux: Ability power +1, focus regen +1 per turn.
    EnergyFlux,
}

impl Weather {
    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::CoolantLeak => "Coolant Leak",
            Self::SmokeScreen => "Smoke Screen",
            Self::DebrisStorm => "Debris Storm",
            Self::EnergyFlux => "Energy Flux",
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
/// Normal = 9×9, Elite = 11×11, Boss = 13×13.
pub fn arena_size_for_encounter(has_elite: bool, has_boss: bool) -> usize {
    if has_boss {
        13
    } else if has_elite {
        11
    } else {
        9
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
/// Derived from the sector modifier of the area where combat starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArenaBiome {
    /// Default station interior.
    StationInterior,
    /// Derelict ship / reduced visibility.
    DerelictShip,
    /// Alien ruins / anomalous areas.
    AlienRuins,
    /// Irradiated zone / hazardous areas.
    IrradiatedZone,
    /// Hydroponics bay with pipes and wiring.
    Hydroponics,
    /// Cryo bay with frozen coolant.
    CryoBay,
    /// Reactor room with plasma and heat.
    ReactorRoom,
}

impl ArenaBiome {
    pub fn from_room_modifier(m: Option<crate::dungeon::RoomModifier>) -> Self {
        match m {
            Some(crate::dungeon::RoomModifier::PoweredDown) => ArenaBiome::DerelictShip,
            Some(crate::dungeon::RoomModifier::HighTech) => ArenaBiome::AlienRuins,
            Some(crate::dungeon::RoomModifier::Irradiated) => ArenaBiome::IrradiatedZone,
            Some(crate::dungeon::RoomModifier::Hydroponics) => ArenaBiome::Hydroponics,
            Some(crate::dungeon::RoomModifier::Cryogenic) => ArenaBiome::CryoBay,
            Some(crate::dungeon::RoomModifier::OverheatedReactor) => ArenaBiome::ReactorRoom,
            None => ArenaBiome::StationInterior,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleTile {
    MetalFloor,
    /// Impassable — blocks movement and line of sight.
    CoverBarrier,
    WiringPanel,
    /// Costs 2 movement to enter.
    CoolantPool,
    FrozenCoolant,
    BlastMark,
    /// +1 ability power for units standing in it.
    OilSlick,
    /// Costs 2 movement to enter.
    DamagedPlating,
    /// Blocks line of sight, decays after N turns. Walkable.
    VentSteam,
    /// Deals 2 damage per turn to units standing on it. Costs 2 movement.
    PlasmaPool,
    /// Deals 1 damage on entry.
    ElectrifiedWire,
    /// +2 ability power for units standing on it (stronger OilSlick).
    HoloTrap,
    /// Costs 2 movement to enter.
    Debris,
    /// Blocks movement, blocks LOS. Pipe tangle.
    PipeTangle,
    /// Slows movement (+1 cost). Cryo zone.
    CryoZone,
    /// One-time energy restore (+15). Becomes MetalFloor after use.
    EnergyNode,
    /// Drains 3 energy per turn while standing on it.
    PowerDrain,
    /// Wait on this tile to restore 10 energy.
    ChargingPad,
    /// When an enemy dies on this tile, player gains 10 energy.
    GravityTrap,
    /// Pushable crate. Blocks movement. Slides when hit, damages entities it collides with.
    CargoCrate,
    /// Conveyor belt — pushes units 1 tile at end of each round.
    ConveyorN,
    ConveyorS,
    ConveyorE,
    ConveyorW,
    /// Explodes when hit. 3 damage to adjacent units, chain-reacts with other canisters.
    FuelCanister,
    /// Plating that cracks on first step. Walkable until it collapses.
    WeakenedPlating,
    /// Damaged floor — collapses into breach next time it is stepped on or at end of round.
    DamagedFloor,
    /// Breached floor. Impassable.
    BreachedFloor,
    /// Hidden proximity mine. Deals 2 damage + Slow on trigger.
    MineTile,
    /// Revealed proximity mine. Permanent hazard: 2 damage + Slow on entry.
    MineTileRevealed,
    /// Slippery lubricant. Flammable: fire turns it into BlastMark + AoE damage.
    Lubricant,
    /// Shield zone. Heals units at start of turn. Timed (uses steam_timers).
    ShieldZone,
    /// Elevated platform. +1 damage attacking downhill, -1 damage received from below.
    ElevatedPlatform,
    /// Gravity well. Pulls nearby units 1 tile closer each round.
    GravityWell,
    /// Active steam vent. Blocks sight, deals 1 damage/turn. Walkable.
    SteamVentActive,
    /// Inactive steam vent. Will activate soon.
    SteamVentInactive,
    /// Energy vent — dormant phase. Safe to stand on. Cycles every 3 turns.
    EnergyVentDormant,
    /// Energy vent — charging phase. Telegraphed glow warning. Activates next turn.
    EnergyVentCharging,
    /// Energy vent — active phase. Deals 3 damage to anyone standing on it.
    EnergyVentActive,
}

impl BattleTile {
    pub fn is_walkable(self) -> bool {
        !matches!(
            self,
            BattleTile::CoverBarrier
                | BattleTile::PipeTangle
                | BattleTile::CargoCrate
                | BattleTile::FuelCanister
                | BattleTile::BreachedFloor
                | BattleTile::GravityWell
        )
    }

    pub fn blocks_los(self) -> bool {
        matches!(
            self,
            BattleTile::CoverBarrier | BattleTile::VentSteam | BattleTile::PipeTangle | BattleTile::SteamVentActive
        )
    }

    pub fn extra_move_cost(self) -> i32 {
        match self {
            BattleTile::CoolantPool | BattleTile::DamagedPlating | BattleTile::PlasmaPool | BattleTile::Debris => 1,
            BattleTile::CryoZone => 1,
            BattleTile::ConveyorN
            | BattleTile::ConveyorS
            | BattleTile::ConveyorE
            | BattleTile::ConveyorW => 1,
            _ => 0,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            BattleTile::MetalFloor => "Metal floor. No special effects.",
            BattleTile::CoverBarrier => "Cover barrier. Impassable.",
            BattleTile::WiringPanel => "Wiring panel. No special effects.",
            BattleTile::CoolantPool => "Coolant pool. Costs 2 movement.",
            BattleTile::FrozenCoolant => "Frozen coolant. Slippery surface.",
            BattleTile::BlastMark => "Blast mark. 1 damage/turn.",
            BattleTile::OilSlick => "Oil slick. Abilities +1 damage.",
            BattleTile::DamagedPlating => "Damaged plating. Costs 2 movement.",
            BattleTile::VentSteam => "Vent steam. Blocks line of sight.",
            BattleTile::PlasmaPool => "Plasma pool. 2 damage/turn. Costs 2 movement.",
            BattleTile::ElectrifiedWire => "Electrified wire. 1 damage on entry.",
            BattleTile::HoloTrap => "Holo trap. Abilities +2 damage.",
            BattleTile::Debris => "Debris. Costs 2 movement.",
            BattleTile::PipeTangle => "Pipe tangle. Impassable, blocks sight.",
            BattleTile::CryoZone => "Cryo zone. Costs 2 movement.",
            BattleTile::EnergyNode => "Energy node. +15 energy (one-time).",
            BattleTile::PowerDrain => "Power drain. -3 energy/turn.",
            BattleTile::ChargingPad => "Charging pad. Wait to restore 10 energy.",
            BattleTile::GravityTrap => "Gravity trap. Enemy death here grants +10 energy.",
            BattleTile::CargoCrate => "Cargo crate. Pushable when attacked. Damages what it hits.",
            BattleTile::ConveyorN => "Conveyor (↑). Pushes units north each round.",
            BattleTile::ConveyorS => "Conveyor (↓). Pushes units south each round.",
            BattleTile::ConveyorE => "Conveyor (→). Pushes units east each round.",
            BattleTile::ConveyorW => "Conveyor (←). Pushes units west each round.",
            BattleTile::FuelCanister => "Fuel canister. Explodes when hit, 3 damage to adjacent.",
            BattleTile::WeakenedPlating => "Weakened plating. Will crack when stepped on.",
            BattleTile::DamagedFloor => "Damaged floor. Will collapse into a breach!",
            BattleTile::BreachedFloor => "Breached floor. Impassable.",
            BattleTile::MineTile => "Metal floor. No special effects.",
            BattleTile::MineTileRevealed => "Proximity mine. 2 damage + Slow on entry.",
            BattleTile::Lubricant => "Lubricant. Slippery (slide 1 extra tile). Flammable!",
            BattleTile::ShieldZone => "Shield zone. Heals units at start of turn.",
            BattleTile::ElevatedPlatform => "Elevated platform. +1 damage attacking down, -1 damage from below.",
            BattleTile::GravityWell => "Gravity well. Pulls nearby units 1 tile closer each round.",
            BattleTile::SteamVentActive => "Active steam vent. Blocks sight, 1 damage/turn.",
            BattleTile::SteamVentInactive => "Inactive steam vent. Will activate soon.",
            BattleTile::EnergyVentDormant => "Energy vent (dormant). Safe for now. Cycles every 3 turns.",
            BattleTile::EnergyVentCharging => "Energy vent (charging)! Will discharge next turn!",
            BattleTile::EnergyVentActive => "Energy vent (active)! 3 damage to anyone standing here!",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            BattleTile::MetalFloor | BattleTile::MineTile => "Metal Floor",
            BattleTile::CoverBarrier => "Cover Barrier",
            BattleTile::WiringPanel => "Wiring Panel",
            BattleTile::CoolantPool => "Coolant Pool",
            BattleTile::FrozenCoolant => "Frozen Coolant",
            BattleTile::BlastMark => "Blast Mark",
            BattleTile::OilSlick => "Oil Slick",
            BattleTile::DamagedPlating => "Damaged Plating",
            BattleTile::VentSteam => "Vent Steam",
            BattleTile::PlasmaPool => "Plasma Pool",
            BattleTile::ElectrifiedWire => "Electrified Wire",
            BattleTile::HoloTrap => "Holo Trap",
            BattleTile::Debris => "Debris",
            BattleTile::PipeTangle => "Pipe Tangle",
            BattleTile::CryoZone => "Cryo Zone",
            BattleTile::EnergyNode => "Energy Node",
            BattleTile::PowerDrain => "Power Drain",
            BattleTile::ChargingPad => "Charging Pad",
            BattleTile::GravityTrap => "Gravity Trap",
            BattleTile::CargoCrate => "Cargo Crate",
            BattleTile::ConveyorN => "Conv ↑",
            BattleTile::ConveyorS => "Conv ↓",
            BattleTile::ConveyorE => "Conv →",
            BattleTile::ConveyorW => "Conv ←",
            BattleTile::FuelCanister => "Fuel Canister",
            BattleTile::WeakenedPlating => "Weakened Plating",
            BattleTile::DamagedFloor => "Damaged Floor",
            BattleTile::BreachedFloor => "Breached Floor",
            BattleTile::MineTileRevealed => "Proximity Mine",
            BattleTile::Lubricant => "Lubricant",
            BattleTile::ShieldZone => "Shield Zone",
            BattleTile::ElevatedPlatform => "Elevated Platform",
            BattleTile::GravityWell => "Gravity Well",
            BattleTile::SteamVentActive | BattleTile::SteamVentInactive => "Steam Vent",
            BattleTile::EnergyVentDormant | BattleTile::EnergyVentCharging | BattleTile::EnergyVentActive => "Energy Vent",
        }
    }

    pub fn special_effects(self) -> Option<&'static str> {
        match self {
            BattleTile::BlastMark => Some("1 damage/turn"),
            BattleTile::PlasmaPool => Some("2 damage/turn"),
            BattleTile::ElectrifiedWire => Some("1 damage on entry"),
            BattleTile::OilSlick => Some("Abilities +1 damage"),
            BattleTile::HoloTrap => Some("Abilities +2 damage"),
            BattleTile::FrozenCoolant => Some("Slippery surface"),
            BattleTile::VentSteam => Some("Blocks LOS"),
            BattleTile::PipeTangle => Some("Blocks LOS"),
            BattleTile::EnergyNode => Some("+15 energy (one-time)"),
            BattleTile::PowerDrain => Some("-3 energy/turn"),
            BattleTile::ChargingPad => Some("Wait to restore 10 eng"),
            BattleTile::GravityTrap => Some("Kill here: +10 energy"),
            BattleTile::CargoCrate => Some("Pushable, damages on hit"),
            BattleTile::ConveyorN | BattleTile::ConveyorS | BattleTile::ConveyorE | BattleTile::ConveyorW => Some("Pushes units each round"),
            BattleTile::FuelCanister => Some("Explodes: 3 dmg AoE"),
            BattleTile::WeakenedPlating => Some("Cracks when stepped on"),
            BattleTile::DamagedFloor => Some("Collapses into breach!"),
            BattleTile::MineTileRevealed => Some("2 dmg + Slow on entry"),
            BattleTile::Lubricant => Some("Slippery + Flammable"),
            BattleTile::ShieldZone => Some("Heals at start of turn"),
            BattleTile::ElevatedPlatform => Some("+1 dmg down, -1 dmg up"),
            BattleTile::GravityWell => Some("Pulls units within 2 tiles each round"),
            BattleTile::SteamVentActive => Some("1 dmg/turn, blocks LOS"),
            BattleTile::SteamVentInactive => Some("Toggles every 2 rounds"),
            BattleTile::EnergyVentDormant => Some("Cycles every 3 turns"),
            BattleTile::EnergyVentCharging => Some("⚡ Discharges NEXT turn!"),
            BattleTile::EnergyVentActive => Some("⚡ 3 dmg this turn!"),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TacticalArena {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<BattleTile>,
    /// Per-tile turn countdown for VentSteam decay (0 = no timer).
    pub steam_timers: Vec<u8>,
    /// Per-tile turn countdown for ShieldZone decay (0 = no timer).
    pub holy_timers: Vec<u8>,
    /// Per-tile age counter for PlasmaPool cooling (0 = fresh or non-plasma).
    pub lava_timers: Vec<u8>,
    /// Per-tile cycle counter for EnergyVent (counts turns until next phase transition).
    pub vent_timers: Vec<u8>,
    pub biome: ArenaBiome,
}

impl TacticalArena {
    pub fn new(width: usize, height: usize, biome: ArenaBiome) -> Self {
        let count = width * height;
        Self {
            width,
            height,
            tiles: vec![BattleTile::MetalFloor; count],
            steam_timers: vec![0; count],
            holy_timers: vec![0; count],
            lava_timers: vec![0; count],
            vent_timers: vec![0; count],
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
            self.tiles[i] = BattleTile::VentSteam;
            self.steam_timers[i] = turns;
        }
    }

    pub fn tick_steam(&mut self) {
        for i in 0..self.tiles.len() {
            if self.tiles[i] == BattleTile::VentSteam && self.steam_timers[i] > 0 {
                self.steam_timers[i] -= 1;
                if self.steam_timers[i] == 0 {
                    self.tiles[i] = BattleTile::MetalFloor;
                }
            }
        }
    }

    pub fn set_holy(&mut self, x: i32, y: i32, turns: u8) {
        if let Some(i) = self.idx(x, y) {
            self.tiles[i] = BattleTile::ShieldZone;
            self.holy_timers[i] = turns;
        }
    }

    pub fn tick_holy(&mut self) {
        for i in 0..self.tiles.len() {
            if self.tiles[i] == BattleTile::ShieldZone && self.holy_timers[i] > 0 {
                self.holy_timers[i] -= 1;
                if self.holy_timers[i] == 0 {
                    self.tiles[i] = BattleTile::MetalFloor;
                }
            }
        }
    }

    /// Place an energy vent tile with its initial cycle timer.
    /// Dormant vents wait 2 turns before charging, then 1 turn before active.
    pub fn set_energy_vent(&mut self, x: i32, y: i32, initial_timer: u8) {
        if let Some(i) = self.idx(x, y) {
            self.tiles[i] = BattleTile::EnergyVentDormant;
            self.vent_timers[i] = initial_timer;
        }
    }

    /// Cycle energy vents through their 3-phase pattern.
    /// Returns true if any vent changed state (for log messages).
    pub fn tick_energy_vents(&mut self) -> (bool, bool, bool) {
        let mut became_charging = false;
        let mut became_active = false;
        let mut became_dormant = false;
        for i in 0..self.tiles.len() {
            match self.tiles[i] {
                BattleTile::EnergyVentDormant => {
                    if self.vent_timers[i] > 1 {
                        self.vent_timers[i] -= 1;
                    } else {
                        self.tiles[i] = BattleTile::EnergyVentCharging;
                        self.vent_timers[i] = 1; // 1 turn of charging before active
                        became_charging = true;
                    }
                }
                BattleTile::EnergyVentCharging => {
                    if self.vent_timers[i] > 1 {
                        self.vent_timers[i] -= 1;
                    } else {
                        self.tiles[i] = BattleTile::EnergyVentActive;
                        self.vent_timers[i] = 1; // 1 turn of active before dormant
                        became_active = true;
                    }
                }
                BattleTile::EnergyVentActive => {
                    if self.vent_timers[i] > 1 {
                        self.vent_timers[i] -= 1;
                    } else {
                        self.tiles[i] = BattleTile::EnergyVentDormant;
                        self.vent_timers[i] = 2; // 2 turns dormant before charging again
                        became_dormant = true;
                    }
                }
                _ => {}
            }
        }
        (became_charging, became_active, became_dormant)
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
    pub const SPEED_FAST: f64 = 0.16;   // Arrives in ~6 frames (lasers, lightning)
    pub const SPEED_NORMAL: f64 = 0.10; // Arrives in ~10 frames (standard attacks)
    pub const SPEED_SLOW: f64 = 0.06;   // Arrives in ~17 frames (heavy projectiles)
    #[allow(dead_code)]
    pub const SPEED_CRAWL: f64 = 0.03;  // Arrives in ~33 frames (area denial)

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
    /// 0 = single tile, 1 = 3×3, 2 = 5×5
    pub aoe_radius: u8,
}

/// A telegraphed area-of-effect attack that detonates after a countdown.
///
/// Created when AoE spells are cast; the impact zone is visible to the
/// player (and AI) so they can dodge.  Processed once per round-wrap in
/// `tick_pending_impacts`.
#[derive(Clone, Debug)]
pub struct PendingImpact {
    pub x: i32,
    pub y: i32,
    pub turns_until_hit: u8,
    pub damage: i32,
    /// 0 = single tile, 1 = 3×3, 2 = 5×5
    pub radius: u8,
    pub source_is_player: bool,
    pub element: Option<WuxingElement>,
    pub glyph: &'static str,
    pub color: &'static str,
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

    /// Environmental hazards are resolving (conveyors, vents, gravity wells, etc.).
    EnvironmentTick {
        /// Countdown timer in frames (~60fps).
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
    #[allow(dead_code)]
    ShieldBreak,
    /// Selecting a target for a standalone skill (K key).
    Skill,
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
    /// Ward tile positions placed by PirateCaptain boss.
    pub ward_tiles: Vec<(i32, i32)>,
    /// Last spell school used by the player (for RogueAICore resistance).
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
    /// Stolen module pickups on the grid (DriftLeviathan).
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
    /// Whether the skill menu is open (K key).
    pub skill_menu_open: bool,
    /// Cursor position in skill menu.
    pub skill_menu_cursor: usize,
    pub projectiles: Vec<Projectile>,
    pub arcing_projectiles: Vec<ArcingProjectile>,
    /// Telegraphed AoE impacts that detonate after a turn countdown.
    pub pending_impacts: Vec<PendingImpact>,
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
    /// PhaseWalk set bonus: player may pass through one impassable tile this battle.
    pub phase_walk_available: bool,
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
        let effective_streak = if self.companion_kind == Some(crate::game::Companion::ScienceOfficer) {
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


#[cfg(test)]
pub mod test_helpers;
