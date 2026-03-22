use crate::combat::action::deal_damage;
use crate::combat::terrain::{apply_terrain_interactions, TerrainSource};
use crate::combat::{AudioEvent, ArcingProjectile, Projectile, ProjectileEffect, TacticalBattle};
use crate::enemy::{PlayerRadicalAbility, RadicalAction};
use crate::status::{StatusInstance, StatusKind};

pub fn apply_radical_action(
    battle: &mut TacticalBattle,
    unit_idx: usize,
    action: RadicalAction,
) -> String {
    let proj_count_before = battle.projectiles.len() + battle.arcing_projectiles.len();
    let result = match action {
        RadicalAction::SpreadingWildfire => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 3));
            battle.audio_events.push(AudioEvent::StatusBurn);
            let ux = battle.units[unit_idx].x;
            let uy = battle.units[unit_idx].y;
            let facing = battle.units[unit_idx].facing;
            let mut fire_tiles = Vec::new();
            for i in 1..=3 {
                fire_tiles.push((ux + facing.dx() * i, uy + facing.dy() * i));
            }
            let terrain_msgs =
                apply_terrain_interactions(battle, TerrainSource::FireSpell, &fire_tiles);
            for tm in terrain_msgs {
                battle.log_message(&tm);
            }
            format!("{} — Spreads wildfire!", action.name())
        }
        RadicalAction::ErosiveFlow => {
            battle.units[0].radical_armor = battle.units[0].radical_armor.saturating_sub(1);
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Slow, 3));
            battle.audio_events.push(AudioEvent::StatusSlow);
            format!("{} — Armor eroded and slowed!", action.name())
        }
        RadicalAction::OverwhelmingForce => {
            let missing = battle.units[unit_idx].max_hp - battle.units[unit_idx].hp;
            let dmg = 1 + missing / 2;
            let ex = battle.units[unit_idx].x;
            let ey = battle.units[unit_idx].y;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: ex as f64,
                from_y: ey as f64,
                to_x: px,
                to_y: py,
                progress: 0.0,
                speed: 0.12,
                arc_height: 0.2,
                effect: ProjectileEffect::Damage(dmg),
                owner_idx: unit_idx,
                glyph: "💪",
                color: "#ff4444",
                done: false,
            });
            format!(
                "{} — Strikes with overwhelming force!",
                action.name(),
            )
        }
        RadicalAction::DoubtSeed => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Confused, 2));
            format!("{} — Sows a seed of doubt!", action.name())
        }
        RadicalAction::DevouringMaw => {
            let ex = battle.units[unit_idx].x;
            let ey = battle.units[unit_idx].y;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: ex as f64,
                from_y: ey as f64,
                to_x: px,
                to_y: py,
                progress: 0.0,
                speed: 0.10,
                arc_height: 0.3,
                effect: ProjectileEffect::Damage(1),
                owner_idx: unit_idx,
                glyph: "👄",
                color: "#aa44aa",
                done: false,
            });
            let mut stole_dodge = false;
            let mut stole_armor = 0;

            let player = &mut battle.units[0];
            if player.radical_dodge {
                player.radical_dodge = false;
                stole_dodge = true;
            } else if player.radical_counter {
                player.radical_counter = false;
            } else if player.radical_armor > 0 {
                stole_armor = player.radical_armor;
                player.radical_armor = 0;
            } else if player.defending {
                player.defending = false;
            }

            if stole_dodge {
                battle.units[unit_idx].radical_dodge = true;
            }
            if stole_armor > 0 {
                battle.units[unit_idx].radical_armor += stole_armor;
            }
            format!("{} — Devours protection!", action.name())
        }
        RadicalAction::WitnessMark => {
            battle.units[0].marked_extra_damage = 3;
            format!("{} — You are marked!", action.name())
        }
        RadicalAction::SleightReversal => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.units[0].x = battle.units[unit_idx].x;
            battle.units[0].y = battle.units[unit_idx].y;
            battle.units[unit_idx].x = px;
            battle.units[unit_idx].y = py;
            format!("{} — Positions swapped!", action.name())
        }
        RadicalAction::RootingGrasp => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Slow, 2));
            battle.audio_events.push(AudioEvent::StatusSlow);
            battle.units[unit_idx].radical_armor += 1;
            format!("{} — Grasping roots!", action.name())
        }
        RadicalAction::HarvestReaping => {
            let p_hp = battle.units[0].hp;
            let p_max = battle.units[0].max_hp;
            let dmg = if p_hp * 100 < p_max * 40 { 3 } else { 1 };
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.arcing_projectiles.push(ArcingProjectile {
                target_x: px,
                target_y: py,
                turns_remaining: 1,
                effect: ProjectileEffect::Damage(dmg),
                glyph: "🌾",
                color: "#ddaa00",
                owner_is_player: false,
            });
            format!(
                "{} — Reaping incoming! (lands next turn)",
                action.name()
            )
        }
        RadicalAction::RevealingDawn => {
            battle.units[unit_idx].statuses.retain(|s| !s.is_negative());
            let ux = battle.units[unit_idx].x;
            let uy = battle.units[unit_idx].y;
            for i in 1..battle.units.len() {
                if battle.units[i].alive {
                    let dist = (battle.units[i].x - ux).abs() + (battle.units[i].y - uy).abs();
                    if dist <= 3 {
                        battle.units[i].statuses.retain(|s| {
                            !matches!(
                                s.kind,
                                StatusKind::Burn { .. }
                                    | StatusKind::Poison { .. }
                                    | StatusKind::Bleed { .. }
                            )
                        });
                    }
                }
            }
            format!("{} — A revealing dawn!", action.name())
        }
        RadicalAction::WaningCurse => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Poison { damage: 1 }, 2));
            format!("{} — A waning curse!", action.name())
        }
        RadicalAction::MortalResilience => {
            let max_hp = battle.units[unit_idx].max_hp;
            let hp = battle.units[unit_idx].hp;
            if hp * 3 <= max_hp {
                battle.units[unit_idx].hp = hp.max(1);
                battle.units[unit_idx].damage += 2;
                format!("{} — Pushed to the brink!", action.name())
            } else {
                battle.units[unit_idx].radical_armor += 1;
                format!("{} — Steels themselves!", action.name())
            }
        }
        RadicalAction::MaternalShield => {
            battle.units[unit_idx].radical_counter = true;
            battle.units[unit_idx].thorn_armor_turns = 3;
            format!("{} — A protective shield forms!", action.name())
        }
        RadicalAction::PotentialBurst => {
            battle.units[0].marked_extra_damage += 2;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.arcing_projectiles.push(ArcingProjectile {
                target_x: px,
                target_y: py,
                turns_remaining: 1,
                effect: ProjectileEffect::Damage(1),
                glyph: "💥",
                color: "#ff8800",
                owner_is_player: false,
            });
            format!(
                "{} — Potential bursts! (lands next turn)",
                action.name()
            )
        }
        RadicalAction::ChasingChaff => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Confused, 1));
            format!("{} — Chases the chaff!", action.name())
        }
        RadicalAction::CrossroadsGambit => {
            let seed = (battle.turn_number as u64)
                .wrapping_mul(31)
                .wrapping_add(unit_idx as u64)
                .wrapping_mul(17)
                % 2;
            if seed == 0 {
                let px = battle.units[0].x;
                let py = battle.units[0].y;
                battle.arcing_projectiles.push(ArcingProjectile {
                    target_x: px,
                    target_y: py,
                    turns_remaining: 1,
                    effect: ProjectileEffect::Damage(3),
                    glyph: "🎲",
                    color: "#ffff00",
                    owner_is_player: false,
                });
                format!(
                    "{} — The gambit succeeds! (lands next turn)",
                    action.name()
                )
            } else {
                battle.units[unit_idx].stunned = true;
                format!("{} — The gambit fails! (Stunned)", action.name())
            }
        }
        RadicalAction::RigidStance => {
            battle.units[unit_idx].radical_armor += 4;
            battle.units[unit_idx].radical_dodge = false;
            format!("{} — Takes a rigid stance!", action.name())
        }
        RadicalAction::GroundingWeight => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Slow, 3));
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.arcing_projectiles.push(ArcingProjectile {
                target_x: px,
                target_y: py,
                turns_remaining: 1,
                effect: ProjectileEffect::Damage(1),
                glyph: "⛰",
                color: "#886644",
                owner_is_player: false,
            });
            format!(
                "{} — A crushing weight incoming! (lands next turn)",
                action.name()
            )
        }
        RadicalAction::EchoStrike => {
            let dmg = battle.units[unit_idx].damage;
            let ex = battle.units[unit_idx].x;
            let ey = battle.units[unit_idx].y;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: ex as f64,
                from_y: ey as f64,
                to_x: px,
                to_y: py,
                progress: 0.0,
                speed: 0.14,
                arc_height: 0.2,
                effect: ProjectileEffect::Damage(dmg),
                owner_idx: unit_idx,
                glyph: "🔁",
                color: "#ffaa00",
                done: false,
            });
            format!("{} — An echoing strike!", action.name())
        }
        RadicalAction::PreciseExecution => {
            let p_hp = battle.units[0].hp;
            let p_max = battle.units[0].max_hp;
            let dmg = if p_hp * 100 < p_max * 25 { 4 } else { 1 };
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.arcing_projectiles.push(ArcingProjectile {
                target_x: px,
                target_y: py,
                turns_remaining: 1,
                effect: ProjectileEffect::Damage(dmg),
                glyph: "🎯",
                color: "#ff2222",
                owner_is_player: false,
            });
            format!(
                "{} — Precise execution incoming! (lands next turn)",
                action.name()
            )
        }
        RadicalAction::CleavingCut => {
            let ex = battle.units[unit_idx].x;
            let ey = battle.units[unit_idx].y;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: ex as f64,
                from_y: ey as f64,
                to_x: px,
                to_y: py,
                progress: 0.0,
                speed: 0.13,
                arc_height: 0.2,
                effect: ProjectileEffect::Damage(2),
                owner_idx: unit_idx,
                glyph: "🗡",
                color: "#cccccc",
                done: false,
            });
            battle.units[0].max_hp = battle.units[0].max_hp.saturating_sub(1).max(1);
            battle.units[0].hp = battle.units[0].hp.min(battle.units[0].max_hp);
            format!(
                "{} — A cleaving cut! (Max HP -1)",
                action.name(),
            )
        }
        RadicalAction::BindingOath => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Slow, 3));
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Confused, 1));
            format!("{} — Bound by oath!", action.name())
        }
        RadicalAction::PursuingSteps => {
            let mut nx = battle.units[unit_idx].x;
            let mut ny = battle.units[unit_idx].y;
            for _ in 0..2 {
                let px = battle.units[0].x;
                let py = battle.units[0].y;
                let dx = (px - nx).signum();
                let dy = (py - ny).signum();
                if (px - nx).abs() > (py - ny).abs() {
                    nx += dx;
                } else {
                    ny += dy;
                }
            }
            battle.units[unit_idx].x = nx.clamp(0, battle.arena.width as i32 - 1);
            battle.units[unit_idx].y = ny.clamp(0, battle.arena.height as i32 - 1);
            let ex = battle.units[unit_idx].x;
            let ey = battle.units[unit_idx].y;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: ex as f64,
                from_y: ey as f64,
                to_x: px,
                to_y: py,
                progress: 0.0,
                speed: 0.14,
                arc_height: 0.1,
                effect: ProjectileEffect::Damage(1),
                owner_idx: unit_idx,
                glyph: "👣",
                color: "#886644",
                done: false,
            });
            format!("{} — Pursues and strikes!", action.name())
        }
        RadicalAction::EntanglingWeb => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Slow, 3));
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Bleed { damage: 1 }, 2));
            format!("{} — Caught in a web!", action.name())
        }
        RadicalAction::ThresholdSeal => {
            battle.units[unit_idx].radical_armor += 3;
            battle.units[unit_idx]
                .statuses
                .push(StatusInstance::new(StatusKind::Slow, 1));
            format!("{} — Seals the threshold!", action.name())
        }
        RadicalAction::CavalryCharge => {
            let dist = (battle.units[unit_idx].x - battle.units[0].x).abs()
                + (battle.units[unit_idx].y - battle.units[0].y).abs();
            let dmg = dist.min(3);
            let facing = battle.units[unit_idx].facing;
            battle.units[0].x =
                (battle.units[0].x + facing.dx() * 2).clamp(0, battle.arena.width as i32 - 1);
            battle.units[0].y =
                (battle.units[0].y + facing.dy() * 2).clamp(0, battle.arena.height as i32 - 1);
            let ex = battle.units[unit_idx].x;
            let ey = battle.units[unit_idx].y;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: ex as f64,
                from_y: ey as f64,
                to_x: px,
                to_y: py,
                progress: 0.0,
                speed: 0.12,
                arc_height: 0.2,
                effect: ProjectileEffect::Damage(dmg),
                owner_idx: unit_idx,
                glyph: "🐎",
                color: "#aa6622",
                done: false,
            });
            format!("{} — A devastating charge!", action.name())
        }
        RadicalAction::SoaringEscape => {
            battle.units[unit_idx].radical_dodge = true;
            battle.units[unit_idx].stored_movement += 2;
            format!("{} — Takes to the skies!", action.name())
        }
        RadicalAction::DownpourBarrage => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let offsets = [(0, 0), (-1, 0), (1, 0)];
            for (dx, dy) in &offsets {
                let tx = px + dx;
                let ty = py + dy;
                if tx >= 0
                    && ty >= 0
                    && (tx as usize) < battle.arena.width
                    && (ty as usize) < battle.arena.height
                {
                    battle.arcing_projectiles.push(ArcingProjectile {
                        target_x: tx,
                        target_y: ty,
                        turns_remaining: 2,
                        effect: ProjectileEffect::Damage(1),
                        glyph: "🌧",
                        color: "#4488ff",
                        owner_is_player: false,
                    });
                }
            }
            format!(
                "{} — Rain barrage incoming! (lands in 2 turns)",
                action.name()
            )
        }
        RadicalAction::PetrifyingGaze => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Slow, 3));
            battle.units[unit_idx].radical_armor += 2;
            format!("{} — Petrifying gaze!", action.name())
        }
        RadicalAction::ParasiticSwarm => {
            let ex = battle.units[unit_idx].x;
            let ey = battle.units[unit_idx].y;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: ex as f64,
                from_y: ey as f64,
                to_x: px,
                to_y: py,
                progress: 0.0,
                speed: 0.09,
                arc_height: 0.4,
                effect: ProjectileEffect::Damage(1),
                owner_idx: unit_idx,
                glyph: "🐛",
                color: "#88aa00",
                done: false,
            });
            let heal = 2; // raw dmg (1) + 1
            battle.units[unit_idx].hp =
                (battle.units[unit_idx].hp + heal).min(battle.units[unit_idx].max_hp);
            format!("{} — A swarm drains you!", action.name())
        }
        RadicalAction::MercenaryPact => {
            let u_hp = battle.units[unit_idx].hp;
            battle.units[unit_idx].hp = (u_hp - 2).max(1);
            let ux = battle.units[unit_idx].x;
            let uy = battle.units[unit_idx].y;
            for i in 1..battle.units.len() {
                if battle.units[i].alive {
                    let dist = (battle.units[i].x - ux).abs() + (battle.units[i].y - uy).abs();
                    if dist <= 3 {
                        battle.units[i].damage += 1;
                        battle.units[i].radical_armor += 1;
                    }
                }
            }
            format!("{} — Blood paid for power!", action.name())
        }
        RadicalAction::ImmovablePeak => {
            battle.units[unit_idx].radical_armor += 3;
            battle.units[unit_idx].fortify_stacks += 1;
            format!("{} — Unyielding as a mountain!", action.name())
        }
        RadicalAction::SavageMaul => {
            let ex = battle.units[unit_idx].x;
            let ey = battle.units[unit_idx].y;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: ex as f64,
                from_y: ey as f64,
                to_x: px,
                to_y: py,
                progress: 0.0,
                speed: 0.11,
                arc_height: 0.3,
                effect: ProjectileEffect::Damage(3),
                owner_idx: unit_idx,
                glyph: "🐕",
                color: "#884400",
                done: false,
            });
            battle.units[unit_idx].hp = (battle.units[unit_idx].hp - 1).max(1);
            battle.units[unit_idx].hp =
                (battle.units[unit_idx].hp + 1).min(battle.units[unit_idx].max_hp);
            format!("{} — Savage maul!", action.name())
        }
        RadicalAction::ArcingShot => {
            let dist = (battle.units[unit_idx].x - battle.units[0].x).abs()
                + (battle.units[unit_idx].y - battle.units[0].y).abs();
            let dmg = if dist >= 3 { 3 } else { 2 };
            battle.arcing_projectiles.push(ArcingProjectile {
                target_x: battle.units[0].x,
                target_y: battle.units[0].y,
                turns_remaining: 2,
                effect: ProjectileEffect::Damage(dmg),
                glyph: "🏹",
                color: "#ff8844",
                owner_is_player: false,
            });
            format!(
                "{} — Launches an arcing shot! (lands in 2 turns)",
                action.name()
            )
        }
        RadicalAction::ConsumingBite => {
            let ex = battle.units[unit_idx].x;
            let ey = battle.units[unit_idx].y;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: ex as f64,
                from_y: ey as f64,
                to_x: px,
                to_y: py,
                progress: 0.0,
                speed: 0.10,
                arc_height: 0.3,
                effect: ProjectileEffect::Damage(2),
                owner_idx: unit_idx,
                glyph: "🦷",
                color: "#44aa44",
                done: false,
            });
            battle.units[unit_idx].max_hp += 1;
            let heal = 2; // raw damage value
            battle.units[unit_idx].hp =
                (battle.units[unit_idx].hp + heal).min(battle.units[unit_idx].max_hp);
            format!("{} — Consuming bite!", action.name())
        }
        RadicalAction::CloakingGuise => {
            battle.units[unit_idx].radical_dodge = true;
            battle.units[unit_idx].fortify_stacks += 1;
            format!("{} — Concealed from sight!", action.name())
        }
        RadicalAction::FlexibleCounter => {
            battle.units[unit_idx].radical_counter = true;
            battle.units[unit_idx].thorn_armor_turns = 2;
            format!("{} — Readies a counter!", action.name())
        }
        RadicalAction::BlitzAssault => {
            let mut nx = battle.units[unit_idx].x;
            let mut ny = battle.units[unit_idx].y;
            let mut moved = 0;
            for _ in 0..3 {
                let px = battle.units[0].x;
                let py = battle.units[0].y;
                if nx == px && ny == py {
                    break;
                }
                let dx = (px - nx).signum();
                let dy = (py - ny).signum();
                if (px - nx).abs() > (py - ny).abs() {
                    nx += dx;
                } else {
                    ny += dy;
                }
                moved += 1;
            }
            battle.units[unit_idx].x = nx.clamp(0, battle.arena.width as i32 - 1);
            battle.units[unit_idx].y = ny.clamp(0, battle.arena.height as i32 - 1);
            let dmg = moved.min(2);
            let ex = battle.units[unit_idx].x;
            let ey = battle.units[unit_idx].y;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: ex as f64,
                from_y: ey as f64,
                to_x: px,
                to_y: py,
                progress: 0.0,
                speed: 0.13,
                arc_height: 0.1,
                effect: ProjectileEffect::Damage(dmg),
                owner_idx: unit_idx,
                glyph: "⚡",
                color: "#ffff44",
                done: false,
            });
            format!("{} — Blitz assault!", action.name())
        }
        RadicalAction::CrushingWheels => {
            let facing = battle.units[unit_idx].facing;
            battle.units[0].x =
                (battle.units[0].x + facing.dx() * 3).clamp(0, battle.arena.width as i32 - 1);
            battle.units[0].y =
                (battle.units[0].y + facing.dy() * 3).clamp(0, battle.arena.height as i32 - 1);
            let ex = battle.units[unit_idx].x;
            let ey = battle.units[unit_idx].y;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.projectiles.push(Projectile {
                from_x: ex as f64,
                from_y: ey as f64,
                to_x: px,
                to_y: py,
                progress: 0.0,
                speed: 0.11,
                arc_height: 0.2,
                effect: ProjectileEffect::Damage(2),
                owner_idx: unit_idx,
                glyph: "🛞",
                color: "#666666",
                done: false,
            });
            format!("{} — Crushing wheels!", action.name())
        }
        RadicalAction::ImperialCommand => {
            let ux = battle.units[unit_idx].x;
            let uy = battle.units[unit_idx].y;
            let mut best_ally = None;
            let mut best_dist = i32::MAX;
            for i in 1..battle.units.len() {
                if i != unit_idx && battle.units[i].alive {
                    let dist = (battle.units[i].x - ux).abs() + (battle.units[i].y - uy).abs();
                    if dist < best_dist {
                        best_dist = dist;
                        best_ally = Some(i);
                    }
                }
            }
            if let Some(ally_idx) = best_ally {
                battle.units[ally_idx].damage += 2;
                let px = battle.units[0].x;
                let py = battle.units[0].y;
                let ax = battle.units[ally_idx].x;
                let ay = battle.units[ally_idx].y;
                let dx = (px - ax).signum();
                let dy = (py - ay).signum();
                if (px - ax).abs() > (py - ay).abs() {
                    battle.units[ally_idx].x = (ax + dx).clamp(0, battle.arena.width as i32 - 1);
                } else {
                    battle.units[ally_idx].y = (ay + dy).clamp(0, battle.arena.height as i32 - 1);
                }
                format!("{} — Issues an imperial command!", action.name())
            } else {
                format!("{} — Command echoed in silence!", action.name())
            }
        }
        RadicalAction::MagnifyingAura => {
            let ux = battle.units[unit_idx].x;
            let uy = battle.units[unit_idx].y;
            for i in 1..battle.units.len() {
                if battle.units[i].alive {
                    let dist = (battle.units[i].x - ux).abs() + (battle.units[i].y - uy).abs();
                    if dist <= 3 {
                        battle.units[i].damage += 1;
                    }
                }
            }
            format!("{} — A magnifying aura!", action.name())
        }
        RadicalAction::NeedleStrike => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.arcing_projectiles.push(ArcingProjectile {
                target_x: px,
                target_y: py,
                turns_remaining: 1,
                effect: ProjectileEffect::PiercingDamage(2),
                glyph: "📌",
                color: "#aaaaaa",
                owner_is_player: false,
            });
            format!(
                "{} — Needle strike incoming! (lands next turn)",
                action.name()
            )
        }
        RadicalAction::ArtisanTrap => {
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 2));
            format!("{} — Artisan's trap!", action.name())
        }
        RadicalAction::CleansingLight => {
            battle.units[unit_idx].statuses.retain(|s| !s.is_negative());
            battle.units[unit_idx].hp =
                (battle.units[unit_idx].hp + 3).min(battle.units[unit_idx].max_hp);
            format!("{} — Cleansed by light!", action.name())
        }
        RadicalAction::ScatteringPages => {
            let ux = battle.units[unit_idx].x;
            let uy = battle.units[unit_idx].y;
            let mut hit = 0;
            for i in 0..battle.units.len() {
                if battle.units[i].alive {
                    let dist =
                        (battle.units[i].x - ux).abs() + (battle.units[i].y - uy).abs();
                    if dist <= 2 {
                        battle.units[i]
                            .statuses
                            .push(StatusInstance::new(StatusKind::Confused, 1));
                        hit += 1;
                    }
                }
            }
            format!("{} — Pages scatter! {} units confused!", action.name(), hit)
        }
        RadicalAction::TrueVision => {
            battle.units[0].radical_armor = 0;
            battle.units[0].radical_dodge = false;
            battle.units[0].radical_counter = false;
            battle.units[0].thorn_armor_turns = 0;
            battle.units[0].defending = false;
            battle.units[0].statuses.retain(|s| s.is_negative());
            format!("{} — All protections dispelled!", action.name())
        }
        RadicalAction::QiDisruption => {
            let drained = battle.focus.min(3);
            battle.focus -= drained;
            format!("{} — {} focus disrupted!", action.name(), drained)
        }
        RadicalAction::ExpandingDomain => {
            battle.units[unit_idx].damage += 1;
            battle.units[unit_idx].radical_armor += 1;
            let ux = battle.units[unit_idx].x;
            let uy = battle.units[unit_idx].y;
            for i in 1..battle.units.len() {
                if i != unit_idx && battle.units[i].alive {
                    let dist =
                        (battle.units[i].x - ux).abs() + (battle.units[i].y - uy).abs();
                    if dist <= 2 {
                        battle.units[i].damage += 1;
                    }
                }
            }
            format!("{} — Domain expands! Power surges!", action.name())
        }
        RadicalAction::SinkholeSnare => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.arena.set_tile(px, py, crate::combat::BattleTile::CrackedFloor);
            battle.arcing_projectiles.push(ArcingProjectile {
                target_x: px,
                target_y: py,
                turns_remaining: 1,
                effect: ProjectileEffect::Damage(1),
                glyph: "🕳",
                color: "#664422",
                owner_is_player: false,
            });
            format!(
                "{} — Ground cracks beneath! Move or fall! (1 turn)",
                action.name()
            )
        }
        RadicalAction::SonicBurst => {
            let ux = battle.units[unit_idx].x;
            let uy = battle.units[unit_idx].y;
            let mut targets = Vec::new();
            for i in 0..battle.units.len() {
                if battle.units[i].alive {
                    let dist =
                        (battle.units[i].x - ux).abs() + (battle.units[i].y - uy).abs();
                    if dist <= 2 {
                        targets.push(i);
                    }
                }
            }
            for &i in &targets {
                if i == unit_idx {
                    continue;
                }
                let tx = battle.units[i].x;
                let ty = battle.units[i].y;
                battle.projectiles.push(Projectile {
                    from_x: ux as f64,
                    from_y: uy as f64,
                    to_x: tx,
                    to_y: ty,
                    progress: 0.0,
                    speed: 0.14,
                    arc_height: 0.1,
                    effect: ProjectileEffect::Damage(2),
                    owner_idx: unit_idx,
                    glyph: "🔊",
                    color: "#ffcc00",
                    done: false,
                });
                battle.units[i].stunned = true;
            }
            // Self-damage from the burst
            battle.units[unit_idx].hp = (battle.units[unit_idx].hp - 1).max(1);
            format!("{} — Sonic shockwave!", action.name())
        }
        RadicalAction::VenomousLash => {
            let ex = battle.units[unit_idx].x;
            let ey = battle.units[unit_idx].y;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.arcing_projectiles.push(ArcingProjectile {
                target_x: px,
                target_y: py,
                turns_remaining: 1,
                effect: ProjectileEffect::Damage(1),
                glyph: "🐍",
                color: "#44cc44",
                owner_is_player: false,
            });
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Poison { damage: 2 }, 2));
            let _ = (ex, ey); // enemy position used for projectile origin display
            format!(
                "{} — Venomous lash! Poison incoming! (1 turn)",
                action.name()
            )
        }
        RadicalAction::IronBodyStance => {
            battle.units[unit_idx].radical_armor += 2;
            battle.units[unit_idx]
                .statuses
                .push(StatusInstance::new(StatusKind::Slow, 2));
            battle.units[unit_idx].fortify_stacks += 1;
            format!("{} — Iron body! Immovable defense!", action.name())
        }
        RadicalAction::GoreCrush => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let mut nx = battle.units[unit_idx].x;
            let mut ny = battle.units[unit_idx].y;
            for _ in 0..2 {
                if nx == px && ny == py {
                    break;
                }
                let dx = (px - nx).signum();
                let dy = (py - ny).signum();
                if (px - nx).abs() >= (py - ny).abs() {
                    nx += dx;
                } else {
                    ny += dy;
                }
            }
            battle.units[unit_idx].x = nx.clamp(0, battle.arena.width as i32 - 1);
            battle.units[unit_idx].y = ny.clamp(0, battle.arena.height as i32 - 1);
            let ex = battle.units[unit_idx].x;
            let ey = battle.units[unit_idx].y;
            battle.projectiles.push(Projectile {
                from_x: ex as f64,
                from_y: ey as f64,
                to_x: px,
                to_y: py,
                progress: 0.0,
                speed: 0.12,
                arc_height: 0.3,
                effect: ProjectileEffect::Damage(2),
                owner_idx: unit_idx,
                glyph: "🐂",
                color: "#aa4400",
                done: false,
            });
            // Knockback player 1 tile away from charge direction
            let kb_dx = (px - ex).signum();
            let kb_dy = (py - ey).signum();
            let new_px = (px + kb_dx).clamp(0, battle.arena.width as i32 - 1);
            let new_py = (py + kb_dy).clamp(0, battle.arena.height as i32 - 1);
            if battle.arena.tile(new_px, new_py).map(|t| t.is_walkable()).unwrap_or(false)
                && battle.unit_at(new_px, new_py).is_none()
            {
                battle.units[0].x = new_px;
                battle.units[0].y = new_py;
            }
            format!("{} — Charges and gores!", action.name())
        }
        RadicalAction::IntoxicatingMist => {
            let ux = battle.units[unit_idx].x;
            let uy = battle.units[unit_idx].y;
            for dx in -2..=2i32 {
                for dy in -2..=2i32 {
                    if dx.abs() + dy.abs() <= 2 && (dx != 0 || dy != 0) {
                        let tx = ux + dx;
                        let ty = uy + dy;
                        if tx >= 0
                            && ty >= 0
                            && tx < battle.arena.width as i32
                            && ty < battle.arena.height as i32
                        {
                            battle
                                .arena
                                .set_tile(tx, ty, crate::combat::BattleTile::Steam);
                        }
                    }
                }
            }
            battle.units[0]
                .statuses
                .push(StatusInstance::new(StatusKind::Confused, 2));
            format!("{} — Intoxicating mist fills the air!", action.name())
        }
        RadicalAction::SproutingBarrier => {
            let ux = battle.units[unit_idx].x;
            let uy = battle.units[unit_idx].y;
            for dx in -2..=2i32 {
                for dy in -2..=2i32 {
                    if dx.abs() + dy.abs() <= 2 && (dx != 0 || dy != 0) {
                        let tx = ux + dx;
                        let ty = uy + dy;
                        if tx >= 0
                            && ty >= 0
                            && tx < battle.arena.width as i32
                            && ty < battle.arena.height as i32
                        {
                            if battle.arena.tile(tx, ty) == Some(crate::combat::BattleTile::Open) {
                                battle
                                    .arena
                                    .set_tile(tx, ty, crate::combat::BattleTile::Grass);
                            }
                        }
                    }
                }
            }
            // Count adjacent grass for armor
            let mut grass_count = 0;
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                if battle.arena.tile(ux + dx, uy + dy)
                    == Some(crate::combat::BattleTile::Grass)
                {
                    grass_count += 1;
                }
            }
            battle.units[unit_idx].radical_armor += grass_count;
            format!(
                "{} — Sprouts grow! +{} armor from grass!",
                action.name(),
                grass_count
            )
        }
        RadicalAction::TidalSurge => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let on_water =
                battle.arena.tile(px, py) == Some(crate::combat::BattleTile::Water);
            battle
                .arena
                .set_tile(px, py, crate::combat::BattleTile::Water);
            if on_water {
                // Extra damage if already on water
                let ex = battle.units[unit_idx].x;
                let ey = battle.units[unit_idx].y;
                battle.projectiles.push(Projectile {
                    from_x: ex as f64,
                    from_y: ey as f64,
                    to_x: px,
                    to_y: py,
                    progress: 0.0,
                    speed: 0.11,
                    arc_height: 0.2,
                    effect: ProjectileEffect::Damage(2),
                    owner_idx: unit_idx,
                    glyph: "🌊",
                    color: "#2288ff",
                    done: false,
                });
            }
            // Push player 2 tiles away
            let ux = battle.units[unit_idx].x;
            let uy = battle.units[unit_idx].y;
            let push_dx = (px - ux).signum();
            let push_dy = (py - uy).signum();
            for _ in 0..2 {
                let new_px =
                    (battle.units[0].x + push_dx).clamp(0, battle.arena.width as i32 - 1);
                let new_py =
                    (battle.units[0].y + push_dy).clamp(0, battle.arena.height as i32 - 1);
                if battle
                    .arena
                    .tile(new_px, new_py)
                    .map(|t| t.is_walkable())
                    .unwrap_or(false)
                    && battle.unit_at(new_px, new_py).is_none()
                {
                    battle.units[0].x = new_px;
                    battle.units[0].y = new_py;
                }
            }
            if on_water {
                format!("{} — Tidal surge! Swept away with force!", action.name())
            } else {
                format!("{} — Water surges! Pushed back!", action.name())
            }
        }
        RadicalAction::BoneShatter => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            battle.units[0].radical_armor = 0;
            battle.units[0].defending = false;
            battle.arcing_projectiles.push(ArcingProjectile {
                target_x: px,
                target_y: py,
                turns_remaining: 1,
                effect: ProjectileEffect::Damage(3),
                glyph: "🦴",
                color: "#eeddcc",
                owner_is_player: false,
            });
            format!(
                "{} — Bone shard incoming! Armor shattered! (1 turn)",
                action.name()
            )
        }
        RadicalAction::AdaptiveShift => {
            battle.units[unit_idx].radical_armor += 2;
            battle.units[unit_idx].fortify_stacks += 1;
            format!("{} — Adapts and hardens!", action.name())
        }
        RadicalAction::BerserkerFury => {
            battle.units[unit_idx].hp = (battle.units[unit_idx].hp - 2).max(1);
            battle.units[unit_idx].damage += 3;
            battle.units[unit_idx]
                .statuses
                .push(StatusInstance::new(StatusKind::Haste, 2));
            format!("{} — BERSERKER FURY! Pain fuels rage!", action.name())
        }
        RadicalAction::FlockAssault => {
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            let offsets = [(-1, 0), (1, 0), (0, -1)];
            for (i, &(ox, oy)) in offsets.iter().enumerate() {
                let tx = (px + ox).clamp(0, battle.arena.width as i32 - 1);
                let ty = (py + oy).clamp(0, battle.arena.height as i32 - 1);
                let delay = if i < 2 { 1 } else { 2 };
                battle.arcing_projectiles.push(ArcingProjectile {
                    target_x: tx,
                    target_y: ty,
                    turns_remaining: delay,
                    effect: ProjectileEffect::Damage(1),
                    glyph: "🐦",
                    color: "#886644",
                    owner_is_player: false,
                });
            }
            format!(
                "{} — A flock descends! 3 strikes incoming!",
                action.name()
            )
        }
    };
    let proj_count_after = battle.projectiles.len() + battle.arcing_projectiles.len();
    if proj_count_after > proj_count_before {
        battle.audio_events.push(AudioEvent::ProjectileLaunch);
    }
    result
}

pub fn apply_player_radical_ability(
    battle: &mut TacticalBattle,
    attacker_idx: usize,
    target_idx: usize,
    ability: PlayerRadicalAbility,
) -> String {
    let target_alive = target_idx < battle.units.len() && battle.units[target_idx].alive;

    match ability {
        PlayerRadicalAbility::FireStrike => {
            let bonus = deal_damage(battle, target_idx, 2);
            if target_alive {
                battle.units[target_idx]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 2));
            }
            format!(
                "{} — +{} fire damage, target burning!",
                ability.name(),
                bonus
            )
        }
        PlayerRadicalAbility::TidalSurge => {
            if target_alive {
                battle.units[target_idx]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Slow, 2));
            }
            battle.focus = (battle.focus + 2).min(battle.max_focus);
            format!("{} — Target slowed, +2 Focus!", ability.name())
        }
        PlayerRadicalAbility::PowerStrike => {
            let bonus = battle.units[attacker_idx].damage / 2;
            let actual = deal_damage(battle, target_idx, bonus.max(1));
            format!("{} — Powerful blow! +{} damage!", ability.name(), actual)
        }
        PlayerRadicalAbility::Insight => {
            crate::combat::ai::calculate_all_intents(battle);
            format!("{} — All enemy intents revealed!", ability.name())
        }
        PlayerRadicalAbility::Devour => {
            let base = battle.units[attacker_idx].damage;
            let heal = (base / 2).max(1);
            battle.units[attacker_idx].hp =
                (battle.units[attacker_idx].hp + heal).min(battle.units[attacker_idx].max_hp);
            format!("{} — Drained {} HP from the enemy!", ability.name(), heal)
        }
        PlayerRadicalAbility::TrueStrike => {
            if target_alive {
                let old_armor = battle.units[target_idx].radical_armor;
                let old_def = battle.units[target_idx].defending;
                battle.units[target_idx].radical_armor = 0;
                battle.units[target_idx].defending = false;
                let actual = deal_damage(battle, target_idx, 2);
                battle.units[target_idx].radical_armor = old_armor;
                battle.units[target_idx].defending = old_def;
                format!(
                    "{} — Pierced all defenses! +{} damage!",
                    ability.name(),
                    actual
                )
            } else {
                format!("{} — True strike!", ability.name())
            }
        }
        PlayerRadicalAbility::SwiftHands => {
            battle.player_acted = false;
            format!("{} — Free action! Attack again!", ability.name())
        }
        PlayerRadicalAbility::Entangle => {
            if target_alive {
                battle.units[target_idx]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Slow, 3));
            }
            battle.units[attacker_idx].radical_armor += 1;
            format!("{} — Target entangled, +1 armor!", ability.name())
        }
        PlayerRadicalAbility::Reap => {
            if target_alive {
                let ratio =
                    battle.units[target_idx].hp as f64 / battle.units[target_idx].max_hp as f64;
                if ratio < 0.4 {
                    let actual = deal_damage(battle, target_idx, 3);
                    format!("{} — Reaped the weak! +{} damage!", ability.name(), actual)
                } else {
                    format!("{} — Target too healthy to reap.", ability.name())
                }
            } else {
                format!("{} — Nothing to reap.", ability.name())
            }
        }
        PlayerRadicalAbility::SolarFlare => {
            battle.units[attacker_idx]
                .statuses
                .retain(|s| !s.is_negative());
            let actual = deal_damage(battle, target_idx, 2);
            format!("{} — Debuffs cleared, +{} damage!", ability.name(), actual)
        }
        PlayerRadicalAbility::MoonVenom => {
            if target_alive {
                battle.units[target_idx]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Poison { damage: 2 }, 3));
            }
            format!("{} — Target poisoned!", ability.name())
        }
        PlayerRadicalAbility::Resilience => {
            let unit = &mut battle.units[attacker_idx];
            unit.max_hp += 2;
            unit.hp += 2;
            format!("{} — Gained 2 temporary HP!", ability.name())
        }
        PlayerRadicalAbility::Guardian => {
            battle.units[attacker_idx].radical_counter = true;
            battle.units[attacker_idx].thorn_armor_turns = 2;
            format!("{} — Counter stance! Thorn armor active!", ability.name())
        }
        PlayerRadicalAbility::GrowingStrike => {
            if target_alive {
                battle.units[target_idx].marked_extra_damage += 2;
            }
            format!("{} — Target marked for +2 damage!", ability.name())
        }
        PlayerRadicalAbility::Harvest => {
            battle.combo_streak += 2;
            format!(
                "{} — Combo extended by 2! ({}x)",
                ability.name(),
                battle.combo_streak
            )
        }
        PlayerRadicalAbility::Gamble => {
            let roll = (battle.turn_number as u64)
                .wrapping_mul(2654435761)
                .wrapping_add(target_idx as u64 * 7)
                % 100;
            if roll < 50 {
                let base = battle.units[attacker_idx].damage;
                let actual = deal_damage(battle, target_idx, base * 2);
                format!("{} — JACKPOT! Triple damage! +{}", ability.name(), actual)
            } else {
                format!("{} — Bad luck! Attack whiffed!", ability.name())
            }
        }
        PlayerRadicalAbility::Shatter => {
            if target_alive {
                battle.units[target_idx].radical_armor = 0;
                battle.units[target_idx].fortify_stacks = 0;
            }
            let actual = deal_damage(battle, target_idx, 1);
            format!("{} — Armor shattered! +{} damage!", ability.name(), actual)
        }
        PlayerRadicalAbility::Earthquake => {
            if target_alive {
                battle.units[target_idx].stunned = true;
                let px = battle.units[attacker_idx].x;
                let py = battle.units[attacker_idx].y;
                let tx = battle.units[target_idx].x;
                let ty = battle.units[target_idx].y;
                let dx = (tx - px).signum();
                let dy = (ty - py).signum();
                for _ in 0..2 {
                    let nx = battle.units[target_idx].x + dx;
                    let ny = battle.units[target_idx].y + dy;
                    if nx >= 0
                        && ny >= 0
                        && nx < battle.arena.width as i32
                        && ny < battle.arena.height as i32
                        && battle
                            .arena
                            .tile(nx, ny)
                            .map(|t| t.is_walkable())
                            .unwrap_or(false)
                        && battle.unit_at(nx, ny).is_none()
                    {
                        battle.units[target_idx].x = nx;
                        battle.units[target_idx].y = ny;
                    }
                }
            }
            format!(
                "{} — Ground shakes! Target pushed and stunned!",
                ability.name()
            )
        }
        PlayerRadicalAbility::DoubleStrike => {
            let base = battle.units[attacker_idx].damage;
            let actual = deal_damage(battle, target_idx, base);
            format!("{} — Double strike! +{} damage!", ability.name(), actual)
        }
        PlayerRadicalAbility::Execution => {
            if target_alive {
                let ratio =
                    battle.units[target_idx].hp as f64 / battle.units[target_idx].max_hp as f64;
                if ratio <= 0.25 {
                    battle.units[target_idx].hp = 0;
                    battle.units[target_idx].alive = false;
                    format!("{} — EXECUTED!", ability.name())
                } else {
                    format!("{} — Target too healthy (need <=25% HP).", ability.name())
                }
            } else {
                format!("{} — Nothing to execute.", ability.name())
            }
        }
        PlayerRadicalAbility::DeepCut => {
            if target_alive {
                battle.units[target_idx]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Bleed { damage: 2 }, 2));
                battle.units[target_idx].max_hp = (battle.units[target_idx].max_hp - 1).max(1);
            }
            format!("{} — Deep wound! Bleeding, -1 max HP!", ability.name())
        }
        PlayerRadicalAbility::Intimidate => {
            if target_alive {
                battle.units[target_idx]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Confused, 2));
            }
            format!("{} — Target confused for 2 turns!", ability.name())
        }
        PlayerRadicalAbility::Lunge => {
            let actual = deal_damage(battle, target_idx, 2);
            battle.player_moved = false;
            format!(
                "{} — Lunge! +{} damage, free movement!",
                ability.name(),
                actual
            )
        }
        PlayerRadicalAbility::Ensnare => {
            if target_alive {
                battle.units[target_idx]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Slow, 3));
                battle.units[target_idx]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Bleed { damage: 1 }, 2));
            }
            format!("{} — Target ensnared! Slow + Bleed!", ability.name())
        }
        PlayerRadicalAbility::Fortify => {
            battle.units[attacker_idx].radical_armor += 3;
            format!("{} — Fortified! +3 armor!", ability.name())
        }
        PlayerRadicalAbility::Charge => {
            if target_alive {
                let px = battle.units[attacker_idx].x;
                let py = battle.units[attacker_idx].y;
                let tx = battle.units[target_idx].x;
                let ty = battle.units[target_idx].y;
                let dx = (tx - px).signum();
                let dy = (ty - py).signum();
                for _ in 0..3 {
                    let nx = battle.units[target_idx].x + dx;
                    let ny = battle.units[target_idx].y + dy;
                    if nx >= 0
                        && ny >= 0
                        && nx < battle.arena.width as i32
                        && ny < battle.arena.height as i32
                        && battle
                            .arena
                            .tile(nx, ny)
                            .map(|t| t.is_walkable())
                            .unwrap_or(false)
                        && battle.unit_at(nx, ny).is_none()
                    {
                        battle.units[target_idx].x = nx;
                        battle.units[target_idx].y = ny;
                    }
                }
            }
            format!("{} — Target knocked back!", ability.name())
        }
        PlayerRadicalAbility::Windstep => {
            battle.units[attacker_idx].stored_movement += 2;
            format!("{} — +2 movement stored!", ability.name())
        }
        PlayerRadicalAbility::Downpour => {
            let tx = battle.units[target_idx].x;
            let ty = battle.units[target_idx].y;
            let mut splashed = 0;
            let splash_targets: Vec<usize> = (1..battle.units.len())
                .filter(|&i| {
                    i != target_idx
                        && battle.units[i].alive
                        && battle.units[i].is_enemy()
                        && (battle.units[i].x - tx).abs() + (battle.units[i].y - ty).abs() <= 1
                })
                .collect();
            for si in splash_targets {
                deal_damage(battle, si, 1);
                splashed += 1;
            }
            format!(
                "{} — Splash! Hit {} adjacent enemies!",
                ability.name(),
                splashed
            )
        }
        PlayerRadicalAbility::Concuss => {
            if target_alive {
                battle.units[target_idx].stunned = true;
            }
            format!("{} — Target stunned!", ability.name())
        }
        PlayerRadicalAbility::Infest => {
            if target_alive {
                battle.units[target_idx]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Poison { damage: 1 }, 3));
                battle.units[target_idx]
                    .statuses
                    .push(StatusInstance::new(StatusKind::Slow, 1));
            }
            format!("{} — Target infested! Poison + Slow!", ability.name())
        }
        PlayerRadicalAbility::Plunder => {
            if !target_alive {
                format!("{} — Plundered! Bonus gold!", ability.name())
            } else {
                format!("{} — Kill to claim bonus gold.", ability.name())
            }
        }
        PlayerRadicalAbility::Bulwark => {
            battle.units[attacker_idx].radical_armor += 2;
            battle.units[attacker_idx].fortify_stacks += 1;
            format!("{} — +2 armor, +1 fortify!", ability.name())
        }
        PlayerRadicalAbility::Frenzy => {
            let actual = deal_damage(battle, target_idx, 1);
            battle.units[attacker_idx].hp =
                (battle.units[attacker_idx].hp + 1).min(battle.units[attacker_idx].max_hp);
            format!("{} — Frenzied! +{} damage, heal 1!", ability.name(), actual)
        }
        PlayerRadicalAbility::Snipe => {
            let actual = deal_damage(battle, target_idx, 2);
            format!("{} — Sniped! +{} damage!", ability.name(), actual)
        }
        PlayerRadicalAbility::Nourish => {
            battle.units[attacker_idx].hp =
                (battle.units[attacker_idx].hp + 3).min(battle.units[attacker_idx].max_hp);
            format!("{} — Nourished! +3 HP!", ability.name())
        }
        PlayerRadicalAbility::Evade => {
            battle.units[attacker_idx].radical_dodge = true;
            format!("{} — Will dodge next attack!", ability.name())
        }
        PlayerRadicalAbility::Riposte => {
            battle.units[attacker_idx].radical_counter = true;
            battle.units[attacker_idx].thorn_armor_turns = 2;
            format!(
                "{} — Riposte stance! Counter + thorn armor!",
                ability.name()
            )
        }
        PlayerRadicalAbility::HitAndRun => {
            battle.player_moved = false;
            format!("{} — Free movement!", ability.name())
        }
        PlayerRadicalAbility::Bulldoze => {
            if target_alive {
                let px = battle.units[attacker_idx].x;
                let py = battle.units[attacker_idx].y;
                let tx = battle.units[target_idx].x;
                let ty = battle.units[target_idx].y;
                let dx = (tx - px).signum();
                let dy = (ty - py).signum();
                let mut tiles_pushed = 0;
                for _ in 0..2 {
                    let nx = battle.units[target_idx].x + dx;
                    let ny = battle.units[target_idx].y + dy;
                    if nx >= 0
                        && ny >= 0
                        && nx < battle.arena.width as i32
                        && ny < battle.arena.height as i32
                        && battle
                            .arena
                            .tile(nx, ny)
                            .map(|t| t.is_walkable())
                            .unwrap_or(false)
                        && battle.unit_at(nx, ny).is_none()
                    {
                        battle.units[target_idx].x = nx;
                        battle.units[target_idx].y = ny;
                        tiles_pushed += 1;
                    }
                }
                if tiles_pushed > 0 {
                    let actual = deal_damage(battle, target_idx, tiles_pushed);
                    format!(
                        "{} — Bulldozed {} tiles! +{} damage!",
                        ability.name(),
                        tiles_pushed,
                        actual
                    )
                } else {
                    format!("{} — Target against wall!", ability.name())
                }
            } else {
                format!("{} — Nothing to push.", ability.name())
            }
        }
        PlayerRadicalAbility::Inspire => {
            battle.units[attacker_idx].damage += 1;
            format!("{} — Inspired! +1 damage!", ability.name())
        }
        PlayerRadicalAbility::Cleave => {
            let px = battle.units[attacker_idx].x;
            let py = battle.units[attacker_idx].y;
            let adj: Vec<usize> = (1..battle.units.len())
                .filter(|&i| {
                    i != target_idx
                        && battle.units[i].alive
                        && battle.units[i].is_enemy()
                        && (battle.units[i].x - px).abs() + (battle.units[i].y - py).abs() <= 1
                })
                .collect();
            let base = battle.units[attacker_idx].damage;
            let mut hit_count = 0;
            for ai in adj {
                deal_damage(battle, ai, base);
                hit_count += 1;
            }
            format!(
                "{} — Cleaved {} additional enemies!",
                ability.name(),
                hit_count
            )
        }
        PlayerRadicalAbility::PreciseStab => {
            if target_alive {
                let old_armor = battle.units[target_idx].radical_armor;
                let old_def = battle.units[target_idx].defending;
                battle.units[target_idx].radical_armor = 0;
                battle.units[target_idx].defending = false;
                let actual = deal_damage(battle, target_idx, 2);
                battle.units[target_idx].radical_armor = old_armor;
                battle.units[target_idx].defending = old_def;
                format!(
                    "{} — Precise stab! +{} armor-piercing!",
                    ability.name(),
                    actual
                )
            } else {
                format!("{} — Precise stab!", ability.name())
            }
        }
        PlayerRadicalAbility::Sabotage => {
            let tx = battle.units[target_idx].x;
            let ty = battle.units[target_idx].y;
            let fire_tiles = vec![(tx - 1, ty), (tx + 1, ty), (tx, ty - 1), (tx, ty + 1)];
            let terrain_msgs =
                apply_terrain_interactions(battle, TerrainSource::FireSpell, &fire_tiles);
            for tm in &terrain_msgs {
                battle.log_message(tm);
            }
            format!("{} — Fire terrain placed!", ability.name())
        }
        PlayerRadicalAbility::Purify => {
            battle.focus = (battle.focus + 3).min(battle.max_focus);
            battle.units[attacker_idx]
                .statuses
                .retain(|s| !s.is_negative());
            format!("{} — Purified! +3 Focus, debuffs cleared!", ability.name())
        }
    }
}
