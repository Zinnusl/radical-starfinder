# Implementation instructions for coding agent

Use `implementation_plan.csv` as the canonical task list. Minimum useful integration:

1. Copy all files under `drop_in/assets/` to the repository's `assets/` folder.
2. Add the player ship location in world/location generation if not present.
3. Add SpriteCache keys for `loc_player_ship_*`, `combat_*`, `obj_ship_*`, `ui_icon_*`, and `effect_*` as needed.
4. Extend render code so combat `BattleTile` variants look up sprite keys before falling back to colored rectangles.
5. For tilesheets, implement a small 32px crop helper and use `tileset_index.csv` for tile ID lookup.
6. For maps, add a CSV loader for `data/maps/*.csv`, validate rectangular rows, and map integer tile IDs to the corresponding tilesheet.
7. Keep existing single-file location tiles working; this pack does not require deleting or overwriting existing repo assets.

Recommended priority order:

- High: player ship standalone tiles, combat terrain sprites, SpriteCache registration snippet.
- Medium: player ship map, expanded area tilesheets, object sprites, UI icons/effects.
- Low: backgrounds and optional sample encounter maps.
