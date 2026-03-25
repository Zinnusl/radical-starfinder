//! Dungeon dialogue outcome application.

use crate::world::dialogue::DungeonOutcome;

pub(crate) fn apply_dungeon_outcome(s: &mut super::GameState, outcome: &DungeonOutcome) -> String {
    match outcome {
        DungeonOutcome::Heal(n) => {
            s.player.hp = (s.player.hp + n).min(s.player.effective_max_hp());
            format!("Healed {} HP", n)
        }
        DungeonOutcome::Damage(n) => {
            s.player.hp = (s.player.hp - n).max(0);
            format!("Took {} damage!", n)
        }
        DungeonOutcome::GainGold(n) => {
            s.player.gold += n;
            format!("Found {} gold!", n)
        }
        DungeonOutcome::LoseGold(n) => {
            s.player.gold = (s.player.gold - n).max(0);
            format!("Lost {} gold!", n)
        }
        DungeonOutcome::GainXp(n) => {
            s.player.skill_tree.gain_xp(*n as u32);
            format!("Gained {} XP!", n)
        }
        DungeonOutcome::GainRadical(r) => {
            format!("Discovered the radical {}!", r)
        }
        DungeonOutcome::GainItem(name) => {
            format!("Found: {}!", name)
        }
        DungeonOutcome::GainEquipment => {
            "Found equipment!".to_string()
        }
        DungeonOutcome::StartFight => {
            "A hostile creature attacks!".to_string()
        }
        DungeonOutcome::Nothing => {
            "You move on.".to_string()
        }
    }
}
