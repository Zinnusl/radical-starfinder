//! Save/load and game state serialization.

use std::collections::HashMap;
use web_sys::window;
use super::*;
use crate::player::{Player, PlayerClass};
use crate::radical;

pub(crate) fn parse_i32(map: &HashMap<String, String>, key: &str, default: i32) -> i32 {
    map.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
}

pub(crate) fn parse_u32(map: &HashMap<String, String>, key: &str, default: u32) -> u32 {
    map.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
}

pub(crate) fn parse_u64(map: &HashMap<String, String>, key: &str, default: u64) -> u64 {
    map.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
}

impl super::GameState {
    pub(crate) fn restart(&mut self) {
        self.total_runs += 1;
        self.save_high_score();
        self.save_stats();
        self.srs = crate::srs::load_srs();
        self.player = Player::new(0, 0, PlayerClass::Envoy);
        self.floor_num = 0;
        self.run_kills = 0;
        self.run_gold_earned = 0;
        self.run_correct_answers = 0;
        self.run_wrong_answers = 0;
        self.run_spells_forged = 0;
        self.run_bosses_killed = 0;
        self.mirror_hint = false;
        self.next_chain_id = 1;
        self.floor_profile = FloorProfile::Normal;
        self.answer_streak = 0;
        self.run_journal = RunJournal::default();
        self.post_mortem_page = 0;
        self.theft_catches = 0;
        self.shop_banned = false;
        self.enemies.clear();
        self.typing.clear();
        // Keep discovered recipes across runs (loaded from localStorage)
        self.combat = CombatState::ClassSelect;
        self.tutorial = None;
        self.show_inventory = false;
        self.show_help = false;
        self.show_settings = false;
        self.crafting_mode = false;
        self.crafting_first = None;
        self.crafting_cursor = 0;
        self.message_tick_delay = 0;
        self.new_floor();
    }

    pub(crate) fn save_high_score(&self) {
        crate::srs::save_srs(&self.srs);
        self.save_stats();
        let storage: Option<web_sys::Storage> =
            window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        if let Some(storage) = storage {
            let prev: i32 = storage
                .get_item("radical_roguelike_best")
                .ok()
                .flatten()
                .and_then(|s: String| s.parse::<i32>().ok())
                .unwrap_or(0);
            if self.best_floor > prev {
                let _ = storage.set_item("radical_roguelike_best", &self.best_floor.to_string());
            }
            // Save discovered recipes
            let recipe_str: String = self
                .discovered_recipes
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let _ = storage.set_item("radical_roguelike_recipes", &recipe_str);

            // Save daily best
            if self.daily_mode {
                let score = self.daily_score();
                let prev_daily: i32 = storage
                    .get_item("radical_roguelike_daily_best")
                    .ok()
                    .flatten()
                    .and_then(|s: String| s.parse::<i32>().ok())
                    .unwrap_or(0);
                if score > prev_daily {
                    let _ = storage.set_item("radical_roguelike_daily_best", &score.to_string());
                }
            }
        }
    }

    /// Calculate daily challenge score.
    pub(crate) fn daily_score(&self) -> i32 {
        self.floor_num * 100 + self.player.gold + self.total_kills as i32 * 10
    }

    pub(crate) fn load_high_score() -> i32 {
        let storage: Option<web_sys::Storage> =
            window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        storage
            .and_then(|s: web_sys::Storage| s.get_item("radical_roguelike_best").ok().flatten())
            .and_then(|s: String| s.parse::<i32>().ok())
            .unwrap_or(0)
    }

    pub(crate) fn save_game(&self) {
        let win = match window() {
            Some(w) => w,
            None => return,
        };
        let storage = match win.local_storage().ok().flatten() {
            Some(s) => s,
            None => return,
        };

        let mut save = String::new();

        // Player stats
        save.push_str(&format!("hp={}\n", self.player.hp));
        save.push_str(&format!("max_hp={}\n", self.player.max_hp));
        save.push_str(&format!("gold={}\n", self.player.gold));
        save.push_str(&format!("class={}\n", self.player.class as u8));
        save.push_str(&format!("floor={}\n", self.floor_num));
        save.push_str(&format!("best={}\n", self.best_floor));

        // Ship stats
        save.push_str(&format!("ship_hull={}\n", self.ship.hull));
        save.push_str(&format!("ship_max_hull={}\n", self.ship.max_hull));
        save.push_str(&format!("ship_fuel={}\n", self.ship.fuel));
        save.push_str(&format!("ship_max_fuel={}\n", self.ship.max_fuel));
        save.push_str(&format!("ship_shields={}\n", self.ship.shields));
        save.push_str(&format!("ship_max_shields={}\n", self.ship.max_shields));
        save.push_str(&format!("ship_weapon={}\n", self.ship.weapon_power));
        save.push_str(&format!("ship_engine={}\n", self.ship.engine_power));
        save.push_str(&format!("ship_sensor={}\n", self.ship.sensor_range));
        save.push_str(&format!("ship_cargo_cap={}\n", self.ship.cargo_capacity));
        save.push_str(&format!("ship_cargo_used={}\n", self.ship.cargo_used));

        // Sector map position
        if let Some(ref map) = self.sector_map {
            save.push_str(&format!("sector={}\n", map.current_sector));
            save.push_str(&format!("system={}\n", map.current_system));
        }

        // Stats
        save.push_str(&format!("kills={}\n", self.total_kills));
        save.push_str(&format!("runs={}\n", self.total_runs));
        save.push_str(&format!("seed={}\n", self.seed));

        // Number of crew
        save.push_str(&format!("crew_count={}\n", self.crew.len()));

        storage.set_item("radical_starfinder_save", &save).ok();
    }

    pub(crate) fn load_game_data() -> Option<HashMap<String, String>> {
        let win = window()?;
        let storage = win.local_storage().ok()??;
        let data = storage.get_item("radical_starfinder_save").ok()??;
        let mut map = HashMap::new();
        for line in data.lines() {
            if let Some((key, val)) = line.split_once('=') {
                map.insert(key.to_string(), val.to_string());
            }
        }
        Some(map)
    }

    #[allow(dead_code)]
    pub(crate) fn has_save() -> bool {
        window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item("radical_starfinder_save").ok().flatten())
            .is_some()
    }

    pub(crate) fn load_recipes() -> Vec<usize> {
        let storage: Option<web_sys::Storage> =
            window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        storage
            .and_then(|s: web_sys::Storage| s.get_item("radical_roguelike_recipes").ok().flatten())
            .map(|s: String| {
                s.split(',')
                    .filter_map(|v| v.parse::<usize>().ok())
                    .filter(|&i| i < radical::RECIPES.len())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn load_settings() -> GameSettings {
        let storage: Option<web_sys::Storage> =
            window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        if let Some(storage) = storage {
            let music_volume = storage
                .get_item("radical_roguelike_music_volume")
                .ok()
                .flatten()
                .and_then(|s: String| s.parse::<u8>().ok())
                .filter(|v| *v <= 100)
                .unwrap_or(100);
            let sfx_volume = storage
                .get_item("radical_roguelike_sfx_volume")
                .ok()
                .flatten()
                .and_then(|s: String| s.parse::<u8>().ok())
                .filter(|v| *v <= 100)
                .unwrap_or(100);
            let screen_shake = storage
                .get_item("radical_roguelike_screen_shake")
                .ok()
                .flatten()
                .map(|s: String| s != "0")
                .unwrap_or(true);
            let text_speed = storage
                .get_item("radical_roguelike_text_speed")
                .ok()
                .flatten()
                .map(|s| TextSpeed::from_storage(&s))
                .unwrap_or(TextSpeed::Normal);
            GameSettings {
                music_volume,
                sfx_volume,
                screen_shake,
                text_speed,
            }
        } else {
            GameSettings::default()
        }
    }

    pub(crate) fn load_stat(key: &str) -> u32 {
        let storage: Option<web_sys::Storage> =
            window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        storage
            .and_then(|s: web_sys::Storage| s.get_item(key).ok().flatten())
            .and_then(|s: String| s.parse::<u32>().ok())
            .unwrap_or(0)
    }

    pub(crate) fn save_stats(&self) {
        let storage: Option<web_sys::Storage> =
            window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        if let Some(storage) = storage {
            let _ = storage.set_item("radical_roguelike_runs", &self.total_runs.to_string());
            let _ = storage.set_item("radical_roguelike_kills", &self.total_kills.to_string());
        }
    }

    pub(crate) fn save_settings(&self) {
        let storage: Option<web_sys::Storage> =
            window().and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        if let Some(storage) = storage {
            let _ = storage.set_item(
                "radical_roguelike_music_volume",
                &self.settings.music_volume.to_string(),
            );
            let _ = storage.set_item(
                "radical_roguelike_sfx_volume",
                &self.settings.sfx_volume.to_string(),
            );
            let _ = storage.set_item(
                "radical_roguelike_screen_shake",
                if self.settings.screen_shake { "1" } else { "0" },
            );
            let _ = storage.set_item(
                "radical_roguelike_text_speed",
                self.settings.text_speed.storage_key(),
            );
        }
    }

}
