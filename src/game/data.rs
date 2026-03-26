//! Static data tables for challenges and mini-games.

/// Sentence data for sentence construction challenges.
/// Organised into three difficulty tiers so `select_sentence_for_floor` can
/// pick easier sentences on early floors and harder ones later.
///
/// Tier 1 — easy (2-3 words, HSK 1)
pub(super) const SENTENCES_EASY: &[(&[&str], &str)] = &[
    (&["我", "是", "学生"], "I am a student"),
    (&["你", "好", "吗"], "How are you?"),
    (&["我们", "去", "学校"], "We go to school"),
    (&["她", "很", "高兴"], "She is very happy"),
    (&["他", "喝", "水"], "He drinks water"),
    (&["我", "吃", "饭"], "I eat"),
    (&["你", "看", "书"], "You read books"),
    (&["他们", "是", "朋友"], "They are friends"),
    (&["我", "很", "好"], "I am fine"),
    (&["她", "有", "猫"], "She has a cat"),
];

/// Tier 2 — medium (3-4 words, HSK 1-2)
pub(super) const SENTENCES_MEDIUM: &[(&[&str], &str)] = &[
    (&["他", "不", "喝", "水"], "He doesn't drink water"),
    (&["我", "想", "吃", "饭"], "I want to eat"),
    (&["今天", "天气", "很", "好"], "Today's weather is good"),
    (&["你", "叫", "什么", "名字"], "What is your name?"),
    (&["他们", "在", "看", "书"], "They are reading books"),
    (&["我", "喜欢", "中国", "菜"], "I like Chinese food"),
    (&["她", "不", "想", "去"], "She doesn't want to go"),
    (&["我们", "明天", "去", "学校"], "We go to school tomorrow"),
    (&["他", "每天", "喝", "茶"], "He drinks tea every day"),
    (&["你", "在", "做", "什么"], "What are you doing?"),
];

/// Tier 3 — hard (4-5 words, HSK 2-3)
pub(super) const SENTENCES_HARD: &[(&[&str], &str)] = &[
    (
        &["我", "昨天", "买", "了", "书"],
        "I bought a book yesterday",
    ),
    (&["她", "每天", "早上", "跑步"], "She runs every morning"),
    (
        &["他们", "下午", "去", "公园", "玩"],
        "They go to the park to play in the afternoon",
    ),
    (&["你", "能", "帮", "我", "吗"], "Can you help me?"),
    (
        &["我", "不", "知道", "他", "在哪儿"],
        "I don't know where he is",
    ),
    (
        &["老师", "今天", "没有", "来", "学校"],
        "The teacher didn't come to school today",
    ),
    (&["我们", "一起", "去", "吃", "饭"], "Let's go eat together"),
    (&["他", "说", "他", "很", "忙"], "He says he is very busy"),
    (
        &["你", "想", "喝", "什么", "茶"],
        "What tea would you like?",
    ),
    (&["她", "的", "朋友", "很", "多"], "She has many friends"),
];

/// Select a sentence appropriate for the current floor.
/// Early floors (≤8) pick from easy, mid floors (9–18) from easy+medium,
/// late floors (19+) can draw from all three tiers.
pub(super) fn select_sentence_for_floor(floor: i32, rng_val: u64) -> (&'static [&'static str], &'static str) {
    if floor <= 8 {
        let idx = rng_val as usize % SENTENCES_EASY.len();
        SENTENCES_EASY[idx]
    } else if floor <= 18 {
        let pool_len = SENTENCES_EASY.len() + SENTENCES_MEDIUM.len();
        let idx = rng_val as usize % pool_len;
        if idx < SENTENCES_EASY.len() {
            SENTENCES_EASY[idx]
        } else {
            SENTENCES_MEDIUM[idx - SENTENCES_EASY.len()]
        }
    } else {
        let pool_len = SENTENCES_EASY.len() + SENTENCES_MEDIUM.len() + SENTENCES_HARD.len();
        let idx = rng_val as usize % pool_len;
        if idx < SENTENCES_EASY.len() {
            SENTENCES_EASY[idx]
        } else if idx < SENTENCES_EASY.len() + SENTENCES_MEDIUM.len() {
            SENTENCES_MEDIUM[idx - SENTENCES_EASY.len()]
        } else {
            SENTENCES_HARD[idx - SENTENCES_EASY.len() - SENTENCES_MEDIUM.len()]
        }
    }
}

pub(super) const STROKE_ORDER_DATA: &[(&str, &[&str], &str, &str)] = &[
    ("明", &["日", "月"], "ming2", "bright"),
    ("休", &["亻", "木"], "xiu1", "rest"),
    ("林", &["木", "木"], "lin2", "forest"),
    ("好", &["女", "子"], "hao3", "good"),
    ("安", &["宀", "女"], "an1", "peace"),
    ("信", &["亻", "言"], "xin4", "believe"),
    ("花", &["艹", "化"], "hua1", "flower"),
    ("想", &["相", "心"], "xiang3", "think"),
    ("吃", &["口", "乞"], "chi1", "eat"),
    ("喝", &["口", "曷"], "he1", "drink"),
];

pub(super) const COMPOUND_DATA: &[(&str, &[&str], &str, &str)] = &[
    ("学生", &["学", "生"], "xue2sheng1", "student"),
    ("老师", &["老", "师"], "lao3shi1", "teacher"),
    ("中国", &["中", "国"], "zhong1guo2", "China"),
    ("朋友", &["朋", "友"], "peng2you3", "friend"),
    ("电话", &["电", "话"], "dian4hua4", "phone"),
    ("天气", &["天", "气"], "tian1qi4", "weather"),
    ("大学", &["大", "学"], "da4xue2", "university"),
    ("飞机", &["飞", "机"], "fei1ji1", "airplane"),
    ("火车", &["火", "车"], "huo3che1", "train"),
    ("书包", &["书", "包"], "shu1bao1", "schoolbag"),
    ("东西", &["东", "西"], "dong1xi1", "thing"),
    ("工作", &["工", "作"], "gong1zuo4", "work"),
];

pub(super) const CLASSIFIER_DATA: &[(&str, &str, &str, &str)] = &[
    ("书", "本", "shu1", "book"),
    ("人", "个", "ren2", "person"),
    ("猫", "只", "mao1", "cat"),
    ("狗", "只", "gou3", "dog"),
    ("车", "辆", "che1", "car"),
    ("花", "朵", "hua1", "flower"),
    ("纸", "张", "zhi3", "paper"),
    ("刀", "把", "dao1", "knife"),
    ("鱼", "条", "yu2", "fish"),
    ("笔", "支", "bi3", "pen"),
    ("衣服", "件", "yi1fu2", "clothes"),
    ("马", "匹", "ma3", "horse"),
    ("河", "条", "he2", "river"),
    ("山", "座", "shan1", "mountain"),
    ("树", "棵", "shu4", "tree"),
];

pub(super) const ALL_CLASSIFIERS: &[&str] = &[
    "个", "本", "只", "辆", "朵", "张", "把", "条", "支", "件", "匹", "座", "棵",
];

/// (hanzi, component_count, pinyin, meaning)
pub(super) const INK_WELL_DATA: &[(&str, u8, &str, &str)] = &[
    ("明", 2, "ming2", "bright"),
    ("休", 2, "xiu1", "rest"),
    ("好", 2, "hao3", "good"),
    ("安", 2, "an1", "peace"),
    ("林", 2, "lin2", "forest"),
    ("想", 2, "xiang3", "think"),
    ("花", 2, "hua1", "flower"),
    ("吃", 2, "chi1", "eat"),
    ("喝", 2, "he1", "drink"),
    ("信", 2, "xin4", "believe"),
    ("忘", 2, "wang4", "forget"),
    ("看", 2, "kan4", "look"),
];

/// (first_half, second_half, full, pinyin, meaning)
pub(super) const CHENGYU_DATA: &[(&str, &str, &str, &str, &str)] = &[
    (
        "\u{4e00}\u{5fc3}",
        "\u{4e00}\u{610f}",
        "\u{4e00}\u{5fc3}\u{4e00}\u{610f}",
        "yi1xin1yi1yi4",
        "wholeheartedly",
    ),
    (
        "\u{534a}\u{9014}",
        "\u{800c}\u{5e9f}",
        "\u{534a}\u{9014}\u{800c}\u{5e9f}",
        "ban4tu2er2fei4",
        "give up halfway",
    ),
    (
        "\u{81ea}\u{8a00}",
        "\u{81ea}\u{8bed}",
        "\u{81ea}\u{8a00}\u{81ea}\u{8bed}",
        "zi4yan2zi4yu3",
        "talk to oneself",
    ),
    (
        "\u{5165}\u{4e61}",
        "\u{968f}\u{4fd7}",
        "\u{5165}\u{4e61}\u{968f}\u{4fd7}",
        "ru4xiang1sui2su2",
        "when in Rome",
    ),
    (
        "\u{9a6c}\u{5230}",
        "\u{6210}\u{529f}",
        "\u{9a6c}\u{5230}\u{6210}\u{529f}",
        "ma3dao4cheng2gong1",
        "instant success",
    ),
    (
        "\u{5fc3}\u{60f3}",
        "\u{4e8b}\u{6210}",
        "\u{5fc3}\u{60f3}\u{4e8b}\u{6210}",
        "xin1xiang3shi4cheng2",
        "wishes come true",
    ),
    (
        "\u{5927}\u{540c}",
        "\u{5c0f}\u{5f02}",
        "\u{5927}\u{540c}\u{5c0f}\u{5f02}",
        "da4tong2xiao3yi4",
        "mostly the same",
    ),
    (
        "\u{767e}\u{53d1}",
        "\u{767e}\u{4e2d}",
        "\u{767e}\u{53d1}\u{767e}\u{4e2d}",
        "bai3fa1bai3zhong4",
        "hit every target",
    ),
    (
        "\u{5343}\u{65b9}",
        "\u{767e}\u{8ba1}",
        "\u{5343}\u{65b9}\u{767e}\u{8ba1}",
        "qian1fang1bai3ji4",
        "by every means",
    ),
    (
        "\u{5f00}\u{95e8}",
        "\u{89c1}\u{5c71}",
        "\u{5f00}\u{95e8}\u{89c1}\u{5c71}",
        "kai1men2jian4shan1",
        "get to the point",
    ),
    (
        "\u{4e07}\u{4e8b}",
        "\u{5982}\u{610f}",
        "\u{4e07}\u{4e8b}\u{5982}\u{610f}",
        "wan4shi4ru2yi4",
        "everything goes well",
    ),
    (
        "\u{5929}\u{4e0b}",
        "\u{592a}\u{5e73}",
        "\u{5929}\u{4e0b}\u{592a}\u{5e73}",
        "tian1xia4tai4ping2",
        "peace under heaven",
    ),
    (
        "\u{5927}\u{5f00}",
        "\u{773c}\u{754c}",
        "\u{5927}\u{5f00}\u{773c}\u{754c}",
        "da4kai1yan3jie4",
        "eye-opening",
    ),
    (
        "\u{4e03}\u{4e0a}",
        "\u{516b}\u{4e0b}",
        "\u{4e03}\u{4e0a}\u{516b}\u{4e0b}",
        "qi1shang4ba1xia4",
        "at sixes and sevens",
    ),
    (
        "\u{4e94}\u{5149}",
        "\u{5341}\u{8272}",
        "\u{4e94}\u{5149}\u{5341}\u{8272}",
        "wu3guang1shi2se4",
        "dazzling",
    ),
    (
        "\u{4e5d}\u{6b7b}",
        "\u{4e00}\u{751f}",
        "\u{4e5d}\u{6b7b}\u{4e00}\u{751f}",
        "jiu3si3yi1sheng1",
        "narrow escape",
    ),
    (
        "\u{4e00}\u{5200}",
        "\u{4e24}\u{65ad}",
        "\u{4e00}\u{5200}\u{4e24}\u{65ad}",
        "yi1dao1liang3duan4",
        "cut cleanly",
    ),
    (
        "\u{4e00}\u{76ee}",
        "\u{4e86}\u{7136}",
        "\u{4e00}\u{76ee}\u{4e86}\u{7136}",
        "yi1mu4liao3ran2",
        "crystal clear",
    ),
    (
        "\u{4e0d}\u{53ef}",
        "\u{601d}\u{8bae}",
        "\u{4e0d}\u{53ef}\u{601d}\u{8bae}",
        "bu4ke3si1yi4",
        "incredible",
    ),
    (
        "\u{6cf0}\u{7136}",
        "\u{81ea}\u{82e5}",
        "\u{6cf0}\u{7136}\u{81ea}\u{82e5}",
        "tai4ran2zi4ruo4",
        "calm and composed",
    ),
    (
        "\u{5b66}\u{4ee5}",
        "\u{81f4}\u{7528}",
        "\u{5b66}\u{4ee5}\u{81f4}\u{7528}",
        "xue2yi3zhi4yong4",
        "learn to apply",
    ),
    (
        "\u{5927}\u{5668}",
        "\u{665a}\u{6210}",
        "\u{5927}\u{5668}\u{665a}\u{6210}",
        "da4qi4wan3cheng2",
        "great minds mature slowly",
    ),
    (
        "\u{53e3}\u{662f}",
        "\u{5fc3}\u{975e}",
        "\u{53e3}\u{662f}\u{5fc3}\u{975e}",
        "kou3shi4xin1fei1",
        "say one thing mean another",
    ),
    (
        "\u{9f99}\u{98de}",
        "\u{51e4}\u{821e}",
        "\u{9f99}\u{98de}\u{51e4}\u{821e}",
        "long2fei1feng4wu3",
        "dragons fly phoenixes dance",
    ),
    (
        "\u{864e}\u{5934}",
        "\u{86c7}\u{5c3e}",
        "\u{864e}\u{5934}\u{86c7}\u{5c3e}",
        "hu3tou2she2wei3",
        "strong start weak end",
    ),
    (
        "\u{6c34}\u{6ef4}",
        "\u{77f3}\u{7a7f}",
        "\u{6c34}\u{6ef4}\u{77f3}\u{7a7f}",
        "shui3di1shi2chuan1",
        "water wears stone",
    ),
    (
        "\u{98ce}\u{548c}",
        "\u{65e5}\u{4e3d}",
        "\u{98ce}\u{548c}\u{65e5}\u{4e3d}",
        "feng1he2ri4li4",
        "gentle breeze sunny day",
    ),
    (
        "\u{91d1}\u{7389}",
        "\u{6ee1}\u{5802}",
        "\u{91d1}\u{7389}\u{6ee1}\u{5802}",
        "jin1yu4man3tang2",
        "riches fill the hall",
    ),
    (
        "\u{5929}\u{957f}",
        "\u{5730}\u{4e45}",
        "\u{5929}\u{957f}\u{5730}\u{4e45}",
        "tian1chang2di4jiu3",
        "everlasting",
    ),
    (
        "\u{5fc3}\u{5982}",
        "\u{6b62}\u{6c34}",
        "\u{5fc3}\u{5982}\u{6b62}\u{6c34}",
        "xin1ru2zhi3shui3",
        "mind still as water",
    ),
    (
        "\u{5149}\u{660e}",
        "\u{78ca}\u{843d}",
        "\u{5149}\u{660e}\u{78ca}\u{843d}",
        "guang1ming2lei3luo4",
        "open and upright",
    ),
    (
        "\u{4e00}\u{8def}",
        "\u{5e73}\u{5b89}",
        "\u{4e00}\u{8def}\u{5e73}\u{5b89}",
        "yi1lu4ping2an1",
        "safe journey",
    ),
];

/// (hanzi, pinyin, meaning, radical, wrong1, wrong2, wrong3)
pub(super) const RADICAL_GARDEN_DATA: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
    ("妈", "ma1", "mother", "女", "马", "口", "木"),
    ("河", "he2", "river", "氵", "口", "可", "亻"),
    ("打", "da3", "hit", "扌", "丁", "口", "大"),
    ("说", "shuo1", "speak", "讠", "兑", "口", "言"),
    ("吗", "ma0", "question particle", "口", "马", "女", "木"),
    ("他", "ta1", "he/him", "亻", "也", "口", "土"),
    ("跑", "pao3", "run", "足", "包", "口", "走"),
    ("猫", "mao1", "cat", "犭", "苗", "口", "豸"),
    ("认", "ren4", "recognize", "讠", "人", "口", "亻"),
    ("饭", "fan4", "rice/meal", "饣", "反", "口", "食"),
];

/// (hanzi, pinyin, meaning) — used by MirrorPool (pinyin typing)
pub(super) const MIRROR_POOL_DATA: &[(&str, &str, &str)] = &[
    ("你好", "ni3hao3", "hello"),
    ("谢谢", "xie4xie4", "thank you"),
    ("再见", "zai4jian4", "goodbye"),
    ("学生", "xue2sheng1", "student"),
    ("老师", "lao3shi1", "teacher"),
    ("中国", "zhong1guo2", "China"),
    ("朋友", "peng2you3", "friend"),
    ("电话", "dian4hua4", "phone"),
    ("天气", "tian1qi4", "weather"),
    ("工作", "gong1zuo4", "work"),
    ("大学", "da4xue2", "university"),
    ("飞机", "fei1ji1", "airplane"),
];
