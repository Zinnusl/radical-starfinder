use super::{
    active_set_bonuses, crafted_item, find_crafting_recipe, has_recipe_with,
    Equipment, EquipEffect, EquipSlot, Faction, Item, ItemKind, ItemState,
    Player, PlayerClass, PlayerForm, SetBonus, CRAFTING_RECIPES, EQUIPMENT_POOL,
};

// ── Existing tests ──────────────────────────────────────────────────────────

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

// ── Helper ──────────────────────────────────────────────────────────────────

fn new_soldier() -> Player {
    Player::new(0, 0, PlayerClass::Soldier)
}

fn equipment_by_name(name: &str) -> &'static Equipment {
    EQUIPMENT_POOL.iter().find(|e| e.name == name).unwrap()
}

// ── Player::new — class HP ──────────────────────────────────────────────────

#[test]
fn new_soldier_has_12_hp() {
    let p = Player::new(0, 0, PlayerClass::Soldier);
    assert_eq!(p.hp, 12);
    assert_eq!(p.max_hp, 12);
}

#[test]
fn new_solarian_has_11_hp() {
    let p = Player::new(0, 0, PlayerClass::Solarian);
    assert_eq!(p.hp, 11);
    assert_eq!(p.max_hp, 11);
}

#[test]
fn new_envoy_has_10_hp() {
    let p = Player::new(0, 0, PlayerClass::Envoy);
    assert_eq!(p.hp, 10);
    assert_eq!(p.max_hp, 10);
}

#[test]
fn new_operative_has_10_hp() {
    let p = Player::new(0, 0, PlayerClass::Operative);
    assert_eq!(p.hp, 10);
}

#[test]
fn new_player_spawns_at_given_position() {
    let p = Player::new(7, 13, PlayerClass::Mystic);
    assert_eq!((p.x, p.y), (7, 13));
}

#[test]
fn new_player_starts_with_zero_gold() {
    let p = new_soldier();
    assert_eq!(p.gold, 0);
}

#[test]
fn new_player_starts_with_empty_inventory() {
    let p = new_soldier();
    assert!(p.items.is_empty());
    assert!(p.radicals.is_empty());
    assert!(p.spells.is_empty());
}

#[test]
fn new_player_starts_as_human_form() {
    let p = new_soldier();
    assert_eq!(p.form, PlayerForm::Human);
    assert_eq!(p.form_timer, 0);
}

// ── PlayerClass::all / data ─────────────────────────────────────────────────

#[test]
fn player_class_all_returns_seven_classes() {
    let all = PlayerClass::all();
    assert_eq!(all.len(), 7);
}

#[test]
fn player_class_all_contains_each_variant() {
    let all = PlayerClass::all();
    assert!(all.contains(&PlayerClass::Envoy));
    assert!(all.contains(&PlayerClass::Mechanic));
    assert!(all.contains(&PlayerClass::Mystic));
    assert!(all.contains(&PlayerClass::Operative));
    assert!(all.contains(&PlayerClass::Solarian));
    assert!(all.contains(&PlayerClass::Soldier));
    assert!(all.contains(&PlayerClass::Technomancer));
}

#[test]
fn player_class_data_returns_correct_name_for_soldier() {
    let data = PlayerClass::Soldier.data();
    assert_eq!(data.name_en, "Soldier");
    assert_eq!(data.name_cn, "士兵");
}

#[test]
fn player_class_data_returns_correct_name_for_envoy() {
    let data = PlayerClass::Envoy.data();
    assert_eq!(data.name_en, "Envoy");
}

// ── Piety system ────────────────────────────────────────────────────────────

#[test]
fn get_piety_returns_zero_for_unknown_faction() {
    let p = new_soldier();
    assert_eq!(p.get_piety(Faction::Consortium), 0);
}

#[test]
fn add_piety_creates_entry_for_new_faction() {
    let mut p = new_soldier();
    p.add_piety(Faction::FreeTraders, 7);
    assert_eq!(p.get_piety(Faction::FreeTraders), 7);
}

#[test]
fn add_piety_accumulates_for_same_faction() {
    let mut p = new_soldier();
    p.add_piety(Faction::Technocracy, 3);
    p.add_piety(Faction::Technocracy, 4);
    assert_eq!(p.get_piety(Faction::Technocracy), 7);
}

#[test]
fn highest_faction_returns_none_when_no_piety() {
    let p = new_soldier();
    assert_eq!(p.highest_faction(), None);
}

#[test]
fn highest_faction_returns_faction_with_most_piety() {
    let mut p = new_soldier();
    p.add_piety(Faction::Consortium, 5);
    p.add_piety(Faction::AncientOrder, 12);
    p.add_piety(Faction::MilitaryAlliance, 8);
    assert_eq!(p.highest_faction(), Some(Faction::AncientOrder));
}

#[test]
fn highest_faction_ignores_zero_piety() {
    let mut p = new_soldier();
    p.add_piety(Faction::Consortium, 0);
    assert_eq!(p.highest_faction(), None);
}

// ── Faction bonuses for all factions ────────────────────────────────────────

#[test]
fn faction_bonus_military_alliance_major() {
    let mut p = new_soldier();
    p.add_piety(Faction::MilitaryAlliance, 15);
    assert_eq!(p.faction_bonus(Faction::MilitaryAlliance), "Major: +1 bonus damage");
}

#[test]
fn faction_bonus_ancient_order_major() {
    let mut p = new_soldier();
    p.add_piety(Faction::AncientOrder, 15);
    assert_eq!(p.faction_bonus(Faction::AncientOrder), "Major: +3 bonus credits on kill");
}

#[test]
fn faction_bonus_free_traders_major() {
    let mut p = new_soldier();
    p.add_piety(Faction::FreeTraders, 15);
    assert_eq!(p.faction_bonus(Faction::FreeTraders), "Major: 15% evade on wrong answer");
}

#[test]
fn faction_bonus_technocracy_major() {
    let mut p = new_soldier();
    p.add_piety(Faction::Technocracy, 15);
    assert_eq!(p.faction_bonus(Faction::Technocracy), "Major: Show pinyin on wrong answer");
}

// ── All faction synergies ───────────────────────────────────────────────────

#[test]
fn faction_synergy_trade_accord() {
    let mut p = new_soldier();
    p.add_piety(Faction::Consortium, 10);
    p.add_piety(Faction::AncientOrder, 10);
    assert_eq!(p.faction_synergy().unwrap().0, "Trade Accord");
}

#[test]
fn faction_synergy_deep_scan_array() {
    let mut p = new_soldier();
    p.add_piety(Faction::Technocracy, 10);
    p.add_piety(Faction::FreeTraders, 10);
    assert_eq!(p.faction_synergy().unwrap().0, "Deep Scan Array");
}

#[test]
fn faction_synergy_war_profiteer() {
    let mut p = new_soldier();
    p.add_piety(Faction::MilitaryAlliance, 10);
    p.add_piety(Faction::AncientOrder, 10);
    assert_eq!(p.faction_synergy().unwrap().0, "War Profiteer");
}

#[test]
fn faction_synergy_tactical_uplink() {
    let mut p = new_soldier();
    p.add_piety(Faction::Technocracy, 10);
    p.add_piety(Faction::MilitaryAlliance, 10);
    assert_eq!(p.faction_synergy().unwrap().0, "Tactical Uplink");
}

#[test]
fn faction_synergy_smugglers_luck() {
    let mut p = new_soldier();
    p.add_piety(Faction::FreeTraders, 10);
    p.add_piety(Faction::AncientOrder, 10);
    assert_eq!(p.faction_synergy().unwrap().0, "Smuggler's Luck");
}

// ── Form system ─────────────────────────────────────────────────────────────

#[test]
fn set_form_changes_form_and_timer() {
    let mut p = new_soldier();
    p.set_form(PlayerForm::Powered, 5);
    assert_eq!(p.form, PlayerForm::Powered);
    assert_eq!(p.form_timer, 5);
}

#[test]
fn tick_form_decrements_timer() {
    let mut p = new_soldier();
    p.set_form(PlayerForm::Void, 3);

    p.tick_form();
    assert_eq!(p.form_timer, 2);
    assert_eq!(p.form, PlayerForm::Void);
}

#[test]
fn tick_form_reverts_to_human_when_timer_reaches_zero() {
    let mut p = new_soldier();
    p.set_form(PlayerForm::Holographic, 1);

    p.tick_form();
    assert_eq!(p.form, PlayerForm::Human);
    assert_eq!(p.form_timer, 0);
}

#[test]
fn tick_form_does_nothing_when_already_human() {
    let mut p = new_soldier();
    p.tick_form();
    assert_eq!(p.form, PlayerForm::Human);
    assert_eq!(p.form_timer, 0);
}

#[test]
fn player_form_names_are_distinct() {
    let names: Vec<&str> = vec![
        PlayerForm::Human.name(),
        PlayerForm::Powered.name(),
        PlayerForm::Cybernetic.name(),
        PlayerForm::Holographic.name(),
        PlayerForm::Void.name(),
    ];
    assert_eq!(names.len(), 5);
    assert_eq!(names[0], "Human");
    assert_eq!(names[1], "Powered Suit");
}

#[test]
fn player_form_glyphs_are_distinct() {
    assert_eq!(PlayerForm::Human.glyph(), "@");
    assert_ne!(PlayerForm::Powered.glyph(), PlayerForm::Void.glyph());
}

// ── Max items per class ─────────────────────────────────────────────────────

#[test]
fn max_items_mechanic_has_highest_capacity() {
    let p = Player::new(0, 0, PlayerClass::Mechanic);
    assert_eq!(p.max_items(), 7);
}

#[test]
fn max_items_envoy_and_technomancer_have_six() {
    assert_eq!(Player::new(0, 0, PlayerClass::Envoy).max_items(), 6);
    assert_eq!(Player::new(0, 0, PlayerClass::Technomancer).max_items(), 6);
}

#[test]
fn max_items_mystic_and_operative_have_five() {
    assert_eq!(Player::new(0, 0, PlayerClass::Mystic).max_items(), 5);
    assert_eq!(Player::new(0, 0, PlayerClass::Operative).max_items(), 5);
}

#[test]
fn max_items_soldier_and_solarian_have_four() {
    assert_eq!(Player::new(0, 0, PlayerClass::Soldier).max_items(), 4);
    assert_eq!(Player::new(0, 0, PlayerClass::Solarian).max_items(), 4);
}

// ── Item management ─────────────────────────────────────────────────────────

#[test]
fn add_item_succeeds_when_inventory_not_full() {
    let mut p = new_soldier();
    let added = p.add_item(Item::MedHypo(10), ItemState::Normal);
    assert!(added);
    assert_eq!(p.items.len(), 1);
}

#[test]
fn add_item_returns_false_when_inventory_full() {
    let mut p = new_soldier(); // max 4
    for _ in 0..4 {
        p.add_item(Item::MedHypo(5), ItemState::Normal);
    }
    let added = p.add_item(Item::MedHypo(5), ItemState::Normal);
    assert!(!added);
    assert_eq!(p.items.len(), 4);
}

#[test]
fn add_item_tracks_item_state() {
    let mut p = new_soldier();
    p.add_item(Item::NeuralBoost, ItemState::Cursed);
    assert_eq!(p.item_states[0], ItemState::Cursed);
}

#[test]
fn take_item_removes_and_returns_item_with_state() {
    let mut p = new_soldier();
    p.add_item(Item::ScannerPulse, ItemState::Blessed);
    p.add_item(Item::MedHypo(5), ItemState::Normal);

    let taken = p.take_item(0);
    assert!(taken.is_some());
    let (item, state) = taken.unwrap();
    assert_eq!(item.kind(), ItemKind::ScannerPulse);
    assert_eq!(state, ItemState::Blessed);
    assert_eq!(p.items.len(), 1);
}

#[test]
fn take_item_returns_none_for_out_of_bounds_index() {
    let mut p = new_soldier();
    assert!(p.take_item(0).is_none());
    assert!(p.take_item(99).is_none());
}

// ── Spells ──────────────────────────────────────────────────────────────────

#[test]
fn add_spell_stores_spell() {
    let mut p = new_soldier();
    p.add_spell(crate::radical::Spell {
        hanzi: "火",
        pinyin: "huo3",
        meaning: "fire",
        effect: crate::radical::SpellEffect::FireAoe(3),
    });
    assert_eq!(p.spells.len(), 1);
}

#[test]
fn cycle_spell_returns_false_when_no_spells() {
    let mut p = new_soldier();
    assert!(!p.cycle_spell());
}

#[test]
fn cycle_spell_advances_selected_index() {
    let mut p = new_soldier();
    let make_spell = |h| crate::radical::Spell {
        hanzi: h,
        pinyin: "x",
        meaning: "x",
        effect: crate::radical::SpellEffect::Shield,
    };
    p.add_spell(make_spell("A"));
    p.add_spell(make_spell("B"));
    p.add_spell(make_spell("C"));

    assert_eq!(p.selected_spell, 0);
    p.cycle_spell();
    assert_eq!(p.selected_spell, 1);
    p.cycle_spell();
    assert_eq!(p.selected_spell, 2);
    p.cycle_spell();
    assert_eq!(p.selected_spell, 0); // wraps around
}

#[test]
fn use_spell_returns_none_when_no_spells() {
    let mut p = new_soldier();
    assert!(p.use_spell().is_none());
}

#[test]
fn use_spell_removes_and_returns_selected_spell() {
    let mut p = new_soldier();
    p.add_spell(crate::radical::Spell {
        hanzi: "水",
        pinyin: "shui3",
        meaning: "water",
        effect: crate::radical::SpellEffect::Heal(5),
    });
    p.add_spell(crate::radical::Spell {
        hanzi: "火",
        pinyin: "huo3",
        meaning: "fire",
        effect: crate::radical::SpellEffect::FireAoe(3),
    });

    p.selected_spell = 1;
    let spell = p.use_spell().unwrap();
    assert_eq!(spell.hanzi, "火");
    assert_eq!(p.spells.len(), 1);
    assert_eq!(p.selected_spell, 0); // resets since index >= len
}

#[test]
fn use_spell_resets_selected_to_zero_when_at_end() {
    let mut p = new_soldier();
    let make_spell = |h| crate::radical::Spell {
        hanzi: h,
        pinyin: "x",
        meaning: "x",
        effect: crate::radical::SpellEffect::Shield,
    };
    p.add_spell(make_spell("A"));
    p.add_spell(make_spell("B"));
    p.selected_spell = 1;

    p.use_spell();
    assert_eq!(p.selected_spell, 0);
}

// ── Radicals ────────────────────────────────────────────────────────────────

#[test]
fn add_radical_appends_to_collection() {
    let mut p = new_soldier();
    p.add_radical("火");
    p.add_radical("水");
    assert_eq!(p.radicals, vec!["火", "水"]);
}

// ── Movement ────────────────────────────────────────────────────────────────

#[test]
fn intended_move_computes_target_position() {
    let p = Player::new(5, 5, PlayerClass::Soldier);
    assert_eq!(p.intended_move(1, 0), (6, 5));
    assert_eq!(p.intended_move(-1, -1), (4, 4));
}

#[test]
fn move_to_updates_position() {
    let mut p = new_soldier();
    p.move_to(10, 20);
    assert_eq!((p.x, p.y), (10, 20));
}

// ── Equipment ───────────────────────────────────────────────────────────────

#[test]
fn equip_weapon_sets_weapon_slot() {
    let mut p = new_soldier();
    let weapon = equipment_by_name("Laser Pistol");
    p.equip(weapon, ItemState::Normal);

    assert!(p.weapon.is_some());
    assert_eq!(p.weapon.unwrap().name, "Laser Pistol");
    assert_eq!(p.weapon_state, ItemState::Normal);
}

#[test]
fn equip_armor_sets_armor_slot() {
    let mut p = new_soldier();
    let armor = equipment_by_name("Kevlar Vest");
    p.equip(armor, ItemState::Blessed);

    assert!(p.armor.is_some());
    assert_eq!(p.armor.unwrap().name, "Kevlar Vest");
    assert_eq!(p.armor_state, ItemState::Blessed);
}

#[test]
fn equip_charm_sets_charm_slot() {
    let mut p = new_soldier();
    let charm = equipment_by_name("Scanner Array");
    p.equip(charm, ItemState::Cursed);

    assert!(p.charm.is_some());
    assert_eq!(p.charm.unwrap().name, "Scanner Array");
    assert_eq!(p.charm_state, ItemState::Cursed);
}

#[test]
fn equipment_state_returns_correct_state_per_slot() {
    let mut p = new_soldier();
    p.equip(equipment_by_name("Laser Pistol"), ItemState::Blessed);
    p.equip(equipment_by_name("Kevlar Vest"), ItemState::Cursed);
    p.equip(equipment_by_name("Scanner Array"), ItemState::Normal);

    assert_eq!(p.equipment_state(EquipSlot::Weapon), ItemState::Blessed);
    assert_eq!(p.equipment_state(EquipSlot::Armor), ItemState::Cursed);
    assert_eq!(p.equipment_state(EquipSlot::Charm), ItemState::Normal);
}

#[test]
fn equipped_effects_returns_empty_when_nothing_equipped() {
    let p = new_soldier();
    assert!(p.equipped_effects().is_empty());
}

#[test]
fn equipped_effects_returns_all_equipped_effects() {
    let mut p = new_soldier();
    p.equip(equipment_by_name("Laser Pistol"), ItemState::Normal);
    p.equip(equipment_by_name("Kevlar Vest"), ItemState::Normal);
    p.equip(equipment_by_name("Scanner Array"), ItemState::Normal);

    let effects = p.equipped_effects();
    assert_eq!(effects.len(), 3);
}

// ── Stat calculations (equipment-based) ─────────────────────────────────────

#[test]
fn bonus_damage_from_weapon() {
    let mut p = new_soldier();
    p.equip(equipment_by_name("Arc Emitter"), ItemState::Normal); // BonusDamage(3)
    assert_eq!(p.bonus_damage(), 3);
}

#[test]
fn bonus_damage_zero_without_weapon() {
    let p = new_soldier();
    assert_eq!(p.bonus_damage(), 0);
}

#[test]
fn damage_reduction_from_armor() {
    let mut p = new_soldier();
    p.equip(equipment_by_name("Power Armor"), ItemState::Normal); // DamageReduction(3)
    assert_eq!(p.damage_reduction(), 3);
}

#[test]
fn damage_reduction_zero_without_armor() {
    let p = new_soldier();
    assert_eq!(p.damage_reduction(), 0);
}

#[test]
fn extra_radical_chance_from_charm() {
    let mut p = new_soldier();
    p.equip(equipment_by_name("Scanner Array"), ItemState::Normal); // ExtraRadicalDrop(50)
    assert_eq!(p.extra_radical_chance(), 50);
}

#[test]
fn heal_on_kill_from_charm() {
    let mut p = new_soldier();
    p.equip(equipment_by_name("Auto-Repair Module"), ItemState::Normal); // HealOnKill(2)
    assert_eq!(p.heal_on_kill(), 2);
}

#[test]
fn gold_bonus_from_charm() {
    let mut p = new_soldier();
    p.equip(equipment_by_name("Salvage Processor"), ItemState::Normal); // GoldBonus(10)
    assert_eq!(p.gold_bonus(), 10);
}

#[test]
fn total_crit_chance_from_weapon() {
    let mut p = new_soldier();
    p.equip(equipment_by_name("Void Lance"), ItemState::Normal); // CriticalStrike(20)
    assert_eq!(p.total_crit_chance(), 20);
}

#[test]
fn total_dodge_chance_zero_by_default() {
    let p = new_soldier();
    assert_eq!(p.total_dodge_chance(), 0);
}

#[test]
fn max_hp_bonus_zero_by_default() {
    let p = new_soldier();
    assert_eq!(p.max_hp_bonus(), 0);
}

#[test]
fn effective_max_hp_equals_max_hp_when_no_bonuses() {
    let p = new_soldier();
    assert_eq!(p.effective_max_hp(), 12);
}

#[test]
fn effective_max_hp_is_at_least_one() {
    let mut p = new_soldier();
    p.max_hp = 0;
    assert_eq!(p.effective_max_hp(), 1);
}

// ── Equipment descriptions ──────────────────────────────────────────────────

#[test]
fn equipment_description_includes_slot() {
    let weapon = equipment_by_name("Laser Pistol");
    let desc = weapon.description();
    assert!(desc.starts_with("[Weapon]"));
}

#[test]
fn armor_description_includes_armor_slot() {
    let armor = equipment_by_name("Kevlar Vest");
    let desc = armor.description();
    assert!(desc.starts_with("[Armor]"));
}

#[test]
fn charm_description_includes_module_slot() {
    let charm = equipment_by_name("Scanner Array");
    let desc = charm.description();
    assert!(desc.starts_with("[Module]"));
}

// ── EquipEffect ─────────────────────────────────────────────────────────────

#[test]
fn equip_effect_same_variant_matches_regardless_of_value() {
    assert!(EquipEffect::BonusDamage(1).same_variant(&EquipEffect::BonusDamage(99)));
}

#[test]
fn equip_effect_same_variant_rejects_different_variant() {
    assert!(!EquipEffect::BonusDamage(1).same_variant(&EquipEffect::DamageReduction(1)));
}

#[test]
fn equip_effect_description_is_non_empty() {
    let effects = [
        EquipEffect::BonusDamage(1),
        EquipEffect::DamageReduction(2),
        EquipEffect::ExtraRadicalDrop(50),
        EquipEffect::HealOnKill(2),
        EquipEffect::GoldBonus(10),
        EquipEffect::Digging,
        EquipEffect::PassiveRegen,
        EquipEffect::SpellPowerBoost(1),
        EquipEffect::LifeSteal(1),
        EquipEffect::DodgeChance(15),
        EquipEffect::FocusRegen(1),
        EquipEffect::KnockbackStrike,
        EquipEffect::ThornsAura(1),
        EquipEffect::EnemyIntentReveal,
        EquipEffect::CriticalStrike(20),
        EquipEffect::HardAnswerDamage(3),
        EquipEffect::HardAnswerArmor(2),
        EquipEffect::HardAnswerHeal(2),
    ];
    for effect in &effects {
        assert!(!effect.description().is_empty());
    }
}

// ── Set bonuses ─────────────────────────────────────────────────────────────

#[test]
fn active_set_bonuses_empty_when_nothing_equipped() {
    let p = new_soldier();
    assert!(active_set_bonuses(&p).is_empty());
}

#[test]
fn frontline_rig_set_bonus_activates_with_matching_gear() {
    let mut p = new_soldier();
    p.equip(equipment_by_name("Laser Pistol"), ItemState::Normal); // BonusDamage
    p.equip(equipment_by_name("Flight Suit"), ItemState::Normal); // DamageReduction
    p.equip(equipment_by_name("Energy Recycler"), ItemState::Normal); // PassiveRegen

    let bonuses = active_set_bonuses(&p);
    assert_eq!(bonuses.len(), 1);
    assert_eq!(bonuses[0].name, "Frontline Rig");
}

#[test]
fn scholars_trinity_set_bonus_activates() {
    let mut p = new_soldier();
    p.equip(equipment_by_name("Lexicon Blade"), ItemState::Normal);
    p.equip(equipment_by_name("Scholar's Aegis"), ItemState::Normal);
    p.equip(equipment_by_name("Wisdom Core"), ItemState::Normal);

    let bonuses = active_set_bonuses(&p);
    assert_eq!(bonuses.len(), 1);
    assert_eq!(bonuses[0].name, "Scholar's Trinity");
}

#[test]
fn has_set_bonus_returns_true_for_matching_bonus() {
    let mut p = new_soldier();
    p.equip(equipment_by_name("Laser Pistol"), ItemState::Normal);
    p.equip(equipment_by_name("Flight Suit"), ItemState::Normal);
    p.equip(equipment_by_name("Energy Recycler"), ItemState::Normal);

    assert!(p.has_set_bonus(|b| matches!(b, SetBonus::BonusDamage(_))));
}

#[test]
fn has_set_bonus_returns_false_when_no_matching_bonus() {
    let p = new_soldier();
    assert!(!p.has_set_bonus(|b| matches!(b, SetBonus::PhaseWalk)));
}

// ── Enchantments ────────────────────────────────────────────────────────────

#[test]
fn enchant_bonus_damage_sums_matching_radicals() {
    let mut p = new_soldier();
    p.enchantments = [Some("力"), Some("火"), None];
    assert_eq!(p.enchant_bonus_damage(), 2);
}

#[test]
fn enchant_bonus_damage_zero_with_no_enchantments() {
    let p = new_soldier();
    assert_eq!(p.enchant_bonus_damage(), 0);
}

#[test]
fn enchant_damage_reduction_sums_water_and_earth() {
    let mut p = new_soldier();
    p.enchantments = [Some("水"), Some("土"), None];
    assert_eq!(p.enchant_damage_reduction(), 2);
}

#[test]
fn enchant_max_hp_bonus_from_heart() {
    let mut p = new_soldier();
    p.enchantments = [Some("心"), Some("心"), None];
    assert_eq!(p.enchant_max_hp_bonus(), 4);
}

#[test]
fn enchant_gold_bonus_from_metal() {
    let mut p = new_soldier();
    p.enchantments = [None, None, Some("金")];
    assert_eq!(p.enchant_gold_bonus(), 3);
}

#[test]
fn enchant_fov_bonus_from_eye() {
    let mut p = new_soldier();
    p.enchantments = [Some("目"), None, None];
    assert_eq!(p.enchant_fov_bonus(), 1);
}

#[test]
fn enchant_ignores_non_matching_radicals() {
    let mut p = new_soldier();
    p.enchantments = [Some("木"), Some("木"), Some("木")];
    assert_eq!(p.enchant_bonus_damage(), 0);
    assert_eq!(p.enchant_damage_reduction(), 0);
    assert_eq!(p.enchant_gold_bonus(), 0);
    assert_eq!(p.enchant_fov_bonus(), 0);
}

// ── Crafting ────────────────────────────────────────────────────────────────

#[test]
fn find_crafting_recipe_returns_recipe_for_valid_pair() {
    let recipe = find_crafting_recipe(ItemKind::MedHypo, ItemKind::ToxinGrenade);
    assert!(recipe.is_some());
    assert_eq!(recipe.unwrap().output, ItemKind::OmniGel);
}

#[test]
fn find_crafting_recipe_is_order_independent() {
    let r1 = find_crafting_recipe(ItemKind::MedHypo, ItemKind::ToxinGrenade);
    let r2 = find_crafting_recipe(ItemKind::ToxinGrenade, ItemKind::MedHypo);
    assert_eq!(r1.unwrap().output, r2.unwrap().output);
}

#[test]
fn find_crafting_recipe_returns_none_for_invalid_pair() {
    let recipe = find_crafting_recipe(ItemKind::MedHypo, ItemKind::ScannerPulse);
    assert!(recipe.is_none());
}

#[test]
fn has_recipe_with_returns_true_for_valid_pairing() {
    assert!(has_recipe_with(ItemKind::MedHypo, ItemKind::ToxinGrenade));
}

#[test]
fn has_recipe_with_returns_false_for_unpairable() {
    assert!(!has_recipe_with(ItemKind::ScannerPulse, ItemKind::PersonalTeleporter));
}

#[test]
fn crafted_item_omni_gel_from_medhypo_and_toxin() {
    let recipe = find_crafting_recipe(ItemKind::MedHypo, ItemKind::ToxinGrenade).unwrap();
    let result = crafted_item(recipe, &Item::MedHypo(5), &Item::ToxinGrenade(3, 2));
    assert!(matches!(result, Item::OmniGel));
}

#[test]
fn crafted_item_sonic_emitter_scales_from_plasma_burst() {
    let recipe = find_crafting_recipe(ItemKind::PlasmaBurst, ItemKind::NaniteSwarm).unwrap();
    let result = crafted_item(recipe, &Item::PlasmaBurst(5), &Item::NaniteSwarm);
    // PlasmaBurst base=5, +3 = 8
    assert!(matches!(result, Item::SonicEmitter(8)));
}

#[test]
fn crafted_item_synth_ale_from_double_medhypo() {
    let recipe = find_crafting_recipe(ItemKind::MedHypo, ItemKind::MedHypo).unwrap();
    let result = crafted_item(recipe, &Item::MedHypo(5), &Item::MedHypo(10));
    assert!(matches!(result, Item::SynthAle(6)));
}

#[test]
fn crafted_item_cloaking_device_from_holo_toxin() {
    let recipe = find_crafting_recipe(ItemKind::HoloDecoy, ItemKind::ToxinGrenade).unwrap();
    let result = crafted_item(recipe, &Item::HoloDecoy(3), &Item::ToxinGrenade(2, 1));
    assert!(matches!(result, Item::CloakingDevice(5)));
}

// ── Item names/descriptions/sell prices ─────────────────────────────────────

#[test]
fn item_name_is_non_empty_for_all_kinds() {
    let items: Vec<Item> = vec![
        Item::MedHypo(5),
        Item::ToxinGrenade(3, 2),
        Item::ScannerPulse,
        Item::PersonalTeleporter,
        Item::EMPGrenade,
        Item::NeuralBoost,
        Item::VenomDart,
        Item::NaniteSwarm,
        Item::ReflectorPlate,
        Item::NavComputer,
        Item::GrappleLine,
        Item::OmniGel,
        Item::CircuitInk,
        Item::ThrusterPack,
        Item::AdrenalineInjector,
        Item::GamblersChip,
        Item::OverchargeCell,
    ];
    for item in &items {
        assert!(!item.name().is_empty());
    }
}

#[test]
fn item_sell_price_is_positive() {
    let items = vec![
        Item::MedHypo(5),
        Item::ScannerPulse,
        Item::Revitalizer(15),
    ];
    for item in &items {
        assert!(item.sell_price() > 0);
    }
}

#[test]
fn item_sell_price_is_40_percent_of_base() {
    // MedHypo base = 25, 40% = 10
    assert_eq!(Item::MedHypo(5).sell_price(), 10);
}

#[test]
fn item_description_is_non_empty() {
    assert!(!Item::MedHypo(5).description().is_empty());
    assert!(!Item::EMPGrenade.description().is_empty());
}

// ── ItemKind index ──────────────────────────────────────────────────────────

#[test]
fn item_kind_indices_are_unique() {
    let kinds = vec![
        ItemKind::MedHypo, ItemKind::ToxinGrenade, ItemKind::ScannerPulse,
        ItemKind::PersonalTeleporter, ItemKind::StimPack, ItemKind::EMPGrenade,
    ];
    let indices: Vec<usize> = kinds.iter().map(|k| k.index()).collect();
    for (i, a) in indices.iter().enumerate() {
        for (j, b) in indices.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "Duplicate index for kinds at positions {} and {}", i, j);
            }
        }
    }
}

#[test]
fn item_kind_med_hypo_index_is_zero() {
    assert_eq!(ItemKind::MedHypo.index(), 0);
}

#[test]
fn item_kind_overcharge_cell_index_is_34() {
    assert_eq!(ItemKind::OverchargeCell.index(), 34);
}

// ── Apply meta progression ──────────────────────────────────────────────────

#[test]
fn apply_meta_progression_adds_hp_bonus() {
    let mut p = new_soldier(); // 12 HP
    p.apply_meta_progression(3, 0, 0);
    assert_eq!(p.max_hp, 15);
    assert_eq!(p.hp, 15);
}

#[test]
fn apply_meta_progression_sets_shop_discount() {
    let mut p = new_soldier();
    p.apply_meta_progression(0, 10, 0);
    assert_eq!(p.shop_discount_pct, 10);
}

#[test]
fn apply_meta_progression_clamps_negative_values_to_zero() {
    let mut p = new_soldier();
    p.apply_meta_progression(0, -5, -3);
    assert_eq!(p.shop_discount_pct, 0);
    assert_eq!(p.spell_power_bonus, 0);
}

// ── Faction name ────────────────────────────────────────────────────────────

#[test]
fn faction_names_contain_descriptor() {
    assert!(Faction::Consortium.name().contains("Commerce"));
    assert!(Faction::FreeTraders.name().contains("Exploration"));
    assert!(Faction::Technocracy.name().contains("Knowledge"));
    assert!(Faction::MilitaryAlliance.name().contains("Defense"));
    assert!(Faction::AncientOrder.name().contains("Secrets"));
}

// ── Ship ────────────────────────────────────────────────────────────────────

#[test]
fn ship_new_has_full_hull_and_fuel() {
    let ship = super::Ship::new();
    assert_eq!(ship.hull, ship.max_hull);
    assert_eq!(ship.fuel, ship.max_fuel);
    assert_eq!(ship.shields, ship.max_shields);
}

// ── Equipment pool ──────────────────────────────────────────────────────────

#[test]
fn equipment_pool_has_all_slots_represented() {
    let has_weapon = EQUIPMENT_POOL.iter().any(|e| e.slot == EquipSlot::Weapon);
    let has_armor = EQUIPMENT_POOL.iter().any(|e| e.slot == EquipSlot::Armor);
    let has_charm = EQUIPMENT_POOL.iter().any(|e| e.slot == EquipSlot::Charm);
    assert!(has_weapon);
    assert!(has_armor);
    assert!(has_charm);
}

// ── Item short_name ─────────────────────────────────────────────────────────

#[test]
fn item_short_name_is_non_empty() {
    assert!(!Item::MedHypo(5).short_name().is_empty());
    assert!(!Item::OverchargeCell.short_name().is_empty());
}


// ── Item name / short_name / description — all 35 variants ──────────────────

fn all_items() -> Vec<Item> {
    vec![
        Item::MedHypo(5),
        Item::ToxinGrenade(3, 2),
        Item::ScannerPulse,
        Item::PersonalTeleporter,
        Item::StimPack(3),
        Item::EMPGrenade,
        Item::RationPack(5),
        Item::FocusStim(3),
        Item::SynthAle(2),
        Item::HoloDecoy(3),
        Item::PlasmaBurst(4),
        Item::NanoShield(3),
        Item::NeuralBoost,
        Item::CreditChip(50),
        Item::ShockModule(8),
        Item::BiogelPatch(2),
        Item::VenomDart,
        Item::DeflectorDrone(3),
        Item::NaniteSwarm,
        Item::Revitalizer(15),
        Item::ReflectorPlate,
        Item::CryoGrenade(3),
        Item::CloakingDevice(4),
        Item::PlasmaShield(2),
        Item::SignalJammer(3),
        Item::NavComputer,
        Item::GrappleLine,
        Item::OmniGel,
        Item::SonicEmitter(5),
        Item::CircuitInk,
        Item::DataCore(10),
        Item::ThrusterPack,
        Item::AdrenalineInjector,
        Item::GamblersChip,
        Item::OverchargeCell,
    ]
}

#[test]
fn item_name_non_empty_all_35_variants() {
    for item in all_items() {
        assert!(!item.name().is_empty(), "name empty for {:?}", item);
    }
}

#[test]
fn item_short_name_non_empty_all_35_variants() {
    for item in all_items() {
        assert!(!item.short_name().is_empty(), "short_name empty for {:?}", item);
    }
}

#[test]
fn item_description_non_empty_all_35_variants() {
    for item in all_items() {
        assert!(!item.description().is_empty(), "description empty for {:?}", item);
    }
}

#[test]
fn item_sell_price_positive_all_35_variants() {
    for item in all_items() {
        assert!(item.sell_price() > 0, "sell_price zero for {:?}", item);
    }
}

// ── PlayerForm::color — all 5 forms ─────────────────────────────────────────

#[test]
fn player_form_color_human_is_white() {
    assert_eq!(PlayerForm::Human.color(), "#ffffff");
}

#[test]
fn player_form_color_powered_is_orange_red() {
    assert_eq!(PlayerForm::Powered.color(), "#ff5500");
}

#[test]
fn player_form_color_cybernetic_is_grey() {
    assert_eq!(PlayerForm::Cybernetic.color(), "#888888");
}

#[test]
fn player_form_color_holographic_is_light_blue() {
    assert_eq!(PlayerForm::Holographic.color(), "#aaddff");
}

#[test]
fn player_form_color_void_is_amber() {
    assert_eq!(PlayerForm::Void.color(), "#ffaa00");
}

// ── Faction::name — all 5 factions ──────────────────────────────────────────

#[test]
fn faction_name_consortium_is_stellar_consortium() {
    assert_eq!(Faction::Consortium.name(), "Stellar Consortium (Commerce)");
}

#[test]
fn faction_name_free_traders_is_free_traders_guild() {
    assert_eq!(Faction::FreeTraders.name(), "Free Traders Guild (Exploration)");
}

#[test]
fn faction_name_technocracy_is_technocracy() {
    assert_eq!(Faction::Technocracy.name(), "Technocracy (Knowledge)");
}

#[test]
fn faction_name_military_alliance_is_military_alliance() {
    assert_eq!(Faction::MilitaryAlliance.name(), "Military Alliance (Defense)");
}

#[test]
fn faction_name_ancient_order_is_ancient_order() {
    assert_eq!(Faction::AncientOrder.name(), "Ancient Order (Secrets)");
}
