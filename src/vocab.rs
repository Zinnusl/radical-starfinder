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

pub fn normalized_pinyin(input: &str) -> String {
    input.replace(' ', "")
}

/// Check if `input` is a valid pinyin for the given hanzi.
/// Accepts concatenated ("peng2you3") or space-separated ("peng2 you3") input.
pub fn check_pinyin(entry: &VocabEntry, input: &str) -> bool {
    entry.pinyin.eq_ignore_ascii_case(&normalized_pinyin(input))
}

pub fn check_pinyin_partial(entry: &VocabEntry, input: &str) -> bool {
    let norm = normalized_pinyin(input);
    if entry.pinyin.eq_ignore_ascii_case(&norm) {
        return false;
    }
    let strip_tones = |s: &str| -> String {
        s.chars()
            .filter(|c| !c.is_ascii_digit())
            .collect::<String>()
            .to_lowercase()
    };
    let expected_base = strip_tones(entry.pinyin);
    let input_base = strip_tones(&norm);
    expected_base == input_base && !input_base.is_empty()
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

pub fn split_hanzi_chars(hanzi: &str, pinyin: &str) -> Vec<(String, String)> {
    let chars: Vec<char> = hanzi.chars().collect();
    let syllables = pinyin_syllables(pinyin);
    if chars.len() == 1 || syllables.len() != chars.len() {
        return vec![(hanzi.to_string(), pinyin.to_string())];
    }
    chars
        .into_iter()
        .zip(syllables.into_iter())
        .map(|(ch, syl)| (ch.to_string(), syl.to_string()))
        .collect()
}

pub struct SentenceEntry {
    pub hanzi: &'static str,
    pub pinyin: &'static str,
    pub meaning: &'static str,
    pub hsk: u8,
}

pub fn sentences_for_floor(floor: i32) -> &'static [SentenceEntry] {
    let max_hsk = match floor {
        1..=5 => 1,
        6..=10 => 2,
        11..=15 => 3,
        16..=20 => 4,
        21..=25 => 5,
        _ => 6,
    };
    let end = SENTENCES
        .iter()
        .position(|s| s.hsk > max_hsk)
        .unwrap_or(SENTENCES.len());
    &SENTENCES[..end]
}

static SENTENCES: &[SentenceEntry] = &[
    // HSK 1
    SentenceEntry {
        hanzi: "你好",
        pinyin: "ni3hao3",
        meaning: "hello",
        hsk: 1,
    },
    SentenceEntry {
        hanzi: "谢谢",
        pinyin: "xie4xie4",
        meaning: "thanks",
        hsk: 1,
    },
    SentenceEntry {
        hanzi: "再见",
        pinyin: "zai4jian4",
        meaning: "goodbye",
        hsk: 1,
    },
    SentenceEntry {
        hanzi: "我是学生",
        pinyin: "wo3shi4xue2sheng1",
        meaning: "I am a student",
        hsk: 1,
    },
    SentenceEntry {
        hanzi: "你吃饭了吗",
        pinyin: "ni3chi1fan4le5ma5",
        meaning: "have you eaten?",
        hsk: 1,
    },
    SentenceEntry {
        hanzi: "他是中国人",
        pinyin: "ta1shi4zhong1guo2ren2",
        meaning: "he is Chinese",
        hsk: 1,
    },
    SentenceEntry {
        hanzi: "我爱你",
        pinyin: "wo3ai4ni3",
        meaning: "I love you",
        hsk: 1,
    },
    SentenceEntry {
        hanzi: "今天很热",
        pinyin: "jin1tian1hen3re4",
        meaning: "today is hot",
        hsk: 1,
    },
    SentenceEntry {
        hanzi: "我不知道",
        pinyin: "wo3bu4zhi1dao4",
        meaning: "I don't know",
        hsk: 1,
    },
    SentenceEntry {
        hanzi: "请坐",
        pinyin: "qing3zuo4",
        meaning: "please sit",
        hsk: 1,
    },
    // HSK 2
    SentenceEntry {
        hanzi: "欢迎你来",
        pinyin: "huan1ying2ni3lai2",
        meaning: "welcome",
        hsk: 2,
    },
    SentenceEntry {
        hanzi: "你几岁了",
        pinyin: "ni3ji3sui4le5",
        meaning: "how old are you?",
        hsk: 2,
    },
    SentenceEntry {
        hanzi: "对不起",
        pinyin: "dui4bu4qi3",
        meaning: "sorry",
        hsk: 2,
    },
    SentenceEntry {
        hanzi: "没有关系",
        pinyin: "mei2you3guan1xi5",
        meaning: "no problem",
        hsk: 2,
    },
    SentenceEntry {
        hanzi: "我想去北京",
        pinyin: "wo3xiang3qu4bei3jing1",
        meaning: "I want to go to Beijing",
        hsk: 2,
    },
    SentenceEntry {
        hanzi: "你在做什么",
        pinyin: "ni3zai4zuo4shen2me5",
        meaning: "what are you doing?",
        hsk: 2,
    },
    SentenceEntry {
        hanzi: "我很高兴",
        pinyin: "wo3hen3gao1xing4",
        meaning: "I am happy",
        hsk: 2,
    },
    SentenceEntry {
        hanzi: "明天见",
        pinyin: "ming2tian1jian4",
        meaning: "see you tomorrow",
        hsk: 2,
    },
    SentenceEntry {
        hanzi: "请问路怎么走",
        pinyin: "qing3wen4lu4zen3me5zou3",
        meaning: "excuse me, how to get there?",
        hsk: 2,
    },
    SentenceEntry {
        hanzi: "他会说中文",
        pinyin: "ta1hui4shuo1zhong1wen2",
        meaning: "he can speak Chinese",
        hsk: 2,
    },
    // HSK 3
    SentenceEntry {
        hanzi: "我觉得很有意思",
        pinyin: "wo3jue2de5hen3you3yi4si5",
        meaning: "I think it's interesting",
        hsk: 3,
    },
    SentenceEntry {
        hanzi: "这个多少钱",
        pinyin: "zhe4ge5duo1shao3qian2",
        meaning: "how much is this?",
        hsk: 3,
    },
    SentenceEntry {
        hanzi: "天气越来越冷",
        pinyin: "tian1qi4yue4lai2yue4leng3",
        meaning: "the weather is getting colder",
        hsk: 3,
    },
    SentenceEntry {
        hanzi: "你打算怎么办",
        pinyin: "ni3da3suan4zen3me5ban4",
        meaning: "what do you plan to do?",
        hsk: 3,
    },
    SentenceEntry {
        hanzi: "别担心",
        pinyin: "bie2dan1xin1",
        meaning: "don't worry",
        hsk: 3,
    },
    SentenceEntry {
        hanzi: "我们一起去吧",
        pinyin: "wo3men5yi4qi3qu4ba5",
        meaning: "let's go together",
        hsk: 3,
    },
    SentenceEntry {
        hanzi: "他已经走了",
        pinyin: "ta1yi3jing1zou3le5",
        meaning: "he already left",
        hsk: 3,
    },
    SentenceEntry {
        hanzi: "请帮我一下",
        pinyin: "qing3bang1wo3yi2xia4",
        meaning: "please help me",
        hsk: 3,
    },
    SentenceEntry {
        hanzi: "你有没有空",
        pinyin: "ni3you3mei2you3kong4",
        meaning: "are you free?",
        hsk: 3,
    },
    SentenceEntry {
        hanzi: "我很喜欢这里",
        pinyin: "wo3hen3xi3huan1zhe4li3",
        meaning: "I like it here",
        hsk: 3,
    },
    // HSK 4
    SentenceEntry {
        hanzi: "他的态度不太好",
        pinyin: "ta1de5tai4du4bu2tai4hao3",
        meaning: "his attitude isn't great",
        hsk: 4,
    },
    SentenceEntry {
        hanzi: "我们应该保护环境",
        pinyin: "wo3men5ying1gai1bao3hu4huan2jing4",
        meaning: "we should protect the environment",
        hsk: 4,
    },
    SentenceEntry {
        hanzi: "这件事情很复杂",
        pinyin: "zhe4jian4shi4qing2hen3fu4za2",
        meaning: "this matter is complex",
        hsk: 4,
    },
    SentenceEntry {
        hanzi: "他经常迟到",
        pinyin: "ta1jing1chang2chi2dao4",
        meaning: "he is often late",
        hsk: 4,
    },
    SentenceEntry {
        hanzi: "希望你能成功",
        pinyin: "xi1wang4ni3neng2cheng2gong1",
        meaning: "I hope you succeed",
        hsk: 4,
    },
    SentenceEntry {
        hanzi: "请尽量早点来",
        pinyin: "qing3jin3liang4zao3dian3lai2",
        meaning: "please come as early as possible",
        hsk: 4,
    },
    SentenceEntry {
        hanzi: "我对这个很感兴趣",
        pinyin: "wo3dui4zhe4ge5hen3gan3xing4qu4",
        meaning: "I'm interested in this",
        hsk: 4,
    },
    SentenceEntry {
        hanzi: "你能不能帮我翻译",
        pinyin: "ni3neng2bu4neng2bang1wo3fan1yi4",
        meaning: "can you help me translate?",
        hsk: 4,
    },
    SentenceEntry {
        hanzi: "我正在考虑这个问题",
        pinyin: "wo3zheng4zai4kao3lv4zhe4ge5wen4ti2",
        meaning: "I'm considering this problem",
        hsk: 4,
    },
    SentenceEntry {
        hanzi: "他们决定推迟会议",
        pinyin: "ta1men5jue2ding4tui1chi2hui4yi4",
        meaning: "they decided to postpone the meeting",
        hsk: 4,
    },
    // HSK 5
    SentenceEntry {
        hanzi: "我们必须面对现实",
        pinyin: "wo3men5bi4xu1mian4dui4xian4shi2",
        meaning: "we must face reality",
        hsk: 5,
    },
    SentenceEntry {
        hanzi: "他的表现令人失望",
        pinyin: "ta1de5biao3xian4ling4ren2shi1wang4",
        meaning: "his performance is disappointing",
        hsk: 5,
    },
    SentenceEntry {
        hanzi: "这个方案值得考虑",
        pinyin: "zhe4ge5fang1an4zhi2de5kao3lv4",
        meaning: "this plan is worth considering",
        hsk: 5,
    },
    SentenceEntry {
        hanzi: "请不要浪费时间",
        pinyin: "qing3bu2yao4lang4fei4shi2jian1",
        meaning: "please don't waste time",
        hsk: 5,
    },
    SentenceEntry {
        hanzi: "我们需要更多的耐心",
        pinyin: "wo3men5xu1yao4geng4duo1de5nai4xin1",
        meaning: "we need more patience",
        hsk: 5,
    },
    // HSK 6
    SentenceEntry {
        hanzi: "他在竞争中脱颖而出",
        pinyin: "ta1zai4jing4zheng1zhong1tuo1ying3er2chu1",
        meaning: "he stood out in the competition",
        hsk: 6,
    },
    SentenceEntry {
        hanzi: "这个项目具有挑战性",
        pinyin: "zhe4ge5xiang4mu4ju4you3tiao3zhan4xing4",
        meaning: "this project is challenging",
        hsk: 6,
    },
    SentenceEntry {
        hanzi: "我们应该珍惜每一天",
        pinyin: "wo3men5ying1gai1zhen1xi1mei3yi4tian1",
        meaning: "we should cherish every day",
        hsk: 6,
    },
];


#[cfg(test)]
mod tests;
