//! Main game state and loop.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement, KeyboardEvent};

use crate::dungeon::{compute_fov, DungeonLevel, Tile};
use crate::enemy::Enemy;
use crate::player::Player;
use crate::render::Renderer;
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
    /// Player is dead
    GameOver,
}

pub struct GameState {
    pub level: DungeonLevel,
    pub player: Player,
    pub renderer: Renderer,
    pub floor_num: i32,
    pub seed: u64,
    pub enemies: Vec<Enemy>,
    pub combat: CombatState,
    pub typing: String,
    pub message: String,
    pub message_timer: u8, // frames to show message
    rng_state: u64,
}

impl GameState {
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
        // Skip first room (player start) and last room (stairs)
        let rooms = self.level.rooms.clone();
        for (i, room) in rooms.iter().enumerate() {
            if i == 0 || i == rooms.len() - 1 {
                continue;
            }
            for _ in 0..ENEMIES_PER_ROOM {
                let entry_idx = self.rng_next() as usize % pool.len();
                let entry: &'static VocabEntry = pool[entry_idx];
                // Random position inside room
                let ex = room.x + 1 + (self.rng_next() % (room.w - 2).max(1) as u64) as i32;
                let ey = room.y + 1 + (self.rng_next() % (room.h - 2).max(1) as u64) as i32;
                if self.level.is_walkable(ex, ey) {
                    self.enemies.push(Enemy::from_vocab(entry, ex, ey));
                }
            }
        }
    }

    fn new_floor(&mut self) {
        self.floor_num += 1;
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
    }

    /// Check if an enemy occupies (x, y). Returns its index.
    fn enemy_at(&self, x: i32, y: i32) -> Option<usize> {
        self.enemies.iter().position(|e| e.is_alive() && e.x == x && e.y == y)
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

        // Stairs
        if self.level.tile(nx, ny) == Tile::StairsDown {
            self.new_floor();
            return;
        }

        // After player moves, enemies take a turn
        self.enemy_turn();

        let (px, py) = (self.player.x, self.player.y);
        compute_fov(&mut self.level, px, py, FOV_RADIUS);
    }

    /// All enemies take one step toward the player if alerted.
    fn enemy_turn(&mut self) {
        let px = self.player.x;
        let py = self.player.y;

        for i in 0..self.enemies.len() {
            if !self.enemies[i].is_alive() {
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
            // Copy enemy data to avoid borrow conflict
            let e_hanzi = self.enemies[enemy_idx].hanzi;
            let e_pinyin = self.enemies[enemy_idx].pinyin;
            let e_meaning = self.enemies[enemy_idx].meaning;
            let e_damage = self.enemies[enemy_idx].damage;

            if vocab::check_pinyin(
                &vocab::VocabEntry {
                    hanzi: e_hanzi,
                    pinyin: e_pinyin,
                    meaning: e_meaning,
                    hsk: 1,
                },
                &self.typing,
            ) {
                // Hit!
                self.enemies[enemy_idx].hp -= 2;
                if self.enemies[enemy_idx].hp <= 0 {
                    self.message = format!("Defeated {}! ({})", e_hanzi, e_meaning);
                    self.message_timer = 60;
                    self.combat = CombatState::Explore;
                } else {
                    self.message = format!("Hit! {} HP left", self.enemies[enemy_idx].hp);
                    self.message_timer = 40;
                }
            } else {
                // Miss — enemy counter-attacks
                self.player.hp -= e_damage;
                self.message = format!(
                    "Wrong! It was \"{}\". {} hits for {}!",
                    e_pinyin, e_hanzi, e_damage
                );
                self.message_timer = 60;
                if self.player.hp <= 0 {
                    self.player.hp = 0;
                    self.combat = CombatState::GameOver;
                }
            }
            self.typing.clear();
        }
    }

    /// Backspace during typing.
    fn backspace(&mut self) {
        self.typing.pop();
    }

    /// Restart after game over.
    fn restart(&mut self) {
        self.player.hp = self.player.max_hp;
        self.floor_num = 0;
        self.enemies.clear();
        self.typing.clear();
        self.combat = CombatState::Explore;
        self.new_floor();
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
        self.renderer.draw(
            &self.level,
            &self.player,
            &self.enemies,
            &self.combat,
            &self.typing,
            &self.message,
            self.floor_num,
        );
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

    let state = Rc::new(RefCell::new(GameState {
        level,
        player,
        renderer,
        floor_num: 1,
        seed,
        enemies: Vec::new(),
        combat: CombatState::Explore,
        typing: String::new(),
        message: String::new(),
        message_timer: 0,
        rng_state: seed,
    }));

    // Initial setup
    {
        let mut s = state.borrow_mut();
        s.spawn_enemies();
        let (px, py) = (s.player.x, s.player.y);
        compute_fov(&mut s.level, px, py, FOV_RADIUS);
    }

    // Keyboard input
    {
        let state = Rc::clone(&state);
        let closure = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            let key = event.key();
            let mut s = state.borrow_mut();

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
                        // Flee — enemy gets a free hit
                        if let CombatState::Fighting { enemy_idx, .. } = s.combat {
                            if enemy_idx < s.enemies.len() && s.enemies[enemy_idx].is_alive() {
                                let dmg = s.enemies[enemy_idx].damage;
                                s.player.hp -= dmg;
                                s.message = format!("Fled! {} hits for {}!", s.enemies[enemy_idx].hanzi, dmg);
                                s.message_timer = 40;
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

            // Exploration movement
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

    Ok(())
}
