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

// ── Additional accuracy tests ──

#[test]
fn accuracy_perfect_score() {
    let entry = CodexEntry {
        hanzi: "人", pinyin: "rén", meaning: "person",
        times_seen: 20, times_correct: 20,
    };
    assert!((entry.accuracy() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn accuracy_all_wrong() {
    let entry = CodexEntry {
        hanzi: "天", pinyin: "tiān", meaning: "sky",
        times_seen: 5, times_correct: 0,
    };
    assert!((entry.accuracy() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn accuracy_single_attempt_correct() {
    let entry = CodexEntry {
        hanzi: "地", pinyin: "dì", meaning: "earth",
        times_seen: 1, times_correct: 1,
    };
    assert!((entry.accuracy() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn accuracy_single_attempt_wrong() {
    let entry = CodexEntry {
        hanzi: "风", pinyin: "fēng", meaning: "wind",
        times_seen: 1, times_correct: 0,
    };
    assert!((entry.accuracy() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn accuracy_half() {
    let entry = CodexEntry {
        hanzi: "雨", pinyin: "yǔ", meaning: "rain",
        times_seen: 100, times_correct: 50,
    };
    assert!((entry.accuracy() - 0.5).abs() < 1e-9);
}

// ── Additional record tests ──

#[test]
fn record_only_incorrect() {
    let mut codex = Codex::new();
    codex.record("日", "rì", "sun", false);
    codex.record("日", "rì", "sun", false);
    codex.record("日", "rì", "sun", false);
    let entry = codex.entries.get("日").unwrap();
    assert_eq!(entry.times_seen, 3);
    assert_eq!(entry.times_correct, 0);
    assert!((entry.accuracy() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn record_only_correct() {
    let mut codex = Codex::new();
    codex.record("月", "yuè", "moon", true);
    codex.record("月", "yuè", "moon", true);
    let entry = codex.entries.get("月").unwrap();
    assert_eq!(entry.times_seen, 2);
    assert_eq!(entry.times_correct, 2);
}

#[test]
fn record_many_different_characters() {
    let mut codex = Codex::new();
    codex.record("金", "jīn", "gold", true);
    codex.record("木", "mù", "wood", false);
    codex.record("水", "shuǐ", "water", true);
    codex.record("火", "huǒ", "fire", false);
    codex.record("土", "tǔ", "earth", true);
    assert_eq!(codex.total_unique(), 5);
}

#[test]
fn record_preserves_metadata() {
    let mut codex = Codex::new();
    codex.record("花", "huā", "flower", true);
    let entry = codex.entries.get("花").unwrap();
    assert_eq!(entry.hanzi, "花");
    assert_eq!(entry.pinyin, "huā");
    assert_eq!(entry.meaning, "flower");
}

// ── Additional sorted_entries tests ──

#[test]
fn sorted_entries_empty_codex() {
    let codex = Codex::new();
    assert!(codex.sorted_entries().is_empty());
}

#[test]
fn sorted_entries_single_entry() {
    let mut codex = Codex::new();
    codex.record("心", "xīn", "heart", true);
    let sorted = codex.sorted_entries();
    assert_eq!(sorted.len(), 1);
    assert_eq!(sorted[0].hanzi, "心");
}

#[test]
fn sorted_entries_equal_counts() {
    let mut codex = Codex::new();
    codex.record("左", "zuǒ", "left", true);
    codex.record("右", "yòu", "right", true);
    let sorted = codex.sorted_entries();
    assert_eq!(sorted.len(), 2);
    // Both seen once — order is stable but either is valid
    assert_eq!(sorted[0].times_seen, sorted[1].times_seen);
}

// ── Additional total_unique tests ──

#[test]
fn total_unique_empty() {
    let codex = Codex::new();
    assert_eq!(codex.total_unique(), 0);
}

#[test]
fn total_unique_many_records_same_char() {
    let mut codex = Codex::new();
    for _ in 0..100 {
        codex.record("学", "xué", "learn", true);
    }
    assert_eq!(codex.total_unique(), 1);
}

// ── Codex::new() tests ──

#[test]
fn new_codex_is_empty() {
    let codex = Codex::new();
    assert!(codex.entries.is_empty());
    assert_eq!(codex.total_unique(), 0);
    assert!(codex.sorted_entries().is_empty());
}

