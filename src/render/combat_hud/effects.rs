//! Combat effects, messages, battle log, and typing UI.

use crate::combat::{
    TacticalBattle, TacticalPhase,
};


impl super::super::Renderer {
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn draw_tactical_effects(
        &self,
        battle: &TacticalBattle,
        anim_t: f64,
        _cell: f64,
        grid_px: f64,
        grid_x: f64,
        grid_y: f64,
        _grid_size: f64,
    ) {

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

        // Battle log (bottom area) — styled with gradient background
        let log_x = grid_x;
        let log_y = grid_y + grid_px + 8.0;
        let log_w = grid_px;
        let log_h = self.canvas_h - log_y - 8.0;
        let line_h = 14.0;
        let max_lines = ((log_h - 10.0) / line_h).floor() as usize;

        // Gradient background: darker at bottom, slightly lighter at top
        for gi in 0..5 {
            let gy_off = gi as f64 * (log_h / 5.0);
            let alpha = 0.45 + gi as f64 * 0.07;
            self.ctx
                .set_fill_style_str(&format!("rgba(8,6,16,{:.3})", alpha));
            self.ctx.fill_rect(log_x, log_y + gy_off, log_w, log_h / 5.0);
        }
        // Accent line at top of log (cyan)
        self.ctx.set_fill_style_str("rgba(0,204,221,0.2)");
        self.ctx.fill_rect(log_x, log_y, log_w, 1.5);
        self.ctx.set_stroke_style_str("rgba(100,80,140,0.25)");
        self.ctx.set_line_width(1.0);
        self.ctx.stroke_rect(log_x, log_y, log_w, log_h);

        // "LOG T{turn}" label with turn indicator
        self.ctx.set_fill_style_str("rgba(120,100,160,0.5)");
        self.ctx.set_font("bold 8px monospace");
        self.ctx.set_text_align("right");
        self.ctx
            .fill_text(
                &format!("LOG · T{}", battle.turn_number),
                log_x + log_w - 4.0,
                log_y + 10.0,
            )
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
            // Alternating row backgrounds
            if i % 2 == 0 {
                self.ctx.set_fill_style_str("rgba(255,255,255,0.02)");
                self.ctx.fill_rect(
                    log_x + 1.0,
                    log_y + 4.0 + i as f64 * line_h,
                    log_w - 2.0,
                    line_h,
                );
            }
            // Left accent bar by message type (expanded categories)
            let lower = msg.to_lowercase();
            let accent_color =
                if lower.contains("damage") || lower.contains("hit") || lower.contains("kill") || lower.contains("attack") {
                    "rgba(255,80,60,0.6)"
                } else if lower.contains("heal") || lower.contains("restore") || lower.contains("regen") {
                    "rgba(60,220,60,0.6)"
                } else if lower.contains("move") || lower.contains("walk") || lower.contains("dash") {
                    "rgba(80,140,255,0.5)"
                } else if lower.contains("ability") || lower.contains("cast") || lower.contains("spell") {
                    "rgba(140,100,255,0.6)"
                } else if lower.contains("status") || lower.contains("poison") || lower.contains("burn") || lower.contains("stun") {
                    "rgba(220,200,60,0.5)"
                } else {
                    "rgba(100,100,120,0.3)"
                };
            self.ctx.set_fill_style_str(accent_color);
            self.ctx.fill_rect(
                log_x + 1.0,
                log_y + 4.0 + i as f64 * line_h,
                2.5,
                line_h - 1.0,
            );
            // Text color by message type (expanded)
            let color = if lower.contains("damage") || lower.contains("hit") || lower.contains("kill") || lower.contains("attack") {
                format!("rgba(255,130,100,{})", alpha)
            } else if lower.contains("heal") || lower.contains("restore") || lower.contains("regen") {
                format!("rgba(100,220,100,{})", alpha)
            } else if lower.contains("move") || lower.contains("walk") || lower.contains("dash") {
                format!("rgba(120,170,255,{})", alpha * 0.9)
            } else if lower.contains("ability") || lower.contains("cast") || lower.contains("spell") {
                format!("rgba(160,140,255,{})", alpha)
            } else if lower.contains("status") || lower.contains("poison") || lower.contains("burn") || lower.contains("stun") {
                format!("rgba(220,210,100,{})", alpha)
            } else {
                format!("rgba(180,175,190,{})", alpha)
            };
            self.ctx.set_fill_style_str(&color);
            self.ctx
                .fill_text(msg, log_x + 10.0, log_y + 12.0 + i as f64 * line_h)
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
                let pulse = ((anim_t * 3.0).sin() * 0.3 + intensity).clamp(0.1, 1.0);
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
}
