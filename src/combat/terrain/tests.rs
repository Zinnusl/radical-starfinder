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

