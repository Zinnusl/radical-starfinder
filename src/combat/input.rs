//! Input handling for tactical combat phases.
//!
//! All keyboard input during a `TacticalBattle` flows through
//! `handle_input()` which dispatches based on the current `TacticalPhase`.

use crate::combat::action::{deal_damage, defend, flank_bonus, move_unit, wait};
use crate::combat::ai::{choose_action, step_away, step_toward, AiAction};
use crate::combat::boss;
use crate::combat::grid::{manhattan, reachable_tiles, tiles_in_range_with_los};
use crate::combat::radical::apply_radical_action;
use crate::combat::terrain::{apply_terrain_interactions, TerrainSource};
use crate::combat::turn::advance_turn;
use crate::combat::{TacticalBattle, TacticalPhase, TargetMode, TypingAction};
use crate::enemy::BossKind;
use crate::radical::SpellEffect;
use crate::status::{tick_statuses, StatusInstance, StatusKind};
use crate::vocab;

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

    // If we're in a typing action, route to typing handler.
    if battle.typing_action.is_some() {
        return handle_typing(battle, key);
    }

    match &battle.phase {
        TacticalPhase::Command => handle_command(battle, key),
        TacticalPhase::Targeting { .. } => handle_targeting(battle, key),
        _ => BattleEvent::None,
    }
}

// ── Command phase ────────────────────────────────────────────────────────────

fn handle_command(battle: &mut TacticalBattle, key: &str) -> BattleEvent {
    match key {
        "m" | "M" if !battle.player_moved => {
            enter_move_targeting(battle);
            BattleEvent::None
        }
        "a" | "A" if !battle.player_acted => {
            enter_attack_targeting(battle);
            BattleEvent::None
        }
        "s" | "S" if !battle.player_acted => {
            if battle.available_spells.is_empty() {
                battle.log_message("No spells available.");
            } else {
                battle.spell_menu_open = true;
                battle.spell_cursor = 0;
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
        "Escape" => BattleEvent::Flee,
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
                SpellEffect::Heal(_) | SpellEffect::Shield | SpellEffect::Reveal => {
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
                    let range = spell_range(&effect);
                    let px = battle.units[0].x;
                    let py = battle.units[0].y;
                    let los_tiles = tiles_in_range_with_los(&battle.arena, px, py, range);

                    let valid: Vec<(i32, i32)> = if matches!(effect, SpellEffect::FireAoe(_)) {
                        los_tiles
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
                    battle.phase = TacticalPhase::Targeting {
                        mode: TargetMode::Spell { spell_idx },
                        cursor_x: cx,
                        cursor_y: cy,
                        valid_targets: valid,
                    };
                }
            }
            BattleEvent::None
        }
        _ => BattleEvent::None,
    }
}

fn spell_range(effect: &SpellEffect) -> i32 {
    match effect {
        SpellEffect::FireAoe(_) => 4,
        SpellEffect::StrongHit(_) => 2,
        SpellEffect::Drain(_) => 1,
        SpellEffect::Stun => 3,
        SpellEffect::Pacify => 3,
        _ => 1,
    }
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
    }
}

fn enter_move_targeting(battle: &mut TacticalBattle) {
    let player = &battle.units[0];
    let movement = player.effective_movement();
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
    };
}

fn enter_attack_targeting(battle: &mut TacticalBattle) {
    let player = &battle.units[0];
    let adjacent = battle.adjacent_enemies(player.x, player.y);
    if adjacent.is_empty() {
        battle.log_message("No adjacent enemies to attack.");
        return;
    }
    // Build valid target positions from adjacent enemies.
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
                ..
            } = &mut battle.phase
            {
                *cx = new_cursor.0;
                *cy = new_cursor.1;
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
            // Find the enemy unit at this position.
            if let Some(target_idx) = battle.unit_at(tx, ty) {
                // Enter typing mode for this enemy.
                // Check if enemy has shields first.
                if !battle.units[target_idx].radical_actions.is_empty()
                    && battle.units[target_idx].radical_armor > 0
                {
                    // For simplicity in MVP, just do basic attack typing.
                }
                battle.typing_action = Some(TypingAction::BasicAttack {
                    target_unit: target_idx,
                });
                battle.typing_buffer.clear();
                battle.phase = TacticalPhase::Command; // Stay in command visually but typing is active.
                battle.log_message("Type the pinyin to attack!");
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

    let target_hanzi = battle.units[target_idx].hanzi;

    if correct {
        battle.last_answer = Some((target_hanzi, true));
        battle.combo_streak += 1;
        let combo = battle.combo_multiplier();
        let flank = flank_bonus(battle, 0, target_idx);
        let base_damage = battle.units[0].damage;
        let raw = (base_damage as f64 * combo * (1.0 + flank)).ceil() as i32;
        let actual = deal_damage(battle, target_idx, raw);

        let tier = battle.combo_tier_name();
        let flank_label = if flank >= 0.50 {
            " Backstab!"
        } else if flank >= 0.25 {
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

        if crate::status::has_envenomed(&battle.units[0].statuses) {
            battle.units[target_idx]
                .statuses
                .push(StatusInstance::new(StatusKind::Poison { damage: 1 }, 3));
            battle.log_message("Poison coats the wound!");
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
        battle.last_answer = Some((target_hanzi, false));
        battle.combo_streak = 0;
        let miss_msg = format!("Wrong! '{}' is incorrect.", input);
        battle.log_message(&miss_msg);

        // RadicalThief steals a spell on wrong answers.
        for i in 1..battle.units.len() {
            if battle.units[i].alive && battle.units[i].boss_kind == Some(BossKind::RadicalThief) {
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

    let msg = match effect {
        SpellEffect::FireAoe(dmg) => {
            let school = spell_effect_school(&effect);
            let cross = [
                (target_x, target_y),
                (target_x - 1, target_y),
                (target_x + 1, target_y),
                (target_x, target_y - 1),
                (target_x, target_y + 1),
            ];
            let mut total_hits = 0;
            for &(cx, cy) in &cross {
                if let Some(idx) = battle.unit_at(cx, cy) {
                    if battle.units[idx].is_enemy() {
                        let resist = boss::elementalist_resistance(battle, idx, school);
                        let bonus = if battle.arena.tile(battle.units[idx].x, battle.units[idx].y)
                            == Some(crate::combat::BattleTile::InkPool)
                        {
                            1
                        } else {
                            0
                        };
                        let final_dmg = ((dmg + bonus) as f64 * resist).ceil() as i32;
                        deal_damage(battle, idx, final_dmg);
                        total_hits += 1;
                    }
                }
            }
            let terrain_msgs = apply_terrain_interactions(
                battle,
                TerrainSource::FireSpell,
                &cross.iter().copied().collect::<Vec<_>>(),
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
            format!("Healed for {} HP!", healed)
        }
        SpellEffect::Reveal => "The battlefield pulses with insight!".to_string(),
        SpellEffect::Shield => {
            battle.units[0].defending = true;
            "A barrier forms around you!".to_string()
        }
        SpellEffect::StrongHit(dmg) => {
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    let resist = boss::elementalist_resistance(battle, idx, "force");
                    let final_dmg = (dmg as f64 * resist).ceil() as i32;
                    let actual = deal_damage(battle, idx, final_dmg);
                    format!("Powerful strike! {} damage!", actual)
                } else {
                    "No target there.".to_string()
                }
            } else {
                "The strike hits empty ground.".to_string()
            }
        }
        SpellEffect::Drain(dmg) => {
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    let resist = boss::elementalist_resistance(battle, idx, "drain");
                    let final_dmg = (dmg as f64 * resist).ceil() as i32;
                    let actual = deal_damage(battle, idx, final_dmg);
                    let unit = &mut battle.units[0];
                    unit.hp = (unit.hp + actual).min(unit.max_hp);
                    format!("Drained {} HP from enemy!", actual)
                } else {
                    "No target there.".to_string()
                }
            } else {
                "The drain dissipates.".to_string()
            }
        }
        SpellEffect::Stun => {
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    let resist = boss::elementalist_resistance(battle, idx, "lightning");
                    if resist < 1.0 {
                        "The stun is resisted!".to_string()
                    } else {
                        battle.units[idx].stunned = true;
                        let stun_msg = format!("{} is stunned!", battle.units[idx].hanzi);
                        let terrain_msgs = apply_terrain_interactions(
                            battle,
                            TerrainSource::LightningSpell,
                            &[(target_x, target_y)],
                        );
                        for tm in &terrain_msgs {
                            battle.log_message(tm);
                        }
                        stun_msg
                    }
                } else {
                    "No target there.".to_string()
                }
            } else {
                "The stun fades.".to_string()
            }
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
    };

    battle.log_message(&msg);

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

    battle.phase = TacticalPhase::Resolve {
        message: msg,
        timer: 30,
        end_turn: true,
    };
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

/// Check if the typed pinyin matches the enemy's hanzi.
fn check_attack_pinyin(battle: &TacticalBattle, target_idx: usize, input: &str) -> bool {
    let unit = &battle.units[target_idx];
    // Use stored hanzi/pinyin on the BattleUnit.
    if !unit.hanzi.is_empty() {
        if let Some(entry) = vocab::vocab_entry_by_hanzi(unit.hanzi) {
            return vocab::check_pinyin(entry, input);
        }
        // Fallback: direct string compare with stored pinyin.
        return unit.pinyin.eq_ignore_ascii_case(&input.replace(' ', ""));
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

    // Boss-specific actions take priority over normal AI.
    if let Some(msg) = boss::boss_action(battle, unit_idx) {
        battle.log_message(msg);
        return BattleEvent::None;
    }

    let action = choose_action(battle, unit_idx);
    let name = enemy_display_name(battle, unit_idx);

    match action {
        AiAction::MoveToward { x, y } => {
            let (fx, fy) = (battle.units[unit_idx].x, battle.units[unit_idx].y);
            if let Some((nx, ny)) = step_toward(battle, fx, fy, x, y) {
                move_unit(battle, unit_idx, nx, ny);
            }
            let dist = manhattan(
                battle.units[unit_idx].x,
                battle.units[unit_idx].y,
                battle.units[0].x,
                battle.units[0].y,
            );
            if dist <= 1 {
                let dmg = battle.units[unit_idx].damage + boss::ink_sage_bonus(battle, unit_idx);
                let multiply = battle.units[unit_idx].radical_multiply;
                let hits = if multiply { 2 } else { 1 };
                battle.units[unit_idx].radical_multiply = false;
                for _ in 0..hits {
                    let actual = deal_damage(battle, 0, dmg);
                    battle.log_message(format!("{} attacks for {} damage!", name, actual));
                }
            }
        }
        AiAction::MoveAway { x, y } => {
            let (fx, fy) = (battle.units[unit_idx].x, battle.units[unit_idx].y);
            if let Some((nx, ny)) = step_away(battle, fx, fy, x, y) {
                move_unit(battle, unit_idx, nx, ny);
                battle.log_message(format!("{} retreats.", name));
            }
        }
        AiAction::MeleeAttack { target_unit } => {
            let dmg = battle.units[unit_idx].damage + boss::ink_sage_bonus(battle, unit_idx);
            let multiply = battle.units[unit_idx].radical_multiply;
            let hits = if multiply { 2 } else { 1 };
            battle.units[unit_idx].radical_multiply = false;
            for _ in 0..hits {
                let actual = deal_damage(battle, target_unit, dmg);
                battle.log_message(format!("{} attacks for {} damage!", name, actual));
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
