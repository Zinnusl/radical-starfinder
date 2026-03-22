//! Status effects that can be applied to crew and hostiles.

/// A timed status effect.
#[derive(Clone, Debug)]
pub enum StatusKind {
    /// Radiation damage per turn for N turns
    Poison {
        damage: i32,
    },
    /// Auto-repair per turn for N turns
    Regen {
        heal: i32,
    },
    /// Overdrive: extra actions (enemy turn skipped on even ticks)
    Haste,
    /// Scrambled: movement is randomized
    Confused,
    /// Entire sector map revealed
    Revealed,
    /// Weapon contaminated: attacks apply Radiation
    Envenomed,
    /// Weapon supercharged with energy: bonus damage
    Empowered {
        amount: i32,
    },
    /// Plasma burn damage over time
    Burn {
        damage: i32,
    },
    /// Energy shield blocks drain for N turns (overworld).
    SpiritShield,
    /// Cryofreeze: skip next turn, then clears
    Freeze,
    /// Impeded: movement reduced by 1
    Slow,
    /// Panicked: forced to move away from source
    Fear,
    /// Shrapnel wound: damage per turn (stacks)
    Bleed {
        damage: i32,
    },
    /// Electrified armor: damages attackers on hit
    Thorns,
    /// Reinforced: each stack adds +1 damage to next attack, consumed on hit
    Fortify {
        stacks: i32,
    },
    /// Cloaked: can't be targeted by hostiles; breaks on attack or ability use
    Invisible,
    /// Anchored: cannot move but gain +2 armor
    Rooted,
    /// Disrupted: deal 50% less damage
    Weakened,
    /// Malware: take 1 extra damage from all sources
    Cursed,
    /// Optimized: deal +1 damage and take -1 damage from all sources
    Blessed,
    /// Soaked: harmless alone, but enables combos (Soaked+PlasmaBurn=Steam, Soaked+Cryofreeze=deep freeze)
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
            StatusKind::Poison { .. } => "☢Rad",
            StatusKind::Burn { .. } => "🔥Pls",
            StatusKind::Regen { .. } => "🔧Rep",
            StatusKind::Haste => "⚡Ovr",
            StatusKind::Confused => "?Scr",
            StatusKind::Revealed => "👁Map",
            StatusKind::Envenomed => "☢Wep",
            StatusKind::Empowered { .. } => "⚡Sup",
            StatusKind::SpiritShield => "🛡Shd",
            StatusKind::Freeze => "❄Cry",
            StatusKind::Slow => "🐌Imp",
            StatusKind::Fear => "😨Pnk",
            StatusKind::Bleed { .. } => "🩸Shr",
            StatusKind::Thorns => "⚡Elc",
            StatusKind::Fortify { .. } => "🛡Rnf",
            StatusKind::Invisible => "👻Clk",
            StatusKind::Rooted => "⚓Anc",
            StatusKind::Weakened => "⬇Dis",
            StatusKind::Cursed => "💀Mlw",
            StatusKind::Blessed => "✨Opt",
            StatusKind::Wet => "💧Skd",
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
            StatusKind::Thorns => "#44aaff",
            StatusKind::Fortify { .. } => "#ffaa00",
            StatusKind::Invisible => "#aaccff",
            StatusKind::Rooted => "#7788aa",
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
                StatusKind::Poison { damage: d } => damage += d,  // radiation
                StatusKind::Burn { damage: d } => damage += d,    // plasma burn
                StatusKind::Bleed { damage: d } => damage += d,   // shrapnel wound
                StatusKind::Regen { heal: h } => heal += h,       // auto-repair
                StatusKind::Cursed => damage += 1,                 // malware corruption
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

pub fn has_confused(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Confused))
}

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

pub fn has_invisible(statuses: &[StatusInstance]) -> bool {
    statuses
        .iter()
        .any(|s| matches!(s.kind, StatusKind::Invisible))
}

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

        // First tick: fresh → skip auto-repair
        let (damage, heal) = tick_statuses(&mut statuses);
        assert_eq!(damage, 0);
        assert_eq!(heal, 0);
        assert_eq!(statuses.len(), 1);

        // Second tick: auto-repair applies
        let (damage, heal) = tick_statuses(&mut statuses);
        assert_eq!(damage, 0);
        assert_eq!(heal, 2);
    }

    #[test]
    fn final_autorepair_tick_removes_the_status() {
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
    fn plasma_burn_does_not_deal_instant_damage() {
        let mut statuses = vec![StatusInstance::new(StatusKind::Burn { damage: 1 }, 2)];

        // First tick: fresh → no damage
        let (damage, _) = tick_statuses(&mut statuses);
        assert_eq!(damage, 0);

        // Second tick: plasma burn damage applies
        let (damage, _) = tick_statuses(&mut statuses);
        assert_eq!(damage, 1);
        assert!(statuses.is_empty());
    }
}
