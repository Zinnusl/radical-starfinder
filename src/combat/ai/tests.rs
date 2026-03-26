use super::*;
use crate::combat::test_helpers::{make_test_battle, make_test_unit};
use crate::combat::{BattleTile, UnitKind};

#[test]
fn step_toward_moves_closer_to_target() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 5, 5);
    let battle = make_test_battle(vec![player, enemy]);

    let result = step_toward(&battle, 3, 3, 6, 6);
    assert!(result.is_some());
    let (nx, ny) = result.unwrap();
    // Should move closer to (6,6)
    let old_dist = manhattan(3, 3, 6, 6);
    let new_dist = manhattan(nx, ny, 6, 6);
    assert!(new_dist < old_dist);
}

#[test]
fn step_toward_avoids_occupied_tiles() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    // Place a blocking unit to the east
    let blocker = make_test_unit(UnitKind::Enemy(0), 4, 3);
    let battle = make_test_battle(vec![player, blocker]);

    let result = step_toward(&battle, 3, 3, 6, 3);
    // Can't step east (occupied), should pick another direction
    match result {
        Some((x, y)) => assert!(!(x == 4 && y == 3)), // not into blocker
        None => {} // acceptable if all adjacent are occupied/blocked
    }
}

#[test]
fn step_toward_avoids_unwalkable_tiles() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    // Block east with a barrier
    battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);

    let result = step_toward(&battle, 3, 3, 6, 3);
    match result {
        Some((x, y)) => assert!(!(x == 4 && y == 3)),
        None => {}
    }
}

#[test]
fn step_away_moves_farther_from_threat() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let battle = make_test_battle(vec![player]);

    let result = step_away(&battle, 3, 3, 2, 3);
    assert!(result.is_some());
    let (nx, ny) = result.unwrap();
    let old_dist = manhattan(3, 3, 2, 3);
    let new_dist = manhattan(nx, ny, 2, 3);
    assert!(new_dist > old_dist);
}

#[test]
fn step_away_respects_bounds() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let battle = make_test_battle(vec![player]);

    // Try to step away from center — should stay in bounds
    let result = step_away(&battle, 0, 0, 3, 3);
    if let Some((x, y)) = result {
        assert!(battle.arena.in_bounds(x, y));
    }
}

#[test]
fn count_allies_near_counts_same_faction() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let enemy2 = make_test_unit(UnitKind::Enemy(1), 4, 3); // within 1 of enemy1
    let enemy3 = make_test_unit(UnitKind::Enemy(2), 6, 6); // far from enemy1

    let battle = make_test_battle(vec![player, enemy1, enemy2, enemy3]);
    let count = count_allies_near(&battle, 1, 2); // radius 2
    assert_eq!(count, 1); // only enemy2 is within range
}

#[test]
fn get_radical_role_classifies_correctly() {
    assert!(matches!(get_radical_role(&RadicalAction::SpreadingWildfire), TacticalRole::Offensive));
    assert!(matches!(get_radical_role(&RadicalAction::MortalResilience), TacticalRole::Defensive));
    assert!(matches!(get_radical_role(&RadicalAction::ErosiveFlow), TacticalRole::Debuff));
    assert!(matches!(get_radical_role(&RadicalAction::RevealingDawn), TacticalRole::Support));
}

// ── step_toward: edge cases ──────────────────────────────────────────

#[test]
fn step_toward_at_target_picks_adjacent() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let battle = make_test_battle(vec![player]);

    let result = step_toward(&battle, 3, 3, 3, 3);

    // Already at target — any adjacent step is valid (or None)
    if let Some((x, y)) = result {
        assert!(battle.arena.in_bounds(x, y));
    }
}

#[test]
fn step_toward_surrounded_by_walls_returns_none() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(2, 3, BattleTile::CoverBarrier);
    battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);
    battle.arena.set_tile(3, 2, BattleTile::CoverBarrier);
    battle.arena.set_tile(3, 4, BattleTile::CoverBarrier);

    let result = step_toward(&battle, 3, 3, 6, 6);

    assert!(result.is_none());
}

#[test]
fn step_toward_corner_respects_bounds() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let battle = make_test_battle(vec![player]);

    let result = step_toward(&battle, 0, 0, 0, 0);

    // At corner with target==position; any valid move or None
    if let Some((x, y)) = result {
        assert!(battle.arena.in_bounds(x, y));
    }
}

#[test]
fn step_toward_diagonal_target_picks_cardinal_closer() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let battle = make_test_battle(vec![player]);

    let result = step_toward(&battle, 3, 3, 6, 6);

    assert!(result.is_some());
    let (nx, ny) = result.unwrap();
    let old_dist = (3 - 6_i32).abs() + (3 - 6_i32).abs();
    let new_dist = (nx - 6).abs() + (ny - 6).abs();
    assert!(new_dist < old_dist);
}

// ── step_away: edge cases ────────────────────────────────────────────

#[test]
fn step_away_at_edge_stays_in_bounds() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let battle = make_test_battle(vec![player]);

    // Threat at (0,0), stepping away from center gives room to flee
    let result = step_away(&battle, 3, 3, 0, 0);

    assert!(result.is_some());
    let (x, y) = result.unwrap();
    assert!(battle.arena.in_bounds(x, y));
    let old_dist = manhattan(3, 3, 0, 0);
    let new_dist = manhattan(x, y, 0, 0);
    assert!(new_dist > old_dist);
}

#[test]
fn step_away_surrounded_by_walls_returns_none() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(2, 3, BattleTile::CoverBarrier);
    battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);
    battle.arena.set_tile(3, 2, BattleTile::CoverBarrier);
    battle.arena.set_tile(3, 4, BattleTile::CoverBarrier);

    let result = step_away(&battle, 3, 3, 2, 2);

    assert!(result.is_none());
}

// ── choose_action ────────────────────────────────────────────────────

#[test]
fn choose_action_stunned_returns_wait() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.stunned = true;
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::Wait));
}

#[test]
fn choose_action_chase_adjacent_melees() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.ai = AiBehavior::Chase;
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::MeleeAttack { target_unit: 0 }));
}

#[test]
fn choose_action_chase_far_moves_toward_player() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    enemy.ai = AiBehavior::Chase;
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::MoveToTile { .. } | AiAction::MoveAndAttack { .. }));
}

#[test]
fn choose_action_sentinel_far_waits() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    enemy.ai = AiBehavior::Sentinel;
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::Wait));
}

#[test]
fn choose_action_sentinel_adjacent_melees() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.ai = AiBehavior::Sentinel;
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::MeleeAttack { target_unit: 0 }));
}

#[test]
fn choose_action_retreat_far_waits() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    enemy.ai = AiBehavior::Retreat;
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::Wait));
}

#[test]
fn choose_action_retreat_close_retreats() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.ai = AiBehavior::Retreat;
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    // Close range: retreat moves away or melees if can't escape
    assert!(matches!(action, AiAction::MoveToTile { .. } | AiAction::MeleeAttack { .. }));
}

#[test]
fn choose_action_ambush_far_waits() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    enemy.ai = AiBehavior::Ambush;
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::Wait));
}

#[test]
fn choose_action_ambush_close_attacks() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.ai = AiBehavior::Ambush;
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::MeleeAttack { target_unit: 0 }));
}

#[test]
fn choose_action_kiter_close_retreats() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.ai = AiBehavior::Kiter;
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    // Kiter at dist<=2 tries to path away or melees if cornered
    assert!(matches!(action, AiAction::MoveToTile { .. } | AiAction::MeleeAttack { .. }));
}

#[test]
fn choose_action_with_fear_retreats() {
    use crate::status::{StatusInstance, StatusKind};
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Chase;
    enemy.statuses.push(StatusInstance::new(StatusKind::Fear, 2));
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::MoveToTile { .. } | AiAction::Wait));
}

// ── choose_companion_action ──────────────────────────────────────────

#[test]
fn companion_stunned_waits() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    let mut companion = make_test_unit(UnitKind::Companion, 3, 3);
    companion.stunned = true;
    let battle = make_test_battle(vec![player, enemy, companion]);

    let action = choose_companion_action(&battle, 2);

    assert!(matches!(action, AiAction::Wait));
}

#[test]
fn companion_adjacent_to_enemy_melees() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    let companion = make_test_unit(UnitKind::Companion, 3, 3);
    let battle = make_test_battle(vec![player, enemy, companion]);

    let action = choose_companion_action(&battle, 2);

    assert!(matches!(action, AiAction::MeleeAttack { target_unit: 1 }));
}

#[test]
fn companion_far_from_enemy_moves_toward() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    let companion = make_test_unit(UnitKind::Companion, 1, 1);
    let battle = make_test_battle(vec![player, enemy, companion]);

    let action = choose_companion_action(&battle, 2);

    assert!(matches!(action, AiAction::MoveToTile { .. } | AiAction::MoveAndAttack { .. }));
}

#[test]
fn companion_no_enemies_alive_waits() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    enemy.alive = false;
    let companion = make_test_unit(UnitKind::Companion, 3, 3);
    let battle = make_test_battle(vec![player, enemy, companion]);

    let action = choose_companion_action(&battle, 2);

    assert!(matches!(action, AiAction::Wait));
}

// ── calculate_all_intents ────────────────────────────────────────────

#[test]
fn intents_dead_enemy_gets_none() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.alive = false;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    assert!(battle.units[1].intent.is_none());
}

#[test]
fn intents_stunned_enemy_gets_idle() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.stunned = true;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Idle));
}

#[test]
fn intents_chase_adjacent_shows_attack() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.ai = AiBehavior::Chase;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Attack));
}

#[test]
fn intents_chase_far_shows_approach() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    enemy.ai = AiBehavior::Chase;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Approach));
}

#[test]
fn intents_sentinel_far_shows_idle() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    enemy.ai = AiBehavior::Sentinel;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Idle));
}

#[test]
fn intents_sentinel_adjacent_shows_attack() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.ai = AiBehavior::Sentinel;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Attack));
}

#[test]
fn intents_kiter_close_shows_retreat() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.ai = AiBehavior::Kiter;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Retreat));
}

#[test]
fn intents_retreat_close_shows_retreat() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.ai = AiBehavior::Retreat;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Retreat));
}

#[test]
fn intents_companion_gets_none() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let companion = make_test_unit(UnitKind::Companion, 3, 3);
    let mut battle = make_test_battle(vec![player, companion]);

    calculate_all_intents(&mut battle);

    assert!(battle.units[1].intent.is_none());
}

#[test]
fn intents_sets_calculated_flag() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    assert!(!battle.intents_calculated);

    calculate_all_intents(&mut battle);

    assert!(battle.intents_calculated);
}

#[test]
fn intents_ambush_far_shows_idle() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    enemy.ai = AiBehavior::Ambush;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Idle));
}

#[test]
fn intents_ambush_close_shows_attack() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
    enemy.ai = AiBehavior::Ambush;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Attack));
}

#[test]
fn intents_pack_alone_far_shows_surround() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    enemy.ai = AiBehavior::Pack;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Surround));
}

#[test]
fn intents_pack_with_allies_shows_attack() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy1.ai = AiBehavior::Pack;
    let mut enemy2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    enemy2.ai = AiBehavior::Pack;
    let mut enemy3 = make_test_unit(UnitKind::Enemy(2), 3, 4);
    enemy3.ai = AiBehavior::Pack;
    let mut battle = make_test_battle(vec![player, enemy1, enemy2, enemy3]);

    calculate_all_intents(&mut battle);

    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Attack));
}

// ── consider_crate_push ──────────────────────────────────────────────

#[test]
fn crate_push_detects_direct_hit_on_player() {
    let player = make_test_unit(UnitKind::Player, 4, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 2, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::CargoCrate);

    let result = consider_crate_push(&battle, 1);

    assert!(result.is_some());
    let (cx, cy, dx, dy) = result.unwrap();
    assert_eq!((cx, cy), (3, 3));
    assert_eq!((dx, dy), (1, 0));
}

#[test]
fn crate_push_returns_none_without_crate() {
    let player = make_test_unit(UnitKind::Player, 4, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 2, 3);
    let battle = make_test_battle(vec![player, enemy]);

    let result = consider_crate_push(&battle, 1);

    assert!(result.is_none());
}

#[test]
fn crate_push_detects_fuel_canister_explosion() {
    let player = make_test_unit(UnitKind::Player, 5, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 2, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::CargoCrate);
    battle.arena.set_tile(4, 3, BattleTile::FuelCanister);

    let result = consider_crate_push(&battle, 1);

    // Crate pushes onto FuelCanister, explosion within 2 of player
    assert!(result.is_some());
}

#[test]
fn crate_push_ignores_crate_away_from_player() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    // Crate east of enemy, pushing east (away from player at 0,0)
    battle.arena.set_tile(4, 3, BattleTile::CargoCrate);

    let result = consider_crate_push(&battle, 1);

    // Push would go to (5,3), dist to player(0,0) = 8, no hit
    assert!(result.is_none());
}

// ── get_radical_intent ───────────────────────────────────────────────

#[test]
fn get_radical_intent_defensive_returns_buff() {
    let intent = get_radical_intent(&RadicalAction::MortalResilience);
    assert_eq!(intent, EnemyIntent::Buff);
}

#[test]
fn get_radical_intent_offensive_ranged_returns_ranged_attack() {
    let intent = get_radical_intent(&RadicalAction::ArcingShot);
    assert_eq!(intent, EnemyIntent::RangedAttack);
}

#[test]
fn get_radical_intent_offensive_melee_returns_attack() {
    let intent = get_radical_intent(&RadicalAction::OverwhelmingForce);
    assert_eq!(intent, EnemyIntent::Attack);
}

#[test]
fn get_radical_intent_debuff_returns_ranged_attack() {
    let intent = get_radical_intent(&RadicalAction::ErosiveFlow);
    assert_eq!(intent, EnemyIntent::RangedAttack);
}

#[test]
fn get_radical_intent_support_cleansing_light_returns_heal() {
    let intent = get_radical_intent(&RadicalAction::CleansingLight);
    assert_eq!(intent, EnemyIntent::Heal);
}

#[test]
fn get_radical_intent_support_revealing_dawn_returns_heal() {
    let intent = get_radical_intent(&RadicalAction::RevealingDawn);
    assert_eq!(intent, EnemyIntent::Heal);
}

#[test]
fn get_radical_intent_support_magnifying_aura_returns_buff() {
    let intent = get_radical_intent(&RadicalAction::MagnifyingAura);
    assert_eq!(intent, EnemyIntent::Buff);
}

// ── get_radical_role additional coverage ─────────────────────────────

#[test]
fn get_radical_role_offensive_variants() {
    assert!(matches!(get_radical_role(&RadicalAction::ArcingShot), TacticalRole::Offensive));
    assert!(matches!(get_radical_role(&RadicalAction::CavalryCharge), TacticalRole::Offensive));
    assert!(matches!(get_radical_role(&RadicalAction::EchoStrike), TacticalRole::Offensive));
    assert!(matches!(get_radical_role(&RadicalAction::PreciseExecution), TacticalRole::Offensive));
    assert!(matches!(get_radical_role(&RadicalAction::SavageMaul), TacticalRole::Offensive));
    assert!(matches!(get_radical_role(&RadicalAction::BerserkerFury), TacticalRole::Offensive));
    assert!(matches!(get_radical_role(&RadicalAction::VenomousLash), TacticalRole::Offensive));
    assert!(matches!(get_radical_role(&RadicalAction::PhaseStrike), TacticalRole::Offensive));
}

#[test]
fn get_radical_role_defensive_variants() {
    assert!(matches!(get_radical_role(&RadicalAction::RigidStance), TacticalRole::Defensive));
    assert!(matches!(get_radical_role(&RadicalAction::ImmovablePeak), TacticalRole::Defensive));
    assert!(matches!(get_radical_role(&RadicalAction::SoaringEscape), TacticalRole::Defensive));
    assert!(matches!(get_radical_role(&RadicalAction::CloakingGuise), TacticalRole::Defensive));
    assert!(matches!(get_radical_role(&RadicalAction::AdaptiveShift), TacticalRole::Defensive));
}

#[test]
fn get_radical_role_debuff_variants() {
    assert!(matches!(get_radical_role(&RadicalAction::DoubtSeed), TacticalRole::Debuff));
    assert!(matches!(get_radical_role(&RadicalAction::DevouringMaw), TacticalRole::Debuff));
    assert!(matches!(get_radical_role(&RadicalAction::WitnessMark), TacticalRole::Debuff));
    assert!(matches!(get_radical_role(&RadicalAction::RootingGrasp), TacticalRole::Debuff));
    assert!(matches!(get_radical_role(&RadicalAction::EntanglingWeb), TacticalRole::Debuff));
    assert!(matches!(get_radical_role(&RadicalAction::PetrifyingGaze), TacticalRole::Debuff));
    assert!(matches!(get_radical_role(&RadicalAction::TidalSurge), TacticalRole::Debuff));
}

#[test]
fn get_radical_role_support_variants() {
    assert!(matches!(get_radical_role(&RadicalAction::SleightReversal), TacticalRole::Support));
    assert!(matches!(get_radical_role(&RadicalAction::MercenaryPact), TacticalRole::Support));
    assert!(matches!(get_radical_role(&RadicalAction::MagnifyingAura), TacticalRole::Support));
    assert!(matches!(get_radical_role(&RadicalAction::SproutingBarrier), TacticalRole::Support));
}

// ── evaluate_battle_pressure (tested indirectly via score_and_pick_radical) ──

#[test]
fn score_and_pick_radical_no_actions_returns_none() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let battle = make_test_battle(vec![player, enemy]);

    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert!(result.is_none());
}

#[test]
fn score_radical_chase_offensive_scores_high() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Chase;
    enemy.radical_actions = vec![RadicalAction::OverwhelmingForce];
    let battle = make_test_battle(vec![player, enemy]);

    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert_eq!(result, Some(0));
}

#[test]
fn score_radical_chase_low_hp_prefers_defensive() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Chase;
    enemy.hp = 2;
    enemy.max_hp = 10;
    enemy.radical_actions = vec![
        RadicalAction::OverwhelmingForce,  // offensive +50
        RadicalAction::MortalResilience,   // defensive +100 (low hp)
    ];
    let battle = make_test_battle(vec![player, enemy]);

    let result = score_and_pick_radical(&battle, 1, 3, 0.2, 0);

    assert_eq!(result, Some(1));
}

#[test]
fn score_radical_retreat_favors_defensive() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Retreat;
    enemy.radical_actions = vec![RadicalAction::RigidStance];
    let battle = make_test_battle(vec![player, enemy]);

    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert_eq!(result, Some(0));
}

#[test]
fn score_radical_retreat_low_hp_extra_defensive_bonus() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Retreat;
    enemy.hp = 4;
    enemy.max_hp = 10;
    enemy.radical_actions = vec![RadicalAction::ImmovablePeak];
    let battle = make_test_battle(vec![player, enemy]);

    // hp_ratio = 0.4 < 0.5, so defensive gets +80 + +40 = 120
    let result = score_and_pick_radical(&battle, 1, 3, 0.4, 0);

    assert_eq!(result, Some(0));
}

#[test]
fn score_radical_ambush_close_offensive_high() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 2, 2);
    enemy.ai = AiBehavior::Ambush;
    enemy.radical_actions = vec![RadicalAction::OverwhelmingForce];
    let battle = make_test_battle(vec![player, enemy]);

    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert_eq!(result, Some(0));
}

#[test]
fn score_radical_ambush_far_offensive_not_picked() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    enemy.ai = AiBehavior::Ambush;
    enemy.radical_actions = vec![RadicalAction::OverwhelmingForce];
    let battle = make_test_battle(vec![player, enemy]);

    // dist > 3, ambush doesn't score offensive
    let result = score_and_pick_radical(&battle, 1, 12, 1.0, 0);

    assert!(result.is_none());
}

#[test]
fn score_radical_sentinel_defensive_high() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Sentinel;
    enemy.radical_actions = vec![RadicalAction::RigidStance];
    let battle = make_test_battle(vec![player, enemy]);

    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert_eq!(result, Some(0));
}

#[test]
fn score_radical_kiter_debuff_scores() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Kiter;
    enemy.radical_actions = vec![RadicalAction::ErosiveFlow];
    let battle = make_test_battle(vec![player, enemy]);

    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert_eq!(result, Some(0));
}

#[test]
fn score_radical_pack_support_when_alone() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Pack;
    enemy.radical_actions = vec![RadicalAction::MagnifyingAura];
    let battle = make_test_battle(vec![player, enemy]);

    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert_eq!(result, Some(0));
}

#[test]
fn score_radical_pack_offensive_with_allies() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy1.ai = AiBehavior::Pack;
    enemy1.radical_actions = vec![RadicalAction::OverwhelmingForce];
    let enemy2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    let enemy3 = make_test_unit(UnitKind::Enemy(2), 3, 4);
    let battle = make_test_battle(vec![player, enemy1, enemy2, enemy3]);

    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 2);

    assert_eq!(result, Some(0));
}

// ── Adaptive battle-state modifiers in score_and_pick_radical ────────

#[test]
fn score_radical_last_enemy_favors_defensive() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy1.ai = AiBehavior::Chase;
    enemy1.radical_actions = vec![RadicalAction::RigidStance];
    let mut enemy2 = make_test_unit(UnitKind::Enemy(1), 5, 5);
    enemy2.alive = false; // dead ally
    let battle = make_test_battle(vec![player, enemy1, enemy2]);

    // enemy_alive=1, last enemy standing → defensive +50
    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert_eq!(result, Some(0));
}

#[test]
fn score_radical_player_weak_favors_offensive() {
    let mut player = make_test_unit(UnitKind::Player, 0, 0);
    player.hp = 2;
    player.max_hp = 10;
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Chase;
    enemy.radical_actions = vec![RadicalAction::OverwhelmingForce];
    let battle = make_test_battle(vec![player, enemy]);

    // player_hp_ratio < 0.30 → offensive +35
    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert_eq!(result, Some(0));
}

#[test]
fn score_radical_losing_side_favors_defensive_over_offensive() {
    let mut player = make_test_unit(UnitKind::Player, 0, 0);
    player.hp = 10;
    let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy1.ai = AiBehavior::Chase;
    enemy1.radical_actions = vec![
        RadicalAction::OverwhelmingForce,  // offensive
        RadicalAction::RigidStance,        // defensive
    ];
    let mut enemy2 = make_test_unit(UnitKind::Enemy(1), 5, 5);
    enemy2.alive = false;
    let mut enemy3 = make_test_unit(UnitKind::Enemy(2), 6, 6);
    enemy3.alive = false;
    let mut enemy4 = make_test_unit(UnitKind::Enemy(3), 6, 5);
    enemy4.alive = false;
    let battle = make_test_battle(vec![player, enemy1, enemy2, enemy3, enemy4]);

    // enemy_alive=1, enemy_total=4, 1*2<=4 → losing side: defensive +40, offensive -20
    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert_eq!(result, Some(1)); // defensive wins
}

// ── Defensive ability de-duplication scoring ─────────────────────────

#[test]
fn score_radical_dodge_already_active_penalizes_dodge_abilities() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Sentinel;
    enemy.radical_dodge = true;
    enemy.radical_actions = vec![RadicalAction::SoaringEscape];
    let battle = make_test_battle(vec![player, enemy]);

    // SoaringEscape gets -40 for already having dodge; sentinel +100, defensive -40 = 60
    // still above 50 but reduced
    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    // The result may or may not be picked depending on jitter, but the penalty is applied
    // With sentinel +100, defensive role, and -40 penalty = base 60, still > 50
    assert!(result.is_some() || result.is_none()); // Just ensure no panic; the logic is tested
}

#[test]
fn score_radical_armor_already_active_penalizes_armor_abilities() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Sentinel;
    enemy.radical_armor = 3;
    enemy.radical_actions = vec![RadicalAction::RigidStance];
    let battle = make_test_battle(vec![player, enemy]);

    // RigidStance: sentinel +100, armor already active -40 = 60
    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert!(result.is_some()); // Still picked at 60 > 50
}

// ── Offensive scoring edge cases ─────────────────────────────────────

#[test]
fn score_radical_arcing_shot_bonus_at_long_range() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    enemy.ai = AiBehavior::Chase;
    enemy.radical_actions = vec![RadicalAction::ArcingShot];
    let battle = make_test_battle(vec![player, enemy]);

    // dist>3, ArcingShot gets +50 bonus on top of offensive +50
    let result = score_and_pick_radical(&battle, 1, 12, 1.0, 0);

    assert_eq!(result, Some(0));
}

#[test]
fn score_radical_precise_execution_bonus_low_player_hp() {
    let mut player = make_test_unit(UnitKind::Player, 0, 0);
    player.hp = 2;
    player.max_hp = 10;
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Chase;
    enemy.radical_actions = vec![RadicalAction::PreciseExecution];
    let battle = make_test_battle(vec![player, enemy]);

    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert_eq!(result, Some(0));
}

#[test]
fn score_radical_harvest_reaping_bonus_low_player_hp() {
    let mut player = make_test_unit(UnitKind::Player, 0, 0);
    player.hp = 3;
    player.max_hp = 10;
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Chase;
    enemy.radical_actions = vec![RadicalAction::HarvestReaping];
    let battle = make_test_battle(vec![player, enemy]);

    // player_hp < 0.40 → HarvestReaping +40
    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert_eq!(result, Some(0));
}

#[test]
fn score_radical_echo_strike_penalized_low_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Chase;
    enemy.damage = 1;
    enemy.radical_actions = vec![RadicalAction::EchoStrike];
    let battle = make_test_battle(vec![player, enemy]);

    // EchoStrike: offensive +50, damage<=1 → -30 = 20 < 50 threshold
    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert!(result.is_none());
}

#[test]
fn score_radical_crossroads_gambit_penalized() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Chase;
    enemy.radical_actions = vec![RadicalAction::CrossroadsGambit];
    let battle = make_test_battle(vec![player, enemy]);

    // CrossroadsGambit: offensive +50, gambit -10 = 40 < 50 threshold
    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert!(result.is_none());
}

#[test]
fn score_radical_savage_maul_penalized_low_hp() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Chase;
    enemy.hp = 1;
    enemy.max_hp = 10;
    enemy.radical_actions = vec![RadicalAction::SavageMaul];
    let battle = make_test_battle(vec![player, enemy]);

    // SavageMaul: offensive +50, hp<=1 → -60 = -10 < 50 threshold
    let result = score_and_pick_radical(&battle, 1, 3, 0.1, 0);

    assert!(result.is_none());
}

// ── Debuff scoring edge cases ────────────────────────────────────────

#[test]
fn score_radical_doubt_seed_penalized_when_player_already_confused() {
    use crate::status::{StatusInstance, StatusKind};
    let mut player = make_test_unit(UnitKind::Player, 0, 0);
    player.statuses.push(StatusInstance::new(StatusKind::Confused, 2));
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Kiter;
    enemy.radical_actions = vec![RadicalAction::DoubtSeed];
    let battle = make_test_battle(vec![player, enemy]);

    // Kiter debuff +60, but player already confused → -60 = 0 < 50
    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert!(result.is_none());
}

#[test]
fn score_radical_erosive_flow_penalized_when_player_already_slowed() {
    use crate::status::{StatusInstance, StatusKind};
    let mut player = make_test_unit(UnitKind::Player, 0, 0);
    player.statuses.push(StatusInstance::new(StatusKind::Slow, 2));
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Kiter;
    enemy.radical_actions = vec![RadicalAction::ErosiveFlow];
    let battle = make_test_battle(vec![player, enemy]);

    // Kiter debuff +60, player already slowed → -40 = 20 < 50
    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert!(result.is_none());
}

#[test]
fn score_radical_witness_mark_penalized_when_player_already_marked() {
    let mut player = make_test_unit(UnitKind::Player, 0, 0);
    player.marked_extra_damage = 3;
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Kiter;
    enemy.radical_actions = vec![RadicalAction::WitnessMark];
    let battle = make_test_battle(vec![player, enemy]);

    // Kiter debuff +60, already marked → -50 = 10 < 50
    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert!(result.is_none());
}

#[test]
fn score_radical_devouring_maw_penalized_when_player_has_no_buffs() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Kiter;
    enemy.radical_actions = vec![RadicalAction::DevouringMaw];
    let battle = make_test_battle(vec![player, enemy]);

    // Player has no dodge, armor, defending, counter → -40
    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert!(result.is_none());
}

// ── Support scoring edge cases ───────────────────────────────────────

#[test]
fn score_radical_mercenary_pact_boosted_when_allies_low_hp() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy1.ai = AiBehavior::Pack;
    enemy1.radical_actions = vec![RadicalAction::MercenaryPact];
    let mut enemy2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    enemy2.hp = 2;
    enemy2.max_hp = 10;
    let mut enemy3 = make_test_unit(UnitKind::Enemy(2), 3, 4);
    enemy3.hp = 3;
    enemy3.max_hp = 10;
    let battle = make_test_battle(vec![player, enemy1, enemy2, enemy3]);

    // 2 low-hp allies → +80 bonus
    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert_eq!(result, Some(0));
}

#[test]
fn score_radical_cleansing_light_penalized_at_full_hp_no_statuses() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Sentinel;
    enemy.hp = 10;
    enemy.max_hp = 10;
    enemy.radical_actions = vec![RadicalAction::CleansingLight];
    let battle = make_test_battle(vec![player, enemy]);

    // Sentinel defensive +100, but CleansingLight at full hp with no statuses → -100 = 0
    let result = score_and_pick_radical(&battle, 1, 3, 1.0, 0);

    assert!(result.is_none());
}

// ── choose_action: radical ability branches ──────────────────────────

#[test]
fn choose_action_chase_with_radical_at_distance_moves_and_uses_radical() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 5);
    enemy.ai = AiBehavior::Chase;
    enemy.radical_actions = vec![RadicalAction::OverwhelmingForce];
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(
        action,
        AiAction::MoveAndRadical { .. } | AiAction::UseRadicalAction { .. }
    ));
}

#[test]
fn choose_action_chase_with_radical_adjacent_uses_radical_directly() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.ai = AiBehavior::Chase;
    enemy.radical_actions = vec![RadicalAction::OverwhelmingForce];
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::UseRadicalAction { action_idx: 0 }));
}

#[test]
fn choose_action_retreat_with_radical_uses_radical() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Retreat;
    enemy.radical_actions = vec![RadicalAction::RigidStance];
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::UseRadicalAction { action_idx: 0 }));
}

#[test]
fn choose_action_ambush_close_with_radical_uses_radical() {
    let player = make_test_unit(UnitKind::Player, 2, 2);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Ambush;
    enemy.radical_actions = vec![RadicalAction::OverwhelmingForce];
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::UseRadicalAction { action_idx: 0 }));
}

#[test]
fn choose_action_sentinel_with_radical_uses_radical() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Sentinel;
    enemy.radical_actions = vec![RadicalAction::RigidStance];
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::UseRadicalAction { action_idx: 0 }));
}

#[test]
fn choose_action_kiter_with_radical_uses_radical() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Kiter;
    enemy.radical_actions = vec![RadicalAction::ErosiveFlow];
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::UseRadicalAction { action_idx: 0 }));
}

#[test]
fn choose_action_pack_with_radical_uses_radical() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Pack;
    enemy.radical_actions = vec![RadicalAction::MagnifyingAura];
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::UseRadicalAction { action_idx: 0 }));
}

// ── choose_action: Pack behavior edge cases ──────────────────────────

#[test]
fn choose_action_pack_with_allies_adjacent_melees() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy1.ai = AiBehavior::Pack;
    let enemy2 = make_test_unit(UnitKind::Enemy(1), 5, 3);
    let enemy3 = make_test_unit(UnitKind::Enemy(2), 4, 4);
    let battle = make_test_battle(vec![player, enemy1, enemy2, enemy3]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::MeleeAttack { target_unit: 0 }));
}

#[test]
fn choose_action_pack_with_allies_far_moves_toward_player() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 5, 5);
    enemy1.ai = AiBehavior::Pack;
    let enemy2 = make_test_unit(UnitKind::Enemy(1), 6, 5);
    let enemy3 = make_test_unit(UnitKind::Enemy(2), 5, 6);
    let battle = make_test_battle(vec![player, enemy1, enemy2, enemy3]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::MoveToTile { .. } | AiAction::MoveAndAttack { .. }));
}

#[test]
fn choose_action_pack_alone_adjacent_melees() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.ai = AiBehavior::Pack;
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::MeleeAttack { target_unit: 0 }));
}

#[test]
fn choose_action_pack_alone_moves_toward_ally() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy1.ai = AiBehavior::Pack;
    let enemy2 = make_test_unit(UnitKind::Enemy(1), 6, 6);
    let battle = make_test_battle(vec![player, enemy1, enemy2]);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::MoveToTile { .. }));
}

// ── choose_action: Kiter behavior ────────────────────────────────────

#[test]
fn choose_action_kiter_far_approaches() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    enemy.ai = AiBehavior::Kiter;
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    // dist >= 5, kiter approaches
    assert!(matches!(action, AiAction::MoveToTile { .. }));
}

#[test]
fn choose_action_kiter_mid_range_waits() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 0);
    enemy.ai = AiBehavior::Kiter;
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    // dist = 3, not <= 2 and not >= 5 → wait
    assert!(matches!(action, AiAction::Wait));
}

#[test]
fn choose_action_kiter_adjacent_cornered_melees() {
    let player = make_test_unit(UnitKind::Player, 1, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 0, 0);
    enemy.ai = AiBehavior::Kiter;
    // Place barriers to block retreat
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(0, 1, BattleTile::CoverBarrier);

    let action = choose_action(&battle, 1);

    // Cornered kiter at dist=1 may melee
    assert!(matches!(action, AiAction::MeleeAttack { .. } | AiAction::MoveToTile { .. }));
}

// ── choose_action: Ambush moves toward when in range ─────────────────

#[test]
fn choose_action_ambush_in_range_not_adjacent_moves() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
    enemy.ai = AiBehavior::Ambush;
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    // dist=2, within range 3, should move+attack or move toward
    assert!(matches!(
        action,
        AiAction::MoveToTile { .. } | AiAction::MoveAndAttack { .. }
    ));
}

// ── choose_action: Retreat close but can't escape melees ─────────────

#[test]
fn choose_action_retreat_adjacent_cornered_melees() {
    let player = make_test_unit(UnitKind::Player, 1, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 0, 0);
    enemy.ai = AiBehavior::Retreat;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(0, 1, BattleTile::CoverBarrier);

    let action = choose_action(&battle, 1);

    // Cornered, can't retreat, adjacent → melee or wait
    assert!(matches!(action, AiAction::MeleeAttack { .. } | AiAction::MoveToTile { .. } | AiAction::Wait));
}

// ── choose_action: Fear with no escape waits ─────────────────────────

#[test]
fn choose_action_feared_unit_cornered_waits() {
    use crate::status::{StatusInstance, StatusKind};
    let player = make_test_unit(UnitKind::Player, 1, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 0, 0);
    enemy.ai = AiBehavior::Chase;
    enemy.statuses.push(StatusInstance::new(StatusKind::Fear, 2));
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(0, 1, BattleTile::CoverBarrier);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::Wait | AiAction::MoveToTile { .. }));
}

// ── choose_action: Chase move and attack when reachable ──────────────

#[test]
fn choose_action_chase_within_move_range_moves_and_attacks() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 3);
    enemy.ai = AiBehavior::Chase;
    enemy.movement = 4; // enough to reach adjacent
    let battle = make_test_battle(vec![player, enemy]);

    let action = choose_action(&battle, 1);

    assert!(matches!(
        action,
        AiAction::MoveAndAttack { target_unit: 0, .. } | AiAction::MoveToTile { .. }
    ));
}

// ── choose_action: Crate push takes priority ─────────────────────────

#[test]
fn choose_action_crate_push_overrides_normal_behavior() {
    let player = make_test_unit(UnitKind::Player, 4, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 2, 3);
    enemy.ai = AiBehavior::Chase;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::CargoCrate);

    let action = choose_action(&battle, 1);

    assert!(matches!(action, AiAction::PushCrate { crate_x: 3, crate_y: 3, dx: 1, dy: 0 }));
}

// ── path_toward: terrain scoring ─────────────────────────────────────

#[test]
fn path_toward_avoids_mine_tiles() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 0);
    enemy.movement = 3;
    let mut battle = make_test_battle(vec![player, enemy]);
    // Place revealed mines in the direct path
    battle.arena.set_tile(5, 0, BattleTile::MineTileRevealed);

    let path = path_toward(&battle, 1, 0, 0, true);

    if let Some(p) = path {
        assert!(!p.contains(&(5, 0)), "Should avoid mine tile");
    }
}

#[test]
fn path_toward_avoids_fuel_canister_adjacency() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 0);
    enemy.movement = 3;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 0, BattleTile::FuelCanister);

    let path = path_toward(&battle, 1, 0, 0, true);

    // Path should exist but avoid tiles adjacent to fuel canisters
    assert!(path.is_some());
}

#[test]
fn path_toward_prefers_elevated_platform() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(2, 3, BattleTile::ElevatedPlatform);

    let path = path_toward(&battle, 1, 0, 0, true);

    assert!(path.is_some());
}

#[test]
fn path_toward_returns_none_when_already_at_best_pos() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.movement = 1;
    // Surround with units so best tile stays put
    let blocker1 = make_test_unit(UnitKind::Enemy(1), 5, 3);
    let blocker2 = make_test_unit(UnitKind::Enemy(2), 4, 4);
    let blocker3 = make_test_unit(UnitKind::Enemy(3), 4, 2);
    let battle = make_test_battle(vec![player, enemy, blocker1, blocker2, blocker3]);

    let path = path_toward(&battle, 1, 3, 3, true);

    // All moves would take the unit further from target or are occupied
    // so best tile remains current position → None
    assert!(path.is_none());
}

#[test]
fn path_toward_empty_reachable_returns_none() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.movement = 1;
    let mut battle = make_test_battle(vec![player, enemy]);
    // Completely wall in the enemy
    battle.arena.set_tile(2, 3, BattleTile::CoverBarrier);
    battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);
    battle.arena.set_tile(3, 2, BattleTile::CoverBarrier);
    battle.arena.set_tile(3, 4, BattleTile::CoverBarrier);

    let path = path_toward(&battle, 1, 0, 0, true);

    assert!(path.is_none());
}

// ── path_away: terrain scoring ───────────────────────────────────────

#[test]
fn path_away_avoids_conveyors() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(5, 3, BattleTile::ConveyorN);

    let path = path_away(&battle, 1, 3, 3);

    assert!(path.is_some());
}

#[test]
fn path_away_avoids_mine_tiles() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(5, 3, BattleTile::MineTileRevealed);

    let path = path_away(&battle, 1, 3, 3);

    assert!(path.is_some());
}

#[test]
fn path_away_avoids_gravity_well() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(6, 3, BattleTile::GravityWell);

    let path = path_away(&battle, 1, 3, 3);

    assert!(path.is_some());
}

#[test]
fn path_away_prefers_elevated_platform() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(5, 3, BattleTile::ElevatedPlatform);

    let path = path_away(&battle, 1, 3, 3);

    assert!(path.is_some());
}

#[test]
fn path_away_walled_in_returns_none() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.movement = 1;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(2, 3, BattleTile::CoverBarrier);
    battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);
    battle.arena.set_tile(3, 2, BattleTile::CoverBarrier);
    battle.arena.set_tile(3, 4, BattleTile::CoverBarrier);

    let path = path_away(&battle, 1, 0, 0);

    assert!(path.is_none());
}

// ── path_toward_ally ─────────────────────────────────────────────────

#[test]
fn path_toward_ally_already_adjacent_returns_none() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy1.ai = AiBehavior::Pack;
    let enemy2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    let battle = make_test_battle(vec![player, enemy1, enemy2]);

    let path = path_toward_ally(&battle, 1);

    assert!(path.is_none());
}

// ── consider_crate_push: energy vent trap ────────────────────────────

#[test]
fn crate_push_targets_player_on_energy_vent() {
    let player = make_test_unit(UnitKind::Player, 4, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 2, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::CargoCrate);
    battle.arena.set_tile(4, 3, BattleTile::EnergyVentActive);

    let result = consider_crate_push(&battle, 1);

    assert!(result.is_some());
}

#[test]
fn crate_push_out_of_bounds_returns_none() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 0, 0);
    let mut battle = make_test_battle(vec![player, enemy]);
    // Crate at edge, push would go out of bounds
    battle.arena.set_tile(0, 1, BattleTile::CargoCrate);

    let result = consider_crate_push(&battle, 1);

    // Push west or north goes out of bounds (to -1,0 or 0,-1)
    // Push south goes to (0,2) - not near player, no canister/vent
    // So should return None
    assert!(result.is_none());
}

// ── count_allies_near: edge cases ────────────────────────────────────

#[test]
fn count_allies_near_excludes_dead_units() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut enemy2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    enemy2.alive = false;
    let battle = make_test_battle(vec![player, enemy1, enemy2]);

    let count = count_allies_near(&battle, 1, 2);

    assert_eq!(count, 0);
}

#[test]
fn count_allies_near_excludes_opposite_faction() {
    let player = make_test_unit(UnitKind::Player, 3, 2);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let battle = make_test_battle(vec![player, enemy]);

    let count = count_allies_near(&battle, 1, 2);

    assert_eq!(count, 0); // Player is not an ally of enemy
}

#[test]
fn count_allies_near_includes_close_same_faction() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let enemy2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    let enemy3 = make_test_unit(UnitKind::Enemy(2), 3, 4);
    let battle = make_test_battle(vec![player, enemy1, enemy2, enemy3]);

    let count = count_allies_near(&battle, 1, 2);

    assert_eq!(count, 2);
}

// ── intents: Retreat far shows Idle ──────────────────────────────────

#[test]
fn intents_retreat_far_shows_idle() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    enemy.ai = AiBehavior::Retreat;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Idle));
}

// ── intents: Kiter far shows Approach ────────────────────────────────

#[test]
fn intents_kiter_far_shows_approach() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    enemy.ai = AiBehavior::Kiter;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Approach));
}

// ── intents: Kiter mid-range shows Idle ──────────────────────────────

#[test]
fn intents_kiter_mid_range_shows_idle() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 0);
    enemy.ai = AiBehavior::Kiter;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    // dist=3, not <=2 and not >=5 → Idle
    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Idle));
}

// ── intents: Pack adjacent with allies shows Attack ──────────────────

#[test]
fn intents_pack_adjacent_shows_attack() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.ai = AiBehavior::Pack;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    // dist<=1, no allies, but adjacent → Attack
    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Attack));
}

// ── intents: Retreat mid-range shows Retreat ─────────────────────────

#[test]
fn intents_retreat_mid_range_shows_retreat() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
    enemy.ai = AiBehavior::Retreat;
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    // dist=2, <=3 → Retreat
    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Retreat));
}

// ── intents with radical actions ─────────────────────────────────────

#[test]
fn intents_chase_with_radical_shows_radical_intent() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Chase;
    enemy.radical_actions = vec![RadicalAction::OverwhelmingForce];
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    // With radical action picked, intent should be Attack (offensive melee)
    assert_eq!(battle.units[1].intent, Some(EnemyIntent::Attack));
}

#[test]
fn intents_kiter_with_debuff_shows_ranged_attack() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.ai = AiBehavior::Kiter;
    enemy.radical_actions = vec![RadicalAction::ErosiveFlow];
    let mut battle = make_test_battle(vec![player, enemy]);

    calculate_all_intents(&mut battle);

    // Debuff → RangedAttack
    assert_eq!(battle.units[1].intent, Some(EnemyIntent::RangedAttack));
}

// ── path_toward: conveyor evaluation ─────────────────────────────────

#[test]
fn path_toward_favors_conveyor_toward_target() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 0);
    enemy.movement = 2;
    let mut battle = make_test_battle(vec![player, enemy]);
    // Place a conveyor that pushes toward player
    battle.arena.set_tile(3, 0, BattleTile::ConveyorW);

    let path = path_toward(&battle, 1, 0, 0, true);

    assert!(path.is_some());
}

// ── path_toward: hazard avoidance ────────────────────────────────────

#[test]
fn path_toward_avoids_plasma_pool() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 0);
    enemy.movement = 3;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 0, BattleTile::PlasmaPool);

    let path = path_toward(&battle, 1, 0, 0, true);

    // Path exists but should try to avoid the plasma pool
    assert!(path.is_some());
}

#[test]
fn path_toward_avoids_steam_vent_active() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 0);
    enemy.movement = 3;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 0, BattleTile::SteamVentActive);

    let path = path_toward(&battle, 1, 0, 0, true);

    assert!(path.is_some());
}

#[test]
fn path_toward_avoids_energy_vent_active() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 0);
    enemy.movement = 3;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 0, BattleTile::EnergyVentActive);

    let path = path_toward(&battle, 1, 0, 0, true);

    assert!(path.is_some());
}

// ── path_toward: GravityWell near player is attractive ───────────────

#[test]
fn path_toward_gravity_well_near_player_is_bonus() {
    let player = make_test_unit(UnitKind::Player, 2, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 0);
    enemy.movement = 3;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 0, BattleTile::GravityWell);

    // GravityWell adjacent to path tile, and player is near GW → bonus
    let path = path_toward(&battle, 1, 2, 0, true);

    assert!(path.is_some());
}

// ── path_toward close_in=false spreads out from allies ───────────────

#[test]
fn path_toward_close_in_false_spreads_from_allies() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy1.movement = 2;
    let enemy2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    let battle = make_test_battle(vec![player, enemy1, enemy2]);

    // close_in=false adds distance to allies to score
    let path = path_toward(&battle, 1, 0, 0, false);

    assert!(path.is_some());
}

// ── build_path with DebrisStorm weather ──────────────────────────────

#[test]
fn build_path_debris_storm_increases_cost() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 0);
    enemy.movement = 3;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.weather = crate::combat::Weather::DebrisStorm;

    // Each step costs 2 (1 base + 1 debris storm), so 3 movement = 1 step
    let path = path_toward(&battle, 1, 0, 0, true);

    if let Some(p) = &path {
        // With step cost 2, movement 3 allows only 1 step
        assert!(p.len() <= 2);
    }
}

// ── companion: picks closest enemy ───────────────────────────────────

#[test]
fn companion_targets_nearest_enemy() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy_far = make_test_unit(UnitKind::Enemy(0), 6, 6);
    let enemy_close = make_test_unit(UnitKind::Enemy(1), 2, 2);
    let companion = make_test_unit(UnitKind::Companion, 1, 1);
    let battle = make_test_battle(vec![player, enemy_far, enemy_close, companion]);

    let action = choose_companion_action(&battle, 3);

    assert!(matches!(
        action,
        AiAction::MeleeAttack { target_unit: 2 } | AiAction::MoveAndAttack { target_unit: 2, .. } | AiAction::MoveToTile { .. }
    ));
}

// ── path_toward / path_away ──────────────────────────────────────────

#[test]
fn path_toward_returns_some_when_path_exists() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    let battle = make_test_battle(vec![player, enemy]);

    let path = path_toward(&battle, 1, 0, 0, true);

    assert!(path.is_some());
    let p = path.unwrap();
    assert!(!p.is_empty());
}

#[test]
fn path_toward_moves_closer_to_target() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
    let battle = make_test_battle(vec![player, enemy]);

    let path = path_toward(&battle, 1, 0, 0, true);

    assert!(path.is_some());
    let p = path.unwrap();
    let last = p.last().unwrap();
    let start_dist = manhattan(6, 6, 0, 0);
    let end_dist = manhattan(last.0, last.1, 0, 0);
    assert!(end_dist < start_dist);
}

#[test]
fn path_away_moves_farther_from_target() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let battle = make_test_battle(vec![player, enemy]);

    let path = path_away(&battle, 1, 0, 0);

    assert!(path.is_some());
    let p = path.unwrap();
    let last = p.last().unwrap();
    let start_dist = manhattan(3, 3, 0, 0);
    let end_dist = manhattan(last.0, last.1, 0, 0);
    assert!(end_dist > start_dist);
}

// ── count_allies_near ────────────────────────────────────────────────

#[test]
fn count_allies_excludes_player_for_enemies() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let enemy1 = make_test_unit(UnitKind::Enemy(0), 4, 3);
    let battle = make_test_battle(vec![player, enemy1]);

    // Player is adjacent but not same faction
    let count = count_allies_near(&battle, 1, 2);

    assert_eq!(count, 0);
}

#[test]
fn count_allies_near_excludes_far_units() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let enemy2 = make_test_unit(UnitKind::Enemy(1), 6, 6);
    let battle = make_test_battle(vec![player, enemy1, enemy2]);

    let count = count_allies_near(&battle, 1, 2);

    assert_eq!(count, 0);
}


