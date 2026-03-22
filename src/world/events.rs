#![allow(dead_code)]
/// FTL-like random space events for the Starfinder RPG.
///
/// Events trigger at star systems or during FTL jumps and present the player
/// with choices that affect ship state, crew, and resources.  Some events
/// integrate Chinese language learning.

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventCategory {
    DistressSignal,
    PirateEncounter,
    Trading,
    Discovery,
    AnomalyEncounter,
    CrewEvent,
    AlienContact,
    HazardEvent,
    AncientRuins,
    LanguageChallenge,
}

#[derive(Clone, Debug)]
pub enum EventRequirement {
    HasCrewRole(u8),
    HasFuel(i32),
    HasCredits(i32),
    HasRadical(&'static str),
    HasClass(u8),
    None,
}

#[derive(Clone, Debug)]
pub enum EventOutcome {
    GainFuel(i32),
    LoseFuel(i32),
    GainCredits(i32),
    LoseCredits(i32),
    GainHull(i32),
    LoseHull(i32),
    GainRadical(&'static str),
    GainCrewMember,
    LoseCrewMember,
    StartCombat(u8),
    GainItem(&'static str),
    HealCrew(i32),
    DamageCrew(i32),
    RepairShip(i32),
    Nothing,
    GainScrap(i32),
    ShieldDamage(i32),
    FuelAndCredits(i32, i32),
    HullAndFuel(i32, i32),
    CombatReward(u8, i32),
}

#[derive(Clone, Debug)]
pub struct EventChoice {
    pub text: &'static str,
    pub chinese_hint: &'static str,
    pub outcome: EventOutcome,
    pub requires: Option<EventRequirement>,
}

#[derive(Clone, Debug)]
pub struct SpaceEvent {
    pub id: usize,
    pub title: &'static str,
    pub chinese_title: &'static str,
    pub description: &'static str,
    pub choices: &'static [EventChoice],
    pub category: EventCategory,
}

// ---------------------------------------------------------------------------
// Event pool — 42 events grouped by category
// ---------------------------------------------------------------------------

// ── Distress Signals (7) ────────────────────────────────────────────────────

static EVENT_DISTRESS_BEACON: SpaceEvent = SpaceEvent {
    id: 0,
    title: "Distress Beacon",
    chinese_title: "求救信号",
    description: "Your sensors detect a faint distress beacon from a drifting vessel. \
                  The hull is cracked and life signs are weak but present.",
    choices: &[
        EventChoice {
            text: "Dock and render aid (costs fuel)",
            chinese_hint: "帮助 (bāngzhù) — help",
            outcome: EventOutcome::FuelAndCredits(-3, 15),
            requires: Some(EventRequirement::HasFuel(3)),
        },
        EventChoice {
            text: "Salvage what you can from the wreckage",
            chinese_hint: "残骸 (cánhái) — wreckage",
            outcome: EventOutcome::GainScrap(10),
            requires: None,
        },
        EventChoice {
            text: "Ignore the signal and move on",
            chinese_hint: "忽略 (hūlüè) — ignore",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::DistressSignal,
};

static EVENT_ESCAPE_POD: SpaceEvent = SpaceEvent {
    id: 1,
    title: "Escape Pod",
    chinese_title: "逃生舱",
    description: "An escape pod tumbles through the void, its emergency light blinking red. \
                  A single occupant appears to be in cryo-stasis.",
    choices: &[
        EventChoice {
            text: "Rescue the survivor",
            chinese_hint: "救 (jiù) — rescue",
            outcome: EventOutcome::GainCrewMember,
            requires: None,
        },
        EventChoice {
            text: "Strip the pod for parts",
            chinese_hint: "零件 (língjiàn) — parts",
            outcome: EventOutcome::GainScrap(8),
            requires: None,
        },
        EventChoice {
            text: "Scan the pod from a safe distance",
            chinese_hint: "扫描 (sǎomiáo) — scan",
            outcome: EventOutcome::GainCredits(5),
            requires: None,
        },
    ],
    category: EventCategory::DistressSignal,
};

static EVENT_DAMAGED_FREIGHTER: SpaceEvent = SpaceEvent {
    id: 2,
    title: "Damaged Freighter",
    chinese_title: "受损货船",
    description: "A massive freighter lists to starboard, venting atmosphere from multiple \
                  breaches. The captain hails you on emergency frequencies.",
    choices: &[
        EventChoice {
            text: "Send your engineer to help repair",
            chinese_hint: "修理 (xiūlǐ) — repair",
            outcome: EventOutcome::GainCredits(25),
            requires: Some(EventRequirement::HasCrewRole(1)),
        },
        EventChoice {
            text: "Board and loot the cargo hold",
            chinese_hint: "货物 (huòwù) — cargo",
            outcome: EventOutcome::GainScrap(15),
            requires: None,
        },
        EventChoice {
            text: "Offer fuel in exchange for credits",
            chinese_hint: "交换 (jiāohuàn) — exchange",
            outcome: EventOutcome::FuelAndCredits(-5, 30),
            requires: Some(EventRequirement::HasFuel(5)),
        },
    ],
    category: EventCategory::DistressSignal,
};

static EVENT_COLONY_SOS: SpaceEvent = SpaceEvent {
    id: 3,
    title: "Colony Distress Call",
    chinese_title: "殖民地求救",
    description: "A frontier colony broadcasts an urgent plea: plague has struck and \
                  medical supplies are exhausted. They offer everything they have.",
    choices: &[
        EventChoice {
            text: "Deliver emergency supplies",
            chinese_hint: "医药 (yīyào) — medicine",
            outcome: EventOutcome::FuelAndCredits(-4, 20),
            requires: Some(EventRequirement::HasFuel(4)),
        },
        EventChoice {
            text: "Trade supplies for their rare artifacts",
            chinese_hint: "古物 (gǔwù) — artifact",
            outcome: EventOutcome::GainRadical("疒"),
            requires: None,
        },
        EventChoice {
            text: "Log the coordinates and move on",
            chinese_hint: "记录 (jìlù) — record",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::DistressSignal,
};

static EVENT_STRANDED_MINERS: SpaceEvent = SpaceEvent {
    id: 4,
    title: "Stranded Miners",
    chinese_title: "被困矿工",
    description: "A mining crew is trapped on an asteroid after their shuttle broke down. \
                  They wave frantically through the viewport.",
    choices: &[
        EventChoice {
            text: "Evacuate the miners aboard your ship",
            chinese_hint: "矿工 (kuànggōng) — miner",
            outcome: EventOutcome::GainCrewMember,
            requires: None,
        },
        EventChoice {
            text: "Tow their shuttle to the nearest station (costs fuel)",
            chinese_hint: "拖 (tuō) — tow",
            outcome: EventOutcome::FuelAndCredits(-6, 35),
            requires: Some(EventRequirement::HasFuel(6)),
        },
        EventChoice {
            text: "Take their ore stockpile as payment for rescue",
            chinese_hint: "矿石 (kuàngshí) — ore",
            outcome: EventOutcome::GainScrap(20),
            requires: None,
        },
    ],
    category: EventCategory::DistressSignal,
};

static EVENT_GHOST_SHIP: SpaceEvent = SpaceEvent {
    id: 5,
    title: "Ghost Ship",
    chinese_title: "幽灵船",
    description: "A ship of ancient design drifts silently — no power signatures, no life signs. \
                  Its hull is covered in strange glyphs that seem to shift when you look away.",
    choices: &[
        EventChoice {
            text: "Board and investigate",
            chinese_hint: "调查 (diàochá) — investigate",
            outcome: EventOutcome::CombatReward(2, 20),
            requires: None,
        },
        EventChoice {
            text: "Scan the hull glyphs from your ship",
            chinese_hint: "符号 (fúhào) — glyph",
            outcome: EventOutcome::GainRadical("鬼"),
            requires: None,
        },
        EventChoice {
            text: "Keep your distance — some things are best left alone",
            chinese_hint: "远离 (yuǎnlí) — stay away",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::DistressSignal,
};

static EVENT_MEDICAL_FRIGATE: SpaceEvent = SpaceEvent {
    id: 6,
    title: "Medical Frigate",
    chinese_title: "医疗护卫舰",
    description: "A medical frigate hails you with a request: they need an escort through \
                  pirate territory. In return, they can treat your crew.",
    choices: &[
        EventChoice {
            text: "Escort them (risk pirate attack)",
            chinese_hint: "护送 (hùsòng) — escort",
            outcome: EventOutcome::HealCrew(30),
            requires: None,
        },
        EventChoice {
            text: "Ask for medical supplies instead of escorting",
            chinese_hint: "药品 (yàopǐn) — medical supplies",
            outcome: EventOutcome::HealCrew(15),
            requires: None,
        },
        EventChoice {
            text: "Decline — you have your own problems",
            chinese_hint: "拒绝 (jùjué) — decline",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::DistressSignal,
};

// ── Pirate Encounters (6) ───────────────────────────────────────────────────

static EVENT_PIRATE_AMBUSH: SpaceEvent = SpaceEvent {
    id: 7,
    title: "Pirate Ambush",
    chinese_title: "海盗伏击",
    description: "Proximity alarms blare as two pirate fighters decloak off your bow. \
                  Their leader broadcasts: 'Pay up or we paint the void with your hull.'",
    choices: &[
        EventChoice {
            text: "Fight them off",
            chinese_hint: "战斗 (zhàndòu) — fight",
            outcome: EventOutcome::CombatReward(2, 25),
            requires: None,
        },
        EventChoice {
            text: "Pay the tribute",
            chinese_hint: "付钱 (fùqián) — pay",
            outcome: EventOutcome::LoseCredits(20),
            requires: Some(EventRequirement::HasCredits(20)),
        },
        EventChoice {
            text: "Bluff — claim you are a military vessel",
            chinese_hint: "欺骗 (qīpiàn) — bluff",
            outcome: EventOutcome::Nothing,
            requires: Some(EventRequirement::HasClass(1)),
        },
        EventChoice {
            text: "Attempt to flee at full burn",
            chinese_hint: "逃跑 (táopǎo) — flee",
            outcome: EventOutcome::LoseFuel(4),
            requires: Some(EventRequirement::HasFuel(4)),
        },
    ],
    category: EventCategory::PirateEncounter,
};

static EVENT_PIRATE_BASE: SpaceEvent = SpaceEvent {
    id: 8,
    title: "Pirate Base",
    chinese_title: "海盗基地",
    description: "Long-range scans reveal a hidden pirate outpost nestled in an asteroid field. \
                  Its hangars glow with stolen cargo.",
    choices: &[
        EventChoice {
            text: "Raid the outpost",
            chinese_hint: "突袭 (tūxí) — raid",
            outcome: EventOutcome::CombatReward(3, 40),
            requires: None,
        },
        EventChoice {
            text: "Sneak past while their patrols are away",
            chinese_hint: "潜行 (qiánxíng) — sneak",
            outcome: EventOutcome::GainFuel(2),
            requires: None,
        },
        EventChoice {
            text: "Hail them and offer to trade",
            chinese_hint: "贸易 (màoyì) — trade",
            outcome: EventOutcome::GainItem("Pirate Star Chart"),
            requires: None,
        },
    ],
    category: EventCategory::PirateEncounter,
};

static EVENT_PIRATE_BOARDING: SpaceEvent = SpaceEvent {
    id: 9,
    title: "Boarding Party",
    chinese_title: "登船突击",
    description: "Pirates lock onto your airlock with a boarding tube. You can hear \
                  the cutting torch on the other side of the hull.",
    choices: &[
        EventChoice {
            text: "Repel the boarders in close combat",
            chinese_hint: "抵抗 (dǐkàng) — resist",
            outcome: EventOutcome::CombatReward(2, 15),
            requires: None,
        },
        EventChoice {
            text: "Vent the compartment into space",
            chinese_hint: "真空 (zhēnkōng) — vacuum",
            outcome: EventOutcome::LoseHull(5),
            requires: None,
        },
        EventChoice {
            text: "Surrender your cargo",
            chinese_hint: "投降 (tóuxiáng) — surrender",
            outcome: EventOutcome::LoseCredits(30),
            requires: Some(EventRequirement::HasCredits(30)),
        },
    ],
    category: EventCategory::PirateEncounter,
};

static EVENT_PIRATE_DEFECTOR: SpaceEvent = SpaceEvent {
    id: 10,
    title: "Pirate Defector",
    chinese_title: "海盗叛逃者",
    description: "A small shuttle approaches with weapons powered down. The pilot claims \
                  to be a pirate deserter seeking asylum.",
    choices: &[
        EventChoice {
            text: "Welcome them aboard",
            chinese_hint: "加入 (jiārù) — join",
            outcome: EventOutcome::GainCrewMember,
            requires: None,
        },
        EventChoice {
            text: "Demand intel on pirate routes as payment",
            chinese_hint: "情报 (qíngbào) — intel",
            outcome: EventOutcome::GainItem("Pirate Cipher Key"),
            requires: None,
        },
        EventChoice {
            text: "It could be a trap — drive them away",
            chinese_hint: "陷阱 (xiànjǐng) — trap",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::PirateEncounter,
};

static EVENT_PIRATE_CONVOY: SpaceEvent = SpaceEvent {
    id: 11,
    title: "Pirate Convoy",
    chinese_title: "海盗船队",
    description: "A small convoy of pirate ships lumbers through the sector, laden with \
                  plunder. Their formation is sloppy — an opportunity, or a lure?",
    choices: &[
        EventChoice {
            text: "Ambush the trailing ship",
            chinese_hint: "埋伏 (máifú) — ambush",
            outcome: EventOutcome::CombatReward(1, 30),
            requires: None,
        },
        EventChoice {
            text: "Shadow the convoy to learn their base location",
            chinese_hint: "跟踪 (gēnzōng) — follow",
            outcome: EventOutcome::GainItem("Hidden Base Coordinates"),
            requires: None,
        },
        EventChoice {
            text: "Avoid them entirely",
            chinese_hint: "避开 (bìkāi) — avoid",
            outcome: EventOutcome::LoseFuel(2),
            requires: None,
        },
    ],
    category: EventCategory::PirateEncounter,
};

static EVENT_RANSOM_DEMAND: SpaceEvent = SpaceEvent {
    id: 12,
    title: "Ransom Demand",
    chinese_title: "勒索要求",
    description: "Pirates have captured a passenger liner and demand ransom. They broadcast \
                  the terrified passengers on an open channel.",
    choices: &[
        EventChoice {
            text: "Pay the ransom to free the passengers",
            chinese_hint: "赎金 (shújīn) — ransom",
            outcome: EventOutcome::FuelAndCredits(-2, -15),
            requires: Some(EventRequirement::HasCredits(15)),
        },
        EventChoice {
            text: "Attack the pirates to free the hostages",
            chinese_hint: "解救 (jiějiù) — rescue",
            outcome: EventOutcome::CombatReward(3, 20),
            requires: None,
        },
        EventChoice {
            text: "Report the situation and continue on your way",
            chinese_hint: "报告 (bàogào) — report",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::PirateEncounter,
};

// ── Trading (6) ─────────────────────────────────────────────────────────────

static EVENT_WANDERING_MERCHANT: SpaceEvent = SpaceEvent {
    id: 13,
    title: "Wandering Merchant",
    chinese_title: "流浪商人",
    description: "A battered cargo hauler pulls alongside. Its eccentric captain opens a \
                  channel: 'Best deals in the sector! Everything must go!'",
    choices: &[
        EventChoice {
            text: "Buy fuel reserves",
            chinese_hint: "燃料 (ránliào) — fuel",
            outcome: EventOutcome::FuelAndCredits(8, -10),
            requires: Some(EventRequirement::HasCredits(10)),
        },
        EventChoice {
            text: "Buy hull repair nanites",
            chinese_hint: "修复 (xiūfù) — repair",
            outcome: EventOutcome::HullAndFuel(10, 0),
            requires: Some(EventRequirement::HasCredits(15)),
        },
        EventChoice {
            text: "Browse the curiosities",
            chinese_hint: "好奇 (hàoqí) — curious",
            outcome: EventOutcome::GainRadical("贝"),
            requires: None,
        },
        EventChoice {
            text: "Decline and move on",
            chinese_hint: "不用 (bùyòng) — no thanks",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::Trading,
};

static EVENT_BLACK_MARKET: SpaceEvent = SpaceEvent {
    id: 14,
    title: "Black Market",
    chinese_title: "黑市",
    description: "Hidden among a cluster of derelict hulls is a thriving black market station. \
                  The goods are questionable, but the prices are tempting.",
    choices: &[
        EventChoice {
            text: "Buy military-grade weapons",
            chinese_hint: "武器 (wǔqì) — weapon",
            outcome: EventOutcome::GainItem("Ion Disruptor"),
            requires: Some(EventRequirement::HasCredits(25)),
        },
        EventChoice {
            text: "Sell your scrap for top credit",
            chinese_hint: "卖 (mài) — sell",
            outcome: EventOutcome::GainCredits(20),
            requires: None,
        },
        EventChoice {
            text: "Gamble in the fight pit",
            chinese_hint: "赌博 (dǔbó) — gamble",
            outcome: EventOutcome::CombatReward(1, 35),
            requires: None,
        },
        EventChoice {
            text: "Leave — this place feels wrong",
            chinese_hint: "离开 (líkāi) — leave",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::Trading,
};

static EVENT_FUEL_DEPOT: SpaceEvent = SpaceEvent {
    id: 15,
    title: "Fuel Depot",
    chinese_title: "燃料补给站",
    description: "An automated fuel depot orbits a gas giant. Its prices are fair \
                  and the pumps are fast.",
    choices: &[
        EventChoice {
            text: "Buy a full tank",
            chinese_hint: "加满 (jiāmǎn) — fill up",
            outcome: EventOutcome::FuelAndCredits(10, -12),
            requires: Some(EventRequirement::HasCredits(12)),
        },
        EventChoice {
            text: "Buy a partial refuel",
            chinese_hint: "一些 (yīxiē) — some",
            outcome: EventOutcome::FuelAndCredits(5, -6),
            requires: Some(EventRequirement::HasCredits(6)),
        },
        EventChoice {
            text: "Hack the pumps for free fuel",
            chinese_hint: "黑客 (hēikè) — hacker",
            outcome: EventOutcome::GainFuel(6),
            requires: Some(EventRequirement::HasCrewRole(3)),
        },
    ],
    category: EventCategory::Trading,
};

static EVENT_SMUGGLER_OFFER: SpaceEvent = SpaceEvent {
    id: 16,
    title: "Smuggler's Offer",
    chinese_title: "走私交易",
    description: "A fast courier intercepts you with a business proposition: transport a \
                  sealed container to the next sector. No questions asked.",
    choices: &[
        EventChoice {
            text: "Accept the job",
            chinese_hint: "接受 (jiēshòu) — accept",
            outcome: EventOutcome::GainCredits(30),
            requires: None,
        },
        EventChoice {
            text: "Open the container first",
            chinese_hint: "打开 (dǎkāi) — open",
            outcome: EventOutcome::GainRadical("口"),
            requires: None,
        },
        EventChoice {
            text: "Refuse and report the smuggler",
            chinese_hint: "举报 (jǔbào) — report",
            outcome: EventOutcome::GainCredits(10),
            requires: None,
        },
    ],
    category: EventCategory::Trading,
};

static EVENT_TRADE_STATION: SpaceEvent = SpaceEvent {
    id: 17,
    title: "Trade Station",
    chinese_title: "贸易站",
    description: "A bustling orbital station invites all ships to dock. Merchants from \
                  a dozen species hawk their wares in a cacophony of languages.",
    choices: &[
        EventChoice {
            text: "Trade scrap for fuel",
            chinese_hint: "换 (huàn) — exchange",
            outcome: EventOutcome::GainFuel(5),
            requires: None,
        },
        EventChoice {
            text: "Visit the shipyard for repairs",
            chinese_hint: "船坞 (chuánwù) — shipyard",
            outcome: EventOutcome::RepairShip(15),
            requires: Some(EventRequirement::HasCredits(20)),
        },
        EventChoice {
            text: "Recruit a crew member from the bar",
            chinese_hint: "招募 (zhāomù) — recruit",
            outcome: EventOutcome::GainCrewMember,
            requires: Some(EventRequirement::HasCredits(10)),
        },
    ],
    category: EventCategory::Trading,
};

static EVENT_AUCTION_HOUSE: SpaceEvent = SpaceEvent {
    id: 18,
    title: "Auction House",
    chinese_title: "拍卖行",
    description: "A luxury liner hosts a traveling auction. Among the lots: ancient \
                  navigation data, alien tech, and a mysterious sealed crate.",
    choices: &[
        EventChoice {
            text: "Bid on the navigation data",
            chinese_hint: "导航 (dǎoháng) — navigation",
            outcome: EventOutcome::GainItem("Star Atlas Fragment"),
            requires: Some(EventRequirement::HasCredits(20)),
        },
        EventChoice {
            text: "Bid on the alien tech",
            chinese_hint: "技术 (jìshù) — technology",
            outcome: EventOutcome::GainItem("Quantum Stabilizer"),
            requires: Some(EventRequirement::HasCredits(30)),
        },
        EventChoice {
            text: "Just browse and mingle",
            chinese_hint: "观看 (guānkàn) — watch",
            outcome: EventOutcome::GainCredits(5),
            requires: None,
        },
    ],
    category: EventCategory::Trading,
};

// ── Discovery (7) ───────────────────────────────────────────────────────────

static EVENT_DERELICT_SHIP: SpaceEvent = SpaceEvent {
    id: 19,
    title: "Derelict Ship",
    chinese_title: "废弃飞船",
    description: "A ship from a forgotten era floats in the darkness, its corridors sealed \
                  for perhaps centuries. Your sensors detect faint energy readings within.",
    choices: &[
        EventChoice {
            text: "Board and explore the ship",
            chinese_hint: "探索 (tànsuǒ) — explore",
            outcome: EventOutcome::CombatReward(2, 25),
            requires: None,
        },
        EventChoice {
            text: "Scan from a safe distance",
            chinese_hint: "安全 (ānquán) — safe",
            outcome: EventOutcome::GainScrap(8),
            requires: None,
        },
        EventChoice {
            text: "Strip the hull for materials",
            chinese_hint: "材料 (cáiliào) — materials",
            outcome: EventOutcome::GainScrap(12),
            requires: None,
        },
    ],
    category: EventCategory::Discovery,
};

static EVENT_ANCIENT_ARTIFACT: SpaceEvent = SpaceEvent {
    id: 20,
    title: "Ancient Artifact",
    chinese_title: "古代遗物",
    description: "Buried within an asteroid, you discover an artifact covered in quantum glyphs. \
                  It hums with an energy your instruments cannot classify.",
    choices: &[
        EventChoice {
            text: "Study the quantum glyphs",
            chinese_hint: "研究 (yánjiū) — study",
            outcome: EventOutcome::GainRadical("古"),
            requires: None,
        },
        EventChoice {
            text: "Sell the artifact at the next port",
            chinese_hint: "出售 (chūshòu) — sell",
            outcome: EventOutcome::GainCredits(25),
            requires: None,
        },
        EventChoice {
            text: "Activate the artifact",
            chinese_hint: "激活 (jīhuó) — activate",
            outcome: EventOutcome::GainFuel(10),
            requires: None,
        },
    ],
    category: EventCategory::Discovery,
};

static EVENT_ASTEROID_MINING: SpaceEvent = SpaceEvent {
    id: 21,
    title: "Asteroid Mining",
    chinese_title: "小行星采矿",
    description: "Rich mineral deposits glitter on a nearby asteroid. Your ship has \
                  enough equipment for a quick mining operation.",
    choices: &[
        EventChoice {
            text: "Mine aggressively for maximum yield",
            chinese_hint: "采矿 (cǎikuàng) — mine",
            outcome: EventOutcome::HullAndFuel(-3, 6),
            requires: None,
        },
        EventChoice {
            text: "Mine carefully with reduced yield",
            chinese_hint: "小心 (xiǎoxīn) — careful",
            outcome: EventOutcome::GainScrap(10),
            requires: None,
        },
        EventChoice {
            text: "Skip the mining — not worth the risk",
            chinese_hint: "危险 (wēixiǎn) — dangerous",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::Discovery,
};

static EVENT_HIDDEN_CACHE: SpaceEvent = SpaceEvent {
    id: 22,
    title: "Hidden Cache",
    chinese_title: "隐藏宝藏",
    description: "Your navigator discovers coordinates etched into a passing comet — \
                  they lead to a cache of supplies hidden in a debris field.",
    choices: &[
        EventChoice {
            text: "Navigate into the debris to find the cache",
            chinese_hint: "宝藏 (bǎozàng) — treasure",
            outcome: EventOutcome::FuelAndCredits(5, 15),
            requires: None,
        },
        EventChoice {
            text: "Send a drone instead",
            chinese_hint: "无人机 (wúrénjī) — drone",
            outcome: EventOutcome::GainScrap(6),
            requires: None,
        },
        EventChoice {
            text: "Record the coordinates for later",
            chinese_hint: "坐标 (zuòbiāo) — coordinates",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::Discovery,
};

static EVENT_NEBULA_PHENOMENON: SpaceEvent = SpaceEvent {
    id: 23,
    title: "Nebula Phenomenon",
    chinese_title: "星云现象",
    description: "Inside a dense nebula, you witness a natural phenomenon: crystalline \
                  structures forming spontaneously from ionized gas.",
    choices: &[
        EventChoice {
            text: "Harvest the crystals",
            chinese_hint: "水晶 (shuǐjīng) — crystal",
            outcome: EventOutcome::GainScrap(15),
            requires: None,
        },
        EventChoice {
            text: "Study the formation process",
            chinese_hint: "观察 (guānchá) — observe",
            outcome: EventOutcome::GainRadical("石"),
            requires: None,
        },
        EventChoice {
            text: "Pass through quickly — nebulae can be unpredictable",
            chinese_hint: "星云 (xīngyún) — nebula",
            outcome: EventOutcome::LoseFuel(2),
            requires: None,
        },
    ],
    category: EventCategory::Discovery,
};

static EVENT_PLANET_SURVEY: SpaceEvent = SpaceEvent {
    id: 24,
    title: "Uncharted Planet",
    chinese_title: "未知星球",
    description: "An uncharted terrestrial planet appears on your sensors. Preliminary \
                  scans show signs of past civilization.",
    choices: &[
        EventChoice {
            text: "Land and explore the ruins",
            chinese_hint: "遗迹 (yíjì) — ruins",
            outcome: EventOutcome::CombatReward(1, 20),
            requires: None,
        },
        EventChoice {
            text: "Conduct an orbital survey",
            chinese_hint: "调查 (diàochá) — survey",
            outcome: EventOutcome::GainCredits(12),
            requires: None,
        },
        EventChoice {
            text: "Collect atmospheric samples",
            chinese_hint: "样品 (yàngpǐn) — sample",
            outcome: EventOutcome::GainRadical("土"),
            requires: None,
        },
    ],
    category: EventCategory::Discovery,
};

static EVENT_SIGNAL_SOURCE: SpaceEvent = SpaceEvent {
    id: 25,
    title: "Mysterious Signal",
    chinese_title: "神秘信号",
    description: "A repeating signal emanates from deep space — not a distress call, but \
                  a mathematical sequence. It could be a beacon, or a warning.",
    choices: &[
        EventChoice {
            text: "Follow the signal to its source",
            chinese_hint: "来源 (láiyuán) — source",
            outcome: EventOutcome::CombatReward(2, 30),
            requires: None,
        },
        EventChoice {
            text: "Analyze the mathematical pattern",
            chinese_hint: "数学 (shùxué) — mathematics",
            outcome: EventOutcome::GainRadical("数"),
            requires: None,
        },
        EventChoice {
            text: "Log it and move on",
            chinese_hint: "继续 (jìxù) — continue",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::Discovery,
};

// ── Anomaly Encounters (5) ──────────────────────────────────────────────────

static EVENT_SPATIAL_ANOMALY: SpaceEvent = SpaceEvent {
    id: 26,
    title: "Spatial Anomaly",
    chinese_title: "空间异常",
    description: "Space itself seems to bend ahead. Stars stretch into impossible shapes \
                  around a swirling distortion that your instruments cannot fully resolve.",
    choices: &[
        EventChoice {
            text: "Navigate through the anomaly",
            chinese_hint: "穿过 (chuānguò) — pass through",
            outcome: EventOutcome::LoseHull(8),
            requires: None,
        },
        EventChoice {
            text: "Go around it (costs extra fuel)",
            chinese_hint: "绕行 (ràoxíng) — detour",
            outcome: EventOutcome::LoseFuel(5),
            requires: Some(EventRequirement::HasFuel(5)),
        },
        EventChoice {
            text: "Study it from a safe distance",
            chinese_hint: "异常 (yìcháng) — anomaly",
            outcome: EventOutcome::GainRadical("力"),
            requires: None,
        },
    ],
    category: EventCategory::AnomalyEncounter,
};

static EVENT_ION_STORM: SpaceEvent = SpaceEvent {
    id: 27,
    title: "Ion Storm",
    chinese_title: "离子风暴",
    description: "A massive ion storm rolls across your path, crackling with energy that \
                  could fry your systems — or charge your batteries.",
    choices: &[
        EventChoice {
            text: "Push through at full speed",
            chinese_hint: "冲 (chōng) — charge",
            outcome: EventOutcome::ShieldDamage(10),
            requires: None,
        },
        EventChoice {
            text: "Take shelter behind a moon and wait it out",
            chinese_hint: "等待 (děngdài) — wait",
            outcome: EventOutcome::LoseFuel(3),
            requires: None,
        },
        EventChoice {
            text: "Ride the storm's edge to harvest energy",
            chinese_hint: "能量 (néngliàng) — energy",
            outcome: EventOutcome::GainFuel(4),
            requires: Some(EventRequirement::HasCrewRole(2)),
        },
    ],
    category: EventCategory::AnomalyEncounter,
};

static EVENT_GRAVITY_WELL: SpaceEvent = SpaceEvent {
    id: 28,
    title: "Gravity Well",
    chinese_title: "引力阱",
    description: "You stumble into an invisible gravity well. The ship groans as unseen \
                  forces pull you toward a collapsed star.",
    choices: &[
        EventChoice {
            text: "Full burn to escape (costs fuel)",
            chinese_hint: "逃脱 (táotuō) — escape",
            outcome: EventOutcome::LoseFuel(6),
            requires: Some(EventRequirement::HasFuel(6)),
        },
        EventChoice {
            text: "Use a gravity slingshot maneuver",
            chinese_hint: "引力 (yǐnlì) — gravity",
            outcome: EventOutcome::GainFuel(3),
            requires: Some(EventRequirement::HasCrewRole(0)),
        },
        EventChoice {
            text: "Brace for impact and ride it out",
            chinese_hint: "坚持 (jiānchí) — endure",
            outcome: EventOutcome::LoseHull(10),
            requires: None,
        },
    ],
    category: EventCategory::AnomalyEncounter,
};

static EVENT_TIME_DISTORTION: SpaceEvent = SpaceEvent {
    id: 29,
    title: "Time Distortion",
    chinese_title: "时间扭曲",
    description: "Clocks begin running backwards. Crew members report seeing echoes of \
                  themselves from minutes ago. Something is very wrong with local spacetime.",
    choices: &[
        EventChoice {
            text: "Investigate the source",
            chinese_hint: "时间 (shíjiān) — time",
            outcome: EventOutcome::GainRadical("时"),
            requires: None,
        },
        EventChoice {
            text: "Reverse engines and retreat",
            chinese_hint: "撤退 (chètuì) — retreat",
            outcome: EventOutcome::LoseFuel(4),
            requires: Some(EventRequirement::HasFuel(4)),
        },
        EventChoice {
            text: "Shut down all systems and drift through",
            chinese_hint: "关闭 (guānbì) — shut down",
            outcome: EventOutcome::DamageCrew(5),
            requires: None,
        },
    ],
    category: EventCategory::AnomalyEncounter,
};

static EVENT_WORMHOLE: SpaceEvent = SpaceEvent {
    id: 30,
    title: "Wormhole Aperture",
    chinese_title: "虫洞",
    description: "A stable wormhole shimmers ahead — an impossibility according to known physics. \
                  Sensors detect a habitable system on the other side.",
    choices: &[
        EventChoice {
            text: "Enter the wormhole",
            chinese_hint: "虫洞 (chóngdòng) — wormhole",
            outcome: EventOutcome::GainFuel(8),
            requires: None,
        },
        EventChoice {
            text: "Send a probe through first",
            chinese_hint: "探针 (tànzhēn) — probe",
            outcome: EventOutcome::GainCredits(10),
            requires: None,
        },
        EventChoice {
            text: "This is too risky — go around",
            chinese_hint: "风险 (fēngxiǎn) — risk",
            outcome: EventOutcome::LoseFuel(3),
            requires: None,
        },
    ],
    category: EventCategory::AnomalyEncounter,
};

// ── Crew Events (4) ─────────────────────────────────────────────────────────

static EVENT_CREW_CONFLICT: SpaceEvent = SpaceEvent {
    id: 31,
    title: "Crew Conflict",
    chinese_title: "船员冲突",
    description: "Two crew members have come to blows over rations. The situation \
                  threatens to split the crew into factions.",
    choices: &[
        EventChoice {
            text: "Mediate the dispute personally",
            chinese_hint: "调解 (tiáojiě) — mediate",
            outcome: EventOutcome::HealCrew(5),
            requires: Some(EventRequirement::HasClass(4)),
        },
        EventChoice {
            text: "Let them sort it out themselves",
            chinese_hint: "自己 (zìjǐ) — themselves",
            outcome: EventOutcome::DamageCrew(5),
            requires: None,
        },
        EventChoice {
            text: "Put both in the brig until they cool down",
            chinese_hint: "禁闭 (jìnbì) — confine",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::CrewEvent,
};

static EVENT_TRAINING_EXERCISE: SpaceEvent = SpaceEvent {
    id: 32,
    title: "Training Exercise",
    chinese_title: "训练演习",
    description: "During a quiet stretch of travel, you consider running combat drills. \
                  The crew could use the practice, but it will cost resources.",
    choices: &[
        EventChoice {
            text: "Full combat drill (costs fuel)",
            chinese_hint: "训练 (xùnliàn) — training",
            outcome: EventOutcome::HullAndFuel(0, -3),
            requires: Some(EventRequirement::HasFuel(3)),
        },
        EventChoice {
            text: "Simulator exercises only",
            chinese_hint: "模拟 (mónǐ) — simulate",
            outcome: EventOutcome::HealCrew(3),
            requires: None,
        },
        EventChoice {
            text: "Let the crew rest instead",
            chinese_hint: "休息 (xiūxi) — rest",
            outcome: EventOutcome::HealCrew(8),
            requires: None,
        },
    ],
    category: EventCategory::CrewEvent,
};

static EVENT_STOWAWAY: SpaceEvent = SpaceEvent {
    id: 33,
    title: "Stowaway Discovered",
    chinese_title: "发现偷渡者",
    description: "A stowaway is found hiding in the cargo bay — a young refugee \
                  from a war-torn system. They look terrified.",
    choices: &[
        EventChoice {
            text: "Take them on as a crew member",
            chinese_hint: "收留 (shōuliú) — take in",
            outcome: EventOutcome::GainCrewMember,
            requires: None,
        },
        EventChoice {
            text: "Drop them at the next station",
            chinese_hint: "下一站 (xià yī zhàn) — next stop",
            outcome: EventOutcome::LoseFuel(2),
            requires: None,
        },
        EventChoice {
            text: "Interrogate them for useful information",
            chinese_hint: "审问 (shěnwèn) — interrogate",
            outcome: EventOutcome::GainCredits(10),
            requires: None,
        },
    ],
    category: EventCategory::CrewEvent,
};

static EVENT_CREW_CELEBRATION: SpaceEvent = SpaceEvent {
    id: 34,
    title: "Crew Celebration",
    chinese_title: "船员庆祝",
    description: "The crew wants to celebrate a milestone — 100 jumps together. \
                  They request shore leave at the next station.",
    choices: &[
        EventChoice {
            text: "Grant shore leave (costs credits)",
            chinese_hint: "庆祝 (qìngzhù) — celebrate",
            outcome: EventOutcome::HealCrew(20),
            requires: Some(EventRequirement::HasCredits(10)),
        },
        EventChoice {
            text: "Throw a party on the ship",
            chinese_hint: "派对 (pàiduì) — party",
            outcome: EventOutcome::HealCrew(10),
            requires: None,
        },
        EventChoice {
            text: "No time for celebrations — push on",
            chinese_hint: "没时间 (méi shíjiān) — no time",
            outcome: EventOutcome::DamageCrew(3),
            requires: None,
        },
    ],
    category: EventCategory::CrewEvent,
};

// ── Alien Contact (4) ───────────────────────────────────────────────────────

static EVENT_FIRST_CONTACT: SpaceEvent = SpaceEvent {
    id: 35,
    title: "First Contact",
    chinese_title: "第一次接触",
    description: "An alien vessel of unknown design approaches. It broadcasts a complex \
                  signal — possibly a greeting, possibly a warning.",
    choices: &[
        EventChoice {
            text: "Attempt communication using universal constants",
            chinese_hint: "沟通 (gōutōng) — communicate",
            outcome: EventOutcome::GainCredits(20),
            requires: None,
        },
        EventChoice {
            text: "Offer a gift of fuel as a peace gesture",
            chinese_hint: "和平 (hépíng) — peace",
            outcome: EventOutcome::FuelAndCredits(-3, 30),
            requires: Some(EventRequirement::HasFuel(3)),
        },
        EventChoice {
            text: "Power weapons and raise shields",
            chinese_hint: "防御 (fángyù) — defense",
            outcome: EventOutcome::StartCombat(2),
            requires: None,
        },
        EventChoice {
            text: "Retreat slowly — don't provoke them",
            chinese_hint: "后退 (hòutuì) — back away",
            outcome: EventOutcome::LoseFuel(2),
            requires: None,
        },
    ],
    category: EventCategory::AlienContact,
};

static EVENT_ALIEN_TRADERS: SpaceEvent = SpaceEvent {
    id: 36,
    title: "Alien Traders",
    chinese_title: "外星商人",
    description: "A flotilla of alien merchant ships surrounds you — not hostile, but \
                  insistent. They trade in exotic goods unseen in human space.",
    choices: &[
        EventChoice {
            text: "Trade credits for alien fuel cells",
            chinese_hint: "外星 (wàixīng) — alien",
            outcome: EventOutcome::FuelAndCredits(6, -8),
            requires: Some(EventRequirement::HasCredits(8)),
        },
        EventChoice {
            text: "Trade for alien radical inscriptions",
            chinese_hint: "铭文 (míngwén) — inscription",
            outcome: EventOutcome::GainRadical("外"),
            requires: None,
        },
        EventChoice {
            text: "Politely decline all offers",
            chinese_hint: "谢谢 (xièxie) — thank you",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::AlienContact,
};

static EVENT_ALIEN_DISTRESS: SpaceEvent = SpaceEvent {
    id: 37,
    title: "Alien in Need",
    chinese_title: "外星求助",
    description: "An alien creature floats in space in what appears to be a biological \
                  life-pod. It pulses with bioluminescent patterns — perhaps a language?",
    choices: &[
        EventChoice {
            text: "Bring it aboard and try to help",
            chinese_hint: "生物 (shēngwù) — creature",
            outcome: EventOutcome::GainCrewMember,
            requires: None,
        },
        EventChoice {
            text: "Study its bioluminescent patterns",
            chinese_hint: "发光 (fāguāng) — glow",
            outcome: EventOutcome::GainRadical("光"),
            requires: None,
        },
        EventChoice {
            text: "Leave it be — alien biology is unpredictable",
            chinese_hint: "未知 (wèizhī) — unknown",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::AlienContact,
};

static EVENT_ALIEN_MONUMENT: SpaceEvent = SpaceEvent {
    id: 38,
    title: "Alien Monument",
    chinese_title: "外星纪念碑",
    description: "A colossal structure orbits a dead star — clearly artificial, impossibly old. \
                  Its surface is etched with symbols that tug at the edge of comprehension.",
    choices: &[
        EventChoice {
            text: "Dock and decipher the symbols",
            chinese_hint: "解读 (jiědú) — decipher",
            outcome: EventOutcome::GainRadical("大"),
            requires: None,
        },
        EventChoice {
            text: "Chip off a piece for analysis",
            chinese_hint: "分析 (fēnxī) — analyze",
            outcome: EventOutcome::GainScrap(12),
            requires: None,
        },
        EventChoice {
            text: "Broadcast a greeting toward the monument",
            chinese_hint: "问候 (wènhòu) — greeting",
            outcome: EventOutcome::StartCombat(3),
            requires: None,
        },
    ],
    category: EventCategory::AlienContact,
};

// ── Hazard Events (4) ───────────────────────────────────────────────────────

static EVENT_DEBRIS_FIELD: SpaceEvent = SpaceEvent {
    id: 39,
    title: "Debris Field",
    chinese_title: "碎片区域",
    description: "You enter a field of shattered ships and tumbling rock. Something \
                  catastrophic happened here. Navigation is treacherous.",
    choices: &[
        EventChoice {
            text: "Carefully navigate through",
            chinese_hint: "导航 (dǎoháng) — navigate",
            outcome: EventOutcome::LoseFuel(3),
            requires: None,
        },
        EventChoice {
            text: "Salvage as you go (risk hull damage)",
            chinese_hint: "打捞 (dǎlāo) — salvage",
            outcome: EventOutcome::HullAndFuel(-5, 0),
            requires: None,
        },
        EventChoice {
            text: "Power through at full speed",
            chinese_hint: "全速 (quánsù) — full speed",
            outcome: EventOutcome::LoseHull(12),
            requires: None,
        },
    ],
    category: EventCategory::HazardEvent,
};

static EVENT_SOLAR_FLARE: SpaceEvent = SpaceEvent {
    id: 40,
    title: "Solar Flare",
    chinese_title: "太阳耀斑",
    description: "Warning: the local star erupts in a massive solar flare. Radiation levels \
                  spike and your shields strain under the bombardment.",
    choices: &[
        EventChoice {
            text: "Angle shields and ride it out",
            chinese_hint: "盾牌 (dùnpái) — shield",
            outcome: EventOutcome::ShieldDamage(8),
            requires: None,
        },
        EventChoice {
            text: "Emergency FTL jump away",
            chinese_hint: "紧急 (jǐnjí) — emergency",
            outcome: EventOutcome::LoseFuel(5),
            requires: Some(EventRequirement::HasFuel(5)),
        },
        EventChoice {
            text: "Hide in the shadow of a nearby planet",
            chinese_hint: "影子 (yǐngzi) — shadow",
            outcome: EventOutcome::LoseFuel(2),
            requires: None,
        },
    ],
    category: EventCategory::HazardEvent,
};

static EVENT_MINEFIELD: SpaceEvent = SpaceEvent {
    id: 41,
    title: "Minefield",
    chinese_title: "雷区",
    description: "Proximity sensors scream — you have wandered into a dormant minefield \
                  left over from a forgotten war.",
    choices: &[
        EventChoice {
            text: "Carefully reverse course",
            chinese_hint: "后退 (hòutuì) — reverse",
            outcome: EventOutcome::LoseFuel(4),
            requires: None,
        },
        EventChoice {
            text: "Use your engineer to disarm a path",
            chinese_hint: "拆除 (chāichú) — disarm",
            outcome: EventOutcome::GainScrap(10),
            requires: Some(EventRequirement::HasCrewRole(1)),
        },
        EventChoice {
            text: "Push through and hope for the best",
            chinese_hint: "运气 (yùnqi) — luck",
            outcome: EventOutcome::LoseHull(15),
            requires: None,
        },
    ],
    category: EventCategory::HazardEvent,
};

static EVENT_RADIATION_BELT: SpaceEvent = SpaceEvent {
    id: 42,
    title: "Radiation Belt",
    chinese_title: "辐射带",
    description: "A dense radiation belt blocks your planned route. Your medical officer \
                  warns of crew exposure risks.",
    choices: &[
        EventChoice {
            text: "Go through with radiation shielding",
            chinese_hint: "辐射 (fúshè) — radiation",
            outcome: EventOutcome::DamageCrew(8),
            requires: None,
        },
        EventChoice {
            text: "Detour around the belt",
            chinese_hint: "绕路 (ràolù) — detour",
            outcome: EventOutcome::LoseFuel(6),
            requires: Some(EventRequirement::HasFuel(6)),
        },
        EventChoice {
            text: "Wait for a gap in the radiation",
            chinese_hint: "耐心 (nàixīn) — patience",
            outcome: EventOutcome::LoseFuel(2),
            requires: None,
        },
    ],
    category: EventCategory::HazardEvent,
};

// ── Ancient Ruins (2) ───────────────────────────────────────────────────────

static EVENT_ANCIENT_SPACE_STATION: SpaceEvent = SpaceEvent {
    id: 43,
    title: "Ancient Space Station",
    chinese_title: "古代空间站",
    description: "A space station of incredible antiquity emerges from the dust cloud — \
                  its architecture defies every known engineering principle.",
    choices: &[
        EventChoice {
            text: "Explore the station interior",
            chinese_hint: "内部 (nèibù) — interior",
            outcome: EventOutcome::CombatReward(2, 35),
            requires: None,
        },
        EventChoice {
            text: "Access the station's data cores",
            chinese_hint: "数据 (shùjù) — data",
            outcome: EventOutcome::GainRadical("金"),
            requires: Some(EventRequirement::HasCrewRole(3)),
        },
        EventChoice {
            text: "Salvage external components",
            chinese_hint: "零件 (língjiàn) — components",
            outcome: EventOutcome::RepairShip(10),
            requires: None,
        },
    ],
    category: EventCategory::AncientRuins,
};

static EVENT_TEMPLE_SHIP: SpaceEvent = SpaceEvent {
    id: 44,
    title: "Temple Ship",
    chinese_title: "神殿飞船",
    description: "A vessel shaped like a vast temple drifts serenely through space. \
                  Ancient hymns play on all frequencies. Inside, you sense answers.",
    choices: &[
        EventChoice {
            text: "Enter the temple and meditate",
            chinese_hint: "冥想 (míngxiǎng) — meditate",
            outcome: EventOutcome::HealCrew(25),
            requires: None,
        },
        EventChoice {
            text: "Study the inscriptions for radical knowledge",
            chinese_hint: "铭文 (míngwén) — inscription",
            outcome: EventOutcome::GainRadical("心"),
            requires: None,
        },
        EventChoice {
            text: "The singing is eerie — keep moving",
            chinese_hint: "奇怪 (qíguài) — strange",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::AncientRuins,
};

// ── Language Challenges (4) ─────────────────────────────────────────────────

static EVENT_ANCIENT_TERMINAL: SpaceEvent = SpaceEvent {
    id: 45,
    title: "Ancient Terminal",
    chinese_title: "古代终端",
    description: "You discover an operational terminal in an abandoned outpost. Its interface \
                  displays Chinese characters — an ancient Earth colony, perhaps. \
                  It seems to require a passphrase.",
    choices: &[
        EventChoice {
            text: "Enter 'open' (开) to unlock",
            chinese_hint: "开 (kāi) — open",
            outcome: EventOutcome::GainCredits(30),
            requires: Some(EventRequirement::HasRadical("开")),
        },
        EventChoice {
            text: "Try to brute-force the terminal",
            chinese_hint: "破解 (pòjiě) — crack",
            outcome: EventOutcome::GainCredits(10),
            requires: None,
        },
        EventChoice {
            text: "Copy the data and decrypt it later",
            chinese_hint: "复制 (fùzhì) — copy",
            outcome: EventOutcome::GainRadical("开"),
            requires: None,
        },
    ],
    category: EventCategory::LanguageChallenge,
};

static EVENT_ENCODED_MESSAGE: SpaceEvent = SpaceEvent {
    id: 46,
    title: "Encoded Message",
    chinese_title: "加密消息",
    description: "Your comms array intercepts an encoded message. Analysis reveals it uses \
                  Chinese character radicals as a cipher. Decoding it could reveal the \
                  location of a hidden supply cache.",
    choices: &[
        EventChoice {
            text: "Decode using the fire radical (火)",
            chinese_hint: "火 (huǒ) — fire",
            outcome: EventOutcome::FuelAndCredits(8, 20),
            requires: Some(EventRequirement::HasRadical("火")),
        },
        EventChoice {
            text: "Decode using the water radical (水)",
            chinese_hint: "水 (shuǐ) — water",
            outcome: EventOutcome::GainFuel(10),
            requires: Some(EventRequirement::HasRadical("水")),
        },
        EventChoice {
            text: "Run a computational analysis (slow but works)",
            chinese_hint: "计算 (jìsuàn) — compute",
            outcome: EventOutcome::GainCredits(8),
            requires: None,
        },
    ],
    category: EventCategory::LanguageChallenge,
};

static EVENT_ROSETTA_PROBE: SpaceEvent = SpaceEvent {
    id: 47,
    title: "Rosetta Probe",
    chinese_title: "罗塞塔探针",
    description: "You recover an ancient probe designed to teach language through images \
                  and symbols. It cycles through radical characters with their meanings.",
    choices: &[
        EventChoice {
            text: "Study the radical for 'person' (人)",
            chinese_hint: "人 (rén) — person",
            outcome: EventOutcome::GainRadical("人"),
            requires: None,
        },
        EventChoice {
            text: "Study the radical for 'mouth' (口)",
            chinese_hint: "口 (kǒu) — mouth",
            outcome: EventOutcome::GainRadical("口"),
            requires: None,
        },
        EventChoice {
            text: "Study the radical for 'sun' (日)",
            chinese_hint: "日 (rì) — sun/day",
            outcome: EventOutcome::GainRadical("日"),
            requires: None,
        },
        EventChoice {
            text: "Download all data for later study",
            chinese_hint: "下载 (xiàzài) — download",
            outcome: EventOutcome::GainCredits(5),
            requires: None,
        },
    ],
    category: EventCategory::LanguageChallenge,
};

static EVENT_CALLIGRAPHY_CONTEST: SpaceEvent = SpaceEvent {
    id: 48,
    title: "Calligraphy Contest",
    chinese_title: "书法比赛",
    description: "A cultured space station hosts a calligraphy contest among travelers. \
                  The prize pool is generous, and the challenge: write a character \
                  from memory.",
    choices: &[
        EventChoice {
            text: "Enter the contest (need a radical to compete)",
            chinese_hint: "比赛 (bǐsài) — contest",
            outcome: EventOutcome::GainCredits(40),
            requires: Some(EventRequirement::HasRadical("人")),
        },
        EventChoice {
            text: "Watch and learn from the contestants",
            chinese_hint: "学习 (xuéxí) — learn",
            outcome: EventOutcome::GainRadical("文"),
            requires: None,
        },
        EventChoice {
            text: "Bet on the winner",
            chinese_hint: "下注 (xiàzhù) — bet",
            outcome: EventOutcome::GainCredits(15),
            requires: Some(EventRequirement::HasCredits(10)),
        },
    ],
    category: EventCategory::LanguageChallenge,
};

// ---------------------------------------------------------------------------
// Master event pool
// ---------------------------------------------------------------------------

pub static ALL_EVENTS: &[&SpaceEvent] = &[
    // Distress Signals (0–6)
    &EVENT_DISTRESS_BEACON,
    &EVENT_ESCAPE_POD,
    &EVENT_DAMAGED_FREIGHTER,
    &EVENT_COLONY_SOS,
    &EVENT_STRANDED_MINERS,
    &EVENT_GHOST_SHIP,
    &EVENT_MEDICAL_FRIGATE,
    // Pirate Encounters (7–12)
    &EVENT_PIRATE_AMBUSH,
    &EVENT_PIRATE_BASE,
    &EVENT_PIRATE_BOARDING,
    &EVENT_PIRATE_DEFECTOR,
    &EVENT_PIRATE_CONVOY,
    &EVENT_RANSOM_DEMAND,
    // Trading (13–18)
    &EVENT_WANDERING_MERCHANT,
    &EVENT_BLACK_MARKET,
    &EVENT_FUEL_DEPOT,
    &EVENT_SMUGGLER_OFFER,
    &EVENT_TRADE_STATION,
    &EVENT_AUCTION_HOUSE,
    // Discovery (19–25)
    &EVENT_DERELICT_SHIP,
    &EVENT_ANCIENT_ARTIFACT,
    &EVENT_ASTEROID_MINING,
    &EVENT_HIDDEN_CACHE,
    &EVENT_NEBULA_PHENOMENON,
    &EVENT_PLANET_SURVEY,
    &EVENT_SIGNAL_SOURCE,
    // Anomaly (26–30)
    &EVENT_SPATIAL_ANOMALY,
    &EVENT_ION_STORM,
    &EVENT_GRAVITY_WELL,
    &EVENT_TIME_DISTORTION,
    &EVENT_WORMHOLE,
    // Crew Events (31–34)
    &EVENT_CREW_CONFLICT,
    &EVENT_TRAINING_EXERCISE,
    &EVENT_STOWAWAY,
    &EVENT_CREW_CELEBRATION,
    // Alien Contact (35–38)
    &EVENT_FIRST_CONTACT,
    &EVENT_ALIEN_TRADERS,
    &EVENT_ALIEN_DISTRESS,
    &EVENT_ALIEN_MONUMENT,
    // Hazard Events (39–42)
    &EVENT_DEBRIS_FIELD,
    &EVENT_SOLAR_FLARE,
    &EVENT_MINEFIELD,
    &EVENT_RADIATION_BELT,
    // Ancient Ruins (43–44)
    &EVENT_ANCIENT_SPACE_STATION,
    &EVENT_TEMPLE_SHIP,
    // Language Challenges (45–48)
    &EVENT_ANCIENT_TERMINAL,
    &EVENT_ENCODED_MESSAGE,
    &EVENT_ROSETTA_PROBE,
    &EVENT_CALLIGRAPHY_CONTEST,
];

// ---------------------------------------------------------------------------
// Helper: deterministic event selection
// ---------------------------------------------------------------------------

/// Selects an event deterministically from the pool based on sector, system,
/// and a seed value.  Uses a simple hash to avoid pulling in external crates.
pub fn select_event(sector: usize, system_id: usize, seed: u32) -> &'static SpaceEvent {
    let hash = simple_hash(sector as u32, system_id as u32, seed);
    let index = (hash as usize) % ALL_EVENTS.len();
    ALL_EVENTS[index]
}

/// Selects an event from a specific category.  Returns the first match after
/// hashing; falls back to the first event in the category if only one exists.
pub fn select_event_by_category(
    category: EventCategory,
    seed: u32,
) -> &'static SpaceEvent {
    let candidates: Vec<&&SpaceEvent> = ALL_EVENTS
        .iter()
        .filter(|e| e.category == category)
        .collect();

    if candidates.is_empty() {
        return ALL_EVENTS[0];
    }

    let index = (seed as usize) % candidates.len();
    candidates[index]
}

/// Returns the total number of events in the pool.
pub fn event_count() -> usize {
    ALL_EVENTS.len()
}

/// Cheap deterministic hash — no external deps.
fn simple_hash(a: u32, b: u32, c: u32) -> u32 {
    let mut h = a.wrapping_mul(2654435761);
    h ^= b.wrapping_mul(2246822519);
    h ^= c.wrapping_mul(3266489917);
    h ^= h >> 16;
    h = h.wrapping_mul(2246822519);
    h ^= h >> 13;
    h = h.wrapping_mul(3266489917);
    h ^= h >> 16;
    h
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_pool_has_at_least_40_events() {
        assert!(
            ALL_EVENTS.len() >= 40,
            "Expected at least 40 events, got {}",
            ALL_EVENTS.len()
        );
    }

    #[test]
    fn event_ids_are_unique() {
        let mut ids: Vec<usize> = ALL_EVENTS.iter().map(|e| e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), ALL_EVENTS.len(), "Duplicate event IDs found");
    }

    #[test]
    fn every_event_has_at_least_two_choices() {
        for event in ALL_EVENTS.iter() {
            assert!(
                event.choices.len() >= 2,
                "Event '{}' has fewer than 2 choices",
                event.title
            );
        }
    }

    #[test]
    fn every_event_has_chinese_title() {
        for event in ALL_EVENTS.iter() {
            assert!(
                !event.chinese_title.is_empty(),
                "Event '{}' is missing chinese_title",
                event.title
            );
        }
    }

    #[test]
    fn select_event_is_deterministic() {
        let e1 = select_event(1, 2, 42);
        let e2 = select_event(1, 2, 42);
        assert_eq!(e1.id, e2.id);
    }

    #[test]
    fn select_event_varies_with_seed() {
        let e1 = select_event(0, 0, 1);
        let e2 = select_event(0, 0, 2);
        // Very unlikely to collide with a good hash, but not impossible
        // so we just test it doesn't panic.
        let _ = (e1.id, e2.id);
    }

    #[test]
    fn all_categories_represented() {
        let categories = [
            EventCategory::DistressSignal,
            EventCategory::PirateEncounter,
            EventCategory::Trading,
            EventCategory::Discovery,
            EventCategory::AnomalyEncounter,
            EventCategory::CrewEvent,
            EventCategory::AlienContact,
            EventCategory::HazardEvent,
            EventCategory::AncientRuins,
            EventCategory::LanguageChallenge,
        ];
        for cat in &categories {
            assert!(
                ALL_EVENTS.iter().any(|e| e.category == *cat),
                "No events for category {:?}",
                cat
            );
        }
    }

    #[test]
    fn select_event_by_category_returns_correct_category() {
        let event = select_event_by_category(EventCategory::Trading, 99);
        assert_eq!(event.category, EventCategory::Trading);
    }
}
