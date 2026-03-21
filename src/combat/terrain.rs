use crate::combat::action::deal_damage;
use crate::combat::{BattleTile, TacticalBattle, Weather};
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
                            if let Some(idx) = battle.unit_at(x, y) {
                                let actual = deal_damage(battle, idx, 1);
                                messages.push(format!("Fire scorches for {} damage!", actual));
                            }
                        }
                        BattleTile::Ice => {
                            battle.arena.set_tile(x, y, BattleTile::Water);
                            messages.push(format!("Ice melts at ({},{})!", x, y));
                        }
                        BattleTile::Water => {
                            battle.arena.set_steam(x, y, 2);
                            messages.push(format!("Steam erupts at ({},{})!", x, y));
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
                    messages.push(format!(
                        "Water conducts! {} is stunned!",
                        battle.units[idx].hanzi
                    ));
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
        let actual = deal_damage(battle, target_idx, 1);
        messages.push(format!("Slammed into obstacle for {} damage!", actual));
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
    }
    messages
}

pub fn explode_barrel(battle: &mut TacticalBattle, bx: i32, by: i32) -> Vec<String> {
    let mut messages = Vec::new();
    if battle.arena.tile(bx, by) != Some(BattleTile::ExplosiveBarrel) {
        return messages;
    }
    battle.arena.set_tile(bx, by, BattleTile::Scorched);
    messages.push(format!("💥 Barrel explodes at ({},{})!", bx, by));

    let deltas: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    for &(dx, dy) in &deltas {
        let nx = bx + dx;
        let ny = by + dy;
        if let Some(idx) = battle.unit_at(nx, ny) {
            let actual = deal_damage(battle, idx, 3);
            let name = if battle.units[idx].is_player() {
                "You".to_string()
            } else {
                battle.units[idx].hanzi.to_string()
            };
            messages.push(format!(
                "{} caught in explosion! (-{} HP)",
                name, actual
            ));
        }
    }

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
