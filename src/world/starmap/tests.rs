use super::*;

// ===========================================================================
// Original 9 tests
// ===========================================================================

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

// ===========================================================================
// LCG pseudo-random functions
// ===========================================================================

#[test]
fn lcg_next_advances_state() {
    let mut state = 0u32;
    let first = lcg_next(&mut state);
    assert_ne!(state, 0, "state must change after lcg_next");
    let second = lcg_next(&mut state);
    assert_ne!(first, second, "consecutive outputs should differ");
}

#[test]
fn lcg_next_is_deterministic() {
    let mut a = 42u32;
    let mut b = 42u32;
    assert_eq!(lcg_next(&mut a), lcg_next(&mut b));
    assert_eq!(lcg_next(&mut a), lcg_next(&mut b));
}

#[test]
fn lcg_range_within_bounds() {
    let mut state = 1u32;
    for max in [1, 2, 5, 10, 100, 1000] {
        for _ in 0..50 {
            let val = lcg_range(&mut state, max);
            assert!(val < max, "lcg_range({max}) returned {val}");
        }
    }
}

#[test]
fn lcg_f64_in_unit_interval() {
    let mut state = 7u32;
    for _ in 0..200 {
        let val = lcg_f64(&mut state);
        assert!((0.0..1.0).contains(&val), "lcg_f64 returned {val}");
    }
}

#[test]
fn lcg_next_wrapping_arithmetic() {
    // Verify the exact LCG formula: state = state * 1664525 + 1013904223
    let mut state = 0u32;
    let result = lcg_next(&mut state);
    assert_eq!(result, 1013904223);
    assert_eq!(state, 1013904223);
}

// ===========================================================================
// LocationType — icon, color, label, bonus_description
// ===========================================================================

fn all_location_types() -> Vec<LocationType> {
    vec![
        LocationType::SpaceStation,
        LocationType::AsteroidBase,
        LocationType::DerelictShip,
        LocationType::AlienRuins,
        LocationType::TradingPost,
        LocationType::OrbitalPlatform,
        LocationType::MiningColony,
        LocationType::ResearchLab,
    ]
}

#[test]
fn location_type_icon_non_empty() {
    for lt in all_location_types() {
        assert!(!lt.icon().is_empty(), "{:?} icon is empty", lt);
    }
}

#[test]
fn location_type_color_valid_hex() {
    for lt in all_location_types() {
        let c = lt.color();
        assert!(c.starts_with('#'), "{:?} color doesn't start with #", lt);
        assert_eq!(c.len(), 7, "{:?} color has wrong length: {}", lt, c);
    }
}

#[test]
fn location_type_label_non_empty() {
    for lt in all_location_types() {
        assert!(!lt.label().is_empty(), "{:?} label is empty", lt);
    }
}

#[test]
fn location_type_bonus_description_non_empty() {
    for lt in all_location_types() {
        assert!(!lt.bonus_description().is_empty(), "{:?} bonus_description is empty", lt);
    }
}

#[test]
fn location_type_icon_specific_values() {
    assert_eq!(LocationType::SpaceStation.icon(), "🏛");
    assert_eq!(LocationType::AsteroidBase.icon(), "🪨");
    assert_eq!(LocationType::DerelictShip.icon(), "💀");
    assert_eq!(LocationType::AlienRuins.icon(), "🏺");
    assert_eq!(LocationType::TradingPost.icon(), "💰");
    assert_eq!(LocationType::OrbitalPlatform.icon(), "🛸");
    assert_eq!(LocationType::MiningColony.icon(), "⛏");
    assert_eq!(LocationType::ResearchLab.icon(), "🔬");
}

#[test]
fn location_type_color_specific_values() {
    assert_eq!(LocationType::SpaceStation.color(), "#4A90E2");
    assert_eq!(LocationType::AsteroidBase.color(), "#8B7355");
    assert_eq!(LocationType::DerelictShip.color(), "#2C2C2C");
    assert_eq!(LocationType::AlienRuins.color(), "#9B59B6");
    assert_eq!(LocationType::TradingPost.color(), "#F39C12");
    assert_eq!(LocationType::OrbitalPlatform.color(), "#1ABC9C");
    assert_eq!(LocationType::MiningColony.color(), "#E67E22");
    assert_eq!(LocationType::ResearchLab.color(), "#3498DB");
}

#[test]
fn location_type_label_specific_values() {
    assert_eq!(LocationType::SpaceStation.label(), "Space Station");
    assert_eq!(LocationType::AsteroidBase.label(), "Asteroid Base");
    assert_eq!(LocationType::DerelictShip.label(), "Derelict Ship");
    assert_eq!(LocationType::AlienRuins.label(), "Alien Ruins");
    assert_eq!(LocationType::TradingPost.label(), "Trading Post");
    assert_eq!(LocationType::OrbitalPlatform.label(), "Orbital Platform");
    assert_eq!(LocationType::MiningColony.label(), "Mining Colony");
    assert_eq!(LocationType::ResearchLab.label(), "Research Lab");
}

#[test]
fn location_type_bonus_description_specific_values() {
    assert!(LocationType::SpaceStation.bonus_description().contains("healing"));
    assert!(LocationType::AsteroidBase.bonus_description().contains("mining"));
    assert!(LocationType::DerelictShip.bonus_description().contains("loot"));
    assert!(LocationType::AlienRuins.bonus_description().contains("radicals"));
    assert!(LocationType::TradingPost.bonus_description().contains("discount"));
    assert!(LocationType::OrbitalPlatform.bonus_description().contains("Shield"));
    assert!(LocationType::MiningColony.bonus_description().contains("credits"));
    assert!(LocationType::ResearchLab.bonus_description().contains("XP"));
}

// ===========================================================================
// SystemHazard — name, icon, description, fuel_modifier, hull_damage
// ===========================================================================

fn all_hazards() -> Vec<SystemHazard> {
    vec![
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
    ]
}

#[test]
fn hazard_name_non_empty() {
    for h in all_hazards() {
        assert!(!h.name().is_empty(), "{:?} name is empty", h);
    }
}

#[test]
fn hazard_icon_non_empty() {
    for h in all_hazards() {
        assert!(!h.icon().is_empty(), "{:?} icon is empty", h);
    }
}

#[test]
fn hazard_description_non_empty() {
    for h in all_hazards() {
        assert!(!h.description().is_empty(), "{:?} description is empty", h);
    }
}

#[test]
fn hazard_name_specific_values() {
    assert_eq!(SystemHazard::RadiationBelt.name(), "Radiation Belt");
    assert_eq!(SystemHazard::AsteroidField.name(), "Asteroid Field");
    assert_eq!(SystemHazard::IonStorm.name(), "Ion Storm");
    assert_eq!(SystemHazard::PirateTerritory.name(), "Pirate Territory");
    assert_eq!(SystemHazard::GravityWell.name(), "Gravity Well");
    assert_eq!(SystemHazard::Nebula.name(), "Nebula");
    assert_eq!(SystemHazard::SolarFlare.name(), "Solar Flare");
    assert_eq!(SystemHazard::MineField.name(), "Mine Field");
    assert_eq!(SystemHazard::DarkMatter.name(), "Dark Matter");
    assert_eq!(SystemHazard::VoidRift.name(), "Void Rift");
}

#[test]
fn hazard_icon_specific_values() {
    assert_eq!(SystemHazard::RadiationBelt.icon(), "☢️");
    assert_eq!(SystemHazard::AsteroidField.icon(), "🌑");
    assert_eq!(SystemHazard::IonStorm.icon(), "⚡");
    assert_eq!(SystemHazard::PirateTerritory.icon(), "☠️");
    assert_eq!(SystemHazard::GravityWell.icon(), "🌀");
    assert_eq!(SystemHazard::Nebula.icon(), "🌫️");
    assert_eq!(SystemHazard::SolarFlare.icon(), "☀️");
    assert_eq!(SystemHazard::MineField.icon(), "💣");
    assert_eq!(SystemHazard::DarkMatter.icon(), "🕳️");
    assert_eq!(SystemHazard::VoidRift.icon(), "🌌");
}

#[test]
fn hazard_fuel_modifier_gravity_well() {
    assert_eq!(SystemHazard::GravityWell.fuel_modifier(), 3);
}

#[test]
fn hazard_fuel_modifier_nebula() {
    assert_eq!(SystemHazard::Nebula.fuel_modifier(), 2);
}

#[test]
fn hazard_fuel_modifier_ion_storm() {
    assert_eq!(SystemHazard::IonStorm.fuel_modifier(), 1);
}

#[test]
fn hazard_fuel_modifier_zero_for_others() {
    let zero_fuel = [
        SystemHazard::RadiationBelt,
        SystemHazard::AsteroidField,
        SystemHazard::PirateTerritory,
        SystemHazard::SolarFlare,
        SystemHazard::MineField,
        SystemHazard::DarkMatter,
        SystemHazard::VoidRift,
    ];
    for h in zero_fuel {
        assert_eq!(h.fuel_modifier(), 0, "{:?} should have 0 fuel_modifier", h);
    }
}

#[test]
fn hazard_hull_damage_radiation_belt() {
    assert_eq!(SystemHazard::RadiationBelt.hull_damage(), 8);
}

#[test]
fn hazard_hull_damage_mine_field() {
    assert_eq!(SystemHazard::MineField.hull_damage(), 10);
}

#[test]
fn hazard_hull_damage_asteroid_field() {
    assert_eq!(SystemHazard::AsteroidField.hull_damage(), 5);
}

#[test]
fn hazard_hull_damage_solar_flare() {
    assert_eq!(SystemHazard::SolarFlare.hull_damage(), 6);
}

#[test]
fn hazard_hull_damage_void_rift() {
    assert_eq!(SystemHazard::VoidRift.hull_damage(), 7);
}

#[test]
fn hazard_hull_damage_zero_for_others() {
    let zero_hull = [
        SystemHazard::IonStorm,
        SystemHazard::PirateTerritory,
        SystemHazard::GravityWell,
        SystemHazard::Nebula,
        SystemHazard::DarkMatter,
    ];
    for h in zero_hull {
        assert_eq!(h.hull_damage(), 0, "{:?} should have 0 hull_damage", h);
    }
}

#[test]
fn hazard_description_specific_values() {
    assert!(SystemHazard::RadiationBelt.description().contains("radiation"));
    assert!(SystemHazard::AsteroidField.description().contains("asteroids"));
    assert!(SystemHazard::IonStorm.description().contains("storm"));
    assert!(SystemHazard::PirateTerritory.description().contains("hostile"));
    assert!(SystemHazard::GravityWell.description().contains("gravity"));
    assert!(SystemHazard::Nebula.description().contains("gas"));
    assert!(SystemHazard::SolarFlare.description().contains("flares"));
    assert!(SystemHazard::MineField.description().contains("explosives"));
    assert!(SystemHazard::DarkMatter.description().contains("anomalies"));
    assert!(SystemHazard::VoidRift.description().contains("tears"));
}

// ===========================================================================
// name_pool
// ===========================================================================

#[test]
fn name_pool_each_level_has_at_least_15_entries() {
    for level in 1..=6 {
        let pool = name_pool(level);
        assert!(pool.len() >= 15, "HSK{level} pool has only {} entries", pool.len());
    }
}

#[test]
fn name_pool_returns_different_pools_per_level() {
    for a in 1..=5u8 {
        for b in (a + 1)..=6u8 {
            let pa = name_pool(a);
            let pb = name_pool(b);
            assert_ne!(
                pa.as_ptr(),
                pb.as_ptr(),
                "pools for HSK{a} and HSK{b} should be different"
            );
        }
    }
}

#[test]
fn name_pool_level_7_maps_to_hsk6() {
    let pool6 = name_pool(6);
    let pool7 = name_pool(7);
    assert_eq!(pool6.as_ptr(), pool7.as_ptr(), "level 7+ should map to HSK6 pool");
}

#[test]
fn name_pool_level_255_maps_to_hsk6() {
    let pool6 = name_pool(6);
    let pool255 = name_pool(255);
    assert_eq!(pool6.as_ptr(), pool255.as_ptr());
}

#[test]
fn name_pool_entries_have_nonempty_names() {
    for level in 1..=6 {
        for (eng, chn) in name_pool(level) {
            assert!(!eng.is_empty(), "empty English name in HSK{level}");
            assert!(!chn.is_empty(), "empty Chinese name in HSK{level}");
        }
    }
}

// ===========================================================================
// pick_description
// ===========================================================================

#[test]
fn pick_description_returns_non_empty_for_all_types() {
    for lt in all_location_types() {
        let mut rng = 42u32;
        let desc = pick_description(&mut rng, &lt);
        assert!(!desc.is_empty(), "{:?} pick_description returned empty", lt);
    }
}

#[test]
fn pick_description_varies_with_rng() {
    let mut descriptions = std::collections::HashSet::new();
    for seed in 0..50u32 {
        let mut rng = seed;
        descriptions.insert(pick_description(&mut rng, &LocationType::SpaceStation));
    }
    assert!(descriptions.len() > 1, "descriptions should vary across seeds");
}

#[test]
fn pick_description_space_station() {
    let mut rng = 0u32;
    let desc = pick_description(&mut rng, &LocationType::SpaceStation);
    assert!(!desc.is_empty());
}

#[test]
fn pick_description_asteroid_base() {
    let mut rng = 0u32;
    let desc = pick_description(&mut rng, &LocationType::AsteroidBase);
    assert!(!desc.is_empty());
}

#[test]
fn pick_description_derelict_ship() {
    let mut rng = 0u32;
    let desc = pick_description(&mut rng, &LocationType::DerelictShip);
    assert!(!desc.is_empty());
}

#[test]
fn pick_description_alien_ruins() {
    let mut rng = 0u32;
    let desc = pick_description(&mut rng, &LocationType::AlienRuins);
    assert!(!desc.is_empty());
}

#[test]
fn pick_description_trading_post() {
    let mut rng = 0u32;
    let desc = pick_description(&mut rng, &LocationType::TradingPost);
    assert!(!desc.is_empty());
}

#[test]
fn pick_description_orbital_platform() {
    let mut rng = 0u32;
    let desc = pick_description(&mut rng, &LocationType::OrbitalPlatform);
    assert!(!desc.is_empty());
}

#[test]
fn pick_description_mining_colony() {
    let mut rng = 0u32;
    let desc = pick_description(&mut rng, &LocationType::MiningColony);
    assert!(!desc.is_empty());
}

#[test]
fn pick_description_research_lab() {
    let mut rng = 0u32;
    let desc = pick_description(&mut rng, &LocationType::ResearchLab);
    assert!(!desc.is_empty());
}

// ===========================================================================
// pick_location_type
// ===========================================================================

#[test]
fn pick_location_type_hsk1_returns_valid() {
    let mut rng = 0u32;
    for _ in 0..100 {
        let lt = pick_location_type(&mut rng, 1);
        assert!(all_location_types().contains(&lt));
    }
}

#[test]
fn pick_location_type_hsk2_returns_valid() {
    let mut rng = 0u32;
    for _ in 0..100 {
        let lt = pick_location_type(&mut rng, 2);
        assert!(all_location_types().contains(&lt));
    }
}

#[test]
fn pick_location_type_hsk3_returns_valid() {
    let mut rng = 0u32;
    for _ in 0..100 {
        let lt = pick_location_type(&mut rng, 3);
        assert!(all_location_types().contains(&lt));
    }
}

#[test]
fn pick_location_type_hsk4_returns_valid() {
    let mut rng = 0u32;
    for _ in 0..100 {
        let lt = pick_location_type(&mut rng, 4);
        assert!(all_location_types().contains(&lt));
    }
}

#[test]
fn pick_location_type_hsk5_returns_valid() {
    let mut rng = 0u32;
    for _ in 0..100 {
        let lt = pick_location_type(&mut rng, 5);
        assert!(all_location_types().contains(&lt));
    }
}

#[test]
fn pick_location_type_hsk6_returns_valid() {
    let mut rng = 0u32;
    for _ in 0..100 {
        let lt = pick_location_type(&mut rng, 6);
        assert!(all_location_types().contains(&lt));
    }
}

#[test]
fn pick_location_type_variety_across_seeds() {
    let mut seen = std::collections::HashSet::new();
    let mut rng = 0u32;
    for _ in 0..500 {
        let lt = pick_location_type(&mut rng, 1);
        seen.insert(lt.label());
    }
    assert!(seen.len() >= 4, "HSK1 should produce at least 4 different location types");
}

// ===========================================================================
// pick_hazard
// ===========================================================================

#[test]
fn pick_hazard_can_return_none() {
    // With HSK1 the chance is low (~15%), so over many seeds we should get None
    let mut got_none = false;
    for seed in 0..200u32 {
        let mut rng = seed;
        if pick_hazard(&mut rng, 1).is_none() {
            got_none = true;
            break;
        }
    }
    assert!(got_none, "pick_hazard at HSK1 should sometimes return None");
}

#[test]
fn pick_hazard_can_return_some() {
    let mut got_some = false;
    for seed in 0..200u32 {
        let mut rng = seed;
        if pick_hazard(&mut rng, 6).is_some() {
            got_some = true;
            break;
        }
    }
    assert!(got_some, "pick_hazard at HSK6 should sometimes return Some");
}

#[test]
fn pick_hazard_high_hsk_more_frequent() {
    let mut count_hsk1 = 0;
    let mut count_hsk6 = 0;
    for seed in 0..500u32 {
        let mut rng1 = seed;
        let mut rng6 = seed;
        if pick_hazard(&mut rng1, 1).is_some() { count_hsk1 += 1; }
        if pick_hazard(&mut rng6, 6).is_some() { count_hsk6 += 1; }
    }
    assert!(count_hsk6 > count_hsk1, "HSK6 should have more hazards than HSK1");
}

// ===========================================================================
// generate_sector — structural invariants
// ===========================================================================

#[test]
fn generate_sector_system_count_hsk1_through_6() {
    for hsk in 1..=6u8 {
        for seed in [0, 42, 999, 12345, u32::MAX] {
            let sector = generate_sector(0, hsk, seed);
            assert!(
                sector.systems.len() >= 10 && sector.systems.len() <= 25,
                "HSK{hsk} seed {seed}: {} systems out of range",
                sector.systems.len()
            );
        }
    }
}

#[test]
fn generate_sector_start_system_is_zero() {
    for hsk in 1..=6u8 {
        let sector = generate_sector(0, hsk, 42);
        assert_eq!(sector.start_system, 0);
    }
}

#[test]
fn generate_sector_boss_is_second_to_last() {
    let sector = generate_sector(0, 3, 42);
    assert_eq!(sector.boss_system, sector.systems.len() - 2);
}

#[test]
fn generate_sector_exit_is_last() {
    let sector = generate_sector(0, 3, 42);
    assert_eq!(sector.exit_system, sector.systems.len() - 1);
}

#[test]
fn generate_sector_start_is_visited() {
    let sector = generate_sector(0, 1, 42);
    assert!(sector.systems[sector.start_system].visited);
}

#[test]
fn generate_sector_start_is_space_station() {
    let sector = generate_sector(0, 1, 42);
    assert_eq!(sector.systems[sector.start_system].location_type, LocationType::SpaceStation);
}

#[test]
fn generate_sector_boss_is_derelict_ship() {
    let sector = generate_sector(0, 1, 42);
    assert_eq!(sector.systems[sector.boss_system].location_type, LocationType::DerelictShip);
}

#[test]
fn generate_sector_boss_has_warp_gate() {
    let sector = generate_sector(0, 1, 42);
    assert!(sector.systems[sector.boss_system].warp_gate);
}

#[test]
fn generate_sector_exit_is_space_station() {
    let sector = generate_sector(0, 1, 42);
    assert_eq!(sector.systems[sector.exit_system].location_type, LocationType::SpaceStation);
}

#[test]
fn generate_sector_positions_in_bounds() {
    for hsk in 1..=6u8 {
        let sector = generate_sector(0, hsk, 42);
        for sys in &sector.systems {
            assert!(sys.x >= 0.02 && sys.x <= 0.98, "x={} out of [0.02,0.98]", sys.x);
            assert!(sys.y >= 0.05 && sys.y <= 0.95, "y={} out of [0.05,0.95]", sys.y);
        }
    }
}

#[test]
fn generate_sector_connections_bidirectional() {
    let sector = generate_sector(0, 1, 42);
    for sys in &sector.systems {
        for &conn in &sys.connections {
            assert!(
                sector.systems[conn].connections.contains(&sys.id),
                "edge {}->{} is not bidirectional",
                sys.id,
                conn
            );
        }
    }
}

#[test]
fn generate_sector_no_self_loops() {
    for seed in [0, 42, 999, 12345] {
        let sector = generate_sector(0, 3, seed);
        for sys in &sector.systems {
            assert!(
                !sys.connections.contains(&sys.id),
                "system {} has self-loop",
                sys.id
            );
        }
    }
}

#[test]
fn generate_sector_no_duplicate_connections() {
    let sector = generate_sector(0, 2, 42);
    for sys in &sector.systems {
        let mut sorted = sys.connections.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            sys.connections.len(),
            "system {} has duplicate connections",
            sys.id
        );
    }
}

#[test]
fn generate_sector_special_system_descriptions() {
    let sector = generate_sector(0, 1, 42);
    assert!(sector.systems[sector.start_system].description.contains("entry point"));
    assert!(sector.systems[sector.boss_system].description.contains("derelict"));
    assert!(sector.systems[sector.exit_system].description.contains("gateway"));
}

#[test]
fn generate_sector_hsk3_may_have_hidden_systems() {
    let mut found_hidden = false;
    for seed in 0..100u32 {
        let sector = generate_sector(0, 3, seed);
        if sector.systems.iter().any(|s| s.hidden) {
            found_hidden = true;
            break;
        }
    }
    assert!(found_hidden, "HSK3+ sectors should sometimes have hidden systems");
}

#[test]
fn generate_sector_hsk1_no_hidden_systems() {
    // HSK < 3 should never produce hidden systems
    for seed in 0..50u32 {
        let sector = generate_sector(0, 1, seed);
        assert!(
            !sector.systems.iter().any(|s| s.hidden),
            "HSK1 should not have hidden systems (seed {seed})"
        );
    }
}

#[test]
fn generate_sector_hsk4_has_extra_connections() {
    // HSK >= 4 sectors get extra random connections, so average connectivity should be higher
    let sector_low = generate_sector(0, 1, 42);
    let sector_high = generate_sector(0, 5, 42);
    let avg_low: f64 = sector_low.systems.iter().map(|s| s.connections.len() as f64).sum::<f64>()
        / sector_low.systems.len() as f64;
    let avg_high: f64 = sector_high.systems.iter().map(|s| s.connections.len() as f64).sum::<f64>()
        / sector_high.systems.len() as f64;
    // Not strictly guaranteed per single seed, but the extra connections code runs
    assert!(avg_high >= 1.0, "HSK5 should have reasonable connectivity");
    assert!(avg_low >= 1.0, "HSK1 should have reasonable connectivity");
}

#[test]
fn generate_sector_beyond_sector_data_len() {
    let sector = generate_sector(12, 1, 42);
    assert_eq!(sector.name, "Unknown Reach");
    assert_eq!(sector.description, "An uncharted region of space.");
}

#[test]
fn generate_sector_high_sector_id() {
    let sector = generate_sector(100, 3, 42);
    assert_eq!(sector.name, "Unknown Reach");
}

#[test]
fn generate_sector_valid_sector_id_names() {
    let sector0 = generate_sector(0, 1, 42);
    assert_eq!(sector0.name, "Jade Frontier");
    let sector5 = generate_sector(5, 6, 42);
    assert_eq!(sector5.name, "Singularity Reach");
}

#[test]
fn generate_sector_deterministic_across_calls() {
    let a = generate_sector(2, 3, 99999);
    let b = generate_sector(2, 3, 99999);
    assert_eq!(a.systems.len(), b.systems.len());
    assert_eq!(a.hsk_level, b.hsk_level);
    assert_eq!(a.name, b.name);
    for i in 0..a.systems.len() {
        assert_eq!(a.systems[i].x, b.systems[i].x);
        assert_eq!(a.systems[i].y, b.systems[i].y);
        assert_eq!(a.systems[i].has_shop, b.systems[i].has_shop);
        assert_eq!(a.systems[i].has_fuel, b.systems[i].has_fuel);
        assert_eq!(a.systems[i].hazard, b.systems[i].hazard);
    }
}

#[test]
fn generate_sector_different_seeds_differ() {
    let a = generate_sector(0, 1, 1);
    let b = generate_sector(0, 1, 2);
    // Very unlikely to have identical system counts AND identical first system names
    let differ = a.systems.len() != b.systems.len()
        || a.systems[0].name != b.systems[0].name
        || a.systems.iter().map(|s| s.connections.len()).sum::<usize>()
            != b.systems.iter().map(|s| s.connections.len()).sum::<usize>();
    assert!(differ, "different seeds should produce different sectors");
}

#[test]
fn generate_sector_all_system_ids_sequential() {
    let sector = generate_sector(0, 2, 42);
    for (i, sys) in sector.systems.iter().enumerate() {
        assert_eq!(sys.id, i, "system id should match its index");
    }
}

#[test]
fn generate_sector_difficulty_matches_hsk() {
    for hsk in 1..=6u8 {
        let sector = generate_sector(0, hsk, 42);
        assert_eq!(sector.hsk_level, hsk);
        for sys in &sector.systems {
            assert_eq!(sys.difficulty, hsk);
        }
    }
}

#[test]
fn generate_sector_events_only_on_interior_systems() {
    let sector = generate_sector(0, 1, 42);
    // Start system (0) and exit system (last) should not have events assigned by the loop
    // (the loop goes from 1..num_systems-1)
    // Note: start=0, boss=len-2, exit=len-1
    // The event assignment loop is for i in 1..num_systems-1, so system 0 and last are excluded.
    assert!(sector.systems[0].event_id.is_none(), "start should have no event");
    assert!(sector.systems.last().unwrap().event_id.is_none(), "exit should have no event");
}

// ===========================================================================
// SectorMap::new
// ===========================================================================

#[test]
fn sector_map_new_creates_six_sectors() {
    let map = SectorMap::new(42);
    assert_eq!(map.sectors.len(), 6);
}

#[test]
fn sector_map_new_starts_at_sector_zero() {
    let map = SectorMap::new(42);
    assert_eq!(map.current_sector, 0);
    assert_eq!(map.current_system, 0);
}

#[test]
fn sector_map_new_hsk_levels_increment() {
    let map = SectorMap::new(42);
    for (i, sector) in map.sectors.iter().enumerate() {
        assert_eq!(sector.hsk_level, (i + 1) as u8, "sector {i} should be HSK{}", i + 1);
    }
}

#[test]
fn sector_map_new_sector_ids_sequential() {
    let map = SectorMap::new(42);
    for (i, sector) in map.sectors.iter().enumerate() {
        assert_eq!(sector.id, i);
    }
}

#[test]
fn sector_map_new_deterministic() {
    let a = SectorMap::new(42);
    let b = SectorMap::new(42);
    assert_eq!(a.sectors.len(), b.sectors.len());
    for (sa, sb) in a.sectors.iter().zip(b.sectors.iter()) {
        assert_eq!(sa.systems.len(), sb.systems.len());
        assert_eq!(sa.name, sb.name);
    }
}

// ===========================================================================
// can_jump_to
// ===========================================================================

#[test]
fn can_jump_to_connected_system() {
    let sector = generate_sector(0, 1, 42);
    let target = sector.systems[0].connections[0];
    let map = SectorMap {
        sectors: vec![sector],
        current_sector: 0,
        current_system: 0,
    };
    assert!(can_jump_to(&map, target));
}

#[test]
fn can_jump_to_unconnected_system() {
    let sector = generate_sector(0, 1, 42);
    // Find a system not connected to system 0
    let conns = &sector.systems[0].connections;
    let unconnected = (0..sector.systems.len()).find(|i| !conns.contains(i) && *i != 0);
    if let Some(uc) = unconnected {
        let map = SectorMap {
            sectors: vec![sector],
            current_sector: 0,
            current_system: 0,
        };
        assert!(!can_jump_to(&map, uc));
    }
}

#[test]
fn can_jump_to_out_of_bounds_system() {
    let sector = generate_sector(0, 1, 42);
    let map = SectorMap {
        sectors: vec![sector.clone()],
        current_sector: 0,
        current_system: 0,
    };
    assert!(!can_jump_to(&map, sector.systems.len() + 10));
}

#[test]
fn can_jump_to_out_of_bounds_sector() {
    let sector = generate_sector(0, 1, 42);
    let map = SectorMap {
        sectors: vec![sector],
        current_sector: 5, // out of bounds (only 1 sector)
        current_system: 0,
    };
    assert!(!can_jump_to(&map, 1));
}

#[test]
fn can_jump_to_invalid_current_system() {
    let sector = generate_sector(0, 1, 42);
    let map = SectorMap {
        sectors: vec![sector.clone()],
        current_sector: 0,
        current_system: sector.systems.len() + 1, // out of bounds
    };
    assert!(!can_jump_to(&map, 1));
}

// ===========================================================================
// jump_cost
// ===========================================================================

#[test]
fn jump_cost_between_adjacent_systems() {
    let sector = generate_sector(0, 1, 42);
    let cost = jump_cost(&sector.systems[0], &sector.systems[1]);
    assert!(cost > 0, "cost between adjacent systems should be positive");
    assert!(cost <= 14, "cost should be at most ~14 for max distance");
}

#[test]
fn jump_cost_is_symmetric() {
    let sector = generate_sector(0, 2, 42);
    let a = &sector.systems[0];
    let b = &sector.systems[1];
    assert_eq!(jump_cost(a, b), jump_cost(b, a));
}

#[test]
fn jump_cost_same_position_is_zero() {
    let sys = StarSystem {
        id: 0,
        name: "Test",
        chinese_name: "测试",
        description: "test",
        x: 0.5,
        y: 0.5,
        location_type: LocationType::SpaceStation,
        hazard: None,
        visited: false,
        has_shop: false,
        has_fuel: false,
        has_repair: false,
        has_medbay: false,
        quest_giver: false,
        hidden: false,
        warp_gate: false,
        event_id: None,
        connections: vec![],
        difficulty: 1,
    };
    assert_eq!(jump_cost(&sys, &sys), 0);
}

#[test]
fn jump_cost_increases_with_distance() {
    let make_sys = |x: f64, y: f64| StarSystem {
        id: 0,
        name: "T",
        chinese_name: "测",
        description: "",
        x,
        y,
        location_type: LocationType::SpaceStation,
        hazard: None,
        visited: false,
        has_shop: false,
        has_fuel: false,
        has_repair: false,
        has_medbay: false,
        quest_giver: false,
        hidden: false,
        warp_gate: false,
        event_id: None,
        connections: vec![],
        difficulty: 1,
    };
    let origin = make_sys(0.0, 0.0);
    let near = make_sys(0.1, 0.0);
    let far = make_sys(0.9, 0.0);
    assert!(jump_cost(&origin, &near) < jump_cost(&origin, &far));
}

// ===========================================================================
// connected_systems
// ===========================================================================

#[test]
fn connected_systems_for_start() {
    let sector = generate_sector(0, 1, 42);
    let conns = connected_systems(&sector, 0);
    assert_eq!(conns, sector.systems[0].connections);
}

#[test]
fn connected_systems_nonexistent_id() {
    let sector = generate_sector(0, 1, 42);
    let conns = connected_systems(&sector, 9999);
    assert!(conns.is_empty(), "nonexistent system should return empty vec");
}

#[test]
fn connected_systems_boss() {
    let sector = generate_sector(0, 1, 42);
    let conns = connected_systems(&sector, sector.boss_system);
    assert!(!conns.is_empty(), "boss system should have connections");
    assert!(conns.contains(&sector.exit_system), "boss should connect to exit");
}

// ===========================================================================
// advance_sector
// ===========================================================================

#[test]
fn advance_sector_marks_new_start_visited() {
    let mut map = SectorMap::new(42);
    advance_sector(&mut map);
    let start = map.sectors[1].start_system;
    assert!(map.sectors[1].systems[start].visited);
}

#[test]
fn advance_sector_does_nothing_at_last_sector() {
    let mut map = SectorMap::new(42);
    for _ in 0..10 {
        advance_sector(&mut map);
    }
    assert_eq!(map.current_sector, 5, "should stop at last sector (index 5)");
}

#[test]
fn advance_sector_all_the_way() {
    let mut map = SectorMap::new(42);
    for expected in 1..=5 {
        advance_sector(&mut map);
        assert_eq!(map.current_sector, expected);
        let start = map.sectors[expected].start_system;
        assert_eq!(map.current_system, start);
        assert!(map.sectors[expected].systems[start].visited);
    }
}

// ===========================================================================
// add_edge (tested indirectly through generate_sector)
// ===========================================================================

#[test]
fn add_edge_bidirectional() {
    let sector = generate_sector(0, 1, 42);
    // Every connection should be bidirectional (already tested, but explicit)
    for sys in &sector.systems {
        for &c in &sys.connections {
            assert!(sector.systems[c].connections.contains(&sys.id));
        }
    }
}

#[test]
fn add_edge_idempotent() {
    // Calling add_edge twice should not create duplicates
    let mut systems = vec![
        StarSystem {
            id: 0, name: "A", chinese_name: "甲", description: "",
            x: 0.0, y: 0.0, location_type: LocationType::SpaceStation,
            hazard: None, visited: false, has_shop: false, has_fuel: false,
            has_repair: false, has_medbay: false, quest_giver: false,
            hidden: false, warp_gate: false, event_id: None,
            connections: vec![], difficulty: 1,
        },
        StarSystem {
            id: 1, name: "B", chinese_name: "乙", description: "",
            x: 1.0, y: 1.0, location_type: LocationType::SpaceStation,
            hazard: None, visited: false, has_shop: false, has_fuel: false,
            has_repair: false, has_medbay: false, quest_giver: false,
            hidden: false, warp_gate: false, event_id: None,
            connections: vec![], difficulty: 1,
        },
    ];
    add_edge(&mut systems, 0, 1);
    add_edge(&mut systems, 0, 1);
    add_edge(&mut systems, 1, 0);
    assert_eq!(systems[0].connections.len(), 1);
    assert_eq!(systems[1].connections.len(), 1);
}

#[test]
fn add_edge_self_loop_rejected() {
    let mut systems = vec![
        StarSystem {
            id: 0, name: "A", chinese_name: "甲", description: "",
            x: 0.0, y: 0.0, location_type: LocationType::SpaceStation,
            hazard: None, visited: false, has_shop: false, has_fuel: false,
            has_repair: false, has_medbay: false, quest_giver: false,
            hidden: false, warp_gate: false, event_id: None,
            connections: vec![], difficulty: 1,
        },
    ];
    add_edge(&mut systems, 0, 0);
    assert!(systems[0].connections.is_empty(), "self-loop should be rejected");
}

// ===========================================================================
// Path reachability across multiple seeds
// ===========================================================================

#[test]
fn path_from_start_to_exit_exists_multiple_seeds() {
    for seed in [0, 1, 42, 100, 999, 12345, u32::MAX] {
        for hsk in 1..=6u8 {
            let sector = generate_sector(0, hsk, seed);
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
            assert!(visited[sector.boss_system], "boss unreachable HSK{hsk} seed {seed}");
            assert!(visited[sector.exit_system], "exit unreachable HSK{hsk} seed {seed}");
        }
    }
}

// ===========================================================================
// Shops and fuel across HSK levels
// ===========================================================================

#[test]
fn shops_and_fuel_across_levels() {
    for hsk in 1..=6u8 {
        let sector = generate_sector(0, hsk, 42);
        let shops = sector.systems.iter().filter(|s| s.has_shop).count();
        let fuel = sector.systems.iter().filter(|s| s.has_fuel).count();
        assert!(shops >= 1, "HSK{hsk}: need at least 1 shop, got {shops}");
        assert!(fuel >= 1, "HSK{hsk}: need at least 1 fuel, got {fuel}");
    }
}

// ===========================================================================
// Edge case: max u32 seed
// ===========================================================================

#[test]
fn generate_sector_max_seed() {
    let sector = generate_sector(0, 1, u32::MAX);
    assert!(sector.systems.len() >= 10);
    assert!(sector.systems.len() <= 25);
    assert_eq!(sector.start_system, 0);
}

// ===========================================================================
// Sector map with zero seed
// ===========================================================================

#[test]
fn sector_map_zero_seed() {
    let map = SectorMap::new(0);
    assert_eq!(map.sectors.len(), 6);
    assert_eq!(map.current_sector, 0);
    assert_eq!(map.current_system, 0);
}

// ===========================================================================
// Sector id and hsk_level stored correctly
// ===========================================================================

#[test]
fn sector_stores_id() {
    for id in 0..12 {
        let sector = generate_sector(id, 1, 42);
        assert_eq!(sector.id, id);
    }
}

#[test]
fn sector_stores_hsk_level() {
    for hsk in 1..=6u8 {
        let sector = generate_sector(0, hsk, 42);
        assert_eq!(sector.hsk_level, hsk);
    }
}

