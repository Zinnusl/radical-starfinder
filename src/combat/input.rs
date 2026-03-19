//! Input handling for tactical combat phases.
//!
//! All keyboard input during a `TacticalBattle` flows through
//! `handle_input()` which dispatches based on the current `TacticalPhase`.

use crate::combat::action::{deal_damage, deal_damage_from, defend, flank_bonus, move_unit, wait};
use crate::combat::ai::{choose_action, step_away, step_toward, AiAction};
use crate::combat::boss;
use crate::combat::grid::{
    manhattan, reachable_tiles, tiles_in_range_with_los, weather_adjusted_range,
};
use crate::combat::radical::apply_radical_action;
use crate::combat::terrain::{apply_knockback, apply_terrain_interactions, TerrainSource};
use crate::combat::turn::advance_turn;
use crate::combat::{
    BattleTile, TacticalBattle, TacticalPhase, TargetMode, TypingAction, Weather, WuxingElement,
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
        Some(BattleTile::InkPool) => 1,
        Some(BattleTile::ArcaneGlyph) => 2,
        _ => 0,
    };
    let weather_bonus = match battle.weather {
        Weather::SpiritualInk => 1,
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
        "i" | "I" if !battle.player_acted => {
            if battle.available_items.is_empty() {
                battle.log_message("No items available.");
            } else {
                battle.item_menu_open = true;
                battle.item_cursor = 0;
            }
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
                    crate::player::PlayerClass::Thief | crate::player::PlayerClass::Assassin
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
                    }
                }
            }
            battle.log_message(&desc);
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
                    let base_range = spell_range(&effect);
                    let range = weather_adjusted_range(base_range, battle.weather);
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

fn use_item_in_combat(
    battle: &mut TacticalBattle,
    menu_idx: usize,
    orig_idx: usize,
    item: &crate::player::Item,
) -> BattleEvent {
    use crate::player::Item;
    let msg = match item {
        Item::HealthPotion(amount) => {
            let heal = *amount;
            let unit = &mut battle.units[0];
            unit.hp = (unit.hp + heal).min(unit.max_hp);
            format!("Healed {} HP!", heal)
        }
        Item::PoisonFlask(dmg, turns) => {
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
            format!("Poisoned {} enemies!", adj.len())
        }
        Item::HastePotion(turns) => {
            let turns = *turns;
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Haste, turns));
            format!("Haste for {} turns!", turns)
        }
        Item::StunBomb => {
            let mut stunned_count = 0;
            for i in 1..battle.units.len() {
                if battle.units[i].alive {
                    battle.units[i].stunned = true;
                    stunned_count += 1;
                }
            }
            format!("Stunned {} enemies!", stunned_count)
        }
        Item::TeleportScroll => {
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
        Item::RevealScroll
        | Item::RiceBall(_)
        | Item::MeditationIncense(_)
        | Item::AncestralWine(_) => {
            battle.log_message("This item has no effect in combat.");
            return BattleEvent::None;
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
                // Check if enemy has shields first.
                if !battle.units[target_idx].radical_actions.is_empty()
                    && battle.units[target_idx].radical_armor > 0
                {
                    // For simplicity in MVP, just do basic attack typing.
                }
                let target_pinyin = battle.units[target_idx].pinyin;
                let syllables = vocab::pinyin_syllables(target_pinyin);
                if syllables.len() > 1 {
                    let base_damage = battle.units[0].damage;
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

    let correct = if correct && battle.weather == Weather::Sandstorm {
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
        battle.last_answer = Some((target_hanzi, true));
        battle.combo_streak += 1;
        let combo = battle.combo_multiplier();
        let flank = flank_bonus(battle, 0, target_idx);
        let base_damage = battle.units[0].damage;

        let focus_cost = target_hanzi.chars().count().max(1) as i32;
        let focus_penalty = if battle.focus < focus_cost { 0.65 } else { 1.0 };
        battle.focus = (battle.focus - focus_cost).max(0);

        let synergy_bonus = if battle.radical_synergy_streak >= 2 {
            (1.0 + 0.25 * (battle.radical_synergy_streak - 1) as f64).min(1.5)
        } else {
            1.0
        };

        let raw = (base_damage as f64 * combo * (1.0 + flank) * focus_penalty * synergy_bonus)
            .ceil() as i32;
        let (actual, wuxing_label) = deal_damage_from(battle, 0, target_idx, raw);

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
        if let Some(wl) = wuxing_label {
            battle.log_message(wl);
        }

        if crate::status::has_envenomed(&battle.units[0].statuses) {
            battle.units[target_idx]
                .statuses
                .push(StatusInstance::new(StatusKind::Poison { damage: 1 }, 3));
            battle.log_message("Poison coats the wound!");
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
            let base_damage = battle.units[0].damage;
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
            let miss_msg = format!("Wrong! '{}' is incorrect.", input);
            battle.log_message(&miss_msg);

            for i in 1..battle.units.len() {
                if battle.units[i].alive
                    && battle.units[i].boss_kind == Some(BossKind::RadicalThief)
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
            let rain_penalty = if battle.weather == Weather::Rain {
                1
            } else {
                0
            };
            let dmg = (dmg - rain_penalty).max(1);
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
                        let bonus = tile_spell_bonus(battle, idx);
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
                    let bonus = tile_spell_bonus(battle, idx);
                    let final_dmg = ((dmg + bonus) as f64 * resist).ceil() as i32;
                    let actual = deal_damage(battle, idx, final_dmg);
                    if battle.units[idx].alive {
                        let px = battle.units[0].x;
                        let py = battle.units[0].y;
                        let kb_msgs = apply_knockback(battle, idx, px, py);
                        for m in &kb_msgs {
                            battle.log_message(m);
                        }
                    }
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
                    let bonus = tile_spell_bonus(battle, idx);
                    let final_dmg = ((dmg + bonus) as f64 * resist).ceil() as i32;
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
            let dmg = ((battle.units[unit_idx].damage as f64) * 1.75).ceil() as i32;
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
                    let (actual, wuxing_label) = deal_damage_from(battle, unit_idx, 0, dmg);
                    battle.log_message(format!("{} attacks for {} damage!", name, actual));
                    if let Some(wl) = wuxing_label {
                        battle.log_message(wl);
                    }
                }
                if dmg >= 3 && battle.units[0].alive {
                    let ex = battle.units[unit_idx].x;
                    let ey = battle.units[unit_idx].y;
                    let kb_msgs = apply_knockback(battle, 0, ex, ey);
                    for m in &kb_msgs {
                        battle.log_message(m);
                    }
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
