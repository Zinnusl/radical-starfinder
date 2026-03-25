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

// ── unlock edge cases ───────────────────────────────────────────────────

#[test]
fn unlock_duplicate_does_not_add_to_popup_queue() {
    let mut tracker = AchievementTracker::new();
    tracker.unlock("first_kill");
    tracker.pop_popup(); // drain first popup
    tracker.unlock("first_kill"); // duplicate
    assert_eq!(tracker.pop_popup(), None);
}

#[test]
fn unlock_multiple_distinct_achievements() {
    let mut tracker = AchievementTracker::new();
    assert!(tracker.unlock("first_kill"));
    assert!(tracker.unlock("first_chest"));
    assert!(tracker.unlock("first_forge"));
    assert_eq!(tracker.unlocked.len(), 3);
}

// ── check_kills ─────────────────────────────────────────────────────────

#[test]
fn check_kills_exact_threshold_10() {
    let mut tracker = AchievementTracker::new();
    tracker.check_kills(10);
    assert!(tracker.unlocked.contains(&"kill_10"));
    assert!(tracker.unlocked.contains(&"first_kill"));
}

#[test]
fn check_kills_100_unlocks_all_kill_achievements() {
    let mut tracker = AchievementTracker::new();
    tracker.check_kills(100);
    assert!(tracker.unlocked.contains(&"first_kill"));
    assert!(tracker.unlocked.contains(&"kill_10"));
    assert!(tracker.unlocked.contains(&"kill_50"));
    assert!(tracker.unlocked.contains(&"kill_100"));
}

#[test]
fn check_kills_idempotent_on_repeated_calls() {
    let mut tracker = AchievementTracker::new();
    tracker.check_kills(10);
    tracker.check_kills(10);
    assert_eq!(
        tracker.unlocked.iter().filter(|&&id| id == "kill_10").count(),
        1
    );
}

// ── check_floor ─────────────────────────────────────────────────────────

#[test]
fn check_floor_exact_threshold_3() {
    let mut tracker = AchievementTracker::new();
    tracker.check_floor(3);
    assert!(tracker.unlocked.contains(&"floor_3"));
    assert!(!tracker.unlocked.contains(&"floor_5"));
}

#[test]
fn check_floor_10_unlocks_all_floor_achievements() {
    let mut tracker = AchievementTracker::new();
    tracker.check_floor(10);
    assert!(tracker.unlocked.contains(&"floor_3"));
    assert!(tracker.unlocked.contains(&"floor_5"));
    assert!(tracker.unlocked.contains(&"floor_10"));
}

// ── check_recipes ───────────────────────────────────────────────────────

#[test]
fn check_recipes_exact_threshold_1() {
    let mut tracker = AchievementTracker::new();
    tracker.check_recipes(1);
    assert!(tracker.unlocked.contains(&"first_forge"));
    assert!(!tracker.unlocked.contains(&"forge_5"));
}

#[test]
fn check_recipes_20_unlocks_all_forge_achievements() {
    let mut tracker = AchievementTracker::new();
    tracker.check_recipes(20);
    assert!(tracker.unlocked.contains(&"first_forge"));
    assert!(tracker.unlocked.contains(&"forge_5"));
    assert!(tracker.unlocked.contains(&"forge_10"));
    assert!(tracker.unlocked.contains(&"forge_20"));
}

#[test]
fn check_recipes_below_threshold_unlocks_nothing() {
    let mut tracker = AchievementTracker::new();
    tracker.check_recipes(0);
    assert!(tracker.unlocked.is_empty());
}

// ── check_gold ──────────────────────────────────────────────────────────

#[test]
fn check_gold_exact_threshold_100() {
    let mut tracker = AchievementTracker::new();
    tracker.check_gold(100);
    assert!(tracker.unlocked.contains(&"gold_100"));
    assert!(!tracker.unlocked.contains(&"gold_500"));
}

#[test]
fn check_gold_500_unlocks_both() {
    let mut tracker = AchievementTracker::new();
    tracker.check_gold(500);
    assert!(tracker.unlocked.contains(&"gold_100"));
    assert!(tracker.unlocked.contains(&"gold_500"));
}

#[test]
fn check_gold_below_threshold_unlocks_nothing() {
    let mut tracker = AchievementTracker::new();
    tracker.check_gold(99);
    assert!(tracker.unlocked.is_empty());
}

// ── check_radicals ──────────────────────────────────────────────────────

#[test]
fn check_radicals_exact_threshold_10() {
    let mut tracker = AchievementTracker::new();
    tracker.check_radicals(10);
    assert!(tracker.unlocked.contains(&"radicals_10"));
}

#[test]
fn check_radicals_below_threshold_unlocks_nothing() {
    let mut tracker = AchievementTracker::new();
    tracker.check_radicals(9);
    assert!(tracker.unlocked.is_empty());
}

// ── check_spells ────────────────────────────────────────────────────────

#[test]
fn check_spells_exact_threshold_5() {
    let mut tracker = AchievementTracker::new();
    tracker.check_spells(5);
    assert!(tracker.unlocked.contains(&"spells_5"));
}

#[test]
fn check_spells_below_threshold_unlocks_nothing() {
    let mut tracker = AchievementTracker::new();
    tracker.check_spells(4);
    assert!(tracker.unlocked.is_empty());
}

// ── check_items ─────────────────────────────────────────────────────────

#[test]
fn check_items_exact_threshold_5() {
    let mut tracker = AchievementTracker::new();
    tracker.check_items(5);
    assert!(tracker.unlocked.contains(&"full_inv"));
}

#[test]
fn check_items_below_threshold_unlocks_nothing() {
    let mut tracker = AchievementTracker::new();
    tracker.check_items(4);
    assert!(tracker.unlocked.is_empty());
}

// ── streak tracking ─────────────────────────────────────────────────────

#[test]
fn record_correct_increments_streak() {
    let mut tracker = AchievementTracker::new();
    tracker.record_correct();
    tracker.record_correct();
    assert_eq!(tracker.correct_streak, 2);
}

#[test]
fn record_miss_after_four_correct_prevents_unlock() {
    let mut tracker = AchievementTracker::new();
    tracker.record_correct();
    tracker.record_correct();
    tracker.record_correct();
    tracker.record_correct();
    tracker.record_miss();
    assert!(!tracker.unlocked.contains(&"perfect_5"));
}

#[test]
fn perfect_5_not_re_queued_on_continued_streak() {
    let mut tracker = AchievementTracker::new();
    // First streak of 5
    for _ in 0..5 {
        tracker.record_correct();
    }
    tracker.pop_popup(); // drain popup
    // Continue to 10
    for _ in 0..5 {
        tracker.record_correct();
    }
    assert_eq!(tracker.pop_popup(), None); // no duplicate popup
}

// ── popup queue ─────────────────────────────────────────────────────────

#[test]
fn pop_popup_returns_none_on_fresh_tracker() {
    let mut tracker = AchievementTracker::new();
    assert_eq!(tracker.pop_popup(), None);
}

#[test]
fn popup_queue_preserves_order_across_many_unlocks() {
    let mut tracker = AchievementTracker::new();
    tracker.unlock("first_kill");
    tracker.unlock("first_chest");
    tracker.unlock("first_forge");

    assert_eq!(tracker.pop_popup(), Some("first_kill"));
    assert_eq!(tracker.pop_popup(), Some("first_chest"));
    assert_eq!(tracker.pop_popup(), Some("first_forge"));
    assert_eq!(tracker.pop_popup(), None);
}

// ── get_def ─────────────────────────────────────────────────────────────

#[test]
fn get_def_returns_correct_fields_for_kill_50() {
    let def = AchievementTracker::get_def("kill_50").unwrap();
    assert_eq!(def.name, "Slayer");
    assert_eq!(def.desc, "Defeat 50 hostiles total");
}

