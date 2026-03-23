//! Space combat types.

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

// ── Subsystem ──────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Subsystem {
    pub hp: i32,
    pub max_hp: i32,
}

impl Subsystem {
    pub fn new(hp: i32) -> Self {
        Self { hp, max_hp: hp }
    }
    pub fn is_destroyed(&self) -> bool {
        self.hp <= 0
    }
    pub fn pct(&self) -> f64 {
        self.hp as f64 / self.max_hp.max(1) as f64
    }
}

// ── Subsystem target ───────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SubsystemTarget {
    Weapons,
    Shields,
    Engines,
    Hull,
}

#[allow(dead_code)]
impl SubsystemTarget {
    pub fn all() -> [SubsystemTarget; 4] {
        [SubsystemTarget::Weapons, SubsystemTarget::Shields, SubsystemTarget::Engines, SubsystemTarget::Hull]
    }
    pub fn name(self) -> &'static str {
        match self {
            SubsystemTarget::Weapons => "Weapons",
            SubsystemTarget::Shields => "Shields",
            SubsystemTarget::Engines => "Engines",
            SubsystemTarget::Hull    => "Hull",
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            SubsystemTarget::Weapons => "W",
            SubsystemTarget::Shields => "S",
            SubsystemTarget::Engines => "E",
            SubsystemTarget::Hull    => "H",
        }
    }
}

// ── Ship weapons ───────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShipWeapon {
    Laser,
    Missiles,
    IonCannon,
    Broadside,
}

#[allow(dead_code)]
impl ShipWeapon {
    pub fn name(self) -> &'static str {
        match self {
            ShipWeapon::Laser     => "Laser",
            ShipWeapon::Missiles  => "Missiles",
            ShipWeapon::IonCannon => "Ion Cannon",
            ShipWeapon::Broadside => "Broadside",
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            ShipWeapon::Laser     => "--",
            ShipWeapon::Missiles  => "=>",
            ShipWeapon::IonCannon => "~~",
            ShipWeapon::Broadside => "==",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            ShipWeapon::Laser     => "Consistent damage, always hits",
            ShipWeapon::Missiles  => "High damage, 75% accuracy",
            ShipWeapon::IonCannon => "2x shield dmg, 0.5x hull",
            ShipWeapon::Broadside => "Hits all subsystems, less dmg",
        }
    }
}

// ── Enemy tactics ──────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnemyTactic {
    Aggressive,
    Disabling,
    Balanced,
    Boarding,
}

// ── Enemy ship ─────────────────────────────────────────────────────

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct EnemyShip {
    pub name: String,
    pub hull: i32,
    pub max_hull: i32,
    pub shields: i32,
    pub max_shields: i32,
    pub weapon_power: i32,
    pub engine_power: i32,
    pub loot_credits: i32,
    pub weapons_sub: Subsystem,
    pub shields_sub: Subsystem,
    pub engines_sub: Subsystem,
    pub tactic: EnemyTactic,
    pub turns_taken: u32,
}

// ── Combat phase ───────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum SpaceCombatPhase {
    Choosing,
    TargetingSubsystem,
    PlayerFiring,
    EnemyFiring,
    Boarding,
    Victory,
    Defeat,
}

