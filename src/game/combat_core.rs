//! Core combat: enemy turns, answer submission, radical actions.

use super::*;

impl GameState {
    pub(super) fn enemy_turn(&mut self) {
        let px = self.player.x;
        let py = self.player.y;
        let mut summons = Vec::new();
        let mut summon_message = None;

        for i in 0..self.enemies.len() {
            if !self.enemies[i].is_alive() {
                continue;
            }
            // Stunned enemies skip their turn
            if self.enemies[i].stunned {
                self.enemies[i].stunned = false;
                continue;
            }
            // Confused enemies have a 50% chance to skip their turn
            if status::has_confused(&self.enemies[i].statuses) && self.rng_next() % 2 == 0 {
                continue;
            }
            // Alert if within FOV radius
            let dist_sq = (self.enemies[i].x - px).pow(2) + (self.enemies[i].y - py).pow(2);
            if dist_sq <= (FOV_RADIUS * FOV_RADIUS) {
                self.enemies[i].alert = true;
            }
            if !self.enemies[i].alert {
                continue;
            }

            if self.enemies[i].boss_kind == Some(BossKind::PirateCaptain) {
                if self.enemies[i].summon_cooldown > 0 {
                    self.enemies[i].summon_cooldown -= 1;
                }
                let nearby_minions = self
                    .enemies
                    .iter()
                    .enumerate()
                    .filter(|(j, e)| {
                        *j != i
                            && e.is_alive()
                            && !e.is_boss
                            && (e.x - self.enemies[i].x).abs() <= 1
                            && (e.y - self.enemies[i].y).abs() <= 1
                    })
                    .count();
                if nearby_minions < 2 && self.enemies[i].summon_cooldown == 0 {
                    if let Some((sx, sy)) =
                        self.find_free_adjacent_tile(self.enemies[i].x, self.enemies[i].y)
                    {
                        let entry = Self::vocab_entry_by_hanzi("门")
                            .or_else(|| Self::vocab_entry_by_hanzi("人"))
                            .unwrap_or(&vocab::VOCAB[0]);
                        let mut ward = Enemy::from_vocab(entry, sx, sy, self.floor_num.max(2));
                        ward.alert = true;
                        ward.gold_value += 4;
                        summons.push(ward);
                        self.enemies[i].summon_cooldown = 3;
                        let visible_idx = self.level.idx(self.enemies[i].x, self.enemies[i].y);
                        if self.level.visible[visible_idx] {
                            summon_message =
                                Some("🚪 The Pirate Captain deploys a shield drone!".to_string());
                        }
                    }
                }
            }

            if self.enemies[i].boss_kind == Some(BossKind::VoidEntity) {
                if self.enemies[i].summon_cooldown > 0 {
                    self.enemies[i].summon_cooldown -= 1;
                }
                let nearby_minions = self
                    .enemies
                    .iter()
                    .enumerate()
                    .filter(|(j, e)| *j != i && e.is_alive() && !e.is_boss)
                    .count();
                if nearby_minions < 3 && self.enemies[i].summon_cooldown == 0 {
                    if let Some((sx, sy)) =
                        self.find_free_adjacent_tile(self.enemies[i].x, self.enemies[i].y)
                    {
                        let mimic_pool = vocab::vocab_for_floor(self.floor_num);
                        if !mimic_pool.is_empty() {
                            let entry = mimic_pool[self.rng_next() as usize % mimic_pool.len()];
                            let mut mimic = Enemy::from_vocab(entry, sx, sy, self.floor_num);
                            mimic.alert = true;
                            mimic.gold_value += 8;
                            summons.push(mimic);
                            self.enemies[i].summon_cooldown = 2;
                            let visible_idx = self.level.idx(self.enemies[i].x, self.enemies[i].y);
                            if self.level.visible[visible_idx] {
                                summon_message =
                                    Some("👑 The Void Entity warps in a doppelgänger!".to_string());
                            }
                        }
                    }
                }
            }

            let (ex, ey) = (self.enemies[i].x, self.enemies[i].y);
            let nearby_allies = self
                .enemies
                .iter()
                .enumerate()
                .filter(|(j, e)| {
                    *j != i && e.is_alive() && e.alert && (e.x - ex).abs() + (e.y - ey).abs() <= 3
                })
                .count();
            let (nx, ny) = self.enemies[i].ai_step(px, py, nearby_allies);

            // Don't walk into walls or other enemies
            if !self.level.is_walkable(nx, ny) {
                continue;
            }
            // Don't stack on other enemies
            let occupied = self
                .enemies
                .iter()
                .enumerate()
                .any(|(j, e)| j != i && e.is_alive() && e.x == nx && e.y == ny);
            if occupied {
                continue;
            }

            // If enemy walks into player → start combat (same as player bumping enemy)
            if nx == px && ny == py {
                if !matches!(
                    self.combat,
                    CombatState::Fighting { .. } | CombatState::TacticalBattle(_)
                ) {
                    self.trigger_shake(4);
                    let battle = combat::transition::enter_combat(
                        &self.player,
                        &self.enemies,
                        &[i],
                        self.floor_num,
                        self.current_room_modifier(),
                        &self.srs,
                        self.companion,
                    );
                    self.combat = CombatState::TacticalBattle(Box::new(battle));
                    self.typing.clear();
                    self.message = format!(
                        "{} attacks! {}",
                        self.enemies[i].hanzi,
                        combat_prompt_for(&self.enemies[i], self.listening_mode, self.mirror_hint)
                    );
                    if let Some(ref comp) = self.companion {
                        let lvl = self.companion_level();
                        if let Some(hint) = comp.contextual_hint(
                            &self.enemies[i],
                            self.player.hp,
                            self.player.max_hp,
                            self.guard_used_this_fight,
                            lvl,
                        ) {
                            self.message.push_str(&format!("\n{}", hint));
                        }
                    }
                    self.message_timer = 255;
                }
                continue;
            }

            self.enemies[i].x = nx;
            self.enemies[i].y = ny;
            self.apply_enemy_tile_effect(i);
        }

        if !summons.is_empty() {
            self.enemies.extend(summons);
            if let Some(message) = summon_message {
                self.message = message;
                self.message_timer = 90;
            }
        }
    }

    /// Handle typing a character during combat.
    pub(super) fn type_char(&mut self, ch: char) {
        if matches!(self.combat, CombatState::GameOver) {
            return;
        }
        if let CombatState::Fighting { .. } = &self.combat {
            self.typing.push(ch);
        }
    }

    /// Submit pinyin answer.
    pub(super) fn submit_answer(&mut self) {
        if let CombatState::Fighting { enemy_idx, .. } = self.combat.clone() {
            if enemy_idx >= self.enemies.len() {
                self.combat = CombatState::Explore;
                return;
            }

            // Component (Shield) Logic
            if !self.enemies[enemy_idx].components.is_empty() {
                let comp_hanzi = self.enemies[enemy_idx].components[0];
                let comp_pinyin = vocab::vocab_entry_by_hanzi(comp_hanzi)
                    .map(|e| e.pinyin)
                    .unwrap_or("???");

                let matches = if let Some(entry) = vocab::vocab_entry_by_hanzi(comp_hanzi) {
                    vocab::check_pinyin(entry, &self.typing)
                } else {
                    self.typing == comp_pinyin
                };

                if matches {
                    self.enemies[enemy_idx].components.remove(0);
                    self.typing.clear();
                    self.trigger_shake(2);
                    if let Some(ref audio) = self.audio {
                        audio.play_hit();
                    }

                    if self.enemies[enemy_idx].components.is_empty() {
                        self.message = format!("The {} is exposed!", self.enemies[enemy_idx].hanzi);
                    } else {
                        self.message = format!("Shattered {} shield!", comp_hanzi);
                    }
                    self.message_timer = 60;
                    return;
                } else {
                    self.message = format!("Shield holds! Need {}", comp_pinyin);
                    self.message_timer = 60;
                    self.typing.clear();
                    return;
                }
            }

            let e_hanzi = self.enemies[enemy_idx].hanzi;
            let e_pinyin = self.enemies[enemy_idx].pinyin;
            let e_meaning = self.enemies[enemy_idx].meaning;
            let e_damage = (self.enemies[enemy_idx].damage
                - self.player.damage_reduction()
                - self.player.enchant_damage_reduction())
            .max(1);
            let e_gold = self.enemies[enemy_idx].gold_value
                + self.player.gold_bonus()
                + self.player.enchant_gold_bonus();
            let e_is_boss = self.enemies[enemy_idx].is_boss;
            let e_is_elite = self.enemies[enemy_idx].is_elite;
            let e_x = self.enemies[enemy_idx].x;
            let e_y = self.enemies[enemy_idx].y;

            // ToneOnly mode: accept just the tone number (1-4) for non-elite
            let tone_only_active = self.listening_mode == ListenMode::ToneOnly && !e_is_elite;

            let elite_step = if e_is_elite {
                Some(vocab::resolve_compound_pinyin_step(
                    e_pinyin,
                    self.enemies[enemy_idx].elite_chain,
                    &self.typing,
                ))
            } else {
                None
            };
            let answer_correct = if tone_only_active {
                let expected_tone = e_pinyin
                    .chars()
                    .last()
                    .and_then(|c| c.to_digit(10))
                    .unwrap_or(1);
                self.typing
                    .trim()
                    .parse::<u32>()
                    .map(|t| t == expected_tone)
                    .unwrap_or(false)
            } else if let Some(step) = elite_step {
                !matches!(step, vocab::CompoundPinyinStep::Miss { .. })
            } else {
                vocab::check_pinyin(
                    &vocab::VocabEntry {
                        hanzi: e_hanzi,
                        pinyin: e_pinyin,
                        meaning: e_meaning,
                        hsk: 1,
                        example: "",
                    },
                    &self.typing,
                )
            };

            if answer_correct {
                self.srs.record(e_hanzi, true);
                self.codex.record(e_hanzi, e_pinyin, e_meaning, true);
                self.run_correct_answers += 1;
                // ResearchLab: double vocab XP
                if self.current_location_type == Some(crate::world::LocationType::ResearchLab) {
                    self.srs.record(e_hanzi, true);
                    self.run_correct_answers += 1;
                }
                // Hit with bonus damage from equipment + room modifiers
                let cursed_bonus = if self.current_room_modifier() == Some(RoomModifier::Irradiated) {
                    1
                } else {
                    0
                };
                let warrior_bonus = if self.player.class == PlayerClass::Soldier {
                    1
                } else {
                    0
                };
                let tone_bonus = self.player.tone_bonus_damage;
                self.player.tone_bonus_damage = 0; // consumed

                let form_bonus = match self.player.form {
                    PlayerForm::Void => 2,
                    PlayerForm::Powered => 1,
                    _ => 0,
                };
                let empowered_bonus = status::empowered_amount(&self.player.statuses);

                let iron_bonus = if self.player.get_piety(Faction::MilitaryAlliance) >= 10 {
                    1
                } else {
                    0
                };
                let tactical_insight = if e_is_elite
                    && self.player.get_piety(Faction::Technocracy) >= 10
                    && self.player.get_piety(Faction::MilitaryAlliance) >= 10
                {
                    2
                } else {
                    0
                };

                let security_bonus = self.crew_bonus(CrewRole::SecurityChief);

                let hit_dmg = 2
                    + self.player.bonus_damage()
                    + self.player.enchant_bonus_damage()
                    + cursed_bonus
                    + warrior_bonus
                    + tone_bonus
                    + form_bonus
                    + empowered_bonus
                    + iron_bonus
                    + tactical_insight
                    + security_bonus;

                self.answer_streak += 1;
                if self.answer_streak > self.run_journal.max_combo {
                    self.run_journal.max_combo = self.answer_streak;
                    if self.answer_streak >= 5 {
                        self.run_journal
                            .log(RunEvent::ComboAchieved(self.answer_streak, self.floor_num));
                    }
                }
                let multiplier = combo_tier(self.answer_streak).multiplier();
                let hit_dmg = ((hit_dmg as f64) * multiplier).round() as i32;
                // Cursed status: reduce damage dealt by 25%
                let hit_dmg = if status::has_cursed(&self.player.statuses) {
                    (hit_dmg * 3 / 4).max(1)
                } else {
                    hit_dmg
                };
                // Revealed enemies take +25% damage
                let hit_dmg = if status::has_revealed(&self.enemies[enemy_idx].statuses) {
                    (hit_dmg * 5 / 4).max(hit_dmg + 1)
                } else {
                    hit_dmg
                };
                let hit_dmg = hit_dmg.max(1);

                if self.answer_streak == 5 || self.answer_streak == 10 {
                    let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
                    self.particles.spawn_streak(sx, sy, &mut self.rng_state);
                    if let Some(ref audio) = self.audio {
                        audio.play_streak_ding();
                    }
                }

                // Status application
                if status::has_envenomed(&self.player.statuses) {
                    self.enemies[enemy_idx]
                        .statuses
                        .push(status::StatusInstance::new(
                            status::StatusKind::Poison { damage: 2 },
                            3,
                        ));
                }
                if self.player.form == PlayerForm::Powered {
                    self.enemies[enemy_idx]
                        .statuses
                        .push(status::StatusInstance::new(
                            status::StatusKind::Burn { damage: 1 },
                            3,
                        ));
                }
                // Attacking from Invisible breaks cloak and applies Revealed
                if status::has_invisible(&self.player.statuses) {
                    self.player
                        .statuses
                        .retain(|s| !matches!(s.kind, status::StatusKind::Invisible));
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Revealed,
                        3,
                    ));
                }

                let mut dealt_dmg = hit_dmg;
                let mut elite_completed_cycle = false;
                let mut elite_message: Option<String> = None;
                if let Some(step) = elite_step {
                    match step {
                        vocab::CompoundPinyinStep::Advanced {
                            matched,
                            next_progress,
                            total,
                        } => {
                            dealt_dmg = elite_chain_damage(hit_dmg, total, false);
                            self.enemies[enemy_idx].elite_chain = next_progress;
                            self.enemies[enemy_idx].hp =
                                elite_remaining_hp(self.enemies[enemy_idx].hp, dealt_dmg, false);
                            let next_expected = self.enemies[enemy_idx]
                                .elite_expected_syllable()
                                .unwrap_or(e_pinyin);
                            elite_message = Some(format!(
                                "⛓ {} clicks! Chain {}/{} — next: {}",
                                matched, next_progress, total, next_expected
                            ));
                        }
                        vocab::CompoundPinyinStep::Completed { matched, total } => {
                            dealt_dmg = elite_chain_damage(hit_dmg, total, true);
                            self.enemies[enemy_idx].elite_chain = 0;
                            self.enemies[enemy_idx].hp -= dealt_dmg;
                            elite_completed_cycle = true;
                            if self.enemies[enemy_idx].hp > 0 {
                                self.enemies[enemy_idx].stunned = true;
                                elite_message = Some(format!(
                                    "✦ Compound broken with {}! {} takes {} damage and staggers.",
                                    matched, e_hanzi, dealt_dmg
                                ));
                            }
                        }
                        vocab::CompoundPinyinStep::Miss { .. } => {}
                    }
                } else {
                    if self.enemies[enemy_idx].radical_dodge {
                        self.enemies[enemy_idx].radical_dodge = false;
                        self.message =
                            format!("🌙 {} dodges your attack with Shadow Step!", e_hanzi);
                        self.message_timer = 60;
                        self.typing.clear();
                        return;
                    }

                    let armor = self.enemies[enemy_idx].radical_armor;
                    if armor > 0 {
                        self.enemies[enemy_idx].radical_armor = 0;
                        dealt_dmg = (dealt_dmg - armor).max(1);
                    }

                    self.enemies[enemy_idx].hp -= dealt_dmg;
                }
                if self.enemies[enemy_idx].hp <= 0 {
                    self.total_kills += 1;
                    self.run_kills += 1;
                    self.run_gold_earned += e_gold;
                    if e_is_boss {
                        self.run_bosses_killed += 1;
                        self.run_journal
                            .log(RunEvent::BossKilled(e_hanzi.to_string(), self.floor_num));
                    } else {
                        self.run_journal
                            .log(RunEvent::EnemyKilled(e_hanzi.to_string(), self.floor_num));
                    }
                    if let Some(ref audio) = self.audio {
                        audio.play_kill();
                    }
                    // Kill particles
                    let (sx, sy) = self.tile_to_screen(e_x, e_y);
                    self.particles.spawn_kill(sx, sy, &mut self.rng_state);
                    self.flash = Some((255, 255, 255, 0.3));
                    // Rewards
                    let mut gold_gain = e_gold;
                    if self.player.get_piety(Faction::MilitaryAlliance) >= 10
                        && self.player.get_piety(Faction::AncientOrder) >= 10
                    {
                        gold_gain *= 2;
                    }
                    if self.player.get_piety(Faction::AncientOrder) >= 10 {
                        gold_gain += 3;
                    }
                    gold_gain = (gold_gain as f64 * self.floor_profile.gold_multiplier()) as i32;
                    gold_gain = gold_gain.max(1);
                    // Location gold bonus
                    if self.current_location_type == Some(crate::world::LocationType::AsteroidBase)
                        || self.current_location_type == Some(crate::world::LocationType::MiningColony)
                    {
                        gold_gain = (gold_gain as f64 * 1.5) as i32;
                    }
                    if self.current_location_type == Some(crate::world::LocationType::AsteroidBase) {
                        gold_gain *= 2;
                    }
                    self.player.gold += gold_gain;

                    // Listening mode bonus gold
                    let listen_bonus = if !self.enemies[enemy_idx].is_elite {
                        match self.listening_mode {
                            ListenMode::ToneOnly => 3,
                            ListenMode::FullAudio => 5,
                            ListenMode::Off => 0,
                        }
                    } else {
                        0
                    };
                    self.player.gold += listen_bonus;
                    let available = radical::radicals_for_floor(self.floor_num);
                    let rad_roll = self.rng_next() % 100;
                    let rad_chance = self.floor_profile.radical_drop_chance();
                    let dropped = if rad_roll < rad_chance {
                        let drop_idx = self.rng_next() as usize % available.len();
                        let ch = available[drop_idx].ch;
                        self.player.add_radical(ch);
                        self.run_journal
                            .log(RunEvent::RadicalCollected(ch.to_string(), self.floor_num));
                        if self.floor_profile.radical_drop_bonus() {
                            let bonus_idx = self.rng_next() as usize % available.len();
                            self.player.add_radical(available[bonus_idx].ch);
                        }
                        Some(ch)
                    } else {
                        None
                    };
                    self.advance_radical_quests();

                    // Elite enemies drop an extra radical
                    if e_is_elite {
                        let drop2 = self.rng_next() as usize % available.len();
                        self.player.add_radical(available[drop2].ch);
                    }

                    // Extra radical from charm
                    let extra_chance = self.player.extra_radical_chance();
                    if extra_chance > 0 && (self.rng_next() % 100) < extra_chance as u64 {
                        let drop2 = self.rng_next() as usize % available.len();
                        self.player.add_radical(available[drop2].ch);
                    }

                    // AlienRuins: bonus radical on kill
                    if self.current_location_type == Some(crate::world::LocationType::AlienRuins) {
                        let bonus_idx = self.rng_next() as usize % available.len();
                        self.player.add_radical(available[bonus_idx].ch);
                    }

                    // Heal on kill from charm
                    let mut heal = self.player.heal_on_kill();
                    if self.player.get_piety(Faction::Consortium) >= 10 {
                        heal += 1;
                    }
                    if heal > 0 {
                        self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                    }

                    // Random equipment drop (5% chance, higher for bosses)
                    let equip_chance = if e_is_boss { 60 } else { 5 };
                    if (self.rng_next() % 100) < equip_chance {
                        let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
                        let eq = &EQUIPMENT_POOL[eq_idx];
                        let current_state = self.player.equipment_state(eq.slot);
                        if current_state == ItemState::Cursed {
                            if let Some(rad) = dropped {
                                self.message = format!(
                                    "Defeated {}! +{}g [{}] ({} blocked by curse!)",
                                    e_hanzi, e_gold, rad, eq.name
                                );
                            } else {
                                self.message = format!(
                                    "Defeated {}! +{}g ({} blocked by curse!)",
                                    e_hanzi, e_gold, eq.name
                                );
                            }
                        } else {
                            let state = self.roll_item_state();
                            self.player.equip(eq, state);
                            let prefix = match state {
                                ItemState::Cursed => "💀 ",
                                ItemState::Blessed => "✨ ",
                                ItemState::Normal => "",
                            };
                            if let Some(rad) = dropped {
                                self.message = format!(
                                    "Defeated {}! +{}g [{}] + {}{}!",
                                    e_hanzi, e_gold, rad, prefix, eq.name
                                );
                            } else {
                                self.message = format!(
                                    "Defeated {}! +{}g + {}{}!",
                                    e_hanzi, e_gold, prefix, eq.name
                                );
                            }
                        }
                    } else if let Some(rad) = dropped {
                        self.message = format!("Defeated {}! +{}g [{}]", e_hanzi, e_gold, rad);
                    } else {
                        self.message = format!("Defeated {}! +{}g", e_hanzi, e_gold);
                    }
                    if e_is_elite && elite_completed_cycle {
                        self.message = format!("⛓ {} — Compound shattered!", self.message);
                    }

                    // Bosses drop a rare radical
                    if e_is_boss {
                        let rares = radical::rare_radicals();
                        if !rares.is_empty() {
                            let rare_idx = self.rng_next() as usize % rares.len();
                            let rare = rares[rare_idx].ch;
                            self.player.add_radical(rare);
                            self.message = format!("{} ✦ Rare radical [{}]!", self.message, rare);
                        }
                    }
                    // Streak indicator
                    let tier = combo_tier(self.answer_streak);
                    if tier != ComboTier::None {
                        self.message = format!(
                            "{} 🔥 {}! ×{}",
                            self.message,
                            tier.name(),
                            self.answer_streak
                        );
                    }
                    self.message_timer = 80;
                    // Tutorial hint: first tutorial fight complete
                    if let Some(tutorial) = self.tutorial.as_mut() {
                        if !tutorial.combat_done {
                            tutorial.combat_done = true;
                            self.message =
                                "Great! Now walk to the ⚒ and forge 好 from 女 + 子.".to_string();
                            self.message_timer = 180;
                        }
                    } else if self.total_runs == 0 && self.player.radicals.len() == 1 {
                        if let Some(rad) = dropped {
                            self.message = format!(
                                "Defeated {}! +{}g [{}] — Walk to an ⚒ anvil to forge spells!",
                                e_hanzi, e_gold, rad
                            );
                        } else {
                            self.message = format!(
                                "Defeated {}! +{}g — Walk to an ⚒ anvil to forge spells!",
                                e_hanzi, e_gold
                            );
                        }
                        self.message_timer = 160;
                    }
                    self.combat = CombatState::Explore;

                    let kill_xp = if e_is_boss {
                        5
                    } else if e_is_elite {
                        3
                    } else {
                        2
                    };
                    self.add_companion_xp(kill_xp);

                    // Achievement checks
                    self.achievements.record_correct();
                    self.achievements.check_kills(self.total_kills);
                    self.achievements.check_gold(self.player.gold);
                    self.achievements.check_radicals(self.player.radicals.len());
                    if e_is_elite {
                        self.achievements.unlock("first_elite");
                    }
                    if e_is_boss {
                        self.achievements.unlock("first_boss");
                    }

                    // Boss bonus sentence challenge (scaled by floor)
                    if e_is_boss
                        && self.floor_num >= 5
                        && self.enemies[enemy_idx].boss_kind != Some(BossKind::HiveQueen)
                        && self.enemies[enemy_idx].boss_kind != Some(BossKind::AncientGuardian)
                        && self.enemies[enemy_idx].boss_kind != Some(BossKind::PirateCaptain)
                    {
                        let base_reward = 15 + self.floor_num * 2;
                        self.begin_sentence_challenge(
                            SentenceChallengeMode::BonusGold { reward: base_reward },
                            "Boss Phase 2! Arrange the words in correct order. ←→ to select, Enter to pick.".to_string(),
                        );
                    }

                    // Quest progress: kill tracking
                    self.advance_kill_quests();
                } else {
                    if let Some(ref audio) = self.audio {
                        audio.play_hit();
                    }
                    if self.maybe_trigger_boss_phase(enemy_idx) {
                        self.typing.clear();
                        return;
                    }
                    if let Some(message) = elite_message {
                        self.message =
                            format!("{} ({} HP left)", message, self.enemies[enemy_idx].hp);
                        self.message_timer = if elite_completed_cycle { 80 } else { 70 };
                    } else {
                        let tier = combo_tier(self.answer_streak);
                        self.message = if tier != ComboTier::None {
                            format!(
                                "Hit for {}! {} HP left 🔥 {}! ×{}",
                                dealt_dmg,
                                self.enemies[enemy_idx].hp,
                                tier.name(),
                                self.answer_streak
                            )
                        } else {
                            format!(
                                "Hit for {}! {} HP left",
                                dealt_dmg, self.enemies[enemy_idx].hp
                            )
                        };
                        self.message_timer = 40;
                    }
                }
            } else {
                // Miss — enemy counter-attacks
                let expected_pinyin =
                    if let Some(vocab::CompoundPinyinStep::Miss { expected, .. }) = elite_step {
                        self.enemies[enemy_idx].elite_chain = 0;
                        expected
                    } else {
                        e_pinyin
                    };
                self.srs.record(e_hanzi, false);
                self.codex.record(e_hanzi, e_pinyin, e_meaning, false);
                self.run_wrong_answers += 1;
                self.answer_streak = 0;
                self.achievements.record_miss();

                let mut thief_stole = None;
                if self.enemies[enemy_idx].boss_kind == Some(BossKind::DriftLeviathan)
                    && !self.player.radicals.is_empty()
                {
                    let steal_idx = self.rng_next() as usize % self.player.radicals.len();
                    let stolen = self.player.radicals.remove(steal_idx);
                    thief_stole = Some(stolen);
                }
                if let Some(ref audio) = self.audio {
                    audio.play_miss();
                }
                if self.enemies[enemy_idx].stunned {
                    self.enemies[enemy_idx].stunned = false;
                    self.message = if e_is_elite {
                        format!(
                            "✗ Wrong chain! Needed \"{}\", but {} is still staggered and cannot counterattack.",
                            expected_pinyin, e_hanzi
                        )
                    } else {
                        format!(
                            "Wrong! (was \"{}\") — {} is stunned and can't counterattack!",
                            expected_pinyin, e_hanzi
                        )
                    };
                    self.message_timer = 70;
                } else if self.player.shield {
                    self.player.shield = false;
                    self.message = if e_is_elite {
                        format!(
                            "✗ Wrong chain! Needed \"{}\" — Shield absorbed the blow!",
                            expected_pinyin
                        )
                    } else {
                        format!(
                            "Wrong! (was \"{}\") — Shield absorbed the blow!",
                            expected_pinyin
                        )
                    };
                    self.message_timer = if e_is_elite { 70 } else { 60 };
                } else if self.companion == Some(Companion::SecurityChief) && !self.guard_used_this_fight {
                    let lvl = self.companion_level();
                    let max_blocks = Companion::SecurityChief.guard_max_blocks(lvl);
                    self.guard_blocks_used += 1;
                    if self.guard_blocks_used >= max_blocks {
                        self.guard_used_this_fight = true;
                    } else {
                        let second_chance = Companion::SecurityChief.guard_second_block_chance(lvl);
                        if (self.rng_next() % 100) >= second_chance {
                            self.guard_used_this_fight = true;
                        }
                    }
                    self.message = if e_is_elite {
                        format!(
                            "✗ Wrong chain! Needed \"{}\" — 🛡 Guard blocks the attack!",
                            expected_pinyin
                        )
                    } else {
                        format!(
                            "Wrong! (was \"{}\") — 🛡 Guard blocks the attack!",
                            expected_pinyin
                        )
                    };
                    self.message_timer = if e_is_elite { 70 } else { 60 };
                } else {
                    let mut evaded = false;
                    if self.player.get_piety(Faction::FreeTraders) >= 10 {
                        if (self.rng_next() % 100) < 15 {
                            evaded = true;
                        }
                    }

                    if evaded {
                        let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
                        self.particles.spawn_synergy(sx, sy, &mut self.rng_state);
                        self.message = if e_is_elite {
                            format!(
                                "✗ Wrong chain! Needed \"{}\", but you evaded {}'s attack!",
                                expected_pinyin, e_hanzi
                            )
                        } else {
                            format!(
                                "Wrong! (was \"{}\") — you evaded {}'s attack!",
                                expected_pinyin, e_hanzi
                            )
                        };
                        self.message_timer = if e_is_elite { 70 } else { 60 };
                    } else {
                        // Apply defense_bonus from ToneDefense reward
                        let def_bonus = self.player.defense_bonus;
                        if def_bonus > 0 {
                            self.player.defense_bonus = 0;
                        }
                        let e_damage = (e_damage - def_bonus).max(0);
                        self.player.hp -= e_damage;

                        if self.enemies[enemy_idx].radical_multiply {
                            self.enemies[enemy_idx].radical_multiply = false;
                            self.player.hp -= e_damage;
                            self.message.push_str(" ✕ Double strike!");
                        }

                        if let Some(ref audio) = self.audio {
                            audio.play_damage();
                        }
                        // Damage particles + shake
                        let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
                        self.particles.spawn_damage(sx, sy, &mut self.rng_state);
                        self.trigger_shake(8);
                        self.flash = Some((255, 50, 50, 0.25));
                        self.message = if e_is_elite {
                            format!(
                                "✗ Wrong chain! Needed \"{}\". {} hits for {} and the compound resets!",
                                expected_pinyin, e_hanzi, e_damage
                            )
                        } else {
                            format!(
                                "Wrong! It was \"{}\". {} hits for {}!",
                                expected_pinyin, e_hanzi, e_damage
                            )
                        };
                        self.message_timer = if e_is_elite { 70 } else { 60 };
                    }
                }

                if self.player.get_piety(Faction::Technocracy) >= 10 {
                    self.mirror_hint = true;
                }

                if let Some(stolen) = thief_stole {
                    self.message
                        .push_str(&format!(" 🥷 The Drift Leviathan absorbed {}!", stolen));
                }

                // Radical actions: enemy uses abilities based on its hanzi components
                {
                    let actions = self.enemies[enemy_idx].radical_actions();
                    let mut action_msgs = Vec::new();
                    for action in actions {
                        // 30% chance per radical action
                        if (self.rng_next() % 100) < 30 {
                            let msg = self.apply_radical_action(enemy_idx, action);
                            action_msgs.push(msg);
                        }
                    }
                    if !action_msgs.is_empty() {
                        self.message.push_str(" ");
                        self.message.push_str(&action_msgs.join(" "));
                    }
                }

                if self.player.hp <= 0 && !self.try_phoenix_revive() {
                    self.player.hp = 0;
                    self.run_journal
                        .log(RunEvent::DiedTo(e_hanzi.to_string(), self.floor_num));
                    self.post_mortem_page = 0;
                    self.combat = CombatState::GameOver;
                    self.message = self.run_summary();
                    self.message_timer = 255;
                    if let Some(ref audio) = self.audio {
                        audio.play_death();
                    }
                    self.save_high_score();
                }
            }
            self.typing.clear();
        }
    }

    /// Apply a radical action triggered by an enemy on a wrong answer.
    /// Returns a description string of what happened.
    pub(super) fn apply_radical_action(&mut self, enemy_idx: usize, action: RadicalAction) -> String {
        match action {
            RadicalAction::SpreadingWildfire => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Burn { damage: 1 },
                        3,
                    ));
                format!("{} — You catch fire!", action.name())
            }
            RadicalAction::ErosiveFlow => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        3,
                    ));
                format!("{} — Water erodes your defenses!", action.name())
            }
            RadicalAction::OverwhelmingForce => {
                self.player.hp -= 2;
                format!("{} — A crushing blow! (-2 HP)", action.name())
            }
            RadicalAction::DoubtSeed => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Confused,
                        2,
                    ));
                format!("{} — Your mind clouds with doubt!", action.name())
            }
            RadicalAction::DevouringMaw => {
                self.player.hp -= 1;
                if self.player.shield {
                    self.player.shield = false;
                }
                format!("{} — Devoured! (-1 HP)", action.name())
            }
            RadicalAction::WitnessMark => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Fear,
                        1,
                    ));
                format!("{} — You feel exposed!", action.name())
            }
            RadicalAction::SleightReversal => {
                self.player.hp -= 1;
                format!("{} — Switched around! (-1 HP)", action.name())
            }
            RadicalAction::RootingGrasp => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        2,
                    ));
                self.enemies[enemy_idx].radical_armor += 1;
                format!("{} — Roots bind you!", action.name())
            }
            RadicalAction::HarvestReaping => {
                let threshold = (self.player.max_hp as f32 * 0.4) as i32;
                let dmg = if self.player.hp < threshold { 3 } else { 1 };
                self.player.hp -= dmg;
                format!("{} — Reaped! (-{} HP)", action.name(), dmg)
            }
            RadicalAction::RevealingDawn => {
                let e = &mut self.enemies[enemy_idx];
                e.statuses.retain(|s| {
                    !matches!(
                        s.kind,
                        crate::status::StatusKind::Burn { .. }
                            | crate::status::StatusKind::Poison { .. }
                            | crate::status::StatusKind::Bleed { .. }
                            | crate::status::StatusKind::Slow
                            | crate::status::StatusKind::Confused
                            | crate::status::StatusKind::Freeze
                            | crate::status::StatusKind::Fear
                    )
                });
                e.hp = (e.hp + 2).min(e.max_hp);
                format!("{} — Bathed in cleansing light!", action.name())
            }
            RadicalAction::WaningCurse => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Poison { damage: 1 },
                        2,
                    ));
                format!("{} — A waning curse takes hold!", action.name())
            }
            RadicalAction::MortalResilience => {
                let e = &mut self.enemies[enemy_idx];
                e.damage += 1;
                e.radical_armor += 1;
                format!("{} — Enemy grows desperate!", action.name())
            }
            RadicalAction::MaternalShield => {
                self.enemies[enemy_idx].radical_dodge = true;
                self.enemies[enemy_idx].radical_armor += 1;
                format!("{} — Takes a protective stance!", action.name())
            }
            RadicalAction::PotentialBurst => {
                self.player.hp -= 1;
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Bleed { damage: 1 },
                        1,
                    ));
                format!("{} — Latent energy erupts! (-1 HP)", action.name())
            }
            RadicalAction::ChasingChaff => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Confused,
                        1,
                    ));
                format!("{} — Chaff clouds your vision!", action.name())
            }
            RadicalAction::CrossroadsGambit => {
                let seed = (self.floor_num as usize)
                    .wrapping_mul(31)
                    .wrapping_add(enemy_idx)
                    % 2;
                if seed == 0 {
                    self.player.hp -= 3;
                    format!("{} — A bad bet! (-3 HP)", action.name())
                } else {
                    format!("{} — The gambit fails!", action.name())
                }
            }
            RadicalAction::RigidStance => {
                self.enemies[enemy_idx].radical_armor += 3;
                format!("{} — Metal scales form!", action.name())
            }
            RadicalAction::GroundingWeight => {
                self.player.hp -= 1;
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        3,
                    ));
                format!("{} — The earth drags you down! (-1 HP)", action.name())
            }
            RadicalAction::EchoStrike => {
                let dmg = self.enemies[enemy_idx].damage;
                self.player.hp -= dmg;
                format!("{} — An echo strikes for {} damage!", action.name(), dmg)
            }
            RadicalAction::PreciseExecution => {
                let threshold = (self.player.max_hp as f32 * 0.25) as i32;
                let dmg = if self.player.hp < threshold { 4 } else { 1 };
                self.player.hp -= dmg;
                format!("{} — Executed! (-{} HP)", action.name(), dmg)
            }
            RadicalAction::CleavingCut => {
                self.player.hp -= 2;
                self.player.max_hp = (self.player.max_hp - 1).max(1);
                format!("{} — A deep cut! (-2 HP, -1 Max HP)", action.name())
            }
            RadicalAction::BindingOath => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        3,
                    ));
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Confused,
                        1,
                    ));
                format!("{} — Bound by oath!", action.name())
            }
            RadicalAction::PursuingSteps => {
                self.player.hp -= 1;
                self.enemies[enemy_idx].alert = true;
                format!("{} — Relentless pursuit! (-1 HP)", action.name())
            }
            RadicalAction::EntanglingWeb => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        3,
                    ));
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Bleed { damage: 1 },
                        2,
                    ));
                format!("{} — Caught in a web!", action.name())
            }
            RadicalAction::ThresholdSeal => {
                self.enemies[enemy_idx].radical_armor += 3;
                format!("{} — A barrier seals the way!", action.name())
            }
            RadicalAction::CavalryCharge => {
                self.player.hp -= 2;
                self.trigger_shake(8);
                format!("{} — Trampled! (-2 HP)", action.name())
            }
            RadicalAction::SoaringEscape => {
                self.enemies[enemy_idx].radical_dodge = true;
                format!("{} — The enemy takes flight!", action.name())
            }
            RadicalAction::DownpourBarrage => {
                self.player.hp -= 1;
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Bleed { damage: 1 },
                        3,
                    ));
                format!("{} — Pierced by rain! (-1 HP)", action.name())
            }
            RadicalAction::PetrifyingGaze => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        3,
                    ));
                self.enemies[enemy_idx].radical_armor += 2;
                format!("{} — Turned to stone!", action.name())
            }
            RadicalAction::ParasiticSwarm => {
                self.player.hp -= 1;
                let e = &mut self.enemies[enemy_idx];
                e.hp = (e.hp + 2).min(e.max_hp);
                format!("{} — Swarmed! (-1 HP, Enemy heals 2)", action.name())
            }
            RadicalAction::MercenaryPact => {
                let e = &mut self.enemies[enemy_idx];
                e.hp -= 2;
                for em in &mut self.enemies {
                    if em.is_alive() {
                        em.alert = true;
                    }
                }
                format!("{} — A rallying cry!", action.name())
            }
            RadicalAction::ImmovablePeak => {
                self.enemies[enemy_idx].radical_armor += 3;
                format!("{} — Immovable as a mountain!", action.name())
            }
            RadicalAction::SavageMaul => {
                self.player.hp -= 3;
                format!("{} — Mauled! (-3 HP)", action.name())
            }
            RadicalAction::ArcingShot => {
                self.player.hp -= 2;
                format!("{} — Struck from above! (-2 HP)", action.name())
            }
            RadicalAction::ConsumingBite => {
                self.player.hp -= 2;
                let e = &mut self.enemies[enemy_idx];
                e.max_hp += 1;
                e.hp = (e.hp + 2).min(e.max_hp);
                format!("{} — Bitten! (-2 HP, Enemy +Max HP)", action.name())
            }
            RadicalAction::CloakingGuise => {
                self.enemies[enemy_idx].radical_dodge = true;
                format!("{} — The enemy vanishes!", action.name())
            }
            RadicalAction::FlexibleCounter => {
                self.enemies[enemy_idx].radical_dodge = true;
                self.enemies[enemy_idx].radical_armor += 1;
                format!("{} — Ready to counter!", action.name())
            }
            RadicalAction::BlitzAssault => {
                self.player.hp -= 2;
                self.trigger_shake(6);
                format!("{} — Blitzed! (-2 HP)", action.name())
            }
            RadicalAction::CrushingWheels => {
                self.player.hp -= 2;
                self.trigger_shake(10);
                format!("{} — Crushed! (-2 HP)", action.name())
            }
            RadicalAction::ImperialCommand => {
                for em in &mut self.enemies {
                    if em.is_alive() {
                        em.alert = true;
                        em.damage += 1;
                    }
                }
                format!("{} — A commanding presence!", action.name())
            }
            RadicalAction::MagnifyingAura => {
                self.enemies[enemy_idx].damage += 1;
                format!("{} — An empowering aura!", action.name())
            }
            RadicalAction::NeedleStrike => {
                self.player.hp -= 2;
                format!("{} — Pierced! (-2 HP)", action.name())
            }
            RadicalAction::ArtisanTrap => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Burn { damage: 1 },
                        2,
                    ));
                format!("{} — Trapped!", action.name())
            }
            RadicalAction::CleansingLight => {
                let e = &mut self.enemies[enemy_idx];
                e.statuses.retain(|s| {
                    !matches!(
                        s.kind,
                        crate::status::StatusKind::Burn { .. }
                            | crate::status::StatusKind::Poison { .. }
                            | crate::status::StatusKind::Bleed { .. }
                            | crate::status::StatusKind::Slow
                            | crate::status::StatusKind::Confused
                            | crate::status::StatusKind::Freeze
                            | crate::status::StatusKind::Fear
                    )
                });
                e.hp = (e.hp + 3).min(e.max_hp);
                format!("{} — Bathed in cleansing light!", action.name())
            }
            RadicalAction::ScatteringPages => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Confused,
                        1,
                    ));
                format!("{} — Pages scatter and blind you!", action.name())
            }
            RadicalAction::TrueVision => {
                if self.player.shield {
                    self.player.shield = false;
                }
                self.player.statuses.retain(|s| s.is_negative());
                format!("{} — All your protections are dispelled!", action.name())
            }
            RadicalAction::QiDisruption => {
                self.player.hp -= 1;
                format!("{} — Your qi is disrupted! (-1 HP)", action.name())
            }
            RadicalAction::ExpandingDomain => {
                self.enemies[enemy_idx].damage += 1;
                format!("{} — Domain expands!", action.name())
            }
            RadicalAction::SinkholeSnare => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        2,
                    ));
                format!("{} — Ground cracks beneath you!", action.name())
            }
            RadicalAction::SonicBurst => {
                self.player.hp -= 2;
                format!("{} — Sonic shockwave! (-2 HP)", action.name())
            }
            RadicalAction::VenomousLash => {
                self.player.hp -= 1;
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Poison { damage: 2 },
                        2,
                    ));
                format!("{} — Venomous lash! Poisoned! (-1 HP)", action.name())
            }
            RadicalAction::IronBodyStance => {
                self.enemies[enemy_idx].hp =
                    (self.enemies[enemy_idx].hp + 2).min(self.enemies[enemy_idx].max_hp);
                format!("{} — Iron body! Enemy hardens!", action.name())
            }
            RadicalAction::GoreCrush => {
                self.player.hp -= 2;
                format!("{} — Gored! (-2 HP)", action.name())
            }
            RadicalAction::IntoxicatingMist => {
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Confused,
                        2,
                    ));
                format!("{} — Intoxicating mist clouds your mind!", action.name())
            }
            RadicalAction::SproutingBarrier => {
                self.enemies[enemy_idx].hp =
                    (self.enemies[enemy_idx].hp + 1).min(self.enemies[enemy_idx].max_hp);
                format!("{} — Sprouting barrier!", action.name())
            }
            RadicalAction::TidalSurge => {
                self.player.hp -= 1;
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Slow,
                        2,
                    ));
                format!("{} — Tidal surge pushes you! (-1 HP)", action.name())
            }
            RadicalAction::BoneShatter => {
                self.player.hp -= 3;
                if self.player.shield {
                    self.player.shield = false;
                }
                format!("{} — Bone shatters your defenses! (-3 HP)", action.name())
            }
            RadicalAction::AdaptiveShift => {
                self.enemies[enemy_idx].damage += 1;
                format!("{} — Enemy adapts and hardens!", action.name())
            }
            RadicalAction::BerserkerFury => {
                self.enemies[enemy_idx].hp = (self.enemies[enemy_idx].hp - 2).max(1);
                self.enemies[enemy_idx].damage += 3;
                format!("{} — Berserker fury! Enemy rages!", action.name())
            }
            RadicalAction::FlockAssault => {
                self.player.hp -= 2;
                format!("{} — A flock descends! (-2 HP)", action.name())
            }
        }
    }

    /// Backspace during typing.
    pub(super) fn backspace(&mut self) {
        self.typing.pop();
    }

    pub(super) fn forge_submit(&mut self) {
        let recipe_idx = if let CombatState::Forging {
            ref recipes,
            cursor,
            ..
        } = self.combat
        {
            if recipes.is_empty() {
                return;
            }
            recipes[cursor]
        } else {
            return;
        };

        let recipe = &radical::RECIPES[recipe_idx];

        let mut available_indices: Vec<usize> = (0..self.player.radicals.len()).collect();
        let mut consumed_indices: Vec<usize> = Vec::new();
        for &needed in recipe.inputs {
            if let Some(pos) = available_indices
                .iter()
                .position(|&i| self.player.radicals[i] == needed)
            {
                consumed_indices.push(available_indices.remove(pos));
            }
        }

        if let Some(ref audio) = self.audio {
            audio.play_forge();
        }
        let spell = Spell {
            hanzi: recipe.output_hanzi,
            pinyin: recipe.output_pinyin,
            meaning: recipe.output_meaning,
            effect: recipe.effect,
        };
        if !self.discovered_recipes.contains(&recipe_idx) {
            self.discovered_recipes.push(recipe_idx);
        }
        self.run_spells_forged += 1;
        self.run_journal.log(RunEvent::SpellForged(
            format!("{} ({})", recipe.output_hanzi, recipe.output_meaning),
            self.floor_num,
        ));
        self.message = format!(
            "Forged {} ({}) — {}! [{}]",
            recipe.output_hanzi,
            recipe.output_pinyin,
            recipe.output_meaning,
            recipe.effect.label()
        );
        self.message_timer = 80;
        self.player.add_spell(spell);
        let forge_quest_message = self.advance_forge_quests(recipe.output_hanzi);
        if let Some(tutorial) = self.tutorial.as_mut() {
            tutorial.forge_done = true;
            self.message = format!(
                "Forged {}! Tutorial complete — Q selects spells, Space casts. Take the stairs to Floor 1.",
                recipe.output_hanzi
            );
            self.message_timer = 220;
        } else if self.total_runs == 0 && self.player.spells.len() == 1 {
            self.message = format!(
                "Forged {}! Q to select spell, Space to cast!",
                recipe.output_hanzi
            );
            self.message_timer = 160;
        }
        if let Some(quest_message) = forge_quest_message {
            self.message = format!("{}  {}", self.message, quest_message);
            self.message_timer = self.message_timer.max(120);
        }
        consumed_indices.sort_unstable_by(|a, b| b.cmp(a));
        for idx in consumed_indices {
            self.player.radicals.remove(idx);
        }
        self.combat = CombatState::Explore;
        self.achievements
            .check_recipes(self.discovered_recipes.len());
        self.achievements.check_spells(self.player.spells.len());
    }

}
