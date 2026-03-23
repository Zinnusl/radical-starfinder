//! Typing/pinyin input system for tactical combat.

use crate::combat::{TacticalBattle, TypingAction};
use crate::vocab;

use super::BattleEvent;

// ── Typing phase ─────────────────────────────────────────────────────────────

pub(super) fn handle_typing(battle: &mut TacticalBattle, key: &str) -> BattleEvent {
    match key {
        "Escape" => {
            // Cancel typing.
            battle.typing_action = None;
            battle.typing_buffer.clear();
            battle.log_message("Attack cancelled.");
            BattleEvent::None
        }
        "Backspace" => {
            battle.typing_buffer.pop();
            BattleEvent::None
        }
        "Enter" => submit_typing(battle),
        _ => {
            // Only accept alphanumeric + space for pinyin input.
            if key.len() == 1 {
                let ch = key.chars().next().unwrap();
                if ch.is_ascii_alphanumeric() || ch == ' ' {
                    battle.typing_buffer.push(ch);
                }
            }
            BattleEvent::None
        }
    }
}

fn submit_typing(battle: &mut TacticalBattle) -> BattleEvent {
    let action = match battle.typing_action.take() {
        Some(a) => a,
        None => return BattleEvent::None,
    };

    let input = battle.typing_buffer.clone();
    battle.typing_buffer.clear();

    match action {
        TypingAction::BasicAttack { target_unit } => {
            super::resolution::resolve_basic_attack(battle, target_unit, &input)
        }
        TypingAction::SpellCast {
            spell_idx,
            target_x,
            target_y,
            effect,
        } => super::resolution::resolve_spell_cast(battle, spell_idx, target_x, target_y, effect, &input),
        TypingAction::ShieldBreak {
            target_unit,
            component,
        } => super::resolution::resolve_shield_break(battle, target_unit, component, &input),
        TypingAction::EliteChain {
            target_unit,
            syllable_progress,
            total_syllables,
            damage_per_syllable,
            damage_dealt,
        } => super::resolution::resolve_elite_chain(
            battle,
            target_unit,
            syllable_progress,
            total_syllables,
            damage_per_syllable,
            damage_dealt,
            &input,
        ),
    }
}

/// Check if the typed pinyin matches the enemy's hanzi.
pub(super) fn check_attack_pinyin(battle: &TacticalBattle, target_idx: usize, input: &str) -> bool {
    let unit = &battle.units[target_idx];
    if !unit.hanzi.is_empty() {
        if let Some(entry) = vocab::vocab_entry_by_hanzi(unit.hanzi) {
            return vocab::check_pinyin(entry, input);
        }
        return unit.pinyin.eq_ignore_ascii_case(&input.replace(' ', ""));
    }
    false
}

pub(super) fn check_partial_pinyin(battle: &TacticalBattle, target_idx: usize, input: &str) -> bool {
    let unit = &battle.units[target_idx];
    if !unit.hanzi.is_empty() {
        if let Some(entry) = vocab::vocab_entry_by_hanzi(unit.hanzi) {
            return vocab::check_pinyin_partial(entry, input);
        }
    }
    false
}

const CHENGYU_LIST: &[(&str, &str)] = &[
    ("\u{4e00}\u{5fc3}\u{4e00}\u{610f}", "Wholehearted"),
    ("\u{4e07}\u{4e8b}\u{5982}\u{610f}", "Everything goes well"),
    ("\u{5929}\u{4e0b}\u{592a}\u{5e73}", "Peace under heaven"),
    ("\u{5fc3}\u{60f3}\u{4e8b}\u{6210}", "Dreams come true"),
    ("\u{5927}\u{5f00}\u{773c}\u{754c}", "Eye-opening"),
    ("\u{4e03}\u{4e0a}\u{516b}\u{4e0b}", "At sixes and sevens"),
    ("\u{4e94}\u{5149}\u{5341}\u{8272}", "Dazzling"),
    ("\u{4e5d}\u{6b7b}\u{4e00}\u{751f}", "Narrow escape"),
    ("\u{534a}\u{9014}\u{800c}\u{5e9f}", "Give up halfway"),
    ("\u{81ea}\u{8a00}\u{81ea}\u{8bed}", "Talk to oneself"),
    ("\u{5165}\u{4e61}\u{968f}\u{4fd7}", "When in Rome"),
    ("\u{9a6c}\u{5230}\u{6210}\u{529f}", "Instant success"),
    ("\u{5927}\u{540c}\u{5c0f}\u{5f02}", "Mostly the same"),
    ("\u{767e}\u{53d1}\u{767e}\u{4e2d}", "Hit every target"),
    ("\u{5343}\u{65b9}\u{767e}\u{8ba1}", "By every means"),
    ("\u{5f00}\u{95e8}\u{89c1}\u{5c71}", "Get to the point"),
    ("\u{4e00}\u{5200}\u{4e24}\u{65ad}", "Cut cleanly"),
    ("\u{4e00}\u{76ee}\u{4e86}\u{7136}", "Crystal clear"),
    ("\u{4e0d}\u{53ef}\u{601d}\u{8bae}", "Incredible"),
    ("\u{6cf0}\u{7136}\u{81ea}\u{82e5}", "Calm and composed"),
    ("\u{5b66}\u{4ee5}\u{81f4}\u{7528}", "Learn to apply"),
    (
        "\u{5927}\u{5668}\u{665a}\u{6210}",
        "Great minds mature slowly",
    ),
    (
        "\u{53e3}\u{662f}\u{5fc3}\u{975e}",
        "Say one thing mean another",
    ),
    (
        "\u{9f99}\u{98de}\u{51e4}\u{821e}",
        "Dragons fly phoenixes dance",
    ),
    ("\u{864e}\u{5934}\u{86c7}\u{5c3e}", "Strong start weak end"),
    ("\u{6c34}\u{6ef4}\u{77f3}\u{7a7f}", "Water wears stone"),
    (
        "\u{98ce}\u{548c}\u{65e5}\u{4e3d}",
        "Gentle breeze sunny day",
    ),
    ("\u{91d1}\u{7389}\u{6ee1}\u{5802}", "Riches fill the hall"),
    ("\u{5929}\u{957f}\u{5730}\u{4e45}", "Everlasting"),
    ("\u{5fc3}\u{5982}\u{6b62}\u{6c34}", "Mind still as water"),
    ("\u{5149}\u{660e}\u{78ca}\u{843d}", "Open and upright"),
    ("\u{4e00}\u{8def}\u{5e73}\u{5b89}", "Safe journey"),
];

pub(super) fn check_chengyu_combo(battle: &mut TacticalBattle) -> Option<String> {
    if battle.chengyu_history.len() < 4 {
        return None;
    }
    let last4: String = battle.chengyu_history[battle.chengyu_history.len() - 4..].join("");
    for &(idiom, name) in CHENGYU_LIST {
        if last4 == idiom {
            battle.chengyu_history.clear();
            let unit = &mut battle.units[0];
            let heal = (unit.max_hp / 3).max(2);
            unit.hp = (unit.hp + heal).min(unit.max_hp);
            for i in 1..battle.units.len() {
                if battle.units[i].alive && battle.units[i].is_enemy() {
                    battle.units[i].stunned = true;
                }
            }
            return Some(format!(
                "CHENGYU! {} ({})! Heal {} HP, all enemies stunned!",
                idiom, name, heal
            ));
        }
    }
    None
}

