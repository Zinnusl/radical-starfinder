use crate::combat::turn::{
    build_turn_queue, enemy_base_movement, enemy_base_speed, player_movement, player_speed,
};
use crate::combat::{
    BattleTile, BattleUnit, Direction, TacticalArena, TacticalBattle, TacticalPhase, UnitKind,
    ARENA_SIZE,
};
use crate::enemy::{AiBehavior, Enemy};
use crate::player::Player;

pub fn enter_combat(
    player: &Player,
    enemies: &[Enemy],
    enemy_indices: &[usize],
    floor: i32,
) -> TacticalBattle {
    let arena = generate_arena(floor);

    let mut units = Vec::new();

    // Player unit at bottom-center.
    let px = (ARENA_SIZE / 2) as i32;
    let py = (ARENA_SIZE - 2) as i32;
    let p_speed = player_speed(player.class, player.form, &player.statuses);
    let p_movement = player_movement(player.form, &player.statuses);

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
        radical_dodge: false,
        radical_multiply: false,
        fortify_stacks: 0,
        boss_kind: None,
        is_decoy: false,
    });

    // Enemy units spread across top half.
    let enemy_count = enemy_indices.len();
    for (i, &ei) in enemy_indices.iter().enumerate() {
        let e = &enemies[ei];
        let spacing = ARENA_SIZE as i32 / (enemy_count as i32 + 1);
        let ex = spacing * (i as i32 + 1);
        let ey = 1 + (i as i32 % 2);

        let e_speed = enemy_base_speed(e.is_elite, e.is_boss);
        let e_movement = enemy_base_movement(e.is_elite, e.is_boss);

        units.push(BattleUnit {
            kind: UnitKind::Enemy(ei),
            x: ex.min(ARENA_SIZE as i32 - 1),
            y: ey,
            facing: Direction::South,
            hanzi: e.hanzi,
            pinyin: e.pinyin,
            speed: e_speed,
            movement: e_movement,
            stored_movement: 0,
            hp: e.hp,
            max_hp: e.max_hp,
            damage: e.damage,
            defending: false,
            alive: e.hp > 0,
            ai: e.ai,
            radical_actions: e.radical_actions(),
            statuses: e.statuses.clone(),
            stunned: e.stunned,
            radical_armor: e.radical_armor,
            radical_dodge: e.radical_dodge,
            radical_multiply: e.radical_multiply,
            fortify_stacks: 0,
            boss_kind: e.boss_kind,
            is_decoy: false,
        });
    }

    let turn_queue = build_turn_queue(&units);

    let is_boss_battle = enemy_indices.iter().any(|&ei| enemies[ei].is_boss);

    TacticalBattle {
        arena,
        units,
        turn_queue,
        turn_queue_pos: 0,
        phase: TacticalPhase::Command,
        turn_number: 1,
        combo_streak: 0,
        player_moved: false,
        player_acted: false,
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
        stolen_spells: Vec::new(),
    }
}

/// Scatter some obstacles and terrain based on floor depth.
fn generate_arena(floor: i32) -> TacticalArena {
    let mut arena = TacticalArena::new(ARENA_SIZE, ARENA_SIZE);

    // Simple deterministic obstacle placement based on floor.
    let obstacle_count = 3 + (floor / 3).min(6) as usize;
    let seed = floor as u64;
    for i in 0..obstacle_count {
        let hash = seed
            .wrapping_mul(2654435761)
            .wrapping_add(i as u64)
            .wrapping_mul(2246822519);
        let x = ((hash >> 16) % ARENA_SIZE as u64) as i32;
        let y = (1 + (hash >> 8) % (ARENA_SIZE as u64 - 3)) as i32;
        // Don't place obstacles on spawn points.
        if y <= 0 || y >= (ARENA_SIZE as i32 - 1) {
            continue;
        }
        let mid = ARENA_SIZE as i32 / 2;
        if y == (ARENA_SIZE as i32 - 2) && (x - mid).abs() <= 1 {
            continue;
        }
        arena.set_tile(x, y, BattleTile::Obstacle);
    }

    // Sprinkle some terrain variety on higher floors.
    if floor >= 3 {
        let terrain_count = (floor / 5).min(4) as usize;
        for i in 0..terrain_count {
            let hash = seed
                .wrapping_mul(1103515245)
                .wrapping_add((i + obstacle_count) as u64)
                .wrapping_mul(12345);
            let x = ((hash >> 16) % ARENA_SIZE as u64) as i32;
            let y = (1 + (hash >> 8) % (ARENA_SIZE as u64 - 3)) as i32;
            if arena.tile(x, y) != Some(BattleTile::Open) {
                continue;
            }
            let tile = match hash % 4 {
                0 => BattleTile::Grass,
                1 => BattleTile::Water,
                2 => BattleTile::BrokenGround,
                _ => BattleTile::InkPool,
            };
            arena.set_tile(x, y, tile);
        }
    }

    arena
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
    for unit in &battle.units {
        if let UnitKind::Enemy(ei) = unit.kind {
            if ei < enemies.len() {
                let was_alive = enemies[ei].hp > 0;
                enemies[ei].hp = unit.hp.max(0);
                if was_alive && enemies[ei].hp <= 0 {
                    killed.push(ei);
                }
            }
        }
    }

    player.spirit = (player.spirit - 2).max(0);
    killed
}
