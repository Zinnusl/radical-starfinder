//! Canvas 2D rendering for the dungeon.

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::dungeon::{DungeonLevel, Tile};
use crate::enemy::Enemy;
use crate::game::CombatState;
use crate::player::Player;

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
const COL_FOG: &str = "#0d0b14";
const COL_PLAYER: &str = "#ffcc33";
const COL_PLAYER_OUTLINE: &str = "#bb8800";
const COL_HP_BAR: &str = "#44cc55";
const COL_HP_BG: &str = "#442222";

pub struct Renderer {
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
    ) {
        // Camera: center on player
        let cam_x = player.x as f64 * TILE_SIZE - self.canvas_w / 2.0 + TILE_SIZE / 2.0;
        let cam_y = player.y as f64 * TILE_SIZE - self.canvas_h / 2.0 + TILE_SIZE / 2.0;

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
                let color = if visible {
                    match tile {
                        Tile::Wall => COL_WALL,
                        Tile::Floor => COL_FLOOR,
                        Tile::Corridor => COL_CORRIDOR,
                        Tile::StairsDown => COL_STAIRS,
                        Tile::Forge => COL_FORGE,
                        Tile::Shop => COL_SHOP,
                    }
                } else {
                    // revealed but not currently visible
                    match tile {
                        Tile::Wall => COL_WALL_REVEALED,
                        Tile::Floor | Tile::StairsDown | Tile::Forge | Tile::Shop => COL_FLOOR_REVEALED,
                        Tile::Corridor => COL_CORRIDOR_REVEALED,
                    }
                };

                self.ctx.set_fill_style_str(color);
                self.ctx.fill_rect(screen_x, screen_y, TILE_SIZE, TILE_SIZE);

                // Stairs icon
                if tile == Tile::StairsDown && visible {
                    self.ctx.set_fill_style_str("#ffffff");
                    self.ctx.set_font("16px monospace");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text("▼", screen_x + TILE_SIZE / 2.0, screen_y + TILE_SIZE * 0.75)
                        .ok();
                }

                // Forge icon
                if tile == Tile::Forge && visible {
                    self.ctx.set_fill_style_str("#ffffff");
                    self.ctx.set_font("16px monospace");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text("⚒", screen_x + TILE_SIZE / 2.0, screen_y + TILE_SIZE * 0.75)
                        .ok();
                }

                // Shop icon
                if tile == Tile::Shop && visible {
                    self.ctx.set_fill_style_str("#ffffff");
                    self.ctx.set_font("16px monospace");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text("$", screen_x + TILE_SIZE / 2.0, screen_y + TILE_SIZE * 0.75)
                        .ok();
                }

                // Subtle grid lines for floors
                if visible && tile.is_walkable() {
                    self.ctx.set_stroke_style_str("rgba(255,255,255,0.04)");
                    self.ctx.set_line_width(0.5);
                    self.ctx
                        .stroke_rect(screen_x, screen_y, TILE_SIZE, TILE_SIZE);
                }
            }
        }

        // Draw player
        let px = player.x as f64 * TILE_SIZE - cam_x;
        let py = player.y as f64 * TILE_SIZE - cam_y;
        let center_x = px + TILE_SIZE / 2.0;
        let center_y = py + TILE_SIZE / 2.0;
        let r = TILE_SIZE * 0.38;

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
            let ey = enemy.y as f64 * TILE_SIZE - cam_y;

            // Red/purple glow for alerted/boss enemies
            if enemy.is_boss {
                self.ctx.set_shadow_color("rgba(200,50,255,0.8)");
                self.ctx.set_shadow_blur(14.0);
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
            let font_size = if enemy.is_boss { "22px" } else { "18px" };
            let color = if enemy.is_boss {
                "#cc66ff"
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

            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_shadow_color("transparent");
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

        // Floor indicator + gold (top-right)
        self.ctx.set_text_align("right");
        self.ctx.set_font("14px monospace");
        self.ctx.set_fill_style_str("#aaa");
        self.ctx
            .fill_text(
                &format!("Floor {}  Best: {}", floor_num, best_floor),
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
            self.ctx.set_fill_style_str("#ff8866");
            self.ctx.fill_text(&format!("⚔ {}", w.name), self.canvas_w - 12.0, eq_y).ok();
            eq_y += 14.0;
        }
        if let Some(a) = player.armor {
            self.ctx.set_fill_style_str("#6688ff");
            self.ctx.fill_text(&format!("🛡 {}", a.name), self.canvas_w - 12.0, eq_y).ok();
            eq_y += 14.0;
        }
        if let Some(c) = player.charm {
            self.ctx.set_fill_style_str("#88ddaa");
            self.ctx.fill_text(&format!("✧ {}", c.name), self.canvas_w - 12.0, eq_y).ok();
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

        // Shield indicator
        if player.shield {
            self.ctx.set_font("12px monospace");
            self.ctx.set_text_align("left");
            self.ctx.set_fill_style_str("#44ddff");
            self.ctx.fill_text("🛡 Shield Active", 12.0, 36.0).ok();
        }

        // Minimap (bottom-right)
        self.draw_minimap(level, player);

        // ── Message bar (bottom-center) ─────────────────────────────────
        if !message.is_empty() {
            self.ctx.set_font("14px monospace");
            self.ctx.set_text_align("center");
            self.ctx.set_fill_style_str("rgba(0,0,0,0.7)");
            self.ctx.fill_rect(
                self.canvas_w * 0.15,
                self.canvas_h - 36.0,
                self.canvas_w * 0.7,
                28.0,
            );
            self.ctx.set_fill_style_str("#ffdd88");
            self.ctx
                .fill_text(message, self.canvas_w / 2.0, self.canvas_h - 16.0)
                .ok();
        }

        // ── Combat UI (center overlay when fighting) ────────────────────
        if let CombatState::Fighting { enemy_idx, .. } = combat {
            let enemy_idx = *enemy_idx;
            if enemy_idx < enemies.len() {
                let enemy = &enemies[enemy_idx];
                let box_w = 320.0;
                let box_h = 140.0;
                let box_x = (self.canvas_w - box_w) / 2.0;
                let box_y = 50.0;

                // Background
                self.ctx.set_fill_style_str("rgba(20,10,30,0.92)");
                self.ctx.fill_rect(box_x, box_y, box_w, box_h);
                self.ctx.set_stroke_style_str("#ff6666");
                self.ctx.set_line_width(2.0);
                self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

                // Enemy hanzi (large)
                self.ctx.set_fill_style_str("#ff6666");
                self.ctx.set_font("48px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(enemy.hanzi, self.canvas_w / 2.0, box_y + 52.0)
                    .ok();

                // Meaning hint
                self.ctx.set_fill_style_str("#999");
                self.ctx.set_font("12px monospace");
                self.ctx
                    .fill_text(
                        &format!("({})", enemy.meaning),
                        self.canvas_w / 2.0,
                        box_y + 72.0,
                    )
                    .ok();

                // Typing input box
                let input_y = box_y + 90.0;
                self.ctx.set_fill_style_str("rgba(0,0,0,0.5)");
                self.ctx.fill_rect(box_x + 30.0, input_y, box_w - 60.0, 28.0);
                self.ctx.set_stroke_style_str("#555");
                self.ctx.set_line_width(1.0);
                self.ctx
                    .stroke_rect(box_x + 30.0, input_y, box_w - 60.0, 28.0);

                // Typed text
                let display = if typing.is_empty() {
                    "type pinyin…"
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
                self.ctx
                    .fill_text(
                        "Enter=submit  Esc=flee  Tab=cycle spell  Space=cast spell",
                        self.canvas_w / 2.0,
                        box_y + box_h + 14.0,
                    )
                    .ok();
            }
        }

        // ── Forge UI overlay ─────────────────────────────────────────────
        if let CombatState::Forging { ref selected } = combat {
            let box_w = 380.0;
            let rad_count = player.radicals.len();
            let box_h = 100.0 + (rad_count as f64 / 5.0).ceil() * 36.0;
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
            self.ctx
                .fill_text(
                    "Press 1-9 to toggle radicals, Enter to forge",
                    self.canvas_w / 2.0,
                    box_y + 44.0,
                )
                .ok();

            let grid_y = box_y + 56.0;
            for (i, rad_ch) in player.radicals.iter().enumerate() {
                if i >= 9 { break; } // Only show first 9 (keys 1-9)
                let col = i % 5;
                let row = i / 5;
                let rx = box_x + 20.0 + col as f64 * 72.0;
                let ry = grid_y + row as f64 * 36.0;

                let is_selected = selected.contains(&i);

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
                    .fill_text(&format!("{}", i + 1), rx + 2.0, ry + 11.0)
                    .ok();

                // Radical character
                self.ctx.set_fill_style_str(if is_selected { "#ffcc33" } else { "#ffaa66" });
                self.ctx.set_font("18px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(rad_ch, rx + 32.0, ry + 24.0)
                    .ok();
            }

            // Show selected combo
            if !selected.is_empty() {
                let combo_y = grid_y + ((rad_count.min(9) as f64 / 5.0).ceil()) * 36.0 + 8.0;
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
                let can_afford = player.gold >= item.cost;
                self.ctx.set_fill_style_str(if can_afford { "#ccffcc" } else { "#666" });
                self.ctx.set_font("13px monospace");
                self.ctx.set_text_align("left");
                self.ctx
                    .fill_text(
                        &format!("{} {} — {}g", marker, item.label, item.cost),
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

        // ── Game Over overlay ───────────────────────────────────────────
        if matches!(combat, CombatState::GameOver) {
            self.ctx.set_fill_style_str("rgba(0,0,0,0.7)");
            self.ctx
                .fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

            self.ctx.set_fill_style_str("#ff4444");
            self.ctx.set_font("48px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("GAME OVER", self.canvas_w / 2.0, self.canvas_h / 2.0 - 20.0)
                .ok();

            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("16px monospace");
            self.ctx
                .fill_text(
                    &format!("Reached floor {}  (Best: {})", floor_num, best_floor),
                    self.canvas_w / 2.0,
                    self.canvas_h / 2.0 + 20.0,
                )
                .ok();

            self.ctx.set_fill_style_str("#ffdd44");
            self.ctx.set_font("14px monospace");
            self.ctx
                .fill_text(
                    &format!("Gold earned: {}", player.gold),
                    self.canvas_w / 2.0,
                    self.canvas_h / 2.0 + 44.0,
                )
                .ok();

            self.ctx.set_fill_style_str("#ffcc33");
            self.ctx.set_font("14px monospace");
            self.ctx
                .fill_text(
                    "Press R to restart",
                    self.canvas_w / 2.0,
                    self.canvas_h / 2.0 + 70.0,
                )
                .ok();
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
}
