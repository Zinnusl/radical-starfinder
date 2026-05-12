# Radical Starfinder Missing Asset Pack

This pack contains generated 32px pixel-art assets intended for `Zinnusl/radical-starfinder`.

## What is included

- Player ship interior tiles and a complete sample ship map.
- Expanded 64-tile sheets for space station, asteroid base, derelict ship, alien ruins, trading post, orbital platform, mining colony, research lab, and space exterior.
- Individual combat terrain sprites for every current `BattleTile` variant.
- Player-ship object sprites, UI icons, effect animation strips, and scene backgrounds.
- `implementation_plan.csv`: per-asset instructions with target repo paths and suggested code hooks.
- `tileset_index.csv`: tile IDs, source rectangles, suggested keys, walkability, and LOS hints.

## Quick implementation path

1. Copy the `drop_in/assets/...` directories into the repo root.
2. For immediate visual coverage, use the individual PNGs and paste relevant lines from `docs/spritecache_register_snippet.rs` into `src/sprites.rs`.
3. For richer area art, copy `tilesheets/*.png` to `assets/sprites/tilesets/` and load/crop tiles using `tileset_index.csv`.
4. Copy `maps/*.csv` to `data/maps/` only if adding fixed/prototype maps. The CSV values are 32px tile IDs from the listed tilesheet.
5. In the combat arena renderer, map `BattleTile::*` variants to `combat_*` sprite keys or crop from `combat_battle_tiles_32px.png`.

## Pixel-art assumptions

- All tile, object, icon, and effect frames are 32×32 unless explicitly marked otherwise.
- Sprites are RGBA PNGs; object/icon/effect assets have transparent backgrounds.
- Draw with nearest-neighbor scaling to preserve crisp pixels.
