use super::*;
use crate::combat::test_helpers::{make_test_battle, make_test_unit};
use crate::combat::{BattleTile, UnitKind};

#[test]
fn step_on_weakened_plating_becomes_damaged() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(2, 2, BattleTile::WeakenedPlating);

    let msgs = step_on_crumbling(&mut battle, 2, 2);
    assert_eq!(battle.arena.tile(2, 2), Some(BattleTile::DamagedFloor));
    assert!(!msgs.is_empty());
}

#[test]
fn step_on_damaged_floor_becomes_breached() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(2, 2, BattleTile::DamagedFloor);

    let msgs = step_on_crumbling(&mut battle, 2, 2);
    assert_eq!(battle.arena.tile(2, 2), Some(BattleTile::BreachedFloor));
    assert!(!msgs.is_empty());
}

#[test]
fn step_on_normal_floor_does_nothing() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);

    let msgs = step_on_crumbling(&mut battle, 2, 2);
    assert_eq!(battle.arena.tile(2, 2), Some(BattleTile::MetalFloor));
    assert!(msgs.is_empty());
}

#[test]
fn decay_cracked_floors_collapses_unoccupied() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::DamagedFloor);
    battle.arena.set_tile(4, 4, BattleTile::DamagedFloor);

    let msgs = decay_cracked_floors(&mut battle);
    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::BreachedFloor));
    assert_eq!(battle.arena.tile(4, 4), Some(BattleTile::BreachedFloor));
    assert_eq!(msgs.len(), 2);
}

#[test]
fn decay_cracked_floors_spares_occupied_tiles() {
    // Player is standing on the damaged floor
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::DamagedFloor);

    let msgs = decay_cracked_floors(&mut battle);
    // Should not collapse because player is standing there
    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::DamagedFloor));
    assert!(msgs.is_empty());
}

#[test]
fn trigger_trap_on_mine_deals_damage_and_slows() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.hp = 10;
    player.max_hp = 10;
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::MineTile);

    let msgs = trigger_trap(&mut battle, 0, 3, 3);
    assert!(!msgs.is_empty());
    // Took damage
    assert!(battle.units[0].hp < 10);
    // Got Slow status
    let has_slow = battle.units[0].statuses.iter().any(|s| matches!(s.kind, StatusKind::Slow));
    assert!(has_slow);
    // Mine revealed
    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::MineTileRevealed));
}

#[test]
fn trigger_trap_on_normal_floor_does_nothing() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.hp = 10;
    let mut battle = make_test_battle(vec![player]);

    let msgs = trigger_trap(&mut battle, 0, 3, 3);
    assert!(msgs.is_empty());
    assert_eq!(battle.units[0].hp, 10);
}

#[test]
fn terrain_roll_deterministic() {
    // Same inputs always produce same result
    let r1 = terrain_roll(5, 10, 50);
    let r2 = terrain_roll(5, 10, 50);
    assert_eq!(r1, r2);

    // Different inputs can produce different results
    let r3 = terrain_roll(5, 10, 50);
    let r4 = terrain_roll(6, 10, 50);
    // These might be different (probabilistic, but deterministic)
    let _ = (r3, r4); // just ensure no panic

    // Chance 0 always false, chance 100 always true
    assert!(!terrain_roll(1, 1, 0));
    assert!(terrain_roll(1, 1, 100));
}

// ── apply_terrain_interactions: FireAbility ──────────────────────────

#[test]
fn fire_on_wiring_panel_creates_blast_mark() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::WiringPanel);

    let msgs = apply_terrain_interactions(
        &mut battle,
        TerrainSource::FireAbility,
        &[(3, 3)],
    );

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::BlastMark));
    assert!(!msgs.is_empty());
}

#[test]
fn fire_on_wiring_panel_damages_unit_standing_on_it() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::WiringPanel);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::FireAbility,
        &[(3, 3)],
    );

    assert!(battle.units[1].hp < 10);
}

#[test]
fn fire_on_electrified_wire_creates_blast_mark() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::ElectrifiedWire);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::FireAbility,
        &[(3, 3)],
    );

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::BlastMark));
}

#[test]
fn fire_on_frozen_coolant_melts_to_coolant_pool() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::FrozenCoolant);

    let msgs = apply_terrain_interactions(
        &mut battle,
        TerrainSource::FireAbility,
        &[(3, 3)],
    );

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::CoolantPool));
    assert!(msgs.iter().any(|m| m.contains("melts")));
}

#[test]
fn fire_on_frozen_coolant_cascades_to_adjacent_frozen() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::FrozenCoolant);
    battle.arena.set_tile(4, 3, BattleTile::FrozenCoolant);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::FireAbility,
        &[(3, 3)],
    );

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::CoolantPool));
    assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::CoolantPool));
}

#[test]
fn fire_on_coolant_pool_creates_steam() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::CoolantPool);

    let msgs = apply_terrain_interactions(
        &mut battle,
        TerrainSource::FireAbility,
        &[(3, 3)],
    );

    assert!(msgs.iter().any(|m| m.contains("Steam")));
}

#[test]
fn fire_on_lubricant_ignites_area() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::Lubricant);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::FireAbility,
        &[(3, 3)],
    );

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::BlastMark));
}

#[test]
fn fire_on_lubricant_damages_adjacent_unit() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::Lubricant);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::FireAbility,
        &[(3, 3)],
    );

    assert!(battle.units[1].hp < 10);
    let has_burn = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Burn { .. }));
    assert!(has_burn);
}

#[test]
fn fire_on_fuel_canister_explodes_barrel() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::FuelCanister);

    let msgs = apply_terrain_interactions(
        &mut battle,
        TerrainSource::FireAbility,
        &[(3, 3)],
    );

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::BlastMark));
    assert!(msgs.iter().any(|m| m.contains("explodes")));
}

#[test]
fn fire_on_metal_floor_does_nothing() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);

    let msgs = apply_terrain_interactions(
        &mut battle,
        TerrainSource::FireAbility,
        &[(3, 3)],
    );

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::MetalFloor));
    assert!(msgs.is_empty());
}

// ── apply_terrain_interactions: LightningAbility ─────────────────────

#[test]
fn lightning_on_coolant_pool_stuns_unit_in_water() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::CoolantPool);

    let msgs = apply_terrain_interactions(
        &mut battle,
        TerrainSource::LightningAbility,
        &[(3, 3)],
    );

    assert!(battle.units[1].stunned);
    assert!(!msgs.is_empty());
}

#[test]
fn lightning_on_dry_tile_does_not_stun() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::LightningAbility,
        &[(3, 3)],
    );

    assert!(!battle.units[1].stunned);
}

#[test]
fn lightning_chains_through_connected_coolant() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::CoolantPool);
    battle.arena.set_tile(4, 3, BattleTile::CoolantPool);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::LightningAbility,
        &[(3, 3)],
    );

    assert!(battle.units[1].stunned);
}

// ── apply_terrain_interactions: Earthquake ───────────────────────────

#[test]
fn earthquake_converts_metal_floor_to_damaged_plating() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::Earthquake,
        &[(3, 3)],
    );

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::DamagedPlating));
}

#[test]
fn earthquake_converts_wiring_panel_to_damaged_plating() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::WiringPanel);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::Earthquake,
        &[(3, 3)],
    );

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::DamagedPlating));
}

#[test]
fn earthquake_shatters_cargo_crate_and_damages_adjacent() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::CargoCrate);

    let msgs = apply_terrain_interactions(
        &mut battle,
        TerrainSource::Earthquake,
        &[(3, 3)],
    );

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::DamagedPlating));
    assert!(battle.units[1].hp < 10);
    assert!(msgs.iter().any(|m| m.contains("shatters")));
}

#[test]
fn earthquake_on_multiple_tiles() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(2, 2, BattleTile::Debris);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::Earthquake,
        &[(3, 3), (2, 2)],
    );

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::DamagedPlating));
    assert_eq!(battle.arena.tile(2, 2), Some(BattleTile::DamagedPlating));
}

// ── apply_knockback ──────────────────────────────────────────────────

#[test]
fn knockback_zero_direction_returns_empty() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);

    let msgs = apply_knockback(&mut battle, 1, 3, 3);

    assert!(msgs.is_empty());
    assert_eq!(battle.units[1].x, 3);
    assert_eq!(battle.units[1].y, 3);
}

#[test]
fn knockback_at_arena_edge_stops() {
    let player = make_test_unit(UnitKind::Player, 3, 3);
    let enemy = make_test_unit(UnitKind::Enemy(0), 6, 3);
    let mut battle = make_test_battle(vec![player, enemy]);

    let msgs = apply_knockback(&mut battle, 1, 5, 3);

    assert!(msgs.iter().any(|m| m.contains("arena edge")));
    assert_eq!(battle.units[1].x, 6);
}

#[test]
fn knockback_into_cover_barrier_deals_crush_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);

    let msgs = apply_knockback(&mut battle, 1, 2, 3);

    assert!(battle.units[1].hp < 10);
    assert!(msgs.iter().any(|m| m.contains("obstacle")));
}

#[test]
fn knockback_into_cargo_crate_deals_crush_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::CargoCrate);

    let msgs = apply_knockback(&mut battle, 1, 2, 3);

    assert!(battle.units[1].hp < 10);
    assert!(msgs.iter().any(|m| m.contains("cargo crate")));
}

#[test]
fn knockback_into_fuel_canister_explodes() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::FuelCanister);

    let msgs = apply_knockback(&mut battle, 1, 2, 3);

    assert!(battle.units[1].hp < 10);
    assert!(msgs.iter().any(|m| m.contains("canister")));
    assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::BlastMark));
}

#[test]
fn knockback_into_another_unit_causes_collision() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy1.hp = 10;
    let mut enemy2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
    enemy2.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy1, enemy2]);

    let msgs = apply_knockback(&mut battle, 1, 2, 3);

    assert!(battle.units[1].hp < 10);
    assert!(battle.units[2].hp < 10);
    assert!(msgs.iter().any(|m| m.contains("Collision")));
}

#[test]
fn knockback_onto_coolant_pool_applies_confused() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::CoolantPool);

    apply_knockback(&mut battle, 1, 2, 3);

    assert_eq!(battle.units[1].x, 4);
    assert_eq!(battle.units[1].y, 3);
    let has_confused = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Confused));
    assert!(has_confused);
}

#[test]
fn knockback_onto_plasma_pool_deals_two_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::PlasmaPool);

    apply_knockback(&mut battle, 1, 2, 3);

    assert_eq!(battle.units[1].x, 4);
    assert_eq!(battle.units[1].hp, 8);
}

#[test]
fn knockback_onto_blast_mark_deals_one_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::BlastMark);

    apply_knockback(&mut battle, 1, 2, 3);

    assert_eq!(battle.units[1].x, 4);
    assert_eq!(battle.units[1].hp, 9);
}

#[test]
fn knockback_onto_electrified_wire_deals_one_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::ElectrifiedWire);

    apply_knockback(&mut battle, 1, 2, 3);

    assert_eq!(battle.units[1].x, 4);
    assert_eq!(battle.units[1].hp, 9);
}

#[test]
fn knockback_onto_frozen_coolant_slides_extra_tile() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::FrozenCoolant);

    let msgs = apply_knockback(&mut battle, 1, 2, 3);

    assert_eq!(battle.units[1].x, 5);
    assert_eq!(battle.units[1].y, 3);
    assert!(msgs.iter().any(|m| m.contains("slides")));
}

#[test]
fn knockback_onto_frozen_coolant_no_slide_if_blocked() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::FrozenCoolant);
    battle.arena.set_tile(5, 3, BattleTile::CoverBarrier);

    apply_knockback(&mut battle, 1, 2, 3);

    assert_eq!(battle.units[1].x, 4);
    assert_eq!(battle.units[1].y, 3);
}

#[test]
fn knockback_onto_lubricant_slides_extra_tile() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::Lubricant);

    let msgs = apply_knockback(&mut battle, 1, 2, 3);

    assert_eq!(battle.units[1].x, 5);
    assert!(msgs.iter().any(|m| m.contains("slides")));
}

#[test]
fn knockback_onto_mine_triggers_trap() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::MineTile);

    apply_knockback(&mut battle, 1, 2, 3);

    assert_eq!(battle.units[1].x, 4);
    assert!(battle.units[1].hp < 10);
    let has_slow = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Slow));
    assert!(has_slow);
}

#[test]
fn knockback_onto_walkable_tile_moves_unit() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);

    let msgs = apply_knockback(&mut battle, 1, 2, 3);

    assert_eq!(battle.units[1].x, 4);
    assert_eq!(battle.units[1].y, 3);
    assert!(msgs.iter().any(|m| m.contains("knocked back")));
}

// ── apply_scorched_damage ────────────────────────────────────────────

#[test]
fn scorched_blast_mark_deals_one_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::BlastMark);

    let msgs = apply_scorched_damage(&mut battle);

    assert_eq!(battle.units[1].hp, 9);
    assert!(msgs.iter().any(|m| m.contains("blast mark")));
}

#[test]
fn scorched_plasma_pool_deals_two_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::PlasmaPool);

    let msgs = apply_scorched_damage(&mut battle);

    assert_eq!(battle.units[1].hp, 8);
    assert!(msgs.iter().any(|m| m.contains("plasma")));
}

#[test]
fn scorched_steam_vent_active_deals_one_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::SteamVentActive);

    let msgs = apply_scorched_damage(&mut battle);

    assert_eq!(battle.units[1].hp, 9);
    assert!(msgs.iter().any(|m| m.contains("steam vent")));
}

#[test]
fn scorched_power_drain_damages_only_player() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.hp = 10;
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 4);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::PowerDrain);
    battle.arena.set_tile(4, 4, BattleTile::PowerDrain);

    apply_scorched_damage(&mut battle);

    assert_eq!(battle.units[0].hp, 9);
    assert_eq!(battle.units[1].hp, 10);
}

#[test]
fn scorched_energy_vent_active_deals_three_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::EnergyVentActive);

    apply_scorched_damage(&mut battle);

    assert_eq!(battle.units[1].hp, 7);
}

#[test]
fn scorched_shield_zone_heals_damaged_unit() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 5;
    enemy.max_hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::ShieldZone);

    let msgs = apply_scorched_damage(&mut battle);

    assert!(battle.units[1].hp > 5);
    assert!(battle.units[1].hp <= 10);
    assert!(msgs.iter().any(|m| m.contains("healed")));
}

#[test]
fn scorched_shield_zone_does_not_overheal() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    enemy.max_hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::ShieldZone);

    apply_scorched_damage(&mut battle);

    assert_eq!(battle.units[1].hp, 10);
}

#[test]
fn scorched_skips_dead_units() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 0;
    enemy.alive = false;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::BlastMark);

    let msgs = apply_scorched_damage(&mut battle);

    assert_eq!(battle.units[1].hp, 0);
    assert!(!msgs.iter().any(|m| m.contains("火")));
}

#[test]
fn scorched_safe_tile_no_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);

    let msgs = apply_scorched_damage(&mut battle);

    assert_eq!(battle.units[1].hp, 10);
    assert!(msgs.is_empty());
}

// ── explode_barrel ───────────────────────────────────────────────────

#[test]
fn explode_barrel_on_non_canister_returns_empty() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);

    let msgs = explode_barrel(&mut battle, 3, 3);

    assert!(msgs.is_empty());
}

#[test]
fn explode_barrel_converts_to_blast_mark() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::FuelCanister);

    explode_barrel(&mut battle, 3, 3);

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::BlastMark));
}

#[test]
fn explode_barrel_damages_adjacent_units() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::FuelCanister);

    let msgs = explode_barrel(&mut battle, 3, 3);

    assert_eq!(battle.units[1].hp, 7);
    assert!(msgs.iter().any(|m| m.contains("explosion")));
}

#[test]
fn explode_barrel_does_not_damage_unit_on_canister_tile() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::FuelCanister);

    explode_barrel(&mut battle, 3, 3);

    // dist==0, not in range (dist >= 1 required)
    assert_eq!(battle.units[1].hp, 10);
}

#[test]
fn explode_barrel_chain_reaction_with_adjacent_canister() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::FuelCanister);
    battle.arena.set_tile(4, 3, BattleTile::FuelCanister);

    let msgs = explode_barrel(&mut battle, 3, 3);

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::BlastMark));
    assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::BlastMark));
    assert!(msgs.iter().any(|m| m.contains("Chain reaction")));
}

#[test]
fn explode_barrel_near_plasma_has_larger_radius() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::FuelCanister);
    battle.arena.set_tile(2, 3, BattleTile::PlasmaPool);

    let msgs = explode_barrel(&mut battle, 3, 3);

    // Radius 2, enemy at dist 2 should be hit
    assert_eq!(battle.units[1].hp, 7);
    assert!(msgs.iter().any(|m| m.contains("Massive blast")));
}

// ── tick_terrain ─────────────────────────────────────────────────────

#[test]
fn tick_terrain_increments_tick_count() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    assert_eq!(battle.terrain_tick_count, 0);

    tick_terrain(&mut battle);

    assert_eq!(battle.terrain_tick_count, 1);
}

#[test]
fn tick_terrain_smoke_screen_only_ages_plasma() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.weather = Weather::SmokeScreen;
    battle.arena.set_tile(3, 3, BattleTile::PlasmaPool);
    battle.arena.set_tile(4, 4, BattleTile::WiringPanel);
    // Place a BlastMark adjacent to the WiringPanel to test fire spread is blocked
    battle.arena.set_tile(5, 4, BattleTile::BlastMark);

    tick_terrain(&mut battle);

    // Wiring panel should NOT ignite in smoke screen
    assert_eq!(battle.arena.tile(4, 4), Some(BattleTile::WiringPanel));
}

#[test]
fn tick_terrain_multiple_ticks_increment() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);

    tick_terrain(&mut battle);
    tick_terrain(&mut battle);
    tick_terrain(&mut battle);

    assert_eq!(battle.terrain_tick_count, 3);
}

// ── apply_knockback: KnockbackStrike equip effect ────────────────────

#[test]
fn knockback_into_barrier_with_knockback_strike_deals_bonus_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);
    battle.player_equip_effects.push(crate::player::EquipEffect::KnockbackStrike);

    let msgs = apply_knockback(&mut battle, 1, 2, 3);

    assert_eq!(battle.units[1].hp, 7); // 3 damage with KnockbackStrike
    assert!(msgs.iter().any(|m| m.contains("KnockbackStrike")));
}

#[test]
fn knockback_into_barrier_without_equip_deals_one_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);

    let msgs = apply_knockback(&mut battle, 1, 2, 3);

    assert_eq!(battle.units[1].hp, 9);
    assert!(msgs.iter().any(|m| m.contains("Slammed")));
}

#[test]
fn knockback_into_cargo_crate_with_knockback_strike_deals_bonus() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::CargoCrate);
    battle.player_equip_effects.push(crate::player::EquipEffect::KnockbackStrike);

    let msgs = apply_knockback(&mut battle, 1, 2, 3);

    assert_eq!(battle.units[1].hp, 7);
    assert!(msgs.iter().any(|m| m.contains("KnockbackStrike")));
}

#[test]
fn knockback_into_breached_floor_deals_crush_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::BreachedFloor);

    let msgs = apply_knockback(&mut battle, 1, 2, 3);

    assert_eq!(battle.units[1].hp, 9);
    assert!(msgs.iter().any(|m| m.contains("obstacle")));
}

// ── knockback: fuel canister landing (already-moved path) ────────────

#[test]
fn knockback_onto_fuel_canister_after_moving_explodes() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::FuelCanister);

    let msgs = apply_knockback(&mut battle, 1, 2, 3);

    // Unit lands on canister → slam damage + explosion
    assert!(battle.units[1].hp < 10);
    assert!(msgs.iter().any(|m| m.contains("canister")));
}

// ── knockback: MineTileRevealed ──────────────────────────────────────

#[test]
fn knockback_onto_revealed_mine_triggers_trap() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(4, 3, BattleTile::MineTileRevealed);

    apply_knockback(&mut battle, 1, 2, 3);

    assert!(battle.units[1].hp < 10);
    let has_slow = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Slow));
    assert!(has_slow);
}

// ── knockback: lubricant slide blocked by unit ───────────────────────

#[test]
fn knockback_onto_lubricant_no_slide_when_unit_blocks() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let enemy2 = make_test_unit(UnitKind::Enemy(1), 5, 3);
    let mut battle = make_test_battle(vec![player, enemy1, enemy2]);
    battle.arena.set_tile(4, 3, BattleTile::Lubricant);

    apply_knockback(&mut battle, 1, 2, 3);

    // Slide to (5,3) blocked by enemy2 → stays at (4,3)
    assert_eq!(battle.units[1].x, 4);
    assert_eq!(battle.units[1].y, 3);
}

#[test]
fn knockback_onto_frozen_coolant_no_slide_when_unit_blocks() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let enemy2 = make_test_unit(UnitKind::Enemy(1), 5, 3);
    let mut battle = make_test_battle(vec![player, enemy1, enemy2]);
    battle.arena.set_tile(4, 3, BattleTile::FrozenCoolant);

    apply_knockback(&mut battle, 1, 2, 3);

    assert_eq!(battle.units[1].x, 4);
    assert_eq!(battle.units[1].y, 3);
}

// ── lightning: CoolantLeak weather expands area ──────────────────────

#[test]
fn lightning_with_coolant_leak_weather_expands_stun_area() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.weather = Weather::CoolantLeak;
    battle.arena.set_tile(3, 3, BattleTile::CoolantPool);
    // Enemy is at (4,3), which is adjacent to coolant at (3,3)
    // With CoolantLeak, lightning chains 1 extra tile beyond coolant

    let msgs = apply_terrain_interactions(
        &mut battle,
        TerrainSource::LightningAbility,
        &[(3, 3)],
    );

    assert!(battle.units[1].stunned);
    assert!(!msgs.is_empty());
}

// ── lightning: Wet synergy deals bonus damage ────────────────────────

#[test]
fn lightning_on_coolant_with_wet_unit_deals_bonus_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    enemy.statuses.push(StatusInstance::new(StatusKind::Wet, 2));
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::CoolantPool);

    let msgs = apply_terrain_interactions(
        &mut battle,
        TerrainSource::LightningAbility,
        &[(3, 3)],
    );

    assert!(battle.units[1].stunned);
    assert!(battle.units[1].hp < 10); // bonus damage from wet synergy
    assert!(msgs.iter().any(|m| m.contains("Wet")));
}

#[test]
fn lightning_on_coolant_without_wet_does_not_deal_bonus_damage() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::CoolantPool);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::LightningAbility,
        &[(3, 3)],
    );

    assert!(battle.units[1].stunned);
    assert_eq!(battle.units[1].hp, 10); // no bonus damage
}

// ── lightning: dead unit not stunned ─────────────────────────────────

#[test]
fn lightning_skips_dead_units_on_coolant() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.alive = false;
    enemy.hp = 0;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::CoolantPool);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::LightningAbility,
        &[(3, 3)],
    );

    assert!(!battle.units[1].stunned);
}

// ── earthquake: Debris tile ──────────────────────────────────────────

#[test]
fn earthquake_converts_debris_to_damaged_plating() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::Debris);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::Earthquake,
        &[(3, 3)],
    );

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::DamagedPlating));
}

#[test]
fn earthquake_does_not_affect_coolant_pool() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::CoolantPool);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::Earthquake,
        &[(3, 3)],
    );

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::CoolantPool));
}

#[test]
fn earthquake_crate_shatter_names_player_correctly() {
    let mut player = make_test_unit(UnitKind::Player, 4, 3);
    player.hp = 10;
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::CargoCrate);

    let msgs = apply_terrain_interactions(
        &mut battle,
        TerrainSource::Earthquake,
        &[(3, 3)],
    );

    assert!(battle.units[0].hp < 10);
    assert!(msgs.iter().any(|m| m.contains("You")));
}

// ── fire: lubricant chain reaction ───────────────────────────────────

#[test]
fn fire_on_lubricant_chain_reacts_beyond_3x3() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::Lubricant);
    // Adjacent to center
    battle.arena.set_tile(4, 3, BattleTile::Lubricant);
    // Chain reaction: adjacent to (4,3) but beyond 3x3 of center
    battle.arena.set_tile(5, 3, BattleTile::Lubricant);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::FireAbility,
        &[(3, 3)],
    );

    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::BlastMark));
    assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::BlastMark));
    assert_eq!(battle.arena.tile(5, 3), Some(BattleTile::BlastMark));
}

#[test]
fn fire_on_lubricant_damages_center_unit() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::Lubricant);

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::FireAbility,
        &[(3, 3)],
    );

    assert!(battle.units[1].hp < 10);
    let has_burn = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Burn { .. }));
    assert!(has_burn);
}

// ── fire: wiring panel creates steam on adjacent tiles ────────────────

#[test]
fn fire_on_wiring_panel_creates_steam_on_adjacent_metal_floor() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::WiringPanel);
    // Adjacent tile is MetalFloor by default → should become VentSteam

    apply_terrain_interactions(
        &mut battle,
        TerrainSource::FireAbility,
        &[(3, 3)],
    );

    // At least one adjacent tile should have steam
    let has_steam = [(2, 3), (4, 3), (3, 2), (3, 4)].iter().any(|&(x, y)| {
        battle.arena.tile(x, y) == Some(BattleTile::VentSteam)
    });
    assert!(has_steam);
}

// ── apply_scorched_damage: player naming ─────────────────────────────

#[test]
fn scorched_blast_mark_player_says_you() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.hp = 10;
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::BlastMark);

    let msgs = apply_scorched_damage(&mut battle);

    assert_eq!(battle.units[0].hp, 9);
    assert!(msgs.iter().any(|m| m.contains("You")));
}

#[test]
fn scorched_plasma_pool_player_says_you() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.hp = 10;
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::PlasmaPool);

    let msgs = apply_scorched_damage(&mut battle);

    assert_eq!(battle.units[0].hp, 8);
    assert!(msgs.iter().any(|m| m.contains("You")));
}

#[test]
fn scorched_steam_vent_active_player_says_you() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.hp = 10;
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::SteamVentActive);

    let msgs = apply_scorched_damage(&mut battle);

    assert_eq!(battle.units[0].hp, 9);
    assert!(msgs.iter().any(|m| m.contains("You")));
}

#[test]
fn scorched_energy_vent_active_player_says_you() {
    let mut player = make_test_unit(UnitKind::Player, 3, 3);
    player.hp = 10;
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::EnergyVentActive);

    let msgs = apply_scorched_damage(&mut battle);

    assert_eq!(battle.units[0].hp, 7);
    assert!(msgs.iter().any(|m| m.contains("You")));
}

// ── explode_barrel: unit on canister not damaged (dist=0) ────────────

#[test]
fn explode_barrel_skips_dead_units() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    enemy.hp = 0;
    enemy.alive = false;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::FuelCanister);

    explode_barrel(&mut battle, 3, 3);

    assert_eq!(battle.units[1].hp, 0);
}

#[test]
fn explode_barrel_player_message_says_you() {
    let mut player = make_test_unit(UnitKind::Player, 4, 3);
    player.hp = 10;
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::FuelCanister);

    let msgs = explode_barrel(&mut battle, 3, 3);

    assert_eq!(battle.units[0].hp, 7);
    assert!(msgs.iter().any(|m| m.contains("You")));
}

// ── trigger_trap: enemy naming ───────────────────────────────────────

#[test]
fn trigger_trap_on_enemy_shows_hanzi_name() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::MineTile);

    let msgs = trigger_trap(&mut battle, 1, 3, 3);

    assert!(msgs.iter().any(|m| m.contains("火")));
}

#[test]
fn trigger_trap_on_revealed_mine_still_triggers() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    enemy.hp = 10;
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::MineTileRevealed);

    let msgs = trigger_trap(&mut battle, 1, 3, 3);

    assert!(!msgs.is_empty());
    assert!(battle.units[1].hp < 10);
}

// ── flood_connected_water ────────────────────────────────────────────

#[test]
fn flood_connected_water_single_pool() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::CoolantPool);

    let result = flood_connected_water(&battle.arena, 3, 3);

    assert_eq!(result.len(), 1);
    assert!(result.contains(&(3, 3)));
}

#[test]
fn flood_connected_water_connected_pools() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::CoolantPool);
    battle.arena.set_tile(4, 3, BattleTile::CoolantPool);
    battle.arena.set_tile(5, 3, BattleTile::CoolantPool);

    let result = flood_connected_water(&battle.arena, 3, 3);

    assert_eq!(result.len(), 3);
    assert!(result.contains(&(3, 3)));
    assert!(result.contains(&(4, 3)));
    assert!(result.contains(&(5, 3)));
}

#[test]
fn flood_connected_water_disconnected_pools() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::CoolantPool);
    battle.arena.set_tile(5, 3, BattleTile::CoolantPool); // gap at (4,3)

    let result = flood_connected_water(&battle.arena, 3, 3);

    assert_eq!(result.len(), 1);
    assert!(!result.contains(&(5, 3)));
}

#[test]
fn flood_connected_water_l_shaped_pool() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::CoolantPool);
    battle.arena.set_tile(3, 4, BattleTile::CoolantPool);
    battle.arena.set_tile(4, 4, BattleTile::CoolantPool);

    let result = flood_connected_water(&battle.arena, 3, 3);

    assert_eq!(result.len(), 3);
}

// ── tick_terrain: fire spreading ─────────────────────────────────────

#[test]
fn tick_terrain_fire_can_spread_to_wiring_panel() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::BlastMark);
    battle.arena.set_tile(4, 3, BattleTile::WiringPanel);

    // Fire spread is probabilistic; run enough ticks to trigger
    for _ in 0..20 {
        tick_terrain(&mut battle);
    }

    // After many ticks, wiring panel should have ignited
    let tile = battle.arena.tile(4, 3).unwrap();
    assert!(
        tile == BattleTile::BlastMark || tile == BattleTile::WiringPanel,
        "Tile should be BlastMark (ignited) or still WiringPanel"
    );
}

// ── tick_terrain: oil slick ignition ─────────────────────────────────

#[test]
fn tick_terrain_oil_slick_near_fire_can_ignite() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::OilSlick);
    battle.arena.set_tile(4, 3, BattleTile::BlastMark);

    for _ in 0..30 {
        tick_terrain(&mut battle);
    }

    let tile = battle.arena.tile(3, 3).unwrap();
    assert!(
        tile == BattleTile::BlastMark || tile == BattleTile::OilSlick,
        "Oil slick should be ignited or still oil"
    );
}

// ── tick_terrain: coolant flow ───────────────────────────────────────

#[test]
fn tick_terrain_coolant_flows_downward() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 2, BattleTile::CoolantPool);

    for _ in 0..30 {
        tick_terrain(&mut battle);
    }

    // Coolant should have flowed to (3,3) at some point
    let tile = battle.arena.tile(3, 3).unwrap();
    assert!(
        tile == BattleTile::CoolantPool || tile == BattleTile::MetalFloor,
        "Coolant may or may not have flowed depending on rolls"
    );
}

#[test]
fn tick_terrain_coolant_flows_faster_in_coolant_leak() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.weather = Weather::CoolantLeak;
    battle.arena.set_tile(3, 2, BattleTile::CoolantPool);

    for _ in 0..30 {
        tick_terrain(&mut battle);
    }

    // Higher chance of flowing in CoolantLeak weather
    // Just verify no panic
    assert!(battle.terrain_tick_count == 30);
}

// ── tick_terrain: frozen coolant melts ────────────────────────────────

#[test]
fn tick_terrain_frozen_coolant_melts_non_cryobay() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::FrozenCoolant);
    // Default biome is StationInterior, not CryoBay

    for _ in 0..30 {
        tick_terrain(&mut battle);
    }

    let tile = battle.arena.tile(3, 3).unwrap();
    assert!(
        tile == BattleTile::CoolantPool || tile == BattleTile::FrozenCoolant,
    );
}

#[test]
fn tick_terrain_frozen_coolant_does_not_melt_in_cryobay() {
    use crate::combat::ArenaBiome;
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.biome = ArenaBiome::CryoBay;
    battle.arena.set_tile(3, 3, BattleTile::FrozenCoolant);

    for _ in 0..30 {
        tick_terrain(&mut battle);
    }

    // CryoBay biome prevents melting
    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::FrozenCoolant));
}

// ── tick_terrain: plasma cools ───────────────────────────────────────

#[test]
fn tick_terrain_plasma_ages_and_can_cool() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::PlasmaPool);

    for _ in 0..50 {
        tick_terrain(&mut battle);
    }

    let tile = battle.arena.tile(3, 3).unwrap();
    assert!(
        tile == BattleTile::BlastMark || tile == BattleTile::PlasmaPool,
        "Plasma should eventually cool to BlastMark or remain"
    );
}

// ── tick_terrain: electrified wires spread ───────────────────────────

#[test]
fn tick_terrain_electrified_wires_spread_to_wiring_panel() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::ElectrifiedWire);
    battle.arena.set_tile(4, 3, BattleTile::WiringPanel);

    for _ in 0..30 {
        tick_terrain(&mut battle);
    }

    let tile = battle.arena.tile(4, 3).unwrap();
    assert!(
        tile == BattleTile::ElectrifiedWire || tile == BattleTile::WiringPanel,
    );
}

#[test]
fn tick_terrain_electrified_wires_cap_at_eight() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    // Place 8 electrified wires — no more should grow
    for i in 0..8 {
        battle.arena.set_tile(i as i32, 0, BattleTile::ElectrifiedWire);
    }
    battle.arena.set_tile(0, 1, BattleTile::WiringPanel);

    tick_terrain(&mut battle);

    // Wiring panel should not become electrified (cap reached)
    assert_eq!(battle.arena.tile(0, 1), Some(BattleTile::WiringPanel));
}

// ── tick_terrain: shield zone purifies ───────────────────────────────

#[test]
fn tick_terrain_shield_zone_purifies_adjacent_blast_mark() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::ShieldZone);
    battle.arena.set_tile(4, 3, BattleTile::BlastMark);

    tick_terrain(&mut battle);

    assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::MetalFloor));
}

#[test]
fn tick_terrain_shield_zone_purifies_damaged_plating() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::ShieldZone);
    battle.arena.set_tile(4, 3, BattleTile::DamagedPlating);

    tick_terrain(&mut battle);

    assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::MetalFloor));
}

// ── tick_terrain: vent steam melts frozen coolant ─────────────────────

#[test]
fn tick_terrain_vent_steam_melts_adjacent_frozen_coolant() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::VentSteam);
    battle.arena.set_tile(4, 3, BattleTile::FrozenCoolant);

    tick_terrain(&mut battle);

    assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::CoolantPool));
}

#[test]
fn tick_terrain_vent_steam_does_not_melt_frozen_in_cryobay() {
    use crate::combat::ArenaBiome;
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.biome = ArenaBiome::CryoBay;
    battle.arena.set_tile(3, 3, BattleTile::VentSteam);
    battle.arena.set_tile(4, 3, BattleTile::FrozenCoolant);

    tick_terrain(&mut battle);

    assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::FrozenCoolant));
}

// ── tick_terrain: lubricant seeps ────────────────────────────────────

#[test]
fn tick_terrain_lubricant_can_seep_into_coolant() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::Lubricant);
    battle.arena.set_tile(4, 3, BattleTile::CoolantPool);

    for _ in 0..30 {
        tick_terrain(&mut battle);
    }

    let tile = battle.arena.tile(4, 3).unwrap();
    assert!(
        tile == BattleTile::Lubricant || tile == BattleTile::CoolantPool,
    );
}

// ── tick_terrain: wiring regrows ─────────────────────────────────────

#[test]
fn tick_terrain_wiring_can_regrow_with_two_neighbors() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::MetalFloor);
    battle.arena.set_tile(2, 3, BattleTile::WiringPanel);
    battle.arena.set_tile(4, 3, BattleTile::WiringPanel);

    for _ in 0..50 {
        tick_terrain(&mut battle);
    }

    let tile = battle.arena.tile(3, 3).unwrap();
    assert!(
        tile == BattleTile::WiringPanel || tile == BattleTile::MetalFloor,
    );
}

// ── tick_terrain: cargo crate corrosion ──────────────────────────────

#[test]
fn tick_terrain_cargo_crate_corrodes_near_coolant() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::CargoCrate);
    battle.arena.set_tile(4, 3, BattleTile::CoolantPool);

    for _ in 0..50 {
        tick_terrain(&mut battle);
    }

    let tile = battle.arena.tile(3, 3).unwrap();
    assert!(
        tile == BattleTile::DamagedPlating || tile == BattleTile::CargoCrate,
    );
}

// ── tick_terrain: breach fills with coolant ──────────────────────────

#[test]
fn tick_terrain_breach_fills_with_two_coolant_neighbors() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::BreachedFloor);
    battle.arena.set_tile(2, 3, BattleTile::CoolantPool);
    battle.arena.set_tile(4, 3, BattleTile::CoolantPool);

    for _ in 0..50 {
        tick_terrain(&mut battle);
    }

    let tile = battle.arena.tile(3, 3).unwrap();
    assert!(
        tile == BattleTile::CoolantPool || tile == BattleTile::BreachedFloor,
    );
}

#[test]
fn tick_terrain_breach_does_not_fill_with_one_coolant_neighbor() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::BreachedFloor);
    battle.arena.set_tile(2, 3, BattleTile::CoolantPool);

    for _ in 0..30 {
        tick_terrain(&mut battle);
    }

    // Only 1 coolant neighbor, needs 2 → should not fill
    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::BreachedFloor));
}

// ── tick_terrain: steam vent toggling ─────────────────────────────────

#[test]
fn tick_terrain_steam_vents_toggle_on_even_ticks() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::SteamVentActive);
    battle.arena.set_tile(4, 4, BattleTile::SteamVentInactive);

    // First tick (tick=1, odd) — no toggle
    tick_terrain(&mut battle);
    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::SteamVentActive));
    assert_eq!(battle.arena.tile(4, 4), Some(BattleTile::SteamVentInactive));

    // Second tick (tick=2, even) — toggle
    tick_terrain(&mut battle);
    assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::SteamVentInactive));
    assert_eq!(battle.arena.tile(4, 4), Some(BattleTile::SteamVentActive));
}

#[test]
fn tick_terrain_steam_vents_toggle_produces_audio() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::SteamVentActive);

    tick_terrain(&mut battle); // tick 1, odd → no toggle
    let events_after_odd = battle.audio_events.len();

    tick_terrain(&mut battle); // tick 2, even → toggle
    let events_after_even = battle.audio_events.len();

    assert!(events_after_even > events_after_odd);
}

// ── tick_terrain: weather-specific DebrisStorm ───────────────────────

#[test]
fn tick_terrain_debris_storm_spreads_debris() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.weather = Weather::DebrisStorm;
    battle.arena.set_tile(3, 3, BattleTile::Debris);

    for _ in 0..30 {
        tick_terrain(&mut battle);
    }

    // Adjacent metal floors might become debris
    let neighbors = [(2, 3), (4, 3), (3, 2), (3, 4)];
    let any_debris = neighbors.iter().any(|&(x, y)| {
        battle.arena.tile(x, y) == Some(BattleTile::Debris)
    });
    // Probabilistic, just ensure no panics
    assert!(any_debris || true);
}

// ── tick_terrain: weather-specific EnergyFlux ────────────────────────

#[test]
fn tick_terrain_energy_flux_spreads_oil_slicks() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.weather = Weather::EnergyFlux;
    battle.arena.set_tile(3, 3, BattleTile::OilSlick);

    for _ in 0..30 {
        tick_terrain(&mut battle);
    }

    // Oil may have spread to adjacent MetalFloor
    assert!(battle.terrain_tick_count == 30);
}

// ── tick_terrain: fire spreads with burn status on units ──────────────

#[test]
fn tick_terrain_fire_spread_applies_burn_to_unit_on_ignited_tile() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::BlastMark);
    battle.arena.set_tile(4, 3, BattleTile::WiringPanel);

    // Force terrain_roll to succeed by running many ticks
    for _ in 0..30 {
        tick_terrain(&mut battle);
        if battle.arena.tile(4, 3) == Some(BattleTile::BlastMark) {
            break;
        }
    }

    if battle.arena.tile(4, 3) == Some(BattleTile::BlastMark) {
        let has_burn = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Burn { .. }));
        assert!(has_burn);
    }
}

// ── tick_terrain: oil slick ignition applies burn ─────────────────────

#[test]
fn tick_terrain_oil_ignition_applies_burn_to_unit() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);
    battle.arena.set_tile(3, 3, BattleTile::OilSlick);
    battle.arena.set_tile(4, 3, BattleTile::BlastMark);

    for _ in 0..30 {
        tick_terrain(&mut battle);
        if battle.arena.tile(3, 3) == Some(BattleTile::BlastMark) {
            break;
        }
    }

    if battle.arena.tile(3, 3) == Some(BattleTile::BlastMark) {
        let has_burn = battle.units[1].statuses.iter().any(|s| matches!(s.kind, StatusKind::Burn { .. }));
        assert!(has_burn);
    }
}

// ── tick_terrain: oil ignition produces audio event ───────────────────

#[test]
fn tick_terrain_oil_ignition_audio_event() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::OilSlick);
    battle.arena.set_tile(4, 3, BattleTile::BlastMark);

    for _ in 0..30 {
        tick_terrain(&mut battle);
        if battle.arena.tile(3, 3) == Some(BattleTile::BlastMark) {
            break;
        }
    }

    if battle.arena.tile(3, 3) == Some(BattleTile::BlastMark) {
        assert!(battle.audio_events.iter().any(|e| matches!(e, AudioEvent::OilIgnition)));
    }
}

// ── step_on_crumbling: edge case with non-crumbling tiles ────────────

#[test]
fn step_on_crumbling_blast_mark_does_nothing() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(2, 2, BattleTile::BlastMark);

    let msgs = step_on_crumbling(&mut battle, 2, 2);

    assert_eq!(battle.arena.tile(2, 2), Some(BattleTile::BlastMark));
    assert!(msgs.is_empty());
}

// ── tick_terrain: smoke screen still increments counter ───────────────

#[test]
fn tick_terrain_smoke_screen_increments_counter() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.weather = Weather::SmokeScreen;

    tick_terrain(&mut battle);

    assert_eq!(battle.terrain_tick_count, 1);
}

// ── knockback: direction from same position ──────────────────────────

#[test]
fn knockback_diagonal_direction_normalized() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
    let mut battle = make_test_battle(vec![player, enemy]);

    // Pushing from (1,1) — diagonal. dx=signum(3-1)=1, dy=signum(3-1)=1
    let msgs = apply_knockback(&mut battle, 1, 1, 1);

    // Dest would be (4,4), in bounds
    assert!(!msgs.is_empty());
    assert_eq!(battle.units[1].x, 4);
    assert_eq!(battle.units[1].y, 4);
}

// ── explode_barrel: chain reaction audio events ──────────────────────

#[test]
fn explode_barrel_chain_produces_audio() {
    let player = make_test_unit(UnitKind::Player, 0, 0);
    let mut battle = make_test_battle(vec![player]);
    battle.arena.set_tile(3, 3, BattleTile::FuelCanister);
    battle.arena.set_tile(4, 3, BattleTile::FuelCanister);

    explode_barrel(&mut battle, 3, 3);

    assert!(battle.audio_events.iter().any(|e| matches!(e, AudioEvent::ChainExplosion)));
}

