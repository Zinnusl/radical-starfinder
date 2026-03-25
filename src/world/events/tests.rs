use super::*;

#[test]
fn event_pool_has_at_least_40_events() {
    assert!(
        ALL_EVENTS.len() >= 40,
        "Expected at least 40 events, got {}",
        ALL_EVENTS.len()
    );
}

#[test]
fn event_ids_are_unique() {
    let mut ids: Vec<usize> = ALL_EVENTS.iter().map(|e| e.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), ALL_EVENTS.len(), "Duplicate event IDs found");
}

#[test]
fn every_event_has_at_least_two_choices() {
    for event in ALL_EVENTS.iter() {
        assert!(
            event.choices.len() >= 2,
            "Event '{}' has fewer than 2 choices",
            event.title
        );
    }
}

#[test]
fn every_event_has_chinese_title() {
    for event in ALL_EVENTS.iter() {
        assert!(
            !event.chinese_title.is_empty(),
            "Event '{}' is missing chinese_title",
            event.title
        );
    }
}

#[test]
fn select_event_is_deterministic() {
    let e1 = select_event(1, 2, 42);
    let e2 = select_event(1, 2, 42);
    assert_eq!(e1.id, e2.id);
}

#[test]
fn select_event_varies_with_seed() {
    let e1 = select_event(0, 0, 1);
    let e2 = select_event(0, 0, 2);
    // Very unlikely to collide with a good hash, but not impossible
    // so we just test it doesn't panic.
    let _ = (e1.id, e2.id);
}

#[test]
fn all_categories_represented() {
    let categories = [
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
    for cat in &categories {
        assert!(
            ALL_EVENTS.iter().any(|e| e.category == *cat),
            "No events for category {:?}",
            cat
        );
    }
}

#[test]
fn select_event_by_category_returns_correct_category() {
    let event = select_event_by_category(EventCategory::Trading, 99);
    assert_eq!(event.category, EventCategory::Trading);
}

#[test]
fn conditional_events_present_in_pool() {
    assert!(
        ALL_EVENTS.iter().any(|e| e.id == 74),
        "Pirate Debt Collection (74) missing"
    );
    assert!(
        ALL_EVENTS.iter().any(|e| e.id == 75),
        "Refugee Gratitude (75) missing"
    );
    assert!(
        ALL_EVENTS.iter().any(|e| e.id == 76),
        "Crew Mutiny Threat (76) missing"
    );
}

#[test]
fn select_event_with_memory_blocks_unqualified_conditional() {
    use crate::game::EventMemory;
    let memory = EventMemory::default(); // faction_standing = 0, no past choices
    // Index 74 is Pirate Debt Collection (requires faction_standing < -5)
    let result = select_event_with_memory(74, &memory, 42);
    assert_ne!(result, 74, "Conditional event 74 should not trigger with neutral standing");
}

#[test]
fn select_event_with_memory_allows_qualified_conditional() {
    use crate::game::EventMemory;
    let mut memory = EventMemory::default();
    memory.faction_standing = -10;
    // Even though we pass index 74, it should stay because condition is met
    let result = select_event_with_memory(74, &memory, 42);
    assert_eq!(result, 74);
}

#[test]
fn record_event_consequence_tracks_choices() {
    use crate::game::EventMemory;
    let mut memory = EventMemory::default();
    record_event_consequence(&mut memory, 33, 0); // helped stowaway
    assert!(memory.has_choice("helped_stowaway"));
    assert_eq!(memory.crew_morale, 5);
    record_event_consequence(&mut memory, 8, 0); // raided pirates
    assert!(memory.has_choice("raided_pirates"));
    assert_eq!(memory.faction_standing, -10);
}

// ── simple_hash determinism ─────────────────────────────────────────────

#[test]
fn select_event_same_sector_system_seed_returns_same_event() {
    let e1 = select_event(5, 10, 999);
    let e2 = select_event(5, 10, 999);
    assert_eq!(e1.id, e2.id);
}

#[test]
fn select_event_different_sector_changes_result() {
    let e1 = select_event(1, 5, 42);
    let e2 = select_event(2, 5, 42);
    // With a good hash, different sectors should almost always give different events
    let _ = (e1.id, e2.id); // just ensure no panic; collision is possible
}

#[test]
fn select_event_different_system_changes_result() {
    let e1 = select_event(3, 0, 42);
    let e2 = select_event(3, 1, 42);
    let _ = (e1.id, e2.id);
}

// ── event_count ─────────────────────────────────────────────────────────

#[test]
fn event_count_matches_all_events_len() {
    assert_eq!(event_count(), ALL_EVENTS.len());
}

#[test]
fn event_count_includes_conditional_events() {
    assert!(event_count() >= 77);
}

// ── select_event_by_category ────────────────────────────────────────────

#[test]
fn select_event_by_category_deterministic_for_same_seed() {
    let e1 = select_event_by_category(EventCategory::Discovery, 42);
    let e2 = select_event_by_category(EventCategory::Discovery, 42);
    assert_eq!(e1.id, e2.id);
}

#[test]
fn select_event_by_category_different_seeds_may_differ() {
    let e1 = select_event_by_category(EventCategory::PirateEncounter, 0);
    let e2 = select_event_by_category(EventCategory::PirateEncounter, 3);
    // With 6 pirate events and different seeds, likely different
    let _ = (e1.id, e2.id); // just verifying no panic
}

#[test]
fn select_event_by_category_pirate_returns_pirate_event() {
    let event = select_event_by_category(EventCategory::PirateEncounter, 7);
    assert_eq!(event.category, EventCategory::PirateEncounter);
}

#[test]
fn select_event_by_category_distress_returns_distress_event() {
    let event = select_event_by_category(EventCategory::DistressSignal, 0);
    assert_eq!(event.category, EventCategory::DistressSignal);
}

#[test]
fn select_event_by_category_crew_returns_crew_event() {
    let event = select_event_by_category(EventCategory::CrewEvent, 2);
    assert_eq!(event.category, EventCategory::CrewEvent);
}

// ── select_event_with_memory ────────────────────────────────────────────

#[test]
fn select_event_with_memory_passes_through_normal_event() {
    use crate::game::EventMemory;
    let memory = EventMemory::default();
    let result = select_event_with_memory(10, &memory, 42);
    // Non-conditional events should pass through (unless injection triggers)
    // Just verify it doesn't panic and returns a valid index
    assert!(result < event_count());
}

#[test]
fn select_event_with_memory_blocks_crew_mutiny_without_low_morale() {
    use crate::game::EventMemory;
    let memory = EventMemory::default(); // crew_morale = 0
    let result = select_event_with_memory(76, &memory, 42);
    assert_ne!(result, 76, "Mutiny event 76 should not trigger with neutral morale");
}

#[test]
fn select_event_with_memory_allows_crew_mutiny_with_low_morale() {
    use crate::game::EventMemory;
    let mut memory = EventMemory::default();
    memory.crew_morale = -15;
    let result = select_event_with_memory(76, &memory, 42);
    assert_eq!(result, 76);
}

#[test]
fn select_event_with_memory_blocks_refugee_gratitude_without_past_help() {
    use crate::game::EventMemory;
    let memory = EventMemory::default();
    let result = select_event_with_memory(75, &memory, 42);
    assert_ne!(result, 75);
}

#[test]
fn select_event_with_memory_allows_refugee_gratitude_with_helped_refugees() {
    use crate::game::EventMemory;
    let mut memory = EventMemory::default();
    memory.record_choice("helped_refugees");
    let result = select_event_with_memory(75, &memory, 42);
    assert_eq!(result, 75);
}

#[test]
fn select_event_with_memory_allows_refugee_gratitude_with_helped_stowaway() {
    use crate::game::EventMemory;
    let mut memory = EventMemory::default();
    memory.record_choice("helped_stowaway");
    let result = select_event_with_memory(75, &memory, 42);
    assert_eq!(result, 75);
}

#[test]
fn select_event_with_memory_fallback_stays_in_non_conditional_range() {
    use crate::game::EventMemory;
    let memory = EventMemory::default();
    let result = select_event_with_memory(74, &memory, 42);
    assert!(result < event_count() - 3, "Fallback should pick from non-conditional pool");
}

// ── record_event_consequence ────────────────────────────────────────────

#[test]
fn record_event_consequence_stowaway_interrogated_lowers_morale() {
    use crate::game::EventMemory;
    let mut memory = EventMemory::default();
    record_event_consequence(&mut memory, 33, 2);
    assert!(memory.has_choice("interrogated_stowaway"));
    assert_eq!(memory.crew_morale, -3);
}

#[test]
fn record_event_consequence_pirate_fight_lowers_faction() {
    use crate::game::EventMemory;
    let mut memory = EventMemory::default();
    record_event_consequence(&mut memory, 7, 0);
    assert!(memory.has_choice("fought_pirates"));
    assert_eq!(memory.faction_standing, -3);
}

#[test]
fn record_event_consequence_pirate_tribute_raises_faction() {
    use crate::game::EventMemory;
    let mut memory = EventMemory::default();
    record_event_consequence(&mut memory, 7, 1);
    assert!(memory.has_choice("paid_pirates"));
    assert_eq!(memory.faction_standing, 2);
}

#[test]
fn record_event_consequence_peaceful_contact_raises_faction() {
    use crate::game::EventMemory;
    let mut memory = EventMemory::default();
    record_event_consequence(&mut memory, 35, 0);
    assert!(memory.has_choice("peaceful_contact"));
    assert_eq!(memory.faction_standing, 5);
}

#[test]
fn record_event_consequence_attacked_aliens_lowers_faction() {
    use crate::game::EventMemory;
    let mut memory = EventMemory::default();
    record_event_consequence(&mut memory, 35, 2);
    assert!(memory.has_choice("attacked_aliens"));
    assert_eq!(memory.faction_standing, -8);
}

#[test]
fn record_event_consequence_unknown_pair_is_noop() {
    use crate::game::EventMemory;
    let mut memory = EventMemory::default();
    record_event_consequence(&mut memory, 9999, 0);
    assert_eq!(memory.crew_morale, 0);
    assert_eq!(memory.faction_standing, 0);
    assert!(memory.past_choices.is_empty());
}

#[test]
fn record_event_consequence_accumulates_morale_changes() {
    use crate::game::EventMemory;
    let mut memory = EventMemory::default();
    record_event_consequence(&mut memory, 33, 0); // +5
    record_event_consequence(&mut memory, 34, 0); // +10
    assert_eq!(memory.crew_morale, 15);
}

#[test]
fn record_event_consequence_crew_conflict_mediate_raises_morale() {
    use crate::game::EventMemory;
    let mut memory = EventMemory::default();
    record_event_consequence(&mut memory, 31, 0);
    assert!(memory.has_choice("good_captain"));
    assert_eq!(memory.crew_morale, 8);
}

// ── event data integrity ────────────────────────────────────────────────

#[test]
fn every_event_id_matches_pool_position() {
    for (idx, event) in ALL_EVENTS.iter().enumerate() {
        assert_eq!(event.id, idx, "Event '{}' has id {} but is at index {}", event.title, event.id, idx);
    }
}

#[test]
fn every_event_has_nonempty_description() {
    for event in ALL_EVENTS.iter() {
        assert!(!event.description.is_empty(), "Event '{}' has empty description", event.title);
    }
}

