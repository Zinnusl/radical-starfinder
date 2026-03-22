//! Spaced Repetition System — tracks per-character pinyin accuracy
//! and biases hostile spawning towards characters the player gets wrong.

use std::collections::HashMap;

/// Tracks per-character accuracy for spaced repetition.
pub struct SrsTracker {
    /// Map from hanzi -> (correct_count, total_attempts, last_seen_deck)
    pub stats: HashMap<String, (u32, u32, i32)>,
    /// Current deck number (updated externally so spawn_weight can decay)
    pub current_deck: i32,
}

impl SrsTracker {
    pub fn new() -> Self {
        SrsTracker {
            stats: HashMap::new(),
            current_deck: 0,
        }
    }

    /// Record an attempt (correct or not). Updates last_seen_deck.
    pub fn record(&mut self, hanzi: &str, correct: bool) {
        let deck = self.current_deck;
        let entry = self.stats.entry(hanzi.to_string()).or_insert((0, 0, deck));
        if correct {
            entry.0 += 1;
        }
        entry.1 += 1;
        entry.2 = deck;
    }

    /// Get accuracy for a character (0.0 to 1.0), returns 0.5 if never seen.
    pub fn accuracy(&self, hanzi: &str) -> f64 {
        match self.stats.get(hanzi) {
            Some(&(correct, total, _)) if total > 0 => correct as f64 / total as f64,
            _ => 0.5,
        }
    }

    /// Mastery tier: 0=unknown (never seen), 1=learning (<60% acc or <3 attempts),
    /// 2=familiar (60-85% with 3+ attempts), 3=mastered (>85% with 5+ attempts).
    pub fn mastery_tier(&self, hanzi: &str) -> u8 {
        match self.stats.get(hanzi) {
            None => 0,
            Some(&(correct, total, _)) => {
                if total < 3 {
                    1
                } else {
                    let acc = correct as f64 / total as f64;
                    if acc > 0.85 && total >= 5 {
                        3
                    } else if acc >= 0.6 {
                        2
                    } else {
                        1
                    }
                }
            }
        }
    }

    /// Get spawn weight for a character — lower accuracy = higher weight,
    /// but weights decay back toward 1x if the character hasn't been seen
    /// for several decks (temporal decay).
    pub fn spawn_weight(&self, hanzi: &str) -> u32 {
        let acc = self.accuracy(hanzi);
        let base = if acc < 0.5 {
            4
        } else if acc < 0.7 {
            2
        } else {
            1
        };
        if base <= 1 {
            return 1;
        }
        let deck_gap = match self.stats.get(hanzi) {
            Some(&(_, _, last_deck)) => (self.current_deck - last_deck).max(0),
            None => return 1,
        };
        if deck_gap >= 8 {
            1
        } else if deck_gap >= 4 {
            ((base + 1) / 2).max(1)
        } else {
            base
        }
    }

    /// Pick a vocab entry from a pool, biased by SRS weights.
    /// Takes a random value and returns the index.
    pub fn weighted_pick(&self, pool: &[&crate::vocab::VocabEntry], rand_val: u64) -> usize {
        if pool.is_empty() {
            return 0;
        }
        let weights: Vec<u32> = pool.iter().map(|e| self.spawn_weight(e.hanzi)).collect();
        let total: u64 = weights.iter().map(|&w| w as u64).sum();
        if total == 0 {
            return rand_val as usize % pool.len();
        }
        let mut pick = rand_val % total;
        for (i, &w) in weights.iter().enumerate() {
            if pick < w as u64 {
                return i;
            }
            pick -= w as u64;
        }
        pool.len() - 1
    }

    /// Serialize to JSON string for localStorage.
    pub fn to_json(&self) -> String {
        let mut json = String::from("{\"deck\":");
        json.push_str(&self.current_deck.to_string());
        json.push_str(",\"stats\":{");
        let mut first = true;
        for (hanzi, &(correct, total, last_floor)) in &self.stats {
            if !first {
                json.push(',');
            }
            first = false;
            json.push('"');
            for c in hanzi.chars() {
                match c {
                    '"' => json.push_str("\\\""),
                    '\\' => json.push_str("\\\\"),
                    _ => json.push(c),
                }
            }
            json.push_str("\":[");
            json.push_str(&correct.to_string());
            json.push(',');
            json.push_str(&total.to_string());
            json.push(',');
            json.push_str(&last_floor.to_string());
            json.push(']');
        }
        json.push_str("}}");
        json
    }

    /// Deserialize from JSON string (backward-compatible with 2-element arrays).
    pub fn from_json(json: &str) -> Self {
        let mut tracker = SrsTracker::new();
        let json = json.trim();

        if let Some(deck_start) = json.find("\"deck\":") {
            let rest = &json[deck_start + 7..];
            let end = rest
                .find(|c: char| !c.is_ascii_digit() && c != '-')
                .unwrap_or(rest.len());
            tracker.current_deck = rest[..end].parse::<i32>().unwrap_or(0);
        }

        let stats_start = match json.find("\"stats\":{") {
            Some(pos) => pos + 9,
            None => return tracker,
        };
        let stats_end = match json[stats_start..].rfind('}') {
            Some(pos) => stats_start + pos,
            None => return tracker,
        };
        let inner = &json[stats_start..stats_end];
        if inner.is_empty() {
            return tracker;
        }

        let mut pos = 0;
        let bytes = inner.as_bytes();
        while pos < bytes.len() {
            match inner[pos..].find('"') {
                Some(q) => pos += q + 1,
                None => break,
            }
            let key_end = match inner[pos..].find('"') {
                Some(q) => pos + q,
                None => break,
            };
            let key = inner[pos..key_end].to_string();
            pos = key_end + 1;

            match inner[pos..].find('[') {
                Some(b) => pos += b + 1,
                None => break,
            }
            let bracket_end = match inner[pos..].find(']') {
                Some(b) => pos + b,
                None => break,
            };
            let nums = &inner[pos..bracket_end];
            pos = bracket_end + 1;

            let parts: Vec<&str> = nums.split(',').collect();
            if parts.len() >= 2 {
                let correct = parts[0].trim().parse::<u32>().unwrap_or(0);
                let total = parts[1].trim().parse::<u32>().unwrap_or(0);
                let last_floor = if parts.len() >= 3 {
                    parts[2].trim().parse::<i32>().unwrap_or(0)
                } else {
                    0
                };
                if total > 0 {
                    tracker.stats.insert(key, (correct, total, last_floor));
                }
            }
        }

        tracker
    }
}

/// Load SRS data from localStorage.
pub fn load_srs() -> SrsTracker {
    let storage: Option<web_sys::Storage> =
        web_sys::window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
    storage
        .and_then(|s: web_sys::Storage| s.get_item("radical_starfinder_srs").ok().flatten())
        .map(|json: String| SrsTracker::from_json(&json))
        .unwrap_or_else(SrsTracker::new)
}

/// Save SRS data to localStorage.
pub fn save_srs(tracker: &SrsTracker) {
    let storage: Option<web_sys::Storage> =
        web_sys::window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
    if let Some(storage) = storage {
        let _ = storage.set_item("radical_starfinder_srs", &tracker.to_json());
    }
}

#[cfg(test)]
mod tests {
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
}
