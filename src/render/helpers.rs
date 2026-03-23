//! Sprite-key lookups, color helpers, and text utilities.

use std::collections::BTreeMap;

use crate::enemy::BossKind;
use crate::game::ShopItemKind;
use crate::player::{Item, ItemKind, ItemState};
use crate::radical;

pub(super) fn boss_sprite_key(kind: BossKind) -> &'static str {
    match kind {
        BossKind::PirateCaptain => "boss_pirate_captain",
        BossKind::HiveQueen => "boss_hive_queen",
        BossKind::RogueAICore => "boss_rogue_ai_core",
        BossKind::VoidEntity => "boss_void_entity",
        BossKind::AncientGuardian => "boss_ancient_guardian",
        BossKind::DriftLeviathan => "boss_drift_leviathan",
    }
}

pub(super) fn enemy_sprite_for_location(location_label: &str, enemy_index: usize) -> &'static str {
    match location_label {
        "Space Station" => if enemy_index % 2 == 0 { "enemy_station_guard" } else { "enemy_maintenance_drone" },
        "Asteroid Base" => if enemy_index % 2 == 0 { "enemy_rock_crawler" } else { "enemy_asteroid_miner" },
        "Derelict Ship" => if enemy_index % 2 == 0 { "enemy_zombie_crew" } else { "enemy_hull_parasite" },
        "Alien Ruins" => if enemy_index % 2 == 0 { "enemy_ruin_sentinel" } else { "enemy_glyph_phantom" },
        "Trading Post" => if enemy_index % 2 == 0 { "enemy_smuggler" } else { "enemy_market_thug" },
        "Orbital Platform" => if enemy_index % 2 == 0 { "enemy_platform_turret" } else { "enemy_void_drifter" },
        "Mining Colony" => if enemy_index % 2 == 0 { "enemy_tunnel_worm" } else { "enemy_gas_specter" },
        "Research Lab" => if enemy_index % 2 == 0 { "enemy_lab_mutant" } else { "enemy_security_bot" },
        _ => "enemy_space_pirate",
    }
}

pub(super) fn item_sprite_key(item: &Item) -> &'static str {
    match item.kind() {
        ItemKind::MedHypo => "item_health_potion",
        ItemKind::ToxinGrenade => "item_poison_flask",
        ItemKind::ScannerPulse => "item_reveal_scroll",
        ItemKind::PersonalTeleporter => "item_teleport_scroll",
        ItemKind::StimPack => "item_haste_potion",
        ItemKind::EMPGrenade => "item_stun_bomb",
        ItemKind::RationPack => "item_rice_ball",
        ItemKind::FocusStim => "item_meditation_incense",
        ItemKind::SynthAle => "item_ancestral_wine",
        ItemKind::HoloDecoy => "item_smoke_screen",
        ItemKind::PlasmaBurst => "item_fire_cracker",
        ItemKind::NanoShield => "item_iron_skin_elixir",
        ItemKind::NeuralBoost => "item_clarity_tea",
        ItemKind::CreditChip => "item_gold_ingot",
        ItemKind::ShockModule => "item_thunder_talisman",
        ItemKind::BiogelPatch => "item_jade_salve",
        ItemKind::VenomDart => "item_serpent_fang",
        ItemKind::DeflectorDrone => "item_warding_charm",
        ItemKind::NaniteSwarm => "item_ink_bomb",
        ItemKind::Revitalizer => "item_phoenix_plume",
        ItemKind::ReflectorPlate => "item_mirror_shard",
        ItemKind::CryoGrenade => "item_frost_vial",
        ItemKind::CloakingDevice => "item_shadow_cloak",
        ItemKind::PlasmaShield => "item_dragon_scale",
        ItemKind::SignalJammer => "item_bamboo_flute",
        ItemKind::NavComputer => "item_jade_compass",
        ItemKind::GrappleLine => "item_silk_rope",
        ItemKind::OmniGel => "item_lotus_elixir",
        ItemKind::SonicEmitter => "item_thunder_drum",
        ItemKind::CircuitInk => "item_cinnabar_ink",
        ItemKind::DataCore => "item_ancestor_token",
        ItemKind::ThrusterPack => "item_wind_fan",
    }
}

pub(super) fn spell_sprite_key(effect: &radical::SpellEffect) -> &'static str {
    match effect {
        radical::SpellEffect::FireAoe(_) => "spell_fire",
        radical::SpellEffect::Heal(_) => "spell_heal",
        radical::SpellEffect::Reveal => "spell_reveal",
        radical::SpellEffect::Shield => "spell_shield",
        radical::SpellEffect::StrongHit(_) => "spell_strike",
        radical::SpellEffect::Drain(_) => "spell_drain",
        radical::SpellEffect::Stun => "spell_stun",
        radical::SpellEffect::Pacify => "spell_pacify",
        radical::SpellEffect::Slow(_) => "spell_stun",
        radical::SpellEffect::Teleport => "spell_reveal",
        radical::SpellEffect::Poison(_, _) => "spell_drain",
        radical::SpellEffect::FocusRestore(_) => "spell_heal",
        radical::SpellEffect::ArmorBreak => "spell_strike",
        radical::SpellEffect::Dash(_) => "spell_reveal",
        radical::SpellEffect::Pierce(_) => "spell_strike",
        radical::SpellEffect::PullToward => "spell_reveal",
        radical::SpellEffect::KnockBack(_) => "spell_strike",
        radical::SpellEffect::Thorns(_) => "spell_shield",
        radical::SpellEffect::Cone(_) => "spell_fire",
        radical::SpellEffect::Wall(_) => "spell_shield",
        radical::SpellEffect::OilSlick => "spell_drain",
        radical::SpellEffect::FreezeGround(_) => "spell_stun",
        radical::SpellEffect::Ignite => "spell_fire",
        radical::SpellEffect::PlantGrowth => "spell_heal",
        radical::SpellEffect::Earthquake(_) => "spell_strike",
        radical::SpellEffect::Sanctify(_) => "spell_heal",
        radical::SpellEffect::FloodWave(_) => "spell_stun",
        radical::SpellEffect::SummonBoulder => "spell_shield",
    }
}

pub(super) fn spell_school_color(effect: &radical::SpellEffect) -> &'static str {
    match effect {
        radical::SpellEffect::FireAoe(_) | radical::SpellEffect::Cone(_) => "#ff6633",
        radical::SpellEffect::Heal(_) | radical::SpellEffect::FocusRestore(_) => "#44dd66",
        radical::SpellEffect::Reveal
        | radical::SpellEffect::Teleport
        | radical::SpellEffect::Dash(_)
        | radical::SpellEffect::PullToward => "#66bbff",
        radical::SpellEffect::Shield
        | radical::SpellEffect::Wall(_)
        | radical::SpellEffect::Thorns(_) => "#88aaff",
        radical::SpellEffect::StrongHit(_)
        | radical::SpellEffect::ArmorBreak
        | radical::SpellEffect::Pierce(_)
        | radical::SpellEffect::KnockBack(_) => "#ff9944",
        radical::SpellEffect::Drain(_) | radical::SpellEffect::Poison(_, _) => "#aa66dd",
        radical::SpellEffect::Stun | radical::SpellEffect::Slow(_) => "#66ddff",
        radical::SpellEffect::Pacify => "#ffdd66",
        radical::SpellEffect::OilSlick => "#8a7a4a",
        radical::SpellEffect::FreezeGround(_) | radical::SpellEffect::FloodWave(_) => "#66ddff",
        radical::SpellEffect::Ignite => "#ff6633",
        radical::SpellEffect::PlantGrowth | radical::SpellEffect::Sanctify(_) => "#44dd66",
        radical::SpellEffect::Earthquake(_) => "#ff9944",
        radical::SpellEffect::SummonBoulder => "#88aaff",
    }
}

pub(super) fn equipment_sprite_key(name: &str) -> Option<&'static str> {
    match name {
        "Brush of Clarity" => Some("equip_brush_of_clarity"),
        "Scholar's Quill" => Some("equip_scholars_quill"),
        "Dragon Fang Pen" => Some("equip_dragon_fang_pen"),
        "Iron Pickaxe" => Some("equip_iron_pickaxe"),
        "Jade Vest" => Some("equip_jade_vest"),
        "Iron Silk Robe" => Some("equip_iron_silk_robe"),
        "Phoenix Mantle" => Some("equip_phoenix_mantle"),
        "Radical Magnet" => Some("equip_radical_magnet"),
        "Life Jade" => Some("equip_life_jade"),
        "Gold Toad" => Some("equip_gold_toad"),
        "Phoenix Feather" => Some("equip_phoenix_feather"),
        _ => None,
    }
}

pub(super) fn shop_item_sprite_key(kind: &ShopItemKind) -> Option<&'static str> {
    match kind {
        ShopItemKind::Radical(_) => None,
        ShopItemKind::HealFull => Some("item_health_potion"),
        ShopItemKind::Equipment(idx) => {
            let eq = crate::player::EQUIPMENT_POOL.get(*idx)?;
            equipment_sprite_key(eq.name)
        }
        ShopItemKind::Consumable(item) => Some(item_sprite_key(item)),
    }
}

pub(super) fn hud_message_color(message: &str) -> &'static str {
    if message.starts_with("Wrong") || message.contains(" hits for ") || message.contains("resets!")
    {
        "#ff7777"
    } else if message.starts_with("⛓")
        || message.contains("Chain ")
        || message.contains("Compound broken")
    {
        "#ffbb66"
    } else if message.contains("Shield")
        || message.contains("Guard")
        || message.contains("stagger")
        || message.contains("stunned")
        || message.contains("counterattack")
    {
        "#66ddff"
    } else if message.starts_with("Defeated")
        || message.starts_with("Forged")
        || message.contains("Found")
        || message.contains("Bought")
        || message.contains("Talent learned")
    {
        "#88ff88"
    } else {
        "#ffdd88"
    }
}

pub(super) fn equipment_name(
    equipment: Option<&crate::player::Equipment>,
    enchantment: Option<&'static str>,
    state: ItemState,
) -> String {
    let prefix = match state {
        ItemState::Cursed => "💀 ",
        ItemState::Blessed => "✨ ",
        ItemState::Normal => "",
    };
    match (equipment, enchantment) {
        (Some(equipment), Some(enchantment)) => {
            format!("{}{} +{}", prefix, equipment.name, enchantment)
        }
        (Some(equipment), None) => format!("{}{}", prefix, equipment.name),
        (None, _) => "None".to_string(),
    }
}

pub(super) fn radical_stack_counts(radicals: &[&'static str]) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for radical in radicals {
        *counts.entry(*radical).or_insert(0) += 1;
    }
    counts
}

pub(super) fn word_wrap(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.len() + 1 + word.len() > max_chars {
            lines.push(current);
            current = word.to_string();
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::radical_stack_counts;

    #[test]
    fn radical_stack_counts_groups_duplicate_radicals() {
        let counts = radical_stack_counts(&["水", "木", "水"]);

        assert_eq!(counts.get("水"), Some(&2));
        assert_eq!(counts.get("木"), Some(&1));
        assert_eq!(counts.len(), 2);
    }

    #[test]
    fn radical_stack_counts_returns_empty_map_for_empty_inventory() {
        let counts = radical_stack_counts(&[]);

        assert!(counts.is_empty());
    }
}
