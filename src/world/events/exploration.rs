//! Exploration events: trading, discovery, anomalies, ancient ruins, language.

use super::types::*;

// ── Trading (6) ─────────────────────────────────────────────────────────────

pub(super) static EVENT_WANDERING_MERCHANT: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_BLACK_MARKET: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_FUEL_DEPOT: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_SMUGGLER_OFFER: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_TRADE_STATION: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_AUCTION_HOUSE: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_DERELICT_SHIP: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_ANCIENT_ARTIFACT: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_ASTEROID_MINING: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_HIDDEN_CACHE: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_NEBULA_PHENOMENON: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_PLANET_SURVEY: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_SIGNAL_SOURCE: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_SPATIAL_ANOMALY: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_ION_STORM: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_GRAVITY_WELL: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_TIME_DISTORTION: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_WORMHOLE: SpaceEvent = SpaceEvent {
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


// ── Ancient Ruins (2) ───────────────────────────────────────────────────────

pub(super) static EVENT_ANCIENT_SPACE_STATION: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_TEMPLE_SHIP: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_ANCIENT_TERMINAL: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_ENCODED_MESSAGE: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_ROSETTA_PROBE: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_CALLIGRAPHY_CONTEST: SpaceEvent = SpaceEvent {
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

