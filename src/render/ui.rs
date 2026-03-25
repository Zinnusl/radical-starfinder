//! UI overlay rendering: inventory, spellbook, codex, console, and modal overlays.

use crate::game::{CombatState, ListenMode};
use crate::player::{active_set_bonuses, Player, ItemState};
use crate::rarity::ItemRarity;

use super::helpers::{
    equipment_name, equipment_rarity_color, equipment_sprite_key, item_sprite_key,
    radical_stack_counts, spell_school_color, spell_sprite_key, word_wrap,
};

impl super::Renderer {
    pub(crate) fn draw_offering_overlay(
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

    pub(crate) fn draw_dipping_source_overlay(&self, player: &Player, item_labels: &[String], cursor: usize) {
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

    pub(crate) fn draw_dipping_target_overlay(
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
            let (name, rarity) = match i {
                0 => (
                    equipment_name(player.weapon, player.enchantments[0], player.weapon_state, player.weapon_rarity, &player.weapon_affixes),
                    player.weapon_rarity,
                ),
                1 => (
                    equipment_name(player.armor, player.enchantments[1], player.armor_state, player.armor_rarity, &player.armor_affixes),
                    player.armor_rarity,
                ),
                _ => (
                    equipment_name(player.charm, player.enchantments[2], player.charm_state, player.charm_rarity, &player.charm_affixes),
                    player.charm_rarity,
                ),
            };
            if rarity != ItemRarity::Normal {
                self.ctx.set_fill_style_str(equipment_rarity_color(rarity));
            }
            self.ctx
                .fill_text(&format!("{}: {}", equips[i], name), box_x + 20.0, y)
                .ok();
            // Show affix effects below the equipment name
            let affixes: &[crate::rarity::RolledAffix] = match i {
                0 => &player.weapon_affixes,
                1 => &player.armor_affixes,
                _ => &player.charm_affixes,
            };
            if !affixes.is_empty() {
                self.ctx.set_font("11px monospace");
                self.ctx.set_fill_style_str(equipment_rarity_color(rarity));
                let descs: Vec<String> = affixes.iter().map(|a| a.affix.effect.describe()).collect();
                self.ctx
                    .fill_text(&format!("  {}", descs.join(", ")), box_x + 20.0, y + 14.0)
                    .ok();
            }
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

    pub(crate) fn draw_help_overlay(&self, combat: &CombatState, listening_mode: ListenMode) {
        let mut lines = vec![
            "Explore: WASD/Arrows move  1-5 use items".to_string(),
            "I inventory  B spellbook  C codex  T skill tree  U crucible".to_string(),
            "V look  O options".to_string(),
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
                    "Floor {}   HP {}/{}   Credits {}   Class {}",
                    floor_num, player.hp, player.effective_max_hp(), player.gold, class_name
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
            ItemRarity,
            &[crate::rarity::RolledAffix],
        ); 3] = [
            (
                "Weapon",
                player.weapon,
                player.enchantments[0],
                player.weapon_state,
                player.weapon_rarity,
                &player.weapon_affixes,
            ),
            (
                "Armor ",
                player.armor,
                player.enchantments[1],
                player.armor_state,
                player.armor_rarity,
                &player.armor_affixes,
            ),
            (
                "Charm ",
                player.charm,
                player.enchantments[2],
                player.charm_state,
                player.charm_rarity,
                &player.charm_affixes,
            ),
        ];
        for (slot_idx, (label, equip, enchant, state, rarity, affixes)) in equip_slots.iter().enumerate() {
            let selected = inventory_cursor == slot_idx;
            if selected {
                self.ctx.set_fill_style_str("rgba(255,204,51,0.15)");
                self.ctx
                    .fill_rect(left_x + 8.0, left_y - 12.0, left_w - 16.0, 16.0);
            }
            let base_color = if selected { "#00ccdd" } else { "#dde7ff" };
            let color = if *rarity != ItemRarity::Normal {
                equipment_rarity_color(*rarity)
            } else {
                base_color
            };
            self.ctx.set_fill_style_str(color);
            let marker = if selected { "▸" } else { " " };
            self.ctx
                .fill_text(
                    &format!(
                        "{} {}: {}",
                        marker,
                        label,
                        equipment_name(*equip, *enchant, *state, *rarity, affixes)
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

        // ── Equipment set bonuses ──
        let set_bonuses = active_set_bonuses(player);
        if !set_bonuses.is_empty() {
            left_y += 10.0;
            self.ctx.set_fill_style_str("#9ab0d7");
            self.ctx
                .fill_text("Set bonuses", left_x + 12.0, left_y)
                .ok();
            left_y += 18.0;
            for set in &set_bonuses {
                self.ctx.set_fill_style_str("#ffcc33");
                self.ctx
                    .fill_text(
                        &format!("\u{1F4E6} {} \u{2014} {}", set.name, set.bonus_description()),
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

    pub fn draw_console(&self, history: &[String], buffer: &str, scroll_offset: usize) {
        let ctx = &self.ctx;
        let w = self.canvas_w;
        let h = (self.canvas_h * 0.4).max(120.0);

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

        // Blinking cursor: use JS performance.now() for timing
        let blink_on = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| ((p.now() / 530.0) as u64) % 2 == 0)
            .unwrap_or(true);
        let cursor = if blink_on { "_" } else { " " };

        ctx.set_fill_style_str("#39ff14");
        ctx.set_font("bold 14px monospace");
        let _ = ctx.fill_text(&format!("> {}{}", buffer, cursor), 10.0, input_y);

        // Scroll indicator
        if scroll_offset > 0 {
            ctx.set_fill_style_str("#556655");
            ctx.set_font("11px monospace");
            let _ = ctx.fill_text(
                &format!("▲ scroll: {} lines  (PgUp/PgDn)", scroll_offset),
                w - 260.0,
                16.0,
            );
        }

        ctx.set_font(&format!("{}px monospace", font_size));
        let max_lines = ((h - 50.0) / line_height) as usize;
        let total = history.len();
        let end = total.saturating_sub(scroll_offset);
        let start = end.saturating_sub(max_lines);
        for (i, line) in history[start..end].iter().enumerate() {
            let y = input_y - 22.0 - (end - start - 1 - i) as f64 * line_height;
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
                    || line.starts_with("Warped")
                    || line.starts_with("Map revealed")
                    || line.starts_with("Killed")
                    || line.starts_with("HP ")
                    || line.starts_with("Focus ")
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

    pub fn draw_skill_tree(&self, player: &crate::player::Player, cursor: usize) {
        let w = self.canvas_w;
        let h = self.canvas_h;

        // Background overlay
        self.ctx.set_fill_style_str("rgba(0, 0, 0, 0.92)");
        self.ctx.fill_rect(0.0, 0.0, w, h);

        // Title
        self.ctx.set_fill_style_str("#ffcc33");
        self.ctx.set_font("bold 18px monospace");
        self.ctx.set_text_align("center");
        self.ctx.fill_text("⚡ SKILL TREE ⚡", w / 2.0, 30.0).ok();

        // Level + points info
        self.ctx.set_fill_style_str("#aaccff");
        self.ctx.set_font("13px monospace");
        let level = player.skill_tree.level;
        let points = player.skill_tree.skill_points;
        self.ctx
            .fill_text(
                &format!("Level {} — {} skill points available", level, points),
                w / 2.0,
                52.0,
            )
            .ok();

        // XP bar — use xp_for_next_level() for remaining XP
        let level_span = (level + 1) * 100;
        let xp_remaining = player.skill_tree.xp_for_next_level();
        let xp_frac = if level_span > 0 {
            (1.0 - xp_remaining as f64 / level_span as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let bar_w = 200.0;
        let bar_x = w / 2.0 - bar_w / 2.0;
        self.ctx.set_fill_style_str("#222244");
        self.ctx.fill_rect(bar_x, 58.0, bar_w, 6.0);
        self.ctx.set_fill_style_str("#6688cc");
        self.ctx.fill_rect(bar_x, 58.0, bar_w * xp_frac, 6.0);

        // Node list
        self.ctx.set_text_align("left");
        let tree = &crate::skill_tree::SKILL_TREE;
        let nodes = tree.nodes;
        let start_y = 80.0;
        let line_h = 18.0;

        // Calculate visible window
        let max_visible = ((h - start_y - 40.0) / line_h) as usize;
        let scroll_offset = if cursor >= max_visible {
            cursor - max_visible + 1
        } else {
            0
        };

        for (i, node) in nodes.iter().enumerate().skip(scroll_offset) {
            let y = start_y + (i - scroll_offset) as f64 * line_h;
            if y > h - 40.0 {
                break;
            }

            let is_selected = i == cursor;
            let is_allocated = player.skill_tree.allocated[i];
            let can_alloc = player.skill_tree.can_allocate(i);

            // Background highlight for selected
            if is_selected {
                self.ctx
                    .set_fill_style_str("rgba(255, 204, 51, 0.15)");
                self.ctx.fill_rect(20.0, y - 12.0, w - 40.0, line_h);
            }

            // Status indicator
            let status = if is_allocated {
                "●"
            } else if can_alloc {
                "○"
            } else {
                "·"
            };

            // Color based on state
            let color = if is_allocated {
                "#44ff44"
            } else if can_alloc && is_selected {
                "#ffffff"
            } else if can_alloc {
                "#aaaaaa"
            } else {
                "#555555"
            };

            // Cluster color indicator
            let cluster_color = node.cluster.color();
            self.ctx.set_fill_style_str(cluster_color);
            self.ctx.set_font(if node.is_notable {
                "bold 12px monospace"
            } else {
                "12px monospace"
            });
            self.ctx.fill_text(status, 24.0, y).ok();

            // Node name
            self.ctx.set_fill_style_str(color);
            let notable_tag = if node.is_notable { " ★" } else { "" };
            self.ctx
                .fill_text(&format!("{}{}", node.name, notable_tag), 40.0, y)
                .ok();

            // Effect description
            self.ctx.set_fill_style_str(if is_allocated {
                "#88cc88"
            } else {
                "#777777"
            });
            self.ctx.set_font("11px monospace");
            self.ctx.fill_text(node.description, 220.0, y).ok();
        }

        // Footer
        self.ctx.set_fill_style_str("#666688");
        self.ctx.set_font("11px monospace");
        self.ctx.set_text_align("center");
        self.ctx
            .fill_text(
                "↑↓ Navigate • Enter: Allocate • T/Esc: Close",
                w / 2.0,
                h - 12.0,
            )
            .ok();
    }

    pub fn draw_crucible(&self, player: &crate::player::Player, cursor: usize) {
        let w = self.canvas_w;
        let h = self.canvas_h;

        // Background overlay
        self.ctx.set_fill_style_str("rgba(0, 0, 0, 0.92)");
        self.ctx.fill_rect(0.0, 0.0, w, h);

        // Title
        self.ctx.set_fill_style_str("#ff9944");
        self.ctx.set_font("bold 18px monospace");
        self.ctx.set_text_align("center");
        self.ctx
            .fill_text("⚒ Equipment Crucible ⚒", w / 2.0, 30.0)
            .ok();

        // Slot data: (label, equipment name, crucible state)
        let slots: [(&str, Option<&str>, &crate::crucible::CrucibleState); 3] = [
            (
                "Weapon",
                player.weapon.map(|e| e.name),
                &player.weapon_crucible,
            ),
            (
                "Armor",
                player.armor.map(|e| e.name),
                &player.armor_crucible,
            ),
            (
                "Charm",
                player.charm.map(|e| e.name),
                &player.charm_crucible,
            ),
        ];

        self.ctx.set_text_align("left");
        let mut y = 60.0;

        for (slot_idx, (label, equip_name, cruc)) in slots.iter().enumerate() {
            let is_selected = slot_idx == cursor;
            let tmpl = cruc.template();

            // Slot header highlight
            if is_selected {
                self.ctx
                    .set_fill_style_str("rgba(255, 153, 68, 0.15)");
                self.ctx.fill_rect(16.0, y - 14.0, w - 32.0, 20.0);
            }

            // Slot header
            let header_color = if is_selected { "#ffcc33" } else { "#aaaaaa" };
            self.ctx.set_fill_style_str(header_color);
            self.ctx.set_font("bold 14px monospace");
            let name_str = equip_name.unwrap_or("(empty)");
            let header = format!(
                "{}: {} — {} (XP: {})",
                label,
                name_str,
                tmpl.name,
                cruc.xp
            );
            self.ctx.fill_text(&header, 24.0, y).ok();
            y += 20.0;

            // Node list for this slot
            for node_idx in 0..5 {
                let node = &tmpl.nodes[node_idx];
                let unlocked = cruc.unlocked[node_idx];

                // Tree structure prefix
                let prefix = match node_idx {
                    0 | 1 => "├─",
                    2 => if cruc.branch_chosen.is_some() || cruc.pending_branch() {
                        "├─"
                    } else {
                        "└─"
                    },
                    3 => "├← L:",
                    4 => "└→ R:",
                    _ => "  ",
                };

                // Status indicator
                let status = if unlocked { "●" } else { "·" };

                // Color
                let color = if unlocked {
                    "#44ff44"
                } else if node_idx < 3 {
                    "#888888"
                } else {
                    // Branch nodes
                    match cruc.branch_chosen {
                        Some(true) if node_idx == 3 => "#44ff44",
                        Some(false) if node_idx == 4 => "#44ff44",
                        _ if cruc.pending_branch() && is_selected => "#ffcc33",
                        _ => "#555555",
                    }
                };

                self.ctx.set_fill_style_str(color);
                self.ctx.set_font("12px monospace");
                let line = format!(
                    "  {} {} {} — {}",
                    prefix, status, node.name, node.description
                );
                self.ctx.fill_text(&line, 32.0, y).ok();

                // XP cost on the right
                self.ctx.set_fill_style_str("#666688");
                self.ctx.set_font("11px monospace");
                self.ctx.set_text_align("right");
                self.ctx
                    .fill_text(&format!("{}xp", node.xp_cost), w - 28.0, y)
                    .ok();
                self.ctx.set_text_align("left");

                y += 16.0;
            }

            // Branch choice prompt
            if cruc.pending_branch() && is_selected {
                self.ctx.set_fill_style_str("#ffcc33");
                self.ctx.set_font("bold 12px monospace");
                self.ctx
                    .fill_text("  ⚡ Branch ready! ← Left  |  → Right", 32.0, y)
                    .ok();
                y += 16.0;
            } else if let Some(left) = cruc.branch_chosen {
                let chosen = if left { "Left (node 3)" } else { "Right (node 4)" };
                self.ctx.set_fill_style_str("#668866");
                self.ctx.set_font("11px monospace");
                self.ctx
                    .fill_text(&format!("  Branch chosen: {}", chosen), 32.0, y)
                    .ok();
                y += 16.0;
            }

            // XP to next
            if let Some(needed) = cruc.xp_to_next() {
                self.ctx.set_fill_style_str("#666688");
                self.ctx.set_font("11px monospace");
                self.ctx
                    .fill_text(&format!("  Next unlock in {} XP", needed), 32.0, y)
                    .ok();
                y += 16.0;
            }

            y += 10.0;
        }

        // Footer
        self.ctx.set_fill_style_str("#666688");
        self.ctx.set_font("11px monospace");
        self.ctx.set_text_align("center");
        self.ctx
            .fill_text(
                "↑↓ Select slot • ←→ Choose branch • U/Esc: Close",
                w / 2.0,
                h - 12.0,
            )
            .ok();
    }

    pub fn draw_dungeon_dialogue(
        &self,
        dialogue: &crate::world::dialogue::DungeonDialogue,
        cursor: usize,
        player: &crate::player::Player,
    ) {
        let box_w = 520.0_f64.min(self.canvas_w - 40.0);
        let box_x = (self.canvas_w - box_w) / 2.0;
        let box_y = 40.0;
        let inner_w = box_w - 24.0;
        // 13px monospace ≈ 7.8px per char
        let max_chars = (inner_w / 7.8) as usize;
        // 16px bold monospace ≈ 9.6px per char
        let title_max = (inner_w / 9.6) as usize;

        let title_text = format!("\u{26a1} {} \u{2014} {}", dialogue.title, dialogue.chinese_title);
        let title_lines = word_wrap(&title_text, title_max);
        let desc_lines = word_wrap(dialogue.description, max_chars);

        // Pre-wrap all choice texts so we can compute total height
        let choice_lines: Vec<Vec<String>> = dialogue.choices.iter()
            .map(|c| word_wrap(&c.text, max_chars.saturating_sub(2)))
            .collect();

        let title_h = title_lines.len() as f64 * 20.0;
        let desc_h = desc_lines.len() as f64 * 18.0;
        let choices_h: f64 = choice_lines.iter()
            .map(|lines| lines.len() as f64 * 16.0 + 20.0) // text lines + hint + gap
            .sum();
        let box_h = (20.0 + title_h + 8.0 + desc_h + 16.0 + choices_h + 12.0)
            .min(self.canvas_h - 80.0);

        // Background
        self.ctx.set_fill_style_str("rgba(8,6,18,0.97)");
        self.ctx.fill_rect(box_x, box_y, box_w, box_h);
        self.ctx.set_stroke_style_str("rgba(100,200,255,0.6)");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(box_x, box_y, box_w, box_h);

        // Title (word-wrapped)
        self.ctx.set_fill_style_str("#64c8ff");
        self.ctx.set_font("bold 16px monospace");
        self.ctx.set_text_align("left");
        let mut y = box_y + 24.0;
        for line in &title_lines {
            let _ = self.ctx.fill_text(line, box_x + 12.0, y);
            y += 20.0;
        }
        y += 8.0;

        // Description (word-wrapped)
        self.ctx.set_fill_style_str("#c0c0c0");
        self.ctx.set_font("13px monospace");
        for line in &desc_lines {
            let _ = self.ctx.fill_text(line, box_x + 12.0, y);
            y += 18.0;
        }
        y += 16.0;

        // Choices (each choice word-wrapped, with hint below)
        for (i, choice) in dialogue.choices.iter().enumerate() {
            let is_selected = i == cursor;
            let meets_req = crate::game::dungeon_events::meets_dungeon_requirement(player, &choice.requires);
            let prefix = if is_selected { "\u{25b8} " } else { "  " };

            if !meets_req {
                self.ctx.set_fill_style_str("#553333");
            } else if is_selected {
                self.ctx.set_fill_style_str("#ffcc00");
            } else {
                self.ctx.set_fill_style_str("#a0a0a0");
            }
            self.ctx.set_font("13px monospace");
            for (j, line) in choice_lines[i].iter().enumerate() {
                let pfx = if j == 0 { prefix } else { "  " };
                let _ = self.ctx.fill_text(
                    &format!("{}{}", pfx, line),
                    box_x + 12.0,
                    y,
                );
                y += 16.0;
            }

            // Chinese hint below choice
            self.ctx.set_fill_style_str("rgba(100,200,255,0.5)");
            self.ctx.set_font("11px monospace");
            let _ = self.ctx.fill_text(
                &format!("    {}", choice.chinese_hint),
                box_x + 12.0,
                y,
            );
            y += 20.0;
        }
    }
}