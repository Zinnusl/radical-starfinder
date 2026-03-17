use crate::combat::input::{execute_enemy_turn_action, BattleEvent};
use crate::combat::terrain::apply_scorched_damage;
use crate::combat::turn::advance_turn;
use crate::combat::{TacticalBattle, TacticalPhase};
use crate::status::tick_statuses;

const _RESOLVE_FRAMES: u8 = 30; // ~500ms at 60fps
const ENEMY_TURN_FRAMES: u8 = 24; // ~400ms
const END_DELAY_FRAMES: u8 = 60; // ~1s before key accepted

pub fn tick_battle(battle: &mut TacticalBattle) -> BattleEvent {
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
            advance_and_set_phase(battle)
        }
        TacticalPhase::End { ref mut timer, .. } => {
            if *timer > 0 {
                *timer -= 1;
            }
            BattleEvent::None
        }
        _ => BattleEvent::None,
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
        battle.arena.tick_steam();
        apply_exhaustion(battle);
        let scorched_msgs = apply_scorched_damage(battle);
        for msg in &scorched_msgs {
            battle.log_message(msg);
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
        battle.log_message("The ink grows restless...");
    } else if turn >= threshold {
        let escalation = (turn - threshold) / 2;
        let dmg = (1 + escalation as i32).min(5);
        battle.units[0].hp -= dmg;
        battle.log_message(format!("Exhaustion! The ink sears for {} damage!", dmg));
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
