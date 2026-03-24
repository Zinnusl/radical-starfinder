use super::*;

#[test]
fn all_achievement_ids_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for a in ACHIEVEMENTS {
        assert!(seen.insert(a.id), "duplicate achievement id: {}", a.id);
    }
}

#[test]
fn get_def_returns_known_achievement() {
    let def = AchievementTracker::get_def("first_kill");
    assert!(def.is_some());
    assert_eq!(def.unwrap().name, "First Blood");
}

#[test]
fn get_def_returns_none_for_unknown() {
    assert!(AchievementTracker::get_def("nonexistent").is_none());
}

#[test]
fn unlock_returns_true_on_first_unlock_false_on_duplicate() {
    let mut tracker = AchievementTracker::new();
    assert!(tracker.unlock("first_kill"), "first unlock should return true");
    assert!(!tracker.unlock("first_kill"), "duplicate unlock should return false");
    assert_eq!(tracker.unlocked.len(), 1);
}

#[test]
fn unlock_rejects_invalid_achievement_id() {
    let mut tracker = AchievementTracker::new();
    assert!(!tracker.unlock("bogus_id"));
    assert!(tracker.unlocked.is_empty());
}

#[test]
fn check_kills_unlocks_progressively() {
    let mut tracker = AchievementTracker::new();
    tracker.check_kills(0);
    assert!(tracker.unlocked.is_empty());

    tracker.check_kills(1);
    assert!(tracker.unlocked.contains(&"first_kill"));
    assert!(!tracker.unlocked.contains(&"kill_10"));

    tracker.check_kills(50);
    assert!(tracker.unlocked.contains(&"kill_10"));
    assert!(tracker.unlocked.contains(&"kill_50"));
    assert!(!tracker.unlocked.contains(&"kill_100"));
}

#[test]
fn check_floor_unlocks_at_thresholds() {
    let mut tracker = AchievementTracker::new();
    tracker.check_floor(2);
    assert!(tracker.unlocked.is_empty());
    tracker.check_floor(5);
    assert!(tracker.unlocked.contains(&"floor_3"));
    assert!(tracker.unlocked.contains(&"floor_5"));
    assert!(!tracker.unlocked.contains(&"floor_10"));
}

#[test]
fn correct_streak_unlocks_scholar_at_five() {
    let mut tracker = AchievementTracker::new();
    for _ in 0..4 {
        tracker.record_correct();
    }
    assert!(!tracker.unlocked.contains(&"perfect_5"));
    tracker.record_correct();
    assert!(tracker.unlocked.contains(&"perfect_5"));
}

#[test]
fn miss_resets_streak() {
    let mut tracker = AchievementTracker::new();
    tracker.record_correct();
    tracker.record_correct();
    tracker.record_miss();
    assert_eq!(tracker.correct_streak, 0);
    // Need 5 more correct to unlock
    for _ in 0..4 {
        tracker.record_correct();
    }
    assert!(!tracker.unlocked.contains(&"perfect_5"));
}

#[test]
fn popup_queue_fifo_order() {
    let mut tracker = AchievementTracker::new();
    tracker.unlock("first_kill");
    tracker.unlock("first_chest");
    assert_eq!(tracker.pop_popup(), Some("first_kill"));
    assert_eq!(tracker.pop_popup(), Some("first_chest"));
    assert_eq!(tracker.pop_popup(), None);
}

