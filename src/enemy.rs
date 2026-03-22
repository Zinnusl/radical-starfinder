//! Enemy entities that inhabit the station decks.

use crate::status::StatusInstance;
use crate::vocab::VocabEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiBehavior {
    Chase,
    Retreat,
    Ambush,
    Sentinel,
    Kiter,
    Pack,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossKind {
    PirateCaptain,
    HiveQueen,
    RogueAICore,
    VoidEntity,
    AncientGuardian,
    DriftLeviathan,
}

impl BossKind {
    pub fn for_floor(floor: i32) -> Option<Self> {
        match floor {
            5 => Some(Self::PirateCaptain),
            10 => Some(Self::HiveQueen),
            15 => Some(Self::RogueAICore),
            20 => Some(Self::VoidEntity),
            25 => Some(Self::AncientGuardian),
            30 => Some(Self::DriftLeviathan),
            _ => None,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::PirateCaptain => "Pirate Captain",
            Self::HiveQueen => "Hive Queen",
            Self::RogueAICore => "Rogue AI Core",
            Self::VoidEntity => "Void Entity",
            Self::AncientGuardian => "Ancient Guardian",
            Self::DriftLeviathan => "Drift Leviathan",
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/decomposition_data.rs"));

/// Special abilities derived from hanzi semantic components.
/// Each radical maps to exactly ONE skill (1:1 mapping).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadicalAction {
    /// 火 fire — Plasma DoT, spreading plasma terrain
    SpreadingWildfire,
    /// 水 water — Corrosive spray, Slow
    ErosiveFlow,
    /// 力 strength — Damage scales with missing HP
    OverwhelmingForce,
    /// 心 heart — Confuse for 2 turns
    DoubtSeed,
    /// 口 mouth — Deal 1 damage, steal a buff
    DevouringMaw,
    /// 目 eye — Mark: all attacks auto-hit +crit
    WitnessMark,
    /// 手 hand — Swap positions with target
    SleightReversal,
    /// 木 wood — Gravity field, +1 armor to self
    RootingGrasp,
    /// 田 field — Execute if HP<40%, else 1 dmg
    HarvestReaping,
    /// 日 sun — Clear own debuffs, buff allies
    RevealingDawn,
    /// 月 moon — Poison DoT 2 dmg for 3 turns
    WaningCurse,
    /// 人 person — If low HP: survive at 1, +2 dmg
    MortalResilience,
    /// 女 woman — Counter + thorn armor
    MaternalShield,
    /// 子 child — 1 dmg + mark for +2 future dmg
    PotentialBurst,
    /// 禾 grain — Confused 1 turn (miss chance)
    ChasingChaff,
    /// 十 cross — 50/50: 4 dmg or stun self
    CrossroadsGambit,
    /// 金 metal — +4 armor, can't dodge
    RigidStance,
    /// 土 earth — Slow 3 turns + 1 dmg
    GroundingWeight,
    /// 又 again — Repeat base damage attack
    EchoStrike,
    /// 寸 inch — Execute if HP<25%, else 1 dmg
    PreciseExecution,
    /// 刀 knife — 2 dmg + reduce max HP by 1
    CleavingCut,
    /// 言 speech — Slow 3 + Confused 1
    BindingOath,
    /// 足 foot — Dash to player + 1 dmg
    PursuingSteps,
    /// 糸 silk — Slow 3 + Bleed 1 for 2 turns
    EntanglingWeb,
    /// 门 gate — +3 armor, Slow self 1 turn
    ThresholdSeal,
    /// 马 horse — Dmg scales with distance, knockback 2
    CavalryCharge,
    /// 鸟 bird — Dodge + stored movement +2
    SoaringEscape,
    /// 雨 rain — 1 dmg + Bleed 1 for 3 turns
    DownpourBarrage,
    /// 石 stone — Slow 3 + give self +2 armor
    PetrifyingGaze,
    /// 虫 insect — 1 dmg, heal self for dealt+1
    ParasiticSwarm,
    /// 贝 shell — Self -2 HP, buff nearby allies
    MercenaryPact,
    /// 山 mountain — +3 armor, +1 fortify
    ImmovablePeak,
    /// 犬 beast — 3 dmg, self-dmg 1, heal 1
    SavageMaul,
    /// 弓 bow — Ranged: 3 dmg at dist>=3, else 1
    ArcingShot,
    /// 食 food — 2 dmg, heal for dealt, +1 max HP
    ConsumingBite,
    /// 衣 clothing — Dodge + fortify (stealth)
    CloakingGuise,
    /// 竹 bamboo — Counter + thorn armor 2 turns
    FlexibleCounter,
    /// 走 walk — Move 3 tiles toward player, dmg per tile
    BlitzAssault,
    /// 车 vehicle — 2 dmg + push 3 tiles
    CrushingWheels,
    /// 王 king — Force nearest ally to gain +2 dmg, approach player
    ImperialCommand,
    /// 大 big — All nearby allies +1 dmg, self +1 dmg
    MagnifyingAura,
    /// 小 small — 2 dmg ignoring all armor
    NeedleStrike,
    /// 工 craft — Burn 2 for 2 turns (delayed trap)
    ArtisanTrap,
    /// 白 white — Remove debuffs from self, heal 3
    CleansingLight,
    /// 页 page — Confuse all units in 2-tile radius for 1 turn
    ScatteringPages,
    /// 见 see — Remove all buffs from target (dispel)
    TrueVision,
    /// 气 air/energy — Drain focus/spirit from player
    QiDisruption,
    /// 广 wide — Create zone where enemy gets +1 damage
    ExpandingDomain,
    /// 穴 cave — Create CrackedFloor under player (delayed pit)
    SinkholeSnare,
    /// 耳 ear — 2 damage AoE in 2-tile radius + stun
    SonicBurst,
    /// 舌 tongue — Ranged 1 dmg + Poison for 2 turns
    VenomousLash,
    /// 身 body — Gain 2 armor + root self (can't move) 2 turns
    IronBodyStance,
    /// 角 horn — Charge toward player + 2 dmg + knockback
    GoreCrush,
    /// 酉 wine vessel — Steam tiles in radius, Confused 2 turns
    IntoxicatingMist,
    /// 豆 bean — Create Grass tiles, gain armor per adjacent Grass
    SproutingBarrier,
    /// 鱼 fish — Water tile under player + push 2, extra dmg on Water
    TidalSurge,
    /// 骨 bone — ArcingProjectile: 3 dmg + armor break
    BoneShatter,
    /// 革 leather — Gain +2 armor and +1 fortify (adaptive defense)
    AdaptiveShift,
    /// 鬥 fight — Self-damage 2 + gain +3 damage for 2 turns
    BerserkerFury,
    /// 隹 short-tailed bird — 3 ArcingProjectiles at random tiles near player
    FlockAssault,
}

impl RadicalAction {
    pub fn radical(self) -> &'static str {
        match self {
            Self::SpreadingWildfire => "火",
            Self::ErosiveFlow => "水",
            Self::OverwhelmingForce => "力",
            Self::DoubtSeed => "心",
            Self::DevouringMaw => "口",
            Self::WitnessMark => "目",
            Self::SleightReversal => "手",
            Self::RootingGrasp => "木",
            Self::HarvestReaping => "田",
            Self::RevealingDawn => "日",
            Self::WaningCurse => "月",
            Self::MortalResilience => "人",
            Self::MaternalShield => "女",
            Self::PotentialBurst => "子",
            Self::ChasingChaff => "禾",
            Self::CrossroadsGambit => "十",
            Self::RigidStance => "金",
            Self::GroundingWeight => "土",
            Self::EchoStrike => "又",
            Self::PreciseExecution => "寸",
            Self::CleavingCut => "刀",
            Self::BindingOath => "言",
            Self::PursuingSteps => "足",
            Self::EntanglingWeb => "糸",
            Self::ThresholdSeal => "门",
            Self::CavalryCharge => "马",
            Self::SoaringEscape => "鸟",
            Self::DownpourBarrage => "雨",
            Self::PetrifyingGaze => "石",
            Self::ParasiticSwarm => "虫",
            Self::MercenaryPact => "贝",
            Self::ImmovablePeak => "山",
            Self::SavageMaul => "犬",
            Self::ArcingShot => "弓",
            Self::ConsumingBite => "食",
            Self::CloakingGuise => "衣",
            Self::FlexibleCounter => "竹",
            Self::BlitzAssault => "走",
            Self::CrushingWheels => "车",
            Self::ImperialCommand => "王",
            Self::MagnifyingAura => "大",
            Self::NeedleStrike => "小",
            Self::ArtisanTrap => "工",
            Self::CleansingLight => "白",
            Self::ScatteringPages => "页",
            Self::TrueVision => "见",
            Self::QiDisruption => "气",
            Self::ExpandingDomain => "广",
            Self::SinkholeSnare => "穴",
            Self::SonicBurst => "耳",
            Self::VenomousLash => "舌",
            Self::IronBodyStance => "身",
            Self::GoreCrush => "角",
            Self::IntoxicatingMist => "酉",
            Self::SproutingBarrier => "豆",
            Self::TidalSurge => "鱼",
            Self::BoneShatter => "骨",
            Self::AdaptiveShift => "革",
            Self::BerserkerFury => "鬥",
            Self::FlockAssault => "隹",
        }
    }

    pub fn from_radical(radical: &str) -> Option<Self> {
        match radical {
            "火" => Some(Self::SpreadingWildfire),
            "水" => Some(Self::ErosiveFlow),
            "力" => Some(Self::OverwhelmingForce),
            "心" => Some(Self::DoubtSeed),
            "口" => Some(Self::DevouringMaw),
            "目" => Some(Self::WitnessMark),
            "手" => Some(Self::SleightReversal),
            "木" => Some(Self::RootingGrasp),
            "田" => Some(Self::HarvestReaping),
            "日" => Some(Self::RevealingDawn),
            "月" => Some(Self::WaningCurse),
            "人" => Some(Self::MortalResilience),
            "女" => Some(Self::MaternalShield),
            "子" => Some(Self::PotentialBurst),
            "禾" => Some(Self::ChasingChaff),
            "十" => Some(Self::CrossroadsGambit),
            "金" => Some(Self::RigidStance),
            "土" => Some(Self::GroundingWeight),
            "又" => Some(Self::EchoStrike),
            "寸" => Some(Self::PreciseExecution),
            "刀" => Some(Self::CleavingCut),
            "言" => Some(Self::BindingOath),
            "足" => Some(Self::PursuingSteps),
            "糸" => Some(Self::EntanglingWeb),
            "门" => Some(Self::ThresholdSeal),
            "马" => Some(Self::CavalryCharge),
            "鸟" => Some(Self::SoaringEscape),
            "雨" => Some(Self::DownpourBarrage),
            "石" => Some(Self::PetrifyingGaze),
            "虫" => Some(Self::ParasiticSwarm),
            "贝" => Some(Self::MercenaryPact),
            "山" => Some(Self::ImmovablePeak),
            "犬" => Some(Self::SavageMaul),
            "弓" => Some(Self::ArcingShot),
            "食" => Some(Self::ConsumingBite),
            "衣" => Some(Self::CloakingGuise),
            "竹" => Some(Self::FlexibleCounter),
            "走" => Some(Self::BlitzAssault),
            "车" => Some(Self::CrushingWheels),
            "王" => Some(Self::ImperialCommand),
            "大" => Some(Self::MagnifyingAura),
            "小" => Some(Self::NeedleStrike),
            "工" => Some(Self::ArtisanTrap),
            "白" => Some(Self::CleansingLight),
            "页" => Some(Self::ScatteringPages),
            "见" => Some(Self::TrueVision),
            "气" => Some(Self::QiDisruption),
            "广" => Some(Self::ExpandingDomain),
            "穴" => Some(Self::SinkholeSnare),
            "耳" => Some(Self::SonicBurst),
            "舌" => Some(Self::VenomousLash),
            "身" => Some(Self::IronBodyStance),
            "角" => Some(Self::GoreCrush),
            "酉" => Some(Self::IntoxicatingMist),
            "豆" => Some(Self::SproutingBarrier),
            "鱼" => Some(Self::TidalSurge),
            "骨" => Some(Self::BoneShatter),
            "革" => Some(Self::AdaptiveShift),
            "鬥" => Some(Self::BerserkerFury),
            "隹" => Some(Self::FlockAssault),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::SpreadingWildfire => "\u{1F525} Plasma Overload",
            Self::ErosiveFlow => "\u{1F4A7} Acid Spray",
            Self::OverwhelmingForce => "\u{1F4AA} Overwhelming Force",
            Self::DoubtSeed => "\u{1F49C} Doubt Seed",
            Self::DevouringMaw => "\u{1F444} Devouring Maw",
            Self::WitnessMark => "\u{1F441} Witness Mark",
            Self::SleightReversal => "\u{270B} Sleight Reversal",
            Self::RootingGrasp => "\u{1F33F} Gravity Snare",
            Self::HarvestReaping => "\u{1F33E} Harvest Reaping",
            Self::RevealingDawn => "\u{2600} Revealing Dawn",
            Self::WaningCurse => "\u{1F319} Waning Curse",
            Self::MortalResilience => "\u{1F464} Mortal Resilience",
            Self::MaternalShield => "\u{1F495} Maternal Shield",
            Self::PotentialBurst => "\u{26A1} Potential Burst",
            Self::ChasingChaff => "\u{1F33E} Chasing Chaff",
            Self::CrossroadsGambit => "\u{2715} Crossroads Gambit",
            Self::RigidStance => "\u{1F6E1} Rigid Stance",
            Self::GroundingWeight => "\u{1F30D} Grounding Weight",
            Self::EchoStrike => "\u{1F504} Echo Strike",
            Self::PreciseExecution => "\u{1F4CF} Precise Execution",
            Self::CleavingCut => "\u{1F5E1} Cleaving Cut",
            Self::BindingOath => "\u{1F4DC} Binding Oath",
            Self::PursuingSteps => "\u{1F463} Pursuing Steps",
            Self::EntanglingWeb => "\u{1F578} Entangling Web",
            Self::ThresholdSeal => "\u{1F6AA} Threshold Seal",
            Self::CavalryCharge => "\u{1F434} Cavalry Charge",
            Self::SoaringEscape => "\u{1F426} Soaring Escape",
            Self::DownpourBarrage => "\u{1F327} Downpour Barrage",
            Self::PetrifyingGaze => "\u{1FAA8} Petrifying Gaze",
            Self::ParasiticSwarm => "\u{1F41B} Parasitic Swarm",
            Self::MercenaryPact => "\u{1F41A} Mercenary Pact",
            Self::ImmovablePeak => "\u{26F0} Immovable Peak",
            Self::SavageMaul => "\u{1F415} Savage Maul",
            Self::ArcingShot => "\u{1F3F9} Arcing Shot",
            Self::ConsumingBite => "\u{1F356} Consuming Bite",
            Self::CloakingGuise => "\u{1F458} Cloaking Guise",
            Self::FlexibleCounter => "\u{1F38B} Flexible Counter",
            Self::BlitzAssault => "\u{1F4A8} Blitz Assault",
            Self::CrushingWheels => "\u{1F6DE} Crushing Wheels",
            Self::ImperialCommand => "\u{1F451} Imperial Command",
            Self::MagnifyingAura => "\u{1F53A} Magnifying Aura",
            Self::NeedleStrike => "\u{1F53B} Needle Strike",
            Self::ArtisanTrap => "\u{2699} Artisan's Trap",
            Self::CleansingLight => "\u{2728} Cleansing Light",
            Self::ScatteringPages => "\u{1F4C4} Scattering Pages",
            Self::TrueVision => "\u{1F440} True Vision",
            Self::QiDisruption => "\u{1F32C} Qi Disruption",
            Self::ExpandingDomain => "\u{1F310} Expanding Domain",
            Self::SinkholeSnare => "\u{1F573} Sinkhole Snare",
            Self::SonicBurst => "\u{1F442} Sonic Burst",
            Self::VenomousLash => "\u{1F40D} Venomous Lash",
            Self::IronBodyStance => "\u{1F9CD} Iron Body Stance",
            Self::GoreCrush => "\u{1F402} Gore Crush",
            Self::IntoxicatingMist => "\u{1F37A} Intoxicating Mist",
            Self::SproutingBarrier => "\u{1F331} Sprouting Barrier",
            Self::TidalSurge => "\u{1F41F} Tidal Surge",
            Self::BoneShatter => "\u{1F9B4} Bone Shatter",
            Self::AdaptiveShift => "\u{1F6E1} Adaptive Shift",
            Self::BerserkerFury => "\u{1F4A2} Berserker Fury",
            Self::FlockAssault => "\u{1F426} Flock Assault",
        }
    }

    #[allow(dead_code)]
    pub fn description(self) -> &'static str {
        match self {
            Self::SpreadingWildfire => "Burns for 1 damage over 3 turns, creates plasma terrain",
            Self::ErosiveFlow => "Corrodes defense, Slows for 3 turns",
            Self::OverwhelmingForce => "Damage scales with missing HP",
            Self::DoubtSeed => "Confuses for 2 turns",
            Self::DevouringMaw => "Deals 1 damage and steals a buff",
            Self::WitnessMark => "Marks target: attacks auto-hit with +2 damage",
            Self::SleightReversal => "Swaps positions with target",
            Self::RootingGrasp => "Gravity locks for 2 turns, gains +1 armor",
            Self::HarvestReaping => "Execute if HP<40%, else 1 damage",
            Self::RevealingDawn => "Clears own debuffs, buffs nearby allies",
            Self::WaningCurse => "Poison DoT: 2 damage for 3 turns",
            Self::MortalResilience => "If low HP: survive at 1 HP, gain +2 damage",
            Self::MaternalShield => "Gains counter-attack and thorn armor",
            Self::PotentialBurst => "Deals 1 damage + marks for +2 future damage",
            Self::ChasingChaff => "Confuses for 1 turn (miss chance)",
            Self::CrossroadsGambit => "50/50: deal 4 damage or stun self",
            Self::RigidStance => "Gains +4 armor, cannot dodge",
            Self::GroundingWeight => "Slows for 3 turns + 1 damage",
            Self::EchoStrike => "Repeats base damage attack",
            Self::PreciseExecution => "Execute if HP<25%, else 1 damage",
            Self::CleavingCut => "Deals 2 damage + reduces max HP by 1",
            Self::BindingOath => "Slows for 3 turns + Confuses for 1 turn",
            Self::PursuingSteps => "Dashes to player + 1 damage",
            Self::EntanglingWeb => "Slows for 3 turns + Bleed 1 for 2 turns",
            Self::ThresholdSeal => "Gains +3 armor, Slows self 1 turn",
            Self::CavalryCharge => "Damage scales with distance, knockback 2",
            Self::SoaringEscape => "Dodges next attack + gains +2 movement",
            Self::DownpourBarrage => "Deals 1 damage + Bleed 1 for 3 turns",
            Self::PetrifyingGaze => "Slows for 3 turns + gains +2 armor",
            Self::ParasiticSwarm => "Deals 1 damage, heals self for dealt+1",
            Self::MercenaryPact => "Self -2 HP, buffs nearby allies +1 damage",
            Self::ImmovablePeak => "Gains +3 armor and +1 fortify",
            Self::SavageMaul => "Deals 3 damage, self-damage 1, heals 1",
            Self::ArcingShot => "Ranged: 3 damage at distance>=3, else 1",
            Self::ConsumingBite => "Deals 2 damage, heals for dealt, +1 max HP",
            Self::CloakingGuise => "Dodges next attack + gains fortify",
            Self::FlexibleCounter => "Gains counter-attack + thorn armor 2 turns",
            Self::BlitzAssault => "Moves 3 tiles toward player, damage per tile moved",
            Self::CrushingWheels => "Deals 2 damage + pushes 3 tiles",
            Self::ImperialCommand => "Nearest ally gains +2 damage and approaches player",
            Self::MagnifyingAura => "All nearby allies and self gain +1 damage",
            Self::NeedleStrike => "Deals 2 damage ignoring all armor",
            Self::ArtisanTrap => "Burns for 2 damage over 2 turns",
            Self::CleansingLight => "Removes all debuffs, heals 3 HP",
            Self::ScatteringPages => "Confuses all units in 2-tile radius for 1 turn",
            Self::TrueVision => "Removes all buffs from target (dispel)",
            Self::QiDisruption => "Drains 3 spirit from the player",
            Self::ExpandingDomain => "Gains +1 damage in a 3x3 zone, +1 armor",
            Self::SinkholeSnare => "Creates cracked floor under player (1 turn to escape)",
            Self::SonicBurst => "2 damage to all in 2-tile radius + stun",
            Self::VenomousLash => "Ranged: 1 damage + Poison 2 for 2 turns",
            Self::IronBodyStance => "Gains +2 armor, roots self for 2 turns",
            Self::GoreCrush => "Charges 2 tiles + 2 damage + knockback 1",
            Self::IntoxicatingMist => "Creates Steam tiles, Confuses 2 turns",
            Self::SproutingBarrier => "Creates Grass tiles, +1 armor per adjacent Grass",
            Self::TidalSurge => "Water under player + push 2, bonus damage on Water",
            Self::BoneShatter => "Arcing 3 damage + removes armor (lands in 1 turn)",
            Self::AdaptiveShift => "Gains +2 armor and +1 fortify",
            Self::BerserkerFury => "Self-damage 2, gains +3 damage for 2 turns",
            Self::FlockAssault => "3 arcing projectiles at random tiles near player",
        }
    }

    pub fn range_info(self) -> &'static str {
        match self {
            // Self-targeted
            Self::RevealingDawn
            | Self::MortalResilience
            | Self::MaternalShield
            | Self::RigidStance
            | Self::ThresholdSeal
            | Self::SoaringEscape
            | Self::ImmovablePeak
            | Self::CloakingGuise
            | Self::FlexibleCounter
            | Self::CleansingLight
            | Self::IronBodyStance
            | Self::SproutingBarrier
            | Self::AdaptiveShift
            | Self::BerserkerFury
            | Self::MercenaryPact
            | Self::ImperialCommand
            | Self::MagnifyingAura
            | Self::ExpandingDomain => "Self",

            // Melee (adjacent)
            Self::DevouringMaw
            | Self::SleightReversal
            | Self::HarvestReaping
            | Self::PreciseExecution
            | Self::CleavingCut
            | Self::SavageMaul
            | Self::ConsumingBite
            | Self::EchoStrike => "Melee (1)",

            // Short range (2 tiles)
            Self::PotentialBurst
            | Self::ParasiticSwarm
            | Self::NeedleStrike
            | Self::GoreCrush => "Short (2)",

            // Medium range (3-4 tiles)
            Self::SpreadingWildfire
            | Self::ErosiveFlow
            | Self::OverwhelmingForce
            | Self::DoubtSeed
            | Self::WitnessMark
            | Self::RootingGrasp
            | Self::ChasingChaff
            | Self::CrossroadsGambit
            | Self::GroundingWeight
            | Self::WaningCurse
            | Self::BindingOath
            | Self::EntanglingWeb
            | Self::PetrifyingGaze
            | Self::CrushingWheels
            | Self::TrueVision
            | Self::QiDisruption
            | Self::VenomousLash
            | Self::ArtisanTrap => "Medium (3)",

            // Long range (5+ tiles)
            Self::PursuingSteps
            | Self::CavalryCharge
            | Self::BlitzAssault
            | Self::SinkholeSnare
            | Self::TidalSurge
            | Self::DownpourBarrage
            | Self::ArcingShot
            | Self::BoneShatter
            | Self::FlockAssault => "Long (5+)",

            // AoE (radius-based)
            Self::ScatteringPages
            | Self::SonicBurst
            | Self::IntoxicatingMist => "AoE (2)",
        }
    }

    pub fn damage_info(self) -> &'static str {
        match self {
            Self::SpreadingWildfire => "DoT 1/turn ×3",
            Self::ErosiveFlow => "No damage",
            Self::OverwhelmingForce => "1-4 (scales HP)",
            Self::DoubtSeed => "No damage",
            Self::DevouringMaw => "1 dmg",
            Self::WitnessMark => "No damage",
            Self::SleightReversal => "No damage",
            Self::RootingGrasp => "No damage",
            Self::HarvestReaping => "1 or execute",
            Self::RevealingDawn => "No damage",
            Self::WaningCurse => "DoT 2/turn ×3",
            Self::MortalResilience => "No damage",
            Self::MaternalShield => "No damage",
            Self::PotentialBurst => "1 + mark +2",
            Self::ChasingChaff => "No damage",
            Self::CrossroadsGambit => "0 or 4 dmg",
            Self::RigidStance => "No damage",
            Self::GroundingWeight => "1 dmg",
            Self::EchoStrike => "2× base dmg",
            Self::PreciseExecution => "1 or execute",
            Self::CleavingCut => "2 dmg",
            Self::BindingOath => "No damage",
            Self::PursuingSteps => "1 dmg",
            Self::EntanglingWeb => "No damage",
            Self::ThresholdSeal => "No damage",
            Self::CavalryCharge => "1-4 (scales dist)",
            Self::SoaringEscape => "No damage",
            Self::DownpourBarrage => "1 + bleed",
            Self::PetrifyingGaze => "No damage",
            Self::ParasiticSwarm => "1 + heal self",
            Self::MercenaryPact => "Self -2 HP",
            Self::ImmovablePeak => "No damage",
            Self::SavageMaul => "3 dmg",
            Self::ArcingShot => "1-3 (by dist)",
            Self::ConsumingBite => "2 + heal self",
            Self::CloakingGuise => "No damage",
            Self::FlexibleCounter => "No damage",
            Self::BlitzAssault => "1 dmg/tile",
            Self::CrushingWheels => "2 + push 3",
            Self::ImperialCommand => "No damage",
            Self::MagnifyingAura => "No damage",
            Self::NeedleStrike => "2 (pierce armor)",
            Self::ArtisanTrap => "Burn 2/turn ×2",
            Self::CleansingLight => "Heal 3",
            Self::ScatteringPages => "No damage",
            Self::TrueVision => "No damage",
            Self::QiDisruption => "Drain 3 spirit",
            Self::ExpandingDomain => "No damage",
            Self::SinkholeSnare => "No damage",
            Self::SonicBurst => "2 AoE + stun",
            Self::VenomousLash => "1 + poison",
            Self::IronBodyStance => "No damage",
            Self::GoreCrush => "2 + knockback",
            Self::IntoxicatingMist => "No damage",
            Self::SproutingBarrier => "No damage",
            Self::TidalSurge => "1-2 + push",
            Self::BoneShatter => "3 + armor break",
            Self::AdaptiveShift => "No damage",
            Self::BerserkerFury => "Self -2 HP",
            Self::FlockAssault => "1 dmg ×3",
        }
    }

    pub fn attack_type(self) -> &'static str {
        match self {
            // Self-buffs (defensive)
            Self::RevealingDawn
            | Self::MortalResilience
            | Self::MaternalShield
            | Self::RigidStance
            | Self::ThresholdSeal
            | Self::SoaringEscape
            | Self::ImmovablePeak
            | Self::CloakingGuise
            | Self::FlexibleCounter
            | Self::CleansingLight
            | Self::IronBodyStance
            | Self::SproutingBarrier
            | Self::AdaptiveShift
            | Self::ExpandingDomain
            | Self::BerserkerFury => "Self-buff",

            // Support (ally-affecting)
            Self::ImperialCommand
            | Self::MagnifyingAura
            | Self::MercenaryPact => "Support",

            // Melee attacks
            Self::DevouringMaw
            | Self::SleightReversal
            | Self::HarvestReaping
            | Self::PreciseExecution
            | Self::CleavingCut
            | Self::SavageMaul
            | Self::ConsumingBite
            | Self::EchoStrike
            | Self::PursuingSteps
            | Self::CavalryCharge
            | Self::BlitzAssault
            | Self::GoreCrush => "Melee",

            // Projectile attacks
            Self::OverwhelmingForce
            | Self::PotentialBurst
            | Self::CrossroadsGambit
            | Self::GroundingWeight
            | Self::CrushingWheels
            | Self::NeedleStrike
            | Self::ParasiticSwarm
            | Self::TidalSurge
            | Self::VenomousLash
            | Self::WaningCurse => "Projectile",

            // Debuffs
            Self::ErosiveFlow
            | Self::DoubtSeed
            | Self::WitnessMark
            | Self::RootingGrasp
            | Self::ChasingChaff
            | Self::BindingOath
            | Self::EntanglingWeb
            | Self::PetrifyingGaze
            | Self::TrueVision
            | Self::QiDisruption
            | Self::ArtisanTrap
            | Self::SinkholeSnare => "Debuff",

            // Arcing projectiles
            Self::DownpourBarrage
            | Self::ArcingShot
            | Self::FlockAssault => "Arcing (2 turns)",

            Self::BoneShatter => "Arcing (1 turn)",

            // AoE
            Self::SpreadingWildfire
            | Self::ScatteringPages
            | Self::SonicBurst
            | Self::IntoxicatingMist => "AoE",
        }
    }

    /// Color for the attack type category in the UI.
    pub fn type_color(self) -> &'static str {
        match self.attack_type() {
            "Melee" | "Projectile" | "AoE" => "#ff6666",
            "Arcing (1 turn)" | "Arcing (2 turns)" => "#ff9944",
            "Self-buff" => "#6688ff",
            "Debuff" => "#ffcc44",
            "Support" => "#66cc66",
            _ => "#cccccc",
        }
    }
}

/// Player-facing abilities derived from collected radicals.
/// Each radical maps to exactly ONE ability (1:1 mapping).
/// These empower the player's attack when chosen from the radical picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerRadicalAbility {
    /// 火 fire — Bonus fire damage + burn
    FireStrike,
    /// 水 water — Slow target, restore focus
    TidalSurge,
    /// 力 strength — +50% base damage
    PowerStrike,
    /// 心 heart — Reveal all enemy intents
    Insight,
    /// 口 mouth — Heal for 50% of damage dealt
    Devour,
    /// 目 eye — Ignore armor, cannot miss
    TrueStrike,
    /// 手 hand — Attack doesn't consume action
    SwiftHands,
    /// 木 wood — Slow target, gain temp armor
    Entangle,
    /// 田 field — Bonus damage vs low HP targets
    Reap,
    /// 日 sun — Clear own debuffs + bonus damage
    SolarFlare,
    /// 月 moon — Apply poison DoT
    MoonVenom,
    /// 人 person — Gain temp HP after attack
    Resilience,
    /// 女 woman — Gain counter + thorn armor
    Guardian,
    /// 子 child — Mark target for bonus future damage
    GrowingStrike,
    /// 禾 grain — Extend combo streak by 2
    Harvest,
    /// 十 cross — Double damage or miss entirely
    Gamble,
    /// 金 metal — Armor-piercing: ignore all armor
    Shatter,
    /// 土 earth — Push target 2 tiles + stun
    Earthquake,
    /// 又 again — Attack hits twice
    DoubleStrike,
    /// 寸 inch — Execute: kill if target HP <= 25%
    Execution,
    /// 刀 knife — Bleed target + reduce max HP
    DeepCut,
    /// 言 speech — Confuse target 2 turns
    Intimidate,
    /// 足 foot — Dash to target + bonus damage
    Lunge,
    /// 糸 silk — Slow + bleed target
    Ensnare,
    /// 门 gate — Gain +3 armor after attack
    Fortify,
    /// 马 horse — Knockback target 3 tiles
    Charge,
    /// 鸟 bird — Gain +2 movement next turn
    Windstep,
    /// 雨 rain — AoE: splash 1 damage to adjacent enemies
    Downpour,
    /// 石 stone — Stun target 1 turn
    Concuss,
    /// 虫 insect — Poison + weaken target
    Infest,
    /// 贝 shell — Gain gold on kill (bonus loot)
    Plunder,
    /// 山 mountain — Gain +2 armor, +1 fortify
    Bulwark,
    /// 犬 beast — Bonus damage + self-heal 1
    Frenzy,
    /// 弓 bow — Attack at range (skip adjacency)
    Snipe,
    /// 食 food — Heal 3 HP after attack
    Nourish,
    /// 衣 clothing — Dodge next incoming attack
    Evade,
    /// 竹 bamboo — Counter next attack + thorn 2 turns
    Riposte,
    /// 走 walk — Free movement after attack
    HitAndRun,
    /// 车 vehicle — Push target + deal damage per tile pushed
    Bulldoze,
    /// 王 king — All attacks this turn deal +1 damage
    Inspire,
    /// 大 big — AoE: hit all adjacent enemies
    Cleave,
    /// 小 small — Ignore armor, +2 bonus damage
    PreciseStab,
    /// 工 craft — Place fire terrain around target
    Sabotage,
    /// 白 white — Restore 3 focus + clear debuffs
    Purify,
}

/// How a player radical ability is targeted when used independently (K key).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillType {
    /// No target needed — applied to self.
    SelfBuff,
    /// Needs an adjacent enemy.
    MeleeTarget,
    /// Needs an enemy within range.
    RangedTarget(i32),
    /// Needs a tile within range.
    GroundTarget(i32),
}

impl PlayerRadicalAbility {
    /// Categorise this ability for standalone skill usage.
    pub fn skill_type(self) -> SkillType {
        match self {
            // Self-buffs: no target needed
            Self::Insight
            | Self::SwiftHands
            | Self::Resilience
            | Self::Guardian
            | Self::Harvest
            | Self::Fortify
            | Self::Windstep
            | Self::Evade
            | Self::Riposte
            | Self::HitAndRun
            | Self::Inspire
            | Self::Purify
            | Self::Bulwark
            | Self::Nourish
            | Self::SolarFlare => SkillType::SelfBuff,

            // Melee-range: needs adjacent enemy
            Self::PowerStrike
            | Self::Devour
            | Self::TrueStrike
            | Self::DoubleStrike
            | Self::Execution
            | Self::DeepCut
            | Self::Lunge
            | Self::Charge
            | Self::Bulldoze
            | Self::PreciseStab
            | Self::Shatter
            | Self::Concuss
            | Self::Frenzy
            | Self::Reap
            | Self::Gamble
            | Self::Plunder => SkillType::MeleeTarget,

            // Ranged: needs enemy in range
            Self::FireStrike
            | Self::TidalSurge
            | Self::Entangle
            | Self::MoonVenom
            | Self::Intimidate
            | Self::GrowingStrike
            | Self::Ensnare
            | Self::Infest => SkillType::RangedTarget(3),
            Self::Snipe => SkillType::RangedTarget(5),

            // Ground-target: needs tile in range
            Self::Earthquake
            | Self::Downpour
            | Self::Sabotage
            | Self::Cleave => SkillType::GroundTarget(3),
        }
    }

    /// Short label for the skill type shown in the skill menu.
    pub fn skill_type_label(self) -> &'static str {
        match self.skill_type() {
            SkillType::SelfBuff => "Self",
            SkillType::MeleeTarget => "Melee",
            SkillType::RangedTarget(_) => "Ranged",
            SkillType::GroundTarget(_) => "Area",
        }
    }

    #[allow(dead_code)]
    pub fn radical(self) -> &'static str {
        match self {
            Self::FireStrike => "火",
            Self::TidalSurge => "水",
            Self::PowerStrike => "力",
            Self::Insight => "心",
            Self::Devour => "口",
            Self::TrueStrike => "目",
            Self::SwiftHands => "手",
            Self::Entangle => "木",
            Self::Reap => "田",
            Self::SolarFlare => "日",
            Self::MoonVenom => "月",
            Self::Resilience => "人",
            Self::Guardian => "女",
            Self::GrowingStrike => "子",
            Self::Harvest => "禾",
            Self::Gamble => "十",
            Self::Shatter => "金",
            Self::Earthquake => "土",
            Self::DoubleStrike => "又",
            Self::Execution => "寸",
            Self::DeepCut => "刀",
            Self::Intimidate => "言",
            Self::Lunge => "足",
            Self::Ensnare => "糸",
            Self::Fortify => "门",
            Self::Charge => "马",
            Self::Windstep => "鸟",
            Self::Downpour => "雨",
            Self::Concuss => "石",
            Self::Infest => "虫",
            Self::Plunder => "贝",
            Self::Bulwark => "山",
            Self::Frenzy => "犬",
            Self::Snipe => "弓",
            Self::Nourish => "食",
            Self::Evade => "衣",
            Self::Riposte => "竹",
            Self::HitAndRun => "走",
            Self::Bulldoze => "车",
            Self::Inspire => "王",
            Self::Cleave => "大",
            Self::PreciseStab => "小",
            Self::Sabotage => "工",
            Self::Purify => "白",
        }
    }

    pub fn from_radical(radical: &str) -> Option<Self> {
        match radical {
            "火" => Some(Self::FireStrike),
            "水" => Some(Self::TidalSurge),
            "力" => Some(Self::PowerStrike),
            "心" => Some(Self::Insight),
            "口" => Some(Self::Devour),
            "目" => Some(Self::TrueStrike),
            "手" => Some(Self::SwiftHands),
            "木" => Some(Self::Entangle),
            "田" => Some(Self::Reap),
            "日" => Some(Self::SolarFlare),
            "月" => Some(Self::MoonVenom),
            "人" => Some(Self::Resilience),
            "女" => Some(Self::Guardian),
            "子" => Some(Self::GrowingStrike),
            "禾" => Some(Self::Harvest),
            "十" => Some(Self::Gamble),
            "金" => Some(Self::Shatter),
            "土" => Some(Self::Earthquake),
            "又" => Some(Self::DoubleStrike),
            "寸" => Some(Self::Execution),
            "刀" => Some(Self::DeepCut),
            "言" => Some(Self::Intimidate),
            "足" => Some(Self::Lunge),
            "糸" => Some(Self::Ensnare),
            "门" => Some(Self::Fortify),
            "马" => Some(Self::Charge),
            "鸟" => Some(Self::Windstep),
            "雨" => Some(Self::Downpour),
            "石" => Some(Self::Concuss),
            "虫" => Some(Self::Infest),
            "贝" => Some(Self::Plunder),
            "山" => Some(Self::Bulwark),
            "犬" => Some(Self::Frenzy),
            "弓" => Some(Self::Snipe),
            "食" => Some(Self::Nourish),
            "衣" => Some(Self::Evade),
            "竹" => Some(Self::Riposte),
            "走" => Some(Self::HitAndRun),
            "车" => Some(Self::Bulldoze),
            "王" => Some(Self::Inspire),
            "大" => Some(Self::Cleave),
            "小" => Some(Self::PreciseStab),
            "工" => Some(Self::Sabotage),
            "白" => Some(Self::Purify),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::FireStrike => "\u{1F525} Fire Strike",
            Self::TidalSurge => "\u{1F4A7} Tidal Surge",
            Self::PowerStrike => "\u{1F4AA} Power Strike",
            Self::Insight => "\u{1F49C} Insight",
            Self::Devour => "\u{1F444} Devour",
            Self::TrueStrike => "\u{1F441} True Strike",
            Self::SwiftHands => "\u{270B} Swift Hands",
            Self::Entangle => "\u{1F33F} Entangle",
            Self::Reap => "\u{1F33E} Reap",
            Self::SolarFlare => "\u{2600} Solar Flare",
            Self::MoonVenom => "\u{1F319} Moon Venom",
            Self::Resilience => "\u{1F464} Resilience",
            Self::Guardian => "\u{1F495} Guardian",
            Self::GrowingStrike => "\u{26A1} Growing Strike",
            Self::Harvest => "\u{1F33E} Harvest",
            Self::Gamble => "\u{2715} Gamble",
            Self::Shatter => "\u{1F6E1} Shatter",
            Self::Earthquake => "\u{1F30D} Earthquake",
            Self::DoubleStrike => "\u{1F504} Double Strike",
            Self::Execution => "\u{1F4CF} Execution",
            Self::DeepCut => "\u{1F5E1} Deep Cut",
            Self::Intimidate => "\u{1F4DC} Intimidate",
            Self::Lunge => "\u{1F463} Lunge",
            Self::Ensnare => "\u{1F578} Ensnare",
            Self::Fortify => "\u{1F6AA} Fortify",
            Self::Charge => "\u{1F434} Charge",
            Self::Windstep => "\u{1F426} Windstep",
            Self::Downpour => "\u{1F327} Downpour",
            Self::Concuss => "\u{1FAA8} Concuss",
            Self::Infest => "\u{1F41B} Infest",
            Self::Plunder => "\u{1F41A} Plunder",
            Self::Bulwark => "\u{26F0} Bulwark",
            Self::Frenzy => "\u{1F415} Frenzy",
            Self::Snipe => "\u{1F3F9} Snipe",
            Self::Nourish => "\u{1F356} Nourish",
            Self::Evade => "\u{1F458} Evade",
            Self::Riposte => "\u{1F38B} Riposte",
            Self::HitAndRun => "\u{1F4A8} Hit & Run",
            Self::Bulldoze => "\u{1F6DE} Bulldoze",
            Self::Inspire => "\u{1F451} Inspire",
            Self::Cleave => "\u{1F53A} Cleave",
            Self::PreciseStab => "\u{1F53B} Precise Stab",
            Self::Sabotage => "\u{2699} Sabotage",
            Self::Purify => "\u{2728} Purify",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::FireStrike => "+2 fire damage, Burn 1 dmg for 2 turns",
            Self::TidalSurge => "Slow target 2 turns, restore 2 Focus",
            Self::PowerStrike => "+50% base damage on this attack",
            Self::Insight => "Reveal all enemy intents for 3 turns",
            Self::Devour => "Heal for 50% of damage dealt",
            Self::TrueStrike => "Ignores armor, cannot miss",
            Self::SwiftHands => "Attack doesn't consume your action",
            Self::Entangle => "Slow target 3 turns, +1 temp armor",
            Self::Reap => "+3 bonus damage if target HP < 40%",
            Self::SolarFlare => "Clear your debuffs, +2 bonus damage",
            Self::MoonVenom => "Poison: 2 dmg for 3 turns",
            Self::Resilience => "Gain 2 temporary HP after attack",
            Self::Guardian => "Gain counter-attack + thorn armor 2 turns",
            Self::GrowingStrike => "Mark target: +2 damage from all sources",
            Self::Harvest => "Extend combo streak by +2",
            Self::Gamble => "50/50: triple damage or miss entirely",
            Self::Shatter => "Destroy all target armor, +1 damage",
            Self::Earthquake => "Push target 2 tiles + stun 1 turn",
            Self::DoubleStrike => "Attack hits a second time for base damage",
            Self::Execution => "Instant kill if target HP <= 25%",
            Self::DeepCut => "Bleed 2 dmg for 2 turns, -1 max HP",
            Self::Intimidate => "Confuse target for 2 turns",
            Self::Lunge => "+2 damage, gain free movement after",
            Self::Ensnare => "Slow 3 turns + Bleed 1 for 2 turns",
            Self::Fortify => "Gain +3 armor after attacking",
            Self::Charge => "Knockback target 3 tiles",
            Self::Windstep => "Gain +2 stored movement",
            Self::Downpour => "Splash 1 damage to all adjacent enemies",
            Self::Concuss => "Stun target for 1 turn",
            Self::Infest => "Poison 1 for 3 turns + Slow 1 turn",
            Self::Plunder => "Bonus gold if this kills the target",
            Self::Bulwark => "Gain +2 armor and +1 fortify",
            Self::Frenzy => "+1 bonus damage, heal 1 HP",
            Self::Snipe => "Can target non-adjacent enemies in LoS",
            Self::Nourish => "Heal 3 HP after attacking",
            Self::Evade => "Dodge the next incoming attack",
            Self::Riposte => "Counter next attack + thorn armor 2 turns",
            Self::HitAndRun => "Gain free movement after attack",
            Self::Bulldoze => "Push target 2 tiles, 1 dmg per tile",
            Self::Inspire => "+1 damage to all attacks this turn",
            Self::Cleave => "Hit all adjacent enemies for base damage",
            Self::PreciseStab => "Ignore armor, +2 bonus damage",
            Self::Sabotage => "Create fire terrain around target",
            Self::Purify => "Restore 3 Focus, clear your debuffs",
        }
    }
}

#[derive(Clone)]
pub struct Enemy {
    pub x: i32,
    pub y: i32,
    pub hanzi: &'static str,
    pub pinyin: &'static str,
    pub meaning: &'static str,
    pub hp: i32,
    pub max_hp: i32,
    pub damage: i32,
    /// Set when the enemy is alerted (player in same room / nearby)
    pub alert: bool,
    /// Boss enemies are tougher and give better rewards
    pub is_boss: bool,
    /// Elite multi-character enemies
    pub is_elite: bool,
    /// Gold dropped on defeat
    pub gold_value: i32,
    /// Stunned: skip next turn
    pub stunned: bool,
    /// Active status effects
    pub statuses: Vec<StatusInstance>,
    /// Floor-specific boss mechanics
    pub boss_kind: Option<BossKind>,
    /// Tracks one-time boss phase mechanics
    pub phase_triggered: bool,
    /// PirateCaptain summon cadence
    pub summon_cooldown: u8,
    /// RogueAICore resistance remembers the last system hacked
    pub resisted_spell: Option<&'static str>,
    /// Elite compounds are dismantled syllable by syllable
    pub elite_chain: usize,
    /// Defensive components (shields) that must be broken first
    pub components: Vec<&'static str>,
    pub ai: AiBehavior,
    /// Temporary armor from radical action (reduces next player hit)
    pub radical_armor: i32,
    /// Will dodge next attack (ShadowStep)
    pub radical_dodge: bool,
    /// Next attack multiplier (Multiply: hits twice)
    pub radical_multiply: bool,
}

impl Enemy {
    pub fn radical_actions(&self) -> Vec<RadicalAction> {
        self.components
            .iter()
            .filter_map(|c| RadicalAction::from_radical(c))
            .collect()
    }

    pub fn from_vocab(entry: &'static VocabEntry, x: i32, y: i32, floor: i32) -> Self {
        let is_elite = crate::vocab::is_elite(entry);
        let hp = if is_elite { 4 + floor } else { 2 + floor / 2 };
        let damage = if is_elite {
            2 + floor / 2
        } else {
            1 + floor / 3
        };
        let gold = if is_elite { 8 + floor * 2 } else { 3 + floor };

        let components = get_components(entry.hanzi);

        let ai = if is_elite {
            AiBehavior::Chase
        } else {
            let seed = (x.wrapping_mul(31) ^ y.wrapping_mul(17) ^ floor.wrapping_mul(7)) as u32;
            match seed % 16 {
                0..=6 => AiBehavior::Chase,
                7..=8 => AiBehavior::Ambush,
                9..=10 => AiBehavior::Retreat,
                11..=12 => AiBehavior::Sentinel,
                13..=14 => AiBehavior::Kiter,
                _ => AiBehavior::Pack,
            }
        };

        Self {
            x,
            y,
            hanzi: entry.hanzi,
            pinyin: entry.pinyin,
            meaning: entry.meaning,
            hp,
            max_hp: hp,
            damage,
            alert: false,
            is_boss: false,
            is_elite,
            gold_value: gold,
            stunned: false,
            statuses: Vec::new(),
            boss_kind: None,
            phase_triggered: false,
            summon_cooldown: 0,
            resisted_spell: None,
            elite_chain: 0,
            components,
            ai,
            radical_armor: 0,
            radical_dodge: false,
            radical_multiply: false,
        }
    }

    pub fn boss_from_vocab(entry: &'static VocabEntry, x: i32, y: i32, floor: i32) -> Self {
        let boss_kind = BossKind::for_floor(floor);
        let (hp, damage, gold, cooldown) = match boss_kind {
            Some(BossKind::PirateCaptain) => (16 + floor, 3 + floor / 3, 40 + floor * 4, 1),
            Some(BossKind::HiveQueen) => (14 + floor, 3 + floor / 3, 45 + floor * 4, 0),
            Some(BossKind::RogueAICore) => (18 + floor, 4 + floor / 3, 50 + floor * 4, 0),
            Some(BossKind::VoidEntity) => (22 + floor, 4 + floor / 3, 55 + floor * 4, 2),
            Some(BossKind::AncientGuardian) => (20 + floor, 5 + floor / 3, 65 + floor * 4, 0),
            Some(BossKind::DriftLeviathan) => (24 + floor, 5 + floor / 3, 80 + floor * 4, 0),
            None => (8 + floor, 2 + floor / 2, 20 + floor * 3, 0),
        };
        Self {
            x,
            y,
            hanzi: entry.hanzi,
            pinyin: entry.pinyin,
            meaning: entry.meaning,
            hp,
            max_hp: hp,
            damage,
            alert: true, // bosses are always alert
            is_boss: true,
            is_elite: false,
            gold_value: gold,
            stunned: false,
            statuses: Vec::new(),
            boss_kind,
            phase_triggered: false,
            summon_cooldown: cooldown,
            resisted_spell: None,
            elite_chain: 0,
            components: Vec::new(),
            ai: AiBehavior::Chase,
            radical_armor: 0,
            radical_dodge: false,
            radical_multiply: false,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// Simple chase AI: move one step toward (tx, ty) if possible.
    /// Returns desired (nx, ny). Caller checks walkability & occupancy.
    pub fn step_toward(&self, tx: i32, ty: i32) -> (i32, i32) {
        let dx = (tx - self.x).signum();
        let dy = (ty - self.y).signum();
        // Prefer axis with larger distance
        if (tx - self.x).abs() >= (ty - self.y).abs() {
            if dx != 0 {
                return (self.x + dx, self.y);
            }
            (self.x, self.y + dy)
        } else {
            if dy != 0 {
                return (self.x, self.y + dy);
            }
            (self.x + dx, self.y)
        }
    }

    pub fn step_retreat(&self, tx: i32, ty: i32) -> (i32, i32) {
        let dx = (self.x - tx).signum();
        let dy = (self.y - ty).signum();
        if (tx - self.x).abs() >= (ty - self.y).abs() {
            if dx != 0 {
                return (self.x + dx, self.y);
            }
            (self.x, self.y + dy)
        } else {
            if dy != 0 {
                return (self.x, self.y + dy);
            }
            (self.x + dx, self.y)
        }
    }

    pub fn ai_step(&self, tx: i32, ty: i32, nearby_allies: usize) -> (i32, i32) {
        let dist = (tx - self.x).abs() + (ty - self.y).abs();
        match self.ai {
            AiBehavior::Chase => self.step_toward(tx, ty),
            AiBehavior::Retreat => {
                if dist <= 2 {
                    self.step_toward(tx, ty)
                } else {
                    self.step_retreat(tx, ty)
                }
            }
            AiBehavior::Ambush => {
                if dist <= 3 {
                    self.step_toward(tx, ty)
                } else {
                    (self.x, self.y)
                }
            }
            AiBehavior::Sentinel => {
                if dist <= 1 {
                    self.step_toward(tx, ty)
                } else {
                    (self.x, self.y)
                }
            }
            AiBehavior::Kiter => {
                if dist <= 2 {
                    self.step_retreat(tx, ty)
                } else if dist >= 5 {
                    self.step_toward(tx, ty)
                } else {
                    (self.x, self.y)
                }
            }
            AiBehavior::Pack => {
                if nearby_allies >= 2 || dist <= 1 {
                    self.step_toward(tx, ty)
                } else {
                    (self.x, self.y)
                }
            }
        }
    }

    pub fn boss_trait_text(&self) -> Option<String> {
        match self.boss_kind {
            Some(BossKind::PirateCaptain) => {
                Some("Deploys shield generators when cornered".to_string())
            }
            Some(BossKind::HiveQueen) => Some(if self.phase_triggered {
                "Sentence duel spent".to_string()
            } else {
                "Triggers a sentence duel at half HP".to_string()
            }),
            Some(BossKind::RogueAICore) => Some(match self.resisted_spell {
                Some(school) => format!("Resists last system: {}", school),
                None => "Adapts to the last system you hacked".to_string(),
            }),
            Some(BossKind::VoidEntity) => {
                Some("Warps reality — answer carefully!".to_string())
            }
            Some(BossKind::AncientGuardian) => Some(if self.phase_triggered {
                "Glyph trial spent".to_string()
            } else {
                "Triggers a glyph trial at half HP".to_string()
            }),
            Some(BossKind::DriftLeviathan) => {
                Some("Absorbs a radical on each wrong answer".to_string())
            }
            None => None,
        }
    }

    pub fn elite_phase_count(&self) -> usize {
        crate::vocab::pinyin_syllables(self.pinyin).len().max(1)
    }

    pub fn elite_expected_syllable(&self) -> Option<&str> {
        if !self.is_elite {
            return None;
        }
        let syllables = crate::vocab::pinyin_syllables(self.pinyin);
        let idx = self.elite_chain.min(syllables.len().saturating_sub(1));
        syllables.get(idx).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::{AiBehavior, BossKind, Enemy};
    use crate::vocab::VOCAB;

    fn friend_entry() -> &'static crate::vocab::VocabEntry {
        VOCAB.iter().find(|entry| entry.hanzi == "朋友").unwrap()
    }

    #[test]
    fn boss_kind_matches_key_floors() {
        assert_eq!(BossKind::for_floor(5), Some(BossKind::PirateCaptain));
        assert_eq!(BossKind::for_floor(10), Some(BossKind::HiveQueen));
        assert_eq!(BossKind::for_floor(15), Some(BossKind::RogueAICore));
        assert_eq!(BossKind::for_floor(20), Some(BossKind::VoidEntity));
        assert_eq!(BossKind::for_floor(25), Some(BossKind::AncientGuardian));
        assert_eq!(BossKind::for_floor(30), Some(BossKind::DriftLeviathan));
        assert_eq!(BossKind::for_floor(35), None);
    }

    #[test]
    fn elite_expected_syllable_tracks_chain_progress() {
        let mut enemy = Enemy::from_vocab(friend_entry(), 0, 0, 6);
        enemy.elite_chain = 1;

        assert_eq!(enemy.elite_expected_syllable(), Some("you3"));
    }

    #[test]
    fn ai_behavior_dispatch_covers_all_variants() {
        let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);

        enemy.ai = AiBehavior::Chase;
        let _ = enemy.ai_step(10, 10, 0);

        enemy.ai = AiBehavior::Retreat;
        let _ = enemy.ai_step(10, 10, 0);

        enemy.ai = AiBehavior::Ambush;
        let _ = enemy.ai_step(10, 10, 0);

        enemy.ai = AiBehavior::Sentinel;
        let _ = enemy.ai_step(10, 10, 0);

        enemy.ai = AiBehavior::Kiter;
        let _ = enemy.ai_step(10, 10, 0);

        enemy.ai = AiBehavior::Pack;
        let _ = enemy.ai_step(10, 10, 0);
    }

    #[test]
    fn sentinel_holds_position_when_far() {
        let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
        enemy.ai = AiBehavior::Sentinel;
        let (nx, ny) = enemy.ai_step(10, 10, 0);
        assert_eq!((nx, ny), (5, 5));
    }

    #[test]
    fn sentinel_chases_when_adjacent() {
        let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
        enemy.ai = AiBehavior::Sentinel;
        let (nx, ny) = enemy.ai_step(6, 5, 0);
        assert_eq!((nx, ny), (6, 5));
    }

    #[test]
    fn kiter_retreats_when_close() {
        let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
        enemy.ai = AiBehavior::Kiter;
        let (nx, ny) = enemy.ai_step(6, 5, 0);
        assert_ne!((nx, ny), (6, 5));
        assert!((nx - 5i32).abs() + (ny - 5i32).abs() <= 1);
    }

    #[test]
    fn kiter_advances_when_far() {
        let mut enemy = Enemy::from_vocab(friend_entry(), 0, 0, 1);
        enemy.ai = AiBehavior::Kiter;
        let (nx, ny) = enemy.ai_step(10, 10, 0);
        assert!(nx > 0 || ny > 0);
    }

    #[test]
    fn kiter_holds_at_medium_range() {
        let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
        enemy.ai = AiBehavior::Kiter;
        // dist = 3+1 = 4, in the hold zone (3..=4)
        let (nx, ny) = enemy.ai_step(8, 6, 0);
        assert_eq!((nx, ny), (5, 5));
    }

    #[test]
    fn pack_holds_without_allies() {
        let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
        enemy.ai = AiBehavior::Pack;
        let (nx, ny) = enemy.ai_step(10, 10, 0);
        assert_eq!((nx, ny), (5, 5));
    }

    #[test]
    fn pack_chases_with_enough_allies() {
        let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
        enemy.ai = AiBehavior::Pack;
        let (nx, ny) = enemy.ai_step(10, 10, 2);
        assert_ne!((nx, ny), (5, 5));
    }

    #[test]
    fn pack_chases_when_adjacent_even_alone() {
        let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
        enemy.ai = AiBehavior::Pack;
        let (nx, ny) = enemy.ai_step(6, 5, 0);
        assert_eq!((nx, ny), (6, 5));
    }

    #[test]
    fn radical_action_from_known_radicals() {
        use super::RadicalAction;
        assert_eq!(
            RadicalAction::from_radical("火"),
            Some(RadicalAction::SpreadingWildfire)
        );
        assert_eq!(
            RadicalAction::from_radical("水"),
            Some(RadicalAction::ErosiveFlow)
        );
        assert_eq!(
            RadicalAction::from_radical("心"),
            Some(RadicalAction::DoubtSeed)
        );
        assert_eq!(
            RadicalAction::from_radical("又"),
            Some(RadicalAction::EchoStrike)
        );
        assert_eq!(RadicalAction::from_radical("xyz"), None);
    }

    #[test]
    fn enemy_radical_actions_from_components() {
        use crate::vocab::VOCAB;
        // 好 has components 女 and 子
        let entry = VOCAB.iter().find(|e| e.hanzi == "好").unwrap();
        let enemy = Enemy::from_vocab(entry, 0, 0, 1);
        let actions = enemy.radical_actions();
        use super::RadicalAction;
        assert!(actions.contains(&RadicalAction::MaternalShield)); // 女
        assert!(actions.contains(&RadicalAction::PotentialBurst)); // 子
        assert_eq!(actions.len(), 2); // 1 per radical, 好 has 2 radicals
    }

    #[test]
    fn enemy_with_generated_components_has_actions() {
        use crate::vocab::VOCAB;
        // Most HSK characters should have at least some components now
        let mut chars_with_actions = 0;
        let total = VOCAB.iter().take(100).count();
        for entry in VOCAB.iter().take(100) {
            let enemy = Enemy::from_vocab(entry, 0, 0, 1);
            if !enemy.radical_actions().is_empty() {
                chars_with_actions += 1;
            }
        }
        // At least 60% of chars should have radical actions with our expanded system
        assert!(
            chars_with_actions > total / 2,
            "Only {}/{} had actions",
            chars_with_actions,
            total
        );
    }
}
