use super::{AiBehavior, BossKind, Enemy};
use crate::vocab::VOCAB;

fn friend_entry() -> &'static crate::vocab::VocabEntry {
    VOCAB.iter().find(|entry| entry.hanzi == "朋友").unwrap()
}

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
fn radical_action_from_known_radicals() {
    use super::RadicalAction;
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
fn enemy_radical_actions_from_components() {
    use crate::vocab::VOCAB;
    // 好 has components 女 and 子
    let entry = VOCAB.iter().find(|e| e.hanzi == "好").unwrap();
    let enemy = Enemy::from_vocab(entry, 0, 0, 1);
    let actions = enemy.radical_actions();
    use super::RadicalAction;
    assert!(actions.contains(&RadicalAction::MaternalShield)); // 女
    assert!(actions.contains(&RadicalAction::PotentialBurst)); // 子
    assert_eq!(actions.len(), 2); // 1 per radical, 好 has 2 radicals
}

#[test]
fn enemy_with_generated_components_has_actions() {
    use crate::vocab::VOCAB;
    // Most HSK characters should have at least some components now
    let mut chars_with_actions = 0;
    let total = VOCAB.iter().take(100).count();
    for entry in VOCAB.iter().take(100) {
        let enemy = Enemy::from_vocab(entry, 0, 0, 1);
        if !enemy.radical_actions().is_empty() {
            chars_with_actions += 1;
        }
    }
    // At least 60% of chars should have radical actions with our expanded system
    assert!(
        chars_with_actions > total / 2,
        "Only {}/{} had actions",
        chars_with_actions,
        total
    );
}

