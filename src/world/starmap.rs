//! Star system navigation and sector map generation.
//!
//! Provides FTL-like star map navigation with branching paths through
//! sectors of increasing HSK difficulty. Each star system is named with
//! Chinese characters matching the sector's HSK level.
#![allow(dead_code)]

use super::LocationType;

// ---------------------------------------------------------------------------
// Core data structures
// ---------------------------------------------------------------------------

/// A single star system on the sector map.
#[derive(Clone, Debug)]
pub struct StarSystem {
    /// Unique id within the sector.
    pub id: usize,
    /// English display name (e.g. "Sky Star").
    pub name: &'static str,
    /// Chinese name for the system (e.g. "天星").
    pub chinese_name: &'static str,
    /// Description of this system/location.
    pub description: &'static str,
    /// Horizontal position on the sector map (0.0–1.0).
    pub x: f64,
    /// Vertical position on the sector map (0.0–1.0).
    pub y: f64,
    /// What kind of location is here.
    pub location_type: LocationType,
    /// Optional hazard that affects this system.
    pub hazard: Option<SystemHazard>,
    /// Whether the player has visited this system.
    pub visited: bool,
    /// Whether this system has a shop.
    pub has_shop: bool,
    /// Whether this system has a fuel station.
    pub has_fuel: bool,
    /// Whether this system can repair hull damage.
    pub has_repair: bool,
    /// Whether this system has a medbay to heal crew.
    pub has_medbay: bool,
    /// Whether this system has a quest available.
    pub quest_giver: bool,
    /// Whether this system is hidden (requires sensor upgrade).
    pub hidden: bool,
    /// Whether this system has a warp gate.
    pub warp_gate: bool,
    /// Optional index into an events list.
    pub event_id: Option<usize>,
    /// IDs of directly connected systems (jump targets).
    pub connections: Vec<usize>,
    /// Difficulty rating 1–6, maps to HSK level.
    pub difficulty: u8,
}

/// A sector is one region of the galaxy containing interconnected star systems.
#[derive(Clone, Debug)]
pub struct Sector {
    /// Sector index (0-based).
    pub id: usize,
    /// Display name for the sector.
    pub name: &'static str,
    /// Description of this sector.
    pub description: &'static str,
    /// All star systems in this sector.
    pub systems: Vec<StarSystem>,
    /// Index of the starting system.
    pub start_system: usize,
    /// Index of the boss system.
    pub boss_system: usize,
    /// Index of the exit system (leads to next sector).
    pub exit_system: usize,
    /// HSK difficulty level for this sector (1–6).
    pub hsk_level: u8,
}

/// The overall galaxy map tracking the player's progression through sectors.
#[derive(Clone, Debug)]
pub struct SectorMap {
    /// All sectors in the galaxy.
    pub sectors: Vec<Sector>,
    /// Index of the sector the player is currently in.
    pub current_sector: usize,
    /// Index of the system the player is currently at (within the current sector).
    pub current_system: usize,
}

// ---------------------------------------------------------------------------
// Simple LCG pseudo-random number generator (no external crate)
// ---------------------------------------------------------------------------

/// Advance a linear congruential generator and return the next value.
fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    *state
}

/// Return a random `usize` in `[0, max)`.
fn lcg_range(state: &mut u32, max: usize) -> usize {
    (lcg_next(state) as usize) % max
}

/// Return a random `f64` in `[0.0, 1.0)`.
fn lcg_f64(state: &mut u32) -> f64 {
    (lcg_next(state) as f64) / (u32::MAX as f64 + 1.0)
}

// ---------------------------------------------------------------------------
// System hazards
// ---------------------------------------------------------------------------

/// Environmental hazards that affect star systems.
#[derive(Clone, Debug, PartialEq)]
pub enum SystemHazard {
    RadiationBelt,
    AsteroidField,
    IonStorm,
    PirateTerritory,
    GravityWell,
    Nebula,
    SolarFlare,
    MineField,
    DarkMatter,
    VoidRift,
}

impl SystemHazard {
    pub fn name(&self) -> &'static str {
        match self {
            SystemHazard::RadiationBelt => "Radiation Belt",
            SystemHazard::AsteroidField => "Asteroid Field",
            SystemHazard::IonStorm => "Ion Storm",
            SystemHazard::PirateTerritory => "Pirate Territory",
            SystemHazard::GravityWell => "Gravity Well",
            SystemHazard::Nebula => "Nebula",
            SystemHazard::SolarFlare => "Solar Flare",
            SystemHazard::MineField => "Mine Field",
            SystemHazard::DarkMatter => "Dark Matter",
            SystemHazard::VoidRift => "Void Rift",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SystemHazard::RadiationBelt => "☢️",
            SystemHazard::AsteroidField => "🌑",
            SystemHazard::IonStorm => "⚡",
            SystemHazard::PirateTerritory => "☠️",
            SystemHazard::GravityWell => "🌀",
            SystemHazard::Nebula => "🌫️",
            SystemHazard::SolarFlare => "☀️",
            SystemHazard::MineField => "💣",
            SystemHazard::DarkMatter => "🕳️",
            SystemHazard::VoidRift => "🌌",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SystemHazard::RadiationBelt => "Intense radiation damages hull on entry.",
            SystemHazard::AsteroidField => "Dense asteroids risk hull damage.",
            SystemHazard::IonStorm => "Electromagnetic storm disables shields temporarily.",
            SystemHazard::PirateTerritory => "High risk of hostile encounters.",
            SystemHazard::GravityWell => "Strong gravity increases fuel consumption.",
            SystemHazard::Nebula => "Dense gas clouds reduce sensor range.",
            SystemHazard::SolarFlare => "Periodic stellar flares cause damage.",
            SystemHazard::MineField => "Leftover explosives damage ships on entry.",
            SystemHazard::DarkMatter => "Unpredictable spatial anomalies.",
            SystemHazard::VoidRift => "Reality tears with mysterious effects.",
        }
    }

    pub fn fuel_modifier(&self) -> i32 {
        match self {
            SystemHazard::GravityWell => 3,
            SystemHazard::Nebula => 2,
            SystemHazard::IonStorm => 1,
            _ => 0,
        }
    }

    pub fn hull_damage(&self) -> i32 {
        match self {
            SystemHazard::RadiationBelt => 8,
            SystemHazard::MineField => 10,
            SystemHazard::AsteroidField => 5,
            SystemHazard::SolarFlare => 6,
            SystemHazard::VoidRift => 7,
            _ => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Star system name pools — Chinese-themed, grouped by HSK level
// ---------------------------------------------------------------------------

/// (English name, Chinese name) pairs per HSK level.
/// Each pool contains at least 15 entries; 6 pools ≥ 90 total (well over 60).

const HSK1_NAMES: &[(&str, &str)] = &[
    ("Sky Star", "天星"),
    ("Sun Harbor", "日港"),
    ("Moon Station", "月站"),
    ("Fire Gate", "火口"),
    ("Water Way", "水路"),
    ("Great Gate", "大门"),
    ("Human Realm", "人间"),
    ("Mountain Ridge", "山岭"),
    ("Gold Field", "金野"),
    ("Earth Base", "土基"),
    ("Tree Haven", "木园"),
    ("Stone Port", "石港"),
    ("Rain Dock", "雨坞"),
    ("Wind Reach", "风域"),
    ("Star Bridge", "星桥"),
    ("Cloud Gate", "云门"),
    ("River Post", "河站"),
    ("Big Sky", "大天"),
    ("Small Moon", "小月"),
    ("Upper Gate", "上门"),
    ("Lower Station", "下站"),
    ("Middle Way", "中路"),
    ("East Wind", "东风"),
    ("West Harbor", "西港"),
    ("South Gate", "南门"),
    ("North Star", "北星"),
    ("White Cloud", "白云"),
    ("Black Stone", "黑石"),
    ("Red Sun", "红日"),
    ("Green Tree", "绿木"),
    ("Long River", "长河"),
    ("One Star", "一星"),
    ("Two Moons", "二月"),
    ("Three Gates", "三门"),
    ("Four Winds", "四风"),
    ("Five Mountains", "五山"),
    ("Six Waters", "六水"),
    ("Seven Stars", "七星"),
    ("Eight Skies", "八天"),
    ("Nine Clouds", "九云"),
    ("Ten Suns", "十日"),
    ("Light Gate", "光门"),
    ("Dark Harbor", "暗港"),
    ("New Moon", "新月"),
    ("Old Stone", "老石"),
];

const HSK2_NAMES: &[(&str, &str)] = &[
    ("Dragon Eye", "龙眼"),
    ("Phoenix City", "凤城"),
    ("Jade Stone", "玉石"),
    ("Cloud Sea", "云海"),
    ("Iron Forge", "铁炉"),
    ("Snow Peak", "雪峰"),
    ("Red Lantern", "红灯"),
    ("Blue Depths", "蓝深"),
    ("Silver Stream", "银溪"),
    ("Flower Garden", "花园"),
    ("Tiger Claw", "虎爪"),
    ("Horse Plains", "马原"),
    ("Fish Shoal", "鱼滩"),
    ("Bird Nest", "鸟巢"),
    ("Thunder Fort", "雷堡"),
    ("Frost Ring", "霜环"),
    ("Wolf Den", "狼窝"),
    ("Bear Mountain", "熊山"),
    ("Snake Valley", "蛇谷"),
    ("Deer Forest", "鹿林"),
    ("Ox Station", "牛站"),
    ("Elephant Gate", "象门"),
    ("Monkey Peak", "猴峰"),
    ("Rabbit Moon", "兔月"),
    ("Rat Harbor", "鼠港"),
    ("Pig Village", "猪村"),
    ("Rooster Point", "鸡点"),
    ("Dog Watch", "狗哨"),
    ("Sheep Fields", "羊野"),
    ("Golden Dragon", "金龙"),
    ("Silver Phoenix", "银凤"),
    ("Iron Tiger", "铁虎"),
    ("Jade Rabbit", "玉兔"),
    ("Crystal Snake", "晶蛇"),
    ("Pearl Fish", "珠鱼"),
    ("Thunder Horse", "雷马"),
    ("Wind Eagle", "风鹰"),
    ("Rain Crane", "雨鹤"),
    ("Storm Wolf", "暴狼"),
    ("Ice Bear", "冰熊"),
    ("Fire Ox", "火牛"),
    ("Stone Elephant", "石象"),
    ("Forest Deer", "林鹿"),
    ("Ocean Whale", "海鲸"),
    ("Mountain Lion", "山狮"),
];

const HSK3_NAMES: &[(&str, &str)] = &[
    ("Shadow Reef", "影礁"),
    ("Spirit Spring", "灵泉"),
    ("Emerald Nebula", "翠云"),
    ("Sword Edge", "剑锋"),
    ("Mirror Lake", "镜湖"),
    ("Silk Road", "丝路"),
    ("Bamboo Grove", "竹林"),
    ("Copper Tower", "铜塔"),
    ("Pearl Bay", "珠湾"),
    ("Ink Well", "墨池"),
    ("Crane Harbor", "鹤港"),
    ("Plum Blossom", "梅花"),
    ("Pine Summit", "松顶"),
    ("Lotus Pond", "莲塘"),
    ("Coral Reef", "珊瑚"),
    ("Amber Gate", "琥门"),
    ("Shield Wall", "盾墙"),
    ("Jade Glass", "翡璃"),
    ("Emerald Light", "翠光"),
    ("Crystal Palace", "晶宫"),
    ("Phantom Mist", "幻雾"),
    ("Shadow Dance", "影舞"),
    ("Spirit Blade", "灵刃"),
    ("Dream Harbor", "梦港"),
    ("Mystic Gate", "秘门"),
    ("Ancient Ruins", "古迹"),
    ("Sacred Temple", "圣殿"),
    ("Hidden Valley", "隐谷"),
    ("Secret Garden", "密园"),
    ("Forbidden Peak", "禁峰"),
    ("Lost City", "失城"),
    ("Forgotten Road", "忘路"),
    ("Silent Watch", "寂哨"),
    ("Peaceful Bay", "宁湾"),
    ("Bright Star", "明星"),
    ("Dark Moon", "暗月"),
    ("Cold Wind", "寒风"),
    ("Warm Harbor", "暖港"),
    ("Deep Ocean", "深海"),
    ("High Mountain", "高山"),
    ("Wide River", "广河"),
    ("Narrow Gate", "窄门"),
    ("Sharp Blade", "利刃"),
    ("Soft Cloud", "柔云"),
    ("Hard Stone", "硬石"),
];

const HSK4_NAMES: &[(&str, &str)] = &[
    ("Celestial Forge", "天锻"),
    ("Void Passage", "虚道"),
    ("Obsidian Spire", "曜塔"),
    ("Crimson Nebula", "绯云"),
    ("Moonlit Marsh", "月泽"),
    ("Ancient Beacon", "古灯"),
    ("Frozen Archive", "冰典"),
    ("Starfall Basin", "陨盆"),
    ("Whirlpool Gate", "漩门"),
    ("Tempest Eye", "暴眼"),
    ("Lantern Drift", "灯漂"),
    ("Oracle Shrine", "卜殿"),
    ("Eclipse Point", "蚀点"),
    ("Dust Haven", "尘港"),
    ("Sapphire Ring", "蓝环"),
    ("Opal Crossing", "瑙渡"),
    ("Abyss Gate", "渊门"),
    ("Dark Void", "暝虚"),
    ("Radiant Star", "曜星"),
    ("Shadow Deep", "幽深"),
    ("Nether Realm", "冥界"),
    ("Mystic Void", "玄虚"),
    ("Profound Ocean", "奥海"),
    ("Auspicious Cloud", "瑞云"),
    ("Fortunate Harbor", "祥港"),
    ("Blessed Gate", "福门"),
    ("Sacred Mountain", "圣山"),
    ("Divine Valley", "神谷"),
    ("Immortal Peak", "仙峰"),
    ("Eternal River", "永河"),
    ("Endless Sky", "无天"),
    ("Boundless Sea", "极海"),
    ("Supreme Tower", "至塔"),
    ("Ultimate Gate", "终门"),
    ("Perfect Circle", "全环"),
    ("True Path", "真道"),
    ("Pure Light", "纯光"),
    ("Clear Mind", "明心"),
    ("Calm Spirit", "静灵"),
    ("Quiet Harbor", "寂港"),
    ("Still Waters", "止水"),
    ("Frozen Time", "凝时"),
    ("Burning Sky", "炽天"),
    ("Falling Stars", "陨星"),
    ("Rising Moon", "升月"),
    ("Setting Sun", "沉日"),
];

const HSK5_NAMES: &[(&str, &str)] = &[
    ("Abyssal Rift", "渊裂"),
    ("Sovereign Throne", "皇座"),
    ("Labyrinth Core", "迷核"),
    ("Phantom Relay", "幻驿"),
    ("Radiant Crucible", "辉炉"),
    ("Twilight Bastion", "暮堡"),
    ("Ethereal Nexus", "灵枢"),
    ("Zenith Spire", "巅塔"),
    ("Cascade Veil", "瀑幕"),
    ("Prism Array", "棱阵"),
    ("Obsidian Maw", "曜口"),
    ("Tempest Crucible", "飓炉"),
    ("Astral Conduit", "星管"),
    ("Gilded Sanctum", "金殿"),
    ("Warden Outpost", "卫哨"),
    ("Imperial Palace", "帝宫"),
    ("Emperor's Gate", "皇门"),
    ("Royal Harbor", "王港"),
    ("Noble Peak", "贵峰"),
    ("Sacred Shrine", "圣祠"),
    ("Sage's Tower", "贤塔"),
    ("Scholar's Haven", "儒港"),
    ("Spirit Soul", "魂魄"),
    ("Eternal Cycle", "轮回"),
    ("Karmic Wheel", "业轮"),
    ("Transcendent Path", "渡道"),
    ("Tribulation Gate", "劫门"),
    ("Calamity Star", "灾星"),
    ("Fortune Moon", "运月"),
    ("Destiny Harbor", "命港"),
    ("Fate's Edge", "缘锋"),
    ("Legacy Vault", "传窖"),
    ("Heritage Gate", "承门"),
    ("Dynasty Spire", "朝塔"),
    ("Reign Peak", "统峰"),
    ("Domain Sea", "域海"),
    ("Territory Sky", "疆天"),
    ("Boundary River", "界河"),
    ("Border Watch", "境哨"),
    ("Frontier Post", "边站"),
    ("Outpost Gate", "前门"),
    ("Vanguard Harbor", "锋港"),
    ("Pioneer Peak", "拓峰"),
    ("Explorer's Rest", "探栖"),
    ("Wanderer's Road", "游路"),
    ("Traveler's Star", "旅星"),
];

const HSK6_NAMES: &[(&str, &str)] = &[
    ("Singularity Well", "奇井"),
    ("Entropy Shrine", "熵祠"),
    ("Transcendence Gate", "超门"),
    ("Paradox Engine", "悖机"),
    ("Primordial Vault", "元窖"),
    ("Eschaton Beacon", "末灯"),
    ("Omniscient Array", "博阵"),
    ("Convergence Node", "汇节"),
    ("Apotheosis Ring", "化环"),
    ("Oblivion Maw", "忘口"),
    ("Chrysalis Dock", "蛹坞"),
    ("Resonance Spire", "鸣塔"),
    ("Antimatter Well", "反井"),
    ("Tesseract Vault", "维窖"),
    ("Quintessence Core", "精核"),
    ("Chaos Origin", "混沌"),
    ("Primordial Mist", "鸿蒙"),
    ("Great Void", "太虚"),
    ("Infinite Pole", "无极"),
    ("Silent Death", "寂灭"),
    ("Nirvana Gate", "涅槃"),
    ("Reincarnation Wheel", "轮回"),
    ("Karmic Return", "因果"),
    ("Cosmic Law", "天理"),
    ("Universal Truth", "道真"),
    ("Supreme Unity", "太一"),
    ("Absolute Void", "绝虚"),
    ("Perfect Silence", "至寂"),
    ("Ultimate Chaos", "极混"),
    ("Eternal Darkness", "永暗"),
    ("Endless Night", "无夜"),
    ("Boundless Abyss", "无渊"),
    ("Infinite Deep", "无底"),
    ("Timeless Void", "不时"),
    ("Spaceless Realm", "无界"),
    ("Formless Gate", "无形"),
    ("Nameless Star", "无名"),
    ("Ineffable Moon", "不言"),
    ("Unknowable Sun", "不知"),
    ("Unspeakable Harbor", "不语"),
    ("Unthinkable Peak", "不思"),
    ("Incomprehensible Sea", "不解"),
    ("Unfathomable Sky", "不测"),
    ("Immeasurable River", "不量"),
    ("Uncountable Stars", "不数"),
    ("Unnameable Void", "不名"),
];

/// Return the name pool for a given HSK level (clamped to 1–6).
fn name_pool(hsk_level: u8) -> &'static [(&'static str, &'static str)] {
    match hsk_level {
        1 => HSK1_NAMES,
        2 => HSK2_NAMES,
        3 => HSK3_NAMES,
        4 => HSK4_NAMES,
        5 => HSK5_NAMES,
        _ => HSK6_NAMES,
    }
}

// ---------------------------------------------------------------------------
// System description pools — one pool per location type
// ---------------------------------------------------------------------------

const STATION_DESCRIPTIONS: &[&str] = &[
    "A bustling hub of commerce and diplomacy.",
    "An aging station held together by hope and duct tape.",
    "Modern facilities gleam under artificial lighting.",
    "Hundreds of ships dock here daily in organized chaos.",
    "A neutral ground where all factions meet to trade.",
    "The main concourse echoes with a dozen languages.",
    "Security is tight; scanners check every arrival.",
    "This station serves as a regional administrative center.",
];

const ASTEROID_BASE_DESCRIPTIONS: &[&str] = &[
    "Carved into a massive asteroid, this base is nearly invisible.",
    "Mining operations have hollowed out this rock over decades.",
    "A rough-and-tumble outpost where miners drink and gamble.",
    "Gravity is low; magnetic boots are required in most areas.",
    "The base clings to the asteroid's surface like a metallic barnacle.",
    "Ore refineries rumble constantly in the lower decks.",
    "Smugglers favor this remote location for clandestine deals.",
    "Ancient excavation tunnels branch off in every direction.",
];

const DERELICT_SHIP_DESCRIPTIONS: &[&str] = &[
    "A massive warship drifts silently, its crew long gone.",
    "Scorch marks and hull breaches tell a story of battle.",
    "Emergency lights flicker in the abandoned corridors.",
    "Salvagers have picked over the wreck, but secrets remain.",
    "This vessel was once the pride of a forgotten fleet.",
    "Strange signals emanate from the ship's dead reactor.",
    "The airlock hangs open, exposing dark passages within.",
    "Frozen corpses still man their stations on the bridge.",
];

const ALIEN_RUINS_DESCRIPTIONS: &[&str] = &[
    "Ancient structures of unknown origin rise from the surface.",
    "Glyphs cover every surface, their meaning lost to time.",
    "Energy readings suggest the ruins are not entirely inert.",
    "No one knows who built this place or why they vanished.",
    "The architecture defies conventional physics.",
    "Explorers report strange dreams after visiting this site.",
    "Artifacts recovered here fetch high prices on the black market.",
    "The ruins seem to shift when not directly observed.",
];

const TRADING_POST_DESCRIPTIONS: &[&str] = &[
    "A free port where anything can be bought or sold.",
    "Merchants from a hundred worlds hawk their wares here.",
    "No questions asked, no records kept—cash only.",
    "The market sprawls across multiple docking rings.",
    "Prices fluctuate wildly based on supply and demand.",
    "This post serves as a neutral meeting ground for rivals.",
    "Black market goods are sold openly alongside legal cargo.",
    "Hagglers and hustlers fill every corridor.",
];

const ORBITAL_PLATFORM_DESCRIPTIONS: &[&str] = &[
    "A sleek platform orbits a gas giant, harvesting fuel.",
    "Military patrols launch from this strategic position.",
    "The platform rotates slowly to simulate gravity.",
    "Observation decks offer stunning views of the planet below.",
    "A network of tethers connects the platform to mining drones.",
    "This facility processes raw materials from nearby asteroids.",
    "Research labs occupy the upper levels of the structure.",
    "The platform serves as an early warning outpost.",
];

const MINING_COLONY_DESCRIPTIONS: &[&str] = &[
    "A frontier colony built around rich mineral deposits.",
    "Life here is hard, but the pay attracts desperate workers.",
    "Dust from the mines coats everything in a fine layer.",
    "The colony bar is the social center after long shifts.",
    "Corporate overseers watch production quotas ruthlessly.",
    "Families have made this harsh place their home.",
    "Mining accidents are common; safety is often ignored.",
    "The colony exports rare elements vital to FTL drives.",
];

const RESEARCH_LAB_DESCRIPTIONS: &[&str] = &[
    "Cutting-edge research into exotic physics happens here.",
    "Scientists work in sterile environments behind thick shielding.",
    "The lab is funded by a coalition of academic institutions.",
    "Experimental drives and weapons are tested in nearby space.",
    "Access is restricted; visitors must be cleared by security.",
    "Breakthroughs here could revolutionize interstellar travel.",
    "The staff is tight-lipped about their current projects.",
    "Strange phenomena have been reported in the lab's vicinity.",
];

/// Get a random description for a location type.
fn pick_description(rng: &mut u32, location_type: &LocationType) -> &'static str {
    let pool = match location_type {
        LocationType::SpaceStation => STATION_DESCRIPTIONS,
        LocationType::AsteroidBase => ASTEROID_BASE_DESCRIPTIONS,
        LocationType::DerelictShip => DERELICT_SHIP_DESCRIPTIONS,
        LocationType::AlienRuins => ALIEN_RUINS_DESCRIPTIONS,
        LocationType::TradingPost => TRADING_POST_DESCRIPTIONS,
        LocationType::OrbitalPlatform => ORBITAL_PLATFORM_DESCRIPTIONS,
        LocationType::MiningColony => MINING_COLONY_DESCRIPTIONS,
        LocationType::ResearchLab => RESEARCH_LAB_DESCRIPTIONS,
    };
    pool[lcg_range(rng, pool.len())]
}

// ---------------------------------------------------------------------------
// Sector names and descriptions
// ---------------------------------------------------------------------------

const SECTOR_DATA: &[(&str, &str)] = &[
    ("Jade Frontier", "The gateway to the frontier. Relatively safe trade routes connect modest stations amid scattered mining operations."),
    ("Dragon Expanse", "Ancient dragon-carved beacons guide travelers through this storied expanse. Pirates lurk in the asteroid shadows."),
    ("Silk Nebula", "Shimmering gas clouds conceal hidden bases. Smugglers and merchants trade secrets as readily as cargo."),
    ("Crimson Dominion", "A war-torn region where crimson stars illuminate the wreckage of old battles. Danger lurks in every system."),
    ("Twilight Sovereignty", "The sovereign domains of mysterious powers. Few outsiders understand the politics of this twilight realm."),
    ("Singularity Reach", "At the edge of known space, reality bends near the great singularity. Only the desperate or foolish venture here."),
    ("Azure Haven", "Blue giants illuminate peaceful colonies. This haven offers respite before venturing into darker sectors."),
    ("Obsidian Wastes", "Black holes and dead stars mark this desolate region. Scavengers pick through ancient battlefield debris."),
    ("Emerald Confluence", "Trade routes converge here like rivers meeting the sea. Prosperity and opportunity abound for the bold."),
    ("Amber Preserve", "A protected zone where ancient artifacts are studied. Strict regulations govern all who enter."),
    ("Violet Expanse", "Purple nebulae hide experimental research stations. Unauthorized entry results in immediate interdiction."),
    ("Scarlet Terminus", "The final frontier before the unknown void. Beyond lies uncharted space and infinite mystery."),
];

// ---------------------------------------------------------------------------
// LocationType helper methods
// ---------------------------------------------------------------------------

impl LocationType {
    pub fn icon(&self) -> &'static str {
        match self {
            LocationType::SpaceStation => "🏛",
            LocationType::AsteroidBase => "🪨",
            LocationType::DerelictShip => "💀",
            LocationType::AlienRuins => "🏺",
            LocationType::TradingPost => "💰",
            LocationType::OrbitalPlatform => "🛸",
            LocationType::MiningColony => "⛏",
            LocationType::ResearchLab => "🔬",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            LocationType::SpaceStation => "#4A90E2",      // Blue
            LocationType::AsteroidBase => "#8B7355",      // Brown
            LocationType::DerelictShip => "#2C2C2C",      // Dark gray
            LocationType::AlienRuins => "#9B59B6",        // Purple
            LocationType::TradingPost => "#F39C12",       // Gold
            LocationType::OrbitalPlatform => "#1ABC9C",   // Teal
            LocationType::MiningColony => "#E67E22",      // Orange
            LocationType::ResearchLab => "#3498DB",       // Light blue
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            LocationType::SpaceStation => "Space Station",
            LocationType::AsteroidBase => "Asteroid Base",
            LocationType::DerelictShip => "Derelict Ship",
            LocationType::AlienRuins => "Alien Ruins",
            LocationType::TradingPost => "Trading Post",
            LocationType::OrbitalPlatform => "Orbital Platform",
            LocationType::MiningColony => "Mining Colony",
            LocationType::ResearchLab => "Research Lab",
        }
    }

    pub fn bonus_description(&self) -> &'static str {
        match self {
            LocationType::SpaceStation => "\u{1f3e5} Free healing on entry",
            LocationType::AsteroidBase => "\u{26cf} Double ore/gold from mining",
            LocationType::DerelictShip => "\u{1f480} More loot, tougher enemies",
            LocationType::AlienRuins => "\u{1f3fa} Bonus radicals from puzzles",
            LocationType::TradingPost => "\u{1f4b0} 25% shop discount",
            LocationType::OrbitalPlatform => "\u{1f6f8} Shield recharge on entry",
            LocationType::MiningColony => "\u{2692} Extra credits per kill",
            LocationType::ResearchLab => "\u{1f52c} Double vocab XP",
        }
    }
}

// ---------------------------------------------------------------------------
// Location type distribution
// ---------------------------------------------------------------------------

const LOCATION_TYPES: &[LocationType] = &[
    LocationType::MiningColony,
    LocationType::SpaceStation,
    LocationType::AsteroidBase,
    LocationType::ResearchLab,
    LocationType::TradingPost,
    LocationType::DerelictShip,
    LocationType::AlienRuins,
    LocationType::OrbitalPlatform,
    LocationType::SpaceStation,
];

/// Pick a `LocationType` weighted by sector theme.
fn pick_location_type(rng: &mut u32, hsk_level: u8) -> LocationType {
    // Early sectors lean toward friendlier locations; later sectors are harsher.
    let roll = lcg_range(rng, 100);
    match hsk_level {
        1 => match roll {
            0..=29  => LocationType::MiningColony,
            30..=49 => LocationType::SpaceStation,
            50..=59 => LocationType::TradingPost,
            60..=69 => LocationType::ResearchLab,
            70..=79 => LocationType::AsteroidBase,
            80..=89 => LocationType::DerelictShip,
            _       => LocationType::AlienRuins,
        },
        2 | 3 => match roll {
            0..=19  => LocationType::MiningColony,
            20..=34 => LocationType::SpaceStation,
            35..=44 => LocationType::AsteroidBase,
            45..=54 => LocationType::ResearchLab,
            55..=64 => LocationType::DerelictShip,
            65..=74 => LocationType::TradingPost,
            75..=84 => LocationType::AlienRuins,
            _       => LocationType::OrbitalPlatform,
        },
        _ => match roll {
            0..=14  => LocationType::MiningColony,
            15..=24 => LocationType::SpaceStation,
            25..=39 => LocationType::DerelictShip,
            40..=49 => LocationType::AsteroidBase,
            50..=59 => LocationType::ResearchLab,
            60..=69 => LocationType::AlienRuins,
            70..=79 => LocationType::OrbitalPlatform,
            80..=89 => LocationType::SpaceStation,
            _       => LocationType::TradingPost,
        },
    }
}

/// Pick a random hazard, or None if roll fails.
fn pick_hazard(rng: &mut u32, hsk_level: u8) -> Option<SystemHazard> {
    // Later sectors have more hazards: HSK1 ~15%, HSK6 ~45%
    let threshold = 85 - (hsk_level as usize * 10);
    if lcg_range(rng, 100) < threshold {
        return None;
    }

    let hazards = [
        SystemHazard::RadiationBelt,
        SystemHazard::AsteroidField,
        SystemHazard::IonStorm,
        SystemHazard::PirateTerritory,
        SystemHazard::GravityWell,
        SystemHazard::Nebula,
        SystemHazard::SolarFlare,
        SystemHazard::MineField,
        SystemHazard::DarkMatter,
        SystemHazard::VoidRift,
    ];
    
    Some(hazards[lcg_range(rng, hazards.len())].clone())
}

// ---------------------------------------------------------------------------
// Sector generation
// ---------------------------------------------------------------------------

/// Generate a complete sector with interconnected star systems.
///
/// # Arguments
/// * `sector_id` — Zero-based index of the sector.
/// * `hsk_level` — HSK difficulty (1–6).
/// * `rng_seed`  — Seed for the LCG so maps are reproducible.
///
/// # Guarantees
/// * 10–22 star systems are created (more in later sectors).
/// * A path from `start_system` → `boss_system` → `exit_system` always exists.
/// * Multiple branching paths and loops for exploration.
/// * 1–2 shops and 1–2 fuel stations are placed.
/// * Repair, medbay, and quest givers are distributed.
/// * ~30% of systems have hazards (more in later sectors).
/// * Hidden systems appear in sectors 3+.
/// * System positions are spread across the map for visual clarity.
pub fn generate_sector(sector_id: usize, hsk_level: u8, rng_seed: u32) -> Sector {
    let mut rng = rng_seed;

    // Number of systems: 10-22, scaling with HSK level
    let base = 10 + (hsk_level as usize) * 2; // 12-22
    let num_systems = base + lcg_range(&mut rng, 4); // +0-3 variance

    let names = name_pool(hsk_level);

    // ── Create systems with positions spread in columns ──────────────
    let mut systems: Vec<StarSystem> = Vec::with_capacity(num_systems);

    // Spread systems left-to-right in columns for a readable layout.
    // The first system is at x≈0.05, the last at x≈0.95.
    for i in 0..num_systems {
        let name_idx = lcg_range(&mut rng, names.len());
        // Avoid duplicate names within a sector by cycling through the pool.
        let effective_idx = (name_idx + i) % names.len();
        let (eng, chn) = names[effective_idx];

        let x_base = (i as f64 + 0.5) / num_systems as f64;
        let x = (x_base + (lcg_f64(&mut rng) - 0.5) * 0.06).clamp(0.02, 0.98);
        let y = (0.15 + lcg_f64(&mut rng) * 0.70).clamp(0.05, 0.95);

        let loc = pick_location_type(&mut rng, hsk_level);
        let desc = pick_description(&mut rng, &loc);
        let hazard = pick_hazard(&mut rng, hsk_level);
        
        // Special features
        let has_repair = lcg_range(&mut rng, 100) < 15;
        let has_medbay = lcg_range(&mut rng, 100) < 20;
        let quest_giver = lcg_range(&mut rng, 100) < 25;
        let hidden = hsk_level >= 3 && lcg_range(&mut rng, 100) < 10;

        systems.push(StarSystem {
            id: i,
            name: eng,
            chinese_name: chn,
            description: desc,
            x,
            y,
            location_type: loc,
            hazard,
            visited: false,
            has_shop: false,
            has_fuel: false,
            has_repair,
            has_medbay,
            quest_giver,
            hidden,
            warp_gate: false,
            event_id: None,
            connections: Vec::new(),
            difficulty: hsk_level,
        });
    }

    // Mark the start system as visited.
    systems[0].visited = true;

    // ── Build the connectivity graph with branching paths ─────────────────────────────────

    let start = 0;
    let boss = num_systems - 2;
    let exit = num_systems - 1;

    // 1) Create a guaranteed main path: start → … → boss → exit.
    //    This path will have branch points where player can choose alternate routes.
    let mut main_path: Vec<usize> = vec![start];
    {
        let mut cursor = start;
        while cursor < boss {
            // Usually step +1; occasionally skip one node forward for variety.
            let step = if cursor + 2 < boss && lcg_range(&mut rng, 3) == 0 {
                2
            } else {
                1
            };
            let next = (cursor + step).min(boss);
            add_edge(&mut systems, cursor, next);
            main_path.push(next);
            cursor = next;
        }
        // boss → exit
        add_edge(&mut systems, boss, exit);
        main_path.push(exit);
    }

    // 2) Add branching paths at strategic points
    // Create 3-5 branch points where paths diverge and optionally rejoin
    let num_branches = 3 + lcg_range(&mut rng, 3);
    for _ in 0..num_branches {
        // Pick a point on the main path to branch from
        if main_path.len() < 3 {
            break;
        }
        let branch_idx = lcg_range(&mut rng, main_path.len() - 2) + 1;
        let branch_point = main_path[branch_idx];
        
        // Create a branch of 1-3 systems
        let branch_len = 1 + lcg_range(&mut rng, 3);
        let mut branch_systems = Vec::new();
        
        for j in 0..branch_len {
            // Find a system not on the main path to use as a branch node
            let candidates: Vec<usize> = (1..num_systems-2)
                .filter(|&s| !main_path.contains(&s) && !branch_systems.contains(&s))
                .collect();
            
            if candidates.is_empty() {
                break;
            }
            
            let branch_node = candidates[lcg_range(&mut rng, candidates.len())];
            
            if j == 0 {
                // Connect first branch node to branch point
                add_edge(&mut systems, branch_point, branch_node);
            } else {
                // Connect to previous branch node
                add_edge(&mut systems, branch_systems[j - 1], branch_node);
            }
            
            branch_systems.push(branch_node);
        }
        
        // 50% chance to rejoin the main path later (creating a loop)
        if lcg_range(&mut rng, 2) == 0 && !branch_systems.is_empty() {
            let rejoin_idx = (branch_idx + 2).min(main_path.len() - 1);
            let rejoin_point = main_path[rejoin_idx];
            let last_branch = *branch_systems.last().unwrap();
            add_edge(&mut systems, last_branch, rejoin_point);
        }
    }

    // 3) Add extra cross-links between neighbours to create alternate routes.
    for i in 0..num_systems.saturating_sub(2) {
        if lcg_range(&mut rng, 3) == 0 {
            add_edge(&mut systems, i, i + 2);
        }
    }
    
    // 4) Add some random connections for late-game sectors to increase complexity
    if hsk_level >= 4 {
        let extra_connections = 2 + lcg_range(&mut rng, 4);
        for _ in 0..extra_connections {
            let a = lcg_range(&mut rng, num_systems);
            let b = lcg_range(&mut rng, num_systems);
            if a != b && (a as isize - b as isize).unsigned_abs() as usize <= 4 {
                add_edge(&mut systems, a, b);
            }
        }
    }

    // ── Place shops (1–2) and fuel stations (1–2) ────────────────────
    let num_shops = 1 + lcg_range(&mut rng, 2);
    let num_fuel  = 1 + lcg_range(&mut rng, 2);

    place_features(&mut systems, &mut rng, num_shops, true);
    place_features(&mut systems, &mut rng, num_fuel, false);

    // ── Override location types for special systems ──────────────────
    systems[start].location_type = LocationType::SpaceStation;
    systems[boss].location_type  = LocationType::DerelictShip;
    systems[exit].location_type  = LocationType::SpaceStation;
    
    // Boss system gets a warp gate
    systems[boss].warp_gate = true;
    
    // Update descriptions for special systems
    systems[start].description = "The entry point to this sector. A safe harbor for new arrivals.";
    systems[boss].description = "A dangerous derelict harboring the sector's greatest threat.";
    systems[exit].description = "The gateway to the next sector. Prepare well before moving on.";

    // ── Assign event IDs to some mid-path systems ────────────────────
    for i in 1..num_systems.saturating_sub(1) {
        if lcg_range(&mut rng, 3) == 0 {
            systems[i].event_id = Some(lcg_range(&mut rng, 100));
        }
    }

    // ── Build sector ─────────────────────────────────────────────────
    let (sector_name, sector_desc) = if sector_id < SECTOR_DATA.len() {
        SECTOR_DATA[sector_id]
    } else {
        ("Unknown Reach", "An uncharted region of space.")
    };

    Sector {
        id: sector_id,
        name: sector_name,
        description: sector_desc,
        systems,
        start_system: start,
        boss_system: boss,
        exit_system: exit,
        hsk_level,
    }
}

/// Add an undirected edge between two systems (idempotent).
fn add_edge(systems: &mut [StarSystem], a: usize, b: usize) {
    if a == b {
        return;
    }
    if !systems[a].connections.contains(&b) {
        systems[a].connections.push(b);
    }
    if !systems[b].connections.contains(&a) {
        systems[b].connections.push(a);
    }
}

/// Scatter shops or fuel stations across interior systems.
fn place_features(systems: &mut [StarSystem], rng: &mut u32, count: usize, is_shop: bool) {
    let len = systems.len();
    if len <= 2 {
        return;
    }
    let mut placed = 0;
    let mut attempts = 0;
    while placed < count && attempts < len * 3 {
        // Avoid start (0), boss (len-2), and exit (len-1).
        let idx = 1 + lcg_range(rng, len.saturating_sub(3).max(1));
        let already = if is_shop { systems[idx].has_shop } else { systems[idx].has_fuel };
        if !already {
            if is_shop {
                systems[idx].has_shop = true;
                systems[idx].location_type = LocationType::TradingPost;
            } else {
                systems[idx].has_fuel = true;
                systems[idx].location_type = LocationType::MiningColony;
            }
            placed += 1;
        }
        attempts += 1;
    }
}

// ---------------------------------------------------------------------------
// SectorMap construction
// ---------------------------------------------------------------------------

impl SectorMap {
    /// Create a new galaxy map starting at sector 0.
    ///
    /// Generates all six sectors up-front so the player can see the full
    /// galactic journey ahead.
    pub fn new(base_seed: u32) -> Self {
        let sectors: Vec<Sector> = (0..6)
            .map(|i| {
                let hsk = (i + 1) as u8;
                let seed = base_seed.wrapping_add(i as u32 * 7919);
                generate_sector(i, hsk, seed)
            })
            .collect();

        SectorMap {
            sectors,
            current_sector: 0,
            current_system: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Navigation helpers
// ---------------------------------------------------------------------------

/// Check whether the player can jump from their current system to `target_system`.
///
/// A jump is valid when:
/// 1. `target_system` is directly connected to the current system.
/// 2. The sector index is valid.
pub fn can_jump_to(map: &SectorMap, target_system: usize) -> bool {
    if map.current_sector >= map.sectors.len() {
        return false;
    }
    let sector = &map.sectors[map.current_sector];
    let cur = map.current_system;
    if cur >= sector.systems.len() {
        return false;
    }
    sector.systems[cur].connections.contains(&target_system)
        && target_system < sector.systems.len()
}

/// Calculate the fuel cost of jumping between two systems.
///
/// Cost is the Euclidean distance × 10, rounded up, so adjacent systems
/// cost roughly 1–14 fuel depending on spacing.
pub fn jump_cost(from: &StarSystem, to: &StarSystem) -> i32 {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let dist = (dx * dx + dy * dy).sqrt();
    (dist * 10.0).ceil() as i32
}

/// Return the IDs of all systems directly reachable from `system_id`.
pub fn connected_systems(sector: &Sector, system_id: usize) -> Vec<usize> {
    sector
        .systems
        .iter()
        .find(|s| s.id == system_id)
        .map(|s| s.connections.clone())
        .unwrap_or_default()
}

/// Advance the player to the next sector.
///
/// The current system is set to the new sector's `start_system` and that
/// system is marked as visited.
pub fn advance_sector(map: &mut SectorMap) {
    if map.current_sector + 1 < map.sectors.len() {
        map.current_sector += 1;
        let start = map.sectors[map.current_sector].start_system;
        map.current_system = start;
        map.sectors[map.current_sector].systems[start].visited = true;
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sector_has_correct_system_count() {
        let sector = generate_sector(0, 1, 42);
        assert!(sector.systems.len() >= 10);
        assert!(sector.systems.len() <= 25);
    }

    #[test]
    fn path_from_start_to_exit_exists() {
        let sector = generate_sector(0, 1, 123);
        // BFS from start to exit.
        let mut visited = vec![false; sector.systems.len()];
        let mut queue = vec![sector.start_system];
        visited[sector.start_system] = true;
        while let Some(cur) = queue.pop() {
            for &nb in &sector.systems[cur].connections {
                if !visited[nb] {
                    visited[nb] = true;
                    queue.push(nb);
                }
            }
        }
        assert!(visited[sector.boss_system], "boss must be reachable from start");
        assert!(visited[sector.exit_system], "exit must be reachable from start");
    }

    #[test]
    fn shops_and_fuel_are_placed() {
        let sector = generate_sector(0, 1, 999);
        let shops = sector.systems.iter().filter(|s| s.has_shop).count();
        let fuel  = sector.systems.iter().filter(|s| s.has_fuel).count();
        assert!(shops >= 1, "at least 1 shop expected");
        assert!(fuel >= 1, "at least 1 fuel station expected");
    }

    #[test]
    fn jump_cost_is_positive() {
        let sector = generate_sector(0, 2, 77);
        if sector.systems.len() >= 2 {
            let cost = jump_cost(&sector.systems[0], &sector.systems[1]);
            assert!(cost > 0);
        }
    }

    #[test]
    fn can_jump_respects_connections() {
        let sector = generate_sector(0, 1, 55);
        let map = SectorMap {
            sectors: vec![sector.clone()],
            current_sector: 0,
            current_system: 0,
        };
        // System 0 should be able to jump to its first connection.
        if let Some(&target) = sector.systems[0].connections.first() {
            assert!(can_jump_to(&map, target));
        }
        // A system with no direct link should fail.
        let disconnected = sector.systems.len() + 100;
        assert!(!can_jump_to(&map, disconnected));
    }

    #[test]
    fn advance_sector_increments() {
        let mut map = SectorMap::new(42);
        assert_eq!(map.current_sector, 0);
        advance_sector(&mut map);
        assert_eq!(map.current_sector, 1);
        assert_eq!(map.current_system, map.sectors[1].start_system);
    }

    #[test]
    fn connected_systems_returns_neighbours() {
        let sector = generate_sector(0, 1, 101);
        let nbrs = connected_systems(&sector, 0);
        assert!(!nbrs.is_empty(), "start system must have connections");
    }

    #[test]
    fn unique_name_pools_have_enough_entries() {
        // Now we have 250+ total unique names across all HSK levels.
        let total: usize = (1..=6).map(|l| name_pool(l).len()).sum();
        assert!(total >= 250, "need at least 250 unique names, got {total}");
    }

    #[test]
    fn deterministic_generation() {
        let a = generate_sector(0, 1, 12345);
        let b = generate_sector(0, 1, 12345);
        assert_eq!(a.systems.len(), b.systems.len());
        for (sa, sb) in a.systems.iter().zip(b.systems.iter()) {
            assert_eq!(sa.name, sb.name);
            assert_eq!(sa.chinese_name, sb.chinese_name);
            assert_eq!(sa.connections, sb.connections);
        }
    }
}



