//! Status effects that can be applied to players and enemies.

/// A timed status effect.
#[derive(Clone, Debug)]
pub enum StatusKind {
    /// Damage per turn for N turns
    Poison { damage: i32 },
    /// Heal per turn for N turns
    #[allow(dead_code)]
    Regen { heal: i32 },
    /// Player gets extra actions (enemy_turn skipped on even ticks)
    Haste,
    /// Player movement is randomized
    #[allow(dead_code)]
    Confused,
    /// Entire map revealed
    #[allow(dead_code)]
    Revealed,
    /// Weapon coated in poison: attacks apply Poison
    Envenomed,
    /// Weapon coated in flame/magic: bonus damage
    Empowered { amount: i32 },
    /// Burn damage over time
    Burn { damage: i32 },
}

/// An active status effect with remaining duration.
#[derive(Clone, Debug)]
pub struct StatusInstance {
    pub kind: StatusKind,
    pub turns_left: i32,
}

impl StatusInstance {
    pub fn new(kind: StatusKind, turns: i32) -> Self {
        Self { kind, turns_left: turns }
    }

    pub fn label(&self) -> &'static str {
        match self.kind {
            StatusKind::Poison { .. } => "☠Psn",
            StatusKind::Burn { .. } => "🔥Brn",
            StatusKind::Regen { .. } => "♥Rgn",
            StatusKind::Haste => "⚡Hst",
            StatusKind::Confused => "?Cnf",
            StatusKind::Revealed => "👁Map",
            StatusKind::Envenomed => "☠Wep",
            StatusKind::Empowered { .. } => "💪Pow",
        }
    }

    pub fn color(&self) -> &'static str {
        match self.kind {
            StatusKind::Poison { .. } => "#88ff44",
            StatusKind::Burn { .. } => "#ff5500",
            StatusKind::Regen { .. } => "#ff88cc",
            StatusKind::Haste => "#ffff44",
            StatusKind::Confused => "#cc44ff",
            StatusKind::Revealed => "#44ccff",
            StatusKind::Envenomed => "#00ff00",
            StatusKind::Empowered { .. } => "#ff4400",
        }
    }
}

/// Tick all statuses on a list, applying effects. Returns (total_damage, total_heal).
/// Removes expired effects.
pub fn tick_statuses(statuses: &mut Vec<StatusInstance>) -> (i32, i32) {
    let mut damage = 0;
    let mut heal = 0;
    for s in statuses.iter_mut() {
        match s.kind {
            StatusKind::Poison { damage: d } => damage += d,
            StatusKind::Burn { damage: d } => damage += d,
            StatusKind::Regen { heal: h } => heal += h,
            _ => {}
        }
        s.turns_left -= 1;
    }
    statuses.retain(|s| s.turns_left > 0);
    (damage, heal)
}

/// Check if a specific status kind is active.
#[allow(dead_code)]
pub fn has_status(statuses: &[StatusInstance], check: &str) -> bool {
    statuses.iter().any(|s| s.label().contains(check))
}

pub fn has_haste(statuses: &[StatusInstance]) -> bool {
    statuses.iter().any(|s| matches!(s.kind, StatusKind::Haste))
}

#[allow(dead_code)]
pub fn has_confused(statuses: &[StatusInstance]) -> bool {
    statuses.iter().any(|s| matches!(s.kind, StatusKind::Confused))
}

#[allow(dead_code)]
pub fn has_revealed(statuses: &[StatusInstance]) -> bool {
    statuses.iter().any(|s| matches!(s.kind, StatusKind::Revealed))
}

pub fn has_envenomed(statuses: &[StatusInstance]) -> bool {
    statuses.iter().any(|s| matches!(s.kind, StatusKind::Envenomed))
}

pub fn empowered_amount(statuses: &[StatusInstance]) -> i32 {
    statuses.iter().filter_map(|s| match s.kind {
        StatusKind::Empowered { amount } => Some(amount),
        _ => None,
    }).sum()
}

#[cfg(test)]
mod tests {
    use super::{has_revealed, tick_statuses, StatusInstance, StatusKind};

    #[test]
    fn regen_status_restores_health_each_tick() {
        let mut statuses = vec![StatusInstance::new(StatusKind::Regen { heal: 2 }, 2)];

        let (damage, heal) = tick_statuses(&mut statuses);

        assert_eq!(damage, 0);
        assert_eq!(heal, 2);
    }

    #[test]
    fn final_regen_tick_removes_the_status() {
        let mut statuses = vec![StatusInstance::new(StatusKind::Regen { heal: 1 }, 1)];

        let _ = tick_statuses(&mut statuses);

        assert!(statuses.is_empty());
    }

    #[test]
    fn revealed_status_is_detected() {
        let statuses = vec![StatusInstance::new(StatusKind::Revealed, 4)];

        assert!(has_revealed(&statuses));
    }
}
