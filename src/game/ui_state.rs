//! UI state management — menus, overlays, settings.

use super::*;
use crate::player::{self, ItemState};

impl super::GameState {
    pub(crate) fn trigger_shake(&mut self, frames: u8) {
        if self.settings.screen_shake {
            self.shake_timer = self.shake_timer.max(frames);
        }
    }

    pub(crate) fn open_settings(&mut self) {
        self.show_settings = true;
        self.settings_cursor = 0;
    }

    pub(crate) fn close_settings(&mut self) {
        self.show_settings = false;
    }

    pub(crate) fn open_inventory(&mut self) {
        self.show_inventory = true;
        self.inventory_cursor = 0;
        self.inventory_inspect = None;
    }

    pub(crate) fn close_inventory(&mut self) {
        self.show_inventory = false;
        self.inventory_inspect = None;
        self.crafting_mode = false;
        self.crafting_first = None;
        self.crafting_cursor = 0;
    }

    pub(crate) fn try_craft(&mut self, first_idx: usize, second_idx: usize) {
        use crate::player::{find_crafting_recipe, crafted_item, ItemState};
        let kind1 = self.player.items[first_idx].kind();
        let kind2 = self.player.items[second_idx].kind();
        if let Some(recipe) = find_crafting_recipe(kind1, kind2) {
            let output = crafted_item(recipe, &self.player.items[first_idx], &self.player.items[second_idx]);
            let output_name = recipe.output_name;
            // Remove the higher index first to avoid shifting the lower one
            let (hi, lo) = if first_idx > second_idx {
                (first_idx, second_idx)
            } else {
                (second_idx, first_idx)
            };
            self.player.items.remove(hi);
            self.player.item_states.remove(hi);
            self.player.items.remove(lo);
            self.player.item_states.remove(lo);
            self.player.add_item(output, ItemState::Normal);
            if let Some(ref audio) = self.audio {
                audio.play_forge();
            }
            self.message = format!("✨ Crafted {}!", output_name);
            self.message_timer = 90;
            // Exit crafting mode after successful craft
            self.crafting_mode = false;
            self.crafting_first = None;
            self.crafting_cursor = 0;
        } else {
            self.message = "These items can't be combined.".to_string();
            self.message_timer = 60;
            self.crafting_first = None;
        }
    }

    pub(crate) fn start_look_mode(&mut self) {
        self.combat = CombatState::Looking {
            x: self.player.x,
            y: self.player.y,
        };
        self.update_look_message(self.player.x, self.player.y);
    }

    pub(crate) fn stop_look_mode(&mut self) {
        self.combat = CombatState::Explore;
        self.message.clear();
        self.message_timer = 0;
    }

    pub(crate) fn move_settings_cursor(&mut self, delta: i32) {
        let next = (self.settings_cursor as i32 + delta).clamp(0, 3);
        self.settings_cursor = next as usize;
    }

    pub(crate) fn adjust_volume(value: u8, delta: i8) -> u8 {
        (value as i16 + delta as i16 * 10).clamp(0, 100) as u8
    }

    pub(crate) fn adjust_selected_setting(&mut self, delta: i8) {
        match self.settings_cursor {
            0 => {
                self.settings.music_volume = Self::adjust_volume(self.settings.music_volume, delta)
            }
            1 => self.settings.sfx_volume = Self::adjust_volume(self.settings.sfx_volume, delta),
            2 => self.settings.screen_shake = !self.settings.screen_shake,
            3 => {
                self.settings.text_speed = if delta < 0 {
                    self.settings.text_speed.previous()
                } else {
                    self.settings.text_speed.next()
                };
            }
            _ => {}
        }
        self.apply_settings();
    }

    pub(crate) fn apply_settings(&mut self) {
        if !self.settings.screen_shake {
            self.shake_timer = 0;
        }
        if let Some(ref mut audio) = self.audio {
            audio.set_music_volume(self.settings.music_volume);
            audio.set_sfx_volume(self.settings.sfx_volume);
        }
        self.save_settings();
    }

    pub(crate) fn tick_message(&mut self) {
        if self.message_timer > 0
            && advance_message_decay(
                &mut self.message_timer,
                &mut self.message_tick_delay,
                self.settings.text_speed,
            )
        {
            self.message.clear();
        }
    }

}
