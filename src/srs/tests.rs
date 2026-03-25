use super::*;

#[test]
fn record_tracks_correct_and_total() {
    let mut tracker = SrsTracker::new();
    tracker.current_deck = 5;
    tracker.record("好", true);
    tracker.record("好", true);
    tracker.record("好", false);

    let stats = tracker.stats.get("好").unwrap();
    assert_eq!(*stats, (2, 3, 5));
}

#[test]
fn accuracy_unseen_returns_half() {
    let tracker = SrsTracker::new();
    assert_eq!(tracker.accuracy("X"), 0.5);
}

#[test]
fn spawn_weight_low_accuracy_is_high() {
    let mut tracker = SrsTracker::new();
    tracker.current_deck = 1;
    tracker.record("错", false);
    tracker.record("错", false);
    tracker.record("错", true);

    assert_eq!(tracker.spawn_weight("错"), 4);
}

#[test]
fn spawn_weight_decays_over_decks() {
    let mut tracker = SrsTracker::new();
    tracker.current_deck = 1;
    tracker.record("错", false);

    assert_eq!(tracker.spawn_weight("错"), 4);

    tracker.current_deck = 5;
    assert_eq!(tracker.spawn_weight("错"), 2);

    tracker.current_deck = 10;
    assert_eq!(tracker.spawn_weight("错"), 1);
}

// --------------- new ---------------

#[test]
fn new_tracker_has_empty_stats() {
    let tracker = SrsTracker::new();
    assert!(tracker.stats.is_empty());
}

#[test]
fn new_tracker_starts_at_deck_zero() {
    let tracker = SrsTracker::new();
    assert_eq!(tracker.current_deck, 0);
}

// --------------- record ---------------

#[test]
fn record_creates_entry_for_unseen_character() {
    let mut tracker = SrsTracker::new();
    tracker.record("新", true);
    assert!(tracker.stats.contains_key("新"));
}

#[test]
fn record_increments_only_total_on_incorrect() {
    let mut tracker = SrsTracker::new();
    tracker.record("坏", false);
    let stats = tracker.stats.get("坏").unwrap();
    assert_eq!(stats.0, 0);
    assert_eq!(stats.1, 1);
}

#[test]
fn record_updates_last_seen_deck() {
    let mut tracker = SrsTracker::new();
    tracker.current_deck = 3;
    tracker.record("看", true);
    tracker.current_deck = 7;
    tracker.record("看", false);
    assert_eq!(tracker.stats.get("看").unwrap().2, 7);
}

// --------------- accuracy ---------------

#[test]
fn accuracy_all_correct_returns_one() {
    let mut tracker = SrsTracker::new();
    tracker.record("好", true);
    tracker.record("好", true);
    assert_eq!(tracker.accuracy("好"), 1.0);
}

#[test]
fn accuracy_all_wrong_returns_zero() {
    let mut tracker = SrsTracker::new();
    tracker.record("坏", false);
    tracker.record("坏", false);
    assert_eq!(tracker.accuracy("坏"), 0.0);
}

#[test]
fn accuracy_mixed_returns_correct_ratio() {
    let mut tracker = SrsTracker::new();
    tracker.record("中", true);
    tracker.record("中", false);
    tracker.record("中", true);
    tracker.record("中", false);
    assert!((tracker.accuracy("中") - 0.5).abs() < f64::EPSILON);
}

// --------------- mastery_tier ---------------

#[test]
fn mastery_tier_unseen_returns_zero() {
    let tracker = SrsTracker::new();
    assert_eq!(tracker.mastery_tier("未"), 0);
}

#[test]
fn mastery_tier_fewer_than_three_attempts_returns_learning() {
    let mut tracker = SrsTracker::new();
    tracker.record("学", true);
    tracker.record("学", true);
    assert_eq!(tracker.mastery_tier("学"), 1);
}

#[test]
fn mastery_tier_at_exactly_three_attempts_below_60_pct_returns_learning() {
    let mut tracker = SrsTracker::new();
    tracker.record("难", false);
    tracker.record("难", true);
    tracker.record("难", false);
    // 1/3 ≈ 33% < 60%
    assert_eq!(tracker.mastery_tier("难"), 1);
}

#[test]
fn mastery_tier_at_three_attempts_above_60_pct_returns_familiar() {
    let mut tracker = SrsTracker::new();
    tracker.record("知", true);
    tracker.record("知", true);
    tracker.record("知", false);
    // 2/3 ≈ 67% >= 60%
    assert_eq!(tracker.mastery_tier("知"), 2);
}

#[test]
fn mastery_tier_five_attempts_above_85_pct_returns_mastered() {
    let mut tracker = SrsTracker::new();
    for _ in 0..5 {
        tracker.record("精", true);
    }
    // 5/5 = 100% > 85% with 5+ attempts
    assert_eq!(tracker.mastery_tier("精"), 3);
}

#[test]
fn mastery_tier_exactly_85_pct_at_five_attempts_returns_familiar() {
    // 85% exactly is NOT > 85%, so should be tier 2
    let mut tracker = SrsTracker::new();
    // We can't get exactly 85% with 5 attempts, so use 20 attempts: 17/20 = 85%
    for _ in 0..17 {
        tracker.record("近", true);
    }
    for _ in 0..3 {
        tracker.record("近", false);
    }
    assert_eq!(tracker.mastery_tier("近"), 2);
}

#[test]
fn mastery_tier_four_correct_of_five_returns_familiar() {
    // 4/5 = 80% which is >= 60% but not > 85%
    let mut tracker = SrsTracker::new();
    for _ in 0..4 {
        tracker.record("半", true);
    }
    tracker.record("半", false);
    assert_eq!(tracker.mastery_tier("半"), 2);
}

// --------------- spawn_weight ---------------

#[test]
fn spawn_weight_unseen_character_returns_one() {
    let tracker = SrsTracker::new();
    assert_eq!(tracker.spawn_weight("无"), 1);
}

#[test]
fn spawn_weight_high_accuracy_returns_one() {
    let mut tracker = SrsTracker::new();
    tracker.current_deck = 1;
    for _ in 0..5 {
        tracker.record("对", true);
    }
    assert_eq!(tracker.spawn_weight("对"), 1);
}

#[test]
fn spawn_weight_medium_accuracy_returns_two() {
    let mut tracker = SrsTracker::new();
    tracker.current_deck = 1;
    // 1 correct, 2 total = 50% accuracy (< 0.7 but >= 0.5)
    tracker.record("半", true);
    tracker.record("半", false);
    assert_eq!(tracker.spawn_weight("半"), 2);
}

// --------------- weighted_pick ---------------

#[test]
fn weighted_pick_returns_zero_for_empty_pool() {
    let tracker = SrsTracker::new();
    let pool: Vec<&crate::vocab::VocabEntry> = vec![];
    assert_eq!(tracker.weighted_pick(&pool, 42), 0);
}

#[test]
fn weighted_pick_single_entry_always_returns_zero() {
    let tracker = SrsTracker::new();
    let entries = crate::vocab::vocab_for_floor(1);
    let pool: Vec<&crate::vocab::VocabEntry> = vec![entries[0]];
    assert_eq!(tracker.weighted_pick(&pool, 0), 0);
    assert_eq!(tracker.weighted_pick(&pool, 999), 0);
}

#[test]
fn weighted_pick_biases_toward_low_accuracy_entries() {
    let mut tracker = SrsTracker::new();
    tracker.current_deck = 1;
    let entries = crate::vocab::vocab_for_floor(1);
    let pool: Vec<&crate::vocab::VocabEntry> = entries.iter().take(2).copied().collect();

    // Make first entry low accuracy, second high accuracy
    tracker.record(pool[0].hanzi, false);
    tracker.record(pool[0].hanzi, false);
    for _ in 0..5 {
        tracker.record(pool[1].hanzi, true);
    }

    // Weight of pool[0] = 4, pool[1] = 1. Total = 5.
    // rand_val 0..3 should pick index 0, rand_val 4 should pick index 1
    assert_eq!(tracker.weighted_pick(&pool, 0), 0);
    assert_eq!(tracker.weighted_pick(&pool, 3), 0);
    assert_eq!(tracker.weighted_pick(&pool, 4), 1);
}

// --------------- to_json / from_json roundtrip ---------------

#[test]
fn json_roundtrip_preserves_stats() {
    let mut tracker = SrsTracker::new();
    tracker.current_deck = 5;
    tracker.record("好", true);
    tracker.record("好", true);
    tracker.record("好", false);

    let json = tracker.to_json();
    let restored = SrsTracker::from_json(&json);

    assert_eq!(restored.current_deck, 5);
    assert_eq!(restored.stats.get("好"), Some(&(2, 3, 5)));
}

#[test]
fn from_json_empty_stats_returns_empty_tracker() {
    let json = r#"{"deck":0,"stats":{}}"#;
    let tracker = SrsTracker::from_json(json);
    assert!(tracker.stats.is_empty());
    assert_eq!(tracker.current_deck, 0);
}

#[test]
fn from_json_invalid_json_returns_default_tracker() {
    let tracker = SrsTracker::from_json("not json at all");
    assert!(tracker.stats.is_empty());
}

#[test]
fn json_roundtrip_preserves_multiple_characters() {
    let mut tracker = SrsTracker::new();
    tracker.current_deck = 2;
    tracker.record("大", true);
    tracker.record("小", false);

    let json = tracker.to_json();
    let restored = SrsTracker::from_json(&json);

    assert_eq!(restored.stats.get("大"), Some(&(1, 1, 2)));
    assert_eq!(restored.stats.get("小"), Some(&(0, 1, 2)));
}

#[test]
fn from_json_backward_compatible_with_two_element_arrays() {
    let json = r#"{"deck":3,"stats":{"好":[2,3]}}"#;
    let tracker = SrsTracker::from_json(json);
    let stats = tracker.stats.get("好").unwrap();
    assert_eq!(stats.0, 2);
    assert_eq!(stats.1, 3);
    assert_eq!(stats.2, 0); // default last_floor
}

