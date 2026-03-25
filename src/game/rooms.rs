//! Room interaction, seal mechanics, and tile effects.

use super::*;

impl GameState {
    pub(super) fn find_free_adjacent_tile(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let dirs = [
            (0, -1),
            (1, 0),
            (0, 1),
            (-1, 0),
            (1, -1),
            (1, 1),
            (-1, 1),
            (-1, -1),
        ];
        dirs.iter().copied().find(|(dx, dy)| {
            let nx = x + dx;
            let ny = y + dy;
            self.level.is_walkable(nx, ny)
                && self.enemy_at(nx, ny).is_none()
                && (nx != self.player.x || ny != self.player.y)
        })
    }

    pub(super) fn paint_seal_cross(&mut self, x: i32, y: i32, tile: Tile) -> usize {
        let mut changed = 0;
        for (tx, ty) in seal_cross_positions(x, y) {
            if !self.level.in_bounds(tx, ty) {
                continue;
            }
            let idx = self.level.idx(tx, ty);
            if can_be_reshaped_by_seal(self.level.tiles[idx]) {
                self.level.tiles[idx] = tile;
                changed += 1;
            }
        }
        changed
    }

    pub(super) fn stun_enemies_on_tiles(&mut self, targets: &[(i32, i32)]) -> usize {
        let mut stunned = 0;
        for idx in 0..self.enemies.len() {
            if !self.enemies[idx].is_alive() {
                continue;
            }
            if targets
                .iter()
                .any(|(tx, ty)| self.enemies[idx].x == *tx && self.enemies[idx].y == *ty)
            {
                self.enemies[idx].stunned = true;
                let (sx, sy) = self.tile_to_screen(self.enemies[idx].x, self.enemies[idx].y);
                self.particles.spawn_stun(sx, sy, &mut self.rng_state);
                stunned += 1;
            }
        }
        stunned
    }

    pub(super) fn damage_enemies_on_tiles(&mut self, targets: &[(i32, i32)]) -> usize {
        let mut pricked = 0;
        for idx in 0..self.enemies.len() {
            if !self.enemies[idx].is_alive() {
                continue;
            }
            if targets
                .iter()
                .any(|(tx, ty)| self.enemies[idx].x == *tx && self.enemies[idx].y == *ty)
            {
                let hp_before = self.enemies[idx].hp;
                self.apply_enemy_tile_effect(idx);
                if self.enemies[idx].hp < hp_before {
                    pricked += 1;
                }
            }
        }
        pricked
    }

    pub(super) fn spawn_seal_ambusher(&mut self, x: i32, y: i32) -> Option<&'static str> {
        let (sx, sy) = self
            .find_free_adjacent_tile(x, y)
            .or_else(|| self.find_free_adjacent_tile(self.player.x, self.player.y))?;
        let pool = vocab::vocab_for_floor(self.floor_num.max(1));
        if pool.is_empty() {
            return None;
        }
        let rand_val = self.rng_next();
        let entry = pool[self.srs.weighted_pick(&pool, rand_val)];
        let mut enemy = Enemy::from_vocab(entry, sx, sy, self.floor_num.max(1));
        enemy.alert = true;
        let hanzi = enemy.hanzi;
        self.enemies.push(enemy);
        Some(hanzi)
    }

    pub(super) fn trigger_seal(&mut self, x: i32, y: i32, kind: SealKind, triggerer: Option<&'static str>) {
        if !self.level.in_bounds(x, y) {
            return;
        }
        let idx = self.level.idx(x, y);
        if !matches!(self.level.tiles[idx], Tile::SecurityLock(current) if current == kind) {
            return;
        }
        self.level.tiles[idx] = Tile::MetalFloor;

        let visible = self.level.visible[idx] || triggerer.is_none();
        let (sx, sy) = self.tile_to_screen(x, y);
        let affected_tiles = seal_cross_positions(x, y);
        match kind {
            SecuritySeal::Thermal => {
                let changed = self.paint_seal_cross(x, y, Tile::Coolant);
                if visible {
                    self.particles.spawn_fire(sx, sy, &mut self.rng_state);
                    self.flash = Some((255, 128, 80, 0.16));
                    self.message = match triggerer {
                        Some(name) => format!(
                            "🔥 {} triggers an {} — oil spills across {} tiles!",
                            name,
                            kind.label(),
                            changed
                        ),
                        None => format!(
                            "🔥 {} bursts open — oil spills across {} nearby tiles!",
                            kind.label(),
                            changed
                        ),
                    };
                    self.message_timer = 90;
                }
            }
            SecuritySeal::Hydraulic => {
                let changed = self.paint_seal_cross(x, y, Tile::CoolantPool);
                let stunned = self.stun_enemies_on_tiles(&affected_tiles);
                if visible {
                    self.particles.spawn_shield(sx, sy, &mut self.rng_state);
                    self.flash = Some((110, 180, 255, 0.14));
                    self.message = match triggerer {
                        Some(name) => format!(
                            "≈ {} releases a {} — {} tiles flood and {} foes stagger!",
                            name,
                            kind.label(),
                            changed,
                            stunned
                        ),
                        None => format!(
                            "≈ {} floods the room — {} tiles turn to water and {} foes stagger!",
                            kind.label(),
                            changed,
                            stunned
                        ),
                    };
                    self.message_timer = 90;
                }
            }
            SecuritySeal::Kinetic => {
                let changed = self.paint_seal_cross(x, y, Tile::LaserGrid);
                let pricked = self.damage_enemies_on_tiles(&affected_tiles);
                if visible {
                    self.particles.spawn_damage(sx, sy, &mut self.rng_state);
                    self.flash = Some((255, 100, 140, 0.14));
                    self.message = match triggerer {
                        Some(name) => format!(
                            "🗡 {} snaps a {} — {} spike tiles rise and prick {} foes!",
                            name,
                            kind.label(),
                            changed,
                            pricked
                        ),
                        None => format!(
                            "🗡 {} flares — {} spike tiles rise and prick {} foes!",
                            kind.label(),
                            changed,
                            pricked
                        ),
                    };
                    self.message_timer = 90;
                }
            }
            SecuritySeal::Sonic => {
                let ambusher = self.spawn_seal_ambusher(x, y);
                if visible {
                    self.particles.spawn_drain(sx, sy, &mut self.rng_state);
                    self.flash = Some((190, 100, 255, 0.16));
                    self.message = match (triggerer, ambusher) {
                        (Some(name), Some(enemy)) => format!(
                            "📣 {} cracks an {} — {} answers the call!",
                            name,
                            kind.label(),
                            enemy
                        ),
                        (None, Some(enemy)) => format!(
                            "📣 {} echoes through the hall — {} answers the call!",
                            kind.label(),
                            enemy
                        ),
                        (Some(name), None) => {
                            format!(
                                "📣 {} stirs an {}, but nothing answers.",
                                name,
                                kind.label()
                            )
                        }
                        (None, None) => {
                            format!("📣 {} hums softly, but nothing answers.", kind.label())
                        }
                    };
                    self.message_timer = 90;
                }
            }
        }
    }

    pub(super) fn apply_player_tile_effect(&mut self, tile: Tile) {
        match tile {
            Tile::LaserGrid => {
                let dmg = (1 + self.floor_num.max(1) / 3).max(1);
                self.player.hp -= dmg;
                if let Some(ref audio) = self.audio {
                    audio.play_damage();
                }
                let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
                self.particles.spawn_damage(sx, sy, &mut self.rng_state);
                self.trigger_shake(6);
                self.flash = Some((255, 60, 60, 0.2));
                self.message = format!("🪤 Spikes jab you for {} damage!", dmg);
                self.message_timer = 70;
                if self.player.hp <= 0 && !self.try_phoenix_revive() {
                    self.player.hp = 0;
                    self.run_journal
                        .log(RunEvent::DiedTo("Spike trap".to_string(), self.floor_num));
                    self.post_mortem_page = 0;
                    self.combat = CombatState::GameOver;
                    self.message = self.run_summary();
                    self.message_timer = 255;
                    if let Some(ref audio) = self.audio {
                        audio.play_death();
                    }
                    self.save_high_score();
                }
            }
            Tile::Coolant => {
                self.message = "🛢 Oil slick — fire magic will ignite nearby puddles.".to_string();
                self.message_timer = 60;
            }
            Tile::CoolantPool => {
                self.message = "≈ Shallow water — stunning magic can arc through it.".to_string();
                self.message_timer = 60;
                if let Some(ref audio) = self.audio {
                    audio.play_water_splash();
                }
            }
            Tile::Trap(trap_type) => {
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor; // trap consumed
                match trap_type {
                    0 => {
                        // Poison trap
                        self.player
                            .statuses
                            .push(crate::status::StatusInstance::new(
                                crate::status::StatusKind::Poison { damage: 1 },
                                5,
                            ));
                        self.message = "💀 Poison trap! Toxic fumes engulf you!".to_string();
                        self.trigger_shake(6);
                        self.flash = Some((120, 255, 80, 0.2));
                    }
                    1 => {
                        // Teleport trap
                        let mut found = false;
                        for _ in 0..100 {
                            let rx = (self.rng_next() % MAP_W as u64) as i32;
                            let ry = (self.rng_next() % MAP_H as u64) as i32;
                            let ri = self.level.idx(rx, ry);
                            if matches!(self.level.tiles[ri], Tile::MetalFloor | Tile::Hallway)
                                && self.level.revealed[ri]
                            {
                                self.player.x = rx;
                                self.player.y = ry;
                                found = true;
                                break;
                            }
                        }
                        self.message = if found {
                            "✦ Teleport trap! The floor gives way beneath you!".to_string()
                        } else {
                            "✦ A teleport trap fizzles...".to_string()
                        };
                        self.trigger_shake(4);
                        self.flash = Some((100, 140, 255, 0.25));
                    }
                    2 => {
                        // Alarm trap — alert all enemies
                        for e in &mut self.enemies {
                            if e.is_alive() {
                                e.alert = true;
                            }
                        }
                        self.message = "🔔 Alarm trap! All monsters are alerted!".to_string();
                        self.trigger_shake(4);
                        self.flash = Some((255, 200, 50, 0.2));
                    }
                    _ => {
                        // Rooted trap — anchored in place
                        self.player
                            .statuses
                            .push(crate::status::StatusInstance::new(
                                crate::status::StatusKind::Rooted,
                                5,
                            ));
                        self.message =
                            "⚓ Gravity snare! You're anchored for 5 turns!".to_string();
                        self.trigger_shake(5);
                        self.flash = Some((120, 130, 170, 0.2));
                    }
                }
                self.message_timer = 60;
                if let Some(ref audio) = self.audio {
                    audio.play_damage();
                }
            }
            Tile::PlasmaVent => {
                let dmg = (2 + self.floor_num.max(1) / 2).max(2);
                self.player.hp -= dmg;
                if let Some(ref audio) = self.audio {
                    audio.play_damage();
                    audio.play_lava_rumble();
                }
                let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
                self.particles.spawn_damage(sx, sy, &mut self.rng_state);
                self.trigger_shake(8);
                self.flash = Some((255, 80, 0, 0.3));
                self.message = format!("🔥 Lava burns you for {} damage!", dmg);
                self.message_timer = 70;
                if self.player.hp <= 0 && !self.try_phoenix_revive() {
                    self.player.hp = 0;
                    self.run_journal
                        .log(RunEvent::DiedTo("Lava".to_string(), self.floor_num));
                    self.post_mortem_page = 0;
                    self.combat = CombatState::GameOver;
                    self.message = self.run_summary();
                    self.message_timer = 255;
                    if let Some(ref audio) = self.audio {
                        audio.play_death();
                    }
                    self.save_high_score();
                }
            }
            Tile::FrozenDeck => {
                self.message = "❄ Ice! The floor is slippery.".to_string();
                self.message_timer = 40;
            }
            Tile::ToxicFungus => {
                self.message = "🍄 Spore cloud! You feel disoriented.".to_string();
                self.message_timer = 60;
                self.flash = Some((180, 100, 255, 0.15));
            }
            Tile::ToxicGas => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Poison { damage: 1 },
                        3,
                    ));
                self.message = "☠ Poison gas! Toxic fumes seep into your lungs!".to_string();
                self.message_timer = 60;
                self.trigger_shake(4);
                self.flash = Some((100, 220, 60, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_damage();
                }
            }
            Tile::CreditCache => {
                let gold = 5 + (self.rng_next() % 16) as i32;
                self.player.gold += gold;
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                self.message = format!("💰 You pick up {} gold!", gold);
                self.message_timer = 50;
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }
            Tile::MedBayTile => {
                let heal = self.player.max_hp / 2;
                self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                self.player.statuses.push(crate::status::StatusInstance::new(
                    crate::status::StatusKind::Regen { heal: 3 },
                    10,
                ));
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::CoolantPool;
                self.message = format!("🌊 The med bay heals {} HP and grants auto-repair!", heal);
                self.message_timer = 80;
                self.flash = Some((100, 255, 200, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_heal();
                }
            }
            _ => {}
        }
    }

    pub(super) fn apply_enemy_tile_effect(&mut self, enemy_idx: usize) {
        if enemy_idx >= self.enemies.len() || !self.enemies[enemy_idx].is_alive() {
            return;
        }
        let tile = self
            .level
            .tile(self.enemies[enemy_idx].x, self.enemies[enemy_idx].y);
        match tile {
            Tile::LaserGrid => {
                self.enemies[enemy_idx].hp -= 1;
                if self.enemies[enemy_idx].hp <= 0 {
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    let idx = self
                        .level
                        .idx(self.enemies[enemy_idx].x, self.enemies[enemy_idx].y);
                    if self.level.visible[idx] {
                        self.message = format!("🪤 {} stumbles into spikes and falls!", e_hanzi);
                        self.message_timer = 60;
                    }
                }
            }
            Tile::SecurityLock(kind) => {
                let (x, y, name) = (
                    self.enemies[enemy_idx].x,
                    self.enemies[enemy_idx].y,
                    self.enemies[enemy_idx].hanzi,
                );
                self.trigger_seal(x, y, kind, Some(name));
            }
            _ => {}
        }
    }

    pub(super) fn ignite_visible_oil(&mut self, bonus_dmg: i32) -> (usize, usize, usize) {
        let mut oil_tiles = Vec::new();
        let mut oil_screens = Vec::new();
        for y in 0..self.level.height {
            for x in 0..self.level.width {
                let idx = self.level.idx(x, y);
                if self.level.visible[idx] && self.level.tiles[idx] == Tile::Coolant {
                    oil_tiles.push((x, y));
                    oil_screens.push(self.tile_to_screen(x, y));
                }
            }
        }
        for &(x, y) in &oil_tiles {
            let idx = self.level.idx(x, y);
            self.level.tiles[idx] = Tile::MetalFloor;
        }
        for (sx, sy) in oil_screens {
            self.particles.spawn_fire(sx, sy, &mut self.rng_state);
        }

        let mut scorched = 0;
        let mut kills = 0;
        for enemy in &mut self.enemies {
            if !enemy.is_alive() {
                continue;
            }
            let hit = oil_tiles
                .iter()
                .any(|&(ox, oy)| (enemy.x - ox).abs() <= 1 && (enemy.y - oy).abs() <= 1);
            if hit {
                enemy.hp -= bonus_dmg;
                scorched += 1;
                if enemy.hp <= 0 {
                    kills += 1;
                }
            }
        }

        (oil_tiles.len(), scorched, kills)
    }

    pub(super) fn electrify_visible_water(&mut self, bonus_dmg: i32) -> (usize, usize, usize) {
        let mut water_tiles = Vec::new();
        let mut water_screens = Vec::new();
        for y in 0..self.level.height {
            for x in 0..self.level.width {
                let idx = self.level.idx(x, y);
                if self.level.visible[idx]
                    && matches!(self.level.tiles[idx], Tile::CoolantPool | Tile::VacuumBreach)
                {
                    water_tiles.push((x, y));
                    water_screens.push(self.tile_to_screen(x, y));
                }
            }
        }
        for (sx, sy) in water_screens {
            self.particles.spawn_stun(sx, sy, &mut self.rng_state);
        }

        let mut shocked = 0;
        let mut kills = 0;
        for enemy in &mut self.enemies {
            if !enemy.is_alive() {
                continue;
            }
            let standing_in_water = water_tiles
                .iter()
                .any(|&(wx, wy)| enemy.x == wx && enemy.y == wy);
            if standing_in_water {
                enemy.stunned = true;
                if bonus_dmg > 0 {
                    enemy.hp -= bonus_dmg;
                    if enemy.hp <= 0 {
                        kills += 1;
                    }
                }
                shocked += 1;
            }
        }

        (water_tiles.len(), shocked, kills)
    }

    pub(super) fn new_floor(&mut self) {
        if let Some(ref audio) = self.audio {
            audio.play_descend();
        }
        crate::srs::save_srs(&self.srs);
        self.codex.save();
        self.floor_num += 1;
        self.run_journal.log(RunEvent::EnteredFloor(self.floor_num));
        self.srs.current_deck = self.floor_num;
        self.tutorial = None;
        self.merchant_reroll_used = false;
        self.shop_banned = false;
        if self.floor_num > self.best_floor {
            self.best_floor = self.floor_num;
        }
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.rng_state = self.seed;
        self.floor_profile = FloorProfile::roll(self.floor_num, self.rng_next());
        self.level = DungeonLevel::generate(MAP_W, MAP_H, self.seed, self.floor_num, self.current_location_type.unwrap_or(crate::world::LocationType::OrbitalPlatform));
        let (sx, sy) = self.level.start_pos();
        self.player.move_to(sx, sy);
        self.enemies.clear();
        self.combat = CombatState::Explore;
        self.typing.clear();
        self.spawn_enemies();
        let (px, py) = (self.player.x, self.player.y);
        compute_fov(&mut self.level, px, py, FOV_RADIUS);
        self.achievements.check_floor(self.floor_num);
        self.save_game();

        if self.companion == Some(Companion::Medic) {
            let lvl = self.companion_level();
            let mut heal = Companion::Medic.heal_per_floor(lvl);
            // Synergy level 2 bonus: extra heal
            if self.companion_synergy_level() >= 2 {
                heal += Companion::Medic.synergy_heal_bonus();
            }
            let max_hp = self.player.max_hp;
            if self.player.hp < max_hp && heal > 0 {
                self.player.hp = (self.player.hp + heal).min(max_hp);
                self.message = format!("🧘 Monk heals you for {} HP.", heal);
                self.message_timer = 60;
                if lvl >= 3 {
                    if let Some(idx) = self.player.statuses.iter().position(|s| s.is_negative()) {
                        let removed = self.player.statuses.remove(idx);
                        self.message
                            .push_str(&format!(" Cured {}.", removed.label()));
                    }
                }
            }
        }

        // Apply equipment set bonuses (e.g. HealOnFloor)
        self.apply_set_bonuses_on_floor();

        // Advance companion bond on each floor transition
        self.advance_companion_bond();

        if self.floor_num > 1 {
            if self.player.get_piety(Faction::Consortium) >= 10 && self.player.get_piety(Faction::AncientOrder) >= 10
            {
                self.player.gold += 5;
            }
        }

        if self.player.get_piety(Faction::Technocracy) >= 10 && self.player.get_piety(Faction::FreeTraders) >= 10 {
            if (self.rng_next() % 100) < 25 {
                self.reveal_entire_floor();
                let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
                self.particles.spawn_synergy(sx, sy, &mut self.rng_state);
                if self.message.is_empty() {
                    self.message = "Scholar's Wind reveals the floor layout!".to_string();
                    self.message_timer = 90;
                }
            }
        }

        if self.player.get_piety(Faction::FreeTraders) >= 10 && self.player.get_piety(Faction::AncientOrder) >= 10 {
            if (self.rng_next() % 100) < 25 {
                let mut tries = 0;
                while tries < 100 {
                    let rx = (self.rng_next() % (MAP_W as u64)) as i32;
                    let ry = (self.rng_next() % (MAP_H as u64)) as i32;
                    if self.level.in_bounds(rx, ry)
                        && self.level.is_walkable(rx, ry)
                        && self.level.tile(rx, ry) == Tile::MetalFloor
                    {
                        if (rx, ry) != (self.player.x, self.player.y) {
                            let idx = self.level.idx(rx, ry);
                            self.level.tiles[idx] = Tile::SupplyCrate;
                            break;
                        }
                    }
                    tries += 1;
                }
            }
        }

        let profile_label = self.floor_profile.label();
        if !profile_label.is_empty() && self.message.is_empty() {
            self.message = profile_label.to_string();
            self.message_timer = 90;
        }

        // Generate new quests for this floor and check floor-based quests
        self.generate_quests();
        self.check_floor_quests();
    }

    /// Check if an enemy occupies (x, y). Returns its index.
    pub(super) fn enemy_at(&self, x: i32, y: i32) -> Option<usize> {
        self.enemies
            .iter()
            .position(|e| e.is_alive() && e.x == x && e.y == y)
    }

    /// Get the room modifier at the player's current position.
    pub(super) fn current_room_modifier(&self) -> Option<RoomModifier> {
        let px = self.player.x;
        let py = self.player.y;
        for room in &self.level.rooms {
            if px >= room.x && px < room.x + room.w && py >= room.y && py < room.y + room.h {
                return room.modifier;
            }
        }
        None
    }

    /// Get the special room kind at the player's current position.
    pub(super) fn current_special_room(&self) -> Option<SpecialRoomKind> {
        let px = self.player.x;
        let py = self.player.y;
        for room in &self.level.rooms {
            if px >= room.x && px < room.x + room.w && py >= room.y && py < room.y + room.h {
                return room.special;
            }
        }
        None
    }

    /// Get the (room.x, room.y) of the room the player is currently in, if any.
    pub(super) fn current_room_origin(&self) -> Option<(i32, i32)> {
        let px = self.player.x;
        let py = self.player.y;
        for room in &self.level.rooms {
            if px >= room.x && px < room.x + room.w && py >= room.y && py < room.y + room.h {
                return Some((room.x, room.y));
            }
        }
        None
    }

    /// Mark the current special room as completed so it won't trigger again.
    pub(super) fn mark_room_completed(&mut self) {
        if let Some((rx, ry)) = self.current_room_origin() {
            self.completed_special_rooms.insert((self.floor_num, rx, ry));
        }
    }

    /// Check if the current special room has already been completed.
    pub(super) fn is_room_completed(&self) -> bool {
        if let Some((rx, ry)) = self.current_room_origin() {
            self.completed_special_rooms.contains(&(self.floor_num, rx, ry))
        } else {
            false
        }
    }

    /// Handle interactive mechanics for the 23 new special room types.
    pub(super) fn handle_special_room_interaction(&mut self, target_tile: Tile) {
        let special = match self.current_special_room() {
            Some(s) => s,
            None => return,
        };

        // Skip rooms already completed
        if self.is_room_completed() {
            return;
        }

        match special {
            // ── Risk/Reward Rooms ────────────────────────────────────
            SpecialRoomKind::WanderingMerchant => {
                // Triggered when stepping on a Chest tile
                if target_tile != Tile::SupplyCrate { return; }
                self.mark_room_completed();
                let roll = self.rng_next() % 3;
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                match roll {
                    0 => {
                        // Great reward: rare radical + gold
                        let rare_radicals: &[&str] = &["龙", "凤", "鬼", "神", "魂", "仙"];
                        let r = rare_radicals[self.rng_next() as usize % rare_radicals.len()];
                        self.player.add_radical(r);
                        self.player.gold += 50;
                        self.message = format!("🎰 JACKPOT! You find the rare radical {} and 50 gold!", r);
                        self.flash = Some((255, 215, 0, 0.3));
                        if let Some(ref audio) = self.audio {
                            audio.play_treasure();
                        }
                    }
                    1 => {
                        // Decent: random item
                        let _ = self.player.add_item(Item::MedHypo(8), ItemState::Normal);
                        self.message = "🎰 You find a Health Potion inside the urn.".to_string();
                        if let Some(ref audio) = self.audio {
                            audio.play_treasure();
                        }
                    }
                    _ => {
                        // Trap: poison + lose gold
                        self.player.statuses.push(crate::status::StatusInstance::new(
                            crate::status::StatusKind::Poison { damage: 2 },
                            5,
                        ));
                        let lost = 20.min(self.player.gold);
                        self.player.gold -= lost;
                        self.message = format!("🎰 TRAP! Poison gas and you lose {} gold!", lost);
                        self.trigger_shake(8);
                        self.flash = Some((120, 255, 80, 0.25));
                        if let Some(ref audio) = self.audio {
                            audio.play_damage();
                        }
                    }
                }
                self.message_timer = 100;
            }

            SpecialRoomKind::EnergyNexus => {
                if target_tile != Tile::Terminal(TerminalKind::Tactical) { return; }
                if self.player.hp <= 5 {
                    self.message = "🩸 You're too weak to sacrifice. You need more than 5 HP.".to_string();
                    self.message_timer = 80;
                    return;
                }
                self.mark_room_completed();
                self.player.hp -= 5;
                self.player.tone_bonus_damage += 1;
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                self.message = "🩸 You sacrifice 5 HP. Permanent +1 damage gained!".to_string();
                self.message_timer = 100;
                self.trigger_shake(6);
                self.flash = Some((180, 0, 0, 0.3));
                if let Some(ref audio) = self.audio {
                    audio.play_damage();
                }
            }

            SpecialRoomKind::CursedSalvage => {
                if target_tile != Tile::SupplyCrate { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Great loot: high-tier equipment + gold
                let equip_idx = 2 + (self.rng_next() as usize % 3); // Dragon Fang Pen or better
                let equip = &EQUIPMENT_POOL[equip_idx.min(EQUIPMENT_POOL.len() - 1)];
                let luck_bonus = self.player.skill_tree.total_item_rarity_bonus();
                let rarity = crate::rarity::roll_rarity(self.floor_num, luck_bonus, self.rng_next());
                let affixes = crate::rarity::roll_affixes(rarity, self.rng_next());
                let display = crate::rarity::rarity_name(equip.name, rarity, &affixes);
                self.player.equip_with_rarity(equip, ItemState::Normal, rarity, affixes);
                self.player.gold += 75;
                // Apply Cursed status for 10 turns (representing floors)
                self.player.statuses.push(crate::status::StatusInstance::new(
                    crate::status::StatusKind::Cursed,
                    50, // ~10 floors worth of turns
                ));
                self.message = format!("💀 You claim {} and 75 gold, but a curse clings to you!", display);
                self.message_timer = 120;
                self.trigger_shake(4);
                self.flash = Some((100, 0, 150, 0.3));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::SoulForge => {
                if target_tile != Tile::QuantumForge { return; }
                if self.player.radicals.is_empty() {
                    self.message = "🔮 The Soul Forge flickers — you have no radicals to offer.".to_string();
                    self.message_timer = 80;
                    return;
                }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Remove a random radical and give a different one
                let remove_idx = self.rng_next() as usize % self.player.radicals.len();
                let old = self.player.radicals.remove(remove_idx);
                let rare_radicals: &[&str] = &["龙", "凤", "鬼", "神", "魂", "仙", "雷", "冰", "光", "暗"];
                let new_rad = rare_radicals[self.rng_next() as usize % rare_radicals.len()];
                self.player.add_radical(new_rad);
                self.message = format!("🔮 The Soul Forge transforms {} into {}!", old, new_rad);
                self.message_timer = 100;
                self.flash = Some((200, 100, 255, 0.25));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::ChemLab => {
                // Triggered when stepping on DeepWater (the well center)
                if target_tile != Tile::VacuumBreach { return; }
                self.mark_room_completed();
                // Tiered cost: take what the player can afford
                let (cost, reward_tier) = if self.player.gold >= 50 {
                    (50, 2)
                } else if self.player.gold >= 25 {
                    (25, 1)
                } else if self.player.gold >= 10 {
                    (10, 0)
                } else {
                    self.message = "🪙 The well needs at least 10 gold...".to_string();
                    self.message_timer = 60;
                    return;
                };
                self.player.gold -= cost;
                match reward_tier {
                    0 => {
                        let _ = self.player.add_item(Item::MedHypo(10), ItemState::Normal);
                        self.message = format!("🪙 You throw {} gold. A potion rises from the depths!", cost);
                    }
                    1 => {
                        let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
                        let equip = &EQUIPMENT_POOL[eq_idx];
                        let luck_bonus = self.player.skill_tree.total_item_rarity_bonus();
                        let rarity = crate::rarity::roll_rarity(self.floor_num, luck_bonus, self.rng_next());
                        let affixes = crate::rarity::roll_affixes(rarity, self.rng_next());
                        let display = crate::rarity::rarity_name(equip.name, rarity, &affixes);
                        self.player.equip_with_rarity(equip, ItemState::Normal, rarity, affixes);
                        self.message = format!("🪙 You throw {} gold. {} rises from the depths!", cost, display);
                    }
                    _ => {
                        let rare_radicals: &[&str] = &["龙", "凤", "鬼", "神", "魂", "仙"];
                        let r = rare_radicals[self.rng_next() as usize % rare_radicals.len()];
                        self.player.add_radical(r);
                        let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
                        let equip = &EQUIPMENT_POOL[eq_idx];
                        let luck_bonus = self.player.skill_tree.total_item_rarity_bonus();
                        let rarity = crate::rarity::roll_rarity(self.floor_num, luck_bonus, self.rng_next());
                        let affixes = crate::rarity::roll_affixes(rarity, self.rng_next());
                        let display = crate::rarity::rarity_name(equip.name, rarity, &affixes);
                        self.player.equip_with_rarity(equip, ItemState::Blessed, rarity, affixes);
                        self.message = format!("🪙 You throw {} gold. Radical {} and blessed {} rise!", cost, r, display);
                    }
                }
                self.message_timer = 120;
                self.flash = Some((100, 180, 255, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            // ── Puzzle Rooms ─────────────────────────────────────────
            SpecialRoomKind::WarpGate => {
                // Stepping on pressure plates in the room
                if target_tile != Tile::PressureSensor { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Simplified: each plate gives a reward or penalty
                if self.rng_next() % 2 == 0 {
                    let radicals: &[&str] = &["水", "火", "金", "木", "土"];
                    let r = radicals[self.rng_next() as usize % radicals.len()];
                    self.player.add_radical(r);
                    self.message = format!("✨ Correct sequence! The rune grants you radical {}!", r);
                    self.flash = Some((100, 255, 200, 0.2));
                    if let Some(ref audio) = self.audio {
                        audio.play_treasure();
                    }
                } else {
                    let dmg = 2 + self.floor_num / 5;
                    self.player.hp -= dmg;
                    self.message = format!("⚡ Wrong order! The rune zaps you for {} damage!", dmg);
                    self.trigger_shake(6);
                    self.flash = Some((255, 100, 50, 0.25));
                    if let Some(ref audio) = self.audio {
                        audio.play_damage();
                    }
                }
                self.message_timer = 80;
            }

            SpecialRoomKind::HoloMaze => {
                // Reward at the center chest
                if target_tile != Tile::SupplyCrate { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                self.player.gold += 30 + (self.floor_num * 3) as i32;
                let _ = self.player.add_item(Item::ScannerPulse, ItemState::Normal);
                self.message = format!("🪞 You navigate the mirrors! {} gold + Reveal Scroll!", 30 + self.floor_num * 3);
                self.message_timer = 100;
                self.flash = Some((200, 200, 255, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::GravityPuzzle => {
                // Reward chest in center after navigating ice + boulders
                if target_tile != Tile::SupplyCrate { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                self.player.gold += 40;
                let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
                let equip = &EQUIPMENT_POOL[eq_idx];
                let luck_bonus = self.player.skill_tree.total_item_rarity_bonus();
                let rarity = crate::rarity::roll_rarity(self.floor_num, luck_bonus, self.rng_next());
                let affixes = crate::rarity::roll_affixes(rarity, self.rng_next());
                let display = crate::rarity::rarity_name(equip.name, rarity, &affixes);
                self.player.equip_with_rarity(equip, ItemState::Normal, rarity, affixes);
                self.message = format!("⚖ Puzzle solved! 40 gold + {}!", display);
                self.message_timer = 100;
                self.flash = Some((200, 255, 150, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::ToneFrequency => {
                // Stepping on Shrine tiles (the steps)
                if target_tile != Tile::CircuitShrine { return; }
                // Don't mark completed — each shrine is a step
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                self.player.hp = (self.player.hp + 5).min(self.player.max_hp);
                self.message = "🎵 Correct tone! +5 HP. Ascend the staircase!".to_string();
                self.message_timer = 60;
                self.flash = Some((255, 220, 100, 0.15));
            }

            SpecialRoomKind::ElementalLock => {
                // Stepping on elemental altars charges the lock
                if !matches!(target_tile, Tile::Terminal(_)) { return; }
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Count remaining altars in room
                let mut remaining = 0;
                if let Some((rx, ry)) = self.current_room_origin() {
                    for room in &self.level.rooms {
                        if room.x == rx && room.y == ry {
                            for ty in room.y..room.y + room.h {
                                for tx in room.x..room.x + room.w {
                                    if self.level.in_bounds(tx, ty) {
                                        let ti = self.level.idx(tx, ty);
                                        if matches!(self.level.tiles[ti], Tile::Terminal(_)) {
                                            remaining += 1;
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }
                }
                if remaining == 0 {
                    // All altars activated — unlock the door, give reward
                    self.mark_room_completed();
                    // Find and unlock the locked door
                    if let Some((rx, ry)) = self.current_room_origin() {
                        for room in &self.level.rooms {
                            if room.x == rx && room.y == ry {
                                for ty in room.y..room.y + room.h {
                                    for tx in room.x..room.x + room.w {
                                        if self.level.in_bounds(tx, ty) {
                                            let ti = self.level.idx(tx, ty);
                                            if self.level.tiles[ti] == Tile::SealedHatch {
                                                self.level.tiles[ti] = Tile::SupplyCrate;
                                            }
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }
                    self.message = "🔓 All elements channeled! The sealed door opens!".to_string();
                    self.flash = Some((255, 255, 200, 0.3));
                } else {
                    self.message = format!("🔮 Elemental energy absorbed! {} altars remaining.", remaining);
                }
                self.message_timer = 80;
            }

            // ── Timed/Wave Rooms ─────────────────────────────────────
            SpecialRoomKind::SurvivalBay => {
                // On first entry, give a big bonus (simulate surviving)
                if target_tile != Tile::LaserGrid && target_tile != Tile::MetalFloor { return; }
                // Only trigger once when entering the arena center area
                if self.player.x != self.level.rooms.iter()
                    .find(|r| r.special == Some(SpecialRoomKind::SurvivalBay))
                    .map(|r| r.x + r.w / 2)
                    .unwrap_or(-1)
                { return; }
                self.mark_room_completed();
                let reward_gold = 20 + self.player.hp * 2;
                self.player.gold += reward_gold;
                self.message = format!("⚔ You survived the pit! {} gold (HP bonus)!", reward_gold);
                self.message_timer = 100;
                self.flash = Some((255, 200, 50, 0.25));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::SalvageRace => {
                // Gold piles are already placed; tile effects handle pickup
                // Just show encouragement on entry
                if target_tile == Tile::CreditCache {
                    // Normal GoldPile effect handles this via apply_player_tile_effect
                    return;
                }
            }

            SpecialRoomKind::DepressurizingChamber => {
                // Chest in center is the goal
                if target_tile != Tile::SupplyCrate { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                self.player.gold += 60;
                let _ = self.player.add_item(Item::PersonalTeleporter, ItemState::Normal);
                self.message = "💎 You grab the treasure before the floor collapses! 60 gold + Teleport Scroll!".to_string();
                self.message_timer = 100;
                self.flash = Some((255, 200, 100, 0.25));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::NanoFlood => {
                if target_tile == Tile::DataWell {
                    self.mark_room_completed();
                    let idx = self.level.idx(self.player.x, self.player.y);
                    self.level.tiles[idx] = Tile::MetalFloor;
                    self.player.spell_power_bonus += 1;
                    self.player.hp -= 2;
                    self.message = "🖋 The ink empowers your spells! +1 spell power, but -2 HP from ink exposure.".to_string();
                    self.message_timer = 100;
                    self.flash = Some((50, 50, 100, 0.2));
                }
            }

            // ── Transformation/Permanent Rooms ───────────────────────
            SpecialRoomKind::FormShrine => {
                if target_tile != Tile::CircuitShrine { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Cycle through forms based on RNG
                let forms = [PlayerForm::Powered, PlayerForm::Cybernetic, PlayerForm::Holographic, PlayerForm::Void];
                let form = forms[self.rng_next() as usize % forms.len()];
                self.player.form = form;
                self.player.form_timer = 0; // permanent
                self.message = format!("🔥 The shrine transforms you into {} form permanently!", form.name());
                self.message_timer = 120;
                self.flash = Some((255, 150, 50, 0.3));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::ClassTrial => {
                if !matches!(target_tile, Tile::Terminal(_)) { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Grant class-specific bonus
                self.player.tone_bonus_damage += 1;
                self.player.defense_bonus += 1;
                self.message = "⚔ Trial complete! +1 damage and +1 defense permanently!".to_string();
                self.message_timer = 100;
                self.flash = Some((200, 180, 50, 0.25));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::DataArchive => {
                if target_tile != Tile::MedBayTile { return; }
                self.mark_room_completed();
                // The med bay tile effect (heal) is handled by apply_player_tile_effect
                // Additional: permanent spell power bonus
                self.player.spell_power_bonus += 1;
                self.message = "✨ The fountain refines your radicals! All spells gain +1 damage permanently!".to_string();
                self.message_timer = 120;
                self.flash = Some((150, 255, 200, 0.3));
            }

            SpecialRoomKind::AncestorCrypt => {
                if target_tile != Tile::SupplyCrate { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Give a high-tier weapon
                let high_tier = &EQUIPMENT_POOL[2]; // Dragon Fang Pen (+3 dmg)
                let luck_bonus = self.player.skill_tree.total_item_rarity_bonus();
                let rarity = crate::rarity::roll_rarity(self.floor_num, luck_bonus, self.rng_next());
                let affixes = crate::rarity::roll_affixes(rarity, self.rng_next());
                let display = crate::rarity::rarity_name(high_tier.name, rarity, &affixes);
                self.player.equip_with_rarity(high_tier, ItemState::Blessed, rarity, affixes);
                self.message = format!("⚔ The ancestor's spirit grants you their blessed {}!", display);
                self.message_timer = 120;
                self.flash = Some((255, 215, 0, 0.3));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            // ── Story/Lore Rooms ─────────────────────────────────────
            SpecialRoomKind::WisdomCore => {
                if target_tile != Tile::HoloPool { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Reveal the map + give info
                for i in 0..self.level.revealed.len() {
                    self.level.revealed[i] = true;
                }
                self.message = "🔮 The prophecy reveals the entire floor! You sense the boss's presence...".to_string();
                self.message_timer = 120;
                self.flash = Some((200, 150, 255, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::SealedMemory => {
                if target_tile != Tile::CircuitShrine { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Reward for recalling memories
                self.player.hp = self.player.max_hp;
                self.player.gold += 25;
                let radicals: &[&str] = &["心", "力", "气", "光"];
                let r = radicals[self.rng_next() as usize % radicals.len()];
                self.player.add_radical(r);
                self.message = format!("🧠 Memories flood back! HP restored, +25 gold, radical {}!", r);
                self.message_timer = 100;
                self.flash = Some((180, 200, 255, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::DemonSeal => {
                if target_tile != Tile::Npc(3) { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Accept the deal: +3 max HP but next floor enemies are tougher
                self.player.max_hp += 3;
                self.player.hp += 3;
                self.demon_deal_floors = 1;
                self.message = "👹 Deal accepted! +3 max HP, but next floor's enemies will be elite-tier!".to_string();
                self.message_timer = 120;
                self.trigger_shake(6);
                self.flash = Some((150, 0, 0, 0.3));
                if let Some(ref audio) = self.audio {
                    audio.play_damage();
                }
            }

            SpecialRoomKind::PhoenixNest => {
                if target_tile == Tile::MedBayTile {
                    // Full heal from stepping on spring (handled by tile effect)
                    // Also grant +2 max HP
                    if !self.is_room_completed() {
                        self.mark_room_completed();
                        self.player.max_hp += 2;
                        self.player.hp = self.player.max_hp;
                        self.message = "🔥 The Phoenix blesses you! Full heal + permanent +2 max HP!".to_string();
                        self.message_timer = 120;
                        self.flash = Some((255, 150, 50, 0.3));
                    }
                } else if target_tile == Tile::SupplyCrate {
                    // PhoenixPlume item
                    let idx = self.level.idx(self.player.x, self.player.y);
                    self.level.tiles[idx] = Tile::MetalFloor;
                    let _ = self.player.add_item(Item::Revitalizer(self.player.max_hp / 2), ItemState::Blessed);
                    self.message = "🔥 You find a blessed Phoenix Plume! Auto-revive on death!".to_string();
                    self.message_timer = 100;
                    if let Some(ref audio) = self.audio {
                        audio.play_treasure();
                    }
                }
            }

            SpecialRoomKind::CalligraphyContest => {
                if target_tile != Tile::DataWell { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // Score-based gold reward
                let score = 20 + (self.rng_next() % 40) as i32;
                self.player.gold += score;
                self.message = format!("🖌 Calligraphy contest! Score: {}. You earn {} gold!", score, score);
                self.message_timer = 100;
                self.flash = Some((255, 255, 200, 0.2));
                if let Some(ref audio) = self.audio {
                    audio.play_treasure();
                }
            }

            SpecialRoomKind::ChallengeTerminal => {
                if !matches!(target_tile, Tile::Terminal(_)) { return; }
                self.mark_room_completed();
                let idx = self.level.idx(self.player.x, self.player.y);
                self.level.tiles[idx] = Tile::MetalFloor;
                // 50/50 harder-than-normal hanzi challenge
                let success = self.rng_next() % 2 == 0;
                if success {
                    self.player.gold += 15;
                    self.player.hp = (self.player.hp + 5).min(self.player.max_hp);
                    let rare_radicals: &[&str] = &["龙", "凤", "鬼", "神", "魂", "仙"];
                    let r = rare_radicals[self.rng_next() as usize % rare_radicals.len()];
                    self.player.add_radical(r);
                    self.message = format!(
                        "🖥 Challenge Terminal conquered! +15 gold, +5 HP, bonus radical {}!",
                        r
                    );
                    self.message_timer = 100;
                    self.flash = Some((100, 255, 100, 0.3));
                    if let Some(ref audio) = self.audio {
                        audio.play_treasure();
                    }
                } else {
                    self.player.hp -= 3;
                    self.player.statuses.push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Weakened,
                        2,
                    ));
                    self.message = "🖥 Challenge Terminal failed! −3 HP, Weakened for 2 turns!".to_string();
                    self.message_timer = 100;
                    self.flash = Some((255, 50, 50, 0.3));
                    if self.player.hp <= 0 {
                        self.combat = CombatState::GameOver;
                    }
                }
            }

            // All other room types — no special interaction on tile step
            _ => {}
        }
    }
}
