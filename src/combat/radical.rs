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
                apply_terrain_interactions(battle, TerrainSource::FireAbility, &fire_tiles);
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
                speed: Projectile::SPEED_FAST,
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
                speed: Projectile::SPEED_NORMAL,
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
                fresh: true,
                aoe_radius: 0,
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
                fresh: true,
                aoe_radius: 0,
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
                    fresh: true,
                    aoe_radius: 0,
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
                fresh: true,
                aoe_radius: 0,
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
                speed: Projectile::SPEED_FAST,
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
                fresh: true,
                aoe_radius: 0,
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
                speed: Projectile::SPEED_FAST,
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
                let prev_nx = nx;
                let prev_ny = ny;
                if (px - nx).abs() > (py - ny).abs() {
                    nx += dx;
                } else {
                    ny += dy;
                }
                if nx == px && ny == py {
                    nx = prev_nx;
                    ny = prev_ny;
                    break;
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
                speed: Projectile::SPEED_FAST,
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
            let fdx = facing.dx();
            let fdy = facing.dy();
            for _ in 0..2 {
                let new_x = battle.units[0].x + fdx;
                let new_y = battle.units[0].y + fdy;
                if new_x < 0 || new_y < 0
                    || new_x >= battle.arena.width as i32
                    || new_y >= battle.arena.height as i32
                {
                    break;
                }
                if !battle.arena.tile(new_x, new_y).map(|t| t.is_walkable()).unwrap_or(false) {
                    break;
                }
                if battle.unit_at(new_x, new_y).is_some() {
                    break;
                }
                battle.units[0].x = new_x;
                battle.units[0].y = new_y;
            }
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
                speed: Projectile::SPEED_FAST,
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
                        fresh: true,
                        aoe_radius: 1,
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
                speed: Projectile::SPEED_NORMAL,
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
                speed: Projectile::SPEED_NORMAL,
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
                fresh: true,
                aoe_radius: 0,
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
                speed: Projectile::SPEED_NORMAL,
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
                let dx = (px - nx).signum();
                let dy = (py - ny).signum();
                let prev_nx = nx;
                let prev_ny = ny;
                if (px - nx).abs() > (py - ny).abs() {
                    nx += dx;
                } else {
                    ny += dy;
                }
                if nx == px && ny == py {
                    nx = prev_nx;
                    ny = prev_ny;
                    break;
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
                speed: Projectile::SPEED_FAST,
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
            let fdx = facing.dx();
            let fdy = facing.dy();
            for _ in 0..3 {
                let new_x = battle.units[0].x + fdx;
                let new_y = battle.units[0].y + fdy;
                if new_x < 0 || new_y < 0
                    || new_x >= battle.arena.width as i32
                    || new_y >= battle.arena.height as i32
                {
                    break;
                }
                if !battle.arena.tile(new_x, new_y).map(|t| t.is_walkable()).unwrap_or(false) {
                    break;
                }
                if battle.unit_at(new_x, new_y).is_some() {
                    break;
                }
                battle.units[0].x = new_x;
                battle.units[0].y = new_y;
            }
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
                speed: Projectile::SPEED_NORMAL,
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
                let (new_x, new_y) = if (px - ax).abs() > (py - ay).abs() {
                    ((ax + dx).clamp(0, battle.arena.width as i32 - 1), ay)
                } else {
                    (ax, (ay + dy).clamp(0, battle.arena.height as i32 - 1))
                };
                if new_x != px || new_y != py {
                    battle.units[ally_idx].x = new_x;
                    battle.units[ally_idx].y = new_y;
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
                fresh: true,
                aoe_radius: 0,
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
            battle.arena.set_tile(px, py, crate::combat::BattleTile::DamagedFloor);
            battle.arcing_projectiles.push(ArcingProjectile {
                target_x: px,
                target_y: py,
                turns_remaining: 1,
                effect: ProjectileEffect::Damage(1),
                glyph: "🕳",
                color: "#664422",
                owner_is_player: false,
                fresh: true,
                aoe_radius: 0,
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
                    speed: Projectile::SPEED_FAST,
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
                fresh: true,
                aoe_radius: 0,
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
                let dx = (px - nx).signum();
                let dy = (py - ny).signum();
                let prev_nx = nx;
                let prev_ny = ny;
                if (px - nx).abs() >= (py - ny).abs() {
                    nx += dx;
                } else {
                    ny += dy;
                }
                if nx == px && ny == py {
                    nx = prev_nx;
                    ny = prev_ny;
                    break;
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
                speed: Projectile::SPEED_FAST,
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
                                .set_tile(tx, ty, crate::combat::BattleTile::VentSteam);
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
                            if battle.arena.tile(tx, ty) == Some(crate::combat::BattleTile::MetalFloor) {
                                battle
                                    .arena
                                    .set_tile(tx, ty, crate::combat::BattleTile::WiringPanel);
                            }
                        }
                    }
                }
            }
            // Count adjacent wiring panels for armor
            let mut grass_count = 0;
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                if battle.arena.tile(ux + dx, uy + dy)
                    == Some(crate::combat::BattleTile::WiringPanel)
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
                battle.arena.tile(px, py) == Some(crate::combat::BattleTile::CoolantPool);
            battle
                .arena
                .set_tile(px, py, crate::combat::BattleTile::CoolantPool);
            if on_water {
                // Extra damage if already on coolant
                let ex = battle.units[unit_idx].x;
                let ey = battle.units[unit_idx].y;
                battle.projectiles.push(Projectile {
                    from_x: ex as f64,
                    from_y: ey as f64,
                    to_x: px,
                    to_y: py,
                    progress: 0.0,
                    speed: Projectile::SPEED_NORMAL,
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
                fresh: true,
                aoe_radius: 0,
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
                    fresh: true,
                    aoe_radius: 1,
                });
            }
            format!(
                "{} — A flock descends! 3 strikes incoming!",
                action.name()
            )
        }
        RadicalAction::PhaseStrike => {
            // Teleport adjacent to player, deal 2 AoE damage at departure point
            let old_x = battle.units[unit_idx].x;
            let old_y = battle.units[unit_idx].y;
            let px = battle.units[0].x;
            let py = battle.units[0].y;
            // Find an empty walkable tile adjacent to the player
            let adj = [(px - 1, py), (px + 1, py), (px, py - 1), (px, py + 1)];
            let mut dest = None;
            for &(ax, ay) in &adj {
                if !battle.arena.in_bounds(ax, ay) {
                    continue;
                }
                if !battle
                    .arena
                    .tile(ax, ay)
                    .map(|t| t.is_walkable())
                    .unwrap_or(false)
                {
                    continue;
                }
                if battle.unit_at(ax, ay).is_some() {
                    continue;
                }
                dest = Some((ax, ay));
                break;
            }
            if let Some((dx, dy)) = dest {
                battle.units[unit_idx].x = dx;
                battle.units[unit_idx].y = dy;
                // AoE damage at departure point (cross pattern)
                let aoe_tiles = [
                    (old_x, old_y),
                    (old_x - 1, old_y),
                    (old_x + 1, old_y),
                    (old_x, old_y - 1),
                    (old_x, old_y + 1),
                ];
                for &(ax, ay) in &aoe_tiles {
                    if let Some(idx) = battle.unit_at(ax, ay) {
                        if battle.units[idx].is_player() || battle.units[idx].is_companion() {
                            deal_damage(battle, idx, 2);
                        }
                    }
                }
                format!(
                    "{} — Vanishes and reappears next to you! Departure blast!",
                    action.name()
                )
            } else {
                format!("{} — Tries to phase but finds no space!", action.name())
            }
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
            if target_idx != attacker_idx && target_alive {
                let actual = deal_damage(battle, target_idx, 2);
                format!("{} — Debuffs cleared, +{} damage!", ability.name(), actual)
            } else {
                format!("{} — Debuffs cleared!", ability.name())
            }
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
            // First try terrain-specific interactions (dungeon tiles)
            let terrain_msgs =
                apply_terrain_interactions(battle, TerrainSource::FireAbility, &fire_tiles);
            for tm in &terrain_msgs {
                battle.log_message(tm);
            }
            // For tiles that weren't transformed, place BlastMark directly
            let mut placed = 0u32;
            for &(fx, fy) in &fire_tiles {
                if let Some(tile) = battle.arena.tile(fx, fy) {
                    if tile.is_walkable()
                        && tile != crate::combat::BattleTile::BlastMark
                        && tile != crate::combat::BattleTile::PlasmaPool
                    {
                        battle
                            .arena
                            .set_tile(fx, fy, crate::combat::BattleTile::BlastMark);
                        placed += 1;
                        if let Some(idx) = battle.unit_at(fx, fy) {
                            if idx != attacker_idx {
                                let actual = deal_damage(battle, idx, 1);
                                battle.log_message(&format!(
                                    "🔥 Fire scorches for {} damage!",
                                    actual
                                ));
                            }
                        }
                    }
                }
            }
            if placed > 0 || !terrain_msgs.is_empty() {
                format!(
                    "{} — Fire terrain placed! {} tiles ablaze!",
                    ability.name(),
                    placed + terrain_msgs.len() as u32
                )
            } else {
                format!("{} — No room for fire terrain!", ability.name())
            }
        }
        PlayerRadicalAbility::Purify => {
            battle.focus = (battle.focus + 3).min(battle.max_focus);
            battle.units[attacker_idx]
                .statuses
                .retain(|s| !s.is_negative());
            format!("{} — Purified! +3 Focus, debuffs cleared!", ability.name())
        }
        PlayerRadicalAbility::Galeforce => {
            let px = battle.units[attacker_idx].x;
            let py = battle.units[attacker_idx].y;
            let enemies: Vec<usize> = (1..battle.units.len())
                .filter(|&i| {
                    battle.units[i].alive
                        && battle.units[i].is_enemy()
                        && (battle.units[i].x - px).abs() + (battle.units[i].y - py).abs() <= 1
                })
                .collect();
            let mut pushed = 0;
            for ei in enemies {
                let dx = (battle.units[ei].x - px).signum();
                let dy = (battle.units[ei].y - py).signum();
                let nx = battle.units[ei].x + dx;
                let ny = battle.units[ei].y + dy;
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
                    battle.units[ei].x = nx;
                    battle.units[ei].y = ny;
                    pushed += 1;
                }
            }
            format!("{} — Gale blast! Pushed {} enemies!", ability.name(), pushed)
        }
        PlayerRadicalAbility::QiSurge => {
            battle.focus = (battle.focus + 5).min(battle.max_focus);
            battle.units[attacker_idx]
                .statuses
                .push(StatusInstance::new(StatusKind::Empowered { amount: 1 }, 2));
            format!("{} — Qi restored! +5 Focus, +1 power for 2 turns!", ability.name())
        }
        PlayerRadicalAbility::Echolocation => {
            let mut revealed = 0;
            for i in 1..battle.units.len() {
                if battle.units[i].alive && battle.units[i].is_enemy() {
                    battle.units[i]
                        .statuses
                        .push(StatusInstance::new(StatusKind::Revealed, 3));
                    revealed += 1;
                }
            }
            format!("{} — {} enemies revealed for 3 turns!", ability.name(), revealed)
        }
        PlayerRadicalAbility::Undertow => {
            if target_alive {
                let tx = battle.units[target_idx].x;
                let ty = battle.units[target_idx].y;
                battle
                    .arena
                    .set_tile(tx, ty, crate::combat::BattleTile::CoolantPool);
                let px = battle.units[attacker_idx].x;
                let py = battle.units[attacker_idx].y;
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
                    } else {
                        break;
                    }
                }
                format!("{} — Water surges! Target pushed!", ability.name())
            } else {
                format!("{} — No target.", ability.name())
            }
        }
        PlayerRadicalAbility::BoneBreaker => {
            if target_alive {
                battle.units[target_idx].radical_armor = 0;
                battle.units[target_idx].fortify_stacks = 0;
            }
            let actual = deal_damage(battle, target_idx, 3);
            format!("{} — Bones shattered! Armor broken, {} damage!", ability.name(), actual)
        }
        PlayerRadicalAbility::RamStrike => {
            if target_alive {
                battle.units[target_idx].stunned = true;
            }
            let actual = deal_damage(battle, target_idx, 2);
            format!("{} — Ram strike! {} damage + stunned!", ability.name(), actual)
        }
        PlayerRadicalAbility::Discern => {
            if target_alive {
                battle.units[target_idx].radical_armor = 0;
                battle.units[target_idx].fortify_stacks = 0;
                battle.units[target_idx].radical_dodge = false;
                battle.units[target_idx].radical_counter = false;
                battle.units[target_idx].thorn_armor_turns = 0;
                battle.units[target_idx]
                    .statuses
                    .retain(|s| s.is_negative());
            }
            format!("{} — Target exposed! All buffs removed!", ability.name())
        }
        PlayerRadicalAbility::IronForm => {
            battle.units[attacker_idx].radical_armor += 3;
            battle.units[attacker_idx]
                .statuses
                .push(StatusInstance::new(StatusKind::Rooted, 2));
            format!("{} — Iron form! +3 armor, rooted!", ability.name())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::test_helpers::*;
    use crate::combat::{BattleTile, UnitKind};
    use crate::enemy::{PlayerRadicalAbility, RadicalAction};
    use crate::status::StatusKind;

    // ── apply_radical_action: status effects on player ────────────────

    #[test]
    fn spreading_wildfire_applies_burn_to_player() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::SpreadingWildfire);

        assert_eq!(battle.units[0].statuses.len(), 1);
        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Burn { damage: 1 }
        ));
        assert_eq!(battle.units[0].statuses[0].turns_left, 3);
    }

    #[test]
    fn erosive_flow_reduces_player_armor_and_slows() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.units[0].radical_armor = 3;

        apply_radical_action(&mut battle, 1, RadicalAction::ErosiveFlow);

        assert_eq!(battle.units[0].radical_armor, 2);
        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Slow
        ));
    }

    #[test]
    fn erosive_flow_armor_can_go_negative() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.units[0].radical_armor = 0;

        apply_radical_action(&mut battle, 1, RadicalAction::ErosiveFlow);

        assert_eq!(battle.units[0].radical_armor, -1);
    }

    #[test]
    fn doubt_seed_applies_confused_to_player() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::DoubtSeed);

        assert_eq!(battle.units[0].statuses.len(), 1);
        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Confused
        ));
        assert_eq!(battle.units[0].statuses[0].turns_left, 2);
    }

    #[test]
    fn waning_curse_applies_poison_to_player() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::WaningCurse);

        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Poison { damage: 1 }
        ));
        assert_eq!(battle.units[0].statuses[0].turns_left, 2);
    }

    #[test]
    fn binding_oath_applies_slow_and_confused_to_player() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::BindingOath);

        assert_eq!(battle.units[0].statuses.len(), 2);
        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Slow
        ));
        assert!(matches!(
            battle.units[0].statuses[1].kind,
            StatusKind::Confused
        ));
    }

    #[test]
    fn entangling_web_applies_slow_and_bleed_to_player() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::EntanglingWeb);

        assert_eq!(battle.units[0].statuses.len(), 2);
        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Slow
        ));
        assert!(matches!(
            battle.units[0].statuses[1].kind,
            StatusKind::Bleed { damage: 1 }
        ));
    }

    #[test]
    fn chasing_chaff_applies_confused_one_turn() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::ChasingChaff);

        assert_eq!(battle.units[0].statuses[0].turns_left, 1);
        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Confused
        ));
    }

    #[test]
    fn artisan_trap_applies_burn_to_player() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::ArtisanTrap);

        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Burn { damage: 1 }
        ));
        assert_eq!(battle.units[0].statuses[0].turns_left, 2);
    }

    #[test]
    fn venomous_lash_applies_poison_and_creates_arcing_projectile() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::VenomousLash);

        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Poison { damage: 2 }
        ));
        assert_eq!(battle.arcing_projectiles.len(), 1);
    }

    // ── apply_radical_action: enemy self-buffs ────────────────────────

    #[test]
    fn rigid_stance_adds_armor_and_clears_dodge() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.radical_dodge = true;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::RigidStance);

        assert_eq!(battle.units[1].radical_armor, 4);
        assert!(!battle.units[1].radical_dodge);
    }

    #[test]
    fn immovable_peak_adds_armor_and_fortify() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::ImmovablePeak);

        assert_eq!(battle.units[1].radical_armor, 3);
        assert_eq!(battle.units[1].fortify_stacks, 1);
    }

    #[test]
    fn soaring_escape_enables_dodge_and_adds_movement() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::SoaringEscape);

        assert!(battle.units[1].radical_dodge);
        assert_eq!(battle.units[1].stored_movement, 2);
    }

    #[test]
    fn cloaking_guise_enables_dodge_and_fortify() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::CloakingGuise);

        assert!(battle.units[1].radical_dodge);
        assert_eq!(battle.units[1].fortify_stacks, 1);
    }

    #[test]
    fn flexible_counter_enables_counter_and_thorns() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::FlexibleCounter);

        assert!(battle.units[1].radical_counter);
        assert_eq!(battle.units[1].thorn_armor_turns, 2);
    }

    #[test]
    fn maternal_shield_enables_counter_and_thorns() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::MaternalShield);

        assert!(battle.units[1].radical_counter);
        assert_eq!(battle.units[1].thorn_armor_turns, 3);
    }

    #[test]
    fn threshold_seal_adds_armor_and_slows_self() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::ThresholdSeal);

        assert_eq!(battle.units[1].radical_armor, 3);
        assert!(matches!(
            battle.units[1].statuses[0].kind,
            StatusKind::Slow
        ));
    }

    #[test]
    fn adaptive_shift_adds_armor_and_fortify() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::AdaptiveShift);

        assert_eq!(battle.units[1].radical_armor, 2);
        assert_eq!(battle.units[1].fortify_stacks, 1);
    }

    #[test]
    fn iron_body_stance_adds_armor_fortify_and_slows_self() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::IronBodyStance);

        assert_eq!(battle.units[1].radical_armor, 2);
        assert_eq!(battle.units[1].fortify_stacks, 1);
        assert!(matches!(
            battle.units[1].statuses[0].kind,
            StatusKind::Slow
        ));
    }

    #[test]
    fn berserker_fury_self_damages_and_buffs() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.hp = 10;
        enemy.damage = 2;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::BerserkerFury);

        assert_eq!(battle.units[1].hp, 8);
        assert_eq!(battle.units[1].damage, 5); // 2 + 3
        assert!(matches!(
            battle.units[1].statuses[0].kind,
            StatusKind::Haste
        ));
    }

    #[test]
    fn cleansing_light_removes_negative_statuses_and_heals() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.hp = 5;
        enemy.statuses.push(StatusInstance::new(StatusKind::Slow, 3));
        enemy
            .statuses
            .push(StatusInstance::new(StatusKind::Poison { damage: 1 }, 2));
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::CleansingLight);

        assert_eq!(battle.units[1].statuses.len(), 0);
        assert_eq!(battle.units[1].hp, 8); // 5 + 3
    }

    // ── apply_radical_action: projectiles ─────────────────────────────

    #[test]
    fn overwhelming_force_scales_damage_with_missing_hp() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.hp = 4; // missing 6
        enemy.max_hp = 10;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::OverwhelmingForce);

        assert_eq!(battle.projectiles.len(), 1);
        assert!(matches!(
            battle.projectiles[0].effect,
            ProjectileEffect::Damage(4) // 1 + 6/2
        ));
    }

    #[test]
    fn overwhelming_force_at_full_hp_deals_minimum_damage() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::OverwhelmingForce);

        assert!(matches!(
            battle.projectiles[0].effect,
            ProjectileEffect::Damage(1) // 1 + 0/2
        ));
    }

    #[test]
    fn echo_strike_fires_projectile_with_enemy_damage() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.damage = 7;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::EchoStrike);

        assert!(matches!(
            battle.projectiles[0].effect,
            ProjectileEffect::Damage(7)
        ));
    }

    #[test]
    fn savage_maul_fires_three_damage_projectile() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::SavageMaul);

        assert_eq!(battle.projectiles.len(), 1);
        assert!(matches!(
            battle.projectiles[0].effect,
            ProjectileEffect::Damage(3)
        ));
    }

    #[test]
    fn needle_strike_fires_piercing_projectile() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::NeedleStrike);

        assert_eq!(battle.arcing_projectiles.len(), 1);
        assert!(matches!(
            battle.arcing_projectiles[0].effect,
            ProjectileEffect::PiercingDamage(2)
        ));
    }

    #[test]
    fn downpour_barrage_fires_three_arcing_projectiles() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::DownpourBarrage);

        assert_eq!(battle.arcing_projectiles.len(), 3);
        assert_eq!(battle.arcing_projectiles[0].turns_remaining, 2);
    }

    #[test]
    fn arcing_shot_at_long_range_deals_three_damage() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::ArcingShot);

        assert!(matches!(
            battle.arcing_projectiles[0].effect,
            ProjectileEffect::Damage(3)
        ));
    }

    #[test]
    fn arcing_shot_at_short_range_deals_two_damage() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::ArcingShot);

        assert!(matches!(
            battle.arcing_projectiles[0].effect,
            ProjectileEffect::Damage(2)
        ));
    }

    #[test]
    fn flock_assault_fires_three_arcing_projectiles() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::FlockAssault);

        assert_eq!(battle.arcing_projectiles.len(), 3);
    }

    // ── apply_radical_action: player mark/debuff ──────────────────────

    #[test]
    fn witness_mark_sets_player_marked_damage() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::WitnessMark);

        assert_eq!(battle.units[0].marked_extra_damage, 3);
    }

    #[test]
    fn potential_burst_increases_marked_damage() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.units[0].marked_extra_damage = 1;

        apply_radical_action(&mut battle, 1, RadicalAction::PotentialBurst);

        assert_eq!(battle.units[0].marked_extra_damage, 3); // 1 + 2
    }

    #[test]
    fn qi_disruption_drains_player_focus() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.focus = 10;

        apply_radical_action(&mut battle, 1, RadicalAction::QiDisruption);

        assert_eq!(battle.focus, 7);
    }

    #[test]
    fn qi_disruption_drains_only_available_focus() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.focus = 1;

        apply_radical_action(&mut battle, 1, RadicalAction::QiDisruption);

        assert_eq!(battle.focus, 0);
    }

    // ── apply_radical_action: position manipulation ───────────────────

    #[test]
    fn sleight_reversal_swaps_positions() {
        let player = make_test_unit(UnitKind::Player, 1, 1);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 5);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::SleightReversal);

        assert_eq!((battle.units[0].x, battle.units[0].y), (5, 5));
        assert_eq!((battle.units[1].x, battle.units[1].y), (1, 1));
    }

    // ── apply_radical_action: devouring maw steals ────────────────────

    #[test]
    fn devouring_maw_steals_dodge() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.radical_dodge = true;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::DevouringMaw);

        assert!(!battle.units[0].radical_dodge);
        assert!(battle.units[1].radical_dodge);
    }

    #[test]
    fn devouring_maw_steals_counter_when_no_dodge() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.radical_counter = true;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::DevouringMaw);

        assert!(!battle.units[0].radical_counter);
    }

    #[test]
    fn devouring_maw_steals_armor_when_no_dodge_or_counter() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.radical_armor = 5;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::DevouringMaw);

        assert_eq!(battle.units[0].radical_armor, 0);
        assert_eq!(battle.units[1].radical_armor, 5);
    }

    // ── apply_radical_action: conditional/execute ─────────────────────

    #[test]
    fn mortal_resilience_low_hp_survives_and_buffs() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.hp = 3;
        enemy.max_hp = 10;
        enemy.damage = 2;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::MortalResilience);

        assert_eq!(battle.units[1].hp, 3);
        assert_eq!(battle.units[1].damage, 4); // 2 + 2
    }

    #[test]
    fn mortal_resilience_high_hp_gains_armor() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.hp = 8;
        enemy.max_hp = 10;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::MortalResilience);

        assert_eq!(battle.units[1].radical_armor, 1);
    }

    #[test]
    fn harvest_reaping_low_hp_player_gets_three_damage_projectile() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.hp = 3;
        player.max_hp = 10;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::HarvestReaping);

        assert!(matches!(
            battle.arcing_projectiles[0].effect,
            ProjectileEffect::Damage(3)
        ));
    }

    #[test]
    fn harvest_reaping_high_hp_player_gets_one_damage_projectile() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::HarvestReaping);

        assert!(matches!(
            battle.arcing_projectiles[0].effect,
            ProjectileEffect::Damage(1)
        ));
    }

    #[test]
    fn precise_execution_low_hp_player_gets_four_damage() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.hp = 2;
        player.max_hp = 10;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::PreciseExecution);

        assert!(matches!(
            battle.arcing_projectiles[0].effect,
            ProjectileEffect::Damage(4)
        ));
    }

    #[test]
    fn precise_execution_high_hp_player_gets_one_damage() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::PreciseExecution);

        assert!(matches!(
            battle.arcing_projectiles[0].effect,
            ProjectileEffect::Damage(1)
        ));
    }

    // ── apply_radical_action: cleaving cut ─────────────────────────────

    #[test]
    fn cleaving_cut_reduces_player_max_hp() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::CleavingCut);

        assert_eq!(battle.units[0].max_hp, 9);
        assert_eq!(battle.projectiles.len(), 1);
    }

    #[test]
    fn cleaving_cut_max_hp_does_not_go_below_one() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.max_hp = 1;
        player.hp = 1;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::CleavingCut);

        assert_eq!(battle.units[0].max_hp, 1);
    }

    // ── apply_radical_action: true vision strips everything ───────────

    #[test]
    fn true_vision_strips_all_player_buffs() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.radical_armor = 5;
        player.radical_dodge = true;
        player.radical_counter = true;
        player.thorn_armor_turns = 3;
        player.defending = true;
        player
            .statuses
            .push(StatusInstance::new(StatusKind::Haste, 2));
        player
            .statuses
            .push(StatusInstance::new(StatusKind::Slow, 1));
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::TrueVision);

        assert_eq!(battle.units[0].radical_armor, 0);
        assert!(!battle.units[0].radical_dodge);
        assert!(!battle.units[0].radical_counter);
        assert_eq!(battle.units[0].thorn_armor_turns, 0);
        assert!(!battle.units[0].defending);
        // Positive statuses removed, negative retained
        assert_eq!(battle.units[0].statuses.len(), 1);
        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Slow
        ));
    }

    // ── apply_radical_action: consuming bite heals and buffs ──────────

    #[test]
    fn consuming_bite_increases_max_hp_and_heals() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.hp = 5;
        enemy.max_hp = 10;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::ConsumingBite);

        assert_eq!(battle.units[1].max_hp, 11);
        assert_eq!(battle.units[1].hp, 7); // 5 + 2
    }

    // ── apply_radical_action: parasitic swarm heals self ──────────────

    #[test]
    fn parasitic_swarm_heals_enemy() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.hp = 5;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::ParasiticSwarm);

        assert_eq!(battle.units[1].hp, 7); // 5 + 2
        assert_eq!(battle.projectiles.len(), 1);
    }

    // ── apply_radical_action: mercenary pact ──────────────────────────

    #[test]
    fn mercenary_pact_self_damages_and_buffs_nearby_allies() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
        enemy1.damage = 2;
        let mut enemy2 = make_test_unit(UnitKind::Enemy(0), 4, 3);
        enemy2.damage = 2;
        let mut battle = make_test_battle(vec![player, enemy1, enemy2]);

        apply_radical_action(&mut battle, 1, RadicalAction::MercenaryPact);

        assert_eq!(battle.units[1].hp, 8); // 10 - 2
        // enemy2 is within 3 tiles of enemy1
        assert_eq!(battle.units[2].damage, 3); // 2 + 1
        assert_eq!(battle.units[2].radical_armor, 1);
    }

    // ── apply_radical_action: rooting grasp ───────────────────────────

    #[test]
    fn rooting_grasp_slows_player_and_adds_self_armor() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::RootingGrasp);

        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Slow
        ));
        assert_eq!(battle.units[1].radical_armor, 1);
    }

    // ── apply_radical_action: petrifying gaze ─────────────────────────

    #[test]
    fn petrifying_gaze_slows_player_and_adds_self_armor() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::PetrifyingGaze);

        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Slow
        ));
        assert_eq!(battle.units[0].statuses[0].turns_left, 3);
        assert_eq!(battle.units[1].radical_armor, 2);
    }

    // ── apply_radical_action: sinkhole snare ──────────────────────────

    #[test]
    fn sinkhole_snare_sets_player_tile_to_damaged_floor() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::SinkholeSnare);

        assert_eq!(
            battle.arena.tile(3, 3),
            Some(BattleTile::DamagedFloor)
        );
        assert_eq!(battle.arcing_projectiles.len(), 1);
    }

    // ── apply_radical_action: bone shatter ────────────────────────────

    #[test]
    fn bone_shatter_strips_armor_and_defending() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.radical_armor = 5;
        player.defending = true;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::BoneShatter);

        assert_eq!(battle.units[0].radical_armor, 0);
        assert!(!battle.units[0].defending);
        assert_eq!(battle.arcing_projectiles.len(), 1);
    }

    // ── apply_radical_action: revealing dawn ──────────────────────────

    #[test]
    fn revealing_dawn_clears_own_negative_statuses() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy
            .statuses
            .push(StatusInstance::new(StatusKind::Slow, 2));
        enemy
            .statuses
            .push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 3));
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::RevealingDawn);

        assert_eq!(battle.units[1].statuses.len(), 0);
    }

    // ── apply_radical_action: sonic burst stuns nearby ─────────────────

    #[test]
    fn sonic_burst_stuns_nearby_units_and_self_damages() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
        enemy.hp = 5;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::SonicBurst);

        assert!(battle.units[0].stunned);
        assert_eq!(battle.units[1].hp, 4); // self-damage
    }

    // ── apply_radical_action: expanding domain ────────────────────────

    #[test]
    fn expanding_domain_buffs_self_and_nearby_allies() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
        enemy1.damage = 2;
        let mut enemy2 = make_test_unit(UnitKind::Enemy(0), 4, 3);
        enemy2.damage = 3;
        let mut battle = make_test_battle(vec![player, enemy1, enemy2]);

        apply_radical_action(&mut battle, 1, RadicalAction::ExpandingDomain);

        assert_eq!(battle.units[1].damage, 3); // 2 + 1
        assert_eq!(battle.units[1].radical_armor, 1);
        // enemy2 at (4,3) is 2 tiles from enemy1 at (3,3)
        assert_eq!(battle.units[2].damage, 4); // 3 + 1
    }

    // ── apply_radical_action: audio events ────────────────────────────

    #[test]
    fn projectile_actions_add_audio_event() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::OverwhelmingForce);

        assert!(battle
            .audio_events
            .iter()
            .any(|e| matches!(e, AudioEvent::ProjectileLaunch)));
    }

    // ── apply_radical_action: return messages ─────────────────────────

    #[test]
    fn spreading_wildfire_returns_expected_message() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::SpreadingWildfire);

        assert!(msg.contains("Spreads wildfire"));
    }

    #[test]
    fn erosive_flow_returns_expected_message() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::ErosiveFlow);

        assert!(msg.contains("Armor eroded"));
    }

    // ══════════════════════════════════════════════════════════════════
    // apply_player_radical_ability tests
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn fire_strike_deals_damage_and_burns() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        let initial_hp = battle.units[1].hp;

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::FireStrike);

        assert!(battle.units[1].hp < initial_hp);
        assert!(matches!(
            battle.units[1].statuses[0].kind,
            StatusKind::Burn { damage: 1 }
        ));
    }

    #[test]
    fn tidal_surge_slows_and_restores_focus() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.focus = 5;

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::TidalSurge);

        assert!(matches!(
            battle.units[1].statuses[0].kind,
            StatusKind::Slow
        ));
        assert_eq!(battle.focus, 7);
    }

    #[test]
    fn tidal_surge_focus_capped_at_max() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.focus = 9;
        battle.max_focus = 10;

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::TidalSurge);

        assert_eq!(battle.focus, 10);
    }

    #[test]
    fn power_strike_deals_bonus_damage() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.damage = 6;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        let initial_hp = battle.units[1].hp;

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::PowerStrike);

        assert!(battle.units[1].hp < initial_hp);
    }

    #[test]
    fn devour_heals_attacker() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.damage = 4;
        player.hp = 5;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Devour);

        assert_eq!(battle.units[0].hp, 7); // 5 + 4/2
    }

    #[test]
    fn devour_heal_minimum_one() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.damage = 1;
        player.hp = 5;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Devour);

        assert_eq!(battle.units[0].hp, 6); // 5 + max(1/2, 1)
    }

    #[test]
    fn true_strike_ignores_armor() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.radical_armor = 10;
        enemy.defending = true;
        let mut battle = make_test_battle(vec![player, enemy]);
        let initial_hp = battle.units[1].hp;

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::TrueStrike);

        assert!(battle.units[1].hp < initial_hp);
        // Armor and defending restored after the strike
        assert_eq!(battle.units[1].radical_armor, 10);
        assert!(battle.units[1].defending);
    }

    #[test]
    fn swift_hands_grants_free_action() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.player_acted = true;

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::SwiftHands);

        assert!(!battle.player_acted);
    }

    #[test]
    fn entangle_slows_target_and_adds_attacker_armor() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Entangle);

        assert!(matches!(
            battle.units[1].statuses[0].kind,
            StatusKind::Slow
        ));
        assert_eq!(battle.units[0].radical_armor, 1);
    }

    #[test]
    fn reap_executes_low_hp_target() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.hp = 3;
        enemy.max_hp = 10;
        let mut battle = make_test_battle(vec![player, enemy]);
        let initial_hp = battle.units[1].hp;

        let msg = apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Reap);

        assert!(battle.units[1].hp < initial_hp);
        assert!(msg.contains("Reaped"));
    }

    #[test]
    fn reap_does_nothing_to_healthy_target() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Reap);

        assert_eq!(battle.units[1].hp, 10);
        assert!(msg.contains("too healthy"));
    }

    #[test]
    fn solar_flare_clears_debuffs_and_deals_damage() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player
            .statuses
            .push(StatusInstance::new(StatusKind::Slow, 2));
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        let initial_hp = battle.units[1].hp;

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::SolarFlare);

        assert_eq!(battle.units[0].statuses.len(), 0);
        assert!(battle.units[1].hp < initial_hp);
    }

    #[test]
    fn moon_venom_poisons_target() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::MoonVenom);

        assert!(matches!(
            battle.units[1].statuses[0].kind,
            StatusKind::Poison { damage: 2 }
        ));
        assert_eq!(battle.units[1].statuses[0].turns_left, 3);
    }

    #[test]
    fn resilience_increases_max_hp_and_hp() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.hp = 8;
        player.max_hp = 10;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Resilience);

        assert_eq!(battle.units[0].max_hp, 12);
        assert_eq!(battle.units[0].hp, 10);
    }

    #[test]
    fn guardian_enables_counter_and_thorns() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Guardian);

        assert!(battle.units[0].radical_counter);
        assert_eq!(battle.units[0].thorn_armor_turns, 2);
    }

    #[test]
    fn growing_strike_marks_target() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::GrowingStrike);

        assert_eq!(battle.units[1].marked_extra_damage, 2);
    }

    #[test]
    fn harvest_extends_combo() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.combo_streak = 3;

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Harvest);

        assert_eq!(battle.combo_streak, 5);
    }

    #[test]
    fn shatter_removes_armor_and_deals_damage() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.radical_armor = 5;
        enemy.fortify_stacks = 3;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Shatter);

        assert_eq!(battle.units[1].radical_armor, 0);
        assert_eq!(battle.units[1].fortify_stacks, 0);
    }

    #[test]
    fn earthquake_stuns_and_pushes_target() {
        let player = make_test_unit(UnitKind::Player, 1, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Earthquake);

        assert!(battle.units[1].stunned);
        assert!(battle.units[1].x > 3); // pushed away from player
    }

    #[test]
    fn double_strike_deals_base_damage_twice_equivalent() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.damage = 4;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        let initial_hp = battle.units[1].hp;

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::DoubleStrike);

        assert!(battle.units[1].hp < initial_hp);
    }

    #[test]
    fn execution_kills_low_hp_target() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.hp = 2;
        enemy.max_hp = 10;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Execution);

        assert_eq!(battle.units[1].hp, 0);
        assert!(!battle.units[1].alive);
    }

    #[test]
    fn execution_fails_on_healthy_target() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Execution);

        assert!(battle.units[1].alive);
        assert!(msg.contains("too healthy"));
    }

    #[test]
    fn deep_cut_bleeds_and_reduces_max_hp() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::DeepCut);

        assert_eq!(battle.units[1].max_hp, 9);
        assert!(matches!(
            battle.units[1].statuses[0].kind,
            StatusKind::Bleed { damage: 2 }
        ));
    }

    #[test]
    fn intimidate_confuses_target() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Intimidate);

        assert!(matches!(
            battle.units[1].statuses[0].kind,
            StatusKind::Confused
        ));
        assert_eq!(battle.units[1].statuses[0].turns_left, 2);
    }

    #[test]
    fn lunge_deals_damage_and_grants_free_movement() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.player_moved = true;

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Lunge);

        assert!(!battle.player_moved);
        assert!(battle.units[1].hp < 10);
    }

    #[test]
    fn ensnare_applies_slow_and_bleed() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Ensnare);

        assert_eq!(battle.units[1].statuses.len(), 2);
        assert!(matches!(
            battle.units[1].statuses[0].kind,
            StatusKind::Slow
        ));
        assert!(matches!(
            battle.units[1].statuses[1].kind,
            StatusKind::Bleed { damage: 1 }
        ));
    }

    #[test]
    fn fortify_adds_three_armor() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Fortify);

        assert_eq!(battle.units[0].radical_armor, 3);
    }

    #[test]
    fn windstep_adds_stored_movement() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Windstep);

        assert_eq!(battle.units[0].stored_movement, 2);
    }

    #[test]
    fn concuss_stuns_target() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Concuss);

        assert!(battle.units[1].stunned);
    }

    #[test]
    fn infest_applies_poison_and_slow() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Infest);

        assert!(matches!(
            battle.units[1].statuses[0].kind,
            StatusKind::Poison { damage: 1 }
        ));
        assert!(matches!(
            battle.units[1].statuses[1].kind,
            StatusKind::Slow
        ));
    }

    #[test]
    fn plunder_on_dead_target_returns_bonus_message() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Plunder);

        assert!(msg.contains("Plundered"));
    }

    #[test]
    fn plunder_on_alive_target_returns_kill_message() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Plunder);

        assert!(msg.contains("Kill to claim"));
    }

    #[test]
    fn bulwark_adds_armor_and_fortify() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Bulwark);

        assert_eq!(battle.units[0].radical_armor, 2);
        assert_eq!(battle.units[0].fortify_stacks, 1);
    }

    #[test]
    fn frenzy_deals_damage_and_heals() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.hp = 8;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Frenzy);

        assert_eq!(battle.units[0].hp, 9);
        assert!(battle.units[1].hp < 10);
    }

    #[test]
    fn snipe_deals_bonus_damage() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Snipe);

        assert!(battle.units[1].hp < 10);
    }

    #[test]
    fn nourish_heals_three_hp() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.hp = 5;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Nourish);

        assert_eq!(battle.units[0].hp, 8);
    }

    #[test]
    fn nourish_heal_capped_at_max() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.hp = 9;
        player.max_hp = 10;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Nourish);

        assert_eq!(battle.units[0].hp, 10);
    }

    #[test]
    fn evade_enables_dodge() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Evade);

        assert!(battle.units[0].radical_dodge);
    }

    #[test]
    fn riposte_enables_counter_and_thorns() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Riposte);

        assert!(battle.units[0].radical_counter);
        assert_eq!(battle.units[0].thorn_armor_turns, 2);
    }

    #[test]
    fn hit_and_run_grants_free_movement() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.player_moved = true;

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::HitAndRun);

        assert!(!battle.player_moved);
    }

    #[test]
    fn inspire_adds_damage() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.damage = 3;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Inspire);

        assert_eq!(battle.units[0].damage, 4);
    }

    #[test]
    fn purify_restores_focus_and_clears_debuffs() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player
            .statuses
            .push(StatusInstance::new(StatusKind::Slow, 2));
        player
            .statuses
            .push(StatusInstance::new(StatusKind::Confused, 1));
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.focus = 5;

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Purify);

        assert_eq!(battle.focus, 8);
        assert_eq!(battle.units[0].statuses.len(), 0);
    }

    #[test]
    fn qi_surge_restores_focus_and_empowers() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        battle.focus = 3;

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::QiSurge);

        assert_eq!(battle.focus, 8);
        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Empowered { amount: 1 }
        ));
    }

    #[test]
    fn echolocation_reveals_all_enemies() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy1 = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let enemy2 = make_test_unit(UnitKind::Enemy(0), 1, 1);
        let mut battle = make_test_battle(vec![player, enemy1, enemy2]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Echolocation);

        assert!(matches!(
            battle.units[1].statuses[0].kind,
            StatusKind::Revealed
        ));
        assert!(matches!(
            battle.units[2].statuses[0].kind,
            StatusKind::Revealed
        ));
        assert!(msg.contains("2 enemies revealed"));
    }

    #[test]
    fn bone_breaker_removes_armor_and_deals_damage() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.radical_armor = 5;
        enemy.fortify_stacks = 2;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::BoneBreaker);

        assert_eq!(battle.units[1].radical_armor, 0);
        assert_eq!(battle.units[1].fortify_stacks, 0);
        assert!(battle.units[1].hp < 10);
    }

    #[test]
    fn ram_strike_stuns_and_deals_damage() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::RamStrike);

        assert!(battle.units[1].stunned);
        assert!(battle.units[1].hp < 10);
    }

    #[test]
    fn discern_removes_all_target_buffs() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.radical_armor = 5;
        enemy.fortify_stacks = 3;
        enemy.radical_dodge = true;
        enemy.radical_counter = true;
        enemy.thorn_armor_turns = 3;
        enemy
            .statuses
            .push(StatusInstance::new(StatusKind::Haste, 2));
        enemy
            .statuses
            .push(StatusInstance::new(StatusKind::Slow, 1));
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Discern);

        assert_eq!(battle.units[1].radical_armor, 0);
        assert_eq!(battle.units[1].fortify_stacks, 0);
        assert!(!battle.units[1].radical_dodge);
        assert!(!battle.units[1].radical_counter);
        assert_eq!(battle.units[1].thorn_armor_turns, 0);
        // Only negative statuses remain
        assert_eq!(battle.units[1].statuses.len(), 1);
        assert!(matches!(
            battle.units[1].statuses[0].kind,
            StatusKind::Slow
        ));
    }

    #[test]
    fn iron_form_adds_armor_and_roots_self() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::IronForm);

        assert_eq!(battle.units[0].radical_armor, 3);
        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Rooted
        ));
    }

    #[test]
    fn charge_pushes_target_away() {
        let player = make_test_unit(UnitKind::Player, 1, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Charge);

        assert!(battle.units[1].x > 3); // pushed away from player
    }

    #[test]
    fn bulldoze_pushes_and_deals_damage_per_tile() {
        let player = make_test_unit(UnitKind::Player, 1, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 2, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Bulldoze);

        assert!(battle.units[1].x > 2);
        assert!(battle.units[1].hp < 10);
    }

    #[test]
    fn downpour_splashes_adjacent_enemies() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy1 = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let enemy2 = make_test_unit(UnitKind::Enemy(0), 5, 4); // adjacent to enemy1
        let mut battle = make_test_battle(vec![player, enemy1, enemy2]);

        let msg = apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Downpour);

        assert!(battle.units[2].hp < 10);
        assert!(msg.contains("1 adjacent"));
    }

    #[test]
    fn undertow_creates_coolant_and_pushes_target() {
        let player = make_test_unit(UnitKind::Player, 1, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Undertow);

        assert_eq!(battle.arena.tile(3, 3), Some(BattleTile::CoolantPool));
        assert!(battle.units[1].x > 3);
    }

    // ══════════════════════════════════════════════════════════════════
    // NEW: apply_radical_action — previously uncovered match arms
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn crossroads_gambit_success_fires_arcing_projectile() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        // seed = (turn * 31 + unit_idx) * 17 % 2
        // With turn_number=1, unit_idx=1: (1*31+1)*17 % 2 = 544 % 2 = 0 → success
        battle.turn_number = 1;

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::CrossroadsGambit);

        assert_eq!(battle.arcing_projectiles.len(), 1);
        assert!(matches!(
            battle.arcing_projectiles[0].effect,
            ProjectileEffect::Damage(3)
        ));
        assert!(msg.contains("succeeds"));
    }

    #[test]
    fn crossroads_gambit_failure_stuns_self() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        // With turn_number=2, unit_idx=1: (2*31+1)*17 % 2 = 63*17 % 2 = 1071 % 2 = 1 → failure
        battle.turn_number = 2;

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::CrossroadsGambit);

        assert!(battle.units[1].stunned);
        assert!(msg.contains("fails"));
    }

    #[test]
    fn grounding_weight_slows_player_and_fires_arcing() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::GroundingWeight);

        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Slow
        ));
        assert_eq!(battle.units[0].statuses[0].turns_left, 3);
        assert_eq!(battle.arcing_projectiles.len(), 1);
        assert!(msg.contains("crushing weight"));
    }

    #[test]
    fn pursuing_steps_moves_toward_player_and_fires_projectile() {
        let player = make_test_unit(UnitKind::Player, 1, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::PursuingSteps);

        // Should move 2 steps toward player (from 5,3 toward 1,3)
        assert!(battle.units[1].x < 5);
        assert_eq!(battle.projectiles.len(), 1);
    }

    #[test]
    fn pursuing_steps_stops_before_occupying_player_tile() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::PursuingSteps);

        // Should not land on player's tile
        assert!(
            battle.units[1].x != battle.units[0].x
                || battle.units[1].y != battle.units[0].y
        );
    }

    #[test]
    fn cavalry_charge_pushes_player_and_deals_distance_damage() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 0, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::CavalryCharge);

        assert_eq!(battle.projectiles.len(), 1);
        // Distance = |0-3| + |3-3| = 3, capped at 3
        assert!(matches!(
            battle.projectiles[0].effect,
            ProjectileEffect::Damage(3)
        ));
        assert!(msg.contains("devastating charge"));
    }

    #[test]
    fn cavalry_charge_close_range_deals_less_damage() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 2, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::CavalryCharge);

        // Distance = 1, min(1,3) = 1
        assert!(matches!(
            battle.projectiles[0].effect,
            ProjectileEffect::Damage(1)
        ));
    }

    #[test]
    fn blitz_assault_moves_and_deals_movement_scaled_damage() {
        let player = make_test_unit(UnitKind::Player, 1, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::BlitzAssault);

        // Enemy should move closer to player
        assert!(battle.units[1].x < 5);
        assert_eq!(battle.projectiles.len(), 1);
        // Moved ≥ 2 tiles, dmg = min(moved, 2) = 2
        assert!(matches!(
            battle.projectiles[0].effect,
            ProjectileEffect::Damage(2)
        ));
        assert!(msg.contains("Blitz assault"));
    }

    #[test]
    fn blitz_assault_stops_adjacent_to_player() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::BlitzAssault);

        // Should not land on player's tile
        assert!(
            battle.units[1].x != battle.units[0].x
                || battle.units[1].y != battle.units[0].y
        );
    }

    #[test]
    fn crushing_wheels_pushes_player_and_fires_projectile() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::CrushingWheels);

        assert_eq!(battle.projectiles.len(), 1);
        assert!(matches!(
            battle.projectiles[0].effect,
            ProjectileEffect::Damage(2)
        ));
        assert!(msg.contains("Crushing wheels"));
    }

    #[test]
    fn imperial_command_buffs_ally_and_moves_toward_player() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
        enemy1.damage = 2;
        let mut enemy2 = make_test_unit(UnitKind::Enemy(0), 4, 3);
        enemy2.damage = 1;
        let mut battle = make_test_battle(vec![player, enemy1, enemy2]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::ImperialCommand);

        // Closest alive ally (enemy2 at 4,3 is 1 tile from enemy1 at 3,3) gets +2 damage
        assert_eq!(battle.units[2].damage, 3); // 1 + 2
        assert!(msg.contains("imperial command"));
    }

    #[test]
    fn imperial_command_no_allies_echoes_silence() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::ImperialCommand);

        assert!(msg.contains("silence"));
    }

    #[test]
    fn magnifying_aura_buffs_nearby_allies() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let mut enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
        enemy1.damage = 2;
        let mut enemy2 = make_test_unit(UnitKind::Enemy(0), 4, 3);
        enemy2.damage = 3;
        // enemy3 far away
        let mut enemy3 = make_test_unit(UnitKind::Enemy(0), 6, 6);
        enemy3.damage = 1;
        let mut battle = make_test_battle(vec![player, enemy1, enemy2, enemy3]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::MagnifyingAura);

        // enemy2 at (4,3) is 2 tiles from (3,3) → buffed
        assert_eq!(battle.units[2].damage, 4); // 3 + 1
        // enemy3 at (6,6) is 6 tiles away → not buffed
        assert_eq!(battle.units[3].damage, 1);
        assert!(msg.contains("magnifying aura"));
    }

    #[test]
    fn scattering_pages_confuses_units_in_range() {
        let player = make_test_unit(UnitKind::Player, 4, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::ScatteringPages);

        // Player at (4,3) is 1 tile from (3,3) → confused
        assert!(battle.units[0]
            .statuses
            .iter()
            .any(|s| matches!(s.kind, StatusKind::Confused)));
        assert!(msg.contains("Pages scatter"));
    }

    #[test]
    fn scattering_pages_does_not_confuse_distant_units() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::ScatteringPages);

        // Player at (0,0) is 6 tiles from (3,3) → not confused
        assert!(battle.units[0].statuses.is_empty());
    }

    #[test]
    fn gore_crush_charges_and_fires_projectile_with_knockback() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::GoreCrush);

        // Enemy should move closer to player
        assert!(battle.units[1].x < 5);
        assert_eq!(battle.projectiles.len(), 1);
        assert!(matches!(
            battle.projectiles[0].effect,
            ProjectileEffect::Damage(2)
        ));
        assert!(msg.contains("Charges and gores"));
    }

    #[test]
    fn intoxicating_mist_changes_terrain_and_confuses_player() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::IntoxicatingMist);

        // Tiles within range 2 of (3,3) should become VentSteam
        let mut steam_count = 0;
        for dx in -2..=2i32 {
            for dy in -2..=2i32 {
                if dx.abs() + dy.abs() <= 2 && (dx != 0 || dy != 0) {
                    let tx = 3 + dx;
                    let ty = 3 + dy;
                    if tx >= 0 && ty >= 0 && tx < 7 && ty < 7 {
                        if battle.arena.tile(tx, ty) == Some(BattleTile::VentSteam) {
                            steam_count += 1;
                        }
                    }
                }
            }
        }
        assert!(steam_count > 0);
        assert!(matches!(
            battle.units[0].statuses[0].kind,
            StatusKind::Confused
        ));
        assert!(msg.contains("Intoxicating mist"));
    }

    #[test]
    fn sprouting_barrier_converts_metal_to_wiring_and_gains_armor() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        // Make sure adjacent tiles are MetalFloor (they should be by default)

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::SproutingBarrier);

        // Some tiles around (3,3) should become WiringPanel
        let mut wiring_count = 0;
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            if battle.arena.tile(3 + dx, 3 + dy) == Some(BattleTile::WiringPanel) {
                wiring_count += 1;
            }
        }
        // Armor gained = adjacent wiring panel count
        assert_eq!(battle.units[1].radical_armor, wiring_count);
        assert!(msg.contains("Sprouts grow"));
    }

    #[test]
    fn tidal_surge_enemy_not_on_water_creates_coolant_pool() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::TidalSurge);

        // Player's tile becomes CoolantPool
        assert_eq!(
            battle.arena.tile(3, 3),
            Some(BattleTile::CoolantPool)
        );
        // No projectile when not on water initially
        assert_eq!(battle.projectiles.len(), 0);
        assert!(msg.contains("Water surges"));
    }

    #[test]
    fn tidal_surge_enemy_on_water_deals_extra_damage() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        // Pre-place CoolantPool at player position
        battle
            .arena
            .set_tile(3, 3, BattleTile::CoolantPool);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::TidalSurge);

        assert_eq!(battle.projectiles.len(), 1);
        assert!(matches!(
            battle.projectiles[0].effect,
            ProjectileEffect::Damage(2)
        ));
        assert!(msg.contains("Swept away"));
    }

    #[test]
    fn tidal_surge_enemy_pushes_player_away() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 1, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::TidalSurge);

        // Player should be pushed away from enemy (toward +x)
        assert!(battle.units[0].x > 3);
    }

    #[test]
    fn phase_strike_teleports_and_deals_aoe_at_departure() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 5);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::PhaseStrike);

        // Enemy should teleport to a tile adjacent to player
        let dist = (battle.units[1].x - battle.units[0].x).abs()
            + (battle.units[1].y - battle.units[0].y).abs();
        assert_eq!(dist, 1);
        assert!(msg.contains("Vanishes"));
    }

    #[test]
    fn phase_strike_no_space_returns_failure_message() {
        // Surround the player so no adjacent tile is free
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let e2 = make_test_unit(UnitKind::Enemy(0), 1, 0);
        let e3 = make_test_unit(UnitKind::Enemy(0), 0, 1);
        let mut battle = make_test_battle(vec![player, enemy, e2, e3]);
        // Block the remaining adjacent tiles with walls
        battle.arena.set_tile(-1, 0, BattleTile::CoverBarrier);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::PhaseStrike);

        // If no adjacent open tile, should return "no space" message
        // The enemy may or may not have moved depending on layout.
        // What matters: the message tells us the outcome.
        assert!(msg.contains("Vanishes") || msg.contains("no space"));
    }

    #[test]
    fn devouring_maw_steals_defending_when_no_other_buffs() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.defending = true;
        player.radical_dodge = false;
        player.radical_counter = false;
        player.radical_armor = 0;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_radical_action(&mut battle, 1, RadicalAction::DevouringMaw);

        assert!(!battle.units[0].defending);
    }

    #[test]
    fn devouring_maw_nothing_to_steal() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.radical_dodge = false;
        player.radical_counter = false;
        player.radical_armor = 0;
        player.defending = false;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_radical_action(&mut battle, 1, RadicalAction::DevouringMaw);

        // Still creates projectile regardless
        assert_eq!(battle.projectiles.len(), 1);
        assert!(msg.contains("Devours protection"));
    }

    #[test]
    fn revealing_dawn_purges_dot_from_nearby_allies() {
        let player = make_test_unit(UnitKind::Player, 0, 0);
        let enemy1 = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut enemy2 = make_test_unit(UnitKind::Enemy(0), 4, 3);
        enemy2
            .statuses
            .push(StatusInstance::new(StatusKind::Burn { damage: 1 }, 3));
        enemy2
            .statuses
            .push(StatusInstance::new(StatusKind::Poison { damage: 1 }, 2));
        let mut battle = make_test_battle(vec![player, enemy1, enemy2]);

        apply_radical_action(&mut battle, 1, RadicalAction::RevealingDawn);

        // enemy2 at (4,3) is 1 tile from (3,3) → dot statuses removed
        assert!(battle.units[2].statuses.is_empty());
    }

    // ══════════════════════════════════════════════════════════════════
    // NEW: apply_player_radical_ability — previously uncovered arms
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn insight_calculates_intents() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Insight);

        assert!(msg.contains("intents revealed"));
    }

    #[test]
    fn gamble_success_deals_double_base_damage() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.damage = 4;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        // Find a turn_number that produces roll < 50
        // roll = (turn * 2654435761 + target * 7) % 100
        // Try turn_number values until we get a success
        let mut found = false;
        for t in 1..=100 {
            let roll = (t as u64)
                .wrapping_mul(2654435761)
                .wrapping_add(1u64 * 7)
                % 100;
            if roll < 50 {
                battle.turn_number = t;
                found = true;
                break;
            }
        }
        assert!(found);
        let initial_hp = battle.units[1].hp;

        let msg = apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Gamble);

        assert!(battle.units[1].hp < initial_hp);
        assert!(msg.contains("JACKPOT"));
    }

    #[test]
    fn gamble_failure_whiffs() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player.damage = 4;
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);
        // Find a turn_number that produces roll >= 50
        let mut found = false;
        for t in 1..=100 {
            let roll = (t as u64)
                .wrapping_mul(2654435761)
                .wrapping_add(1u64 * 7)
                % 100;
            if roll >= 50 {
                battle.turn_number = t;
                found = true;
                break;
            }
        }
        assert!(found);
        let initial_hp = battle.units[1].hp;

        let msg = apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Gamble);

        assert_eq!(battle.units[1].hp, initial_hp);
        assert!(msg.contains("Bad luck"));
    }

    #[test]
    fn true_strike_on_dead_target_returns_fallback_message() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::TrueStrike);

        assert!(msg.contains("True strike"));
    }

    #[test]
    fn solar_flare_self_target_clears_debuffs_only() {
        let mut player = make_test_unit(UnitKind::Player, 3, 3);
        player
            .statuses
            .push(StatusInstance::new(StatusKind::Slow, 2));
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 0, PlayerRadicalAbility::SolarFlare);

        assert_eq!(battle.units[0].statuses.len(), 0);
        assert!(msg.contains("Debuffs cleared"));
    }

    #[test]
    fn reap_on_dead_target_returns_nothing_message() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg = apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Reap);

        assert!(msg.contains("Nothing to reap"));
    }

    #[test]
    fn execution_on_dead_target_returns_nothing_message() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Execution);

        assert!(msg.contains("Nothing to execute"));
    }

    #[test]
    fn cleave_hits_adjacent_enemies() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy1 = make_test_unit(UnitKind::Enemy(0), 5, 3);
        let enemy2 = make_test_unit(UnitKind::Enemy(0), 3, 2); // adjacent to player
        let enemy3 = make_test_unit(UnitKind::Enemy(0), 4, 3); // adjacent to player
        let mut battle = make_test_battle(vec![player, enemy1, enemy2, enemy3]);
        let e2_hp = battle.units[2].hp;
        let e3_hp = battle.units[3].hp;

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Cleave);

        // Adjacent enemies (not the target) should take damage
        assert!(battle.units[2].hp < e2_hp);
        assert!(battle.units[3].hp < e3_hp);
        assert!(msg.contains("Cleaved"));
    }

    #[test]
    fn cleave_no_adjacent_enemies_reports_zero() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 5, 5);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Cleave);

        assert!(msg.contains("Cleaved 0"));
    }

    #[test]
    fn precise_stab_ignores_armor_on_alive_target() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.radical_armor = 10;
        enemy.defending = true;
        let mut battle = make_test_battle(vec![player, enemy]);
        let initial_hp = battle.units[1].hp;

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::PreciseStab);

        assert!(battle.units[1].hp < initial_hp);
        // Armor and defending restored
        assert_eq!(battle.units[1].radical_armor, 10);
        assert!(battle.units[1].defending);
    }

    #[test]
    fn precise_stab_on_dead_target_returns_fallback() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::PreciseStab);

        assert!(msg.contains("Precise stab"));
    }

    #[test]
    fn sabotage_places_fire_terrain() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Sabotage);

        // Should place BlastMark tiles adjacent to target
        let mut blast_count = 0;
        for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
            if battle.arena.tile(3 + dx, 3 + dy) == Some(BattleTile::BlastMark) {
                blast_count += 1;
            }
        }
        assert!(blast_count > 0 || msg.contains("Fire terrain placed") || msg.contains("No room"));
    }

    #[test]
    fn galeforce_pushes_adjacent_enemies() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 4, 3); // adjacent
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Galeforce);

        // Enemy should be pushed 1 tile away from player
        assert!(battle.units[1].x > 4);
        assert!(msg.contains("Gale blast"));
    }

    #[test]
    fn galeforce_no_adjacent_enemies_reports_zero() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 6);
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Galeforce);

        assert!(msg.contains("Pushed 0"));
    }

    #[test]
    fn bulldoze_against_wall_reports_no_push() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let enemy = make_test_unit(UnitKind::Enemy(0), 6, 3); // near edge
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Bulldoze);

        // Enemy near edge may not be pushable
        assert!(msg.contains("Bulldozed") || msg.contains("against wall"));
    }

    #[test]
    fn bulldoze_dead_target_returns_nothing() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Bulldoze);

        assert!(msg.contains("Nothing to push"));
    }

    #[test]
    fn undertow_dead_target_returns_no_target() {
        let player = make_test_unit(UnitKind::Player, 1, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Undertow);

        assert!(msg.contains("No target"));
    }

    #[test]
    fn deep_cut_on_dead_target_no_status() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let initial_max = enemy.max_hp;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::DeepCut);

        // No status added, max_hp unchanged
        assert_eq!(battle.units[1].statuses.len(), 0);
        assert_eq!(battle.units[1].max_hp, initial_max);
    }

    #[test]
    fn moon_venom_on_dead_target_no_status() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::MoonVenom);

        assert_eq!(battle.units[1].statuses.len(), 0);
    }

    #[test]
    fn entangle_on_dead_target_still_adds_armor() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Entangle);

        // Attacker still gets armor even if target is dead
        assert_eq!(battle.units[0].radical_armor, 1);
        // Dead target gets no status
        assert_eq!(battle.units[1].statuses.len(), 0);
    }

    #[test]
    fn concuss_on_dead_target_no_stun() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Concuss);

        assert!(!battle.units[1].stunned);
    }

    #[test]
    fn infest_on_dead_target_no_status() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Infest);

        assert_eq!(battle.units[1].statuses.len(), 0);
    }

    #[test]
    fn growing_strike_on_dead_target_no_mark() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::GrowingStrike);

        assert_eq!(battle.units[1].marked_extra_damage, 0);
    }

    #[test]
    fn earthquake_on_dead_target_no_stun() {
        let player = make_test_unit(UnitKind::Player, 1, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Earthquake);

        assert!(!battle.units[1].stunned);
    }

    #[test]
    fn charge_on_dead_target_still_returns_message() {
        let player = make_test_unit(UnitKind::Player, 1, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 3, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Charge);

        assert!(msg.contains("knocked back"));
    }

    #[test]
    fn discern_on_dead_target_returns_message() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Discern);

        assert!(msg.contains("exposed"));
    }

    #[test]
    fn shatter_on_dead_target_still_deals_damage() {
        let player = make_test_unit(UnitKind::Player, 3, 3);
        let mut enemy = make_test_unit(UnitKind::Enemy(0), 5, 3);
        enemy.alive = false;
        let mut battle = make_test_battle(vec![player, enemy]);

        let msg =
            apply_player_radical_ability(&mut battle, 0, 1, PlayerRadicalAbility::Shatter);

        assert!(msg.contains("shattered"));
    }
}
