use crate::combat::{BattleTile, Direction, TacticalBattle, WuxingElement};

pub fn move_unit(
    battle: &mut TacticalBattle,
    unit_idx: usize,
    dest_x: i32,
    dest_y: i32,
) -> Vec<String> {
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

    let mut messages = Vec::new();
    if battle.arena.tile(dest_x, dest_y) == Some(BattleTile::Thorns) {
        let actual = deal_damage(battle, unit_idx, 1);
        let name = if battle.units[unit_idx].is_player() {
            "You".to_string()
        } else {
            battle.units[unit_idx].hanzi.to_string()
        };
        messages.push(format!("{} is pierced by thorns! (-{} HP)", name, actual));
    }
    // SpiritWell: one-time spirit restore, convert to Open
    if battle.arena.tile(dest_x, dest_y) == Some(BattleTile::SpiritWell)
        && battle.units[unit_idx].is_player()
    {
        battle.arena.set_tile(dest_x, dest_y, BattleTile::Open);
        battle.pending_spirit_delta += 15;
        messages.push("🌊 The Spirit Well restores your energy! (+15 spirit)".to_string());
    }
    // TrapTile: trigger hidden or revealed spike trap
    let dest_tile = battle.arena.tile(dest_x, dest_y);
    if dest_tile == Some(BattleTile::TrapTile) || dest_tile == Some(BattleTile::TrapTileRevealed) {
        let mut trap_msgs =
            crate::combat::terrain::trigger_trap(battle, unit_idx, dest_x, dest_y);
        messages.append(&mut trap_msgs);
    }
    // CrumblingFloor / CrackedFloor: step-on interaction
    {
        let mut crumble_msgs =
            crate::combat::terrain::step_on_crumbling(battle, dest_x, dest_y);
        messages.append(&mut crumble_msgs);
    }
    messages
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
    unit.stored_movement = (unit.stored_movement + 1).min(2);
    let is_player = unit.is_player();
    let ux = unit.x;
    let uy = unit.y;
    if is_player {
        battle.player_acted = true;
        let tile = battle.arena.tile(ux, uy);
        if tile == Some(BattleTile::MeditationStone) {
            battle.pending_spirit_delta += 10;
            battle.log_message("You meditate on the stone... (+10 spirit)");
        } else {
            battle.log_message("You wait, gathering momentum.");
        }
    }
}

/// Deal damage to a unit, accounting for defending status and armor.
/// Returns actual damage dealt.
pub fn deal_damage(battle: &mut TacticalBattle, target_idx: usize, raw_damage: i32) -> i32 {
    if target_idx == 0 && battle.god_mode {
        return 0;
    }
    let unit = &mut battle.units[target_idx];
    let mut damage = raw_damage;

    if unit.defending {
        damage = damage / 2;
    }
    damage -= unit.radical_armor;
    unit.radical_armor = 0;

    damage = damage.max(1);

    if unit.marked_extra_damage > 0 {
        damage += unit.marked_extra_damage;
        unit.marked_extra_damage = 0;
    }

    let reading_bonus = word_group_order_bonus(battle, target_idx, damage);
    damage += reading_bonus;

    let unit = &mut battle.units[target_idx];
    unit.hp -= damage;
    if unit.hp <= 0 {
        unit.hp = 0;
        unit.alive = false;
        if reading_bonus > 0 {
            battle.log_message(format!("Reading order bonus! +{} damage", reading_bonus));
        }
        // SoulTrap: if enemy dies on a SoulTrap tile, log it for spirit gain
        if !battle.units[target_idx].is_player() {
            let dx = battle.units[target_idx].x;
            let dy = battle.units[target_idx].y;
            if battle.arena.tile(dx, dy) == Some(BattleTile::SoulTrap) {
                battle.pending_spirit_delta += 10;
                battle.log_message("💀 Soul Trap captures the fallen spirit! (+10 spirit)");
            }
        }
    }
    damage
}

/// +50% bonus damage when killing word-group members in reading order (left→right).
fn word_group_order_bonus(battle: &TacticalBattle, target_idx: usize, base_damage: i32) -> i32 {
    let target = &battle.units[target_idx];
    let group = match target.word_group {
        Some(g) => g,
        None => return 0,
    };
    let order = target.word_group_order;
    if order == 0 {
        return (base_damage as f64 * 0.5) as i32;
    }
    let prev_dead = battle
        .units
        .iter()
        .any(|u| u.word_group == Some(group) && u.word_group_order == order - 1 && !u.alive);
    if prev_dead {
        (base_damage as f64 * 0.5) as i32
    } else {
        0
    }
}

pub fn deal_damage_from(
    battle: &mut TacticalBattle,
    attacker_idx: usize,
    target_idx: usize,
    raw_damage: i32,
) -> (i32, Option<&'static str>) {
    let atk_elem = battle.units[attacker_idx].wuxing_element;
    let def_elem = battle.units[target_idx].wuxing_element;
    let multiplier = WuxingElement::multiplier(atk_elem, def_elem);
    let modified = (raw_damage as f64 * multiplier).ceil() as i32;
    let actual = deal_damage(battle, target_idx, modified);
    let label = if multiplier > 1.0 {
        Some("Super effective!")
    } else if multiplier < 1.0 {
        Some("Not very effective...")
    } else {
        None
    };

    let target = &mut battle.units[target_idx];
    let mut attacker_damage = 0;
    let mut retaliation_msg = None;

    if target.thorn_armor_turns > 0 {
        attacker_damage += 1;
        retaliation_msg = Some("Thorn armor retaliates for 1 damage!".to_string());
    }
    if target
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Thorns))
    {
        attacker_damage += 2;
        let msg = match retaliation_msg {
            Some(prev) => format!("{} Thorns aura retaliates for 2 damage!", prev),
            None => "Thorns aura retaliates for 2 damage!".to_string(),
        };
        retaliation_msg = Some(msg);
    }
    if target.radical_counter {
        attacker_damage += 2;
        target.radical_counter = false;
        let msg = if retaliation_msg.is_some() {
            format!(
                "{} Radical Counter strikes back for 2 damage!",
                retaliation_msg.unwrap()
            )
        } else {
            "Radical Counter strikes back for 2 damage!".to_string()
        };
        retaliation_msg = Some(msg);
    }

    if attacker_damage > 0 {
        let attacker = &mut battle.units[attacker_idx];
        attacker.hp -= attacker_damage;
        if attacker.hp <= 0 {
            attacker.hp = 0;
            attacker.alive = false;
        }
        if let Some(msg) = retaliation_msg {
            battle.log_message(msg);
        }
    }

    (actual, label)
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
