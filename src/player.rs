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
                "Allows you to breach through bulkheads by walking into them.".to_string()
            }
            EquipEffect::SpiritSustain => {
                "Halves energy drain rate — lose 1 energy every 2 moves instead of every move."
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
            EquipSlot::Charm => "Module",
        };
        format!("[{}] {}", slot, self.effect.description())
    }
}

#[allow(dead_code)]
pub const MAX_ITEMS: usize = 5;
pub const ITEM_KIND_COUNT: usize = 32;
pub const MYSTERY_ITEM_APPEARANCES: [&str; ITEM_KIND_COUNT] = [
    "Red Canister ⬡",
    "Blue Vial ◆",
    "Green Capsule ◇",
    "Black Module ■",
    "Silver Disc ○",
    "Orange Tube ◈",
    "Gold Cylinder ◉",
    "White Cartridge △",
    "Purple Injector ▽",
    "Cyan Cell ☆",
    "Crimson Syringe ◎",
    "Bronze Container ▣",
    "Teal Ampule ⬢",
    "Amber Flask ▲",
    "Neon Charge ★",
    "Gray Pellet ▥",
    "Indigo Pod ◐",
    "Chrome Pack ◑",
    "Cobalt Vial ⬟",
    "Violet Sphere ⬠",
    "Frost Canister ❄",
    "Shadow Module ◩",
    "Pearl Cell ◬",
    "Emerald Capsule ◭",
    "Spark Cartridge ⚡",
    "Silk Tube ◮",
    "Orchid Injector ✦",
    "Steel Pod ✧",
    "Plasma Charge ⚛",
    "Coral Syringe ✶",
    "Quantum Cell ✸",
    "Nova Canister ✹",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemKind {
    MedHypo,
    ToxinGrenade,
    ScannerPulse,
    PersonalTeleporter,
    StimPack,
    EMPGrenade,
    RationPack,
    FocusStim,
    SynthAle,
    HoloDecoy,
    PlasmaBurst,
    NanoShield,
    NeuralBoost,
    CreditChip,
    ShockModule,
    BiogelPatch,
    VenomDart,
    DeflectorDrone,
    NaniteSwarm,
    Revitalizer,
    ReflectorPlate,
    CryoGrenade,
    CloakingDevice,
    PlasmaShield,
    SignalJammer,
    NavComputer,
    GrappleLine,
    OmniGel,
    SonicEmitter,
    CircuitInk,
    DataCore,
    ThrusterPack,
}

/// A recipe for combining two consumable items into a new one.
pub struct CraftingRecipe {
    pub input1: ItemKind,
    pub input2: ItemKind,
    pub output: ItemKind,
    pub output_name: &'static str,
}

pub const CRAFTING_RECIPES: &[CraftingRecipe] = &[
    CraftingRecipe { input1: ItemKind::MedHypo, input2: ItemKind::ToxinGrenade, output: ItemKind::OmniGel, output_name: "🧬 Omni-Gel" },
    CraftingRecipe { input1: ItemKind::PlasmaBurst, input2: ItemKind::NaniteSwarm, output: ItemKind::SonicEmitter, output_name: "📡 Sonic Emitter" },
    CraftingRecipe { input1: ItemKind::CryoGrenade, input2: ItemKind::SonicEmitter, output: ItemKind::EMPGrenade, output_name: "⚡ EMP Grenade" },
    CraftingRecipe { input1: ItemKind::MedHypo, input2: ItemKind::MedHypo, output: ItemKind::SynthAle, output_name: "🍺 Synth-Ale" },
    CraftingRecipe { input1: ItemKind::HoloDecoy, input2: ItemKind::ToxinGrenade, output: ItemKind::CloakingDevice, output_name: "👻 Cloaking Device" },
    CraftingRecipe { input1: ItemKind::NanoShield, input2: ItemKind::PlasmaShield, output: ItemKind::DeflectorDrone, output_name: "🛸 Deflector Drone" },
    CraftingRecipe { input1: ItemKind::FocusStim, input2: ItemKind::NeuralBoost, output: ItemKind::OmniGel, output_name: "🧬 Omni-Gel" },
    CraftingRecipe { input1: ItemKind::VenomDart, input2: ItemKind::ToxinGrenade, output: ItemKind::VenomDart, output_name: "☠ Venom Dart+" },
    CraftingRecipe { input1: ItemKind::PlasmaBurst, input2: ItemKind::PlasmaBurst, output: ItemKind::PlasmaBurst, output_name: "💥 Plasma Burst+" },
    CraftingRecipe { input1: ItemKind::BiogelPatch, input2: ItemKind::MedHypo, output: ItemKind::BiogelPatch, output_name: "💊 Biogel Patch+" },
    CraftingRecipe { input1: ItemKind::ThrusterPack, input2: ItemKind::GrappleLine, output: ItemKind::SignalJammer, output_name: "📶 Signal Jammer" },
    CraftingRecipe { input1: ItemKind::ReflectorPlate, input2: ItemKind::DeflectorDrone, output: ItemKind::ReflectorPlate, output_name: "🪞 Reflector Plate+" },
    CraftingRecipe { input1: ItemKind::ShockModule, input2: ItemKind::CryoGrenade, output: ItemKind::SonicEmitter, output_name: "📡 Sonic Emitter" },
    CraftingRecipe { input1: ItemKind::CreditChip, input2: ItemKind::CreditChip, output: ItemKind::Revitalizer, output_name: "💉 Revitalizer" },
    CraftingRecipe { input1: ItemKind::StimPack, input2: ItemKind::NeuralBoost, output: ItemKind::StimPack, output_name: "⚡ Stim-Pack+" },
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
        ItemKind::OmniGel => Item::OmniGel,
        ItemKind::SonicEmitter => {
            let base = match (item1, item2) {
                (Item::PlasmaBurst(d), _) | (_, Item::PlasmaBurst(d)) => *d,
                (Item::ShockModule(d), _) | (_, Item::ShockModule(d)) => *d,
                (Item::SonicEmitter(d), _) | (_, Item::SonicEmitter(d)) => *d,
                _ => 4,
            };
            Item::SonicEmitter(base + 3)
        }
        ItemKind::EMPGrenade => Item::EMPGrenade,
        ItemKind::SynthAle => Item::SynthAle(6),
        ItemKind::CloakingDevice => Item::CloakingDevice(5),
        ItemKind::DeflectorDrone => {
            let base = match (item1, item2) {
                (Item::DeflectorDrone(d), _) | (_, Item::DeflectorDrone(d)) => *d,
                (Item::NanoShield(d), _) | (_, Item::NanoShield(d)) => *d,
                _ => 5,
            };
            Item::DeflectorDrone(base + 3)
        }
        ItemKind::VenomDart => Item::VenomDart,
        ItemKind::PlasmaBurst => {
            let base = match (item1, item2) {
                (Item::PlasmaBurst(d1), Item::PlasmaBurst(d2)) => (*d1).max(*d2),
                (Item::PlasmaBurst(d), _) | (_, Item::PlasmaBurst(d)) => *d,
                _ => 5,
            };
            Item::PlasmaBurst(base + 4)
        }
        ItemKind::BiogelPatch => {
            let base = match (item1, item2) {
                (Item::BiogelPatch(d), _) | (_, Item::BiogelPatch(d)) => *d,
                _ => 2,
            };
            Item::BiogelPatch(base + 2)
        }
        ItemKind::SignalJammer => Item::SignalJammer(4),
        ItemKind::ReflectorPlate => Item::ReflectorPlate,
        ItemKind::Revitalizer => Item::Revitalizer(15),
        ItemKind::StimPack => {
            let base = match (item1, item2) {
                (Item::StimPack(d), _) | (_, Item::StimPack(d)) => *d,
                _ => 5,
            };
            Item::StimPack(base + 3)
        }
        // Fallback for any future recipes
        _ => Item::MedHypo(10),
    }
}

impl ItemKind {
    pub fn index(self) -> usize {
        match self {
            ItemKind::MedHypo => 0,
            ItemKind::ToxinGrenade => 1,
            ItemKind::ScannerPulse => 2,
            ItemKind::PersonalTeleporter => 3,
            ItemKind::StimPack => 4,
            ItemKind::EMPGrenade => 5,
            ItemKind::RationPack => 6,
            ItemKind::FocusStim => 7,
            ItemKind::SynthAle => 8,
            ItemKind::HoloDecoy => 9,
            ItemKind::PlasmaBurst => 10,
            ItemKind::NanoShield => 11,
            ItemKind::NeuralBoost => 12,
            ItemKind::CreditChip => 13,
            ItemKind::ShockModule => 14,
            ItemKind::BiogelPatch => 15,
            ItemKind::VenomDart => 16,
            ItemKind::DeflectorDrone => 17,
            ItemKind::NaniteSwarm => 18,
            ItemKind::Revitalizer => 19,
            ItemKind::ReflectorPlate => 20,
            ItemKind::CryoGrenade => 21,
            ItemKind::CloakingDevice => 22,
            ItemKind::PlasmaShield => 23,
            ItemKind::SignalJammer => 24,
            ItemKind::NavComputer => 25,
            ItemKind::GrappleLine => 26,
            ItemKind::OmniGel => 27,
            ItemKind::SonicEmitter => 28,
            ItemKind::CircuitInk => 29,
            ItemKind::DataCore => 30,
            ItemKind::ThrusterPack => 31,
        }
    }
}

/// Consumable items the player can carry and use.
#[derive(Clone, Debug)]
pub enum Item {
    /// Heal N HP instantly
    MedHypo(i32),
    /// Apply poison (dmg, turns) to adjacent enemies
    ToxinGrenade(i32, i32),
    /// Reveal entire floor map
    ScannerPulse,
    /// Teleport to random explored walkable tile
    PersonalTeleporter,
    /// Grant haste for N turns
    StimPack(i32),
    /// Stun all visible enemies
    EMPGrenade,
    /// Restore spirit energy
    RationPack(i32),
    /// Block spirit drain for N moves
    FocusStim(i32),
    /// Full spirit restore + Confused for N turns
    SynthAle(i32),
    /// Grant Haste for N turns (smoke cover)
    HoloDecoy(i32),
    /// AoE damage to all visible enemies
    PlasmaBurst(i32),
    /// Grant Shield + Regen for N turns
    NanoShield(i32),
    /// Remove all negative status effects
    NeuralBoost,
    /// Gain N gold instantly
    CreditChip(i32),
    /// High damage to nearest enemy
    ShockModule(i32),
    /// Regen N per turn for 5 turns
    BiogelPatch(i32),
    /// Apply Envenomed to self weapon for 5 turns
    VenomDart,
    /// Grant Shield + SpiritShield for N turns
    DeflectorDrone(i32),
    /// Stun all visible enemies + confuse (like enhanced EMPGrenade)
    NaniteSwarm,
    /// Auto-revive on death, restoring N HP (passive, consumed on death)
    Revitalizer(i32),
    /// Reflect next attack back at attacker (1 use)
    ReflectorPlate,
    /// Freeze all adjacent enemies for N turns
    CryoGrenade(i32),
    /// Become invisible for N turns
    CloakingDevice(i32),
    /// Gain +N armor for the rest of combat
    PlasmaShield(i32),
    /// Confuse all enemies for N turns
    SignalJammer(i32),
    /// Reveal all traps and hidden tiles on the floor
    NavComputer,
    /// Pull a distant enemy to adjacent tile
    GrappleLine,
    /// Cure all negative status effects
    OmniGel,
    /// Deal N damage to all enemies + Slow 1
    SonicEmitter(i32),
    /// Next spell deals double damage
    CircuitInk,
    /// Revive with N HP on death (passive, consumed on use)
    DataCore(i32),
    /// Push all adjacent enemies 2 tiles away
    ThrusterPack,
}

impl Item {
    pub fn kind(&self) -> ItemKind {
        match self {
            Item::MedHypo(_) => ItemKind::MedHypo,
            Item::ToxinGrenade(_, _) => ItemKind::ToxinGrenade,
            Item::ScannerPulse => ItemKind::ScannerPulse,
            Item::PersonalTeleporter => ItemKind::PersonalTeleporter,
            Item::StimPack(_) => ItemKind::StimPack,
            Item::EMPGrenade => ItemKind::EMPGrenade,
            Item::RationPack(_) => ItemKind::RationPack,
            Item::FocusStim(_) => ItemKind::FocusStim,
            Item::SynthAle(_) => ItemKind::SynthAle,
            Item::HoloDecoy(_) => ItemKind::HoloDecoy,
            Item::PlasmaBurst(_) => ItemKind::PlasmaBurst,
            Item::NanoShield(_) => ItemKind::NanoShield,
            Item::NeuralBoost => ItemKind::NeuralBoost,
            Item::CreditChip(_) => ItemKind::CreditChip,
            Item::ShockModule(_) => ItemKind::ShockModule,
            Item::BiogelPatch(_) => ItemKind::BiogelPatch,
            Item::VenomDart => ItemKind::VenomDart,
            Item::DeflectorDrone(_) => ItemKind::DeflectorDrone,
            Item::NaniteSwarm => ItemKind::NaniteSwarm,
            Item::Revitalizer(_) => ItemKind::Revitalizer,
            Item::ReflectorPlate => ItemKind::ReflectorPlate,
            Item::CryoGrenade(_) => ItemKind::CryoGrenade,
            Item::CloakingDevice(_) => ItemKind::CloakingDevice,
            Item::PlasmaShield(_) => ItemKind::PlasmaShield,
            Item::SignalJammer(_) => ItemKind::SignalJammer,
            Item::NavComputer => ItemKind::NavComputer,
            Item::GrappleLine => ItemKind::GrappleLine,
            Item::OmniGel => ItemKind::OmniGel,
            Item::SonicEmitter(_) => ItemKind::SonicEmitter,
            Item::CircuitInk => ItemKind::CircuitInk,
            Item::DataCore(_) => ItemKind::DataCore,
            Item::ThrusterPack => ItemKind::ThrusterPack,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Item::MedHypo(_) => "💊 Med-Hypo",
            Item::ToxinGrenade(_, _) => "☠ Toxin Grenade",
            Item::ScannerPulse => "📡 Scanner Pulse",
            Item::PersonalTeleporter => "✦ Personal Teleporter",
            Item::StimPack(_) => "⚡ Stim-Pack",
            Item::EMPGrenade => "⚡ EMP Grenade",
            Item::RationPack(_) => "🍱 Ration Pack",
            Item::FocusStim(_) => "🧠 Focus Stim",
            Item::SynthAle(_) => "🍺 Synth-Ale",
            Item::HoloDecoy(_) => "👤 Holo-Decoy",
            Item::PlasmaBurst(_) => "💥 Plasma Burst",
            Item::NanoShield(_) => "🛡 Nano-Shield",
            Item::NeuralBoost => "🧠 Neural Boost",
            Item::CreditChip(_) => "💰 Credit Chip",
            Item::ShockModule(_) => "⚡ Shock Module",
            Item::BiogelPatch(_) => "💊 Biogel Patch",
            Item::VenomDart => "☠ Venom Dart",
            Item::DeflectorDrone(_) => "🛸 Deflector Drone",
            Item::NaniteSwarm => "🤖 Nanite Swarm",
            Item::Revitalizer(_) => "💉 Revitalizer",
            Item::ReflectorPlate => "🪞 Reflector Plate",
            Item::CryoGrenade(_) => "❄ Cryo Grenade",
            Item::CloakingDevice(_) => "👻 Cloaking Device",
            Item::PlasmaShield(_) => "🔰 Plasma Shield",
            Item::SignalJammer(_) => "📶 Signal Jammer",
            Item::NavComputer => "🧭 Nav Computer",
            Item::GrappleLine => "🪝 Grapple Line",
            Item::OmniGel => "🧬 Omni-Gel",
            Item::SonicEmitter(_) => "📡 Sonic Emitter",
            Item::CircuitInk => "🔧 Circuit Ink",
            Item::DataCore(_) => "💾 Data Core",
            Item::ThrusterPack => "🚀 Thruster Pack",
        }
    }

    #[allow(dead_code)]
    pub fn short_name(&self) -> &'static str {
        match self {
            Item::MedHypo(_) => "MedHypo",
            Item::ToxinGrenade(_, _) => "Toxin",
            Item::ScannerPulse => "Scanner",
            Item::PersonalTeleporter => "Teleport",
            Item::StimPack(_) => "Stim",
            Item::EMPGrenade => "EMP",
            Item::RationPack(_) => "Ration",
            Item::FocusStim(_) => "Focus",
            Item::SynthAle(_) => "SynthAle",
            Item::HoloDecoy(_) => "HoloDecoy",
            Item::PlasmaBurst(_) => "Plasma",
            Item::NanoShield(_) => "NanoShld",
            Item::NeuralBoost => "Neural",
            Item::CreditChip(_) => "Credits",
            Item::ShockModule(_) => "Shock",
            Item::BiogelPatch(_) => "Biogel",
            Item::VenomDart => "Venom",
            Item::DeflectorDrone(_) => "Deflect",
            Item::NaniteSwarm => "Nanites",
            Item::Revitalizer(_) => "Revital",
            Item::ReflectorPlate => "Reflect",
            Item::CryoGrenade(_) => "Cryo",
            Item::CloakingDevice(_) => "Cloak",
            Item::PlasmaShield(_) => "PShield",
            Item::SignalJammer(_) => "Jammer",
            Item::NavComputer => "NavComp",
            Item::GrappleLine => "Grapple",
            Item::OmniGel => "OmniGel",
            Item::SonicEmitter(_) => "Sonic",
            Item::CircuitInk => "Circuit",
            Item::DataCore(_) => "DataCore",
            Item::ThrusterPack => "Thruster",
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
            Item::MedHypo(_) => "Injects nanite-infused medicine to restore HP instantly. Use during exploration or combat.",
            Item::ToxinGrenade(_, _) => "Lob at an adjacent enemy to inflict toxic damage over several turns.",
            Item::ScannerPulse => "Emits a wide-band scan revealing the entire deck layout, rooms, and hidden areas.",
            Item::PersonalTeleporter => "Short-range teleport to a random explored tile. Perfect for emergency extraction.",
            Item::StimPack(_) => "Military-grade stimulant granting Haste for extra actions each turn.",
            Item::EMPGrenade => "Electromagnetic pulse that disables all visible enemies for several turns.",
            Item::RationPack(_) => "Standard-issue rations that restore spirit energy to stave off exhaustion.",
            Item::FocusStim(_) => "Neural stabilizer that shields your spirit from decay for several moves.",
            Item::SynthAle(_) => "Potent synthetic alcohol that fully restores spirit but leaves you disoriented.",
            Item::HoloDecoy(_) => "Projects a holographic duplicate, granting you swift movement for several turns.",
            Item::PlasmaBurst(_) => "Overcharged plasma cell that detonates dealing damage to all visible enemies.",
            Item::NanoShield(_) => "Deploys a nanite barrier granting a protective shield and slow regeneration.",
            Item::NeuralBoost => "Cortical stimulant that purges all negative effects from your neural system.",
            Item::CreditChip(_) => "A high-denomination credit chip. Can be cashed in for instant wealth.",
            Item::ShockModule(_) => "Discharges a focused energy bolt at the nearest enemy, dealing heavy damage.",
            Item::BiogelPatch(_) => "Medical-grade biogel that slowly regenerates wounds over time.",
            Item::VenomDart => "Coats your weapon in synthesized neurotoxin, envenoming strikes for several turns.",
            Item::DeflectorDrone(_) => "Deploys a drone that projects shields protecting both body and spirit.",
            Item::NaniteSwarm => "Releases blinding nanites on all visible enemies, stunning them in place.",
            Item::Revitalizer(_) => "Emergency revival system that activates on death, restoring you from critical state.",
            Item::ReflectorPlate => "Energy-reflective plating that bounces the next attack back at the attacker.",
            Item::CryoGrenade(_) => "Flash-freezes all adjacent enemies solid for a short time on detonation.",
            Item::CloakingDevice(_) => "Bends light around you, making you invisible to enemies for several turns.",
            Item::PlasmaShield(_) => "Hardened plasma barrier that grants bonus armor for the rest of combat.",
            Item::SignalJammer(_) => "Scrambles enemy communications, confusing all enemies for several turns.",
            Item::NavComputer => "Portable navigation computer that reveals all traps and hidden areas on the deck.",
            Item::GrappleLine => "Magnetic grapple line that pulls a distant enemy to an adjacent tile.",
            Item::OmniGel => "Universal repair compound that cures all negative status effects instantly.",
            Item::SonicEmitter(_) => "Emits destructive sound waves dealing damage to all enemies and slowing them.",
            Item::CircuitInk => "Conductive nano-ink that empowers your next spell to deal double damage.",
            Item::DataCore(_) => "Emergency backup data core that revives you with HP upon death. Consumed on use.",
            Item::ThrusterPack => "Directional thruster burst that blasts all adjacent enemies away.",
        }
    }
}

pub const EQUIPMENT_POOL: &[Equipment] = &[
    Equipment {
        name: "Laser Pistol",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::BonusDamage(1),
    },
    Equipment {
        name: "Plasma Rifle",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::BonusDamage(2),
    },
    Equipment {
        name: "Arc Emitter",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::BonusDamage(3),
    },
    Equipment {
        name: "Flight Suit",
        slot: EquipSlot::Armor,
        effect: EquipEffect::DamageReduction(1),
    },
    Equipment {
        name: "Kevlar Vest",
        slot: EquipSlot::Armor,
        effect: EquipEffect::DamageReduction(2),
    },
    Equipment {
        name: "Power Armor",
        slot: EquipSlot::Armor,
        effect: EquipEffect::DamageReduction(3),
    },
    Equipment {
        name: "Scanner Array",
        slot: EquipSlot::Charm,
        effect: EquipEffect::ExtraRadicalDrop(50),
    },
    Equipment {
        name: "Auto-Repair Module",
        slot: EquipSlot::Charm,
        effect: EquipEffect::HealOnKill(2),
    },
    Equipment {
        name: "Salvage Processor",
        slot: EquipSlot::Charm,
        effect: EquipEffect::GoldBonus(10),
    },
    Equipment {
        name: "Targeting Computer",
        slot: EquipSlot::Charm,
        effect: EquipEffect::HealOnKill(3),
    },
    Equipment {
        name: "Plasma Torch",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::Digging,
    },
    Equipment {
        name: "Energy Recycler",
        slot: EquipSlot::Charm,
        effect: EquipEffect::SpiritSustain,
    },
    Equipment {
        name: "Psi Amplifier",
        slot: EquipSlot::Charm,
        effect: EquipEffect::SpellPowerBoost(1),
    },
    Equipment {
        name: "Zero Blade",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::LifeSteal(1),
    },
    Equipment {
        name: "Nanoweave Suit",
        slot: EquipSlot::Armor,
        effect: EquipEffect::DodgeChance(15),
    },
    Equipment {
        name: "Neural Uplink",
        slot: EquipSlot::Charm,
        effect: EquipEffect::FocusRegen(1),
    },
    Equipment {
        name: "Ion Cannon",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::KnockbackStrike,
    },
    Equipment {
        name: "Energy Shield",
        slot: EquipSlot::Armor,
        effect: EquipEffect::ThornsAura(1),
    },
    Equipment {
        name: "Threat Analyzer",
        slot: EquipSlot::Charm,
        effect: EquipEffect::EnemyIntentReveal,
    },
    Equipment {
        name: "Void Lance",
        slot: EquipSlot::Weapon,
        effect: EquipEffect::CriticalStrike(20),
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Faction {
    Consortium,       // Commerce & diplomacy
    FreeTraders,      // Exploration & freedom
    Technocracy,      // Knowledge & technology
    MilitaryAlliance, // Warfare & defense
    AncientOrder,     // Wealth & ancient secrets
}

// Type alias for backward compatibility with other modules
pub type Deity = Faction;

impl Faction {
    pub fn name(&self) -> &'static str {
        match self {
            Faction::Consortium => "Stellar Consortium (Commerce)",
            Faction::FreeTraders => "Free Traders Guild (Exploration)",
            Faction::Technocracy => "Technocracy (Knowledge)",
            Faction::MilitaryAlliance => "Military Alliance (Defense)",
            Faction::AncientOrder => "Ancient Order (Secrets)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerForm {
    Human,
    Powered, // Energy damage aura, immune to energy
    #[allow(dead_code)]
    Cybernetic, // Enhanced armor, reduced speed
    Holographic, // Phase through, weak attack
    Void, // Void damage, fast
}

impl PlayerForm {
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            PlayerForm::Human => "Human",
            PlayerForm::Powered => "Powered Suit",
            PlayerForm::Cybernetic => "Cybernetic",
            PlayerForm::Holographic => "Holographic",
            PlayerForm::Void => "Void Walker",
        }
    }

    pub fn glyph(&self) -> &'static str {
        match self {
            PlayerForm::Human => "@",
            PlayerForm::Powered => "⚡",
            PlayerForm::Cybernetic => "⛨",
            PlayerForm::Holographic => "◇",
            PlayerForm::Void => "◈",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            PlayerForm::Human => "#ffffff",
            PlayerForm::Powered => "#ff5500",
            PlayerForm::Cybernetic => "#888888",
            PlayerForm::Holographic => "#aaddff",
            PlayerForm::Void => "#ffaa00",
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
    Envoy,
    Mechanic,
    Mystic,
    Operative,
    Solarian,
    Soldier,
    Technomancer,
}

impl PlayerClass {
    pub fn all() -> Vec<PlayerClass> {
        vec![
            PlayerClass::Envoy,
            PlayerClass::Mechanic,
            PlayerClass::Mystic,
            PlayerClass::Operative,
            PlayerClass::Solarian,
            PlayerClass::Soldier,
            PlayerClass::Technomancer,
        ]
    }

    pub fn data(&self) -> ClassData {
        match self {
            PlayerClass::Envoy => ClassData {
                name_en: "Envoy",
                name_cn: "使节",
                lore: "Diplomat. Crew morale+, shop discount.",
                color: "#88ccff",
                icon: "E",
            },
            PlayerClass::Mechanic => ClassData {
                name_en: "Mechanic",
                name_cn: "技师",
                lore: "Tech expert. Repair+, drone ally.",
                color: "#88ff88",
                icon: "M",
            },
            PlayerClass::Mystic => ClassData {
                name_en: "Mystic",
                name_cn: "秘术师",
                lore: "Psychic powers. Spell+, sense foes.",
                color: "#cc88ff",
                icon: "Y",
            },
            PlayerClass::Operative => ClassData {
                name_en: "Operative",
                name_cn: "特工",
                lore: "Stealth/skills. Init+, crit hits.",
                color: "#aa4444",
                icon: "O",
            },
            PlayerClass::Solarian => ClassData {
                name_en: "Solarian",
                name_cn: "恒星者",
                lore: "Cosmic warrior. Melee+, solar mode.",
                color: "#ffaa00",
                icon: "S",
            },
            PlayerClass::Soldier => ClassData {
                name_en: "Soldier",
                name_cn: "士兵",
                lore: "Heavy combat. HP+, armor+, ranged+.",
                color: "#ff8888",
                icon: "D",
            },
            PlayerClass::Technomancer => ClassData {
                name_en: "Technomancer",
                name_cn: "技法师",
                lore: "Tech+magic. Spell variety, hacking.",
                color: "#ffffff",
                icon: "T",
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
    /// Active faction favor (piety)
    pub piety: Vec<(Faction, i32)>,
    /// Current physical form
    pub form: PlayerForm,
    /// Turns remaining in current form (0 = permanent/human)
    pub form_timer: i32,
    /// Spirit energy — ticks down each move, player takes damage at 0
    pub spirit: i32,
    pub max_spirit: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CrewRole {
    ScienceOfficer,
    Medic,
    Quartermaster,
    SecurityChief,
    Pilot,
    Engineer,
}

impl CrewRole {
    pub fn name(&self) -> &'static str {
        match self {
            CrewRole::ScienceOfficer => "Science Officer",
            CrewRole::Medic => "Medic",
            CrewRole::Quartermaster => "Quartermaster",
            CrewRole::SecurityChief => "Security Chief",
            CrewRole::Pilot => "Pilot",
            CrewRole::Engineer => "Engineer",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            CrewRole::ScienceOfficer => "🔬",
            CrewRole::Medic => "💊",
            CrewRole::Quartermaster => "📦",
            CrewRole::SecurityChief => "🛡",
            CrewRole::Pilot => "🚀",
            CrewRole::Engineer => "🔧",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Ship {
    pub hull: i32,
    pub max_hull: i32,
    pub fuel: i32,
    pub max_fuel: i32,
    pub shields: i32,
    pub max_shields: i32,
    pub weapon_power: i32,
    pub engine_power: i32,
    pub sensor_range: i32,
    pub cargo_capacity: i32,
    pub cargo_used: i32,
}

impl Ship {
    pub fn new() -> Self {
        Self {
            hull: 50,
            max_hull: 50,
            fuel: 100,
            max_fuel: 100,
            shields: 20,
            max_shields: 20,
            weapon_power: 5,
            engine_power: 5,
            sensor_range: 3,
            cargo_capacity: 20,
            cargo_used: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CrewMember {
    pub name: String,
    pub role: CrewRole,
    pub hp: i32,
    pub max_hp: i32,
    pub level: u8,
    pub xp: u32,
    pub skill: i32,
    pub morale: i32,
}

impl Player {
    pub fn new(x: i32, y: i32, class: PlayerClass) -> Self {
        let (hp, max_hp) = match class {
            PlayerClass::Soldier => (13, 13),
            PlayerClass::Solarian => (11, 11),
            PlayerClass::Envoy | PlayerClass::Mechanic => (10, 10),
            PlayerClass::Mystic | PlayerClass::Technomancer => (9, 9),
            PlayerClass::Operative => (8, 8),
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
            PlayerClass::Mystic => 180,
            PlayerClass::Technomancer | PlayerClass::Envoy => 170,
            PlayerClass::Mechanic => 160,
            _ => 150,
        }
    }

    pub fn has_spirit_sustain(&self) -> bool {
        self.charm
            .map(|c| matches!(c.effect, EquipEffect::SpiritSustain))
            .unwrap_or(false)
    }

    pub fn get_piety(&self, faction: Faction) -> i32 {
        self.piety
            .iter()
            .find(|(d, _)| *d == faction)
            .map(|(_, p)| *p)
            .unwrap_or(0)
    }

    pub fn add_piety(&mut self, faction: Faction, amount: i32) {
        if let Some((_, p)) = self.piety.iter_mut().find(|(d, _)| *d == faction) {
            *p += amount;
        } else {
            self.piety.push((faction, amount));
        }
    }

    pub fn highest_faction(&self) -> Option<Faction> {
        self.piety
            .iter()
            .filter(|&&(_, p)| p > 0)
            .max_by_key(|&&(_, p)| p)
            .map(|&(d, _)| d)
    }

    pub fn faction_bonus(&self, faction: Faction) -> &'static str {
        let p = self.get_piety(faction);
        if p >= 15 {
            match faction {
                Faction::Consortium => "Major: +1 HP on kill",
                Faction::MilitaryAlliance => "Major: +1 bonus damage",
                Faction::AncientOrder => "Major: +3 bonus credits on kill",
                Faction::FreeTraders => "Major: 15% evade on wrong answer",
                Faction::Technocracy => "Major: Show pinyin on wrong answer",
            }
        } else if p >= 10 {
            match faction {
                Faction::Consortium => "Moderate: +1 HP on kill",
                Faction::MilitaryAlliance => "Moderate: +1 bonus damage",
                Faction::AncientOrder => "Moderate: +3 bonus credits on kill",
                Faction::FreeTraders => "Moderate: 15% evade on wrong answer",
                Faction::Technocracy => "Moderate: Show pinyin on wrong answer",
            }
        } else if p >= 5 {
            "Minor standing"
        } else {
            "None"
        }
    }

    pub fn faction_synergy(&self) -> Option<(&'static str, &'static str)> {
        let p = |d| self.get_piety(d) >= 10;
        if p(Faction::Consortium) && p(Faction::MilitaryAlliance) {
            Some(("Vanguard Protocol", "Heal 1 HP per kill AND +1 damage"))
        } else if p(Faction::Consortium) && p(Faction::AncientOrder) {
            Some(("Trade Accord", "+5 credits per deck cleared"))
        } else if p(Faction::Technocracy) && p(Faction::FreeTraders) {
            Some(("Deep Scan Array", "Reveal map on deck entry (25% chance)"))
        } else if p(Faction::MilitaryAlliance) && p(Faction::AncientOrder) {
            Some(("War Profiteer", "Enemies drop double credits"))
        } else if p(Faction::Technocracy) && p(Faction::MilitaryAlliance) {
            Some(("Tactical Uplink", "+2 bonus damage to elites"))
        } else if p(Faction::FreeTraders) && p(Faction::AncientOrder) {
            Some(("Smuggler's Luck", "25% chance for extra item on deck"))
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
            PlayerClass::Mechanic => 7,
            PlayerClass::Envoy | PlayerClass::Technomancer => 6,
            PlayerClass::Mystic | PlayerClass::Operative => 5,
            PlayerClass::Soldier | PlayerClass::Solarian => 4,
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
    use super::{Faction, Item, ItemKind, Player, PlayerClass};

    #[test]
    fn item_kind_matches_variant() {
        assert_eq!(Item::MedHypo(5).kind(), ItemKind::MedHypo);
        assert_eq!(Item::PersonalTeleporter.kind(), ItemKind::PersonalTeleporter);
    }

    #[test]
    fn item_display_name_uses_mystery_label_until_identified() {
        let item = Item::ScannerPulse;

        assert_eq!(
            item.display_name(false, "Green Capsule ◇"),
            "? Green Capsule ◇"
        );
        assert_eq!(
            item.display_name(true, "Green Capsule ◇"),
            "📡 Scanner Pulse"
        );
    }

    #[test]
    fn faction_synergy_requires_dual_standing() {
        let mut player = Player::new(0, 0, PlayerClass::Soldier);
        player.add_piety(Faction::Consortium, 10);
        player.add_piety(Faction::MilitaryAlliance, 10);
        assert_eq!(
            player.faction_synergy(),
            Some(("Vanguard Protocol", "Heal 1 HP per kill AND +1 damage"))
        );
    }

    #[test]
    fn faction_synergy_returns_none_without_threshold() {
        let mut player = Player::new(0, 0, PlayerClass::Soldier);
        player.add_piety(Faction::Consortium, 9);
        player.add_piety(Faction::MilitaryAlliance, 10);
        assert_eq!(player.faction_synergy(), None);
    }

    #[test]
    fn faction_bonus_tiers() {
        let mut player = Player::new(0, 0, PlayerClass::Soldier);
        assert_eq!(player.faction_bonus(Faction::Consortium), "None");

        player.add_piety(Faction::Consortium, 5);
        assert_eq!(player.faction_bonus(Faction::Consortium), "Minor standing");

        player.add_piety(Faction::Consortium, 5);
        assert_eq!(
            player.faction_bonus(Faction::Consortium),
            "Moderate: +1 HP on kill"
        );

        player.add_piety(Faction::Consortium, 5);
        assert_eq!(player.faction_bonus(Faction::Consortium), "Major: +1 HP on kill");
    }
}
