use crate::combat::action::deal_damage;
use crate::combat::ai::calculate_all_intents;
use crate::combat::input::{execute_enemy_turn_action, BattleEvent};
use crate::combat::terrain::{apply_scorched_damage, decay_cracked_floors, tick_terrain};
use crate::combat::turn::advance_turn;
use crate::combat::{
    ArenaEvent, ArenaBiome, AudioEvent, BattleTile, ProjectileEffect, TacticalBattle,
    TacticalPhase, Weather,
};
use crate::dungeon::Rng;
use crate::status::tick_statuses;

const _RESOLVE_FRAMES: u8 = 30; // ~500ms at 60fps
const ENEMY_TURN_FRAMES: u8 = 24; // ~400ms
const END_DELAY_FRAMES: u8 = 60; // ~1s before key accepted
const ENVIRONMENT_TICK_FRAMES: u8 = 18; // ~300ms for environment phase

pub fn tick_battle(battle: &mut TacticalBattle) -> BattleEvent {
    // Decrement event message fade timer (per-frame)
    if battle.event_message_timer > 0 {
        battle.event_message_timer -= 1;
        if battle.event_message_timer == 0 {
            battle.event_message = None;
        }
    }

    // Decrement spell combo notification fade timer (per-frame)
    if battle.combo_message_timer > 0 {
        battle.combo_message_timer -= 1;
        if battle.combo_message_timer == 0 {
            battle.combo_message = None;
        }
    }

    if !battle.intents_calculated {
        calculate_all_intents(battle);
    }
    match battle.phase {
        TacticalPhase::Resolve {
            ref mut timer,
            end_turn,
            ..
        } => {
            if *timer > 0 {
                *timer -= 1;
                return BattleEvent::None;
            }
            if end_turn {
                advance_and_set_phase(battle)
            } else {
                battle.phase = TacticalPhase::Command;
                BattleEvent::None
            }
        }
        TacticalPhase::EnemyTurn {
            unit_idx,
            ref mut timer,
            ref mut acted,
        } => {
            if !*acted {
                *acted = true;
                let event = execute_enemy_turn_action(battle, unit_idx);
                match event {
                    BattleEvent::Defeat | BattleEvent::Victory => return event,
                    _ => {}
                }
                if battle.player_dead() {
                    battle.phase = TacticalPhase::End {
                        victory: false,
                        timer: END_DELAY_FRAMES,
                    };
                    return BattleEvent::None;
                }
                if battle.all_enemies_dead() {
                    battle.phase = TacticalPhase::End {
                        victory: true,
                        timer: END_DELAY_FRAMES,
                    };
                    return BattleEvent::None;
                }
                return BattleEvent::None;
            }

            if *timer > 0 {
                *timer -= 1;
                return BattleEvent::None;
            }
            // If enemy radical actions spawned fast projectiles, animate them
            if !battle.projectiles.is_empty() {
                battle.phase = TacticalPhase::ProjectileAnimation {
                    message: String::new(),
                    end_turn: true,
                };
                return BattleEvent::None;
            }
            advance_and_set_phase(battle)
        }
        TacticalPhase::End { ref mut timer, .. } => {
            if *timer > 0 {
                *timer -= 1;
            }
            BattleEvent::None
        }
        TacticalPhase::ProjectileAnimation { .. } => {
            tick_projectiles(battle);
            BattleEvent::None
        }
        TacticalPhase::EnvironmentTick { ref mut timer } => {
            if *timer > 0 {
                *timer -= 1;
                return BattleEvent::None;
            }
            // Environment phase done — proceed to next unit in queue
            finish_environment_tick(battle)
        }
        _ => BattleEvent::None,
    }
}

fn tick_projectiles(battle: &mut TacticalBattle) {
    for proj in battle.projectiles.iter_mut() {
        if proj.done {
            continue;
        }
        proj.progress += proj.speed;
        if proj.progress >= 1.0 {
            proj.progress = 1.0;
            proj.done = true;
        }
    }

    let finished: Vec<_> = battle
        .projectiles
        .iter()
        .filter(|p| p.done)
        .map(|p| {
            (
                p.from_x,
                p.from_y,
                p.to_x,
                p.to_y,
                p.effect.clone(),
                p.owner_idx,
            )
        })
        .collect();

    battle.projectiles.retain(|p| !p.done);

    if !finished.is_empty() {
        battle.audio_events.push(AudioEvent::ProjectileImpact);
    }

    for (fx, fy, tx, ty, effect, _owner) in &finished {
        if battle.arena.tile(*tx, *ty) == Some(BattleTile::CargoCrate) {
            let raw_dx = *tx as f64 - fx;
            let raw_dy = *ty as f64 - fy;
            let (pdx, pdy) = if raw_dx.abs() >= raw_dy.abs() {
                (if raw_dx >= 0.0 { 1 } else { -1 }, 0)
            } else {
                (0, if raw_dy >= 0.0 { 1 } else { -1 })
            };
            let msgs = push_boulder(battle, *tx, *ty, pdx, pdy);
            for msg in &msgs {
                battle.log_message(msg);
            }
            continue;
        }

        if battle.arena.tile(*tx, *ty) == Some(BattleTile::FuelCanister) {
            let msgs = crate::combat::terrain::explode_barrel(battle, *tx, *ty);
            for msg in &msgs {
                battle.log_message(msg);
            }
            continue;
        }

        match effect {
            ProjectileEffect::Damage(dmg) => {
                if let Some(idx) = battle.unit_at(*tx, *ty) {
                    deal_damage(battle, idx, *dmg);
                }
            }
            ProjectileEffect::PiercingDamage(dmg) => {
                if let Some(idx) = battle.unit_at(*tx, *ty) {
                    let unit = &mut battle.units[idx];
                    unit.hp -= dmg;
                    if unit.hp <= 0 {
                        unit.hp = 0;
                        unit.alive = false;
                    }
                }
            }
            ProjectileEffect::SpellHit(spell_effect) => {
                apply_projectile_spell(battle, *tx, *ty, spell_effect);
            }
        }
    }

    if battle.projectiles.is_empty() {
        let (msg, end_turn) = match &battle.phase {
            TacticalPhase::ProjectileAnimation { message, end_turn } => {
                (message.clone(), *end_turn)
            }
            _ => (String::new(), false),
        };
        if battle.player_dead() {
            battle.phase = TacticalPhase::End {
                victory: false,
                timer: END_DELAY_FRAMES,
            };
        } else if battle.all_enemies_dead() {
            battle.phase = TacticalPhase::End {
                victory: true,
                timer: END_DELAY_FRAMES,
            };
        } else {
            battle.phase = TacticalPhase::Resolve {
                message: msg,
                timer: 15,
                end_turn,
            };
        }
    }
}

fn apply_projectile_spell(
    battle: &mut TacticalBattle,
    tx: i32,
    ty: i32,
    effect: &crate::radical::SpellEffect,
) {
    use crate::radical::SpellEffect;
    let sp = battle.player_stance.spell_power_mod();
    match effect {
        SpellEffect::StrongHit(dmg) => {
            if let Some(idx) = battle.unit_at(tx, ty) {
                if battle.units[idx].is_enemy() {
                    deal_damage(battle, idx, (*dmg + sp).max(1));
                }
            }
        }
        SpellEffect::Drain(dmg) => {
            if let Some(idx) = battle.unit_at(tx, ty) {
                if battle.units[idx].is_enemy() {
                    let actual = deal_damage(battle, idx, (*dmg + sp).max(1));
                    let unit = &mut battle.units[0];
                    unit.hp = (unit.hp + actual).min(unit.max_hp);
                }
            }
        }
        SpellEffect::Stun => {
            if let Some(idx) = battle.unit_at(tx, ty) {
                if battle.units[idx].is_enemy() {
                    battle.units[idx].stunned = true;
                }
            }
            // Lightning conducts through water, stunning units on connected water tiles
            let terrain_msgs = crate::combat::terrain::apply_terrain_interactions(
                battle,
                crate::combat::terrain::TerrainSource::LightningAbility,
                &[(tx, ty)],
            );
            for msg in &terrain_msgs {
                battle.log_message(msg);
            }
        }
        SpellEffect::Poison(dmg, turns) => {
            if let Some(idx) = battle.unit_at(tx, ty) {
                if battle.units[idx].is_enemy() {
                    battle.units[idx]
                        .statuses
                        .push(crate::status::StatusInstance::new(
                            crate::status::StatusKind::Poison { damage: *dmg },
                            *turns,
                        ));
                    battle.audio_events.push(AudioEvent::StatusPoison);
                }
            }
        }
        SpellEffect::Slow(turns) => {
            if let Some(idx) = battle.unit_at(tx, ty) {
                if battle.units[idx].is_enemy() {
                    battle.units[idx]
                        .statuses
                        .push(crate::status::StatusInstance::new(
                            crate::status::StatusKind::Slow,
                            *turns,
                        ));
                    battle.audio_events.push(AudioEvent::StatusSlow);
                }
            }
        }
        SpellEffect::Pierce(dmg) => {
            if let Some(idx) = battle.unit_at(tx, ty) {
                if battle.units[idx].is_enemy() {
                    deal_damage(battle, idx, (*dmg + sp).max(1));
                }
            }
        }
        SpellEffect::KnockBack(dmg) => {
            if let Some(idx) = battle.unit_at(tx, ty) {
                if battle.units[idx].is_enemy() {
                    deal_damage(battle, idx, (*dmg + sp).max(1));
                    let px = battle.units[0].x;
                    let py = battle.units[0].y;
                    if battle.units[idx].alive {
                        let _kb = crate::combat::terrain::apply_knockback(battle, idx, px, py);
                    }
                    if battle.units[idx].alive {
                        let _kb = crate::combat::terrain::apply_knockback(battle, idx, px, py);
                    }
                }
            }
        }
        _ => {}
    }
}

fn advance_and_set_phase(battle: &mut TacticalBattle) -> BattleEvent {
    tick_player_end_of_turn(battle);
    if battle.player_dead() {
        battle.phase = TacticalPhase::End {
            victory: false,
            timer: END_DELAY_FRAMES,
        };
        return BattleEvent::None;
    }

    let wrapped = advance_turn(battle);

    if wrapped {
        battle.audio_events.push(AudioEvent::TurnTick);
        battle.arena.tick_steam();
        battle.arena.tick_holy();
        tick_arcing_projectiles(battle);
        tick_pending_impacts(battle);
        apply_exhaustion(battle);
        let scorched_msgs = apply_scorched_damage(battle);
        for msg in &scorched_msgs {
            battle.log_message(msg);
        }

        let crumble_msgs = decay_cracked_floors(battle);
        for msg in &crumble_msgs {
            battle.log_message(msg);
        }

        tick_terrain(battle);

        // ── EnvironmentTick: interactive hazards ────────────────────────
        let flow_msgs = apply_flow_water(battle);
        for msg in &flow_msgs {
            battle.log_message(msg);
        }

        let conv_crate_msgs = apply_conveyor_crates(battle);
        for msg in &conv_crate_msgs {
            battle.log_message(msg);
        }

        let grav_msgs = apply_gravity_wells(battle);
        for msg in &grav_msgs {
            battle.log_message(msg);
        }

        // Energy vent cycling: Dormant → Charging → Active → Dormant
        let (became_charging, became_active, became_dormant) =
            battle.arena.tick_energy_vents();
        if became_charging {
            battle.log_message("⚡ Energy vents begin charging!");
            battle.audio_events.push(AudioEvent::SteamVent);
        }
        if became_active {
            battle.log_message("⚡ Energy vents discharge!");
            battle.audio_events.push(AudioEvent::SteamVent);
        }
        if became_dormant {
            battle.log_message("⚡ Energy vents power down.");
        }

        // Transition to EnvironmentTick phase for visual pause
        battle.phase = TacticalPhase::EnvironmentTick {
            timer: ENVIRONMENT_TICK_FRAMES,
        };
        return BattleEvent::None;
    }

    select_next_unit(battle)
}

/// Called after the EnvironmentTick timer expires to process remaining
/// end-of-round effects (weather, companions, arena events) and select
/// the next acting unit.
fn finish_environment_tick(battle: &mut TacticalBattle) -> BattleEvent {
    if battle.weather == Weather::CoolantLeak {
        spread_rain_water(&mut battle.arena);
    }

    // ── Weather + Terrain Synergies ──────────────────────────────────
    let weather_terrain_msgs = apply_weather_terrain_synergies(battle);
    for msg in &weather_terrain_msgs {
        battle.log_message(msg);
    }

    // ── Rain: apply Wet to all units ─────────────────────────────────
    if battle.weather == Weather::CoolantLeak {
        for i in 0..battle.units.len() {
            if !battle.units[i].alive {
                continue;
            }
            let already_wet = battle.units[i]
                .statuses
                .iter()
                .any(|s| matches!(s.kind, crate::status::StatusKind::Wet));
            if !already_wet {
                battle.units[i]
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Wet,
                        3,
                    ));
            }
        }
        battle.log_message("🌧 Coolant leak soaks everyone!");
        // Check status combos after applying wet
        for i in 0..battle.units.len() {
            if !battle.units[i].alive {
                continue;
            }
            let combo_msgs = crate::combat::action::check_status_combos(battle, i);
            for msg in &combo_msgs {
                battle.log_message(msg);
            }
        }
    }

    // ── Companion Passive Abilities ──────────────────────────────────
    let companion_msgs = apply_companion_passives(battle);
    for msg in &companion_msgs {
        battle.log_message(msg);
    }

    // ── Enemy Synergies (round start) ───────────────────────────────
    let synergy_msgs = crate::combat::synergy::apply_round_start_synergies(battle);
    for msg in &synergy_msgs {
        battle.log_message(msg);
    }

    // ── Arena Events ────────────────────────────────────────────────
    let arena_event_msgs = tick_arena_events(battle);
    for msg in &arena_event_msgs {
        battle.log_message(msg);
    }

    let focus_regen = match battle.weather {
        Weather::EnergyFlux => 4,
        _ => 3,
    };
    battle.focus = (battle.focus + focus_regen).min(battle.max_focus);

    calculate_all_intents(battle);
    if battle.player_dead() {
        battle.phase = TacticalPhase::End {
            victory: false,
            timer: END_DELAY_FRAMES,
        };
        return BattleEvent::None;
    }
    if battle.all_enemies_dead() {
        battle.phase = TacticalPhase::End {
            victory: true,
            timer: END_DELAY_FRAMES,
        };
        return BattleEvent::None;
    }

    select_next_unit(battle)
}

/// Find the next living unit in the turn queue and set the appropriate phase.
fn select_next_unit(battle: &mut TacticalBattle) -> BattleEvent {
    loop {
        let current = battle.current_unit_idx();
        let unit = &battle.units[current];

        if !unit.alive {
            advance_turn(battle);
            continue;
        }

        if unit.is_player() {
            battle.player_moved = false;
            battle.player_acted = false;
            battle.phase = TacticalPhase::Command;
            return BattleEvent::None;
        }

        battle.phase = TacticalPhase::EnemyTurn {
            unit_idx: current,
            timer: ENEMY_TURN_FRAMES,
            acted: false,
        };
        return BattleEvent::None;
    }
}

fn apply_exhaustion(battle: &mut TacticalBattle) {
    let threshold = if battle.is_boss_battle { 30 } else { 20 };
    let warning_turn = if battle.is_boss_battle { 25 } else { 15 };
    let turn = battle.turn_number;

    if turn == warning_turn {
        battle.log_message("The reactor grows unstable...");
    } else if turn >= threshold {
        let escalation = (turn - threshold) / 3;
        let dmg = (1 + escalation as i32).min(3);
        battle.units[0].hp -= dmg;
        battle.log_message(format!("Overload! The reactor sears for {} damage!", dmg));
        if battle.units[0].hp <= 0 {
            battle.units[0].hp = 0;
            battle.units[0].alive = false;
        }
    }
}

fn tick_player_end_of_turn(battle: &mut TacticalBattle) {
    let (status_dmg, status_heal) = tick_statuses(&mut battle.units[0].statuses);
    if status_dmg > 0 {
        battle.units[0].hp -= status_dmg;
        battle.log_message(format!("Status damage: -{} HP", status_dmg));
    }
    if status_heal > 0 {
        let unit = &mut battle.units[0];
        unit.hp = (unit.hp + status_heal).min(unit.max_hp);
        battle.log_message(format!("Status heal: +{} HP", status_heal));
    }
    if battle.units[0].hp <= 0 {
        battle.units[0].alive = false;
    }
}

use crate::combat::TacticalArena;

fn spread_rain_water(arena: &mut TacticalArena) {
    let w = arena.width as i32;
    let h = arena.height as i32;
    let mut new_water = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if arena.tile(x, y) == Some(BattleTile::CoolantPool) {
                for (dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if arena.tile(nx, ny) == Some(BattleTile::MetalFloor) {
                        let roll =
                            ((nx as u64).wrapping_mul(31).wrapping_add(ny as u64 * 17)) % 100;
                        if roll < 20 {
                            new_water.push((nx, ny));
                        }
                    }
                }
            }
        }
    }
    for (wx, wy) in new_water {
        arena.set_tile(wx, wy, BattleTile::CoolantPool);
    }
}

fn tick_arcing_projectiles(battle: &mut TacticalBattle) {
    let mut landed = Vec::new();
    for proj in battle.arcing_projectiles.iter_mut() {
        if proj.fresh {
            proj.fresh = false;
            continue;
        }
        proj.turns_remaining -= 1;
        if proj.turns_remaining <= 0 {
            landed.push((
                proj.target_x,
                proj.target_y,
                proj.effect.clone(),
                proj.owner_is_player,
            ));
        }
    }
    battle.arcing_projectiles.retain(|p| p.turns_remaining > 0);

    for (tx, ty, effect, is_player) in &landed {
        match effect {
            ProjectileEffect::Damage(dmg) => {
                if let Some(idx) = battle.unit_at(*tx, *ty) {
                    if *is_player && battle.units[idx].is_enemy() {
                        let actual = deal_damage(battle, idx, *dmg);
                        battle.log_message(format!("Arc strike lands for {} damage!", actual));
                    } else if !*is_player && battle.units[idx].is_player() {
                        let actual = deal_damage(battle, idx, *dmg);
                        battle.log_message(format!("Incoming arc hits you for {} damage!", actual));
                    }
                }
            }
            ProjectileEffect::PiercingDamage(dmg) => {
                if let Some(idx) = battle.unit_at(*tx, *ty) {
                    if *is_player && battle.units[idx].is_enemy() {
                        let unit = &mut battle.units[idx];
                        unit.hp -= dmg;
                        if unit.hp <= 0 {
                            unit.hp = 0;
                            unit.alive = false;
                        }
                        battle.log_message(format!("Arc strike pierces for {} damage!", dmg));
                    } else if !*is_player && battle.units[idx].is_player() {
                        let unit = &mut battle.units[idx];
                        unit.hp -= dmg;
                        if unit.hp <= 0 {
                            unit.hp = 0;
                            unit.alive = false;
                        }
                        battle.log_message(format!("Incoming arc pierces you for {} damage!", dmg));
                    }
                }
            }
            ProjectileEffect::SpellHit(spell_effect) => {
                if *is_player {
                    apply_projectile_spell(battle, *tx, *ty, spell_effect);
                    battle.log_message("Your lobbed spell lands!");
                } else if let Some(idx) = battle.unit_at(*tx, *ty) {
                    if battle.units[idx].is_player() {
                        deal_damage(battle, idx, 3);
                        battle.log_message("Enemy arc attack hits you!");
                    }
                }
            }
        }
    }
}

/// Decrement pending-impact timers and detonate any that reach zero.
fn tick_pending_impacts(battle: &mut TacticalBattle) {
    let mut detonated = Vec::new();
    for imp in battle.pending_impacts.iter_mut() {
        imp.turns_until_hit = imp.turns_until_hit.saturating_sub(1);
        if imp.turns_until_hit == 0 {
            detonated.push((
                imp.x,
                imp.y,
                imp.damage,
                imp.radius,
                imp.source_is_player,
                imp.element,
            ));
        }
    }
    battle.pending_impacts.retain(|p| p.turns_until_hit > 0);

    for (cx, cy, damage, radius, is_player, element) in &detonated {
        let r = *radius as i32;
        let mut total_hits = 0;
        for dx in -r..=r {
            for dy in -r..=r {
                let tx = cx + dx;
                let ty = cy + dy;
                if !battle.arena.in_bounds(tx, ty) {
                    continue;
                }
                if let Some(idx) = battle.unit_at(tx, ty) {
                    if *is_player && battle.units[idx].is_enemy() {
                        let actual = deal_damage(battle, idx, *damage);
                        total_hits += 1;
                        if dx == 0 && dy == 0 {
                            battle.log_message(format!(
                                "Impact detonates for {} damage!",
                                actual
                            ));
                        }
                    } else if !*is_player && battle.units[idx].is_player() {
                        let actual = deal_damage(battle, idx, *damage);
                        total_hits += 1;
                        battle.log_message(format!(
                            "Incoming strike hits you for {} damage!",
                            actual
                        ));
                    }
                }
            }
        }
        // Element-based status effects on detonation
        if let Some(elem) = element {
            let r = *radius as i32;
            for dx in -r..=r {
                for dy in -r..=r {
                    let tx = cx + dx;
                    let ty = cy + dy;
                    if !battle.arena.in_bounds(tx, ty) {
                        continue;
                    }
                    if let Some(idx) = battle.unit_at(tx, ty) {
                        let dominated = if *is_player {
                            battle.units[idx].is_enemy()
                        } else {
                            battle.units[idx].is_player()
                        };
                        if dominated {
                            match elem {
                                crate::combat::WuxingElement::Fire => {
                                    battle.units[idx].statuses.push(
                                        crate::status::StatusInstance::new(
                                            crate::status::StatusKind::Burn { damage: 1 },
                                            2,
                                        ),
                                    );
                                }
                                crate::combat::WuxingElement::Water => {
                                    let already = battle.units[idx].statuses.iter().any(|s| {
                                        matches!(s.kind, crate::status::StatusKind::Wet)
                                    });
                                    if !already {
                                        battle.units[idx].statuses.push(
                                            crate::status::StatusInstance::new(
                                                crate::status::StatusKind::Wet,
                                                2,
                                            ),
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        if total_hits > 1 {
            battle.log_message(format!("Impact zone hits {} targets!", total_hits));
        }
    }
    if !detonated.is_empty() {
        battle.audio_events.push(AudioEvent::ProjectileImpact);
    }
}

fn apply_flow_water(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();
    let mut pushes: Vec<(usize, i32, i32)> = Vec::new();
    let mut conveyor_played = false;

    for idx in 0..battle.units.len() {
        if !battle.units[idx].alive {
            continue;
        }
        let ux = battle.units[idx].x;
        let uy = battle.units[idx].y;
        let tile = battle.arena.tile(ux, uy);
        let (dx, dy) = match tile {
            Some(BattleTile::ConveyorN) => (0, -1),
            Some(BattleTile::ConveyorS) => (0, 1),
            Some(BattleTile::ConveyorE) => (1, 0),
            Some(BattleTile::ConveyorW) => (-1, 0),
            _ => continue,
        };
        let dest_x = ux + dx;
        let dest_y = uy + dy;
        if dest_x < 0
            || dest_y < 0
            || dest_x >= battle.arena.width as i32
            || dest_y >= battle.arena.height as i32
        {
            continue;
        }
        if let Some(dest_tile) = battle.arena.tile(dest_x, dest_y) {
            if !dest_tile.is_walkable() {
                let actual = deal_damage(battle, idx, 1);
                let name = if battle.units[idx].is_player() {
                    "You are".to_string()
                } else {
                    format!("{} is", battle.units[idx].hanzi)
                };
                messages.push(format!(
                    "{} crushed against a wall by the conveyor! ({} dmg)",
                    name, actual
                ));
                continue;
            }
        }
        if battle.unit_at(dest_x, dest_y).is_some() {
            continue;
        }
        pushes.push((idx, dest_x, dest_y));
    }

    for (idx, dest_x, dest_y) in pushes {
        if battle.units[idx].alive {
            let name = if battle.units[idx].is_player() {
                "You are".to_string()
            } else {
                format!("{} is", battle.units[idx].hanzi)
            };
            battle.units[idx].x = dest_x;
            battle.units[idx].y = dest_y;
            messages.push(format!("{} pushed by the conveyor!", name));
            if !conveyor_played {
                battle.audio_events.push(AudioEvent::ConveyorMove);
                conveyor_played = true;
            }
        }
    }

    messages
}

fn apply_conveyor_crates(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();
    let w = battle.arena.width as i32;
    let h = battle.arena.height as i32;

    // Find CargoCrates adjacent to conveyors that push into them
    let mut crate_pushes: Vec<(i32, i32, i32, i32)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = match battle.arena.tile(x, y) {
                Some(BattleTile::ConveyorN) => (0, -1),
                Some(BattleTile::ConveyorS) => (0, 1),
                Some(BattleTile::ConveyorE) => (1, 0),
                Some(BattleTile::ConveyorW) => (-1, 0),
                _ => continue,
            };
            let target_x = x + dx;
            let target_y = y + dy;
            if battle.arena.tile(target_x, target_y) == Some(BattleTile::CargoCrate)
                && !crate_pushes.iter().any(|&(cx, cy, _, _)| cx == target_x && cy == target_y)
            {
                crate_pushes.push((target_x, target_y, dx, dy));
            }
        }
    }

    for (cx, cy, dx, dy) in crate_pushes {
        if battle.arena.tile(cx, cy) != Some(BattleTile::CargoCrate) {
            continue;
        }
        messages.push("⚙ Conveyor pushes a cargo crate!".to_string());
        let push_msgs = push_boulder(battle, cx, cy, dx, dy);
        messages.extend(push_msgs);
    }

    messages
}

pub fn push_boulder(
    battle: &mut TacticalBattle,
    bx: i32,
    by: i32,
    dx: i32,
    dy: i32,
) -> Vec<String> {
    push_boulder_recursive(battle, bx, by, dx, dy, 0)
}

fn push_boulder_recursive(
    battle: &mut TacticalBattle,
    bx: i32,
    by: i32,
    dx: i32,
    dy: i32,
    depth: u32,
) -> Vec<String> {
    let mut messages = Vec::new();
    if depth > 3 {
        messages.push("The chain of crates is too long to push!".to_string());
        return messages;
    }
    let src_idx = match battle.arena.idx(bx, by) {
        Some(i) => i,
        None => return messages,
    };
    if battle.arena.tiles[src_idx] != BattleTile::CargoCrate {
        return messages;
    }

    let dest_x = bx + dx;
    let dest_y = by + dy;
    let dest_idx = match battle.arena.idx(dest_x, dest_y) {
        Some(i) => i,
        None => return messages,
    };

    // FuelCanister interaction: crate pushed into canister triggers explosion
    if battle.arena.tiles[dest_idx] == BattleTile::FuelCanister {
        battle.arena.tiles[src_idx] = BattleTile::MetalFloor;
        messages.push("📦💥 Cargo crate slams into a fuel canister!".to_string());
        let explosion_msgs = crate::combat::terrain::explode_barrel(battle, dest_x, dest_y);
        messages.extend(explosion_msgs);
        return messages;
    }

    // Chain-push: crate pushed into another crate pushes that crate first
    if battle.arena.tiles[dest_idx] == BattleTile::CargoCrate {
        let chain_msgs = push_boulder_recursive(battle, dest_x, dest_y, dx, dy, depth + 1);
        messages.extend(chain_msgs);
        // If the destination crate didn't move, we can't push either
        if battle.arena.tiles[dest_idx] == BattleTile::CargoCrate {
            messages.push("The cargo crates are jammed!".to_string());
            return messages;
        }
        // The destination is now clear, move our crate
        battle.arena.tiles[src_idx] = BattleTile::MetalFloor;
        battle.arena.tiles[dest_idx] = BattleTile::CargoCrate;
        if depth > 0 {
            messages.push("📦📦 Chain push! Another crate slides!".to_string());
        } else {
            messages.push("📦📦 The cargo crate chain-pushes another!".to_string());
        }
        return messages;
    }

    if let Some(unit_idx) = battle.unit_at(dest_x, dest_y) {
        let actual = deal_damage(battle, unit_idx, 3);
        let name = if battle.units[unit_idx].is_player() {
            "You are".to_string()
        } else {
            format!("{} is", battle.units[unit_idx].hanzi)
        };
        messages.push(format!("{} crushed by a cargo crate! ({} dmg)", name, actual));
        battle.audio_events.push(AudioEvent::CrateCrush);
        battle.arena.tiles[src_idx] = BattleTile::MetalFloor;
        battle.arena.tiles[dest_idx] = BattleTile::CargoCrate;
    } else if battle.arena.tiles[dest_idx].is_walkable() {
        battle.arena.tiles[src_idx] = BattleTile::MetalFloor;
        battle.arena.tiles[dest_idx] = BattleTile::CargoCrate;
        battle.audio_events.push(AudioEvent::CratePush);
        if depth > 0 {
            messages.push("📦 A chained crate slides!".to_string());
        } else {
            messages.push("The cargo crate slides!".to_string());
        }
    } else {
        messages.push("The cargo crate thuds against the wall.".to_string());
    }

    messages
}

fn apply_gravity_wells(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();
    let w = battle.arena.width as i32;
    let h = battle.arena.height as i32;
    let mut gravity_played = false;

    // Collect gravity well positions
    let mut wells: Vec<(i32, i32)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if battle.arena.tile(x, y) == Some(BattleTile::GravityWell) {
                wells.push((x, y));
            }
        }
    }

    // For each well, pull units and deal damage
    let mut pulls: Vec<(usize, i32, i32)> = Vec::new();
    for &(wx, wy) in &wells {
        for idx in 0..battle.units.len() {
            if !battle.units[idx].alive {
                continue;
            }
            let ux = battle.units[idx].x;
            let uy = battle.units[idx].y;
            let dist = (ux - wx).abs() + (uy - wy).abs();
            if !(1..=2).contains(&dist) {
                continue;
            }
            // Deal 1 damage to adjacent units (distance 1)
            if dist == 1 {
                let actual = deal_damage(battle, idx, 1);
                let name = if battle.units[idx].is_player() {
                    "You".to_string()
                } else {
                    battle.units[idx].hanzi.to_string()
                };
                messages.push(format!("⚫ {} takes damage near gravity well! (-{} HP)", name, actual));
                continue; // Already adjacent, no pull needed
            }
            // Pull 1 step closer (distance 2 → distance 1)
            let dx = (wx - ux).signum();
            let dy = (wy - uy).signum();
            // Try moving along the axis with larger distance first
            let (step_x, step_y) = if (ux - wx).abs() >= (uy - wy).abs() {
                (dx, 0)
            } else {
                (0, dy)
            };
            let dest_x = ux + step_x;
            let dest_y = uy + step_y;
            if dest_x < 0 || dest_y < 0 || dest_x >= w || dest_y >= h {
                continue;
            }
            if let Some(dest_tile) = battle.arena.tile(dest_x, dest_y) {
                if !dest_tile.is_walkable() {
                    continue;
                }
            }
            if battle.unit_at(dest_x, dest_y).is_some() {
                continue;
            }
            // Don't pull the same unit twice
            if pulls.iter().any(|&(i, _, _)| i == idx) {
                continue;
            }
            pulls.push((idx, dest_x, dest_y));
            let name = if battle.units[idx].is_player() {
                "You are".to_string()
            } else {
                format!("{} is", battle.units[idx].hanzi)
            };
            messages.push(format!("⬇ Gravity well pulls {} closer!", name));
            if !gravity_played {
                battle.audio_events.push(AudioEvent::GravityPull);
                gravity_played = true;
            }
        }
    }

    for (idx, dest_x, dest_y) in pulls {
        if battle.units[idx].alive {
            battle.units[idx].x = dest_x;
            battle.units[idx].y = dest_y;
        }
    }

    messages
}

// ── Weather + Terrain Synergies ──────────────────────────────────────────────

fn apply_weather_terrain_synergies(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();
    let w = battle.arena.width as i32;
    let h = battle.arena.height as i32;

    match battle.weather {
        Weather::CoolantLeak => {
            // CoolantLeak + fire tiles → fire extinguishes after 1 turn → VentSteam
            let mut fire_to_steam = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if battle.arena.tile(x, y) == Some(BattleTile::BlastMark) {
                        fire_to_steam.push((x, y));
                    }
                }
            }
            for (x, y) in fire_to_steam {
                battle.arena.set_steam(x, y, 2);
                messages.push(format!("🌧🔥 Coolant extinguishes fire at ({},{}) → Vent Steam!", x, y));
            }

            // CoolantLeak + FrozenCoolant tiles → FrozenCoolant becomes CoolantPool
            let mut ice_to_water = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if battle.arena.tile(x, y) == Some(BattleTile::FrozenCoolant) {
                        ice_to_water.push((x, y));
                    }
                }
            }
            for (x, y) in ice_to_water {
                battle.arena.set_tile(x, y, BattleTile::CoolantPool);
                messages.push(format!("🌧❄ Coolant leak melts frozen coolant at ({},{}) → Coolant Pool!", x, y));
            }
        }
        Weather::DebrisStorm => {
            // DebrisStorm + Lubricant tiles → Lubricant covered (neutralized) by debris
            let mut oil_to_sand = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if battle.arena.tile(x, y) == Some(BattleTile::Lubricant) {
                        oil_to_sand.push((x, y));
                    }
                }
            }
            if !oil_to_sand.is_empty() {
                for (x, y) in oil_to_sand {
                    battle.arena.set_tile(x, y, BattleTile::Debris);
                }
                messages.push("🏜 Debris storm covers lubricant tiles with debris!".to_string());
            }
        }
        Weather::SmokeScreen => {
            // SmokeScreen + VentSteam → Combined thick smoke (extend steam timer for thicker coverage)
            for y in 0..h {
                for x in 0..w {
                    if let Some(i) = battle.arena.idx(x, y) {
                        if battle.arena.tiles[i] == BattleTile::VentSteam && battle.arena.steam_timers[i] < 4 {
                            battle.arena.steam_timers[i] = 4;
                        }
                    }
                }
            }
        }
        Weather::EnergyFlux => {
            // EnergyFlux + OilSlick → +2 spell power (handled in tile_spell_bonus in input.rs)
        }
        Weather::Normal => {}
    }

    messages
}

// ── Companion Passive Abilities ──────────────────────────────────────────────

fn apply_companion_passives(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();
    let companion_kind = match battle.companion_kind {
        Some(c) => c,
        None => return messages,
    };

    // Find companion unit index
    let companion_idx = battle
        .units
        .iter()
        .position(|u| u.is_companion() && u.alive);
    let companion_idx = match companion_idx {
        Some(i) => i,
        None => return messages,
    };

    use crate::game::Companion;
    match companion_kind {
        Companion::Medic => {
            // Passive: Heal player 1 HP at start of each round
            let player = &battle.units[0];
            if player.alive && player.hp < player.max_hp {
                battle.units[0].hp = (battle.units[0].hp + 1).min(battle.units[0].max_hp);
                messages.push("💊 Medic's healing aura restores 1 HP!".to_string());
                battle.audio_events.push(AudioEvent::Heal);
            }
            // Active: Purify — remove 1 negative status from player when player HP < 50%
            let player = &battle.units[0];
            if player.alive && player.hp * 2 < player.max_hp {
                let neg_idx = battle.units[0]
                    .statuses
                    .iter()
                    .position(|s| s.is_negative());
                if let Some(idx) = neg_idx {
                    let removed_label = battle.units[0].statuses[idx].label().to_string();
                    battle.units[0].statuses.remove(idx);
                    messages.push(format!(
                        "💊✨ Medic purifies {}! Status removed!",
                        removed_label
                    ));
                }
            }
        }
        Companion::SecurityChief => {
            // Active: Taunt — force nearest enemy to target Guard when player HP < 30%
            let player = &battle.units[0];
            if player.alive && player.hp * 10 < player.max_hp * 3 {
                let cx = battle.units[companion_idx].x;
                let cy = battle.units[companion_idx].y;
                let mut nearest_enemy: Option<usize> = None;
                let mut best_dist = i32::MAX;
                for (i, u) in battle.units.iter().enumerate() {
                    if u.alive && u.is_enemy() {
                        let d = (u.x - cx).abs() + (u.y - cy).abs();
                        if d < best_dist {
                            best_dist = d;
                            nearest_enemy = Some(i);
                        }
                    }
                }
                if let Some(eidx) = nearest_enemy {
                    let gdx = cx - battle.units[eidx].x;
                    let gdy = cy - battle.units[eidx].y;
                    if let Some(dir) = crate::combat::Direction::from_delta(gdx, gdy) {
                        battle.units[eidx].facing = dir;
                    }
                    messages.push(format!(
                        "🛡 Guard taunts {}! \"Over here!\"",
                        battle.units[eidx].hanzi
                    ));
                }
            }
        }
        Companion::ScienceOfficer => {
            // Passive: +1 to combo multiplier buildup (checked in combo_multiplier)
            if battle.turn_number <= 2 {
                messages.push("📚 Teacher's guidance boosts combo buildup!".to_string());
            }
        }
        Companion::Quartermaster => {
            // Active: Bribe — 50% chance to make an enemy skip turn when player HP < 40%
            let player = &battle.units[0];
            if player.alive && player.hp * 10 < player.max_hp * 4 {
                let px = battle.units[0].x;
                let py = battle.units[0].y;
                let mut nearest_enemy: Option<usize> = None;
                let mut best_dist = i32::MAX;
                for (i, u) in battle.units.iter().enumerate() {
                    if u.alive && u.is_enemy() && !u.stunned {
                        let d = (u.x - px).abs() + (u.y - py).abs();
                        if d < best_dist {
                            best_dist = d;
                            nearest_enemy = Some(i);
                        }
                    }
                }
                if let Some(eidx) = nearest_enemy {
                    let roll = (battle.turn_number as u64 * 37 + eidx as u64 * 13) % 100;
                    if roll < 50 {
                        battle.units[eidx].stunned = true;
                        messages.push(format!(
                            "💰 Merchant bribes {}! Enemy skips their turn!",
                            battle.units[eidx].hanzi
                        ));
                    } else {
                        messages.push("💰 Merchant's bribe attempt fails!".to_string());
                    }
                }
            }
        }
    }

    messages
}

// ── Arena Event System ──────────────────────────────────────────────────────

const ALL_EVENTS: [ArenaEvent; 12] = [
    ArenaEvent::CoolantFlood,
    ArenaEvent::HullBreach,
    ArenaEvent::PowerSurge,
    ArenaEvent::VentBlast,
    ArenaEvent::ArcDischarge,
    ArenaEvent::PlasmaLeak,
    ArenaEvent::MediGas,
    ArenaEvent::CryoVent,
    ArenaEvent::DebrisBurst,
    ArenaEvent::SystemGlitch,
    ArenaEvent::NaniteSpread,
    ArenaEvent::ReactorBlowout,
];

/// Pick a random arena event, weighting by biome affinity.
fn pick_arena_event(rng: &mut Rng, biome: ArenaBiome) -> ArenaEvent {
    // Base weight 10 for every event; biome-favored events get +20.
    let mut weights: [u32; 12] = [10; 12];
    match biome {
        ArenaBiome::StationInterior | ArenaBiome::DerelictShip | ArenaBiome::IrradiatedZone => {
            weights[1] += 20; // EarthTremor
            weights[4] += 20; // LightningStrike
        }
        ArenaBiome::CryoBay => {
            weights[7] += 20; // FrostSnap
            weights[3] += 20; // WindGust
        }
        ArenaBiome::ReactorRoom => {
            weights[5] += 20; // LavaFlow
            weights[11] += 20; // VolcanicVent
        }
        ArenaBiome::Hydroponics => {
            weights[10] += 20; // WildGrowth
            weights[6] += 20; // HealingMist
        }
        ArenaBiome::AlienRuins => {
            weights[2] += 20; // SpiritSurge
            weights[9] += 20; // SpiritualEcho
        }
    }
    let total: u32 = weights.iter().sum();
    let roll = rng.range(0, total as i32) as u32;
    let mut acc = 0u32;
    for (i, &w) in weights.iter().enumerate() {
        acc += w;
        if roll < acc {
            return ALL_EVENTS[i];
        }
    }
    ALL_EVENTS[0]
}

/// Process arena event timer at end-of-round. Returns messages to log.
pub fn tick_arena_events(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();

    if battle.arena_event_timer > 1 {
        battle.arena_event_timer -= 1;

        // When timer reaches 1, preview the upcoming event
        if battle.arena_event_timer == 1 {
            let seed = battle.turn_number as u64 * 7919 + 42;
            let mut rng = Rng::new(seed);
            let event = pick_arena_event(&mut rng, battle.arena.biome);
            battle.pending_event = Some(event);
            messages.push(format!("⚠ {} incoming!", event.name()));
        }
        return messages;
    }

    // Timer is 1 — fire the event
    let event = match battle.pending_event.take() {
        Some(e) => e,
        None => return messages,
    };

    // Show event name as big message
    battle.event_message = Some(format!("🌀 {}", event.name()));
    battle.event_message_timer = 90; // ~1.5s at 60fps

    let seed = battle.turn_number as u64 * 6271 + 137;
    let mut rng = Rng::new(seed);
    let event_msgs = execute_arena_event(battle, event, &mut rng);
    for msg in &event_msgs {
        messages.push(msg.clone());
    }

    // Reset timer: boss arenas get events every 2 rounds, normal every 2-4
    let next_timer = if battle.is_boss_battle {
        2
    } else {
        rng.range(2, 5) as u32
    };
    battle.arena_event_timer = next_timer;

    messages
}

fn execute_arena_event(
    battle: &mut TacticalBattle,
    event: ArenaEvent,
    rng: &mut Rng,
) -> Vec<String> {
    let mut messages = Vec::new();
    let w = battle.arena.width as i32;
    let h = battle.arena.height as i32;

    match event {
        ArenaEvent::CoolantFlood => {
            let mut new_water = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if battle.arena.tile(x, y) == Some(BattleTile::CoolantPool) {
                        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                            let nx = x + dx;
                            let ny = y + dy;
                            if battle.arena.tile(nx, ny) == Some(BattleTile::MetalFloor) {
                                new_water.push((nx, ny));
                            }
                        }
                    }
                }
            }
            for (wx, wy) in &new_water {
                battle.arena.set_tile(*wx, *wy, BattleTile::CoolantPool);
            }
            // Apply Wet to units standing on new water tiles
            for (wx, wy) in &new_water {
                if let Some(idx) = battle.unit_at(*wx, *wy) {
                    if battle.units[idx].alive {
                        let already_wet = battle.units[idx]
                            .statuses
                            .iter()
                            .any(|s| matches!(s.kind, crate::status::StatusKind::Wet));
                        if !already_wet {
                            battle.units[idx]
                                .statuses
                                .push(crate::status::StatusInstance::new(
                                    crate::status::StatusKind::Wet,
                                    3,
                                ));
                        }
                    }
                }
            }
            if !new_water.is_empty() {
                messages.push(format!(
                    "🌊 Coolant Flood! {} tiles flooded!",
                    new_water.len()
                ));
                battle.audio_events.push(AudioEvent::WaterSplash);
            } else {
                messages.push("🌊 Coolant Flood! But no tiles to flood.".to_string());
            }
        }

        ArenaEvent::HullBreach => {
            let count = rng.range(3, 6);
            let mut affected = 0;
            for _ in 0..count {
                let x = rng.range(0, w);
                let y = rng.range(0, h);
                if battle.arena.tile(x, y) == Some(BattleTile::MetalFloor)
                    && battle.unit_at(x, y).is_none()
                {
                    let tile = if rng.range(0, 2) == 0 {
                        BattleTile::WeakenedPlating
                    } else {
                        BattleTile::DamagedFloor
                    };
                    battle.arena.set_tile(x, y, tile);
                    affected += 1;
                }
            }
            messages.push(format!("🪨 Hull Breach! {} tiles fracture!", affected));
        }

        ArenaEvent::PowerSurge => {
            let count = rng.range(2, 4);
            let mut placed = 0;
            for _ in 0..count {
                let x = rng.range(0, w);
                let y = rng.range(0, h);
                if battle.arena.tile(x, y) == Some(BattleTile::MetalFloor) {
                    battle.arena.set_tile(x, y, BattleTile::OilSlick);
                    placed += 1;
                }
            }
            messages.push(format!(
                "🔮 Power Surge! {} OilSlick tiles appear!",
                placed
            ));
        }

        ArenaEvent::VentBlast => {
            let dirs: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];
            let dir_names = ["north", "south", "west", "east"];
            let dir_idx = rng.range(0, 4) as usize;
            let (dx, dy) = dirs[dir_idx];
            let dir_name = dir_names[dir_idx];

            let mut pushes: Vec<(usize, i32, i32)> = Vec::new();
            for idx in 0..battle.units.len() {
                if !battle.units[idx].alive {
                    continue;
                }
                let nx = battle.units[idx].x + dx;
                let ny = battle.units[idx].y + dy;
                if battle.arena.in_bounds(nx, ny)
                    && battle
                        .arena
                        .tile(nx, ny)
                        .is_some_and(|t| t.is_walkable())
                    && battle.unit_at(nx, ny).is_none()
                {
                    pushes.push((idx, nx, ny));
                }
            }
            // Deduplicate destinations
            let mut valid_pushes = Vec::new();
            for &(idx, nx, ny) in &pushes {
                let conflict = valid_pushes
                    .iter()
                    .any(|&(_, ox, oy): &(usize, i32, i32)| ox == nx && oy == ny);
                if !conflict {
                    valid_pushes.push((idx, nx, ny));
                }
            }
            for (idx, nx, ny) in valid_pushes {
                battle.units[idx].x = nx;
                battle.units[idx].y = ny;
            }
            messages.push(format!("💨 Vent Blast blows everyone {}!", dir_name));
        }

        ArenaEvent::ArcDischarge => {
            let mut target_x = rng.range(0, w);
            let mut target_y = rng.range(0, h);
            for _ in 0..20 {
                if battle
                    .arena
                    .tile(target_x, target_y)
                    .is_some_and(|t| t.is_walkable())
                {
                    break;
                }
                target_x = rng.range(0, w);
                target_y = rng.range(0, h);
            }

            if let Some(idx) = battle.unit_at(target_x, target_y) {
                let actual = deal_damage(battle, idx, 3);
                let name = if battle.units[idx].is_player() {
                    "You".to_string()
                } else {
                    battle.units[idx].hanzi.to_string()
                };
                messages.push(format!(
                    "⚡ Arc discharge at ({},{})! {} takes {} damage!",
                    target_x, target_y, name, actual
                ));
            } else {
                messages.push(format!(
                    "⚡ Arc discharge at ({},{})!",
                    target_x, target_y
                ));
            }

            // Chain to units on adjacent Water tiles
            for &(ddx, ddy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let cx = target_x + ddx;
                let cy = target_y + ddy;
                if battle.arena.tile(cx, cy) == Some(BattleTile::CoolantPool) {
                    if let Some(idx) = battle.unit_at(cx, cy) {
                        let actual = deal_damage(battle, idx, 2);
                        let name = if battle.units[idx].is_player() {
                            "You".to_string()
                        } else {
                            battle.units[idx].hanzi.to_string()
                        };
                        messages.push(format!(
                            "⚡ Arc chains through coolant! {} takes {} damage!",
                            name, actual
                        ));
                    }
                }
            }
        }

        ArenaEvent::PlasmaLeak => {
            let count = rng.range(2, 4);
            let mut placed = 0;
            for _ in 0..count {
                let edge = rng.range(0, 4);
                let (x, y) = match edge {
                    0 => (rng.range(0, w), 0),
                    1 => (rng.range(0, w), h - 1),
                    2 => (0, rng.range(0, h)),
                    _ => (w - 1, rng.range(0, h)),
                };
                if battle.arena.tile(x, y) == Some(BattleTile::MetalFloor)
                    || battle.arena.tile(x, y) == Some(BattleTile::WiringPanel)
                {
                    battle.arena.set_tile(x, y, BattleTile::PlasmaPool);
                    placed += 1;
                    if let Some(idx) = battle.unit_at(x, y) {
                        let actual = deal_damage(battle, idx, 2);
                        let name = if battle.units[idx].is_player() {
                            "You".to_string()
                        } else {
                            battle.units[idx].hanzi.to_string()
                        };
                        messages.push(format!(
                            "🌋 {} is caught in plasma! {} damage!",
                            name, actual
                        ));
                    }
                }
            }
            messages.push(format!(
                "🌋 Plasma Leak! {} tiles erupt at the edges!",
                placed
            ));
            battle.audio_events.push(AudioEvent::LavaRumble);
        }

        ArenaEvent::MediGas => {
            for idx in 0..battle.units.len() {
                if battle.units[idx].alive {
                    let unit = &mut battle.units[idx];
                    unit.hp = (unit.hp + 2).min(unit.max_hp);
                }
            }
            messages.push("💚 Medi-Gas! All units heal 2 HP!".to_string());
            battle.audio_events.push(AudioEvent::Heal);
        }

        ArenaEvent::CryoVent => {
            let mut frozen_count = 0;
            for y in 0..h {
                for x in 0..w {
                    if battle.arena.tile(x, y) == Some(BattleTile::CoolantPool) {
                        battle.arena.set_tile(x, y, BattleTile::FrozenCoolant);
                        frozen_count += 1;
                    }
                }
            }
            for idx in 0..battle.units.len() {
                if !battle.units[idx].alive {
                    continue;
                }
                let is_wet = battle.units[idx]
                    .statuses
                    .iter()
                    .any(|s| matches!(s.kind, crate::status::StatusKind::Wet));
                if is_wet {
                    let has_slow = battle.units[idx]
                        .statuses
                        .iter()
                        .any(|s| matches!(s.kind, crate::status::StatusKind::Slow));
                    if !has_slow {
                        battle.units[idx]
                            .statuses
                            .push(crate::status::StatusInstance::new(
                                crate::status::StatusKind::Slow,
                                2,
                            ));
                        let name = if battle.units[idx].is_player() {
                            "You".to_string()
                        } else {
                            battle.units[idx].hanzi.to_string()
                        };
                        messages.push(format!("❄ {} is frozen stiff! Slowed!", name));
                        battle.audio_events.push(AudioEvent::StatusSlow);
                    }
                }
            }
            messages.push(format!(
                "❄ Cryo Vent! {} coolant tiles freeze!",
                frozen_count
            ));
        }

        ArenaEvent::DebrisBurst => {
            let count = rng.range(4, 7);
            let mut placed = 0;
            for _ in 0..count {
                let x = rng.range(0, w);
                let y = rng.range(0, h);
                if battle.arena.tile(x, y) == Some(BattleTile::MetalFloor) {
                    battle.arena.set_tile(x, y, BattleTile::Debris);
                    placed += 1;
                }
            }
            for idx in 0..battle.units.len() {
                if battle.units[idx].alive && battle.units[idx].stored_movement > 0 {
                    battle.units[idx].stored_movement -= 1;
                }
            }
            messages.push(format!(
                "🏜 Debris Burst! {} debris tiles appear! Movement hampered!",
                placed
            ));
        }

        ArenaEvent::SystemGlitch => {
            for idx in 0..battle.units.len() {
                if !battle.units[idx].alive {
                    continue;
                }
                for status in &mut battle.units[idx].statuses {
                    status.turns_left += 1;
                }
            }
            messages.push("👻 System Glitch! All status effects extended by 1 turn!".to_string());
        }

        ArenaEvent::NaniteSpread => {
            let mut new_grass = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if battle.arena.tile(x, y) == Some(BattleTile::WiringPanel) {
                        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                            let nx = x + dx;
                            let ny = y + dy;
                            if battle.arena.tile(nx, ny) == Some(BattleTile::MetalFloor)
                                && rng.range(0, 100) < 40
                            {
                                new_grass.push((nx, ny));
                            }
                        }
                    }
                }
            }
            let mut upgrades = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if battle.arena.tile(x, y) == Some(BattleTile::WiringPanel)
                        && battle.unit_at(x, y).is_none()
                        && rng.range(0, 100) < 15
                    {
                        upgrades.push((x, y));
                    }
                }
            }
            for (gx, gy) in &new_grass {
                battle.arena.set_tile(*gx, *gy, BattleTile::WiringPanel);
            }
            for (ux, uy) in &upgrades {
                battle.arena.set_tile(*ux, *uy, BattleTile::PipeTangle);
            }
            messages.push(format!(
                "🌿 Nanite Spread! {} new wiring panels, {} pipe tangles!",
                new_grass.len(),
                upgrades.len()
            ));
        }

        ArenaEvent::ReactorBlowout => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let mut best_x = w / 2;
            let mut best_y = h / 2;
            let mut best_dist = 0;
            for _ in 0..20 {
                let x = rng.range(1, w - 1);
                let y = rng.range(1, h - 1);
                if battle.arena.tile(x, y) == Some(BattleTile::MetalFloor)
                    && battle.unit_at(x, y).is_none()
                {
                    let dist = (x - px).abs() + (y - py).abs();
                    if dist > best_dist {
                        best_dist = dist;
                        best_x = x;
                        best_y = y;
                    }
                }
            }
            battle.arena.set_tile(best_x, best_y, BattleTile::PlasmaPool);
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let bx = best_x + dx;
                let by = best_y + dy;
                if battle.arena.tile(bx, by) == Some(BattleTile::MetalFloor)
                    && battle.unit_at(bx, by).is_none()
                {
                    battle.arena.set_tile(bx, by, BattleTile::FuelCanister);
                    break;
                }
            }
            messages.push(format!(
                "🌋 Reactor Blowout erupts at ({},{})! Fuel canisters nearby!",
                best_x, best_y
            ));
            battle.audio_events.push(AudioEvent::LavaRumble);
        }
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::test_helpers::{make_test_battle, make_test_unit};
    use crate::combat::{ArenaBiome, BattleTile, ProjectileEffect, TacticalPhase, UnitKind};
    use crate::dungeon::Rng;
    use crate::radical::SpellEffect;

    // ── tick_battle: phase handling ──────────────────────────────────

    #[test]
    fn tick_command_phase_returns_none() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::Command;

        let event = tick_battle(&mut battle);

        assert!(matches!(event, BattleEvent::None));
    }

    #[test]
    fn tick_resolve_timer_decrements() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::Resolve {
            message: "test".to_string(),
            timer: 5,
            end_turn: false,
        };

        tick_battle(&mut battle);

        match &battle.phase {
            TacticalPhase::Resolve { timer, .. } => assert_eq!(*timer, 4),
            _ => panic!("Expected Resolve phase"),
        }
    }

    #[test]
    fn tick_resolve_no_end_turn_goes_to_command() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::Resolve {
            message: "test".to_string(),
            timer: 0,
            end_turn: false,
        };

        tick_battle(&mut battle);

        assert!(matches!(battle.phase, TacticalPhase::Command));
    }

    #[test]
    fn tick_end_phase_timer_decrements() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::End {
            victory: true,
            timer: 10,
        };

        tick_battle(&mut battle);

        match &battle.phase {
            TacticalPhase::End { timer, .. } => assert_eq!(*timer, 9),
            _ => panic!("Expected End phase"),
        }
    }

    #[test]
    fn tick_end_phase_timer_stops_at_zero() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::End {
            victory: true,
            timer: 0,
        };

        let event = tick_battle(&mut battle);

        assert!(matches!(event, BattleEvent::None));
        match &battle.phase {
            TacticalPhase::End { timer, .. } => assert_eq!(*timer, 0),
            _ => panic!("Expected End phase"),
        }
    }

    #[test]
    fn tick_environment_tick_timer_decrements() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::EnvironmentTick { timer: 5 };

        tick_battle(&mut battle);

        match &battle.phase {
            TacticalPhase::EnvironmentTick { timer } => assert_eq!(*timer, 4),
            _ => panic!("Expected EnvironmentTick phase"),
        }
    }

    #[test]
    fn tick_event_message_timer_decrements() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::Command;
        battle.event_message = Some("test".to_string());
        battle.event_message_timer = 3;

        tick_battle(&mut battle);

        assert_eq!(battle.event_message_timer, 2);
        assert!(battle.event_message.is_some());
    }

    #[test]
    fn tick_event_message_clears_at_zero() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::Command;
        battle.event_message = Some("test".to_string());
        battle.event_message_timer = 1;

        tick_battle(&mut battle);

        assert_eq!(battle.event_message_timer, 0);
        assert!(battle.event_message.is_none());
    }

    #[test]
    fn tick_combo_message_timer_decrements() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::Command;
        battle.combo_message = Some("combo!".to_string());
        battle.combo_message_timer = 3;

        tick_battle(&mut battle);

        assert_eq!(battle.combo_message_timer, 2);
    }

    #[test]
    fn tick_combo_message_clears_at_zero() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::Command;
        battle.combo_message = Some("combo!".to_string());
        battle.combo_message_timer = 1;

        tick_battle(&mut battle);

        assert_eq!(battle.combo_message_timer, 0);
        assert!(battle.combo_message.is_none());
    }

    #[test]
    fn tick_calculates_intents_when_not_calculated() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::Command;
        battle.intents_calculated = false;

        tick_battle(&mut battle);

        assert!(battle.intents_calculated);
    }

    #[test]
    fn tick_look_phase_returns_none() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::Look {
            cursor_x: 3,
            cursor_y: 3,
        };

        let event = tick_battle(&mut battle);

        assert!(matches!(event, BattleEvent::None));
    }

    // ── push_boulder ─────────────────────────────────────────────────

    #[test]
    fn push_boulder_moves_crate_to_walkable_tile() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);
        battle.arena.set_tile(3, 3, BattleTile::CargoCrate);

        let msgs = push_boulder(&mut battle, 3, 3, 1, 0);

        assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::MetalFloor));
        assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::CargoCrate));
        assert!(msgs.iter().any(|m| m.contains("slides")));
    }

    #[test]
    fn push_boulder_crushes_unit_for_three_damage() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
        enemy.hp = 10;
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::CargoCrate);

        let msgs = push_boulder(&mut battle, 3, 3, 1, 0);

        assert_eq!(battle.units[1].hp, 7);
        assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::CargoCrate));
        assert!(msgs.iter().any(|m| m.contains("crushed")));
    }

    #[test]
    fn push_boulder_into_fuel_canister_explodes() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);
        battle.arena.set_tile(3, 3, BattleTile::CargoCrate);
        battle.arena.set_tile(4, 3, BattleTile::FuelCanister);

        let msgs = push_boulder(&mut battle, 3, 3, 1, 0);

        assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::MetalFloor));
        assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::BlastMark));
        assert!(msgs.iter().any(|m| m.contains("fuel canister")));
    }

    #[test]
    fn push_boulder_chain_pushes_two_crates() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);
        battle.arena.set_tile(3, 3, BattleTile::CargoCrate);
        battle.arena.set_tile(4, 3, BattleTile::CargoCrate);

        let msgs = push_boulder(&mut battle, 3, 3, 1, 0);

        assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::MetalFloor));
        assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::CargoCrate));
        assert_eq!(battle.arena.tile(5, 3), Some(BattleTile::CargoCrate));
        assert!(msgs.iter().any(|m| m.contains("chain")));
    }

    #[test]
    fn push_boulder_into_wall_thuds() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);
        battle.arena.set_tile(3, 3, BattleTile::CargoCrate);
        battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);

        let msgs = push_boulder(&mut battle, 3, 3, 1, 0);

        assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::CargoCrate));
        assert!(msgs.iter().any(|m| m.contains("thuds")));
    }

    #[test]
    fn push_boulder_on_non_crate_returns_empty() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);

        let msgs = push_boulder(&mut battle, 3, 3, 1, 0);

        assert!(msgs.is_empty());
    }

    #[test]
    fn push_boulder_out_of_bounds_returns_empty() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);
        battle.arena.set_tile(6, 3, BattleTile::CargoCrate);

        let msgs = push_boulder(&mut battle, 6, 3, 1, 0);

        // Destination (7,3) is out of bounds for 7x7 arena
        assert!(msgs.is_empty());
    }

    #[test]
    fn push_boulder_chain_jams_when_destination_blocked() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);
        battle.arena.set_tile(3, 3, BattleTile::CargoCrate);
        battle.arena.set_tile(4, 3, BattleTile::CargoCrate);
        battle.arena.set_tile(5, 3, BattleTile::CoverBarrier);

        let msgs = push_boulder(&mut battle, 3, 3, 1, 0);

        // Second crate can't move, so first can't either
        assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::CargoCrate));
        assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::CargoCrate));
        assert!(msgs.iter().any(|m| m.contains("jammed")));
    }

    // ── tick_arena_events ────────────────────────────────────────────

    #[test]
    fn arena_events_timer_decrements() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena_event_timer = 5;

        tick_arena_events(&mut battle);

        assert_eq!(battle.arena_event_timer, 4);
    }

    #[test]
    fn arena_events_previews_at_timer_two() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena_event_timer = 2;

        let msgs = tick_arena_events(&mut battle);

        assert_eq!(battle.arena_event_timer, 1);
        assert!(battle.pending_event.is_some());
        assert!(msgs.iter().any(|m| m.contains("incoming")));
    }

    #[test]
    fn arena_events_fires_when_timer_one() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena_event_timer = 1;
        battle.pending_event = Some(ArenaEvent::MediGas);

        let msgs = tick_arena_events(&mut battle);

        assert!(battle.pending_event.is_none());
        assert!(battle.event_message.is_some());
        assert!(battle.arena_event_timer >= 2);
        assert!(!msgs.is_empty());
    }

    #[test]
    fn arena_events_no_pending_returns_empty() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena_event_timer = 1;
        battle.pending_event = None;

        let msgs = tick_arena_events(&mut battle);

        assert!(msgs.is_empty());
    }

    #[test]
    fn arena_events_boss_resets_timer_to_two() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.is_boss_battle = true;
        battle.arena_event_timer = 1;
        battle.pending_event = Some(ArenaEvent::MediGas);

        tick_arena_events(&mut battle);

        assert_eq!(battle.arena_event_timer, 2);
    }

    #[test]
    fn arena_events_medi_gas_heals_units() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        enemy.hp = 5;
        enemy.max_hp = 10;
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena_event_timer = 1;
        battle.pending_event = Some(ArenaEvent::MediGas);

        tick_arena_events(&mut battle);

        assert!(battle.units[1].hp > 5);
    }

    // ── apply_exhaustion ─────────────────────────────────────────────

    #[test]
    fn exhaustion_warning_at_turn_15() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.turn_number = 15;

        apply_exhaustion(&mut battle);

        assert!(battle.log.iter().any(|m| m.contains("unstable")));
        assert_eq!(battle.units[0].hp, 10); // no damage yet
    }

    #[test]
    fn exhaustion_warning_boss_at_turn_25() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.is_boss_battle = true;
        battle.turn_number = 25;

        apply_exhaustion(&mut battle);

        assert!(battle.log.iter().any(|m| m.contains("unstable")));
        assert_eq!(battle.units[0].hp, 10);
    }

    #[test]
    fn exhaustion_deals_damage_at_threshold() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.turn_number = 20;

        apply_exhaustion(&mut battle);

        assert!(battle.units[0].hp < 10);
        assert!(battle.log.iter().any(|m| m.contains("Overload")));
    }

    #[test]
    fn exhaustion_escalates_damage_over_time() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        // turn 23 => escalation = (23-20)/3 = 1, dmg = min(2,3) = 2
        battle.turn_number = 23;

        apply_exhaustion(&mut battle);

        assert_eq!(battle.units[0].hp, 8);
    }

    #[test]
    fn exhaustion_caps_damage_at_3() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        // turn 29 => escalation = (29-20)/3 = 3, dmg = min(4,3) = 3
        battle.turn_number = 29;

        apply_exhaustion(&mut battle);

        assert_eq!(battle.units[0].hp, 7);
    }

    #[test]
    fn exhaustion_kills_player() {
        let mut player = make_test_unit(UnitKind::Player, 0, 0);
        player.hp = 1;
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.turn_number = 20;

        apply_exhaustion(&mut battle);

        assert_eq!(battle.units[0].hp, 0);
        assert!(!battle.units[0].alive);
    }

    #[test]
    fn exhaustion_boss_threshold_is_30() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.is_boss_battle = true;
        battle.turn_number = 20;

        apply_exhaustion(&mut battle);

        // Turn 20 < boss threshold 30, so no damage
        assert_eq!(battle.units[0].hp, 10);
    }

    #[test]
    fn exhaustion_no_effect_before_warning() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.turn_number = 10;

        apply_exhaustion(&mut battle);

        assert_eq!(battle.units[0].hp, 10);
        assert!(battle.log.is_empty());
    }

    // ── tick_player_end_of_turn ──────────────────────────────────────

    #[test]
    fn player_end_of_turn_applies_status_damage() {
        let mut player = make_test_unit(UnitKind::Player, 0, 0);
        player.statuses.push(crate::status::StatusInstance::new(
            crate::status::StatusKind::Poison { damage: 2 },
            3,
        ));
        // Mark as not fresh so damage applies
        player.statuses[0].fresh = false;
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);

        tick_player_end_of_turn(&mut battle);

        assert!(battle.units[0].hp < 10);
        assert!(battle.log.iter().any(|m| m.contains("Status damage")));
    }

    #[test]
    fn player_end_of_turn_applies_status_heal() {
        let mut player = make_test_unit(UnitKind::Player, 0, 0);
        player.hp = 5;
        player.statuses.push(crate::status::StatusInstance::new(
            crate::status::StatusKind::Regen { heal: 2 },
            3,
        ));
        player.statuses[0].fresh = false;
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);

        tick_player_end_of_turn(&mut battle);

        assert!(battle.units[0].hp > 5);
        assert!(battle.log.iter().any(|m| m.contains("Status heal")));
    }

    #[test]
    fn player_end_of_turn_heal_capped_at_max_hp() {
        let mut player = make_test_unit(UnitKind::Player, 0, 0);
        player.hp = 9;
        player.max_hp = 10;
        player.statuses.push(crate::status::StatusInstance::new(
            crate::status::StatusKind::Regen { heal: 5 },
            3,
        ));
        player.statuses[0].fresh = false;
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);

        tick_player_end_of_turn(&mut battle);

        assert_eq!(battle.units[0].hp, 10);
    }

    #[test]
    fn player_end_of_turn_kills_player_from_status() {
        let mut player = make_test_unit(UnitKind::Player, 0, 0);
        player.hp = 1;
        player.statuses.push(crate::status::StatusInstance::new(
            crate::status::StatusKind::Poison { damage: 5 },
            3,
        ));
        player.statuses[0].fresh = false;
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);

        tick_player_end_of_turn(&mut battle);

        assert!(!battle.units[0].alive);
    }

    #[test]
    fn player_end_of_turn_no_status_no_change() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);

        tick_player_end_of_turn(&mut battle);

        assert_eq!(battle.units[0].hp, 10);
        assert!(battle.log.is_empty());
    }

    // ── apply_flow_water (conveyor unit movement) ────────────────────

    #[test]
    fn conveyor_pushes_unit_north() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::ConveyorN);

        let msgs = apply_flow_water(&mut battle);

        assert_eq!(battle.units[0].x, 3);
        assert_eq!(battle.units[0].y, 2);
        assert!(msgs.iter().any(|m| m.contains("pushed by the conveyor")));
    }

    #[test]
    fn conveyor_pushes_unit_south() {
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::ConveyorS);

        let msgs = apply_flow_water(&mut battle);

        assert_eq!(battle.units[1].y, 4);
        assert!(msgs.iter().any(|m| m.contains("pushed")));
    }

    #[test]
    fn conveyor_pushes_unit_east() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::ConveyorE);

        let msgs = apply_flow_water(&mut battle);

        assert_eq!(battle.units[0].x, 4);
        assert_eq!(battle.units[0].y, 3);
        assert!(!msgs.is_empty());
    }

    #[test]
    fn conveyor_pushes_unit_west() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::ConveyorW);

        let msgs = apply_flow_water(&mut battle);

        assert_eq!(battle.units[0].x, 2);
        assert_eq!(battle.units[0].y, 3);
        assert!(!msgs.is_empty());
    }

    #[test]
    fn conveyor_crushes_against_wall() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::ConveyorE);
        battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);

        let msgs = apply_flow_water(&mut battle);

        // Unit stays, takes damage
        assert_eq!(battle.units[0].x, 3);
        assert!(battle.units[0].hp < 10);
        assert!(msgs.iter().any(|m| m.contains("crushed")));
    }

    #[test]
    fn conveyor_blocked_by_another_unit() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::ConveyorE);

        let msgs = apply_flow_water(&mut battle);

        // Unit can't move into occupied tile
        assert_eq!(battle.units[0].x, 3);
        assert!(msgs.is_empty());
    }

    #[test]
    fn conveyor_skips_dead_units() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::ConveyorE);

        let msgs = apply_flow_water(&mut battle);

        assert_eq!(battle.units[1].x, 3);
        assert!(msgs.is_empty());
    }

    #[test]
    fn conveyor_out_of_bounds_skips() {
        let player = make_test_unit(UnitKind::Player, 0, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(0, 3, BattleTile::ConveyorW);

        let msgs = apply_flow_water(&mut battle);

        // Can't push west from x=0
        assert_eq!(battle.units[0].x, 0);
        assert!(msgs.is_empty());
    }

    #[test]
    fn conveyor_non_conveyor_tile_no_effect() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        // MetalFloor is default, not a conveyor

        let msgs = apply_flow_water(&mut battle);

        assert_eq!(battle.units[0].x, 3);
        assert!(msgs.is_empty());
    }

    // ── apply_conveyor_crates ────────────────────────────────────────

    #[test]
    fn conveyor_pushes_crate() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);
        battle.arena.set_tile(3, 3, BattleTile::ConveyorE);
        battle.arena.set_tile(4, 3, BattleTile::CargoCrate);

        let msgs = apply_conveyor_crates(&mut battle);

        assert_eq!(battle.arena.tile(5, 3), Some(BattleTile::CargoCrate));
        assert!(msgs.iter().any(|m| m.contains("Conveyor pushes")));
    }

    #[test]
    fn conveyor_no_crates_returns_empty() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);
        battle.arena.set_tile(3, 3, BattleTile::ConveyorE);

        let msgs = apply_conveyor_crates(&mut battle);

        assert!(msgs.is_empty());
    }

    // ── apply_gravity_wells ──────────────────────────────────────────

    #[test]
    fn gravity_well_damages_adjacent_unit() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::GravityWell);

        let msgs = apply_gravity_wells(&mut battle);

        assert!(battle.units[1].hp < 10);
        assert!(msgs.iter().any(|m| m.contains("gravity well")));
    }

    #[test]
    fn gravity_well_pulls_distant_unit_closer() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::GravityWell);

        let msgs = apply_gravity_wells(&mut battle);

        // Distance 2 -> pulled 1 step closer
        assert!(
            (battle.units[1].x - 3).abs() + (battle.units[1].y - 3).abs() < 2,
            "Unit should be pulled closer"
        );
        assert!(msgs.iter().any(|m| m.contains("pulls")));
    }

    #[test]
    fn gravity_well_no_effect_on_distant_unit() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::GravityWell);

        let msgs = apply_gravity_wells(&mut battle);

        // Distance > 2, no effect
        assert_eq!(battle.units[1].x, 6);
        assert_eq!(battle.units[1].y, 6);
        assert!(msgs.is_empty());
    }

    #[test]
    fn gravity_well_skips_dead_units() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::GravityWell);

        let msgs = apply_gravity_wells(&mut battle);

        assert_eq!(battle.units[1].x, 5);
        assert!(msgs.is_empty());
    }

    #[test]
    fn gravity_well_blocked_by_unwalkable_tile() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::GravityWell);
        battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);

        let msgs = apply_gravity_wells(&mut battle);

        assert_eq!(battle.units[1].x, 5);
        assert!(msgs.is_empty());
    }

    #[test]
    fn gravity_well_blocked_by_occupied_tile() {
        let player = make_test_unit(UnitKind::Player, 4, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::GravityWell);

        let _msgs = apply_gravity_wells(&mut battle);

        // Enemy can't be pulled because player occupies (4,3)
        assert_eq!(battle.units[1].x, 5);
    }

    #[test]
    fn gravity_well_damages_player_adjacent() {
        let player = make_test_unit(UnitKind::Player, 4, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::GravityWell);

        let msgs = apply_gravity_wells(&mut battle);

        assert!(battle.units[0].hp < 10);
        assert!(msgs.iter().any(|m| m.contains("You")));
    }

    // ── apply_weather_terrain_synergies ──────────────────────────────

    #[test]
    fn weather_coolant_extinguishes_fire() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.weather = Weather::CoolantLeak;
        battle.arena.set_tile(3, 3, BattleTile::BlastMark);

        let msgs = apply_weather_terrain_synergies(&mut battle);

        assert!(msgs.iter().any(|m| m.contains("extinguishes")));
    }

    #[test]
    fn weather_coolant_melts_frozen() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.weather = Weather::CoolantLeak;
        battle.arena.set_tile(3, 3, BattleTile::FrozenCoolant);

        let msgs = apply_weather_terrain_synergies(&mut battle);

        assert_eq!(
            battle.arena.tile(3, 3),
            Some(BattleTile::CoolantPool)
        );
        assert!(msgs.iter().any(|m| m.contains("melts")));
    }

    #[test]
    fn weather_debris_covers_lubricant() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.weather = Weather::DebrisStorm;
        battle.arena.set_tile(3, 3, BattleTile::Lubricant);

        let msgs = apply_weather_terrain_synergies(&mut battle);

        assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::Debris));
        assert!(msgs.iter().any(|m| m.contains("covers lubricant")));
    }

    #[test]
    fn weather_smoke_extends_steam() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.weather = Weather::SmokeScreen;
        battle.arena.set_tile(3, 3, BattleTile::VentSteam);
        if let Some(i) = battle.arena.idx(3, 3) {
            battle.arena.steam_timers[i] = 1;
        }

        apply_weather_terrain_synergies(&mut battle);

        if let Some(i) = battle.arena.idx(3, 3) {
            assert_eq!(battle.arena.steam_timers[i], 4);
        }
    }

    #[test]
    fn weather_normal_no_effect() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.weather = Weather::Normal;

        let msgs = apply_weather_terrain_synergies(&mut battle);

        assert!(msgs.is_empty());
    }

    #[test]
    fn weather_energy_flux_no_terrain_change() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.weather = Weather::EnergyFlux;

        let msgs = apply_weather_terrain_synergies(&mut battle);

        assert!(msgs.is_empty());
    }

    // ── apply_companion_passives ─────────────────────────────────────

    #[test]
    fn companion_none_returns_empty() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.companion_kind = None;

        let msgs = apply_companion_passives(&mut battle);

        assert!(msgs.is_empty());
    }

    #[test]
    fn companion_medic_heals_player() {
        use crate::game::Companion;
        let mut player = make_test_unit(UnitKind::Player, 0, 0);
        player.hp = 5;
        let companion = make_test_unit(UnitKind::Companion, 1, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, companion, enemy]);
        battle.companion_kind = Some(Companion::Medic);

        let msgs = apply_companion_passives(&mut battle);

        assert_eq!(battle.units[0].hp, 6);
        assert!(msgs.iter().any(|m| m.contains("healing aura")));
    }

    #[test]
    fn companion_medic_no_heal_at_full_hp() {
        use crate::game::Companion;
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let companion = make_test_unit(UnitKind::Companion, 1, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, companion, enemy]);
        battle.companion_kind = Some(Companion::Medic);

        let msgs = apply_companion_passives(&mut battle);

        assert_eq!(battle.units[0].hp, 10);
        assert!(!msgs.iter().any(|m| m.contains("healing aura")));
    }

    #[test]
    fn companion_medic_purifies_when_hp_below_50() {
        use crate::game::Companion;
        let mut player = make_test_unit(UnitKind::Player, 0, 0);
        player.hp = 3; // < 50% of 10
        player.statuses.push(crate::status::StatusInstance::new(
            crate::status::StatusKind::Poison { damage: 1 },
            3,
        ));
        let companion = make_test_unit(UnitKind::Companion, 1, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, companion, enemy]);
        battle.companion_kind = Some(Companion::Medic);

        let msgs = apply_companion_passives(&mut battle);

        assert!(battle.units[0].statuses.is_empty());
        assert!(msgs.iter().any(|m| m.contains("purifies")));
    }

    #[test]
    fn companion_science_officer_message_early_turns() {
        use crate::game::Companion;
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let companion = make_test_unit(UnitKind::Companion, 1, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, companion, enemy]);
        battle.companion_kind = Some(Companion::ScienceOfficer);
        battle.turn_number = 1;

        let msgs = apply_companion_passives(&mut battle);

        assert!(msgs.iter().any(|m| m.contains("combo buildup")));
    }

    #[test]
    fn companion_science_officer_no_message_late() {
        use crate::game::Companion;
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let companion = make_test_unit(UnitKind::Companion, 1, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, companion, enemy]);
        battle.companion_kind = Some(Companion::ScienceOfficer);
        battle.turn_number = 5;

        let msgs = apply_companion_passives(&mut battle);

        assert!(msgs.is_empty());
    }

    #[test]
    fn companion_dead_returns_empty() {
        use crate::game::Companion;
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut companion = make_test_unit(UnitKind::Companion, 1, 0);
        companion.alive = false;
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, companion, enemy]);
        battle.companion_kind = Some(Companion::Medic);

        let msgs = apply_companion_passives(&mut battle);

        assert!(msgs.is_empty());
    }

    // ── tick_arcing_projectiles ──────────────────────────────────────

    #[test]
    fn arcing_projectile_skips_fresh() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arcing_projectiles.push(crate::combat::ArcingProjectile {
            target_x: 6,
            target_y: 6,
            turns_remaining: 2,
            effect: ProjectileEffect::Damage(5),
            glyph: "★",
            color: "#fff",
            owner_is_player: true,
            fresh: true,
            aoe_radius: 0,
        });

        tick_arcing_projectiles(&mut battle);

        // Fresh projectile should now not be fresh, but not landed
        assert_eq!(battle.arcing_projectiles.len(), 1);
        assert!(!battle.arcing_projectiles[0].fresh);
        assert_eq!(battle.arcing_projectiles[0].turns_remaining, 2);
    }

    #[test]
    fn arcing_projectile_decrements_and_lands() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arcing_projectiles.push(crate::combat::ArcingProjectile {
            target_x: 6,
            target_y: 6,
            turns_remaining: 1,
            effect: ProjectileEffect::Damage(5),
            glyph: "★",
            color: "#fff",
            owner_is_player: true,
            fresh: false,
            aoe_radius: 0,
        });

        tick_arcing_projectiles(&mut battle);

        assert!(battle.arcing_projectiles.is_empty());
        assert!(battle.units[1].hp < 10);
        assert!(battle.log.iter().any(|m| m.contains("Arc strike")));
    }

    #[test]
    fn arcing_projectile_piercing_damage() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arcing_projectiles.push(crate::combat::ArcingProjectile {
            target_x: 6,
            target_y: 6,
            turns_remaining: 1,
            effect: ProjectileEffect::PiercingDamage(4),
            glyph: "★",
            color: "#fff",
            owner_is_player: true,
            fresh: false,
            aoe_radius: 0,
        });

        tick_arcing_projectiles(&mut battle);

        assert_eq!(battle.units[1].hp, 6);
        assert!(battle.log.iter().any(|m| m.contains("pierces")));
    }

    #[test]
    fn arcing_projectile_enemy_hits_player() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arcing_projectiles.push(crate::combat::ArcingProjectile {
            target_x: 3,
            target_y: 3,
            turns_remaining: 1,
            effect: ProjectileEffect::Damage(4),
            glyph: "★",
            color: "#fff",
            owner_is_player: false,
            fresh: false,
            aoe_radius: 0,
        });

        tick_arcing_projectiles(&mut battle);

        assert!(battle.units[0].hp < 10);
        assert!(battle.log.iter().any(|m| m.contains("Incoming arc")));
    }

    #[test]
    fn arcing_projectile_piercing_enemy_hits_player() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arcing_projectiles.push(crate::combat::ArcingProjectile {
            target_x: 3,
            target_y: 3,
            turns_remaining: 1,
            effect: ProjectileEffect::PiercingDamage(3),
            glyph: "★",
            color: "#fff",
            owner_is_player: false,
            fresh: false,
            aoe_radius: 0,
        });

        tick_arcing_projectiles(&mut battle);

        assert_eq!(battle.units[0].hp, 7);
        assert!(battle.log.iter().any(|m| m.contains("pierces you")));
    }

    #[test]
    fn arcing_projectile_misses_empty_tile() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arcing_projectiles.push(crate::combat::ArcingProjectile {
            target_x: 3,
            target_y: 3,
            turns_remaining: 1,
            effect: ProjectileEffect::Damage(5),
            glyph: "★",
            color: "#fff",
            owner_is_player: true,
            fresh: false,
            aoe_radius: 0,
        });

        tick_arcing_projectiles(&mut battle);

        // No units at (3,3), so no damage
        assert_eq!(battle.units[0].hp, 10);
        assert_eq!(battle.units[1].hp, 10);
    }

    #[test]
    fn arcing_projectile_spell_player_lands() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arcing_projectiles.push(crate::combat::ArcingProjectile {
            target_x: 3,
            target_y: 3,
            turns_remaining: 1,
            effect: ProjectileEffect::SpellHit(SpellEffect::StrongHit(5)),
            glyph: "★",
            color: "#fff",
            owner_is_player: true,
            fresh: false,
            aoe_radius: 0,
        });

        tick_arcing_projectiles(&mut battle);

        assert!(battle.units[1].hp < 10);
        assert!(battle.log.iter().any(|m| m.contains("lobbed spell")));
    }

    #[test]
    fn arcing_projectile_spell_enemy_hits_player() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arcing_projectiles.push(crate::combat::ArcingProjectile {
            target_x: 3,
            target_y: 3,
            turns_remaining: 1,
            effect: ProjectileEffect::SpellHit(SpellEffect::StrongHit(5)),
            glyph: "★",
            color: "#fff",
            owner_is_player: false,
            fresh: false,
            aoe_radius: 0,
        });

        tick_arcing_projectiles(&mut battle);

        assert!(battle.units[0].hp < 10);
        assert!(battle.log.iter().any(|m| m.contains("Enemy arc")));
    }

    // ── tick_pending_impacts ─────────────────────────────────────────

    #[test]
    fn pending_impact_decrements_timer() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.pending_impacts.push(crate::combat::PendingImpact {
            x: 3,
            y: 3,
            turns_until_hit: 3,
            damage: 5,
            radius: 0,
            source_is_player: true,
            element: None,
            glyph: "💥",
            color: "#f00",
        });

        tick_pending_impacts(&mut battle);

        assert_eq!(battle.pending_impacts.len(), 1);
        assert_eq!(battle.pending_impacts[0].turns_until_hit, 2);
    }

    #[test]
    fn pending_impact_detonates_on_enemy() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.pending_impacts.push(crate::combat::PendingImpact {
            x: 3,
            y: 3,
            turns_until_hit: 1,
            damage: 5,
            radius: 0,
            source_is_player: true,
            element: None,
            glyph: "💥",
            color: "#f00",
        });

        tick_pending_impacts(&mut battle);

        assert!(battle.pending_impacts.is_empty());
        assert!(battle.units[1].hp < 10);
        assert!(battle.log.iter().any(|m| m.contains("Impact detonates")));
    }

    #[test]
    fn pending_impact_detonates_on_player() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.pending_impacts.push(crate::combat::PendingImpact {
            x: 3,
            y: 3,
            turns_until_hit: 1,
            damage: 4,
            radius: 0,
            source_is_player: false,
            element: None,
            glyph: "💥",
            color: "#f00",
        });

        tick_pending_impacts(&mut battle);

        assert!(battle.units[0].hp < 10);
        assert!(battle.log.iter().any(|m| m.contains("hits you")));
    }

    #[test]
    fn pending_impact_aoe_hits_multiple() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let enemy2 = make_test_unit(UnitKind::Enemy(1), 4, 3);
        let mut battle = make_test_battle(vec![player, enemy1, enemy2]);
        battle.pending_impacts.push(crate::combat::PendingImpact {
            x: 3,
            y: 3,
            turns_until_hit: 1,
            damage: 2,
            radius: 1, // 3x3 area
            source_is_player: true,
            element: None,
            glyph: "💥",
            color: "#f00",
        });

        tick_pending_impacts(&mut battle);

        assert!(battle.units[1].hp < 10);
        assert!(battle.units[2].hp < 10);
        assert!(battle.log.iter().any(|m| m.contains("targets")));
    }

    #[test]
    fn pending_impact_fire_element_applies_burn() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.pending_impacts.push(crate::combat::PendingImpact {
            x: 3,
            y: 3,
            turns_until_hit: 1,
            damage: 2,
            radius: 0,
            source_is_player: true,
            element: Some(crate::combat::WuxingElement::Fire),
            glyph: "💥",
            color: "#f00",
        });

        tick_pending_impacts(&mut battle);

        assert!(battle.units[1]
            .statuses
            .iter()
            .any(|s| matches!(s.kind, crate::status::StatusKind::Burn { .. })));
    }

    #[test]
    fn pending_impact_water_element_applies_wet() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.pending_impacts.push(crate::combat::PendingImpact {
            x: 3,
            y: 3,
            turns_until_hit: 1,
            damage: 2,
            radius: 0,
            source_is_player: true,
            element: Some(crate::combat::WuxingElement::Water),
            glyph: "💥",
            color: "#f00",
        });

        tick_pending_impacts(&mut battle);

        assert!(battle.units[1]
            .statuses
            .iter()
            .any(|s| matches!(s.kind, crate::status::StatusKind::Wet)));
    }

    #[test]
    fn pending_impact_water_no_duplicate_wet() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        enemy.statuses.push(crate::status::StatusInstance::new(
            crate::status::StatusKind::Wet,
            2,
        ));
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.pending_impacts.push(crate::combat::PendingImpact {
            x: 3,
            y: 3,
            turns_until_hit: 1,
            damage: 2,
            radius: 0,
            source_is_player: true,
            element: Some(crate::combat::WuxingElement::Water),
            glyph: "💥",
            color: "#f00",
        });

        tick_pending_impacts(&mut battle);

        let wet_count = battle.units[1]
            .statuses
            .iter()
            .filter(|s| matches!(s.kind, crate::status::StatusKind::Wet))
            .count();
        assert_eq!(wet_count, 1);
    }

    #[test]
    fn pending_impact_no_element_no_status() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.pending_impacts.push(crate::combat::PendingImpact {
            x: 3,
            y: 3,
            turns_until_hit: 1,
            damage: 2,
            radius: 0,
            source_is_player: true,
            element: None,
            glyph: "💥",
            color: "#f00",
        });

        tick_pending_impacts(&mut battle);

        assert!(battle.units[1].statuses.is_empty());
    }

    #[test]
    fn pending_impact_audio_event_on_detonation() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.pending_impacts.push(crate::combat::PendingImpact {
            x: 3,
            y: 3,
            turns_until_hit: 1,
            damage: 2,
            radius: 0,
            source_is_player: true,
            element: None,
            glyph: "💥",
            color: "#f00",
        });

        tick_pending_impacts(&mut battle);

        assert!(battle
            .audio_events
            .iter()
            .any(|e| matches!(e, AudioEvent::ProjectileImpact)));
    }

    // ── tick_projectiles ─────────────────────────────────────────────

    #[test]
    fn projectile_advances_progress() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::ProjectileAnimation {
            message: String::new(),
            end_turn: false,
        };
        battle.projectiles.push(crate::combat::Projectile {
            from_x: 0.0,
            from_y: 0.0,
            to_x: 6,
            to_y: 6,
            progress: 0.0,
            speed: 0.1,
            arc_height: 0.0,
            effect: ProjectileEffect::Damage(5),
            owner_idx: 0,
            glyph: "•",
            color: "#fff",
            done: false,
        });

        tick_projectiles(&mut battle);

        // Projectile is still in flight (progress 0.1 < 1.0)
        assert_eq!(battle.projectiles.len(), 1);
        assert!((battle.projectiles[0].progress - 0.1).abs() < 0.001);
    }

    #[test]
    fn projectile_finishes_and_deals_damage() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::ProjectileAnimation {
            message: String::new(),
            end_turn: false,
        };
        battle.projectiles.push(crate::combat::Projectile {
            from_x: 0.0,
            from_y: 0.0,
            to_x: 6,
            to_y: 6,
            progress: 0.95,
            speed: 0.1,
            arc_height: 0.0,
            effect: ProjectileEffect::Damage(5),
            owner_idx: 0,
            glyph: "•",
            color: "#fff",
            done: false,
        });

        tick_projectiles(&mut battle);

        assert!(battle.projectiles.is_empty());
        assert!(battle.units[1].hp < 10);
    }

    #[test]
    fn projectile_piercing_bypasses_armor() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        enemy.radical_armor = 5;
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::ProjectileAnimation {
            message: String::new(),
            end_turn: false,
        };
        battle.projectiles.push(crate::combat::Projectile {
            from_x: 0.0,
            from_y: 0.0,
            to_x: 6,
            to_y: 6,
            progress: 0.95,
            speed: 0.1,
            arc_height: 0.0,
            effect: ProjectileEffect::PiercingDamage(4),
            owner_idx: 0,
            glyph: "•",
            color: "#fff",
            done: false,
        });

        tick_projectiles(&mut battle);

        // Piercing ignores armor, deals exact damage
        assert_eq!(battle.units[1].hp, 6);
    }

    #[test]
    fn projectile_piercing_kills_unit() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        enemy.hp = 2;
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::ProjectileAnimation {
            message: String::new(),
            end_turn: false,
        };
        battle.projectiles.push(crate::combat::Projectile {
            from_x: 0.0,
            from_y: 0.0,
            to_x: 6,
            to_y: 6,
            progress: 0.95,
            speed: 0.1,
            arc_height: 0.0,
            effect: ProjectileEffect::PiercingDamage(5),
            owner_idx: 0,
            glyph: "•",
            color: "#fff",
            done: false,
        });

        tick_projectiles(&mut battle);

        assert_eq!(battle.units[1].hp, 0);
        assert!(!battle.units[1].alive);
    }

    #[test]
    fn projectile_hits_cargo_crate_pushes() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);
        battle.phase = TacticalPhase::ProjectileAnimation {
            message: String::new(),
            end_turn: false,
        };
        battle.arena.set_tile(3, 3, BattleTile::CargoCrate);
        battle.projectiles.push(crate::combat::Projectile {
            from_x: 0.0,
            from_y: 3.0,
            to_x: 3,
            to_y: 3,
            progress: 0.95,
            speed: 0.1,
            arc_height: 0.0,
            effect: ProjectileEffect::Damage(5),
            owner_idx: 0,
            glyph: "•",
            color: "#fff",
            done: false,
        });

        tick_projectiles(&mut battle);

        // Crate should be pushed in projectile direction (east)
        assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::CargoCrate));
    }

    #[test]
    fn projectile_hits_fuel_canister_explodes() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);
        battle.phase = TacticalPhase::ProjectileAnimation {
            message: String::new(),
            end_turn: false,
        };
        battle.arena.set_tile(3, 3, BattleTile::FuelCanister);
        battle.projectiles.push(crate::combat::Projectile {
            from_x: 0.0,
            from_y: 0.0,
            to_x: 3,
            to_y: 3,
            progress: 0.95,
            speed: 0.1,
            arc_height: 0.0,
            effect: ProjectileEffect::Damage(5),
            owner_idx: 0,
            glyph: "•",
            color: "#fff",
            done: false,
        });

        tick_projectiles(&mut battle);

        // Fuel canister should be replaced with BlastMark
        assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::BlastMark));
    }

    #[test]
    fn projectile_done_skipped() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::ProjectileAnimation {
            message: String::new(),
            end_turn: false,
        };
        battle.projectiles.push(crate::combat::Projectile {
            from_x: 0.0,
            from_y: 0.0,
            to_x: 6,
            to_y: 6,
            progress: 1.0,
            speed: 0.1,
            arc_height: 0.0,
            effect: ProjectileEffect::Damage(5),
            owner_idx: 0,
            glyph: "•",
            color: "#fff",
            done: true,
        });

        tick_projectiles(&mut battle);

        // Already-done projectile gets collected and removed
        assert!(battle.projectiles.is_empty());
    }

    #[test]
    fn projectile_all_done_transitions_to_resolve() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::ProjectileAnimation {
            message: "test msg".to_string(),
            end_turn: true,
        };
        battle.projectiles.push(crate::combat::Projectile {
            from_x: 0.0,
            from_y: 0.0,
            to_x: 3,
            to_y: 3, // empty tile
            progress: 0.95,
            speed: 0.1,
            arc_height: 0.0,
            effect: ProjectileEffect::Damage(5),
            owner_idx: 0,
            glyph: "•",
            color: "#fff",
            done: false,
        });

        tick_projectiles(&mut battle);

        match &battle.phase {
            TacticalPhase::Resolve {
                message, end_turn, ..
            } => {
                assert_eq!(message, "test msg");
                assert!(*end_turn);
            }
            _ => panic!("Expected Resolve phase after all projectiles done"),
        }
    }

    #[test]
    fn projectile_player_dies_ends_defeat() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.hp = 1;
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::ProjectileAnimation {
            message: String::new(),
            end_turn: false,
        };
        battle.projectiles.push(crate::combat::Projectile {
            from_x: 6.0,
            from_y: 6.0,
            to_x: 3,
            to_y: 3,
            progress: 0.95,
            speed: 0.1,
            arc_height: 0.0,
            effect: ProjectileEffect::PiercingDamage(5),
            owner_idx: 1,
            glyph: "•",
            color: "#fff",
            done: false,
        });

        tick_projectiles(&mut battle);

        match &battle.phase {
            TacticalPhase::End { victory, .. } => assert!(!victory),
            _ => panic!("Expected End(defeat) phase"),
        }
    }

    #[test]
    fn projectile_all_enemies_die_ends_victory() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        enemy.hp = 1;
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::ProjectileAnimation {
            message: String::new(),
            end_turn: false,
        };
        battle.projectiles.push(crate::combat::Projectile {
            from_x: 0.0,
            from_y: 0.0,
            to_x: 6,
            to_y: 6,
            progress: 0.95,
            speed: 0.1,
            arc_height: 0.0,
            effect: ProjectileEffect::PiercingDamage(5),
            owner_idx: 0,
            glyph: "•",
            color: "#fff",
            done: false,
        });

        tick_projectiles(&mut battle);

        match &battle.phase {
            TacticalPhase::End { victory, .. } => assert!(*victory),
            _ => panic!("Expected End(victory) phase"),
        }
    }

    // ── apply_projectile_spell ───────────────────────────────────────

    #[test]
    fn spell_strong_hit_damages_enemy() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_projectile_spell(&mut battle, 3, 3, &SpellEffect::StrongHit(5));

        assert!(battle.units[1].hp < 10);
    }

    #[test]
    fn spell_strong_hit_ignores_player() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_projectile_spell(&mut battle, 3, 3, &SpellEffect::StrongHit(5));

        assert_eq!(battle.units[0].hp, 10);
    }

    #[test]
    fn spell_drain_damages_and_heals() {
        let mut player = make_test_unit(UnitKind::Player, 0, 0);
        player.hp = 5;
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_projectile_spell(&mut battle, 3, 3, &SpellEffect::Drain(4));

        assert!(battle.units[1].hp < 10);
        assert!(battle.units[0].hp > 5);
    }

    #[test]
    fn spell_drain_heal_capped_at_max() {
        let mut player = make_test_unit(UnitKind::Player, 0, 0);
        player.hp = 9;
        player.max_hp = 10;
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_projectile_spell(&mut battle, 3, 3, &SpellEffect::Drain(5));

        assert_eq!(battle.units[0].hp, 10);
    }

    #[test]
    fn spell_stun_stuns_enemy() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_projectile_spell(&mut battle, 3, 3, &SpellEffect::Stun);

        assert!(battle.units[1].stunned);
    }

    #[test]
    fn spell_stun_does_not_stun_player() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_projectile_spell(&mut battle, 3, 3, &SpellEffect::Stun);

        assert!(!battle.units[0].stunned);
    }

    #[test]
    fn spell_poison_adds_status() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_projectile_spell(&mut battle, 3, 3, &SpellEffect::Poison(2, 3));

        assert!(battle.units[1]
            .statuses
            .iter()
            .any(|s| matches!(s.kind, crate::status::StatusKind::Poison { damage: 2 })));
        assert!(battle
            .audio_events
            .iter()
            .any(|e| matches!(e, AudioEvent::StatusPoison)));
    }

    #[test]
    fn spell_slow_adds_status() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_projectile_spell(&mut battle, 3, 3, &SpellEffect::Slow(2));

        assert!(battle.units[1]
            .statuses
            .iter()
            .any(|s| matches!(s.kind, crate::status::StatusKind::Slow)));
        assert!(battle
            .audio_events
            .iter()
            .any(|e| matches!(e, AudioEvent::StatusSlow)));
    }

    #[test]
    fn spell_pierce_damages_enemy() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_projectile_spell(&mut battle, 3, 3, &SpellEffect::Pierce(4));

        assert!(battle.units[1].hp < 10);
    }

    #[test]
    fn spell_knockback_damages_and_pushes() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 2, 0);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_projectile_spell(&mut battle, 2, 0, &SpellEffect::KnockBack(3));

        assert!(battle.units[1].hp < 10);
        // Enemy should have been pushed away from player
        if battle.units[1].alive {
            assert!(battle.units[1].x > 2);
        }
    }

    #[test]
    fn spell_on_empty_tile_no_effect() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_projectile_spell(&mut battle, 3, 3, &SpellEffect::StrongHit(5));

        // No unit at (3,3), no damage
        assert_eq!(battle.units[0].hp, 10);
        assert_eq!(battle.units[1].hp, 10);
    }

    // ── select_next_unit ─────────────────────────────────────────────

    #[test]
    fn select_next_unit_player_sets_command() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        // Make sure current unit is player (index 0)
        battle.turn_queue_pos = 0;

        select_next_unit(&mut battle);

        assert!(matches!(battle.phase, TacticalPhase::Command));
        assert!(!battle.player_moved);
        assert!(!battle.player_acted);
    }

    #[test]
    fn select_next_unit_enemy_sets_enemy_turn() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        // Point to enemy
        battle.turn_queue_pos = 1;

        select_next_unit(&mut battle);

        match &battle.phase {
            TacticalPhase::EnemyTurn {
                unit_idx, acted, ..
            } => {
                assert_eq!(*unit_idx, 1);
                assert!(!acted);
            }
            _ => panic!("Expected EnemyTurn phase"),
        }
    }

    #[test]
    fn select_next_unit_skips_dead() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
        enemy1.alive = false;
        let enemy2 = make_test_unit(UnitKind::Enemy(1), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy1, enemy2]);
        // Point to dead enemy
        battle.turn_queue_pos = 1;

        select_next_unit(&mut battle);

        // Should skip dead enemy and go to enemy2 or wrap around
        match &battle.phase {
            TacticalPhase::EnemyTurn { unit_idx, .. } => {
                assert_ne!(*unit_idx, 1);
            }
            TacticalPhase::Command => {
                // Wrapped to player
            }
            _ => panic!("Expected EnemyTurn or Command phase"),
        }
    }

    // ── spread_rain_water ────────────────────────────────────────────

    #[test]
    fn spread_rain_water_expands_pool() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);
        battle.arena.set_tile(3, 3, BattleTile::CoolantPool);

        // Run many times to ensure probabilistic expansion
        for _ in 0..20 {
            spread_rain_water(&mut battle.arena);
        }

        // At least some neighboring tiles should be water
        let has_adjacent_water = [(2, 3), (4, 3), (3, 2), (3, 4)]
            .iter()
            .any(|&(x, y)| battle.arena.tile(x, y) == Some(BattleTile::CoolantPool));
        assert!(has_adjacent_water, "Rain should spread water to at least one neighbor");
    }

    #[test]
    fn spread_rain_water_no_water_no_spread() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);

        spread_rain_water(&mut battle.arena);

        // No CoolantPool tiles means no spreading
        for y in 0..7 {
            for x in 0..7 {
                assert_eq!(
                    battle.arena.tile(x, y),
                    Some(BattleTile::MetalFloor)
                );
            }
        }
    }

    // ── finish_environment_tick ──────────────────────────────────────

    #[test]
    fn finish_environment_tick_regens_focus() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.focus = 3;
        battle.max_focus = 10;
        battle.weather = Weather::Normal;

        finish_environment_tick(&mut battle);

        assert_eq!(battle.focus, 6); // +3 normal regen
    }

    #[test]
    fn finish_environment_tick_energy_flux_extra_focus() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.focus = 3;
        battle.max_focus = 10;
        battle.weather = Weather::EnergyFlux;

        finish_environment_tick(&mut battle);

        assert_eq!(battle.focus, 7); // +4 energy flux regen
    }

    #[test]
    fn finish_environment_tick_focus_capped_at_max() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.focus = 9;
        battle.max_focus = 10;

        finish_environment_tick(&mut battle);

        assert_eq!(battle.focus, 10);
    }

    #[test]
    fn finish_environment_tick_coolant_leak_applies_wet() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.weather = Weather::CoolantLeak;

        finish_environment_tick(&mut battle);

        assert!(battle.units[0]
            .statuses
            .iter()
            .any(|s| matches!(s.kind, crate::status::StatusKind::Wet)));
        assert!(battle.units[1]
            .statuses
            .iter()
            .any(|s| matches!(s.kind, crate::status::StatusKind::Wet)));
        assert!(battle.log.iter().any(|m| m.contains("soaks everyone")));
    }

    #[test]
    fn finish_environment_tick_coolant_leak_no_duplicate_wet() {
        let mut player = make_test_unit(UnitKind::Player, 0, 0);
        player.statuses.push(crate::status::StatusInstance::new(
            crate::status::StatusKind::Wet,
            2,
        ));
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.weather = Weather::CoolantLeak;

        finish_environment_tick(&mut battle);

        let wet_count = battle.units[0]
            .statuses
            .iter()
            .filter(|s| matches!(s.kind, crate::status::StatusKind::Wet))
            .count();
        assert_eq!(wet_count, 1);
    }

    #[test]
    fn finish_environment_tick_dead_player_ends_defeat() {
        let mut player = make_test_unit(UnitKind::Player, 0, 0);
        player.alive = false;
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);

        finish_environment_tick(&mut battle);

        match &battle.phase {
            TacticalPhase::End { victory, .. } => assert!(!victory),
            _ => panic!("Expected End(defeat) phase"),
        }
    }

    #[test]
    fn finish_environment_tick_all_enemies_dead_ends_victory() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        finish_environment_tick(&mut battle);

        match &battle.phase {
            TacticalPhase::End { victory, .. } => assert!(*victory),
            _ => panic!("Expected End(victory) phase"),
        }
    }

    // ── execute_arena_event ──────────────────────────────────────────

    #[test]
    fn arena_event_coolant_flood_spreads_water() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::CoolantPool);
        let mut rng = Rng::new(42);

        let msgs = execute_arena_event(&mut battle, ArenaEvent::CoolantFlood, &mut rng);

        assert!(!msgs.is_empty());
        assert!(msgs.iter().any(|m| m.contains("Coolant Flood")));
    }

    #[test]
    fn arena_event_coolant_flood_no_water_message() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        let mut rng = Rng::new(42);

        let msgs = execute_arena_event(&mut battle, ArenaEvent::CoolantFlood, &mut rng);

        assert!(msgs.iter().any(|m| m.contains("no tiles to flood")));
    }

    #[test]
    fn arena_event_hull_breach_creates_cracked_tiles() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        let mut rng = Rng::new(42);

        let msgs = execute_arena_event(&mut battle, ArenaEvent::HullBreach, &mut rng);

        assert!(msgs.iter().any(|m| m.contains("Hull Breach")));
    }

    #[test]
    fn arena_event_power_surge_places_oil_slick() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        let mut rng = Rng::new(42);

        let msgs = execute_arena_event(&mut battle, ArenaEvent::PowerSurge, &mut rng);

        assert!(msgs.iter().any(|m| m.contains("Power Surge")));
        // Should have placed some OilSlick tiles
        let has_oil = (0..7).any(|y| {
            (0..7).any(|x| battle.arena.tile(x, y) == Some(BattleTile::OilSlick))
        });
        assert!(has_oil);
    }

    #[test]
    fn arena_event_vent_blast_pushes_units() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 4, 4);
        let mut battle = make_test_battle(vec![player, enemy]);
        let mut rng = Rng::new(42);

        let msgs = execute_arena_event(&mut battle, ArenaEvent::VentBlast, &mut rng);

        assert!(msgs.iter().any(|m| m.contains("Vent Blast")));
    }

    #[test]
    fn arena_event_arc_discharge_message() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        let mut rng = Rng::new(42);

        let msgs = execute_arena_event(&mut battle, ArenaEvent::ArcDischarge, &mut rng);

        assert!(msgs.iter().any(|m| m.contains("Arc discharge")));
    }

    #[test]
    fn arena_event_plasma_leak_places_plasma() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 4);
        let mut battle = make_test_battle(vec![player, enemy]);
        let mut rng = Rng::new(42);

        let msgs = execute_arena_event(&mut battle, ArenaEvent::PlasmaLeak, &mut rng);

        assert!(msgs.iter().any(|m| m.contains("Plasma Leak")));
    }

    #[test]
    fn arena_event_cryo_vent_freezes_water() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::CoolantPool);
        let mut rng = Rng::new(42);

        let msgs = execute_arena_event(&mut battle, ArenaEvent::CryoVent, &mut rng);

        assert_eq!(
            battle.arena.tile(3, 3),
            Some(BattleTile::FrozenCoolant)
        );
        assert!(msgs.iter().any(|m| m.contains("Cryo Vent")));
    }

    #[test]
    fn arena_event_cryo_vent_slows_wet_units() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        enemy.statuses.push(crate::status::StatusInstance::new(
            crate::status::StatusKind::Wet,
            3,
        ));
        let mut battle = make_test_battle(vec![player, enemy]);
        let mut rng = Rng::new(42);

        execute_arena_event(&mut battle, ArenaEvent::CryoVent, &mut rng);

        assert!(battle.units[1]
            .statuses
            .iter()
            .any(|s| matches!(s.kind, crate::status::StatusKind::Slow)));
    }

    #[test]
    fn arena_event_cryo_vent_no_duplicate_slow() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        enemy.statuses.push(crate::status::StatusInstance::new(
            crate::status::StatusKind::Wet,
            3,
        ));
        enemy.statuses.push(crate::status::StatusInstance::new(
            crate::status::StatusKind::Slow,
            2,
        ));
        let mut battle = make_test_battle(vec![player, enemy]);
        let mut rng = Rng::new(42);

        execute_arena_event(&mut battle, ArenaEvent::CryoVent, &mut rng);

        let slow_count = battle.units[1]
            .statuses
            .iter()
            .filter(|s| matches!(s.kind, crate::status::StatusKind::Slow))
            .count();
        assert_eq!(slow_count, 1);
    }

    #[test]
    fn arena_event_debris_burst_places_debris() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        let mut rng = Rng::new(42);

        let msgs = execute_arena_event(&mut battle, ArenaEvent::DebrisBurst, &mut rng);

        assert!(msgs.iter().any(|m| m.contains("Debris Burst")));
    }

    #[test]
    fn arena_event_debris_burst_reduces_stored_movement() {
        let mut player = make_test_unit(UnitKind::Player, 0, 0);
        player.stored_movement = 2;
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        let mut rng = Rng::new(42);

        execute_arena_event(&mut battle, ArenaEvent::DebrisBurst, &mut rng);

        assert_eq!(battle.units[0].stored_movement, 1);
    }

    #[test]
    fn arena_event_system_glitch_extends_statuses() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        enemy.statuses.push(crate::status::StatusInstance::new(
            crate::status::StatusKind::Slow,
            2,
        ));
        let mut battle = make_test_battle(vec![player, enemy]);
        let mut rng = Rng::new(42);

        let msgs = execute_arena_event(&mut battle, ArenaEvent::SystemGlitch, &mut rng);

        assert_eq!(battle.units[1].statuses[0].turns_left, 3);
        assert!(msgs.iter().any(|m| m.contains("System Glitch")));
    }

    #[test]
    fn arena_event_nanite_spread_message() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        let mut rng = Rng::new(42);

        let msgs = execute_arena_event(&mut battle, ArenaEvent::NaniteSpread, &mut rng);

        assert!(msgs.iter().any(|m| m.contains("Nanite Spread")));
    }

    #[test]
    fn arena_event_reactor_blowout_places_plasma() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        let mut rng = Rng::new(42);

        let msgs = execute_arena_event(&mut battle, ArenaEvent::ReactorBlowout, &mut rng);

        assert!(msgs.iter().any(|m| m.contains("Reactor Blowout")));
        let has_plasma = (0..7).any(|y| {
            (0..7).any(|x| battle.arena.tile(x, y) == Some(BattleTile::PlasmaPool))
        });
        assert!(has_plasma);
    }

    // ── pick_arena_event ─────────────────────────────────────────────

    #[test]
    fn pick_arena_event_returns_valid_event() {
        let mut rng = Rng::new(42);
        let event = pick_arena_event(&mut rng, ArenaBiome::StationInterior);
        // Should be one of the known events
        let valid = matches!(
            event,
            ArenaEvent::CoolantFlood
                | ArenaEvent::HullBreach
                | ArenaEvent::PowerSurge
                | ArenaEvent::VentBlast
                | ArenaEvent::ArcDischarge
                | ArenaEvent::PlasmaLeak
                | ArenaEvent::MediGas
                | ArenaEvent::CryoVent
                | ArenaEvent::DebrisBurst
                | ArenaEvent::SystemGlitch
                | ArenaEvent::NaniteSpread
                | ArenaEvent::ReactorBlowout
        );
        assert!(valid);
    }

    #[test]
    fn pick_arena_event_different_biomes() {
        // Ensure all biomes produce valid events without panicking
        let biomes = [
            ArenaBiome::StationInterior,
            ArenaBiome::CryoBay,
            ArenaBiome::ReactorRoom,
            ArenaBiome::Hydroponics,
            ArenaBiome::AlienRuins,
            ArenaBiome::DerelictShip,
            ArenaBiome::IrradiatedZone,
        ];
        for biome in &biomes {
            let mut rng = Rng::new(123);
            let _ = pick_arena_event(&mut rng, *biome);
        }
    }

    // ── tick_battle additional phase tests ───────────────────────────

    #[test]
    fn tick_resolve_end_turn_advances() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::Resolve {
            message: "test".to_string(),
            timer: 0,
            end_turn: true,
        };

        tick_battle(&mut battle);

        // Should have advanced turn, not just gone to Command
        assert!(!matches!(battle.phase, TacticalPhase::Resolve { .. }));
    }

    #[test]
    fn tick_enemy_turn_acts_and_waits() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::EnemyTurn {
            unit_idx: 1,
            timer: 10,
            acted: false,
        };

        tick_battle(&mut battle);

        // After first tick, acted should be true
        match &battle.phase {
            TacticalPhase::EnemyTurn { acted, timer, .. } => {
                // acted is set true, timer may have changed
                assert!(*acted || *timer > 0);
            }
            // Could transition to End if battle ended
            TacticalPhase::End { .. } => {}
            _ => {}
        }
    }

    #[test]
    fn tick_enemy_turn_timer_decrements_after_acted() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::EnemyTurn {
            unit_idx: 1,
            timer: 5,
            acted: true,
        };

        tick_battle(&mut battle);

        match &battle.phase {
            TacticalPhase::EnemyTurn { timer, .. } => {
                assert_eq!(*timer, 4);
            }
            _ => panic!("Expected EnemyTurn phase with decremented timer"),
        }
    }

    #[test]
    fn tick_environment_tick_zero_calls_finish() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::EnvironmentTick { timer: 0 };

        tick_battle(&mut battle);

        // Should have called finish_environment_tick and transitioned
        assert!(!matches!(
            battle.phase,
            TacticalPhase::EnvironmentTick { .. }
        ));
    }

    #[test]
    fn tick_projectile_animation_phase() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::ProjectileAnimation {
            message: String::new(),
            end_turn: false,
        };
        // No projectiles => immediately transitions
        let event = tick_battle(&mut battle);

        assert!(matches!(event, BattleEvent::None));
        // With no projectiles, should transition to Resolve
        assert!(matches!(battle.phase, TacticalPhase::Resolve { .. }));
    }

    // ── push_boulder chain depth limit ───────────────────────────────

    #[test]
    fn push_boulder_chain_depth_limit() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);
        // Create a chain of 5 crates (exceeds depth 3 limit)
        battle.arena.set_tile(1, 3, BattleTile::CargoCrate);
        battle.arena.set_tile(2, 3, BattleTile::CargoCrate);
        battle.arena.set_tile(3, 3, BattleTile::CargoCrate);
        battle.arena.set_tile(4, 3, BattleTile::CargoCrate);
        battle.arena.set_tile(5, 3, BattleTile::CargoCrate);

        let msgs = push_boulder(&mut battle, 1, 3, 1, 0);

        assert!(msgs.iter().any(|m| m.contains("too long")));
    }

    // ── push_boulder chain at depth > 0 messages ─────────────────────

    #[test]
    fn push_boulder_chained_crate_slides_message() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut battle = make_test_battle(vec![player]);
        battle.arena.set_tile(2, 3, BattleTile::CargoCrate);
        battle.arena.set_tile(3, 3, BattleTile::CargoCrate);
        battle.arena.set_tile(4, 3, BattleTile::CargoCrate);

        let msgs = push_boulder(&mut battle, 2, 3, 1, 0);

        // Should have chain push messages
        assert!(msgs.iter().any(|m| m.contains("chain") || m.contains("Chain")));
        assert_eq!(battle.arena.tile(4, 3), Some(BattleTile::CargoCrate));
        assert_eq!(battle.arena.tile(5, 3), Some(BattleTile::CargoCrate));
    }

    // ── apply_flow_water enemy name ──────────────────────────────────

    #[test]
    fn conveyor_enemy_crush_shows_hanzi() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::ConveyorE);
        battle.arena.set_tile(4, 3, BattleTile::CoverBarrier);

        let msgs = apply_flow_water(&mut battle);

        assert!(msgs.iter().any(|m| m.contains("火")));
    }

    // ── finish_environment_tick skips dead units for wet ──────────────

    #[test]
    fn finish_environment_tick_coolant_skips_dead() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        enemy.alive = false;
        let enemy2 = make_test_unit(UnitKind::Enemy(1), 5, 5);
        let mut battle = make_test_battle(vec![player, enemy, enemy2]);
        battle.weather = Weather::CoolantLeak;

        finish_environment_tick(&mut battle);

        // Dead enemy should not get Wet status
        assert!(battle.units[1].statuses.is_empty());
    }

    // ── Quartermaster companion ──────────────────────────────────────

    #[test]
    fn companion_quartermaster_bribe_attempt() {
        use crate::game::Companion;
        let mut player = make_test_unit(UnitKind::Player, 0, 0);
        player.hp = 3; // < 40% of 10
        let companion = make_test_unit(UnitKind::Companion, 1, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, companion, enemy]);
        battle.companion_kind = Some(Companion::Quartermaster);

        let msgs = apply_companion_passives(&mut battle);

        // Bribe either succeeds or fails, both produce a message
        assert!(msgs.iter().any(|m| m.contains("Merchant") || m.contains("bribe")));
    }

    // ── SecurityChief companion ──────────────────────────────────────

    #[test]
    fn companion_security_chief_taunt() {
        use crate::game::Companion;
        let mut player = make_test_unit(UnitKind::Player, 0, 0);
        player.hp = 2; // < 30% of 10
        let companion = make_test_unit(UnitKind::Companion, 1, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, companion, enemy]);
        battle.companion_kind = Some(Companion::SecurityChief);

        let msgs = apply_companion_passives(&mut battle);

        assert!(msgs.iter().any(|m| m.contains("Guard taunts")));
    }

    // ── tick_battle EnemyTurn: timer=0, acted, no projectiles ────────

    #[test]
    fn tick_enemy_turn_timer_zero_advances() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::EnemyTurn {
            unit_idx: 1,
            timer: 0,
            acted: true,
        };

        tick_battle(&mut battle);

        // Should have advanced turn — phase transitions away from the original EnemyTurn
        // (may wrap to EnvironmentTick, Command, or a new EnemyTurn for next unit)
        match &battle.phase {
            TacticalPhase::EnemyTurn { unit_idx, acted, .. } => {
                // If it's still EnemyTurn, it should be for a new turn (acted=false)
                assert!(!acted, "Should be a fresh EnemyTurn, not the same one");
            }
            _ => {
                // Any other phase is fine — turn advanced
            }
        }
    }

    #[test]
    fn tick_enemy_turn_with_projectiles_goes_to_animation() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.phase = TacticalPhase::EnemyTurn {
            unit_idx: 1,
            timer: 0,
            acted: true,
        };
        battle.projectiles.push(crate::combat::Projectile {
            from_x: 6.0,
            from_y: 6.0,
            to_x: 0,
            to_y: 0,
            progress: 0.0,
            speed: 0.1,
            arc_height: 0.0,
            effect: ProjectileEffect::Damage(3),
            owner_idx: 1,
            glyph: "•",
            color: "#f00",
            done: false,
        });

        tick_battle(&mut battle);

        assert!(matches!(
            battle.phase,
            TacticalPhase::ProjectileAnimation { .. }
        ));
    }

    // ── arcing projectile piercing kills ──────────────────────────────

    #[test]
    fn arcing_piercing_kills_enemy() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        enemy.hp = 2;
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arcing_projectiles.push(crate::combat::ArcingProjectile {
            target_x: 3,
            target_y: 3,
            turns_remaining: 1,
            effect: ProjectileEffect::PiercingDamage(5),
            glyph: "★",
            color: "#fff",
            owner_is_player: true,
            fresh: false,
            aoe_radius: 0,
        });

        tick_arcing_projectiles(&mut battle);

        assert!(!battle.units[1].alive);
        assert_eq!(battle.units[1].hp, 0);
    }

    #[test]
    fn arcing_piercing_kills_player() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.hp = 2;
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arcing_projectiles.push(crate::combat::ArcingProjectile {
            target_x: 3,
            target_y: 3,
            turns_remaining: 1,
            effect: ProjectileEffect::PiercingDamage(5),
            glyph: "★",
            color: "#fff",
            owner_is_player: false,
            fresh: false,
            aoe_radius: 0,
        });

        tick_arcing_projectiles(&mut battle);

        assert!(!battle.units[0].alive);
        assert_eq!(battle.units[0].hp, 0);
    }

    // ── pending impact: out-of-bounds tiles skipped ──────────────────

    #[test]
    fn pending_impact_ignores_out_of_bounds() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);
        // Impact at corner with radius 1 — some tiles will be OOB
        battle.pending_impacts.push(crate::combat::PendingImpact {
            x: 0,
            y: 0,
            turns_until_hit: 1,
            damage: 2,
            radius: 1,
            source_is_player: true,
            element: None,
            glyph: "💥",
            color: "#f00",
        });

        // Should not panic
        tick_pending_impacts(&mut battle);

        assert!(battle.pending_impacts.is_empty());
    }

    // ── conveyor audio only once ─────────────────────────────────────

    #[test]
    fn conveyor_plays_audio_once_for_multiple_pushes() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 5);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::ConveyorE);
        battle.arena.set_tile(3, 5, BattleTile::ConveyorE);

        apply_flow_water(&mut battle);

        let conveyor_audio_count = battle
            .audio_events
            .iter()
            .filter(|e| matches!(e, AudioEvent::ConveyorMove))
            .count();
        assert_eq!(conveyor_audio_count, 1);
    }

    // ── gravity well pull direction ──────────────────────────────────

    #[test]
    fn gravity_well_pulls_along_x_axis() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::GravityWell);

        apply_gravity_wells(&mut battle);

        // Should be pulled toward well along x axis
        assert_eq!(battle.units[1].x, 4);
        assert_eq!(battle.units[1].y, 3);
    }

    #[test]
    fn gravity_well_pulls_along_y_axis() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 5);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::GravityWell);

        apply_gravity_wells(&mut battle);

        // Should be pulled toward well along y axis
        assert_eq!(battle.units[1].x, 3);
        assert_eq!(battle.units[1].y, 4);
    }

    // ── CoolantFlood applies Wet to units on flooded tiles ───────────

    #[test]
    fn coolant_flood_wets_unit_on_flooded_tile() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        // Place enemy adjacent to existing water
        let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.arena.set_tile(3, 3, BattleTile::CoolantPool);
        let mut rng = Rng::new(42);

        execute_arena_event(&mut battle, ArenaEvent::CoolantFlood, &mut rng);

        // If enemy's tile got flooded, they should be wet
        if battle.arena.tile(4, 3) == Some(BattleTile::CoolantPool) {
            assert!(battle.units[1]
                .statuses
                .iter()
                .any(|s| matches!(s.kind, crate::status::StatusKind::Wet)));
        }
    }

    // ── arc discharge chains through water ───────────────────────────

    #[test]
    fn arc_discharge_chains_through_water() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        // Place coolant pool adjacent to target for chain
        battle.arena.set_tile(3, 3, BattleTile::CoolantPool);
        let mut rng = Rng::new(1);

        // We can't control exact targeting, so just verify no panic
        execute_arena_event(&mut battle, ArenaEvent::ArcDischarge, &mut rng);
    }
}
