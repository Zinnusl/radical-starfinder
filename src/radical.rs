//! Radical data, recipe system, and quantum forge logic for space tech modules.

/// A Chinese radical with its unicode representation and meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Radical {
    pub ch: &'static str,
    pub name: &'static str,
    pub meaning: &'static str,
    pub rare: bool,
}

/// A recipe: combining radicals in the quantum forge produces a character with a tech effect.
#[derive(Clone, Copy, Debug)]
pub struct Recipe {
    pub inputs: &'static [&'static str], // radical chars
    pub output_hanzi: &'static str,
    pub output_pinyin: &'static str,
    pub output_meaning: &'static str,
    pub effect: SpellEffect,
}

/// What a synthesized character does when used as an ability in space combat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellEffect {
    /// Deal plasma damage to all visible enemies
    FireAoe(i32),
    /// Nano-repair the player's hull
    Heal(i32),
    /// Scan the current deck layout with sensors
    Reveal,
    /// Project an energy barrier blocking the next incoming hit
    Shield,
    /// Deal extra kinetic damage to current target
    StrongHit(i32),
    /// Siphon energy from target, damaging and repairing player
    Drain(i32),
    /// EMP current enemy (skips their next action)
    Stun,
    /// Override a hostile's systems, ending the fight
    Pacify,
    /// Apply Cryo Beam status for N turns
    Slow(i32),
    /// Phase shift — swap positions with target enemy
    Teleport,
    /// Apply corrosive damage over time
    Poison(i32, i32),
    /// Recalibrate targeting systems
    FocusRestore(i32),
    /// Breach enemy shield plating
    ArmorBreak,
    /// Boost in a straight line, damaging enemies in the path
    Dash(i32),
    /// Penetrating shot hitting all enemies in a straight line
    Pierce(i32),
    /// Tractor beam pulls target enemy toward the player
    PullToward,
    /// Repulsor blast — knock target back and deal damage
    KnockBack(i32),
    /// Deploy nanite cloud for N turns — counter-attack when hit
    Thorns(i32),
    /// Arc blast: 3-wide expanding cone in a direction
    Cone(i32),
    /// Project a force wall of obstacles (length tiles)
    Wall(i32),
    /// Create a 3×3 area of Lubricant tiles (flammable, slippery)
    OilSlick,
    /// Cryo field in a cross pattern; deal i32 damage + Slow 2
    FreezeGround(i32),
    /// Plasma ignition on target tile and adjacent tiles; apply Burn
    Ignite,
    /// Nanite growth in 3×3; upgrade existing growth to dense thicket
    PlantGrowth,
    /// Seismic charge in a large cross; Open→CrumblingFloor, CrumblingFloor→Pit; deal i32 damage
    Earthquake(i32),
    /// Purify field tiles that repair i32 HP/turn for 3 rounds
    Sanctify(i32),
    /// Coolant wave: 5×3 line, push units 2 tiles, deal i32 damage, create coolant tiles
    FloodWave(i32),
    /// Deploy a barrier on target empty tile
    SummonBoulder,
    /// Charge toward a target — move adjacent and deal damage scaling with distance traveled
    Charge(i32),
    /// Blink (teleport) to an empty tile — AoE damage at departure point
    Blink(i32),
}

impl SpellEffect {
    pub fn label(&self) -> &'static str {
        match self {
            SpellEffect::FireAoe(_) => "🔥 Plasma",
            SpellEffect::Heal(_) => "💚 Nano Repair",
            SpellEffect::Reveal => "👁 Sensor Scan",
            SpellEffect::Shield => "🛡 Energy Barrier",
            SpellEffect::StrongHit(_) => "⚔ Kinetic Strike",
            SpellEffect::Drain(_) => "🩸 Siphon",
            SpellEffect::Stun => "⚡ EMP",
            SpellEffect::Pacify => "☯ Override",
            SpellEffect::Slow(_) => "❄ Cryo Beam",
            SpellEffect::Teleport => "🌀 Phase Shift",
            SpellEffect::Poison(_, _) => "☠ Corrosion",
            SpellEffect::FocusRestore(_) => "🎯 Recalibrate",
            SpellEffect::ArmorBreak => "💥 Shield Breach",
            SpellEffect::Dash(_) => "🏃 Boost",
            SpellEffect::Pierce(_) => "🗡 Penetrator",
            SpellEffect::PullToward => "🧲 Tractor Beam",
            SpellEffect::KnockBack(_) => "💨 Repulsor",
            SpellEffect::Thorns(_) => "🌿 Nanite Cloud",
            SpellEffect::Cone(_) => "🔺 Arc Blast",
            SpellEffect::Wall(_) => "🧱 Force Wall",
            SpellEffect::OilSlick => "🛢 Lubricant",
            SpellEffect::FreezeGround(_) => "❄ Cryo Field",
            SpellEffect::Ignite => "🔥 Plasma Ignition",
            SpellEffect::PlantGrowth => "🌱 Nanite Growth",
            SpellEffect::Earthquake(_) => "💎 Seismic Charge",
            SpellEffect::Sanctify(_) => "✨ Purify Field",
            SpellEffect::FloodWave(_) => "🌊 Coolant Wave",
            SpellEffect::SummonBoulder => "🪨 Deploy Barrier",
            SpellEffect::Charge(_) => "🐎 Charge",
            SpellEffect::Blink(_) => "⚡ Blink",
        }
    }

    pub fn description(&self) -> String {
        match self {
            SpellEffect::FireAoe(dmg) => format!("Deals {} plasma damage to all visible enemies.", dmg),
            SpellEffect::Heal(amt) => format!("Nano-repairs {} hull HP instantly.", amt),
            SpellEffect::Reveal => "Scans the entire deck layout with sensors.".to_string(),
            SpellEffect::Shield => "Projects an energy barrier blocking the next incoming hit.".to_string(),
            SpellEffect::StrongHit(dmg) => format!("Deals {} bonus kinetic damage to current target.", dmg),
            SpellEffect::Drain(dmg) => {
                format!("Siphons {} energy from target, repairing you for the same amount.", dmg)
            }
            SpellEffect::Stun => "EMP disrupts the current enemy, skipping their next action.".to_string(),
            SpellEffect::Pacify => {
                "Overrides a hostile's systems, ending the fight peacefully.".to_string()
            }
            SpellEffect::Slow(turns) => {
                format!("Cryo beam slows target for {} turns, reducing their movement.", turns)
            }
            SpellEffect::Teleport => "Phase shift — swap positions with target enemy.".to_string(),
            SpellEffect::Poison(dmg, turns) => {
                format!(
                    "Corrodes target for {} damage/turn over {} turns.",
                    dmg, turns
                )
            }
            SpellEffect::FocusRestore(amt) => format!("Recalibrates targeting systems, restoring {} focus.", amt),
            SpellEffect::ArmorBreak => {
                "Breaches target's shield plating, leaving them vulnerable.".to_string()
            }
            SpellEffect::Dash(dmg) => {
                format!(
                    "Boost in a straight line, dealing {} damage to enemies in the path.",
                    dmg
                )
            }
            SpellEffect::Pierce(dmg) => {
                format!(
                    "Fire a penetrating shot that hits all enemies in a line for {} damage.",
                    dmg
                )
            }
            SpellEffect::PullToward => "Tractor beam pulls target enemy toward you by up to 3 tiles.".to_string(),
            SpellEffect::KnockBack(dmg) => {
                format!(
                    "Repulsor blast knocks target back 2 tiles and deals {} damage. Extra damage if they hit a wall.",
                    dmg
                )
            }
            SpellEffect::Thorns(turns) => {
                format!(
                    "Deploy nanite cloud for {} turns. Enemies that hit you take 2 counter-damage.",
                    turns
                )
            }
            SpellEffect::Cone(dmg) => {
                format!(
                    "Arc blast dealing {} damage to all enemies in a 3-wide cone.",
                    dmg
                )
            }
            SpellEffect::Wall(len) => {
                format!(
                    "Project a force wall of {} obstacle tiles perpendicular to your aim direction.",
                    len
                )
            }
            SpellEffect::OilSlick => {
                "Create a 3×3 lubricant slick. Flammable! Units slide 1 extra tile.".to_string()
            }
            SpellEffect::FreezeGround(dmg) => {
                format!(
                    "Cryo field freezes tiles in a cross pattern. {} damage + Slow 2 to units hit.",
                    dmg
                )
            }
            SpellEffect::Ignite => {
                "Plasma ignition on target area. Burns nanites and lubricant. Applies Burn for 3 turns."
                    .to_string()
            }
            SpellEffect::PlantGrowth => {
                "Nanite growth in a 3×3 area. Existing growth becomes dense thicket. Repair 1 on nanites."
                    .to_string()
            }
            SpellEffect::Earthquake(dmg) => {
                format!(
                    "Seismic charge detonates in a large cross. {} damage. Open→Crumbling, Crumbling→Pit.",
                    dmg
                )
            }
            SpellEffect::Sanctify(heal) => {
                format!(
                    "Create a purify field that repairs {} HP/turn for 3 rounds. Cleanses corrupted tiles.",
                    heal
                )
            }
            SpellEffect::FloodWave(dmg) => {
                format!(
                    "Send a 5×3 coolant wave. {} damage, pushes units 2 tiles, creates coolant tiles.",
                    dmg
                )
            }
            SpellEffect::SummonBoulder => {
                "Deploy a barrier on target tile. Blocks movement and line of sight.".to_string()
            }
            SpellEffect::Charge(dmg) => {
                format!(
                    "Charge toward target enemy, stopping adjacent. {} base damage + 50% bonus per tile traveled.",
                    dmg
                )
            }
            SpellEffect::Blink(dmg) => {
                format!(
                    "Teleport to an empty tile. {} damage AoE explosion at departure point.",
                    dmg
                )
            }
        }
    }
}

/// A synthesized ability the player can use in space combat.
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
    Radical {
        ch: "言",
        name: "yán",
        meaning: "speech",
        rare: false,
    },
    Radical {
        ch: "门",
        name: "mén",
        meaning: "door/gate",
        rare: false,
    },
    Radical {
        ch: "犬",
        name: "quǎn",
        meaning: "dog",
        rare: false,
    },
    Radical {
        ch: "禾",
        name: "hé",
        meaning: "grain",
        rare: false,
    },
    Radical {
        ch: "车",
        name: "chē",
        meaning: "vehicle",
        rare: false,
    },
    Radical {
        ch: "又",
        name: "yòu",
        meaning: "again/claw",
        rare: false,
    },
    Radical {
        ch: "白",
        name: "bái",
        meaning: "white",
        rare: false,
    },
    Radical {
        ch: "气",
        name: "qì",
        meaning: "air/energy",
        rare: false,
    },
    // ── Rare radicals (elite drops) ──────────────────────────────────────────
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
    // ── Rare recipes (require elite-drop radicals) ───────────────────────────
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
    Recipe {
        inputs: &["马", "力"],
        output_hanzi: "驰",
        output_pinyin: "chí",
        output_meaning: "gallop/charge",
        effect: SpellEffect::Dash(3),
    },
    Recipe {
        inputs: &["风", "刀"],
        output_hanzi: "刮",
        output_pinyin: "guā",
        output_meaning: "scrape/gust",
        effect: SpellEffect::Dash(2),
    },
    // --- Pierce recipes ---
    Recipe {
        inputs: &["刀", "金"],
        output_hanzi: "刺",
        output_pinyin: "cì",
        output_meaning: "to pierce",
        effect: SpellEffect::Pierce(3),
    },
    Recipe {
        inputs: &["竹", "刀"],
        output_hanzi: "箭",
        output_pinyin: "jiàn",
        output_meaning: "arrow",
        effect: SpellEffect::Pierce(2),
    },
    // --- PullToward recipes ---
    Recipe {
        inputs: &["手", "力"],
        output_hanzi: "拉",
        output_pinyin: "lā",
        output_meaning: "to pull",
        effect: SpellEffect::PullToward,
    },
    Recipe {
        inputs: &["水", "手"],
        output_hanzi: "汲",
        output_pinyin: "jí",
        output_meaning: "to draw water",
        effect: SpellEffect::PullToward,
    },
    // --- KnockBack recipes ---
    Recipe {
        inputs: &["手", "大"],
        output_hanzi: "推",
        output_pinyin: "tuī",
        output_meaning: "to push",
        effect: SpellEffect::KnockBack(3),
    },
    Recipe {
        inputs: &["石", "力"],
        output_hanzi: "砸",
        output_pinyin: "zá",
        output_meaning: "to smash",
        effect: SpellEffect::KnockBack(4),
    },
    // --- Thorns recipes ---
    Recipe {
        inputs: &["竹", "虫"],
        output_hanzi: "蔑",
        output_pinyin: "miè",
        output_meaning: "bamboo thorn",
        effect: SpellEffect::Thorns(3),
    },
    Recipe {
        inputs: &["木", "小"],
        output_hanzi: "棘",
        output_pinyin: "jí",
        output_meaning: "thorns/bramble",
        effect: SpellEffect::Thorns(4),
    },
    // --- Cone recipes ---
    Recipe {
        inputs: &["火", "大"],
        output_hanzi: "炎",
        output_pinyin: "yán",
        output_meaning: "blaze",
        effect: SpellEffect::Cone(3),
    },
    Recipe {
        inputs: &["风", "火"],
        output_hanzi: "烽",
        output_pinyin: "fēng",
        output_meaning: "beacon fire",
        effect: SpellEffect::Cone(4),
    },
    // --- Wall recipes ---
    Recipe {
        inputs: &["土", "山"],
        output_hanzi: "堤",
        output_pinyin: "dī",
        output_meaning: "embankment",
        effect: SpellEffect::Wall(3),
    },
    Recipe {
        inputs: &["石", "土"],
        output_hanzi: "砌",
        output_pinyin: "qì",
        output_meaning: "to build/stack",
        effect: SpellEffect::Wall(3),
    },
    // --- Rare recipes (stronger variants) ---
    Recipe {
        inputs: &["雷", "金"],
        output_hanzi: "锋",
        output_pinyin: "fēng",
        output_meaning: "sharp edge",
        effect: SpellEffect::Pierce(5),
    },
    Recipe {
        inputs: &["龙", "石"],
        output_hanzi: "壁",
        output_pinyin: "bì",
        output_meaning: "dragon wall",
        effect: SpellEffect::Wall(5),
    },
    // ── HSK 2 recipes (using new radicals: 言门犬禾车又白气) ──────────────
    Recipe {
        inputs: &["言", "口"],
        output_hanzi: "话",
        output_pinyin: "huà",
        output_meaning: "speech/words",
        effect: SpellEffect::Stun,
    },
    Recipe {
        inputs: &["言", "心"],
        output_hanzi: "诚",
        output_pinyin: "chéng",
        output_meaning: "sincere",
        effect: SpellEffect::Pacify,
    },
    Recipe {
        inputs: &["言", "刀"],
        output_hanzi: "诀",
        output_pinyin: "jué",
        output_meaning: "incantation",
        effect: SpellEffect::Pierce(4),
    },
    Recipe {
        inputs: &["门", "口"],
        output_hanzi: "问",
        output_pinyin: "wèn",
        output_meaning: "to ask",
        effect: SpellEffect::Reveal,
    },
    Recipe {
        inputs: &["门", "日"],
        output_hanzi: "间",
        output_pinyin: "jiān",
        output_meaning: "space/between",
        effect: SpellEffect::Teleport,
    },
    Recipe {
        inputs: &["门", "心"],
        output_hanzi: "闷",
        output_pinyin: "mèn",
        output_meaning: "stifled/stuffy",
        effect: SpellEffect::Slow(3),
    },
    Recipe {
        inputs: &["犬", "口"],
        output_hanzi: "吠",
        output_pinyin: "fèi",
        output_meaning: "to bark",
        effect: SpellEffect::KnockBack(2),
    },
    Recipe {
        inputs: &["犬", "火"],
        output_hanzi: "狱",
        output_pinyin: "yù",
        output_meaning: "prison/hell",
        effect: SpellEffect::FireAoe(6),
    },
    Recipe {
        inputs: &["犬", "言"],
        output_hanzi: "狺",
        output_pinyin: "yín",
        output_meaning: "snarling",
        effect: SpellEffect::Thorns(4),
    },
    Recipe {
        inputs: &["禾", "日"],
        output_hanzi: "香",
        output_pinyin: "xiāng",
        output_meaning: "fragrant",
        effect: SpellEffect::Heal(6),
    },
    Recipe {
        inputs: &["禾", "刀"],
        output_hanzi: "利",
        output_pinyin: "lì",
        output_meaning: "sharp/benefit",
        effect: SpellEffect::StrongHit(6),
    },
    Recipe {
        inputs: &["禾", "火"],
        output_hanzi: "秋",
        output_pinyin: "qiū",
        output_meaning: "autumn",
        effect: SpellEffect::Cone(3),
    },
    Recipe {
        inputs: &["车", "力"],
        output_hanzi: "轧",
        output_pinyin: "yà",
        output_meaning: "to crush/roll over",
        effect: SpellEffect::Dash(4),
    },
    Recipe {
        inputs: &["车", "风"],
        output_hanzi: "飙",
        output_pinyin: "biāo",
        output_meaning: "whirlwind",
        effect: SpellEffect::KnockBack(3),
    },
    Recipe {
        inputs: &["又", "木"],
        output_hanzi: "权",
        output_pinyin: "quán",
        output_meaning: "authority/power",
        effect: SpellEffect::ArmorBreak,
    },
    Recipe {
        inputs: &["又", "鸟"],
        output_hanzi: "鸡",
        output_pinyin: "jī",
        output_meaning: "rooster",
        effect: SpellEffect::Pierce(2),
    },
    Recipe {
        inputs: &["又", "贝"],
        output_hanzi: "贰",
        output_pinyin: "èr",
        output_meaning: "double/duplicate",
        effect: SpellEffect::FocusRestore(4),
    },
    Recipe {
        inputs: &["白", "水"],
        output_hanzi: "泉",
        output_pinyin: "quán",
        output_meaning: "spring/fountain",
        effect: SpellEffect::Heal(8),
    },
    Recipe {
        inputs: &["白", "王"],
        output_hanzi: "皇",
        output_pinyin: "huáng",
        output_meaning: "emperor",
        effect: SpellEffect::StrongHit(8),
    },
    Recipe {
        inputs: &["白", "月"],
        output_hanzi: "皎",
        output_pinyin: "jiǎo",
        output_meaning: "bright moonlight",
        effect: SpellEffect::Shield,
    },
    Recipe {
        inputs: &["气", "水"],
        output_hanzi: "氽",
        output_pinyin: "tǔn",
        output_meaning: "to float/boil",
        effect: SpellEffect::Poison(2, 4),
    },
    Recipe {
        inputs: &["气", "山"],
        output_hanzi: "岚",
        output_pinyin: "lán",
        output_meaning: "mountain mist",
        effect: SpellEffect::Slow(4),
    },
    Recipe {
        inputs: &["气", "力"],
        output_hanzi: "劲",
        output_pinyin: "jìn",
        output_meaning: "strength/force",
        effect: SpellEffect::Dash(3),
    },
    Recipe {
        inputs: &["门", "马"],
        output_hanzi: "闯",
        output_pinyin: "chuǎng",
        output_meaning: "to rush/charge",
        effect: SpellEffect::Dash(5),
    },
    Recipe {
        inputs: &["白", "气", "火"],
        output_hanzi: "炽",
        output_pinyin: "chì",
        output_meaning: "blazing",
        effect: SpellEffect::Cone(5),
    },
    Recipe {
        inputs: &["言", "禾", "心"],
        output_hanzi: "谢",
        output_pinyin: "xiè",
        output_meaning: "to thank/wither",
        effect: SpellEffect::Drain(6),
    },
    Recipe {
        inputs: &["车", "门", "金"],
        output_hanzi: "锁",
        output_pinyin: "suǒ",
        output_meaning: "lock/chain",
        effect: SpellEffect::PullToward,
    },
    Recipe {
        inputs: &["犬", "龙"],
        output_hanzi: "獒",
        output_pinyin: "áo",
        output_meaning: "mastiff",
        effect: SpellEffect::KnockBack(6),
    },
    Recipe {
        inputs: &["气", "雷"],
        output_hanzi: "霆",
        output_pinyin: "tíng",
        output_meaning: "thunderbolt",
        effect: SpellEffect::Pierce(6),
    },
    // ── Terrain Spells ──────────────────────────────────────────────────────
    Recipe {
        inputs: &["水", "土"],
        output_hanzi: "油",
        output_pinyin: "yóu",
        output_meaning: "oil",
        effect: SpellEffect::OilSlick,
    },
    Recipe {
        inputs: &["水", "石"],
        output_hanzi: "滑",
        output_pinyin: "huá",
        output_meaning: "slippery",
        effect: SpellEffect::OilSlick,
    },
    Recipe {
        inputs: &["水", "金"],
        output_hanzi: "冰",
        output_pinyin: "bīng",
        output_meaning: "ice",
        effect: SpellEffect::FreezeGround(3),
    },
    Recipe {
        inputs: &["雨", "金"],
        output_hanzi: "冻",
        output_pinyin: "dòng",
        output_meaning: "to freeze",
        effect: SpellEffect::FreezeGround(2),
    },
    Recipe {
        inputs: &["火", "气"],
        output_hanzi: "燃",
        output_pinyin: "rán",
        output_meaning: "to ignite",
        effect: SpellEffect::Ignite,
    },
    Recipe {
        inputs: &["火", "田"],
        output_hanzi: "燎",
        output_pinyin: "liáo",
        output_meaning: "wildfire",
        effect: SpellEffect::Ignite,
    },
    Recipe {
        inputs: &["木", "土"],
        output_hanzi: "林",
        output_pinyin: "lín",
        output_meaning: "forest",
        effect: SpellEffect::PlantGrowth,
    },
    Recipe {
        inputs: &["禾", "水"],
        output_hanzi: "苗",
        output_pinyin: "miáo",
        output_meaning: "sprout/seedling",
        effect: SpellEffect::PlantGrowth,
    },
    Recipe {
        inputs: &["土", "力", "山"],
        output_hanzi: "震",
        output_pinyin: "zhèn",
        output_meaning: "earthquake",
        effect: SpellEffect::Earthquake(4),
    },
    Recipe {
        inputs: &["山", "力"],
        output_hanzi: "崩",
        output_pinyin: "bēng",
        output_meaning: "to collapse",
        effect: SpellEffect::Earthquake(3),
    },
    Recipe {
        inputs: &["日", "心"],
        output_hanzi: "圣",
        output_pinyin: "shèng",
        output_meaning: "holy/sacred",
        effect: SpellEffect::Sanctify(2),
    },
    Recipe {
        inputs: &["白", "日", "心"],
        output_hanzi: "晖",
        output_pinyin: "huī",
        output_meaning: "radiance",
        effect: SpellEffect::Sanctify(3),
    },
    Recipe {
        inputs: &["水", "大"],
        output_hanzi: "洪",
        output_pinyin: "hóng",
        output_meaning: "flood",
        effect: SpellEffect::FloodWave(3),
    },
    Recipe {
        inputs: &["雨", "大"],
        output_hanzi: "涝",
        output_pinyin: "lào",
        output_meaning: "waterlogging",
        effect: SpellEffect::FloodWave(2),
    },
    Recipe {
        inputs: &["石", "大"],
        output_hanzi: "磊",
        output_pinyin: "lěi",
        output_meaning: "pile of rocks",
        effect: SpellEffect::SummonBoulder,
    },
    Recipe {
        inputs: &["土", "大"],
        output_hanzi: "墩",
        output_pinyin: "dūn",
        output_meaning: "mound/block",
        effect: SpellEffect::SummonBoulder,
    },
    // ── Charge recipes ──────────────────────────────────────────────────────
    Recipe {
        inputs: &["马", "火"],
        output_hanzi: "骋",
        output_pinyin: "chěng",
        output_meaning: "to gallop/charge",
        effect: SpellEffect::Charge(3),
    },
    Recipe {
        inputs: &["车", "大"],
        output_hanzi: "辗",
        output_pinyin: "niǎn",
        output_meaning: "to roll over",
        effect: SpellEffect::Charge(4),
    },
    Recipe {
        inputs: &["力", "足"],
        output_hanzi: "蹴",
        output_pinyin: "cù",
        output_meaning: "to kick/rush",
        effect: SpellEffect::Charge(3),
    },
    // ── Blink recipes ───────────────────────────────────────────────────────
    Recipe {
        inputs: &["门", "风"],
        output_hanzi: "闪",
        output_pinyin: "shǎn",
        output_meaning: "flash/dodge",
        effect: SpellEffect::Blink(3),
    },
    Recipe {
        inputs: &["风", "足"],
        output_hanzi: "遁",
        output_pinyin: "dùn",
        output_meaning: "to vanish/escape",
        effect: SpellEffect::Blink(2),
    },
    Recipe {
        inputs: &["气", "门"],
        output_hanzi: "瞬",
        output_pinyin: "shùn",
        output_meaning: "instant/blink",
        effect: SpellEffect::Blink(4),
    },
];

/// Try to synthesize a character from a set of radicals. Order-independent.
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
const COMMON_RADICAL_COUNT: usize = 41;

/// Get a subset of radicals available for a given deck.
/// Earlier decks have fewer radicals. Excludes rare radicals.
pub fn radicals_for_floor(floor: i32) -> &'static [Radical] {
    let count = match floor {
        1 => 10,
        2 => 16,
        3 => 22,
        4 => 28,
        5 => 34,
        _ => COMMON_RADICAL_COUNT,
    };
    &RADICALS[..count.min(COMMON_RADICAL_COUNT)]
}

/// Get the list of rare radicals (elite drops only).
pub fn rare_radicals() -> &'static [Radical] {
    &RADICALS[COMMON_RADICAL_COUNT..]
}


#[cfg(test)]
mod tests;
