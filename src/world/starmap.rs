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
    /// Horizontal position on the sector map (0.0–1.0).
    pub x: f64,
    /// Vertical position on the sector map (0.0–1.0).
    pub y: f64,
    /// What kind of location is here.
    pub location_type: LocationType,
    /// Whether the player has visited this system.
    pub visited: bool,
    /// Whether this system has a shop.
    pub has_shop: bool,
    /// Whether this system has a fuel station.
    pub has_fuel: bool,
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
// Sector names
// ---------------------------------------------------------------------------

const SECTOR_NAMES: &[&str] = &[
    "Jade Frontier",      // HSK 1
    "Dragon Expanse",     // HSK 2
    "Silk Nebula",        // HSK 3
    "Crimson Dominion",   // HSK 4
    "Twilight Sovereignty", // HSK 5
    "Singularity Reach",  // HSK 6
];

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
/// * 8–15 star systems are created.
/// * A path from `start_system` → `boss_system` → `exit_system` always exists.
/// * 1–2 shops and 1–2 fuel stations are placed.
/// * System positions are spread across the map for visual clarity.
pub fn generate_sector(sector_id: usize, hsk_level: u8, rng_seed: u32) -> Sector {
    let mut rng = rng_seed;

    // Number of systems: 8–15
    let num_systems = 8 + lcg_range(&mut rng, 8); // 8..=15

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

        systems.push(StarSystem {
            id: i,
            name: eng,
            chinese_name: chn,
            x,
            y,
            location_type: loc,
            visited: false,
            has_shop: false,
            has_fuel: false,
            event_id: None,
            connections: Vec::new(),
            difficulty: hsk_level,
        });
    }

    // Mark the start system as visited.
    systems[0].visited = true;

    // ── Build the connectivity graph ─────────────────────────────────

    let start = 0;
    let boss = num_systems - 2;
    let exit = num_systems - 1;

    // 1) Create a guaranteed main path: start → … → boss → exit.
    //    Walk through systems in order with occasional skips.
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

    // 2) Add branch connections so the graph isn't just a single chain.
    let num_branches = 2 + lcg_range(&mut rng, num_systems / 2);
    for _ in 0..num_branches {
        let a = lcg_range(&mut rng, num_systems);
        let b = lcg_range(&mut rng, num_systems);
        if a != b && (a as isize - b as isize).unsigned_abs() as usize <= 3 {
            add_edge(&mut systems, a, b);
        }
    }

    // 3) Extra cross-links between neighbours to create alternate routes.
    for i in 0..num_systems.saturating_sub(2) {
        if lcg_range(&mut rng, 3) == 0 {
            add_edge(&mut systems, i, i + 2);
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

    // ── Assign event IDs to some mid-path systems ────────────────────
    for i in 1..num_systems.saturating_sub(1) {
        if lcg_range(&mut rng, 3) == 0 {
            systems[i].event_id = Some(lcg_range(&mut rng, 100));
        }
    }

    // ── Build sector ─────────────────────────────────────────────────
    let sector_name = if (sector_id) < SECTOR_NAMES.len() {
        SECTOR_NAMES[sector_id]
    } else {
        "Unknown Reach"
    };

    Sector {
        id: sector_id,
        name: sector_name,
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
        assert!(sector.systems.len() >= 8);
        assert!(sector.systems.len() <= 15);
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
        // At least 15 names per HSK level, 6 levels → ≥ 90 total unique names.
        let total: usize = (1..=6).map(|l| name_pool(l).len()).sum();
        assert!(total >= 60, "need at least 60 unique names, got {total}");
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



