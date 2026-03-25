//! Dialogue data generated from `.ink` files at build time.
//!
//! Starmap dialogues map directly to [`super::events::SpaceEvent`] and are
//! triggered during FTL travel / star system exploration.
//!
//! Dungeon dialogues are used during procedural dungeon exploration.

#![allow(dead_code)]

// ── Dungeon dialogue types ──────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Dungeon dialogue pool integrity ─────────────────────────────────

    #[test]
    fn dungeon_pool_has_100_dialogues() {
        assert_eq!(
            ALL_DUNGEON_DIALOGUES.len(),
            100,
            "Expected exactly 100 dungeon dialogues, got {}",
            ALL_DUNGEON_DIALOGUES.len()
        );
    }

    #[test]
    fn dungeon_ids_are_unique() {
        let mut ids: Vec<usize> = ALL_DUNGEON_DIALOGUES.iter().map(|d| d.id).collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "Duplicate dungeon dialogue IDs found");
    }

    #[test]
    fn every_dungeon_dialogue_has_at_least_two_choices() {
        for dlg in ALL_DUNGEON_DIALOGUES.iter() {
            assert!(
                dlg.choices.len() >= 2,
                "Dungeon dialogue '{}' (id {}) has fewer than 2 choices",
                dlg.title, dlg.id
            );
        }
    }

    #[test]
    fn every_dungeon_dialogue_has_non_empty_chinese_title() {
        for dlg in ALL_DUNGEON_DIALOGUES.iter() {
            assert!(
                !dlg.chinese_title.is_empty(),
                "Dungeon dialogue '{}' (id {}) is missing chinese_title",
                dlg.title, dlg.id
            );
        }
    }

    #[test]
    fn every_dungeon_choice_has_non_empty_chinese_hint() {
        for dlg in ALL_DUNGEON_DIALOGUES.iter() {
            for (i, choice) in dlg.choices.iter().enumerate() {
                assert!(
                    !choice.chinese_hint.is_empty(),
                    "Dungeon dialogue '{}' (id {}), choice {} has empty chinese_hint",
                    dlg.title, dlg.id, i
                );
            }
        }
    }

    #[test]
    fn every_dungeon_dialogue_has_non_empty_description() {
        for dlg in ALL_DUNGEON_DIALOGUES.iter() {
            assert!(
                dlg.description.len() >= 10,
                "Dungeon dialogue '{}' (id {}) has too-short description: '{}'",
                dlg.title, dlg.id, dlg.description
            );
        }
    }

    #[test]
    fn dungeon_dialogues_cover_multiple_categories() {
        let categories: std::collections::HashSet<_> =
            ALL_DUNGEON_DIALOGUES.iter().map(|d| d.category).collect();
        assert!(
            categories.len() >= 5,
            "Expected at least 5 distinct dungeon categories, got {}",
            categories.len()
        );
    }

    // ── Starmap event pool integrity ────────────────────────────────────

    #[test]
    fn starmap_pool_has_77_events() {
        assert_eq!(
            ALL_STARMAP_EVENTS.len(),
            77,
            "Expected exactly 77 starmap events, got {}",
            ALL_STARMAP_EVENTS.len()
        );
    }

    #[test]
    fn starmap_ids_are_unique() {
        let mut ids: Vec<usize> = ALL_STARMAP_EVENTS.iter().map(|e| e.id).collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "Duplicate starmap event IDs found");
    }

    #[test]
    fn every_starmap_event_has_at_least_two_choices() {
        for ev in ALL_STARMAP_EVENTS.iter() {
            assert!(
                ev.choices.len() >= 2,
                "Starmap event '{}' (id {}) has fewer than 2 choices",
                ev.title, ev.id
            );
        }
    }

    #[test]
    fn every_starmap_event_has_non_empty_chinese_title() {
        for ev in ALL_STARMAP_EVENTS.iter() {
            assert!(
                !ev.chinese_title.is_empty(),
                "Starmap event '{}' (id {}) is missing chinese_title",
                ev.title, ev.id
            );
        }
    }

    #[test]
    fn every_starmap_choice_has_non_empty_chinese_hint() {
        for ev in ALL_STARMAP_EVENTS.iter() {
            for (i, choice) in ev.choices.iter().enumerate() {
                assert!(
                    !choice.chinese_hint.is_empty(),
                    "Starmap event '{}' (id {}), choice {} has empty chinese_hint",
                    ev.title, ev.id, i
                );
            }
        }
    }

    #[test]
    fn starmap_events_cover_all_categories() {
        use crate::world::events::EventCategory;
        let required = [
            EventCategory::DistressSignal,
            EventCategory::PirateEncounter,
            EventCategory::Trading,
            EventCategory::Discovery,
            EventCategory::AnomalyEncounter,
            EventCategory::CrewEvent,
            EventCategory::AlienContact,
            EventCategory::HazardEvent,
            EventCategory::AncientRuins,
            EventCategory::LanguageChallenge,
        ];
        for cat in &required {
            assert!(
                ALL_STARMAP_EVENTS.iter().any(|e| e.category == *cat),
                "Starmap events missing category {:?}",
                cat
            );
        }
    }

    // ── Cross-pool checks ───────────────────────────────────────────────

    #[test]
    fn no_dialogue_has_zero_length_title() {
        for dlg in ALL_DUNGEON_DIALOGUES.iter() {
            assert!(!dlg.title.is_empty(), "Dungeon dialogue id {} has empty title", dlg.id);
        }
        for ev in ALL_STARMAP_EVENTS.iter() {
            assert!(!ev.title.is_empty(), "Starmap event id {} has empty title", ev.id);
        }
    }

    #[test]
    fn no_choice_has_empty_text() {
        for dlg in ALL_DUNGEON_DIALOGUES.iter() {
            for (i, c) in dlg.choices.iter().enumerate() {
                assert!(
                    !c.text.is_empty(),
                    "Dungeon '{}' choice {} has empty text", dlg.title, i
                );
            }
        }
        for ev in ALL_STARMAP_EVENTS.iter() {
            for (i, c) in ev.choices.iter().enumerate() {
                assert!(
                    !c.text.is_empty(),
                    "Starmap '{}' choice {} has empty text", ev.title, i
                );
            }
        }
    }
}
