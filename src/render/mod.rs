//! Canvas 2D rendering for the station.

pub mod combat_hud;
pub mod helpers;
pub mod starmap;
pub mod tiles;
pub mod ui;
mod overlays;

use js_sys::Date;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::world::{DungeonLevel, Tile};
use crate::enemy::{Enemy};
use crate::game::{combo_tier, CombatState, ComboTier, GameSettings, ListenMode};
use crate::particle::ParticleSystem;
use crate::player::{Faction, Player, PlayerForm};
use crate::radical;
use crate::sprites::SpriteCache;

use tiles::{tile_palette, tile_sprite_key, should_tile_sprite, is_wall_tile};
use helpers::{
    boss_sprite_key, enemy_sprite_for_location, hud_message_color, item_sprite_key,
    shop_item_sprite_key, spell_sprite_key,
};

const TILE_SIZE: f64 = 24.0;

// Colors
const COL_WALL: &str = "#1a1a2a";
const COL_WALL_REVEALED: &str = "#1a1428";
const COL_FLOOR: &str = "#2a2a3a";
const COL_FLOOR_REVEALED: &str = "#252535";
const COL_CORRIDOR: &str = "#2a3344";
const COL_CORRIDOR_REVEALED: &str = "#272040";
const COL_STAIRS: &str = "#8ab4ff";
const COL_FORGE: &str = "#ff8844";
const COL_SHOP: &str = "#44dd88";
const COL_CHEST: &str = "#ddaa33";
const COL_FOG: &str = "#060612";
const COL_PLAYER: &str = "#00ccdd";
const COL_PLAYER_OUTLINE: &str = "#008899";
const COL_HP_BAR: &str = "#44cc55";
const COL_HP_BG: &str = "#442222";

pub struct Renderer {
    #[allow(dead_code)]
    pub canvas: HtmlCanvasElement,
    pub ctx: CanvasRenderingContext2d,
    pub canvas_w: f64,
    pub canvas_h: f64,
    sprites: SpriteCache,
}


impl Renderer {
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, &'static str> {
        let ctx: CanvasRenderingContext2d = canvas
            .get_context("2d")
            .map_err(|_| "get_context failed")?
            .ok_or("no 2d context")?
            .dyn_into()
            .map_err(|_| "not a CanvasRenderingContext2d")?;
        let canvas_w = canvas.width() as f64;
        let canvas_h = canvas.height() as f64;
        Ok(Self {
            canvas,
            ctx,
            canvas_w,
            canvas_h,
            sprites: SpriteCache::new(),
        })
    }

    /// Update cached dimensions after canvas resize.
    pub fn sync_size(&mut self) {
        self.canvas_w = self.canvas.width() as f64;
        self.canvas_h = self.canvas.height() as f64;
    }

    fn draw_sprite_icon(&self, key: &str, x: f64, y: f64, size: f64) -> bool {
        if self.sprites.is_loaded(key) {
            if let Some(img) = self.sprites.get(key) {
                self.ctx
                    .draw_image_with_html_image_element_and_dw_and_dh(img, x, y, size, size)
                    .ok();
                return true;
            }
        }
        false
    }

    /// Render the full game frame.
    pub fn draw(
        &self,
        level: &DungeonLevel,
        player: &Player,
        enemies: &[Enemy],
        combat: &CombatState,
        typing: &str,
        message: &str,
        floor_num: i32,
        best_floor: i32,
        total_kills: u32,
        total_runs: u32,
        recipes_found: usize,
        srs: &crate::srs::SrsTracker,
        particles: &ParticleSystem,
        shake_timer: u8,
        flash: Option<(u8, u8, u8, f64)>,
        achievement_popup: Option<(&str, &str)>,
        room_modifier: Option<crate::world::RoomModifier>,
        listening_mode: ListenMode,
        companion: Option<crate::game::Companion>,
        companion_level: u8,
        quests: &[crate::game::Quest],
        tutorial_hint: Option<&str>,
        show_help: bool,
        item_labels: &[String],
        settings: &GameSettings,
        show_settings: bool,
        settings_cursor: usize,
        answer_streak: u32,
        floor_profile_label: &str,
        codex: &crate::codex::Codex,
        run_journal: &crate::game::RunJournal,
        post_mortem_page: usize,
        class_cursor: usize,
        location_label: &str,
        location_bonus: &str,
        show_minimap: bool,
        shop_sell_mode: bool,
    ) {
        let anim_t = Date::now() / 1000.0;
        // Screen shake offset
        let shake_x = if shake_timer > 0 {
            (shake_timer as f64 * 1.7).sin() * 4.0
        } else {
            0.0
        };
        let shake_y = if shake_timer > 0 {
            (shake_timer as f64 * 2.3).cos() * 3.0
        } else {
            0.0
        };

        // Camera: center on player
        let cam_x = player.x as f64 * TILE_SIZE - self.canvas_w / 2.0 + TILE_SIZE / 2.0 - shake_x;
        let cam_y = player.y as f64 * TILE_SIZE - self.canvas_h / 2.0 + TILE_SIZE / 2.0 - shake_y;

        // Clear
        self.ctx.set_fill_style_str(COL_FOG);
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

        // Determine visible tile range
        let start_tx = ((cam_x / TILE_SIZE).floor() as i32 - 1).max(0);
        let start_ty = ((cam_y / TILE_SIZE).floor() as i32 - 1).max(0);
        let end_tx = (((cam_x + self.canvas_w) / TILE_SIZE).ceil() as i32 + 1).min(level.width);
        let end_ty = (((cam_y + self.canvas_h) / TILE_SIZE).ceil() as i32 + 1).min(level.height);

        // Draw tiles
        for ty in start_ty..end_ty {
            for tx in start_tx..end_tx {
                let idx = level.idx(tx, ty);
                let screen_x = tx as f64 * TILE_SIZE - cam_x;
                let screen_y = ty as f64 * TILE_SIZE - cam_y;

                let visible = level.visible[idx];
                let revealed = level.revealed[idx];

                if !visible && !revealed {
                    continue; // fog
                }

                let tile = level.tiles[idx];
                let palette = tile_palette(tile, visible);

                self.ctx.set_fill_style_str(palette.fill);
                self.ctx.fill_rect(screen_x, screen_y, TILE_SIZE, TILE_SIZE);

                let mut tile_sprite_drawn = false;
                let sprite_key = tile_sprite_key(tile, location_label);
                if self.sprites.is_loaded(sprite_key) {
                    if let Some(img) = self.sprites.get(sprite_key) {
                        if !visible {
                            self.ctx.set_global_alpha(0.4);
                        }
                        if should_tile_sprite(tile) {
                            self.draw_tiling_sprite(img, tx, ty, screen_x, screen_y);
                        } else {
                            self.ctx
                                .draw_image_with_html_image_element_and_dw_and_dh(
                                    img, screen_x, screen_y, TILE_SIZE, TILE_SIZE,
                                )
                                .ok();
                        }
                        if !visible {
                            self.ctx.set_global_alpha(1.0);
                        }
                        tile_sprite_drawn = true;
                    }
                }

                if visible && !tile_sprite_drawn {
                    self.draw_tile_surface(level, tile, palette, tx, ty, screen_x, screen_y, anim_t);
                }
            }
        }

        // (Seam blending pass removed — tiling sprites now flow continuously)

        // Autotile border pass
        for ty in start_ty..end_ty {
            for tx in start_tx..end_tx {
                let idx = level.idx(tx, ty);
                if !level.visible[idx] && !level.revealed[idx] {
                    continue;
                }
                let tile = level.tiles[idx];
                let screen_x = tx as f64 * TILE_SIZE - cam_x;
                let screen_y = ty as f64 * TILE_SIZE - cam_y;
                let alpha = if level.visible[idx] { 1.0 } else { 0.4 };
                self.ctx.set_global_alpha(alpha);
                if is_wall_tile(tile) {
                    self.draw_wall_borders(level, tx, ty, screen_x, screen_y);
                } else if tile.is_walkable() {
                    self.draw_floor_borders(level, tx, ty, screen_x, screen_y);
                }
                self.ctx.set_global_alpha(1.0);
            }
        }

        self.draw_room_ambience(room_modifier, anim_t);

        // Draw player
        let px = player.x as f64 * TILE_SIZE - cam_x;
        let py = player.y as f64 * TILE_SIZE - cam_y;
        let center_x = px + TILE_SIZE / 2.0;
        let center_y = py + TILE_SIZE / 2.0 + (anim_t * 4.4).sin() * 1.4;
        let r = TILE_SIZE * 0.38;

        let player_key = match player.form {
            PlayerForm::Human => "player_human",
            PlayerForm::Powered => "player_flame",
            PlayerForm::Cybernetic => "player_stone",
            PlayerForm::Holographic => "player_mist",
            PlayerForm::Void => "player_tiger",
        };
        let mut player_sprite_drawn = false;
        if self.sprites.is_loaded(player_key) {
            if let Some(img) = self.sprites.get(player_key) {
                self.ctx
                    .draw_image_with_html_image_element_and_dw_and_dh(
                        img, px, py, TILE_SIZE, TILE_SIZE,
                    )
                    .ok();
                player_sprite_drawn = true;
            }
        }

        if !player_sprite_drawn && player.form == PlayerForm::Human {
            // Glow
            self.ctx.set_shadow_color("rgba(255,204,51,0.5)");
            self.ctx.set_shadow_blur(12.0);

            // Body circle
            self.ctx.set_fill_style_str(COL_PLAYER);
            self.ctx.begin_path();
            self.ctx
                .arc(center_x, center_y, r, 0.0, std::f64::consts::TAU)
                .ok();
            self.ctx.fill();

            // Outline
            self.ctx.set_stroke_style_str(COL_PLAYER_OUTLINE);
            self.ctx.set_line_width(2.0);
            self.ctx.stroke();

            // Eyes
            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_fill_style_str("#222");
            self.ctx.begin_path();
            self.ctx
                .arc(
                    center_x - r * 0.3,
                    center_y - r * 0.15,
                    r * 0.15,
                    0.0,
                    std::f64::consts::TAU,
                )
                .ok();
            self.ctx.fill();
            self.ctx.begin_path();
            self.ctx
                .arc(
                    center_x + r * 0.3,
                    center_y - r * 0.15,
                    r * 0.15,
                    0.0,
                    std::f64::consts::TAU,
                )
                .ok();
            self.ctx.fill();

            // Ears (triangles)
            self.ctx.set_fill_style_str(COL_PLAYER);
            self.ctx.begin_path();
            self.ctx.move_to(center_x - r * 0.6, center_y - r * 0.5);
            self.ctx.line_to(center_x - r * 0.15, center_y - r * 1.15);
            self.ctx.line_to(center_x + r * 0.1, center_y - r * 0.5);
            self.ctx.fill();
            self.ctx.begin_path();
            self.ctx.move_to(center_x + r * 0.6, center_y - r * 0.5);
            self.ctx.line_to(center_x + r * 0.15, center_y - r * 1.15);
            self.ctx.line_to(center_x - r * 0.1, center_y - r * 0.5);
            self.ctx.fill();
        } else if !player_sprite_drawn {
            // Render Form Glyph
            self.ctx.set_shadow_color(player.form.color());
            self.ctx.set_shadow_blur(15.0);

            self.ctx.set_font("bold 22px serif");
            self.ctx.set_text_align("center");
            self.ctx.set_text_baseline("middle");
            self.ctx.set_fill_style_str(player.form.color());
            self.ctx
                .fill_text(player.form.glyph(), center_x, center_y)
                .ok();
        }

        if player.form_timer > 0 {
            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_fill_style_str("#444");
            self.ctx
                .fill_rect(center_x - 10.0, center_y + 12.0, 20.0, 3.0);
            self.ctx.set_fill_style_str(if player_sprite_drawn {
                "#00ccdd"
            } else {
                player.form.color()
            });
            let pct = (player.form_timer as f64 / 50.0).min(1.0);
            self.ctx
                .fill_rect(center_x - 10.0, center_y + 12.0, 20.0 * pct, 3.0);
        }

        // Reset shadow
        self.ctx.set_shadow_blur(0.0);
        self.ctx.set_shadow_color("transparent");

        // ── Enemies ─────────────────────────────────────────────────────
        for (i, enemy) in enemies.iter().enumerate() {
            if !enemy.is_alive() {
                continue;
            }
            if !level.in_bounds(enemy.x, enemy.y) {
                continue;
            }
            let eidx = level.idx(enemy.x, enemy.y);
            if !level.visible[eidx] {
                continue;
            }
            let ex = enemy.x as f64 * TILE_SIZE - cam_x;
            let ey = enemy.y as f64 * TILE_SIZE - cam_y
                + (anim_t * 3.3 + (enemy.x as f64 * 0.37) + (enemy.y as f64 * 0.23)).sin() * 1.2;

            let enemy_sprite_key = if enemy.is_boss {
                enemy
                    .boss_kind
                    .map(boss_sprite_key)
                    .unwrap_or("enemy_generic")
            } else if enemy.is_elite {
                "enemy_elite"
            } else {
                enemy_sprite_for_location(location_label, i)
            };
            if self.sprites.is_loaded(enemy_sprite_key) {
                if let Some(img) = self.sprites.get(enemy_sprite_key) {
                    self.ctx
                        .draw_image_with_html_image_element_and_dw_and_dh(
                            img, ex, ey, TILE_SIZE, TILE_SIZE,
                        )
                        .ok();
                }
            }

            // Red/purple/gold glow for alerted/boss/elite enemies
            if enemy.is_boss {
                self.ctx.set_shadow_color("rgba(200,50,255,0.8)");
                self.ctx.set_shadow_blur(14.0);
            } else if enemy.is_elite {
                self.ctx.set_shadow_color("rgba(255,200,50,0.7)");
                self.ctx.set_shadow_blur(12.0);
            } else if enemy.alert {
                self.ctx.set_shadow_color("rgba(255,60,60,0.6)");
                self.ctx.set_shadow_blur(10.0);
            }

            // Highlight the enemy being fought
            let is_fighting =
                matches!(combat, CombatState::Fighting { enemy_idx, .. } if *enemy_idx == i);
            if is_fighting {
                self.ctx.set_stroke_style_str("#ff4444");
                self.ctx.set_line_width(2.0);
                self.ctx
                    .stroke_rect(ex + 1.0, ey + 1.0, TILE_SIZE - 2.0, TILE_SIZE - 2.0);
            }

            // Draw Hanzi character (bosses are larger and purple)
            let font_size = if enemy.is_boss {
                "22px"
            } else if enemy.is_elite {
                "16px"
            } else {
                "18px"
            };
            let color = if enemy.is_boss {
                "#cc66ff"
            } else if enemy.is_elite {
                "#00ccdd"
            } else if enemy.alert {
                "#ff6666"
            } else {
                "#cc8888"
            };
            self.ctx.set_fill_style_str(color);
            self.ctx
                .set_font(&format!("{} 'Noto Serif SC', 'SimSun', serif", font_size));
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(enemy.hanzi, ex + TILE_SIZE / 2.0, ey + TILE_SIZE * 0.72)
                .ok();

            // Small HP bar below
            if enemy.hp < enemy.max_hp {
                let hp_frac = enemy.hp as f64 / enemy.max_hp as f64;
                self.ctx.set_fill_style_str("#440000");
                self.ctx
                    .fill_rect(ex + 2.0, ey + TILE_SIZE - 4.0, TILE_SIZE - 4.0, 3.0);
                self.ctx.set_fill_style_str("#ff4444");
                self.ctx.fill_rect(
                    ex + 2.0,
                    ey + TILE_SIZE - 4.0,
                    (TILE_SIZE - 4.0) * hp_frac,
                    3.0,
                );
            }

            // Elite star marker
            if enemy.is_elite {
                self.ctx.set_fill_style_str("#00ccdd");
                self.ctx.set_font("10px monospace");
                self.ctx.set_text_align("left");
                self.ctx.fill_text("★", ex, ey + 8.0).ok();
            }

            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_shadow_color("transparent");
        }

        if let CombatState::Looking { x, y } = combat {
            let min_x = (player.x - 3).max(0);
            let max_x = (player.x + 3).min(level.width - 1);
            let min_y = (player.y - 3).max(0);
            let max_y = (player.y + 3).min(level.height - 1);
            let left = min_x as f64 * TILE_SIZE - cam_x;
            let top = min_y as f64 * TILE_SIZE - cam_y;
            let width = (max_x - min_x + 1) as f64 * TILE_SIZE;
            let height = (max_y - min_y + 1) as f64 * TILE_SIZE;

            self.ctx.set_stroke_style_str("rgba(143,168,255,0.45)");
            self.ctx.set_line_width(1.5);
            self.ctx
                .stroke_rect(left + 0.5, top + 0.5, width - 1.0, height - 1.0);

            let look_x = *x as f64 * TILE_SIZE - cam_x;
            let look_y = *y as f64 * TILE_SIZE - cam_y;
            self.ctx.set_fill_style_str("rgba(143,168,255,0.16)");
            self.ctx.fill_rect(look_x, look_y, TILE_SIZE, TILE_SIZE);
            self.ctx.set_stroke_style_str("#dbe7ff");
            self.ctx.set_line_width(2.0);
            self.ctx
                .stroke_rect(look_x + 1.5, look_y + 1.5, TILE_SIZE - 3.0, TILE_SIZE - 3.0);
            self.ctx.set_font("10px monospace");
            self.ctx.set_text_align("center");
            self.ctx.set_fill_style_str("#dbe7ff");
            self.ctx
                .fill_text("LOOK", look_x + TILE_SIZE / 2.0, look_y - 4.0)
                .ok();
        }

        if let CombatState::Aiming { dx, dy, spell_idx } = combat {
            let max_range = 10;
            let mut cx = player.x;
            let mut cy = player.y;
            for _ in 0..max_range {
                cx += dx;
                cy += dy;
                if !level.in_bounds(cx, cy) {
                    break;
                }
                let idx = (cy * level.width + cx) as usize;
                if !level.tiles[idx].is_walkable() {
                    break;
                }
                let tx = cx as f64 * TILE_SIZE - cam_x;
                let ty = cy as f64 * TILE_SIZE - cam_y;
                if enemies
                    .iter()
                    .any(|e| e.is_alive() && e.x == cx && e.y == cy)
                {
                    self.ctx.set_fill_style_str("rgba(255,80,60,0.35)");
                    self.ctx.fill_rect(tx, ty, TILE_SIZE, TILE_SIZE);
                    self.ctx.set_stroke_style_str("#ff5040");
                    self.ctx.set_line_width(2.0);
                    self.ctx
                        .stroke_rect(tx + 1.0, ty + 1.0, TILE_SIZE - 2.0, TILE_SIZE - 2.0);
                    break;
                }
                self.ctx.set_fill_style_str("rgba(255,200,80,0.18)");
                self.ctx.fill_rect(tx, ty, TILE_SIZE, TILE_SIZE);
            }
            let arrow = match (*dx, *dy) {
                (0, -1) => "↑",
                (0, 1) => "↓",
                (-1, 0) => "←",
                (1, 0) => "→",
                _ => "·",
            };
            let px = player.x as f64 * TILE_SIZE - cam_x;
            let py = player.y as f64 * TILE_SIZE - cam_y;
            self.ctx.set_font("16px monospace");
            self.ctx.set_text_align("center");
            self.ctx.set_fill_style_str("#ffcc44");
            self.ctx
                .fill_text(arrow, px + TILE_SIZE / 2.0, py - 4.0)
                .ok();

            let spell_label = if *spell_idx < player.spells.len() {
                format!(
                    "{} {}",
                    player.spells[*spell_idx].hanzi,
                    player.spells[*spell_idx].effect.label()
                )
            } else {
                "Spell".to_string()
            };
            self.ctx.set_font("10px monospace");
            self.ctx.set_fill_style_str("#ffdd88");
            self.ctx
                .fill_text(&spell_label, px + TILE_SIZE / 2.0, py - 16.0)
                .ok();
        }

        // ── HUD ─────────────────────────────────────────────────────────
        // HP bar (top-left)
        let bar_x = 12.0;
        let bar_y = 12.0;
        let bar_w = 160.0;
        let bar_h = 16.0;
        let hp_frac = (player.hp as f64 / player.max_hp as f64).clamp(0.0, 1.0);

        self.ctx.set_fill_style_str(COL_HP_BG);
        self.ctx.fill_rect(bar_x, bar_y, bar_w, bar_h);
        self.ctx.set_fill_style_str(COL_HP_BAR);
        self.ctx.fill_rect(bar_x, bar_y, bar_w * hp_frac, bar_h);
        self.ctx.set_stroke_style_str("#666");
        self.ctx.set_line_width(1.0);
        self.ctx.stroke_rect(bar_x, bar_y, bar_w, bar_h);

        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_font("12px monospace");
        self.ctx.set_text_align("left");
        self.ctx
            .fill_text(
                &format!("HP {}/{}", player.hp, player.max_hp),
                bar_x + 4.0,
                bar_y + 12.0,
            )
            .ok();


        // Status effect icons (right of HP bar)
        {
            let mut sx = bar_x + bar_w + 8.0;
            self.ctx.set_font("10px monospace");
            for s in &player.statuses {
                self.ctx.set_fill_style_str(s.color());
                self.ctx
                    .fill_text(&format!("{}{}", s.label(), s.turns_left), sx, bar_y + 12.0)
                    .ok();
                sx += 44.0;
            }
        }

        // Floor indicator + gold (top-right)
        self.ctx.set_text_align("right");
        self.ctx.set_font("14px monospace");
        self.ctx.set_fill_style_str("#aaa");
        let floor_label = if !location_label.is_empty() {
            if floor_num == 0 {
                format!("{} — Tutorial  Best: {}", location_label, best_floor)
            } else {
                format!("{} — Deck {}  Best: {}", location_label, floor_num, best_floor)
            }
        } else if floor_num == 0 {
            format!("Tutorial  Best: {}", best_floor)
        } else {
            format!("Floor {}  Best: {}", floor_num, best_floor)
        };
        self.ctx
            .fill_text(&floor_label, self.canvas_w - 12.0, 24.0)
            .ok();
        // Floor profile (Famine / Radical Rich / Siege)
        if !floor_profile_label.is_empty() {
            self.ctx.set_fill_style_str("#dd8844");
            self.ctx.set_font("11px monospace");
            self.ctx
                .fill_text(floor_profile_label, self.canvas_w - 12.0, 38.0)
                .ok();
        }
        // Location bonus description
        let mut bonus_offset = 0.0;
        if !location_bonus.is_empty() {
            let bonus_y = if floor_profile_label.is_empty() { 38.0 } else { 50.0 };
            self.ctx.set_fill_style_str("#44dd88");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(location_bonus, self.canvas_w - 12.0, bonus_y)
                .ok();
            bonus_offset = 12.0;
        }
        self.ctx.set_fill_style_str("#ffdd44");
        self.ctx.set_font("14px monospace");
        let gold_y = if floor_profile_label.is_empty() {
            42.0 + bonus_offset
        } else {
            52.0 + bonus_offset
        };
        self.ctx
            .fill_text(&format!("{}g", player.gold), self.canvas_w - 12.0, gold_y)
            .ok();

        // Equipment display (top-right, below gold)
        let mut eq_y = gold_y + 16.0;
        self.ctx.set_font("10px monospace");
        if let Some(w) = player.weapon {
            let ench = player.enchantments[0]
                .map(|e| format!(" [{}]", e))
                .unwrap_or_default();
            self.ctx.set_fill_style_str("#ff8866");
            self.ctx
                .fill_text(&format!("⚔ {}{}", w.name, ench), self.canvas_w - 12.0, eq_y)
                .ok();
            eq_y += 14.0;
        }
        if let Some(a) = player.armor {
            let ench = player.enchantments[1]
                .map(|e| format!(" [{}]", e))
                .unwrap_or_default();
            self.ctx.set_fill_style_str("#6688ff");
            self.ctx
                .fill_text(&format!("🛡 {}{}", a.name, ench), self.canvas_w - 12.0, eq_y)
                .ok();
            eq_y += 14.0;
        }
        if let Some(c) = player.charm {
            let ench = player.enchantments[2]
                .map(|e| format!(" [{}]", e))
                .unwrap_or_default();
            self.ctx.set_fill_style_str("#88ddaa");
            self.ctx
                .fill_text(&format!("✧ {}{}", c.name, ench), self.canvas_w - 12.0, eq_y)
                .ok();
            eq_y += 14.0;
        }

        // Room modifier indicator
        if let Some(modifier) = room_modifier {
            let (label, color) = match modifier {
                crate::world::RoomModifier::PoweredDown => ("🌑 Powered Down", "#8888bb"),
                crate::world::RoomModifier::HighTech => ("✨ High-Tech Module", "#aa66ff"),
                crate::world::RoomModifier::Irradiated => ("☢ Irradiated Zone", "#ff6666"),
                crate::world::RoomModifier::Hydroponics => ("🌿 Hydroponics Bay", "#66cc66"),
                crate::world::RoomModifier::Cryogenic => ("❄ Cryo Bay", "#88ccff"),
                crate::world::RoomModifier::OverheatedReactor => ("🔥 Reactor Core", "#ff6600"),
            };
            self.ctx.set_fill_style_str(color);
            self.ctx.fill_text(label, self.canvas_w - 12.0, eq_y).ok();
            eq_y += 14.0;
        }
        // Listening mode indicator
        if listening_mode.is_active() {
            self.ctx.set_fill_style_str("#aa66ff");
            self.ctx
                .fill_text(
                    &format!("🎧 {}", listening_mode.label()),
                    self.canvas_w - 12.0,
                    eq_y,
                )
                .ok();
            eq_y += 14.0;
        }
        // Companion indicator
        if let Some(comp) = companion {
            self.ctx.set_fill_style_str("#55ccaa");
            let label = if companion_level > 0 {
                format!("{} {} Lv.{}", comp.icon(), comp.name(), companion_level)
            } else {
                format!("{} {}", comp.icon(), comp.name())
            };
            self.ctx.fill_text(&label, self.canvas_w - 12.0, eq_y).ok();
            eq_y += 14.0;
        }
        // Faction piety
        for &(deity, piety) in &player.piety {
            if piety != 0 {
                let (icon, color) = match deity {
                    Faction::Consortium => ("🟢", "#66cc88"),
                    Faction::MilitaryAlliance => ("⚙", "#99aacc"),
                    Faction::AncientOrder => ("💰", "#ddaa44"),
                    Faction::FreeTraders => ("🌀", "#66bbdd"),
                    Faction::Technocracy => ("🪞", "#bb88dd"),
                };
                self.ctx.set_fill_style_str(color);
                self.ctx
                    .fill_text(&format!("{} {:+}", icon, piety), self.canvas_w - 12.0, eq_y)
                    .ok();
                eq_y += 14.0;
            }
        }
        if let Some((synergy_name, _)) = player.faction_synergy() {
            self.ctx.set_fill_style_str("#ffd700");
            self.ctx
                .fill_text(&format!("⚡ {}", synergy_name), self.canvas_w - 12.0, eq_y)
                .ok();
            eq_y += 14.0;
        }

        self.ctx.set_fill_style_str("#9cb7ff");
        self.ctx
            .fill_text("[V] Look", self.canvas_w - 12.0, eq_y)
            .ok();
        eq_y += 14.0;
        self.ctx.set_fill_style_str("#7e8dbb");
        self.ctx
            .fill_text("[X] Skip floor", self.canvas_w - 12.0, eq_y)
            .ok();
        eq_y += 14.0;
        self.ctx.set_fill_style_str("#8fa8ff");
        self.ctx
            .fill_text("[?] Help", self.canvas_w - 12.0, eq_y)
            .ok();

        if let Some(tutorial_hint) = tutorial_hint {
            let box_w = 360.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            self.ctx.set_fill_style_str("rgba(10,8,18,0.82)");
            self.ctx.fill_rect(box_x, 10.0, box_w, 38.0);
            self.ctx.set_stroke_style_str("#00ccdd");
            self.ctx.set_line_width(1.0);
            self.ctx.stroke_rect(box_x, 10.0, box_w, 38.0);
            self.ctx.set_text_align("center");
            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.set_font("11px monospace");
            self.ctx
                .fill_text("Tutorial", self.canvas_w / 2.0, 24.0)
                .ok();
            self.ctx.set_fill_style_str("#ddd");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(tutorial_hint, self.canvas_w / 2.0, 39.0)
                .ok();
        }

        // ── Spell bar (left side) ────────────────────────────────────────
        if !player.spells.is_empty() {
            let sp_x = 12.0;
            let sp_y = 44.0;
            self.ctx.set_font("11px monospace");
            self.ctx.set_text_align("left");
            self.ctx.set_fill_style_str("#44aaff");
            self.ctx.fill_text("Spells:", sp_x, sp_y).ok();
            for (i, spell) in player.spells.iter().enumerate() {
                let y = sp_y + 16.0 + i as f64 * 16.0;
                let selected = i == player.selected_spell;
                self.ctx
                    .set_fill_style_str(if selected { "#00ccdd" } else { "#88bbdd" });
                self.ctx.set_font("12px monospace");
                let marker = if selected { "►" } else { " " };
                let mut text_x = sp_x;
                if self.draw_sprite_icon(spell_sprite_key(&spell.effect), sp_x, y - 12.0, 14.0) {
                    text_x += 18.0;
                }
                self.ctx
                    .fill_text(
                        &format!("{}{} {}", marker, spell.hanzi, spell.effect.label()),
                        text_x,
                        y,
                    )
                    .ok();
            }
        }

        // ── Item inventory (below spells, left side) ────────────────────
        if !player.items.is_empty() {
            let spell_count = player.spells.len();
            let base_y = if player.spells.is_empty() {
                44.0
            } else {
                44.0 + 16.0 + spell_count as f64 * 16.0 + 8.0
            };
            self.ctx.set_font("11px monospace");
            self.ctx.set_text_align("left");
            self.ctx.set_fill_style_str("#ddaa44");
            self.ctx
                .fill_text("Items [1-5]  [I] Inventory:", 12.0, base_y)
                .ok();
            for (i, label) in item_labels.iter().enumerate() {
                let y = base_y + 16.0 + i as f64 * 14.0;
                self.ctx.set_fill_style_str("#ccbb66");
                self.ctx.set_font("11px monospace");
                let mut text_x = 12.0;
                if let Some(item) = player.items.get(i) {
                    if self.draw_sprite_icon(item_sprite_key(item), 12.0, y - 11.0, 12.0) {
                        text_x += 16.0;
                    }
                }
                self.ctx
                    .fill_text(&format!("{}: {}", i + 1, label), text_x, y)
                    .ok();
            }
        }

        // Shield indicator
        if player.shield {
            self.ctx.set_font("12px monospace");
            self.ctx.set_text_align("left");
            self.ctx.set_fill_style_str("#44ddff");
            self.ctx.fill_text("🛡 Energy Barrier Active", 12.0, 36.0).ok();
        }

        // ── Particles ────────────────────────────────────────────────────
        for p in &particles.particles {
            let alpha = p.life.max(0.0).min(1.0);
            let radius = (p.size * (0.35 + alpha * 0.65)).max(1.0);
            self.ctx
                .set_fill_style_str(&format!("rgba({},{},{},{})", p.r, p.g, p.b, alpha));
            self.ctx
                .set_shadow_color(&format!("rgba({},{},{},{})", p.r, p.g, p.b, alpha * 0.7));
            self.ctx.set_shadow_blur(radius * 2.5);
            self.ctx.begin_path();
            self.ctx
                .arc(p.x, p.y, radius, 0.0, std::f64::consts::TAU)
                .ok();
            self.ctx.fill();

            let speed = (p.vx * p.vx + p.vy * p.vy).sqrt();
            if speed > 1.2 {
                self.ctx.set_stroke_style_str(&format!(
                    "rgba({},{},{},{})",
                    p.r,
                    p.g,
                    p.b,
                    alpha * 0.35
                ));
                self.ctx.set_line_width(radius.max(1.0));
                self.ctx.begin_path();
                self.ctx.move_to(p.x, p.y);
                self.ctx.line_to(p.x - p.vx * 1.5, p.y - p.vy * 1.5);
                self.ctx.stroke();
            }
        }
        self.ctx.set_shadow_blur(0.0);
        self.ctx.set_shadow_color("transparent");

        // ── Flash overlay ───────────────────────────────────────────────
        if let Some((r, g, b, a)) = flash {
            self.ctx
                .set_fill_style_str(&format!("rgba({},{},{},{})", r, g, b, a));
            self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);
        }

        // ── Achievement popup (top-center) ──────────────────────────────
        if let Some((icon_name, desc)) = achievement_popup {
            let pw = 280.0;
            let ph = 50.0;
            let px = (self.canvas_w - pw) / 2.0;
            let py = 50.0;
            self.ctx.set_fill_style_str("rgba(40,30,60,0.9)");
            self.ctx.fill_rect(px, py, pw, ph);
            self.ctx.set_stroke_style_str("#00ccdd");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(px, py, pw, ph);
            self.ctx.set_font("bold 14px monospace");
            self.ctx.set_text_align("center");
            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx
                .fill_text("🏆 Achievement Unlocked!", px + pw / 2.0, py + 20.0)
                .ok();
            self.ctx.set_font("12px monospace");
            self.ctx.set_fill_style_str("#ffffff");
            self.ctx
                .fill_text(
                    &format!("{} — {}", icon_name, desc),
                    px + pw / 2.0,
                    py + 40.0,
                )
                .ok();
        }

        // Minimap (bottom-right)
        if show_minimap {
            self.draw_minimap(level, player);
        }

        // Quest tracker (bottom-left)
        if !quests.is_empty() {
            let mut qy = self.canvas_h - 16.0 * quests.len() as f64 - 8.0;
            self.ctx.set_font("11px monospace");
            self.ctx.set_text_align("left");
            for q in quests {
                let status = if q.completed { "✓" } else { "○" };
                let progress = match &q.goal {
                    crate::game::QuestGoal::KillEnemies(c, t) => format!("{}/{}", c, t),
                    crate::game::QuestGoal::ReachFloor(f) => format!("F{}", f),
                    crate::game::QuestGoal::CollectRadicals(c, t) => format!("{}/{}", c, t),
                    crate::game::QuestGoal::ForgeCharacter(ch) => ch.to_string(),
                };
                let color = if q.completed { "#66ff66" } else { "#aaaacc" };
                self.ctx.set_fill_style_str(color);
                self.ctx
                    .fill_text(
                        &format!("{} {} [{}]", status, q.description, progress),
                        8.0,
                        qy,
                    )
                    .ok();
                qy += 16.0;
            }
        }

        // ── Message bar (bottom-center) — styled with gradient ─────────
        if !message.is_empty() {
            let message_lift = (anim_t * 7.5).sin().abs() * 2.0;
            let msg_x = self.canvas_w * 0.15;
            let msg_y = self.canvas_h - 38.0 - message_lift;
            let msg_w = self.canvas_w * 0.7;
            let msg_h = 30.0;
            self.ctx.set_font("14px monospace");
            self.ctx.set_text_align("center");
            // Gradient background
            self.ctx.set_fill_style_str("rgba(0,0,0,0.85)");
            self.ctx.fill_rect(msg_x, msg_y, msg_w, msg_h);
            self.ctx.set_fill_style_str("rgba(40,30,60,0.4)");
            self.ctx.fill_rect(msg_x, msg_y, msg_w, msg_h * 0.5);
            // Accent border
            self.ctx.set_stroke_style_str("rgba(120,100,160,0.3)");
            self.ctx.set_line_width(1.0);
            self.ctx.stroke_rect(msg_x, msg_y, msg_w, msg_h);
            self.ctx.set_fill_style_str(hud_message_color(message));
            self.ctx
                .fill_text(
                    message,
                    self.canvas_w / 2.0,
                    self.canvas_h - 17.0 - message_lift,
                )
                .ok();
        }

        // ── Combat UI (center overlay when fighting) ────────────────────
        if let CombatState::Fighting { enemy_idx, .. } = combat {
            let enemy_idx = *enemy_idx;
            if enemy_idx < enemies.len() {
                let enemy = &enemies[enemy_idx];
                let boss_trait = enemy.boss_trait_text();
                let elite_hint = if enemy.is_elite {
                    let target = enemy
                        .hanzi
                        .chars()
                        .nth(enemy.elite_chain)
                        .map(|ch| ch.to_string())
                        .unwrap_or_else(|| enemy.hanzi.chars().last().unwrap_or('？').to_string());
                    Some(format!(
                        "Compound Break {}/{} — {} = {}",
                        enemy.elite_chain + 1,
                        enemy.elite_phase_count(),
                        target,
                        enemy.elite_expected_syllable().unwrap_or(enemy.pinyin)
                    ))
                } else {
                    None
                };
                let box_w = 320.0;
                let box_h = if boss_trait.is_some() {
                    160.0
                } else if enemy.is_elite {
                    172.0
                } else {
                    140.0
                };
                let box_x = (self.canvas_w - box_w) / 2.0;
                let box_y = 50.0 + (anim_t * 4.0).sin() * 2.0;
                let hanzi_y = if boss_trait.is_some() {
                    box_y + 46.0
                } else {
                    box_y + 52.0
                };

                // Background
                self.ctx.set_fill_style_str("rgba(20,10,30,0.92)");
                self.ctx.fill_rect(box_x, box_y, box_w, box_h);
                self.ctx.set_stroke_style_str("#ff6666");
                self.ctx.set_line_width(2.0);
                self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

                // Streak badge (top-right corner of combat box)
                let tier = combo_tier(answer_streak);
                if tier != ComboTier::None {
                    let color = match tier {
                        ComboTier::None => "#ffffff",
                        ComboTier::Good => "#aaddff",
                        ComboTier::Great => "#44dd88",
                        ComboTier::Excellent => "#ffdd44",
                        ComboTier::Perfect => "#ff8844",
                        ComboTier::Radical => "#ff4422",
                    };
                    self.ctx.set_fill_style_str(color);
                    self.ctx.set_font("bold 11px monospace");
                    self.ctx.set_text_align("right");
                    self.ctx
                        .fill_text(
                            &format!("🔥 {}! ×{}", tier.name(), answer_streak),
                            box_x + box_w - 8.0,
                            box_y + 14.0,
                        )
                        .ok();
                    self.ctx.set_text_align("center");
                }

                if let Some(kind) = enemy.boss_kind {
                    self.ctx.set_fill_style_str("#ffcc88");
                    self.ctx.set_font("bold 12px monospace");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text(kind.title(), self.canvas_w / 2.0, box_y + 16.0)
                        .ok();
                }

                // Enemy hanzi (large) — hidden in listening mode for non-elite
                let show_hanzi = !listening_mode.is_active() || enemy.is_elite;
                self.ctx
                    .set_fill_style_str(if listening_mode.is_active() && !enemy.is_elite {
                        "#aa66ff"
                    } else {
                        "#ff6666"
                    });
                self.ctx.set_font("48px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(
                        if show_hanzi { enemy.hanzi } else { "🎧 ???" },
                        self.canvas_w / 2.0,
                        hanzi_y,
                    )
                    .ok();

                // Meaning hint (hidden in listening mode)
                if show_hanzi {
                    self.ctx.set_fill_style_str("#999");
                    self.ctx.set_font("12px monospace");
                    self.ctx
                        .fill_text(
                            &format!("({})", enemy.meaning),
                            self.canvas_w / 2.0,
                            hanzi_y + 20.0,
                        )
                        .ok();
                } else {
                    self.ctx.set_fill_style_str("#aa66ff");
                    self.ctx.set_font("12px monospace");
                    // Teacher companion reveals meaning even in listening mode
                    let teacher_hint = if companion == Some(crate::game::Companion::ScienceOfficer) {
                        &format!("📚 ({})", enemy.meaning)
                    } else {
                        "(listen carefully...)"
                    };
                    self.ctx
                        .fill_text(teacher_hint, self.canvas_w / 2.0, hanzi_y + 20.0)
                        .ok();
                }

                if let Some(trait_text) = boss_trait.as_deref() {
                    self.ctx.set_fill_style_str("#c8a6ff");
                    self.ctx.set_font("11px monospace");
                    self.ctx
                        .fill_text(trait_text, self.canvas_w / 2.0, box_y + 84.0)
                        .ok();
                } else if let Some(elite_hint) = elite_hint.as_deref() {
                    self.ctx.set_fill_style_str("#ffcc88");
                    self.ctx.set_font("11px monospace");
                    self.ctx
                        .fill_text(elite_hint, self.canvas_w / 2.0, box_y + 84.0)
                        .ok();
                    self.ctx.set_fill_style_str("#aa8877");
                    self.ctx.set_font("10px monospace");
                    self.ctx
                        .fill_text(
                            "Finish the full chain to shatter the compound.",
                            self.canvas_w / 2.0,
                            box_y + 98.0,
                        )
                        .ok();

                    let segment_count = enemy.elite_phase_count().max(1);
                    let seg_w = 24.0;
                    let seg_gap = 8.0;
                    let total_w = segment_count as f64 * seg_w
                        + segment_count.saturating_sub(1) as f64 * seg_gap;
                    let seg_x = self.canvas_w / 2.0 - total_w / 2.0;
                    let seg_y = box_y + 104.0;
                    for step in 0..segment_count {
                        let filled = step < enemy.elite_chain;
                        let current = step == enemy.elite_chain.min(segment_count - 1);
                        let x = seg_x + step as f64 * (seg_w + seg_gap);
                        self.ctx.set_fill_style_str(if filled {
                            "#ffbb44"
                        } else if current {
                            "rgba(255,204,102,0.28)"
                        } else {
                            "rgba(255,255,255,0.08)"
                        });
                        self.ctx.fill_rect(x, seg_y, seg_w, 8.0);
                        self.ctx
                            .set_stroke_style_str(if current { "#ffdd88" } else { "#665544" });
                        self.ctx.set_line_width(1.0);
                        self.ctx.stroke_rect(x, seg_y, seg_w, 8.0);
                    }
                }

                // Typing input box
                let input_y = if boss_trait.is_some() {
                    box_y + 102.0
                } else if enemy.is_elite {
                    box_y + 114.0
                } else {
                    box_y + 90.0
                };
                self.ctx.set_fill_style_str("rgba(0,0,0,0.5)");
                self.ctx
                    .fill_rect(box_x + 30.0, input_y, box_w - 60.0, 28.0);
                self.ctx.set_stroke_style_str("#555");
                self.ctx.set_line_width(1.0);
                self.ctx
                    .stroke_rect(box_x + 30.0, input_y, box_w - 60.0, 28.0);

                // Typed text
                let display = if typing.is_empty() {
                    if enemy.is_elite {
                        "type next syllable…"
                    } else {
                        "type pinyin…"
                    }
                } else {
                    typing
                };
                self.ctx
                    .set_fill_style_str(if typing.is_empty() { "#555" } else { "#00ccdd" });
                self.ctx.set_font("16px monospace");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(display, self.canvas_w / 2.0, input_y + 20.0)
                    .ok();

                // Hint text
                self.ctx.set_fill_style_str("#555");
                self.ctx.set_font("10px monospace");
                // Show example sentence if available
                let example = crate::vocab::VOCAB
                    .iter()
                    .find(|v| v.hanzi == enemy.hanzi)
                    .map(|v| v.example)
                    .unwrap_or("");
                if !example.is_empty() && show_hanzi {
                    self.ctx.set_fill_style_str("#667788");
                    self.ctx.set_font("11px 'Noto Serif SC', monospace");
                    self.ctx
                        .fill_text(example, self.canvas_w / 2.0, box_y + box_h - 8.0)
                        .ok();
                    self.ctx.set_fill_style_str("#555");
                    self.ctx.set_font("10px monospace");
                }
                self.ctx
                    .fill_text(
                        if enemy.is_elite {
                            "Enter=submit syllable  Esc=flee  Q=cycle spell  Space=cast spell"
                        } else {
                            "Enter=submit  Esc=flee  Q=cycle spell  Space=cast spell"
                        },
                        self.canvas_w / 2.0,
                        box_y + box_h + 14.0,
                    )
                    .ok();
            }
        }

        // ── Tactical Battle overlay ──────────────────────────────────────
        if let CombatState::TacticalBattle(ref battle) = combat {
            self.draw_tactical_battle(battle, anim_t, player);
        }


        // Overlays (forge, shop, enchanting, challenges, game over, class select)
        self.draw_overlays(
            combat, player, anim_t, typing,
            floor_num, best_floor, total_kills, total_runs,
            recipes_found, srs, settings, show_settings,
            settings_cursor, codex, run_journal, post_mortem_page,
            class_cursor, item_labels, shop_sell_mode, answer_streak,
            companion, companion_level, location_label,
        );


        if show_help {
            self.draw_help_overlay(combat, listening_mode);
        }
    }

    fn draw_room_ambience(&self, room_modifier: Option<crate::world::RoomModifier>, anim_t: f64) {
        match room_modifier {
            Some(crate::world::RoomModifier::PoweredDown) => {
                self.ctx.set_fill_style_str("rgba(10,10,22,0.16)");
                self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);
                self.ctx.set_fill_style_str("rgba(150,160,220,0.08)");
                for i in 0..8 {
                    let seed = i as f64 * 0.83;
                    let x = (((anim_t * 0.27) + seed).sin() * 0.5 + 0.5) * self.canvas_w;
                    let y = (((anim_t * 0.18) + seed * 1.7).cos() * 0.5 + 0.5) * self.canvas_h;
                    let r = 18.0 + (((anim_t * 0.9) + seed).sin() * 0.5 + 0.5) * 18.0;
                    self.ctx.begin_path();
                    self.ctx.arc(x, y, r, 0.0, std::f64::consts::TAU).ok();
                    self.ctx.fill();
                }
            }
            Some(crate::world::RoomModifier::HighTech) => {
                for i in 0..14 {
                    let seed = i as f64 * 1.13;
                    let x = (((anim_t * 0.41) + seed).sin() * 0.5 + 0.5) * self.canvas_w;
                    let y = (((anim_t * 0.66) + seed * 0.7).cos() * 0.5 + 0.5) * self.canvas_h;
                    let alpha = 0.18 + (((anim_t * 2.0) + seed).sin() * 0.5 + 0.5) * 0.16;
                    let radius = 1.5 + (((anim_t * 1.3) + seed).cos() * 0.5 + 0.5) * 2.5;
                    self.ctx
                        .set_fill_style_str(&format!("rgba(180,120,255,{alpha})"));
                    self.ctx
                        .set_shadow_color(&format!("rgba(180,120,255,{})", alpha * 1.5));
                    self.ctx.set_shadow_blur(8.0);
                    self.ctx.begin_path();
                    self.ctx.arc(x, y, radius, 0.0, std::f64::consts::TAU).ok();
                    self.ctx.fill();
                }
                self.ctx.set_shadow_blur(0.0);
                self.ctx.set_shadow_color("transparent");
            }
            Some(crate::world::RoomModifier::Irradiated) => {
                self.ctx.set_fill_style_str("rgba(40,0,0,0.08)");
                self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);
                for i in 0..12 {
                    let seed = i as f64 * 0.69;
                    let x = (((anim_t * 0.33) + seed).sin() * 0.5 + 0.5) * self.canvas_w;
                    let y = (((anim_t * 1.2) + seed * 1.9).fract()) * self.canvas_h;
                    let len = 8.0 + (((anim_t * 1.1) + seed).sin() * 0.5 + 0.5) * 14.0;
                    self.ctx.set_stroke_style_str("rgba(255,90,90,0.22)");
                    self.ctx.set_line_width(1.5);
                    self.ctx.begin_path();
                    self.ctx.move_to(x, y);
                    self.ctx.line_to(x - 2.5, y + len);
                    self.ctx.stroke();
                }
            }
            Some(crate::world::RoomModifier::Hydroponics) => {
                // Floating green leaves drifting down
                self.ctx.set_fill_style_str("rgba(20,60,10,0.06)");
                self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);
                for i in 0..10 {
                    let seed = i as f64 * 1.27;
                    let x = (((anim_t * 0.22) + seed).sin() * 0.5 + 0.5) * self.canvas_w;
                    let y = (((anim_t * 0.35) + seed * 1.3).fract()) * self.canvas_h;
                    let alpha = 0.15 + (((anim_t * 1.5) + seed).sin() * 0.5 + 0.5) * 0.12;
                    let sz = 3.0 + (((anim_t * 0.8) + seed).cos() * 0.5 + 0.5) * 3.0;
                    self.ctx
                        .set_fill_style_str(&format!("rgba(80,180,60,{alpha})"));
                    self.ctx.begin_path();
                    // Leaf shape: two arcs
                    let _ = self.ctx.ellipse(
                        x,
                        y,
                        sz,
                        sz * 0.5,
                        (anim_t * 0.3 + seed).sin() * 0.5,
                        0.0,
                        std::f64::consts::TAU,
                    );
                    self.ctx.fill();
                }
            }
            Some(crate::world::RoomModifier::Cryogenic) => {
                // Blue-white frost overlay with snowflake particles
                self.ctx.set_fill_style_str("rgba(180,210,240,0.06)");
                self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);
                for i in 0..16 {
                    let seed = i as f64 * 0.91;
                    let x = (((anim_t * 0.15) + seed * 2.1).sin() * 0.5 + 0.5) * self.canvas_w;
                    let y = (((anim_t * 0.25) + seed * 0.8).fract()) * self.canvas_h;
                    let alpha = 0.2 + (((anim_t * 1.8) + seed).cos() * 0.5 + 0.5) * 0.15;
                    let r = 1.5 + (((anim_t * 1.2) + seed).sin() * 0.5 + 0.5) * 1.5;
                    self.ctx
                        .set_fill_style_str(&format!("rgba(220,240,255,{alpha})"));
                    self.ctx.begin_path();
                    self.ctx.arc(x, y, r, 0.0, std::f64::consts::TAU).ok();
                    self.ctx.fill();
                }
            }
            Some(crate::world::RoomModifier::OverheatedReactor) => {
                // Red-orange heat haze with rising embers
                self.ctx.set_fill_style_str("rgba(40,8,0,0.10)");
                self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);
                for i in 0..14 {
                    let seed = i as f64 * 0.77;
                    let x = (((anim_t * 0.38) + seed).sin() * 0.5 + 0.5) * self.canvas_w;
                    let y = self.canvas_h - (((anim_t * 0.6) + seed * 1.5).fract()) * self.canvas_h;
                    let alpha = 0.25 + (((anim_t * 2.5) + seed).sin() * 0.5 + 0.5) * 0.2;
                    let r = 1.0 + (((anim_t * 1.6) + seed).cos() * 0.5 + 0.5) * 2.0;
                    self.ctx
                        .set_fill_style_str(&format!("rgba(255,120,20,{alpha})"));
                    self.ctx
                        .set_shadow_color(&format!("rgba(255,80,10,{})", alpha * 1.2));
                    self.ctx.set_shadow_blur(6.0);
                    self.ctx.begin_path();
                    self.ctx.arc(x, y, r, 0.0, std::f64::consts::TAU).ok();
                    self.ctx.fill();
                }
                self.ctx.set_shadow_blur(0.0);
                self.ctx.set_shadow_color("transparent");
            }
            None => {}
        }
    }

    fn draw_minimap(&self, level: &DungeonLevel, player: &Player) {
        let mm_scale = 2.0;
        let mm_w = level.width as f64 * mm_scale;
        let mm_h = level.height as f64 * mm_scale;
        let mm_x = self.canvas_w - mm_w - 8.0;
        let mm_y = self.canvas_h - mm_h - 22.0;

        self.ctx.set_fill_style_str("rgba(5,3,10,0.75)");
        self.ctx
            .fill_rect(mm_x - 4.0, mm_y - 14.0, mm_w + 8.0, mm_h + 18.0);
        self.ctx.set_stroke_style_str("rgba(100,80,140,0.5)");
        self.ctx.set_line_width(1.0);
        self.ctx
            .stroke_rect(mm_x - 4.0, mm_y - 14.0, mm_w + 8.0, mm_h + 18.0);

        self.ctx.set_fill_style_str("rgba(140,120,180,0.6)");
        self.ctx.set_font("bold 8px monospace");
        self.ctx.set_text_align("center");
        self.ctx
            .fill_text("MAP", mm_x + mm_w / 2.0, mm_y - 4.0)
            .ok();
        self.ctx.set_text_align("left");

        for ty in 0..level.height {
            for tx in 0..level.width {
                let idx = level.idx(tx, ty);
                if !level.revealed[idx] {
                    continue;
                }
                let tile = level.tiles[idx];
                if tile == Tile::Bulkhead {
                    continue;
                }
                let px = mm_x + tx as f64 * mm_scale;
                let py = mm_y + ty as f64 * mm_scale;
                let color = if level.visible[idx] {
                    "rgba(150,140,180,0.7)"
                } else {
                    "rgba(80,70,100,0.5)"
                };
                self.ctx.set_fill_style_str(color);
                self.ctx.fill_rect(px, py, mm_scale, mm_scale);

                if level.revealed[idx] {
                    let poi_color = match tile {
                        Tile::Airlock => Some(COL_STAIRS),
                        Tile::QuantumForge => Some(COL_FORGE),
                        Tile::TradeTerminal => Some(COL_SHOP),
                        Tile::SupplyCrate => Some(COL_CHEST),
                        Tile::Terminal(_) => Some("#cc88ff"),
                        Tile::WarpGatePortal => Some("#ff44aa"),
                        Tile::MedBayTile => Some("#66ffdd"),
                        Tile::CreditCache => Some("#ffd700"),
                        Tile::DataRack => Some("#c49a6c"),
                        _ => None,
                    };
                    if let Some(c) = poi_color {
                        self.ctx.set_fill_style_str(c);
                        self.ctx.fill_rect(px, py, mm_scale, mm_scale);
                    }
                }
            }
        }

        self.ctx.set_fill_style_str(COL_PLAYER);
        self.ctx.fill_rect(
            mm_x + player.x as f64 * mm_scale - 0.5,
            mm_y + player.y as f64 * mm_scale - 0.5,
            mm_scale + 1.0,
            mm_scale + 1.0,
        );
    }
}
