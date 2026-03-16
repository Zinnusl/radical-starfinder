//! Canvas 2D rendering for the dungeon.

use std::collections::BTreeMap;

use js_sys::Date;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::dungeon::{AltarKind, DungeonLevel, SealKind, Tile};
use crate::enemy::{BossKind, Enemy};
use crate::game::{CombatState, GameSettings, ListenMode, ShopItemKind, TalentTree};
use crate::particle::ParticleSystem;
use crate::player::{Deity, Item, ItemKind, Player, PlayerForm};
use crate::radical;
use crate::sprites::SpriteCache;

const TILE_SIZE: f64 = 24.0;

// Colors
const COL_WALL: &str = "#2a1f3d";
const COL_WALL_REVEALED: &str = "#1a1428";
const COL_FLOOR: &str = "#4a4260";
const COL_FLOOR_REVEALED: &str = "#2d2840";
const COL_CORRIDOR: &str = "#3d3555";
const COL_CORRIDOR_REVEALED: &str = "#272040";
const COL_STAIRS: &str = "#8ab4ff";
const COL_FORGE: &str = "#ff8844";
const COL_SHOP: &str = "#44dd88";
const COL_CHEST: &str = "#ddaa33";
const COL_FOG: &str = "#0d0b14";
const COL_PLAYER: &str = "#ffcc33";
const COL_PLAYER_OUTLINE: &str = "#bb8800";
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
        room_modifier: Option<crate::dungeon::RoomModifier>,
        listening_mode: ListenMode,
        companion: Option<crate::game::Companion>,
        quests: &[crate::game::Quest],
        tutorial_hint: Option<&str>,
        show_help: bool,
        item_labels: &[String],
        settings: &GameSettings,
        show_settings: bool,
        settings_cursor: usize,
        talents: &TalentTree,
        show_talent_tree: bool,
        talent_cursor: usize,
        knowledge_points_available: i32,
        knowledge_points_total: i32,
        knowledge_progress: usize,
        knowledge_step: usize,
        answer_streak: u32,
        floor_profile_label: &str,
        codex: &crate::codex::Codex,
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
                let sprite_key = tile_sprite_key(tile);
                if self.sprites.is_loaded(sprite_key) {
                    if let Some(img) = self.sprites.get(sprite_key) {
                        if !visible {
                            self.ctx.set_global_alpha(0.4);
                        }
                        self.ctx
                            .draw_image_with_html_image_element_and_dw_and_dh(
                                img, screen_x, screen_y, TILE_SIZE, TILE_SIZE,
                            )
                            .ok();
                        if !visible {
                            self.ctx.set_global_alpha(1.0);
                        }
                        tile_sprite_drawn = true;
                    }
                }

                if visible && !tile_sprite_drawn {
                    self.draw_tile_surface(tile, palette, tx, ty, screen_x, screen_y, anim_t);
                }
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
            PlayerForm::Flame => "player_flame",
            PlayerForm::Stone => "player_stone",
            PlayerForm::Mist => "player_mist",
            PlayerForm::Tiger => "player_tiger",
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
                "#ffcc33"
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
                "enemy_generic"
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
                "#ffcc33"
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
                self.ctx.set_fill_style_str("#ffcc33");
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
        let floor_label = if floor_num == 0 {
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
        self.ctx.set_fill_style_str("#ffdd44");
        self.ctx.set_font("14px monospace");
        let gold_y = if floor_profile_label.is_empty() {
            42.0
        } else {
            52.0
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
                crate::dungeon::RoomModifier::Dark => ("🌑 Dark Room", "#8888bb"),
                crate::dungeon::RoomModifier::Arcane => ("✨ Arcane Room", "#aa66ff"),
                crate::dungeon::RoomModifier::Cursed => ("💀 Cursed Room", "#ff6666"),
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
            self.ctx
                .fill_text(
                    &format!("{} {}", comp.icon(), comp.name()),
                    self.canvas_w - 12.0,
                    eq_y,
                )
                .ok();
            eq_y += 14.0;
        }
        // Deity piety
        for &(deity, piety) in &player.piety {
            if piety != 0 {
                let (icon, color) = match deity {
                    Deity::Jade => ("🟢", "#66cc88"),
                    Deity::Iron => ("⚙", "#99aacc"),
                    Deity::Gold => ("💰", "#ddaa44"),
                    Deity::Gale => ("🌀", "#66bbdd"),
                    Deity::Mirror => ("🪞", "#bb88dd"),
                };
                self.ctx.set_fill_style_str(color);
                self.ctx
                    .fill_text(&format!("{} {:+}", icon, piety), self.canvas_w - 12.0, eq_y)
                    .ok();
                eq_y += 14.0;
            }
        }
        if let Some((synergy_name, _)) = player.deity_synergy() {
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
            self.ctx.set_stroke_style_str("#ffcc33");
            self.ctx.set_line_width(1.0);
            self.ctx.stroke_rect(box_x, 10.0, box_w, 38.0);
            self.ctx.set_text_align("center");
            self.ctx.set_fill_style_str("#ffcc33");
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
                    .set_fill_style_str(if selected { "#ffcc33" } else { "#88bbdd" });
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
            self.ctx.fill_text("🛡 Shield Active", 12.0, 36.0).ok();
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
            self.ctx.set_stroke_style_str("#ffcc33");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(px, py, pw, ph);
            self.ctx.set_font("bold 14px monospace");
            self.ctx.set_text_align("center");
            self.ctx.set_fill_style_str("#ffcc33");
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
        self.draw_minimap(level, player);

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

        // ── Message bar (bottom-center) ─────────────────────────────────
        if !message.is_empty() {
            let message_lift = (anim_t * 7.5).sin().abs() * 2.0;
            self.ctx.set_font("14px monospace");
            self.ctx.set_text_align("center");
            self.ctx.set_fill_style_str("rgba(0,0,0,0.78)");
            self.ctx.fill_rect(
                self.canvas_w * 0.15,
                self.canvas_h - 38.0 - message_lift,
                self.canvas_w * 0.7,
                30.0,
            );
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
                if answer_streak >= 3 {
                    self.ctx.set_fill_style_str(if answer_streak >= 10 {
                        "#ff4422"
                    } else if answer_streak >= 5 {
                        "#ff8844"
                    } else {
                        "#cc8833"
                    });
                    self.ctx.set_font("bold 11px monospace");
                    self.ctx.set_text_align("right");
                    self.ctx
                        .fill_text(
                            &format!("🔥×{}", answer_streak),
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
                    let teacher_hint = if companion == Some(crate::game::Companion::Teacher) {
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
                    .set_fill_style_str(if typing.is_empty() { "#555" } else { "#ffcc33" });
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
                    .set_fill_style_str(if is_cursor { "#ffcc33" } else { "#888" });
                self.ctx.set_font("11px monospace");
                self.ctx
                    .fill_text(&format!("{}{}", marker, num), box_x + 10.0, ry + 17.0)
                    .ok();

                self.ctx
                    .set_fill_style_str(if is_cursor { "#ffcc33" } else { "#ffaa66" });
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
                        "#ffcc33"
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
                self.ctx.set_fill_style_str("#ffcc33");
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

                    self.ctx.set_fill_style_str("#ffcc33");
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
            let box_w = 350.0;
            let box_h = 60.0 + items.len() as f64 * 28.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 50.0;

            // Background
            self.ctx.set_fill_style_str("rgba(10,30,20,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#44dd88");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            // Title
            self.ctx.set_fill_style_str("#44dd88");
            self.ctx.set_font("18px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("$ Shop $", self.canvas_w / 2.0, box_y + 26.0)
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

            // Items
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
                let total_discount = (player.shop_discount_pct
                    + if companion == Some(crate::game::Companion::Merchant) {
                        20
                    } else {
                        0
                    })
                .clamp(0, 50);
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

            // Hint
            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    "↑↓=browse  Enter=buy  Esc=leave",
                    self.canvas_w / 2.0,
                    box_y + box_h + 14.0,
                )
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
                    .set_stroke_style_str(if selected { "#ffcc33" } else { "#555" });
                self.ctx.set_line_width(if selected { 2.0 } else { 1.0 });
                self.ctx.stroke_rect(tx + 2.0, ty, tile_w - 4.0, 36.0);
                self.ctx
                    .set_fill_style_str(if selected { "#ffcc33" } else { "#ccccee" });
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

            self.ctx.set_fill_style_str("#ffcc33");
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
                    .set_fill_style_str(if selected { "#ffcc33" } else { "#ccccee" });
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

            self.ctx.set_fill_style_str("#ffcc33");
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
                    .set_fill_style_str(if selected { "#ffcc33" } else { "#ccccee" });
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

            self.ctx.set_fill_style_str("#ffcc33");
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
            self.ctx.set_fill_style_str("#ffcc33");
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
            let mut y = 60.0 + (anim_t * 2.1).sin() * 4.0;

            self.ctx.set_fill_style_str("#ffcc33");
            self.ctx.set_font("32px monospace");
            self.ctx.set_text_align("center");
            self.ctx.fill_text("选择你的道路", cx, y).ok();
            y += 30.0;
            self.ctx.set_fill_style_str("#888");
            self.ctx.set_font("14px monospace");
            self.ctx.fill_text("Choose Your Path", cx, y).ok();
            y += 50.0;

            let classes = [
                (
                    "1",
                    "📚 Scholar",
                    "#44aaff",
                    "Balanced. Hints show meaning in combat.",
                ),
                (
                    "2",
                    "⚔ Warrior",
                    "#ff6644",
                    "+3 HP, +1 damage, 4 item slots.",
                ),
                (
                    "3",
                    "⚗ Alchemist",
                    "#44dd88",
                    "7 item slots, 2x potion healing.",
                ),
            ];

            for (key, name, color, desc) in &classes {
                self.ctx.set_fill_style_str(color);
                self.ctx.set_font("22px monospace");
                self.ctx
                    .fill_text(&format!("[{}] {}", key, name), cx, y)
                    .ok();
                y += 22.0;
                self.ctx.set_fill_style_str("#999");
                self.ctx.set_font("12px monospace");
                self.ctx.fill_text(desc, cx, y).ok();
                y += 36.0;
            }

            if total_runs == 0 {
                self.ctx.set_fill_style_str("#66ccff");
                self.ctx.set_font("12px monospace");
                self.ctx
                    .fill_text("First run starts with a short tutorial floor.", cx, y)
                    .ok();
                y += 24.0;
            }

            y += 10.0;
            self.ctx.set_fill_style_str("#ffcc33");
            self.ctx.set_font("14px monospace");
            self.ctx
                .fill_text("[D] Daily Challenge (fixed seed)", cx, y)
                .ok();
            y += 24.0;
            self.ctx.set_fill_style_str("#88bbff");
            self.ctx.set_font("12px monospace");
            self.ctx.fill_text("[O] Options", cx, y).ok();
            y += 20.0;
            self.ctx
                .set_fill_style_str(if knowledge_points_available > 0 {
                    "#aaff88"
                } else {
                    "#88ddff"
                });
            self.ctx
                .fill_text(
                    &format!(
                        "[T] Talent Tree  ({} KP available)",
                        knowledge_points_available
                    ),
                    cx,
                    y,
                )
                .ok();
            y += 18.0;
            self.ctx.set_fill_style_str("#8899aa");
            self.ctx.set_font("11px monospace");
            self.ctx
                .fill_text(
                    &format!(
                        "Meta bonuses: +{} HP  -{}% shop  +{} spell",
                        talents.starting_hp_bonus(),
                        talents.shop_discount_pct(),
                        talents.spell_power_bonus()
                    ),
                    cx,
                    y,
                )
                .ok();
        }

        // ── Game Over overlay ───────────────────────────────────────────
        if matches!(combat, CombatState::GameOver) {
            self.ctx.set_fill_style_str("rgba(0,0,0,0.75)");
            self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

            let cx = self.canvas_w / 2.0;
            let mut y = self.canvas_h / 2.0 - 80.0 + (anim_t * 1.7).sin() * 4.0;

            self.ctx.set_fill_style_str("#ff4444");
            self.ctx.set_font("48px monospace");
            self.ctx.set_text_align("center");
            self.ctx.fill_text("GAME OVER", cx, y).ok();
            y += 40.0;

            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("16px monospace");
            self.ctx
                .fill_text(
                    &format!("Floor {} reached  (Best: {})", floor_num, best_floor),
                    cx,
                    y,
                )
                .ok();
            y += 28.0;

            // Stats box
            self.ctx.set_fill_style_str("#ffdd44");
            self.ctx.set_font("13px monospace");
            self.ctx
                .fill_text(
                    &format!(
                        "Gold: {}  |  Spells: {}  |  Recipes: {}/{}",
                        player.gold,
                        player.spells.len(),
                        recipes_found,
                        crate::radical::RECIPES.len()
                    ),
                    cx,
                    y,
                )
                .ok();
            y += 22.0;

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
            y += 22.0;

            // SRS accuracy summary
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
            y += 30.0;

            self.ctx.set_fill_style_str("#ffcc33");
            self.ctx.set_font("14px monospace");
            self.ctx.fill_text("Press R to restart", cx, y).ok();
            y += 20.0;
            self.ctx.set_fill_style_str("#88ddff");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(
                    &format!(
                        "[T] Talent Tree  ({} KP available)",
                        knowledge_points_available
                    ),
                    cx,
                    y,
                )
                .ok();
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

        if show_talent_tree {
            let box_w = 420.0;
            let box_h = 240.0;
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = (self.canvas_h - box_h) / 2.0 + (anim_t * 2.4).sin() * 3.0;
            self.ctx.set_fill_style_str("rgba(4,10,18,0.7)");
            self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);
            self.ctx.set_fill_style_str("rgba(16,24,36,0.97)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#66ddaa");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            self.ctx.set_text_align("center");
            self.ctx.set_fill_style_str("#c8ffb0");
            self.ctx.set_font("bold 18px monospace");
            self.ctx
                .fill_text("Talent Tree / 天赋", self.canvas_w / 2.0, box_y + 26.0)
                .ok();

            self.ctx.set_fill_style_str("#aacccc");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(
                    &format!(
                        "Knowledge Points: {} available / {} earned",
                        knowledge_points_available, knowledge_points_total
                    ),
                    self.canvas_w / 2.0,
                    box_y + 46.0,
                )
                .ok();
            self.ctx
                .fill_text(
                    &format!(
                        "Progress to next point: {}/{} unique codex entries",
                        knowledge_progress, knowledge_step
                    ),
                    self.canvas_w / 2.0,
                    box_y + 62.0,
                )
                .ok();

            for idx in 0..3 {
                let y = box_y + 98.0 + idx as f64 * 42.0;
                let selected = idx == talent_cursor;
                let rank = talents.rank(idx);
                let max_rank = TalentTree::max_rank(idx);
                let pips = format!(
                    "{}{}",
                    "●".repeat(rank as usize),
                    "○".repeat(max_rank.saturating_sub(rank) as usize)
                );
                if selected {
                    self.ctx.set_fill_style_str("rgba(102,221,170,0.14)");
                    self.ctx
                        .fill_rect(box_x + 16.0, y - 18.0, box_w - 32.0, 30.0);
                }
                self.ctx.set_text_align("left");
                self.ctx
                    .set_fill_style_str(if selected { "#ffdd88" } else { "#ddeeff" });
                self.ctx.set_font("bold 14px monospace");
                self.ctx
                    .fill_text(TalentTree::title(idx), box_x + 24.0, y)
                    .ok();
                self.ctx.set_fill_style_str("#99b8c8");
                self.ctx.set_font("11px monospace");
                self.ctx
                    .fill_text(TalentTree::description(idx), box_x + 24.0, y + 14.0)
                    .ok();
                self.ctx.set_text_align("right");
                self.ctx.set_fill_style_str("#88ffcc");
                self.ctx.set_font("12px monospace");
                self.ctx
                    .fill_text(
                        &format!("{}  {}", talents.bonus_text(idx), pips),
                        box_x + box_w - 24.0,
                        y + 6.0,
                    )
                    .ok();
            }

            self.ctx.set_text_align("center");
            self.ctx.set_fill_style_str("#7784aa");
            self.ctx.set_font("11px monospace");
            self.ctx
                .fill_text(
                    "↑↓ select  Enter=buy rank  Esc/T=close",
                    self.canvas_w / 2.0,
                    box_y + box_h - 16.0,
                )
                .ok();
        }

        if show_help {
            self.draw_help_overlay(combat, listening_mode);
        }
    }

    fn draw_tile_surface(
        &self,
        tile: Tile,
        palette: TilePalette,
        tx: i32,
        ty: i32,
        screen_x: f64,
        screen_y: f64,
        anim_t: f64,
    ) {
        let pattern = tile_pattern_seed(tx, ty);
        let (highlight, shadow) =
            if matches!(tile, Tile::Wall | Tile::CrackedWall | Tile::BrittleWall) {
                ("rgba(255,255,255,0.08)", "rgba(0,0,0,0.32)")
            } else {
                ("rgba(255,255,255,0.06)", "rgba(0,0,0,0.24)")
            };

        self.ctx.set_fill_style_str(highlight);
        self.ctx
            .fill_rect(screen_x + 0.5, screen_y + 0.5, TILE_SIZE - 1.0, 1.0);
        self.ctx
            .fill_rect(screen_x + 0.5, screen_y + 1.5, 1.0, TILE_SIZE - 2.0);
        self.ctx.set_fill_style_str(shadow);
        self.ctx.fill_rect(
            screen_x + TILE_SIZE - 1.5,
            screen_y + 1.5,
            1.0,
            TILE_SIZE - 2.0,
        );
        self.ctx.fill_rect(
            screen_x + 1.5,
            screen_y + TILE_SIZE - 1.5,
            TILE_SIZE - 2.0,
            1.0,
        );

        match tile {
            Tile::Floor | Tile::Corridor | Tile::CursedFloor => {
                self.ctx.set_fill_style_str(if tile == Tile::Corridor {
                    "rgba(215,225,255,0.06)"
                } else {
                    "rgba(255,255,255,0.05)"
                });
                let spark_x = screen_x + 4.0 + (pattern % 11) as f64;
                let spark_y = screen_y + 4.0 + ((pattern / 11) % 9) as f64;
                self.ctx.fill_rect(spark_x, spark_y, 2.0, 2.0);
                if tile == Tile::Corridor {
                    self.ctx.set_fill_style_str("rgba(170,190,255,0.05)");
                    self.ctx.fill_rect(
                        screen_x + 4.0,
                        screen_y + TILE_SIZE / 2.0 - 0.5,
                        TILE_SIZE - 8.0,
                        1.0,
                    );
                }
                if tile == Tile::CursedFloor {
                    // Subtle cursed shimmer — barely visible trap hint
                    self.ctx.set_fill_style_str("rgba(180,120,255,0.06)");
                    let cx = screen_x + 6.0 + ((pattern / 3) % 8) as f64;
                    let cy = screen_y + 6.0 + ((pattern / 7) % 8) as f64;
                    self.ctx.fill_rect(cx, cy, 2.0, 2.0);
                }
            }
            Tile::Wall | Tile::CrackedWall | Tile::BrittleWall => {
                self.ctx.set_fill_style_str("rgba(0,0,0,0.14)");
                self.ctx.fill_rect(
                    screen_x + 3.0,
                    screen_y + 3.0,
                    TILE_SIZE - 6.0,
                    TILE_SIZE - 6.0,
                );
                self.ctx.set_fill_style_str("rgba(255,255,255,0.07)");
                let seam_y = screen_y + 7.0 + (pattern % 6) as f64;
                self.ctx
                    .fill_rect(screen_x + 3.0, seam_y, TILE_SIZE - 6.0, 1.0);
                let seam_x = screen_x + 7.0 + ((pattern / 5) % 8) as f64;
                self.ctx
                    .fill_rect(seam_x, screen_y + 3.0, 1.0, TILE_SIZE / 2.0 - 1.0);
                if tile == Tile::CrackedWall {
                    self.ctx.set_stroke_style_str("rgba(255,180,120,0.65)");
                    self.ctx.set_line_width(1.2);
                    self.ctx.begin_path();
                    self.ctx.move_to(screen_x + 12.0, screen_y + 3.0);
                    self.ctx.line_to(screen_x + 10.0, screen_y + 9.0);
                    self.ctx.line_to(screen_x + 14.0, screen_y + 14.0);
                    self.ctx.line_to(screen_x + 9.0, screen_y + 21.0);
                    self.ctx.stroke();

                    self.ctx.begin_path();
                    self.ctx.move_to(screen_x + 10.0, screen_y + 9.0);
                    self.ctx.line_to(screen_x + 6.0, screen_y + 12.0);
                    self.ctx.stroke();

                    self.ctx.begin_path();
                    self.ctx.move_to(screen_x + 14.0, screen_y + 14.0);
                    self.ctx.line_to(screen_x + 18.0, screen_y + 17.0);
                    self.ctx.stroke();
                } else if tile == Tile::BrittleWall {
                    self.ctx.set_stroke_style_str("rgba(255,221,170,0.58)");
                    self.ctx.set_line_width(1.0);
                    self.ctx.begin_path();
                    self.ctx.move_to(screen_x + 7.0, screen_y + 6.0);
                    self.ctx.line_to(screen_x + 12.0, screen_y + 11.0);
                    self.ctx.line_to(screen_x + 9.0, screen_y + 17.0);
                    self.ctx.line_to(screen_x + 15.0, screen_y + 21.0);
                    self.ctx.stroke();

                    self.ctx.begin_path();
                    self.ctx.move_to(screen_x + 12.0, screen_y + 11.0);
                    self.ctx.line_to(screen_x + 18.0, screen_y + 9.0);
                    self.ctx.stroke();
                }
            }
            Tile::Water | Tile::DeepWater => {
                let wave_shift = (anim_t * 3.2 + tx as f64 * 0.7 + ty as f64 * 0.4).sin() * 2.0;
                self.ctx.set_fill_style_str(if tile == Tile::DeepWater {
                    "rgba(160,200,255,0.14)"
                } else {
                    "rgba(210,230,255,0.11)"
                });
                self.ctx.fill_rect(
                    screen_x + 3.0 + wave_shift,
                    screen_y + 7.0,
                    TILE_SIZE - 8.0,
                    1.5,
                );
                self.ctx.fill_rect(
                    screen_x + 5.0 - wave_shift,
                    screen_y + 14.0,
                    TILE_SIZE - 10.0,
                    1.5,
                );
                if tile == Tile::DeepWater {
                    self.ctx.set_fill_style_str("rgba(26,48,89,0.28)");
                    self.ctx
                        .fill_rect(screen_x + 4.0, screen_y + 18.0, TILE_SIZE - 8.0, 3.0);
                }
            }
            Tile::Oil => {
                self.ctx.set_fill_style_str("rgba(255,224,154,0.10)");
                self.ctx.fill_rect(
                    screen_x + 4.0,
                    screen_y + TILE_SIZE - 8.0,
                    TILE_SIZE - 8.0,
                    2.0,
                );
                self.ctx.set_fill_style_str("rgba(255,255,255,0.06)");
                self.ctx
                    .fill_rect(screen_x + 6.0, screen_y + 6.0, TILE_SIZE - 14.0, 1.5);
            }
            Tile::Crate => {
                self.ctx.set_fill_style_str("rgba(255,225,180,0.08)");
                self.ctx.fill_rect(
                    screen_x + 4.0,
                    screen_y + 4.0,
                    TILE_SIZE - 8.0,
                    TILE_SIZE - 8.0,
                );
                self.ctx.set_fill_style_str("rgba(62,33,14,0.45)");
                self.ctx
                    .fill_rect(screen_x + 8.0, screen_y + 4.0, 1.5, TILE_SIZE - 8.0);
                self.ctx
                    .fill_rect(screen_x + 14.5, screen_y + 4.0, 1.5, TILE_SIZE - 8.0);
            }
            Tile::Spikes => {
                self.ctx.set_fill_style_str("rgba(255,220,220,0.08)");
                self.ctx.fill_rect(
                    screen_x + 4.0,
                    screen_y + TILE_SIZE - 7.0,
                    TILE_SIZE - 8.0,
                    2.0,
                );
            }
            Tile::Bridge => {
                // Planks
                self.ctx.set_fill_style_str("rgba(160,110,60,0.4)");
                self.ctx
                    .fill_rect(screen_x + 2.0, screen_y + 4.0, TILE_SIZE - 4.0, 4.0);
                self.ctx
                    .fill_rect(screen_x + 2.0, screen_y + 11.0, TILE_SIZE - 4.0, 4.0);
                self.ctx
                    .fill_rect(screen_x + 2.0, screen_y + 18.0, TILE_SIZE - 4.0, 4.0);
                // Nails
                self.ctx.set_fill_style_str("rgba(100,100,100,0.5)");
                self.ctx.fill_rect(screen_x + 4.0, screen_y + 5.0, 1.0, 1.0);
                self.ctx
                    .fill_rect(screen_x + TILE_SIZE - 5.0, screen_y + 5.0, 1.0, 1.0);
                self.ctx
                    .fill_rect(screen_x + 4.0, screen_y + 12.0, 1.0, 1.0);
                self.ctx
                    .fill_rect(screen_x + TILE_SIZE - 5.0, screen_y + 12.0, 1.0, 1.0);
                self.ctx
                    .fill_rect(screen_x + 4.0, screen_y + 19.0, 1.0, 1.0);
                self.ctx
                    .fill_rect(screen_x + TILE_SIZE - 5.0, screen_y + 19.0, 1.0, 1.0);
            }
            Tile::StairsDown
            | Tile::Forge
            | Tile::Shop
            | Tile::Chest
            | Tile::Npc(_)
            | Tile::Shrine
            | Tile::StrokeShrine
            | Tile::ToneWall
            | Tile::CompoundShrine
            | Tile::ClassifierShrine
            | Tile::InkWell
            | Tile::AncestorShrine
            | Tile::TranslationAltar
            | Tile::RadicalGarden
            | Tile::MirrorPool
            | Tile::StoneTutor
            | Tile::CodexShrine
            | Tile::WordBridge
            | Tile::LockedDoor
            | Tile::Altar(_)
            | Tile::Seal(_)
            | Tile::Sign(_) => {
                if let Some(plate_fill) = tile_plate_fill(tile) {
                    self.ctx.set_fill_style_str(plate_fill);
                    self.ctx.fill_rect(
                        screen_x + 3.0,
                        screen_y + 3.0,
                        TILE_SIZE - 6.0,
                        TILE_SIZE - 6.0,
                    );
                }
            }
        }

        if let Some(accent) = palette.accent {
            self.ctx.set_stroke_style_str(accent);
            self.ctx.set_line_width(1.0);
            self.ctx.stroke_rect(
                screen_x + 1.5,
                screen_y + 1.5,
                TILE_SIZE - 3.0,
                TILE_SIZE - 3.0,
            );
        }

        if let Some(glyph) = palette.glyph {
            self.ctx
                .set_shadow_color(palette.accent.unwrap_or("transparent"));
            self.ctx
                .set_shadow_blur(if palette.accent.is_some() { 8.0 } else { 0.0 });
            self.ctx.set_fill_style_str(palette.glyph_color);
            self.ctx.set_font(tile_glyph_font(tile));
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    glyph,
                    screen_x + TILE_SIZE / 2.0,
                    tile_glyph_y(tile, screen_y, anim_t, tx, ty),
                )
                .ok();
            self.ctx.set_shadow_color("transparent");
            self.ctx.set_shadow_blur(0.0);
        }

        if tile.is_walkable() {
            self.ctx.set_stroke_style_str("rgba(255,255,255,0.05)");
            self.ctx.set_line_width(0.5);
            self.ctx
                .stroke_rect(screen_x, screen_y, TILE_SIZE, TILE_SIZE);
        }
    }

    fn draw_offering_overlay(
        &self,
        player: &Player,
        item_labels: &[String],
        altar_kind: crate::dungeon::AltarKind,
        cursor: usize,
    ) {
        let box_w = 360.0;
        let items_len = player.items.len().max(1);
        let box_h = 100.0 + items_len as f64 * 28.0;
        let box_x = (self.canvas_w - box_w) / 2.0;
        let box_y = 60.0;

        self.ctx.set_fill_style_str("rgba(15,20,30,0.95)");
        self.ctx.fill_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("#ffaa44");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

        let god_name = match altar_kind {
            crate::dungeon::AltarKind::Jade => "Jade Emperor",
            crate::dungeon::AltarKind::Gale => "Wind Walker",
            crate::dungeon::AltarKind::Mirror => "Mirror Sage",
            crate::dungeon::AltarKind::Iron => "Iron General",
            crate::dungeon::AltarKind::Gold => "Golden Toad",
        };

        // Find current piety
        let piety = player
            .piety
            .iter()
            .find(|(d, _)| match (d, altar_kind) {
                (crate::player::Deity::Jade, crate::dungeon::AltarKind::Jade) => true,
                (crate::player::Deity::Gale, crate::dungeon::AltarKind::Gale) => true,
                (crate::player::Deity::Mirror, crate::dungeon::AltarKind::Mirror) => true,
                (crate::player::Deity::Iron, crate::dungeon::AltarKind::Iron) => true,
                (crate::player::Deity::Gold, crate::dungeon::AltarKind::Gold) => true,
                _ => false,
            })
            .map(|(_, p)| *p)
            .unwrap_or(0);

        self.ctx.set_fill_style_str("#ffaa44");
        self.ctx.set_font("bold 16px monospace");
        self.ctx.set_text_align("center");
        self.ctx
            .fill_text(
                &format!("Altar of {}", god_name),
                self.canvas_w / 2.0,
                box_y + 24.0,
            )
            .ok();

        self.ctx.set_font("12px monospace");
        self.ctx.set_fill_style_str("#ffd700");
        self.ctx
            .fill_text(
                &format!("Favor: {}", piety),
                self.canvas_w / 2.0,
                box_y + 42.0,
            )
            .ok();

        self.ctx.set_fill_style_str("#aaaaaa");
        self.ctx
            .fill_text("Select item to offer:", self.canvas_w / 2.0, box_y + 64.0)
            .ok();

        if player.items.is_empty() {
            self.ctx.set_fill_style_str("#888");
            self.ctx.set_font("14px monospace");
            self.ctx
                .fill_text("(Empty Inventory)", self.canvas_w / 2.0, box_y + 90.0)
                .ok();
        } else {
            for (i, label) in item_labels.iter().enumerate() {
                let y = box_y + 90.0 + i as f64 * 28.0;
                let selected = i == cursor;

                if selected {
                    self.ctx.set_fill_style_str("rgba(255,170,68,0.2)");
                    self.ctx
                        .fill_rect(box_x + 10.0, y - 18.0, box_w - 20.0, 24.0);
                }

                self.ctx
                    .set_fill_style_str(if selected { "#ffffff" } else { "#cccccc" });
                self.ctx.set_font("14px monospace");
                self.ctx.set_text_align("left");
                self.ctx.fill_text(label, box_x + 20.0, y).ok();
            }
        }

        // Footer help
        self.ctx.set_text_align("center");
        self.ctx.set_fill_style_str("#7784aa");
        self.ctx.set_font("11px monospace");
        self.ctx
            .fill_text(
                "Enter=offer  P=pray (cost 20)  Esc=leave",
                self.canvas_w / 2.0,
                box_y + box_h - 12.0,
            )
            .ok();
    }

    fn draw_dipping_source_overlay(&self, player: &Player, item_labels: &[String], cursor: usize) {
        let box_w = 320.0;
        let items_len = player.items.len().max(1);
        let box_h = 80.0 + items_len as f64 * 28.0;
        let box_x = (self.canvas_w - box_w) / 2.0;
        let box_y = 60.0;

        self.ctx.set_fill_style_str("rgba(15,20,30,0.95)");
        self.ctx.fill_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("#88aaff");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

        self.ctx.set_fill_style_str("#88aaff");
        self.ctx.set_font("bold 16px monospace");
        self.ctx.set_text_align("center");
        self.ctx
            .fill_text(
                "Dip what? (Select Potion)",
                self.canvas_w / 2.0,
                box_y + 24.0,
            )
            .ok();

        if player.items.is_empty() {
            self.ctx.set_fill_style_str("#888");
            self.ctx.set_font("14px monospace");
            self.ctx
                .fill_text("(Empty)", self.canvas_w / 2.0, box_y + 50.0)
                .ok();
        } else {
            for (i, label) in item_labels.iter().enumerate() {
                let y = box_y + 50.0 + i as f64 * 28.0;
                let selected = i == cursor;

                if selected {
                    self.ctx.set_fill_style_str("rgba(100,120,200,0.3)");
                    self.ctx
                        .fill_rect(box_x + 10.0, y - 18.0, box_w - 20.0, 24.0);
                }

                self.ctx
                    .set_fill_style_str(if selected { "#ffffff" } else { "#aaaaaa" });
                self.ctx.set_font("14px monospace");
                self.ctx.set_text_align("left");
                self.ctx.fill_text(label, box_x + 20.0, y).ok();
            }
        }

        self.ctx.set_text_align("center");
        self.ctx.set_fill_style_str("#7784aa");
        self.ctx.set_font("11px monospace");
        self.ctx
            .fill_text(
                "Enter=select  Esc=cancel",
                self.canvas_w / 2.0,
                box_y + box_h - 12.0,
            )
            .ok();
    }

    fn draw_dipping_target_overlay(
        &self,
        player: &Player,
        item_labels: &[String],
        source_idx: usize,
        cursor: usize,
    ) {
        let items_len = player.items.len().max(1);
        let total_rows = 3 + items_len;
        let box_w = 340.0;
        let box_h = 70.0 + total_rows as f64 * 28.0;
        let box_x = (self.canvas_w - box_w) / 2.0;
        let box_y = 60.0;

        self.ctx.set_fill_style_str("rgba(15,20,30,0.95)");
        self.ctx.fill_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("#88aaff");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

        self.ctx.set_fill_style_str("#88aaff");
        self.ctx.set_font("bold 16px monospace");
        self.ctx.set_text_align("center");
        self.ctx
            .fill_text("Dip into what?", self.canvas_w / 2.0, box_y + 24.0)
            .ok();

        let mut y = box_y + 50.0;

        // Equipment
        let equips = ["Weapon", "Armor", "Charm"];
        for i in 0..3 {
            let selected = cursor == i;
            if selected {
                self.ctx.set_fill_style_str("rgba(100,120,200,0.3)");
                self.ctx
                    .fill_rect(box_x + 10.0, y - 18.0, box_w - 20.0, 24.0);
            }
            self.ctx
                .set_fill_style_str(if selected { "#ffffff" } else { "#aaaaaa" });
            self.ctx.set_font("14px monospace");
            self.ctx.set_text_align("left");
            let name = match i {
                0 => equipment_name(player.weapon, player.enchantments[0]),
                1 => equipment_name(player.armor, player.enchantments[1]),
                _ => equipment_name(player.charm, player.enchantments[2]),
            };
            self.ctx
                .fill_text(&format!("{}: {}", equips[i], name), box_x + 20.0, y)
                .ok();
            y += 28.0;
        }

        // Items
        if player.items.is_empty() {
            self.ctx.set_fill_style_str("#888");
            self.ctx
                .fill_text("(Empty Inventory)", box_x + 20.0, y)
                .ok();
        } else {
            for (i, label) in item_labels.iter().enumerate() {
                let display_idx = 3 + i;
                let selected = cursor == display_idx;

                if selected {
                    self.ctx.set_fill_style_str("rgba(100,120,200,0.3)");
                    self.ctx
                        .fill_rect(box_x + 10.0, y - 18.0, box_w - 20.0, 24.0);
                }

                let color = if i == source_idx {
                    "#6688aa"
                } else if selected {
                    "#ffffff"
                } else {
                    "#aaaaaa"
                };
                self.ctx.set_fill_style_str(color);
                self.ctx.set_font("14px monospace");
                self.ctx.set_text_align("left");

                let suffix = if i == source_idx { " (Source)" } else { "" };
                self.ctx
                    .fill_text(&format!("{}{}", label, suffix), box_x + 20.0, y)
                    .ok();
                y += 28.0;
            }
        }

        self.ctx.set_text_align("center");
        self.ctx.set_fill_style_str("#7784aa");
        self.ctx.set_font("11px monospace");
        self.ctx
            .fill_text(
                "Enter=select  Esc=cancel",
                self.canvas_w / 2.0,
                box_y + box_h - 12.0,
            )
            .ok();
    }

    fn draw_help_overlay(&self, combat: &CombatState, listening_mode: ListenMode) {
        let mut lines = vec![
            "Explore: WASD/Arrows move  1-5 use items".to_string(),
            "I inventory  B spellbook  C codex  V look  O options".to_string(),
            format!(
                "L listening ({})  X skip floor  ? toggle help",
                listening_mode.label()
            ),
        ];

        let mode_title = match combat {
            CombatState::Fighting { .. } => {
                lines.push("Combat: Enter submit  Q cycle spell  Space cast".to_string());
                lines.push("Esc flee  Elite compounds break one syllable at a time".to_string());
                if listening_mode.is_active() {
                    lines.push("R replay the heard tone during audio fights".to_string());
                }
                "Combat Controls"
            }
            CombatState::Forging { .. } => {
                lines.push("Forge: ↑/↓ browse recipes  1-9 quick pick".to_string());
                lines.push("Enter forge  E enchant  Esc close".to_string());
                "Forge Controls"
            }
            CombatState::Enchanting { .. } => {
                lines.push("Enchant: 1-3 or ↑↓+Enter = pick slot".to_string());
                lines.push("Then 1-6 = radical  ←/→ page  Esc back".to_string());
                "Enchant Controls"
            }
            CombatState::Shopping { .. } => {
                lines.push("Shop: Up/Down browse  Enter buy  Esc leave".to_string());
                "Shop Controls"
            }
            CombatState::SentenceChallenge { .. } => {
                lines.push("Sentence: <-/-> select  Enter pick".to_string());
                lines.push("Backspace undo  Esc skip".to_string());
                "Sentence Controls"
            }
            CombatState::ToneBattle { .. } => {
                lines.push("Tone battle: 1-4 answer tones".to_string());
                lines.push("Listen for the contour, not just the vowel".to_string());
                "Tone Controls"
            }
            CombatState::Looking { .. } => {
                lines.push("Look: WASD/Arrows move the cursor up to 3 tiles".to_string());
                lines.push("Enter, V, or Esc close  inspect enemies and terrain".to_string());
                "Look Controls"
            }
            CombatState::ClassSelect => {
                lines.push("Class select: 1 Scholar  2 Warrior  3 Alchemist".to_string());
                lines.push("D daily challenge  T talents".to_string());
                "Menu Controls"
            }
            CombatState::GameOver => {
                lines.push("Game over: R restart  T talents  I inventory".to_string());
                "Game Over Controls"
            }
            CombatState::Explore => {
                lines.push("Q cycle spell  Space cast (offensive spells aim first)".to_string());
                lines.push(
                    "Script seals can flood rooms, raise spikes, or summon ambushes.".to_string(),
                );
                "Quick Reference"
            }
            CombatState::Offering { .. } => {
                lines.push("Altar: Select item to sacrifice for piety".to_string());
                lines.push("P pray (costs 20 piety)  Esc cancel".to_string());
                "Altar Controls"
            }
            CombatState::DippingSource { .. } => {
                lines.push("Dipping: Select a potion to apply".to_string());
                "Dip Controls"
            }
            CombatState::DippingTarget { .. } => {
                lines.push("Dipping: Select weapon/armor/charm to coat".to_string());
                "Dip Controls"
            }
            CombatState::StrokeOrder { .. } => {
                lines.push("Stroke: ↑/↓ select  Enter place  Backspace undo".to_string());
                lines.push("Arrange components in correct writing order  Esc skip".to_string());
                "Stroke Order Controls"
            }
            CombatState::ToneDefense { .. } => {
                lines.push("Tone Wall: 1-4 pick the correct tone".to_string());
                lines.push("Block attacks! Wrong = 1 damage  Esc flee".to_string());
                "Tone Defense Controls"
            }
            CombatState::CompoundBuilder { .. } => {
                lines.push("Compound: ↑/↓ select  Enter place  Backspace undo".to_string());
                lines.push("Combine characters into a word  Esc skip".to_string());
                "Compound Builder Controls"
            }
            CombatState::ClassifierMatch { .. } => {
                lines.push("Classifier: 1-4 pick the correct measure word".to_string());
                lines.push("3 rounds — earn 5 gold per correct  Esc flee".to_string());
                "Classifier Match Controls"
            }
            CombatState::Aiming { .. } => {
                lines.push("Aim: Arrows pick direction  Enter/Space fire".to_string());
                lines.push("Esc cancel  Spell flies until it hits a wall or enemy".to_string());
                "Aim Controls"
            }
            CombatState::InkWellChallenge { .. } => {
                lines.push("Ink Well: 1-9 guess number of components".to_string());
                lines.push("Correct = +1 HP  Esc leave".to_string());
                "Ink Well Controls"
            }
            CombatState::AncestorChallenge { .. } => {
                lines.push("Ancestor Shrine: 1-4 complete the chengyu".to_string());
                lines.push("Correct = +10 gold  Esc leave".to_string());
                "Ancestor Shrine Controls"
            }
            CombatState::TranslationChallenge { .. } => {
                lines.push("Translation: 1-4 pick Chinese for the meaning".to_string());
                lines.push("3 rounds, 2+ correct = +1 max HP  Esc leave".to_string());
                "Translation Controls"
            }
            CombatState::RadicalGardenChallenge { .. } => {
                lines.push("Radical Garden: 1-4 identify the radical".to_string());
                lines.push("Correct = free radical for inventory  Esc leave".to_string());
                "Radical Garden Controls"
            }
            CombatState::MirrorPoolChallenge { .. } => {
                lines.push("Mirror Pool: type pinyin, Enter submit".to_string());
                lines.push("Correct = +1 spell power  Backspace delete  Esc leave".to_string());
                "Mirror Pool Controls"
            }
            CombatState::StoneTutorChallenge { .. } => {
                lines.push("Stone Tutor: Space to advance from study to quiz".to_string());
                lines.push("Quiz: 1-4 pick tone  3 rounds  Esc leave".to_string());
                "Stone Tutor Controls"
            }
            CombatState::CodexChallenge { .. } => {
                lines.push("Codex Shrine: 1-4 pick correct meaning".to_string());
                lines.push("3 rounds — earn 5 gold per correct  Esc leave".to_string());
                "Codex Shrine Controls"
            }
            CombatState::Journal { .. } => {
                lines.push("Journal: browse encountered characters".to_string());
                lines.push("←/→ change page  Esc or J close".to_string());
                "Journal Controls"
            }
            CombatState::WordBridgeChallenge { .. } => {
                lines.push("Word Bridge: 1-4 pick the matching character".to_string());
                lines.push("Correct = bridge over water  Esc leave".to_string());
                "Word Bridge Controls"
            }
            CombatState::LockedDoorChallenge { .. } => {
                lines.push("Locked Door: 1-4 pick the correct meaning".to_string());
                lines.push("Correct = door opens  Wrong = -1 HP  Esc leave".to_string());
                "Locked Door Controls"
            }
            CombatState::CursedFloorChallenge { .. } => {
                lines.push("Cursed Floor: 1-4 pick the correct tone".to_string());
                lines.push("Correct = +1 gold  Wrong = -2 gold".to_string());
                "Cursed Floor Controls"
            }
        };

        let box_w = 350.0;
        let box_h = 50.0 + lines.len() as f64 * 16.0;
        let box_x = 14.0;
        let box_y = 92.0;

        self.ctx.set_fill_style_str("rgba(8,12,22,0.88)");
        self.ctx.fill_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("#7f9cff");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

        self.ctx.set_text_align("left");
        self.ctx.set_fill_style_str("#c8d8ff");
        self.ctx.set_font("bold 14px monospace");
        self.ctx
            .fill_text(mode_title, box_x + 12.0, box_y + 20.0)
            .ok();
        self.ctx.set_fill_style_str("#8fa8ff");
        self.ctx.set_font("10px monospace");
        self.ctx
            .fill_text("Help Overlay", box_x + box_w - 92.0, box_y + 20.0)
            .ok();

        self.ctx.set_fill_style_str("#dbe7ff");
        self.ctx.set_font("11px monospace");
        for (idx, line) in lines.iter().enumerate() {
            self.ctx
                .fill_text(line, box_x + 12.0, box_y + 40.0 + idx as f64 * 16.0)
                .ok();
        }
    }

    fn draw_room_ambience(&self, room_modifier: Option<crate::dungeon::RoomModifier>, anim_t: f64) {
        match room_modifier {
            Some(crate::dungeon::RoomModifier::Dark) => {
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
            Some(crate::dungeon::RoomModifier::Arcane) => {
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
            Some(crate::dungeon::RoomModifier::Cursed) => {
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
            None => {}
        }
    }

    fn draw_minimap(&self, level: &DungeonLevel, player: &Player) {
        let mm_scale = 2.0;
        let mm_w = level.width as f64 * mm_scale;
        let mm_h = level.height as f64 * mm_scale;
        let mm_x = self.canvas_w - mm_w - 8.0;
        let mm_y = self.canvas_h - mm_h - 8.0;

        // Background
        self.ctx.set_fill_style_str("rgba(0,0,0,0.6)");
        self.ctx
            .fill_rect(mm_x - 2.0, mm_y - 2.0, mm_w + 4.0, mm_h + 4.0);

        for ty in 0..level.height {
            for tx in 0..level.width {
                let idx = level.idx(tx, ty);
                if !level.revealed[idx] {
                    continue;
                }
                let tile = level.tiles[idx];
                if tile == Tile::Wall {
                    continue;
                }
                let color = if level.visible[idx] {
                    "rgba(150,140,180,0.7)"
                } else {
                    "rgba(80,70,100,0.5)"
                };
                self.ctx.set_fill_style_str(color);
                self.ctx.fill_rect(
                    mm_x + tx as f64 * mm_scale,
                    mm_y + ty as f64 * mm_scale,
                    mm_scale,
                    mm_scale,
                );
            }
        }

        // Player dot
        self.ctx.set_fill_style_str(COL_PLAYER);
        self.ctx.fill_rect(
            mm_x + player.x as f64 * mm_scale - 0.5,
            mm_y + player.y as f64 * mm_scale - 0.5,
            mm_scale + 1.0,
            mm_scale + 1.0,
        );
    }

    pub fn draw_inventory(
        &self,
        player: &Player,
        floor_num: i32,
        recipes_found: usize,
        best_floor: i32,
        total_kills: u32,
        companion: Option<crate::game::Companion>,
        item_labels: &[String],
        inventory_cursor: usize,
        inventory_inspect: Option<usize>,
    ) {
        let box_x = 24.0;
        let box_y = 28.0;
        let box_w = self.canvas_w - 48.0;
        let box_h = self.canvas_h - 56.0;

        self.ctx.set_fill_style_str("rgba(0,0,0,0.88)");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

        self.ctx.set_fill_style_str("rgba(18,20,32,0.98)");
        self.ctx.fill_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("#5e6ea8");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

        self.ctx.set_text_align("center");
        self.ctx.set_font("bold 22px monospace");
        self.ctx.set_fill_style_str("#ffcc33");
        self.ctx
            .fill_text("Inventory", self.canvas_w / 2.0, box_y + 28.0)
            .ok();
        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#9aaad8");
        self.ctx
            .fill_text(
                "I/Esc close   ↑↓ navigate items   Enter inspect",
                self.canvas_w / 2.0,
                box_y + 46.0,
            )
            .ok();

        let class_name = match player.class {
            crate::player::PlayerClass::Scholar => "Scholar",
            crate::player::PlayerClass::Warrior => "Warrior",
            crate::player::PlayerClass::Alchemist => "Alchemist",
        };
        let companion_text = companion
            .map(|ally| format!("{} {}", ally.icon(), ally.name()))
            .unwrap_or_else(|| "No companion".to_string());

        self.ctx.set_font("12px monospace");
        self.ctx.set_text_align("left");
        self.ctx.set_fill_style_str("#dde7ff");
        self.ctx
            .fill_text(
                &format!(
                    "Floor {}   HP {}/{}   Gold {}   Class {}",
                    floor_num, player.hp, player.max_hp, player.gold, class_name
                ),
                box_x + 18.0,
                box_y + 66.0,
            )
            .ok();
        self.ctx.set_text_align("right");
        self.ctx
            .fill_text(&companion_text, box_x + box_w - 18.0, box_y + 66.0)
            .ok();

        let panel_y = box_y + 82.0;
        let panel_h = box_h - 110.0;
        let gap = 16.0;
        let left_x = box_x + 16.0;
        let left_w = 232.0;
        let mid_x = left_x + left_w + gap;
        let mid_w = 248.0;
        let right_x = mid_x + mid_w + gap;
        let right_w = box_x + box_w - 16.0 - right_x;

        self.ctx.set_fill_style_str("rgba(255,255,255,0.03)");
        self.ctx.fill_rect(left_x, panel_y, left_w, panel_h);
        self.ctx.fill_rect(mid_x, panel_y, mid_w, panel_h);
        self.ctx.fill_rect(right_x, panel_y, right_w, panel_h);
        self.ctx.set_stroke_style_str("#39456c");
        self.ctx.stroke_rect(left_x, panel_y, left_w, panel_h);
        self.ctx.stroke_rect(mid_x, panel_y, mid_w, panel_h);
        self.ctx.stroke_rect(right_x, panel_y, right_w, panel_h);

        self.ctx.set_font("bold 13px monospace");
        self.ctx.set_text_align("left");
        self.ctx.set_fill_style_str("#89a3ff");
        self.ctx
            .fill_text("Loadout", left_x + 12.0, panel_y + 22.0)
            .ok();
        self.ctx
            .fill_text("Spells", mid_x + 12.0, panel_y + 22.0)
            .ok();
        self.ctx
            .fill_text("Radicals & Progress", right_x + 12.0, panel_y + 22.0)
            .ok();

        let mut left_y = panel_y + 44.0;
        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#9ab0d7");
        self.ctx.fill_text("Equipment", left_x + 12.0, left_y).ok();
        left_y += 18.0;

        let equip_slots: [(
            &str,
            Option<&crate::player::Equipment>,
            Option<&'static str>,
        ); 3] = [
            ("Weapon", player.weapon, player.enchantments[0]),
            ("Armor ", player.armor, player.enchantments[1]),
            ("Charm ", player.charm, player.enchantments[2]),
        ];
        for (slot_idx, (label, equip, enchant)) in equip_slots.iter().enumerate() {
            let selected = inventory_cursor == slot_idx;
            if selected {
                self.ctx.set_fill_style_str("rgba(255,204,51,0.15)");
                self.ctx
                    .fill_rect(left_x + 8.0, left_y - 12.0, left_w - 16.0, 16.0);
            }
            self.ctx
                .set_fill_style_str(if selected { "#ffcc33" } else { "#dde7ff" });
            let marker = if selected { "▸" } else { " " };
            self.ctx
                .fill_text(
                    &format!("{} {}: {}", marker, label, equipment_name(*equip, *enchant)),
                    left_x + 12.0,
                    left_y,
                )
                .ok();
            if let Some(equipment) = equip {
                if let Some(icon) = equipment_sprite_key(equipment.name) {
                    self.draw_sprite_icon(icon, left_x + left_w - 26.0, left_y - 12.0, 14.0);
                }
            }
            left_y += 16.0;
        }

        left_y += 26.0;
        self.ctx.set_fill_style_str("#9ab0d7");
        self.ctx
            .fill_text("Consumables", left_x + 12.0, left_y)
            .ok();
        left_y += 18.0;
        self.ctx.set_fill_style_str("#dde7ff");
        if player.items.is_empty() {
            self.ctx
                .fill_text("No consumables picked up yet.", left_x + 12.0, left_y)
                .ok();
            left_y += 16.0;
        } else {
            for (idx, label) in item_labels.iter().enumerate() {
                let selected = inventory_cursor == idx + 3;
                if selected {
                    self.ctx.set_fill_style_str("rgba(255,204,51,0.15)");
                    self.ctx
                        .fill_rect(left_x + 8.0, left_y - 12.0, left_w - 16.0, 16.0);
                }
                self.ctx
                    .set_fill_style_str(if selected { "#ffcc33" } else { "#dde7ff" });
                if let Some(item) = player.items.get(idx) {
                    self.draw_sprite_icon(
                        item_sprite_key(item),
                        left_x + 12.0,
                        left_y - 11.0,
                        12.0,
                    );
                }
                let marker = if selected { "▸" } else { " " };
                self.ctx
                    .fill_text(
                        &format!("{} {}. {}", marker, idx + 1, label),
                        left_x + 28.0,
                        left_y,
                    )
                    .ok();
                left_y += 16.0;
            }
        }

        left_y += 10.0;
        self.ctx.set_fill_style_str("#9ab0d7");
        self.ctx
            .fill_text("Active effects", left_x + 12.0, left_y)
            .ok();
        left_y += 18.0;
        if player.shield {
            self.ctx.set_fill_style_str("#7fd8ff");
            self.ctx
                .fill_text("🛡 Shield Active", left_x + 12.0, left_y)
                .ok();
            left_y += 16.0;
        }
        if player.statuses.is_empty() {
            self.ctx.set_fill_style_str("#dde7ff");
            self.ctx
                .fill_text("No temporary effects active.", left_x + 12.0, left_y)
                .ok();
        } else {
            for status in &player.statuses {
                self.ctx.set_fill_style_str(status.color());
                self.ctx
                    .fill_text(
                        &format!("{} ({} turns)", status.label(), status.turns_left),
                        left_x + 12.0,
                        left_y,
                    )
                    .ok();
                left_y += 16.0;
            }
        }

        let mut spell_y = panel_y + 44.0;
        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#9ab0d7");
        self.ctx
            .fill_text("Forged characters", mid_x + 12.0, spell_y)
            .ok();
        spell_y += 18.0;
        if player.spells.is_empty() {
            self.ctx.set_fill_style_str("#dde7ff");
            self.ctx
                .fill_text("Forge characters to unlock spells.", mid_x + 12.0, spell_y)
                .ok();
        } else {
            let max_spells = (((panel_h - 70.0) / 38.0) as usize).max(1);
            for (idx, spell) in player.spells.iter().take(max_spells).enumerate() {
                let selected = idx == player.selected_spell;
                if selected {
                    self.ctx.set_fill_style_str("rgba(255,204,51,0.14)");
                    self.ctx
                        .fill_rect(mid_x + 8.0, spell_y - 14.0, mid_w - 16.0, 30.0);
                }
                self.ctx.set_font("bold 13px monospace");
                self.ctx
                    .set_fill_style_str(if selected { "#ffdd88" } else { "#dde7ff" });
                let marker = if selected { "►" } else { " " };
                self.draw_sprite_icon(
                    spell_sprite_key(&spell.effect),
                    mid_x + 12.0,
                    spell_y - 11.0,
                    12.0,
                );
                self.ctx
                    .fill_text(
                        &format!("{} {} {}", marker, spell.hanzi, spell.pinyin),
                        mid_x + 28.0,
                        spell_y,
                    )
                    .ok();
                spell_y += 14.0;
                self.ctx.set_font("11px monospace");
                self.ctx.set_fill_style_str("#9fc2ff");
                self.ctx
                    .fill_text(
                        &format!("{} — {}", spell.effect.label(), spell.meaning),
                        mid_x + 24.0,
                        spell_y,
                    )
                    .ok();
                spell_y += 24.0;
            }

            if player.spells.len() > max_spells {
                self.ctx.set_font("11px monospace");
                self.ctx.set_fill_style_str("#7e8dbb");
                self.ctx
                    .fill_text(
                        &format!("...and {} more", player.spells.len() - max_spells),
                        mid_x + 12.0,
                        spell_y,
                    )
                    .ok();
            }
        }

        let mut right_y = panel_y + 44.0;
        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#dde7ff");
        self.ctx
            .fill_text(
                &format!("Radicals carried: {}", player.radicals.len()),
                right_x + 12.0,
                right_y,
            )
            .ok();
        right_y += 16.0;
        self.ctx
            .fill_text(
                &format!("Recipes known: {}", recipes_found),
                right_x + 12.0,
                right_y,
            )
            .ok();
        right_y += 16.0;
        self.ctx
            .fill_text(
                &format!("Best floor: {}", best_floor),
                right_x + 12.0,
                right_y,
            )
            .ok();
        right_y += 16.0;
        self.ctx
            .fill_text(
                &format!("Total kills: {}", total_kills),
                right_x + 12.0,
                right_y,
            )
            .ok();

        right_y += 26.0;
        self.ctx.set_fill_style_str("#9ab0d7");
        self.ctx
            .fill_text("Grouped radicals", right_x + 12.0, right_y)
            .ok();
        right_y += 18.0;

        let radical_counts = radical_stack_counts(&player.radicals);
        if radical_counts.is_empty() {
            self.ctx.set_fill_style_str("#dde7ff");
            self.ctx
                .fill_text("No radicals picked up yet.", right_x + 12.0, right_y)
                .ok();
        } else {
            let available_rows = ((panel_y + panel_h - right_y - 16.0) / 16.0).floor() as usize;
            let rows_per_col = available_rows.max(1);
            let col_w = (right_w - 24.0) / 2.0;
            for (idx, (radical, count)) in radical_counts.iter().take(rows_per_col * 2).enumerate()
            {
                let col = idx / rows_per_col;
                let row = idx % rows_per_col;
                let x = right_x + 12.0 + col as f64 * col_w;
                let y = right_y + row as f64 * 16.0;
                self.ctx.set_fill_style_str("#ffb566");
                self.ctx.set_font("13px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.fill_text(radical, x, y).ok();
                self.ctx.set_fill_style_str("#dde7ff");
                self.ctx.set_font("11px monospace");
                self.ctx
                    .fill_text(&format!(" x{}", count), x + 16.0, y)
                    .ok();
            }

            if radical_counts.len() > rows_per_col * 2 {
                self.ctx.set_fill_style_str("#7e8dbb");
                self.ctx.set_font("11px monospace");
                self.ctx
                    .fill_text(
                        &format!(
                            "...and {} more stacks",
                            radical_counts.len() - rows_per_col * 2
                        ),
                        right_x + 12.0,
                        panel_y + panel_h - 12.0,
                    )
                    .ok();
            }
        }

        self.ctx.set_text_align("center");
        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#7784aa");
        let footer = if item_labels.iter().any(|label| label.starts_with('?')) {
            "Mystery seals identify themselves on use. Use 1-5 in exploration to test them."
        } else {
            "Use 1-5 in exploration to consume items. Selected spell is highlighted here for combat."
        };
        self.ctx
            .fill_text(footer, self.canvas_w / 2.0, box_y + box_h - 16.0)
            .ok();

        if let Some(inspect_idx) = inventory_inspect {
            let (popup_name, popup_desc): (String, String) = if inspect_idx < 3 {
                let equip_opt = match inspect_idx {
                    0 => player.weapon,
                    1 => player.armor,
                    _ => player.charm,
                };
                if let Some(eq) = equip_opt {
                    let enchant = player.enchantments[inspect_idx];
                    let name = if let Some(ench_str) = enchant {
                        format!("{} +{}", eq.name, ench_str)
                    } else {
                        eq.name.to_string()
                    };
                    (name, eq.description())
                } else {
                    let slot = match inspect_idx {
                        0 => "Weapon",
                        1 => "Armor",
                        _ => "Charm",
                    };
                    (
                        format!("{}: Empty", slot),
                        "No equipment in this slot.".to_string(),
                    )
                }
            } else if let Some(item) = player.items.get(inspect_idx - 3) {
                (item.name().to_string(), item.description().to_string())
            } else {
                ("???".to_string(), "Unknown item.".to_string())
            };

            let pop_w = 320.0;
            let pop_h = 100.0;
            let pop_x = (self.canvas_w - pop_w) / 2.0;
            let pop_y = (self.canvas_h - pop_h) / 2.0;

            self.ctx.set_fill_style_str("rgba(10,8,24,0.96)");
            self.ctx.fill_rect(pop_x, pop_y, pop_w, pop_h);
            self.ctx.set_stroke_style_str("#ffcc33");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(pop_x, pop_y, pop_w, pop_h);

            self.ctx.set_fill_style_str("#ffcc33");
            self.ctx.set_font("bold 14px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(&popup_name, self.canvas_w / 2.0, pop_y + 24.0)
                .ok();

            self.ctx.set_fill_style_str("#ccdaff");
            self.ctx.set_font("11px monospace");
            self.ctx.set_text_align("left");
            let max_line_chars = 40;
            let mut dy = pop_y + 46.0;
            for line in word_wrap(&popup_desc, max_line_chars) {
                self.ctx.fill_text(&line, pop_x + 16.0, dy).ok();
                dy += 14.0;
            }

            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("10px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    "Esc / Backspace = close",
                    self.canvas_w / 2.0,
                    pop_y + pop_h - 10.0,
                )
                .ok();
        }
    }

    pub fn draw_spellbook(&self, player: &Player) {
        let box_x = 40.0;
        let box_y = 40.0;
        let box_w = self.canvas_w - 80.0;
        let box_h = self.canvas_h - 80.0;

        self.ctx.set_fill_style_str("rgba(0,0,0,0.88)");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

        self.ctx.set_fill_style_str("rgba(18,16,36,0.98)");
        self.ctx.fill_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("#7b5ebd");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

        self.ctx.set_text_align("center");
        self.ctx.set_font("bold 22px monospace");
        self.ctx.set_fill_style_str("#cc99ff");
        self.ctx
            .fill_text("Spellbook", self.canvas_w / 2.0, box_y + 28.0)
            .ok();
        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#9aaad8");
        self.ctx
            .fill_text("B / Esc to close", self.canvas_w / 2.0, box_y + 46.0)
            .ok();

        if player.spells.is_empty() {
            self.ctx.set_fill_style_str("#888");
            self.ctx.set_font("14px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    "No spells forged yet. Use a Forge to combine radicals!",
                    self.canvas_w / 2.0,
                    self.canvas_h / 2.0,
                )
                .ok();
            return;
        }

        let col_w = (box_w - 32.0) / 2.0;
        let col_x = [box_x + 16.0, box_x + 16.0 + col_w];
        let mut col_y = [box_y + 66.0, box_y + 66.0];
        let max_y = box_y + box_h - 30.0;

        for (idx, spell) in player.spells.iter().enumerate() {
            let col = if col_y[0] <= col_y[1] { 0 } else { 1 };
            let x = col_x[col];
            let y = &mut col_y[col];

            if *y + 60.0 > max_y {
                continue;
            }

            let selected = idx == player.selected_spell;
            if selected {
                self.ctx.set_fill_style_str("rgba(204,153,255,0.1)");
                self.ctx.fill_rect(x, *y - 4.0, col_w - 8.0, 56.0);
            }

            self.draw_sprite_icon(spell_sprite_key(&spell.effect), x + 4.0, *y - 2.0, 14.0);

            self.ctx.set_font("bold 14px monospace");
            self.ctx
                .set_fill_style_str(if selected { "#ffdd88" } else { "#dde7ff" });
            self.ctx.set_text_align("left");
            let marker = if selected { "►" } else { " " };
            self.ctx
                .fill_text(
                    &format!(
                        "{} {} {} — {}",
                        marker,
                        spell.hanzi,
                        spell.pinyin,
                        spell.effect.label()
                    ),
                    x + 22.0,
                    *y + 10.0,
                )
                .ok();

            self.ctx.set_font("11px monospace");
            self.ctx.set_fill_style_str("#aab8dd");
            self.ctx
                .fill_text(&format!("\"{}\"", spell.meaning), x + 22.0, *y + 26.0)
                .ok();

            self.ctx.set_fill_style_str("#8899bb");
            self.ctx
                .fill_text(&spell.effect.description(), x + 22.0, *y + 40.0)
                .ok();

            *y += 56.0;
        }

        self.ctx.set_fill_style_str("#7784aa");
        self.ctx.set_font("11px monospace");
        self.ctx.set_text_align("center");
        self.ctx
            .fill_text(
                "Use Q/E in exploration to switch selected spell. Press number keys in combat to cast.",
                self.canvas_w / 2.0,
                box_y + box_h - 12.0,
            )
            .ok();
    }

    /// Draw the character codex overlay.
    pub fn draw_codex(&self, entries: &[&crate::codex::CodexEntry]) {
        // Dim background
        self.ctx.set_fill_style_str("rgba(0,0,0,0.85)");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

        // Title
        self.ctx.set_font("bold 20px monospace");
        self.ctx.set_text_align("center");
        self.ctx.set_fill_style_str("#ffcc33");
        self.ctx
            .fill_text("📖 Character Codex", self.canvas_w / 2.0, 35.0)
            .ok();

        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#aaaaaa");
        self.ctx
            .fill_text(
                &format!(
                    "{} characters encountered — Press C or Esc to close",
                    entries.len()
                ),
                self.canvas_w / 2.0,
                55.0,
            )
            .ok();

        // Column headers
        let y_start = 80.0;
        let row_h = 20.0;
        self.ctx.set_font("bold 12px monospace");
        self.ctx.set_text_align("left");
        self.ctx.set_fill_style_str("#888888");
        self.ctx.fill_text("Char", 30.0, y_start).ok();
        self.ctx.fill_text("Pinyin", 100.0, y_start).ok();
        self.ctx.fill_text("Meaning", 240.0, y_start).ok();
        self.ctx.fill_text("Seen", 450.0, y_start).ok();
        self.ctx.fill_text("Acc%", 520.0, y_start).ok();

        // Separator
        self.ctx.set_stroke_style_str("#444444");
        self.ctx.begin_path();
        self.ctx.move_to(20.0, y_start + 6.0);
        self.ctx.line_to(self.canvas_w - 20.0, y_start + 6.0);
        self.ctx.stroke();

        // Entries (max ~23 rows that fit on screen)
        let max_rows = ((self.canvas_h - y_start - 30.0) / row_h) as usize;
        self.ctx.set_font("14px 'Noto Serif SC', 'SimSun', serif");
        for (i, entry) in entries.iter().take(max_rows).enumerate() {
            let y = y_start + 10.0 + (i as f64 + 1.0) * row_h;
            let acc = entry.accuracy();
            let color = if acc >= 0.8 {
                "#66dd66"
            } else if acc >= 0.5 {
                "#dddd66"
            } else {
                "#dd6666"
            };

            // Hanzi
            self.ctx.set_fill_style_str("#ffffff");
            self.ctx.set_font("16px 'Noto Serif SC', 'SimSun', serif");
            self.ctx.fill_text(entry.hanzi, 30.0, y).ok();

            // Pinyin
            self.ctx.set_font("12px monospace");
            self.ctx.set_fill_style_str("#cccccc");
            self.ctx.fill_text(entry.pinyin, 100.0, y).ok();

            // Meaning
            self.ctx.set_fill_style_str("#aaaacc");
            // Truncate long meanings
            let meaning = if entry.meaning.len() > 24 {
                &entry.meaning[..24]
            } else {
                entry.meaning
            };
            self.ctx.fill_text(meaning, 240.0, y).ok();

            // Times seen
            self.ctx.set_fill_style_str("#cccccc");
            self.ctx
                .fill_text(&entry.times_seen.to_string(), 450.0, y)
                .ok();

            // Accuracy
            self.ctx.set_fill_style_str(color);
            self.ctx
                .fill_text(&format!("{:.0}%", acc * 100.0), 520.0, y)
                .ok();
        }

        if entries.len() > max_rows {
            self.ctx.set_font("11px monospace");
            self.ctx.set_text_align("center");
            self.ctx.set_fill_style_str("#666666");
            self.ctx
                .fill_text(
                    &format!("...and {} more", entries.len() - max_rows),
                    self.canvas_w / 2.0,
                    self.canvas_h - 10.0,
                )
                .ok();
        }
    }
}

fn tile_sprite_key(tile: Tile) -> &'static str {
    match tile {
        Tile::Wall => "tile_wall",
        Tile::CrackedWall => "tile_cracked_wall",
        Tile::BrittleWall => "tile_brittle_wall",
        Tile::Floor => "tile_floor",
        Tile::Corridor => "tile_corridor",
        Tile::StairsDown => "tile_stairs_down",
        Tile::Forge => "tile_forge",
        Tile::Shop => "tile_shop",
        Tile::Chest => "tile_chest",
        Tile::Crate => "tile_crate",
        Tile::Spikes => "tile_spikes",
        Tile::Oil => "tile_oil",
        Tile::Water => "tile_water",
        Tile::DeepWater => "tile_deep_water",
        Tile::Bridge => "tile_bridge",
        Tile::Shrine => "obj_shrine",
        Tile::StrokeShrine => "obj_shrine",
        Tile::ToneWall => "obj_shrine",
        Tile::CompoundShrine => "obj_shrine",
        Tile::ClassifierShrine => "obj_shrine",
        Tile::InkWell => "obj_shrine",
        Tile::AncestorShrine => "obj_shrine",
        Tile::TranslationAltar => "obj_shrine",
        Tile::RadicalGarden => "obj_shrine",
        Tile::MirrorPool => "obj_shrine",
        Tile::StoneTutor => "obj_shrine",
        Tile::CodexShrine => "obj_shrine",
        Tile::WordBridge => "tile_bridge",
        Tile::LockedDoor => "obj_shrine",
        Tile::CursedFloor => "tile_floor",
        Tile::Altar(AltarKind::Jade) => "obj_altar_jade",
        Tile::Altar(AltarKind::Gale) => "obj_altar_gale",
        Tile::Altar(AltarKind::Mirror) => "obj_altar_mirror",
        Tile::Altar(AltarKind::Iron) => "obj_altar_iron",
        Tile::Altar(AltarKind::Gold) => "obj_altar_gold",
        Tile::Seal(SealKind::Ember) => "obj_seal_ember",
        Tile::Seal(SealKind::Tide) => "obj_seal_tide",
        Tile::Seal(SealKind::Thorn) => "obj_seal_thorn",
        Tile::Seal(SealKind::Echo) => "obj_seal_echo",
        Tile::Sign(_) => "obj_sign",
        Tile::Npc(0) => "npc_teacher",
        Tile::Npc(1) => "npc_monk",
        Tile::Npc(2) => "npc_merchant",
        Tile::Npc(_) => "npc_guard",
    }
}

fn boss_sprite_key(kind: BossKind) -> &'static str {
    match kind {
        BossKind::Gatekeeper => "boss_gatekeeper",
        BossKind::Scholar => "boss_scholar",
        BossKind::Elementalist => "boss_elementalist",
        BossKind::MimicKing => "boss_mimic_king",
        BossKind::InkSage => "boss_ink_sage",
        BossKind::RadicalThief => "boss_radical_thief",
    }
}

fn item_sprite_key(item: &Item) -> &'static str {
    match item.kind() {
        ItemKind::HealthPotion => "item_health_potion",
        ItemKind::PoisonFlask => "item_poison_flask",
        ItemKind::RevealScroll => "item_reveal_scroll",
        ItemKind::TeleportScroll => "item_teleport_scroll",
        ItemKind::HastePotion => "item_haste_potion",
        ItemKind::StunBomb => "item_stun_bomb",
    }
}

fn spell_sprite_key(effect: &radical::SpellEffect) -> &'static str {
    match effect {
        radical::SpellEffect::FireAoe(_) => "spell_fire",
        radical::SpellEffect::Heal(_) => "spell_heal",
        radical::SpellEffect::Reveal => "spell_reveal",
        radical::SpellEffect::Shield => "spell_shield",
        radical::SpellEffect::StrongHit(_) => "spell_strike",
        radical::SpellEffect::Drain(_) => "spell_drain",
        radical::SpellEffect::Stun => "spell_stun",
        radical::SpellEffect::Pacify => "spell_pacify",
    }
}

fn equipment_sprite_key(name: &str) -> Option<&'static str> {
    match name {
        "Brush of Clarity" => Some("equip_brush_of_clarity"),
        "Scholar's Quill" => Some("equip_scholars_quill"),
        "Dragon Fang Pen" => Some("equip_dragon_fang_pen"),
        "Iron Pickaxe" => Some("equip_iron_pickaxe"),
        "Jade Vest" => Some("equip_jade_vest"),
        "Iron Silk Robe" => Some("equip_iron_silk_robe"),
        "Phoenix Mantle" => Some("equip_phoenix_mantle"),
        "Radical Magnet" => Some("equip_radical_magnet"),
        "Life Jade" => Some("equip_life_jade"),
        "Gold Toad" => Some("equip_gold_toad"),
        "Phoenix Feather" => Some("equip_phoenix_feather"),
        _ => None,
    }
}

fn shop_item_sprite_key(kind: &ShopItemKind) -> Option<&'static str> {
    match kind {
        ShopItemKind::Radical(_) => None,
        ShopItemKind::HealFull => Some("item_health_potion"),
        ShopItemKind::Equipment(idx) => {
            let eq = crate::player::EQUIPMENT_POOL.get(*idx)?;
            equipment_sprite_key(eq.name)
        }
        ShopItemKind::Consumable(item) => Some(item_sprite_key(item)),
    }
}

fn hud_message_color(message: &str) -> &'static str {
    if message.starts_with("Wrong") || message.contains(" hits for ") || message.contains("resets!")
    {
        "#ff7777"
    } else if message.starts_with("⛓")
        || message.contains("Chain ")
        || message.contains("Compound broken")
    {
        "#ffbb66"
    } else if message.contains("Shield")
        || message.contains("Guard")
        || message.contains("stagger")
        || message.contains("stunned")
        || message.contains("counterattack")
    {
        "#66ddff"
    } else if message.starts_with("Defeated")
        || message.starts_with("Forged")
        || message.contains("Found")
        || message.contains("Bought")
        || message.contains("Talent learned")
    {
        "#88ff88"
    } else {
        "#ffdd88"
    }
}

fn equipment_name(
    equipment: Option<&crate::player::Equipment>,
    enchantment: Option<&'static str>,
) -> String {
    match (equipment, enchantment) {
        (Some(equipment), Some(enchantment)) => format!("{} +{}", equipment.name, enchantment),
        (Some(equipment), None) => equipment.name.to_string(),
        (None, _) => "None".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TilePalette {
    fill: &'static str,
    accent: Option<&'static str>,
    glyph: Option<&'static str>,
    glyph_color: &'static str,
}

fn tile_palette(tile: Tile, visible: bool) -> TilePalette {
    if visible {
        match tile {
            Tile::Wall => TilePalette {
                fill: COL_WALL,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CrackedWall => TilePalette {
                fill: "#47324f",
                accent: Some("#d89c74"),
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::BrittleWall => TilePalette {
                fill: "#5b473a",
                accent: Some("#f2d29e"),
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Floor => TilePalette {
                fill: COL_FLOOR,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Corridor => TilePalette {
                fill: COL_CORRIDOR,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::StairsDown => TilePalette {
                fill: COL_STAIRS,
                accent: Some("#d7e7ff"),
                glyph: Some("▼"),
                glyph_color: "#ffffff",
            },
            Tile::Forge => TilePalette {
                fill: COL_FORGE,
                accent: Some("#ffd1aa"),
                glyph: Some("⚒"),
                glyph_color: "#ffffff",
            },
            Tile::Shop => TilePalette {
                fill: COL_SHOP,
                accent: Some("#bfffd4"),
                glyph: Some("$"),
                glyph_color: "#ffffff",
            },
            Tile::Chest => TilePalette {
                fill: COL_CHEST,
                accent: Some("#ffe29e"),
                glyph: Some("◆"),
                glyph_color: "#fff7dc",
            },
            Tile::Crate => TilePalette {
                fill: "#6a4527",
                accent: Some("#a77b52"),
                glyph: Some("▣"),
                glyph_color: "#fff0d8",
            },
            Tile::Spikes => TilePalette {
                fill: "#7e434a",
                accent: Some("#d9a0a0"),
                glyph: Some("^"),
                glyph_color: "#fff1f1",
            },
            Tile::Oil => TilePalette {
                fill: "#4f3a1c",
                accent: Some("#e7c56d"),
                glyph: Some("~"),
                glyph_color: "#ffdd88",
            },
            Tile::Water => TilePalette {
                fill: "#4466cc",
                accent: Some("#9fc4ff"),
                glyph: Some("≈"),
                glyph_color: "#e5efff",
            },
            Tile::DeepWater => TilePalette {
                fill: "#16386d",
                accent: Some("#7fb4ff"),
                glyph: Some("≈"),
                glyph_color: "#e5efff",
            },
            Tile::Npc(0) => TilePalette {
                fill: "#3b5876",
                accent: Some("#7fccff"),
                glyph: Some("📚"),
                glyph_color: "#ffffff",
            },
            Tile::Npc(1) => TilePalette {
                fill: "#2f6c5d",
                accent: Some("#96ffd8"),
                glyph: Some("🧘"),
                glyph_color: "#ffffff",
            },
            Tile::Npc(2) => TilePalette {
                fill: "#5e5331",
                accent: Some("#ffd588"),
                glyph: Some("💰"),
                glyph_color: "#ffffff",
            },
            Tile::Npc(_) => TilePalette {
                fill: "#4e476a",
                accent: Some("#d2c4ff"),
                glyph: Some("🛡"),
                glyph_color: "#ffffff",
            },
            Tile::Shrine => TilePalette {
                fill: "#7d5d2a",
                accent: Some("#ffd07a"),
                glyph: Some("🔔"),
                glyph_color: "#fff8e2",
            },
            Tile::StrokeShrine => TilePalette {
                fill: "#1a2d4a",
                accent: Some("#88ccff"),
                glyph: Some("筆"),
                glyph_color: "#88ccff",
            },
            Tile::ToneWall => TilePalette {
                fill: "#3a1515",
                accent: Some("#dd6644"),
                glyph: Some("壁"),
                glyph_color: "#dd6644",
            },
            Tile::CompoundShrine => TilePalette {
                fill: "#1a3a1a",
                accent: Some("#66dd88"),
                glyph: Some("合"),
                glyph_color: "#66dd88",
            },
            Tile::ClassifierShrine => TilePalette {
                fill: "#3a2a1a",
                accent: Some("#ddaa44"),
                glyph: Some("量"),
                glyph_color: "#ddaa44",
            },
            Tile::InkWell => TilePalette {
                fill: "#1a1a2d",
                accent: Some("#9999ee"),
                glyph: Some("墨"),
                glyph_color: "#9999ee",
            },
            Tile::AncestorShrine => TilePalette {
                fill: "#2d1a1a",
                accent: Some("#ee9966"),
                glyph: Some("祖"),
                glyph_color: "#ee9966",
            },
            Tile::TranslationAltar => TilePalette {
                fill: "#1a2d2d",
                accent: Some("#66cccc"),
                glyph: Some("译"),
                glyph_color: "#66cccc",
            },
            Tile::RadicalGarden => TilePalette {
                fill: "#1a2d1a",
                accent: Some("#88ee66"),
                glyph: Some("部"),
                glyph_color: "#88ee66",
            },
            Tile::MirrorPool => TilePalette {
                fill: "#1a1a3a",
                accent: Some("#aaaaff"),
                glyph: Some("鏡"),
                glyph_color: "#aaaaff",
            },
            Tile::StoneTutor => TilePalette {
                fill: "#2d2d1a",
                accent: Some("#cccc66"),
                glyph: Some("石"),
                glyph_color: "#cccc66",
            },
            Tile::CodexShrine => TilePalette {
                fill: "#2a1a3a",
                accent: Some("#dd99ff"),
                glyph: Some("典"),
                glyph_color: "#dd99ff",
            },
            Tile::WordBridge => TilePalette {
                fill: "#1a1a2d",
                accent: Some("#66aaff"),
                glyph: Some("桥"),
                glyph_color: "#66aaff",
            },
            Tile::LockedDoor => TilePalette {
                fill: "#2d1a1a",
                accent: Some("#ff6644"),
                glyph: Some("锁"),
                glyph_color: "#ff6644",
            },
            Tile::CursedFloor => TilePalette {
                fill: "#1a1a1a",
                accent: None,
                glyph: None,
                glyph_color: "#aa44aa",
            },
            Tile::Altar(kind) => TilePalette {
                fill: altar_fill(kind),
                accent: Some(kind.color()),
                glyph: Some(kind.icon()),
                glyph_color: kind.color(),
            },
            Tile::Seal(kind) => TilePalette {
                fill: seal_fill(kind),
                accent: Some(kind.color()),
                glyph: Some(kind.icon()),
                glyph_color: kind.color(),
            },
            Tile::Sign(_) => TilePalette {
                fill: "#8a6b47",
                accent: Some("#d7b07b"),
                glyph: Some("?"),
                glyph_color: "#ffffff",
            },
            Tile::Bridge => TilePalette {
                fill: "#8b4513",
                accent: Some("#a0522d"),
                glyph: None,
                glyph_color: "#ffffff",
            },
        }
    } else {
        match tile {
            Tile::Wall => TilePalette {
                fill: COL_WALL_REVEALED,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CrackedWall => TilePalette {
                fill: "#2d2338",
                accent: Some("#805d48"),
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::BrittleWall => TilePalette {
                fill: "#342c26",
                accent: Some("#7d6a57"),
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Floor => TilePalette {
                fill: COL_FLOOR_REVEALED,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Corridor => TilePalette {
                fill: COL_CORRIDOR_REVEALED,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::StairsDown => TilePalette {
                fill: "#243857",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Forge => TilePalette {
                fill: "#4b2b1d",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Shop => TilePalette {
                fill: "#1e4a33",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Chest => TilePalette {
                fill: "#5a441b",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Crate => TilePalette {
                fill: "#3f2c1c",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Spikes => TilePalette {
                fill: "#4a2d32",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Oil => TilePalette {
                fill: "#3a2f1b",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Water => TilePalette {
                fill: "#213f6b",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::DeepWater => TilePalette {
                fill: "#132846",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Npc(_) => TilePalette {
                fill: "#27465c",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Shrine => TilePalette {
                fill: "#4f3d20",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::StrokeShrine => TilePalette {
                fill: "#111822",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::ToneWall => TilePalette {
                fill: "#1a1010",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CompoundShrine => TilePalette {
                fill: "#112211",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::ClassifierShrine => TilePalette {
                fill: "#221a11",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::InkWell => TilePalette {
                fill: "#111118",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::AncestorShrine => TilePalette {
                fill: "#181111",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::TranslationAltar => TilePalette {
                fill: "#111818",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::RadicalGarden => TilePalette {
                fill: "#111811",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::MirrorPool => TilePalette {
                fill: "#11111f",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::StoneTutor => TilePalette {
                fill: "#181811",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CodexShrine => TilePalette {
                fill: "#16101e",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::WordBridge => TilePalette {
                fill: "#101018",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::LockedDoor => TilePalette {
                fill: "#1a1010",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CursedFloor => TilePalette {
                fill: "#111111",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Altar(kind) => TilePalette {
                fill: altar_revealed_fill(kind),
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Seal(kind) => TilePalette {
                fill: seal_revealed_fill(kind),
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Sign(_) => TilePalette {
                fill: "#4b3a26",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Bridge => TilePalette {
                fill: "#5c4033",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
        }
    }
}

fn tile_plate_fill(tile: Tile) -> Option<&'static str> {
    match tile {
        Tile::StairsDown => Some("rgba(255,255,255,0.14)"),
        Tile::Forge => Some("rgba(255,226,194,0.16)"),
        Tile::Shop => Some("rgba(207,255,224,0.15)"),
        Tile::Chest => Some("rgba(255,231,173,0.16)"),
        Tile::Npc(_) => Some("rgba(225,245,255,0.12)"),
        Tile::Shrine => Some("rgba(255,224,156,0.16)"),
        Tile::StrokeShrine => Some("rgba(136,204,255,0.16)"),
        Tile::ToneWall => Some("rgba(221,102,68,0.16)"),
        Tile::CompoundShrine => Some("rgba(102,221,136,0.16)"),
        Tile::ClassifierShrine => Some("rgba(221,170,68,0.16)"),
        Tile::InkWell => Some("rgba(153,153,238,0.16)"),
        Tile::AncestorShrine => Some("rgba(238,153,102,0.16)"),
        Tile::TranslationAltar => Some("rgba(102,204,204,0.16)"),
        Tile::RadicalGarden => Some("rgba(136,238,102,0.16)"),
        Tile::MirrorPool => Some("rgba(170,170,255,0.16)"),
        Tile::StoneTutor => Some("rgba(204,204,102,0.16)"),
        Tile::CodexShrine => Some("rgba(221,153,255,0.16)"),
        Tile::WordBridge => Some("rgba(102,170,255,0.16)"),
        Tile::LockedDoor => Some("rgba(255,102,68,0.16)"),
        Tile::Altar(kind) => Some(altar_plate_fill(kind)),
        Tile::Seal(kind) => Some(seal_plate_fill(kind)),
        Tile::Sign(_) => Some("rgba(255,236,200,0.10)"),
        _ => None,
    }
}

fn altar_fill(kind: AltarKind) -> &'static str {
    match kind {
        AltarKind::Jade => "#30563f",
        AltarKind::Gale => "#334d74",
        AltarKind::Mirror => "#5a456e",
        AltarKind::Iron => "#4a4a4a",
        AltarKind::Gold => "#665522",
    }
}

fn altar_revealed_fill(kind: AltarKind) -> &'static str {
    match kind {
        AltarKind::Jade => "#214231",
        AltarKind::Gale => "#243b56",
        AltarKind::Mirror => "#443255",
        AltarKind::Iron => "#333333",
        AltarKind::Gold => "#443a1a",
    }
}

fn altar_plate_fill(kind: AltarKind) -> &'static str {
    match kind {
        AltarKind::Jade => "rgba(102,221,153,0.14)",
        AltarKind::Gale => "rgba(136,204,255,0.14)",
        AltarKind::Mirror => "rgba(221,184,255,0.14)",
        AltarKind::Iron => "rgba(200,200,200,0.14)",
        AltarKind::Gold => "rgba(255,215,0,0.14)",
    }
}

fn seal_fill(kind: SealKind) -> &'static str {
    match kind {
        SealKind::Ember => "#6a3529",
        SealKind::Tide => "#264d79",
        SealKind::Thorn => "#5f3144",
        SealKind::Echo => "#4f3a68",
    }
}

fn seal_revealed_fill(kind: SealKind) -> &'static str {
    match kind {
        SealKind::Ember => "#44251d",
        SealKind::Tide => "#1b3652",
        SealKind::Thorn => "#412230",
        SealKind::Echo => "#352646",
    }
}

fn seal_plate_fill(kind: SealKind) -> &'static str {
    match kind {
        SealKind::Ember => "rgba(255,155,115,0.16)",
        SealKind::Tide => "rgba(144,201,255,0.16)",
        SealKind::Thorn => "rgba(255,158,184,0.14)",
        SealKind::Echo => "rgba(212,164,255,0.16)",
    }
}

fn tile_glyph_font(tile: Tile) -> &'static str {
    match tile {
        Tile::Seal(_) => "bold 14px 'Noto Serif SC', 'SimSun', serif",
        Tile::Crate | Tile::Spikes | Tile::Oil | Tile::Water | Tile::DeepWater => "15px monospace",
        _ => "16px monospace",
    }
}

fn tile_glyph_y(tile: Tile, screen_y: f64, anim_t: f64, tx: i32, ty: i32) -> f64 {
    let base = screen_y + TILE_SIZE * 0.75;
    match tile {
        Tile::Water | Tile::DeepWater => {
            base + (anim_t * 3.5 + tx as f64 * 0.6 + ty as f64 * 0.35).sin() * 1.4
        }
        Tile::Oil => base + (anim_t * 2.0 + tx as f64 * 0.4).sin() * 0.6,
        Tile::Shrine => base + (anim_t * 2.5).sin() * 0.9,
        Tile::StrokeShrine => base + (anim_t * 2.7 + tx as f64 * 0.3).sin() * 0.8,
        Tile::ToneWall => base + (anim_t * 3.0 + ty as f64 * 0.3).sin() * 0.7,
        Tile::CompoundShrine => base + (anim_t * 2.4 + tx as f64 * 0.2).sin() * 0.8,
        Tile::ClassifierShrine => base + (anim_t * 2.6 + ty as f64 * 0.25).sin() * 0.7,
        Tile::InkWell => base + (anim_t * 2.3 + tx as f64 * 0.25).sin() * 0.7,
        Tile::AncestorShrine => base + (anim_t * 2.9 + ty as f64 * 0.35).sin() * 0.8,
        Tile::TranslationAltar => base + (anim_t * 2.5 + tx as f64 * 0.3).sin() * 0.75,
        Tile::RadicalGarden => base + (anim_t * 2.2 + tx as f64 * 0.2).sin() * 0.9,
        Tile::MirrorPool => base + (anim_t * 3.2 + ty as f64 * 0.4).sin() * 1.0,
        Tile::StoneTutor => base + (anim_t * 2.0 + tx as f64 * 0.15).sin() * 0.6,
        Tile::CodexShrine => base + (anim_t * 2.4 + tx as f64 * 0.2).sin() * 0.8,
        Tile::WordBridge => base + (anim_t * 2.6 + ty as f64 * 0.3).sin() * 0.7,
        Tile::LockedDoor => base + (anim_t * 1.8 + tx as f64 * 0.1).sin() * 0.5,
        Tile::Altar(_) => base + (anim_t * 2.8 + ty as f64 * 0.4).sin() * 0.8,
        Tile::Seal(_) => base + (anim_t * 3.1 + tx as f64 * 0.35 + ty as f64 * 0.2).sin() * 0.7,
        Tile::StairsDown => base + (anim_t * 1.8).sin() * 0.4,
        _ => base,
    }
}

fn tile_pattern_seed(tx: i32, ty: i32) -> u32 {
    (tx as u32)
        .wrapping_mul(73_856_093)
        .wrapping_add((ty as u32).wrapping_mul(19_349_663))
        ^ 0x9e37_79b9
}

fn radical_stack_counts(radicals: &[&'static str]) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for radical in radicals {
        *counts.entry(*radical).or_insert(0) += 1;
    }
    counts
}

fn word_wrap(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.len() + 1 + word.len() > max_chars {
            lines.push(current);
            current = word.to_string();
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::{radical_stack_counts, tile_palette, TilePalette};
    use crate::dungeon::{AltarKind, Tile};

    #[test]
    fn radical_stack_counts_groups_duplicate_radicals() {
        let counts = radical_stack_counts(&["水", "木", "水"]);

        assert_eq!(counts.get("水"), Some(&2));
        assert_eq!(counts.get("木"), Some(&1));
        assert_eq!(counts.len(), 2);
    }

    #[test]
    fn radical_stack_counts_returns_empty_map_for_empty_inventory() {
        let counts = radical_stack_counts(&[]);

        assert!(counts.is_empty());
    }

    #[test]
    fn tile_palette_highlights_interactive_tiles_when_visible() {
        let stairs = tile_palette(Tile::StairsDown, true);

        assert_eq!(
            stairs,
            TilePalette {
                fill: "#8ab4ff",
                accent: Some("#d7e7ff"),
                glyph: Some("▼"),
                glyph_color: "#ffffff",
            }
        );
    }

    #[test]
    fn tile_palette_keeps_special_tiles_distinct_when_revealed() {
        let revealed_chest = tile_palette(Tile::Chest, false);
        let revealed_floor = tile_palette(Tile::Floor, false);
        let revealed_altar = tile_palette(Tile::Altar(AltarKind::Jade), false);

        assert_eq!(revealed_chest.fill, "#5a441b");
        assert_ne!(revealed_chest.fill, revealed_floor.fill);
        assert_eq!(revealed_altar.fill, "#214231");
    }
}
