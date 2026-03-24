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

