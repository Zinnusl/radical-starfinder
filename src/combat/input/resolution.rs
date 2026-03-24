//! Action resolution for tactical combat (attacks, spells, combos).

use crate::combat::action::{deal_damage, deal_damage_from, flank_bonus};
use crate::combat::boss;
use crate::combat::grid::manhattan;
use crate::combat::terrain::{apply_knockback, apply_terrain_interactions, TerrainSource};
use crate::combat::{
    AudioEvent, BattleTile, PendingImpact, Projectile, ProjectileEffect, TacticalBattle,
    TacticalPhase, TypingAction, Weather, WuxingElement,
};
use crate::enemy::BossKind;
use crate::radical::SpellEffect;
use crate::status::{StatusInstance, StatusKind};
use crate::vocab;

use super::BattleEvent;

#[allow(dead_code)]
fn tile_spell_bonus(battle: &TacticalBattle, unit_idx: usize) -> i32 {
    let tile_bonus = match battle
        .arena
        .tile(battle.units[unit_idx].x, battle.units[unit_idx].y)
    {
        // InkPool → +2 spell power with EnergyFlux weather, +1 otherwise
        Some(BattleTile::OilSlick) => {
            if battle.weather == Weather::EnergyFlux { 2 } else { 1 }
        }
        Some(BattleTile::HoloTrap) => 2,
        _ => 0,
    };
    let weather_bonus = match battle.weather {
        Weather::EnergyFlux => 1,
        _ => 0,
    };
    tile_bonus + weather_bonus
}

fn spell_effect_school(effect: &SpellEffect) -> &'static str {
    match effect {
        SpellEffect::FireAoe(_) => "fire",
        SpellEffect::Stun => "lightning",
        SpellEffect::Drain(_) => "drain",
        SpellEffect::StrongHit(_) => "force",
        SpellEffect::Heal(_) => "heal",
        SpellEffect::Shield => "shield",
        SpellEffect::Reveal => "reveal",
        SpellEffect::Pacify => "pacify",
        SpellEffect::Slow(_) => "ice",
        SpellEffect::Teleport => "wind",
        SpellEffect::Poison(_, _) => "poison",
        SpellEffect::FocusRestore(_) => "focus",
        SpellEffect::ArmorBreak => "force",
        SpellEffect::Dash(_) => "wind",
        SpellEffect::Pierce(_) => "force",
        SpellEffect::PullToward => "wind",
        SpellEffect::KnockBack(_) => "force",
        SpellEffect::Thorns(_) => "poison",
        SpellEffect::Cone(_) => "fire",
        SpellEffect::Wall(_) => "shield",
        SpellEffect::OilSlick => "poison",
        SpellEffect::FreezeGround(_) => "ice",
        SpellEffect::Ignite => "fire",
        SpellEffect::PlantGrowth => "heal",
        SpellEffect::Earthquake(_) => "force",
        SpellEffect::Sanctify(_) => "heal",
        SpellEffect::FloodWave(_) => "ice",
        SpellEffect::SummonBoulder => "shield",
        SpellEffect::Charge(_) => "force",
        SpellEffect::Blink(_) => "wind",
    }
}

/// Map a SpellEffect to a Wuxing element for the combo chain system.
pub fn spell_effect_element(effect: &SpellEffect) -> Option<WuxingElement> {
    match effect {
        // Fire
        SpellEffect::FireAoe(_) | SpellEffect::Cone(_) | SpellEffect::Ignite => {
            Some(WuxingElement::Fire)
        }
        // Water/Ice
        SpellEffect::Slow(_) | SpellEffect::FreezeGround(_) | SpellEffect::FloodWave(_) => {
            Some(WuxingElement::Water)
        }
        // Metal/Force
        SpellEffect::StrongHit(_)
        | SpellEffect::ArmorBreak
        | SpellEffect::Pierce(_)
        | SpellEffect::KnockBack(_)
        | SpellEffect::Charge(_)
        | SpellEffect::Stun => Some(WuxingElement::Metal),
        // Wood/Nature
        SpellEffect::Poison(_, _)
        | SpellEffect::Thorns(_)
        | SpellEffect::PlantGrowth
        | SpellEffect::OilSlick
        | SpellEffect::Heal(_)
        | SpellEffect::Drain(_) => Some(WuxingElement::Wood),
        // Earth
        SpellEffect::Earthquake(_)
        | SpellEffect::Wall(_)
        | SpellEffect::SummonBoulder
        | SpellEffect::Shield => Some(WuxingElement::Earth),
        // No element
        _ => None,
    }
}

/// Name of the combo triggered by casting `prev` then `current` elements in sequence.
pub fn spell_combo_name(
    prev: WuxingElement,
    current: WuxingElement,
) -> Option<&'static str> {
    match (prev, current) {
        (WuxingElement::Water, WuxingElement::Fire) => Some("Steam Burst"),
        (WuxingElement::Water, WuxingElement::Earth) => Some("Avalanche"),
        (WuxingElement::Fire, WuxingElement::Fire) => Some("Inferno"),
        (WuxingElement::Wood, WuxingElement::Fire) => Some("Toxic Cloud"),
        (WuxingElement::Fire, WuxingElement::Metal) => Some("Tempering"),
        (WuxingElement::Metal, WuxingElement::Water) => Some("Lightning Storm"),
        (WuxingElement::Earth, WuxingElement::Earth) => Some("Petrify"),
        (WuxingElement::Wood, WuxingElement::Water) => Some("Overgrowth"),
        (WuxingElement::Metal, WuxingElement::Earth) => Some("Shatter"),
        (WuxingElement::Wood, WuxingElement::Earth) => Some("Entangle"),
        (WuxingElement::Fire, WuxingElement::Wood) => Some("Purifying Flame"),
        (WuxingElement::Water, WuxingElement::Metal) => Some("Frozen Edge"),
        _ => None,
    }
}

/// Apply a spell combo effect. Called when the player casts two spells of the right
/// elements within 2 turns. Returns a log message describing the combo.
fn apply_spell_combo(
    battle: &mut TacticalBattle,
    combo_name: &str,
    target_x: i32,
    target_y: i32,
) -> String {
    let px = battle.units[0].x;
    let py = battle.units[0].y;

    match combo_name {
        "Steam Burst" => {
            // AoE Steam tiles in 2-radius + 2 dmg to all in area + Confused 1 turn
            let mut hits = 0;
            for dx in -2..=2_i32 {
                for dy in -2..=2_i32 {
                    if dx.abs() + dy.abs() > 2 {
                        continue;
                    }
                    let tx = target_x + dx;
                    let ty = target_y + dy;
                    if battle.arena.in_bounds(tx, ty) {
                        if let Some(tile) = battle.arena.tile(tx, ty) {
                            if tile.is_walkable() {
                                battle.arena.set_tile(tx, ty, BattleTile::VentSteam);
                                battle.arena.set_steam(tx, ty, 3);
                            }
                        }
                        if let Some(idx) = battle.unit_at(tx, ty) {
                            if battle.units[idx].is_enemy() {
                                deal_damage(battle, idx, 2);
                                battle.units[idx]
                                    .statuses
                                    .push(StatusInstance::new(StatusKind::Confused, 1));
                                hits += 1;
                            }
                        }
                    }
                }
            }
            format!(
                "Steam erupts! {} enemies take 2 dmg and are Confused!",
                hits
            )
        }
        "Avalanche" => {
            // 4 dmg in cone + Slow 3 turns + BrokenGround tiles
            let cone = super::targeting::compute_aoe_preview(
                &SpellEffect::Cone(4),
                target_x,
                target_y,
                px,
                py,
            );
            let mut hits = 0;
            for &(tx, ty) in &cone {
                if battle.arena.in_bounds(tx, ty) {
                    battle.arena.set_tile(tx, ty, BattleTile::DamagedPlating);
                    if let Some(idx) = battle.unit_at(tx, ty) {
                        if battle.units[idx].is_enemy() {
                            deal_damage(battle, idx, 4);
                            battle.units[idx]
                                .statuses
                                .push(StatusInstance::new(StatusKind::Slow, 3));
                            hits += 1;
                        }
                    }
                }
            }
            format!("Avalanche! {} enemies take 4 dmg and are Slowed!", hits)
        }
        "Inferno" => {
            // Double the second Fire spell's damage + Burn 2 for 3 turns
            // We apply 4 bonus fire damage to target + burn
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    deal_damage(battle, idx, 4);
                    battle.units[idx]
                        .statuses
                        .push(StatusInstance::new(StatusKind::Burn { damage: 2 }, 3));
                    return "Inferno! Double fire damage + Burn!".to_string();
                }
            }
            // AoE fallback — burn everything in cross
            let cross = [
                (target_x, target_y),
                (target_x - 1, target_y),
                (target_x + 1, target_y),
                (target_x, target_y - 1),
                (target_x, target_y + 1),
            ];
            let mut hits = 0;
            for &(tx, ty) in &cross {
                if let Some(idx) = battle.unit_at(tx, ty) {
                    if battle.units[idx].is_enemy() {
                        deal_damage(battle, idx, 4);
                        battle.units[idx]
                            .statuses
                            .push(StatusInstance::new(StatusKind::Burn { damage: 2 }, 3));
                        hits += 1;
                    }
                }
            }
            format!("Inferno! {} enemies scorched with Burn!", hits)
        }
        "Toxic Cloud" => {
            // Poison gas: 3×3, 2 dmg + Poison(1) 3 turns
            let mut hits = 0;
            for dx in -1..=1_i32 {
                for dy in -1..=1_i32 {
                    let tx = target_x + dx;
                    let ty = target_y + dy;
                    if battle.arena.in_bounds(tx, ty) {
                        if let Some(idx) = battle.unit_at(tx, ty) {
                            if battle.units[idx].is_enemy() {
                                deal_damage(battle, idx, 2);
                                battle.units[idx].statuses.push(StatusInstance::new(
                                    StatusKind::Poison { damage: 1 },
                                    3,
                                ));
                                hits += 1;
                            }
                        }
                    }
                }
            }
            format!("Toxic Cloud! {} enemies take 2 dmg + Poison!", hits)
        }
        "Tempering" => {
            // Player gains +2 armor for 3 turns + Fortify 2
            battle.combo_armor_bonus = 2;
            battle.combo_armor_turns = 3;
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Fortify { stacks: 2 }, 3));
            "Tempering! +2 armor for 3 turns + Fortify!".to_string()
        }
        "Lightning Storm" => {
            // Chain lightning: 3 dmg to target + 2 dmg to all Wet enemies
            let mut hits = 0;
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    deal_damage(battle, idx, 3);
                    hits += 1;
                }
            }
            let wet_targets: Vec<usize> = (1..battle.units.len())
                .filter(|&i| {
                    battle.units[i].alive
                        && battle.units[i].is_enemy()
                        && battle.units[i]
                            .statuses
                            .iter()
                            .any(|s| matches!(s.kind, StatusKind::Wet))
                })
                .collect();
            for idx in wet_targets {
                deal_damage(battle, idx, 2);
                hits += 1;
            }
            format!("Lightning Storm! {} enemies struck!", hits)
        }
        "Petrify" => {
            // Target turned to stone: skip 2 turns + 4 armor
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    battle.units[idx].stunned = true;
                    battle.units[idx]
                        .statuses
                        .push(StatusInstance::new(StatusKind::Freeze, 2));
                    battle.units[idx].radical_armor += 4;
                    return "Petrify! Enemy turned to stone!".to_string();
                }
            }
            "Petrify! The ground trembles...".to_string()
        }
        "Overgrowth" => {
            // Create Grass+BambooThicket in 3×3, heal player 3 HP
            for dx in -1..=1_i32 {
                for dy in -1..=1_i32 {
                    let tx = target_x + dx;
                    let ty = target_y + dy;
                    if battle.arena.in_bounds(tx, ty) {
                        if let Some(tile) = battle.arena.tile(tx, ty) {
                            if tile.is_walkable() && battle.unit_at(tx, ty).is_none() {
                                if dx == 0 && dy == 0 {
                                    battle
                                        .arena
                                        .set_tile(tx, ty, BattleTile::PipeTangle);
                                } else {
                                    battle.arena.set_tile(tx, ty, BattleTile::WiringPanel);
                                }
                            }
                        }
                    }
                }
            }
            let player = &mut battle.units[0];
            player.hp = (player.hp + 3).min(player.max_hp);
            "Overgrowth! Lush growth spreads, heal 3 HP!".to_string()
        }
        "Shatter" => {
            // ArmorBreak + 3 dmg + BrokenGround under target
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    battle.units[idx].radical_armor = 0;
                    deal_damage(battle, idx, 3);
                    battle.arena.set_tile(target_x, target_y, BattleTile::DamagedPlating);
                    return "Shatter! Armor broken + 3 dmg!".to_string();
                }
            }
            battle.arena.set_tile(target_x, target_y, BattleTile::DamagedPlating);
            "Shatter! The ground cracks!".to_string()
        }
        "Entangle" => {
            // Rooted 3 turns + Thorns tiles around target
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    battle.units[idx]
                        .statuses
                        .push(StatusInstance::new(StatusKind::Rooted, 3));
                }
            }
            let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];
            for (dx, dy) in &deltas {
                let tx = target_x + dx;
                let ty = target_y + dy;
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        if tile.is_walkable() {
                            battle.arena.set_tile(tx, ty, BattleTile::ElectrifiedWire);
                        }
                    }
                }
            }
            "Entangle! Enemy Rooted + Thorns spread!".to_string()
        }
        "Purifying Flame" => {
            // Remove all negative statuses from player + 2 dmg AoE
            battle.units[0]
                .statuses
                .retain(|s| !s.is_negative());
            let cross = [
                (target_x, target_y),
                (target_x - 1, target_y),
                (target_x + 1, target_y),
                (target_x, target_y - 1),
                (target_x, target_y + 1),
            ];
            let mut hits = 0;
            for &(tx, ty) in &cross {
                if let Some(idx) = battle.unit_at(tx, ty) {
                    if battle.units[idx].is_enemy() {
                        deal_damage(battle, idx, 2);
                        hits += 1;
                    }
                }
            }
            format!(
                "Purifying Flame! Cleansed + {} enemies take 2 dmg!",
                hits
            )
        }
        "Frozen Edge" => {
            // Next 3 basic attacks apply Slow 1 + deal +1 damage
            battle.frozen_edge_charges = 3;
            "Frozen Edge! Next 3 attacks deal +1 dmg + Slow!".to_string()
        }
        _ => "Unknown combo!".to_string(),
    }
}

fn wuxing_element_name(elem: &WuxingElement) -> &'static str {
    match elem {
        WuxingElement::Water => "Water",
        WuxingElement::Fire => "Fire",
        WuxingElement::Metal => "Metal",
        WuxingElement::Wood => "Wood",
        WuxingElement::Earth => "Earth",
    }
}

pub(super) fn resolve_basic_attack(
    battle: &mut TacticalBattle,
    target_idx: usize,
    input: &str,
) -> BattleEvent {
    if target_idx >= battle.units.len() || !battle.units[target_idx].alive {
        battle.log_message("Target is gone.");
        return super::try_end_player_turn(battle);
    }

    if !battle.units[target_idx].is_enemy() {
        battle.log_message("Invalid target.");
        return BattleEvent::None;
    }

    let correct = super::typing::check_attack_pinyin(battle, target_idx, input);

    let correct = if correct && battle.weather == Weather::DebrisStorm {
        let roll = (battle.turn_number as u64 * 7 + target_idx as u64 * 13) % 100;
        if roll < 10 {
            battle.log_message("Sandstorm obscures your aim — miss!");
            false
        } else {
            true
        }
    } else {
        correct
    };

    let target_hanzi = battle.units[target_idx].hanzi;

    if correct {
        if battle.units[target_idx].radical_dodge {
            battle.units[target_idx].radical_dodge = false;
            battle.last_answer = Some((target_hanzi, true));
            battle.audio_events.push(AudioEvent::TypingCorrect);
            battle.log_message(format!(
                "{} dodges the attack!",
                battle.units[target_idx].hanzi
            ));
            battle.player_acted = true;
            battle.typing_action = None;
            battle.phase = TacticalPhase::Resolve {
                message: "Dodged!".to_string(),
                timer: 20,
                end_turn: true,
            };
            return BattleEvent::None;
        }

        battle.last_answer = Some((target_hanzi, true));
        battle.combo_streak += 1;
        battle.audio_events.push(AudioEvent::TypingCorrect);
        let combo = battle.combo_multiplier();
        let flank = flank_bonus(battle, 0, target_idx);
        let base_damage = (battle.units[0].damage + battle.player_stance.damage_mod()).max(1);

        let focus_cost = target_hanzi.chars().count().max(1) as i32;
        let focus_penalty = if battle.focus < focus_cost { 0.65 } else { 1.0 };
        battle.focus = (battle.focus - focus_cost).max(0);

        let synergy_bonus = if battle.radical_synergy_streak >= 2 {
            (1.0 + 0.25 * (battle.radical_synergy_streak - 1) as f64).min(1.5)
        } else {
            1.0
        };

        // CriticalStrike + Backstab synergy: guaranteed crit from behind
        let crit_multiplier = if crate::combat::action::critical_backstab_check(battle, target_idx) {
            battle.log_message("⚔💀 Critical Backstab! Guaranteed critical hit!");
            battle.audio_events.push(AudioEvent::CriticalHit);
            2.0
        } else {
            // Normal crit check from CriticalStrike equipment
            let crit_chance: i32 = battle.player_equip_effects.iter().filter_map(|e| {
                if let crate::player::EquipEffect::CriticalStrike(pct) = e { Some(*pct) } else { None }
            }).sum();
            if crit_chance > 0 {
                let roll = (battle.turn_number as u64 * 11 + target_idx as u64 * 7 + battle.combo_streak as u64) % 100;
                if (roll as i32) < crit_chance {
                    battle.log_message("⚔ Critical strike!");
                    battle.audio_events.push(AudioEvent::CriticalHit);
                    2.0
                } else {
                    1.0
                }
            } else {
                1.0
            }
        };

        let raw = (base_damage as f64 * combo * (1.0 + flank) * focus_penalty * synergy_bonus * crit_multiplier)
            .ceil() as i32;
        // Frozen Edge combo bonus: +1 damage + Slow on next 3 basic attacks
        let frozen_bonus = if battle.frozen_edge_charges > 0 {
            battle.frozen_edge_charges -= 1;
            1
        } else {
            0
        };
        let (actual, wuxing_label) = deal_damage_from(battle, 0, target_idx, raw + frozen_bonus);
        if frozen_bonus > 0 {
            battle.units[target_idx]
                .statuses
                .push(crate::status::StatusInstance::new(crate::status::StatusKind::Slow, 1));
            battle.log_message("❄ Frozen Edge! +1 dmg + Slow!");
        }

        let tier = battle.combo_tier_name();
        let flank_label = if flank >= 0.50 {
            battle.audio_events.push(AudioEvent::CriticalHit);
            " Backstab!"
        } else if flank >= 0.25 {
            battle.audio_events.push(AudioEvent::CriticalHit);
            " Flanked!"
        } else {
            ""
        };
        let msg = if tier.is_empty() {
            format!("Hit for {} damage!{}", actual, flank_label)
        } else {
            format!("{} combo! Hit for {} damage!{}", tier, actual, flank_label)
        };
        battle.log_message(&msg);
        if let Some(wl) = wuxing_label {
            battle.log_message(wl);
        }

        if let Some(ability_idx) = battle.selected_radical_ability.take() {
            if ability_idx < battle.player_radical_abilities.len() {
                let (radical_str, ability) = battle.player_radical_abilities[ability_idx];
                let ability_msg = crate::combat::radical::apply_player_radical_ability(
                    battle, 0, target_idx, ability,
                );
                battle.log_message(&ability_msg);
                battle.consumed_radicals.push(radical_str);
                battle.player_radical_abilities.remove(ability_idx);
            }
        }

        if crate::status::has_envenomed(&battle.units[0].statuses) {
            battle.units[target_idx]
                .statuses
                .push(StatusInstance::new(StatusKind::Poison { damage: 1 }, 3));
            battle.log_message("Poison coats the wound!");
            battle.audio_events.push(AudioEvent::StatusPoison);
            // Check status combos after applying poison
            let combo_msgs = crate::combat::action::check_status_combos(battle, target_idx);
            for m in &combo_msgs {
                battle.log_message(m);
            }
        }

        // LifeSteal + Poison synergy: drain extra 1 HP from poisoned enemies
        if !battle.units[target_idx].alive {
            let has_lifesteal = battle.player_equip_effects.iter().any(|e| {
                matches!(e, crate::player::EquipEffect::LifeSteal(_))
            });
            let was_poisoned = battle.units[target_idx]
                .statuses
                .iter()
                .any(|s| matches!(s.kind, StatusKind::Poison { .. }));
            if has_lifesteal && was_poisoned {
                battle.units[0].hp = (battle.units[0].hp + 1).min(battle.units[0].max_hp);
                battle.log_message("🧛 LifeSteal drains extra from poisoned foe! (+1 HP)");
            }
        }

        if battle.units[target_idx].alive && (flank >= 0.50 || battle.combo_streak >= 5) {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let kb_msgs = apply_knockback(battle, target_idx, px, py);
            for m in &kb_msgs {
                battle.log_message(m);
            }
        }

        if battle.units[target_idx].charge_remaining.is_some() {
            battle.units[target_idx].charge_remaining = None;
            battle.log_message("Charge interrupted!");
        }

        let killed_element = battle.units[target_idx].wuxing_element;
        if !battle.units[target_idx].alive {
            if let Some(elem) = killed_element {
                let elem_name = wuxing_element_name(&elem);
                if battle.radical_synergy_radical == Some(elem_name) {
                    battle.radical_synergy_streak += 1;
                } else {
                    battle.radical_synergy_radical = Some(elem_name);
                    battle.radical_synergy_streak = 1;
                }
                if battle.radical_synergy_streak >= 3 {
                    let splash_dmg = if battle.radical_synergy_streak >= 4 {
                        4
                    } else {
                        2
                    };
                    let tx = battle.units[target_idx].x;
                    let ty = battle.units[target_idx].y;
                    let splash_targets: Vec<usize> = (1..battle.units.len())
                        .filter(|&i| {
                            battle.units[i].alive
                                && battle.units[i].is_enemy()
                                && manhattan(battle.units[i].x, battle.units[i].y, tx, ty) <= 1
                        })
                        .collect();
                    for si in splash_targets {
                        deal_damage(battle, si, splash_dmg);
                    }
                    battle.log_message(format!(
                        "{} synergy x{}! AoE splash for {} damage!",
                        elem_name, battle.radical_synergy_streak, splash_dmg
                    ));
                }
            } else {
                battle.radical_synergy_radical = None;
                battle.radical_synergy_streak = 0;
            }

            battle.chengyu_history.push(target_hanzi.to_string());
            if battle.chengyu_history.len() > 4 {
                battle.chengyu_history.remove(0);
            }
            if let Some(effect_msg) = super::typing::check_chengyu_combo(battle) {
                battle.log_message(&effect_msg);
            }
        }

        if !battle.units[target_idx].alive {
            battle.log_message("Enemy defeated!");
            if battle.all_enemies_dead() {
                battle.phase = TacticalPhase::End {
                    victory: true,
                    timer: 60,
                };
                return BattleEvent::None;
            }
        }

        battle.player_acted = true;
        battle.phase = TacticalPhase::Resolve {
            message: msg,
            timer: 30,
            end_turn: true,
        };
        BattleEvent::None
    } else {
        let partial = super::typing::check_partial_pinyin(battle, target_idx, input);
        if partial {
            battle.last_answer = Some((target_hanzi, false));
            battle.combo_streak = 0;
            battle.selected_radical_ability = None;
            battle.audio_events.push(AudioEvent::TypingError);
            let base_damage = (battle.units[0].damage + battle.player_stance.damage_mod()).max(1);
            let half_dmg = (base_damage / 2).max(1);
            let actual = deal_damage(battle, target_idx, half_dmg);
            let msg = format!("Close! Wrong tone. {} half-damage.", actual);
            battle.log_message(&msg);

            if !battle.units[target_idx].alive {
                battle.log_message("Enemy defeated!");
                if battle.all_enemies_dead() {
                    battle.phase = TacticalPhase::End {
                        victory: true,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
            }

            battle.player_acted = true;
            battle.phase = TacticalPhase::Resolve {
                message: msg,
                timer: 24,
                end_turn: true,
            };
            BattleEvent::None
        } else {
            battle.last_answer = Some((target_hanzi, false));
            battle.combo_streak = 0;
            battle.selected_radical_ability = None;
            battle.audio_events.push(AudioEvent::TypingError);
            let miss_msg = format!("Wrong! '{}' is incorrect.", input);
            battle.log_message(&miss_msg);

            for i in 1..battle.units.len() {
                if battle.units[i].alive
                    && battle.units[i].boss_kind == Some(BossKind::DriftLeviathan)
                {
                    if let Some(steal_msg) = boss::steal_spell(battle, i) {
                        battle.log_message(steal_msg);
                    }
                    break;
                }
            }

            if battle.units[target_idx].alive {
                let enemy_dmg = battle.units[target_idx].damage;
                let actual = deal_damage(battle, 0, enemy_dmg);
                battle.log_message(format!("Counter-attack! {} damage!", actual));

                if battle.units[0].hp <= 0 {
                    battle.units[0].alive = false;
                    battle.phase = TacticalPhase::End {
                        victory: false,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
            }
            battle.phase = TacticalPhase::Resolve {
                message: miss_msg,
                timer: 24,
                end_turn: false,
            };
            BattleEvent::None
        }
    }
}

pub(super) fn resolve_spell_cast(
    battle: &mut TacticalBattle,
    spell_idx: usize,
    target_x: i32,
    target_y: i32,
    effect: SpellEffect,
    input: &str,
) -> BattleEvent {
    if spell_idx >= battle.available_spells.len() {
        battle.log_message("Spell no longer available.");
        return BattleEvent::None;
    }

    let spell_hanzi = battle.available_spells[spell_idx].0;

    let correct = if let Some(entry) = vocab::vocab_entry_by_hanzi(spell_hanzi) {
        vocab::check_pinyin(entry, input)
    } else {
        let stored_pinyin = battle.available_spells[spell_idx].1;
        stored_pinyin.eq_ignore_ascii_case(&input.replace(' ', ""))
    };

    if !correct {
        battle.last_answer = Some((spell_hanzi, false));
        battle.combo_streak = 0;
        battle.audio_events.push(AudioEvent::TypingError);
        let miss_msg = format!("Wrong! '{}' — spell fizzles.", input);
        battle.log_message(&miss_msg);
        battle.phase = TacticalPhase::Resolve {
            message: miss_msg,
            timer: 24,
            end_turn: false,
        };
        return BattleEvent::None;
    }

    battle.last_answer = Some((spell_hanzi, true));
    battle.combo_streak += 1;
    battle.audio_events.push(AudioEvent::TypingCorrect);
    battle
        .audio_events
        .push(AudioEvent::SpellElement(spell_effect_school(&effect).to_string()));

    let spell_power = battle.player_stance.spell_power_mod();

    let msg = match effect {
        SpellEffect::FireAoe(dmg) => {
            let rain_penalty = if battle.weather == Weather::CoolantLeak {
                1
            } else {
                0
            };
            let dmg = (dmg + spell_power - rain_penalty).max(1);
            let mut cross = vec![
                (target_x, target_y),
                (target_x - 1, target_y),
                (target_x + 1, target_y),
                (target_x, target_y - 1),
                (target_x, target_y + 1),
            ];
            if crate::combat::action::spell_power_extra_tiles(battle) {
                cross.push((target_x - 1, target_y - 1));
                cross.push((target_x + 1, target_y - 1));
                cross.push((target_x - 1, target_y + 1));
                cross.push((target_x + 1, target_y + 1));
                battle.log_message("📖 SpellPower expands the terrain effect!");
            }
            // Terrain interactions happen immediately (visual feedback)
            let terrain_msgs = apply_terrain_interactions(
                battle,
                TerrainSource::FireAbility,
                &cross,
            );
            for tm in &terrain_msgs {
                battle.log_message(tm);
            }
            // Damage is telegraphed: detonates next round
            for &(cx, cy) in &cross {
                if battle.arena.in_bounds(cx, cy) {
                    battle.pending_impacts.push(PendingImpact {
                        x: cx,
                        y: cy,
                        turns_until_hit: 1,
                        damage: dmg,
                        radius: 0,
                        source_is_player: true,
                        element: Some(WuxingElement::Fire),
                        glyph: "🔥",
                        color: "#ff4422",
                    });
                }
            }
            format!(
                "Fire erupts across {} tiles! Impact in 1 turn!",
                cross.len()
            )
        }
        SpellEffect::Heal(amt) => {
            let unit = &mut battle.units[0];
            let healed = amt.min(unit.max_hp - unit.hp);
            unit.hp = (unit.hp + amt).min(unit.max_hp);
            battle.audio_events.push(AudioEvent::Heal);
            format!("Healed for {} HP!", healed)
        }
        SpellEffect::Reveal => {
            let mut revealed = 0;
            for i in 0..battle.arena.tiles.len() {
                if battle.arena.tiles[i] == BattleTile::MineTile {
                    battle.arena.tiles[i] = BattleTile::MineTileRevealed;
                    revealed += 1;
                }
            }
            if revealed > 0 {
                format!(
                    "The battlefield pulses with insight! {} hidden traps revealed!",
                    revealed
                )
            } else {
                "The battlefield pulses with insight!".to_string()
            }
        }
        SpellEffect::Shield => {
            battle.units[0].defending = true;
            battle.audio_events.push(AudioEvent::ShieldBlock);
            "A barrier forms around you!".to_string()
        }
        SpellEffect::StrongHit(_dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: px as f64,
                from_y: py as f64,
                to_x: target_x,
                to_y: target_y,
                progress: 0.0,
                speed: Projectile::SPEED_NORMAL,
                arc_height: 0.3,
                effect: ProjectileEffect::SpellHit(effect),
                owner_idx: 0,
                glyph: "⚔",
                color: "#ffcc33",
                done: false,
            });
            battle.audio_events.push(AudioEvent::ProjectileLaunch);
            "Powerful strike launched!".to_string()
        }
        SpellEffect::Drain(_dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: px as f64,
                from_y: py as f64,
                to_x: target_x,
                to_y: target_y,
                progress: 0.0,
                speed: Projectile::SPEED_SLOW,
                arc_height: 0.5,
                effect: ProjectileEffect::SpellHit(effect),
                owner_idx: 0,
                glyph: "🩸",
                color: "#aa44ff",
                done: false,
            });
            battle.audio_events.push(AudioEvent::ProjectileLaunch);
            "Draining force launched!".to_string()
        }
        SpellEffect::Stun => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: px as f64,
                from_y: py as f64,
                to_x: target_x,
                to_y: target_y,
                progress: 0.0,
                speed: Projectile::SPEED_FAST,
                arc_height: 0.2,
                effect: ProjectileEffect::SpellHit(effect),
                owner_idx: 0,
                glyph: "⚡",
                color: "#44ddff",
                done: false,
            });
            battle.audio_events.push(AudioEvent::ProjectileLaunch);
            "Lightning bolt launched!".to_string()
        }
        SpellEffect::Pacify => {
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    battle.units[idx].hp = 0;
                    battle.units[idx].alive = false;
                    format!("{} is pacified!", battle.units[idx].hanzi)
                } else {
                    "No target there.".to_string()
                }
            } else {
                "Peace finds no one.".to_string()
            }
        }
        SpellEffect::Slow(_turns) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: px as f64,
                from_y: py as f64,
                to_x: target_x,
                to_y: target_y,
                progress: 0.0,
                speed: Projectile::SPEED_SLOW,
                arc_height: 0.4,
                effect: ProjectileEffect::SpellHit(effect),
                owner_idx: 0,
                glyph: "❄",
                color: "#88ccff",
                done: false,
            });
            battle.audio_events.push(AudioEvent::ProjectileLaunch);
            "Freezing bolt launched!".to_string()
        }
        SpellEffect::Teleport => {
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    let (px, py) = (battle.units[0].x, battle.units[0].y);
                    let (ex, ey) = (battle.units[idx].x, battle.units[idx].y);
                    battle.units[0].x = ex;
                    battle.units[0].y = ey;
                    battle.units[idx].x = px;
                    battle.units[idx].y = py;
                    format!("Swapped positions with {}!", battle.units[idx].hanzi)
                } else {
                    "No target there.".to_string()
                }
            } else {
                "The spell finds no anchor.".to_string()
            }
        }
        SpellEffect::Poison(_dmg, _turns) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: px as f64,
                from_y: py as f64,
                to_x: target_x,
                to_y: target_y,
                progress: 0.0,
                speed: Projectile::SPEED_SLOW,
                arc_height: 0.6,
                effect: ProjectileEffect::SpellHit(effect),
                owner_idx: 0,
                glyph: "☠",
                color: "#44ff44",
                done: false,
            });
            battle.audio_events.push(AudioEvent::ProjectileLaunch);
            "Poison bolt launched!".to_string()
        }
        SpellEffect::FocusRestore(amt) => {
            battle.focus = (battle.focus + amt).min(battle.max_focus);
            format!("Focus restored by {}!", amt)
        }
        SpellEffect::ArmorBreak => {
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    let stripped = battle.units[idx].radical_armor;
                    battle.units[idx].radical_armor = 0;
                    format!(
                        "{}'s armor shattered! ({} armor removed)",
                        battle.units[idx].hanzi, stripped
                    )
                } else {
                    "No target there.".to_string()
                }
            } else {
                "The force hits nothing.".to_string()
            }
        }
        SpellEffect::Dash(dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let path = super::targeting::line_between(px, py, target_x, target_y);
            let mut hits = 0;
            for &(tx, ty) in &path {
                if let Some(idx) = battle.unit_at(tx, ty) {
                    if battle.units[idx].is_enemy() && battle.units[idx].alive {
                        deal_damage(battle, idx, dmg);
                        hits += 1;
                    }
                }
            }
            battle.units[0].x = target_x;
            battle.units[0].y = target_y;
            battle.player_moved = true;
            if hits > 0 {
                format!("Dashed through {} enemies for {} damage each!", hits, dmg)
            } else {
                "Dashed to new position!".to_string()
            }
        }
        SpellEffect::Pierce(dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let dx = (target_x - px).signum();
            let dy = (target_y - py).signum();
            let mut hits = 0;
            let (mut x, mut y) = (px, py);
            for _ in 0..6 {
                x += dx;
                y += dy;
                if x < 0
                    || y < 0
                    || x >= battle.arena.width as i32
                    || y >= battle.arena.height as i32
                {
                    break;
                }
                if let Some(BattleTile::CoverBarrier) = battle.arena.tile(x, y) {
                    break;
                }
                if let Some(idx) = battle.unit_at(x, y) {
                    if battle.units[idx].is_enemy() && battle.units[idx].alive {
                        deal_damage(battle, idx, dmg);
                        hits += 1;
                    }
                }
            }
            if hits > 0 {
                format!("Piercing bolt hits {} enemies for {} each!", hits, dmg)
            } else {
                "The bolt pierces through empty air.".to_string()
            }
        }
        SpellEffect::PullToward => {
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    let px = battle.units[0].x;
                    let py = battle.units[0].y;
                    let ex = battle.units[idx].x;
                    let ey = battle.units[idx].y;
                    let dx = (px - ex).signum();
                    let dy = (py - ey).signum();
                    let mut dest_x = ex;
                    let mut dest_y = ey;
                    for _ in 0..3 {
                        let nx = dest_x + dx;
                        let ny = dest_y + dy;
                        if !battle.arena.in_bounds(nx, ny) {
                            break;
                        }
                        if let Some(t) = battle.arena.tile(nx, ny) {
                            if !t.is_walkable() {
                                break;
                            }
                        }
                        if battle.unit_at(nx, ny).is_some() && !(nx == px && ny == py) {
                            break;
                        }
                        if nx == px && ny == py {
                            break;
                        }
                        dest_x = nx;
                        dest_y = ny;
                    }
                    let pulled = (ex - dest_x).abs() + (ey - dest_y).abs();
                    battle.units[idx].x = dest_x;
                    battle.units[idx].y = dest_y;
                    format!(
                        "Pulled {} {} tiles closer!",
                        battle.units[idx].hanzi, pulled
                    )
                } else {
                    "No target there.".to_string()
                }
            } else {
                "The pull finds no anchor.".to_string()
            }
        }
        SpellEffect::KnockBack(dmg) => {
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    let px = battle.units[0].x;
                    let py = battle.units[0].y;
                    deal_damage(battle, idx, dmg);
                    let kb1 = apply_knockback(battle, idx, px, py);
                    for m in &kb1 {
                        battle.log_message(m);
                    }
                    if battle.units[idx].alive {
                        let kb2 = apply_knockback(battle, idx, px, py);
                        for m in &kb2 {
                            battle.log_message(m);
                        }
                    }
                    format!(
                        "Knocked {} back with {} damage!",
                        battle.units[idx].hanzi, dmg
                    )
                } else {
                    "No target there.".to_string()
                }
            } else {
                "The force hits nothing.".to_string()
            }
        }
        SpellEffect::Thorns(turns) => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Thorns, turns));
            format!("Thorns aura active for {} turns!", turns)
        }
        SpellEffect::Cone(dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = super::targeting::compute_aoe_preview(&effect, target_x, target_y, px, py);
            // Cone blast is telegraphed: detonates next round
            for &(cx, cy) in &preview {
                if battle.arena.in_bounds(cx, cy) {
                    battle.pending_impacts.push(PendingImpact {
                        x: cx,
                        y: cy,
                        turns_until_hit: 1,
                        damage: dmg,
                        radius: 0,
                        source_is_player: true,
                        element: Some(WuxingElement::Metal),
                        glyph: "⚡",
                        color: "#cccccc",
                    });
                }
            }
            format!("Arc blast charging across {} tiles! Impact in 1 turn!", preview.len())
        }
        SpellEffect::Wall(len) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let dx = (target_x - px).signum();
            let dy = (target_y - py).signum();
            let mut placed = 0;
            let half = len / 2;
            for i in -half..=half {
                let (wx, wy) = if dx != 0 && dy == 0 {
                    (target_x, target_y + i)
                } else if dy != 0 && dx == 0 {
                    (target_x + i, target_y)
                } else {
                    (target_x + i, target_y)
                };
                if battle.arena.in_bounds(wx, wy)
                    && battle.unit_at(wx, wy).is_none()
                    && battle.arena.tile(wx, wy) != Some(BattleTile::CoverBarrier)
                {
                    battle.arena.set_tile(wx, wy, BattleTile::CoverBarrier);
                    placed += 1;
                }
            }
            format!("Raised a wall of {} stone pillars!", placed)
        }
        SpellEffect::OilSlick => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = super::targeting::compute_aoe_preview(&effect, target_x, target_y, px, py);
            let mut placed = 0;
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        if tile.is_walkable() && tile != BattleTile::Lubricant {
                            battle.arena.set_tile(tx, ty, BattleTile::Lubricant);
                            placed += 1;
                        }
                    }
                }
            }
            format!("Oil slick covers {} tiles!", placed)
        }
        SpellEffect::FreezeGround(dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = super::targeting::compute_aoe_preview(&effect, target_x, target_y, px, py);
            let mut frozen = 0;
            // Freeze terrain immediately (visual feedback)
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        if matches!(tile, BattleTile::CoolantPool | BattleTile::MetalFloor) {
                            battle.arena.set_tile(tx, ty, BattleTile::FrozenCoolant);
                            frozen += 1;
                        }
                    }
                }
            }
            // Damage + Slow are telegraphed: crystallization detonates next round
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    battle.pending_impacts.push(PendingImpact {
                        x: tx,
                        y: ty,
                        turns_until_hit: 1,
                        damage: dmg,
                        radius: 0,
                        source_is_player: true,
                        element: Some(WuxingElement::Water),
                        glyph: "❄",
                        color: "#88ccff",
                    });
                }
            }
            format!("Ground freezes! {} tiles frozen! Cryo blast in 1 turn!", frozen)
        }
        SpellEffect::Ignite => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = super::targeting::compute_aoe_preview(&effect, target_x, target_y, px, py);
            let mut burned = 0;
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        match tile {
                            BattleTile::WiringPanel | BattleTile::ElectrifiedWire => {
                                battle.arena.set_tile(tx, ty, BattleTile::BlastMark);
                                burned += 1;
                            }
                            BattleTile::Lubricant => {
                                battle.arena.set_tile(tx, ty, BattleTile::BlastMark);
                                burned += 1;
                                // Oil explosion: 3 damage to unit on this tile
                                if let Some(idx) = battle.unit_at(tx, ty) {
                                    deal_damage(battle, idx, 3);
                                    battle.log_message(&format!(
                                        "Lubricant ignites! {} takes 3 damage!",
                                        battle.units[idx].hanzi
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some(idx) = battle.unit_at(tx, ty) {
                        battle.units[idx]
                            .statuses
                            .push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 3));
                    }
                }
            }
            format!("Plasma ignites {} tiles! Burn applied!", burned)
        }
        SpellEffect::PlantGrowth => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = super::targeting::compute_aoe_preview(&effect, target_x, target_y, px, py);
            let mut grown = 0;
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        if tile == BattleTile::WiringPanel {
                            if battle.unit_at(tx, ty).is_none() {
                                battle.arena.set_tile(tx, ty, BattleTile::PipeTangle);
                                grown += 1;
                            }
                        } else if tile == BattleTile::MetalFloor || tile == BattleTile::BlastMark {
                            battle.arena.set_tile(tx, ty, BattleTile::WiringPanel);
                            grown += 1;
                        }
                    }
                }
            }
            // Heal player 1 if standing on Grass
            let player_tile = battle.arena.tile(battle.units[0].x, battle.units[0].y);
            if player_tile == Some(BattleTile::WiringPanel) {
                let unit = &mut battle.units[0];
                let healed = 1_i32.min(unit.max_hp - unit.hp);
                unit.hp = (unit.hp + 1).min(unit.max_hp);
                if healed > 0 {
                    battle.log_message("Standing on wiring panel restores 1 HP!");
                }
            }
            format!("Nanites spread! {} tiles transformed!", grown)
        }
        SpellEffect::Earthquake(dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = super::targeting::compute_aoe_preview(&effect, target_x, target_y, px, py);
            // Terrain cracking happens immediately (visual warning)
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        match tile {
                            BattleTile::MetalFloor | BattleTile::WiringPanel | BattleTile::Debris => {
                                battle.arena.set_tile(tx, ty, BattleTile::WeakenedPlating);
                            }
                            BattleTile::WeakenedPlating => {
                                if battle.unit_at(tx, ty).is_none() {
                                    battle.arena.set_tile(tx, ty, BattleTile::BreachedFloor);
                                } else {
                                    battle.arena.set_tile(tx, ty, BattleTile::DamagedFloor);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            // Push cargo crates outward from center
            for &(tx, ty) in &preview {
                if battle.arena.tile(tx, ty) == Some(BattleTile::CargoCrate) {
                    let bdx = (tx - target_x).signum();
                    let bdy = (ty - target_y).signum();
                    if bdx == 0 && bdy == 0 { continue; }
                    let nx = tx + bdx;
                    let ny = ty + bdy;
                    if battle.arena.in_bounds(nx, ny)
                        && battle.arena.tile(nx, ny).map(|t| t.is_walkable()).unwrap_or(false)
                        && battle.unit_at(nx, ny).is_none()
                    {
                        battle.arena.set_tile(tx, ty, BattleTile::MetalFloor);
                        battle.arena.set_tile(nx, ny, BattleTile::CargoCrate);
                    }
                }
            }
            let terrain_msgs = apply_terrain_interactions(
                battle,
                TerrainSource::Earthquake,
                &preview,
            );
            for tm in &terrain_msgs {
                battle.log_message(tm);
            }
            // Seismic damage is telegraphed: detonates in 2 turns
            battle.pending_impacts.push(PendingImpact {
                x: target_x,
                y: target_y,
                turns_until_hit: 2,
                damage: dmg,
                radius: 2,
                source_is_player: true,
                element: Some(WuxingElement::Earth),
                glyph: "💥",
                color: "#cc9944",
            });
            format!("The deck shakes! Seismic charge detonates in 2 turns!")
        }
        SpellEffect::Sanctify(heal) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = super::targeting::compute_aoe_preview(&effect, target_x, target_y, px, py);
            let mut sanctified = 0;
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        if tile.is_walkable() {
                            battle.arena.set_holy(tx, ty, 3);
                            // Store heal amount in steam_timers (reuse for shield zone heal amount)
                            if let Some(i) = battle.arena.idx(tx, ty) {
                                battle.arena.steam_timers[i] = heal as u8;
                            }
                            sanctified += 1;
                        }
                    }
                }
            }
            format!("Shield field covers {} tiles! Heals {} HP/turn for 3 rounds.", sanctified, heal)
        }
        SpellEffect::FloodWave(dmg) => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let preview = super::targeting::compute_aoe_preview(&effect, target_x, target_y, px, py);
            let dx = (target_x - px).signum();
            let dy = (target_y - py).signum();
            // Push units immediately (wave front)
            let mut push_targets = Vec::new();
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(idx) = battle.unit_at(tx, ty) {
                        if battle.units[idx].is_enemy() {
                            push_targets.push(idx);
                        }
                    }
                }
            }
            for idx in push_targets {
                if battle.units[idx].alive {
                    for _ in 0..2 {
                        let nx = battle.units[idx].x + dx;
                        let ny = battle.units[idx].y + dy;
                        if battle.arena.in_bounds(nx, ny)
                            && battle.arena.tile(nx, ny).map(|t| t.is_walkable()).unwrap_or(false)
                            && battle.unit_at(nx, ny).is_none()
                        {
                            battle.units[idx].x = nx;
                            battle.units[idx].y = ny;
                        } else {
                            break;
                        }
                    }
                }
            }
            // Place coolant tiles immediately (visible hazard)
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    if let Some(tile) = battle.arena.tile(tx, ty) {
                        if tile.is_walkable() && tile != BattleTile::CoolantPool {
                            battle.arena.set_tile(tx, ty, BattleTile::CoolantPool);
                        }
                    }
                }
            }
            // Damage is telegraphed: wave crashes next round
            for &(tx, ty) in &preview {
                if battle.arena.in_bounds(tx, ty) {
                    battle.pending_impacts.push(PendingImpact {
                        x: tx,
                        y: ty,
                        turns_until_hit: 1,
                        damage: dmg,
                        radius: 0,
                        source_is_player: true,
                        element: Some(WuxingElement::Water),
                        glyph: "🌊",
                        color: "#4488ff",
                    });
                }
            }
            format!("Coolant wave surges! Impact in 1 turn!")
        }
        SpellEffect::SummonBoulder => {
            if battle.arena.in_bounds(target_x, target_y) {
                let tile = battle.arena.tile(target_x, target_y);
                if tile.map(|t| t.is_walkable()).unwrap_or(false)
                    && battle.unit_at(target_x, target_y).is_none()
                {
                    battle.arena.set_tile(target_x, target_y, BattleTile::CargoCrate);
                    "A cargo crate materializes!".to_string()
                } else {
                    "Cannot place crate there!".to_string()
                }
            } else {
                "Target out of bounds.".to_string()
            }
        }
        SpellEffect::Charge(base_dmg) => {
            // Move toward target, stop adjacent. Damage = base + 50% per tile traveled.
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let _dist = (target_x - px).abs() + (target_y - py).abs();
            // Find the closest empty walkable tile adjacent to the target
            let dx = (target_x - px).signum();
            let dy = (target_y - py).signum();
            let mut dest_x = px;
            let mut dest_y = py;
            let mut tiles_moved = 0;
            // Move along the line toward target, stop one tile before target (or at target if empty)
            let path = super::targeting::line_between(px, py, target_x, target_y);
            for &(tx, ty) in &path {
                if tx == target_x && ty == target_y {
                    break; // Don't move onto the target's tile
                }
                if !battle.arena.in_bounds(tx, ty) {
                    break;
                }
                let walkable = battle
                    .arena
                    .tile(tx, ty)
                    .map(|t| t.is_walkable())
                    .unwrap_or(false);
                if !walkable {
                    break;
                }
                if battle.unit_at(tx, ty).is_some() {
                    break;
                }
                dest_x = tx;
                dest_y = ty;
                tiles_moved += 1;
            }
            // Move the player
            if dest_x != px || dest_y != py {
                battle.units[0].x = dest_x;
                battle.units[0].y = dest_y;
                battle.player_moved = true;
                if let Some(dir) = crate::combat::Direction::from_delta(dx, dy) {
                    battle.units[0].facing = dir;
                }
            }
            // Deal damage to the target: base + 50% per tile traveled
            if let Some(idx) = battle.unit_at(target_x, target_y) {
                if battle.units[idx].is_enemy() {
                    let scaled_dmg =
                        base_dmg + (tiles_moved as f64 * 0.5).ceil() as i32 + spell_power;
                    deal_damage(battle, idx, scaled_dmg);
                    format!(
                        "Charged {} tiles into {}! {} damage!",
                        tiles_moved, battle.units[idx].hanzi, scaled_dmg
                    )
                } else {
                    format!("Charged {} tiles forward!", tiles_moved)
                }
            } else {
                format!("Charged {} tiles forward!", tiles_moved)
            }
        }
        SpellEffect::Blink(dmg) => {
            // Teleport to empty target tile, AoE damage at departure point
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let dmg = dmg + spell_power;
            // AoE explosion at departure point (cross pattern)
            let aoe_tiles = vec![
                (px, py),
                (px - 1, py),
                (px + 1, py),
                (px, py - 1),
                (px, py + 1),
            ];
            let mut hits = 0;
            for &(ax, ay) in &aoe_tiles {
                if let Some(idx) = battle.unit_at(ax, ay) {
                    if battle.units[idx].is_enemy() && battle.units[idx].alive {
                        deal_damage(battle, idx, dmg);
                        hits += 1;
                    }
                }
            }
            // Teleport player
            battle.units[0].x = target_x;
            battle.units[0].y = target_y;
            battle.player_moved = true;
            if hits > 0 {
                format!(
                    "Blinked away! Departure explosion hits {} enemies for {} damage!",
                    hits, dmg
                )
            } else {
                "Blinked to new position!".to_string()
            }
        }
    };

    battle.log_message(&msg);

    // ── Ability combo chain check ──────────────────────────────────────────
    let current_element = spell_effect_element(&effect);
    if let Some(cur_elem) = current_element {
        if let Some(prev_elem) = battle.last_spell_element {
            if battle.turn_number.saturating_sub(battle.last_spell_turn) <= 2 {
                if let Some(combo_name) = spell_combo_name(prev_elem, cur_elem) {
                    let combo_msg =
                        apply_spell_combo(battle, combo_name, target_x, target_y);
                    battle.log_message(format!("⚡ COMBO: {}!", combo_name));
                    battle.log_message(&combo_msg);
                    battle.combo_message =
                        Some(format!("⚡ COMBO: {}!", combo_name));
                    battle.combo_message_timer = 60;
                    battle
                        .audio_events
                        .push(AudioEvent::ComboStrike);
                }
            }
        }
        battle.last_spell_element = Some(cur_elem);
        battle.last_spell_turn = battle.turn_number;
    }

    battle.last_spell_school = Some(spell_effect_school(&effect));
    battle.spent_spell_index = Some(spell_idx);
    battle.player_acted = true;

    if battle.all_enemies_dead() {
        battle.phase = TacticalPhase::End {
            victory: true,
            timer: 60,
        };
        return BattleEvent::None;
    }

    if battle.units[0].hp <= 0 {
        battle.units[0].alive = false;
        battle.phase = TacticalPhase::End {
            victory: false,
            timer: 60,
        };
        return BattleEvent::None;
    }

    if !battle.projectiles.is_empty() {
        battle.phase = TacticalPhase::ProjectileAnimation {
            message: msg,
            end_turn: true,
        };
    } else {
        battle.phase = TacticalPhase::Resolve {
            message: msg,
            timer: 30,
            end_turn: true,
        };
    }
    BattleEvent::None
}

pub(super) fn resolve_shield_break(
    battle: &mut TacticalBattle,
    target_idx: usize,
    component: &'static str,
    input: &str,
) -> BattleEvent {
    if target_idx >= battle.units.len() || !battle.units[target_idx].alive {
        return super::try_end_player_turn(battle);
    }

    let correct = if let Some(entry) = vocab::vocab_entry_by_hanzi(component) {
        vocab::check_pinyin(entry, input)
    } else {
        false
    };

    if correct {
        battle.log_message(format!("Shattered {} shield!", component));
        // Remove the first radical action (shield) from the enemy.
        if !battle.units[target_idx].radical_actions.is_empty() {
            battle.units[target_idx].radical_actions.remove(0);
        }
    } else {
        battle.log_message(format!("Shield holds! '{}' incorrect.", input));
    }
    BattleEvent::None
}

pub(super) fn resolve_elite_chain(
    battle: &mut TacticalBattle,
    target_idx: usize,
    syllable_progress: usize,
    total_syllables: usize,
    damage_per_syllable: i32,
    damage_dealt: i32,
    input: &str,
) -> BattleEvent {
    if target_idx >= battle.units.len() || !battle.units[target_idx].alive {
        battle.log_message("Target is gone.");
        return super::try_end_player_turn(battle);
    }

    let pinyin = battle.units[target_idx].pinyin;
    let step = vocab::resolve_compound_pinyin_step(pinyin, syllable_progress, input);

    match step {
        vocab::CompoundPinyinStep::Advanced {
            next_progress,
            total,
            ..
        } => {
            let actual = deal_damage(battle, target_idx, damage_per_syllable);
            let new_dealt = damage_dealt + actual;
            battle.log_message(format!(
                "Part {}/{}! Hit for {} damage!",
                next_progress, total, actual
            ));

            if !battle.units[target_idx].alive {
                battle.last_answer = Some((battle.units[target_idx].hanzi, true));
                battle.combo_streak += 1;
                battle.player_acted = true;
                return super::try_end_player_turn(battle);
            }

            battle.typing_action = Some(TypingAction::EliteChain {
                target_unit: target_idx,
                syllable_progress: next_progress,
                total_syllables,
                damage_per_syllable,
                damage_dealt: new_dealt,
            });
            battle.typing_buffer.clear();
            BattleEvent::None
        }
        vocab::CompoundPinyinStep::Completed { total, .. } => {
            let combo_bonus = 1.0 + (total as f64 - 1.0) * 0.15;
            let flank = flank_bonus(battle, 0, target_idx);
            let final_hit =
                (damage_per_syllable as f64 * combo_bonus * (1.0 + flank)).ceil() as i32;
            let actual = deal_damage(battle, target_idx, final_hit);
            let total_dealt = damage_dealt + actual;

            battle.last_answer = Some((battle.units[target_idx].hanzi, true));
            battle.combo_streak += 1;
            let tier = battle.combo_tier_name();

            let flank_label = if flank >= 0.50 {
                " Backstab!"
            } else if flank >= 0.25 {
                " Flanked!"
            } else {
                ""
            };
            let msg = if tier.is_empty() {
                format!(
                    "Chain complete! {} total damage!{}",
                    total_dealt, flank_label
                )
            } else {
                format!(
                    "{} chain combo! {} total damage!{}",
                    tier, total_dealt, flank_label
                )
            };
            battle.log_message(&msg);

            if crate::status::has_envenomed(&battle.units[0].statuses) {
                battle.units[target_idx]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Poison { damage: 1 }, 3));
                battle.log_message("Venom applied!");
            }

            battle.player_acted = true;
            super::try_end_player_turn(battle)
        }
        vocab::CompoundPinyinStep::Miss { expected, .. } => {
            battle.last_answer = Some((battle.units[target_idx].hanzi, false));
            battle.combo_streak = 0;
            battle.log_message(format!(
                "Chain broken! Expected '{}', got '{}'",
                expected, input
            ));

            let counter_dmg = battle.units[target_idx].damage / 2;
            if counter_dmg > 0 {
                battle.units[0].hp -= counter_dmg;
                battle.log_message(format!("Counter-attack! {} damage!", counter_dmg));
                if battle.units[0].hp <= 0 {
                    battle.units[0].alive = false;
                    battle.phase = TacticalPhase::End {
                        victory: false,
                        timer: 60,
                    };
                    return BattleEvent::None;
                }
            }

            battle.player_acted = true;
            super::try_end_player_turn(battle)
        }
    }
}

