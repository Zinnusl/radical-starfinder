use crate::combat::grid::{manhattan, reachable_tiles};
use crate::combat::{EnemyIntent, TacticalBattle};
use crate::enemy::{AiBehavior, RadicalAction};
use std::collections::{HashMap, VecDeque};

pub enum AiAction {
    MeleeAttack {
        target_unit: usize,
    },
    UseRadicalAction {
        action_idx: usize,
    },
    Wait,
    MoveToTile {
        path: Vec<(i32, i32)>,
    },
    MoveAndAttack {
        path: Vec<(i32, i32)>,
        target_unit: usize,
    },
    MoveAndRadical {
        path: Vec<(i32, i32)>,
        action_idx: usize,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TacticalRole {
    Offensive,
    Defensive,
    Debuff,
    Support,
}

fn get_radical_role(action: &RadicalAction) -> TacticalRole {
    match action {
        RadicalAction::SpreadingWildfire
        | RadicalAction::OverwhelmingForce
        | RadicalAction::HarvestReaping
        | RadicalAction::WaningCurse
        | RadicalAction::PotentialBurst
        | RadicalAction::CrossroadsGambit
        | RadicalAction::EchoStrike
        | RadicalAction::PreciseExecution
        | RadicalAction::CleavingCut
        | RadicalAction::PursuingSteps
        | RadicalAction::CavalryCharge
        | RadicalAction::DownpourBarrage
        | RadicalAction::ParasiticSwarm
        | RadicalAction::SavageMaul
        | RadicalAction::ArcingShot
        | RadicalAction::ConsumingBite
        | RadicalAction::BlitzAssault
        | RadicalAction::CrushingWheels
        | RadicalAction::NeedleStrike
        | RadicalAction::ArtisanTrap
        | RadicalAction::SonicBurst
        | RadicalAction::GoreCrush
        | RadicalAction::BoneShatter
        | RadicalAction::BerserkerFury
        | RadicalAction::FlockAssault
        | RadicalAction::VenomousLash => TacticalRole::Offensive,

        RadicalAction::MortalResilience
        | RadicalAction::MaternalShield
        | RadicalAction::RigidStance
        | RadicalAction::ThresholdSeal
        | RadicalAction::SoaringEscape
        | RadicalAction::ImmovablePeak
        | RadicalAction::CloakingGuise
        | RadicalAction::FlexibleCounter
        | RadicalAction::IronBodyStance
        | RadicalAction::AdaptiveShift => TacticalRole::Defensive,

        RadicalAction::ErosiveFlow
        | RadicalAction::DoubtSeed
        | RadicalAction::DevouringMaw
        | RadicalAction::WitnessMark
        | RadicalAction::RootingGrasp
        | RadicalAction::ChasingChaff
        | RadicalAction::GroundingWeight
        | RadicalAction::BindingOath
        | RadicalAction::EntanglingWeb
        | RadicalAction::PetrifyingGaze
        | RadicalAction::ImperialCommand
        | RadicalAction::ScatteringPages
        | RadicalAction::TrueVision
        | RadicalAction::QiDisruption
        | RadicalAction::SinkholeSnare
        | RadicalAction::IntoxicatingMist
        | RadicalAction::TidalSurge => TacticalRole::Debuff,

        RadicalAction::SleightReversal
        | RadicalAction::RevealingDawn
        | RadicalAction::MercenaryPact
        | RadicalAction::MagnifyingAura
        | RadicalAction::CleansingLight
        | RadicalAction::ExpandingDomain
        | RadicalAction::SproutingBarrier => TacticalRole::Support,
    }
}

fn get_radical_intent(action: &RadicalAction) -> EnemyIntent {
    let role = get_radical_role(action);
    match role {
        TacticalRole::Defensive => EnemyIntent::Buff,
        TacticalRole::Offensive => match action {
            RadicalAction::SpreadingWildfire
            | RadicalAction::ArcingShot
            | RadicalAction::CavalryCharge
            | RadicalAction::CrushingWheels
            | RadicalAction::DownpourBarrage
            | RadicalAction::ArtisanTrap
            | RadicalAction::VenomousLash
            | RadicalAction::BoneShatter
            | RadicalAction::FlockAssault => EnemyIntent::RangedAttack,
            _ => EnemyIntent::Attack,
        },
        TacticalRole::Debuff => EnemyIntent::RangedAttack,
        TacticalRole::Support => match action {
            RadicalAction::CleansingLight | RadicalAction::RevealingDawn => EnemyIntent::Heal,
            _ => EnemyIntent::Buff,
        },
    }
}

pub fn calculate_all_intents(battle: &mut TacticalBattle) {
    let player_x = battle.units[0].x;
    let player_y = battle.units[0].y;

    for i in 1..battle.units.len() {
        if !battle.units[i].alive {
            battle.units[i].intent = None;
            continue;
        }

        // Companions don't show enemy intents.
        if battle.units[i].is_companion() {
            battle.units[i].intent = None;
            continue;
        }

        if battle.units[i].stunned {
            battle.units[i].intent = Some(EnemyIntent::Idle);
            continue;
        }

        let ai = battle.units[i].ai;
        let dist = manhattan(battle.units[i].x, battle.units[i].y, player_x, player_y);
        let hp_ratio = battle.units[i].hp as f32 / battle.units[i].max_hp as f32;
        let allies_near = count_allies_near(battle, i, 3);

        let best_radical = score_and_pick_radical(battle, i, dist, hp_ratio, allies_near);

        let intent = match ai {
            AiBehavior::Chase => {
                if let Some(r) = best_radical {
                    get_radical_intent(&battle.units[i].radical_actions[r])
                } else if dist <= 1
                    || (dist <= 1 + battle.units[i].effective_movement() && best_radical.is_none())
                {
                    EnemyIntent::Attack
                } else {
                    EnemyIntent::Approach
                }
            }
            AiBehavior::Retreat => {
                if let Some(r) = best_radical {
                    get_radical_intent(&battle.units[i].radical_actions[r])
                } else if dist <= 1 {
                    EnemyIntent::Retreat
                } else if dist <= 3 {
                    EnemyIntent::Retreat
                } else {
                    EnemyIntent::Idle
                }
            }
            AiBehavior::Ambush => {
                if dist <= 3 {
                    if let Some(r) = best_radical {
                        get_radical_intent(&battle.units[i].radical_actions[r])
                    } else {
                        EnemyIntent::Attack
                    }
                } else {
                    EnemyIntent::Idle
                }
            }
            AiBehavior::Sentinel => {
                if let Some(r) = best_radical {
                    get_radical_intent(&battle.units[i].radical_actions[r])
                } else if dist <= 1 {
                    EnemyIntent::Attack
                } else {
                    EnemyIntent::Idle
                }
            }
            AiBehavior::Kiter => {
                if let Some(r) = best_radical {
                    get_radical_intent(&battle.units[i].radical_actions[r])
                } else if dist <= 2 {
                    EnemyIntent::Retreat
                } else if dist >= 5 {
                    EnemyIntent::Approach
                } else {
                    EnemyIntent::Idle
                }
            }
            AiBehavior::Pack => {
                if let Some(r) = best_radical {
                    get_radical_intent(&battle.units[i].radical_actions[r])
                } else if allies_near >= 2 {
                    EnemyIntent::Attack
                } else if dist <= 1 {
                    EnemyIntent::Attack
                } else {
                    EnemyIntent::Surround
                }
            }
        };

        battle.units[i].intent = Some(intent);
    }
    battle.intents_calculated = true;
}

fn count_allies_near(battle: &TacticalBattle, unit_idx: usize, radius: i32) -> usize {
    let unit = &battle.units[unit_idx];
    let is_enemy_unit = unit.is_enemy();
    let mut count = 0;
    for (i, other) in battle.units.iter().enumerate() {
        if i != unit_idx && other.alive {
            // Only count same-faction units as allies.
            let same_faction = if is_enemy_unit {
                other.is_enemy()
            } else {
                !other.is_enemy()
            };
            if same_faction && manhattan(unit.x, unit.y, other.x, other.y) <= radius {
                count += 1;
            }
        }
    }
    count
}

fn score_and_pick_radical(
    battle: &TacticalBattle,
    unit_idx: usize,
    dist: i32,
    hp_ratio: f32,
    allies_near: usize,
) -> Option<usize> {
    let unit = &battle.units[unit_idx];
    if unit.radical_actions.is_empty() {
        return None;
    }

    let seed = (battle.turn_number as u64)
        .wrapping_mul(31)
        .wrapping_add(unit_idx as u64)
        .wrapping_mul(17);

    let mut best_score = 0;
    let mut best_idx = None;

    for (i, action) in unit.radical_actions.iter().enumerate() {
        let role = get_radical_role(action);
        let mut score = 0;

        match unit.ai {
            AiBehavior::Chase => {
                if role == TacticalRole::Offensive {
                    score += 50;
                }
                if hp_ratio < 0.3 && role == TacticalRole::Defensive {
                    score += 100;
                }
            }
            AiBehavior::Retreat => {
                if role == TacticalRole::Defensive {
                    score += 80;
                }
                if hp_ratio < 0.5 && role == TacticalRole::Defensive {
                    score += 40;
                }
            }
            AiBehavior::Ambush => {
                if dist <= 3 {
                    if role == TacticalRole::Offensive {
                        score += 100;
                    }
                }
            }
            AiBehavior::Sentinel => {
                if role == TacticalRole::Defensive {
                    score += 100;
                }
            }
            AiBehavior::Kiter => {
                if role == TacticalRole::Debuff || role == TacticalRole::Offensive {
                    score += 60;
                }
            }
            AiBehavior::Pack => {
                if role == TacticalRole::Support {
                    if allies_near == 0 {
                        score += 100;
                    }
                } else if allies_near >= 2 && role == TacticalRole::Offensive {
                    score += 50;
                }
            }
        }

        match role {
            TacticalRole::Defensive => {
                if hp_ratio < 0.5 {
                    score += 30;
                }
                if unit.radical_dodge
                    && matches!(
                        action,
                        RadicalAction::SoaringEscape | RadicalAction::CloakingGuise
                    )
                {
                    score -= 40;
                }
                if unit.radical_armor > 0
                    && matches!(
                        action,
                        RadicalAction::RigidStance
                            | RadicalAction::ThresholdSeal
                            | RadicalAction::ImmovablePeak
                    )
                {
                    score -= 40;
                }
            }
            TacticalRole::Offensive => {
                if dist > 3 && matches!(action, RadicalAction::ArcingShot) {
                    score += 50;
                }
                if dist > 3
                    && matches!(
                        action,
                        RadicalAction::SpreadingWildfire | RadicalAction::CavalryCharge
                    )
                {
                    score += 30;
                }
                let player_hp_ratio = battle.units[0].hp as f32 / battle.units[0].max_hp as f32;
                if player_hp_ratio < 0.25
                    && matches!(
                        action,
                        RadicalAction::PreciseExecution | RadicalAction::HarvestReaping
                    )
                {
                    score += 50;
                }
                if player_hp_ratio < 0.40 && matches!(action, RadicalAction::HarvestReaping) {
                    score += 40;
                }
                if matches!(action, RadicalAction::EchoStrike) && unit.damage <= 1 {
                    score -= 30;
                }
                if matches!(action, RadicalAction::CrossroadsGambit) {
                    score -= 10;
                }
                if matches!(action, RadicalAction::SavageMaul) && unit.hp <= 1 {
                    score -= 60;
                }
            }
            TacticalRole::Debuff => {
                if matches!(
                    action,
                    RadicalAction::DoubtSeed
                        | RadicalAction::ChasingChaff
                        | RadicalAction::BindingOath
                ) && battle.units[0]
                    .statuses
                    .iter()
                    .any(|s| matches!(s.kind, crate::status::StatusKind::Confused))
                {
                    score -= 60;
                }
                if matches!(
                    action,
                    RadicalAction::ErosiveFlow
                        | RadicalAction::GroundingWeight
                        | RadicalAction::RootingGrasp
                        | RadicalAction::EntanglingWeb
                        | RadicalAction::PetrifyingGaze
                ) && battle.units[0]
                    .statuses
                    .iter()
                    .any(|s| matches!(s.kind, crate::status::StatusKind::Slow))
                {
                    score -= 40;
                }
                if matches!(action, RadicalAction::WitnessMark)
                    && battle.units[0].marked_extra_damage > 0
                {
                    score -= 50;
                }
                if matches!(action, RadicalAction::DevouringMaw)
                    && (!battle.units[0].radical_dodge
                        && battle.units[0].radical_armor == 0
                        && !battle.units[0].defending
                        && !battle.units[0].radical_counter)
                {
                    score -= 40;
                }
            }
            TacticalRole::Support => {
                if matches!(
                    action,
                    RadicalAction::MercenaryPact
                        | RadicalAction::MagnifyingAura
                        | RadicalAction::RevealingDawn
                ) {
                    let mut low_hp_allies = 0;
                    for i in 1..battle.units.len() {
                        if battle.units[i].alive
                            && (battle.units[i].hp as f32 / battle.units[i].max_hp as f32) < 0.5
                        {
                            low_hp_allies += 1;
                        }
                    }
                    if low_hp_allies >= 2 {
                        score += 80;
                    } else if low_hp_allies == 0 {
                        score -= 50;
                    }
                }
                if matches!(action, RadicalAction::CleansingLight)
                    && unit.statuses.is_empty()
                    && hp_ratio == 1.0
                {
                    score -= 100;
                }
            }
        }

        let action_seed = seed.wrapping_add(i as u64);
        let jitter = (action_seed % 10) as i32;
        score += jitter;

        if score > 50 && score > best_score {
            best_score = score;
            best_idx = Some(i);
        }
    }

    best_idx
}

pub fn choose_action(battle: &TacticalBattle, unit_idx: usize) -> AiAction {
    let unit = &battle.units[unit_idx];
    if unit.stunned {
        return AiAction::Wait;
    }

    let player = &battle.units[0];

    if unit
        .statuses
        .iter()
        .any(|s| matches!(s.kind, crate::status::StatusKind::Fear))
    {
        if let Some(path) = path_away(battle, unit_idx, player.x, player.y) {
            return AiAction::MoveToTile { path };
        }
        return AiAction::Wait;
    }

    let dist = manhattan(unit.x, unit.y, player.x, player.y);
    let hp_ratio = unit.hp as f32 / unit.max_hp as f32;
    let allies_near = count_allies_near(battle, unit_idx, 3);

    let best_radical = score_and_pick_radical(battle, unit_idx, dist, hp_ratio, allies_near);

    match unit.ai {
        AiBehavior::Chase => {
            if let Some(r) = best_radical {
                if dist > 1 {
                    if let Some(path) = path_toward(battle, unit_idx, player.x, player.y, true) {
                        return AiAction::MoveAndRadical {
                            path,
                            action_idx: r,
                        };
                    }
                }
                return AiAction::UseRadicalAction { action_idx: r };
            }

            if dist <= 1 {
                AiAction::MeleeAttack { target_unit: 0 }
            } else if let Some(path) = path_toward(battle, unit_idx, player.x, player.y, true) {
                let last = path.last().copied().unwrap_or((unit.x, unit.y));
                if manhattan(last.0, last.1, player.x, player.y) <= 1 {
                    AiAction::MoveAndAttack {
                        path,
                        target_unit: 0,
                    }
                } else {
                    AiAction::MoveToTile { path }
                }
            } else {
                AiAction::Wait
            }
        }
        AiBehavior::Retreat => {
            if let Some(r) = best_radical {
                return AiAction::UseRadicalAction { action_idx: r };
            }

            if dist <= 2 {
                if let Some(path) = path_away(battle, unit_idx, player.x, player.y) {
                    AiAction::MoveToTile { path }
                } else if dist <= 1 {
                    AiAction::MeleeAttack { target_unit: 0 }
                } else {
                    AiAction::Wait
                }
            } else {
                AiAction::Wait
            }
        }
        AiBehavior::Ambush => {
            if dist <= 3 {
                if let Some(r) = best_radical {
                    return AiAction::UseRadicalAction { action_idx: r };
                }

                if dist <= 1 {
                    AiAction::MeleeAttack { target_unit: 0 }
                } else if let Some(path) = path_toward(battle, unit_idx, player.x, player.y, true) {
                    let last = path.last().copied().unwrap_or((unit.x, unit.y));
                    if manhattan(last.0, last.1, player.x, player.y) <= 1 {
                        AiAction::MoveAndAttack {
                            path,
                            target_unit: 0,
                        }
                    } else {
                        AiAction::MoveToTile { path }
                    }
                } else {
                    AiAction::Wait
                }
            } else {
                AiAction::Wait
            }
        }
        AiBehavior::Sentinel => {
            if let Some(r) = best_radical {
                return AiAction::UseRadicalAction { action_idx: r };
            }

            if dist <= 1 {
                AiAction::MeleeAttack { target_unit: 0 }
            } else {
                AiAction::Wait
            }
        }
        AiBehavior::Kiter => {
            if let Some(r) = best_radical {
                return AiAction::UseRadicalAction { action_idx: r };
            }

            if dist <= 2 {
                if let Some(path) = path_away(battle, unit_idx, player.x, player.y) {
                    AiAction::MoveToTile { path }
                } else if dist <= 1 {
                    AiAction::MeleeAttack { target_unit: 0 }
                } else {
                    AiAction::Wait
                }
            } else if dist >= 5 {
                if let Some(path) = path_toward(battle, unit_idx, player.x, player.y, false) {
                    AiAction::MoveToTile { path }
                } else {
                    AiAction::Wait
                }
            } else {
                AiAction::Wait
            }
        }
        AiBehavior::Pack => {
            if let Some(r) = best_radical {
                return AiAction::UseRadicalAction { action_idx: r };
            }

            if allies_near >= 2 {
                if dist <= 1 {
                    AiAction::MeleeAttack { target_unit: 0 }
                } else if let Some(path) = path_toward(battle, unit_idx, player.x, player.y, true) {
                    let last = path.last().copied().unwrap_or((unit.x, unit.y));
                    if manhattan(last.0, last.1, player.x, player.y) <= 1 {
                        AiAction::MoveAndAttack {
                            path,
                            target_unit: 0,
                        }
                    } else {
                        AiAction::MoveToTile { path }
                    }
                } else {
                    AiAction::Wait
                }
            } else {
                if dist <= 1 {
                    AiAction::MeleeAttack { target_unit: 0 }
                } else if let Some(path) = path_toward_ally(battle, unit_idx) {
                    AiAction::MoveToTile { path }
                } else if let Some(path) = path_toward(battle, unit_idx, player.x, player.y, true)
                {
                    AiAction::MoveToTile { path }
                } else {
                    AiAction::Wait
                }
            }
        }
    }
}

pub fn path_toward(
    battle: &TacticalBattle,
    unit_idx: usize,
    target_x: i32,
    target_y: i32,
    close_in: bool,
) -> Option<Vec<(i32, i32)>> {
    let unit = &battle.units[unit_idx];
    let movement = unit.effective_movement();
    let reachable = reachable_tiles(battle, unit.x, unit.y, movement);

    if reachable.is_empty() {
        return None;
    }

    let mut best_tile = (unit.x, unit.y);
    let mut best_score = i32::MIN;

    for &(rx, ry) in &reachable {
        let d = manhattan(rx, ry, target_x, target_y);
        let mut score = -d * 10;

        if let Some(tile) = battle.arena.tile(rx, ry) {
            if matches!(
                tile,
                crate::combat::BattleTile::ConveyorN
                    | crate::combat::BattleTile::ConveyorS
                    | crate::combat::BattleTile::ConveyorE
                    | crate::combat::BattleTile::ConveyorW
            ) {
                score -= 30;
            }
            if tile == crate::combat::BattleTile::MineTileRevealed {
                score -= 40;
            }
        }
        // Avoid tiles adjacent to explosive barrels
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = rx + dx;
            let ny = ry + dy;
            if let Some(adj) = battle.arena.tile(nx, ny) {
                if adj == crate::combat::BattleTile::FuelCanister {
                    score -= 50;
                }
            }
        }

        if !close_in {
            let mut dist_to_allies = 0;
            for (i, other) in battle.units.iter().enumerate() {
                if other.alive && other.is_enemy() && i != unit_idx {
                    dist_to_allies += manhattan(rx, ry, other.x, other.y);
                }
            }
            score += dist_to_allies;
        }

        if score > best_score || (score == best_score && (rx, ry) < best_tile) {
            best_score = score;
            best_tile = (rx, ry);
        }
    }

    if best_tile == (unit.x, unit.y) {
        return None;
    }

    build_path(battle, unit.x, unit.y, best_tile.0, best_tile.1, movement)
}

pub fn path_away(
    battle: &TacticalBattle,
    unit_idx: usize,
    target_x: i32,
    target_y: i32,
) -> Option<Vec<(i32, i32)>> {
    let unit = &battle.units[unit_idx];
    let movement = unit.effective_movement();
    let reachable = reachable_tiles(battle, unit.x, unit.y, movement);

    if reachable.is_empty() {
        return None;
    }

    let mut best_tile = (unit.x, unit.y);
    let mut best_dist = manhattan(unit.x, unit.y, target_x, target_y);

    for &(rx, ry) in &reachable {
        let d = manhattan(rx, ry, target_x, target_y);
        let mut adj_d = d;
        if let Some(tile) = battle.arena.tile(rx, ry) {
            if matches!(
                tile,
                crate::combat::BattleTile::ConveyorN
                    | crate::combat::BattleTile::ConveyorS
                    | crate::combat::BattleTile::ConveyorE
                    | crate::combat::BattleTile::ConveyorW
            ) {
                adj_d -= 3;
            }
            if tile == crate::combat::BattleTile::MineTileRevealed {
                adj_d -= 4;
            }
        }
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = rx + dx;
            let ny = ry + dy;
            if let Some(adj) = battle.arena.tile(nx, ny) {
                if adj == crate::combat::BattleTile::FuelCanister {
                    adj_d -= 5;
                }
            }
        }
        if adj_d > best_dist || (adj_d == best_dist && (rx, ry) < best_tile) {
            best_dist = adj_d;
            best_tile = (rx, ry);
        }
    }

    if best_tile == (unit.x, unit.y) {
        return None;
    }

    build_path(battle, unit.x, unit.y, best_tile.0, best_tile.1, movement)
}

/// Pack AI: move toward the nearest ally to group up before attacking.
fn path_toward_ally(battle: &TacticalBattle, unit_idx: usize) -> Option<Vec<(i32, i32)>> {
    let unit = &battle.units[unit_idx];

    // Find nearest alive enemy ally.
    let mut nearest_ally: Option<(i32, i32)> = None;
    let mut best_dist = i32::MAX;
    for (i, other) in battle.units.iter().enumerate() {
        if i != unit_idx && other.alive && other.is_enemy() {
            let d = manhattan(unit.x, unit.y, other.x, other.y);
            if d < best_dist {
                best_dist = d;
                nearest_ally = Some((other.x, other.y));
            }
        }
    }

    let (ax, ay) = nearest_ally?;
    if best_dist <= 1 {
        return None; // Already adjacent.
    }

    path_toward(battle, unit_idx, ax, ay, true)
}

fn build_path(
    battle: &TacticalBattle,
    start_x: i32,
    start_y: i32,
    target_x: i32,
    target_y: i32,
    max_movement: i32,
) -> Option<Vec<(i32, i32)>> {
    let arena = &battle.arena;
    let w = arena.width as i32;
    let h = arena.height as i32;

    let mut queue = VecDeque::new();
    queue.push_back((start_x, start_y, 0i32));

    let mut parent = HashMap::new();
    let mut cost_map = HashMap::new();
    cost_map.insert((start_x, start_y), 0);

    let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];

    while let Some((cx, cy, cost)) = queue.pop_front() {
        if cx == target_x && cy == target_y {
            break;
        }

        for (dx, dy) in &deltas {
            let nx = cx + dx;
            let ny = cy + dy;

            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            if let Some(tile) = arena.tile(nx, ny) {
                if !tile.is_walkable() {
                    continue;
                }
            }

            if battle.unit_at(nx, ny).is_some() && (nx != target_x || ny != target_y) {
                continue;
            }

            let mut step_cost = 1;
            if let Some(tile) = arena.tile(nx, ny) {
                step_cost += tile.extra_move_cost();
            }
            if battle.weather == crate::combat::Weather::DebrisStorm {
                step_cost += 1;
            }

            let new_cost = cost + step_cost;
            if new_cost > max_movement {
                continue;
            }

            let current_known_cost = cost_map.get(&(nx, ny)).copied().unwrap_or(i32::MAX);
            if new_cost < current_known_cost {
                cost_map.insert((nx, ny), new_cost);
                parent.insert((nx, ny), (cx, cy));
                queue.push_back((nx, ny, new_cost));
            }
        }
    }

    if !parent.contains_key(&(target_x, target_y)) {
        return None;
    }

    let mut path = Vec::new();
    let mut curr = (target_x, target_y);
    while curr != (start_x, start_y) {
        path.push(curr);
        curr = parent[&curr];
    }
    path.reverse();
    Some(path)
}

pub fn step_toward(
    battle: &TacticalBattle,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
) -> Option<(i32, i32)> {
    let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    let mut best: Option<(i32, i32)> = None;
    let mut best_dist = i32::MAX;

    for (dx, dy) in &deltas {
        let nx = from_x + dx;
        let ny = from_y + dy;
        if !battle.arena.in_bounds(nx, ny) {
            continue;
        }
        if let Some(tile) = battle.arena.tile(nx, ny) {
            if !tile.is_walkable() {
                continue;
            }
        }
        if battle.unit_at(nx, ny).is_some() {
            continue;
        }
        let d = manhattan(nx, ny, to_x, to_y);
        if d < best_dist {
            best_dist = d;
            best = Some((nx, ny));
        }
    }
    best
}

pub fn step_away(
    battle: &TacticalBattle,
    from_x: i32,
    from_y: i32,
    away_x: i32,
    away_y: i32,
) -> Option<(i32, i32)> {
    let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    let mut best: Option<(i32, i32)> = None;
    let mut best_dist = i32::MIN;

    for (dx, dy) in &deltas {
        let nx = from_x + dx;
        let ny = from_y + dy;
        if !battle.arena.in_bounds(nx, ny) {
            continue;
        }
        if let Some(tile) = battle.arena.tile(nx, ny) {
            if !tile.is_walkable() {
                continue;
            }
        }
        if battle.unit_at(nx, ny).is_some() {
            continue;
        }
        let d = manhattan(nx, ny, away_x, away_y);
        if d > best_dist {
            best_dist = d;
            best = Some((nx, ny));
        }
    }
    best
}

/// Choose an action for a companion unit — targets the nearest alive enemy.
pub fn choose_companion_action(battle: &TacticalBattle, unit_idx: usize) -> AiAction {
    let unit = &battle.units[unit_idx];
    if unit.stunned {
        return AiAction::Wait;
    }

    // Find nearest alive enemy.
    let mut best_target: Option<usize> = None;
    let mut best_dist = i32::MAX;
    for (i, other) in battle.units.iter().enumerate() {
        if other.alive && other.is_enemy() {
            let d = manhattan(unit.x, unit.y, other.x, other.y);
            if d < best_dist {
                best_dist = d;
                best_target = Some(i);
            }
        }
    }

    let target_idx = match best_target {
        Some(t) => t,
        None => return AiAction::Wait,
    };
    let target = &battle.units[target_idx];

    if best_dist <= 1 {
        AiAction::MeleeAttack {
            target_unit: target_idx,
        }
    } else if let Some(path) = path_toward(battle, unit_idx, target.x, target.y, true) {
        let last = path.last().copied().unwrap_or((unit.x, unit.y));
        if manhattan(last.0, last.1, target.x, target.y) <= 1 {
            AiAction::MoveAndAttack {
                path,
                target_unit: target_idx,
            }
        } else {
            AiAction::MoveToTile { path }
        }
    } else {
        AiAction::Wait
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::test_helpers::{make_test_battle, make_test_unit};
    use crate::combat::{BattleTile, UnitKind};

    #[test]
    fn step_toward_moves_closer_to_target() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 5);
        let battle = make_test_battle(vec![player, enemy]);

        let result = step_toward(&battle, 3, 3, 6, 6);
        assert!(result.is_some());
        let (nx, ny) = result.unwrap();
        // Should move closer to (6,6)
        let old_dist = manhattan(3, 3, 6, 6);
        let new_dist = manhattan(nx, ny, 6, 6);
        assert!(new_dist < old_dist);
    }

    #[test]
    fn step_toward_avoids_occupied_tiles() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        // Place a blocking unit to the east
        let blocker = make_test_unit(UnitKind::Enemy(0), 4, 3);
        let battle = make_test_battle(vec![player, blocker]);

        let result = step_toward(&battle, 3, 3, 6, 3);
        // Can't step east (occupied), should pick another direction
        match result {
            Some((x, y)) => assert!(!(x == 4 && y == 3)), // not into blocker
            None => {} // acceptable if all adjacent are occupied/blocked
        }
    }

    #[test]
    fn step_toward_avoids_unwalkable_tiles() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);
        // Block east with a barrier
        battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);

        let result = step_toward(&battle, 3, 3, 6, 3);
        match result {
            Some((x, y)) => assert!(!(x == 4 && y == 3)),
            None => {}
        }
    }

    #[test]
    fn step_away_moves_farther_from_threat() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let battle = make_test_battle(vec![player]);

        let result = step_away(&battle, 3, 3, 2, 3);
        assert!(result.is_some());
        let (nx, ny) = result.unwrap();
        let old_dist = manhattan(3, 3, 2, 3);
        let new_dist = manhattan(nx, ny, 2, 3);
        assert!(new_dist > old_dist);
    }

    #[test]
    fn step_away_respects_bounds() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let battle = make_test_battle(vec![player]);

        // Try to step away from center — should stay in bounds
        let result = step_away(&battle, 0, 0, 3, 3);
        if let Some((x, y)) = result {
            assert!(battle.arena.in_bounds(x, y));
        }
    }

    #[test]
    fn count_allies_near_counts_same_faction() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let enemy2 = make_test_unit(UnitKind::Enemy(1), 4, 3); // within 1 of enemy1
        let enemy3 = make_test_unit(UnitKind::Enemy(2), 6, 6); // far from enemy1

        let battle = make_test_battle(vec![player, enemy1, enemy2, enemy3]);
        let count = count_allies_near(&battle, 1, 2); // radius 2
        assert_eq!(count, 1); // only enemy2 is within range
    }

    #[test]
    fn get_radical_role_classifies_correctly() {
        assert!(matches!(get_radical_role(&RadicalAction::SpreadingWildfire), TacticalRole::Offensive));
        assert!(matches!(get_radical_role(&RadicalAction::MortalResilience), TacticalRole::Defensive));
        assert!(matches!(get_radical_role(&RadicalAction::ErosiveFlow), TacticalRole::Debuff));
        assert!(matches!(get_radical_role(&RadicalAction::RevealingDawn), TacticalRole::Support));
    }
}
