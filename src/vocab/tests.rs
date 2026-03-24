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

