//! Enemy synergies: tactical interactions between groups of enemies.

use crate::combat::grid::manhattan;
use crate::combat::{BattleTile, TacticalBattle, WuxingElement};
use crate::enemy::AiBehavior;
use crate::status::StatusKind;

// ── Round-Start Synergies ────────────────────────────────────────────────────

/// Apply all round-start enemy synergies. Called once when the turn queue wraps.
pub fn apply_round_start_synergies(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();

    reset_synergy_state(battle);
    messages.extend(apply_sentinel_formation(battle));
    messages.extend(apply_elemental_resonance(battle));
    messages.extend(apply_elemental_clash(battle));
    messages.extend(apply_leader_aura(battle));
    tick_sacrifice_bonuses(battle);

    messages
}

/// Reset per-round synergy bonuses.
fn reset_synergy_state(battle: &mut TacticalBattle) {
    battle.attacks_on_player_this_round = 0;
    for unit in battle.units.iter_mut() {
        unit.synergy_damage_bonus = 0;
        unit.elemental_resonance = false;
    }
}

// ── Shield Formation ───────────────────────────────────────────────────────

/// Sentinels share 1 armor with adjacent allies. If 2+ Sentinels adjacent,
/// each gets +1 additional armor.
fn apply_sentinel_formation(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();

    // Collect sentinel positions and indices.
    let sentinels: Vec<usize> = (1..battle.units.len())
        .filter(|&i| {
            battle.units[i].alive
                && battle.units[i].is_enemy()
                && battle.units[i].ai == AiBehavior::Sentinel
        })
        .collect();

    if sentinels.is_empty() {
        return messages;
    }

    // Check for formation bonus: sentinel adjacent to another sentinel.
    let mut sentinel_formation_bonus: Vec<usize> = Vec::new();
    for &si in &sentinels {
        let sx = battle.units[si].x;
        let sy = battle.units[si].y;
        let adj_sentinel_count = sentinels
            .iter()
            .filter(|&&oj| oj != si && manhattan(sx, sy, battle.units[oj].x, battle.units[oj].y) == 1)
            .count();
        if adj_sentinel_count >= 1 {
            sentinel_formation_bonus.push(si);
        }
    }

    // Apply formation bonus to sentinels in formation.
    if sentinel_formation_bonus.len() >= 2 {
        for &si in &sentinel_formation_bonus {
            battle.units[si].radical_armor += 1;
        }
        messages.push("🛡 Shield formation! Adjacent sentinels gain +1 armor.".to_string());
    }

    // Each sentinel shares 1 armor with adjacent allies.
    let mut armored: Vec<usize> = Vec::new();
    for &si in &sentinels {
        let sx = battle.units[si].x;
        let sy = battle.units[si].y;
        let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for (dx, dy) in &deltas {
            if let Some(idx) = battle.unit_at(sx + dx, sy + dy) {
                if idx != si && battle.units[idx].is_enemy() && !armored.contains(&idx) {
                    battle.units[idx].radical_armor += 1;
                    armored.push(idx);
                }
            }
        }
    }
    if !armored.is_empty() {
        messages.push(format!(
            "🛡 Sentinels share armor with {} allies.",
            armored.len()
        ));
    }

    messages
}

// ── Elemental Resonance ──────────────────────────────────────────────────────

/// When 2+ enemies of the same WuxingElement are within 2 tiles,
/// all get +1 to their element's effect.
fn apply_elemental_resonance(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();

    // Gather alive enemies with elements.
    let elemental: Vec<(usize, WuxingElement)> = (1..battle.units.len())
        .filter_map(|i| {
            if battle.units[i].alive && battle.units[i].is_enemy() {
                battle.units[i].wuxing_element.map(|e| (i, e))
            } else {
                None
            }
        })
        .collect();

    if elemental.len() < 2 {
        return messages;
    }

    // For each element, find groups within 2 tiles.
    let elements = [
        WuxingElement::Fire,
        WuxingElement::Water,
        WuxingElement::Earth,
        WuxingElement::Metal,
        WuxingElement::Wood,
    ];

    for &elem in &elements {
        let of_element: Vec<usize> = elemental
            .iter()
            .filter(|(_, e)| *e == elem)
            .map(|(i, _)| *i)
            .collect();

        if of_element.len() < 2 {
            continue;
        }

        // Find which units are in resonance (within 2 tiles of at least one same-element ally).
        let mut resonating: Vec<usize> = Vec::new();
        for &ui in &of_element {
            let ux = battle.units[ui].x;
            let uy = battle.units[ui].y;
            let has_nearby = of_element.iter().any(|&oj| {
                oj != ui && manhattan(ux, uy, battle.units[oj].x, battle.units[oj].y) <= 2
            });
            if has_nearby {
                resonating.push(ui);
            }
        }

        if resonating.len() < 2 {
            continue;
        }

        // Apply element-specific bonus.
        match elem {
            WuxingElement::Fire => {
                // Burn effects deal +1 damage: boost existing burn statuses.
                for &ri in &resonating {
                    battle.units[ri].elemental_resonance = true;
                    for s in battle.units[ri].statuses.iter_mut() {
                        if let StatusKind::Burn { ref mut damage } = s.kind {
                            *damage += 1;
                        }
                    }
                }
                messages.push("🔥 Plasma resonance! Burn effects intensify.".to_string());
            }
            WuxingElement::Water => {
                // Slow effects last +1 turn.
                for &ri in &resonating {
                    battle.units[ri].elemental_resonance = true;
                }
                // Extend Slow on the player if any exists.
                for s in battle.units[0].statuses.iter_mut() {
                    if matches!(s.kind, StatusKind::Slow) {
                        s.turns_left += 1;
                    }
                }
                messages.push("💧 Coolant resonance! Slow effects linger.".to_string());
            }
            WuxingElement::Earth => {
                // +1 armor to all resonating enemies.
                for &ri in &resonating {
                    battle.units[ri].elemental_resonance = true;
                    battle.units[ri].radical_armor += 1;
                }
                messages.push("🪨 Hull resonance! Enemies gain +1 armor.".to_string());
            }
            WuxingElement::Metal => {
                // +1 damage to all resonating enemies.
                for &ri in &resonating {
                    battle.units[ri].elemental_resonance = true;
                    battle.units[ri].synergy_damage_bonus += 1;
                }
                messages.push("⚔ Metal resonance! Enemies gain +1 damage.".to_string());
            }
            WuxingElement::Wood => {
                // Heal 1 HP per round to all resonating enemies.
                for &ri in &resonating {
                    battle.units[ri].elemental_resonance = true;
                    let unit = &mut battle.units[ri];
                    unit.hp = (unit.hp + 1).min(unit.max_hp);
                }
                messages.push("🌿 Bio resonance! Enemies regenerate 1 HP.".to_string());
            }
        }
    }

    messages
}

// ── Elemental Clash ──────────────────────────────────────────────────────────

/// Opposing elements adjacent to each other create terrain effects.
fn apply_elemental_clash(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();

    let elemental: Vec<(usize, WuxingElement, i32, i32)> = (1..battle.units.len())
        .filter_map(|i| {
            if battle.units[i].alive && battle.units[i].is_enemy() {
                battle.units[i]
                    .wuxing_element
                    .map(|e| (i, e, battle.units[i].x, battle.units[i].y))
            } else {
                None
            }
        })
        .collect();

    // Track processed pairs to avoid duplicates.
    let mut processed = Vec::new();

    for &(ai, ae, ax, ay) in &elemental {
        for &(bi, be, bx, by) in &elemental {
            if ai >= bi {
                continue;
            }
            if manhattan(ax, ay, bx, by) != 1 {
                continue;
            }
            let pair = (ai.min(bi), ai.max(bi));
            if processed.contains(&pair) {
                continue;
            }
            processed.push(pair);

            // Fire + Water → Steam tile between them
            if (ae == WuxingElement::Fire && be == WuxingElement::Water)
                || (ae == WuxingElement::Water && be == WuxingElement::Fire)
            {
                let fire_idx = if ae == WuxingElement::Fire { ai } else { bi };
                let fx = battle.units[fire_idx].x;
                let fy = battle.units[fire_idx].y;
                battle.arena.set_steam(fx, fy, 2);
                messages.push("💨 Plasma and Coolant clash — steam erupts!".to_string());
            }

            // Fire + Wood → Wood takes 1 burn damage
            if (ae == WuxingElement::Fire && be == WuxingElement::Wood)
                || (ae == WuxingElement::Wood && be == WuxingElement::Fire)
            {
                let wood_idx = if ae == WuxingElement::Wood { ai } else { bi };
                battle.units[wood_idx].hp -= 1;
                if battle.units[wood_idx].hp <= 0 {
                    battle.units[wood_idx].hp = 0;
                    battle.units[wood_idx].alive = false;
                }
                let name = battle.units[wood_idx].hanzi;
                messages.push(format!(
                    "🔥🌿 Plasma scorches {} for 1 burn damage!",
                    name
                ));
            }

            // Water + Earth → Mud (Slow tile) created
            if (ae == WuxingElement::Water && be == WuxingElement::Earth)
                || (ae == WuxingElement::Earth && be == WuxingElement::Water)
            {
                let water_idx = if ae == WuxingElement::Water { ai } else { bi };
                let wx = battle.units[water_idx].x;
                let wy = battle.units[water_idx].y;
                if battle.arena.tile(wx, wy) == Some(BattleTile::MetalFloor) {
                    battle.arena.set_tile(wx, wy, BattleTile::DamagedPlating);
                    messages
                        .push("💧🪨 Coolant and Metal mix — sludge slows the ground!".to_string());
                }
            }
        }
    }

    messages
}

// ── Leader Aura ──────────────────────────────────────────────────────────────

/// Elite enemies: non-elite allies within 3 tiles get +1 speed.
/// Boss battles: all enemies get +1 damage.
fn apply_leader_aura(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();

    // Boss aura: in boss fights, all enemies get +1 damage.
    if battle.is_boss_battle {
        let has_boss = battle
            .units
            .iter()
            .any(|u| u.alive && u.boss_kind.is_some());
        if has_boss {
            for i in 1..battle.units.len() {
                if battle.units[i].alive && battle.units[i].is_enemy() && battle.units[i].boss_kind.is_none() {
                    battle.units[i].synergy_damage_bonus += 1;
                }
            }
            messages.push("👑 Boss presence! All enemies gain +1 damage.".to_string());
        }
    }

    // Elite leader aura: elites (word_group.is_some()) boost nearby non-elites.
    let elites: Vec<(i32, i32)> = (1..battle.units.len())
        .filter(|&i| {
            battle.units[i].alive
                && battle.units[i].is_enemy()
                && battle.units[i].word_group.is_some()
        })
        .map(|i| (battle.units[i].x, battle.units[i].y))
        .collect();

    if !elites.is_empty() {
        let mut boosted = false;
        for i in 1..battle.units.len() {
            if !battle.units[i].alive || !battle.units[i].is_enemy() {
                continue;
            }
            if battle.units[i].word_group.is_some() {
                continue;
            }
            let ux = battle.units[i].x;
            let uy = battle.units[i].y;
            let near_elite = elites
                .iter()
                .any(|&(ex, ey)| manhattan(ux, uy, ex, ey) <= 3);
            if near_elite {
                battle.units[i].speed += 1;
                boosted = true;
            }
        }
        if boosted {
            messages.push("⚡ Commander's aura! Nearby allies gain +1 speed.".to_string());
        }
    }

    messages
}

// ── Sacrifice Bonus Tracking ─────────────────────────────────────────────────

fn tick_sacrifice_bonuses(battle: &mut TacticalBattle) {
    for unit in battle.units.iter_mut() {
        if unit.sacrifice_bonus_turns > 0 {
            unit.sacrifice_bonus_turns -= 1;
            if unit.sacrifice_bonus_turns <= 0 {
                unit.sacrifice_bonus_damage = 0;
            }
        }
    }
}

// ── Attack-Time Synergies ────────────────────────────────────────────────────

/// Pack tactics: count adjacent allies for Pack enemies, return bonus damage.
pub fn pack_tactics_bonus(battle: &TacticalBattle, unit_idx: usize) -> i32 {
    let unit = &battle.units[unit_idx];
    if unit.ai != AiBehavior::Pack {
        return 0;
    }
    let ux = unit.x;
    let uy = unit.y;
    let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    let mut adjacent_allies = 0;
    for (dx, dy) in &deltas {
        if let Some(idx) = battle.unit_at(ux + dx, uy + dy) {
            if idx != unit_idx && battle.units[idx].is_enemy() {
                adjacent_allies += 1;
            }
        }
    }
    adjacent_allies
}

/// Coordinated attack: second+ enemy attacking the player this round gets +1 damage.
pub fn coordinated_attack_bonus(battle: &TacticalBattle) -> i32 {
    if battle.attacks_on_player_this_round >= 1 {
        1
    } else {
        0
    }
}

/// Get total synergy damage bonus for an enemy attack (pack + coordinated + synergy_damage_bonus + sacrifice).
pub fn total_attack_synergy_bonus(battle: &TacticalBattle, unit_idx: usize) -> (i32, Vec<String>) {
    let mut bonus = 0;
    let mut msgs = Vec::new();

    let pack = pack_tactics_bonus(battle, unit_idx);
    if pack > 0 {
        bonus += pack;
        msgs.push(format!("🐺 Pack tactics! +{} damage", pack));
    }

    let coord = coordinated_attack_bonus(battle);
    if coord > 0 {
        bonus += coord;
        msgs.push("⚔ Coordinated attack! +1 damage".to_string());
    }

    let synergy = battle.units[unit_idx].synergy_damage_bonus;
    if synergy > 0 {
        bonus += synergy;
    }

    let sacrifice = battle.units[unit_idx].sacrifice_bonus_damage;
    if sacrifice > 0 {
        bonus += sacrifice;
    }

    (bonus, msgs)
}

// ── Sacrifice ────────────────────────────────────────────────────────────────

/// When an enemy with <25% HP is adjacent to an ally, 20% chance it sacrifices
/// itself: dies, heals ally for remaining HP, gives ally +2 damage for 2 turns.
/// Returns true if sacrifice happened.
pub fn try_sacrifice(battle: &mut TacticalBattle, unit_idx: usize) -> bool {
    let unit = &battle.units[unit_idx];
    if !unit.is_enemy() || !unit.alive {
        return false;
    }

    let hp_ratio = unit.hp as f32 / unit.max_hp.max(1) as f32;
    if hp_ratio >= 0.25 {
        return false;
    }

    let ux = unit.x;
    let uy = unit.y;
    let remaining_hp = unit.hp;

    // Find adjacent alive enemy ally.
    let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    let mut best_ally: Option<usize> = None;
    let mut best_hp_ratio = f32::MAX;

    for (dx, dy) in &deltas {
        if let Some(idx) = battle.unit_at(ux + dx, uy + dy) {
            if idx != unit_idx && battle.units[idx].is_enemy() {
                let ally_ratio =
                    battle.units[idx].hp as f32 / battle.units[idx].max_hp.max(1) as f32;
                if ally_ratio < best_hp_ratio {
                    best_hp_ratio = ally_ratio;
                    best_ally = Some(idx);
                }
            }
        }
    }

    let ally_idx = match best_ally {
        Some(a) => a,
        None => return false,
    };

    // 20% chance based on deterministic seed.
    let seed = (battle.turn_number as u64)
        .wrapping_mul(41)
        .wrapping_add(unit_idx as u64 * 23);
    if seed % 5 != 0 {
        return false;
    }

    // Perform sacrifice.
    let sacrificer_name = battle.units[unit_idx].hanzi.to_string();
    let ally_name = battle.units[ally_idx].hanzi.to_string();

    battle.units[unit_idx].hp = 0;
    battle.units[unit_idx].alive = false;

    let ally = &mut battle.units[ally_idx];
    ally.hp = (ally.hp + remaining_hp).min(ally.max_hp);
    ally.sacrifice_bonus_damage = 2;
    ally.sacrifice_bonus_turns = 2;

    battle.log_message(format!(
        "💀🔥 {} sacrifices itself for {}! (+{} HP, +2 damage for 2 turns)",
        sacrificer_name, ally_name, remaining_hp
    ));

    true
}

// ── Revenge (Death Synergy) ──────────────────────────────────────────────────

/// When an enemy dies, all adjacent enemy allies get Fortify(1) — they're enraged.
pub fn on_enemy_death_revenge(battle: &mut TacticalBattle, dead_idx: usize) {
    if !battle.units[dead_idx].is_enemy() {
        return;
    }

    let dx = battle.units[dead_idx].x;
    let dy = battle.units[dead_idx].y;
    let dead_name = battle.units[dead_idx].hanzi.to_string();

    let deltas = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)];
    let mut enraged = Vec::new();

    for (ddx, ddy) in &deltas {
        let nx = dx + ddx;
        let ny = dy + ddy;
        if let Some(idx) = battle.unit_at(nx, ny) {
            if battle.units[idx].is_enemy() && battle.units[idx].alive {
                enraged.push(idx);
            }
        }
    }

    for &idx in &enraged {
        battle.units[idx].fortify_stacks += 1;
        let ally_name = battle.units[idx].hanzi.to_string();
        battle.log_message(format!(
            "😡 {} is enraged by {}'s death! (+1 damage)",
            ally_name, dead_name
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::test_helpers::{make_test_battle, make_test_unit};
    use crate::combat::UnitKind;
    use crate::enemy::AiBehavior;

    #[test]
    fn coordinated_attack_bonus_zero_on_first_attack() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 2, 0);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.attacks_on_player_this_round = 0;

        assert_eq!(coordinated_attack_bonus(&battle), 0);
    }

    #[test]
    fn coordinated_attack_bonus_one_on_subsequent_attacks() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 2, 0);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.attacks_on_player_this_round = 1;

        assert_eq!(coordinated_attack_bonus(&battle), 1);
    }

    #[test]
    fn pack_tactics_counts_adjacent_allies() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut pack_enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        pack_enemy.ai = AiBehavior::Pack;
        // Adjacent enemies
        let ally1 = make_test_unit(UnitKind::Enemy(1), 4, 3); // right
        let ally2 = make_test_unit(UnitKind::Enemy(2), 3, 4); // below

        let battle = make_test_battle(vec![player, pack_enemy, ally1, ally2]);
        let bonus = pack_tactics_bonus(&battle, 1);
        assert_eq!(bonus, 2); // 2 adjacent allies
    }

    #[test]
    fn pack_tactics_zero_for_non_pack_ai() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut chase_enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        chase_enemy.ai = AiBehavior::Chase;
        let ally = make_test_unit(UnitKind::Enemy(1), 4, 3);

        let battle = make_test_battle(vec![player, chase_enemy, ally]);
        assert_eq!(pack_tactics_bonus(&battle, 1), 0);
    }

    #[test]
    fn total_attack_synergy_bonus_combines_all_sources() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut pack_enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        pack_enemy.ai = AiBehavior::Pack;
        pack_enemy.synergy_damage_bonus = 2;
        pack_enemy.sacrifice_bonus_damage = 1;
        let ally = make_test_unit(UnitKind::Enemy(1), 4, 3);

        let mut battle = make_test_battle(vec![player, pack_enemy, ally]);
        battle.attacks_on_player_this_round = 1; // +1 coordinated

        let (bonus, msgs) = total_attack_synergy_bonus(&battle, 1);
        // pack (1 adjacent) + coordinated (1) + synergy_damage (2) + sacrifice (1) = 5
        assert_eq!(bonus, 5);
        assert!(!msgs.is_empty());
    }

    #[test]
    fn on_enemy_death_revenge_enrages_adjacent_enemies() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut dead_enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        dead_enemy.alive = false;
        dead_enemy.hp = 0;
        let mut adjacent_enemy = make_test_unit(UnitKind::Enemy(1), 4, 3);
        adjacent_enemy.fortify_stacks = 0;
        let far_enemy = make_test_unit(UnitKind::Enemy(2), 6, 6);

        let mut battle = make_test_battle(vec![player, dead_enemy, adjacent_enemy, far_enemy]);

        on_enemy_death_revenge(&mut battle, 1);

        assert_eq!(battle.units[2].fortify_stacks, 1); // adjacent → enraged
        assert_eq!(battle.units[3].fortify_stacks, 0); // far → not enraged
    }

    #[test]
    fn on_enemy_death_revenge_ignores_player_death() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.alive = false;
        let adjacent_enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);

        let mut battle = make_test_battle(vec![player, adjacent_enemy]);

        on_enemy_death_revenge(&mut battle, 0);

        // Player death should not trigger revenge
        assert_eq!(battle.units[1].fortify_stacks, 0);
    }
}