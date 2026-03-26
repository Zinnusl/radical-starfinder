use super::*;

// ── Helper ──────────────────────────────────────────────────────────────────

fn make_test_level(width: i32, height: i32, tiles: Vec<Tile>) -> LocationLevel {
    let size = (width * height) as usize;
    assert_eq!(tiles.len(), size);
    LocationLevel {
        width,
        height,
        tiles,
        rooms: vec![],
        visible: vec![false; size],
        revealed: vec![false; size],
    }
}

fn make_open_level(width: i32, height: i32) -> LocationLevel {
    let size = (width * height) as usize;
    LocationLevel {
        width,
        height,
        tiles: vec![Tile::MetalFloor; size],
        rooms: vec![],
        visible: vec![false; size],
        revealed: vec![false; size],
    }
}

fn make_room(x: i32, y: i32, w: i32, h: i32) -> Room {
    Room { x, y, w, h, modifier: None, special: None }
}

// ═══════════════════════════════════════════════════════════════════════════
// Rng
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn rng_seed_zero_becomes_one() {
    let rng = Rng::new(0);
    assert_eq!(rng.0, 1, "seed 0 must be promoted to 1");
}

#[test]
fn rng_nonzero_seed_preserved() {
    let rng = Rng::new(42);
    assert_eq!(rng.0, 42);
}

#[test]
fn rng_deterministic_sequence() {
    let mut a = Rng::new(12345);
    let mut b = Rng::new(12345);
    for _ in 0..100 {
        assert_eq!(a.next_u64(), b.next_u64());
    }
}

#[test]
fn rng_different_seeds_differ() {
    let mut a = Rng::new(1);
    let mut b = Rng::new(2);
    let mut same = true;
    for _ in 0..10 {
        if a.next_u64() != b.next_u64() {
            same = false;
            break;
        }
    }
    assert!(!same);
}

#[test]
fn rng_next_u64_changes_state() {
    let mut rng = Rng::new(7);
    let first = rng.next_u64();
    let second = rng.next_u64();
    assert_ne!(first, second);
}

#[test]
fn rng_range_bounds() {
    let mut rng = Rng::new(999);
    for _ in 0..200 {
        let v = rng.range(5, 10);
        assert!(v >= 5 && v < 10, "range value {v} out of [5,10)");
    }
}

#[test]
fn rng_range_lo_eq_hi_returns_lo() {
    let mut rng = Rng::new(42);
    for _ in 0..20 {
        assert_eq!(rng.range(7, 7), 7);
    }
}

#[test]
fn rng_range_hi_less_than_lo_returns_lo() {
    let mut rng = Rng::new(42);
    assert_eq!(rng.range(10, 5), 10);
}

#[test]
fn rng_range_single_value() {
    let mut rng = Rng::new(1);
    for _ in 0..20 {
        assert_eq!(rng.range(3, 4), 3);
    }
}

#[test]
fn rng_range_negative() {
    let mut rng = Rng::new(100);
    for _ in 0..100 {
        let v = rng.range(-5, 0);
        assert!(v >= -5 && v < 0, "range value {v} out of [-5,0)");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TerminalKind
// ═══════════════════════════════════════════════════════════════════════════

const ALL_TERMINALS: [TerminalKind; 5] = [
    TerminalKind::Quantum,
    TerminalKind::Stellar,
    TerminalKind::Holographic,
    TerminalKind::Tactical,
    TerminalKind::Commerce,
];

#[test]
fn terminal_icon_non_empty() {
    for t in ALL_TERMINALS {
        assert!(!t.icon().is_empty(), "{t:?} icon is empty");
    }
}

#[test]
fn terminal_color_non_empty() {
    for t in ALL_TERMINALS {
        assert!(!t.color().is_empty(), "{t:?} color is empty");
    }
}

#[test]
fn terminal_name_non_empty() {
    for t in ALL_TERMINALS {
        assert!(!t.name().is_empty(), "{t:?} name is empty");
    }
}

#[test]
fn terminal_deity_mapping() {
    use crate::player::Faction;
    assert_eq!(TerminalKind::Quantum.deity(), Faction::Consortium);
    assert_eq!(TerminalKind::Stellar.deity(), Faction::FreeTraders);
    assert_eq!(TerminalKind::Holographic.deity(), Faction::Technocracy);
    assert_eq!(TerminalKind::Tactical.deity(), Faction::MilitaryAlliance);
    assert_eq!(TerminalKind::Commerce.deity(), Faction::AncientOrder);
}

#[test]
fn terminal_random_covers_all_variants() {
    let mut seen = std::collections::HashSet::new();
    let mut rng = Rng::new(1);
    for _ in 0..500 {
        seen.insert(TerminalKind::random(&mut rng));
    }
    for t in ALL_TERMINALS {
        assert!(seen.contains(&t), "random never produced {t:?}");
    }
}

#[test]
fn terminal_name_contains_terminal() {
    for t in ALL_TERMINALS {
        assert!(t.name().contains("Terminal"), "{t:?} name missing 'Terminal'");
    }
}

#[test]
fn terminal_color_is_hex() {
    for t in ALL_TERMINALS {
        assert!(t.color().starts_with('#'), "{t:?} color not hex");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SecuritySeal
// ═══════════════════════════════════════════════════════════════════════════

const ALL_SEALS: [SecuritySeal; 4] = [
    SecuritySeal::Thermal,
    SecuritySeal::Hydraulic,
    SecuritySeal::Kinetic,
    SecuritySeal::Sonic,
];

#[test]
fn seal_icon_non_empty() {
    for s in ALL_SEALS {
        assert!(!s.icon().is_empty(), "{s:?} icon is empty");
    }
}

#[test]
fn seal_color_non_empty() {
    for s in ALL_SEALS {
        assert!(!s.color().is_empty(), "{s:?} color is empty");
    }
}

#[test]
fn seal_label_non_empty() {
    for s in ALL_SEALS {
        assert!(!s.label().is_empty(), "{s:?} label is empty");
    }
}

#[test]
fn seal_label_contains_seal() {
    for s in ALL_SEALS {
        assert!(s.label().contains("seal"), "{s:?} label missing 'seal'");
    }
}

#[test]
fn seal_color_is_hex() {
    for s in ALL_SEALS {
        assert!(s.color().starts_with('#'), "{s:?} color not hex");
    }
}

#[test]
fn seal_random_covers_all_variants() {
    let mut seen = std::collections::HashSet::new();
    let mut rng = Rng::new(7);
    for _ in 0..400 {
        seen.insert(SecuritySeal::random(&mut rng));
    }
    for s in ALL_SEALS {
        assert!(seen.contains(&s), "random never produced {s:?}");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SpecialRoomKind
// ═══════════════════════════════════════════════════════════════════════════

const ALL_SPECIAL_ROOMS: [SpecialRoomKind; 74] = [
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

#[test]
fn special_room_name_non_empty() {
    for r in ALL_SPECIAL_ROOMS {
        assert!(!r.name().is_empty(), "{r:?} name is empty");
    }
}

#[test]
fn special_room_description_non_empty() {
    for r in ALL_SPECIAL_ROOMS {
        assert!(!r.description().is_empty(), "{r:?} description is empty");
    }
}

#[test]
fn special_room_random_floor_1_returns_base_rooms() {
    let mut rng = Rng::new(42);
    for _ in 0..200 {
        let room = SpecialRoomKind::random_for_floor(&mut rng, 1);
        // Floor-gated rooms should NOT appear at floor 1
        assert_ne!(room, SpecialRoomKind::PhoenixNest);
        assert_ne!(room, SpecialRoomKind::WarpGate);
        assert_ne!(room, SpecialRoomKind::FormShrine);
        assert_ne!(room, SpecialRoomKind::DemonSeal);
        assert_ne!(room, SpecialRoomKind::EnergyNexus);
        assert_ne!(room, SpecialRoomKind::CursedSalvage);
        assert_ne!(room, SpecialRoomKind::HiddenCache);
    }
}

#[test]
fn special_room_floor_3_unlocks_content() {
    let mut seen = std::collections::HashSet::new();
    let mut rng = Rng::new(1);
    for _ in 0..2000 {
        seen.insert(SpecialRoomKind::random_for_floor(&mut rng, 3));
    }
    // Floor 3 should unlock these
    assert!(seen.contains(&SpecialRoomKind::HiddenCache));
    assert!(seen.contains(&SpecialRoomKind::DuelArena));
    assert!(seen.contains(&SpecialRoomKind::SensorArray));
    // But NOT floor 5+ exclusive rooms
    assert!(!seen.contains(&SpecialRoomKind::EnergyNexus));
    assert!(!seen.contains(&SpecialRoomKind::PhoenixNest));
}

#[test]
fn special_room_floor_5_unlocks_content() {
    let mut seen = std::collections::HashSet::new();
    let mut rng = Rng::new(1);
    for _ in 0..5000 {
        seen.insert(SpecialRoomKind::random_for_floor(&mut rng, 5));
    }
    assert!(seen.contains(&SpecialRoomKind::EnergyNexus));
    assert!(seen.contains(&SpecialRoomKind::CursedSalvage));
    assert!(seen.contains(&SpecialRoomKind::SoulForge));
    // But NOT floor 10+ exclusive rooms
    assert!(!seen.contains(&SpecialRoomKind::FormShrine));
    assert!(!seen.contains(&SpecialRoomKind::PhoenixNest));
}

#[test]
fn special_room_floor_10_unlocks_content() {
    let mut seen = std::collections::HashSet::new();
    let mut rng = Rng::new(1);
    for _ in 0..5000 {
        seen.insert(SpecialRoomKind::random_for_floor(&mut rng, 10));
    }
    assert!(seen.contains(&SpecialRoomKind::FormShrine));
    assert!(seen.contains(&SpecialRoomKind::DemonSeal));
    assert!(!seen.contains(&SpecialRoomKind::PhoenixNest));
}

#[test]
fn special_room_floor_20_unlocks_phoenix() {
    let mut seen = std::collections::HashSet::new();
    let mut rng = Rng::new(1);
    for _ in 0..10000 {
        seen.insert(SpecialRoomKind::random_for_floor(&mut rng, 20));
    }
    assert!(seen.contains(&SpecialRoomKind::PhoenixNest));
    assert!(!seen.contains(&SpecialRoomKind::WarpGate));
}

#[test]
fn special_room_floor_25_unlocks_warp_gate() {
    let mut seen = std::collections::HashSet::new();
    let mut rng = Rng::new(1);
    for _ in 0..10000 {
        seen.insert(SpecialRoomKind::random_for_floor(&mut rng, 25));
    }
    assert!(seen.contains(&SpecialRoomKind::WarpGate));
    assert!(seen.contains(&SpecialRoomKind::PhoenixNest));
}

#[test]
fn special_room_random_always_returns_valid() {
    let mut rng = Rng::new(42);
    for floor in [1, 2, 3, 5, 10, 15, 20, 25, 50] {
        for _ in 0..100 {
            let room = SpecialRoomKind::random_for_floor(&mut rng, floor);
            // Must be one of the known variants (name must not panic)
            assert!(!room.name().is_empty());
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tile
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn tile_bulkhead_not_walkable() {
    assert!(!Tile::Bulkhead.is_walkable());
}

#[test]
fn tile_damaged_bulkhead_not_walkable() {
    assert!(!Tile::DamagedBulkhead.is_walkable());
}

#[test]
fn tile_weak_bulkhead_not_walkable() {
    assert!(!Tile::WeakBulkhead.is_walkable());
}

#[test]
fn tile_vacuum_breach_not_walkable() {
    assert!(!Tile::VacuumBreach.is_walkable());
}

#[test]
fn tile_salvage_crate_not_walkable() {
    assert!(!Tile::SalvageCrate.is_walkable());
}

#[test]
fn tile_cargo_pipes_not_walkable() {
    assert!(!Tile::CargoPipes.is_walkable());
}

#[test]
fn tile_ore_vein_not_walkable() {
    assert!(!Tile::OreVein.is_walkable());
}

#[test]
fn tile_cargo_crate_not_walkable() {
    assert!(!Tile::CargoCrate.is_walkable());
}

#[test]
fn tile_crystal_panel_not_walkable() {
    assert!(!Tile::CrystalPanel.is_walkable());
}

#[test]
fn tile_metal_floor_walkable() {
    assert!(Tile::MetalFloor.is_walkable());
}

#[test]
fn tile_hallway_walkable() {
    assert!(Tile::Hallway.is_walkable());
}

#[test]
fn tile_airlock_walkable() {
    assert!(Tile::Airlock.is_walkable());
}

#[test]
fn tile_quantum_forge_walkable() {
    assert!(Tile::QuantumForge.is_walkable());
}

#[test]
fn tile_trade_terminal_walkable() {
    assert!(Tile::TradeTerminal.is_walkable());
}

#[test]
fn tile_supply_crate_walkable() {
    assert!(Tile::SupplyCrate.is_walkable());
}

#[test]
fn tile_laser_grid_walkable() {
    assert!(Tile::LaserGrid.is_walkable());
}

#[test]
fn tile_coolant_walkable() {
    assert!(Tile::Coolant.is_walkable());
}

#[test]
fn tile_coolant_pool_walkable() {
    assert!(Tile::CoolantPool.is_walkable());
}

#[test]
fn tile_npc_walkable() {
    for id in 0..4u8 {
        assert!(Tile::Npc(id).is_walkable(), "Npc({id}) should be walkable");
    }
}

#[test]
fn tile_nav_beacon_walkable() {
    assert!(Tile::NavBeacon.is_walkable());
}

#[test]
fn tile_terminal_walkable() {
    for tk in ALL_TERMINALS {
        assert!(Tile::Terminal(tk).is_walkable());
    }
}

#[test]
fn tile_security_lock_walkable() {
    for s in ALL_SEALS {
        assert!(Tile::SecurityLock(s).is_walkable());
    }
}

#[test]
fn tile_info_panel_walkable() {
    assert!(Tile::InfoPanel(0).is_walkable());
}

#[test]
fn tile_catwalk_walkable() {
    assert!(Tile::Catwalk.is_walkable());
}

#[test]
fn tile_interactive_tiles_walkable() {
    let walkable_tiles = [
        Tile::CircuitShrine,
        Tile::FrequencyWall,
        Tile::CompoundShrine,
        Tile::ClassifierNode,
        Tile::DataWell,
        Tile::MemorialNode,
        Tile::TranslationTerminal,
        Tile::RadicalLab,
        Tile::HoloPool,
        Tile::DroidTutor,
        Tile::CodexTerminal,
        Tile::DataBridge,
        Tile::CorruptedFloor,
        Tile::PlasmaVent,
        Tile::FrozenDeck,
        Tile::ToxicFungus,
        Tile::ToxicGas,
        Tile::DataRack,
        Tile::PressureSensor,
        Tile::WarpGatePortal,
        Tile::MedBayTile,
        Tile::CreditCache,
    ];
    for t in walkable_tiles {
        assert!(t.is_walkable(), "{t:?} should be walkable");
    }
}

#[test]
fn tile_trap_walkable() {
    for trap_type in 0..3u8 {
        assert!(Tile::Trap(trap_type).is_walkable());
    }
}

#[test]
fn tile_special_room_walkable() {
    assert!(Tile::SpecialRoom(SpecialRoomKind::CargoBay).is_walkable());
    assert!(Tile::SpecialRoom(SpecialRoomKind::PhoenixNest).is_walkable());
}

#[test]
fn tile_sealed_hatch_not_walkable() {
    assert!(!Tile::SealedHatch.is_walkable());
}

// ═══════════════════════════════════════════════════════════════════════════
// Room
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn room_center_basic() {
    let r = make_room(0, 0, 10, 10);
    assert_eq!(r.center(), (5, 5));
}

#[test]
fn room_center_offset() {
    let r = make_room(3, 7, 6, 4);
    assert_eq!(r.center(), (6, 9));
}

#[test]
fn room_center_odd_dimensions() {
    let r = make_room(0, 0, 5, 3);
    assert_eq!(r.center(), (2, 1));
}

#[test]
fn room_center_unit() {
    let r = make_room(5, 5, 1, 1);
    assert_eq!(r.center(), (5, 5));
}

#[test]
fn room_intersects_overlap() {
    let a = make_room(0, 0, 5, 5);
    let b = make_room(3, 3, 5, 5);
    assert!(a.intersects(&b));
    assert!(b.intersects(&a));
}

#[test]
fn room_intersects_no_overlap() {
    let a = make_room(0, 0, 3, 3);
    let b = make_room(5, 5, 3, 3);
    assert!(!a.intersects(&b));
    assert!(!b.intersects(&a));
}

#[test]
fn room_intersects_adjacent_not_overlapping() {
    // Rooms touching at edge should NOT intersect (boundary condition)
    let a = make_room(0, 0, 5, 5);
    let b = make_room(5, 0, 5, 5);
    assert!(!a.intersects(&b));
    assert!(!b.intersects(&a));
}

#[test]
fn room_intersects_adjacent_vertically() {
    let a = make_room(0, 0, 5, 5);
    let b = make_room(0, 5, 5, 5);
    assert!(!a.intersects(&b));
}

#[test]
fn room_intersects_contained() {
    let outer = make_room(0, 0, 10, 10);
    let inner = make_room(2, 2, 3, 3);
    assert!(outer.intersects(&inner));
    assert!(inner.intersects(&outer));
}

#[test]
fn room_intersects_self() {
    let r = make_room(0, 0, 5, 5);
    assert!(r.intersects(&r));
}

#[test]
fn room_intersects_single_pixel_overlap() {
    let a = make_room(0, 0, 5, 5);
    let b = make_room(4, 4, 5, 5);
    assert!(a.intersects(&b));
}

#[test]
fn room_modifier_field() {
    let mut r = make_room(0, 0, 5, 5);
    assert_eq!(r.modifier, None);
    r.modifier = Some(RoomModifier::PoweredDown);
    assert_eq!(r.modifier, Some(RoomModifier::PoweredDown));
}

#[test]
fn room_special_field() {
    let mut r = make_room(0, 0, 5, 5);
    assert_eq!(r.special, None);
    r.special = Some(SpecialRoomKind::MedBay);
    assert_eq!(r.special, Some(SpecialRoomKind::MedBay));
}

// ═══════════════════════════════════════════════════════════════════════════
// LocationLevel
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn level_idx_basic() {
    let level = make_open_level(10, 10);
    assert_eq!(level.idx(0, 0), 0);
    assert_eq!(level.idx(9, 0), 9);
    assert_eq!(level.idx(0, 1), 10);
    assert_eq!(level.idx(5, 3), 35);
}

#[test]
fn level_in_bounds_valid() {
    let level = make_open_level(10, 8);
    assert!(level.in_bounds(0, 0));
    assert!(level.in_bounds(9, 7));
    assert!(level.in_bounds(5, 3));
}

#[test]
fn level_in_bounds_invalid() {
    let level = make_open_level(10, 8);
    assert!(!level.in_bounds(-1, 0));
    assert!(!level.in_bounds(0, -1));
    assert!(!level.in_bounds(10, 0));
    assert!(!level.in_bounds(0, 8));
    assert!(!level.in_bounds(-1, -1));
    assert!(!level.in_bounds(100, 100));
}

#[test]
fn level_tile_in_bounds() {
    let tiles = vec![Tile::MetalFloor, Tile::Hallway, Tile::Bulkhead, Tile::Airlock];
    let level = make_test_level(2, 2, tiles);
    assert_eq!(level.tile(0, 0), Tile::MetalFloor);
    assert_eq!(level.tile(1, 0), Tile::Hallway);
    assert_eq!(level.tile(0, 1), Tile::Bulkhead);
    assert_eq!(level.tile(1, 1), Tile::Airlock);
}

#[test]
fn level_tile_out_of_bounds_returns_bulkhead() {
    let level = make_open_level(5, 5);
    assert_eq!(level.tile(-1, 0), Tile::Bulkhead);
    assert_eq!(level.tile(0, -1), Tile::Bulkhead);
    assert_eq!(level.tile(5, 0), Tile::Bulkhead);
    assert_eq!(level.tile(0, 5), Tile::Bulkhead);
    assert_eq!(level.tile(100, 100), Tile::Bulkhead);
}

#[test]
fn level_is_walkable_metal_floor() {
    let level = make_open_level(5, 5);
    assert!(level.is_walkable(2, 2));
}

#[test]
fn level_is_walkable_bulkhead() {
    let tiles = vec![Tile::Bulkhead; 4];
    let level = make_test_level(2, 2, tiles);
    assert!(!level.is_walkable(0, 0));
}

#[test]
fn level_is_walkable_out_of_bounds() {
    let level = make_open_level(5, 5);
    assert!(!level.is_walkable(-1, 0));
    assert!(!level.is_walkable(5, 0));
    assert!(!level.is_walkable(0, -1));
    assert!(!level.is_walkable(0, 5));
}

// ═══════════════════════════════════════════════════════════════════════════
// LocationType
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn location_type_variants_exist() {
    let types = [
        LocationType::SpaceStation,
        LocationType::AsteroidBase,
        LocationType::DerelictShip,
        LocationType::AlienRuins,
        LocationType::TradingPost,
        LocationType::OrbitalPlatform,
        LocationType::MiningColony,
        LocationType::ResearchLab,
    ];
    // Ensure all distinct
    for i in 0..types.len() {
        for j in (i + 1)..types.len() {
            assert_ne!(types[i], types[j]);
        }
    }
}

#[test]
fn location_type_clone_eq() {
    let t = LocationType::SpaceStation;
    let t2 = t;
    assert_eq!(t, t2);
}

// ═══════════════════════════════════════════════════════════════════════════
// RoomModifier
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn room_modifier_variants_exist() {
    let mods = [
        RoomModifier::PoweredDown,
        RoomModifier::HighTech,
        RoomModifier::Irradiated,
        RoomModifier::Hydroponics,
        RoomModifier::Cryogenic,
        RoomModifier::OverheatedReactor,
    ];
    for i in 0..mods.len() {
        for j in (i + 1)..mods.len() {
            assert_ne!(mods[i], mods[j]);
        }
    }
}

#[test]
fn room_modifier_clone_eq() {
    let m = RoomModifier::HighTech;
    let m2 = m;
    assert_eq!(m, m2);
}

// ═══════════════════════════════════════════════════════════════════════════
// FOV (field of view)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn fov_origin_always_visible() {
    let mut level = make_open_level(10, 10);
    compute_fov(&mut level, 5, 5, 4);
    let idx = level.idx(5, 5);
    assert!(level.visible[idx]);
    assert!(level.revealed[idx]);
}

#[test]
fn fov_open_room_all_within_radius_visible() {
    let mut level = make_open_level(20, 20);
    let ox = 10;
    let oy = 10;
    let radius = 5;
    compute_fov(&mut level, ox, oy, radius);
    for y in 0..20 {
        for x in 0..20 {
            let dx = x - ox;
            let dy = y - oy;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= radius * radius {
                let idx = level.idx(x, y);
                assert!(
                    level.visible[idx],
                    "({x},{y}) dist²={dist_sq} should be visible"
                );
            }
        }
    }
}

#[test]
fn fov_wall_blocks_vision() {
    // 5x5 level, wall at (2,1), origin at (0,1)
    let mut tiles = vec![Tile::MetalFloor; 25];
    // Place wall column at x=2
    tiles[2] = Tile::Bulkhead;    // (2,0)
    tiles[7] = Tile::Bulkhead;    // (2,1)
    tiles[12] = Tile::Bulkhead;   // (2,2)
    tiles[17] = Tile::Bulkhead;   // (2,3)
    tiles[22] = Tile::Bulkhead;   // (2,4)
    let mut level = make_test_level(5, 5, tiles);
    compute_fov(&mut level, 0, 2, 10);

    // Origin should be visible
    assert!(level.visible[level.idx(0, 2)]);
    // Tile left of wall should be visible
    assert!(level.visible[level.idx(1, 2)]);
    // Wall itself may be visible (walls are usually revealed)
    // Tiles behind the wall should NOT be visible
    assert!(
        !level.visible[level.idx(4, 2)],
        "(4,2) behind wall should not be visible"
    );
}

#[test]
fn fov_radius_zero_only_origin() {
    let mut level = make_open_level(10, 10);
    compute_fov(&mut level, 5, 5, 0);
    let origin_idx = level.idx(5, 5);
    assert!(level.visible[origin_idx]);

    // No other tile should be visible
    for i in 0..100 {
        if i != origin_idx {
            assert!(!level.visible[i], "tile {i} should not be visible with radius 0");
        }
    }
}

#[test]
fn fov_out_of_bounds_origin_no_panic() {
    let mut level = make_open_level(5, 5);
    // Should not panic even with OOB origin
    compute_fov(&mut level, -1, -1, 3);
    compute_fov(&mut level, 10, 10, 3);
    compute_fov(&mut level, -100, -100, 0);
}

#[test]
fn fov_clears_previous_visibility() {
    let mut level = make_open_level(10, 10);
    // First FOV
    compute_fov(&mut level, 0, 0, 3);
    assert!(level.visible[level.idx(0, 0)]);

    // Second FOV from far corner clears old visibility
    compute_fov(&mut level, 9, 9, 1);
    assert!(!level.visible[level.idx(0, 0)], "old origin should no longer be visible");
    assert!(level.visible[level.idx(9, 9)]);
}

#[test]
fn fov_revealed_persists_across_calls() {
    let mut level = make_open_level(10, 10);
    compute_fov(&mut level, 0, 0, 2);
    let first_revealed = level.revealed[level.idx(0, 0)];
    assert!(first_revealed);

    compute_fov(&mut level, 9, 9, 2);
    // Origin of first call should still be revealed
    assert!(level.revealed[level.idx(0, 0)]);
    // Origin of second call should also be revealed
    assert!(level.revealed[level.idx(9, 9)]);
}

#[test]
fn fov_corner_origin() {
    let mut level = make_open_level(10, 10);
    compute_fov(&mut level, 0, 0, 3);
    assert!(level.visible[level.idx(0, 0)]);
    assert!(level.visible[level.idx(1, 0)]);
    assert!(level.visible[level.idx(0, 1)]);
}

#[test]
fn fov_symmetric_open_room() {
    let mut level = make_open_level(21, 21);
    let ox = 10;
    let oy = 10;
    compute_fov(&mut level, ox, oy, 5);
    // In an open room, (ox+3, oy) and (ox-3, oy) should both be visible
    assert!(level.visible[level.idx(ox + 3, oy)]);
    assert!(level.visible[level.idx(ox - 3, oy)]);
    assert!(level.visible[level.idx(ox, oy + 3)]);
    assert!(level.visible[level.idx(ox, oy - 3)]);
}

#[test]
fn fov_large_radius_no_panic() {
    let mut level = make_open_level(5, 5);
    compute_fov(&mut level, 2, 2, 100);
    // All tiles should be visible
    for i in 0..25 {
        assert!(level.visible[i]);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// AltarKind / SealKind type aliases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn type_alias_altar_kind() {
    let _: AltarKind = TerminalKind::Quantum;
}

#[test]
fn type_alias_seal_kind() {
    let _: SealKind = SecuritySeal::Thermal;
}

#[test]
fn type_alias_dungeon_level() {
    let level = make_open_level(3, 3);
    let _dl: &DungeonLevel = &level;
}
