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

// --------------- check_pinyin ---------------

#[test]
fn check_pinyin_is_case_insensitive() {
    assert!(check_pinyin(friend_entry(), "PENG2YOU3"));
}

#[test]
fn check_pinyin_rejects_wrong_pinyin() {
    assert!(!check_pinyin(friend_entry(), "ma1"));
}

#[test]
fn check_pinyin_rejects_empty_input() {
    assert!(!check_pinyin(friend_entry(), ""));
}

#[test]
fn check_pinyin_accepts_mixed_case() {
    assert!(check_pinyin(friend_entry(), "Peng2You3"));
}

// --------------- check_pinyin_partial ---------------

use super::check_pinyin_partial;

#[test]
fn check_pinyin_partial_accepts_toneless_input() {
    assert!(check_pinyin_partial(friend_entry(), "pengyou"));
}

#[test]
fn check_pinyin_partial_rejects_exact_match() {
    assert!(!check_pinyin_partial(friend_entry(), "peng2you3"));
}

#[test]
fn check_pinyin_partial_rejects_wrong_syllables() {
    assert!(!check_pinyin_partial(friend_entry(), "mama"));
}

#[test]
fn check_pinyin_partial_rejects_empty_input() {
    assert!(!check_pinyin_partial(friend_entry(), ""));
}

#[test]
fn check_pinyin_partial_is_case_insensitive() {
    assert!(check_pinyin_partial(friend_entry(), "PENGYOU"));
}

// --------------- pinyin_syllables ---------------

#[test]
fn pinyin_syllables_returns_single_syllable_for_single_tone() {
    assert_eq!(pinyin_syllables("ma1"), vec!["ma1"]);
}

#[test]
fn pinyin_syllables_returns_input_when_no_tone_digits() {
    assert_eq!(pinyin_syllables("hello"), vec!["hello"]);
}

#[test]
fn pinyin_syllables_splits_two_syllables() {
    assert_eq!(pinyin_syllables("peng2you3"), vec!["peng2", "you3"]);
}

#[test]
fn pinyin_syllables_splits_four_syllables() {
    assert_eq!(
        pinyin_syllables("ni3hao3ma5a5"),
        vec!["ni3", "hao3", "ma5", "a5"]
    );
}

// --------------- resolve_compound_pinyin_step ---------------

#[test]
fn resolve_compound_pinyin_step_accepts_case_insensitive_input() {
    assert_eq!(
        resolve_compound_pinyin_step("peng2you3", 0, "PENG2"),
        CompoundPinyinStep::Advanced {
            matched: "peng2",
            next_progress: 1,
            total: 2,
        }
    );
}

#[test]
fn resolve_compound_pinyin_step_single_syllable_completes_immediately() {
    assert_eq!(
        resolve_compound_pinyin_step("ma1", 0, "ma1"),
        CompoundPinyinStep::Completed {
            matched: "ma1",
            total: 1,
        }
    );
}

// --------------- is_elite ---------------

use super::is_elite;

#[test]
fn is_elite_returns_true_for_multi_char_hanzi() {
    assert!(is_elite(friend_entry()));
}

#[test]
fn is_elite_returns_false_for_single_char_hanzi() {
    let single = VOCAB.iter().find(|e| e.hanzi.chars().count() == 1).unwrap();
    assert!(!is_elite(single));
}

// --------------- split_hanzi_chars ---------------

use super::split_hanzi_chars;

#[test]
fn split_hanzi_chars_zips_multi_char_with_syllables() {
    let result = split_hanzi_chars("朋友", "peng2you3");
    assert_eq!(
        result,
        vec![
            ("朋".to_string(), "peng2".to_string()),
            ("友".to_string(), "you3".to_string()),
        ]
    );
}

#[test]
fn split_hanzi_chars_returns_whole_for_single_char() {
    let result = split_hanzi_chars("大", "da4");
    assert_eq!(result, vec![("大".to_string(), "da4".to_string())]);
}

#[test]
fn split_hanzi_chars_returns_whole_when_count_mismatch() {
    let result = split_hanzi_chars("朋友", "peng2you3men5");
    assert_eq!(
        result,
        vec![("朋友".to_string(), "peng2you3men5".to_string())]
    );
}

#[test]
fn split_hanzi_chars_handles_three_char_hanzi() {
    let result = split_hanzi_chars("朋友们", "peng2you3men5");
    assert_eq!(
        result,
        vec![
            ("朋".to_string(), "peng2".to_string()),
            ("友".to_string(), "you3".to_string()),
            ("们".to_string(), "men5".to_string()),
        ]
    );
}

// --------------- vocab_for_floor ---------------

use super::vocab_for_floor;

#[test]
fn vocab_for_floor_1_returns_only_hsk1() {
    let entries = vocab_for_floor(1);
    assert!(entries.iter().all(|e| e.hsk == 1));
}

#[test]
fn vocab_for_floor_1_returns_non_empty() {
    assert!(!vocab_for_floor(1).is_empty());
}

#[test]
fn vocab_for_floor_6_includes_hsk2() {
    let entries = vocab_for_floor(6);
    assert!(entries.iter().any(|e| e.hsk == 2));
}

#[test]
fn vocab_for_floor_30_includes_hsk6() {
    let entries = vocab_for_floor(30);
    assert!(entries.iter().any(|e| e.hsk == 6));
}

#[test]
fn vocab_for_floor_0_returns_all_entries() {
    let all_count = VOCAB.len();
    let floor0_count = vocab_for_floor(0).len();
    assert_eq!(floor0_count, all_count);
}

#[test]
fn vocab_for_floor_higher_floor_returns_more_entries() {
    let low = vocab_for_floor(1).len();
    let high = vocab_for_floor(30).len();
    assert!(high > low);
}

// --------------- vocab_entry_by_hanzi ---------------

use super::vocab_entry_by_hanzi;

#[test]
fn vocab_entry_by_hanzi_finds_existing_entry() {
    let entry = vocab_entry_by_hanzi("朋友");
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().hanzi, "朋友");
}

#[test]
fn vocab_entry_by_hanzi_returns_none_for_nonexistent() {
    assert!(vocab_entry_by_hanzi("ZZZZZ").is_none());
}

#[test]
fn vocab_entry_by_hanzi_returns_none_for_empty_string() {
    assert!(vocab_entry_by_hanzi("").is_none());
}

// --------------- sentences_for_floor ---------------

use super::sentences_for_floor;

#[test]
fn sentences_for_floor_1_returns_only_hsk1() {
    let sents = sentences_for_floor(1);
    assert!(sents.iter().all(|s| s.hsk == 1));
}

#[test]
fn sentences_for_floor_1_returns_non_empty() {
    assert!(!sentences_for_floor(1).is_empty());
}

#[test]
fn sentences_for_floor_6_includes_hsk2() {
    let sents = sentences_for_floor(6);
    assert!(sents.iter().any(|s| s.hsk == 2));
}

#[test]
fn sentences_for_floor_30_includes_hsk6() {
    let sents = sentences_for_floor(30);
    assert!(sents.iter().any(|s| s.hsk == 6));
}

#[test]
fn sentences_for_higher_floor_returns_more_entries() {
    let low = sentences_for_floor(1).len();
    let high = sentences_for_floor(30).len();
    assert!(high > low);
}

