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

// ---------------------------------------------------------------------------
// Special room generation tests — exercise every SpecialRoomKind match arm
// ---------------------------------------------------------------------------

/// Helper: create a level with a single large room filled with MetalFloor,
/// call generate_special_room for the given kind, return the level.
fn run_special_room(kind: SpecialRoomKind, seed: u64) -> LocationLevel {
    let width = 30;
    let height = 20;
    let room = Room {
        x: 2,
        y: 2,
        w: 16,
        h: 12,
        modifier: None,
        special: Some(kind),
    };
    let mut level = LocationLevel {
        width,
        height,
        tiles: vec![Tile::Bulkhead; (width * height) as usize],
        rooms: vec![room.clone()],
        visible: vec![false; (width * height) as usize],
        revealed: vec![false; (width * height) as usize],
    };
    // Carve the room interior to MetalFloor
    for y in room.y..room.y + room.h {
        for x in room.x..room.x + room.w {
            let idx = level.idx(x, y);
            level.tiles[idx] = Tile::MetalFloor;
        }
    }
    let mut rng = Rng::new(seed);
    level.generate_special_room(kind, &room, &mut rng);
    level
}

/// Check that at least one tile in the room differs from MetalFloor,
/// meaning the special room generation actually placed something.
fn assert_room_modified(level: &LocationLevel, room_x: i32, room_y: i32, room_w: i32, room_h: i32) {
    let mut has_non_floor = false;
    for y in room_y..room_y + room_h {
        for x in room_x..room_x + room_w {
            if level.in_bounds(x, y) && level.tile(x, y) != Tile::MetalFloor {
                has_non_floor = true;
                break;
            }
        }
        if has_non_floor { break; }
    }
    assert!(has_non_floor, "special room should modify at least one tile");
}

fn contains_tile(level: &LocationLevel, target: Tile) -> bool {
    level.tiles.iter().any(|t| *t == target)
}

fn count_tile(level: &LocationLevel, target: Tile) -> usize {
    level.tiles.iter().filter(|t| **t == target).count()
}

#[test]
fn special_room_cargo_bay() {
    let level = run_special_room(SpecialRoomKind::CargoBay, 1);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::CreditCache) || contains_tile(&level, Tile::CoolantPool));
    assert!(contains_tile(&level, Tile::SupplyCrate));
    assert!(count_tile(&level, Tile::SalvageCrate) > 0 || count_tile(&level, Tile::CoolantPool) > 0);
}

#[test]
fn special_room_ore_deposit() {
    let level = run_special_room(SpecialRoomKind::OreDeposit, 2);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::OreVein));
    assert!(contains_tile(&level, Tile::CargoCrate) || contains_tile(&level, Tile::CreditCache));
}

#[test]
fn special_room_hall_of_records() {
    let level = run_special_room(SpecialRoomKind::HallOfRecords, 3);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::CrystalPanel));
    assert!(contains_tile(&level, Tile::DataRack));
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Quantum)));
}

#[test]
fn special_room_hidden_cache() {
    let level = run_special_room(SpecialRoomKind::HiddenCache, 4);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::SupplyCrate));
    assert!(contains_tile(&level, Tile::ToxicFungus) || contains_tile(&level, Tile::Coolant));
}

#[test]
fn special_room_offering_terminal() {
    let level = run_special_room(SpecialRoomKind::OfferingTerminal, 5);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Commerce)));
    assert!(contains_tile(&level, Tile::CoolantPool));
    assert!(contains_tile(&level, Tile::CrystalPanel));
}

#[test]
fn special_room_hydroponics_garden() {
    let level = run_special_room(SpecialRoomKind::HydroponicsGarden, 6);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::CargoPipes));
    assert!(contains_tile(&level, Tile::CoolantPool));
    assert!(contains_tile(&level, Tile::MedBayTile));
}

#[test]
fn special_room_pirate_stash() {
    let level = run_special_room(SpecialRoomKind::PirateStash, 7);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::OreVein) || contains_tile(&level, Tile::CreditCache));
    assert!(contains_tile(&level, Tile::CrystalPanel));
}

#[test]
fn special_room_xeno_crystal_vault() {
    let level = run_special_room(SpecialRoomKind::XenoCrystalVault, 8);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::CrystalPanel));
    assert!(contains_tile(&level, Tile::SupplyCrate));
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Quantum)));
}

#[test]
fn special_room_relic_chamber() {
    let level = run_special_room(SpecialRoomKind::RelicChamber, 9);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::DataRack));
    assert!(contains_tile(&level, Tile::SupplyCrate));
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Holographic)));
}

#[test]
fn special_room_med_bay() {
    let level = run_special_room(SpecialRoomKind::MedBay, 10);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::MedBayTile));
    assert!(contains_tile(&level, Tile::CoolantPool));
    assert!(contains_tile(&level, Tile::CrystalPanel));
}

#[test]
fn special_room_arena_challenge() {
    let level = run_special_room(SpecialRoomKind::ArenaChallenge, 11);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::LaserGrid));
    assert!(contains_tile(&level, Tile::CargoCrate));
    assert!(contains_tile(&level, Tile::PressureSensor));
}

#[test]
fn special_room_security_checkpoint() {
    let level = run_special_room(SpecialRoomKind::SecurityCheckpoint, 12);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::LaserGrid));
    assert!(contains_tile(&level, Tile::SalvageCrate));
    assert!(contains_tile(&level, Tile::PressureSensor));
}

#[test]
fn special_room_security_zone() {
    let level = run_special_room(SpecialRoomKind::SecurityZone, 13);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::LaserGrid));
    assert!(contains_tile(&level, Tile::CargoCrate));
    assert!(contains_tile(&level, Tile::PressureSensor));
    assert!(contains_tile(&level, Tile::SupplyCrate));
}

#[test]
fn special_room_toxic_maze() {
    let level = run_special_room(SpecialRoomKind::ToxicMaze, 14);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::ToxicGas));
    assert!(contains_tile(&level, Tile::SupplyCrate));
}

#[test]
fn special_room_gravity_puzzle() {
    let level = run_special_room(SpecialRoomKind::GravityPuzzle, 15);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::PressureSensor));
    assert!(contains_tile(&level, Tile::CargoCrate));
    assert!(contains_tile(&level, Tile::CrystalPanel));
    assert!(contains_tile(&level, Tile::SupplyCrate));
}

#[test]
fn special_room_holographic_room() {
    let level = run_special_room(SpecialRoomKind::HolographicRoom, 16);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::FrozenDeck));
    assert!(contains_tile(&level, Tile::CrystalPanel));
    assert!(contains_tile(&level, Tile::HoloPool));
}

#[test]
fn special_room_energy_nexus() {
    let level = run_special_room(SpecialRoomKind::EnergyNexus, 17);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Quantum)));
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Stellar)));
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Tactical)));
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Holographic)));
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Commerce)));
    assert!(contains_tile(&level, Tile::CoolantPool));
    assert!(contains_tile(&level, Tile::PlasmaVent));
}

#[test]
fn special_room_space_hulk_rise() {
    let level = run_special_room(SpecialRoomKind::SpaceHulkRise, 18);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::LaserGrid));
    assert!(contains_tile(&level, Tile::CorruptedFloor));
}

#[test]
fn special_room_pirate_ambush() {
    let level = run_special_room(SpecialRoomKind::PirateAmbush, 19);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::SalvageCrate));
    assert!(contains_tile(&level, Tile::CreditCache));
    assert!(level.tiles.iter().any(|t| matches!(t, Tile::Trap(_))));
}

#[test]
fn special_room_duel_arena() {
    let level = run_special_room(SpecialRoomKind::DuelArena, 20);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::Coolant));
    assert!(contains_tile(&level, Tile::LaserGrid));
    assert!(contains_tile(&level, Tile::CargoCrate));
    assert!(contains_tile(&level, Tile::PressureSensor));
}

#[test]
fn special_room_data_archive() {
    let level = run_special_room(SpecialRoomKind::DataArchive, 21);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::DataRack));
    assert!(contains_tile(&level, Tile::CrystalPanel));
    assert!(contains_tile(&level, Tile::DataWell));
}

#[test]
fn special_room_signal_hall() {
    let level = run_special_room(SpecialRoomKind::SignalHall, 22);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::CrystalPanel));
    assert!(contains_tile(&level, Tile::DataRack));
    assert!(contains_tile(&level, Tile::DataWell));
}

#[test]
fn special_room_researcher_study() {
    let level = run_special_room(SpecialRoomKind::ResearcherStudy, 23);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::DataRack));
    assert!(contains_tile(&level, Tile::CodexTerminal));
}

#[test]
fn special_room_sensor_array() {
    let level = run_special_room(SpecialRoomKind::SensorArray, 24);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::HoloPool));
    assert!(contains_tile(&level, Tile::CrystalPanel));
    assert!(contains_tile(&level, Tile::CoolantPool));
}

#[test]
fn special_room_inscription_wall() {
    let level = run_special_room(SpecialRoomKind::InscriptionWall, 25);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::DataRack));
    assert!(contains_tile(&level, Tile::RadicalLab));
}

#[test]
fn special_room_training_simulator() {
    let level = run_special_room(SpecialRoomKind::TrainingSimulator, 26);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::Coolant));
    assert!(contains_tile(&level, Tile::SalvageCrate));
    assert!(contains_tile(&level, Tile::PressureSensor));
}

#[test]
fn special_room_zen_chamber() {
    let level = run_special_room(SpecialRoomKind::ZenChamber, 27);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::MedBayTile));
    assert!(contains_tile(&level, Tile::CoolantPool));
    assert!(contains_tile(&level, Tile::CrystalPanel));
    assert!(contains_tile(&level, Tile::CargoPipes));
}

#[test]
fn special_room_translation_challenge() {
    let level = run_special_room(SpecialRoomKind::TranslationChallenge, 28);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::TranslationTerminal));
    assert!(contains_tile(&level, Tile::DataRack));
    assert!(contains_tile(&level, Tile::DataWell));
}

#[test]
fn special_room_ancient_datapad() {
    let level = run_special_room(SpecialRoomKind::AncientDatapad, 29);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::SupplyCrate));
    assert!(contains_tile(&level, Tile::DataRack));
    assert!(contains_tile(&level, Tile::ToxicFungus));
}

#[test]
fn special_room_wisdom_core() {
    let level = run_special_room(SpecialRoomKind::WisdomCore, 30);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::VacuumBreach));
    assert!(contains_tile(&level, Tile::DroidTutor));
    assert!(contains_tile(&level, Tile::CoolantPool));
    assert!(contains_tile(&level, Tile::DataRack));
}

#[test]
fn special_room_flooded_compartment() {
    let level = run_special_room(SpecialRoomKind::FloodedCompartment, 31);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::CoolantPool) || contains_tile(&level, Tile::VacuumBreach));
    assert!(contains_tile(&level, Tile::CargoCrate));
}

#[test]
fn special_room_cryogenic_bay() {
    let level = run_special_room(SpecialRoomKind::CryogenicBay, 32);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::FrozenDeck));
    assert!(contains_tile(&level, Tile::CrystalPanel));
    assert!(contains_tile(&level, Tile::CargoCrate));
}

#[test]
fn special_room_plasma_crossing() {
    let level = run_special_room(SpecialRoomKind::PlasmaCrossing, 33);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::PlasmaVent));
    assert!(contains_tile(&level, Tile::CargoCrate));
    assert!(contains_tile(&level, Tile::SupplyCrate));
}

#[test]
fn special_room_pipe_forest() {
    let level = run_special_room(SpecialRoomKind::PipeForest, 34);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::CargoPipes));
    assert!(contains_tile(&level, Tile::CrystalPanel));
}

#[test]
fn special_room_vent_tunnel() {
    let level = run_special_room(SpecialRoomKind::VentTunnel, 35);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::LaserGrid));
    assert!(contains_tile(&level, Tile::PressureSensor));
    assert!(contains_tile(&level, Tile::CrystalPanel));
}

#[test]
fn special_room_crystal_cave() {
    let level = run_special_room(SpecialRoomKind::CrystalCave, 36);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::CrystalPanel));
    assert!(contains_tile(&level, Tile::FrozenDeck));
}

#[test]
fn special_room_fungal_grotto() {
    let level = run_special_room(SpecialRoomKind::FungalGrotto, 37);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::ToxicFungus));
    assert!(contains_tile(&level, Tile::CrystalPanel));
}

#[test]
fn special_room_coolant_river() {
    let level = run_special_room(SpecialRoomKind::CoolantRiver, 38);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::VacuumBreach) || contains_tile(&level, Tile::CoolantPool));
    assert!(contains_tile(&level, Tile::Catwalk));
    assert!(contains_tile(&level, Tile::CrystalPanel));
}

#[test]
fn special_room_echoing_hull() {
    let level = run_special_room(SpecialRoomKind::EchoingHull, 39);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::CoolantPool));
    assert!(contains_tile(&level, Tile::CargoCrate));
    assert!(contains_tile(&level, Tile::FrozenDeck));
    assert!(contains_tile(&level, Tile::CrystalPanel));
}

#[test]
fn special_room_dark_sector() {
    let level = run_special_room(SpecialRoomKind::DarkSector, 40);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::CorruptedFloor));
    assert!(contains_tile(&level, Tile::CrystalPanel));
    assert!(level.tiles.iter().any(|t| matches!(t, Tile::Trap(_))));
}

#[test]
fn special_room_wandering_merchant() {
    let level = run_special_room(SpecialRoomKind::WanderingMerchant, 41);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::TradeTerminal));
    assert!(contains_tile(&level, Tile::DataRack));
    assert!(contains_tile(&level, Tile::SalvageCrate));
}

#[test]
fn special_room_hermit_sage() {
    let level = run_special_room(SpecialRoomKind::HermitSage, 42);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::Npc(1)));
    assert!(contains_tile(&level, Tile::DataRack));
    assert!(contains_tile(&level, Tile::ToxicFungus));
}

#[test]
fn special_room_detention_cell() {
    let level = run_special_room(SpecialRoomKind::DetentionCell, 43);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::Npc(3)));
    assert!(contains_tile(&level, Tile::SealedHatch));
    assert!(contains_tile(&level, Tile::LaserGrid) || contains_tile(&level, Tile::WeakBulkhead));
}

#[test]
fn special_room_memorial_shrine() {
    let level = run_special_room(SpecialRoomKind::MemorialShrine, 44);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::NavBeacon));
    assert!(contains_tile(&level, Tile::CargoPipes));
    assert!(contains_tile(&level, Tile::CoolantPool));
}

#[test]
fn special_room_cantina_bay() {
    let level = run_special_room(SpecialRoomKind::CantinaBay, 45);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::Npc(2)));
    assert!(contains_tile(&level, Tile::MedBayTile));
    assert!(contains_tile(&level, Tile::SalvageCrate));
}

#[test]
fn special_room_engineer_workshop() {
    let level = run_special_room(SpecialRoomKind::EngineerWorkshop, 46);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::QuantumForge));
    assert!(contains_tile(&level, Tile::OreVein));
    assert!(contains_tile(&level, Tile::PlasmaVent));
}

#[test]
fn special_room_chem_lab() {
    let level = run_special_room(SpecialRoomKind::ChemLab, 47);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::QuantumForge));
    assert!(contains_tile(&level, Tile::CoolantPool));
    assert!(contains_tile(&level, Tile::ToxicFungus));
    assert!(contains_tile(&level, Tile::DataRack));
}

#[test]
fn special_room_fortune_teller() {
    let level = run_special_room(SpecialRoomKind::FortuneTeller, 48);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::Npc(0)));
    assert!(contains_tile(&level, Tile::HoloPool));
    assert!(contains_tile(&level, Tile::CrystalPanel));
    assert!(contains_tile(&level, Tile::CargoPipes));
}

#[test]
fn special_room_refugee_bay() {
    let level = run_special_room(SpecialRoomKind::RefugeeBay, 49);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::MedBayTile));
    assert!(contains_tile(&level, Tile::SalvageCrate));
    assert!(level.tiles.iter().any(|t| matches!(t, Tile::Npc(_))));
}

#[test]
fn special_room_warp_gate() {
    let level = run_special_room(SpecialRoomKind::WarpGate, 50);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::WarpGatePortal));
    assert!(contains_tile(&level, Tile::CreditCache));
    assert!(contains_tile(&level, Tile::PlasmaVent));
    assert!(contains_tile(&level, Tile::CrystalPanel));
}

#[test]
fn special_room_gambling_den() {
    let level = run_special_room(SpecialRoomKind::GamblingDen, 51);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::SupplyCrate));
    assert!(contains_tile(&level, Tile::CreditCache));
}

#[test]
fn special_room_blood_terminal() {
    let level = run_special_room(SpecialRoomKind::BloodTerminal, 52);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Tactical)));
    assert!(contains_tile(&level, Tile::Coolant));
}

#[test]
fn special_room_cursed_salvage() {
    let level = run_special_room(SpecialRoomKind::CursedSalvage, 53);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::SupplyCrate));
    assert!(contains_tile(&level, Tile::CorruptedFloor));
}

#[test]
fn special_room_soul_forge() {
    let level = run_special_room(SpecialRoomKind::SoulForge, 54);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::QuantumForge));
    assert!(contains_tile(&level, Tile::CrystalPanel));
    assert!(contains_tile(&level, Tile::PlasmaVent));
}

#[test]
fn special_room_wishing_reactor() {
    let level = run_special_room(SpecialRoomKind::WishingReactor, 55);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::VacuumBreach));
    assert!(contains_tile(&level, Tile::CoolantPool));
    assert!(contains_tile(&level, Tile::CreditCache));
}

#[test]
fn special_room_cipher_gate() {
    let level = run_special_room(SpecialRoomKind::CipherGate, 56);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::PressureSensor));
    assert!(contains_tile(&level, Tile::SealedHatch));
}

#[test]
fn special_room_holo_maze() {
    let level = run_special_room(SpecialRoomKind::HoloMaze, 57);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::CrystalPanel));
    assert!(contains_tile(&level, Tile::SupplyCrate));
}

#[test]
fn special_room_gravity_plate() {
    let level = run_special_room(SpecialRoomKind::GravityPlate, 58);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::PressureSensor));
    assert!(contains_tile(&level, Tile::CargoCrate));
    assert!(contains_tile(&level, Tile::FrozenDeck));
    assert!(contains_tile(&level, Tile::SupplyCrate));
}

#[test]
fn special_room_tone_frequency() {
    let level = run_special_room(SpecialRoomKind::ToneFrequency, 59);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::NavBeacon));
    assert!(contains_tile(&level, Tile::SupplyCrate));
}

#[test]
fn special_room_elemental_lock() {
    let level = run_special_room(SpecialRoomKind::ElementalLock, 60);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::SealedHatch));
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Quantum)));
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Stellar)));
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Tactical)));
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Commerce)));
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Holographic)));
    assert!(contains_tile(&level, Tile::SupplyCrate));
}

#[test]
fn special_room_survival_bay() {
    let level = run_special_room(SpecialRoomKind::SurvivalBay, 61);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::LaserGrid));
}

#[test]
fn special_room_salvage_race() {
    let level = run_special_room(SpecialRoomKind::SalvageRace, 62);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::CreditCache));
}

#[test]
fn special_room_depressurizing_chamber() {
    let level = run_special_room(SpecialRoomKind::DepressurizingChamber, 63);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::LaserGrid));
    assert!(contains_tile(&level, Tile::SupplyCrate));
}

#[test]
fn special_room_nano_flood() {
    let level = run_special_room(SpecialRoomKind::NanoFlood, 64);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::Coolant));
    assert!(contains_tile(&level, Tile::DataWell));
    assert!(contains_tile(&level, Tile::SupplyCrate));
}

#[test]
fn special_room_form_shrine() {
    let level = run_special_room(SpecialRoomKind::FormShrine, 65);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::NavBeacon));
    assert!(contains_tile(&level, Tile::PlasmaVent));
    assert!(contains_tile(&level, Tile::CargoCrate));
    assert!(contains_tile(&level, Tile::CoolantPool));
    assert!(contains_tile(&level, Tile::PressureSensor));
}

#[test]
fn special_room_class_trial() {
    let level = run_special_room(SpecialRoomKind::ClassTrial, 66);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::PressureSensor));
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Tactical)));
}

#[test]
fn special_room_radical_reactor() {
    let level = run_special_room(SpecialRoomKind::RadicalReactor, 67);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::MedBayTile));
    assert!(contains_tile(&level, Tile::CoolantPool));
    assert!(contains_tile(&level, Tile::CrystalPanel));
}

#[test]
fn special_room_ancestor_crypt() {
    let level = run_special_room(SpecialRoomKind::AncestorCrypt, 68);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::SupplyCrate));
    assert!(contains_tile(&level, Tile::Terminal(AltarKind::Quantum)));
    assert!(contains_tile(&level, Tile::DataRack));
}

#[test]
fn special_room_prophecy_room() {
    let level = run_special_room(SpecialRoomKind::ProphecyRoom, 69);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::DataRack));
    assert!(contains_tile(&level, Tile::HoloPool));
}

#[test]
fn special_room_sealed_memory() {
    let level = run_special_room(SpecialRoomKind::SealedMemory, 70);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::NavBeacon));
    assert!(contains_tile(&level, Tile::CoolantPool));
    assert!(contains_tile(&level, Tile::DataRack));
}

#[test]
fn special_room_demon_seal() {
    let level = run_special_room(SpecialRoomKind::DemonSeal, 71);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::Npc(3)));
    assert!(contains_tile(&level, Tile::PlasmaVent));
}

#[test]
fn special_room_phoenix_nest() {
    let level = run_special_room(SpecialRoomKind::PhoenixNest, 72);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::MedBayTile));
    assert!(contains_tile(&level, Tile::PlasmaVent));
    assert!(contains_tile(&level, Tile::SupplyCrate));
}

#[test]
fn special_room_calligraphy_contest() {
    let level = run_special_room(SpecialRoomKind::CalligraphyContest, 73);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::DataWell));
    assert!(contains_tile(&level, Tile::Npc(0)));
}

#[test]
fn special_room_challenge_terminal() {
    let level = run_special_room(SpecialRoomKind::ChallengeTerminal, 74);
    assert_room_modified(&level, 2, 2, 16, 12);
    assert!(contains_tile(&level, Tile::DataWell));
    assert!(contains_tile(&level, Tile::SupplyCrate));
}

// ---------------------------------------------------------------------------
// Determinism: same kind + same seed → identical tiles
// ---------------------------------------------------------------------------

#[test]
fn special_room_generation_is_deterministic() {
    let kinds = [
        SpecialRoomKind::CargoBay,
        SpecialRoomKind::ToxicMaze,
        SpecialRoomKind::PirateAmbush,
        SpecialRoomKind::FloodedCompartment,
        SpecialRoomKind::DarkSector,
        SpecialRoomKind::RefugeeBay,
        SpecialRoomKind::NanoFlood,
    ];
    for kind in &kinds {
        let a = run_special_room(*kind, 999);
        let b = run_special_room(*kind, 999);
        assert_eq!(a.tiles, b.tiles, "determinism failed for {:?}", kind);
    }
}

// ---------------------------------------------------------------------------
// Different seeds produce different layouts (for RNG-dependent rooms)
// ---------------------------------------------------------------------------

#[test]
fn special_room_different_seeds_vary() {
    let a = run_special_room(SpecialRoomKind::CargoBay, 1);
    let b = run_special_room(SpecialRoomKind::CargoBay, 99999);
    // Very unlikely to produce identical layouts with different seeds
    assert_ne!(a.tiles, b.tiles, "different seeds should (almost always) differ");
}

// ---------------------------------------------------------------------------
// Bulk test: every kind can run without panic on multiple seeds
// ---------------------------------------------------------------------------

#[test]
fn all_special_room_kinds_run_without_panic() {
    let all_kinds = [
        SpecialRoomKind::CargoBay,
        SpecialRoomKind::OreDeposit,
        SpecialRoomKind::HallOfRecords,
        SpecialRoomKind::HiddenCache,
        SpecialRoomKind::OfferingTerminal,
        SpecialRoomKind::HydroponicsGarden,
        SpecialRoomKind::PirateStash,
        SpecialRoomKind::XenoCrystalVault,
        SpecialRoomKind::RelicChamber,
        SpecialRoomKind::MedBay,
        SpecialRoomKind::ArenaChallenge,
        SpecialRoomKind::SecurityCheckpoint,
        SpecialRoomKind::SecurityZone,
        SpecialRoomKind::ToxicMaze,
        SpecialRoomKind::GravityPuzzle,
        SpecialRoomKind::HolographicRoom,
        SpecialRoomKind::EnergyNexus,
        SpecialRoomKind::SpaceHulkRise,
        SpecialRoomKind::PirateAmbush,
        SpecialRoomKind::DuelArena,
        SpecialRoomKind::DataArchive,
        SpecialRoomKind::SignalHall,
        SpecialRoomKind::ResearcherStudy,
        SpecialRoomKind::SensorArray,
        SpecialRoomKind::InscriptionWall,
        SpecialRoomKind::TrainingSimulator,
        SpecialRoomKind::ZenChamber,
        SpecialRoomKind::TranslationChallenge,
        SpecialRoomKind::AncientDatapad,
        SpecialRoomKind::WisdomCore,
        SpecialRoomKind::FloodedCompartment,
        SpecialRoomKind::CryogenicBay,
        SpecialRoomKind::PlasmaCrossing,
        SpecialRoomKind::PipeForest,
        SpecialRoomKind::VentTunnel,
        SpecialRoomKind::CrystalCave,
        SpecialRoomKind::FungalGrotto,
        SpecialRoomKind::CoolantRiver,
        SpecialRoomKind::EchoingHull,
        SpecialRoomKind::DarkSector,
        SpecialRoomKind::WanderingMerchant,
        SpecialRoomKind::HermitSage,
        SpecialRoomKind::DetentionCell,
        SpecialRoomKind::MemorialShrine,
        SpecialRoomKind::CantinaBay,
        SpecialRoomKind::EngineerWorkshop,
        SpecialRoomKind::ChemLab,
        SpecialRoomKind::FortuneTeller,
        SpecialRoomKind::RefugeeBay,
        SpecialRoomKind::WarpGate,
        SpecialRoomKind::GamblingDen,
        SpecialRoomKind::BloodTerminal,
        SpecialRoomKind::CursedSalvage,
        SpecialRoomKind::SoulForge,
        SpecialRoomKind::WishingReactor,
        SpecialRoomKind::CipherGate,
        SpecialRoomKind::HoloMaze,
        SpecialRoomKind::GravityPlate,
        SpecialRoomKind::ToneFrequency,
        SpecialRoomKind::ElementalLock,
        SpecialRoomKind::SurvivalBay,
        SpecialRoomKind::SalvageRace,
        SpecialRoomKind::DepressurizingChamber,
        SpecialRoomKind::NanoFlood,
        SpecialRoomKind::FormShrine,
        SpecialRoomKind::ClassTrial,
        SpecialRoomKind::RadicalReactor,
        SpecialRoomKind::AncestorCrypt,
        SpecialRoomKind::ProphecyRoom,
        SpecialRoomKind::SealedMemory,
        SpecialRoomKind::DemonSeal,
        SpecialRoomKind::PhoenixNest,
        SpecialRoomKind::CalligraphyContest,
        SpecialRoomKind::ChallengeTerminal,
    ];
    for kind in &all_kinds {
        for seed in [1, 42, 9999, 123456] {
            run_special_room(*kind, seed);
        }
    }
}

// ---------------------------------------------------------------------------
// Boundary: rooms at the edge of the level don't panic
// ---------------------------------------------------------------------------

#[test]
fn special_room_at_level_edge() {
    let width = 20;
    let height = 16;
    let room = Room {
        x: 0,
        y: 0,
        w: 10,
        h: 8,
        modifier: None,
        special: Some(SpecialRoomKind::EnergyNexus),
    };
    let mut level = LocationLevel {
        width,
        height,
        tiles: vec![Tile::Bulkhead; (width * height) as usize],
        rooms: vec![room.clone()],
        visible: vec![false; (width * height) as usize],
        revealed: vec![false; (width * height) as usize],
    };
    for y in room.y..room.y + room.h {
        for x in room.x..room.x + room.w {
            if level.in_bounds(x, y) {
                let idx = level.idx(x, y);
                level.tiles[idx] = Tile::MetalFloor;
            }
        }
    }
    let mut rng = Rng::new(42);
    // Should not panic
    level.generate_special_room(SpecialRoomKind::EnergyNexus, &room, &mut rng);
}

// ---------------------------------------------------------------------------
// Small room doesn't panic (some features may be clipped)
// ---------------------------------------------------------------------------

#[test]
fn special_room_small_room_no_panic() {
    let width = 20;
    let height = 16;
    let room = Room {
        x: 5,
        y: 5,
        w: 5,
        h: 5,
        modifier: None,
        special: Some(SpecialRoomKind::WarpGate),
    };
    let mut level = LocationLevel {
        width,
        height,
        tiles: vec![Tile::Bulkhead; (width * height) as usize],
        rooms: vec![room.clone()],
        visible: vec![false; (width * height) as usize],
        revealed: vec![false; (width * height) as usize],
    };
    for y in room.y..room.y + room.h {
        for x in room.x..room.x + room.w {
            let idx = level.idx(x, y);
            level.tiles[idx] = Tile::MetalFloor;
        }
    }
    let mut rng = Rng::new(42);
    // Should not panic even with a small room
    level.generate_special_room(SpecialRoomKind::WarpGate, &room, &mut rng);
    assert_room_modified(&level, 5, 5, 5, 5);
}

