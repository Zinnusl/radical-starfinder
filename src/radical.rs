//! Radical data, recipe system, and forge logic.

/// A Chinese radical with its unicode representation and meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Radical {
    pub ch: &'static str,
    pub name: &'static str,
    pub meaning: &'static str,
    pub rare: bool,
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
    /// Reveal the current floor layout
    Reveal,
    /// Block the next incoming hit
    Shield,
    /// Deal extra damage to current target
    StrongHit(i32),
    /// Damage target and heal player (vampiric)
    Drain(i32),
    /// Stun current enemy (skips their next attack)
    Stun,
    /// Convince a foe to stand down
    Pacify,
    /// Apply Slow status for N turns
    Slow(i32),
    /// Swap positions with target enemy
    Teleport,
    /// Apply poison damage over time
    Poison(i32, i32),
    /// Restore mental focus
    FocusRestore(i32),
    /// Strip enemy radical armor
    ArmorBreak,
}

impl SpellEffect {
    pub fn label(&self) -> &'static str {
        match self {
            SpellEffect::FireAoe(_) => "🔥 Fire",
            SpellEffect::Heal(_) => "💚 Heal",
            SpellEffect::Reveal => "👁 Reveal",
            SpellEffect::Shield => "🛡 Shield",
            SpellEffect::StrongHit(_) => "⚔ Strike",
            SpellEffect::Drain(_) => "🩸 Drain",
            SpellEffect::Stun => "⚡ Stun",
            SpellEffect::Pacify => "☯ Pacify",
            SpellEffect::Slow(_) => "❄ Slow",
            SpellEffect::Teleport => "🌀 Swap",
            SpellEffect::Poison(_, _) => "☠ Poison",
            SpellEffect::FocusRestore(_) => "🧘 Focus",
            SpellEffect::ArmorBreak => "💥 Shatter",
        }
    }

    pub fn description(&self) -> String {
        match self {
            SpellEffect::FireAoe(dmg) => format!("Deals {} damage to all visible enemies.", dmg),
            SpellEffect::Heal(amt) => format!("Restores {} HP instantly.", amt),
            SpellEffect::Reveal => "Reveals the entire floor map.".to_string(),
            SpellEffect::Shield => "Blocks the next incoming hit this combat.".to_string(),
            SpellEffect::StrongHit(dmg) => format!("Deals {} bonus damage to current target.", dmg),
            SpellEffect::Drain(dmg) => {
                format!("Deals {} damage and heals you for the same amount.", dmg)
            }
            SpellEffect::Stun => "Stuns the current enemy, skipping their next attack.".to_string(),
            SpellEffect::Pacify => {
                "Convinces a foe to stand down, ending the fight peacefully.".to_string()
            }
            SpellEffect::Slow(turns) => {
                format!("Slows target for {} turns, reducing their movement.", turns)
            }
            SpellEffect::Teleport => "Swap positions with target enemy.".to_string(),
            SpellEffect::Poison(dmg, turns) => {
                format!(
                    "Poisons target for {} damage/turn over {} turns.",
                    dmg, turns
                )
            }
            SpellEffect::FocusRestore(amt) => format!("Restores {} mental focus.", amt),
            SpellEffect::ArmorBreak => {
                "Destroys target's radical armor, leaving them vulnerable.".to_string()
            }
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
    Radical {
        ch: "火",
        name: "huǒ",
        meaning: "fire",
        rare: false,
    },
    Radical {
        ch: "水",
        name: "shuǐ",
        meaning: "water",
        rare: false,
    },
    Radical {
        ch: "木",
        name: "mù",
        meaning: "wood",
        rare: false,
    },
    Radical {
        ch: "金",
        name: "jīn",
        meaning: "metal/gold",
        rare: false,
    },
    Radical {
        ch: "土",
        name: "tǔ",
        meaning: "earth",
        rare: false,
    },
    Radical {
        ch: "日",
        name: "rì",
        meaning: "sun/day",
        rare: false,
    },
    Radical {
        ch: "月",
        name: "yuè",
        meaning: "moon/month",
        rare: false,
    },
    Radical {
        ch: "心",
        name: "xīn",
        meaning: "heart",
        rare: false,
    },
    Radical {
        ch: "口",
        name: "kǒu",
        meaning: "mouth",
        rare: false,
    },
    Radical {
        ch: "手",
        name: "shǒu",
        meaning: "hand",
        rare: false,
    },
    Radical {
        ch: "目",
        name: "mù",
        meaning: "eye",
        rare: false,
    },
    Radical {
        ch: "人",
        name: "rén",
        meaning: "person",
        rare: false,
    },
    Radical {
        ch: "大",
        name: "dà",
        meaning: "big",
        rare: false,
    },
    Radical {
        ch: "小",
        name: "xiǎo",
        meaning: "small",
        rare: false,
    },
    Radical {
        ch: "山",
        name: "shān",
        meaning: "mountain",
        rare: false,
    },
    Radical {
        ch: "石",
        name: "shí",
        meaning: "stone",
        rare: false,
    },
    Radical {
        ch: "雨",
        name: "yǔ",
        meaning: "rain",
        rare: false,
    },
    Radical {
        ch: "风",
        name: "fēng",
        meaning: "wind",
        rare: false,
    },
    Radical {
        ch: "刀",
        name: "dāo",
        meaning: "knife",
        rare: false,
    },
    Radical {
        ch: "力",
        name: "lì",
        meaning: "power",
        rare: false,
    },
    Radical {
        ch: "田",
        name: "tián",
        meaning: "field",
        rare: false,
    },
    Radical {
        ch: "女",
        name: "nǚ",
        meaning: "woman",
        rare: false,
    },
    Radical {
        ch: "子",
        name: "zǐ",
        meaning: "child",
        rare: false,
    },
    Radical {
        ch: "王",
        name: "wáng",
        meaning: "king",
        rare: false,
    },
    Radical {
        ch: "竹",
        name: "zhú",
        meaning: "bamboo",
        rare: false,
    },
    Radical {
        ch: "米",
        name: "mǐ",
        meaning: "rice",
        rare: false,
    },
    Radical {
        ch: "虫",
        name: "chóng",
        meaning: "insect",
        rare: false,
    },
    Radical {
        ch: "贝",
        name: "bèi",
        meaning: "shell/treasure",
        rare: false,
    },
    Radical {
        ch: "马",
        name: "mǎ",
        meaning: "horse",
        rare: false,
    },
    Radical {
        ch: "鸟",
        name: "niǎo",
        meaning: "bird",
        rare: false,
    },
    // ── Rare radicals (boss drops) ──────────────────────────────────────────
    Radical {
        ch: "龙",
        name: "lóng",
        meaning: "dragon",
        rare: true,
    },
    Radical {
        ch: "鬼",
        name: "guǐ",
        meaning: "ghost",
        rare: true,
    },
    Radical {
        ch: "玉",
        name: "yù",
        meaning: "jade",
        rare: true,
    },
    Radical {
        ch: "雷",
        name: "léi",
        meaning: "thunder",
        rare: true,
    },
    Radical {
        ch: "凤",
        name: "fèng",
        meaning: "phoenix",
        rare: true,
    },
];

// ── Recipes (verified via hanzicraft.com decomposition) ─────────────────────
// All recipes verified: inputs match actual Level 1 components (recursively
// decomposed to game radicals). Generated by tools/verify_recipes.py.

pub const RECIPES: &[Recipe] = &[
    // ── Fire / AoE ──────────────────────────────────────────────────────────
    Recipe {
        inputs: &["日", "月"],
        output_hanzi: "明",
        output_pinyin: "míng",
        output_meaning: "bright/clear",
        effect: SpellEffect::Reveal,
    },
    Recipe {
        inputs: &["雨", "田"],
        output_hanzi: "雷",
        output_pinyin: "léi",
        output_meaning: "thunder",
        effect: SpellEffect::FireAoe(5),
    },
    Recipe {
        inputs: &["火", "虫"],
        output_hanzi: "烛",
        output_pinyin: "zhú",
        output_meaning: "candle/flame",
        effect: SpellEffect::FireAoe(3),
    },
    Recipe {
        inputs: &["口", "鸟"],
        output_hanzi: "鸣",
        output_pinyin: "míng",
        output_meaning: "cry of a bird",
        effect: SpellEffect::Slow(2),
    },
    Recipe {
        inputs: &["口", "日"],
        output_hanzi: "唱",
        output_pinyin: "chàng",
        output_meaning: "to sing",
        effect: SpellEffect::FireAoe(3),
    },
    Recipe {
        inputs: &["火", "山"],
        output_hanzi: "灿",
        output_pinyin: "càn",
        output_meaning: "glorious/radiant",
        effect: SpellEffect::FireAoe(3),
    },
    Recipe {
        inputs: &["日", "王"],
        output_hanzi: "旺",
        output_pinyin: "wàng",
        output_meaning: "prosperous",
        effect: SpellEffect::FireAoe(3),
    },
    Recipe {
        inputs: &["雨", "木", "目"],
        output_hanzi: "霜",
        output_pinyin: "shuāng",
        output_meaning: "frost",
        effect: SpellEffect::Slow(3),
    },
    Recipe {
        inputs: &["火", "土"],
        output_hanzi: "灶",
        output_pinyin: "zào",
        output_meaning: "stove/furnace",
        effect: SpellEffect::FireAoe(3),
    },
    Recipe {
        inputs: &["木", "火"],
        output_hanzi: "焚",
        output_pinyin: "fén",
        output_meaning: "to burn",
        effect: SpellEffect::FireAoe(4),
    },
    // ── Healing ─────────────────────────────────────────────────────────────
    Recipe {
        inputs: &["女", "子"],
        output_hanzi: "好",
        output_pinyin: "hǎo",
        output_meaning: "good/well",
        effect: SpellEffect::Heal(6),
    },
    Recipe {
        inputs: &["木", "口"],
        output_hanzi: "杏",
        output_pinyin: "xìng",
        output_meaning: "apricot",
        effect: SpellEffect::Heal(3),
    },
    Recipe {
        inputs: &["木", "子"],
        output_hanzi: "李",
        output_pinyin: "lǐ",
        output_meaning: "plum tree",
        effect: SpellEffect::Heal(2),
    },
    Recipe {
        inputs: &["木", "目", "心"],
        output_hanzi: "想",
        output_pinyin: "xiǎng",
        output_meaning: "to think/miss",
        effect: SpellEffect::Heal(4),
    },
    Recipe {
        inputs: &["田", "心"],
        output_hanzi: "思",
        output_pinyin: "sī",
        output_meaning: "to ponder",
        effect: SpellEffect::FocusRestore(3),
    },
    Recipe {
        inputs: &["田", "木"],
        output_hanzi: "果",
        output_pinyin: "guǒ",
        output_meaning: "fruit/result",
        effect: SpellEffect::Heal(3),
    },
    Recipe {
        inputs: &["木", "风"],
        output_hanzi: "枫",
        output_pinyin: "fēng",
        output_meaning: "maple tree",
        effect: SpellEffect::Teleport,
    },
    Recipe {
        inputs: &["力", "口", "木"],
        output_hanzi: "架",
        output_pinyin: "jià",
        output_meaning: "frame/support",
        effect: SpellEffect::Heal(4),
    },
    Recipe {
        inputs: &["木", "米", "女"],
        output_hanzi: "楼",
        output_pinyin: "lóu",
        output_meaning: "building/floor",
        effect: SpellEffect::Heal(5),
    },
    Recipe {
        inputs: &["竹", "木", "目"],
        output_hanzi: "箱",
        output_pinyin: "xiāng",
        output_meaning: "box/chest",
        effect: SpellEffect::Heal(4),
    },
    Recipe {
        inputs: &["山", "月"],
        output_hanzi: "崩",
        output_pinyin: "bēng",
        output_meaning: "to collapse",
        effect: SpellEffect::ArmorBreak,
    },
    Recipe {
        inputs: &["米", "大"],
        output_hanzi: "类",
        output_pinyin: "lèi",
        output_meaning: "kind/type",
        effect: SpellEffect::FocusRestore(2),
    },
    Recipe {
        inputs: &["月", "田", "心"],
        output_hanzi: "腮",
        output_pinyin: "sāi",
        output_meaning: "cheek",
        effect: SpellEffect::Heal(4),
    },
    // ── Shield / Defense ────────────────────────────────────────────────────
    Recipe {
        inputs: &["山", "石"],
        output_hanzi: "岩",
        output_pinyin: "yán",
        output_meaning: "rock/cliff",
        effect: SpellEffect::Shield,
    },
    Recipe {
        inputs: &["小", "土"],
        output_hanzi: "尘",
        output_pinyin: "chén",
        output_meaning: "dust",
        effect: SpellEffect::Shield,
    },
    Recipe {
        inputs: &["王", "田", "土"],
        output_hanzi: "理",
        output_pinyin: "lǐ",
        output_meaning: "logic/reason",
        effect: SpellEffect::Pacify,
    },
    Recipe {
        inputs: &["口", "王"],
        output_hanzi: "呈",
        output_pinyin: "chéng",
        output_meaning: "to present",
        effect: SpellEffect::Shield,
    },
    Recipe {
        inputs: &["土", "人"],
        output_hanzi: "坐",
        output_pinyin: "zuò",
        output_meaning: "to sit",
        effect: SpellEffect::Shield,
    },
    Recipe {
        inputs: &["山", "风"],
        output_hanzi: "岚",
        output_pinyin: "lán",
        output_meaning: "mountain mist",
        effect: SpellEffect::Teleport,
    },
    Recipe {
        inputs: &["人", "王"],
        output_hanzi: "全",
        output_pinyin: "quán",
        output_meaning: "whole/complete",
        effect: SpellEffect::Shield,
    },
    // ── Strike / Damage ─────────────────────────────────────────────────────
    Recipe {
        inputs: &["田", "力"],
        output_hanzi: "男",
        output_pinyin: "nán",
        output_meaning: "male/man",
        effect: SpellEffect::StrongHit(4),
    },
    Recipe {
        inputs: &["力", "口"],
        output_hanzi: "加",
        output_pinyin: "jiā",
        output_meaning: "to add/increase",
        effect: SpellEffect::StrongHit(3),
    },
    Recipe {
        inputs: &["手", "目"],
        output_hanzi: "看",
        output_pinyin: "kàn",
        output_meaning: "to look/watch",
        effect: SpellEffect::StrongHit(3),
    },
    Recipe {
        inputs: &["女", "口"],
        output_hanzi: "如",
        output_pinyin: "rú",
        output_meaning: "as if/like",
        effect: SpellEffect::Slow(2),
    },
    Recipe {
        inputs: &["刀", "口"],
        output_hanzi: "召",
        output_pinyin: "zhào",
        output_meaning: "to summon",
        effect: SpellEffect::StrongHit(3),
    },
    Recipe {
        inputs: &["力", "口", "贝"],
        output_hanzi: "贺",
        output_pinyin: "hè",
        output_meaning: "to congratulate",
        effect: SpellEffect::StrongHit(4),
    },
    // ── Drain (damage + heal) ───────────────────────────────────────────────
    Recipe {
        inputs: &["口", "贝"],
        output_hanzi: "呗",
        output_pinyin: "bài",
        output_meaning: "to chant",
        effect: SpellEffect::Poison(1, 4),
    },
    Recipe {
        inputs: &["虫", "马"],
        output_hanzi: "蚂",
        output_pinyin: "mǎ",
        output_meaning: "ant",
        effect: SpellEffect::Poison(2, 3),
    },
    // ── Stun (skip enemy turn) ──────────────────────────────────────────────
    Recipe {
        inputs: &["口", "马"],
        output_hanzi: "吗",
        output_pinyin: "ma",
        output_meaning: "question particle",
        effect: SpellEffect::Stun,
    },
    Recipe {
        inputs: &["石", "马"],
        output_hanzi: "码",
        output_pinyin: "mǎ",
        output_meaning: "number/code",
        effect: SpellEffect::Stun,
    },
    Recipe {
        inputs: &["女", "马"],
        output_hanzi: "妈",
        output_pinyin: "mā",
        output_meaning: "mother",
        effect: SpellEffect::Stun,
    },
    Recipe {
        inputs: &["竹", "马"],
        output_hanzi: "笃",
        output_pinyin: "dǔ",
        output_meaning: "sincere/earnest",
        effect: SpellEffect::Stun,
    },
    // ── Rare recipes (require boss-drop radicals) ───────────────────────────
    Recipe {
        inputs: &["龙", "火"],
        output_hanzi: "炎龙",
        output_pinyin: "yán lóng",
        output_meaning: "flame dragon",
        effect: SpellEffect::FireAoe(8),
    },
    Recipe {
        inputs: &["龙", "水"],
        output_hanzi: "泷",
        output_pinyin: "lóng",
        output_meaning: "waterfall",
        effect: SpellEffect::Heal(8),
    },
    Recipe {
        inputs: &["鬼", "火"],
        output_hanzi: "鬼火",
        output_pinyin: "guǐ huǒ",
        output_meaning: "will-o-wisp",
        effect: SpellEffect::Poison(3, 3),
    },
    Recipe {
        inputs: &["玉", "心"],
        output_hanzi: "瑰",
        output_pinyin: "guī",
        output_meaning: "precious gem",
        effect: SpellEffect::Shield,
    },
    Recipe {
        inputs: &["雷", "力"],
        output_hanzi: "雷击",
        output_pinyin: "léi jī",
        output_meaning: "lightning strike",
        effect: SpellEffect::StrongHit(8),
    },
    Recipe {
        inputs: &["凤", "火"],
        output_hanzi: "凤凰",
        output_pinyin: "fèng huáng",
        output_meaning: "phoenix",
        effect: SpellEffect::Heal(10),
    },
    Recipe {
        inputs: &["龙", "鬼"],
        output_hanzi: "魂",
        output_pinyin: "hún",
        output_meaning: "soul",
        effect: SpellEffect::Drain(8),
    },
    Recipe {
        inputs: &["玉", "龙"],
        output_hanzi: "珑",
        output_pinyin: "lóng",
        output_meaning: "exquisite",
        effect: SpellEffect::StrongHit(10),
    },
];

/// Try to forge a character from a set of radicals. Order-independent.
#[allow(dead_code)]
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

/// Find recipes that are a near-miss for the given radicals.
/// Returns hints like "You have 2/3 radicals for 明 (bright)".
#[allow(dead_code)]
pub fn near_miss_hints(radicals: &[&str]) -> Vec<String> {
    let mut hints = Vec::new();
    for recipe in RECIPES {
        let mut matched = 0;
        let mut missing: Vec<&str> = Vec::new();
        for inp in recipe.inputs.iter() {
            if radicals.contains(inp) {
                matched += 1;
            } else {
                missing.push(inp);
            }
        }
        if missing.len() == 1 && matched >= 1 {
            hints.push(format!(
                "Close! {}/{} for {} ({}) — need [{}]",
                matched,
                recipe.inputs.len(),
                recipe.output_hanzi,
                recipe.output_meaning,
                missing[0]
            ));
        }
    }
    hints
}

/// Return indices into RECIPES for all recipes craftable from the given radicals.
/// Handles duplicate radicals correctly (a recipe needing two 火 requires two 火 in hand).
pub fn craftable_recipes(player_radicals: &[&str]) -> Vec<usize> {
    let mut results = Vec::new();
    for (idx, recipe) in RECIPES.iter().enumerate() {
        let mut available: Vec<&str> = player_radicals.to_vec();
        let mut can_craft = true;
        for &needed in recipe.inputs {
            if let Some(pos) = available.iter().position(|&r| r == needed) {
                available.remove(pos);
            } else {
                can_craft = false;
                break;
            }
        }
        if can_craft {
            results.push(idx);
        }
    }
    results
}

/// Number of common (non-rare) radicals.
const COMMON_RADICAL_COUNT: usize = 32;

/// Get a subset of radicals available for a given floor.
/// Earlier floors have fewer radicals. Excludes rare radicals.
pub fn radicals_for_floor(floor: i32) -> &'static [Radical] {
    let count = match floor {
        1 => 10,
        2 => 15,
        3 => 20,
        4 => 25,
        _ => COMMON_RADICAL_COUNT,
    };
    &RADICALS[..count.min(COMMON_RADICAL_COUNT)]
}

/// Get the list of rare radicals (boss drops only).
pub fn rare_radicals() -> &'static [Radical] {
    &RADICALS[COMMON_RADICAL_COUNT..]
}

#[cfg(test)]
mod tests {
    use super::{near_miss_hints, try_forge, SpellEffect};

    #[test]
    fn utility_spell_labels_are_stable() {
        assert_eq!(SpellEffect::Reveal.label(), "👁 Reveal");
        assert_eq!(SpellEffect::Pacify.label(), "☯ Pacify");
    }

    #[test]
    fn verified_utility_recipes_map_to_new_effects() {
        assert!(matches!(
            try_forge(&["日", "月"]).map(|recipe| recipe.effect),
            Some(SpellEffect::Reveal)
        ));
        assert!(matches!(
            try_forge(&["王", "田", "土"]).map(|recipe| recipe.effect),
            Some(SpellEffect::Pacify)
        ));
    }

    #[test]
    fn near_miss_hints_finds_one_missing_radical() {
        let hints = near_miss_hints(&["女"]);
        assert!(hints.len() >= 1);
        let hint = hints.iter().find(|h| h.contains("好")).unwrap();
        assert!(hint.contains("子"));
    }

    #[test]
    fn near_miss_hints_empty_for_no_close_match() {
        let empty_hints = near_miss_hints(&[]);
        assert!(empty_hints.is_empty());
    }
}
