//! Item system: chests, item usage, and phoenix revival.

use super::*;

impl GameState {
    pub(super) fn open_chest(&mut self, cx: i32, cy: i32) {
        // Chest open particles
        let (sx, sy) = self.tile_to_screen(cx, cy);
        self.particles.spawn_chest(sx, sy, &mut self.rng_state);
        self.achievements.unlock("first_chest");
        if let Some(ref audio) = self.audio {
            audio.play_treasure();
        }

        // Remove chest tile
        let idx = self.level.idx(cx, cy);
        self.level.tiles[idx] = Tile::MetalFloor;

        let roll = self.rng_next() % 100;
        if roll < 70 {
            // 70% — loot (item, gold, or radical)
            let loot_type = self.rng_next() % 3;
            match loot_type {
                0 => {
                    // Random item
                    let item = self.random_item();
                    let state = self.roll_item_state();
                    let name = self.item_display_name(&item);
                    let prefix = match state {
                        ItemState::Cursed => "💀 ",
                        ItemState::Blessed => "✨ ",
                        ItemState::Normal => "",
                    };
                    if self.player.add_item(item, state) {
                        self.ship.cargo_used = (self.ship.cargo_used + 1).min(self.ship.cargo_capacity);
                        self.message = format!("◆ Found {}{}!", prefix, name);
                        self.achievements.check_items(self.player.items.len());
                    } else {
                        self.message = "◆ Chest had an item but inventory is full!".to_string();
                    }
                }
                1 => {
                    // Gold
                    let base = 10 + (self.rng_next() % 20) as i32 + self.floor_num * 3;
                    let gold = ((base as f64) * self.floor_profile.gold_multiplier()) as i32;
                    let gold = gold.max(1);
                    self.player.gold += gold;
                    self.message = format!("◆ Found {}g!", gold);
                }
                _ => {
                    // Radical
                    let available = radical::radicals_for_floor(self.floor_num);
                    let drop_idx = self.rng_next() as usize % available.len();
                    let dropped = available[drop_idx].ch;
                    self.player.add_radical(dropped);
                    if self.floor_profile.radical_drop_bonus() {
                        let bonus_idx = self.rng_next() as usize % available.len();
                        let bonus = available[bonus_idx].ch;
                        self.player.add_radical(bonus);
                        self.message = format!("◆ Found radicals [{}] + [{}]!", dropped, bonus);
                    } else {
                        self.message = format!("◆ Found radical [{}]!", dropped);
                    }
                }
            }
            self.message_timer = 60;
        } else if roll < 90 {
            // 20% — trap
            let trap_type = self.rng_next() % 3;
            if trap_type == 0 {
                // Poison trap
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Poison { damage: 1 },
                    5,
                ));
                self.message = "◆ Trapped! Poisoned for 5 turns!".to_string();
            } else if trap_type == 1 {
                // Rooted trap
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Rooted,
                    4,
                ));
                self.message = "◆ Trapped! Gravity snare anchors you for 4 turns!".to_string();
            } else {
                // Damage trap
                let dmg = 2 + self.floor_num / 2;
                self.player.hp -= dmg;
                if self.player.hp <= 0 && !self.try_phoenix_revive() {
                    self.player.hp = 0;
                    if let Some(ref audio) = self.audio {
                        audio.play_death();
                    }
                    self.run_journal
                        .log(RunEvent::DiedTo("Spike trap".to_string(), self.floor_num));
                    self.post_mortem_page = 0;
                    self.combat = CombatState::GameOver;
                    self.message = self.run_summary();
                    self.message_timer = 255;
                } else {
                    self.message = format!("◆ Trapped! Spike trap deals {} damage!", dmg);
                }
            }
            self.message_timer = 60;
        } else {
            // 10% — mimic! Spawn an enemy here
            let pool = vocab::vocab_for_floor(self.floor_num);
            if !pool.is_empty() {
                let entry_idx = self.rng_next() as usize % pool.len();
                let entry: &'static VocabEntry = pool[entry_idx];
                let mut mimic = Enemy::from_vocab(entry, cx, cy, self.floor_num);
                mimic.hp = mimic.hp + 2; // mimics are tougher
                mimic.damage += 1;
                mimic.alert = true;
                mimic.gold_value *= 2; // better drops
                self.enemies.push(mimic);
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
                self.message = format!(
                    "◆ It's a Mimic! Type pinyin for {} ({})",
                    entry.hanzi, entry.meaning
                );
                self.message_timer = 255;
            }
        }
    }

    pub(super) fn roll_item_state(&mut self) -> ItemState {
        let roll = self.rng_next() % 100;
        if roll < 15 {
            ItemState::Cursed
        } else if roll < 25 {
            ItemState::Blessed
        } else {
            ItemState::Normal
        }
    }

    /// Generate a random item appropriate for the current floor.
    pub(super) fn random_item(&mut self) -> crate::player::Item {
        use crate::player::Item;
        match self.rng_next() % 32 {
            0 => Item::MedHypo(4 + self.floor_num),
            1 => Item::ToxinGrenade(2, 3),
            2 => Item::ScannerPulse,
            3 => Item::PersonalTeleporter,
            4 => Item::StimPack(5),
            5 => Item::EMPGrenade,
            6 | 7 => Item::RationPack(40),
            8 => Item::FocusStim(5),
            9 => Item::SynthAle(3),
            10 => Item::HoloDecoy(4),
            11 => Item::PlasmaBurst(3 + self.floor_num / 2),
            12 => Item::NanoShield(5),
            13 => Item::NeuralBoost,
            14 => Item::CreditChip(8 + self.floor_num * 2),
            15 => Item::ShockModule(5 + self.floor_num),
            16 => Item::BiogelPatch(2),
            17 => Item::VenomDart,
            18 => Item::DeflectorDrone(5),
            19 => Item::NaniteSwarm,
            20 => Item::ReflectorPlate,
            21 => Item::CryoGrenade(1),
            22 => Item::CloakingDevice(2),
            23 => Item::PlasmaShield(3),
            24 => Item::SignalJammer(2),
            25 => Item::NavComputer,
            26 => Item::GrappleLine,
            27 => Item::OmniGel,
            28 => Item::SonicEmitter(2),
            29 => Item::CircuitInk,
            30 => Item::DataCore(5),
            _ => Item::ThrusterPack,
        }
    }

    /// Use a consumable item from inventory.
    pub(super) fn use_item(&mut self, idx: usize) {
        if idx >= self.player.items.len() {
            return;
        }
        let item = self.player.items.remove(idx);
        let item_state = self.player.item_states.remove(idx);
        let kind = item.kind();
        let appearance = self.item_appearance(kind).to_string();
        let true_name = item.name();
        let newly_identified = self.identify_item_kind(kind);
        if let Some(ref audio) = self.audio {
            audio.play_spell();
        }

        match item {
            crate::player::Item::MedHypo(heal) => {
                let heal = if self.player.class == PlayerClass::Mechanic {
                    heal * 2
                } else {
                    heal
                };
                let heal = match item_state {
                    ItemState::Cursed => (heal / 2).max(1),
                    ItemState::Blessed => heal * 3 / 2,
                    ItemState::Normal => heal,
                };
                self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.message = format!(
                    "{}💚 Healed {} HP! ({}/{})",
                    prefix, heal, self.player.hp, self.player.max_hp
                );
                if item_state == ItemState::Cursed {
                    self.player
                        .statuses
                        .push(crate::status::StatusInstance::new(
                            crate::status::StatusKind::Poison { damage: 1 },
                            3,
                        ));
                    self.message.push_str(" But the curse poisons you!");
                } else {
                    self.player
                        .statuses
                        .push(crate::status::StatusInstance::new(
                            crate::status::StatusKind::Regen { heal: 2 },
                            3,
                        ));
                    self.message.push_str(" Auto-repair engaged for 3 turns!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::ToxinGrenade(dmg, turns) => {
                let dmg = match item_state {
                    ItemState::Blessed => dmg * 2,
                    _ => dmg,
                };
                let px = self.player.x;
                let py = self.player.y;
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() && (e.x - px).abs() <= 1 && (e.y - py).abs() <= 1 {
                        e.statuses.push(status::StatusInstance::new(
                            status::StatusKind::Poison { damage: dmg },
                            turns,
                        ));
                        count += 1;
                    }
                }
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.message = format!(
                    "{}☠ Poisoned {} enemies! ({} dmg × {} turns)",
                    prefix, count, dmg, turns
                );
                if item_state == ItemState::Cursed {
                    self.player
                        .statuses
                        .push(crate::status::StatusInstance::new(
                            crate::status::StatusKind::Poison { damage: 1 },
                            2,
                        ));
                    self.message.push_str(" But the fumes poison you!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::ScannerPulse => {
                self.reveal_entire_floor();
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.message = format!("{}👁 Map revealed!", prefix);
                if item_state == ItemState::Blessed {
                    self.player.hp = (self.player.hp + 3).min(self.player.max_hp);
                    self.message.push_str(" You feel revitalized! (+3 HP)");
                } else if item_state == ItemState::Cursed {
                    let mut spawned = false;
                    for dir in &[
                        (0, 1),
                        (1, 0),
                        (0, -1),
                        (-1, 0),
                        (1, 1),
                        (-1, -1),
                        (1, -1),
                        (-1, 1),
                    ] {
                        let tx = self.player.x + dir.0;
                        let ty = self.player.y + dir.1;
                        if tx > 0 && tx < MAP_W - 1 && ty > 0 && ty < MAP_H - 1 {
                            let idx = self.level.idx(tx, ty);
                            if self.level.tiles[idx].is_walkable()
                                && self.enemy_at(tx, ty).is_none()
                            {
                                let pool = crate::vocab::vocab_for_floor(self.floor_num);
                                if !pool.is_empty() {
                                    let entry = pool[self.rng_next() as usize % pool.len()];
                                    let mut e = Enemy::from_vocab(entry, tx, ty, self.floor_num);
                                    e.alert = true;
                                    self.enemies.push(e);
                                    spawned = true;
                                    break;
                                }
                            }
                        }
                    }
                    if spawned {
                        self.message.push_str(" An enemy was summoned!");
                    }
                }
                // Apply Revealed to all visible enemies (they take +25% damage)
                let mut revealed_count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() {
                        let i = self.level.idx(e.x, e.y);
                        if self.level.visible[i] {
                            e.statuses.push(status::StatusInstance::new(
                                status::StatusKind::Revealed,
                                5,
                            ));
                            revealed_count += 1;
                        }
                    }
                }
                if revealed_count > 0 {
                    self.message.push_str(&format!(
                        " {} enemies revealed — they take bonus damage!",
                        revealed_count
                    ));
                }
                self.message_timer = 60;
            }
            crate::player::Item::PersonalTeleporter => {
                // Find random explored walkable tile
                let mut candidates = Vec::new();
                for y in 0..self.level.height {
                    for x in 0..self.level.width {
                        let i = self.level.idx(x, y);
                        if self.level.revealed[i]
                            && self.level.tiles[i].is_walkable()
                            && self.enemy_at(x, y).is_none()
                            && (x != self.player.x || y != self.player.y)
                        {
                            candidates.push((x, y));
                        }
                    }
                }
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                if let Some(&(tx, ty)) =
                    candidates.get(self.rng_next() as usize % candidates.len().max(1))
                {
                    self.player.move_to(tx, ty);
                    let (px, py) = (self.player.x, self.player.y);
                    compute_fov(&mut self.level, px, py, FOV_RADIUS);
                    self.message = format!("{}✦ Teleported!", prefix);
                } else {
                    self.message = format!("{}Teleport fizzled — nowhere to go!", prefix);
                }
                if item_state == ItemState::Blessed {
                    self.player.shield = true;
                    self.message.push_str(" You gained a shield!");
                } else if item_state == ItemState::Cursed {
                    self.player.hp -= 2;
                    self.message.push_str(" The rough transit deals 2 damage!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::StimPack(turns) => {
                let turns = match item_state {
                    ItemState::Blessed => turns * 2,
                    _ => turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Poison { damage: 1 },
                        3,
                    ));
                    self.message =
                        format!("{}The foul concoction slows you (poison 3 turns)!", prefix);
                } else {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Haste,
                        turns,
                    ));
                    self.message = format!(
                        "{}⚡ Haste for {} turns! Enemies move at half speed.",
                        prefix, turns
                    );
                }
                self.message_timer = 60;
            }
            crate::player::Item::EMPGrenade => {
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() {
                        let i = self.level.idx(e.x, e.y);
                        if self.level.visible[i] {
                            e.stunned = true;
                            if item_state == ItemState::Blessed {
                                e.hp -= 2;
                            }
                            count += 1;
                        }
                    }
                }
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.message = format!("{}💥 Stunned {} enemies!", prefix, count);
                if item_state == ItemState::Blessed {
                    self.message.push_str(" (Dealt 2 holy damage to each!)");
                } else if item_state == ItemState::Cursed {
                    self.player.hp -= 2;
                    self.message
                        .push_str(" The blast backfires and deals 2 damage to you!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::RationPack(amount) => {
                let amount = match item_state {
                    ItemState::Cursed => amount / 2,
                    ItemState::Blessed => (amount as f64 * 1.5) as i32,
                    ItemState::Normal => amount,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.hp = (self.player.hp + amount).min(self.player.max_hp);
                self.message = format!("{}🍙 Restored {} HP!", prefix, amount);
                self.message_timer = 60;
            }
            crate::player::Item::FocusStim(turns) => {
                let turns = match item_state {
                    ItemState::Cursed => (turns / 2).max(1),
                    ItemState::Blessed => turns + 3,
                    ItemState::Normal => turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player
                    .statuses
                    .push(crate::status::StatusInstance::new(
                        crate::status::StatusKind::Shield,
                        turns,
                    ));
                self.message = format!("{}🛡 Shield active for {} turns!", prefix, turns);
                self.message_timer = 60;
            }
            crate::player::Item::SynthAle(confuse_turns) => {
                let confuse_turns = match item_state {
                    ItemState::Cursed => confuse_turns + 3,
                    ItemState::Blessed => 0,
                    ItemState::Normal => confuse_turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.hp = self.player.max_hp;
                if confuse_turns > 0 {
                    self.player
                        .statuses
                        .push(crate::status::StatusInstance::new(
                            crate::status::StatusKind::Confused,
                            confuse_turns,
                        ));
                    self.message = format!(
                        "{}🍶 Restored 5 HP! But you feel dizzy for {} turns...",
                        prefix, confuse_turns
                    );
                } else {
                    self.message = format!(
                        "{}🍶 Restored 5 HP! The blessed wine clears your mind!",
                        prefix
                    );
                }
                self.message_timer = 60;
            }
            crate::player::Item::HoloDecoy(turns) => {
                let turns = match item_state {
                    ItemState::Cursed => (turns / 2).max(1),
                    ItemState::Blessed => turns * 2,
                    ItemState::Normal => turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Haste,
                    turns,
                ));
                self.message = format!("{}🌫 Smoke screen! Haste for {} turns!", prefix, turns);
                if item_state == ItemState::Cursed {
                    self.player
                        .statuses
                        .push(status::StatusInstance::new(status::StatusKind::Confused, 2));
                    self.message.push_str(" The smoke disorients you!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::PlasmaBurst(damage) => {
                let damage = match item_state {
                    ItemState::Blessed => damage * 2,
                    _ => damage,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() {
                        let i = self.level.idx(e.x, e.y);
                        if self.level.visible[i] {
                            e.hp -= damage;
                            if item_state == ItemState::Blessed {
                                e.statuses.push(status::StatusInstance::new(
                                    status::StatusKind::Burn { damage: 1 },
                                    2,
                                ));
                            }
                            count += 1;
                        }
                    }
                }
                self.message = format!(
                    "{}🧨 Cracker hit {} enemies for {} damage!",
                    prefix, count, damage
                );
                if item_state == ItemState::Cursed {
                    let self_dmg = (damage / 2).max(1);
                    self.player.hp -= self_dmg;
                    self.message
                        .push_str(&format!(" Backfire! You take {} damage!", self_dmg));
                }
                self.message_timer = 60;
            }
            crate::player::Item::NanoShield(turns) => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let regen_amt = if item_state == ItemState::Blessed {
                    2
                } else {
                    1
                };
                let regen_turns = match item_state {
                    ItemState::Cursed => (turns / 2).max(1),
                    _ => turns,
                };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Regen { heal: regen_amt },
                    regen_turns,
                ));
                if item_state != ItemState::Cursed {
                    self.player.shield = true;
                    self.message = format!(
                        "{}🛡 Iron Skin! Shield + Regen({}) for {} turns!",
                        prefix, regen_amt, regen_turns
                    );
                } else {
                    self.message = format!(
                        "{}Regen({}) for {} turns, but no shield!",
                        prefix, regen_amt, regen_turns
                    );
                }
                self.message_timer = 60;
            }
            crate::player::Item::NeuralBoost => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                if item_state == ItemState::Cursed {
                    if !self.player.statuses.is_empty() {
                        let idx = self.rng_next() as usize % self.player.statuses.len();
                        let removed = self.player.statuses.remove(idx);
                        let _ = removed;
                    }
                    self.player
                        .statuses
                        .push(status::StatusInstance::new(status::StatusKind::Confused, 2));
                    self.message = format!("{}🍵 Removed one status, but now confused!", prefix);
                } else {
                    self.player.statuses.retain(|s| !s.is_negative());
                    self.message = format!("{}🍵 All negative effects purged!", prefix);
                    if item_state == ItemState::Blessed {
                        self.player.hp = (self.player.hp + 3).min(self.player.max_hp);
                        self.message.push_str(" +3 HP!");
                    }
                }
                self.message_timer = 60;
            }
            crate::player::Item::CreditChip(amount) => {
                let amount = match item_state {
                    ItemState::Cursed => (amount / 2).max(1),
                    ItemState::Blessed => amount * 2,
                    ItemState::Normal => amount,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.gold += amount;
                self.message = format!("{}🪙 Gained {} gold!", prefix, amount);
                self.message_timer = 60;
            }
            crate::player::Item::ShockModule(damage) => {
                let damage = match item_state {
                    ItemState::Cursed => (damage / 2).max(1),
                    ItemState::Blessed => damage * 2,
                    ItemState::Normal => damage,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let px = self.player.x;
                let py = self.player.y;
                let mut nearest: Option<(usize, i32)> = None;
                for (i, e) in self.enemies.iter().enumerate() {
                    if e.is_alive() {
                        let dist = (e.x - px).abs() + (e.y - py).abs();
                        if nearest.is_none() || dist < nearest.unwrap().1 {
                            nearest = Some((i, dist));
                        }
                    }
                }
                if let Some((idx, _)) = nearest {
                    self.enemies[idx].hp -= damage;
                    if item_state == ItemState::Blessed {
                        self.enemies[idx].stunned = true;
                    }
                    self.message = format!("{}⚡ Thunder strikes for {} damage!", prefix, damage);
                    if item_state == ItemState::Blessed {
                        self.message.push_str(" Target stunned!");
                    }
                } else {
                    self.message = format!("{}⚡ Thunder crackles but finds no target!", prefix);
                }
                self.message_timer = 60;
            }
            crate::player::Item::BiogelPatch(regen) => {
                let regen = match item_state {
                    ItemState::Cursed => (regen / 2).max(1),
                    ItemState::Blessed => regen * 2,
                    ItemState::Normal => regen,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Regen { heal: regen },
                    5,
                ));
                self.message = format!(
                    "{}💎 Jade Salve! Regen {} per turn for 5 turns!",
                    prefix, regen
                );
                if item_state == ItemState::Blessed {
                    self.player.hp = (self.player.hp + 2).min(self.player.max_hp);
                    self.message.push_str(" +2 HP!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::VenomDart => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let venom_turns = if item_state == ItemState::Blessed {
                    8
                } else {
                    5
                };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Envenomed,
                    venom_turns,
                ));
                self.message = format!("{}🐍 Weapon envenomed for {} turns!", prefix, venom_turns);
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Poison { damage: 1 },
                        3,
                    ));
                    self.message.push_str(" The venom bites back!");
                }
                if item_state == ItemState::Blessed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Empowered { amount: 1 },
                        5,
                    ));
                    self.message.push_str(" You feel empowered!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::DeflectorDrone(turns) => {
                let turns = match item_state {
                    ItemState::Cursed => (turns / 2).max(1),
                    _ => turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Shield,
                    turns,
                ));
                if item_state != ItemState::Cursed {
                    self.player.shield = true;
                    self.message = format!(
                        "{}🔮 Ward active! Double Shield for {} turns!",
                        prefix, turns
                    );
                } else {
                    self.message = format!(
                        "{}Shield for {} turns, but no physical shield!",
                        prefix, turns
                    );
                }
                if item_state == ItemState::Blessed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Regen { heal: 1 },
                        turns,
                    ));
                    self.message.push_str(" +Regen!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::NaniteSwarm => {
                let mut count = 0;
                let stun_turns = if item_state == ItemState::Blessed {
                    5
                } else {
                    3
                };
                for e in &mut self.enemies {
                    if e.is_alive() {
                        let i = self.level.idx(e.x, e.y);
                        if self.level.visible[i] {
                            e.stunned = true;
                            if item_state == ItemState::Blessed {
                                e.hp -= 1;
                            }
                            count += 1;
                        }
                    }
                }
                let _ = stun_turns;
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.message = format!("{}🖤 Ink splatters {} enemies!", prefix, count);
                if item_state == ItemState::Blessed {
                    self.message.push_str(" (Dealt 1 damage each!)");
                }
                if item_state == ItemState::Cursed {
                    self.player
                        .statuses
                        .push(status::StatusInstance::new(status::StatusKind::Confused, 2));
                    self.message
                        .push_str(" Ink splashes back — you're confused!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::Revitalizer(_) => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.message = format!(
                    "{}🔥 The Phoenix Plume glows warmly... it activates on death, not by hand.",
                    prefix
                );
                self.message_timer = 60;
                self.player.items.insert(idx, item);
                self.player.item_states.insert(idx, item_state);
            }
            crate::player::Item::ReflectorPlate => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let thorns_turns = if item_state == ItemState::Blessed { 3 } else { 1 };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Thorns,
                    thorns_turns,
                ));
                self.player.shield = true;
                self.message = format!(
                    "{}🪞 Mirror Shard activated! Next attack will be reflected!",
                    prefix
                );
                if item_state == ItemState::Cursed {
                    self.player.hp -= 1;
                    self.message.push_str(" A shard cuts you for 1 damage!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::CryoGrenade(turns) => {
                let turns = match item_state {
                    ItemState::Cursed => 1,
                    ItemState::Blessed => turns + 1,
                    ItemState::Normal => turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let px = self.player.x;
                let py = self.player.y;
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() && (e.x - px).abs() <= 1 && (e.y - py).abs() <= 1 {
                        e.statuses.push(status::StatusInstance::new(
                            status::StatusKind::Freeze,
                            turns,
                        ));
                        count += 1;
                    }
                }
                self.message = format!(
                    "{}❄ Frost Vial freezes {} adjacent enemies for {} turns!",
                    prefix, count, turns
                );
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Slow,
                        2,
                    ));
                    self.message.push_str(" The cold slows you too!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::CloakingDevice(turns) => {
                let turns = match item_state {
                    ItemState::Cursed => 1,
                    ItemState::Blessed => turns * 2,
                    ItemState::Normal => turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Invisible,
                    turns,
                ));
                self.message = format!(
                    "{}👻 Shadow Cloak! Invisible for {} turns!",
                    prefix, turns
                );
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Confused,
                        2,
                    ));
                    self.message.push_str(" The shadows disorient you!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::PlasmaShield(armor) => {
                let armor = match item_state {
                    ItemState::Cursed => (armor / 2).max(1),
                    ItemState::Blessed => armor + 2,
                    ItemState::Normal => armor,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Fortify { stacks: armor },
                    99,
                ));
                self.player.shield = true;
                self.message = format!(
                    "{}🐉 Dragon Scale! Gained +{} armor and a shield!",
                    prefix, armor
                );
                self.message_timer = 60;
            }
            crate::player::Item::SignalJammer(turns) => {
                let turns = match item_state {
                    ItemState::Cursed => 1,
                    ItemState::Blessed => turns + 2,
                    ItemState::Normal => turns,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() {
                        let i = self.level.idx(e.x, e.y);
                        if self.level.visible[i] {
                            e.statuses.push(status::StatusInstance::new(
                                status::StatusKind::Confused,
                                turns,
                            ));
                            count += 1;
                        }
                    }
                }
                self.message = format!(
                    "{}🎋 Bamboo Flute confuses {} enemies for {} turns!",
                    prefix, count, turns
                );
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Confused,
                        1,
                    ));
                    self.message.push_str(" The melody confuses you too!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::NavComputer => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.reveal_entire_floor();
                self.message = format!(
                    "{}🧭 Jade Compass reveals all traps and hidden areas!",
                    prefix
                );
                if item_state == ItemState::Blessed {
                    self.player.hp = (self.player.hp + 3).min(self.player.max_hp);
                    self.message.push_str(" You feel revitalized! (+3 HP)");
                } else if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Confused,
                        3,
                    ));
                    self.message.push_str(" The visions disorient you!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::GrappleLine => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let px = self.player.x;
                let py = self.player.y;
                let mut nearest: Option<(usize, i32)> = None;
                for (i, e) in self.enemies.iter().enumerate() {
                    if e.is_alive() {
                        let dist = (e.x - px).abs() + (e.y - py).abs();
                        if dist > 1 && (nearest.is_none() || dist < nearest.unwrap().1) {
                            nearest = Some((i, dist));
                        }
                    }
                }
                if let Some((eidx, _)) = nearest {
                    let dirs: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];
                    let mut placed = false;
                    for &(dx, dy) in &dirs {
                        let tx = px + dx;
                        let ty = py + dy;
                        let ti = self.level.idx(tx, ty);
                        if self.level.tiles[ti].is_walkable() && self.enemy_at(tx, ty).is_none() {
                            self.enemies[eidx].x = tx;
                            self.enemies[eidx].y = ty;
                            placed = true;
                            break;
                        }
                    }
                    if placed {
                        self.message = format!("{}🪢 Silk Rope pulls an enemy close!", prefix);
                        if item_state == ItemState::Blessed {
                            self.enemies[eidx].stunned = true;
                            self.message.push_str(" The enemy is stunned!");
                        }
                    } else {
                        self.message = format!("{}🪢 No space to pull enemy to!", prefix);
                    }
                } else {
                    self.message = format!("{}🪢 No distant enemies to pull!", prefix);
                }
                if item_state == ItemState::Cursed {
                    self.player.hp -= 1;
                    self.message.push_str(" The rope snaps back and hurts you!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::OmniGel => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Weakened,
                        3,
                    ));
                    self.message = format!("{}🪷 The tainted elixir weakens you!", prefix);
                } else {
                    self.player.statuses.retain(|s| !s.is_negative());
                    self.message = format!("{}🪷 Lotus Elixir purges all negative effects!", prefix);
                    if item_state == ItemState::Blessed {
                        self.player.statuses.push(status::StatusInstance::new(
                            status::StatusKind::Blessed,
                            5,
                        ));
                        self.message.push_str(" You feel blessed!");
                    }
                }
                self.message_timer = 60;
            }
            crate::player::Item::SonicEmitter(damage) => {
                let damage = match item_state {
                    ItemState::Cursed => (damage / 2).max(1),
                    ItemState::Blessed => damage * 2,
                    ItemState::Normal => damage,
                };
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() {
                        let i = self.level.idx(e.x, e.y);
                        if self.level.visible[i] {
                            e.hp -= damage;
                            e.statuses.push(status::StatusInstance::new(
                                status::StatusKind::Slow,
                                1,
                            ));
                            count += 1;
                        }
                    }
                }
                self.message = format!(
                    "{}🥁 Thunder Drum hits {} enemies for {} damage + Slow!",
                    prefix, count, damage
                );
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Slow,
                        1,
                    ));
                    self.message.push_str(" The vibration slows you too!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::CircuitInk => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let empower_amount = if item_state == ItemState::Blessed { 4 } else { 2 };
                let empower_turns = if item_state == ItemState::Cursed { 2 } else { 5 };
                self.player.statuses.push(status::StatusInstance::new(
                    status::StatusKind::Empowered { amount: empower_amount },
                    empower_turns,
                ));
                self.message = format!(
                    "{}🖊 Cinnabar Ink! +{} spell damage for {} turns!",
                    prefix, empower_amount, empower_turns
                );
                if item_state == ItemState::Cursed {
                    self.player.statuses.push(status::StatusInstance::new(
                        status::StatusKind::Weakened,
                        2,
                    ));
                    self.message.push_str(" But the ink weakens your body!");
                }
                self.message_timer = 60;
            }
            crate::player::Item::DataCore(_) => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                self.message = format!(
                    "{}🏺 The Ancestor Token hums gently... it activates on death, not by hand.",
                    prefix
                );
                self.message_timer = 60;
                self.player.items.insert(idx, item);
                self.player.item_states.insert(idx, item_state);
            }
            crate::player::Item::ThrusterPack => {
                let prefix = match item_state {
                    ItemState::Cursed => "💀 Cursed! ",
                    ItemState::Blessed => "✨ Blessed! ",
                    ItemState::Normal => "",
                };
                let px = self.player.x;
                let py = self.player.y;
                let push_dist = if item_state == ItemState::Blessed { 3 } else { 2 };
                let mut count = 0;
                for e in &mut self.enemies {
                    if e.is_alive() && (e.x - px).abs() <= 1 && (e.y - py).abs() <= 1 {
                        let dx = (e.x - px).signum();
                        let dy = (e.y - py).signum();
                        for _ in 0..push_dist {
                            let nx = e.x + dx;
                            let ny = e.y + dy;
                            if nx > 0 && nx < MAP_W - 1 && ny > 0 && ny < MAP_H - 1 {
                                let ni = self.level.idx(nx, ny);
                                if self.level.tiles[ni].is_walkable() {
                                    e.x = nx;
                                    e.y = ny;
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        count += 1;
                    }
                }
                self.message = format!(
                    "{}🌬 Wind Fan pushes {} enemies away!",
                    prefix, count
                );
                if item_state == ItemState::Cursed {
                    self.player.hp -= 1;
                    self.message.push_str(" The gust blows back on you for 1 damage!");
                }
                self.message_timer = 60;
            }
        }

        if newly_identified {
            self.message.push_str(&format!(
                " The {} reveals itself as {}.",
                appearance, true_name
            ));
        }
    }

    pub(super) fn try_phoenix_revive(&mut self) -> bool {
        if self.player.hp > 0 {
            return false;
        }
        let plume_pos = self
            .player
            .items
            .iter()
            .position(|item| matches!(item, crate::player::Item::Revitalizer(_)));
        if let Some(idx) = plume_pos {
            let item = self.player.items.remove(idx);
            let item_state = self.player.item_states.remove(idx);
            let heal = match &item {
                crate::player::Item::Revitalizer(h) => *h,
                _ => 1,
            };
            let heal = match item_state {
                ItemState::Cursed => (heal / 2).max(1),
                ItemState::Blessed => self.player.max_hp,
                ItemState::Normal => heal,
            };
            self.player.hp = heal.min(self.player.max_hp);
            self.message = format!(
                "🔥 The Phoenix Plume ignites! You are reborn with {} HP!",
                self.player.hp
            );
            if item_state == ItemState::Cursed {
                self.player
                    .statuses
                    .push(status::StatusInstance::new(status::StatusKind::Confused, 3));
                self.message.push_str(" But you feel disoriented...");
            }
            self.message_timer = 120;
            return true;
        }
        let token_pos = self
            .player
            .items
            .iter()
            .position(|item| matches!(item, crate::player::Item::DataCore(_)));
        if let Some(idx) = token_pos {
            let item = self.player.items.remove(idx);
            let item_state = self.player.item_states.remove(idx);
            let heal = match &item {
                crate::player::Item::DataCore(h) => *h,
                _ => 5,
            };
            let heal = match item_state {
                ItemState::Cursed => (heal / 2).max(1),
                ItemState::Blessed => heal * 2,
                ItemState::Normal => heal,
            };
            self.player.hp = heal.min(self.player.max_hp);
            self.message = format!(
                "🏺 The Ancestor Token glows! Your ancestors revive you with {} HP!",
                self.player.hp
            );
            if item_state == ItemState::Cursed {
                self.player
                    .statuses
                    .push(status::StatusInstance::new(status::StatusKind::Weakened, 3));
                self.message.push_str(" But you feel weakened...");
            }
            self.message_timer = 120;
            return true;
        }
        false
    }

}
