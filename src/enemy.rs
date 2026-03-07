//! Enemy entities that live on the dungeon floor.

use crate::status::StatusInstance;
use crate::vocab::VocabEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossKind {
    Gatekeeper,
    Scholar,
    Elementalist,
}

impl BossKind {
    pub fn for_floor(floor: i32) -> Option<Self> {
        match floor {
            5 => Some(Self::Gatekeeper),
            10 => Some(Self::Scholar),
            15 => Some(Self::Elementalist),
            _ => None,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Gatekeeper => "Gatekeeper",
            Self::Scholar => "Scholar",
            Self::Elementalist => "Elementalist",
        }
    }
}

fn get_components(hanzi: &str) -> Vec<&'static str> {
    match hanzi {
        "明" => vec!["日", "月"],
        "好" => vec!["女", "子"],
        "林" => vec!["木", "木"],
        "休" => vec!["人", "木"],
        "看" => vec!["手", "目"],
        "体" => vec!["人", "本"],
        "男" => vec!["田", "力"],
        "思" => vec!["田", "心"],
        "李" => vec!["木", "子"],
        "早" => vec!["日", "十"],
        "香" => vec!["禾", "日"],
        "杏" => vec!["木", "口"],
        "呆" => vec!["口", "木"],
        "森" => vec!["木", "林"],
        "晶" => vec!["日", "日", "日"],
        "众" => vec!["人", "人", "人"],
        "品" => vec!["口", "口", "口"],
        _ => vec![],
    }
}

#[derive(Clone)]
pub struct Enemy {
    pub x: i32,
    pub y: i32,
    pub hanzi: &'static str,
    pub pinyin: &'static str,
    pub meaning: &'static str,
    pub hp: i32,
    pub max_hp: i32,
    pub damage: i32,
    /// Set when the enemy is alerted (player in same room / nearby)
    pub alert: bool,
    /// Boss enemies are tougher and give better rewards
    pub is_boss: bool,
    /// Elite multi-character enemies
    pub is_elite: bool,
    /// Gold dropped on defeat
    pub gold_value: i32,
    /// Stunned: skip next turn
    pub stunned: bool,
    /// Active status effects
    pub statuses: Vec<StatusInstance>,
    /// Floor-specific boss mechanics
    pub boss_kind: Option<BossKind>,
    /// Tracks one-time boss phase mechanics
    pub phase_triggered: bool,
    /// Gatekeeper summon cadence
    pub summon_cooldown: u8,
    /// Elementalist resistance remembers the last spell school used
    pub resisted_spell: Option<&'static str>,
    /// Elite compounds are dismantled syllable by syllable
    pub elite_chain: usize,
    /// Defensive components (shields) that must be broken first
    pub components: Vec<&'static str>,
}

impl Enemy {
    pub fn from_vocab(entry: &'static VocabEntry, x: i32, y: i32, floor: i32) -> Self {
        let is_elite = crate::vocab::is_elite(entry);
        let hp = if is_elite { 4 + floor } else { 2 + floor / 2 };
        let damage = if is_elite { 2 + floor / 2 } else { 1 + floor / 3 };
        let gold = if is_elite { 15 + floor * 3 } else { 5 + floor * 2 };
        
        let components = get_components(entry.hanzi);
        
        Self {
            x,
            y,
            hanzi: entry.hanzi,
            pinyin: entry.pinyin,
            meaning: entry.meaning,
            hp,
            max_hp: hp,
            damage,
            alert: false,
            is_boss: false,
            is_elite,
            gold_value: gold,
            stunned: false,
            statuses: Vec::new(),
            boss_kind: None,
            phase_triggered: false,
            summon_cooldown: 0,
            resisted_spell: None,
            elite_chain: 0,
            components,
        }
    }

    pub fn boss_from_vocab(entry: &'static VocabEntry, x: i32, y: i32, floor: i32) -> Self {
        let boss_kind = BossKind::for_floor(floor);
        let (hp, damage, gold, cooldown) = match boss_kind {
            Some(BossKind::Gatekeeper) => (16 + floor, 3 + floor / 3, 60 + floor * 5, 1),
            Some(BossKind::Scholar) => (14 + floor, 3 + floor / 3, 70 + floor * 5, 0),
            Some(BossKind::Elementalist) => (18 + floor, 4 + floor / 3, 80 + floor * 5, 0),
            None => (8 + floor, 2 + floor / 2, 30 + floor * 5, 0),
        };
        Self {
            x,
            y,
            hanzi: entry.hanzi,
            pinyin: entry.pinyin,
            meaning: entry.meaning,
            hp,
            max_hp: hp,
            damage,
            alert: true, // bosses are always alert
            is_boss: true,
            is_elite: false,
            gold_value: gold,
            stunned: false,
            statuses: Vec::new(),
            boss_kind,
            phase_triggered: false,
            summon_cooldown: cooldown,
            resisted_spell: None,
            elite_chain: 0,
            components: Vec::new(), // Bosses don't use standard components yet
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// Simple chase AI: move one step toward (tx, ty) if possible.
    /// Returns desired (nx, ny). Caller checks walkability & occupancy.
    pub fn step_toward(&self, tx: i32, ty: i32) -> (i32, i32) {
        let dx = (tx - self.x).signum();
        let dy = (ty - self.y).signum();
        // Prefer axis with larger distance
        if (tx - self.x).abs() >= (ty - self.y).abs() {
            if dx != 0 {
                return (self.x + dx, self.y);
            }
            (self.x, self.y + dy)
        } else {
            if dy != 0 {
                return (self.x, self.y + dy);
            }
            (self.x + dx, self.y)
        }
    }

    pub fn boss_trait_text(&self) -> Option<String> {
        match self.boss_kind {
            Some(BossKind::Gatekeeper) => Some("Summons 门 wards when cornered".to_string()),
            Some(BossKind::Scholar) => Some(if self.phase_triggered {
                "Sentence duel spent".to_string()
            } else {
                "Triggers a sentence duel at half HP".to_string()
            }),
            Some(BossKind::Elementalist) => Some(match self.resisted_spell {
                Some(school) => format!("Resists last spell: {}", school),
                None => "Adapts to the last spell you cast".to_string(),
            }),
            None => None,
        }
    }

    pub fn elite_phase_count(&self) -> usize {
        crate::vocab::pinyin_syllables(self.pinyin).len().max(1)
    }

    pub fn elite_expected_syllable(&self) -> Option<&str> {
        if !self.is_elite {
            return None;
        }
        let syllables = crate::vocab::pinyin_syllables(self.pinyin);
        let idx = self.elite_chain.min(syllables.len().saturating_sub(1));
        syllables.get(idx).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::{BossKind, Enemy};
    use crate::vocab::VOCAB;

    fn friend_entry() -> &'static crate::vocab::VocabEntry {
        VOCAB.iter().find(|entry| entry.hanzi == "朋友").unwrap()
    }

    #[test]
    fn boss_kind_matches_key_floors() {
        assert_eq!(BossKind::for_floor(5), Some(BossKind::Gatekeeper));
        assert_eq!(BossKind::for_floor(10), Some(BossKind::Scholar));
        assert_eq!(BossKind::for_floor(15), Some(BossKind::Elementalist));
        assert_eq!(BossKind::for_floor(20), None);
    }

    #[test]
    fn elite_expected_syllable_tracks_chain_progress() {
        let mut enemy = Enemy::from_vocab(friend_entry(), 0, 0, 6);
        enemy.elite_chain = 1;

        assert_eq!(enemy.elite_expected_syllable(), Some("you3"));
    }
}
