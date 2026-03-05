//! Spaced Repetition System — tracks per-character pinyin accuracy
//! and biases enemy spawning towards characters the player gets wrong.

use std::collections::HashMap;

/// Tracks per-character accuracy for spaced repetition.
pub struct SrsTracker {
    /// Map from hanzi -> (correct_count, total_attempts)
    pub stats: HashMap<String, (u32, u32)>,
}

impl SrsTracker {
    pub fn new() -> Self {
        SrsTracker {
            stats: HashMap::new(),
        }
    }

    /// Record an attempt (correct or not).
    pub fn record(&mut self, hanzi: &str, correct: bool) {
        let entry = self.stats.entry(hanzi.to_string()).or_insert((0, 0));
        if correct {
            entry.0 += 1;
        }
        entry.1 += 1;
    }

    /// Get accuracy for a character (0.0 to 1.0), returns 0.5 if never seen.
    pub fn accuracy(&self, hanzi: &str) -> f64 {
        match self.stats.get(hanzi) {
            Some(&(correct, total)) if total > 0 => correct as f64 / total as f64,
            _ => 0.5,
        }
    }

    /// Get spawn weight for a character — lower accuracy = higher weight.
    /// Characters with <50% accuracy get 4x weight, <70% get 2x, rest get 1x.
    pub fn spawn_weight(&self, hanzi: &str) -> u32 {
        let acc = self.accuracy(hanzi);
        if acc < 0.5 {
            4
        } else if acc < 0.7 {
            2
        } else {
            1
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
        let mut json = String::from("{\"stats\":{");
        let mut first = true;
        for (hanzi, &(correct, total)) in &self.stats {
            if !first {
                json.push(',');
            }
            first = false;
            json.push('"');
            // Escape any quotes in hanzi (unlikely but safe)
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
            json.push(']');
        }
        json.push_str("}}");
        json
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Self {
        let mut tracker = SrsTracker::new();
        // Expected format: {"stats":{"大":[5,8],"好":[3,3]}}
        let json = json.trim();
        // Find the inner stats object
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

        // Parse key-value pairs: "hanzi":[correct,total]
        let mut pos = 0;
        let bytes = inner.as_bytes();
        while pos < bytes.len() {
            // Find opening quote for key
            match inner[pos..].find('"') {
                Some(q) => pos += q + 1,
                None => break,
            }
            // Find closing quote for key
            let key_end = match inner[pos..].find('"') {
                Some(q) => pos + q,
                None => break,
            };
            let key = inner[pos..key_end].to_string();
            pos = key_end + 1;

            // Find '['
            match inner[pos..].find('[') {
                Some(b) => pos += b + 1,
                None => break,
            }
            // Find ']'
            let bracket_end = match inner[pos..].find(']') {
                Some(b) => pos + b,
                None => break,
            };
            let nums = &inner[pos..bracket_end];
            pos = bracket_end + 1;

            // Parse "correct,total"
            if let Some(comma) = nums.find(',') {
                let correct = nums[..comma].trim().parse::<u32>().unwrap_or(0);
                let total = nums[comma + 1..].trim().parse::<u32>().unwrap_or(0);
                if total > 0 {
                    tracker.stats.insert(key, (correct, total));
                }
            }
        }

        tracker
    }
}

/// Load SRS data from localStorage.
pub fn load_srs() -> SrsTracker {
    let storage: Option<web_sys::Storage> = web_sys::window()
        .and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
    storage
        .and_then(|s: web_sys::Storage| s.get_item("radical_roguelike_srs").ok().flatten())
        .map(|json: String| SrsTracker::from_json(&json))
        .unwrap_or_else(SrsTracker::new)
}

/// Save SRS data to localStorage.
pub fn save_srs(tracker: &SrsTracker) {
    let storage: Option<web_sys::Storage> = web_sys::window()
        .and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
    if let Some(storage) = storage {
        let _ = storage.set_item("radical_roguelike_srs", &tracker.to_json());
    }
}
