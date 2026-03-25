//! Space event outcome processing.

use crate::player::{CrewMember, CrewRole, Item, ItemState, Player, Ship};
use crate::world::events::{EventOutcome, EventRequirement};

/// Side effects that the caller must handle after applying an event outcome.
pub(crate) enum EventSideEffect {
    None,
    StartCombat { difficulty: i32 },
}

/// Check whether the player/ship/crew meet a starmap event choice requirement.
pub(crate) fn meets_event_requirement(
    player: &Player,
    ship: &Ship,
    crew: &[CrewMember],
    req: &Option<EventRequirement>,
) -> bool {
    match req {
        None | Some(EventRequirement::None) => true,
        Some(EventRequirement::HasCredits(n)) => player.gold >= *n,
        Some(EventRequirement::HasFuel(n)) => ship.fuel >= *n,
        Some(EventRequirement::HasCrewRole(role_id)) => {
            let role = match role_id {
                0 => CrewRole::ScienceOfficer,
                1 => CrewRole::Medic,
                2 => CrewRole::Quartermaster,
                3 => CrewRole::SecurityChief,
                _ => return false,
            };
            crew.iter().any(|c| c.role == role)
        }
        Some(EventRequirement::HasRadical(r)) => player.radicals.contains(r),
        Some(EventRequirement::HasClass(class_id)) => {
            let class = crate::player::PlayerClass::all();
            class.get(*class_id as usize).map_or(false, |c| {
                std::mem::discriminant(&player.class) == std::mem::discriminant(c)
            })
        }
    }
}

pub(crate) fn apply_event_outcome(
    player: &mut Player,
    ship: &mut Ship,
    crew: &mut Vec<CrewMember>,
    outcome: &EventOutcome,
) -> (String, EventSideEffect) {
    match outcome {
        EventOutcome::GainFuel(n) => {
            ship.fuel = (ship.fuel + n).min(ship.max_fuel);
            (format!("+{} fuel", n), EventSideEffect::None)
        }
        EventOutcome::LoseFuel(n) => {
            ship.fuel = (ship.fuel - n).max(0);
            (format!("-{} fuel", n), EventSideEffect::None)
        }
        EventOutcome::GainCredits(n) => {
            player.gold += n;
            (format!("+{} credits", n), EventSideEffect::None)
        }
        EventOutcome::LoseCredits(n) => {
            player.gold = (player.gold - n).max(0);
            (format!("-{} credits", n), EventSideEffect::None)
        }
        EventOutcome::GainHull(n) => {
            ship.hull = (ship.hull + n).min(ship.max_hull);
            (format!("+{} hull", n), EventSideEffect::None)
        }
        EventOutcome::LoseHull(n) => {
            ship.hull = (ship.hull - n).max(1);
            (format!("-{} hull", n), EventSideEffect::None)
        }
        EventOutcome::RepairShip(n) => {
            ship.hull = (ship.hull + n).min(ship.max_hull);
            (format!("Ship repaired +{}", n), EventSideEffect::None)
        }
        EventOutcome::ShieldDamage(n) => {
            ship.shields = (ship.shields - n).max(0);
            (format!("-{} shields", n), EventSideEffect::None)
        }
        EventOutcome::HealCrew(n) => {
            player.hp = (player.hp + n).min(player.max_hp);
            for c in crew.iter_mut() {
                c.morale = (c.morale + 3).min(100);
            }
            (format!("Crew healed +{}", n), EventSideEffect::None)
        }
        EventOutcome::DamageCrew(n) => {
            player.hp = (player.hp - n).max(1);
            for c in crew.iter_mut() {
                c.morale = (c.morale - 5).max(0);
            }
            (format!("Crew took {} damage", n), EventSideEffect::None)
        }
        EventOutcome::GainScrap(n) => {
            player.gold += n;
            (format!("+{} scrap", n), EventSideEffect::None)
        }
        EventOutcome::FuelAndCredits(f, c) => {
            ship.fuel = (ship.fuel + f).min(ship.max_fuel);
            player.gold += c;
            (format!("+{} fuel, +{} credits", f, c), EventSideEffect::None)
        }
        EventOutcome::HullAndFuel(h, f) => {
            ship.hull = (ship.hull + h).min(ship.max_hull);
            ship.fuel = (ship.fuel + f).min(ship.max_fuel);
            (format!("+{} hull, +{} fuel", h, f), EventSideEffect::None)
        }
        EventOutcome::GainRadical(r) => {
            if !player.radicals.contains(r) {
                player.radicals.push(r);
            }
            (format!("Learned radical: {}", r), EventSideEffect::None)
        }
        EventOutcome::GainCrewMember => {
            let new_crew = CrewMember {
                name: "New Recruit".to_string(),
                role: CrewRole::Engineer,
                hp: 10, max_hp: 10,
                level: 1, xp: 0,
                morale: 50, skill: 1,
            };
            crew.push(new_crew);
            ("New crew member joined!".to_string(), EventSideEffect::None)
        }
        EventOutcome::LoseCrewMember => {
            if crew.len() > 1 {
                crew.pop();
                ("Lost a crew member...".to_string(), EventSideEffect::None)
            } else {
                ("Crew member narrowly survived.".to_string(), EventSideEffect::None)
            }
        }
        EventOutcome::StartCombat(difficulty) => {
            (
                "Hostile contact! Entering combat!".to_string(),
                EventSideEffect::StartCombat { difficulty: *difficulty as i32 },
            )
        }
        EventOutcome::CombatReward(_difficulty, credits) => {
            player.gold += credits;
            (format!("Combat resolved! +{} credits", credits), EventSideEffect::None)
        }
        EventOutcome::GainItem(item_name) => {
            let item = match *item_name {
                "nano_shield" => Item::NanoShield(3),
                "toxin_grenade" => Item::ToxinGrenade(12, 3),
                "biogel_patch" => Item::BiogelPatch(5),
                "stim_pack" => Item::StimPack(3),
                "scanner_pulse" => Item::ScannerPulse,
                "emp_grenade" => Item::EMPGrenade,
                "credit_chip" => Item::CreditChip(20),
                "neural_boost" => Item::NeuralBoost,
                _ => Item::MedHypo(15),
            };
            let added = player.add_item(item, ItemState::Normal);
            if added {
                ship.cargo_used = (ship.cargo_used + 1).min(ship.cargo_capacity);
                (format!("Found: {}!", item_name), EventSideEffect::None)
            } else {
                ("Found an item but inventory is full!".to_string(), EventSideEffect::None)
            }
        }
        EventOutcome::Nothing => {
            ("Nothing happened.".to_string(), EventSideEffect::None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{Player, PlayerClass, Ship, CrewMember, CrewRole};

    fn test_player() -> Player {
        Player::new(0, 0, PlayerClass::Envoy)
    }

    fn test_ship() -> Ship {
        Ship::new()
    }

    fn test_crew() -> Vec<CrewMember> {
        vec![CrewMember {
            name: "Test".to_string(),
            role: CrewRole::Pilot,
            hp: 10, max_hp: 10,
            level: 1, xp: 0,
            morale: 50, skill: 1,
        }]
    }

    #[test]
    fn gain_fuel_normal() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        s.fuel = 50;
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::GainFuel(10));
        assert_eq!(s.fuel, 60);
        assert_eq!(msg, "+10 fuel");
    }

    #[test]
    fn gain_fuel_clamps_to_max() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        s.fuel = 95;
        apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::GainFuel(999));
        assert_eq!(s.fuel, s.max_fuel);
    }

    #[test]
    fn lose_fuel_normal() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        s.fuel = 50;
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::LoseFuel(10));
        assert_eq!(s.fuel, 40);
        assert_eq!(msg, "-10 fuel");
    }

    #[test]
    fn lose_fuel_clamps_to_zero() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        s.fuel = 5;
        apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::LoseFuel(999));
        assert_eq!(s.fuel, 0);
    }

    #[test]
    fn gain_credits_normal() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        p.gold = 10;
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::GainCredits(5));
        assert_eq!(p.gold, 15);
        assert_eq!(msg, "+5 credits");
    }

    #[test]
    fn lose_credits_normal() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        p.gold = 20;
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::LoseCredits(5));
        assert_eq!(p.gold, 15);
        assert_eq!(msg, "-5 credits");
    }

    #[test]
    fn lose_credits_clamps_to_zero() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        p.gold = 3;
        apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::LoseCredits(100));
        assert_eq!(p.gold, 0);
    }

    #[test]
    fn gain_hull_normal() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        s.hull = 30;
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::GainHull(10));
        assert_eq!(s.hull, 40);
        assert_eq!(msg, "+10 hull");
    }

    #[test]
    fn gain_hull_clamps_to_max() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        s.hull = s.max_hull - 1;
        apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::GainHull(999));
        assert_eq!(s.hull, s.max_hull);
    }

    #[test]
    fn lose_hull_normal() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        s.hull = 30;
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::LoseHull(10));
        assert_eq!(s.hull, 20);
        assert_eq!(msg, "-10 hull");
    }

    #[test]
    fn lose_hull_clamps_to_one() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        s.hull = 5;
        apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::LoseHull(999));
        assert_eq!(s.hull, 1);
    }

    #[test]
    fn repair_ship() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        s.hull = 20;
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::RepairShip(15));
        assert_eq!(s.hull, 35);
        assert_eq!(msg, "Ship repaired +15");
    }

    #[test]
    fn shield_damage() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        s.shields = 15;
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::ShieldDamage(20));
        assert_eq!(s.shields, 0);
        assert_eq!(msg, "-20 shields");
    }

    #[test]
    fn heal_crew() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        p.hp = 5;
        c[0].morale = 40;
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::HealCrew(3));
        assert_eq!(p.hp, 8);
        assert_eq!(c[0].morale, 43);
        assert_eq!(msg, "Crew healed +3");
    }

    #[test]
    fn heal_crew_hp_capped() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        p.hp = p.max_hp;
        apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::HealCrew(100));
        assert_eq!(p.hp, p.max_hp);
    }

    #[test]
    fn heal_crew_morale_capped() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        c[0].morale = 99;
        apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::HealCrew(1));
        assert_eq!(c[0].morale, 100);
    }

    #[test]
    fn damage_crew() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        p.hp = 8;
        c[0].morale = 50;
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::DamageCrew(3));
        assert_eq!(p.hp, 5);
        assert_eq!(c[0].morale, 45);
        assert_eq!(msg, "Crew took 3 damage");
    }

    #[test]
    fn damage_crew_hp_min_one() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        p.hp = 2;
        apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::DamageCrew(999));
        assert_eq!(p.hp, 1);
    }

    #[test]
    fn damage_crew_morale_min_zero() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        c[0].morale = 2;
        apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::DamageCrew(1));
        assert_eq!(c[0].morale, 0);
    }

    #[test]
    fn gain_scrap() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        p.gold = 10;
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::GainScrap(7));
        assert_eq!(p.gold, 17);
        assert_eq!(msg, "+7 scrap");
    }

    #[test]
    fn fuel_and_credits() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        s.fuel = 50;
        p.gold = 10;
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::FuelAndCredits(5, 15));
        assert_eq!(s.fuel, 55);
        assert_eq!(p.gold, 25);
        assert_eq!(msg, "+5 fuel, +15 credits");
    }

    #[test]
    fn hull_and_fuel() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        s.hull = 30;
        s.fuel = 50;
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::HullAndFuel(10, 5));
        assert_eq!(s.hull, 40);
        assert_eq!(s.fuel, 55);
        assert_eq!(msg, "+10 hull, +5 fuel");
    }

    #[test]
    fn gain_radical_new() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::GainRadical("火"));
        assert!(p.radicals.contains(&"火"));
        assert_eq!(msg, "Learned radical: 火");
    }

    #[test]
    fn gain_radical_duplicate_not_added() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        p.radicals.push("火");
        apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::GainRadical("火"));
        assert_eq!(p.radicals.iter().filter(|&&r| r == "火").count(), 1);
    }

    #[test]
    fn gain_crew_member() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        let before = c.len();
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::GainCrewMember);
        assert_eq!(c.len(), before + 1);
        assert_eq!(msg, "New crew member joined!");
    }

    #[test]
    fn lose_crew_member_with_multiple() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        c.push(CrewMember {
            name: "Extra".to_string(), role: CrewRole::Medic,
            hp: 10, max_hp: 10, level: 1, xp: 0, morale: 50, skill: 1,
        });
        let before = c.len();
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::LoseCrewMember);
        assert_eq!(c.len(), before - 1);
        assert_eq!(msg, "Lost a crew member...");
    }

    #[test]
    fn lose_crew_member_with_single_keeps_them() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        assert_eq!(c.len(), 1);
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::LoseCrewMember);
        assert_eq!(c.len(), 1);
        assert_eq!(msg, "Crew member narrowly survived.");
    }

    #[test]
    fn start_combat_returns_side_effect() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        let (msg, effect) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::StartCombat(3));
        assert_eq!(msg, "Hostile contact! Entering combat!");
        match effect {
            EventSideEffect::StartCombat { difficulty } => assert_eq!(difficulty, 3),
            EventSideEffect::None => panic!("Expected StartCombat side effect"),
        }
    }

    #[test]
    fn combat_reward() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        p.gold = 10;
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::CombatReward(1, 25));
        assert_eq!(p.gold, 35);
        assert_eq!(msg, "Combat resolved! +25 credits");
    }

    #[test]
    fn gain_item_nano_shield() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::GainItem("nano_shield"));
        assert_eq!(msg, "Found: nano_shield!");
    }

    #[test]
    fn gain_item_unknown_defaults_to_medhypo() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::GainItem("unknown_item"));
        assert_eq!(msg, "Found: unknown_item!");
    }

    #[test]
    fn nothing() {
        let mut p = test_player();
        let mut s = test_ship();
        let mut c = test_crew();
        let (msg, _) = apply_event_outcome(&mut p, &mut s, &mut c, &EventOutcome::Nothing);
        assert_eq!(msg, "Nothing happened.");
    }

    // --- Requirement checks ---

    #[test]
    fn requirement_none_always_met() {
        let p = test_player();
        let s = test_ship();
        let c = test_crew();
        assert!(meets_event_requirement(&p, &s, &c, &None));
        assert!(meets_event_requirement(&p, &s, &c, &Some(EventRequirement::None)));
    }

    #[test]
    fn requirement_has_credits_met_when_enough() {
        let mut p = test_player();
        p.gold = 30;
        let s = test_ship();
        let c = test_crew();
        assert!(meets_event_requirement(&p, &s, &c, &Some(EventRequirement::HasCredits(30))));
        assert!(meets_event_requirement(&p, &s, &c, &Some(EventRequirement::HasCredits(1))));
    }

    #[test]
    fn requirement_has_credits_unmet_when_insufficient() {
        let mut p = test_player();
        p.gold = 5;
        let s = test_ship();
        let c = test_crew();
        assert!(!meets_event_requirement(&p, &s, &c, &Some(EventRequirement::HasCredits(10))));
    }

    #[test]
    fn requirement_has_fuel_met_when_enough() {
        let p = test_player();
        let mut s = test_ship();
        s.fuel = 50;
        let c = test_crew();
        assert!(meets_event_requirement(&p, &s, &c, &Some(EventRequirement::HasFuel(50))));
    }

    #[test]
    fn requirement_has_fuel_unmet_when_low() {
        let p = test_player();
        let mut s = test_ship();
        s.fuel = 2;
        let c = test_crew();
        assert!(!meets_event_requirement(&p, &s, &c, &Some(EventRequirement::HasFuel(5))));
    }

    #[test]
    fn requirement_has_crew_role_met_when_present() {
        let p = test_player();
        let s = test_ship();
        let c = vec![CrewMember {
            name: "Doc".to_string(),
            role: CrewRole::Medic,
            hp: 10, max_hp: 10,
            level: 1, xp: 0,
            morale: 50, skill: 1,
        }];
        assert!(meets_event_requirement(&p, &s, &c, &Some(EventRequirement::HasCrewRole(1))));
    }

    #[test]
    fn requirement_has_crew_role_unmet_when_absent() {
        let p = test_player();
        let s = test_ship();
        let c = test_crew(); // has Pilot, not Medic
        assert!(!meets_event_requirement(&p, &s, &c, &Some(EventRequirement::HasCrewRole(1))));
    }

    #[test]
    fn requirement_has_radical_met_when_collected() {
        let mut p = test_player();
        p.radicals.push("水");
        let s = test_ship();
        let c = test_crew();
        assert!(meets_event_requirement(&p, &s, &c, &Some(EventRequirement::HasRadical("水"))));
    }

    #[test]
    fn requirement_has_radical_unmet_when_missing() {
        let p = test_player();
        let s = test_ship();
        let c = test_crew();
        assert!(!meets_event_requirement(&p, &s, &c, &Some(EventRequirement::HasRadical("水"))));
    }

    #[test]
    fn requirement_has_class_met_when_matching() {
        let p = test_player(); // Envoy = index 0
        let s = test_ship();
        let c = test_crew();
        assert!(meets_event_requirement(&p, &s, &c, &Some(EventRequirement::HasClass(0))));
    }

    #[test]
    fn requirement_has_class_unmet_when_wrong() {
        let p = test_player(); // Envoy = index 0
        let s = test_ship();
        let c = test_crew();
        assert!(!meets_event_requirement(&p, &s, &c, &Some(EventRequirement::HasClass(5))));
    }

    #[test]
    fn requirement_invalid_crew_role_always_unmet() {
        let p = test_player();
        let s = test_ship();
        let c = test_crew();
        assert!(!meets_event_requirement(&p, &s, &c, &Some(EventRequirement::HasCrewRole(99))));
    }
}
