//! Input handling for tactical combat phases.
//!
//! All keyboard input during a `TacticalBattle` flows through
//! `handle_input()` which dispatches based on the current `TacticalPhase`.

use crate::combat::action::{deal_damage, deal_damage_from, defend, flank_bonus, move_unit, wait};
use crate::combat::ai::{choose_action, choose_companion_action, AiAction};
use crate::combat::boss;
use crate::combat::grid::{
    manhattan, reachable_tiles, tiles_in_range_with_los, weather_adjusted_range,
};
use crate::combat::radical::apply_radical_action;
use crate::combat::terrain::{apply_knockback, apply_terrain_interactions, TerrainSource};
use crate::combat::turn::advance_turn;
use crate::combat::{
    AudioEvent, BattleTile, Projectile, ProjectileEffect, TacticalBattle, TacticalPhase,
    TargetMode, TypingAction, Weather, WuxingElement,
};
use crate::enemy::BossKind;
use crate::radical::SpellEffect;
use crate::status::{tick_statuses, StatusInstance, StatusKind};
use crate::vocab;

fn tile_spell_bonus(battle: &TacticalBattle, unit_idx: usize) -> i32 {
    let tile_bonus = match battle
        .arena
        .tile(battle.units[unit_idx].x, battle.units[unit_idx].y)
    {
        // SpiritualInk + InkPool → +2 spell power instead of +1
        Some(BattleTile::OilSlick) => {
            if battle.weather == Weather::EnergyFlux { 2 } else { 1 }
        }
        Some(BattleTile::HoloTrap) => 2,
        _ => 0,
    };
    let weather_bonus = match battle.weather {
        Weather::EnergyFlux => 1,
        _ => 0,
    };
    tile_bonus + weather_bonus
}

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
        return handle_typing(battle, key);
    }

    match &battle.phase {
        TacticalPhase::Command => handle_command(battle, key),
        TacticalPhase::Targeting { .. } => handle_targeting(battle, key),
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
            enter_move_targeting(battle);
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
                enter_attack_targeting(battle);
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
                    crate::player::PlayerClass::Operative | crate::player::PlayerClass::Operative
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
                    let base_range = spell_range(&effect) + battle.player_stance.spell_range_mod();
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
                        dash_target_tiles(battle, px, py, range)
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
                    let preview = compute_aoe_preview(&effect, cx, cy, px, py);
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
            enter_attack_targeting(battle);
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

fn spell_range(effect: &SpellEffect) -> i32 {
    match effect {
        SpellEffect::FireAoe(_) => 4,
        SpellEffect::StrongHit(_) => 2,
        SpellEffect::Drain(_) => 1,
        SpellEffect::Stun => 3,
        SpellEffect::Pacify => 3,
        SpellEffect::Slow(_) => 3,
        SpellEffect::Teleport => 4,
        SpellEffect::Poison(_, _) => 2,
        SpellEffect::ArmorBreak => 2,
        SpellEffect::Dash(_) => 5,
        SpellEffect::Pierce(_) => 6,
        SpellEffect::PullToward => 4,
        SpellEffect::KnockBack(_) => 2,
        SpellEffect::Thorns(_) => 0,
        SpellEffect::Cone(_) => 3,
        SpellEffect::Wall(_) => 4,
        SpellEffect::OilSlick => 4,
        SpellEffect::FreezeGround(_) => 5,
        SpellEffect::Ignite => 4,
        SpellEffect::PlantGrowth => 3,
        SpellEffect::Earthquake(_) => 3,
        SpellEffect::Sanctify(_) => 3,
        SpellEffect::FloodWave(_) => 5,
        SpellEffect::SummonBoulder => 4,
        _ => 1,
    }
}

fn compute_aoe_preview(
    effect: &SpellEffect,
    cx: i32,
    cy: i32,
    px: i32,
    py: i32,
) -> Vec<(i32, i32)> {
    match effect {
        SpellEffect::FireAoe(_) => {
            // Cross pattern: center + 4 cardinal neighbors
            vec![
                (cx, cy),
                (cx - 1, cy),
                (cx + 1, cy),
                (cx, cy - 1),
                (cx, cy + 1),
            ]
        }
        SpellEffect::Poison(_, _) => {
            // Small cloud: center + 2 adjacent
            vec![(cx, cy), (cx + 1, cy), (cx, cy + 1)]
        }
        SpellEffect::Dash(_) => line_between(px, py, cx, cy),
        SpellEffect::Pierce(_) => {
            let dx = (cx - px).signum();
            let dy = (cy - py).signum();
            if dx == 0 && dy == 0 {
                vec![(cx, cy)]
            } else {
                let mut tiles = Vec::new();
                let (mut x, mut y) = (px, py);
                for _ in 0..6 {
                    x += dx;
                    y += dy;
                    tiles.push((x, y));
                }
                tiles
            }
        }
        SpellEffect::Cone(_) => {
            let dx = (cx - px).signum();
            let dy = (cy - py).signum();
            let mut tiles = Vec::new();
            if dx != 0 && dy == 0 {
                // Horizontal cone
                tiles.push((px + dx, py));
                tiles.push((px + dx * 2, py));
                tiles.push((px + dx * 2, py - 1));
                tiles.push((px + dx * 2, py + 1));
                tiles.push((px + dx * 3, py));
                tiles.push((px + dx * 3, py - 1));
                tiles.push((px + dx * 3, py + 1));
            } else if dy != 0 && dx == 0 {
                // Vertical cone
                tiles.push((px, py + dy));
                tiles.push((px, py + dy * 2));
                tiles.push((px - 1, py + dy * 2));
                tiles.push((px + 1, py + dy * 2));
                tiles.push((px, py + dy * 3));
                tiles.push((px - 1, py + dy * 3));
                tiles.push((px + 1, py + dy * 3));
            } else {
                tiles.push((cx, cy));
            }
            tiles
        }
        SpellEffect::Wall(_) => {
            let dx = (cx - px).signum();
            let dy = (cy - py).signum();
            let mut tiles = Vec::new();
            if dx != 0 && dy == 0 {
                // Horizontal aim → vertical wall
                tiles.push((cx, cy - 1));
                tiles.push((cx, cy));
                tiles.push((cx, cy + 1));
            } else if dy != 0 && dx == 0 {
                // Vertical aim → horizontal wall
                tiles.push((cx - 1, cy));
                tiles.push((cx, cy));
                tiles.push((cx + 1, cy));
            } else {
                tiles.push((cx, cy));
            }
            tiles
        }
        SpellEffect::OilSlick | SpellEffect::PlantGrowth => {
            // 3×3 square
            let mut tiles = Vec::new();
            for dy in -1..=1 {
                for dx in -1..=1 {
                    tiles.push((cx + dx, cy + dy));
                }
            }
            tiles
        }
        SpellEffect::FreezeGround(_) | SpellEffect::Ignite | SpellEffect::Sanctify(_) => {
            // Cross pattern: center + 4 cardinal
            vec![
                (cx, cy),
                (cx - 1, cy),
                (cx + 1, cy),
                (cx, cy - 1),
                (cx, cy + 1),
            ]
        }
        SpellEffect::Earthquake(_) => {
            // Large cross: range 2 from center (13 tiles)
            let mut tiles = vec![(cx, cy)];
            for d in 1..=2 {
                tiles.push((cx - d, cy));
                tiles.push((cx + d, cy));
                tiles.push((cx, cy - d));
                tiles.push((cx, cy + d));
            }
            // Diagonals at distance 1
            tiles.push((cx - 1, cy - 1));
            tiles.push((cx + 1, cy - 1));
            tiles.push((cx - 1, cy + 1));
            tiles.push((cx + 1, cy + 1));
            tiles
        }
        SpellEffect::FloodWave(_) => {
            // 5×3 rectangle in direction from player
            let dx = (cx - px).signum();
            let dy = (cy - py).signum();
            let mut tiles = Vec::new();
            if dx != 0 && dy == 0 {
                // Horizontal wave
                for i in 0..5 {
                    for j in -1..=1 {
                        tiles.push((cx + dx * i, cy + j));
                    }
                }
            } else if dy != 0 && dx == 0 {
                // Vertical wave
                for i in 0..5 {
                    for j in -1..=1 {
                        tiles.push((cx + j, cy + dy * i));
                    }
                }
            } else {
                // Diagonal fallback: line
                for i in 0..5 {
                    tiles.push((cx + dx * i, cy + dy * i));
                }
            }
            tiles
        }
        SpellEffect::SummonBoulder => vec![(cx, cy)],
        _ => vec![(cx, cy)],
    }
}

fn line_between(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    let mut tiles = Vec::new();
    let dx = (x1 - x0).signum();
    let dy = (y1 - y0).signum();
    if dx == 0 && dy == 0 {
        return vec![(x0, y0)];
    }
    let (mut x, mut y) = (x0, y0);
    loop {
        x += dx;
        y += dy;
        tiles.push((x, y));
        if x == x1 && y == y1 {
            break;
        }
        if tiles.len() > 20 {
            break;
        }
    }
    tiles
}

fn dash_target_tiles(battle: &TacticalBattle, px: i32, py: i32, range: i32) -> Vec<(i32, i32)> {
    let mut targets = Vec::new();
    let directions = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    for &(dx, dy) in &directions {
        let mut last_open = None;
        for dist in 1..=range {
            let tx = px + dx * dist;
            let ty = py + dy * dist;
            if tx < 0
                || ty < 0
                || tx >= battle.arena.width as i32
                || ty >= battle.arena.height as i32
            {
                break;
            }
            match battle.arena.tile(tx, ty) {
                Some(t) if !t.is_walkable() => break,
                None => break,
                _ => {
                    last_open = Some((tx, ty));
                }
            }
        }
        if let Some(tile) = last_open {
            if tile != (px, py) {
                targets.push(tile);
            }
        }
    }
    targets
}

fn spell_effect_school(effect: &SpellEffect) -> &'static str {
    match effect {
        SpellEffect::FireAoe(_) => "fire",
        SpellEffect::Stun => "lightning",
        SpellEffect::Drain(_) => "drain",
        SpellEffect::StrongHit(_) => "force",
        SpellEffect::Heal(_) => "heal",
        SpellEffect::Shield => "shield",
        SpellEffect::Reveal => "reveal",
        SpellEffect::Pacify => "pacify",
        SpellEffect::Slow(_) => "ice",
        SpellEffect::Teleport => "wind",
        SpellEffect::Poison(_, _) => "poison",
        SpellEffect::FocusRestore(_) => "focus",
        SpellEffect::ArmorBreak => "force",
        SpellEffect::Dash(_) => "wind",
        SpellEffect::Pierce(_) => "force",
        SpellEffect::PullToward => "wind",
        SpellEffect::KnockBack(_) => "force",
        SpellEffect::Thorns(_) => "poison",
        SpellEffect::Cone(_) => "fire",
        SpellEffect::Wall(_) => "shield",
        SpellEffect::OilSlick => "poison",
        SpellEffect::FreezeGround(_) => "ice",
        SpellEffect::Ignite => "fire",
        SpellEffect::PlantGrowth => "heal",
        SpellEffect::Earthquake(_) => "force",
        SpellEffect::Sanctify(_) => "heal",
        SpellEffect::FloodWave(_) => "ice",
        SpellEffect::SummonBoulder => "shield",
    }
}

/// Map a SpellEffect to a Wuxing element for the combo chain system.
pub fn spell_effect_element(effect: &SpellEffect) -> Option<WuxingElement> {
    match effect {
        // Fire
        SpellEffect::FireAoe(_) | SpellEffect::Cone(_) | SpellEffect::Ignite => {
            Some(WuxingElement::Fire)
        }
        // Water/Ice
        SpellEffect::Slow(_) | SpellEffect::FreezeGround(_) | SpellEffect::FloodWave(_) => {
            Some(WuxingElement::Water)
        }
        // Metal/Force
        SpellEffect::StrongHit(_)
        | SpellEffect::ArmorBreak
        | SpellEffect::Pierce(_)
        | SpellEffect::KnockBack(_)
        | SpellEffect::Stun => Some(WuxingElement::Metal),
        // Wood/Nature
        SpellEffect::Poison(_, _)
        | SpellEffect::Thorns(_)
        | SpellEffect::PlantGrowth
        | SpellEffect::OilSlick
        | SpellEffect::Heal(_)
        | SpellEffect::Drain(_) => Some(WuxingElement::Wood),
        // Earth
        SpellEffect::Earthquake(_)
        | SpellEffect::Wall(_)
        | SpellEffect::SummonBoulder
        | SpellEffect::Shield => Some(WuxingElement::Earth),
        // No element
        _ => None,
    }
}

/// Name of the combo triggered by casting `prev` then `current` elements in sequence.
pub fn spell_combo_name(
    prev: WuxingElement,
    current: WuxingElement,
) -> Option<&'static str> {
    match (prev, current) {
        (WuxingElement::Water, WuxingElement::Fire) => Some("Steam Burst"),
        (WuxingElement::Water, WuxingElement::Earth) => Some("Avalanche"),
        (WuxingElement::Fire, WuxingElement::Fire) => Some("Inferno"),
        (WuxingElement::Wood, WuxingElement::Fire) => Some("Toxic Cloud"),
        (WuxingElement::Fire, WuxingElement::Metal) => Some("Tempering"),
        (WuxingElement::Metal, WuxingElement::Water) => Some("Lightning Storm"),
        (WuxingElement::Earth, WuxingElement::Earth) => Some("Petrify"),
        (WuxingElement::Wood, WuxingElement::Water) => Some("Overgrowth"),
        (WuxingElement::Metal, WuxingElement::Earth) => Some("Shatter"),
        (WuxingElement::Wood, WuxingElement::Earth) => Some("Entangle"),
        (WuxingElement::Fire, WuxingElement::Wood) => Some("Purifying Flame"),
        (WuxingElement::Water, WuxingElement::Metal) => Some("Frozen Edge"),
        _ => None,
    }
}

/// Apply a spell combo effect. Called when the player casts two spells of the right
/// elements within 2 turns. Returns a log message describing the combo.
fn apply_spell_combo(
    battle: &mut TacticalBattle,
    combo_name: &str,
    target_x: i32,
    target_y: i32,
) -> String {
    let px = battle.units[0].x;
    let py = battle.units[0].y;

    match combo_name {
        "Steam Burst" => {
            // AoE Steam tiles in 2-radius + 2 dmg to all in area + Confused 1 turn
            let mut hits = 0;
            for dx in -2..=2_i32 {
                for dy in -2..=2_i32 {
                    if dx.abs() + dy.abs() > 2 {
                        continue;
                    }
                    let tx = target_x + dx;
                    let ty = target_y + dy;
                    if battle.arena.in_bounds(tx, ty) {
                        if let Some(tile) = battle.arena.tile(tx, ty) {
                            if tile.is_walkable() {
                                battle.arena.set_tile(tx, ty, BattleTile::VentSteam);
                                battle.arena.set_steam(tx, ty, 3);
                            }
                        }
                        if let Some(idx) = battle.unit_at(tx, ty) {
                            if battle.units[idx].is_enemy() {
                                deal_damage(battle, idx, 2);
                                battle.units[idx]
                                    .statuses
                                    .push(StatusInstance::new(StatusKind::Confused, 1));
                                hits += 1;
                            }
                        }
                    }
                }
            }
            format!(
                "Steam erupts! {} enemies take 2 dmg and are Confused!",
                hits
            )
        }
        "Avalanche" => {
            // 4 dmg in cone + Slow 3 turns + BrokenGround tiles
            let cone = compute_aoe_preview(
                &SpellEffect::Cone(4),
                target_x,
                target_y,
                px,
                py,
            );
            let mut hits = 0;
            for &(tx, ty) in &cone {
                if battle.arena.in_bounds(tx, ty) {
                    battle.arena.set_tile(tx, ty, BattleTile::DamagedPlating);
                    if let Some(idx) = battle.unit_at(tx, ty) {
                        if battle.units[idx].is_enemy() {
                            deal_damage(battle, idx, 4);
                            battle.units[idx]
                                .statuses
                                .push(StatusInstance::new(StatusKind::Slow, 3));
                            hits += 1;
                        }
                    }
                }
            }
            format!("Avalanche! {} enemies take 4 dmg and are Slowed!", hits)
        }
        "Inferno" => {
            // Double the second Fire spell's damage + Burn 2 for 3 turns
            // We apply 4 bonus fire damage to target + burn
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    deal_damage(battle, idx, 4);
                    battle.units[idx]
                        .statuses
                        .push(StatusInstance::new(StatusKind::Burn { damage: 2 }, 3));
                    return "Inferno! Double fire damage + Burn!".to_string();
                }
            }
            // AoE fallback — burn everything in cross
            let cross = [
                (target_x, target_y),
                (target_x - 1, target_y),
                (target_x + 1, target_y),
                (target_x, target_y - 1),
                (target_x, target_y + 1),
            ];
            let mut hits = 0;
            for &(tx, ty) in &cross {
                if let Some(idx) = battle.unit_at(tx, ty) {
                    if battle.units[idx].is_enemy() {
                        deal_damage(battle, idx, 4);
                        battle.units[idx]
                            .statuses
                            .push(StatusInstance::new(StatusKind::Burn { damage: 2 }, 3));
                        hits += 1;
                    }
                }
            }
            format!("Inferno! {} enemies scorched with Burn!", hits)
        }
        "Toxic Cloud" => {
            // Poison gas: 3×3, 2 dmg + Poison(1) 3 turns
            let mut hits = 0;
            for dx in -1..=1_i32 {
                for dy in -1..=1_i32 {
                    let tx = target_x + dx;
                    let ty = target_y + dy;
                    if battle.arena.in_bounds(tx, ty) {
                        if let Some(idx) = battle.unit_at(tx, ty) {
                            if battle.units[idx].is_enemy() {
                                deal_damage(battle, idx, 2);
                                battle.units[idx].statuses.push(StatusInstance::new(
                                    StatusKind::Poison { damage: 1 },
                                    3,
                                ));
                                hits += 1;
                            }
                        }
                    }
                }
            }
            format!("Toxic Cloud! {} enemies take 2 dmg + Poison!", hits)
        }
        "Tempering" => {
            // Player gains +2 armor for 3 turns + Fortify 2
            battle.combo_armor_bonus = 2;
            battle.combo_armor_turns = 3;
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Fortify { stacks: 2 }, 3));
            "Tempering! +2 armor for 3 turns + Fortify!".to_string()
        }
        "Lightning Storm" => {
            // Chain lightning: 3 dmg to target + 2 dmg to all Wet enemies
            let mut hits = 0;
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    deal_damage(battle, idx, 3);
                    hits += 1;
                }
            }
            let wet_targets: Vec<usize> = (1..battle.units.len())
                .filter(|&i| {
                    battle.units[i].alive
                        && battle.units[i].is_enemy()
                        && battle.units[i]
                            .statuses
                            .iter()
                            .any(|s| matches!(s.kind, StatusKind::Wet))
                })
                .collect();
            for idx in wet_targets {
                deal_damage(battle, idx, 2);
                hits += 1;
            }
            format!("Lightning Storm! {} enemies struck!", hits)
        }
        "Petrify" => {
            // Target turned to stone: skip 2 turns + 4 armor
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    battle.units[idx].stunned = true;
                    battle.units[idx]
                        .statuses
                        .push(StatusInstance::new(StatusKind::Freeze, 2));
                    battle.units[idx].radical_armor += 4;
                    return "Petrify! Enemy turned to stone!".to_string();
                }
            }
            "Petrify! The ground trembles...".to_string()
        }
        "Overgrowth" => {
            // Create Grass+BambooThicket in 3×3, heal player 3 HP
            for dx in -1..=1_i32 {
                for dy in -1..=1_i32 {
                    let tx = target_x + dx;
                    let ty = target_y + dy;
                    if battle.arena.in_bounds(tx, ty) {
                        if let Some(tile) = battle.arena.tile(tx, ty) {
                            if tile.is_walkable() && battle.unit_at(tx, ty).is_none() {
                                if dx == 0 && dy == 0 {
                                    battle
                                        .arena
                                        .set_tile(tx, ty, BattleTile::PipeTangle);
                                } else {
                                    battle.arena.set_tile(tx, ty, BattleTile::WiringPanel);
                                }
                            }
                        }
                    }
                }
            }
            let player = &mut battle.units[0];
            player.hp = (player.hp + 3).min(player.max_hp);
            "Overgrowth! Lush growth spreads, heal 3 HP!".to_string()
        }
        "Shatter" => {
            // ArmorBreak + 3 dmg + BrokenGround under target
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    battle.units[idx].radical_armor = 0;
                    deal_damage(battle, idx, 3);
                    battle.arena.set_tile(target_x, target_y, BattleTile::DamagedPlating);
                    return "Shatter! Armor broken + 3 dmg!".to_string();
                }
            }
            battle.arena.set_tile(target_x, target_y, BattleTile::DamagedPlating);
            "Shatter! The ground cracks!".to_string()
        }
        "Entangle" => {
            // Rooted 3 turns + Thorns tiles around target
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    battle.units[idx]
                        .statuses
                        .push(StatusInstance::new(StatusKind::Rooted, 3));
                }
            }
            let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];
            for (dx, dy) in &deltas {
                let tx = target_x + dx;
                let ty = target_y + dy;
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        if tile.is_walkable() {
                            battle.arena.set_tile(tx, ty, BattleTile::ElectrifiedWire);
                        }
                    }
                }
            }
            "Entangle! Enemy Rooted + Thorns spread!".to_string()
        }
        "Purifying Flame" => {
            // Remove all negative statuses from player + 2 dmg AoE
            battle.units[0]
                .statuses
                .retain(|s| !s.is_negative());
            let cross = [
                (target_x, target_y),
                (target_x - 1, target_y),
                (target_x + 1, target_y),
                (target_x, target_y - 1),
                (target_x, target_y + 1),
            ];
            let mut hits = 0;
            for &(tx, ty) in &cross {
                if let Some(idx) = battle.unit_at(tx, ty) {
                    if battle.units[idx].is_enemy() {
                        deal_damage(battle, idx, 2);
                        hits += 1;
                    }
                }
            }
            format!(
                "Purifying Flame! Cleansed + {} enemies take 2 dmg!",
                hits
            )
        }
        "Frozen Edge" => {
            // Next 3 basic attacks apply Slow 1 + deal +1 damage
            battle.frozen_edge_charges = 3;
            "Frozen Edge! Next 3 attacks deal +1 dmg + Slow!".to_string()
        }
        _ => "Unknown combo!".to_string(),
    }
}

fn enter_move_targeting(battle: &mut TacticalBattle) {
    let player = &battle.units[0];
    let base_movement = player.effective_movement();
    let movement = (base_movement + battle.player_stance.movement_mod()).max(1);
    let valid = reachable_tiles(battle, player.x, player.y, movement);
    if valid.is_empty() {
        battle.log_message("No valid movement targets.");
        return;
    }
    // Start cursor at first valid tile.
    let (cx, cy) = valid[0];
    battle.phase = TacticalPhase::Targeting {
        mode: TargetMode::Move,
        cursor_x: cx,
        cursor_y: cy,
        valid_targets: valid,
        aoe_preview: vec![],
    };
}

fn enter_attack_targeting(battle: &mut TacticalBattle) {
    let player = &battle.units[0];
    let adjacent = battle.adjacent_enemies(player.x, player.y);
    let px = player.x;
    let py = player.y;

    if adjacent.is_empty() {
        let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        let interactable_tiles: Vec<(i32, i32)> = deltas
            .iter()
            .filter_map(|&(dx, dy)| {
                let nx = px + dx;
                let ny = py + dy;
                if matches!(
                    battle.arena.tile(nx, ny),
                    Some(BattleTile::CargoCrate) | Some(BattleTile::FuelCanister)
                ) {
                    Some((nx, ny))
                } else {
                    None
                }
            })
            .collect();
        if interactable_tiles.is_empty() {
            battle.log_message("No adjacent enemies to attack.");
            return;
        }
        let (cx, cy) = interactable_tiles[0];
        battle.phase = TacticalPhase::Targeting {
            mode: TargetMode::Attack,
            cursor_x: cx,
            cursor_y: cy,
            valid_targets: interactable_tiles,
            aoe_preview: vec![],
        };
        return;
    }

    let valid: Vec<(i32, i32)> = adjacent
        .iter()
        .map(|&idx| (battle.units[idx].x, battle.units[idx].y))
        .collect();
    let (cx, cy) = valid[0];
    battle.phase = TacticalPhase::Targeting {
        mode: TargetMode::Attack,
        cursor_x: cx,
        cursor_y: cy,
        valid_targets: valid,
        aoe_preview: vec![],
    };
}

// ── Targeting phase ──────────────────────────────────────────────────────────

fn handle_targeting(battle: &mut TacticalBattle, key: &str) -> BattleEvent {
    // Extract targeting state. We need to clone mode to avoid borrow issues.
    let (mode, cursor_x, cursor_y, valid_targets) = match &battle.phase {
        TacticalPhase::Targeting {
            mode,
            cursor_x,
            cursor_y,
            valid_targets,
            ..
        } => (mode.clone(), *cursor_x, *cursor_y, valid_targets.clone()),
        _ => return BattleEvent::None,
    };

    match key {
        "Escape" => {
            // Cancel targeting, go back to command.
            battle.phase = TacticalPhase::Command;
            BattleEvent::None
        }
        "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" => {
            // Cycle through valid targets in the pressed direction.
            let (dx, dy) = match key {
                "ArrowUp" => (0, -1),
                "ArrowDown" => (0, 1),
                "ArrowLeft" => (-1, 0),
                "ArrowRight" => (1, 0),
                _ => (0, 0),
            };
            let new_cursor = find_next_target(cursor_x, cursor_y, dx, dy, &valid_targets);
            if let TacticalPhase::Targeting {
                cursor_x: cx,
                cursor_y: cy,
                aoe_preview,
                ref mode,
                ..
            } = &mut battle.phase
            {
                *cx = new_cursor.0;
                *cy = new_cursor.1;
                if let TargetMode::Spell { spell_idx } = mode {
                    if *spell_idx < battle.available_spells.len() {
                        let (_, _, effect) = battle.available_spells[*spell_idx];
                        let ppx = battle.units[0].x;
                        let ppy = battle.units[0].y;
                        *aoe_preview =
                            compute_aoe_preview(&effect, new_cursor.0, new_cursor.1, ppx, ppy);
                    }
                }
            }
            BattleEvent::None
        }
        "Enter" => {
            // Confirm the target.
            confirm_target(battle, &mode, cursor_x, cursor_y)
        }
        _ => BattleEvent::None,
    }
}

/// Find the next valid target in the given direction from current cursor.
fn find_next_target(cx: i32, cy: i32, dx: i32, dy: i32, valid: &[(i32, i32)]) -> (i32, i32) {
    if valid.is_empty() {
        return (cx, cy);
    }
    // Find closest valid target in the pressed direction.
    let mut best = (cx, cy);
    let mut best_dist = i32::MAX;
    for &(vx, vy) in valid {
        // Must be in the general direction pressed.
        let along = (vx - cx) * dx + (vy - cy) * dy;
        if along <= 0 && (vx != cx || vy != cy) {
            continue; // wrong direction or same spot
        }
        let dist = manhattan(cx, cy, vx, vy);
        if dist > 0 && dist < best_dist {
            best_dist = dist;
            best = (vx, vy);
        }
    }
    // If nothing found in that direction, wrap around.
    if best == (cx, cy) {
        // Just pick the first valid target that isn't current.
        for &(vx, vy) in valid {
            if vx != cx || vy != cy {
                return (vx, vy);
            }
        }
    }
    best
}

fn confirm_target(battle: &mut TacticalBattle, mode: &TargetMode, tx: i32, ty: i32) -> BattleEvent {
    match mode {
        TargetMode::Move => {
            let idx = battle.current_unit_idx();
            move_unit(battle, idx, tx, ty);
            battle.log_message(format!("Moved to ({}, {}).", tx, ty));
            battle.player_moved = true;

            // Pick up stolen spells at destination.
            if let Some(pickup_msg) = boss::try_pickup_stolen_spell(battle, tx, ty) {
                battle.log_message(pickup_msg);
            }

            // Check for adjacent ward tiles (Gatekeeper) — auto-destroy.
            let adj = [(tx - 1, ty), (tx + 1, ty), (tx, ty - 1), (tx, ty + 1)];
            for (wx, wy) in adj {
                if boss::try_destroy_ward(battle, wx, wy) {
                    battle.log_message(format!("Ward at ({},{}) shattered!", wx, wy));
                }
            }

            if battle.player_acted {
                battle.phase = TacticalPhase::Resolve {
                    message: "Moved.".to_string(),
                    timer: 10,
                    end_turn: true,
                };
                return BattleEvent::None;
            }
            battle.phase = TacticalPhase::Command;
            BattleEvent::None
        }
        TargetMode::Attack => {
            if battle.arena.tile(tx, ty) == Some(BattleTile::CargoCrate) {
                let px = battle.units[0].x;
                let py = battle.units[0].y;
                let dx = tx - px;
                let dy = ty - py;
                let msgs = crate::combat::tick::push_boulder(battle, tx, ty, dx, dy);
                for msg in &msgs {
                    battle.log_message(msg);
                }
                battle.player_acted = true;
                if battle.player_dead() {
                    battle.phase = TacticalPhase::End {
                        victory: false,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
                if battle.all_enemies_dead() {
                    battle.phase = TacticalPhase::End {
                        victory: true,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
                battle.phase = TacticalPhase::Resolve {
                    message: "Pushed boulder!".to_string(),
                    timer: 15,
                    end_turn: true,
                };
                return BattleEvent::None;
            }

            if battle.arena.tile(tx, ty) == Some(BattleTile::FuelCanister) {
                let msgs = crate::combat::terrain::explode_barrel(battle, tx, ty);
                for msg in &msgs {
                    battle.log_message(msg);
                }
                battle.player_acted = true;
                if battle.player_dead() {
                    battle.phase = TacticalPhase::End {
                        victory: false,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
                if battle.all_enemies_dead() {
                    battle.phase = TacticalPhase::End {
                        victory: true,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
                battle.phase = TacticalPhase::Resolve {
                    message: "Barrel explodes!".to_string(),
                    timer: 15,
                    end_turn: true,
                };
                return BattleEvent::None;
            }

            if let Some(target_idx) = battle.unit_at(tx, ty) {
                // Check if enemy has shields first.
                if !battle.units[target_idx].radical_actions.is_empty()
                    && battle.units[target_idx].radical_armor > 0
                {
                    // For simplicity in MVP, just do basic attack typing.
                }
                let target_pinyin = battle.units[target_idx].pinyin;
                let syllables = vocab::pinyin_syllables(target_pinyin);
                if syllables.len() > 1 {
                    let base_damage = (battle.units[0].damage + battle.player_stance.damage_mod()).max(1);
                    let per_syl = (base_damage as f64 / syllables.len() as f64).ceil() as i32;
                    battle.typing_action = Some(TypingAction::EliteChain {
                        target_unit: target_idx,
                        syllable_progress: 0,
                        total_syllables: syllables.len(),
                        damage_per_syllable: per_syl.max(1),
                        damage_dealt: 0,
                    });
                    battle.typing_buffer.clear();
                    let hanzi = battle.units[target_idx].hanzi;
                    battle.log_message(format!(
                        "Chain attack! Type each syllable of {} ({} parts)",
                        hanzi,
                        syllables.len()
                    ));
                } else {
                    battle.typing_action = Some(TypingAction::BasicAttack {
                        target_unit: target_idx,
                    });
                    battle.typing_buffer.clear();
                    battle.log_message("Type the pinyin to attack!");
                }
                battle.phase = TacticalPhase::Command;
            }
            BattleEvent::None
        }
        TargetMode::Spell { spell_idx } => {
            if *spell_idx < battle.available_spells.len() {
                let (hanzi, _pinyin, effect) = battle.available_spells[*spell_idx];
                battle.typing_action = Some(TypingAction::SpellCast {
                    spell_idx: *spell_idx,
                    target_x: tx,
                    target_y: ty,
                    effect,
                });
                battle.typing_buffer.clear();
                battle.log_message(format!("Type pinyin for {} to cast!", hanzi));
            }
            battle.phase = TacticalPhase::Command;
            BattleEvent::None
        }
        TargetMode::ShieldBreak => {
            battle.phase = TacticalPhase::Command;
            BattleEvent::None
        }
        TargetMode::Skill => {
            if let Some(ability_idx) = battle.selected_radical_ability.take() {
                if ability_idx < battle.player_radical_abilities.len() {
                    let target = battle.unit_at(tx, ty);
                    let target_idx = target.unwrap_or(0);
                    let (radical_str, ability) = battle.player_radical_abilities[ability_idx];
                    let msg = crate::combat::radical::apply_player_radical_ability(
                        battle, 0, target_idx, ability,
                    );
                    battle.log_message(&msg);
                    battle.consumed_radicals.push(radical_str);
                    battle.player_radical_abilities.remove(ability_idx);
                    battle.player_acted = true;
                    return try_end_player_turn(battle);
                }
            }
            battle.phase = TacticalPhase::Command;
            BattleEvent::None
        }
    }
}

// ── Typing phase ─────────────────────────────────────────────────────────────

fn handle_typing(battle: &mut TacticalBattle, key: &str) -> BattleEvent {
    match key {
        "Escape" => {
            // Cancel typing.
            battle.typing_action = None;
            battle.typing_buffer.clear();
            battle.log_message("Attack cancelled.");
            BattleEvent::None
        }
        "Backspace" => {
            battle.typing_buffer.pop();
            BattleEvent::None
        }
        "Enter" => submit_typing(battle),
        _ => {
            // Only accept alphanumeric + space for pinyin input.
            if key.len() == 1 {
                let ch = key.chars().next().unwrap();
                if ch.is_ascii_alphanumeric() || ch == ' ' {
                    battle.typing_buffer.push(ch);
                }
            }
            BattleEvent::None
        }
    }
}

fn submit_typing(battle: &mut TacticalBattle) -> BattleEvent {
    let action = match battle.typing_action.take() {
        Some(a) => a,
        None => return BattleEvent::None,
    };

    let input = battle.typing_buffer.clone();
    battle.typing_buffer.clear();

    match action {
        TypingAction::BasicAttack { target_unit } => {
            resolve_basic_attack(battle, target_unit, &input)
        }
        TypingAction::SpellCast {
            spell_idx,
            target_x,
            target_y,
            effect,
        } => resolve_spell_cast(battle, spell_idx, target_x, target_y, effect, &input),
        TypingAction::ShieldBreak {
            target_unit,
            component,
        } => resolve_shield_break(battle, target_unit, component, &input),
        TypingAction::EliteChain {
            target_unit,
            syllable_progress,
            total_syllables,
            damage_per_syllable,
            damage_dealt,
        } => resolve_elite_chain(
            battle,
            target_unit,
            syllable_progress,
            total_syllables,
            damage_per_syllable,
            damage_dealt,
            &input,
        ),
    }
}

fn resolve_basic_attack(
    battle: &mut TacticalBattle,
    target_idx: usize,
    input: &str,
) -> BattleEvent {
    if target_idx >= battle.units.len() || !battle.units[target_idx].alive {
        battle.log_message("Target is gone.");
        return try_end_player_turn(battle);
    }

    if !battle.units[target_idx].is_enemy() {
        battle.log_message("Invalid target.");
        return BattleEvent::None;
    }

    let correct = check_attack_pinyin(battle, target_idx, input);

    let correct = if correct && battle.weather == Weather::DebrisStorm {
        let roll = (battle.turn_number as u64 * 7 + target_idx as u64 * 13) % 100;
        if roll < 10 {
            battle.log_message("Sandstorm obscures your aim — miss!");
            false
        } else {
            true
        }
    } else {
        correct
    };

    let target_hanzi = battle.units[target_idx].hanzi;

    if correct {
        if battle.units[target_idx].radical_dodge {
            battle.units[target_idx].radical_dodge = false;
            battle.last_answer = Some((target_hanzi, true));
            battle.audio_events.push(AudioEvent::TypingCorrect);
            battle.log_message(format!(
                "{} dodges the attack!",
                battle.units[target_idx].hanzi
            ));
            battle.player_acted = true;
            battle.typing_action = None;
            battle.phase = TacticalPhase::Resolve {
                message: "Dodged!".to_string(),
                timer: 20,
                end_turn: true,
            };
            return BattleEvent::None;
        }

        battle.last_answer = Some((target_hanzi, true));
        battle.combo_streak += 1;
        battle.audio_events.push(AudioEvent::TypingCorrect);
        let combo = battle.combo_multiplier();
        let flank = flank_bonus(battle, 0, target_idx);
        let base_damage = (battle.units[0].damage + battle.player_stance.damage_mod()).max(1);

        let focus_cost = target_hanzi.chars().count().max(1) as i32;
        let focus_penalty = if battle.focus < focus_cost { 0.65 } else { 1.0 };
        battle.focus = (battle.focus - focus_cost).max(0);

        let synergy_bonus = if battle.radical_synergy_streak >= 2 {
            (1.0 + 0.25 * (battle.radical_synergy_streak - 1) as f64).min(1.5)
        } else {
            1.0
        };

        // CriticalStrike + Backstab synergy: guaranteed crit from behind
        let crit_multiplier = if crate::combat::action::critical_backstab_check(battle, target_idx) {
            battle.log_message("⚔💀 Critical Backstab! Guaranteed critical hit!");
            battle.audio_events.push(AudioEvent::CriticalHit);
            2.0
        } else {
            // Normal crit check from CriticalStrike equipment
            let crit_chance: i32 = battle.player_equip_effects.iter().filter_map(|e| {
                if let crate::player::EquipEffect::CriticalStrike(pct) = e { Some(*pct) } else { None }
            }).sum();
            if crit_chance > 0 {
                let roll = (battle.turn_number as u64 * 11 + target_idx as u64 * 7 + battle.combo_streak as u64) % 100;
                if (roll as i32) < crit_chance {
                    battle.log_message("⚔ Critical strike!");
                    battle.audio_events.push(AudioEvent::CriticalHit);
                    2.0
                } else {
                    1.0
                }
            } else {
                1.0
            }
        };

        let raw = (base_damage as f64 * combo * (1.0 + flank) * focus_penalty * synergy_bonus * crit_multiplier)
            .ceil() as i32;
        // Frozen Edge combo bonus: +1 damage + Slow on next 3 basic attacks
        let frozen_bonus = if battle.frozen_edge_charges > 0 {
            battle.frozen_edge_charges -= 1;
            1
        } else {
            0
        };
        let (actual, wuxing_label) = deal_damage_from(battle, 0, target_idx, raw + frozen_bonus);
        if frozen_bonus > 0 {
            battle.units[target_idx]
                .statuses
                .push(crate::status::StatusInstance::new(crate::status::StatusKind::Slow, 1));
            battle.log_message("❄ Frozen Edge! +1 dmg + Slow!");
        }

        let tier = battle.combo_tier_name();
        let flank_label = if flank >= 0.50 {
            battle.audio_events.push(AudioEvent::CriticalHit);
            " Backstab!"
        } else if flank >= 0.25 {
            battle.audio_events.push(AudioEvent::CriticalHit);
            " Flanked!"
        } else {
            ""
        };
        let msg = if tier.is_empty() {
            format!("Hit for {} damage!{}", actual, flank_label)
        } else {
            format!("{} combo! Hit for {} damage!{}", tier, actual, flank_label)
        };
        battle.log_message(&msg);
        if let Some(wl) = wuxing_label {
            battle.log_message(wl);
        }

        if let Some(ability_idx) = battle.selected_radical_ability.take() {
            if ability_idx < battle.player_radical_abilities.len() {
                let (radical_str, ability) = battle.player_radical_abilities[ability_idx];
                let ability_msg = crate::combat::radical::apply_player_radical_ability(
                    battle, 0, target_idx, ability,
                );
                battle.log_message(&ability_msg);
                battle.consumed_radicals.push(radical_str);
                battle.player_radical_abilities.remove(ability_idx);
            }
        }

        if crate::status::has_envenomed(&battle.units[0].statuses) {
            battle.units[target_idx]
                .statuses
                .push(StatusInstance::new(StatusKind::Poison { damage: 1 }, 3));
            battle.log_message("Poison coats the wound!");
            battle.audio_events.push(AudioEvent::StatusPoison);
            // Check status combos after applying poison
            let combo_msgs = crate::combat::action::check_status_combos(battle, target_idx);
            for m in &combo_msgs {
                battle.log_message(m);
            }
        }

        // LifeSteal + Poison synergy: drain extra 1 HP from poisoned enemies
        if !battle.units[target_idx].alive {
            let has_lifesteal = battle.player_equip_effects.iter().any(|e| {
                matches!(e, crate::player::EquipEffect::LifeSteal(_))
            });
            let was_poisoned = battle.units[target_idx]
                .statuses
                .iter()
                .any(|s| matches!(s.kind, StatusKind::Poison { .. }));
            if has_lifesteal && was_poisoned {
                battle.units[0].hp = (battle.units[0].hp + 1).min(battle.units[0].max_hp);
                battle.log_message("🧛 LifeSteal drains extra from poisoned foe! (+1 HP)");
            }
        }

        if battle.units[target_idx].alive && (flank >= 0.50 || battle.combo_streak >= 5) {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let kb_msgs = apply_knockback(battle, target_idx, px, py);
            for m in &kb_msgs {
                battle.log_message(m);
            }
        }

        if battle.units[target_idx].charge_remaining.is_some() {
            battle.units[target_idx].charge_remaining = None;
            battle.log_message("Charge interrupted!");
        }

        let killed_element = battle.units[target_idx].wuxing_element;
        if !battle.units[target_idx].alive {
            if let Some(elem) = killed_element {
                let elem_name = wuxing_element_name(&elem);
                if battle.radical_synergy_radical == Some(elem_name) {
                    battle.radical_synergy_streak += 1;
                } else {
                    battle.radical_synergy_radical = Some(elem_name);
                    battle.radical_synergy_streak = 1;
                }
                if battle.radical_synergy_streak >= 3 {
                    let splash_dmg = if battle.radical_synergy_streak >= 4 {
                        4
                    } else {
                        2
                    };
                    let tx = battle.units[target_idx].x;
                    let ty = battle.units[target_idx].y;
                    let splash_targets: Vec<usize> = (1..battle.units.len())
                        .filter(|&i| {
                            battle.units[i].alive
                                && battle.units[i].is_enemy()
                                && manhattan(battle.units[i].x, battle.units[i].y, tx, ty) <= 1
                        })
                        .collect();
                    for si in splash_targets {
                        deal_damage(battle, si, splash_dmg);
                    }
                    battle.log_message(format!(
                        "{} synergy x{}! AoE splash for {} damage!",
                        elem_name, battle.radical_synergy_streak, splash_dmg
                    ));
                }
            } else {
                battle.radical_synergy_radical = None;
                battle.radical_synergy_streak = 0;
            }

            battle.chengyu_history.push(target_hanzi.to_string());
            if battle.chengyu_history.len() > 4 {
                battle.chengyu_history.remove(0);
            }
            if let Some(effect_msg) = check_chengyu_combo(battle) {
                battle.log_message(&effect_msg);
            }
        }

        if !battle.units[target_idx].alive {
            battle.log_message("Enemy defeated!");
            if battle.all_enemies_dead() {
                battle.phase = TacticalPhase::End {
                    victory: true,
                    timer: 60,
                };
                return BattleEvent::None;
            }
        }

        battle.player_acted = true;
        battle.phase = TacticalPhase::Resolve {
            message: msg,
            timer: 30,
            end_turn: true,
        };
        BattleEvent::None
    } else {
        let partial = check_partial_pinyin(battle, target_idx, input);
        if partial {
            battle.last_answer = Some((target_hanzi, false));
            battle.combo_streak = 0;
            battle.selected_radical_ability = None;
            battle.audio_events.push(AudioEvent::TypingError);
            let base_damage = (battle.units[0].damage + battle.player_stance.damage_mod()).max(1);
            let half_dmg = (base_damage / 2).max(1);
            let actual = deal_damage(battle, target_idx, half_dmg);
            let msg = format!("Close! Wrong tone. {} half-damage.", actual);
            battle.log_message(&msg);

            if !battle.units[target_idx].alive {
                battle.log_message("Enemy defeated!");
                if battle.all_enemies_dead() {
                    battle.phase = TacticalPhase::End {
                        victory: true,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
            }

            battle.player_acted = true;
            battle.phase = TacticalPhase::Resolve {
                message: msg,
                timer: 24,
                end_turn: true,
            };
            BattleEvent::None
        } else {
            battle.last_answer = Some((target_hanzi, false));
            battle.combo_streak = 0;
            battle.selected_radical_ability = None;
            battle.audio_events.push(AudioEvent::TypingError);
            let miss_msg = format!("Wrong! '{}' is incorrect.", input);
            battle.log_message(&miss_msg);

            for i in 1..battle.units.len() {
                if battle.units[i].alive
                    && battle.units[i].boss_kind == Some(BossKind::DriftLeviathan)
                {
                    if let Some(steal_msg) = boss::steal_spell(battle, i) {
                        battle.log_message(steal_msg);
                    }
                    break;
                }
            }

            if battle.units[target_idx].alive {
                let enemy_dmg = battle.units[target_idx].damage;
                let actual = deal_damage(battle, 0, enemy_dmg);
                battle.log_message(format!("Counter-attack! {} damage!", actual));

                if battle.units[0].hp <= 0 {
                    battle.units[0].alive = false;
                    battle.phase = TacticalPhase::End {
                        victory: false,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
            }
            battle.phase = TacticalPhase::Resolve {
                message: miss_msg,
                timer: 24,
                end_turn: false,
            };
            BattleEvent::None
        }
    }
}

fn resolve_spell_cast(
    battle: &mut TacticalBattle,
    spell_idx: usize,
    target_x: i32,
    target_y: i32,
    effect: SpellEffect,
    input: &str,
) -> BattleEvent {
    if spell_idx >= battle.available_spells.len() {
        battle.log_message("Spell no longer available.");
        return BattleEvent::None;
    }

    let spell_hanzi = battle.available_spells[spell_idx].0;

    let correct = if let Some(entry) = vocab::vocab_entry_by_hanzi(spell_hanzi) {
        vocab::check_pinyin(entry, input)
    } else {
        let stored_pinyin = battle.available_spells[spell_idx].1;
        stored_pinyin.eq_ignore_ascii_case(&input.replace(' ', ""))
    };

    if !correct {
        battle.last_answer = Some((spell_hanzi, false));
        battle.combo_streak = 0;
        battle.audio_events.push(AudioEvent::TypingError);
        let miss_msg = format!("Wrong! '{}' — spell fizzles.", input);
        battle.log_message(&miss_msg);
        battle.phase = TacticalPhase::Resolve {
            message: miss_msg,
            timer: 24,
            end_turn: false,
        };
        return BattleEvent::None;
    }

    battle.last_answer = Some((spell_hanzi, true));
    battle.combo_streak += 1;
    battle.audio_events.push(AudioEvent::TypingCorrect);
    battle
        .audio_events
        .push(AudioEvent::SpellElement(spell_effect_school(&effect).to_string()));

    let spell_power = battle.player_stance.spell_power_mod();

    let msg = match effect {
        SpellEffect::FireAoe(dmg) => {
            let rain_penalty = if battle.weather == Weather::CoolantLeak {
                1
            } else {
                0
            };
            let dmg = (dmg + spell_power - rain_penalty).max(1);
            let school = spell_effect_school(&effect);
            let mut cross = vec![
                (target_x, target_y),
                (target_x - 1, target_y),
                (target_x + 1, target_y),
                (target_x, target_y - 1),
                (target_x, target_y + 1),
            ];
            // SpellPowerBoost + terrain spell → affect 1 extra tile
            if crate::combat::action::spell_power_extra_tiles(battle) {
                cross.push((target_x - 1, target_y - 1));
                cross.push((target_x + 1, target_y - 1));
                cross.push((target_x - 1, target_y + 1));
                cross.push((target_x + 1, target_y + 1));
                battle.log_message("📖 SpellPower expands the terrain effect!");
            }
            let mut total_hits = 0;
            for &(cx, cy) in &cross {
                if let Some(idx) = battle.unit_at(cx, cy) {
                    if battle.units[idx].is_enemy() {
                        let resist = boss::elementalist_resistance(battle, idx, school);
                        let bonus = tile_spell_bonus(battle, idx);
                        let final_dmg = ((dmg + bonus) as f64 * resist).ceil() as i32;
                        deal_damage(battle, idx, final_dmg);
                        total_hits += 1;
                    }
                }
            }
            let terrain_msgs = apply_terrain_interactions(
                battle,
                TerrainSource::FireAbility,
                &cross,
            );
            for tm in &terrain_msgs {
                battle.log_message(tm);
            }
            format!(
                "Fire erupts! Hit {} enemies for {} damage!",
                total_hits, dmg
            )
        }
        SpellEffect::Heal(amt) => {
            let unit = &mut battle.units[0];
            let healed = amt.min(unit.max_hp - unit.hp);
            unit.hp = (unit.hp + amt).min(unit.max_hp);
            battle.audio_events.push(AudioEvent::Heal);
            format!("Healed for {} HP!", healed)
        }
        SpellEffect::Reveal => {
            let mut revealed = 0;
            for i in 0..battle.arena.tiles.len() {
                if battle.arena.tiles[i] == BattleTile::MineTile {
                    battle.arena.tiles[i] = BattleTile::MineTileRevealed;
                    revealed += 1;
                }
            }
            if revealed > 0 {
                format!(
                    "The battlefield pulses with insight! {} hidden traps revealed!",
                    revealed
                )
            } else {
                "The battlefield pulses with insight!".to_string()
            }
        }
        SpellEffect::Shield => {
            battle.units[0].defending = true;
            battle.audio_events.push(AudioEvent::ShieldBlock);
            "A barrier forms around you!".to_string()
        }
        SpellEffect::StrongHit(_dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: px as f64,
                from_y: py as f64,
                to_x: target_x,
                to_y: target_y,
                progress: 0.0,
                speed: 0.10,
                arc_height: 0.3,
                effect: ProjectileEffect::SpellHit(effect),
                owner_idx: 0,
                glyph: "⚔",
                color: "#ffcc33",
                done: false,
            });
            battle.audio_events.push(AudioEvent::ProjectileLaunch);
            "Powerful strike launched!".to_string()
        }
        SpellEffect::Drain(_dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: px as f64,
                from_y: py as f64,
                to_x: target_x,
                to_y: target_y,
                progress: 0.0,
                speed: 0.07,
                arc_height: 0.5,
                effect: ProjectileEffect::SpellHit(effect),
                owner_idx: 0,
                glyph: "🩸",
                color: "#aa44ff",
                done: false,
            });
            battle.audio_events.push(AudioEvent::ProjectileLaunch);
            "Draining force launched!".to_string()
        }
        SpellEffect::Stun => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: px as f64,
                from_y: py as f64,
                to_x: target_x,
                to_y: target_y,
                progress: 0.0,
                speed: 0.12,
                arc_height: 0.2,
                effect: ProjectileEffect::SpellHit(effect),
                owner_idx: 0,
                glyph: "⚡",
                color: "#44ddff",
                done: false,
            });
            battle.audio_events.push(AudioEvent::ProjectileLaunch);
            "Lightning bolt launched!".to_string()
        }
        SpellEffect::Pacify => {
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    battle.units[idx].hp = 0;
                    battle.units[idx].alive = false;
                    format!("{} is pacified!", battle.units[idx].hanzi)
                } else {
                    "No target there.".to_string()
                }
            } else {
                "Peace finds no one.".to_string()
            }
        }
        SpellEffect::Slow(_turns) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: px as f64,
                from_y: py as f64,
                to_x: target_x,
                to_y: target_y,
                progress: 0.0,
                speed: 0.06,
                arc_height: 0.4,
                effect: ProjectileEffect::SpellHit(effect),
                owner_idx: 0,
                glyph: "❄",
                color: "#88ccff",
                done: false,
            });
            battle.audio_events.push(AudioEvent::ProjectileLaunch);
            "Freezing bolt launched!".to_string()
        }
        SpellEffect::Teleport => {
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    let (px, py) = (battle.units[0].x, battle.units[0].y);
                    let (ex, ey) = (battle.units[idx].x, battle.units[idx].y);
                    battle.units[0].x = ex;
                    battle.units[0].y = ey;
                    battle.units[idx].x = px;
                    battle.units[idx].y = py;
                    format!("Swapped positions with {}!", battle.units[idx].hanzi)
                } else {
                    "No target there.".to_string()
                }
            } else {
                "The spell finds no anchor.".to_string()
            }
        }
        SpellEffect::Poison(_dmg, _turns) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: px as f64,
                from_y: py as f64,
                to_x: target_x,
                to_y: target_y,
                progress: 0.0,
                speed: 0.06,
                arc_height: 0.6,
                effect: ProjectileEffect::SpellHit(effect),
                owner_idx: 0,
                glyph: "☠",
                color: "#44ff44",
                done: false,
            });
            battle.audio_events.push(AudioEvent::ProjectileLaunch);
            "Poison bolt launched!".to_string()
        }
        SpellEffect::FocusRestore(amt) => {
            battle.focus = (battle.focus + amt).min(battle.max_focus);
            format!("Focus restored by {}!", amt)
        }
        SpellEffect::ArmorBreak => {
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    let stripped = battle.units[idx].radical_armor;
                    battle.units[idx].radical_armor = 0;
                    format!(
                        "{}'s armor shattered! ({} armor removed)",
                        battle.units[idx].hanzi, stripped
                    )
                } else {
                    "No target there.".to_string()
                }
            } else {
                "The force hits nothing.".to_string()
            }
        }
        SpellEffect::Dash(dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let path = line_between(px, py, target_x, target_y);
            let mut hits = 0;
            for &(tx, ty) in &path {
                if let Some(idx) = battle.unit_at(tx, ty) {
                    if battle.units[idx].is_enemy() && battle.units[idx].alive {
                        deal_damage(battle, idx, dmg);
                        hits += 1;
                    }
                }
            }
            battle.units[0].x = target_x;
            battle.units[0].y = target_y;
            battle.player_moved = true;
            if hits > 0 {
                format!("Dashed through {} enemies for {} damage each!", hits, dmg)
            } else {
                "Dashed to new position!".to_string()
            }
        }
        SpellEffect::Pierce(dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let dx = (target_x - px).signum();
            let dy = (target_y - py).signum();
            let mut hits = 0;
            let (mut x, mut y) = (px, py);
            for _ in 0..6 {
                x += dx;
                y += dy;
                if x < 0
                    || y < 0
                    || x >= battle.arena.width as i32
                    || y >= battle.arena.height as i32
                {
                    break;
                }
                if let Some(BattleTile::CoverBarrier) = battle.arena.tile(x, y) {
                    break;
                }
                if let Some(idx) = battle.unit_at(x, y) {
                    if battle.units[idx].is_enemy() && battle.units[idx].alive {
                        deal_damage(battle, idx, dmg);
                        hits += 1;
                    }
                }
            }
            if hits > 0 {
                format!("Piercing bolt hits {} enemies for {} each!", hits, dmg)
            } else {
                "The bolt pierces through empty air.".to_string()
            }
        }
        SpellEffect::PullToward => {
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    let px = battle.units[0].x;
                    let py = battle.units[0].y;
                    let ex = battle.units[idx].x;
                    let ey = battle.units[idx].y;
                    let dx = (px - ex).signum();
                    let dy = (py - ey).signum();
                    let mut dest_x = ex;
                    let mut dest_y = ey;
                    for _ in 0..3 {
                        let nx = dest_x + dx;
                        let ny = dest_y + dy;
                        if !battle.arena.in_bounds(nx, ny) {
                            break;
                        }
                        if let Some(t) = battle.arena.tile(nx, ny) {
                            if !t.is_walkable() {
                                break;
                            }
                        }
                        if battle.unit_at(nx, ny).is_some() && !(nx == px && ny == py) {
                            break;
                        }
                        if nx == px && ny == py {
                            break;
                        }
                        dest_x = nx;
                        dest_y = ny;
                    }
                    let pulled = (ex - dest_x).abs() + (ey - dest_y).abs();
                    battle.units[idx].x = dest_x;
                    battle.units[idx].y = dest_y;
                    format!(
                        "Pulled {} {} tiles closer!",
                        battle.units[idx].hanzi, pulled
                    )
                } else {
                    "No target there.".to_string()
                }
            } else {
                "The pull finds no anchor.".to_string()
            }
        }
        SpellEffect::KnockBack(dmg) => {
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    let px = battle.units[0].x;
                    let py = battle.units[0].y;
                    deal_damage(battle, idx, dmg);
                    let kb1 = apply_knockback(battle, idx, px, py);
                    for m in &kb1 {
                        battle.log_message(m);
                    }
                    if battle.units[idx].alive {
                        let kb2 = apply_knockback(battle, idx, px, py);
                        for m in &kb2 {
                            battle.log_message(m);
                        }
                    }
                    format!(
                        "Knocked {} back with {} damage!",
                        battle.units[idx].hanzi, dmg
                    )
                } else {
                    "No target there.".to_string()
                }
            } else {
                "The force hits nothing.".to_string()
            }
        }
        SpellEffect::Thorns(turns) => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Thorns, turns));
            format!("Thorns aura active for {} turns!", turns)
        }
        SpellEffect::Cone(dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = compute_aoe_preview(&effect, target_x, target_y, px, py);
            let school = spell_effect_school(&effect);
            let mut total_hits = 0;
            for &(cx, cy) in &preview {
                if let Some(idx) = battle.unit_at(cx, cy) {
                    if battle.units[idx].is_enemy() {
                        let resist = boss::elementalist_resistance(battle, idx, school);
                        let bonus = tile_spell_bonus(battle, idx);
                        let final_dmg = ((dmg + bonus) as f64 * resist).ceil() as i32;
                        deal_damage(battle, idx, final_dmg);
                        total_hits += 1;
                    }
                }
            }
            format!("Cone blast hits {} enemies for {} damage!", total_hits, dmg)
        }
        SpellEffect::Wall(len) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let dx = (target_x - px).signum();
            let dy = (target_y - py).signum();
            let mut placed = 0;
            let half = len / 2;
            for i in -half..=half {
                let (wx, wy) = if dx != 0 && dy == 0 {
                    (target_x, target_y + i)
                } else if dy != 0 && dx == 0 {
                    (target_x + i, target_y)
                } else {
                    (target_x + i, target_y)
                };
                if battle.arena.in_bounds(wx, wy)
                    && battle.unit_at(wx, wy).is_none()
                    && battle.arena.tile(wx, wy) != Some(BattleTile::CoverBarrier)
                {
                    battle.arena.set_tile(wx, wy, BattleTile::CoverBarrier);
                    placed += 1;
                }
            }
            format!("Raised a wall of {} stone pillars!", placed)
        }
        SpellEffect::OilSlick => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = compute_aoe_preview(&effect, target_x, target_y, px, py);
            let mut placed = 0;
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        if tile.is_walkable() && tile != BattleTile::Lubricant {
                            battle.arena.set_tile(tx, ty, BattleTile::Lubricant);
                            placed += 1;
                        }
                    }
                }
            }
            format!("Oil slick covers {} tiles!", placed)
        }
        SpellEffect::FreezeGround(dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = compute_aoe_preview(&effect, target_x, target_y, px, py);
            let mut frozen = 0;
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        if matches!(tile, BattleTile::CoolantPool | BattleTile::MetalFloor) {
                            battle.arena.set_tile(tx, ty, BattleTile::FrozenCoolant);
                            frozen += 1;
                        }
                    }
                    if let Some(idx) = battle.unit_at(tx, ty) {
                        let school = spell_effect_school(&effect);
                        let resist = boss::elementalist_resistance(battle, idx, school);
                        let bonus = tile_spell_bonus(battle, idx);
                        let final_dmg = ((dmg + bonus) as f64 * resist).ceil() as i32;
                        deal_damage(battle, idx, final_dmg);
                        battle.units[idx]
                            .statuses
                            .push(StatusInstance::new(StatusKind::Slow, 2));
                    }
                }
            }
            format!("Ground freezes! {} tiles frozen, {} damage!", frozen, dmg)
        }
        SpellEffect::Ignite => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = compute_aoe_preview(&effect, target_x, target_y, px, py);
            let mut burned = 0;
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        match tile {
                            BattleTile::WiringPanel | BattleTile::ElectrifiedWire => {
                                battle.arena.set_tile(tx, ty, BattleTile::BlastMark);
                                burned += 1;
                            }
                            BattleTile::Lubricant => {
                                battle.arena.set_tile(tx, ty, BattleTile::BlastMark);
                                burned += 1;
                                // Oil explosion: 3 damage to unit on this tile
                                if let Some(idx) = battle.unit_at(tx, ty) {
                                    deal_damage(battle, idx, 3);
                                    battle.log_message(&format!(
                                        "Lubricant ignites! {} takes 3 damage!",
                                        battle.units[idx].hanzi
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some(idx) = battle.unit_at(tx, ty) {
                        battle.units[idx]
                            .statuses
                            .push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 3));
                    }
                }
            }
            format!("Plasma ignites {} tiles! Burn applied!", burned)
        }
        SpellEffect::PlantGrowth => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = compute_aoe_preview(&effect, target_x, target_y, px, py);
            let mut grown = 0;
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        if tile == BattleTile::WiringPanel {
                            if battle.unit_at(tx, ty).is_none() {
                                battle.arena.set_tile(tx, ty, BattleTile::PipeTangle);
                                grown += 1;
                            }
                        } else if tile == BattleTile::MetalFloor || tile == BattleTile::BlastMark {
                            battle.arena.set_tile(tx, ty, BattleTile::WiringPanel);
                            grown += 1;
                        }
                    }
                }
            }
            // Heal player 1 if standing on Grass
            let player_tile = battle.arena.tile(battle.units[0].x, battle.units[0].y);
            if player_tile == Some(BattleTile::WiringPanel) {
                let unit = &mut battle.units[0];
                let healed = 1_i32.min(unit.max_hp - unit.hp);
                unit.hp = (unit.hp + 1).min(unit.max_hp);
                if healed > 0 {
                    battle.log_message("Standing on wiring panel restores 1 HP!");
                }
            }
            format!("Nanites spread! {} tiles transformed!", grown)
        }
        SpellEffect::Earthquake(dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = compute_aoe_preview(&effect, target_x, target_y, px, py);
            let school = spell_effect_school(&effect);
            let mut hits = 0;
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        match tile {
                            BattleTile::MetalFloor | BattleTile::WiringPanel | BattleTile::Debris => {
                                battle.arena.set_tile(tx, ty, BattleTile::WeakenedPlating);
                            }
                            BattleTile::WeakenedPlating => {
                                if battle.unit_at(tx, ty).is_none() {
                                    battle.arena.set_tile(tx, ty, BattleTile::BreachedFloor);
                                } else {
                                    battle.arena.set_tile(tx, ty, BattleTile::DamagedFloor);
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some(idx) = battle.unit_at(tx, ty) {
                        let resist = boss::elementalist_resistance(battle, idx, school);
                        let bonus = tile_spell_bonus(battle, idx);
                        let final_dmg = ((dmg + bonus) as f64 * resist).ceil() as i32;
                        deal_damage(battle, idx, final_dmg);
                        hits += 1;
                    }
                }
            }
            // Push cargo crates outward from center
            for &(tx, ty) in &preview {
                if battle.arena.tile(tx, ty) == Some(BattleTile::CargoCrate) {
                    let bdx = (tx - target_x).signum();
                    let bdy = (ty - target_y).signum();
                    if bdx == 0 && bdy == 0 { continue; }
                    let nx = tx + bdx;
                    let ny = ty + bdy;
                    if battle.arena.in_bounds(nx, ny)
                        && battle.arena.tile(nx, ny).map(|t| t.is_walkable()).unwrap_or(false)
                        && battle.unit_at(nx, ny).is_none()
                    {
                        battle.arena.set_tile(tx, ty, BattleTile::MetalFloor);
                        battle.arena.set_tile(nx, ny, BattleTile::CargoCrate);
                    }
                }
            }
            // Apply earthquake terrain interactions (e.g. cracking remaining ground)
            let terrain_msgs = apply_terrain_interactions(
                battle,
                TerrainSource::Earthquake,
                &preview,
            );
            for tm in &terrain_msgs {
                battle.log_message(tm);
            }
            format!("The deck shakes! {} damage to {} units!", dmg, hits)
        }
        SpellEffect::Sanctify(heal) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = compute_aoe_preview(&effect, target_x, target_y, px, py);
            let mut sanctified = 0;
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        if tile.is_walkable() {
                            battle.arena.set_holy(tx, ty, 3);
                            // Store heal amount in steam_timers (reuse for shield zone heal amount)
                            if let Some(i) = battle.arena.idx(tx, ty) {
                                battle.arena.steam_timers[i] = heal as u8;
                            }
                            sanctified += 1;
                        }
                    }
                }
            }
            format!("Shield field covers {} tiles! Heals {} HP/turn for 3 rounds.", sanctified, heal)
        }
        SpellEffect::FloodWave(dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = compute_aoe_preview(&effect, target_x, target_y, px, py);
            let school = spell_effect_school(&effect);
            let dx = (target_x - px).signum();
            let dy = (target_y - py).signum();
            let mut hits = 0;
            // Push units first, then place water
            let mut push_targets = Vec::new();
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(idx) = battle.unit_at(tx, ty) {
                        if battle.units[idx].is_enemy() {
                            let resist = boss::elementalist_resistance(battle, idx, school);
                            let bonus = tile_spell_bonus(battle, idx);
                            let final_dmg = ((dmg + bonus) as f64 * resist).ceil() as i32;
                            deal_damage(battle, idx, final_dmg);
                            push_targets.push(idx);
                            hits += 1;
                        }
                    }
                }
            }
            // Push units 2 tiles in wave direction
            for idx in push_targets {
                if battle.units[idx].alive {
                    for _ in 0..2 {
                        let nx = battle.units[idx].x + dx;
                        let ny = battle.units[idx].y + dy;
                        if battle.arena.in_bounds(nx, ny)
                            && battle.arena.tile(nx, ny).map(|t| t.is_walkable()).unwrap_or(false)
                            && battle.unit_at(nx, ny).is_none()
                        {
                            battle.units[idx].x = nx;
                            battle.units[idx].y = ny;
                        } else {
                            break;
                        }
                    }
                }
            }
            // Place coolant tiles
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        if tile.is_walkable() && tile != BattleTile::CoolantPool {
                            battle.arena.set_tile(tx, ty, BattleTile::CoolantPool);
                        }
                    }
                }
            }
            format!("Coolant wave hits {} enemies for {} damage!", hits, dmg)
        }
        SpellEffect::SummonBoulder => {
            if battle.arena.in_bounds(target_x, target_y) {
                let tile = battle.arena.tile(target_x, target_y);
                if tile.map(|t| t.is_walkable()).unwrap_or(false)
                    && battle.unit_at(target_x, target_y).is_none()
                {
                    battle.arena.set_tile(target_x, target_y, BattleTile::CargoCrate);
                    "A cargo crate materializes!".to_string()
                } else {
                    "Cannot place crate there!".to_string()
                }
            } else {
                "Target out of bounds.".to_string()
            }
        }
    };

    battle.log_message(&msg);

    // ── Ability combo chain check ──────────────────────────────────────────
    let current_element = spell_effect_element(&effect);
    if let Some(cur_elem) = current_element {
        if let Some(prev_elem) = battle.last_spell_element {
            if battle.turn_number.saturating_sub(battle.last_spell_turn) <= 2 {
                if let Some(combo_name) = spell_combo_name(prev_elem, cur_elem) {
                    let combo_msg =
                        apply_spell_combo(battle, combo_name, target_x, target_y);
                    battle.log_message(format!("⚡ COMBO: {}!", combo_name));
                    battle.log_message(&combo_msg);
                    battle.combo_message =
                        Some(format!("⚡ COMBO: {}!", combo_name));
                    battle.combo_message_timer = 60;
                    battle
                        .audio_events
                        .push(AudioEvent::ComboStrike);
                }
            }
        }
        battle.last_spell_element = Some(cur_elem);
        battle.last_spell_turn = battle.turn_number;
    }

    battle.last_spell_school = Some(spell_effect_school(&effect));
    battle.spent_spell_index = Some(spell_idx);
    battle.player_acted = true;

    if battle.all_enemies_dead() {
        battle.phase = TacticalPhase::End {
            victory: true,
            timer: 60,
        };
        return BattleEvent::None;
    }

    if battle.units[0].hp <= 0 {
        battle.units[0].alive = false;
        battle.phase = TacticalPhase::End {
            victory: false,
            timer: 60,
        };
        return BattleEvent::None;
    }

    if !battle.projectiles.is_empty() {
        battle.phase = TacticalPhase::ProjectileAnimation {
            message: msg,
            end_turn: true,
        };
    } else {
        battle.phase = TacticalPhase::Resolve {
            message: msg,
            timer: 30,
            end_turn: true,
        };
    }
    BattleEvent::None
}

fn resolve_shield_break(
    battle: &mut TacticalBattle,
    target_idx: usize,
    component: &'static str,
    input: &str,
) -> BattleEvent {
    if target_idx >= battle.units.len() || !battle.units[target_idx].alive {
        return try_end_player_turn(battle);
    }

    let correct = if let Some(entry) = vocab::vocab_entry_by_hanzi(component) {
        vocab::check_pinyin(entry, input)
    } else {
        false
    };

    if correct {
        battle.log_message(format!("Shattered {} shield!", component));
        // Remove the first radical action (shield) from the enemy.
        if !battle.units[target_idx].radical_actions.is_empty() {
            battle.units[target_idx].radical_actions.remove(0);
        }
    } else {
        battle.log_message(format!("Shield holds! '{}' incorrect.", input));
    }
    BattleEvent::None
}

fn resolve_elite_chain(
    battle: &mut TacticalBattle,
    target_idx: usize,
    syllable_progress: usize,
    total_syllables: usize,
    damage_per_syllable: i32,
    damage_dealt: i32,
    input: &str,
) -> BattleEvent {
    if target_idx >= battle.units.len() || !battle.units[target_idx].alive {
        battle.log_message("Target is gone.");
        return try_end_player_turn(battle);
    }

    let pinyin = battle.units[target_idx].pinyin;
    let step = vocab::resolve_compound_pinyin_step(pinyin, syllable_progress, input);

    match step {
        vocab::CompoundPinyinStep::Advanced {
            next_progress,
            total,
            ..
        } => {
            let actual = deal_damage(battle, target_idx, damage_per_syllable);
            let new_dealt = damage_dealt + actual;
            battle.log_message(format!(
                "Part {}/{}! Hit for {} damage!",
                next_progress, total, actual
            ));

            if !battle.units[target_idx].alive {
                battle.last_answer = Some((battle.units[target_idx].hanzi, true));
                battle.combo_streak += 1;
                battle.player_acted = true;
                return try_end_player_turn(battle);
            }

            battle.typing_action = Some(TypingAction::EliteChain {
                target_unit: target_idx,
                syllable_progress: next_progress,
                total_syllables,
                damage_per_syllable,
                damage_dealt: new_dealt,
            });
            battle.typing_buffer.clear();
            BattleEvent::None
        }
        vocab::CompoundPinyinStep::Completed { total, .. } => {
            let combo_bonus = 1.0 + (total as f64 - 1.0) * 0.15;
            let flank = flank_bonus(battle, 0, target_idx);
            let final_hit =
                (damage_per_syllable as f64 * combo_bonus * (1.0 + flank)).ceil() as i32;
            let actual = deal_damage(battle, target_idx, final_hit);
            let total_dealt = damage_dealt + actual;

            battle.last_answer = Some((battle.units[target_idx].hanzi, true));
            battle.combo_streak += 1;
            let tier = battle.combo_tier_name();

            let flank_label = if flank >= 0.50 {
                " Backstab!"
            } else if flank >= 0.25 {
                " Flanked!"
            } else {
                ""
            };
            let msg = if tier.is_empty() {
                format!(
                    "Chain complete! {} total damage!{}",
                    total_dealt, flank_label
                )
            } else {
                format!(
                    "{} chain combo! {} total damage!{}",
                    tier, total_dealt, flank_label
                )
            };
            battle.log_message(&msg);

            if crate::status::has_envenomed(&battle.units[0].statuses) {
                battle.units[target_idx]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Poison { damage: 1 }, 3));
                battle.log_message("Venom applied!");
            }

            battle.player_acted = true;
            try_end_player_turn(battle)
        }
        vocab::CompoundPinyinStep::Miss { expected, .. } => {
            battle.last_answer = Some((battle.units[target_idx].hanzi, false));
            battle.combo_streak = 0;
            battle.log_message(format!(
                "Chain broken! Expected '{}', got '{}'",
                expected, input
            ));

            let counter_dmg = battle.units[target_idx].damage / 2;
            if counter_dmg > 0 {
                battle.units[0].hp -= counter_dmg;
                battle.log_message(format!("Counter-attack! {} damage!", counter_dmg));
                if battle.units[0].hp <= 0 {
                    battle.units[0].alive = false;
                    battle.phase = TacticalPhase::End {
                        victory: false,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
            }

            battle.player_acted = true;
            try_end_player_turn(battle)
        }
    }
}

/// Check if the typed pinyin matches the enemy's hanzi.
fn check_attack_pinyin(battle: &TacticalBattle, target_idx: usize, input: &str) -> bool {
    let unit = &battle.units[target_idx];
    if !unit.hanzi.is_empty() {
        if let Some(entry) = vocab::vocab_entry_by_hanzi(unit.hanzi) {
            return vocab::check_pinyin(entry, input);
        }
        return unit.pinyin.eq_ignore_ascii_case(&input.replace(' ', ""));
    }
    false
}

fn check_partial_pinyin(battle: &TacticalBattle, target_idx: usize, input: &str) -> bool {
    let unit = &battle.units[target_idx];
    if !unit.hanzi.is_empty() {
        if let Some(entry) = vocab::vocab_entry_by_hanzi(unit.hanzi) {
            return vocab::check_pinyin_partial(entry, input);
        }
    }
    false
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

fn wuxing_element_name(elem: &WuxingElement) -> &'static str {
    match elem {
        WuxingElement::Water => "Water",
        WuxingElement::Fire => "Fire",
        WuxingElement::Metal => "Metal",
        WuxingElement::Wood => "Wood",
        WuxingElement::Earth => "Earth",
    }
}

const CHENGYU_LIST: &[(&str, &str)] = &[
    ("\u{4e00}\u{5fc3}\u{4e00}\u{610f}", "Wholehearted"),
    ("\u{4e07}\u{4e8b}\u{5982}\u{610f}", "Everything goes well"),
    ("\u{5929}\u{4e0b}\u{592a}\u{5e73}", "Peace under heaven"),
    ("\u{5fc3}\u{60f3}\u{4e8b}\u{6210}", "Dreams come true"),
    ("\u{5927}\u{5f00}\u{773c}\u{754c}", "Eye-opening"),
    ("\u{4e03}\u{4e0a}\u{516b}\u{4e0b}", "At sixes and sevens"),
    ("\u{4e94}\u{5149}\u{5341}\u{8272}", "Dazzling"),
    ("\u{4e5d}\u{6b7b}\u{4e00}\u{751f}", "Narrow escape"),
    ("\u{534a}\u{9014}\u{800c}\u{5e9f}", "Give up halfway"),
    ("\u{81ea}\u{8a00}\u{81ea}\u{8bed}", "Talk to oneself"),
    ("\u{5165}\u{4e61}\u{968f}\u{4fd7}", "When in Rome"),
    ("\u{9a6c}\u{5230}\u{6210}\u{529f}", "Instant success"),
    ("\u{5927}\u{540c}\u{5c0f}\u{5f02}", "Mostly the same"),
    ("\u{767e}\u{53d1}\u{767e}\u{4e2d}", "Hit every target"),
    ("\u{5343}\u{65b9}\u{767e}\u{8ba1}", "By every means"),
    ("\u{5f00}\u{95e8}\u{89c1}\u{5c71}", "Get to the point"),
    ("\u{4e00}\u{5200}\u{4e24}\u{65ad}", "Cut cleanly"),
    ("\u{4e00}\u{76ee}\u{4e86}\u{7136}", "Crystal clear"),
    ("\u{4e0d}\u{53ef}\u{601d}\u{8bae}", "Incredible"),
    ("\u{6cf0}\u{7136}\u{81ea}\u{82e5}", "Calm and composed"),
    ("\u{5b66}\u{4ee5}\u{81f4}\u{7528}", "Learn to apply"),
    (
        "\u{5927}\u{5668}\u{665a}\u{6210}",
        "Great minds mature slowly",
    ),
    (
        "\u{53e3}\u{662f}\u{5fc3}\u{975e}",
        "Say one thing mean another",
    ),
    (
        "\u{9f99}\u{98de}\u{51e4}\u{821e}",
        "Dragons fly phoenixes dance",
    ),
    ("\u{864e}\u{5934}\u{86c7}\u{5c3e}", "Strong start weak end"),
    ("\u{6c34}\u{6ef4}\u{77f3}\u{7a7f}", "Water wears stone"),
    (
        "\u{98ce}\u{548c}\u{65e5}\u{4e3d}",
        "Gentle breeze sunny day",
    ),
    ("\u{91d1}\u{7389}\u{6ee1}\u{5802}", "Riches fill the hall"),
    ("\u{5929}\u{957f}\u{5730}\u{4e45}", "Everlasting"),
    ("\u{5fc3}\u{5982}\u{6b62}\u{6c34}", "Mind still as water"),
    ("\u{5149}\u{660e}\u{78ca}\u{843d}", "Open and upright"),
    ("\u{4e00}\u{8def}\u{5e73}\u{5b89}", "Safe journey"),
];

fn check_chengyu_combo(battle: &mut TacticalBattle) -> Option<String> {
    if battle.chengyu_history.len() < 4 {
        return None;
    }
    let last4: String = battle.chengyu_history[battle.chengyu_history.len() - 4..].join("");
    for &(idiom, name) in CHENGYU_LIST {
        if last4 == idiom {
            battle.chengyu_history.clear();
            let unit = &mut battle.units[0];
            let heal = (unit.max_hp / 3).max(2);
            unit.hp = (unit.hp + heal).min(unit.max_hp);
            for i in 1..battle.units.len() {
                if battle.units[i].alive && battle.units[i].is_enemy() {
                    battle.units[i].stunned = true;
                }
            }
            return Some(format!(
                "CHENGYU! {} ({})! Heal {} HP, all enemies stunned!",
                idiom, name, heal
            ));
        }
    }
    None
}


