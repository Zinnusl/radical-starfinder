use crate::combat::action::deal_damage;
use crate::combat::{ArenaBiome, BattleTile, TacticalBattle, Weather};
use crate::status::{StatusInstance, StatusKind};
use std::collections::VecDeque;

pub enum TerrainSource {
    FireSpell,
    LightningSpell,
    Earthquake,
}

pub fn apply_terrain_interactions(
    battle: &mut TacticalBattle,
    source: TerrainSource,
    affected_tiles: &[(i32, i32)],
) -> Vec<String> {
    let mut messages = Vec::new();

    match source {
        TerrainSource::FireSpell => {
            for &(x, y) in affected_tiles {
                if let Some(tile) = battle.arena.tile(x, y) {
                    match tile {
                        BattleTile::Grass | BattleTile::Thorns => {
                            battle.arena.set_tile(x, y, BattleTile::Scorched);
                            messages.push(format!("Grass burns at ({},{})!", x, y));
                            // Burning grass creates Steam (smoke) blocking LOS
                            let deltas: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                            for &(sdx, sdy) in &deltas {
                                let sx = x + sdx;
                                let sy = y + sdy;
                                if let Some(st) = battle.arena.tile(sx, sy) {
                                    if st == BattleTile::Open || st == BattleTile::Grass {
                                        battle.arena.set_steam(sx, sy, 2);
                                    }
                                }
                            }
                            messages.push(format!("💨 Smoke billows from burning grass!"));
                            if let Some(idx) = battle.unit_at(x, y) {
                                let actual = deal_damage(battle, idx, 1);
                                messages.push(format!("Fire scorches for {} damage!", actual));
                            }
                        }
                        BattleTile::Ice => {
                            battle.arena.set_tile(x, y, BattleTile::Water);
                            messages.push(format!("Ice melts at ({},{})!", x, y));
                            // Water+Ice cascade: melted water melts adjacent ice
                            let deltas: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                            for &(cdx, cdy) in &deltas {
                                let cx = x + cdx;
                                let cy = y + cdy;
                                if battle.arena.tile(cx, cy) == Some(BattleTile::Ice) {
                                    battle.arena.set_tile(cx, cy, BattleTile::Water);
                                    messages.push(format!("🌊 Ice melts into water at ({},{})! Flood cascade!", cx, cy));
                                }
                            }
                        }
                        BattleTile::Water => {
                            battle.arena.set_steam(x, y, 2);
                            messages.push(format!("Steam erupts at ({},{})!", x, y));
                        }
                        BattleTile::Oil => {
                            // Oil ignites into 3×3 fire — chain reaction spreads to all connected oil!
                            battle.arena.set_tile(x, y, BattleTile::Scorched);
                            messages.push(format!("Oil ignites at ({},{})!", x, y));
                            let deltas_3x3: [(i32, i32); 8] = [
                                (-1, -1), (0, -1), (1, -1),
                                (-1, 0),           (1, 0),
                                (-1, 1),  (0, 1),  (1, 1),
                            ];
                            // Collect chain-reaction oil tiles beyond the initial 3x3
                            let mut chain_oil = Vec::new();
                            for &(adx, ady) in &deltas_3x3 {
                                let ax = x + adx;
                                let ay = y + ady;
                                if let Some(at) = battle.arena.tile(ax, ay) {
                                    if at == BattleTile::Oil {
                                        battle.arena.set_tile(ax, ay, BattleTile::Scorched);
                                        // Check for further oil chain reaction
                                        for &(cdx, cdy) in &[(-1i32,0),(1,0),(0,-1i32),(0,1)] {
                                            let cx = ax + cdx;
                                            let cy = ay + cdy;
                                            if battle.arena.tile(cx, cy) == Some(BattleTile::Oil) {
                                                chain_oil.push((cx, cy));
                                            }
                                        }
                                    }
                                }
                                if let Some(aidx) = battle.unit_at(ax, ay) {
                                    let actual = deal_damage(battle, aidx, 3);
                                    use crate::status::{StatusInstance, StatusKind};
                                    battle.units[aidx]
                                        .statuses
                                        .push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 2));
                                    messages.push(format!(
                                        "Oil fire burns {} for {} damage!",
                                        battle.units[aidx].hanzi, actual
                                    ));
                                }
                            }
                            // Chain reaction: ignite oil tiles beyond 3x3 blast
                            for (cx, cy) in chain_oil {
                                if battle.arena.tile(cx, cy) == Some(BattleTile::Oil) {
                                    battle.arena.set_tile(cx, cy, BattleTile::Scorched);
                                    messages.push(format!("🔥 Oil chain reaction at ({},{})!", cx, cy));
                                    if let Some(cidx) = battle.unit_at(cx, cy) {
                                        let actual = deal_damage(battle, cidx, 3);
                                        use crate::status::{StatusInstance, StatusKind};
                                        battle.units[cidx]
                                            .statuses
                                            .push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 2));
                                        messages.push(format!(
                                            "Chain fire burns {} for {} damage!",
                                            battle.units[cidx].hanzi, actual
                                        ));
                                    }
                                }
                            }
                            // Also damage unit on the center tile
                            if let Some(cidx) = battle.unit_at(x, y) {
                                let actual = deal_damage(battle, cidx, 3);
                                use crate::status::{StatusInstance, StatusKind};
                                battle.units[cidx]
                                    .statuses
                                    .push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 2));
                                messages.push(format!(
                                    "Oil fire burns {} for {} damage!",
                                    battle.units[cidx].hanzi, actual
                                ));
                            }
                        }
                        BattleTile::ExplosiveBarrel => {
                            let mut barrel_msgs = explode_barrel(battle, x, y);
                            messages.append(&mut barrel_msgs);
                        }
                        _ => {}
                    }
                }
            }
        }
        TerrainSource::LightningSpell => {
            let mut stun_targets = Vec::new();
            for &(x, y) in affected_tiles {
                if battle.arena.tile(x, y) == Some(BattleTile::Water) {
                    let connected = flood_connected_water(&battle.arena, x, y);
                    // In rain, lightning chains 1 extra tile beyond water
                    let expanded = if battle.weather == Weather::Rain {
                        let mut extra = Vec::new();
                        let deltas: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                        for &(wx, wy) in &connected {
                            for (dx, dy) in &deltas {
                                let nx = wx + dx;
                                let ny = wy + dy;
                                if battle.arena.in_bounds(nx, ny)
                                    && !connected.contains(&(nx, ny))
                                    && !extra.contains(&(nx, ny))
                                {
                                    extra.push((nx, ny));
                                }
                            }
                        }
                        let mut all = connected;
                        all.extend(extra);
                        all
                    } else {
                        connected
                    };
                    for (wx, wy) in expanded {
                        if let Some(idx) = battle.unit_at(wx, wy) {
                            if !stun_targets.contains(&idx) {
                                stun_targets.push(idx);
                            }
                        }
                    }
                }
            }
            for idx in stun_targets {
                if battle.units[idx].alive {
                    battle.units[idx].stunned = true;
                    // Lightning + Wet synergy: deal +2 bonus damage to wet units
                    let is_wet = battle.units[idx]
                        .statuses
                        .iter()
                        .any(|s| matches!(s.kind, crate::status::StatusKind::Wet));
                    if is_wet {
                        let actual = deal_damage(battle, idx, 2);
                        messages.push(format!(
                            "⚡💧 Lightning + Wet! {} is shocked for {} bonus damage and stunned!",
                            battle.units[idx].hanzi, actual
                        ));
                    } else {
                        messages.push(format!(
                            "Water conducts! {} is stunned!",
                            battle.units[idx].hanzi
                        ));
                    }
                }
            }
        }
        TerrainSource::Earthquake => {
            for &(x, y) in affected_tiles {
                if let Some(tile) = battle.arena.tile(x, y) {
                    if matches!(
                        tile,
                        BattleTile::Open | BattleTile::Grass | BattleTile::Sand
                    ) {
                        battle.arena.set_tile(x, y, BattleTile::BrokenGround);
                    }
                    // Earthquake + Boulder → Boulders shatter into BrokenGround
                    if tile == BattleTile::Boulder {
                        battle.arena.set_tile(x, y, BattleTile::BrokenGround);
                        messages.push(format!("🪨 Boulder shatters at ({},{})!", x, y));
                        // Shatter deals 1 damage to adjacent units
                        let deltas: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                        for &(dx, dy) in &deltas {
                            let ax = x + dx;
                            let ay = y + dy;
                            if let Some(aidx) = battle.unit_at(ax, ay) {
                                let actual = deal_damage(battle, aidx, 1);
                                let aname = if battle.units[aidx].is_player() {
                                    "You".to_string()
                                } else {
                                    battle.units[aidx].hanzi.to_string()
                                };
                                messages.push(format!(
                                    "{} hit by boulder shrapnel! (-{} HP)",
                                    aname, actual
                                ));
                            }
                        }
                    }
                }
            }
            messages.push("The ground cracks and buckles!".to_string());
        }
    }

    messages
}

fn flood_connected_water(
    arena: &crate::combat::TacticalArena,
    start_x: i32,
    start_y: i32,
) -> Vec<(i32, i32)> {
    let mut visited = vec![false; arena.width * arena.height];
    let mut result = Vec::new();
    let mut queue = VecDeque::new();

    if let Some(i) = arena.idx(start_x, start_y) {
        visited[i] = true;
        queue.push_back((start_x, start_y));
        result.push((start_x, start_y));
    }

    let deltas: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    while let Some((cx, cy)) = queue.pop_front() {
        for (dx, dy) in &deltas {
            let nx = cx + dx;
            let ny = cy + dy;
            if let Some(i) = arena.idx(nx, ny) {
                if !visited[i] && arena.tiles[i] == BattleTile::Water {
                    visited[i] = true;
                    queue.push_back((nx, ny));
                    result.push((nx, ny));
                }
            }
        }
    }

    result
}

pub fn apply_knockback(
    battle: &mut TacticalBattle,
    target_idx: usize,
    from_x: i32,
    from_y: i32,
) -> Vec<String> {
    let mut messages = Vec::new();
    let tx = battle.units[target_idx].x;
    let ty = battle.units[target_idx].y;

    let dx = (tx - from_x).signum();
    let dy = (ty - from_y).signum();
    if dx == 0 && dy == 0 {
        return messages;
    }

    let dest_x = tx + dx;
    let dest_y = ty + dy;

    if !battle.arena.in_bounds(dest_x, dest_y) {
        messages.push("Knockback stopped at arena edge!".to_string());
        return messages;
    }

    let dest_tile = battle
        .arena
        .tile(dest_x, dest_y)
        .unwrap_or(BattleTile::Open);

    if dest_tile == BattleTile::Obstacle || dest_tile == BattleTile::Pit {
        // KnockbackStrike + Wall/Boulder crush synergy: +2 bonus damage
        let has_knockback_equip = battle.player_equip_effects.iter().any(|e| {
            matches!(e, crate::player::EquipEffect::KnockbackStrike)
        });
        let crush_dmg = if has_knockback_equip { 3 } else { 1 };
        let actual = deal_damage(battle, target_idx, crush_dmg);
        if has_knockback_equip {
            messages.push(format!(
                "💥 Crushed into obstacle for {} damage! (KnockbackStrike bonus!)",
                actual
            ));
        } else {
            messages.push(format!("Slammed into obstacle for {} damage!", actual));
        }
        return messages;
    }

    if dest_tile == BattleTile::Boulder {
        // KnockbackStrike + Boulder crush synergy: +2 bonus damage
        let has_knockback_equip = battle.player_equip_effects.iter().any(|e| {
            matches!(e, crate::player::EquipEffect::KnockbackStrike)
        });
        let crush_dmg = if has_knockback_equip { 3 } else { 1 };
        let actual = deal_damage(battle, target_idx, crush_dmg);
        if has_knockback_equip {
            messages.push(format!(
                "💥 Crushed into boulder for {} damage! (KnockbackStrike bonus!)",
                actual
            ));
        } else {
            messages.push(format!("Slammed into boulder for {} damage!", actual));
        }
        return messages;
    }

    if dest_tile == BattleTile::ExplosiveBarrel {
        let actual = deal_damage(battle, target_idx, 1);
        messages.push(format!("Slammed into a barrel for {} damage!", actual));
        let mut barrel_msgs = explode_barrel(battle, dest_x, dest_y);
        messages.append(&mut barrel_msgs);
        return messages;
    }

    if let Some(collide_idx) = battle.unit_at(dest_x, dest_y) {
        // Collision damage: both the knocked unit and the unit it hits take 1 damage.
        let actual_target = deal_damage(battle, target_idx, 1);
        let actual_collide = deal_damage(battle, collide_idx, 1);
        messages.push(format!(
            "Collision! {} takes {} damage, {} takes {} damage!",
            battle.units[target_idx].hanzi,
            actual_target,
            battle.units[collide_idx].hanzi,
            actual_collide
        ));
        return messages;
    }

    battle.units[target_idx].x = dest_x;
    battle.units[target_idx].y = dest_y;

    match dest_tile {
        BattleTile::Water => {
            use crate::status::{StatusInstance, StatusKind};
            battle.units[target_idx]
                .statuses
                .push(StatusInstance::new(StatusKind::Confused, 1));
            messages.push("Knocked into water — slowed!".to_string());
        }
        BattleTile::Lava => {
            let actual = deal_damage(battle, target_idx, 2);
            messages.push(format!(
                "{} knocked into lava for {} damage!",
                battle.units[target_idx].hanzi, actual
            ));
        }
        BattleTile::Scorched => {
            let actual = deal_damage(battle, target_idx, 1);
            messages.push(format!(
                "{} knocked onto scorched ground for {} damage!",
                battle.units[target_idx].hanzi, actual
            ));
        }
        BattleTile::Thorns => {
            let actual = deal_damage(battle, target_idx, 1);
            messages.push(format!(
                "{} knocked into thorns for {} damage!",
                battle.units[target_idx].hanzi, actual
            ));
        }
        BattleTile::Ice => {
            messages.push(format!("{} slides on ice!", battle.units[target_idx].hanzi));
            let slide_x = dest_x + dx;
            let slide_y = dest_y + dy;
            if battle.arena.in_bounds(slide_x, slide_y)
                && battle
                    .arena
                    .tile(slide_x, slide_y)
                    .map(|t| t.is_walkable())
                    .unwrap_or(false)
                && battle.unit_at(slide_x, slide_y).is_none()
            {
                battle.units[target_idx].x = slide_x;
                battle.units[target_idx].y = slide_y;
                messages.push(format!("Slid to ({},{})!", slide_x, slide_y));
            }
        }
        BattleTile::Oil => {
            messages.push(format!("{} slides on oil!", battle.units[target_idx].hanzi));
            let slide_x = dest_x + dx;
            let slide_y = dest_y + dy;
            if battle.arena.in_bounds(slide_x, slide_y)
                && battle
                    .arena
                    .tile(slide_x, slide_y)
                    .map(|t| t.is_walkable())
                    .unwrap_or(false)
                && battle.unit_at(slide_x, slide_y).is_none()
            {
                battle.units[target_idx].x = slide_x;
                battle.units[target_idx].y = slide_y;
                messages.push(format!("Slid to ({},{})!", slide_x, slide_y));
            }
        }
        BattleTile::ExplosiveBarrel => {
            messages.push(format!(
                "{} knocked into a barrel!",
                battle.units[target_idx].hanzi
            ));
            let mut barrel_msgs = explode_barrel(battle, dest_x, dest_y);
            messages.append(&mut barrel_msgs);
        }
        BattleTile::TrapTile | BattleTile::TrapTileRevealed => {
            let mut trap_msgs = trigger_trap(battle, target_idx, dest_x, dest_y);
            messages.append(&mut trap_msgs);
        }
        _ => {
            messages.push(format!("{} knocked back!", battle.units[target_idx].hanzi));
        }
    }

    messages
}

pub fn apply_scorched_damage(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();
    for i in 0..battle.units.len() {
        if !battle.units[i].alive {
            continue;
        }
        let tile = battle.arena.tile(battle.units[i].x, battle.units[i].y);
        if tile == Some(BattleTile::Scorched) {
            let actual = deal_damage(battle, i, 1);
            let name = if battle.units[i].is_player() {
                "You".to_string()
            } else {
                battle.units[i].hanzi.to_string()
            };
            messages.push(format!(
                "{} burns on scorched ground! (-{} HP)",
                name, actual
            ));
        }
        if tile == Some(BattleTile::Lava) {
            let actual = deal_damage(battle, i, 2);
            let name = if battle.units[i].is_player() {
                "You".to_string()
            } else {
                battle.units[i].hanzi.to_string()
            };
            messages.push(format!("{} sears in lava! (-{} HP)", name, actual));
        }
        // SpiritDrain: drains spirit from player standing on it
        if tile == Some(BattleTile::SpiritDrain) && battle.units[i].is_player() {
            battle.pending_spirit_delta -= 3;
            messages.push("⚫ The spirit drain saps your energy! (-3 spirit)".to_string());
        }
        // HolyGround: heal units standing on it
        if tile == Some(BattleTile::HolyGround) {
            let ux = battle.units[i].x;
            let uy = battle.units[i].y;
            // Heal amount stored in steam_timers for HolyGround tiles
            let heal_amt = battle
                .arena
                .idx(ux, uy)
                .map(|idx| battle.arena.steam_timers[idx] as i32)
                .unwrap_or(1)
                .max(1);
            let unit = &mut battle.units[i];
            let healed = heal_amt.min(unit.max_hp - unit.hp);
            unit.hp = (unit.hp + heal_amt).min(unit.max_hp);
            if healed > 0 {
                let name = if unit.is_player() {
                    "You".to_string()
                } else {
                    unit.hanzi.to_string()
                };
                messages.push(format!(
                    "✨ {} healed {} HP by holy ground!",
                    name, healed
                ));
            }
        }
    }
    messages
}

pub fn explode_barrel(battle: &mut TacticalBattle, bx: i32, by: i32) -> Vec<String> {
    let mut messages = Vec::new();
    if battle.arena.tile(bx, by) != Some(BattleTile::ExplosiveBarrel) {
        return messages;
    }
    // Check if barrel is on or adjacent to Lava → larger 2-tile radius explosion
    let on_lava = {
        let deltas: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        deltas.iter().any(|&(dx, dy)| {
            battle.arena.tile(bx + dx, by + dy) == Some(BattleTile::Lava)
        }) || false // barrel tile itself was ExplosiveBarrel, can't be Lava
    };
    battle.arena.set_tile(bx, by, BattleTile::Scorched);

    let radius = if on_lava { 2 } else { 1 };
    if on_lava {
        messages.push(format!("💥🌋 Barrel explodes near lava at ({},{})! Massive blast!", bx, by));
    } else {
        messages.push(format!("💥 Barrel explodes at ({},{})!", bx, by));
    }

    // Damage units within blast radius
    for i in 0..battle.units.len() {
        if !battle.units[i].alive {
            continue;
        }
        let dist = (battle.units[i].x - bx).abs() + (battle.units[i].y - by).abs();
        if dist >= 1 && dist <= radius {
            let actual = deal_damage(battle, i, 3);
            let name = if battle.units[i].is_player() {
                "You".to_string()
            } else {
                battle.units[i].hanzi.to_string()
            };
            messages.push(format!(
                "{} caught in explosion! (-{} HP)",
                name, actual
            ));
        }
    }

    let deltas: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

    let mut chain_targets = Vec::new();
    for &(dx, dy) in &deltas {
        let nx = bx + dx;
        let ny = by + dy;
        if battle.arena.tile(nx, ny) == Some(BattleTile::ExplosiveBarrel) {
            chain_targets.push((nx, ny));
        }
    }
    for (cx, cy) in chain_targets {
        messages.push("Chain reaction!".to_string());
        let mut chain_msgs = explode_barrel(battle, cx, cy);
        messages.append(&mut chain_msgs);
    }

    messages
}

pub fn trigger_trap(
    battle: &mut TacticalBattle,
    unit_idx: usize,
    tx: i32,
    ty: i32,
) -> Vec<String> {
    let mut messages = Vec::new();
    let tile = battle.arena.tile(tx, ty);
    if tile != Some(BattleTile::TrapTile) && tile != Some(BattleTile::TrapTileRevealed) {
        return messages;
    }
    battle.arena.set_tile(tx, ty, BattleTile::TrapTileRevealed);
    let actual = deal_damage(battle, unit_idx, 2);
    let name = if battle.units[unit_idx].is_player() {
        "You".to_string()
    } else {
        battle.units[unit_idx].hanzi.to_string()
    };
    messages.push(format!("▲ {} triggers a spike trap! (-{} HP)", name, actual));
    use crate::status::{StatusInstance, StatusKind};
    battle.units[unit_idx]
        .statuses
        .push(StatusInstance::new(StatusKind::Slow, 2));
    messages.push(format!("{} is slowed!", name));
    messages
}

pub fn step_on_crumbling(battle: &mut TacticalBattle, x: i32, y: i32) -> Vec<String> {
    let mut messages = Vec::new();
    let tile = battle.arena.tile(x, y);
    if tile == Some(BattleTile::CrumblingFloor) {
        battle.arena.set_tile(x, y, BattleTile::CrackedFloor);
        messages.push("The floor cracks beneath your feet!".to_string());
    } else if tile == Some(BattleTile::CrackedFloor) {
        battle.arena.set_tile(x, y, BattleTile::Pit);
        messages.push("The floor collapses into a pit!".to_string());
    }
    messages
}

pub fn decay_cracked_floors(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();
    let w = battle.arena.width as i32;
    let h = battle.arena.height as i32;
    for y in 0..h {
        for x in 0..w {
            if battle.arena.tile(x, y) == Some(BattleTile::CrackedFloor) {
                if battle.unit_at(x, y).is_none() {
                    battle.arena.set_tile(x, y, BattleTile::Pit);
                    messages.push(format!("Cracked floor collapses at ({},{})!", x, y));
                }
            }
        }
    }
    messages
}

// ── Terrain Tick System ─────────────────────────────────────────────────────

const DELTAS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// Deterministic pseudo-random roll: returns true when the hash falls below `chance` (0–100).
fn terrain_roll(tick: u32, tile_idx: usize, chance: u32) -> bool {
    ((tick as u64).wrapping_mul(7).wrapping_add((tile_idx as u64).wrapping_mul(13))) % 100
        < chance as u64
}

/// Called once per round to evolve terrain. Processes rules in priority order.
pub fn tick_terrain(battle: &mut TacticalBattle) {
    battle.terrain_tick_count += 1;
    let tick = battle.terrain_tick_count;
    let w = battle.arena.width as i32;
    let h = battle.arena.height as i32;
    let weather = battle.weather;
    let biome = battle.arena.biome;

    // In Fog weather, terrain is stabilized — no evolution.
    if weather == Weather::Fog {
        // Still age lava timers even in fog
        for i in 0..battle.arena.tiles.len() {
            if battle.arena.tiles[i] == BattleTile::Lava && battle.arena.lava_timers[i] < 255 {
                battle.arena.lava_timers[i] += 1;
            }
        }
        return;
    }

    // ── 1. Fire Spreads ─────────────────────────────────────────────────
    let fire_chance = if weather == Weather::Rain { 20 } else { 40 };
    {
        let mut ignitions = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::Scorched) {
                    for &(dx, dy) in &DELTAS {
                        let nx = x + dx;
                        let ny = y + dy;
                        if battle.arena.tile(nx, ny) == Some(BattleTile::Grass) {
                            if let Some(idx) = battle.arena.idx(nx, ny) {
                                if terrain_roll(tick, idx, fire_chance) {
                                    ignitions.push((nx, ny));
                                }
                            }
                        }
                    }
                }
            }
        }
        for (x, y) in ignitions {
            battle.arena.set_tile(x, y, BattleTile::Scorched);
            battle.log_message(format!("🔥 Fire spreads to ({},{})!", x, y));
            if let Some(uid) = battle.unit_at(x, y) {
                battle.units[uid]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 2));
            }
        }
    }

    // ── 2. Water Flows (downward / y+1) ─────────────────────────────────
    let water_chance = if weather == Weather::Rain { 60 } else { 30 };
    {
        let mut new_water = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::Water) {
                    let ny = y + 1;
                    if battle.arena.tile(x, ny) == Some(BattleTile::Open) {
                        if let Some(idx) = battle.arena.idx(x, ny) {
                            if terrain_roll(tick, idx, water_chance) {
                                new_water.push((x, ny));
                            }
                        }
                    }
                }
            }
        }
        for (x, y) in new_water {
            battle.arena.set_tile(x, y, BattleTile::Water);
            battle.log_message(format!("💧 Water flows to ({},{})!", x, y));
        }
    }

    // ── 3. Ice Melts ────────────────────────────────────────────────────
    if biome != ArenaBiome::Frozen {
        let ice_chance = if weather == Weather::Rain { 50 } else { 20 };
        let mut melts = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::Ice) {
                    if let Some(idx) = battle.arena.idx(x, y) {
                        if terrain_roll(tick, idx, ice_chance) {
                            melts.push((x, y));
                        }
                    }
                }
            }
        }
        for (x, y) in melts {
            battle.arena.set_tile(x, y, BattleTile::Water);
            battle.log_message(format!("🧊 Ice melts at ({},{})!", x, y));
        }
    }

    // ── 4. Lava Cools ───────────────────────────────────────────────────
    {
        let mut cooled = Vec::new();
        for i in 0..battle.arena.tiles.len() {
            if battle.arena.tiles[i] == BattleTile::Lava {
                if battle.arena.lava_timers[i] < 255 {
                    battle.arena.lava_timers[i] += 1;
                }
                if battle.arena.lava_timers[i] >= 3 && terrain_roll(tick, i, 15) {
                    let x = (i % battle.arena.width) as i32;
                    let y = (i / battle.arena.width) as i32;
                    cooled.push((x, y, i));
                }
            }
        }
        for (x, y, i) in cooled {
            battle.arena.tiles[i] = BattleTile::Scorched;
            battle.arena.lava_timers[i] = 0;
            battle.log_message(format!("🌋 Lava cools at ({},{})!", x, y));
        }
    }

    // ── 5. Thorns Grow ──────────────────────────────────────────────────
    {
        let thorns_count = battle
            .arena
            .tiles
            .iter()
            .filter(|t| **t == BattleTile::Thorns)
            .count();
        if thorns_count < 8 {
            let mut growth = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if battle.arena.tile(x, y) == Some(BattleTile::Thorns) {
                        for &(dx, dy) in &DELTAS {
                            let nx = x + dx;
                            let ny = y + dy;
                            if battle.arena.tile(nx, ny) == Some(BattleTile::Grass) {
                                if let Some(idx) = battle.arena.idx(nx, ny) {
                                    if terrain_roll(tick, idx, 25) {
                                        growth.push((nx, ny));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let remaining = 8 - thorns_count;
            for (i, (x, y)) in growth.into_iter().enumerate() {
                if i >= remaining {
                    break;
                }
                battle.arena.set_tile(x, y, BattleTile::Thorns);
                battle.log_message(format!("🌿 Thorns grow at ({},{})!", x, y));
            }
        }
    }

    // ── 6. Holy Ground Pulses ───────────────────────────────────────────
    {
        let mut purified = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::HolyGround) {
                    for &(dx, dy) in &DELTAS {
                        let nx = x + dx;
                        let ny = y + dy;
                        if let Some(t) = battle.arena.tile(nx, ny) {
                            if t == BattleTile::Scorched || t == BattleTile::BrokenGround {
                                purified.push((nx, ny));
                            }
                        }
                    }
                }
            }
        }
        for (x, y) in purified {
            battle.arena.set_tile(x, y, BattleTile::Open);
            battle.log_message(format!("✨ Holy ground purifies ({},{})!", x, y));
        }
    }

    // ── 7. Steam + Ice Interaction ──────────────────────────────────────
    if biome != ArenaBiome::Frozen {
        let mut steam_melts = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::Steam) {
                    for &(dx, dy) in &DELTAS {
                        let nx = x + dx;
                        let ny = y + dy;
                        if battle.arena.tile(nx, ny) == Some(BattleTile::Ice) {
                            steam_melts.push((nx, ny));
                        }
                    }
                }
            }
        }
        for (x, y) in steam_melts {
            battle.arena.set_tile(x, y, BattleTile::Water);
            battle.log_message(format!("♨️ Steam melts ice at ({},{})!", x, y));
        }
    }

    // ── 8. Oil Seeps ────────────────────────────────────────────────────
    {
        let mut new_oil = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::Oil) {
                    for &(dx, dy) in &DELTAS {
                        let nx = x + dx;
                        let ny = y + dy;
                        if battle.arena.tile(nx, ny) == Some(BattleTile::Water) {
                            if let Some(idx) = battle.arena.idx(nx, ny) {
                                if terrain_roll(tick, idx, 20) {
                                    new_oil.push((nx, ny));
                                }
                            }
                        }
                    }
                }
            }
        }
        for (x, y) in new_oil {
            battle.arena.set_tile(x, y, BattleTile::Oil);
            battle.log_message(format!("🛢️ Oil seeps to ({},{})!", x, y));
        }
    }

    // ── 9. Grass Regrows ────────────────────────────────────────────────
    {
        let mut regrowth = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::Open) {
                    let mut grass_neighbors = 0u32;
                    for &(dx, dy) in &DELTAS {
                        let nx = x + dx;
                        let ny = y + dy;
                        if battle.arena.tile(nx, ny) == Some(BattleTile::Grass) {
                            grass_neighbors += 1;
                        }
                    }
                    if grass_neighbors >= 2 {
                        if let Some(idx) = battle.arena.idx(x, y) {
                            if terrain_roll(tick, idx, 10) {
                                regrowth.push((x, y));
                            }
                        }
                    }
                }
            }
        }
        for (x, y) in regrowth {
            battle.arena.set_tile(x, y, BattleTile::Grass);
        }
    }

    // ── 10. Boulder Erosion ─────────────────────────────────────────────
    {
        let mut eroded = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::Boulder) {
                    let has_water_neighbor = DELTAS.iter().any(|&(dx, dy)| {
                        battle.arena.tile(x + dx, y + dy) == Some(BattleTile::Water)
                    });
                    if has_water_neighbor {
                        if let Some(idx) = battle.arena.idx(x, y) {
                            if terrain_roll(tick, idx, 10) {
                                eroded.push((x, y));
                            }
                        }
                    }
                }
            }
        }
        for (x, y) in eroded {
            battle.arena.set_tile(x, y, BattleTile::BrokenGround);
            battle.log_message(format!("🪨 Boulder erodes at ({},{})!", x, y));
        }
    }

    // ── 11. CrackedFloor Collapses (already handled by decay_cracked_floors) ──

    // ── 12. Pit Fills ───────────────────────────────────────────────────
    {
        let mut filled = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::Pit) {
                    let mut water_neighbors = 0u32;
                    for &(dx, dy) in &DELTAS {
                        let nx = x + dx;
                        let ny = y + dy;
                        if battle.arena.tile(nx, ny) == Some(BattleTile::Water) {
                            water_neighbors += 1;
                        }
                    }
                    if water_neighbors >= 2 {
                        if let Some(idx) = battle.arena.idx(x, y) {
                            if terrain_roll(tick, idx, 15) {
                                filled.push((x, y));
                            }
                        }
                    }
                }
            }
        }
        for (x, y) in filled {
            battle.arena.set_tile(x, y, BattleTile::Water);
            battle.log_message(format!("💧 Water fills the pit at ({},{})!", x, y));
        }
    }

    // ── Weather-specific terrain modifiers ───────────────────────────────
    match weather {
        Weather::Sandstorm => {
            let mut sanded = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if battle.arena.tile(x, y) == Some(BattleTile::Open) {
                        let has_sand_neighbor = DELTAS.iter().any(|&(dx, dy)| {
                            battle.arena.tile(x + dx, y + dy) == Some(BattleTile::Sand)
                        });
                        if has_sand_neighbor {
                            if let Some(idx) = battle.arena.idx(x, y) {
                                if terrain_roll(tick, idx, 20) {
                                    sanded.push((x, y));
                                }
                            }
                        }
                    }
                }
            }
            for (x, y) in sanded {
                battle.arena.set_tile(x, y, BattleTile::Sand);
                battle.log_message(format!("🏜️ Sand reclaims ({},{})!", x, y));
            }
        }
        Weather::SpiritualInk => {
            let mut inked = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if battle.arena.tile(x, y) == Some(BattleTile::InkPool) {
                        for &(dx, dy) in &DELTAS {
                            let nx = x + dx;
                            let ny = y + dy;
                            if battle.arena.tile(nx, ny) == Some(BattleTile::Open) {
                                if let Some(idx) = battle.arena.idx(nx, ny) {
                                    if terrain_roll(tick, idx, 10) {
                                        inked.push((nx, ny));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            for (x, y) in inked {
                battle.arena.set_tile(x, y, BattleTile::InkPool);
                battle.log_message(format!("🖋️ Ink pools pulse to ({},{})!", x, y));
            }
        }
        _ => {}
    }
}
