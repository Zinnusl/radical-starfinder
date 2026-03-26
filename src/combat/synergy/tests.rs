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

// ── apply_round_start_synergies (integration) ──────────────────────────

#[test]
fn apply_round_start_synergies_resets_synergy_state() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.synergy_damage_bonus = 5;
    enemy.elemental_resonance = true;

    let mut battle = make_test_battle(vec![player, enemy]);
    battle.attacks_on_player_this_round = 3;

    apply_round_start_synergies(&mut battle);

    assert_eq!(battle.attacks_on_player_this_round, 0);
    assert_eq!(battle.units[1].synergy_damage_bonus, 0);
    assert!(!battle.units[1].elemental_resonance);
}

#[test]
fn apply_round_start_synergies_returns_messages() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut s1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    s1.ai = AiBehavior::Sentinel;
    let mut s2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    s2.ai = AiBehavior::Sentinel;

    let mut battle = make_test_battle(vec![player, s1, s2]);

    let msgs = apply_round_start_synergies(&mut battle);
    assert!(!msgs.is_empty());
}

// ── apply_sentinel_formation ────────────────────────────────────────────

#[test]
fn sentinel_formation_two_adjacent_sentinels_gain_armor() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut s1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    s1.ai = AiBehavior::Sentinel;
    let mut s2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    s2.ai = AiBehavior::Sentinel;

    let mut battle = make_test_battle(vec![player, s1, s2]);

    let msgs = apply_round_start_synergies(&mut battle);

    // Both sentinels get +1 formation armor
    assert!(battle.units[1].radical_armor >= 1);
    assert!(battle.units[2].radical_armor >= 1);
    assert!(msgs.iter().any(|m| m.contains("Shield formation")));
}

#[test]
fn sentinel_shares_armor_with_adjacent_non_sentinel_ally() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut sentinel = make_test_unit(UnitKind::Enemy(0), 3, 3);
    sentinel.ai = AiBehavior::Sentinel;
    let ally = make_test_unit(UnitKind::Enemy(1), 4, 3); // adjacent, non-sentinel

    let mut battle = make_test_battle(vec![player, sentinel, ally]);

    let msgs = apply_round_start_synergies(&mut battle);

    // Ally gets +1 shared armor from adjacent sentinel
    assert_eq!(battle.units[2].radical_armor, 1);
    assert!(msgs.iter().any(|m| m.contains("Sentinels share armor")));
}

#[test]
fn sentinel_formation_no_bonus_for_lone_sentinel() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut sentinel = make_test_unit(UnitKind::Enemy(0), 3, 3);
    sentinel.ai = AiBehavior::Sentinel;
    // No adjacent sentinel, no adjacent ally

    let mut battle = make_test_battle(vec![player, sentinel]);

    let msgs = apply_round_start_synergies(&mut battle);

    // No formation bonus
    assert_eq!(battle.units[1].radical_armor, 0);
    assert!(!msgs.iter().any(|m| m.contains("Shield formation")));
}

#[test]
fn sentinel_formation_does_not_share_armor_with_player() {
    let player = make_test_unit(UnitKind::Player, 4, 3); // adjacent to sentinel
    let mut sentinel = make_test_unit(UnitKind::Enemy(0), 3, 3);
    sentinel.ai = AiBehavior::Sentinel;

    let mut battle = make_test_battle(vec![player, sentinel]);

    apply_round_start_synergies(&mut battle);

    // Player should NOT get armor from enemy sentinel
    assert_eq!(battle.units[0].radical_armor, 0);
}

// ── apply_elemental_resonance ───────────────────────────────────────────

#[test]
fn fire_resonance_boosts_burn_damage() {
    use crate::status::{StatusInstance, StatusKind};

    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut fire1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    fire1.wuxing_element = Some(WuxingElement::Fire);
    fire1.statuses.push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 3));
    let mut fire2 = make_test_unit(UnitKind::Enemy(1), 4, 3); // within 2 tiles
    fire2.wuxing_element = Some(WuxingElement::Fire);

    let mut battle = make_test_battle(vec![player, fire1, fire2]);

    let msgs = apply_round_start_synergies(&mut battle);

    // Fire resonance should boost burn damage +1
    let burn_status = battle.units[1].statuses.iter().find(|s| matches!(s.kind, StatusKind::Burn { .. }));
    assert!(burn_status.is_some());
    if let Some(s) = burn_status {
        if let StatusKind::Burn { damage } = s.kind {
            assert_eq!(damage, 2); // 1 + 1 from resonance
        }
    }
    assert!(battle.units[1].elemental_resonance);
    assert!(msgs.iter().any(|m| m.contains("Plasma resonance")));
}

#[test]
fn water_resonance_extends_player_slow() {
    use crate::status::{StatusInstance, StatusKind};

    let mut player = make_test_unit(UnitKind::Player, 0, 0);
    player.statuses.push(StatusInstance::new(StatusKind::Slow, 2));
    let mut water1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    water1.wuxing_element = Some(WuxingElement::Water);
    let mut water2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    water2.wuxing_element = Some(WuxingElement::Water);

    let mut battle = make_test_battle(vec![player, water1, water2]);

    let msgs = apply_round_start_synergies(&mut battle);

    // Player's slow should be extended by 1 turn
    let slow = battle.units[0].statuses.iter().find(|s| matches!(s.kind, StatusKind::Slow));
    assert!(slow.is_some());
    assert_eq!(slow.unwrap().turns_left, 3); // 2 + 1
    assert!(msgs.iter().any(|m| m.contains("Coolant resonance")));
}

#[test]
fn earth_resonance_grants_armor() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut earth1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    earth1.wuxing_element = Some(WuxingElement::Earth);
    let mut earth2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    earth2.wuxing_element = Some(WuxingElement::Earth);

    let mut battle = make_test_battle(vec![player, earth1, earth2]);

    let msgs = apply_round_start_synergies(&mut battle);

    assert_eq!(battle.units[1].radical_armor, 1);
    assert_eq!(battle.units[2].radical_armor, 1);
    assert!(battle.units[1].elemental_resonance);
    assert!(msgs.iter().any(|m| m.contains("Hull resonance")));
}

#[test]
fn metal_resonance_grants_damage_bonus() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut metal1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    metal1.wuxing_element = Some(WuxingElement::Metal);
    let mut metal2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    metal2.wuxing_element = Some(WuxingElement::Metal);

    let mut battle = make_test_battle(vec![player, metal1, metal2]);

    let msgs = apply_round_start_synergies(&mut battle);

    assert_eq!(battle.units[1].synergy_damage_bonus, 1);
    assert_eq!(battle.units[2].synergy_damage_bonus, 1);
    assert!(battle.units[1].elemental_resonance);
    assert!(msgs.iter().any(|m| m.contains("Metal resonance")));
}

#[test]
fn wood_resonance_heals_enemies() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut wood1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    wood1.wuxing_element = Some(WuxingElement::Wood);
    wood1.hp = 8;
    let mut wood2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    wood2.wuxing_element = Some(WuxingElement::Wood);
    wood2.hp = 10; // already at max

    let mut battle = make_test_battle(vec![player, wood1, wood2]);

    let msgs = apply_round_start_synergies(&mut battle);

    assert_eq!(battle.units[1].hp, 9); // 8 + 1 healed
    assert_eq!(battle.units[2].hp, 10); // capped at max_hp
    assert!(battle.units[1].elemental_resonance);
    assert!(msgs.iter().any(|m| m.contains("Bio resonance")));
}

#[test]
fn elemental_resonance_requires_two_within_distance() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut fire1 = make_test_unit(UnitKind::Enemy(0), 1, 1);
    fire1.wuxing_element = Some(WuxingElement::Fire);
    let mut fire2 = make_test_unit(UnitKind::Enemy(1), 5, 5); // manhattan distance = 8, too far
    fire2.wuxing_element = Some(WuxingElement::Fire);

    let mut battle = make_test_battle(vec![player, fire1, fire2]);

    let msgs = apply_round_start_synergies(&mut battle);

    assert!(!battle.units[1].elemental_resonance);
    assert!(!battle.units[2].elemental_resonance);
    assert!(!msgs.iter().any(|m| m.contains("Plasma resonance")));
}

#[test]
fn elemental_resonance_ignores_dead_enemies() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut fire1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    fire1.wuxing_element = Some(WuxingElement::Fire);
    let mut fire2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    fire2.wuxing_element = Some(WuxingElement::Fire);
    fire2.alive = false;
    fire2.hp = 0;

    let mut battle = make_test_battle(vec![player, fire1, fire2]);

    let msgs = apply_round_start_synergies(&mut battle);

    assert!(!battle.units[1].elemental_resonance);
    assert!(!msgs.iter().any(|m| m.contains("resonance")));
}

// ── apply_elemental_clash ───────────────────────────────────────────────

#[test]
fn fire_water_clash_creates_steam() {
    use crate::combat::BattleTile;

    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut fire = make_test_unit(UnitKind::Enemy(0), 3, 3);
    fire.wuxing_element = Some(WuxingElement::Fire);
    let mut water = make_test_unit(UnitKind::Enemy(1), 4, 3); // adjacent
    water.wuxing_element = Some(WuxingElement::Water);

    let mut battle = make_test_battle(vec![player, fire, water]);

    let msgs = apply_round_start_synergies(&mut battle);

    // Steam tile should be placed at the fire unit's position
    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::VentSteam));
    assert!(msgs.iter().any(|m| m.contains("steam")));
}

#[test]
fn fire_wood_clash_damages_wood_enemy() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut fire = make_test_unit(UnitKind::Enemy(0), 3, 3);
    fire.wuxing_element = Some(WuxingElement::Fire);
    let mut wood = make_test_unit(UnitKind::Enemy(1), 4, 3);
    wood.wuxing_element = Some(WuxingElement::Wood);
    wood.hp = 5;

    let mut battle = make_test_battle(vec![player, fire, wood]);

    let msgs = apply_round_start_synergies(&mut battle);

    assert_eq!(battle.units[2].hp, 4); // 5 - 1 burn damage
    assert!(msgs.iter().any(|m| m.contains("scorches")));
}

#[test]
fn fire_wood_clash_kills_at_one_hp() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut fire = make_test_unit(UnitKind::Enemy(0), 3, 3);
    fire.wuxing_element = Some(WuxingElement::Fire);
    let mut wood = make_test_unit(UnitKind::Enemy(1), 4, 3);
    wood.wuxing_element = Some(WuxingElement::Wood);
    wood.hp = 1;

    let mut battle = make_test_battle(vec![player, fire, wood]);

    apply_round_start_synergies(&mut battle);

    assert_eq!(battle.units[2].hp, 0);
    assert!(!battle.units[2].alive);
}

#[test]
fn water_earth_clash_creates_damaged_plating() {
    use crate::combat::BattleTile;

    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut water = make_test_unit(UnitKind::Enemy(0), 3, 3);
    water.wuxing_element = Some(WuxingElement::Water);
    let mut earth = make_test_unit(UnitKind::Enemy(1), 4, 3);
    earth.wuxing_element = Some(WuxingElement::Earth);

    let mut battle = make_test_battle(vec![player, water, earth]);
    // Ensure the water position is MetalFloor (default)
    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::MetalFloor));

    let msgs = apply_round_start_synergies(&mut battle);

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::DamagedPlating));
    assert!(msgs.iter().any(|m| m.contains("sludge")));
}

#[test]
fn water_earth_clash_skips_non_metal_floor() {
    use crate::combat::BattleTile;

    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut water = make_test_unit(UnitKind::Enemy(0), 3, 3);
    water.wuxing_element = Some(WuxingElement::Water);
    let mut earth = make_test_unit(UnitKind::Enemy(1), 4, 3);
    earth.wuxing_element = Some(WuxingElement::Earth);

    let mut battle = make_test_battle(vec![player, water, earth]);
    battle.arena.set_tile(3, 3, BattleTile::OilSlick);

    let msgs = apply_round_start_synergies(&mut battle);

    // Should not convert non-MetalFloor tiles
    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::OilSlick));
    assert!(!msgs.iter().any(|m| m.contains("sludge")));
}

#[test]
fn elemental_clash_non_adjacent_no_effect() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut fire = make_test_unit(UnitKind::Enemy(0), 1, 1);
    fire.wuxing_element = Some(WuxingElement::Fire);
    let mut water = make_test_unit(UnitKind::Enemy(1), 5, 5); // far away
    water.wuxing_element = Some(WuxingElement::Water);

    let mut battle = make_test_battle(vec![player, fire, water]);

    let msgs = apply_round_start_synergies(&mut battle);

    assert!(!msgs.iter().any(|m| m.contains("steam") || m.contains("scorches") || m.contains("sludge")));
}

// ── apply_leader_aura ──────────────────────────────────────────────────

#[test]
fn boss_aura_grants_damage_bonus_to_non_boss_enemies() {
    use crate::enemy::BossKind;

    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut boss = make_test_unit(UnitKind::Enemy(0), 3, 3);
    boss.boss_kind = Some(BossKind::PirateCaptain);
    let minion = make_test_unit(UnitKind::Enemy(1), 5, 5);

    let mut battle = make_test_battle(vec![player, boss, minion]);
    battle.is_boss_battle = true;

    let msgs = apply_round_start_synergies(&mut battle);

    // Minion gets +1 damage from boss aura
    assert_eq!(battle.units[2].synergy_damage_bonus, 1);
    // Boss itself should NOT get the bonus (boss_kind.is_none() check)
    assert_eq!(battle.units[1].synergy_damage_bonus, 0);
    assert!(msgs.iter().any(|m| m.contains("Boss presence")));
}

#[test]
fn boss_aura_no_effect_if_not_boss_battle() {
    use crate::enemy::BossKind;

    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut boss = make_test_unit(UnitKind::Enemy(0), 3, 3);
    boss.boss_kind = Some(BossKind::PirateCaptain);
    let minion = make_test_unit(UnitKind::Enemy(1), 5, 5);

    let mut battle = make_test_battle(vec![player, boss, minion]);
    battle.is_boss_battle = false;

    let msgs = apply_round_start_synergies(&mut battle);

    assert_eq!(battle.units[2].synergy_damage_bonus, 0);
    assert!(!msgs.iter().any(|m| m.contains("Boss presence")));
}

#[test]
fn boss_aura_no_effect_if_boss_dead() {
    use crate::enemy::BossKind;

    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut boss = make_test_unit(UnitKind::Enemy(0), 3, 3);
    boss.boss_kind = Some(BossKind::PirateCaptain);
    boss.alive = false;
    boss.hp = 0;
    let minion = make_test_unit(UnitKind::Enemy(1), 5, 5);

    let mut battle = make_test_battle(vec![player, boss, minion]);
    battle.is_boss_battle = true;

    let msgs = apply_round_start_synergies(&mut battle);

    assert_eq!(battle.units[2].synergy_damage_bonus, 0);
    assert!(!msgs.iter().any(|m| m.contains("Boss presence")));
}

#[test]
fn elite_aura_boosts_nearby_non_elite_speed() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut elite = make_test_unit(UnitKind::Enemy(0), 3, 3);
    elite.word_group = Some(0); // marks as elite
    let nearby = make_test_unit(UnitKind::Enemy(1), 4, 3); // within 3 tiles
    let initial_speed = nearby.speed;
    let far = make_test_unit(UnitKind::Enemy(2), 0, 6); // too far (manhattan 9)

    let mut battle = make_test_battle(vec![player, elite, nearby, far]);

    let msgs = apply_round_start_synergies(&mut battle);

    assert_eq!(battle.units[2].speed, initial_speed + 1);
    // Far unit should not be boosted
    assert_eq!(battle.units[3].speed, 4); // unchanged
    assert!(msgs.iter().any(|m| m.contains("Commander's aura")));
}

#[test]
fn elite_aura_does_not_boost_other_elites() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut elite1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    elite1.word_group = Some(0);
    let mut elite2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    elite2.word_group = Some(1);

    let mut battle = make_test_battle(vec![player, elite1, elite2]);

    apply_round_start_synergies(&mut battle);

    // Elites should NOT boost each other (word_group.is_some() skips them)
    assert_eq!(battle.units[1].speed, 4);
    assert_eq!(battle.units[2].speed, 4);
}

// ── tick_sacrifice_bonuses ──────────────────────────────────────────────

#[test]
fn tick_sacrifice_bonuses_decrements_turns() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.sacrifice_bonus_damage = 2;
    enemy.sacrifice_bonus_turns = 2;

    let mut battle = make_test_battle(vec![player, enemy]);

    apply_round_start_synergies(&mut battle);

    assert_eq!(battle.units[1].sacrifice_bonus_turns, 1);
    assert_eq!(battle.units[1].sacrifice_bonus_damage, 2); // still active
}

#[test]
fn tick_sacrifice_bonuses_clears_on_expiry() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.sacrifice_bonus_damage = 2;
    enemy.sacrifice_bonus_turns = 1;

    let mut battle = make_test_battle(vec![player, enemy]);

    apply_round_start_synergies(&mut battle);

    assert_eq!(battle.units[1].sacrifice_bonus_turns, 0);
    assert_eq!(battle.units[1].sacrifice_bonus_damage, 0); // cleared
}

#[test]
fn tick_sacrifice_bonuses_no_effect_when_zero() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.sacrifice_bonus_damage = 0;
    enemy.sacrifice_bonus_turns = 0;

    let mut battle = make_test_battle(vec![player, enemy]);

    apply_round_start_synergies(&mut battle);

    assert_eq!(battle.units[1].sacrifice_bonus_turns, 0);
    assert_eq!(battle.units[1].sacrifice_bonus_damage, 0);
}

