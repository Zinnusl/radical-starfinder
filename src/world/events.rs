#![allow(dead_code)]
/// FTL-like random space events for the Starfinder RPG.
///
/// Events trigger at star systems or during FTL jumps and present the player
/// with choices that affect ship state, crew, and resources.  Some events
/// integrate Chinese language learning.

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventCategory {
    DistressSignal,
    PirateEncounter,
    Trading,
    Discovery,
    AnomalyEncounter,
    CrewEvent,
    AlienContact,
    HazardEvent,
    AncientRuins,
    LanguageChallenge,
}

#[derive(Clone, Debug)]
pub enum EventRequirement {
    HasCrewRole(u8),
    HasFuel(i32),
    HasCredits(i32),
    HasRadical(&'static str),
    HasClass(u8),
    None,
}

#[derive(Clone, Debug)]
pub enum EventOutcome {
    GainFuel(i32),
    LoseFuel(i32),
    GainCredits(i32),
    LoseCredits(i32),
    GainHull(i32),
    LoseHull(i32),
    GainRadical(&'static str),
    GainCrewMember,
    LoseCrewMember,
    StartCombat(u8),
    GainItem(&'static str),
    HealCrew(i32),
    DamageCrew(i32),
    RepairShip(i32),
    Nothing,
    GainScrap(i32),
    ShieldDamage(i32),
    FuelAndCredits(i32, i32),
    HullAndFuel(i32, i32),
    CombatReward(u8, i32),
}

#[derive(Clone, Debug)]
pub struct EventChoice {
    pub text: &'static str,
    pub chinese_hint: &'static str,
    pub outcome: EventOutcome,
    pub requires: Option<EventRequirement>,
}

#[derive(Clone, Debug)]
pub struct SpaceEvent {
    pub id: usize,
    pub title: &'static str,
    pub chinese_title: &'static str,
    pub description: &'static str,
    pub choices: &'static [EventChoice],
    pub category: EventCategory,
}


mod encounters;
mod exploration;
mod social;
mod advanced;

use encounters::*;
use exploration::*;
use social::*;
use advanced::*;

pub(super) mod types {
    pub use super::{EventCategory, EventRequirement, EventOutcome, EventChoice, SpaceEvent};
}

// ---------------------------------------------------------------------------
// Consequence tracking: maps (event id, choice index) → memory updates
// ---------------------------------------------------------------------------

use crate::game::EventMemory;

/// Records persistent consequences in EventMemory based on which choice the
/// player made in a specific event.  Called after `apply_event_outcome`.
pub fn record_event_consequence(memory: &mut EventMemory, event_id: usize, choice_idx: usize) {
    match (event_id, choice_idx) {
        // Stowaway: take them on → helped_stowaway, morale +5
        (33, 0) => {
            memory.record_choice("helped_stowaway");
            memory.crew_morale += 5;
        }
        // Stowaway: interrogate → morale -3
        (33, 2) => {
            memory.record_choice("interrogated_stowaway");
            memory.crew_morale -= 3;
        }
        // Pirate Ambush: fight → faction -3
        (7, 0) => {
            memory.record_choice("fought_pirates");
            memory.faction_standing -= 3;
        }
        // Pirate Ambush: pay tribute → faction +2
        (7, 1) => {
            memory.record_choice("paid_pirates");
            memory.faction_standing += 2;
        }
        // Pirate Defector: welcome → faction -5
        (10, 0) => {
            memory.record_choice("sheltered_defector");
            memory.faction_standing -= 5;
        }
        // Pirate Defector: drive away → faction +1
        (10, 2) => {
            memory.faction_standing += 1;
        }
        // Crew Celebration: shore leave → morale +10
        (34, 0) => {
            memory.crew_morale += 10;
        }
        // Crew Celebration: push on → morale -5
        (34, 2) => {
            memory.crew_morale -= 5;
        }
        // Crew Conflict: mediate → morale +8, good captain
        (31, 0) => {
            memory.record_choice("good_captain");
            memory.crew_morale += 8;
        }
        // Crew Conflict: let them sort it → morale -8
        (31, 1) => {
            memory.crew_morale -= 8;
        }
        // Refugee Convoy: share fuel → helped refugees, morale +3
        (51, 0) => {
            memory.record_choice("helped_refugees");
            memory.crew_morale += 3;
        }
        // Refugee Convoy: ignore → morale -2
        (51, 4) => {
            memory.record_choice("ignored_refugees");
            memory.crew_morale -= 2;
        }
        // First Contact: communicate → peaceful, faction +5
        (35, 0) => {
            memory.record_choice("peaceful_contact");
            memory.faction_standing += 5;
        }
        // First Contact: offer gift → faction +3
        (35, 1) => {
            memory.faction_standing += 3;
        }
        // First Contact: power weapons → attacked aliens, faction -8
        (35, 2) => {
            memory.record_choice("attacked_aliens");
            memory.faction_standing -= 8;
        }
        // Pirate Base: raid → faction -10
        (8, 0) => {
            memory.record_choice("raided_pirates");
            memory.faction_standing -= 10;
        }
        // Pirate Base: trade → faction +3
        (8, 2) => {
            memory.faction_standing += 3;
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Master event pool
// ---------------------------------------------------------------------------

pub static ALL_EVENTS: &[&SpaceEvent] = &[
    // Distress Signals (0–6)
    &EVENT_DISTRESS_BEACON,
    &EVENT_ESCAPE_POD,
    &EVENT_DAMAGED_FREIGHTER,
    &EVENT_COLONY_SOS,
    &EVENT_STRANDED_MINERS,
    &EVENT_GHOST_SHIP,
    &EVENT_MEDICAL_FRIGATE,
    // Pirate Encounters (7–12)
    &EVENT_PIRATE_AMBUSH,
    &EVENT_PIRATE_BASE,
    &EVENT_PIRATE_BOARDING,
    &EVENT_PIRATE_DEFECTOR,
    &EVENT_PIRATE_CONVOY,
    &EVENT_RANSOM_DEMAND,
    // Trading (13–18)
    &EVENT_WANDERING_MERCHANT,
    &EVENT_BLACK_MARKET,
    &EVENT_FUEL_DEPOT,
    &EVENT_SMUGGLER_OFFER,
    &EVENT_TRADE_STATION,
    &EVENT_AUCTION_HOUSE,
    // Discovery (19–25)
    &EVENT_DERELICT_SHIP,
    &EVENT_ANCIENT_ARTIFACT,
    &EVENT_ASTEROID_MINING,
    &EVENT_HIDDEN_CACHE,
    &EVENT_NEBULA_PHENOMENON,
    &EVENT_PLANET_SURVEY,
    &EVENT_SIGNAL_SOURCE,
    // Anomaly (26–30)
    &EVENT_SPATIAL_ANOMALY,
    &EVENT_ION_STORM,
    &EVENT_GRAVITY_WELL,
    &EVENT_TIME_DISTORTION,
    &EVENT_WORMHOLE,
    // Crew Events (31–34)
    &EVENT_CREW_CONFLICT,
    &EVENT_TRAINING_EXERCISE,
    &EVENT_STOWAWAY,
    &EVENT_CREW_CELEBRATION,
    // Alien Contact (35–38)
    &EVENT_FIRST_CONTACT,
    &EVENT_ALIEN_TRADERS,
    &EVENT_ALIEN_DISTRESS,
    &EVENT_ALIEN_MONUMENT,
    // Hazard Events (39–42)
    &EVENT_DEBRIS_FIELD,
    &EVENT_SOLAR_FLARE,
    &EVENT_MINEFIELD,
    &EVENT_RADIATION_BELT,
    // Ancient Ruins (43–44)
    &EVENT_ANCIENT_SPACE_STATION,
    &EVENT_TEMPLE_SHIP,
    // Language Challenges (45–48)
    &EVENT_ANCIENT_TERMINAL,
    &EVENT_ENCODED_MESSAGE,
    &EVENT_ROSETTA_PROBE,
    &EVENT_CALLIGRAPHY_CONTEST,
    // New Events (49–73)
    &EVENT_QUANTUM_LABORATORY,
    &EVENT_VOID_LEVIATHAN,
    &EVENT_REFUGEE_CONVOY,
    &EVENT_CHRONO_MERCHANT,
    &EVENT_FUNGAL_STATION,
    &EVENT_PIRATE_KINGS_COURT,
    &EVENT_STELLAR_NURSERY,
    &EVENT_AI_UPRISING,
    &EVENT_CRYSTAL_CAVES,
    &EVENT_BOUNTY_BOARD,
    &EVENT_NEBULA_SANCTUARY,
    &EVENT_GRAVITY_SLINGSHOT,
    &EVENT_ABANDONED_SHIPYARD,
    &EVENT_SPORE_CLOUD,
    &EVENT_MERCENARY_OUTPOST,
    &EVENT_SINGING_COMET,
    &EVENT_CLONE_LAB,
    &EVENT_SPACE_WHALE_MIGRATION,
    &EVENT_SALVAGE_COMPETITION,
    &EVENT_DIMENSIONAL_RIFT,
    &EVENT_ALIEN_ARENA,
    &EVENT_SOLAR_FORGE,
    &EVENT_GHOST_FLEET,
    &EVENT_MEDITATION_NEBULA,
    &EVENT_EMERGENCY_BEACON,
    // Conditional events — gated by EventMemory (74–76)
    &EVENT_PIRATE_DEBT_COLLECTION,
    &EVENT_REFUGEE_GRATITUDE,
    &EVENT_CREW_MUTINY_THREAT,
];

// ---------------------------------------------------------------------------
// Helper: deterministic event selection
// ---------------------------------------------------------------------------

/// Selects an event deterministically from the pool based on sector, system,
/// and a seed value.  Uses a simple hash to avoid pulling in external crates.
pub fn select_event(sector: usize, system_id: usize, seed: u32) -> &'static SpaceEvent {
    let hash = simple_hash(sector as u32, system_id as u32, seed);
    let index = (hash as usize) % ALL_EVENTS.len();
    ALL_EVENTS[index]
}

/// Indices of conditional events that require specific EventMemory state.
const CONDITIONAL_EVENT_INDICES: &[(usize, fn(&EventMemory) -> bool)] = &[
    (74, |m| m.faction_standing < -5),                                      // Pirate Debt Collection
    (75, |m| m.has_choice("helped_refugees") || m.has_choice("helped_stowaway")), // Refugee Gratitude
    (76, |m| m.crew_morale < -10),                                          // Crew Mutiny Threat
];

/// Memory-aware event selection.  Given the original event index from the
/// starmap, checks whether any conditional events should be injected based
/// on the player's EventMemory.
///
/// - If a conditional event is eligible and the seed hash favours it (≈40%),
///   the conditional event replaces the original.
/// - If the original index itself points to a conditional event whose
///   condition is NOT met, falls back to a regular event.
pub fn select_event_with_memory(
    original_id: usize,
    memory: &EventMemory,
    seed: u32,
) -> usize {
    // Check if the original id IS a conditional event that doesn't qualify
    for &(idx, condition) in CONDITIONAL_EVENT_INDICES {
        if original_id == idx && !condition(memory) {
            // Fall back to a different event using hash
            let fallback = simple_hash(original_id as u32, seed, 999);
            // Pick from the non-conditional portion of the pool (0..74)
            let base_count = ALL_EVENTS.len() - CONDITIONAL_EVENT_INDICES.len();
            return (fallback as usize) % base_count;
        }
    }

    // Try to inject a qualifying conditional event (~40% chance per eligible)
    let inject_hash = simple_hash(seed, original_id as u32, 7777);
    if inject_hash % 100 < 40 {
        for &(idx, condition) in CONDITIONAL_EVENT_INDICES {
            if condition(memory) {
                return idx;
            }
        }
    }

    original_id
}

/// Selects an event from a specific category.  Returns the first match after
/// hashing; falls back to the first event in the category if only one exists.
pub fn select_event_by_category(
    category: EventCategory,
    seed: u32,
) -> &'static SpaceEvent {
    let candidates: Vec<&&SpaceEvent> = ALL_EVENTS
        .iter()
        .filter(|e| e.category == category)
        .collect();

    if candidates.is_empty() {
        return ALL_EVENTS[0];
    }

    let index = (seed as usize) % candidates.len();
    candidates[index]
}

/// Returns the total number of events in the pool.
pub fn event_count() -> usize {
    ALL_EVENTS.len()
}

/// Cheap deterministic hash — no external deps.
fn simple_hash(a: u32, b: u32, c: u32) -> u32 {
    let mut h = a.wrapping_mul(2654435761);
    h ^= b.wrapping_mul(2246822519);
    h ^= c.wrapping_mul(3266489917);
    h ^= h >> 16;
    h = h.wrapping_mul(2246822519);
    h ^= h >> 13;
    h = h.wrapping_mul(3266489917);
    h ^= h >> 16;
    h
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------


#[cfg(test)]
mod tests;
