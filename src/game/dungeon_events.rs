//! Dungeon dialogue outcome application.

use crate::player::Player;
use crate::world::dialogue::DungeonOutcome;

pub(crate) fn apply_dungeon_outcome(player: &mut Player, outcome: &DungeonOutcome) -> String {
    match outcome {
        DungeonOutcome::Heal(n) => {
            player.hp = (player.hp + n).min(player.effective_max_hp());
            format!("Healed {} HP", n)
        }
        DungeonOutcome::Damage(n) => {
            player.hp = (player.hp - n).max(0);
            format!("Took {} damage!", n)
        }
        DungeonOutcome::GainGold(n) => {
            player.gold += n;
            format!("Found {} gold!", n)
        }
        DungeonOutcome::LoseGold(n) => {
            player.gold = (player.gold - n).max(0);
            format!("Lost {} gold!", n)
        }
        DungeonOutcome::GainXp(n) => {
            player.skill_tree.gain_xp(*n as u32);
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
        DungeonOutcome::GainCredits(n) => {
            player.gold += n;
            format!("Gained {} credits!", n)
        }
        DungeonOutcome::LoseCredits(n) => {
            player.gold = (player.gold - n).max(0);
            format!("Lost {} credits!", n)
        }
        DungeonOutcome::GainCrewMember => {
            "A new crew member joins you!".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{Player, PlayerClass};

    fn test_player() -> Player {
        Player::new(0, 0, PlayerClass::Envoy)
    }

    #[test]
    fn heal_normal() {
        let mut p = test_player();
        p.hp = 5;
        let msg = apply_dungeon_outcome(&mut p, &DungeonOutcome::Heal(3));
        assert_eq!(p.hp, 8);
        assert_eq!(msg, "Healed 3 HP");
    }

    #[test]
    fn heal_clamps_to_max() {
        let mut p = test_player();
        let max = p.effective_max_hp();
        p.hp = max - 1;
        let msg = apply_dungeon_outcome(&mut p, &DungeonOutcome::Heal(100));
        assert_eq!(p.hp, max);
        assert!(msg.contains("100"));
    }

    #[test]
    fn damage_normal() {
        let mut p = test_player();
        p.hp = 8;
        let msg = apply_dungeon_outcome(&mut p, &DungeonOutcome::Damage(3));
        assert_eq!(p.hp, 5);
        assert_eq!(msg, "Took 3 damage!");
    }

    #[test]
    fn damage_clamps_to_zero() {
        let mut p = test_player();
        p.hp = 2;
        apply_dungeon_outcome(&mut p, &DungeonOutcome::Damage(999));
        assert_eq!(p.hp, 0);
    }

    #[test]
    fn gain_gold() {
        let mut p = test_player();
        p.gold = 10;
        let msg = apply_dungeon_outcome(&mut p, &DungeonOutcome::GainGold(5));
        assert_eq!(p.gold, 15);
        assert_eq!(msg, "Found 5 gold!");
    }

    #[test]
    fn lose_gold_normal() {
        let mut p = test_player();
        p.gold = 10;
        let msg = apply_dungeon_outcome(&mut p, &DungeonOutcome::LoseGold(3));
        assert_eq!(p.gold, 7);
        assert_eq!(msg, "Lost 3 gold!");
    }

    #[test]
    fn lose_gold_clamps_to_zero() {
        let mut p = test_player();
        p.gold = 2;
        apply_dungeon_outcome(&mut p, &DungeonOutcome::LoseGold(100));
        assert_eq!(p.gold, 0);
    }

    #[test]
    fn gain_xp() {
        let mut p = test_player();
        let msg = apply_dungeon_outcome(&mut p, &DungeonOutcome::GainXp(10));
        assert_eq!(msg, "Gained 10 XP!");
    }

    #[test]
    fn gain_radical_message() {
        let mut p = test_player();
        let msg = apply_dungeon_outcome(&mut p, &DungeonOutcome::GainRadical("水"));
        assert_eq!(msg, "Discovered the radical 水!");
    }

    #[test]
    fn gain_item_message() {
        let mut p = test_player();
        let msg = apply_dungeon_outcome(&mut p, &DungeonOutcome::GainItem("Laser Sword"));
        assert_eq!(msg, "Found: Laser Sword!");
    }

    #[test]
    fn gain_equipment_message() {
        let mut p = test_player();
        let msg = apply_dungeon_outcome(&mut p, &DungeonOutcome::GainEquipment);
        assert_eq!(msg, "Found equipment!");
    }

    #[test]
    fn start_fight_message() {
        let mut p = test_player();
        let msg = apply_dungeon_outcome(&mut p, &DungeonOutcome::StartFight);
        assert_eq!(msg, "A hostile creature attacks!");
    }

    #[test]
    fn nothing_message() {
        let mut p = test_player();
        let msg = apply_dungeon_outcome(&mut p, &DungeonOutcome::Nothing);
        assert_eq!(msg, "You move on.");
    }

    #[test]
    fn gain_credits() {
        let mut p = test_player();
        p.gold = 5;
        let msg = apply_dungeon_outcome(&mut p, &DungeonOutcome::GainCredits(10));
        assert_eq!(p.gold, 15);
        assert_eq!(msg, "Gained 10 credits!");
    }

    #[test]
    fn lose_credits_clamps_to_zero() {
        let mut p = test_player();
        p.gold = 3;
        apply_dungeon_outcome(&mut p, &DungeonOutcome::LoseCredits(100));
        assert_eq!(p.gold, 0);
    }

    #[test]
    fn gain_crew_member_message() {
        let mut p = test_player();
        let msg = apply_dungeon_outcome(&mut p, &DungeonOutcome::GainCrewMember);
        assert_eq!(msg, "A new crew member joins you!");
    }
}
