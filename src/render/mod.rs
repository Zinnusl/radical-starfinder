//! Canvas 2D rendering for the station.

pub mod combat_hud;
pub mod helpers;
pub mod starmap;
pub mod tiles;
pub mod ui;

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

        let spirit_y = bar_y + bar_h + 4.0;
        let spirit_frac = (player.spirit as f64 / player.max_spirit as f64).clamp(0.0, 1.0);
        let spirit_color = if player.spirit < player.max_spirit / 6 {
            "#ff4444"
        } else {
            "#8844ff"
        };

        self.ctx.set_fill_style_str(COL_HP_BG);
        self.ctx.fill_rect(bar_x, spirit_y, bar_w, bar_h);
        self.ctx.set_fill_style_str(spirit_color);
        self.ctx
            .fill_rect(bar_x, spirit_y, bar_w * spirit_frac, bar_h);
        self.ctx.set_stroke_style_str("#666");
        self.ctx.set_line_width(1.0);
        self.ctx.stroke_rect(bar_x, spirit_y, bar_w, bar_h);

        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_font("12px monospace");
        self.ctx.set_text_align("left");
        self.ctx
            .fill_text(
                &format!("🌕 Spirit: {}/{}", player.spirit, player.max_spirit),
                bar_x + 4.0,
                spirit_y + 12.0,
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

        // ── Forge UI overlay ─────────────────────────────────────────────
        if let CombatState::Forging {
            ref recipes,
            cursor,
        } = combat
        {
            let visible_count = recipes.len().min(9);
            let row_h = 28.0;
            let box_w = 400.0;
            let box_h = 70.0 + visible_count as f64 * row_h;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 40.0;

            self.ctx.set_fill_style_str("rgba(30,15,10,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#ff8844");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#ff8844");
            self.ctx.set_font("18px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("⚒ Radical Forge ⚒", self.canvas_w / 2.0, box_y + 26.0)
                .ok();

            self.ctx.set_font("11px monospace");
            self.ctx.set_fill_style_str("#aaa");
            self.ctx
                .fill_text(
                    "↑/↓ browse  1-9 quick pick  Enter forge  E enchant  Esc close",
                    self.canvas_w / 2.0,
                    box_y + 44.0,
                )
                .ok();

            let cursor = *cursor;
            let scroll_offset = if cursor >= 9 { cursor - 8 } else { 0 };
            let list_y = box_y + 56.0;
            for vis_i in 0..recipes.len().min(9) {
                let abs_i = scroll_offset + vis_i;
                if abs_i >= recipes.len() {
                    break;
                }
                let recipe_idx = recipes[abs_i];
                let recipe = &radical::RECIPES[recipe_idx];
                let is_cursor = abs_i == cursor;
                let ry = list_y + vis_i as f64 * row_h;

                self.ctx.set_fill_style_str(if is_cursor {
                    "rgba(255,136,68,0.3)"
                } else {
                    "rgba(0,0,0,0.2)"
                });
                self.ctx
                    .fill_rect(box_x + 6.0, ry, box_w - 12.0, row_h - 2.0);
                if is_cursor {
                    self.ctx.set_stroke_style_str("#ffaa66");
                    self.ctx.set_line_width(1.0);
                    self.ctx
                        .stroke_rect(box_x + 6.0, ry, box_w - 12.0, row_h - 2.0);
                }

                let marker = if is_cursor { "►" } else { " " };
                let num = if vis_i < 9 {
                    format!("{}", vis_i + 1)
                } else {
                    " ".to_string()
                };
                self.ctx.set_text_align("left");
                self.ctx
                    .set_fill_style_str(if is_cursor { "#00ccdd" } else { "#888" });
                self.ctx.set_font("11px monospace");
                self.ctx
                    .fill_text(&format!("{}{}", marker, num), box_x + 10.0, ry + 17.0)
                    .ok();

                self.ctx
                    .set_fill_style_str(if is_cursor { "#00ccdd" } else { "#ffaa66" });
                self.ctx.set_font("16px 'Noto Serif SC', 'SimSun', serif");
                self.ctx
                    .fill_text(recipe.output_hanzi, box_x + 34.0, ry + 19.0)
                    .ok();

                let desc_x = box_x + 56.0;
                let icon_size = 16.0;
                let mut text_x = desc_x;
                if self.draw_sprite_icon(
                    spell_sprite_key(&recipe.effect),
                    desc_x,
                    ry + (row_h - icon_size) / 2.0 - 1.0,
                    icon_size,
                ) {
                    text_x += icon_size + 6.0;
                }

                self.ctx
                    .set_fill_style_str(if is_cursor { "#eeddbb" } else { "#aa9977" });
                self.ctx.set_font("11px monospace");
                let components = recipe.inputs.iter().copied().collect::<Vec<_>>().join("+");
                self.ctx
                    .fill_text(
                        &format!(
                            "{} ({}) — {} [{}]",
                            recipe.output_pinyin,
                            components,
                            recipe.output_meaning,
                            recipe.effect.label()
                        ),
                        text_x,
                        ry + 17.0,
                    )
                    .ok();
            }

            if recipes.len() > 9 {
                self.ctx.set_fill_style_str("#666");
                self.ctx.set_font("10px monospace");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(
                        &format!("{}/{} recipes (scroll with ↑/↓)", cursor + 1, recipes.len()),
                        self.canvas_w / 2.0,
                        list_y + 9.0 * row_h + 4.0,
                    )
                    .ok();
            }

            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("10px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    "Enter=forge  E=enchant  Esc=cancel",
                    self.canvas_w / 2.0,
                    box_y + box_h + 14.0,
                )
                .ok();
        }

        // ── Enchanting UI overlay (two-phase) ────────────────────────────
        if let CombatState::Enchanting { step, slot, page } = combat {
            if *step == 0 {
                // ── Phase 0: Select equipment slot ──────────────────────────
                let box_w = 340.0;
                let box_h = 140.0;
                let box_x = (self.canvas_w - box_w) / 2.0;
                let box_y = 60.0;

                self.ctx.set_fill_style_str("rgba(15,10,30,0.95)");
                self.ctx.fill_rect(box_x, box_y, box_w, box_h);
                self.ctx.set_stroke_style_str("#aa66ff");
                self.ctx.set_line_width(2.0);
                self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

                // Title
                self.ctx.set_fill_style_str("#aa66ff");
                self.ctx.set_font("18px monospace");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text("✦ Enchant Equipment ✦", self.canvas_w / 2.0, box_y + 26.0)
                    .ok();

                self.ctx.set_fill_style_str("#aaa");
                self.ctx.set_font("11px monospace");
                self.ctx
                    .fill_text(
                        "Select a slot to enchant",
                        self.canvas_w / 2.0,
                        box_y + 44.0,
                    )
                    .ok();

                // Equipment slots
                let slots: [(&str, Option<&str>, Option<&str>); 3] = [
                    (
                        "1: Weapon",
                        player.weapon.map(|e| e.name),
                        player.enchantments[0],
                    ),
                    (
                        "2: Armor",
                        player.armor.map(|e| e.name),
                        player.enchantments[1],
                    ),
                    (
                        "3: Charm",
                        player.charm.map(|e| e.name),
                        player.enchantments[2],
                    ),
                ];
                let slot_y = box_y + 62.0;
                for (i, (label, equip_name, ench)) in slots.iter().enumerate() {
                    let is_selected = i == *slot;
                    let has_equip = equip_name.is_some();
                    let color = if is_selected {
                        "#00ccdd"
                    } else if has_equip {
                        "#ccc"
                    } else {
                        "#555"
                    };
                    self.ctx.set_fill_style_str(color);
                    self.ctx.set_font("13px monospace");
                    self.ctx.set_text_align("left");
                    let marker = if is_selected { "▸ " } else { "  " };
                    let eq_name = equip_name.unwrap_or("(empty)");
                    let ench_str = ench.map(|e| format!(" [{}]", e)).unwrap_or_default();
                    self.ctx
                        .fill_text(
                            &format!("{}{} {}{}", marker, label, eq_name, ench_str),
                            box_x + 20.0,
                            slot_y + i as f64 * 22.0,
                        )
                        .ok();
                }

                // Bottom hint
                self.ctx.set_fill_style_str("#666");
                self.ctx.set_font("10px monospace");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(
                        "1-3 or ↑↓+Enter = pick slot   Esc = cancel",
                        self.canvas_w / 2.0,
                        box_y + box_h + 14.0,
                    )
                    .ok();
            } else {
                // ── Phase 1: Select radical to apply ────────────────────────
                let rad_count = player.radicals.len();
                let page_size: usize = 6;
                let page_start = page * page_size;
                let page_end = (page_start + page_size).min(rad_count);
                let page_count = page_end - page_start;
                let max_page = if rad_count == 0 {
                    0
                } else {
                    (rad_count - 1) / page_size
                };

                let box_w = 380.0;
                let box_h = 120.0 + (page_count as f64 / 3.0).ceil() * 40.0;
                let box_x = (self.canvas_w - box_w) / 2.0;
                let box_y = 40.0;

                self.ctx.set_fill_style_str("rgba(15,10,30,0.95)");
                self.ctx.fill_rect(box_x, box_y, box_w, box_h);
                self.ctx.set_stroke_style_str("#aa66ff");
                self.ctx.set_line_width(2.0);
                self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

                // Title
                self.ctx.set_fill_style_str("#aa66ff");
                self.ctx.set_font("18px monospace");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text("✦ Enchant Equipment ✦", self.canvas_w / 2.0, box_y + 26.0)
                    .ok();

                let slot_label = match slot {
                    0 => "Weapon",
                    1 => "Armor",
                    _ => "Charm",
                };
                let equip_name = match slot {
                    0 => player.weapon.map(|e| e.name).unwrap_or("—"),
                    1 => player.armor.map(|e| e.name).unwrap_or("—"),
                    _ => player.charm.map(|e| e.name).unwrap_or("—"),
                };
                self.ctx.set_fill_style_str("#00ccdd");
                self.ctx.set_font("12px monospace");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(
                        &format!("Enchanting: {} ({})", slot_label, equip_name),
                        self.canvas_w / 2.0,
                        box_y + 48.0,
                    )
                    .ok();

                self.ctx.set_fill_style_str("#aaa");
                self.ctx.set_font("11px monospace");
                self.ctx
                    .fill_text(
                        &format!("Pick radical (page {}/{})", page + 1, max_page + 1),
                        self.canvas_w / 2.0,
                        box_y + 66.0,
                    )
                    .ok();

                let grid_y = box_y + 78.0;
                for (i, abs_idx) in (page_start..page_end).enumerate() {
                    let rad_ch = player.radicals[abs_idx];
                    let col = i % 3;
                    let row = i / 3;
                    let rx = box_x + 20.0 + col as f64 * 120.0;
                    let ry = grid_y + row as f64 * 40.0;

                    self.ctx.set_fill_style_str("rgba(0,0,0,0.3)");
                    self.ctx.fill_rect(rx, ry, 110.0, 34.0);
                    self.ctx.set_stroke_style_str("#aa66ff");
                    self.ctx.set_line_width(1.0);
                    self.ctx.stroke_rect(rx, ry, 110.0, 34.0);

                    self.ctx.set_fill_style_str("#00ccdd");
                    self.ctx.set_font("11px monospace");
                    self.ctx.set_text_align("left");
                    self.ctx
                        .fill_text(&format!("{}:", i + 1), rx + 4.0, ry + 14.0)
                        .ok();

                    self.ctx.set_fill_style_str("#cc99ff");
                    self.ctx.set_font("20px 'Noto Serif SC', 'SimSun', serif");
                    self.ctx.set_text_align("center");
                    self.ctx.fill_text(rad_ch, rx + 55.0, ry + 26.0).ok();
                }

                if rad_count == 0 {
                    self.ctx.set_fill_style_str("#666");
                    self.ctx.set_font("12px monospace");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text(
                            "No radicals collected yet!",
                            self.canvas_w / 2.0,
                            grid_y + 20.0,
                        )
                        .ok();
                }

                // Bottom hint
                self.ctx.set_fill_style_str("#666");
                self.ctx.set_font("10px monospace");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(
                        "1-6 = pick radical   ←/→ = page   Esc = back",
                        self.canvas_w / 2.0,
                        box_y + box_h + 14.0,
                    )
                    .ok();
            }
        }

        // ── Shop UI overlay ─────────────────────────────────────────────
        if let CombatState::Shopping { ref items, cursor } = combat {
            let display_items_len = if shop_sell_mode {
                player.items.len()
            } else {
                items.len()
            };
            let box_w = 350.0;
            let box_h = 60.0 + display_items_len.max(1) as f64 * 28.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 50.0;

            // Background
            self.ctx.set_fill_style_str("rgba(10,30,20,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str(if shop_sell_mode { "#dd8844" } else { "#44dd88" });
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            // Title
            self.ctx.set_fill_style_str(if shop_sell_mode { "#dd8844" } else { "#44dd88" });
            self.ctx.set_font("18px monospace");
            self.ctx.set_text_align("center");
            let title = if shop_sell_mode { "$ Sell Items $" } else { "$ Shop $" };
            self.ctx
                .fill_text(title, self.canvas_w / 2.0, box_y + 26.0)
                .ok();

            // Gold display
            self.ctx.set_fill_style_str("#ffdd44");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(
                    &format!("Your gold: {}", player.gold),
                    self.canvas_w / 2.0,
                    box_y + 42.0,
                )
                .ok();

            if shop_sell_mode {
                // Sell mode: show player inventory with sell prices
                if player.items.is_empty() {
                    let y = box_y + 60.0;
                    self.ctx.set_fill_style_str("#666");
                    self.ctx.set_font("13px monospace");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text("No items to sell", self.canvas_w / 2.0, y + 10.0)
                        .ok();
                } else {
                    for (i, item) in player.items.iter().enumerate() {
                        let y = box_y + 60.0 + i as f64 * 28.0;
                        let selected = i == *cursor;

                        if selected {
                            self.ctx.set_fill_style_str("rgba(221,136,68,0.15)");
                            self.ctx
                                .fill_rect(box_x + 10.0, y - 6.0, box_w - 20.0, 24.0);
                        }

                        let marker = if selected { "►" } else { " " };
                        let sell_price = item.sell_price();
                        let label = if i < item_labels.len() {
                            &item_labels[i]
                        } else {
                            item.name()
                        };
                        self.ctx.set_fill_style_str("#ffcc99");
                        self.ctx.set_font("13px monospace");
                        self.ctx.set_text_align("left");
                        let price_label = format!("{} {} — {}g", marker, label, sell_price);
                        let mut text_x = box_x + 15.0;
                        let icon_key = item_sprite_key(item);
                        if self.draw_sprite_icon(icon_key, box_x + 15.0, y - 4.0, 16.0) {
                            text_x += 20.0;
                        }
                        self.ctx.fill_text(&price_label, text_x, y + 10.0).ok();
                    }
                }
            } else {
                // Buy mode: show shop items (existing logic)
                for (i, item) in items.iter().enumerate() {
                    let y = box_y + 60.0 + i as f64 * 28.0;
                    let selected = i == *cursor;

                    // Selection highlight
                    if selected {
                        self.ctx.set_fill_style_str("rgba(68,221,136,0.15)");
                        self.ctx
                            .fill_rect(box_x + 10.0, y - 6.0, box_w - 20.0, 24.0);
                    }

                    let marker = if selected { "►" } else { " " };
                    let companion_discount = companion
                        .map(|c| c.shop_discount_pct(companion_level))
                        .unwrap_or(0);
                    let total_discount = (player.shop_discount_pct + companion_discount).clamp(0, 50);
                    let display_cost = ((item.cost * (100 - total_discount)) + 99) / 100;
                    let can_afford = player.gold >= display_cost;
                    self.ctx
                        .set_fill_style_str(if can_afford { "#ccffcc" } else { "#666" });
                    self.ctx.set_font("13px monospace");
                    self.ctx.set_text_align("left");
                    let price_label = if total_discount > 0 {
                        format!(
                            "{} {} — {}g ({}% off)",
                            marker, item.label, display_cost, total_discount
                        )
                    } else {
                        format!("{} {} — {}g", marker, item.label, item.cost)
                    };
                    let mut text_x = box_x + 15.0;
                    if let Some(icon_key) = shop_item_sprite_key(&item.kind) {
                        if self.draw_sprite_icon(icon_key, box_x + 15.0, y - 4.0, 16.0) {
                            text_x += 20.0;
                        }
                    }
                    self.ctx.fill_text(&price_label, text_x, y + 10.0).ok();
                }
            }

            // Hint
            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx.set_text_align("center");
            let has_reroll =
                companion == Some(crate::game::Companion::Quartermaster) && companion_level >= 3;
            let hint_text = if shop_sell_mode {
                "↑↓=browse  Enter=sell  Tab=buy mode  Esc=leave"
            } else if has_reroll {
                "↑↓=browse  Enter=buy  Tab=sell  R=reroll  Esc=leave"
            } else {
                "↑↓=browse  Enter=buy  Tab=sell  Esc=leave"
            };
            self.ctx
                .fill_text(hint_text, self.canvas_w / 2.0, box_y + box_h + 14.0)
                .ok();
        }

        // ── Offering / Altar overlay ────────────────────────────────────
        if let CombatState::Offering { altar_kind, cursor } = combat {
            self.draw_offering_overlay(player, item_labels, *altar_kind, *cursor);
        }

        // ── Dipping Source overlay ──────────────────────────────────────
        if let CombatState::DippingSource { cursor } = combat {
            self.draw_dipping_source_overlay(player, item_labels, *cursor);
        }

        // ── Dipping Target overlay ──────────────────────────────────────
        if let CombatState::DippingTarget { source_idx, cursor } = combat {
            self.draw_dipping_target_overlay(player, item_labels, *source_idx, *cursor);
        }

        // ── Sentence Challenge overlay ──────────────────────────────────
        if let CombatState::SentenceChallenge {
            ref tiles,
            ref words,
            cursor,
            ref arranged,
            meaning,
            ..
        } = combat
        {
            let box_w = 380.0;
            let box_h = 200.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 40.0 + (anim_t * 3.2).sin() * 3.0;

            self.ctx.set_fill_style_str("rgba(15,10,30,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#ff8866");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            // Title
            self.ctx.set_fill_style_str("#ff8866");
            self.ctx.set_font("bold 14px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    "Boss Phase 2 — Arrange the Sentence!",
                    self.canvas_w / 2.0,
                    box_y + 22.0,
                )
                .ok();

            // Meaning hint
            self.ctx.set_fill_style_str("#999");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(
                    &format!("Meaning: {}", meaning),
                    self.canvas_w / 2.0,
                    box_y + 42.0,
                )
                .ok();

            // Arranged so far
            let arranged_text: String = arranged
                .iter()
                .map(|&i| words[i])
                .collect::<Vec<_>>()
                .join(" ");
            self.ctx.set_fill_style_str("#66ff66");
            self.ctx.set_font("20px 'Noto Serif SC', serif");
            self.ctx
                .fill_text(
                    if arranged_text.is_empty() {
                        "..."
                    } else {
                        &arranged_text
                    },
                    self.canvas_w / 2.0,
                    box_y + 75.0,
                )
                .ok();

            // Remaining tiles
            let remaining: Vec<usize> = tiles
                .iter()
                .copied()
                .filter(|t| !arranged.contains(t))
                .collect();
            let tile_w = 60.0;
            let total_w = remaining.len() as f64 * tile_w;
            let start_x = (self.canvas_w - total_w) / 2.0;
            for (i, &word_idx) in remaining.iter().enumerate() {
                let tx = start_x + i as f64 * tile_w;
                let ty = box_y + 100.0;
                let selected = i == *cursor;
                self.ctx.set_fill_style_str(if selected {
                    "rgba(100,80,160,0.8)"
                } else {
                    "rgba(40,30,60,0.8)"
                });
                self.ctx.fill_rect(tx + 2.0, ty, tile_w - 4.0, 36.0);
                self.ctx
                    .set_stroke_style_str(if selected { "#00ccdd" } else { "#555" });
                self.ctx.set_line_width(if selected { 2.0 } else { 1.0 });
                self.ctx.stroke_rect(tx + 2.0, ty, tile_w - 4.0, 36.0);
                self.ctx
                    .set_fill_style_str(if selected { "#00ccdd" } else { "#ccccee" });
                self.ctx.set_font("16px 'Noto Serif SC', serif");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(words[word_idx], tx + tile_w / 2.0, ty + 24.0)
                    .ok();
            }

            // Controls hint
            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    "←→ select  Enter=pick  Backspace=undo  Esc=skip",
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Stroke Order overlay ──────────────────────────────────────
        if let CombatState::StrokeOrder {
            hanzi,
            ref components,
            correct_order: _,
            cursor,
            ref arranged,
            pinyin,
            meaning,
        } = combat
        {
            let box_w = 320.0;
            let box_h = 220.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 3.0).sin() * 2.0;

            self.ctx.set_fill_style_str("rgba(15,20,40,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#88ccff");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#88ccff");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("筆 Stroke Order", self.canvas_w / 2.0, box_y + 24.0)
                .ok();

            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.set_font("42px 'Noto Serif SC', 'SimSun', serif");
            self.ctx
                .fill_text(hanzi, self.canvas_w / 2.0, box_y + 72.0)
                .ok();

            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("11px monospace");
            self.ctx
                .fill_text(
                    &format!("{} — {}", pinyin, meaning),
                    self.canvas_w / 2.0,
                    box_y + 88.0,
                )
                .ok();

            let built: String = arranged.iter().copied().collect::<Vec<_>>().join(" + ");
            self.ctx.set_fill_style_str("#88ccff");
            self.ctx.set_font("14px monospace");
            self.ctx
                .fill_text(
                    &format!("Built: [{}]", built),
                    self.canvas_w / 2.0,
                    box_y + 110.0,
                )
                .ok();

            let remaining: Vec<&&str> = components
                .iter()
                .filter(|c| !arranged.contains(c))
                .collect();
            self.ctx.set_font("16px 'Noto Serif SC', serif");
            for (i, part) in remaining.iter().enumerate() {
                let y = box_y + 135.0 + i as f64 * 22.0;
                let selected = i == *cursor;
                self.ctx
                    .set_fill_style_str(if selected { "#00ccdd" } else { "#ccccee" });
                let marker = if selected { "▸ " } else { "  " };
                self.ctx
                    .fill_text(&format!("{}{}", marker, part), self.canvas_w / 2.0, y)
                    .ok();
            }

            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    "↑↓ select  Enter=place  Backspace=undo  Esc=skip",
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Tone Defense overlay ────────────────────────────────────────
        if let CombatState::ToneDefense {
            round,
            hanzi,
            pinyin: _,
            meaning: _,
            correct_tone: _,
            score,
            last_result,
        } = combat
        {
            let box_w = 320.0;
            let box_h = 200.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 3.0).sin() * 2.0;

            self.ctx.set_fill_style_str("rgba(30,15,15,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#dd6644");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#dd6644");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    &format!("壁 Tone Wall — Round {}/5", round + 1),
                    self.canvas_w / 2.0,
                    box_y + 24.0,
                )
                .ok();

            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.set_font("42px 'Noto Serif SC', 'SimSun', serif");
            self.ctx
                .fill_text(hanzi, self.canvas_w / 2.0, box_y + 75.0)
                .ok();

            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(
                    &format!("Blocked: {}/{}", score, round + 1),
                    self.canvas_w / 2.0,
                    box_y + 95.0,
                )
                .ok();

            let tones = [
                "1: ā (flat)",
                "2: á (rising)",
                "3: ǎ (dip)",
                "4: à (falling)",
            ];
            self.ctx.set_font("14px monospace");
            for (i, label) in tones.iter().enumerate() {
                let y = box_y + 115.0 + i as f64 * 18.0;
                self.ctx.set_fill_style_str("#ccccee");
                self.ctx.fill_text(label, self.canvas_w / 2.0, y).ok();
            }

            if let Some(was_correct) = last_result {
                let (txt, col) = if *was_correct {
                    ("✓", "#66ff66")
                } else {
                    ("✗", "#ff6666")
                };
                self.ctx.set_fill_style_str(col);
                self.ctx.set_font("20px monospace");
                self.ctx.fill_text(txt, box_x + 20.0, box_y + 24.0).ok();
            }

            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    "1-4 pick tone  Esc=flee  Wrong = -1 HP",
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Compound Builder overlay ────────────────────────────────────
        if let CombatState::CompoundBuilder {
            ref parts,
            correct_compound: _,
            pinyin: _,
            meaning,
            cursor,
            ref arranged,
        } = combat
        {
            let box_w = 320.0;
            let box_h = 220.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 3.0).sin() * 2.0;

            self.ctx.set_fill_style_str("rgba(15,30,20,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#66dd88");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#66dd88");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("合 Compound Builder", self.canvas_w / 2.0, box_y + 24.0)
                .ok();

            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(
                    &format!("Hint: {}", meaning),
                    self.canvas_w / 2.0,
                    box_y + 44.0,
                )
                .ok();

            let built: String = arranged.iter().copied().collect::<Vec<_>>().join("");
            self.ctx.set_fill_style_str("#66dd88");
            self.ctx.set_font("28px 'Noto Serif SC', serif");
            self.ctx
                .fill_text(
                    &format!("[{}]", if built.is_empty() { "?" } else { &built }),
                    self.canvas_w / 2.0,
                    box_y + 80.0,
                )
                .ok();

            let remaining: Vec<&&str> = parts.iter().filter(|p| !arranged.contains(p)).collect();
            self.ctx.set_font("18px 'Noto Serif SC', serif");
            for (i, part) in remaining.iter().enumerate() {
                let y = box_y + 115.0 + i as f64 * 26.0;
                let selected = i == *cursor;
                self.ctx
                    .set_fill_style_str(if selected { "#00ccdd" } else { "#ccccee" });
                let marker = if selected { "▸ " } else { "  " };
                self.ctx
                    .fill_text(&format!("{}{}", marker, part), self.canvas_w / 2.0, y)
                    .ok();
            }

            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    "↑↓ select  Enter=place  Backspace=undo  Esc=skip",
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Classifier Match overlay ────────────────────────────────────
        if let CombatState::ClassifierMatch {
            round,
            noun,
            noun_pinyin: _,
            noun_meaning,
            correct_classifier: _,
            ref options,
            correct_idx: _,
            score,
            last_result,
        } = combat
        {
            let box_w = 320.0;
            let box_h = 200.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 3.0).sin() * 2.0;

            self.ctx.set_fill_style_str("rgba(30,25,10,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#ddaa44");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#ddaa44");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    &format!("量 Classifier — Round {}/3", round + 1),
                    self.canvas_w / 2.0,
                    box_y + 24.0,
                )
                .ok();

            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.set_font("36px 'Noto Serif SC', 'SimSun', serif");
            self.ctx
                .fill_text(noun, self.canvas_w / 2.0, box_y + 68.0)
                .ok();

            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("11px monospace");
            self.ctx
                .fill_text(noun_meaning, self.canvas_w / 2.0, box_y + 85.0)
                .ok();

            self.ctx.set_fill_style_str("#ccccee");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(
                    &format!("Score: {}/{}", score, round + 1),
                    self.canvas_w / 2.0,
                    box_y + 100.0,
                )
                .ok();

            self.ctx.set_font("16px 'Noto Serif SC', serif");
            for (i, opt) in options.iter().enumerate() {
                let y = box_y + 122.0 + i as f64 * 22.0;
                self.ctx.set_fill_style_str("#ccccee");
                self.ctx
                    .fill_text(&format!("{}: {}", i + 1, opt), self.canvas_w / 2.0, y)
                    .ok();
            }

            if let Some(was_correct) = last_result {
                let (txt, col) = if *was_correct {
                    ("✓", "#66ff66")
                } else {
                    ("✗", "#ff6666")
                };
                self.ctx.set_fill_style_str(col);
                self.ctx.set_font("20px monospace");
                self.ctx.fill_text(txt, box_x + 20.0, box_y + 24.0).ok();
            }

            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    "1-4 pick classifier  Esc=flee  5g per correct",
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── InkWell overlay ─────────────────────────────────────────────
        if let CombatState::InkWellChallenge {
            hanzi,
            correct_count: _,
            pinyin,
            meaning,
        } = combat
        {
            let box_w = 300.0;
            let box_h = 160.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 3.0).sin() * 2.0;

            self.ctx.set_fill_style_str("rgba(20,20,40,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#9999ee");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#9999ee");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("墨 Ink Well", self.canvas_w / 2.0, box_y + 24.0)
                .ok();

            self.ctx.set_fill_style_str("#eeeeff");
            self.ctx.set_font("36px 'Noto Serif SC', 'SimSun', serif");
            self.ctx
                .fill_text(hanzi, self.canvas_w / 2.0, box_y + 70.0)
                .ok();

            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(
                    &format!("{} — {}", pinyin, meaning),
                    self.canvas_w / 2.0,
                    box_y + 92.0,
                )
                .ok();

            self.ctx.set_fill_style_str("#ccccee");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(
                    "How many components? Press 1-9",
                    self.canvas_w / 2.0,
                    box_y + 116.0,
                )
                .ok();

            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    "Correct = +1 HP  Esc=leave",
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Ancestor Shrine overlay ─────────────────────────────────────
        if let CombatState::AncestorChallenge {
            first_half,
            correct_second: _,
            full: _,
            pinyin: _,
            meaning,
            ref options,
            correct_idx: _,
        } = combat
        {
            let box_w = 320.0;
            let box_h = 200.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 3.0).sin() * 2.0;

            self.ctx.set_fill_style_str("rgba(40,20,20,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#ee9966");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#ee9966");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("祖 Ancestor Shrine", self.canvas_w / 2.0, box_y + 24.0)
                .ok();

            self.ctx.set_fill_style_str("#ffcc88");
            self.ctx.set_font("28px 'Noto Serif SC', 'SimSun', serif");
            self.ctx
                .fill_text(
                    &format!("{}____", first_half),
                    self.canvas_w / 2.0,
                    box_y + 65.0,
                )
                .ok();

            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("11px monospace");
            self.ctx
                .fill_text(meaning, self.canvas_w / 2.0, box_y + 85.0)
                .ok();

            self.ctx.set_font("16px 'Noto Serif SC', serif");
            for (i, opt) in options.iter().enumerate() {
                let y = box_y + 110.0 + i as f64 * 22.0;
                self.ctx.set_fill_style_str("#ccccee");
                self.ctx
                    .fill_text(&format!("{}: {}", i + 1, opt), self.canvas_w / 2.0, y)
                    .ok();
            }

            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    "1-4 complete chengyu  Correct=+10g  Esc=leave",
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Translation Altar overlay ───────────────────────────────────
        if let CombatState::TranslationChallenge {
            round,
            meaning,
            correct_hanzi: _,
            correct_pinyin: _,
            ref options,
            correct_idx: _,
            score,
        } = combat
        {
            let box_w = 320.0;
            let box_h = 210.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 3.0).sin() * 2.0;

            self.ctx.set_fill_style_str("rgba(20,40,40,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#66cccc");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#66cccc");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    &format!("译 Translation — Round {}/3", round + 1),
                    self.canvas_w / 2.0,
                    box_y + 24.0,
                )
                .ok();

            self.ctx.set_fill_style_str("#ffffff");
            self.ctx.set_font("18px monospace");
            self.ctx
                .fill_text(
                    &format!("\"{}\"", meaning),
                    self.canvas_w / 2.0,
                    box_y + 58.0,
                )
                .ok();

            self.ctx.set_fill_style_str("#ccccee");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(
                    &format!("Score: {}/{}", score, round + 1),
                    self.canvas_w / 2.0,
                    box_y + 80.0,
                )
                .ok();

            self.ctx.set_font("18px 'Noto Serif SC', serif");
            for (i, opt) in options.iter().enumerate() {
                let y = box_y + 106.0 + i as f64 * 24.0;
                self.ctx.set_fill_style_str("#ccccee");
                self.ctx
                    .fill_text(&format!("{}: {}", i + 1, opt), self.canvas_w / 2.0, y)
                    .ok();
            }

            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    "1-4 pick  2+ correct=+1 max HP  Esc=leave",
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Radical Garden overlay ──────────────────────────────────────
        if let CombatState::RadicalGardenChallenge {
            hanzi,
            pinyin: _,
            meaning,
            correct_radical: _,
            ref options,
            correct_idx: _,
        } = combat
        {
            let box_w = 310.0;
            let box_h = 200.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 3.0).sin() * 2.0;

            self.ctx.set_fill_style_str("rgba(20,40,20,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#88ee66");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#88ee66");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("部 Radical Garden", self.canvas_w / 2.0, box_y + 24.0)
                .ok();

            self.ctx.set_fill_style_str("#aaffaa");
            self.ctx.set_font("36px 'Noto Serif SC', 'SimSun', serif");
            self.ctx
                .fill_text(hanzi, self.canvas_w / 2.0, box_y + 70.0)
                .ok();

            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("11px monospace");
            self.ctx
                .fill_text(meaning, self.canvas_w / 2.0, box_y + 88.0)
                .ok();

            self.ctx.set_font("18px 'Noto Serif SC', serif");
            for (i, opt) in options.iter().enumerate() {
                let y = box_y + 112.0 + i as f64 * 22.0;
                self.ctx.set_fill_style_str("#cceecc");
                self.ctx
                    .fill_text(&format!("{}: {}", i + 1, opt), self.canvas_w / 2.0, y)
                    .ok();
            }

            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    "1-4 identify radical  Correct=free radical  Esc=leave",
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Mirror Pool overlay ─────────────────────────────────────────
        if let CombatState::MirrorPoolChallenge {
            hanzi,
            correct_pinyin: _,
            meaning,
            ref input,
        } = combat
        {
            let box_w = 310.0;
            let box_h = 180.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 3.0).sin() * 2.0;

            self.ctx.set_fill_style_str("rgba(20,20,50,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#aaaaff");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#aaaaff");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("鏡 Mirror Pool", self.canvas_w / 2.0, box_y + 24.0)
                .ok();

            self.ctx.set_fill_style_str("#ddddff");
            self.ctx.set_font("36px 'Noto Serif SC', 'SimSun', serif");
            self.ctx
                .fill_text(hanzi, self.canvas_w / 2.0, box_y + 70.0)
                .ok();

            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(meaning, self.canvas_w / 2.0, box_y + 90.0)
                .ok();

            self.ctx.set_fill_style_str("#ffffff");
            self.ctx.set_font("18px monospace");
            let display_input = if input.is_empty() {
                "_ ".to_string()
            } else {
                format!("{}▏", input)
            };
            self.ctx
                .fill_text(&display_input, self.canvas_w / 2.0, box_y + 120.0)
                .ok();

            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    "Type pinyin  Enter=submit  Backspace=del  Esc=leave",
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Stone Tutor overlay ─────────────────────────────────────────
        if let CombatState::StoneTutorChallenge {
            round,
            hanzi,
            pinyin,
            meaning,
            correct_tone: _,
            phase,
            score,
        } = combat
        {
            let box_w = 310.0;
            let box_h = 200.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 3.0).sin() * 2.0;

            self.ctx.set_fill_style_str("rgba(40,40,20,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#cccc66");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#cccc66");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    &format!("石 Stone Tutor — Round {}/3", round + 1),
                    self.canvas_w / 2.0,
                    box_y + 24.0,
                )
                .ok();

            self.ctx.set_fill_style_str("#ffffaa");
            self.ctx.set_font("36px 'Noto Serif SC', 'SimSun', serif");
            self.ctx
                .fill_text(hanzi, self.canvas_w / 2.0, box_y + 70.0)
                .ok();

            if *phase == 0 {
                self.ctx.set_fill_style_str("#ccccaa");
                self.ctx.set_font("14px monospace");
                self.ctx
                    .fill_text(
                        &format!("{} — {}", pinyin, meaning),
                        self.canvas_w / 2.0,
                        box_y + 95.0,
                    )
                    .ok();

                self.ctx.set_fill_style_str("#aaaaaa");
                self.ctx.set_font("12px monospace");
                self.ctx
                    .fill_text(
                        "Study this character. Press Space to quiz.",
                        self.canvas_w / 2.0,
                        box_y + 125.0,
                    )
                    .ok();
            } else {
                self.ctx.set_fill_style_str("#ccccaa");
                self.ctx.set_font("14px monospace");
                self.ctx
                    .fill_text(meaning, self.canvas_w / 2.0, box_y + 92.0)
                    .ok();

                self.ctx.set_fill_style_str("#ccccee");
                self.ctx.set_font("14px monospace");
                self.ctx
                    .fill_text("What tone? 1-4", self.canvas_w / 2.0, box_y + 116.0)
                    .ok();

                self.ctx.set_font("12px monospace");
                for i in 1..=4u8 {
                    let label = match i {
                        1 => "1: ˉ flat",
                        2 => "2: ˊ rising",
                        3 => "3: ˇ dip",
                        _ => "4: ˋ falling",
                    };
                    let y = box_y + 134.0 + (i - 1) as f64 * 16.0;
                    self.ctx.set_fill_style_str("#aaaacc");
                    self.ctx.fill_text(label, self.canvas_w / 2.0, y).ok();
                }
            }

            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    &format!("Score: {}/{}  Esc=leave", score, round),
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Codex Challenge overlay ─────────────────────────────────────
        if let CombatState::CodexChallenge {
            round,
            hanzi,
            pinyin: _,
            meaning: _,
            options,
            correct_idx: _,
            score,
        } = combat
        {
            let box_w = 340.0;
            let box_h = 220.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 3.0).sin() * 2.0;

            self.ctx.set_fill_style_str("rgba(25,15,45,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#cc88ff");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#cc88ff");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    &format!("典 Codex Shrine — Round {}/3", round + 1),
                    self.canvas_w / 2.0,
                    box_y + 24.0,
                )
                .ok();

            self.ctx.set_fill_style_str("#eeddff");
            self.ctx.set_font("36px 'Noto Serif SC', 'SimSun', serif");
            self.ctx
                .fill_text(hanzi, self.canvas_w / 2.0, box_y + 72.0)
                .ok();

            self.ctx.set_fill_style_str("#bbaadd");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text("What does this mean?", self.canvas_w / 2.0, box_y + 92.0)
                .ok();

            self.ctx.set_font("14px monospace");
            for (i, opt) in options.iter().enumerate() {
                let y = box_y + 114.0 + i as f64 * 20.0;
                self.ctx.set_fill_style_str("#ddccee");
                self.ctx
                    .fill_text(&format!("{}: {}", i + 1, opt), self.canvas_w / 2.0, y)
                    .ok();
            }

            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    &format!("Score: {}/{}  Esc=leave", score, round),
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Journal overlay ─────────────────────────────────────────────
        if let CombatState::Journal { page } = combat {
            let box_w = 360.0;
            let box_h = 300.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 30.0;

            self.ctx.set_fill_style_str("rgba(12,10,28,0.96)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#88aaff");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#88aaff");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");

            let entries = codex.sorted_entries();
            let total = entries.len();
            let per_page = 10;
            let pages = if total == 0 {
                1
            } else {
                (total + per_page - 1) / per_page
            };
            let cur_page = *page;

            self.ctx
                .fill_text(
                    &format!("📖 Character Journal — {}/{}", cur_page + 1, pages),
                    self.canvas_w / 2.0,
                    box_y + 24.0,
                )
                .ok();

            if total == 0 {
                self.ctx.set_fill_style_str("#777");
                self.ctx.set_font("14px monospace");
                self.ctx
                    .fill_text(
                        "No characters encountered yet.",
                        self.canvas_w / 2.0,
                        box_y + 100.0,
                    )
                    .ok();
            } else {
                self.ctx.set_text_align("left");
                self.ctx.set_fill_style_str("#667799");
                self.ctx.set_font("10px monospace");
                self.ctx
                    .fill_text(
                        "Char  Pinyin        Meaning          Acc",
                        box_x + 14.0,
                        box_y + 44.0,
                    )
                    .ok();

                let start = cur_page * per_page;
                let end = (start + per_page).min(total);
                for (i, entry) in entries[start..end].iter().enumerate() {
                    let y = box_y + 62.0 + i as f64 * 22.0;

                    self.ctx.set_fill_style_str("#eeddff");
                    self.ctx.set_font("16px 'Noto Serif SC', 'SimSun', serif");
                    self.ctx.fill_text(entry.hanzi, box_x + 14.0, y).ok();

                    self.ctx.set_fill_style_str("#aabbcc");
                    self.ctx.set_font("11px monospace");
                    self.ctx.fill_text(entry.pinyin, box_x + 50.0, y).ok();

                    self.ctx.set_fill_style_str("#99aabb");
                    self.ctx.fill_text(entry.meaning, box_x + 145.0, y).ok();

                    let acc = (entry.accuracy() * 100.0) as u32;
                    let acc_color = if acc >= 80 {
                        "#88ff88"
                    } else if acc >= 50 {
                        "#ffcc44"
                    } else {
                        "#ff6666"
                    };
                    self.ctx.set_fill_style_str(acc_color);
                    self.ctx
                        .fill_text(&format!("{}%", acc), box_x + 290.0, y)
                        .ok();
                }
            }

            self.ctx.set_text_align("center");
            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    "←/→ page  Esc/J=close",
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Word Bridge Challenge overlay ───────────────────────────────
        if let CombatState::WordBridgeChallenge {
            meaning,
            correct_hanzi: _,
            correct_pinyin: _,
            options,
            correct_idx: _,
            bridge_x: _,
            bridge_y: _,
        } = combat
        {
            let box_w = 340.0;
            let box_h = 200.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 3.0).sin() * 2.0;

            self.ctx.set_fill_style_str("rgba(15,30,40,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#44ccaa");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#44ccaa");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("桥 Word Bridge", self.canvas_w / 2.0, box_y + 24.0)
                .ok();

            self.ctx.set_fill_style_str("#ddeeff");
            self.ctx.set_font("18px monospace");
            self.ctx
                .fill_text(
                    &format!("Which character means \"{}\"?", meaning),
                    self.canvas_w / 2.0,
                    box_y + 58.0,
                )
                .ok();

            self.ctx.set_font("20px 'Noto Serif SC', 'SimSun', serif");
            for (i, opt) in options.iter().enumerate() {
                let y = box_y + 90.0 + i as f64 * 24.0;
                self.ctx.set_fill_style_str("#ccffee");
                self.ctx
                    .fill_text(&format!("{}: {}", i + 1, opt), self.canvas_w / 2.0, y)
                    .ok();
            }

            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    "Pick 1-4  Esc=leave",
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Locked Door Challenge overlay ───────────────────────────────
        if let CombatState::LockedDoorChallenge {
            hanzi,
            pinyin: _,
            correct_meaning: _,
            options,
            correct_idx: _,
            door_x: _,
            door_y: _,
        } = combat
        {
            let box_w = 340.0;
            let box_h = 220.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 3.0).sin() * 2.0;

            self.ctx.set_fill_style_str("rgba(35,20,15,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#cc6633");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#cc6633");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("锁 Locked Door", self.canvas_w / 2.0, box_y + 24.0)
                .ok();

            self.ctx.set_fill_style_str("#ffddcc");
            self.ctx.set_font("36px 'Noto Serif SC', 'SimSun', serif");
            self.ctx
                .fill_text(hanzi, self.canvas_w / 2.0, box_y + 72.0)
                .ok();

            self.ctx.set_fill_style_str("#cc9977");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text("What does this mean?", self.canvas_w / 2.0, box_y + 92.0)
                .ok();

            self.ctx.set_font("14px monospace");
            for (i, opt) in options.iter().enumerate() {
                let y = box_y + 114.0 + i as f64 * 20.0;
                self.ctx.set_fill_style_str("#ffeecc");
                self.ctx
                    .fill_text(&format!("{}: {}", i + 1, opt), self.canvas_w / 2.0, y)
                    .ok();
            }

            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    "Pick 1-4  Esc=leave",
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Cursed Floor Challenge overlay ──────────────────────────────
        if let CombatState::CursedFloorChallenge {
            hanzi,
            pinyin: _,
            meaning,
            correct_tone: _,
        } = combat
        {
            let box_w = 320.0;
            let box_h = 200.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 4.0).sin() * 3.0;

            self.ctx.set_fill_style_str("rgba(30,10,35,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#bb44ff");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#bb44ff");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("咒 Cursed Floor!", self.canvas_w / 2.0, box_y + 24.0)
                .ok();

            self.ctx.set_fill_style_str("#eeccff");
            self.ctx.set_font("36px 'Noto Serif SC', 'SimSun', serif");
            self.ctx
                .fill_text(hanzi, self.canvas_w / 2.0, box_y + 70.0)
                .ok();

            self.ctx.set_fill_style_str("#ccaadd");
            self.ctx.set_font("14px monospace");
            self.ctx
                .fill_text(meaning, self.canvas_w / 2.0, box_y + 92.0)
                .ok();

            self.ctx.set_fill_style_str("#ccccee");
            self.ctx.set_font("14px monospace");
            self.ctx
                .fill_text("What tone? 1-4", self.canvas_w / 2.0, box_y + 116.0)
                .ok();

            self.ctx.set_font("12px monospace");
            for i in 1..=4u8 {
                let label = match i {
                    1 => "1: ˉ flat",
                    2 => "2: ˊ rising",
                    3 => "3: ˇ dip",
                    _ => "4: ˋ falling",
                };
                let y = box_y + 134.0 + (i - 1) as f64 * 16.0;
                self.ctx.set_fill_style_str("#aaaacc");
                self.ctx.fill_text(label, self.canvas_w / 2.0, y).ok();
            }

            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx
                .fill_text(
                    "Pick 1-4  Wrong = -2 gold",
                    self.canvas_w / 2.0,
                    box_y + box_h - 10.0,
                )
                .ok();
        }

        // ── Tone Battle overlay ─────────────────────────────────────────
        if let CombatState::ToneBattle {
            round,
            hanzi,
            correct_tone: _,
            score,
            last_result,
        } = combat
        {
            let box_w = 320.0;
            let box_h = 180.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 60.0 + (anim_t * 3.0).sin() * 2.0;

            self.ctx.set_fill_style_str("rgba(20,15,40,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#ddaa55");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            // Title
            self.ctx.set_fill_style_str("#ddaa55");
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    &format!("🔔 Tone Shrine — Round {}/5", round + 1),
                    self.canvas_w / 2.0,
                    box_y + 24.0,
                )
                .ok();

            // Character
            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.set_font("42px 'Noto Serif SC', 'SimSun', serif");
            self.ctx
                .fill_text(hanzi, self.canvas_w / 2.0, box_y + 75.0)
                .ok();

            // Score
            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(
                    &format!("Score: {}/{}", score, round + 1),
                    self.canvas_w / 2.0,
                    box_y + 95.0,
                )
                .ok();

            // Tone options
            let tones = [
                "1: ā (flat)",
                "2: á (rising)",
                "3: ǎ (dip)",
                "4: à (falling)",
            ];
            self.ctx.set_font("14px monospace");
            for (i, label) in tones.iter().enumerate() {
                let y = box_y + 115.0 + i as f64 * 18.0;
                self.ctx.set_fill_style_str("#ccccee");
                self.ctx.fill_text(label, self.canvas_w / 2.0, y).ok();
            }

            // Last result indicator
            if let Some(was_correct) = last_result {
                let (txt, col) = if *was_correct {
                    ("✓", "#66ff66")
                } else {
                    ("✗", "#ff6666")
                };
                self.ctx.set_fill_style_str(col);
                self.ctx.set_font("20px monospace");
                self.ctx.fill_text(txt, box_x + 20.0, box_y + 24.0).ok();
            }
        }

        // ── Game Over overlay ───────────────────────────────────────────
        // ── Class selection screen ──────────────────────────────────────
        if matches!(combat, CombatState::ClassSelect) {
            self.ctx.set_fill_style_str("rgba(0,0,0,0.85)");
            self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

            let cx = self.canvas_w / 2.0;
            let mut y = 40.0 + (anim_t * 2.1).sin() * 4.0;

            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.set_font("32px monospace");
            self.ctx.set_text_align("center");
            self.ctx.fill_text("选择你的道路", cx, y).ok();
            y += 30.0;
            self.ctx.set_fill_style_str("#888");
            self.ctx.set_font("14px monospace");
            self.ctx.fill_text("Choose Your Path", cx, y).ok();
            y += 40.0;

            let all_classes = crate::player::PlayerClass::all();
            let total = all_classes.len();
            let cursor = class_cursor;

            let page_size = 6;
            let page = cursor / page_size;
            let start_idx_cls = page * page_size;
            let end_idx_cls = (start_idx_cls + page_size).min(total);

            for i in start_idx_cls..end_idx_cls {
                let class_var: crate::player::PlayerClass = all_classes[i];
                let data = class_var.data();

                let is_selected = i == cursor;
                let bg_color = if is_selected {
                    "rgba(255,255,255,0.15)"
                } else {
                    "rgba(0,0,0,0.4)"
                };
                let border_color = if is_selected { data.color } else { "#444" };

                self.ctx.set_fill_style_str(bg_color);
                self.ctx.set_stroke_style_str(border_color);
                self.ctx.set_line_width(if is_selected { 2.0 } else { 1.0 });

                let box_w = 400.0;
                let box_h = 50.0;
                let box_x = cx - box_w / 2.0;

                self.ctx.fill_rect(box_x, y, box_w, box_h);
                self.ctx.stroke_rect(box_x, y, box_w, box_h);

                // Icon
                self.ctx.set_fill_style_str(data.color);
                self.ctx.set_font("20px monospace");
                self.ctx.set_text_align("left");
                self.ctx.fill_text(data.icon, box_x + 15.0, y + 32.0).ok();

                // Name
                self.ctx
                    .set_fill_style_str(if is_selected { "#fff" } else { "#ccc" });
                self.ctx.set_font("16px monospace");
                self.ctx
                    .fill_text(
                        &format!("{} {}", data.name_cn, data.name_en),
                        box_x + 45.0,
                        y + 22.0,
                    )
                    .ok();

                let dummy = crate::player::Player::new(0, 0, class_var);
                self.ctx.set_fill_style_str("#aaa");
                self.ctx.set_font("12px monospace");
                self.ctx
                    .fill_text(
                        &format!("HP:{} Items:{}", dummy.max_hp, dummy.max_items()),
                        box_x + 280.0,
                        y + 22.0,
                    )
                    .ok();

                // Lore
                self.ctx.set_fill_style_str(data.color);
                self.ctx.set_font("12px monospace");
                self.ctx.fill_text(data.lore, box_x + 45.0, y + 40.0).ok();

                y += box_h + 10.0;
            }

            y += 10.0;
            let total_pages = (total + page_size - 1) / page_size;
            self.ctx.set_fill_style_str("#888");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    &format!(
                        "Page {}/{} (↑/↓ to scroll, Enter to select)",
                        page + 1,
                        total_pages
                    ),
                    cx,
                    y,
                )
                .ok();

            if total_runs == 0 {
                y += 24.0;
                self.ctx.set_fill_style_str("#66ccff");
                self.ctx.set_font("12px monospace");
                self.ctx
                    .fill_text("First run starts with a short tutorial floor.", cx, y)
                    .ok();
            }

            y += 20.0;
            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.set_font("14px monospace");
            self.ctx
                .fill_text("[D] Daily Challenge (fixed seed)", cx, y)
                .ok();
            y += 24.0;
            self.ctx.set_fill_style_str("#88bbff");
            self.ctx.set_font("12px monospace");
            self.ctx.fill_text("[O] Options", cx, y).ok();
        }

        // ── Game Over overlay — Post-mortem ────────────────────────────
        if matches!(combat, CombatState::GameOver) {
            self.ctx.set_fill_style_str("rgba(0,0,0,0.82)");
            self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

            let cx = self.canvas_w / 2.0;

            if post_mortem_page == 0 {
                // ── Page 0: Summary ──────────────────────────────────
                let mut y = self.canvas_h / 2.0 - 120.0 + (anim_t * 1.7).sin() * 4.0;

                self.ctx.set_fill_style_str("#ff4444");
                self.ctx.set_font("42px monospace");
                self.ctx.set_text_align("center");
                self.ctx.fill_text("☠ RUN COMPLETE", cx, y).ok();
                y += 36.0;

                // Cause of death
                let cause = run_journal.death_cause();
                self.ctx.set_fill_style_str("#ff8888");
                self.ctx.set_font("16px monospace");
                self.ctx
                    .fill_text(&format!("Slain by: {}", cause), cx, y)
                    .ok();
                y += 30.0;

                // Floor reached
                self.ctx.set_fill_style_str("#aaa");
                self.ctx.set_font("15px monospace");
                let reached_label = if !location_label.is_empty() {
                    format!("{} — Deck {} reached  (Best: {})", location_label, floor_num, best_floor)
                } else {
                    format!("Floor {} reached  (Best: {})", floor_num, best_floor)
                };
                self.ctx
                    .fill_text(&reached_label, cx, y)
                    .ok();
                y += 26.0;

                // Kills / bosses
                let kills = run_journal.enemies_killed_count();
                self.ctx.set_fill_style_str("#ffdd44");
                self.ctx.set_font("13px monospace");
                self.ctx
                    .fill_text(
                        &format!(
                            "Enemies slain: {}  |  Gold: {}  |  Spells: {}",
                            kills,
                            player.gold,
                            player.spells.len()
                        ),
                        cx,
                        y,
                    )
                    .ok();
                y += 22.0;

                // Max combo
                self.ctx.set_fill_style_str("#88ddff");
                self.ctx
                    .fill_text(
                        &format!(
                            "Max combo: {}×  |  Recipes: {}/{}",
                            run_journal.max_combo,
                            recipes_found,
                            crate::radical::RECIPES.len()
                        ),
                        cx,
                        y,
                    )
                    .ok();
                y += 22.0;

                // SRS accuracy
                let total_attempts: u32 = srs.stats.values().map(|(_, t, _)| t).sum();
                let total_correct: u32 = srs.stats.values().map(|(c, _, _)| c).sum();
                let pct = if total_attempts > 0 {
                    (total_correct as f64 / total_attempts as f64 * 100.0) as u32
                } else {
                    0
                };
                self.ctx.set_fill_style_str("#aaddaa");
                self.ctx
                    .fill_text(
                        &format!(
                            "Pinyin accuracy: {}% ({}/{})",
                            pct, total_correct, total_attempts
                        ),
                        cx,
                        y,
                    )
                    .ok();
                y += 22.0;

                // Total runs / kills
                self.ctx.set_fill_style_str("#88bbff");
                self.ctx
                    .fill_text(
                        &format!(
                            "Total runs: {}  |  Total kills: {}",
                            total_runs + 1,
                            total_kills
                        ),
                        cx,
                        y,
                    )
                    .ok();
                y += 34.0;

                // Navigation hint
                self.ctx.set_fill_style_str("#00ccdd");
                self.ctx.set_font("14px monospace");
                self.ctx
                    .fill_text("Press R to restart  |  → Floor log", cx, y)
                    .ok();
            } else {
                // ── Page 1+: Floor-by-floor log ──────────────────────
                let mut y = 60.0;
                self.ctx.set_fill_style_str("#ff8844");
                self.ctx.set_font("28px monospace");
                self.ctx.set_text_align("center");
                self.ctx.fill_text("📜 Floor Log", cx, y).ok();
                y += 36.0;

                let max_fl = run_journal.max_floor();
                let floors_per_page = 8;
                let start_floor = 1 + (post_mortem_page - 1) * floors_per_page;
                let end_floor = (start_floor + floors_per_page).min(max_fl as usize + 1);
                let total_pages = ((max_fl as usize).saturating_sub(1)) / floors_per_page + 1;

                self.ctx.set_font("13px monospace");
                self.ctx.set_text_align("left");
                let left = (cx - 200.0).max(20.0);

                for fl in start_floor..end_floor {
                    let line = run_journal.floor_line(fl as i32);
                    self.ctx.set_fill_style_str("#ffdd44");
                    self.ctx.fill_text(&format!("F{:>2}:", fl), left, y).ok();
                    self.ctx.set_fill_style_str("#ccc");
                    self.ctx.fill_text(&line, left + 44.0, y).ok();
                    y += 22.0;
                }

                y += 16.0;
                self.ctx.set_fill_style_str("#888");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(&format!("Page {}/{}", post_mortem_page, total_pages), cx, y)
                    .ok();
                y += 28.0;

                self.ctx.set_fill_style_str("#00ccdd");
                self.ctx.set_font("14px monospace");
                self.ctx
                    .fill_text("← Back  |  → Next  |  R Restart", cx, y)
                    .ok();
            }
        }

        if show_settings {
            let box_w = 360.0;
            let box_h = 220.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = (self.canvas_h - box_h) / 2.0 + (anim_t * 2.8).sin() * 3.0;
            let rows = [
                ("Music Volume", format!("{}%", settings.music_volume)),
                ("SFX Volume", format!("{}%", settings.sfx_volume)),
                (
                    "Screen Shake",
                    if settings.screen_shake {
                        "On".to_string()
                    } else {
                        "Off".to_string()
                    },
                ),
                ("Text Speed", settings.text_speed.label().to_string()),
            ];

            self.ctx.set_fill_style_str("rgba(0,0,0,0.65)");
            self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);
            self.ctx.set_fill_style_str("rgba(20,18,36,0.97)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#88bbff");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_fill_style_str("#ffcc88");
            self.ctx.set_font("bold 18px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("Options / 设置", self.canvas_w / 2.0, box_y + 26.0)
                .ok();

            self.ctx.set_font("14px monospace");
            for (i, (label, value)) in rows.iter().enumerate() {
                let y = box_y + 60.0 + i as f64 * 34.0;
                let selected = i == settings_cursor;
                if selected {
                    self.ctx.set_fill_style_str("rgba(136,187,255,0.16)");
                    self.ctx
                        .fill_rect(box_x + 16.0, y - 16.0, box_w - 32.0, 24.0);
                }
                self.ctx
                    .set_fill_style_str(if selected { "#ffdd88" } else { "#ccd6ff" });
                self.ctx.set_text_align("left");
                self.ctx.fill_text(label, box_x + 24.0, y).ok();
                self.ctx.set_text_align("right");
                self.ctx.fill_text(value, box_x + box_w - 24.0, y).ok();
            }

            self.ctx.set_fill_style_str("#7784aa");
            self.ctx.set_font("11px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    "↑↓ select  ←→ adjust  Enter=cycle/toggle  Esc/O=close",
                    self.canvas_w / 2.0,
                    box_y + box_h - 16.0,
                )
                .ok();
        }

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
