//! Console input handling and cheat commands.

use super::*;
use crate::combat;
use crate::enemy::{BossKind, Enemy};
use crate::player::ItemState;
use crate::radical;
use crate::vocab;

pub(crate) fn longest_common_prefix_ci(strings: &[&str]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    let first: Vec<char> = strings[0].chars().collect();
    let mut len = first.len();
    for s in &strings[1..] {
        let chars: Vec<char> = s.chars().collect();
        len = len.min(chars.len());
        for i in 0..len {
            if first[i].to_lowercase().next() != chars[i].to_lowercase().next() {
                len = i;
                break;
            }
        }
    }
    strings[0][..strings[0].char_indices().nth(len).map_or(strings[0].len(), |(i, _)| i)].to_string()
}

impl super::GameState {
    const CONSOLE_COMMANDS: &'static [&'static str] = &[
        "help", "god", "hp", "gold", "floor", "reveal", "kill_all",
        "focus", "clear", "stats", "items", "give_item",
        "radicals", "give_radical", "spells", "give_spell", "fight", "boss",
    ];

    const ITEM_NAMES: &'static [&'static str] = &[
        "HealthPotion", "PoisonFlask", "RevealScroll", "TeleportScroll",
        "HastePotion", "StunBomb", "RiceBall", "MeditationIncense",
        "AncestralWine", "SmokeScreen", "FireCracker", "IronSkinElixir",
        "ClarityTea", "GoldIngot", "ThunderTalisman", "JadeSalve",
        "SerpentFang", "WardingCharm", "InkBomb", "PhoenixPlume",
    ];

    const BOSS_NAMES: &'static [&'static str] = &[
        "PirateCaptain", "HiveQueen", "RogueAICore", "VoidEntity", "AncientGuardian", "DriftLeviathan",
    ];

    const FIGHT_TYPES: &'static [&'static str] = &["normal", "elite", "boss"];

    pub(crate) fn tab_complete(&mut self) {
        let input = self.console_buffer.clone();
        let has_space = input.contains(' ');

        if has_space {
            // Argument completion
            let parts: Vec<&str> = input.splitn(2, ' ').collect();
            let cmd = parts[0];
            let arg_prefix = parts.get(1).unwrap_or(&"");

            let candidates: Vec<&str> = match cmd {
                "give_item" => Self::ITEM_NAMES.iter()
                    .filter(|n| n.to_lowercase().starts_with(&arg_prefix.to_lowercase()))
                    .copied().collect(),
                "boss" => Self::BOSS_NAMES.iter()
                    .filter(|n| n.to_lowercase().starts_with(&arg_prefix.to_lowercase()))
                    .copied().collect(),
                "fight" => Self::FIGHT_TYPES.iter()
                    .filter(|n| n.starts_with(&arg_prefix.to_lowercase()))
                    .copied().collect(),
                _ => return,
            };

            if candidates.is_empty() {
                return;
            }

            let prefix_key = format!("arg:{}", input);
            if self.tab_prefix == prefix_key {
                // Cycle through matches
                self.tab_cycle_index = (self.tab_cycle_index + 1) % self.tab_matches.len();
                self.console_buffer = format!("{} {}", cmd, self.tab_matches[self.tab_cycle_index]);
            } else {
                // New completion
                self.tab_matches = candidates.iter().map(|s| s.to_string()).collect();
                self.tab_cycle_index = 0;
                if candidates.len() == 1 {
                    self.console_buffer = format!("{} {}", cmd, candidates[0]);
                    self.tab_prefix = format!("arg:{}", self.console_buffer);
                } else {
                    let lcp = longest_common_prefix_ci(&candidates);
                    self.console_buffer = format!("{} {}", cmd, lcp);
                    self.console_history.push(format!("  completions: {}", candidates.join(", ")));
                    self.tab_prefix = format!("arg:{}", self.console_buffer);
                }
            }
        } else {
            // Command name completion
            let prefix = input.to_lowercase();
            let candidates: Vec<&str> = Self::CONSOLE_COMMANDS.iter()
                .filter(|c| c.starts_with(&prefix))
                .copied().collect();

            if candidates.is_empty() {
                return;
            }

            let prefix_key = format!("cmd:{}", input);
            if self.tab_prefix == prefix_key {
                // Cycle through matches
                self.tab_cycle_index = (self.tab_cycle_index + 1) % self.tab_matches.len();
                self.console_buffer = format!("{} ", self.tab_matches[self.tab_cycle_index]);
            } else {
                // New completion
                self.tab_matches = candidates.iter().map(|s| s.to_string()).collect();
                self.tab_cycle_index = 0;
                if candidates.len() == 1 {
                    self.console_buffer = format!("{} ", candidates[0]);
                    self.tab_prefix = format!("cmd:{}", self.console_buffer);
                } else {
                    let lcp = longest_common_prefix_ci(&candidates);
                    self.console_buffer = lcp;
                    self.console_history.push(format!("  completions: {}", candidates.join(", ")));
                    self.tab_prefix = format!("cmd:{}", self.console_buffer);
                }
            }
        }
    }

    pub(crate) fn execute_console_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        let response = match parts[0] {
            "help" => {
                self.console_history.push("=== CHEAT CONSOLE ===".into());
                self.console_history
                    .push("help         - Show this help".into());
                self.console_history
                    .push("god          - Toggle god mode".into());
                self.console_history
                    .push("hp [n]       - Set HP to n (or full)".into());
                self.console_history
                    .push("gold [n]     - Add n gold (default 100)".into());
                self.console_history
                    .push("floor [n]    - Go to floor n".into());
                self.console_history
                    .push("reveal       - Reveal entire map".into());
                self.console_history
                    .push("kill_all     - Kill all enemies".into());
                self.console_history
                    .push("focus [n]    - Set focus in combat".into());
                self.console_history
                    .push("clear        - Clear console".into());
                self.console_history
                    .push("stats        - Show player stats".into());
                self.console_history
                    .push("items        - List all item types".into());
                self.console_history
                    .push("give_item <name> - Give item by name".into());
                self.console_history
                    .push("radicals     - List all radicals".into());
                self.console_history
                    .push("give_radical <ch> - Give a radical".into());
                self.console_history
                    .push("spells       - List player spells".into());
                self.console_history
                    .push("give_spell <hanzi> - Give spell by hanzi".into());
                self.console_history
                    .push("fight <type> - Fight normal/elite/boss".into());
                self.console_history
                    .push("boss <name>  - Fight a specific boss".into());
                return;
            }
            "god" => {
                self.god_mode = !self.god_mode;
                if let CombatState::TacticalBattle(ref mut battle) = self.combat {
                    battle.god_mode = self.god_mode;
                }
                format!("God mode: {}", if self.god_mode { "ON" } else { "OFF" })
            }
            "hp" => {
                let amount = parts.get(1).and_then(|s| s.parse::<i32>().ok());
                match amount {
                    Some(n) => {
                        self.player.hp = n.min(self.player.max_hp);
                        format!("HP set to {}/{}", self.player.hp, self.player.max_hp)
                    }
                    None => {
                        self.player.hp = self.player.max_hp;
                        format!("HP restored to {}", self.player.max_hp)
                    }
                }
            }
            "gold" => {
                let amount = parts
                    .get(1)
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(100);
                self.player.gold += amount;
                format!("Added {} gold (total: {})", amount, self.player.gold)
            }
            "floor" => {
                if let Some(n) = parts.get(1).and_then(|s| s.parse::<i32>().ok()) {
                    if n >= 1 {
                        self.floor_num = n - 1;
                        self.new_floor();
                        format!("Warped to floor {}", self.floor_num)
                    } else {
                        "Floor must be >= 1".into()
                    }
                } else {
                    format!("Current floor: {}. Usage: floor [n]", self.floor_num)
                }
            }
            "reveal" => {
                self.reveal_entire_floor();
                for v in self.level.visible.iter_mut() {
                    *v = true;
                }
                "Map revealed!".into()
            }
            "kill_all" => {
                if let CombatState::TacticalBattle(ref mut battle) = self.combat {
                    let mut killed = 0;
                    for unit in battle.units.iter_mut() {
                        if unit.is_enemy() && unit.alive {
                            unit.hp = 0;
                            unit.alive = false;
                            killed += 1;
                        }
                    }
                    format!("Killed {} tactical enemies", killed)
                } else {
                    let count = self.enemies.len();
                    for e in self.enemies.iter_mut() {
                        e.hp = 0;
                    }
                    self.enemies.clear();
                    format!("Killed {} enemies", count)
                }
            }
            "focus" => {
                if let CombatState::TacticalBattle(ref mut battle) = self.combat {
                    let amount = parts
                        .get(1)
                        .and_then(|s| s.parse::<i32>().ok())
                        .unwrap_or(battle.max_focus);
                    battle.focus = amount;
                    format!("Focus set to {}/{}", battle.focus, battle.max_focus)
                } else {
                    "Not in tactical combat".into()
                }
            }
            "clear" => {
                self.console_history.clear();
                return;
            }
            "stats" => {
                self.console_history.push(format!(
                    "HP: {}/{}  Gold: {}  Floor: {}",
                    self.player.hp, self.player.max_hp, self.player.gold, self.floor_num
                ));
                self.console_history.push(format!(
                    "Kills: {}  God: {}",
                    self.total_kills, self.god_mode
                ));
                self.console_history.push(format!(
                    "Radicals: {}  Spells: {}  Items: {}",
                    self.player.radicals.len(),
                    self.player.spells.len(),
                    self.player.items.len()
                ));
                return;
            }
            "items" => {
                let all: &[(&str, &str)] = &[
                    ("HealthPotion", "Heal HP"),
                    ("PoisonFlask", "Poison enemies"),
                    ("RevealScroll", "Reveal map"),
                    ("TeleportScroll", "Teleport"),
                    ("HastePotion", "Grant haste"),
                    ("StunBomb", "Stun enemies"),
                    ("RiceBall", "Restore HP"),
                    ("MeditationIncense", "Grant regen"),
                    ("AncestralWine", "Restore 5 HP"),
                    ("SmokeScreen", "Smoke + haste"),
                    ("FireCracker", "AoE damage"),
                    ("IronSkinElixir", "Shield + regen"),
                    ("ClarityTea", "Cleanse debuffs"),
                    ("GoldIngot", "Gain gold"),
                    ("ThunderTalisman", "High damage"),
                    ("JadeSalve", "Regen over time"),
                    ("SerpentFang", "Envenom weapon"),
                    ("WardingCharm", "Shield + regen"),
                    ("InkBomb", "Stun + confuse"),
                    ("PhoenixPlume", "Auto-revive"),
                ];
                self.console_history.push("=== ITEM TYPES ===".into());
                for (name, desc) in all {
                    self.console_history
                        .push(format!("  {} - {}", name, desc));
                }
                return;
            }
            "give_item" => {
                use crate::player::Item;
                if let Some(name) = parts.get(1) {
                    let lower = name.to_lowercase();
                    let item = match lower.as_str() {
                        "healthpotion" => Some(Item::MedHypo(5 + self.floor_num)),
                        "poisonflask" => Some(Item::ToxinGrenade(2, 3)),
                        "revealscroll" => Some(Item::ScannerPulse),
                        "teleportscroll" => Some(Item::PersonalTeleporter),
                        "hastepotion" => Some(Item::StimPack(5)),
                        "stunbomb" => Some(Item::EMPGrenade),
                        "riceball" => Some(Item::RationPack(40)),
                        "meditationincense" => Some(Item::FocusStim(5)),
                        "ancestralwine" => Some(Item::SynthAle(3)),
                        "smokescreen" => Some(Item::HoloDecoy(4)),
                        "firecracker" => Some(Item::PlasmaBurst(3 + self.floor_num / 2)),
                        "ironskinelixir" => Some(Item::NanoShield(5)),
                        "claritytea" => Some(Item::NeuralBoost),
                        "goldingot" => Some(Item::CreditChip(8 + self.floor_num * 2)),
                        "thundertalisman" => Some(Item::ShockModule(5 + self.floor_num)),
                        "jadesalve" => Some(Item::BiogelPatch(2)),
                        "serpentfang" => Some(Item::VenomDart),
                        "wardingcharm" => Some(Item::DeflectorDrone(5)),
                        "inkbomb" => Some(Item::NaniteSwarm),
                        "phoenixplume" => Some(Item::Revitalizer(5)),
                        _ => None,
                    };
                    match item {
                        Some(it) => {
                            let name_str = it.name().to_string();
                            if self.player.add_item(it, ItemState::Normal) {
                                format!("Added {}", name_str)
                            } else {
                                "Inventory full!".into()
                            }
                        }
                        None => format!("Unknown item '{}'. Type 'items' to list.", name),
                    }
                } else {
                    "Usage: give_item <name>".into()
                }
            }
            "radicals" => {
                self.console_history.push("=== RADICALS ===".into());
                for r in radical::RADICALS.iter() {
                    let tag = if r.rare { " [rare]" } else { "" };
                    self.console_history
                        .push(format!("  {} ({}) - {}{}", r.ch, r.name, r.meaning, tag));
                }
                return;
            }
            "give_radical" => {
                if let Some(ch) = parts.get(1) {
                    if let Some(r) = radical::RADICALS.iter().find(|r| r.ch == *ch) {
                        self.player.add_radical(r.ch);
                        format!("Added radical {} ({})", r.ch, r.meaning)
                    } else {
                        format!("Unknown radical '{}'. Type 'radicals' to list.", ch)
                    }
                } else {
                    "Usage: give_radical <char>".into()
                }
            }
            "spells" => {
                if self.player.spells.is_empty() {
                    self.console_history.push("No spells.".into());
                } else {
                    self.console_history.push("=== SPELLS ===".into());
                    for (i, s) in self.player.spells.iter().enumerate() {
                        let sel = if i == self.player.selected_spell {
                            " ◀"
                        } else {
                            ""
                        };
                        self.console_history.push(format!(
                            "  {} {} ({}) - {:?}{}",
                            s.hanzi, s.pinyin, s.meaning, s.effect, sel
                        ));
                    }
                }
                return;
            }
            "give_spell" => {
                if let Some(hanzi) = parts.get(1) {
                    if let Some(recipe) = radical::RECIPES
                        .iter()
                        .find(|r| r.output_hanzi == *hanzi)
                    {
                        self.player.add_spell(Spell {
                            hanzi: recipe.output_hanzi,
                            pinyin: recipe.output_pinyin,
                            meaning: recipe.output_meaning,
                            effect: recipe.effect,
                        });
                        format!(
                            "Added spell {} ({} - {})",
                            recipe.output_hanzi, recipe.output_pinyin, recipe.output_meaning
                        )
                    } else {
                        format!("No recipe for '{}'. Check radical.rs RECIPES.", hanzi)
                    }
                } else {
                    "Usage: give_spell <hanzi>".into()
                }
            }
            "fight" => {
                if let Some(kind) = parts.get(1) {
                    let lower = kind.to_lowercase();
                    let pool = vocab::vocab_for_floor(self.floor_num);
                    if pool.is_empty() {
                        "No vocab entries for this floor!".into()
                    } else {
                        let entry = pool[self.rng_next() as usize % pool.len()];
                        let px = self.player.x;
                        let py = self.player.y;
                        match lower.as_str() {
                            "normal" => {
                                let e = Enemy::from_vocab(entry, px + 1, py, self.floor_num);
                                let label = format!("Fight: {} ({})", e.hanzi, e.meaning);
                                self.enemies.push(e);
                                let idx = self.enemies.len() - 1;
                                let battle = combat::transition::enter_combat(
                                    &self.player,
                                    &self.enemies,
                                    &[idx],
                                    self.floor_num,
                                    self.current_room_modifier(),
                                    &self.srs,
                                    self.companion,
                                );
                                self.combat = CombatState::TacticalBattle(Box::new(battle));
                                self.typing.clear();
                                label
                            }
                            "elite" => {
                                let elite_pool: Vec<&VocabEntry> =
                                    pool.iter().filter(|v| vocab::is_elite(v)).copied().collect();
                                let ep = if elite_pool.is_empty() {
                                    entry
                                } else {
                                    elite_pool[self.rng_next() as usize % elite_pool.len()]
                                };
                                let e = Enemy::from_vocab(ep, px + 1, py, self.floor_num);
                                let label = format!("Fight elite: {} ({})", e.hanzi, e.meaning);
                                self.enemies.push(e);
                                let idx = self.enemies.len() - 1;
                                let battle = combat::transition::enter_combat(
                                    &self.player,
                                    &self.enemies,
                                    &[idx],
                                    self.floor_num,
                                    self.current_room_modifier(),
                                    &self.srs,
                                    self.companion,
                                );
                                self.combat = CombatState::TacticalBattle(Box::new(battle));
                                self.typing.clear();
                                label
                            }
                            "boss" => {
                                let e =
                                    Enemy::boss_from_vocab(entry, px + 1, py, self.floor_num);
                                let label = format!("Fight boss: {} ({})", e.hanzi, e.meaning);
                                self.enemies.push(e);
                                let idx = self.enemies.len() - 1;
                                let battle = combat::transition::enter_combat(
                                    &self.player,
                                    &self.enemies,
                                    &[idx],
                                    self.floor_num,
                                    self.current_room_modifier(),
                                    &self.srs,
                                    self.companion,
                                );
                                self.combat = CombatState::TacticalBattle(Box::new(battle));
                                self.typing.clear();
                                label
                            }
                            _ => format!(
                                "Unknown fight type '{}'. Use: normal, elite, boss",
                                kind
                            ),
                        }
                    }
                } else {
                    "Usage: fight <normal|elite|boss>".into()
                }
            }
            "boss" => {
                if let Some(name) = parts.get(1) {
                    let lower = name.to_lowercase();
                    let boss_kind = match lower.as_str() {
                        "piratecaptain" | "gatekeeper" => Some((BossKind::PirateCaptain, 5)),
                        "hivequeen" | "scholar" => Some((BossKind::HiveQueen, 10)),
                        "rogueaicore" | "elementalist" => Some((BossKind::RogueAICore, 15)),
                        "voidentity" | "mimicking" => Some((BossKind::VoidEntity, 20)),
                        "ancientguardian" | "inksage" => Some((BossKind::AncientGuardian, 25)),
                        "driftleviathan" | "radicalthief" => Some((BossKind::DriftLeviathan, 30)),
                        _ => None,
                    };
                    match boss_kind {
                        Some((_kind, floor)) => {
                            let pool = vocab::vocab_for_floor(floor);
                            let entry = if pool.is_empty() {
                                &vocab::VOCAB[0]
                            } else {
                                pool[self.rng_next() as usize % pool.len()]
                            };
                            let px = self.player.x;
                            let py = self.player.y;
                            let e = Enemy::boss_from_vocab(entry, px + 1, py, floor);
                            let label = format!(
                                "Boss fight: {} ({}) - {}",
                                e.hanzi,
                                e.meaning,
                                _kind.title()
                            );
                            self.enemies.push(e);
                            let idx = self.enemies.len() - 1;
                            let battle = combat::transition::enter_combat(
                                &self.player,
                                &self.enemies,
                                &[idx],
                                floor,
                                self.current_room_modifier(),
                                &self.srs,
                                self.companion,
                            );
                            self.combat = CombatState::TacticalBattle(Box::new(battle));
                            self.typing.clear();
                            label
                        }
                        None => format!(
                            "Unknown boss '{}'. Options: PirateCaptain, HiveQueen, RogueAICore, VoidEntity, AncientGuardian, DriftLeviathan",
                            name
                        ),
                    }
                } else {
                    "Usage: boss <name> (PirateCaptain/HiveQueen/RogueAICore/VoidEntity/AncientGuardian/DriftLeviathan)".into()
                }
            }
            other => {
                format!("Unknown command: '{}'. Type 'help' for commands.", other)
            }
        };
        self.console_history.push(format!("> {}", cmd));
        self.console_history.push(response);
        while self.console_history.len() > 100 {
            self.console_history.remove(0);
        }
    }

}
