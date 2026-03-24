//! Feature placement: room modifiers, NPCs, shrines, altars, seals.

use super::super::*;

impl LocationLevel {
    /// Assign random modifiers to some modules (not first or last).
    pub(super) fn assign_room_modifiers(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 2 {
            return;
        }
        for i in 1..n - 1 {
            if rng.next_u64() % 100 < 30 {
                self.rooms[i].modifier = Some(match rng.next_u64() % 6 {
                    0 => RoomModifier::PoweredDown,
                    1 => RoomModifier::HighTech,
                    2 => RoomModifier::Irradiated,
                    3 => RoomModifier::Hydroponics,
                    4 => RoomModifier::Cryogenic,
                    _ => RoomModifier::OverheatedReactor,
                });
            }
        }
    }

    /// Place a companion NPC in a random middle module (~40% chance per deck).
    pub(super) fn place_npcs(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 40 {
            return;
        }
        // Pick a random middle room (not first, last, or second-to-last)
        let room_idx = 1 + (rng.next_u64() as usize % (n - 3));
        let room = &self.rooms[room_idx];
        let npc_type = (rng.next_u64() % 4) as u8;
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2 + 1; // offset from center
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::Npc(npc_type);
        }
    }

    /// Place a tone shrine in a random middle room (~30% chance).
    pub(super) fn place_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 30 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 1;
        let cy = room.y + room.h / 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::NavBeacon;
        }
    }

    pub(super) fn place_stroke_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 20 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 + 1;
        let cy = room.y + room.h / 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::CircuitShrine;
        }
    }

    pub(super) fn place_tone_walls(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 20 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2 + 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::FrequencyWall;
        }
    }

    pub(super) fn place_compound_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 20 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 1;
        let cy = room.y + room.h / 2 + 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::CompoundShrine;
        }
    }

    pub(super) fn place_classifier_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 20 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 + 1;
        let cy = room.y + room.h / 2 + 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::ClassifierNode;
        }
    }

    pub(super) fn place_ink_wells(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 1;
        let cy = room.y + room.h / 2 - 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::DataWell;
        }
    }

    pub(super) fn place_ancestor_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 + 1;
        let cy = room.y + room.h / 2 - 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::MemorialNode;
        }
    }

    pub(super) fn place_translation_altars(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2 - 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::TranslationTerminal;
        }
    }

    pub(super) fn place_radical_gardens(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 2;
        let cy = room.y + room.h / 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::RadicalLab;
        }
    }

    pub(super) fn place_mirror_pools(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 15 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 + 2;
        let cy = room.y + room.h / 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::HoloPool;
        }
    }

    pub(super) fn place_stone_tutors(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2 + 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::DroidTutor;
        }
    }

    pub(super) fn place_codex_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 15 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 1;
        let cy = room.y + room.h / 2 + 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::MetalFloor {
            self.tiles[idx] = Tile::CodexTerminal;
        }
    }

    pub(super) fn place_word_bridges(&mut self, rng: &mut Rng) {
        for i in 0..self.tiles.len() {
            if self.tiles[i] == Tile::VacuumBreach {
                let x = (i % self.width as usize) as i32;
                let y = (i / self.width as usize) as i32;
                for &(dx, dy) in &[(0, -1), (0, 1), (-1, 0), (1, 0)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if self.in_bounds(nx, ny) {
                        let ni = self.idx(nx, ny);
                        if self.tiles[ni] == Tile::MetalFloor && rng.next_u64() % 100 < 8 {
                            self.tiles[ni] = Tile::DataBridge;
                            break;
                        }
                    }
                }
            }
        }
    }

    pub(super) fn place_locked_doors(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 4 {
            return;
        }
        if rng.next_u64() % 100 >= 12 {
            return;
        }
        let room_idx = 2 + (rng.next_u64() as usize % (n - 3));
        let room = &self.rooms[room_idx];
        let doorways = [
            (room.x + room.w / 2, room.y),
            (room.x + room.w / 2, room.y + room.h - 1),
            (room.x, room.y + room.h / 2),
            (room.x + room.w - 1, room.y + room.h / 2),
        ];
        for (dx, dy) in doorways {
            if self.in_bounds(dx, dy) {
                let di = self.idx(dx, dy);
                if self.tiles[di] == Tile::MetalFloor || self.tiles[di] == Tile::Hallway {
                    self.tiles[di] = Tile::SealedHatch;
                    return;
                }
            }
        }
    }

    pub(super) fn place_cursed_floors(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        let count = 1 + (rng.next_u64() as usize % 3);
        for _ in 0..count {
            let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
            let room = &self.rooms[room_idx];
            let rx = room.x + 1 + (rng.next_u64() as i32 % (room.w - 2).max(1));
            let ry = room.y + 1 + (rng.next_u64() as i32 % (room.h - 2).max(1));
            if self.in_bounds(rx, ry) {
                let ri = self.idx(rx, ry);
                if self.tiles[ri] == Tile::MetalFloor {
                    self.tiles[ri] = Tile::CorruptedFloor;
                }
            }
        }
    }

    /// Place a terminal in a quiet side module (~35% chance).
    pub(super) fn place_altars(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 35 {
            return;
        }
        for _ in 0..12 {
            let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
            let room = &self.rooms[room_idx];
            let has_special = (room.y..room.y + room.h).any(|ry| {
                (room.x..room.x + room.w).any(|rx| {
                    if rx >= 0 && ry >= 0 && rx < self.width && ry < self.height {
                        let idx = (ry * self.width + rx) as usize;
                        matches!(
                            self.tiles[idx],
                            Tile::QuantumForge
                                | Tile::TradeTerminal
                                | Tile::Airlock
                                | Tile::SupplyCrate
                                | Tile::Npc(_)
                                | Tile::NavBeacon
                                | Tile::Terminal(_)
                                | Tile::SecurityLock(_)
                                | Tile::InfoPanel(_)
                        )
                    } else {
                        false
                    }
                })
            });
            if has_special {
                continue;
            }

            let ax = room.x + room.w / 2;
            let ay = room.y + room.h / 2;
            if !self.in_bounds(ax, ay) {
                continue;
            }
            let idx = self.idx(ax, ay);
            if self.tiles[idx] == Tile::MetalFloor {
                self.tiles[idx] = Tile::Terminal(AltarKind::random(rng));
                return;
            }
        }
    }

    /// Place 1-2 security locks that reshape modules when stepped on.
    pub(super) fn place_seals(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }

        let seal_count = 1 + (rng.next_u64() % 2) as usize;
        let mut used_rooms = Vec::new();
        let mut placed = 0;
        for _ in 0..20 {
            if placed >= seal_count {
                break;
            }

            let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
            if used_rooms.contains(&room_idx) {
                continue;
            }
            let room = &self.rooms[room_idx];
            let has_special = (room.y..room.y + room.h).any(|ry| {
                (room.x..room.x + room.w).any(|rx| {
                    if rx >= 0 && ry >= 0 && rx < self.width && ry < self.height {
                        let idx = (ry * self.width + rx) as usize;
                        matches!(
                            self.tiles[idx],
                            Tile::QuantumForge
                                | Tile::TradeTerminal
                                | Tile::Airlock
                                | Tile::SupplyCrate
                                | Tile::Npc(_)
                                | Tile::NavBeacon
                                | Tile::Terminal(_)
                                | Tile::SecurityLock(_)
                                | Tile::InfoPanel(_)
                        )
                    } else {
                        false
                    }
                })
            });
            if has_special {
                continue;
            }

            let sx = room.x + room.w / 2;
            let sy = room.y + room.h / 2;
            if !self.in_bounds(sx, sy) {
                continue;
            }
            let idx = self.idx(sx, sy);
            if self.tiles[idx] != Tile::MetalFloor {
                continue;
            }

            self.tiles[idx] = Tile::SecurityLock(SealKind::random(rng));
            used_rooms.push(room_idx);
            placed += 1;
        }
    }
}
