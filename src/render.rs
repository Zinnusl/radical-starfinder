//! Canvas 2D rendering for the station.

use std::collections::BTreeMap;

use js_sys::Date;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::combat::{
    ArenaBiome, BattleTile, Direction, EnemyIntent, TacticalBattle, TacticalPhase, TargetMode,
    TypingAction, Weather, WuxingElement,
};
use crate::world::{TerminalKind, DungeonLevel, SealKind, Tile, LocationType};
use crate::enemy::{BossKind, Enemy};
use crate::game::{combo_tier, CombatState, ComboTier, GameSettings, ListenMode, ShopItemKind};
use crate::particle::ParticleSystem;
use crate::player::{Faction, Item, ItemKind, ItemState, Player, PlayerForm, Ship};
use crate::world::starmap::SectorMap;
use crate::world::ship::{ShipLayout, ShipTile};
use crate::world::events::SpaceEvent;
use crate::radical;
use crate::sprites::SpriteCache;

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

            // Hint
            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx.set_text_align("center");
            let has_reroll =
                companion == Some(crate::game::Companion::Quartermaster) && companion_level >= 3;
            let hint_text = if has_reroll {
                "↑↓=browse  Enter=buy  R=reroll  Esc=leave"
            } else {
                "↑↓=browse  Enter=buy  Esc=leave"
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
            if matches!(tile, Tile::Bulkhead | Tile::DamagedBulkhead | Tile::WeakBulkhead) {
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
            Tile::NavBeacon | Tile::SpecialRoom(_) | Tile::SalvageCrate
            | Tile::MetalFloor | Tile::Hallway | Tile::CorruptedFloor | Tile::Catwalk
            | Tile::Trap(_) => {},
            | Tile::FrozenDeck | Tile::ToxicGas | Tile::ToxicFungus => {
                self.ctx.set_fill_style_str(if tile == Tile::Hallway {
                    "rgba(215,225,255,0.06)"
                } else {
                    "rgba(255,255,255,0.05)"
                });
                let spark_x = screen_x + 4.0 + (pattern % 11) as f64;
                let spark_y = screen_y + 4.0 + ((pattern / 11) % 9) as f64;
                self.ctx.fill_rect(spark_x, spark_y, 2.0, 2.0);
                if tile == Tile::Hallway {
                    self.ctx.set_fill_style_str("rgba(170,190,255,0.05)");
                    self.ctx.fill_rect(
                        screen_x + 4.0,
                        screen_y + TILE_SIZE / 2.0 - 0.5,
                        TILE_SIZE - 8.0,
                        1.0,
                    );
                }
                if tile == Tile::CorruptedFloor {
                    // Subtle cursed shimmer — barely visible trap hint
                    self.ctx.set_fill_style_str("rgba(180,120,255,0.06)");
                    let cx = screen_x + 6.0 + ((pattern / 3) % 8) as f64;
                    let cy = screen_y + 6.0 + ((pattern / 7) % 8) as f64;
                    self.ctx.fill_rect(cx, cy, 2.0, 2.0);
                }
            }
            Tile::Bulkhead | Tile::DamagedBulkhead | Tile::WeakBulkhead | Tile::CargoPipes => {
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
                if tile == Tile::DamagedBulkhead {
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
                } else if tile == Tile::WeakBulkhead {
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
            Tile::CoolantPool | Tile::VacuumBreach | Tile::PlasmaVent => {
                let wave_shift = (anim_t * 3.2 + tx as f64 * 0.7 + ty as f64 * 0.4).sin() * 2.0;
                self.ctx.set_fill_style_str(if tile == Tile::VacuumBreach {
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
                if tile == Tile::VacuumBreach {
                    self.ctx.set_fill_style_str("rgba(26,48,89,0.28)");
                    self.ctx
                        .fill_rect(screen_x + 4.0, screen_y + 18.0, TILE_SIZE - 8.0, 3.0);
                }
            }
            Tile::Coolant => {
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
            Tile::LaserGrid => {
                self.ctx.set_fill_style_str("rgba(255,220,220,0.08)");
                self.ctx.fill_rect(
                    screen_x + 4.0,
                    screen_y + TILE_SIZE - 7.0,
                    TILE_SIZE - 8.0,
                    2.0,
                );
            }
            Tile::Airlock
            | Tile::QuantumForge
            | Tile::TradeTerminal
            | Tile::SupplyCrate
            | Tile::Npc(_)
            | Tile::CircuitShrine
            | Tile::RadicalLab
            | Tile::FrequencyWall
            | Tile::CompoundShrine
            | Tile::ClassifierNode
            | Tile::DataWell
            | Tile::MemorialNode
            | Tile::TranslationTerminal
            | Tile::HoloPool
            | Tile::DroidTutor
            | Tile::CodexTerminal
            | Tile::DataBridge
            | Tile::SealedHatch
            | Tile::Terminal(_)
            | Tile::SecurityLock(_)
            | Tile::InfoPanel(_)
            | Tile::OreVein
            | Tile::DataRack
            | Tile::WarpGatePortal
            | Tile::MedBayTile
            | Tile::CreditCache
            | Tile::CrystalPanel
            | Tile::PressureSensor
            | Tile::CargoCrate => {
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
        altar_kind: crate::world::TerminalKind,
        cursor: usize,
    ) {
        let box_w = 360.0;
        let items_len = player.items.len().max(1);
        let box_h = 100.0 + items_len as f64 * 28.0;
        let box_x = (self.canvas_w - box_w) / 2.0;
        let box_y = 60.0;

        self.ctx.set_fill_style_str("rgba(10,8,20,0.96)");
        self.ctx.fill_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("rgba(255,170,68,0.5)");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("rgba(255,170,68,0.15)");
        self.ctx.set_line_width(1.0);
        self.ctx
            .stroke_rect(box_x + 1.0, box_y + 1.0, box_w - 2.0, box_h - 2.0);

        let god_name = match altar_kind {
            crate::world::TerminalKind::Quantum => "Consortium Executive",
            crate::world::TerminalKind::Stellar => "Free Trader Captain",
            crate::world::TerminalKind::Holographic => "Technocracy AI",
            crate::world::TerminalKind::Tactical => "Alliance Commander",
            crate::world::TerminalKind::Commerce => "Ancient Order Master",
        };

        // Find current piety
        let piety = player
            .piety
            .iter()
            .find(|(d, _)| match (d, altar_kind) {
                (crate::player::Faction::Consortium, crate::world::TerminalKind::Quantum) => true,
                (crate::player::Faction::FreeTraders, crate::world::TerminalKind::Stellar) => true,
                (crate::player::Faction::Technocracy, crate::world::TerminalKind::Holographic) => true,
                (crate::player::Faction::MilitaryAlliance, crate::world::TerminalKind::Tactical) => true,
                (crate::player::Faction::AncientOrder, crate::world::TerminalKind::Commerce) => true,
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

        self.ctx.set_fill_style_str("rgba(10,8,20,0.96)");
        self.ctx.fill_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("rgba(100,140,220,0.5)");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("rgba(100,140,220,0.15)");
        self.ctx.set_line_width(1.0);
        self.ctx
            .stroke_rect(box_x + 1.0, box_y + 1.0, box_w - 2.0, box_h - 2.0);

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

        self.ctx.set_fill_style_str("rgba(10,8,20,0.96)");
        self.ctx.fill_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("rgba(100,140,220,0.5)");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("rgba(100,140,220,0.15)");
        self.ctx.set_line_width(1.0);
        self.ctx
            .stroke_rect(box_x + 1.0, box_y + 1.0, box_w - 2.0, box_h - 2.0);

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
                0 => equipment_name(player.weapon, player.enchantments[0], player.weapon_state),
                1 => equipment_name(player.armor, player.enchantments[1], player.armor_state),
                _ => equipment_name(player.charm, player.enchantments[2], player.charm_state),
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
            CombatState::TacticalBattle(_) => {
                lines.push("Tactical: M move  A attack  S spell  D defend  W wait".to_string());
                lines.push("Arrow keys navigate  Enter confirm  Esc cancel/flee".to_string());
                "Tactical Combat Controls"
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
                lines.push("D daily challenge".to_string());
                "Menu Controls"
            }
            CombatState::GameOver => {
                lines.push("Game over: R restart  I inventory".to_string());
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
                lines.push("[P] Purify cursed equipment  Esc cancel".to_string());
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

        self.ctx.set_fill_style_str("rgba(10,8,20,0.92)");
        self.ctx.fill_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("rgba(100,80,140,0.4)");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("rgba(120,90,180,0.15)");
        self.ctx.set_line_width(1.0);
        self.ctx
            .stroke_rect(box_x + 1.0, box_y + 1.0, box_w - 2.0, box_h - 2.0);

        self.ctx.set_text_align("left");
        self.ctx.set_fill_style_str("#00ccdd");
        self.ctx.set_font("bold 14px monospace");
        self.ctx
            .fill_text(mode_title, box_x + 12.0, box_y + 20.0)
            .ok();
        self.ctx.set_fill_style_str("#7784aa");
        self.ctx.set_font("10px monospace");
        self.ctx.set_text_align("right");
        self.ctx
            .fill_text("? to close", box_x + box_w - 12.0, box_y + 20.0)
            .ok();

        self.ctx.set_stroke_style_str("rgba(100,80,140,0.3)");
        self.ctx.set_line_width(1.0);
        self.ctx.begin_path();
        self.ctx.move_to(box_x + 10.0, box_y + 28.0);
        self.ctx.line_to(box_x + box_w - 10.0, box_y + 28.0);
        self.ctx.stroke();

        self.ctx.set_text_align("left");
        self.ctx.set_font("11px monospace");
        for (idx, line) in lines.iter().enumerate() {
            let line_y = box_y + 42.0 + idx as f64 * 16.0;
            if idx < 3 {
                self.ctx.set_fill_style_str("#9aaad8");
            } else {
                self.ctx.set_fill_style_str("#dbe7ff");
            }
            self.ctx.fill_text(line, box_x + 12.0, line_y).ok();
        }
    }

    fn draw_tactical_battle(&self, battle: &TacticalBattle, anim_t: f64, _player: &Player) {
        let grid_size = battle.arena.width as f64;
        let cell = 36.0_f64;
        let grid_px = grid_size * cell;
        let grid_x = (self.canvas_w - grid_px) / 2.0;
        let grid_y = 30.0;

        // Full-screen dark backdrop
        self.ctx.set_fill_style_str("rgba(10,6,18,0.94)");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

        // ── Turn order queue strip (top of screen) ──────────────────────
        {
            let tq_cell = 28.0_f64;
            let tq_gap = 3.0;
            let tq_count = battle.turn_queue.len();
            let tq_total_w = tq_count as f64 * (tq_cell + tq_gap) - tq_gap;
            let tq_x0 = (self.canvas_w - tq_total_w) / 2.0;
            let tq_y = 1.0;
            let tq_hp_h = 3.0; // HP pip height below each portrait

            // Subtle background strip
            self.ctx.set_fill_style_str("rgba(0,0,0,0.4)");
            self.ctx.fill_rect(
                tq_x0 - 4.0,
                tq_y - 1.0,
                tq_total_w + 8.0,
                tq_cell + tq_hp_h + 4.0,
            );

            for (qi, &uid) in battle.turn_queue.iter().enumerate() {
                let sx = tq_x0 + qi as f64 * (tq_cell + tq_gap);
                let unit = &battle.units[uid];
                let is_current = qi == battle.turn_queue_pos;

                // Background with rounded corners via path
                let r = 4.0;
                self.ctx.begin_path();
                self.ctx.move_to(sx + r, tq_y);
                self.ctx.line_to(sx + tq_cell - r, tq_y);
                self.ctx
                    .arc(
                        sx + tq_cell - r,
                        tq_y + r,
                        r,
                        -std::f64::consts::FRAC_PI_2,
                        0.0,
                    )
                    .ok();
                self.ctx.line_to(sx + tq_cell, tq_y + tq_cell - r);
                self.ctx
                    .arc(
                        sx + tq_cell - r,
                        tq_y + tq_cell - r,
                        r,
                        0.0,
                        std::f64::consts::FRAC_PI_2,
                    )
                    .ok();
                self.ctx.line_to(sx + r, tq_y + tq_cell);
                self.ctx
                    .arc(
                        sx + r,
                        tq_y + tq_cell - r,
                        r,
                        std::f64::consts::FRAC_PI_2,
                        std::f64::consts::PI,
                    )
                    .ok();
                self.ctx.line_to(sx, tq_y + r);
                self.ctx
                    .arc(
                        sx + r,
                        tq_y + r,
                        r,
                        std::f64::consts::PI,
                        std::f64::consts::PI * 1.5,
                    )
                    .ok();
                self.ctx.close_path();

                let bg = if !unit.alive {
                    "rgba(40,40,40,0.6)"
                } else if unit.is_player() {
                    if is_current {
                        "rgba(100,75,20,0.85)"
                    } else {
                        "rgba(60,45,15,0.75)"
                    }
                } else if unit.is_companion() {
                    if is_current {
                        "rgba(30,90,60,0.85)"
                    } else {
                        "rgba(25,70,50,0.75)"
                    }
                } else if is_current {
                    "rgba(100,25,25,0.85)"
                } else {
                    "rgba(60,20,20,0.7)"
                };
                self.ctx.set_fill_style_str(bg);
                self.ctx.fill();

                // Current turn glow
                if is_current && unit.alive {
                    let glow_pulse = ((anim_t * 4.0).sin() * 0.3 + 0.7).clamp(0.4, 1.0);
                    let glow_color = if unit.is_player() {
                        format!("rgba(255,204,50,{})", glow_pulse)
                    } else if unit.is_companion() {
                        format!("rgba(68,204,136,{})", glow_pulse)
                    } else {
                        format!("rgba(255,80,80,{})", glow_pulse)
                    };
                    self.ctx.set_stroke_style_str(&glow_color);
                    self.ctx.set_line_width(2.5);
                    self.ctx.stroke();
                    // Outer glow via shadow
                    self.ctx.set_shadow_color(&glow_color);
                    self.ctx.set_shadow_blur(6.0);
                    self.ctx.stroke();
                    self.ctx.set_shadow_blur(0.0);
                    self.ctx.set_shadow_color("transparent");
                } else {
                    self.ctx.set_stroke_style_str("rgba(255,255,255,0.12)");
                    self.ctx.set_line_width(0.5);
                    self.ctx.stroke();
                }

                // Unit glyph
                let glyph = if unit.is_player() {
                    "你"
                } else if unit.is_companion() {
                    if unit.hanzi.is_empty() { "友" } else { unit.hanzi }
                } else if !unit.hanzi.is_empty() {
                    unit.hanzi
                } else {
                    "敌"
                };
                let fg = if !unit.alive {
                    "#555"
                } else if unit.is_player() {
                    "#00ccdd"
                } else if unit.is_companion() {
                    "#44cc88"
                } else {
                    "#ff6666"
                };
                self.ctx.set_fill_style_str(fg);
                self.ctx.set_font("15px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(glyph, sx + tq_cell / 2.0, tq_y + tq_cell / 2.0 + 5.0)
                    .ok();

                // HP pip under portrait
                if unit.alive {
                    let hp_frac = if unit.max_hp > 0 {
                        (unit.hp as f64 / unit.max_hp as f64).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let pip_y = tq_y + tq_cell + 1.0;
                    self.ctx.set_fill_style_str("rgba(0,0,0,0.5)");
                    self.ctx.fill_rect(sx + 2.0, pip_y, tq_cell - 4.0, tq_hp_h);
                    let hp_color = if unit.is_player() || unit.is_companion() {
                        "#44cc55"
                    } else if hp_frac > 0.5 {
                        "#cc4444"
                    } else {
                        "#ff6644"
                    };
                    self.ctx.set_fill_style_str(hp_color);
                    self.ctx
                        .fill_rect(sx + 2.0, pip_y, (tq_cell - 4.0) * hp_frac, tq_hp_h);
                } else {
                    // Dead unit: X overlay
                    self.ctx.set_fill_style_str("rgba(255,50,50,0.4)");
                    self.ctx.set_font("bold 12px monospace");
                    self.ctx
                        .fill_text("✕", sx + tq_cell / 2.0, tq_y + tq_cell / 2.0 + 4.0)
                        .ok();
                }

                // Turn order number (small, top-left corner)
                self.ctx.set_fill_style_str("rgba(255,255,255,0.3)");
                self.ctx.set_font("bold 7px monospace");
                self.ctx.set_text_align("left");
                self.ctx
                    .fill_text(&format!("{}", qi + 1), sx + 2.0, tq_y + 8.0)
                    .ok();
            }
            self.ctx.set_text_align("left");
        }

        // Grid tiles — sprite-based with flat color fallback
        self.ctx.set_image_smoothing_enabled(false);
        let biome = &battle.arena.biome;
        for gy in 0..battle.arena.height {
            for gx in 0..battle.arena.width {
                let tile = battle
                    .arena
                    .tile(gx as i32, gy as i32)
                    .unwrap_or(BattleTile::MetalFloor);
                let sx = grid_x + gx as f64 * cell;
                let sy = grid_y + gy as f64 * cell;

                let sprite_key = match tile {
                    BattleTile::MetalFloor => match biome {
                        ArenaBiome::StationInterior => "arena_floor_stone",
                        ArenaBiome::DerelictShip => "arena_floor_dark",
                        ArenaBiome::AlienRuins => "arena_floor_arcane",
                        ArenaBiome::IrradiatedZone => "arena_floor_cursed",
                        ArenaBiome::Hydroponics => "arena_floor_garden",
                        ArenaBiome::CryoBay => "arena_floor_frozen",
                        ArenaBiome::ReactorRoom => "arena_floor_infernal",
                    },
                    BattleTile::CoverBarrier => match biome {
                        ArenaBiome::StationInterior => "arena_obstacle_stone",
                        ArenaBiome::DerelictShip => "arena_obstacle_dark",
                        ArenaBiome::AlienRuins => "arena_obstacle_arcane",
                        ArenaBiome::IrradiatedZone => "arena_obstacle_cursed",
                        ArenaBiome::Hydroponics => "arena_obstacle_garden",
                        ArenaBiome::CryoBay => "arena_obstacle_frozen",
                        ArenaBiome::ReactorRoom => "arena_obstacle_infernal",
                    },
                    BattleTile::WiringPanel => "arena_grass",
                    BattleTile::CoolantPool => "arena_water",
                    BattleTile::FrozenCoolant => "arena_ice",
                    BattleTile::BlastMark => "arena_scorched",
                    BattleTile::OilSlick => "arena_ink_pool",
                    BattleTile::DamagedPlating => "arena_broken_ground",
                    BattleTile::VentSteam => "arena_steam",
                    BattleTile::PlasmaPool => "arena_lava",
                    BattleTile::ElectrifiedWire => "arena_thorns",
                    BattleTile::HoloTrap => "arena_arcane_glyph",
                    BattleTile::Debris => "arena_sand",
                    BattleTile::PipeTangle => "arena_bamboo_thicket",
                    BattleTile::CryoZone => "arena_frozen_ground",
                    BattleTile::EnergyNode => "arena_spirit_well",
                    BattleTile::PowerDrain => "arena_spirit_drain",
                    BattleTile::ChargingPad => "arena_meditation_stone",
                    BattleTile::GravityTrap => "arena_soul_trap",
                    BattleTile::CargoCrate => "arena_obstacle_stone",
                    BattleTile::ConveyorN
                    | BattleTile::ConveyorS
                    | BattleTile::ConveyorE
                    | BattleTile::ConveyorW => "arena_water",
                    BattleTile::FuelCanister => "arena_obstacle_stone",
                    BattleTile::WeakenedPlating => "arena_broken_ground",
                    BattleTile::DamagedFloor => "arena_broken_ground",
                    BattleTile::BreachedFloor => "arena_obstacle_dark",
                    BattleTile::MineTile => match biome {
                        ArenaBiome::StationInterior => "arena_floor_stone",
                        ArenaBiome::DerelictShip => "arena_floor_dark",
                        ArenaBiome::AlienRuins => "arena_floor_arcane",
                        ArenaBiome::IrradiatedZone => "arena_floor_cursed",
                        ArenaBiome::Hydroponics => "arena_floor_garden",
                        ArenaBiome::CryoBay => "arena_floor_frozen",
                        ArenaBiome::ReactorRoom => "arena_floor_infernal",
                    },
                    BattleTile::MineTileRevealed => "arena_thorns",
                    BattleTile::Lubricant => "arena_water",
                    BattleTile::ShieldZone => "arena_spirit_well",
                    BattleTile::ElevatedPlatform => "arena_broken_ground",
                };

                if !self.draw_sprite_icon(sprite_key, sx, sy, cell) {
                    let fill = match tile {
                        BattleTile::MetalFloor => "#3a3458",
                        BattleTile::CoverBarrier => "#1a1428",
                        BattleTile::WiringPanel => "#2a4a2a",
                        BattleTile::CoolantPool => "#1a2a4a",
                        BattleTile::FrozenCoolant => "#3a4a6a",
                        BattleTile::BlastMark => "#4a2a1a",
                        BattleTile::OilSlick => "#2a2a4a",
                        BattleTile::DamagedPlating => "#3a3030",
                        BattleTile::VentSteam => "#5a5a6a",
                        BattleTile::PlasmaPool => "#6a2a0a",
                        BattleTile::ElectrifiedWire => "#2a3a1a",
                        BattleTile::HoloTrap => "#2a2a5a",
                        BattleTile::Debris => "#5a4a2a",
                        BattleTile::PipeTangle => "#1a3a1a",
                        BattleTile::CryoZone => "#4a5a6a",
                        BattleTile::EnergyNode => "#2244aa",
                        BattleTile::PowerDrain => "#1a0a2a",
                        BattleTile::ChargingPad => "#4a4a5a",
                        BattleTile::GravityTrap => "#3a1a3a",
                        BattleTile::CargoCrate => "#5a4a3a",
                        BattleTile::ConveyorN
                        | BattleTile::ConveyorS
                        | BattleTile::ConveyorE
                        | BattleTile::ConveyorW => "#1a3a5a",
                        BattleTile::FuelCanister => "#6a3a1a",
                        BattleTile::WeakenedPlating => "#3a3430",
                        BattleTile::DamagedFloor => "#4a3a28",
                        BattleTile::BreachedFloor => "#0a0a0a",
                        BattleTile::MineTile => "#3a3458",
                        BattleTile::MineTileRevealed => "#4a2a2a",
                        BattleTile::Lubricant => "#2a2018",
                        BattleTile::ShieldZone => "#4a4a22",
                        BattleTile::ElevatedPlatform => "#5a4a30",
                    };
                    self.ctx.set_fill_style_str(fill);
                    self.ctx.fill_rect(sx, sy, cell, cell);

                    if tile == BattleTile::CoverBarrier {
                        self.ctx.set_fill_style_str("#2a2038");
                        self.ctx
                            .fill_rect(sx + 4.0, sy + 4.0, cell - 8.0, cell - 8.0);
                    }

                    if tile == BattleTile::CargoCrate {
                        self.ctx.set_fill_style_str("#8a7a6a");
                        self.ctx.set_font("bold 14px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("●", sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                            .ok();
                    }

                    if tile == BattleTile::FuelCanister {
                        self.ctx.set_fill_style_str("#ff6633");
                        self.ctx.set_font("bold 14px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("☢", sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                            .ok();
                    }

                    if tile == BattleTile::DamagedFloor {
                        self.ctx.set_fill_style_str("rgba(200,180,120,0.7)");
                        self.ctx.set_font("bold 12px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("⚠", sx + cell / 2.0, sy + cell / 2.0 + 4.0)
                            .ok();
                    }

                    if tile == BattleTile::BreachedFloor {
                        self.ctx.set_fill_style_str("#2a2a2a");
                        self.ctx
                            .fill_rect(sx + 4.0, sy + 4.0, cell - 8.0, cell - 8.0);
                    }

                    if tile == BattleTile::MineTileRevealed {
                        self.ctx.set_fill_style_str("#cc3333");
                        self.ctx.set_font("bold 14px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("▲", sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                            .ok();
                    }

                    if tile == BattleTile::Lubricant {
                        let pulse = ((anim_t * 2.0).sin() * 0.15 + 0.5).max(0.3);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(80,60,20,{})", pulse));
                        self.ctx.set_font("bold 12px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("~", sx + cell / 2.0, sy + cell / 2.0 + 4.0)
                            .ok();
                    }

                    if tile == BattleTile::ShieldZone {
                        let pulse = ((anim_t * 2.5).sin() * 0.2 + 0.7).max(0.4);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(255,215,80,{})", pulse));
                        self.ctx.set_font("bold 14px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("✦", sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                            .ok();
                    }

                    if tile == BattleTile::ElevatedPlatform {
                        self.ctx.set_fill_style_str("rgba(200,180,130,0.7)");
                        self.ctx.set_font("bold 14px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("▲", sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                            .ok();
                    }

                    let flow_arrow = match tile {
                        BattleTile::ConveyorN => Some("↑"),
                        BattleTile::ConveyorS => Some("↓"),
                        BattleTile::ConveyorE => Some("→"),
                        BattleTile::ConveyorW => Some("←"),
                        _ => None,
                    };
                    if let Some(arrow) = flow_arrow {
                        let pulse = ((anim_t * 3.0).sin() * 0.15 + 0.6).max(0.3);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(100,180,255,{})", pulse));
                        self.ctx.set_font("bold 16px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text(arrow, sx + cell / 2.0, sy + cell / 2.0 + 6.0)
                            .ok();
                    }
                }

                // ── Terrain visual effects (animated overlays) ──
                match tile {
                    BattleTile::CoolantPool
                    | BattleTile::ConveyorN
                    | BattleTile::ConveyorS
                    | BattleTile::ConveyorE
                    | BattleTile::ConveyorW => {
                        let wave = ((anim_t * 2.5 + gx as f64 * 0.7 + gy as f64 * 0.5).sin()
                            * 0.12
                            + 0.08)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(80,160,255,{:.3})", wave));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        let wave2 = ((anim_t * 1.8 + gx as f64 * 1.1 - gy as f64 * 0.9).sin()
                            * 0.06
                            + 0.04)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(120,200,255,{:.3})", wave2));
                        self.ctx.fill_rect(sx, sy + cell * 0.5, cell, cell * 0.5);
                    }
                    BattleTile::PlasmaPool => {
                        let glow = ((anim_t * 3.0 + gx as f64 * 0.5 + gy as f64 * 0.3).sin()
                            * 0.15
                            + 0.15)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(255,120,20,{:.3})", glow));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        let flicker =
                            ((anim_t * 7.0 + gx as f64 * 2.3).sin() * 0.08 + 0.05).max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(255,200,50,{:.3})", flicker));
                        self.ctx
                            .fill_rect(sx + 4.0, sy + 4.0, cell - 8.0, cell - 8.0);
                    }
                    BattleTile::FrozenCoolant | BattleTile::CryoZone => {
                        let seed = (gx.wrapping_mul(31).wrapping_add(gy.wrapping_mul(17))) as f64;
                        let sparkle = ((anim_t * 4.0 + seed).sin() * 0.5 + 0.5).max(0.0);
                        if sparkle > 0.7 {
                            let dot_x = sx + (seed * 7.3) % cell;
                            let dot_y = sy + (seed * 13.7) % cell;
                            self.ctx.set_fill_style_str(&format!(
                                "rgba(200,230,255,{:.3})",
                                sparkle * 0.6
                            ));
                            self.ctx.fill_rect(dot_x, dot_y, 2.0, 2.0);
                        }
                        let sparkle2 = ((anim_t * 3.5 + seed * 1.7).sin() * 0.5 + 0.5).max(0.0);
                        if sparkle2 > 0.65 {
                            let dot_x = sx + (seed * 3.1 + 5.0) % cell;
                            let dot_y = sy + (seed * 11.3 + 8.0) % cell;
                            self.ctx.set_fill_style_str(&format!(
                                "rgba(220,240,255,{:.3})",
                                sparkle2 * 0.5
                            ));
                            self.ctx.fill_rect(dot_x, dot_y, 1.5, 1.5);
                        }
                    }
                    BattleTile::WiringPanel | BattleTile::PipeTangle => {
                        let hash = (gx.wrapping_mul(7).wrapping_add(gy.wrapping_mul(13)) % 4) as f64;
                        let shade_alpha = 0.06 + hash * 0.03;
                        let (r, g, b) = if hash > 2.0 {
                            (60, 120, 40)
                        } else {
                            (30, 80, 20)
                        };
                        self.ctx.set_fill_style_str(&format!(
                            "rgba({},{},{},{:.3})",
                            r, g, b, shade_alpha
                        ));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        let sway = ((anim_t * 1.5 + gx as f64 * 0.4).sin() * 0.04 + 0.02)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(80,160,60,{:.3})", sway));
                        self.ctx.fill_rect(sx, sy, cell * 0.5, cell);
                    }
                    BattleTile::OilSlick => {
                        let swirl = ((anim_t * 2.0
                            + gx as f64 * 1.3
                            + gy as f64 * 0.7)
                            .sin()
                            * 0.1
                            + 0.08)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(60,40,120,{:.3})", swirl));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        let swirl2 = ((anim_t * 1.5 - gx as f64 * 0.9 + gy as f64 * 1.1).cos()
                            * 0.06
                            + 0.04)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(30,20,80,{:.3})", swirl2));
                        self.ctx
                            .fill_rect(sx + 2.0, sy + 2.0, cell - 4.0, cell - 4.0);
                    }
                    BattleTile::VentSteam => {
                        let fade = ((anim_t * 2.0 + gx as f64 * 0.6 + gy as f64 * 0.8).sin()
                            * 0.12
                            + 0.15)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(180,180,200,{:.3})", fade));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        let rise =
                            ((anim_t * 3.0 + gx as f64).sin() * 0.06 + 0.04).max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(220,220,240,{:.3})", rise));
                        self.ctx
                            .fill_rect(sx + cell * 0.25, sy, cell * 0.5, cell * 0.6);
                    }
                    BattleTile::Lubricant => {
                        let sheen = ((anim_t * 2.0 + gx as f64 * 0.8 + gy as f64 * 0.6).sin()
                            * 0.1
                            + 0.08)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(140,120,40,{:.3})", sheen));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                    }
                    BattleTile::ShieldZone => {
                        let glow = ((anim_t * 2.5 + gx as f64 * 0.3 + gy as f64 * 0.5).sin()
                            * 0.1
                            + 0.12)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(220,200,100,{:.3})", glow));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                    }
                    BattleTile::ElevatedPlatform => {
                        let glow = ((anim_t * 1.5 + gx as f64 * 0.4 + gy as f64 * 0.6).sin()
                            * 0.08
                            + 0.1)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(200,180,130,{:.3})", glow));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                    }
                    _ => {}
                }

                // ── Interactive tile pulsing glow ──
                match tile {
                    BattleTile::EnergyNode => {
                        let glow = ((anim_t * 3.0 + gx as f64 * 0.4).sin() * 0.12 + 0.15)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(60,120,255,{:.3})", glow));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        self.ctx.set_stroke_style_str(&format!(
                            "rgba(80,160,255,{:.3})",
                            glow + 0.1
                        ));
                        self.ctx.set_line_width(1.0);
                        self.ctx
                            .stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
                    }
                    BattleTile::ChargingPad => {
                        let glow = ((anim_t * 2.5 + gy as f64 * 0.3).sin() * 0.1 + 0.12)
                            .max(0.0);
                        self.ctx.set_fill_style_str(&format!(
                            "rgba(160,140,200,{:.3})",
                            glow
                        ));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        self.ctx.set_stroke_style_str(&format!(
                            "rgba(180,160,220,{:.3})",
                            glow + 0.08
                        ));
                        self.ctx.set_line_width(1.0);
                        self.ctx
                            .stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
                    }
                    BattleTile::FuelCanister => {
                        let glow = ((anim_t * 4.0).sin() * 0.12 + 0.1).max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(255,100,30,{:.3})", glow));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        self.ctx.set_stroke_style_str(&format!(
                            "rgba(255,140,50,{:.3})",
                            glow + 0.1
                        ));
                        self.ctx.set_line_width(1.0);
                        self.ctx
                            .stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
                    }
                    BattleTile::ShieldZone => {
                        let glow = ((anim_t * 2.5 + gx as f64 * 0.3).sin() * 0.1 + 0.12)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(255,220,100,{:.3})", glow));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        self.ctx.set_stroke_style_str(&format!(
                            "rgba(255,230,120,{:.3})",
                            glow + 0.1
                        ));
                        self.ctx.set_line_width(1.0);
                        self.ctx
                            .stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
                    }
                    BattleTile::ElevatedPlatform => {
                        let glow = ((anim_t * 2.0 + gx as f64 * 0.5).sin() * 0.08 + 0.1)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(180,160,110,{:.3})", glow));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        self.ctx.set_stroke_style_str(&format!(
                            "rgba(200,180,130,{:.3})",
                            glow + 0.08
                        ));
                        self.ctx.set_line_width(1.0);
                        self.ctx
                            .stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
                    }
                    _ => {}
                }

                self.ctx.set_stroke_style_str("rgba(255,255,255,0.06)");
                self.ctx.set_line_width(0.5);
                self.ctx.stroke_rect(sx, sy, cell, cell);
            }
        }
        self.ctx.set_image_smoothing_enabled(true);

        // Trap proximity hint: show "?" on tiles adjacent to hidden traps
        for gy in 0..battle.arena.height {
            for gx in 0..battle.arena.width {
                let x = gx as i32;
                let y = gy as i32;
                if battle.arena.tile(x, y) != Some(BattleTile::MineTile) {
                    continue;
                }
                for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if battle.unit_at(nx, ny).is_some() {
                        let sx = grid_x + gx as f64 * cell;
                        let sy = grid_y + gy as f64 * cell;
                        self.ctx.set_fill_style_str("rgba(255,200,50,0.5)");
                        self.ctx.set_font("bold 12px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("?", sx + cell / 2.0, sy + cell / 2.0 + 4.0)
                            .ok();
                        break;
                    }
                }
            }
        }

        // Grid border
        self.ctx.set_stroke_style_str("#665588");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(grid_x, grid_y, grid_px, grid_px);

        // Ward tile overlays (Pirate Captain boss — shield generator glyphs)
        for &(wx, wy) in &battle.ward_tiles {
            let sx = grid_x + wx as f64 * cell;
            let sy = grid_y + wy as f64 * cell;
            self.ctx.set_fill_style_str("rgba(200,150,50,0.35)");
            self.ctx.fill_rect(sx, sy, cell, cell);
            self.ctx.set_fill_style_str("#cc9933");
            self.ctx.set_font("18px 'Noto Serif SC', 'SimSun', serif");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("門", sx + cell / 2.0, sy + cell / 2.0 + 6.0)
                .ok();
        }

        // Stolen spell pickup overlays (RadicalThief boss)
        for (sx_pos, sy_pos, hanzi, _, _) in &battle.stolen_spells {
            let sx = grid_x + *sx_pos as f64 * cell;
            let sy = grid_y + *sy_pos as f64 * cell;
            let pulse = ((anim_t * 4.0).sin() * 0.15 + 0.4).max(0.2);
            self.ctx
                .set_fill_style_str(&format!("rgba(100,200,255,{})", pulse));
            self.ctx.fill_rect(sx, sy, cell, cell);
            self.ctx.set_fill_style_str("#66ccff");
            self.ctx.set_font("16px 'Noto Serif SC', 'SimSun', serif");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(hanzi, sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                .ok();
        }

        // Targeting overlay
        if let TacticalPhase::Targeting {
            ref valid_targets,
            cursor_x,
            cursor_y,
            ref aoe_preview,
            ref mode,
            ..
        } = battle.phase
        {
            for &(vx, vy) in valid_targets {
                let sx = grid_x + vx as f64 * cell;
                let sy = grid_y + vy as f64 * cell;
                self.ctx.set_fill_style_str("rgba(100,180,255,0.18)");
                self.ctx.fill_rect(sx, sy, cell, cell);
            }

            // Determine AoE color based on spell type
            let (aoe_r, aoe_g, aoe_b) = match mode {
                TargetMode::Spell { spell_idx } => {
                    if *spell_idx < battle.available_spells.len() {
                        match &battle.available_spells[*spell_idx].2 {
                            radical::SpellEffect::FireAoe(_)
                            | radical::SpellEffect::Cone(_) => (255, 80, 30),
                            radical::SpellEffect::Poison(_, _)
                            | radical::SpellEffect::Drain(_) => (80, 200, 60),
                            radical::SpellEffect::Slow(_)
                            | radical::SpellEffect::Stun => (80, 160, 255),
                            radical::SpellEffect::Heal(_)
                            | radical::SpellEffect::FocusRestore(_) => (80, 220, 120),
                            _ => (255, 160, 60),
                        }
                    } else {
                        (255, 100, 50)
                    }
                }
                _ => (255, 100, 50),
            };

            if aoe_preview.len() > 1 {
                let aoe_pulse = ((anim_t * 5.0).sin() * 0.1 + 0.3).max(0.15);
                for &(ax, ay) in aoe_preview {
                    if ax >= 0
                        && ay >= 0
                        && (ax as usize) < battle.arena.width
                        && (ay as usize) < battle.arena.height
                    {
                        let sx = grid_x + ax as f64 * cell;
                        let sy = grid_y + ay as f64 * cell;
                        self.ctx.set_fill_style_str(&format!(
                            "rgba({},{},{},{:.3})",
                            aoe_r, aoe_g, aoe_b, aoe_pulse
                        ));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        let br = (aoe_r as i32 + 40).min(255) as u8;
                        let bg = (aoe_g as i32 + 40).min(255) as u8;
                        let bb = (aoe_b as i32 + 10).min(255) as u8;
                        self.ctx.set_stroke_style_str(&format!(
                            "rgba({},{},{},{:.3})",
                            br,
                            bg,
                            bb,
                            aoe_pulse + 0.2
                        ));
                        self.ctx.set_line_width(1.5);
                        self.ctx
                            .stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
                    }
                }
            }

            let cx = grid_x + cursor_x as f64 * cell;
            let cy = grid_y + cursor_y as f64 * cell;
            let pulse = ((anim_t * 6.0).sin() * 0.15 + 0.55).max(0.3);
            self.ctx
                .set_stroke_style_str(&format!("rgba(255,204,50,{:.3})", pulse));
            self.ctx.set_line_width(2.5);
            self.ctx
                .stroke_rect(cx + 1.0, cy + 1.0, cell - 2.0, cell - 2.0);
            // Crosshair on cursor tile
            let ch_alpha = ((anim_t * 4.0).sin() * 0.15 + 0.45).max(0.2);
            self.ctx.set_stroke_style_str(&format!(
                "rgba({},{},{},{:.3})",
                aoe_r, aoe_g, aoe_b, ch_alpha
            ));
            self.ctx.set_line_width(1.0);
            self.ctx.begin_path();
            self.ctx.move_to(cx + cell * 0.5, cy + 2.0);
            self.ctx.line_to(cx + cell * 0.5, cy + cell - 2.0);
            self.ctx.move_to(cx + 2.0, cy + cell * 0.5);
            self.ctx.line_to(cx + cell - 2.0, cy + cell * 0.5);
            self.ctx.stroke();
        }

        // Look mode overlay
        if let TacticalPhase::Look { cursor_x, cursor_y } = battle.phase {
            let cx = grid_x + cursor_x as f64 * cell;
            let cy = grid_y + cursor_y as f64 * cell;
            let pulse = ((anim_t * 4.0).sin() * 0.12 + 0.45).max(0.25);
            self.ctx
                .set_fill_style_str(&format!("rgba(100,180,255,{})", pulse));
            self.ctx.fill_rect(cx, cy, cell, cell);
            self.ctx
                .set_stroke_style_str(&format!("rgba(100,200,255,{})", pulse + 0.3));
            self.ctx.set_line_width(2.5);
            self.ctx
                .stroke_rect(cx + 1.0, cy + 1.0, cell - 2.0, cell - 2.0);

            self.ctx.set_fill_style_str("#66ccff");
            self.ctx.set_font("bold 9px monospace");
            self.ctx.set_text_align("center");
            self.ctx.fill_text("LOOK", cx + cell / 2.0, cy - 3.0).ok();
            self.ctx.set_text_align("left");
        }

        if let TacticalPhase::Deployment {
            cursor_x,
            cursor_y,
            ref valid_tiles,
        } = battle.phase
        {
            for &(vx, vy) in valid_tiles {
                let sx = grid_x + vx as f64 * cell;
                let sy = grid_y + vy as f64 * cell;
                self.ctx.set_fill_style_str("rgba(80,255,120,0.15)");
                self.ctx.fill_rect(sx, sy, cell, cell);
            }
            let cx = grid_x + cursor_x as f64 * cell;
            let cy = grid_y + cursor_y as f64 * cell;
            let pulse = ((anim_t * 5.0).sin() * 0.15 + 0.6).max(0.3);
            self.ctx
                .set_stroke_style_str(&format!("rgba(80,255,120,{})", pulse));
            self.ctx.set_line_width(2.5);
            self.ctx
                .stroke_rect(cx + 1.0, cy + 1.0, cell - 2.0, cell - 2.0);
            self.ctx.set_fill_style_str("#66ff88");
            self.ctx.set_font("bold 9px monospace");
            self.ctx.set_text_align("center");
            self.ctx.fill_text("DEPLOY", cx + cell / 2.0, cy - 3.0).ok();
            self.ctx.set_text_align("left");
        }

        // Units
        for (i, unit) in battle.units.iter().enumerate() {
            if !unit.alive {
                continue;
            }
            let sx = grid_x + unit.x as f64 * cell;
            let sy = grid_y + unit.y as f64 * cell;

            if unit.is_player() {
                self.ctx.set_fill_style_str("rgba(255,204,50,0.22)");
                self.ctx
                    .fill_rect(sx + 2.0, sy + 2.0, cell - 4.0, cell - 4.0);
                self.ctx.set_fill_style_str(COL_PLAYER);
                self.ctx.set_font("22px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text("你", sx + cell / 2.0, sy + cell / 2.0 + 7.0)
                    .ok();
            } else if unit.is_companion() {
                self.ctx.set_fill_style_str("rgba(68,204,136,0.18)");
                self.ctx
                    .fill_rect(sx + 2.0, sy + 2.0, cell - 4.0, cell - 4.0);
                self.ctx.set_fill_style_str("#44cc88");
                self.ctx.set_font("20px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.set_text_align("center");
                let glyph = if unit.hanzi.is_empty() {
                    "友"
                } else {
                    unit.hanzi
                };
                self.ctx
                    .fill_text(glyph, sx + cell / 2.0, sy + cell / 2.0 + 7.0)
                    .ok();
            } else {
                let is_decoy = unit.is_decoy;
                let bg_color = if is_decoy {
                    "rgba(180,80,255,0.18)"
                } else {
                    "rgba(255,80,80,0.18)"
                };
                self.ctx.set_fill_style_str(bg_color);
                self.ctx
                    .fill_rect(sx + 2.0, sy + 2.0, cell - 4.0, cell - 4.0);
                let fg_color = if is_decoy { "#cc88ff" } else { "#ff6666" };
                self.ctx.set_fill_style_str(fg_color);
                self.ctx.set_font("20px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.set_text_align("center");
                let glyph = if unit.hanzi.is_empty() {
                    "敌"
                } else {
                    unit.hanzi
                };
                self.ctx
                    .fill_text(glyph, sx + cell / 2.0, sy + cell / 2.0 + 7.0)
                    .ok();
                if is_decoy {
                    self.ctx.set_fill_style_str("rgba(200,140,255,0.6)");
                    self.ctx.set_font("8px monospace");
                    self.ctx.set_text_align("right");
                    self.ctx.fill_text("?", sx + cell - 1.0, sy + 10.0).ok();
                }
            }

            // HP bar under unit (thicker, with border)
            let bar_w = cell - 4.0;
            let bar_h = 5.0;
            let bar_x = sx + 2.0;
            let bar_y = sy + cell - 7.0;
            let hp_frac = if unit.max_hp > 0 {
                (unit.hp as f64 / unit.max_hp as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            self.ctx.set_fill_style_str("rgba(0,0,0,0.6)");
            self.ctx
                .fill_rect(bar_x - 0.5, bar_y - 0.5, bar_w + 1.0, bar_h + 1.0);
            self.ctx.set_fill_style_str(COL_HP_BG);
            self.ctx.fill_rect(bar_x, bar_y, bar_w, bar_h);
            let hp_color = if hp_frac > 0.6 {
                "#44cc55"
            } else if hp_frac > 0.3 {
                "#ccaa22"
            } else {
                "#cc3322"
            };
            self.ctx.set_fill_style_str(hp_color);
            self.ctx.fill_rect(bar_x, bar_y, bar_w * hp_frac, bar_h);
            self.ctx.set_stroke_style_str("rgba(255,255,255,0.15)");
            self.ctx.set_line_width(0.5);
            self.ctx.stroke_rect(bar_x, bar_y, bar_w, bar_h);

            if let Some(wg) = unit.word_group {
                const GROUP_COLORS: &[&str] = &[
                    "#ff9944", "#44ddff", "#ff44cc", "#88ff44", "#ffdd44", "#44ffaa", "#dd88ff",
                    "#ff6688",
                ];
                let color = GROUP_COLORS[wg % GROUP_COLORS.len()];
                self.ctx.set_fill_style_str(color);
                self.ctx
                    .fill_rect(sx + 2.0, sy + cell - 2.0, cell - 4.0, 2.0);
            }

            // Defending indicator
            if unit.defending {
                self.ctx.set_fill_style_str("rgba(100,150,255,0.6)");
                self.ctx.set_font("10px monospace");
                self.ctx.set_text_align("right");
                self.ctx.fill_text("🛡", sx + cell - 1.0, sy + 10.0).ok();
            }

            {
                let arrow = match unit.facing {
                    Direction::North => "▲",
                    Direction::South => "▼",
                    Direction::East => "►",
                    Direction::West => "◄",
                };
                let alpha = if unit.is_player() { "0.7" } else { "0.4" };
                self.ctx
                    .set_fill_style_str(&format!("rgba(255,255,255,{})", alpha));
                self.ctx.set_font("8px monospace");
                self.ctx.set_text_align("left");
                self.ctx.fill_text(arrow, sx + 1.0, sy + 10.0).ok();
            }

            // Momentum indicator (directional arrows)
            if unit.momentum > 0 {
                let arrow_ch = match unit.last_move_dir {
                    Some(Direction::North) => "\u{2191}",
                    Some(Direction::South) => "\u{2193}",
                    Some(Direction::East) => "\u{2192}",
                    Some(Direction::West) => "\u{2190}",
                    None => "\u{2192}",
                };
                let color = match unit.momentum {
                    1 => "rgba(255,255,255,0.6)",
                    2 => "rgba(255,255,100,0.7)",
                    _ => "rgba(255,180,50,0.8)",
                };
                self.ctx.set_fill_style_str(color);
                self.ctx.set_font("7px monospace");
                self.ctx.set_text_align("right");
                let text: String = (0..unit.momentum).map(|_| arrow_ch).collect();
                self.ctx
                    .fill_text(&text, sx + cell - 1.0, sy + cell - 9.0)
                    .ok();
            }

            // Cornered indicator
            {
                let ux = unit.x;
                let uy = unit.y;
                let mut walls = 0i32;
                for &(cdx, cdy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    match battle.arena.tile(ux + cdx, uy + cdy) {
                        None => walls += 1,
                        Some(t) if !t.is_walkable() => walls += 1,
                        _ => {}
                    }
                }
                if walls >= 2 {
                    self.ctx.set_fill_style_str("rgba(255,80,80,0.8)");
                    self.ctx.set_font("bold 7px monospace");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text("!", sx + cell / 2.0, sy + cell - 9.0)
                        .ok();
                }
            }

            // Active turn indicator (brighter, with shadow glow)
            if i == battle.current_unit_idx() {
                let glow_color = if unit.is_player() {
                    "#00ccdd"
                } else if unit.is_companion() {
                    "#44cc88"
                } else {
                    "#ffffff"
                };
                self.ctx.set_shadow_color(glow_color);
                self.ctx.set_shadow_blur(4.0);
                self.ctx.set_stroke_style_str(glow_color);
                self.ctx.set_line_width(2.0);
                self.ctx
                    .stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
                self.ctx.set_shadow_blur(0.0);
                self.ctx.set_shadow_color("transparent");
            }

            if let TacticalPhase::EnemyTurn { unit_idx, .. } = battle.phase {
                if i == unit_idx && !unit.is_player() {
                    let pulse = ((anim_t * 8.0).sin() * 0.3 + 0.7).clamp(0.3, 1.0);
                    let color = if unit.is_companion() {
                        format!("rgba(50,200,120,{})", pulse)
                    } else {
                        format!("rgba(255,80,80,{})", pulse)
                    };
                    self.ctx.set_stroke_style_str(&color);
                    self.ctx.set_line_width(2.5);
                    self.ctx
                        .stroke_rect(sx + 0.5, sy + 0.5, cell - 1.0, cell - 1.0);
                }
            }

            // Status effect icons (small colored dots/text above HP bar)
            if !unit.statuses.is_empty() {
                let max_show = 5;
                let icon_size = 7.0;
                let start_x = sx + 2.0;
                let icon_y = bar_y - icon_size - 1.0;
                for (si, status) in unit.statuses.iter().take(max_show).enumerate() {
                    let ix = start_x + si as f64 * (icon_size + 1.0);
                    let icon_pulse =
                        ((anim_t * 3.0 + si as f64 * 1.2).sin() * 0.15 + 0.85).clamp(0.6, 1.0);
                    self.ctx.set_global_alpha(icon_pulse);
                    self.ctx.set_fill_style_str(status.color());
                    self.ctx.begin_path();
                    self.ctx
                        .arc(
                            ix + icon_size / 2.0,
                            icon_y + icon_size / 2.0,
                            icon_size / 2.0,
                            0.0,
                            std::f64::consts::TAU,
                        )
                        .ok();
                    self.ctx.fill();
                    self.ctx.set_global_alpha(1.0);
                    self.ctx.set_fill_style_str("rgba(0,0,0,0.7)");
                    self.ctx.set_font("bold 5px monospace");
                    self.ctx.set_text_align("center");
                    let short = match status.kind {
                        crate::status::StatusKind::Poison { .. } => "P",
                        crate::status::StatusKind::Burn { .. } => "B",
                        crate::status::StatusKind::Regen { .. } => "R",
                        crate::status::StatusKind::Haste => "H",
                        crate::status::StatusKind::Confused => "?",
                        crate::status::StatusKind::Revealed => "E",
                        crate::status::StatusKind::Envenomed => "V",
                        crate::status::StatusKind::Empowered { .. } => "W",
                        crate::status::StatusKind::SpiritShield => "S",
                        crate::status::StatusKind::Freeze => "F",
                        crate::status::StatusKind::Slow => "~",
                        crate::status::StatusKind::Fear => "!",
                        crate::status::StatusKind::Bleed { .. } => "X",
                        crate::status::StatusKind::Thorns => "T",
                        crate::status::StatusKind::Fortify { .. } => "A",
                        crate::status::StatusKind::Invisible => "I",
                        crate::status::StatusKind::Rooted => "R",
                        crate::status::StatusKind::Weakened => "w",
                        crate::status::StatusKind::Cursed => "C",
                        crate::status::StatusKind::Blessed => "★",
                        crate::status::StatusKind::Wet => "D",
                    };
                    self.ctx
                        .fill_text(short, ix + icon_size / 2.0, icon_y + icon_size / 2.0 + 2.0)
                        .ok();
                }
                if unit.statuses.len() > max_show {
                    self.ctx.set_fill_style_str("rgba(255,255,255,0.5)");
                    self.ctx.set_font("bold 6px monospace");
                    let ix = start_x + max_show as f64 * (icon_size + 1.0);
                    self.ctx
                        .fill_text("+", ix + 2.0, icon_y + icon_size / 2.0 + 2.0)
                        .ok();
                }
            }

            if !unit.is_player() {
                if let Some(ref intent) = unit.intent {
                    let icon = match intent {
                        EnemyIntent::Attack => "⚔",
                        EnemyIntent::Approach => "→",
                        EnemyIntent::RadicalAbility { .. } => "✦",
                        EnemyIntent::Retreat => "←",
                        EnemyIntent::Idle => "·",
                        EnemyIntent::Buff => "↑",
                        EnemyIntent::Heal => "+",
                        EnemyIntent::RangedAttack => "◎",
                        EnemyIntent::Surround => "◇",
                    };
                    let intent_color = match intent {
                        EnemyIntent::Attack => "#ff4444",
                        EnemyIntent::Approach => "#ffaa44",
                        EnemyIntent::RadicalAbility { .. } => "#cc44ff",
                        EnemyIntent::Retreat => "#44aaff",
                        EnemyIntent::Idle => "#888888",
                        EnemyIntent::Buff => "#44ff88",
                        EnemyIntent::Heal => "#44ff44",
                        EnemyIntent::RangedAttack => "#ff8844",
                        EnemyIntent::Surround => "#ffff44",
                    };
                    // Intent icon with background bubble
                    self.ctx.set_fill_style_str("rgba(0,0,0,0.5)");
                    self.ctx.begin_path();
                    self.ctx
                        .arc(sx + cell / 2.0, sy - 4.0, 6.0, 0.0, std::f64::consts::TAU)
                        .ok();
                    self.ctx.fill();
                    self.ctx.set_fill_style_str(intent_color);
                    self.ctx.set_font("bold 9px monospace");
                    self.ctx.set_text_align("center");
                    self.ctx.fill_text(icon, sx + cell / 2.0, sy - 1.0).ok();
                }

                if let Some(ref elem) = unit.wuxing_element {
                    let dot_color = match elem {
                        WuxingElement::Water => "#4488ff",
                        WuxingElement::Fire => "#ff4422",
                        WuxingElement::Metal => "#cccccc",
                        WuxingElement::Wood => "#44cc44",
                        WuxingElement::Earth => "#cc9944",
                    };
                    self.ctx.set_fill_style_str(dot_color);
                    self.ctx.begin_path();
                    self.ctx
                        .arc(sx + cell - 4.0, sy + 4.0, 2.5, 0.0, std::f64::consts::TAU)
                        .ok();
                    self.ctx.fill();
                }

                if unit.mastery_tier >= 2 {
                    let tier_color = if unit.mastery_tier >= 3 {
                        "#44ff44"
                    } else {
                        "#88cc88"
                    };
                    self.ctx.set_fill_style_str(tier_color);
                    self.ctx.set_font("7px monospace");
                    self.ctx.set_text_align("left");
                    let pips = if unit.mastery_tier >= 3 { "***" } else { "**" };
                    self.ctx.fill_text(pips, sx + 1.0, sy + cell - 9.0).ok();
                }

                if let Some(remaining) = unit.charge_remaining {
                    self.ctx.set_fill_style_str("#ffdd00");
                    self.ctx.set_font("bold 8px monospace");
                    self.ctx.set_text_align("right");
                    self.ctx
                        .fill_text(&format!("~{}", remaining), sx + cell - 1.0, sy + cell - 9.0)
                        .ok();
                }
            }
        }

        // ── Projectiles ──────────────────────────────────────────────
        for proj in &battle.projectiles {
            let (px, py) = proj.current_pos();
            let sx = grid_x + px * cell;
            let sy = grid_y + py * cell;
            self.ctx.set_shadow_color(proj.color);
            self.ctx.set_shadow_blur(8.0);
            self.ctx.set_fill_style_str(proj.color);
            self.ctx.set_font("16px 'Noto Serif SC', 'SimSun', serif");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(proj.glyph, sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                .ok();
        }
        self.ctx.set_shadow_blur(0.0);
        self.ctx.set_shadow_color("transparent");

        for arc in &battle.arcing_projectiles {
            let sx = grid_x + arc.target_x as f64 * cell;
            let sy = grid_y + arc.target_y as f64 * cell;
            let urgent = arc.turns_remaining <= 1;
            let pulse_speed = if urgent { 6.0 } else { 3.5 };
            let pulse = ((anim_t * pulse_speed).sin() * 0.2 + 0.5).clamp(0.25, 0.7);

            let (fill_r, fill_g, fill_b) = if urgent { (255, 60, 60) } else { (255, 200, 50) };
            let fill_color = format!("rgba({},{},{},{})", fill_r, fill_g, fill_b, pulse * 0.55);
            self.ctx.set_fill_style_str(&fill_color);
            self.ctx.fill_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);

            let border_alpha = if urgent { pulse * 1.2 } else { pulse * 0.8 };
            let border_color = format!("rgba({},{},{},{})", fill_r, fill_g, fill_b, border_alpha.min(1.0));
            if urgent {
                self.ctx.set_shadow_color(&format!("rgba({},{},{},0.8)", fill_r, fill_g, fill_b));
                self.ctx.set_shadow_blur(8.0);
            }
            self.ctx.set_stroke_style_str(&border_color);
            self.ctx.set_line_width(if urgent { 2.5 } else { 1.5 });
            self.ctx.stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_shadow_color("transparent");

            self.ctx.set_fill_style_str(arc.color);
            self.ctx.set_font("14px 'Noto Serif SC', 'SimSun', serif");
            self.ctx.set_text_align("center");
            self.ctx.fill_text(arc.glyph, sx + cell / 2.0, sy + cell / 2.0 + 2.0).ok();

            let count_color = if urgent { "rgba(255,255,255,0.95)" } else { "rgba(255,255,200,0.8)" };
            self.ctx.set_fill_style_str(count_color);
            self.ctx.set_font("bold 11px monospace");
            self.ctx.set_text_align("right");
            self.ctx.fill_text(
                &format!("{}", arc.turns_remaining),
                sx + cell - 3.0,
                sy + cell - 4.0,
            ).ok();
        }

        // Right panel: info area
        let panel_x = grid_x + grid_px + 16.0;
        let panel_w = self.canvas_w - panel_x - 8.0;
        let mut py = grid_y;

        // Panel background
        self.ctx.set_fill_style_str("rgba(10,8,20,0.7)");
        self.ctx
            .fill_rect(panel_x - 6.0, py - 4.0, panel_w + 8.0, grid_px + 8.0);
        self.ctx.set_stroke_style_str("rgba(100,80,140,0.4)");
        self.ctx.set_line_width(1.0);
        self.ctx
            .stroke_rect(panel_x - 6.0, py - 4.0, panel_w + 8.0, grid_px + 8.0);

        let player_unit = &battle.units[0];
        let p_bar_w = panel_w.min(130.0);

        // ─ HP section ─
        self.ctx.set_fill_style_str("#00ccdd");
        self.ctx.set_font("bold 12px monospace");
        self.ctx.set_text_align("left");
        self.ctx.fill_text("HP", panel_x, py + 12.0).ok();
        self.ctx.set_fill_style_str("#cccccc");
        self.ctx.set_font("12px monospace");
        self.ctx.set_text_align("right");
        self.ctx
            .fill_text(
                &format!("{}/{}", player_unit.hp, player_unit.max_hp),
                panel_x + p_bar_w,
                py + 12.0,
            )
            .ok();
        self.ctx.set_text_align("left");
        py += 16.0;

        let p_hp_frac = if player_unit.max_hp > 0 {
            (player_unit.hp as f64 / player_unit.max_hp as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let hp_bar_h = 8.0;
        self.ctx.set_fill_style_str("rgba(0,0,0,0.5)");
        self.ctx
            .fill_rect(panel_x - 1.0, py - 1.0, p_bar_w + 2.0, hp_bar_h + 2.0);
        self.ctx.set_fill_style_str(COL_HP_BG);
        self.ctx.fill_rect(panel_x, py, p_bar_w, hp_bar_h);
        let panel_hp_color = if p_hp_frac > 0.6 {
            COL_HP_BAR
        } else if p_hp_frac > 0.3 {
            "#ccaa22"
        } else {
            "#cc4422"
        };
        self.ctx.set_fill_style_str(panel_hp_color);
        self.ctx
            .fill_rect(panel_x, py, p_bar_w * p_hp_frac, hp_bar_h);
        self.ctx.set_stroke_style_str("rgba(255,255,255,0.12)");
        self.ctx.set_line_width(0.5);
        self.ctx.stroke_rect(panel_x, py, p_bar_w, hp_bar_h);
        py += hp_bar_h + 6.0;

        // ─ Focus section ─
        {
            let focus_frac = if battle.max_focus > 0 {
                (battle.focus as f64 / battle.max_focus as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            self.ctx.set_fill_style_str("#8888cc");
            self.ctx.set_font("bold 10px monospace");
            self.ctx.fill_text("Focus", panel_x, py + 10.0).ok();
            self.ctx.set_fill_style_str("#aaaacc");
            self.ctx.set_font("10px monospace");
            self.ctx.set_text_align("right");
            self.ctx
                .fill_text(
                    &format!("{}/{}", battle.focus, battle.max_focus),
                    panel_x + p_bar_w,
                    py + 10.0,
                )
                .ok();
            self.ctx.set_text_align("left");
            py += 14.0;
            let focus_bar_h = 5.0;
            self.ctx.set_fill_style_str("rgba(0,0,0,0.4)");
            self.ctx
                .fill_rect(panel_x - 1.0, py - 1.0, p_bar_w + 2.0, focus_bar_h + 2.0);
            self.ctx.set_fill_style_str("#222244");
            self.ctx.fill_rect(panel_x, py, p_bar_w, focus_bar_h);
            self.ctx.set_fill_style_str("#6666cc");
            self.ctx
                .fill_rect(panel_x, py, p_bar_w * focus_frac, focus_bar_h);
            self.ctx.set_stroke_style_str("rgba(255,255,255,0.08)");
            self.ctx.set_line_width(0.5);
            self.ctx.stroke_rect(panel_x, py, p_bar_w, focus_bar_h);
            py += focus_bar_h + 6.0;
        }

        // ─ Player status effects ─
        if !player_unit.statuses.is_empty() {
            self.ctx.set_font("9px monospace");
            let mut status_x = panel_x;
            for status in &player_unit.statuses {
                let lbl = status.label();
                self.ctx.set_fill_style_str(status.color());
                self.ctx.fill_text(lbl, status_x, py + 9.0).ok();
                status_x += lbl.len() as f64 * 5.5 + 6.0;
                if status_x > panel_x + p_bar_w - 20.0 {
                    py += 12.0;
                    status_x = panel_x;
                }
            }
            py += 14.0;
        }

        // ─ Stance indicator ─
        if !matches!(battle.player_stance, crate::combat::PlayerStance::Balanced) {
            self.ctx.set_font("10px monospace");
            self.ctx.set_fill_style_str(battle.player_stance.color());
            self.ctx
                .fill_text(
                    &format!("{} {}", battle.player_stance.icon(), battle.player_stance.name()),
                    panel_x,
                    py + 10.0,
                )
                .ok();
            py += 14.0;
        }

        // ─ Separator ─
        self.ctx.set_stroke_style_str("rgba(100,80,140,0.3)");
        self.ctx.set_line_width(1.0);
        self.ctx.begin_path();
        self.ctx.move_to(panel_x, py);
        self.ctx.line_to(panel_x + p_bar_w, py);
        self.ctx.stroke();
        py += 6.0;

        if !matches!(battle.weather, Weather::Normal) {
            let weather_label = battle.weather.name();
            let weather_color = match battle.weather {
                Weather::CoolantLeak => "#4488ff",
                Weather::SmokeScreen => "#aaaaaa",
                Weather::DebrisStorm => "#ccaa44",
                Weather::EnergyFlux => "#cc88ff",
                Weather::Normal => "#888888",
            };
            self.ctx.set_fill_style_str(weather_color);
            self.ctx.set_font("10px monospace");
            self.ctx.fill_text(weather_label, panel_x, py + 10.0).ok();
            py += 14.0;
        }

        if battle.radical_synergy_streak >= 2 {
            self.ctx.set_fill_style_str("#ffaa44");
            self.ctx.set_font("bold 10px monospace");
            let synergy_name = battle.radical_synergy_radical.unwrap_or("?");
            self.ctx
                .fill_text(
                    &format!(
                        "{} Synergy x{}",
                        synergy_name, battle.radical_synergy_streak
                    ),
                    panel_x,
                    py + 10.0,
                )
                .ok();
            py += 14.0;
        }

        // Turn info
        {
            let threshold = if battle.is_boss_battle { 15 } else { 10 };
            let warning = if battle.is_boss_battle { 13 } else { 8 };
            let turn_color = if battle.turn_number >= threshold {
                "#ff4422"
            } else if battle.turn_number >= warning {
                "#ffaa33"
            } else {
                "#8888aa"
            };
            self.ctx.set_fill_style_str(turn_color);
            self.ctx.set_font("10px monospace");
            let label = if battle.turn_number >= threshold {
                format!("Turn {} ─ EXHAUSTION!", battle.turn_number)
            } else if battle.turn_number >= warning {
                format!("Turn {} ─ Ink restless…", battle.turn_number)
            } else {
                format!("Turn {}", battle.turn_number)
            };
            self.ctx.fill_text(&label, panel_x, py + 10.0).ok();
            py += 16.0;
        }

        if let TacticalPhase::EnemyTurn { unit_idx, .. } = battle.phase {
            self.ctx.set_fill_style_str("#ff6666");
            self.ctx.set_font("bold 11px monospace");
            let enemy_name =
                if unit_idx < battle.units.len() && !battle.units[unit_idx].hanzi.is_empty() {
                    battle.units[unit_idx].hanzi
                } else {
                    "Enemy"
                };
            self.ctx
                .fill_text(&format!("{} acts…", enemy_name), panel_x, py + 10.0)
                .ok();
            py += 16.0;
        }

        if let TacticalPhase::Resolve { ref message, .. } = battle.phase {
            self.ctx.set_fill_style_str("#ffdd88");
            self.ctx.set_font("11px monospace");
            self.ctx.fill_text(message, panel_x, py + 10.0).ok();
            py += 16.0;
        }

        // Combo streak with glow
        if battle.combo_streak > 0 {
            let tier = battle.combo_tier_name();
            let combo_color = match battle.combo_streak {
                1..=2 => "#aaddff",
                3..=4 => "#44dd88",
                5..=7 => "#ffdd44",
                8..=11 => "#ff8844",
                _ => "#ff4422",
            };
            self.ctx.set_fill_style_str(combo_color);
            self.ctx.set_font("bold 12px monospace");
            if battle.combo_streak >= 5 {
                self.ctx.set_shadow_color(combo_color);
                self.ctx.set_shadow_blur(6.0);
            }
            self.ctx
                .fill_text(
                    &format!("{} x{}", tier, battle.combo_streak),
                    panel_x,
                    py + 10.0,
                )
                .ok();
            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_shadow_color("transparent");
            py += 16.0;
        }

        // Action menu (Command phase) — styled
        if matches!(battle.phase, TacticalPhase::Command) && battle.typing_action.is_none() {
            py += 4.0;
            self.ctx.set_stroke_style_str("rgba(100,80,140,0.3)");
            self.ctx.set_line_width(1.0);
            self.ctx.begin_path();
            self.ctx.move_to(panel_x, py);
            self.ctx.line_to(panel_x + p_bar_w, py);
            self.ctx.stroke();
            py += 6.0;

            self.ctx.set_font("bold 11px monospace");
            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.fill_text("─ Actions ─", panel_x, py + 10.0).ok();
            py += 16.0;

            let actions: &[(&str, &str, bool)] = &[
                ("M", "Move", !battle.player_moved),
                ("A", "Attack", !battle.player_acted),
                ("S", "Spell", !battle.player_acted),
                ("K", "Skill", !battle.player_acted),
                ("I", "Item", !battle.player_acted),
                ("D", "Defend", !battle.player_acted),
                ("W", "Wait", true),
                ("R", "Rotate", true),
                ("F", "Stance", true),
                ("V", "Look", true),
            ];
            self.ctx.set_font("11px monospace");
            for (hotkey, label, enabled) in actions {
                if *enabled {
                    self.ctx.set_fill_style_str("rgba(255,204,50,0.08)");
                    self.ctx
                        .fill_rect(panel_x - 2.0, py - 1.0, p_bar_w + 2.0, 14.0);
                    self.ctx.set_fill_style_str("#dde0e8");
                } else {
                    self.ctx.set_fill_style_str("#444");
                }
                self.ctx
                    .fill_text(&format!("[{}] {}", hotkey, label), panel_x, py + 10.0)
                    .ok();
                py += 14.0;
            }

            py += 4.0;
            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("9px monospace");
            self.ctx.fill_text("Esc=flee", panel_x, py + 10.0).ok();
            py += 14.0;
        }

        if battle.spell_menu_open && matches!(battle.phase, TacticalPhase::Command) {
            py += 6.0;
            self.ctx.set_fill_style_str("rgba(0,0,0,0.6)");
            self.ctx.fill_rect(
                panel_x - 4.0,
                py,
                panel_w.min(170.0),
                16.0 + battle.available_spells.len() as f64 * 18.0 + 20.0,
            );
            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.set_font("bold 11px monospace");
            self.ctx.fill_text("Spells:", panel_x, py + 12.0).ok();
            py += 18.0;

            if battle.available_spells.is_empty() {
                self.ctx.set_fill_style_str("#666");
                self.ctx.set_font("10px monospace");
                self.ctx.fill_text("(none)", panel_x, py + 10.0).ok();
                py += 16.0;
            } else {
                self.ctx.set_font("11px monospace");
                for (i, (hanzi, _pinyin, effect)) in battle.available_spells.iter().enumerate() {
                    let selected = i == battle.spell_cursor;
                    if selected {
                        self.ctx.set_fill_style_str("rgba(255,204,50,0.2)");
                        self.ctx
                            .fill_rect(panel_x - 2.0, py - 2.0, panel_w.min(166.0), 16.0);
                        self.ctx.set_fill_style_str("#00ccdd");
                    } else {
                        self.ctx.set_fill_style_str("#aaa");
                    }
                    // Check if casting this spell would trigger a combo
                    let combo_indicator =
                        if let Some(prev_elem) = battle.last_spell_element {
                            if battle.turn_number.saturating_sub(battle.last_spell_turn) <= 2 {
                                if let Some(cur_elem) =
                                    crate::combat::input::spell_effect_element(effect)
                                {
                                    if crate::combat::input::spell_combo_name(prev_elem, cur_elem)
                                        .is_some()
                                    {
                                        "⚡"
                                    } else {
                                        ""
                                    }
                                } else {
                                    ""
                                }
                            } else {
                                ""
                            }
                        } else {
                            ""
                        };
                    let label = effect.label();
                    self.ctx
                        .fill_text(
                            &format!("{}{} {}", combo_indicator, hanzi, label),
                            panel_x,
                            py + 10.0,
                        )
                        .ok();
                    py += 18.0;
                }
            }

            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("9px monospace");
            self.ctx
                .fill_text("Enter=select  Esc=back", panel_x, py + 10.0)
                .ok();
            py += 14.0;
        }

        if battle.item_menu_open && matches!(battle.phase, TacticalPhase::Command) {
            py += 6.0;
            self.ctx.set_fill_style_str("rgba(0,0,0,0.6)");
            self.ctx.fill_rect(
                panel_x - 4.0,
                py,
                panel_w.min(170.0),
                16.0 + battle.available_items.len() as f64 * 18.0 + 20.0,
            );
            self.ctx.set_fill_style_str("#44dd88");
            self.ctx.set_font("bold 11px monospace");
            self.ctx.fill_text("Items:", panel_x, py + 12.0).ok();
            py += 18.0;

            if battle.available_items.is_empty() {
                self.ctx.set_fill_style_str("#666");
                self.ctx.set_font("10px monospace");
                self.ctx.fill_text("(none)", panel_x, py + 10.0).ok();
                py += 16.0;
            } else {
                self.ctx.set_font("11px monospace");
                for (i, (_orig_idx, item)) in battle.available_items.iter().enumerate() {
                    let selected = i == battle.item_cursor;
                    if selected {
                        self.ctx.set_fill_style_str("rgba(68,221,136,0.2)");
                        self.ctx
                            .fill_rect(panel_x - 2.0, py - 2.0, panel_w.min(166.0), 16.0);
                        self.ctx.set_fill_style_str("#44dd88");
                    } else {
                        self.ctx.set_fill_style_str("#aaa");
                    }
                    self.ctx
                        .fill_text(item.short_name(), panel_x, py + 10.0)
                        .ok();
                    py += 18.0;
                }
            }

            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("9px monospace");
            self.ctx
                .fill_text("Enter=use  Esc=back", panel_x, py + 10.0)
                .ok();
            py += 14.0;
        }

        if battle.radical_picker_open && matches!(battle.phase, TacticalPhase::Command) {
            py += 6.0;
            let total_options = 1 + battle.player_radical_abilities.len();
            self.ctx.set_fill_style_str("rgba(0,0,0,0.6)");
            self.ctx.fill_rect(
                panel_x - 4.0,
                py,
                panel_w.min(200.0),
                16.0 + total_options as f64 * 18.0 + 36.0,
            );
            self.ctx.set_fill_style_str("#88ccff");
            self.ctx.set_font("bold 11px monospace");
            self.ctx.fill_text("Attack With:", panel_x, py + 12.0).ok();
            py += 18.0;

            let cursor = battle.radical_picker_cursor;
            self.ctx.set_font("11px monospace");

            let selected = cursor == 0;
            if selected {
                self.ctx.set_fill_style_str("rgba(136,204,255,0.2)");
                self.ctx
                    .fill_rect(panel_x - 2.0, py - 2.0, panel_w.min(196.0), 16.0);
                self.ctx.set_fill_style_str("#88ccff");
            } else {
                self.ctx.set_fill_style_str("#aaa");
            }
            self.ctx
                .fill_text("\u{2694} Normal Attack", panel_x, py + 10.0)
                .ok();
            py += 18.0;

            for (i, (radical, ability)) in battle.player_radical_abilities.iter().enumerate() {
                let selected = cursor == i + 1;
                if selected {
                    self.ctx.set_fill_style_str("rgba(136,204,255,0.2)");
                    self.ctx
                        .fill_rect(panel_x - 2.0, py - 2.0, panel_w.min(196.0), 16.0);
                    self.ctx.set_fill_style_str("#88ccff");
                } else {
                    self.ctx.set_fill_style_str("#aaa");
                }
                self.ctx
                    .fill_text(
                        &format!("{} {}", radical, ability.name()),
                        panel_x,
                        py + 10.0,
                    )
                    .ok();
                py += 18.0;
            }

            if cursor > 0 && cursor <= battle.player_radical_abilities.len() {
                let (_, ability) = &battle.player_radical_abilities[cursor - 1];
                self.ctx.set_fill_style_str("#667799");
                self.ctx.set_font("9px monospace");
                self.ctx
                    .fill_text(ability.description(), panel_x, py + 10.0)
                    .ok();
                py += 14.0;
            }

            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("9px monospace");
            self.ctx
                .fill_text("Enter=select  Esc=back", panel_x, py + 10.0)
                .ok();
            py += 14.0;
        }

        if battle.skill_menu_open && matches!(battle.phase, TacticalPhase::Command) {
            py += 6.0;
            let count = battle.player_radical_abilities.len();
            self.ctx.set_fill_style_str("rgba(0,0,0,0.6)");
            self.ctx.fill_rect(
                panel_x - 4.0,
                py,
                panel_w.min(200.0),
                16.0 + count as f64 * 18.0 + 36.0,
            );
            self.ctx.set_fill_style_str("#ff9944");
            self.ctx.set_font("bold 11px monospace");
            self.ctx.fill_text("Skills:", panel_x, py + 12.0).ok();
            py += 18.0;

            if count == 0 {
                self.ctx.set_fill_style_str("#666");
                self.ctx.set_font("10px monospace");
                self.ctx.fill_text("(none)", panel_x, py + 10.0).ok();
                py += 16.0;
            } else {
                self.ctx.set_font("11px monospace");
                for (i, (radical, ability)) in battle.player_radical_abilities.iter().enumerate() {
                    let selected = i == battle.skill_menu_cursor;
                    if selected {
                        self.ctx.set_fill_style_str("rgba(255,153,68,0.2)");
                        self.ctx
                            .fill_rect(panel_x - 2.0, py - 2.0, panel_w.min(196.0), 16.0);
                        self.ctx.set_fill_style_str("#ff9944");
                    } else {
                        self.ctx.set_fill_style_str("#aaa");
                    }
                    self.ctx
                        .fill_text(
                            &format!("{} {} [{}]", radical, ability.name(), ability.skill_type_label()),
                            panel_x,
                            py + 10.0,
                        )
                        .ok();
                    py += 18.0;
                }
            }

            if battle.skill_menu_cursor < count {
                let (_, ability) = &battle.player_radical_abilities[battle.skill_menu_cursor];
                self.ctx.set_fill_style_str("#997744");
                self.ctx.set_font("9px monospace");
                self.ctx
                    .fill_text(ability.description(), panel_x, py + 10.0)
                    .ok();
                py += 14.0;
            }

            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("9px monospace");
            self.ctx
                .fill_text("Enter=use  Esc=back", panel_x, py + 10.0)
                .ok();
            py += 14.0;
        }

        // Targeting mode label
        if let TacticalPhase::Targeting { ref mode, .. } = battle.phase {
            py += 6.0;
            let label = match mode {
                TargetMode::Move => "Select move target",
                TargetMode::Attack => "Select attack target",
                TargetMode::Spell { .. } => "Select spell target",
                TargetMode::ShieldBreak => "Select shield target",
                TargetMode::Skill => "Select skill target",
            };
            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.set_font("bold 11px monospace");
            self.ctx.fill_text(label, panel_x, py + 10.0).ok();
            py += 16.0;
            self.ctx.set_fill_style_str("#999");
            self.ctx.set_font("9px monospace");
            self.ctx
                .fill_text("Arrows=navigate Enter=confirm", panel_x, py + 10.0)
                .ok();
            py += 12.0;
            self.ctx.fill_text("Esc=cancel", panel_x, py + 10.0).ok();
            py += 16.0;
        }

        if let TacticalPhase::Look { cursor_x, cursor_y } = battle.phase {
            py += 6.0;
            // ── LOOK MODE header ──
            self.ctx.set_fill_style_str("#66ccff");
            self.ctx.set_font("bold 11px monospace");
            self.ctx
                .fill_text("── LOOK MODE ──", panel_x, py + 10.0)
                .ok();
            py += 18.0;

            if let Some(tile) = battle.arena.tile(cursor_x, cursor_y) {
                // ── Tile info section ──
                self.ctx.set_fill_style_str("rgba(40,40,60,0.5)");
                self.ctx
                    .fill_rect(panel_x - 4.0, py - 2.0, p_bar_w + 8.0, 14.0);

                self.ctx.set_fill_style_str("#aaddee");
                self.ctx.set_font("bold 10px monospace");
                self.ctx
                    .fill_text(
                        &format!("Tile: {}", tile.name()),
                        panel_x,
                        py + 10.0,
                    )
                    .ok();
                py += 15.0;

                // Movement cost and LOS
                let move_cost = 1 + tile.extra_move_cost();
                let walkable = tile.is_walkable();
                let blocks = tile.blocks_los();
                self.ctx.set_fill_style_str("#999");
                self.ctx.set_font("9px monospace");
                if !walkable {
                    self.ctx
                        .fill_text("Impassable", panel_x, py + 10.0)
                        .ok();
                } else {
                    self.ctx
                        .fill_text(
                            &format!(
                                "Move: {}  LOS: {}",
                                move_cost,
                                if blocks { "blocked" } else { "clear" }
                            ),
                            panel_x,
                            py + 10.0,
                        )
                        .ok();
                }
                py += 12.0;

                // Special effects
                if let Some(fx) = tile.special_effects() {
                    self.ctx.set_fill_style_str("#ddaa44");
                    self.ctx.set_font("9px monospace");
                    self.ctx
                        .fill_text(&format!("⚡ {}", fx), panel_x, py + 10.0)
                        .ok();
                    py += 12.0;
                }

                // ── Unit info section ──
                for unit in &battle.units {
                    if unit.alive && unit.x == cursor_x && unit.y == cursor_y {
                        py += 4.0;
                        // Separator line
                        self.ctx
                            .set_stroke_style_str("rgba(100,140,180,0.3)");
                        self.ctx.set_line_width(1.0);
                        self.ctx.begin_path();
                        self.ctx.move_to(panel_x, py);
                        self.ctx.line_to(panel_x + p_bar_w, py);
                        self.ctx.stroke();
                        py += 6.0;

                        if unit.is_player() {
                            self.ctx.set_fill_style_str("#00ccdd");
                            self.ctx.set_font("bold 10px monospace");
                            self.ctx
                                .fill_text("You (Player)", panel_x, py + 10.0)
                                .ok();
                            py += 14.0;
                            // Player HP bar
                            let phf = if unit.max_hp > 0 {
                                (unit.hp as f64 / unit.max_hp as f64)
                                    .clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            self.ctx.set_fill_style_str("#333");
                            self.ctx.fill_rect(panel_x, py, p_bar_w, 6.0);
                            self.ctx.set_fill_style_str(if phf > 0.5 {
                                "#44cc44"
                            } else if phf > 0.25 {
                                "#ccaa22"
                            } else {
                                "#cc4422"
                            });
                            self.ctx
                                .fill_rect(panel_x, py, p_bar_w * phf, 6.0);
                            py += 8.0;
                            self.ctx.set_fill_style_str("#ccc");
                            self.ctx.set_font("9px monospace");
                            self.ctx
                                .fill_text(
                                    &format!("HP: {}/{}", unit.hp, unit.max_hp),
                                    panel_x,
                                    py + 10.0,
                                )
                                .ok();
                            py += 14.0;
                        } else if unit.is_companion() {
                            let name = if unit.hanzi.is_empty() {
                                "Companion"
                            } else {
                                unit.hanzi
                            };
                            self.ctx.set_fill_style_str("#44cc88");
                            self.ctx
                                .fill_text(
                                    &format!(
                                        "{} HP:{}/{}",
                                        name, unit.hp, unit.max_hp
                                    ),
                                    panel_x,
                                    py + 10.0,
                                )
                                .ok();
                            py += 14.0;
                        } else {
                            // ── Enemy header ──
                            self.ctx.set_fill_style_str("#ff6666");
                            self.ctx.set_font("bold 10px monospace");
                            let name = if unit.hanzi.is_empty() {
                                "Enemy"
                            } else {
                                unit.hanzi
                            };
                            let label = if !unit.pinyin.is_empty() {
                                format!("{} ({})", name, unit.pinyin)
                            } else {
                                name.to_string()
                            };
                            self.ctx
                                .fill_text(&label, panel_x, py + 10.0)
                                .ok();
                            py += 14.0;

                            // HP bar
                            let ehf = if unit.max_hp > 0 {
                                (unit.hp as f64 / unit.max_hp as f64)
                                    .clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            self.ctx.set_fill_style_str("#333");
                            self.ctx.fill_rect(panel_x, py, p_bar_w, 6.0);
                            self.ctx.set_fill_style_str(if ehf > 0.5 {
                                "#cc4444"
                            } else if ehf > 0.25 {
                                "#cc6622"
                            } else {
                                "#882222"
                            });
                            self.ctx
                                .fill_rect(panel_x, py, p_bar_w * ehf, 6.0);
                            py += 8.0;
                            self.ctx.set_fill_style_str("#ccc");
                            self.ctx.set_font("9px monospace");
                            self.ctx
                                .fill_text(
                                    &format!(
                                        "HP: {}/{}",
                                        unit.hp, unit.max_hp
                                    ),
                                    panel_x,
                                    py + 10.0,
                                )
                                .ok();
                            py += 12.0;

                            // Armor, Speed, Element
                            let mut stats_line = format!(
                                "Armor:{}  Spd:{}",
                                unit.radical_armor, unit.speed
                            );
                            if let Some(elem) = unit.wuxing_element {
                                stats_line.push_str(&format!(
                                    "  {}",
                                    elem.label()
                                ));
                            }
                            self.ctx.set_fill_style_str("#aaa");
                            self.ctx.set_font("9px monospace");
                            self.ctx
                                .fill_text(&stats_line, panel_x, py + 10.0)
                                .ok();
                            py += 13.0;

                            // ── Status effects ──
                            if !unit.statuses.is_empty() {
                                self.ctx.set_font("9px monospace");
                                let mut sx_off = 0.0;
                                for st in &unit.statuses {
                                    let lbl = format!(
                                        "{} {}t",
                                        st.label(),
                                        st.turns_left
                                    );
                                    let lbl_w =
                                        lbl.len() as f64 * 5.5 + 4.0;
                                    if sx_off + lbl_w > p_bar_w
                                        && sx_off > 0.0
                                    {
                                        py += 12.0;
                                        sx_off = 0.0;
                                    }
                                    self.ctx.set_fill_style_str(st.color());
                                    self.ctx
                                        .fill_text(
                                            &lbl,
                                            panel_x + sx_off,
                                            py + 10.0,
                                        )
                                        .ok();
                                    sx_off += lbl_w;
                                }
                                py += 13.0;
                            }

                            // ── Abilities section ──
                            if !unit.radical_actions.is_empty() {
                                py += 2.0;
                                self.ctx.set_stroke_style_str(
                                    "rgba(100,140,180,0.3)",
                                );
                                self.ctx.set_line_width(1.0);
                                self.ctx.begin_path();
                                self.ctx.move_to(panel_x, py);
                                self.ctx.line_to(panel_x + p_bar_w, py);
                                self.ctx.stroke();
                                py += 4.0;

                                self.ctx.set_fill_style_str("#88aacc");
                                self.ctx.set_font("bold 9px monospace");
                                self.ctx
                                    .fill_text(
                                        "── Abilities ──",
                                        panel_x,
                                        py + 10.0,
                                    )
                                    .ok();
                                py += 14.0;

                                for skill in &unit.radical_actions {
                                    // Skill name with radical
                                    self.ctx.set_fill_style_str(
                                        skill.type_color(),
                                    );
                                    self.ctx.set_font("bold 9px monospace");
                                    let skill_label = format!(
                                        "{} {}",
                                        skill.radical(),
                                        skill.name()
                                    );
                                    let display_label =
                                        if skill_label.chars().count() > 24 {
                                            let s: String = skill_label
                                                .chars()
                                                .take(21)
                                                .collect();
                                            format!("{}...", s)
                                        } else {
                                            skill_label
                                        };
                                    self.ctx
                                        .fill_text(
                                            &display_label,
                                            panel_x,
                                            py + 10.0,
                                        )
                                        .ok();
                                    py += 11.0;

                                    // Description (word-wrapped)
                                    self.ctx.set_fill_style_str("#999");
                                    self.ctx.set_font("8px monospace");
                                    let desc = skill.description();
                                    let words: Vec<&str> =
                                        desc.split_whitespace().collect();
                                    let mut line = String::from("  ");
                                    for word in &words {
                                        if line.len() + word.len() + 1 > 24
                                            && line.len() > 2
                                        {
                                            self.ctx
                                                .fill_text(
                                                    &line, panel_x,
                                                    py + 10.0,
                                                )
                                                .ok();
                                            py += 10.0;
                                            line = String::from("  ");
                                        }
                                        if line.len() > 2 {
                                            line.push(' ');
                                        }
                                        line.push_str(word);
                                    }
                                    if line.len() > 2 {
                                        self.ctx
                                            .fill_text(
                                                &line, panel_x, py + 10.0,
                                            )
                                            .ok();
                                        py += 10.0;
                                    }

                                    // Range + Damage + Type
                                    self.ctx.set_fill_style_str("#777");
                                    self.ctx.set_font("8px monospace");
                                    let info_line = format!(
                                        "  {} | {} | {}",
                                        skill.range_info(),
                                        skill.damage_info(),
                                        skill.attack_type()
                                    );
                                    let info_display =
                                        if info_line.chars().count() > 26 {
                                            let s: String = info_line
                                                .chars()
                                                .take(23)
                                                .collect();
                                            format!("{}...", s)
                                        } else {
                                            info_line
                                        };
                                    self.ctx
                                        .fill_text(
                                            &info_display,
                                            panel_x,
                                            py + 10.0,
                                        )
                                        .ok();
                                    py += 13.0;
                                }
                            }

                            // ── Intent ──
                            if let Some(intent) = &unit.intent {
                                py += 2.0;
                                self.ctx.set_fill_style_str("#ddaa44");
                                self.ctx.set_font("bold 9px monospace");
                                self.ctx
                                    .fill_text(
                                        &format!(
                                            "Intent: {} →",
                                            intent.label()
                                        ),
                                        panel_x,
                                        py + 10.0,
                                    )
                                    .ok();
                                py += 14.0;
                            }
                        }
                    }
                }
            }

            py += 6.0;
            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("9px monospace");
            self.ctx
                .fill_text("Arrows=look  Esc/V=exit", panel_x, py + 10.0)
                .ok();
            py += 14.0;
        }

        // Resolve phase message banner
        if let TacticalPhase::Resolve {
            ref message, timer, ..
        } = battle.phase
        {
            let banner_h = 36.0;
            let banner_y = grid_y + grid_px * 0.4;
            // Semi-transparent overlay
            self.ctx.set_fill_style_str("rgba(0,0,0,0.35)");
            self.ctx.fill_rect(grid_x, banner_y, grid_px, banner_h);
            // Message text centered on the grid
            let alpha = if timer > 20 { 1.0 } else { timer as f64 / 20.0 };
            self.ctx
                .set_fill_style_str(&format!("rgba(255,220,100,{})", alpha));
            self.ctx.set_font("bold 14px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(message, grid_x + grid_px / 2.0, banner_y + 22.0)
                .ok();
            self.ctx.set_text_align("left");
        }

        // ── Arena event warning banner ──────────────────────────────────
        if let Some(ref pending) = battle.pending_event {
            let banner_h = 28.0;
            let banner_y = grid_y - 2.0;
            let danger = pending.danger_level();
            let bg_color = match danger {
                "damaging" => "rgba(180,30,30,0.55)",
                "beneficial" => "rgba(30,140,60,0.55)",
                _ => "rgba(40,80,160,0.55)",
            };
            self.ctx.set_fill_style_str(bg_color);
            self.ctx.fill_rect(grid_x, banner_y, grid_px, banner_h);

            let text_color = match danger {
                "damaging" => "#ff6666",
                "beneficial" => "#88ff88",
                _ => "#88bbff",
            };
            self.ctx.set_fill_style_str(text_color);
            self.ctx.set_font("bold 12px monospace");
            self.ctx.set_text_align("center");
            let pulse = (anim_t * 4.0).sin().abs();
            let warning_text = format!("⚠ {} incoming! ⚠", pending.name());
            self.ctx.set_shadow_color(text_color);
            self.ctx.set_shadow_blur(4.0 + pulse * 4.0);
            self.ctx
                .fill_text(&warning_text, grid_x + grid_px / 2.0, banner_y + 18.0)
                .ok();
            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_shadow_color("transparent");
            self.ctx.set_text_align("left");
        }

        // ── Arena event trigger message (large text) ────────────────────
        if let Some(ref event_msg) = battle.event_message {
            if battle.event_message_timer > 0 {
                let alpha = (battle.event_message_timer as f64 / 90.0).min(1.0);
                let scale = 1.0 + (1.0 - alpha) * 0.3;
                let font_size = (20.0 * scale) as u32;
                self.ctx
                    .set_fill_style_str(&format!("rgba(255,240,180,{})", alpha));
                self.ctx
                    .set_font(&format!("bold {}px monospace", font_size));
                self.ctx.set_text_align("center");
                let msg_y = grid_y + grid_px * 0.3;
                self.ctx.set_shadow_color("rgba(255,200,50,0.6)");
                self.ctx.set_shadow_blur(8.0);
                self.ctx
                    .fill_text(event_msg, grid_x + grid_px / 2.0, msg_y)
                    .ok();
                self.ctx.set_shadow_blur(0.0);
                self.ctx.set_shadow_color("transparent");
                self.ctx.set_text_align("left");
            }
        }

        // ── Spell combo notification overlay ─────────────────────────────
        if let Some(ref combo_msg) = battle.combo_message {
            if battle.combo_message_timer > 0 {
                let alpha = (battle.combo_message_timer as f64 / 60.0).min(1.0);
                let scale = 1.0 + (1.0 - alpha) * 0.5;
                let font_size = (22.0 * scale) as u32;
                self.ctx
                    .set_fill_style_str(&format!("rgba(120,220,255,{})", alpha));
                self.ctx
                    .set_font(&format!("bold {}px monospace", font_size));
                self.ctx.set_text_align("center");
                let msg_y = grid_y + grid_px * 0.45;
                self.ctx.set_shadow_color(&format!("rgba(80,180,255,{})", alpha * 0.8));
                self.ctx.set_shadow_blur(12.0);
                self.ctx
                    .fill_text(combo_msg, grid_x + grid_px / 2.0, msg_y)
                    .ok();
                self.ctx.set_shadow_blur(0.0);
                self.ctx.set_shadow_color("transparent");
                self.ctx.set_text_align("left");
            }
        }

        // Typing input box (when active)
        if let Some(ref action) = battle.typing_action {
            let target_label = match action {
                TypingAction::BasicAttack { target_unit } => {
                    let u = &battle.units[*target_unit];
                    if u.hanzi.is_empty() {
                        "Enemy".to_string()
                    } else {
                        format!("{}", u.hanzi)
                    }
                }
                TypingAction::SpellCast {
                    spell_idx, effect, ..
                } => {
                    if *spell_idx < battle.available_spells.len() {
                        let hanzi = battle.available_spells[*spell_idx].0;
                        format!("{} {}", hanzi, effect.label())
                    } else {
                        effect.label().to_string()
                    }
                }
                TypingAction::ShieldBreak { component, .. } => format!("Break {}", component),
                TypingAction::EliteChain {
                    target_unit,
                    syllable_progress,
                    total_syllables,
                    ..
                } => {
                    let u = &battle.units[*target_unit];
                    let hanzi = if u.hanzi.is_empty() { "Enemy" } else { u.hanzi };
                    format!("{} [{}/{}]", hanzi, syllable_progress + 1, total_syllables)
                }
            };

            let input_w = panel_w.min(160.0);
            let input_x = panel_x;
            let input_y = py + 4.0;

            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.set_font("bold 12px monospace");
            self.ctx
                .fill_text(&target_label, input_x, input_y + 10.0)
                .ok();

            // Show the hanzi large
            if let TypingAction::BasicAttack { target_unit } = action {
                let u = &battle.units[*target_unit];
                if !u.hanzi.is_empty() {
                    self.ctx.set_fill_style_str("#ff6666");
                    self.ctx.set_font("36px 'Noto Serif SC', 'SimSun', serif");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text(u.hanzi, input_x + input_w / 2.0, input_y + 50.0)
                        .ok();
                    self.ctx.set_text_align("left");
                }
            }

            if let TypingAction::SpellCast { spell_idx, .. } = action {
                if *spell_idx < battle.available_spells.len() {
                    let hanzi = battle.available_spells[*spell_idx].0;
                    self.ctx.set_fill_style_str("#44aaff");
                    self.ctx.set_font("36px 'Noto Serif SC', 'SimSun', serif");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text(hanzi, input_x + input_w / 2.0, input_y + 50.0)
                        .ok();
                    self.ctx.set_text_align("left");
                }
            }

            if let TypingAction::EliteChain {
                target_unit,
                syllable_progress,
                total_syllables,
                ..
            } = action
            {
                let u = &battle.units[*target_unit];
                if !u.hanzi.is_empty() {
                    self.ctx.set_fill_style_str("#ff9933");
                    self.ctx.set_font("36px 'Noto Serif SC', 'SimSun', serif");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text(u.hanzi, input_x + input_w / 2.0, input_y + 50.0)
                        .ok();
                    self.ctx.set_text_align("left");

                    let progress_text =
                        format!("Syllable {}/{}", syllable_progress + 1, total_syllables);
                    self.ctx.set_fill_style_str("#00ccdd");
                    self.ctx.set_font("10px monospace");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text(&progress_text, input_x + input_w / 2.0, input_y + 56.0)
                        .ok();
                    self.ctx.set_text_align("left");
                }
            }

            let box_y = input_y + 58.0;
            self.ctx.set_fill_style_str("rgba(0,0,0,0.5)");
            self.ctx.fill_rect(input_x, box_y, input_w, 26.0);
            self.ctx.set_stroke_style_str("#555");
            self.ctx.set_line_width(1.0);
            self.ctx.stroke_rect(input_x, box_y, input_w, 26.0);

            let display = if battle.typing_buffer.is_empty() {
                "type pinyin…"
            } else {
                &battle.typing_buffer
            };
            self.ctx
                .set_fill_style_str(if battle.typing_buffer.is_empty() {
                    "#555"
                } else {
                    "#00ccdd"
                });
            self.ctx.set_font("14px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(display, input_x + input_w / 2.0, box_y + 18.0)
                .ok();
            self.ctx.set_text_align("left");

            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("9px monospace");
            self.ctx
                .fill_text("Enter=submit  Esc=cancel", input_x, box_y + 40.0)
                .ok();
        }

        // Battle log (bottom area) — styled with gradient background
        let log_x = grid_x;
        let log_y = grid_y + grid_px + 8.0;
        let log_w = grid_px;
        let log_h = self.canvas_h - log_y - 8.0;
        let line_h = 14.0;
        let max_lines = ((log_h - 10.0) / line_h).floor() as usize;

        // Gradient background: darker at bottom, slightly lighter at top
        for gi in 0..4 {
            let gy_off = gi as f64 * (log_h / 4.0);
            let alpha = 0.45 + gi as f64 * 0.08;
            self.ctx
                .set_fill_style_str(&format!("rgba(8,6,16,{:.3})", alpha));
            self.ctx.fill_rect(log_x, log_y + gy_off, log_w, log_h / 4.0);
        }
        // Accent line at top of log
        self.ctx.set_fill_style_str("rgba(100,80,160,0.3)");
        self.ctx.fill_rect(log_x, log_y, log_w, 1.0);
        self.ctx.set_stroke_style_str("rgba(100,80,140,0.25)");
        self.ctx.set_line_width(1.0);
        self.ctx.stroke_rect(log_x, log_y, log_w, log_h);

        // "LOG" label
        self.ctx.set_fill_style_str("rgba(120,100,160,0.5)");
        self.ctx.set_font("bold 8px monospace");
        self.ctx.set_text_align("right");
        self.ctx
            .fill_text("LOG", log_x + log_w - 4.0, log_y + 10.0)
            .ok();
        self.ctx.set_text_align("left");

        self.ctx.set_font("10px monospace");
        let start = if battle.log.len() > max_lines {
            battle.log.len() - max_lines
        } else {
            0
        };
        let total_showing = battle.log[start..].len();
        for (i, msg) in battle.log[start..].iter().enumerate() {
            let recency = if total_showing > 0 {
                i as f64 / total_showing as f64
            } else {
                1.0
            };
            let alpha = (recency * 0.6 + 0.4).min(1.0);
            let color = if msg.contains("damage") || msg.contains("hit") || msg.contains("kill") {
                format!("rgba(255,130,100,{})", alpha)
            } else if msg.contains("heal") || msg.contains("restore") {
                format!("rgba(100,220,100,{})", alpha)
            } else if msg.contains("ability") || msg.contains("cast") {
                format!("rgba(130,160,255,{})", alpha)
            } else if msg.contains("move") || msg.contains("walk") {
                format!("rgba(160,160,180,{})", alpha * 0.8)
            } else {
                format!("rgba(180,175,190,{})", alpha)
            };
            self.ctx.set_fill_style_str(&color);
            self.ctx
                .fill_text(msg, log_x + 6.0, log_y + 12.0 + i as f64 * line_h)
                .ok();
        }

        // Fade overlay at top of log when scrolled
        if battle.log.len() > max_lines {
            for fade_i in 0..3 {
                let fade_alpha = 0.6 - fade_i as f64 * 0.2;
                self.ctx
                    .set_fill_style_str(&format!("rgba(8,6,16,{})", fade_alpha));
                self.ctx.fill_rect(
                    log_x + 1.0,
                    log_y + 1.0 + fade_i as f64 * 6.0,
                    log_w - 2.0,
                    6.0,
                );
            }
        }

        // Exhaustion border pulse
        {
            let warning_turn: u32 = if battle.is_boss_battle { 13 } else { 8 };
            let threshold: u32 = if battle.is_boss_battle { 15 } else { 10 };
            if battle.turn_number >= warning_turn {
                let intensity = if battle.turn_number >= threshold {
                    0.7
                } else {
                    0.4
                };
                let pulse = ((anim_t * 3.0).sin() * 0.3 + intensity).max(0.1).min(1.0);
                let border_w = 3.0;
                let color = format!("rgba(255,40,40,{})", pulse);
                self.ctx.set_fill_style_str(&color);
                self.ctx.fill_rect(0.0, 0.0, self.canvas_w, border_w);
                self.ctx
                    .fill_rect(0.0, self.canvas_h - border_w, self.canvas_w, border_w);
                self.ctx.fill_rect(0.0, 0.0, border_w, self.canvas_h);
                self.ctx
                    .fill_rect(self.canvas_w - border_w, 0.0, border_w, self.canvas_h);
            }
        }

        // End phase splash
        if let TacticalPhase::End { victory, .. } = battle.phase {
            self.ctx.set_fill_style_str("rgba(0,0,0,0.75)");
            self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

            let text = if victory { "VICTORY" } else { "DEFEAT" };
            let color = if victory { "#44dd88" } else { "#ff4444" };
            let glow = if victory {
                "rgba(68,221,136,0.5)"
            } else {
                "rgba(255,68,68,0.5)"
            };
            self.ctx.set_shadow_color(glow);
            self.ctx.set_shadow_blur(20.0);
            self.ctx.set_fill_style_str(color);
            self.ctx.set_font("bold 48px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(text, self.canvas_w / 2.0, self.canvas_h / 2.0)
                .ok();
            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_shadow_color("transparent");

            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("14px monospace");
            self.ctx
                .fill_text(
                    "Press any key to continue",
                    self.canvas_w / 2.0,
                    self.canvas_h / 2.0 + 36.0,
                )
                .ok();
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

    pub fn draw_inventory(
        &self,
        player: &Player,
        floor_num: i32,
        recipes_found: usize,
        best_floor: i32,
        total_kills: u32,
        companion: Option<crate::game::Companion>,
        companion_level: u8,
        item_labels: &[String],
        inventory_cursor: usize,
        inventory_inspect: Option<usize>,
        crafting_mode: bool,
        crafting_first: Option<usize>,
        crafting_cursor: usize,
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
        self.ctx.set_fill_style_str("#00ccdd");
        self.ctx
            .fill_text("Inventory", self.canvas_w / 2.0, box_y + 28.0)
            .ok();
        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#9aaad8");
        let header_hint = if crafting_mode {
            "Esc back   ↑↓ navigate   Enter select item"
        } else {
            "I/Esc close   ↑↓ navigate items   Enter inspect   C craft"
        };
        self.ctx
            .fill_text(
                header_hint,
                self.canvas_w / 2.0,
                box_y + 46.0,
            )
            .ok();

        let class_name = player.class.data().name_en;
        let companion_text = companion
            .map(|ally| {
                if companion_level > 0 {
                    format!("{} {} Lv.{}", ally.icon(), ally.name(), companion_level)
                } else {
                    format!("{} {}", ally.icon(), ally.name())
                }
            })
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
            ItemState,
        ); 3] = [
            (
                "Weapon",
                player.weapon,
                player.enchantments[0],
                player.weapon_state,
            ),
            (
                "Armor ",
                player.armor,
                player.enchantments[1],
                player.armor_state,
            ),
            (
                "Charm ",
                player.charm,
                player.enchantments[2],
                player.charm_state,
            ),
        ];
        for (slot_idx, (label, equip, enchant, state)) in equip_slots.iter().enumerate() {
            let selected = inventory_cursor == slot_idx;
            if selected {
                self.ctx.set_fill_style_str("rgba(255,204,51,0.15)");
                self.ctx
                    .fill_rect(left_x + 8.0, left_y - 12.0, left_w - 16.0, 16.0);
            }
            self.ctx
                .set_fill_style_str(if selected { "#00ccdd" } else { "#dde7ff" });
            let marker = if selected { "▸" } else { " " };
            self.ctx
                .fill_text(
                    &format!(
                        "{} {}: {}",
                        marker,
                        label,
                        equipment_name(*equip, *enchant, *state)
                    ),
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
        let section_title = if crafting_mode {
            if crafting_first.is_some() {
                "Crafting — Select second item"
            } else {
                "Crafting — Select first item"
            }
        } else {
            "Consumables"
        };
        self.ctx
            .fill_text(section_title, left_x + 12.0, left_y)
            .ok();
        left_y += 18.0;
        self.ctx.set_fill_style_str("#dde7ff");
        if player.items.is_empty() {
            self.ctx
                .fill_text("No consumables picked up yet.", left_x + 12.0, left_y)
                .ok();
            left_y += 16.0;
        } else {
            // Determine which item kind is selected first (for recipe highlighting)
            let first_kind = if crafting_mode {
                crafting_first.and_then(|fi| player.items.get(fi).map(|it| it.kind()))
            } else {
                None
            };
            for (idx, label) in item_labels.iter().enumerate() {
                let selected = if crafting_mode {
                    crafting_cursor == idx
                } else {
                    inventory_cursor == idx + 3
                };
                let is_first_pick = crafting_mode && crafting_first == Some(idx);
                let is_compatible = first_kind
                    .and_then(|fk| player.items.get(idx).map(|it| {
                        crafting_first != Some(idx)
                            && crate::player::has_recipe_with(fk, it.kind())
                    }))
                    .unwrap_or(false);

                if is_first_pick {
                    self.ctx.set_fill_style_str("rgba(100,200,255,0.18)");
                    self.ctx
                        .fill_rect(left_x + 8.0, left_y - 12.0, left_w - 16.0, 16.0);
                } else if selected {
                    self.ctx.set_fill_style_str("rgba(255,204,51,0.15)");
                    self.ctx
                        .fill_rect(left_x + 8.0, left_y - 12.0, left_w - 16.0, 16.0);
                } else if is_compatible {
                    self.ctx.set_fill_style_str("rgba(80,255,120,0.10)");
                    self.ctx
                        .fill_rect(left_x + 8.0, left_y - 12.0, left_w - 16.0, 16.0);
                }

                let color = if is_first_pick {
                    "#66ccff"
                } else if selected {
                    "#00ccdd"
                } else if is_compatible {
                    "#66ff88"
                } else {
                    "#dde7ff"
                };
                self.ctx.set_fill_style_str(color);
                if let Some(item) = player.items.get(idx) {
                    self.draw_sprite_icon(
                        item_sprite_key(item),
                        left_x + 12.0,
                        left_y - 11.0,
                        12.0,
                    );
                }
                let marker = if is_first_pick {
                    "★"
                } else if selected {
                    "▸"
                } else {
                    " "
                };
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

        // Show recipe preview when both items are selected
        if crafting_mode {
            if let Some(fi) = crafting_first {
                if let (Some(item1), Some(item2)) = (
                    player.items.get(fi),
                    player.items.get(crafting_cursor),
                ) {
                    if fi != crafting_cursor {
                        left_y += 4.0;
                        if let Some(recipe) =
                            crate::player::find_crafting_recipe(item1.kind(), item2.kind())
                        {
                            self.ctx.set_fill_style_str("#66ff88");
                            self.ctx
                                .fill_text(
                                    &format!("→ {}", recipe.output_name),
                                    left_x + 12.0,
                                    left_y,
                                )
                                .ok();
                        } else {
                            self.ctx.set_fill_style_str("#ff6666");
                            self.ctx
                                .fill_text("✗ No recipe", left_x + 12.0, left_y)
                                .ok();
                        }
                    }
                }
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
                .fill_text("🛡 Energy Barrier Active", left_x + 12.0, left_y)
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
        let footer = if crafting_mode {
            "Select two items to combine. Compatible pairs glow green."
        } else if item_labels.iter().any(|label| label.starts_with('?')) {
            "Mystery seals identify themselves on use. Use 1-5 in exploration to test them."
        } else {
            "Use 1-5 to consume items. C to craft/combine two items together."
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
            self.ctx.set_stroke_style_str("#00ccdd");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(pop_x, pop_y, pop_w, pop_h);

            self.ctx.set_fill_style_str("#00ccdd");
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

        // Full-screen dim
        self.ctx.set_fill_style_str("rgba(0,0,0,0.88)");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

        // Panel background
        self.ctx.set_fill_style_str("rgba(10,8,20,0.98)");
        self.ctx.fill_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("rgba(100,80,140,0.4)");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

        // Inner highlight border
        self.ctx.set_stroke_style_str("rgba(120,90,180,0.15)");
        self.ctx.set_line_width(1.0);
        self.ctx
            .stroke_rect(box_x + 1.0, box_y + 1.0, box_w - 2.0, box_h - 2.0);

        // Title with shadow glow
        self.ctx.set_text_align("center");
        self.ctx.set_font("bold 22px monospace");
        self.ctx.set_shadow_color("#cc99ff");
        self.ctx.set_shadow_blur(12.0);
        self.ctx.set_fill_style_str("#cc99ff");
        self.ctx
            .fill_text("─── Spellbook ───", self.canvas_w / 2.0, box_y + 28.0)
            .ok();
        self.ctx.set_shadow_blur(0.0);

        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#7784aa");
        self.ctx
            .fill_text("B / Esc to close", self.canvas_w / 2.0, box_y + 46.0)
            .ok();

        // Separator line below title
        self.ctx.set_stroke_style_str("rgba(100,80,140,0.3)");
        self.ctx.set_line_width(1.0);
        self.ctx.begin_path();
        self.ctx.move_to(box_x + 16.0, box_y + 54.0);
        self.ctx.line_to(box_x + box_w - 16.0, box_y + 54.0);
        self.ctx.stroke();

        if player.spells.is_empty() {
            self.ctx.set_fill_style_str("#666");
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
        let card_h = 60.0;

        for (idx, spell) in player.spells.iter().enumerate() {
            let col = if col_y[0] <= col_y[1] { 0 } else { 1 };
            let x = col_x[col];
            let y = &mut col_y[col];

            if *y + card_h + 4.0 > max_y {
                continue;
            }

            let selected = idx == player.selected_spell;

            // School-based accent color for the left border
            let school_color = spell_school_color(&spell.effect);

            // Card background
            if selected {
                self.ctx.set_fill_style_str("rgba(204,153,255,0.12)");
            } else {
                self.ctx.set_fill_style_str("rgba(255,255,255,0.03)");
            }
            self.ctx.fill_rect(x + 4.0, *y - 4.0, col_w - 16.0, card_h);

            // School-colored left accent bar
            self.ctx.set_fill_style_str(school_color);
            self.ctx.fill_rect(x + 4.0, *y - 4.0, 3.0, card_h);

            // Card border (subtle)
            if selected {
                self.ctx.set_stroke_style_str("rgba(204,153,255,0.4)");
            } else {
                self.ctx.set_stroke_style_str("rgba(100,80,140,0.2)");
            }
            self.ctx.set_line_width(1.0);
            self.ctx
                .stroke_rect(x + 4.0, *y - 4.0, col_w - 16.0, card_h);

            // Spell icon
            self.draw_sprite_icon(spell_sprite_key(&spell.effect), x + 12.0, *y - 2.0, 14.0);

            // Spell name
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
                    x + 30.0,
                    *y + 10.0,
                )
                .ok();

            // Meaning
            self.ctx.set_font("11px monospace");
            self.ctx.set_fill_style_str("#aab8dd");
            self.ctx
                .fill_text(&format!("\"{}\"", spell.meaning), x + 30.0, *y + 26.0)
                .ok();

            // Description
            self.ctx.set_fill_style_str("#7889aa");
            self.ctx
                .fill_text(&spell.effect.description(), x + 30.0, *y + 40.0)
                .ok();

            // Spell index number (right side)
            self.ctx.set_text_align("right");
            self.ctx.set_font("10px monospace");
            self.ctx.set_fill_style_str("#556688");
            self.ctx
                .fill_text(&format!("#{}", idx + 1), x + col_w - 20.0, *y + 10.0)
                .ok();
            self.ctx.set_text_align("left");

            *y += card_h + 4.0;
        }

        // Footer separator
        self.ctx.set_stroke_style_str("rgba(100,80,140,0.3)");
        self.ctx.set_line_width(1.0);
        self.ctx.begin_path();
        self.ctx.move_to(box_x + 16.0, box_y + box_h - 26.0);
        self.ctx.line_to(box_x + box_w - 16.0, box_y + box_h - 26.0);
        self.ctx.stroke();

        self.ctx.set_fill_style_str("#7784aa");
        self.ctx.set_font("11px monospace");
        self.ctx.set_text_align("center");
        self.ctx
            .fill_text(
                "Q/E switch spell in exploration  ·  Number keys cast in combat",
                self.canvas_w / 2.0,
                box_y + box_h - 12.0,
            )
            .ok();
    }

    /// Draw the character codex overlay.
    pub fn draw_codex(&self, entries: &[&crate::codex::CodexEntry]) {
        let box_x = 30.0;
        let box_y = 20.0;
        let box_w = self.canvas_w - 60.0;
        let box_h = self.canvas_h - 40.0;

        self.ctx.set_fill_style_str("rgba(0,0,0,0.88)");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

        self.ctx.set_fill_style_str("rgba(10,8,20,0.98)");
        self.ctx.fill_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("rgba(100,80,140,0.4)");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("rgba(120,90,180,0.15)");
        self.ctx.set_line_width(1.0);
        self.ctx
            .stroke_rect(box_x + 1.0, box_y + 1.0, box_w - 2.0, box_h - 2.0);

        self.ctx.set_font("bold 22px monospace");
        self.ctx.set_text_align("center");
        self.ctx.set_shadow_color("#00ccdd");
        self.ctx.set_shadow_blur(10.0);
        self.ctx.set_fill_style_str("#00ccdd");
        self.ctx
            .fill_text("─── Character Codex ───", self.canvas_w / 2.0, box_y + 28.0)
            .ok();
        self.ctx.set_shadow_blur(0.0);

        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#7784aa");
        self.ctx
            .fill_text(
                &format!(
                    "{} characters encountered  ·  C / Esc to close",
                    entries.len()
                ),
                self.canvas_w / 2.0,
                box_y + 46.0,
            )
            .ok();

        self.ctx.set_stroke_style_str("rgba(100,80,140,0.3)");
        self.ctx.set_line_width(1.0);
        self.ctx.begin_path();
        self.ctx.move_to(box_x + 16.0, box_y + 54.0);
        self.ctx.line_to(box_x + box_w - 16.0, box_y + 54.0);
        self.ctx.stroke();

        let y_start = box_y + 72.0;
        let row_h = 22.0;
        let left = box_x + 18.0;

        self.ctx.set_font("bold 11px monospace");
        self.ctx.set_text_align("left");
        self.ctx.set_fill_style_str("#7784aa");
        self.ctx.fill_text("CHAR", left, y_start).ok();
        self.ctx.fill_text("PINYIN", left + 70.0, y_start).ok();
        self.ctx.fill_text("MEANING", left + 210.0, y_start).ok();
        self.ctx.fill_text("SEEN", left + 400.0, y_start).ok();
        self.ctx.fill_text("ACCURACY", left + 450.0, y_start).ok();

        self.ctx.set_stroke_style_str("rgba(100,80,140,0.25)");
        self.ctx.begin_path();
        self.ctx.move_to(box_x + 16.0, y_start + 6.0);
        self.ctx.line_to(box_x + box_w - 16.0, y_start + 6.0);
        self.ctx.stroke();

        let max_rows = ((box_y + box_h - y_start - 40.0) / row_h) as usize;
        for (i, entry) in entries.iter().take(max_rows).enumerate() {
            let y = y_start + 10.0 + (i as f64 + 1.0) * row_h;
            let acc = entry.accuracy();

            if i % 2 == 0 {
                self.ctx.set_fill_style_str("rgba(255,255,255,0.02)");
            } else {
                self.ctx.set_fill_style_str("rgba(0,0,0,0.1)");
            }
            self.ctx
                .fill_rect(box_x + 12.0, y - 14.0, box_w - 24.0, row_h);

            self.ctx.set_fill_style_str("#ffffff");
            self.ctx.set_font("16px 'Noto Serif SC', 'SimSun', serif");
            self.ctx.fill_text(entry.hanzi, left, y).ok();

            self.ctx.set_font("12px monospace");
            self.ctx.set_fill_style_str("#cccccc");
            self.ctx.fill_text(entry.pinyin, left + 70.0, y).ok();

            self.ctx.set_fill_style_str("#aab8dd");
            let meaning = if entry.meaning.len() > 24 {
                &entry.meaning[..24]
            } else {
                entry.meaning
            };
            self.ctx.fill_text(meaning, left + 210.0, y).ok();

            self.ctx.set_fill_style_str("#cccccc");
            self.ctx
                .fill_text(&entry.times_seen.to_string(), left + 400.0, y)
                .ok();

            let bar_x = left + 450.0;
            let bar_w = 50.0;
            let bar_h = 6.0;
            let bar_y = y - 5.0;
            self.ctx.set_fill_style_str("rgba(255,255,255,0.08)");
            self.ctx.fill_rect(bar_x, bar_y, bar_w, bar_h);

            let acc_color = if acc >= 0.8 {
                "#44cc55"
            } else if acc >= 0.5 {
                "#ddbb33"
            } else {
                "#dd4444"
            };
            self.ctx.set_fill_style_str(acc_color);
            self.ctx.fill_rect(bar_x, bar_y, bar_w * acc, bar_h);

            self.ctx.set_stroke_style_str("rgba(100,80,140,0.3)");
            self.ctx.set_line_width(1.0);
            self.ctx.stroke_rect(bar_x, bar_y, bar_w, bar_h);

            self.ctx.set_font("10px monospace");
            self.ctx.set_fill_style_str(acc_color);
            self.ctx
                .fill_text(&format!("{:.0}%", acc * 100.0), bar_x + bar_w + 6.0, y)
                .ok();
        }

        if entries.len() > max_rows {
            self.ctx.set_font("11px monospace");
            self.ctx.set_text_align("center");
            self.ctx.set_fill_style_str("#556688");
            self.ctx
                .fill_text(
                    &format!("… and {} more", entries.len() - max_rows),
                    self.canvas_w / 2.0,
                    box_y + box_h - 12.0,
                )
                .ok();
        }
    }

    pub fn draw_console(&self, history: &[String], buffer: &str) {
        let ctx = &self.ctx;
        let w = self.canvas_w;
        let h = self.canvas_h / 2.0;

        ctx.set_fill_style_str("rgba(5,3,15,0.92)");
        ctx.fill_rect(0.0, 0.0, w, h);

        ctx.set_stroke_style_str("#39ff14");
        ctx.set_line_width(2.0);
        ctx.begin_path();
        ctx.move_to(0.0, h);
        ctx.line_to(w, h);
        let _ = ctx.stroke();

        ctx.set_fill_style_str("rgba(57,255,20,0.03)");
        ctx.fill_rect(0.0, 0.0, w, 24.0);
        ctx.set_stroke_style_str("rgba(57,255,20,0.2)");
        ctx.set_line_width(1.0);
        ctx.begin_path();
        ctx.move_to(0.0, 24.0);
        ctx.line_to(w, 24.0);
        let _ = ctx.stroke();

        ctx.set_fill_style_str("#39ff14");
        ctx.set_font("bold 12px monospace");
        ctx.set_text_align("left");
        let _ = ctx.fill_text("RADICAL DUNGEON CONSOLE", 10.0, 16.0);
        ctx.set_fill_style_str("#1a8a0a");
        let _ = ctx.fill_text("│", 220.0, 16.0);
        ctx.set_fill_style_str("#338822");
        ctx.set_font("11px monospace");
        let _ = ctx.fill_text("type 'help' for commands  ·  ` to close", 230.0, 16.0);

        let font_size = 14.0;
        let line_height = font_size + 4.0;
        ctx.set_font(&format!("{}px monospace", font_size));

        let input_y = h - 10.0;

        ctx.set_fill_style_str("rgba(57,255,20,0.06)");
        ctx.fill_rect(0.0, input_y - 16.0, w, 22.0);
        ctx.set_stroke_style_str("rgba(57,255,20,0.25)");
        ctx.set_line_width(1.0);
        ctx.begin_path();
        ctx.move_to(0.0, input_y - 16.0);
        ctx.line_to(w, input_y - 16.0);
        let _ = ctx.stroke();

        ctx.set_fill_style_str("#39ff14");
        ctx.set_font("bold 14px monospace");
        let _ = ctx.fill_text(&format!("> {}_", buffer), 10.0, input_y);

        ctx.set_font(&format!("{}px monospace", font_size));
        let max_lines = ((h - 50.0) / line_height) as usize;
        let start = if history.len() > max_lines {
            history.len() - max_lines
        } else {
            0
        };
        for (i, line) in history[start..].iter().enumerate() {
            let y = input_y - 22.0 - (history[start..].len() - 1 - i) as f64 * line_height;
            if y > 28.0 {
                let color = if line.starts_with("> ") {
                    "#556655"
                } else if line.starts_with("===") || line.starts_with("--- ") {
                    "#00ccdd"
                } else if line.starts_with("ERROR")
                    || line.starts_with("Unknown")
                    || line.starts_with("No ")
                {
                    "#ff5555"
                } else if line.starts_with("Set ")
                    || line.starts_with("Added")
                    || line.starts_with("Healed")
                    || line.starts_with("Restored")
                    || line.starts_with("God mode")
                    || line.starts_with("Teleported")
                    || line.starts_with("Granted")
                {
                    "#55ff77"
                } else if line.starts_with("  ") {
                    "#88aa88"
                } else {
                    "#aaccaa"
                };
                ctx.set_fill_style_str(color);
                let _ = ctx.fill_text(line, 10.0, y);
            }
        }
    }
}

fn tile_sprite_key(tile: Tile, location_label: &str) -> &'static str {
    // Map location label to sprite key prefix for wall/floor/corridor
    let loc_prefix: Option<&'static str> = match location_label {
        "Space Station" => Some("loc_space_station"),
        "Asteroid Base" => Some("loc_asteroid_base"),
        "Derelict Ship" => Some("loc_derelict_ship"),
        "Alien Ruins" => Some("loc_alien_ruins"),
        "Trading Post" => Some("loc_trading_post"),
        "Orbital Platform" => Some("loc_orbital_platform"),
        "Mining Colony" => Some("loc_mining_colony"),
        "Research Lab" => Some("loc_research_lab"),
        _ => None,
    };
    match tile {
        Tile::Bulkhead | Tile::CargoPipes | Tile::CrystalPanel => {
            if let Some(p) = loc_prefix {
                match p {
                    "loc_space_station" => "loc_space_station_wall",
                    "loc_asteroid_base" => "loc_asteroid_base_wall",
                    "loc_derelict_ship" => "loc_derelict_ship_wall",
                    "loc_alien_ruins" => "loc_alien_ruins_wall",
                    "loc_trading_post" => "loc_trading_post_wall",
                    "loc_orbital_platform" => "loc_orbital_platform_wall",
                    "loc_mining_colony" => "loc_mining_colony_wall",
                    "loc_research_lab" => "loc_research_lab_wall",
                    _ => "tile_wall",
                }
            } else {
                "tile_wall"
            }
        }
        Tile::MetalFloor | Tile::CorruptedFloor | Tile::FrozenDeck | Tile::CreditCache
        | Tile::ToxicFungus | Tile::ToxicGas | Tile::PressureSensor => {
            if let Some(p) = loc_prefix {
                match p {
                    "loc_space_station" => "loc_space_station_floor",
                    "loc_asteroid_base" => "loc_asteroid_base_floor",
                    "loc_derelict_ship" => "loc_derelict_ship_floor",
                    "loc_alien_ruins" => "loc_alien_ruins_floor",
                    "loc_trading_post" => "loc_trading_post_floor",
                    "loc_orbital_platform" => "loc_orbital_platform_floor",
                    "loc_mining_colony" => "loc_mining_colony_floor",
                    "loc_research_lab" => "loc_research_lab_floor",
                    _ => "tile_floor",
                }
            } else {
                "tile_floor"
            }
        }
        Tile::Hallway | Tile::Catwalk | Tile::DataBridge => {
            if let Some(p) = loc_prefix {
                match p {
                    "loc_space_station" => "loc_space_station_corridor",
                    "loc_asteroid_base" => "loc_asteroid_base_corridor",
                    "loc_derelict_ship" => "loc_derelict_ship_corridor",
                    "loc_alien_ruins" => "loc_alien_ruins_corridor",
                    "loc_trading_post" => "loc_trading_post_corridor",
                    "loc_orbital_platform" => "loc_orbital_platform_corridor",
                    "loc_mining_colony" => "loc_mining_colony_corridor",
                    "loc_research_lab" => "loc_research_lab_corridor",
                    _ => "tile_corridor",
                }
            } else {
                "tile_corridor"
            }
        }
        Tile::Airlock => "tile_stairs_down_scifi",
        Tile::QuantumForge => "obj_quantum_forge",
        Tile::TradeTerminal => "obj_space_shop",
        Tile::SupplyCrate | Tile::SalvageCrate | Tile::CargoCrate => "obj_cargo_crate",
        Tile::MedBayTile => "obj_medbay",
        Tile::PlasmaVent => "obj_plasma_vent",
        Tile::WarpGatePortal => "obj_warp_gate",
        Tile::DataRack => "obj_data_archive",
        Tile::OreVein => "obj_loot_container",
        Tile::NavBeacon => "obj_holo_map",
        Tile::SpecialRoom(_) => "obj_terminal",
        Tile::DamagedBulkhead => "tile_cracked_wall",
        Tile::WeakBulkhead => "tile_brittle_wall",
        Tile::LaserGrid => "tile_spikes",
        Tile::Coolant => "tile_oil",
        Tile::CoolantPool => "tile_water",
        Tile::VacuumBreach => "tile_deep_water",
        Tile::CircuitShrine => "obj_alien_artifact",
        Tile::RadicalLab => "obj_reactor_core",
        Tile::FrequencyWall => "obj_shield_generator",
        Tile::CompoundShrine => "obj_alien_artifact",
        Tile::ClassifierNode => "obj_terminal",
        Tile::DataWell => "obj_data_archive",
        Tile::MemorialNode => "obj_alien_artifact",
        Tile::TranslationTerminal => "obj_terminal",
        Tile::HoloPool => "obj_holo_map",
        Tile::DroidTutor => "obj_robot_wreck",
        Tile::CodexTerminal => "obj_terminal",
        Tile::SealedHatch => "obj_containment_cell",
        Tile::Terminal(TerminalKind::Quantum) => "obj_alien_artifact",
        Tile::Terminal(TerminalKind::Stellar) => "obj_reactor_core",
        Tile::Terminal(TerminalKind::Holographic) => "obj_holo_map",
        Tile::Terminal(TerminalKind::Tactical) => "obj_weapon_rack",
        Tile::Terminal(TerminalKind::Commerce) => "obj_space_shop",
        Tile::SecurityLock(SealKind::Thermal) => "obj_shield_generator",
        Tile::SecurityLock(SealKind::Hydraulic) => "obj_fuel_pump",
        Tile::SecurityLock(SealKind::Kinetic) => "obj_turret",
        Tile::SecurityLock(SealKind::Sonic) => "obj_containment_cell",
        Tile::InfoPanel(_) => "obj_terminal",
        Tile::Npc(0) => "npc_teacher",
        Tile::Npc(1) => "npc_monk",
        Tile::Npc(2) => "npc_merchant",
        Tile::Npc(_) => "npc_guard",
        Tile::Trap(_) => "tile_floor",
    }
}

fn boss_sprite_key(kind: BossKind) -> &'static str {
    match kind {
        BossKind::PirateCaptain => "boss_pirate_captain",
        BossKind::HiveQueen => "boss_hive_queen",
        BossKind::RogueAICore => "boss_rogue_ai_core",
        BossKind::VoidEntity => "boss_void_entity",
        BossKind::AncientGuardian => "boss_ancient_guardian",
        BossKind::DriftLeviathan => "boss_drift_leviathan",
    }
}

fn item_sprite_key(item: &Item) -> &'static str {
    match item.kind() {
        ItemKind::MedHypo => "item_health_potion",
        ItemKind::ToxinGrenade => "item_poison_flask",
        ItemKind::ScannerPulse => "item_reveal_scroll",
        ItemKind::PersonalTeleporter => "item_teleport_scroll",
        ItemKind::StimPack => "item_haste_potion",
        ItemKind::EMPGrenade => "item_stun_bomb",
        ItemKind::RationPack => "item_rice_ball",
        ItemKind::FocusStim => "item_meditation_incense",
        ItemKind::SynthAle => "item_ancestral_wine",
        ItemKind::HoloDecoy => "item_smoke_screen",
        ItemKind::PlasmaBurst => "item_fire_cracker",
        ItemKind::NanoShield => "item_iron_skin_elixir",
        ItemKind::NeuralBoost => "item_clarity_tea",
        ItemKind::CreditChip => "item_gold_ingot",
        ItemKind::ShockModule => "item_thunder_talisman",
        ItemKind::BiogelPatch => "item_jade_salve",
        ItemKind::VenomDart => "item_serpent_fang",
        ItemKind::DeflectorDrone => "item_warding_charm",
        ItemKind::NaniteSwarm => "item_ink_bomb",
        ItemKind::Revitalizer => "item_phoenix_plume",
        ItemKind::ReflectorPlate => "item_mirror_shard",
        ItemKind::CryoGrenade => "item_frost_vial",
        ItemKind::CloakingDevice => "item_shadow_cloak",
        ItemKind::PlasmaShield => "item_dragon_scale",
        ItemKind::SignalJammer => "item_bamboo_flute",
        ItemKind::NavComputer => "item_jade_compass",
        ItemKind::GrappleLine => "item_silk_rope",
        ItemKind::OmniGel => "item_lotus_elixir",
        ItemKind::SonicEmitter => "item_thunder_drum",
        ItemKind::CircuitInk => "item_cinnabar_ink",
        ItemKind::DataCore => "item_ancestor_token",
        ItemKind::ThrusterPack => "item_wind_fan",
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
        radical::SpellEffect::Slow(_) => "spell_stun",
        radical::SpellEffect::Teleport => "spell_reveal",
        radical::SpellEffect::Poison(_, _) => "spell_drain",
        radical::SpellEffect::FocusRestore(_) => "spell_heal",
        radical::SpellEffect::ArmorBreak => "spell_strike",
        radical::SpellEffect::Dash(_) => "spell_reveal",
        radical::SpellEffect::Pierce(_) => "spell_strike",
        radical::SpellEffect::PullToward => "spell_reveal",
        radical::SpellEffect::KnockBack(_) => "spell_strike",
        radical::SpellEffect::Thorns(_) => "spell_shield",
        radical::SpellEffect::Cone(_) => "spell_fire",
        radical::SpellEffect::Wall(_) => "spell_shield",
        radical::SpellEffect::OilSlick => "spell_drain",
        radical::SpellEffect::FreezeGround(_) => "spell_stun",
        radical::SpellEffect::Ignite => "spell_fire",
        radical::SpellEffect::PlantGrowth => "spell_heal",
        radical::SpellEffect::Earthquake(_) => "spell_strike",
        radical::SpellEffect::Sanctify(_) => "spell_heal",
        radical::SpellEffect::FloodWave(_) => "spell_stun",
        radical::SpellEffect::SummonBoulder => "spell_shield",
    }
}

fn spell_school_color(effect: &radical::SpellEffect) -> &'static str {
    match effect {
        radical::SpellEffect::FireAoe(_) | radical::SpellEffect::Cone(_) => "#ff6633",
        radical::SpellEffect::Heal(_) | radical::SpellEffect::FocusRestore(_) => "#44dd66",
        radical::SpellEffect::Reveal
        | radical::SpellEffect::Teleport
        | radical::SpellEffect::Dash(_)
        | radical::SpellEffect::PullToward => "#66bbff",
        radical::SpellEffect::Shield
        | radical::SpellEffect::Wall(_)
        | radical::SpellEffect::Thorns(_) => "#88aaff",
        radical::SpellEffect::StrongHit(_)
        | radical::SpellEffect::ArmorBreak
        | radical::SpellEffect::Pierce(_)
        | radical::SpellEffect::KnockBack(_) => "#ff9944",
        radical::SpellEffect::Drain(_) | radical::SpellEffect::Poison(_, _) => "#aa66dd",
        radical::SpellEffect::Stun | radical::SpellEffect::Slow(_) => "#66ddff",
        radical::SpellEffect::Pacify => "#ffdd66",
        radical::SpellEffect::OilSlick => "#8a7a4a",
        radical::SpellEffect::FreezeGround(_) | radical::SpellEffect::FloodWave(_) => "#66ddff",
        radical::SpellEffect::Ignite => "#ff6633",
        radical::SpellEffect::PlantGrowth | radical::SpellEffect::Sanctify(_) => "#44dd66",
        radical::SpellEffect::Earthquake(_) => "#ff9944",
        radical::SpellEffect::SummonBoulder => "#88aaff",
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
    state: ItemState,
) -> String {
    let prefix = match state {
        ItemState::Cursed => "💀 ",
        ItemState::Blessed => "✨ ",
        ItemState::Normal => "",
    };
    match (equipment, enchantment) {
        (Some(equipment), Some(enchantment)) => {
            format!("{}{} +{}", prefix, equipment.name, enchantment)
        }
        (Some(equipment), None) => format!("{}{}", prefix, equipment.name),
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
            Tile::NavBeacon | Tile::SpecialRoom(_) | Tile::SalvageCrate => TilePalette {
                fill: "#444",
                accent: None,
                glyph: None,
                glyph_color: "#fff",
            },
            Tile::Bulkhead => TilePalette {
                fill: COL_WALL,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::DamagedBulkhead => TilePalette {
                fill: "#47324f",
                accent: Some("#d89c74"),
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::WeakBulkhead => TilePalette {
                fill: "#5b473a",
                accent: Some("#f2d29e"),
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::MetalFloor => TilePalette {
                fill: COL_FLOOR,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Hallway => TilePalette {
                fill: COL_CORRIDOR,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Airlock => TilePalette {
                fill: COL_STAIRS,
                accent: Some("#d7e7ff"),
                glyph: Some("▼"),
                glyph_color: "#ffffff",
            },
            Tile::QuantumForge => TilePalette {
                fill: COL_FORGE,
                accent: Some("#ffd1aa"),
                glyph: Some("⚒"),
                glyph_color: "#ffffff",
            },
            Tile::TradeTerminal => TilePalette {
                fill: COL_SHOP,
                accent: Some("#bfffd4"),
                glyph: Some("$"),
                glyph_color: "#ffffff",
            },
            Tile::SupplyCrate => TilePalette {
                fill: COL_CHEST,
                accent: Some("#ffe29e"),
                glyph: Some("◆"),
                glyph_color: "#fff7dc",
            },
            Tile::LaserGrid => TilePalette {
                fill: "#7e434a",
                accent: Some("#d9a0a0"),
                glyph: Some("^"),
                glyph_color: "#fff1f1",
            },
            Tile::Coolant => TilePalette {
                fill: "#4f3a1c",
                accent: Some("#e7c56d"),
                glyph: Some("~"),
                glyph_color: "#ffdd88",
            },
            Tile::CoolantPool => TilePalette {
                fill: "#4466cc",
                accent: Some("#9fc4ff"),
                glyph: Some("≈"),
                glyph_color: "#e5efff",
            },
            Tile::VacuumBreach => TilePalette {
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
            Tile::CircuitShrine => TilePalette {
                fill: "#7d5d2a",
                accent: Some("#ffd07a"),
                glyph: Some("🔔"),
                glyph_color: "#fff8e2",
            },
            // Removed StrokeShrine, mapped to RadicalLab
            Tile::FrequencyWall => TilePalette {
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
            Tile::ClassifierNode => TilePalette {
                fill: "#3a2a1a",
                accent: Some("#ddaa44"),
                glyph: Some("量"),
                glyph_color: "#ddaa44",
            },
            Tile::DataWell => TilePalette {
                fill: "#1a1a2d",
                accent: Some("#9999ee"),
                glyph: Some("墨"),
                glyph_color: "#9999ee",
            },
            Tile::MemorialNode => TilePalette {
                fill: "#2d1a1a",
                accent: Some("#ee9966"),
                glyph: Some("祖"),
                glyph_color: "#ee9966",
            },
            Tile::TranslationTerminal => TilePalette {
                fill: "#1a2d2d",
                accent: Some("#66cccc"),
                glyph: Some("译"),
                glyph_color: "#66cccc",
            },
            Tile::RadicalLab => TilePalette {
                fill: "#1a2d1a",
                accent: Some("#88ee66"),
                glyph: Some("部"),
                glyph_color: "#88ee66",
            },
            Tile::HoloPool => TilePalette {
                fill: "#1a1a3a",
                accent: Some("#aaaaff"),
                glyph: Some("鏡"),
                glyph_color: "#aaaaff",
            },
            Tile::DroidTutor => TilePalette {
                fill: "#2d2d1a",
                accent: Some("#cccc66"),
                glyph: Some("石"),
                glyph_color: "#cccc66",
            },
            Tile::CodexTerminal => TilePalette {
                fill: "#2a1a3a",
                accent: Some("#dd99ff"),
                glyph: Some("典"),
                glyph_color: "#dd99ff",
            },
            Tile::DataBridge => TilePalette {
                fill: "#1a1a2d",
                accent: Some("#66aaff"),
                glyph: Some("桥"),
                glyph_color: "#66aaff",
            },
            Tile::SealedHatch => TilePalette {
                fill: "#2d1a1a",
                accent: Some("#ff6644"),
                glyph: Some("锁"),
                glyph_color: "#ff6644",
            },
            Tile::CorruptedFloor => TilePalette {
                fill: "#1a1a1a",
                accent: None,
                glyph: None,
                glyph_color: "#aa44aa",
            },
            Tile::Terminal(kind) => TilePalette {
                fill: altar_fill(kind),
                accent: Some(kind.color()),
                glyph: Some(kind.icon()),
                glyph_color: kind.color(),
            },
            Tile::SecurityLock(kind) => TilePalette {
                fill: seal_fill(kind),
                accent: Some(kind.color()),
                glyph: Some(kind.icon()),
                glyph_color: kind.color(),
            },
            Tile::InfoPanel(_) => TilePalette {
                fill: "#8a6b47",
                accent: Some("#d7b07b"),
                glyph: Some("?"),
                glyph_color: "#ffffff",
            },
            Tile::Catwalk => TilePalette {
                fill: "#8b4513",
                accent: Some("#a0522d"),
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Trap(_) => TilePalette {
                fill: COL_FLOOR,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::OreVein => TilePalette {
                fill: "#5a4a1e",
                accent: Some("#ffd700"),
                glyph: Some("矿"),
                glyph_color: "#ffd700",
            },
            Tile::PlasmaVent => TilePalette {
                fill: "#8b2500",
                accent: Some("#ff4500"),
                glyph: Some("~"),
                glyph_color: "#ff6633",
            },
            Tile::FrozenDeck => TilePalette {
                fill: "#a8d8ea",
                accent: Some("#e0f0ff"),
                glyph: Some("·"),
                glyph_color: "#e0f0ff",
            },
            Tile::CargoPipes => TilePalette {
                fill: "#2d5a27",
                accent: Some("#6abf4b"),
                glyph: Some("‖"),
                glyph_color: "#88dd66",
            },
            Tile::ToxicFungus => TilePalette {
                fill: "#4a2d5a",
                accent: Some("#bb77dd"),
                glyph: Some("♠"),
                glyph_color: "#cc88ee",
            },
            Tile::ToxicGas => TilePalette {
                fill: "#2a4a2a",
                accent: Some("#77dd44"),
                glyph: Some("░"),
                glyph_color: "#88ee55",
            },
            Tile::DataRack => TilePalette {
                fill: "#5a3a1e",
                accent: Some("#c49a6c"),
                glyph: Some("书"),
                glyph_color: "#ddb888",
            },
            Tile::PressureSensor => TilePalette {
                fill: "#555555",
                accent: Some("#999999"),
                glyph: Some("◫"),
                glyph_color: "#bbbbbb",
            },
            Tile::CargoCrate => TilePalette {
                fill: "#666655",
                accent: Some("#998877"),
                glyph: Some("●"),
                glyph_color: "#bbaa99",
            },
            Tile::CrystalPanel => TilePalette {
                fill: "#3a3a6a",
                accent: Some("#aaaaff"),
                glyph: Some("◇"),
                glyph_color: "#ccccff",
            },
            Tile::WarpGatePortal => TilePalette {
                fill: "#4a1a3a",
                accent: Some("#ff44aa"),
                glyph: Some("龙"),
                glyph_color: "#ff66cc",
            },
            Tile::MedBayTile => TilePalette {
                fill: "#2a5a5a",
                accent: Some("#66ffdd"),
                glyph: Some("泉"),
                glyph_color: "#88ffee",
            },
            Tile::CreditCache => TilePalette {
                fill: "#5a4a1e",
                accent: Some("#ffd700"),
                glyph: Some("¥"),
                glyph_color: "#ffdd44",
            },
        }
    } else {
        match tile {
            Tile::NavBeacon | Tile::SpecialRoom(_) | Tile::SalvageCrate => TilePalette {
                fill: "#222",
                accent: None,
                glyph: None,
                glyph_color: "#555",
            },
            Tile::Bulkhead => TilePalette {
                fill: COL_WALL_REVEALED,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::DamagedBulkhead => TilePalette {
                fill: "#2d2338",
                accent: Some("#805d48"),
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::WeakBulkhead => TilePalette {
                fill: "#342c26",
                accent: Some("#7d6a57"),
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::MetalFloor => TilePalette {
                fill: COL_FLOOR_REVEALED,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Hallway => TilePalette {
                fill: COL_CORRIDOR_REVEALED,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Airlock => TilePalette {
                fill: "#243857",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::QuantumForge => TilePalette {
                fill: "#4b2b1d",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::TradeTerminal => TilePalette {
                fill: "#1e4a33",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::SupplyCrate => TilePalette {
                fill: "#5a441b",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::LaserGrid => TilePalette {
                fill: "#4a2d32",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Coolant => TilePalette {
                fill: "#3a2f1b",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CoolantPool => TilePalette {
                fill: "#213f6b",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::VacuumBreach => TilePalette {
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
            Tile::CircuitShrine => TilePalette {
                fill: "#4f3d20",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::RadicalLab => TilePalette {
                fill: "#111822",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::FrequencyWall => TilePalette {
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
            Tile::ClassifierNode => TilePalette {
                fill: "#221a11",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::DataWell => TilePalette {
                fill: "#111118",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::MemorialNode => TilePalette {
                fill: "#181111",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::TranslationTerminal => TilePalette {
                fill: "#111818",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::HoloPool => TilePalette {
                fill: "#11111f",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::DroidTutor => TilePalette {
                fill: "#181811",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CodexTerminal => TilePalette {
                fill: "#16101e",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::DataBridge => TilePalette {
                fill: "#101018",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::SealedHatch => TilePalette {
                fill: "#1a1010",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CorruptedFloor => TilePalette {
                fill: "#111111",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Terminal(kind) => TilePalette {
                fill: altar_revealed_fill(kind),
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::SecurityLock(kind) => TilePalette {
                fill: seal_revealed_fill(kind),
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::InfoPanel(_) => TilePalette {
                fill: "#4b3a26",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Catwalk => TilePalette {
                fill: "#5c4033",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Trap(_) => TilePalette {
                fill: COL_FLOOR_REVEALED,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::OreVein => TilePalette {
                fill: "#3a3214",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::PlasmaVent => TilePalette {
                fill: "#5a1a00",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::FrozenDeck => TilePalette {
                fill: "#6a8a9a",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CargoPipes => TilePalette {
                fill: "#1e3a1b",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::ToxicFungus => TilePalette {
                fill: "#2d1a3a",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::ToxicGas => TilePalette {
                fill: "#1a2d1a",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::DataRack => TilePalette {
                fill: "#3a2614",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::PressureSensor => TilePalette {
                fill: "#333333",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CargoCrate => TilePalette {
                fill: "#3a3a33",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CrystalPanel => TilePalette {
                fill: "#222244",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::WarpGatePortal => TilePalette {
                fill: "#2d1024",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::MedBayTile => TilePalette {
                fill: "#1a3a3a",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CreditCache => TilePalette {
                fill: "#3a3214",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
        }
    }
}

fn tile_plate_fill(tile: Tile) -> Option<&'static str> {
    match tile {
        Tile::Airlock => Some("rgba(255,255,255,0.14)"),
        Tile::QuantumForge => Some("rgba(255,226,194,0.16)"),
        Tile::TradeTerminal => Some("rgba(207,255,224,0.15)"),
        Tile::SupplyCrate => Some("rgba(255,231,173,0.16)"),
        Tile::Npc(_) => Some("rgba(225,245,255,0.12)"),
        Tile::CircuitShrine => Some("rgba(255,224,156,0.16)"),
        Tile::RadicalLab => Some("rgba(136,204,255,0.16)"),
        Tile::FrequencyWall => Some("rgba(221,102,68,0.16)"),
        Tile::CompoundShrine => Some("rgba(102,221,136,0.16)"),
        Tile::ClassifierNode => Some("rgba(221,170,68,0.16)"),
        Tile::DataWell => Some("rgba(153,153,238,0.16)"),
        Tile::MemorialNode => Some("rgba(238,153,102,0.16)"),
        Tile::TranslationTerminal => Some("rgba(102,204,204,0.16)"),
        Tile::HoloPool => Some("rgba(170,170,255,0.16)"),
        Tile::DroidTutor => Some("rgba(204,204,102,0.16)"),
        Tile::CodexTerminal => Some("rgba(221,153,255,0.16)"),
        Tile::DataBridge => Some("rgba(102,170,255,0.16)"),
        Tile::SealedHatch => Some("rgba(255,102,68,0.16)"),
        Tile::Terminal(kind) => Some(altar_plate_fill(kind)),
        Tile::SecurityLock(kind) => Some(seal_plate_fill(kind)),
        Tile::InfoPanel(_) => Some("rgba(255,236,200,0.10)"),
        Tile::OreVein => Some("rgba(255,215,0,0.16)"),
        Tile::PlasmaVent => Some("rgba(255,69,0,0.18)"),
        Tile::DataRack => Some("rgba(196,154,108,0.14)"),
        Tile::WarpGatePortal => Some("rgba(255,68,170,0.18)"),
        Tile::MedBayTile => Some("rgba(102,255,221,0.16)"),
        Tile::CreditCache => Some("rgba(255,215,0,0.16)"),
        Tile::CrystalPanel => Some("rgba(170,170,255,0.14)"),
        _ => None,
    }
}

fn altar_fill(kind: TerminalKind) -> &'static str {
    match kind {
        TerminalKind::Quantum => "#30563f",
        TerminalKind::Stellar => "#334d74",
        TerminalKind::Holographic => "#5a456e",
        TerminalKind::Tactical => "#4a4a4a",
        TerminalKind::Commerce => "#665522",
    }
}

fn altar_revealed_fill(kind: TerminalKind) -> &'static str {
    match kind {
        TerminalKind::Quantum => "#214231",
        TerminalKind::Stellar => "#243b56",
        TerminalKind::Holographic => "#443255",
        TerminalKind::Tactical => "#333333",
        TerminalKind::Commerce => "#443a1a",
    }
}

fn altar_plate_fill(kind: TerminalKind) -> &'static str {
    match kind {
        TerminalKind::Quantum => "rgba(102,221,153,0.14)",
        TerminalKind::Stellar => "rgba(136,204,255,0.14)",
        TerminalKind::Holographic => "rgba(221,184,255,0.14)",
        TerminalKind::Tactical => "rgba(200,200,200,0.14)",
        TerminalKind::Commerce => "rgba(255,215,0,0.14)",
    }
}

fn seal_fill(kind: SealKind) -> &'static str {
    match kind {
        SealKind::Thermal => "#6a3529",
        SealKind::Hydraulic => "#264d79",
        SealKind::Kinetic => "#5f3144",
        SealKind::Sonic => "#4f3a68",
    }
}

fn seal_revealed_fill(kind: SealKind) -> &'static str {
    match kind {
        SealKind::Thermal => "#44251d",
        SealKind::Hydraulic => "#1b3652",
        SealKind::Kinetic => "#412230",
        SealKind::Sonic => "#352646",
    }
}

fn seal_plate_fill(kind: SealKind) -> &'static str {
    match kind {
        SealKind::Thermal => "rgba(255,155,115,0.16)",
        SealKind::Hydraulic => "rgba(144,201,255,0.16)",
        SealKind::Kinetic => "rgba(255,158,184,0.14)",
        SealKind::Sonic => "rgba(212,164,255,0.16)",
    }
}

fn tile_glyph_font(tile: Tile) -> &'static str {
    match tile {
        Tile::SecurityLock(_) => "bold 14px 'Noto Serif SC', 'SimSun', serif",
        Tile::SalvageCrate | Tile::LaserGrid | Tile::Coolant | Tile::CoolantPool | Tile::VacuumBreach => "15px monospace",
        _ => "16px monospace",
    }
}

fn tile_glyph_y(tile: Tile, screen_y: f64, anim_t: f64, tx: i32, ty: i32) -> f64 {
    let base = screen_y + TILE_SIZE * 0.75;
    match tile {
        Tile::CoolantPool | Tile::VacuumBreach => {
            base + (anim_t * 3.5 + tx as f64 * 0.6 + ty as f64 * 0.35).sin() * 1.4
        }
        Tile::Coolant => base + (anim_t * 2.0 + tx as f64 * 0.4).sin() * 0.6,
        Tile::CircuitShrine => base + (anim_t * 2.5).sin() * 0.9,
        Tile::RadicalLab => base + (anim_t * 2.7 + tx as f64 * 0.3).sin() * 0.8,
        Tile::FrequencyWall => base + (anim_t * 3.0 + ty as f64 * 0.3).sin() * 0.7,
        Tile::CompoundShrine => base + (anim_t * 2.4 + tx as f64 * 0.2).sin() * 0.8,
        Tile::ClassifierNode => base + (anim_t * 2.6 + ty as f64 * 0.25).sin() * 0.7,
        Tile::DataWell => base + (anim_t * 2.3 + tx as f64 * 0.25).sin() * 0.7,
        Tile::MemorialNode => base + (anim_t * 2.9 + ty as f64 * 0.35).sin() * 0.8,
        Tile::TranslationTerminal => base + (anim_t * 2.5 + tx as f64 * 0.3).sin() * 0.75,
        Tile::HoloPool => base + (anim_t * 3.2 + ty as f64 * 0.4).sin() * 1.0,
        Tile::DroidTutor => base + (anim_t * 2.0 + tx as f64 * 0.15).sin() * 0.6,
        Tile::CodexTerminal => base + (anim_t * 2.4 + tx as f64 * 0.2).sin() * 0.8,
        Tile::DataBridge => base + (anim_t * 2.6 + ty as f64 * 0.3).sin() * 0.7,
        Tile::SealedHatch => base + (anim_t * 1.8 + tx as f64 * 0.1).sin() * 0.5,
        Tile::Terminal(_) => base + (anim_t * 2.8 + ty as f64 * 0.4).sin() * 0.8,
        Tile::SecurityLock(_) => base + (anim_t * 3.1 + tx as f64 * 0.35 + ty as f64 * 0.2).sin() * 0.7,
        Tile::Airlock => base + (anim_t * 1.8).sin() * 0.4,
        Tile::PlasmaVent => base + (anim_t * 3.0 + tx as f64 * 0.5 + ty as f64 * 0.3).sin() * 1.2,
        Tile::MedBayTile => base + (anim_t * 2.5 + tx as f64 * 0.3).sin() * 0.9,
        Tile::WarpGatePortal => base + (anim_t * 3.5 + ty as f64 * 0.4).sin() * 1.1,
        Tile::CreditCache => base + (anim_t * 1.5).sin() * 0.3,
        Tile::ToxicFungus => base + (anim_t * 2.0 + tx as f64 * 0.2).sin() * 0.5,
        Tile::ToxicGas => base + (anim_t * 2.8 + tx as f64 * 0.4 + ty as f64 * 0.3).sin() * 0.8,
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
    use crate::world::{TerminalKind, Tile};

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
        let stairs = tile_palette(Tile::Airlock, true);

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
        let revealed_chest = tile_palette(Tile::SupplyCrate, false);
        let revealed_floor = tile_palette(Tile::MetalFloor, false);
        let revealed_altar = tile_palette(Tile::Terminal(TerminalKind::Quantum), false);

        assert_eq!(revealed_chest.fill, "#5a441b");
        assert_ne!(revealed_chest.fill, revealed_floor.fill);
        assert_eq!(revealed_altar.fill, "#214231");
    }
}


impl Renderer {
    #[allow(dead_code)]
    pub fn draw_starmap(
        &self,
        sector_map: &SectorMap,
        anim_t: f64,
        _settings: &GameSettings,
        selected_target: Option<usize>,
    ) {
        // Clear background
        self.ctx.set_fill_style_str("#000000");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

        // ========== 1. RICH ANIMATED STARFIELD BACKGROUND ==========
        
        // Draw nebula patches (large semi-transparent colored circles)
        let nebulae = [
            (0.2, 0.3, 180.0, "rgba(88, 44, 120, 0.15)"), // Purple
            (0.7, 0.2, 220.0, "rgba(44, 88, 120, 0.12)"), // Blue
            (0.5, 0.7, 200.0, "rgba(120, 44, 88, 0.10)"), // Red
            (0.1, 0.8, 150.0, "rgba(44, 120, 88, 0.08)"), // Teal
            (0.9, 0.6, 170.0, "rgba(120, 88, 44, 0.11)"), // Orange
        ];
        
        for (nx, ny, radius, color) in nebulae {
            let x = nx * self.canvas_w;
            let y = ny * self.canvas_h;
            self.ctx.set_fill_style_str(color);
            self.ctx.begin_path();
            self.ctx.arc(x, y, radius, 0.0, std::f64::consts::TAU).ok();
            self.ctx.fill();
        }
        
        // Draw 250 background stars with varied sizes, brightness, and twinkling
        for i in 0..250 {
            let seed = (i * 1234567) as u32;
            let x = ((seed.wrapping_mul(2654435761)) % (self.canvas_w as u32)) as f64;
            let y = ((seed.wrapping_mul(987654321)) % (self.canvas_h as u32)) as f64;
            
            // Size: 1-3px
            let size_seed = (seed >> 8) & 3;
            let size = 1.0 + size_seed as f64 * 0.5;
            
            // Color variety
            let color_seed = (seed >> 12) & 15;
            let color = match color_seed {
                0..=1 => "#8888ff", // Blue-white
                2..=3 => "#ffddaa", // Yellow
                4 => "#ffaa88",     // Red-orange
                _ => "#ffffff",     // White (most common)
            };
            
            // Twinkling: some stars pulse
            let twinkle_seed = (seed >> 16) & 7;
            let alpha = if twinkle_seed < 2 {
                // Twinkling stars
                let phase = anim_t * (2.0 + twinkle_seed as f64 * 0.5);
                0.3 + 0.7 * ((phase + i as f64 * 0.5).sin() * 0.5 + 0.5)
            } else {
                // Static stars
                0.6 + (seed & 0xFF) as f64 / 255.0 * 0.4
            };
            
            self.ctx.set_global_alpha(alpha);
            self.ctx.set_fill_style_str(color);
            self.ctx.fill_rect(x, y, size, size);
        }
        self.ctx.set_global_alpha(1.0);

        // ========== 2. SECTOR PROGRESS BAR (top) ==========
        if let Some(sector) = sector_map.sectors.get(sector_map.current_sector) {
            // Progress bar background
            self.ctx.set_fill_style_str("rgba(20, 20, 40, 0.8)");
            self.ctx.fill_rect(100.0, 5.0, self.canvas_w - 200.0, 20.0);
            
            // Find start, boss, exit systems
            let mut start_x = 0.0;
            let mut boss_x = 1.0;
            let mut exit_x = 1.0;
            let mut current_x = 0.5;
            
            for sys in &sector.systems {
                if sys.id == 0 { start_x = sys.x; }
                if sys.id == sector_map.current_system { current_x = sys.x; }
                // Assume boss is marked by difficulty > 8 or event_id exists
                if sys.difficulty >= 8 || sys.event_id.is_some() { 
                    if sys.x > boss_x { boss_x = sys.x; }
                }
                // Exit is rightmost
                if sys.x > exit_x { exit_x = sys.x; }
            }
            
            // Progress fill (how far player has gone)
            let progress = (current_x - start_x) / (exit_x - start_x).max(0.01);
            let bar_width = (self.canvas_w - 200.0) * progress;
            self.ctx.set_fill_style_str("rgba(0, 200, 220, 0.5)");
            self.ctx.fill_rect(100.0, 5.0, bar_width, 20.0);
            
            // Labels
            self.ctx.set_font("10px monospace");
            self.ctx.set_fill_style_str("#00dd88");
            self.ctx.fill_text("START", 105.0, 18.0).ok();
            
            self.ctx.set_fill_style_str("#ff4444");
            let boss_screen_x = 100.0 + (boss_x - start_x) / (exit_x - start_x).max(0.01) * (self.canvas_w - 200.0);
            self.ctx.fill_text("⚠ BOSS", boss_screen_x - 30.0, 18.0).ok();
            
            self.ctx.set_fill_style_str("#ffdd44");
            self.ctx.fill_text("EXIT →", self.canvas_w - 145.0, 18.0).ok();
        }

        // ========== 3. DRAW SECTOR MAP ==========
        if let Some(sector) = sector_map.sectors.get(sector_map.current_sector) {
            let cx = self.canvas_w / 2.0;
            let cy = self.canvas_h / 2.0 + 20.0; // Offset down a bit for progress bar
            let scale = 300.0;

            // ========== 3a. DRAW CONNECTIONS ==========
            for system in &sector.systems {
                let sx = cx + (system.x - 0.5) * scale * 2.0;
                let sy = cy + (system.y - 0.5) * scale * 2.0;

                for &target_id in &system.connections {
                    if let Some(target) = sector.systems.iter().find(|s| s.id == target_id) {
                        let tx = cx + (target.x - 0.5) * scale * 2.0;
                        let ty = cy + (target.y - 0.5) * scale * 2.0;
                        
                        // Determine if this is main path (x increases) or branch
                        let is_main_path = target.x > system.x + 0.05;
                        
                        // Animated line to selected target
                        let is_selected_connection = Some(target_id) == selected_target 
                            && system.id == sector_map.current_system;
                        
                        if is_selected_connection {
                            // Pulsing animated line
                            let pulse = (anim_t * 4.0).sin() * 0.5 + 0.5;
                            let alpha = 0.5 + pulse * 0.5;
                            self.ctx.set_global_alpha(alpha);
                            self.ctx.set_stroke_style_str("#00ffff");
                            self.ctx.set_line_width(3.0);
                        } else if is_main_path {
                            self.ctx.set_stroke_style_str("#556688");
                            self.ctx.set_line_width(2.0);
                        } else {
                            self.ctx.set_stroke_style_str("#333355");
                            self.ctx.set_line_width(1.0);
                        }
                        
                        self.ctx.begin_path();
                        self.ctx.move_to(sx, sy);
                        self.ctx.line_to(tx, ty);
                        self.ctx.stroke();
                        self.ctx.set_global_alpha(1.0);
                        
                        // Draw fuel cost on connection
                        if system.id == sector_map.current_system {
                            let dx = target.x - system.x;
                            let dy = target.y - system.y;
                            let fuel_cost = ((dx * dx + dy * dy).sqrt() * 10.0).ceil() as i32;
                            
                            let mid_x = (sx + tx) / 2.0;
                            let mid_y = (sy + ty) / 2.0;
                            
                            self.ctx.set_font("10px monospace");
                            self.ctx.set_fill_style_str("rgba(255, 220, 100, 0.9)");
                            self.ctx.fill_text(&format!("{}", fuel_cost), mid_x + 5.0, mid_y - 5.0).ok();
                        }
                    }
                }
            }

            // ========== 3b. DRAW SYSTEMS ==========
            for system in &sector.systems {
                let sx = cx + (system.x - 0.5) * scale * 2.0;
                let sy = cy + (system.y - 0.5) * scale * 2.0;
                let is_current = system.id == sector_map.current_system;
                let is_boss = system.difficulty >= 8 || system.event_id.is_some();
                let is_exit = system.x >= 0.95; // Rightmost systems

                // Determine color by type (if not visited) or override for current/visited
                let (color, glow_color) = if is_current {
                    ("#00ffff", Some("#00ffff"))
                } else if is_boss {
                    ("#ff4444", Some("#ff4444"))
                } else if is_exit {
                    ("#ffdd44", Some("#ffdd44"))
                } else if system.visited {
                    ("#44aa88", None)
                } else {
                    match system.location_type {
                        LocationType::SpaceStation => ("#ffdd44", None),
                        LocationType::AsteroidBase => ("#cc7733", None),
                        LocationType::DerelictShip => ("#8844cc", None),
                        LocationType::AlienRuins => ("#44ccaa", None),
                        LocationType::TradingPost => ("#44dd44", None),
                        LocationType::OrbitalPlatform => ("#4488ff", None),
                        LocationType::MiningColony => ("#cc8844", None),
                        LocationType::ResearchLab => ("#ff88cc", None),
                    }
                };
                
                // Pulsing glow for current/boss/exit systems
                if let Some(glow) = glow_color {
                    let pulse = (anim_t * 3.0).sin() * 0.3 + 0.7;
                    self.ctx.set_global_alpha(pulse * 0.3);
                    self.ctx.set_fill_style_str(glow);
                    self.ctx.begin_path();
                    self.ctx.arc(sx, sy, 20.0, 0.0, std::f64::consts::TAU).ok();
                    self.ctx.fill();
                    self.ctx.set_global_alpha(1.0);
                }
                
                // Draw system icon by type
                self.ctx.set_fill_style_str(color);
                self.ctx.set_stroke_style_str(color);
                self.ctx.set_line_width(2.0);
                
                match system.location_type {
                    LocationType::SpaceStation => {
                        // Square with inner dot
                        self.ctx.stroke_rect(sx - 7.0, sy - 7.0, 14.0, 14.0);
                        self.ctx.begin_path();
                        self.ctx.arc(sx, sy, 3.0, 0.0, std::f64::consts::TAU).ok();
                        self.ctx.fill();
                    }
                    LocationType::AsteroidBase => {
                        // Triangle
                        self.ctx.begin_path();
                        self.ctx.move_to(sx, sy - 8.0);
                        self.ctx.line_to(sx - 7.0, sy + 6.0);
                        self.ctx.line_to(sx + 7.0, sy + 6.0);
                        self.ctx.line_to(sx, sy - 8.0);
                        self.ctx.fill();
                    }
                    LocationType::DerelictShip => {
                        // X shape
                        self.ctx.begin_path();
                        self.ctx.move_to(sx - 7.0, sy - 7.0);
                        self.ctx.line_to(sx + 7.0, sy + 7.0);
                        self.ctx.move_to(sx + 7.0, sy - 7.0);
                        self.ctx.line_to(sx - 7.0, sy + 7.0);
                        self.ctx.stroke();
                    }
                    LocationType::AlienRuins => {
                        // Diamond
                        self.ctx.begin_path();
                        self.ctx.move_to(sx, sy - 8.0);
                        self.ctx.line_to(sx + 8.0, sy);
                        self.ctx.line_to(sx, sy + 8.0);
                        self.ctx.line_to(sx - 8.0, sy);
                        self.ctx.line_to(sx, sy - 8.0);
                        self.ctx.fill();
                    }
                    LocationType::TradingPost => {
                        // Large circle with ring
                        self.ctx.begin_path();
                        self.ctx.arc(sx, sy, 6.0, 0.0, std::f64::consts::TAU).ok();
                        self.ctx.fill();
                        self.ctx.begin_path();
                        self.ctx.arc(sx, sy, 9.0, 0.0, std::f64::consts::TAU).ok();
                        self.ctx.stroke();
                    }
                    LocationType::OrbitalPlatform => {
                        // Rectangle/dash
                        self.ctx.fill_rect(sx - 10.0, sy - 3.0, 20.0, 6.0);
                    }
                    LocationType::MiningColony => {
                        // Pentagon
                        self.ctx.begin_path();
                        for i in 0..5 {
                            let angle = (i as f64 / 5.0) * std::f64::consts::TAU - std::f64::consts::PI / 2.0;
                            let px = sx + 7.0 * angle.cos();
                            let py = sy + 7.0 * angle.sin();
                            if i == 0 {
                                self.ctx.move_to(px, py);
                            } else {
                                self.ctx.line_to(px, py);
                            }
                        }
                        self.ctx.line_to(sx + 7.0 * (-std::f64::consts::PI / 2.0).cos(), 
                                        sy + 7.0 * (-std::f64::consts::PI / 2.0).sin());
                        self.ctx.fill();
                    }
                    LocationType::ResearchLab => {
                        // 4-pointed star
                        self.ctx.begin_path();
                        for i in 0..8 {
                            let angle = (i as f64 / 8.0) * std::f64::consts::TAU;
                            let r = if i % 2 == 0 { 8.0 } else { 3.0 };
                            let px = sx + r * angle.cos();
                            let py = sy + r * angle.sin();
                            if i == 0 {
                                self.ctx.move_to(px, py);
                            } else {
                                self.ctx.line_to(px, py);
                            }
                        }
                        self.ctx.fill();
                    }
                }
                
                // Selection highlight
                if Some(system.id) == selected_target {
                    self.ctx.set_stroke_style_str("#00ffff");
                    self.ctx.set_line_width(3.0);
                    let pulse_r = 12.0 + (anim_t * 5.0).sin() * 2.0;
                    self.ctx.begin_path();
                    self.ctx.arc(sx, sy, pulse_r, 0.0, std::f64::consts::TAU).ok();
                    self.ctx.stroke();
                }
                
                // ========== 4. SYSTEM LABELS ==========
                // Always show names, but dimmer for unvisited
                let name_alpha = if system.visited || is_current { 1.0 } else { 0.6 };
                self.ctx.set_global_alpha(name_alpha);
                
                self.ctx.set_font("12px monospace");
                self.ctx.set_fill_style_str("#ffffff");
                self.ctx.fill_text(system.name, sx + 12.0, sy - 2.0).ok();
                
                self.ctx.set_font("10px monospace");
                self.ctx.set_fill_style_str("#aaaaaa");
                self.ctx.fill_text(system.chinese_name, sx + 12.0, sy + 10.0).ok();
                
                // Show location type for unvisited
                if !system.visited && !is_current {
                    let type_name = match system.location_type {
                        LocationType::SpaceStation => "Station",
                        LocationType::AsteroidBase => "Asteroid",
                        LocationType::DerelictShip => "Derelict",
                        LocationType::AlienRuins => "Ruins",
                        LocationType::TradingPost => "Trading",
                        LocationType::OrbitalPlatform => "Platform",
                        LocationType::MiningColony => "Mining",
                        LocationType::ResearchLab => "Lab",
                    };
                    self.ctx.set_font("9px monospace");
                    self.ctx.set_fill_style_str("#888888");
                    self.ctx.fill_text(type_name, sx + 12.0, sy + 20.0).ok();
                }
                
                // Hazard indicator
                if system.hazard.is_some() {
                    self.ctx.set_font("10px monospace");
                    self.ctx.set_fill_style_str("#ff6644");
                    self.ctx.fill_text("⚠", sx - 14.0, sy - 6.0).ok();
                }
                
                self.ctx.set_global_alpha(1.0);
            }
            
            // ========== 5. SYSTEM DETAIL POPUP ==========
            if let Some(sel_id) = selected_target {
                if let Some(sel_sys) = sector.systems.iter().find(|s| s.id == sel_id) {
                    let current_sys = &sector.systems[sector_map.current_system];
                    let dx = sel_sys.x - current_sys.x;
                    let dy = sel_sys.y - current_sys.y;
                    let fuel_cost = ((dx * dx + dy * dy).sqrt() * 10.0).ceil() as i32;
                    
                    // Position popup near selected system
                    let popup_x = cx + (sel_sys.x - 0.5) * scale * 2.0 + 20.0;
                    let popup_y = cy + (sel_sys.y - 0.5) * scale * 2.0 - 60.0;
                    
                    // Background — taller to fit more info
                    self.ctx.set_fill_style_str("rgba(10, 10, 20, 0.9)");
                    self.ctx.fill_rect(popup_x, popup_y, 200.0, 110.0);
                    self.ctx.set_stroke_style_str("#00ffff");
                    self.ctx.set_line_width(1.0);
                    self.ctx.stroke_rect(popup_x, popup_y, 200.0, 110.0);
                    
                    // Content
                    self.ctx.set_font("bold 11px monospace");
                    self.ctx.set_fill_style_str("#ffffff");
                    self.ctx.fill_text(sel_sys.name, popup_x + 5.0, popup_y + 15.0).ok();
                    
                    self.ctx.set_font("10px monospace");
                    self.ctx.set_fill_style_str("#aaaaaa");
                    self.ctx.fill_text(sel_sys.chinese_name, popup_x + 5.0, popup_y + 28.0).ok();
                    
                    let type_str = format!("Type: {:?}", sel_sys.location_type);
                    self.ctx.fill_text(&type_str, popup_x + 5.0, popup_y + 42.0).ok();
                    
                    // Features
                    let mut feats = Vec::new();
                    if sel_sys.has_shop { feats.push("💰Shop"); }
                    if sel_sys.has_fuel { feats.push("⛽Fuel"); }
                    if sel_sys.has_repair { feats.push("🔧Repair"); }
                    if sel_sys.has_medbay { feats.push("🏥Med"); }
                    if sel_sys.quest_giver { feats.push("❗Quest"); }
                    if !feats.is_empty() {
                        self.ctx.set_fill_style_str("#44dd44");
                        self.ctx.fill_text(&feats.join(" "), popup_x + 5.0, popup_y + 55.0).ok();
                    }
                    
                    // Hazard warning
                    if let Some(ref hazard) = sel_sys.hazard {
                        self.ctx.set_font("bold 10px monospace");
                        self.ctx.set_fill_style_str("#ff6644");
                        self.ctx.fill_text(&format!("⚠ {}", hazard.name()), popup_x + 5.0, popup_y + 68.0).ok();
                    }
                    
                    // Fuel cost
                    self.ctx.set_font("bold 11px monospace");
                    self.ctx.set_fill_style_str("#ffdd44");
                    self.ctx.fill_text(&format!("Fuel Cost: {}", fuel_cost), popup_x + 5.0, popup_y + 85.0).ok();
                }
            }
        }
    }

    pub fn draw_starmap_hud(&self, map: &SectorMap, ship: &Ship, cursor: usize) {
        self.ctx.set_text_align("left");
        
        // ========== TOP-LEFT: SHIP STATUS PANEL ==========
        // Panel background
        self.ctx.set_fill_style_str("rgba(10, 10, 20, 0.85)");
        self.ctx.fill_rect(10.0, 35.0, 240.0, 120.0);
        self.ctx.set_stroke_style_str("#00ccdd");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(10.0, 35.0, 240.0, 120.0);
        
        // Title
        self.ctx.set_font("bold 20px monospace");
        self.ctx.set_fill_style_str("#00ccdd");
        self.ctx.fill_text("★ STAR MAP", 20.0, 30.0).ok();
        
        // Hull bar
        self.ctx.set_font("12px monospace");
        self.ctx.set_fill_style_str("#cccccc");
        self.ctx.fill_text("HULL", 20.0, 55.0).ok();
        
        let hull_pct = ship.hull as f64 / ship.max_hull as f64;
        let hull_color = if hull_pct > 0.6 {
            "#44cc55"
        } else if hull_pct > 0.3 {
            "#cccc44"
        } else {
            "#cc4444"
        };
        
        // Bar background
        self.ctx.set_fill_style_str("#222222");
        self.ctx.fill_rect(20.0, 60.0, 220.0, 14.0);
        // Bar fill
        self.ctx.set_fill_style_str(hull_color);
        self.ctx.fill_rect(20.0, 60.0, 220.0 * hull_pct, 14.0);
        // Bar border
        self.ctx.set_stroke_style_str("#666666");
        self.ctx.set_line_width(1.0);
        self.ctx.stroke_rect(20.0, 60.0, 220.0, 14.0);
        // Value text
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_font("bold 10px monospace");
        self.ctx.fill_text(&format!("{}/{}", ship.hull, ship.max_hull), 25.0, 71.0).ok();
        
        // Fuel bar
        self.ctx.set_font("12px monospace");
        self.ctx.set_fill_style_str("#cccccc");
        self.ctx.fill_text("FUEL", 20.0, 90.0).ok();
        
        let fuel_pct = ship.fuel as f64 / ship.max_fuel as f64;
        self.ctx.set_fill_style_str("#222222");
        self.ctx.fill_rect(20.0, 95.0, 220.0, 14.0);
        self.ctx.set_fill_style_str("#4488ff");
        self.ctx.fill_rect(20.0, 95.0, 220.0 * fuel_pct, 14.0);
        self.ctx.set_stroke_style_str("#666666");
        self.ctx.stroke_rect(20.0, 95.0, 220.0, 14.0);
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_font("bold 10px monospace");
        self.ctx.fill_text(&format!("{}/{}", ship.fuel, ship.max_fuel), 25.0, 106.0).ok();
        
        // Shields bar
        self.ctx.set_font("12px monospace");
        self.ctx.set_fill_style_str("#cccccc");
        self.ctx.fill_text("SHIELDS", 20.0, 125.0).ok();
        
        let shield_pct = ship.shields as f64 / ship.max_shields as f64;
        self.ctx.set_fill_style_str("#222222");
        self.ctx.fill_rect(20.0, 130.0, 220.0, 14.0);
        self.ctx.set_fill_style_str("#00ccdd");
        self.ctx.fill_rect(20.0, 130.0, 220.0 * shield_pct, 14.0);
        self.ctx.set_stroke_style_str("#666666");
        self.ctx.stroke_rect(20.0, 130.0, 220.0, 14.0);
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_font("bold 10px monospace");
        self.ctx.fill_text(&format!("{}/{}", ship.shields, ship.max_shields), 25.0, 141.0).ok();
        
        // Cargo
        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#aaaaaa");
        self.ctx.fill_text(&format!("Cargo: {}/{}", ship.cargo_used, ship.cargo_capacity), 20.0, 152.0).ok();
        
        // ========== TOP-RIGHT: SECTOR INFO ==========
        if let Some(sector) = map.sectors.get(map.current_sector) {
            let panel_x = self.canvas_w - 260.0;
            
            // Panel background
            self.ctx.set_fill_style_str("rgba(10, 10, 20, 0.85)");
            self.ctx.fill_rect(panel_x, 35.0, 250.0, 80.0);
            self.ctx.set_stroke_style_str("#ffdd44");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(panel_x, 35.0, 250.0, 80.0);
            
            // Sector name
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_fill_style_str("#ffdd44");
            self.ctx.fill_text(&format!("SECTOR: {}", sector.name), panel_x + 10.0, 55.0).ok();
            
            self.ctx.set_font("12px monospace");
            self.ctx.set_fill_style_str("#cccccc");
            self.ctx.fill_text(&format!("HSK Level: {}", sector.hsk_level), panel_x + 10.0, 72.0).ok();
            
            // Systems explored count
            let visited_count = sector.systems.iter().filter(|s| s.visited).count();
            let total_count = sector.systems.len();
            self.ctx.fill_text(&format!("Explored: {}/{}", visited_count, total_count), panel_x + 10.0, 88.0).ok();
            
            // Sector description
            self.ctx.set_font("10px monospace");
            self.ctx.set_fill_style_str("#888888");
            self.ctx.fill_text(sector.description, panel_x + 10.0, 105.0).ok();
        }
        
        // ========== BOTTOM: NAVIGATION PANEL ==========
        if let Some(sector) = map.sectors.get(map.current_sector) {
            let sys = &sector.systems[map.current_system];
            let panel_y = self.canvas_h - 150.0;
            
            // Panel background
            self.ctx.set_fill_style_str("rgba(10, 10, 20, 0.90)");
            self.ctx.fill_rect(10.0, panel_y, self.canvas_w - 20.0, 125.0);
            self.ctx.set_stroke_style_str("#44dd88");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(10.0, panel_y, self.canvas_w - 20.0, 125.0);
            
            // Current system box
            self.ctx.set_font("bold 14px monospace");
            self.ctx.set_fill_style_str("#44dd88");
            self.ctx.fill_text("CURRENT LOCATION", 20.0, panel_y + 20.0).ok();
            
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_fill_style_str("#ffffff");
            self.ctx.fill_text(sys.name, 20.0, panel_y + 40.0).ok();
            
            self.ctx.set_font("12px monospace");
            self.ctx.set_fill_style_str("#aaaaaa");
            self.ctx.fill_text(sys.chinese_name, 20.0, panel_y + 56.0).ok();
            
            let type_str = format!("{:?}", sys.location_type);
            self.ctx.fill_text(&type_str, 20.0, panel_y + 72.0).ok();
            
            // System description
            self.ctx.set_font("10px monospace");
            self.ctx.set_fill_style_str("#777777");
            self.ctx.fill_text(sys.description, 200.0, panel_y + 40.0).ok();
            
            // Hazard warning
            if let Some(ref hazard) = sys.hazard {
                self.ctx.set_font("bold 11px monospace");
                self.ctx.set_fill_style_str("#ff6644");
                self.ctx.fill_text(&format!("⚠ {} {}", hazard.icon(), hazard.name()), 200.0, panel_y + 56.0).ok();
            }
            
            // Shop/Fuel/Repair/Medbay indicators
            let mut features = Vec::new();
            if sys.has_shop { features.push("💰Shop"); }
            if sys.has_fuel { features.push("⛽Fuel"); }
            if sys.has_repair { features.push("🔧Repair"); }
            if sys.has_medbay { features.push("🏥Medbay"); }
            if sys.quest_giver { features.push("❗Quest"); }
            if sys.warp_gate { features.push("🌀Warp"); }
            if !features.is_empty() {
                self.ctx.set_fill_style_str("#44dd44");
                self.ctx.fill_text(&format!("Available: {}", features.join(", ")), 20.0, panel_y + 88.0).ok();
            }
            
            // Jump targets
            let connections = &sys.connections;
            if !connections.is_empty() {
                self.ctx.set_font("bold 13px monospace");
                self.ctx.set_fill_style_str("#cccccc");
                self.ctx.fill_text("JUMP TARGETS:", 20.0, panel_y + 108.0).ok();
                
                self.ctx.set_font("11px monospace");
                self.ctx.set_fill_style_str("#666666");
                self.ctx.fill_text("(Use ←/→ to select, Enter to jump, E to explore)", 180.0, panel_y + 108.0).ok();
                
                // Draw targets horizontally with fuel costs
                let mut x_pos = 20.0;
                for (i, &conn_id) in connections.iter().enumerate() {
                    if let Some(target) = sector.systems.iter().find(|s| s.id == conn_id) {
                        let is_selected = i == cursor % connections.len();
                        
                        // Calculate fuel cost
                        let dx = target.x - sys.x;
                        let dy = target.y - sys.y;
                        let fuel_cost = ((dx * dx + dy * dy).sqrt() * 10.0).ceil() as i32;
                        let can_afford = ship.fuel >= fuel_cost;
                        
                        if is_selected {
                            self.ctx.set_fill_style_str("#00ffff");
                            self.ctx.set_font("bold 12px monospace");
                            self.ctx.fill_text("▶", x_pos, panel_y + 125.0).ok();
                            self.ctx.fill_text(&format!("{} ", target.name), x_pos + 12.0, panel_y + 125.0).ok();
                            
                            // Fuel cost indicator
                            let cost_color = if can_afford { "#ffdd44" } else { "#ff4444" };
                            self.ctx.set_fill_style_str(cost_color);
                            self.ctx.set_font("bold 11px monospace");
                            self.ctx.fill_text(&format!("[Fuel: {}]", fuel_cost), x_pos + 12.0 + (target.name.len() as f64 * 7.5), panel_y + 125.0).ok();
                            
                            x_pos += (target.name.len() as f64 + 12.0) * 8.0;
                        } else {
                            self.ctx.set_fill_style_str("#666666");
                            self.ctx.set_font("11px monospace");
                            self.ctx.fill_text(&format!("{}  ", target.name), x_pos, panel_y + 125.0).ok();
                            x_pos += (target.name.len() as f64 + 2.0) * 7.0;
                        }
                    }
                }
            }
        }
        
        // ========== BOTTOM-RIGHT: LEGEND ==========
        let legend_x = self.canvas_w - 210.0;
        let legend_y = self.canvas_h - 150.0;
        
        self.ctx.set_fill_style_str("rgba(10, 10, 20, 0.85)");
        self.ctx.fill_rect(legend_x, legend_y, 200.0, 125.0);
        self.ctx.set_stroke_style_str("#888888");
        self.ctx.set_line_width(1.0);
        self.ctx.stroke_rect(legend_x, legend_y, 200.0, 125.0);
        
        self.ctx.set_font("bold 11px monospace");
        self.ctx.set_fill_style_str("#cccccc");
        self.ctx.fill_text("LEGEND", legend_x + 5.0, legend_y + 15.0).ok();
        
        let legend_items = [
            ("#ffdd44", "Space Station"),
            ("#cc7733", "Asteroid Base"),
            ("#8844cc", "Derelict Ship"),
            ("#44ccaa", "Alien Ruins"),
            ("#44dd44", "Trading Post"),
            ("#4488ff", "Orbital Platform"),
            ("#cc8844", "Mining Colony"),
            ("#ff88cc", "Research Lab"),
        ];
        
        self.ctx.set_font("10px monospace");
        for (i, (color, name)) in legend_items.iter().enumerate() {
            let y = legend_y + 30.0 + i as f64 * 13.0;
            
            // Color swatch
            self.ctx.set_fill_style_str(color);
            self.ctx.fill_rect(legend_x + 5.0, y - 8.0, 10.0, 10.0);
            
            // Name
            self.ctx.set_fill_style_str("#aaaaaa");
            self.ctx.fill_text(name, legend_x + 20.0, y).ok();
        }
        
        // ========== BOTTOM: CONTROLS REMINDER ==========
        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#444444");
        self.ctx.fill_text("[M] Map  [E] Explore Current  [Enter] Jump  [←/→] Select Target", 20.0, self.canvas_h - 5.0).ok();
    }

    pub fn draw_ship_interior(
        &self,
        layout: &ShipLayout,
        ship_x: i32,
        ship_y: i32,
        _crew: &[crate::player::CrewMember],
        _anim_t: f64,
    ) {
        // Clear
        self.ctx.set_fill_style_str("#111111");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

        let tx_size = 32.0;
        let offset_x = (self.canvas_w - layout.width as f64 * tx_size) / 2.0;
        let offset_y = (self.canvas_h - layout.height as f64 * tx_size) / 2.0;

        for (i, tile) in layout.tiles.iter().enumerate() {
            let x = (i as i32 % layout.width) as f64;
            let y = (i as i32 / layout.width) as f64;
            let screen_x = offset_x + x * tx_size;
            let screen_y = offset_y + y * tx_size;

            let color = match tile {
                ShipTile::Floor => "#222233",
                ShipTile::Wall => "#444455",
                ShipTile::Door => "#666677",
                ShipTile::Console(_) => "#0088aa",
                ShipTile::CrewStation(_) => "#00aa88",
                ShipTile::Decoration(_) => "#333344",
                ShipTile::Empty => continue,
            };

            self.ctx.set_fill_style_str(color);
            self.ctx.fill_rect(screen_x, screen_y, tx_size, tx_size);
            
            // Grid lines
            self.ctx.set_stroke_style_str("#333344");
            self.ctx.stroke_rect(screen_x, screen_y, tx_size, tx_size);
        }

        // Draw Player
        let px = offset_x + ship_x as f64 * tx_size;
        let py = offset_y + ship_y as f64 * tx_size;
        
        self.ctx.set_fill_style_str(COL_PLAYER);
        self.ctx.begin_path();
        self.ctx.arc(px + tx_size/2.0, py + tx_size/2.0, tx_size/3.0, 0.0, std::f64::consts::TAU).ok();
        self.ctx.fill();
    }

    pub fn draw_space_combat(
        &self,
        player_ship: &Ship,
        // enemy_ship: &Ship, // Assuming enemy ship struct is same or similar
        // For now just draw HUD
        _anim_t: f64,
    ) {
        self.ctx.set_fill_style_str("#000000");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);
        
        // Draw Player Ship (Left)
        self.ctx.set_fill_style_str("#00ccdd");
        self.ctx.fill_rect(100.0, 300.0, 100.0, 60.0);
        
        // Draw Enemy Ship (Right)
        self.ctx.set_fill_style_str("#ff5555");
        self.ctx.fill_rect(self.canvas_w - 200.0, 300.0, 100.0, 60.0);
        
        // UI
        self.ctx.set_font("20px monospace");
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.fill_text(&format!("Hull: {}/{}", player_ship.hull, player_ship.max_hull), 20.0, 40.0).ok();
        self.ctx.fill_text(&format!("Shields: {}/{}", player_ship.shields, player_ship.max_shields), 20.0, 70.0).ok();
    }

    pub fn draw_event(
        &self,
        event: &SpaceEvent,
        cursor: usize,
    ) {
        // Overlay background
        self.ctx.set_fill_style_str("rgba(0, 0, 0, 0.9)");
        self.ctx.fill_rect(50.0, 50.0, self.canvas_w - 100.0, self.canvas_h - 100.0);
        
        self.ctx.set_stroke_style_str("#00ccdd");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(50.0, 50.0, self.canvas_w - 100.0, self.canvas_h - 100.0);
        
        // Title
        self.ctx.set_font("bold 24px serif");
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_text_align("center");
        self.ctx.fill_text(event.title, self.canvas_w / 2.0, 100.0).ok();
        
        self.ctx.set_font("20px serif");
        self.ctx.set_fill_style_str("#cccccc");
        self.ctx.fill_text(event.chinese_title, self.canvas_w / 2.0, 130.0).ok();
        
        // Description (simple wrap)
        self.ctx.set_font("16px monospace");
        self.ctx.set_fill_style_str("#aaaaaa");
        self.ctx.set_text_align("left");
        self.ctx.fill_text(event.description, 80.0, 180.0).ok();
        
        // Choices
        let start_y = 300.0;
        for (i, choice) in event.choices.iter().enumerate() {
            let y = start_y + i as f64 * 40.0;
            if i == cursor {
                self.ctx.set_fill_style_str("#004455");
                self.ctx.fill_rect(70.0, y - 20.0, self.canvas_w - 140.0, 30.0);
                self.ctx.set_fill_style_str("#ffffff");
            } else {
                self.ctx.set_fill_style_str("#888888");
            }
            self.ctx.fill_text(&format!("{}. {}", i+1, choice.text), 80.0, y).ok();
        }
    }
}



