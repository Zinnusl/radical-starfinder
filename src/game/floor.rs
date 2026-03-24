//! Floor generation, spawning, and tutorial logic.

use super::*;

impl GameState {
    pub(super) fn spawn_enemies(&mut self) {
        let pool = vocab::vocab_for_floor(self.floor_num);
        if pool.is_empty() {
            return;
        }
        let rooms = self.level.rooms.clone();
        let is_boss_floor = self.floor_num % 5 == 0 && self.floor_num > 0;
        let mut enemies_per_room = 1 + self.floor_num / 4;
        if self.floor_profile == FloorProfile::Siege {
            enemies_per_room += 2;
        }

        for (i, room) in rooms.iter().enumerate() {
            if i == 0 || i == rooms.len() - 1 {
                continue;
            }
            // Boss in second-to-last room on boss floors
            if is_boss_floor && i == rooms.len() - 2 {
                let entry: &'static VocabEntry = match BossKind::for_floor(self.floor_num) {
                    Some(BossKind::PirateCaptain) => Self::vocab_entry_by_hanzi("门")
                        .unwrap_or(pool[self.rng_next() as usize % pool.len()]),
                    Some(BossKind::HiveQueen) => Self::vocab_entry_by_hanzi("学")
                        .unwrap_or(pool[self.rng_next() as usize % pool.len()]),
                    Some(BossKind::RogueAICore) => Self::vocab_entry_by_hanzi("电")
                        .unwrap_or(pool[self.rng_next() as usize % pool.len()]),
                    Some(BossKind::VoidEntity) => Self::vocab_entry_by_hanzi("王")
                        .unwrap_or(pool[self.rng_next() as usize % pool.len()]),
                    Some(BossKind::AncientGuardian) => Self::vocab_entry_by_hanzi("书")
                        .unwrap_or(pool[self.rng_next() as usize % pool.len()]),
                    Some(BossKind::DriftLeviathan) => Self::vocab_entry_by_hanzi("盗")
                        .unwrap_or(pool[self.rng_next() as usize % pool.len()]),
                    None => pool[self.rng_next() as usize % pool.len()],
                };
                let (cx, cy) = room.center();
                self.enemies
                    .push(Enemy::boss_from_vocab(entry, cx, cy, self.floor_num));
                continue;
            }
            for _ in 0..enemies_per_room.min(ENEMIES_PER_ROOM as i32 + self.floor_num / 3) {
                let rand_val = self.rng_next();
                let entry_idx = self.srs.weighted_pick(&pool, rand_val);
                let entry: &'static VocabEntry = pool[entry_idx];
                let ex = room.x + 1 + (self.rng_next() % (room.w - 2).max(1) as u64) as i32;
                let ey = room.y + 1 + (self.rng_next() % (room.h - 2).max(1) as u64) as i32;
                if tile_allows_enemy_spawn(self.level.tile(ex, ey)) {
                    self.enemies
                        .push(Enemy::from_vocab(entry, ex, ey, self.floor_num));
                }
            }
        }
        // DerelictShip: tougher enemies (+50% HP), more gold drops
        if self.current_location_type == Some(crate::world::LocationType::DerelictShip) {
            for e in &mut self.enemies {
                e.hp = (e.hp * 3) / 2;
                e.max_hp = (e.max_hp * 3) / 2;
                e.gold_value = (e.gold_value * 3) / 2;
            }
            // 20% chance to apply Cursed when entering a DerelictShip
            if self.rng_next() % 5 == 0 {
                self.player.statuses.push(crate::status::StatusInstance::new(
                    crate::status::StatusKind::Cursed,
                    30,
                ));
                self.message = "💀 A malware payload infects your systems! You feel cursed.".to_string();
                self.message_timer = 100;
            }
        }
    }

    pub(super) fn start_tutorial(&mut self, class: PlayerClass) {
        self.floor_num = 0;
        self.srs.current_deck = 0;
        self.level = DungeonLevel::tutorial(MAP_W, MAP_H);
        let (sx, sy) = self.level.start_pos();
        self.player = self.make_player(sx, sy, class);
        self.reset_item_lore();
        self.player.add_radical("女");
        self.player.add_radical("子");
        self.enemies.clear();

        let enemy_room = &self.level.rooms[1];
        let entry = vocab::VOCAB
            .iter()
            .find(|entry| entry.hanzi == "大")
            .unwrap_or(&vocab::VOCAB[0]);
        let mut enemy = Enemy::from_vocab(entry, enemy_room.x, enemy_room.y + enemy_room.h / 2, 1);
        enemy.gold_value = 0;
        self.enemies.push(enemy);

        self.combat = CombatState::Explore;
        self.typing.clear();
        self.message =
            "Tutorial Floor — read the signs, defeat 大, then forge 好 from 女 + 子.".to_string();
        self.message_timer = 220;
        self.tutorial = Some(TutorialState::default());

        let (px, py) = (self.player.x, self.player.y);
        compute_fov(&mut self.level, px, py, FOV_RADIUS);
    }

    pub(super) fn show_tutorial_sign(&mut self, sign_id: u8) {
        let text = match (sign_id, self.tutorial.as_ref()) {
            (0, _) => "Move with WASD or arrow keys. Walk onto signs to read tips.",
            (1, _) => "Combat: bump into 大, type da4, then press Enter to attack.",
            (2, Some(tutorial)) if !tutorial.combat_done => {
                "Beat the enemy first. Then use the forge with 女 + 子."
            }
            (2, _) => "Forge: stand on ⚒, press 1, 2, then Enter to make 好 (hao3).",
            (3, Some(tutorial)) if !tutorial.is_complete() => {
                "The stairs stay sealed until you defeat 大 and forge one spell."
            }
            (3, _) => "The stairs are open. Step on ▼ to begin Floor 1.",
            _ => return,
        };
        self.message = text.to_string();
        self.message_timer = 220;
    }

    pub(super) fn tutorial_hint(&self) -> Option<&'static str> {
        self.tutorial.as_ref().map(TutorialState::objective_text)
    }

    pub(super) fn tutorial_exit_blocker(&self) -> Option<&'static str> {
        tutorial_exit_blocker_for(self.tutorial.as_ref())
    }

    pub(super) fn descend_floor(&mut self, force_skip: bool) -> bool {
        if !force_skip {
            if let Some(blocker) = self.tutorial_exit_blocker() {
                self.message = blocker.to_string();
                self.message_timer = 150;
                return false;
            }
        }

        let leaving_tutorial = self.tutorial.is_some();
        if leaving_tutorial {
            self.tutorial = None;
        }
        self.new_floor();

        if leaving_tutorial {
            self.message = if force_skip {
                "⏭ Tutorial skipped. Welcome to Floor 1.".to_string()
            } else {
                "Tutorial complete! Welcome to Floor 1.".to_string()
            };
            self.message_timer = 180;
        } else if force_skip {
            self.message = format!("⏭ Test skip — descended to Floor {}.", self.floor_num);
            self.message_timer = 90;
        }

        true
    }

    pub(super) fn reveal_entire_floor(&mut self) {
        for revealed in self.level.revealed.iter_mut() {
            *revealed = true;
        }
    }

    pub(super) fn pacify_gold_reward(base_gold: i32, spell_power: i32) -> i32 {
        ((base_gold + 1) / 2).max(4) + spell_power.max(0)
    }

    pub(super) fn invoke_altar(&mut self, _x: i32, _y: i32, kind: AltarKind) {
        // Player is already on the tile (x, y) because move_to was called before this.

        let god = kind.deity();

        let has_piety = self.player.piety.iter().any(|(d, _)| *d == god);

        if !has_piety {
            if let Some(highest) = self.player.highest_faction() {
                if highest != god && self.player.get_piety(highest) >= 15 {
                    self.player.add_piety(highest, -3);
                    self.message = format!(
                        "⚠ {} disapproves of your wandering faith! (-3 favor)",
                        highest.name()
                    );
                    self.message_timer = 255;
                }
            }
            self.player.piety.push((god, 0));
        }

        if let Some(ref audio) = self.audio {
            audio.play_spell();
        }

        // Open Offering menu
        self.combat = CombatState::Offering {
            altar_kind: kind,
            cursor: 0,
        };

        let god_name = match kind {
            TerminalKind::Quantum => "Quantum Sage",
            TerminalKind::Stellar => "Stellar Navigator",
            TerminalKind::Holographic => "Holo Architect",
            TerminalKind::Tactical => "Tactical Commander",
            TerminalKind::Commerce => "Trade Consortium",
        };
        if !(!has_piety && self.message.contains("disapproves")) {
            self.message = format!("You kneel before the Altar of {}.", god_name);
            self.message_timer = 255;
        }
    }

    pub(super) fn begin_sentence_challenge(&mut self, mode: SentenceChallengeMode, intro: String) {
        let rng_val = self.rng_next();
        let (words, meaning) = select_sentence_for_floor(self.floor_num, rng_val);
        let mut tiles: Vec<usize> = (0..words.len()).collect();
        for i in (1..tiles.len()).rev() {
            let j = self.rng_next() as usize % (i + 1);
            tiles.swap(i, j);
        }
        self.combat = CombatState::SentenceChallenge {
            tiles,
            words: words.to_vec(),
            cursor: 0,
            arranged: Vec::new(),
            meaning,
            mode,
        };
        self.message = intro;
        self.message_timer = 150;
    }

    pub(super) fn maybe_trigger_boss_phase(&mut self, enemy_idx: usize) -> bool {
        if enemy_idx >= self.enemies.len() || !self.enemies[enemy_idx].is_alive() {
            return false;
        }
        // Save tactical battle state before sentence challenge replaces combat state
        if matches!(self.combat, CombatState::TacticalBattle(_)) {
            if let CombatState::TacticalBattle(battle) =
                std::mem::replace(&mut self.combat, CombatState::Explore)
            {
                self.saved_battle = Some(battle);
            }
        }
        if self.enemies[enemy_idx].boss_kind == Some(BossKind::PirateCaptain)
            && !self.enemies[enemy_idx].phase_triggered
            && self.enemies[enemy_idx].hp <= self.enemies[enemy_idx].max_hp / 2
        {
            self.enemies[enemy_idx].phase_triggered = true;
            self.begin_sentence_challenge(
                SentenceChallengeMode::GatekeeperSeal {
                    boss_idx: enemy_idx,
                    success_damage: 4 + self.floor_num / 3,
                    failure_damage_to_player: 3 + self.floor_num / 4,
                },
                "🔒 The Pirate Captain activates a lockdown seal! Arrange the sentence to shatter it.".to_string(),
            );
            return true;
        }
        if self.enemies[enemy_idx].boss_kind == Some(BossKind::HiveQueen)
            && !self.enemies[enemy_idx].phase_triggered
            && self.enemies[enemy_idx].hp <= self.enemies[enemy_idx].max_hp / 2
        {
            self.enemies[enemy_idx].phase_triggered = true;
            self.begin_sentence_challenge(
                SentenceChallengeMode::ScholarTrial {
                    boss_idx: enemy_idx,
                    success_damage: 6 + self.floor_num / 2,
                    failure_heal: 3 + self.floor_num / 5,
                },
                "📜 The Hive Queen emits a psionic syntax duel! Arrange the sentence to break the swarm link."
                    .to_string(),
            );
            return true;
        }
        if self.enemies[enemy_idx].boss_kind == Some(BossKind::AncientGuardian)
            && !self.enemies[enemy_idx].phase_triggered
            && self.enemies[enemy_idx].hp <= self.enemies[enemy_idx].max_hp / 2
        {
            self.enemies[enemy_idx].phase_triggered = true;
            self.begin_sentence_challenge(
                SentenceChallengeMode::ScholarTrial {
                    boss_idx: enemy_idx,
                    success_damage: 8 + self.floor_num / 2,
                    failure_heal: 5 + self.floor_num / 4,
                },
                "🖌 The Ancient Guardian projects a glyph trial! Arrange the sentence to dispel the energy ward."
                    .to_string(),
            );
            return true;
        }
        false
    }

}
