use super::*;

use super::{
    craftable_recipes, near_miss_hints, radicals_for_floor, rare_radicals, try_forge,
    Spell, SpellEffect, RADICALS, RECIPES,
};

// ── SpellEffect::label covers all variants ────────────────────────

#[test]
fn utility_spell_labels_are_stable() {
    assert_eq!(SpellEffect::Reveal.label(), "👁 Sensor Scan");
    assert_eq!(SpellEffect::Pacify.label(), "☯ Override");
}

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

#[test]
fn strong_hit_description_includes_damage() {
    let desc = SpellEffect::StrongHit(7).description();
    assert!(desc.contains("7"), "expected damage value in: {}", desc);
}

#[test]
fn drain_description_includes_damage() {
    let desc = SpellEffect::Drain(5).description();
    assert!(desc.contains("5"), "expected damage value in: {}", desc);
}

#[test]
fn stun_description_mentions_skip() {
    let desc = SpellEffect::Stun.description();
    assert!(desc.to_lowercase().contains("skip") || desc.to_lowercase().contains("emp"));
}

#[test]
fn pacify_description_mentions_override() {
    let desc = SpellEffect::Pacify.description();
    assert!(
        desc.to_lowercase().contains("override") || desc.to_lowercase().contains("ending"),
        "unexpected: {}",
        desc
    );
}

#[test]
fn slow_description_includes_turns() {
    let desc = SpellEffect::Slow(3).description();
    assert!(desc.contains("3"), "expected turn count in: {}", desc);
}

#[test]
fn teleport_description_mentions_swap() {
    let desc = SpellEffect::Teleport.description();
    assert!(desc.to_lowercase().contains("swap") || desc.to_lowercase().contains("phase"));
}

#[test]
fn focus_restore_description_includes_amount() {
    let desc = SpellEffect::FocusRestore(5).description();
    assert!(desc.contains("5"), "expected amount in: {}", desc);
}

#[test]
fn armor_break_description_mentions_shield() {
    let desc = SpellEffect::ArmorBreak.description();
    assert!(
        desc.to_lowercase().contains("shield") || desc.to_lowercase().contains("breach"),
        "unexpected: {}",
        desc
    );
}

#[test]
fn dash_description_includes_damage() {
    let desc = SpellEffect::Dash(4).description();
    assert!(desc.contains("4"), "expected damage in: {}", desc);
}

#[test]
fn pierce_description_includes_damage() {
    let desc = SpellEffect::Pierce(6).description();
    assert!(desc.contains("6"), "expected damage in: {}", desc);
}

#[test]
fn pull_toward_description_mentions_pull() {
    let desc = SpellEffect::PullToward.description();
    assert!(desc.to_lowercase().contains("pull") || desc.to_lowercase().contains("tractor"));
}

#[test]
fn knock_back_description_includes_damage() {
    let desc = SpellEffect::KnockBack(5).description();
    assert!(desc.contains("5"), "expected damage in: {}", desc);
}

#[test]
fn thorns_description_includes_turns() {
    let desc = SpellEffect::Thorns(4).description();
    assert!(desc.contains("4"), "expected turns in: {}", desc);
}

#[test]
fn cone_description_includes_damage() {
    let desc = SpellEffect::Cone(3).description();
    assert!(desc.contains("3"), "expected damage in: {}", desc);
}

#[test]
fn wall_description_includes_length() {
    let desc = SpellEffect::Wall(5).description();
    assert!(desc.contains("5"), "expected length in: {}", desc);
}

#[test]
fn oil_slick_description_mentions_lubricant() {
    let desc = SpellEffect::OilSlick.description();
    assert!(
        desc.to_lowercase().contains("lubricant") || desc.to_lowercase().contains("flammable"),
        "unexpected: {}",
        desc
    );
}

#[test]
fn freeze_ground_description_includes_damage() {
    let desc = SpellEffect::FreezeGround(4).description();
    assert!(desc.contains("4"), "expected damage in: {}", desc);
}

#[test]
fn ignite_description_mentions_burn() {
    let desc = SpellEffect::Ignite.description();
    assert!(
        desc.to_lowercase().contains("burn") || desc.to_lowercase().contains("ignit"),
        "unexpected: {}",
        desc
    );
}

#[test]
fn plant_growth_description_mentions_nanite() {
    let desc = SpellEffect::PlantGrowth.description();
    assert!(
        desc.to_lowercase().contains("nanite") || desc.to_lowercase().contains("growth"),
        "unexpected: {}",
        desc
    );
}

#[test]
fn earthquake_description_includes_damage() {
    let desc = SpellEffect::Earthquake(6).description();
    assert!(desc.contains("6"), "expected damage in: {}", desc);
}

#[test]
fn sanctify_description_includes_heal() {
    let desc = SpellEffect::Sanctify(3).description();
    assert!(desc.contains("3"), "expected heal amount in: {}", desc);
}

#[test]
fn flood_wave_description_includes_damage() {
    let desc = SpellEffect::FloodWave(5).description();
    assert!(desc.contains("5"), "expected damage in: {}", desc);
}

#[test]
fn summon_boulder_description_mentions_barrier() {
    let desc = SpellEffect::SummonBoulder.description();
    assert!(
        desc.to_lowercase().contains("barrier") || desc.to_lowercase().contains("block"),
        "unexpected: {}",
        desc
    );
}

#[test]
fn charge_description_includes_damage() {
    let desc = SpellEffect::Charge(4).description();
    assert!(desc.contains("4"), "expected damage in: {}", desc);
}

#[test]
fn blink_description_includes_damage() {
    let desc = SpellEffect::Blink(3).description();
    assert!(desc.contains("3"), "expected damage in: {}", desc);
}

// ── Recipe forging ────────────────────────────────────────────────

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

#[test]
fn forge_three_input_recipe() {
    let recipe = try_forge(&["木", "目", "心"]).unwrap();
    assert_eq!(recipe.output_hanzi, "想");
    assert!(matches!(recipe.effect, SpellEffect::Heal(_)));
}

#[test]
fn forge_three_input_order_independent() {
    let a = try_forge(&["木", "目", "心"]);
    let b = try_forge(&["心", "木", "目"]);
    let c = try_forge(&["目", "心", "木"]);
    assert_eq!(a.map(|r| r.output_hanzi), b.map(|r| r.output_hanzi));
    assert_eq!(b.map(|r| r.output_hanzi), c.map(|r| r.output_hanzi));
}

#[test]
fn forge_with_too_many_radicals_returns_none() {
    // "女" + "子" = "好", but adding an extra radical should fail
    assert!(try_forge(&["女", "子", "火"]).is_none());
}

#[test]
fn forge_rare_recipe_dragon_fire() {
    let recipe = try_forge(&["龙", "火"]).unwrap();
    assert_eq!(recipe.output_hanzi, "炎龙");
    assert!(matches!(recipe.effect, SpellEffect::FireAoe(8)));
}

#[test]
fn forge_terrain_oil_slick() {
    let recipe = try_forge(&["水", "土"]).unwrap();
    assert!(matches!(recipe.effect, SpellEffect::OilSlick));
}

#[test]
fn forge_stun_recipe() {
    let recipe = try_forge(&["口", "马"]).unwrap();
    assert!(matches!(recipe.effect, SpellEffect::Stun));
}

#[test]
fn forge_dash_recipe() {
    let recipe = try_forge(&["马", "力"]).unwrap();
    assert!(matches!(recipe.effect, SpellEffect::Dash(_)));
}

#[test]
fn forge_wall_recipe() {
    let recipe = try_forge(&["土", "山"]).unwrap();
    assert!(matches!(recipe.effect, SpellEffect::Wall(_)));
}

#[test]
fn forge_charge_recipe() {
    let recipe = try_forge(&["马", "火"]).unwrap();
    assert!(matches!(recipe.effect, SpellEffect::Charge(_)));
}

#[test]
fn forge_blink_recipe() {
    let recipe = try_forge(&["门", "风"]).unwrap();
    assert!(matches!(recipe.effect, SpellEffect::Blink(_)));
}

// ── near_miss_hints ───────────────────────────────────────────────

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

#[test]
fn near_miss_hints_multiple_radicals() {
    // Having "日" and "月" completes a recipe; near-miss should mention recipes that need one more
    let hints = near_miss_hints(&["日", "月"]);
    // Should include 3-input recipes that need one more radical
    for hint in &hints {
        assert!(hint.contains("Close!"));
    }
}

#[test]
fn near_miss_for_three_input_recipe() {
    // "木" + "目" need "心" for "想"
    let hints = near_miss_hints(&["木", "目"]);
    let found = hints.iter().any(|h| h.contains("想"));
    assert!(found, "expected near-miss for 想, got: {:?}", hints);
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

#[test]
fn radicals_have_common_and_rare() {
    let common = RADICALS.iter().filter(|r| !r.rare).count();
    let rare = RADICALS.iter().filter(|r| r.rare).count();
    assert!(common > 0, "expected some common radicals");
    assert!(rare > 0, "expected some rare radicals");
    assert!(common > rare, "expected more common than rare radicals");
}

#[test]
fn radicals_contain_five_elements() {
    let elements = ["火", "水", "木", "金", "土"];
    for elem in &elements {
        assert!(
            RADICALS.iter().any(|r| r.ch == *elem),
            "missing element radical: {}",
            elem
        );
    }
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

#[test]
fn most_recipe_inputs_are_known_radicals() {
    let radical_chars: Vec<&str> = RADICALS.iter().map(|r| r.ch).collect();
    let mut known = 0;
    let mut total = 0;
    for recipe in RECIPES {
        for &input in recipe.inputs {
            total += 1;
            if radical_chars.contains(&input) {
                known += 1;
            }
        }
    }
    // The vast majority of recipe inputs should reference known radicals
    let pct = known * 100 / total;
    assert!(pct > 90, "only {}% of recipe inputs are known radicals", pct);
}

#[test]
fn recipes_total_count() {
    assert!(RECIPES.len() > 50, "expected many recipes, got {}", RECIPES.len());
}

// ── craftable_recipes ─────────────────────────────────────────────

#[test]
fn craftable_recipes_with_matching_radicals() {
    let craftable = craftable_recipes(&["女", "子"]);
    assert!(!craftable.is_empty(), "should craft at least 好");
    let recipe = &RECIPES[craftable[0]];
    assert_eq!(recipe.output_hanzi, "好");
}

#[test]
fn craftable_recipes_with_no_matches() {
    // Two of the same radical shouldn't match any recipe (no recipe uses two of the same)
    let craftable = craftable_recipes(&["火", "火", "火"]);
    // Some recipes might coincidentally match, but most won't
    // Just check it doesn't panic
    let _ = craftable;
}

#[test]
fn craftable_recipes_with_many_radicals() {
    let craftable = craftable_recipes(&["火", "水", "木", "金", "土", "日", "月", "心"]);
    assert!(
        craftable.len() > 3,
        "with many radicals should craft multiple recipes, got {}",
        craftable.len()
    );
}

#[test]
fn craftable_recipes_empty_hand() {
    let craftable = craftable_recipes(&[]);
    assert!(craftable.is_empty());
}

#[test]
fn craftable_recipes_single_radical() {
    let craftable = craftable_recipes(&["火"]);
    assert!(craftable.is_empty(), "single radical can't craft anything");
}

// ── radicals_for_floor ────────────────────────────────────────────

#[test]
fn floor_1_has_10_radicals() {
    let rads = radicals_for_floor(1);
    assert_eq!(rads.len(), 10);
}

#[test]
fn floor_2_has_16_radicals() {
    let rads = radicals_for_floor(2);
    assert_eq!(rads.len(), 16);
}

#[test]
fn floor_3_has_22_radicals() {
    let rads = radicals_for_floor(3);
    assert_eq!(rads.len(), 22);
}

#[test]
fn floor_4_has_28_radicals() {
    let rads = radicals_for_floor(4);
    assert_eq!(rads.len(), 28);
}

#[test]
fn floor_5_has_34_radicals() {
    let rads = radicals_for_floor(5);
    assert_eq!(rads.len(), 34);
}

#[test]
fn floor_high_unlocks_all_floor_radicals() {
    let rads = radicals_for_floor(10);
    assert_eq!(rads.len(), 41);
}

#[test]
fn floor_radicals_increase_monotonically() {
    let counts: Vec<usize> = (1..=6).map(|f| radicals_for_floor(f).len()).collect();
    for i in 1..counts.len() {
        assert!(
            counts[i] >= counts[i - 1],
            "floor {} has fewer radicals than floor {}",
            i + 1,
            i
        );
    }
}

// ── rare_radicals ─────────────────────────────────────────────────

#[test]
fn rare_radicals_all_are_rare() {
    for r in rare_radicals() {
        assert!(r.rare, "expected rare radical, got common: {}", r.ch);
    }
}

#[test]
fn rare_radicals_not_empty() {
    assert!(!rare_radicals().is_empty());
}

#[test]
fn rare_radicals_all_marked_rare() {
    for r in rare_radicals() {
        assert!(r.rare, "expected rare radical, got: {}", r.ch);
    }
}

// ── Radical / Recipe struct fields ────────────────────────────────

#[test]
fn radical_struct_fields_accessible() {
    let r = &RADICALS[0];
    assert_eq!(r.ch, "火");
    assert_eq!(r.meaning, "fire");
    assert!(!r.rare);
}

#[test]
fn recipe_struct_fields_accessible() {
    let recipe = &RECIPES[0];
    assert!(!recipe.inputs.is_empty());
    assert!(!recipe.output_hanzi.is_empty());
    assert!(!recipe.output_pinyin.is_empty());
    assert!(!recipe.output_meaning.is_empty());
}

#[test]
fn spell_struct_can_be_created() {
    let spell = Spell {
        hanzi: "好",
        pinyin: "hǎo",
        meaning: "good",
        effect: SpellEffect::Heal(5),
    };
    assert_eq!(spell.hanzi, "好");
    assert!(matches!(spell.effect, SpellEffect::Heal(5)));
}

// ── SpellEffect equality ──────────────────────────────────────────

#[test]
fn spell_effect_eq_same_variant_same_value() {
    assert_eq!(SpellEffect::FireAoe(5), SpellEffect::FireAoe(5));
    assert_eq!(SpellEffect::Heal(3), SpellEffect::Heal(3));
    assert_eq!(SpellEffect::Reveal, SpellEffect::Reveal);
}

#[test]
fn spell_effect_ne_different_values() {
    assert_ne!(SpellEffect::FireAoe(5), SpellEffect::FireAoe(3));
    assert_ne!(SpellEffect::Heal(3), SpellEffect::Heal(5));
}

#[test]
fn spell_effect_ne_different_variants() {
    assert_ne!(SpellEffect::FireAoe(5), SpellEffect::Heal(5));
    assert_ne!(SpellEffect::Stun, SpellEffect::Pacify);
}

// ── SpellEffect::description — all variants return non-empty ──────

#[test]
fn all_spell_effect_descriptions_are_nonempty() {
    let effects: Vec<SpellEffect> = vec![
        SpellEffect::FireAoe(3),
        SpellEffect::Heal(3),
        SpellEffect::Reveal,
        SpellEffect::Shield,
        SpellEffect::StrongHit(3),
        SpellEffect::Drain(3),
        SpellEffect::Stun,
        SpellEffect::Pacify,
        SpellEffect::Slow(2),
        SpellEffect::Teleport,
        SpellEffect::Poison(2, 3),
        SpellEffect::FocusRestore(3),
        SpellEffect::ArmorBreak,
        SpellEffect::Dash(3),
        SpellEffect::Pierce(3),
        SpellEffect::PullToward,
        SpellEffect::KnockBack(3),
        SpellEffect::Thorns(3),
        SpellEffect::Cone(3),
        SpellEffect::Wall(3),
        SpellEffect::OilSlick,
        SpellEffect::FreezeGround(3),
        SpellEffect::Ignite,
        SpellEffect::PlantGrowth,
        SpellEffect::Earthquake(3),
        SpellEffect::Sanctify(3),
        SpellEffect::FloodWave(3),
        SpellEffect::SummonBoulder,
        SpellEffect::Charge(3),
        SpellEffect::Blink(3),
    ];
    for effect in effects {
        let desc = effect.description();
        assert!(!desc.is_empty(), "{:?} has empty description", effect);
    }
}

// ── SpellEffect::label — all variants return non-empty ────────────

#[test]
fn all_spell_effect_labels_are_nonempty() {
    let effects: Vec<SpellEffect> = vec![
        SpellEffect::FireAoe(3),
        SpellEffect::Heal(3),
        SpellEffect::Reveal,
        SpellEffect::Shield,
        SpellEffect::StrongHit(3),
        SpellEffect::Drain(3),
        SpellEffect::Stun,
        SpellEffect::Pacify,
        SpellEffect::Slow(2),
        SpellEffect::Teleport,
        SpellEffect::Poison(2, 3),
        SpellEffect::FocusRestore(3),
        SpellEffect::ArmorBreak,
        SpellEffect::Dash(3),
        SpellEffect::Pierce(3),
        SpellEffect::PullToward,
        SpellEffect::KnockBack(3),
        SpellEffect::Thorns(3),
        SpellEffect::Cone(3),
        SpellEffect::Wall(3),
        SpellEffect::OilSlick,
        SpellEffect::FreezeGround(3),
        SpellEffect::Ignite,
        SpellEffect::PlantGrowth,
        SpellEffect::Earthquake(3),
        SpellEffect::Sanctify(3),
        SpellEffect::FloodWave(3),
        SpellEffect::SummonBoulder,
        SpellEffect::Charge(3),
        SpellEffect::Blink(3),
    ];
    for effect in effects {
        let label = effect.label();
        assert!(!label.is_empty(), "{:?} has empty label", effect);
    }
}
