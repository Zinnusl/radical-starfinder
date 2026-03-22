//! BSP-based dungeon generation.
//!
//! Splits a rectangular area recursively, places rooms in leaves,
//! then connects sibling rooms with corridors.

use crate::player::Deity;

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

// ── Tile types ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum AltarKind {
    Jade,
    Gale,
    Mirror,
    Iron,
    Gold,
}

impl AltarKind {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Jade => "☯",   // Yin-Yang (Balance/Life)
            Self::Gale => "✦",   // Sparkle/Wind
            Self::Mirror => "◈", // Diamond/Reflection
            Self::Iron => "⚔",   // Crossed Swords (War)
            Self::Gold => "¥",   // Yen/Yuan (Wealth)
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Self::Jade => "#66dd99",   // Green
            Self::Gale => "#88ccff",   // Sky Blue
            Self::Mirror => "#ddb8ff", // Purple
            Self::Iron => "#ff5555",   // Red
            Self::Gold => "#ffd700",   // Gold
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Jade => "Jade Altar",
            Self::Gale => "Gale Altar",
            Self::Mirror => "Mirror Altar",
            Self::Iron => "Iron Altar",
            Self::Gold => "Gold Altar",
        }
    }

    pub fn deity(self) -> Deity {
        match self {
            Self::Jade => Deity::Jade,
            Self::Gale => Deity::Gale,
            Self::Mirror => Deity::Mirror,
            Self::Iron => Deity::Iron,
            Self::Gold => Deity::Gold,
        }
    }

    fn random(rng: &mut Rng) -> Self {
        match rng.next_u64() % 5 {
            0 => Self::Jade,
            1 => Self::Gale,
            2 => Self::Mirror,
            3 => Self::Iron,
            _ => Self::Gold,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SealKind {
    Ember,
    Tide,
    Thorn,
    Echo,
}

impl SealKind {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Ember => "火",
            Self::Tide => "水",
            Self::Thorn => "刃",
            Self::Echo => "回",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Self::Ember => "#ff9b73",
            Self::Tide => "#90c9ff",
            Self::Thorn => "#ff9eb8",
            Self::Echo => "#d4a4ff",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ember => "Ember seal",
            Self::Tide => "Tide seal",
            Self::Thorn => "Thorn seal",
            Self::Echo => "Echo seal",
        }
    }

    fn random(rng: &mut Rng) -> Self {
        match rng.next_u64() % 4 {
            0 => Self::Ember,
            1 => Self::Tide,
            2 => Self::Thorn,
            _ => Self::Echo,
        }
    }
}

// ── Special room types ──────────────────────────────────────────────────────

/// 73 unique special room types for dungeon variety.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum SpecialRoomKind {
    // Treasure & Reward (1–10)
    TreasureVault,
    GoldMine,
    AncestralHall,
    HiddenCache,
    OfferingAltar,
    LotusGarden,
    DragonHoard,
    JadeVault,
    RelicChamber,
    SpiritSpring,
    // Challenge & Combat (11–20)
    ArenaChallenge,
    EliteGuardPost,
    TrapGauntlet,
    PoisonMaze,
    BoulderPuzzle,
    MirrorRoom,
    ElementalNexus,
    GraveyardRise,
    BanditAmbush,
    DuelCircle,
    // Knowledge & Learning (21–30)
    Library,
    CalligraphyHall,
    ScholarStudy,
    OracleRoom,
    InscriptionWall,
    MasterDojo,
    MeditationChamber,
    TranslationChallenge,
    AncientScroll,
    WisdomWell,
    // Environmental & Puzzle (31–40)
    FloodedChamber,
    FrozenCavern,
    LavaCrossing,
    BambooForest,
    WindTunnel,
    CrystalCave,
    MushroomGrotto,
    UndergroundRiver,
    EchoingCavern,
    ShadowRealm,
    // NPC & Story (41–50)
    WanderingMerchant,
    HermitSage,
    PrisonerCell,
    SpiritShrine,
    TeaHouse,
    SmithyWorkshop,
    AlchemyLab,
    FortuneTeller,
    RefugeeCamp,
    DragonGate,
    // Risk/Reward (51–55)
    GamblingDen,
    BloodAltar,
    CursedTreasure,
    SoulForge,
    WishingWell,
    // Puzzle (56–60)
    RuneGate,
    MirrorMaze,
    WeightPuzzle,
    ToneStaircase,
    ElementalLock,
    // Timed/Wave (61–64)
    SurvivalPit,
    TreasureRace,
    CollapsingChamber,
    InkFlood,
    // Transformation/Permanent (65–68)
    FormShrine,
    ClassTrial,
    RadicalFountain,
    AncestorTomb,
    // Story/Lore (69–73)
    ProphecyRoom,
    SealedMemory,
    DemonSeal,
    PhoenixNest,
    CalligraphyContest,
}

impl SpecialRoomKind {
    /// Human-readable name shown when the player enters the room.
    pub fn name(self) -> &'static str {
        match self {
            Self::TreasureVault => "Treasure Vault",
            Self::GoldMine => "Gold Mine",
            Self::AncestralHall => "Ancestral Hall",
            Self::HiddenCache => "Hidden Cache",
            Self::OfferingAltar => "Offering Altar",
            Self::LotusGarden => "Lotus Garden",
            Self::DragonHoard => "Dragon Hoard",
            Self::JadeVault => "Jade Vault",
            Self::RelicChamber => "Relic Chamber",
            Self::SpiritSpring => "Spirit Spring",
            Self::ArenaChallenge => "Arena Challenge",
            Self::EliteGuardPost => "Elite Guard Post",
            Self::TrapGauntlet => "Trap Gauntlet",
            Self::PoisonMaze => "Poison Maze",
            Self::BoulderPuzzle => "Boulder Puzzle",
            Self::MirrorRoom => "Mirror Room",
            Self::ElementalNexus => "Elemental Nexus",
            Self::GraveyardRise => "Graveyard Rise",
            Self::BanditAmbush => "Bandit Ambush",
            Self::DuelCircle => "Duel Circle",
            Self::Library => "Library",
            Self::CalligraphyHall => "Calligraphy Hall",
            Self::ScholarStudy => "Scholar's Study",
            Self::OracleRoom => "Oracle Room",
            Self::InscriptionWall => "Inscription Wall",
            Self::MasterDojo => "Master's Dojo",
            Self::MeditationChamber => "Meditation Chamber",
            Self::TranslationChallenge => "Translation Challenge",
            Self::AncientScroll => "Ancient Scroll Room",
            Self::WisdomWell => "Wisdom Well",
            Self::FloodedChamber => "Flooded Chamber",
            Self::FrozenCavern => "Frozen Cavern",
            Self::LavaCrossing => "Lava Crossing",
            Self::BambooForest => "Bamboo Forest",
            Self::WindTunnel => "Wind Tunnel",
            Self::CrystalCave => "Crystal Cave",
            Self::MushroomGrotto => "Mushroom Grotto",
            Self::UndergroundRiver => "Underground River",
            Self::EchoingCavern => "Echoing Cavern",
            Self::ShadowRealm => "Shadow Realm",
            Self::WanderingMerchant => "Wandering Merchant",
            Self::HermitSage => "Hermit Sage",
            Self::PrisonerCell => "Prisoner Cell",
            Self::SpiritShrine => "Spirit Shrine",
            Self::TeaHouse => "Tea House",
            Self::SmithyWorkshop => "Smithy Workshop",
            Self::AlchemyLab => "Alchemy Lab",
            Self::FortuneTeller => "Fortune Teller",
            Self::RefugeeCamp => "Refugee Camp",
            Self::DragonGate => "Dragon Gate",
            Self::GamblingDen => "Gambling Den",
            Self::BloodAltar => "Blood Altar",
            Self::CursedTreasure => "Cursed Treasure",
            Self::SoulForge => "Soul Forge",
            Self::WishingWell => "Wishing Well",
            Self::RuneGate => "Rune Gate",
            Self::MirrorMaze => "Mirror Maze",
            Self::WeightPuzzle => "Weight Puzzle",
            Self::ToneStaircase => "Tone Staircase",
            Self::ElementalLock => "Elemental Lock",
            Self::SurvivalPit => "Survival Pit",
            Self::TreasureRace => "Treasure Race",
            Self::CollapsingChamber => "Collapsing Chamber",
            Self::InkFlood => "Ink Flood",
            Self::FormShrine => "Form Shrine",
            Self::ClassTrial => "Class Trial",
            Self::RadicalFountain => "Radical Fountain",
            Self::AncestorTomb => "Ancestor's Tomb",
            Self::ProphecyRoom => "Prophecy Room",
            Self::SealedMemory => "Sealed Memory",
            Self::DemonSeal => "Demon Seal",
            Self::PhoenixNest => "Phoenix Nest",
            Self::CalligraphyContest => "Calligraphy Contest",
        }
    }

    /// Description shown when entering the room.
    pub fn description(self) -> &'static str {
        match self {
            Self::TreasureVault => "A locked vault with chests, guarded by a powerful sentinel.",
            Self::GoldMine => "Glittering ore veins line the walls. Mine them for gold.",
            Self::AncestralHall => "Ancestral tablets glow softly, offering their blessings.",
            Self::HiddenCache => "A concealed alcove with rare supplies.",
            Self::OfferingAltar => "An ancient altar. Sacrifice 50 gold for a powerful gift.",
            Self::LotusGarden => "A tranquil garden of blooming lotus. Your wounds heal.",
            Self::DragonHoard => "Piles of gold glitter across the chamber floor.",
            Self::JadeVault => "A pristine vault containing a rare jade radical.",
            Self::RelicChamber => "An ancient relic pulses with life energy. +1 max HP.",
            Self::SpiritSpring => "Crystal water bubbles up, restoring spirit and health.",
            Self::ArenaChallenge => "An arena where waves of challengers await!",
            Self::EliteGuardPost => "Elite guards block the passage ahead.",
            Self::TrapGauntlet => "A corridor bristling with deadly spike traps.",
            Self::PoisonMaze => "Toxic fumes drift through this treacherous maze.",
            Self::BoulderPuzzle => "Heavy boulders and pressure plates — a puzzle awaits.",
            Self::MirrorRoom => "Your shadow separates and turns hostile!",
            Self::ElementalNexus => "Five elemental forces converge in this nexus.",
            Self::GraveyardRise => "The dead stir in their graves...",
            Self::BanditAmbush => "Seems quiet... too quiet.",
            Self::DuelCircle => "A ritual dueling circle. Face a champion 1v1.",
            Self::Library => "Dusty bookshelves hold forgotten knowledge.",
            Self::CalligraphyHall => "Practice halls echo with the scratch of brush on paper.",
            Self::ScholarStudy => "A scholar's collection of hanzi reference texts.",
            Self::OracleRoom => "The oracle's eye reveals all that is hidden on this floor.",
            Self::InscriptionWall => "Ancient inscriptions teach a forgotten radical.",
            Self::MasterDojo => "A training hall. Push yourself to gain +1 base damage.",
            Self::MeditationChamber => "A place of deep stillness. Spirit energy gathers here.",
            Self::TranslationChallenge => "Stone tablets await correct pinyin translations.",
            Self::AncientScroll => "A weathered scroll contains a rare spell formula.",
            Self::WisdomWell => "A well of wisdom. Answer its riddle for a reward.",
            Self::FloodedChamber => "Water has flooded this chamber. Currents pull at you.",
            Self::FrozenCavern => "Ice coats every surface. Movement is treacherous.",
            Self::LavaCrossing => "Molten lava flows between narrow stone paths.",
            Self::BambooForest => "Dense bamboo thickets obscure your vision.",
            Self::WindTunnel => "A howling gale pushes through this narrow passage.",
            Self::CrystalCave => "Luminous crystals line the walls, refracting light.",
            Self::MushroomGrotto => "Giant mushrooms release clouds of disorienting spores.",
            Self::UndergroundRiver => "An underground river rushes through the chamber.",
            Self::EchoingCavern => "Every sound reverberates endlessly in this cavern.",
            Self::ShadowRealm => "Impenetrable darkness limits your sight to 2 tiles.",
            Self::WanderingMerchant => "A traveling merchant offers rare and unique wares.",
            Self::HermitSage => "A wise hermit offers to trade HP for spell power.",
            Self::PrisonerCell => "Someone is locked in a cell! Free them for an ally.",
            Self::SpiritShrine => "A sacred shrine radiates divine energy.",
            Self::TeaHouse => "A cozy rest stop. Enjoy tea and hear rumors.",
            Self::SmithyWorkshop => "A master smith can upgrade one piece of equipment.",
            Self::AlchemyLab => "Bubbling cauldrons and strange reagents fill the room.",
            Self::FortuneTeller => "A mysterious seer offers to reveal the boss's weakness.",
            Self::RefugeeCamp => "Displaced travelers seek aid. Help them for karma.",
            Self::DragonGate => "An otherworldly portal shimmers with draconic power.",
            Self::GamblingDen => "Three mysterious urns await. Pick one — fortune or ruin?",
            Self::BloodAltar => "A crimson altar pulses with dark energy. Sacrifice HP for power.",
            Self::CursedTreasure => "An ornate chest radiates ominous energy. Great reward… or great cost.",
            Self::SoulForge => "Ethereal flames can transform one radical into another.",
            Self::WishingWell => "A shimmering well. Throw gold to receive a gift from the depths.",
            Self::RuneGate => "Four elemental runes must be stepped on in Wuxing cycle order.",
            Self::MirrorMaze => "Reflective surfaces distort space. Navigate carefully to the center.",
            Self::WeightPuzzle => "Pressure plates and boulders. Push all boulders onto plates to unlock.",
            Self::ToneStaircase => "Four ascending steps glow with tonal marks. Ascend with correct tones.",
            Self::ElementalLock => "A sealed door requires elemental energy — cast one spell of each element.",
            Self::SurvivalPit => "The exits seal shut! Survive waves of enemies for a reward.",
            Self::TreasureRace => "Gold coins materialize on random tiles! Grab them before they vanish!",
            Self::CollapsingChamber => "The floor crumbles from the edges inward! Reach the center treasure!",
            Self::InkFlood => "Dark ink seeps across the floor. Stand in it for power, but beware the drain.",
            Self::FormShrine => "An ancient shrine offers transformation into a new form. Choose wisely.",
            Self::ClassTrial => "A trial chamber tests your abilities. Succeed for a class bonus.",
            Self::RadicalFountain => "Luminous waters refine all radicals you carry. Spells gain +1 damage.",
            Self::AncestorTomb => "An ancient warrior's tomb. Pray to receive their legendary weapon.",
            Self::ProphecyRoom => "Murals depict the boss of this tier and hint at its weakness.",
            Self::SealedMemory => "Echoes of the past. Recall what you've learned for a reward.",
            Self::DemonSeal => "A sealed demon offers dark power — but at what cost?",
            Self::PhoenixNest => "A radiant nest of eternal flame. The phoenix bestows its blessing.",
            Self::CalligraphyContest => "A master calligrapher challenges you. Speed and accuracy earn gold.",
        }
    }

    /// Weighted random selection based on floor level.
    fn random_for_floor(rng: &mut Rng, floor: i32) -> Self {
        // Build a weighted pool based on floor progression
        let mut pool: Vec<Self> = Vec::new();

        // Always available
        pool.extend_from_slice(&[
            Self::TreasureVault, Self::GoldMine, Self::AncestralHall,
            Self::LotusGarden, Self::DragonHoard, Self::RelicChamber,
            Self::SpiritSpring, Self::ArenaChallenge, Self::EliteGuardPost,
            Self::TrapGauntlet, Self::BanditAmbush,
            Self::Library, Self::CalligraphyHall, Self::ScholarStudy,
            Self::InscriptionWall, Self::MasterDojo, Self::MeditationChamber,
            Self::TranslationChallenge, Self::WisdomWell,
            Self::FloodedChamber, Self::FrozenCavern, Self::LavaCrossing,
            Self::BambooForest, Self::WindTunnel, Self::CrystalCave,
            Self::MushroomGrotto, Self::EchoingCavern, Self::ShadowRealm,
            Self::WanderingMerchant, Self::TeaHouse, Self::SmithyWorkshop,
            Self::SpiritShrine, Self::RefugeeCamp,
            // New: always available
            Self::GamblingDen, Self::WishingWell, Self::TreasureRace,
            Self::CalligraphyContest, Self::ProphecyRoom,
        ]);

        // Floor 3+: harder challenges and more rewards
        if floor >= 3 {
            pool.extend_from_slice(&[
                Self::HiddenCache, Self::OfferingAltar, Self::JadeVault,
                Self::PoisonMaze, Self::BoulderPuzzle, Self::MirrorRoom,
                Self::GraveyardRise, Self::DuelCircle,
                Self::OracleRoom, Self::AncientScroll,
                Self::UndergroundRiver,
                Self::HermitSage, Self::PrisonerCell, Self::AlchemyLab,
                Self::FortuneTeller,
                // New: floor 3+
                Self::BloodAltar, Self::WeightPuzzle, Self::ToneStaircase,
                Self::SurvivalPit, Self::ClassTrial, Self::SealedMemory,
            ]);
        }

        // Floor 5+: elemental and advanced
        if floor >= 5 {
            pool.extend_from_slice(&[
                Self::ElementalNexus,
                // New: floor 5+
                Self::CursedTreasure, Self::SoulForge, Self::RuneGate,
                Self::MirrorMaze, Self::ElementalLock,
                Self::CollapsingChamber, Self::InkFlood,
                Self::RadicalFountain, Self::AncestorTomb,
            ]);
        }

        // Floor 10+: transformation rooms
        if floor >= 10 {
            pool.extend_from_slice(&[
                Self::FormShrine, Self::DemonSeal,
            ]);
        }

        // Floor 20+: rare endgame rooms
        if floor >= 20 {
            pool.push(Self::PhoenixNest);
        }

        // Floor 25+: endgame content
        if floor >= 25 {
            pool.push(Self::DragonGate);
        }

        let idx = rng.next_u64() as usize % pool.len();
        pool[idx]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    Wall,
    CrackedWall,
    /// A visible weak barrier for optional vault-style puzzle niches
    BrittleWall,
    Floor,
    Corridor,
    StairsDown,
    Forge,
    Shop,
    Chest,
    /// Smashable crate with supplies or traps
    Crate,
    /// NetHack-style spike trap
    Spikes,
    /// Slick oil that can be ignited by fire magic
    Oil,
    /// Water that conducts stunning spells
    Water,
    /// Too deep to wade through, but a crate can span it
    DeepWater,
    /// NPC companion (0=Teacher, 1=Monk, 2=Merchant, 3=Guard)
    Npc(u8),
    /// Tone shrine for tone battle mini-game
    Shrine,
    /// One-shot altar that grants a blessing
    Altar(AltarKind),
    /// One-shot script seal that reshapes the room when stepped on
    Seal(SealKind),
    /// Tutorial signpost with a scripted message
    Sign(u8),
    /// Bridge created by pushing a crate into water
    Bridge,
    /// Stroke order challenge shrine
    StrokeShrine,
    /// Tone defense wall
    ToneWall,
    /// Compound word builder shrine
    CompoundShrine,
    /// Classifier matching shrine
    ClassifierShrine,
    /// Ink well — component count quiz, reward HP
    InkWell,
    /// Ancestor shrine — chengyu/proverb completion
    AncestorShrine,
    /// Translation altar — pick correct Chinese for English
    TranslationAltar,
    /// Radical garden — identify the radical of a hanzi
    RadicalGarden,
    /// Mirror pool — type pinyin for shown hanzi
    MirrorPool,
    /// Stone tutor — teaches then quizzes tone
    StoneTutor,
    /// Codex shrine — quizzes characters from player's codex
    CodexShrine,
    /// Word bridge — answer vocab to extend bridge over deep water
    WordBridge,
    /// Locked door — blocks passage until question answered
    LockedDoor,
    /// Cursed floor — hidden trap, tone quiz when stepped on
    CursedFloor,
    /// Hidden trap tile (type: 0=poison, 1=teleport, 2=alarm)
    Trap(u8),
    /// Gold ore vein — mine for gold
    GoldOre,
    /// Lava — deals damage when walked on
    Lava,
    /// Ice — slippery, sliding movement
    Ice,
    /// Dense bamboo — blocks movement and line of sight
    Bamboo,
    /// Mushroom — spore cloud causes confusion
    Mushroom,
    /// Poison gas — applies poison when walked through
    PoisonGas,
    /// Bookshelf — interactable, teaches knowledge
    Bookshelf,
    /// Pressure plate — puzzle element
    PressurePlate,
    /// Boulder — pushable onto pressure plates
    Boulder,
    /// Crystal — reflective wall
    Crystal,
    /// Dragon gate portal — end-game content
    DragonGatePortal,
    /// Spirit spring — restores HP and spirit
    SpiritSpringTile,
    /// Gold pile — pick up for gold
    GoldPile,
}

impl Tile {
    pub fn is_walkable(self) -> bool {
        matches!(
            self,
            Tile::Floor
                | Tile::Corridor
                | Tile::StairsDown
                | Tile::Forge
                | Tile::Shop
                | Tile::Chest
                | Tile::Spikes
                | Tile::Oil
                | Tile::Water
                | Tile::Npc(_)
                | Tile::Shrine
                | Tile::Altar(_)
                | Tile::Seal(_)
                | Tile::Sign(_)
                | Tile::Bridge
                | Tile::StrokeShrine
                | Tile::ToneWall
                | Tile::CompoundShrine
                | Tile::ClassifierShrine
                | Tile::InkWell
                | Tile::AncestorShrine
                | Tile::TranslationAltar
                | Tile::RadicalGarden
                | Tile::MirrorPool
                | Tile::StoneTutor
                | Tile::CodexShrine
                | Tile::WordBridge
                | Tile::CursedFloor
                | Tile::Trap(_)
                | Tile::Lava
                | Tile::Ice
                | Tile::Mushroom
                | Tile::PoisonGas
                | Tile::Bookshelf
                | Tile::PressurePlate
                | Tile::DragonGatePortal
                | Tile::SpiritSpringTile
                | Tile::GoldPile
        )
    }
}

// ── Room descriptor ─────────────────────────────────────────────────────────

/// Room environment modifier affecting gameplay.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RoomModifier {
    /// Reduced FOV to 2 tiles
    Dark,
    /// Spells deal 2x damage
    Arcane,
    /// Enemies take 1 extra damage per hit
    Cursed,
    /// Overgrown bamboo and natural terrain
    Garden,
    /// Icy and frozen terrain
    Frozen,
    /// Lava and fire hazards
    Infernal,
}

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

// ── BSP node ────────────────────────────────────────────────────────────────

struct BspNode {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    left: Option<Box<BspNode>>,
    right: Option<Box<BspNode>>,
    room: Option<Room>,
}

const MIN_LEAF: i32 = 7;
const MIN_ROOM: i32 = 4;

impl BspNode {
    fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            x,
            y,
            w,
            h,
            left: None,
            right: None,
            room: None,
        }
    }

    fn split(&mut self, rng: &mut Rng) -> bool {
        if self.left.is_some() {
            return false; // already split
        }
        // Decide split direction: prefer splitting the longer axis
        let split_h = if self.w > self.h && (self.w as f64 / self.h as f64) >= 1.25 {
            false // split vertically
        } else if self.h > self.w && (self.h as f64 / self.w as f64) >= 1.25 {
            true // split horizontally
        } else {
            rng.next_u64() % 2 == 0
        };

        let max = if split_h { self.h } else { self.w } - MIN_LEAF;
        if max < MIN_LEAF {
            return false; // too small
        }

        let split = rng.range(MIN_LEAF, max + 1);

        if split_h {
            self.left = Some(Box::new(BspNode::new(self.x, self.y, self.w, split)));
            self.right = Some(Box::new(BspNode::new(
                self.x,
                self.y + split,
                self.w,
                self.h - split,
            )));
        } else {
            self.left = Some(Box::new(BspNode::new(self.x, self.y, split, self.h)));
            self.right = Some(Box::new(BspNode::new(
                self.x + split,
                self.y,
                self.w - split,
                self.h,
            )));
        }
        true
    }

    fn create_rooms(&mut self, rng: &mut Rng) {
        if let (Some(ref mut l), Some(ref mut r)) = (&mut self.left, &mut self.right) {
            l.create_rooms(rng);
            r.create_rooms(rng);
        } else {
            // Leaf node — place a room
            let w = rng.range(MIN_ROOM, self.w - 1);
            let h = rng.range(MIN_ROOM, self.h - 1);
            let x = self.x + rng.range(1, self.w - w);
            let y = self.y + rng.range(1, self.h - h);
            self.room = Some(Room {
                x,
                y,
                w,
                h,
                modifier: None,
                special: None,
            });
        }
    }

    fn get_room(&self) -> Option<&Room> {
        if self.room.is_some() {
            return self.room.as_ref();
        }
        // Search children for any room (pick left-first)
        if let Some(ref l) = self.left {
            if let Some(r) = l.get_room() {
                return Some(r);
            }
        }
        if let Some(ref r) = self.right {
            return r.get_room();
        }
        None
    }

    fn collect_corridors(&self, corridors: &mut Vec<((i32, i32), (i32, i32))>) {
        if let (Some(ref l), Some(ref r)) = (&self.left, &self.right) {
            l.collect_corridors(corridors);
            r.collect_corridors(corridors);
            // Connect a room from each child
            if let (Some(lr), Some(rr)) = (l.get_room(), r.get_room()) {
                corridors.push((lr.center(), rr.center()));
            }
        }
    }
}

// ── DungeonLevel ────────────────────────────────────────────────────────────

pub struct DungeonLevel {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Tile>,
    pub rooms: Vec<Room>,
    pub visible: Vec<bool>,
    pub revealed: Vec<bool>,
}

impl DungeonLevel {
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
            Tile::Wall
        }
    }

    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && self.tiles[self.idx(x, y)].is_walkable()
    }

    fn area_is_solid_wall(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        if x < 1 || y < 1 || x + w >= self.width - 1 || y + h >= self.height - 1 {
            return false;
        }

        for ty in y..y + h {
            for tx in x..x + w {
                if self.tile(tx, ty) != Tile::Wall {
                    return false;
                }
            }
        }

        true
    }

    fn carve_rect(&mut self, x: i32, y: i32, w: i32, h: i32, tile: Tile) {
        for ty in y..y + h {
            for tx in x..x + w {
                let idx = self.idx(tx, ty);
                self.tiles[idx] = tile;
            }
        }
    }

    fn place_secret_room_feature(
        &mut self,
        room_x: i32,
        room_y: i32,
        room_w: i32,
        room_h: i32,
        rng: &mut Rng,
    ) {
        let cx = room_x + room_w / 2;
        let cy = room_y + room_h / 2;

        match rng.next_u64() % 4 {
            0 => {
                for &(dx, dy) in &[(0, 0), (-1, 0), (1, 0)] {
                    let tx = cx + dx;
                    let ty = cy + dy;
                    if self.in_bounds(tx, ty) && self.tile(tx, ty) == Tile::Floor {
                        let idx = self.idx(tx, ty);
                        self.tiles[idx] = Tile::Chest;
                    }
                }
            }
            1 => {
                let idx = self.idx(cx, cy);
                self.tiles[idx] = Tile::Altar(AltarKind::random(rng));
            }
            2 => {
                let idx = self.idx(cx, cy);
                self.tiles[idx] = Tile::Shrine;
            }
            _ => {
                let idx = self.idx(cx, cy);
                self.tiles[idx] = Tile::Forge;
            }
        }
    }

    fn try_place_secret_room_candidate(
        &mut self,
        secret_x: i32,
        secret_y: i32,
        secret_w: i32,
        secret_h: i32,
        door_x: i32,
        door_y: i32,
        rng: &mut Rng,
    ) -> bool {
        if secret_x < 1
            || secret_y < 1
            || secret_x + secret_w >= self.width - 1
            || secret_y + secret_h >= self.height - 1
        {
            return false;
        }

        if !self.area_is_solid_wall(secret_x - 1, secret_y - 1, secret_w + 2, secret_h + 2) {
            return false;
        }

        self.carve_rect(secret_x, secret_y, secret_w, secret_h, Tile::Floor);
        let door_idx = self.idx(door_x, door_y);
        self.tiles[door_idx] = Tile::CrackedWall;
        self.place_secret_room_feature(secret_x, secret_y, secret_w, secret_h, rng);
        true
    }

    fn place_secret_room(&mut self, rng: &mut Rng) {
        if self.rooms.len() < 4 {
            return;
        }

        for _ in 0..24 {
            let room_idx = 1 + (rng.next_u64() as usize % (self.rooms.len() - 2));
            let room = self.rooms[room_idx].clone();
            let start_dir = (rng.next_u64() % 4) as usize;

            for dir_offset in 0..4 {
                let secret_w = 4 + (rng.next_u64() % 3) as i32;
                let secret_h = 4 + (rng.next_u64() % 3) as i32;
                let max_x = (self.width - secret_w - 1).max(1);
                let max_y = (self.height - secret_h - 1).max(1);
                let dir = (start_dir + dir_offset) % 4;

                let candidate = match dir {
                    0 => {
                        let door_x = rng.range(room.x + 1, room.x + room.w - 1);
                        let door_y = room.y - 1;
                        let secret_x = (door_x - secret_w / 2).clamp(1, max_x);
                        let secret_y = door_y - secret_h;
                        if secret_y < 1 || door_x < secret_x || door_x >= secret_x + secret_w {
                            None
                        } else {
                            Some((secret_x, secret_y, door_x, door_y))
                        }
                    }
                    1 => {
                        let door_x = rng.range(room.x + 1, room.x + room.w - 1);
                        let door_y = room.y + room.h;
                        let secret_x = (door_x - secret_w / 2).clamp(1, max_x);
                        let secret_y = door_y + 1;
                        if secret_y + secret_h >= self.height - 1
                            || door_x < secret_x
                            || door_x >= secret_x + secret_w
                        {
                            None
                        } else {
                            Some((secret_x, secret_y, door_x, door_y))
                        }
                    }
                    2 => {
                        let door_x = room.x - 1;
                        let door_y = rng.range(room.y + 1, room.y + room.h - 1);
                        let secret_x = door_x - secret_w;
                        let secret_y = (door_y - secret_h / 2).clamp(1, max_y);
                        if secret_x < 1 || door_y < secret_y || door_y >= secret_y + secret_h {
                            None
                        } else {
                            Some((secret_x, secret_y, door_x, door_y))
                        }
                    }
                    _ => {
                        let door_x = room.x + room.w;
                        let door_y = rng.range(room.y + 1, room.y + room.h - 1);
                        let secret_x = door_x + 1;
                        let secret_y = (door_y - secret_h / 2).clamp(1, max_y);
                        if secret_x + secret_w >= self.width - 1
                            || door_y < secret_y
                            || door_y >= secret_y + secret_h
                        {
                            None
                        } else {
                            Some((secret_x, secret_y, door_x, door_y))
                        }
                    }
                };

                let Some((secret_x, secret_y, door_x, door_y)) = candidate else {
                    continue;
                };

                if self.try_place_secret_room_candidate(
                    secret_x, secret_y, secret_w, secret_h, door_x, door_y, rng,
                ) {
                    return;
                }
            }
        }

        for room_idx in 1..self.rooms.len() - 1 {
            let room = self.rooms[room_idx].clone();
            for &(secret_w, secret_h) in &[(4, 4), (5, 4), (4, 5), (5, 5)] {
                for door_x in room.x + 1..room.x + room.w - 1 {
                    let secret_x = door_x - secret_w / 2;
                    let top_y = room.y - 1 - secret_h;
                    if door_x >= secret_x
                        && door_x < secret_x + secret_w
                        && self.try_place_secret_room_candidate(
                            secret_x,
                            top_y,
                            secret_w,
                            secret_h,
                            door_x,
                            room.y - 1,
                            rng,
                        )
                    {
                        return;
                    }

                    let bottom_y = room.y + room.h + 1;
                    if door_x >= secret_x
                        && door_x < secret_x + secret_w
                        && self.try_place_secret_room_candidate(
                            secret_x,
                            bottom_y,
                            secret_w,
                            secret_h,
                            door_x,
                            room.y + room.h,
                            rng,
                        )
                    {
                        return;
                    }
                }

                for door_y in room.y + 1..room.y + room.h - 1 {
                    let secret_y = door_y - secret_h / 2;
                    let left_x = room.x - 1 - secret_w;
                    if door_y >= secret_y
                        && door_y < secret_y + secret_h
                        && self.try_place_secret_room_candidate(
                            left_x,
                            secret_y,
                            secret_w,
                            secret_h,
                            room.x - 1,
                            door_y,
                            rng,
                        )
                    {
                        return;
                    }

                    let right_x = room.x + room.w + 1;
                    if door_y >= secret_y
                        && door_y < secret_y + secret_h
                        && self.try_place_secret_room_candidate(
                            right_x,
                            secret_y,
                            secret_w,
                            secret_h,
                            room.x + room.w,
                            door_y,
                            rng,
                        )
                    {
                        return;
                    }
                }
            }
        }
    }

    fn place_bridge_setup_in_room(&mut self, room: &Room, rng: &mut Rng) -> bool {
        let mut water_tiles = Vec::new();
        for y in room.y + 1..room.y + room.h - 1 {
            for x in room.x + 1..room.x + room.w - 1 {
                if self.tile(x, y) == Tile::Water {
                    water_tiles.push((x, y));
                }
            }
        }

        if water_tiles.is_empty() {
            return false;
        }

        let water_start = (rng.next_u64() as usize) % water_tiles.len();
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for water_offset in 0..water_tiles.len() {
            let (wx, wy) = water_tiles[(water_start + water_offset) % water_tiles.len()];
            let dir_start = (rng.next_u64() % 4) as usize;
            for dir_offset in 0..4 {
                let (dx, dy) = dirs[(dir_start + dir_offset) % 4];
                let crate_x = wx - dx;
                let crate_y = wy - dy;
                let stand_x = crate_x - dx;
                let stand_y = crate_y - dy;

                if !self.in_bounds(crate_x, crate_y) || !self.in_bounds(stand_x, stand_y) {
                    continue;
                }

                if self.tile(crate_x, crate_y) != Tile::Floor {
                    continue;
                }

                let stand_tile = self.tile(stand_x, stand_y);
                if !stand_tile.is_walkable() || stand_tile == Tile::Water {
                    continue;
                }

                let idx = self.idx(crate_x, crate_y);
                self.tiles[idx] = Tile::Crate;
                return true;
            }
        }

        false
    }

    fn place_bridge_setup(&mut self, rng: &mut Rng) -> bool {
        if self.rooms.len() < 4 {
            return false;
        }

        let room_count = self.rooms.len().saturating_sub(2);
        if room_count == 0 {
            return false;
        }

        let start = (rng.next_u64() as usize) % room_count;
        for offset in 0..room_count {
            let room_idx = 1 + (start + offset) % room_count;
            let room = self.rooms[room_idx].clone();
            if self.place_bridge_setup_in_room(&room, rng) {
                return true;
            }
        }

        false
    }

    fn room_is_plain(&self, room: &Room) -> bool {
        for y in room.y..room.y + room.h {
            for x in room.x..room.x + room.w {
                if !self.in_bounds(x, y) {
                    return false;
                }
                if !matches!(self.tile(x, y), Tile::Floor | Tile::Corridor) {
                    return false;
                }
            }
        }
        true
    }

    fn try_place_puzzle_niche(
        &mut self,
        barrier_x: i32,
        barrier_y: i32,
        dx: i32,
        dy: i32,
        barrier_tile: Tile,
        with_crate: bool,
    ) -> bool {
        let reward_x = barrier_x + dx;
        let reward_y = barrier_y + dy;
        let back_x = reward_x + dx;
        let back_y = reward_y + dy;
        let approach_x = barrier_x - dx;
        let approach_y = barrier_y - dy;
        let stand_x = approach_x - dx;
        let stand_y = approach_y - dy;
        let side_a = (-dy, dx);
        let side_b = (dy, -dx);

        let mut floor_positions = vec![
            (barrier_x, barrier_y),
            (reward_x, reward_y),
            (back_x, back_y),
            (approach_x, approach_y),
            (barrier_x + side_a.0, barrier_y + side_a.1),
            (reward_x + side_a.0, reward_y + side_a.1),
            (back_x + side_a.0, back_y + side_a.1),
            (barrier_x + side_b.0, barrier_y + side_b.1),
            (reward_x + side_b.0, reward_y + side_b.1),
            (back_x + side_b.0, back_y + side_b.1),
        ];
        if with_crate {
            floor_positions.push((stand_x, stand_y));
        }

        if floor_positions
            .iter()
            .any(|&(x, y)| !self.in_bounds(x, y) || self.tile(x, y) != Tile::Floor)
        {
            return false;
        }

        let wall_positions = [
            (back_x, back_y),
            (barrier_x + side_a.0, barrier_y + side_a.1),
            (reward_x + side_a.0, reward_y + side_a.1),
            (back_x + side_a.0, back_y + side_a.1),
            (barrier_x + side_b.0, barrier_y + side_b.1),
            (reward_x + side_b.0, reward_y + side_b.1),
            (back_x + side_b.0, back_y + side_b.1),
        ];

        for (x, y) in wall_positions {
            let idx = self.idx(x, y);
            self.tiles[idx] = Tile::Wall;
        }

        let barrier_idx = self.idx(barrier_x, barrier_y);
        self.tiles[barrier_idx] = barrier_tile;

        let reward_idx = self.idx(reward_x, reward_y);
        self.tiles[reward_idx] = Tile::Chest;

        if with_crate {
            let crate_idx = self.idx(approach_x, approach_y);
            self.tiles[crate_idx] = Tile::Crate;
        }

        true
    }

    fn try_place_brittle_vault(&mut self, room: &Room, rng: &mut Rng) -> bool {
        let mut candidates = Vec::new();
        if room.w >= 6 && room.h >= 5 {
            let y = rng.range(room.y + 2, room.y + room.h - 2);
            candidates.push((room.x + room.w - 4, y, 1, 0));
            candidates.push((room.x + 3, y, -1, 0));
        }
        if room.h >= 6 && room.w >= 5 {
            let x = rng.range(room.x + 2, room.x + room.w - 2);
            candidates.push((x, room.y + room.h - 4, 0, 1));
            candidates.push((x, room.y + 3, 0, -1));
        }

        if candidates.is_empty() {
            return false;
        }

        let start = (rng.next_u64() as usize) % candidates.len();
        for offset in 0..candidates.len() {
            let (x, y, dx, dy) = candidates[(start + offset) % candidates.len()];
            if self.try_place_puzzle_niche(x, y, dx, dy, Tile::BrittleWall, false) {
                return true;
            }
        }

        false
    }

    fn try_place_deep_water_cache(&mut self, room: &Room, rng: &mut Rng) -> bool {
        let mut candidates = Vec::new();
        if room.w >= 7 && room.h >= 5 {
            let y = rng.range(room.y + 2, room.y + room.h - 2);
            candidates.push((room.x + room.w - 4, y, 1, 0));
            candidates.push((room.x + 3, y, -1, 0));
        }
        if room.h >= 7 && room.w >= 5 {
            let x = rng.range(room.x + 2, room.x + room.w - 2);
            candidates.push((x, room.y + room.h - 4, 0, 1));
            candidates.push((x, room.y + 3, 0, -1));
        }

        if candidates.is_empty() {
            return false;
        }

        let start = (rng.next_u64() as usize) % candidates.len();
        for offset in 0..candidates.len() {
            let (x, y, dx, dy) = candidates[(start + offset) % candidates.len()];
            if self.try_place_puzzle_niche(x, y, dx, dy, Tile::DeepWater, true) {
                return true;
            }
        }

        false
    }

    /// Spike bridge: 3-wide spike corridor with a chest on the far side.
    fn try_place_spike_bridge(&mut self, room: &Room, rng: &mut Rng) -> bool {
        if room.w < 7 || room.h < 5 {
            return false;
        }
        let cy = rng.range(room.y + 1, room.y + room.h - 1);
        let sx = room.x + 2;
        for dx in 0..3 {
            let x = sx + dx;
            if !self.in_bounds(x, cy) || self.tile(x, cy) != Tile::Floor {
                return false;
            }
        }
        let reward_x = sx + 3;
        if !self.in_bounds(reward_x, cy) || self.tile(reward_x, cy) != Tile::Floor {
            return false;
        }
        for dx in 0..3 {
            let idx = self.idx(sx + dx, cy);
            self.tiles[idx] = Tile::Spikes;
        }
        let idx = self.idx(reward_x, cy);
        self.tiles[idx] = Tile::Chest;
        true
    }

    /// Oil-fire trap: oil slick leading to a chest, ignitable by fire spell.
    fn try_place_oil_fire_trap(&mut self, room: &Room, rng: &mut Rng) -> bool {
        if room.w < 6 || room.h < 5 {
            return false;
        }
        let cy = rng.range(room.y + 1, room.y + room.h - 1);
        let sx = room.x + 2;
        for dx in 0..3 {
            let x = sx + dx;
            if !self.in_bounds(x, cy) || self.tile(x, cy) != Tile::Floor {
                return false;
            }
        }
        for dx in 0..2 {
            let idx = self.idx(sx + dx, cy);
            self.tiles[idx] = Tile::Oil;
        }
        let idx = self.idx(sx + 2, cy);
        self.tiles[idx] = Tile::Chest;
        true
    }

    /// Seal chain: two seals placed near each other for cascading reshaping.
    fn try_place_seal_chain(&mut self, room: &Room, rng: &mut Rng) -> bool {
        if room.w < 6 || room.h < 5 {
            return false;
        }
        let cx = rng.range(room.x + 1, room.x + room.w - 3);
        let cy = rng.range(room.y + 1, room.y + room.h - 1);
        if !self.in_bounds(cx, cy) || self.tile(cx, cy) != Tile::Floor {
            return false;
        }
        if !self.in_bounds(cx + 2, cy) || self.tile(cx + 2, cy) != Tile::Floor {
            return false;
        }
        let kind_a = SealKind::random(&mut Rng::new(rng.next_u64()));
        let kind_b = SealKind::random(&mut Rng::new(rng.next_u64()));
        let idx_a = self.idx(cx, cy);
        self.tiles[idx_a] = Tile::Seal(kind_a);
        let idx_b = self.idx(cx + 2, cy);
        self.tiles[idx_b] = Tile::Seal(kind_b);
        true
    }

    fn place_puzzle_room_in_room(&mut self, room: &Room, rng: &mut Rng) -> bool {
        let variant_count = 5;
        let start = (rng.next_u64() % variant_count) as usize;
        for offset in 0..variant_count as usize {
            let placed = match (start + offset) % variant_count as usize {
                0 => self.try_place_brittle_vault(room, rng),
                1 => self.try_place_deep_water_cache(room, rng),
                2 => self.try_place_spike_bridge(room, rng),
                3 => self.try_place_oil_fire_trap(room, rng),
                _ => self.try_place_seal_chain(room, rng),
            };
            if placed {
                return true;
            }
        }

        false
    }

    pub fn place_puzzle_rooms(&mut self, rng: &mut Rng) {
        if self.rooms.len() < 4 {
            return;
        }

        let room_count = self.rooms.len().saturating_sub(2);
        if room_count == 0 {
            return;
        }

        let desired = if room_count >= 4 && rng.next_u64() % 100 < 50 {
            2
        } else {
            1
        };
        let start = (rng.next_u64() as usize) % room_count;
        let mut placed = 0;
        for offset in 0..room_count {
            let room_idx = 1 + (start + offset) % room_count;
            let room = self.rooms[room_idx].clone();
            if !self.room_is_plain(&room) {
                continue;
            }
            if self.place_puzzle_room_in_room(&room, rng) {
                placed += 1;
                if placed >= desired {
                    break;
                }
            }
        }
    }

    /// Place stairs down in the last room.
    pub fn place_stairs(&mut self) {
        if let Some(room) = self.rooms.last() {
            let (cx, cy) = room.center();
            let idx = self.idx(cx, cy);
            self.tiles[idx] = Tile::StairsDown;
        }
    }

    /// Place forge workbenches in 1-2 middle rooms.
    pub fn place_forges(&mut self, rng: &mut Rng) {
        if self.rooms.len() < 3 {
            return;
        }
        // Pick 1-2 rooms (not first or last)
        let candidates: Vec<usize> = (1..self.rooms.len() - 1).collect();
        let count = if candidates.len() >= 3 { 2 } else { 1 };
        let mut placed = 0;
        let mut used = Vec::new();
        while placed < count {
            let pick = rng.range(0, candidates.len() as i32) as usize;
            if used.contains(&pick) {
                // Avoid infinite loop if few candidates
                if used.len() >= candidates.len() {
                    break;
                }
                continue;
            }
            used.push(pick);
            let room = &self.rooms[candidates[pick]];
            // Place forge at an offset from center so it doesn't overlap stairs
            let fx = room.x + 1;
            let fy = room.y + 1;
            if self.in_bounds(fx, fy) {
                let idx = self.idx(fx, fy);
                if self.tiles[idx] == Tile::Floor {
                    self.tiles[idx] = Tile::Forge;
                    placed += 1;
                }
            }
        }
    }

    /// Place a shop in one middle room (if enough rooms).
    pub fn place_shop(&mut self, rng: &mut Rng) {
        if self.rooms.len() < 4 {
            return;
        }
        // Pick a room that isn't first, last, or already has a forge
        for _ in 0..10 {
            let pick = rng.range(1, self.rooms.len() as i32 - 1) as usize;
            let room = &self.rooms[pick];
            let fx = room.x + room.w - 2;
            let fy = room.y + 1;
            if self.in_bounds(fx, fy) {
                let idx = self.idx(fx, fy);
                if self.tiles[idx] == Tile::Floor {
                    self.tiles[idx] = Tile::Shop;
                    return;
                }
            }
        }
    }

    /// Place treasure chests in one room (2-3 chests).
    pub fn place_chests(&mut self, rng: &mut Rng) {
        if self.rooms.len() < 5 {
            return;
        }
        // Pick a middle room that doesn't have forge/shop/stairs
        for _ in 0..20 {
            let pick = rng.range(1, self.rooms.len() as i32 - 1) as usize;
            let room = &self.rooms[pick];
            // Check room doesn't already have special tiles
            let has_special = (room.y..room.y + room.h).any(|ry| {
                (room.x..room.x + room.w).any(|rx| {
                    if rx >= 0 && ry >= 0 && rx < self.width && ry < self.height {
                        let idx = (ry * self.width + rx) as usize;
                        matches!(self.tiles[idx], Tile::Forge | Tile::Shop | Tile::StairsDown)
                    } else {
                        false
                    }
                })
            });
            if has_special {
                continue;
            }

            let chest_count = rng.range(2, 4); // 2-3 chests
            let mut placed = 0;
            for _ in 0..10 {
                let cx = rng.range(room.x + 1, room.x + room.w - 1);
                let cy = rng.range(room.y + 1, room.y + room.h - 1);
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Chest;
                        placed += 1;
                        if placed >= chest_count {
                            break;
                        }
                    }
                }
            }
            if placed > 0 {
                return;
            }
        }
    }

    /// Place hazard tiles inspired by classic roguelikes.
    pub fn place_hazards(&mut self, rng: &mut Rng) {
        if self.rooms.len() < 4 {
            return;
        }

        let hazard_rooms = 1 + (rng.next_u64() % 2) as usize;
        let mut guaranteed_water = false;
        for _ in 0..hazard_rooms {
            for _ in 0..20 {
                let pick = rng.range(1, self.rooms.len() as i32 - 1) as usize;
                let room = &self.rooms[pick];
                let has_special = (room.y..room.y + room.h).any(|ry| {
                    (room.x..room.x + room.w).any(|rx| {
                        if rx >= 0 && ry >= 0 && rx < self.width && ry < self.height {
                            let idx = (ry * self.width + rx) as usize;
                            matches!(
                                self.tiles[idx],
                                Tile::Forge
                                    | Tile::Shop
                                    | Tile::StairsDown
                                    | Tile::Chest
                                    | Tile::Npc(_)
                                    | Tile::Shrine
                                    | Tile::Altar(_)
                                    | Tile::Seal(_)
                                    | Tile::Sign(_)
                            )
                        } else {
                            false
                        }
                    })
                });
                if has_special {
                    continue;
                }

                let hazard_count = rng.range(2, 5);
                let mut placed = 0;
                for _ in 0..16 {
                    let hx = rng.range(room.x + 1, room.x + room.w - 1);
                    let hy = rng.range(room.y + 1, room.y + room.h - 1);
                    if !self.in_bounds(hx, hy) {
                        continue;
                    }
                    let idx = self.idx(hx, hy);
                    if self.tiles[idx] != Tile::Floor {
                        continue;
                    }
                    self.tiles[idx] = if !guaranteed_water {
                        guaranteed_water = true;
                        Tile::Water
                    } else {
                        match rng.next_u64() % 3 {
                            0 => Tile::Spikes,
                            1 => Tile::Oil,
                            _ => Tile::Water,
                        }
                    };
                    placed += 1;
                    if placed >= hazard_count {
                        break;
                    }
                }
                if placed > 0 {
                    break;
                }
            }
        }
    }

    /// Place smashable crates with supplies or trap gas.
    pub fn place_crates(&mut self, rng: &mut Rng) {
        if self.rooms.len() < 4 {
            return;
        }

        let _ = self.place_bridge_setup(rng);
        let crate_rooms = 1 + (rng.next_u64() % 2) as usize;
        for _ in 0..crate_rooms {
            for _ in 0..20 {
                let pick = rng.range(1, self.rooms.len() as i32 - 1) as usize;
                let room = &self.rooms[pick];
                let has_special = (room.y..room.y + room.h).any(|ry| {
                    (room.x..room.x + room.w).any(|rx| {
                        if rx >= 0 && ry >= 0 && rx < self.width && ry < self.height {
                            let idx = (ry * self.width + rx) as usize;
                            matches!(
                                self.tiles[idx],
                                Tile::Forge
                                    | Tile::Shop
                                    | Tile::StairsDown
                                    | Tile::Chest
                                    | Tile::Npc(_)
                                    | Tile::Shrine
                                    | Tile::Altar(_)
                                    | Tile::Seal(_)
                                    | Tile::Sign(_)
                            )
                        } else {
                            false
                        }
                    })
                });
                if has_special {
                    continue;
                }

                let crate_count = rng.range(1, 3);
                let mut placed = 0;
                for _ in 0..12 {
                    let cx = rng.range(room.x + 1, room.x + room.w - 1);
                    let cy = rng.range(room.y + 1, room.y + room.h - 1);
                    if !self.in_bounds(cx, cy) {
                        continue;
                    }
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] != Tile::Floor {
                        continue;
                    }
                    self.tiles[idx] = Tile::Crate;
                    placed += 1;
                    if placed >= crate_count {
                        break;
                    }
                }
                if placed > 0 {
                    break;
                }
            }
        }
    }

    /// Get player start position (center of first room).
    pub fn start_pos(&self) -> (i32, i32) {
        self.rooms.first().map(|r| r.center()).unwrap_or((1, 1))
    }

    /// Scripted tutorial floor used on the first run.
    pub fn tutorial(width: i32, height: i32) -> Self {
        debug_assert!(
            width >= 44 && height >= 30,
            "tutorial floor expects the default map size"
        );

        let size = (width * height) as usize;
        let mut level = DungeonLevel {
            width,
            height,
            tiles: vec![Tile::Wall; size],
            rooms: vec![
                Room {
                    x: 4,
                    y: 18,
                    w: 9,
                    h: 9,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 16,
                    y: 18,
                    w: 9,
                    h: 9,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 29,
                    y: 18,
                    w: 13,
                    h: 9,
                    modifier: None,
                    special: None,
                },
            ],
            visible: vec![false; size],
            revealed: vec![false; size],
        };

        fn carve_room(level: &mut DungeonLevel, room: &Room) {
            for ry in room.y..room.y + room.h {
                for rx in room.x..room.x + room.w {
                    let idx = level.idx(rx, ry);
                    level.tiles[idx] = Tile::Floor;
                }
            }
        }

        fn carve_h_corridor(level: &mut DungeonLevel, x1: i32, x2: i32, y: i32) {
            for x in x1.min(x2)..=x1.max(x2) {
                let idx = level.idx(x, y);
                if level.tiles[idx] == Tile::Wall {
                    level.tiles[idx] = Tile::Corridor;
                }
            }
        }

        let rooms = level.rooms.clone();
        for room in &rooms {
            carve_room(&mut level, room);
        }

        carve_h_corridor(&mut level, 13, 15, 22);
        carve_h_corridor(&mut level, 25, 28, 22);

        let sign0 = level.idx(8, 20);
        let sign1 = level.idx(13, 22);
        let sign2 = level.idx(30, 20);
        let sign3 = level.idx(38, 20);
        let forge = level.idx(32, 22);
        let stairs = level.idx(39, 22);
        level.tiles[sign0] = Tile::Sign(0);
        level.tiles[sign1] = Tile::Sign(1);
        level.tiles[sign2] = Tile::Sign(2);
        level.tiles[sign3] = Tile::Sign(3);
        level.tiles[forge] = Tile::Forge;
        level.tiles[stairs] = Tile::StairsDown;

        level
    }

    pub fn generate(width: i32, height: i32, seed: u64, floor: i32) -> Self {
        let mut rng = Rng::new(seed);
        let size = (width * height) as usize;
        let mut tiles = vec![Tile::Wall; size];
        let mut root = BspNode::new(0, 0, width, height);

        // Recursive split
        let mut leaves = vec![&mut root as *mut BspNode];
        let mut did_split = true;
        while did_split {
            did_split = false;
            let mut new_leaves = Vec::new();
            for ptr in &leaves {
                let node = unsafe { &mut **ptr };
                if node.left.is_none() && node.split(&mut rng) {
                    new_leaves.push(node.left.as_mut().unwrap().as_mut() as *mut BspNode);
                    new_leaves.push(node.right.as_mut().unwrap().as_mut() as *mut BspNode);
                    did_split = true;
                } else if node.left.is_none() {
                    new_leaves.push(*ptr);
                }
            }
            if did_split {
                leaves = new_leaves;
            }
        }

        // Create rooms in leaves
        root.create_rooms(&mut rng);

        // Collect rooms
        let mut rooms = Vec::new();
        fn collect_rooms(node: &BspNode, out: &mut Vec<Room>) {
            if let Some(ref r) = node.room {
                out.push(r.clone());
            }
            if let Some(ref l) = node.left {
                collect_rooms(l, out);
            }
            if let Some(ref r) = node.right {
                collect_rooms(r, out);
            }
        }
        collect_rooms(&root, &mut rooms);

        // Carve rooms
        for room in &rooms {
            for ry in room.y..room.y + room.h {
                for rx in room.x..room.x + room.w {
                    if rx >= 0 && ry >= 0 && rx < width && ry < height {
                        tiles[(ry * width + rx) as usize] = Tile::Floor;
                    }
                }
            }
        }

        // Collect and carve corridors (L-shaped)
        let mut corridors = Vec::new();
        root.collect_corridors(&mut corridors);
        for ((x1, y1), (x2, y2)) in &corridors {
            // Horizontal then vertical
            let (x1, y1, x2, y2) = (*x1, *y1, *x2, *y2);
            let xmin = x1.min(x2);
            let xmax = x1.max(x2);
            for x in xmin..=xmax {
                if x >= 0 && y1 >= 0 && x < width && y1 < height {
                    let i = (y1 * width + x) as usize;
                    if tiles[i] == Tile::Wall {
                        tiles[i] = Tile::Corridor;
                    }
                }
            }
            let ymin = y1.min(y2);
            let ymax = y1.max(y2);
            for y in ymin..=ymax {
                if x2 >= 0 && y >= 0 && x2 < width && y < height {
                    let i = (y * width + x2) as usize;
                    if tiles[i] == Tile::Wall {
                        tiles[i] = Tile::Corridor;
                    }
                }
            }
        }

        let visible = vec![false; size];
        let revealed = vec![false; size];

        let mut level = DungeonLevel {
            width,
            height,
            tiles,
            rooms,
            visible,
            revealed,
        };
        level.place_special_rooms(&mut rng, floor);
        level.place_stairs();
        level.place_forges(&mut rng);
        level.place_shop(&mut rng);
        level.place_chests(&mut rng);
        level.assign_room_modifiers(&mut rng);
        level.place_npcs(&mut rng);
        level.place_shrines(&mut rng);
        level.place_stroke_shrines(&mut rng);
        level.place_tone_walls(&mut rng);
        level.place_compound_shrines(&mut rng);
        level.place_classifier_shrines(&mut rng);
        level.place_ink_wells(&mut rng);
        level.place_ancestor_shrines(&mut rng);
        level.place_translation_altars(&mut rng);
        level.place_radical_gardens(&mut rng);
        level.place_mirror_pools(&mut rng);
        level.place_stone_tutors(&mut rng);
        level.place_codex_shrines(&mut rng);
        level.place_word_bridges(&mut rng);
        level.place_locked_doors(&mut rng);
        level.place_cursed_floors(&mut rng);
        level.place_altars(&mut rng);
        level.place_seals(&mut rng);
        level.place_hazards(&mut rng);
        level.place_crates(&mut rng);
        level.place_secret_room(&mut rng);
        level.place_puzzle_rooms(&mut rng);

        // Place trap tiles on deeper floors
        if floor >= 2 {
            let trap_count = 2 + floor / 3;
            let mut placed = 0;
            for _ in 0..trap_count * 10 {
                let x = (rng.next_u64() % width as u64) as i32;
                let y = (rng.next_u64() % height as u64) as i32;
                let idx = (y * width + x) as usize;
                if idx < level.tiles.len() && level.tiles[idx] == Tile::Floor {
                    let trap_type = (rng.next_u64() % 3) as u8;
                    level.tiles[idx] = Tile::Trap(trap_type);
                    placed += 1;
                    if placed >= trap_count {
                        break;
                    }
                }
            }
        }

        level
    }

    /// Assign 2-4 rooms per floor as special rooms with unique layouts.
    fn place_special_rooms(&mut self, rng: &mut Rng, floor: i32) {
        let n = self.rooms.len();
        if n <= 4 {
            return;
        }
        // 2-4 special rooms per floor, scaling with room count
        let desired = 2 + (rng.next_u64() % 3.min(((n - 2) / 3) as u64)) as usize;
        let desired = desired.min(n - 2);
        let mut used = Vec::new();
        let mut placed = 0;
        for _ in 0..desired * 8 {
            if placed >= desired {
                break;
            }
            let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
            if used.contains(&room_idx) {
                continue;
            }
            // Skip rooms that already have special content
            let room = &self.rooms[room_idx];
            if room.special.is_some() {
                continue;
            }
            let kind = SpecialRoomKind::random_for_floor(rng, floor);
            let room = self.rooms[room_idx].clone();
            self.generate_special_room(kind, &room, rng);
            self.rooms[room_idx].special = Some(kind);
            used.push(room_idx);
            placed += 1;
        }
    }

    /// Generate tile layout for a special room.
    fn generate_special_room(&mut self, kind: SpecialRoomKind, room: &Room, rng: &mut Rng) {
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2;
        match kind {
            SpecialRoomKind::TreasureVault => {
                // Gold piles scattered around the vault
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 7 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::GoldPile;
                            }
                        }
                    }
                }
                // Water moat around center treasure area
                for &(dx, dy) in &[(-2,-1),(-2,0),(-2,1),(2,-1),(2,0),(2,1),(-1,-2),(0,-2),(1,-2),(-1,2),(0,2),(1,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::GoldPile) {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Crate barriers flanking the vault
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1),(3,-1),(3,0),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::GoldPile) {
                            self.tiles[idx] = Tile::Crate;
                        }
                    }
                }
                // 3 chests in center
                for dx in -1..=1 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        self.tiles[idx] = Tile::Chest;
                    }
                }
                // Locked door at room edge
                if self.in_bounds(room.x, cy) {
                    let idx = self.idx(room.x, cy);
                    if matches!(self.tiles[idx], Tile::Floor | Tile::Corridor) {
                        self.tiles[idx] = Tile::LockedDoor;
                    }
                }
            }
            SpecialRoomKind::GoldMine => {
                // Dense gold ore on walls (higher coverage)
                for x in room.x..room.x + room.w {
                    for y in [room.y, room.y + room.h - 1] {
                        if self.in_bounds(x, y) && rng.next_u64() % 2 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::GoldOre;
                            }
                        }
                    }
                }
                for y in room.y..room.y + room.h {
                    for x in [room.x, room.x + room.w - 1] {
                        if self.in_bounds(x, y) && rng.next_u64() % 2 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::GoldOre;
                            }
                        }
                    }
                }
                // Boulder support pillars
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2),(0,-2),(0,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Boulder;
                        }
                    }
                }
                // Gold piles near veins
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 8 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::GoldPile;
                            }
                        }
                    }
                }
                // Crate (mine cart) at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Crate;
                    }
                }
            }
            SpecialRoomKind::AncestralHall => {
                // Crystal candles along walls
                for y in room.y + 1..room.y + room.h - 1 {
                    if (y - room.y) % 3 == 1 {
                        for &x in &[room.x + 1, room.x + room.w - 2] {
                            if self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::Floor {
                                    self.tiles[idx] = Tile::Crystal;
                                }
                            }
                        }
                    }
                }
                // Bookshelf pew rows on both sides of center aisle
                for y in room.y + 1..room.y + room.h - 1 {
                    if y != cy && (y - room.y) % 2 == 0 {
                        for &x_off in &[2, 3] {
                            let x_left = room.x + x_off;
                            let x_right = room.x + room.w - 1 - x_off;
                            if self.in_bounds(x_left, y) {
                                let idx = self.idx(x_left, y);
                                if self.tiles[idx] == Tile::Floor {
                                    self.tiles[idx] = Tile::Bookshelf;
                                }
                            }
                            if self.in_bounds(x_right, y) {
                                let idx = self.idx(x_right, y);
                                if self.tiles[idx] == Tile::Floor {
                                    self.tiles[idx] = Tile::Bookshelf;
                                }
                            }
                        }
                    }
                }
                // Water basin behind altar
                for dx in -1..=1 {
                    let x = cx + dx;
                    let y = cy - 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Jade altar at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Altar(AltarKind::Jade);
                }
            }
            SpecialRoomKind::HiddenCache => {
                // Oil (dust) and mushroom (cobwebs) in neglected corners
                for &(dx, dy) in &[(1,1),(1,-1),(-1,1),(-1,-1)] {
                    let x = cx + dx * 2;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
                for &(dx, _dy) in &[(1,0),(-1,0),(2,0),(-2,0)] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Oil;
                        }
                    }
                }
                // Crate (old supplies)
                if self.in_bounds(cx + 1, cy - 1) {
                    let idx = self.idx(cx + 1, cy - 1);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Crate;
                    }
                }
                // Chest at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Chest;
                }
                // Cracked wall entrance
                let wall_x = room.x;
                if self.in_bounds(wall_x, cy) {
                    let idx = self.idx(wall_x, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::CrackedWall;
                    }
                }
                // More cracked walls for atmosphere
                if self.in_bounds(wall_x, cy - 1) {
                    let idx = self.idx(wall_x, cy - 1);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::CrackedWall;
                    }
                }
            }
            SpecialRoomKind::OfferingAltar => {
                // Water purification basin
                for dx in -1..=1 {
                    let x = cx + dx;
                    let y = cy + 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Crystal candles at diagonal positions
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Mushroom incense at far positions
                for &(dx, dy) in &[(-2,-1),(2,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
                // Gold pile offerings at cardinal positions
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::GoldPile;
                        }
                    }
                }
                // Central altar
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Altar(AltarKind::Gold);
                }
            }
            SpecialRoomKind::LotusGarden => {
                // Bamboo border along room edges
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) && x != cx {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Bamboo;
                            }
                        }
                    }
                }
                for y in room.y + 2..room.y + room.h - 2 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) && y != cy {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Bamboo;
                            }
                        }
                    }
                }
                // Water pond around center (2-tile radius)
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        if dx == 0 && dy == 0 { continue; }
                        if dx.abs() + dy.abs() > 3 { continue; }
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Water;
                            }
                        }
                    }
                }
                // Mushroom (lotus plants) near water
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
                // Spirit spring at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SpiritSpringTile;
                }
            }
            SpecialRoomKind::DragonHoard => {
                // Gold ore on walls
                for x in room.x..room.x + room.w {
                    for y in [room.y, room.y + room.h - 1] {
                        if self.in_bounds(x, y) && rng.next_u64() % 2 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::GoldOre;
                            }
                        }
                    }
                }
                // Dense gold piles throughout
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 3 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::GoldPile;
                            }
                        }
                    }
                }
                // Crystal gems scattered
                for &(dx, dy) in &[(-2,0),(2,0),(0,-2),(0,2),(-1,-1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Crate treasure boxes at corners
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::GoldPile) {
                            self.tiles[idx] = Tile::Crate;
                        }
                    }
                }
            }
            SpecialRoomKind::JadeVault => {
                // Crystal walls along full inner perimeter
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Crystal;
                            }
                        }
                    }
                }
                for y in room.y + 2..room.y + room.h - 2 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Crystal;
                            }
                        }
                    }
                }
                // Gold piles near chest
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::GoldPile;
                        }
                    }
                }
                // Jade altar behind chest
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    self.tiles[idx] = Tile::Altar(AltarKind::Jade);
                }
                // Chest in center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Chest;
                }
                // Locked door at entry
                if self.in_bounds(room.x, cy) {
                    let idx = self.idx(room.x, cy);
                    if matches!(self.tiles[idx], Tile::Floor | Tile::Corridor | Tile::Crystal) {
                        self.tiles[idx] = Tile::LockedDoor;
                    }
                }
            }
            SpecialRoomKind::RelicChamber => {
                // Bookshelves (lore) on walls
                for y in room.y + 1..room.y + room.h - 1 {
                    if (y - room.y) % 2 == 1 {
                        for &x in &[room.x + 1, room.x + room.w - 2] {
                            if self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::Floor {
                                    self.tiles[idx] = Tile::Bookshelf;
                                }
                            }
                        }
                    }
                }
                // Water purification pool at entry
                for dx in -1..=1 {
                    let x = cx + dx;
                    let y = cy + 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Crystal lighting flanks
                for &(dx, dy) in &[(-2, 0), (2, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Mirror altar backdrop
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    self.tiles[idx] = Tile::Altar(AltarKind::Mirror);
                }
                // Chest (relic) at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Chest;
                }
            }
            SpecialRoomKind::SpiritSpring => {
                // Bamboo accents at room edges
                for &(dx, dy) in &[(-3,-2),(-3,2),(3,-2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bamboo;
                        }
                    }
                }
                // Water ring (2-wide)
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        if dx == 0 && dy == 0 { continue; }
                        let dist = dx.abs().max(dy.abs());
                        if dist >= 1 && dist <= 2 {
                            let x = cx + dx;
                            let y = cy + dy;
                            if self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::Floor {
                                    self.tiles[idx] = Tile::Water;
                                }
                            }
                        }
                    }
                }
                // Crystal accents at compass points
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Mushroom herbs near water
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Water) {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
                // Spirit spring at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SpiritSpringTile;
                }
            }
            SpecialRoomKind::ArenaChallenge => {
                // Oil (sand/blood) scattered in center area
                for y in room.y + 2..room.y + room.h - 2 {
                    for x in room.x + 2..room.x + room.w - 2 {
                        if self.in_bounds(x, y) && rng.next_u64() % 5 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Oil;
                            }
                        }
                    }
                }
                // Spikes inner border
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if matches!(self.tiles[idx], Tile::Floor | Tile::Oil) {
                                self.tiles[idx] = Tile::Spikes;
                            }
                        }
                    }
                }
                for y in room.y + 1..room.y + room.h - 1 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if matches!(self.tiles[idx], Tile::Floor | Tile::Oil) {
                                self.tiles[idx] = Tile::Spikes;
                            }
                        }
                    }
                }
                // Boulder pillars at corners
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Oil) {
                            self.tiles[idx] = Tile::Boulder;
                        }
                    }
                }
                // Pressure plate arena markers
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx * 2;
                    let y = cy + dy * 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Oil) {
                            self.tiles[idx] = Tile::PressurePlate;
                        }
                    }
                }
            }
            SpecialRoomKind::EliteGuardPost => {
                // Spikes (caltrops) near entrance
                for dx in [-2, -1, 1, 2] {
                    let x = cx + dx;
                    let y = cy - 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Spikes;
                        }
                    }
                }
                // Crate barricades on sides
                for &(dx, dy) in &[(-2,-1),(-2,1),(2,-1),(2,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crate;
                        }
                    }
                }
                // Boulder obstacles
                for &(dx, dy) in &[(-3, 0), (3, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Boulder;
                        }
                    }
                }
                // Pressure plates alarm strip
                for x in room.x + 1..room.x + room.w - 1 {
                    if self.in_bounds(x, cy) && x != cx {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::PressurePlate;
                        }
                    }
                }
            }
            SpecialRoomKind::TrapGauntlet => {
                // Oil slicks between spike rows
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x + y) % 3 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Oil;
                            }
                        }
                    }
                }
                // Dense spike checkerboard
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x + y) % 2 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Spikes;
                            }
                        }
                    }
                }
                // Boulder safe spots for cover
                for &(dx, dy) in &[(0,-1),(0,1),(-2,0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::Boulder;
                    }
                }
                // Pressure plate triggers
                for &(dx, dy) in &[(-1,-2),(1,-2),(-1,2),(1,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::PressurePlate;
                    }
                }
                // Chest reward at far end
                let rx = room.x + room.w - 2;
                if self.in_bounds(rx, cy) {
                    let idx = self.idx(rx, cy);
                    self.tiles[idx] = Tile::Chest;
                }
            }
            SpecialRoomKind::PoisonMaze => {
                // Gas walls forming corridors (structured pattern)
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let rel_x = x - room.x;
                            let rel_y = y - room.y;
                            let is_wall = (rel_x % 3 != 0) && (rel_y % 3 != 0);
                            if is_wall {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::Floor {
                                    self.tiles[idx] = Tile::PoisonGas;
                                }
                            }
                        }
                    }
                }
                // Mushroom patches in gas corridors
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 8 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::PoisonGas {
                                self.tiles[idx] = Tile::Mushroom;
                            }
                        }
                    }
                }
                // Water safe zones at corners
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::PoisonGas {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Chest reward at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Chest;
                }
            }
            SpecialRoomKind::BoulderPuzzle => {
                // Pressure plates at puzzle positions
                let plate_positions = [
                    (cx - 1, cy - 1), (cx + 1, cy - 1),
                    (cx - 1, cy + 1), (cx + 1, cy + 1),
                ];
                let boulder_positions = [
                    (cx - 1, cy), (cx + 1, cy),
                    (cx, cy - 1), (cx, cy + 1),
                ];
                for &(px, py) in &plate_positions {
                    if self.in_bounds(px, py) {
                        let idx = self.idx(px, py);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::PressurePlate;
                        }
                    }
                }
                for &(bx, by) in &boulder_positions {
                    if self.in_bounds(bx, by) {
                        let idx = self.idx(bx, by);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Boulder;
                        }
                    }
                }
                // Crystal markers near plates
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Cracked wall hints
                for &x in &[room.x + 1, room.x + room.w - 2] {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::CrackedWall;
                        }
                    }
                }
                // Oil ground texture
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 6 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Oil;
                            }
                        }
                    }
                }
                // Chest reward at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Chest;
                }
            }
            SpecialRoomKind::MirrorRoom => {
                // Ice reflective floor sections
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x + y) % 3 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Ice;
                            }
                        }
                    }
                }
                // Water accents at edges
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Ice) {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Crystal walls forming diamond pattern
                for &(dx, dy) in &[(-2,0),(2,0),(0,-2),(0,2),(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::Crystal;
                    }
                }
                // Mirror pool at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::MirrorPool;
                }
            }
            SpecialRoomKind::ElementalNexus => {
                // Elemental tiles around each altar
                // Water around Gale (west)
                for &(dx, dy) in &[(-3,-1),(-3,1),(-1,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Lava around Iron (east)
                for &(dx, dy) in &[(3,-1),(3,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Lava;
                        }
                    }
                }
                // Ice around Mirror (north)
                for &(dx, dy) in &[(-1,-3),(1,-3),(-1,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Ice;
                        }
                    }
                }
                // Crystal around Gold (south)
                for &(dx, dy) in &[(-1,3),(1,3),(1,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Bamboo around Jade (center)
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bamboo;
                        }
                    }
                }
                // 5 elemental altars in cross pattern
                let positions = [
                    (cx, cy, AltarKind::Jade),
                    (cx - 2, cy, AltarKind::Gale),
                    (cx + 2, cy, AltarKind::Iron),
                    (cx, cy - 2, AltarKind::Mirror),
                    (cx, cy + 2, AltarKind::Gold),
                ];
                for &(x, y, kind) in &positions {
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::Altar(kind);
                    }
                }
            }
            SpecialRoomKind::GraveyardRise => {
                // Grid of spikes (tombstones) in rows
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x - room.x) % 2 == 1 && (y - room.y) % 2 == 1 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Spikes;
                            }
                        }
                    }
                }
                // Oil (disturbed earth) between tombstones
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 5 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Oil;
                            }
                        }
                    }
                }
                // Mushroom (dead flowers) near graves
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2),(0,0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
                // CursedFloor path through cemetery
                for x in room.x + 1..room.x + room.w - 1 {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::CursedFloor;
                        }
                    }
                }
            }
            SpecialRoomKind::BanditAmbush => {
                // Oil (escape routes / slippery floor)
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 6 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Oil;
                            }
                        }
                    }
                }
                // Crate hiding spots forming ambush positions
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2),(-3,0),(3,0),(0,-2),(0,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Oil) {
                            self.tiles[idx] = Tile::Crate;
                        }
                    }
                }
                // Hidden traps near gold
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        let trap_type = (rng.next_u64() % 3) as u8;
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Oil) {
                            self.tiles[idx] = Tile::Trap(trap_type);
                        }
                    }
                }
                // Gold bait in center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::GoldPile;
                }
            }
            SpecialRoomKind::DuelCircle => {
                // Oil (sand pit) inside circle
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Oil;
                            }
                        }
                    }
                }
                // Spikes outer boundary
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3),(-3,-1),(-3,1),(3,-1),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Spikes;
                        }
                    }
                }
                // Boulder pillars at far corners
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Boulder;
                        }
                    }
                }
                // Pressure plate ring
                for &(dx, dy) in &[
                    (-2,0),(2,0),(0,-2),(0,2),
                    (-1,-1),(1,-1),(-1,1),(1,1),
                ] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::PressurePlate;
                    }
                }
            }
            SpecialRoomKind::Library => {
                // Rows of bookshelves with center aisle
                for y in room.y + 1..room.y + room.h - 1 {
                    if (y - room.y) % 2 == 0 {
                        for x in room.x + 1..room.x + room.w - 1 {
                            if x != cx && self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::Floor {
                                    self.tiles[idx] = Tile::Bookshelf;
                                }
                            }
                        }
                    }
                }
                // Crystal lanterns at end of bookshelf rows
                for y in room.y + 1..room.y + room.h - 1 {
                    if (y - room.y) % 2 == 0 {
                        for &x in &[room.x + 1, room.x + room.w - 2] {
                            if self.in_bounds(x, y) {
                                let idx = self.idx(x, y);
                                if self.tiles[idx] == Tile::Bookshelf {
                                    self.tiles[idx] = Tile::Crystal;
                                }
                            }
                        }
                    }
                }
                // Crate (reading desks) in aisle
                for y in room.y + 2..room.y + room.h - 2 {
                    if (y - room.y) % 4 == 1 && self.in_bounds(cx, y) {
                        let idx = self.idx(cx, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crate;
                        }
                    }
                }
                // InkWell at study positions
                for &(dx, dy) in &[(-1, 1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::InkWell;
                        }
                    }
                }
            }
            SpecialRoomKind::CalligraphyHall => {
                // Crystal lighting overhead
                for &x in &[room.x + 1, room.x + room.w - 2] {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Crate storage at corners
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crate;
                        }
                    }
                }
                // Writing stations: bookshelf + inkwell pairs
                for dx in [-2, 0, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bookshelf;
                        }
                    }
                    if self.in_bounds(x, cy + 1) {
                        let idx = self.idx(x, cy + 1);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::InkWell;
                        }
                    }
                }
                // Additional bookshelf rows
                for dx in [-2, 0, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy - 1) {
                        let idx = self.idx(x, cy - 1);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bookshelf;
                        }
                    }
                }
            }
            SpecialRoomKind::ScholarStudy => {
                // Bookshelves on north and south walls
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Bookshelf;
                            }
                        }
                    }
                }
                // Crystal lamp
                if self.in_bounds(cx + 1, cy) {
                    let idx = self.idx(cx + 1, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Crystal;
                    }
                }
                // InkWell at desk
                if self.in_bounds(cx - 1, cy) {
                    let idx = self.idx(cx - 1, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::InkWell;
                    }
                }
                // Crate (desk)
                if self.in_bounds(cx, cy + 1) {
                    let idx = self.idx(cx, cy + 1);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Crate;
                    }
                }
                // Codex shrine at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::CodexShrine;
                }
            }
            SpecialRoomKind::OracleRoom => {
                // Water border accents
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Bamboo curtains at edges
                for &(dx, dy) in &[(-3,-2),(-3,2),(3,-2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bamboo;
                        }
                    }
                }
                // Mushroom (incense) at corners
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
                // Crystal ring around mirror pool
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1),(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::Crystal;
                    }
                }
                // Mirror pool at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::MirrorPool;
                }
            }
            SpecialRoomKind::InscriptionWall => {
                // Full bookshelf walls on left and right
                for y in room.y + 1..room.y + room.h - 1 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Bookshelf;
                            }
                        }
                    }
                }
                // Crystal lighting
                for &(dx, dy) in &[(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // InkWell for practice
                if self.in_bounds(cx + 1, cy) {
                    let idx = self.idx(cx + 1, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::InkWell;
                    }
                }
                // Radical garden at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::RadicalGarden;
                }
            }
            SpecialRoomKind::MasterDojo => {
                // Oil (training mat) in center area
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Oil;
                            }
                        }
                    }
                }
                // Crate (weapon rack) at side
                for &(dx, dy) in &[(-3, -1), (-3, 0), (-3, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crate;
                        }
                    }
                }
                // Boulder (heavy bag) obstacles
                for &(dx, dy) in &[(3, -1), (3, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Boulder;
                        }
                    }
                }
                // Pressure plates (training dummies)
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::PressurePlate;
                    }
                }
            }
            SpecialRoomKind::MeditationChamber => {
                // Bamboo border accents
                for &(dx, dy) in &[(-3,-2),(-3,2),(3,-2),(3,2),(-3,0),(3,0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bamboo;
                        }
                    }
                }
                // Mushroom (incense) at corners
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
                // Water at cardinal positions
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Crystal candles at diagonal positions
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Spirit spring at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SpiritSpringTile;
                }
            }
            SpecialRoomKind::TranslationChallenge => {
                // Bookshelves (reference texts) behind altars
                for dx in -2..=2 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy - 1) {
                        let idx = self.idx(x, cy - 1);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bookshelf;
                        }
                    }
                }
                // Crystal lighting
                for &(dx, dy) in &[(-2, 0), (2, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // InkWell at translation stations
                for dx in -1..=1 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy + 1) {
                        let idx = self.idx(x, cy + 1);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::InkWell;
                        }
                    }
                }
                // Translation altars in a row
                for dx in -1..=1 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        self.tiles[idx] = Tile::TranslationAltar;
                    }
                }
            }
            SpecialRoomKind::AncientScroll => {
                // Mushroom (cobwebs) in corners
                for &(dx, dy) in &[(-2,-1),(2,-1),(-2,1),(2,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
                // Crystal lighting
                for &(dx, dy) in &[(-1, 0), (1, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Cracked wall (age damage)
                if self.in_bounds(room.x + 1, cy + 1) {
                    let idx = self.idx(room.x + 1, cy + 1);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::CrackedWall;
                    }
                }
                // Bookshelf alcove (expanded)
                for dx in -1..=1 {
                    let x = cx + dx;
                    for &y in &[cy - 1, cy - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Bookshelf;
                            }
                        }
                    }
                }
                // Chest (scroll) at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Chest;
                }
            }
            SpecialRoomKind::WisdomWell => {
                // Bookshelf (wisdom texts) at walls
                for &(dx, dy) in &[(-3, -1), (-3, 0), (-3, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bookshelf;
                        }
                    }
                }
                // Crystal markers around well
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Water ring around well
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1),(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Deep water well at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::DeepWater;
                }
                // Stone tutor nearby
                if self.in_bounds(cx + 2, cy) {
                    let idx = self.idx(cx + 2, cy);
                    self.tiles[idx] = Tile::StoneTutor;
                }
            }
            SpecialRoomKind::FloodedChamber => {
                // Most floor tiles become water, some deep water
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = if rng.next_u64() % 5 == 0 {
                                    Tile::DeepWater
                                } else {
                                    Tile::Water
                                };
                            }
                        }
                    }
                }
                // Safe path through center row
                for x in room.x..room.x + room.w {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if matches!(self.tiles[idx], Tile::Water | Tile::DeepWater) {
                            self.tiles[idx] = Tile::Floor;
                        }
                    }
                }
                // Boulder stepping stones
                for &(dx, dy) in &[(-2,-2),(0,-2),(2,-2),(-2,2),(0,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Water | Tile::DeepWater) {
                            self.tiles[idx] = Tile::Boulder;
                        }
                    }
                }
                // Crystal cave formations
                for &(dx, dy) in &[(-3, 0), (3, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Water | Tile::DeepWater) {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Mushroom (algae) patches
                for _ in 0..3 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Water {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
            }
            SpecialRoomKind::FrozenCavern => {
                // Ice tiles everywhere
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Ice;
                            }
                        }
                    }
                }
                // Crystal pillars
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1), (0, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Ice {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Boulder (frozen rocks)
                for &(dx, dy) in &[(-3, 0), (3, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Ice {
                            self.tiles[idx] = Tile::Boulder;
                        }
                    }
                }
                // Water (frozen puddle edges)
                for &(dx, dy) in &[(-1,-2),(1,-2),(-1,2),(1,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Ice {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
            }
            SpecialRoomKind::LavaCrossing => {
                // Lava floor
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Lava;
                            }
                        }
                    }
                }
                // Safe cross-shaped path
                for x in room.x..room.x + room.w {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::Lava {
                            self.tiles[idx] = Tile::Floor;
                        }
                    }
                }
                for y in room.y..room.y + room.h {
                    if self.in_bounds(cx, y) {
                        let idx = self.idx(cx, y);
                        if self.tiles[idx] == Tile::Lava {
                            self.tiles[idx] = Tile::Floor;
                        }
                    }
                }
                // Boulder (heat-resistant rocks) on safe paths
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Boulder;
                        }
                    }
                }
                // Gold ore (volcanic deposits) on walls
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Lava {
                            self.tiles[idx] = Tile::GoldOre;
                        }
                    }
                }
                // Chest reward at far corner
                let rx = room.x + room.w - 2;
                let ry = room.y + 1;
                if self.in_bounds(rx, ry) {
                    let idx = self.idx(rx, ry);
                    self.tiles[idx] = Tile::Chest;
                }
            }
            SpecialRoomKind::BambooForest => {
                // Dense bamboo with gaps for navigation
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 3 != 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Bamboo;
                            }
                        }
                    }
                }
                // Ensure center is clear
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Bamboo {
                                self.tiles[idx] = Tile::Floor;
                            }
                        }
                    }
                }
                // Ensure paths from edges to center
                for x in room.x..room.x + room.w {
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::Bamboo {
                            self.tiles[idx] = Tile::Floor;
                        }
                    }
                }
                // Water stream along one axis
                for y in room.y + 2..room.y + room.h - 2 {
                    let x = cx - 2;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Bamboo {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Mushroom undergrowth
                for &(dx, dy) in &[(-1,-2),(1,-2),(-1,2),(1,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Bamboo {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
                // Crystal (lanterns/fireflies)
                for &(dx, dy) in &[(0,-2),(0,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Bamboo) {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
            }
            SpecialRoomKind::WindTunnel => {
                // Oil (wind streaks) along center
                for x in room.x + 1..room.x + room.w - 1 {
                    if self.in_bounds(x, cy) && x % 2 == 0 {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Oil;
                        }
                    }
                }
                // Spike barriers on sides
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[cy - 1, cy + 1] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Spikes;
                            }
                        }
                    }
                }
                // Pressure plate wind triggers
                for &dx in &[-2, 0, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Oil) {
                            self.tiles[idx] = Tile::PressurePlate;
                        }
                    }
                }
                // Crystal wind chimes at walls
                for &(dx, dy) in &[(-3,-1),(3,-1),(-3,1),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Spikes) {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
            }
            SpecialRoomKind::CrystalCave => {
                // Water (cave pools) scattered
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 8 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Water;
                            }
                        }
                    }
                }
                // Ice (cold spots)
                for &(dx, dy) in &[(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Ice;
                        }
                    }
                }
                // Mushroom (cave growth)
                for _ in 0..3 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
                // Crystal formations (denser, with pattern)
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 5 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Crystal;
                            }
                        }
                    }
                }
            }
            SpecialRoomKind::MushroomGrotto => {
                // Mushroom clusters scattered
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 4 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Mushroom;
                            }
                        }
                    }
                }
                // Water puddles
                for _ in 0..4 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Poison gas spore clouds near mushroom clusters
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 10 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::PoisonGas;
                            }
                        }
                    }
                }
                // Crystal bioluminescence
                for &(dx, dy) in &[(-2,0),(2,0),(0,-2),(0,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Mushroom) {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
            }
            SpecialRoomKind::UndergroundRiver => {
                // River of water/deep water running through middle
                for x in room.x..room.x + room.w {
                    for dy in -1..=1 {
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = if dy == 0 {
                                    Tile::DeepWater
                                } else {
                                    Tile::Water
                                };
                            }
                        }
                    }
                }
                // Boulder (river rocks) in shallows
                for &(dx, dy) in &[(-3, -1), (-1, 1), (2, -1), (3, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Water | Tile::DeepWater) {
                            self.tiles[idx] = Tile::Boulder;
                        }
                    }
                }
                // Crystal cave formations on banks
                for &(dx, dy) in &[(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Mushroom (riverside growth)
                for &(dx, dy) in &[(-1, -2), (1, -2), (-1, 2), (1, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
                // Bridge crossing at center
                for dy in -1..=1 {
                    if self.in_bounds(cx, cy + dy) {
                        let idx = self.idx(cx, cy + dy);
                        self.tiles[idx] = Tile::Bridge;
                    }
                }
            }
            SpecialRoomKind::EchoingCavern => {
                // Water (sound-amplifying pools)
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Boulder (echo-generating rock formations)
                for &(dx, dy) in &[(-1, -2), (1, -2), (-1, 2), (1, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Boulder;
                        }
                    }
                }
                // Ice patches (frozen condensation)
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Ice;
                        }
                    }
                }
                // Crystal echo points at compass positions
                for &(dx, dy) in &[(-3, 0), (3, 0), (0, -3), (0, 3)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::Crystal;
                    }
                }
            }
            SpecialRoomKind::ShadowRealm => {
                // Oil (shadow puddles)
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 4 == 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Oil;
                            }
                        }
                    }
                }
                // CursedFloor patches
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1),(0,0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Oil) {
                            self.tiles[idx] = Tile::CursedFloor;
                        }
                    }
                }
                // Crystal (dim lights at far edges)
                for &(dx, dy) in &[(-3,-2),(3,-2),(-3,2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Oil) {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Hidden traps
                for _ in 0..5 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Oil) {
                            let trap_type = (rng.next_u64() % 3) as u8;
                            self.tiles[idx] = Tile::Trap(trap_type);
                        }
                    }
                }
            }
            SpecialRoomKind::WanderingMerchant => {
                // Bookshelf (wares display) at walls
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1),(3,-1),(3,0),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bookshelf;
                        }
                    }
                }
                // Gold pile (display items)
                for &(dx, dy) in &[(-1, 1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::GoldPile;
                        }
                    }
                }
                // Crate inventory around shop
                for &(dx, dy) in &[(-1, -1), (1, -1), (-2, 0), (2, 0)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crate;
                        }
                    }
                }
                // Shop at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Shop;
                }
            }
            SpecialRoomKind::HermitSage => {
                // Crystal lantern
                if self.in_bounds(cx + 2, cy) {
                    let idx = self.idx(cx + 2, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Crystal;
                    }
                }
                // Mushroom (herb garden)
                for &(dx, dy) in &[(1, 1), (2, 1), (1, -1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
                // Water (tea/medicine)
                if self.in_bounds(cx - 1, cy + 1) {
                    let idx = self.idx(cx - 1, cy + 1);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Water;
                    }
                }
                // Bookshelves (study)
                for dx in -1..=1 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy - 1) {
                        let idx = self.idx(x, cy - 1);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bookshelf;
                        }
                    }
                }
                // Crate (storage)
                if self.in_bounds(cx - 2, cy) {
                    let idx = self.idx(cx - 2, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Crate;
                    }
                }
                // Hermit NPC
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Npc(1);
                }
            }
            SpecialRoomKind::PrisonerCell => {
                // Spikes (chains on walls)
                for y in room.y + 1..room.y + room.h - 1 {
                    if self.in_bounds(room.x + room.w - 2, y) && y != cy {
                        let idx = self.idx(room.x + room.w - 2, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Spikes;
                        }
                    }
                }
                // Water puddle
                if self.in_bounds(cx + 1, cy + 1) {
                    let idx = self.idx(cx + 1, cy + 1);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Water;
                    }
                }
                // Crate (bed/bench)
                if self.in_bounds(cx - 1, cy + 1) {
                    let idx = self.idx(cx - 1, cy + 1);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Crate;
                    }
                }
                // Iron bars (brittle walls)
                for y in room.y + 1..room.y + room.h - 1 {
                    if y != cy && self.in_bounds(room.x, y) {
                        let idx = self.idx(room.x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::BrittleWall;
                        }
                    }
                }
                // NPC prisoner
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Npc(3);
                }
                // Locked door
                if self.in_bounds(room.x, cy) {
                    let idx = self.idx(room.x, cy);
                    if matches!(self.tiles[idx], Tile::Floor | Tile::Corridor) {
                        self.tiles[idx] = Tile::LockedDoor;
                    }
                }
            }
            SpecialRoomKind::SpiritShrine => {
                // Bamboo sacred grove at edges
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1),(3,-1),(3,0),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bamboo;
                        }
                    }
                }
                // Crystal votive candles
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Water blessing pool
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Shrine at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Shrine;
                }
            }
            SpecialRoomKind::TeaHouse => {
                // Bamboo decoration at corners
                for &(dx, dy) in &[(-3,-2),(-3,2),(3,-2),(3,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bamboo;
                        }
                    }
                }
                // Water ornamental feature
                for &(dx, dy) in &[(0, 2), (-1, 2), (1, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Crate (tea tables)
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crate;
                        }
                    }
                }
                // Bookshelves (menu/scrolls) on sides
                for &dx in &[-2, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bookshelf;
                        }
                    }
                }
                // Spirit spring (tea pot warmth)
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    self.tiles[idx] = Tile::SpiritSpringTile;
                }
                // Tea house keeper NPC
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Npc(2);
                }
            }
            SpecialRoomKind::SmithyWorkshop => {
                // Gold ore (raw materials) at walls
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::GoldOre;
                        }
                    }
                }
                // Lava heat source (expanded)
                for &(dx, dy) in &[(0, 1), (-1, 1), (1, 1), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Lava;
                        }
                    }
                }
                // Crate (tools) on sides
                for &(dx, dy) in &[(-2, 0), (2, 0), (2, -1), (-2, -1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crate;
                        }
                    }
                }
                // Boulder (anvil)
                if self.in_bounds(cx + 1, cy) {
                    let idx = self.idx(cx + 1, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Boulder;
                    }
                }
                // Forge at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Forge;
                }
            }
            SpecialRoomKind::AlchemyLab => {
                // Water (vials/solutions) scattered
                for &(dx, dy) in &[(0, -1), (1, -1), (-1, -1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Bookshelves (recipe books)
                for &(dx, dy) in &[(-2, -1), (-2, 0), (-2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bookshelf;
                        }
                    }
                }
                // Mushroom reagents
                for &(dx, dy) in &[(2, -1), (2, 0), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
                // Crystal (reagent containers)
                for &(dx, dy) in &[(-1, 1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Poison gas (fume hood exhaust)
                if self.in_bounds(cx, cy + 2) {
                    let idx = self.idx(cx, cy + 2);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::PoisonGas;
                    }
                }
                // Forge (cauldron) at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Forge;
                }
            }
            SpecialRoomKind::FortuneTeller => {
                // Bamboo curtains
                for &(dx, dy) in &[(-3,-1),(-3,0),(-3,1),(3,-1),(3,0),(3,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bamboo;
                        }
                    }
                }
                // Oil (mysterious aura)
                for &(dx, dy) in &[(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Oil;
                        }
                    }
                }
                // Mushroom (incense)
                for &(dx, dy) in &[(-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Mushroom;
                        }
                    }
                }
                // Crystal ball
                if self.in_bounds(cx, cy + 1) {
                    let idx = self.idx(cx, cy + 1);
                    self.tiles[idx] = Tile::Crystal;
                }
                // Mirror pool (scrying)
                if self.in_bounds(cx - 1, cy) {
                    let idx = self.idx(cx - 1, cy);
                    self.tiles[idx] = Tile::MirrorPool;
                }
                // Fortune teller NPC
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Npc(0);
                }
            }
            SpecialRoomKind::RefugeeCamp => {
                // Oil (campfire residue)
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Oil;
                        }
                    }
                }
                // Water (well/water source)
                if self.in_bounds(cx + 2, cy + 1) {
                    let idx = self.idx(cx + 2, cy + 1);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Water;
                    }
                }
                // Crate (supplies, expanded)
                for &(dx, dy) in &[(-1, 1), (0, -1), (2, -1), (-2, -1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Oil) {
                            self.tiles[idx] = Tile::Crate;
                        }
                    }
                }
                // Spirit spring (campfire warmth)
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::SpiritSpringTile;
                }
                // NPCs (refugees)
                for &(dx, dy) in &[(-2, 0), (1, -1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::Oil) {
                            let npc_type = (rng.next_u64() % 4) as u8;
                            self.tiles[idx] = Tile::Npc(npc_type);
                        }
                    }
                }
            }
            SpecialRoomKind::DragonGate => {
                // Gold pile offerings around approach
                for &(dx, dy) in &[(-3,0),(3,0),(0,-3),(0,3),(-2,-2),(2,-2),(-2,2),(2,2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::GoldPile;
                        }
                    }
                }
                // Lava moat
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1),(-1,-1),(1,-1),(-1,1),(1,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::GoldPile) {
                            self.tiles[idx] = Tile::Lava;
                        }
                    }
                }
                // Crystal power conduits
                for &(dx, dy) in &[(-2,-1),(2,-1),(-2,1),(2,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if matches!(self.tiles[idx], Tile::Floor | Tile::GoldPile) {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Safe approach paths (clear lava for cardinal approaches)
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        self.tiles[idx] = Tile::Floor;
                    }
                    let lx = cx + dx / 2;
                    let ly = cy + dy / 2;
                    if self.in_bounds(lx, ly) {
                        let idx = self.idx(lx, ly);
                        if self.tiles[idx] == Tile::Lava {
                            self.tiles[idx] = Tile::Floor;
                        }
                    }
                }
                // Dragon gate portal at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::DragonGatePortal;
                }
            }
            SpecialRoomKind::GamblingDen => {
                // 3 urns (chests) in a row, player picks one
                for dx in -1..=1 {
                    let x = cx + dx * 2;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Chest;
                        }
                    }
                }
                // Gold decoration around the den
                for &(dx, dy) in &[(-3, 0), (3, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::GoldPile;
                        }
                    }
                }
            }
            SpecialRoomKind::BloodAltar => {
                // Central altar with blood pools (lava-styled)
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Altar(AltarKind::Iron);
                    }
                }
                for &(dx, dy) in &[(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Oil;
                        }
                    }
                }
            }
            SpecialRoomKind::CursedTreasure => {
                // Ominous chest surrounded by cursed floor
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Chest;
                    }
                }
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::CursedFloor;
                            }
                        }
                    }
                }
            }
            SpecialRoomKind::SoulForge => {
                // Ethereal forge with crystal pillars
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Forge;
                    }
                }
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
                // Spirit spring fuel
                if self.in_bounds(cx, cy + 1) {
                    let idx = self.idx(cx, cy + 1);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Lava;
                    }
                }
            }
            SpecialRoomKind::WishingWell => {
                // Deep water well with water ring
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::DeepWater;
                    }
                }
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Gold offerings around well
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::GoldPile;
                        }
                    }
                }
            }
            SpecialRoomKind::RuneGate => {
                // 4 pressure plates in cardinal directions (rune positions)
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::PressurePlate;
                        }
                    }
                }
                // Locked treasure in center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::LockedDoor;
                    }
                }
            }
            SpecialRoomKind::MirrorMaze => {
                // Grid of crystals (mirrors) with walkable paths
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && (x + y) % 3 == 0 && !(x == cx && y == cy) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Crystal;
                            }
                        }
                    }
                }
                // Reward in center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Chest;
                    }
                }
            }
            SpecialRoomKind::WeightPuzzle => {
                // 3 pressure plates in a triangle, 3 boulders nearby, ice floor
                let plates = [(cx - 2, cy), (cx + 2, cy), (cx, cy - 2)];
                let boulders = [(cx - 2, cy + 1), (cx + 2, cy + 1), (cx, cy + 1)];
                for &(px, py) in &plates {
                    if self.in_bounds(px, py) {
                        let idx = self.idx(px, py);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::PressurePlate;
                        }
                    }
                }
                for &(bx, by) in &boulders {
                    if self.in_bounds(bx, by) {
                        let idx = self.idx(bx, by);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Boulder;
                        }
                    }
                }
                // Ice floor for sliding
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Ice;
                            }
                        }
                    }
                }
                // Locked chest reward
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::Chest;
                }
            }
            SpecialRoomKind::ToneStaircase => {
                // 4 ascending shrines in a line
                for i in 0..4 {
                    let x = cx - 1 + i;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Shrine;
                        }
                    }
                }
                // Reward chest at the top
                if self.in_bounds(cx + 3, cy) {
                    let idx = self.idx(cx + 3, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Chest;
                    }
                }
            }
            SpecialRoomKind::ElementalLock => {
                // Locked door with 5 elemental altars
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::LockedDoor;
                    }
                }
                let altars = [
                    (cx - 2, cy - 1, AltarKind::Jade),
                    (cx + 2, cy - 1, AltarKind::Gale),
                    (cx - 2, cy + 1, AltarKind::Iron),
                    (cx + 2, cy + 1, AltarKind::Gold),
                    (cx, cy + 2, AltarKind::Mirror),
                ];
                for &(ax, ay, akind) in &altars {
                    if self.in_bounds(ax, ay) {
                        let idx = self.idx(ax, ay);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Altar(akind);
                        }
                    }
                }
                // Chest behind the locked door
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Chest;
                    }
                }
            }
            SpecialRoomKind::SurvivalPit => {
                // Open arena with spikes border, central floor
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Spikes;
                            }
                        }
                    }
                }
                for y in room.y + 1..room.y + room.h - 1 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Spikes;
                            }
                        }
                    }
                }
            }
            SpecialRoomKind::TreasureRace => {
                // Open room, gold piles scattered (collected on step)
                for _ in 0..6 {
                    let x = rng.range(room.x + 1, room.x + room.w - 1);
                    let y = rng.range(room.y + 1, room.y + room.h - 1);
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::GoldPile;
                        }
                    }
                }
            }
            SpecialRoomKind::CollapsingChamber => {
                // Spikes on the outer ring, treasure chest in center
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Spikes;
                            }
                        }
                    }
                }
                for y in room.y + 2..room.y + room.h - 2 {
                    for &x in &[room.x + 1, room.x + room.w - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Spikes;
                            }
                        }
                    }
                }
                // Treasure at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Chest;
                    }
                }
            }
            SpecialRoomKind::InkFlood => {
                // Ink (oil) flooding most of the room
                for y in room.y + 1..room.y + room.h - 1 {
                    for x in room.x + 1..room.x + room.w - 1 {
                        if self.in_bounds(x, y) && rng.next_u64() % 3 != 0 {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Oil;
                            }
                        }
                    }
                }
                // InkWell at center for bonus
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    self.tiles[idx] = Tile::InkWell;
                }
                // Chest as reward near edge
                let rx = room.x + room.w - 2;
                if self.in_bounds(rx, cy) {
                    let idx = self.idx(rx, cy);
                    self.tiles[idx] = Tile::Chest;
                }
            }
            SpecialRoomKind::FormShrine => {
                // Central shrine with 4 elemental markers
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Shrine;
                    }
                }
                let markers = [
                    (cx - 2, cy, Tile::Lava),     // Flame
                    (cx + 2, cy, Tile::Boulder),   // Stone
                    (cx, cy - 2, Tile::Water),     // Mist
                    (cx, cy + 2, Tile::PressurePlate), // Tiger
                ];
                for &(mx, my, tile) in &markers {
                    if self.in_bounds(mx, my) {
                        let idx = self.idx(mx, my);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = tile;
                        }
                    }
                }
            }
            SpecialRoomKind::ClassTrial => {
                // Training dummies (pressure plates) with obstacle course
                for &(dx, dy) in &[(-2, -1), (2, -1), (-2, 1), (2, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::PressurePlate;
                        }
                    }
                }
                // Reward altar at center
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Altar(AltarKind::Iron);
                    }
                }
            }
            SpecialRoomKind::RadicalFountain => {
                // Spirit spring fountain with water and crystals
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::SpiritSpringTile;
                    }
                }
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Water;
                            }
                        }
                    }
                }
                for &(dx, dy) in &[(-2, -2), (2, -2), (-2, 2), (2, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Crystal;
                        }
                    }
                }
            }
            SpecialRoomKind::AncestorTomb => {
                // Tomb with bookshelf walls and central chest (weapon)
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Chest;
                    }
                }
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Altar(AltarKind::Jade);
                    }
                }
                for dx in -1..=1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        let x = cx + dx;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Bookshelf;
                            }
                        }
                    }
                }
            }
            SpecialRoomKind::ProphecyRoom => {
                // Murals (bookshelves) on all walls, crystal ball in center
                for x in room.x + 1..room.x + room.w - 1 {
                    for &y in &[room.y + 1, room.y + room.h - 2] {
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Bookshelf;
                            }
                        }
                    }
                }
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::MirrorPool;
                    }
                }
            }
            SpecialRoomKind::SealedMemory => {
                // Meditation space with shrine and water
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Shrine;
                    }
                }
                for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Water;
                        }
                    }
                }
                // Bookshelves as memory records
                for dx in -2..=2 {
                    let x = cx + dx;
                    if self.in_bounds(x, cy - 2) {
                        let idx = self.idx(x, cy - 2);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Bookshelf;
                        }
                    }
                }
            }
            SpecialRoomKind::DemonSeal => {
                // Sealed demon NPC with lava barrier
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Npc(3);
                    }
                }
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.in_bounds(x, y) {
                            let idx = self.idx(x, y);
                            if self.tiles[idx] == Tile::Floor {
                                self.tiles[idx] = Tile::Lava;
                            }
                        }
                    }
                }
                // Safe approach from south
                if self.in_bounds(cx, cy + 1) {
                    let idx = self.idx(cx, cy + 1);
                    self.tiles[idx] = Tile::Floor;
                }
                if self.in_bounds(cx, cy + 2) {
                    let idx = self.idx(cx, cy + 2);
                    self.tiles[idx] = Tile::Floor;
                }
            }
            SpecialRoomKind::PhoenixNest => {
                // Central spirit spring with lava nest ring
                if self.in_bounds(cx, cy) {
                    let idx = self.idx(cx, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::SpiritSpringTile;
                    }
                }
                for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::Lava;
                        }
                    }
                }
                // Chest with phoenix plume
                if self.in_bounds(cx + 2, cy) {
                    let idx = self.idx(cx + 2, cy);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Chest;
                    }
                }
                // Safe paths
                for &(dx, dy) in &[(-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let x = cx + dx;
                    let y = cy + dy;
                    if self.in_bounds(x, y) {
                        let idx = self.idx(x, y);
                        if self.tiles[idx] != Tile::Chest {
                            self.tiles[idx] = Tile::Floor;
                        }
                    }
                }
            }
            SpecialRoomKind::CalligraphyContest => {
                // Writing stations with ink wells
                for dx in [-2, 0, 2] {
                    let x = cx + dx;
                    if self.in_bounds(x, cy) {
                        let idx = self.idx(x, cy);
                        if self.tiles[idx] == Tile::Floor {
                            self.tiles[idx] = Tile::InkWell;
                        }
                    }
                }
                // NPC judge
                if self.in_bounds(cx, cy - 1) {
                    let idx = self.idx(cx, cy - 1);
                    if self.tiles[idx] == Tile::Floor {
                        self.tiles[idx] = Tile::Npc(0);
                    }
                }
            }
        }
    }

    /// Assign random modifiers to some rooms (not first or last).
    fn assign_room_modifiers(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 2 {
            return;
        }
        for i in 1..n - 1 {
            if rng.next_u64() % 100 < 30 {
                self.rooms[i].modifier = Some(match rng.next_u64() % 6 {
                    0 => RoomModifier::Dark,
                    1 => RoomModifier::Arcane,
                    2 => RoomModifier::Cursed,
                    3 => RoomModifier::Garden,
                    4 => RoomModifier::Frozen,
                    _ => RoomModifier::Infernal,
                });
            }
        }
    }

    /// Place a companion NPC in a random middle room (~40% chance per floor).
    fn place_npcs(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 40 {
            return;
        }
        // Pick a random middle room (not first, last, or second-to-last)
        let room_idx = 1 + (rng.next_u64() as usize % (n - 3));
        let room = &self.rooms[room_idx];
        let npc_type = (rng.next_u64() % 4) as u8;
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2 + 1; // offset from center
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::Npc(npc_type);
        }
    }

    /// Place a tone shrine in a random middle room (~30% chance).
    fn place_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 30 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 1;
        let cy = room.y + room.h / 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::Shrine;
        }
    }

    fn place_stroke_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 20 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 + 1;
        let cy = room.y + room.h / 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::StrokeShrine;
        }
    }

    fn place_tone_walls(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 20 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2 + 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::ToneWall;
        }
    }

    fn place_compound_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 20 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 1;
        let cy = room.y + room.h / 2 + 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::CompoundShrine;
        }
    }

    fn place_classifier_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 20 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 + 1;
        let cy = room.y + room.h / 2 + 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::ClassifierShrine;
        }
    }

    fn place_ink_wells(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 1;
        let cy = room.y + room.h / 2 - 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::InkWell;
        }
    }

    fn place_ancestor_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 + 1;
        let cy = room.y + room.h / 2 - 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::AncestorShrine;
        }
    }

    fn place_translation_altars(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2 - 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::TranslationAltar;
        }
    }

    fn place_radical_gardens(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 2;
        let cy = room.y + room.h / 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::RadicalGarden;
        }
    }

    fn place_mirror_pools(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 15 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 + 2;
        let cy = room.y + room.h / 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::MirrorPool;
        }
    }

    fn place_stone_tutors(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 18 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2;
        let cy = room.y + room.h / 2 + 2;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::StoneTutor;
        }
    }

    fn place_codex_shrines(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 15 {
            return;
        }
        let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
        let room = &self.rooms[room_idx];
        let cx = room.x + room.w / 2 - 1;
        let cy = room.y + room.h / 2 + 1;
        let idx = self.idx(cx, cy);
        if self.tiles[idx] == Tile::Floor {
            self.tiles[idx] = Tile::CodexShrine;
        }
    }

    fn place_word_bridges(&mut self, rng: &mut Rng) {
        for i in 0..self.tiles.len() {
            if self.tiles[i] == Tile::DeepWater {
                let x = (i % self.width as usize) as i32;
                let y = (i / self.width as usize) as i32;
                for &(dx, dy) in &[(0, -1), (0, 1), (-1, 0), (1, 0)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if self.in_bounds(nx, ny) {
                        let ni = self.idx(nx, ny);
                        if self.tiles[ni] == Tile::Floor && rng.next_u64() % 100 < 8 {
                            self.tiles[ni] = Tile::WordBridge;
                            break;
                        }
                    }
                }
            }
        }
    }

    fn place_locked_doors(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 4 {
            return;
        }
        if rng.next_u64() % 100 >= 12 {
            return;
        }
        let room_idx = 2 + (rng.next_u64() as usize % (n - 3));
        let room = &self.rooms[room_idx];
        let doorways = [
            (room.x + room.w / 2, room.y),
            (room.x + room.w / 2, room.y + room.h - 1),
            (room.x, room.y + room.h / 2),
            (room.x + room.w - 1, room.y + room.h / 2),
        ];
        for (dx, dy) in doorways {
            if self.in_bounds(dx, dy) {
                let di = self.idx(dx, dy);
                if self.tiles[di] == Tile::Floor || self.tiles[di] == Tile::Corridor {
                    self.tiles[di] = Tile::LockedDoor;
                    return;
                }
            }
        }
    }

    fn place_cursed_floors(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        let count = 1 + (rng.next_u64() as usize % 3);
        for _ in 0..count {
            let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
            let room = &self.rooms[room_idx];
            let rx = room.x + 1 + (rng.next_u64() as i32 % (room.w - 2).max(1));
            let ry = room.y + 1 + (rng.next_u64() as i32 % (room.h - 2).max(1));
            if self.in_bounds(rx, ry) {
                let ri = self.idx(rx, ry);
                if self.tiles[ri] == Tile::Floor {
                    self.tiles[ri] = Tile::CursedFloor;
                }
            }
        }
    }

    /// Place a blessing altar in a quiet side room (~35% chance).
    fn place_altars(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }
        if rng.next_u64() % 100 >= 35 {
            return;
        }
        for _ in 0..12 {
            let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
            let room = &self.rooms[room_idx];
            let has_special = (room.y..room.y + room.h).any(|ry| {
                (room.x..room.x + room.w).any(|rx| {
                    if rx >= 0 && ry >= 0 && rx < self.width && ry < self.height {
                        let idx = (ry * self.width + rx) as usize;
                        matches!(
                            self.tiles[idx],
                            Tile::Forge
                                | Tile::Shop
                                | Tile::StairsDown
                                | Tile::Chest
                                | Tile::Npc(_)
                                | Tile::Shrine
                                | Tile::Altar(_)
                                | Tile::Seal(_)
                                | Tile::Sign(_)
                        )
                    } else {
                        false
                    }
                })
            });
            if has_special {
                continue;
            }

            let ax = room.x + room.w / 2;
            let ay = room.y + room.h / 2;
            if !self.in_bounds(ax, ay) {
                continue;
            }
            let idx = self.idx(ax, ay);
            if self.tiles[idx] == Tile::Floor {
                self.tiles[idx] = Tile::Altar(AltarKind::random(rng));
                return;
            }
        }
    }

    /// Place 1-2 script seals that reshape rooms when stepped on.
    fn place_seals(&mut self, rng: &mut Rng) {
        let n = self.rooms.len();
        if n <= 3 {
            return;
        }

        let seal_count = 1 + (rng.next_u64() % 2) as usize;
        let mut used_rooms = Vec::new();
        let mut placed = 0;
        for _ in 0..20 {
            if placed >= seal_count {
                break;
            }

            let room_idx = 1 + (rng.next_u64() as usize % (n - 2));
            if used_rooms.contains(&room_idx) {
                continue;
            }
            let room = &self.rooms[room_idx];
            let has_special = (room.y..room.y + room.h).any(|ry| {
                (room.x..room.x + room.w).any(|rx| {
                    if rx >= 0 && ry >= 0 && rx < self.width && ry < self.height {
                        let idx = (ry * self.width + rx) as usize;
                        matches!(
                            self.tiles[idx],
                            Tile::Forge
                                | Tile::Shop
                                | Tile::StairsDown
                                | Tile::Chest
                                | Tile::Npc(_)
                                | Tile::Shrine
                                | Tile::Altar(_)
                                | Tile::Seal(_)
                                | Tile::Sign(_)
                        )
                    } else {
                        false
                    }
                })
            });
            if has_special {
                continue;
            }

            let sx = room.x + room.w / 2;
            let sy = room.y + room.h / 2;
            if !self.in_bounds(sx, sy) {
                continue;
            }
            let idx = self.idx(sx, sy);
            if self.tiles[idx] != Tile::Floor {
                continue;
            }

            self.tiles[idx] = Tile::Seal(SealKind::random(rng));
            used_rooms.push(room_idx);
            placed += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AltarKind, DungeonLevel, Rng, Room, SealKind, Tile};

    fn make_clean_test_level() -> DungeonLevel {
        let width = 24;
        let height = 24;
        let mut level = DungeonLevel {
            width,
            height,
            tiles: vec![Tile::Wall; (width * height) as usize],
            rooms: vec![
                Room {
                    x: 1,
                    y: 1,
                    w: 5,
                    h: 5,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 8,
                    y: 1,
                    w: 5,
                    h: 5,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 15,
                    y: 1,
                    w: 5,
                    h: 5,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 1,
                    y: 10,
                    w: 5,
                    h: 5,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 8,
                    y: 10,
                    w: 5,
                    h: 5,
                    modifier: None,
                    special: None,
                },
            ],
            visible: vec![false; (width * height) as usize],
            revealed: vec![false; (width * height) as usize],
        };
        for room in level.rooms.clone() {
            for y in room.y..room.y + room.h {
                for x in room.x..room.x + room.w {
                    let idx = level.idx(x, y);
                    level.tiles[idx] = Tile::Floor;
                }
            }
        }
        level
    }

    fn make_spacious_test_level() -> DungeonLevel {
        let width = 40;
        let height = 28;
        let mut level = DungeonLevel {
            width,
            height,
            tiles: vec![Tile::Wall; (width * height) as usize],
            rooms: vec![
                Room {
                    x: 1,
                    y: 1,
                    w: 8,
                    h: 8,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 11,
                    y: 1,
                    w: 8,
                    h: 8,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 21,
                    y: 1,
                    w: 8,
                    h: 8,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 1,
                    y: 12,
                    w: 8,
                    h: 8,
                    modifier: None,
                    special: None,
                },
                Room {
                    x: 11,
                    y: 12,
                    w: 8,
                    h: 8,
                    modifier: None,
                    special: None,
                },
            ],
            visible: vec![false; (width * height) as usize],
            revealed: vec![false; (width * height) as usize],
        };
        for room in level.rooms.clone() {
            for y in room.y..room.y + room.h {
                for x in room.x..room.x + room.w {
                    let idx = level.idx(x, y);
                    level.tiles[idx] = Tile::Floor;
                }
            }
        }
        level
    }

    fn has_pushable_bridge_setup(level: &DungeonLevel) -> bool {
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for y in 0..level.height {
            for x in 0..level.width {
                if level.tile(x, y) != Tile::Water {
                    continue;
                }

                for (dx, dy) in dirs {
                    let crate_x = x - dx;
                    let crate_y = y - dy;
                    let stand_x = crate_x - dx;
                    let stand_y = crate_y - dy;
                    if !level.in_bounds(crate_x, crate_y) || !level.in_bounds(stand_x, stand_y) {
                        continue;
                    }

                    if level.tile(crate_x, crate_y) == Tile::Crate {
                        let stand_tile = level.tile(stand_x, stand_y);
                        if stand_tile.is_walkable() && stand_tile != Tile::Water {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn has_brittle_vault(level: &DungeonLevel) -> bool {
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for y in 0..level.height {
            for x in 0..level.width {
                if level.tile(x, y) != Tile::BrittleWall {
                    continue;
                }

                for (dx, dy) in dirs {
                    let chest_x = x + dx;
                    let chest_y = y + dy;
                    let back_x = chest_x + dx;
                    let back_y = chest_y + dy;
                    let side_a = (-dy, dx);
                    let side_b = (dy, -dx);
                    if !level.in_bounds(chest_x, chest_y) || !level.in_bounds(back_x, back_y) {
                        continue;
                    }

                    if level.tile(chest_x, chest_y) == Tile::Chest
                        && level.tile(back_x, back_y) == Tile::Wall
                        && level.tile(x + side_a.0, y + side_a.1) == Tile::Wall
                        && level.tile(chest_x + side_a.0, chest_y + side_a.1) == Tile::Wall
                        && level.tile(x + side_b.0, y + side_b.1) == Tile::Wall
                        && level.tile(chest_x + side_b.0, chest_y + side_b.1) == Tile::Wall
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn has_deep_water_cache(level: &DungeonLevel) -> bool {
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for y in 0..level.height {
            for x in 0..level.width {
                if level.tile(x, y) != Tile::DeepWater {
                    continue;
                }

                for (dx, dy) in dirs {
                    let crate_x = x - dx;
                    let crate_y = y - dy;
                    let stand_x = crate_x - dx;
                    let stand_y = crate_y - dy;
                    let chest_x = x + dx;
                    let chest_y = y + dy;
                    let back_x = chest_x + dx;
                    let back_y = chest_y + dy;
                    let side_a = (-dy, dx);
                    let side_b = (dy, -dx);
                    if !level.in_bounds(crate_x, crate_y)
                        || !level.in_bounds(stand_x, stand_y)
                        || !level.in_bounds(chest_x, chest_y)
                        || !level.in_bounds(back_x, back_y)
                    {
                        continue;
                    }

                    if level.tile(crate_x, crate_y) == Tile::Crate
                        && level.tile(stand_x, stand_y) == Tile::Floor
                        && level.tile(chest_x, chest_y) == Tile::Chest
                        && level.tile(back_x, back_y) == Tile::Wall
                        && level.tile(x + side_a.0, y + side_a.1) == Tile::Wall
                        && level.tile(chest_x + side_a.0, chest_y + side_a.1) == Tile::Wall
                        && level.tile(x + side_b.0, y + side_b.1) == Tile::Wall
                        && level.tile(chest_x + side_b.0, chest_y + side_b.1) == Tile::Wall
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn has_spike_bridge(level: &DungeonLevel) -> bool {
        for y in 0..level.height {
            for x in 0..level.width - 3 {
                if level.tile(x, y) == Tile::Spikes
                    && level.tile(x + 1, y) == Tile::Spikes
                    && level.tile(x + 2, y) == Tile::Spikes
                    && level.tile(x + 3, y) == Tile::Chest
                {
                    return true;
                }
            }
        }
        false
    }

    fn has_oil_fire_trap(level: &DungeonLevel) -> bool {
        for y in 0..level.height {
            for x in 0..level.width - 2 {
                if level.tile(x, y) == Tile::Oil
                    && level.tile(x + 1, y) == Tile::Oil
                    && level.tile(x + 2, y) == Tile::Chest
                {
                    return true;
                }
            }
        }
        false
    }

    fn has_seal_chain(level: &DungeonLevel) -> bool {
        for y in 0..level.height {
            for x in 0..level.width - 2 {
                if matches!(level.tile(x, y), Tile::Seal(_))
                    && matches!(level.tile(x + 2, y), Tile::Seal(_))
                {
                    return true;
                }
            }
        }
        false
    }

    fn has_any_puzzle_room(level: &DungeonLevel) -> bool {
        has_brittle_vault(level)
            || has_deep_water_cache(level)
            || has_spike_bridge(level)
            || has_oil_fire_trap(level)
            || has_seal_chain(level)
    }

    #[test]
    fn hazards_and_altars_are_walkable_but_crates_block() {
        assert!(Tile::Spikes.is_walkable());
        assert!(Tile::Oil.is_walkable());
        assert!(Tile::Water.is_walkable());
        assert!(Tile::Altar(AltarKind::Jade).is_walkable());
        assert!(Tile::Seal(SealKind::Ember).is_walkable());
        assert!(!Tile::Crate.is_walkable());
        assert!(!Tile::CrackedWall.is_walkable());
        assert!(!Tile::BrittleWall.is_walkable());
        assert!(!Tile::DeepWater.is_walkable());
    }

    #[test]
    fn place_altars_adds_a_blessing_site_to_clean_levels() {
        let mut level = make_clean_test_level();
        let mut rng = Rng::new(2);

        level.place_altars(&mut rng);

        assert!(level
            .tiles
            .iter()
            .any(|tile| matches!(tile, Tile::Altar(_))));
    }

    #[test]
    fn place_seals_adds_script_seals_to_clean_levels() {
        let mut level = make_clean_test_level();
        let mut rng = Rng::new(7);

        level.place_seals(&mut rng);

        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::Seal(_))));
    }

    #[test]
    fn place_secret_room_carves_hidden_chamber_with_cracked_entrance() {
        let mut level = make_clean_test_level();
        let mut rng = Rng::new(11);
        let original_open_tiles = level
            .tiles
            .iter()
            .filter(|tile| !matches!(tile, Tile::Wall))
            .count();

        level.place_secret_room(&mut rng);

        let new_open_tiles = level
            .tiles
            .iter()
            .filter(|tile| !matches!(tile, Tile::Wall))
            .count();
        assert!(level
            .tiles
            .iter()
            .any(|tile| matches!(tile, Tile::CrackedWall)));
        assert!(new_open_tiles > original_open_tiles);
    }

    #[test]
    fn place_secret_room_adds_hidden_point_of_interest() {
        let mut level = make_clean_test_level();
        let mut rng = Rng::new(11);

        level.place_secret_room(&mut rng);

        assert!(level.tiles.iter().any(|tile| {
            matches!(
                tile,
                Tile::Chest | Tile::Forge | Tile::Shrine | Tile::Altar(_)
            )
        }));
    }

    #[test]
    fn generated_levels_hide_secret_rooms_on_most_runs() {
        let mut secret_count = 0;
        for seed in 1..=24 {
            let level = DungeonLevel::generate(48, 48, seed, 1);
            if level
                .tiles
                .iter()
                .any(|tile| matches!(tile, Tile::CrackedWall))
            {
                secret_count += 1;
            }
        }

        assert!(
            secret_count >= 18,
            "expected secret-room entrances on most sample floors, found {secret_count}"
        );
    }

    #[test]
    fn generated_levels_regularly_offer_bridge_building_setups() {
        let mut bridge_count = 0;
        for seed in 1..=24 {
            let level = DungeonLevel::generate(48, 48, seed, 1);
            if has_pushable_bridge_setup(&level) {
                bridge_count += 1;
            }
        }

        assert!(
            bridge_count >= 10,
            "expected bridge setups across the sample set, found {bridge_count}"
        );
    }

    #[test]
    fn place_puzzle_rooms_adds_visible_environmental_niches() {
        let mut level = make_spacious_test_level();
        let mut rng = Rng::new(19);

        level.place_puzzle_rooms(&mut rng);

        assert!(has_any_puzzle_room(&level));
    }

    #[test]
    fn generated_levels_regularly_offer_puzzle_rooms() {
        let mut puzzle_count = 0;
        let mut brittle_count = 0;
        let mut deep_water_count = 0;
        let mut spike_count = 0;
        let mut oil_count = 0;
        let mut seal_count = 0;
        for seed in 1..=24 {
            let level = DungeonLevel::generate(48, 48, seed, 1);
            if has_any_puzzle_room(&level) {
                puzzle_count += 1;
            }
            if has_brittle_vault(&level) {
                brittle_count += 1;
            }
            if has_deep_water_cache(&level) {
                deep_water_count += 1;
            }
            if has_spike_bridge(&level) {
                spike_count += 1;
            }
            if has_oil_fire_trap(&level) {
                oil_count += 1;
            }
            if has_seal_chain(&level) {
                seal_count += 1;
            }
        }

        assert!(
            puzzle_count >= 16,
            "expected puzzle rooms on most sample floors, found {puzzle_count}"
        );
        let variant_types = [
            brittle_count > 0,
            deep_water_count > 0,
            spike_count > 0,
            oil_count > 0,
            seal_count > 0,
        ];
        let variants_seen = variant_types.iter().filter(|&&v| v).count();
        assert!(
            variants_seen >= 2,
            "expected at least 2 puzzle room variants across 24 seeds, saw {variants_seen}"
        );
    }

    #[test]
    fn tutorial_floor_has_required_landmarks() {
        let level = DungeonLevel::tutorial(48, 48);

        assert_eq!(level.start_pos(), (8, 22));
        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::Sign(0))));
        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::Sign(1))));
        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::Sign(2))));
        assert!(level.tiles.iter().any(|tile| matches!(tile, Tile::Sign(3))));
        assert!(level.tiles.iter().any(|tile| *tile == Tile::Forge));
        assert!(level.tiles.iter().any(|tile| *tile == Tile::StairsDown));
        assert!(level.is_walkable(8, 20));
    }
}
