//! Status effects that can be applied to players and enemies.

/// A timed status effect.
#[derive(Clone, Debug)]
pub enum StatusKind {
    /// Damage per turn for N turns
    Poison {
        damage: i32,
    },
    /// Heal per turn for N turns
    #[allow(dead_code)]
    Regen {
        heal: i32,
    },
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
    Empowered {
        amount: i32,
    },
    /// Burn damage over time
    Burn {
        damage: i32,
    },
    /// Blocks spirit drain for N turns (overworld).
    SpiritShield,
    /// Frozen: skip next turn, then clears
    Freeze,
    /// Slow: movement reduced by 1
    Slow,
    /// Fear: forced to move away from source
    Fear,
    /// Bleeding: damage per turn (stacks)
    Bleed {
        damage: i32,
    },
    Thorns,
    /// Each stack adds +1 damage to next attack, consumed on hit
    Fortify {
        stacks: i32,
    },
    /// Can't be targeted by enemies; breaks on attack or spell cast
    Invisible,
    /// Cannot move but gain +2 armor
    #[allow(dead_code)]
    Rooted,
    /// Deal 50% less damage
    Weakened,
    /// Take 1 extra damage from all sources
    #[allow(dead_code)]
    Cursed,
    /// Deal +1 damage and take -1 damage from all sources
    Blessed,
    /// Wet: harmless alone, but enables combos (Wet+Burn=Steam, Wet+Freeze=deep freeze)
    Wet,
}

/// An active status effect with remaining duration.
#[derive(Clone, Debug)]
pub struct StatusInstance {
    pub kind: StatusKind,
    pub turns_left: i32,
    /// When true, the first tick skips damage/heal so statuses don't deal
    /// instant damage on the same turn they are applied.
    pub fresh: bool,
}

impl StatusInstance {
    pub fn new(kind: StatusKind, turns: i32) -> Self {
        Self {
            kind,
            turns_left: turns,
            fresh: true,
        }
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
            StatusKind::SpiritShield => "🌕Spr",
            StatusKind::Freeze => "❄Frz",
            StatusKind::Slow => "🐌Slw",
            StatusKind::Fear => "😨Fer",
            StatusKind::Bleed { .. } => "🩸Bld",
            StatusKind::Thorns => "🌿Thn",
            StatusKind::Fortify { .. } => "💪Frt",
            StatusKind::Invisible => "👻Inv",
            StatusKind::Rooted => "🌳Rte",
            StatusKind::Weakened => "⬇Wkn",
            StatusKind::Cursed => "💀Crs",
            StatusKind::Blessed => "✨Bls",
            StatusKind::Wet => "💧Wet",
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
            StatusKind::SpiritShield => "#8844ff",
            StatusKind::Freeze => "#00ffff",
            StatusKind::Slow => "#aaaaaa",
            StatusKind::Fear => "#660066",
            StatusKind::Bleed { .. } => "#aa0000",
            StatusKind::Thorns => "#44cc44",
            StatusKind::Fortify { .. } => "#ffaa00",
            StatusKind::Invisible => "#aaccff",
            StatusKind::Rooted => "#886633",
            StatusKind::Weakened => "#888888",
            StatusKind::Cursed => "#660044",
            StatusKind::Blessed => "#ffffaa",
            StatusKind::Wet => "#4488ff",
        }
    }

    pub fn is_negative(&self) -> bool {
        matches!(
            self.kind,
            StatusKind::Poison { .. }
                | StatusKind::Burn { .. }
                | StatusKind::Confused
                | StatusKind::Freeze
                | StatusKind::Slow
                | StatusKind::Fear
                | StatusKind::Bleed { .. }
                | StatusKind::Rooted
                | StatusKind::Weakened
                | StatusKind::Cursed
        )
    }
}

/// Tick all statuses on a list, applying effects. Returns (total_damage, total_heal).
/// Removes expired effects.  Fresh statuses skip their damage/heal on the
/// first tick so that newly-applied effects don't deal instant damage.
pub fn tick_statuses(statuses: &mut Vec<StatusInstance>) -> (i32, i32) {
    let mut damage = 0;
    let mut heal = 0;
    for s in statuses.iter_mut() {
        if s.fresh {
            // Skip damage/heal on the turn the status was applied.
            s.fresh = false;
        } else {
            match s.kind {
                StatusKind::Poison { damage: d } => damage += d,
                StatusKind::Burn { damage: d } => damage += d,
                StatusKind::Bleed { damage: d } => damage += d,
                StatusKind::Regen { heal: h } => heal += h,
                StatusKind::Cursed => damage += 1,
                _ => {}
            }
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
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Confused))
}

#[allow(dead_code)]
pub fn has_revealed(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Revealed))
}

pub fn has_envenomed(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Envenomed))
}

pub fn has_spirit_shield(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::SpiritShield))
}

pub fn empowered_amount(statuses: &[StatusInstance]) -> i32 {
    statuses
        .iter()
        .filter_map(|s| match s.kind {
            StatusKind::Empowered { amount } => Some(amount),
            _ => None,
        })
        .sum()
}

#[allow(dead_code)]
pub fn has_invisible(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Invisible))
}

#[allow(dead_code)]
pub fn has_rooted(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Rooted))
}

#[allow(dead_code)]
pub fn has_weakened(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Weakened))
}

#[allow(dead_code)]
pub fn has_cursed(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Cursed))
}

#[allow(dead_code)]
pub fn has_blessed(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Blessed))
}

#[allow(dead_code)]
pub fn has_wet(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Wet))
}

#[allow(dead_code)]
pub fn has_burn(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Burn { .. }))
}

#[allow(dead_code)]
pub fn has_freeze(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Freeze))
}

#[allow(dead_code)]
pub fn has_poison(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Poison { .. }))
}

#[allow(dead_code)]
pub fn has_slow(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Slow))
}

#[allow(dead_code)]
pub fn has_fortify(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Fortify { .. }))
}

#[allow(dead_code)]
pub fn fortify_stacks(statuses: &[StatusInstance]) -> i32 {
    statuses
        .iter()
        .filter_map(|s| match s.kind {
            StatusKind::Fortify { stacks } => Some(stacks),
            _ => None,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::{has_revealed, tick_statuses, StatusInstance, StatusKind};

    #[test]
    fn fresh_status_skips_first_tick() {
        let mut statuses = vec![StatusInstance::new(StatusKind::Regen { heal: 2 }, 3)];

        // First tick: fresh → skip heal
        let (damage, heal) = tick_statuses(&mut statuses);
        assert_eq!(damage, 0);
        assert_eq!(heal, 0);
        assert_eq!(statuses.len(), 1);

        // Second tick: heal applies
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

    #[test]
    fn burn_does_not_deal_instant_damage() {
        let mut statuses = vec![StatusInstance::new(StatusKind::Burn { damage: 1 }, 2)];

        // First tick: fresh → no damage
        let (damage, _) = tick_statuses(&mut statuses);
        assert_eq!(damage, 0);

        // Second tick: damage applies
        let (damage, _) = tick_statuses(&mut statuses);
        assert_eq!(damage, 1);
        assert!(statuses.is_empty());
    }
}
