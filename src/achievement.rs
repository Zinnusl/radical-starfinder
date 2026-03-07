//! Achievement system — persistent milestones tracked via localStorage.

/// An achievement definition.
pub struct AchievementDef {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    #[allow(dead_code)]
    pub icon: &'static str,
}

pub const ACHIEVEMENTS: &[AchievementDef] = &[
    AchievementDef { id: "first_kill", name: "First Blood", desc: "Defeat your first enemy", icon: "⚔" },
    AchievementDef { id: "kill_10", name: "Warrior", desc: "Defeat 10 enemies total", icon: "⚔" },
    AchievementDef { id: "kill_50", name: "Slayer", desc: "Defeat 50 enemies total", icon: "⚔" },
    AchievementDef { id: "kill_100", name: "Legend", desc: "Defeat 100 enemies total", icon: "⚔" },
    AchievementDef { id: "floor_3", name: "Delver", desc: "Reach floor 3", icon: "🏔" },
    AchievementDef { id: "floor_5", name: "Explorer", desc: "Reach floor 5", icon: "🏔" },
    AchievementDef { id: "floor_10", name: "Deep Diver", desc: "Reach floor 10", icon: "🏔" },
    AchievementDef { id: "first_forge", name: "Apprentice", desc: "Forge your first spell", icon: "🔨" },
    AchievementDef { id: "forge_5", name: "Smith", desc: "Discover 5 recipes", icon: "🔨" },
    AchievementDef { id: "forge_10", name: "Master Smith", desc: "Discover 10 recipes", icon: "🔨" },
    AchievementDef { id: "forge_20", name: "Grand Forgemaster", desc: "Discover 20 recipes", icon: "🔨" },
    AchievementDef { id: "gold_100", name: "Prospector", desc: "Hold 100 gold at once", icon: "💰" },
    AchievementDef { id: "gold_500", name: "Wealthy", desc: "Hold 500 gold at once", icon: "💰" },
    AchievementDef { id: "first_elite", name: "Elite Slayer", desc: "Defeat an elite enemy", icon: "★" },
    AchievementDef { id: "first_boss", name: "Boss Killer", desc: "Defeat a boss", icon: "👑" },
    AchievementDef { id: "first_chest", name: "Treasure Hunter", desc: "Open your first chest", icon: "◆" },
    AchievementDef { id: "full_inv", name: "Pack Rat", desc: "Fill your item inventory", icon: "🎒" },
    AchievementDef { id: "perfect_5", name: "Scholar", desc: "5 correct pinyin in a row", icon: "📖" },
    AchievementDef { id: "radicals_10", name: "Collector", desc: "Hold 10 radicals at once", icon: "字" },
    AchievementDef { id: "spells_5", name: "Spellbook", desc: "Have 5 spells at once", icon: "✨" },
];

/// Tracker for unlocked achievements.
pub struct AchievementTracker {
    pub unlocked: Vec<&'static str>,
    /// Queue of newly unlocked achievement names for popup display.
    pub popup_queue: Vec<&'static str>,
    /// Streak counter for consecutive correct answers.
    pub correct_streak: u32,
}

impl AchievementTracker {
    pub fn new() -> Self {
        Self {
            unlocked: Vec::new(),
            popup_queue: Vec::new(),
            correct_streak: 0,
        }
    }

    pub fn load() -> Self {
        let mut tracker = Self::new();
        let storage = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten());
        if let Some(storage) = storage {
            if let Ok(Some(data)) = storage.get_item("radical_roguelike_achievements") {
                tracker.unlocked = data.split(',').filter(|s| !s.is_empty()).map(|s| {
                    // Find the matching static str
                    ACHIEVEMENTS.iter().find(|a| a.id == s).map(|a| a.id).unwrap_or("")
                }).filter(|s| !s.is_empty()).collect();
            }
        }
        tracker
    }

    pub fn save(&self) {
        let storage = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten());
        if let Some(storage) = storage {
            let data: String = self.unlocked.join(",");
            let _ = storage.set_item("radical_roguelike_achievements", &data);
        }
    }

    /// Try to unlock an achievement. Returns true if newly unlocked.
    pub fn unlock(&mut self, id: &'static str) -> bool {
        if self.unlocked.contains(&id) {
            return false;
        }
        // Verify it's a valid achievement
        if ACHIEVEMENTS.iter().any(|a| a.id == id) {
            self.unlocked.push(id);
            self.popup_queue.push(id);
            self.save();
            true
        } else {
            false
        }
    }

    /// Check and unlock achievements based on current game stats.
    pub fn check_kills(&mut self, total_kills: u32) {
        if total_kills >= 1 { self.unlock("first_kill"); }
        if total_kills >= 10 { self.unlock("kill_10"); }
        if total_kills >= 50 { self.unlock("kill_50"); }
        if total_kills >= 100 { self.unlock("kill_100"); }
    }

    pub fn check_floor(&mut self, floor: i32) {
        if floor >= 3 { self.unlock("floor_3"); }
        if floor >= 5 { self.unlock("floor_5"); }
        if floor >= 10 { self.unlock("floor_10"); }
    }

    pub fn check_recipes(&mut self, count: usize) {
        if count >= 1 { self.unlock("first_forge"); }
        if count >= 5 { self.unlock("forge_5"); }
        if count >= 10 { self.unlock("forge_10"); }
        if count >= 20 { self.unlock("forge_20"); }
    }

    pub fn check_gold(&mut self, gold: i32) {
        if gold >= 100 { self.unlock("gold_100"); }
        if gold >= 500 { self.unlock("gold_500"); }
    }

    pub fn check_radicals(&mut self, count: usize) {
        if count >= 10 { self.unlock("radicals_10"); }
    }

    pub fn check_spells(&mut self, count: usize) {
        if count >= 5 { self.unlock("spells_5"); }
    }

    pub fn check_items(&mut self, count: usize) {
        if count >= 5 { self.unlock("full_inv"); }
    }

    pub fn record_correct(&mut self) {
        self.correct_streak += 1;
        if self.correct_streak >= 5 { self.unlock("perfect_5"); }
    }

    pub fn record_miss(&mut self) {
        self.correct_streak = 0;
    }

    /// Pop the next achievement popup, if any.
    pub fn pop_popup(&mut self) -> Option<&'static str> {
        if self.popup_queue.is_empty() {
            None
        } else {
            Some(self.popup_queue.remove(0))
        }
    }

    /// Get achievement def by id.
    pub fn get_def(id: &str) -> Option<&'static AchievementDef> {
        ACHIEVEMENTS.iter().find(|a| a.id == id)
    }
}
