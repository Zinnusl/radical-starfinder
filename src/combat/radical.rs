use crate::combat::action::deal_damage;
use crate::combat::terrain::{apply_terrain_interactions, TerrainSource};
use crate::combat::TacticalBattle;
use crate::enemy::RadicalAction;
use crate::status::{StatusInstance, StatusKind};

/// Apply a radical action from enemy `unit_idx` to the tactical battle.
/// Returns a log message describing what happened.
pub fn apply_radical_action(
    battle: &mut TacticalBattle,
    unit_idx: usize,
    action: RadicalAction,
) -> String {
    match action {
        RadicalAction::FireBreath => {
            let ux = battle.units[unit_idx].x;
            let uy = battle.units[unit_idx].y;
            let facing = battle.units[unit_idx].facing;
            let mut fire_tiles = Vec::new();
            for i in 1..=3 {
                fire_tiles.push((ux + facing.dx() * i, uy + facing.dy() * i));
            }
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 3));
            let terrain_msgs =
                apply_terrain_interactions(battle, TerrainSource::FireSpell, &fire_tiles);
            for tm in &terrain_msgs {
                battle.log_message(tm);
            }
            format!("{} — You catch fire!", action.name())
        }
        RadicalAction::WaterShield => {
            let unit = &mut battle.units[unit_idx];
            unit.hp = (unit.hp + 2).min(unit.max_hp);
            format!("{} — Enemy heals 2 HP!", action.name())
        }
        RadicalAction::PowerStrike => {
            let actual = deal_damage(battle, 0, 2);
            format!("{} — Extra {} damage!", action.name(), actual)
        }
        RadicalAction::SelfHeal => {
            let unit = &mut battle.units[unit_idx];
            unit.hp = (unit.hp + 3).min(unit.max_hp);
            format!("{} — Enemy heals 3 HP!", action.name())
        }
        RadicalAction::WarCry => {
            // WarCry affects spirit which lives on Player, not BattleUnit.
            // In tactical mode, we just deal 1 damage as a substitute.
            let actual = deal_damage(battle, 0, 1);
            format!("{} — War cry shakes you! (-{} HP)", action.name(), actual)
        }
        RadicalAction::TrueSight => {
            // Strip defending status from player.
            battle.units[0].defending = false;
            format!("{} — Your guard is broken!", action.name())
        }
        RadicalAction::Disarm => {
            battle.units[0].damage = (battle.units[0].damage - 1).max(1);
            format!("{} — Your grip weakens!", action.name())
        }
        RadicalAction::Root => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Poison { damage: 1 }, 1));
            format!("{} — Roots bind your feet!", action.name())
        }
        RadicalAction::Fortify => {
            battle.units[unit_idx].fortify_stacks += 1;
            battle.units[unit_idx].damage += 1;
            format!("{} — Enemy grows stronger!", action.name())
        }
        RadicalAction::Radiance => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Confused, 1));
            format!("{} — Blinding light!", action.name())
        }
        RadicalAction::ShadowStep => {
            battle.units[unit_idx].radical_dodge = true;
            format!("{} — The enemy fades into shadow!", action.name())
        }
        RadicalAction::CallAlly => {
            // In tactical battle, CallAlly has no effect (no overworld enemies).
            format!("{} — A rallying cry echoes!", action.name())
        }
        RadicalAction::Charm => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Confused, 2));
            format!("{} — Your mind clouds!", action.name())
        }
        RadicalAction::Swift => {
            let dmg = battle.units[unit_idx].damage;
            let actual = deal_damage(battle, 0, dmg);
            format!("{} — Swift follow-up for {} damage!", action.name(), actual)
        }
        RadicalAction::Leech => {
            let dmg = battle.units[unit_idx].damage;
            let actual = deal_damage(battle, 0, dmg);
            let unit = &mut battle.units[unit_idx];
            unit.hp = (unit.hp + actual).min(unit.max_hp);
            format!("{} — Drains {} life force!", action.name(), actual)
        }
        RadicalAction::Multiply => {
            battle.units[unit_idx].radical_multiply = true;
            format!("{} — Next attack strikes twice!", action.name())
        }
        RadicalAction::Armor => {
            battle.units[unit_idx].radical_armor += 2;
            format!("{} — Metal scales form!", action.name())
        }
        RadicalAction::Earthquake => {
            let ux = battle.units[unit_idx].x;
            let uy = battle.units[unit_idx].y;
            let mut affected = Vec::new();
            for dy in -2..=2i32 {
                for dx in -2..=2i32 {
                    if dx.abs() + dy.abs() <= 2 {
                        affected.push((ux + dx, uy + dy));
                    }
                }
            }
            let actual = deal_damage(battle, 0, 1);
            let terrain_msgs =
                apply_terrain_interactions(battle, TerrainSource::Earthquake, &affected);
            for tm in &terrain_msgs {
                battle.log_message(tm);
            }
            format!("{} — The ground shakes! (-{} HP)", action.name(), actual)
        }
    }
}
