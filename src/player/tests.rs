use super::{Faction, Item, ItemKind, Player, PlayerClass};

#[test]
fn item_kind_matches_variant() {
    assert_eq!(Item::MedHypo(5).kind(), ItemKind::MedHypo);
    assert_eq!(Item::PersonalTeleporter.kind(), ItemKind::PersonalTeleporter);
}

#[test]
fn item_display_name_uses_mystery_label_until_identified() {
    let item = Item::ScannerPulse;

    assert_eq!(
        item.display_name(false, "Green Capsule ◇"),
        "? Green Capsule ◇"
    );
    assert_eq!(
        item.display_name(true, "Green Capsule ◇"),
        "📡 Scanner Pulse"
    );
}

#[test]
fn faction_synergy_requires_dual_standing() {
    let mut player = Player::new(0, 0, PlayerClass::Soldier);
    player.add_piety(Faction::Consortium, 10);
    player.add_piety(Faction::MilitaryAlliance, 10);
    assert_eq!(
        player.faction_synergy(),
        Some(("Vanguard Protocol", "Heal 1 HP per kill AND +1 damage"))
    );
}

#[test]
fn faction_synergy_returns_none_without_threshold() {
    let mut player = Player::new(0, 0, PlayerClass::Soldier);
    player.add_piety(Faction::Consortium, 9);
    player.add_piety(Faction::MilitaryAlliance, 10);
    assert_eq!(player.faction_synergy(), None);
}

#[test]
fn faction_bonus_tiers() {
    let mut player = Player::new(0, 0, PlayerClass::Soldier);
    assert_eq!(player.faction_bonus(Faction::Consortium), "None");

    player.add_piety(Faction::Consortium, 5);
    assert_eq!(player.faction_bonus(Faction::Consortium), "Minor standing");

    player.add_piety(Faction::Consortium, 5);
    assert_eq!(
        player.faction_bonus(Faction::Consortium),
        "Moderate: +1 HP on kill"
    );

    player.add_piety(Faction::Consortium, 5);
    assert_eq!(player.faction_bonus(Faction::Consortium), "Major: +1 HP on kill");
}

