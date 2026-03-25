//! Starmap, class selection, ship interior, ship upgrades, space combat, and event rendering.

use js_sys::Date;

use crate::game::{EnemyShip, GameSettings, SpaceCombatPhase, SubsystemTarget, ShipWeapon};
use crate::player::{Player, PlayerClass, Ship};
use crate::world::LocationType;
use crate::world::starmap::SectorMap;
use crate::world::ship::{ShipLayout, ShipRoom, ShipTile, get_room_at};
use crate::world::events::SpaceEvent;

use super::COL_PLAYER;
use super::helpers::word_wrap;

impl super::Renderer {
    #[allow(dead_code)]
    pub fn draw_starmap(
        &self,
        sector_map: &SectorMap,
        anim_t: f64,
        _settings: &GameSettings,
        selected_target: Option<usize>,
    ) {
        // Clear background
        self.ctx.set_fill_style_str("#000000");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

        // ========== 1. RICH ANIMATED STARFIELD BACKGROUND ==========
        
        // Draw nebula patches (large semi-transparent colored circles)
        let nebulae = [
            (0.2, 0.3, 180.0, "rgba(88, 44, 120, 0.15)"), // Purple
            (0.7, 0.2, 220.0, "rgba(44, 88, 120, 0.12)"), // Blue
            (0.5, 0.7, 200.0, "rgba(120, 44, 88, 0.10)"), // Red
            (0.1, 0.8, 150.0, "rgba(44, 120, 88, 0.08)"), // Teal
            (0.9, 0.6, 170.0, "rgba(120, 88, 44, 0.11)"), // Orange
        ];
        
        for (nx, ny, radius, color) in nebulae {
            let x = nx * self.canvas_w;
            let y = ny * self.canvas_h;
            self.ctx.set_fill_style_str(color);
            self.ctx.begin_path();
            self.ctx.arc(x, y, radius, 0.0, std::f64::consts::TAU).ok();
            self.ctx.fill();
        }
        
        // Draw 250 background stars with varied sizes, brightness, and twinkling
        for i in 0..250 {
            let seed = (i * 1234567) as u32;
            let x = ((seed.wrapping_mul(2654435761)) % (self.canvas_w as u32)) as f64;
            let y = ((seed.wrapping_mul(987654321)) % (self.canvas_h as u32)) as f64;
            
            // Size: 1-3px
            let size_seed = (seed >> 8) & 3;
            let size = 1.0 + size_seed as f64 * 0.5;
            
            // Color variety
            let color_seed = (seed >> 12) & 15;
            let color = match color_seed {
                0..=1 => "#8888ff", // Blue-white
                2..=3 => "#ffddaa", // Yellow
                4 => "#ffaa88",     // Red-orange
                _ => "#ffffff",     // White (most common)
            };
            
            // Twinkling: some stars pulse
            let twinkle_seed = (seed >> 16) & 7;
            let alpha = if twinkle_seed < 2 {
                // Twinkling stars
                let phase = anim_t * (2.0 + twinkle_seed as f64 * 0.5);
                0.3 + 0.7 * ((phase + i as f64 * 0.5).sin() * 0.5 + 0.5)
            } else {
                // Static stars
                0.6 + (seed & 0xFF) as f64 / 255.0 * 0.4
            };
            
            self.ctx.set_global_alpha(alpha);
            self.ctx.set_fill_style_str(color);
            self.ctx.fill_rect(x, y, size, size);
        }
        self.ctx.set_global_alpha(1.0);

        // ========== 2. SECTOR PROGRESS BAR (top) ==========
        if let Some(sector) = sector_map.sectors.get(sector_map.current_sector) {
            // Progress bar background
            self.ctx.set_fill_style_str("rgba(20, 20, 40, 0.8)");
            self.ctx.fill_rect(100.0, 5.0, self.canvas_w - 200.0, 20.0);
            
            // Find start, boss, exit systems
            let mut start_x = 0.0;
            let mut boss_x = 1.0;
            let mut exit_x = 1.0;
            let mut current_x = 0.5;
            
            for sys in &sector.systems {
                if sys.id == 0 { start_x = sys.x; }
                if sys.id == sector_map.current_system { current_x = sys.x; }
                // Assume boss is marked by difficulty > 8 or event_id exists
                if sys.difficulty >= 8 || sys.event_id.is_some() { 
                    if sys.x > boss_x { boss_x = sys.x; }
                }
                // Exit is rightmost
                if sys.x > exit_x { exit_x = sys.x; }
            }
            
            // Progress fill (how far player has gone)
            let progress = (current_x - start_x) / (exit_x - start_x).max(0.01);
            let bar_width = (self.canvas_w - 200.0) * progress;
            self.ctx.set_fill_style_str("rgba(0, 200, 220, 0.5)");
            self.ctx.fill_rect(100.0, 5.0, bar_width, 20.0);
            
            // Labels
            self.ctx.set_font("10px monospace");
            self.ctx.set_fill_style_str("#00dd88");
            self.ctx.fill_text("START", 105.0, 18.0).ok();
            
            self.ctx.set_fill_style_str("#ff4444");
            let boss_screen_x = 100.0 + (boss_x - start_x) / (exit_x - start_x).max(0.01) * (self.canvas_w - 200.0);
            self.ctx.fill_text("⚠ BOSS", boss_screen_x - 30.0, 18.0).ok();
            
            self.ctx.set_fill_style_str("#ffdd44");
            self.ctx.fill_text("EXIT →", self.canvas_w - 145.0, 18.0).ok();
        }

        // ========== 3. DRAW SECTOR MAP ==========
        if let Some(sector) = sector_map.sectors.get(sector_map.current_sector) {
            let cx = self.canvas_w / 2.0;
            let cy = self.canvas_h / 2.0 + 20.0; // Offset down a bit for progress bar
            let scale = 300.0;

            // ========== 3a. DRAW CONNECTIONS ==========
            for system in &sector.systems {
                let sx = cx + (system.x - 0.5) * scale * 2.0;
                let sy = cy + (system.y - 0.5) * scale * 2.0;

                for &target_id in &system.connections {
                    if let Some(target) = sector.systems.iter().find(|s| s.id == target_id) {
                        let tx = cx + (target.x - 0.5) * scale * 2.0;
                        let ty = cy + (target.y - 0.5) * scale * 2.0;
                        
                        // Determine if this is main path (x increases) or branch
                        let is_main_path = target.x > system.x + 0.05;
                        
                        // Animated line to selected target
                        let is_selected_connection = Some(target_id) == selected_target 
                            && system.id == sector_map.current_system;
                        
                        if is_selected_connection {
                            // Pulsing animated line
                            let pulse = (anim_t * 4.0).sin() * 0.5 + 0.5;
                            let alpha = 0.5 + pulse * 0.5;
                            self.ctx.set_global_alpha(alpha);
                            self.ctx.set_stroke_style_str("#00ffff");
                            self.ctx.set_line_width(3.0);
                        } else if is_main_path {
                            self.ctx.set_stroke_style_str("#556688");
                            self.ctx.set_line_width(2.0);
                        } else {
                            self.ctx.set_stroke_style_str("#333355");
                            self.ctx.set_line_width(1.0);
                        }
                        
                        self.ctx.begin_path();
                        self.ctx.move_to(sx, sy);
                        self.ctx.line_to(tx, ty);
                        self.ctx.stroke();
                        self.ctx.set_global_alpha(1.0);
                        
                        // Draw fuel cost on connection
                        if system.id == sector_map.current_system {
                            let dx = target.x - system.x;
                            let dy = target.y - system.y;
                            let fuel_cost = ((dx * dx + dy * dy).sqrt() * 10.0).ceil() as i32;
                            
                            let mid_x = (sx + tx) / 2.0;
                            let mid_y = (sy + ty) / 2.0;
                            
                            self.ctx.set_font("10px monospace");
                            self.ctx.set_fill_style_str("rgba(255, 220, 100, 0.9)");
                            self.ctx.fill_text(&format!("{}", fuel_cost), mid_x + 5.0, mid_y - 5.0).ok();
                        }
                    }
                }
            }

            // ========== 3b. DRAW SYSTEMS ==========
            for system in &sector.systems {
                let sx = cx + (system.x - 0.5) * scale * 2.0;
                let sy = cy + (system.y - 0.5) * scale * 2.0;
                let is_current = system.id == sector_map.current_system;
                let is_boss = system.difficulty >= 8 || system.event_id.is_some();
                let is_exit = system.x >= 0.95; // Rightmost systems

                // Determine color by type (if not visited) or override for current/visited
                let (color, glow_color) = if is_current {
                    ("#00ffff", Some("#00ffff"))
                } else if is_boss {
                    ("#ff4444", Some("#ff4444"))
                } else if is_exit {
                    ("#ffdd44", Some("#ffdd44"))
                } else if system.visited {
                    ("#44aa88", None)
                } else {
                    match system.location_type {
                        LocationType::SpaceStation => ("#ffdd44", None),
                        LocationType::AsteroidBase => ("#cc7733", None),
                        LocationType::DerelictShip => ("#8844cc", None),
                        LocationType::AlienRuins => ("#44ccaa", None),
                        LocationType::TradingPost => ("#44dd44", None),
                        LocationType::OrbitalPlatform => ("#4488ff", None),
                        LocationType::MiningColony => ("#cc8844", None),
                        LocationType::ResearchLab => ("#ff88cc", None),
                    }
                };
                
                // Pulsing glow for current/boss/exit systems
                if let Some(glow) = glow_color {
                    let pulse = (anim_t * 3.0).sin() * 0.3 + 0.7;
                    self.ctx.set_global_alpha(pulse * 0.3);
                    self.ctx.set_fill_style_str(glow);
                    self.ctx.begin_path();
                    self.ctx.arc(sx, sy, 20.0, 0.0, std::f64::consts::TAU).ok();
                    self.ctx.fill();
                    self.ctx.set_global_alpha(1.0);
                }
                
                // Draw system icon by type
                self.ctx.set_fill_style_str(color);
                self.ctx.set_stroke_style_str(color);
                self.ctx.set_line_width(2.0);
                
                match system.location_type {
                    LocationType::SpaceStation => {
                        // Square with inner dot
                        self.ctx.stroke_rect(sx - 7.0, sy - 7.0, 14.0, 14.0);
                        self.ctx.begin_path();
                        self.ctx.arc(sx, sy, 3.0, 0.0, std::f64::consts::TAU).ok();
                        self.ctx.fill();
                    }
                    LocationType::AsteroidBase => {
                        // Triangle
                        self.ctx.begin_path();
                        self.ctx.move_to(sx, sy - 8.0);
                        self.ctx.line_to(sx - 7.0, sy + 6.0);
                        self.ctx.line_to(sx + 7.0, sy + 6.0);
                        self.ctx.line_to(sx, sy - 8.0);
                        self.ctx.fill();
                    }
                    LocationType::DerelictShip => {
                        // X shape
                        self.ctx.begin_path();
                        self.ctx.move_to(sx - 7.0, sy - 7.0);
                        self.ctx.line_to(sx + 7.0, sy + 7.0);
                        self.ctx.move_to(sx + 7.0, sy - 7.0);
                        self.ctx.line_to(sx - 7.0, sy + 7.0);
                        self.ctx.stroke();
                    }
                    LocationType::AlienRuins => {
                        // Diamond
                        self.ctx.begin_path();
                        self.ctx.move_to(sx, sy - 8.0);
                        self.ctx.line_to(sx + 8.0, sy);
                        self.ctx.line_to(sx, sy + 8.0);
                        self.ctx.line_to(sx - 8.0, sy);
                        self.ctx.line_to(sx, sy - 8.0);
                        self.ctx.fill();
                    }
                    LocationType::TradingPost => {
                        // Large circle with ring
                        self.ctx.begin_path();
                        self.ctx.arc(sx, sy, 6.0, 0.0, std::f64::consts::TAU).ok();
                        self.ctx.fill();
                        self.ctx.begin_path();
                        self.ctx.arc(sx, sy, 9.0, 0.0, std::f64::consts::TAU).ok();
                        self.ctx.stroke();
                    }
                    LocationType::OrbitalPlatform => {
                        // Rectangle/dash
                        self.ctx.fill_rect(sx - 10.0, sy - 3.0, 20.0, 6.0);
                    }
                    LocationType::MiningColony => {
                        // Pentagon
                        self.ctx.begin_path();
                        for i in 0..5 {
                            let angle = (i as f64 / 5.0) * std::f64::consts::TAU - std::f64::consts::PI / 2.0;
                            let px = sx + 7.0 * angle.cos();
                            let py = sy + 7.0 * angle.sin();
                            if i == 0 {
                                self.ctx.move_to(px, py);
                            } else {
                                self.ctx.line_to(px, py);
                            }
                        }
                        self.ctx.line_to(sx + 7.0 * (-std::f64::consts::PI / 2.0).cos(), 
                                        sy + 7.0 * (-std::f64::consts::PI / 2.0).sin());
                        self.ctx.fill();
                    }
                    LocationType::ResearchLab => {
                        // 4-pointed star
                        self.ctx.begin_path();
                        for i in 0..8 {
                            let angle = (i as f64 / 8.0) * std::f64::consts::TAU;
                            let r = if i % 2 == 0 { 8.0 } else { 3.0 };
                            let px = sx + r * angle.cos();
                            let py = sy + r * angle.sin();
                            if i == 0 {
                                self.ctx.move_to(px, py);
                            } else {
                                self.ctx.line_to(px, py);
                            }
                        }
                        self.ctx.fill();
                    }
                }
                
                // Selection highlight
                if Some(system.id) == selected_target {
                    self.ctx.set_stroke_style_str("#00ffff");
                    self.ctx.set_line_width(3.0);
                    let pulse_r = 12.0 + (anim_t * 5.0).sin() * 2.0;
                    self.ctx.begin_path();
                    self.ctx.arc(sx, sy, pulse_r, 0.0, std::f64::consts::TAU).ok();
                    self.ctx.stroke();
                }
                
                // ========== 4. SYSTEM LABELS ==========
                // Always show names, but dimmer for unvisited
                let name_alpha = if system.visited || is_current { 1.0 } else { 0.6 };
                self.ctx.set_global_alpha(name_alpha);
                
                self.ctx.set_font("12px monospace");
                self.ctx.set_fill_style_str("#ffffff");
                self.ctx.fill_text(system.name, sx + 12.0, sy - 2.0).ok();
                
                self.ctx.set_font("10px monospace");
                self.ctx.set_fill_style_str("#aaaaaa");
                self.ctx.fill_text(system.chinese_name, sx + 12.0, sy + 10.0).ok();
                
                // Show location type for unvisited
                if !system.visited && !is_current {
                    let type_name = match system.location_type {
                        LocationType::SpaceStation => "Station",
                        LocationType::AsteroidBase => "Asteroid",
                        LocationType::DerelictShip => "Derelict",
                        LocationType::AlienRuins => "Ruins",
                        LocationType::TradingPost => "Trading",
                        LocationType::OrbitalPlatform => "Platform",
                        LocationType::MiningColony => "Mining",
                        LocationType::ResearchLab => "Lab",
                    };
                    self.ctx.set_font("9px monospace");
                    self.ctx.set_fill_style_str("#888888");
                    self.ctx.fill_text(type_name, sx + 12.0, sy + 20.0).ok();
                }
                
                // Hazard indicator
                if system.hazard.is_some() {
                    self.ctx.set_font("10px monospace");
                    self.ctx.set_fill_style_str("#ff6644");
                    self.ctx.fill_text("⚠", sx - 14.0, sy - 6.0).ok();
                }
                
                self.ctx.set_global_alpha(1.0);
            }
            
            // ========== 5. SYSTEM DETAIL POPUP ==========
            if let Some(sel_id) = selected_target {
                if let Some(sel_sys) = sector.systems.iter().find(|s| s.id == sel_id) {
                    let current_sys = &sector.systems[sector_map.current_system];
                    let dx = sel_sys.x - current_sys.x;
                    let dy = sel_sys.y - current_sys.y;
                    let fuel_cost = ((dx * dx + dy * dy).sqrt() * 10.0).ceil() as i32;
                    
                    // Position popup near selected system
                    let popup_x = cx + (sel_sys.x - 0.5) * scale * 2.0 + 20.0;
                    let popup_y = cy + (sel_sys.y - 0.5) * scale * 2.0 - 60.0;
                    
                    // Background — taller to fit more info
                    self.ctx.set_fill_style_str("rgba(10, 10, 20, 0.9)");
                    self.ctx.fill_rect(popup_x, popup_y, 200.0, 110.0);
                    self.ctx.set_stroke_style_str("#00ffff");
                    self.ctx.set_line_width(1.0);
                    self.ctx.stroke_rect(popup_x, popup_y, 200.0, 110.0);
                    
                    // Content
                    self.ctx.set_font("bold 11px monospace");
                    self.ctx.set_fill_style_str("#ffffff");
                    self.ctx.fill_text(sel_sys.name, popup_x + 5.0, popup_y + 15.0).ok();
                    
                    self.ctx.set_font("10px monospace");
                    self.ctx.set_fill_style_str("#aaaaaa");
                    self.ctx.fill_text(sel_sys.chinese_name, popup_x + 5.0, popup_y + 28.0).ok();
                    
                    let type_str = format!("Type: {:?}", sel_sys.location_type);
                    self.ctx.fill_text(&type_str, popup_x + 5.0, popup_y + 42.0).ok();
                    
                    // Features
                    let mut feats = Vec::new();
                    if sel_sys.has_shop { feats.push("💰Shop"); }
                    if sel_sys.has_fuel { feats.push("⛽Fuel"); }
                    if sel_sys.has_repair { feats.push("🔧Repair"); }
                    if sel_sys.has_medbay { feats.push("🏥Med"); }
                    if sel_sys.quest_giver { feats.push("❗Quest"); }
                    if !feats.is_empty() {
                        self.ctx.set_fill_style_str("#44dd44");
                        self.ctx.fill_text(&feats.join(" "), popup_x + 5.0, popup_y + 55.0).ok();
                    }
                    
                    // Hazard warning
                    if let Some(ref hazard) = sel_sys.hazard {
                        self.ctx.set_font("bold 10px monospace");
                        self.ctx.set_fill_style_str("#ff6644");
                        self.ctx.fill_text(&format!("⚠ {}", hazard.name()), popup_x + 5.0, popup_y + 68.0).ok();
                    }
                    
                    // Fuel cost
                    self.ctx.set_font("bold 11px monospace");
                    self.ctx.set_fill_style_str("#ffdd44");
                    self.ctx.fill_text(&format!("Fuel Cost: {}", fuel_cost), popup_x + 5.0, popup_y + 85.0).ok();
                }
            }
        }
    }

    pub fn draw_starmap_hud(&self, map: &SectorMap, ship: &Ship, cursor: usize) {
        self.ctx.set_text_align("left");
        
        // ========== TOP-LEFT: SHIP STATUS PANEL ==========
        // Panel background
        self.ctx.set_fill_style_str("rgba(10, 10, 20, 0.85)");
        self.ctx.fill_rect(10.0, 35.0, 240.0, 120.0);
        self.ctx.set_stroke_style_str("#00ccdd");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(10.0, 35.0, 240.0, 120.0);
        
        // Title
        self.ctx.set_font("bold 20px monospace");
        self.ctx.set_fill_style_str("#00ccdd");
        self.ctx.fill_text("★ STAR MAP", 20.0, 30.0).ok();
        
        // Hull bar
        self.ctx.set_font("12px monospace");
        self.ctx.set_fill_style_str("#cccccc");
        self.ctx.fill_text("HULL", 20.0, 55.0).ok();
        
        let hull_pct = ship.hull as f64 / ship.max_hull as f64;
        let hull_color = if hull_pct > 0.6 {
            "#44cc55"
        } else if hull_pct > 0.3 {
            "#cccc44"
        } else {
            "#cc4444"
        };
        
        // Bar background
        self.ctx.set_fill_style_str("#222222");
        self.ctx.fill_rect(20.0, 60.0, 220.0, 14.0);
        // Bar fill
        self.ctx.set_fill_style_str(hull_color);
        self.ctx.fill_rect(20.0, 60.0, 220.0 * hull_pct, 14.0);
        // Bar border
        self.ctx.set_stroke_style_str("#666666");
        self.ctx.set_line_width(1.0);
        self.ctx.stroke_rect(20.0, 60.0, 220.0, 14.0);
        // Value text
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_font("bold 10px monospace");
        self.ctx.fill_text(&format!("{}/{}", ship.hull, ship.max_hull), 25.0, 71.0).ok();
        
        // Fuel bar
        self.ctx.set_font("12px monospace");
        self.ctx.set_fill_style_str("#cccccc");
        self.ctx.fill_text("FUEL", 20.0, 90.0).ok();
        
        let fuel_pct = ship.fuel as f64 / ship.max_fuel as f64;
        self.ctx.set_fill_style_str("#222222");
        self.ctx.fill_rect(20.0, 95.0, 220.0, 14.0);
        self.ctx.set_fill_style_str("#4488ff");
        self.ctx.fill_rect(20.0, 95.0, 220.0 * fuel_pct, 14.0);
        self.ctx.set_stroke_style_str("#666666");
        self.ctx.stroke_rect(20.0, 95.0, 220.0, 14.0);
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_font("bold 10px monospace");
        self.ctx.fill_text(&format!("{}/{}", ship.fuel, ship.max_fuel), 25.0, 106.0).ok();
        
        // Shields bar
        self.ctx.set_font("12px monospace");
        self.ctx.set_fill_style_str("#cccccc");
        self.ctx.fill_text("SHIELDS", 20.0, 125.0).ok();
        
        let shield_pct = ship.shields as f64 / ship.max_shields as f64;
        self.ctx.set_fill_style_str("#222222");
        self.ctx.fill_rect(20.0, 130.0, 220.0, 14.0);
        self.ctx.set_fill_style_str("#00ccdd");
        self.ctx.fill_rect(20.0, 130.0, 220.0 * shield_pct, 14.0);
        self.ctx.set_stroke_style_str("#666666");
        self.ctx.stroke_rect(20.0, 130.0, 220.0, 14.0);
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_font("bold 10px monospace");
        self.ctx.fill_text(&format!("{}/{}", ship.shields, ship.max_shields), 25.0, 141.0).ok();
        
        // Cargo
        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#aaaaaa");
        self.ctx.fill_text(&format!("Cargo: {}/{}", ship.cargo_used, ship.cargo_capacity), 20.0, 152.0).ok();
        
        // ========== TOP-RIGHT: SECTOR INFO ==========
        if let Some(sector) = map.sectors.get(map.current_sector) {
            let panel_x = self.canvas_w - 260.0;
            
            // Panel background
            self.ctx.set_fill_style_str("rgba(10, 10, 20, 0.85)");
            self.ctx.fill_rect(panel_x, 35.0, 250.0, 80.0);
            self.ctx.set_stroke_style_str("#ffdd44");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(panel_x, 35.0, 250.0, 80.0);
            
            // Sector name
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_fill_style_str("#ffdd44");
            self.ctx.fill_text(&format!("SECTOR: {}", sector.name), panel_x + 10.0, 55.0).ok();
            
            self.ctx.set_font("12px monospace");
            self.ctx.set_fill_style_str("#cccccc");
            self.ctx.fill_text(&format!("HSK Level: {}", sector.hsk_level), panel_x + 10.0, 72.0).ok();
            
            // Systems explored count
            let visited_count = sector.systems.iter().filter(|s| s.visited).count();
            let total_count = sector.systems.len();
            self.ctx.fill_text(&format!("Explored: {}/{}", visited_count, total_count), panel_x + 10.0, 88.0).ok();
            
            // Sector description
            self.ctx.set_font("10px monospace");
            self.ctx.set_fill_style_str("#888888");
            self.ctx.fill_text(sector.description, panel_x + 10.0, 105.0).ok();
        }
        
        // ========== BOTTOM: NAVIGATION PANEL ==========
        if let Some(sector) = map.sectors.get(map.current_sector) {
            let sys = &sector.systems[map.current_system];
            let panel_y = self.canvas_h - 150.0;
            
            // Panel background
            self.ctx.set_fill_style_str("rgba(10, 10, 20, 0.90)");
            self.ctx.fill_rect(10.0, panel_y, self.canvas_w - 20.0, 125.0);
            self.ctx.set_stroke_style_str("#44dd88");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(10.0, panel_y, self.canvas_w - 20.0, 125.0);
            
            // Current system box
            self.ctx.set_font("bold 14px monospace");
            self.ctx.set_fill_style_str("#44dd88");
            self.ctx.fill_text("CURRENT LOCATION", 20.0, panel_y + 20.0).ok();
            
            self.ctx.set_font("bold 16px monospace");
            self.ctx.set_fill_style_str("#ffffff");
            self.ctx.fill_text(sys.name, 20.0, panel_y + 40.0).ok();
            
            self.ctx.set_font("12px monospace");
            self.ctx.set_fill_style_str("#aaaaaa");
            self.ctx.fill_text(sys.chinese_name, 20.0, panel_y + 56.0).ok();
            
            let type_str = format!("{:?}", sys.location_type);
            self.ctx.fill_text(&type_str, 20.0, panel_y + 72.0).ok();
            
            // System description
            self.ctx.set_font("10px monospace");
            self.ctx.set_fill_style_str("#777777");
            self.ctx.fill_text(sys.description, 200.0, panel_y + 40.0).ok();
            
            // Hazard warning
            if let Some(ref hazard) = sys.hazard {
                self.ctx.set_font("bold 11px monospace");
                self.ctx.set_fill_style_str("#ff6644");
                self.ctx.fill_text(&format!("⚠ {} {}", hazard.icon(), hazard.name()), 200.0, panel_y + 56.0).ok();
            }
            
            // Shop/Fuel/Repair/Medbay indicators
            let mut features = Vec::new();
            if sys.has_shop { features.push("💰Shop"); }
            if sys.has_fuel { features.push("⛽Fuel"); }
            if sys.has_repair { features.push("🔧Repair"); }
            if sys.has_medbay { features.push("🏥Medbay"); }
            if sys.quest_giver { features.push("❗Quest"); }
            if sys.warp_gate { features.push("🌀Warp"); }
            if !features.is_empty() {
                self.ctx.set_fill_style_str("#44dd44");
                self.ctx.fill_text(&format!("Available: {}", features.join(", ")), 20.0, panel_y + 88.0).ok();
            }
            
            // Jump targets
            let connections = &sys.connections;
            if !connections.is_empty() {
                self.ctx.set_font("bold 13px monospace");
                self.ctx.set_fill_style_str("#cccccc");
                self.ctx.fill_text("JUMP TARGETS:", 20.0, panel_y + 108.0).ok();
                
                self.ctx.set_font("11px monospace");
                self.ctx.set_fill_style_str("#666666");
                self.ctx.fill_text("(Use ←/→ to select, Enter to jump, E to explore)", 180.0, panel_y + 108.0).ok();
                
                // Draw targets horizontally with fuel costs
                let mut x_pos = 20.0;
                for (i, &conn_id) in connections.iter().enumerate() {
                    if let Some(target) = sector.systems.iter().find(|s| s.id == conn_id) {
                        let is_selected = i == cursor % connections.len();
                        
                        // Calculate fuel cost
                        let dx = target.x - sys.x;
                        let dy = target.y - sys.y;
                        let fuel_cost = ((dx * dx + dy * dy).sqrt() * 10.0).ceil() as i32;
                        let can_afford = ship.fuel >= fuel_cost;
                        
                        if is_selected {
                            self.ctx.set_fill_style_str("#00ffff");
                            self.ctx.set_font("bold 12px monospace");
                            self.ctx.fill_text("▶", x_pos, panel_y + 125.0).ok();
                            self.ctx.fill_text(&format!("{} ", target.name), x_pos + 12.0, panel_y + 125.0).ok();
                            
                            // Fuel cost indicator
                            let cost_color = if can_afford { "#ffdd44" } else { "#ff4444" };
                            self.ctx.set_fill_style_str(cost_color);
                            self.ctx.set_font("bold 11px monospace");
                            self.ctx.fill_text(&format!("[Fuel: {}]", fuel_cost), x_pos + 12.0 + (target.name.len() as f64 * 7.5), panel_y + 125.0).ok();
                            
                            x_pos += (target.name.len() as f64 + 12.0) * 8.0;
                        } else {
                            self.ctx.set_fill_style_str("#666666");
                            self.ctx.set_font("11px monospace");
                            self.ctx.fill_text(&format!("{}  ", target.name), x_pos, panel_y + 125.0).ok();
                            x_pos += (target.name.len() as f64 + 2.0) * 7.0;
                        }
                    }
                }
            }
        }
        
        // ========== BOTTOM-RIGHT: LEGEND ==========
        let legend_x = self.canvas_w - 210.0;
        let legend_y = self.canvas_h - 150.0;
        
        self.ctx.set_fill_style_str("rgba(10, 10, 20, 0.85)");
        self.ctx.fill_rect(legend_x, legend_y, 200.0, 125.0);
        self.ctx.set_stroke_style_str("#888888");
        self.ctx.set_line_width(1.0);
        self.ctx.stroke_rect(legend_x, legend_y, 200.0, 125.0);
        
        self.ctx.set_font("bold 11px monospace");
        self.ctx.set_fill_style_str("#cccccc");
        self.ctx.fill_text("LEGEND", legend_x + 5.0, legend_y + 15.0).ok();
        
        let legend_items = [
            ("#ffdd44", "Space Station"),
            ("#cc7733", "Asteroid Base"),
            ("#8844cc", "Derelict Ship"),
            ("#44ccaa", "Alien Ruins"),
            ("#44dd44", "Trading Post"),
            ("#4488ff", "Orbital Platform"),
            ("#cc8844", "Mining Colony"),
            ("#ff88cc", "Research Lab"),
        ];
        
        self.ctx.set_font("10px monospace");
        for (i, (color, name)) in legend_items.iter().enumerate() {
            let y = legend_y + 30.0 + i as f64 * 13.0;
            
            // Color swatch
            self.ctx.set_fill_style_str(color);
            self.ctx.fill_rect(legend_x + 5.0, y - 8.0, 10.0, 10.0);
            
            // Name
            self.ctx.set_fill_style_str("#aaaaaa");
            self.ctx.fill_text(name, legend_x + 20.0, y).ok();
        }
        
        // ========== BOTTOM: CONTROLS REMINDER ==========
        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#444444");
        self.ctx.fill_text("[M] Map  [E] Explore Current  [Enter] Jump  [←/→] Select Target", 20.0, self.canvas_h - 5.0).ok();
    }

    /// Draw class selection overlay on top of the starmap.
    pub fn draw_class_select(&self, cursor: usize, has_continue: bool) {
        let anim_t = Date::now() / 1000.0;

        self.ctx.set_fill_style_str("rgba(0,0,0,0.85)");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

        let cx = self.canvas_w / 2.0;
        let mut y = 40.0 + (anim_t * 2.1).sin() * 4.0;

        self.ctx.set_fill_style_str("#00ccdd");
        self.ctx.set_font("32px monospace");
        self.ctx.set_text_align("center");
        self.ctx.fill_text("选择你的道路", cx, y).ok();
        y += 30.0;
        self.ctx.set_fill_style_str("#888");
        self.ctx.set_font("14px monospace");
        self.ctx.fill_text("Choose Your Path", cx, y).ok();
        y += 40.0;

        // "Continue Previous Game" option
        if has_continue {
            let is_selected = cursor == 0;
            let bg_color = if is_selected {
                "rgba(255,200,0,0.20)"
            } else {
                "rgba(0,0,0,0.4)"
            };
            let border_color = if is_selected { "#ffcc00" } else { "#444" };

            self.ctx.set_fill_style_str(bg_color);
            self.ctx.set_stroke_style_str(border_color);
            self.ctx.set_line_width(if is_selected { 2.0 } else { 1.0 });

            let box_w = 400.0;
            let box_h = 50.0;
            let box_x = cx - box_w / 2.0;

            self.ctx.fill_rect(box_x, y, box_w, box_h);
            self.ctx.stroke_rect(box_x, y, box_w, box_h);

            self.ctx.set_fill_style_str("#ffcc00");
            self.ctx.set_font("20px monospace");
            self.ctx.set_text_align("left");
            self.ctx.fill_text("▸", box_x + 15.0, y + 32.0).ok();

            self.ctx
                .set_fill_style_str(if is_selected { "#ffe066" } else { "#ccaa00" });
            self.ctx.set_font("16px monospace");
            self.ctx
                .fill_text("Continue Previous Game", box_x + 45.0, y + 32.0)
                .ok();

            y += box_h + 16.0;
        }

        let continue_offset = if has_continue { 1 } else { 0 };
        let class_cursor = cursor.saturating_sub(continue_offset);

        let all_classes = PlayerClass::all();
        let total = all_classes.len();

        let page_size = 6;
        let page = class_cursor / page_size;
        let start_idx = page * page_size;
        let end_idx = (start_idx + page_size).min(total);

        for i in start_idx..end_idx {
            let class_var = all_classes[i];
            let data = class_var.data();

            let is_selected = i + continue_offset == cursor;
            let bg_color = if is_selected {
                "rgba(255,255,255,0.15)"
            } else {
                "rgba(0,0,0,0.4)"
            };
            let border_color = if is_selected { data.color } else { "#444" };

            self.ctx.set_fill_style_str(bg_color);
            self.ctx.set_stroke_style_str(border_color);
            self.ctx.set_line_width(if is_selected { 2.0 } else { 1.0 });

            let box_w = 400.0;
            let box_h = 50.0;
            let box_x = cx - box_w / 2.0;

            self.ctx.fill_rect(box_x, y, box_w, box_h);
            self.ctx.stroke_rect(box_x, y, box_w, box_h);

            self.ctx.set_fill_style_str(data.color);
            self.ctx.set_font("20px monospace");
            self.ctx.set_text_align("left");
            self.ctx.fill_text(data.icon, box_x + 15.0, y + 32.0).ok();

            self.ctx
                .set_fill_style_str(if is_selected { "#fff" } else { "#ccc" });
            self.ctx.set_font("16px monospace");
            self.ctx
                .fill_text(
                    &format!("{} {}", data.name_cn, data.name_en),
                    box_x + 45.0,
                    y + 22.0,
                )
                .ok();

            let dummy = Player::new(0, 0, class_var);
            self.ctx.set_fill_style_str("#aaa");
            self.ctx.set_font("12px monospace");
            self.ctx
                .fill_text(
                    &format!("HP:{} Items:{}", dummy.max_hp, dummy.max_items()),
                    box_x + 280.0,
                    y + 22.0,
                )
                .ok();

            self.ctx.set_fill_style_str(data.color);
            self.ctx.set_font("12px monospace");
            self.ctx.fill_text(data.lore, box_x + 45.0, y + 40.0).ok();

            y += box_h + 10.0;
        }

        y += 10.0;
        let total_pages = (total + page_size - 1) / page_size;
        self.ctx.set_fill_style_str("#888");
        self.ctx.set_text_align("center");
        self.ctx
            .fill_text(
                &format!(
                    "Page {}/{} (↑/↓ to scroll, Enter to select)",
                    page + 1,
                    total_pages
                ),
                cx,
                y,
            )
            .ok();
        self.ctx.set_text_align("left");
    }

    pub fn draw_ship_interior(
        &self,
        layout: &ShipLayout,
        ship_x: i32,
        ship_y: i32,
        crew: &[crate::player::CrewMember],
        ship: &Ship,
        message: &str,
        player_hp: i32,
        player_max_hp: i32,
        player_gold: i32,
        show_help: bool,
    ) {
        // Clear
        self.ctx.set_fill_style_str("#0a0a14");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);

        let tx_size = 28.0;
        let hud_top = 60.0;
        let hud_bottom = 80.0;
        let map_area_h = self.canvas_h - hud_top - hud_bottom;
        let offset_x = (self.canvas_w - layout.width as f64 * tx_size) / 2.0;
        let offset_y = hud_top + (map_area_h - layout.height as f64 * tx_size) / 2.0;

        // ── Tile map ────────────────────────────────────────────────────
        for (i, tile) in layout.tiles.iter().enumerate() {
            let x = (i as i32 % layout.width) as f64;
            let y = (i as i32 / layout.width) as f64;
            let screen_x = offset_x + x * tx_size;
            let screen_y = offset_y + y * tx_size;

            let color = match tile {
                ShipTile::Floor => "#1a1a2e",
                ShipTile::Wall => "#2a2a3e",
                ShipTile::Door => "#3a3a55",
                ShipTile::Console(_) => "#0a2a3a",
                ShipTile::CrewStation(_) => "#0a2a2a",
                ShipTile::Decoration(_) => "#1e1e30",
                ShipTile::Empty => continue,
            };

            self.ctx.set_fill_style_str(color);
            self.ctx.fill_rect(screen_x, screen_y, tx_size, tx_size);

            // Grid lines
            self.ctx.set_stroke_style_str("#1a1a2a");
            self.ctx.stroke_rect(screen_x, screen_y, tx_size, tx_size);

            // Console icons
            if let ShipTile::Console(room) = tile {
                let icon = match room {
                    ShipRoom::Bridge => "\u{2316}",       // ⌖ (position indicator)
                    ShipRoom::EngineRoom => "\u{2699}",   // ⚙
                    ShipRoom::QuantumForge => "\u{2692}",  // ⚒
                    ShipRoom::CrewQuarters => "\u{263A}",  // ☺
                    ShipRoom::CargoBay => "\u{25A3}",     // ▣
                    ShipRoom::Medbay => "+",
                    ShipRoom::WeaponsBay => "\u{2694}",   // ⚔
                    ShipRoom::Airlock => "\u{25CE}",      // ◎
                    ShipRoom::Corridor => " ",
                };
                self.ctx.set_font(&format!("{}px monospace", (tx_size * 0.6) as i32));
                self.ctx.set_fill_style_str("#00ccdd");
                self.ctx.set_text_align("center");
                self.ctx.set_text_baseline("middle");
                self.ctx.fill_text(icon, screen_x + tx_size / 2.0, screen_y + tx_size / 2.0).ok();
                // Glow border
                self.ctx.set_stroke_style_str("rgba(0, 200, 220, 0.4)");
                self.ctx.set_line_width(1.5);
                self.ctx.stroke_rect(screen_x + 1.0, screen_y + 1.0, tx_size - 2.0, tx_size - 2.0);
                self.ctx.set_line_width(1.0);
            }

            // Crew station indicators
            if let ShipTile::CrewStation(idx) = tile {
                let label = if (*idx) < crew.len() {
                    crew[*idx].role.icon()
                } else {
                    "\u{25CB}" // ○ empty station
                };
                self.ctx.set_font(&format!("{}px monospace", (tx_size * 0.55) as i32));
                self.ctx.set_fill_style_str("#00aa88");
                self.ctx.set_text_align("center");
                self.ctx.set_text_baseline("middle");
                self.ctx.fill_text(label, screen_x + tx_size / 2.0, screen_y + tx_size / 2.0).ok();
            }

            // Door markers
            if *tile == ShipTile::Door {
                self.ctx.set_stroke_style_str("#556677");
                self.ctx.set_line_width(2.0);
                self.ctx.stroke_rect(screen_x + 2.0, screen_y + 2.0, tx_size - 4.0, tx_size - 4.0);
                self.ctx.set_line_width(1.0);
            }
        }

        // ── Room labels ─────────────────────────────────────────────────
        self.ctx.set_font("11px monospace");
        self.ctx.set_text_align("center");
        self.ctx.set_text_baseline("top");
        for (lx, ly, room) in &layout.room_labels {
            if *room == ShipRoom::Corridor { continue; }
            let sx = offset_x + *lx as f64 * tx_size + tx_size / 2.0;
            let sy = offset_y + *ly as f64 * tx_size + 2.0;
            // Shadow
            self.ctx.set_fill_style_str("rgba(0,0,0,0.7)");
            self.ctx.fill_text(room.name(), sx + 1.0, sy + 1.0).ok();
            // Text
            self.ctx.set_fill_style_str("#8888aa");
            self.ctx.fill_text(room.name(), sx, sy).ok();
        }

        // ── Player ──────────────────────────────────────────────────────
        let px = offset_x + ship_x as f64 * tx_size;
        let py = offset_y + ship_y as f64 * tx_size;

        // Glow
        self.ctx.set_fill_style_str("rgba(0, 200, 220, 0.15)");
        self.ctx.begin_path();
        self.ctx.arc(px + tx_size / 2.0, py + tx_size / 2.0, tx_size * 0.7, 0.0, std::f64::consts::TAU).ok();
        self.ctx.fill();

        // Player dot
        self.ctx.set_fill_style_str(COL_PLAYER);
        self.ctx.begin_path();
        self.ctx.arc(px + tx_size / 2.0, py + tx_size / 2.0, tx_size / 3.0, 0.0, std::f64::consts::TAU).ok();
        self.ctx.fill();

        // ── Top HUD ─────────────────────────────────────────────────────
        self.ctx.set_fill_style_str("rgba(10, 10, 20, 0.9)");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, hud_top);
        self.ctx.set_stroke_style_str("#333355");
        let _ = self.ctx.begin_path();
        self.ctx.move_to(0.0, hud_top);
        self.ctx.line_to(self.canvas_w, hud_top);
        self.ctx.stroke();

        // Current room name + description
        let current_room = get_room_at(layout, ship_x, ship_y);
        self.ctx.set_font("16px monospace");
        self.ctx.set_fill_style_str("#00eeff");
        self.ctx.set_text_align("left");
        self.ctx.set_text_baseline("top");
        self.ctx.fill_text(&format!("\u{25C6} {}", current_room.name()), 12.0, 8.0).ok();

        self.ctx.set_font("12px monospace");
        self.ctx.set_fill_style_str("#6666aa");
        self.ctx.fill_text(current_room.description(), 12.0, 28.0).ok();

        // Ship stats (right side)
        self.ctx.set_text_align("right");
        self.ctx.set_font("12px monospace");
        let rx = self.canvas_w - 12.0;

        self.ctx.set_fill_style_str("#cc4444");
        self.ctx.fill_text(&format!("HP {}/{}", player_hp, player_max_hp), rx, 6.0).ok();

        self.ctx.set_fill_style_str("#4488cc");
        self.ctx.fill_text(&format!("Hull {}/{}", ship.hull, ship.max_hull), rx, 20.0).ok();

        self.ctx.set_fill_style_str("#44cc88");
        self.ctx.fill_text(&format!("Shields {}/{}", ship.shields, ship.max_shields), rx, 34.0).ok();

        self.ctx.set_fill_style_str("#ccaa44");
        self.ctx.fill_text(&format!("{}g", player_gold), rx, 48.0).ok();

        // ── Bottom bar ──────────────────────────────────────────────────
        let bar_y = self.canvas_h - hud_bottom;
        self.ctx.set_fill_style_str("rgba(10, 10, 20, 0.9)");
        self.ctx.fill_rect(0.0, bar_y, self.canvas_w, hud_bottom);
        self.ctx.set_stroke_style_str("#333355");
        let _ = self.ctx.begin_path();
        self.ctx.move_to(0.0, bar_y);
        self.ctx.line_to(self.canvas_w, bar_y);
        self.ctx.stroke();

        // Message
        if !message.is_empty() {
            self.ctx.set_font("14px monospace");
            self.ctx.set_fill_style_str("#ddddff");
            self.ctx.set_text_align("left");
            self.ctx.fill_text(message, 12.0, bar_y + 14.0).ok();
        }

        // Controls hint
        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#555577");
        self.ctx.set_text_align("center");
        self.ctx.fill_text(
            "[E] Interact  [M] Star Map  [?] Help  [Esc] Exit",
            self.canvas_w / 2.0,
            bar_y + hud_bottom - 16.0,
        ).ok();

        // Crew count
        if !crew.is_empty() {
            self.ctx.set_text_align("right");
            self.ctx.set_fill_style_str("#00aa88");
            self.ctx.fill_text(
                &format!("Crew: {}", crew.len()),
                self.canvas_w - 12.0,
                bar_y + 14.0,
            ).ok();
        }

        // Reset text baseline
        self.ctx.set_text_align("left");
        self.ctx.set_text_baseline("alphabetic");

        // ── Help overlay ────────────────────────────────────────────────
        if show_help {
            self.draw_ship_help();
        }
    }

    fn draw_ship_help(&self) {
        let panel_w = 420.0_f64;
        let panel_h = 340.0_f64;
        let px = (self.canvas_w - panel_w) / 2.0;
        let py = (self.canvas_h - panel_h) / 2.0;

        // Overlay
        self.ctx.set_fill_style_str("rgba(0, 0, 0, 0.92)");
        self.ctx.fill_rect(px, py, panel_w, panel_h);
        self.ctx.set_stroke_style_str("#00ccdd");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(px, py, panel_w, panel_h);
        self.ctx.set_line_width(1.0);

        // Title
        self.ctx.set_font("18px monospace");
        self.ctx.set_fill_style_str("#00eeff");
        self.ctx.set_text_align("center");
        self.ctx.fill_text("Ship Controls", px + panel_w / 2.0, py + 28.0).ok();

        // Controls list
        let controls = [
            ("Arrows / WASD", "Move around the ship"),
            ("E", "Interact with adjacent console or crew"),
            ("M", "Open star map"),
            ("Esc", "Return to star map"),
            ("?", "Toggle this help"),
            ("", ""),
            ("", "--- Consoles ---"),
            ("Bridge", "Access navigation / star map"),
            ("Medbay", "Heal 10 HP"),
            ("Engine Room", "Repair 5 hull points"),
            ("Weapons Bay", "Calibrate weapons (+1 power)"),
            ("Cargo Bay", "Browse ship upgrades"),
            ("Crew Quarters", "Rest and recover full HP"),
            ("Quantum Forge", "Crafting info"),
            ("Airlock", "Exit ship to explore location"),
        ];

        self.ctx.set_text_align("left");
        self.ctx.set_font("13px monospace");
        let mut row_y = py + 54.0;
        for (key, desc) in &controls {
            if key.is_empty() && desc.starts_with("---") {
                // Section divider
                self.ctx.set_fill_style_str("#555577");
                self.ctx.fill_text(desc, px + panel_w / 2.0 - 50.0, row_y).ok();
            } else if key.is_empty() {
                // Spacer
            } else {
                self.ctx.set_fill_style_str("#00ccdd");
                self.ctx.fill_text(key, px + 24.0, row_y).ok();
                self.ctx.set_fill_style_str("#aaaacc");
                self.ctx.fill_text(desc, px + 180.0, row_y).ok();
            }
            row_y += 18.0;
        }

        // Footer
        self.ctx.set_font("11px monospace");
        self.ctx.set_fill_style_str("#555577");
        self.ctx.set_text_align("center");
        self.ctx.fill_text("Press ? or Esc to close", px + panel_w / 2.0, py + panel_h - 14.0).ok();
        self.ctx.set_text_align("left");
    }

    pub fn draw_ship_upgrades(
        &self,
        cursor: usize,
        purchased: &[crate::world::ship::ShipUpgrade],
        gold: i32,
    ) {
        use crate::world::ship::ShipUpgrade;

        let all = ShipUpgrade::all();
        let panel_w = 500.0_f64;
        let panel_h = 40.0 * all.len() as f64 + 100.0;
        let px = (self.canvas_w - panel_w) / 2.0;
        let py = (self.canvas_h - panel_h) / 2.0;

        // Overlay background
        self.ctx.set_fill_style_str("rgba(0, 0, 0, 0.92)");
        self.ctx.fill_rect(px, py, panel_w, panel_h);
        self.ctx.set_stroke_style_str("#00ccdd");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(px, py, panel_w, panel_h);

        // Title
        self.ctx.set_font("20px monospace");
        self.ctx.set_fill_style_str("#00eeff");
        self.ctx.set_text_align("center");
        self.ctx.fill_text("Ship Upgrades", px + panel_w / 2.0, py + 30.0).ok();

        // Gold display
        self.ctx.set_font("14px monospace");
        self.ctx.set_fill_style_str("#ffcc00");
        self.ctx.fill_text(&format!("Credits: {}", gold), px + panel_w / 2.0, py + 52.0).ok();
        self.ctx.set_text_align("left");

        // List upgrades
        let list_y = py + 70.0;
        for (i, upgrade) in all.iter().enumerate() {
            let row_y = list_y + i as f64 * 40.0;
            let owned = purchased.contains(upgrade);

            // Highlight cursor
            if i == cursor {
                self.ctx.set_fill_style_str("rgba(0, 200, 220, 0.15)");
                self.ctx.fill_rect(px + 10.0, row_y - 14.0, panel_w - 20.0, 36.0);
            }

            // Name + status
            self.ctx.set_font("14px monospace");
            let prefix = if owned { "\u{2713} " } else if i == cursor { "> " } else { "  " };
            let name_color = if owned { "#55aa55" } else if gold >= upgrade.cost() { "#ffffff" } else { "#aa5555" };
            self.ctx.set_fill_style_str(name_color);
            self.ctx.fill_text(&format!("{}{}", prefix, upgrade.name()), px + 20.0, row_y + 4.0).ok();

            // Description
            self.ctx.set_fill_style_str("#888888");
            self.ctx.set_font("12px monospace");
            self.ctx.fill_text(upgrade.description(), px + 240.0, row_y + 4.0).ok();

            // Cost
            let cost_str = if owned { "OWNED".to_string() } else { format!("{}c", upgrade.cost()) };
            self.ctx.set_fill_style_str(if owned { "#55aa55" } else { "#ffcc00" });
            self.ctx.fill_text(&cost_str, px + panel_w - 70.0, row_y + 4.0).ok();
        }

        // Footer
        self.ctx.set_font("12px monospace");
        self.ctx.set_fill_style_str("#666666");
        self.ctx.set_text_align("center");
        self.ctx.fill_text("[Enter] Buy  [Esc] Close", px + panel_w / 2.0, py + panel_h - 12.0).ok();
        self.ctx.set_text_align("left");
    }

    pub fn draw_space_combat(
        &self,
        player_ship: &Ship,
        enemy_ship: &EnemyShip,
        phase: &SpaceCombatPhase,
        cursor: usize,
        target_cursor: usize,
        log: &[String],
        crew: &[crate::player::CrewMember],
        weapon: ShipWeapon,
        evading: bool,
        anim_t: f64,
    ) {
        let w = self.canvas_w;
        let h = self.canvas_h;

        // ── Background: dark space with stars ──
        self.ctx.set_fill_style_str("#050510");
        self.ctx.fill_rect(0.0, 0.0, w, h);

        let star_seed = (anim_t * 0.1) as u64;
        let mut rng = star_seed.wrapping_mul(1664525).wrapping_add(1013904223);
        for _ in 0..60 {
            rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
            let sx = (rng % (w as u64)) as f64;
            rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
            let sy = (rng % (h as u64)) as f64;
            rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
            let brightness = 0.3 + (rng % 70) as f64 / 100.0;
            let b = (brightness * 255.0) as u8;
            self.ctx.set_fill_style_str(&format!("rgb({},{},{})", b, b, b));
            self.ctx.fill_rect(sx, sy, 2.0, 2.0);
        }

        // ── Player ship (left) ──
        let ps_x = 80.0;
        let ps_y = h * 0.30;
        self.ctx.set_fill_style_str("#00ccdd");
        self.ctx.begin_path();
        self.ctx.move_to(ps_x + 120.0, ps_y + 40.0);
        self.ctx.line_to(ps_x, ps_y);
        self.ctx.line_to(ps_x, ps_y + 80.0);
        self.ctx.close_path();
        self.ctx.fill();
        // Engine glow
        let glow_alpha = 0.5 + 0.3 * (anim_t * 3.0).sin();
        self.ctx.set_fill_style_str(&format!("rgba(0,200,255,{:.2})", glow_alpha));
        self.ctx.fill_rect(ps_x - 18.0, ps_y + 20.0, 18.0, 40.0);
        if evading {
            self.ctx.set_fill_style_str("rgba(0,255,200,0.3)");
            self.ctx.fill_rect(ps_x - 5.0, ps_y - 5.0, 130.0, 90.0);
        }

        // ── Enemy ship (right) ──
        let es_x = w - 200.0;
        let es_y = h * 0.30;
        self.ctx.set_fill_style_str("#ff5555");
        self.ctx.begin_path();
        self.ctx.move_to(es_x, es_y + 40.0);
        self.ctx.line_to(es_x + 120.0, es_y);
        self.ctx.line_to(es_x + 120.0, es_y + 80.0);
        self.ctx.close_path();
        self.ctx.fill();
        // Enemy engine glow
        self.ctx.set_fill_style_str(&format!("rgba(255,100,50,{:.2})", glow_alpha));
        self.ctx.fill_rect(es_x + 120.0, es_y + 20.0, 18.0, 40.0);

        // ── Player stats (top-left panel) ──
        let panel_x = 15.0;
        let panel_y = 15.0;
        self.ctx.set_fill_style_str("rgba(0,10,30,0.85)");
        self.ctx.fill_rect(panel_x, panel_y, 230.0, 100.0);
        self.ctx.set_stroke_style_str("#00ccdd");
        self.ctx.set_line_width(1.0);
        self.ctx.stroke_rect(panel_x, panel_y, 230.0, 100.0);

        self.ctx.set_font("14px monospace");
        self.ctx.set_fill_style_str("#00ccdd");
        self.ctx.fill_text("YOUR SHIP", panel_x + 5.0, panel_y + 16.0).ok();

        // Hull bar
        self.draw_bar(panel_x + 5.0, panel_y + 22.0, 218.0, 14.0,
            player_ship.hull as f64 / player_ship.max_hull.max(1) as f64,
            &format!("Hull: {}/{}", player_ship.hull.max(0), player_ship.max_hull),
            true);

        // Shield bar
        self.draw_bar(panel_x + 5.0, panel_y + 40.0, 218.0, 14.0,
            player_ship.shields as f64 / player_ship.max_shields.max(1) as f64,
            &format!("Shields: {}/{}", player_ship.shields.max(0), player_ship.max_shields),
            false);

        // Weapon / Engine power
        self.ctx.set_font("12px monospace");
        self.ctx.set_fill_style_str("#cccccc");
        self.ctx.fill_text(&format!("Wpn: {}  Eng: {}", player_ship.weapon_power, player_ship.engine_power),
            panel_x + 5.0, panel_y + 72.0).ok();
        self.ctx.fill_text(&format!("Crew: {}", crew.len()), panel_x + 5.0, panel_y + 88.0).ok();

        // ── Enemy stats (top-right panel) ──
        let ep_x = w - 250.0;
        let ep_y = 15.0;
        self.ctx.set_fill_style_str("rgba(30,0,0,0.85)");
        self.ctx.fill_rect(ep_x, ep_y, 235.0, 140.0);
        self.ctx.set_stroke_style_str("#ff5555");
        self.ctx.set_line_width(1.0);
        self.ctx.stroke_rect(ep_x, ep_y, 235.0, 140.0);

        self.ctx.set_font("14px monospace");
        self.ctx.set_fill_style_str("#ff5555");
        self.ctx.fill_text(&enemy_ship.name, ep_x + 5.0, ep_y + 16.0).ok();

        // Enemy Hull bar
        self.draw_bar(ep_x + 5.0, ep_y + 22.0, 223.0, 14.0,
            enemy_ship.hull as f64 / enemy_ship.max_hull.max(1) as f64,
            &format!("Hull: {}/{}", enemy_ship.hull.max(0), enemy_ship.max_hull),
            true);

        // Enemy Shield bar
        self.draw_bar(ep_x + 5.0, ep_y + 40.0, 223.0, 14.0,
            enemy_ship.shields as f64 / enemy_ship.max_shields.max(1) as f64,
            &format!("Shields: {}/{}", enemy_ship.shields.max(0), enemy_ship.max_shields),
            false);

        // Subsystem mini-bars
        let subs = [
            ("Wpn", &enemy_ship.weapons_sub),
            ("Shd", &enemy_ship.shields_sub),
            ("Eng", &enemy_ship.engines_sub),
        ];
        self.ctx.set_font("11px monospace");
        for (i, (label, sub)) in subs.iter().enumerate() {
            let sy = ep_y + 60.0 + i as f64 * 20.0;
            if sub.is_destroyed() {
                self.ctx.set_fill_style_str("#ff2222");
                self.ctx.fill_text(&format!("{}: DESTROYED", label), ep_x + 5.0, sy + 12.0).ok();
            } else {
                self.ctx.set_fill_style_str("#888888");
                self.ctx.fill_text(&format!("{}:", label), ep_x + 5.0, sy + 12.0).ok();
                self.ctx.set_fill_style_str("#333333");
                self.ctx.fill_rect(ep_x + 40.0, sy, 140.0, 14.0);
                let pct = sub.pct().max(0.0);
                let color = if pct > 0.5 { "#44cc44" } else if pct > 0.25 { "#ccaa00" } else { "#cc4444" };
                self.ctx.set_fill_style_str(color);
                self.ctx.fill_rect(ep_x + 40.0, sy, 140.0 * pct, 14.0);
                self.ctx.set_fill_style_str("#ffffff");
                self.ctx.set_font("10px monospace");
                self.ctx.fill_text(&format!("{}/{}", sub.hp.max(0), sub.max_hp), ep_x + 185.0, sy + 11.0).ok();
                self.ctx.set_font("11px monospace");
            }
        }

        // Enemy tactic indicator
        self.ctx.set_fill_style_str("#777777");
        self.ctx.set_font("10px monospace");
        let tactic_str = match enemy_ship.tactic {
            crate::game::EnemyTactic::Aggressive => "Tactic: Aggressive",
            crate::game::EnemyTactic::Disabling  => "Tactic: Disabling",
            crate::game::EnemyTactic::Balanced   => "Tactic: Balanced",
            crate::game::EnemyTactic::Boarding   => "Tactic: Boarding",
        };
        self.ctx.fill_text(tactic_str, ep_x + 5.0, ep_y + 132.0).ok();

        // ── Battle log (center, last 6 messages) ──
        self.ctx.set_font("13px monospace");
        let log_start = if log.len() > 6 { log.len() - 6 } else { 0 };
        let log_y_base = h * 0.52;
        for (i, msg) in log[log_start..].iter().enumerate() {
            let alpha = 0.4 + 0.1 * i as f64;
            self.ctx.set_fill_style_str(&format!("rgba(200,200,200,{:.2})", alpha.min(1.0)));
            let text_x = (w / 2.0 - 200.0).max(20.0);
            self.ctx.fill_text(msg, text_x, log_y_base + i as f64 * 20.0).ok();
        }

        // ── Crew bonuses (bottom-left) ──
        self.ctx.set_font("10px monospace");
        let mut bonus_y = h - 120.0;
        let mut has_bonus = false;
        let role_bonuses: &[(&str, &str)] = &[
            ("Pilot", "+20% evasion"),
            ("Engineer", "+2 shields/turn"),
            ("Gunner", "+2 weapon dmg"),
            ("Medic", "+1 crew HP post-battle"),
            ("Scientist", "sensor boost"),
            ("QM", "+25% loot"),
        ];
        let role_checks: &[crate::player::CrewRole] = &[
            crate::player::CrewRole::Pilot,
            crate::player::CrewRole::Engineer,
            crate::player::CrewRole::SecurityChief,
            crate::player::CrewRole::Medic,
            crate::player::CrewRole::ScienceOfficer,
            crate::player::CrewRole::Quartermaster,
        ];
        for (i, role) in role_checks.iter().enumerate() {
            if crew.iter().any(|c| c.role == *role) {
                if !has_bonus {
                    self.ctx.set_fill_style_str("#00aacc");
                    self.ctx.fill_text("Crew Bonuses:", 15.0, bonus_y).ok();
                    bonus_y += 14.0;
                    has_bonus = true;
                }
                self.ctx.set_fill_style_str("#66ccaa");
                self.ctx.fill_text(&format!(" {} {}", role_bonuses[i].0, role_bonuses[i].1), 15.0, bonus_y).ok();
                bonus_y += 13.0;
            }
        }

        // ── Action menu (bottom) ──
        match phase {
            SpaceCombatPhase::Choosing => {
                // Two rows of 4 buttons
                let actions_r1 = ["-- Laser", "=> Missiles", "~~ Ion Cannon", "== Broadside"];
                let actions_r2 = ["Shields", "Evade", "Board", "Flee"];
                let btn_w = 130.0;
                let btn_h = 36.0;
                let gap = 8.0;
                let total_row_w = 4.0 * btn_w + 3.0 * gap;
                let start_x = (w - total_row_w) / 2.0;
                let row1_y = h - 90.0;
                let row2_y = h - 48.0;

                self.ctx.set_font("13px monospace");
                for (i, action) in actions_r1.iter().enumerate() {
                    let bx = start_x + i as f64 * (btn_w + gap);
                    if i == cursor && cursor < 4 {
                        self.ctx.set_fill_style_str("#00ccdd");
                        self.ctx.fill_rect(bx, row1_y, btn_w, btn_h);
                        self.ctx.set_fill_style_str("#000000");
                    } else {
                        self.ctx.set_fill_style_str("#1a1a2e");
                        self.ctx.fill_rect(bx, row1_y, btn_w, btn_h);
                        self.ctx.set_stroke_style_str("#555577");
                        self.ctx.stroke_rect(bx, row1_y, btn_w, btn_h);
                        self.ctx.set_fill_style_str("#cccccc");
                    }
                    self.ctx.fill_text(action, bx + 6.0, row1_y + 24.0).ok();
                }
                for (i, action) in actions_r2.iter().enumerate() {
                    let bx = start_x + i as f64 * (btn_w + gap);
                    let ci = i + 4;
                    if ci == cursor {
                        self.ctx.set_fill_style_str("#00ccdd");
                        self.ctx.fill_rect(bx, row2_y, btn_w, btn_h);
                        self.ctx.set_fill_style_str("#000000");
                    } else {
                        self.ctx.set_fill_style_str("#1a1a2e");
                        self.ctx.fill_rect(bx, row2_y, btn_w, btn_h);
                        self.ctx.set_stroke_style_str("#555577");
                        self.ctx.stroke_rect(bx, row2_y, btn_w, btn_h);
                        self.ctx.set_fill_style_str("#cccccc");
                    }
                    self.ctx.fill_text(action, bx + 6.0, row2_y + 24.0).ok();
                }

                // Weapon description hint
                if cursor < 4 {
                    let weapons = [ShipWeapon::Laser, ShipWeapon::Missiles, ShipWeapon::IonCannon, ShipWeapon::Broadside];
                    self.ctx.set_font("11px monospace");
                    self.ctx.set_fill_style_str("#888888");
                    self.ctx.fill_text(weapons[cursor].description(), start_x, h - 4.0).ok();
                }
            }
            SpaceCombatPhase::TargetingSubsystem => {
                // Targeting UI — show weapon name and subsystem list
                let panel_w = 260.0;
                let panel_h = 140.0;
                let px = w - panel_w - 30.0;
                let py = h * 0.45;

                self.ctx.set_fill_style_str("rgba(0,20,40,0.92)");
                self.ctx.fill_rect(px, py, panel_w, panel_h);
                self.ctx.set_stroke_style_str("#00ccdd");
                self.ctx.set_line_width(1.5);
                self.ctx.stroke_rect(px, py, panel_w, panel_h);

                self.ctx.set_font("14px monospace");
                self.ctx.set_fill_style_str("#00ccdd");
                self.ctx.fill_text(&format!("{} {} - Select Target", weapon.icon(), weapon.name()), px + 8.0, py + 18.0).ok();

                let targets = SubsystemTarget::all();
                self.ctx.set_font("13px monospace");
                for (i, target) in targets.iter().enumerate() {
                    let ty = py + 30.0 + i as f64 * 26.0;
                    let selected = i == target_cursor;

                    if selected {
                        self.ctx.set_fill_style_str("rgba(0,200,220,0.2)");
                        self.ctx.fill_rect(px + 4.0, ty, panel_w - 8.0, 22.0);
                    }

                    // Arrow indicator
                    self.ctx.set_fill_style_str(if selected { "#00ffff" } else { "#666666" });
                    self.ctx.fill_text(if selected { ">" } else { " " }, px + 8.0, ty + 16.0).ok();

                    // Target name
                    self.ctx.set_fill_style_str(if selected { "#ffffff" } else { "#aaaaaa" });
                    self.ctx.fill_text(target.name(), px + 24.0, ty + 16.0).ok();

                    // Mini HP bar for that subsystem
                    let (sub_hp, sub_max) = match target {
                        SubsystemTarget::Weapons => (enemy_ship.weapons_sub.hp, enemy_ship.weapons_sub.max_hp),
                        SubsystemTarget::Shields => (enemy_ship.shields_sub.hp, enemy_ship.shields_sub.max_hp),
                        SubsystemTarget::Engines => (enemy_ship.engines_sub.hp, enemy_ship.engines_sub.max_hp),
                        SubsystemTarget::Hull    => (enemy_ship.hull, enemy_ship.max_hull),
                    };
                    let bar_x = px + 110.0;
                    let bar_w = 90.0;
                    self.ctx.set_fill_style_str("#222222");
                    self.ctx.fill_rect(bar_x, ty + 2.0, bar_w, 12.0);
                    let pct = (sub_hp as f64 / sub_max.max(1) as f64).max(0.0);
                    let color = if pct > 0.5 { "#44cc44" } else if pct > 0.25 { "#ccaa00" } else { "#cc4444" };
                    self.ctx.set_fill_style_str(color);
                    self.ctx.fill_rect(bar_x, ty + 2.0, bar_w * pct, 12.0);
                    self.ctx.set_fill_style_str("#ffffff");
                    self.ctx.set_font("10px monospace");
                    self.ctx.fill_text(&format!("{}/{}", sub_hp.max(0), sub_max), bar_x + bar_w + 4.0, ty + 14.0).ok();
                    self.ctx.set_font("13px monospace");
                }

                // Hint
                self.ctx.set_font("11px monospace");
                self.ctx.set_fill_style_str("#666666");
                self.ctx.fill_text("Enter: fire  Esc: back", px + 8.0, py + panel_h - 6.0).ok();
            }
            _ => {}
        }

        // ── Victory/Defeat overlay ──
        match phase {
            SpaceCombatPhase::Victory => {
                self.ctx.set_fill_style_str("rgba(0, 40, 0, 0.85)");
                self.ctx.fill_rect(w * 0.15, h * 0.25, w * 0.7, h * 0.4);
                self.ctx.set_stroke_style_str("#44ff44");
                self.ctx.set_line_width(2.0);
                self.ctx.stroke_rect(w * 0.15, h * 0.25, w * 0.7, h * 0.4);

                self.ctx.set_font("32px monospace");
                self.ctx.set_fill_style_str("#44ff44");
                self.ctx.fill_text("VICTORY!", w * 0.38, h * 0.35).ok();

                self.ctx.set_font("14px monospace");
                self.ctx.set_fill_style_str("#ffffff");
                self.ctx.fill_text(&format!("Loot: {} credits", enemy_ship.loot_credits), w * 0.30, h * 0.42).ok();

                // Subsystem damage summary
                self.ctx.set_fill_style_str("#aaaaaa");
                let summary_y = h * 0.47;
                let subs_summary = [
                    ("Enemy Weapons", &enemy_ship.weapons_sub),
                    ("Enemy Shields", &enemy_ship.shields_sub),
                    ("Enemy Engines", &enemy_ship.engines_sub),
                ];
                for (i, (lbl, sub)) in subs_summary.iter().enumerate() {
                    let status = if sub.is_destroyed() { "DESTROYED" } else { &format!("{}/{}", sub.hp.max(0), sub.max_hp) };
                    let color = if sub.is_destroyed() { "#ff4444" } else { "#88cc88" };
                    self.ctx.set_fill_style_str(color);
                    self.ctx.fill_text(&format!("{}: {}", lbl, status), w * 0.30, summary_y + i as f64 * 18.0).ok();
                }

                self.ctx.set_fill_style_str("#888888");
                self.ctx.fill_text("Press any key to continue", w * 0.34, h * 0.60).ok();
            }
            SpaceCombatPhase::Defeat => {
                self.ctx.set_fill_style_str("rgba(40, 0, 0, 0.85)");
                self.ctx.fill_rect(w * 0.15, h * 0.25, w * 0.7, h * 0.4);
                self.ctx.set_stroke_style_str("#ff4444");
                self.ctx.set_line_width(2.0);
                self.ctx.stroke_rect(w * 0.15, h * 0.25, w * 0.7, h * 0.4);

                self.ctx.set_font("32px monospace");
                self.ctx.set_fill_style_str("#ff4444");
                self.ctx.fill_text("DEFEAT!", w * 0.39, h * 0.38).ok();
                self.ctx.set_font("16px monospace");
                self.ctx.set_fill_style_str("#ffffff");
                self.ctx.fill_text("Your ship was destroyed...", w * 0.32, h * 0.48).ok();
                self.ctx.fill_text("Press any key to continue", w * 0.34, h * 0.55).ok();
            }
            _ => {}
        }
    }

    /// Draw a horizontal bar with label text
    fn draw_bar(&self, x: f64, y: f64, w: f64, h: f64, pct: f64, label: &str, is_hull: bool) {
        let pct = pct.max(0.0).min(1.0);
        self.ctx.set_fill_style_str("#333333");
        self.ctx.fill_rect(x, y, w, h);
        let color = if is_hull {
            if pct > 0.5 { "#44ff44" } else if pct > 0.25 { "#ffaa00" } else { "#ff4444" }
        } else {
            "#4488ff"
        };
        self.ctx.set_fill_style_str(color);
        self.ctx.fill_rect(x, y, w * pct, h);
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_font("11px monospace");
        self.ctx.fill_text(label, x + 4.0, y + h - 3.0).ok();
    }

    pub fn draw_event(
        &self,
        event: &SpaceEvent,
        cursor: usize,
    ) {
        let box_x = 50.0;
        let box_y = 50.0;
        let box_w = self.canvas_w - 100.0;
        let box_h = self.canvas_h - 100.0;
        let inner_w = box_w - 60.0;
        // 16px monospace ≈ 9.6px per char
        let max_chars = (inner_w / 9.6) as usize;

        // Overlay background
        self.ctx.set_fill_style_str("rgba(0, 0, 0, 0.9)");
        self.ctx.fill_rect(box_x, box_y, box_w, box_h);
        
        self.ctx.set_stroke_style_str("#00ccdd");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(box_x, box_y, box_w, box_h);
        
        // Title
        self.ctx.set_font("bold 24px serif");
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_text_align("center");
        self.ctx.fill_text(event.title, self.canvas_w / 2.0, 100.0).ok();
        
        self.ctx.set_font("20px serif");
        self.ctx.set_fill_style_str("#cccccc");
        self.ctx.fill_text(event.chinese_title, self.canvas_w / 2.0, 130.0).ok();
        
        // Description (word-wrapped)
        self.ctx.set_font("16px monospace");
        self.ctx.set_fill_style_str("#aaaaaa");
        self.ctx.set_text_align("left");
        let desc_lines = word_wrap(event.description, max_chars);
        let mut y = 180.0;
        for line in &desc_lines {
            self.ctx.fill_text(line, 80.0, y).ok();
            y += 22.0;
        }

        // Pre-wrap choices
        let choice_lines: Vec<Vec<String>> = event.choices.iter()
            .map(|c| word_wrap(&format!("{}. {}", 0, c.text), max_chars.saturating_sub(3)))
            .collect();

        // Choices
        let start_y = y + 40.0;
        let mut cy = start_y;
        for (i, _choice) in event.choices.iter().enumerate() {
            if i == cursor {
                let choice_h = choice_lines[i].len() as f64 * 20.0 + 4.0;
                self.ctx.set_fill_style_str("#004455");
                self.ctx.fill_rect(70.0, cy - 18.0, inner_w + 20.0, choice_h);
                self.ctx.set_fill_style_str("#ffffff");
            } else {
                self.ctx.set_fill_style_str("#888888");
            }
            for (j, line) in choice_lines[i].iter().enumerate() {
                let text = if j == 0 {
                    format!("{}. {}", i + 1, &line[line.find(". ").map(|p| p + 2).unwrap_or(0)..])
                } else {
                    format!("   {}", line)
                };
                self.ctx.fill_text(&text, 80.0, cy).ok();
                cy += 20.0;
            }
            cy += 12.0;
        }
    }
}

