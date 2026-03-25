//! Tactical battle (combat arena) rendering.

use crate::combat::TacticalBattle;
use crate::player::Player;

use super::FloatingText;

mod grid;
mod panels;
mod effects;

/// Extract a floating damage/heal number from a log message.
fn parse_float_from_log(msg: &str) -> Option<(String, String, bool)> {
    let lower = msg.to_lowercase();
    let is_critical = lower.contains("critical") || lower.contains("crit!");
    if is_critical {
        return Some(("CRITICAL!".to_string(), "#ffd700".to_string(), true));
    }
    // Extract numeric value from message
    let num: Option<String> = {
        let mut digits = String::new();
        let mut found = false;
        for ch in msg.chars() {
            if ch.is_ascii_digit() {
                digits.push(ch);
                found = true;
            } else if found {
                break;
            }
        }
        if found && !digits.is_empty() { Some(digits) } else { None }
    };
    if lower.contains("damage") || lower.contains("hit") || lower.contains("attacks") {
        let text = num.map(|n| format!("-{}", n)).unwrap_or_else(|| "HIT".to_string());
        Some((text, "#ff6644".to_string(), false))
    } else if lower.contains("heal") || lower.contains("restore") || lower.contains("regen") {
        let text = num.map(|n| format!("+{}", n)).unwrap_or_else(|| "HEAL".to_string());
        Some((text, "#44dd66".to_string(), false))
    } else if lower.contains("shield") || lower.contains("block") {
        Some(("BLOCK".to_string(), "#6688ff".to_string(), false))
    } else if lower.contains("miss") || lower.contains("dodge") {
        Some(("MISS".to_string(), "#aaaaaa".to_string(), false))
    } else {
        None
    }
}

impl super::Renderer {
    pub(crate) fn draw_tactical_battle(&self, battle: &TacticalBattle, anim_t: f64, _player: &Player) {
        let grid_size = battle.arena.width as f64;
        let max_grid_px = (self.canvas_h - 80.0).min(self.canvas_w * 0.55);
        let cell = (max_grid_px / grid_size).floor().max(24.0).min(36.0);
        let grid_px = grid_size * cell;
        let grid_x = (self.canvas_w - grid_px) / 2.0;
        let grid_y = 30.0;

        // Full-screen dark backdrop
        self.ctx.set_fill_style_str("rgba(10,6,18,0.94)");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

        // ── Floating damage text lifecycle ──
        {
            let prev_len = self.last_log_len.get();
            let cur_len = battle.log.len();
            // New battle detected (log reset)
            if cur_len < prev_len {
                self.combat_floats.borrow_mut().clear();
                self.last_log_len.set(0);
            }
            // Spawn floats for new log entries
            if cur_len > prev_len {
                let mut floats = self.combat_floats.borrow_mut();
                for (idx, msg) in battle.log[prev_len..].iter().enumerate() {
                    if let Some((text, color, is_critical)) = parse_float_from_log(msg) {
                        // Position near a unit mentioned in the message
                        let (fx, fy) = {
                            let mut pos = (grid_x + grid_px / 2.0, grid_y + grid_px * 0.35);
                            for unit in &battle.units {
                                if unit.alive && !unit.hanzi.is_empty() && msg.contains(unit.hanzi) {
                                    pos = (
                                        grid_x + unit.x as f64 * cell + cell / 2.0,
                                        grid_y + unit.y as f64 * cell,
                                    );
                                    break;
                                }
                            }
                            pos
                        };
                        let jitter_x = ((idx as f64 * 17.3 + cur_len as f64 * 7.1) % 30.0) - 15.0;
                        floats.push(FloatingText {
                            x: fx + jitter_x,
                            y: fy,
                            text,
                            color,
                            spawn_time: anim_t,
                            is_critical,
                        });
                    }
                }
                self.last_log_len.set(cur_len);
            }
            // Remove expired floats
            self.combat_floats.borrow_mut().retain(|f| anim_t - f.spawn_time < 2.0);
        }

        // Grid, terrain, units, projectiles
        self.draw_tactical_grid(battle, anim_t, _player, cell, grid_px, grid_x, grid_y, grid_size);

        // Right panel, menus, look mode
        self.draw_tactical_panels(battle, anim_t, cell, grid_px, grid_x, grid_y, grid_size, _player);

        // Banners, messages, typing UI, battle log
        self.draw_tactical_effects(battle, anim_t, cell, grid_px, grid_x, grid_y, grid_size);

        // ── Render floating damage/heal numbers ──
        {
            let floats = self.combat_floats.borrow();
            for float in floats.iter() {
                let age = anim_t - float.spawn_time;
                let duration = 1.8;
                if age > duration { continue; }
                let progress = age / duration;
                let rise = progress * 40.0;
                let alpha = (1.0 - progress * progress).max(0.0);
                let base_size: f64 = if float.is_critical { 18.0 } else { 14.0 };
                let scale = if float.is_critical { 1.0 + progress * 0.3 } else { 1.0 };
                let font_size = (base_size * scale) as u32;

                self.ctx.set_global_alpha(alpha);
                if float.is_critical {
                    self.ctx.set_shadow_color(&float.color);
                    self.ctx.set_shadow_blur(8.0);
                }
                self.ctx.set_fill_style_str(&float.color);
                self.ctx.set_font(&format!("bold {}px monospace", font_size));
                self.ctx.set_text_align("center");
                self.ctx.fill_text(&float.text, float.x, float.y - rise).ok();
                self.ctx.set_shadow_blur(0.0);
                self.ctx.set_shadow_color("transparent");
                self.ctx.set_global_alpha(1.0);
            }
            self.ctx.set_text_align("left");
        }
    }
}
