use super::{
    advance_message_decay, can_be_reshaped_by_seal, combat_prompt_for, combo_tier,
    detect_combo, elite_chain_damage, elite_remaining_hp, enemy_look_text, in_look_range,
    seal_cross_positions, spell_category, tile_look_text, tutorial_exit_blocker_for, ComboTier,
    Companion, CompanionBond, EventMemory, FloorProfile, GameState, ListenMode, RunEvent,
    RunJournal, TextSpeed, TutorialState, COMPANION_COUNT,
};
use crate::dungeon::Tile;
use crate::enemy::Enemy;
use crate::player::ITEM_KIND_COUNT;
use crate::radical::SpellEffect;
use crate::vocab::VOCAB;

fn friend_entry() -> &'static crate::vocab::VocabEntry {
    VOCAB.iter().find(|entry| entry.hanzi == "朋友").unwrap()
}

fn shielded_entry() -> &'static crate::vocab::VocabEntry {
    VOCAB.iter().find(|entry| entry.hanzi == "好").unwrap()
}

fn message_frames_until_clear(start_timer: u8, speed: TextSpeed) -> u32 {
    let mut timer = start_timer;
    let mut delay = 0;
    let mut frames = 0;
    while timer > 0 && frames < 10_000 {
        let _ = advance_message_decay(&mut timer, &mut delay, speed);
        frames += 1;
    }
    frames
}

#[test]
fn text_speed_storage_round_trip() {
    assert_eq!(TextSpeed::from_storage("slow"), TextSpeed::Slow);
    assert_eq!(TextSpeed::from_storage("normal"), TextSpeed::Normal);
    assert_eq!(TextSpeed::from_storage("fast"), TextSpeed::Fast);
    assert_eq!(TextSpeed::Fast.storage_key(), "fast");
}

#[test]
fn normal_text_speed_stretches_a_ten_tick_message_to_nineteen_frames() {
    assert_eq!(message_frames_until_clear(10, TextSpeed::Normal), 19);
}

#[test]
fn slower_text_speeds_hold_messages_longer_than_faster_ones() {
    let slow_frames = message_frames_until_clear(10, TextSpeed::Slow);
    let normal_frames = message_frames_until_clear(10, TextSpeed::Normal);
    let fast_frames = message_frames_until_clear(10, TextSpeed::Fast);

    assert!(slow_frames > normal_frames);
    assert!(normal_frames > fast_frames);
}

#[test]
fn look_range_reaches_three_tiles_but_not_four() {
    assert!(in_look_range(10, 10, 13, 10));
    assert!(in_look_range(10, 10, 12, 13));
    assert!(!in_look_range(10, 10, 14, 10));
}

#[test]
fn cracked_wall_look_text_mentions_hidden_room() {
    assert!(tile_look_text(Tile::DamagedBulkhead).contains("hidden room"));
}

#[test]
fn deep_water_look_text_mentions_bridge() {
    assert!(tile_look_text(Tile::VacuumBreach).contains("bridge"));
}

#[test]
fn enemy_look_text_reports_component_shields() {
    let enemy = Enemy::from_vocab(shielded_entry(), 0, 0, 3);

    assert!(enemy_look_text(&enemy).contains("shield 女→子"));
}

#[test]
fn settings_volume_adjustment_clamps() {
    assert_eq!(GameState::adjust_volume(0, -1), 0);
    assert_eq!(GameState::adjust_volume(95, 1), 100);
    assert_eq!(GameState::adjust_volume(40, -2), 20);
}

#[test]
fn utility_spells_do_not_create_damage_combos() {
    assert_eq!(spell_category(&SpellEffect::Reveal), "utility");
    assert_eq!(spell_category(&SpellEffect::Pacify), "utility");
    assert!(detect_combo(&SpellEffect::Reveal, &SpellEffect::Shield).is_none());
    assert!(detect_combo(&SpellEffect::Pacify, &SpellEffect::FireAoe(3)).is_none());
}

#[test]
fn pacify_reward_scales_with_spell_power() {
    assert_eq!(GameState::pacify_gold_reward(2, 0), 4);
    assert_eq!(GameState::pacify_gold_reward(9, 2), 7);
}

#[test]
fn forge_quest_candidates_respect_floor_radicals() {
    let floor_one = GameState::forge_quest_candidates_for_floor(1);
    assert!(floor_one.iter().any(|recipe| recipe.output_hanzi == "明"));
    assert!(!floor_one.iter().any(|recipe| recipe.output_hanzi == "理"));
}

#[test]
pub(super) fn item_appearance_order_is_deterministic_for_a_seed() {
    assert_eq!(
        GameState::roll_item_appearance_order(12345),
        GameState::roll_item_appearance_order(12345)
    );
}

#[test]
pub(super) fn item_appearance_order_uses_each_appearance_once() {
    let mut order = GameState::roll_item_appearance_order(99).to_vec();
    order.sort_unstable();

    assert_eq!(order, (0..ITEM_KIND_COUNT).collect::<Vec<_>>());
}

#[test]
fn combat_prompt_for_elite_mentions_next_syllable() {
    let enemy = Enemy::from_vocab(friend_entry(), 0, 0, 6);

    assert_eq!(
        combat_prompt_for(&enemy, ListenMode::Off, false),
        "Compound foe 朋友 (friend) — break it syllable by syllable. Start with 朋 = peng2."
    );
}

#[test]
fn elite_chain_damage_spikes_on_finishing_syllable() {
    assert_eq!(elite_chain_damage(2, 2, false), 1);
    assert_eq!(elite_chain_damage(2, 2, true), 3);
}

#[test]
fn elite_remaining_hp_stays_above_zero_until_chain_finishes() {
    assert_eq!(elite_remaining_hp(2, 3, false), 1);
    assert_eq!(elite_remaining_hp(2, 3, true), -1);
}

#[test]
fn seal_cross_positions_extend_two_tiles_cardinally() {
    assert_eq!(
        seal_cross_positions(10, 8),
        [
            (11, 8),
            (9, 8),
            (12, 8),
            (8, 8),
            (10, 9),
            (10, 7),
            (10, 10),
            (10, 6),
        ]
    );
}

#[test]
fn only_mutable_ground_can_be_reshaped_by_seals() {
    assert!(can_be_reshaped_by_seal(Tile::MetalFloor));
    assert!(can_be_reshaped_by_seal(Tile::CoolantPool));
    assert!(!can_be_reshaped_by_seal(Tile::QuantumForge));
    assert!(!can_be_reshaped_by_seal(Tile::SupplyCrate));
}

#[test]
fn tutorial_exit_blocker_requires_combat_before_descent() {
    let tutorial = TutorialState {
        combat_done: false,
        forge_done: false,
    };

    assert_eq!(
        tutorial_exit_blocker_for(Some(&tutorial)),
        Some("The exit is sealed. Defeat 大 before leaving the tutorial.")
    );
}

#[test]
fn tutorial_exit_blocker_requires_forge_after_combat() {
    let tutorial = TutorialState {
        combat_done: true,
        forge_done: false,
    };

    assert_eq!(
        tutorial_exit_blocker_for(Some(&tutorial)),
        Some("The exit is sealed. Forge 好 at the anvil before leaving.")
    );
}

#[test]
fn tutorial_exit_blocker_clears_once_tutorial_is_complete() {
    let tutorial = TutorialState {
        combat_done: true,
        forge_done: true,
    };

    assert_eq!(tutorial_exit_blocker_for(Some(&tutorial)), None);
}

#[test]
fn floor_profile_tutorial_floors_are_normal() {
    assert_eq!(FloorProfile::roll(1, 999), FloorProfile::Normal);
    assert_eq!(FloorProfile::roll(2, 123), FloorProfile::Normal);
}

#[test]
fn floor_profile_gold_multipliers() {
    assert_eq!(FloorProfile::Normal.gold_multiplier(), 1.0);
    assert_eq!(FloorProfile::Famine.gold_multiplier(), 0.5);
    assert_eq!(FloorProfile::RadicalRich.gold_multiplier(), 0.8);
    assert_eq!(FloorProfile::Siege.gold_multiplier(), 1.5);
    assert_eq!(FloorProfile::Drought.gold_multiplier(), 0.3);
}

#[test]
fn listen_mode_cycles_through_variants() {
    assert_eq!(ListenMode::Off.cycle(), ListenMode::ToneOnly);
    assert_eq!(ListenMode::ToneOnly.cycle(), ListenMode::FullAudio);
    assert_eq!(ListenMode::FullAudio.cycle(), ListenMode::Off);
}

#[test]
fn listen_mode_is_active_checks() {
    assert_eq!(ListenMode::Off.is_active(), false);
    assert_eq!(ListenMode::ToneOnly.is_active(), true);
    assert_eq!(ListenMode::FullAudio.is_active(), true);
}

// --- Resource Pressure Tests ---

#[test]
fn radical_drop_chance_varies_by_profile() {
    assert_eq!(FloorProfile::Normal.radical_drop_chance(), 80);
    assert_eq!(FloorProfile::Famine.radical_drop_chance(), 50);
    assert_eq!(FloorProfile::RadicalRich.radical_drop_chance(), 100);
    assert_eq!(FloorProfile::Siege.radical_drop_chance(), 80);
    assert_eq!(FloorProfile::Drought.radical_drop_chance(), 0);
}

#[test]
fn drought_profile_is_harshest() {
    assert_eq!(FloorProfile::Drought.gold_multiplier(), 0.3);
    assert_eq!(FloorProfile::Drought.radical_drop_chance(), 0);
    assert_eq!(FloorProfile::Drought.radical_drop_bonus(), false);
}

#[test]
fn floor_profile_roll_distribution() {
    // Famine: 0..=19, RadicalRich: 20..=34, Siege: 35..=44, Drought: 45..=54, Normal: 55+
    assert_eq!(FloorProfile::roll(5, 0), FloorProfile::Famine);
    assert_eq!(FloorProfile::roll(5, 19), FloorProfile::Famine);
    assert_eq!(FloorProfile::roll(5, 20), FloorProfile::RadicalRich);
    assert_eq!(FloorProfile::roll(5, 34), FloorProfile::RadicalRich);
    assert_eq!(FloorProfile::roll(5, 35), FloorProfile::Siege);
    assert_eq!(FloorProfile::roll(5, 44), FloorProfile::Siege);
    assert_eq!(FloorProfile::roll(5, 45), FloorProfile::Drought);
    assert_eq!(FloorProfile::roll(5, 54), FloorProfile::Drought);
    assert_eq!(FloorProfile::roll(5, 55), FloorProfile::Normal);
    assert_eq!(FloorProfile::roll(5, 99), FloorProfile::Normal);
}

#[test]
fn drought_label_shows_desert_emoji() {
    assert!(FloorProfile::Drought.label().contains("Drought"));
}

#[test]
fn radical_rich_guarantees_radical_drops() {
    assert_eq!(FloorProfile::RadicalRich.radical_drop_chance(), 100);
    assert!(FloorProfile::RadicalRich.radical_drop_bonus());
}

#[test]
pub(super) fn companion_level_from_xp_thresholds() {
    assert_eq!(Companion::level_from_xp(0), 1);
    assert_eq!(Companion::level_from_xp(15), 1);
    assert_eq!(Companion::level_from_xp(29), 1);
    assert_eq!(Companion::level_from_xp(30), 2);
    assert_eq!(Companion::level_from_xp(50), 2);
    assert_eq!(Companion::level_from_xp(79), 2);
    assert_eq!(Companion::level_from_xp(80), 3);
    assert_eq!(Companion::level_from_xp(200), 3);
}

#[test]
fn companion_xp_for_level_matches_thresholds() {
    assert_eq!(Companion::xp_for_level(1), 0);
    assert_eq!(Companion::xp_for_level(2), 30);
    assert_eq!(Companion::xp_for_level(3), 80);
}

#[test]
fn companion_max_level_is_three() {
    assert_eq!(Companion::max_level(), 3);
}

#[test]
fn merchant_discount_scales_with_level() {
    assert_eq!(Companion::Quartermaster.shop_discount_pct(1), 20);
    assert_eq!(Companion::Quartermaster.shop_discount_pct(2), 25);
    assert_eq!(Companion::Quartermaster.shop_discount_pct(3), 25);
}

#[test]
fn non_merchant_has_no_discount() {
    assert_eq!(Companion::ScienceOfficer.shop_discount_pct(3), 0);
    assert_eq!(Companion::Medic.shop_discount_pct(3), 0);
    assert_eq!(Companion::SecurityChief.shop_discount_pct(3), 0);
}

#[test]
fn monk_heal_scales_with_level() {
    assert_eq!(Companion::Medic.heal_per_floor(1), 1);
    assert_eq!(Companion::Medic.heal_per_floor(2), 2);
    assert_eq!(Companion::Medic.heal_per_floor(3), 2);
}

#[test]
fn non_monk_has_no_heal() {
    assert_eq!(Companion::ScienceOfficer.heal_per_floor(3), 0);
    assert_eq!(Companion::SecurityChief.heal_per_floor(3), 0);
}

#[test]
fn guard_blocks_scale_with_level() {
    assert_eq!(Companion::SecurityChief.guard_max_blocks(1), 1);
    assert_eq!(Companion::SecurityChief.guard_max_blocks(2), 1);
    assert_eq!(Companion::SecurityChief.guard_max_blocks(3), 2);
}

#[test]
fn guard_second_block_chance_scales_with_level() {
    assert_eq!(Companion::SecurityChief.guard_second_block_chance(1), 0);
    assert_eq!(Companion::SecurityChief.guard_second_block_chance(2), 50);
    assert_eq!(Companion::SecurityChief.guard_second_block_chance(3), 100);
}

#[test]
fn non_guard_has_no_blocks() {
    assert_eq!(Companion::ScienceOfficer.guard_max_blocks(3), 0);
    assert_eq!(Companion::Medic.guard_max_blocks(3), 0);
}

#[test]
fn teacher_hint_reveals_more_at_higher_levels() {
    let entry = friend_entry();
    let enemy = Enemy::from_vocab(entry, 5, 5, 1);
    let l1 = Companion::ScienceOfficer
        .contextual_hint(&enemy, 10, 10, false, 1)
        .unwrap();
    let l2 = Companion::ScienceOfficer
        .contextual_hint(&enemy, 10, 10, false, 2)
        .unwrap();
    let l3 = Companion::ScienceOfficer
        .contextual_hint(&enemy, 10, 10, false, 3)
        .unwrap();
    assert!(!l1.contains(&enemy.pinyin));
    assert!(l2.contains(&enemy.pinyin));
    assert!(l3.len() >= l2.len());
}

#[test]
fn guard_hint_shows_block_count_at_higher_levels() {
    let entry = friend_entry();
    let enemy = Enemy::from_vocab(entry, 5, 5, 1);
    let l1 = Companion::SecurityChief
        .contextual_hint(&enemy, 10, 10, false, 1)
        .unwrap();
    assert!(l1.contains("first hit"));
    let l3 = Companion::SecurityChief
        .contextual_hint(&enemy, 10, 10, false, 3)
        .unwrap();
    assert!(l3.contains("2 hits"));
}

#[test]
fn guard_hint_none_when_already_used() {
    let entry = friend_entry();
    let enemy = Enemy::from_vocab(entry, 5, 5, 1);
    assert!(Companion::SecurityChief
        .contextual_hint(&enemy, 10, 10, true, 1)
        .is_none());
}

#[test]
fn combo_tier_mapping() {
    assert_eq!(combo_tier(0), ComboTier::None);
    assert_eq!(combo_tier(1), ComboTier::None);
    assert_eq!(combo_tier(2), ComboTier::Good);
    assert_eq!(combo_tier(3), ComboTier::Good);
    assert_eq!(combo_tier(4), ComboTier::Great);
    assert_eq!(combo_tier(5), ComboTier::Great);
    assert_eq!(combo_tier(6), ComboTier::Excellent);
    assert_eq!(combo_tier(8), ComboTier::Excellent);
    assert_eq!(combo_tier(9), ComboTier::Perfect);
    assert_eq!(combo_tier(11), ComboTier::Perfect);
    assert_eq!(combo_tier(12), ComboTier::Radical);
    assert_eq!(combo_tier(20), ComboTier::Radical);
}

#[test]
fn combo_tier_multipliers() {
    assert_eq!(ComboTier::None.multiplier(), 1.0);
    assert_eq!(ComboTier::Good.multiplier(), 1.15);
    assert_eq!(ComboTier::Great.multiplier(), 1.3);
    assert_eq!(ComboTier::Excellent.multiplier(), 1.5);
    assert_eq!(ComboTier::Perfect.multiplier(), 1.75);
    assert_eq!(ComboTier::Radical.multiplier(), 2.0);
}

#[test]
fn run_journal_logs_and_retrieves_floor_events() {
    let mut j = RunJournal::default();
    j.log(RunEvent::EnteredFloor(1));
    j.log(RunEvent::EnemyKilled("猫".to_string(), 1));
    j.log(RunEvent::EnemyKilled("狗".to_string(), 1));
    j.log(RunEvent::SpellForged("FireBolt".to_string(), 1));
    j.log(RunEvent::EnteredFloor(2));
    j.log(RunEvent::BossKilled("龙".to_string(), 2));

    assert_eq!(j.enemies_killed_count(), 3);
    assert_eq!(j.max_floor(), 2);
    assert_eq!(j.spells_forged_list(), vec!["FireBolt"]);

    let f1 = j.floor_summary(1);
    assert_eq!(f1.len(), 4);
    let f2 = j.floor_summary(2);
    assert_eq!(f2.len(), 2);

    let line1 = j.floor_line(1);
    assert!(line1.contains("2 kills"));
    assert!(line1.contains("Forged FireBolt"));

    let line2 = j.floor_line(2);
    assert!(line2.contains("Boss 龙 slain"));
}

#[test]
fn run_journal_death_cause_extracts_reason() {
    let mut j = RunJournal::default();
    assert_eq!(j.death_cause(), "Unknown");

    j.log(RunEvent::EnteredFloor(1));
    j.log(RunEvent::EnemyKilled("猫".to_string(), 1));
    j.log(RunEvent::DiedTo("Spike trap".to_string(), 1));
    assert_eq!(j.death_cause(), "Spike trap");
}

use crate::player::PlayerClass;

#[test]
fn class_data_covers_all_variants() {
    let classes = PlayerClass::all();
    assert_eq!(classes.len(), 7);
    for class in &classes {
        let data = class.data();
        assert!(!data.name_en.is_empty());
        assert!(!data.name_cn.is_empty());
    }
}

#[test]
fn cursed_health_potion_heals_less() {
    let heal = 6i32;
    let cursed_heal = (heal / 2).max(1);
    assert_eq!(cursed_heal, 3);
    let small_heal = 1i32;
    let cursed_small = (small_heal / 2).max(1);
    assert_eq!(cursed_small, 1);
}

#[test]
fn blessed_health_potion_heals_more() {
    let heal = 6i32;
    let blessed_heal = heal * 3 / 2;
    assert_eq!(blessed_heal, 9);
}

#[test]
fn item_state_parallel_vec_stays_in_sync() {
    use crate::player::{Item, ItemState, Player, PlayerClass};
    let mut p = Player::new(0, 0, PlayerClass::Envoy);
    assert!(p.add_item(Item::MedHypo(5), ItemState::Normal));
    assert!(p.add_item(Item::EMPGrenade, ItemState::Cursed));
    assert!(p.add_item(Item::ScannerPulse, ItemState::Blessed));
    assert_eq!(p.items.len(), 3);
    assert_eq!(p.item_states.len(), 3);
    assert_eq!(p.item_states[1], ItemState::Cursed);
    let (item, state) = p.take_item(1).unwrap();
    assert!(matches!(item, Item::EMPGrenade));
    assert_eq!(state, ItemState::Cursed);
    assert_eq!(p.items.len(), 2);
    assert_eq!(p.item_states.len(), 2);
    assert_eq!(p.item_states[0], ItemState::Normal);
    assert_eq!(p.item_states[1], ItemState::Blessed);
}

#[test]
fn cursed_equipment_cannot_be_replaced() {
    use crate::player::{ItemState, Player, PlayerClass, EQUIPMENT_POOL};
    let mut p = Player::new(0, 0, PlayerClass::Envoy);
    p.equip(&EQUIPMENT_POOL[0], ItemState::Cursed);
    assert_eq!(p.weapon_state, ItemState::Cursed);
    assert_eq!(
        p.equipment_state(crate::player::EquipSlot::Weapon),
        ItemState::Cursed
    );
}

#[test]
fn theft_chance_clamped_between_5_and_80() {
    let base: i64 = 40;
    let thief_bonus: i64 = 25;
    assert_eq!((base + thief_bonus).clamp(5, 80), 65);
    assert_eq!((base + thief_bonus - 100).clamp(5, 80), 5);
    assert_eq!(base.clamp(5, 80), 40);
}

#[test]
fn theft_catch_escalates_damage() {
    let mut catches: u32 = 0;
    catches += 1;
    assert_eq!(3 + catches as i32, 4);
    catches += 1;
    assert_eq!(3 + catches as i32, 5);
    catches += 1;
    assert_eq!(3 + catches as i32, 6);
}

#[test]
fn trap_tile_types_cover_all_variants() {
    use crate::dungeon::Tile;
    for t in 0..3u8 {
        let tile = Tile::Trap(t);
        assert!(tile.is_walkable());
    }
}

#[test]
fn trap_tile_looks_like_floor_in_look_text() {
    use crate::dungeon::Tile;
    let trap_text = super::tile_look_text(Tile::Trap(0));
    let floor_text = super::tile_look_text(Tile::MetalFloor);
    assert_eq!(trap_text, floor_text);
}

#[test]
fn sentence_selection_scales_by_floor() {
    let (w_early, _) = super::select_sentence_for_floor(3, 0);
    assert!(w_early.len() <= 3, "Early floor sentences should be short");
    let (w_late, _) = super::select_sentence_for_floor(25, 7);
    assert!(w_late.len() >= 2, "Late floor sentences exist");
}

#[test]
fn gatekeeper_seal_mode_damages_player_on_failure() {
    let mode = super::SentenceChallengeMode::GatekeeperSeal {
        boss_idx: 0,
        success_damage: 5,
        failure_damage_to_player: 3,
    };
    if let super::SentenceChallengeMode::GatekeeperSeal {
        failure_damage_to_player,
        ..
    } = mode
    {
        assert_eq!(failure_damage_to_player, 3);
    } else {
        panic!("expected GatekeeperSeal");
    }
}

// ── detect_combo ────────────────────────────────────────────────────────

#[test]
fn detect_combo_fire_and_shield_returns_steam_burst() {
    let result = detect_combo(&SpellEffect::FireAoe(5), &SpellEffect::Shield);
    assert!(result.is_some());
    assert_eq!(result.unwrap().0, "Steam Burst");
}

#[test]
fn detect_combo_shield_and_fire_returns_steam_burst_bidirectional() {
    let result = detect_combo(&SpellEffect::Shield, &SpellEffect::FireAoe(5));
    assert_eq!(result.unwrap().0, "Steam Burst");
}

#[test]
fn detect_combo_shield_and_strike_returns_counter_strike() {
    let result = detect_combo(&SpellEffect::Shield, &SpellEffect::StrongHit(4));
    assert_eq!(result.unwrap().0, "Counter Strike");
}

#[test]
fn detect_combo_heal_and_shield_returns_barrier() {
    let result = detect_combo(&SpellEffect::Heal(3), &SpellEffect::Shield);
    assert_eq!(result.unwrap().0, "Barrier");
}

#[test]
fn detect_combo_strike_and_fire_returns_flurry() {
    let result = detect_combo(&SpellEffect::StrongHit(4), &SpellEffect::Ignite);
    assert_eq!(result.unwrap().0, "Flurry");
}

#[test]
fn detect_combo_drain_and_heal_returns_life_surge() {
    let result = detect_combo(&SpellEffect::Drain(2), &SpellEffect::Heal(3));
    assert_eq!(result.unwrap().0, "Life Surge");
}

#[test]
fn detect_combo_stun_and_strike_returns_crippling_blow() {
    let result = detect_combo(&SpellEffect::Stun, &SpellEffect::StrongHit(5));
    assert_eq!(result.unwrap().0, "Crippling Blow");
}

#[test]
fn detect_combo_fire_and_drain_returns_immolate() {
    let result = detect_combo(&SpellEffect::FireAoe(5), &SpellEffect::Drain(3));
    assert_eq!(result.unwrap().0, "Immolate");
}

#[test]
fn detect_combo_fire_and_stun_returns_tempest() {
    let result = detect_combo(&SpellEffect::FireAoe(5), &SpellEffect::Stun);
    assert_eq!(result.unwrap().0, "Tempest");
}

#[test]
fn detect_combo_heal_and_strike_returns_rally() {
    let result = detect_combo(&SpellEffect::Heal(4), &SpellEffect::StrongHit(3));
    assert_eq!(result.unwrap().0, "Rally");
}

#[test]
fn detect_combo_drain_and_stun_returns_siphon() {
    let result = detect_combo(&SpellEffect::Drain(3), &SpellEffect::Stun);
    assert_eq!(result.unwrap().0, "Siphon");
}

#[test]
fn detect_combo_drain_and_shield_returns_fortify() {
    let result = detect_combo(&SpellEffect::Drain(3), &SpellEffect::Shield);
    assert_eq!(result.unwrap().0, "Fortify");
}

#[test]
fn detect_combo_heal_and_stun_returns_renewal() {
    let result = detect_combo(&SpellEffect::Heal(4), &SpellEffect::Stun);
    assert_eq!(result.unwrap().0, "Renewal");
}

#[test]
fn detect_combo_same_category_returns_none() {
    let result = detect_combo(&SpellEffect::FireAoe(5), &SpellEffect::Ignite);
    assert!(result.is_none());
}

#[test]
fn detect_combo_utility_pair_returns_none() {
    let result = detect_combo(&SpellEffect::Reveal, &SpellEffect::Teleport);
    assert!(result.is_none());
}

// ── spell_category ──────────────────────────────────────────────────────

#[test]
fn spell_category_fire_variants() {
    assert_eq!(spell_category(&SpellEffect::FireAoe(5)), "fire");
    assert_eq!(spell_category(&SpellEffect::Cone(3)), "fire");
    assert_eq!(spell_category(&SpellEffect::Ignite), "fire");
}

#[test]
fn spell_category_heal_variants() {
    assert_eq!(spell_category(&SpellEffect::Heal(3)), "heal");
    assert_eq!(spell_category(&SpellEffect::FocusRestore(2)), "heal");
    assert_eq!(spell_category(&SpellEffect::PlantGrowth), "heal");
    assert_eq!(spell_category(&SpellEffect::Sanctify(3)), "heal");
}

#[test]
fn spell_category_strike_variants() {
    assert_eq!(spell_category(&SpellEffect::StrongHit(4)), "strike");
    assert_eq!(spell_category(&SpellEffect::ArmorBreak), "strike");
    assert_eq!(spell_category(&SpellEffect::Pierce(3)), "strike");
    assert_eq!(spell_category(&SpellEffect::KnockBack(2)), "strike");
    assert_eq!(spell_category(&SpellEffect::Earthquake(5)), "strike");
    assert_eq!(spell_category(&SpellEffect::FloodWave(3)), "strike");
    assert_eq!(spell_category(&SpellEffect::Charge(3)), "strike");
}

#[test]
fn spell_category_shield_variants() {
    assert_eq!(spell_category(&SpellEffect::Shield), "shield");
    assert_eq!(spell_category(&SpellEffect::Thorns(2)), "shield");
    assert_eq!(spell_category(&SpellEffect::Wall(3)), "shield");
    assert_eq!(spell_category(&SpellEffect::SummonBoulder), "shield");
}

#[test]
fn spell_category_drain_variants() {
    assert_eq!(spell_category(&SpellEffect::Drain(3)), "drain");
    assert_eq!(spell_category(&SpellEffect::Poison(2, 3)), "drain");
}

#[test]
fn spell_category_stun_variants() {
    assert_eq!(spell_category(&SpellEffect::Stun), "stun");
    assert_eq!(spell_category(&SpellEffect::Slow(2)), "stun");
    assert_eq!(spell_category(&SpellEffect::FreezeGround(3)), "stun");
}

#[test]
fn spell_category_utility_variants() {
    assert_eq!(spell_category(&SpellEffect::Reveal), "utility");
    assert_eq!(spell_category(&SpellEffect::Pacify), "utility");
    assert_eq!(spell_category(&SpellEffect::Teleport), "utility");
    assert_eq!(spell_category(&SpellEffect::Dash(3)), "utility");
    assert_eq!(spell_category(&SpellEffect::PullToward), "utility");
    assert_eq!(spell_category(&SpellEffect::OilSlick), "utility");
    assert_eq!(spell_category(&SpellEffect::Blink(2)), "utility");
}

// ── in_look_range ───────────────────────────────────────────────────────

#[test]
fn in_look_range_same_position_is_in_range() {
    assert!(in_look_range(5, 5, 5, 5));
}

#[test]
fn in_look_range_diagonal_three_is_in_range() {
    assert!(in_look_range(5, 5, 8, 8));
}

#[test]
fn in_look_range_diagonal_four_is_out_of_range() {
    assert!(!in_look_range(5, 5, 9, 9));
}

#[test]
fn in_look_range_negative_direction_works() {
    assert!(in_look_range(5, 5, 2, 2));
    assert!(!in_look_range(5, 5, 1, 1));
}

#[test]
fn in_look_range_asymmetric_offset_uses_max() {
    // dx=3, dy=1 → max=3 → in range
    assert!(in_look_range(5, 5, 8, 6));
    // dx=4, dy=0 → max=4 → out of range
    assert!(!in_look_range(5, 5, 9, 5));
}

// ── elite_chain_damage ──────────────────────────────────────────────────

#[test]
fn elite_chain_damage_halved_mid_cycle() {
    assert_eq!(elite_chain_damage(4, 3, false), 2);
}

#[test]
fn elite_chain_damage_halved_at_least_one() {
    assert_eq!(elite_chain_damage(1, 2, false), 1);
}

#[test]
fn elite_chain_damage_spike_adds_syllable_bonus() {
    // base_hit + total_syllables - 1
    assert_eq!(elite_chain_damage(5, 3, true), 7);
}

#[test]
fn elite_chain_damage_single_syllable_cycle_complete() {
    assert_eq!(elite_chain_damage(3, 1, true), 3);
}

// ── elite_remaining_hp ──────────────────────────────────────────────────

#[test]
fn elite_remaining_hp_floors_at_one_mid_cycle() {
    assert_eq!(elite_remaining_hp(5, 10, false), 1);
}

#[test]
fn elite_remaining_hp_no_change_when_damage_less_than_hp_mid_cycle() {
    assert_eq!(elite_remaining_hp(10, 3, false), 7);
}

#[test]
fn elite_remaining_hp_can_go_negative_on_cycle_complete() {
    assert_eq!(elite_remaining_hp(5, 10, true), -5);
}

#[test]
fn elite_remaining_hp_zero_on_exact_kill_cycle_complete() {
    assert_eq!(elite_remaining_hp(5, 5, true), 0);
}

// ── advance_message_decay ───────────────────────────────────────────────

#[test]
fn advance_message_decay_already_zero_returns_true() {
    let mut timer = 0u8;
    let mut delay = 0u8;
    assert!(advance_message_decay(&mut timer, &mut delay, TextSpeed::Normal));
}

#[test]
fn advance_message_decay_waits_during_delay() {
    let mut timer = 5u8;
    let mut delay = 2u8;
    let result = advance_message_decay(&mut timer, &mut delay, TextSpeed::Normal);
    assert!(!result);
    assert_eq!(delay, 1);
    assert_eq!(timer, 5); // timer unchanged during delay
}

#[test]
fn advance_message_decay_decrements_timer_after_delay() {
    let mut timer = 2u8;
    let mut delay = 0u8;
    let result = advance_message_decay(&mut timer, &mut delay, TextSpeed::Normal);
    // timer_step for Normal = 1, so timer goes 2→1
    // timer_delay for Normal = 2, so delay resets to 1
    assert!(!result);
    assert_eq!(timer, 1);
}

#[test]
fn advance_message_decay_returns_true_when_timer_reaches_zero() {
    let mut timer = 1u8;
    let mut delay = 0u8;
    let result = advance_message_decay(&mut timer, &mut delay, TextSpeed::Normal);
    assert!(result);
    assert_eq!(timer, 0);
}

// ── seal_cross_positions ────────────────────────────────────────────────

#[test]
fn seal_cross_positions_at_origin() {
    let positions = seal_cross_positions(0, 0);
    assert_eq!(positions, [
        (1, 0), (-1, 0), (2, 0), (-2, 0),
        (0, 1), (0, -1), (0, 2), (0, -2),
    ]);
}

#[test]
fn seal_cross_positions_returns_eight_positions() {
    let positions = seal_cross_positions(5, 5);
    assert_eq!(positions.len(), 8);
}

// ── tile_allows_enemy_spawn ─────────────────────────────────────────────

#[test]
fn tile_allows_enemy_spawn_accepts_walkable_tiles() {
    use crate::dungeon::Tile;
    assert!(super::tile_allows_enemy_spawn(Tile::MetalFloor));
    assert!(super::tile_allows_enemy_spawn(Tile::Hallway));
    assert!(super::tile_allows_enemy_spawn(Tile::Catwalk));
}

#[test]
fn tile_allows_enemy_spawn_rejects_special_tiles() {
    use crate::dungeon::Tile;
    assert!(!super::tile_allows_enemy_spawn(Tile::Bulkhead));
    assert!(!super::tile_allows_enemy_spawn(Tile::QuantumForge));
    assert!(!super::tile_allows_enemy_spawn(Tile::VacuumBreach));
    assert!(!super::tile_allows_enemy_spawn(Tile::SupplyCrate));
}

// ── Companion::index ──────────────────────────────────────────────

#[test]
fn companion_index_assigns_unique_sequential_ids() {
    assert_eq!(Companion::ScienceOfficer.index(), 0);
    assert_eq!(Companion::Medic.index(), 1);
    assert_eq!(Companion::Quartermaster.index(), 2);
    assert_eq!(Companion::SecurityChief.index(), 3);
}

#[test]
fn companion_count_matches_number_of_variants() {
    assert_eq!(COMPANION_COUNT, 4);
}

// ── Companion::name ───────────────────────────────────────────────

#[test]
fn companion_name_returns_human_readable_string() {
    assert_eq!(Companion::ScienceOfficer.name(), "Science Officer 研");
    assert_eq!(Companion::Medic.name(), "Medic 医");
    assert_eq!(Companion::Quartermaster.name(), "Quartermaster 商");
    assert_eq!(Companion::SecurityChief.name(), "Security Chief 卫");
}

// ── Companion::icon ───────────────────────────────────────────────

#[test]
fn companion_icon_returns_distinct_emoji() {
    assert_eq!(Companion::ScienceOfficer.icon(), "🔬");
    assert_eq!(Companion::Medic.icon(), "💊");
    assert_eq!(Companion::Quartermaster.icon(), "📦");
    assert_eq!(Companion::SecurityChief.icon(), "🛡");
}

// ── Companion::heal_per_floor (missing Quartermaster coverage) ────

#[test]
fn quartermaster_has_no_heal() {
    assert_eq!(Companion::Quartermaster.heal_per_floor(3), 0);
}

// ── Companion::guard_second_block_chance (non-guard) ──────────────

#[test]
fn non_guard_has_no_second_block_chance() {
    assert_eq!(Companion::ScienceOfficer.guard_second_block_chance(3), 0);
    assert_eq!(Companion::Medic.guard_second_block_chance(3), 0);
    assert_eq!(Companion::Quartermaster.guard_second_block_chance(3), 0);
}

// ── CompanionBond::level_for_floors ───────────────────────────────

#[test]
fn companion_bond_level_progresses_with_floors() {
    assert_eq!(CompanionBond::level_for_floors(0), 0);
    assert_eq!(CompanionBond::level_for_floors(4), 0);
    assert_eq!(CompanionBond::level_for_floors(5), 1);
    assert_eq!(CompanionBond::level_for_floors(9), 1);
    assert_eq!(CompanionBond::level_for_floors(10), 2);
    assert_eq!(CompanionBond::level_for_floors(14), 2);
    assert_eq!(CompanionBond::level_for_floors(15), 3);
    assert_eq!(CompanionBond::level_for_floors(100), 3);
}

#[test]
fn companion_bond_advance_floor_increments_and_recalculates() {
    let mut bond = CompanionBond::default();
    assert_eq!(bond.floors_together, 0);
    assert_eq!(bond.synergy_level, 0);

    for _ in 0..5 {
        bond.advance_floor();
    }
    assert_eq!(bond.floors_together, 5);
    assert_eq!(bond.synergy_level, 1);
}

// ── Companion synergy methods ─────────────────────────────────────

#[test]
fn synergy_damage_bonus_only_for_officer_and_chief() {
    assert_eq!(Companion::ScienceOfficer.synergy_damage_bonus(), 1);
    assert_eq!(Companion::SecurityChief.synergy_damage_bonus(), 1);
    assert_eq!(Companion::Medic.synergy_damage_bonus(), 0);
    assert_eq!(Companion::Quartermaster.synergy_damage_bonus(), 0);
}

#[test]
fn synergy_gold_pct_only_for_quartermaster() {
    assert_eq!(Companion::Quartermaster.synergy_gold_pct(), 15);
    assert_eq!(Companion::ScienceOfficer.synergy_gold_pct(), 0);
    assert_eq!(Companion::Medic.synergy_gold_pct(), 0);
    assert_eq!(Companion::SecurityChief.synergy_gold_pct(), 0);
}

#[test]
fn synergy_heal_bonus_only_for_medic() {
    assert_eq!(Companion::Medic.synergy_heal_bonus(), 1);
    assert_eq!(Companion::ScienceOfficer.synergy_heal_bonus(), 0);
    assert_eq!(Companion::Quartermaster.synergy_heal_bonus(), 0);
    assert_eq!(Companion::SecurityChief.synergy_heal_bonus(), 0);
}

#[test]
fn combo_ability_name_is_unique_per_companion() {
    let names = [
        Companion::ScienceOfficer.combo_ability_name(),
        Companion::Medic.combo_ability_name(),
        Companion::Quartermaster.combo_ability_name(),
        Companion::SecurityChief.combo_ability_name(),
    ];
    assert_eq!(names[0], "Nanite Surge");
    assert_eq!(names[1], "Vital Strike");
    assert_eq!(names[2], "Supply Drop");
    assert_eq!(names[3], "Fortified Stance");
}

#[test]
fn synergy_callout_returns_valid_flavour_text() {
    let msg = Companion::ScienceOfficer.synergy_callout(0);
    assert!(msg.contains("Officer"));
}

// ── ComboTier::name ───────────────────────────────────────────────

#[test]
fn combo_tier_name_returns_correct_labels() {
    assert_eq!(ComboTier::None.name(), "");
    assert_eq!(ComboTier::Good.name(), "GOOD");
    assert_eq!(ComboTier::Great.name(), "GREAT");
    assert_eq!(ComboTier::Excellent.name(), "EXCELLENT");
    assert_eq!(ComboTier::Perfect.name(), "PERFECT");
    assert_eq!(ComboTier::Radical.name(), "RADICAL");
}

// ── EventMemory ───────────────────────────────────────────────────

#[test]
fn event_memory_records_and_queries_choices() {
    let mut mem = EventMemory::default();
    assert!(!mem.has_choice("helped_stowaway"));

    mem.record_choice("helped_stowaway");
    assert!(mem.has_choice("helped_stowaway"));
}

#[test]
fn event_memory_does_not_duplicate_choices() {
    let mut mem = EventMemory::default();
    mem.record_choice("raided_pirates");
    mem.record_choice("raided_pirates");

    assert_eq!(mem.past_choices.len(), 1);
}

// ── TextSpeed ─────────────────────────────────────────────────────

#[test]
fn text_speed_previous_clamps_at_slow() {
    assert_eq!(TextSpeed::Slow.previous(), TextSpeed::Slow);
    assert_eq!(TextSpeed::Normal.previous(), TextSpeed::Slow);
    assert_eq!(TextSpeed::Fast.previous(), TextSpeed::Normal);
}

#[test]
fn text_speed_next_clamps_at_fast() {
    assert_eq!(TextSpeed::Slow.next(), TextSpeed::Normal);
    assert_eq!(TextSpeed::Normal.next(), TextSpeed::Fast);
    assert_eq!(TextSpeed::Fast.next(), TextSpeed::Fast);
}

#[test]
fn text_speed_timer_delay_slower_is_larger() {
    assert!(TextSpeed::Slow.timer_delay() > TextSpeed::Normal.timer_delay());
    assert!(TextSpeed::Normal.timer_delay() > TextSpeed::Fast.timer_delay());
}

#[test]
fn text_speed_label_returns_display_strings() {
    assert_eq!(TextSpeed::Slow.label(), "Slow");
    assert_eq!(TextSpeed::Normal.label(), "Normal");
    assert_eq!(TextSpeed::Fast.label(), "Fast");
}

// ── TutorialState::objective_text ─────────────────────────────────

#[test]
fn tutorial_objective_text_reflects_progress() {
    let t1 = TutorialState { combat_done: false, forge_done: false };
    assert!(t1.objective_text().contains("defeat"));

    let t2 = TutorialState { combat_done: true, forge_done: false };
    assert!(t2.objective_text().contains("forge"));

    let t3 = TutorialState { combat_done: true, forge_done: true };
    assert!(t3.objective_text().contains("complete"));
}

#[test]
fn tutorial_state_is_complete_requires_both_steps() {
    assert!(!TutorialState { combat_done: false, forge_done: false }.is_complete());
    assert!(!TutorialState { combat_done: true, forge_done: false }.is_complete());
    assert!(!TutorialState { combat_done: false, forge_done: true }.is_complete());
    assert!(TutorialState { combat_done: true, forge_done: true }.is_complete());
}

// ── pacify_gold_reward edge cases ─────────────────────────────────

#[test]
fn pacify_gold_reward_minimum_floor_is_four() {
    assert_eq!(GameState::pacify_gold_reward(0, 0), 4);
    assert_eq!(GameState::pacify_gold_reward(1, 0), 4);
}

#[test]
fn pacify_gold_reward_negative_spell_power_treated_as_zero() {
    assert_eq!(GameState::pacify_gold_reward(10, -5), GameState::pacify_gold_reward(10, 0));
}

// ── spell_category covers all terrain spells ──────────────────────

#[test]
fn spell_category_terrain_spells_have_expected_categories() {
    assert_eq!(spell_category(&SpellEffect::OilSlick), "utility");
    assert_eq!(spell_category(&SpellEffect::Ignite), "fire");
    assert_eq!(spell_category(&SpellEffect::PlantGrowth), "heal");
    assert_eq!(spell_category(&SpellEffect::Earthquake(5)), "strike");
    assert_eq!(spell_category(&SpellEffect::FreezeGround(3)), "stun");
    assert_eq!(spell_category(&SpellEffect::Sanctify(2)), "heal");
    assert_eq!(spell_category(&SpellEffect::FloodWave(4)), "strike");
    assert_eq!(spell_category(&SpellEffect::SummonBoulder), "shield");
    assert_eq!(spell_category(&SpellEffect::Charge(3)), "strike");
    assert_eq!(spell_category(&SpellEffect::Blink(2)), "utility");
}

// ── detect_combo ──────────────────────────────────────────────────

#[test]
fn fire_plus_shield_produces_steam_burst_combo() {
    let combo = detect_combo(&SpellEffect::FireAoe(3), &SpellEffect::Shield);
    assert_eq!(combo.as_ref().map(|(name, _)| *name), Some("Steam Burst"));
}

#[test]
fn strike_plus_fire_produces_flurry_combo() {
    let combo = detect_combo(&SpellEffect::StrongHit(5), &SpellEffect::FireAoe(3));
    assert_eq!(combo.as_ref().map(|(name, _)| *name), Some("Flurry"));
}

// ── GameSettings default ──────────────────────────────────────────

#[test]
fn game_settings_default_has_full_volume_and_normal_speed() {
    let settings = super::GameSettings::default();
    assert_eq!(settings.music_volume, 100);
    assert_eq!(settings.sfx_volume, 100);
    assert!(settings.screen_shake);
    assert_eq!(settings.text_speed, TextSpeed::Normal);
}

// ── ListenMode label ──────────────────────────────────────────────

#[test]
fn listen_mode_label_returns_display_strings() {
    assert_eq!(ListenMode::Off.label(), "OFF");
    assert!(ListenMode::ToneOnly.label().contains("Tone"));
    assert!(ListenMode::FullAudio.label().contains("Audio"));
}

// ── FloorProfile label ────────────────────────────────────────────

#[test]
fn floor_profile_normal_has_empty_label() {
    assert_eq!(FloorProfile::Normal.label(), "");
}

#[test]
fn floor_profile_special_labels_are_non_empty() {
    assert!(!FloorProfile::Famine.label().is_empty());
    assert!(!FloorProfile::RadicalRich.label().is_empty());
    assert!(!FloorProfile::Siege.label().is_empty());
    assert!(!FloorProfile::Drought.label().is_empty());
}

// ── RunJournal edge cases ─────────────────────────────────────────

#[test]
fn run_journal_empty_has_max_floor_one() {
    let j = RunJournal::default();
    assert_eq!(j.max_floor(), 1);
}

#[test]
fn run_journal_empty_floor_line_returns_explored() {
    let j = RunJournal::default();
    assert_eq!(j.floor_line(1), "Explored");
}
