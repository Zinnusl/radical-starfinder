use crate::combat::{AudioEvent, BattleTile, Direction, TacticalBattle, WuxingElement};

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
    let move_dir = Direction::from_delta(dx, dy);
    if let Some(dir) = move_dir {
        unit.facing = dir;
        // Build momentum on straight-line movement
        if unit.last_move_dir == Some(dir) {
            unit.momentum = (unit.momentum + 1).min(3);
        } else {
            unit.momentum = 1;
        }
        unit.last_move_dir = Some(dir);
    } else {
        unit.momentum = 0;
        unit.last_move_dir = None;
    }
    unit.x = dest_x;
    unit.y = dest_y;
    unit.stored_movement = 0;

    if unit.is_player() {
        battle.player_moved = true;
    }

    let mut messages = Vec::new();
    if battle.arena.tile(dest_x, dest_y) == Some(BattleTile::ElectrifiedWire) {
        let actual = deal_damage(battle, unit_idx, 1);
        let name = if battle.units[unit_idx].is_player() {
            "You".to_string()
        } else {
            battle.units[unit_idx].hanzi.to_string()
        };
        messages.push(format!("{} is pierced by thorns! (-{} HP)", name, actual));
    }
    // Water tile: apply Wet status for 3 turns
    if battle.arena.tile(dest_x, dest_y) == Some(BattleTile::CoolantPool) {
        use crate::status::{has_wet, StatusInstance, StatusKind};
        if !has_wet(&battle.units[unit_idx].statuses) {
            battle.units[unit_idx]
                .statuses
                .push(StatusInstance::new(StatusKind::Wet, 3));
            let name = if battle.units[unit_idx].is_player() {
                "You get".to_string()
            } else {
                format!("{} gets", battle.units[unit_idx].hanzi)
            };
            messages.push(format!("💧 {} soaked!", name));
        }
        let mut combo_msgs = check_status_combos(battle, unit_idx);
        messages.append(&mut combo_msgs);
    }
    // EnergyNode: one-time HP restore, convert to Open
    if battle.arena.tile(dest_x, dest_y) == Some(BattleTile::EnergyNode)
        && battle.units[unit_idx].is_player()
    {
        battle.arena.set_tile(dest_x, dest_y, BattleTile::MetalFloor);
        battle.units[unit_idx].hp = (battle.units[unit_idx].hp + 3).min(battle.units[unit_idx].max_hp);
        messages.push("🌊 The Energy Node restores your vitality! (+3 HP)".to_string());
    }
    // TrapTile: trigger hidden or revealed spike trap
    let dest_tile = battle.arena.tile(dest_x, dest_y);
    if dest_tile == Some(BattleTile::MineTile) || dest_tile == Some(BattleTile::MineTileRevealed) {
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
    // Terrain audio cues (only for the player to avoid spam)
    if battle.units[unit_idx].is_player() {
        let dest = battle.arena.tile(dest_x, dest_y);
        if dest == Some(BattleTile::CoolantPool) {
            battle.audio_events.push(AudioEvent::WaterSplash);
        }
        if dest == Some(BattleTile::PlasmaPool) {
            battle.audio_events.push(AudioEvent::LavaRumble);
        }
    }
    // Charge attack: momentum 3 adjacent to an enemy triggers free attack
    if battle.units[unit_idx].momentum >= 3 {
        let ux = battle.units[unit_idx].x;
        let uy = battle.units[unit_idx].y;
        let is_player = battle.units[unit_idx].is_player();
        let is_enemy = battle.units[unit_idx].is_enemy();
        let mut charge_target = None;
        for (i, u) in battle.units.iter().enumerate() {
            if i == unit_idx || !u.alive {
                continue;
            }
            if (u.x - ux).abs() + (u.y - uy).abs() != 1 {
                continue;
            }
            if (is_player && u.is_enemy()) || (is_enemy && (u.is_player() || u.is_companion())) {
                charge_target = Some(i);
                break;
            }
        }
        if let Some(target) = charge_target {
            let charge_dmg = battle.units[unit_idx].damage + 2;
            let name = if is_player {
                "You".to_string()
            } else {
                battle.units[unit_idx].hanzi.to_string()
            };
            let (actual, _) = deal_damage_from(battle, unit_idx, target, charge_dmg);
            messages.push(format!("⚡ {} charge attack! {} damage!", name, actual));
            battle.units[unit_idx].momentum = 0;
        }
    }
    messages
}

pub fn defend(battle: &mut TacticalBattle, unit_idx: usize) {
    battle.units[unit_idx].defending = true;
    battle.units[unit_idx].momentum = 0;
    battle.units[unit_idx].last_move_dir = None;
    if battle.units[unit_idx].is_player() {
        battle.player_acted = true;
        let msg = "You brace for impact.".to_string();
        battle.log_message(msg);
    }
}

pub fn wait(battle: &mut TacticalBattle, unit_idx: usize) {
    let unit = &mut battle.units[unit_idx];
    unit.stored_movement = (unit.stored_movement + 1).min(2);
    unit.momentum = 0;
    unit.last_move_dir = None;
    let is_player = unit.is_player();
    let ux = unit.x;
    let uy = unit.y;
    if is_player {
        battle.player_acted = true;
        let tile = battle.arena.tile(ux, uy);
        if tile == Some(BattleTile::ChargingPad) {
            battle.focus = (battle.focus + 3).min(battle.max_focus);
            battle.log_message("You meditate on the stone... (+3 focus)");
        } else {
            battle.log_message("You wait, gathering momentum.");
        }
    }
}

/// Deal damage to a unit, accounting for defending status and armor.
/// Guard companion intercept: if Guard is adjacent to player, absorbs some damage.
/// Returns actual damage dealt.
pub fn deal_damage(battle: &mut TacticalBattle, target_idx: usize, raw_damage: i32) -> i32 {
    if target_idx == 0 && battle.god_mode {
        return 0;
    }

    // Guard passive: intercept attacks aimed at player if Guard is adjacent
    if target_idx == 0 {
        if let Some(crate::game::Companion::SecurityChief) = battle.companion_kind {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let guard_adj = battle.units.iter().position(|u| {
                u.is_companion()
                    && u.alive
                    && (u.x - px).abs() + (u.y - py).abs() <= 1
            });
            if let Some(gidx) = guard_adj {
                // Guard intercepts half the damage (min 1)
                let intercepted = (raw_damage / 2).max(1);
                let remaining = (raw_damage - intercepted).max(1);
                battle.units[gidx].hp -= intercepted;
                if battle.units[gidx].hp <= 0 {
                    battle.units[gidx].hp = 0;
                    battle.units[gidx].alive = false;
                }
                battle.log_message(format!(
                    "🛡 Guard intercepts {} damage!",
                    intercepted
                ));
                // Continue with reduced damage for the player
                let unit = &mut battle.units[target_idx];
                let mut damage = remaining;
                // Apply stance armor modifier
                damage -= battle.player_stance.armor_mod();
                if unit.defending {
                    damage = damage / 2;
                    battle.audio_events.push(AudioEvent::ShieldBlock);
                }
                damage -= unit.radical_armor;
                unit.radical_armor = 0;
                damage = damage.max(1);
                let unit = &mut battle.units[target_idx];
                unit.hp -= damage;
                if unit.hp <= 0 {
                    unit.hp = 0;
                    unit.alive = false;
                }
                return damage;
            }
        }
    }

    let unit = &mut battle.units[target_idx];
    let mut damage = raw_damage;

    // Apply stance armor modifier when player is the target
    if target_idx == 0 {
        damage -= battle.player_stance.armor_mod();
        // Apply ability combo armor bonus (Tempering)
        damage -= battle.combo_armor_bonus;
    }

    if unit.defending {
        damage = damage / 2;
        battle.audio_events.push(AudioEvent::ShieldBlock);
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
        if !battle.units[target_idx].is_player() {
            battle.audio_events.push(AudioEvent::EnemyDeath);
        }
        if reading_bonus > 0 {
            battle.log_message(format!("Reading order bonus! +{} damage", reading_bonus));
        }
        // SoulTrap: if enemy dies on a GravityTrap tile, heal player
        if !battle.units[target_idx].is_player() {
            let dx = battle.units[target_idx].x;
            let dy = battle.units[target_idx].y;
            if battle.arena.tile(dx, dy) == Some(BattleTile::GravityTrap) {
                battle.units[0].hp = (battle.units[0].hp + 2).min(battle.units[0].max_hp);
                battle.log_message("💀 Soul Trap captures the fallen essence! (+2 HP)");
            }
            // Revenge: adjacent allies get enraged when an enemy dies.
            crate::combat::synergy::on_enemy_death_revenge(battle, target_idx);
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

/// Check if a unit is cornered (adjacent to 2+ walls/obstacles).
pub fn is_cornered(battle: &TacticalBattle, unit_idx: usize) -> bool {
    let ux = battle.units[unit_idx].x;
    let uy = battle.units[unit_idx].y;
    let mut wall_count = 0;
    for &(ddx, ddy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
        match battle.arena.tile(ux + ddx, uy + ddy) {
            None => wall_count += 1,
            Some(t) if !t.is_walkable() => wall_count += 1,
            _ => {}
        }
    }
    wall_count >= 2
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
    let mut modified = (raw_damage as f64 * multiplier).ceil() as i32;

    // Momentum bonus: momentum / 2 (0 at 0-1, +1 at 2-3)
    let momentum = battle.units[attacker_idx].momentum;
    let momentum_bonus = momentum / 2;
    if momentum_bonus > 0 {
        modified += momentum_bonus;
        battle.log_message(format!("Momentum +{}!", momentum_bonus));
    }

    // High ground damage modifier
    let atk_tile = battle.arena.tile(battle.units[attacker_idx].x, battle.units[attacker_idx].y);
    let def_tile = battle.arena.tile(battle.units[target_idx].x, battle.units[target_idx].y);
    if atk_tile == Some(BattleTile::ElevatedPlatform) && def_tile != Some(BattleTile::ElevatedPlatform) {
        modified += 1;
    } else if atk_tile != Some(BattleTile::ElevatedPlatform) && def_tile == Some(BattleTile::ElevatedPlatform) {
        modified = (modified - 1).max(1);
    }

    // Cornered penalty: +1 damage to cornered targets
    if is_cornered(battle, target_idx) {
        modified += 1;
        battle.log_message("Cornered!");
    }

    // Equipment set first-strike bonus (turn 1 only)
    let first_strike = battle.units[attacker_idx].first_strike_bonus;
    if first_strike > 0 && battle.turn_number <= 1 {
        modified += first_strike;
        battle.log_message(format!("First strike +{}!", first_strike));
    }

    let actual = deal_damage(battle, target_idx, modified);

    // Reset attacker momentum after attack (spent on the attack)
    battle.units[attacker_idx].momentum = 0;
    battle.units[attacker_idx].last_move_dir = None;

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
        // ElectrifiedBarrier + Fortify synergy: barrier damage boosted by fortify stacks
        let mut fortify_boost = target.fortify_stacks;
        // Aggressive stance + Fortify synergy: fortify stacks doubled
        if target_idx == 0 && battle.player_stance == crate::combat::PlayerStance::Aggressive {
            fortify_boost *= 2;
        }
        attacker_damage += 1 + fortify_boost;
        if fortify_boost > 0 {
            retaliation_msg = Some(format!(
                "🌿⚔ Fortified thorn armor retaliates for {} damage!",
                1 + fortify_boost
            ));
        } else {
            retaliation_msg = Some("Thorn armor retaliates for 1 damage!".to_string());
        }
    }
    if target
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Thorns))
    {
        let mut thorn_dmg = 2;
        // Defensive stance + Electrified barrier synergy: +1 barrier damage
        if target_idx == 0 && battle.player_stance == crate::combat::PlayerStance::Defensive {
            thorn_dmg += 1;
        }
        attacker_damage += thorn_dmg;
        let msg = match retaliation_msg {
            Some(prev) => format!("{} Thorns aura retaliates for {} damage!", prev, thorn_dmg),
            None => format!("Electrified barrier retaliates for {} damage!", thorn_dmg),
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

// ── Status Effect Combos ─────────────────────────────────────────────────────

/// Check for status effect combinations on a unit and trigger emergent interactions.
/// Called after any status is applied.
pub fn check_status_combos(battle: &mut TacticalBattle, unit_idx: usize) -> Vec<String> {
    let mut messages = Vec::new();

    // Gather status flags before mutating
    let has_wet = battle.units[unit_idx]
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Wet));
    let has_burn = battle.units[unit_idx]
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Burn { .. }));
    let has_freeze = battle.units[unit_idx]
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Freeze));
    let has_poison = battle.units[unit_idx]
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Poison { .. }));
    let has_slow = battle.units[unit_idx]
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Slow));
    let has_haste = battle.units[unit_idx]
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Haste));
    let has_fortify = battle.units[unit_idx]
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Fortify { .. }));
    let has_weakened = battle.units[unit_idx]
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Weakened));
    let has_blessed = battle.units[unit_idx]
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Blessed));
    let has_cursed = battle.units[unit_idx]
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Cursed));
    let on_lubricant = battle
        .arena
        .tile(battle.units[unit_idx].x, battle.units[unit_idx].y)
        == Some(BattleTile::Lubricant);

    let name = if battle.units[unit_idx].is_player() {
        "You".to_string()
    } else {
        battle.units[unit_idx].hanzi.to_string()
    };

    // Wet + Burn → Steam (clears both, Steam tile, 1 damage)
    if has_wet && has_burn {
        use crate::status::StatusKind;
        battle.units[unit_idx]
            .statuses
            .retain(|s| !matches!(s.kind, StatusKind::Wet | StatusKind::Burn { .. }));
        let ux = battle.units[unit_idx].x;
        let uy = battle.units[unit_idx].y;
        battle.arena.set_steam(ux, uy, 2);
        let actual = deal_damage(battle, unit_idx, 1);
        messages.push(format!(
            "💨 Wet + Burn = Steam! {} takes {} damage!",
            name, actual
        ));
        battle.audio_events.push(AudioEvent::WaterSplash);
    }

    // Wet + Freeze → Instant Frozen (skip 2 turns, +3 armor)
    if has_wet && has_freeze {
        use crate::status::{StatusInstance, StatusKind};
        battle.units[unit_idx]
            .statuses
            .retain(|s| !matches!(s.kind, StatusKind::Wet | StatusKind::Freeze));
        // Deep freeze: 2-turn freeze
        battle.units[unit_idx]
            .statuses
            .push(StatusInstance::new(StatusKind::Freeze, 2));
        battle.units[unit_idx].radical_armor += 3;
        messages.push(format!(
            "❄💧 Wet + Freeze = Deep Freeze! {} is frozen solid for 2 turns (+3 armor)!",
            name
        ));
    }

    // Lubricant tile + Burn → Explosion (3 damage to unit and adjacent, BlastMark tiles)
    if on_lubricant && has_burn {
        use crate::status::StatusKind;
        battle.units[unit_idx]
            .statuses
            .retain(|s| !matches!(s.kind, StatusKind::Burn { .. }));
        let ux = battle.units[unit_idx].x;
        let uy = battle.units[unit_idx].y;
        battle.arena.set_tile(ux, uy, BattleTile::BlastMark);
        let actual = deal_damage(battle, unit_idx, 3);
        messages.push(format!(
            "💥 Lubricant + Burn = Explosion! {} takes {} damage!",
            name, actual
        ));
        let deltas: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for &(dx, dy) in &deltas {
            let ax = ux + dx;
            let ay = uy + dy;
            if let Some(adj_tile) = battle.arena.tile(ax, ay) {
                if adj_tile == BattleTile::Lubricant {
                    battle.arena.set_tile(ax, ay, BattleTile::BlastMark);
                }
            }
            if let Some(aidx) = battle.unit_at(ax, ay) {
                let adj_actual = deal_damage(battle, aidx, 3);
                let adj_name = if battle.units[aidx].is_player() {
                    "You".to_string()
                } else {
                    battle.units[aidx].hanzi.to_string()
                };
                messages.push(format!(
                    "{} caught in oil explosion! (-{} HP)",
                    adj_name, adj_actual
                ));
            }
        }
    }

    // Poison + Burn → Toxic Fumes (2 damage AoE in 2-tile radius, Confused 1 turn)
    if has_poison && has_burn {
        use crate::status::{StatusInstance, StatusKind};
        battle.units[unit_idx]
            .statuses
            .retain(|s| !matches!(s.kind, StatusKind::Poison { .. } | StatusKind::Burn { .. }));
        let ux = battle.units[unit_idx].x;
        let uy = battle.units[unit_idx].y;
        messages.push(format!(
            "☠🔥 Poison + Burn = Toxic Fumes! Noxious gas erupts around {}!",
            name
        ));
        for i in 0..battle.units.len() {
            if !battle.units[i].alive || i == unit_idx {
                continue;
            }
            let dist = (battle.units[i].x - ux).abs() + (battle.units[i].y - uy).abs();
            if dist <= 2 {
                let actual = deal_damage(battle, i, 2);
                battle.units[i]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Confused, 1));
                let aname = if battle.units[i].is_player() {
                    "You".to_string()
                } else {
                    battle.units[i].hanzi.to_string()
                };
                messages.push(format!(
                    "{} chokes on toxic fumes! (-{} HP, Confused!)",
                    aname, actual
                ));
            }
        }
    }

    // Slow + Haste → Cancel each other out
    if has_slow && has_haste {
        use crate::status::StatusKind;
        battle.units[unit_idx]
            .statuses
            .retain(|s| !matches!(s.kind, StatusKind::Slow | StatusKind::Haste));
        messages.push(format!(
            "⚡🐌 Slow and Haste cancel out on {}!",
            name
        ));
    }

    // Fortify + Weakened → Cancel each other out
    if has_fortify && has_weakened {
        use crate::status::StatusKind;
        battle.units[unit_idx]
            .statuses
            .retain(|s| !matches!(s.kind, StatusKind::Fortify { .. } | StatusKind::Weakened));
        messages.push(format!(
            "💪⬇ Fortify and Weakened cancel out on {}!",
            name
        ));
    }

    // Blessed + Cursed → Cancel each other out
    if has_blessed && has_cursed {
        use crate::status::StatusKind;
        battle.units[unit_idx]
            .statuses
            .retain(|s| !matches!(s.kind, StatusKind::Blessed | StatusKind::Cursed));
        messages.push(format!(
            "✨💀 Blessed and Cursed cancel out on {}!",
            name
        ));
    }

    messages
}

// ── Equipment Synergies ──────────────────────────────────────────────────────

/// Check if player has a specific equipment effect.
#[allow(dead_code)]
pub fn player_has_equip(battle: &TacticalBattle, check: fn(&crate::player::EquipEffect) -> bool) -> bool {
    battle.player_equip_effects.iter().any(|e| check(e))
}

/// LifeSteal + Poison synergy: drain extra 1 HP from poisoned enemies.
#[allow(dead_code)]
pub fn lifesteal_poison_bonus(battle: &mut TacticalBattle, target_idx: usize, base_heal: i32) -> i32 {
    let has_lifesteal = battle.player_equip_effects.iter().any(|e| {
        matches!(e, crate::player::EquipEffect::LifeSteal(_))
    });
    let target_poisoned = battle.units[target_idx]
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Poison { .. }));
    if has_lifesteal && target_poisoned {
        battle.log_message("🧛 LifeSteal drains extra from poisoned foe! (+1 HP)");
        base_heal + 1
    } else {
        base_heal
    }
}

/// CriticalStrike + Backstab synergy: 100% crit chance from behind.
pub fn critical_backstab_check(battle: &TacticalBattle, target_idx: usize) -> bool {
    let has_crit = battle.player_equip_effects.iter().any(|e| {
        matches!(e, crate::player::EquipEffect::CriticalStrike(_))
    });
    if !has_crit {
        return false;
    }
    let flank = flank_bonus(battle, 0, target_idx);
    // Backstab (from behind) guarantees crit
    flank >= 0.50
}

/// AbilityPowerBoost terrain synergy: terrain abilities affect 1 extra tile.
#[allow(dead_code)]
pub fn spell_power_extra_tiles(battle: &TacticalBattle) -> bool {
    battle.player_equip_effects.iter().any(|e| {
        matches!(e, crate::player::EquipEffect::SpellPowerBoost(_))
    })
}


#[cfg(test)]
mod tests;
