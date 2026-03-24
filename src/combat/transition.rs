use crate::combat::turn::{
    build_turn_queue, enemy_base_movement, enemy_base_speed, player_movement, player_speed,
};
use crate::combat::{
    arena_size_for_encounter, ArenaBiome, BattleTile, BattleUnit, Direction, TacticalArena,
    TacticalBattle, TacticalPhase, UnitKind, Weather, WuxingElement,
};
use crate::world::RoomModifier;
use crate::enemy::{AiBehavior, Enemy};
use crate::game::Companion;
use crate::player::Player;
use crate::srs::SrsTracker;
use crate::vocab::SentenceEntry;

pub fn enter_combat(
    player: &Player,
    enemies: &[Enemy],
    enemy_indices: &[usize],
    floor: i32,
    room_modifier: Option<RoomModifier>,
    srs: &SrsTracker,
    companion: Option<Companion>,
) -> TacticalBattle {
    let has_elite = enemy_indices.iter().any(|&ei| enemies[ei].is_elite);
    let has_boss = enemy_indices.iter().any(|&ei| enemies[ei].is_boss);
    let arena_size = arena_size_for_encounter(has_elite, has_boss);
    let biome = ArenaBiome::from_room_modifier(room_modifier);
    let arena = generate_arena(floor, arena_size, biome);

    let mut units = Vec::new();

    // Player unit at bottom-center.
    let px = (arena_size / 2) as i32;
    let py = (arena_size - 2) as i32;
    let p_speed = player_speed(player.class, player.form, &player.statuses);
    let p_movement = player_movement(player.form, &player.statuses, crate::combat::PlayerStance::Balanced);

    let p_damage = 2 + player.bonus_damage() + player.enchant_bonus_damage();

    units.push(BattleUnit {
        kind: UnitKind::Player,
        x: px,
        y: py,
        facing: Direction::North,
        hanzi: "",
        pinyin: "",
        speed: p_speed,
        movement: p_movement,
        stored_movement: 0,
        hp: player.hp,
        max_hp: player.max_hp,
        damage: p_damage,
        defending: false,
        alive: true,
        ai: AiBehavior::Chase,
        radical_actions: Vec::new(),
        statuses: player.statuses.clone(),
        stunned: false,
        radical_armor: 0,
        radical_counter: false,
        marked_extra_damage: 0,
        thorn_armor_turns: 0,
        radical_dodge: false,
        radical_multiply: false,
        fortify_stacks: 0,
        boss_kind: None,
        is_decoy: false,
        word_group: None,
        word_group_order: 0,
        wuxing_element: None,
        intent: None,
        mastery_tier: 0,
        charge_remaining: None,
        synergy_damage_bonus: 0,
        elemental_resonance: false,
        sacrifice_bonus_damage: 0,
        sacrifice_bonus_turns: 0,
        momentum: 0,
        last_move_dir: None,
    });

    // Companion unit adjacent to player (if any).
    if let Some(companion) = companion {
        let (c_hp, c_damage, c_movement, c_speed, c_hanzi, c_pinyin) = match companion {
            Companion::SecurityChief => (8, 2, 2, 3, "卫", "wèi"),
            Companion::Medic => (6, 1, 2, 3, "医", "yī"),
            Companion::ScienceOfficer => (5, 1, 2, 3, "研", "yán"),
            Companion::Quartermaster => (5, 1, 2, 3, "商", "shāng"),
        };
        let cx = px - 1;
        let cy = py;
        let c_hanzi_static: &'static str = Box::leak(c_hanzi.to_string().into_boxed_str());
        let c_pinyin_static: &'static str = Box::leak(c_pinyin.to_string().into_boxed_str());
        units.push(BattleUnit {
            kind: UnitKind::Companion,
            x: cx,
            y: cy,
            facing: Direction::North,
            hanzi: c_hanzi_static,
            pinyin: c_pinyin_static,
            speed: c_speed,
            movement: c_movement,
            stored_movement: 0,
            hp: c_hp,
            max_hp: c_hp,
            damage: c_damage,
            defending: false,
            alive: true,
            ai: AiBehavior::Chase,
            radical_actions: Vec::new(),
            statuses: Vec::new(),
            stunned: false,
            radical_armor: 0,
            radical_counter: false,
            marked_extra_damage: 0,
            thorn_armor_turns: 0,
            radical_dodge: false,
            radical_multiply: false,
            fortify_stacks: 0,
            boss_kind: None,
            is_decoy: false,
            word_group: None,
            word_group_order: 0,
            wuxing_element: None,
            intent: None,
            mastery_tier: 0,
            charge_remaining: None,
            synergy_damage_bonus: 0,
            elemental_resonance: false,
            sacrifice_bonus_damage: 0,
            sacrifice_bonus_turns: 0,
            momentum: 0,
            last_move_dir: None,
        });
    }
    // Multi-char words are split into one BattleUnit per character.
    let mut word_group_id: usize = 0;

    let total_units: usize = enemy_indices
        .iter()
        .map(|&ei| {
            let e = &enemies[ei];
            crate::vocab::split_hanzi_chars(e.hanzi, e.pinyin).len()
        })
        .sum();

    let mut unit_slot = 0usize;
    for &ei in enemy_indices.iter() {
        let e = &enemies[ei];
        let chars = crate::vocab::split_hanzi_chars(e.hanzi, e.pinyin);
        let char_count = chars.len();
        let is_multi = char_count > 1;

        let e_speed = enemy_base_speed(e.is_elite, e.is_boss);
        let e_movement = enemy_base_movement(e.is_elite, e.is_boss);

        let hp_per = if is_multi {
            (e.hp / char_count as i32).max(1)
        } else {
            e.hp
        };
        let max_hp_per = if is_multi {
            (e.max_hp / char_count as i32).max(1)
        } else {
            e.max_hp
        };
        let dmg_per = if is_multi {
            (e.damage / char_count as i32).max(1)
        } else {
            e.damage
        };

        let group = if is_multi {
            let g = word_group_id;
            word_group_id += 1;
            Some(g)
        } else {
            None
        };

        for (ci, (ch_hanzi, ch_pinyin)) in chars.into_iter().enumerate() {
            let spacing = arena_size as i32 / (total_units as i32 + 1);
            let ex = (spacing * (unit_slot as i32 + 1)).min(arena_size as i32 - 1);
            let ey = 1 + (unit_slot as i32 % 2);

            // Leak the per-char strings so they become &'static str
            // (battle lifetime matches game tick — acceptable for WASM).
            let hanzi_static: &'static str = Box::leak(ch_hanzi.into_boxed_str());
            let pinyin_static: &'static str = Box::leak(ch_pinyin.into_boxed_str());

            units.push(BattleUnit {
                kind: UnitKind::Enemy(ei),
                x: ex,
                y: ey,
                facing: Direction::South,
                hanzi: hanzi_static,
                pinyin: pinyin_static,
                speed: e_speed,
                movement: e_movement,
                stored_movement: 0,
                hp: hp_per,
                max_hp: max_hp_per,
                damage: dmg_per,
                defending: false,
                alive: e.hp > 0,
                ai: e.ai,
                radical_actions: e.radical_actions(),
                statuses: e.statuses.clone(),
                stunned: e.stunned,
                radical_armor: e.radical_armor,
                radical_counter: false,
                marked_extra_damage: 0,
                thorn_armor_turns: 0,
                radical_dodge: e.radical_dodge,
                radical_multiply: e.radical_multiply,
                fortify_stacks: 0,
                boss_kind: e.boss_kind,
                is_decoy: false,
                word_group: group,
                word_group_order: ci as u8,
                wuxing_element: derive_wuxing_element(e),
                intent: None,
                mastery_tier: srs.mastery_tier(hanzi_static),
                charge_remaining: if hanzi_static.chars().count() >= 3 {
                    Some((hanzi_static.chars().count() as u8) - 1)
                } else {
                    None
                },
                synergy_damage_bonus: 0,
                elemental_resonance: false,
                sacrifice_bonus_damage: 0,
                sacrifice_bonus_turns: 0,
                momentum: 0,
                last_move_dir: None,
            });

            let last_idx = units.len() - 1;
            apply_mastery_debuffs(&mut units[last_idx]);

            unit_slot += 1;
        }
    }

    let turn_queue = build_turn_queue(&units);

    let is_boss_battle = enemy_indices.iter().any(|&ei| enemies[ei].is_boss);

    // Determine weather — Clear most of the time, variety on deeper floors.
    let weather = pick_weather(floor, biome);

    let deploy_tiles = compute_deployment_tiles(&arena, &units);
    let initial_phase = if deploy_tiles.len() > 1 {
        let (cx, cy) = deploy_tiles[deploy_tiles.len() / 2];
        TacticalPhase::Deployment {
            cursor_x: cx,
            cursor_y: cy,
            valid_tiles: deploy_tiles,
        }
    } else {
        TacticalPhase::Command
    };

    TacticalBattle {
        arena,
        units,
        turn_queue,
        turn_queue_pos: 0,
        phase: initial_phase,
        turn_number: 1,
        combo_streak: 0,
        player_moved: false,
        player_acted: false,
        player_stance: crate::combat::PlayerStance::Balanced,
        typing_buffer: String::new(),
        typing_action: None,
        log: vec!["Battle begins!".to_string()],
        last_answer: None,
        is_boss_battle,
        available_spells: player
            .spells
            .iter()
            .map(|s| (s.hanzi, s.pinyin, s.effect))
            .collect(),
        spell_cursor: 0,
        spell_menu_open: false,
        spent_spell_index: None,
        ward_tiles: Vec::new(),
        last_spell_school: None,
        last_spell_element: None,
        last_spell_turn: 0,
        combo_message: None,
        combo_message_timer: 0,
        combo_armor_bonus: 0,
        combo_armor_turns: 0,
        frozen_edge_charges: 0,
        stolen_spells: Vec::new(),
        player_class: Some(player.class),
        available_items: player
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| (i, item.clone()))
            .collect(),
        used_item_indices: Vec::new(),
        item_menu_open: false,
        item_cursor: 0,
        weather,
        terrain_tick_count: 0,
        focus: 10,
        max_focus: 10,
        radical_synergy_radical: None,
        radical_synergy_streak: 0,
        chengyu_history: Vec::new(),
        intents_calculated: false,
        player_radical_abilities: player
            .radicals
            .iter()
            .filter_map(|r| {
                crate::enemy::PlayerRadicalAbility::from_radical(r).map(|ability| (*r, ability))
            })
            .collect(),
        consumed_radicals: Vec::new(),
        selected_radical_ability: None,
        radical_picker_open: false,
        radical_picker_cursor: 0,
        skill_menu_open: false,
        skill_menu_cursor: 0,
        projectiles: Vec::new(),
        arcing_projectiles: Vec::new(),
        god_mode: false,
        audio_events: Vec::new(),
        companion_kind: companion,
        player_equip_effects: {
            let mut effects = Vec::new();
            if let Some(w) = player.weapon {
                effects.push(w.effect);
            }
            if let Some(a) = player.armor {
                effects.push(a.effect);
            }
            if let Some(c) = player.charm {
                effects.push(c.effect);
            }
            effects
        },
        attacks_on_player_this_round: 0,
        arena_event_timer: 3,
        pending_event: None,
        event_message: None,
        event_message_timer: 0,
    }
}

/// Scatter some obstacles and terrain based on floor depth.
fn generate_arena(floor: i32, size: usize, biome: ArenaBiome) -> TacticalArena {
    let mut arena = TacticalArena::new(size, size, biome);

    let obstacle_count = 3 + (floor / 3).min(6) as usize;
    let seed = floor as u64;
    for i in 0..obstacle_count {
        let hash = seed
            .wrapping_mul(2654435761)
            .wrapping_add(i as u64)
            .wrapping_mul(2246822519);
        let x = ((hash >> 16) % size as u64) as i32;
        let y = (1 + (hash >> 8) % (size as u64 - 3)) as i32;
        if y <= 0 || y >= (size as i32 - 1) {
            continue;
        }
        let mid = size as i32 / 2;
        if y == (size as i32 - 2) && (x - mid).abs() <= 1 {
            continue;
        }
        arena.set_tile(x, y, BattleTile::CoverBarrier);
    }

    if floor >= 3 {
        let terrain_count = (floor / 5).min(4) as usize
            + match biome {
                ArenaBiome::StationInterior => 0,
                _ => 2,
            };
        for i in 0..terrain_count {
            let hash = seed
                .wrapping_mul(1103515245)
                .wrapping_add((i + obstacle_count) as u64)
                .wrapping_mul(12345);
            let x = ((hash >> 16) % size as u64) as i32;
            let y = (1 + (hash >> 8) % (size as u64 - 3)) as i32;
            if arena.tile(x, y) != Some(BattleTile::MetalFloor) {
                continue;
            }
            let tile = match biome {
                ArenaBiome::StationInterior => match hash % 5 {
                    0 => BattleTile::WiringPanel,
                    1 => BattleTile::CoolantPool,
                    2 => BattleTile::DamagedPlating,
                    3 => BattleTile::EnergyNode,
                    _ => BattleTile::OilSlick,
                },
                ArenaBiome::DerelictShip => match hash % 6 {
                    0 => BattleTile::OilSlick,
                    1 => BattleTile::DamagedPlating,
                    2 => BattleTile::PowerDrain,
                    3 => BattleTile::GravityTrap,
                    4 => BattleTile::Debris,
                    _ => BattleTile::ElectrifiedWire,
                },
                ArenaBiome::AlienRuins => match hash % 6 {
                    0 => BattleTile::HoloTrap,
                    1 => BattleTile::OilSlick,
                    2 => BattleTile::VentSteam,
                    3 => BattleTile::EnergyNode,
                    4 => BattleTile::ChargingPad,
                    _ => BattleTile::FrozenCoolant,
                },
                ArenaBiome::IrradiatedZone => match hash % 5 {
                    0 => BattleTile::BlastMark,
                    1 => BattleTile::PlasmaPool,
                    2 => BattleTile::DamagedPlating,
                    3 => BattleTile::PowerDrain,
                    _ => BattleTile::ElectrifiedWire,
                },
                ArenaBiome::Hydroponics => match hash % 4 {
                    0 => BattleTile::WiringPanel,
                    1 => BattleTile::CoolantPool,
                    2 => BattleTile::PipeTangle,
                    _ => BattleTile::ElectrifiedWire,
                },
                ArenaBiome::CryoBay => match hash % 4 {
                    0 => BattleTile::FrozenCoolant,
                    1 => BattleTile::CryoZone,
                    2 => BattleTile::CoolantPool,
                    _ => BattleTile::DamagedPlating,
                },
                ArenaBiome::ReactorRoom => match hash % 4 {
                    0 => BattleTile::PlasmaPool,
                    1 => BattleTile::BlastMark,
                    2 => BattleTile::Debris,
                    _ => BattleTile::DamagedPlating,
                },
            };
            arena.set_tile(x, y, tile);
        }
    }

    if floor >= 2 {
        let boulder_count = 2 + ((floor / 3) as usize).min(3);
        let boulder_seed = seed.wrapping_mul(7919).wrapping_add(42);
        for i in 0..boulder_count {
            let hash = boulder_seed
                .wrapping_mul(48271)
                .wrapping_add(i as u64)
                .wrapping_mul(16807);
            let x = ((hash >> 16) % size as u64) as i32;
            let y = (2 + (hash >> 8) % (size as u64 - 4)) as i32;
            let mid = size as i32 / 2;
            if y >= (size as i32 - 2) && (x - mid).abs() <= 1 {
                continue;
            }
            if arena.tile(x, y) != Some(BattleTile::MetalFloor) {
                continue;
            }
            arena.set_tile(x, y, BattleTile::CargoCrate);
        }
    }

    // Interactive terrain: explosive barrels, crumbling floors, trap tiles
    if floor >= 3 {
        let interactive_seed = seed.wrapping_mul(9901).wrapping_add(77);
        // Explosive barrels: 2-3 per arena
        let barrel_count = 2 + ((interactive_seed >> 4) % 2) as usize;
        for i in 0..barrel_count {
            let hash = interactive_seed
                .wrapping_mul(6469)
                .wrapping_add(i as u64)
                .wrapping_mul(22123);
            let x = ((hash >> 16) % size as u64) as i32;
            let y = (2 + (hash >> 8) % (size as u64 - 4)) as i32;
            let mid = size as i32 / 2;
            if y >= (size as i32 - 2) && (x - mid).abs() <= 1 {
                continue;
            }
            if arena.tile(x, y) != Some(BattleTile::MetalFloor) {
                continue;
            }
            arena.set_tile(x, y, BattleTile::FuelCanister);
        }

        // Crumbling floors: 1-2 per arena
        let crumble_count = 1 + ((interactive_seed >> 12) % 2) as usize;
        for i in 0..crumble_count {
            let hash = interactive_seed
                .wrapping_mul(8461)
                .wrapping_add((i + barrel_count) as u64)
                .wrapping_mul(30011);
            let x = ((hash >> 16) % size as u64) as i32;
            let y = (1 + (hash >> 8) % (size as u64 - 3)) as i32;
            if arena.tile(x, y) != Some(BattleTile::MetalFloor) {
                continue;
            }
            arena.set_tile(x, y, BattleTile::WeakenedPlating);
        }

        // Trap tiles: 1-3 per arena
        let trap_count = 1 + ((interactive_seed >> 20) % 3) as usize;
        for i in 0..trap_count {
            let hash = interactive_seed
                .wrapping_mul(11003)
                .wrapping_add((i + barrel_count + crumble_count) as u64)
                .wrapping_mul(40037);
            let x = ((hash >> 16) % size as u64) as i32;
            let y = (1 + (hash >> 8) % (size as u64 - 3)) as i32;
            let mid = size as i32 / 2;
            if y >= (size as i32 - 2) && (x - mid).abs() <= 1 {
                continue;
            }
            if arena.tile(x, y) != Some(BattleTile::MetalFloor) {
                continue;
            }
            arena.set_tile(x, y, BattleTile::MineTile);
        }
    }

    // Oil slick clusters for chain ignition potential (floor >= 3)
    if floor >= 3 {
        let oil_seed = seed.wrapping_mul(13397).wrapping_add(123);
        let oil_count = 2 + (floor as usize / 3).min(4);
        let mut placed_oil = 0;
        for attempt in 0..(oil_count * 5) {
            let hash = oil_seed
                .wrapping_mul(7307)
                .wrapping_add(attempt as u64)
                .wrapping_mul(25171);
            let ox = ((hash >> 16) % size as u64) as i32;
            let oy = (1 + (hash >> 8) % (size as u64 - 3)) as i32;
            if arena.tile(ox, oy) == Some(BattleTile::MetalFloor) {
                arena.set_tile(ox, oy, BattleTile::OilSlick);
                placed_oil += 1;
                // Place adjacent oil for cluster effect
                let deltas: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                let adj_hash = hash.wrapping_add(700);
                let adj_dir = (adj_hash % 4) as usize;
                let (adx, ady) = deltas[adj_dir];
                if arena.tile(ox + adx, oy + ady) == Some(BattleTile::MetalFloor) {
                    arena.set_tile(ox + adx, oy + ady, BattleTile::OilSlick);
                }
                if placed_oil >= oil_count {
                    break;
                }
            }
        }
    }

    // Extra fuel canisters near crates for chain explosion potential
    if floor >= 4 {
        let fuel_adj_seed = seed.wrapping_mul(14879).wrapping_add(200);
        for y in 0..size as i32 {
            for x in 0..size as i32 {
                if arena.tile(x, y) == Some(BattleTile::CargoCrate) {
                    let try_dirs: [(i32, i32); 4] = [(2, 0), (-2, 0), (0, 2), (0, -2)];
                    let dir_hash = fuel_adj_seed
                        .wrapping_mul(x as u64 + 1)
                        .wrapping_add(y as u64);
                    let dir_idx = (dir_hash % 4) as usize;
                    let (dx, dy) = try_dirs[dir_idx];
                    let fx = x + dx;
                    let fy = y + dy;
                    if arena.tile(fx, fy) == Some(BattleTile::MetalFloor) {
                        arena.set_tile(fx, fy, BattleTile::FuelCanister);
                    }
                }
            }
        }
    }

    if floor >= 4 {
        let flow_seed = seed.wrapping_mul(6271).wrapping_add(99);
        let flow_hash = flow_seed.wrapping_mul(31337);
        let should_place = flow_hash % 3 != 0;
        if should_place {
            let dir = flow_hash % 4;
            let (flow_tile, is_horizontal) = match dir {
                0 => (BattleTile::ConveyorN, false),
                1 => (BattleTile::ConveyorS, false),
                2 => (BattleTile::ConveyorE, true),
                _ => (BattleTile::ConveyorW, true),
            };
            let channel_len = 3 + (floor / 6).min(3) as i32;
            if is_horizontal {
                let y = (2 + (flow_hash >> 8) % (size as u64 - 4)) as i32;
                let start_x = ((flow_hash >> 16) % (size as u64 / 2)) as i32;
                for dx in 0..channel_len {
                    let x = start_x + dx;
                    if x >= size as i32 {
                        break;
                    }
                    if arena.tile(x, y) == Some(BattleTile::MetalFloor) {
                        arena.set_tile(x, y, flow_tile);
                    }
                }
            } else {
                let x = (2 + (flow_hash >> 8) % (size as u64 - 4)) as i32;
                let start_y = (1 + (flow_hash >> 16) % (size as u64 / 2)) as i32;
                for dy in 0..channel_len {
                    let y = start_y + dy;
                    if y >= size as i32 - 1 {
                        break;
                    }
                    if arena.tile(x, y) == Some(BattleTile::MetalFloor) {
                        arena.set_tile(x, y, flow_tile);
                    }
                }
            }
        }
    }

    // High Ground: scaled to arena size (hill positions)
    {
        let hg_seed = seed.wrapping_mul(12347).wrapping_add(55);
        let hg_count = 1 + (size / 5);
        for i in 0..hg_count {
            let hash = hg_seed
                .wrapping_mul(7727)
                .wrapping_add(i as u64)
                .wrapping_mul(19991);
            let x = ((hash >> 16) % size as u64) as i32;
            let y = (2 + (hash >> 8) % (size as u64 - 4)) as i32;
            let mid = size as i32 / 2;
            if y >= (size as i32 - 2) && (x - mid).abs() <= 1 {
                continue;
            }
            if arena.tile(x, y) != Some(BattleTile::MetalFloor) {
                continue;
            }
            arena.set_tile(x, y, BattleTile::ElevatedPlatform);
        }
    }

    // Gravity Wells: 1-2 per arena in AlienRuins/ReactorRoom biomes, floor >= 5
    if floor >= 5 && matches!(biome, ArenaBiome::AlienRuins | ArenaBiome::ReactorRoom) {
        let gw_seed = seed.wrapping_mul(17389).wrapping_add(77);
        let gw_count = 1 + ((gw_seed >> 4) % 2) as usize;
        for i in 0..gw_count {
            let hash = gw_seed
                .wrapping_mul(8831)
                .wrapping_add(i as u64)
                .wrapping_mul(21013);
            let x = ((hash >> 16) % size as u64) as i32;
            let y = (2 + (hash >> 8) % (size as u64 - 4)) as i32;
            let mid = size as i32 / 2;
            if y >= (size as i32 - 2) && (x - mid).abs() <= 1 {
                continue;
            }
            if arena.tile(x, y) != Some(BattleTile::MetalFloor) {
                continue;
            }
            arena.set_tile(x, y, BattleTile::GravityWell);
        }
    }

    // Steam Vents: 2-3 per arena in Hydroponics/CryoBay biomes, floor >= 4
    if floor >= 4 && matches!(biome, ArenaBiome::Hydroponics | ArenaBiome::CryoBay) {
        let sv_seed = seed.wrapping_mul(19403).wrapping_add(99);
        let sv_count = 2 + ((sv_seed >> 4) % 2) as usize;
        for i in 0..sv_count {
            let hash = sv_seed
                .wrapping_mul(9173)
                .wrapping_add(i as u64)
                .wrapping_mul(23017);
            let x = ((hash >> 16) % size as u64) as i32;
            let y = (2 + (hash >> 8) % (size as u64 - 4)) as i32;
            let mid = size as i32 / 2;
            if y >= (size as i32 - 2) && (x - mid).abs() <= 1 {
                continue;
            }
            if arena.tile(x, y) != Some(BattleTile::MetalFloor) {
                continue;
            }
            arena.set_tile(x, y, BattleTile::SteamVentInactive);
        }
    }

    // Energy Vents: ~35% chance, 2-3 per arena, floor >= 3
    // Appear in StationInterior, IrradiatedZone, ReactorRoom, DerelictShip biomes
    if floor >= 3 {
        let ev_seed = seed.wrapping_mul(21773).wrapping_add(137);
        let ev_chance = ev_seed % 100;
        let biome_ok = matches!(
            biome,
            ArenaBiome::StationInterior
                | ArenaBiome::IrradiatedZone
                | ArenaBiome::ReactorRoom
                | ArenaBiome::DerelictShip
        );
        if biome_ok && ev_chance < 35 {
            let ev_count = 2 + ((ev_seed >> 8) % 2) as usize;
            for i in 0..ev_count {
                let hash = ev_seed
                    .wrapping_mul(10607)
                    .wrapping_add(i as u64)
                    .wrapping_mul(27449);
                let x = ((hash >> 16) % size as u64) as i32;
                let y = (2 + (hash >> 8) % (size as u64 - 4)) as i32;
                let mid = size as i32 / 2;
                if y >= (size as i32 - 2) && (x - mid).abs() <= 1 {
                    continue;
                }
                if arena.tile(x, y) != Some(BattleTile::MetalFloor) {
                    continue;
                }
                // Stagger initial timers so vents don't all fire at once
                let initial_timer = 1 + (hash % 3) as u8;
                arena.set_energy_vent(x, y, initial_timer);
            }
        }
    }

    // Extra cover barriers near hazards for destructible cover dynamics
    if floor >= 3 {
        let barrier_seed = seed.wrapping_mul(16411).wrapping_add(300);
        let mut placed_barriers = 0;
        let max_extra_barriers = 2 + size / 4;
        'outer: for y in 1..size as i32 - 1 {
            for x in 0..size as i32 {
                if placed_barriers >= max_extra_barriers {
                    break 'outer;
                }
                let is_hazard = matches!(
                    arena.tile(x, y),
                    Some(BattleTile::FuelCanister)
                        | Some(BattleTile::OilSlick)
                        | Some(BattleTile::CargoCrate)
                );
                if !is_hazard {
                    continue;
                }
                let deltas: [(i32, i32); 4] = [(1, 1), (-1, 1), (1, -1), (-1, -1)];
                let hash = barrier_seed
                    .wrapping_mul(x as u64 + 1)
                    .wrapping_add(y as u64)
                    .wrapping_mul(31991);
                let dir_idx = (hash % 4) as usize;
                let (dx, dy) = deltas[dir_idx];
                let bx = x + dx;
                let by = y + dy;
                let mid = size as i32 / 2;
                if by >= (size as i32 - 2) && (bx - mid).abs() <= 1 {
                    continue;
                }
                if by <= 0 || by >= size as i32 - 1 {
                    continue;
                }
                if arena.tile(bx, by) == Some(BattleTile::MetalFloor) {
                    arena.set_tile(bx, by, BattleTile::CoverBarrier);
                    placed_barriers += 1;
                }
            }
        }
    }

    arena
}

pub fn enemies_from_sentence(sentence: &SentenceEntry, floor: i32) -> Vec<Enemy> {
    let chars: Vec<char> = sentence.hanzi.chars().collect();
    let syllables = crate::vocab::pinyin_syllables(sentence.pinyin);

    chars
        .into_iter()
        .enumerate()
        .map(|(i, ch)| {
            let hanzi: &'static str = Box::leak(ch.to_string().into_boxed_str());
            let pinyin: &'static str = if i < syllables.len() {
                Box::leak(syllables[i].to_string().into_boxed_str())
            } else {
                Box::leak(ch.to_string().into_boxed_str())
            };
            let hp = 1 + floor / 3;
            let damage = 1 + floor / 4;
            Enemy {
                x: i as i32,
                y: 0,
                hanzi,
                pinyin,
                meaning: sentence.meaning,
                hp,
                max_hp: hp,
                damage,
                alert: true,
                is_boss: false,
                is_elite: false,
                gold_value: 2 + floor,
                stunned: false,
                statuses: Vec::new(),
                boss_kind: None,
                phase_triggered: false,
                summon_cooldown: 0,
                resisted_spell: None,
                elite_chain: 0,
                components: Vec::new(),
                ai: AiBehavior::Chase,
                radical_armor: 0,
                radical_dodge: false,
                radical_multiply: false,
            }
        })
        .collect()
}

/// Sync battle results back to the player and enemies.
/// Returns indices of enemies that were killed during this battle.
pub fn exit_combat(
    battle: &TacticalBattle,
    player: &mut Player,
    enemies: &mut [Enemy],
) -> Vec<usize> {
    if let Some(player_unit) = battle.units.first() {
        player.hp = player_unit.hp.max(0);
    }

    let mut killed = Vec::new();

    // With per-char splitting, multiple BattleUnits may share the same enemy index.
    // Sum remaining HP across all sub-units for each overworld enemy.
    let mut hp_sums: std::collections::HashMap<usize, i32> = std::collections::HashMap::new();
    for unit in &battle.units {
        if let UnitKind::Enemy(ei) = unit.kind {
            if ei < enemies.len() {
                *hp_sums.entry(ei).or_insert(0) += unit.hp.max(0);
            }
        }
    }

    for (&ei, &total_hp) in &hp_sums {
        let was_alive = enemies[ei].hp > 0;
        enemies[ei].hp = total_hp;
        if was_alive && total_hp <= 0 {
            killed.push(ei);
        }
    }

    killed
}

fn compute_deployment_tiles(arena: &TacticalArena, units: &[BattleUnit]) -> Vec<(i32, i32)> {
    let h = arena.height as i32;
    let mut tiles = Vec::new();
    for y in (h - 3)..h {
        for x in 0..arena.width as i32 {
            if arena.tile(x, y).map(|t| t.is_walkable()).unwrap_or(false)
                && !units.iter().skip(1).any(|u| u.x == x && u.y == y)
            {
                tiles.push((x, y));
            }
        }
    }
    tiles
}

fn derive_wuxing_element(enemy: &Enemy) -> Option<WuxingElement> {
    for comp in &enemy.components {
        if let Some(elem) = WuxingElement::from_radical(comp) {
            return Some(elem);
        }
    }
    None
}

fn apply_mastery_debuffs(unit: &mut BattleUnit) {
    match unit.mastery_tier {
        3 => {
            unit.hp = ((unit.hp as f64) * 0.7).ceil() as i32;
            unit.max_hp = ((unit.max_hp as f64) * 0.7).ceil() as i32;
            unit.damage = ((unit.damage as f64) * 0.7).ceil() as i32;
        }
        2 => {
            unit.hp = ((unit.hp as f64) * 0.85).ceil() as i32;
            unit.max_hp = ((unit.max_hp as f64) * 0.85).ceil() as i32;
            unit.damage = ((unit.damage as f64) * 0.85).ceil() as i32;
        }
        _ => {}
    }
    unit.hp = unit.hp.max(1);
    unit.max_hp = unit.max_hp.max(1);
    unit.damage = unit.damage.max(1);
}

fn pick_weather(floor: i32, biome: ArenaBiome) -> Weather {
    if floor < 5 {
        return Weather::Normal;
    }
    let hash = (floor as u64)
        .wrapping_mul(2654435761)
        .wrapping_add(biome as u64);
    match hash % 10 {
        0..=4 => Weather::Normal,
        5 => Weather::CoolantLeak,
        6 => Weather::SmokeScreen,
        7 => Weather::DebrisStorm,
        8..=9 => match biome {
            ArenaBiome::AlienRuins => Weather::EnergyFlux,
            ArenaBiome::DerelictShip => Weather::SmokeScreen,
            ArenaBiome::IrradiatedZone => Weather::DebrisStorm,
            ArenaBiome::StationInterior => Weather::CoolantLeak,
            ArenaBiome::Hydroponics => Weather::CoolantLeak,
            ArenaBiome::CryoBay => Weather::SmokeScreen,
            ArenaBiome::ReactorRoom => Weather::DebrisStorm,
        },
        _ => Weather::Normal,
    }
}


