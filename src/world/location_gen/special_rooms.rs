//! Special room layout generation.

use super::super::*;

impl LocationLevel {
    /// Generate tile layout for a special compartment.
    pub(super) fn generate_special_room(&mut self, kind: SpecialRoomKind, room: &Room, rng: &mut Rng) {
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2;
        match kind {
            SpecialRoomKind::CargoBay => {
                // Gold piles scattered around the vault
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 7 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CreditCache;
                            }
                        }
                    }
                }
                // Water moat around center treasure area
                for &(dx, dy) in &[(-2,-1),(-2,0),(-2,1),(2,-1),(2,0),(2,1),(-1,-2),(0,-2),(1,-2),(-1,2),(0,2),(1,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::CreditCache) {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Crate barriers flanking the vault
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1),(3,-1),(3,0),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::CreditCache) {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // 3 chests in center
                for dx in -1..=1 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
                // Locked door at room edge
                if self.in_bounds(room.x, cy) {
                    let idx = self.idx(room.x, cy);
                    if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Hallway) {
                        self.tiles[idx] = Tile::SealedHatch;
                    }
                }
            }
            SpecialRoomKind::OreDeposit => {
                // Dense gold ore on walls (higher coverage)
                for x in room.x..room.x + room.w {
                    for y in [room.y, room.y + room.h - 1] {
                        if self.in_bounds(x, y) && rng.next_u64() % 2 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::OreVein;
                            }
                        }
                    }
                }
                for y in room.y..room.y + room.h {
                    for x in [room.x, room.x + room.w - 1] {
                        if self.in_bounds(x, y) && rng.next_u64() % 2 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::OreVein;
                            }
                        }
                    }
                }
                // Boulder support pillars
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2),(0,-2),(0,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Gold piles near veins
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 8 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CreditCache;
                            }
                        }
                    }
                }
                // Crate (mine cart) at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SalvageCrate;
                    }
                }
            }
            SpecialRoomKind::HallOfRecords => {
                // Crystal candles along walls
                for y in room.y + 1..room.y + room.h - 1 {
                    if (y - room.y) % 3 == 1 {
                        for &x in &[room.x + 1, room.x + room.w - 2] {
                            if self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::MetalFloor {
                                    self.tiles[idx] = Tile::CrystalPanel;
                                }
                            }
                        }
                    }
                }
                // Bookshelf pew rows on both sides of center aisle
                for y in room.y + 1..room.y + room.h - 1 {
                    if y != cy && (y - room.y) % 2 == 0 {
                        for &x_off in &[2, 3] {
                            let x_left = room.x + x_off;
                            let x_right = room.x + room.w - 1 - x_off;
                            if self.in_bounds(x_left, y) {
                                let idx = self.idx(x_left, y);
                                if self.tiles[idx] == Tile::MetalFloor {
                                    self.tiles[idx] = Tile::DataRack;
                                }
                            }
                            if self.in_bounds(x_right, y) {
                                let idx = self.idx(x_right, y);
                                if self.tiles[idx] == Tile::MetalFloor {
                                    self.tiles[idx] = Tile::DataRack;
                                }
                            }
                        }
                    }
                }
                // Water basin behind altar
                for dx in -1..=1 {
                    let x = cx + dx;
                    let y = cy - 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Jade altar at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Terminal(AltarKind::Quantum);
                }
            }
            SpecialRoomKind::HiddenCache => {
                // Oil (dust) and mushroom (cobwebs) in neglected corners
                for &(dx, dy) in &[(1,1),(1,-1),(-1,1),(-1,-1)] {
                    let x = cx + dx * 2;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                for &(dx, _dy) in &[(1,0),(-1,0),(2,0),(-2,0)] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Coolant;
                        }
                    }
                }
                // Crate (old supplies)
                if self.in_bounds(cx + 1, cy - 1) {
                    let idx = self.idx(cx + 1, cy - 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SalvageCrate;
                    }
                }
                // Chest at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
                // Cracked wall entrance
                let wall_x = room.x;
                if self.in_bounds(wall_x, cy) {
                    let idx = self.idx(wall_x, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::DamagedBulkhead;
                    }
                }
                // More cracked walls for atmosphere
                if self.in_bounds(wall_x, cy - 1) {
                    let idx = self.idx(wall_x, cy - 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::DamagedBulkhead;
                    }
                }
            }
            SpecialRoomKind::OfferingTerminal => {
                // Water purification basin
                for dx in -1..=1 {
                    let x = cx + dx;
                    let y = cy + 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Crystal candles at diagonal positions
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Mushroom incense at far positions
                for &(dx, dy) in &[(-2,-1),(2,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Gold pile offerings at cardinal positions
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CreditCache;
                        }
                    }
                }
                // Central altar
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Terminal(AltarKind::Commerce);
                }
            }
            SpecialRoomKind::HydroponicsGarden => {
                // Bamboo border along room edges
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) && x != cx {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CargoPipes;
                            }
                        }
                    }
                }
                for y in room.y + 2..room.y + room.h - 2 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) && y != cy {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CargoPipes;
                            }
                        }
                    }
                }
                // Water pond around center (2-tile radius)
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        if dx == 0 && dy == 0 { continue; }
                        if dx.abs() + dy.abs() > 3 { continue; }
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CoolantPool;
                            }
                        }
                    }
                }
                // Mushroom (lotus plants) near water
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Med bay at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::MedBayTile;
                }
            }
            SpecialRoomKind::PirateStash => {
                // Gold ore on walls
                for x in room.x..room.x + room.w {
                    for y in [room.y, room.y + room.h - 1] {
                        if self.in_bounds(x, y) && rng.next_u64() % 2 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::OreVein;
                            }
                        }
                    }
                }
                // Dense gold piles throughout
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 3 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CreditCache;
                            }
                        }
                    }
                }
                // Crystal gems scattered
                for &(dx, dy) in &[(-2,0),(2,0),(0,-2),(0,2),(-1,-1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Crate treasure boxes at corners
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::CreditCache) {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
            }
            SpecialRoomKind::XenoCrystalVault => {
                // Crystal walls along full inner perimeter
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CrystalPanel;
                            }
                        }
                    }
                }
                for y in room.y + 2..room.y + room.h - 2 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CrystalPanel;
                            }
                        }
                    }
                }
                // Gold piles near chest
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CreditCache;
                        }
                    }
                }
                // Jade altar behind chest
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    self.tiles[idx] = Tile::Terminal(AltarKind::Quantum);
                }
                // Chest in center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
                // Locked door at entry
                if self.in_bounds(room.x, cy) {
                    let idx = self.idx(room.x, cy);
                    if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Hallway | Tile::CrystalPanel) {
                        self.tiles[idx] = Tile::SealedHatch;
                    }
                }
            }
            SpecialRoomKind::RelicChamber => {
                // Bookshelves (lore) on walls
                for y in room.y + 1..room.y + room.h - 1 {
                    if (y - room.y) % 2 == 1 {
                        for &x in &[room.x + 1, room.x + room.w - 2] {
                            if self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::MetalFloor {
                                    self.tiles[idx] = Tile::DataRack;
                                }
                            }
                        }
                    }
                }
                // Water purification pool at entry
                for dx in -1..=1 {
                    let x = cx + dx;
                    let y = cy + 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Crystal lighting flanks
                for &(dx, dy) in &[(-2, 0), (2, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Mirror altar backdrop
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    self.tiles[idx] = Tile::Terminal(AltarKind::Holographic);
                }
                // Chest (relic) at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::MedBay => {
                // Bamboo accents at room edges
                for &(dx, dy) in &[(-3,-2),(-3,2),(3,-2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoPipes;
                        }
                    }
                }
                // Water ring (2-wide)
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        if dx == 0 && dy == 0 { continue; }
                        let dist = dx.abs().max(dy.abs());
                        if dist >= 1 && dist <= 2 {
                            let x = cx + dx;
                            let y = cy + dy;
                            if self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::MetalFloor {
                                    self.tiles[idx] = Tile::CoolantPool;
                                }
                            }
                        }
                    }
                }
                // Crystal accents at compass points
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Mushroom herbs near water
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::CoolantPool) {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Med bay at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::MedBayTile;
                }
            }
            SpecialRoomKind::ArenaChallenge => {
                // Oil (sand/blood) scattered in center area
                for y in room.y + 2..room.y + room.h - 2 {
                    for x in room.x + 2..room.x + room.w - 2 {
                        if self.in_bounds(x, y) && rng.next_u64() % 5 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Coolant;
                            }
                        }
                    }
                }
                // Spikes inner border
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Coolant) {
                                self.tiles[idx] = Tile::LaserGrid;
                            }
                        }
                    }
                }
                for y in room.y + 1..room.y + room.h - 1 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Coolant) {
                                self.tiles[idx] = Tile::LaserGrid;
                            }
                        }
                    }
                }
                // Boulder pillars at corners
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Coolant) {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Pressure plate arena markers
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx * 2;
                    let y = cy + dy * 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Coolant) {
                            self.tiles[idx] = Tile::PressureSensor;
                        }
                    }
                }
            }
            SpecialRoomKind::SecurityCheckpoint => {
                // Spikes (caltrops) near entrance
                for dx in [-2, -1, 1, 2] {
                    let x = cx + dx;
                    let y = cy - 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::LaserGrid;
                        }
                    }
                }
                // Crate barricades on sides
                for &(dx, dy) in &[(-2,-1),(-2,1),(2,-1),(2,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Boulder obstacles
                for &(dx, dy) in &[(-3, 0), (3, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Pressure plates alarm strip
                for x in room.x + 1..room.x + room.w - 1 {
                    if self.in_bounds(x, cy) && x != cx {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PressureSensor;
                        }
                    }
                }
            }
            SpecialRoomKind::SecurityZone => {
                // Oil slicks between spike rows
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x + y) % 3 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Coolant;
                            }
                        }
                    }
                }
                // Dense spike checkerboard
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x + y) % 2 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::LaserGrid;
                            }
                        }
                    }
                }
                // Boulder safe spots for cover
                for &(dx, dy) in &[(0,-1),(0,1),(-2,0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::CargoCrate;
                    }
                }
                // Pressure plate triggers
                for &(dx, dy) in &[(-1,-2),(1,-2),(-1,2),(1,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::PressureSensor;
                    }
                }
                // Chest reward at far end
                let rx = room.x + room.w - 2;
                if self.in_bounds(rx, cy) {
                    let idx = self.idx(rx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::ToxicMaze => {
                // Gas walls forming corridors (structured pattern)
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let rel_x = x - room.x;
                            let rel_y = y - room.y;
                            let is_wall = (rel_x % 3 != 0) && (rel_y % 3 != 0);
                            if is_wall {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::MetalFloor {
                                    self.tiles[idx] = Tile::ToxicGas;
                                }
                            }
                        }
                    }
                }
                // Mushroom patches in gas corridors
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 8 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::ToxicGas {
                                self.tiles[idx] = Tile::ToxicFungus;
                            }
                        }
                    }
                }
                // Water safe zones at corners
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::ToxicGas {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Chest reward at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::GravityPuzzle => {
                // Pressure plates at puzzle positions
                let plate_positions = [
                    (cx - 1, cy - 1), (cx + 1, cy - 1),
                    (cx - 1, cy + 1), (cx + 1, cy + 1),
                ];
                let boulder_positions = [
                    (cx - 1, cy), (cx + 1, cy),
                    (cx, cy - 1), (cx, cy + 1),
                ];
                for &(px, py) in &plate_positions {
                    if self.in_bounds(px, py) {
                        let idx = self.idx(px, py);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PressureSensor;
                        }
                    }
                }
                for &(bx, by) in &boulder_positions {
                    if self.in_bounds(bx, by) {
                        let idx = self.idx(bx, by);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Crystal markers near plates
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Cracked wall hints
                for &x in &[room.x + 1, room.x + room.w - 2] {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DamagedBulkhead;
                        }
                    }
                }
                // Oil ground texture
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 6 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Coolant;
                            }
                        }
                    }
                }
                // Chest reward at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::HolographicRoom => {
                // Ice reflective floor sections
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x + y) % 3 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::FrozenDeck;
                            }
                        }
                    }
                }
                // Water accents at edges
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::FrozenDeck) {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Crystal walls forming diamond pattern
                for &(dx, dy) in &[(-2,0),(2,0),(0,-2),(0,2),(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::CrystalPanel;
                    }
                }
                // Mirror pool at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::HoloPool;
                }
            }
            SpecialRoomKind::EnergyNexus => {
                // Elemental tiles around each altar
                // Water around Gale (west)
                for &(dx, dy) in &[(-3,-1),(-3,1),(-1,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Lava around Iron (east)
                for &(dx, dy) in &[(3,-1),(3,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PlasmaVent;
                        }
                    }
                }
                // Ice around Mirror (north)
                for &(dx, dy) in &[(-1,-3),(1,-3),(-1,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::FrozenDeck;
                        }
                    }
                }
                // Crystal around Gold (south)
                for &(dx, dy) in &[(-1,3),(1,3),(1,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Bamboo around Jade (center)
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoPipes;
                        }
                    }
                }
                // 5 elemental altars in cross pattern
                let positions = [
                    (cx, cy, AltarKind::Quantum),
                    (cx - 2, cy, AltarKind::Stellar),
                    (cx + 2, cy, AltarKind::Tactical),
                    (cx, cy - 2, AltarKind::Holographic),
                    (cx, cy + 2, AltarKind::Commerce),
                ];
                for &(x, y, kind) in &positions {
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::Terminal(kind);
                    }
                }
            }
            SpecialRoomKind::SpaceHulkRise => {
                // Grid of spikes (tombstones) in rows
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x - room.x) % 2 == 1 && (y - room.y) % 2 == 1 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::LaserGrid;
                            }
                        }
                    }
                }
                // Oil (disturbed earth) between tombstones
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 5 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Coolant;
                            }
                        }
                    }
                }
                // Mushroom (dead flowers) near graves
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2),(0,0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // CursedFloor path through cemetery
                for x in room.x + 1..room.x + room.w - 1 {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CorruptedFloor;
                        }
                    }
                }
            }
            SpecialRoomKind::PirateAmbush => {
                // Oil (escape routes / slippery floor)
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 6 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Coolant;
                            }
                        }
                    }
                }
                // Crate hiding spots forming ambush positions
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2),(-3,0),(3,0),(0,-2),(0,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Coolant) {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Hidden traps near gold
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        let trap_type = (rng.next_u64() % 3) as u8;
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Coolant) {
                            self.tiles[idx] = Tile::Trap(trap_type);
                        }
                    }
                }
                // Gold bait in center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::CreditCache;
                }
            }
            SpecialRoomKind::DuelArena => {
                // Oil (sand pit) inside circle
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Coolant;
                            }
                        }
                    }
                }
                // Spikes outer boundary
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3),(-3,-1),(-3,1),(3,-1),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::LaserGrid;
                        }
                    }
                }
                // Boulder pillars at far corners
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Pressure plate ring
                for &(dx, dy) in &[
                    (-2,0),(2,0),(0,-2),(0,2),
                    (-1,-1),(1,-1),(-1,1),(1,1),
                ] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::PressureSensor;
                    }
                }
            }
            SpecialRoomKind::DataArchive => {
                // Rows of bookshelves with center aisle
                for y in room.y + 1..room.y + room.h - 1 {
                    if (y - room.y) % 2 == 0 {
                        for x in room.x + 1..room.x + room.w - 1 {
                            if x != cx && self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::MetalFloor {
                                    self.tiles[idx] = Tile::DataRack;
                                }
                            }
                        }
                    }
                }
                // Crystal lanterns at end of bookshelf rows
                for y in room.y + 1..room.y + room.h - 1 {
                    if (y - room.y) % 2 == 0 {
                        for &x in &[room.x + 1, room.x + room.w - 2] {
                            if self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::DataRack {
                                    self.tiles[idx] = Tile::CrystalPanel;
                                }
                            }
                        }
                    }
                }
                // Crate (reading desks) in aisle
                for y in room.y + 2..room.y + room.h - 2 {
                    if (y - room.y) % 4 == 1 && self.in_bounds(cx, y) {
                        let idx = self.idx(cx, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // InkWell at study positions
                for &(dx, dy) in &[(-1, 1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataWell;
                        }
                    }
                }
            }
            SpecialRoomKind::SignalHall => {
                // Crystal lighting overhead
                for &x in &[room.x + 1, room.x + room.w - 2] {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Crate storage at corners
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Writing stations: bookshelf + inkwell pairs
                for dx in [-2, 0, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                    if self.in_bounds(x, cy + 1) {
                        let idx = self.idx(x, cy + 1);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataWell;
                        }
                    }
                }
                // Additional bookshelf rows
                for dx in [-2, 0, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy - 1) {
                        let idx = self.idx(x, cy - 1);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
            }
            SpecialRoomKind::ResearcherStudy => {
                // Bookshelves on north and south walls
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::DataRack;
                            }
                        }
                    }
                }
                // Crystal lamp
                if self.in_bounds(cx + 1, cy) {
                    let idx = self.idx(cx + 1, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::CrystalPanel;
                    }
                }
                // InkWell at desk
                if self.in_bounds(cx - 1, cy) {
                    let idx = self.idx(cx - 1, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::DataWell;
                    }
                }
                // Crate (desk)
                if self.in_bounds(cx, cy + 1) {
                    let idx = self.idx(cx, cy + 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SalvageCrate;
                    }
                }
                // Codex shrine at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::CodexTerminal;
                }
            }
            SpecialRoomKind::SensorArray => {
                // Water border accents
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Bamboo curtains at edges
                for &(dx, dy) in &[(-3,-2),(-3,2),(3,-2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoPipes;
                        }
                    }
                }
                // Mushroom (incense) at corners
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Crystal ring around mirror pool
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1),(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::CrystalPanel;
                    }
                }
                // Mirror pool at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::HoloPool;
                }
            }
            SpecialRoomKind::InscriptionWall => {
                // Full bookshelf walls on left and right
                for y in room.y + 1..room.y + room.h - 1 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::DataRack;
                            }
                        }
                    }
                }
                // Crystal lighting
                for &(dx, dy) in &[(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // InkWell for practice
                if self.in_bounds(cx + 1, cy) {
                    let idx = self.idx(cx + 1, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::DataWell;
                    }
                }
                // Radical garden at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::RadicalLab;
                }
            }
            SpecialRoomKind::TrainingSimulator => {
                // Oil (training mat) in center area
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Coolant;
                            }
                        }
                    }
                }
                // Crate (weapon rack) at side
                for &(dx, dy) in &[(-3, -1), (-3, 0), (-3, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Boulder (heavy bag) obstacles
                for &(dx, dy) in &[(3, -1), (3, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Pressure plates (training dummies)
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::PressureSensor;
                    }
                }
            }
            SpecialRoomKind::ZenChamber => {
                // Bamboo border accents
                for &(dx, dy) in &[(-3,-2),(-3,2),(3,-2),(3,2),(-3,0),(3,0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoPipes;
                        }
                    }
                }
                // Mushroom (incense) at corners
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Water at cardinal positions
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Crystal candles at diagonal positions
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Med bay at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::MedBayTile;
                }
            }
            SpecialRoomKind::TranslationChallenge => {
                // Bookshelves (reference texts) behind altars
                for dx in -2..=2 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy - 1) {
                        let idx = self.idx(x, cy - 1);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
                // Crystal lighting
                for &(dx, dy) in &[(-2, 0), (2, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // InkWell at translation stations
                for dx in -1..=1 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy + 1) {
                        let idx = self.idx(x, cy + 1);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataWell;
                        }
                    }
                }
                // Translation altars in a row
                for dx in -1..=1 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        self.tiles[idx] = Tile::TranslationTerminal;
                    }
                }
            }
            SpecialRoomKind::AncientDatapad => {
                // Mushroom (cobwebs) in corners
                for &(dx, dy) in &[(-2,-1),(2,-1),(-2,1),(2,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Crystal lighting
                for &(dx, dy) in &[(-1, 0), (1, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Cracked wall (age damage)
                if self.in_bounds(room.x + 1, cy + 1) {
                    let idx = self.idx(room.x + 1, cy + 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::DamagedBulkhead;
                    }
                }
                // Bookshelf alcove (expanded)
                for dx in -1..=1 {
                    let x = cx + dx;
                    for &y in &[cy - 1, cy - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::DataRack;
                            }
                        }
                    }
                }
                // Chest (scroll) at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::WisdomCore => {
                // Bookshelf (wisdom texts) at walls
                for &(dx, dy) in &[(-3, -1), (-3, 0), (-3, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
                // Crystal markers around well
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Water ring around well
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1),(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Deep water well at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::VacuumBreach;
                }
                // Stone tutor nearby
                if self.in_bounds(cx + 2, cy) {
                    let idx = self.idx(cx + 2, cy);
                    self.tiles[idx] = Tile::DroidTutor;
                }
            }
            SpecialRoomKind::FloodedCompartment => {
                // Most floor tiles become water, some deep water
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = if rng.next_u64() % 5 == 0 {
                                    Tile::VacuumBreach
                                } else {
                                    Tile::CoolantPool
                                };
                            }
                        }
                    }
                }
                // Safe path through center row
                for x in room.x..room.x + room.w {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if matches!(self.tiles[idx], Tile::CoolantPool | Tile::VacuumBreach) {
                            self.tiles[idx] = Tile::MetalFloor;
                        }
                    }
                }
                // Boulder stepping stones
                for &(dx, dy) in &[(-2,-2),(0,-2),(2,-2),(-2,2),(0,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::CoolantPool | Tile::VacuumBreach) {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Crystal cave formations
                for &(dx, dy) in &[(-3, 0), (3, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::CoolantPool | Tile::VacuumBreach) {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Mushroom (algae) patches
                for _ in 0..3 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::CoolantPool {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
            }
            SpecialRoomKind::CryogenicBay => {
                // Ice tiles everywhere
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::FrozenDeck;
                            }
                        }
                    }
                }
                // Crystal pillars
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1), (0, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::FrozenDeck {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Boulder (frozen rocks)
                for &(dx, dy) in &[(-3, 0), (3, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::FrozenDeck {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Water (frozen puddle edges)
                for &(dx, dy) in &[(-1,-2),(1,-2),(-1,2),(1,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::FrozenDeck {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
            }
            SpecialRoomKind::PlasmaCrossing => {
                // Lava floor
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::PlasmaVent;
                            }
                        }
                    }
                }
                // Safe cross-shaped path
                for x in room.x..room.x + room.w {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::PlasmaVent {
                            self.tiles[idx] = Tile::MetalFloor;
                        }
                    }
                }
                for y in room.y..room.y + room.h {
                    if self.in_bounds(cx, y) {
                        let idx = self.idx(cx, y);
                        if self.tiles[idx] == Tile::PlasmaVent {
                            self.tiles[idx] = Tile::MetalFloor;
                        }
                    }
                }
                // Boulder (heat-resistant rocks) on safe paths
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Gold ore (volcanic deposits) on walls
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::PlasmaVent {
                            self.tiles[idx] = Tile::OreVein;
                        }
                    }
                }
                // Chest reward at far corner
                let rx = room.x + room.w - 2;
                let ry = room.y + 1;
                if self.in_bounds(rx, ry) {
                    let idx = self.idx(rx, ry);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::PipeForest => {
                // Dense bamboo with gaps for navigation
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 3 != 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CargoPipes;
                            }
                        }
                    }
                }
                // Ensure center is clear
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::CargoPipes {
                                self.tiles[idx] = Tile::MetalFloor;
                            }
                        }
                    }
                }
                // Ensure paths from edges to center
                for x in room.x..room.x + room.w {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::CargoPipes {
                            self.tiles[idx] = Tile::MetalFloor;
                        }
                    }
                }
                // Water stream along one axis
                for y in room.y + 2..room.y + room.h - 2 {
                    let x = cx - 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::CargoPipes {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Mushroom undergrowth
                for &(dx, dy) in &[(-1,-2),(1,-2),(-1,2),(1,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::CargoPipes {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Crystal (lanterns/fireflies)
                for &(dx, dy) in &[(0,-2),(0,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::CargoPipes) {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
            }
            SpecialRoomKind::VentTunnel => {
                // Oil (wind streaks) along center
                for x in room.x + 1..room.x + room.w - 1 {
                    if self.in_bounds(x, cy) && x % 2 == 0 {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Coolant;
                        }
                    }
                }
                // Spike barriers on sides
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[cy - 1, cy + 1] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::LaserGrid;
                            }
                        }
                    }
                }
                // Pressure plate wind triggers
                for &dx in &[-2, 0, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Coolant) {
                            self.tiles[idx] = Tile::PressureSensor;
                        }
                    }
                }
                // Crystal wind chimes at walls
                for &(dx, dy) in &[(-3,-1),(3,-1),(-3,1),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::LaserGrid) {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
            }
            SpecialRoomKind::CrystalCave => {
                // Water (cave pools) scattered
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 8 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CoolantPool;
                            }
                        }
                    }
                }
                // Ice (cold spots)
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::FrozenDeck;
                        }
                    }
                }
                // Mushroom (cave growth)
                for _ in 0..3 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Crystal formations (denser, with pattern)
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 5 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CrystalPanel;
                            }
                        }
                    }
                }
            }
            SpecialRoomKind::FungalGrotto => {
                // Mushroom clusters scattered
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 4 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::ToxicFungus;
                            }
                        }
                    }
                }
                // Water puddles
                for _ in 0..4 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Poison gas spore clouds near mushroom clusters
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 10 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::ToxicGas;
                            }
                        }
                    }
                }
                // Crystal bioluminescence
                for &(dx, dy) in &[(-2,0),(2,0),(0,-2),(0,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::ToxicFungus) {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
            }
            SpecialRoomKind::CoolantRiver => {
                // River of water/deep water running through middle
                for x in room.x..room.x + room.w {
                    for dy in -1..=1 {
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = if dy == 0 {
                                    Tile::VacuumBreach
                                } else {
                                    Tile::CoolantPool
                                };
                            }
                        }
                    }
                }
                // Boulder (river rocks) in shallows
                for &(dx, dy) in &[(-3, -1), (-1, 1), (2, -1), (3, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::CoolantPool | Tile::VacuumBreach) {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Crystal cave formations on banks
                for &(dx, dy) in &[(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Mushroom (riverside growth)
                for &(dx, dy) in &[(-1, -2), (1, -2), (-1, 2), (1, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Bridge crossing at center
                for dy in -1..=1 {
                    if self.in_bounds(cx, cy + dy) {
                        let idx = self.idx(cx, cy + dy);
                        self.tiles[idx] = Tile::Catwalk;
                    }
                }
            }
            SpecialRoomKind::EchoingHull => {
                // Water (sound-amplifying pools)
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Boulder (echo-generating rock formations)
                for &(dx, dy) in &[(-1, -2), (1, -2), (-1, 2), (1, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Ice patches (frozen condensation)
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::FrozenDeck;
                        }
                    }
                }
                // Crystal echo points at compass positions
                for &(dx, dy) in &[(-3, 0), (3, 0), (0, -3), (0, 3)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::CrystalPanel;
                    }
                }
            }
            SpecialRoomKind::DarkSector => {
                // Oil (shadow puddles)
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 4 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Coolant;
                            }
                        }
                    }
                }
                // CursedFloor patches
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1),(0,0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Coolant) {
                            self.tiles[idx] = Tile::CorruptedFloor;
                        }
                    }
                }
                // Crystal (dim lights at far edges)
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Coolant) {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Hidden traps
                for _ in 0..5 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Coolant) {
                            let trap_type = (rng.next_u64() % 3) as u8;
                            self.tiles[idx] = Tile::Trap(trap_type);
                        }
                    }
                }
            }
            SpecialRoomKind::WanderingMerchant => {
                // Bookshelf (wares display) at walls
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1),(3,-1),(3,0),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
                // Gold pile (display items)
                for &(dx, dy) in &[(-1, 1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CreditCache;
                        }
                    }
                }
                // Crate inventory around shop
                for &(dx, dy) in &[(-1, -1), (1, -1), (-2, 0), (2, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Shop at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::TradeTerminal;
                }
            }
            SpecialRoomKind::HermitSage => {
                // Crystal lantern
                if self.in_bounds(cx + 2, cy) {
                    let idx = self.idx(cx + 2, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::CrystalPanel;
                    }
                }
                // Mushroom (herb garden)
                for &(dx, dy) in &[(1, 1), (2, 1), (1, -1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Water (tea/medicine)
                if self.in_bounds(cx - 1, cy + 1) {
                    let idx = self.idx(cx - 1, cy + 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::CoolantPool;
                    }
                }
                // Bookshelves (study)
                for dx in -1..=1 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy - 1) {
                        let idx = self.idx(x, cy - 1);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
                // Crate (storage)
                if self.in_bounds(cx - 2, cy) {
                    let idx = self.idx(cx - 2, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SalvageCrate;
                    }
                }
                // Hermit NPC
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Npc(1);
                }
            }
            SpecialRoomKind::DetentionCell => {
                // Spikes (chains on walls)
                for y in room.y + 1..room.y + room.h - 1 {
                    if self.in_bounds(room.x + room.w - 2, y) && y != cy {
                        let idx = self.idx(room.x + room.w - 2, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::LaserGrid;
                        }
                    }
                }
                // Water puddle
                if self.in_bounds(cx + 1, cy + 1) {
                    let idx = self.idx(cx + 1, cy + 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::CoolantPool;
                    }
                }
                // Crate (bed/bench)
                if self.in_bounds(cx - 1, cy + 1) {
                    let idx = self.idx(cx - 1, cy + 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SalvageCrate;
                    }
                }
                // Iron bars (brittle walls)
                for y in room.y + 1..room.y + room.h - 1 {
                    if y != cy && self.in_bounds(room.x, y) {
                        let idx = self.idx(room.x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::WeakBulkhead;
                        }
                    }
                }
                // NPC prisoner
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Npc(3);
                }
                // Locked door
                if self.in_bounds(room.x, cy) {
                    let idx = self.idx(room.x, cy);
                    if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Hallway) {
                        self.tiles[idx] = Tile::SealedHatch;
                    }
                }
            }
            SpecialRoomKind::MemorialShrine => {
                // Bamboo sacred grove at edges
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1),(3,-1),(3,0),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoPipes;
                        }
                    }
                }
                // Crystal votive candles
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Water blessing pool
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Shrine at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::NavBeacon;
                }
            }
            SpecialRoomKind::CantinaBay => {
                // Bamboo decoration at corners
                for &(dx, dy) in &[(-3,-2),(-3,2),(3,-2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoPipes;
                        }
                    }
                }
                // Water ornamental feature
                for &(dx, dy) in &[(0, 2), (-1, 2), (1, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Crate (tea tables)
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Bookshelves (menu/scrolls) on sides
                for &dx in &[-2, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
                // Med bay (tea pot warmth)
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    self.tiles[idx] = Tile::MedBayTile;
                }
                // Tea house keeper NPC
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Npc(2);
                }
            }
            SpecialRoomKind::EngineerWorkshop => {
                // Gold ore (raw materials) at walls
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::OreVein;
                        }
                    }
                }
                // Lava heat source (expanded)
                for &(dx, dy) in &[(0, 1), (-1, 1), (1, 1), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PlasmaVent;
                        }
                    }
                }
                // Crate (tools) on sides
                for &(dx, dy) in &[(-2, 0), (2, 0), (2, -1), (-2, -1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Boulder (anvil)
                if self.in_bounds(cx + 1, cy) {
                    let idx = self.idx(cx + 1, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::CargoCrate;
                    }
                }
                // Forge at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::QuantumForge;
                }
            }
            SpecialRoomKind::ChemLab => {
                // Water (vials/solutions) scattered
                for &(dx, dy) in &[(0, -1), (1, -1), (-1, -1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Bookshelves (recipe books)
                for &(dx, dy) in &[(-2, -1), (-2, 0), (-2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
                // Mushroom reagents
                for &(dx, dy) in &[(2, -1), (2, 0), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Crystal (reagent containers)
                for &(dx, dy) in &[(-1, 1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Poison gas (fume hood exhaust)
                if self.in_bounds(cx, cy + 2) {
                    let idx = self.idx(cx, cy + 2);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::ToxicGas;
                    }
                }
                // Forge (cauldron) at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::QuantumForge;
                }
            }
            SpecialRoomKind::FortuneTeller => {
                // Bamboo curtains
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1),(3,-1),(3,0),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoPipes;
                        }
                    }
                }
                // Oil (mysterious aura)
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Coolant;
                        }
                    }
                }
                // Mushroom (incense)
                for &(dx, dy) in &[(-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::ToxicFungus;
                        }
                    }
                }
                // Crystal ball
                if self.in_bounds(cx, cy + 1) {
                    let idx = self.idx(cx, cy + 1);
                    self.tiles[idx] = Tile::CrystalPanel;
                }
                // Mirror pool (scrying)
                if self.in_bounds(cx - 1, cy) {
                    let idx = self.idx(cx - 1, cy);
                    self.tiles[idx] = Tile::HoloPool;
                }
                // Fortune teller NPC
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Npc(0);
                }
            }
            SpecialRoomKind::RefugeeBay => {
                // Oil (campfire residue)
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Coolant;
                        }
                    }
                }
                // Water (well/water source)
                if self.in_bounds(cx + 2, cy + 1) {
                    let idx = self.idx(cx + 2, cy + 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::CoolantPool;
                    }
                }
                // Crate (supplies, expanded)
                for &(dx, dy) in &[(-1, 1), (0, -1), (2, -1), (-2, -1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Coolant) {
                            self.tiles[idx] = Tile::SalvageCrate;
                        }
                    }
                }
                // Med bay (campfire warmth)
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::MedBayTile;
                }
                // NPCs (refugees)
                for &(dx, dy) in &[(-2, 0), (1, -1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::Coolant) {
                            let npc_type = (rng.next_u64() % 4) as u8;
                            self.tiles[idx] = Tile::Npc(npc_type);
                        }
                    }
                }
            }
            SpecialRoomKind::WarpGate => {
                // Gold pile offerings around approach
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3),(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CreditCache;
                        }
                    }
                }
                // Lava moat
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1),(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::CreditCache) {
                            self.tiles[idx] = Tile::PlasmaVent;
                        }
                    }
                }
                // Crystal power conduits
                for &(dx, dy) in &[(-2,-1),(2,-1),(-2,1),(2,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::MetalFloor | Tile::CreditCache) {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Safe approach paths (clear lava for cardinal approaches)
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::MetalFloor;
                    }
                    let lx = cx + dx / 2;
                    let ly = cy + dy / 2;
                    if self.in_bounds(lx, ly) {
                        let idx = self.idx(lx, ly);
                        if self.tiles[idx] == Tile::PlasmaVent {
                            self.tiles[idx] = Tile::MetalFloor;
                        }
                    }
                }
                // Dragon gate portal at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::WarpGatePortal;
                }
            }
            SpecialRoomKind::GamblingDen => {
                // 3 urns (chests) in a row, player picks one
                for dx in -1..=1 {
                    let x = cx + dx * 2;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::SupplyCrate;
                        }
                    }
                }
                // Gold decoration around the den
                for &(dx, dy) in &[(-3, 0), (3, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CreditCache;
                        }
                    }
                }
            }
            SpecialRoomKind::BloodTerminal => {
                // Central altar with blood pools (lava-styled)
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::Terminal(AltarKind::Tactical);
                    }
                }
                for &(dx, dy) in &[(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Coolant;
                        }
                    }
                }
            }
            SpecialRoomKind::CursedSalvage => {
                // Ominous chest surrounded by cursed floor
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CorruptedFloor;
                            }
                        }
                    }
                }
            }
            SpecialRoomKind::SoulForge => {
                // Ethereal forge with crystal pillars
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::QuantumForge;
                    }
                }
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
                // Med bay fuel
                if self.in_bounds(cx, cy + 1) {
                    let idx = self.idx(cx, cy + 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::PlasmaVent;
                    }
                }
            }
            SpecialRoomKind::WishingReactor => {
                // Deep water well with water ring
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::VacuumBreach;
                    }
                }
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Gold offerings around well
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CreditCache;
                        }
                    }
                }
            }
            SpecialRoomKind::CipherGate => {
                // 4 pressure plates in cardinal directions (rune positions)
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PressureSensor;
                        }
                    }
                }
                // Locked treasure in center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SealedHatch;
                    }
                }
            }
            SpecialRoomKind::HoloMaze => {
                // Grid of crystals (mirrors) with walkable paths
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x + y) % 3 == 0 && !(x == cx && y == cy) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CrystalPanel;
                            }
                        }
                    }
                }
                // Reward in center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
            }
            SpecialRoomKind::GravityPlate => {
                // 3 pressure plates in a triangle, 3 boulders nearby, ice floor
                let plates = [(cx - 2, cy), (cx + 2, cy), (cx, cy - 2)];
                let boulders = [(cx - 2, cy + 1), (cx + 2, cy + 1), (cx, cy + 1)];
                for &(px, py) in &plates {
                    if self.in_bounds(px, py) {
                        let idx = self.idx(px, py);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PressureSensor;
                        }
                    }
                }
                for &(bx, by) in &boulders {
                    if self.in_bounds(bx, by) {
                        let idx = self.idx(bx, by);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CargoCrate;
                        }
                    }
                }
                // Ice floor for sliding
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::FrozenDeck;
                            }
                        }
                    }
                }
                // Locked chest reward
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::ToneFrequency => {
                // 4 ascending shrines in a line
                for i in 0..4 {
                    let x = cx - 1 + i;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::NavBeacon;
                        }
                    }
                }
                // Reward chest at the top
                if self.in_bounds(cx + 3, cy) {
                    let idx = self.idx(cx + 3, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
            }
            SpecialRoomKind::ElementalLock => {
                // Locked door with 5 elemental altars
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SealedHatch;
                    }
                }
                let altars = [
                    (cx - 2, cy - 1, AltarKind::Quantum),
                    (cx + 2, cy - 1, AltarKind::Stellar),
                    (cx - 2, cy + 1, AltarKind::Tactical),
                    (cx + 2, cy + 1, AltarKind::Commerce),
                    (cx, cy + 2, AltarKind::Holographic),
                ];
                for &(ax, ay, akind) in &altars {
                    if self.in_bounds(ax, ay) {
                        let idx = self.idx(ax, ay);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::Terminal(akind);
                        }
                    }
                }
                // Chest behind the locked door
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
            }
            SpecialRoomKind::SurvivalBay => {
                // Open arena with spikes border, central floor
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::LaserGrid;
                            }
                        }
                    }
                }
                for y in room.y + 1..room.y + room.h - 1 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::LaserGrid;
                            }
                        }
                    }
                }
            }
            SpecialRoomKind::SalvageRace => {
                // Open room, gold piles scattered (collected on step)
                for _ in 0..6 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CreditCache;
                        }
                    }
                }
            }
            SpecialRoomKind::DepressurizingChamber => {
                // Spikes on the outer ring, treasure chest in center
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::LaserGrid;
                            }
                        }
                    }
                }
                for y in room.y + 2..room.y + room.h - 2 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::LaserGrid;
                            }
                        }
                    }
                }
                // Treasure at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
            }
            SpecialRoomKind::NanoFlood => {
                // Ink (oil) flooding most of the room
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 3 != 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::Coolant;
                            }
                        }
                    }
                }
                // InkWell at center for bonus
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::DataWell;
                }
                // Chest as reward near edge
                let rx = room.x + room.w - 2;
                if self.in_bounds(rx, cy) {
                    let idx = self.idx(rx, cy);
                    self.tiles[idx] = Tile::SupplyCrate;
                }
            }
            SpecialRoomKind::FormShrine => {
                // Central shrine with 4 elemental markers
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::NavBeacon;
                    }
                }
                let markers = [
                    (cx - 2, cy, Tile::PlasmaVent),     // Flame
                    (cx + 2, cy, Tile::CargoCrate),   // Stone
                    (cx, cy - 2, Tile::CoolantPool),     // Mist
                    (cx, cy + 2, Tile::PressureSensor), // Tiger
                ];
                for &(mx, my, tile) in &markers {
                    if self.in_bounds(mx, my) {
                        let idx = self.idx(mx, my);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = tile;
                        }
                    }
                }
            }
            SpecialRoomKind::ClassTrial => {
                // Training dummies (pressure plates) with obstacle course
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PressureSensor;
                        }
                    }
                }
                // Reward altar at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::Terminal(AltarKind::Tactical);
                    }
                }
            }
            SpecialRoomKind::RadicalReactor => {
                // Med bay fountain with water and crystals
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::MedBayTile;
                    }
                }
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::CoolantPool;
                            }
                        }
                    }
                }
                for &(dx, dy) in &[(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CrystalPanel;
                        }
                    }
                }
            }
            SpecialRoomKind::AncestorCrypt => {
                // Tomb with bookshelf walls and central chest (weapon)
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::Terminal(AltarKind::Quantum);
                    }
                }
                for dx in -1..=1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        let x = cx + dx;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::DataRack;
                            }
                        }
                    }
                }
            }
            SpecialRoomKind::ProphecyRoom => {
                // Murals (bookshelves) on all walls, crystal ball in center
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::DataRack;
                            }
                        }
                    }
                }
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::HoloPool;
                    }
                }
            }
            SpecialRoomKind::SealedMemory => {
                // Meditation space with shrine and water
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::NavBeacon;
                    }
                }
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::CoolantPool;
                        }
                    }
                }
                // Bookshelves as memory records
                for dx in -2..=2 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy - 2) {
                        let idx = self.idx(x, cy - 2);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataRack;
                        }
                    }
                }
            }
            SpecialRoomKind::DemonSeal => {
                // Sealed demon NPC with lava barrier
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::Npc(3);
                    }
                }
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::MetalFloor {
                                self.tiles[idx] = Tile::PlasmaVent;
                            }
                        }
                    }
                }
                // Safe approach from south
                if self.in_bounds(cx, cy + 1) {
                    let idx = self.idx(cx, cy + 1);
                    self.tiles[idx] = Tile::MetalFloor;
                }
                if self.in_bounds(cx, cy + 2) {
                    let idx = self.idx(cx, cy + 2);
                    self.tiles[idx] = Tile::MetalFloor;
                }
            }
            SpecialRoomKind::PhoenixNest => {
                // Central med bay with lava nest ring
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::MedBayTile;
                    }
                }
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::PlasmaVent;
                        }
                    }
                }
                // Chest with phoenix plume
                if self.in_bounds(cx + 2, cy) {
                    let idx = self.idx(cx + 2, cy);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::SupplyCrate;
                    }
                }
                // Safe paths
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] != Tile::SupplyCrate {
                            self.tiles[idx] = Tile::MetalFloor;
                        }
                    }
                }
            }
            SpecialRoomKind::CalligraphyContest => {
                // Writing stations with ink wells
                for dx in [-2, 0, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::MetalFloor {
                            self.tiles[idx] = Tile::DataWell;
                        }
                    }
                }
                // NPC judge
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    if self.tiles[idx] == Tile::MetalFloor {
                        self.tiles[idx] = Tile::Npc(0);
                    }
                }
            }
        }
    }
}
