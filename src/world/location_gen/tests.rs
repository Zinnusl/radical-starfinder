use super::*;

pub(super) fn make_clean_test_level() -> LocationLevel {
    let width = 24;
    let height = 24;
    let mut level = LocationLevel {
        width,
        height,
        tiles: vec![Tile::Bulkhead; (width * height) as usize],
        rooms: vec![
            Room {
                x: 1,
                y: 1,
                w: 5,
                h: 5,
                modifier: None,
                special: None,
            },
            Room {
                x: 8,
                y: 1,
                w: 5,
                h: 5,
                modifier: None,
                special: None,
            },
            Room {
                x: 15,
                y: 1,
                w: 5,
                h: 5,
                modifier: None,
                special: None,
            },
            Room {
                x: 1,
                y: 10,
                w: 5,
                h: 5,
                modifier: None,
                special: None,
            },
            Room {
                x: 8,
                y: 10,
                w: 5,
                h: 5,
                modifier: None,
                special: None,
            },
        ],
        visible: vec![false; (width * height) as usize],
        revealed: vec![false; (width * height) as usize],
    };
    for room in level.rooms.clone() {
        for y in room.y..room.y + room.h {
            for x in room.x..room.x + room.w {
                let idx = level.idx(x, y);
                level.tiles[idx] = Tile::MetalFloor;
            }
        }
    }
    level
}

pub(super) fn make_spacious_test_level() -> LocationLevel {
    let width = 40;
    let height = 28;
    let mut level = LocationLevel {
        width,
        height,
        tiles: vec![Tile::Bulkhead; (width * height) as usize],
        rooms: vec![
            Room {
                x: 1,
                y: 1,
                w: 8,
                h: 8,
                modifier: None,
                special: None,
            },
            Room {
                x: 11,
                y: 1,
                w: 8,
                h: 8,
                modifier: None,
                special: None,
            },
            Room {
                x: 21,
                y: 1,
                w: 8,
                h: 8,
                modifier: None,
                special: None,
            },
            Room {
                x: 1,
                y: 12,
                w: 8,
                h: 8,
                modifier: None,
                special: None,
            },
            Room {
                x: 11,
                y: 12,
                w: 8,
                h: 8,
                modifier: None,
                special: None,
            },
        ],
        visible: vec![false; (width * height) as usize],
        revealed: vec![false; (width * height) as usize],
    };
    for room in level.rooms.clone() {
        for y in room.y..room.y + room.h {
            for x in room.x..room.x + room.w {
                let idx = level.idx(x, y);
                level.tiles[idx] = Tile::MetalFloor;
            }
        }
    }
    level
}

pub(super) fn has_pushable_bridge_setup(level: &LocationLevel) -> bool {
    let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    for y in 0..level.height {
        for x in 0..level.width {
            if level.tile(x, y) != Tile::CoolantPool {
                continue;
            }

            for (dx, dy) in dirs {
                let crate_x = x - dx;
                let crate_y = y - dy;
                let stand_x = crate_x - dx;
                let stand_y = crate_y - dy;
                if !level.in_bounds(crate_x, crate_y) || !level.in_bounds(stand_x, stand_y) {
                    continue;
                }

                if level.tile(crate_x, crate_y) == Tile::SalvageCrate {
                    let stand_tile = level.tile(stand_x, stand_y);
                    if stand_tile.is_walkable() && stand_tile != Tile::CoolantPool {
                        return true;
                    }
                }
            }
        }
    }

    false
}

pub(super) fn has_brittle_vault(level: &LocationLevel) -> bool {
    let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    for y in 0..level.height {
        for x in 0..level.width {
            if level.tile(x, y) != Tile::WeakBulkhead {
                continue;
            }

            for (dx, dy) in dirs {
                let chest_x = x + dx;
                let chest_y = y + dy;
                let back_x = chest_x + dx;
                let back_y = chest_y + dy;
                let side_a = (-dy, dx);
                let side_b = (dy, -dx);
                if !level.in_bounds(chest_x, chest_y) || !level.in_bounds(back_x, back_y) {
                    continue;
                }

                if level.tile(chest_x, chest_y) == Tile::SupplyCrate
                    && level.tile(back_x, back_y) == Tile::Bulkhead
                    && level.tile(x + side_a.0, y + side_a.1) == Tile::Bulkhead
                    && level.tile(chest_x + side_a.0, chest_y + side_a.1) == Tile::Bulkhead
                    && level.tile(x + side_b.0, y + side_b.1) == Tile::Bulkhead
                    && level.tile(chest_x + side_b.0, chest_y + side_b.1) == Tile::Bulkhead
                {
                    return true;
                }
            }
        }
    }

    false
}

pub(super) fn has_deep_water_cache(level: &LocationLevel) -> bool {
    let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    for y in 0..level.height {
        for x in 0..level.width {
            if level.tile(x, y) != Tile::VacuumBreach {
                continue;
            }

            for (dx, dy) in dirs {
                let crate_x = x - dx;
                let crate_y = y - dy;
                let stand_x = crate_x - dx;
                let stand_y = crate_y - dy;
                let chest_x = x + dx;
                let chest_y = y + dy;
                let back_x = chest_x + dx;
                let back_y = chest_y + dy;
                let side_a = (-dy, dx);
                let side_b = (dy, -dx);
                if !level.in_bounds(crate_x, crate_y)
                    || !level.in_bounds(stand_x, stand_y)
                    || !level.in_bounds(chest_x, chest_y)
                    || !level.in_bounds(back_x, back_y)
                {
                    continue;
                }

                if level.tile(crate_x, crate_y) == Tile::SalvageCrate
                    && level.tile(stand_x, stand_y) == Tile::MetalFloor
                    && level.tile(chest_x, chest_y) == Tile::SupplyCrate
                    && level.tile(back_x, back_y) == Tile::Bulkhead
                    && level.tile(x + side_a.0, y + side_a.1) == Tile::Bulkhead
                    && level.tile(chest_x + side_a.0, chest_y + side_a.1) == Tile::Bulkhead
                    && level.tile(x + side_b.0, y + side_b.1) == Tile::Bulkhead
                    && level.tile(chest_x + side_b.0, chest_y + side_b.1) == Tile::Bulkhead
                {
                    return true;
                }
            }
        }
    }

    false
}

pub(super) fn has_spike_bridge(level: &LocationLevel) -> bool {
    for y in 0..level.height {
        for x in 0..level.width - 3 {
            if level.tile(x, y) == Tile::LaserGrid
                && level.tile(x + 1, y) == Tile::LaserGrid
                && level.tile(x + 2, y) == Tile::LaserGrid
                && level.tile(x + 3, y) == Tile::SupplyCrate
            {
                return true;
            }
        }
    }
    false
}

pub(super) fn has_oil_fire_trap(level: &LocationLevel) -> bool {
    for y in 0..level.height {
        for x in 0..level.width - 2 {
            if level.tile(x, y) == Tile::Coolant
                && level.tile(x + 1, y) == Tile::Coolant
                && level.tile(x + 2, y) == Tile::SupplyCrate
            {
                return true;
            }
        }
    }
    false
}

pub(super) fn has_seal_chain(level: &LocationLevel) -> bool {
    for y in 0..level.height {
        for x in 0..level.width - 2 {
            if matches!(level.tile(x, y), Tile::SecurityLock(_))
                && matches!(level.tile(x + 2, y), Tile::SecurityLock(_))
            {
                return true;
            }
        }
    }
    false
}

pub(super) fn has_any_puzzle_room(level: &LocationLevel) -> bool {
    has_brittle_vault(level)
        || has_deep_water_cache(level)
        || has_spike_bridge(level)
        || has_oil_fire_trap(level)
        || has_seal_chain(level)
}

#[test]
pub(super) fn hazards_and_altars_are_walkable_but_crates_block() {
    assert!(Tile::LaserGrid.is_walkable());
    assert!(Tile::Coolant.is_walkable());
    assert!(Tile::CoolantPool.is_walkable());
    assert!(Tile::Terminal(AltarKind::Quantum).is_walkable());
    assert!(Tile::SecurityLock(SealKind::Thermal).is_walkable());
    assert!(!Tile::SalvageCrate.is_walkable());
    assert!(!Tile::DamagedBulkhead.is_walkable());
    assert!(!Tile::WeakBulkhead.is_walkable());
    assert!(!Tile::VacuumBreach.is_walkable());
}

#[test]
pub(super) fn place_altars_adds_a_blessing_site_to_clean_levels() {
    let mut level = make_clean_test_level();
    let mut rng = Rng::new(2);

    level.place_altars(&mut rng);

    assert!(level
        .tiles
        .iter()
        .any(|tile| matches!(tile, Tile::Terminal(_))));
}

#[test]
pub(super) fn place_seals_adds_script_seals_to_clean_levels() {
    let mut level = make_clean_test_level();
    let mut rng = Rng::new(7);

    level.place_seals(&mut rng);

    assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::SecurityLock(_))));
}

#[test]
pub(super) fn place_secret_room_carves_hidden_chamber_with_cracked_entrance() {
    let mut level = make_clean_test_level();
    let mut rng = Rng::new(11);
    let original_open_tiles = level
        .tiles
        .iter()
        .filter(|tile| !matches!(tile, Tile::Bulkhead))
        .count();

    level.place_secret_room(&mut rng);

    let new_open_tiles = level
        .tiles
        .iter()
        .filter(|tile| !matches!(tile, Tile::Bulkhead))
        .count();
    assert!(level
        .tiles
        .iter()
        .any(|tile| matches!(tile, Tile::DamagedBulkhead)));
    assert!(new_open_tiles > original_open_tiles);
}

#[test]
pub(super) fn place_secret_room_adds_hidden_point_of_interest() {
    let mut level = make_clean_test_level();
    let mut rng = Rng::new(11);

    level.place_secret_room(&mut rng);

    assert!(level.tiles.iter().any(|tile| {
        matches!(
            tile,
            Tile::SupplyCrate | Tile::QuantumForge | Tile::NavBeacon | Tile::Terminal(_)
        )
    }));
}

#[test]
pub(super) fn generated_levels_hide_secret_rooms_on_most_runs() {
    let mut secret_count = 0;
    for seed in 1..=24 {
        let level = LocationLevel::generate(48, 48, seed, 1, LocationType::SpaceStation);
        if level
            .tiles
            .iter()
            .any(|tile| matches!(tile, Tile::DamagedBulkhead))
        {
            secret_count += 1;
        }
    }

    assert!(
        secret_count >= 18,
        "expected secret-room entrances on most sample floors, found {secret_count}"
    );
}

#[test]
pub(super) fn generated_levels_regularly_offer_bridge_building_setups() {
    let mut bridge_count = 0;
    for seed in 1..=24 {
        let level = LocationLevel::generate(48, 48, seed, 1, LocationType::SpaceStation);
        if has_pushable_bridge_setup(&level) {
            bridge_count += 1;
        }
    }

    assert!(
        bridge_count >= 10,
        "expected bridge setups across the sample set, found {bridge_count}"
    );
}

#[test]
pub(super) fn place_puzzle_rooms_adds_visible_environmental_niches() {
    let mut level = make_spacious_test_level();
    let mut rng = Rng::new(19);

    level.place_puzzle_rooms(&mut rng);

    assert!(has_any_puzzle_room(&level));
}

#[test]
pub(super) fn generated_levels_regularly_offer_puzzle_rooms() {
    let mut puzzle_count = 0;
    let mut brittle_count = 0;
    let mut deep_water_count = 0;
    let mut spike_count = 0;
    let mut oil_count = 0;
    let mut seal_count = 0;
    for seed in 1..=24 {
        let level = LocationLevel::generate(48, 48, seed, 1, LocationType::SpaceStation);
        if has_any_puzzle_room(&level) {
            puzzle_count += 1;
        }
        if has_brittle_vault(&level) {
            brittle_count += 1;
        }
        if has_deep_water_cache(&level) {
            deep_water_count += 1;
        }
        if has_spike_bridge(&level) {
            spike_count += 1;
        }
        if has_oil_fire_trap(&level) {
            oil_count += 1;
        }
        if has_seal_chain(&level) {
            seal_count += 1;
        }
    }

    assert!(
        puzzle_count >= 16,
        "expected puzzle rooms on most sample floors, found {puzzle_count}"
    );
    let variant_types = [
        brittle_count > 0,
        deep_water_count > 0,
        spike_count > 0,
        oil_count > 0,
        seal_count > 0,
    ];
    let variants_seen = variant_types.iter().filter(|&&v| v).count();
    assert!(
        variants_seen >= 2,
        "expected at least 2 puzzle room variants across 24 seeds, saw {variants_seen}"
    );
}

#[test]
pub(super) fn tutorial_floor_has_required_landmarks() {
    let level = LocationLevel::tutorial(48, 48);

    assert_eq!(level.start_pos(), (8, 22));
    assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::InfoPanel(0))));
    assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::InfoPanel(1))));
    assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::InfoPanel(2))));
    assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::InfoPanel(3))));
    assert!(level.tiles.iter().any(|tile| *tile == Tile::QuantumForge));
    assert!(level.tiles.iter().any(|tile| *tile == Tile::Airlock));
    assert!(level.is_walkable(8, 20));
}

