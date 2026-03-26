//! World module — space station generation, tile types, fog of war.
#![allow(dead_code)]

pub mod fov;
pub mod location_gen;
pub mod starmap;
pub mod ship;
// TODO: create during integration
pub mod events;
pub mod dialogue;

pub use fov::compute_fov;

use crate::player::Faction;

// ── PRNG ────────────────────────────────────────────────────────────────────

/// Simple PRNG (xorshift64) so we don't need an external crate.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 { 1 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Random i32 in [lo, hi) (hi > lo).
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo) as u64) as i32
    }
}

// ── Location type (space exploration) ───────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LocationType {
    SpaceStation,
    AsteroidBase,
    DerelictShip,
    AlienRuins,
    TradingPost,
    OrbitalPlatform,
    MiningColony,
    ResearchLab,
}

// ── Terminal kind (was AltarKind) ───────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TerminalKind {
    Quantum,
    Stellar,
    Holographic,
    Tactical,
    Commerce,
}

/// Backward-compatible alias.
pub type AltarKind = TerminalKind;

impl TerminalKind {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Quantum => "☯",
            Self::Stellar => "✦",
            Self::Holographic => "◈",
            Self::Tactical => "⚔",
            Self::Commerce => "¥",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Self::Quantum => "#66dd99",
            Self::Stellar => "#88ccff",
            Self::Holographic => "#ddb8ff",
            Self::Tactical => "#ff5555",
            Self::Commerce => "#ffd700",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Quantum => "Quantum Terminal",
            Self::Stellar => "Stellar Terminal",
            Self::Holographic => "Holographic Terminal",
            Self::Tactical => "Tactical Terminal",
            Self::Commerce => "Commerce Terminal",
        }
    }

    pub fn deity(self) -> Faction {
        match self {
            Self::Quantum => Faction::Consortium,
            Self::Stellar => Faction::FreeTraders,
            Self::Holographic => Faction::Technocracy,
            Self::Tactical => Faction::MilitaryAlliance,
            Self::Commerce => Faction::AncientOrder,
        }
    }

    pub fn random(rng: &mut Rng) -> Self {
        match rng.next_u64() % 5 {
            0 => Self::Quantum,
            1 => Self::Stellar,
            2 => Self::Holographic,
            3 => Self::Tactical,
            _ => Self::Commerce,
        }
    }
}

// ── Security seal (was SealKind) ────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum SecuritySeal {
    Thermal,
    Hydraulic,
    Kinetic,
    Sonic,
}

/// Backward-compatible alias.
pub type SealKind = SecuritySeal;

impl SecuritySeal {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Thermal => "火",
            Self::Hydraulic => "水",
            Self::Kinetic => "刃",
            Self::Sonic => "回",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Self::Thermal => "#ff9b73",
            Self::Hydraulic => "#90c9ff",
            Self::Kinetic => "#ff9eb8",
            Self::Sonic => "#d4a4ff",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Thermal => "Thermal seal",
            Self::Hydraulic => "Hydraulic seal",
            Self::Kinetic => "Kinetic seal",
            Self::Sonic => "Sonic seal",
        }
    }

    pub fn random(rng: &mut Rng) -> Self {
        match rng.next_u64() % 4 {
            0 => Self::Thermal,
            1 => Self::Hydraulic,
            2 => Self::Kinetic,
            _ => Self::Sonic,
        }
    }
}

// ── Special room types ──────────────────────────────────────────────────────

/// 73 unique special room types for location variety.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum SpecialRoomKind {
    // Treasure & Reward (1–10)
    CargoBay,
    OreDeposit,
    HallOfRecords,
    HiddenCache,
    OfferingTerminal,
    HydroponicsGarden,
    PirateStash,
    XenoCrystalVault,
    RelicChamber,
    MedBay,
    // Challenge & Combat (11–20)
    ArenaChallenge,
    SecurityCheckpoint,
    SecurityZone,
    ToxicMaze,
    GravityPuzzle,
    HolographicRoom,
    EnergyNexus,
    SpaceHulkRise,
    PirateAmbush,
    DuelArena,
    // Knowledge & Learning (21–30)
    DataArchive,
    SignalHall,
    ResearcherStudy,
    SensorArray,
    InscriptionWall,
    TrainingSimulator,
    ZenChamber,
    TranslationChallenge,
    AncientDatapad,
    WisdomCore,
    // Environmental & Puzzle (31–40)
    FloodedCompartment,
    CryogenicBay,
    PlasmaCrossing,
    PipeForest,
    VentTunnel,
    CrystalCave,
    FungalGrotto,
    CoolantRiver,
    EchoingHull,
    DarkSector,
    // NPC & Story (41–50)
    WanderingMerchant,
    HermitSage,
    DetentionCell,
    MemorialShrine,
    CantinaBay,
    EngineerWorkshop,
    ChemLab,
    FortuneTeller,
    RefugeeBay,
    WarpGate,
    // Risk/Reward (51–55)
    GamblingDen,
    BloodTerminal,
    CursedSalvage,
    SoulForge,
    WishingReactor,
    // Puzzle (56–60)
    CipherGate,
    HoloMaze,
    GravityPlate,
    ToneFrequency,
    ElementalLock,
    // Timed/Wave (61–64)
    SurvivalBay,
    SalvageRace,
    DepressurizingChamber,
    NanoFlood,
    // Transformation/Permanent (65–68)
    FormShrine,
    ClassTrial,
    RadicalReactor,
    AncestorCrypt,
    // Story/Lore (69–73)
    ProphecyRoom,
    SealedMemory,
    DemonSeal,
    PhoenixNest,
    CalligraphyContest,
    ChallengeTerminal,
}

impl SpecialRoomKind {
    /// Human-readable name shown when the player enters the room.
    pub fn name(self) -> &'static str {
        match self {
            Self::CargoBay => "Cargo Bay",
            Self::OreDeposit => "Ore Deposit",
            Self::HallOfRecords => "Hall of Records",
            Self::HiddenCache => "Hidden Cache",
            Self::OfferingTerminal => "Offering Terminal",
            Self::HydroponicsGarden => "Hydroponics Garden",
            Self::PirateStash => "Pirate Stash",
            Self::XenoCrystalVault => "Xeno-Crystal Vault",
            Self::RelicChamber => "Relic Chamber",
            Self::MedBay => "Med Bay",
            Self::ArenaChallenge => "Arena Challenge",
            Self::SecurityCheckpoint => "Security Checkpoint",
            Self::SecurityZone => "Security Zone",
            Self::ToxicMaze => "Toxic Maze",
            Self::GravityPuzzle => "Gravity Puzzle",
            Self::HolographicRoom => "Holographic Room",
            Self::EnergyNexus => "Energy Nexus",
            Self::SpaceHulkRise => "Space Hulk Rise",
            Self::PirateAmbush => "Pirate Ambush",
            Self::DuelArena => "Duel Arena",
            Self::DataArchive => "Data Archive",
            Self::SignalHall => "Signal Hall",
            Self::ResearcherStudy => "Researcher's Study",
            Self::SensorArray => "Sensor Array",
            Self::InscriptionWall => "Inscription Wall",
            Self::TrainingSimulator => "Training Simulator",
            Self::ZenChamber => "Zen Chamber",
            Self::TranslationChallenge => "Translation Challenge",
            Self::AncientDatapad => "Ancient Datapad",
            Self::WisdomCore => "Wisdom Core",
            Self::FloodedCompartment => "Flooded Compartment",
            Self::CryogenicBay => "Cryogenic Bay",
            Self::PlasmaCrossing => "Plasma Crossing",
            Self::PipeForest => "Pipe Forest",
            Self::VentTunnel => "Vent Tunnel",
            Self::CrystalCave => "Crystal Cave",
            Self::FungalGrotto => "Fungal Grotto",
            Self::CoolantRiver => "Coolant River",
            Self::EchoingHull => "Echoing Hull",
            Self::DarkSector => "Dark Sector",
            Self::WanderingMerchant => "Wandering Merchant",
            Self::HermitSage => "Hermit Sage",
            Self::DetentionCell => "Detention Cell",
            Self::MemorialShrine => "Memorial Shrine",
            Self::CantinaBay => "Cantina Bay",
            Self::EngineerWorkshop => "Engineer Workshop",
            Self::ChemLab => "Chem Lab",
            Self::FortuneTeller => "Fortune Teller",
            Self::RefugeeBay => "Refugee Bay",
            Self::WarpGate => "Warp Gate",
            Self::GamblingDen => "Gambling Den",
            Self::BloodTerminal => "Blood Terminal",
            Self::CursedSalvage => "Cursed Salvage",
            Self::SoulForge => "Soul Forge",
            Self::WishingReactor => "Wishing Reactor",
            Self::CipherGate => "Cipher Gate",
            Self::HoloMaze => "Holo-Maze",
            Self::GravityPlate => "Gravity Plate",
            Self::ToneFrequency => "Tone Frequency",
            Self::ElementalLock => "Elemental Lock",
            Self::SurvivalBay => "Survival Bay",
            Self::SalvageRace => "Salvage Race",
            Self::DepressurizingChamber => "Depressurizing Chamber",
            Self::NanoFlood => "Nano Flood",
            Self::FormShrine => "Form Shrine",
            Self::ClassTrial => "Class Trial",
            Self::RadicalReactor => "Radical Reactor",
            Self::AncestorCrypt => "Ancestor's Crypt",
            Self::ProphecyRoom => "Prophecy Room",
            Self::SealedMemory => "Sealed Memory",
            Self::DemonSeal => "Demon Seal",
            Self::PhoenixNest => "Phoenix Nest",
            Self::CalligraphyContest => "Calligraphy Contest",
            Self::ChallengeTerminal => "Challenge Terminal",
        }
    }

    /// Description shown when entering the room.
    pub fn description(self) -> &'static str {
        match self {
            Self::CargoBay => "A sealed cargo bay with supply crates, guarded by a security drone.",
            Self::OreDeposit => "Rich mineral veins line the asteroid walls. Mine them for credits.",
            Self::HallOfRecords => "Holographic records glow softly, offering their data.",
            Self::HiddenCache => "A concealed panel reveals rare supplies.",
            Self::OfferingTerminal => "An ancient terminal. Insert 50 credits for a powerful upgrade.",
            Self::HydroponicsGarden => "A tranquil garden of engineered flora. Your wounds heal.",
            Self::PirateStash => "Piles of stolen credits and salvage litter the compartment.",
            Self::XenoCrystalVault => "A pristine vault containing a rare xeno-crystal.",
            Self::RelicChamber => "An alien relic pulses with life energy. +1 max HP.",
            Self::MedBay => "Medical nanites activate, restoring health and vitality.",
            Self::ArenaChallenge => "A combat arena where waves of challengers await!",
            Self::SecurityCheckpoint => "Elite security bots block the passage ahead.",
            Self::SecurityZone => "A corridor bristling with automated laser defenses.",
            Self::ToxicMaze => "Toxic fumes drift through this treacherous maze of pipes.",
            Self::GravityPuzzle => "Gravity plating malfunctions — a puzzle awaits.",
            Self::HolographicRoom => "Your holographic double separates and turns hostile!",
            Self::EnergyNexus => "Five energy conduits converge in this nexus.",
            Self::SpaceHulkRise => "Dormant systems in the derelict hulk reactivate...",
            Self::PirateAmbush => "Seems quiet... too quiet.",
            Self::DuelArena => "A ritual dueling arena. Face a champion 1v1.",
            Self::DataArchive => "Dusty data racks hold forgotten knowledge.",
            Self::SignalHall => "Practice halls echo with intercepted transmissions.",
            Self::ResearcherStudy => "A researcher's collection of xenolinguistic texts.",
            Self::SensorArray => "The sensor array reveals all that is hidden on this deck.",
            Self::InscriptionWall => "Ancient inscriptions teach a forgotten radical.",
            Self::TrainingSimulator => "A training simulator. Push yourself to gain +1 base damage.",
            Self::ZenChamber => "A place of deep stillness. Restorative energy gathers here.",
            Self::TranslationChallenge => "Data tablets await correct translations.",
            Self::AncientDatapad => "A weathered datapad contains a rare spell formula.",
            Self::WisdomCore => "An AI wisdom core. Answer its query for a reward.",
            Self::FloodedCompartment => "Coolant has flooded this compartment. Currents pull at you.",
            Self::CryogenicBay => "Ice coats every surface. Movement is treacherous.",
            Self::PlasmaCrossing => "Superheated plasma flows between narrow walkways.",
            Self::PipeForest => "Dense conduit thickets obscure your vision.",
            Self::VentTunnel => "A howling gale pushes through this narrow ventilation shaft.",
            Self::CrystalCave => "Luminous crystals line the walls, refracting light.",
            Self::FungalGrotto => "Alien fungi release clouds of disorienting spores.",
            Self::CoolantRiver => "A coolant river rushes through the compartment.",
            Self::EchoingHull => "Every sound reverberates endlessly in this hollow hull.",
            Self::DarkSector => "Total power failure limits your sight to 2 tiles.",
            Self::WanderingMerchant => "A traveling merchant offers rare and unique wares.",
            Self::HermitSage => "A wise hermit offers to trade HP for spell power.",
            Self::DetentionCell => "Someone is locked in a cell! Free them for an ally.",
            Self::MemorialShrine => "A memorial shrine radiates residual energy.",
            Self::CantinaBay => "A cozy rest stop. Enjoy synth-tea and hear rumors.",
            Self::EngineerWorkshop => "A master engineer can upgrade one piece of equipment.",
            Self::ChemLab => "Bubbling vials and strange reagents fill the lab.",
            Self::FortuneTeller => "A mysterious AI seer offers to reveal the boss's weakness.",
            Self::RefugeeBay => "Displaced travelers seek aid. Help them for karma.",
            Self::WarpGate => "An otherworldly portal shimmers with warp energy.",
            Self::GamblingDen => "Three mysterious containers await. Pick one — fortune or ruin?",
            Self::BloodTerminal => "A crimson terminal pulses with dark energy. Sacrifice HP for power.",
            Self::CursedSalvage => "An ornate crate radiates ominous energy. Great reward… or great cost.",
            Self::SoulForge => "Ethereal flames can transform one radical into another.",
            Self::WishingReactor => "A shimmering reactor. Feed credits to receive a gift from its core.",
            Self::CipherGate => "Four elemental ciphers must be activated in sequence.",
            Self::HoloMaze => "Holographic walls distort space. Navigate carefully to the center.",
            Self::GravityPlate => "Pressure plates and cargo. Push all crates onto plates to unlock.",
            Self::ToneFrequency => "Four ascending frequencies glow with tonal marks. Tune correctly.",
            Self::ElementalLock => "A sealed door requires elemental energy — cast one spell of each element.",
            Self::SurvivalBay => "The exits seal shut! Survive waves of enemies for a reward.",
            Self::SalvageRace => "Credit chips materialize on random tiles! Grab them before they vanish!",
            Self::DepressurizingChamber => "The hull breaches from the edges inward! Reach the center treasure!",
            Self::NanoFlood => "Dark nanites seep across the floor. Stand in them for power, but beware the drain.",
            Self::FormShrine => "An ancient shrine offers transformation into a new form. Choose wisely.",
            Self::ClassTrial => "A trial chamber tests your abilities. Succeed for a class bonus.",
            Self::RadicalReactor => "Luminous reactor refines all radicals you carry. Spells gain +1 damage.",
            Self::AncestorCrypt => "An ancient warrior's crypt. Pray to receive their legendary weapon.",
            Self::ProphecyRoom => "Holographic murals depict the boss of this sector and hint at its weakness.",
            Self::SealedMemory => "Echoes of the past. Recall what you've learned for a reward.",
            Self::DemonSeal => "A sealed entity offers dark power — but at what cost?",
            Self::PhoenixNest => "A radiant nest of eternal plasma. The phoenix bestows its blessing.",
            Self::CalligraphyContest => "A master calligrapher challenges you. Speed and accuracy earn credits.",
            Self::ChallengeTerminal => "A hardened terminal issues a brutal hanzi challenge. Great risk, great reward.",
        }
    }

    /// Weighted random selection based on deck level.
    pub fn random_for_floor(rng: &mut Rng, floor: i32) -> Self {
        let mut pool: Vec<Self> = Vec::new();

        // Always available
        pool.extend_from_slice(&[
            Self::CargoBay, Self::OreDeposit, Self::HallOfRecords,
            Self::HydroponicsGarden, Self::PirateStash, Self::RelicChamber,
            Self::MedBay, Self::ArenaChallenge, Self::SecurityCheckpoint,
            Self::SecurityZone, Self::PirateAmbush,
            Self::DataArchive, Self::SignalHall, Self::ResearcherStudy,
            Self::InscriptionWall, Self::TrainingSimulator, Self::ZenChamber,
            Self::TranslationChallenge, Self::WisdomCore,
            Self::FloodedCompartment, Self::CryogenicBay, Self::PlasmaCrossing,
            Self::PipeForest, Self::VentTunnel, Self::CrystalCave,
            Self::FungalGrotto, Self::EchoingHull, Self::DarkSector,
            Self::WanderingMerchant, Self::CantinaBay, Self::EngineerWorkshop,
            Self::MemorialShrine, Self::RefugeeBay,
            Self::GamblingDen, Self::WishingReactor, Self::SalvageRace,
            Self::CalligraphyContest, Self::ProphecyRoom,
            Self::ChallengeTerminal,
        ]);

        // Deck 3+: harder challenges and more rewards
        if floor >= 3 {
            pool.extend_from_slice(&[
                Self::HiddenCache, Self::OfferingTerminal, Self::XenoCrystalVault,
                Self::ToxicMaze, Self::GravityPuzzle, Self::HolographicRoom,
                Self::SpaceHulkRise, Self::DuelArena,
                Self::SensorArray, Self::AncientDatapad,
                Self::CoolantRiver,
                Self::HermitSage, Self::DetentionCell, Self::ChemLab,
                Self::FortuneTeller,
                Self::BloodTerminal, Self::GravityPlate, Self::ToneFrequency,
                Self::SurvivalBay, Self::ClassTrial, Self::SealedMemory,
            ]);
        }

        // Deck 5+: elemental and advanced
        if floor >= 5 {
            pool.extend_from_slice(&[
                Self::EnergyNexus,
                Self::CursedSalvage, Self::SoulForge, Self::CipherGate,
                Self::HoloMaze, Self::ElementalLock,
                Self::DepressurizingChamber, Self::NanoFlood,
                Self::RadicalReactor, Self::AncestorCrypt,
            ]);
        }

        // Deck 10+: transformation rooms
        if floor >= 10 {
            pool.extend_from_slice(&[
                Self::FormShrine, Self::DemonSeal,
            ]);
        }

        // Deck 20+: rare endgame rooms
        if floor >= 20 {
            pool.push(Self::PhoenixNest);
        }

        // Deck 25+: endgame content
        if floor >= 25 {
            pool.push(Self::WarpGate);
        }

        let idx = rng.next_u64() as usize % pool.len();
        pool[idx]
    }
}

// ── Tile types ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Tile {
    Bulkhead,
    DamagedBulkhead,
    /// A visible weak barrier for optional vault-style puzzle niches
    WeakBulkhead,
    MetalFloor,
    Hallway,
    Airlock,
    QuantumForge,
    TradeTerminal,
    SupplyCrate,
    /// Smashable salvage crate with supplies or traps
    SalvageCrate,
    /// Automated laser grid trap
    LaserGrid,
    /// Slick coolant that can be ignited by fire spells
    Coolant,
    /// Coolant pool that conducts stunning spells
    CoolantPool,
    /// Vacuum breach — impassable unless bridged
    VacuumBreach,
    /// NPC companion (0=Engineer, 1=Medic, 2=Merchant, 3=Marine)
    Npc(u8),
    /// Navigation beacon for mini-game
    NavBeacon,
    /// Faction terminal that grants a blessing
    Terminal(TerminalKind),
    /// Security lock that reshapes the room when activated
    SecurityLock(SecuritySeal),
    /// Info panel with a scripted message
    InfoPanel(u8),
    /// Catwalk created by pushing a crate into a breach
    Catwalk,
    /// Circuit calibration shrine
    CircuitShrine,
    /// Frequency defense wall
    FrequencyWall,
    /// Compound assembly shrine
    CompoundShrine,
    /// Classifier matching node
    ClassifierNode,
    /// Data well — component count quiz, reward HP
    DataWell,
    /// Memorial node — proverb completion
    MemorialNode,
    /// Translation terminal — pick correct translation
    TranslationTerminal,
    /// Radical lab — identify the radical of a character
    RadicalLab,
    /// Holographic pool — type pinyin for shown character
    HoloPool,
    /// Droid tutor — teaches then quizzes
    DroidTutor,
    /// Codex terminal — quizzes characters from player's codex
    CodexTerminal,
    /// Data bridge — answer vocab to extend bridge over breach
    DataBridge,
    /// Sealed hatch — blocks passage until question answered
    SealedHatch,
    /// Corrupted floor — hidden trap, quiz when stepped on
    CorruptedFloor,
    /// Hidden trap tile (type: 0=toxin, 1=teleport, 2=alarm)
    Trap(u8),
    /// Ore vein — mine for credits
    OreVein,
    /// Plasma vent — deals damage when walked on
    PlasmaVent,
    /// Frozen deck — slippery, sliding movement
    FrozenDeck,
    /// Cargo pipes — blocks movement and line of sight
    CargoPipes,
    /// Toxic fungus — spore cloud causes confusion
    ToxicFungus,
    /// Toxic gas — applies poison when walked through
    ToxicGas,
    /// Data rack — interactable, teaches knowledge
    DataRack,
    /// Pressure sensor — puzzle element
    PressureSensor,
    /// Cargo crate — pushable onto pressure sensors
    CargoCrate,
    /// Crystal panel — reflective wall
    CrystalPanel,
    /// Warp gate portal — end-game content
    WarpGatePortal,
    /// Med bay tile — restores HP
    MedBayTile,
    /// Credit cache — pick up for credits
    CreditCache,
    /// Special room marker
    SpecialRoom(SpecialRoomKind),
}

impl Tile {
    pub fn is_walkable(self) -> bool {
        matches!(
            self,
            Tile::MetalFloor
                | Tile::Hallway
                | Tile::Airlock
                | Tile::QuantumForge
                | Tile::TradeTerminal
                | Tile::SupplyCrate
                | Tile::LaserGrid
                | Tile::Coolant
                | Tile::CoolantPool
                | Tile::Npc(_)
                | Tile::NavBeacon
                | Tile::Terminal(_)
                | Tile::SecurityLock(_)
                | Tile::InfoPanel(_)
                | Tile::Catwalk
                | Tile::CircuitShrine
                | Tile::FrequencyWall
                | Tile::CompoundShrine
                | Tile::ClassifierNode
                | Tile::DataWell
                | Tile::MemorialNode
                | Tile::TranslationTerminal
                | Tile::RadicalLab
                | Tile::HoloPool
                | Tile::DroidTutor
                | Tile::CodexTerminal
                | Tile::DataBridge
                | Tile::CorruptedFloor
                | Tile::Trap(_)
                | Tile::PlasmaVent
                | Tile::FrozenDeck
                | Tile::ToxicFungus
                | Tile::ToxicGas
                | Tile::DataRack
                | Tile::PressureSensor
                | Tile::WarpGatePortal
                | Tile::MedBayTile
                | Tile::CreditCache
                | Tile::SpecialRoom(_)
        )
    }
}

// ── Room modifier ───────────────────────────────────────────────────────────

/// Room environment modifier affecting gameplay.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RoomModifier {
    /// Reduced FOV to 2 tiles
    PoweredDown,
    /// Spells deal 2x damage
    HighTech,
    /// Enemies take 1 extra damage per hit
    Irradiated,
    /// Engineered flora and natural terrain
    Hydroponics,
    /// Cryogenic and frozen terrain
    Cryogenic,
    /// Overheated reactor and fire hazards
    OverheatedReactor,
}

// ── Room descriptor ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Room {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub modifier: Option<RoomModifier>,
    pub special: Option<SpecialRoomKind>,
}

impl Room {
    pub fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }

    #[allow(dead_code)]
    pub fn intersects(&self, other: &Room) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }
}

// ── LocationLevel (was DungeonLevel) ────────────────────────────────────────

pub struct LocationLevel {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Tile>,
    pub rooms: Vec<Room>,
    pub visible: Vec<bool>,
    pub revealed: Vec<bool>,
}

/// Backward-compatible alias.
pub type DungeonLevel = LocationLevel;

impl LocationLevel {
    pub fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.width + x) as usize
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }

    pub fn tile(&self, x: i32, y: i32) -> Tile {
        if self.in_bounds(x, y) {
            self.tiles[self.idx(x, y)]
        } else {
            Tile::Bulkhead
        }
    }

    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && self.tiles[self.idx(x, y)].is_walkable()
    }
}

#[cfg(test)]
mod tests;
