use std::collections::HashMap;

use web_sys::HtmlImageElement;

pub struct SpriteCache {
    sprites: HashMap<&'static str, HtmlImageElement>,
}

impl SpriteCache {
    pub fn new() -> Self {
        let mut sprites = HashMap::new();

        fn register(
            sprites: &mut HashMap<&'static str, HtmlImageElement>,
            key: &'static str,
            path: &'static str,
        ) {
            if let Ok(img) = HtmlImageElement::new() {
                img.set_src(path);
                sprites.insert(key, img);
            }
        }

        register(&mut sprites, "tile_wall", "assets/sprites/tiles/wall.png");
        register(
            &mut sprites,
            "tile_cracked_wall",
            "assets/sprites/tiles/cracked_wall.png",
        );
        register(
            &mut sprites,
            "tile_brittle_wall",
            "assets/sprites/tiles/brittle_wall.png",
        );
        register(&mut sprites, "tile_floor", "assets/sprites/tiles/floor.png");
        register(
            &mut sprites,
            "tile_corridor",
            "assets/sprites/tiles/corridor.png",
        );
        register(
            &mut sprites,
            "tile_stairs_down",
            "assets/sprites/tiles/stairs_down.png",
        );
        register(&mut sprites, "tile_forge", "assets/sprites/tiles/forge.png");
        register(&mut sprites, "tile_shop", "assets/sprites/tiles/shop.png");
        register(&mut sprites, "tile_chest", "assets/sprites/tiles/chest.png");
        register(&mut sprites, "tile_crate", "assets/sprites/tiles/crate.png");
        register(
            &mut sprites,
            "tile_spikes",
            "assets/sprites/tiles/spikes.png",
        );
        register(&mut sprites, "tile_oil", "assets/sprites/tiles/oil.png");
        register(&mut sprites, "tile_water", "assets/sprites/tiles/water.png");
        register(
            &mut sprites,
            "tile_deep_water",
            "assets/sprites/tiles/deep_water.png",
        );
        register(
            &mut sprites,
            "tile_bridge",
            "assets/sprites/tiles/bridge.png",
        );

        register(
            &mut sprites,
            "obj_shrine",
            "assets/sprites/objects/shrine.png",
        );
        register(
            &mut sprites,
            "obj_altar_jade",
            "assets/sprites/objects/altar_jade.png",
        );
        register(
            &mut sprites,
            "obj_altar_gale",
            "assets/sprites/objects/altar_gale.png",
        );
        register(
            &mut sprites,
            "obj_altar_mirror",
            "assets/sprites/objects/altar_mirror.png",
        );
        register(
            &mut sprites,
            "obj_altar_iron",
            "assets/sprites/objects/altar_iron.png",
        );
        register(
            &mut sprites,
            "obj_altar_gold",
            "assets/sprites/objects/altar_gold.png",
        );
        register(
            &mut sprites,
            "obj_seal_ember",
            "assets/sprites/objects/seal_ember.png",
        );
        register(
            &mut sprites,
            "obj_seal_tide",
            "assets/sprites/objects/seal_tide.png",
        );
        register(
            &mut sprites,
            "obj_seal_thorn",
            "assets/sprites/objects/seal_thorn.png",
        );
        register(
            &mut sprites,
            "obj_seal_echo",
            "assets/sprites/objects/seal_echo.png",
        );
        register(&mut sprites, "obj_sign", "assets/sprites/objects/sign.png");

        register(
            &mut sprites,
            "player_human",
            "assets/sprites/player/human.png",
        );
        register(
            &mut sprites,
            "player_flame",
            "assets/sprites/player/flame.png",
        );
        register(
            &mut sprites,
            "player_stone",
            "assets/sprites/player/stone.png",
        );
        register(
            &mut sprites,
            "player_mist",
            "assets/sprites/player/mist.png",
        );
        register(
            &mut sprites,
            "player_tiger",
            "assets/sprites/player/tiger.png",
        );

        register(
            &mut sprites,
            "npc_teacher",
            "assets/sprites/npcs/teacher.png",
        );
        register(&mut sprites, "npc_monk", "assets/sprites/npcs/monk.png");
        register(
            &mut sprites,
            "npc_merchant",
            "assets/sprites/npcs/merchant.png",
        );
        register(&mut sprites, "npc_guard", "assets/sprites/npcs/guard.png");

        register(
            &mut sprites,
            "boss_gatekeeper",
            "assets/sprites/bosses/gatekeeper.png",
        );
        register(
            &mut sprites,
            "boss_scholar",
            "assets/sprites/bosses/scholar.png",
        );
        register(
            &mut sprites,
            "boss_elementalist",
            "assets/sprites/bosses/elementalist.png",
        );
        register(
            &mut sprites,
            "boss_mimic_king",
            "assets/sprites/bosses/mimic_king.png",
        );
        register(
            &mut sprites,
            "boss_ink_sage",
            "assets/sprites/bosses/ink_sage.png",
        );
        register(
            &mut sprites,
            "boss_radical_thief",
            "assets/sprites/bosses/radical_thief.png",
        );

        register(
            &mut sprites,
            "enemy_generic",
            "assets/sprites/enemies/generic.png",
        );
        register(
            &mut sprites,
            "enemy_elite",
            "assets/sprites/enemies/elite.png",
        );

        register(
            &mut sprites,
            "item_health_potion",
            "assets/sprites/items/health_potion.png",
        );
        register(
            &mut sprites,
            "item_poison_flask",
            "assets/sprites/items/poison_flask.png",
        );
        register(
            &mut sprites,
            "item_reveal_scroll",
            "assets/sprites/items/reveal_scroll.png",
        );
        register(
            &mut sprites,
            "item_teleport_scroll",
            "assets/sprites/items/teleport_scroll.png",
        );
        register(
            &mut sprites,
            "item_haste_potion",
            "assets/sprites/items/haste_potion.png",
        );
        register(
            &mut sprites,
            "item_stun_bomb",
            "assets/sprites/items/stun_bomb.png",
        );
        register(
            &mut sprites,
            "item_meditation_incense",
            "assets/sprites/items/meditation_incense.png",
        );
        register(
            &mut sprites,
            "item_ancestral_wine",
            "assets/sprites/items/ancestral_wine.png",
        );
        register(
            &mut sprites,
            "item_rice_ball",
            "assets/sprites/items/rice_ball.png",
        );
        register(
            &mut sprites,
            "item_smoke_screen",
            "assets/sprites/items/smoke_screen.png",
        );
        register(
            &mut sprites,
            "item_fire_cracker",
            "assets/sprites/items/fire_cracker.png",
        );
        register(
            &mut sprites,
            "item_iron_skin_elixir",
            "assets/sprites/items/iron_skin_elixir.png",
        );
        register(
            &mut sprites,
            "item_clarity_tea",
            "assets/sprites/items/clarity_tea.png",
        );
        register(
            &mut sprites,
            "item_gold_ingot",
            "assets/sprites/items/gold_ingot.png",
        );
        register(
            &mut sprites,
            "item_thunder_talisman",
            "assets/sprites/items/thunder_talisman.png",
        );
        register(
            &mut sprites,
            "item_jade_salve",
            "assets/sprites/items/jade_salve.png",
        );
        register(
            &mut sprites,
            "item_serpent_fang",
            "assets/sprites/items/serpent_fang.png",
        );
        register(
            &mut sprites,
            "item_warding_charm",
            "assets/sprites/items/warding_charm.png",
        );
        register(
            &mut sprites,
            "item_ink_bomb",
            "assets/sprites/items/ink_bomb.png",
        );
        register(
            &mut sprites,
            "item_phoenix_plume",
            "assets/sprites/items/phoenix_plume.png",
        );
        register(
            &mut sprites,
            "item_mirror_shard",
            "assets/sprites/items/mirror_shard.png",
        );
        register(
            &mut sprites,
            "item_frost_vial",
            "assets/sprites/items/frost_vial.png",
        );
        register(
            &mut sprites,
            "item_shadow_cloak",
            "assets/sprites/items/shadow_cloak.png",
        );
        register(
            &mut sprites,
            "item_dragon_scale",
            "assets/sprites/items/dragon_scale.png",
        );
        register(
            &mut sprites,
            "item_bamboo_flute",
            "assets/sprites/items/bamboo_flute.png",
        );
        register(
            &mut sprites,
            "item_jade_compass",
            "assets/sprites/items/jade_compass.png",
        );
        register(
            &mut sprites,
            "item_silk_rope",
            "assets/sprites/items/silk_rope.png",
        );
        register(
            &mut sprites,
            "item_lotus_elixir",
            "assets/sprites/items/lotus_elixir.png",
        );
        register(
            &mut sprites,
            "item_thunder_drum",
            "assets/sprites/items/thunder_drum.png",
        );
        register(
            &mut sprites,
            "item_cinnabar_ink",
            "assets/sprites/items/cinnabar_ink.png",
        );
        register(
            &mut sprites,
            "item_ancestor_token",
            "assets/sprites/items/ancestor_token.png",
        );
        register(
            &mut sprites,
            "item_wind_fan",
            "assets/sprites/items/wind_fan.png",
        );

        register(
            &mut sprites,
            "equip_brush_of_clarity",
            "assets/sprites/equipment/brush_of_clarity.png",
        );
        register(
            &mut sprites,
            "equip_scholars_quill",
            "assets/sprites/equipment/scholars_quill.png",
        );
        register(
            &mut sprites,
            "equip_dragon_fang_pen",
            "assets/sprites/equipment/dragon_fang_pen.png",
        );
        register(
            &mut sprites,
            "equip_iron_pickaxe",
            "assets/sprites/equipment/iron_pickaxe.png",
        );
        register(
            &mut sprites,
            "equip_jade_vest",
            "assets/sprites/equipment/jade_vest.png",
        );
        register(
            &mut sprites,
            "equip_iron_silk_robe",
            "assets/sprites/equipment/iron_silk_robe.png",
        );
        register(
            &mut sprites,
            "equip_phoenix_mantle",
            "assets/sprites/equipment/phoenix_mantle.png",
        );
        register(
            &mut sprites,
            "equip_radical_magnet",
            "assets/sprites/equipment/radical_magnet.png",
        );
        register(
            &mut sprites,
            "equip_life_jade",
            "assets/sprites/equipment/life_jade.png",
        );
        register(
            &mut sprites,
            "equip_gold_toad",
            "assets/sprites/equipment/gold_toad.png",
        );
        register(
            &mut sprites,
            "equip_phoenix_feather",
            "assets/sprites/equipment/phoenix_feather.png",
        );

        // Arena tile sprites (biome floors)
        register(
            &mut sprites,
            "arena_floor_stone",
            "assets/sprites/tiles/arena_floor_stone.png",
        );
        register(
            &mut sprites,
            "arena_floor_dark",
            "assets/sprites/tiles/arena_floor_dark.png",
        );
        register(
            &mut sprites,
            "arena_floor_arcane",
            "assets/sprites/tiles/arena_floor_arcane.png",
        );
        register(
            &mut sprites,
            "arena_floor_cursed",
            "assets/sprites/tiles/arena_floor_cursed.png",
        );
        // Arena tile sprites (biome obstacles)
        register(
            &mut sprites,
            "arena_obstacle_stone",
            "assets/sprites/tiles/arena_obstacle_stone.png",
        );
        register(
            &mut sprites,
            "arena_obstacle_dark",
            "assets/sprites/tiles/arena_obstacle_dark.png",
        );
        register(
            &mut sprites,
            "arena_obstacle_arcane",
            "assets/sprites/tiles/arena_obstacle_arcane.png",
        );
        register(
            &mut sprites,
            "arena_obstacle_cursed",
            "assets/sprites/tiles/arena_obstacle_cursed.png",
        );
        register(
            &mut sprites,
            "arena_floor_garden",
            "assets/sprites/tiles/arena_floor_garden.png",
        );
        register(
            &mut sprites,
            "arena_floor_frozen",
            "assets/sprites/tiles/arena_floor_frozen.png",
        );
        register(
            &mut sprites,
            "arena_floor_infernal",
            "assets/sprites/tiles/arena_floor_infernal.png",
        );
        register(
            &mut sprites,
            "arena_obstacle_garden",
            "assets/sprites/tiles/arena_obstacle_garden.png",
        );
        register(
            &mut sprites,
            "arena_obstacle_frozen",
            "assets/sprites/tiles/arena_obstacle_frozen.png",
        );
        register(
            &mut sprites,
            "arena_obstacle_infernal",
            "assets/sprites/tiles/arena_obstacle_infernal.png",
        );
        // Arena tile sprites (special terrain)
        register(
            &mut sprites,
            "arena_grass",
            "assets/sprites/tiles/arena_grass.png",
        );
        register(
            &mut sprites,
            "arena_water",
            "assets/sprites/tiles/arena_water.png",
        );
        register(
            &mut sprites,
            "arena_ice",
            "assets/sprites/tiles/arena_ice.png",
        );
        register(
            &mut sprites,
            "arena_scorched",
            "assets/sprites/tiles/arena_scorched.png",
        );
        register(
            &mut sprites,
            "arena_ink_pool",
            "assets/sprites/tiles/arena_ink_pool.png",
        );
        register(
            &mut sprites,
            "arena_broken_ground",
            "assets/sprites/tiles/arena_broken_ground.png",
        );
        register(
            &mut sprites,
            "arena_steam",
            "assets/sprites/tiles/arena_steam.png",
        );
        register(
            &mut sprites,
            "arena_lava",
            "assets/sprites/tiles/arena_lava.png",
        );
        register(
            &mut sprites,
            "arena_thorns",
            "assets/sprites/tiles/arena_thorns.png",
        );
        register(
            &mut sprites,
            "arena_arcane_glyph",
            "assets/sprites/tiles/arena_arcane_glyph.png",
        );
        register(
            &mut sprites,
            "arena_sand",
            "assets/sprites/tiles/arena_sand.png",
        );
        register(
            &mut sprites,
            "arena_bamboo_thicket",
            "assets/sprites/tiles/arena_bamboo_thicket.png",
        );
        register(
            &mut sprites,
            "arena_frozen_ground",
            "assets/sprites/tiles/arena_frozen_ground.png",
        );
        register(
            &mut sprites,
            "arena_spirit_well",
            "assets/sprites/tiles/arena_spirit_well.png",
        );
        register(
            &mut sprites,
            "arena_spirit_drain",
            "assets/sprites/tiles/arena_spirit_drain.png",
        );
        register(
            &mut sprites,
            "arena_meditation_stone",
            "assets/sprites/tiles/arena_meditation_stone.png",
        );
        register(
            &mut sprites,
            "arena_soul_trap",
            "assets/sprites/tiles/arena_soul_trap.png",
        );

        // ── Location-specific tiles ──────────────────────────────────────
        // Space Station
        register(&mut sprites, "loc_space_station_wall", "assets/sprites/tiles/locations/space_station/wall.png");
        register(&mut sprites, "loc_space_station_floor", "assets/sprites/tiles/locations/space_station/floor.png");
        register(&mut sprites, "loc_space_station_corridor", "assets/sprites/tiles/locations/space_station/corridor.png");
        register(&mut sprites, "loc_space_station_door", "assets/sprites/tiles/locations/space_station/door.png");
        // Asteroid Base
        register(&mut sprites, "loc_asteroid_base_wall", "assets/sprites/tiles/locations/asteroid_base/wall.png");
        register(&mut sprites, "loc_asteroid_base_floor", "assets/sprites/tiles/locations/asteroid_base/floor.png");
        register(&mut sprites, "loc_asteroid_base_corridor", "assets/sprites/tiles/locations/asteroid_base/corridor.png");
        // Derelict Ship
        register(&mut sprites, "loc_derelict_ship_wall", "assets/sprites/tiles/locations/derelict_ship/wall.png");
        register(&mut sprites, "loc_derelict_ship_floor", "assets/sprites/tiles/locations/derelict_ship/floor.png");
        register(&mut sprites, "loc_derelict_ship_corridor", "assets/sprites/tiles/locations/derelict_ship/corridor.png");
        // Alien Ruins
        register(&mut sprites, "loc_alien_ruins_wall", "assets/sprites/tiles/locations/alien_ruins/wall.png");
        register(&mut sprites, "loc_alien_ruins_floor", "assets/sprites/tiles/locations/alien_ruins/floor.png");
        register(&mut sprites, "loc_alien_ruins_corridor", "assets/sprites/tiles/locations/alien_ruins/corridor.png");
        // Trading Post
        register(&mut sprites, "loc_trading_post_wall", "assets/sprites/tiles/locations/trading_post/wall.png");
        register(&mut sprites, "loc_trading_post_floor", "assets/sprites/tiles/locations/trading_post/floor.png");
        register(&mut sprites, "loc_trading_post_corridor", "assets/sprites/tiles/locations/trading_post/corridor.png");
        // Orbital Platform
        register(&mut sprites, "loc_orbital_platform_wall", "assets/sprites/tiles/locations/orbital_platform/wall.png");
        register(&mut sprites, "loc_orbital_platform_floor", "assets/sprites/tiles/locations/orbital_platform/floor.png");
        register(&mut sprites, "loc_orbital_platform_corridor", "assets/sprites/tiles/locations/orbital_platform/corridor.png");
        // Mining Colony
        register(&mut sprites, "loc_mining_colony_wall", "assets/sprites/tiles/locations/mining_colony/wall.png");
        register(&mut sprites, "loc_mining_colony_floor", "assets/sprites/tiles/locations/mining_colony/floor.png");
        register(&mut sprites, "loc_mining_colony_corridor", "assets/sprites/tiles/locations/mining_colony/corridor.png");
        // Research Lab
        register(&mut sprites, "loc_research_lab_wall", "assets/sprites/tiles/locations/research_lab/wall.png");
        register(&mut sprites, "loc_research_lab_floor", "assets/sprites/tiles/locations/research_lab/floor.png");
        register(&mut sprites, "loc_research_lab_corridor", "assets/sprites/tiles/locations/research_lab/corridor.png");

        // ── Space object sprites ─────────────────────────────────────────
        register(&mut sprites, "obj_quantum_forge", "assets/sprites/objects/quantum_forge.png");
        register(&mut sprites, "obj_terminal", "assets/sprites/objects/terminal.png");
        register(&mut sprites, "obj_cargo_crate", "assets/sprites/objects/cargo_crate.png");
        register(&mut sprites, "obj_medbay", "assets/sprites/objects/medbay.png");
        register(&mut sprites, "obj_shield_generator", "assets/sprites/objects/shield_generator.png");
        register(&mut sprites, "obj_weapon_rack", "assets/sprites/objects/weapon_rack.png");
        register(&mut sprites, "obj_alien_artifact", "assets/sprites/objects/alien_artifact.png");
        register(&mut sprites, "obj_reactor_core", "assets/sprites/objects/reactor_core.png");
        register(&mut sprites, "obj_space_shop", "assets/sprites/objects/space_shop.png");
        register(&mut sprites, "obj_plasma_vent", "assets/sprites/objects/plasma_vent.png");
        register(&mut sprites, "obj_data_archive", "assets/sprites/objects/data_archive.png");
        register(&mut sprites, "obj_escape_pod", "assets/sprites/objects/escape_pod.png");
        register(&mut sprites, "obj_warp_gate", "assets/sprites/objects/warp_gate.png");
        register(&mut sprites, "obj_containment_cell", "assets/sprites/objects/containment_cell.png");
        register(&mut sprites, "obj_holo_map", "assets/sprites/objects/holo_map.png");
        register(&mut sprites, "obj_fuel_pump", "assets/sprites/objects/fuel_pump.png");
        register(&mut sprites, "obj_robot_wreck", "assets/sprites/objects/robot_wreck.png");
        register(&mut sprites, "obj_turret", "assets/sprites/objects/turret.png");
        register(&mut sprites, "obj_loot_container", "assets/sprites/objects/loot_container.png");
        register(&mut sprites, "obj_life_support", "assets/sprites/objects/life_support.png");

        // ── Sci-fi stairs/airlock ────────────────────────────────────────
        register(&mut sprites, "tile_stairs_down_scifi", "assets/sprites/tiles/stairs_down_scifi.png");

        // ── Space enemy sprites ──────────────────────────────────────────
        register(&mut sprites, "enemy_space_pirate", "assets/sprites/enemies/space_pirate.png");
        register(&mut sprites, "enemy_rogue_ai", "assets/sprites/enemies/rogue_ai.png");
        register(&mut sprites, "enemy_alien_warrior", "assets/sprites/enemies/alien_warrior.png");

        // ── Location-specific enemy sprites ─────────────────────────────
        register(&mut sprites, "enemy_station_guard", "assets/sprites/enemies/station_guard.png");
        register(&mut sprites, "enemy_maintenance_drone", "assets/sprites/enemies/maintenance_drone.png");
        register(&mut sprites, "enemy_rock_crawler", "assets/sprites/enemies/rock_crawler.png");
        register(&mut sprites, "enemy_asteroid_miner", "assets/sprites/enemies/asteroid_miner.png");
        register(&mut sprites, "enemy_zombie_crew", "assets/sprites/enemies/zombie_crew.png");
        register(&mut sprites, "enemy_hull_parasite", "assets/sprites/enemies/hull_parasite.png");
        register(&mut sprites, "enemy_ruin_sentinel", "assets/sprites/enemies/ruin_sentinel.png");
        register(&mut sprites, "enemy_glyph_phantom", "assets/sprites/enemies/glyph_phantom.png");
        register(&mut sprites, "enemy_smuggler", "assets/sprites/enemies/smuggler.png");
        register(&mut sprites, "enemy_market_thug", "assets/sprites/enemies/market_thug.png");
        register(&mut sprites, "enemy_platform_turret", "assets/sprites/enemies/platform_turret.png");
        register(&mut sprites, "enemy_void_drifter", "assets/sprites/enemies/void_drifter.png");
        register(&mut sprites, "enemy_tunnel_worm", "assets/sprites/enemies/tunnel_worm.png");
        register(&mut sprites, "enemy_gas_specter", "assets/sprites/enemies/gas_specter.png");
        register(&mut sprites, "enemy_lab_mutant", "assets/sprites/enemies/lab_mutant.png");
        register(&mut sprites, "enemy_security_bot", "assets/sprites/enemies/security_bot.png");

        // ── Space boss sprites ───────────────────────────────────────────
        register(&mut sprites, "boss_pirate_captain", "assets/sprites/bosses/pirate_captain.png");
        register(&mut sprites, "boss_hive_queen", "assets/sprites/bosses/hive_queen.png");
        register(&mut sprites, "boss_rogue_ai_core", "assets/sprites/bosses/rogue_ai_core.png");
        register(&mut sprites, "boss_void_entity", "assets/sprites/bosses/void_entity.png");
        register(&mut sprites, "boss_ancient_guardian", "assets/sprites/bosses/ancient_guardian.png");
        register(&mut sprites, "boss_drift_leviathan", "assets/sprites/bosses/drift_leviathan.png");

        // ── Player sprite ────────────────────────────────────────────────
        register(&mut sprites, "player_starfinder", "assets/sprites/player/starfinder.png");

        register(
            &mut sprites,
            "spell_fire",
            "assets/sprites/ui/spell_fire.png",
        );
        register(
            &mut sprites,
            "spell_heal",
            "assets/sprites/ui/spell_heal.png",
        );
        register(
            &mut sprites,
            "spell_reveal",
            "assets/sprites/ui/spell_reveal.png",
        );
        register(
            &mut sprites,
            "spell_shield",
            "assets/sprites/ui/spell_shield.png",
        );
        register(
            &mut sprites,
            "spell_strike",
            "assets/sprites/ui/spell_strike.png",
        );
        register(
            &mut sprites,
            "spell_drain",
            "assets/sprites/ui/spell_drain.png",
        );
        register(
            &mut sprites,
            "spell_stun",
            "assets/sprites/ui/spell_stun.png",
        );
        register(
            &mut sprites,
            "spell_pacify",
            "assets/sprites/ui/spell_pacify.png",
        );

        Self { sprites }
    }

    pub fn get(&self, key: &str) -> Option<&HtmlImageElement> {
        self.sprites.get(key)
    }

    pub fn is_loaded(&self, key: &str) -> bool {
        self.get(key)
            .map(|img| img.complete() && img.natural_width() > 0)
            .unwrap_or(false)
    }
}
