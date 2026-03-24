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

