//! Main game state and loop.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement, KeyboardEvent};

use crate::achievement::AchievementTracker;
use crate::audio::Audio;
use crate::codex::Codex;
use crate::dungeon::{compute_fov, DungeonLevel, RoomModifier, Tile};
use crate::enemy::Enemy;
use crate::particle::ParticleSystem;
use crate::player::{Player, EQUIPMENT_POOL};
use crate::radical::{self, Spell, SpellEffect};
use crate::render::Renderer;
use crate::srs::SrsTracker;
use crate::status;
use crate::vocab::{self, VocabEntry};

const MAP_W: i32 = 48;
const MAP_H: i32 = 48;
const FOV_RADIUS: i32 = 8;
const ENEMIES_PER_ROOM: i32 = 1;

/// Combat phase when the player is adjacent to / engages an enemy.
#[derive(Clone, Debug)]
pub enum CombatState {
    /// Normal exploration — no active fight
    Explore,
    /// Fighting an enemy: index into `enemies` vec
    Fighting {
        enemy_idx: usize,
        timer_ms: f64,
    },
    /// Player is at a forge workbench, selecting radicals
    Forging {
        selected: Vec<usize>,
        page: usize,
    },
    /// Player is at a shop, browsing items
    Shopping {
        /// Items for sale: (description, cost, action)
        items: Vec<ShopItem>,
        cursor: usize,
    },
    /// Player is enchanting equipment at a forge
    Enchanting {
        /// 0=weapon, 1=armor, 2=charm
        slot: usize,
        /// Which page of radicals to show
        page: usize,
    },
    /// Player is dead
    GameOver,
}

#[derive(Clone, Debug)]
pub struct ShopItem {
    pub label: String,
    pub cost: i32,
    pub kind: ShopItemKind,
}

#[derive(Clone, Debug)]
pub enum ShopItemKind {
    Radical(&'static str),
    HealFull,
    Equipment(usize), // index into EQUIPMENT_POOL
    Consumable(crate::player::Item),
}

pub struct GameState {
    pub level: DungeonLevel,
    pub player: Player,
    pub renderer: Renderer,
    pub audio: Option<Audio>,
    pub floor_num: i32,
    pub seed: u64,
    pub enemies: Vec<Enemy>,
    pub combat: CombatState,
    pub typing: String,
    pub message: String,
    pub message_timer: u8,
    pub discovered_recipes: Vec<usize>,
    pub best_floor: i32,
    pub srs: SrsTracker,
    pub total_kills: u32,
    pub total_runs: u32,
    /// Move counter for haste effect
    pub move_count: u32,
    /// Particle effects
    pub particles: ParticleSystem,
    /// Screen shake remaining frames
    pub shake_timer: u8,
    /// Flash overlay (r, g, b, alpha 0.0..1.0)
    pub flash: Option<(u8, u8, u8, f64)>,
    /// Achievement tracker
    pub achievements: AchievementTracker,
    /// Achievement popup: (name, desc) + timer frames
    pub achievement_popup: Option<(&'static str, &'static str, u16)>,
    /// Character codex
    pub codex: Codex,
    /// Whether codex overlay is showing
    pub show_codex: bool,
    /// Last spell effect used (for combos)
    pub last_spell: Option<SpellEffect>,
    /// Turns since last spell (combo window)
    pub spell_combo_timer: u8,
    rng_state: u64,
}

impl GameState {
    /// Convert tile position to screen coordinates for particles.
    fn tile_to_screen(&self, tx: i32, ty: i32) -> (f64, f64) {
        let cam_x = self.player.x as f64 * 24.0 - self.renderer.canvas_w / 2.0 + 12.0;
        let cam_y = self.player.y as f64 * 24.0 - self.renderer.canvas_h / 2.0 + 12.0;
        (tx as f64 * 24.0 - cam_x + 12.0, ty as f64 * 24.0 - cam_y + 12.0)
    }

    fn rng_next(&mut self) -> u64 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        x
    }

    fn spawn_enemies(&mut self) {
        let pool = vocab::vocab_for_floor(self.floor_num);
        if pool.is_empty() {
            return;
        }
        let rooms = self.level.rooms.clone();
        let is_boss_floor = self.floor_num % 5 == 0 && self.floor_num > 0;
        let enemies_per_room = 1 + self.floor_num / 4; // more enemies on deeper floors

        for (i, room) in rooms.iter().enumerate() {
            if i == 0 || i == rooms.len() - 1 {
                continue;
            }
            // Boss in second-to-last room on boss floors
            if is_boss_floor && i == rooms.len() - 2 {
                let entry_idx = self.rng_next() as usize % pool.len();
                let entry: &'static VocabEntry = pool[entry_idx];
                let (cx, cy) = room.center();
                self.enemies.push(Enemy::boss_from_vocab(entry, cx, cy, self.floor_num));
                continue;
            }
            for _ in 0..enemies_per_room.min(ENEMIES_PER_ROOM as i32 + self.floor_num / 3) {
                let rand_val = self.rng_next();
                let entry_idx = self.srs.weighted_pick(&pool, rand_val);
                let entry: &'static VocabEntry = pool[entry_idx];
                let ex = room.x + 1 + (self.rng_next() % (room.w - 2).max(1) as u64) as i32;
                let ey = room.y + 1 + (self.rng_next() % (room.h - 2).max(1) as u64) as i32;
                if self.level.is_walkable(ex, ey) {
                    self.enemies.push(Enemy::from_vocab(entry, ex, ey, self.floor_num));
                }
            }
        }
    }

    fn new_floor(&mut self) {
        if let Some(ref audio) = self.audio { audio.play_descend(); }
        crate::srs::save_srs(&self.srs);
        self.codex.save();
        self.floor_num += 1;
        if self.floor_num > self.best_floor {
            self.best_floor = self.floor_num;
        }
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.rng_state = self.seed;
        self.level = DungeonLevel::generate(MAP_W, MAP_H, self.seed);
        let (sx, sy) = self.level.start_pos();
        self.player.move_to(sx, sy);
        self.enemies.clear();
        self.combat = CombatState::Explore;
        self.typing.clear();
        self.spawn_enemies();
        let (px, py) = (self.player.x, self.player.y);
        compute_fov(&mut self.level, px, py, FOV_RADIUS);
        self.achievements.check_floor(self.floor_num);
    }

    /// Check if an enemy occupies (x, y). Returns its index.
    fn enemy_at(&self, x: i32, y: i32) -> Option<usize> {
        self.enemies.iter().position(|e| e.is_alive() && e.x == x && e.y == y)
    }

    /// Get the room modifier at the player's current position.
    fn current_room_modifier(&self) -> Option<RoomModifier> {
        let px = self.player.x;
        let py = self.player.y;
        for room in &self.level.rooms {
            if px >= room.x && px < room.x + room.w && py >= room.y && py < room.y + room.h {
                return room.modifier;
            }
        }
        None
    }

    /// Effective FOV radius (reduced in Dark rooms).
    fn effective_fov(&self) -> i32 {
        let base = if self.current_room_modifier() == Some(RoomModifier::Dark) {
            2
        } else {
            FOV_RADIUS
        };
        base + self.player.enchant_fov_bonus()
    }

    /// Try to move player. Bumping into an enemy starts combat.
    fn try_move(&mut self, dx: i32, dy: i32) {
        if matches!(self.combat, CombatState::GameOver) {
            return;
        }
        // If fighting, ignore movement
        if matches!(self.combat, CombatState::Fighting { .. }) {
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
        if pdmg > 0 {
            self.player.hp -= pdmg;
            self.message = format!("☠ Poison deals {} damage!", pdmg);
            self.message_timer = 40;
            if self.player.hp <= 0 {
                self.player.hp = 0;
                if let Some(ref audio) = self.audio { audio.play_death(); }
                self.combat = CombatState::GameOver;
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
            for r in self.level.revealed.iter_mut() { *r = true; }
        }

        let (nx, ny) = self.player.intended_move(dx, dy);
        if !self.level.is_walkable(nx, ny) {
            return;
        }

        // Check for enemy bump → start combat
        if let Some(idx) = self.enemy_at(nx, ny) {
            self.combat = CombatState::Fighting {
                enemy_idx: idx,
                timer_ms: 0.0,
            };
            self.typing.clear();
            self.message = format!(
                "Type pinyin for {} ({})",
                self.enemies[idx].hanzi, self.enemies[idx].meaning
            );
            self.message_timer = 255;
            return;
        }

        self.player.move_to(nx, ny);
        if let Some(ref audio) = self.audio { audio.play_step(); }

        // Stairs
        if self.level.tile(nx, ny) == Tile::StairsDown {
            self.new_floor();
            return;
        }

        // Forge workbench
        if self.level.tile(nx, ny) == Tile::Forge {
            if self.player.radicals.is_empty() {
                self.message = "Forge workbench — but you have no radicals!".to_string();
                self.message_timer = 60;
            } else {
                self.combat = CombatState::Forging {
                    selected: Vec::new(),
                    page: 0,
                };
                self.message = "Select radicals with 1-9, ←/→ to page. Enter to forge. E to enchant.".to_string();
                self.message_timer = 255;
                let (px, py) = (self.player.x, self.player.y);
                compute_fov(&mut self.level, px, py, FOV_RADIUS);
                return;
            }
        }

        // Shop
        if self.level.tile(nx, ny) == Tile::Shop {
            let items = self.generate_shop_items();
            self.combat = CombatState::Shopping { items, cursor: 0 };
            self.message = "Welcome to the shop! ↑↓ to browse, Enter to buy, Esc to leave.".to_string();
            self.message_timer = 255;
            let (px, py) = (self.player.x, self.player.y);
            compute_fov(&mut self.level, px, py, FOV_RADIUS);
            return;
        }

        // Chest
        if self.level.tile(nx, ny) == Tile::Chest {
            self.open_chest(nx, ny);
        }

        // After player moves, enemies take a turn (skipped on even moves during haste)
        self.move_count += 1;
        let skip_enemy = status::has_haste(&self.player.statuses) && self.move_count % 2 == 0;
        if !skip_enemy {
            self.enemy_turn();
        }

        let (px, py) = (self.player.x, self.player.y);
        let fov = self.effective_fov();
        compute_fov(&mut self.level, px, py, fov);
    }

    /// All enemies take one step toward the player if alerted.
    fn enemy_turn(&mut self) {
        let px = self.player.x;
        let py = self.player.y;

        for i in 0..self.enemies.len() {
            if !self.enemies[i].is_alive() {
                continue;
            }
            // Stunned enemies skip their turn
            if self.enemies[i].stunned {
                self.enemies[i].stunned = false;
                continue;
            }
            // Alert if within FOV radius
            let dist_sq = (self.enemies[i].x - px).pow(2) + (self.enemies[i].y - py).pow(2);
            if dist_sq <= (FOV_RADIUS * FOV_RADIUS) {
                self.enemies[i].alert = true;
            }
            if !self.enemies[i].alert {
                continue;
            }

            let (nx, ny) = self.enemies[i].step_toward(px, py);

            // Don't walk into walls or other enemies
            if !self.level.is_walkable(nx, ny) {
                continue;
            }
            // Don't stack on other enemies
            let occupied = self.enemies.iter().enumerate().any(|(j, e)| {
                j != i && e.is_alive() && e.x == nx && e.y == ny
            });
            if occupied {
                continue;
            }

            // If enemy walks into player → start combat (same as player bumping enemy)
            if nx == px && ny == py {
                if !matches!(self.combat, CombatState::Fighting { .. }) {
                    // Shake on enemy engagement
                    self.shake_timer = 4;
                    self.combat = CombatState::Fighting {
                        enemy_idx: i,
                        timer_ms: 0.0,
                    };
                    self.typing.clear();
                    self.message = format!(
                        "{} attacks! Type pinyin for {} ({})",
                        self.enemies[i].hanzi, self.enemies[i].hanzi, self.enemies[i].meaning
                    );
                    self.message_timer = 255;
                }
                continue;
            }

            self.enemies[i].x = nx;
            self.enemies[i].y = ny;
        }
    }

    /// Handle typing a character during combat.
    fn type_char(&mut self, ch: char) {
        if matches!(self.combat, CombatState::GameOver) {
            return;
        }
        if let CombatState::Fighting { .. } = &self.combat {
            self.typing.push(ch);
        }
    }

    /// Submit pinyin answer.
    fn submit_answer(&mut self) {
        if let CombatState::Fighting { enemy_idx, .. } = self.combat.clone() {
            if enemy_idx >= self.enemies.len() {
                self.combat = CombatState::Explore;
                return;
            }
            let e_hanzi = self.enemies[enemy_idx].hanzi;
            let e_pinyin = self.enemies[enemy_idx].pinyin;
            let e_meaning = self.enemies[enemy_idx].meaning;
            let e_damage = (self.enemies[enemy_idx].damage - self.player.damage_reduction() - self.player.enchant_damage_reduction()).max(1);
            let e_gold = self.enemies[enemy_idx].gold_value + self.player.gold_bonus() + self.player.enchant_gold_bonus();
            let e_is_boss = self.enemies[enemy_idx].is_boss;
            let e_x = self.enemies[enemy_idx].x;
            let e_y = self.enemies[enemy_idx].y;

            if vocab::check_pinyin(
                &vocab::VocabEntry {
                    hanzi: e_hanzi,
                    pinyin: e_pinyin,
                    meaning: e_meaning,
                    hsk: 1,
                },
                &self.typing,
            ) {
                self.srs.record(e_hanzi, true);
                self.codex.record(e_hanzi, e_pinyin, e_meaning, true);
                // Hit with bonus damage from equipment + room modifiers
                let cursed_bonus = if self.current_room_modifier() == Some(RoomModifier::Cursed) { 1 } else { 0 };
                let hit_dmg = 2 + self.player.bonus_damage() + self.player.enchant_bonus_damage() + cursed_bonus;
                self.enemies[enemy_idx].hp -= hit_dmg;
                if self.enemies[enemy_idx].hp <= 0 {
                    self.total_kills += 1;
                    if let Some(ref audio) = self.audio { audio.play_kill(); }
                    // Kill particles
                    let (sx, sy) = self.tile_to_screen(e_x, e_y);
                    self.particles.spawn_kill(sx, sy, &mut self.rng_state);
                    self.flash = Some((255, 255, 255, 0.3));
                    // Rewards
                    self.player.gold += e_gold;
                    let available = radical::radicals_for_floor(self.floor_num);
                    let drop_idx = self.rng_next() as usize % available.len();
                    let dropped = available[drop_idx].ch;
                    self.player.add_radical(dropped);

                    // Elite enemies drop an extra radical
                    let e_is_elite = self.enemies[enemy_idx].is_elite;
                    if e_is_elite {
                        let drop2 = self.rng_next() as usize % available.len();
                        self.player.add_radical(available[drop2].ch);
                    }

                    // Extra radical from charm
                    let extra_chance = self.player.extra_radical_chance();
                    if extra_chance > 0 && (self.rng_next() % 100) < extra_chance as u64 {
                        let drop2 = self.rng_next() as usize % available.len();
                        self.player.add_radical(available[drop2].ch);
                    }

                    // Heal on kill from charm
                    let heal = self.player.heal_on_kill();
                    if heal > 0 {
                        self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                    }

                    // Random equipment drop (10% chance, higher for bosses)
                    let equip_chance = if e_is_boss { 60 } else { 10 };
                    if (self.rng_next() % 100) < equip_chance {
                        let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
                        let eq = &EQUIPMENT_POOL[eq_idx];
                        self.player.equip(eq);
                        self.message = format!(
                            "Defeated {}! +{}g [{}] + {}!",
                            e_hanzi, e_gold, dropped, eq.name
                        );
                    } else {
                        self.message = format!(
                            "Defeated {}! +{}g [{}]",
                            e_hanzi, e_gold, dropped
                        );
                    }
                    self.message_timer = 80;
                    // Tutorial hint: first radical collected
                    if self.total_runs == 0 && self.player.radicals.len() == 1 {
                        self.message = format!(
                            "Defeated {}! +{}g [{}] — Walk to an ⚒ anvil to forge spells!",
                            e_hanzi, e_gold, dropped
                        );
                        self.message_timer = 160;
                    }
                    self.combat = CombatState::Explore;

                    // Achievement checks
                    self.achievements.record_correct();
                    self.achievements.check_kills(self.total_kills);
                    self.achievements.check_gold(self.player.gold);
                    self.achievements.check_radicals(self.player.radicals.len());
                    if e_is_elite { self.achievements.unlock("first_elite"); }
                    if e_is_boss { self.achievements.unlock("first_boss"); }
                } else {
                    if let Some(ref audio) = self.audio { audio.play_hit(); }
                    self.message = format!("Hit for {}! {} HP left", hit_dmg, self.enemies[enemy_idx].hp);
                    self.message_timer = 40;
                }
            } else {
                // Miss — enemy counter-attacks
                self.srs.record(e_hanzi, false);
                self.codex.record(e_hanzi, e_pinyin, e_meaning, false);
                self.achievements.record_miss();
                if let Some(ref audio) = self.audio { audio.play_miss(); }
                if self.player.shield {
                    self.player.shield = false;
                    self.message = format!(
                        "Wrong! (was \"{}\") — Shield absorbed the blow!",
                        e_pinyin
                    );
                    self.message_timer = 60;
                } else {
                    self.player.hp -= e_damage;
                    if let Some(ref audio) = self.audio { audio.play_damage(); }
                    // Damage particles + shake
                    let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
                    self.particles.spawn_damage(sx, sy, &mut self.rng_state);
                    self.shake_timer = 8;
                    self.flash = Some((255, 50, 50, 0.25));
                    self.message = format!(
                        "Wrong! It was \"{}\". {} hits for {}!",
                        e_pinyin, e_hanzi, e_damage
                    );
                    self.message_timer = 60;
                }
                if self.player.hp <= 0 {
                    self.player.hp = 0;
                    self.combat = CombatState::GameOver;
                    if let Some(ref audio) = self.audio { audio.play_death(); }
                    self.save_high_score();
                }
            }
            self.typing.clear();
        }
    }

    /// Backspace during typing.
    fn backspace(&mut self) {
        self.typing.pop();
    }

    /// Toggle a radical index in forge selection.
    fn forge_toggle(&mut self, radical_idx: usize) {
        if let CombatState::Forging { ref mut selected, .. } = self.combat {
            if radical_idx >= self.player.radicals.len() {
                return;
            }
            if let Some(pos) = selected.iter().position(|&i| i == radical_idx) {
                selected.remove(pos);
            } else if selected.len() < 3 {
                selected.push(radical_idx);
            }
        }
    }

    /// Attempt to forge with selected radicals.
    fn forge_submit(&mut self) {
        let selected = if let CombatState::Forging { ref selected, .. } = self.combat {
            selected.clone()
        } else {
            return;
        };

        if selected.is_empty() {
            self.message = "Select radicals first!".to_string();
            self.message_timer = 40;
            return;
        }

        let rad_chars: Vec<&str> = selected
            .iter()
            .map(|&i| self.player.radicals[i])
            .collect();

        if let Some(recipe) = radical::try_forge(&rad_chars) {
            if let Some(ref audio) = self.audio { audio.play_forge(); }
            let spell = Spell {
                hanzi: recipe.output_hanzi,
                pinyin: recipe.output_pinyin,
                meaning: recipe.output_meaning,
                effect: recipe.effect,
            };
            // Track discovery
            let recipe_idx = radical::RECIPES
                .iter()
                .position(|r| r.output_hanzi == recipe.output_hanzi);
            if let Some(idx) = recipe_idx {
                if !self.discovered_recipes.contains(&idx) {
                    self.discovered_recipes.push(idx);
                }
            }
            self.message = format!(
                "Forged {} ({}) — {}! [{}]",
                recipe.output_hanzi,
                recipe.output_pinyin,
                recipe.output_meaning,
                recipe.effect.label()
            );
            self.message_timer = 80;
            self.player.add_spell(spell);
            // Tutorial hint: first spell forged
            if self.total_runs == 0 && self.player.spells.len() == 1 {
                self.message = format!(
                    "Forged {}! In combat: Tab to select spell, Space to cast!",
                    recipe.output_hanzi
                );
                self.message_timer = 160;
            }
            // Consume radicals (remove in reverse order to avoid index shifting)
            let mut to_remove: Vec<usize> = selected.clone();
            to_remove.sort_unstable_by(|a, b| b.cmp(a));
            for idx in to_remove {
                self.player.radicals.remove(idx);
            }
            self.combat = CombatState::Explore;
            self.achievements.check_recipes(self.discovered_recipes.len());
            self.achievements.check_spells(self.player.spells.len());
        } else {
            if let Some(ref audio) = self.audio { audio.play_forge_fail(); }
            self.message = "No recipe found for that combination...".to_string();
            self.message_timer = 60;
        }
    }

    /// Generate shop items for current floor.
    fn generate_shop_items(&mut self) -> Vec<ShopItem> {
        let mut items = Vec::new();

        // Always offer heal
        items.push(ShopItem {
            label: "Full Heal".to_string(),
            cost: 15 + self.floor_num * 3,
            kind: ShopItemKind::HealFull,
        });

        // Offer 2 random radicals
        let available = radical::radicals_for_floor(self.floor_num);
        for _ in 0..2 {
            let idx = self.rng_next() as usize % available.len();
            let rad = available[idx];
            items.push(ShopItem {
                label: format!("Radical [{}] ({})", rad.ch, rad.meaning),
                cost: 10 + self.floor_num,
                kind: ShopItemKind::Radical(rad.ch),
            });
        }

        // Offer 1 random equipment
        let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
        let eq = &EQUIPMENT_POOL[eq_idx];
        items.push(ShopItem {
            label: format!("{} ({:?})", eq.name, eq.slot),
            cost: 25 + self.floor_num * 5,
            kind: ShopItemKind::Equipment(eq_idx),
        });

        // Offer 1 random consumable item
        let consumable = self.random_item();
        let cname = consumable.name().to_string();
        items.push(ShopItem {
            label: cname,
            cost: 12 + self.floor_num * 2,
            kind: ShopItemKind::Consumable(consumable),
        });

        items
    }

    /// Buy item from shop.
    fn shop_buy(&mut self) {
        if let CombatState::Shopping { ref items, cursor } = self.combat.clone() {
            if cursor >= items.len() { return; }
            let item = &items[cursor];
            if self.player.gold < item.cost {
                self.message = format!("Not enough gold! Need {} (have {})", item.cost, self.player.gold);
                self.message_timer = 40;
                return;
            }
            self.player.gold -= item.cost;
            if let Some(ref audio) = self.audio { audio.play_buy(); }
            match &item.kind {
                ShopItemKind::Radical(ch) => {
                    self.player.add_radical(ch);
                    self.message = format!("Bought radical [{}]!", ch);
                }
                ShopItemKind::HealFull => {
                    self.player.hp = self.player.max_hp;
                    self.message = "Fully healed!".to_string();
                }
                ShopItemKind::Equipment(idx) => {
                    let eq = &EQUIPMENT_POOL[*idx];
                    self.player.equip(eq);
                    self.message = format!("Equipped {}!", eq.name);
                }
                ShopItemKind::Consumable(consumable) => {
                    let name = consumable.name().to_string();
                    if self.player.add_item(consumable.clone()) {
                        self.message = format!("Bought {}!", name);
                    } else {
                        self.message = "Item inventory full!".to_string();
                        self.player.gold += item.cost; // refund
                    }
                }
            }
            self.message_timer = 60;
        }
    }

    /// Use a spell during combat (Tab to cycle, Space to cast).
    fn use_spell(&mut self) {
        if let CombatState::Fighting { enemy_idx, .. } = self.combat {
            // Copy enemy position for particles before spell takes effect
            let e_screen = if enemy_idx < self.enemies.len() {
                Some(self.tile_to_screen(self.enemies[enemy_idx].x, self.enemies[enemy_idx].y))
            } else {
                None
            };
            let p_screen = self.tile_to_screen(self.player.x, self.player.y);

            if let Some(spell) = self.player.use_spell() {
                if let Some(ref audio) = self.audio { audio.play_spell(); }
                // Arcane room doubles spell damage
                let arcane_mult = if self.current_room_modifier() == Some(RoomModifier::Arcane) { 2 } else { 1 };
                match spell.effect {
                    SpellEffect::FireAoe(dmg) => {
                        let dmg = dmg * arcane_mult;
                        // Fire particles at player position (AoE emanates from player)
                        self.particles.spawn_fire(p_screen.0, p_screen.1, &mut self.rng_state);
                        // Damage all visible enemies
                        let mut killed = 0;
                        for e in &mut self.enemies {
                            if e.is_alive() {
                                let eidx = self.level.idx(e.x, e.y);
                                if self.level.visible[eidx] {
                                    e.hp -= dmg;
                                    if e.hp <= 0 { killed += 1; }
                                }
                            }
                        }
                        self.message = format!(
                            "{}🔥 {} deals {} damage to all! ({} defeated)",
                            spell.hanzi, spell.meaning, dmg, killed
                        );
                        self.message_timer = 80;
                        // If the fought enemy died, return to explore
                        if enemy_idx < self.enemies.len() && !self.enemies[enemy_idx].is_alive() {
                            self.combat = CombatState::Explore;
                            self.typing.clear();
                        }
                    }
                    SpellEffect::Heal(amount) => {
                        let amount = amount * arcane_mult;
                        self.player.hp = (self.player.hp + amount).min(self.player.max_hp);
                        self.particles.spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                        self.flash = Some((60, 220, 80, 0.2));
                        self.message = format!(
                            "{} heals {} HP! (now {}/{})",
                            spell.hanzi, amount, self.player.hp, self.player.max_hp
                        );
                        self.message_timer = 60;
                    }
                    SpellEffect::Shield => {
                        self.player.shield = true;
                        self.particles.spawn_shield(p_screen.0, p_screen.1, &mut self.rng_state);
                        self.message = format!(
                            "{} — Shield active! Next hit will be blocked.",
                            spell.hanzi
                        );
                        self.message_timer = 60;
                    }
                    SpellEffect::StrongHit(dmg) => {
                        let dmg = dmg * arcane_mult;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}⚔ Devastating {} damage! Defeated {}! Got [{}]",
                                    spell.hanzi, dmg, e_hanzi, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}⚔ {} damage to {}! ({} HP left)",
                                    spell.hanzi, dmg, e_hanzi, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Drain(dmg) => {
                        let dmg = dmg * arcane_mult;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_drain(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            self.player.hp = (self.player.hp + dmg).min(self.player.max_hp);
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}🩸 Drained {} HP from {}! Defeated! Got [{}]",
                                    spell.hanzi, dmg, e_hanzi, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}🩸 Drained {} HP from {}! +{} HP ({} left)",
                                    spell.hanzi, dmg, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Stun => {
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_stun(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].stunned = true;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            self.message = format!(
                                "{}⚡ {} is stunned! It will skip its next action.",
                                spell.hanzi, e_hanzi
                            );
                            self.message_timer = 60;
                        }
                    }
                }

                // ── Combo detection ─────────────────────────────────────
                let current_effect = spell.effect;
                if let Some(prev) = self.last_spell.take() {
                    let combo = detect_combo(&prev, &current_effect);
                    if let Some((combo_name, combo_effect)) = combo {
                        self.apply_combo(enemy_idx, &combo_name, combo_effect, p_screen, e_screen);
                    }
                }
                self.last_spell = Some(current_effect);
                self.spell_combo_timer = 3;
            } else {
                self.message = "No spells available!".to_string();
                self.message_timer = 40;
            }
        }
    }

    /// Apply a spell combo bonus.
    fn apply_combo(
        &mut self,
        enemy_idx: usize,
        name: &str,
        effect: ComboEffect,
        p_screen: (f64, f64),
        e_screen: Option<(f64, f64)>,
    ) {
        // Flash gold for combo
        self.flash = Some((255, 200, 50, 0.3));
        match effect {
            ComboEffect::Steam => {
                // AoE stun all visible enemies
                for e in &mut self.enemies {
                    if e.is_alive() {
                        e.stunned = true;
                    }
                }
                self.particles.spawn_fire(p_screen.0, p_screen.1 - 10.0, &mut self.rng_state);
                self.particles.spawn_shield(p_screen.0, p_screen.1 + 10.0, &mut self.rng_state);
                self.message = format!("💥 COMBO: {}! All enemies stunned!", name);
            }
            ComboEffect::Counter(dmg) => {
                // Reflect damage + shield
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].hp -= dmg;
                    self.player.shield = true;
                    if let Some((ex, ey)) = e_screen {
                        self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                    }
                }
                self.message = format!("💥 COMBO: {}! {} reflected + Shield!", name, dmg);
            }
            ComboEffect::Barrier(amount) => {
                // Strong shield + heal
                self.player.shield = true;
                self.player.hp = (self.player.hp + amount).min(self.player.max_hp);
                self.particles.spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                self.particles.spawn_shield(p_screen.0, p_screen.1, &mut self.rng_state);
                self.message = format!("💥 COMBO: {}! Shield + {} HP!", name, amount);
            }
            ComboEffect::Flurry(dmg) => {
                // Triple hit
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].hp -= dmg;
                    if let Some((ex, ey)) = e_screen {
                        self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                        self.particles.spawn_fire(ex, ey, &mut self.rng_state);
                    }
                }
                self.message = format!("💥 COMBO: {}! {} damage flurry!", name, dmg);
            }
        }
        self.message_timer = 100;
        // Check if fought enemy died from combo
        if enemy_idx < self.enemies.len() && !self.enemies[enemy_idx].is_alive() {
            self.combat = CombatState::Explore;
            self.typing.clear();
        }
    }

    /// Open a treasure chest tile.
    fn open_chest(&mut self, cx: i32, cy: i32) {
        // Chest open particles
        let (sx, sy) = self.tile_to_screen(cx, cy);
        self.particles.spawn_chest(sx, sy, &mut self.rng_state);
        self.achievements.unlock("first_chest");

        // Remove chest tile
        let idx = self.level.idx(cx, cy);
        self.level.tiles[idx] = Tile::Floor;

        let roll = self.rng_next() % 100;
        if roll < 70 {
            // 70% — loot (item, gold, or radical)
            let loot_type = self.rng_next() % 3;
            match loot_type {
                0 => {
                    // Random item
                    let item = self.random_item();
                    let name = item.name().to_string();
                    if self.player.add_item(item) {
                        self.message = format!("◆ Found {}!", name);
                        self.achievements.check_items(self.player.items.len());
                    } else {
                        self.message = "◆ Chest had an item but inventory is full!".to_string();
                    }
                }
                1 => {
                    // Gold
                    let gold = 10 + (self.rng_next() % 20) as i32 + self.floor_num * 3;
                    self.player.gold += gold;
                    self.message = format!("◆ Found {}g!", gold);
                }
                _ => {
                    // Radical
                    let available = radical::radicals_for_floor(self.floor_num);
                    let drop_idx = self.rng_next() as usize % available.len();
                    let dropped = available[drop_idx].ch;
                    self.player.add_radical(dropped);
                    self.message = format!("◆ Found radical [{}]!", dropped);
                }
            }
            self.message_timer = 60;
        } else if roll < 90 {
            // 20% — trap
            let trap_type = self.rng_next() % 2;
            if trap_type == 0 {
                // Poison trap
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Poison { damage: 1 }, 5
                ));
                self.message = "◆ Trapped! Poisoned for 5 turns!".to_string();
            } else {
                // Damage trap
                let dmg = 2 + self.floor_num / 2;
                self.player.hp -= dmg;
                if self.player.hp <= 0 {
                    self.player.hp = 0;
                    if let Some(ref audio) = self.audio { audio.play_death(); }
                    self.combat = CombatState::GameOver;
                }
                self.message = format!("◆ Trapped! Spike trap deals {} damage!", dmg);
            }
            self.message_timer = 60;
        } else {
            // 10% — mimic! Spawn an enemy here
            let pool = vocab::vocab_for_floor(self.floor_num);
            if !pool.is_empty() {
                let entry_idx = self.rng_next() as usize % pool.len();
                let entry: &'static VocabEntry = pool[entry_idx];
                let mut mimic = Enemy::from_vocab(entry, cx, cy, self.floor_num);
                mimic.hp = mimic.hp + 2; // mimics are tougher
                mimic.damage += 1;
                mimic.alert = true;
                mimic.gold_value *= 2; // better drops
                self.enemies.push(mimic);
                // Immediately start combat
                let idx = self.enemies.len() - 1;
                self.combat = CombatState::Fighting {
                    enemy_idx: idx,
                    timer_ms: 0.0,
                };
                self.typing.clear();
                self.message = format!("◆ It's a Mimic! Type pinyin for {} ({})", entry.hanzi, entry.meaning);
                self.message_timer = 255;
            }
        }
    }

    /// Generate a random item appropriate for the current floor.
    fn random_item(&mut self) -> crate::player::Item {
        use crate::player::Item;
        match self.rng_next() % 6 {
            0 => Item::HealthPotion(4 + self.floor_num),
            1 => Item::PoisonFlask(2, 3),
            2 => Item::RevealScroll,
            3 => Item::TeleportScroll,
            4 => Item::HastePotion(5),
            _ => Item::StunBomb,
        }
    }

    /// Use a consumable item from inventory.
    fn use_item(&mut self, idx: usize) {
        if idx >= self.player.items.len() {
            return;
        }
        let item = self.player.items.remove(idx);
        if let Some(ref audio) = self.audio { audio.play_spell(); }

        match item {
            crate::player::Item::HealthPotion(heal) => {
                self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                self.message = format!("💚 Healed {} HP! ({}/{})", heal, self.player.hp, self.player.max_hp);
                self.message_timer = 60;
            }
            crate::player::Item::PoisonFlask(dmg, turns) => {
                let px = self.player.x;
                let py = self.player.y;
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() && (e.x - px).abs() <= 1 && (e.y - py).abs() <= 1 {
                        e.statuses.push(status::StatusInstance::new(
                            status::StatusKind::Poison { damage: dmg }, turns
                        ));
                        count += 1;
                    }
                }
                self.message = format!("☠ Poisoned {} enemies! ({} dmg × {} turns)", count, dmg, turns);
                self.message_timer = 60;
            }
            crate::player::Item::RevealScroll => {
                for r in self.level.revealed.iter_mut() { *r = true; }
                self.message = "👁 Map revealed!".to_string();
                self.message_timer = 60;
            }
            crate::player::Item::TeleportScroll => {
                // Find random explored walkable tile
                let mut candidates = Vec::new();
                for y in 0..self.level.height {
                    for x in 0..self.level.width {
                        let i = self.level.idx(x, y);
                        if self.level.revealed[i] && self.level.tiles[i].is_walkable()
                            && self.enemy_at(x, y).is_none()
                            && (x != self.player.x || y != self.player.y) {
                            candidates.push((x, y));
                        }
                    }
                }
                if let Some(&(tx, ty)) = candidates.get(self.rng_next() as usize % candidates.len().max(1)) {
                    self.player.move_to(tx, ty);
                    let (px, py) = (self.player.x, self.player.y);
                    compute_fov(&mut self.level, px, py, FOV_RADIUS);
                    self.message = "✦ Teleported!".to_string();
                } else {
                    self.message = "Teleport fizzled — nowhere to go!".to_string();
                }
                self.message_timer = 60;
            }
            crate::player::Item::HastePotion(turns) => {
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Haste, turns
                ));
                self.message = format!("⚡ Haste for {} turns! Enemies move at half speed.", turns);
                self.message_timer = 60;
            }
            crate::player::Item::StunBomb => {
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() {
                        let i = self.level.idx(e.x, e.y);
                        if self.level.visible[i] {
                            e.stunned = true;
                            count += 1;
                        }
                    }
                }
                self.message = format!("💥 Stunned {} enemies!", count);
                self.message_timer = 60;
            }
        }
    }

    /// Restart after game over.
    fn restart(&mut self) {
        self.total_runs += 1;
        self.save_high_score();
        self.save_stats();
        self.srs = crate::srs::load_srs();
        self.player = Player::new(0, 0);
        self.floor_num = 0;
        self.enemies.clear();
        self.typing.clear();
        // Keep discovered recipes across runs (loaded from localStorage)
        self.combat = CombatState::Explore;
        self.new_floor();
    }

    fn save_high_score(&self) {
        crate::srs::save_srs(&self.srs);
        self.save_stats();
        let storage: Option<web_sys::Storage> = window()
            .and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
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
            let recipe_str: String = self.discovered_recipes.iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let _ = storage.set_item("radical_roguelike_recipes", &recipe_str);
        }
    }

    fn load_high_score() -> i32 {
        let storage: Option<web_sys::Storage> = window()
            .and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        storage
            .and_then(|s: web_sys::Storage| s.get_item("radical_roguelike_best").ok().flatten())
            .and_then(|s: String| s.parse::<i32>().ok())
            .unwrap_or(0)
    }

    fn load_recipes() -> Vec<usize> {
        let storage: Option<web_sys::Storage> = window()
            .and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
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

    fn load_stat(key: &str) -> u32 {
        let storage: Option<web_sys::Storage> = window()
            .and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        storage
            .and_then(|s: web_sys::Storage| s.get_item(key).ok().flatten())
            .and_then(|s: String| s.parse::<u32>().ok())
            .unwrap_or(0)
    }

    fn save_stats(&self) {
        let storage: Option<web_sys::Storage> = window()
            .and_then(|w: web_sys::Window| w.local_storage().ok().flatten());
        if let Some(storage) = storage {
            let _ = storage.set_item("radical_roguelike_runs", &self.total_runs.to_string());
            let _ = storage.set_item("radical_roguelike_kills", &self.total_kills.to_string());
        }
    }

    fn tick_message(&mut self) {
        if self.message_timer > 0 {
            self.message_timer -= 1;
            if self.message_timer == 0 {
                self.message.clear();
            }
        }
    }

    fn render(&self) {
        let popup = self.achievement_popup.map(|(n, d, _)| (n, d));
        let room_mod = self.current_room_modifier();
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
            self.shake_timer,
            self.flash,
            popup,
            room_mod,
        );
        if self.show_codex {
            let entries = self.codex.sorted_entries();
            self.renderer.draw_codex(&entries);
        }
    }
}

/// Combo effects from spell combinations.
enum ComboEffect {
    Steam,         // Fire + Shield: AoE stun
    Counter(i32),  // Shield + Strike: reflect damage
    Barrier(i32),  // Heal + Shield: shield + heal
    Flurry(i32),   // Strike + Fire: triple damage
}

/// Detect if two spell effects form a combo.
fn detect_combo(prev: &SpellEffect, current: &SpellEffect) -> Option<(&'static str, ComboEffect)> {
    match (spell_category(prev), spell_category(current)) {
        ("fire", "shield") | ("shield", "fire") => Some(("Steam Burst", ComboEffect::Steam)),
        ("shield", "strike") | ("strike", "shield") => Some(("Counter Strike", ComboEffect::Counter(6))),
        ("heal", "shield") | ("shield", "heal") => Some(("Barrier", ComboEffect::Barrier(4))),
        ("strike", "fire") | ("fire", "strike") => Some(("Flurry", ComboEffect::Flurry(8))),
        ("drain", "heal") | ("heal", "drain") => Some(("Life Surge", ComboEffect::Barrier(6))),
        ("stun", "strike") | ("strike", "stun") => Some(("Crippling Blow", ComboEffect::Flurry(10))),
        _ => None,
    }
}

fn spell_category(effect: &SpellEffect) -> &'static str {
    match effect {
        SpellEffect::FireAoe(_) => "fire",
        SpellEffect::Heal(_) => "heal",
        SpellEffect::Shield => "shield",
        SpellEffect::StrongHit(_) => "strike",
        SpellEffect::Drain(_) => "drain",
        SpellEffect::Stun => "stun",
    }
}

pub fn init_game() -> Result<(), JsValue> {
    let win = window().ok_or("no window")?;
    let doc = win.document().ok_or("no document")?;

    // Create canvas
    let canvas: HtmlCanvasElement = doc.create_element("canvas")?.dyn_into()?;
    canvas.set_id("game-canvas");
    canvas.set_width(800);
    canvas.set_height(600);
    canvas.set_attribute(
        "style",
        "display:block; margin:0 auto; background:#0d0b14; image-rendering:pixelated;",
    )?;
    doc.body().unwrap().append_child(&canvas)?;

    // Remove loading indicator
    if let Some(el) = doc.get_element_by_id("loading") {
        el.remove();
    }

    let renderer = Renderer::new(canvas).map_err(|e| JsValue::from_str(e))?;

    let seed = win.performance().map(|p| p.now() as u64).unwrap_or(42);
    let level = DungeonLevel::generate(MAP_W, MAP_H, seed);
    let (sx, sy) = level.start_pos();
    let player = Player::new(sx, sy);

    let best_floor = GameState::load_high_score();
    let srs = crate::srs::load_srs();
    let audio = Audio::new();
    let total_runs = GameState::load_stat("radical_roguelike_runs");
    let total_kills = GameState::load_stat("radical_roguelike_kills");

    let state = Rc::new(RefCell::new(GameState {
        level,
        player,
        renderer,
        audio,
        floor_num: 1,
        seed,
        enemies: Vec::new(),
        combat: CombatState::Explore,
        typing: String::new(),
        message: String::new(),
        message_timer: 0,
        discovered_recipes: GameState::load_recipes(),
        best_floor,
        srs,
        total_kills,
        total_runs,
        move_count: 0,
        particles: ParticleSystem::new(),
        shake_timer: 0,
        flash: None,
        achievements: AchievementTracker::load(),
        achievement_popup: None,
        codex: Codex::load(&vocab::VOCAB),
        show_codex: false,
        last_spell: None,
        spell_combo_timer: 0,
        rng_state: seed,
    }));

    // Initial setup
    {
        let mut s = state.borrow_mut();
        s.spawn_enemies();
        let (px, py) = (s.player.x, s.player.y);
        compute_fov(&mut s.level, px, py, FOV_RADIUS);
        // Tutorial hint on first run
        if s.total_runs == 0 && s.best_floor == 0 {
            s.message = "Welcome! Arrow keys to move. Bump enemies to fight — type their pinyin!".to_string();
            s.message_timer = 200;
        }
    }

    // Keyboard input
    {
        let state = Rc::clone(&state);
        let closure = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            let key = event.key();
            let mut s = state.borrow_mut();

            // Resume audio context on first interaction (browser requirement)
            if let Some(ref audio) = s.audio { audio.resume(); }

            // Game over: press R to restart
            if matches!(s.combat, CombatState::GameOver) {
                if key == "r" || key == "R" {
                    s.restart();
                    s.render();
                }
                return;
            }

            // Combat typing mode
            if matches!(s.combat, CombatState::Fighting { .. }) {
                event.prevent_default();
                match key.as_str() {
                    "Enter" => {
                        s.submit_answer();
                        s.tick_message();
                        s.render();
                    }
                    "Backspace" => {
                        s.backspace();
                        s.render();
                    }
                    "Escape" => {
                        // Flee — enemy gets a free hit (shield can block)
                        if let CombatState::Fighting { enemy_idx, .. } = s.combat {
                            if enemy_idx < s.enemies.len() && s.enemies[enemy_idx].is_alive() {
                                if s.player.shield {
                                    s.player.shield = false;
                                    s.message = "Fled! Shield absorbed the blow!".to_string();
                                    s.message_timer = 40;
                                } else {
                                    let dmg = s.enemies[enemy_idx].damage;
                                    s.player.hp -= dmg;
                                    s.message = format!("Fled! {} hits for {}!", s.enemies[enemy_idx].hanzi, dmg);
                                    s.message_timer = 40;
                                }
                                if s.player.hp <= 0 {
                                    s.player.hp = 0;
                                    s.combat = CombatState::GameOver;
                                } else {
                                    s.combat = CombatState::Explore;
                                }
                            } else {
                                s.combat = CombatState::Explore;
                            }
                        }
                        s.typing.clear();
                        s.render();
                    }
                    "Tab" => {
                        // Cycle selected spell
                        s.player.cycle_spell();
                        s.render();
                    }
                    " " => {
                        // Cast selected spell
                        s.use_spell();
                        s.render();
                    }
                    _ => {
                        if let Some(ch) = key.chars().next() {
                            if key.len() == 1 && (ch.is_ascii_alphanumeric()) {
                                s.type_char(ch);
                                s.render();
                            }
                        }
                    }
                }
                return;
            }

            // Forge mode
            if matches!(s.combat, CombatState::Forging { .. }) {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message.clear();
                        s.message_timer = 0;
                        s.render();
                    }
                    "Enter" => {
                        s.forge_submit();
                        s.render();
                    }
                    "ArrowLeft" => {
                        if let CombatState::Forging { ref mut page, .. } = s.combat {
                            if *page > 0 { *page -= 1; }
                        }
                        s.render();
                    }
                    "ArrowRight" => {
                        let max_page = s.player.radicals.len().saturating_sub(1) / 9;
                        if let CombatState::Forging { ref mut page, .. } = s.combat {
                            if *page < max_page { *page += 1; }
                        }
                        s.render();
                    }
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                        let slot = key.parse::<usize>().unwrap_or(1) - 1;
                        let page = if let CombatState::Forging { page, .. } = s.combat { page } else { 0 };
                        let idx = page * 9 + slot;
                        s.forge_toggle(idx);
                        s.render();
                    }
                    "e" | "E" => {
                        // Enter enchant mode — pick a slot first
                        let has_equip = s.player.weapon.is_some() || s.player.armor.is_some() || s.player.charm.is_some();
                        if !has_equip {
                            s.message = "No equipment to enchant!".to_string();
                            s.message_timer = 90;
                        } else if s.player.radicals.is_empty() {
                            s.message = "No radicals to enchant with!".to_string();
                            s.message_timer = 90;
                        } else {
                            s.combat = CombatState::Enchanting { slot: 0, page: 0 };
                            s.message = "Enchant: 1=Weapon 2=Armor 3=Charm. Pick slot, then radical.".to_string();
                            s.message_timer = 255;
                        }
                        s.render();
                    }
                    _ => {}
                }
                return;
            }

            // Enchanting mode
            if matches!(s.combat, CombatState::Enchanting { .. }) {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message.clear();
                        s.message_timer = 0;
                        s.render();
                    }
                    "1" | "2" | "3" => {
                        let slot_idx = key.parse::<usize>().unwrap_or(1) - 1;
                        let has_slot = match slot_idx {
                            0 => s.player.weapon.is_some(),
                            1 => s.player.armor.is_some(),
                            2 => s.player.charm.is_some(),
                            _ => false,
                        };
                        if has_slot {
                            if let CombatState::Enchanting { ref mut slot, .. } = s.combat {
                                *slot = slot_idx;
                            }
                            let slot_name = match slot_idx { 0 => "Weapon", 1 => "Armor", _ => "Charm" };
                            s.message = format!("Enchanting {}. Pick radical 4-9 or ←/→ to page.", slot_name);
                            s.message_timer = 255;
                        } else {
                            let slot_name = match slot_idx { 0 => "Weapon", 1 => "Armor", _ => "Charm" };
                            s.message = format!("No {} equipped!", slot_name);
                            s.message_timer = 90;
                        }
                        s.render();
                    }
                    "ArrowLeft" => {
                        if let CombatState::Enchanting { ref mut page, .. } = s.combat {
                            if *page > 0 { *page -= 1; }
                        }
                        s.render();
                    }
                    "ArrowRight" => {
                        let max_page = s.player.radicals.len().saturating_sub(1) / 9;
                        if let CombatState::Enchanting { ref mut page, .. } = s.combat {
                            if *page < max_page { *page += 1; }
                        }
                        s.render();
                    }
                    "4" | "5" | "6" | "7" | "8" | "9" => {
                        let rad_slot = key.parse::<usize>().unwrap_or(4) - 1;
                        let page = if let CombatState::Enchanting { page, .. } = s.combat { page } else { 0 };
                        let idx = page * 9 + rad_slot;
                        let slot = if let CombatState::Enchanting { slot, .. } = s.combat { slot } else { 0 };
                        if idx < s.player.radicals.len() {
                            let radical = s.player.radicals[idx];
                            s.player.enchantments[slot] = Some(radical);
                            // Consume the radical
                            s.player.radicals.remove(idx);
                            let slot_name = match slot { 0 => "Weapon", 1 => "Armor", _ => "Charm" };
                            let bonus = match radical {
                                "力" | "火" => "+1 damage",
                                "水" | "土" => "+1 defense",
                                "心" => "+2 max HP",
                                "金" => "+3 gold/kill",
                                "目" => "+1 FOV",
                                _ => "+1 damage",
                            };
                            // Apply max HP bonus immediately
                            if radical == "心" {
                                s.player.max_hp += 2;
                                s.player.hp = s.player.hp.min(s.player.max_hp);
                            }
                            if let Some(ref audio) = s.audio { audio.play_forge(); }
                            let cam_x = s.player.x as f64 * 24.0 - s.renderer.canvas_w / 2.0 + 12.0;
                            let cam_y = s.player.y as f64 * 24.0 - s.renderer.canvas_h / 2.0 + 12.0;
                            let sx = s.player.x as f64 * 24.0 - cam_x + 12.0;
                            let sy = s.player.y as f64 * 24.0 - cam_y + 12.0;
                            let gs = &mut *s;
                            gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                            s.message = format!("Enchanted {} with {} ({})!", slot_name, radical, bonus);
                            s.message_timer = 120;
                            s.combat = CombatState::Explore;
                            let recipe_count = s.discovered_recipes.len();
                            s.achievements.check_recipes(recipe_count);
                        } else {
                            s.message = "No radical at that slot.".to_string();
                            s.message_timer = 60;
                        }
                        s.render();
                    }
                    _ => {}
                }
                return;
            }

            // Shop mode
            if matches!(s.combat, CombatState::Shopping { .. }) {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message.clear();
                        s.message_timer = 0;
                        s.render();
                    }
                    "ArrowUp" | "w" | "W" => {
                        if let CombatState::Shopping { ref items, ref mut cursor } = s.combat {
                            if *cursor > 0 { *cursor -= 1; }
                            let _ = items;
                        }
                        s.render();
                    }
                    "ArrowDown" | "s" | "S" => {
                        if let CombatState::Shopping { ref items, ref mut cursor } = s.combat {
                            if *cursor + 1 < items.len() { *cursor += 1; }
                        }
                        s.render();
                    }
                    "Enter" => {
                        s.shop_buy();
                        s.render();
                    }
                    _ => {}
                }
                return;
            }

            // Exploration movement + item usage
            // Toggle codex with 'c'
            if key == "c" || key == "C" {
                s.show_codex = !s.show_codex;
                s.render();
                return;
            }
            // Close codex on Escape
            if s.show_codex {
                if key == "Escape" {
                    s.show_codex = false;
                    s.render();
                }
                return;
            }
            match key.as_str() {
                "1" | "2" | "3" | "4" | "5" => {
                    let idx = key.parse::<usize>().unwrap_or(1) - 1;
                    s.use_item(idx);
                    s.tick_message();
                    s.render();
                    return;
                }
                _ => {}
            }
            let (dx, dy) = match key.as_str() {
                "ArrowUp" | "w" | "W" => (0, -1),
                "ArrowDown" | "s" | "S" => (0, 1),
                "ArrowLeft" | "a" | "A" => (-1, 0),
                "ArrowRight" | "d" | "D" => (1, 0),
                _ => return,
            };
            event.prevent_default();
            s.try_move(dx, dy);
            s.tick_message();
            s.render();
        });
        doc.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Initial render
    state.borrow().render();

    // Animation loop for particles, screen shake, and flash effects
    {
        let state = Rc::clone(&state);
        let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
        let g = Rc::clone(&f);
        *g.borrow_mut() = Some(Closure::new(move || {
            {
                let mut s = state.borrow_mut();
                // Tick music
                let mood = match s.combat {
                    CombatState::Fighting { enemy_idx, .. } => {
                        if enemy_idx < s.enemies.len() && s.enemies[enemy_idx].is_boss {
                            crate::audio::MusicMood::Boss
                        } else {
                            crate::audio::MusicMood::Combat
                        }
                    }
                    CombatState::GameOver => crate::audio::MusicMood::Silent,
                    _ => crate::audio::MusicMood::Explore,
                };
                if let Some(ref mut audio) = s.audio {
                    audio.set_mood(mood);
                    audio.tick_music();
                }

                // Tick achievement popup
                if s.achievement_popup.is_none() {
                    if let Some(id) = s.achievements.pop_popup() {
                        if let Some(def) = AchievementTracker::get_def(id) {
                            s.achievement_popup = Some((def.name, def.desc, 180)); // ~3 seconds at 60fps
                        }
                    }
                }
                if let Some((_, _, ref mut timer)) = s.achievement_popup {
                    if *timer > 0 {
                        *timer -= 1;
                    } else {
                        s.achievement_popup = None;
                    }
                }

                let has_popup = s.achievement_popup.is_some();
                let needs_anim = s.particles.is_active() || s.shake_timer > 0 || s.flash.is_some() || has_popup;
                if needs_anim {
                    s.particles.tick();
                    if s.shake_timer > 0 {
                        s.shake_timer -= 1;
                    }
                    if let Some((_, _, _, ref mut a)) = s.flash {
                        *a -= 0.05;
                        if *a <= 0.0 {
                            s.flash = None;
                        }
                    }
                    // Render is called inside the borrow_mut scope
                    s.render();
                }
            }
            // Schedule next frame
            if let Some(win) = window() {
                let _ = win.request_animation_frame(
                    f.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
                );
            }
        }));
        let win = window().ok_or("no window")?;
        let _ = win.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }

    Ok(())
}
