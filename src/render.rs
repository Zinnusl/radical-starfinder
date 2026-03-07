//! Canvas 2D rendering for the dungeon.

use std::collections::BTreeMap;

use js_sys::Date;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::dungeon::{AltarKind, DungeonLevel, SealKind, Tile};
use crate::enemy::Enemy;
use crate::game::{CombatState, GameSettings, TalentTree};
use crate::particle::ParticleSystem;
use crate::player::{Player, PlayerForm};

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
        })
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
        listening_mode: bool,
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
    ) {
        let anim_t = Date::now() / 1000.0;
        // Screen shake offset
        let shake_x = if shake_timer > 0 { (shake_timer as f64 * 1.7).sin() * 4.0 } else { 0.0 };
        let shake_y = if shake_timer > 0 { (shake_timer as f64 * 2.3).cos() * 3.0 } else { 0.0 };

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

                if visible {
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

        if player.form == PlayerForm::Human {
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
        } else {
            // Render Form Glyph
            self.ctx.set_shadow_color(player.form.color());
            self.ctx.set_shadow_blur(15.0);

            self.ctx.set_font("bold 22px serif");
            self.ctx.set_text_align("center");
            self.ctx.set_text_baseline("middle");
            self.ctx.set_fill_style_str(player.form.color());
            self.ctx.fill_text(player.form.glyph(), center_x, center_y).ok();
            
            // Draw timer bar below if temporal
            if player.form_timer > 0 {
                self.ctx.set_shadow_blur(0.0);
                self.ctx.set_fill_style_str("#444");
                self.ctx.fill_rect(center_x - 10.0, center_y + 12.0, 20.0, 3.0);
                self.ctx.set_fill_style_str(player.form.color());
                let pct = (player.form_timer as f64 / 50.0).min(1.0); // Assume max 50 for bar scaling
                self.ctx.fill_rect(center_x - 10.0, center_y + 12.0, 20.0 * pct, 3.0);
            }
        }

        // Reset shadow
        self.ctx.set_shadow_blur(0.0);
        self.ctx.set_shadow_color("transparent");

        // ── Enemies ─────────────────────────────────────────────────────
        for (i, enemy) in enemies.iter().enumerate() {
            if !enemy.is_alive() {
                continue;
            }
            let eidx = level.idx(enemy.x, enemy.y);
            if !level.visible[eidx] {
                continue;
            }
            let ex = enemy.x as f64 * TILE_SIZE - cam_x;
            let ey = enemy.y as f64 * TILE_SIZE - cam_y
                + (anim_t * 3.3 + (enemy.x as f64 * 0.37) + (enemy.y as f64 * 0.23)).sin() * 1.2;

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
            let is_fighting = matches!(combat, CombatState::Fighting { enemy_idx, .. } if *enemy_idx == i);
            if is_fighting {
                self.ctx.set_stroke_style_str("#ff4444");
                self.ctx.set_line_width(2.0);
                self.ctx
                    .stroke_rect(ex + 1.0, ey + 1.0, TILE_SIZE - 2.0, TILE_SIZE - 2.0);
            }

            // Draw Hanzi character (bosses are larger and purple)
            let font_size = if enemy.is_boss { "22px" } else if enemy.is_elite { "16px" } else { "18px" };
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
            self.ctx.set_font(&format!("{} 'Noto Serif SC', 'SimSun', serif", font_size));
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(enemy.hanzi, ex + TILE_SIZE / 2.0, ey + TILE_SIZE * 0.72)
                .ok();

            // Small HP bar below
            if enemy.hp < enemy.max_hp {
                let hp_frac = enemy.hp as f64 / enemy.max_hp as f64;
                self.ctx.set_fill_style_str("#440000");
                self.ctx.fill_rect(ex + 2.0, ey + TILE_SIZE - 4.0, TILE_SIZE - 4.0, 3.0);
                self.ctx.set_fill_style_str("#ff4444");
                self.ctx.fill_rect(ex + 2.0, ey + TILE_SIZE - 4.0, (TILE_SIZE - 4.0) * hp_frac, 3.0);
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
                self.ctx.fill_text(&format!("{}{}", s.label(), s.turns_left), sx, bar_y + 12.0).ok();
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
            .fill_text(
                &floor_label,
                self.canvas_w - 12.0,
                24.0,
            )
            .ok();
        self.ctx.set_fill_style_str("#ffdd44");
        self.ctx
            .fill_text(
                &format!("{}g", player.gold),
                self.canvas_w - 12.0,
                42.0,
            )
            .ok();

        // Equipment display (top-right, below gold)
        let mut eq_y = 58.0;
        self.ctx.set_font("10px monospace");
        if let Some(w) = player.weapon {
            let ench = player.enchantments[0].map(|e| format!(" [{}]", e)).unwrap_or_default();
            self.ctx.set_fill_style_str("#ff8866");
            self.ctx.fill_text(&format!("⚔ {}{}", w.name, ench), self.canvas_w - 12.0, eq_y).ok();
            eq_y += 14.0;
        }
        if let Some(a) = player.armor {
            let ench = player.enchantments[1].map(|e| format!(" [{}]", e)).unwrap_or_default();
            self.ctx.set_fill_style_str("#6688ff");
            self.ctx.fill_text(&format!("🛡 {}{}", a.name, ench), self.canvas_w - 12.0, eq_y).ok();
            eq_y += 14.0;
        }
        if let Some(c) = player.charm {
            let ench = player.enchantments[2].map(|e| format!(" [{}]", e)).unwrap_or_default();
            self.ctx.set_fill_style_str("#88ddaa");
            self.ctx.fill_text(&format!("✧ {}{}", c.name, ench), self.canvas_w - 12.0, eq_y).ok();
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
        if listening_mode {
            self.ctx.set_fill_style_str("#aa66ff");
            self.ctx.fill_text("🎧 Listening", self.canvas_w - 12.0, eq_y).ok();
            eq_y += 14.0;
        }
        // Companion indicator
        if let Some(comp) = companion {
            self.ctx.set_fill_style_str("#55ccaa");
            self.ctx.fill_text(&format!("{} {}", comp.icon(), comp.name()), self.canvas_w - 12.0, eq_y).ok();
            eq_y += 14.0;
        }

        self.ctx.set_fill_style_str("#9cb7ff");
        self.ctx.fill_text("[V] Look", self.canvas_w - 12.0, eq_y).ok();
        eq_y += 14.0;
        self.ctx.set_fill_style_str("#7e8dbb");
        self.ctx.fill_text("[X] Skip floor", self.canvas_w - 12.0, eq_y).ok();
        eq_y += 14.0;
        self.ctx.set_fill_style_str("#8fa8ff");
        self.ctx.fill_text("[?] Help", self.canvas_w - 12.0, eq_y).ok();

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
            self.ctx.fill_text("Tutorial", self.canvas_w / 2.0, 24.0).ok();
            self.ctx.set_fill_style_str("#ddd");
            self.ctx.set_font("12px monospace");
            self.ctx.fill_text(tutorial_hint, self.canvas_w / 2.0, 39.0).ok();
        }

        // ── Radical inventory (left side) ───────────────────────────────
        if !player.radicals.is_empty() {
            let inv_x = 12.0;
            let inv_y = 44.0;
            self.ctx.set_font("11px monospace");
            self.ctx.set_text_align("left");
            self.ctx.set_fill_style_str("#ff8844");
            self.ctx.fill_text("Radicals:", inv_x, inv_y).ok();
            self.ctx.set_font("14px 'Noto Serif SC', 'SimSun', serif");
            self.ctx.set_fill_style_str("#ffaa66");
            let rad_str: String = player.radicals.iter().copied().collect::<Vec<_>>().join(" ");
            self.ctx.fill_text(&rad_str, inv_x, inv_y + 16.0).ok();
        }

        // ── Spell bar (below radicals) ──────────────────────────────────
        if !player.spells.is_empty() {
            let sp_x = 12.0;
            let sp_y = if player.radicals.is_empty() { 44.0 } else { 78.0 };
            self.ctx.set_font("11px monospace");
            self.ctx.set_text_align("left");
            self.ctx.set_fill_style_str("#44aaff");
            self.ctx.fill_text("Spells:", sp_x, sp_y).ok();
            for (i, spell) in player.spells.iter().enumerate() {
                let y = sp_y + 16.0 + i as f64 * 16.0;
                let selected = i == player.selected_spell;
                self.ctx.set_fill_style_str(if selected { "#ffcc33" } else { "#88bbdd" });
                self.ctx.set_font("12px monospace");
                let marker = if selected { "►" } else { " " };
                self.ctx
                    .fill_text(
                        &format!("{}{} {}", marker, spell.hanzi, spell.effect.label()),
                        sp_x,
                        y,
                    )
                    .ok();
            }
        }

        // ── Item inventory (below spells, left side) ────────────────────
        if !player.items.is_empty() {
            let spell_count = player.spells.len();
            let base_y = if player.radicals.is_empty() && player.spells.is_empty() {
                44.0
            } else if player.spells.is_empty() {
                78.0
            } else {
                let sp_y = if player.radicals.is_empty() { 44.0 } else { 78.0 };
                sp_y + 16.0 + spell_count as f64 * 16.0 + 8.0
            };
            self.ctx.set_font("11px monospace");
            self.ctx.set_text_align("left");
            self.ctx.set_fill_style_str("#ddaa44");
            self.ctx.fill_text("Items [1-5]  [I] Inventory:", 12.0, base_y).ok();
            for (i, label) in item_labels.iter().enumerate() {
                let y = base_y + 16.0 + i as f64 * 14.0;
                self.ctx.set_fill_style_str("#ccbb66");
                self.ctx.set_font("11px monospace");
                self.ctx.fill_text(&format!("{}: {}", i + 1, label), 12.0, y).ok();
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
            self.ctx.arc(p.x, p.y, radius, 0.0, std::f64::consts::TAU).ok();
            self.ctx.fill();

            let speed = (p.vx * p.vx + p.vy * p.vy).sqrt();
            if speed > 1.2 {
                self.ctx
                    .set_stroke_style_str(&format!("rgba({},{},{},{})", p.r, p.g, p.b, alpha * 0.35));
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
            self.ctx.set_fill_style_str(&format!("rgba({},{},{},{})", r, g, b, a));
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
            self.ctx.fill_text("🏆 Achievement Unlocked!", px + pw / 2.0, py + 20.0).ok();
            self.ctx.set_font("12px monospace");
            self.ctx.set_fill_style_str("#ffffff");
            self.ctx.fill_text(&format!("{} — {}", icon_name, desc), px + pw / 2.0, py + 40.0).ok();
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
                self.ctx.fill_text(
                    &format!("{} {} [{}]", status, q.description, progress),
                    8.0,
                    qy,
                ).ok();
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
                .fill_text(message, self.canvas_w / 2.0, self.canvas_h - 17.0 - message_lift)
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
                let hanzi_y = if boss_trait.is_some() { box_y + 46.0 } else { box_y + 52.0 };

                // Background
                self.ctx.set_fill_style_str("rgba(20,10,30,0.92)");
                self.ctx.fill_rect(box_x, box_y, box_w, box_h);
                self.ctx.set_stroke_style_str("#ff6666");
                self.ctx.set_line_width(2.0);
                self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

                if let Some(kind) = enemy.boss_kind {
                    self.ctx.set_fill_style_str("#ffcc88");
                    self.ctx.set_font("bold 12px monospace");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text(kind.title(), self.canvas_w / 2.0, box_y + 16.0)
                        .ok();
                }

                // Enemy hanzi (large) — hidden in listening mode for non-elite
                let show_hanzi = !listening_mode || enemy.is_elite;
                self.ctx.set_fill_style_str(if listening_mode && !enemy.is_elite { "#aa66ff" } else { "#ff6666" });
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
                        .fill_text(
                            teacher_hint,
                            self.canvas_w / 2.0,
                            hanzi_y + 20.0,
                        )
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
                    let total_w =
                        segment_count as f64 * seg_w + segment_count.saturating_sub(1) as f64 * seg_gap;
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
                        self.ctx.set_stroke_style_str(if current { "#ffdd88" } else { "#665544" });
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
                self.ctx.fill_rect(box_x + 30.0, input_y, box_w - 60.0, 28.0);
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
                self.ctx.set_fill_style_str(if typing.is_empty() {
                    "#555"
                } else {
                    "#ffcc33"
                });
                self.ctx.set_font("16px monospace");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(display, self.canvas_w / 2.0, input_y + 20.0)
                    .ok();

                // Hint text
                self.ctx.set_fill_style_str("#555");
                self.ctx.set_font("10px monospace");
                // Show example sentence if available
                let example = crate::vocab::VOCAB.iter()
                    .find(|v| v.hanzi == enemy.hanzi)
                    .map(|v| v.example)
                    .unwrap_or("");
                if !example.is_empty() && show_hanzi {
                    self.ctx.set_fill_style_str("#667788");
                    self.ctx.set_font("11px 'Noto Serif SC', monospace");
                    self.ctx.fill_text(example, self.canvas_w / 2.0, box_y + box_h - 8.0).ok();
                    self.ctx.set_fill_style_str("#555");
                    self.ctx.set_font("10px monospace");
                }
                self.ctx
                    .fill_text(
                        if enemy.is_elite {
                            "Enter=submit syllable  Esc=flee  Tab=cycle spell  Space=cast spell"
                        } else {
                            "Enter=submit  Esc=flee  Tab=cycle spell  Space=cast spell"
                        },
                        self.canvas_w / 2.0,
                        box_y + box_h + 14.0,
                    )
                    .ok();
            }
        }

        // ── Forge UI overlay ─────────────────────────────────────────────
        if let CombatState::Forging { ref selected, ref page } = combat {
            let page = *page;
            let rad_count = player.radicals.len();
            let page_size = 9;
            let max_page = rad_count.saturating_sub(1) / page_size;
            let page_start = page * page_size;
            let page_end = (page_start + page_size).min(rad_count);
            let page_count = page_end - page_start;

            let box_w = 380.0;
            let box_h = 100.0 + (page_count as f64 / 5.0).ceil() * 36.0
                + if max_page > 0 { 20.0 } else { 0.0 };
            let box_x = (self.canvas_w - box_w) / 2.0;
            let box_y = 40.0;

            // Background
            self.ctx.set_fill_style_str("rgba(30,15,10,0.95)");
            self.ctx.fill_rect(box_x, box_y, box_w, box_h);
            self.ctx.set_stroke_style_str("#ff8844");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

            // Title
            self.ctx.set_fill_style_str("#ff8844");
            self.ctx.set_font("18px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("⚒ Radical Forge ⚒", self.canvas_w / 2.0, box_y + 26.0)
                .ok();

            // Show radicals in grid
            self.ctx.set_font("11px monospace");
            self.ctx.set_fill_style_str("#aaa");
            let hint = if max_page > 0 {
                format!("1-9 toggle, ←/→ page ({}/{}), Enter forge", page + 1, max_page + 1)
            } else {
                "Press 1-9 to toggle radicals, Enter to forge".to_string()
            };
            self.ctx
                .fill_text(&hint, self.canvas_w / 2.0, box_y + 44.0)
                .ok();

            let grid_y = box_y + 56.0;
            for (slot, abs_idx) in (page_start..page_end).enumerate() {
                let rad_ch = player.radicals[abs_idx];
                let col = slot % 5;
                let row = slot / 5;
                let rx = box_x + 20.0 + col as f64 * 72.0;
                let ry = grid_y + row as f64 * 36.0;

                let is_selected = selected.contains(&abs_idx);

                // Slot background
                self.ctx.set_fill_style_str(if is_selected {
                    "rgba(255,136,68,0.3)"
                } else {
                    "rgba(0,0,0,0.3)"
                });
                self.ctx.fill_rect(rx, ry, 64.0, 30.0);
                self.ctx.set_stroke_style_str(if is_selected {
                    "#ffaa66"
                } else {
                    "#555"
                });
                self.ctx.set_line_width(1.0);
                self.ctx.stroke_rect(rx, ry, 64.0, 30.0);

                // Number key
                self.ctx.set_fill_style_str("#888");
                self.ctx.set_font("10px monospace");
                self.ctx.set_text_align("left");
                self.ctx
                    .fill_text(&format!("{}", slot + 1), rx + 2.0, ry + 11.0)
                    .ok();

                // Radical character
                self.ctx.set_fill_style_str(if is_selected { "#ffcc33" } else { "#ffaa66" });
                self.ctx.set_font("18px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(rad_ch, rx + 32.0, ry + 24.0)
                    .ok();
            }

            // Page arrows
            if max_page > 0 {
                let arrow_y = grid_y + (page_count as f64 / 5.0).ceil() * 36.0 + 4.0;
                self.ctx.set_fill_style_str(if page > 0 { "#ffaa66" } else { "#444" });
                self.ctx.set_font("14px monospace");
                self.ctx.set_text_align("center");
                self.ctx.fill_text("◀", box_x + 40.0, arrow_y + 12.0).ok();
                self.ctx.set_fill_style_str(if page < max_page { "#ffaa66" } else { "#444" });
                self.ctx.fill_text("▶", box_x + box_w - 40.0, arrow_y + 12.0).ok();
            }

            // Show selected combo
            if !selected.is_empty() {
                let combo_y = grid_y + ((page_count as f64 / 5.0).ceil()) * 36.0
                    + if max_page > 0 { 24.0 } else { 8.0 };
                let combo_str: String = selected
                    .iter()
                    .map(|&i| player.radicals[i])
                    .collect::<Vec<_>>()
                    .join(" + ");
                self.ctx.set_fill_style_str("#ffcc33");
                self.ctx.set_font("16px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(
                        &format!("Forging: {} → ?", combo_str),
                        self.canvas_w / 2.0,
                        combo_y,
                    )
                    .ok();
            }

            // Bottom hint
            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("10px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    "Enter=forge  Esc=cancel",
                    self.canvas_w / 2.0,
                    box_y + box_h + 14.0,
                )
                .ok();
        }

        // ── Enchanting UI overlay ───────────────────────────────────────
        if let CombatState::Enchanting { slot, page } = combat {
            let rad_count = player.radicals.len();
            let page_size = 9;
            let _max_page = rad_count.saturating_sub(1) / page_size;
            let page_start = page * page_size;
            let page_end = (page_start + page_size).min(rad_count);
            let page_count = page_end - page_start;

            let box_w = 380.0;
            let box_h = 160.0 + (page_count as f64 / 5.0).ceil() * 36.0;
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
            self.ctx.fill_text("✦ Enchant Equipment ✦", self.canvas_w / 2.0, box_y + 26.0).ok();

            // Equipment slots
            let slots = [
                (0, "1:Weapon", player.weapon.map(|e| e.name).unwrap_or("—"), player.enchantments[0]),
                (1, "2:Armor", player.armor.map(|e| e.name).unwrap_or("—"), player.enchantments[1]),
                (2, "3:Charm", player.charm.map(|e| e.name).unwrap_or("—"), player.enchantments[2]),
            ];
            let slot_y = box_y + 46.0;
            for (i, &(slot_idx, label, name, ench)) in slots.iter().enumerate() {
                let color = if slot_idx == *slot { "#ffcc33" } else { "#888" };
                self.ctx.set_fill_style_str(color);
                self.ctx.set_font("12px monospace");
                self.ctx.set_text_align("left");
                let ench_str = ench.map(|e| format!(" [{}]", e)).unwrap_or_default();
                self.ctx.fill_text(
                    &format!("{} {}{}", label, name, ench_str),
                    box_x + 20.0,
                    slot_y + i as f64 * 20.0,
                ).ok();
            }

            // Radical grid
            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("11px monospace");
            self.ctx.set_text_align("center");
            self.ctx.fill_text("Pick radical (4-9):", self.canvas_w / 2.0, slot_y + 72.0).ok();

            let grid_y = slot_y + 84.0;
            for (i, abs_idx) in (page_start..page_end).enumerate() {
                let rad_ch = player.radicals[abs_idx];
                let col = i % 5;
                let row = i / 5;
                let rx = box_x + 20.0 + col as f64 * 72.0;
                let ry = grid_y + row as f64 * 36.0;

                self.ctx.set_fill_style_str("rgba(0,0,0,0.3)");
                self.ctx.fill_rect(rx, ry, 64.0, 30.0);
                self.ctx.set_stroke_style_str("#aa66ff");
                self.ctx.set_line_width(1.0);
                self.ctx.stroke_rect(rx, ry, 64.0, 30.0);

                self.ctx.set_fill_style_str("#888");
                self.ctx.set_font("10px monospace");
                self.ctx.set_text_align("left");
                self.ctx.fill_text(&format!("{}", i + 1), rx + 2.0, ry + 11.0).ok();

                self.ctx.set_fill_style_str("#cc99ff");
                self.ctx.set_font("18px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.set_text_align("center");
                self.ctx.fill_text(rad_ch, rx + 32.0, ry + 24.0).ok();
            }

            // Bottom hint
            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("10px monospace");
            self.ctx.set_text_align("center");
            self.ctx.fill_text("1-3=slot  4-9=radical  Esc=cancel", self.canvas_w / 2.0, box_y + box_h + 14.0).ok();
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
                    self.ctx.fill_rect(box_x + 10.0, y - 6.0, box_w - 20.0, 24.0);
                }

                let marker = if selected { "►" } else { " " };
                let total_discount = (player.shop_discount_pct
                    + if companion == Some(crate::game::Companion::Merchant) { 20 } else { 0 })
                    .clamp(0, 50);
                let display_cost = ((item.cost * (100 - total_discount)) + 99) / 100;
                let can_afford = player.gold >= display_cost;
                self.ctx.set_fill_style_str(if can_afford { "#ccffcc" } else { "#666" });
                self.ctx.set_font("13px monospace");
                self.ctx.set_text_align("left");
                let price_label = if total_discount > 0 {
                    format!("{} {} — {}g ({}% off)", marker, item.label, display_cost, total_discount)
                } else {
                    format!("{} {} — {}g", marker, item.label, item.cost)
                };
                self.ctx
                    .fill_text(
                        &price_label,
                        box_x + 15.0,
                        y + 10.0,
                    )
                    .ok();
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
            self.ctx.fill_text("Boss Phase 2 — Arrange the Sentence!", self.canvas_w / 2.0, box_y + 22.0).ok();

            // Meaning hint
            self.ctx.set_fill_style_str("#999");
            self.ctx.set_font("12px monospace");
            self.ctx.fill_text(&format!("Meaning: {}", meaning), self.canvas_w / 2.0, box_y + 42.0).ok();

            // Arranged so far
            let arranged_text: String = arranged.iter().map(|&i| words[i]).collect::<Vec<_>>().join(" ");
            self.ctx.set_fill_style_str("#66ff66");
            self.ctx.set_font("20px 'Noto Serif SC', serif");
            self.ctx.fill_text(
                if arranged_text.is_empty() { "..." } else { &arranged_text },
                self.canvas_w / 2.0,
                box_y + 75.0,
            ).ok();

            // Remaining tiles
            let remaining: Vec<usize> = tiles.iter().copied()
                .filter(|t| !arranged.contains(t))
                .collect();
            let tile_w = 60.0;
            let total_w = remaining.len() as f64 * tile_w;
            let start_x = (self.canvas_w - total_w) / 2.0;
            for (i, &word_idx) in remaining.iter().enumerate() {
                let tx = start_x + i as f64 * tile_w;
                let ty = box_y + 100.0;
                let selected = i == *cursor;
                self.ctx.set_fill_style_str(if selected { "rgba(100,80,160,0.8)" } else { "rgba(40,30,60,0.8)" });
                self.ctx.fill_rect(tx + 2.0, ty, tile_w - 4.0, 36.0);
                self.ctx.set_stroke_style_str(if selected { "#ffcc33" } else { "#555" });
                self.ctx.set_line_width(if selected { 2.0 } else { 1.0 });
                self.ctx.stroke_rect(tx + 2.0, ty, tile_w - 4.0, 36.0);
                self.ctx.set_fill_style_str(if selected { "#ffcc33" } else { "#ccccee" });
                self.ctx.set_font("16px 'Noto Serif SC', serif");
                self.ctx.set_text_align("center");
                self.ctx.fill_text(words[word_idx], tx + tile_w / 2.0, ty + 24.0).ok();
            }

            // Controls hint
            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("10px monospace");
            self.ctx.fill_text("←→ select  Enter=pick  Backspace=undo  Esc=skip", self.canvas_w / 2.0, box_y + box_h - 10.0).ok();
        }

        // ── Tone Battle overlay ─────────────────────────────────────────
        if let CombatState::ToneBattle { round, hanzi, correct_tone: _, score, last_result } = combat {
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
            self.ctx.fill_text(&format!("🔔 Tone Shrine — Round {}/5", round + 1), self.canvas_w / 2.0, box_y + 24.0).ok();

            // Character
            self.ctx.set_fill_style_str("#ffcc33");
            self.ctx.set_font("42px 'Noto Serif SC', 'SimSun', serif");
            self.ctx.fill_text(hanzi, self.canvas_w / 2.0, box_y + 75.0).ok();

            // Score
            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("12px monospace");
            self.ctx.fill_text(&format!("Score: {}/{}", score, round + 1), self.canvas_w / 2.0, box_y + 95.0).ok();

            // Tone options
            let tones = ["1: ā (flat)", "2: á (rising)", "3: ǎ (dip)", "4: à (falling)"];
            self.ctx.set_font("14px monospace");
            for (i, label) in tones.iter().enumerate() {
                let y = box_y + 115.0 + i as f64 * 18.0;
                self.ctx.set_fill_style_str("#ccccee");
                self.ctx.fill_text(label, self.canvas_w / 2.0, y).ok();
            }

            // Last result indicator
            if let Some(was_correct) = last_result {
                let (txt, col) = if *was_correct { ("✓", "#66ff66") } else { ("✗", "#ff6666") };
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
                ("1", "📚 Scholar", "#44aaff", "Balanced. Hints show meaning in combat."),
                ("2", "⚔ Warrior", "#ff6644", "+3 HP, +1 damage, 4 item slots."),
                ("3", "⚗ Alchemist", "#44dd88", "7 item slots, 2x potion healing."),
            ];

            for (key, name, color, desc) in &classes {
                self.ctx.set_fill_style_str(color);
                self.ctx.set_font("22px monospace");
                self.ctx.fill_text(&format!("[{}] {}", key, name), cx, y).ok();
                y += 22.0;
                self.ctx.set_fill_style_str("#999");
                self.ctx.set_font("12px monospace");
                self.ctx.fill_text(desc, cx, y).ok();
                y += 36.0;
            }

            if total_runs == 0 {
                self.ctx.set_fill_style_str("#66ccff");
                self.ctx.set_font("12px monospace");
                self.ctx.fill_text("First run starts with a short tutorial floor.", cx, y).ok();
                y += 24.0;
            }

            y += 10.0;
            self.ctx.set_fill_style_str("#ffcc33");
            self.ctx.set_font("14px monospace");
            self.ctx.fill_text("[D] Daily Challenge (fixed seed)", cx, y).ok();
            y += 24.0;
            self.ctx.set_fill_style_str("#88bbff");
            self.ctx.set_font("12px monospace");
            self.ctx.fill_text("[O] Options", cx, y).ok();
            y += 20.0;
            self.ctx.set_fill_style_str(if knowledge_points_available > 0 { "#aaff88" } else { "#88ddff" });
            self.ctx.fill_text(
                &format!("[T] Talent Tree  ({} KP available)", knowledge_points_available),
                cx,
                y,
            ).ok();
            y += 18.0;
            self.ctx.set_fill_style_str("#8899aa");
            self.ctx.set_font("11px monospace");
            self.ctx.fill_text(
                &format!(
                    "Meta bonuses: +{} HP  -{}% shop  +{} spell",
                    talents.starting_hp_bonus(),
                    talents.shop_discount_pct(),
                    talents.spell_power_bonus()
                ),
                cx,
                y,
            ).ok();
        }

        // ── Game Over overlay ───────────────────────────────────────────
        if matches!(combat, CombatState::GameOver) {
            self.ctx.set_fill_style_str("rgba(0,0,0,0.75)");
            self.ctx
                .fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

            let cx = self.canvas_w / 2.0;
            let mut y = self.canvas_h / 2.0 - 80.0 + (anim_t * 1.7).sin() * 4.0;

            self.ctx.set_fill_style_str("#ff4444");
            self.ctx.set_font("48px monospace");
            self.ctx.set_text_align("center");
            self.ctx.fill_text("GAME OVER", cx, y).ok();
            y += 40.0;

            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("16px monospace");
            self.ctx.fill_text(
                &format!("Floor {} reached  (Best: {})", floor_num, best_floor),
                cx, y,
            ).ok();
            y += 28.0;

            // Stats box
            self.ctx.set_fill_style_str("#ffdd44");
            self.ctx.set_font("13px monospace");
            self.ctx.fill_text(
                &format!("Gold: {}  |  Spells: {}  |  Recipes: {}/{}",
                    player.gold, player.spells.len(), recipes_found, crate::radical::RECIPES.len()),
                cx, y,
            ).ok();
            y += 22.0;

            self.ctx.set_fill_style_str("#88bbff");
            self.ctx.fill_text(
                &format!("Total runs: {}  |  Total kills: {}",
                    total_runs + 1, total_kills),
                cx, y,
            ).ok();
            y += 22.0;

            // SRS accuracy summary
            let total_attempts: u32 = srs.stats.values().map(|(_, t)| t).sum();
            let total_correct: u32 = srs.stats.values().map(|(c, _)| c).sum();
            let pct = if total_attempts > 0 {
                (total_correct as f64 / total_attempts as f64 * 100.0) as u32
            } else { 0 };
            self.ctx.set_fill_style_str("#aaddaa");
            self.ctx.fill_text(
                &format!("Pinyin accuracy: {}% ({}/{})", pct, total_correct, total_attempts),
                cx, y,
            ).ok();
            y += 30.0;

            self.ctx.set_fill_style_str("#ffcc33");
            self.ctx.set_font("14px monospace");
            self.ctx.fill_text("Press R to restart", cx, y).ok();
            y += 20.0;
            self.ctx.set_fill_style_str("#88ddff");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(
                    &format!("[T] Talent Tree  ({} KP available)", knowledge_points_available),
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
                    if settings.screen_shake { "On".to_string() } else { "Off".to_string() },
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
            self.ctx.fill_text("Options / 设置", self.canvas_w / 2.0, box_y + 26.0).ok();

            self.ctx.set_font("14px monospace");
            for (i, (label, value)) in rows.iter().enumerate() {
                let y = box_y + 60.0 + i as f64 * 34.0;
                let selected = i == settings_cursor;
                if selected {
                    self.ctx.set_fill_style_str("rgba(136,187,255,0.16)");
                    self.ctx.fill_rect(box_x + 16.0, y - 16.0, box_w - 32.0, 24.0);
                }
                self.ctx.set_fill_style_str(if selected { "#ffdd88" } else { "#ccd6ff" });
                self.ctx.set_text_align("left");
                self.ctx.fill_text(label, box_x + 24.0, y).ok();
                self.ctx.set_text_align("right");
                self.ctx.fill_text(value, box_x + box_w - 24.0, y).ok();
            }

            self.ctx.set_fill_style_str("#7784aa");
            self.ctx.set_font("11px monospace");
            self.ctx.set_text_align("center");
            self.ctx.fill_text(
                "↑↓ select  ←→ adjust  Enter=cycle/toggle  Esc/O=close",
                self.canvas_w / 2.0,
                box_y + box_h - 16.0,
            ).ok();
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
            self.ctx.fill_text("Talent Tree / 天赋", self.canvas_w / 2.0, box_y + 26.0).ok();

            self.ctx.set_fill_style_str("#aacccc");
            self.ctx.set_font("12px monospace");
            self.ctx.fill_text(
                &format!(
                    "Knowledge Points: {} available / {} earned",
                    knowledge_points_available, knowledge_points_total
                ),
                self.canvas_w / 2.0,
                box_y + 46.0,
            ).ok();
            self.ctx.fill_text(
                &format!(
                    "Progress to next point: {}/{} unique codex entries",
                    knowledge_progress, knowledge_step
                ),
                self.canvas_w / 2.0,
                box_y + 62.0,
            ).ok();

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
                    self.ctx.fill_rect(box_x + 16.0, y - 18.0, box_w - 32.0, 30.0);
                }
                self.ctx.set_text_align("left");
                self.ctx.set_fill_style_str(if selected { "#ffdd88" } else { "#ddeeff" });
                self.ctx.set_font("bold 14px monospace");
                self.ctx.fill_text(TalentTree::title(idx), box_x + 24.0, y).ok();
                self.ctx.set_fill_style_str("#99b8c8");
                self.ctx.set_font("11px monospace");
                self.ctx.fill_text(TalentTree::description(idx), box_x + 24.0, y + 14.0).ok();
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
            self.ctx.fill_text(
                "↑↓ select  Enter=buy rank  Esc/T=close",
                self.canvas_w / 2.0,
                box_y + box_h - 16.0,
            ).ok();
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
        let (highlight, shadow) = if matches!(tile, Tile::Wall | Tile::CrackedWall) {
            ("rgba(255,255,255,0.08)", "rgba(0,0,0,0.32)")
        } else {
            ("rgba(255,255,255,0.06)", "rgba(0,0,0,0.24)")
        };

        self.ctx.set_fill_style_str(highlight);
        self.ctx.fill_rect(screen_x + 0.5, screen_y + 0.5, TILE_SIZE - 1.0, 1.0);
        self.ctx.fill_rect(screen_x + 0.5, screen_y + 1.5, 1.0, TILE_SIZE - 2.0);
        self.ctx.set_fill_style_str(shadow);
        self.ctx
            .fill_rect(screen_x + TILE_SIZE - 1.5, screen_y + 1.5, 1.0, TILE_SIZE - 2.0);
        self.ctx
            .fill_rect(screen_x + 1.5, screen_y + TILE_SIZE - 1.5, TILE_SIZE - 2.0, 1.0);

        match tile {
            Tile::Floor | Tile::Corridor => {
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
                    self.ctx
                        .fill_rect(screen_x + 4.0, screen_y + TILE_SIZE / 2.0 - 0.5, TILE_SIZE - 8.0, 1.0);
                }
            }
            Tile::Wall | Tile::CrackedWall => {
                self.ctx.set_fill_style_str("rgba(0,0,0,0.14)");
                self.ctx
                    .fill_rect(screen_x + 3.0, screen_y + 3.0, TILE_SIZE - 6.0, TILE_SIZE - 6.0);
                self.ctx.set_fill_style_str("rgba(255,255,255,0.07)");
                let seam_y = screen_y + 7.0 + (pattern % 6) as f64;
                self.ctx.fill_rect(screen_x + 3.0, seam_y, TILE_SIZE - 6.0, 1.0);
                let seam_x = screen_x + 7.0 + ((pattern / 5) % 8) as f64;
                self.ctx.fill_rect(seam_x, screen_y + 3.0, 1.0, TILE_SIZE / 2.0 - 1.0);
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
                }
            }
            Tile::Water => {
                let wave_shift = (anim_t * 3.2 + tx as f64 * 0.7 + ty as f64 * 0.4).sin() * 2.0;
                self.ctx.set_fill_style_str("rgba(210,230,255,0.11)");
                self.ctx
                    .fill_rect(screen_x + 3.0 + wave_shift, screen_y + 7.0, TILE_SIZE - 8.0, 1.5);
                self.ctx
                    .fill_rect(screen_x + 5.0 - wave_shift, screen_y + 14.0, TILE_SIZE - 10.0, 1.5);
            }
            Tile::Oil => {
                self.ctx.set_fill_style_str("rgba(255,224,154,0.10)");
                self.ctx
                    .fill_rect(screen_x + 4.0, screen_y + TILE_SIZE - 8.0, TILE_SIZE - 8.0, 2.0);
                self.ctx.set_fill_style_str("rgba(255,255,255,0.06)");
                self.ctx
                    .fill_rect(screen_x + 6.0, screen_y + 6.0, TILE_SIZE - 14.0, 1.5);
            }
            Tile::Crate => {
                self.ctx.set_fill_style_str("rgba(255,225,180,0.08)");
                self.ctx
                    .fill_rect(screen_x + 4.0, screen_y + 4.0, TILE_SIZE - 8.0, TILE_SIZE - 8.0);
                self.ctx.set_fill_style_str("rgba(62,33,14,0.45)");
                self.ctx
                    .fill_rect(screen_x + 8.0, screen_y + 4.0, 1.5, TILE_SIZE - 8.0);
                self.ctx
                    .fill_rect(screen_x + 14.5, screen_y + 4.0, 1.5, TILE_SIZE - 8.0);
            }
            Tile::Spikes => {
                self.ctx.set_fill_style_str("rgba(255,220,220,0.08)");
                self.ctx
                    .fill_rect(screen_x + 4.0, screen_y + TILE_SIZE - 7.0, TILE_SIZE - 8.0, 2.0);
            }
            Tile::Bridge => {
                // Planks
                self.ctx.set_fill_style_str("rgba(160,110,60,0.4)");
                self.ctx.fill_rect(screen_x + 2.0, screen_y + 4.0, TILE_SIZE - 4.0, 4.0);
                self.ctx.fill_rect(screen_x + 2.0, screen_y + 11.0, TILE_SIZE - 4.0, 4.0);
                self.ctx.fill_rect(screen_x + 2.0, screen_y + 18.0, TILE_SIZE - 4.0, 4.0);
                // Nails
                self.ctx.set_fill_style_str("rgba(100,100,100,0.5)");
                self.ctx.fill_rect(screen_x + 4.0, screen_y + 5.0, 1.0, 1.0);
                self.ctx.fill_rect(screen_x + TILE_SIZE - 5.0, screen_y + 5.0, 1.0, 1.0);
                self.ctx.fill_rect(screen_x + 4.0, screen_y + 12.0, 1.0, 1.0);
                self.ctx.fill_rect(screen_x + TILE_SIZE - 5.0, screen_y + 12.0, 1.0, 1.0);
                self.ctx.fill_rect(screen_x + 4.0, screen_y + 19.0, 1.0, 1.0);
                self.ctx.fill_rect(screen_x + TILE_SIZE - 5.0, screen_y + 19.0, 1.0, 1.0);
            }
            Tile::StairsDown
            | Tile::Forge
            | Tile::Shop
            | Tile::Chest
            | Tile::Npc(_)
            | Tile::Shrine
            | Tile::Altar(_)
            | Tile::Seal(_)
            | Tile::Sign(_) => {
                if let Some(plate_fill) = tile_plate_fill(tile) {
                    self.ctx.set_fill_style_str(plate_fill);
                    self.ctx
                        .fill_rect(screen_x + 3.0, screen_y + 3.0, TILE_SIZE - 6.0, TILE_SIZE - 6.0);
                }
            }
        }

        if let Some(accent) = palette.accent {
            self.ctx.set_stroke_style_str(accent);
            self.ctx.set_line_width(1.0);
            self.ctx
                .stroke_rect(screen_x + 1.5, screen_y + 1.5, TILE_SIZE - 3.0, TILE_SIZE - 3.0);
        }

        if let Some(glyph) = palette.glyph {
            self.ctx.set_shadow_color(palette.accent.unwrap_or("transparent"));
            self.ctx.set_shadow_blur(if palette.accent.is_some() { 8.0 } else { 0.0 });
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
        let piety = player.piety.iter().find(|(d, _)| match (d, altar_kind) {
            (crate::player::Deity::Jade, crate::dungeon::AltarKind::Jade) => true,
            (crate::player::Deity::Gale, crate::dungeon::AltarKind::Gale) => true,
            (crate::player::Deity::Mirror, crate::dungeon::AltarKind::Mirror) => true,
            (crate::player::Deity::Iron, crate::dungeon::AltarKind::Iron) => true,
            (crate::player::Deity::Gold, crate::dungeon::AltarKind::Gold) => true,
            _ => false,
        }).map(|(_, p)| *p).unwrap_or(0);

        self.ctx.set_fill_style_str("#ffaa44");
        self.ctx.set_font("bold 16px monospace");
        self.ctx.set_text_align("center");
        self.ctx.fill_text(&format!("Altar of {}", god_name), self.canvas_w / 2.0, box_y + 24.0).ok();

        self.ctx.set_font("12px monospace");
        self.ctx.set_fill_style_str("#ffd700");
        self.ctx.fill_text(&format!("Favor: {}", piety), self.canvas_w / 2.0, box_y + 42.0).ok();

        self.ctx.set_fill_style_str("#aaaaaa");
        self.ctx.fill_text("Select item to offer:", self.canvas_w / 2.0, box_y + 64.0).ok();

        if player.items.is_empty() {
             self.ctx.set_fill_style_str("#888");
             self.ctx.set_font("14px monospace");
             self.ctx.fill_text("(Empty Inventory)", self.canvas_w / 2.0, box_y + 90.0).ok();
        } else {
             for (i, label) in item_labels.iter().enumerate() {
                 let y = box_y + 90.0 + i as f64 * 28.0;
                 let selected = i == cursor;

                 if selected {
                     self.ctx.set_fill_style_str("rgba(255,170,68,0.2)");
                     self.ctx.fill_rect(box_x + 10.0, y - 18.0, box_w - 20.0, 24.0);
                 }

                 self.ctx.set_fill_style_str(if selected { "#ffffff" } else { "#cccccc" });
                 self.ctx.set_font("14px monospace");
                 self.ctx.set_text_align("left");
                 self.ctx.fill_text(label, box_x + 20.0, y).ok();
             }
        }

        // Footer help
        self.ctx.set_text_align("center");
        self.ctx.set_fill_style_str("#7784aa");
        self.ctx.set_font("11px monospace");
        self.ctx.fill_text(
            "Enter=offer  P=pray (cost 20)  Esc=leave",
            self.canvas_w / 2.0,
            box_y + box_h - 12.0,
        ).ok();
    }

    fn draw_dipping_source_overlay(
        &self,
        player: &Player,
        item_labels: &[String],
        cursor: usize,
    ) {
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
        self.ctx.fill_text("Dip what? (Select Potion)", self.canvas_w / 2.0, box_y + 24.0).ok();

        if player.items.is_empty() {
             self.ctx.set_fill_style_str("#888");
             self.ctx.set_font("14px monospace");
             self.ctx.fill_text("(Empty)", self.canvas_w / 2.0, box_y + 50.0).ok();
        } else {
             for (i, label) in item_labels.iter().enumerate() {
                 let y = box_y + 50.0 + i as f64 * 28.0;
                 let selected = i == cursor;

                 if selected {
                     self.ctx.set_fill_style_str("rgba(100,120,200,0.3)");
                     self.ctx.fill_rect(box_x + 10.0, y - 18.0, box_w - 20.0, 24.0);
                 }

                 self.ctx.set_fill_style_str(if selected { "#ffffff" } else { "#aaaaaa" });
                 self.ctx.set_font("14px monospace");
                 self.ctx.set_text_align("left");
                 self.ctx.fill_text(label, box_x + 20.0, y).ok();
             }
        }

        self.ctx.set_text_align("center");
        self.ctx.set_fill_style_str("#7784aa");
        self.ctx.set_font("11px monospace");
        self.ctx.fill_text(
            "Enter=select  Esc=cancel",
            self.canvas_w / 2.0,
            box_y + box_h - 12.0,
        ).ok();
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
        self.ctx.fill_text("Dip into what?", self.canvas_w / 2.0, box_y + 24.0).ok();

        let mut y = box_y + 50.0;

        // Equipment
        let equips = ["Weapon", "Armor", "Charm"];
        for i in 0..3 {
            let selected = cursor == i;
             if selected {
                 self.ctx.set_fill_style_str("rgba(100,120,200,0.3)");
                 self.ctx.fill_rect(box_x + 10.0, y - 18.0, box_w - 20.0, 24.0);
             }
            self.ctx.set_fill_style_str(if selected { "#ffffff" } else { "#aaaaaa" });
            self.ctx.set_font("14px monospace");
            self.ctx.set_text_align("left");
            let name = match i {
                0 => equipment_name(player.weapon, player.enchantments[0]),
                1 => equipment_name(player.armor, player.enchantments[1]),
                _ => equipment_name(player.charm, player.enchantments[2]),
            };
            self.ctx.fill_text(&format!("{}: {}", equips[i], name), box_x + 20.0, y).ok();
            y += 28.0;
        }

        // Items
        if player.items.is_empty() {
             self.ctx.set_fill_style_str("#888");
             self.ctx.fill_text("(Empty Inventory)", box_x + 20.0, y).ok();
        } else {
            for (i, label) in item_labels.iter().enumerate() {
                let display_idx = 3 + i;
                let selected = cursor == display_idx;

                if selected {
                     self.ctx.set_fill_style_str("rgba(100,120,200,0.3)");
                     self.ctx.fill_rect(box_x + 10.0, y - 18.0, box_w - 20.0, 24.0);
                }

                let color = if i == source_idx { "#6688aa" } else if selected { "#ffffff" } else { "#aaaaaa" };
                self.ctx.set_fill_style_str(color);
                self.ctx.set_font("14px monospace");
                self.ctx.set_text_align("left");

                let suffix = if i == source_idx { " (Source)" } else { "" };
                self.ctx.fill_text(&format!("{}{}", label, suffix), box_x + 20.0, y).ok();
                y += 28.0;
            }
        }

        self.ctx.set_text_align("center");
        self.ctx.set_fill_style_str("#7784aa");
        self.ctx.set_font("11px monospace");
        self.ctx.fill_text(
            "Enter=select  Esc=cancel",
            self.canvas_w / 2.0,
            box_y + box_h - 12.0,
        ).ok();
    }

    fn draw_help_overlay(&self, combat: &CombatState, listening_mode: bool) {
        let mut lines = vec![
            "Explore: WASD/Arrows move  1-5 use items".to_string(),
            "I inventory  C codex  V look  O options".to_string(),
            format!(
                "L listening ({})  X skip floor  ? toggle help",
                if listening_mode { "on" } else { "off" }
            ),
        ];

        let mode_title = match combat {
            CombatState::Fighting { .. } => {
                lines.push("Combat: Enter submit  Tab cycle spell  Space cast".to_string());
                lines.push("Esc flee  Elite compounds break one syllable at a time".to_string());
                if listening_mode {
                    lines.push("R replay the heard tone during audio fights".to_string());
                }
                "Combat Controls"
            }
            CombatState::Forging { .. } => {
                lines.push("Forge: 1-9 toggle radicals  <-/-> page".to_string());
                lines.push("Enter forge  E enchant  Esc close".to_string());
                "Forge Controls"
            }
            CombatState::Enchanting { .. } => {
                lines.push("Enchant: 1-3 pick slot  4-9 pick radical".to_string());
                lines.push("<-/-> page  Esc close".to_string());
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
                lines.push("Script seals can flood rooms, raise spikes, or summon ambushes.".to_string());
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
        self.ctx.fill_text(mode_title, box_x + 12.0, box_y + 20.0).ok();
        self.ctx.set_fill_style_str("#8fa8ff");
        self.ctx.set_font("10px monospace");
        self.ctx.fill_text("Help Overlay", box_x + box_w - 92.0, box_y + 20.0).ok();

        self.ctx.set_fill_style_str("#dbe7ff");
        self.ctx.set_font("11px monospace");
        for (idx, line) in lines.iter().enumerate() {
            self.ctx
                .fill_text(line, box_x + 12.0, box_y + 40.0 + idx as f64 * 16.0)
                .ok();
        }
    }

    fn draw_room_ambience(
        &self,
        room_modifier: Option<crate::dungeon::RoomModifier>,
        anim_t: f64,
    ) {
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
                    self.ctx
                        .set_stroke_style_str("rgba(255,90,90,0.22)");
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
        self.ctx.fill_rect(mm_x - 2.0, mm_y - 2.0, mm_w + 4.0, mm_h + 4.0);

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
        self.ctx.fill_text("Inventory", self.canvas_w / 2.0, box_y + 28.0).ok();
        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#9aaad8");
        self.ctx.fill_text("Press I or Esc to close", self.canvas_w / 2.0, box_y + 46.0).ok();

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
        self.ctx.fill_text("Loadout", left_x + 12.0, panel_y + 22.0).ok();
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
        self.ctx.set_fill_style_str("#dde7ff");
        self.ctx
            .fill_text(
                &format!(
                    "Weapon: {}",
                    equipment_name(player.weapon, player.enchantments[0])
                ),
                left_x + 12.0,
                left_y,
            )
            .ok();
        left_y += 16.0;
        self.ctx
            .fill_text(
                &format!(
                    "Armor:  {}",
                    equipment_name(player.armor, player.enchantments[1])
                ),
                left_x + 12.0,
                left_y,
            )
            .ok();
        left_y += 16.0;
        self.ctx
            .fill_text(
                &format!(
                    "Charm:  {}",
                    equipment_name(player.charm, player.enchantments[2])
                ),
                left_x + 12.0,
                left_y,
            )
            .ok();

        left_y += 26.0;
        self.ctx.set_fill_style_str("#9ab0d7");
        self.ctx.fill_text("Consumables", left_x + 12.0, left_y).ok();
        left_y += 18.0;
        self.ctx.set_fill_style_str("#dde7ff");
        if player.items.is_empty() {
            self.ctx
                .fill_text("No consumables picked up yet.", left_x + 12.0, left_y)
                .ok();
            left_y += 16.0;
        } else {
            for (idx, label) in item_labels.iter().enumerate() {
                self.ctx
                    .fill_text(
                        &format!("{}. {}", idx + 1, label),
                        left_x + 12.0,
                        left_y,
                    )
                    .ok();
                left_y += 16.0;
            }
        }

        left_y += 10.0;
        self.ctx.set_fill_style_str("#9ab0d7");
        self.ctx.fill_text("Active effects", left_x + 12.0, left_y).ok();
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
                self.ctx
                    .fill_text(
                        &format!("{} {} {}", marker, spell.hanzi, spell.pinyin),
                        mid_x + 12.0,
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
            for (idx, (radical, count)) in radical_counts.iter().take(rows_per_col * 2).enumerate() {
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
                        &format!("...and {} more stacks", radical_counts.len() - rows_per_col * 2),
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
            .fill_text(
                footer,
                self.canvas_w / 2.0,
                box_y + box_h - 16.0,
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
        self.ctx.fill_text("📖 Character Codex", self.canvas_w / 2.0, 35.0).ok();

        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#aaaaaa");
        self.ctx.fill_text(
            &format!("{} characters encountered — Press C or Esc to close", entries.len()),
            self.canvas_w / 2.0,
            55.0,
        ).ok();

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
            self.ctx.fill_text(&entry.times_seen.to_string(), 450.0, y).ok();

            // Accuracy
            self.ctx.set_fill_style_str(color);
            self.ctx.fill_text(&format!("{:.0}%", acc * 100.0), 520.0, y).ok();
        }

        if entries.len() > max_rows {
            self.ctx.set_font("11px monospace");
            self.ctx.set_text_align("center");
            self.ctx.set_fill_style_str("#666666");
            self.ctx.fill_text(
                &format!("...and {} more", entries.len() - max_rows),
                self.canvas_w / 2.0,
                self.canvas_h - 10.0,
            ).ok();
        }
    }
}

fn hud_message_color(message: &str) -> &'static str {
    if message.starts_with("Wrong") || message.contains(" hits for ") || message.contains("resets!") {
        "#ff7777"
    } else if message.starts_with("⛓") || message.contains("Chain ") || message.contains("Compound broken") {
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
        Tile::Crate | Tile::Spikes | Tile::Oil | Tile::Water => "15px monospace",
        _ => "16px monospace",
    }
}

fn tile_glyph_y(tile: Tile, screen_y: f64, anim_t: f64, tx: i32, ty: i32) -> f64 {
    let base = screen_y + TILE_SIZE * 0.75;
    match tile {
        Tile::Water => base + (anim_t * 3.5 + tx as f64 * 0.6 + ty as f64 * 0.35).sin() * 1.4,
        Tile::Oil => base + (anim_t * 2.0 + tx as f64 * 0.4).sin() * 0.6,
        Tile::Shrine => base + (anim_t * 2.5).sin() * 0.9,
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
