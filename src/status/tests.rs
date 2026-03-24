use super::{has_revealed, tick_statuses, StatusInstance, StatusKind};

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

