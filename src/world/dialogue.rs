//! Dialogue data generated from `.ink` files at build time.
//!
//! Starmap dialogues map directly to [`super::events::SpaceEvent`] and are
//! triggered during FTL travel / star system exploration.
//!
//! Dungeon dialogues are used during procedural dungeon exploration.

#![allow(dead_code)]

// ── Dungeon dialogue types ──────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DungeonCategory {
    Discovery,
    Trader,
    Alien,
    Hazard,
    Crew,
    Puzzle,
    Lore,
    Shrine,
    Wreckage,
    Terminal,
    Creature,
    Anomaly,
}

#[derive(Clone, Debug)]
pub enum DungeonOutcome {
    Heal(i32),
    Damage(i32),
    GainGold(i32),
    LoseGold(i32),
    GainXp(i32),
    GainRadical(&'static str),
    GainItem(&'static str),
    GainEquipment,
    StartFight,
    Nothing,
    GainCredits(i32),
    LoseCredits(i32),
    GainCrewMember,
}

#[derive(Clone, Debug)]
pub enum DungeonRequirement {
    HasGold(i32),
    HasHp(i32),
    HasRadical(&'static str),
    HasClass(u8),
    None,
}

#[derive(Clone, Debug)]
pub struct DungeonChoice {
    pub text: &'static str,
    pub chinese_hint: &'static str,
    pub outcome: DungeonOutcome,
    pub requires: Option<DungeonRequirement>,
}

#[derive(Clone, Debug)]
pub struct DungeonDialogue {
    pub id: usize,
    pub title: &'static str,
    pub chinese_title: &'static str,
    pub description: &'static str,
    pub choices: &'static [DungeonChoice],
    pub category: DungeonCategory,
}

// ── Generated data ──────────────────────────────────────────────────────────

include!(concat!(env!("OUT_DIR"), "/dialogue_data.rs"));
