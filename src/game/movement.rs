//! Player movement, look mode, and FOV.

use super::*;

impl GameState {
    pub(super) fn effective_fov(&self) -> i32 {
        let base = if self.current_room_modifier() == Some(RoomModifier::PoweredDown)
            || self.current_special_room() == Some(SpecialRoomKind::DarkSector)
        {
            2
        } else {
            FOV_RADIUS
        };
        base + self.player.enchant_fov_bonus() + self.crew_bonus(CrewRole::ScienceOfficer)
    }

    pub(super) fn look_text(&self, x: i32, y: i32) -> String {
        if !self.level.in_bounds(x, y) || !in_look_range(self.player.x, self.player.y, x, y) {
            return format!("Look range is {} tiles.", LOOK_RANGE);
        }

        let idx = self.level.idx(x, y);
        if x == self.player.x && y == self.player.y {
            return format!(
                "You are here — {} form, HP {}/{}.",
                self.player.form.name(),
                self.player.hp,
                self.player.max_hp
            );
        }

        if !self.level.revealed[idx] {
            return "Unseen darkness.".to_string();
        }

        if self.level.visible[idx] {
            if let Some(enemy_idx) = self.enemy_at(x, y) {
                return enemy_look_text(&self.enemies[enemy_idx]);
            }
        }

        let mut text = tile_look_text(self.level.tile(x, y));
        if !self.level.visible[idx] {
            text.push_str(" (remembered)");
        }
        text
    }

    pub(super) fn update_look_message(&mut self, x: i32, y: i32) {
        self.message = self.look_text(x, y);
        self.message_timer = 255;
    }

    pub(super) fn move_look_cursor(&mut self, dx: i32, dy: i32) {
        let CombatState::Looking { x, y } = self.combat.clone() else {
            return;
        };
        let min_x = (self.player.x - LOOK_RANGE).max(0);
        let max_x = (self.player.x + LOOK_RANGE).min(self.level.width - 1);
        let min_y = (self.player.y - LOOK_RANGE).max(0);
        let max_y = (self.player.y + LOOK_RANGE).min(self.level.height - 1);
        let next_x = (x + dx).clamp(min_x, max_x);
        let next_y = (y + dy).clamp(min_y, max_y);
        self.combat = CombatState::Looking {
            x: next_x,
            y: next_y,
        };
        self.update_look_message(next_x, next_y);
    }

    /// Try to move player. Bumping into an enemy starts combat.
    pub(super) fn try_move(&mut self, dx: i32, dy: i32) {
        if matches!(self.combat, CombatState::GameOver) {
            return;
        }
        // If fighting, ignore movement
        if matches!(
            self.combat,
            CombatState::Fighting { .. } | CombatState::TacticalBattle(_)
        ) {
            return;
        }

        // Tick player statuses
        // Decrement combo timer
        if self.spell_combo_timer > 0 {
            self.spell_combo_timer -= 1;
            if self.spell_combo_timer == 0 {
                self.last_spell = None;
            }
        }
        let (pdmg, pheal) = status::tick_statuses(&mut self.player.statuses);
        self.player.tick_form();

        // Hunger clock: grace period then periodic HP drain
        {
            let grace: u32 = if self.floor_num <= 1 { 200 } else { 100 };
            let warning_threshold = grace * 4 / 5;

            if self.move_count == warning_threshold {
                self.message = "⚠ Your energy reserves are running low...".to_string();
                self.message_timer = 60;
            } else if self.move_count > grace && (self.move_count - grace) % 25 == 0 {
                let starvation_dmg = 1 + self.floor_num / 5;
                self.player.hp -= starvation_dmg;
                self.message = format!(
                    "🌑 Exhaustion drains your life force! (-{} HP)",
                    starvation_dmg
                );
                self.message_timer = 40;
                if self.player.hp <= 0 && !self.try_phoenix_revive() {
                    self.player.hp = 0;
                    if let Some(ref audio) = self.audio {
                        audio.play_death();
                    }
                    self.run_journal
                        .log(RunEvent::DiedTo("Starvation".to_string(), self.floor_num));
                    self.post_mortem_page = 0;
                    self.combat = CombatState::GameOver;
                    self.message = self.run_summary();
                    self.message_timer = 255;
                    return;
                }
            }
        }

        if pdmg > 0 {
            self.player.hp -= pdmg;
            self.message = format!("☠ Poison deals {} damage!", pdmg);
            self.message_timer = 40;
            if self.player.hp <= 0 && !self.try_phoenix_revive() {
                self.player.hp = 0;
                if let Some(ref audio) = self.audio {
                    audio.play_death();
                }
                self.run_journal
                    .log(RunEvent::DiedTo("Poison".to_string(), self.floor_num));
                self.post_mortem_page = 0;
                self.combat = CombatState::GameOver;
                self.message = self.run_summary();
                self.message_timer = 255;
                return;
            }
        }
        if pheal > 0 {
            self.player.hp = (self.player.hp + pheal).min(self.player.max_hp);
        }

        // Tick enemy statuses
        for e in &mut self.enemies {
            if e.is_alive() {
                let (edmg, _) = status::tick_statuses(&mut e.statuses);
                if edmg > 0 {
                    e.hp -= edmg;
                }
            }
        }

        // Confused: randomize direction
        let (dx, dy) = if status::has_confused(&self.player.statuses) {
            let dirs = [(0, -1), (0, 1), (-1, 0), (1, 0)];
            dirs[self.rng_next() as usize % 4]
        } else {
            (dx, dy)
        };

        // If map-reveal status active, reveal all tiles
        if status::has_revealed(&self.player.statuses) {
            self.reveal_entire_floor();
        }

        // Rooted: block movement
        if status::has_rooted(&self.player.statuses) {
            self.message = "⚓ You're anchored and cannot move!".to_string();
            self.message_timer = 40;
            return;
        }

        let (nx, ny) = self.player.intended_move(dx, dy);
        let target_tile = self.level.tile(nx, ny);
        if target_tile == Tile::CargoCrate {
            let pdx = nx - self.player.x;
            let pdy = ny - self.player.y;
            let px = nx + pdx;
            let py = ny + pdy;

            // Check bounds
            if !self.level.in_bounds(px, py) {
                self.message = "It's jammed against the wall.".to_string();
                self.message_timer = 30;
                return;
            }

            let push_target_idx = self.level.idx(px, py);
            let push_target_tile = self.level.tiles[push_target_idx];

            // Allow pushing into open space or liquids
            let can_push = matches!(
                push_target_tile,
                Tile::MetalFloor
                    | Tile::Hallway
                    | Tile::CoolantPool
                    | Tile::VacuumBreach
                    | Tile::Coolant
                    | Tile::LaserGrid
                    | Tile::Catwalk
            );
            let enemy_behind = self.enemy_at(px, py).is_some();

            if can_push && !enemy_behind {
                if matches!(push_target_tile, Tile::CoolantPool | Tile::VacuumBreach) {
                    self.level.tiles[push_target_idx] = Tile::Catwalk;
                    self.message = if push_target_tile == Tile::VacuumBreach {
                        "The crate drops into the pool, forming a rough bridge!".to_string()
                    } else {
                        "The crate splashes into place, forming a bridge!".to_string()
                    };
                    self.message_timer = 80;
                    let (sx, sy) = self.tile_to_screen(px, py);
                    self.particles.spawn_bridge(sx, sy, &mut self.rng_state);
                    self.trigger_shake(4);
                    if let Some(ref audio) = self.audio {
                        audio.play_bridge();
                    }
                } else {
                    self.level.tiles[push_target_idx] = Tile::CargoCrate;
                    self.message = "You shove the crate aside.".to_string();
                    self.message_timer = 40;
                }
                let current_idx = self.level.idx(nx, ny);
                self.level.tiles[current_idx] = Tile::MetalFloor;

                // Player moves into the spot
                self.player.x = nx;
                self.player.y = ny;
                self.move_count += 1;

                let skip_enemy =
                    status::has_haste(&self.player.statuses) && self.move_count % 2 == 0;
                if !skip_enemy {
                    self.enemy_turn();
                }
                let (px, py) = (self.player.x, self.player.y);
                let fov = self.effective_fov();
                compute_fov(&mut self.level, px, py, fov);
                return;
            } else {
                self.message = "It won't budge.".to_string();
                self.message_timer = 30;
                return;
            }
        }

        if matches!(
            target_tile,
            Tile::Bulkhead | Tile::DamagedBulkhead | Tile::WeakBulkhead
        ) {
            // Check for digging using weapon effect
            let can_dig = self
                .player
                .weapon
                .map_or(false, |eq| matches!(eq.effect, EquipEffect::Digging))
                || self.player.form == PlayerForm::Cybernetic;

            if can_dig {
                let cracked_wall = target_tile == Tile::DamagedBulkhead;
                let brittle_wall = target_tile == Tile::WeakBulkhead;
                let idx = self.level.idx(nx, ny);
                self.level.tiles[idx] = Tile::MetalFloor;
                let (sx, sy) = self.tile_to_screen(nx, ny);
                self.particles.spawn_dig(sx, sy, &mut self.rng_state);
                self.trigger_shake(if cracked_wall {
                    6
                } else if brittle_wall {
                    5
                } else {
                    4
                });
                if let Some(ref audio) = self.audio {
                    audio.play_dig();
                }
                self.message = if cracked_wall {
                    "You smash through the cracked wall and uncover a hidden chamber!".to_string()
                } else if brittle_wall {
                    "You break through the brittle wall and crack open the cache!".to_string()
                } else {
                    "Stone chips fly as you dig a rough tunnel.".to_string()
                };
                self.message_timer = if cracked_wall {
                    120
                } else if brittle_wall {
                    100
                } else {
                    75
                };
                self.move_count += 1;

                let skip_enemy =
                    status::has_haste(&self.player.statuses) && self.move_count % 2 == 0;
                if !skip_enemy {
                    self.enemy_turn();
                }
                let (px, py) = (self.player.x, self.player.y);
                let fov = self.effective_fov();
                compute_fov(&mut self.level, px, py, fov);
                return;
            }

            if target_tile == Tile::DamagedBulkhead {
                self.message =
                    "The wall is cracked. A digging tool could break through.".to_string();
                self.message_timer = 90;
            } else if target_tile == Tile::WeakBulkhead {
                self.message =
                    "The brittle wall could be smashed open with a digging tool.".to_string();
                self.message_timer = 90;
            }
        }

        if target_tile == Tile::SealedHatch {
            if let Some(ref audio) = self.audio { audio.sfx_door(); }
            self.start_locked_door(nx, ny);
            return;
        }

        // Boulder pushing (similar to crate)
        if target_tile == Tile::CargoCrate {
            let pdx = nx - self.player.x;
            let pdy = ny - self.player.y;
            let px = nx + pdx;
            let py = ny + pdy;
            if self.level.in_bounds(px, py) {
                let push_idx = self.level.idx(px, py);
                let push_tile = self.level.tiles[push_idx];
                if push_tile == Tile::PressureSensor || push_tile == Tile::MetalFloor {
                    let boulder_idx = self.level.idx(nx, ny);
                    self.level.tiles[boulder_idx] = Tile::MetalFloor;
                    self.level.tiles[push_idx] = Tile::CargoCrate;
                    if push_tile == Tile::PressureSensor {
                        self.message = "🪨 The boulder clicks onto the pressure plate!".to_string();
                        self.trigger_shake(4);
                    } else {
                        self.message = "🪨 You heave the boulder aside.".to_string();
                    }
                    self.message_timer = 50;
                    self.player.x = nx;
                    self.player.y = ny;
                    self.move_count += 1;
                    let skip_enemy =
                        status::has_haste(&self.player.statuses) && self.move_count % 2 == 0;
                    if !skip_enemy {
                        self.enemy_turn();
                    }
                    let (px2, py2) = (self.player.x, self.player.y);
                    let fov = self.effective_fov();
                    compute_fov(&mut self.level, px2, py2, fov);
                    return;
                } else {
                    self.message = "The boulder won't budge that way.".to_string();
                    self.message_timer = 30;
                    return;
                }
            } else {
                self.message = "The boulder is jammed against the wall.".to_string();
                self.message_timer = 30;
                return;
            }
        }

        // Bookshelf interaction
        if target_tile == Tile::DataRack {
            self.message = "📚 You study the ancient texts and feel wiser.".to_string();
            self.message_timer = 60;
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        // GoldOre mining
        if target_tile == Tile::OreVein {
            let mut gold = 5 + (self.rng_next() % 11) as i32;
            // AsteroidBase: double ore/gold from mining
            if self.current_location_type == Some(crate::world::LocationType::AsteroidBase) {
                gold *= 2;
            }
            self.player.gold += gold;
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            self.message = format!("⛏ You mine {} gold from the ore vein!", gold);
            self.message_timer = 60;
            self.trigger_shake(3);
            return;
        }

        // Crystal wall — not walkable, just shows message
        if target_tile == Tile::CrystalPanel {
            self.message = "💎 The crystal formation is too solid to pass.".to_string();
            self.message_timer = 40;
            return;
        }

        // Bamboo — not walkable without cutting
        if target_tile == Tile::CargoPipes {
            let can_dig = self
                .player
                .weapon
                .map_or(false, |eq| matches!(eq.effect, EquipEffect::Digging))
                || self.player.form == PlayerForm::Cybernetic;
            if can_dig {
                let idx = self.level.idx(nx, ny);
                self.level.tiles[idx] = Tile::MetalFloor;
                self.message = "🎋 You cut through the bamboo thicket!".to_string();
                self.message_timer = 50;
                self.trigger_shake(3);
                return;
            }
            self.message = "🎋 Dense bamboo blocks the way. A cutting tool might help.".to_string();
            self.message_timer = 60;
            return;
        }

        if !target_tile.is_walkable() {
            return;
        }

        // Check for enemy bump → start tactical combat
        if let Some(idx) = self.enemy_at(nx, ny) {
            let mut combat_indices = vec![idx];
            let mut sentence_ambush = false;

            if self.floor_num >= 8 && !self.enemies[idx].is_boss {
                let chance = if self.floor_num >= 15 { 3 } else { 5 };
                if self.rng_next() % chance == 0 {
                    let pool = vocab::sentences_for_floor(self.floor_num);
                    if !pool.is_empty() {
                        let entry_idx = self.rng_next() as usize % pool.len();
                        let sentence = &pool[entry_idx];
                        let sent_enemies =
                            combat::transition::enemies_from_sentence(sentence, self.floor_num);
                        for mut se in sent_enemies {
                            se.x = nx;
                            se.y = ny;
                            self.enemies.push(se);
                            combat_indices.push(self.enemies.len() - 1);
                        }
                        sentence_ambush = true;
                    }
                }
            }

            let battle = combat::transition::enter_combat(
                &self.player,
                &self.enemies,
                &combat_indices,
                self.floor_num,
                self.current_room_modifier(),
                &self.srs,
                self.companion,
            );
            self.combat = CombatState::TacticalBattle(Box::new(battle));
            self.typing.clear();
            self.guard_used_this_fight = false;
            self.guard_blocks_used = 0;
            if let Some(ref audio) = self.audio {
                let has_boss = combat_indices.iter().any(|&ei| self.enemies[ei].is_boss);
                if has_boss {
                    audio.play_boss_encounter();
                } else {
                    audio.play_combat_start();
                }
            }
            if self.listening_mode.is_active() && !self.enemies[idx].is_elite {
                let pinyin = self.enemies[idx].pinyin;
                let tone_num = pinyin
                    .chars()
                    .last()
                    .and_then(|c| c.to_digit(10))
                    .unwrap_or(1) as u8;
                if let Some(ref audio) = self.audio {
                    audio.play_chinese_tone(tone_num);
                }
                self.message =
                    combat_prompt_for(&self.enemies[idx], self.listening_mode, self.mirror_hint);
            } else {
                self.message =
                    combat_prompt_for(&self.enemies[idx], self.listening_mode, self.mirror_hint);
            }
            if let Some(ref comp) = self.companion {
                let lvl = self.companion_level();
                if let Some(hint) = comp.contextual_hint(
                    &self.enemies[idx],
                    self.player.hp,
                    self.player.max_hp,
                    self.guard_used_this_fight,
                    lvl,
                ) {
                    self.message.push_str(&format!("\n{}", hint));
                }
            }
            if sentence_ambush {
                self.message
                    .push_str("\n\u{26a0} Sentence ambush! Extra enemies joined the fight!");
            }
            self.message_timer = 255;
            return;
        }

        if target_tile == Tile::Airlock {
            if let Some(blocker) = self.tutorial_exit_blocker() {
                self.message = blocker.to_string();
                self.message_timer = 150;
                return;
            }
        }

        self.player.move_to(nx, ny);
        if let Some(ref audio) = self.audio {
            audio.play_step();
        }

        if let Tile::InfoPanel(sign_id) = target_tile {
            self.show_tutorial_sign(sign_id);
        }

        if let Tile::SecurityLock(kind) = target_tile {
            self.trigger_seal(nx, ny, kind, None);
        }

        // Stairs
        if target_tile == Tile::Airlock {
            self.descend_floor(false);
            return;
        }

        // Forge workbench
        if target_tile == Tile::QuantumForge {
            if self.player.radicals.is_empty() {
                self.message = "Forge workbench — but you have no radicals!".to_string();
                self.message_timer = 60;
            } else {
                let recipes = radical::craftable_recipes(&self.player.radicals);
                if recipes.is_empty() {
                    self.message =
                        "Forge workbench — no recipes available with your radicals.".to_string();
                    self.message_timer = 80;
                } else {
                    let count = recipes.len();
                    self.combat = CombatState::Forging { recipes, cursor: 0 };
                    self.message = format!(
                        "{} recipe{} available. ↑/↓ browse, Enter forge, E enchant, Esc close.",
                        count,
                        if count == 1 { "" } else { "s" }
                    );
                    self.message_timer = 255;
                }
                let (px, py) = (self.player.x, self.player.y);
                compute_fov(&mut self.level, px, py, FOV_RADIUS);
                return;
            }
        }

        // Shop
        if target_tile == Tile::TradeTerminal {
            if self.shop_banned {
                self.message =
                    "🚫 The shopkeeper slams the door shut. You're not welcome here!".to_string();
                self.message_timer = 80;
                return;
            }
            let items = self.generate_shop_items();
            self.combat = CombatState::Shopping { items, cursor: 0 };
            self.message =
                "Welcome to the shop! ↑↓ browse, Enter buy, G grab (steal), Esc leave.".to_string();
            self.message_timer = 255;
            let (px, py) = (self.player.x, self.player.y);
            compute_fov(&mut self.level, px, py, FOV_RADIUS);
            return;
        }

        // Chest
        if target_tile == Tile::SupplyCrate {
            self.open_chest(nx, ny);
        }

        // NPC companion recruit or quest
        if let Tile::Npc(npc_type) = target_tile {
            let comp = match npc_type {
                0 => Companion::ScienceOfficer,
                1 => Companion::Medic,
                2 => Companion::Quartermaster,
                _ => Companion::SecurityChief,
            };
            if self.companion.is_some() {
                if self.quests.len() < 2 {
                    let has_active_chain = self.quests.iter().any(|q| q.is_chain() && !q.completed);
                    let quest = if !has_active_chain && (self.rng_next() % 100) < 30 {
                        let cid = self.next_chain_id;
                        self.next_chain_id += 1;
                        self.generate_chain_quest(0, cid)
                    } else {
                        self.generate_quest()
                    };
                    self.message = format!("{} gives quest: {}", comp.icon(), quest.description);
                    self.quests.push(quest);
                } else {
                    self.message = format!(
                        "{} {} waves hello! (quest slots full)",
                        comp.icon(),
                        comp.name()
                    );
                }
            } else {
                self.companion = Some(comp);
                self.companion_xp = 0;
                self.merchant_reroll_used = false;
                self.message = format!("{} {} joins your party!", comp.icon(), comp.name());
            }
            self.message_timer = 100;
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
        }

        if let Tile::Terminal(kind) = target_tile {
            self.invoke_altar(nx, ny, kind);
        }

        // Tone shrine interaction
        if target_tile == Tile::CircuitShrine {
            self.start_tone_battle();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::CompoundShrine {
            self.start_stroke_order();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::FrequencyWall {
            self.start_tone_defense();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::CompoundShrine {
            self.start_compound_builder();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::ClassifierNode {
            self.start_classifier_match();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::DataWell {
            self.start_ink_well();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::MemorialNode {
            self.start_ancestor_challenge();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::TranslationTerminal {
            self.start_translation_challenge();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::RadicalLab {
            self.start_radical_garden();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::HoloPool {
            self.start_mirror_pool();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::DroidTutor {
            self.start_stone_tutor();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::CodexTerminal {
            self.start_codex_challenge();
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            return;
        }

        if target_tile == Tile::DataBridge {
            let dirs = [(0, -1), (0, 1), (-1, 0), (1, 0)];
            let mut bridge_target = (nx, ny);
            for (ddx, ddy) in &dirs {
                let bx = nx + ddx;
                let by = ny + ddy;
                if self.level.in_bounds(bx, by) && self.level.tile(bx, by) == Tile::VacuumBreach {
                    bridge_target = (bx, by);
                    break;
                }
            }
            self.start_word_bridge(bridge_target.0, bridge_target.1);
            return;
        }

        if target_tile == Tile::CorruptedFloor {
            self.player.move_to(nx, ny);
            let idx = self.level.idx(nx, ny);
            self.level.tiles[idx] = Tile::MetalFloor;
            self.start_cursed_floor();
            return;
        }

        // Dragon Gate Portal interaction
        if target_tile == Tile::WarpGatePortal {
            self.message = "🐉 The Dragon Gate pulses with ancient power! You are not yet ready...".to_string();
            self.message_timer = 100;
        }

        // Show special room description on first entry
        if let Some(special) = self.current_special_room() {
            let sr_desc = special.description();
            if !self.message.contains(sr_desc) && self.message_timer < 20 {
                self.message = format!("📍 {} — {}", special.name(), sr_desc);
                self.message_timer = 100;
            }
        }

        // Handle special room interactions on tile step
        self.handle_special_room_interaction(target_tile);

        self.apply_player_tile_effect(target_tile);
        if matches!(self.combat, CombatState::GameOver) {
            return;
        }

        // After player moves, enemies take a turn(skipped on even moves during haste)
        self.move_count += 1;
        let skip_enemy = status::has_haste(&self.player.statuses) && self.move_count % 2 == 0;
        if !skip_enemy {
            self.enemy_turn();
        }

        let (px, py) = (self.player.x, self.player.y);
        let fov = self.effective_fov();
        compute_fov(&mut self.level, px, py, fov);
        self.companion_exploration_hint();
    }
}
