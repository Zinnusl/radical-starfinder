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

