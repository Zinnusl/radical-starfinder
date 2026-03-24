use super::*;

#[test]
fn accuracy_zero_when_never_seen() {
    let entry = CodexEntry {
        hanzi: "大", pinyin: "dà", meaning: "big",
        times_seen: 0, times_correct: 0,
    };
    assert!((entry.accuracy() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn accuracy_calculated_correctly() {
    let entry = CodexEntry {
        hanzi: "水", pinyin: "shuǐ", meaning: "water",
        times_seen: 10, times_correct: 7,
    };
    assert!((entry.accuracy() - 0.7).abs() < 1e-9);
}

#[test]
fn record_creates_and_updates_entry() {
    let mut codex = Codex::new();
    codex.record("火", "huǒ", "fire", true);
    let entry = codex.entries.get("火").unwrap();
    assert_eq!(entry.times_seen, 1);
    assert_eq!(entry.times_correct, 1);

    codex.record("火", "huǒ", "fire", false);
    let entry = codex.entries.get("火").unwrap();
    assert_eq!(entry.times_seen, 2);
    assert_eq!(entry.times_correct, 1);
}

#[test]
fn sorted_entries_ordered_by_times_seen_desc() {
    let mut codex = Codex::new();
    codex.record("一", "yī", "one", true);
    for _ in 0..5 {
        codex.record("二", "èr", "two", true);
    }
    for _ in 0..3 {
        codex.record("三", "sān", "three", false);
    }
    let sorted = codex.sorted_entries();
    assert_eq!(sorted[0].hanzi, "二"); // 5 times
    assert_eq!(sorted[1].hanzi, "三"); // 3 times
    assert_eq!(sorted[2].hanzi, "一"); // 1 time
}

#[test]
fn total_unique_counts_distinct_characters() {
    let mut codex = Codex::new();
    assert_eq!(codex.total_unique(), 0);
    codex.record("山", "shān", "mountain", true);
    codex.record("山", "shān", "mountain", true); // same char
    codex.record("木", "mù", "tree", false);
    assert_eq!(codex.total_unique(), 2);
}

