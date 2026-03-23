//! Tactical battle (combat arena) rendering.

use crate::combat::{
    ArenaBiome, BattleTile, Direction, EnemyIntent, TacticalBattle, TacticalPhase, TargetMode,
    TypingAction, Weather, WuxingElement,
};
use crate::player::Player;
use crate::radical;

use super::{COL_PLAYER, COL_HP_BAR, COL_HP_BG};

impl super::Renderer {
    pub(crate) fn draw_tactical_battle(&self, battle: &TacticalBattle, anim_t: f64, _player: &Player) {
        let grid_size = battle.arena.width as f64;
        let cell = 36.0_f64;
        let grid_px = grid_size * cell;
        let grid_x = (self.canvas_w - grid_px) / 2.0;
        let grid_y = 30.0;

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
                    let hp_color = if unit.is_player() || unit.is_companion() {
                        "#44cc55"
                    } else if hp_frac > 0.5 {
                        "#cc4444"
                    } else {
                        "#ff6644"
                    };
                    self.ctx.set_fill_style_str(hp_color);
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
            self.ctx
                .set_fill_style_str(&format!("rgba(100,180,255,{})", pulse));
            self.ctx.fill_rect(cx, cy, cell, cell);
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

            // HP bar under unit (thicker, with border)
            let bar_w = cell - 4.0;
            let bar_h = 5.0;
            let bar_x = sx + 2.0;
            let bar_y = sy + cell - 7.0;
            let hp_frac = if unit.max_hp > 0 {
                (unit.hp as f64 / unit.max_hp as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            self.ctx.set_fill_style_str("rgba(0,0,0,0.6)");
            self.ctx
                .fill_rect(bar_x - 0.5, bar_y - 0.5, bar_w + 1.0, bar_h + 1.0);
            self.ctx.set_fill_style_str(COL_HP_BG);
            self.ctx.fill_rect(bar_x, bar_y, bar_w, bar_h);
            let hp_color = if hp_frac > 0.6 {
                "#44cc55"
            } else if hp_frac > 0.3 {
                "#ccaa22"
            } else {
                "#cc3322"
            };
            self.ctx.set_fill_style_str(hp_color);
            self.ctx.fill_rect(bar_x, bar_y, bar_w * hp_frac, bar_h);
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
                        crate::status::StatusKind::SpiritShield => "S",
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
        }

        // Right panel: info area
        let panel_x = grid_x + grid_px + 16.0;
        let panel_w = self.canvas_w - panel_x - 8.0;
        let mut py = grid_y;

        // Panel background
        self.ctx.set_fill_style_str("rgba(10,8,20,0.7)");
        self.ctx
            .fill_rect(panel_x - 6.0, py - 4.0, panel_w + 8.0, grid_px + 8.0);
        self.ctx.set_stroke_style_str("rgba(100,80,140,0.4)");
        self.ctx.set_line_width(1.0);
        self.ctx
            .stroke_rect(panel_x - 6.0, py - 4.0, panel_w + 8.0, grid_px + 8.0);

        let player_unit = &battle.units[0];
        let p_bar_w = panel_w.min(130.0);

        // ─ HP section ─
        self.ctx.set_fill_style_str("#00ccdd");
        self.ctx.set_font("bold 12px monospace");
        self.ctx.set_text_align("left");
        self.ctx.fill_text("HP", panel_x, py + 12.0).ok();
        self.ctx.set_fill_style_str("#cccccc");
        self.ctx.set_font("12px monospace");
        self.ctx.set_text_align("right");
        self.ctx
            .fill_text(
                &format!("{}/{}", player_unit.hp, player_unit.max_hp),
                panel_x + p_bar_w,
                py + 12.0,
            )
            .ok();
        self.ctx.set_text_align("left");
        py += 16.0;

        let p_hp_frac = if player_unit.max_hp > 0 {
            (player_unit.hp as f64 / player_unit.max_hp as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let hp_bar_h = 8.0;
        self.ctx.set_fill_style_str("rgba(0,0,0,0.5)");
        self.ctx
            .fill_rect(panel_x - 1.0, py - 1.0, p_bar_w + 2.0, hp_bar_h + 2.0);
        self.ctx.set_fill_style_str(COL_HP_BG);
        self.ctx.fill_rect(panel_x, py, p_bar_w, hp_bar_h);
        let panel_hp_color = if p_hp_frac > 0.6 {
            COL_HP_BAR
        } else if p_hp_frac > 0.3 {
            "#ccaa22"
        } else {
            "#cc4422"
        };
        self.ctx.set_fill_style_str(panel_hp_color);
        self.ctx
            .fill_rect(panel_x, py, p_bar_w * p_hp_frac, hp_bar_h);
        self.ctx.set_stroke_style_str("rgba(255,255,255,0.12)");
        self.ctx.set_line_width(0.5);
        self.ctx.stroke_rect(panel_x, py, p_bar_w, hp_bar_h);
        py += hp_bar_h + 6.0;

        // ─ Focus section ─
        {
            let focus_frac = if battle.max_focus > 0 {
                (battle.focus as f64 / battle.max_focus as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            self.ctx.set_fill_style_str("#8888cc");
            self.ctx.set_font("bold 10px monospace");
            self.ctx.fill_text("Focus", panel_x, py + 10.0).ok();
            self.ctx.set_fill_style_str("#aaaacc");
            self.ctx.set_font("10px monospace");
            self.ctx.set_text_align("right");
            self.ctx
                .fill_text(
                    &format!("{}/{}", battle.focus, battle.max_focus),
                    panel_x + p_bar_w,
                    py + 10.0,
                )
                .ok();
            self.ctx.set_text_align("left");
            py += 14.0;
            let focus_bar_h = 5.0;
            self.ctx.set_fill_style_str("rgba(0,0,0,0.4)");
            self.ctx
                .fill_rect(panel_x - 1.0, py - 1.0, p_bar_w + 2.0, focus_bar_h + 2.0);
            self.ctx.set_fill_style_str("#222244");
            self.ctx.fill_rect(panel_x, py, p_bar_w, focus_bar_h);
            self.ctx.set_fill_style_str("#6666cc");
            self.ctx
                .fill_rect(panel_x, py, p_bar_w * focus_frac, focus_bar_h);
            self.ctx.set_stroke_style_str("rgba(255,255,255,0.08)");
            self.ctx.set_line_width(0.5);
            self.ctx.stroke_rect(panel_x, py, p_bar_w, focus_bar_h);
            py += focus_bar_h + 6.0;
        }

        // ─ Player status effects ─
        if !player_unit.statuses.is_empty() {
            self.ctx.set_font("9px monospace");
            let mut status_x = panel_x;
            for status in &player_unit.statuses {
                let lbl = status.label();
                self.ctx.set_fill_style_str(status.color());
                self.ctx.fill_text(lbl, status_x, py + 9.0).ok();
                status_x += lbl.len() as f64 * 5.5 + 6.0;
                if status_x > panel_x + p_bar_w - 20.0 {
                    py += 12.0;
                    status_x = panel_x;
                }
            }
            py += 14.0;
        }

        // ─ Stance indicator ─
        if !matches!(battle.player_stance, crate::combat::PlayerStance::Balanced) {
            self.ctx.set_font("10px monospace");
            self.ctx.set_fill_style_str(battle.player_stance.color());
            self.ctx
                .fill_text(
                    &format!("{} {}", battle.player_stance.icon(), battle.player_stance.name()),
                    panel_x,
                    py + 10.0,
                )
                .ok();
            py += 14.0;
        }

        // ─ Separator ─
        self.ctx.set_stroke_style_str("rgba(100,80,140,0.3)");
        self.ctx.set_line_width(1.0);
        self.ctx.begin_path();
        self.ctx.move_to(panel_x, py);
        self.ctx.line_to(panel_x + p_bar_w, py);
        self.ctx.stroke();
        py += 6.0;

        if !matches!(battle.weather, Weather::Normal) {
            let weather_label = battle.weather.name();
            let weather_color = match battle.weather {
                Weather::CoolantLeak => "#4488ff",
                Weather::SmokeScreen => "#aaaaaa",
                Weather::DebrisStorm => "#ccaa44",
                Weather::EnergyFlux => "#cc88ff",
                Weather::Normal => "#888888",
            };
            self.ctx.set_fill_style_str(weather_color);
            self.ctx.set_font("10px monospace");
            self.ctx.fill_text(weather_label, panel_x, py + 10.0).ok();
            py += 14.0;
        }

        if battle.radical_synergy_streak >= 2 {
            self.ctx.set_fill_style_str("#ffaa44");
            self.ctx.set_font("bold 10px monospace");
            let synergy_name = battle.radical_synergy_radical.unwrap_or("?");
            self.ctx
                .fill_text(
                    &format!(
                        "{} Synergy x{}",
                        synergy_name, battle.radical_synergy_streak
                    ),
                    panel_x,
                    py + 10.0,
                )
                .ok();
            py += 14.0;
        }

        // Turn info
        {
            let threshold = if battle.is_boss_battle { 15 } else { 10 };
            let warning = if battle.is_boss_battle { 13 } else { 8 };
            let turn_color = if battle.turn_number >= threshold {
                "#ff4422"
            } else if battle.turn_number >= warning {
                "#ffaa33"
            } else {
                "#8888aa"
            };
            self.ctx.set_fill_style_str(turn_color);
            self.ctx.set_font("10px monospace");
            let label = if battle.turn_number >= threshold {
                format!("Turn {} ─ EXHAUSTION!", battle.turn_number)
            } else if battle.turn_number >= warning {
                format!("Turn {} ─ Ink restless…", battle.turn_number)
            } else {
                format!("Turn {}", battle.turn_number)
            };
            self.ctx.fill_text(&label, panel_x, py + 10.0).ok();
            py += 16.0;
        }

        if let TacticalPhase::EnemyTurn { unit_idx, .. } = battle.phase {
            self.ctx.set_fill_style_str("#ff6666");
            self.ctx.set_font("bold 11px monospace");
            let enemy_name =
                if unit_idx < battle.units.len() && !battle.units[unit_idx].hanzi.is_empty() {
                    battle.units[unit_idx].hanzi
                } else {
                    "Enemy"
                };
            self.ctx
                .fill_text(&format!("{} acts…", enemy_name), panel_x, py + 10.0)
                .ok();
            py += 16.0;
        }

        if let TacticalPhase::Resolve { ref message, .. } = battle.phase {
            self.ctx.set_fill_style_str("#ffdd88");
            self.ctx.set_font("11px monospace");
            self.ctx.fill_text(message, panel_x, py + 10.0).ok();
            py += 16.0;
        }

        // Combo streak with glow
        if battle.combo_streak > 0 {
            let tier = battle.combo_tier_name();
            let combo_color = match battle.combo_streak {
                1..=2 => "#aaddff",
                3..=4 => "#44dd88",
                5..=7 => "#ffdd44",
                8..=11 => "#ff8844",
                _ => "#ff4422",
            };
            self.ctx.set_fill_style_str(combo_color);
            self.ctx.set_font("bold 12px monospace");
            if battle.combo_streak >= 5 {
                self.ctx.set_shadow_color(combo_color);
                self.ctx.set_shadow_blur(6.0);
            }
            self.ctx
                .fill_text(
                    &format!("{} x{}", tier, battle.combo_streak),
                    panel_x,
                    py + 10.0,
                )
                .ok();
            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_shadow_color("transparent");
            py += 16.0;
        }

        // Action menu (Command phase) — styled
        if matches!(battle.phase, TacticalPhase::Command) && battle.typing_action.is_none() {
            py += 4.0;
            self.ctx.set_stroke_style_str("rgba(100,80,140,0.3)");
            self.ctx.set_line_width(1.0);
            self.ctx.begin_path();
            self.ctx.move_to(panel_x, py);
            self.ctx.line_to(panel_x + p_bar_w, py);
            self.ctx.stroke();
            py += 6.0;

            self.ctx.set_font("bold 11px monospace");
            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.fill_text("─ Actions ─", panel_x, py + 10.0).ok();
            py += 16.0;

            let actions: &[(&str, &str, bool)] = &[
                ("M", "Move", !battle.player_moved),
                ("A", "Attack", !battle.player_acted),
                ("S", "Spell", !battle.player_acted),
                ("K", "Skill", !battle.player_acted),
                ("I", "Item", !battle.player_acted),
                ("D", "Defend", !battle.player_acted),
                ("W", "Wait", true),
                ("R", "Rotate", true),
                ("F", "Stance", true),
                ("V", "Look", true),
            ];
            self.ctx.set_font("11px monospace");
            for (hotkey, label, enabled) in actions {
                if *enabled {
                    self.ctx.set_fill_style_str("rgba(255,204,50,0.08)");
                    self.ctx
                        .fill_rect(panel_x - 2.0, py - 1.0, p_bar_w + 2.0, 14.0);
                    self.ctx.set_fill_style_str("#dde0e8");
                } else {
                    self.ctx.set_fill_style_str("#444");
                }
                self.ctx
                    .fill_text(&format!("[{}] {}", hotkey, label), panel_x, py + 10.0)
                    .ok();
                py += 14.0;
            }

            py += 4.0;
            self.ctx.set_fill_style_str("#555");
            self.ctx.set_font("9px monospace");
            self.ctx.fill_text("Esc=flee", panel_x, py + 10.0).ok();
            py += 14.0;
        }

        if battle.spell_menu_open && matches!(battle.phase, TacticalPhase::Command) {
            py += 6.0;
            self.ctx.set_fill_style_str("rgba(0,0,0,0.6)");
            self.ctx.fill_rect(
                panel_x - 4.0,
                py,
                panel_w.min(170.0),
                16.0 + battle.available_spells.len() as f64 * 18.0 + 20.0,
            );
            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.set_font("bold 11px monospace");
            self.ctx.fill_text("Spells:", panel_x, py + 12.0).ok();
            py += 18.0;

            if battle.available_spells.is_empty() {
                self.ctx.set_fill_style_str("#666");
                self.ctx.set_font("10px monospace");
                self.ctx.fill_text("(none)", panel_x, py + 10.0).ok();
                py += 16.0;
            } else {
                self.ctx.set_font("11px monospace");
                for (i, (hanzi, _pinyin, effect)) in battle.available_spells.iter().enumerate() {
                    let selected = i == battle.spell_cursor;
                    if selected {
                        self.ctx.set_fill_style_str("rgba(255,204,50,0.2)");
                        self.ctx
                            .fill_rect(panel_x - 2.0, py - 2.0, panel_w.min(166.0), 16.0);
                        self.ctx.set_fill_style_str("#00ccdd");
                    } else {
                        self.ctx.set_fill_style_str("#aaa");
                    }
                    // Check if casting this spell would trigger a combo
                    let combo_indicator =
                        if let Some(prev_elem) = battle.last_spell_element {
                            if battle.turn_number.saturating_sub(battle.last_spell_turn) <= 2 {
                                if let Some(cur_elem) =
                                    crate::combat::input::spell_effect_element(effect)
                                {
                                    if crate::combat::input::spell_combo_name(prev_elem, cur_elem)
                                        .is_some()
                                    {
                                        "⚡"
                                    } else {
                                        ""
                                    }
                                } else {
                                    ""
                                }
                            } else {
                                ""
                            }
                        } else {
                            ""
                        };
                    let label = effect.label();
                    self.ctx
                        .fill_text(
                            &format!("{}{} {}", combo_indicator, hanzi, label),
                            panel_x,
                            py + 10.0,
                        )
                        .ok();
                    py += 18.0;
                }
            }

            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("9px monospace");
            self.ctx
                .fill_text("Enter=select  Esc=back", panel_x, py + 10.0)
                .ok();
            py += 14.0;
        }

        if battle.item_menu_open && matches!(battle.phase, TacticalPhase::Command) {
            py += 6.0;
            self.ctx.set_fill_style_str("rgba(0,0,0,0.6)");
            self.ctx.fill_rect(
                panel_x - 4.0,
                py,
                panel_w.min(170.0),
                16.0 + battle.available_items.len() as f64 * 18.0 + 20.0,
            );
            self.ctx.set_fill_style_str("#44dd88");
            self.ctx.set_font("bold 11px monospace");
            self.ctx.fill_text("Items:", panel_x, py + 12.0).ok();
            py += 18.0;

            if battle.available_items.is_empty() {
                self.ctx.set_fill_style_str("#666");
                self.ctx.set_font("10px monospace");
                self.ctx.fill_text("(none)", panel_x, py + 10.0).ok();
                py += 16.0;
            } else {
                self.ctx.set_font("11px monospace");
                for (i, (_orig_idx, item)) in battle.available_items.iter().enumerate() {
                    let selected = i == battle.item_cursor;
                    if selected {
                        self.ctx.set_fill_style_str("rgba(68,221,136,0.2)");
                        self.ctx
                            .fill_rect(panel_x - 2.0, py - 2.0, panel_w.min(166.0), 16.0);
                        self.ctx.set_fill_style_str("#44dd88");
                    } else {
                        self.ctx.set_fill_style_str("#aaa");
                    }
                    self.ctx
                        .fill_text(item.short_name(), panel_x, py + 10.0)
                        .ok();
                    py += 18.0;
                }
            }

            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("9px monospace");
            self.ctx
                .fill_text("Enter=use  Esc=back", panel_x, py + 10.0)
                .ok();
            py += 14.0;
        }

        if battle.radical_picker_open && matches!(battle.phase, TacticalPhase::Command) {
            py += 6.0;
            let total_options = 1 + battle.player_radical_abilities.len();
            self.ctx.set_fill_style_str("rgba(0,0,0,0.6)");
            self.ctx.fill_rect(
                panel_x - 4.0,
                py,
                panel_w.min(200.0),
                16.0 + total_options as f64 * 18.0 + 36.0,
            );
            self.ctx.set_fill_style_str("#88ccff");
            self.ctx.set_font("bold 11px monospace");
            self.ctx.fill_text("Attack With:", panel_x, py + 12.0).ok();
            py += 18.0;

            let cursor = battle.radical_picker_cursor;
            self.ctx.set_font("11px monospace");

            let selected = cursor == 0;
            if selected {
                self.ctx.set_fill_style_str("rgba(136,204,255,0.2)");
                self.ctx
                    .fill_rect(panel_x - 2.0, py - 2.0, panel_w.min(196.0), 16.0);
                self.ctx.set_fill_style_str("#88ccff");
            } else {
                self.ctx.set_fill_style_str("#aaa");
            }
            self.ctx
                .fill_text("\u{2694} Normal Attack", panel_x, py + 10.0)
                .ok();
            py += 18.0;

            for (i, (radical, ability)) in battle.player_radical_abilities.iter().enumerate() {
                let selected = cursor == i + 1;
                if selected {
                    self.ctx.set_fill_style_str("rgba(136,204,255,0.2)");
                    self.ctx
                        .fill_rect(panel_x - 2.0, py - 2.0, panel_w.min(196.0), 16.0);
                    self.ctx.set_fill_style_str("#88ccff");
                } else {
                    self.ctx.set_fill_style_str("#aaa");
                }
                self.ctx
                    .fill_text(
                        &format!("{} {}", radical, ability.name()),
                        panel_x,
                        py + 10.0,
                    )
                    .ok();
                py += 18.0;
            }

            if cursor > 0 && cursor <= battle.player_radical_abilities.len() {
                let (_, ability) = &battle.player_radical_abilities[cursor - 1];
                self.ctx.set_fill_style_str("#667799");
                self.ctx.set_font("9px monospace");
                self.ctx
                    .fill_text(ability.description(), panel_x, py + 10.0)
                    .ok();
                py += 14.0;
            }

            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("9px monospace");
            self.ctx
                .fill_text("Enter=select  Esc=back", panel_x, py + 10.0)
                .ok();
            py += 14.0;
        }

        if battle.skill_menu_open && matches!(battle.phase, TacticalPhase::Command) {
            py += 6.0;
            let count = battle.player_radical_abilities.len();
            self.ctx.set_fill_style_str("rgba(0,0,0,0.6)");
            self.ctx.fill_rect(
                panel_x - 4.0,
                py,
                panel_w.min(200.0),
                16.0 + count as f64 * 18.0 + 36.0,
            );
            self.ctx.set_fill_style_str("#ff9944");
            self.ctx.set_font("bold 11px monospace");
            self.ctx.fill_text("Skills:", panel_x, py + 12.0).ok();
            py += 18.0;

            if count == 0 {
                self.ctx.set_fill_style_str("#666");
                self.ctx.set_font("10px monospace");
                self.ctx.fill_text("(none)", panel_x, py + 10.0).ok();
                py += 16.0;
            } else {
                self.ctx.set_font("11px monospace");
                for (i, (radical, ability)) in battle.player_radical_abilities.iter().enumerate() {
                    let selected = i == battle.skill_menu_cursor;
                    if selected {
                        self.ctx.set_fill_style_str("rgba(255,153,68,0.2)");
                        self.ctx
                            .fill_rect(panel_x - 2.0, py - 2.0, panel_w.min(196.0), 16.0);
                        self.ctx.set_fill_style_str("#ff9944");
                    } else {
                        self.ctx.set_fill_style_str("#aaa");
                    }
                    self.ctx
                        .fill_text(
                            &format!("{} {} [{}]", radical, ability.name(), ability.skill_type_label()),
                            panel_x,
                            py + 10.0,
                        )
                        .ok();
                    py += 18.0;
                }
            }

            if battle.skill_menu_cursor < count {
                let (_, ability) = &battle.player_radical_abilities[battle.skill_menu_cursor];
                self.ctx.set_fill_style_str("#997744");
                self.ctx.set_font("9px monospace");
                self.ctx
                    .fill_text(ability.description(), panel_x, py + 10.0)
                    .ok();
                py += 14.0;
            }

            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("9px monospace");
            self.ctx
                .fill_text("Enter=use  Esc=back", panel_x, py + 10.0)
                .ok();
            py += 14.0;
        }

        // Targeting mode label
        if let TacticalPhase::Targeting { ref mode, .. } = battle.phase {
            py += 6.0;
            let label = match mode {
                TargetMode::Move => "Select move target",
                TargetMode::Attack => "Select attack target",
                TargetMode::Spell { .. } => "Select spell target",
                TargetMode::ShieldBreak => "Select shield target",
                TargetMode::Skill => "Select skill target",
            };
            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.set_font("bold 11px monospace");
            self.ctx.fill_text(label, panel_x, py + 10.0).ok();
            py += 16.0;
            self.ctx.set_fill_style_str("#999");
            self.ctx.set_font("9px monospace");
            self.ctx
                .fill_text("Arrows=navigate Enter=confirm", panel_x, py + 10.0)
                .ok();
            py += 12.0;
            self.ctx.fill_text("Esc=cancel", panel_x, py + 10.0).ok();
            py += 16.0;
        }

        if let TacticalPhase::Look { cursor_x, cursor_y } = battle.phase {
            py += 6.0;
            // ── LOOK MODE header ──
            self.ctx.set_fill_style_str("#66ccff");
            self.ctx.set_font("bold 11px monospace");
            self.ctx
                .fill_text("── LOOK MODE ──", panel_x, py + 10.0)
                .ok();
            py += 18.0;

            if let Some(tile) = battle.arena.tile(cursor_x, cursor_y) {
                // ── Tile info section ──
                self.ctx.set_fill_style_str("rgba(40,40,60,0.5)");
                self.ctx
                    .fill_rect(panel_x - 4.0, py - 2.0, p_bar_w + 8.0, 14.0);

                self.ctx.set_fill_style_str("#aaddee");
                self.ctx.set_font("bold 10px monospace");
                self.ctx
                    .fill_text(
                        &format!("Tile: {}", tile.name()),
                        panel_x,
                        py + 10.0,
                    )
                    .ok();
                py += 15.0;

                // Movement cost and LOS
                let move_cost = 1 + tile.extra_move_cost();
                let walkable = tile.is_walkable();
                let blocks = tile.blocks_los();
                self.ctx.set_fill_style_str("#999");
                self.ctx.set_font("9px monospace");
                if !walkable {
                    self.ctx
                        .fill_text("Impassable", panel_x, py + 10.0)
                        .ok();
                } else {
                    self.ctx
                        .fill_text(
                            &format!(
                                "Move: {}  LOS: {}",
                                move_cost,
                                if blocks { "blocked" } else { "clear" }
                            ),
                            panel_x,
                            py + 10.0,
                        )
                        .ok();
                }
                py += 12.0;

                // Special effects
                if let Some(fx) = tile.special_effects() {
                    self.ctx.set_fill_style_str("#ddaa44");
                    self.ctx.set_font("9px monospace");
                    self.ctx
                        .fill_text(&format!("⚡ {}", fx), panel_x, py + 10.0)
                        .ok();
                    py += 12.0;
                }

                // ── Unit info section ──
                for unit in &battle.units {
                    if unit.alive && unit.x == cursor_x && unit.y == cursor_y {
                        py += 4.0;
                        // Separator line
                        self.ctx
                            .set_stroke_style_str("rgba(100,140,180,0.3)");
                        self.ctx.set_line_width(1.0);
                        self.ctx.begin_path();
                        self.ctx.move_to(panel_x, py);
                        self.ctx.line_to(panel_x + p_bar_w, py);
                        self.ctx.stroke();
                        py += 6.0;

                        if unit.is_player() {
                            self.ctx.set_fill_style_str("#00ccdd");
                            self.ctx.set_font("bold 10px monospace");
                            self.ctx
                                .fill_text("You (Player)", panel_x, py + 10.0)
                                .ok();
                            py += 14.0;
                            // Player HP bar
                            let phf = if unit.max_hp > 0 {
                                (unit.hp as f64 / unit.max_hp as f64)
                                    .clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            self.ctx.set_fill_style_str("#333");
                            self.ctx.fill_rect(panel_x, py, p_bar_w, 6.0);
                            self.ctx.set_fill_style_str(if phf > 0.5 {
                                "#44cc44"
                            } else if phf > 0.25 {
                                "#ccaa22"
                            } else {
                                "#cc4422"
                            });
                            self.ctx
                                .fill_rect(panel_x, py, p_bar_w * phf, 6.0);
                            py += 8.0;
                            self.ctx.set_fill_style_str("#ccc");
                            self.ctx.set_font("9px monospace");
                            self.ctx
                                .fill_text(
                                    &format!("HP: {}/{}", unit.hp, unit.max_hp),
                                    panel_x,
                                    py + 10.0,
                                )
                                .ok();
                            py += 14.0;
                        } else if unit.is_companion() {
                            let name = if unit.hanzi.is_empty() {
                                "Companion"
                            } else {
                                unit.hanzi
                            };
                            self.ctx.set_fill_style_str("#44cc88");
                            self.ctx
                                .fill_text(
                                    &format!(
                                        "{} HP:{}/{}",
                                        name, unit.hp, unit.max_hp
                                    ),
                                    panel_x,
                                    py + 10.0,
                                )
                                .ok();
                            py += 14.0;
                        } else {
                            // ── Enemy header ──
                            self.ctx.set_fill_style_str("#ff6666");
                            self.ctx.set_font("bold 10px monospace");
                            let name = if unit.hanzi.is_empty() {
                                "Enemy"
                            } else {
                                unit.hanzi
                            };
                            let label = if !unit.pinyin.is_empty() {
                                format!("{} ({})", name, unit.pinyin)
                            } else {
                                name.to_string()
                            };
                            self.ctx
                                .fill_text(&label, panel_x, py + 10.0)
                                .ok();
                            py += 14.0;

                            // HP bar
                            let ehf = if unit.max_hp > 0 {
                                (unit.hp as f64 / unit.max_hp as f64)
                                    .clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            self.ctx.set_fill_style_str("#333");
                            self.ctx.fill_rect(panel_x, py, p_bar_w, 6.0);
                            self.ctx.set_fill_style_str(if ehf > 0.5 {
                                "#cc4444"
                            } else if ehf > 0.25 {
                                "#cc6622"
                            } else {
                                "#882222"
                            });
                            self.ctx
                                .fill_rect(panel_x, py, p_bar_w * ehf, 6.0);
                            py += 8.0;
                            self.ctx.set_fill_style_str("#ccc");
                            self.ctx.set_font("9px monospace");
                            self.ctx
                                .fill_text(
                                    &format!(
                                        "HP: {}/{}",
                                        unit.hp, unit.max_hp
                                    ),
                                    panel_x,
                                    py + 10.0,
                                )
                                .ok();
                            py += 12.0;

                            // Armor, Speed, Element
                            let mut stats_line = format!(
                                "Armor:{}  Spd:{}",
                                unit.radical_armor, unit.speed
                            );
                            if let Some(elem) = unit.wuxing_element {
                                stats_line.push_str(&format!(
                                    "  {}",
                                    elem.label()
                                ));
                            }
                            self.ctx.set_fill_style_str("#aaa");
                            self.ctx.set_font("9px monospace");
                            self.ctx
                                .fill_text(&stats_line, panel_x, py + 10.0)
                                .ok();
                            py += 13.0;

                            // ── Status effects ──
                            if !unit.statuses.is_empty() {
                                self.ctx.set_font("9px monospace");
                                let mut sx_off = 0.0;
                                for st in &unit.statuses {
                                    let lbl = format!(
                                        "{} {}t",
                                        st.label(),
                                        st.turns_left
                                    );
                                    let lbl_w =
                                        lbl.len() as f64 * 5.5 + 4.0;
                                    if sx_off + lbl_w > p_bar_w
                                        && sx_off > 0.0
                                    {
                                        py += 12.0;
                                        sx_off = 0.0;
                                    }
                                    self.ctx.set_fill_style_str(st.color());
                                    self.ctx
                                        .fill_text(
                                            &lbl,
                                            panel_x + sx_off,
                                            py + 10.0,
                                        )
                                        .ok();
                                    sx_off += lbl_w;
                                }
                                py += 13.0;
                            }

                            // ── Abilities section ──
                            if !unit.radical_actions.is_empty() {
                                py += 2.0;
                                self.ctx.set_stroke_style_str(
                                    "rgba(100,140,180,0.3)",
                                );
                                self.ctx.set_line_width(1.0);
                                self.ctx.begin_path();
                                self.ctx.move_to(panel_x, py);
                                self.ctx.line_to(panel_x + p_bar_w, py);
                                self.ctx.stroke();
                                py += 4.0;

                                self.ctx.set_fill_style_str("#88aacc");
                                self.ctx.set_font("bold 9px monospace");
                                self.ctx
                                    .fill_text(
                                        "── Abilities ──",
                                        panel_x,
                                        py + 10.0,
                                    )
                                    .ok();
                                py += 14.0;

                                for skill in &unit.radical_actions {
                                    // Skill name with radical
                                    self.ctx.set_fill_style_str(
                                        skill.type_color(),
                                    );
                                    self.ctx.set_font("bold 9px monospace");
                                    let skill_label = format!(
                                        "{} {}",
                                        skill.radical(),
                                        skill.name()
                                    );
                                    let display_label =
                                        if skill_label.chars().count() > 24 {
                                            let s: String = skill_label
                                                .chars()
                                                .take(21)
                                                .collect();
                                            format!("{}...", s)
                                        } else {
                                            skill_label
                                        };
                                    self.ctx
                                        .fill_text(
                                            &display_label,
                                            panel_x,
                                            py + 10.0,
                                        )
                                        .ok();
                                    py += 11.0;

                                    // Description (word-wrapped)
                                    self.ctx.set_fill_style_str("#999");
                                    self.ctx.set_font("8px monospace");
                                    let desc = skill.description();
                                    let words: Vec<&str> =
                                        desc.split_whitespace().collect();
                                    let mut line = String::from("  ");
                                    for word in &words {
                                        if line.len() + word.len() + 1 > 24
                                            && line.len() > 2
                                        {
                                            self.ctx
                                                .fill_text(
                                                    &line, panel_x,
                                                    py + 10.0,
                                                )
                                                .ok();
                                            py += 10.0;
                                            line = String::from("  ");
                                        }
                                        if line.len() > 2 {
                                            line.push(' ');
                                        }
                                        line.push_str(word);
                                    }
                                    if line.len() > 2 {
                                        self.ctx
                                            .fill_text(
                                                &line, panel_x, py + 10.0,
                                            )
                                            .ok();
                                        py += 10.0;
                                    }

                                    // Range + Damage + Type
                                    self.ctx.set_fill_style_str("#777");
                                    self.ctx.set_font("8px monospace");
                                    let info_line = format!(
                                        "  {} | {} | {}",
                                        skill.range_info(),
                                        skill.damage_info(),
                                        skill.attack_type()
                                    );
                                    let info_display =
                                        if info_line.chars().count() > 26 {
                                            let s: String = info_line
                                                .chars()
                                                .take(23)
                                                .collect();
                                            format!("{}...", s)
                                        } else {
                                            info_line
                                        };
                                    self.ctx
                                        .fill_text(
                                            &info_display,
                                            panel_x,
                                            py + 10.0,
                                        )
                                        .ok();
                                    py += 13.0;
                                }
                            }

                            // ── Intent ──
                            if let Some(intent) = &unit.intent {
                                py += 2.0;
                                self.ctx.set_fill_style_str("#ddaa44");
                                self.ctx.set_font("bold 9px monospace");
                                self.ctx
                                    .fill_text(
                                        &format!(
                                            "Intent: {} →",
                                            intent.label()
                                        ),
                                        panel_x,
                                        py + 10.0,
                                    )
                                    .ok();
                                py += 14.0;
                            }
                        }
                    }
                }
            }

            py += 6.0;
            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("9px monospace");
            self.ctx
                .fill_text("Arrows=look  Esc/V=exit", panel_x, py + 10.0)
                .ok();
            py += 14.0;
        }

        // Resolve phase message banner
        if let TacticalPhase::Resolve {
            ref message, timer, ..
        } = battle.phase
        {
            let banner_h = 36.0;
            let banner_y = grid_y + grid_px * 0.4;
            // Semi-transparent overlay
            self.ctx.set_fill_style_str("rgba(0,0,0,0.35)");
            self.ctx.fill_rect(grid_x, banner_y, grid_px, banner_h);
            // Message text centered on the grid
            let alpha = if timer > 20 { 1.0 } else { timer as f64 / 20.0 };
            self.ctx
                .set_fill_style_str(&format!("rgba(255,220,100,{})", alpha));
            self.ctx.set_font("bold 14px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(message, grid_x + grid_px / 2.0, banner_y + 22.0)
                .ok();
            self.ctx.set_text_align("left");
        }

        // ── Arena event warning banner ──────────────────────────────────
        if let Some(ref pending) = battle.pending_event {
            let banner_h = 28.0;
            let banner_y = grid_y - 2.0;
            let danger = pending.danger_level();
            let bg_color = match danger {
                "damaging" => "rgba(180,30,30,0.55)",
                "beneficial" => "rgba(30,140,60,0.55)",
                _ => "rgba(40,80,160,0.55)",
            };
            self.ctx.set_fill_style_str(bg_color);
            self.ctx.fill_rect(grid_x, banner_y, grid_px, banner_h);

            let text_color = match danger {
                "damaging" => "#ff6666",
                "beneficial" => "#88ff88",
                _ => "#88bbff",
            };
            self.ctx.set_fill_style_str(text_color);
            self.ctx.set_font("bold 12px monospace");
            self.ctx.set_text_align("center");
            let pulse = (anim_t * 4.0).sin().abs();
            let warning_text = format!("⚠ {} incoming! ⚠", pending.name());
            self.ctx.set_shadow_color(text_color);
            self.ctx.set_shadow_blur(4.0 + pulse * 4.0);
            self.ctx
                .fill_text(&warning_text, grid_x + grid_px / 2.0, banner_y + 18.0)
                .ok();
            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_shadow_color("transparent");
            self.ctx.set_text_align("left");
        }

        // ── Arena event trigger message (large text) ────────────────────
        if let Some(ref event_msg) = battle.event_message {
            if battle.event_message_timer > 0 {
                let alpha = (battle.event_message_timer as f64 / 90.0).min(1.0);
                let scale = 1.0 + (1.0 - alpha) * 0.3;
                let font_size = (20.0 * scale) as u32;
                self.ctx
                    .set_fill_style_str(&format!("rgba(255,240,180,{})", alpha));
                self.ctx
                    .set_font(&format!("bold {}px monospace", font_size));
                self.ctx.set_text_align("center");
                let msg_y = grid_y + grid_px * 0.3;
                self.ctx.set_shadow_color("rgba(255,200,50,0.6)");
                self.ctx.set_shadow_blur(8.0);
                self.ctx
                    .fill_text(event_msg, grid_x + grid_px / 2.0, msg_y)
                    .ok();
                self.ctx.set_shadow_blur(0.0);
                self.ctx.set_shadow_color("transparent");
                self.ctx.set_text_align("left");
            }
        }

        // ── Spell combo notification overlay ─────────────────────────────
        if let Some(ref combo_msg) = battle.combo_message {
            if battle.combo_message_timer > 0 {
                let alpha = (battle.combo_message_timer as f64 / 60.0).min(1.0);
                let scale = 1.0 + (1.0 - alpha) * 0.5;
                let font_size = (22.0 * scale) as u32;
                self.ctx
                    .set_fill_style_str(&format!("rgba(120,220,255,{})", alpha));
                self.ctx
                    .set_font(&format!("bold {}px monospace", font_size));
                self.ctx.set_text_align("center");
                let msg_y = grid_y + grid_px * 0.45;
                self.ctx.set_shadow_color(&format!("rgba(80,180,255,{})", alpha * 0.8));
                self.ctx.set_shadow_blur(12.0);
                self.ctx
                    .fill_text(combo_msg, grid_x + grid_px / 2.0, msg_y)
                    .ok();
                self.ctx.set_shadow_blur(0.0);
                self.ctx.set_shadow_color("transparent");
                self.ctx.set_text_align("left");
            }
        }

        // Typing input box (when active)
        if let Some(ref action) = battle.typing_action {
            let target_label = match action {
                TypingAction::BasicAttack { target_unit } => {
                    let u = &battle.units[*target_unit];
                    if u.hanzi.is_empty() {
                        "Enemy".to_string()
                    } else {
                        format!("{}", u.hanzi)
                    }
                }
                TypingAction::SpellCast {
                    spell_idx, effect, ..
                } => {
                    if *spell_idx < battle.available_spells.len() {
                        let hanzi = battle.available_spells[*spell_idx].0;
                        format!("{} {}", hanzi, effect.label())
                    } else {
                        effect.label().to_string()
                    }
                }
                TypingAction::ShieldBreak { component, .. } => format!("Break {}", component),
                TypingAction::EliteChain {
                    target_unit,
                    syllable_progress,
                    total_syllables,
                    ..
                } => {
                    let u = &battle.units[*target_unit];
                    let hanzi = if u.hanzi.is_empty() { "Enemy" } else { u.hanzi };
                    format!("{} [{}/{}]", hanzi, syllable_progress + 1, total_syllables)
                }
            };

            let input_w = panel_w.min(160.0);
            let input_x = panel_x;
            let input_y = py + 4.0;

            self.ctx.set_fill_style_str("#00ccdd");
            self.ctx.set_font("bold 12px monospace");
            self.ctx
                .fill_text(&target_label, input_x, input_y + 10.0)
                .ok();

            // Show the hanzi large
            if let TypingAction::BasicAttack { target_unit } = action {
                let u = &battle.units[*target_unit];
                if !u.hanzi.is_empty() {
                    self.ctx.set_fill_style_str("#ff6666");
                    self.ctx.set_font("36px 'Noto Serif SC', 'SimSun', serif");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text(u.hanzi, input_x + input_w / 2.0, input_y + 50.0)
                        .ok();
                    self.ctx.set_text_align("left");
                }
            }

            if let TypingAction::SpellCast { spell_idx, .. } = action {
                if *spell_idx < battle.available_spells.len() {
                    let hanzi = battle.available_spells[*spell_idx].0;
                    self.ctx.set_fill_style_str("#44aaff");
                    self.ctx.set_font("36px 'Noto Serif SC', 'SimSun', serif");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text(hanzi, input_x + input_w / 2.0, input_y + 50.0)
                        .ok();
                    self.ctx.set_text_align("left");
                }
            }

            if let TypingAction::EliteChain {
                target_unit,
                syllable_progress,
                total_syllables,
                ..
            } = action
            {
                let u = &battle.units[*target_unit];
                if !u.hanzi.is_empty() {
                    self.ctx.set_fill_style_str("#ff9933");
                    self.ctx.set_font("36px 'Noto Serif SC', 'SimSun', serif");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text(u.hanzi, input_x + input_w / 2.0, input_y + 50.0)
                        .ok();
                    self.ctx.set_text_align("left");

                    let progress_text =
                        format!("Syllable {}/{}", syllable_progress + 1, total_syllables);
                    self.ctx.set_fill_style_str("#00ccdd");
                    self.ctx.set_font("10px monospace");
                    self.ctx.set_text_align("center");
                    self.ctx
                        .fill_text(&progress_text, input_x + input_w / 2.0, input_y + 56.0)
                        .ok();
                    self.ctx.set_text_align("left");
                }
            }

            let box_y = input_y + 58.0;
            self.ctx.set_fill_style_str("rgba(0,0,0,0.5)");
            self.ctx.fill_rect(input_x, box_y, input_w, 26.0);
            self.ctx.set_stroke_style_str("#555");
            self.ctx.set_line_width(1.0);
            self.ctx.stroke_rect(input_x, box_y, input_w, 26.0);

            let display = if battle.typing_buffer.is_empty() {
                "type pinyin…"
            } else {
                &battle.typing_buffer
            };
            self.ctx
                .set_fill_style_str(if battle.typing_buffer.is_empty() {
                    "#555"
                } else {
                    "#00ccdd"
                });
            self.ctx.set_font("14px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(display, input_x + input_w / 2.0, box_y + 18.0)
                .ok();
            self.ctx.set_text_align("left");

            self.ctx.set_fill_style_str("#666");
            self.ctx.set_font("9px monospace");
            self.ctx
                .fill_text("Enter=submit  Esc=cancel", input_x, box_y + 40.0)
                .ok();
        }

        // Battle log (bottom area) — styled with gradient background
        let log_x = grid_x;
        let log_y = grid_y + grid_px + 8.0;
        let log_w = grid_px;
        let log_h = self.canvas_h - log_y - 8.0;
        let line_h = 14.0;
        let max_lines = ((log_h - 10.0) / line_h).floor() as usize;

        // Gradient background: darker at bottom, slightly lighter at top
        for gi in 0..4 {
            let gy_off = gi as f64 * (log_h / 4.0);
            let alpha = 0.45 + gi as f64 * 0.08;
            self.ctx
                .set_fill_style_str(&format!("rgba(8,6,16,{:.3})", alpha));
            self.ctx.fill_rect(log_x, log_y + gy_off, log_w, log_h / 4.0);
        }
        // Accent line at top of log
        self.ctx.set_fill_style_str("rgba(100,80,160,0.3)");
        self.ctx.fill_rect(log_x, log_y, log_w, 1.0);
        self.ctx.set_stroke_style_str("rgba(100,80,140,0.25)");
        self.ctx.set_line_width(1.0);
        self.ctx.stroke_rect(log_x, log_y, log_w, log_h);

        // "LOG" label
        self.ctx.set_fill_style_str("rgba(120,100,160,0.5)");
        self.ctx.set_font("bold 8px monospace");
        self.ctx.set_text_align("right");
        self.ctx
            .fill_text("LOG", log_x + log_w - 4.0, log_y + 10.0)
            .ok();
        self.ctx.set_text_align("left");

        self.ctx.set_font("10px monospace");
        let start = if battle.log.len() > max_lines {
            battle.log.len() - max_lines
        } else {
            0
        };
        let total_showing = battle.log[start..].len();
        for (i, msg) in battle.log[start..].iter().enumerate() {
            let recency = if total_showing > 0 {
                i as f64 / total_showing as f64
            } else {
                1.0
            };
            let alpha = (recency * 0.6 + 0.4).min(1.0);
            let color = if msg.contains("damage") || msg.contains("hit") || msg.contains("kill") {
                format!("rgba(255,130,100,{})", alpha)
            } else if msg.contains("heal") || msg.contains("restore") {
                format!("rgba(100,220,100,{})", alpha)
            } else if msg.contains("ability") || msg.contains("cast") {
                format!("rgba(130,160,255,{})", alpha)
            } else if msg.contains("move") || msg.contains("walk") {
                format!("rgba(160,160,180,{})", alpha * 0.8)
            } else {
                format!("rgba(180,175,190,{})", alpha)
            };
            self.ctx.set_fill_style_str(&color);
            self.ctx
                .fill_text(msg, log_x + 6.0, log_y + 12.0 + i as f64 * line_h)
                .ok();
        }

        // Fade overlay at top of log when scrolled
        if battle.log.len() > max_lines {
            for fade_i in 0..3 {
                let fade_alpha = 0.6 - fade_i as f64 * 0.2;
                self.ctx
                    .set_fill_style_str(&format!("rgba(8,6,16,{})", fade_alpha));
                self.ctx.fill_rect(
                    log_x + 1.0,
                    log_y + 1.0 + fade_i as f64 * 6.0,
                    log_w - 2.0,
                    6.0,
                );
            }
        }

        // Exhaustion border pulse
        {
            let warning_turn: u32 = if battle.is_boss_battle { 13 } else { 8 };
            let threshold: u32 = if battle.is_boss_battle { 15 } else { 10 };
            if battle.turn_number >= warning_turn {
                let intensity = if battle.turn_number >= threshold {
                    0.7
                } else {
                    0.4
                };
                let pulse = ((anim_t * 3.0).sin() * 0.3 + intensity).max(0.1).min(1.0);
                let border_w = 3.0;
                let color = format!("rgba(255,40,40,{})", pulse);
                self.ctx.set_fill_style_str(&color);
                self.ctx.fill_rect(0.0, 0.0, self.canvas_w, border_w);
                self.ctx
                    .fill_rect(0.0, self.canvas_h - border_w, self.canvas_w, border_w);
                self.ctx.fill_rect(0.0, 0.0, border_w, self.canvas_h);
                self.ctx
                    .fill_rect(self.canvas_w - border_w, 0.0, border_w, self.canvas_h);
            }
        }

        // End phase splash
        if let TacticalPhase::End { victory, .. } = battle.phase {
            self.ctx.set_fill_style_str("rgba(0,0,0,0.75)");
            self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

            let text = if victory { "VICTORY" } else { "DEFEAT" };
            let color = if victory { "#44dd88" } else { "#ff4444" };
            let glow = if victory {
                "rgba(68,221,136,0.5)"
            } else {
                "rgba(255,68,68,0.5)"
            };
            self.ctx.set_shadow_color(glow);
            self.ctx.set_shadow_blur(20.0);
            self.ctx.set_fill_style_str(color);
            self.ctx.set_font("bold 48px monospace");
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(text, self.canvas_w / 2.0, self.canvas_h / 2.0)
                .ok();
            self.ctx.set_shadow_blur(0.0);
            self.ctx.set_shadow_color("transparent");

            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("14px monospace");
            self.ctx
                .fill_text(
                    "Press any key to continue",
                    self.canvas_w / 2.0,
                    self.canvas_h / 2.0 + 36.0,
                )
                .ok();
        }
    }
}
