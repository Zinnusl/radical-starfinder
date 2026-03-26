//! Ship interior — walkable tile-based map for the player's starship.
//!
//! The ship is a fixed layout (not procedural) that the player explores
//! between missions. Rooms contain interactive consoles, crew stations,
//! and decorative tiles that make the ship feel lived-in.
#![allow(dead_code)]

// ── Ship room types ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShipRoom {
    Bridge,
    EngineRoom,
    QuantumForge,
    CrewQuarters,
    CargoBay,
    Medbay,
    WeaponsBay,
    Airlock,
    Corridor,
}

impl ShipRoom {
    pub fn name(self) -> &'static str {
        match self {
            Self::Bridge => "Bridge",
            Self::EngineRoom => "Engine Room",
            Self::QuantumForge => "Quantum Forge",
            Self::CrewQuarters => "Crew Quarters",
            Self::CargoBay => "Cargo Bay",
            Self::Medbay => "Medbay",
            Self::WeaponsBay => "Weapons Bay",
            Self::Airlock => "Airlock",
            Self::Corridor => "Corridor",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Bridge => "Navigation and starmap access. Plot your course through the sector.",
            Self::EngineRoom => "FTL drive controls and fuel management systems.",
            Self::QuantumForge => "Radical forging station. Craft and enhance equipment.",
            Self::CrewQuarters => "Rest area for the crew. Interact with companions.",
            Self::CargoBay => "Inventory management and loot storage.",
            Self::Medbay => "Medical bay. Heal injuries and cure status effects.",
            Self::WeaponsBay => "Weapon management and ship armament controls.",
            Self::Airlock => "Exit to the current location. Step outside to explore.",
            Self::Corridor => "Connecting hallway between ship sections.",
        }
    }
}

// ── Ship tile types ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShipTile {
    Floor,
    Wall,
    Door,
    Console(ShipRoom),
    CrewStation(usize),
    Decoration(u8),
    Empty,
}

// ── Ship layout ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ShipLayout {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<ShipTile>,
    pub room_labels: Vec<(i32, i32, ShipRoom)>,
}

// ── Ship upgrade system ─────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShipUpgrade {
    ReinforcedHull,
    ExtendedFuelTanks,
    AdvancedShields,
    CargoExpansion,
    SensorArray,
    AutoRepairDrone,
    WeaponBooster,
    EngineBooster,
    MedicalBay,
    QuantumForgeUpgrade,
}

impl ShipUpgrade {
    pub fn name(self) -> &'static str {
        match self {
            Self::ReinforcedHull => "Reinforced Hull",
            Self::ExtendedFuelTanks => "Extended Fuel Tanks",
            Self::AdvancedShields => "Advanced Shields",
            Self::CargoExpansion => "Cargo Expansion",
            Self::SensorArray => "Sensor Array",
            Self::AutoRepairDrone => "Auto-Repair Drone",
            Self::WeaponBooster => "Weapon Booster",
            Self::EngineBooster => "Engine Booster",
            Self::MedicalBay => "Medical Bay Upgrade",
            Self::QuantumForgeUpgrade => "Quantum Forge Upgrade",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::ReinforcedHull => "+20 max hull integrity",
            Self::ExtendedFuelTanks => "+30 max fuel capacity",
            Self::AdvancedShields => "+10 max shield strength",
            Self::CargoExpansion => "+5 cargo slots",
            Self::SensorArray => "+2 sensor range",
            Self::AutoRepairDrone => "Heal 2 hull per jump",
            Self::WeaponBooster => "+2 weapon power",
            Self::EngineBooster => "-1 fuel per jump",
            Self::MedicalBay => "Heal crew +2 HP per jump",
            Self::QuantumForgeUpgrade => "+1 forge slot",
        }
    }

    pub fn cost(self) -> i32 {
        match self {
            Self::ReinforcedHull => 200,
            Self::ExtendedFuelTanks => 150,
            Self::AdvancedShields => 250,
            Self::CargoExpansion => 100,
            Self::SensorArray => 175,
            Self::AutoRepairDrone => 300,
            Self::WeaponBooster => 225,
            Self::EngineBooster => 275,
            Self::MedicalBay => 200,
            Self::QuantumForgeUpgrade => 350,
        }
    }

    pub fn all() -> &'static [ShipUpgrade] {
        &[
            ShipUpgrade::ReinforcedHull,
            ShipUpgrade::ExtendedFuelTanks,
            ShipUpgrade::AdvancedShields,
            ShipUpgrade::CargoExpansion,
            ShipUpgrade::SensorArray,
            ShipUpgrade::AutoRepairDrone,
            ShipUpgrade::WeaponBooster,
            ShipUpgrade::EngineBooster,
            ShipUpgrade::MedicalBay,
            ShipUpgrade::QuantumForgeUpgrade,
        ]
    }
}

// ── Layout generation ───────────────────────────────────────────────────────

/// Build the fixed interior layout of the player's starship.
///
/// ```text
///    +----------+----------+
///    |  Bridge  | Weapons  |
///    |   [C]    |  Bay [C] |
///    +----+D+---+---+D+----+
///         | Corridor |
///    +----+D+---+---+D+----+
///    |  Forge  | Crew Qtrs |
///    |   [C]   |  S  [C]   |
///    +----+D+---+---+D+----+
///         | Corridor |
///    +----+D+---+---+D+----+
///    | Engine  |  Medbay   |
///    |   [C]   |    [C]    |
///    +----+--+D+-+D+--+----+
///            | Cargo  |
///            | Bay    |
///            +--[A]---+
/// ```
///
/// Grid: 24 wide × 18 tall.  `[C]` = console, `[A]` = airlock, `S` = crew station.
pub fn generate_ship_layout() -> ShipLayout {
    let width: i32 = 24;
    let height: i32 = 18;
    let mut tiles = vec![ShipTile::Empty; (width * height) as usize];

    // ── helpers ──────────────────────────────────────────────────────────
    let set = |tiles: &mut Vec<ShipTile>, x: i32, y: i32, t: ShipTile| {
        if x >= 0 && x < width && y >= 0 && y < height {
            tiles[(y * width + x) as usize] = t;
        }
    };

    let draw_rect = |tiles: &mut Vec<ShipTile>, x1: i32, y1: i32, x2: i32, y2: i32| {
        for y in y1..=y2 {
            for x in x1..=x2 {
                let t = if x == x1 || x == x2 || y == y1 || y == y2 {
                    ShipTile::Wall
                } else {
                    ShipTile::Floor
                };
                tiles[(y * width + x) as usize] = t;
            }
        }
    };

    // ── rooms (outer wall boundaries, inclusive) ─────────────────────────
    //
    //  Bridge       (3, 0)–(11, 3)   interior cols 4-10, rows 1-2
    //  WeaponsBay   (11,0)–(20, 3)   interior cols 12-19, rows 1-2
    //  UpperCorr    (7, 3)–(15, 5)   interior cols 8-14, row 4
    //  Forge        (3, 5)–(11, 8)   interior cols 4-10, rows 6-7
    //  CrewQuarters (11,5)–(20, 8)   interior cols 12-19, rows 6-7
    //  LowerCorr    (7, 8)–(15,10)   interior cols 8-14, row 9
    //  EngineRoom   (3,10)–(11,13)   interior cols 4-10, rows 11-12
    //  Medbay       (11,10)–(20,13)  interior cols 12-19, rows 11-12
    //  CargoBay     (8,13)–(15,17)   interior cols 9-14, rows 14-16

    draw_rect(&mut tiles, 3, 0, 11, 3);     // Bridge
    draw_rect(&mut tiles, 11, 0, 20, 3);    // WeaponsBay (shares wall col 11)
    draw_rect(&mut tiles, 7, 3, 15, 5);     // Upper Corridor
    draw_rect(&mut tiles, 3, 5, 11, 8);     // Quantum Forge
    draw_rect(&mut tiles, 11, 5, 20, 8);    // Crew Quarters
    draw_rect(&mut tiles, 7, 8, 15, 10);    // Lower Corridor
    draw_rect(&mut tiles, 3, 10, 11, 13);   // Engine Room
    draw_rect(&mut tiles, 11, 10, 20, 13);  // Medbay
    draw_rect(&mut tiles, 8, 13, 15, 17);   // Cargo Bay

    // ── doors (replace wall tiles to connect rooms) ─────────────────────
    let doors: &[(i32, i32)] = &[
        // Bridge / Weapons ↔ Upper Corridor
        (9, 3),
        (13, 3),
        // Upper Corridor ↔ Forge / Crew Quarters
        (9, 5),
        (13, 5),
        // Forge / Crew ↔ Lower Corridor
        (9, 8),
        (13, 8),
        // Lower Corridor ↔ Engine / Medbay
        (9, 10),
        (13, 10),
        // Engine / Medbay ↔ Cargo Bay
        (10, 13),
        (12, 13),
    ];
    for &(dx, dy) in doors {
        set(&mut tiles, dx, dy, ShipTile::Door);
    }

    // ── consoles ────────────────────────────────────────────────────────
    let consoles: &[(i32, i32, ShipRoom)] = &[
        (6, 1, ShipRoom::Bridge),
        (16, 1, ShipRoom::WeaponsBay),
        (6, 6, ShipRoom::QuantumForge),
        (16, 6, ShipRoom::CrewQuarters),
        (6, 11, ShipRoom::EngineRoom),
        (16, 11, ShipRoom::Medbay),
        (11, 15, ShipRoom::CargoBay),
        (12, 16, ShipRoom::Airlock),
    ];
    for &(cx, cy, room) in consoles {
        set(&mut tiles, cx, cy, ShipTile::Console(room));
    }

    // ── crew stations ───────────────────────────────────────────────────
    // 0 = pilot, 1 = gunner, 2 = engineer, 3 = medic
    let crew: &[(i32, i32, usize)] = &[
        (8, 1, 0),   // pilot on the Bridge
        (18, 2, 1),  // gunner in Weapons Bay
        (8, 12, 2),  // engineer in Engine Room
        (18, 12, 3), // medic in Medbay
    ];
    for &(sx, sy, idx) in crew {
        set(&mut tiles, sx, sy, ShipTile::CrewStation(idx));
    }

    // ── decorations ─────────────────────────────────────────────────────
    // 0 = pipes, 1 = screen/monitor, 2 = crates, 3 = conduit,
    // 4 = panel, 5 = machinery
    let decos: &[(i32, i32, u8)] = &[
        // Bridge — navigation screens, control panels
        (4, 1, 1),
        (10, 1, 4),
        (5, 2, 1),
        // Weapons Bay — weapon racks, ammo crates, targeting
        (14, 2, 0),
        (19, 1, 2),
        (17, 2, 4),
        // Quantum Forge — energy conduits, forge panels
        (4, 7, 3),
        (5, 6, 0),
        (10, 7, 4),
        // Crew Quarters — bunks, personal effects
        (14, 7, 2),
        (19, 7, 1),
        (18, 6, 5),
        // Engine Room — pipes, power conduits, machinery
        (4, 11, 0),
        (5, 12, 3),
        (10, 12, 5),
        // Medbay — medical panels, monitors, fluid conduit
        (14, 11, 4),
        (19, 12, 1),
        (17, 11, 3),
        // Cargo Bay — crates and ventilation
        (9, 14, 2),
        (14, 14, 2),
        (9, 16, 2),
        (13, 16, 0),
        // Corridors — pipe accents
        (8, 4, 0),
        (14, 4, 0),
        (8, 9, 0),
        (14, 9, 0),
    ];
    for &(dx, dy, variant) in decos {
        set(&mut tiles, dx, dy, ShipTile::Decoration(variant));
    }

    // ── room labels (for UI overlay text) ───────────────────────────────
    let room_labels = vec![
        (7, 1, ShipRoom::Bridge),
        (15, 1, ShipRoom::WeaponsBay),
        (7, 6, ShipRoom::QuantumForge),
        (15, 6, ShipRoom::CrewQuarters),
        (7, 11, ShipRoom::EngineRoom),
        (15, 11, ShipRoom::Medbay),
        (11, 14, ShipRoom::CargoBay),
        (11, 4, ShipRoom::Corridor),
        (11, 9, ShipRoom::Corridor),
    ];

    ShipLayout { width, height, tiles, room_labels }
}

// ── Interaction helpers ─────────────────────────────────────────────────────

pub fn tile_at(layout: &ShipLayout, x: i32, y: i32) -> ShipTile {
    if x < 0 || x >= layout.width || y < 0 || y >= layout.height {
        return ShipTile::Empty;
    }
    layout.tiles[(y * layout.width + x) as usize]
}

pub fn is_walkable(tile: ShipTile) -> bool {
    matches!(tile, ShipTile::Floor | ShipTile::Door | ShipTile::Console(_))
}

pub fn get_console_room(layout: &ShipLayout, x: i32, y: i32) -> Option<ShipRoom> {
    match tile_at(layout, x, y) {
        ShipTile::Console(room) => Some(room),
        _ => None,
    }
}

/// Determine which ship room a tile coordinate belongs to.
pub fn get_room_at(layout: &ShipLayout, x: i32, y: i32) -> ShipRoom {
    // Check for airlock console first
    if let ShipTile::Console(ShipRoom::Airlock) = tile_at(layout, x, y) {
        return ShipRoom::Airlock;
    }

    // Top room row (y 0–3): Bridge (left) / Weapons Bay (right)
    if (0..=3).contains(&y) {
        if (3..=11).contains(&x) {
            return ShipRoom::Bridge;
        }
        if (12..=20).contains(&x) {
            return ShipRoom::WeaponsBay;
        }
    }

    // Upper corridor (y 4)
    if y == 4 && (7..=15).contains(&x) {
        return ShipRoom::Corridor;
    }

    // Middle room row (y 5–8): Forge (left) / Crew Quarters (right)
    if (5..=8).contains(&y) {
        if (3..=11).contains(&x) {
            return ShipRoom::QuantumForge;
        }
        if (12..=20).contains(&x) {
            return ShipRoom::CrewQuarters;
        }
    }

    // Lower corridor (y 9)
    if y == 9 && (7..=15).contains(&x) {
        return ShipRoom::Corridor;
    }

    // Bottom room row (y 10–13): Engine (left) / Medbay (right)
    if (10..=13).contains(&y) {
        if (3..=11).contains(&x) {
            return ShipRoom::EngineRoom;
        }
        if (12..=20).contains(&x) {
            return ShipRoom::Medbay;
        }
    }

    // Cargo bay (y 14–17)
    if (14..=17).contains(&y) && (8..=15).contains(&x) {
        return ShipRoom::CargoBay;
    }

    ShipRoom::Corridor
}


#[cfg(test)]
mod tests {
    use super::*;

    // ── ShipRoom::name() ──

    #[test]
    fn ship_room_names() {
        assert_eq!(ShipRoom::Bridge.name(), "Bridge");
        assert_eq!(ShipRoom::EngineRoom.name(), "Engine Room");
        assert_eq!(ShipRoom::QuantumForge.name(), "Quantum Forge");
        assert_eq!(ShipRoom::CrewQuarters.name(), "Crew Quarters");
        assert_eq!(ShipRoom::CargoBay.name(), "Cargo Bay");
        assert_eq!(ShipRoom::Medbay.name(), "Medbay");
        assert_eq!(ShipRoom::WeaponsBay.name(), "Weapons Bay");
        assert_eq!(ShipRoom::Airlock.name(), "Airlock");
        assert_eq!(ShipRoom::Corridor.name(), "Corridor");
    }

    // ── ShipRoom::description() ──

    #[test]
    fn ship_room_descriptions_non_empty() {
        let rooms = [
            ShipRoom::Bridge,
            ShipRoom::EngineRoom,
            ShipRoom::QuantumForge,
            ShipRoom::CrewQuarters,
            ShipRoom::CargoBay,
            ShipRoom::Medbay,
            ShipRoom::WeaponsBay,
            ShipRoom::Airlock,
            ShipRoom::Corridor,
        ];
        for room in &rooms {
            assert!(!room.description().is_empty(), "{:?} has empty description", room);
        }
    }

    #[test]
    fn ship_room_descriptions_specific() {
        assert!(ShipRoom::Bridge.description().contains("Navigation"));
        assert!(ShipRoom::EngineRoom.description().contains("FTL"));
        assert!(ShipRoom::QuantumForge.description().contains("forge") || ShipRoom::QuantumForge.description().contains("Radical"));
        assert!(ShipRoom::Medbay.description().contains("Medical") || ShipRoom::Medbay.description().contains("Heal"));
        assert!(ShipRoom::Airlock.description().contains("Exit") || ShipRoom::Airlock.description().contains("explore"));
    }

    // ── ShipUpgrade::name() ──

    #[test]
    fn ship_upgrade_names() {
        assert_eq!(ShipUpgrade::ReinforcedHull.name(), "Reinforced Hull");
        assert_eq!(ShipUpgrade::ExtendedFuelTanks.name(), "Extended Fuel Tanks");
        assert_eq!(ShipUpgrade::AdvancedShields.name(), "Advanced Shields");
        assert_eq!(ShipUpgrade::CargoExpansion.name(), "Cargo Expansion");
        assert_eq!(ShipUpgrade::SensorArray.name(), "Sensor Array");
        assert_eq!(ShipUpgrade::AutoRepairDrone.name(), "Auto-Repair Drone");
        assert_eq!(ShipUpgrade::WeaponBooster.name(), "Weapon Booster");
        assert_eq!(ShipUpgrade::EngineBooster.name(), "Engine Booster");
        assert_eq!(ShipUpgrade::MedicalBay.name(), "Medical Bay Upgrade");
        assert_eq!(ShipUpgrade::QuantumForgeUpgrade.name(), "Quantum Forge Upgrade");
    }

    // ── ShipUpgrade::description() ──

    #[test]
    fn ship_upgrade_descriptions_non_empty() {
        for upgrade in ShipUpgrade::all() {
            assert!(!upgrade.description().is_empty(), "{:?} has empty description", upgrade);
        }
    }

    // ── ShipUpgrade::cost() ──

    #[test]
    fn ship_upgrade_costs() {
        assert_eq!(ShipUpgrade::ReinforcedHull.cost(), 200);
        assert_eq!(ShipUpgrade::ExtendedFuelTanks.cost(), 150);
        assert_eq!(ShipUpgrade::AdvancedShields.cost(), 250);
        assert_eq!(ShipUpgrade::CargoExpansion.cost(), 100);
        assert_eq!(ShipUpgrade::SensorArray.cost(), 175);
        assert_eq!(ShipUpgrade::AutoRepairDrone.cost(), 300);
        assert_eq!(ShipUpgrade::WeaponBooster.cost(), 225);
        assert_eq!(ShipUpgrade::EngineBooster.cost(), 275);
        assert_eq!(ShipUpgrade::MedicalBay.cost(), 200);
        assert_eq!(ShipUpgrade::QuantumForgeUpgrade.cost(), 350);
    }

    #[test]
    fn ship_upgrade_costs_all_positive() {
        for upgrade in ShipUpgrade::all() {
            assert!(upgrade.cost() > 0, "{:?} has non-positive cost", upgrade);
        }
    }

    // ── ShipUpgrade::all() ──

    #[test]
    fn ship_upgrade_all_returns_10() {
        assert_eq!(ShipUpgrade::all().len(), 10);
    }

    #[test]
    fn ship_upgrade_all_unique() {
        let all = ShipUpgrade::all();
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "Duplicate upgrade at indices {} and {}", i, j);
                }
            }
        }
    }

    // ── generate_ship_layout() ──

    #[test]
    fn layout_has_correct_dimensions() {
        let layout = generate_ship_layout();
        assert_eq!(layout.width, 24);
        assert_eq!(layout.height, 18);
        assert_eq!(layout.tiles.len(), (24 * 18) as usize);
    }

    #[test]
    fn layout_has_room_labels() {
        let layout = generate_ship_layout();
        assert!(!layout.room_labels.is_empty());
        let label_rooms: Vec<ShipRoom> = layout.room_labels.iter().map(|&(_, _, r)| r).collect();
        assert!(label_rooms.contains(&ShipRoom::Bridge));
        assert!(label_rooms.contains(&ShipRoom::EngineRoom));
        assert!(label_rooms.contains(&ShipRoom::QuantumForge));
        assert!(label_rooms.contains(&ShipRoom::CrewQuarters));
        assert!(label_rooms.contains(&ShipRoom::CargoBay));
        assert!(label_rooms.contains(&ShipRoom::Medbay));
        assert!(label_rooms.contains(&ShipRoom::WeaponsBay));
    }

    #[test]
    fn layout_has_doors() {
        let layout = generate_ship_layout();
        let door_count = layout.tiles.iter().filter(|t| matches!(t, ShipTile::Door)).count();
        assert!(door_count >= 8, "Expected at least 8 doors, found {}", door_count);
    }

    #[test]
    fn layout_has_consoles() {
        let layout = generate_ship_layout();
        let console_count = layout.tiles.iter().filter(|t| matches!(t, ShipTile::Console(_))).count();
        assert!(console_count >= 7, "Expected at least 7 consoles, found {}", console_count);
    }

    #[test]
    fn layout_has_floors() {
        let layout = generate_ship_layout();
        let floor_count = layout.tiles.iter().filter(|t| matches!(t, ShipTile::Floor)).count();
        assert!(floor_count > 50, "Expected many floor tiles, found {}", floor_count);
    }

    #[test]
    fn layout_has_crew_stations() {
        let layout = generate_ship_layout();
        let station_count = layout.tiles.iter().filter(|t| matches!(t, ShipTile::CrewStation(_))).count();
        assert_eq!(station_count, 4, "Expected 4 crew stations (pilot, gunner, engineer, medic)");
    }

    // ── tile_at() ──

    #[test]
    fn tile_at_out_of_bounds_returns_empty() {
        let layout = generate_ship_layout();
        assert_eq!(tile_at(&layout, -1, 0), ShipTile::Empty);
        assert_eq!(tile_at(&layout, 0, -1), ShipTile::Empty);
        assert_eq!(tile_at(&layout, 100, 0), ShipTile::Empty);
        assert_eq!(tile_at(&layout, 0, 100), ShipTile::Empty);
        assert_eq!(tile_at(&layout, -5, -5), ShipTile::Empty);
    }

    #[test]
    fn tile_at_valid_coords_returns_tile() {
        let layout = generate_ship_layout();
        // Corner should be a real tile (Wall or Empty)
        let t = tile_at(&layout, 0, 0);
        // Just verify it doesn't panic and returns something
        assert!(matches!(t, ShipTile::Floor | ShipTile::Wall | ShipTile::Empty
            | ShipTile::Door | ShipTile::Console(_) | ShipTile::CrewStation(_)
            | ShipTile::Decoration(_)));
    }

    #[test]
    fn tile_at_boundary() {
        let layout = generate_ship_layout();
        // Last valid coordinate
        let t = tile_at(&layout, 23, 17);
        assert!(matches!(t, ShipTile::Floor | ShipTile::Wall | ShipTile::Empty
            | ShipTile::Door | ShipTile::Console(_) | ShipTile::CrewStation(_)
            | ShipTile::Decoration(_)));
        // First out-of-bounds
        assert_eq!(tile_at(&layout, 24, 0), ShipTile::Empty);
        assert_eq!(tile_at(&layout, 0, 18), ShipTile::Empty);
    }

    // ── is_walkable() ──

    #[test]
    fn is_walkable_floor() {
        assert!(is_walkable(ShipTile::Floor));
    }

    #[test]
    fn is_walkable_door() {
        assert!(is_walkable(ShipTile::Door));
    }

    #[test]
    fn is_walkable_console() {
        assert!(is_walkable(ShipTile::Console(ShipRoom::Bridge)));
    }

    #[test]
    fn is_not_walkable_wall() {
        assert!(!is_walkable(ShipTile::Wall));
    }

    #[test]
    fn is_not_walkable_empty() {
        assert!(!is_walkable(ShipTile::Empty));
    }

    #[test]
    fn is_not_walkable_crew_station() {
        assert!(!is_walkable(ShipTile::CrewStation(0)));
    }

    #[test]
    fn is_not_walkable_decoration() {
        assert!(!is_walkable(ShipTile::Decoration(0)));
    }

    // ── get_console_room() ──

    #[test]
    fn get_console_room_at_console_tile() {
        let layout = generate_ship_layout();
        // Find a console tile
        let mut found = false;
        for y in 0..layout.height {
            for x in 0..layout.width {
                if let ShipTile::Console(room) = tile_at(&layout, x, y) {
                    let result = get_console_room(&layout, x, y);
                    assert_eq!(result, Some(room));
                    found = true;
                    break;
                }
            }
            if found { break; }
        }
        assert!(found, "No console tile found in layout");
    }

    #[test]
    fn get_console_room_at_non_console_returns_none() {
        let layout = generate_ship_layout();
        // Find a floor tile
        for y in 0..layout.height {
            for x in 0..layout.width {
                if tile_at(&layout, x, y) == ShipTile::Floor {
                    assert_eq!(get_console_room(&layout, x, y), None);
                    return;
                }
            }
        }
    }

    #[test]
    fn get_console_room_out_of_bounds_returns_none() {
        let layout = generate_ship_layout();
        assert_eq!(get_console_room(&layout, -1, -1), None);
    }

    // ── get_room_at() ──

    #[test]
    fn get_room_at_bridge_area() {
        let layout = generate_ship_layout();
        assert_eq!(get_room_at(&layout, 5, 1), ShipRoom::Bridge);
        assert_eq!(get_room_at(&layout, 8, 2), ShipRoom::Bridge);
    }

    #[test]
    fn get_room_at_weapons_bay() {
        let layout = generate_ship_layout();
        assert_eq!(get_room_at(&layout, 15, 1), ShipRoom::WeaponsBay);
    }

    #[test]
    fn get_room_at_quantum_forge() {
        let layout = generate_ship_layout();
        assert_eq!(get_room_at(&layout, 5, 6), ShipRoom::QuantumForge);
    }

    #[test]
    fn get_room_at_crew_quarters() {
        let layout = generate_ship_layout();
        assert_eq!(get_room_at(&layout, 15, 6), ShipRoom::CrewQuarters);
    }

    #[test]
    fn get_room_at_engine_room() {
        let layout = generate_ship_layout();
        assert_eq!(get_room_at(&layout, 5, 11), ShipRoom::EngineRoom);
    }

    #[test]
    fn get_room_at_medbay() {
        let layout = generate_ship_layout();
        assert_eq!(get_room_at(&layout, 15, 11), ShipRoom::Medbay);
    }

    #[test]
    fn get_room_at_cargo_bay() {
        let layout = generate_ship_layout();
        assert_eq!(get_room_at(&layout, 10, 15), ShipRoom::CargoBay);
    }

    #[test]
    fn get_room_at_upper_corridor() {
        let layout = generate_ship_layout();
        assert_eq!(get_room_at(&layout, 10, 4), ShipRoom::Corridor);
    }

    #[test]
    fn get_room_at_lower_corridor() {
        let layout = generate_ship_layout();
        assert_eq!(get_room_at(&layout, 10, 9), ShipRoom::Corridor);
    }

    #[test]
    fn get_room_at_defaults_to_corridor() {
        let layout = generate_ship_layout();
        // Outside all defined room regions
        assert_eq!(get_room_at(&layout, 0, 0), ShipRoom::Corridor);
    }

    // ── Layout consistency ──

    #[test]
    fn all_consoles_have_known_rooms() {
        let layout = generate_ship_layout();
        for y in 0..layout.height {
            for x in 0..layout.width {
                if let ShipTile::Console(room) = tile_at(&layout, x, y) {
                    // Every console room should have a valid name
                    assert!(!room.name().is_empty());
                }
            }
        }
    }

    #[test]
    fn doors_connect_walkable_areas() {
        let layout = generate_ship_layout();
        for y in 0..layout.height {
            for x in 0..layout.width {
                if tile_at(&layout, x, y) == ShipTile::Door {
                    // At least one adjacent tile should be walkable (Floor, Door, or Console)
                    let neighbors = [
                        tile_at(&layout, x - 1, y),
                        tile_at(&layout, x + 1, y),
                        tile_at(&layout, x, y - 1),
                        tile_at(&layout, x, y + 1),
                    ];
                    let walkable_neighbors = neighbors.iter().filter(|t| is_walkable(**t)).count();
                    assert!(walkable_neighbors >= 1,
                        "Door at ({},{}) has no walkable neighbors", x, y);
                }
            }
        }
    }

    #[test]
    fn layout_deterministic() {
        let l1 = generate_ship_layout();
        let l2 = generate_ship_layout();
        assert_eq!(l1.width, l2.width);
        assert_eq!(l1.height, l2.height);
        assert_eq!(l1.tiles.len(), l2.tiles.len());
        for (i, (t1, t2)) in l1.tiles.iter().zip(l2.tiles.iter()).enumerate() {
            assert_eq!(t1, t2, "Tiles differ at index {}", i);
        }
    }
}
