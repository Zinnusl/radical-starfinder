#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SpriteAsset {
    pub(crate) key: String,
    pub(crate) path: String,
}

const PLAYER_SHIP_TILE_SUFFIXES: &[&str] = &[
    "floor",
    "wall",
    "corridor",
    "door",
    "door_open",
    "airlock",
    "window",
    "bridge_floor",
    "engine_room_floor",
    "reactor_floor",
    "medbay_floor",
    "lab_floor",
    "cargo_floor",
    "crew_quarters_floor",
    "stairs_down",
    "warp_gate_pad",
];

const EXPANDED_LOCATION_TILE_SUFFIXES: &[&str] = &[
    "door",
    "door_open",
    "airlock",
    "window",
    "variant_floor_panel",
    "variant_wall_reinforced",
    "hazard_floor_ns",
    "hazard_floor_ew",
    "glyph_floor",
    "shop_floor",
    "stairs_down",
    "warp_gate_pad",
];

const EXPANDED_LOCATIONS: &[(&str, &str, &str)] = &[
    ("space_station", "space_station_expansion", "space_station"),
    ("asteroid_base", "asteroid_base", "asteroid_base"),
    ("derelict_ship", "derelict_ship", "derelict_ship"),
    ("alien_ruins", "alien_ruins", "alien_ruins"),
    ("trading_post", "trading_post", "trading_post"),
    ("orbital_platform", "orbital_platform", "orbital_platform"),
    ("mining_colony", "mining_colony", "mining_colony"),
    ("research_lab", "research_lab", "research_lab"),
    ("space_exterior", "space_exterior", "space_exterior"),
];

const COMBAT_TILE_SUFFIXES: &[&str] = &[
    "metal_floor",
    "cover_barrier",
    "wiring_panel",
    "coolant_pool",
    "frozen_coolant",
    "blast_mark",
    "oil_slick",
    "damaged_plating",
    "vent_steam",
    "plasma_pool",
    "electrified_wire",
    "holo_trap",
    "debris",
    "pipe_tangle",
    "cryo_zone",
    "energy_node",
    "power_drain",
    "charging_pad",
    "gravity_trap",
    "cargo_crate",
    "conveyor_n",
    "conveyor_s",
    "conveyor_e",
    "conveyor_w",
    "fuel_canister",
    "weakened_plating",
    "damaged_floor",
    "breached_floor",
    "mine_tile",
    "mine_tile_revealed",
    "lubricant",
    "shield_zone",
    "elevated_platform",
    "gravity_well",
    "steam_vent_active",
    "steam_vent_inactive",
    "energy_vent_dormant",
    "energy_vent_charging",
    "energy_vent_active",
];

const PLAYER_SHIP_OBJECT_SUFFIXES: &[&str] = &[
    "captains_chair",
    "nav_console",
    "pilot_console",
    "engineering_console",
    "reactor_console",
    "crew_bunk",
    "locker",
    "oxygen_plant",
    "galley_table",
    "med_scanner",
    "lab_microscope",
    "specimen_tank",
    "cargo_palette",
    "fuel_tank",
    "server_rack",
    "starmap_projector",
    "radical_core",
    "quantum_glyph_analyzer",
    "airlock_panel",
    "repair_bot_dock",
    "escape_pod_docked",
    "trophy_case",
    "hydroponic_planter",
    "training_dummy",
];

const UI_ICON_SUFFIXES: &[&str] = &[
    "hull_integrity",
    "oxygen",
    "energy",
    "credits",
    "fuel",
    "map",
    "quest",
    "radical_core",
    "glyph_shard",
    "codex",
    "inventory",
    "shields",
    "danger",
    "shop",
    "crew",
    "warp",
    "medkit",
    "keycard",
    "language",
    "settings",
];

const EFFECT_SPRITES: &[(&str, &str)] = &[
    ("laser_bolt", "laser_bolt_4f"),
    ("plasma_bolt", "plasma_bolt_4f"),
    ("shield_ring", "shield_ring_6f"),
    ("explosion", "explosion_6f"),
    ("steam_puff", "steam_puff_5f"),
    ("coolant_splash", "coolant_splash_4f"),
    ("warp_flash", "warp_flash_6f"),
    ("glyph_resonance", "glyph_resonance_6f"),
    ("spark_arc", "spark_arc_4f"),
    ("dust_debris", "dust_debris_4f"),
];

const BACKGROUND_SPRITES: &[(&str, &str)] = &[
    ("space", "space_background_640x360"),
    ("hangar", "hangar_background_640x360"),
    ("bridge", "bridge_background_640x360"),
];

const TILESET_SPRITES: &[(&str, &str)] = &[
    ("player_ship", "player_ship_tileset_32px"),
    (
        "space_station_expansion",
        "space_station_expansion_tileset_32px",
    ),
    ("asteroid_base", "asteroid_base_tileset_32px"),
    ("derelict_ship", "derelict_ship_tileset_32px"),
    ("alien_ruins", "alien_ruins_tileset_32px"),
    ("trading_post", "trading_post_tileset_32px"),
    ("orbital_platform", "orbital_platform_tileset_32px"),
    ("mining_colony", "mining_colony_tileset_32px"),
    ("research_lab", "research_lab_tileset_32px"),
    ("space_exterior", "space_exterior_tileset_32px"),
    ("combat_battle_tiles", "combat_battle_tiles_32px"),
];

/// Preconditions: the missing-assets pack has been copied to the repo paths
/// encoded in this catalog.
/// Postconditions: returns every generated SpriteCache asset definition the
/// render layer may request, with repo-relative PNG paths and stable keys.
pub(crate) fn generated_sprite_assets() -> Vec<SpriteAsset> {
    player_ship_location_assets()
        .into_iter()
        .chain(expanded_location_assets())
        .chain(combat_tile_assets())
        .chain(player_ship_object_assets())
        .chain(ui_icon_assets())
        .chain(effect_assets())
        .chain(background_assets())
        .chain(tileset_assets())
        .chain(ui_tilesheet_assets())
        .collect()
}

/// Preconditions: none.
/// Postconditions: returns SpriteCache entries for every standalone player
/// ship location tile in the asset pack.
fn player_ship_location_assets() -> Vec<SpriteAsset> {
    PLAYER_SHIP_TILE_SUFFIXES
        .iter()
        .map(|suffix| SpriteAsset {
            key: format!("loc_player_ship_{suffix}"),
            path: format!("assets/sprites/tiles/locations/player_ship/{suffix}.png"),
        })
        .collect()
}

/// Preconditions: none.
/// Postconditions: returns SpriteCache entries for every expanded location
/// tile variant copied from the asset pack.
fn expanded_location_assets() -> Vec<SpriteAsset> {
    EXPANDED_LOCATIONS
        .iter()
        .flat_map(|(_, key_prefix, path_prefix)| {
            EXPANDED_LOCATION_TILE_SUFFIXES
                .iter()
                .map(move |suffix| SpriteAsset {
                    key: format!("loc_{key_prefix}_{suffix}"),
                    path: format!(
                        "assets/sprites/tiles/locations/{path_prefix}/expanded/{suffix}.png"
                    ),
                })
        })
        .collect()
}

/// Preconditions: none.
/// Postconditions: returns SpriteCache entries for every tactical combat
/// BattleTile sprite supplied as an individual PNG.
fn combat_tile_assets() -> Vec<SpriteAsset> {
    COMBAT_TILE_SUFFIXES
        .iter()
        .map(|suffix| SpriteAsset {
            key: format!("combat_{suffix}"),
            path: format!("assets/sprites/tiles/combat/{suffix}.png"),
        })
        .collect()
}

/// Preconditions: none.
/// Postconditions: returns SpriteCache entries for every transparent player
/// ship object sprite.
fn player_ship_object_assets() -> Vec<SpriteAsset> {
    PLAYER_SHIP_OBJECT_SUFFIXES
        .iter()
        .map(|suffix| SpriteAsset {
            key: format!("obj_ship_{suffix}"),
            path: format!("assets/sprites/objects/player_ship/{suffix}.png"),
        })
        .collect()
}

/// Preconditions: none.
/// Postconditions: returns SpriteCache entries for every HUD/UI icon.
fn ui_icon_assets() -> Vec<SpriteAsset> {
    UI_ICON_SUFFIXES
        .iter()
        .map(|suffix| SpriteAsset {
            key: format!("ui_icon_{suffix}"),
            path: format!("assets/sprites/ui/icons/{suffix}.png"),
        })
        .collect()
}

/// Preconditions: none.
/// Postconditions: returns SpriteCache entries for every effect animation
/// strip, keyed without the frame-count suffix.
fn effect_assets() -> Vec<SpriteAsset> {
    EFFECT_SPRITES
        .iter()
        .map(|(key, file)| SpriteAsset {
            key: format!("effect_{key}"),
            path: format!("assets/sprites/effects/{file}.png"),
        })
        .collect()
}

/// Preconditions: none.
/// Postconditions: returns SpriteCache entries for scene backgrounds.
fn background_assets() -> Vec<SpriteAsset> {
    BACKGROUND_SPRITES
        .iter()
        .map(|(key, file)| SpriteAsset {
            key: format!("bg_{key}"),
            path: format!("assets/sprites/backgrounds/{file}.png"),
        })
        .collect()
}

/// Preconditions: none.
/// Postconditions: returns SpriteCache entries for 32px tilesheets copied to
/// `assets/sprites/tilesets`.
fn tileset_assets() -> Vec<SpriteAsset> {
    TILESET_SPRITES
        .iter()
        .map(|(key, file)| SpriteAsset {
            key: format!("tileset_{key}"),
            path: format!("assets/sprites/tilesets/{file}.png"),
        })
        .collect()
}

/// Preconditions: none.
/// Postconditions: returns SpriteCache entries for non-location sprite sheets
/// that intentionally live outside `assets/sprites/tilesets`.
fn ui_tilesheet_assets() -> Vec<SpriteAsset> {
    [SpriteAsset {
        key: "tileset_ui_icon_sheet".to_string(),
        path: "assets/sprites/ui/icon_sheet_32px.png".to_string(),
    }]
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preconditions: the catalog builder is callable in unit tests.
    /// Postconditions: verifies representative high-priority pack groups are
    /// exposed through generated SpriteCache keys.
    #[test]
    fn generated_sprite_assets_include_pack_priority_groups() {
        let assets = generated_sprite_assets();

        [
            "loc_player_ship_floor",
            "loc_player_ship_warp_gate_pad",
            "combat_metal_floor",
            "combat_energy_vent_active",
            "obj_ship_starmap_projector",
            "ui_icon_hull_integrity",
            "effect_explosion",
            "bg_bridge",
            "tileset_player_ship",
        ]
        .into_iter()
        .for_each(|key| {
            assert!(
                assets.iter().any(|asset| asset.key == key),
                "missing generated asset key {key}"
            );
        });
    }

    /// Preconditions: the generated catalog contains all copied sprite groups.
    /// Postconditions: proves SpriteCache registration cannot silently
    /// overwrite a duplicate generated key.
    #[test]
    fn generated_sprite_assets_have_unique_keys() {
        let assets = generated_sprite_assets();

        assets.iter().enumerate().for_each(|(idx, asset)| {
            assert!(
                assets
                    .iter()
                    .skip(idx + 1)
                    .all(|other| other.key != asset.key),
                "duplicate generated asset key {}",
                asset.key
            );
        });
    }

    /// Preconditions: generated asset entries are built from hard-coded pack
    /// metadata.
    /// Postconditions: every entry is browser-loadable by repo-relative PNG
    /// path convention.
    #[test]
    fn generated_sprite_asset_paths_are_repo_relative_pngs() {
        generated_sprite_assets().into_iter().for_each(|asset| {
            assert!(
                asset.path.starts_with("assets/sprites/"),
                "path must be repo-relative asset path: {}",
                asset.path
            );
            assert!(
                asset.path.ends_with(".png"),
                "sprite assets must point to png files: {}",
                asset.path
            );
        });
    }

    /// Preconditions: implementation_plan.csv concrete targets have been
    /// copied from the missing-assets pack into the repo.
    /// Postconditions: every generated SpriteCache entry points at a file that
    /// exists in the working tree.
    #[test]
    fn generated_sprite_asset_files_exist_in_repo() {
        generated_sprite_assets().into_iter().for_each(|asset| {
            assert!(
                std::path::Path::new(&asset.path).is_file(),
                "missing copied asset file for {} at {}",
                asset.key,
                asset.path
            );
        });
    }
}
