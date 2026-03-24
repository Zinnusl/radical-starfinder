//! Tactical grid, terrain, units, and projectile rendering.

use crate::combat::{
    ArenaBiome, BattleTile, Direction, EnemyIntent, Projectile, TacticalBattle,
    TacticalPhase, TargetMode, TypingAction, Weather, WuxingElement,
};
use crate::player::Player;
use crate::radical;

use super::super::{COL_PLAYER, COL_HP_BAR, COL_HP_BG, hp_gradient_color};

impl super::super::Renderer {
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn draw_tactical_grid(
        &self,
        battle: &TacticalBattle,
        anim_t: f64,
        _player: &Player,
        cell: f64,
        grid_px: f64,
        grid_x: f64,
        grid_y: f64,
        grid_size: f64,
    ) {

        // Full-screen dark backdrop
        self.ctx.set_fill_style_str("rgba(10,6,18,0.94)");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

        // ── Turn order queue strip (top of screen) ──────────────────────
        {
            let tq_cell = 28.0_f64;
            let tq_gap = 3.0;
            let tq_count = battle.turn_queue.len();
            let tq_total_w = tq_count as f64 * (tq_cell + tq_gap) - tq_gap;
            let tq_x0 = (self.canvas_w - tq_total_w) / 2.0;
            let tq_y = 1.0;
            let tq_hp_h = 3.0; // HP pip height below each portrait

            // Subtle background strip
            self.ctx.set_fill_style_str("rgba(0,0,0,0.4)");
            self.ctx.fill_rect(
                tq_x0 - 4.0,
                tq_y - 1.0,
                tq_total_w + 8.0,
                tq_cell + tq_hp_h + 4.0,
            );

            for (qi, &uid) in battle.turn_queue.iter().enumerate() {
                let sx = tq_x0 + qi as f64 * (tq_cell + tq_gap);
                let unit = &battle.units[uid];
                let is_current = qi == battle.turn_queue_pos;

                // Background with rounded corners via path
                let r = 4.0;
                self.ctx.begin_path();
                self.ctx.move_to(sx + r, tq_y);
                self.ctx.line_to(sx + tq_cell - r, tq_y);
                self.ctx
                    .arc(
                        sx + tq_cell - r,
                        tq_y + r,
                        r,
                        -std::f64::consts::FRAC_PI_2,
                        0.0,
                    )
                    .ok();
                self.ctx.line_to(sx + tq_cell, tq_y + tq_cell - r);
                self.ctx
                    .arc(
                        sx + tq_cell - r,
                        tq_y + tq_cell - r,
                        r,
                        0.0,
                        std::f64::consts::FRAC_PI_2,
                    )
                    .ok();
                self.ctx.line_to(sx + r, tq_y + tq_cell);
                self.ctx
                    .arc(
                        sx + r,
                        tq_y + tq_cell - r,
                        r,
                        std::f64::consts::FRAC_PI_2,
                        std::f64::consts::PI,
                    )
                    .ok();
                self.ctx.line_to(sx, tq_y + r);
                self.ctx
                    .arc(
                        sx + r,
                        tq_y + r,
                        r,
                        std::f64::consts::PI,
                        std::f64::consts::PI * 1.5,
                    )
                    .ok();
                self.ctx.close_path();

                let bg = if !unit.alive {
                    "rgba(40,40,40,0.6)"
                } else if unit.is_player() {
                    if is_current {
                        "rgba(100,75,20,0.85)"
                    } else {
                        "rgba(60,45,15,0.75)"
                    }
                } else if unit.is_companion() {
                    if is_current {
                        "rgba(30,90,60,0.85)"
                    } else {
                        "rgba(25,70,50,0.75)"
                    }
                } else if is_current {
                    "rgba(100,25,25,0.85)"
                } else {
                    "rgba(60,20,20,0.7)"
                };
                self.ctx.set_fill_style_str(bg);
                self.ctx.fill();

                // Current turn glow
                if is_current && unit.alive {
                    let glow_pulse = ((anim_t * 4.0).sin() * 0.3 + 0.7).clamp(0.4, 1.0);
                    let glow_color = if unit.is_player() {
                        format!("rgba(255,204,50,{})", glow_pulse)
                    } else if unit.is_companion() {
                        format!("rgba(68,204,136,{})", glow_pulse)
                    } else {
                        format!("rgba(255,80,80,{})", glow_pulse)
                    };
                    self.ctx.set_stroke_style_str(&glow_color);
                    self.ctx.set_line_width(2.5);
                    self.ctx.stroke();
                    // Outer glow via shadow
                    self.ctx.set_shadow_color(&glow_color);
                    self.ctx.set_shadow_blur(6.0);
                    self.ctx.stroke();
                    self.ctx.set_shadow_blur(0.0);
                    self.ctx.set_shadow_color("transparent");

                    // Animated scan line across active portrait
                    let scan_progress = (anim_t * 1.5) % 1.0;
                    let scan_y_pos = tq_y + scan_progress * tq_cell;
                    self.ctx.set_fill_style_str("rgba(255,255,255,0.1)");
                    self.ctx.fill_rect(sx + 2.0, scan_y_pos, tq_cell - 4.0, 2.0);
                } else {
                    self.ctx.set_stroke_style_str("rgba(255,255,255,0.12)");
                    self.ctx.set_line_width(0.5);
                    self.ctx.stroke();
                }

                // Unit glyph
                let glyph = if unit.is_player() {
                    "你"
                } else if unit.is_companion() {
                    if unit.hanzi.is_empty() { "友" } else { unit.hanzi }
                } else if !unit.hanzi.is_empty() {
                    unit.hanzi
                } else {
                    "敌"
                };
                let fg = if !unit.alive {
                    "#555"
                } else if unit.is_player() {
                    "#00ccdd"
                } else if unit.is_companion() {
                    "#44cc88"
                } else {
                    "#ff6666"
                };
                self.ctx.set_fill_style_str(fg);
                self.ctx.set_font("15px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text(glyph, sx + tq_cell / 2.0, tq_y + tq_cell / 2.0 + 5.0)
                    .ok();

                // HP pip under portrait
                if unit.alive {
                    let hp_frac = if unit.max_hp > 0 {
                        (unit.hp as f64 / unit.max_hp as f64).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let pip_y = tq_y + tq_cell + 1.0;
                    self.ctx.set_fill_style_str("rgba(0,0,0,0.5)");
                    self.ctx.fill_rect(sx + 2.0, pip_y, tq_cell - 4.0, tq_hp_h);
                    let hp_color = hp_gradient_color(hp_frac);
                    self.ctx.set_fill_style_str(&hp_color);
                    self.ctx
                        .fill_rect(sx + 2.0, pip_y, (tq_cell - 4.0) * hp_frac, tq_hp_h);
                } else {
                    // Dead unit: X overlay
                    self.ctx.set_fill_style_str("rgba(255,50,50,0.4)");
                    self.ctx.set_font("bold 12px monospace");
                    self.ctx
                        .fill_text("✕", sx + tq_cell / 2.0, tq_y + tq_cell / 2.0 + 4.0)
                        .ok();
                }

                // Turn order number (small, top-left corner)
                self.ctx.set_fill_style_str("rgba(255,255,255,0.3)");
                self.ctx.set_font("bold 7px monospace");
                self.ctx.set_text_align("left");
                self.ctx
                    .fill_text(&format!("{}", qi + 1), sx + 2.0, tq_y + 8.0)
                    .ok();
            }
            self.ctx.set_text_align("left");
        }

        // Grid tiles — sprite-based with flat color fallback
        self.ctx.set_image_smoothing_enabled(false);
        let biome = &battle.arena.biome;
        for gy in 0..battle.arena.height {
            for gx in 0..battle.arena.width {
                let tile = battle
                    .arena
                    .tile(gx as i32, gy as i32)
                    .unwrap_or(BattleTile::MetalFloor);
                let sx = grid_x + gx as f64 * cell;
                let sy = grid_y + gy as f64 * cell;

                let sprite_key = match tile {
                    BattleTile::MetalFloor => match biome {
                        ArenaBiome::StationInterior => "arena_floor_stone",
                        ArenaBiome::DerelictShip => "arena_floor_dark",
                        ArenaBiome::AlienRuins => "arena_floor_arcane",
                        ArenaBiome::IrradiatedZone => "arena_floor_cursed",
                        ArenaBiome::Hydroponics => "arena_floor_garden",
                        ArenaBiome::CryoBay => "arena_floor_frozen",
                        ArenaBiome::ReactorRoom => "arena_floor_infernal",
                    },
                    BattleTile::CoverBarrier => match biome {
                        ArenaBiome::StationInterior => "arena_obstacle_stone",
                        ArenaBiome::DerelictShip => "arena_obstacle_dark",
                        ArenaBiome::AlienRuins => "arena_obstacle_arcane",
                        ArenaBiome::IrradiatedZone => "arena_obstacle_cursed",
                        ArenaBiome::Hydroponics => "arena_obstacle_garden",
                        ArenaBiome::CryoBay => "arena_obstacle_frozen",
                        ArenaBiome::ReactorRoom => "arena_obstacle_infernal",
                    },
                    BattleTile::WiringPanel => "arena_grass",
                    BattleTile::CoolantPool => "arena_water",
                    BattleTile::FrozenCoolant => "arena_ice",
                    BattleTile::BlastMark => "arena_scorched",
                    BattleTile::OilSlick => "arena_ink_pool",
                    BattleTile::DamagedPlating => "arena_broken_ground",
                    BattleTile::VentSteam => "arena_steam",
                    BattleTile::PlasmaPool => "arena_lava",
                    BattleTile::ElectrifiedWire => "arena_thorns",
                    BattleTile::HoloTrap => "arena_arcane_glyph",
                    BattleTile::Debris => "arena_sand",
                    BattleTile::PipeTangle => "arena_bamboo_thicket",
                    BattleTile::CryoZone => "arena_frozen_ground",
                    BattleTile::EnergyNode => "arena_spirit_well",
                    BattleTile::PowerDrain => "arena_spirit_drain",
                    BattleTile::ChargingPad => "arena_meditation_stone",
                    BattleTile::GravityTrap => "arena_soul_trap",
                    BattleTile::CargoCrate => "arena_obstacle_stone",
                    BattleTile::ConveyorN
                    | BattleTile::ConveyorS
                    | BattleTile::ConveyorE
                    | BattleTile::ConveyorW => "arena_water",
                    BattleTile::FuelCanister => "arena_obstacle_stone",
                    BattleTile::WeakenedPlating => "arena_broken_ground",
                    BattleTile::DamagedFloor => "arena_broken_ground",
                    BattleTile::BreachedFloor => "arena_obstacle_dark",
                    BattleTile::MineTile => match biome {
                        ArenaBiome::StationInterior => "arena_floor_stone",
                        ArenaBiome::DerelictShip => "arena_floor_dark",
                        ArenaBiome::AlienRuins => "arena_floor_arcane",
                        ArenaBiome::IrradiatedZone => "arena_floor_cursed",
                        ArenaBiome::Hydroponics => "arena_floor_garden",
                        ArenaBiome::CryoBay => "arena_floor_frozen",
                        ArenaBiome::ReactorRoom => "arena_floor_infernal",
                    },
                    BattleTile::MineTileRevealed => "arena_thorns",
                    BattleTile::Lubricant => "arena_water",
                    BattleTile::ShieldZone => "arena_spirit_well",
                    BattleTile::ElevatedPlatform => "arena_broken_ground",
                    BattleTile::GravityWell => "arena_soul_trap",
                    BattleTile::SteamVentActive => "arena_steam",
                    BattleTile::SteamVentInactive => match biome {
                        ArenaBiome::StationInterior => "arena_floor_stone",
                        ArenaBiome::DerelictShip => "arena_floor_dark",
                        ArenaBiome::AlienRuins => "arena_floor_arcane",
                        ArenaBiome::IrradiatedZone => "arena_floor_cursed",
                        ArenaBiome::Hydroponics => "arena_floor_garden",
                        ArenaBiome::CryoBay => "arena_floor_frozen",
                        ArenaBiome::ReactorRoom => "arena_floor_infernal",
                    },
                    BattleTile::EnergyVentDormant => "arena_meditation_stone",
                    BattleTile::EnergyVentCharging => "arena_arcane_glyph",
                    BattleTile::EnergyVentActive => "arena_lava",
                };

                if !self.draw_tiling_sprite_key(sprite_key, gx, gy, sx, sy, cell) {
                    let fill = match tile {
                        BattleTile::MetalFloor => "#3a3458",
                        BattleTile::CoverBarrier => "#1a1428",
                        BattleTile::WiringPanel => "#2a4a2a",
                        BattleTile::CoolantPool => "#1a2a4a",
                        BattleTile::FrozenCoolant => "#3a4a6a",
                        BattleTile::BlastMark => "#4a2a1a",
                        BattleTile::OilSlick => "#2a2a4a",
                        BattleTile::DamagedPlating => "#3a3030",
                        BattleTile::VentSteam => "#5a5a6a",
                        BattleTile::PlasmaPool => "#6a2a0a",
                        BattleTile::ElectrifiedWire => "#2a3a1a",
                        BattleTile::HoloTrap => "#2a2a5a",
                        BattleTile::Debris => "#5a4a2a",
                        BattleTile::PipeTangle => "#1a3a1a",
                        BattleTile::CryoZone => "#4a5a6a",
                        BattleTile::EnergyNode => "#2244aa",
                        BattleTile::PowerDrain => "#1a0a2a",
                        BattleTile::ChargingPad => "#4a4a5a",
                        BattleTile::GravityTrap => "#3a1a3a",
                        BattleTile::CargoCrate => "#5a4a3a",
                        BattleTile::ConveyorN
                        | BattleTile::ConveyorS
                        | BattleTile::ConveyorE
                        | BattleTile::ConveyorW => "#1a3a5a",
                        BattleTile::FuelCanister => "#6a3a1a",
                        BattleTile::WeakenedPlating => "#3a3430",
                        BattleTile::DamagedFloor => "#4a3a28",
                        BattleTile::BreachedFloor => "#0a0a0a",
                        BattleTile::MineTile => "#3a3458",
                        BattleTile::MineTileRevealed => "#4a2a2a",
                        BattleTile::Lubricant => "#2a2018",
                        BattleTile::ShieldZone => "#4a4a22",
                        BattleTile::ElevatedPlatform => "#5a4a30",
                        BattleTile::GravityWell => "#2a0a3a",
                        BattleTile::SteamVentActive => "#5a5a6a",
                        BattleTile::SteamVentInactive => "#3a3a40",
                        BattleTile::EnergyVentDormant => "#2a3a4a",
                        BattleTile::EnergyVentCharging => "#4a4a1a",
                        BattleTile::EnergyVentActive => "#6a5a0a",
                    };
                    self.ctx.set_fill_style_str(fill);
                    self.ctx.fill_rect(sx, sy, cell, cell);

                    if tile == BattleTile::CoverBarrier {
                        self.ctx.set_fill_style_str("#2a2038");
                        self.ctx
                            .fill_rect(sx + 4.0, sy + 4.0, cell - 8.0, cell - 8.0);
                    }

                    if tile == BattleTile::CargoCrate {
                        self.ctx.set_fill_style_str("#8a7a6a");
                        self.ctx.set_font("bold 14px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("●", sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                            .ok();
                    }

                    if tile == BattleTile::FuelCanister {
                        self.ctx.set_fill_style_str("#ff6633");
                        self.ctx.set_font("bold 14px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("☢", sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                            .ok();
                    }

                    if tile == BattleTile::DamagedFloor {
                        self.ctx.set_fill_style_str("rgba(200,180,120,0.7)");
                        self.ctx.set_font("bold 12px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("⚠", sx + cell / 2.0, sy + cell / 2.0 + 4.0)
                            .ok();
                    }

                    if tile == BattleTile::BreachedFloor {
                        self.ctx.set_fill_style_str("#2a2a2a");
                        self.ctx
                            .fill_rect(sx + 4.0, sy + 4.0, cell - 8.0, cell - 8.0);
                    }

                    if tile == BattleTile::MineTileRevealed {
                        self.ctx.set_fill_style_str("#cc3333");
                        self.ctx.set_font("bold 14px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("▲", sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                            .ok();
                    }

                    if tile == BattleTile::Lubricant {
                        let pulse = ((anim_t * 2.0).sin() * 0.15 + 0.5).max(0.3);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(80,60,20,{})", pulse));
                        self.ctx.set_font("bold 12px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("~", sx + cell / 2.0, sy + cell / 2.0 + 4.0)
                            .ok();
                    }

                    if tile == BattleTile::ShieldZone {
                        let pulse = ((anim_t * 2.5).sin() * 0.2 + 0.7).max(0.4);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(255,215,80,{})", pulse));
                        self.ctx.set_font("bold 14px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("✦", sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                            .ok();
                    }

                    if tile == BattleTile::ElevatedPlatform {
                        self.ctx.set_fill_style_str("rgba(200,180,130,0.7)");
                        self.ctx.set_font("bold 14px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("▲", sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                            .ok();
                    }

                    if tile == BattleTile::GravityWell {
                        let pulse = ((anim_t * 2.0).sin() * 0.2 + 0.7).max(0.4);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(120,40,180,{})", pulse));
                        self.ctx.set_font("bold 14px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("◉", sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                            .ok();
                    }

                    if tile == BattleTile::SteamVentActive {
                        let pulse = ((anim_t * 3.0).sin() * 0.2 + 0.7).max(0.4);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(180,180,200,{})", pulse));
                        self.ctx.set_font("bold 14px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("♨", sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                            .ok();
                    }

                    if tile == BattleTile::SteamVentInactive {
                        self.ctx.set_fill_style_str("rgba(100,100,110,0.4)");
                        self.ctx.set_font("bold 14px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("♨", sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                            .ok();
                    }

                    let flow_arrow = match tile {
                        BattleTile::ConveyorN => Some("↑"),
                        BattleTile::ConveyorS => Some("↓"),
                        BattleTile::ConveyorE => Some("→"),
                        BattleTile::ConveyorW => Some("←"),
                        _ => None,
                    };
                    if let Some(arrow) = flow_arrow {
                        let pulse = ((anim_t * 3.0).sin() * 0.15 + 0.6).max(0.3);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(100,180,255,{})", pulse));
                        self.ctx.set_font("bold 16px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text(arrow, sx + cell / 2.0, sy + cell / 2.0 + 6.0)
                            .ok();
                    }
                }

                // ── Terrain visual effects (animated overlays) ──
                match tile {
                    BattleTile::CoolantPool
                    | BattleTile::ConveyorN
                    | BattleTile::ConveyorS
                    | BattleTile::ConveyorE
                    | BattleTile::ConveyorW => {
                        let wave = ((anim_t * 2.5 + gx as f64 * 0.7 + gy as f64 * 0.5).sin()
                            * 0.12
                            + 0.08)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(80,160,255,{:.3})", wave));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        let wave2 = ((anim_t * 1.8 + gx as f64 * 1.1 - gy as f64 * 0.9).sin()
                            * 0.06
                            + 0.04)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(120,200,255,{:.3})", wave2));
                        self.ctx.fill_rect(sx, sy + cell * 0.5, cell, cell * 0.5);
                    }
                    BattleTile::PlasmaPool => {
                        let glow = ((anim_t * 3.0 + gx as f64 * 0.5 + gy as f64 * 0.3).sin()
                            * 0.15
                            + 0.15)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(255,120,20,{:.3})", glow));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        let flicker =
                            ((anim_t * 7.0 + gx as f64 * 2.3).sin() * 0.08 + 0.05).max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(255,200,50,{:.3})", flicker));
                        self.ctx
                            .fill_rect(sx + 4.0, sy + 4.0, cell - 8.0, cell - 8.0);
                    }
                    BattleTile::FrozenCoolant | BattleTile::CryoZone => {
                        let seed = (gx.wrapping_mul(31).wrapping_add(gy.wrapping_mul(17))) as f64;
                        let sparkle = ((anim_t * 4.0 + seed).sin() * 0.5 + 0.5).max(0.0);
                        if sparkle > 0.7 {
                            let dot_x = sx + (seed * 7.3) % cell;
                            let dot_y = sy + (seed * 13.7) % cell;
                            self.ctx.set_fill_style_str(&format!(
                                "rgba(200,230,255,{:.3})",
                                sparkle * 0.6
                            ));
                            self.ctx.fill_rect(dot_x, dot_y, 2.0, 2.0);
                        }
                        let sparkle2 = ((anim_t * 3.5 + seed * 1.7).sin() * 0.5 + 0.5).max(0.0);
                        if sparkle2 > 0.65 {
                            let dot_x = sx + (seed * 3.1 + 5.0) % cell;
                            let dot_y = sy + (seed * 11.3 + 8.0) % cell;
                            self.ctx.set_fill_style_str(&format!(
                                "rgba(220,240,255,{:.3})",
                                sparkle2 * 0.5
                            ));
                            self.ctx.fill_rect(dot_x, dot_y, 1.5, 1.5);
                        }
                    }
                    BattleTile::WiringPanel | BattleTile::PipeTangle => {
                        let hash = (gx.wrapping_mul(7).wrapping_add(gy.wrapping_mul(13)) % 4) as f64;
                        let shade_alpha = 0.06 + hash * 0.03;
                        let (r, g, b) = if hash > 2.0 {
                            (60, 120, 40)
                        } else {
                            (30, 80, 20)
                        };
                        self.ctx.set_fill_style_str(&format!(
                            "rgba({},{},{},{:.3})",
                            r, g, b, shade_alpha
                        ));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        let sway = ((anim_t * 1.5 + gx as f64 * 0.4).sin() * 0.04 + 0.02)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(80,160,60,{:.3})", sway));
                        self.ctx.fill_rect(sx, sy, cell * 0.5, cell);
                    }
                    BattleTile::OilSlick => {
                        let swirl = ((anim_t * 2.0
                            + gx as f64 * 1.3
                            + gy as f64 * 0.7)
                            .sin()
                            * 0.1
                            + 0.08)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(60,40,120,{:.3})", swirl));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        let swirl2 = ((anim_t * 1.5 - gx as f64 * 0.9 + gy as f64 * 1.1).cos()
                            * 0.06
                            + 0.04)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(30,20,80,{:.3})", swirl2));
                        self.ctx
                            .fill_rect(sx + 2.0, sy + 2.0, cell - 4.0, cell - 4.0);
                    }
                    BattleTile::VentSteam => {
                        let fade = ((anim_t * 2.0 + gx as f64 * 0.6 + gy as f64 * 0.8).sin()
                            * 0.12
                            + 0.15)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(180,180,200,{:.3})", fade));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        let rise =
                            ((anim_t * 3.0 + gx as f64).sin() * 0.06 + 0.04).max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(220,220,240,{:.3})", rise));
                        self.ctx
                            .fill_rect(sx + cell * 0.25, sy, cell * 0.5, cell * 0.6);
                    }
                    BattleTile::Lubricant => {
                        let sheen = ((anim_t * 2.0 + gx as f64 * 0.8 + gy as f64 * 0.6).sin()
                            * 0.1
                            + 0.08)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(140,120,40,{:.3})", sheen));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                    }
                    BattleTile::ShieldZone => {
                        let glow = ((anim_t * 2.5 + gx as f64 * 0.3 + gy as f64 * 0.5).sin()
                            * 0.1
                            + 0.12)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(220,200,100,{:.3})", glow));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                    }
                    BattleTile::ElevatedPlatform => {
                        let glow = ((anim_t * 1.5 + gx as f64 * 0.4 + gy as f64 * 0.6).sin()
                            * 0.08
                            + 0.1)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(200,180,130,{:.3})", glow));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                    }
                    _ => {}
                }

                // ── Interactive tile pulsing glow ──
                match tile {
                    BattleTile::EnergyNode => {
                        let glow = ((anim_t * 3.0 + gx as f64 * 0.4).sin() * 0.12 + 0.15)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(60,120,255,{:.3})", glow));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        self.ctx.set_stroke_style_str(&format!(
                            "rgba(80,160,255,{:.3})",
                            glow + 0.1
                        ));
                        self.ctx.set_line_width(1.0);
                        self.ctx
                            .stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
                    }
                    BattleTile::ChargingPad => {
                        let glow = ((anim_t * 2.5 + gy as f64 * 0.3).sin() * 0.1 + 0.12)
                            .max(0.0);
                        self.ctx.set_fill_style_str(&format!(
                            "rgba(160,140,200,{:.3})",
                            glow
                        ));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        self.ctx.set_stroke_style_str(&format!(
                            "rgba(180,160,220,{:.3})",
                            glow + 0.08
                        ));
                        self.ctx.set_line_width(1.0);
                        self.ctx
                            .stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
                    }
                    BattleTile::FuelCanister => {
                        let glow = ((anim_t * 4.0).sin() * 0.12 + 0.1).max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(255,100,30,{:.3})", glow));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        self.ctx.set_stroke_style_str(&format!(
                            "rgba(255,140,50,{:.3})",
                            glow + 0.1
                        ));
                        self.ctx.set_line_width(1.0);
                        self.ctx
                            .stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
                    }
                    BattleTile::ShieldZone => {
                        let glow = ((anim_t * 2.5 + gx as f64 * 0.3).sin() * 0.1 + 0.12)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(255,220,100,{:.3})", glow));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        self.ctx.set_stroke_style_str(&format!(
                            "rgba(255,230,120,{:.3})",
                            glow + 0.1
                        ));
                        self.ctx.set_line_width(1.0);
                        self.ctx
                            .stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
                    }
                    BattleTile::ElevatedPlatform => {
                        let glow = ((anim_t * 2.0 + gx as f64 * 0.5).sin() * 0.08 + 0.1)
                            .max(0.0);
                        self.ctx
                            .set_fill_style_str(&format!("rgba(180,160,110,{:.3})", glow));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        self.ctx.set_stroke_style_str(&format!(
                            "rgba(200,180,130,{:.3})",
                            glow + 0.08
                        ));
                        self.ctx.set_line_width(1.0);
                        self.ctx
                            .stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
                    }
                    _ => {}
                }

                self.ctx.set_stroke_style_str("rgba(255,255,255,0.06)");
                self.ctx.set_line_width(0.5);
                self.ctx.stroke_rect(sx, sy, cell, cell);
            }
        }
        self.ctx.set_image_smoothing_enabled(true);

        // Trap proximity hint: show "?" on tiles adjacent to hidden traps
        for gy in 0..battle.arena.height {
            for gx in 0..battle.arena.width {
                let x = gx as i32;
                let y = gy as i32;
                if battle.arena.tile(x, y) != Some(BattleTile::MineTile) {
                    continue;
                }
                for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if battle.unit_at(nx, ny).is_some() {
                        let sx = grid_x + gx as f64 * cell;
                        let sy = grid_y + gy as f64 * cell;
                        self.ctx.set_fill_style_str("rgba(255,200,50,0.5)");
                        self.ctx.set_font("bold 12px monospace");
                        self.ctx.set_text_align("center");
                        self.ctx
                            .fill_text("?", sx + cell / 2.0, sy + cell / 2.0 + 4.0)
                            .ok();
                        break;
                    }
                }
            }
        }

        // Grid border
        self.ctx.set_stroke_style_str("#665588");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(grid_x, grid_y, grid_px, grid_px);

        // Ward tile overlays (Pirate Captain boss — shield generator glyphs)
        for &(wx, wy) in &battle.ward_tiles {
            let sx = grid_x + wx as f64 * cell;
            let sy = grid_y + wy as f64 * cell;
            self.ctx.set_fill_style_str("rgba(200,150,50,0.35)");
            self.ctx.fill_rect(sx, sy, cell, cell);
            self.ctx.set_fill_style_str("#cc9933");
            self.ctx.set_font("18px 'Noto Serif SC', 'SimSun', serif");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text("門", sx + cell / 2.0, sy + cell / 2.0 + 6.0)
                .ok();
        }

        // Stolen spell pickup overlays (RadicalThief boss)
        for (sx_pos, sy_pos, hanzi, _, _) in &battle.stolen_spells {
            let sx = grid_x + *sx_pos as f64 * cell;
            let sy = grid_y + *sy_pos as f64 * cell;
            let pulse = ((anim_t * 4.0).sin() * 0.15 + 0.4).max(0.2);
            self.ctx
                .set_fill_style_str(&format!("rgba(100,200,255,{})", pulse));
            self.ctx.fill_rect(sx, sy, cell, cell);
            self.ctx.set_fill_style_str("#66ccff");
            self.ctx.set_font("16px 'Noto Serif SC', 'SimSun', serif");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(hanzi, sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                .ok();
        }

        // Targeting overlay
        if let TacticalPhase::Targeting {
            ref valid_targets,
            cursor_x,
            cursor_y,
            ref aoe_preview,
            ref mode,
            ..
        } = battle.phase
        {
            for &(vx, vy) in valid_targets {
                let sx = grid_x + vx as f64 * cell;
                let sy = grid_y + vy as f64 * cell;
                self.ctx.set_fill_style_str("rgba(100,180,255,0.18)");
                self.ctx.fill_rect(sx, sy, cell, cell);
            }

            // Determine AoE color based on spell type
            let (aoe_r, aoe_g, aoe_b) = match mode {
                TargetMode::Spell { spell_idx } => {
                    if *spell_idx < battle.available_spells.len() {
                        match &battle.available_spells[*spell_idx].2 {
                            radical::SpellEffect::FireAoe(_)
                            | radical::SpellEffect::Cone(_) => (255, 80, 30),
                            radical::SpellEffect::Poison(_, _)
                            | radical::SpellEffect::Drain(_) => (80, 200, 60),
                            radical::SpellEffect::Slow(_)
                            | radical::SpellEffect::Stun => (80, 160, 255),
                            radical::SpellEffect::Heal(_)
                            | radical::SpellEffect::FocusRestore(_) => (80, 220, 120),
                            _ => (255, 160, 60),
                        }
                    } else {
                        (255, 100, 50)
                    }
                }
                _ => (255, 100, 50),
            };

            if aoe_preview.len() > 1 {
                let aoe_pulse = ((anim_t * 5.0).sin() * 0.1 + 0.3).max(0.15);
                for &(ax, ay) in aoe_preview {
                    if ax >= 0
                        && ay >= 0
                        && (ax as usize) < battle.arena.width
                        && (ay as usize) < battle.arena.height
                    {
                        let sx = grid_x + ax as f64 * cell;
                        let sy = grid_y + ay as f64 * cell;
                        self.ctx.set_fill_style_str(&format!(
                            "rgba({},{},{},{:.3})",
                            aoe_r, aoe_g, aoe_b, aoe_pulse
                        ));
                        self.ctx.fill_rect(sx, sy, cell, cell);
                        let br = (aoe_r as i32 + 40).min(255) as u8;
                        let bg = (aoe_g as i32 + 40).min(255) as u8;
                        let bb = (aoe_b as i32 + 10).min(255) as u8;
                        self.ctx.set_stroke_style_str(&format!(
                            "rgba({},{},{},{:.3})",
                            br,
                            bg,
                            bb,
                            aoe_pulse + 0.2
                        ));
                        self.ctx.set_line_width(1.5);
                        self.ctx
                            .stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
                    }
                }
            }

            let cx = grid_x + cursor_x as f64 * cell;
            let cy = grid_y + cursor_y as f64 * cell;
            let pulse = ((anim_t * 6.0).sin() * 0.15 + 0.55).max(0.3);
            self.ctx
                .set_stroke_style_str(&format!("rgba(255,204,50,{:.3})", pulse));
            self.ctx.set_line_width(2.5);
            self.ctx
                .stroke_rect(cx + 1.0, cy + 1.0, cell - 2.0, cell - 2.0);
            // Crosshair on cursor tile
            let ch_alpha = ((anim_t * 4.0).sin() * 0.15 + 0.45).max(0.2);
            self.ctx.set_stroke_style_str(&format!(
                "rgba({},{},{},{:.3})",
                aoe_r, aoe_g, aoe_b, ch_alpha
            ));
            self.ctx.set_line_width(1.0);
            self.ctx.begin_path();
            self.ctx.move_to(cx + cell * 0.5, cy + 2.0);
            self.ctx.line_to(cx + cell * 0.5, cy + cell - 2.0);
            self.ctx.move_to(cx + 2.0, cy + cell * 0.5);
            self.ctx.line_to(cx + cell - 2.0, cy + cell * 0.5);
            self.ctx.stroke();
        }

        // Look mode overlay
        if let TacticalPhase::Look { cursor_x, cursor_y } = battle.phase {
            let cx = grid_x + cursor_x as f64 * cell;
            let cy = grid_y + cursor_y as f64 * cell;
            let pulse = ((anim_t * 4.0).sin() * 0.12 + 0.45).max(0.25);

            // Adjacent tile subtle borders
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let ax = cursor_x + dx;
                let ay = cursor_y + dy;
                if ax >= 0
                    && ay >= 0
                    && (ax as usize) < battle.arena.width
                    && (ay as usize) < battle.arena.height
                {
                    let adj_x = grid_x + ax as f64 * cell;
                    let adj_y = grid_y + ay as f64 * cell;
                    self.ctx.set_fill_style_str("rgba(100,180,255,0.06)");
                    self.ctx.fill_rect(adj_x, adj_y, cell, cell);
                    self.ctx.set_stroke_style_str("rgba(100,180,255,0.18)");
                    self.ctx.set_line_width(1.0);
                    self.ctx
                        .stroke_rect(adj_x + 0.5, adj_y + 0.5, cell - 1.0, cell - 1.0);
                }
            }

            // Hovered tile soft glow (via shadow)
            self.ctx.set_shadow_color("rgba(100,200,255,0.5)");
            self.ctx.set_shadow_blur(8.0);
            self.ctx
                .set_fill_style_str(&format!("rgba(100,180,255,{})", pulse));
            self.ctx.fill_rect(cx, cy, cell, cell);
            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_shadow_color("transparent");

            self.ctx
                .set_stroke_style_str(&format!("rgba(100,200,255,{})", pulse + 0.3));
            self.ctx.set_line_width(2.5);
            self.ctx
                .stroke_rect(cx + 1.0, cy + 1.0, cell - 2.0, cell - 2.0);

            self.ctx.set_fill_style_str("#66ccff");
            self.ctx.set_font("bold 9px monospace");
            self.ctx.set_text_align("center");
            self.ctx.fill_text("LOOK", cx + cell / 2.0, cy - 3.0).ok();
            self.ctx.set_text_align("left");
        }

        if let TacticalPhase::Deployment {
            cursor_x,
            cursor_y,
            ref valid_tiles,
        } = battle.phase
        {
            for &(vx, vy) in valid_tiles {
                let sx = grid_x + vx as f64 * cell;
                let sy = grid_y + vy as f64 * cell;
                self.ctx.set_fill_style_str("rgba(80,255,120,0.15)");
                self.ctx.fill_rect(sx, sy, cell, cell);
            }
            let cx = grid_x + cursor_x as f64 * cell;
            let cy = grid_y + cursor_y as f64 * cell;
            let pulse = ((anim_t * 5.0).sin() * 0.15 + 0.6).max(0.3);
            self.ctx
                .set_stroke_style_str(&format!("rgba(80,255,120,{})", pulse));
            self.ctx.set_line_width(2.5);
            self.ctx
                .stroke_rect(cx + 1.0, cy + 1.0, cell - 2.0, cell - 2.0);
            self.ctx.set_fill_style_str("#66ff88");
            self.ctx.set_font("bold 9px monospace");
            self.ctx.set_text_align("center");
            self.ctx.fill_text("DEPLOY", cx + cell / 2.0, cy - 3.0).ok();
            self.ctx.set_text_align("left");
        }

        // Units
        for (i, unit) in battle.units.iter().enumerate() {
            if !unit.alive {
                continue;
            }
            let sx = grid_x + unit.x as f64 * cell;
            let sy = grid_y + unit.y as f64 * cell;

            if unit.is_player() {
                self.ctx.set_fill_style_str("rgba(255,204,50,0.22)");
                self.ctx
                    .fill_rect(sx + 2.0, sy + 2.0, cell - 4.0, cell - 4.0);
                self.ctx.set_fill_style_str(COL_PLAYER);
                self.ctx.set_font("22px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.set_text_align("center");
                self.ctx
                    .fill_text("你", sx + cell / 2.0, sy + cell / 2.0 + 7.0)
                    .ok();
            } else if unit.is_companion() {
                self.ctx.set_fill_style_str("rgba(68,204,136,0.18)");
                self.ctx
                    .fill_rect(sx + 2.0, sy + 2.0, cell - 4.0, cell - 4.0);
                self.ctx.set_fill_style_str("#44cc88");
                self.ctx.set_font("20px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.set_text_align("center");
                let glyph = if unit.hanzi.is_empty() {
                    "友"
                } else {
                    unit.hanzi
                };
                self.ctx
                    .fill_text(glyph, sx + cell / 2.0, sy + cell / 2.0 + 7.0)
                    .ok();
            } else {
                let is_decoy = unit.is_decoy;
                let bg_color = if is_decoy {
                    "rgba(180,80,255,0.18)"
                } else {
                    "rgba(255,80,80,0.18)"
                };
                self.ctx.set_fill_style_str(bg_color);
                self.ctx
                    .fill_rect(sx + 2.0, sy + 2.0, cell - 4.0, cell - 4.0);
                let fg_color = if is_decoy { "#cc88ff" } else { "#ff6666" };
                self.ctx.set_fill_style_str(fg_color);
                self.ctx.set_font("20px 'Noto Serif SC', 'SimSun', serif");
                self.ctx.set_text_align("center");
                let glyph = if unit.hanzi.is_empty() {
                    "敌"
                } else {
                    unit.hanzi
                };
                self.ctx
                    .fill_text(glyph, sx + cell / 2.0, sy + cell / 2.0 + 7.0)
                    .ok();
                if is_decoy {
                    self.ctx.set_fill_style_str("rgba(200,140,255,0.6)");
                    self.ctx.set_font("8px monospace");
                    self.ctx.set_text_align("right");
                    self.ctx.fill_text("?", sx + cell - 1.0, sy + 10.0).ok();
                }
            }

            // HP bar under unit (with gradient color, drop shadow, shimmer)
            let bar_w = cell - 4.0;
            let bar_h = 5.0;
            let bar_x = sx + 2.0;
            let bar_y = sy + cell - 7.0;
            let hp_frac = if unit.max_hp > 0 {
                (unit.hp as f64 / unit.max_hp as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            // Drop shadow
            self.ctx.set_fill_style_str("rgba(0,0,0,0.7)");
            self.ctx
                .fill_rect(bar_x, bar_y + 1.0, bar_w + 1.0, bar_h + 1.0);
            // Background
            self.ctx.set_fill_style_str(COL_HP_BG);
            self.ctx.fill_rect(bar_x, bar_y, bar_w, bar_h);
            // Gradient fill
            let hp_color = hp_gradient_color(hp_frac);
            self.ctx.set_fill_style_str(&hp_color);
            self.ctx.fill_rect(bar_x, bar_y, bar_w * hp_frac, bar_h);
            // Highlight shimmer (bright line at top of bar)
            self.ctx.set_fill_style_str("rgba(255,255,255,0.18)");
            self.ctx.fill_rect(bar_x, bar_y, bar_w * hp_frac, 1.0);
            // Border
            self.ctx.set_stroke_style_str("rgba(255,255,255,0.15)");
            self.ctx.set_line_width(0.5);
            self.ctx.stroke_rect(bar_x, bar_y, bar_w, bar_h);

            if let Some(wg) = unit.word_group {
                const GROUP_COLORS: &[&str] = &[
                    "#ff9944", "#44ddff", "#ff44cc", "#88ff44", "#ffdd44", "#44ffaa", "#dd88ff",
                    "#ff6688",
                ];
                let color = GROUP_COLORS[wg % GROUP_COLORS.len()];
                self.ctx.set_fill_style_str(color);
                self.ctx
                    .fill_rect(sx + 2.0, sy + cell - 2.0, cell - 4.0, 2.0);
            }

            // Defending indicator
            if unit.defending {
                self.ctx.set_fill_style_str("rgba(100,150,255,0.6)");
                self.ctx.set_font("10px monospace");
                self.ctx.set_text_align("right");
                self.ctx.fill_text("🛡", sx + cell - 1.0, sy + 10.0).ok();
            }

            {
                let arrow = match unit.facing {
                    Direction::North => "▲",
                    Direction::South => "▼",
                    Direction::East => "►",
                    Direction::West => "◄",
                };
                let alpha = if unit.is_player() { "0.7" } else { "0.4" };
                self.ctx
                    .set_fill_style_str(&format!("rgba(255,255,255,{})", alpha));
                self.ctx.set_font("8px monospace");
                self.ctx.set_text_align("left");
                self.ctx.fill_text(arrow, sx + 1.0, sy + 10.0).ok();
            }

            // Momentum indicator (directional arrows)
            if unit.momentum > 0 {
                let arrow_ch = match unit.last_move_dir {
                    Some(Direction::North) => "\u{2191}",
                    Some(Direction::South) => "\u{2193}",
                    Some(Direction::East) => "\u{2192}",
                    Some(Direction::West) => "\u{2190}",
                    None => "\u{2192}",
                };
                let color = match unit.momentum {
                    1 => "rgba(255,255,255,0.6)",
                    2 => "rgba(255,255,100,0.7)",
                    _ => "rgba(255,180,50,0.8)",
                };
                self.ctx.set_fill_style_str(color);
                self.ctx.set_font("7px monospace");
                self.ctx.set_text_align("right");
                let text: String = (0..unit.momentum).map(|_| arrow_ch).collect();
                self.ctx
                    .fill_text(&text, sx + cell - 1.0, sy + cell - 9.0)
                    .ok();
            }

            // Cornered indicator
            {
                let ux = unit.x;
                let uy = unit.y;
                let mut walls = 0i32;
                for &(cdx, cdy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    match battle.arena.tile(ux + cdx, uy + cdy) {
                        None => walls += 1,
                        Some(t) if !t.is_walkable() => walls += 1,
                        _ => {}
                    }
                }
                if walls >= 2 {
                    self.ctx.set_fill_style_str("rgba(255,80,80,0.8)");
                    self.ctx.set_font("bold 7px monospace");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text("!", sx + cell / 2.0, sy + cell - 9.0)
                        .ok();
                }
            }

            // Active turn indicator (brighter, with shadow glow)
            if i == battle.current_unit_idx() {
                let glow_color = if unit.is_player() {
                    "#00ccdd"
                } else if unit.is_companion() {
                    "#44cc88"
                } else {
                    "#ffffff"
                };
                self.ctx.set_shadow_color(glow_color);
                self.ctx.set_shadow_blur(4.0);
                self.ctx.set_stroke_style_str(glow_color);
                self.ctx.set_line_width(2.0);
                self.ctx
                    .stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
                self.ctx.set_shadow_blur(0.0);
                self.ctx.set_shadow_color("transparent");
            }

            if let TacticalPhase::EnemyTurn { unit_idx, .. } = battle.phase {
                if i == unit_idx && !unit.is_player() {
                    let pulse = ((anim_t * 8.0).sin() * 0.3 + 0.7).clamp(0.3, 1.0);
                    let color = if unit.is_companion() {
                        format!("rgba(50,200,120,{})", pulse)
                    } else {
                        format!("rgba(255,80,80,{})", pulse)
                    };
                    self.ctx.set_stroke_style_str(&color);
                    self.ctx.set_line_width(2.5);
                    self.ctx
                        .stroke_rect(sx + 0.5, sy + 0.5, cell - 1.0, cell - 1.0);
                }
            }

            // Status effect icons (small colored dots/text above HP bar)
            if !unit.statuses.is_empty() {
                let max_show = 5;
                let icon_size = 7.0;
                let start_x = sx + 2.0;
                let icon_y = bar_y - icon_size - 1.0;
                for (si, status) in unit.statuses.iter().take(max_show).enumerate() {
                    let ix = start_x + si as f64 * (icon_size + 1.0);
                    let icon_pulse =
                        ((anim_t * 3.0 + si as f64 * 1.2).sin() * 0.15 + 0.85).clamp(0.6, 1.0);
                    self.ctx.set_global_alpha(icon_pulse);
                    self.ctx.set_fill_style_str(status.color());
                    self.ctx.begin_path();
                    self.ctx
                        .arc(
                            ix + icon_size / 2.0,
                            icon_y + icon_size / 2.0,
                            icon_size / 2.0,
                            0.0,
                            std::f64::consts::TAU,
                        )
                        .ok();
                    self.ctx.fill();
                    self.ctx.set_global_alpha(1.0);
                    self.ctx.set_fill_style_str("rgba(0,0,0,0.7)");
                    self.ctx.set_font("bold 5px monospace");
                    self.ctx.set_text_align("center");
                    let short = match status.kind {
                        crate::status::StatusKind::Poison { .. } => "P",
                        crate::status::StatusKind::Burn { .. } => "B",
                        crate::status::StatusKind::Regen { .. } => "R",
                        crate::status::StatusKind::Haste => "H",
                        crate::status::StatusKind::Confused => "?",
                        crate::status::StatusKind::Revealed => "E",
                        crate::status::StatusKind::Envenomed => "V",
                        crate::status::StatusKind::Empowered { .. } => "W",
                        crate::status::StatusKind::Shield => "S",
                        crate::status::StatusKind::Freeze => "F",
                        crate::status::StatusKind::Slow => "~",
                        crate::status::StatusKind::Fear => "!",
                        crate::status::StatusKind::Bleed { .. } => "X",
                        crate::status::StatusKind::Thorns => "T",
                        crate::status::StatusKind::Fortify { .. } => "A",
                        crate::status::StatusKind::Invisible => "I",
                        crate::status::StatusKind::Rooted => "R",
                        crate::status::StatusKind::Weakened => "w",
                        crate::status::StatusKind::Cursed => "C",
                        crate::status::StatusKind::Blessed => "★",
                        crate::status::StatusKind::Wet => "D",
                    };
                    self.ctx
                        .fill_text(short, ix + icon_size / 2.0, icon_y + icon_size / 2.0 + 2.0)
                        .ok();
                }
                if unit.statuses.len() > max_show {
                    self.ctx.set_fill_style_str("rgba(255,255,255,0.5)");
                    self.ctx.set_font("bold 6px monospace");
                    let ix = start_x + max_show as f64 * (icon_size + 1.0);
                    self.ctx
                        .fill_text("+", ix + 2.0, icon_y + icon_size / 2.0 + 2.0)
                        .ok();
                }
            }

            if !unit.is_player() {
                if let Some(ref intent) = unit.intent {
                    let icon = match intent {
                        EnemyIntent::Attack => "⚔",
                        EnemyIntent::Approach => "→",
                        EnemyIntent::RadicalAbility { .. } => "✦",
                        EnemyIntent::Retreat => "←",
                        EnemyIntent::Idle => "·",
                        EnemyIntent::Buff => "↑",
                        EnemyIntent::Heal => "+",
                        EnemyIntent::RangedAttack => "◎",
                        EnemyIntent::Surround => "◇",
                    };
                    let intent_color = match intent {
                        EnemyIntent::Attack => "#ff4444",
                        EnemyIntent::Approach => "#ffaa44",
                        EnemyIntent::RadicalAbility { .. } => "#cc44ff",
                        EnemyIntent::Retreat => "#44aaff",
                        EnemyIntent::Idle => "#888888",
                        EnemyIntent::Buff => "#44ff88",
                        EnemyIntent::Heal => "#44ff44",
                        EnemyIntent::RangedAttack => "#ff8844",
                        EnemyIntent::Surround => "#ffff44",
                    };
                    // Intent icon with background bubble
                    self.ctx.set_fill_style_str("rgba(0,0,0,0.5)");
                    self.ctx.begin_path();
                    self.ctx
                        .arc(sx + cell / 2.0, sy - 4.0, 6.0, 0.0, std::f64::consts::TAU)
                        .ok();
                    self.ctx.fill();
                    self.ctx.set_fill_style_str(intent_color);
                    self.ctx.set_font("bold 9px monospace");
                    self.ctx.set_text_align("center");
                    self.ctx.fill_text(icon, sx + cell / 2.0, sy - 1.0).ok();
                }

                if let Some(ref elem) = unit.wuxing_element {
                    let dot_color = match elem {
                        WuxingElement::Water => "#4488ff",
                        WuxingElement::Fire => "#ff4422",
                        WuxingElement::Metal => "#cccccc",
                        WuxingElement::Wood => "#44cc44",
                        WuxingElement::Earth => "#cc9944",
                    };
                    self.ctx.set_fill_style_str(dot_color);
                    self.ctx.begin_path();
                    self.ctx
                        .arc(sx + cell - 4.0, sy + 4.0, 2.5, 0.0, std::f64::consts::TAU)
                        .ok();
                    self.ctx.fill();
                }

                if unit.mastery_tier >= 2 {
                    let tier_color = if unit.mastery_tier >= 3 {
                        "#44ff44"
                    } else {
                        "#88cc88"
                    };
                    self.ctx.set_fill_style_str(tier_color);
                    self.ctx.set_font("7px monospace");
                    self.ctx.set_text_align("left");
                    let pips = if unit.mastery_tier >= 3 { "***" } else { "**" };
                    self.ctx.fill_text(pips, sx + 1.0, sy + cell - 9.0).ok();
                }

                if let Some(remaining) = unit.charge_remaining {
                    self.ctx.set_fill_style_str("#ffdd00");
                    self.ctx.set_font("bold 8px monospace");
                    self.ctx.set_text_align("right");
                    self.ctx
                        .fill_text(&format!("~{}", remaining), sx + cell - 1.0, sy + cell - 9.0)
                        .ok();
                }
            }
        }

        // ── Projectiles ──────────────────────────────────────────────
        for proj in &battle.projectiles {
            let (px, py) = proj.current_pos();
            let sx = grid_x + px * cell;
            let sy = grid_y + py * cell;

            // Trail behind fast projectiles
            if proj.speed >= Projectile::SPEED_FAST && proj.progress > 0.1 {
                let trail_len = 3;
                for t in 1..=trail_len {
                    let tp = (proj.progress - t as f64 * 0.04).max(0.0);
                    let (tpx, tpy) = (
                        proj.from_x + (proj.to_x as f64 - proj.from_x) * tp,
                        proj.from_y + (proj.to_y as f64 - proj.from_y) * tp,
                    );
                    let tsx = grid_x + tpx * cell;
                    let tsy = grid_y + tpy * cell;
                    let trail_alpha = 0.3 - t as f64 * 0.08;
                    self.ctx.set_fill_style_str(
                        &format!("rgba(200,200,255,{})", trail_alpha.max(0.05)),
                    );
                    self.ctx
                        .fill_rect(tsx + cell / 2.0 - 2.0, tsy + cell / 2.0 - 2.0, 4.0, 4.0);
                }
            }

            self.ctx.set_shadow_color(proj.color);
            self.ctx.set_shadow_blur(8.0);
            self.ctx.set_fill_style_str(proj.color);
            self.ctx.set_font("16px 'Noto Serif SC', 'SimSun', serif");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(proj.glyph, sx + cell / 2.0, sy + cell / 2.0 + 5.0)
                .ok();
        }
        self.ctx.set_shadow_blur(0.0);
        self.ctx.set_shadow_color("transparent");

        for arc in &battle.arcing_projectiles {
            let sx = grid_x + arc.target_x as f64 * cell;
            let sy = grid_y + arc.target_y as f64 * cell;
            let urgent = arc.turns_remaining <= 1;
            let pulse_speed = if urgent { 6.0 } else { 3.5 };
            let pulse = ((anim_t * pulse_speed).sin() * 0.2 + 0.5).clamp(0.25, 0.7);

            // AoE danger zone rendering
            if arc.aoe_radius > 0 {
                let r = arc.aoe_radius as i32;
                for dx in -r..=r {
                    for dy in -r..=r {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let ax = arc.target_x + dx;
                        let ay = arc.target_y + dy;
                        if ax >= 0
                            && ay >= 0
                            && (ax as usize) < battle.arena.width
                            && (ay as usize) < battle.arena.height
                        {
                            let asx = grid_x + ax as f64 * cell;
                            let asy = grid_y + ay as f64 * cell;
                            let aoe_alpha = pulse * 0.25;
                            let (ar, ag, ab) =
                                if urgent { (255, 60, 60) } else { (255, 180, 50) };
                            self.ctx.set_fill_style_str(&format!(
                                "rgba({},{},{},{})",
                                ar, ag, ab, aoe_alpha
                            ));
                            self.ctx
                                .fill_rect(asx + 2.0, asy + 2.0, cell - 4.0, cell - 4.0);
                            self.ctx.set_stroke_style_str(&format!(
                                "rgba({},{},{},{})",
                                ar,
                                ag,
                                ab,
                                aoe_alpha * 1.5
                            ));
                            self.ctx.set_line_width(0.5);
                            self.ctx
                                .stroke_rect(asx + 2.0, asy + 2.0, cell - 4.0, cell - 4.0);
                        }
                    }
                }
            }

            let (fill_r, fill_g, fill_b) = if urgent { (255, 60, 60) } else { (255, 200, 50) };
            let fill_color = format!("rgba({},{},{},{})", fill_r, fill_g, fill_b, pulse * 0.55);
            self.ctx.set_fill_style_str(&fill_color);
            self.ctx.fill_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);

            let border_alpha = if urgent { pulse * 1.2 } else { pulse * 0.8 };
            let border_color = format!("rgba({},{},{},{})", fill_r, fill_g, fill_b, border_alpha.min(1.0));
            if urgent {
                self.ctx.set_shadow_color(&format!("rgba({},{},{},0.8)", fill_r, fill_g, fill_b));
                self.ctx.set_shadow_blur(8.0);
            }
            self.ctx.set_stroke_style_str(&border_color);
            self.ctx.set_line_width(if urgent { 2.5 } else { 1.5 });
            self.ctx.stroke_rect(sx + 1.0, sy + 1.0, cell - 2.0, cell - 2.0);
            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_shadow_color("transparent");

            self.ctx.set_fill_style_str(arc.color);
            self.ctx.set_font("14px 'Noto Serif SC', 'SimSun', serif");
            self.ctx.set_text_align("center");
            self.ctx.fill_text(arc.glyph, sx + cell / 2.0, sy + cell / 2.0 + 2.0).ok();

            let count_color = if urgent { "rgba(255,255,255,0.95)" } else { "rgba(255,255,200,0.8)" };
            self.ctx.set_fill_style_str(count_color);
            self.ctx.set_font("bold 11px monospace");
            self.ctx.set_text_align("right");
            self.ctx.fill_text(
                &format!("{}", arc.turns_remaining),
                sx + cell - 3.0,
                sy + cell - 4.0,
            ).ok();

            // Danger icon for imminent impacts
            if arc.turns_remaining <= 1 {
                self.ctx.set_fill_style_str("rgba(255,40,40,0.9)");
                self.ctx.set_font("bold 10px monospace");
                self.ctx.set_text_align("left");
                self.ctx.fill_text("!", sx + 2.0, sy + 10.0).ok();
            }
        }

        // ── Pending Impact Indicators ──────────────────────────────────
        for imp in &battle.pending_impacts {
            let urgent = imp.turns_until_hit <= 1;
            let pulse_speed = if urgent { 7.0 } else { 3.0 };
            let pulse = ((anim_t * pulse_speed).sin() * 0.25 + 0.55).clamp(0.2, 0.8);

            let (base_r, base_g, base_b) = match imp.element {
                Some(WuxingElement::Fire)  => (255, 60, 20),
                Some(WuxingElement::Water) => (40, 120, 255),
                Some(WuxingElement::Metal) => (200, 200, 220),
                Some(WuxingElement::Wood)  => (60, 200, 60),
                Some(WuxingElement::Earth) => (200, 150, 60),
                None                       => (255, 180, 50),
            };

            let r = imp.radius as i32;
            for dx in -r..=r {
                for dy in -r..=r {
                    let tx = imp.x + dx;
                    let ty = imp.y + dy;
                    if tx < 0
                        || ty < 0
                        || (tx as usize) >= battle.arena.width
                        || (ty as usize) >= battle.arena.height
                    {
                        continue;
                    }
                    let tsx = grid_x + tx as f64 * cell;
                    let tsy = grid_y + ty as f64 * cell;

                    // Pulsing fill
                    let fill_alpha = if urgent { pulse * 0.45 } else { pulse * 0.25 };
                    self.ctx.set_fill_style_str(&format!(
                        "rgba({},{},{},{})",
                        base_r, base_g, base_b, fill_alpha
                    ));
                    self.ctx
                        .fill_rect(tsx + 1.0, tsy + 1.0, cell - 2.0, cell - 2.0);

                    // Border
                    let border_alpha = if urgent { pulse * 1.0 } else { pulse * 0.5 };
                    self.ctx.set_stroke_style_str(&format!(
                        "rgba({},{},{},{})",
                        base_r, base_g, base_b, border_alpha.min(1.0)
                    ));
                    self.ctx.set_line_width(if urgent { 2.0 } else { 1.0 });
                    self.ctx
                        .stroke_rect(tsx + 1.0, tsy + 1.0, cell - 2.0, cell - 2.0);
                }
            }

            // Center glyph and countdown on the impact origin tile
            let csx = grid_x + imp.x as f64 * cell;
            let csy = grid_y + imp.y as f64 * cell;

            if urgent {
                self.ctx.set_shadow_color(&format!(
                    "rgba({},{},{},0.8)",
                    base_r, base_g, base_b
                ));
                self.ctx.set_shadow_blur(8.0);
            }
            self.ctx.set_fill_style_str(imp.color);
            self.ctx.set_font("14px 'Noto Serif SC', 'SimSun', serif");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(imp.glyph, csx + cell / 2.0, csy + cell / 2.0 + 2.0)
                .ok();
            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_shadow_color("transparent");

            // Turn countdown
            let count_color = if urgent {
                "rgba(255,255,255,0.95)"
            } else {
                "rgba(255,255,200,0.8)"
            };
            self.ctx.set_fill_style_str(count_color);
            self.ctx.set_font("bold 11px monospace");
            self.ctx.set_text_align("right");
            self.ctx
                .fill_text(
                    &format!("{}", imp.turns_until_hit),
                    csx + cell - 3.0,
                    csy + cell - 4.0,
                )
                .ok();

            // Danger icon for imminent impacts
            if urgent {
                self.ctx.set_fill_style_str("rgba(255,40,40,0.9)");
                self.ctx.set_font("bold 10px monospace");
                self.ctx.set_text_align("left");
                self.ctx.fill_text("!", csx + 2.0, csy + 10.0).ok();
            }
        }
    }
}
