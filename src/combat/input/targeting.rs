//! Targeting system for tactical combat.

use crate::combat::action::move_unit;
use crate::combat::boss;
use crate::combat::grid::{manhattan, reachable_tiles};
use crate::combat::{BattleTile, TacticalBattle, TacticalPhase, TargetMode, TypingAction};
use crate::radical::SpellEffect;
use crate::vocab;

use super::BattleEvent;

pub(super) fn spell_range(effect: &SpellEffect) -> i32 {
    match effect {
        SpellEffect::FireAoe(_) => 4,
        SpellEffect::StrongHit(_) => 2,
        SpellEffect::Drain(_) => 1,
        SpellEffect::Stun => 3,
        SpellEffect::Pacify => 3,
        SpellEffect::Slow(_) => 3,
        SpellEffect::Teleport => 4,
        SpellEffect::Poison(_, _) => 2,
        SpellEffect::ArmorBreak => 2,
        SpellEffect::Dash(_) => 5,
        SpellEffect::Pierce(_) => 6,
        SpellEffect::PullToward => 4,
        SpellEffect::KnockBack(_) => 2,
        SpellEffect::Thorns(_) => 0,
        SpellEffect::Cone(_) => 3,
        SpellEffect::Wall(_) => 4,
        SpellEffect::OilSlick => 4,
        SpellEffect::FreezeGround(_) => 5,
        SpellEffect::Ignite => 4,
        SpellEffect::PlantGrowth => 3,
        SpellEffect::Earthquake(_) => 3,
        SpellEffect::Sanctify(_) => 3,
        SpellEffect::FloodWave(_) => 5,
        SpellEffect::SummonBoulder => 4,
        SpellEffect::Charge(_) => 5,
        SpellEffect::Blink(_) => 4,
        _ => 1,
    }
}

pub(super) fn compute_aoe_preview(
    effect: &SpellEffect,
    cx: i32,
    cy: i32,
    px: i32,
    py: i32,
) -> Vec<(i32, i32)> {
    match effect {
        SpellEffect::FireAoe(_) => {
            // Cross pattern: center + 4 cardinal neighbors
            vec![
                (cx, cy),
                (cx - 1, cy),
                (cx + 1, cy),
                (cx, cy - 1),
                (cx, cy + 1),
            ]
        }
        SpellEffect::Poison(_, _) => {
            // Small cloud: center + 2 adjacent
            vec![(cx, cy), (cx + 1, cy), (cx, cy + 1)]
        }
        SpellEffect::Dash(_) => line_between(px, py, cx, cy),
        SpellEffect::Pierce(_) => {
            let dx = (cx - px).signum();
            let dy = (cy - py).signum();
            if dx == 0 && dy == 0 {
                vec![(cx, cy)]
            } else {
                let mut tiles = Vec::new();
                let (mut x, mut y) = (px, py);
                for _ in 0..6 {
                    x += dx;
                    y += dy;
                    tiles.push((x, y));
                }
                tiles
            }
        }
        SpellEffect::Cone(_) => {
            let dx = (cx - px).signum();
            let dy = (cy - py).signum();
            let mut tiles = Vec::new();
            if dx != 0 && dy == 0 {
                // Horizontal cone
                tiles.push((px + dx, py));
                tiles.push((px + dx * 2, py));
                tiles.push((px + dx * 2, py - 1));
                tiles.push((px + dx * 2, py + 1));
                tiles.push((px + dx * 3, py));
                tiles.push((px + dx * 3, py - 1));
                tiles.push((px + dx * 3, py + 1));
            } else if dy != 0 && dx == 0 {
                // Vertical cone
                tiles.push((px, py + dy));
                tiles.push((px, py + dy * 2));
                tiles.push((px - 1, py + dy * 2));
                tiles.push((px + 1, py + dy * 2));
                tiles.push((px, py + dy * 3));
                tiles.push((px - 1, py + dy * 3));
                tiles.push((px + 1, py + dy * 3));
            } else {
                tiles.push((cx, cy));
            }
            tiles
        }
        SpellEffect::Wall(_) => {
            let dx = (cx - px).signum();
            let dy = (cy - py).signum();
            let mut tiles = Vec::new();
            if dx != 0 && dy == 0 {
                // Horizontal aim → vertical wall
                tiles.push((cx, cy - 1));
                tiles.push((cx, cy));
                tiles.push((cx, cy + 1));
            } else if dy != 0 && dx == 0 {
                // Vertical aim → horizontal wall
                tiles.push((cx - 1, cy));
                tiles.push((cx, cy));
                tiles.push((cx + 1, cy));
            } else {
                tiles.push((cx, cy));
            }
            tiles
        }
        SpellEffect::OilSlick | SpellEffect::PlantGrowth => {
            // 3×3 square
            let mut tiles = Vec::new();
            for dy in -1..=1 {
                for dx in -1..=1 {
                    tiles.push((cx + dx, cy + dy));
                }
            }
            tiles
        }
        SpellEffect::FreezeGround(_) | SpellEffect::Ignite | SpellEffect::Sanctify(_) => {
            // Cross pattern: center + 4 cardinal
            vec![
                (cx, cy),
                (cx - 1, cy),
                (cx + 1, cy),
                (cx, cy - 1),
                (cx, cy + 1),
            ]
        }
        SpellEffect::Earthquake(_) => {
            // Large cross: range 2 from center (13 tiles)
            let mut tiles = vec![(cx, cy)];
            for d in 1..=2 {
                tiles.push((cx - d, cy));
                tiles.push((cx + d, cy));
                tiles.push((cx, cy - d));
                tiles.push((cx, cy + d));
            }
            // Diagonals at distance 1
            tiles.push((cx - 1, cy - 1));
            tiles.push((cx + 1, cy - 1));
            tiles.push((cx - 1, cy + 1));
            tiles.push((cx + 1, cy + 1));
            tiles
        }
        SpellEffect::FloodWave(_) => {
            // 5×3 rectangle in direction from player
            let dx = (cx - px).signum();
            let dy = (cy - py).signum();
            let mut tiles = Vec::new();
            if dx != 0 && dy == 0 {
                // Horizontal wave
                for i in 0..5 {
                    for j in -1..=1 {
                        tiles.push((cx + dx * i, cy + j));
                    }
                }
            } else if dy != 0 && dx == 0 {
                // Vertical wave
                for i in 0..5 {
                    for j in -1..=1 {
                        tiles.push((cx + j, cy + dy * i));
                    }
                }
            } else {
                // Diagonal fallback: line
                for i in 0..5 {
                    tiles.push((cx + dx * i, cy + dy * i));
                }
            }
            tiles
        }
        SpellEffect::SummonBoulder => vec![(cx, cy)],
        SpellEffect::Charge(_) => {
            // Show the path from player to target
            line_between(px, py, cx, cy)
        }
        SpellEffect::Blink(_) => {
            // Show AoE explosion at player's current (departure) position
            vec![
                (px, py),
                (px - 1, py),
                (px + 1, py),
                (px, py - 1),
                (px, py + 1),
            ]
        }
        _ => vec![(cx, cy)],
    }
}

pub(super) fn line_between(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    let mut tiles = Vec::new();
    let dx = (x1 - x0).signum();
    let dy = (y1 - y0).signum();
    if dx == 0 && dy == 0 {
        return vec![(x0, y0)];
    }
    let (mut x, mut y) = (x0, y0);
    loop {
        x += dx;
        y += dy;
        tiles.push((x, y));
        if x == x1 && y == y1 {
            break;
        }
        if tiles.len() > 20 {
            break;
        }
    }
    tiles
}

pub(super) fn dash_target_tiles(battle: &TacticalBattle, px: i32, py: i32, range: i32) -> Vec<(i32, i32)> {
    let mut targets = Vec::new();
    let directions = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    for &(dx, dy) in &directions {
        let mut last_open = None;
        for dist in 1..=range {
            let tx = px + dx * dist;
            let ty = py + dy * dist;
            if tx < 0
                || ty < 0
                || tx >= battle.arena.width as i32
                || ty >= battle.arena.height as i32
            {
                break;
            }
            match battle.arena.tile(tx, ty) {
                Some(t) if !t.is_walkable() => break,
                None => break,
                _ => {
                    last_open = Some((tx, ty));
                }
            }
        }
        if let Some(tile) = last_open {
            if tile != (px, py) {
                targets.push(tile);
            }
        }
    }
    targets
}

/// Blink (teleport) targets: all walkable, unoccupied tiles within range and LOS.
pub(super) fn blink_target_tiles(
    battle: &TacticalBattle,
    los_tiles: &[(i32, i32)],
) -> Vec<(i32, i32)> {
    let px = battle.units[0].x;
    let py = battle.units[0].y;
    los_tiles
        .iter()
        .copied()
        .filter(|&(tx, ty)| {
            if tx == px && ty == py {
                return false;
            }
            let walkable = battle
                .arena
                .tile(tx, ty)
                .map(|t| t.is_walkable())
                .unwrap_or(false);
            walkable && battle.unit_at(tx, ty).is_none()
        })
        .collect()
}

pub(super) fn enter_move_targeting(battle: &mut TacticalBattle) {
    let player = &battle.units[0];
    let base_movement = player.effective_movement();
    let movement = (base_movement + battle.player_stance.movement_mod()).max(1);
    let valid = reachable_tiles(battle, player.x, player.y, movement);
    if valid.is_empty() {
        battle.log_message("No valid movement targets.");
        return;
    }
    // Start cursor at first valid tile.
    let (cx, cy) = valid[0];
    battle.phase = TacticalPhase::Targeting {
        mode: TargetMode::Move,
        cursor_x: cx,
        cursor_y: cy,
        valid_targets: valid,
        aoe_preview: vec![],
    };
}

pub(super) fn enter_attack_targeting(battle: &mut TacticalBattle) {
    let player = &battle.units[0];
    let adjacent = battle.adjacent_enemies(player.x, player.y);
    let px = player.x;
    let py = player.y;

    if adjacent.is_empty() {
        let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        let interactable_tiles: Vec<(i32, i32)> = deltas
            .iter()
            .filter_map(|&(dx, dy)| {
                let nx = px + dx;
                let ny = py + dy;
                if matches!(
                    battle.arena.tile(nx, ny),
                    Some(BattleTile::CargoCrate) | Some(BattleTile::FuelCanister)
                ) {
                    Some((nx, ny))
                } else {
                    None
                }
            })
            .collect();
        if interactable_tiles.is_empty() {
            battle.log_message("No adjacent enemies to attack.");
            return;
        }
        let (cx, cy) = interactable_tiles[0];
        battle.phase = TacticalPhase::Targeting {
            mode: TargetMode::Attack,
            cursor_x: cx,
            cursor_y: cy,
            valid_targets: interactable_tiles,
            aoe_preview: vec![],
        };
        return;
    }

    let valid: Vec<(i32, i32)> = adjacent
        .iter()
        .map(|&idx| (battle.units[idx].x, battle.units[idx].y))
        .collect();
    let (cx, cy) = valid[0];
    battle.phase = TacticalPhase::Targeting {
        mode: TargetMode::Attack,
        cursor_x: cx,
        cursor_y: cy,
        valid_targets: valid,
        aoe_preview: vec![],
    };
}

// ── Targeting phase ──────────────────────────────────────────────────────────

pub(super) fn handle_targeting(battle: &mut TacticalBattle, key: &str) -> BattleEvent {
    // Extract targeting state. We need to clone mode to avoid borrow issues.
    let (mode, cursor_x, cursor_y, valid_targets) = match &battle.phase {
        TacticalPhase::Targeting {
            mode,
            cursor_x,
            cursor_y,
            valid_targets,
            ..
        } => (mode.clone(), *cursor_x, *cursor_y, valid_targets.clone()),
        _ => return BattleEvent::None,
    };

    match key {
        "Escape" => {
            // Cancel targeting, go back to command.
            battle.phase = TacticalPhase::Command;
            BattleEvent::None
        }
        "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" => {
            // Cycle through valid targets in the pressed direction.
            let (dx, dy) = match key {
                "ArrowUp" => (0, -1),
                "ArrowDown" => (0, 1),
                "ArrowLeft" => (-1, 0),
                "ArrowRight" => (1, 0),
                _ => (0, 0),
            };
            let new_cursor = find_next_target(cursor_x, cursor_y, dx, dy, &valid_targets);
            if let TacticalPhase::Targeting {
                cursor_x: cx,
                cursor_y: cy,
                aoe_preview,
                ref mode,
                ..
            } = &mut battle.phase
            {
                *cx = new_cursor.0;
                *cy = new_cursor.1;
                if let TargetMode::Spell { spell_idx } = mode {
                    if *spell_idx < battle.available_spells.len() {
                        let (_, _, effect) = battle.available_spells[*spell_idx];
                        let ppx = battle.units[0].x;
                        let ppy = battle.units[0].y;
                        *aoe_preview =
                            compute_aoe_preview(&effect, new_cursor.0, new_cursor.1, ppx, ppy);
                    }
                }
            }
            BattleEvent::None
        }
        "Enter" => {
            // Confirm the target.
            confirm_target(battle, &mode, cursor_x, cursor_y)
        }
        _ => BattleEvent::None,
    }
}

/// Find the next valid target in the given direction from current cursor.
fn find_next_target(cx: i32, cy: i32, dx: i32, dy: i32, valid: &[(i32, i32)]) -> (i32, i32) {
    if valid.is_empty() {
        return (cx, cy);
    }
    // Find closest valid target in the pressed direction.
    let mut best = (cx, cy);
    let mut best_dist = i32::MAX;
    for &(vx, vy) in valid {
        // Must be in the general direction pressed.
        let along = (vx - cx) * dx + (vy - cy) * dy;
        if along <= 0 && (vx != cx || vy != cy) {
            continue; // wrong direction or same spot
        }
        let dist = manhattan(cx, cy, vx, vy);
        if dist > 0 && dist < best_dist {
            best_dist = dist;
            best = (vx, vy);
        }
    }
    // If nothing found in that direction, wrap around.
    if best == (cx, cy) {
        // Just pick the first valid target that isn't current.
        for &(vx, vy) in valid {
            if vx != cx || vy != cy {
                return (vx, vy);
            }
        }
    }
    best
}

fn confirm_target(battle: &mut TacticalBattle, mode: &TargetMode, tx: i32, ty: i32) -> BattleEvent {
    match mode {
        TargetMode::Move => {
            let idx = battle.current_unit_idx();
            move_unit(battle, idx, tx, ty);
            battle.log_message(format!("Moved to ({}, {}).", tx, ty));
            battle.player_moved = true;

            // Pick up stolen spells at destination.
            if let Some(pickup_msg) = boss::try_pickup_stolen_spell(battle, tx, ty) {
                battle.log_message(pickup_msg);
            }

            // Check for adjacent ward tiles (Gatekeeper) — auto-destroy.
            let adj = [(tx - 1, ty), (tx + 1, ty), (tx, ty - 1), (tx, ty + 1)];
            for (wx, wy) in adj {
                if boss::try_destroy_ward(battle, wx, wy) {
                    battle.log_message(format!("Ward at ({},{}) shattered!", wx, wy));
                }
            }

            if battle.player_acted {
                battle.phase = TacticalPhase::Resolve {
                    message: "Moved.".to_string(),
                    timer: 10,
                    end_turn: true,
                };
                return BattleEvent::None;
            }
            battle.phase = TacticalPhase::Command;
            BattleEvent::None
        }
        TargetMode::Attack => {
            if battle.arena.tile(tx, ty) == Some(BattleTile::CargoCrate) {
                let px = battle.units[0].x;
                let py = battle.units[0].y;
                let dx = tx - px;
                let dy = ty - py;
                let msgs = crate::combat::tick::push_boulder(battle, tx, ty, dx, dy);
                for msg in &msgs {
                    battle.log_message(msg);
                }
                battle.player_acted = true;
                if battle.player_dead() {
                    battle.phase = TacticalPhase::End {
                        victory: false,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
                if battle.all_enemies_dead() {
                    battle.phase = TacticalPhase::End {
                        victory: true,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
                battle.phase = TacticalPhase::Resolve {
                    message: "Pushed boulder!".to_string(),
                    timer: 15,
                    end_turn: true,
                };
                return BattleEvent::None;
            }

            if battle.arena.tile(tx, ty) == Some(BattleTile::FuelCanister) {
                let msgs = crate::combat::terrain::explode_barrel(battle, tx, ty);
                for msg in &msgs {
                    battle.log_message(msg);
                }
                battle.player_acted = true;
                if battle.player_dead() {
                    battle.phase = TacticalPhase::End {
                        victory: false,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
                if battle.all_enemies_dead() {
                    battle.phase = TacticalPhase::End {
                        victory: true,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
                battle.phase = TacticalPhase::Resolve {
                    message: "Barrel explodes!".to_string(),
                    timer: 15,
                    end_turn: true,
                };
                return BattleEvent::None;
            }

            if let Some(target_idx) = battle.unit_at(tx, ty) {
                // Check if enemy has shields first.
                if !battle.units[target_idx].radical_actions.is_empty()
                    && battle.units[target_idx].radical_armor > 0
                {
                    // For simplicity in MVP, just do basic attack typing.
                }
                let target_pinyin = battle.units[target_idx].pinyin;
                let syllables = vocab::pinyin_syllables(target_pinyin);
                if syllables.len() > 1 {
                    let base_damage = (battle.units[0].damage + battle.player_stance.damage_mod()).max(1);
                    let per_syl = (base_damage as f64 / syllables.len() as f64).ceil() as i32;
                    battle.typing_action = Some(TypingAction::EliteChain {
                        target_unit: target_idx,
                        syllable_progress: 0,
                        total_syllables: syllables.len(),
                        damage_per_syllable: per_syl.max(1),
                        damage_dealt: 0,
                    });
                    battle.typing_buffer.clear();
                    let hanzi = battle.units[target_idx].hanzi;
                    battle.log_message(format!(
                        "Chain attack! Type each syllable of {} ({} parts)",
                        hanzi,
                        syllables.len()
                    ));
                } else {
                    battle.typing_action = Some(TypingAction::BasicAttack {
                        target_unit: target_idx,
                    });
                    battle.typing_buffer.clear();
                    battle.log_message("Type the pinyin to attack!");
                }
                battle.phase = TacticalPhase::Command;
            }
            BattleEvent::None
        }
        TargetMode::Spell { spell_idx } => {
            if *spell_idx < battle.available_spells.len() {
                let (hanzi, _pinyin, effect) = battle.available_spells[*spell_idx];
                battle.typing_action = Some(TypingAction::SpellCast {
                    spell_idx: *spell_idx,
                    target_x: tx,
                    target_y: ty,
                    effect,
                });
                battle.typing_buffer.clear();
                battle.log_message(format!("Type pinyin for {} to cast!", hanzi));
            }
            battle.phase = TacticalPhase::Command;
            BattleEvent::None
        }
        TargetMode::ShieldBreak => {
            battle.phase = TacticalPhase::Command;
            BattleEvent::None
        }
        TargetMode::Skill => {
            if let Some(ability_idx) = battle.selected_radical_ability.take() {
                if ability_idx < battle.player_radical_abilities.len() {
                    let target = battle.unit_at(tx, ty);
                    let target_idx = target.unwrap_or(0);
                    let (radical_str, ability) = battle.player_radical_abilities[ability_idx];
                    let msg = crate::combat::radical::apply_player_radical_ability(
                        battle, 0, target_idx, ability,
                    );
                    battle.log_message(&msg);
                    battle.consumed_radicals.push(radical_str);
                    battle.player_radical_abilities.remove(ability_idx);
                    battle.player_acted = true;
                    return super::try_end_player_turn(battle);
                }
            }
            battle.phase = TacticalPhase::Command;
            BattleEvent::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::test_helpers::{make_test_battle, make_test_unit};
    use crate::combat::UnitKind;
    use crate::radical::SpellEffect;

    // ── spell_range ───────────────────────────────────────────────────────────

    #[test]
    fn spell_range_fire_aoe_returns_4() {
        assert_eq!(spell_range(&SpellEffect::FireAoe(3)), 4);
    }

    #[test]
    fn spell_range_strong_hit_returns_2() {
        assert_eq!(spell_range(&SpellEffect::StrongHit(5)), 2);
    }

    #[test]
    fn spell_range_drain_returns_1() {
        assert_eq!(spell_range(&SpellEffect::Drain(3)), 1);
    }

    #[test]
    fn spell_range_stun_returns_3() {
        assert_eq!(spell_range(&SpellEffect::Stun), 3);
    }

    #[test]
    fn spell_range_pacify_returns_3() {
        assert_eq!(spell_range(&SpellEffect::Pacify), 3);
    }

    #[test]
    fn spell_range_slow_returns_3() {
        assert_eq!(spell_range(&SpellEffect::Slow(2)), 3);
    }

    #[test]
    fn spell_range_teleport_returns_4() {
        assert_eq!(spell_range(&SpellEffect::Teleport), 4);
    }

    #[test]
    fn spell_range_poison_returns_2() {
        assert_eq!(spell_range(&SpellEffect::Poison(2, 3)), 2);
    }

    #[test]
    fn spell_range_armor_break_returns_2() {
        assert_eq!(spell_range(&SpellEffect::ArmorBreak), 2);
    }

    #[test]
    fn spell_range_dash_returns_5() {
        assert_eq!(spell_range(&SpellEffect::Dash(3)), 5);
    }

    #[test]
    fn spell_range_pierce_returns_6() {
        assert_eq!(spell_range(&SpellEffect::Pierce(3)), 6);
    }

    #[test]
    fn spell_range_pull_toward_returns_4() {
        assert_eq!(spell_range(&SpellEffect::PullToward), 4);
    }

    #[test]
    fn spell_range_knockback_returns_2() {
        assert_eq!(spell_range(&SpellEffect::KnockBack(2)), 2);
    }

    #[test]
    fn spell_range_thorns_returns_0() {
        assert_eq!(spell_range(&SpellEffect::Thorns(3)), 0);
    }

    #[test]
    fn spell_range_cone_returns_3() {
        assert_eq!(spell_range(&SpellEffect::Cone(4)), 3);
    }

    #[test]
    fn spell_range_wall_returns_4() {
        assert_eq!(spell_range(&SpellEffect::Wall(3)), 4);
    }

    #[test]
    fn spell_range_oil_slick_returns_4() {
        assert_eq!(spell_range(&SpellEffect::OilSlick), 4);
    }

    #[test]
    fn spell_range_freeze_ground_returns_5() {
        assert_eq!(spell_range(&SpellEffect::FreezeGround(2)), 5);
    }

    #[test]
    fn spell_range_ignite_returns_4() {
        assert_eq!(spell_range(&SpellEffect::Ignite), 4);
    }

    #[test]
    fn spell_range_plant_growth_returns_3() {
        assert_eq!(spell_range(&SpellEffect::PlantGrowth), 3);
    }

    #[test]
    fn spell_range_earthquake_returns_3() {
        assert_eq!(spell_range(&SpellEffect::Earthquake(3)), 3);
    }

    #[test]
    fn spell_range_sanctify_returns_3() {
        assert_eq!(spell_range(&SpellEffect::Sanctify(2)), 3);
    }

    #[test]
    fn spell_range_flood_wave_returns_5() {
        assert_eq!(spell_range(&SpellEffect::FloodWave(2)), 5);
    }

    #[test]
    fn spell_range_summon_boulder_returns_4() {
        assert_eq!(spell_range(&SpellEffect::SummonBoulder), 4);
    }

    #[test]
    fn spell_range_charge_returns_5() {
        assert_eq!(spell_range(&SpellEffect::Charge(3)), 5);
    }

    #[test]
    fn spell_range_blink_returns_4() {
        assert_eq!(spell_range(&SpellEffect::Blink(2)), 4);
    }

    #[test]
    fn spell_range_heal_falls_through_to_default_1() {
        assert_eq!(spell_range(&SpellEffect::Heal(5)), 1);
    }

    #[test]
    fn spell_range_reveal_falls_through_to_default_1() {
        assert_eq!(spell_range(&SpellEffect::Reveal), 1);
    }

    // ── compute_aoe_preview ───────────────────────────────────────────────────

    #[test]
    fn aoe_preview_fire_aoe_produces_cross_5_tiles() {
        let tiles = compute_aoe_preview(&SpellEffect::FireAoe(3), 3, 3, 0, 0);
        assert_eq!(tiles.len(), 5);
        assert!(tiles.contains(&(3, 3)));
        assert!(tiles.contains(&(2, 3)));
        assert!(tiles.contains(&(4, 3)));
        assert!(tiles.contains(&(3, 2)));
        assert!(tiles.contains(&(3, 4)));
    }

    #[test]
    fn aoe_preview_poison_produces_3_tiles() {
        let tiles = compute_aoe_preview(&SpellEffect::Poison(2, 3), 2, 2, 0, 0);
        assert_eq!(tiles.len(), 3);
        assert!(tiles.contains(&(2, 2)));
        assert!(tiles.contains(&(3, 2)));
        assert!(tiles.contains(&(2, 3)));
    }

    #[test]
    fn aoe_preview_dash_returns_line_from_player_to_cursor() {
        let tiles = compute_aoe_preview(&SpellEffect::Dash(3), 3, 0, 0, 0);
        assert_eq!(tiles, vec![(1, 0), (2, 0), (3, 0)]);
    }

    #[test]
    fn aoe_preview_pierce_horizontal_produces_6_tiles() {
        let tiles = compute_aoe_preview(&SpellEffect::Pierce(3), 3, 0, 0, 0);
        assert_eq!(tiles.len(), 6);
        assert!(tiles.contains(&(1, 0)));
        assert!(tiles.contains(&(6, 0)));
    }

    #[test]
    fn aoe_preview_pierce_same_point_returns_that_point() {
        let tiles = compute_aoe_preview(&SpellEffect::Pierce(3), 0, 0, 0, 0);
        assert_eq!(tiles, vec![(0, 0)]);
    }

    #[test]
    fn aoe_preview_cone_horizontal_right_produces_7_tiles() {
        // Player at (0,3), cursor to the right at (3,3) → dx=1, dy=0
        let tiles = compute_aoe_preview(&SpellEffect::Cone(3), 3, 3, 0, 3);
        assert_eq!(tiles.len(), 7);
        assert!(tiles.contains(&(1, 3))); // first column
        assert!(tiles.contains(&(2, 2)));
        assert!(tiles.contains(&(2, 4)));
        assert!(tiles.contains(&(3, 2)));
        assert!(tiles.contains(&(3, 3)));
        assert!(tiles.contains(&(3, 4)));
    }

    #[test]
    fn aoe_preview_cone_vertical_down_produces_7_tiles() {
        // Player at (3,0), cursor below at (3,3) → dx=0, dy=1
        let tiles = compute_aoe_preview(&SpellEffect::Cone(3), 3, 3, 3, 0);
        assert_eq!(tiles.len(), 7);
        assert!(tiles.contains(&(3, 1)));
    }

    #[test]
    fn aoe_preview_cone_diagonal_returns_single_tile() {
        // Diagonal aim: dx=1, dy=1 → fallback to single tile
        let tiles = compute_aoe_preview(&SpellEffect::Cone(3), 3, 3, 1, 1);
        assert_eq!(tiles, vec![(3, 3)]);
    }

    #[test]
    fn aoe_preview_wall_horizontal_aim_produces_vertical_wall() {
        // Player at (0,3), cursor right at (3,3) → dx=1, dy=0 → vertical wall at (3, 2..4)
        let tiles = compute_aoe_preview(&SpellEffect::Wall(3), 3, 3, 0, 3);
        assert_eq!(tiles.len(), 3);
        assert!(tiles.contains(&(3, 2)));
        assert!(tiles.contains(&(3, 3)));
        assert!(tiles.contains(&(3, 4)));
    }

    #[test]
    fn aoe_preview_wall_vertical_aim_produces_horizontal_wall() {
        // Player at (3,0), cursor down at (3,3) → dx=0, dy=1 → horizontal wall
        let tiles = compute_aoe_preview(&SpellEffect::Wall(3), 3, 3, 3, 0);
        assert_eq!(tiles.len(), 3);
        assert!(tiles.contains(&(2, 3)));
        assert!(tiles.contains(&(3, 3)));
        assert!(tiles.contains(&(4, 3)));
    }

    #[test]
    fn aoe_preview_wall_diagonal_returns_single_tile() {
        let tiles = compute_aoe_preview(&SpellEffect::Wall(3), 3, 3, 1, 1);
        assert_eq!(tiles, vec![(3, 3)]);
    }

    #[test]
    fn aoe_preview_oil_slick_produces_3x3_square_9_tiles() {
        let tiles = compute_aoe_preview(&SpellEffect::OilSlick, 3, 3, 0, 0);
        assert_eq!(tiles.len(), 9);
        assert!(tiles.contains(&(2, 2)));
        assert!(tiles.contains(&(4, 4)));
        assert!(tiles.contains(&(3, 3)));
    }

    #[test]
    fn aoe_preview_plant_growth_produces_3x3_square_9_tiles() {
        let tiles = compute_aoe_preview(&SpellEffect::PlantGrowth, 5, 5, 0, 0);
        assert_eq!(tiles.len(), 9);
        assert!(tiles.contains(&(4, 4)));
        assert!(tiles.contains(&(6, 6)));
    }

    #[test]
    fn aoe_preview_freeze_ground_produces_cross_5_tiles() {
        let tiles = compute_aoe_preview(&SpellEffect::FreezeGround(2), 3, 3, 0, 0);
        assert_eq!(tiles.len(), 5);
        assert!(tiles.contains(&(3, 3)));
        assert!(tiles.contains(&(2, 3)));
        assert!(tiles.contains(&(4, 3)));
    }

    #[test]
    fn aoe_preview_ignite_produces_cross_5_tiles() {
        let tiles = compute_aoe_preview(&SpellEffect::Ignite, 3, 3, 0, 0);
        assert_eq!(tiles.len(), 5);
    }

    #[test]
    fn aoe_preview_sanctify_produces_cross_5_tiles() {
        let tiles = compute_aoe_preview(&SpellEffect::Sanctify(2), 3, 3, 0, 0);
        assert_eq!(tiles.len(), 5);
    }

    #[test]
    fn aoe_preview_earthquake_produces_13_tiles() {
        let tiles = compute_aoe_preview(&SpellEffect::Earthquake(3), 3, 3, 0, 0);
        assert_eq!(tiles.len(), 13);
        assert!(tiles.contains(&(3, 3)));
        assert!(tiles.contains(&(1, 3)));
        assert!(tiles.contains(&(5, 3)));
        assert!(tiles.contains(&(2, 2)));
    }

    #[test]
    fn aoe_preview_flood_wave_horizontal_produces_15_tiles() {
        // Player at (0,3), cursor right at (3,3) → dx=1, dy=0 → horizontal wave
        let tiles = compute_aoe_preview(&SpellEffect::FloodWave(2), 3, 3, 0, 3);
        assert_eq!(tiles.len(), 15);
    }

    #[test]
    fn aoe_preview_flood_wave_vertical_produces_15_tiles() {
        // Player at (3,0), cursor down at (3,3) → dx=0, dy=1 → vertical wave
        let tiles = compute_aoe_preview(&SpellEffect::FloodWave(2), 3, 3, 3, 0);
        assert_eq!(tiles.len(), 15);
    }

    #[test]
    fn aoe_preview_flood_wave_diagonal_fallback_produces_5_tiles() {
        // Diagonal: dx=1, dy=1 → fallback diagonal line
        let tiles = compute_aoe_preview(&SpellEffect::FloodWave(2), 3, 3, 1, 1);
        assert_eq!(tiles.len(), 5);
    }

    #[test]
    fn aoe_preview_summon_boulder_returns_single_target_tile() {
        let tiles = compute_aoe_preview(&SpellEffect::SummonBoulder, 4, 2, 0, 0);
        assert_eq!(tiles, vec![(4, 2)]);
    }

    #[test]
    fn aoe_preview_charge_returns_line_from_player_to_cursor() {
        let tiles = compute_aoe_preview(&SpellEffect::Charge(3), 3, 0, 0, 0);
        assert_eq!(tiles, vec![(1, 0), (2, 0), (3, 0)]);
    }

    #[test]
    fn aoe_preview_blink_returns_cross_at_player_position() {
        let tiles = compute_aoe_preview(&SpellEffect::Blink(2), 5, 5, 2, 2);
        assert_eq!(tiles.len(), 5);
        assert!(tiles.contains(&(2, 2)));
        assert!(tiles.contains(&(1, 2)));
        assert!(tiles.contains(&(3, 2)));
        assert!(tiles.contains(&(2, 1)));
        assert!(tiles.contains(&(2, 3)));
    }

    #[test]
    fn aoe_preview_default_effect_returns_single_target_tile() {
        let tiles = compute_aoe_preview(&SpellEffect::Stun, 4, 3, 0, 0);
        assert_eq!(tiles, vec![(4, 3)]);
    }

    // ── line_between ──────────────────────────────────────────────────────────

    #[test]
    fn line_between_same_point_returns_that_point() {
        let tiles = line_between(3, 3, 3, 3);
        assert_eq!(tiles, vec![(3, 3)]);
    }

    #[test]
    fn line_between_horizontal_right_includes_all_tiles() {
        let tiles = line_between(0, 0, 4, 0);
        assert_eq!(tiles, vec![(1, 0), (2, 0), (3, 0), (4, 0)]);
    }

    #[test]
    fn line_between_horizontal_left_includes_all_tiles() {
        let tiles = line_between(4, 0, 0, 0);
        assert_eq!(tiles, vec![(3, 0), (2, 0), (1, 0), (0, 0)]);
    }

    #[test]
    fn line_between_vertical_down_includes_all_tiles() {
        let tiles = line_between(0, 0, 0, 3);
        assert_eq!(tiles, vec![(0, 1), (0, 2), (0, 3)]);
    }

    #[test]
    fn line_between_diagonal_includes_diagonal_tiles() {
        let tiles = line_between(0, 0, 3, 3);
        assert_eq!(tiles, vec![(1, 1), (2, 2), (3, 3)]);
    }

    #[test]
    fn line_between_caps_at_20_tiles_for_long_lines() {
        // Horizontal line of length 25, should stop at 20
        let tiles = line_between(0, 0, 25, 0);
        assert!(tiles.len() <= 21, "Too many tiles: {}", tiles.len());
    }

    // ── find_next_target ──────────────────────────────────────────────────────

    #[test]
    fn find_next_target_returns_current_when_no_valid_targets() {
        let targets: Vec<(i32, i32)> = vec![];
        let result = find_next_target(3, 3, 1, 0, &targets);
        assert_eq!(result, (3, 3));
    }

    #[test]
    fn find_next_target_finds_nearest_in_given_direction() {
        let targets = vec![(4, 3), (5, 3), (6, 3)];
        let result = find_next_target(3, 3, 1, 0, &targets); // moving right
        assert_eq!(result, (4, 3));
    }

    #[test]
    fn find_next_target_ignores_targets_in_wrong_direction() {
        let targets = vec![(1, 3), (2, 3)]; // all to the left
        // Moving right (dx=1): no targets in direction, wraps to first non-current
        let result = find_next_target(3, 3, 1, 0, &targets);
        assert_ne!(result, (3, 3)); // should wrap to some target
    }

    // ── dash_target_tiles ─────────────────────────────────────────────────────

    #[test]
    fn dash_target_tiles_returns_4_cardinal_directions_in_open_arena() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let battle = make_test_battle(vec![player]);
        let targets = dash_target_tiles(&battle, 3, 3, 3);
        assert_eq!(targets.len(), 4, "Should have 4 cardinal dash targets");
    }

    #[test]
    fn dash_target_tiles_range_1_returns_adjacent_tiles() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let battle = make_test_battle(vec![player]);
        let targets = dash_target_tiles(&battle, 3, 3, 1);
        // With range 1, should get immediate adjacent in each cardinal direction
        assert_eq!(targets.len(), 4);
        assert!(targets.contains(&(4, 3)));
        assert!(targets.contains(&(2, 3)));
        assert!(targets.contains(&(3, 4)));
        assert!(targets.contains(&(3, 2)));
    }

    #[test]
    fn dash_target_tiles_from_corner_returns_fewer_directions() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let battle = make_test_battle(vec![player]);
        let targets = dash_target_tiles(&battle, 0, 0, 3);
        // From corner (0,0), can only dash right and down
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&(3, 0)));
        assert!(targets.contains(&(0, 3)));
    }

    // ── blink_target_tiles ────────────────────────────────────────────────────

    #[test]
    fn blink_target_tiles_excludes_player_position() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let battle = make_test_battle(vec![player]);
        let los_tiles = vec![(2, 3), (3, 3), (4, 3)];
        let targets = blink_target_tiles(&battle, &los_tiles);
        assert!(!targets.contains(&(3, 3)), "Player tile must be excluded");
    }

    #[test]
    fn blink_target_tiles_excludes_occupied_tiles() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(1), 4, 3);
        enemy.alive = true;
        let battle = make_test_battle(vec![player, enemy]);
        let los_tiles = vec![(2, 3), (3, 3), (4, 3), (5, 3)];
        let targets = blink_target_tiles(&battle, &los_tiles);
        assert!(!targets.contains(&(4, 3)), "Occupied enemy tile must be excluded");
    }

    #[test]
    fn blink_target_tiles_includes_empty_walkable_tiles() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let battle = make_test_battle(vec![player]);
        let los_tiles = vec![(1, 3), (5, 3)];
        let targets = blink_target_tiles(&battle, &los_tiles);
        assert!(targets.contains(&(1, 3)));
        assert!(targets.contains(&(5, 3)));
    }
}
