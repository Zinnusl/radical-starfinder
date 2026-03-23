//! Space event outcome processing.

use super::*;
use crate::player::{CrewMember, CrewRole, Item, ItemState};
use crate::world::events::EventOutcome;

pub(crate) fn apply_event_outcome(s: &mut GameState, outcome: &EventOutcome) -> String {
    match outcome {
        EventOutcome::GainFuel(n) => {
            s.ship.fuel = (s.ship.fuel + n).min(s.ship.max_fuel);
            format!("+{} fuel", n)
        }
        EventOutcome::LoseFuel(n) => {
            s.ship.fuel = (s.ship.fuel - n).max(0);
            format!("-{} fuel", n)
        }
        EventOutcome::GainCredits(n) => {
            s.player.gold += n;
            format!("+{} credits", n)
        }
        EventOutcome::LoseCredits(n) => {
            s.player.gold = (s.player.gold - n).max(0);
            format!("-{} credits", n)
        }
        EventOutcome::GainHull(n) => {
            s.ship.hull = (s.ship.hull + n).min(s.ship.max_hull);
            format!("+{} hull", n)
        }
        EventOutcome::LoseHull(n) => {
            s.ship.hull = (s.ship.hull - n).max(1);
            format!("-{} hull", n)
        }
        EventOutcome::RepairShip(n) => {
            s.ship.hull = (s.ship.hull + n).min(s.ship.max_hull);
            format!("Ship repaired +{}", n)
        }
        EventOutcome::ShieldDamage(n) => {
            s.ship.shields = (s.ship.shields - n).max(0);
            format!("-{} shields", n)
        }
        EventOutcome::HealCrew(n) => {
            s.player.hp = (s.player.hp + n).min(s.player.max_hp);
            for crew in s.crew.iter_mut() {
                crew.morale = (crew.morale + 3).min(100);
            }
            format!("Crew healed +{}", n)
        }
        EventOutcome::DamageCrew(n) => {
            s.player.hp = (s.player.hp - n).max(1);
            for crew in s.crew.iter_mut() {
                crew.morale = (crew.morale - 5).max(0);
            }
            format!("Crew took {} damage", n)
        }
        EventOutcome::GainScrap(n) => {
            s.player.gold += n;
            format!("+{} scrap", n)
        }
        EventOutcome::FuelAndCredits(f, c) => {
            s.ship.fuel = (s.ship.fuel + f).min(s.ship.max_fuel);
            s.player.gold += c;
            format!("+{} fuel, +{} credits", f, c)
        }
        EventOutcome::HullAndFuel(h, f) => {
            s.ship.hull = (s.ship.hull + h).min(s.ship.max_hull);
            s.ship.fuel = (s.ship.fuel + f).min(s.ship.max_fuel);
            format!("+{} hull, +{} fuel", h, f)
        }
        EventOutcome::GainRadical(r) => {
            if !s.player.radicals.contains(r) {
                s.player.radicals.push(r);
            }
            format!("Learned radical: {}", r)
        }
        EventOutcome::GainCrewMember => {
            let new_crew = CrewMember {
                name: "New Recruit".to_string(),
                role: CrewRole::Engineer,
                hp: 10, max_hp: 10,
                level: 1, xp: 0,
                morale: 50, skill: 1,
            };
            s.crew.push(new_crew);
            "New crew member joined!".to_string()
        }
        EventOutcome::LoseCrewMember => {
            if s.crew.len() > 1 {
                s.crew.pop();
                "Lost a crew member...".to_string()
            } else {
                "Crew member narrowly survived.".to_string()
            }
        }
        EventOutcome::StartCombat(difficulty) => {
            s.game_mode = GameMode::LocationExploration;
            s.combat = CombatState::Explore;
            for _ in 0..*difficulty {
                s.spawn_enemies();
            }
            "Hostile contact! Entering combat!".to_string()
        }
        EventOutcome::CombatReward(_difficulty, credits) => {
            s.player.gold += credits;
            format!("Combat resolved! +{} credits", credits)
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
            let added = s.player.add_item(item, ItemState::Normal);
            if added {
                s.ship.cargo_used = (s.ship.cargo_used + 1).min(s.ship.cargo_capacity);
                format!("Found: {}!", item_name)
            } else {
                "Found an item but inventory is full!".to_string()
            }
        }
        EventOutcome::Nothing => {
            "Nothing happened.".to_string()
        }
    }
}

