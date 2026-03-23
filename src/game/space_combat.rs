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
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum SpaceCombatPhase {
    Choosing,
    PlayerFiring,
    EnemyFiring,
    Boarding,
    Victory,
    Defeat,
}

