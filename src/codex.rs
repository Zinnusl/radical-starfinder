//! Character Xenopedia — encyclopedia of encountered hostile creatures.

use std::collections::HashMap;

/// Entry for a single character in the codex.
#[derive(Clone)]
pub struct CodexEntry {
    pub hanzi: &'static str,
    pub pinyin: &'static str,
    pub meaning: &'static str,
    pub times_seen: u32,
    pub times_correct: u32,
}

impl CodexEntry {
    pub fn accuracy(&self) -> f64 {
        if self.times_seen == 0 {
            0.0
        } else {
            self.times_correct as f64 / self.times_seen as f64
        }
    }
}

/// The character codex — tracks all encountered characters.
pub struct Codex {
    pub entries: HashMap<&'static str, CodexEntry>,
}

impl Codex {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Record an encounter with a character.
    pub fn record(
        &mut self,
        hanzi: &'static str,
        pinyin: &'static str,
        meaning: &'static str,
        correct: bool,
    ) {
        let entry = self.entries.entry(hanzi).or_insert(CodexEntry {
            hanzi,
            pinyin,
            meaning,
            times_seen: 0,
            times_correct: 0,
        });
        entry.times_seen += 1;
        if correct {
            entry.times_correct += 1;
        }
    }

    /// Get sorted entries (by times seen, descending).
    pub fn sorted_entries(&self) -> Vec<&CodexEntry> {
        let mut entries: Vec<&CodexEntry> = self.entries.values().collect();
        entries.sort_by(|a, b| b.times_seen.cmp(&a.times_seen));
        entries
    }

    #[allow(dead_code)]
    pub fn total_unique(&self) -> usize {
        self.entries.len()
    }

    /// Save to localStorage.
    pub fn save(&self) {
        let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());
        if let Some(storage) = storage {
            // Format: hanzi|pinyin|meaning|seen|correct;hanzi2|...
            let data: String = self
                .entries
                .values()
                .map(|e| {
                    format!(
                        "{}|{}|{}|{}|{}",
                        e.hanzi, e.pinyin, e.meaning, e.times_seen, e.times_correct
                    )
                })
                .collect::<Vec<_>>()
                .join(";");
            let _ = storage.set_item("radical_starfinder_codex", &data);
        }
    }

    /// Load from localStorage.
    pub fn load(vocab_pool: &[crate::vocab::VocabEntry]) -> Self {
        let mut codex = Self::new();
        let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());
        if let Some(storage) = storage {
            if let Ok(Some(data)) = storage.get_item("radical_starfinder_codex") {
                for entry_str in data.split(';') {
                    let parts: Vec<&str> = entry_str.split('|').collect();
                    if parts.len() >= 5 {
                        let hanzi_str = parts[0];
                        // Find matching static str from vocab pool
                        if let Some(ve) = vocab_pool.iter().find(|v| v.hanzi == hanzi_str) {
                            let seen = parts[3].parse::<u32>().unwrap_or(0);
                            let correct = parts[4].parse::<u32>().unwrap_or(0);
                            let entry = codex.entries.entry(ve.hanzi).or_insert(CodexEntry {
                                hanzi: ve.hanzi,
                                pinyin: ve.pinyin,
                                meaning: ve.meaning,
                                times_seen: 0,
                                times_correct: 0,
                            });
                            entry.times_seen = seen;
                            entry.times_correct = correct;
                        }
                    }
                }
            }
        }
        codex
    }
}

#[cfg(test)]
mod tests {
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
}