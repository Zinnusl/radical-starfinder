//! Spell casting, aiming, and combo effects.

use super::*;

impl GameState {
    pub(super) fn use_spell(&mut self) {
        if let CombatState::Fighting { enemy_idx, .. } = self.combat {
            // Copy enemy position for particles before spell takes effect
            let e_screen = if enemy_idx < self.enemies.len() {
                Some(self.tile_to_screen(self.enemies[enemy_idx].x, self.enemies[enemy_idx].y))
            } else {
                None
            };
            let p_screen = self.tile_to_screen(self.player.x, self.player.y);

            if let Some(spell) = self.player.use_spell() {
                if let Some(ref audio) = self.audio {
                    audio.play_spell();
                }
                // Arcane room doubles spell damage
                let arcane_mult = if self.current_room_modifier() == Some(RoomModifier::HighTech) {
                    2
                } else {
                    1
                };
                let spell_power =
                    self.player.spell_power_bonus + self.player.spell_power_temp_bonus;
                self.player.spell_power_temp_bonus = 0;
                let current_effect = spell.effect;
                let spell_school = match current_effect {
                    SpellEffect::FireAoe(_) => Some("Fire"),
                    SpellEffect::StrongHit(_) => Some("Strike"),
                    SpellEffect::Drain(_) => Some("Drain"),
                    SpellEffect::Stun => Some("Stun"),
                    SpellEffect::Heal(_)
                    | SpellEffect::Reveal
                    | SpellEffect::Shield
                    | SpellEffect::Pacify
                    | SpellEffect::Slow(_)
                    | SpellEffect::FocusRestore(_)
                    | SpellEffect::ArmorBreak => None,
                    SpellEffect::Teleport => None,
                    SpellEffect::Poison(_, _) => Some("Poison"),
                    SpellEffect::Dash(_) => None,
                    SpellEffect::Pierce(_) => Some("Strike"),
                    SpellEffect::PullToward => None,
                    SpellEffect::KnockBack(_) => Some("Strike"),
                    SpellEffect::Thorns(_) => None,
                    SpellEffect::Cone(_) => Some("Fire"),
                    SpellEffect::Wall(_) => None,
                    SpellEffect::OilSlick
                    | SpellEffect::Ignite
                    | SpellEffect::PlantGrowth
                    | SpellEffect::Earthquake(_)
                    | SpellEffect::Sanctify(_)
                    | SpellEffect::FloodWave(_)
                    | SpellEffect::SummonBoulder
                    | SpellEffect::FreezeGround(_) => None,
                };
                let elementalist_resisted = enemy_idx < self.enemies.len()
                    && self.enemies[enemy_idx].boss_kind == Some(BossKind::RogueAICore)
                    && spell_school.is_some()
                    && self.enemies[enemy_idx].resisted_spell == spell_school;
                match spell.effect {
                    SpellEffect::FireAoe(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        // Fire particles at player position (AoE emanates from player)
                        self.particles
                            .spawn_fire(p_screen.0, p_screen.1, &mut self.rng_state);
                        // Damage all visible enemies
                        let mut killed = 0;
                        let mut boss_resisted = false;
                        for (idx, e) in self.enemies.iter_mut().enumerate() {
                            if e.is_alive() {
                                let eidx = self.level.idx(e.x, e.y);
                                if self.level.visible[eidx] {
                                    let applied_dmg = if idx == enemy_idx && elementalist_resisted {
                                        boss_resisted = true;
                                        (dmg / 2).max(1)
                                    } else {
                                        dmg
                                    };
                                    e.hp -= applied_dmg;
                                    if e.hp <= 0 {
                                        killed += 1;
                                    }
                                }
                            }
                        }
                        let oil_bonus = (1 + self.floor_num.max(1) / 4).max(1) + spell_power;
                        let (oil_tiles, scorched, oil_kills) = self.ignite_visible_oil(oil_bonus);
                        killed += oil_kills;
                        let resist_text = if boss_resisted {
                            " The Rogue AI Core dampens the repeated fire spell!"
                        } else {
                            ""
                        };
                        self.message = if oil_tiles > 0 {
                            format!(
                                "{}🔥 {} deals {} damage to all! Oil ignites on {} tiles and scorches {} foes! ({} defeated){}",
                                spell.hanzi, spell.meaning, dmg, oil_tiles, scorched, killed, resist_text
                            )
                        } else {
                            format!(
                                "{}🔥 {} deals {} damage to all! ({} defeated){}",
                                spell.hanzi, spell.meaning, dmg, killed, resist_text
                            )
                        };
                        self.message_timer = 80;
                        // If the fought enemy died, return to explore
                        if enemy_idx < self.enemies.len() && !self.enemies[enemy_idx].is_alive() {
                            self.combat = CombatState::Explore;
                            self.typing.clear();
                        }
                    }
                    SpellEffect::Heal(amount) => {
                        let amount = amount * arcane_mult + spell_power;
                        self.player.hp = (self.player.hp + amount).min(self.player.max_hp);
                        self.particles
                            .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                        self.flash = Some((60, 220, 80, 0.2));
                        self.message = format!(
                            "{} heals {} HP! (now {}/{})",
                            spell.hanzi, amount, self.player.hp, self.player.max_hp
                        );
                        self.message_timer = 60;
                    }
                    SpellEffect::Reveal => {
                        self.reveal_entire_floor();
                        self.particles
                            .spawn_teleport(p_screen.0, p_screen.1, &mut self.rng_state);
                        self.flash = Some((100, 210, 255, 0.18));
                        self.message = format!(
                            "{}👁 The dungeon's paths blaze into focus. Floor map revealed!",
                            spell.hanzi
                        );
                        self.message_timer = 70;
                    }
                    SpellEffect::Shield => {
                        self.player.shield = true;
                        self.particles
                            .spawn_shield(p_screen.0, p_screen.1, &mut self.rng_state);
                        self.message =
                            format!("{} — Shield active! Next hit will be blocked.", spell.hanzi);
                        self.message_timer = 60;
                    }
                    SpellEffect::StrongHit(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                            }
                            let applied_dmg = if elementalist_resisted {
                                (dmg / 2).max(1)
                            } else {
                                dmg
                            };
                            self.enemies[enemy_idx].hp -= applied_dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}⚔ Devastating {} damage! Defeated {}! Got [{}]{}",
                                    spell.hanzi,
                                    applied_dmg,
                                    e_hanzi,
                                    available[drop_idx].ch,
                                    if elementalist_resisted {
                                        " (partially resisted)"
                                    } else {
                                        ""
                                    }
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}⚔ {} damage to {}! ({} HP left){}",
                                    spell.hanzi,
                                    applied_dmg,
                                    e_hanzi,
                                    self.enemies[enemy_idx].hp,
                                    if elementalist_resisted {
                                        " The Rogue AI Core resists the repeated school."
                                    } else {
                                        ""
                                    }
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Drain(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_drain(ex, ey, &mut self.rng_state);
                            }
                            let applied_dmg = if elementalist_resisted {
                                (dmg / 2).max(1)
                            } else {
                                dmg
                            };
                            self.enemies[enemy_idx].hp -= applied_dmg;
                            self.player.hp = (self.player.hp + applied_dmg).min(self.player.max_hp);
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}🩸 Drained {} HP from {}! Defeated! Got [{}]{}",
                                    spell.hanzi,
                                    applied_dmg,
                                    e_hanzi,
                                    available[drop_idx].ch,
                                    if elementalist_resisted {
                                        " (partially resisted)"
                                    } else {
                                        ""
                                    }
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}🩸 Drained {} HP from {}! +{} HP ({} left){}",
                                    spell.hanzi,
                                    applied_dmg,
                                    e_hanzi,
                                    applied_dmg,
                                    self.enemies[enemy_idx].hp,
                                    if elementalist_resisted {
                                        " The Rogue AI Core resists the repeated school."
                                    } else {
                                        ""
                                    }
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Stun => {
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_stun(ex, ey, &mut self.rng_state);
                            }
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            let (water_tiles, shocked, water_kills) =
                                self.electrify_visible_water(arcane_mult + spell_power);
                            if !elementalist_resisted {
                                self.enemies[enemy_idx].stunned = true;
                            }
                            self.message = if water_tiles > 0 {
                                format!(
                                    "{}⚡ {}{} Water arcs through {} foes on {} tiles! ({} fried)",
                                    spell.hanzi,
                                    e_hanzi,
                                    if elementalist_resisted {
                                        " resists the repeated stun!"
                                    } else {
                                        " is stunned!"
                                    },
                                    shocked,
                                    water_tiles,
                                    water_kills
                                )
                            } else {
                                format!(
                                    "{}⚡ {}{}",
                                    spell.hanzi,
                                    e_hanzi,
                                    if elementalist_resisted {
                                        " resists the repeated stun!"
                                    } else {
                                        " is stunned! It will skip its next action."
                                    }
                                )
                            };
                            self.message_timer = 60;
                            if !self.enemies[enemy_idx].is_alive() {
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            }
                        }
                    }
                    SpellEffect::Pacify => {
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_shield(ex, ey, &mut self.rng_state);
                            }
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].is_boss {
                                self.enemies[enemy_idx].stunned = true;
                                self.flash = Some((150, 190, 255, 0.14));
                                self.message = format!(
                                    "{}☯ {} resists a full truce, but falters and loses its next action.",
                                    spell.hanzi, e_hanzi
                                );
                                self.message_timer = 80;
                            } else {
                                let gold = Self::pacify_gold_reward(
                                    self.enemies[enemy_idx].gold_value,
                                    spell_power,
                                );
                                self.player.gold += gold;
                                self.enemies[enemy_idx].hp = 0;
                                self.flash = Some((120, 220, 190, 0.18));
                                self.message = format!(
                                    "{}☯ You reason with {}. It withdraws peacefully and leaves {} gold behind.",
                                    spell.hanzi, e_hanzi, gold
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            }
                        }
                    }
                    SpellEffect::Slow(turns) => {
                        if enemy_idx < self.enemies.len() {
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            self.enemies[enemy_idx].stunned = true; // overworld: stun as proxy for slow
                            self.flash = Some((150, 200, 255, 0.15));
                            self.message = format!(
                                "{}🐌 {} is slowed for {} turns! It loses its next action.",
                                spell.hanzi, e_hanzi, turns
                            );
                            self.message_timer = 60;
                        }
                    }
                    SpellEffect::Teleport => {
                        // Overworld: blink past the enemy
                        self.flash = Some((100, 210, 255, 0.2));
                        self.message = format!(
                            "{}💨 You vanish in a gust of wind and reappear safely!",
                            spell.hanzi
                        );
                        self.message_timer = 60;
                        self.combat = CombatState::Explore;
                        self.typing.clear();
                    }
                    SpellEffect::Poison(dmg, turns) => {
                        if enemy_idx < self.enemies.len() {
                            let applied_dmg = dmg * turns * arcane_mult + spell_power;
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_drain(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= applied_dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}☠ Venom courses through {}! {} poison damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, applied_dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}☠ Poison seeps into {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, applied_dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::FocusRestore(amt) => {
                        let amt = amt * arcane_mult + spell_power;
                        self.player.hp = (self.player.hp + amt).min(self.player.max_hp);
                        self.particles
                            .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                        self.flash = Some((80, 180, 255, 0.15));
                        self.message = format!(
                            "{}🧘 Mental focus restored! +{} HP (now {}/{})",
                            spell.hanzi, amt, self.player.hp, self.player.max_hp
                        );
                        self.message_timer = 60;
                    }
                    SpellEffect::ArmorBreak => {
                        if enemy_idx < self.enemies.len() {
                            let dmg = 2 * arcane_mult + spell_power;
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}💥 Armor-breaking strike shatters {}! {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}💥 Armor-breaking force hits {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Dash(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}💨 Charged through {}! {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}💨 Dashing strike hits {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        } else {
                            self.flash = Some((100, 210, 255, 0.2));
                            self.message = format!("{}💨 You dash past in a blur!", spell.hanzi);
                            self.message_timer = 60;
                            self.combat = CombatState::Explore;
                            self.typing.clear();
                        }
                    }
                    SpellEffect::Pierce(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_damage(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}🔱 Piercing bolt skewers {}! {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}🔱 Piercing bolt hits {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::PullToward => {
                        if enemy_idx < self.enemies.len() {
                            self.enemies[enemy_idx].stunned = true;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            self.message = format!(
                                "{}🧲 {} is yanked toward you and dazed!",
                                spell.hanzi, e_hanzi
                            );
                            self.message_timer = 60;
                        }
                    }
                    SpellEffect::KnockBack(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_damage(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}🤜 {} sent flying! {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}🤜 {} knocked back for {} damage! ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Thorns(turns) => {
                        self.player.shield = true;
                        self.particles
                            .spawn_shield(p_screen.0, p_screen.1, &mut self.rng_state);
                        self.message = format!(
                            "{}🌿 Thorns grow around you for {} turns!",
                            spell.hanzi, turns
                        );
                        self.message_timer = 60;
                    }
                    SpellEffect::Cone(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_fire(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}🔺 Cone blast engulfs {}! {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}🔺 Cone blast hits {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Wall(_) => {
                        self.flash = Some((160, 140, 100, 0.15));
                        self.message =
                            format!("{}🧱 A wall of stone erupts from the ground!", spell.hanzi);
                        self.message_timer = 60;
                    }
                    SpellEffect::OilSlick => {
                        self.flash = Some((80, 60, 20, 0.15));
                        self.message =
                            format!("{}🛢 Oil spreads across the ground!", spell.hanzi);
                        self.message_timer = 60;
                    }
                    SpellEffect::FreezeGround(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_damage(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}🧊 Ice encases {}! {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}🧊 Ground freezes beneath {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Ignite => {
                        self.flash = Some((255, 120, 30, 0.2));
                        self.message =
                            format!("{}🔥 Flames erupt and spread!", spell.hanzi);
                        self.message_timer = 60;
                    }
                    SpellEffect::PlantGrowth => {
                        self.flash = Some((60, 180, 60, 0.15));
                        let healed = 1_i32.min(self.player.max_hp - self.player.hp);
                        self.player.hp = (self.player.hp + 1).min(self.player.max_hp);
                        self.message = format!(
                            "{}🌿 Nature blooms! +{} HP",
                            spell.hanzi, healed
                        );
                        self.message_timer = 60;
                    }
                    SpellEffect::Earthquake(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_damage(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}💥 The earth shakes! {} crushed for {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}💥 Earthquake hits {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::Sanctify(heal) => {
                        let heal = heal * arcane_mult + spell_power;
                        self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                        self.particles
                            .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                        self.flash = Some((255, 215, 80, 0.2));
                        self.message = format!(
                            "{}✨ Holy light purifies! +{} HP",
                            spell.hanzi, heal
                        );
                        self.message_timer = 60;
                    }
                    SpellEffect::FloodWave(dmg) => {
                        let dmg = dmg * arcane_mult + spell_power;
                        if enemy_idx < self.enemies.len() {
                            if let Some((ex, ey)) = e_screen {
                                self.particles.spawn_damage(ex, ey, &mut self.rng_state);
                            }
                            self.enemies[enemy_idx].hp -= dmg;
                            let e_hanzi = self.enemies[enemy_idx].hanzi;
                            if self.enemies[enemy_idx].hp <= 0 {
                                let available = radical::radicals_for_floor(self.floor_num);
                                let drop_idx = self.rng_next() as usize % available.len();
                                self.player.add_radical(available[drop_idx].ch);
                                self.message = format!(
                                    "{}🌊 Flood wave sweeps away {}! {} damage! Defeated! Got [{}]",
                                    spell.hanzi, e_hanzi, dmg, available[drop_idx].ch
                                );
                                self.message_timer = 80;
                                self.combat = CombatState::Explore;
                                self.typing.clear();
                            } else {
                                self.message = format!(
                                    "{}🌊 Flood wave hits {}! {} damage ({} HP left)",
                                    spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                                );
                                self.message_timer = 60;
                            }
                        }
                    }
                    SpellEffect::SummonBoulder => {
                        self.flash = Some((140, 120, 90, 0.15));
                        self.message =
                            format!("{}🪨 A boulder materializes!", spell.hanzi);
                        self.message_timer = 60;
                    }
                }

                if enemy_idx < self.enemies.len()
                    && self.enemies[enemy_idx].boss_kind == Some(BossKind::RogueAICore)
                {
                    if let Some(school) = spell_school {
                        self.enemies[enemy_idx].resisted_spell = Some(school);
                    }
                }

                // ── Combo detection ─────────────────────────────────────
                if let Some(prev) = self.last_spell.take() {
                    let combo = detect_combo(&prev, &current_effect);
                    if let Some((combo_name, combo_effect)) = combo {
                        self.apply_combo(enemy_idx, &combo_name, combo_effect, p_screen, e_screen);
                    }
                }
                self.last_spell = Some(current_effect);
                self.spell_combo_timer = 3;
                if matches!(self.combat, CombatState::Fighting { .. })
                    && self.maybe_trigger_boss_phase(enemy_idx)
                {
                    self.typing.clear();
                }
            } else {
                self.message = "No spells available!".to_string();
                self.message_timer = 40;
            }
        }
    }

    /// Use a spell during exploration (Q to cycle, Space to cast).
    /// Only utility spells (Heal, Shield, Reveal) work outside combat.
    pub(super) fn use_spell_explore(&mut self) {
        if !matches!(self.combat, CombatState::Explore) {
            return;
        }
        if self.player.selected_spell >= self.player.spells.len() {
            self.message = "No spells available!".to_string();
            self.message_timer = 40;
            return;
        }
        let effect = self.player.spells[self.player.selected_spell].effect;
        match effect {
            SpellEffect::Heal(_)
            | SpellEffect::Shield
            | SpellEffect::Reveal
            | SpellEffect::Thorns(_) => {}
            _ => {
                // Offensive spell — enter aiming mode instead of rejecting
                let label = effect.label();
                self.combat = CombatState::Aiming {
                    spell_idx: self.player.selected_spell,
                    dx: 0,
                    dy: -1,
                };
                self.message = format!("{} — Aim with arrows, Enter to fire, Esc cancel", label);
                self.message_timer = 120;
                return;
            }
        }
        let spell = self.player.use_spell().unwrap();
        if let Some(ref audio) = self.audio {
            audio.play_spell();
        }
        let p_screen = self.tile_to_screen(self.player.x, self.player.y);
        let spell_power = self.player.spell_power_bonus + self.player.spell_power_temp_bonus;
        self.player.spell_power_temp_bonus = 0;
        let arcane_mult = if self.current_room_modifier() == Some(RoomModifier::HighTech) {
            2
        } else {
            1
        };
        match spell.effect {
            SpellEffect::Heal(amount) => {
                let amount = amount * arcane_mult + spell_power;
                self.player.hp = (self.player.hp + amount).min(self.player.max_hp);
                self.particles
                    .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                self.flash = Some((60, 220, 80, 0.2));
                self.message = format!(
                    "{} heals {} HP! (now {}/{})",
                    spell.hanzi, amount, self.player.hp, self.player.max_hp
                );
                self.message_timer = 60;
            }
            SpellEffect::Reveal => {
                self.reveal_entire_floor();
                self.particles
                    .spawn_teleport(p_screen.0, p_screen.1, &mut self.rng_state);
                self.flash = Some((100, 210, 255, 0.18));
                self.message = format!(
                    "{}👁 The dungeon's paths blaze into focus. Floor map revealed!",
                    spell.hanzi
                );
                self.message_timer = 70;
            }
            SpellEffect::Shield => {
                self.player.shield = true;
                self.particles
                    .spawn_shield(p_screen.0, p_screen.1, &mut self.rng_state);
                self.message =
                    format!("{} — Shield active! Next hit will be blocked.", spell.hanzi);
                self.message_timer = 60;
            }
            SpellEffect::Thorns(turns) => {
                self.player.shield = true;
                self.particles
                    .spawn_shield(p_screen.0, p_screen.1, &mut self.rng_state);
                self.message = format!("{}🌿 Thorns active for {} turns!", spell.hanzi, turns);
                self.message_timer = 60;
            }
            _ => unreachable!(),
        }
    }

    pub(super) fn fire_aimed_spell(&mut self, spell_idx: usize, dx: i32, dy: i32) {
        if spell_idx >= self.player.spells.len() {
            self.combat = CombatState::Explore;
            return;
        }
        let effect = self.player.spells[spell_idx].effect;
        let arcane_mult = if self.current_room_modifier() == Some(RoomModifier::HighTech) {
            2
        } else {
            1
        };
        let spell_power = self.player.spell_power_bonus + self.player.spell_power_temp_bonus;
        self.player.spell_power_temp_bonus = 0;

        let mut cx = self.player.x + dx;
        let mut cy = self.player.y + dy;
        let mut hit_enemy: Option<usize> = None;
        let max_range = 10;
        for _ in 0..max_range {
            if !self.level.in_bounds(cx, cy)
                || !self.level.tiles[self.level.idx(cx, cy)].is_walkable()
            {
                break;
            }
            if let Some(idx) = self.enemy_at(cx, cy) {
                hit_enemy = Some(idx);
                break;
            }
            cx += dx;
            cy += dy;
        }

        let spell = self.player.use_spell().unwrap();
        if let Some(ref audio) = self.audio {
            audio.play_spell();
        }
        let p_screen = self.tile_to_screen(self.player.x, self.player.y);

        if let Some(enemy_idx) = hit_enemy {
            let e_screen =
                self.tile_to_screen(self.enemies[enemy_idx].x, self.enemies[enemy_idx].y);
            match effect {
                SpellEffect::FireAoe(dmg) => {
                    let dmg = dmg * arcane_mult + spell_power;
                    self.particles
                        .spawn_fire(e_screen.0, e_screen.1, &mut self.rng_state);
                    self.enemies[enemy_idx].hp -= dmg;
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    if self.enemies[enemy_idx].hp <= 0 {
                        self.message = format!(
                            "{}🔥 {} takes {} damage and is defeated!",
                            spell.hanzi, e_hanzi, dmg
                        );
                    } else {
                        self.message = format!(
                            "{}🔥 {} takes {} damage! ({} HP left)",
                            spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                        );
                    }
                }
                SpellEffect::StrongHit(dmg) => {
                    let dmg = dmg * arcane_mult + spell_power;
                    self.particles
                        .spawn_damage(e_screen.0, e_screen.1, &mut self.rng_state);
                    self.enemies[enemy_idx].hp -= dmg;
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    if self.enemies[enemy_idx].hp <= 0 {
                        self.message = format!(
                            "{}⚔ {} takes {} damage and is defeated!",
                            spell.hanzi, e_hanzi, dmg
                        );
                    } else {
                        self.message = format!(
                            "{}⚔ {} takes {} damage! ({} HP left)",
                            spell.hanzi, e_hanzi, dmg, self.enemies[enemy_idx].hp
                        );
                    }
                }
                SpellEffect::Drain(dmg) => {
                    let dmg = dmg * arcane_mult + spell_power;
                    self.particles
                        .spawn_damage(e_screen.0, e_screen.1, &mut self.rng_state);
                    self.enemies[enemy_idx].hp -= dmg;
                    self.player.hp = (self.player.hp + dmg).min(self.player.max_hp);
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    if self.enemies[enemy_idx].hp <= 0 {
                        self.message = format!(
                            "{}🩸 Drained {} from {} — foe defeated! +{} HP",
                            spell.hanzi, dmg, e_hanzi, dmg
                        );
                    } else {
                        self.message = format!(
                            "{}🩸 Drained {} from {}! +{} HP ({} HP left)",
                            spell.hanzi, dmg, e_hanzi, dmg, self.enemies[enemy_idx].hp
                        );
                    }
                }
                SpellEffect::Stun => {
                    self.enemies[enemy_idx].stunned = true;
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    self.particles
                        .spawn_damage(e_screen.0, e_screen.1, &mut self.rng_state);
                    self.message = format!("{}⚡ {} is stunned!", spell.hanzi, e_hanzi);
                }
                SpellEffect::Pacify => {
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    if self.enemies[enemy_idx].is_boss {
                        self.enemies[enemy_idx].stunned = true;
                        self.message = format!(
                            "{}☯ {} resists pacification but is dazed!",
                            spell.hanzi, e_hanzi
                        );
                    } else {
                        self.enemies[enemy_idx].hp = 0;
                        self.message =
                            format!("{}☯ {} stands down peacefully.", spell.hanzi, e_hanzi);
                    }
                }
                SpellEffect::Pierce(dmg) | SpellEffect::KnockBack(dmg) | SpellEffect::Cone(dmg) => {
                    let dmg = dmg * arcane_mult + spell_power;
                    self.particles
                        .spawn_damage(e_screen.0, e_screen.1, &mut self.rng_state);
                    self.enemies[enemy_idx].hp -= dmg;
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    let icon = match effect {
                        SpellEffect::Pierce(_) => "🔱",
                        SpellEffect::KnockBack(_) => "🤜",
                        SpellEffect::Cone(_) => "🔺",
                        _ => "",
                    };
                    if self.enemies[enemy_idx].hp <= 0 {
                        self.message = format!(
                            "{}{} {} takes {} damage and is defeated!",
                            spell.hanzi, icon, e_hanzi, dmg
                        );
                    } else {
                        self.message = format!(
                            "{}{} {} takes {} damage! ({} HP left)",
                            spell.hanzi, icon, e_hanzi, dmg, self.enemies[enemy_idx].hp
                        );
                    }
                }
                SpellEffect::PullToward => {
                    self.enemies[enemy_idx].stunned = true;
                    let e_hanzi = self.enemies[enemy_idx].hanzi;
                    self.message =
                        format!("{}🧲 {} is pulled closer and dazed!", spell.hanzi, e_hanzi);
                }
                _ => {}
            }
            self.flash = Some((255, 200, 100, 0.15));
            self.shake_timer = 4;
        } else {
            self.particles
                .spawn_fire(p_screen.0, p_screen.1, &mut self.rng_state);
            self.message = format!("{} flies off but hits nothing.", spell.hanzi);
        }
        self.message_timer = 80;
        self.combat = CombatState::Explore;
    }

    /// Apply a spell combo bonus.
    pub(super) fn apply_combo(
        &mut self,
        enemy_idx: usize,
        name: &str,
        effect: ComboEffect,
        p_screen: (f64, f64),
        e_screen: Option<(f64, f64)>,
    ) {
        // Flash gold for combo
        self.flash = Some((255, 200, 50, 0.3));
        match effect {
            ComboEffect::Steam => {
                // AoE stun all visible enemies
                for e in &mut self.enemies {
                    if e.is_alive() {
                        e.stunned = true;
                    }
                }
                self.particles
                    .spawn_fire(p_screen.0, p_screen.1 - 10.0, &mut self.rng_state);
                self.particles
                    .spawn_shield(p_screen.0, p_screen.1 + 10.0, &mut self.rng_state);
                self.message = format!("💥 COMBO: {}! All enemies stunned!", name);
            }
            ComboEffect::Counter(dmg) => {
                // Reflect damage + shield
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].hp -= dmg;
                    self.player.shield = true;
                    if let Some((ex, ey)) = e_screen {
                        self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                    }
                }
                self.message = format!("💥 COMBO: {}! {} reflected + Shield!", name, dmg);
            }
            ComboEffect::Barrier(amount) => {
                // Strong shield + heal
                self.player.shield = true;
                self.player.hp = (self.player.hp + amount).min(self.player.max_hp);
                self.particles
                    .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                self.particles
                    .spawn_shield(p_screen.0, p_screen.1, &mut self.rng_state);
                self.message = format!("💥 COMBO: {}! Shield + {} HP!", name, amount);
            }
            ComboEffect::Flurry(dmg) => {
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].hp -= dmg;
                    if let Some((ex, ey)) = e_screen {
                        self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                        self.particles.spawn_fire(ex, ey, &mut self.rng_state);
                    }
                }
                self.message = format!("💥 COMBO: {}! {} damage flurry!", name, dmg);
            }
            ComboEffect::Ignite(dmg) => {
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].hp -= dmg;
                    let heal = dmg / 2;
                    self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                    if let Some((ex, ey)) = e_screen {
                        self.particles.spawn_fire(ex, ey, &mut self.rng_state);
                    }
                    self.particles
                        .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                }
                self.message = format!("💥 COMBO: {}! {} fire damage + lifesteal!", name, dmg);
            }
            ComboEffect::Tempest(dmg) => {
                for e in &mut self.enemies {
                    if e.is_alive() {
                        e.hp -= dmg;
                    }
                }
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].stunned = true;
                }
                self.particles
                    .spawn_fire(p_screen.0, p_screen.1, &mut self.rng_state);
                self.message = format!("💥 COMBO: {}! {} AoE damage + stun!", name, dmg);
            }
            ComboEffect::Rally(dmg) => {
                let heal = dmg / 2;
                self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].hp -= dmg;
                    if let Some((ex, ey)) = e_screen {
                        self.particles.spawn_kill(ex, ey, &mut self.rng_state);
                    }
                }
                self.particles
                    .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                self.message = format!("💥 COMBO: {}! {} damage + {} HP!", name, dmg, heal);
            }
            ComboEffect::Siphon(dmg) => {
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].hp -= dmg;
                    self.enemies[enemy_idx].stunned = true;
                    let heal = dmg;
                    self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                    if let Some((ex, ey)) = e_screen {
                        self.particles.spawn_damage(ex, ey, &mut self.rng_state);
                    }
                    self.particles
                        .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                }
                self.message = format!("💥 COMBO: {}! {} drained + stun!", name, dmg);
            }
            ComboEffect::Fortify(amount) => {
                self.player.shield = true;
                if enemy_idx < self.enemies.len() {
                    self.enemies[enemy_idx].hp -= amount;
                    if let Some((ex, ey)) = e_screen {
                        self.particles.spawn_damage(ex, ey, &mut self.rng_state);
                    }
                }
                self.player.hp = (self.player.hp + amount).min(self.player.max_hp);
                self.particles
                    .spawn_shield(p_screen.0, p_screen.1, &mut self.rng_state);
                self.particles
                    .spawn_heal(p_screen.0, p_screen.1, &mut self.rng_state);
                self.message = format!(
                    "💥 COMBO: {}! Shield + {} damage + {} HP!",
                    name, amount, amount
                );
            }
        }
        self.message_timer = 100;
        // Check if fought enemy died from combo
        if enemy_idx < self.enemies.len() && !self.enemies[enemy_idx].is_alive() {
            self.combat = CombatState::Explore;
            self.typing.clear();
        }
    }
}
