use crate::combat::action::deal_damage;
use crate::combat::{BattleTile, TacticalBattle};
use crate::radical::SpellEffect;
use std::collections::VecDeque;

pub enum TerrainSource {
    FireSpell,
    LightningSpell,
    Earthquake,
}

impl TerrainSource {
    pub fn from_spell_effect(effect: &SpellEffect) -> Option<Self> {
        match effect {
            SpellEffect::FireAoe(_) => Some(TerrainSource::FireSpell),
            SpellEffect::Stun => Some(TerrainSource::LightningSpell),
            _ => None,
        }
    }
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
                        BattleTile::Grass => {
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
                    for (wx, wy) in connected {
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
                    if matches!(tile, BattleTile::Open | BattleTile::Grass) {
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

    if dest_tile == BattleTile::Obstacle {
        let actual = deal_damage(battle, target_idx, 1);
        messages.push(format!("Slammed into obstacle for {} damage!", actual));
        return messages;
    }

    if battle.unit_at(dest_x, dest_y).is_some() {
        messages.push("Knockback blocked by another unit!".to_string());
        return messages;
    }

    battle.units[target_idx].x = dest_x;
    battle.units[target_idx].y = dest_y;

    if dest_tile == BattleTile::Water {
        use crate::status::{StatusInstance, StatusKind};
        battle.units[target_idx]
            .statuses
            .push(StatusInstance::new(StatusKind::Confused, 1));
        messages.push("Knocked into water — slowed!".to_string());
    } else {
        messages.push(format!("{} knocked back!", battle.units[target_idx].hanzi));
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
    }
    messages
}
