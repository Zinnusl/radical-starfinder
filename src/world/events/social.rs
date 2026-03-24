//! Social events: crew interactions.

use super::types::*;

// ── Crew Events (4) ─────────────────────────────────────────────────────────

pub(super) static EVENT_CREW_CONFLICT: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_TRAINING_EXERCISE: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_STOWAWAY: SpaceEvent = SpaceEvent {
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

pub(super) static EVENT_CREW_CELEBRATION: SpaceEvent = SpaceEvent {
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

