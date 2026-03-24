//! Right info panel, action menus, and look mode display.

use crate::combat::{
    ArenaBiome, BattleTile, Direction, EnemyIntent, Projectile, TacticalBattle, TacticalPhase,
    TargetMode, TypingAction, Weather, WuxingElement,
};
use crate::player::Player;
use crate::radical;

use super::super::{COL_PLAYER, COL_HP_BAR, COL_HP_BG};

impl super::super::Renderer {
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn draw_tactical_panels(
        &self,
        battle: &TacticalBattle,
        anim_t: f64,
        cell: f64,
        grid_px: f64,
        grid_x: f64,
        grid_y: f64,
        grid_size: f64,
    ) {
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
        // Inner glow border
        self.ctx.set_stroke_style_str("rgba(100,80,160,0.15)");
        self.ctx.set_line_width(1.0);
        self.ctx
            .stroke_rect(panel_x - 5.5, py - 3.5, panel_w + 7.0, grid_px + 7.0);
        // Top accent line
        self.ctx.set_fill_style_str("rgba(0,204,221,0.25)");
        self.ctx.fill_rect(panel_x - 6.0, py - 4.0, panel_w + 8.0, 1.5);

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
        // Low HP glow effect
        if p_hp_frac < 0.3 {
            self.ctx.set_shadow_color("rgba(255,50,50,0.5)");
            self.ctx.set_shadow_blur(4.0);
            self.ctx.set_fill_style_str(panel_hp_color);
            self.ctx
                .fill_rect(panel_x, py, p_bar_w * p_hp_frac, hp_bar_h);
            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_shadow_color("transparent");
        }
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
            // Section header underline
            self.ctx.set_fill_style_str("rgba(0,204,221,0.15)");
            self.ctx.fill_rect(panel_x, py + 12.0, p_bar_w, 1.0);
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
                    // Darker inner background
                    self.ctx.set_fill_style_str("rgba(0,204,221,0.06)");
                    self.ctx
                        .fill_rect(panel_x - 2.0, py - 1.0, p_bar_w + 2.0, 14.0);
                    // Left accent bar
                    self.ctx.set_fill_style_str("rgba(0,204,221,0.5)");
                    self.ctx.fill_rect(panel_x - 2.0, py - 1.0, 2.0, 14.0);
                    // Hotkey in cyan, label in light
                    self.ctx.set_fill_style_str("#00ccdd");
                    self.ctx
                        .fill_text(&format!("[{}]", hotkey), panel_x, py + 10.0)
                        .ok();
                    self.ctx.set_fill_style_str("#dde0e8");
                    self.ctx
                        .fill_text(&format!(" {}", label), panel_x + 22.0, py + 10.0)
                        .ok();
                } else {
                    // Disabled: dim text, no accent
                    self.ctx.set_fill_style_str("#333");
                    self.ctx
                        .fill_rect(panel_x - 2.0, py - 1.0, p_bar_w + 2.0, 14.0);
                    self.ctx.set_fill_style_str("#555");
                    self.ctx
                        .fill_text(&format!("[{}] {}", hotkey, label), panel_x, py + 10.0)
                        .ok();
                }
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
            let spell_menu_w = panel_w.min(170.0);
            let spell_menu_h = 16.0 + battle.available_spells.len() as f64 * 18.0 + 20.0;
            self.ctx.set_fill_style_str("rgba(0,0,0,0.6)");
            self.ctx.fill_rect(
                panel_x - 4.0,
                py,
                spell_menu_w,
                spell_menu_h,
            );
            // Menu border
            self.ctx.set_stroke_style_str("rgba(100,80,160,0.3)");
            self.ctx.set_line_width(1.0);
            self.ctx
                .stroke_rect(panel_x - 4.0, py, spell_menu_w, spell_menu_h);
            // Header accent
            self.ctx.set_fill_style_str("rgba(0,204,221,0.2)");
            self.ctx.fill_rect(panel_x - 4.0, py, spell_menu_w, 1.5);
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
            let item_menu_w = panel_w.min(170.0);
            let item_menu_h = 16.0 + battle.available_items.len() as f64 * 18.0 + 20.0;
            self.ctx.set_fill_style_str("rgba(0,0,0,0.6)");
            self.ctx.fill_rect(
                panel_x - 4.0,
                py,
                item_menu_w,
                item_menu_h,
            );
            // Menu border
            self.ctx.set_stroke_style_str("rgba(100,80,160,0.3)");
            self.ctx.set_line_width(1.0);
            self.ctx
                .stroke_rect(panel_x - 4.0, py, item_menu_w, item_menu_h);
            // Header accent
            self.ctx.set_fill_style_str("rgba(0,204,221,0.2)");
            self.ctx.fill_rect(panel_x - 4.0, py, item_menu_w, 1.5);
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
            let skill_menu_w = panel_w.min(200.0);
            let skill_menu_h = 16.0 + count as f64 * 18.0 + 36.0;
            self.ctx.set_fill_style_str("rgba(0,0,0,0.6)");
            self.ctx.fill_rect(
                panel_x - 4.0,
                py,
                skill_menu_w,
                skill_menu_h,
            );
            // Menu border
            self.ctx.set_stroke_style_str("rgba(100,80,160,0.3)");
            self.ctx.set_line_width(1.0);
            self.ctx
                .stroke_rect(panel_x - 4.0, py, skill_menu_w, skill_menu_h);
            // Header accent
            self.ctx.set_fill_style_str("rgba(0,204,221,0.2)");
            self.ctx.fill_rect(panel_x - 4.0, py, skill_menu_w, 1.5);
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
            // Section header underline
            self.ctx.set_fill_style_str("rgba(0,204,221,0.15)");
            self.ctx.fill_rect(panel_x, py + 12.0, p_bar_w, 1.0);
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
                                // Section header underline
                                self.ctx.set_fill_style_str("rgba(0,204,221,0.15)");
                                self.ctx.fill_rect(panel_x, py + 12.0, p_bar_w, 1.0);
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
    }
}
