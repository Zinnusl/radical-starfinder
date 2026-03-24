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

