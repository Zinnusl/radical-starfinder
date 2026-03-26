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


