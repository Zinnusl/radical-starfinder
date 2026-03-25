use super::{near_miss_hints, try_forge, SpellEffect, RADICALS, RECIPES};

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

// ── SpellEffect::label covers all variants ────────────────────────

#[test]
fn damage_spell_labels_include_emoji_prefix() {
    assert_eq!(SpellEffect::FireAoe(5).label(), "🔥 Plasma");
    assert_eq!(SpellEffect::StrongHit(3).label(), "⚔ Kinetic Strike");
    assert_eq!(SpellEffect::Drain(2).label(), "🩸 Siphon");
    assert_eq!(SpellEffect::Pierce(4).label(), "🗡 Penetrator");
    assert_eq!(SpellEffect::KnockBack(3).label(), "💨 Repulsor");
    assert_eq!(SpellEffect::Cone(2).label(), "🔺 Arc Blast");
    assert_eq!(SpellEffect::Charge(3).label(), "🐎 Charge");
    assert_eq!(SpellEffect::Blink(2).label(), "⚡ Blink");
}

#[test]
fn defensive_spell_labels_include_emoji_prefix() {
    assert_eq!(SpellEffect::Heal(5).label(), "💚 Nano Repair");
    assert_eq!(SpellEffect::Shield.label(), "🛡 Energy Barrier");
    assert_eq!(SpellEffect::Thorns(3).label(), "🌿 Nanite Cloud");
    assert_eq!(SpellEffect::Wall(4).label(), "🧱 Force Wall");
    assert_eq!(SpellEffect::SummonBoulder.label(), "🪨 Deploy Barrier");
    assert_eq!(SpellEffect::Sanctify(2).label(), "✨ Purify Field");
}

#[test]
fn control_spell_labels_include_emoji_prefix() {
    assert_eq!(SpellEffect::Stun.label(), "⚡ EMP");
    assert_eq!(SpellEffect::Slow(2).label(), "❄ Cryo Beam");
    assert_eq!(SpellEffect::Teleport.label(), "🌀 Phase Shift");
    assert_eq!(SpellEffect::PullToward.label(), "🧲 Tractor Beam");
    assert_eq!(SpellEffect::FreezeGround(3).label(), "❄ Cryo Field");
}

#[test]
fn misc_spell_labels_include_emoji_prefix() {
    assert_eq!(SpellEffect::Poison(2, 3).label(), "☠ Corrosion");
    assert_eq!(SpellEffect::FocusRestore(3).label(), "🎯 Recalibrate");
    assert_eq!(SpellEffect::ArmorBreak.label(), "💥 Shield Breach");
    assert_eq!(SpellEffect::Dash(3).label(), "🏃 Boost");
    assert_eq!(SpellEffect::OilSlick.label(), "🛢 Lubricant");
    assert_eq!(SpellEffect::Ignite.label(), "🔥 Plasma Ignition");
    assert_eq!(SpellEffect::PlantGrowth.label(), "🌱 Nanite Growth");
    assert_eq!(SpellEffect::Earthquake(5).label(), "💎 Seismic Charge");
    assert_eq!(SpellEffect::FloodWave(4).label(), "🌊 Coolant Wave");
}

// ── SpellEffect::description ──────────────────────────────────────

#[test]
fn heal_description_includes_amount() {
    let desc = SpellEffect::Heal(8).description();
    assert!(desc.contains("8"));
}

#[test]
fn fire_aoe_description_includes_damage() {
    let desc = SpellEffect::FireAoe(5).description();
    assert!(desc.contains("5"));
}

#[test]
fn poison_description_includes_damage_and_turns() {
    let desc = SpellEffect::Poison(3, 4).description();
    assert!(desc.contains("3") && desc.contains("4"));
}

#[test]
fn reveal_description_mentions_scan() {
    let desc = SpellEffect::Reveal.description();
    assert!(desc.to_lowercase().contains("scan"));
}

#[test]
fn shield_description_mentions_barrier() {
    let desc = SpellEffect::Shield.description();
    assert!(desc.to_lowercase().contains("barrier"));
}

// ── Recipe forging ────────────────────────────────────────────────

#[test]
fn forge_hao_from_nv_zi_produces_heal() {
    let recipe = try_forge(&["女", "子"]).unwrap();
    assert_eq!(recipe.output_hanzi, "好");
    assert!(matches!(recipe.effect, SpellEffect::Heal(_)));
}

#[test]
fn forge_is_order_independent() {
    let a = try_forge(&["女", "子"]);
    let b = try_forge(&["子", "女"]);
    assert_eq!(a.map(|r| r.output_hanzi), b.map(|r| r.output_hanzi));
}

#[test]
fn forge_with_wrong_radicals_returns_none() {
    assert!(try_forge(&["火", "火"]).is_none());
}

#[test]
fn forge_with_too_few_radicals_returns_none() {
    assert!(try_forge(&["火"]).is_none());
}

// ── RADICALS data integrity ───────────────────────────────────────

#[test]
fn all_radicals_have_non_empty_fields() {
    for r in RADICALS {
        assert!(!r.ch.is_empty(), "radical ch is empty");
        assert!(!r.name.is_empty(), "radical name is empty for {}", r.ch);
        assert!(!r.meaning.is_empty(), "radical meaning is empty for {}", r.ch);
    }
}

#[test]
fn radical_chars_are_unique() {
    let mut chars: Vec<&str> = RADICALS.iter().map(|r| r.ch).collect();
    let len = chars.len();
    chars.sort();
    chars.dedup();
    assert_eq!(chars.len(), len, "duplicate radical chars found");
}

// ── RECIPES data integrity ────────────────────────────────────────

#[test]
fn all_recipes_have_at_least_two_inputs() {
    for recipe in RECIPES {
        assert!(
            recipe.inputs.len() >= 2,
            "recipe {} has fewer than 2 inputs",
            recipe.output_hanzi
        );
    }
}

#[test]
fn all_recipe_outputs_have_non_empty_fields() {
    for recipe in RECIPES {
        assert!(!recipe.output_hanzi.is_empty());
        assert!(!recipe.output_pinyin.is_empty());
        assert!(!recipe.output_meaning.is_empty());
    }
}
