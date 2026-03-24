//! Free helper functions used by the game module.

use super::*;


/// Combo effects from spell combinations.
pub(super) enum ComboEffect {
    Steam,        // Fire + Shield: AoE stun
    Counter(i32), // Shield + Strike: reflect damage
    Barrier(i32), // Heal + Shield: shield + heal
    Flurry(i32),  // Strike + Fire: triple damage
    Ignite(i32),  // Fire + Drain: burn DoT + lifesteal
    Tempest(i32), // Fire + Stun: AoE damage + stun target
    Rally(i32),   // Heal + Strike: heal + bonus hit
    Siphon(i32),  // Drain + Stun: massive drain while stunned
    Fortify(i32), // Drain + Shield: shield + heal from stolen life
}

/// Detect if two spell effects form a combo.
pub(super) fn detect_combo(prev: &SpellEffect, current: &SpellEffect) -> Option<(&'static str, ComboEffect)> {
    match (spell_category(prev), spell_category(current)) {
        ("fire", "shield") | ("shield", "fire") => Some(("Steam Burst", ComboEffect::Steam)),
        ("shield", "strike") | ("strike", "shield") => {
            Some(("Counter Strike", ComboEffect::Counter(6)))
        }
        ("heal", "shield") | ("shield", "heal") => Some(("Barrier", ComboEffect::Barrier(4))),
        ("strike", "fire") | ("fire", "strike") => Some(("Flurry", ComboEffect::Flurry(8))),
        ("drain", "heal") | ("heal", "drain") => Some(("Life Surge", ComboEffect::Barrier(6))),
        ("stun", "strike") | ("strike", "stun") => {
            Some(("Crippling Blow", ComboEffect::Flurry(10)))
        }
        ("fire", "drain") | ("drain", "fire") => Some(("Immolate", ComboEffect::Ignite(8))),
        ("fire", "stun") | ("stun", "fire") => Some(("Tempest", ComboEffect::Tempest(4))),
        ("heal", "strike") | ("strike", "heal") => Some(("Rally", ComboEffect::Rally(6))),
        ("drain", "stun") | ("stun", "drain") => Some(("Siphon", ComboEffect::Siphon(6))),
        ("drain", "shield") | ("shield", "drain") => Some(("Fortify", ComboEffect::Fortify(4))),
        ("heal", "stun") | ("stun", "heal") => Some(("Renewal", ComboEffect::Barrier(5))),
        _ => None,
    }
}

pub(super) fn spell_category(effect: &SpellEffect) -> &'static str {
    match effect {
        SpellEffect::FireAoe(_) => "fire",
        SpellEffect::Heal(_) => "heal",
        SpellEffect::Reveal => "utility",
        SpellEffect::Shield => "shield",
        SpellEffect::StrongHit(_) => "strike",
        SpellEffect::Drain(_) => "drain",
        SpellEffect::Stun => "stun",
        SpellEffect::Pacify => "utility",
        SpellEffect::Slow(_) => "stun",
        SpellEffect::Teleport => "utility",
        SpellEffect::Poison(_, _) => "drain",
        SpellEffect::FocusRestore(_) => "heal",
        SpellEffect::ArmorBreak => "strike",
        SpellEffect::Dash(_) => "utility",
        SpellEffect::Pierce(_) => "strike",
        SpellEffect::PullToward => "utility",
        SpellEffect::KnockBack(_) => "strike",
        SpellEffect::Thorns(_) => "shield",
        SpellEffect::Cone(_) => "fire",
        SpellEffect::Wall(_) => "shield",
        SpellEffect::OilSlick => "utility",
        SpellEffect::FreezeGround(_) => "stun",
        SpellEffect::Ignite => "fire",
        SpellEffect::PlantGrowth => "heal",
        SpellEffect::Earthquake(_) => "strike",
        SpellEffect::Sanctify(_) => "heal",
        SpellEffect::FloodWave(_) => "strike",
        SpellEffect::SummonBoulder => "shield",
        SpellEffect::Charge(_) => "strike",
        SpellEffect::Blink(_) => "utility",
    }
}

pub(super) fn combat_prompt_for(enemy: &Enemy, listening_mode: ListenMode, mirror_hint: bool) -> String {
    let pinyin_hint = if mirror_hint {
        format!(" (Hint: {})", enemy.pinyin)
    } else {
        String::new()
    };

    if enemy.is_elite {
        let target = enemy
            .hanzi
            .chars()
            .nth(enemy.elite_chain)
            .map(|ch| ch.to_string())
            .unwrap_or_else(|| enemy.hanzi.chars().last().unwrap_or('？').to_string());
        let expected = enemy.elite_expected_syllable().unwrap_or(enemy.pinyin);
        format!(
            "Compound foe {} ({}) — break it syllable by syllable. Start with {} = {}.{}",
            enemy.hanzi, enemy.meaning, target, expected, pinyin_hint
        )
    } else if !enemy.components.is_empty() {
        let comp = enemy.components[0];
        let pinyin = vocab::vocab_entry_by_hanzi(comp)
            .map(|e| e.pinyin)
            .unwrap_or("???");
        format!("Shielded by {}! Type {} to break.", comp, pinyin)
    } else if listening_mode == ListenMode::ToneOnly {
        format!(
            "🎵 What tone is {}? Type 1-4...{}",
            enemy.meaning, pinyin_hint
        )
    } else if listening_mode == ListenMode::FullAudio {
        format!("🎧 Listen! Type the pinyin you hear...{}", pinyin_hint)
    } else {
        format!(
            "Type pinyin for {} ({}){}",
            enemy.hanzi, enemy.meaning, pinyin_hint
        )
    }
}

pub(super) fn in_look_range(origin_x: i32, origin_y: i32, target_x: i32, target_y: i32) -> bool {
    (target_x - origin_x).abs().max((target_y - origin_y).abs()) <= LOOK_RANGE
}

pub(super) fn tile_look_text(tile: Tile) -> String {
    match tile {
        Tile::Bulkhead => "Solid wall.".to_string(),
        Tile::DamagedBulkhead => {
            "Cracked wall — a digging tool could break into a hidden room.".to_string()
        }
        Tile::WeakBulkhead => {
            "Brittle wall — a digging tool could break into the cache behind it.".to_string()
        }
        Tile::MetalFloor => "Open floor.".to_string(),
        Tile::Hallway => "Corridor passage.".to_string(),
        Tile::Airlock => "Stairs down to the next floor.".to_string(),
        Tile::QuantumForge => "Forge — combine radicals or enchant gear here.".to_string(),
        Tile::TradeTerminal => "Shop — buy gear, radicals, and consumables.".to_string(),
        Tile::SupplyCrate => "Treasure chest — step onto it to open it.".to_string(),
        Tile::CargoCrate => "Crate — push it, or shove it into deep water to make a bridge.".to_string(),
        Tile::LaserGrid => "Spike trap — hurts anything that steps on it.".to_string(),
        Tile::Coolant => "Oil slick — fire can ignite it.".to_string(),
        Tile::CoolantPool => {
            "Shallow water — you can wade through it, and lightning arcs through it.".to_string()
        }
        Tile::VacuumBreach => {
            "Deep water — too deep to cross on foot; a crate could bridge it.".to_string()
        }
        Tile::Npc(0) => format!("{} — offers meaning hints.", Companion::ScienceOfficer.name()),
        Tile::Npc(1) => format!("{} — heals you between floors.", Companion::Medic.name()),
        Tile::Npc(2) => format!(
            "{} — discounts goods and may offer quests.",
            Companion::Quartermaster.name()
        ),
        Tile::Npc(_) => format!(
            "{} — can block the first hit in a fight.",
            Companion::SecurityChief.name()
        ),
        Tile::CircuitShrine => "Tone shrine — complete a tone challenge for bonus damage.".to_string(),
        Tile::CompoundShrine => "Stroke shrine — arrange character components in order.".to_string(),
        Tile::FrequencyWall => "Tone wall — identify tones to defend against attacks.".to_string(),
        Tile::ClassifierNode => {
            "Classifier shrine — match nouns with correct classifiers.".to_string()
        }
        Tile::Terminal(kind) => format!("{} — offer items here, or pray with 20 favor.", kind.name()),
        Tile::SecurityLock(kind) => format!(
            "{} — one-shot script seal that reshapes the room.",
            kind.label()
        ),
        Tile::InfoPanel(_) => "Tutorial sign — step onto it to read the guidance.".to_string(),
        Tile::Catwalk => "Bridge — safe footing laid over water.".to_string(),
        Tile::DataWell => "Ink well — guess the component count to restore HP.".to_string(),
        Tile::MemorialNode => "Ancestor shrine — complete the proverb for gold.".to_string(),
        Tile::TranslationTerminal => {
            "Translation altar — pick the correct Chinese for English meaning.".to_string()
        }
        Tile::RadicalLab => "Radical garden — identify the radical to harvest it.".to_string(),
        Tile::HoloPool => "Mirror pool — type the pinyin to gain spell power.".to_string(),
        Tile::DroidTutor => "Stone tutor — study, then prove you learned the tone.".to_string(),
        Tile::CodexTerminal => "Codex shrine — quiz on characters you've encountered.".to_string(),
        Tile::DataBridge => "Word bridge — answer correctly to bridge the water.".to_string(),
        Tile::SealedHatch => "Locked door — translate to unlock.".to_string(),
        Tile::CorruptedFloor => "Cursed floor — a hidden trap awaits the unwary.".to_string(),
        Tile::Trap(_) => "Open floor.".to_string(),
        Tile::OreVein => "Gold ore vein — mine it for gold.".to_string(),
        Tile::PlasmaVent => "Molten lava — stepping on it will burn you!".to_string(),
        Tile::FrozenDeck => "Ice — slippery surface, be careful.".to_string(),
        Tile::CargoPipes => "Dense bamboo — blocks passage.".to_string(),
        Tile::ToxicFungus => "Giant mushroom — spore cloud causes disorientation.".to_string(),
        Tile::ToxicGas => "Poison gas — toxic fumes linger here.".to_string(),
        Tile::DataRack => "Data rack — interact for information.".to_string(),
        Tile::SalvageCrate => "Salvage crate — smash for loot.".to_string(),
        Tile::NavBeacon => "Nav beacon — activate for map.".to_string(),
        Tile::SpecialRoom(_) => "Special room.".to_string(),
        Tile::PressureSensor => "Pressure plate — something heavy might activate it.".to_string(),
        Tile::CrystalPanel => "Crystal formation — reflects light beautifully.".to_string(),
        Tile::WarpGatePortal => "Dragon Gate — an otherworldly portal shimmering with power.".to_string(),
        Tile::MedBayTile => "Med bay — step in to restore HP.".to_string(),
        Tile::CreditCache => "Gold pile — walk over it to collect.".to_string(),
    }
}

pub(super) fn tile_allows_enemy_spawn(tile: Tile) -> bool {
    matches!(
        tile,
        Tile::MetalFloor | Tile::Hallway | Tile::Coolant | Tile::CoolantPool | Tile::LaserGrid | Tile::Catwalk
    )
}

pub(super) fn enemy_look_text(enemy: &Enemy) -> String {
    let role = if enemy.is_boss {
        "Boss"
    } else if enemy.is_elite {
        "Elite"
    } else {
        "Enemy"
    };

    let mut text = format!(
        "{} {} ({}) HP {}/{}",
        role, enemy.hanzi, enemy.meaning, enemy.hp, enemy.max_hp
    );
    if !enemy.components.is_empty() {
        text.push_str(&format!(" — shield {}.", enemy.components.join("→")));
    } else if enemy.is_elite {
        if let Some(next) = enemy.elite_expected_syllable() {
            text.push_str(&format!(" — next {}.", next));
        }
    }
    if let Some(trait_text) = enemy.boss_trait_text() {
        text.push_str(&format!(" {}", trait_text));
    }

    let actions = enemy.radical_actions();
    if !actions.is_empty() {
        let mut by_radical: Vec<(&str, Vec<&str>)> = Vec::new();
        for action in &actions {
            let rad = action.radical();
            if let Some(entry) = by_radical.iter_mut().find(|(r, _)| *r == rad) {
                entry.1.push(action.name());
            } else {
                by_radical.push((rad, vec![action.name()]));
            }
        }
        let grouped: Vec<String> = by_radical
            .iter()
            .map(|(rad, names)| format!("{}: {}", rad, names.join(", ")))
            .collect();
        text.push_str(&format!(" | Abilities: {}", grouped.join(" | ")));
    }

    text
}

pub(super) fn elite_chain_damage(base_hit: i32, total_syllables: usize, completing_cycle: bool) -> i32 {
    if completing_cycle {
        base_hit + total_syllables.saturating_sub(1) as i32
    } else {
        (base_hit / 2).max(1)
    }
}

pub(super) fn elite_remaining_hp(current_hp: i32, damage: i32, completing_cycle: bool) -> i32 {
    if completing_cycle {
        current_hp - damage
    } else {
        (current_hp - damage).max(1)
    }
}

pub(super) fn advance_message_decay(
    message_timer: &mut u8,
    message_tick_delay: &mut u8,
    text_speed: TextSpeed,
) -> bool {
    if *message_timer == 0 {
        return true;
    }

    if *message_tick_delay > 0 {
        *message_tick_delay -= 1;
        return false;
    }

    *message_tick_delay = text_speed.timer_delay().saturating_sub(1);
    *message_timer = message_timer.saturating_sub(text_speed.timer_step());
    *message_timer == 0
}

pub(super) fn tutorial_exit_blocker_for(tutorial: Option<&TutorialState>) -> Option<&'static str> {
    let tutorial = tutorial?;
    if !tutorial.combat_done {
        Some("The exit is sealed. Defeat 大 before leaving the tutorial.")
    } else if !tutorial.forge_done {
        Some("The exit is sealed. Forge 好 at the anvil before leaving.")
    } else {
        None
    }
}

pub(super) fn can_be_reshaped_by_seal(tile: Tile) -> bool {
    matches!(
        tile,
        Tile::MetalFloor | Tile::Hallway | Tile::Coolant | Tile::CoolantPool | Tile::LaserGrid
    )
}

pub(super) fn seal_cross_positions(x: i32, y: i32) -> [(i32, i32); 8] {
    [
        (x + 1, y),
        (x - 1, y),
        (x + 2, y),
        (x - 2, y),
        (x, y + 1),
        (x, y - 1),
        (x, y + 2),
        (x, y - 2),
    ]
}
