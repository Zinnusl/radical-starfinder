//! Input handling for tactical combat phases.
//!
//! All keyboard input during a \TacticalBattle\ flows through
//! \handle_input()\ which dispatches based on the current \TacticalPhase\.

mod resolution;
mod targeting;
mod typing;

pub use resolution::{spell_combo_name, spell_effect_element};

use crate::combat::action::{deal_damage, deal_damage_from, defend, move_unit, wait};
use crate::combat::ai::{choose_action, choose_companion_action, AiAction};
use crate::combat::boss;
use crate::combat::grid::{tiles_in_range_with_los, weather_adjusted_range};
use crate::combat::radical::apply_radical_action;
use crate::combat::terrain::apply_knockback;
use crate::combat::turn::advance_turn;
use crate::combat::{
    AudioEvent, TacticalBattle, TacticalPhase, TargetMode, TypingAction,
};
use crate::radical::SpellEffect;
use crate::status::{tick_statuses, StatusInstance, StatusKind};

/// Events that the game loop must handle after input processing.
#[derive(Clone, Debug)]
pub enum BattleEvent {
    /// Nothing special — just re-render.
    None,
    /// Player chose to flee.
    Flee,
    /// Battle ended in victory.
    Victory,
    /// Player died.
    Defeat,
}

/// Main entry point: process one key press in tactical battle.
/// Returns a `BattleEvent` that `game.rs` should react to.
pub fn handle_input(battle: &mut TacticalBattle, key: &str) -> BattleEvent {
    // Don't accept input during enemy turns or resolve animations.
    match &battle.phase {
        TacticalPhase::EnemyTurn { .. } | TacticalPhase::Resolve { .. } => {
            return BattleEvent::None;
        }
        TacticalPhase::End { victory, timer, .. } => {
            if *timer == 0 {
                return if *victory {
                    BattleEvent::Victory
                } else {
                    BattleEvent::Defeat
                };
            }
            return BattleEvent::None;
        }
        _ => {}
    }

    // If spell menu is open, route to spell menu handler.
    if battle.spell_menu_open {
        return handle_spell_menu(battle, key);
    }

    // If item menu is open, route to item menu handler.
    if battle.item_menu_open {
        return handle_item_menu(battle, key);
    }

    // If skill menu is open, route to skill menu handler.
    if battle.skill_menu_open {
        return handle_skill_menu(battle, key);
    }

    // If radical picker is open, route to radical picker handler.
    if battle.radical_picker_open {
        return handle_radical_picker(battle, key);
    }

    // If we're in a typing action, route to typing handler.
    if battle.typing_action.is_some() {
        return typing::handle_typing(battle, key);
    }

    match &battle.phase {
        TacticalPhase::Command => handle_command(battle, key),
        TacticalPhase::Targeting { .. } => targeting::handle_targeting(battle, key),
        TacticalPhase::Look { .. } => handle_look(battle, key),
        TacticalPhase::Deployment { .. } => handle_deployment(battle, key),
        _ => BattleEvent::None,
    }
}

// ── Command phase ────────────────────────────────────────────────────────────

fn handle_command(battle: &mut TacticalBattle, key: &str) -> BattleEvent {
    if battle.units[0]
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Freeze))
    {
        battle.log_message("You are frozen solid! Turn skipped.");
        battle.player_acted = true;
        battle.player_moved = true;
        return try_end_player_turn(battle);
    }

    match key {
        "m" | "M" if !battle.player_moved => {
            targeting::enter_move_targeting(battle);
            BattleEvent::None
        }
        "a" | "A" if !battle.player_acted => {
            if battle.units[0]
                .statuses
                .iter()
                .any(|s| matches!(s.kind, crate::status::StatusKind::Fear))
            {
                battle.log_message("You're too afraid to attack!");
                return BattleEvent::None;
            }
            if battle.player_radical_abilities.is_empty() {
                targeting::enter_attack_targeting(battle);
            } else {
                battle.radical_picker_open = true;
                battle.radical_picker_cursor = 0;
            }
            BattleEvent::None
        }
        "s" | "S" if !battle.player_acted => {
            if !battle.player_stance.can_cast_spells() {
                battle.log_message("Can't cast spells in Mobile stance!");
                return BattleEvent::None;
            }
            if battle.units[0]
                .statuses
                .iter()
                .any(|s| matches!(s.kind, crate::status::StatusKind::Fear))
            {
                battle.log_message("Fear grips you! Cannot cast spells!");
                return BattleEvent::None;
            }
            if battle.available_spells.is_empty() {
                battle.log_message("No spells available.");
            } else {
                battle.spell_menu_open = true;
                battle.spell_cursor = 0;
            }
            BattleEvent::None
        }
        "k" | "K" if !battle.player_acted => {
            if battle.player_radical_abilities.is_empty() {
                battle.log_message("No skills available.");
            } else {
                battle.skill_menu_open = true;
                battle.skill_menu_cursor = 0;
            }
            BattleEvent::None
        }
        "d" | "D" if !battle.player_acted => {
            let idx = battle.current_unit_idx();
            defend(battle, idx);
            battle.player_acted = true;
            battle.phase = TacticalPhase::Resolve {
                message: "Defending.".to_string(),
                timer: 20,
                end_turn: true,
            };
            BattleEvent::None
        }
        "w" | "W" => {
            let idx = battle.current_unit_idx();
            wait(battle, idx);
            battle.player_acted = true;
            battle.player_moved = true;
            battle.phase = TacticalPhase::Resolve {
                message: "Waiting...".to_string(),
                timer: 15,
                end_turn: true,
            };
            BattleEvent::None
        }
        "r" | "R" => {
            battle.units[0].facing = battle.units[0].facing.rotate_cw();
            let dir_name = match battle.units[0].facing {
                crate::combat::Direction::North => "North",
                crate::combat::Direction::South => "South",
                crate::combat::Direction::East => "East",
                crate::combat::Direction::West => "West",
            };
            battle.log_message(format!("Facing {}.", dir_name));
            BattleEvent::None
        }
        "i" | "I" if !battle.player_acted => {
            if battle.available_items.is_empty() {
                battle.log_message("No items available.");
            } else {
                battle.item_menu_open = true;
                battle.item_cursor = 0;
            }
            BattleEvent::None
        }
        "f" | "F" => {
            let new_stance = battle.player_stance.next();
            battle.player_stance = new_stance;
            battle.log_message(format!(
                "Stance: {} — {}",
                new_stance.name(),
                new_stance.description()
            ));
            BattleEvent::None
        }
        "v" | "V" | "l" | "L" => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.phase = TacticalPhase::Look {
                cursor_x: px,
                cursor_y: py,
            };
            BattleEvent::None
        }
        "Escape" => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let w = battle.arena.width as i32;
            let h = battle.arena.height as i32;
            let on_edge = px == 0 || py == 0 || px == w - 1 || py == h - 1;
            if !on_edge {
                battle.log_message("Must be on arena edge to flee!");
                return BattleEvent::None;
            }
            let mut chance: i32 = 60;
            if let Some(class) = battle.player_class {
                if matches!(
                    class,
                    crate::player::PlayerClass::Operative
                ) {
                    chance += 20;
                }
            }
            let mut min_dist = i32::MAX;
            for u in &battle.units[1..] {
                if u.alive {
                    let d = (u.x - px).abs() + (u.y - py).abs();
                    if d < min_dist {
                        min_dist = d;
                    }
                }
            }
            if min_dist < i32::MAX {
                chance += (min_dist * 10).min(30);
            }
            chance = chance.min(95);
            let roll = ((battle.turn_number as u64).wrapping_mul(2654435761) % 100) as i32;
            if roll < chance {
                let adj = battle.adjacent_enemies(px, py);
                for eidx in &adj {
                    if battle.units[*eidx].alive {
                        let dmg = battle.units[*eidx].damage;
                        let actual = deal_damage(battle, 0, dmg);
                        battle.log_message(format!("Free attack! {} damage as you flee!", actual));
                    }
                }
                if battle.units[0].hp <= 0 {
                    battle.units[0].alive = false;
                    battle.phase = TacticalPhase::End {
                        victory: false,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
                battle.log_message("Escaped!");
                BattleEvent::Flee
            } else {
                battle.log_message(format!("Failed to flee! ({}% chance)", chance));
                battle.player_acted = true;
                battle.player_moved = true;
                battle.phase = TacticalPhase::Resolve {
                    message: "Failed to flee!".to_string(),
                    timer: 20,
                    end_turn: true,
                };
                BattleEvent::None
            }
        }
        _ => BattleEvent::None,
    }
}

fn handle_look(battle: &mut TacticalBattle, key: &str) -> BattleEvent {
    let (cx, cy) = match battle.phase {
        TacticalPhase::Look { cursor_x, cursor_y } => (cursor_x, cursor_y),
        _ => return BattleEvent::None,
    };

    let (dx, dy) = match key {
        "ArrowUp" | "w" => (0, -1),
        "ArrowDown" | "s" => (0, 1),
        "ArrowLeft" | "a" => (-1, 0),
        "ArrowRight" | "d" => (1, 0),
        "Escape" | "v" | "V" | "l" | "L" | "Enter" | " " => {
            battle.phase = TacticalPhase::Command;
            return BattleEvent::None;
        }
        _ => return BattleEvent::None,
    };

    let nx = cx + dx;
    let ny = cy + dy;
    if nx >= 0
        && ny >= 0
        && (nx as usize) < battle.arena.width
        && (ny as usize) < battle.arena.height
    {
        battle.phase = TacticalPhase::Look {
            cursor_x: nx,
            cursor_y: ny,
        };
        if let Some(tile) = battle.arena.tile(nx, ny) {
            let mut desc = tile.description().to_string();
            let mut log_msgs = Vec::new();
            for unit in &battle.units {
                if unit.alive && unit.x == nx && unit.y == ny {
                    if unit.is_player() {
                        desc.push_str(" | You");
                    } else {
                        let name = if unit.hanzi.is_empty() {
                            "Enemy"
                        } else {
                            unit.hanzi
                        };
                        desc.push_str(&format!(" | {} HP:{}/{}", name, unit.hp, unit.max_hp));

                        let ai_behavior = match unit.ai {
                            crate::enemy::AiBehavior::Chase => "Chase",
                            crate::enemy::AiBehavior::Retreat => "Retreat",
                            crate::enemy::AiBehavior::Ambush => "Ambush",
                            crate::enemy::AiBehavior::Sentinel => "Sentinel",
                            crate::enemy::AiBehavior::Kiter => "Kiter",
                            crate::enemy::AiBehavior::Pack => "Pack",
                        };
                        desc.push_str(&format!(" | AI: {}", ai_behavior));

                        if !unit.radical_actions.is_empty() {
                            let mut by_radical: Vec<(&str, Vec<&str>)> = Vec::new();
                            for skill in &unit.radical_actions {
                                let rad = skill.radical();
                                if let Some(entry) = by_radical.iter_mut().find(|(r, _)| *r == rad)
                                {
                                    entry.1.push(skill.name());
                                } else {
                                    by_radical.push((rad, vec![skill.name()]));
                                }
                            }
                            let grouped: Vec<String> = by_radical
                                .iter()
                                .map(|(rad, names)| format!("{}: {}", rad, names.join(", ")))
                                .collect();
                            desc.push_str(&format!(" | Skills: {}", grouped.join(" | ")));
                            for skill in &unit.radical_actions {
                                log_msgs.push(format!(
                                    "[{}] {}: {}",
                                    skill.radical(),
                                    skill.name(),
                                    skill.description()
                                ));
                            }
                        }

                        if let Some(intent) = &unit.intent {
                            desc.push_str(&format!(" | Intent: {}", intent.label()));
                        }
                    }
                }
            }
            battle.log_message(&desc);
            for msg in log_msgs {
                battle.log_message(&msg);
            }
        }
    }
    BattleEvent::None
}

fn handle_deployment(battle: &mut TacticalBattle, key: &str) -> BattleEvent {
    let (cx, cy, valid_tiles) = match &battle.phase {
        TacticalPhase::Deployment {
            cursor_x,
            cursor_y,
            valid_tiles,
        } => (*cursor_x, *cursor_y, valid_tiles.clone()),
        _ => return BattleEvent::None,
    };

    match key {
        "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" => {
            let (dx, dy) = match key {
                "ArrowUp" => (0, -1),
                "ArrowDown" => (0, 1),
                "ArrowLeft" => (-1, 0),
                "ArrowRight" => (1, 0),
                _ => (0, 0),
            };
            let nx = cx + dx;
            let ny = cy + dy;
            if valid_tiles.contains(&(nx, ny)) {
                if let TacticalPhase::Deployment {
                    cursor_x, cursor_y, ..
                } = &mut battle.phase
                {
                    *cursor_x = nx;
                    *cursor_y = ny;
                }
            } else {
                // Find closest valid tile in that direction
                let mut best = None;
                let mut best_dist = i32::MAX;
                for &(vx, vy) in &valid_tiles {
                    let along = (vx - cx) * dx + (vy - cy) * dy;
                    if along <= 0 {
                        continue;
                    }
                    let dist = (vx - cx).abs() + (vy - cy).abs();
                    if dist < best_dist {
                        best_dist = dist;
                        best = Some((vx, vy));
                    }
                }
                if let Some((bx, by)) = best {
                    if let TacticalPhase::Deployment {
                        cursor_x, cursor_y, ..
                    } = &mut battle.phase
                    {
                        *cursor_x = bx;
                        *cursor_y = by;
                    }
                }
            }
            BattleEvent::None
        }
        "Enter" | " " => {
            battle.units[0].x = cx;
            battle.units[0].y = cy;
            battle.phase = TacticalPhase::Command;
            battle.log_message(format!("Deployed at ({}, {}).", cx, cy));
            BattleEvent::None
        }
        _ => BattleEvent::None,
    }
}

fn handle_skill_menu(battle: &mut TacticalBattle, key: &str) -> BattleEvent {
    let total = battle.player_radical_abilities.len();
    match key {
        "Escape" => {
            battle.skill_menu_open = false;
            BattleEvent::None
        }
        "ArrowUp" => {
            battle.skill_menu_cursor = battle.skill_menu_cursor.saturating_sub(1);
            BattleEvent::None
        }
        "ArrowDown" => {
            battle.skill_menu_cursor = (battle.skill_menu_cursor + 1).min(total.saturating_sub(1));
            BattleEvent::None
        }
        "Enter" => {
            battle.skill_menu_open = false;
            let idx = battle.skill_menu_cursor;
            if idx >= total {
                return BattleEvent::None;
            }
            let (_radical_str, ability) = battle.player_radical_abilities[idx];
            let skill_type = ability.skill_type();

            match skill_type {
                crate::enemy::SkillType::SelfBuff => {
                    let (radical_str, ability) = battle.player_radical_abilities[idx];
                    let msg = crate::combat::radical::apply_player_radical_ability(
                        battle, 0, 0, ability,
                    );
                    battle.log_message(&msg);
                    battle.consumed_radicals.push(radical_str);
                    battle.player_radical_abilities.remove(idx);
                    battle.player_acted = true;
                    try_end_player_turn(battle)
                }
                crate::enemy::SkillType::MeleeTarget => {
                    battle.selected_radical_ability = Some(idx);
                    let px = battle.units[0].x;
                    let py = battle.units[0].y;
                    let adjacent: Vec<(i32, i32)> = battle
                        .adjacent_enemies(px, py)
                        .iter()
                        .map(|&i| (battle.units[i].x, battle.units[i].y))
                        .collect();
                    if adjacent.is_empty() {
                        battle.log_message("No adjacent enemies for this melee skill.");
                        battle.selected_radical_ability = None;
                        return BattleEvent::None;
                    }
                    let (cx, cy) = adjacent[0];
                    battle.phase = TacticalPhase::Targeting {
                        mode: TargetMode::Skill,
                        cursor_x: cx,
                        cursor_y: cy,
                        valid_targets: adjacent,
                        aoe_preview: vec![],
                    };
                    BattleEvent::None
                }
                crate::enemy::SkillType::RangedTarget(range) => {
                    battle.selected_radical_ability = Some(idx);
                    let px = battle.units[0].x;
                    let py = battle.units[0].y;
                    let los = tiles_in_range_with_los(&battle.arena, px, py, range);
                    let valid: Vec<(i32, i32)> = los
                        .into_iter()
                        .filter(|&(tx, ty)| {
                            battle
                                .unit_at(tx, ty)
                                .map(|i| battle.units[i].is_enemy())
                                .unwrap_or(false)
                        })
                        .collect();
                    if valid.is_empty() {
                        battle.log_message("No enemies in range for this skill.");
                        battle.selected_radical_ability = None;
                        return BattleEvent::None;
                    }
                    let (cx, cy) = valid[0];
                    battle.phase = TacticalPhase::Targeting {
                        mode: TargetMode::Skill,
                        cursor_x: cx,
                        cursor_y: cy,
                        valid_targets: valid,
                        aoe_preview: vec![],
                    };
                    BattleEvent::None
                }
                crate::enemy::SkillType::GroundTarget(range) => {
                    battle.selected_radical_ability = Some(idx);
                    let px = battle.units[0].x;
                    let py = battle.units[0].y;
                    let los = tiles_in_range_with_los(&battle.arena, px, py, range);
                    let (cx, cy) = if los.is_empty() { (px, py) } else { los[0] };
                    battle.phase = TacticalPhase::Targeting {
                        mode: TargetMode::Skill,
                        cursor_x: cx,
                        cursor_y: cy,
                        valid_targets: los,
                        aoe_preview: vec![],
                    };
                    BattleEvent::None
                }
            }
        }
        _ => BattleEvent::None,
    }
}

fn handle_spell_menu(battle: &mut TacticalBattle, key: &str) -> BattleEvent {
    match key {
        "Escape" => {
            battle.spell_menu_open = false;
            BattleEvent::None
        }
        "ArrowUp" => {
            battle.spell_cursor = battle.spell_cursor.saturating_sub(1);
            BattleEvent::None
        }
        "ArrowDown" => {
            battle.spell_cursor =
                (battle.spell_cursor + 1).min(battle.available_spells.len().saturating_sub(1));
            BattleEvent::None
        }
        "Enter" => {
            if battle.spell_cursor >= battle.available_spells.len() {
                battle.spell_menu_open = false;
                return BattleEvent::None;
            }
            let (hanzi, _pinyin, effect) = battle.available_spells[battle.spell_cursor];
            let spell_idx = battle.spell_cursor;
            battle.spell_menu_open = false;

            match effect {
                SpellEffect::Heal(_)
                | SpellEffect::Shield
                | SpellEffect::Reveal
                | SpellEffect::FocusRestore(_)
                | SpellEffect::Thorns(_) => {
                    let px = battle.units[0].x;
                    let py = battle.units[0].y;
                    battle.typing_action = Some(TypingAction::SpellCast {
                        spell_idx,
                        target_x: px,
                        target_y: py,
                        effect,
                    });
                    battle.typing_buffer.clear();
                    battle.log_message(format!("Type pinyin for {} to cast!", hanzi));
                }
                _ => {
                    let base_range = targeting::spell_range(&effect) + battle.player_stance.spell_range_mod();
                    let range = weather_adjusted_range(base_range, battle.weather);
                    let px = battle.units[0].x;
                    let py = battle.units[0].y;
                    let los_tiles = tiles_in_range_with_los(&battle.arena, px, py, range);

                    let valid: Vec<(i32, i32)> = if matches!(
                        effect,
                        SpellEffect::FireAoe(_)
                            | SpellEffect::Poison(_, _)
                            | SpellEffect::Cone(_)
                            | SpellEffect::Wall(_)
                            | SpellEffect::Pierce(_)
                    ) {
                        los_tiles
                    } else if matches!(effect, SpellEffect::Dash(_)) {
                        targeting::dash_target_tiles(battle, px, py, range)
                    } else {
                        los_tiles
                            .into_iter()
                            .filter(|&(tx, ty)| {
                                battle
                                    .unit_at(tx, ty)
                                    .map(|idx| battle.units[idx].is_enemy())
                                    .unwrap_or(false)
                            })
                            .collect()
                    };

                    if valid.is_empty() {
                        battle.log_message("No valid targets in range.");
                        return BattleEvent::None;
                    }

                    let (cx, cy) = valid[0];
                    let preview = targeting::compute_aoe_preview(&effect, cx, cy, px, py);
                    battle.phase = TacticalPhase::Targeting {
                        mode: TargetMode::Spell { spell_idx },
                        cursor_x: cx,
                        cursor_y: cy,
                        valid_targets: valid,
                        aoe_preview: preview,
                    };
                }
            }
            BattleEvent::None
        }
        _ => BattleEvent::None,
    }
}

fn handle_item_menu(battle: &mut TacticalBattle, key: &str) -> BattleEvent {
    match key {
        "Escape" => {
            battle.item_menu_open = false;
            BattleEvent::None
        }
        "ArrowUp" => {
            battle.item_cursor = battle.item_cursor.saturating_sub(1);
            BattleEvent::None
        }
        "ArrowDown" => {
            battle.item_cursor =
                (battle.item_cursor + 1).min(battle.available_items.len().saturating_sub(1));
            BattleEvent::None
        }
        "Enter" => {
            if battle.item_cursor >= battle.available_items.len() {
                battle.item_menu_open = false;
                return BattleEvent::None;
            }
            let (orig_idx, item) = battle.available_items[battle.item_cursor].clone();
            let menu_idx = battle.item_cursor;
            battle.item_menu_open = false;
            use_item_in_combat(battle, menu_idx, orig_idx, &item)
        }
        _ => BattleEvent::None,
    }
}

fn handle_radical_picker(battle: &mut TacticalBattle, key: &str) -> BattleEvent {
    let total_options = 1 + battle.player_radical_abilities.len(); // 0=normal, 1+=abilities
    match key {
        "Escape" => {
            battle.radical_picker_open = false;
            BattleEvent::None
        }
        "ArrowUp" => {
            battle.radical_picker_cursor = battle.radical_picker_cursor.saturating_sub(1);
            BattleEvent::None
        }
        "ArrowDown" => {
            battle.radical_picker_cursor =
                (battle.radical_picker_cursor + 1).min(total_options.saturating_sub(1));
            BattleEvent::None
        }
        "Enter" => {
            battle.radical_picker_open = false;
            if battle.radical_picker_cursor == 0 {
                battle.selected_radical_ability = None;
            } else {
                battle.selected_radical_ability = Some(battle.radical_picker_cursor - 1);
            }
            targeting::enter_attack_targeting(battle);
            BattleEvent::None
        }
        _ => BattleEvent::None,
    }
}

fn use_item_in_combat(
    battle: &mut TacticalBattle,
    menu_idx: usize,
    orig_idx: usize,
    item: &crate::player::Item,
) -> BattleEvent {
    use crate::player::Item;
    let msg = match item {
        Item::MedHypo(amount) => {
            let heal = *amount;
            let unit = &mut battle.units[0];
            unit.hp = (unit.hp + heal).min(unit.max_hp);
            battle.audio_events.push(AudioEvent::Heal);
            format!("Healed {} HP!", heal)
        }
        Item::ToxinGrenade(dmg, turns) => {
            let dmg = *dmg;
            let turns = *turns;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let adj = battle.adjacent_enemies(px, py);
            if adj.is_empty() {
                battle.log_message("No adjacent enemies to poison!");
                return BattleEvent::None;
            }
            for &eidx in &adj {
                battle.units[eidx].statuses.push(StatusInstance::new(
                    StatusKind::Poison { damage: dmg },
                    turns,
                ));
            }
            battle.audio_events.push(AudioEvent::StatusPoison);
            format!("Poisoned {} enemies!", adj.len())
        }
        Item::StimPack(turns) => {
            let turns = *turns;
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Haste, turns));
            format!("Haste for {} turns!", turns)
        }
        Item::EMPGrenade => {
            let mut stunned_count = 0;
            for i in 1..battle.units.len() {
                if battle.units[i].alive {
                    battle.units[i].stunned = true;
                    stunned_count += 1;
                }
            }
            format!("Stunned {} enemies!", stunned_count)
        }
        Item::PersonalTeleporter => {
            let w = battle.arena.width;
            let h = battle.arena.height;
            let seed = battle.turn_number as u64;
            let mut best = None;
            for attempt in 0..50u64 {
                let hash = seed
                    .wrapping_mul(2654435761)
                    .wrapping_add(attempt)
                    .wrapping_mul(2246822519);
                let tx = (hash % w as u64) as i32;
                let ty = ((hash >> 16) % h as u64) as i32;
                if battle
                    .arena
                    .tile(tx, ty)
                    .map(|t| t.is_walkable())
                    .unwrap_or(false)
                    && battle.unit_at(tx, ty).is_none()
                {
                    best = Some((tx, ty));
                    break;
                }
            }
            if let Some((tx, ty)) = best {
                battle.units[0].x = tx;
                battle.units[0].y = ty;
                format!("Teleported to ({},{})!", tx, ty)
            } else {
                battle.log_message("No valid teleport destination!");
                return BattleEvent::None;
            }
        }
        Item::ScannerPulse
        | Item::RationPack(_)
        | Item::FocusStim(_)
        | Item::SynthAle(_)
        | Item::CreditChip(_)
        | Item::Revitalizer(_)
        | Item::NavComputer
        | Item::DataCore(_) => {
            battle.log_message("This item has no effect in combat.");
            return BattleEvent::None;
        }
        Item::HoloDecoy(turns) => {
            let turns = *turns;
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Haste, turns));
            format!("Smoke screen! Haste for {} turns!", turns)
        }
        Item::PlasmaBurst(damage) => {
            let damage = *damage;
            let mut count = 0;
            for i in 1..battle.units.len() {
                if battle.units[i].alive {
                    battle.units[i].hp -= damage;
                    count += 1;
                }
            }
            format!("Cracker hit {} enemies for {} damage!", count, damage)
        }
        Item::PlasmaShield(turns) => {
            let turns = *turns;
            battle.units[0].defending = true;
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Regen { heal: 1 }, turns));
            battle.audio_events.push(AudioEvent::ShieldBlock);
            format!("Iron Skin! Shield + Regen for {} turns!", turns)
        }
        Item::NeuralBoost => {
            battle.units[0].statuses.retain(|s| !s.is_negative());
            "All negative effects purged!".to_string()
        }
        Item::ShockModule(damage) => {
            let damage = *damage;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let mut nearest: Option<(usize, i32)> = None;
            for i in 1..battle.units.len() {
                if battle.units[i].alive {
                    let dist = (battle.units[i].x - px).abs() + (battle.units[i].y - py).abs();
                    if nearest.is_none() || dist < nearest.unwrap().1 {
                        nearest = Some((i, dist));
                    }
                }
            }
            if let Some((idx, _)) = nearest {
                battle.units[idx].hp -= damage;
                format!("Thunder strikes for {} damage!", damage)
            } else {
                battle.log_message("No target for thunder!");
                return BattleEvent::None;
            }
        }
        Item::BiogelPatch(regen) => {
            let regen = *regen;
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Regen { heal: regen }, 5));
            format!("Jade Salve! Regen {} per turn for 5 turns!", regen)
        }
        Item::VenomDart => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Envenomed, 5));
            "Weapon envenomed for 5 turns!".to_string()
        }
        Item::DeflectorDrone(turns) => {
            let turns = *turns;
            battle.units[0].defending = true;
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::SpiritShield, turns));
            battle.audio_events.push(AudioEvent::ShieldBlock);
            format!("Ward active! Shield + Spirit Shield for {} turns!", turns)
        }
        Item::NaniteSwarm => {
            let mut count = 0;
            for i in 1..battle.units.len() {
                if battle.units[i].alive {
                    battle.units[i].stunned = true;
                    count += 1;
                }
            }
            format!("Ink splatters {} enemies!", count)
        }
        Item::ReflectorPlate => {
            battle.units[0].statuses.push(StatusInstance::new(
                StatusKind::Thorns,
                1,
            ));
            battle.units[0].defending = true;
            "Mirror Shard! Next attack will be reflected!".to_string()
        }
        Item::CryoGrenade(turns) => {
            let turns = *turns;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let adj = battle.adjacent_enemies(px, py);
            if adj.is_empty() {
                battle.log_message("No adjacent enemies to freeze!");
                return BattleEvent::None;
            }
            for &eidx in &adj {
                battle.units[eidx].statuses.push(StatusInstance::new(
                    StatusKind::Freeze,
                    turns,
                ));
            }
            battle.audio_events.push(AudioEvent::StatusSlow);
            format!("Frost Vial freezes {} adjacent enemies!", adj.len())
        }
        Item::CloakingDevice(turns) => {
            let turns = *turns;
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Invisible, turns));
            format!("Shadow Cloak! Invisible for {} turns!", turns)
        }
        Item::NanoShield(armor) => {
            let armor = *armor;
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Fortify { stacks: armor }, 99));
            battle.units[0].defending = true;
            format!("Dragon Scale! +{} armor and shield!", armor)
        }
        // SignalJammer handled at end of match
        Item::GrappleLine => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let mut nearest: Option<(usize, i32)> = None;
            for i in 1..battle.units.len() {
                if battle.units[i].alive {
                    let dist = (battle.units[i].x - px).abs() + (battle.units[i].y - py).abs();
                    if dist > 1 && (nearest.is_none() || dist < nearest.unwrap().1) {
                        nearest = Some((i, dist));
                    }
                }
            }
            if let Some((eidx, _)) = nearest {
                let dirs: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];
                let mut placed = false;
                for &(dx, dy) in &dirs {
                    let tx = px + dx;
                    let ty = py + dy;
                    if battle.arena.tile(tx, ty).map(|t| t.is_walkable()).unwrap_or(false)
                        && battle.unit_at(tx, ty).is_none()
                    {
                        battle.units[eidx].x = tx;
                        battle.units[eidx].y = ty;
                        placed = true;
                        break;
                    }
                }
                if placed {
                    "Silk Rope pulls an enemy close!".to_string()
                } else {
                    battle.log_message("No space to pull enemy to!");
                    return BattleEvent::None;
                }
            } else {
                battle.log_message("No distant enemies to pull!");
                return BattleEvent::None;
            }
        }
        Item::OmniGel => {
            battle.units[0].statuses.retain(|s| !s.is_negative());
            "Lotus Elixir purges all negative effects!".to_string()
        }
        Item::SonicEmitter(damage) => {
            let damage = *damage;
            let mut count = 0;
            for i in 1..battle.units.len() {
                if battle.units[i].alive {
                    battle.units[i].hp -= damage;
                    battle.units[i].statuses.push(StatusInstance::new(
                        StatusKind::Slow,
                        1,
                    ));
                    count += 1;
                }
            }
            format!("Thunder Drum hits {} enemies for {} damage + Slow!", count, damage)
        }
        Item::CircuitInk => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Empowered { amount: 2 }, 5));
            "Cinnabar Ink! +2 spell damage for 5 turns!".to_string()
        }
        Item::ThrusterPack => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let adj = battle.adjacent_enemies(px, py);
            let mut count = 0;
            for &eidx in &adj {
                let dx = (battle.units[eidx].x - px).signum();
                let dy = (battle.units[eidx].y - py).signum();
                for _ in 0..2 {
                    let nx = battle.units[eidx].x + dx;
                    let ny = battle.units[eidx].y + dy;
                    if battle.arena.tile(nx, ny).map(|t| t.is_walkable()).unwrap_or(false)
                        && battle.unit_at(nx, ny).is_none()
                    {
                        battle.units[eidx].x = nx;
                        battle.units[eidx].y = ny;
                    } else {
                        break;
                    }
                }
                count += 1;
            }
            format!("Thruster Pack pushes {} enemies away!", count)
        }
        Item::SignalJammer(turns) => {
            let turns = *turns;
            let mut count = 0;
            for i in 1..battle.units.len() {
                if battle.units[i].alive {
                    battle.units[i].statuses.push(StatusInstance::new(
                        StatusKind::Confused,
                        turns,
                    ));
                    count += 1;
                }
            }
            format!("Signal Jammer confuses {} enemies for {} turns!", count, turns)
        }
    };
    battle.log_message(&msg);
    battle.available_items.remove(menu_idx);
    battle.used_item_indices.push(orig_idx);
    battle.player_acted = true;
    battle.phase = TacticalPhase::Resolve {
        message: msg,
        timer: 20,
        end_turn: true,
    };
    BattleEvent::None
}

// ── Turn advancement ─────────────────────────────────────────────────────────

/// Try to end the player's turn. If both moved and acted (or acted with defend/wait),
/// advance to the next unit.
fn try_end_player_turn(battle: &mut TacticalBattle) -> BattleEvent {
    // Player turn ends when they've acted. Movement is optional.
    if !battle.player_acted {
        battle.phase = TacticalPhase::Command;
        return BattleEvent::None;
    }

    // Tick player statuses at end of player turn.
    let (status_dmg, status_heal) = tick_statuses(&mut battle.units[0].statuses);
    if status_dmg > 0 {
        battle.units[0].hp -= status_dmg;
        battle.log_message(format!("Status damage: -{} HP", status_dmg));
    }
    if status_heal > 0 {
        let unit = &mut battle.units[0];
        unit.hp = (unit.hp + status_heal).min(unit.max_hp);
        battle.log_message(format!("Status heal: +{} HP", status_heal));
    }
    if battle.units[0].hp <= 0 {
        battle.units[0].alive = false;
        battle.phase = TacticalPhase::End {
            victory: false,
            timer: 60,
        };
        return BattleEvent::None;
    }

    // Advance to next unit in turn queue.
    advance_to_next(battle)
}

/// Advance the turn queue and run enemy turns until it's the player's turn again
/// (or the battle ends).
fn advance_to_next(battle: &mut TacticalBattle) -> BattleEvent {
    advance_turn(battle);

    loop {
        let current = battle.current_unit_idx();
        let unit = &battle.units[current];

        if !unit.alive {
            advance_turn(battle);
            continue;
        }

        if unit.is_player() {
            battle.player_moved = false;
            battle.player_acted = false;
            battle.phase = TacticalPhase::Command;
            return BattleEvent::None;
        }

        battle.phase = TacticalPhase::EnemyTurn {
            unit_idx: current,
            timer: 24,
            acted: false,
        };
        return BattleEvent::None;
    }
}

// ── Enemy turn execution ─────────────────────────────────────────────────────

pub fn execute_enemy_turn_action(battle: &mut TacticalBattle, unit_idx: usize) -> BattleEvent {
    // Tick enemy statuses.
    let (status_dmg, status_heal) = tick_statuses(&mut battle.units[unit_idx].statuses);
    if status_dmg > 0 {
        battle.units[unit_idx].hp -= status_dmg;
        if battle.units[unit_idx].hp <= 0 {
            battle.units[unit_idx].alive = false;
            battle.log_message("An enemy succumbs to status effects!");
            return BattleEvent::None;
        }
    }
    if status_heal > 0 {
        let unit = &mut battle.units[unit_idx];
        unit.hp = (unit.hp + status_heal).min(unit.max_hp);
    }

    if battle.units[unit_idx].thorn_armor_turns > 0 {
        battle.units[unit_idx].thorn_armor_turns -= 1;
    }

    if battle.units[unit_idx]
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Freeze))
    {
        battle.log_message(format!(
            "{} is frozen and can't act!",
            enemy_display_name(battle, unit_idx)
        ));
        return BattleEvent::None;
    }

    // Check stunned.
    if battle.units[unit_idx].stunned {
        battle.units[unit_idx].stunned = false;
        battle.log_message(format!(
            "{} is stunned and can't act!",
            enemy_display_name(battle, unit_idx)
        ));
        return BattleEvent::None;
    }

    // Clear defending at start of this unit's turn.
    battle.units[unit_idx].defending = false;

    if let Some(remaining) = battle.units[unit_idx].charge_remaining {
        if remaining > 0 {
            battle.units[unit_idx].charge_remaining = Some(remaining - 1);
            let name = enemy_display_name(battle, unit_idx);
            battle.log_message(format!(
                "{} is charging... ({} turns left)",
                name,
                remaining - 1
            ));
            return BattleEvent::None;
        } else {
            battle.units[unit_idx].charge_remaining = None;
            let name = enemy_display_name(battle, unit_idx);
            let dmg = (((battle.units[unit_idx].damage + battle.units[unit_idx].fortify_stacks)
                as f64)
                * 1.75)
                .ceil() as i32;
            let (actual, wuxing_label) = deal_damage_from(battle, unit_idx, 0, dmg);
            battle.log_message(format!(
                "{} unleashes charged attack for {} damage!",
                name, actual
            ));
            if let Some(wl) = wuxing_label {
                battle.log_message(wl);
            }
            return BattleEvent::None;
        }
    }

    if let Some(msg) = boss::boss_action(battle, unit_idx) {
        battle.log_message(msg);
        return BattleEvent::None;
    }

    // ── Sacrifice check: low-HP enemies may sacrifice for allies ─────
    if battle.units[unit_idx].is_enemy() {
        if crate::combat::synergy::try_sacrifice(battle, unit_idx) {
            return BattleEvent::None;
        }
    }

    let action = if battle.units[unit_idx].is_companion() {
        choose_companion_action(battle, unit_idx)
    } else {
        choose_action(battle, unit_idx)
    };
    let name = enemy_display_name(battle, unit_idx);

    match action {
        AiAction::MeleeAttack { target_unit } => {
            let (synergy_bonus, synergy_msgs) = crate::combat::synergy::total_attack_synergy_bonus(battle, unit_idx);
            let dmg = battle.units[unit_idx].damage
                + battle.units[unit_idx].fortify_stacks
                + boss::ink_sage_bonus(battle, unit_idx)
                + synergy_bonus;
            for sm in &synergy_msgs {
                battle.log_message(sm);
            }
            if target_unit == 0 {
                battle.attacks_on_player_this_round += 1;
            }
            let multiply = battle.units[unit_idx].radical_multiply;
            let hits = if multiply { 2 } else { 1 };
            battle.units[unit_idx].radical_multiply = false;
            for _ in 0..hits {
                let (actual, wuxing_label) = deal_damage_from(battle, unit_idx, target_unit, dmg);
                battle.log_message(format!("{} attacks for {} damage!", name, actual));
                if let Some(wl) = wuxing_label {
                    battle.log_message(wl);
                }
            }
            if dmg >= 3 && battle.units[target_unit].alive {
                let ex = battle.units[unit_idx].x;
                let ey = battle.units[unit_idx].y;
                let kb_msgs = apply_knockback(battle, target_unit, ex, ey);
                for m in &kb_msgs {
                    battle.log_message(m);
                }
            }
        }
        AiAction::UseRadicalAction { action_idx } => {
            if action_idx < battle.units[unit_idx].radical_actions.len() {
                let radical = battle.units[unit_idx].radical_actions[action_idx];
                let msg = apply_radical_action(battle, unit_idx, radical);
                battle.log_message(msg);
            }
        }
        AiAction::Wait => {
            // Enemy waits — do nothing.
        }
        AiAction::MoveToTile { path } => {
            for &(nx, ny) in &path {
                move_unit(battle, unit_idx, nx, ny);
            }
        }
        AiAction::MoveAndAttack { path, target_unit } => {
            for &(nx, ny) in &path {
                move_unit(battle, unit_idx, nx, ny);
            }
            let (synergy_bonus, synergy_msgs) = crate::combat::synergy::total_attack_synergy_bonus(battle, unit_idx);
            let dmg = battle.units[unit_idx].damage
                + battle.units[unit_idx].fortify_stacks
                + boss::ink_sage_bonus(battle, unit_idx)
                + synergy_bonus;
            for sm in &synergy_msgs {
                battle.log_message(sm);
            }
            if target_unit == 0 {
                battle.attacks_on_player_this_round += 1;
            }
            let multiply = battle.units[unit_idx].radical_multiply;
            let hits = if multiply { 2 } else { 1 };
            battle.units[unit_idx].radical_multiply = false;
            for _ in 0..hits {
                let (actual, wuxing_label) = deal_damage_from(battle, unit_idx, target_unit, dmg);
                battle.log_message(format!("{} attacks for {} damage!", name, actual));
                if let Some(wl) = wuxing_label {
                    battle.log_message(wl);
                }
            }
            if dmg >= 3 && battle.units[target_unit].alive {
                let ex = battle.units[unit_idx].x;
                let ey = battle.units[unit_idx].y;
                let kb_msgs = apply_knockback(battle, target_unit, ex, ey);
                for m in &kb_msgs {
                    battle.log_message(m);
                }
            }
        }
        AiAction::MoveAndRadical { path, action_idx } => {
            for &(nx, ny) in &path {
                move_unit(battle, unit_idx, nx, ny);
            }
            if action_idx < battle.units[unit_idx].radical_actions.len() {
                let radical = battle.units[unit_idx].radical_actions[action_idx];
                let msg = apply_radical_action(battle, unit_idx, radical);
                battle.log_message(msg);
            }
        }
        AiAction::PushCrate {
            crate_x,
            crate_y,
            dx,
            dy,
        } => {
            battle.log_message(format!("{} shoves a cargo crate!", name));
            let msgs = crate::combat::tick::push_boulder(battle, crate_x, crate_y, dx, dy);
            for m in &msgs {
                battle.log_message(m);
            }
        }
    }

    BattleEvent::None
}

fn enemy_display_name(battle: &TacticalBattle, unit_idx: usize) -> String {
    let unit = &battle.units[unit_idx];
    if !unit.hanzi.is_empty() {
        unit.hanzi.to_string()
    } else {
        format!("Enemy {}", unit_idx)
    }
}

