//! Vocabulary data — Hanzi with pinyin, meaning, and HSK level.
//!
//! Static arrays compiled into the binary.

#[derive(Clone, Copy, Debug)]
pub struct VocabEntry {
    pub hanzi: &'static str,
    pub pinyin: &'static str,
    pub meaning: &'static str,
    pub hsk: u8,               // 1–6
    pub example: &'static str, // example sentence (empty if none)
}

include!(concat!(env!("OUT_DIR"), "/vocab_data.rs"));

/// Get vocab entries for a given max HSK level.
pub fn vocab_for_floor(floor: i32) -> Vec<&'static VocabEntry> {
    let max_hsk = match floor {
        1..=5 => 1,
        6..=10 => 2,
        11..=15 => 3,
        16..=20 => 4,
        21..=25 => 5,
        26..=30 => 6,
        _ => 7,
    };
    VOCAB.iter().filter(|v| v.hsk <= max_hsk).collect()
}

/// Find a vocab entry by its Hanzi character(s).
pub fn vocab_entry_by_hanzi(hanzi: &str) -> Option<&'static VocabEntry> {
    VOCAB.iter().find(|v| v.hanzi == hanzi)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompoundPinyinStep<'a> {
    Miss {
        expected: &'a str,
        total: usize,
    },
    Advanced {
        matched: &'a str,
        next_progress: usize,
        total: usize,
    },
    Completed {
        matched: &'a str,
        total: usize,
    },
}

fn normalized_pinyin(input: &str) -> String {
    input.replace(' ', "")
}

/// Check if `input` is a valid pinyin for the given hanzi.
/// Accepts concatenated ("peng2you3") or space-separated ("peng2 you3") input.
pub fn check_pinyin(entry: &VocabEntry, input: &str) -> bool {
    entry.pinyin.eq_ignore_ascii_case(&normalized_pinyin(input))
}

pub fn pinyin_syllables(pinyin: &str) -> Vec<&str> {
    let mut syllables = Vec::new();
    let mut start = 0;
    for (idx, ch) in pinyin.char_indices() {
        if ch.is_ascii_digit() {
            let end = idx + ch.len_utf8();
            syllables.push(&pinyin[start..end]);
            start = end;
        }
    }
    if syllables.is_empty() {
        syllables.push(pinyin);
    }
    syllables
}

pub fn resolve_compound_pinyin_step<'a>(
    pinyin: &'a str,
    progress: usize,
    input: &str,
) -> CompoundPinyinStep<'a> {
    let syllables = pinyin_syllables(pinyin);
    let total = syllables.len().max(1);
    let current_idx = progress.min(total - 1);
    let expected = syllables[current_idx];
    if expected.eq_ignore_ascii_case(&normalized_pinyin(input)) {
        if current_idx + 1 == total {
            CompoundPinyinStep::Completed {
                matched: expected,
                total,
            }
        } else {
            CompoundPinyinStep::Advanced {
                matched: expected,
                next_progress: current_idx + 1,
                total,
            }
        }
    } else {
        CompoundPinyinStep::Miss { expected, total }
    }
}

/// Returns true if this vocab entry is a multi-character word (elite).
pub fn is_elite(entry: &VocabEntry) -> bool {
    entry.hanzi.chars().count() > 1
}

#[cfg(test)]
mod tests {
    use super::{
        check_pinyin, pinyin_syllables, resolve_compound_pinyin_step, CompoundPinyinStep, VOCAB,
    };

    fn friend_entry() -> &'static super::VocabEntry {
        VOCAB.iter().find(|entry| entry.hanzi == "朋友").unwrap()
    }

    fn friends_entry() -> &'static super::VocabEntry {
        VOCAB.iter().find(|entry| entry.hanzi == "朋友们").unwrap()
    }

    #[test]
    fn pengyoumen_has_correct_pinyin() {
        let entry = friends_entry();
        assert_eq!(entry.pinyin, "peng2you3men5");
    }

    #[test]
    fn pengyoumen_has_correct_meaning() {
        let entry = friends_entry();
        assert_eq!(entry.meaning, "friends");
    }

    #[test]
    fn pengyoumen_check_pinyin_concatenated() {
        assert!(check_pinyin(friends_entry(), "peng2you3men5"));
    }

    #[test]
    fn pengyoumen_check_pinyin_space_separated() {
        assert!(check_pinyin(friends_entry(), "peng2 you3 men5"));
    }

    #[test]
    fn pengyoumen_compound_step_advances() {
        assert_eq!(
            resolve_compound_pinyin_step("peng2you3men5", 0, "peng2"),
            CompoundPinyinStep::Advanced {
                matched: "peng2",
                next_progress: 1,
                total: 3,
            }
        );
    }

    #[test]
    fn pengyoumen_compound_step_completes_on_final() {
        assert_eq!(
            resolve_compound_pinyin_step("peng2you3men5", 2, "men5"),
            CompoundPinyinStep::Completed {
                matched: "men5",
                total: 3,
            }
        );
    }

    #[test]
    fn check_pinyin_accepts_space_separated_compound_input() {
        assert!(check_pinyin(friend_entry(), "peng2 you3"));
    }

    #[test]
    fn pinyin_syllables_split_compound_words_on_tone_numbers() {
        assert_eq!(pinyin_syllables("dui4bu4qi3"), vec!["dui4", "bu4", "qi3"]);
    }

    #[test]
    fn resolve_compound_pinyin_step_advances_on_correct_next_syllable() {
        assert_eq!(
            resolve_compound_pinyin_step("peng2you3", 0, "peng2"),
            CompoundPinyinStep::Advanced {
                matched: "peng2",
                next_progress: 1,
                total: 2,
            }
        );
    }

    #[test]
    fn resolve_compound_pinyin_step_completes_on_final_syllable() {
        assert_eq!(
            resolve_compound_pinyin_step("peng2you3", 1, "you3"),
            CompoundPinyinStep::Completed {
                matched: "you3",
                total: 2,
            }
        );
    }

    #[test]
    fn resolve_compound_pinyin_step_reports_expected_syllable_on_miss() {
        assert_eq!(
            resolve_compound_pinyin_step("peng2you3", 1, "peng2"),
            CompoundPinyinStep::Miss {
                expected: "you3",
                total: 2,
            }
        );
    }

    #[test]
    fn wenyijie_has_three_syllable_pinyin() {
        let entry = VOCAB.iter().find(|e| e.hanzi == "文艺界");
        if let Some(e) = entry {
            assert_eq!(pinyin_syllables(e.pinyin).len(), 3);
        }
    }

    #[test]
    fn chenggonglv_has_three_syllable_pinyin() {
        let entry = VOCAB.iter().find(|e| e.hanzi == "成功率");
        if let Some(e) = entry {
            assert_eq!(pinyin_syllables(e.pinyin).len(), 3);
        }
    }
}
