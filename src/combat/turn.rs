use crate::combat::{BattleUnit, TacticalBattle, UnitKind};
use crate::player::{PlayerClass, PlayerForm};
use crate::status::has_haste;

/// Player base speed by class category.
/// Fast (Assassin, Thief, Swordsman): 5
/// Normal (most classes): 4
/// Slow (Ironclad, Earthmover): 3
pub fn player_base_speed(class: PlayerClass) -> i32 {
    match class {
        PlayerClass::Assassin | PlayerClass::Thief | PlayerClass::Swordsman => 5,
        PlayerClass::Ironclad | PlayerClass::Earthmover => 3,
        _ => 4,
    }
}

pub fn player_speed(
    class: PlayerClass,
    form: PlayerForm,
    statuses: &[crate::status::StatusInstance],
) -> i32 {
    let mut speed = player_base_speed(class);
    if has_haste(statuses) {
        speed += 2;
    }
    match form {
        PlayerForm::Tiger => speed += 2,
        PlayerForm::Stone => speed -= 1,
        _ => {}
    }
    speed
}

pub fn player_base_movement() -> i32 {
    3
}

pub fn player_movement(form: PlayerForm, statuses: &[crate::status::StatusInstance]) -> i32 {
    let mut mv = player_base_movement();
    if has_haste(statuses) {
        mv += 1;
    }
    mv
}

pub fn enemy_base_speed(is_elite: bool, is_boss: bool) -> i32 {
    if is_boss {
        4
    } else if is_elite {
        4
    } else {
        3
    }
}

pub fn enemy_base_movement(is_elite: bool, is_boss: bool) -> i32 {
    if is_elite || is_boss {
        3
    } else {
        2
    }
}

/// Build the turn queue: indices into `battle.units` sorted by speed descending.
/// Ties: player first, then enemies by index.
pub fn build_turn_queue(units: &[BattleUnit]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..units.len()).filter(|&i| units[i].alive).collect();
    indices.sort_by(|&a, &b| {
        let sa = units[a].speed;
        let sb = units[b].speed;
        sb.cmp(&sa).then_with(|| {
            let pa = matches!(units[a].kind, UnitKind::Player);
            let pb = matches!(units[b].kind, UnitKind::Player);
            pb.cmp(&pa).then(a.cmp(&b))
        })
    });
    indices
}

/// Advance to the next turn. Returns true if the queue wrapped (new round).
pub fn advance_turn(battle: &mut TacticalBattle) -> bool {
    battle.turn_queue_pos += 1;

    // Skip dead or removed units.
    while battle.turn_queue_pos < battle.turn_queue.len() {
        let idx = battle.turn_queue[battle.turn_queue_pos];
        if battle.units[idx].alive {
            break;
        }
        battle.turn_queue_pos += 1;
    }

    if battle.turn_queue_pos >= battle.turn_queue.len() {
        battle.turn_queue = build_turn_queue(&battle.units);
        battle.turn_queue_pos = 0;
        battle.turn_number += 1;
        // Reset defending flag for all units at round start.
        for unit in &mut battle.units {
            unit.defending = false;
        }
        true
    } else {
        false
    }
}
