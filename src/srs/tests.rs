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

