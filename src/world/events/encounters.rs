//! Combat and danger encounters: distress signals, pirates, aliens, hazards.

use super::types::*;

// ── Distress Signals (7) ────────────────────────────────────────────────────

pub(super) static EVENT_DISTRESS_BEACON: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_ESCAPE_POD: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_DAMAGED_FREIGHTER: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_COLONY_SOS: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_STRANDED_MINERS: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_GHOST_SHIP: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_MEDICAL_FRIGATE: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_PIRATE_AMBUSH: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_PIRATE_BASE: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_PIRATE_BOARDING: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_PIRATE_DEFECTOR: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_PIRATE_CONVOY: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_RANSOM_DEMAND: SpaceEvent = SpaceEvent {
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


// ── Alien Contact (4) ───────────────────────────────────────────────────────

pub(super) static EVENT_FIRST_CONTACT: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_ALIEN_TRADERS: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_ALIEN_DISTRESS: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_ALIEN_MONUMENT: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_DEBRIS_FIELD: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_SOLAR_FLARE: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_MINEFIELD: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_RADIATION_BELT: SpaceEvent = SpaceEvent {
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

