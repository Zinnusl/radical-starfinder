//! Main game rendering dispatch.

use super::*;

impl GameState {
    pub(super) fn render(&self) {
        // ── Game mode dispatch ──
        match self.game_mode {
            GameMode::Starmap => {
                if let Some(ref map) = self.sector_map {
                    let selected_target = if let Some(sector) = map.sectors.get(map.current_sector) {
                        let sys = &sector.systems[map.current_system];
                        let connections = &sys.connections;
                        if !connections.is_empty() {
                            Some(connections[self.starmap_cursor % connections.len()])
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    self.renderer.draw_starmap(map, js_sys::Date::now() / 1000.0, &self.settings, selected_target);
                    // Draw HUD overlay with fuel, hull, current system info
                    self.renderer.draw_starmap_hud(map, &self.ship, self.starmap_cursor);
                }
                if self.show_class_select {
                    self.renderer.draw_class_select(self.class_cursor, self.has_continue_option);
                }
                return;
            }
            GameMode::ShipInterior => {
                self.renderer.draw_ship_interior(
                    &self.ship_layout, self.ship_player_x, self.ship_player_y,
                    &self.crew, &self.ship, &self.message,
                    self.player.hp, self.player.max_hp, self.player.gold,
                    self.show_ship_help,
                );
                if self.show_ship_upgrades {
                    self.renderer.draw_ship_upgrades(self.ship_upgrade_cursor, &self.ship.upgrades, self.player.gold);
                }
                return;
            }
            GameMode::SpaceCombat => {
                if let Some(ref enemy) = self.enemy_ship {
                    self.renderer.draw_space_combat(
                        &self.ship, enemy, &self.space_combat_phase,
                        self.space_combat_cursor, self.space_combat_target_cursor,
                        &self.space_combat_log, &self.crew,
                        self.space_combat_weapon, self.space_combat_evading,
                        js_sys::Date::now() / 1000.0,
                    );
                }
                return;
            }
            GameMode::Event => {
                if let Some(event_idx) = self.current_event {
                    if let Some(event) = ALL_EVENTS.get(event_idx) {
                        self.renderer.draw_event(event, self.event_choice_cursor);
                    }
                }
                return;
            }
            GameMode::DungeonEvent => {
                if let Some(dlg_idx) = self.current_dungeon_dialogue {
                    if let Some(dlg) = crate::world::dialogue::ALL_DUNGEON_DIALOGUES.get(dlg_idx) {
                        self.renderer.draw_dungeon_dialogue(dlg, self.dungeon_dialogue_cursor);
                    }
                }
                return;
            }
            GameMode::LocationExploration | GameMode::GroundCombat => {
                // Fall through to existing render logic
            }
        }
        
        let popup = self.achievement_popup.map(|(n, d, _)| (n, d));
        let room_mod = self.current_room_modifier();
        let tutorial_hint = self.tutorial_hint();
        let show_help =
            self.show_help && !self.show_inventory && !self.show_codex && !self.show_settings;
        let item_labels: Vec<String> = self
            .player
            .items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let state = self
                    .player
                    .item_states
                    .get(idx)
                    .copied()
                    .unwrap_or(ItemState::Normal);
                let prefix = match state {
                    ItemState::Cursed => "💀 ",
                    ItemState::Blessed => "✨ ",
                    ItemState::Normal => "",
                };
                format!("{}{}", prefix, self.item_display_name(item))
            })
            .collect();
        self.renderer.draw(
            &self.level,
            &self.player,
            &self.enemies,
            &self.combat,
            &self.typing,
            &self.message,
            self.floor_num,
            self.best_floor,
            self.total_kills,
            self.total_runs,
            self.discovered_recipes.len(),
            &self.srs,
            &self.particles,
            if self.settings.screen_shake {
                self.shake_timer
            } else {
                0
            },
            self.flash,
            popup,
            room_mod,
            self.listening_mode,
            self.companion,
            self.companion_level(),
            &self.quests,
            tutorial_hint,
            show_help,
            &item_labels,
            &self.settings,
            self.show_settings,
            self.settings_cursor,
            self.answer_streak,
            self.floor_profile.label(),
            &self.codex,
            &self.run_journal,
            self.post_mortem_page,
            self.class_cursor,
            self.current_location_type.map(|lt| lt.label()).unwrap_or(""),
            self.current_location_type.map(|lt| lt.bonus_description()).unwrap_or(""),
            self.show_minimap,
            self.shop_sell_mode,
        );
        if self.show_inventory {
            self.renderer.draw_inventory(
                &self.player,
                self.floor_num,
                self.discovered_recipes.len(),
                self.best_floor,
                self.total_kills,
                self.companion,
                self.companion_level(),
                &item_labels,
                self.inventory_cursor,
                self.inventory_inspect,
                self.crafting_mode,
                self.crafting_first,
                self.crafting_cursor,
            );
        } else if self.show_skill_tree {
            self.renderer.draw_skill_tree(&self.player, self.skill_tree_cursor);
        } else if self.show_crucible {
            self.renderer.draw_crucible(&self.player, self.crucible_cursor);
        } else if self.show_spellbook {
            self.renderer.draw_spellbook(&self.player);
        } else if self.show_codex {
            let entries = self.codex.sorted_entries();
            self.renderer.draw_codex(&entries);
        }

        if self.console.active {
            self.renderer
                .draw_console(&self.console.history, &self.console.input_buffer, self.console.scroll_offset);
        }
    }

}
