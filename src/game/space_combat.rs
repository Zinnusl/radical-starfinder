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
    pub is_boss: bool,
    pub initial_tactic: EnemyTactic,
}

// ── Adaptive tactic selection ──────────────────────────────────────

impl EnemyShip {
    /// Evaluate the current battle state and dynamically adapt the enemy tactic.
    ///
    /// Returns a log message when the tactic changes, or `None` if unchanged.
    ///
    /// **Boss enemies** escalate through phased tactics as their HP drops:
    ///   >75% → Balanced, >50% → Aggressive, >25% → Disabling, ≤25% → Boarding.
    ///
    /// **Regular enemies** react to the player's condition:
    ///   - Enemy losing (hull <40%): switch to Disabling (evasive/defensive)
    ///   - Player hull critical (<25%): switch to Boarding (go for the kill)
    ///   - Player shields low (<30%): switch to Aggressive (shield-penetrating)
    pub fn adapt_space_combat_tactic(
        &mut self,
        player_shield_pct: f64,
        player_hull_pct: f64,
    ) -> Option<&'static str> {
        let enemy_hull_pct = self.hull as f64 / self.max_hull.max(1) as f64;

        if self.is_boss {
            let new_tactic = if enemy_hull_pct > 0.75 {
                EnemyTactic::Balanced
            } else if enemy_hull_pct > 0.50 {
                EnemyTactic::Aggressive
            } else if enemy_hull_pct > 0.25 {
                EnemyTactic::Disabling
            } else {
                EnemyTactic::Boarding
            };
            if new_tactic != self.tactic {
                self.tactic = new_tactic;
                return Some(match new_tactic {
                    EnemyTactic::Balanced => "shifts to balanced fire patterns!",
                    EnemyTactic::Aggressive => "becomes more aggressive!",
                    EnemyTactic::Disabling => "focuses on disabling your systems!",
                    EnemyTactic::Boarding => "prepares a desperate boarding action!",
                });
            }
            return None;
        }

        // Regular enemies adapt to battle conditions (priority ordered).

        // Self-preservation: switch to evasive/disabling when losing.
        if enemy_hull_pct < 0.40 {
            if self.tactic != EnemyTactic::Disabling {
                self.tactic = EnemyTactic::Disabling;
                return Some("switches to evasive tactics!");
            }
            return None;
        }

        // Opportunistic: press the kill when player hull is critical.
        if player_hull_pct < 0.25 {
            if self.tactic != EnemyTactic::Boarding {
                self.tactic = EnemyTactic::Boarding;
                return Some("moves in for a boarding action!");
            }
            return None;
        }

        // Exploit weakness: target hull directly when shields are low.
        if player_shield_pct < 0.30 {
            if self.tactic != EnemyTactic::Aggressive {
                self.tactic = EnemyTactic::Aggressive;
                return Some("targets your weakened shields!");
            }
            return None;
        }

        None
    }
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

