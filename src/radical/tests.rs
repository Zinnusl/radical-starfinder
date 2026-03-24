use super::{near_miss_hints, try_forge, SpellEffect};

#[test]
fn utility_spell_labels_are_stable() {
    assert_eq!(SpellEffect::Reveal.label(), "👁 Sensor Scan");
    assert_eq!(SpellEffect::Pacify.label(), "☯ Override");
}

#[test]
fn verified_utility_recipes_map_to_new_effects() {
    assert!(matches!(
        try_forge(&["日", "月"]).map(|recipe| recipe.effect),
        Some(SpellEffect::Reveal)
    ));
    assert!(matches!(
        try_forge(&["王", "田", "土"]).map(|recipe| recipe.effect),
        Some(SpellEffect::Pacify)
    ));
}

#[test]
fn near_miss_hints_finds_one_missing_radical() {
    let hints = near_miss_hints(&["女"]);
    assert!(hints.len() >= 1);
    let hint = hints.iter().find(|h| h.contains("好")).unwrap();
    assert!(hint.contains("子"));
}

#[test]
fn near_miss_hints_empty_for_no_close_match() {
    let empty_hints = near_miss_hints(&[]);
    assert!(empty_hints.is_empty());
}

