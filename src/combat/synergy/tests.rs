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

// ── pack_tactics_bonus ──────────────────────────────────────────────────

#[test]
fn pack_tactics_zero_when_no_adjacent_allies() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut lone_pack = make_test_unit(UnitKind::Enemy(0), 3, 3);
    lone_pack.ai = AiBehavior::Pack;

    let battle = make_test_battle(vec![player, lone_pack]);

    assert_eq!(pack_tactics_bonus(&battle, 1), 0);
}

#[test]
fn pack_tactics_ignores_diagonal_allies() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut pack_enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    pack_enemy.ai = AiBehavior::Pack;
    let diagonal_ally = make_test_unit(UnitKind::Enemy(1), 4, 4);

    let battle = make_test_battle(vec![player, pack_enemy, diagonal_ally]);

    assert_eq!(pack_tactics_bonus(&battle, 1), 0);
}

#[test]
fn pack_tactics_ignores_adjacent_player() {
    let player = make_test_unit(UnitKind::Player, 3, 2);
    let mut pack_enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    pack_enemy.ai = AiBehavior::Pack;

    let battle = make_test_battle(vec![player, pack_enemy]);

    assert_eq!(pack_tactics_bonus(&battle, 1), 0);
}

#[test]
fn pack_tactics_counts_up_to_four_cardinal_allies() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut pack_enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    pack_enemy.ai = AiBehavior::Pack;
    let ally_n = make_test_unit(UnitKind::Enemy(1), 3, 2);
    let ally_s = make_test_unit(UnitKind::Enemy(2), 3, 4);
    let ally_e = make_test_unit(UnitKind::Enemy(3), 4, 3);
    let ally_w = make_test_unit(UnitKind::Enemy(4), 2, 3);

    let battle = make_test_battle(vec![player, pack_enemy, ally_n, ally_s, ally_e, ally_w]);

    assert_eq!(pack_tactics_bonus(&battle, 1), 4);
}

#[test]
fn pack_tactics_ignores_dead_allies() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut pack_enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    pack_enemy.ai = AiBehavior::Pack;
    let mut dead_ally = make_test_unit(UnitKind::Enemy(1), 4, 3);
    dead_ally.alive = false;
    dead_ally.hp = 0;

    let battle = make_test_battle(vec![player, pack_enemy, dead_ally]);

    assert_eq!(pack_tactics_bonus(&battle, 1), 0);
}

// ── coordinated_attack_bonus ────────────────────────────────────────────

#[test]
fn coordinated_attack_bonus_one_regardless_of_attack_count() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 2, 0);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.attacks_on_player_this_round = 5;

    assert_eq!(coordinated_attack_bonus(&battle), 1);
}

// ── total_attack_synergy_bonus ──────────────────────────────────────────

#[test]
fn total_attack_synergy_bonus_zero_with_no_bonuses() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);

    let battle = make_test_battle(vec![player, enemy]);

    let (bonus, msgs) = total_attack_synergy_bonus(&battle, 1);
    assert_eq!(bonus, 0);
    assert!(msgs.is_empty());
}

#[test]
fn total_attack_synergy_bonus_includes_sacrifice_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.sacrifice_bonus_damage = 2;

    let battle = make_test_battle(vec![player, enemy]);

    let (bonus, _) = total_attack_synergy_bonus(&battle, 1);
    assert_eq!(bonus, 2);
}

#[test]
fn total_attack_synergy_bonus_includes_synergy_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.synergy_damage_bonus = 3;

    let battle = make_test_battle(vec![player, enemy]);

    let (bonus, _) = total_attack_synergy_bonus(&battle, 1);
    assert_eq!(bonus, 3);
}

#[test]
fn total_attack_synergy_bonus_generates_pack_message() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut pack_enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    pack_enemy.ai = AiBehavior::Pack;
    let ally = make_test_unit(UnitKind::Enemy(1), 4, 3);

    let battle = make_test_battle(vec![player, pack_enemy, ally]);

    let (_, msgs) = total_attack_synergy_bonus(&battle, 1);
    assert!(msgs.iter().any(|m| m.contains("Pack tactics")));
}

#[test]
fn total_attack_synergy_bonus_generates_coordinated_message() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.attacks_on_player_this_round = 2;

    let (_, msgs) = total_attack_synergy_bonus(&battle, 1);
    assert!(msgs.iter().any(|m| m.contains("Coordinated")));
}

// ── try_sacrifice ───────────────────────────────────────────────────────

#[test]
fn try_sacrifice_fails_when_hp_above_25_percent() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 3; // 30% of 10 max_hp
    let ally = make_test_unit(UnitKind::Enemy(1), 4, 3);

    let mut battle = make_test_battle(vec![player, enemy, ally]);

    assert!(!try_sacrifice(&mut battle, 1));
    assert!(battle.units[1].alive);
}

#[test]
fn try_sacrifice_fails_when_no_adjacent_ally() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 1; // 10% of max_hp

    let mut battle = make_test_battle(vec![player, enemy]);

    assert!(!try_sacrifice(&mut battle, 1));
    assert!(battle.units[1].alive);
}

#[test]
fn try_sacrifice_fails_for_player_unit() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.hp = 1;
    let ally = make_test_unit(UnitKind::Enemy(0), 4, 3);

    let mut battle = make_test_battle(vec![player, ally]);

    assert!(!try_sacrifice(&mut battle, 0));
    assert!(battle.units[0].alive);
}

#[test]
fn try_sacrifice_heals_ally_and_grants_bonus_when_seed_aligns() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut sacrificer = make_test_unit(UnitKind::Enemy(0), 3, 3);
    sacrificer.hp = 2;
    sacrificer.max_hp = 10;
    let mut ally = make_test_unit(UnitKind::Enemy(1), 4, 3);
    ally.hp = 5;
    ally.max_hp = 10;

    // Brute-force a turn_number where seed % 5 == 0
    // seed = turn_number * 41 + unit_idx * 23; unit_idx = 1 => seed = turn*41 + 23
    // We need (turn*41 + 23) % 5 == 0
    // turn=2: 82+23 = 105, 105 % 5 = 0 ✓
    let mut battle = make_test_battle(vec![player, sacrificer, ally]);
    battle.turn_number = 2;

    let result = try_sacrifice(&mut battle, 1);

    assert!(result);
    assert!(!battle.units[1].alive);
    assert_eq!(battle.units[1].hp, 0);
    assert_eq!(battle.units[2].hp, 7); // 5 + 2 remaining hp
    assert_eq!(battle.units[2].sacrifice_bonus_damage, 2);
    assert_eq!(battle.units[2].sacrifice_bonus_turns, 2);
}

#[test]
fn try_sacrifice_does_not_overheal_ally_past_max_hp() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut sacrificer = make_test_unit(UnitKind::Enemy(0), 3, 3);
    sacrificer.hp = 2;
    sacrificer.max_hp = 10;
    let mut ally = make_test_unit(UnitKind::Enemy(1), 4, 3);
    ally.hp = 9;
    ally.max_hp = 10;

    let mut battle = make_test_battle(vec![player, sacrificer, ally]);
    battle.turn_number = 2; // seed = 2*41+23 = 105, 105%5 = 0

    try_sacrifice(&mut battle, 1);

    assert_eq!(battle.units[2].hp, 10); // capped at max_hp
}

#[test]
fn try_sacrifice_fails_when_seed_does_not_align() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut sacrificer = make_test_unit(UnitKind::Enemy(0), 3, 3);
    sacrificer.hp = 1;
    let ally = make_test_unit(UnitKind::Enemy(1), 4, 3);

    // turn=1: seed = 1*41 + 23 = 64, 64 % 5 = 4 ≠ 0
    let mut battle = make_test_battle(vec![player, sacrificer, ally]);
    battle.turn_number = 1;

    assert!(!try_sacrifice(&mut battle, 1));
    assert!(battle.units[1].alive);
}

// ── on_enemy_death_revenge ──────────────────────────────────────────────

#[test]
fn on_enemy_death_revenge_ignores_dead_adjacent_enemies() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut dead_enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    dead_enemy.alive = false;
    dead_enemy.hp = 0;
    let mut also_dead = make_test_unit(UnitKind::Enemy(1), 4, 3);
    also_dead.alive = false;
    also_dead.hp = 0;

    let mut battle = make_test_battle(vec![player, dead_enemy, also_dead]);

    on_enemy_death_revenge(&mut battle, 1);

    assert_eq!(battle.units[2].fortify_stacks, 0);
}

#[test]
fn on_enemy_death_revenge_stacks_with_existing_fortify() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut dead_enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    dead_enemy.alive = false;
    let mut adjacent = make_test_unit(UnitKind::Enemy(1), 4, 3);
    adjacent.fortify_stacks = 2;

    let mut battle = make_test_battle(vec![player, dead_enemy, adjacent]);

    on_enemy_death_revenge(&mut battle, 1);

    assert_eq!(battle.units[2].fortify_stacks, 3);
}

#[test]
fn on_enemy_death_revenge_enrages_multiple_adjacent_enemies() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut dead_enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    dead_enemy.alive = false;
    let adj_north = make_test_unit(UnitKind::Enemy(1), 3, 2);
    let adj_south = make_test_unit(UnitKind::Enemy(2), 3, 4);
    let adj_east = make_test_unit(UnitKind::Enemy(3), 4, 3);

    let mut battle = make_test_battle(vec![player, dead_enemy, adj_north, adj_south, adj_east]);

    on_enemy_death_revenge(&mut battle, 1);

    assert_eq!(battle.units[2].fortify_stacks, 1);
    assert_eq!(battle.units[3].fortify_stacks, 1);
    assert_eq!(battle.units[4].fortify_stacks, 1);
}

