//! Vocabulary data — Hanzi with pinyin, meaning, and HSK level.
//!
//! Static arrays compiled into the binary. Kept simple for Phase 2;
//! a build.rs JSON pipeline can replace this later for scale.

#[derive(Clone, Copy, Debug)]
pub struct VocabEntry {
    pub hanzi: &'static str,
    pub pinyin: &'static str,
    pub meaning: &'static str,
    pub hsk: u8, // 1–6
    pub example: &'static str, // example sentence (empty if none)
}

/// Core vocabulary pool. Each entry: (hanzi, pinyin, meaning, hsk_level).
pub static VOCAB: &[VocabEntry] = &[
    // HSK 1 — basic
    VocabEntry { hanzi: "人", pinyin: "ren2", meaning: "person", hsk: 1, example: "这个人很好 — this person is good" },
    VocabEntry { hanzi: "大", pinyin: "da4", meaning: "big", hsk: 1, example: "大山 — big mountain" },
    VocabEntry { hanzi: "小", pinyin: "xiao3", meaning: "small", hsk: 1, example: "小手 — small hand" },
    VocabEntry { hanzi: "中", pinyin: "zhong1", meaning: "middle", hsk: 1, example: "中国 — China (middle country)" },
    VocabEntry { hanzi: "上", pinyin: "shang4", meaning: "up", hsk: 1, example: "上山 — go up the mountain" },
    VocabEntry { hanzi: "下", pinyin: "xia4", meaning: "down", hsk: 1, example: "下山 — go down the mountain" },
    VocabEntry { hanzi: "天", pinyin: "tian1", meaning: "sky/day", hsk: 1, example: "今天 — today" },
    VocabEntry { hanzi: "日", pinyin: "ri4", meaning: "sun/day", hsk: 1, example: "日月 — sun and moon" },
    VocabEntry { hanzi: "月", pinyin: "yue4", meaning: "moon", hsk: 1, example: "三月 — March (3rd moon)" },
    VocabEntry { hanzi: "水", pinyin: "shui3", meaning: "water", hsk: 1, example: "水火 — water and fire" },
    VocabEntry { hanzi: "火", pinyin: "huo3", meaning: "fire", hsk: 1, example: "大火 — big fire" },
    VocabEntry { hanzi: "山", pinyin: "shan1", meaning: "mountain", hsk: 1, example: "山上 — on the mountain" },
    VocabEntry { hanzi: "口", pinyin: "kou3", meaning: "mouth", hsk: 1, example: "人口 — population" },
    VocabEntry { hanzi: "手", pinyin: "shou3", meaning: "hand", hsk: 1, example: "左手 — left hand" },
    VocabEntry { hanzi: "目", pinyin: "mu4", meaning: "eye", hsk: 1, example: "目中 — in one's eyes" },
    VocabEntry { hanzi: "心", pinyin: "xin1", meaning: "heart", hsk: 1, example: "心中 — in one's heart" },
    VocabEntry { hanzi: "好", pinyin: "hao3", meaning: "good", hsk: 1, example: "你好 — hello" },
    VocabEntry { hanzi: "你", pinyin: "ni3", meaning: "you", hsk: 1, example: "你好 — hello (lit. you good)" },
    VocabEntry { hanzi: "我", pinyin: "wo3", meaning: "I/me", hsk: 1, example: "我是人 — I am a person" },
    VocabEntry { hanzi: "他", pinyin: "ta1", meaning: "he", hsk: 1, example: "他不好 — he's not good" },
    VocabEntry { hanzi: "她", pinyin: "ta1", meaning: "she", hsk: 1, example: "她很好 — she is great" },
    VocabEntry { hanzi: "不", pinyin: "bu4", meaning: "not", hsk: 1, example: "不好 — not good" },
    VocabEntry { hanzi: "是", pinyin: "shi4", meaning: "is/yes", hsk: 1, example: "我是人 — I am a person" },
    VocabEntry { hanzi: "一", pinyin: "yi1", meaning: "one", hsk: 1, example: "" },
    VocabEntry { hanzi: "二", pinyin: "er4", meaning: "two", hsk: 1, example: "" },
    VocabEntry { hanzi: "三", pinyin: "san1", meaning: "three", hsk: 1, example: "" },
    VocabEntry { hanzi: "四", pinyin: "si4", meaning: "four", hsk: 1, example: "" },
    VocabEntry { hanzi: "五", pinyin: "wu3", meaning: "five", hsk: 1, example: "" },
    VocabEntry { hanzi: "六", pinyin: "liu4", meaning: "six", hsk: 1, example: "" },
    VocabEntry { hanzi: "七", pinyin: "qi1", meaning: "seven", hsk: 1, example: "" },
    VocabEntry { hanzi: "八", pinyin: "ba1", meaning: "eight", hsk: 1, example: "" },
    VocabEntry { hanzi: "九", pinyin: "jiu3", meaning: "nine", hsk: 1, example: "" },
    VocabEntry { hanzi: "十", pinyin: "shi2", meaning: "ten", hsk: 1, example: "" },

    // HSK 2 — intermediate basics
    VocabEntry { hanzi: "花", pinyin: "hua1", meaning: "flower", hsk: 2, example: "" },
    VocabEntry { hanzi: "鱼", pinyin: "yu2", meaning: "fish", hsk: 2, example: "" },
    VocabEntry { hanzi: "鸟", pinyin: "niao3", meaning: "bird", hsk: 2, example: "" },
    VocabEntry { hanzi: "猫", pinyin: "mao1", meaning: "cat", hsk: 2, example: "" },
    VocabEntry { hanzi: "狗", pinyin: "gou3", meaning: "dog", hsk: 2, example: "" },
    VocabEntry { hanzi: "马", pinyin: "ma3", meaning: "horse", hsk: 2, example: "" },
    VocabEntry { hanzi: "牛", pinyin: "niu2", meaning: "cow", hsk: 2, example: "" },
    VocabEntry { hanzi: "虎", pinyin: "hu3", meaning: "tiger", hsk: 2, example: "" },
    VocabEntry { hanzi: "龙", pinyin: "long2", meaning: "dragon", hsk: 2, example: "" },
    VocabEntry { hanzi: "风", pinyin: "feng1", meaning: "wind", hsk: 2, example: "" },
    VocabEntry { hanzi: "雨", pinyin: "yu3", meaning: "rain", hsk: 2, example: "" },
    VocabEntry { hanzi: "雪", pinyin: "xue3", meaning: "snow", hsk: 2, example: "" },
    VocabEntry { hanzi: "石", pinyin: "shi2", meaning: "stone", hsk: 2, example: "" },
    VocabEntry { hanzi: "金", pinyin: "jin1", meaning: "gold", hsk: 2, example: "" },
    VocabEntry { hanzi: "木", pinyin: "mu4", meaning: "wood", hsk: 2, example: "" },
    VocabEntry { hanzi: "土", pinyin: "tu3", meaning: "earth", hsk: 2, example: "" },
    VocabEntry { hanzi: "门", pinyin: "men2", meaning: "door", hsk: 2, example: "" },
    VocabEntry { hanzi: "刀", pinyin: "dao1", meaning: "knife", hsk: 2, example: "" },
    VocabEntry { hanzi: "力", pinyin: "li4", meaning: "power", hsk: 2, example: "" },
    VocabEntry { hanzi: "王", pinyin: "wang2", meaning: "king", hsk: 2, example: "" },
    VocabEntry { hanzi: "女", pinyin: "nv3", meaning: "woman", hsk: 2, example: "" },
    VocabEntry { hanzi: "子", pinyin: "zi3", meaning: "child", hsk: 2, example: "" },
    VocabEntry { hanzi: "学", pinyin: "xue2", meaning: "study", hsk: 2, example: "" },
    VocabEntry { hanzi: "食", pinyin: "shi2", meaning: "food", hsk: 2, example: "" },
    VocabEntry { hanzi: "米", pinyin: "mi3", meaning: "rice", hsk: 2, example: "" },
    VocabEntry { hanzi: "竹", pinyin: "zhu2", meaning: "bamboo", hsk: 2, example: "" },
    VocabEntry { hanzi: "耳", pinyin: "er3", meaning: "ear", hsk: 2, example: "" },
    VocabEntry { hanzi: "足", pinyin: "zu2", meaning: "foot", hsk: 2, example: "" },

    // HSK 3 — tougher
    VocabEntry { hanzi: "电", pinyin: "dian4", meaning: "electric", hsk: 3, example: "" },
    VocabEntry { hanzi: "云", pinyin: "yun2", meaning: "cloud", hsk: 3, example: "" },
    VocabEntry { hanzi: "星", pinyin: "xing1", meaning: "star", hsk: 3, example: "" },
    VocabEntry { hanzi: "光", pinyin: "guang1", meaning: "light", hsk: 3, example: "" },
    VocabEntry { hanzi: "气", pinyin: "qi4", meaning: "air/qi", hsk: 3, example: "" },
    VocabEntry { hanzi: "血", pinyin: "xue4", meaning: "blood", hsk: 3, example: "" },
    VocabEntry { hanzi: "骨", pinyin: "gu3", meaning: "bone", hsk: 3, example: "" },
    VocabEntry { hanzi: "鬼", pinyin: "gui3", meaning: "ghost", hsk: 3, example: "" },
    VocabEntry { hanzi: "夜", pinyin: "ye4", meaning: "night", hsk: 3, example: "" },
    VocabEntry { hanzi: "剑", pinyin: "jian4", meaning: "sword", hsk: 3, example: "" },
    VocabEntry { hanzi: "盾", pinyin: "dun4", meaning: "shield", hsk: 3, example: "" },
    VocabEntry { hanzi: "毒", pinyin: "du2", meaning: "poison", hsk: 3, example: "" },
    VocabEntry { hanzi: "铁", pinyin: "tie3", meaning: "iron", hsk: 3, example: "" },
    VocabEntry { hanzi: "玉", pinyin: "yu4", meaning: "jade", hsk: 3, example: "" },
    VocabEntry { hanzi: "魔", pinyin: "mo2", meaning: "demon", hsk: 3, example: "" },
    VocabEntry { hanzi: "灵", pinyin: "ling2", meaning: "spirit", hsk: 3, example: "" },
    // Multi-character (elite) words — HSK 2+
    VocabEntry { hanzi: "朋友", pinyin: "peng2you3", meaning: "friend", hsk: 2, example: "" },
    VocabEntry { hanzi: "学生", pinyin: "xue2sheng1", meaning: "student", hsk: 2, example: "" },
    VocabEntry { hanzi: "老师", pinyin: "lao3shi1", meaning: "teacher", hsk: 2, example: "" },
    VocabEntry { hanzi: "你好", pinyin: "ni3hao3", meaning: "hello", hsk: 2, example: "" },
    VocabEntry { hanzi: "谢谢", pinyin: "xie4xie4", meaning: "thanks", hsk: 2, example: "" },
    VocabEntry { hanzi: "对不起", pinyin: "dui4bu4qi3", meaning: "sorry", hsk: 2, example: "" },
    VocabEntry { hanzi: "再见", pinyin: "zai4jian4", meaning: "goodbye", hsk: 2, example: "" },
    VocabEntry { hanzi: "中国", pinyin: "zhong1guo2", meaning: "China", hsk: 2, example: "" },
    VocabEntry { hanzi: "今天", pinyin: "jin1tian1", meaning: "today", hsk: 2, example: "" },
    VocabEntry { hanzi: "明天", pinyin: "ming2tian1", meaning: "tomorrow", hsk: 2, example: "" },
    VocabEntry { hanzi: "时间", pinyin: "shi2jian1", meaning: "time", hsk: 3, example: "" },
    VocabEntry { hanzi: "力量", pinyin: "li4liang4", meaning: "strength", hsk: 3, example: "" },
    VocabEntry { hanzi: "生命", pinyin: "sheng1ming4", meaning: "life", hsk: 3, example: "" },
    VocabEntry { hanzi: "黑暗", pinyin: "hei1an4", meaning: "darkness", hsk: 3, example: "" },
    VocabEntry { hanzi: "危险", pinyin: "wei1xian3", meaning: "danger", hsk: 3, example: "" },
    VocabEntry { hanzi: "战斗", pinyin: "zhan4dou4", meaning: "battle", hsk: 3, example: "" },
    VocabEntry { hanzi: "武器", pinyin: "wu3qi4", meaning: "weapon", hsk: 3, example: "" },
    VocabEntry { hanzi: "勇气", pinyin: "yong3qi4", meaning: "courage", hsk: 3, example: "" },
];

/// Get vocab entries for a given max HSK level.
pub fn vocab_for_floor(floor: i32) -> Vec<&'static VocabEntry> {
    let max_hsk = match floor {
        1..=5 => 1,
        6..=10 => 2,
        11..=15 => 3,
        _ => 4,
    };
    VOCAB.iter().filter(|v| v.hsk <= max_hsk).collect()
}

/// Check if `input` is a valid pinyin for the given hanzi.
/// Accepts concatenated ("peng2you3") or space-separated ("peng2 you3") input.
pub fn check_pinyin(entry: &VocabEntry, input: &str) -> bool {
    let normalized = input.replace(' ', "");
    entry.pinyin.eq_ignore_ascii_case(&normalized)
}

/// Returns true if this vocab entry is a multi-character word (elite).
pub fn is_elite(entry: &VocabEntry) -> bool {
    entry.hanzi.chars().count() > 1
}
