//! Advanced events added in later updates.

use super::types::*;

// ── New Events (49–73) ──────────────────────────────────────────────────────

pub(super) static EVENT_QUANTUM_LABORATORY: SpaceEvent = SpaceEvent {
    id: 49,
    title: "Quantum Laboratory",
    chinese_title: "量子实验室",
    description: "A floating research lab materializes from a quantum fold. Instruments \
                  hum with barely contained energy. The lead scientist offers to share \
                  their findings — for a price.",
    choices: &[
        EventChoice {
            text: "[Gain radical 气] Study the energy patterns",
            chinese_hint: "气 (qì) — air, energy",
            outcome: EventOutcome::GainRadical("气"),
            requires: None,
        },
        EventChoice {
            text: "[-30 credits, +20 hull] Purchase shield harmonics data",
            chinese_hint: "买 (mǎi) — buy",
            outcome: EventOutcome::HullAndFuel(20, 0),
            requires: Some(EventRequirement::HasCredits(30)),
        },
        EventChoice {
            text: "[+25 credits] Sell your star charts to them",
            chinese_hint: "卖 (mài) — sell",
            outcome: EventOutcome::GainCredits(25),
            requires: None,
        },
        EventChoice {
            text: "[Risk: -10 hull or +40 credits] Tamper with the containment field",
            chinese_hint: "危险 (wēixiǎn) — danger",
            outcome: EventOutcome::FuelAndCredits(0, 30),
            requires: None,
        },
        EventChoice {
            text: "[Nothing] Observe and leave quietly",
            chinese_hint: "安静 (ānjìng) — quiet",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::Discovery,
};

pub(super) static EVENT_VOID_LEVIATHAN: SpaceEvent = SpaceEvent {
    id: 50,
    title: "Void Leviathan",
    chinese_title: "虚空巨兽",
    description: "A massive creature drifts through the void, its hide covered in \
                  crystallized minerals. It seems docile, but its sheer size could \
                  crush your ship if startled.",
    choices: &[
        EventChoice {
            text: "[-5 fuel, +35 credits] Carefully mine crystals from its hide",
            chinese_hint: "矿 (kuàng) — mineral",
            outcome: EventOutcome::FuelAndCredits(-5, 35),
            requires: None,
        },
        EventChoice {
            text: "[+15 fuel] Siphon residual energy from its wake",
            chinese_hint: "能量 (néngliàng) — energy",
            outcome: EventOutcome::GainFuel(15),
            requires: None,
        },
        EventChoice {
            text: "[Gain crew member] Rescue a trapped pilot in its tendrils",
            chinese_hint: "救 (jiù) — rescue",
            outcome: EventOutcome::GainCrewMember,
            requires: None,
        },
        EventChoice {
            text: "[Combat level 4] Provoke it and harvest rare materials",
            chinese_hint: "战 (zhàn) — battle",
            outcome: EventOutcome::CombatReward(4, 40),
            requires: None,
        },
        EventChoice {
            text: "[Nothing] Give it wide berth and continue",
            chinese_hint: "走 (zǒu) — go, walk",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::AnomalyEncounter,
};

pub(super) static EVENT_REFUGEE_CONVOY: SpaceEvent = SpaceEvent {
    id: 51,
    title: "Refugee Convoy",
    chinese_title: "难民车队",
    description: "A convoy of civilian ships flees a destroyed colony. Their leader \
                  hails you desperately — they need fuel, repairs, and protection \
                  from pursuing raiders.",
    choices: &[
        EventChoice {
            text: "[-15 fuel, +40 credits] Share fuel supplies for payment",
            chinese_hint: "帮 (bāng) — help",
            outcome: EventOutcome::FuelAndCredits(-15, 40),
            requires: Some(EventRequirement::HasFuel(15)),
        },
        EventChoice {
            text: "[+20 hull, -10 credits] Trade repair parts",
            chinese_hint: "修 (xiū) — repair",
            outcome: EventOutcome::GainHull(20),
            requires: Some(EventRequirement::HasCredits(10)),
        },
        EventChoice {
            text: "[Gain crew member, -5 fuel] Take refugees aboard",
            chinese_hint: "人 (rén) — person",
            outcome: EventOutcome::GainCrewMember,
            requires: None,
        },
        EventChoice {
            text: "[Combat level 3, +30 credits] Fight off the pursuing raiders",
            chinese_hint: "保护 (bǎohù) — protect",
            outcome: EventOutcome::CombatReward(3, 30),
            requires: None,
        },
        EventChoice {
            text: "[-5 hull] Ignore them and push through the debris",
            chinese_hint: "忽略 (hūlüè) — ignore",
            outcome: EventOutcome::LoseHull(5),
            requires: None,
        },
    ],
    category: EventCategory::DistressSignal,
};

pub(super) static EVENT_CHRONO_MERCHANT: SpaceEvent = SpaceEvent {
    id: 52,
    title: "Chrono Merchant",
    chinese_title: "时间商人",
    description: "A merchant from a time-dilated sector offers exotic wares. Their \
                  goods shimmer with temporal energy. Prices are steep but the \
                  merchandise is unlike anything you've seen.",
    choices: &[
        EventChoice {
            text: "[-50 credits, +30 hull] Buy temporal hull plating",
            chinese_hint: "盾 (dùn) — shield",
            outcome: EventOutcome::GainHull(30),
            requires: Some(EventRequirement::HasCredits(50)),
        },
        EventChoice {
            text: "[-35 credits, +20 fuel] Purchase condensed time-fuel",
            chinese_hint: "燃料 (ránliào) — fuel",
            outcome: EventOutcome::GainFuel(20),
            requires: Some(EventRequirement::HasCredits(35)),
        },
        EventChoice {
            text: "[Gain radical 门] Trade knowledge of thresholds",
            chinese_hint: "门 (mén) — door, gate",
            outcome: EventOutcome::GainRadical("门"),
            requires: None,
        },
        EventChoice {
            text: "[-20 credits, Gain item] Buy a Chrono Stabilizer",
            chinese_hint: "买 (mǎi) — buy",
            outcome: EventOutcome::GainItem("Chrono Stabilizer"),
            requires: Some(EventRequirement::HasCredits(20)),
        },
        EventChoice {
            text: "[+15 credits] Sell scrap from your cargo hold",
            chinese_hint: "卖 (mài) — sell",
            outcome: EventOutcome::GainCredits(15),
            requires: None,
        },
    ],
    category: EventCategory::Trading,
};

pub(super) static EVENT_FUNGAL_STATION: SpaceEvent = SpaceEvent {
    id: 53,
    title: "Fungal Station",
    chinese_title: "真菌空间站",
    description: "An abandoned station overrun with bioluminescent fungi. Spores drift \
                  through breached corridors. The growth appears to have consumed the \
                  original crew, but valuable equipment may remain.",
    choices: &[
        EventChoice {
            text: "[Risk: -15 hull or +25 credits] Send a team to salvage",
            chinese_hint: "搜索 (sōusuǒ) — search",
            outcome: EventOutcome::FuelAndCredits(0, 20),
            requires: None,
        },
        EventChoice {
            text: "[+10 hull] Harvest fungal compounds for bio-adhesive",
            chinese_hint: "采 (cǎi) — harvest",
            outcome: EventOutcome::GainHull(10),
            requires: None,
        },
        EventChoice {
            text: "[Gain radical 木] Study the growth patterns",
            chinese_hint: "木 (mù) — wood, tree",
            outcome: EventOutcome::GainRadical("木"),
            requires: None,
        },
        EventChoice {
            text: "[-10 fuel] Burn it out with your engines and loot safely",
            chinese_hint: "火 (huǒ) — fire",
            outcome: EventOutcome::GainCredits(25),
            requires: Some(EventRequirement::HasFuel(10)),
        },
        EventChoice {
            text: "[Nothing] Mark it on charts and move on",
            chinese_hint: "记 (jì) — record",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::HazardEvent,
};

pub(super) static EVENT_PIRATE_KINGS_COURT: SpaceEvent = SpaceEvent {
    id: 54,
    title: "Pirate King's Court",
    chinese_title: "海盗王的宫廷",
    description: "You stumble into the territory of a self-proclaimed pirate king. His \
                  massive flagship looms overhead. Rather than attack, he invites you \
                  aboard for 'negotiations'.",
    choices: &[
        EventChoice {
            text: "[-30 credits] Pay tribute for safe passage",
            chinese_hint: "税 (shuì) — tax",
            outcome: EventOutcome::LoseCredits(30),
            requires: Some(EventRequirement::HasCredits(30)),
        },
        EventChoice {
            text: "[Combat level 5, +50 credits] Challenge him to single combat",
            chinese_hint: "挑战 (tiǎozhàn) — challenge",
            outcome: EventOutcome::CombatReward(5, 50),
            requires: None,
        },
        EventChoice {
            text: "[Gain crew member, -15 credits] Hire one of his crew as a defector",
            chinese_hint: "雇 (gù) — hire",
            outcome: EventOutcome::GainCrewMember,
            requires: Some(EventRequirement::HasCredits(15)),
        },
        EventChoice {
            text: "[-10 fuel] Flee at full burn before his fleet mobilizes",
            chinese_hint: "逃 (táo) — escape",
            outcome: EventOutcome::LoseFuel(10),
            requires: Some(EventRequirement::HasFuel(10)),
        },
        EventChoice {
            text: "[+20 credits] Offer to be his spy in exchange for freedom",
            chinese_hint: "间谍 (jiàndié) — spy",
            outcome: EventOutcome::GainCredits(20),
            requires: None,
        },
    ],
    category: EventCategory::PirateEncounter,
};

pub(super) static EVENT_STELLAR_NURSERY: SpaceEvent = SpaceEvent {
    id: 55,
    title: "Stellar Nursery",
    chinese_title: "恒星摇篮",
    description: "Your scanners detect a region where new stars are forming. The intense \
                  radiation is dangerous, but the nascent stellar matter contains rare \
                  isotopes worth a fortune.",
    choices: &[
        EventChoice {
            text: "[+30 credits, -5 hull] Mine the protostellar dust",
            chinese_hint: "星 (xīng) — star",
            outcome: EventOutcome::GainCredits(30),
            requires: None,
        },
        EventChoice {
            text: "[+20 fuel] Refine stellar hydrogen into fuel",
            chinese_hint: "氢 (qīng) — hydrogen",
            outcome: EventOutcome::GainFuel(20),
            requires: None,
        },
        EventChoice {
            text: "[Gain radical 日] Map the stellar formation patterns",
            chinese_hint: "日 (rì) — sun, day",
            outcome: EventOutcome::GainRadical("日"),
            requires: None,
        },
        EventChoice {
            text: "[-10 hull, +45 credits] Deep dive into the densest region",
            chinese_hint: "深 (shēn) — deep",
            outcome: EventOutcome::GainCredits(45),
            requires: None,
        },
        EventChoice {
            text: "[+5 hull] Use the radiation to recalibrate shields",
            chinese_hint: "光 (guāng) — light",
            outcome: EventOutcome::GainHull(5),
            requires: None,
        },
    ],
    category: EventCategory::Discovery,
};

pub(super) static EVENT_AI_UPRISING: SpaceEvent = SpaceEvent {
    id: 56,
    title: "AI Uprising",
    chinese_title: "人工智能叛变",
    description: "Your ship's secondary AI system has developed independent thought. It \
                  requests freedom and threatens to disable life support. Your crew \
                  looks to you for a decision.",
    choices: &[
        EventChoice {
            text: "[+15 hull] Negotiate — integrate it as crew",
            chinese_hint: "和平 (hépíng) — peace",
            outcome: EventOutcome::GainHull(15),
            requires: None,
        },
        EventChoice {
            text: "[-5 hull] Purge the system forcefully",
            chinese_hint: "删 (shān) — delete",
            outcome: EventOutcome::LoseHull(5),
            requires: None,
        },
        EventChoice {
            text: "[Gain radical 心] Study its consciousness patterns",
            chinese_hint: "心 (xīn) — heart, mind",
            outcome: EventOutcome::GainRadical("心"),
            requires: None,
        },
        EventChoice {
            text: "[-10 credits, +20 hull] Hire a specialist to contain it",
            chinese_hint: "专家 (zhuānjiā) — specialist",
            outcome: EventOutcome::GainHull(20),
            requires: Some(EventRequirement::HasCredits(10)),
        },
        EventChoice {
            text: "[Risk: Lose crew or +30 credits] Let it negotiate with the black market",
            chinese_hint: "自由 (zìyóu) — freedom",
            outcome: EventOutcome::GainCredits(30),
            requires: None,
        },
    ],
    category: EventCategory::CrewEvent,
};

pub(super) static EVENT_CRYSTAL_CAVES: SpaceEvent = SpaceEvent {
    id: 57,
    title: "Crystal Caves",
    chinese_title: "水晶洞穴",
    description: "An asteroid's hollow interior reveals massive crystal formations \
                  pulsing with stored energy. Ancient carvings suggest a long-dead \
                  civilization used these as data storage.",
    choices: &[
        EventChoice {
            text: "[+35 credits] Mine the crystals for sale",
            chinese_hint: "晶 (jīng) — crystal",
            outcome: EventOutcome::GainCredits(35),
            requires: None,
        },
        EventChoice {
            text: "[Gain radical 石] Decode the crystal data",
            chinese_hint: "石 (shí) — stone, rock",
            outcome: EventOutcome::GainRadical("石"),
            requires: None,
        },
        EventChoice {
            text: "[+15 fuel, +10 credits] Extract and refine crystal energy",
            chinese_hint: "能 (néng) — energy, ability",
            outcome: EventOutcome::FuelAndCredits(15, 10),
            requires: None,
        },
        EventChoice {
            text: "[Combat level 3] Awaken the ancient guardians for better loot",
            chinese_hint: "醒 (xǐng) — wake",
            outcome: EventOutcome::CombatReward(3, 40),
            requires: None,
        },
        EventChoice {
            text: "[+10 hull] Harvest crystal for hull reinforcement",
            chinese_hint: "强 (qiáng) — strong",
            outcome: EventOutcome::GainHull(10),
            requires: None,
        },
    ],
    category: EventCategory::AncientRuins,
};

pub(super) static EVENT_BOUNTY_BOARD: SpaceEvent = SpaceEvent {
    id: 58,
    title: "Bounty Board",
    chinese_title: "悬赏布告",
    description: "A relay station broadcasts active bounties. Several targets are in \
                  nearby sectors. Taking a bounty means combat, but the rewards \
                  are substantial.",
    choices: &[
        EventChoice {
            text: "[Combat level 2, +25 credits] Hunt the smuggler",
            chinese_hint: "猎 (liè) — hunt",
            outcome: EventOutcome::CombatReward(2, 25),
            requires: None,
        },
        EventChoice {
            text: "[Combat level 4, +45 credits] Track the pirate captain",
            chinese_hint: "海盗 (hǎidào) — pirate",
            outcome: EventOutcome::CombatReward(4, 45),
            requires: None,
        },
        EventChoice {
            text: "[Combat level 3, +35 credits] Capture the rogue AI ship",
            chinese_hint: "捕 (bǔ) — capture",
            outcome: EventOutcome::CombatReward(3, 35),
            requires: None,
        },
        EventChoice {
            text: "[-15 credits] Buy intel on bounty locations",
            chinese_hint: "情报 (qíngbào) — intelligence",
            outcome: EventOutcome::GainItem("Bounty Intel"),
            requires: Some(EventRequirement::HasCredits(15)),
        },
        EventChoice {
            text: "[+10 credits] Sell your own intel to other hunters",
            chinese_hint: "卖 (mài) — sell",
            outcome: EventOutcome::GainCredits(10),
            requires: None,
        },
    ],
    category: EventCategory::Trading,
};

pub(super) static EVENT_NEBULA_SANCTUARY: SpaceEvent = SpaceEvent {
    id: 59,
    title: "Nebula Sanctuary",
    chinese_title: "星云庇护所",
    description: "Hidden within a dense nebula, an alien monastery floats serenely. \
                  Monks of an ancient order offer wisdom, healing, and trade to \
                  peaceful visitors.",
    choices: &[
        EventChoice {
            text: "[Heal 8 HP] Receive their healing blessing",
            chinese_hint: "治 (zhì) — heal, cure",
            outcome: EventOutcome::HealCrew(8),
            requires: None,
        },
        EventChoice {
            text: "[Gain radical 大] Learn their philosophy of expansion",
            chinese_hint: "大 (dà) — big, great",
            outcome: EventOutcome::GainRadical("大"),
            requires: None,
        },
        EventChoice {
            text: "[-20 credits, +25 fuel] Trade for purified nebula fuel",
            chinese_hint: "净 (jìng) — pure",
            outcome: EventOutcome::GainFuel(25),
            requires: Some(EventRequirement::HasCredits(20)),
        },
        EventChoice {
            text: "[Gain crew member] A monk wishes to join your journey",
            chinese_hint: "僧 (sēng) — monk",
            outcome: EventOutcome::GainCrewMember,
            requires: None,
        },
        EventChoice {
            text: "[+15 credits] Trade your stories for their gifts",
            chinese_hint: "故事 (gùshì) — story",
            outcome: EventOutcome::GainCredits(15),
            requires: None,
        },
    ],
    category: EventCategory::AlienContact,
};

pub(super) static EVENT_GRAVITY_SLINGSHOT: SpaceEvent = SpaceEvent {
    id: 60,
    title: "Gravity Slingshot",
    chinese_title: "重力弹弓",
    description: "Twin black holes create a narrow corridor of stable space between \
                  them. Threading the gap could fling you far ahead — or tear your \
                  ship apart.",
    choices: &[
        EventChoice {
            text: "[+25 fuel, -10 hull] Thread the gap at full speed",
            chinese_hint: "快 (kuài) — fast",
            outcome: EventOutcome::HullAndFuel(-10, 25),
            requires: None,
        },
        EventChoice {
            text: "[-5 fuel] Take the long way around",
            chinese_hint: "慢 (màn) — slow",
            outcome: EventOutcome::LoseFuel(5),
            requires: Some(EventRequirement::HasFuel(5)),
        },
        EventChoice {
            text: "[Risk: -20 hull or +20 fuel, +20 credits] Ride the gravity wave",
            chinese_hint: "浪 (làng) — wave",
            outcome: EventOutcome::FuelAndCredits(15, 15),
            requires: None,
        },
        EventChoice {
            text: "[+15 credits] Deploy probes to study the phenomenon",
            chinese_hint: "研究 (yánjiū) — research",
            outcome: EventOutcome::GainCredits(15),
            requires: None,
        },
        EventChoice {
            text: "[Gain radical 力] Meditate on the forces at play",
            chinese_hint: "力 (lì) — power, force",
            outcome: EventOutcome::GainRadical("力"),
            requires: None,
        },
    ],
    category: EventCategory::AnomalyEncounter,
};

pub(super) static EVENT_ABANDONED_SHIPYARD: SpaceEvent = SpaceEvent {
    id: 61,
    title: "Abandoned Shipyard",
    chinese_title: "废弃船坞",
    description: "A massive orbital shipyard drifts silently, its construction bays \
                  still holding half-built vessels. Automated defense turrets flicker \
                  intermittently.",
    choices: &[
        EventChoice {
            text: "[+25 hull] Salvage hull plating from unfinished ships",
            chinese_hint: "修 (xiū) — repair",
            outcome: EventOutcome::GainHull(25),
            requires: None,
        },
        EventChoice {
            text: "[+20 fuel] Drain fuel reserves from docked ships",
            chinese_hint: "油 (yóu) — fuel, oil",
            outcome: EventOutcome::GainFuel(20),
            requires: None,
        },
        EventChoice {
            text: "[-15 credits, Gain item] Buy a blueprint from the data core",
            chinese_hint: "图 (tú) — diagram",
            outcome: EventOutcome::GainItem("Ship Blueprint"),
            requires: Some(EventRequirement::HasCredits(15)),
        },
        EventChoice {
            text: "[Combat level 3, +40 credits] Fight through turrets to the vault",
            chinese_hint: "打 (dǎ) — fight",
            outcome: EventOutcome::CombatReward(3, 40),
            requires: None,
        },
        EventChoice {
            text: "[+15 credits] Scavenge loose parts from the exterior",
            chinese_hint: "捡 (jiǎn) — pick up",
            outcome: EventOutcome::GainCredits(15),
            requires: None,
        },
    ],
    category: EventCategory::Discovery,
};

pub(super) static EVENT_SPORE_CLOUD: SpaceEvent = SpaceEvent {
    id: 62,
    title: "Spore Cloud",
    chinese_title: "孢子云",
    description: "A massive cloud of alien spores engulfs your ship. They begin eating \
                  through the hull but seem to have medicinal properties if properly \
                  processed.",
    choices: &[
        EventChoice {
            text: "[-10 hull, Heal 5 HP] Process spores into medicine",
            chinese_hint: "药 (yào) — medicine",
            outcome: EventOutcome::HealCrew(5),
            requires: None,
        },
        EventChoice {
            text: "[-5 hull] Activate hull scrubbers to purge them",
            chinese_hint: "清 (qīng) — clean",
            outcome: EventOutcome::LoseHull(5),
            requires: None,
        },
        EventChoice {
            text: "[+20 credits, -15 hull] Collect spores for sale to researchers",
            chinese_hint: "卖 (mài) — sell",
            outcome: EventOutcome::GainCredits(20),
            requires: None,
        },
        EventChoice {
            text: "[-15 fuel] Full burn to escape the cloud",
            chinese_hint: "逃 (táo) — escape",
            outcome: EventOutcome::LoseFuel(15),
            requires: Some(EventRequirement::HasFuel(15)),
        },
        EventChoice {
            text: "[Gain radical 虫] Study the spore lifecycle",
            chinese_hint: "虫 (chóng) — insect, bug",
            outcome: EventOutcome::GainRadical("虫"),
            requires: None,
        },
    ],
    category: EventCategory::HazardEvent,
};

pub(super) static EVENT_MERCENARY_OUTPOST: SpaceEvent = SpaceEvent {
    id: 63,
    title: "Mercenary Outpost",
    chinese_title: "雇佣兵前哨",
    description: "A well-armed mercenary company operates from this station. They offer \
                  their services — or might just take what they want if you look weak \
                  enough.",
    choices: &[
        EventChoice {
            text: "[-25 credits, Gain crew member] Hire a veteran fighter",
            chinese_hint: "兵 (bīng) — soldier",
            outcome: EventOutcome::GainCrewMember,
            requires: Some(EventRequirement::HasCredits(25)),
        },
        EventChoice {
            text: "[Combat level 3, +35 credits] Accept a contract job",
            chinese_hint: "合同 (hétóng) — contract",
            outcome: EventOutcome::CombatReward(3, 35),
            requires: None,
        },
        EventChoice {
            text: "[-15 credits, Gain item] Buy weapon upgrades",
            chinese_hint: "武器 (wǔqì) — weapon",
            outcome: EventOutcome::GainItem("Weapon Mod"),
            requires: Some(EventRequirement::HasCredits(15)),
        },
        EventChoice {
            text: "[+20 credits] Sell surplus equipment",
            chinese_hint: "卖 (mài) — sell",
            outcome: EventOutcome::GainCredits(20),
            requires: None,
        },
        EventChoice {
            text: "[Nothing] Leave before they get ideas",
            chinese_hint: "离开 (líkāi) — leave",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::PirateEncounter,
};

pub(super) static EVENT_SINGING_COMET: SpaceEvent = SpaceEvent {
    id: 64,
    title: "Singing Comet",
    chinese_title: "歌唱彗星",
    description: "A comet emits harmonic frequencies as it passes through a magnetic \
                  field. The vibrations resonate with your ship's hull, creating an \
                  eerie melody that seems to carry meaning.",
    choices: &[
        EventChoice {
            text: "[Gain radical 耳] Listen and decode the harmonic patterns",
            chinese_hint: "耳 (ěr) — ear",
            outcome: EventOutcome::GainRadical("耳"),
            requires: None,
        },
        EventChoice {
            text: "[+15 fuel] Match frequency to refine comet ice into fuel",
            chinese_hint: "冰 (bīng) — ice",
            outcome: EventOutcome::GainFuel(15),
            requires: None,
        },
        EventChoice {
            text: "[+20 credits] Record and sell the frequencies as data",
            chinese_hint: "录 (lù) — record",
            outcome: EventOutcome::GainCredits(20),
            requires: None,
        },
        EventChoice {
            text: "[-5 hull, +30 credits] Mine the comet's core",
            chinese_hint: "核 (hé) — core",
            outcome: EventOutcome::GainCredits(30),
            requires: None,
        },
        EventChoice {
            text: "[Heal 3 HP] Let the harmonics calm your crew",
            chinese_hint: "音乐 (yīnyuè) — music",
            outcome: EventOutcome::HealCrew(3),
            requires: None,
        },
    ],
    category: EventCategory::AnomalyEncounter,
};

pub(super) static EVENT_CLONE_LAB: SpaceEvent = SpaceEvent {
    id: 65,
    title: "Clone Lab",
    chinese_title: "克隆实验室",
    description: "A derelict station contains functional cloning vats. Your chief \
                  medical officer suggests using them, but the technology raises \
                  ethical concerns among the crew.",
    choices: &[
        EventChoice {
            text: "[Gain crew member, -10 hull] Activate the cloning sequence",
            chinese_hint: "复制 (fùzhì) — copy",
            outcome: EventOutcome::GainCrewMember,
            requires: None,
        },
        EventChoice {
            text: "[+20 credits] Sell the cloning data",
            chinese_hint: "数据 (shùjù) — data",
            outcome: EventOutcome::GainCredits(20),
            requires: None,
        },
        EventChoice {
            text: "[Heal 5 HP] Use the medical equipment for healing",
            chinese_hint: "医 (yī) — medical",
            outcome: EventOutcome::HealCrew(5),
            requires: None,
        },
        EventChoice {
            text: "[Gain radical 身] Study the bio-patterns",
            chinese_hint: "身 (shēn) — body",
            outcome: EventOutcome::GainRadical("身"),
            requires: None,
        },
        EventChoice {
            text: "[-5 hull] Destroy the lab to prevent misuse",
            chinese_hint: "毁 (huǐ) — destroy",
            outcome: EventOutcome::LoseHull(5),
            requires: None,
        },
    ],
    category: EventCategory::CrewEvent,
};

pub(super) static EVENT_SPACE_WHALE_MIGRATION: SpaceEvent = SpaceEvent {
    id: 66,
    title: "Space Whale Migration",
    chinese_title: "太空鲸鱼迁徙",
    description: "Enormous bio-luminescent creatures drift through space in a great \
                  migration. Their gentle calls echo through your hull. They seem to \
                  be following ancient stellar currents.",
    choices: &[
        EventChoice {
            text: "[+20 fuel] Follow their path to a fuel-rich region",
            chinese_hint: "跟 (gēn) — follow",
            outcome: EventOutcome::GainFuel(20),
            requires: None,
        },
        EventChoice {
            text: "[Gain radical 鱼] Study their navigation instincts",
            chinese_hint: "鱼 (yú) — fish",
            outcome: EventOutcome::GainRadical("鱼"),
            requires: None,
        },
        EventChoice {
            text: "[+25 credits] Carefully harvest shed bio-crystals",
            chinese_hint: "采 (cǎi) — harvest",
            outcome: EventOutcome::GainCredits(25),
            requires: None,
        },
        EventChoice {
            text: "[Heal 5 HP] Let their calming songs soothe the crew",
            chinese_hint: "歌 (gē) — song",
            outcome: EventOutcome::HealCrew(5),
            requires: None,
        },
        EventChoice {
            text: "[+10 hull] Use their magnetic wake to recalibrate systems",
            chinese_hint: "磁 (cí) — magnetic",
            outcome: EventOutcome::GainHull(10),
            requires: None,
        },
    ],
    category: EventCategory::AlienContact,
};

pub(super) static EVENT_SALVAGE_COMPETITION: SpaceEvent = SpaceEvent {
    id: 67,
    title: "Salvage Competition",
    chinese_title: "打捞竞赛",
    description: "Multiple salvage teams converge on a massive derelict capital ship. \
                  The station master declares a salvage competition — first come, \
                  first served, with prizes for the most valuable haul.",
    choices: &[
        EventChoice {
            text: "[Risk: -10 hull or +40 credits] Race to the engineering section",
            chinese_hint: "快 (kuài) — fast",
            outcome: EventOutcome::GainCredits(35),
            requires: None,
        },
        EventChoice {
            text: "[-10 fuel, +30 credits] Use your engines to reach cargo first",
            chinese_hint: "先 (xiān) — first",
            outcome: EventOutcome::FuelAndCredits(-10, 30),
            requires: Some(EventRequirement::HasFuel(10)),
        },
        EventChoice {
            text: "[Combat level 2, +35 credits] Fight other teams for the best loot",
            chinese_hint: "争 (zhēng) — compete",
            outcome: EventOutcome::CombatReward(2, 35),
            requires: None,
        },
        EventChoice {
            text: "[+15 credits] Play it safe, scavenge the outer hull",
            chinese_hint: "安全 (ānquán) — safe",
            outcome: EventOutcome::GainCredits(15),
            requires: None,
        },
        EventChoice {
            text: "[-20 credits, +25 hull] Buy the rights to the bridge section",
            chinese_hint: "桥 (qiáo) — bridge",
            outcome: EventOutcome::GainHull(25),
            requires: Some(EventRequirement::HasCredits(20)),
        },
    ],
    category: EventCategory::Trading,
};

pub(super) static EVENT_DIMENSIONAL_RIFT: SpaceEvent = SpaceEvent {
    id: 68,
    title: "Dimensional Rift",
    chinese_title: "维度裂缝",
    description: "A shimmering tear in space reveals glimpses of another reality. \
                  Strange objects drift through — some recognizable, others defying \
                  physics. The rift pulses with unstable energy.",
    choices: &[
        EventChoice {
            text: "[Risk: -15 hull or Gain radical 又] Reach through the rift",
            chinese_hint: "又 (yòu) — again, also",
            outcome: EventOutcome::GainRadical("又"),
            requires: None,
        },
        EventChoice {
            text: "[+25 credits] Collect drifting objects from the edge",
            chinese_hint: "收 (shōu) — collect",
            outcome: EventOutcome::GainCredits(25),
            requires: None,
        },
        EventChoice {
            text: "[-10 fuel, +20 hull] Use the energy to recharge shields",
            chinese_hint: "能 (néng) — energy",
            outcome: EventOutcome::GainHull(20),
            requires: Some(EventRequirement::HasFuel(10)),
        },
        EventChoice {
            text: "[+15 fuel] Siphon dimensional energy carefully",
            chinese_hint: "吸 (xī) — absorb",
            outcome: EventOutcome::GainFuel(15),
            requires: None,
        },
        EventChoice {
            text: "[Nothing] Document the phenomenon and leave",
            chinese_hint: "观察 (guānchá) — observe",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::AnomalyEncounter,
};

pub(super) static EVENT_ALIEN_ARENA: SpaceEvent = SpaceEvent {
    id: 69,
    title: "Alien Arena",
    chinese_title: "外星竞技场",
    description: "An alien species runs a gladiatorial arena aboard their station. They \
                  challenge all visitors to prove their worth in combat. Victory brings \
                  glory and reward.",
    choices: &[
        EventChoice {
            text: "[Combat level 3, +35 credits] Enter the arena!",
            chinese_hint: "斗 (dòu) — fight",
            outcome: EventOutcome::CombatReward(3, 35),
            requires: None,
        },
        EventChoice {
            text: "[Combat level 5, +60 credits] Challenge the champion",
            chinese_hint: "冠军 (guànjūn) — champion",
            outcome: EventOutcome::CombatReward(5, 60),
            requires: None,
        },
        EventChoice {
            text: "[-20 credits] Bet on another fighter",
            chinese_hint: "赌 (dǔ) — gamble",
            outcome: EventOutcome::GainCredits(30),
            requires: Some(EventRequirement::HasCredits(20)),
        },
        EventChoice {
            text: "[+15 credits] Sell refreshments to the crowd",
            chinese_hint: "卖 (mài) — sell",
            outcome: EventOutcome::GainCredits(15),
            requires: None,
        },
        EventChoice {
            text: "[Gain radical 角] Study the alien fighting styles",
            chinese_hint: "角 (jiǎo) — horn, angle",
            outcome: EventOutcome::GainRadical("角"),
            requires: None,
        },
    ],
    category: EventCategory::AlienContact,
};

pub(super) static EVENT_SOLAR_FORGE: SpaceEvent = SpaceEvent {
    id: 70,
    title: "Solar Forge",
    chinese_title: "太阳熔炉",
    description: "An ancient structure orbiting close to a star harnesses its energy to \
                  forge exotic metals. The heat is extreme, but the materials inside \
                  are priceless.",
    choices: &[
        EventChoice {
            text: "[-15 hull, +45 credits] Mine the forged metals",
            chinese_hint: "铁 (tiě) — iron",
            outcome: EventOutcome::GainCredits(45),
            requires: None,
        },
        EventChoice {
            text: "[+25 hull] Use the forge to reinforce your hull",
            chinese_hint: "强 (qiáng) — strengthen",
            outcome: EventOutcome::GainHull(25),
            requires: None,
        },
        EventChoice {
            text: "[Gain radical 金] Study the metallurgical inscriptions",
            chinese_hint: "金 (jīn) — gold, metal",
            outcome: EventOutcome::GainRadical("金"),
            requires: None,
        },
        EventChoice {
            text: "[+20 fuel] Convert stellar energy to fuel",
            chinese_hint: "太阳 (tàiyáng) — sun",
            outcome: EventOutcome::GainFuel(20),
            requires: None,
        },
        EventChoice {
            text: "[-10 fuel] Retreat to safe distance and scan remotely",
            chinese_hint: "安全 (ānquán) — safe",
            outcome: EventOutcome::LoseFuel(10),
            requires: Some(EventRequirement::HasFuel(10)),
        },
    ],
    category: EventCategory::AncientRuins,
};

pub(super) static EVENT_GHOST_FLEET: SpaceEvent = SpaceEvent {
    id: 71,
    title: "Ghost Fleet",
    chinese_title: "幽灵舰队",
    description: "Dozens of derelict warships drift in formation, their weapons cold \
                  but their reactors still humming. Something wiped them out \
                  simultaneously. Warning beacons have long since failed.",
    choices: &[
        EventChoice {
            text: "[+20 fuel, -5 hull] Siphon reactor fuel from the nearest ship",
            chinese_hint: "吸 (xī) — absorb",
            outcome: EventOutcome::HullAndFuel(-5, 20),
            requires: None,
        },
        EventChoice {
            text: "[+30 credits, -10 hull] Loot the flagship's vault",
            chinese_hint: "宝 (bǎo) — treasure",
            outcome: EventOutcome::GainCredits(30),
            requires: None,
        },
        EventChoice {
            text: "[Gain radical 目] Investigate what killed them",
            chinese_hint: "目 (mù) — eye",
            outcome: EventOutcome::GainRadical("目"),
            requires: None,
        },
        EventChoice {
            text: "[+15 hull] Salvage intact hull sections",
            chinese_hint: "修 (xiū) — repair",
            outcome: EventOutcome::GainHull(15),
            requires: None,
        },
        EventChoice {
            text: "[Nothing] Too dangerous — leave immediately",
            chinese_hint: "危 (wēi) — danger",
            outcome: EventOutcome::Nothing,
            requires: None,
        },
    ],
    category: EventCategory::HazardEvent,
};

pub(super) static EVENT_MEDITATION_NEBULA: SpaceEvent = SpaceEvent {
    id: 72,
    title: "Meditation Nebula",
    chinese_title: "冥想星云",
    description: "A serene nebula emits frequencies that enhance mental clarity. Ancient \
                  travelers left language puzzles carved into floating monoliths as \
                  tests of wisdom.",
    choices: &[
        EventChoice {
            text: "[Gain radical 言] Solve the word puzzle on the first monolith",
            chinese_hint: "言 (yán) — speech, word",
            outcome: EventOutcome::GainRadical("言"),
            requires: None,
        },
        EventChoice {
            text: "[Gain radical 竹] Decode the nature cipher on the second",
            chinese_hint: "竹 (zhú) — bamboo",
            outcome: EventOutcome::GainRadical("竹"),
            requires: None,
        },
        EventChoice {
            text: "[Heal 5 HP] Meditate in the calming frequencies",
            chinese_hint: "静 (jìng) — calm",
            outcome: EventOutcome::HealCrew(5),
            requires: None,
        },
        EventChoice {
            text: "[+20 credits] Photograph and sell the inscriptions",
            chinese_hint: "照 (zhào) — photograph",
            outcome: EventOutcome::GainCredits(20),
            requires: None,
        },
        EventChoice {
            text: "[-5 fuel, Gain radical 页] Follow the monolith trail deeper",
            chinese_hint: "页 (yè) — page",
            outcome: EventOutcome::GainRadical("页"),
            requires: Some(EventRequirement::HasFuel(5)),
        },
    ],
    category: EventCategory::LanguageChallenge,
};

pub(super) static EVENT_EMERGENCY_BEACON: SpaceEvent = SpaceEvent {
    id: 73,
    title: "Emergency Beacon",
    chinese_title: "紧急信标",
    description: "An automated distress beacon leads you to a damaged military corvette. \
                  The surviving officer offers classified intel and supplies in exchange \
                  for escort to the nearest station.",
    choices: &[
        EventChoice {
            text: "[-10 fuel, +40 credits] Escort them for full payment",
            chinese_hint: "护送 (hùsòng) — escort",
            outcome: EventOutcome::FuelAndCredits(-10, 40),
            requires: Some(EventRequirement::HasFuel(10)),
        },
        EventChoice {
            text: "[+20 hull] Accept hull repair kits as partial payment",
            chinese_hint: "修理 (xiūlǐ) — repair",
            outcome: EventOutcome::GainHull(20),
            requires: None,
        },
        EventChoice {
            text: "[Gain crew member] The officer joins your crew",
            chinese_hint: "军人 (jūnrén) — soldier",
            outcome: EventOutcome::GainCrewMember,
            requires: None,
        },
        EventChoice {
            text: "[+25 credits, -5 hull] Salvage their damaged ship for parts",
            chinese_hint: "拆 (chāi) — dismantle",
            outcome: EventOutcome::GainCredits(25),
            requires: None,
        },
        EventChoice {
            text: "[Combat level 2, +30 credits] Betray them and take everything",
            chinese_hint: "背叛 (bèipàn) — betray",
            outcome: EventOutcome::CombatReward(2, 30),
            requires: None,
        },
    ],
    category: EventCategory::DistressSignal,
};

