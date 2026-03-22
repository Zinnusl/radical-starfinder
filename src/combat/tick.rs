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

        let flow_msgs = apply_flow_water(battle);
        for msg in &flow_msgs {
            battle.log_message(msg);
        }

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
    }

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
    let threshold = if battle.is_boss_battle { 15 } else { 10 };
    let warning_turn = if battle.is_boss_battle { 13 } else { 8 };
    let turn = battle.turn_number;

    if turn == warning_turn {
        battle.log_message("The reactor grows unstable...");
    } else if turn >= threshold {
        let escalation = (turn - threshold) / 2;
        let dmg = (1 + escalation as i32).min(5);
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

fn apply_flow_water(battle: &mut TacticalBattle) -> Vec<String> {
    let mut messages = Vec::new();
    let mut pushes: Vec<(usize, i32, i32)> = Vec::new();

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
        }
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
    let mut messages = Vec::new();
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

    if let Some(unit_idx) = battle.unit_at(dest_x, dest_y) {
        let actual = deal_damage(battle, unit_idx, 3);
        let name = if battle.units[unit_idx].is_player() {
            "You are".to_string()
        } else {
            format!("{} is", battle.units[unit_idx].hanzi)
        };
        messages.push(format!("{} crushed by a cargo crate! ({} dmg)", name, actual));
        battle.arena.tiles[src_idx] = BattleTile::MetalFloor;
        battle.arena.tiles[dest_idx] = BattleTile::CargoCrate;
    } else if battle.arena.tiles[dest_idx].is_walkable() {
        battle.arena.tiles[src_idx] = BattleTile::MetalFloor;
        battle.arena.tiles[dest_idx] = BattleTile::CargoCrate;
        messages.push("The cargo crate slides!".to_string());
    } else {
        messages.push("The cargo crate thuds against the wall.".to_string());
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
                        if battle.arena.tiles[i] == BattleTile::VentSteam {
                            if battle.arena.steam_timers[i] < 4 {
                                battle.arena.steam_timers[i] = 4;
                            }
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
                        .map_or(false, |t| t.is_walkable())
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
                    .map_or(false, |t| t.is_walkable())
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


