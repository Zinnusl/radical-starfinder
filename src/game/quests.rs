//! Quest system and challenge mini-games.

use super::*;

impl GameState {
    pub(super) fn advance_kill_quests(&mut self) {
        for q in &mut self.quests {
            if q.completed {
                continue;
            }
            if let QuestGoal::KillEnemies(ref mut current, _) = q.goal {
                *current += 1;
            }
        }
        self.collect_quest_rewards();
    }

    /// Advance radical-collect quests.
    pub(super) fn advance_radical_quests(&mut self) {
        for q in &mut self.quests {
            if q.completed {
                continue;
            }
            if let QuestGoal::CollectRadicals(ref mut current, _) = q.goal {
                *current += 1;
            }
        }
        self.collect_quest_rewards();
    }

    /// Complete forge-character quests when the requested hanzi is created.
    pub(super) fn advance_forge_quests(&mut self, forged_hanzi: &'static str) -> Option<String> {
        let mut reward_messages = Vec::new();
        let mut chain_follow_ups: Vec<(u8, u32)> = Vec::new();
        for q in &mut self.quests {
            if q.completed {
                continue;
            }
            if let QuestGoal::ForgeCharacter(target) = q.goal {
                if target == forged_hanzi {
                    q.completed = true;
                    if q.gold_reward > 0 {
                        self.player.gold += q.gold_reward;
                        if q.is_chain() && q.chain_step < 4 {
                            reward_messages.push(format!(
                                "⛓ Chain quest step complete: {}! +{}g — Next step incoming!",
                                q.description, q.gold_reward
                            ));
                            chain_follow_ups.push((q.chain_step, q.chain_id));
                        } else if q.is_chain() && q.chain_step >= 4 {
                            reward_messages.push(format!(
                                "🏆 Quest chain complete: {}! +{}g — Bonus: +20g!",
                                q.description, q.gold_reward
                            ));
                            self.player.gold += 20;
                        } else {
                            reward_messages.push(format!(
                                "Quest complete: {}! +{}g",
                                q.description, q.gold_reward
                            ));
                        }
                        q.gold_reward = 0;
                    }
                }
            }
        }
        for (step, cid) in chain_follow_ups {
            self.quests.retain(|q| !(q.chain_id == cid && q.completed));
            let follow_up = self.generate_chain_quest(step, cid);
            self.quests.push(follow_up);
        }
        if reward_messages.is_empty() {
            None
        } else {
            Some(reward_messages.join(" "))
        }
    }

    /// Check floor-based quests.
    pub(super) fn check_floor_quests(&mut self) {
        let floor = self.floor_num;
        for q in &mut self.quests {
            if q.completed {
                continue;
            }
            if let QuestGoal::ReachFloor(target) = q.goal {
                if floor >= target {
                    q.completed = true;
                }
            }
        }
        self.collect_quest_rewards();
    }

    /// Generate 1-3 quests based on current floor and seed.
    pub(super) fn generate_quests(&mut self) {
        self.quests.retain(|q| !q.completed);
        if self.quests.len() >= 5 {
            return;
        }
        let seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let num_quests = 1 + (seed % 3) as usize;

        for i in 0..num_quests {
            if self.quests.len() >= 5 {
                break;
            }
            let qseed = seed.wrapping_add(i as u64 * 7919);
            let quest = match qseed % 4 {
                0 => Quest {
                    description: format!("Eliminate {} hostiles", 3 + (qseed % 5) as i32),
                    goal: QuestGoal::KillEnemies(0, 3 + (qseed % 5) as i32),
                    gold_reward: 30 + (qseed % 40) as i32,
                    completed: false,
                    chain_step: 0,
                    chain_id: 0,
                },
                1 => Quest {
                    description: format!("Reach deck {}", self.floor_num + 2 + (qseed % 3) as i32),
                    goal: QuestGoal::ReachFloor(self.floor_num + 2 + (qseed % 3) as i32),
                    gold_reward: 50 + (qseed % 50) as i32,
                    completed: false,
                    chain_step: 0,
                    chain_id: 0,
                },
                2 => Quest {
                    description: format!("Collect {} radicals", 2 + (qseed % 4) as i32),
                    goal: QuestGoal::CollectRadicals(0, 2 + (qseed % 4) as i32),
                    gold_reward: 40 + (qseed % 30) as i32,
                    completed: false,
                    chain_step: 0,
                    chain_id: 0,
                },
                _ => Quest {
                    description: "Clear the deck of enemies".to_string(),
                    goal: QuestGoal::KillEnemies(0, 5 + (qseed % 3) as i32),
                    gold_reward: 60 + (qseed % 40) as i32,
                    completed: false,
                    chain_step: 0,
                    chain_id: 0,
                },
            };
            if !self.quests.iter().any(|q| q.description == quest.description) {
                self.quests.push(quest);
            }
        }
    }

    /// Collect rewards from completed quests.
    pub(super) fn collect_quest_rewards(&mut self) {
        let mut chain_follow_ups: Vec<(u8, u32)> = Vec::new();
        let mut quest_xp: u32 = 0;
        for q in &mut self.quests {
            if q.completed && q.gold_reward > 0 {
                self.player.gold += q.gold_reward;
                quest_xp += 10;
                // ResearchLab: double vocab XP
                if self.current_location_type == Some(crate::world::LocationType::ResearchLab) {
                    quest_xp += 10;
                }
                if q.is_chain() && q.chain_step < 4 {
                    self.message = format!(
                        "⛓ Chain quest step complete: {}! +{}g — Next step incoming!",
                        q.description, q.gold_reward
                    );
                    chain_follow_ups.push((q.chain_step, q.chain_id));
                } else if q.is_chain() && q.chain_step >= 4 {
                    self.message = format!(
                        "🏆 Quest chain complete: {}! +{}g — Bonus: +20g!",
                        q.description, q.gold_reward
                    );
                    self.player.gold += 20;
                } else {
                    self.message =
                        format!("Quest complete: {}! +{}g", q.description, q.gold_reward);
                }
                self.message_timer = 100;
                q.gold_reward = 0;
            }
        }
        if quest_xp > 0 {
            self.add_companion_xp(quest_xp);
        }
        for (step, cid) in chain_follow_ups {
            self.quests.retain(|q| !(q.chain_id == cid && q.completed));
            let follow_up = self.generate_chain_quest(step, cid);
            self.quests.push(follow_up);
        }
    }

    /// Start a tone battle at a shrine.
    pub(super) fn start_tone_battle(&mut self) {
        let (hanzi, tone) = self.pick_tone_battle_char();
        if let Some(ref audio) = self.audio {
            audio.play_chinese_tone(tone);
        }
        self.combat = CombatState::ToneBattle {
            round: 0,
            hanzi,
            correct_tone: tone,
            score: 0,
            last_result: None,
        };
        self.message = "🔔 Tone Shrine! Listen and press 1-4 for the correct tone.".to_string();
        self.message_timer = 120;
    }

    /// Pick a random character and extract its tone for the tone battle.
    pub(super) fn pick_tone_battle_char(&mut self) -> (&'static str, u8) {
        let v = &vocab::VOCAB;
        let idx = self.rng_next() as usize % v.len();
        let entry = &v[idx];
        let tone = entry
            .pinyin
            .chars()
            .last()
            .and_then(|c| c.to_digit(10))
            .unwrap_or(1) as u8;
        (entry.hanzi, tone)
    }

    /// Start a stroke order challenge at a StrokeShrine.
    pub(super) fn start_stroke_order(&mut self) {
        let idx = self.rng_next() as usize % STROKE_ORDER_DATA.len();
        let (hanzi, components, pinyin, meaning) = STROKE_ORDER_DATA[idx];
        let correct_order: Vec<&'static str> = components.to_vec();
        // Fisher-Yates shuffle for components
        let mut shuffled = correct_order.clone();
        let n = shuffled.len();
        for i in (1..n).rev() {
            let j = self.rng_next() as usize % (i + 1);
            shuffled.swap(i, j);
        }
        self.combat = CombatState::StrokeOrder {
            hanzi,
            components: shuffled,
            correct_order,
            cursor: 0,
            arranged: Vec::new(),
            pinyin,
            meaning,
        };
        self.message = format!(
            "筆 Stroke Shrine! Arrange the components of {} in order.",
            hanzi
        );
        self.message_timer = 120;
    }

    /// Start a tone defense challenge at a ToneWall.
    pub(super) fn start_tone_defense(&mut self) {
        let pool = vocab::vocab_for_floor(self.floor_num);
        let entry = if pool.is_empty() {
            &vocab::VOCAB[self.rng_next() as usize % vocab::VOCAB.len()]
        } else {
            pool[self.rng_next() as usize % pool.len()]
        };
        let tone = entry
            .pinyin
            .chars()
            .last()
            .and_then(|c| c.to_digit(10))
            .unwrap_or(1) as u8;
        self.combat = CombatState::ToneDefense {
            round: 0,
            hanzi: entry.hanzi,
            pinyin: entry.pinyin,
            meaning: entry.meaning,
            correct_tone: tone,
            score: 0,
            last_result: None,
        };
        self.message = format!("壁 Tone Wall! What tone is {}? Press 1-4.", entry.hanzi);
        self.message_timer = 120;
    }

    /// Start a compound builder challenge at a CompoundShrine.
    pub(super) fn start_compound_builder(&mut self) {
        let idx = self.rng_next() as usize % COMPOUND_DATA.len();
        let (compound, parts, pinyin, meaning) = COMPOUND_DATA[idx];
        let correct_compound = compound;
        // Fisher-Yates shuffle for parts
        let mut shuffled: Vec<&'static str> = parts.to_vec();
        let n = shuffled.len();
        for i in (1..n).rev() {
            let j = self.rng_next() as usize % (i + 1);
            shuffled.swap(i, j);
        }
        self.combat = CombatState::CompoundBuilder {
            parts: shuffled,
            correct_compound,
            pinyin,
            meaning,
            cursor: 0,
            arranged: Vec::new(),
        };
        self.message = format!(
            "合 Compound Shrine! Combine the characters into a word. ({})",
            meaning
        );
        self.message_timer = 120;
    }

    /// Start a classifier match challenge at a ClassifierShrine.
    pub(super) fn start_classifier_match(&mut self) {
        let idx = self.rng_next() as usize % CLASSIFIER_DATA.len();
        let (noun, correct_classifier, noun_pinyin, noun_meaning) = CLASSIFIER_DATA[idx];
        // Build 4 options: 1 correct + 3 random wrong
        let mut options: Vec<&'static str> = vec![correct_classifier];
        let mut attempts = 0;
        while options.len() < 4 && attempts < 50 {
            let c = ALL_CLASSIFIERS[self.rng_next() as usize % ALL_CLASSIFIERS.len()];
            if !options.contains(&c) {
                options.push(c);
            }
            attempts += 1;
        }
        // Pad if not enough unique classifiers found
        while options.len() < 4 {
            options.push("个");
        }
        // Fisher-Yates shuffle
        let n = options.len();
        for i in (1..n).rev() {
            let j = self.rng_next() as usize % (i + 1);
            options.swap(i, j);
        }
        let correct_idx = options
            .iter()
            .position(|&c| c == correct_classifier)
            .unwrap_or(0);
        let options_arr: [&'static str; 4] = [options[0], options[1], options[2], options[3]];
        self.combat = CombatState::ClassifierMatch {
            round: 0,
            noun,
            noun_pinyin,
            noun_meaning,
            correct_classifier,
            options: options_arr,
            correct_idx,
            score: 0,
            last_result: None,
        };
        self.message = format!(
            "量 Classifier Shrine! Which measure word for {}? Press 1-4.",
            noun
        );
        self.message_timer = 120;
    }

    pub(super) fn start_ink_well(&mut self) {
        let idx = self.rng_next() as usize % INK_WELL_DATA.len();
        let (hanzi, correct_count, pinyin, meaning) = INK_WELL_DATA[idx];
        self.combat = CombatState::InkWellChallenge {
            hanzi,
            correct_count,
            pinyin,
            meaning,
        };
        self.message = format!(
            "墨 Ink Well! {} ({}) — How many components? Press 1-9.",
            hanzi, meaning
        );
        self.message_timer = 120;
    }

    pub(super) fn start_ancestor_challenge(&mut self) {
        let idx = self.rng_next() as usize % CHENGYU_DATA.len();
        let (first_half, correct_second, full, pinyin, meaning) = CHENGYU_DATA[idx];
        let mut options: Vec<&'static str> = vec![correct_second];
        let mut attempts = 0;
        while options.len() < 4 && attempts < 50 {
            let other_idx = self.rng_next() as usize % CHENGYU_DATA.len();
            let (_, other_second, _, _, _) = CHENGYU_DATA[other_idx];
            if !options.contains(&other_second) {
                options.push(other_second);
            }
            attempts += 1;
        }
        while options.len() < 4 {
            options.push("??");
        }
        let n = options.len();
        for i in (1..n).rev() {
            let j = self.rng_next() as usize % (i + 1);
            options.swap(i, j);
        }
        let correct_idx = options
            .iter()
            .position(|&s| s == correct_second)
            .unwrap_or(0);
        let options_arr: [&'static str; 4] = [options[0], options[1], options[2], options[3]];
        self.combat = CombatState::AncestorChallenge {
            first_half,
            correct_second,
            full,
            pinyin,
            meaning,
            options: options_arr,
            correct_idx,
        };
        self.message = format!(
            "祖 Ancestor Shrine! Complete the chengyu: {}____. Press 1-4.",
            first_half
        );
        self.message_timer = 120;
    }

    pub(super) fn start_translation_challenge(&mut self) {
        let vocab = vocab::vocab_for_floor(self.floor_num);
        if vocab.len() < 4 {
            self.message = "Not enough vocabulary for this floor.".into();
            self.message_timer = 60;
            return;
        }
        let idx = self.rng_next() as usize % vocab.len();
        let correct = vocab[idx];
        let mut options: Vec<&'static str> = vec![correct.hanzi];
        let mut attempts = 0;
        while options.len() < 4 && attempts < 50 {
            let other_idx = self.rng_next() as usize % vocab.len();
            if !options.contains(&vocab[other_idx].hanzi) {
                options.push(vocab[other_idx].hanzi);
            }
            attempts += 1;
        }
        while options.len() < 4 {
            options.push("?");
        }
        let n = options.len();
        for i in (1..n).rev() {
            let j = self.rng_next() as usize % (i + 1);
            options.swap(i, j);
        }
        let correct_idx = options
            .iter()
            .position(|&s| s == correct.hanzi)
            .unwrap_or(0);
        let options_arr: [&'static str; 4] = [options[0], options[1], options[2], options[3]];
        self.combat = CombatState::TranslationChallenge {
            round: 0,
            meaning: correct.meaning,
            correct_hanzi: correct.hanzi,
            correct_pinyin: correct.pinyin,
            options: options_arr,
            correct_idx,
            score: 0,
        };
        self.message = format!(
            "译 Translation Altar! Which Chinese means \"{}\"? Press 1-4. (Round 1/3)",
            correct.meaning
        );
        self.message_timer = 120;
    }

    pub(super) fn start_radical_garden(&mut self) {
        let idx = self.rng_next() as usize % RADICAL_GARDEN_DATA.len();
        let (hanzi, pinyin, meaning, radical, w1, w2, w3) = RADICAL_GARDEN_DATA[idx];
        let mut options: Vec<&'static str> = vec![radical, w1, w2, w3];
        let n = options.len();
        for i in (1..n).rev() {
            let j = self.rng_next() as usize % (i + 1);
            options.swap(i, j);
        }
        let correct_idx = options.iter().position(|&s| s == radical).unwrap_or(0);
        let options_arr: [&'static str; 4] = [options[0], options[1], options[2], options[3]];
        self.combat = CombatState::RadicalGardenChallenge {
            hanzi,
            pinyin,
            meaning,
            correct_radical: radical,
            options: options_arr,
            correct_idx,
        };
        self.message = format!(
            "部 Radical Garden! What is the radical of {}? Press 1-4.",
            hanzi
        );
        self.message_timer = 120;
    }

    pub(super) fn start_mirror_pool(&mut self) {
        let idx = self.rng_next() as usize % MIRROR_POOL_DATA.len();
        let (hanzi, pinyin, meaning) = MIRROR_POOL_DATA[idx];
        self.combat = CombatState::MirrorPoolChallenge {
            hanzi,
            correct_pinyin: pinyin,
            meaning,
            input: String::new(),
        };
        self.typing = String::new();
        self.message = format!(
            "鏡 Mirror Pool! Type the pinyin for {} ({}). Press Enter to submit.",
            hanzi, meaning
        );
        self.message_timer = 120;
    }

    pub(super) fn start_stone_tutor(&mut self) {
        let vocab = vocab::vocab_for_floor(self.floor_num);
        if vocab.is_empty() {
            self.message = "No vocabulary available.".into();
            self.message_timer = 60;
            return;
        }
        let idx = self.rng_next() as usize % vocab.len();
        let entry = vocab[idx];
        let tone = entry
            .pinyin
            .chars()
            .last()
            .and_then(|c| c.to_digit(10))
            .unwrap_or(1) as u8;
        self.combat = CombatState::StoneTutorChallenge {
            round: 0,
            hanzi: entry.hanzi,
            pinyin: entry.pinyin,
            meaning: entry.meaning,
            correct_tone: tone,
            phase: 0,
            score: 0,
        };
        self.message = format!(
            "石 Stone Tutor! Study: {} — {} ({}). Press Space to continue to quiz.",
            entry.hanzi, entry.pinyin, entry.meaning
        );
        self.message_timer = 120;
    }

    pub(super) fn start_codex_challenge(&mut self) {
        let codex_data: Vec<(&'static str, &'static str, &'static str)> = self
            .codex
            .sorted_entries()
            .iter()
            .map(|e| (e.hanzi, e.pinyin, e.meaning))
            .collect();
        let vocab = vocab::vocab_for_floor(self.floor_num);
        let use_codex = codex_data.len() >= 4;
        if !use_codex && vocab.len() < 4 {
            self.message = "Not enough vocabulary yet.".into();
            self.message_timer = 60;
            return;
        }
        let (hanzi, pinyin, meaning, distractors) = if use_codex {
            let idx = self.rng_next() as usize % codex_data.len();
            let (h, p, m) = codex_data[idx];
            let mut dist: Vec<&'static str> = codex_data
                .iter()
                .filter(|e| e.0 != h)
                .map(|e| e.2)
                .collect();
            while dist.len() < 3 {
                let vi = self.rng_next() as usize % vocab.len();
                let vm = vocab[vi].meaning;
                if vm != m && !dist.contains(&vm) {
                    dist.push(vm);
                }
            }
            for i in (1..dist.len()).rev() {
                let j = self.rng_next() as usize % (i + 1);
                dist.swap(i, j);
            }
            (h, p, m, [dist[0], dist[1], dist[2], ""])
        } else {
            let idx = self.rng_next() as usize % vocab.len();
            let entry = vocab[idx];
            let mut dist: Vec<&'static str> = vocab
                .iter()
                .filter(|e| e.hanzi != entry.hanzi)
                .map(|e| e.meaning)
                .collect();
            for i in (1..dist.len()).rev() {
                let j = self.rng_next() as usize % (i + 1);
                dist.swap(i, j);
            }
            (
                entry.hanzi,
                entry.pinyin,
                entry.meaning,
                [dist[0], dist[1], dist[2], ""],
            )
        };
        let correct_idx = self.rng_next() as usize % 4;
        let mut options = [distractors[0], distractors[1], distractors[2], meaning];
        // Shift correct answer into correct_idx
        options[3] = options[correct_idx];
        options[correct_idx] = meaning;
        self.combat = CombatState::CodexChallenge {
            round: 0,
            hanzi,
            pinyin,
            meaning,
            options,
            correct_idx,
            score: 0,
        };
        self.message = format!("典 Codex Shrine! What does {} mean? Pick 1-4.", hanzi);
        self.message_timer = 120;
    }

    pub(super) fn start_word_bridge(&mut self, bridge_x: i32, bridge_y: i32) {
        let vocab = vocab::vocab_for_floor(self.floor_num);
        if vocab.len() < 4 {
            self.message = "Not enough vocabulary.".into();
            self.message_timer = 60;
            return;
        }
        let idx = self.rng_next() as usize % vocab.len();
        let entry = vocab[idx];
        let mut others: Vec<&'static str> = vocab
            .iter()
            .filter(|e| e.hanzi != entry.hanzi)
            .map(|e| e.hanzi)
            .collect();
        for i in (1..others.len()).rev() {
            let j = self.rng_next() as usize % (i + 1);
            others.swap(i, j);
        }
        let correct_idx = self.rng_next() as usize % 4;
        let mut options = [others[0], others[1], others[2], entry.hanzi];
        options[3] = options[correct_idx];
        options[correct_idx] = entry.hanzi;
        self.combat = CombatState::WordBridgeChallenge {
            meaning: entry.meaning,
            correct_hanzi: entry.hanzi,
            correct_pinyin: entry.pinyin,
            options,
            correct_idx,
            bridge_x,
            bridge_y,
        };
        self.message = format!(
            "桥 Word Bridge! Which character means \"{}\"? Pick 1-4.",
            entry.meaning
        );
        self.message_timer = 120;
    }

    pub(super) fn start_locked_door(&mut self, door_x: i32, door_y: i32) {
        let vocab = vocab::vocab_for_floor(self.floor_num);
        if vocab.len() < 4 {
            self.message = "Not enough vocabulary.".into();
            self.message_timer = 60;
            return;
        }
        let idx = self.rng_next() as usize % vocab.len();
        let entry = vocab[idx];
        let mut others: Vec<&'static str> = vocab
            .iter()
            .filter(|e| e.meaning != entry.meaning)
            .map(|e| e.meaning)
            .collect();
        for i in (1..others.len()).rev() {
            let j = self.rng_next() as usize % (i + 1);
            others.swap(i, j);
        }
        let correct_idx = self.rng_next() as usize % 4;
        let mut options = [others[0], others[1], others[2], entry.meaning];
        options[3] = options[correct_idx];
        options[correct_idx] = entry.meaning;
        self.combat = CombatState::LockedDoorChallenge {
            hanzi: entry.hanzi,
            pinyin: entry.pinyin,
            correct_meaning: entry.meaning,
            options,
            correct_idx,
            door_x,
            door_y,
        };
        self.message = format!("锁 Locked Door! What does {} mean? Pick 1-4.", entry.hanzi);
        self.message_timer = 120;
    }

    pub(super) fn start_cursed_floor(&mut self) {
        let vocab = vocab::vocab_for_floor(self.floor_num);
        if vocab.is_empty() {
            self.message = "The curse fizzles.".into();
            self.message_timer = 60;
            return;
        }
        let idx = self.rng_next() as usize % vocab.len();
        let entry = vocab[idx];
        let tone = entry
            .pinyin
            .chars()
            .last()
            .and_then(|c| c.to_digit(10))
            .unwrap_or(1) as u8;
        self.combat = CombatState::CursedFloorChallenge {
            hanzi: entry.hanzi,
            pinyin: entry.pinyin,
            meaning: entry.meaning,
            correct_tone: tone,
        };
        self.message = format!(
            "咒 Cursed Floor! What tone is {} ({})? Press 1-4.",
            entry.hanzi, entry.meaning
        );
        self.message_timer = 120;
    }

    pub(super) fn forge_quest_candidates_for_floor(floor: i32) -> Vec<&'static radical::Recipe> {
        let available = radical::radicals_for_floor(floor.max(1));
        radical::RECIPES
            .iter()
            .filter(|recipe| {
                recipe
                    .inputs
                    .iter()
                    .all(|input| available.iter().any(|radical| radical.ch == *input))
            })
            .collect()
    }

    /// Generate a random quest.
    pub(super) fn generate_quest(&mut self) -> Quest {
        let floor = self.floor_num;
        match self.rng_next() % 4 {
            0 => {
                let target = 3 + (floor / 3) as i32;
                Quest {
                    description: format!("Defeat {} enemies", target),
                    goal: QuestGoal::KillEnemies(0, target),
                    gold_reward: 10 + floor * 3,
                    completed: false,
                    chain_step: 0,
                    chain_id: 0,
                }
            }
            1 => {
                let target_floor = floor + 2;
                Quest {
                    description: format!("Reach floor {}", target_floor),
                    goal: QuestGoal::ReachFloor(target_floor),
                    gold_reward: 14 + floor * 3,
                    completed: false,
                    chain_step: 0,
                    chain_id: 0,
                }
            }
            2 => {
                let target = 3 + (floor / 2) as i32;
                Quest {
                    description: format!("Collect {} radicals", target),
                    goal: QuestGoal::CollectRadicals(0, target),
                    gold_reward: 8 + floor * 2,
                    completed: false,
                    chain_step: 0,
                    chain_id: 0,
                }
            }
            _ => {
                let candidates = Self::forge_quest_candidates_for_floor(floor);
                if candidates.is_empty() {
                    let target = 3 + (floor / 2) as i32;
                    Quest {
                        description: format!("Collect {} radicals", target),
                        goal: QuestGoal::CollectRadicals(0, target),
                        gold_reward: 8 + floor * 2,
                        completed: false,
                        chain_step: 0,
                        chain_id: 0,
                    }
                } else {
                    let recipe = candidates[self.rng_next() as usize % candidates.len()];
                    Quest {
                        description: format!(
                            "Forge {} ({})",
                            recipe.output_hanzi, recipe.output_meaning
                        ),
                        goal: QuestGoal::ForgeCharacter(recipe.output_hanzi),
                        gold_reward: 12 + floor * 3,
                        completed: false,
                        chain_step: 0,
                        chain_id: 0,
                    }
                }
            }
        }
    }

    pub(super) fn generate_chain_quest(&mut self, step: u8, chain_id: u32) -> Quest {
        let floor = self.floor_num;
        let escalation = step as i32;
        match step {
            0 => {
                let target = 3 + (floor / 3) + escalation;
                Quest {
                    description: format!("⛓① Defeat {} enemies", target),
                    goal: QuestGoal::KillEnemies(0, target),
                    gold_reward: 7 + floor * 2,
                    completed: false,
                    chain_step: 1,
                    chain_id,
                }
            }
            1 => {
                let target = 3 + (floor / 2) + escalation;
                Quest {
                    description: format!("⛓② Collect {} radicals", target),
                    goal: QuestGoal::CollectRadicals(0, target),
                    gold_reward: 10 + floor * 3,
                    completed: false,
                    chain_step: 2,
                    chain_id,
                }
            }
            2 => {
                let candidates = Self::forge_quest_candidates_for_floor(floor);
                if !candidates.is_empty() {
                    let recipe = candidates[self.rng_next() as usize % candidates.len()];
                    Quest {
                        description: format!(
                            "⛓③ Forge {} ({})",
                            recipe.output_hanzi, recipe.output_meaning
                        ),
                        goal: QuestGoal::ForgeCharacter(recipe.output_hanzi),
                        gold_reward: 18 + floor * 4,
                        completed: false,
                        chain_step: 3,
                        chain_id,
                    }
                } else {
                    let target = 5 + (floor / 2) + escalation;
                    Quest {
                        description: format!("⛓③ Defeat {} enemies", target),
                        goal: QuestGoal::KillEnemies(0, target),
                        gold_reward: 18 + floor * 4,
                        completed: false,
                        chain_step: 3,
                        chain_id,
                    }
                }
            }
            _ => {
                let target_floor = floor + 3;
                Quest {
                    description: format!("⛓④ Reach floor {} (finale!)", target_floor),
                    goal: QuestGoal::ReachFloor(target_floor),
                    gold_reward: 28 + floor * 4,
                    completed: false,
                    chain_step: 4,
                    chain_id,
                }
            }
        }
    }
}
