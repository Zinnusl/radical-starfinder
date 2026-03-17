use crate::combat::{Direction, TacticalBattle};

pub fn move_unit(battle: &mut TacticalBattle, unit_idx: usize, dest_x: i32, dest_y: i32) {
    let unit = &mut battle.units[unit_idx];
    let old_x = unit.x;
    let old_y = unit.y;
    let dx = dest_x - old_x;
    let dy = dest_y - old_y;
    if let Some(dir) = Direction::from_delta(dx, dy) {
        unit.facing = dir;
    }
    unit.x = dest_x;
    unit.y = dest_y;
    unit.stored_movement = 0;

    if unit.is_player() {
        battle.player_moved = true;
    }
}

pub fn defend(battle: &mut TacticalBattle, unit_idx: usize) {
    battle.units[unit_idx].defending = true;
    if battle.units[unit_idx].is_player() {
        battle.player_acted = true;
        let msg = "You brace for impact.".to_string();
        battle.log_message(msg);
    }
}

pub fn wait(battle: &mut TacticalBattle, unit_idx: usize) {
    let unit = &mut battle.units[unit_idx];
    // Max +2 stored movement.
    unit.stored_movement = (unit.stored_movement + 1).min(2);
    if unit.is_player() {
        battle.player_acted = true;
        battle.log_message("You wait, gathering momentum.");
    }
}

/// Deal damage to a unit, accounting for defending status and armor.
/// Returns actual damage dealt.
pub fn deal_damage(battle: &mut TacticalBattle, target_idx: usize, raw_damage: i32) -> i32 {
    let unit = &mut battle.units[target_idx];
    let mut damage = raw_damage;

    if unit.defending {
        damage = damage / 2;
    }
    damage -= unit.radical_armor;
    unit.radical_armor = 0;

    damage = damage.max(1);
    unit.hp -= damage;
    if unit.hp <= 0 {
        unit.hp = 0;
        unit.alive = false;
    }
    damage
}

/// Backstab (+50%) if attacker is behind target's facing.
/// Flank (+25%) if attacker is to the side of target's facing.
/// 0% if attacking from the front.
pub fn flank_bonus(battle: &TacticalBattle, attacker_idx: usize, target_idx: usize) -> f64 {
    let atk = &battle.units[attacker_idx];
    let tgt = &battle.units[target_idx];
    let dx = atk.x - tgt.x;
    let dy = atk.y - tgt.y;
    let facing = tgt.facing;
    let behind = facing.opposite();

    if let Some(attack_dir) = Direction::from_delta(dx, dy) {
        if attack_dir == behind {
            0.50
        } else if attack_dir == facing {
            0.0
        } else {
            0.25
        }
    } else {
        0.0
    }
}
