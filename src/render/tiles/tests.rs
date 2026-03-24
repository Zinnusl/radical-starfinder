use super::{tile_palette, TilePalette};
use crate::world::{TerminalKind, Tile};

#[test]
fn tile_palette_highlights_interactive_tiles_when_visible() {
    let stairs = tile_palette(Tile::Airlock, true);

    assert_eq!(
        stairs,
        TilePalette {
            fill: "#8ab4ff",
            accent: Some("#d7e7ff"),
            glyph: Some("▼"),
            glyph_color: "#ffffff",
        }
    );
}

#[test]
fn tile_palette_keeps_special_tiles_distinct_when_revealed() {
    let revealed_chest = tile_palette(Tile::SupplyCrate, false);
    let revealed_floor = tile_palette(Tile::MetalFloor, false);
    let revealed_altar = tile_palette(Tile::Terminal(TerminalKind::Quantum), false);

    assert_eq!(revealed_chest.fill, "#5a441b");
    assert_ne!(revealed_chest.fill, revealed_floor.fill);
    assert_eq!(revealed_altar.fill, "#214231");
}

