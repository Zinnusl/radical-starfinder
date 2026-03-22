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
        BossKind::Gatekeeper => gatekeeper_action(battle, unit_idx),
        BossKind::Scholar => scholar_action(battle, unit_idx),
        BossKind::Elementalist => elementalist_action(battle, unit_idx),
        BossKind::MimicKing => mimic_king_action(battle, unit_idx),
        BossKind::InkSage => ink_sage_action(battle, unit_idx),
        BossKind::RadicalThief => radical_thief_action(battle, unit_idx),
    }
}

// ── Gatekeeper ───────────────────────────────────────────────────────────────
// Summons 门 ward tiles that block movement. Player must type "men2" to destroy.

fn gatekeeper_action(battle: &mut TacticalBattle, unit_idx: usize) -> Option<String> {
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
        if battle.arena.tile(wx, wy) != Some(BattleTile::Open) {
            continue;
        }
        if battle.unit_at(wx, wy).is_some() {
            continue;
        }
        if battle.ward_tiles.contains(&(wx, wy)) {
            continue;
        }
        battle.arena.set_tile(wx, wy, BattleTile::Obstacle);
        battle.ward_tiles.push((wx, wy));
        return Some(format!("Gatekeeper summons a 门 ward at ({},{})!", wx, wy));
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
        battle.arena.set_tile(x, y, BattleTile::Open);
        true
    } else {
        false
    }
}

// ── Scholar ──────────────────────────────────────────────────────────────────
// At 50% HP switches to Retreat AI. Sentence duel handled by game.rs.

fn scholar_action(battle: &mut TacticalBattle, unit_idx: usize) -> Option<String> {
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
                return Some("Scholar retreats, muttering ancient verses...".to_string());
            }
        }
        return Some("Scholar studies you cautiously...".to_string());
    }
    None
}

// ── Elementalist ─────────────────────────────────────────────────────────────
// Adapts resistance to last spell school. Terrain shifts every 4th turn.

fn elementalist_action(battle: &mut TacticalBattle, _unit_idx: usize) -> Option<String> {
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
            let current = battle.arena.tile(x, y).unwrap_or(BattleTile::Open);
            if current == BattleTile::Obstacle {
                continue;
            }
            let new_tile = match hash % 4 {
                0 => BattleTile::Water,
                1 => BattleTile::Ice,
                2 => BattleTile::InkPool,
                _ => BattleTile::Grass,
            };
            battle.arena.set_tile(x, y, new_tile);
            shifted += 1;
        }
        if shifted > 0 {
            return Some("Elementalist reshapes the terrain!".to_string());
        }
    }
    None
}

/// Returns damage multiplier for spells against Elementalist (0.5 if resisted).
pub fn elementalist_resistance(
    battle: &TacticalBattle,
    target_idx: usize,
    spell_school: &str,
) -> f64 {
    if let Some(BossKind::Elementalist) = battle.units[target_idx].boss_kind {
        if let Some(resisted) = battle.last_spell_school {
            if resisted == spell_school {
                return 0.5;
            }
        }
    }
    1.0
}

// ── MimicKing ────────────────────────────────────────────────────────────────
// Spawns decoy copies every 5 turns. Decoys have same hanzi but different pinyin.

fn mimic_king_action(battle: &mut TacticalBattle, unit_idx: usize) -> Option<String> {
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

        return Some("Mimic King conjures a decoy! Which is real?".to_string());
    }
    None
}

// ── InkSage ──────────────────────────────────────────────────────────────────
// Creates InkPool terrain every 2nd turn. +2 damage bonus on InkPool.
// Calligraphy trial at 50% HP handled by game.rs.

fn ink_sage_action(battle: &mut TacticalBattle, unit_idx: usize) -> Option<String> {
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
        let tile = battle.arena.tile(x, y).unwrap_or(BattleTile::Open);
        if tile == BattleTile::Open || tile == BattleTile::Grass {
            battle.arena.set_tile(x, y, BattleTile::InkPool);
            placed += 1;
        }
    }

    if placed > 0 {
        Some("Ink Sage spills ink across the battlefield!".to_string())
    } else {
        None
    }
}

/// Ink Sage damage bonus: +2 when standing on InkPool.
pub fn ink_sage_bonus(battle: &TacticalBattle, unit_idx: usize) -> i32 {
    if let Some(BossKind::InkSage) = battle.units[unit_idx].boss_kind {
        let tile = battle
            .arena
            .tile(battle.units[unit_idx].x, battle.units[unit_idx].y);
        if tile == Some(BattleTile::InkPool) {
            return 2;
        }
    }
    0
}

// ── RadicalThief ─────────────────────────────────────────────────────────────
// Steals a spell on wrong answer. Stolen spells appear as grid pickups.
// Spell theft handled in input.rs (resolve_basic_attack on wrong answer).
// This function handles the thief's own tactical behavior.

fn radical_thief_action(battle: &mut TacticalBattle, unit_idx: usize) -> Option<String> {
    if !battle.stolen_spells.is_empty() {
        let (sx, sy, _, _, _) = battle.stolen_spells[0];
        let fx = battle.units[unit_idx].x;
        let fy = battle.units[unit_idx].y;
        let dist_to_spell = manhattan(fx, fy, sx, sy);
        let dist_to_player = manhattan(fx, fy, battle.units[0].x, battle.units[0].y);

        if dist_to_spell > 1 && dist_to_player > dist_to_spell {
            if let Some((nx, ny)) = step_toward(battle, fx, fy, sx, sy) {
                move_unit(battle, unit_idx, nx, ny);
                return Some("Radical Thief guards the stolen spell!".to_string());
            }
        }
    }
    None
}

/// Steal a random spell from the player's available spells and place it on the grid.
/// Returns a log message if a spell was stolen.
pub fn steal_spell(battle: &mut TacticalBattle, unit_idx: usize) -> Option<String> {
    if battle.units[unit_idx].boss_kind != Some(BossKind::RadicalThief) {
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

    Some(format!("Radical Thief steals {}!", hanzi))
}

/// Try to pick up a stolen spell at position (x, y). Returns log message if picked up.
pub fn try_pickup_stolen_spell(battle: &mut TacticalBattle, x: i32, y: i32) -> Option<String> {
    let pos = battle
        .stolen_spells
        .iter()
        .position(|&(sx, sy, _, _, _)| sx == x && sy == y)?;
    let (_, _, hanzi, pinyin, effect) = battle.stolen_spells.remove(pos);
    battle.available_spells.push((hanzi, pinyin, effect));
    Some(format!("Recovered stolen spell {}!", hanzi))
}
