use super::*;

#[test]
fn fresh_status_skips_first_tick() {
    let mut statuses = vec![StatusInstance::new(StatusKind::Regen { heal: 2 }, 3)];

    // First tick: fresh → skip auto-repair
    let (damage, heal) = tick_statuses(&mut statuses);
    assert_eq!(damage, 0);
    assert_eq!(heal, 0);
    assert_eq!(statuses.len(), 1);

    // Second tick: auto-repair applies
    let (damage, heal) = tick_statuses(&mut statuses);
    assert_eq!(damage, 0);
    assert_eq!(heal, 2);
}

#[test]
fn final_autorepair_tick_removes_the_status() {
    let mut statuses = vec![StatusInstance::new(StatusKind::Regen { heal: 1 }, 1)];

    let _ = tick_statuses(&mut statuses);

    assert!(statuses.is_empty());
}

#[test]
fn revealed_status_is_detected() {
    let statuses = vec![StatusInstance::new(StatusKind::Revealed, 4)];

    assert!(has_revealed(&statuses));
}

#[test]
fn plasma_burn_does_not_deal_instant_damage() {
    let mut statuses = vec![StatusInstance::new(StatusKind::Burn { damage: 1 }, 2)];

    // First tick: fresh → no damage
    let (damage, _) = tick_statuses(&mut statuses);
    assert_eq!(damage, 0);

    // Second tick: plasma burn damage applies
    let (damage, _) = tick_statuses(&mut statuses);
    assert_eq!(damage, 1);
    assert!(statuses.is_empty());
}

// --------------- StatusInstance::new ---------------

#[test]
fn new_status_starts_fresh() {
    let status = StatusInstance::new(StatusKind::Haste, 3);
    assert!(status.fresh);
}

#[test]
fn new_status_has_correct_turns_left() {
    let status = StatusInstance::new(StatusKind::Slow, 5);
    assert_eq!(status.turns_left, 5);
}

// --------------- label ---------------

#[test]
fn poison_label_contains_rad() {
    let s = StatusInstance::new(StatusKind::Poison { damage: 1 }, 3);
    assert_eq!(s.label(), "☢Rad");
}

#[test]
fn haste_label_contains_ovr() {
    let s = StatusInstance::new(StatusKind::Haste, 2);
    assert_eq!(s.label(), "⚡Ovr");
}

#[test]
fn cursed_label_contains_mlw() {
    let s = StatusInstance::new(StatusKind::Cursed, 2);
    assert_eq!(s.label(), "💀Mlw");
}

#[test]
fn shield_label_contains_shd() {
    let s = StatusInstance::new(StatusKind::Shield, 2);
    assert_eq!(s.label(), "🛡Shd");
}

// --------------- color ---------------

#[test]
fn poison_color_is_green() {
    let s = StatusInstance::new(StatusKind::Poison { damage: 1 }, 1);
    assert_eq!(s.color(), "#88ff44");
}

#[test]
fn burn_color_is_orange() {
    let s = StatusInstance::new(StatusKind::Burn { damage: 1 }, 1);
    assert_eq!(s.color(), "#ff5500");
}

#[test]
fn freeze_color_is_cyan() {
    let s = StatusInstance::new(StatusKind::Freeze, 1);
    assert_eq!(s.color(), "#00ffff");
}

// --------------- is_negative ---------------

#[test]
fn poison_is_negative() {
    let s = StatusInstance::new(StatusKind::Poison { damage: 1 }, 1);
    assert!(s.is_negative());
}

#[test]
fn burn_is_negative() {
    let s = StatusInstance::new(StatusKind::Burn { damage: 2 }, 1);
    assert!(s.is_negative());
}

#[test]
fn bleed_is_negative() {
    let s = StatusInstance::new(StatusKind::Bleed { damage: 1 }, 1);
    assert!(s.is_negative());
}

#[test]
fn confused_is_negative() {
    let s = StatusInstance::new(StatusKind::Confused, 1);
    assert!(s.is_negative());
}

#[test]
fn freeze_is_negative() {
    let s = StatusInstance::new(StatusKind::Freeze, 1);
    assert!(s.is_negative());
}

#[test]
fn slow_is_negative() {
    let s = StatusInstance::new(StatusKind::Slow, 1);
    assert!(s.is_negative());
}

#[test]
fn rooted_is_negative() {
    let s = StatusInstance::new(StatusKind::Rooted, 1);
    assert!(s.is_negative());
}

#[test]
fn weakened_is_negative() {
    let s = StatusInstance::new(StatusKind::Weakened, 1);
    assert!(s.is_negative());
}

#[test]
fn cursed_is_negative() {
    let s = StatusInstance::new(StatusKind::Cursed, 1);
    assert!(s.is_negative());
}

#[test]
fn regen_is_not_negative() {
    let s = StatusInstance::new(StatusKind::Regen { heal: 1 }, 1);
    assert!(!s.is_negative());
}

#[test]
fn haste_is_not_negative() {
    let s = StatusInstance::new(StatusKind::Haste, 1);
    assert!(!s.is_negative());
}

#[test]
fn blessed_is_not_negative() {
    let s = StatusInstance::new(StatusKind::Blessed, 1);
    assert!(!s.is_negative());
}

#[test]
fn invisible_is_not_negative() {
    let s = StatusInstance::new(StatusKind::Invisible, 1);
    assert!(!s.is_negative());
}

#[test]
fn shield_is_not_negative() {
    let s = StatusInstance::new(StatusKind::Shield, 1);
    assert!(!s.is_negative());
}

// --------------- tick_statuses ---------------

#[test]
fn tick_empty_list_returns_zero_damage_and_heal() {
    let mut statuses: Vec<StatusInstance> = vec![];
    let (damage, heal) = tick_statuses(&mut statuses);
    assert_eq!(damage, 0);
    assert_eq!(heal, 0);
}

#[test]
fn tick_poison_deals_damage_after_fresh_turn() {
    let mut statuses = vec![StatusInstance::new(StatusKind::Poison { damage: 3 }, 3)];
    tick_statuses(&mut statuses); // fresh: skip damage
    let (damage, _) = tick_statuses(&mut statuses);
    assert_eq!(damage, 3);
}

#[test]
fn tick_stacking_poisons_sum_damage() {
    let mut statuses = vec![
        StatusInstance::new(StatusKind::Poison { damage: 2 }, 3),
        StatusInstance::new(StatusKind::Poison { damage: 3 }, 3),
    ];
    tick_statuses(&mut statuses); // fresh: skip
    let (damage, _) = tick_statuses(&mut statuses);
    assert_eq!(damage, 5);
}

#[test]
fn tick_mixed_damage_and_heal() {
    let mut statuses = vec![
        StatusInstance::new(StatusKind::Poison { damage: 4 }, 3),
        StatusInstance::new(StatusKind::Regen { heal: 2 }, 3),
    ];
    tick_statuses(&mut statuses); // fresh: skip
    let (damage, heal) = tick_statuses(&mut statuses);
    assert_eq!(damage, 4);
    assert_eq!(heal, 2);
}

#[test]
fn tick_removes_expired_statuses() {
    let mut statuses = vec![
        StatusInstance::new(StatusKind::Haste, 1),
        StatusInstance::new(StatusKind::Slow, 3),
    ];
    tick_statuses(&mut statuses);
    assert_eq!(statuses.len(), 1);
}

#[test]
fn tick_cursed_deals_one_damage_after_fresh() {
    let mut statuses = vec![StatusInstance::new(StatusKind::Cursed, 3)];
    tick_statuses(&mut statuses); // fresh: skip
    let (damage, _) = tick_statuses(&mut statuses);
    assert_eq!(damage, 1);
}

#[test]
fn tick_bleed_deals_damage_after_fresh() {
    let mut statuses = vec![StatusInstance::new(StatusKind::Bleed { damage: 5 }, 3)];
    tick_statuses(&mut statuses); // fresh: skip
    let (damage, _) = tick_statuses(&mut statuses);
    assert_eq!(damage, 5);
}

#[test]
fn tick_haste_produces_no_damage_or_heal() {
    let mut statuses = vec![StatusInstance::new(StatusKind::Haste, 3)];
    tick_statuses(&mut statuses); // fresh
    let (damage, heal) = tick_statuses(&mut statuses);
    assert_eq!(damage, 0);
    assert_eq!(heal, 0);
}

// --------------- has_* presence checks ---------------

#[test]
fn has_haste_returns_true_when_present() {
    let statuses = vec![StatusInstance::new(StatusKind::Haste, 2)];
    assert!(has_haste(&statuses));
}

#[test]
fn has_haste_returns_false_when_absent() {
    let statuses = vec![StatusInstance::new(StatusKind::Slow, 2)];
    assert!(!has_haste(&statuses));
}

#[test]
fn has_haste_returns_false_for_empty_list() {
    let statuses: Vec<StatusInstance> = vec![];
    assert!(!has_haste(&statuses));
}

#[test]
fn has_confused_returns_true_when_present() {
    let statuses = vec![StatusInstance::new(StatusKind::Confused, 2)];
    assert!(has_confused(&statuses));
}

#[test]
fn has_confused_returns_false_when_absent() {
    let statuses = vec![StatusInstance::new(StatusKind::Haste, 2)];
    assert!(!has_confused(&statuses));
}

#[test]
fn has_rooted_returns_true_when_present() {
    let statuses = vec![StatusInstance::new(StatusKind::Rooted, 2)];
    assert!(has_rooted(&statuses));
}

#[test]
fn has_cursed_returns_true_when_present() {
    let statuses = vec![StatusInstance::new(StatusKind::Cursed, 2)];
    assert!(has_cursed(&statuses));
}

#[test]
fn has_cursed_returns_false_when_absent() {
    let statuses: Vec<StatusInstance> = vec![];
    assert!(!has_cursed(&statuses));
}

#[test]
fn has_invisible_returns_true_when_present() {
    let statuses = vec![StatusInstance::new(StatusKind::Invisible, 2)];
    assert!(has_invisible(&statuses));
}

#[test]
fn has_envenomed_returns_true_when_present() {
    let statuses = vec![StatusInstance::new(StatusKind::Envenomed, 2)];
    assert!(has_envenomed(&statuses));
}

#[test]
fn has_weakened_returns_true_when_present() {
    let statuses = vec![StatusInstance::new(StatusKind::Weakened, 2)];
    assert!(has_weakened(&statuses));
}

#[test]
fn has_blessed_returns_true_when_present() {
    let statuses = vec![StatusInstance::new(StatusKind::Blessed, 2)];
    assert!(has_blessed(&statuses));
}

#[test]
fn has_wet_returns_true_when_present() {
    let statuses = vec![StatusInstance::new(StatusKind::Wet, 2)];
    assert!(has_wet(&statuses));
}

#[test]
fn has_burn_returns_true_when_present() {
    let statuses = vec![StatusInstance::new(StatusKind::Burn { damage: 1 }, 2)];
    assert!(has_burn(&statuses));
}

#[test]
fn has_freeze_returns_true_when_present() {
    let statuses = vec![StatusInstance::new(StatusKind::Freeze, 2)];
    assert!(has_freeze(&statuses));
}

#[test]
fn has_poison_returns_true_when_present() {
    let statuses = vec![StatusInstance::new(StatusKind::Poison { damage: 1 }, 2)];
    assert!(has_poison(&statuses));
}

#[test]
fn has_slow_returns_true_when_present() {
    let statuses = vec![StatusInstance::new(StatusKind::Slow, 2)];
    assert!(has_slow(&statuses));
}

#[test]
fn has_fortify_returns_true_when_present() {
    let statuses = vec![StatusInstance::new(StatusKind::Fortify { stacks: 2 }, 3)];
    assert!(has_fortify(&statuses));
}

// --------------- empowered_amount / fortify_stacks ---------------

#[test]
fn empowered_amount_sums_multiple_stacks() {
    let statuses = vec![
        StatusInstance::new(StatusKind::Empowered { amount: 3 }, 2),
        StatusInstance::new(StatusKind::Empowered { amount: 2 }, 2),
    ];
    assert_eq!(empowered_amount(&statuses), 5);
}

#[test]
fn empowered_amount_returns_zero_when_absent() {
    let statuses = vec![StatusInstance::new(StatusKind::Haste, 2)];
    assert_eq!(empowered_amount(&statuses), 0);
}

#[test]
fn fortify_stacks_sums_multiple_instances() {
    let statuses = vec![
        StatusInstance::new(StatusKind::Fortify { stacks: 1 }, 2),
        StatusInstance::new(StatusKind::Fortify { stacks: 3 }, 2),
    ];
    assert_eq!(fortify_stacks(&statuses), 4);
}

#[test]
fn fortify_stacks_returns_zero_when_absent() {
    let statuses: Vec<StatusInstance> = vec![];
    assert_eq!(fortify_stacks(&statuses), 0);
}

// --------------- has_status generic ---------------

#[test]
fn has_status_matches_by_label_substring() {
    let statuses = vec![StatusInstance::new(StatusKind::Haste, 2)];
    assert!(has_status(&statuses, "Ovr"));
}

#[test]
fn has_status_returns_false_for_unmatched_substring() {
    let statuses = vec![StatusInstance::new(StatusKind::Haste, 2)];
    assert!(!has_status(&statuses, "Rad"));
}

