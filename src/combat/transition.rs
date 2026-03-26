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
use crate::player::{active_set_bonuses, Player, SetBonus};
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

    // Equipment set bonuses
    let set_bonuses = active_set_bonuses(player);
    let set_flat: i32 = set_bonuses.iter().map(|s| match s.bonus { SetBonus::BonusDamage(d) => d, _ => 0 }).sum();
    let set_first_strike: i32 = set_bonuses.iter().map(|s| match s.bonus { SetBonus::FirstStrikeDamage(d) => d, _ => 0 }).sum();

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
        max_hp: player.effective_max_hp(),
        damage: p_damage + set_flat,
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
        first_strike_bonus: set_first_strike,
    });    // Companion unit adjacent to player (if any).
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
            first_strike_bonus: 0,
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
                first_strike_bonus: 0,
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
        pending_impacts: Vec::new(),
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
        phase_walk_available: !player.phase_walk_used
            && player.has_set_bonus(|b| matches!(b, SetBonus::PhaseWalk)),
        riposte_charges: player.riposte_charges,
        overcharge_active: player.overcharge_active,
        hubris_mode: player.hubris_mode,
        hard_answer_armor_bonus: player.hard_answer_armor_bonus,
        has_polyglot: player.skill_tree.has_polyglot(),
        has_linguists_fury: player.skill_tree.has_linguists_fury(),
        pending_skill_xp: 0,
        pending_weapon_crucible_xp: 0,
        pending_armor_crucible_xp: 0,
        pending_charm_crucible_xp: 0,
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
        let should_place = flow_hash % 3 != 0 ;
        if should_place {
            let dir = flow_hash % 4;
            let (flow_tile, is_horizontal) = match dir {
                0 => (BattleTile::ConveyorN, false),
                1 => (BattleTile::ConveyorS, false),
                2 => (BattleTile::ConveyorE, true),
                _ => (BattleTile::ConveyorW, true),
            };
            let channel_len = 3 + (floor / 6).min(3);
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

    // Sync risk/reward state back to player.
    player.riposte_charges = battle.riposte_charges;
    player.overcharge_active = battle.overcharge_active;
    player.hard_answer_armor_bonus = battle.hard_answer_armor_bonus;

    // Apply accumulated XP.
    player.skill_tree.gain_xp(battle.pending_skill_xp as u32);
    player.weapon_crucible.gain_xp(battle.pending_weapon_crucible_xp as u32);
    player.armor_crucible.gain_xp(battle.pending_armor_crucible_xp as u32);
    player.charm_crucible.gain_xp(battle.pending_charm_crucible_xp as u32);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::test_helpers::*;
    use crate::player::PlayerClass;
    use crate::vocab::SentenceEntry;

    fn make_basic_player() -> Player {
        Player::new(0, 0, PlayerClass::Soldier)
    }

    fn make_basic_enemy() -> Enemy {
        Enemy {
            x: 3,
            y: 1,
            hanzi: "火",
            pinyin: "huo3",
            meaning: "fire",
            hp: 10,
            max_hp: 10,
            damage: 3,
            alert: true,
            is_boss: false,
            is_elite: false,
            gold_value: 5,
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
    }

    fn make_boss_enemy() -> Enemy {
        let mut e = make_basic_enemy();
        e.is_boss = true;
        e.hp = 30;
        e.max_hp = 30;
        e.damage = 8;
        e
    }

    fn make_elite_enemy() -> Enemy {
        let mut e = make_basic_enemy();
        e.is_elite = true;
        e.hp = 15;
        e.max_hp = 15;
        e.damage = 5;
        e
    }

    fn make_multi_char_enemy() -> Enemy {
        Enemy {
            x: 3,
            y: 1,
            hanzi: "朋友",
            pinyin: "peng2you3",
            meaning: "friend",
            hp: 12,
            max_hp: 12,
            damage: 4,
            alert: true,
            is_boss: false,
            is_elite: false,
            gold_value: 8,
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
    }

    // ── enter_combat tests ──────────────────────────────────────

    #[test]
    fn enter_combat_returns_battle_with_player_unit() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(&player, &enemies, &[0], 1, None, &srs, None);

        assert!(battle.units.len() >= 2, "should have player + at least 1 enemy");
        assert_eq!(battle.units[0].kind, UnitKind::Player);
        assert!(battle.units[0].alive);
        assert_eq!(battle.units[0].hp, player.hp);
    }

    #[test]
    fn enter_combat_player_positioned_at_bottom_center() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(&player, &enemies, &[0], 1, None, &srs, None);
        let arena_size = battle.arena.width;

        let expected_x = (arena_size / 2) as i32;
        let expected_y = (arena_size - 2) as i32;
        assert_eq!(battle.units[0].x, expected_x);
        assert_eq!(battle.units[0].y, expected_y);
    }

    #[test]
    fn enter_combat_places_enemy_units() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(&player, &enemies, &[0], 1, None, &srs, None);

        let enemy_units: Vec<_> = battle
            .units
            .iter()
            .filter(|u| matches!(u.kind, UnitKind::Enemy(_)))
            .collect();
        assert_eq!(enemy_units.len(), 1);
        assert!(enemy_units[0].alive);
    }

    #[test]
    fn enter_combat_turn_queue_contains_all_units() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(&player, &enemies, &[0], 1, None, &srs, None);

        assert_eq!(battle.turn_queue.len(), battle.units.len());
    }

    #[test]
    fn enter_combat_boss_sets_is_boss_battle() {
        let player = make_basic_player();
        let enemies = vec![make_boss_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(&player, &enemies, &[0], 1, None, &srs, None);

        assert!(battle.is_boss_battle);
    }

    #[test]
    fn enter_combat_non_boss_clears_is_boss_battle() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(&player, &enemies, &[0], 1, None, &srs, None);

        assert!(!battle.is_boss_battle);
    }

    #[test]
    fn enter_combat_boss_creates_larger_arena() {
        let player = make_basic_player();
        let normal_enemy = make_basic_enemy();
        let boss_enemy = make_boss_enemy();
        let srs = SrsTracker::new();

        let normal_battle = enter_combat(&player, &[normal_enemy], &[0], 1, None, &srs, None);
        let boss_battle = enter_combat(&player, &[boss_enemy], &[0], 1, None, &srs, None);

        assert!(boss_battle.arena.width > normal_battle.arena.width);
    }

    #[test]
    fn enter_combat_with_companion_adds_companion_unit() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(
            &player,
            &enemies,
            &[0],
            1,
            None,
            &srs,
            Some(Companion::SecurityChief),
        );

        let companion_units: Vec<_> = battle
            .units
            .iter()
            .filter(|u| u.kind == UnitKind::Companion)
            .collect();
        assert_eq!(companion_units.len(), 1);
        assert_eq!(companion_units[0].hp, 8);
        assert_eq!(companion_units[0].damage, 2);
    }

    #[test]
    fn enter_combat_medic_companion_has_correct_stats() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(
            &player,
            &enemies,
            &[0],
            1,
            None,
            &srs,
            Some(Companion::Medic),
        );

        let comp = battle
            .units
            .iter()
            .find(|u| u.kind == UnitKind::Companion)
            .unwrap();
        assert_eq!(comp.hp, 6);
        assert_eq!(comp.damage, 1);
    }

    #[test]
    fn enter_combat_companion_placed_left_of_player() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(
            &player,
            &enemies,
            &[0],
            1,
            None,
            &srs,
            Some(Companion::SecurityChief),
        );

        let player_unit = &battle.units[0];
        let comp_unit = battle
            .units
            .iter()
            .find(|u| u.kind == UnitKind::Companion)
            .unwrap();
        assert_eq!(comp_unit.x, player_unit.x - 1);
        assert_eq!(comp_unit.y, player_unit.y);
    }

    #[test]
    fn enter_combat_multi_char_enemy_splits_into_units() {
        let player = make_basic_player();
        let enemies = vec![make_multi_char_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(&player, &enemies, &[0], 1, None, &srs, None);

        let enemy_units: Vec<_> = battle
            .units
            .iter()
            .filter(|u| matches!(u.kind, UnitKind::Enemy(0)))
            .collect();
        assert_eq!(enemy_units.len(), 2, "朋友 should split into 2 units");
        assert!(enemy_units.iter().all(|u| u.word_group == Some(0)));
        assert_eq!(enemy_units[0].word_group_order, 0);
        assert_eq!(enemy_units[1].word_group_order, 1);
    }

    #[test]
    fn enter_combat_multi_char_distributes_hp() {
        let player = make_basic_player();
        let enemies = vec![make_multi_char_enemy()]; // hp=12, 2 chars
        let srs = SrsTracker::new();

        let battle = enter_combat(&player, &enemies, &[0], 1, None, &srs, None);

        let enemy_units: Vec<_> = battle
            .units
            .iter()
            .filter(|u| matches!(u.kind, UnitKind::Enemy(_)))
            .collect();
        let total_hp: i32 = enemy_units.iter().map(|u| u.hp).sum();
        assert_eq!(total_hp, 12, "total HP should equal original enemy HP");
    }

    #[test]
    fn enter_combat_room_modifier_changes_biome() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(
            &player,
            &enemies,
            &[0],
            1,
            Some(RoomModifier::Hydroponics),
            &srs,
            None,
        );

        assert_eq!(battle.arena.biome, ArenaBiome::Hydroponics);
    }

    #[test]
    fn enter_combat_multiple_enemies() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy(), make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(&player, &enemies, &[0, 1], 1, None, &srs, None);

        let enemy_count = battle
            .units
            .iter()
            .filter(|u| matches!(u.kind, UnitKind::Enemy(_)))
            .count();
        assert_eq!(enemy_count, 2);
    }

    #[test]
    fn enter_combat_weather_normal_on_low_floor() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(&player, &enemies, &[0], 1, None, &srs, None);

        assert_eq!(battle.weather, Weather::Normal);
    }

    #[test]
    fn enter_combat_arena_has_cover_barriers() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(&player, &enemies, &[0], 5, None, &srs, None);
        let arena = &battle.arena;

        let barrier_count = (0..arena.width)
            .flat_map(|x| (0..arena.height).map(move |y| (x as i32, y as i32)))
            .filter(|&(x, y)| arena.tile(x, y) == Some(BattleTile::CoverBarrier))
            .count();
        assert!(barrier_count > 0, "arena should have cover barriers");
    }

    #[test]
    fn enter_combat_preserves_player_hp() {
        let mut player = make_basic_player();
        player.hp = 7;
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(&player, &enemies, &[0], 1, None, &srs, None);

        assert_eq!(battle.units[0].hp, 7);
    }

    #[test]
    fn enter_combat_log_starts_with_battle_begins() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(&player, &enemies, &[0], 1, None, &srs, None);

        assert_eq!(battle.log, vec!["Battle begins!"]);
    }

    // ── exit_combat tests ──────────────────────────────────────

    #[test]
    fn exit_combat_syncs_player_hp() {
        let mut player = make_basic_player();
        player.hp = 10;
        let mut enemies = vec![make_basic_enemy()];
        let mut battle = make_test_battle(vec![
            make_test_unit(UnitKind::Player, 4, 5),
            make_test_unit(UnitKind::Enemy(0), 4, 1),
        ]);
        battle.units[0].hp = 5;

        exit_combat(&battle, &mut player, &mut enemies);

        assert_eq!(player.hp, 5);
    }

    #[test]
    fn exit_combat_returns_killed_enemy_indices() {
        let mut player = make_basic_player();
        let mut enemies = vec![make_basic_enemy(), make_basic_enemy()];
        enemies[0].hp = 5;
        enemies[1].hp = 5;
        let mut battle = make_test_battle(vec![
            make_test_unit(UnitKind::Player, 4, 5),
            make_test_unit(UnitKind::Enemy(0), 3, 1),
            make_test_unit(UnitKind::Enemy(1), 5, 1),
        ]);
        battle.units[1].hp = 0; // enemy 0 killed
        battle.units[2].hp = 3; // enemy 1 alive

        let killed = exit_combat(&battle, &mut player, &mut enemies);

        assert!(killed.contains(&0));
        assert!(!killed.contains(&1));
    }

    #[test]
    fn exit_combat_preserves_alive_enemy_hp() {
        let mut player = make_basic_player();
        let mut enemies = vec![make_basic_enemy()];
        enemies[0].hp = 10;
        let mut battle = make_test_battle(vec![
            make_test_unit(UnitKind::Player, 4, 5),
            make_test_unit(UnitKind::Enemy(0), 4, 1),
        ]);
        battle.units[1].hp = 7;

        exit_combat(&battle, &mut player, &mut enemies);

        assert_eq!(enemies[0].hp, 7);
    }

    #[test]
    fn exit_combat_dead_enemy_has_zero_hp() {
        let mut player = make_basic_player();
        let mut enemies = vec![make_basic_enemy()];
        enemies[0].hp = 10;
        let mut battle = make_test_battle(vec![
            make_test_unit(UnitKind::Player, 4, 5),
            make_test_unit(UnitKind::Enemy(0), 4, 1),
        ]);
        battle.units[1].hp = -2;

        let killed = exit_combat(&battle, &mut player, &mut enemies);

        assert_eq!(enemies[0].hp, 0);
        assert!(killed.contains(&0));
    }

    #[test]
    fn exit_combat_aggregates_split_unit_hp() {
        let mut player = make_basic_player();
        let mut enemies = vec![make_multi_char_enemy()];
        enemies[0].hp = 12;
        let mut battle = make_test_battle(vec![
            make_test_unit(UnitKind::Player, 4, 5),
            make_test_unit(UnitKind::Enemy(0), 3, 1),
            make_test_unit(UnitKind::Enemy(0), 5, 1),
        ]);
        battle.units[1].hp = 3;
        battle.units[2].hp = 4;

        exit_combat(&battle, &mut player, &mut enemies);

        assert_eq!(enemies[0].hp, 7);
    }

    #[test]
    fn exit_combat_split_units_all_dead_kills_enemy() {
        let mut player = make_basic_player();
        let mut enemies = vec![make_multi_char_enemy()];
        enemies[0].hp = 12;
        let mut battle = make_test_battle(vec![
            make_test_unit(UnitKind::Player, 4, 5),
            make_test_unit(UnitKind::Enemy(0), 3, 1),
            make_test_unit(UnitKind::Enemy(0), 5, 1),
        ]);
        battle.units[1].hp = 0;
        battle.units[2].hp = -1;

        let killed = exit_combat(&battle, &mut player, &mut enemies);

        assert!(killed.contains(&0));
        assert_eq!(enemies[0].hp, 0);
    }

    #[test]
    fn exit_combat_syncs_riposte_charges() {
        let mut player = make_basic_player();
        player.riposte_charges = 0;
        let mut enemies = vec![make_basic_enemy()];
        let mut battle = make_test_battle(vec![
            make_test_unit(UnitKind::Player, 4, 5),
            make_test_unit(UnitKind::Enemy(0), 4, 1),
        ]);
        battle.riposte_charges = 3;

        exit_combat(&battle, &mut player, &mut enemies);

        assert_eq!(player.riposte_charges, 3);
    }

    #[test]
    fn exit_combat_syncs_overcharge_state() {
        let mut player = make_basic_player();
        player.overcharge_active = false;
        let mut enemies = vec![make_basic_enemy()];
        let mut battle = make_test_battle(vec![
            make_test_unit(UnitKind::Player, 4, 5),
            make_test_unit(UnitKind::Enemy(0), 4, 1),
        ]);
        battle.overcharge_active = true;

        exit_combat(&battle, &mut player, &mut enemies);

        assert!(player.overcharge_active);
    }

    #[test]
    fn exit_combat_clamps_negative_player_hp_to_zero() {
        let mut player = make_basic_player();
        let mut enemies = vec![make_basic_enemy()];
        let mut battle = make_test_battle(vec![
            make_test_unit(UnitKind::Player, 4, 5),
            make_test_unit(UnitKind::Enemy(0), 4, 1),
        ]);
        battle.units[0].hp = -5;

        exit_combat(&battle, &mut player, &mut enemies);

        assert_eq!(player.hp, 0);
    }

    #[test]
    fn exit_combat_no_enemies_returns_empty_killed() {
        let mut player = make_basic_player();
        let mut enemies: Vec<Enemy> = Vec::new();
        let battle = make_test_battle(vec![
            make_test_unit(UnitKind::Player, 4, 5),
        ]);

        let killed = exit_combat(&battle, &mut player, &mut enemies);

        assert!(killed.is_empty());
    }

    // ── enemies_from_sentence tests ─────────────────────────────

    #[test]
    fn enemies_from_sentence_creates_one_per_character() {
        let sentence = SentenceEntry {
            hanzi: "你好",
            pinyin: "ni3 hao3",
            meaning: "hello",
            hsk: 1,
        };

        let enemies = enemies_from_sentence(&sentence, 3);

        assert_eq!(enemies.len(), 2);
    }

    #[test]
    fn enemies_from_sentence_hp_scales_with_floor() {
        let sentence = SentenceEntry {
            hanzi: "大",
            pinyin: "da4",
            meaning: "big",
            hsk: 1,
        };

        let floor_1 = enemies_from_sentence(&sentence, 1);
        let floor_9 = enemies_from_sentence(&sentence, 9);

        assert_eq!(floor_1[0].hp, 1 + 1 / 3); // 1
        assert_eq!(floor_9[0].hp, 1 + 9 / 3); // 4
        assert!(floor_9[0].hp > floor_1[0].hp);
    }

    #[test]
    fn enemies_from_sentence_damage_scales_with_floor() {
        let sentence = SentenceEntry {
            hanzi: "大",
            pinyin: "da4",
            meaning: "big",
            hsk: 1,
        };

        let enemies = enemies_from_sentence(&sentence, 8);

        assert_eq!(enemies[0].damage, 1 + 8 / 4);
    }

    #[test]
    fn enemies_from_sentence_gold_scales_with_floor() {
        let sentence = SentenceEntry {
            hanzi: "大",
            pinyin: "da4",
            meaning: "big",
            hsk: 1,
        };

        let enemies = enemies_from_sentence(&sentence, 5);

        assert_eq!(enemies[0].gold_value, 2 + 5);
    }

    #[test]
    fn enemies_from_sentence_preserves_meaning() {
        let sentence = SentenceEntry {
            hanzi: "你好",
            pinyin: "ni3 hao3",
            meaning: "hello",
            hsk: 1,
        };

        let enemies = enemies_from_sentence(&sentence, 1);

        assert!(enemies.iter().all(|e| e.meaning == "hello"));
    }

    #[test]
    fn enemies_from_sentence_enemies_are_not_bosses_or_elites() {
        let sentence = SentenceEntry {
            hanzi: "你好",
            pinyin: "ni3 hao3",
            meaning: "hello",
            hsk: 1,
        };

        let enemies = enemies_from_sentence(&sentence, 1);

        assert!(enemies.iter().all(|e| !e.is_boss && !e.is_elite));
    }

    #[test]
    fn enemies_from_sentence_positions_sequentially() {
        let sentence = SentenceEntry {
            hanzi: "你好吗",
            pinyin: "ni3 hao3 ma5",
            meaning: "how are you",
            hsk: 1,
        };

        let enemies = enemies_from_sentence(&sentence, 1);

        assert_eq!(enemies[0].x, 0);
        assert_eq!(enemies[1].x, 1);
        assert_eq!(enemies[2].x, 2);
    }

    // ── pick_weather tests ──────────────────────────────────────

    #[test]
    fn pick_weather_returns_normal_on_low_floors() {
        for floor in 0..5 {
            let weather = pick_weather(floor, ArenaBiome::StationInterior);
            assert_eq!(weather, Weather::Normal);
        }
    }

    #[test]
    fn pick_weather_can_return_non_normal_on_high_floors() {
        let mut found_non_normal = false;
        for floor in 5..20 {
            for biome in [
                ArenaBiome::StationInterior,
                ArenaBiome::DerelictShip,
                ArenaBiome::AlienRuins,
                ArenaBiome::CryoBay,
                ArenaBiome::ReactorRoom,
            ] {
                if pick_weather(floor, biome) != Weather::Normal {
                    found_non_normal = true;
                    break;
                }
            }
        }
        assert!(found_non_normal);
    }

    // ── apply_mastery_debuffs tests (tested indirectly) ─────────

    #[test]
    fn mastery_tier_3_reduces_enemy_stats() {
        let mut unit = make_test_unit(UnitKind::Enemy(0), 0, 0);
        unit.hp = 10;
        unit.max_hp = 10;
        unit.damage = 10;
        unit.mastery_tier = 3;

        apply_mastery_debuffs(&mut unit);

        // 10 * 0.7 = 7, ceil = 7
        assert_eq!(unit.hp, 7);
        assert_eq!(unit.max_hp, 7);
        assert_eq!(unit.damage, 7);
    }

    #[test]
    fn mastery_tier_2_reduces_enemy_stats_less() {
        let mut unit = make_test_unit(UnitKind::Enemy(0), 0, 0);
        unit.hp = 10;
        unit.max_hp = 10;
        unit.damage = 10;
        unit.mastery_tier = 2;

        apply_mastery_debuffs(&mut unit);

        // 10 * 0.85 = 8.5, ceil = 9
        assert_eq!(unit.hp, 9);
        assert_eq!(unit.max_hp, 9);
        assert_eq!(unit.damage, 9);
    }

    #[test]
    fn mastery_tier_0_does_not_change_stats() {
        let mut unit = make_test_unit(UnitKind::Enemy(0), 0, 0);
        unit.hp = 10;
        unit.max_hp = 10;
        unit.damage = 5;
        unit.mastery_tier = 0;

        apply_mastery_debuffs(&mut unit);

        assert_eq!(unit.hp, 10);
        assert_eq!(unit.max_hp, 10);
        assert_eq!(unit.damage, 5);
    }

    #[test]
    fn mastery_debuffs_enforce_minimum_one() {
        let mut unit = make_test_unit(UnitKind::Enemy(0), 0, 0);
        unit.hp = 1;
        unit.max_hp = 1;
        unit.damage = 1;
        unit.mastery_tier = 3;

        apply_mastery_debuffs(&mut unit);

        assert_eq!(unit.hp, 1);
        assert_eq!(unit.max_hp, 1);
        assert_eq!(unit.damage, 1);
    }

    // ── derive_wuxing_element tests ─────────────────────────────

    #[test]
    fn derive_wuxing_element_returns_none_for_empty_components() {
        let enemy = make_basic_enemy();

        let element = derive_wuxing_element(&enemy);

        assert!(element.is_none());
    }

    // ── generate_arena tests (via enter_combat) ─────────────────

    #[test]
    fn arena_dimensions_match_encounter_size() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(&player, &enemies, &[0], 1, None, &srs, None);
        let expected_size = arena_size_for_encounter(false, false);

        assert_eq!(battle.arena.width, expected_size);
        assert_eq!(battle.arena.height, expected_size);
    }

    #[test]
    fn arena_for_elite_is_larger_than_normal() {
        let expected_normal = arena_size_for_encounter(false, false);
        let expected_elite = arena_size_for_encounter(true, false);

        assert!(expected_elite > expected_normal);
    }

    #[test]
    fn arena_for_boss_is_largest() {
        let expected_elite = arena_size_for_encounter(true, false);
        let expected_boss = arena_size_for_encounter(false, true);

        assert!(expected_boss > expected_elite);
    }

    // ══════════════════════════════════════════════════════════════════
    // NEW: apply_mastery_debuffs tests
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn apply_mastery_debuffs_tier_3_reduces_stats() {
        let mut unit = make_test_unit(UnitKind::Enemy(0), 3, 3);
        unit.hp = 10;
        unit.max_hp = 10;
        unit.damage = 10;
        unit.mastery_tier = 3;

        apply_mastery_debuffs(&mut unit);

        // 10 * 0.7 = 7.0 → ceil = 7
        assert_eq!(unit.hp, 7);
        assert_eq!(unit.max_hp, 7);
        assert_eq!(unit.damage, 7);
    }

    #[test]
    fn apply_mastery_debuffs_tier_2_reduces_stats() {
        let mut unit = make_test_unit(UnitKind::Enemy(0), 3, 3);
        unit.hp = 10;
        unit.max_hp = 10;
        unit.damage = 10;
        unit.mastery_tier = 2;

        apply_mastery_debuffs(&mut unit);

        // 10 * 0.85 = 8.5 → ceil = 9
        assert_eq!(unit.hp, 9);
        assert_eq!(unit.max_hp, 9);
        assert_eq!(unit.damage, 9);
    }

    #[test]
    fn apply_mastery_debuffs_tier_1_no_change() {
        let mut unit = make_test_unit(UnitKind::Enemy(0), 3, 3);
        unit.hp = 10;
        unit.max_hp = 10;
        unit.damage = 5;
        unit.mastery_tier = 1;

        apply_mastery_debuffs(&mut unit);

        assert_eq!(unit.hp, 10);
        assert_eq!(unit.max_hp, 10);
        assert_eq!(unit.damage, 5);
    }

    #[test]
    fn apply_mastery_debuffs_tier_0_no_change() {
        let mut unit = make_test_unit(UnitKind::Enemy(0), 3, 3);
        unit.hp = 10;
        unit.max_hp = 10;
        unit.damage = 5;
        unit.mastery_tier = 0;

        apply_mastery_debuffs(&mut unit);

        assert_eq!(unit.hp, 10);
        assert_eq!(unit.max_hp, 10);
        assert_eq!(unit.damage, 5);
    }

    #[test]
    fn apply_mastery_debuffs_clamps_to_one() {
        let mut unit = make_test_unit(UnitKind::Enemy(0), 3, 3);
        unit.hp = 1;
        unit.max_hp = 1;
        unit.damage = 1;
        unit.mastery_tier = 3;

        apply_mastery_debuffs(&mut unit);

        // 1 * 0.7 = 0.7 → ceil = 1, then clamped to max(1)
        assert_eq!(unit.hp, 1);
        assert_eq!(unit.max_hp, 1);
        assert_eq!(unit.damage, 1);
    }

    // ══════════════════════════════════════════════════════════════════
    // NEW: pick_weather tests
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn pick_weather_returns_normal_below_floor_5() {
        assert_eq!(pick_weather(1, ArenaBiome::StationInterior), Weather::Normal);
        assert_eq!(pick_weather(4, ArenaBiome::AlienRuins), Weather::Normal);
    }

    #[test]
    fn pick_weather_can_return_non_normal_at_floor_5_plus() {
        // Test that at least one biome/floor combo gives something other than Normal
        let mut found_non_normal = false;
        for floor in 5..=20 {
            for biome in [
                ArenaBiome::StationInterior,
                ArenaBiome::AlienRuins,
                ArenaBiome::DerelictShip,
                ArenaBiome::IrradiatedZone,
                ArenaBiome::Hydroponics,
                ArenaBiome::CryoBay,
                ArenaBiome::ReactorRoom,
            ] {
                if pick_weather(floor, biome) != Weather::Normal {
                    found_non_normal = true;
                    break;
                }
            }
            if found_non_normal {
                break;
            }
        }
        assert!(found_non_normal, "some floor/biome combo should yield non-Normal weather");
    }

    // ══════════════════════════════════════════════════════════════════
    // NEW: compute_deployment_tiles tests
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn compute_deployment_tiles_covers_bottom_rows() {
        let arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
        let units = vec![make_test_unit(UnitKind::Player, 3, 5)];

        let tiles = compute_deployment_tiles(&arena, &units);

        // Should include tiles in the bottom 3 rows (y=4,5,6)
        assert!(!tiles.is_empty());
        for &(_, y) in &tiles {
            assert!(y >= 4);
        }
    }

    #[test]
    fn compute_deployment_tiles_excludes_enemy_positions() {
        let arena = TacticalArena::new(7, 7, ArenaBiome::StationInterior);
        let player = make_test_unit(UnitKind::Player, 3, 5);
        let enemy = make_test_unit(UnitKind::Enemy(0), 2, 5);
        let units = vec![player, enemy];

        let tiles = compute_deployment_tiles(&arena, &units);

        // enemy at (2,5) should be excluded
        assert!(!tiles.contains(&(2, 5)));
    }

    // ══════════════════════════════════════════════════════════════════
    // NEW: derive_wuxing_element tests
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn derive_wuxing_element_no_components_returns_none() {
        let enemy = make_basic_enemy();

        let elem = derive_wuxing_element(&enemy);

        assert!(elem.is_none());
    }

    #[test]
    fn derive_wuxing_element_with_fire_radical() {
        let mut enemy = make_basic_enemy();
        enemy.components = vec!["火"];

        let elem = derive_wuxing_element(&enemy);

        assert_eq!(elem, Some(WuxingElement::Fire));
    }

    #[test]
    fn derive_wuxing_element_with_water_radical() {
        let mut enemy = make_basic_enemy();
        enemy.components = vec!["水"];

        let elem = derive_wuxing_element(&enemy);

        assert_eq!(elem, Some(WuxingElement::Water));
    }

    #[test]
    fn derive_wuxing_element_with_unknown_radical_returns_none() {
        let mut enemy = make_basic_enemy();
        enemy.components = vec!["口"];

        let elem = derive_wuxing_element(&enemy);

        // 口 is not a wuxing element radical
        assert!(elem.is_none());
    }

    // ══════════════════════════════════════════════════════════════════
    // NEW: exit_combat edge cases
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn exit_combat_syncs_hard_answer_armor_bonus() {
        let mut player = make_basic_player();
        player.hard_answer_armor_bonus = 0;
        let mut enemies = vec![make_basic_enemy()];
        let mut battle = make_test_battle(vec![
            make_test_unit(UnitKind::Player, 4, 5),
            make_test_unit(UnitKind::Enemy(0), 4, 1),
        ]);
        battle.hard_answer_armor_bonus = 5;

        exit_combat(&battle, &mut player, &mut enemies);

        assert_eq!(player.hard_answer_armor_bonus, 5);
    }

    #[test]
    fn exit_combat_syncs_skill_xp() {
        let mut player = make_basic_player();
        let initial_xp = player.skill_tree.xp;
        let mut enemies = vec![make_basic_enemy()];
        let mut battle = make_test_battle(vec![
            make_test_unit(UnitKind::Player, 4, 5),
            make_test_unit(UnitKind::Enemy(0), 4, 1),
        ]);
        battle.pending_skill_xp = 50;

        exit_combat(&battle, &mut player, &mut enemies);

        assert!(player.skill_tree.xp >= initial_xp + 50);
    }

    // ══════════════════════════════════════════════════════════════════
    // NEW: enemies_from_sentence edge cases
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn enemies_from_sentence_single_char() {
        let sentence = SentenceEntry {
            hanzi: "人",
            pinyin: "ren2",
            meaning: "person",
            hsk: 1,
        };

        let enemies = enemies_from_sentence(&sentence, 1);

        assert_eq!(enemies.len(), 1);
        assert_eq!(enemies[0].hanzi, "人");
    }

    #[test]
    fn enemies_from_sentence_all_enemies_alive() {
        let sentence = SentenceEntry {
            hanzi: "你好吗",
            pinyin: "ni3 hao3 ma5",
            meaning: "how are you",
            hsk: 1,
        };

        let enemies = enemies_from_sentence(&sentence, 3);

        assert_eq!(enemies.len(), 3);
        for e in &enemies {
            assert!(e.hp > 0);
        }
    }

    // ══════════════════════════════════════════════════════════════════
    // NEW: generate_arena tests (via enter_combat)
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn enter_combat_deep_floor_has_more_obstacles() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle_floor_1 = enter_combat(&player, &enemies, &[0], 1, None, &srs, None);
        let battle_floor_10 = enter_combat(&player, &enemies, &[0], 10, None, &srs, None);

        let count_obstacles = |battle: &TacticalBattle| {
            (0..battle.arena.width)
                .flat_map(|x| (0..battle.arena.height).map(move |y| (x as i32, y as i32)))
                .filter(|&(x, y)| {
                    let tile = battle.arena.tile(x, y);
                    tile != Some(BattleTile::MetalFloor) && tile.is_some()
                })
                .count()
        };

        assert!(count_obstacles(&battle_floor_10) > count_obstacles(&battle_floor_1));
    }

    #[test]
    fn enter_combat_with_elite_uses_larger_arena() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy(), make_elite_enemy()];
        let normal_enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let normal_battle = enter_combat(&player, &normal_enemies, &[0], 1, None, &srs, None);
        let elite_battle = enter_combat(&player, &enemies, &[0, 1], 1, None, &srs, None);

        assert!(elite_battle.arena.width >= normal_battle.arena.width);
    }

    #[test]
    fn enter_combat_science_officer_companion() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(
            &player,
            &enemies,
            &[0],
            1,
            None,
            &srs,
            Some(Companion::ScienceOfficer),
        );

        let comp = battle
            .units
            .iter()
            .find(|u| u.kind == UnitKind::Companion)
            .unwrap();
        assert_eq!(comp.hp, 5);
        assert_eq!(comp.damage, 1);
    }

    #[test]
    fn enter_combat_quartermaster_companion() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let battle = enter_combat(
            &player,
            &enemies,
            &[0],
            1,
            None,
            &srs,
            Some(Companion::Quartermaster),
        );

        let comp = battle
            .units
            .iter()
            .find(|u| u.kind == UnitKind::Companion)
            .unwrap();
        assert_eq!(comp.hp, 5);
        assert_eq!(comp.damage, 1);
    }

    #[test]
    fn enter_combat_various_biomes() {
        let player = make_basic_player();
        let enemies = vec![make_basic_enemy()];
        let srs = SrsTracker::new();

        let modifiers = [
            (Some(RoomModifier::PoweredDown), ArenaBiome::DerelictShip),
            (Some(RoomModifier::HighTech), ArenaBiome::AlienRuins),
            (Some(RoomModifier::Irradiated), ArenaBiome::IrradiatedZone),
            (Some(RoomModifier::Cryogenic), ArenaBiome::CryoBay),
            (Some(RoomModifier::OverheatedReactor), ArenaBiome::ReactorRoom),
        ];
        for (modifier, expected_biome) in modifiers {
            let battle = enter_combat(&player, &enemies, &[0], 1, modifier, &srs, None);
            assert_eq!(battle.arena.biome, expected_biome);
        }
    }
}
