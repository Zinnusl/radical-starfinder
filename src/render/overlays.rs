//! Screen overlay rendering (forge, shop, enchanting, challenges, game over).

use super::*;

impl Renderer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_overlays(
        &self,
        combat: &CombatState,
        player: &Player,
        anim_t: f64,
        typing: &str,
        floor_num: i32,
        best_floor: i32,
        total_kills: u32,
        total_runs: u32,
        recipes_found: usize,
        srs: &crate::srs::SrsTracker,
        settings: &GameSettings,
        show_settings: bool,
        settings_cursor: usize,
        codex: &crate::codex::Codex,
        run_journal: &crate::game::RunJournal,
        post_mortem_page: usize,
        class_cursor: usize,
        item_labels: &[String],
        shop_sell_mode: bool,
        answer_streak: u32,
        companion: Option<crate::game::Companion>,
        companion_level: u8,
        location_label: &str,
    ) {
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
    }
}
