//! Dungeon dialogue definitions — encounter dialogues during dungeon exploration.

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum DungeonCategory {
    Discovery,
    Trader,
    Alien,
    Hazard,
    Crew,
    Puzzle,
    Lore,
    Shrine,
    Wreckage,
    Terminal,
    Creature,
    Anomaly,
}

#[derive(Clone, Debug)]
pub enum DungeonOutcome {
    Heal(i32),
    Damage(i32),
    GainGold(i32),
    LoseGold(i32),
    GainXp(i32),
    GainRadical(&'static str),
    GainItem(&'static str),
    GainEquipment,
    StartFight,
    Nothing,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum DungeonRequirement {
    HasGold(i32),
    HasHp(i32),
    HasRadical(&'static str),
    HasClass(u8),
    None,
}

#[derive(Clone, Debug)]
pub struct DungeonChoice {
    pub text: &'static str,
    pub chinese_hint: &'static str,
    pub outcome: DungeonOutcome,
    pub requires: Option<DungeonRequirement>,
}

#[derive(Clone, Debug)]
pub struct DungeonDialogue {
    pub id: usize,
    pub title: &'static str,
    pub chinese_title: &'static str,
    pub description: &'static str,
    pub choices: &'static [DungeonChoice],
    pub category: DungeonCategory,
}

// This will eventually be replaced by: include!(concat!(env!("OUT_DIR"), "/dialogue_data.rs"));
// For now, use hardcoded test data:

pub static ALL_DUNGEON_DIALOGUES: &[&DungeonDialogue] = &[
    &DUNGEON_TEST_1,
    &DUNGEON_TEST_2,
    &DUNGEON_TEST_3,
];

static DUNGEON_TEST_1: DungeonDialogue = DungeonDialogue {
    id: 0,
    title: "Abandoned Terminal",
    chinese_title: "废弃终端",
    description: "A flickering terminal hums in the darkness. Strange symbols scroll across its cracked screen.",
    choices: &[
        DungeonChoice {
            text: "Access the terminal",
            chinese_hint: "终端 (zhōngduān) — terminal",
            outcome: DungeonOutcome::GainXp(15),
            requires: None,
        },
        DungeonChoice {
            text: "Smash it for parts",
            chinese_hint: "打破 (dǎpò) — smash",
            outcome: DungeonOutcome::GainGold(10),
            requires: None,
        },
        DungeonChoice {
            text: "Leave it alone",
            chinese_hint: "离开 (líkāi) — leave",
            outcome: DungeonOutcome::Nothing,
            requires: None,
        },
    ],
    category: DungeonCategory::Terminal,
};

static DUNGEON_TEST_2: DungeonDialogue = DungeonDialogue {
    id: 1,
    title: "Wounded Creature",
    chinese_title: "受伤的生物",
    description: "A small alien creature whimpers behind a collapsed beam. Its luminescent eyes watch you warily.",
    choices: &[
        DungeonChoice {
            text: "Help the creature",
            chinese_hint: "帮助 (bāngzhù) — help",
            outcome: DungeonOutcome::Heal(10),
            requires: None,
        },
        DungeonChoice {
            text: "Ignore it",
            chinese_hint: "忽略 (hūlüè) — ignore",
            outcome: DungeonOutcome::Nothing,
            requires: None,
        },
        DungeonChoice {
            text: "Scavenge nearby parts",
            chinese_hint: "搜索 (sōusuǒ) — search",
            outcome: DungeonOutcome::GainGold(5),
            requires: None,
        },
    ],
    category: DungeonCategory::Creature,
};

static DUNGEON_TEST_3: DungeonDialogue = DungeonDialogue {
    id: 2,
    title: "Unstable Reactor",
    chinese_title: "不稳定的反应堆",
    description: "Warning klaxons blare as coolant drips from a cracked reactor casing. The air crackles with energy.",
    choices: &[
        DungeonChoice {
            text: "Attempt emergency shutdown",
            chinese_hint: "关闭 (guānbì) — shut down",
            outcome: DungeonOutcome::GainXp(25),
            requires: Some(DungeonRequirement::HasHp(20)),
        },
        DungeonChoice {
            text: "Salvage exposed power cells",
            chinese_hint: "危险 (wēixiǎn) — danger",
            outcome: DungeonOutcome::Damage(8),
            requires: None,
        },
        DungeonChoice {
            text: "Back away carefully",
            chinese_hint: "安全 (ānquán) — safe",
            outcome: DungeonOutcome::Nothing,
            requires: None,
        },
    ],
    category: DungeonCategory::Hazard,
};
