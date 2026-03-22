//! Player state and movement.

use crate::radical::Spell;
use crate::status::StatusInstance;

/// Equipment slot types
#[derive(Clone, Debug)]
pub struct Equipment {
    pub name: &'static str,
    pub slot: EquipSlot,
    pub effect: EquipEffect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipSlot {
    Weapon,
    Armor,
    Charm,
}

#[derive(Clone, Copy, Debug)]
pub enum EquipEffect {
    /// Extra damage on correct answer
    BonusDamage(i32),
    /// Reduce incoming damage
    DamageReduction(i32),
    /// Extra radical drop chance (percentage 0-100)
    ExtraRadicalDrop(i32),
    /// Heal on kill
    HealOnKill(i32),
    /// Extra gold on kill
    GoldBonus(i32),
    /// Allows digging through walls
    Digging,
    /// Halves spirit drain rate (drain 1 per 2 moves instead of every move)
    SpiritSustain,
    /// +1 to all spell damage
    SpellPowerBoost(i32),
    /// Heal HP per kill (weapon variant)
    LifeSteal(i32),
    /// Percentage chance to avoid attacks entirely
    DodgeChance(i32),
    /// Restore focus per turn
    FocusRegen(i32),
    /// Basic attacks push enemies 1 tile
    KnockbackStrike,
    /// Attackers take damage when hitting you
    ThornsAura(i32),
    /// Always see enemy intent
    EnemyIntentReveal,
    /// Percentage chance for double damage
    CriticalStrike(i32),
}

impl EquipEffect {
    pub fn description(&self) -> String {
        match self {
            EquipEffect::BonusDamage(n) => {
                format!("Deals +{} bonus damage on correct combat answers.", n)
            }
            EquipEffect::DamageReduction(n) => format!("Reduces incoming damage by {} per hit.", n),
            EquipEffect::ExtraRadicalDrop(n) => format!(
                "{}% extra chance to find radical drops from defeated enemies.",
                n
            ),
            EquipEffect::HealOnKill(n) => {
                format!("Restores {} HP whenever you defeat an enemy.", n)
            }
            EquipEffect::GoldBonus(n) => {
                format!("Earn +{} bonus gold from each enemy defeated.", n)
            }
            EquipEffect::Digging => {
                "Allows you to dig through dungeon walls by walking into them.".to_string()
            }
            EquipEffect::SpiritSustain => {
                "Halves spirit drain rate — lose 1 spirit every 2 moves instead of every move."
                    .to_string()
            }
            EquipEffect::SpellPowerBoost(n) => {
                format!("Adds +{} bonus damage to all spells.", n)
            }
            EquipEffect::LifeSteal(n) => {
                format!("Heals {} HP whenever you defeat an enemy.", n)
            }
            EquipEffect::DodgeChance(n) => {
                format!("{}% chance to completely avoid incoming attacks.", n)
            }
            EquipEffect::FocusRegen(n) => {
                format!("Restores {} focus each turn.", n)
            }
            EquipEffect::KnockbackStrike => {
                "Basic attacks push enemies back 1 tile.".to_string()
            }
            EquipEffect::ThornsAura(n) => {
                format!("Attackers take {} damage when they hit you.", n)
            }
            EquipEffect::EnemyIntentReveal => {
                "Always reveals what enemies intend to do next.".to_string()
            }
            EquipEffect::CriticalStrike(n) => {
                format!("{}% chance to deal double damage on attacks.", n)
            }
        }
    }
}

impl Equipment {
    pub fn description(&self) -> String {
        let slot = match self.slot {
            EquipSlot::Weapon => "Weapon",
            EquipSlot::Armor => "Armor",
            EquipSlot::Charm => "Charm",
        };
        format!("[{}] {}", slot, self.effect.description())
    }
}

#[allow(dead_code)]
pub const MAX_ITEMS: usize = 5;
pub const ITEM_KIND_COUNT: usize = 32;
pub const MYSTERY_ITEM_APPEARANCES: [&str; ITEM_KIND_COUNT] = [
    "Vermilion Seal 朱符",
    "Jade Seal 玉符",
    "Cloud Seal 云符",
    "Ink Seal 墨符",
    "Mirror Seal 镜符",
    "Storm Seal 雷符",
    "Phoenix Seal 凤符",
    "Moon Seal 月符",
    "Dragon Seal 龙符",
    "Tiger Seal 虎符",
    "Flame Seal 火符",
    "Mountain Seal 山符",
    "Lotus Seal 莲符",
    "River Seal 河符",
    "Star Seal 星符",
    "Serpent Seal 蛇符",
    "Bamboo Seal 竹符",
    "Iron Seal 铁符",
    "Sun Seal 日符",
    "Wind Seal 风符",
    "Frost Seal 霜符",
    "Shadow Seal 影符",
    "Pearl Seal 珠符",
    "Crane Seal 鹤符",
    "Thunder Seal 雷符",
    "Silk Seal 丝符",
    "Orchid Seal 兰符",
    "Willow Seal 柳符",
    "Amber Seal 琥符",
    "Coral Seal 珊符",
    "Spirit Seal 灵符",
    "Dawn Seal 曦符",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemKind {
    HealthPotion,
    PoisonFlask,
    RevealScroll,
    TeleportScroll,
    HastePotion,
    StunBomb,
    RiceBall,
    MeditationIncense,
    AncestralWine,
    SmokeScreen,
    FireCracker,
    IronSkinElixir,
    ClarityTea,
    GoldIngot,
    ThunderTalisman,
    JadeSalve,
    SerpentFang,
    WardingCharm,
    InkBomb,
    PhoenixPlume,
    MirrorShard,
    FrostVial,
    ShadowCloak,
    DragonScale,
    BambooFlute,
    JadeCompass,
    SilkRope,
    LotusElixir,
    ThunderDrum,
    CinnabarInk,
    AncestorToken,
    WindFan,
}

/// A recipe for combining two consumable items into a new one.
pub struct CraftingRecipe {
    pub input1: ItemKind,
    pub input2: ItemKind,
    pub output: ItemKind,
    pub output_name: &'static str,
}

pub const CRAFTING_RECIPES: &[CraftingRecipe] = &[
    CraftingRecipe { input1: ItemKind::HealthPotion, input2: ItemKind::PoisonFlask, output: ItemKind::LotusElixir, output_name: "🪷 Lotus Elixir" },
    CraftingRecipe { input1: ItemKind::FireCracker, input2: ItemKind::InkBomb, output: ItemKind::ThunderDrum, output_name: "🥁 Thunder Drum" },
    CraftingRecipe { input1: ItemKind::FrostVial, input2: ItemKind::ThunderDrum, output: ItemKind::StunBomb, output_name: "💥 Stun Bomb" },
    CraftingRecipe { input1: ItemKind::HealthPotion, input2: ItemKind::HealthPotion, output: ItemKind::AncestralWine, output_name: "🍶 Ancestral Wine" },
    CraftingRecipe { input1: ItemKind::SmokeScreen, input2: ItemKind::PoisonFlask, output: ItemKind::ShadowCloak, output_name: "👻 Shadow Cloak" },
    CraftingRecipe { input1: ItemKind::IronSkinElixir, input2: ItemKind::DragonScale, output: ItemKind::WardingCharm, output_name: "🔮 Warding Charm" },
    CraftingRecipe { input1: ItemKind::MeditationIncense, input2: ItemKind::ClarityTea, output: ItemKind::LotusElixir, output_name: "🪷 Lotus Elixir" },
    CraftingRecipe { input1: ItemKind::SerpentFang, input2: ItemKind::PoisonFlask, output: ItemKind::SerpentFang, output_name: "🐍 Serpent Fang+" },
    CraftingRecipe { input1: ItemKind::FireCracker, input2: ItemKind::FireCracker, output: ItemKind::FireCracker, output_name: "🧨 Fire Cracker+" },
    CraftingRecipe { input1: ItemKind::JadeSalve, input2: ItemKind::HealthPotion, output: ItemKind::JadeSalve, output_name: "💎 Jade Salve+" },
    CraftingRecipe { input1: ItemKind::WindFan, input2: ItemKind::SilkRope, output: ItemKind::BambooFlute, output_name: "🎋 Bamboo Flute" },
    CraftingRecipe { input1: ItemKind::MirrorShard, input2: ItemKind::WardingCharm, output: ItemKind::MirrorShard, output_name: "🪞 Mirror Shard+" },
    CraftingRecipe { input1: ItemKind::ThunderTalisman, input2: ItemKind::FrostVial, output: ItemKind::ThunderDrum, output_name: "🥁 Thunder Drum" },
    CraftingRecipe { input1: ItemKind::GoldIngot, input2: ItemKind::GoldIngot, output: ItemKind::PhoenixPlume, output_name: "🔥 Phoenix Plume" },
    CraftingRecipe { input1: ItemKind::HastePotion, input2: ItemKind::ClarityTea, output: ItemKind::HastePotion, output_name: "⚡ Haste Potion+" },
];

/// Look up a crafting recipe by two item kinds (order-independent).
pub fn find_crafting_recipe(a: ItemKind, b: ItemKind) -> Option<&'static CraftingRecipe> {
    CRAFTING_RECIPES.iter().find(|r| {
        (r.input1 == a && r.input2 == b) || (r.input1 == b && r.input2 == a)
    })
}

/// Check whether an item kind can pair with another to form any recipe.
pub fn has_recipe_with(selected: ItemKind, candidate: ItemKind) -> bool {
    find_crafting_recipe(selected, candidate).is_some()
}

/// Create the output item for a crafting recipe, using input items to scale stats.
pub fn crafted_item(recipe: &CraftingRecipe, item1: &Item, item2: &Item) -> Item {
    match recipe.output {
        ItemKind::LotusElixir => Item::LotusElixir,
        ItemKind::ThunderDrum => {
            let base = match (item1, item2) {
                (Item::FireCracker(d), _) | (_, Item::FireCracker(d)) => *d,
                (Item::ThunderTalisman(d), _) | (_, Item::ThunderTalisman(d)) => *d,
                (Item::ThunderDrum(d), _) | (_, Item::ThunderDrum(d)) => *d,
                _ => 4,
            };
            Item::ThunderDrum(base + 3)
        }
        ItemKind::StunBomb => Item::StunBomb,
        ItemKind::AncestralWine => Item::AncestralWine(6),
        ItemKind::ShadowCloak => Item::ShadowCloak(5),
        ItemKind::WardingCharm => {
            let base = match (item1, item2) {
                (Item::WardingCharm(d), _) | (_, Item::WardingCharm(d)) => *d,
                (Item::IronSkinElixir(d), _) | (_, Item::IronSkinElixir(d)) => *d,
                _ => 5,
            };
            Item::WardingCharm(base + 3)
        }
        ItemKind::SerpentFang => Item::SerpentFang,
        ItemKind::FireCracker => {
            let base = match (item1, item2) {
                (Item::FireCracker(d1), Item::FireCracker(d2)) => (*d1).max(*d2),
                (Item::FireCracker(d), _) | (_, Item::FireCracker(d)) => *d,
                _ => 5,
            };
            Item::FireCracker(base + 4)
        }
        ItemKind::JadeSalve => {
            let base = match (item1, item2) {
                (Item::JadeSalve(d), _) | (_, Item::JadeSalve(d)) => *d,
                _ => 2,
            };
            Item::JadeSalve(base + 2)
        }
        ItemKind::BambooFlute => Item::BambooFlute(4),
        ItemKind::MirrorShard => Item::MirrorShard,
        ItemKind::PhoenixPlume => Item::PhoenixPlume(15),
        ItemKind::HastePotion => {
            let base = match (item1, item2) {
                (Item::HastePotion(d), _) | (_, Item::HastePotion(d)) => *d,
                _ => 5,
            };
            Item::HastePotion(base + 3)
        }
        // Fallback for any future recipes
        _ => Item::HealthPotion(10),
    }
}

impl ItemKind {
    pub fn index(self) -> usize {
        match self {
            ItemKind::HealthPotion => 0,
            ItemKind::PoisonFlask => 1,
            ItemKind::RevealScroll => 2,
            ItemKind::TeleportScroll => 3,
            ItemKind::HastePotion => 4,
            ItemKind::StunBomb => 5,
            ItemKind::RiceBall => 6,
            ItemKind::MeditationIncense => 7,
            ItemKind::AncestralWine => 8,
            ItemKind::SmokeScreen => 9,
            ItemKind::FireCracker => 10,
            ItemKind::IronSkinElixir => 11,
            ItemKind::ClarityTea => 12,
            ItemKind::GoldIngot => 13,
            ItemKind::ThunderTalisman => 14,
            ItemKind::JadeSalve => 15,
            ItemKind::SerpentFang => 16,
            ItemKind::WardingCharm => 17,
            ItemKind::InkBomb => 18,
            ItemKind::PhoenixPlume => 19,
            ItemKind::MirrorShard => 20,
            ItemKind::FrostVial => 21,
            ItemKind::ShadowCloak => 22,
            ItemKind::DragonScale => 23,
            ItemKind::BambooFlute => 24,
            ItemKind::JadeCompass => 25,
            ItemKind::SilkRope => 26,
            ItemKind::LotusElixir => 27,
            ItemKind::ThunderDrum => 28,
            ItemKind::CinnabarInk => 29,
            ItemKind::AncestorToken => 30,
            ItemKind::WindFan => 31,
        }
    }
}

/// Consumable items the player can carry and use.
#[derive(Clone, Debug)]
pub enum Item {
    /// Heal N HP instantly
    HealthPotion(i32),
    /// Apply poison (dmg, turns) to adjacent enemies
    PoisonFlask(i32, i32),
    /// Reveal entire floor map
    RevealScroll,
    /// Teleport to random explored walkable tile
    TeleportScroll,
    /// Grant haste for N turns
    HastePotion(i32),
    /// Stun all visible enemies
    StunBomb,
    /// Restore spirit energy
    RiceBall(i32),
    /// Block spirit drain for N moves
    MeditationIncense(i32),
    /// Full spirit restore + Confused for N turns
    AncestralWine(i32),
    /// Grant Haste for N turns (smoke cover)
    SmokeScreen(i32),
    /// AoE damage to all visible enemies
    FireCracker(i32),
    /// Grant Shield + Regen for N turns
    IronSkinElixir(i32),
    /// Remove all negative status effects
    ClarityTea,
    /// Gain N gold instantly
    GoldIngot(i32),
    /// High damage to nearest enemy
    ThunderTalisman(i32),
    /// Regen N per turn for 5 turns
    JadeSalve(i32),
    /// Apply Envenomed to self weapon for 5 turns
    SerpentFang,
    /// Grant Shield + SpiritShield for N turns
    WardingCharm(i32),
    /// Stun all visible enemies + confuse (like enhanced StunBomb)
    InkBomb,
    /// Auto-revive on death, restoring N HP (passive, consumed on death)
    PhoenixPlume(i32),
    /// Reflect next attack back at attacker (1 use)
    MirrorShard,
    /// Freeze all adjacent enemies for N turns
    FrostVial(i32),
    /// Become invisible for N turns
    ShadowCloak(i32),
    /// Gain +N armor for the rest of combat
    DragonScale(i32),
    /// Confuse all enemies for N turns
    BambooFlute(i32),
    /// Reveal all traps and hidden tiles on the floor
    JadeCompass,
    /// Pull a distant enemy to adjacent tile
    SilkRope,
    /// Cure all negative status effects
    LotusElixir,
    /// Deal N damage to all enemies + Slow 1
    ThunderDrum(i32),
    /// Next spell deals double damage
    CinnabarInk,
    /// Revive with N HP on death (passive, consumed on use)
    AncestorToken(i32),
    /// Push all adjacent enemies 2 tiles away
    WindFan,
}

impl Item {
    pub fn kind(&self) -> ItemKind {
        match self {
            Item::HealthPotion(_) => ItemKind::HealthPotion,
            Item::PoisonFlask(_, _) => ItemKind::PoisonFlask,
            Item::RevealScroll => ItemKind::RevealScroll,
            Item::TeleportScroll => ItemKind::TeleportScroll,
            Item::HastePotion(_) => ItemKind::HastePotion,
            Item::StunBomb => ItemKind::StunBomb,
            Item::RiceBall(_) => ItemKind::RiceBall,
            Item::MeditationIncense(_) => ItemKind::MeditationIncense,
            Item::AncestralWine(_) => ItemKind::AncestralWine,
            Item::SmokeScreen(_) => ItemKind::SmokeScreen,
            Item::FireCracker(_) => ItemKind::FireCracker,
            Item::IronSkinElixir(_) => ItemKind::IronSkinElixir,
            Item::ClarityTea => ItemKind::ClarityTea,
            Item::GoldIngot(_) => ItemKind::GoldIngot,
            Item::ThunderTalisman(_) => ItemKind::ThunderTalisman,
            Item::JadeSalve(_) => ItemKind::JadeSalve,
            Item::SerpentFang => ItemKind::SerpentFang,
            Item::WardingCharm(_) => ItemKind::WardingCharm,
            Item::InkBomb => ItemKind::InkBomb,
            Item::PhoenixPlume(_) => ItemKind::PhoenixPlume,
            Item::MirrorShard => ItemKind::MirrorShard,
            Item::FrostVial(_) => ItemKind::FrostVial,
            Item::ShadowCloak(_) => ItemKind::ShadowCloak,
            Item::DragonScale(_) => ItemKind::DragonScale,
            Item::BambooFlute(_) => ItemKind::BambooFlute,
            Item::JadeCompass => ItemKind::JadeCompass,
            Item::SilkRope => ItemKind::SilkRope,
            Item::LotusElixir => ItemKind::LotusElixir,
            Item::ThunderDrum(_) => ItemKind::ThunderDrum,
            Item::CinnabarInk => ItemKind::CinnabarInk,
            Item::AncestorToken(_) => ItemKind::AncestorToken,
            Item::WindFan => ItemKind::WindFan,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Item::HealthPotion(_) => "💚 Health Potion",
            Item::PoisonFlask(_, _) => "☠ Poison Flask",
            Item::RevealScroll => "👁 Reveal Scroll",
            Item::TeleportScroll => "✦ Teleport Scroll",
            Item::HastePotion(_) => "⚡ Haste Potion",
            Item::StunBomb => "💥 Stun Bomb",
            Item::RiceBall(_) => "🍙 Rice Ball",
            Item::MeditationIncense(_) => "🧘 Meditation Incense",
            Item::AncestralWine(_) => "🍶 Ancestral Wine",
            Item::SmokeScreen(_) => "🌫 Smoke Screen",
            Item::FireCracker(_) => "🧨 Fire Cracker",
            Item::IronSkinElixir(_) => "🛡 Iron Skin Elixir",
            Item::ClarityTea => "🍵 Clarity Tea",
            Item::GoldIngot(_) => "🪙 Gold Ingot",
            Item::ThunderTalisman(_) => "⚡ Thunder Talisman",
            Item::JadeSalve(_) => "💎 Jade Salve",
            Item::SerpentFang => "🐍 Serpent Fang",
            Item::WardingCharm(_) => "🔮 Warding Charm",
            Item::InkBomb => "🖤 Ink Bomb",
            Item::PhoenixPlume(_) => "🔥 Phoenix Plume",
            Item::MirrorShard => "🪞 Mirror Shard",
            Item::FrostVial(_) => "❄ Frost Vial",
            Item::ShadowCloak(_) => "👻 Shadow Cloak",
            Item::DragonScale(_) => "🐉 Dragon Scale",
            Item::BambooFlute(_) => "🎋 Bamboo Flute",
            Item::JadeCompass => "🧭 Jade Compass",
            Item::SilkRope => "🪢 Silk Rope",
            Item::LotusElixir => "🪷 Lotus Elixir",
            Item::ThunderDrum(_) => "🥁 Thunder Drum",
            Item::CinnabarInk => "🖊 Cinnabar Ink",
            Item::AncestorToken(_) => "🏺 Ancestor Token",
            Item::WindFan => "🌬 Wind Fan",
        }
    }

    #[allow(dead_code)]
    pub fn short_name(&self) -> &'static str {
        match self {
            Item::HealthPotion(_) => "HP Pot",
            Item::PoisonFlask(_, _) => "Poison",
            Item::RevealScroll => "Reveal",
            Item::TeleportScroll => "Teleport",
            Item::HastePotion(_) => "Haste",
            Item::StunBomb => "Stun",
            Item::RiceBall(_) => "Rice",
            Item::MeditationIncense(_) => "Incense",
            Item::AncestralWine(_) => "Wine",
            Item::SmokeScreen(_) => "Smoke",
            Item::FireCracker(_) => "Cracker",
            Item::IronSkinElixir(_) => "IronSkin",
            Item::ClarityTea => "Clarity",
            Item::GoldIngot(_) => "Gold",
            Item::ThunderTalisman(_) => "Thunder",
            Item::JadeSalve(_) => "Salve",
            Item::SerpentFang => "Fang",
            Item::WardingCharm(_) => "Ward",
            Item::InkBomb => "InkBomb",
            Item::PhoenixPlume(_) => "Phoenix",
            Item::MirrorShard => "Mirror",
            Item::FrostVial(_) => "Frost",
            Item::ShadowCloak(_) => "Shadow",
            Item::DragonScale(_) => "Scale",
            Item::BambooFlute(_) => "Flute",
            Item::JadeCompass => "Compass",
            Item::SilkRope => "Rope",
            Item::LotusElixir => "Lotus",
            Item::ThunderDrum(_) => "Drum",
            Item::CinnabarInk => "Cinnabar",
            Item::AncestorToken(_) => "Ancestor",
            Item::WindFan => "Fan",
        }
    }

    pub fn display_name(&self, identified: bool, appearance: &'static str) -> String {
        if identified {
            self.name().to_string()
        } else {
            format!("? {}", appearance)
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Item::HealthPotion(_) => "Restores HP instantly when used. Drink during exploration or combat to heal wounds.",
            Item::PoisonFlask(_, _) => "Throw at an adjacent enemy to inflict poison damage over several turns.",
            Item::RevealScroll => "Reveals the entire floor map, showing all rooms, corridors, and hidden areas.",
            Item::TeleportScroll => "Instantly teleport to a random explored tile. Useful for escaping danger.",
            Item::HastePotion(_) => "Grants Haste, letting you take extra actions each turn for a short duration.",
            Item::StunBomb => "Stuns all visible enemies for several turns, preventing them from acting.",
            Item::RiceBall(_) => "Restores spirit energy. Eat to stave off spiritual exhaustion.",
            Item::MeditationIncense(_) => "Burns fragrant incense that shields your spirit from decay for several moves.",
            Item::AncestralWine(_) => "Potent rice wine that fully restores spirit but leaves you confused and disoriented.",
            Item::SmokeScreen(_) => "Releases a burst of smoke granting you swift movement for several turns.",
            Item::FireCracker(_) => "Explodes with a thunderous crack, dealing damage to all visible enemies.",
            Item::IronSkinElixir(_) => "Hardens your skin like iron, granting a protective shield and slow regeneration.",
            Item::ClarityTea => "A calming brew that purges all negative effects from your body and mind.",
            Item::GoldIngot(_) => "A gleaming bar of pure gold. Can be sold or used for instant wealth.",
            Item::ThunderTalisman(_) => "Channels a bolt of lightning at the nearest enemy, dealing heavy damage.",
            Item::JadeSalve(_) => "A soothing jade ointment that slowly regenerates your wounds over time.",
            Item::SerpentFang => "Coats your weapon in deadly venom, envenoming your strikes for several turns.",
            Item::WardingCharm(_) => "Erects a protective ward that shields both body and spirit from harm.",
            Item::InkBomb => "Splatters blinding ink on all visible enemies, stunning them in place.",
            Item::PhoenixPlume(_) => "A mystical feather that burns on death, reviving you from the brink of destruction.",
            Item::MirrorShard => "A shard of enchanted mirror that reflects the next attack back at the attacker.",
            Item::FrostVial(_) => "Shatters on impact, freezing all adjacent enemies solid for a short time.",
            Item::ShadowCloak(_) => "Wraps you in shadow, making you invisible to enemies for several turns.",
            Item::DragonScale(_) => "A hardened dragon scale that grants bonus armor for the rest of combat.",
            Item::BambooFlute(_) => "Plays a haunting melody that confuses all enemies for several turns.",
            Item::JadeCompass => "An ancient compass that reveals all traps and hidden tiles on the floor.",
            Item::SilkRope => "A magically weighted rope that pulls a distant enemy to an adjacent tile.",
            Item::LotusElixir => "A pure lotus extract that cures all negative status effects instantly.",
            Item::ThunderDrum(_) => "A war drum that sends shockwaves dealing damage to all enemies and slowing them.",
            Item::CinnabarInk => "Mystical red ink that empowers your next spell to deal double damage.",
            Item::AncestorToken(_) => "A sacred ancestral token that revives you with HP upon death. Consumed on use.",
            Item::WindFan => "A powerful fan that blasts all adjacent enemies away with a gust of wind.",
        }
    }
}

pub const EQUIPMENT_POOL: &[Equipment] = &[
    Equipment {
        name: "Brush of Clarity",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::BonusDamage(1),
    },
    Equipment {
        name: "Scholar's Quill",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::BonusDamage(2),
    },
    Equipment {
        name: "Dragon Fang Pen",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::BonusDamage(3),
    },
    Equipment {
        name: "Jade Vest",
        slot: EquipSlot::Armor,
        effect: EquipEffect::DamageReduction(1),
    },
    Equipment {
        name: "Iron Silk Robe",
        slot: EquipSlot::Armor,
        effect: EquipEffect::DamageReduction(2),
    },
    Equipment {
        name: "Phoenix Mantle",
        slot: EquipSlot::Armor,
        effect: EquipEffect::DamageReduction(3),
    },
    Equipment {
        name: "Radical Magnet",
        slot: EquipSlot::Charm,
        effect: EquipEffect::ExtraRadicalDrop(50),
    },
    Equipment {
        name: "Life Jade",
        slot: EquipSlot::Charm,
        effect: EquipEffect::HealOnKill(2),
    },
    Equipment {
        name: "Gold Toad",
        slot: EquipSlot::Charm,
        effect: EquipEffect::GoldBonus(10),
    },
    Equipment {
        name: "Phoenix Feather",
        slot: EquipSlot::Charm,
        effect: EquipEffect::HealOnKill(3),
    },
    Equipment {
        name: "Iron Pickaxe",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::Digging,
    },
    Equipment {
        name: "Spirit Talisman",
        slot: EquipSlot::Charm,
        effect: EquipEffect::SpiritSustain,
    },
    Equipment {
        name: "Jade Bracelet",
        slot: EquipSlot::Charm,
        effect: EquipEffect::SpellPowerBoost(1),
    },
    Equipment {
        name: "Dragon Fang Sword",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::LifeSteal(1),
    },
    Equipment {
        name: "Silk Armor",
        slot: EquipSlot::Armor,
        effect: EquipEffect::DodgeChance(15),
    },
    Equipment {
        name: "Inkstone Pendant",
        slot: EquipSlot::Charm,
        effect: EquipEffect::FocusRegen(1),
    },
    Equipment {
        name: "Iron Gauntlets",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::KnockbackStrike,
    },
    Equipment {
        name: "Bamboo Shield",
        slot: EquipSlot::Armor,
        effect: EquipEffect::ThornsAura(1),
    },
    Equipment {
        name: "Oracle Bone",
        slot: EquipSlot::Charm,
        effect: EquipEffect::EnemyIntentReveal,
    },
    Equipment {
        name: "Tiger Claw",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::CriticalStrike(20),
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Deity {
    Jade,   // Life
    Gale,   // Travel
    Mirror, // Knowledge
    Iron,   // War
    Gold,   // Wealth
}

impl Deity {
    pub fn name(&self) -> &'static str {
        match self {
            Deity::Jade => "Jade Emperor (Life)",
            Deity::Gale => "Wind Walker (Travel)",
            Deity::Mirror => "Mirror Sage (Knowledge)",
            Deity::Iron => "Iron General (War)",
            Deity::Gold => "Golden Toad (Wealth)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerForm {
    Human,
    Flame, // Immune to fire, burn on touch
    #[allow(dead_code)]
    Stone, // High Def, slow
    Mist,  // High Evasion, weak atk
    Tiger, // High Atk, fast
}

impl PlayerForm {
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            PlayerForm::Human => "Human",
            PlayerForm::Flame => "Flame Avatar",
            PlayerForm::Stone => "Stone Golem",
            PlayerForm::Mist => "Mist Spirit",
            PlayerForm::Tiger => "Tiger Demon",
        }
    }

    pub fn glyph(&self) -> &'static str {
        match self {
            PlayerForm::Human => "@",
            PlayerForm::Flame => "火",
            PlayerForm::Stone => "石",
            PlayerForm::Mist => "气",
            PlayerForm::Tiger => "虎",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            PlayerForm::Human => "#ffffff",
            PlayerForm::Flame => "#ff5500",
            PlayerForm::Stone => "#888888",
            PlayerForm::Mist => "#aaddff",
            PlayerForm::Tiger => "#ffaa00",
        }
    }
}

/// Player class specialization chosen at game start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClassData {
    pub name_en: &'static str,
    pub name_cn: &'static str,
    pub lore: &'static str,
    pub color: &'static str,
    pub icon: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerClass {
    Scholar,
    Warrior,
    Alchemist,
    Monk,
    Thief,
    Calligrapher,
    Exorcist,
    Herbalist,
    Diviner,
    Swordsman,
    Merchant,
    Pilgrim,
    Beastmaster,
    Scribe,
    Assassin,
    Earthmover,
    Shaman,
    Wanderer,
    Ironclad,
    Inkmaster,
}

impl PlayerClass {
    pub fn all() -> Vec<PlayerClass> {
        vec![
            PlayerClass::Scholar,
            PlayerClass::Warrior,
            PlayerClass::Alchemist,
            PlayerClass::Monk,
            PlayerClass::Thief,
            PlayerClass::Calligrapher,
            PlayerClass::Exorcist,
            PlayerClass::Herbalist,
            PlayerClass::Diviner,
            PlayerClass::Swordsman,
            PlayerClass::Merchant,
            PlayerClass::Pilgrim,
            PlayerClass::Beastmaster,
            PlayerClass::Scribe,
            PlayerClass::Assassin,
            PlayerClass::Earthmover,
            PlayerClass::Shaman,
            PlayerClass::Wanderer,
            PlayerClass::Ironclad,
            PlayerClass::Inkmaster,
        ]
    }

    pub fn data(&self) -> ClassData {
        match self {
            PlayerClass::Scholar => ClassData {
                name_en: "Scholar",
                name_cn: "学者",
                lore: "Balanced, hints in combat.",
                color: "#88ccff",
                icon: "S",
            },
            PlayerClass::Warrior => ClassData {
                name_en: "Warrior",
                name_cn: "武士",
                lore: "+3 HP, +1 dmg.",
                color: "#ff8888",
                icon: "W",
            },
            PlayerClass::Alchemist => ClassData {
                name_en: "Alchemist",
                name_cn: "炼丹师",
                lore: "2x potion healing, +2 slots.",
                color: "#88ff88",
                icon: "A",
            },
            PlayerClass::Monk => ClassData {
                name_en: "Monk",
                name_cn: "僧侣",
                lore: "Regen 1HP/floor.",
                color: "#ffcc88",
                icon: "M",
            },
            PlayerClass::Thief => ClassData {
                name_en: "Thief",
                name_cn: "盗贼",
                lore: "Start with gold, extra gold drops.",
                color: "#cc88ff",
                icon: "T",
            },
            PlayerClass::Calligrapher => ClassData {
                name_en: "Calligrapher",
                name_cn: "书法家",
                lore: "Bonus spell power.",
                color: "#ffffff",
                icon: "C",
            },
            PlayerClass::Exorcist => ClassData {
                name_en: "Exorcist",
                name_cn: "驱魔师",
                lore: "Bonus vs bosses.",
                color: "#ff88cc",
                icon: "E",
            },
            PlayerClass::Herbalist => ClassData {
                name_en: "Herbalist",
                name_cn: "草药师",
                lore: "Start w/ 2 health pots.",
                color: "#aaffaa",
                icon: "H",
            },
            PlayerClass::Diviner => ClassData {
                name_en: "Diviner",
                name_cn: "占卜师",
                lore: "Map partially revealed.",
                color: "#ddaadd",
                icon: "D",
            },
            PlayerClass::Swordsman => ClassData {
                name_en: "Swordsman",
                name_cn: "剑客",
                lore: "Start w/ weapon, crit chance.",
                color: "#ffaaaa",
                icon: "S",
            },
            PlayerClass::Merchant => ClassData {
                name_en: "Merchant",
                name_cn: "商人",
                lore: "Shop discount, extra gold.",
                color: "#ffffaa",
                icon: "M",
            },
            PlayerClass::Pilgrim => ClassData {
                name_en: "Pilgrim",
                name_cn: "朝圣者",
                lore: "Start w/ piety.",
                color: "#ccccaa",
                icon: "P",
            },
            PlayerClass::Beastmaster => ClassData {
                name_en: "Beastmaster",
                name_cn: "驯兽师",
                lore: "Start w/ companion.",
                color: "#ccaa88",
                icon: "B",
            },
            PlayerClass::Scribe => ClassData {
                name_en: "Scribe",
                name_cn: "抄写员",
                lore: "Bonus radical drops.",
                color: "#eeeeee",
                icon: "S",
            },
            PlayerClass::Assassin => ClassData {
                name_en: "Assassin",
                name_cn: "刺客",
                lore: "High dmg, low HP.",
                color: "#aa4444",
                icon: "A",
            },
            PlayerClass::Earthmover => ClassData {
                name_en: "Earthmover",
                name_cn: "土行者",
                lore: "Start w/ pickaxe.",
                color: "#aa8866",
                icon: "E",
            },
            PlayerClass::Shaman => ClassData {
                name_en: "Shaman",
                name_cn: "巫",
                lore: "Start w/ 2 random spells.",
                color: "#88ccaa",
                icon: "S",
            },
            PlayerClass::Wanderer => ClassData {
                name_en: "Wanderer",
                name_cn: "浪人",
                lore: "Random bonus each floor.",
                color: "#aaaaaa",
                icon: "W",
            },
            PlayerClass::Ironclad => ClassData {
                name_en: "Ironclad",
                name_cn: "铁甲",
                lore: "Start w/ armor, high HP.",
                color: "#8888aa",
                icon: "I",
            },
            PlayerClass::Inkmaster => ClassData {
                name_en: "Inkmaster",
                name_cn: "墨师",
                lore: "Start w/ extra radicals.",
                color: "#444444",
                icon: "I",
            },
        }
    }
}

/// Whether an item or piece of equipment is normal, cursed, or blessed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemState {
    Normal,
    Cursed,
    Blessed,
}

#[derive(Clone)]
pub struct Player {
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    /// Player class
    pub class: PlayerClass,
    /// Collected radicals (stored as their &str character)
    pub radicals: Vec<&'static str>,
    /// Forged spells ready to use in combat
    pub spells: Vec<Spell>,
    /// Index of currently selected spell (for combat use)
    pub selected_spell: usize,
    /// Shield active (blocks next hit)
    pub shield: bool,
    /// Active status effects
    pub statuses: Vec<StatusInstance>,
    /// Consumable items (max MAX_ITEMS)
    pub items: Vec<Item>,
    /// State of each consumable item (parallel to `items`)
    pub item_states: Vec<ItemState>,
    /// Equipped items (up to 3: weapon, armor, charm)
    pub weapon: Option<&'static Equipment>,
    pub armor: Option<&'static Equipment>,
    pub charm: Option<&'static Equipment>,
    /// State of equipped weapon/armor/charm
    pub weapon_state: ItemState,
    pub armor_state: ItemState,
    pub charm_state: ItemState,
    /// Enchantments on equipment slots: [weapon, armor, charm]
    pub enchantments: [Option<&'static str>; 3],
    /// Bonus damage from tone shrine (used once, then reset)
    pub tone_bonus_damage: i32,
    /// Bonus defense from tone defense wall (used once, then reset)
    pub defense_bonus: i32,
    /// Bonus spell power from compound builder (used once, then reset)
    pub spell_power_temp_bonus: i32,
    /// Permanent shop discount from meta progression (percentage)
    pub shop_discount_pct: i32,
    /// Permanent spell potency bonus from meta progression
    pub spell_power_bonus: i32,
    /// Active god favor (piety)
    pub piety: Vec<(Deity, i32)>,
    /// Current physical form
    pub form: PlayerForm,
    /// Turns remaining in current form (0 = permanent/human)
    pub form_timer: i32,
    /// Spirit energy — ticks down each move, player takes damage at 0
    pub spirit: i32,
    pub max_spirit: i32,
}

impl Player {
    pub fn new(x: i32, y: i32, class: PlayerClass) -> Self {
        let (hp, max_hp) = match class {
            PlayerClass::Warrior => (13, 13),
            PlayerClass::Ironclad => (14, 14),
            PlayerClass::Exorcist | PlayerClass::Swordsman | PlayerClass::Earthmover => (11, 11),
            PlayerClass::Scholar
            | PlayerClass::Alchemist
            | PlayerClass::Pilgrim
            | PlayerClass::Wanderer => (10, 10),
            PlayerClass::Thief
            | PlayerClass::Calligrapher
            | PlayerClass::Herbalist
            | PlayerClass::Merchant
            | PlayerClass::Beastmaster
            | PlayerClass::Scribe
            | PlayerClass::Inkmaster => (9, 9),
            PlayerClass::Monk | PlayerClass::Diviner | PlayerClass::Shaman => (8, 8),
            PlayerClass::Assassin => (7, 7),
        };
        Self {
            x,
            y,
            hp,
            max_hp,
            gold: 0,
            class,
            radicals: Vec::new(),
            spells: Vec::new(),
            selected_spell: 0,
            shield: false,
            statuses: Vec::new(),
            items: Vec::new(),
            item_states: Vec::new(),
            weapon: None,
            armor: None,
            charm: None,
            weapon_state: ItemState::Normal,
            armor_state: ItemState::Normal,
            charm_state: ItemState::Normal,
            enchantments: [None; 3],
            tone_bonus_damage: 0,
            defense_bonus: 0,
            spell_power_temp_bonus: 0,
            shop_discount_pct: 0,
            spell_power_bonus: 0,
            piety: Vec::new(),
            form: PlayerForm::Human,
            form_timer: 0,
            spirit: Self::base_max_spirit(class),
            max_spirit: Self::base_max_spirit(class),
        }
    }

    fn base_max_spirit(class: PlayerClass) -> i32 {
        match class {
            PlayerClass::Monk | PlayerClass::Pilgrim => 180,
            PlayerClass::Herbalist | PlayerClass::Calligrapher | PlayerClass::Inkmaster => 170,
            PlayerClass::Scholar => 160,
            _ => 150,
        }
    }

    pub fn has_spirit_sustain(&self) -> bool {
        self.charm
            .map(|c| matches!(c.effect, EquipEffect::SpiritSustain))
            .unwrap_or(false)
    }

    pub fn get_piety(&self, deity: Deity) -> i32 {
        self.piety
            .iter()
            .find(|(d, _)| *d == deity)
            .map(|(_, p)| *p)
            .unwrap_or(0)
    }

    pub fn add_piety(&mut self, deity: Deity, amount: i32) {
        if let Some((_, p)) = self.piety.iter_mut().find(|(d, _)| *d == deity) {
            *p += amount;
        } else {
            self.piety.push((deity, amount));
        }
    }

    pub fn highest_deity(&self) -> Option<Deity> {
        self.piety
            .iter()
            .filter(|&&(_, p)| p > 0)
            .max_by_key(|&&(_, p)| p)
            .map(|&(d, _)| d)
    }

    pub fn devotion_bonus(&self, deity: Deity) -> &'static str {
        let p = self.get_piety(deity);
        if p >= 15 {
            match deity {
                Deity::Jade => "Major: +1 HP on kill",
                Deity::Iron => "Major: +1 bonus damage",
                Deity::Gold => "Major: +3 bonus gold on kill",
                Deity::Gale => "Major: 15% evade on wrong answer",
                Deity::Mirror => "Major: Show pinyin on wrong answer",
            }
        } else if p >= 10 {
            match deity {
                Deity::Jade => "Moderate: +1 HP on kill",
                Deity::Iron => "Moderate: +1 bonus damage",
                Deity::Gold => "Moderate: +3 bonus gold on kill",
                Deity::Gale => "Moderate: 15% evade on wrong answer",
                Deity::Mirror => "Moderate: Show pinyin on wrong answer",
            }
        } else if p >= 5 {
            "Minor devotion"
        } else {
            "None"
        }
    }

    pub fn deity_synergy(&self) -> Option<(&'static str, &'static str)> {
        let p = |d| self.get_piety(d) >= 10;
        if p(Deity::Jade) && p(Deity::Iron) {
            Some(("Paladin's Vigor", "Heal 1 HP per kill AND +1 damage"))
        } else if p(Deity::Jade) && p(Deity::Gold) {
            Some(("Merchant's Blessing", "+5 gold per floor cleared"))
        } else if p(Deity::Mirror) && p(Deity::Gale) {
            Some(("Scholar's Wind", "Reveal map on floor entry (25% chance)"))
        } else if p(Deity::Iron) && p(Deity::Gold) {
            Some(("Warlord's Tithe", "Enemies drop double gold"))
        } else if p(Deity::Mirror) && p(Deity::Iron) {
            Some(("Tactical Insight", "+2 bonus damage to elites"))
        } else if p(Deity::Gale) && p(Deity::Gold) {
            Some(("Fortune's Breeze", "25% chance for extra item on floor"))
        } else {
            None
        }
    }

    pub fn set_form(&mut self, form: PlayerForm, duration: i32) {
        self.form = form;
        self.form_timer = duration;
    }

    pub fn tick_form(&mut self) {
        if self.form_timer > 0 {
            self.form_timer -= 1;
            if self.form_timer == 0 {
                self.form = PlayerForm::Human;
            }
        }
    }

    #[allow(dead_code)]
    pub fn apply_meta_progression(
        &mut self,
        starting_hp_bonus: i32,
        shop_discount_pct: i32,
        spell_power_bonus: i32,
    ) {
        if starting_hp_bonus > 0 {
            self.max_hp += starting_hp_bonus;
            self.hp += starting_hp_bonus;
        }
        self.shop_discount_pct = shop_discount_pct.max(0);
        self.spell_power_bonus = spell_power_bonus.max(0);
    }

    /// Max items depends on class
    pub fn max_items(&self) -> usize {
        match self.class {
            PlayerClass::Alchemist => 7,
            PlayerClass::Thief | PlayerClass::Herbalist => 6,
            PlayerClass::Scholar
            | PlayerClass::Calligrapher
            | PlayerClass::Diviner
            | PlayerClass::Merchant
            | PlayerClass::Scribe
            | PlayerClass::Shaman
            | PlayerClass::Wanderer
            | PlayerClass::Inkmaster => 5,
            PlayerClass::Warrior
            | PlayerClass::Monk
            | PlayerClass::Exorcist
            | PlayerClass::Swordsman
            | PlayerClass::Pilgrim
            | PlayerClass::Beastmaster
            | PlayerClass::Assassin => 4,
            PlayerClass::Earthmover | PlayerClass::Ironclad => 3,
        }
    }

    /// Attempt to move by (dx, dy). Returns the target position.
    pub fn intended_move(&self, dx: i32, dy: i32) -> (i32, i32) {
        (self.x + dx, self.y + dy)
    }

    pub fn move_to(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn add_item(&mut self, item: Item, state: ItemState) -> bool {
        if self.items.len() < self.max_items() {
            self.items.push(item);
            self.item_states.push(state);
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn take_item(&mut self, idx: usize) -> Option<(Item, ItemState)> {
        if idx < self.items.len() {
            let state = self.item_states.remove(idx);
            Some((self.items.remove(idx), state))
        } else {
            None
        }
    }

    pub fn add_radical(&mut self, ch: &'static str) {
        self.radicals.push(ch);
    }

    pub fn add_spell(&mut self, spell: Spell) {
        self.spells.push(spell);
    }

    pub fn cycle_spell(&mut self) -> bool {
        if self.spells.is_empty() {
            return false;
        }
        self.selected_spell = (self.selected_spell + 1) % self.spells.len();
        true
    }

    pub fn use_spell(&mut self) -> Option<Spell> {
        if self.spells.is_empty() {
            return None;
        }
        let idx = self.selected_spell.min(self.spells.len() - 1);
        let spell = self.spells.remove(idx);
        if self.selected_spell >= self.spells.len() && !self.spells.is_empty() {
            self.selected_spell = 0;
        }
        Some(spell)
    }

    /// Get bonus attack damage from equipment
    pub fn bonus_damage(&self) -> i32 {
        match self.weapon {
            Some(eq) => match eq.effect {
                EquipEffect::BonusDamage(d) => d,
                _ => 0,
            },
            None => 0,
        }
    }

    /// Get damage reduction from armor
    pub fn damage_reduction(&self) -> i32 {
        match self.armor {
            Some(eq) => match eq.effect {
                EquipEffect::DamageReduction(d) => d,
                _ => 0,
            },
            None => 0,
        }
    }

    /// Check extra radical drop chance (percentage)
    pub fn extra_radical_chance(&self) -> i32 {
        match self.charm {
            Some(eq) => match eq.effect {
                EquipEffect::ExtraRadicalDrop(pct) => pct,
                _ => 0,
            },
            None => 0,
        }
    }

    /// Get heal-on-kill amount
    pub fn heal_on_kill(&self) -> i32 {
        match self.charm {
            Some(eq) => match eq.effect {
                EquipEffect::HealOnKill(amt) => amt,
                _ => 0,
            },
            None => 0,
        }
    }

    /// Get gold bonus
    pub fn gold_bonus(&self) -> i32 {
        match self.charm {
            Some(eq) => match eq.effect {
                EquipEffect::GoldBonus(amt) => amt,
                _ => 0,
            },
            None => 0,
        }
    }

    pub fn equip(&mut self, equipment: &'static Equipment, state: ItemState) {
        match equipment.slot {
            EquipSlot::Weapon => {
                self.weapon = Some(equipment);
                self.weapon_state = state;
            }
            EquipSlot::Armor => {
                self.armor = Some(equipment);
                self.armor_state = state;
            }
            EquipSlot::Charm => {
                self.charm = Some(equipment);
                self.charm_state = state;
            }
        }
    }

    pub fn equipment_state(&self, slot: EquipSlot) -> ItemState {
        match slot {
            EquipSlot::Weapon => self.weapon_state,
            EquipSlot::Armor => self.armor_state,
            EquipSlot::Charm => self.charm_state,
        }
    }

    /// Bonus damage from enchantments (力=+1, 火=+1)
    pub fn enchant_bonus_damage(&self) -> i32 {
        self.enchantments
            .iter()
            .filter_map(|e| *e)
            .map(|r| match r {
                "力" | "火" => 1,
                _ => 0,
            })
            .sum()
    }

    /// Bonus damage reduction from enchantments (水=+1, 土=+1)
    pub fn enchant_damage_reduction(&self) -> i32 {
        self.enchantments
            .iter()
            .filter_map(|e| *e)
            .map(|r| match r {
                "水" | "土" => 1,
                _ => 0,
            })
            .sum()
    }

    /// Bonus max HP from enchantments (心=+2)
    #[allow(dead_code)]
    pub fn enchant_max_hp_bonus(&self) -> i32 {
        self.enchantments
            .iter()
            .filter_map(|e| *e)
            .map(|r| match r {
                "心" => 2,
                _ => 0,
            })
            .sum()
    }

    /// Bonus gold from enchantments (金=+3)
    pub fn enchant_gold_bonus(&self) -> i32 {
        self.enchantments
            .iter()
            .filter_map(|e| *e)
            .map(|r| match r {
                "金" => 3,
                _ => 0,
            })
            .sum()
    }

    /// Bonus FOV from enchantments (目=+1)
    pub fn enchant_fov_bonus(&self) -> i32 {
        self.enchantments
            .iter()
            .filter_map(|e| *e)
            .map(|r| match r {
                "目" => 1,
                _ => 0,
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::{Deity, Item, ItemKind, Player, PlayerClass};

    #[test]
    fn item_kind_matches_variant() {
        assert_eq!(Item::HealthPotion(5).kind(), ItemKind::HealthPotion);
        assert_eq!(Item::TeleportScroll.kind(), ItemKind::TeleportScroll);
    }

    #[test]
    fn item_display_name_uses_mystery_label_until_identified() {
        let item = Item::RevealScroll;

        assert_eq!(
            item.display_name(false, "Cloud Seal 云符"),
            "? Cloud Seal 云符"
        );
        assert_eq!(
            item.display_name(true, "Cloud Seal 云符"),
            "👁 Reveal Scroll"
        );
    }

    #[test]
    fn deity_synergy_requires_dual_devotion() {
        let mut player = Player::new(0, 0, PlayerClass::Warrior);
        player.add_piety(Deity::Jade, 10);
        player.add_piety(Deity::Iron, 10);
        assert_eq!(
            player.deity_synergy(),
            Some(("Paladin's Vigor", "Heal 1 HP per kill AND +1 damage"))
        );
    }

    #[test]
    fn deity_synergy_returns_none_without_threshold() {
        let mut player = Player::new(0, 0, PlayerClass::Warrior);
        player.add_piety(Deity::Jade, 9);
        player.add_piety(Deity::Iron, 10);
        assert_eq!(player.deity_synergy(), None);
    }

    #[test]
    fn devotion_bonus_tiers() {
        let mut player = Player::new(0, 0, PlayerClass::Warrior);
        assert_eq!(player.devotion_bonus(Deity::Jade), "None");

        player.add_piety(Deity::Jade, 5);
        assert_eq!(player.devotion_bonus(Deity::Jade), "Minor devotion");

        player.add_piety(Deity::Jade, 5);
        assert_eq!(
            player.devotion_bonus(Deity::Jade),
            "Moderate: +1 HP on kill"
        );

        player.add_piety(Deity::Jade, 5);
        assert_eq!(player.devotion_bonus(Deity::Jade), "Major: +1 HP on kill");
    }
}
