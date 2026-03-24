//! Rituals, victory/defeat handling, and run summary.

use super::*;

impl GameState {
    pub(super) fn perform_offering(&mut self, altar: AltarKind, idx: usize) {
        if idx >= self.player.items.len() {
            return;
        }
        let item = self.player.items.remove(idx);
        let _item_state = self.player.item_states.remove(idx);
        let deity = altar.deity();

        // Basic offering logic
        let piety_gain = match (deity, item.kind()) {
            (Faction::Consortium, ItemKind::MedHypo) => 5,
            (Faction::Consortium, _) => 1,
            (Faction::FreeTraders, ItemKind::StimPack | ItemKind::PersonalTeleporter) => 5,
            (Faction::Technocracy, ItemKind::ScannerPulse) => 5,
            (Faction::MilitaryAlliance, ItemKind::EMPGrenade | ItemKind::ToxinGrenade) => 5,
            (Faction::AncientOrder, _) => 2,
            _ => 1,
        };

        self.player.add_piety(deity, piety_gain);
        let bonus_text = self.player.faction_bonus(deity);
        self.message = if bonus_text == "None" || bonus_text == "Minor devotion" {
            format!(
                "Offered {} to {}. (+{} favor).",
                item.name(),
                deity.name(),
                piety_gain
            )
        } else {
            format!(
                "Offered {} to {}. (+{} favor) [{}]",
                item.name(),
                deity.name(),
                piety_gain,
                bonus_text
            )
        };
        self.message_timer = 90;
        self.combat = CombatState::Explore;
        let (sx, sy) = self.tile_to_screen(self.player.x, self.player.y);
        self.particles.spawn_altar(sx, sy, &mut self.rng_state);
        if let Some(ref audio) = self.audio {
            audio.play_spell();
        }
    }

    pub(super) fn pray_at_altar(&mut self, altar: AltarKind) {
        let deity = altar.deity();
        let piety = self.player.get_piety(deity);

        if piety >= 20 {
            // Grant Boon
            self.player.add_piety(deity, -20);
            match deity {
                Faction::Consortium => {
                    self.player.max_hp += 5;
                    self.player.hp = self.player.max_hp;
                    self.message = "Jade Emperor grants you vitality! (+5 Max HP)".to_string();
                }
                Faction::FreeTraders => {
                    self.player.set_form(PlayerForm::Holographic, 50);
                    self.message = "Wind Walker grants you form of Mist!".to_string();
                }
                Faction::Technocracy => {
                    for i in 0..ITEM_KIND_COUNT {
                        self.identified_items[i] = true;
                    }
                    self.message = "Mirror Sage reveals all truths!".to_string();
                }
                Faction::MilitaryAlliance => {
                    self.player.set_form(PlayerForm::Void, 50);
                    self.message = "Iron General grants you form of Tiger!".to_string();
                }
                Faction::AncientOrder => {
                    self.player.gold += 100;
                    self.message = "Golden Toad rains coins upon you!".to_string();
                }
            }
            if let Some(ref audio) = self.audio { audio.play_level_up(); }
        } else if piety < 0 {
            // Smite
            self.player.hp -= 5;
            self.message = format!("{} is offended by your pestering! (-5 HP)", deity.name());
            if let Some(ref audio) = self.audio { audio.play_damage(); }
        } else {
            let bonus = self.player.faction_bonus(deity);
            if bonus == "None" || bonus == "Minor devotion" {
                self.message = format!("{} ignores you. (Favor: {})", deity.name(), piety);
            } else {
                self.message = format!(
                    "{} acknowledges you. [{}] (Favor: {})",
                    deity.name(),
                    bonus,
                    piety
                );
            }
        }
        self.message_timer = 120;
    }

    pub(super) fn perform_dip(&mut self, source_idx: usize, target_cursor: usize) {
        // target_cursor: 0=Weapon, 1=Armor, 2=Charm, 3+=Inventory Items (idx - 3)
        // If target is an item, we need to be careful with indices because we remove the potion first.

        if source_idx >= self.player.items.len() {
            return;
        }

        // Remove potion first
        let potion = self.player.items.remove(source_idx);
        let _potion_state = self.player.item_states.remove(source_idx);

        // Adjust target index if it was an item
        let effective_target_idx = if target_cursor >= 3 {
            let raw_idx = target_cursor - 3;
            if source_idx < raw_idx {
                raw_idx - 1
            } else {
                raw_idx
            }
        } else {
            0 // unused
        };

        match target_cursor {
            0 => {
                // Weapon
                if let Some(_) = self.player.weapon {
                    if matches!(potion.kind(), ItemKind::ToxinGrenade) {
                        self.message =
                            "Coated weapon with poison! (Attacks poison enemies)".to_string();
                        self.player.statuses.push(status::StatusInstance::new(
                            status::StatusKind::Envenomed,
                            20,
                        ));
                    } else if matches!(potion.kind(), ItemKind::StimPack) {
                        self.message = "Coated weapon with speed! (Attacks empowered)".to_string();
                        self.player.statuses.push(status::StatusInstance::new(
                            status::StatusKind::Empowered { amount: 1 },
                            20,
                        ));
                    } else {
                        self.message = "Nothing happens.".to_string();
                    }
                } else {
                    self.message = "No weapon to coat.".to_string();
                    self.player.add_item(potion, ItemState::Normal); // Return item
                }
            }
            1 => {
                // Armor
                if let Some(_) = self.player.armor {
                    self.message = "You wash your armor.".to_string();
                } else {
                    self.message = "No armor.".to_string();
                    self.player.add_item(potion, ItemState::Normal);
                }
            }
            2 => {
                // Charm
                self.message = "You dip the charm. It sparkles.".to_string();
            }
            _ => {
                // Inventory Item
                if effective_target_idx < self.player.items.len() {
                    self.message = "Mixing not yet implemented. Item returned.".to_string();
                    self.player.add_item(potion, ItemState::Normal);
                } else {
                    // Invalid target (shouldn't happen)
                    self.player.add_item(potion, ItemState::Normal);
                }
            }
        }
        self.combat = CombatState::Explore;
        self.message_timer = 90;
    }

    pub(super) fn handle_tactical_victory(&mut self, killed: &[usize], combo: u32) {
        let mut total_gold_gained: i32 = 0;
        let mut last_radical_drop: Option<&str> = None;
        let mut equip_msg: Option<String> = None;
        let mut item_msg: Option<String> = None;
        let mut sentence_challenge: Option<(SentenceChallengeMode, String)> = None;

        for &ei in killed {
            if ei >= self.enemies.len() {
                continue;
            }
            let e_hanzi = self.enemies[ei].hanzi;
            let e_is_boss = self.enemies[ei].is_boss;
            let e_is_elite = self.enemies[ei].is_elite;
            let e_gold_base = self.enemies[ei].gold_value
                + self.player.gold_bonus()
                + self.player.enchant_gold_bonus();

            self.total_kills += 1;
            self.run_kills += 1;
            self.run_gold_earned += e_gold_base;
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

            let (sx, sy) = self.tile_to_screen(self.enemies[ei].x, self.enemies[ei].y);
            self.particles.spawn_kill(sx, sy, &mut self.rng_state);
            self.flash = Some((255, 255, 255, 0.3));

            let mut gold_gain = e_gold_base;
            if self.player.get_piety(Faction::MilitaryAlliance) >= 10 && self.player.get_piety(Faction::AncientOrder) >= 10
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
            total_gold_gained += gold_gain;

            let listen_bonus = if !e_is_elite {
                match self.listening_mode {
                    ListenMode::ToneOnly => 3,
                    ListenMode::FullAudio => 5,
                    ListenMode::Off => 0,
                }
            } else {
                0
            };
            self.player.gold += listen_bonus;
            total_gold_gained += listen_bonus;

            let available = radical::radicals_for_floor(self.floor_num);
            let rad_roll = self.rng_next() % 100;
            let rad_chance = self.floor_profile.radical_drop_chance();
            if rad_roll < rad_chance {
                let drop_idx = self.rng_next() as usize % available.len();
                let ch = available[drop_idx].ch;
                self.player.add_radical(ch);
                self.run_journal
                    .log(RunEvent::RadicalCollected(ch.to_string(), self.floor_num));
                if self.floor_profile.radical_drop_bonus() {
                    let bonus_idx = self.rng_next() as usize % available.len();
                    self.player.add_radical(available[bonus_idx].ch);
                }
                last_radical_drop = Some(ch);
            }
            self.advance_radical_quests();

            if e_is_elite {
                let drop2 = self.rng_next() as usize % available.len();
                self.player.add_radical(available[drop2].ch);
            }

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

            let mut heal = self.player.heal_on_kill();
            if self.player.get_piety(Faction::Consortium) >= 10 {
                heal += 1;
            }
            if heal > 0 {
                self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
            }

            let equip_chance: u64 = if e_is_boss { 60 } else { 5 };
            if (self.rng_next() % 100) < equip_chance {
                let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
                let eq = &EQUIPMENT_POOL[eq_idx];
                let current_state = self.player.equipment_state(eq.slot);
                if current_state == ItemState::Cursed {
                    equip_msg = Some(format!("{} blocked by curse!", eq.name));
                } else {
                    let state = self.roll_item_state();
                    self.player.equip(eq, state);
                    let prefix = match state {
                        ItemState::Cursed => "💀 ",
                        ItemState::Blessed => "✨ ",
                        ItemState::Normal => "",
                    };
                    equip_msg = Some(format!("{}{}", prefix, eq.name));
                }
            }

            let item_chance: u64 = if e_is_boss {
                40
            } else if e_is_elite {
                15
            } else {
                4
            };
            if (self.rng_next() % 100) < item_chance {
                let drop_item = if e_is_boss && (self.rng_next() % 5) == 0 {
                    crate::player::Item::Revitalizer(self.player.max_hp / 2)
                } else {
                    self.random_item()
                };
                let state = self.roll_item_state();
                let name = drop_item.name().to_string();
                if self.player.add_item(drop_item, state) {
                    self.ship.cargo_used = (self.ship.cargo_used + 1).min(self.ship.cargo_capacity);
                    let prefix = match state {
                        ItemState::Cursed => "💀 ",
                        ItemState::Blessed => "✨ ",
                        ItemState::Normal => "",
                    };
                    item_msg = Some(format!("{}{}", prefix, name));
                }
            }

            if e_is_boss {
                let rares = radical::rare_radicals();
                if !rares.is_empty() {
                    let rare_idx = self.rng_next() as usize % rares.len();
                    let rare = rares[rare_idx].ch;
                    self.player.add_radical(rare);
                    last_radical_drop = Some(rare);
                }
            }

            let kill_xp = if e_is_boss {
                5
            } else if e_is_elite {
                3
            } else {
                2
            };
            self.add_companion_xp(kill_xp);

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

            if e_is_boss
                && self.floor_num >= 5
                && self.enemies[ei].boss_kind != Some(BossKind::HiveQueen)
                && self.enemies[ei].boss_kind != Some(BossKind::AncientGuardian)
                && self.enemies[ei].boss_kind != Some(BossKind::PirateCaptain)
            {
                let base_reward = 15 + self.floor_num * 2;
                sentence_challenge = Some((
                    SentenceChallengeMode::BonusGold { reward: base_reward },
                    "Boss Phase 2! Arrange the words in correct order. ←→ to select, Enter to pick.".to_string(),
                ));
            }

            self.advance_kill_quests();
        }

        self.answer_streak = combo;

        // Crew XP gain: each living crew member gains XP from combat
        let crew_xp_gain = killed.len() as u32 * 10;
        for crew in self.crew.iter_mut() {
            if crew.hp > 0 {
                crew.xp += crew_xp_gain;
                let xp_threshold = crew.level as u32 * 100;
                if crew.xp >= xp_threshold {
                    crew.xp -= xp_threshold;
                    crew.level += 1;
                    crew.skill += 1;
                }
                crew.morale = (crew.morale + 2).min(100);
            }
        }

        // Medic crew bonus: heal player after combat
        let medic_heal = self.crew_bonus(CrewRole::Medic);
        if medic_heal > 0 {
            self.player.hp = (self.player.hp + medic_heal).min(self.player.max_hp);
        }

        let mut msg = format!("Victory! +{}g", total_gold_gained);
        if let Some(rad) = last_radical_drop {
            msg = format!("{} [{}]", msg, rad);
        }
        if let Some(eq) = equip_msg {
            msg = format!("{} + {}", msg, eq);
        }
        if let Some(itm) = item_msg {
            msg = format!("{} + {}", msg, itm);
        }
        let tier = combo_tier(self.answer_streak);
        if tier != ComboTier::None {
            msg = format!("{} 🔥 {}! ×{}", msg, tier.name(), self.answer_streak);
        }
        self.message = msg;
        self.message_timer = 90;

        if let Some(tutorial) = self.tutorial.as_mut() {
            if !tutorial.combat_done {
                tutorial.combat_done = true;
                self.message = "Great! Now walk to the ⚒ and forge 好 from 女 + 子.".to_string();
                self.message_timer = 180;
            }
        }

        if let Some((mode, intro)) = sentence_challenge {
            self.combat = CombatState::Explore;
            self.begin_sentence_challenge(mode, intro);
        } else {
            self.combat = CombatState::Explore;
        }
    }

    pub(super) fn handle_tactical_defeat(&mut self, killer_name: String) {
        self.player.hp = 0;
        self.run_journal
            .log(RunEvent::DiedTo(killer_name, self.floor_num));
        self.post_mortem_page = 0;
        self.combat = CombatState::GameOver;
        self.message = self.run_summary();
        self.message_timer = 255;
        if let Some(ref audio) = self.audio {
            audio.play_death();
        }
        self.save_high_score();
    }

    pub(super) fn run_summary(&self) -> String {
        let accuracy = if self.run_correct_answers + self.run_wrong_answers > 0 {
            (self.run_correct_answers as f64
                / (self.run_correct_answers + self.run_wrong_answers) as f64
                * 100.0) as u32
        } else {
            0
        };
        format!(
            "☠ You died on floor {}!  ⚔ {} kills | 🏆 {} bosses | 💰 {} gold | ✅ {}% accuracy ({}/{}) | 🔨 {} spells forged  — Press R to restart",
            self.floor_num,
            self.run_kills,
            self.run_bosses_killed,
            self.run_gold_earned,
            accuracy,
            self.run_correct_answers,
            self.run_correct_answers + self.run_wrong_answers,
            self.run_spells_forged,
        )
    }



}
