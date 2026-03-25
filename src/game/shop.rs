//! Shop and crafting logic.

use super::*;
use crate::player::{ItemState, EQUIPMENT_POOL};
use crate::radical;
use crate::rarity::{ItemRarity, RolledAffix};

#[derive(Clone, Debug)]
pub struct ShopItem {
    pub label: String,
    pub cost: i32,
    pub kind: ShopItemKind,
}

#[derive(Clone, Debug)]
pub enum ShopItemKind {
    Radical(&'static str),
    HealFull,
    Equipment(usize, ItemRarity, Vec<RolledAffix>), // index into EQUIPMENT_POOL + rarity + affixes
    Consumable(crate::player::Item),
}

impl super::GameState {
    pub(crate) fn discounted_cost(&self, base_cost: i32) -> i32 {
        let pct = 100 - self.effective_shop_discount_pct();
        let mut cost = ((base_cost * pct).max(0) + 99) / 100;
        let surcharge = (base_cost as f64 * self.theft_catches as f64 * 0.15) as i32;
        cost += surcharge;
        cost
    }

    pub(crate) fn generate_shop_items(&mut self) -> Vec<ShopItem> {
        let mut items = Vec::new();

        // Always offer heal
        items.push(ShopItem {
            label: "Full Heal".to_string(),
            cost: 12 + self.floor_num * 3,
            kind: ShopItemKind::HealFull,
        });

        // Offer 2 random radicals
        let available = radical::radicals_for_floor(self.floor_num);
        for _ in 0..2 {
            let idx = self.rng_next() as usize % available.len();
            let rad = available[idx];
            items.push(ShopItem {
                label: format!("Radical [{}] ({})", rad.ch, rad.meaning),
                cost: 8 + self.floor_num * 2,
                kind: ShopItemKind::Radical(rad.ch),
            });
        }

        // Offer 1 random equipment with rarity
        let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
        let eq = &EQUIPMENT_POOL[eq_idx];
        let luck_bonus = self.player.skill_tree.total_item_rarity_bonus();
        let rarity = crate::rarity::roll_rarity(self.floor_num, luck_bonus, self.rng_next());
        let affixes = crate::rarity::roll_affixes(rarity, self.rng_next());
        let display = crate::rarity::rarity_name(eq.name, rarity, &affixes);
        let rarity_tag = match rarity {
            ItemRarity::Normal => "".to_string(),
            _ => format!(" [{}]", rarity.label()),
        };
        items.push(ShopItem {
            label: format!("{}{} ({:?})", display, rarity_tag, eq.slot),
            cost: 20 + self.floor_num * 5,
            kind: ShopItemKind::Equipment(eq_idx, rarity, affixes),
        });

        // Offer 1 random consumable item
        let consumable = self.random_item();
        let cname = self.item_display_name(&consumable);
        items.push(ShopItem {
            label: cname,
            cost: 10 + self.floor_num * 2,
            kind: ShopItemKind::Consumable(consumable),
        });

        items.push(ShopItem {
            label: "🍙 Rice Ball".to_string(),
            cost: 5,
            kind: ShopItemKind::Consumable(crate::player::Item::RationPack(40)),
        });

        items
    }

    /// Buy item from shop.
    pub(crate) fn shop_buy(&mut self) {
        if let CombatState::Shopping { ref items, cursor } = self.combat.clone() {
            if cursor >= items.len() {
                return;
            }
            let item = &items[cursor];
            let effective_cost = self.discounted_cost(item.cost);
            if self.player.gold < effective_cost {
                self.message = format!(
                    "Not enough gold! Need {} (have {})",
                    effective_cost, self.player.gold
                );
                self.message_timer = 40;
                return;
            }
            self.player.gold -= effective_cost;
            if let Some(ref audio) = self.audio {
                audio.play_buy();
            }
            match &item.kind {
                ShopItemKind::Radical(ch) => {
                    self.player.add_radical(ch);
                    self.message = format!("Bought radical [{}]!", ch);
                }
                ShopItemKind::HealFull => {
                    self.player.hp = self.player.max_hp;
                    self.message = "Fully healed!".to_string();
                }
                ShopItemKind::Equipment(idx, rarity, affixes) => {
                    let eq = &EQUIPMENT_POOL[*idx];
                    let current_state = self.player.equipment_state(eq.slot);
                    if current_state == ItemState::Cursed {
                        self.message = format!(
                            "💀 Your {} slot is cursed! Visit an altar to purify.",
                            match eq.slot {
                                crate::player::EquipSlot::Weapon => "weapon",
                                crate::player::EquipSlot::Armor => "armor",
                                crate::player::EquipSlot::Charm => "charm",
                            }
                        );
                        self.player.gold += effective_cost; // refund
                    } else {
                        let display = crate::rarity::rarity_name(eq.name, *rarity, affixes);
                        self.player.equip_with_rarity(eq, ItemState::Normal, *rarity, affixes.clone());
                        self.message = format!("Equipped {}!", display);
                    }
                }
                ShopItemKind::Consumable(consumable) => {
                    let name = self.item_display_name(consumable);
                    if self.player.add_item(consumable.clone(), ItemState::Normal) {
                        self.message = format!("Bought {}!", name);
                    } else {
                        self.message = "Inventory full!".to_string();
                        self.player.gold += effective_cost; // refund
                    }
                }
            }
            self.message_timer = 60;
        }
    }

    /// Sell an item from the player's inventory.
    pub(crate) fn shop_sell(&mut self) {
        if let CombatState::Shopping { ref mut cursor, .. } = self.combat {
            if self.player.items.is_empty() {
                self.message = "No items to sell!".to_string();
                self.message_timer = 40;
                return;
            }
            if *cursor >= self.player.items.len() {
                *cursor = self.player.items.len().saturating_sub(1);
                return;
            }
            let idx = *cursor;
            let item = &self.player.items[idx];
            let price = item.sell_price();
            let name = self.item_display_name(item);
            self.player.gold += price;
            self.player.items.remove(idx);
            self.player.item_states.remove(idx);
            self.ship.cargo_used = (self.ship.cargo_used - 1).max(0);
            if let Some(ref audio) = self.audio {
                audio.play_buy();
            }
            self.message = format!("Sold {} for {}g", name, price);
            self.message_timer = 60;
            // Adjust cursor if it's now past the end
            if let CombatState::Shopping { ref mut cursor, .. } = self.combat {
                if *cursor >= self.player.items.len() && *cursor > 0 {
                    *cursor -= 1;
                }
            }
        }
    }

    /// Attempt to steal the currently highlighted shop item.
    pub(crate) fn shop_steal(&mut self) {
        if self.shop_banned {
            self.message = "🚫 The shopkeeper refuses to serve you!".to_string();
            self.message_timer = 60;
            return;
        }

        if let CombatState::Shopping { ref items, cursor } = self.combat.clone() {
            if cursor >= items.len() {
                return;
            }

            let mut chance: i64 = 40;
            if self.player.class == PlayerClass::Operative {
                chance += 25;
            }
            if self.player.class == PlayerClass::Operative {
                chance += 15;
            }
            chance -= (self.theft_catches as i64) * 10;
            chance = chance.clamp(5, 80);

            let roll = (self.rng_next() % 100) as i64;

            if roll < chance {
                let item = &items[cursor];
                match &item.kind {
                    ShopItemKind::Radical(ch) => {
                        self.player.add_radical(ch);
                        self.message = format!(
                            "🤫 You pocket radical [{}] while the shopkeeper looks away!",
                            ch
                        );
                    }
                    ShopItemKind::HealFull => {
                        self.player.hp = self.player.max_hp;
                        self.message = "🤫 You sip the healing brew unnoticed!".to_string();
                    }
                    ShopItemKind::Equipment(idx, rarity, affixes) => {
                        let eq = &EQUIPMENT_POOL[*idx];
                        let display = crate::rarity::rarity_name(eq.name, *rarity, affixes);
                        self.player.equip_with_rarity(eq, ItemState::Normal, *rarity, affixes.clone());
                        self.message =
                            format!("🤫 You slip on the {} when nobody's watching!", display);
                    }
                    ShopItemKind::Consumable(consumable) => {
                        if !self.player.add_item(consumable.clone(), ItemState::Normal) {
                            self.message = "Inventory full — can't steal!".to_string();
                            self.message_timer = 40;
                            return;
                        }
                        self.message = "🤫 Five-finger discount! Item pocketed.".to_string();
                    }
                }
                self.message_timer = 80;
                if let Some(ref audio) = self.audio {
                    audio.play_buy();
                }
                if let CombatState::Shopping {
                    ref mut items,
                    ref mut cursor,
                } = self.combat
                {
                    if *cursor < items.len() {
                        items.remove(*cursor);
                        if *cursor >= items.len() && *cursor > 0 {
                            *cursor -= 1;
                        }
                    }
                }
            } else {
                self.theft_catches += 1;
                self.shop_banned = true;
                let dmg = 3 + self.theft_catches as i32;
                self.player.hp -= dmg;
                self.combat = CombatState::Explore;
                self.message = format!(
                    "🚨 Caught stealing! The shopkeeper strikes you for {} damage and throws you out!",
                    dmg
                );
                self.message_timer = 100;
                if let Some(ref audio) = self.audio {
                    audio.play_damage();
                }
                self.trigger_shake(10);
                self.flash = Some((255, 50, 50, 0.3));

                if self.player.hp <= 0 && !self.try_phoenix_revive() {
                    self.player.hp = 0;
                    self.run_journal.log(RunEvent::DiedTo(
                        "Angry shopkeeper".to_string(),
                        self.floor_num,
                    ));
                    self.post_mortem_page = 0;
                    self.combat = CombatState::GameOver;
                    self.message = self.run_summary();
                    self.message_timer = 255;
                    if let Some(ref audio) = self.audio {
                        audio.play_death();
                    }
                }
            }
        }
    }

    /// Merchant L3 perk: reroll one shop item per floor.
    pub(crate) fn shop_reroll(&mut self) {
        if self.merchant_reroll_used {
            self.message = "Already rerolled this floor!".to_string();
            self.message_timer = 40;
            return;
        }
        let has_merchant_l3 =
            self.companion == Some(Companion::Quartermaster) && self.companion_level() >= 3;
        if !has_merchant_l3 {
            return;
        }
        // Phase 1: Extract cursor and old item kind (immutable borrow, then drop)
        let (cursor, old_kind) = if let CombatState::Shopping { ref items, cursor } = self.combat {
            if cursor < items.len() {
                (cursor, items[cursor].kind.clone())
            } else {
                return;
            }
        } else {
            return;
        };
        // Phase 2: Generate new item (free to use &mut self)
        let new_item = match old_kind {
            ShopItemKind::HealFull => ShopItem {
                label: "Full Heal".to_string(),
                cost: 20 + self.floor_num * 4,
                kind: ShopItemKind::HealFull,
            },
            ShopItemKind::Radical(_) => {
                let available = radical::radicals_for_floor(self.floor_num);
                let idx = self.rng_next() as usize % available.len();
                let rad = available[idx];
                ShopItem {
                    label: format!("Radical [{}] ({})", rad.ch, rad.meaning),
                    cost: 12 + self.floor_num * 2,
                    kind: ShopItemKind::Radical(rad.ch),
                }
            }
            ShopItemKind::Equipment(..) => {
                let eq_idx = self.rng_next() as usize % EQUIPMENT_POOL.len();
                let eq = &EQUIPMENT_POOL[eq_idx];
                let luck_bonus = self.player.skill_tree.total_item_rarity_bonus();
                let rarity = crate::rarity::roll_rarity(self.floor_num, luck_bonus, self.rng_next());
                let affixes = crate::rarity::roll_affixes(rarity, self.rng_next());
                let display = crate::rarity::rarity_name(eq.name, rarity, &affixes);
                let rarity_tag = match rarity {
                    ItemRarity::Normal => "".to_string(),
                    _ => format!(" [{}]", rarity.label()),
                };
                ShopItem {
                    label: format!("{}{} ({:?})", display, rarity_tag, eq.slot),
                    cost: 30 + self.floor_num * 6,
                    kind: ShopItemKind::Equipment(eq_idx, rarity, affixes),
                }
            }
            ShopItemKind::Consumable(_) => {
                let consumable = self.random_item();
                let cname = self.item_display_name(&consumable);
                ShopItem {
                    label: cname,
                    cost: 15 + self.floor_num * 3,
                    kind: ShopItemKind::Consumable(consumable),
                }
            }
        };
        // Phase 3: Replace item in shop
        if let CombatState::Shopping { ref mut items, .. } = self.combat {
            items[cursor] = new_item;
        }
        self.merchant_reroll_used = true;
        self.message = "💰 Merchant rerolled the item!".to_string();
        self.message_timer = 60;
    }

}
