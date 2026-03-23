use crate::combat::action::deal_damage;
use crate::combat::{ArenaBiome, AudioEvent, BattleTile, TacticalBattle, Weather};
use crate::status::{StatusInstance, StatusKind};
use std::collections::VecDeque;

pub enum TerrainSource {
    FireAbility,
    LightningAbility,
    Earthquake,
}

pub fn apply_terrain_interactions(
    battle: &mut TacticalBattle,
    source: TerrainSource,
    affected_tiles: &[(i32, i32)],
) -> Vec<String> {
    let mut messages = Vec::new();

    match source {
        TerrainSource::FireAbility => {
            for &(x, y) in affected_tiles {
                if let Some(tile) = battle.arena.tile(x, y) {
                    match tile {
                        BattleTile::WiringPanel | BattleTile::ElectrifiedWire => {
                            battle.arena.set_tile(x, y, BattleTile::BlastMark);
                            messages.push(format!("Wiring panel sparks at ({},{})!", x, y));
                            // Burning wiring panel creates VentSteam (smoke) blocking LOS
                            let deltas: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                            for &(sdx, sdy) in &deltas {
                                let sx = x + sdx;
                                let sy = y + sdy;
                                if let Some(st) = battle.arena.tile(sx, sy) {
                                    if st == BattleTile::MetalFloor || st == BattleTile::WiringPanel {
                                        battle.arena.set_steam(sx, sy, 2);
                                    }
                                }
                            }
                            messages.push(format!("💨 Smoke billows from burning wiring!"));
                            if let Some(idx) = battle.unit_at(x, y) {
                                let actual = deal_damage(battle, idx, 1);
                                messages.push(format!("Fire scorches for {} damage!", actual));
                            }
                        }
                        BattleTile::FrozenCoolant => {
                            battle.arena.set_tile(x, y, BattleTile::CoolantPool);
                            messages.push(format!("Frozen coolant melts at ({},{})!", x, y));
                            // CoolantPool+FrozenCoolant cascade: melted coolant melts adjacent frozen coolant
                            let deltas: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                            for &(cdx, cdy) in &deltas {
                                let cx = x + cdx;
                                let cy = y + cdy;
                                if battle.arena.tile(cx, cy) == Some(BattleTile::FrozenCoolant) {
                                    battle.arena.set_tile(cx, cy, BattleTile::CoolantPool);
                                    messages.push(format!("🌊 Frozen coolant melts into coolant at ({},{})! Flood cascade!", cx, cy));
                                }
                            }
                        }
                        BattleTile::CoolantPool => {
                            battle.arena.set_steam(x, y, 2);
                            messages.push(format!("Steam erupts at ({},{})!", x, y));
                        }
                        BattleTile::Lubricant => {
                            // Lubricant ignites into 3×3 fire — chain reaction spreads to all connected lubricant!
                            battle.arena.set_tile(x, y, BattleTile::BlastMark);
                            messages.push(format!("Lubricant ignites at ({},{})!", x, y));
                            let deltas_3x3: [(i32, i32); 8] = [
                                (-1, -1), (0, -1), (1, -1),
                                (-1, 0),           (1, 0),
                                (-1, 1),  (0, 1),  (1, 1),
                            ];
                            // Collect chain-reaction lubricant tiles beyond the initial 3x3
                            let mut chain_lubricant = Vec::new();
                            for &(adx, ady) in &deltas_3x3 {
                                let ax = x + adx;
                                let ay = y + ady;
                                if let Some(at) = battle.arena.tile(ax, ay) {
                                    if at == BattleTile::Lubricant {
                                        battle.arena.set_tile(ax, ay, BattleTile::BlastMark);
                                        // Check for further lubricant chain reaction
                                        for &(cdx, cdy) in &[(-1i32,0),(1,0),(0,-1i32),(0,1)] {
                                            let cx = ax + cdx;
                                            let cy = ay + cdy;
                                            if battle.arena.tile(cx, cy) == Some(BattleTile::Lubricant) {
                                                chain_lubricant.push((cx, cy));
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
                                        "Lubricant fire burns {} for {} damage!",
                                        battle.units[aidx].hanzi, actual
                                    ));
                                }
                            }
                            // Chain reaction: ignite lubricant tiles beyond 3x3 blast
                            for (cx, cy) in chain_lubricant {
                                if battle.arena.tile(cx, cy) == Some(BattleTile::Lubricant) {
                                    battle.arena.set_tile(cx, cy, BattleTile::BlastMark);
                                    messages.push(format!("🔥 Lubricant chain reaction at ({},{})!", cx, cy));
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
                                    "Lubricant fire burns {} for {} damage!",
                                    battle.units[cidx].hanzi, actual
                                ));
                            }
                        }
                        BattleTile::FuelCanister => {
                            let mut canister_msgs = explode_barrel(battle, x, y);
                            messages.append(&mut canister_msgs);
                        }
                        _ => {}
                    }
                }
            }
        }
        TerrainSource::LightningAbility => {
            let mut stun_targets = Vec::new();
            for &(x, y) in affected_tiles {
                if battle.arena.tile(x, y) == Some(BattleTile::CoolantPool) {
                    let connected = flood_connected_water(&battle.arena, x, y);
                    // In coolant leak, lightning chains 1 extra tile beyond coolant
                    let expanded = if battle.weather == Weather::CoolantLeak {
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
                            "Coolant conducts! {} is stunned!",
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
                        BattleTile::MetalFloor | BattleTile::WiringPanel | BattleTile::Debris
                    ) {
                        battle.arena.set_tile(x, y, BattleTile::DamagedPlating);
                    }
                    // Earthquake + CargoCrate → Crates shatter into DamagedPlating
                    if tile == BattleTile::CargoCrate {
                        battle.arena.set_tile(x, y, BattleTile::DamagedPlating);
                        messages.push(format!("🪨 Cargo crate shatters at ({},{})!", x, y));
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
                if !visited[i] && arena.tiles[i] == BattleTile::CoolantPool {
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
        .unwrap_or(BattleTile::MetalFloor);

    if dest_tile == BattleTile::CoverBarrier || dest_tile == BattleTile::BreachedFloor {
        // KnockbackStrike + Wall/CargoCrate crush synergy: +2 bonus damage
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

    if dest_tile == BattleTile::CargoCrate {
        // KnockbackStrike + CargoCrate crush synergy: +2 bonus damage
        let has_knockback_equip = battle.player_equip_effects.iter().any(|e| {
            matches!(e, crate::player::EquipEffect::KnockbackStrike)
        });
        let crush_dmg = if has_knockback_equip { 3 } else { 1 };
        let actual = deal_damage(battle, target_idx, crush_dmg);
        if has_knockback_equip {
            messages.push(format!(
                "💥 Crushed into cargo crate for {} damage! (KnockbackStrike bonus!)",
                actual
            ));
        } else {
            messages.push(format!("Slammed into cargo crate for {} damage!", actual));
        }
        return messages;
    }

    if dest_tile == BattleTile::FuelCanister {
        let actual = deal_damage(battle, target_idx, 1);
        messages.push(format!("Slammed into a canister for {} damage!", actual));
        let mut canister_msgs = explode_barrel(battle, dest_x, dest_y);
        messages.append(&mut canister_msgs);
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
        BattleTile::CoolantPool => {
            use crate::status::{StatusInstance, StatusKind};
            battle.units[target_idx]
                .statuses
                .push(StatusInstance::new(StatusKind::Confused, 1));
            messages.push("Knocked into coolant — slowed!".to_string());
        }
        BattleTile::PlasmaPool => {
            let actual = deal_damage(battle, target_idx, 2);
            messages.push(format!(
                "{} knocked into plasma for {} damage!",
                battle.units[target_idx].hanzi, actual
            ));
        }
        BattleTile::BlastMark => {
            let actual = deal_damage(battle, target_idx, 1);
            messages.push(format!(
                "{} knocked onto blast mark for {} damage!",
                battle.units[target_idx].hanzi, actual
            ));
        }
        BattleTile::ElectrifiedWire => {
            let actual = deal_damage(battle, target_idx, 1);
            messages.push(format!(
                "{} knocked into electrified wire for {} damage!",
                battle.units[target_idx].hanzi, actual
            ));
        }
        BattleTile::FrozenCoolant => {
            messages.push(format!("{} slides on frozen coolant!", battle.units[target_idx].hanzi));
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
        BattleTile::Lubricant => {
            messages.push(format!("{} slides on lubricant!", battle.units[target_idx].hanzi));
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
        BattleTile::FuelCanister => {
            messages.push(format!(
                "{} knocked into a canister!",
                battle.units[target_idx].hanzi
            ));
            let mut canister_msgs = explode_barrel(battle, dest_x, dest_y);
            messages.append(&mut canister_msgs);
        }
        BattleTile::MineTile | BattleTile::MineTileRevealed => {
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
        if tile == Some(BattleTile::BlastMark) {
            let actual = deal_damage(battle, i, 1);
            let name = if battle.units[i].is_player() {
                "You".to_string()
            } else {
                battle.units[i].hanzi.to_string()
            };
            messages.push(format!(
                "{} burns on blast mark! (-{} HP)",
                name, actual
            ));
        }
        if tile == Some(BattleTile::PlasmaPool) {
            let actual = deal_damage(battle, i, 2);
            let name = if battle.units[i].is_player() {
                "You".to_string()
            } else {
                battle.units[i].hanzi.to_string()
            };
            messages.push(format!("{} sears in plasma! (-{} HP)", name, actual));
        }
        if tile == Some(BattleTile::SteamVentActive) {
            let actual = deal_damage(battle, i, 1);
            let name = if battle.units[i].is_player() {
                "You".to_string()
            } else {
                battle.units[i].hanzi.to_string()
            };
            messages.push(format!(
                "♨ {} scalded by steam vent! (-{} HP)",
                name, actual
            ));
        }
        // PowerDrain: damages player standing on it
        if tile == Some(BattleTile::PowerDrain) && battle.units[i].is_player() {
            battle.units[i].hp -= 1;
            messages.push("⚫ The power drain saps your vitality! (-1 HP)".to_string());
        }
        // ShieldZone: heal units standing on it
        if tile == Some(BattleTile::ShieldZone) {
            let ux = battle.units[i].x;
            let uy = battle.units[i].y;
            // Heal amount stored in steam_timers for ShieldZone tiles
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
                    "✨ {} healed {} HP by shield zone!",
                    name, healed
                ));
            }
        }
    }
    messages
}

pub fn explode_barrel(battle: &mut TacticalBattle, bx: i32, by: i32) -> Vec<String> {
    let mut messages = Vec::new();
    if battle.arena.tile(bx, by) != Some(BattleTile::FuelCanister) {
        return messages;
    }
    // Check if canister is on or adjacent to PlasmaPool → larger 2-tile radius explosion
    let on_plasma = {
        let deltas: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        deltas.iter().any(|&(dx, dy)| {
            battle.arena.tile(bx + dx, by + dy) == Some(BattleTile::PlasmaPool)
        }) || false // canister tile itself was FuelCanister, can't be PlasmaPool
    };
    battle.arena.set_tile(bx, by, BattleTile::BlastMark);

    let radius = if on_plasma { 2 } else { 1 };
    if on_plasma {
        messages.push(format!("💥🌋 Fuel canister explodes near plasma at ({},{})! Massive blast!", bx, by));
    } else {
        messages.push(format!("💥 Fuel canister explodes at ({},{})!", bx, by));
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
        if battle.arena.tile(nx, ny) == Some(BattleTile::FuelCanister) {
            chain_targets.push((nx, ny));
        }
    }
    for (cx, cy) in chain_targets {
        messages.push("Chain reaction!".to_string());
        battle.audio_events.push(AudioEvent::ChainExplosion);
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
    if tile != Some(BattleTile::MineTile) && tile != Some(BattleTile::MineTileRevealed) {
        return messages;
    }
    battle.arena.set_tile(tx, ty, BattleTile::MineTileRevealed);
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
    if tile == Some(BattleTile::WeakenedPlating) {
        battle.arena.set_tile(x, y, BattleTile::DamagedFloor);
        messages.push("The floor cracks beneath your feet!".to_string());
    } else if tile == Some(BattleTile::DamagedFloor) {
        battle.arena.set_tile(x, y, BattleTile::BreachedFloor);
        messages.push("The plating collapses into a breach!".to_string());
    }
    messages
}

pub fn decay_cracked_floors(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();
    let w = battle.arena.width as i32;
    let h = battle.arena.height as i32;
    for y in 0..h {
        for x in 0..w {
            if battle.arena.tile(x, y) == Some(BattleTile::DamagedFloor) {
                if battle.unit_at(x, y).is_none() {
                    battle.arena.set_tile(x, y, BattleTile::BreachedFloor);
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
    if weather == Weather::SmokeScreen {
        // Still age plasma timers even in smoke
        for i in 0..battle.arena.tiles.len() {
            if battle.arena.tiles[i] == BattleTile::PlasmaPool && battle.arena.lava_timers[i] < 255 {
                battle.arena.lava_timers[i] += 1;
            }
        }
        return;
    }

    // ── 1. Fire Spreads ─────────────────────────────────────────────────
    let fire_chance = if weather == Weather::CoolantLeak { 20 } else { 40 };
    {
        let mut ignitions = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::BlastMark) {
                    for &(dx, dy) in &DELTAS {
                        let nx = x + dx;
                        let ny = y + dy;
                        if battle.arena.tile(nx, ny) == Some(BattleTile::WiringPanel) {
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
            battle.arena.set_tile(x, y, BattleTile::BlastMark);
            battle.log_message(format!("🔥 Fire spreads to ({},{})!", x, y));
            if let Some(uid) = battle.unit_at(x, y) {
                battle.units[uid]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 2));
            }
        }
    }

    // ── 1b. OilSlick Ignition ──────────────────────────────────────────────
    {
        let mut oil_ignitions = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::OilSlick) {
                    for &(dx, dy) in &DELTAS {
                        let nx = x + dx;
                        let ny = y + dy;
                        if battle.arena.tile(nx, ny) == Some(BattleTile::BlastMark) {
                            if let Some(idx) = battle.arena.idx(x, y) {
                                if terrain_roll(tick, idx, 30) {
                                    oil_ignitions.push((x, y));
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
        for (x, y) in oil_ignitions {
            battle.arena.set_tile(x, y, BattleTile::BlastMark);
            battle.log_message(format!("🔥🛢 Oil slick ignites at ({},{})!", x, y));
            battle.audio_events.push(AudioEvent::OilIgnition);
            if let Some(uid) = battle.unit_at(x, y) {
                battle.units[uid]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 2));
            }
        }
    }

    // ── 2. Coolant Flows (downward / y+1) ─────────────────────────────────
    let coolant_chance = if weather == Weather::CoolantLeak { 60 } else { 30 };
    {
        let mut new_coolant = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::CoolantPool) {
                    let ny = y + 1;
                    if battle.arena.tile(x, ny) == Some(BattleTile::MetalFloor) {
                        if let Some(idx) = battle.arena.idx(x, ny) {
                            if terrain_roll(tick, idx, coolant_chance) {
                                new_coolant.push((x, ny));
                            }
                        }
                    }
                }
            }
        }
        for (x, y) in new_coolant {
            battle.arena.set_tile(x, y, BattleTile::CoolantPool);
            battle.log_message(format!("💧 Coolant flows to ({},{})!", x, y));
        }
    }

    // ── 3. Frozen Coolant Melts ────────────────────────────────────────────────────
    if biome != ArenaBiome::CryoBay {
        let melt_chance = if weather == Weather::CoolantLeak { 50 } else { 20 };
        let mut melts = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::FrozenCoolant) {
                    if let Some(idx) = battle.arena.idx(x, y) {
                        if terrain_roll(tick, idx, melt_chance) {
                            melts.push((x, y));
                        }
                    }
                }
            }
        }
        for (x, y) in melts {
            battle.arena.set_tile(x, y, BattleTile::CoolantPool);
            battle.log_message(format!("🧊 Frozen coolant melts at ({},{})!", x, y));
        }
    }

    // ── 4. Plasma Cools ───────────────────────────────────────────────────
    {
        let mut cooled = Vec::new();
        for i in 0..battle.arena.tiles.len() {
            if battle.arena.tiles[i] == BattleTile::PlasmaPool {
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
            battle.arena.tiles[i] = BattleTile::BlastMark;
            battle.arena.lava_timers[i] = 0;
            battle.log_message(format!("🌋 Plasma cools at ({},{})!", x, y));
        }
    }

    // ── 5. Electrified Wires Spread ──────────────────────────────────────────────────
    {
        let wire_count = battle
            .arena
            .tiles
            .iter()
            .filter(|t| **t == BattleTile::ElectrifiedWire)
            .count();
        if wire_count < 8 {
            let mut growth = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if battle.arena.tile(x, y) == Some(BattleTile::ElectrifiedWire) {
                        for &(dx, dy) in &DELTAS {
                            let nx = x + dx;
                            let ny = y + dy;
                            if battle.arena.tile(nx, ny) == Some(BattleTile::WiringPanel) {
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
            let remaining = 8 - wire_count;
            for (i, (x, y)) in growth.into_iter().enumerate() {
                if i >= remaining {
                    break;
                }
                battle.arena.set_tile(x, y, BattleTile::ElectrifiedWire);
                battle.log_message(format!("⚡ Electrified wires spread at ({},{})!", x, y));
            }
        }
    }

    // ── 6. Shield Zone Pulses ───────────────────────────────────────────
    {
        let mut purified = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::ShieldZone) {
                    for &(dx, dy) in &DELTAS {
                        let nx = x + dx;
                        let ny = y + dy;
                        if let Some(t) = battle.arena.tile(nx, ny) {
                            if t == BattleTile::BlastMark || t == BattleTile::DamagedPlating {
                                purified.push((nx, ny));
                            }
                        }
                    }
                }
            }
        }
        for (x, y) in purified {
            battle.arena.set_tile(x, y, BattleTile::MetalFloor);
            battle.log_message(format!("✨ Shield zone purifies ({},{})!", x, y));
        }
    }

    // ── 7. VentSteam + FrozenCoolant Interaction ──────────────────────────────────────
    if biome != ArenaBiome::CryoBay {
        let mut steam_melts = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::VentSteam) {
                    for &(dx, dy) in &DELTAS {
                        let nx = x + dx;
                        let ny = y + dy;
                        if battle.arena.tile(nx, ny) == Some(BattleTile::FrozenCoolant) {
                            steam_melts.push((nx, ny));
                        }
                    }
                }
            }
        }
        for (x, y) in steam_melts {
            battle.arena.set_tile(x, y, BattleTile::CoolantPool);
            battle.log_message(format!("♨️ Vent steam melts frozen coolant at ({},{})!", x, y));
        }
    }

    // ── 8. Lubricant Seeps ────────────────────────────────────────────────────
    {
        let mut new_lubricant = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::Lubricant) {
                    for &(dx, dy) in &DELTAS {
                        let nx = x + dx;
                        let ny = y + dy;
                        if battle.arena.tile(nx, ny) == Some(BattleTile::CoolantPool) {
                            if let Some(idx) = battle.arena.idx(nx, ny) {
                                if terrain_roll(tick, idx, 20) {
                                    new_lubricant.push((nx, ny));
                                }
                            }
                        }
                    }
                }
            }
        }
        for (x, y) in new_lubricant {
            battle.arena.set_tile(x, y, BattleTile::Lubricant);
            battle.log_message(format!("🛢️ Lubricant seeps to ({},{})!", x, y));
        }
    }

    // ── 9. Wiring Regrows ────────────────────────────────────────────────
    {
        let mut regrowth = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::MetalFloor) {
                    let mut wiring_neighbors = 0u32;
                    for &(dx, dy) in &DELTAS {
                        let nx = x + dx;
                        let ny = y + dy;
                        if battle.arena.tile(nx, ny) == Some(BattleTile::WiringPanel) {
                            wiring_neighbors += 1;
                        }
                    }
                    if wiring_neighbors >= 2 {
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
            battle.arena.set_tile(x, y, BattleTile::WiringPanel);
        }
    }

    // ── 10. Cargo Crate Corrosion ─────────────────────────────────────────────
    {
        let mut eroded = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::CargoCrate) {
                    let has_coolant_neighbor = DELTAS.iter().any(|&(dx, dy)| {
                        battle.arena.tile(x + dx, y + dy) == Some(BattleTile::CoolantPool)
                    });
                    if has_coolant_neighbor {
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
            battle.arena.set_tile(x, y, BattleTile::DamagedPlating);
            battle.log_message(format!("🪨 Cargo crate corrodes at ({},{})!", x, y));
        }
    }

    // ── 11. CrackedFloor Collapses (already handled by decay_cracked_floors) ──

    // ── 12. Breach Fills ───────────────────────────────────────────────────
    {
        let mut filled = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if battle.arena.tile(x, y) == Some(BattleTile::BreachedFloor) {
                    let mut coolant_neighbors = 0u32;
                    for &(dx, dy) in &DELTAS {
                        let nx = x + dx;
                        let ny = y + dy;
                        if battle.arena.tile(nx, ny) == Some(BattleTile::CoolantPool) {
                            coolant_neighbors += 1;
                        }
                    }
                    if coolant_neighbors >= 2 {
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
            battle.arena.set_tile(x, y, BattleTile::CoolantPool);
            battle.log_message(format!("💧 Coolant fills the breach at ({},{})!", x, y));
        }
    }

    // ── 13. Steam Vent Toggling ───────────────────────────────────────────
    if tick % 2 == 0 {
        let mut toggled_active = false;
        let mut toggled_inactive = false;
        for i in 0..battle.arena.tiles.len() {
            match battle.arena.tiles[i] {
                BattleTile::SteamVentActive => {
                    battle.arena.tiles[i] = BattleTile::SteamVentInactive;
                    toggled_inactive = true;
                }
                BattleTile::SteamVentInactive => {
                    battle.arena.tiles[i] = BattleTile::SteamVentActive;
                    toggled_active = true;
                }
                _ => {}
            }
        }
        if toggled_active {
            battle.log_message("♨ Steam vents activate!");
            battle.audio_events.push(AudioEvent::SteamVent);
        }
        if toggled_inactive {
            battle.log_message("♨ Steam vents deactivate!");
            battle.audio_events.push(AudioEvent::SteamVent);
        }
    }

    // ── Weather-specific terrain modifiers ───────────────────────────────
    match weather {
        Weather::DebrisStorm => {
            let mut debris_spread = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if battle.arena.tile(x, y) == Some(BattleTile::MetalFloor) {
                        let has_debris_neighbor = DELTAS.iter().any(|&(dx, dy)| {
                            battle.arena.tile(x + dx, y + dy) == Some(BattleTile::Debris)
                        });
                        if has_debris_neighbor {
                            if let Some(idx) = battle.arena.idx(x, y) {
                                if terrain_roll(tick, idx, 20) {
                                    debris_spread.push((x, y));
                                }
                            }
                        }
                    }
                }
            }
            for (x, y) in debris_spread {
                battle.arena.set_tile(x, y, BattleTile::Debris);
                battle.log_message(format!("🏜️ Debris reclaims ({},{})!", x, y));
            }
        }
        Weather::EnergyFlux => {
            let mut oil_spread = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if battle.arena.tile(x, y) == Some(BattleTile::OilSlick) {
                        for &(dx, dy) in &DELTAS {
                            let nx = x + dx;
                            let ny = y + dy;
                            if battle.arena.tile(nx, ny) == Some(BattleTile::MetalFloor) {
                                if let Some(idx) = battle.arena.idx(nx, ny) {
                                    if terrain_roll(tick, idx, 10) {
                                        oil_spread.push((nx, ny));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            for (x, y) in oil_spread {
                battle.arena.set_tile(x, y, BattleTile::OilSlick);
                battle.log_message(format!("🖋️ Oil slicks pulse to ({},{})!", x, y));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
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
}
