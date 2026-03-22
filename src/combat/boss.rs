use crate::combat::action::move_unit;
use crate::combat::ai::{step_away, step_toward};
use crate::combat::grid::manhattan;
use crate::combat::{BattleTile, BattleUnit, Direction, TacticalBattle, UnitKind};
use crate::enemy::{AiBehavior, BossKind};

/// Try to perform a boss-specific action. Returns Some(log_message) if the boss
/// acted (skipping normal AI), or None to fall through to normal AI.
pub fn boss_action(battle: &mut TacticalBattle, unit_idx: usize) -> Option<String> {
    let boss_kind = battle.units[unit_idx].boss_kind?;
    match boss_kind {
        BossKind::PirateCaptain => pirate_captain_action(battle, unit_idx),
        BossKind::HiveQueen => hive_queen_action(battle, unit_idx),
        BossKind::RogueAICore => rogue_ai_core_action(battle, unit_idx),
        BossKind::VoidEntity => void_entity_action(battle, unit_idx),
        BossKind::AncientGuardian => ancient_guardian_action(battle, unit_idx),
        BossKind::DriftLeviathan => drift_leviathan_action(battle, unit_idx),
    }
}

// ── Pirate Captain ────────────────────────────────────────────────────────────
// Deploys shield generator tiles that block movement. Player must type "men2" to destroy.

fn pirate_captain_action(battle: &mut TacticalBattle, unit_idx: usize) -> Option<String> {
    let turn = battle.turn_number;
    if turn % 3 != 0 || battle.ward_tiles.len() >= 4 {
        return None;
    }

    let bx = battle.units[unit_idx].x;
    let by = battle.units[unit_idx].y;
    let px = battle.units[0].x;
    let py = battle.units[0].y;

    let mid_x = (bx + px) / 2;
    let mid_y = (by + py) / 2;

    let candidates = [
        (mid_x, mid_y),
        (mid_x + 1, mid_y),
        (mid_x - 1, mid_y),
        (mid_x, mid_y + 1),
        (mid_x, mid_y - 1),
    ];

    for (wx, wy) in candidates {
        if !battle.arena.in_bounds(wx, wy) {
            continue;
        }
        if battle.arena.tile(wx, wy) != Some(BattleTile::MetalFloor) {
            continue;
        }
        if battle.unit_at(wx, wy).is_some() {
            continue;
        }
        if battle.ward_tiles.contains(&(wx, wy)) {
            continue;
        }
        battle.arena.set_tile(wx, wy, BattleTile::CoverBarrier);
        battle.ward_tiles.push((wx, wy));
        return Some(format!("Pirate Captain deploys a shield generator at ({},{})!", wx, wy));
    }
    None
}

/// Called when player moves onto a ward tile. Returns true if ward was destroyed.
pub fn try_destroy_ward(battle: &mut TacticalBattle, x: i32, y: i32) -> bool {
    if let Some(pos) = battle
        .ward_tiles
        .iter()
        .position(|&(wx, wy)| wx == x && wy == y)
    {
        battle.ward_tiles.remove(pos);
        battle.arena.set_tile(x, y, BattleTile::MetalFloor);
        true
    } else {
        false
    }
}

// ── Hive Queen ───────────────────────────────────────────────────────────────
// At 50% HP switches to Retreat AI. Sentence duel handled by game.rs.

fn hive_queen_action(battle: &mut TacticalBattle, unit_idx: usize) -> Option<String> {
    let unit = &battle.units[unit_idx];
    let hp_ratio = unit.hp as f64 / unit.max_hp.max(1) as f64;

    if hp_ratio <= 0.5 {
        let px = battle.units[0].x;
        let py = battle.units[0].y;
        let fx = battle.units[unit_idx].x;
        let fy = battle.units[unit_idx].y;
        let dist = manhattan(fx, fy, px, py);

        if dist <= 1 {
            if let Some((nx, ny)) = step_away(battle, fx, fy, px, py) {
                move_unit(battle, unit_idx, nx, ny);
                return Some("Hive Queen retreats, emitting pheromone signals...".to_string());
            }
        }
        return Some("Hive Queen studies you cautiously...".to_string());
    }
    None
}

// ── Rogue AI Core ────────────────────────────────────────────────────────────
// Adapts resistance to last system hacked. Terrain shifts every 4th turn.

fn rogue_ai_core_action(battle: &mut TacticalBattle, _unit_idx: usize) -> Option<String> {
    let turn = battle.turn_number;

    if turn % 4 == 0 {
        let seed = turn as u64;
        let size = battle.arena.width as i32;
        let mut shifted = 0;
        for i in 0..3 {
            let hash = seed
                .wrapping_mul(2654435761)
                .wrapping_add(i as u64)
                .wrapping_mul(2246822519);
            let x = ((hash >> 16) % size as u64) as i32;
            let y = (1 + (hash >> 8) % (size as u64 - 2)) as i32;

            if !battle.arena.in_bounds(x, y) {
                continue;
            }
            if battle.unit_at(x, y).is_some() {
                continue;
            }
            let current = battle.arena.tile(x, y).unwrap_or(BattleTile::MetalFloor);
            if current == BattleTile::CoverBarrier {
                continue;
            }
            let new_tile = match hash % 4 {
                0 => BattleTile::CoolantPool,
                1 => BattleTile::FrozenCoolant,
                2 => BattleTile::OilSlick,
                _ => BattleTile::WiringPanel,
            };
            battle.arena.set_tile(x, y, new_tile);
            shifted += 1;
        }
        if shifted > 0 {
            return Some("Rogue AI Core reconfigures the environment!".to_string());
        }
    }
    None
}

/// Returns damage multiplier for spells against Rogue AI Core (0.5 if resisted).
pub fn elementalist_resistance(
    battle: &TacticalBattle,
    target_idx: usize,
    spell_school: &str,
) -> f64 {
    if let Some(BossKind::RogueAICore) = battle.units[target_idx].boss_kind {
        if let Some(resisted) = battle.last_spell_school {
            if resisted == spell_school {
                return 0.5;
            }
        }
    }
    1.0
}

// ── Void Entity ──────────────────────────────────────────────────────────────
// Spawns decoy copies every 5 turns. Decoys have same hanzi but different pinyin.

fn void_entity_action(battle: &mut TacticalBattle, unit_idx: usize) -> Option<String> {
    let turn = battle.turn_number;
    if turn % 5 != 0 {
        return None;
    }

    let decoy_count = battle
        .units
        .iter()
        .filter(|u| u.is_decoy && u.alive)
        .count();
    if decoy_count >= 2 {
        return None;
    }

    let bx = battle.units[unit_idx].x;
    let by = battle.units[unit_idx].y;
    let boss_hanzi = battle.units[unit_idx].hanzi;

    let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    for (dx, dy) in deltas {
        let nx = bx + dx;
        let ny = by + dy;
        if !battle.arena.in_bounds(nx, ny) {
            continue;
        }
        if battle
            .arena
            .tile(nx, ny)
            .map(|t| !t.is_walkable())
            .unwrap_or(true)
        {
            continue;
        }
        if battle.unit_at(nx, ny).is_some() {
            continue;
        }

        let decoy = BattleUnit {
            kind: UnitKind::Enemy(999), // sentinel value — decoys don't map to real enemies
            x: nx,
            y: ny,
            facing: Direction::South,
            hanzi: boss_hanzi,
            pinyin: "???",
            speed: 3,
            movement: 2,
            stored_movement: 0,
            hp: 4,
            max_hp: 4,
            damage: 1,
            defending: false,
            alive: true,
            ai: AiBehavior::Chase,
            radical_actions: Vec::new(),
            statuses: Vec::new(),
            stunned: false,
            radical_armor: 0,
            radical_counter: false,
            marked_extra_damage: 0,
            thorn_armor_turns: 0,
            radical_dodge: false,
            radical_multiply: false,
            fortify_stacks: 0,
            boss_kind: None,
            is_decoy: true,
            word_group: None,
            word_group_order: 0,
            wuxing_element: None,
            intent: None,
            mastery_tier: 0,
            charge_remaining: None,
            synergy_damage_bonus: 0,
            elemental_resonance: false,
            sacrifice_bonus_damage: 0,
            sacrifice_bonus_turns: 0,
            momentum: 0,
            last_move_dir: None,
        };

        battle.units.push(decoy);
        battle.turn_queue = crate::combat::turn::build_turn_queue(&battle.units);
        if battle.turn_queue_pos >= battle.turn_queue.len() {
            battle.turn_queue_pos = 0;
        }

        return Some("Void Entity warps a decoy into existence! Which is real?".to_string());
    }
    None
}

// ── Ancient Guardian ──────────────────────────────────────────────────────────
// Creates OilSlick terrain every 2nd turn. +2 damage bonus on OilSlick.
// Holographic trial at 50% HP handled by game.rs.

fn ancient_guardian_action(battle: &mut TacticalBattle, unit_idx: usize) -> Option<String> {
    let turn = battle.turn_number;
    if turn % 2 != 0 {
        return None;
    }

    let bx = battle.units[unit_idx].x;
    let by = battle.units[unit_idx].y;

    let targets = [(bx - 1, by), (bx + 1, by), (bx, by - 1), (bx, by + 1)];

    let mut placed = 0;
    for (x, y) in targets {
        if !battle.arena.in_bounds(x, y) {
            continue;
        }
        let tile = battle.arena.tile(x, y).unwrap_or(BattleTile::MetalFloor);
        if tile == BattleTile::MetalFloor || tile == BattleTile::WiringPanel {
            battle.arena.set_tile(x, y, BattleTile::OilSlick);
            placed += 1;
        }
    }

    if placed > 0 {
        Some("Ancient Guardian floods the area with energy fields!".to_string())
    } else {
        None
    }
}

/// Ancient Guardian damage bonus: +2 when standing on OilSlick.
pub fn ink_sage_bonus(battle: &TacticalBattle, unit_idx: usize) -> i32 {
    if let Some(BossKind::AncientGuardian) = battle.units[unit_idx].boss_kind {
        let tile = battle
            .arena
            .tile(battle.units[unit_idx].x, battle.units[unit_idx].y);
        if tile == Some(BattleTile::OilSlick) {
            return 2;
        }
    }
    0
}

// ── Drift Leviathan ──────────────────────────────────────────────────────────
// Absorbs a module on wrong answer. Absorbed modules appear as grid pickups.
// Module absorption handled in input.rs (resolve_basic_attack on wrong answer).
// This function handles the leviathan's own tactical behavior.

fn drift_leviathan_action(battle: &mut TacticalBattle, unit_idx: usize) -> Option<String> {
    if !battle.stolen_spells.is_empty() {
        let (sx, sy, _, _, _) = battle.stolen_spells[0];
        let fx = battle.units[unit_idx].x;
        let fy = battle.units[unit_idx].y;
        let dist_to_spell = manhattan(fx, fy, sx, sy);
        let dist_to_player = manhattan(fx, fy, battle.units[0].x, battle.units[0].y);

        if dist_to_spell > 1 && dist_to_player > dist_to_spell {
            if let Some((nx, ny)) = step_toward(battle, fx, fy, sx, sy) {
                move_unit(battle, unit_idx, nx, ny);
                return Some("Drift Leviathan guards the absorbed module!".to_string());
            }
        }
    }
    None
}

/// Absorb a random module from the player's available modules and place it on the grid.
/// Returns a log message if a module was absorbed.
pub fn steal_spell(battle: &mut TacticalBattle, unit_idx: usize) -> Option<String> {
    if battle.units[unit_idx].boss_kind != Some(BossKind::DriftLeviathan) {
        return None;
    }
    if battle.available_spells.is_empty() {
        return None;
    }

    let spell_idx = battle.available_spells.len() - 1;
    let (hanzi, pinyin, effect) = battle.available_spells.remove(spell_idx);

    // Place it near the thief
    let tx = battle.units[unit_idx].x;
    let ty = battle.units[unit_idx].y;
    let deltas = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1)];
    let mut placed = false;
    for (dx, dy) in deltas {
        let nx = tx + dx;
        let ny = ty + dy;
        if battle.arena.in_bounds(nx, ny)
            && battle
                .arena
                .tile(nx, ny)
                .map(|t| t.is_walkable())
                .unwrap_or(false)
            && battle.unit_at(nx, ny).is_none()
        {
            battle.stolen_spells.push((nx, ny, hanzi, pinyin, effect));
            placed = true;
            break;
        }
    }
    if !placed {
        // Place at thief's own position as fallback
        battle.stolen_spells.push((tx, ty, hanzi, pinyin, effect));
    }

    Some(format!("Drift Leviathan absorbs {}!", hanzi))
}

/// Try to pick up a stolen spell at position (x, y). Returns log message if picked up.
pub fn try_pickup_stolen_spell(battle: &mut TacticalBattle, x: i32, y: i32) -> Option<String> {
    let pos = battle
        .stolen_spells
        .iter()
        .position(|&(sx, sy, _, _, _)| sx == x && sy == y)?;
    let (_, _, hanzi, pinyin, effect) = battle.stolen_spells.remove(pos);
    battle.available_spells.push((hanzi, pinyin, effect));
    Some(format!("Recovered stolen module {}!", hanzi))
}
