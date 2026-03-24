//! Tile rendering: surfaces, borders, palettes, and tiling-sprite helpers.

use crate::world::{TerminalKind, DungeonLevel, SealKind, Tile};

use super::TILE_SIZE;
use super::{COL_WALL, COL_WALL_REVEALED, COL_FLOOR, COL_FLOOR_REVEALED};
use super::{COL_CORRIDOR, COL_CORRIDOR_REVEALED, COL_STAIRS, COL_FORGE, COL_SHOP, COL_CHEST};

impl super::Renderer {
    /// Draw a sprite as a seamless tiling texture for arena-style grids,
    /// using grid coordinates to offset into the texture.
    pub(crate) fn draw_tiling_sprite_key(
        &self,
        key: &str,
        gx: usize,
        gy: usize,
        dest_x: f64,
        dest_y: f64,
        cell_size: f64,
    ) -> bool {
        if self.sprites.is_loaded(key) {
            if let Some(img) = self.sprites.get(key) {
                self.draw_tiling_sprite_sized(img, gx as i32, gy as i32, dest_x, dest_y, cell_size);
                return true;
            }
        }
        false
    }

    /// Draw a sprite as a seamless tiling texture, offset by the tile's grid
    /// position so adjacent tiles show adjacent portions of the texture.
    pub(crate) fn draw_tiling_sprite(
        &self,
        img: &web_sys::HtmlImageElement,
        tx: i32,
        ty: i32,
        dest_x: f64,
        dest_y: f64,
    ) {
        self.draw_tiling_sprite_sized(img, tx, ty, dest_x, dest_y, TILE_SIZE);
    }

    /// Draw a sprite as a seamless tiling texture with configurable cell size.
    pub(crate) fn draw_tiling_sprite_sized(
        &self,
        img: &web_sys::HtmlImageElement,
        tx: i32,
        ty: i32,
        dest_x: f64,
        dest_y: f64,
        cell_size: f64,
    ) {
        let iw = img.natural_width() as f64;
        let ih = img.natural_height() as f64;
        if iw <= 0.0 || ih <= 0.0 {
            return;
        }
        let src_x = ((tx as f64 * cell_size) % iw + iw) % iw;
        let src_y = ((ty as f64 * cell_size) % ih + ih) % ih;
        let fit_w = (iw - src_x).min(cell_size);
        let fit_h = (ih - src_y).min(cell_size);

        let _ = self
            .ctx
            .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, src_x, src_y, fit_w, fit_h, dest_x, dest_y, fit_w, fit_h,
            );
        if fit_w < cell_size {
            let rem = cell_size - fit_w;
            let _ = self
                .ctx
                .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    img,
                    0.0,
                    src_y,
                    rem,
                    fit_h,
                    dest_x + fit_w,
                    dest_y,
                    rem,
                    fit_h,
                );
        }
        if fit_h < cell_size {
            let rem = cell_size - fit_h;
            let _ = self
                .ctx
                .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    img,
                    src_x,
                    0.0,
                    fit_w,
                    rem,
                    dest_x,
                    dest_y + fit_h,
                    fit_w,
                    rem,
                );
        }
        if fit_w < cell_size && fit_h < cell_size {
            let rem_w = cell_size - fit_w;
            let rem_h = cell_size - fit_h;
            let _ = self
                .ctx
                .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    img,
                    0.0,
                    0.0,
                    rem_w,
                    rem_h,
                    dest_x + fit_w,
                    dest_y + fit_h,
                    rem_w,
                    rem_h,
                );
        }
    }

    pub(crate) fn draw_tile_surface(
        &self,
        level: &DungeonLevel,
        tile: Tile,
        palette: TilePalette,
        tx: i32,
        ty: i32,
        screen_x: f64,
        screen_y: f64,
        anim_t: f64,
    ) {
        let pattern = tile_pattern_seed(tx, ty);
        let (highlight, shadow) =
            if matches!(tile, Tile::Bulkhead | Tile::DamagedBulkhead | Tile::WeakBulkhead) {
                ("rgba(255,255,255,0.08)", "rgba(0,0,0,0.32)")
            } else {
                ("rgba(255,255,255,0.06)", "rgba(0,0,0,0.24)")
            };

        // Only draw bevel edges toward tiles that don't visually connect
        let n_top = !level.in_bounds(tx, ty - 1)
            || !tiles_connect(tile, level.tiles[level.idx(tx, ty - 1)]);
        let n_right = !level.in_bounds(tx + 1, ty)
            || !tiles_connect(tile, level.tiles[level.idx(tx + 1, ty)]);
        let n_bottom = !level.in_bounds(tx, ty + 1)
            || !tiles_connect(tile, level.tiles[level.idx(tx, ty + 1)]);
        let n_left = !level.in_bounds(tx - 1, ty)
            || !tiles_connect(tile, level.tiles[level.idx(tx - 1, ty)]);

        if n_top || n_left {
            self.ctx.set_fill_style_str(highlight);
            if n_top {
                self.ctx
                    .fill_rect(screen_x + 0.5, screen_y + 0.5, TILE_SIZE - 1.0, 1.0);
            }
            if n_left {
                self.ctx
                    .fill_rect(screen_x + 0.5, screen_y + 1.5, 1.0, TILE_SIZE - 2.0);
            }
        }
        if n_right || n_bottom {
            self.ctx.set_fill_style_str(shadow);
            if n_right {
                self.ctx.fill_rect(
                    screen_x + TILE_SIZE - 1.5,
                    screen_y + 1.5,
                    1.0,
                    TILE_SIZE - 2.0,
                );
            }
            if n_bottom {
                self.ctx.fill_rect(
                    screen_x + 1.5,
                    screen_y + TILE_SIZE - 1.5,
                    TILE_SIZE - 2.0,
                    1.0,
                );
            }
        }

        match tile {
            Tile::NavBeacon | Tile::SpecialRoom(_) | Tile::SalvageCrate
            | Tile::MetalFloor | Tile::Hallway | Tile::CorruptedFloor | Tile::Catwalk
            | Tile::Trap(_) => {},
            | Tile::FrozenDeck | Tile::ToxicGas | Tile::ToxicFungus => {
                self.ctx.set_fill_style_str(if tile == Tile::Hallway {
                    "rgba(215,225,255,0.06)"
                } else {
                    "rgba(255,255,255,0.05)"
                });
                let spark_x = screen_x + 4.0 + (pattern % 11) as f64;
                let spark_y = screen_y + 4.0 + ((pattern / 11) % 9) as f64;
                self.ctx.fill_rect(spark_x, spark_y, 2.0, 2.0);
                if tile == Tile::Hallway {
                    self.ctx.set_fill_style_str("rgba(170,190,255,0.05)");
                    self.ctx.fill_rect(
                        screen_x + 4.0,
                        screen_y + TILE_SIZE / 2.0 - 0.5,
                        TILE_SIZE - 8.0,
                        1.0,
                    );
                }
                if tile == Tile::CorruptedFloor {
                    // Subtle cursed shimmer — barely visible trap hint
                    self.ctx.set_fill_style_str("rgba(180,120,255,0.06)");
                    let cx = screen_x + 6.0 + ((pattern / 3) % 8) as f64;
                    let cy = screen_y + 6.0 + ((pattern / 7) % 8) as f64;
                    self.ctx.fill_rect(cx, cy, 2.0, 2.0);
                }
            }
            Tile::Bulkhead | Tile::DamagedBulkhead | Tile::WeakBulkhead | Tile::CargoPipes => {
                self.ctx.set_fill_style_str("rgba(0,0,0,0.14)");
                self.ctx.fill_rect(
                    screen_x + 3.0,
                    screen_y + 3.0,
                    TILE_SIZE - 6.0,
                    TILE_SIZE - 6.0,
                );
                self.ctx.set_fill_style_str("rgba(255,255,255,0.07)");
                let seam_y = screen_y + 7.0 + (pattern % 6) as f64;
                self.ctx
                    .fill_rect(screen_x + 3.0, seam_y, TILE_SIZE - 6.0, 1.0);
                let seam_x = screen_x + 7.0 + ((pattern / 5) % 8) as f64;
                self.ctx
                    .fill_rect(seam_x, screen_y + 3.0, 1.0, TILE_SIZE / 2.0 - 1.0);
                if tile == Tile::DamagedBulkhead {
                    self.ctx.set_stroke_style_str("rgba(255,180,120,0.65)");
                    self.ctx.set_line_width(1.2);
                    self.ctx.begin_path();
                    self.ctx.move_to(screen_x + 12.0, screen_y + 3.0);
                    self.ctx.line_to(screen_x + 10.0, screen_y + 9.0);
                    self.ctx.line_to(screen_x + 14.0, screen_y + 14.0);
                    self.ctx.line_to(screen_x + 9.0, screen_y + 21.0);
                    self.ctx.stroke();

                    self.ctx.begin_path();
                    self.ctx.move_to(screen_x + 10.0, screen_y + 9.0);
                    self.ctx.line_to(screen_x + 6.0, screen_y + 12.0);
                    self.ctx.stroke();

                    self.ctx.begin_path();
                    self.ctx.move_to(screen_x + 14.0, screen_y + 14.0);
                    self.ctx.line_to(screen_x + 18.0, screen_y + 17.0);
                    self.ctx.stroke();
                } else if tile == Tile::WeakBulkhead {
                    self.ctx.set_stroke_style_str("rgba(255,221,170,0.58)");
                    self.ctx.set_line_width(1.0);
                    self.ctx.begin_path();
                    self.ctx.move_to(screen_x + 7.0, screen_y + 6.0);
                    self.ctx.line_to(screen_x + 12.0, screen_y + 11.0);
                    self.ctx.line_to(screen_x + 9.0, screen_y + 17.0);
                    self.ctx.line_to(screen_x + 15.0, screen_y + 21.0);
                    self.ctx.stroke();

                    self.ctx.begin_path();
                    self.ctx.move_to(screen_x + 12.0, screen_y + 11.0);
                    self.ctx.line_to(screen_x + 18.0, screen_y + 9.0);
                    self.ctx.stroke();
                }
            }
            Tile::CoolantPool | Tile::VacuumBreach | Tile::PlasmaVent => {
                let wave_shift = (anim_t * 3.2 + tx as f64 * 0.7 + ty as f64 * 0.4).sin() * 2.0;
                self.ctx.set_fill_style_str(if tile == Tile::VacuumBreach {
                    "rgba(160,200,255,0.14)"
                } else {
                    "rgba(210,230,255,0.11)"
                });
                self.ctx.fill_rect(
                    screen_x + 3.0 + wave_shift,
                    screen_y + 7.0,
                    TILE_SIZE - 8.0,
                    1.5,
                );
                self.ctx.fill_rect(
                    screen_x + 5.0 - wave_shift,
                    screen_y + 14.0,
                    TILE_SIZE - 10.0,
                    1.5,
                );
                if tile == Tile::VacuumBreach {
                    self.ctx.set_fill_style_str("rgba(26,48,89,0.28)");
                    self.ctx
                        .fill_rect(screen_x + 4.0, screen_y + 18.0, TILE_SIZE - 8.0, 3.0);
                }
            }
            Tile::Coolant => {
                self.ctx.set_fill_style_str("rgba(255,224,154,0.10)");
                self.ctx.fill_rect(
                    screen_x + 4.0,
                    screen_y + TILE_SIZE - 8.0,
                    TILE_SIZE - 8.0,
                    2.0,
                );
                self.ctx.set_fill_style_str("rgba(255,255,255,0.06)");
                self.ctx
                    .fill_rect(screen_x + 6.0, screen_y + 6.0, TILE_SIZE - 14.0, 1.5);
            }
            Tile::LaserGrid => {
                self.ctx.set_fill_style_str("rgba(255,220,220,0.08)");
                self.ctx.fill_rect(
                    screen_x + 4.0,
                    screen_y + TILE_SIZE - 7.0,
                    TILE_SIZE - 8.0,
                    2.0,
                );
            }
            Tile::Airlock
            | Tile::QuantumForge
            | Tile::TradeTerminal
            | Tile::SupplyCrate
            | Tile::Npc(_)
            | Tile::CircuitShrine
            | Tile::RadicalLab
            | Tile::FrequencyWall
            | Tile::CompoundShrine
            | Tile::ClassifierNode
            | Tile::DataWell
            | Tile::MemorialNode
            | Tile::TranslationTerminal
            | Tile::HoloPool
            | Tile::DroidTutor
            | Tile::CodexTerminal
            | Tile::DataBridge
            | Tile::SealedHatch
            | Tile::Terminal(_)
            | Tile::SecurityLock(_)
            | Tile::InfoPanel(_)
            | Tile::OreVein
            | Tile::DataRack
            | Tile::WarpGatePortal
            | Tile::MedBayTile
            | Tile::CreditCache
            | Tile::CrystalPanel
            | Tile::PressureSensor
            | Tile::CargoCrate => {
                if let Some(plate_fill) = tile_plate_fill(tile) {
                    self.ctx.set_fill_style_str(plate_fill);
                    self.ctx.fill_rect(
                        screen_x + 3.0,
                        screen_y + 3.0,
                        TILE_SIZE - 6.0,
                        TILE_SIZE - 6.0,
                    );
                }
            }
        }

        if let Some(accent) = palette.accent {
            self.ctx.set_stroke_style_str(accent);
            self.ctx.set_line_width(1.0);
            self.ctx.stroke_rect(
                screen_x + 1.5,
                screen_y + 1.5,
                TILE_SIZE - 3.0,
                TILE_SIZE - 3.0,
            );
        }

        if let Some(glyph) = palette.glyph {
            self.ctx
                .set_shadow_color(palette.accent.unwrap_or("transparent"));
            self.ctx
                .set_shadow_blur(if palette.accent.is_some() { 8.0 } else { 0.0 });
            self.ctx.set_fill_style_str(palette.glyph_color);
            self.ctx.set_font(tile_glyph_font(tile));
            self.ctx.set_text_align("center");
            self.ctx
                .fill_text(
                    glyph,
                    screen_x + TILE_SIZE / 2.0,
                    tile_glyph_y(tile, screen_y, anim_t, tx, ty),
                )
                .ok();
            self.ctx.set_shadow_color("transparent");
            self.ctx.set_shadow_blur(0.0);
        }

        if tile.is_walkable() {
            self.ctx.set_stroke_style_str("rgba(255,255,255,0.05)");
            self.ctx.set_line_width(0.5);
            self.ctx
                .stroke_rect(screen_x, screen_y, TILE_SIZE, TILE_SIZE);
        }
    }

    pub(crate) fn draw_wall_borders(&self, level: &DungeonLevel, tx: i32, ty: i32, sx: f64, sy: f64) {
        let t = TILE_SIZE;
        let neighbors = [
            (tx, ty - 1), // north
            (tx + 1, ty), // east
            (tx, ty + 1), // south
            (tx - 1, ty), // west
        ];

        for (i, &(nx, ny)) in neighbors.iter().enumerate() {
            let is_wall_neighbor = if level.in_bounds(nx, ny) {
                is_wall_tile(level.tiles[level.idx(nx, ny)])
            } else {
                true
            };

            if !is_wall_neighbor {
                // Wall edge facing open space — subtle highlight line
                self.ctx.set_stroke_style_str("rgba(60,75,100,0.3)");
                self.ctx.set_line_width(1.0);
                self.ctx.begin_path();
                match i {
                    0 => { self.ctx.move_to(sx, sy + 0.5); self.ctx.line_to(sx + t, sy + 0.5); }
                    1 => { self.ctx.move_to(sx + t - 0.5, sy); self.ctx.line_to(sx + t - 0.5, sy + t); }
                    2 => { self.ctx.move_to(sx, sy + t - 0.5); self.ctx.line_to(sx + t, sy + t - 0.5); }
                    3 => { self.ctx.move_to(sx + 0.5, sy); self.ctx.line_to(sx + 0.5, sy + t); }
                    _ => {}
                }
                self.ctx.stroke();

                // Shadow cast onto the floor side
                self.ctx.set_fill_style_str("rgba(0,0,0,0.18)");
                match i {
                    0 => self.ctx.fill_rect(sx, sy - 2.0, t, 2.0),
                    1 => self.ctx.fill_rect(sx + t, sy, 2.0, t),
                    2 => self.ctx.fill_rect(sx, sy + t, t, 2.0),
                    3 => self.ctx.fill_rect(sx - 2.0, sy, 2.0, t),
                    _ => {}
                }
            }
            // Wall-to-wall: no seam line — adjacent walls form a continuous surface
        }

        // Diagonal corners — inner corner bevels
        let diagonals: [(i32, i32, usize, usize); 4] = [
            (tx - 1, ty - 1, 3, 0), // NW
            (tx + 1, ty - 1, 1, 0), // NE
            (tx + 1, ty + 1, 1, 2), // SE
            (tx - 1, ty + 1, 3, 2), // SW
        ];

        for &(dx, dy, side_a, side_b) in &diagonals {
            let a_wall = if level.in_bounds(neighbors[side_a].0, neighbors[side_a].1) {
                is_wall_tile(level.tiles[level.idx(neighbors[side_a].0, neighbors[side_a].1)])
            } else {
                true
            };

            let b_wall = if level.in_bounds(neighbors[side_b].0, neighbors[side_b].1) {
                is_wall_tile(level.tiles[level.idx(neighbors[side_b].0, neighbors[side_b].1)])
            } else {
                true
            };

            let diag_floor = if level.in_bounds(dx, dy) {
                !is_wall_tile(level.tiles[level.idx(dx, dy)])
            } else {
                false
            };

            if a_wall && b_wall && diag_floor {
                self.ctx.set_fill_style_str("rgba(0,0,0,0.25)");
                let cs = 3.0;
                match (side_a, side_b) {
                    (3, 0) => self.ctx.fill_rect(sx, sy, cs, cs),
                    (1, 0) => self.ctx.fill_rect(sx + t - cs, sy, cs, cs),
                    (1, 2) => self.ctx.fill_rect(sx + t - cs, sy + t - cs, cs, cs),
                    (3, 2) => self.ctx.fill_rect(sx, sy + t - cs, cs, cs),
                    _ => {}
                }
            }
        }
    }

    pub(crate) fn draw_floor_borders(&self, level: &DungeonLevel, tx: i32, ty: i32, sx: f64, sy: f64) {
        let t = TILE_SIZE;
        let neighbors = [(tx, ty - 1), (tx + 1, ty), (tx, ty + 1), (tx - 1, ty)];

        for (i, &(nx, ny)) in neighbors.iter().enumerate() {
            let neighbor_is_wall = if level.in_bounds(nx, ny) {
                is_wall_tile(level.tiles[level.idx(nx, ny)])
            } else {
                true
            };

            if neighbor_is_wall {
                let shadow_depth = 2.0;
                self.ctx.set_fill_style_str("rgba(0,0,0,0.12)");
                match i {
                    0 => self.ctx.fill_rect(sx, sy, t, shadow_depth),
                    1 => self.ctx.fill_rect(sx + t - shadow_depth, sy, shadow_depth, t),
                    2 => self.ctx.fill_rect(sx, sy + t - shadow_depth, t, shadow_depth),
                    3 => self.ctx.fill_rect(sx, sy, shadow_depth, t),
                    _ => {}
                }
            }
            // Floor-to-floor: no edge lines — adjacent floors form a continuous surface
        }
    }
}

pub(super) fn is_wall_tile(tile: Tile) -> bool {
    matches!(
        tile,
        Tile::Bulkhead
            | Tile::DamagedBulkhead
            | Tile::WeakBulkhead
            | Tile::CargoPipes
            | Tile::CrystalPanel
    )
}

/// Visual grouping for tiles — tiles in the same group should blend seamlessly.
pub(super) fn tile_visual_group(tile: Tile) -> u8 {
    match tile {
        // Wall group
        Tile::Bulkhead | Tile::DamagedBulkhead | Tile::WeakBulkhead
        | Tile::CargoPipes | Tile::CrystalPanel => 0,
        // Basic floor group
        Tile::MetalFloor | Tile::Hallway | Tile::CorruptedFloor
        | Tile::FrozenDeck | Tile::Catwalk | Tile::PressureSensor => 1,
        // Water/coolant group
        Tile::CoolantPool | Tile::VacuumBreach => 2,
        // Coolant/oil group
        Tile::Coolant => 3,
        // Everything else gets a unique group (won't connect)
        _ => 255,
    }
}

/// Returns true if two tiles should visually connect (no seam between them).
pub(super) fn tiles_connect(a: Tile, b: Tile) -> bool {
    let ga = tile_visual_group(a);
    let gb = tile_visual_group(b);
    ga == gb && ga != 255
}

/// Returns true if this tile type should use tiling (offset) sprite rendering
/// instead of the default "draw full sprite scaled to tile size" approach.
pub(super) fn should_tile_sprite(tile: Tile) -> bool {
    tile_visual_group(tile) != 255
}

pub(super) fn tile_sprite_key(tile: Tile, location_label: &str) -> &'static str {
    // Map location label to sprite key prefix for wall/floor/corridor
    let loc_prefix: Option<&'static str> = match location_label {
        "Space Station" => Some("loc_space_station"),
        "Asteroid Base" => Some("loc_asteroid_base"),
        "Derelict Ship" => Some("loc_derelict_ship"),
        "Alien Ruins" => Some("loc_alien_ruins"),
        "Trading Post" => Some("loc_trading_post"),
        "Orbital Platform" => Some("loc_orbital_platform"),
        "Mining Colony" => Some("loc_mining_colony"),
        "Research Lab" => Some("loc_research_lab"),
        _ => None,
    };
    match tile {
        Tile::Bulkhead | Tile::CargoPipes | Tile::CrystalPanel => {
            if let Some(p) = loc_prefix {
                match p {
                    "loc_space_station" => "loc_space_station_wall",
                    "loc_asteroid_base" => "loc_asteroid_base_wall",
                    "loc_derelict_ship" => "loc_derelict_ship_wall",
                    "loc_alien_ruins" => "loc_alien_ruins_wall",
                    "loc_trading_post" => "loc_trading_post_wall",
                    "loc_orbital_platform" => "loc_orbital_platform_wall",
                    "loc_mining_colony" => "loc_mining_colony_wall",
                    "loc_research_lab" => "loc_research_lab_wall",
                    _ => "tile_wall",
                }
            } else {
                "tile_wall"
            }
        }
        Tile::MetalFloor | Tile::CorruptedFloor | Tile::FrozenDeck | Tile::CreditCache
        | Tile::ToxicFungus | Tile::ToxicGas | Tile::PressureSensor => {
            if let Some(p) = loc_prefix {
                match p {
                    "loc_space_station" => "loc_space_station_floor",
                    "loc_asteroid_base" => "loc_asteroid_base_floor",
                    "loc_derelict_ship" => "loc_derelict_ship_floor",
                    "loc_alien_ruins" => "loc_alien_ruins_floor",
                    "loc_trading_post" => "loc_trading_post_floor",
                    "loc_orbital_platform" => "loc_orbital_platform_floor",
                    "loc_mining_colony" => "loc_mining_colony_floor",
                    "loc_research_lab" => "loc_research_lab_floor",
                    _ => "tile_floor",
                }
            } else {
                "tile_floor"
            }
        }
        Tile::Hallway | Tile::Catwalk | Tile::DataBridge => {
            if let Some(p) = loc_prefix {
                match p {
                    "loc_space_station" => "loc_space_station_corridor",
                    "loc_asteroid_base" => "loc_asteroid_base_corridor",
                    "loc_derelict_ship" => "loc_derelict_ship_corridor",
                    "loc_alien_ruins" => "loc_alien_ruins_corridor",
                    "loc_trading_post" => "loc_trading_post_corridor",
                    "loc_orbital_platform" => "loc_orbital_platform_corridor",
                    "loc_mining_colony" => "loc_mining_colony_corridor",
                    "loc_research_lab" => "loc_research_lab_corridor",
                    _ => "tile_corridor",
                }
            } else {
                "tile_corridor"
            }
        }
        Tile::Airlock => "tile_stairs_down_scifi",
        Tile::QuantumForge => "obj_quantum_forge",
        Tile::TradeTerminal => "obj_space_shop",
        Tile::SupplyCrate | Tile::SalvageCrate | Tile::CargoCrate => "obj_cargo_crate",
        Tile::MedBayTile => "obj_medbay",
        Tile::PlasmaVent => "obj_plasma_vent",
        Tile::WarpGatePortal => "obj_warp_gate",
        Tile::DataRack => "obj_data_archive",
        Tile::OreVein => "obj_loot_container",
        Tile::NavBeacon => "obj_holo_map",
        Tile::SpecialRoom(_) => "obj_terminal",
        Tile::DamagedBulkhead => "tile_cracked_wall",
        Tile::WeakBulkhead => "tile_brittle_wall",
        Tile::LaserGrid => "tile_spikes",
        Tile::Coolant => "tile_oil",
        Tile::CoolantPool => "tile_water",
        Tile::VacuumBreach => "tile_deep_water",
        Tile::CircuitShrine => "obj_alien_artifact",
        Tile::RadicalLab => "obj_reactor_core",
        Tile::FrequencyWall => "obj_shield_generator",
        Tile::CompoundShrine => "obj_alien_artifact",
        Tile::ClassifierNode => "obj_terminal",
        Tile::DataWell => "obj_data_archive",
        Tile::MemorialNode => "obj_alien_artifact",
        Tile::TranslationTerminal => "obj_terminal",
        Tile::HoloPool => "obj_holo_map",
        Tile::DroidTutor => "obj_robot_wreck",
        Tile::CodexTerminal => "obj_terminal",
        Tile::SealedHatch => "obj_containment_cell",
        Tile::Terminal(TerminalKind::Quantum) => "obj_alien_artifact",
        Tile::Terminal(TerminalKind::Stellar) => "obj_reactor_core",
        Tile::Terminal(TerminalKind::Holographic) => "obj_holo_map",
        Tile::Terminal(TerminalKind::Tactical) => "obj_weapon_rack",
        Tile::Terminal(TerminalKind::Commerce) => "obj_space_shop",
        Tile::SecurityLock(SealKind::Thermal) => "obj_shield_generator",
        Tile::SecurityLock(SealKind::Hydraulic) => "obj_fuel_pump",
        Tile::SecurityLock(SealKind::Kinetic) => "obj_turret",
        Tile::SecurityLock(SealKind::Sonic) => "obj_containment_cell",
        Tile::InfoPanel(_) => "obj_terminal",
        Tile::Npc(0) => "npc_teacher",
        Tile::Npc(1) => "npc_monk",
        Tile::Npc(2) => "npc_merchant",
        Tile::Npc(_) => "npc_guard",
        Tile::Trap(_) => "tile_floor",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TilePalette {
    pub(crate) fill: &'static str,
    pub(crate) accent: Option<&'static str>,
    pub(crate) glyph: Option<&'static str>,
    pub(crate) glyph_color: &'static str,
}

pub(crate) fn tile_palette(tile: Tile, visible: bool) -> TilePalette {
    if visible {
        match tile {
            Tile::NavBeacon | Tile::SpecialRoom(_) | Tile::SalvageCrate => TilePalette {
                fill: "#444",
                accent: None,
                glyph: None,
                glyph_color: "#fff",
            },
            Tile::Bulkhead => TilePalette {
                fill: COL_WALL,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::DamagedBulkhead => TilePalette {
                fill: "#47324f",
                accent: Some("#d89c74"),
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::WeakBulkhead => TilePalette {
                fill: "#5b473a",
                accent: Some("#f2d29e"),
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::MetalFloor => TilePalette {
                fill: COL_FLOOR,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Hallway => TilePalette {
                fill: COL_CORRIDOR,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Airlock => TilePalette {
                fill: COL_STAIRS,
                accent: Some("#d7e7ff"),
                glyph: Some("▼"),
                glyph_color: "#ffffff",
            },
            Tile::QuantumForge => TilePalette {
                fill: COL_FORGE,
                accent: Some("#ffd1aa"),
                glyph: Some("⚒"),
                glyph_color: "#ffffff",
            },
            Tile::TradeTerminal => TilePalette {
                fill: COL_SHOP,
                accent: Some("#bfffd4"),
                glyph: Some("$"),
                glyph_color: "#ffffff",
            },
            Tile::SupplyCrate => TilePalette {
                fill: COL_CHEST,
                accent: Some("#ffe29e"),
                glyph: Some("◆"),
                glyph_color: "#fff7dc",
            },
            Tile::LaserGrid => TilePalette {
                fill: "#7e434a",
                accent: Some("#d9a0a0"),
                glyph: Some("^"),
                glyph_color: "#fff1f1",
            },
            Tile::Coolant => TilePalette {
                fill: "#4f3a1c",
                accent: Some("#e7c56d"),
                glyph: Some("~"),
                glyph_color: "#ffdd88",
            },
            Tile::CoolantPool => TilePalette {
                fill: "#4466cc",
                accent: Some("#9fc4ff"),
                glyph: Some("≈"),
                glyph_color: "#e5efff",
            },
            Tile::VacuumBreach => TilePalette {
                fill: "#16386d",
                accent: Some("#7fb4ff"),
                glyph: Some("≈"),
                glyph_color: "#e5efff",
            },
            Tile::Npc(0) => TilePalette {
                fill: "#3b5876",
                accent: Some("#7fccff"),
                glyph: Some("📚"),
                glyph_color: "#ffffff",
            },
            Tile::Npc(1) => TilePalette {
                fill: "#2f6c5d",
                accent: Some("#96ffd8"),
                glyph: Some("🧘"),
                glyph_color: "#ffffff",
            },
            Tile::Npc(2) => TilePalette {
                fill: "#5e5331",
                accent: Some("#ffd588"),
                glyph: Some("💰"),
                glyph_color: "#ffffff",
            },
            Tile::Npc(_) => TilePalette {
                fill: "#4e476a",
                accent: Some("#d2c4ff"),
                glyph: Some("🛡"),
                glyph_color: "#ffffff",
            },
            Tile::CircuitShrine => TilePalette {
                fill: "#7d5d2a",
                accent: Some("#ffd07a"),
                glyph: Some("🔔"),
                glyph_color: "#fff8e2",
            },
            // Removed StrokeShrine, mapped to RadicalLab
            Tile::FrequencyWall => TilePalette {
                fill: "#3a1515",
                accent: Some("#dd6644"),
                glyph: Some("壁"),
                glyph_color: "#dd6644",
            },
            Tile::CompoundShrine => TilePalette {
                fill: "#1a3a1a",
                accent: Some("#66dd88"),
                glyph: Some("合"),
                glyph_color: "#66dd88",
            },
            Tile::ClassifierNode => TilePalette {
                fill: "#3a2a1a",
                accent: Some("#ddaa44"),
                glyph: Some("量"),
                glyph_color: "#ddaa44",
            },
            Tile::DataWell => TilePalette {
                fill: "#1a1a2d",
                accent: Some("#9999ee"),
                glyph: Some("墨"),
                glyph_color: "#9999ee",
            },
            Tile::MemorialNode => TilePalette {
                fill: "#2d1a1a",
                accent: Some("#ee9966"),
                glyph: Some("祖"),
                glyph_color: "#ee9966",
            },
            Tile::TranslationTerminal => TilePalette {
                fill: "#1a2d2d",
                accent: Some("#66cccc"),
                glyph: Some("译"),
                glyph_color: "#66cccc",
            },
            Tile::RadicalLab => TilePalette {
                fill: "#1a2d1a",
                accent: Some("#88ee66"),
                glyph: Some("部"),
                glyph_color: "#88ee66",
            },
            Tile::HoloPool => TilePalette {
                fill: "#1a1a3a",
                accent: Some("#aaaaff"),
                glyph: Some("鏡"),
                glyph_color: "#aaaaff",
            },
            Tile::DroidTutor => TilePalette {
                fill: "#2d2d1a",
                accent: Some("#cccc66"),
                glyph: Some("石"),
                glyph_color: "#cccc66",
            },
            Tile::CodexTerminal => TilePalette {
                fill: "#2a1a3a",
                accent: Some("#dd99ff"),
                glyph: Some("典"),
                glyph_color: "#dd99ff",
            },
            Tile::DataBridge => TilePalette {
                fill: "#1a1a2d",
                accent: Some("#66aaff"),
                glyph: Some("桥"),
                glyph_color: "#66aaff",
            },
            Tile::SealedHatch => TilePalette {
                fill: "#2d1a1a",
                accent: Some("#ff6644"),
                glyph: Some("锁"),
                glyph_color: "#ff6644",
            },
            Tile::CorruptedFloor => TilePalette {
                fill: "#1a1a1a",
                accent: None,
                glyph: None,
                glyph_color: "#aa44aa",
            },
            Tile::Terminal(kind) => TilePalette {
                fill: altar_fill(kind),
                accent: Some(kind.color()),
                glyph: Some(kind.icon()),
                glyph_color: kind.color(),
            },
            Tile::SecurityLock(kind) => TilePalette {
                fill: seal_fill(kind),
                accent: Some(kind.color()),
                glyph: Some(kind.icon()),
                glyph_color: kind.color(),
            },
            Tile::InfoPanel(_) => TilePalette {
                fill: "#8a6b47",
                accent: Some("#d7b07b"),
                glyph: Some("?"),
                glyph_color: "#ffffff",
            },
            Tile::Catwalk => TilePalette {
                fill: "#8b4513",
                accent: Some("#a0522d"),
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Trap(_) => TilePalette {
                fill: COL_FLOOR,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::OreVein => TilePalette {
                fill: "#5a4a1e",
                accent: Some("#ffd700"),
                glyph: Some("矿"),
                glyph_color: "#ffd700",
            },
            Tile::PlasmaVent => TilePalette {
                fill: "#8b2500",
                accent: Some("#ff4500"),
                glyph: Some("~"),
                glyph_color: "#ff6633",
            },
            Tile::FrozenDeck => TilePalette {
                fill: "#a8d8ea",
                accent: Some("#e0f0ff"),
                glyph: Some("·"),
                glyph_color: "#e0f0ff",
            },
            Tile::CargoPipes => TilePalette {
                fill: "#2d5a27",
                accent: Some("#6abf4b"),
                glyph: Some("‖"),
                glyph_color: "#88dd66",
            },
            Tile::ToxicFungus => TilePalette {
                fill: "#4a2d5a",
                accent: Some("#bb77dd"),
                glyph: Some("♠"),
                glyph_color: "#cc88ee",
            },
            Tile::ToxicGas => TilePalette {
                fill: "#2a4a2a",
                accent: Some("#77dd44"),
                glyph: Some("░"),
                glyph_color: "#88ee55",
            },
            Tile::DataRack => TilePalette {
                fill: "#5a3a1e",
                accent: Some("#c49a6c"),
                glyph: Some("书"),
                glyph_color: "#ddb888",
            },
            Tile::PressureSensor => TilePalette {
                fill: "#555555",
                accent: Some("#999999"),
                glyph: Some("◫"),
                glyph_color: "#bbbbbb",
            },
            Tile::CargoCrate => TilePalette {
                fill: "#666655",
                accent: Some("#998877"),
                glyph: Some("●"),
                glyph_color: "#bbaa99",
            },
            Tile::CrystalPanel => TilePalette {
                fill: "#3a3a6a",
                accent: Some("#aaaaff"),
                glyph: Some("◇"),
                glyph_color: "#ccccff",
            },
            Tile::WarpGatePortal => TilePalette {
                fill: "#4a1a3a",
                accent: Some("#ff44aa"),
                glyph: Some("龙"),
                glyph_color: "#ff66cc",
            },
            Tile::MedBayTile => TilePalette {
                fill: "#2a5a5a",
                accent: Some("#66ffdd"),
                glyph: Some("泉"),
                glyph_color: "#88ffee",
            },
            Tile::CreditCache => TilePalette {
                fill: "#5a4a1e",
                accent: Some("#ffd700"),
                glyph: Some("¥"),
                glyph_color: "#ffdd44",
            },
        }
    } else {
        match tile {
            Tile::NavBeacon | Tile::SpecialRoom(_) | Tile::SalvageCrate => TilePalette {
                fill: "#222",
                accent: None,
                glyph: None,
                glyph_color: "#555",
            },
            Tile::Bulkhead => TilePalette {
                fill: COL_WALL_REVEALED,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::DamagedBulkhead => TilePalette {
                fill: "#2d2338",
                accent: Some("#805d48"),
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::WeakBulkhead => TilePalette {
                fill: "#342c26",
                accent: Some("#7d6a57"),
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::MetalFloor => TilePalette {
                fill: COL_FLOOR_REVEALED,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Hallway => TilePalette {
                fill: COL_CORRIDOR_REVEALED,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Airlock => TilePalette {
                fill: "#243857",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::QuantumForge => TilePalette {
                fill: "#4b2b1d",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::TradeTerminal => TilePalette {
                fill: "#1e4a33",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::SupplyCrate => TilePalette {
                fill: "#5a441b",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::LaserGrid => TilePalette {
                fill: "#4a2d32",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Coolant => TilePalette {
                fill: "#3a2f1b",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CoolantPool => TilePalette {
                fill: "#213f6b",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::VacuumBreach => TilePalette {
                fill: "#132846",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Npc(_) => TilePalette {
                fill: "#27465c",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CircuitShrine => TilePalette {
                fill: "#4f3d20",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::RadicalLab => TilePalette {
                fill: "#111822",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::FrequencyWall => TilePalette {
                fill: "#1a1010",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CompoundShrine => TilePalette {
                fill: "#112211",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::ClassifierNode => TilePalette {
                fill: "#221a11",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::DataWell => TilePalette {
                fill: "#111118",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::MemorialNode => TilePalette {
                fill: "#181111",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::TranslationTerminal => TilePalette {
                fill: "#111818",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::HoloPool => TilePalette {
                fill: "#11111f",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::DroidTutor => TilePalette {
                fill: "#181811",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CodexTerminal => TilePalette {
                fill: "#16101e",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::DataBridge => TilePalette {
                fill: "#101018",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::SealedHatch => TilePalette {
                fill: "#1a1010",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CorruptedFloor => TilePalette {
                fill: "#111111",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Terminal(kind) => TilePalette {
                fill: altar_revealed_fill(kind),
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::SecurityLock(kind) => TilePalette {
                fill: seal_revealed_fill(kind),
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::InfoPanel(_) => TilePalette {
                fill: "#4b3a26",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Catwalk => TilePalette {
                fill: "#5c4033",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::Trap(_) => TilePalette {
                fill: COL_FLOOR_REVEALED,
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::OreVein => TilePalette {
                fill: "#3a3214",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::PlasmaVent => TilePalette {
                fill: "#5a1a00",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::FrozenDeck => TilePalette {
                fill: "#6a8a9a",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CargoPipes => TilePalette {
                fill: "#1e3a1b",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::ToxicFungus => TilePalette {
                fill: "#2d1a3a",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::ToxicGas => TilePalette {
                fill: "#1a2d1a",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::DataRack => TilePalette {
                fill: "#3a2614",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::PressureSensor => TilePalette {
                fill: "#333333",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CargoCrate => TilePalette {
                fill: "#3a3a33",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CrystalPanel => TilePalette {
                fill: "#222244",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::WarpGatePortal => TilePalette {
                fill: "#2d1024",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::MedBayTile => TilePalette {
                fill: "#1a3a3a",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
            Tile::CreditCache => TilePalette {
                fill: "#3a3214",
                accent: None,
                glyph: None,
                glyph_color: "#ffffff",
            },
        }
    }
}

pub(super) fn tile_plate_fill(tile: Tile) -> Option<&'static str> {
    match tile {
        Tile::Airlock => Some("rgba(255,255,255,0.14)"),
        Tile::QuantumForge => Some("rgba(255,226,194,0.16)"),
        Tile::TradeTerminal => Some("rgba(207,255,224,0.15)"),
        Tile::SupplyCrate => Some("rgba(255,231,173,0.16)"),
        Tile::Npc(_) => Some("rgba(225,245,255,0.12)"),
        Tile::CircuitShrine => Some("rgba(255,224,156,0.16)"),
        Tile::RadicalLab => Some("rgba(136,204,255,0.16)"),
        Tile::FrequencyWall => Some("rgba(221,102,68,0.16)"),
        Tile::CompoundShrine => Some("rgba(102,221,136,0.16)"),
        Tile::ClassifierNode => Some("rgba(221,170,68,0.16)"),
        Tile::DataWell => Some("rgba(153,153,238,0.16)"),
        Tile::MemorialNode => Some("rgba(238,153,102,0.16)"),
        Tile::TranslationTerminal => Some("rgba(102,204,204,0.16)"),
        Tile::HoloPool => Some("rgba(170,170,255,0.16)"),
        Tile::DroidTutor => Some("rgba(204,204,102,0.16)"),
        Tile::CodexTerminal => Some("rgba(221,153,255,0.16)"),
        Tile::DataBridge => Some("rgba(102,170,255,0.16)"),
        Tile::SealedHatch => Some("rgba(255,102,68,0.16)"),
        Tile::Terminal(kind) => Some(altar_plate_fill(kind)),
        Tile::SecurityLock(kind) => Some(seal_plate_fill(kind)),
        Tile::InfoPanel(_) => Some("rgba(255,236,200,0.10)"),
        Tile::OreVein => Some("rgba(255,215,0,0.16)"),
        Tile::PlasmaVent => Some("rgba(255,69,0,0.18)"),
        Tile::DataRack => Some("rgba(196,154,108,0.14)"),
        Tile::WarpGatePortal => Some("rgba(255,68,170,0.18)"),
        Tile::MedBayTile => Some("rgba(102,255,221,0.16)"),
        Tile::CreditCache => Some("rgba(255,215,0,0.16)"),
        Tile::CrystalPanel => Some("rgba(170,170,255,0.14)"),
        _ => None,
    }
}

fn altar_fill(kind: TerminalKind) -> &'static str {
    match kind {
        TerminalKind::Quantum => "#30563f",
        TerminalKind::Stellar => "#334d74",
        TerminalKind::Holographic => "#5a456e",
        TerminalKind::Tactical => "#4a4a4a",
        TerminalKind::Commerce => "#665522",
    }
}

fn altar_revealed_fill(kind: TerminalKind) -> &'static str {
    match kind {
        TerminalKind::Quantum => "#214231",
        TerminalKind::Stellar => "#243b56",
        TerminalKind::Holographic => "#443255",
        TerminalKind::Tactical => "#333333",
        TerminalKind::Commerce => "#443a1a",
    }
}

fn altar_plate_fill(kind: TerminalKind) -> &'static str {
    match kind {
        TerminalKind::Quantum => "rgba(102,221,153,0.14)",
        TerminalKind::Stellar => "rgba(136,204,255,0.14)",
        TerminalKind::Holographic => "rgba(221,184,255,0.14)",
        TerminalKind::Tactical => "rgba(200,200,200,0.14)",
        TerminalKind::Commerce => "rgba(255,215,0,0.14)",
    }
}

fn seal_fill(kind: SealKind) -> &'static str {
    match kind {
        SealKind::Thermal => "#6a3529",
        SealKind::Hydraulic => "#264d79",
        SealKind::Kinetic => "#5f3144",
        SealKind::Sonic => "#4f3a68",
    }
}

fn seal_revealed_fill(kind: SealKind) -> &'static str {
    match kind {
        SealKind::Thermal => "#44251d",
        SealKind::Hydraulic => "#1b3652",
        SealKind::Kinetic => "#412230",
        SealKind::Sonic => "#352646",
    }
}

fn seal_plate_fill(kind: SealKind) -> &'static str {
    match kind {
        SealKind::Thermal => "rgba(255,155,115,0.16)",
        SealKind::Hydraulic => "rgba(144,201,255,0.16)",
        SealKind::Kinetic => "rgba(255,158,184,0.14)",
        SealKind::Sonic => "rgba(212,164,255,0.16)",
    }
}

fn tile_glyph_font(tile: Tile) -> &'static str {
    match tile {
        Tile::SecurityLock(_) => "bold 14px 'Noto Serif SC', 'SimSun', serif",
        Tile::SalvageCrate | Tile::LaserGrid | Tile::Coolant | Tile::CoolantPool | Tile::VacuumBreach => "15px monospace",
        _ => "16px monospace",
    }
}

fn tile_glyph_y(tile: Tile, screen_y: f64, anim_t: f64, tx: i32, ty: i32) -> f64 {
    let base = screen_y + TILE_SIZE * 0.75;
    match tile {
        Tile::CoolantPool | Tile::VacuumBreach => {
            base + (anim_t * 3.5 + tx as f64 * 0.6 + ty as f64 * 0.35).sin() * 1.4
        }
        Tile::Coolant => base + (anim_t * 2.0 + tx as f64 * 0.4).sin() * 0.6,
        Tile::CircuitShrine => base + (anim_t * 2.5).sin() * 0.9,
        Tile::RadicalLab => base + (anim_t * 2.7 + tx as f64 * 0.3).sin() * 0.8,
        Tile::FrequencyWall => base + (anim_t * 3.0 + ty as f64 * 0.3).sin() * 0.7,
        Tile::CompoundShrine => base + (anim_t * 2.4 + tx as f64 * 0.2).sin() * 0.8,
        Tile::ClassifierNode => base + (anim_t * 2.6 + ty as f64 * 0.25).sin() * 0.7,
        Tile::DataWell => base + (anim_t * 2.3 + tx as f64 * 0.25).sin() * 0.7,
        Tile::MemorialNode => base + (anim_t * 2.9 + ty as f64 * 0.35).sin() * 0.8,
        Tile::TranslationTerminal => base + (anim_t * 2.5 + tx as f64 * 0.3).sin() * 0.75,
        Tile::HoloPool => base + (anim_t * 3.2 + ty as f64 * 0.4).sin() * 1.0,
        Tile::DroidTutor => base + (anim_t * 2.0 + tx as f64 * 0.15).sin() * 0.6,
        Tile::CodexTerminal => base + (anim_t * 2.4 + tx as f64 * 0.2).sin() * 0.8,
        Tile::DataBridge => base + (anim_t * 2.6 + ty as f64 * 0.3).sin() * 0.7,
        Tile::SealedHatch => base + (anim_t * 1.8 + tx as f64 * 0.1).sin() * 0.5,
        Tile::Terminal(_) => base + (anim_t * 2.8 + ty as f64 * 0.4).sin() * 0.8,
        Tile::SecurityLock(_) => base + (anim_t * 3.1 + tx as f64 * 0.35 + ty as f64 * 0.2).sin() * 0.7,
        Tile::Airlock => base + (anim_t * 1.8).sin() * 0.4,
        Tile::PlasmaVent => base + (anim_t * 3.0 + tx as f64 * 0.5 + ty as f64 * 0.3).sin() * 1.2,
        Tile::MedBayTile => base + (anim_t * 2.5 + tx as f64 * 0.3).sin() * 0.9,
        Tile::WarpGatePortal => base + (anim_t * 3.5 + ty as f64 * 0.4).sin() * 1.1,
        Tile::CreditCache => base + (anim_t * 1.5).sin() * 0.3,
        Tile::ToxicFungus => base + (anim_t * 2.0 + tx as f64 * 0.2).sin() * 0.5,
        Tile::ToxicGas => base + (anim_t * 2.8 + tx as f64 * 0.4 + ty as f64 * 0.3).sin() * 0.8,
        _ => base,
    }
}

fn tile_pattern_seed(tx: i32, ty: i32) -> u32 {
    (tx as u32)
        .wrapping_mul(73_856_093)
        .wrapping_add((ty as u32).wrapping_mul(19_349_663))
        ^ 0x9e37_79b9
}


#[cfg(test)]
mod tests;
