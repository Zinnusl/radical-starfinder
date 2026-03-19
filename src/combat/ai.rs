use crate::combat::grid::manhattan;
use crate::combat::{EnemyIntent, TacticalBattle};
use crate::enemy::AiBehavior;

pub enum AiAction {
    MoveToward { x: i32, y: i32 },
    MoveAway { x: i32, y: i32 },
    MeleeAttack { target_unit: usize },
    UseRadicalAction { action_idx: usize },
    Wait,
}

pub fn calculate_all_intents(battle: &mut TacticalBattle) {
    let player_x = battle.units[0].x;
    let player_y = battle.units[0].y;

    for i in 1..battle.units.len() {
        if !battle.units[i].alive {
            battle.units[i].intent = None;
            continue;
        }
        let dist = manhattan(battle.units[i].x, battle.units[i].y, player_x, player_y);
        let ai = battle.units[i].ai;
        let has_radical = !battle.units[i].radical_actions.is_empty();
        let stunned = battle.units[i].stunned;

        if stunned {
            battle.units[i].intent = Some(EnemyIntent::Idle);
            continue;
        }

        let intent = match ai {
            AiBehavior::Chase | AiBehavior::Pack => {
                if dist <= 1 {
                    if has_radical {
                        EnemyIntent::RadicalAbility { name: "Ability" }
                    } else {
                        EnemyIntent::Attack
                    }
                } else {
                    EnemyIntent::Approach
                }
            }
            AiBehavior::Retreat | AiBehavior::Kiter => {
                if dist <= 1 {
                    EnemyIntent::Retreat
                } else if dist <= 3 {
                    EnemyIntent::Idle
                } else {
                    EnemyIntent::Approach
                }
            }
            AiBehavior::Ambush => {
                if dist <= 1 {
                    EnemyIntent::Attack
                } else if dist <= 3 {
                    EnemyIntent::Approach
                } else {
                    EnemyIntent::Idle
                }
            }
            AiBehavior::Sentinel => {
                if dist <= 1 {
                    EnemyIntent::Attack
                } else {
                    EnemyIntent::Idle
                }
            }
        };

        battle.units[i].intent = Some(intent);
    }
    battle.intents_calculated = true;
}

/// Choose an action for the enemy at `unit_idx` based on its AiBehavior.
pub fn choose_action(battle: &TacticalBattle, unit_idx: usize) -> AiAction {
    let unit = &battle.units[unit_idx];
    let player = &battle.units[0];
    let dist = manhattan(unit.x, unit.y, player.x, player.y);

    if unit.stunned {
        return AiAction::Wait;
    }

    // Try radical action (30% chance per available action).
    let seed = (battle.turn_number as u64)
        .wrapping_mul(31)
        .wrapping_add(unit_idx as u64)
        .wrapping_mul(17);
    for (i, _action) in unit.radical_actions.iter().enumerate() {
        let action_seed = seed.wrapping_add(i as u64);
        if action_seed % 100 < 30 {
            return AiAction::UseRadicalAction { action_idx: i };
        }
    }

    match unit.ai {
        AiBehavior::Chase | AiBehavior::Pack => {
            if dist <= 1 {
                AiAction::MeleeAttack { target_unit: 0 }
            } else {
                AiAction::MoveToward {
                    x: player.x,
                    y: player.y,
                }
            }
        }
        AiBehavior::Retreat | AiBehavior::Kiter => {
            if dist <= 1 {
                AiAction::MoveAway {
                    x: player.x,
                    y: player.y,
                }
            } else if dist <= 3 {
                AiAction::Wait
            } else {
                AiAction::MoveToward {
                    x: player.x,
                    y: player.y,
                }
            }
        }
        AiBehavior::Ambush => {
            if dist <= 3 {
                if dist <= 1 {
                    AiAction::MeleeAttack { target_unit: 0 }
                } else {
                    AiAction::MoveToward {
                        x: player.x,
                        y: player.y,
                    }
                }
            } else {
                AiAction::Wait
            }
        }
        AiBehavior::Sentinel => {
            if dist <= 1 {
                AiAction::MeleeAttack { target_unit: 0 }
            } else {
                AiAction::Wait
            }
        }
    }
}

pub fn step_toward(
    battle: &TacticalBattle,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
) -> Option<(i32, i32)> {
    let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    let mut best: Option<(i32, i32)> = None;
    let mut best_dist = i32::MAX;

    for (dx, dy) in &deltas {
        let nx = from_x + dx;
        let ny = from_y + dy;
        if !battle.arena.in_bounds(nx, ny) {
            continue;
        }
        if let Some(tile) = battle.arena.tile(nx, ny) {
            if !tile.is_walkable() {
                continue;
            }
        }
        if battle.unit_at(nx, ny).is_some() {
            continue;
        }
        let d = manhattan(nx, ny, to_x, to_y);
        if d < best_dist {
            best_dist = d;
            best = Some((nx, ny));
        }
    }
    best
}

pub fn step_away(
    battle: &TacticalBattle,
    from_x: i32,
    from_y: i32,
    away_x: i32,
    away_y: i32,
) -> Option<(i32, i32)> {
    let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    let mut best: Option<(i32, i32)> = None;
    let mut best_dist = i32::MIN;

    for (dx, dy) in &deltas {
        let nx = from_x + dx;
        let ny = from_y + dy;
        if !battle.arena.in_bounds(nx, ny) {
            continue;
        }
        if let Some(tile) = battle.arena.tile(nx, ny) {
            if !tile.is_walkable() {
                continue;
            }
        }
        if battle.unit_at(nx, ny).is_some() {
            continue;
        }
        let d = manhattan(nx, ny, away_x, away_y);
        if d > best_dist {
            best_dist = d;
            best = Some((nx, ny));
        }
    }
    best
}
