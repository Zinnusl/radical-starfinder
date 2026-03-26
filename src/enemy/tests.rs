use super::{AiBehavior, BossKind, Enemy, RadicalAction, PlayerRadicalAbility, SkillType};
use crate::vocab::VOCAB;

fn friend_entry() -> &'static crate::vocab::VocabEntry {
    VOCAB.iter().find(|entry| entry.hanzi == "朋友").unwrap()
}

fn single_char_entry() -> &'static crate::vocab::VocabEntry {
    VOCAB.iter().find(|entry| entry.hanzi == "好").unwrap()
}

// ── BossKind ────────────────────────────────────────────────────────────────

#[test]
fn boss_kind_matches_key_floors() {
    assert_eq!(BossKind::for_floor(5), Some(BossKind::PirateCaptain));
    assert_eq!(BossKind::for_floor(10), Some(BossKind::HiveQueen));
    assert_eq!(BossKind::for_floor(15), Some(BossKind::RogueAICore));
    assert_eq!(BossKind::for_floor(20), Some(BossKind::VoidEntity));
    assert_eq!(BossKind::for_floor(25), Some(BossKind::AncientGuardian));
    assert_eq!(BossKind::for_floor(30), Some(BossKind::DriftLeviathan));
    assert_eq!(BossKind::for_floor(35), None);
}

#[test]
fn boss_kind_returns_none_for_non_boss_floor() {
    assert_eq!(BossKind::for_floor(1), None);
    assert_eq!(BossKind::for_floor(7), None);
    assert_eq!(BossKind::for_floor(0), None);
}

#[test]
fn boss_kind_title_pirate_captain() {
    assert_eq!(BossKind::PirateCaptain.title(), "Pirate Captain");
}

#[test]
fn boss_kind_title_hive_queen() {
    assert_eq!(BossKind::HiveQueen.title(), "Hive Queen");
}

#[test]
fn boss_kind_title_rogue_ai_core() {
    assert_eq!(BossKind::RogueAICore.title(), "Rogue AI Core");
}

#[test]
fn boss_kind_title_void_entity() {
    assert_eq!(BossKind::VoidEntity.title(), "Void Entity");
}

#[test]
fn boss_kind_title_ancient_guardian() {
    assert_eq!(BossKind::AncientGuardian.title(), "Ancient Guardian");
}

#[test]
fn boss_kind_title_drift_leviathan() {
    assert_eq!(BossKind::DriftLeviathan.title(), "Drift Leviathan");
}

// ── Enemy::from_vocab ───────────────────────────────────────────────────────

#[test]
fn from_vocab_sets_hanzi_and_pinyin() {
    let entry = friend_entry();
    let enemy = Enemy::from_vocab(entry, 3, 4, 1);
    assert_eq!(enemy.hanzi, "朋友");
    assert_eq!(enemy.pinyin, entry.pinyin);
    assert_eq!(enemy.meaning, entry.meaning);
}

#[test]
fn from_vocab_sets_position() {
    let enemy = Enemy::from_vocab(friend_entry(), 7, 11, 1);
    assert_eq!((enemy.x, enemy.y), (7, 11));
}

#[test]
fn from_vocab_multi_char_is_elite() {
    let enemy = Enemy::from_vocab(friend_entry(), 0, 0, 1);
    assert!(enemy.is_elite, "Multi-character word should be elite");
    assert!(!enemy.is_boss);
}

#[test]
fn from_vocab_single_char_is_not_elite() {
    // Find a single-char entry
    let entry = VOCAB.iter().find(|e| e.hanzi.chars().count() == 1).unwrap();
    let enemy = Enemy::from_vocab(entry, 0, 0, 1);
    assert!(!enemy.is_elite);
}

#[test]
fn from_vocab_elite_has_higher_hp_than_normal() {
    let floor = 5;
    let elite_entry = friend_entry(); // multi-char = elite
    let normal_entry = VOCAB.iter().find(|e| e.hanzi.chars().count() == 1).unwrap();

    let elite = Enemy::from_vocab(elite_entry, 0, 0, floor);
    let normal = Enemy::from_vocab(normal_entry, 0, 0, floor);
    assert!(elite.hp > normal.hp);
}

#[test]
fn from_vocab_elite_has_higher_damage_than_normal() {
    let floor = 8;
    let elite = Enemy::from_vocab(friend_entry(), 0, 0, floor);
    let normal_entry = VOCAB.iter().find(|e| e.hanzi.chars().count() == 1).unwrap();
    let normal = Enemy::from_vocab(normal_entry, 0, 0, floor);
    assert!(elite.damage >= normal.damage);
}

#[test]
fn from_vocab_elite_has_chase_ai() {
    let enemy = Enemy::from_vocab(friend_entry(), 0, 0, 1);
    assert_eq!(enemy.ai, AiBehavior::Chase);
}

#[test]
fn from_vocab_starts_not_alert() {
    let enemy = Enemy::from_vocab(friend_entry(), 0, 0, 1);
    assert!(!enemy.alert);
}

#[test]
fn from_vocab_hp_scales_with_floor() {
    let e1 = Enemy::from_vocab(friend_entry(), 0, 0, 1);
    let e10 = Enemy::from_vocab(friend_entry(), 0, 0, 10);
    assert!(e10.hp > e1.hp);
}

#[test]
fn from_vocab_gold_scales_with_floor() {
    let e1 = Enemy::from_vocab(friend_entry(), 0, 0, 1);
    let e10 = Enemy::from_vocab(friend_entry(), 0, 0, 10);
    assert!(e10.gold_value > e1.gold_value);
}

// ── Enemy::boss_from_vocab ──────────────────────────────────────────────────

#[test]
fn boss_from_vocab_is_boss() {
    let entry = friend_entry();
    let boss = Enemy::boss_from_vocab(entry, 0, 0, 5);
    assert!(boss.is_boss);
    assert!(!boss.is_elite);
}

#[test]
fn boss_from_vocab_is_always_alert() {
    let boss = Enemy::boss_from_vocab(friend_entry(), 0, 0, 5);
    assert!(boss.alert);
}

#[test]
fn boss_from_vocab_has_correct_boss_kind() {
    let boss = Enemy::boss_from_vocab(friend_entry(), 0, 0, 5);
    assert_eq!(boss.boss_kind, Some(BossKind::PirateCaptain));
}

#[test]
fn boss_from_vocab_has_chase_ai() {
    let boss = Enemy::boss_from_vocab(friend_entry(), 0, 0, 5);
    assert_eq!(boss.ai, AiBehavior::Chase);
}

#[test]
fn boss_from_vocab_pirate_captain_has_summon_cooldown() {
    let boss = Enemy::boss_from_vocab(friend_entry(), 0, 0, 5);
    assert_eq!(boss.summon_cooldown, 1);
}

#[test]
fn boss_from_vocab_hive_queen_has_zero_cooldown() {
    let boss = Enemy::boss_from_vocab(friend_entry(), 0, 0, 10);
    assert_eq!(boss.summon_cooldown, 0);
}

#[test]
fn boss_from_vocab_void_entity_has_cooldown_two() {
    let boss = Enemy::boss_from_vocab(friend_entry(), 0, 0, 20);
    assert_eq!(boss.summon_cooldown, 2);
}

#[test]
fn boss_from_vocab_non_boss_floor_still_creates_boss() {
    let boss = Enemy::boss_from_vocab(friend_entry(), 0, 0, 3);
    assert!(boss.is_boss);
    assert_eq!(boss.boss_kind, None);
}

#[test]
fn boss_from_vocab_drift_leviathan_has_highest_hp() {
    let pirate = Enemy::boss_from_vocab(friend_entry(), 0, 0, 5);
    let leviathan = Enemy::boss_from_vocab(friend_entry(), 0, 0, 30);
    assert!(leviathan.hp > pirate.hp);
}

// ── Enemy::is_alive ─────────────────────────────────────────────────────────

#[test]
fn is_alive_true_when_hp_positive() {
    let enemy = Enemy::from_vocab(friend_entry(), 0, 0, 1);
    assert!(enemy.is_alive());
}

#[test]
fn is_alive_false_when_hp_zero() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 0, 0, 1);
    enemy.hp = 0;
    assert!(!enemy.is_alive());
}

#[test]
fn is_alive_false_when_hp_negative() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 0, 0, 1);
    enemy.hp = -5;
    assert!(!enemy.is_alive());
}

// ── Enemy::step_toward ──────────────────────────────────────────────────────

#[test]
fn step_toward_moves_along_primary_axis() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    enemy.x = 5;
    enemy.y = 5;

    let (nx, ny) = enemy.step_toward(10, 5);
    assert_eq!((nx, ny), (6, 5));
}

#[test]
fn step_toward_prefers_x_when_equal_distance() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    let (nx, ny) = enemy.step_toward(8, 8);
    // Equal distance, should prefer x axis (abs(dx) >= abs(dy))
    assert_eq!((nx, ny), (6, 5));
}

#[test]
fn step_toward_moves_in_y_when_y_distance_larger() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    let (nx, ny) = enemy.step_toward(5, 10);
    assert_eq!((nx, ny), (5, 6));
}

#[test]
fn step_toward_same_position_stays_put() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    let (nx, ny) = enemy.step_toward(5, 5);
    assert_eq!((nx, ny), (5, 5));
}

// ── Enemy::step_retreat ─────────────────────────────────────────────────────

#[test]
fn step_retreat_moves_away_from_target() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    let (nx, ny) = enemy.step_retreat(3, 5);
    // Target at (3,5) is left of enemy, retreat goes right
    assert_eq!((nx, ny), (6, 5));
}

#[test]
fn step_retreat_moves_away_in_y_when_y_distance_larger() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    let (nx, ny) = enemy.step_retreat(5, 2);
    assert_eq!((nx, ny), (5, 6));
}

// ── Enemy::ai_step ──────────────────────────────────────────────────────────

#[test]
fn elite_expected_syllable_tracks_chain_progress() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 0, 0, 6);
    enemy.elite_chain = 1;

    assert_eq!(enemy.elite_expected_syllable(), Some("you3"));
}

#[test]
fn ai_behavior_dispatch_covers_all_variants() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);

    enemy.ai = AiBehavior::Chase;
    let _ = enemy.ai_step(10, 10, 0);

    enemy.ai = AiBehavior::Retreat;
    let _ = enemy.ai_step(10, 10, 0);

    enemy.ai = AiBehavior::Ambush;
    let _ = enemy.ai_step(10, 10, 0);

    enemy.ai = AiBehavior::Sentinel;
    let _ = enemy.ai_step(10, 10, 0);

    enemy.ai = AiBehavior::Kiter;
    let _ = enemy.ai_step(10, 10, 0);

    enemy.ai = AiBehavior::Pack;
    let _ = enemy.ai_step(10, 10, 0);
}

#[test]
fn sentinel_holds_position_when_far() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    enemy.ai = AiBehavior::Sentinel;
    let (nx, ny) = enemy.ai_step(10, 10, 0);
    assert_eq!((nx, ny), (5, 5));
}

#[test]
fn sentinel_chases_when_adjacent() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    enemy.ai = AiBehavior::Sentinel;
    let (nx, ny) = enemy.ai_step(6, 5, 0);
    assert_eq!((nx, ny), (6, 5));
}

#[test]
fn kiter_retreats_when_close() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    enemy.ai = AiBehavior::Kiter;
    let (nx, ny) = enemy.ai_step(6, 5, 0);
    assert_ne!((nx, ny), (6, 5));
    assert!((nx - 5i32).abs() + (ny - 5i32).abs() <= 1);
}

#[test]
fn kiter_advances_when_far() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 0, 0, 1);
    enemy.ai = AiBehavior::Kiter;
    let (nx, ny) = enemy.ai_step(10, 10, 0);
    assert!(nx > 0 || ny > 0);
}

#[test]
fn kiter_holds_at_medium_range() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    enemy.ai = AiBehavior::Kiter;
    // dist = 3+1 = 4, in the hold zone (3..=4)
    let (nx, ny) = enemy.ai_step(8, 6, 0);
    assert_eq!((nx, ny), (5, 5));
}

#[test]
fn pack_holds_without_allies() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    enemy.ai = AiBehavior::Pack;
    let (nx, ny) = enemy.ai_step(10, 10, 0);
    assert_eq!((nx, ny), (5, 5));
}

#[test]
fn pack_chases_with_enough_allies() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    enemy.ai = AiBehavior::Pack;
    let (nx, ny) = enemy.ai_step(10, 10, 2);
    assert_ne!((nx, ny), (5, 5));
}

#[test]
fn pack_chases_when_adjacent_even_alone() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    enemy.ai = AiBehavior::Pack;
    let (nx, ny) = enemy.ai_step(6, 5, 0);
    assert_eq!((nx, ny), (6, 5));
}

#[test]
fn chase_always_moves_toward_target() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 0, 0, 1);
    enemy.ai = AiBehavior::Chase;
    let (nx, ny) = enemy.ai_step(5, 3, 0);
    let new_dist = (5 - nx).abs() + (3 - ny).abs();
    let old_dist = (5_i32 - 0).abs() + (3_i32 - 0).abs();
    assert!(new_dist < old_dist);
}

#[test]
fn retreat_approaches_when_very_close() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    enemy.ai = AiBehavior::Retreat;
    // dist=1, retreaters approach when dist <= 2
    let (nx, ny) = enemy.ai_step(6, 5, 0);
    assert_eq!((nx, ny), (6, 5));
}

#[test]
fn retreat_flees_when_far() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    enemy.ai = AiBehavior::Retreat;
    // dist=5, retreaters flee
    let (nx, ny) = enemy.ai_step(10, 5, 0);
    assert!(nx < 5 || ny < 5 || (nx == 5 && ny == 5)); // moved away or stayed
    assert_eq!((nx, ny), (4, 5)); // retreats along x axis
}

#[test]
fn ambush_waits_when_far() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    enemy.ai = AiBehavior::Ambush;
    let (nx, ny) = enemy.ai_step(15, 15, 0); // dist=20
    assert_eq!((nx, ny), (5, 5));
}

#[test]
fn ambush_charges_when_close() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 5, 5, 1);
    enemy.ai = AiBehavior::Ambush;
    let (nx, ny) = enemy.ai_step(7, 5, 0); // dist=2 <= 3
    assert_ne!((nx, ny), (5, 5));
}

// ── RadicalAction ───────────────────────────────────────────────────────────

#[test]
fn radical_action_from_known_radicals() {
    assert_eq!(
        RadicalAction::from_radical("火"),
        Some(RadicalAction::SpreadingWildfire)
    );
    assert_eq!(
        RadicalAction::from_radical("水"),
        Some(RadicalAction::ErosiveFlow)
    );
    assert_eq!(
        RadicalAction::from_radical("心"),
        Some(RadicalAction::DoubtSeed)
    );
    assert_eq!(
        RadicalAction::from_radical("又"),
        Some(RadicalAction::EchoStrike)
    );
    assert_eq!(RadicalAction::from_radical("xyz"), None);
}

#[test]
fn radical_action_name_is_non_empty_for_all_variants() {
    let actions = [
        RadicalAction::SpreadingWildfire,
        RadicalAction::ErosiveFlow,
        RadicalAction::OverwhelmingForce,
        RadicalAction::DoubtSeed,
        RadicalAction::DevouringMaw,
        RadicalAction::WitnessMark,
        RadicalAction::SleightReversal,
        RadicalAction::RootingGrasp,
        RadicalAction::HarvestReaping,
        RadicalAction::RevealingDawn,
        RadicalAction::WaningCurse,
        RadicalAction::MortalResilience,
        RadicalAction::MaternalShield,
        RadicalAction::PotentialBurst,
        RadicalAction::EchoStrike,
        RadicalAction::PhaseStrike,
    ];
    for action in &actions {
        assert!(!action.name().is_empty(), "Name empty for {:?}", action);
    }
}

#[test]
fn radical_action_radical_roundtrips() {
    let actions = [
        RadicalAction::SpreadingWildfire,
        RadicalAction::ErosiveFlow,
        RadicalAction::OverwhelmingForce,
        RadicalAction::DoubtSeed,
        RadicalAction::EchoStrike,
        RadicalAction::PhaseStrike,
        RadicalAction::ImmovablePeak,
        RadicalAction::FlockAssault,
    ];
    for action in &actions {
        let radical = action.radical();
        let recovered = RadicalAction::from_radical(radical);
        assert_eq!(recovered, Some(*action), "Roundtrip failed for {:?}", action);
    }
}

#[test]
fn radical_action_description_non_empty() {
    assert!(!RadicalAction::SpreadingWildfire.description().is_empty());
    assert!(!RadicalAction::PhaseStrike.description().is_empty());
}

#[test]
fn radical_action_range_info_non_empty() {
    assert!(!RadicalAction::SpreadingWildfire.range_info().is_empty());
    assert!(!RadicalAction::DevouringMaw.range_info().is_empty());
    assert!(!RadicalAction::PursuingSteps.range_info().is_empty());
    assert!(!RadicalAction::RevealingDawn.range_info().is_empty());
}

#[test]
fn radical_action_damage_info_non_empty() {
    assert!(!RadicalAction::SpreadingWildfire.damage_info().is_empty());
    assert!(!RadicalAction::PhaseStrike.damage_info().is_empty());
}

#[test]
fn radical_action_attack_type_covers_all_categories() {
    // Check a representative from each category
    assert_eq!(RadicalAction::RevealingDawn.attack_type(), "Self-buff");
    assert_eq!(RadicalAction::ImperialCommand.attack_type(), "Support");
    assert_eq!(RadicalAction::DevouringMaw.attack_type(), "Melee");
    assert_eq!(RadicalAction::OverwhelmingForce.attack_type(), "Projectile");
    assert_eq!(RadicalAction::ErosiveFlow.attack_type(), "Debuff");
    assert_eq!(RadicalAction::DownpourBarrage.attack_type(), "Arcing (2 turns)");
    assert_eq!(RadicalAction::BoneShatter.attack_type(), "Arcing (1 turn)");
    assert_eq!(RadicalAction::SpreadingWildfire.attack_type(), "AoE");
}

#[test]
fn radical_action_type_color_non_empty() {
    assert!(!RadicalAction::DevouringMaw.type_color().is_empty());
    assert!(!RadicalAction::RevealingDawn.type_color().is_empty());
}

// ── enemy radical_actions ───────────────────────────────────────────────────

#[test]
fn enemy_radical_actions_from_components() {
    let entry = VOCAB.iter().find(|e| e.hanzi == "好").unwrap();
    let enemy = Enemy::from_vocab(entry, 0, 0, 1);
    let actions = enemy.radical_actions();
    assert!(actions.contains(&RadicalAction::MaternalShield)); // 女
    assert!(actions.contains(&RadicalAction::PotentialBurst)); // 子
    assert_eq!(actions.len(), 2); // 1 per radical, 好 has 2 radicals
}

#[test]
fn enemy_with_generated_components_has_actions() {
    let mut chars_with_actions = 0;
    let total = VOCAB.iter().take(100).count();
    for entry in VOCAB.iter().take(100) {
        let enemy = Enemy::from_vocab(entry, 0, 0, 1);
        if !enemy.radical_actions().is_empty() {
            chars_with_actions += 1;
        }
    }
    assert!(
        chars_with_actions > total / 2,
        "Only {}/{} had actions",
        chars_with_actions,
        total
    );
}

// ── PlayerRadicalAbility ────────────────────────────────────────────────────

#[test]
fn player_radical_ability_from_radical_fire() {
    assert_eq!(
        PlayerRadicalAbility::from_radical("火"),
        Some(PlayerRadicalAbility::FireStrike)
    );
}

#[test]
fn player_radical_ability_from_radical_water() {
    assert_eq!(
        PlayerRadicalAbility::from_radical("水"),
        Some(PlayerRadicalAbility::TidalSurge)
    );
}

#[test]
fn player_radical_ability_from_unknown_radical_returns_none() {
    assert_eq!(PlayerRadicalAbility::from_radical("xyz"), None);
}

#[test]
fn player_radical_ability_roundtrips_through_radical() {
    let abilities = [
        PlayerRadicalAbility::FireStrike,
        PlayerRadicalAbility::TidalSurge,
        PlayerRadicalAbility::PowerStrike,
        PlayerRadicalAbility::Insight,
        PlayerRadicalAbility::Devour,
        PlayerRadicalAbility::DoubleStrike,
        PlayerRadicalAbility::Galeforce,
        PlayerRadicalAbility::IronForm,
    ];
    for ability in &abilities {
        let radical = ability.radical();
        let recovered = PlayerRadicalAbility::from_radical(radical);
        assert_eq!(recovered, Some(*ability));
    }
}

#[test]
fn player_radical_ability_name_non_empty() {
    assert!(!PlayerRadicalAbility::FireStrike.name().is_empty());
    assert!(!PlayerRadicalAbility::IronForm.name().is_empty());
    assert!(!PlayerRadicalAbility::Purify.name().is_empty());
}

#[test]
fn player_radical_ability_description_non_empty() {
    assert!(!PlayerRadicalAbility::FireStrike.description().is_empty());
    assert!(!PlayerRadicalAbility::IronForm.description().is_empty());
}

// ── PlayerRadicalAbility::skill_type ────────────────────────────────────────

#[test]
fn skill_type_self_buff_for_insight() {
    assert_eq!(PlayerRadicalAbility::Insight.skill_type(), SkillType::SelfBuff);
}

#[test]
fn skill_type_melee_for_power_strike() {
    assert_eq!(PlayerRadicalAbility::PowerStrike.skill_type(), SkillType::MeleeTarget);
}

#[test]
fn skill_type_ranged_for_fire_strike() {
    assert_eq!(PlayerRadicalAbility::FireStrike.skill_type(), SkillType::RangedTarget(3));
}

#[test]
fn skill_type_ranged_for_snipe_has_longer_range() {
    assert_eq!(PlayerRadicalAbility::Snipe.skill_type(), SkillType::RangedTarget(5));
}

#[test]
fn skill_type_ground_for_earthquake() {
    assert_eq!(PlayerRadicalAbility::Earthquake.skill_type(), SkillType::GroundTarget(3));
}

// ── PlayerRadicalAbility::skill_type_label ──────────────────────────────────

#[test]
fn skill_type_label_self() {
    assert_eq!(PlayerRadicalAbility::Insight.skill_type_label(), "Self");
}

#[test]
fn skill_type_label_melee() {
    assert_eq!(PlayerRadicalAbility::PowerStrike.skill_type_label(), "Melee");
}

#[test]
fn skill_type_label_ranged() {
    assert_eq!(PlayerRadicalAbility::FireStrike.skill_type_label(), "Ranged");
}

#[test]
fn skill_type_label_area() {
    assert_eq!(PlayerRadicalAbility::Earthquake.skill_type_label(), "Area");
}

// ── boss_trait_text ─────────────────────────────────────────────────────────

#[test]
fn boss_trait_text_pirate_captain() {
    let boss = Enemy::boss_from_vocab(friend_entry(), 0, 0, 5);
    let text = boss.boss_trait_text();
    assert!(text.is_some());
    assert!(text.unwrap().contains("shield"));
}

#[test]
fn boss_trait_text_hive_queen_before_phase() {
    let boss = Enemy::boss_from_vocab(friend_entry(), 0, 0, 10);
    let text = boss.boss_trait_text().unwrap();
    assert!(text.contains("sentence duel"));
}

#[test]
fn boss_trait_text_hive_queen_after_phase() {
    let mut boss = Enemy::boss_from_vocab(friend_entry(), 0, 0, 10);
    boss.phase_triggered = true;
    let text = boss.boss_trait_text().unwrap();
    assert!(text.contains("spent"));
}

#[test]
fn boss_trait_text_rogue_ai_no_resist() {
    let boss = Enemy::boss_from_vocab(friend_entry(), 0, 0, 15);
    let text = boss.boss_trait_text().unwrap();
    assert!(text.contains("Adapts"));
}

#[test]
fn boss_trait_text_rogue_ai_with_resist() {
    let mut boss = Enemy::boss_from_vocab(friend_entry(), 0, 0, 15);
    boss.resisted_spell = Some("Fireball");
    let text = boss.boss_trait_text().unwrap();
    assert!(text.contains("Fireball"));
}

#[test]
fn boss_trait_text_none_for_non_boss() {
    let enemy = Enemy::from_vocab(friend_entry(), 0, 0, 1);
    assert!(enemy.boss_trait_text().is_none());
}

// ── elite_phase_count / elite_expected_syllable ─────────────────────────────

#[test]
fn elite_expected_syllable_none_for_non_elite() {
    let entry = VOCAB.iter().find(|e| e.hanzi.chars().count() == 1).unwrap();
    let enemy = Enemy::from_vocab(entry, 0, 0, 1);
    assert_eq!(enemy.elite_expected_syllable(), None);
}

#[test]
fn elite_expected_syllable_first_syllable_at_chain_zero() {
    let mut enemy = Enemy::from_vocab(friend_entry(), 0, 0, 1);
    enemy.elite_chain = 0;
    let syllable = enemy.elite_expected_syllable();
    assert!(syllable.is_some());
    assert_eq!(syllable.unwrap(), "peng2");
}

#[test]
fn elite_phase_count_at_least_one() {
    let enemy = Enemy::from_vocab(friend_entry(), 0, 0, 1);
    assert!(enemy.elite_phase_count() >= 1);
}


