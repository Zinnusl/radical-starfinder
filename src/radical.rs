//! Radical data, recipe system, and forge logic.

/// A Chinese radical with its unicode representation and meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Radical {
    pub ch: &'static str,
    pub name: &'static str,
    pub meaning: &'static str,
}

/// A recipe: combining radicals produces a character with an effect.
#[derive(Clone, Copy, Debug)]
pub struct Recipe {
    pub inputs: &'static [&'static str], // radical chars
    pub output_hanzi: &'static str,
    pub output_pinyin: &'static str,
    pub output_meaning: &'static str,
    pub effect: SpellEffect,
}

/// What a forged character does when used as a spell in combat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellEffect {
    /// Deal damage to all visible enemies
    FireAoe(i32),
    /// Heal the player
    Heal(i32),
    /// Block the next incoming hit
    Shield,
    /// Deal extra damage to current target
    StrongHit(i32),
}

impl SpellEffect {
    pub fn label(&self) -> &'static str {
        match self {
            SpellEffect::FireAoe(_) => "🔥 Fire",
            SpellEffect::Heal(_) => "💚 Heal",
            SpellEffect::Shield => "🛡 Shield",
            SpellEffect::StrongHit(_) => "⚔ Strike",
        }
    }
}

/// A forged spell the player can use in combat.
#[derive(Clone, Debug)]
pub struct Spell {
    pub hanzi: &'static str,
    pub pinyin: &'static str,
    pub meaning: &'static str,
    pub effect: SpellEffect,
}

// ── Radical catalog ─────────────────────────────────────────────────────────

pub const RADICALS: &[Radical] = &[
    Radical { ch: "火", name: "huǒ", meaning: "fire" },
    Radical { ch: "水", name: "shuǐ", meaning: "water" },
    Radical { ch: "木", name: "mù", meaning: "wood" },
    Radical { ch: "金", name: "jīn", meaning: "metal/gold" },
    Radical { ch: "土", name: "tǔ", meaning: "earth" },
    Radical { ch: "日", name: "rì", meaning: "sun/day" },
    Radical { ch: "月", name: "yuè", meaning: "moon/month" },
    Radical { ch: "心", name: "xīn", meaning: "heart" },
    Radical { ch: "口", name: "kǒu", meaning: "mouth" },
    Radical { ch: "手", name: "shǒu", meaning: "hand" },
    Radical { ch: "目", name: "mù", meaning: "eye" },
    Radical { ch: "人", name: "rén", meaning: "person" },
    Radical { ch: "大", name: "dà", meaning: "big" },
    Radical { ch: "小", name: "xiǎo", meaning: "small" },
    Radical { ch: "山", name: "shān", meaning: "mountain" },
    Radical { ch: "石", name: "shí", meaning: "stone" },
    Radical { ch: "雨", name: "yǔ", meaning: "rain" },
    Radical { ch: "风", name: "fēng", meaning: "wind" },
    Radical { ch: "刀", name: "dāo", meaning: "knife" },
    Radical { ch: "力", name: "lì", meaning: "power" },
    Radical { ch: "田", name: "tián", meaning: "field" },
    Radical { ch: "女", name: "nǚ", meaning: "woman" },
    Radical { ch: "子", name: "zǐ", meaning: "child" },
    Radical { ch: "王", name: "wáng", meaning: "king" },
    Radical { ch: "竹", name: "zhú", meaning: "bamboo" },
    Radical { ch: "米", name: "mǐ", meaning: "rice" },
    Radical { ch: "虫", name: "chóng", meaning: "insect" },
    Radical { ch: "贝", name: "bèi", meaning: "shell/treasure" },
    Radical { ch: "马", name: "mǎ", meaning: "horse" },
    Radical { ch: "鸟", name: "niǎo", meaning: "bird" },
];

// ── Recipes ─────────────────────────────────────────────────────────────────

pub const RECIPES: &[Recipe] = &[
    // Fire combos
    Recipe { inputs: &["火", "火"], output_hanzi: "炎", output_pinyin: "yán", output_meaning: "flame/blaze", effect: SpellEffect::FireAoe(3) },
    Recipe { inputs: &["火", "山"], output_hanzi: "灾", output_pinyin: "zāi", output_meaning: "disaster", effect: SpellEffect::FireAoe(4) },
    Recipe { inputs: &["火", "木"], output_hanzi: "烧", output_pinyin: "shāo", output_meaning: "to burn", effect: SpellEffect::FireAoe(2) },
    // Healing combos
    Recipe { inputs: &["心", "人"], output_hanzi: "仁", output_pinyin: "rén", output_meaning: "benevolence", effect: SpellEffect::Heal(4) },
    Recipe { inputs: &["水", "心"], output_hanzi: "沁", output_pinyin: "qìn", output_meaning: "to seep/refresh", effect: SpellEffect::Heal(3) },
    Recipe { inputs: &["日", "月"], output_hanzi: "明", output_pinyin: "míng", output_meaning: "bright/clear", effect: SpellEffect::Heal(5) },
    Recipe { inputs: &["木", "子"], output_hanzi: "李", output_pinyin: "lǐ", output_meaning: "plum tree", effect: SpellEffect::Heal(2) },
    // Shield combos
    Recipe { inputs: &["金", "土"], output_hanzi: "坚", output_pinyin: "jiān", output_meaning: "solid/firm", effect: SpellEffect::Shield },
    Recipe { inputs: &["石", "山"], output_hanzi: "岩", output_pinyin: "yán", output_meaning: "rock/cliff", effect: SpellEffect::Shield },
    Recipe { inputs: &["王", "金"], output_hanzi: "玉", output_pinyin: "yù", output_meaning: "jade", effect: SpellEffect::Shield },
    // Strong hit combos
    Recipe { inputs: &["刀", "力"], output_hanzi: "刃", output_pinyin: "rèn", output_meaning: "blade edge", effect: SpellEffect::StrongHit(5) },
    Recipe { inputs: &["手", "力"], output_hanzi: "拳", output_pinyin: "quán", output_meaning: "fist", effect: SpellEffect::StrongHit(4) },
    Recipe { inputs: &["大", "力"], output_hanzi: "奋", output_pinyin: "fèn", output_meaning: "exert effort", effect: SpellEffect::StrongHit(3) },
    Recipe { inputs: &["金", "刀"], output_hanzi: "剑", output_pinyin: "jiàn", output_meaning: "sword", effect: SpellEffect::StrongHit(6) },
    Recipe { inputs: &["风", "刀"], output_hanzi: "刮", output_pinyin: "guā", output_meaning: "to scrape/gust", effect: SpellEffect::FireAoe(2) },
    // Nature combos
    Recipe { inputs: &["水", "木"], output_hanzi: "沐", output_pinyin: "mù", output_meaning: "to bathe", effect: SpellEffect::Heal(3) },
    Recipe { inputs: &["雨", "田"], output_hanzi: "雷", output_pinyin: "léi", output_meaning: "thunder", effect: SpellEffect::FireAoe(5) },
    Recipe { inputs: &["水", "土"], output_hanzi: "泥", output_pinyin: "ní", output_meaning: "mud", effect: SpellEffect::Shield },
    // Misc
    Recipe { inputs: &["女", "子"], output_hanzi: "好", output_pinyin: "hǎo", output_meaning: "good", effect: SpellEffect::Heal(6) },
    Recipe { inputs: &["口", "大"], output_hanzi: "呗", output_pinyin: "bài", output_meaning: "to chant", effect: SpellEffect::StrongHit(3) },
    Recipe { inputs: &["竹", "马"], output_hanzi: "笃", output_pinyin: "dǔ", output_meaning: "sincere/earnest", effect: SpellEffect::Shield },
    Recipe { inputs: &["米", "口"], output_hanzi: "粮", output_pinyin: "liáng", output_meaning: "grain/provisions", effect: SpellEffect::Heal(4) },
    Recipe { inputs: &["目", "心"], output_hanzi: "想", output_pinyin: "xiǎng", output_meaning: "to think/miss", effect: SpellEffect::StrongHit(4) },
    Recipe { inputs: &["虫", "火"], output_hanzi: "烛", output_pinyin: "zhú", output_meaning: "candle", effect: SpellEffect::FireAoe(3) },
    Recipe { inputs: &["鸟", "山"], output_hanzi: "岛", output_pinyin: "dǎo", output_meaning: "island", effect: SpellEffect::Shield },
];

/// Try to forge a character from a set of radicals. Order-independent.
pub fn try_forge(radicals: &[&str]) -> Option<&'static Recipe> {
    for recipe in RECIPES {
        if recipe.inputs.len() == radicals.len() {
            // Check if all inputs are present (order-independent)
            let mut matched = vec![false; recipe.inputs.len()];
            let mut all_found = true;
            for rad in radicals {
                let mut found = false;
                for (i, inp) in recipe.inputs.iter().enumerate() {
                    if !matched[i] && inp == rad {
                        matched[i] = true;
                        found = true;
                        break;
                    }
                }
                if !found {
                    all_found = false;
                    break;
                }
            }
            if all_found && matched.iter().all(|m| *m) {
                return Some(recipe);
            }
        }
    }
    None
}

/// Get a subset of radicals available for a given floor.
/// Earlier floors have fewer radicals.
pub fn radicals_for_floor(floor: i32) -> &'static [Radical] {
    let count = match floor {
        1 => 10,
        2 => 15,
        3 => 20,
        4 => 25,
        _ => RADICALS.len(),
    };
    &RADICALS[..count.min(RADICALS.len())]
}
