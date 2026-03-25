//! Main game state and loop.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement, KeyboardEvent};

use crate::achievement::AchievementTracker;
use crate::audio::Audio;
use crate::codex::Codex;
use crate::combat;
use crate::world::{compute_fov, TerminalKind, AltarKind, DungeonLevel, RoomModifier, SecuritySeal, SealKind, SpecialRoomKind, Tile};
use crate::world::starmap::{SectorMap, generate_sector, jump_cost};
use crate::world::ship::{ShipLayout, ShipRoom, ShipTile, generate_ship_layout};
use crate::world::events::{ALL_EVENTS, record_event_consequence};
use crate::enemy::{BossKind, Enemy, RadicalAction};
use crate::particle::ParticleSystem;
use crate::player::{
    Faction, EquipEffect, Item, ItemKind, ItemState, Player, PlayerClass, PlayerForm, EQUIPMENT_POOL,
    ITEM_KIND_COUNT, MYSTERY_ITEM_APPEARANCES, Ship, CrewMember, CrewRole,
};
use crate::radical::{self, Spell, SpellEffect};
use crate::render::Renderer;
use crate::srs::SrsTracker;
use crate::status;
use crate::vocab::{self, VocabEntry};

pub(super) const MAP_W: i32 = 48;
pub(super) const MAP_H: i32 = 48;
pub(super) const FOV_RADIUS: i32 = 8;
pub(super) const ENEMIES_PER_ROOM: i32 = 1;
pub(super) const LOOK_RANGE: i32 = 3;

mod console;
mod events;
mod input;
mod serialization;
mod shop;
mod space_combat;
mod ui_state;

pub(crate) use events::*;
pub use shop::*;
pub use space_combat::*;
use serialization::{parse_i32, parse_u32, parse_u64};



mod types;
pub use types::*;
mod data;
use data::*;
mod helpers;
use helpers::*;
mod floor;
mod rooms;
mod movement;
mod combat_core;
mod quests;
mod spells;
mod items;
mod victory;
mod game_render;


pub struct GameState {
    pub level: DungeonLevel,
    pub player: Player,
    pub renderer: Renderer,
    pub audio: Option<Audio>,
    pub floor_num: i32,
    pub seed: u64,
    pub enemies: Vec<Enemy>,
    pub combat: CombatState,
    pub typing: String,
    pub message: String,
    pub message_timer: u8,
    message_tick_delay: u8,
    pub discovered_recipes: Vec<usize>,
    pub best_floor: i32,
    pub srs: SrsTracker,
    pub total_kills: u32,
    pub total_runs: u32,
    /// Move counter for haste effect
    pub move_count: u32,
    /// Particle effects
    pub particles: ParticleSystem,
    /// Screen shake remaining frames
    pub shake_timer: u8,
    /// Flash overlay (r, g, b, alpha 0.0..1.0)
    pub flash: Option<(u8, u8, u8, f64)>,
    /// Achievement tracker
    pub achievements: AchievementTracker,
    /// Achievement popup: (name, desc) + timer frames
    pub achievement_popup: Option<(&'static str, &'static str, u16)>,
    /// Character codex
    pub codex: Codex,
    /// Whether codex overlay is showing
    pub show_codex: bool,
    pub show_inventory: bool,
    pub inventory_cursor: usize,
    pub inventory_inspect: Option<usize>,
    pub show_spellbook: bool,
    pub show_skill_tree: bool,
    pub skill_tree_cursor: usize,
    pub show_crucible: bool,
    pub crucible_cursor: usize,
    pub show_help: bool,
    pub show_minimap: bool,
    item_appearance_order: [usize; ITEM_KIND_COUNT],
    identified_items: [bool; ITEM_KIND_COUNT],
    pub settings: GameSettings,
    pub show_settings: bool,
    pub settings_cursor: usize,
    /// Last spell effect used (for combos)
    pub last_spell: Option<SpellEffect>,
    /// Turns since last spell (combo window)
    pub spell_combo_timer: u8,
    /// Listening mode for audio-based combat
    pub listening_mode: ListenMode,
    /// Active companion NPC
    pub companion: Option<Companion>,
    /// Guard companion: used block this fight?
    pub guard_used_this_fight: bool,
    pub guard_blocks_used: u8,
    pub companion_xp: u32,
    /// Per-companion bond tracking (indexed by Companion::index())
    pub companion_bonds: [CompanionBond; COMPANION_COUNT],
    pub merchant_reroll_used: bool,
    /// Active quests
    pub quests: Vec<Quest>,
    /// Daily challenge mode (fixed seed)
    pub daily_mode: bool,

    /// Active scripted tutorial state for first-time players
    tutorial: Option<TutorialState>,
    rng_state: u64,
    run_kills: u32,
    run_gold_earned: i32,
    run_correct_answers: u32,
    run_wrong_answers: u32,
    run_spells_forged: u32,
    run_bosses_killed: u32,
    pub mirror_hint: bool,
    next_chain_id: u32,
    pub floor_profile: FloorProfile,
    pub answer_streak: u32,
    pub run_journal: RunJournal,
    pub post_mortem_page: usize,
    pub class_cursor: usize,
    /// How many times player has been caught stealing
    pub theft_catches: u32,
    /// Whether this floor's shop is banned (caught stealing)
    pub shop_banned: bool,
    /// Saved TacticalBattle state for boss sentence challenges.
    /// When a boss triggers a sentence challenge mid-tactical-battle,
    /// the battle state is stashed here and restored after the challenge.
    pub saved_battle: Option<Box<combat::TacticalBattle>>,
    /// Quake-style drop-down debug console
    pub console: console::DebugConsole,
    /// God mode (invincible)
    pub god_mode: bool,
    /// Set of (floor, room_x, room_y) for special rooms already activated
    pub completed_special_rooms: HashSet<(i32, i32, i32)>,
    /// Whether the demon deal is active (enemies on next floor are elite)
    pub demon_deal_floors: i32,
    /// Whether the crafting sub-mode is active inside the inventory screen
    pub crafting_mode: bool,
    /// Index (into player.items) of the first item selected for crafting
    pub crafting_first: Option<usize>,
    /// Cursor position while selecting items for crafting
    pub crafting_cursor: usize,
    /// Current game mode (Starmap/ShipInterior/etc.)
    pub game_mode: GameMode,
    /// Sector map for space exploration
    pub sector_map: Option<SectorMap>,
    /// Ship interior layout
    pub ship_layout: ShipLayout,
    /// Ship stats (hull, fuel, shields)
    pub ship: Ship,
    /// Crew members aboard the ship
    pub crew: Vec<CrewMember>,
    /// Current event index (if in Event mode)
    pub current_event: Option<usize>,
    /// Cursor for event choice selection
    pub event_choice_cursor: usize,
    /// Persistent consequence tracking for events
    pub event_memory: EventMemory,
    /// Player position inside ship
    pub ship_player_x: i32,
    pub ship_player_y: i32,
    /// Cursor for starmap system selection
    pub starmap_cursor: usize,
    /// Which LocationType the player is currently exploring (set when entering from starmap)
    pub current_location_type: Option<crate::world::LocationType>,
    /// Whether the class selection overlay is shown on the starmap
    pub show_class_select: bool,
    /// Whether the player has already selected a class
    pub class_selected: bool,
    /// Whether a saved game exists (shows "Continue" option in class select)
    pub has_continue_option: bool,
    /// Ship upgrade shop cursor position
    pub ship_upgrade_cursor: usize,
    /// Whether the ship upgrade shop overlay is showing
    pub show_ship_upgrades: bool,
    /// Whether the ship help overlay is showing
    pub show_ship_help: bool,
    /// Pending crew recruit at a space station
    pub pending_recruit: Option<CrewMember>,
    /// Current enemy ship in space combat
    pub enemy_ship: Option<EnemyShip>,
    /// Current phase of space combat
    pub space_combat_phase: SpaceCombatPhase,
    /// Cursor for space combat action selection
    pub space_combat_cursor: usize,
    /// Battle log messages for space combat
    pub space_combat_log: Vec<String>,
    /// Currently selected weapon for space combat
    pub space_combat_weapon: ShipWeapon,
    /// Currently selected subsystem target
    pub space_combat_target: SubsystemTarget,
    /// Cursor for subsystem target selection
    pub space_combat_target_cursor: usize,
    /// Whether the player is evading this turn
    pub space_combat_evading: bool,
    /// Whether the shop is in sell mode (player sells items)
    pub shop_sell_mode: bool,
}

impl GameState {
    // ── Space combat helpers ──────────────────────────────────────

    pub(super) fn apply_subsystem_damage(enemy: &mut EnemyShip, target: SubsystemTarget, dmg: i32) {
        match target {
            SubsystemTarget::Weapons => enemy.weapons_sub.hp -= dmg,
            SubsystemTarget::Shields => enemy.shields_sub.hp -= dmg,
            SubsystemTarget::Engines => enemy.engines_sub.hp -= dmg,
            SubsystemTarget::Hull    => {
                // Hull target: absorb with shields first, then hull
                let shield_absorb = dmg.min(enemy.shields);
                enemy.shields -= shield_absorb;
                let hull_dmg = dmg - shield_absorb;
                enemy.hull -= hull_dmg;
            }
        }
    }

    pub(super) fn apply_subsystem_effects(enemy: &mut EnemyShip) {
        if enemy.weapons_sub.is_destroyed() {
            enemy.weapon_power = (enemy.weapon_power / 2).max(1);
        }
        if enemy.shields_sub.is_destroyed() {
            enemy.shields = 0;
            enemy.max_shields = 0;
        }
        // Hull subsystem acts as the overall hull check
        if enemy.weapons_sub.is_destroyed() && enemy.shields_sub.is_destroyed() && enemy.engines_sub.is_destroyed() {
            // All subsystems destroyed — crippled, auto-damage hull
            enemy.hull -= 5;
        }
    }

    pub(super) fn enemy_fires(enemy: &mut EnemyShip, s: &mut GameState, evading: bool) {
        // Adapt tactic based on current battle state before firing.
        let player_shield_pct = s.ship.shields as f64 / s.ship.max_shields.max(1) as f64;
        let player_hull_pct = s.ship.hull as f64 / s.ship.max_hull.max(1) as f64;
        if let Some(adapt_msg) = enemy.adapt_space_combat_tactic(player_shield_pct, player_hull_pct) {
            s.space_combat_log.push(format!("{} {}", enemy.name, adapt_msg));
        }

        let mut e_dmg = enemy.weapon_power;
        if enemy.weapons_sub.is_destroyed() {
            e_dmg = (e_dmg / 2).max(1);
        }
        if evading {
            e_dmg = (e_dmg / 2).max(1);
        }

        // Determine tactic-based message
        let tactic_msg = match enemy.tactic {
            EnemyTactic::Aggressive => {
                "targets your hull!"
            }
            EnemyTactic::Disabling => {
                if enemy.turns_taken % 2 == 0 {
                    // Reduce player weapon power temporarily
                    s.ship.weapon_power = (s.ship.weapon_power - 1).max(1);
                    "targets your weapons! (-1 weapon power)"
                } else {
                    s.ship.engine_power = (s.ship.engine_power - 1).max(1);
                    "targets your engines! (-1 engine power)"
                }
            }
            EnemyTactic::Balanced => {
                match enemy.turns_taken % 3 {
                    0 => "fires a focused volley!",
                    1 => {
                        s.ship.weapon_power = (s.ship.weapon_power - 1).max(1);
                        "strafes your weapons!"
                    }
                    _ => "fires a broadside!",
                }
            }
            EnemyTactic::Boarding => {
                if s.ship.weapon_power <= 2 {
                    // Attempt boarding — extra hull damage
                    e_dmg += 3;
                    "attempts boarding action!"
                } else {
                    "fires cautiously."
                }
            }
        };

        let e_shield_absorb = e_dmg.min(s.ship.shields);
        s.ship.shields -= e_shield_absorb;
        let e_hull_dmg = e_dmg - e_shield_absorb;
        s.ship.hull -= e_hull_dmg;
        s.space_combat_log.push(format!("{} {} {} dmg ({} shield, {} hull)",
            enemy.name, tactic_msg, e_dmg, e_shield_absorb, e_hull_dmg));
        if let Some(ref audio) = s.audio { audio.play_enemy_weapon_fire(); }
    }

    /// Convert tile position to screen coordinates for particles.
    pub(super) fn tile_to_screen(&self, tx: i32, ty: i32) -> (f64, f64) {
        let cam_x = self.player.x as f64 * 24.0 - self.renderer.canvas_w / 2.0 + 12.0;
        let cam_y = self.player.y as f64 * 24.0 - self.renderer.canvas_h / 2.0 + 12.0;
        (
            tx as f64 * 24.0 - cam_x + 12.0,
            ty as f64 * 24.0 - cam_y + 12.0,
        )
    }

    pub(super) fn rng_next(&mut self) -> u64 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        x
    }

    pub(super) fn crew_bonus(&self, role: CrewRole) -> i32 {
        self.crew
            .iter()
            .filter(|c| c.role == role && c.hp > 0)
            .map(|c| {
                let morale_mult = if c.morale >= 80 { 120 }
                    else if c.morale >= 50 { 100 }
                    else if c.morale >= 20 { 70 }
                    else { 40 };
                (c.skill * morale_mult) / 100
            })
            .sum()
    }

    #[allow(dead_code)]
    pub(super) fn generate_recruit(&mut self) -> CrewMember {
        let seed = self.rng_next() as u32;
        let roles = CrewRole::all();
        let role = roles[(seed as usize) % roles.len()];
        let names = [
            "Zara", "Kael", "Nova", "Rexx", "Lyra", "Voss", "Juno", "Talon", "Echo", "Blaze",
        ];
        let name = names[(seed as usize / 6) % names.len()];
        CrewMember {
            name: name.to_string(),
            role,
            hp: 10,
            max_hp: 10,
            level: 1,
            xp: 0,
            skill: 2 + (seed % 3) as i32,
            morale: 80,
        }
    }


    pub(super) fn make_player(&mut self, x: i32, y: i32, class: PlayerClass) -> Player {
        let mut player = Player::new(x, y, class);
        match class {
            PlayerClass::Soldier => {
                player.weapon = Some(&crate::player::EQUIPMENT_POOL[0]); // Brush of Clarity
            }
            PlayerClass::Mystic => {
                player.items.push(crate::player::Item::MedHypo(5));
                player.item_states.push(ItemState::Normal);
                player.items.push(crate::player::Item::MedHypo(5));
                player.item_states.push(ItemState::Normal);
            }
            PlayerClass::Operative => {
                player.gold += 20;
            }
            PlayerClass::Envoy => {
                player.gold += 15;
                player.shop_discount_pct = 20;
            }
            PlayerClass::Technomancer => {
                player.radicals.push("木");
                player.radicals.push("水");
                player.radicals.push("火");
            }
            _ => {}
        }
        player
    }

    pub(super) fn companion_level(&self) -> u8 {
        if self.companion.is_some() {
            Companion::level_from_xp(self.companion_xp)
        } else {
            0
        }
    }

    pub(super) fn add_companion_xp(&mut self, amount: u32) {
        if self.companion.is_some() {
            let old_level = self.companion_level();
            self.companion_xp = self.companion_xp.saturating_add(amount);
            let new_level = self.companion_level();
            if new_level > old_level {
                if let Some(ref comp) = self.companion {
                    self.message = format!(
                        "{} {} reached level {}!",
                        comp.icon(),
                        comp.name(),
                        new_level
                    );
                    self.message_timer = 90;
                }
            }
        }
    }

    /// Current companion's synergy bond level (0-3).
    pub(super) fn companion_synergy_level(&self) -> u8 {
        match self.companion {
            Some(c) => self.companion_bonds[c.index()].synergy_level,
            None => 0,
        }
    }

    /// Advance the current companion's bond and show a message on level-up.
    pub(super) fn advance_companion_bond(&mut self) {
        if let Some(comp) = self.companion {
            let bond = &mut self.companion_bonds[comp.index()];
            let old_level = bond.synergy_level;
            bond.advance_floor();
            let new_level = bond.synergy_level;
            if new_level > old_level {
                self.message = match new_level {
                    1 => format!(
                        "🤝 Bond Lv1 with {}! Combat Callouts unlocked!",
                        comp.name()
                    ),
                    2 => format!(
                        "🤝 Bond Lv2 with {}! Passive Bonus unlocked!",
                        comp.name()
                    ),
                    3 => format!(
                        "🤝 Bond Lv3 with {}! {} unlocked!",
                        comp.name(),
                        comp.combo_ability_name()
                    ),
                    _ => String::new(),
                };
                self.message_timer = 120;
            }
        }
    }

    pub(super) fn effective_shop_discount_pct(&self) -> i32 {
        let mut discount = self.player.shop_discount_pct;
        if let Some(ref comp) = self.companion {
            discount += comp.shop_discount_pct(self.companion_level());
        }
        // TradingPost location bonus: 25% shop discount
        if self.current_location_type == Some(crate::world::LocationType::TradingPost) {
            discount += 25;
        }
        // Quartermaster crew bonus: +5% per skill point
        discount += self.crew_bonus(CrewRole::Quartermaster) * 5;
        discount.clamp(0, 50)
    }

    pub(super) fn companion_exploration_hint(&mut self) {
        let comp = match self.companion {
            Some(c) => c,
            None => return,
        };
        if self.message_timer > 20 {
            return;
        }
        if !matches!(self.combat, CombatState::Explore) {
            return;
        }

        let w = self.level.width;
        let h = self.level.height;
        let level = self.companion_level();

        let hint: Option<String> = match comp {
            Companion::ScienceOfficer => {
                let mut forge_visible = false;
                for y in 0..h {
                    for x in 0..w {
                        let idx = y * w + x;
                        if self.level.visible[idx as usize]
                            && self.level.tiles[idx as usize] == Tile::QuantumForge
                        {
                            forge_visible = true;
                            break;
                        }
                    }
                    if forge_visible {
                        break;
                    }
                }
                if forge_visible && !self.player.radicals.is_empty() {
                    Some("📚 I see a forge! You have radicals to combine.".to_string())
                } else {
                    None
                }
            }
            Companion::Medic => {
                if self.player.hp <= self.player.effective_max_hp() / 3 {
                    let mut shrine_visible = false;
                    for y in 0..h {
                        for x in 0..w {
                            let idx = y * w + x;
                            if self.level.visible[idx as usize] {
                                let t = self.level.tiles[idx as usize];
                                if t == Tile::CircuitShrine
                                    || matches!(t, Tile::Terminal(_))
                                    || t == Tile::MemorialNode
                                {
                                    shrine_visible = true;
                                    break;
                                }
                            }
                        }
                        if shrine_visible {
                            break;
                        }
                    }
                    if shrine_visible {
                        let heal = Companion::Medic.heal_per_floor(level);
                        Some(format!(
                            "🧘 A shrine nearby — rest may help. I'll mend {} HP next floor.",
                            heal
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Companion::Quartermaster => {
                if self.floor_profile.radical_drop_bonus() {
                    let mut chest_visible = false;
                    for y in 0..h {
                        for x in 0..w {
                            let idx = y * w + x;
                            if self.level.visible[idx as usize]
                                && self.level.tiles[idx as usize] == Tile::SupplyCrate
                            {
                                chest_visible = true;
                                break;
                            }
                        }
                        if chest_visible {
                            break;
                        }
                    }
                    if chest_visible {
                        Some("💰 Chest spotted on a rich floor — extra loot likely!".to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Companion::SecurityChief => {
                let px = self.player.x;
                let py = self.player.y;
                let alert_count = self
                    .enemies
                    .iter()
                    .filter(|e| {
                        e.is_alive()
                            && e.alert
                            && (e.x - px).abs() <= FOV_RADIUS
                            && (e.y - py).abs() <= FOV_RADIUS
                    })
                    .count();
                if alert_count >= 3 {
                    let blocks = Companion::SecurityChief.guard_max_blocks(level);
                    Some(format!(
                        "🛡 {} enemies closing in! I can block {} hit{}.",
                        alert_count,
                        blocks,
                        if blocks > 1 { "s" } else { "" }
                    ))
                } else {
                    None
                }
            }
        };

        if let Some(text) = hint {
            self.message = text;
            self.message_timer = 70;
        }
    }


    pub(super) fn roll_item_appearance_order(seed: u64) -> [usize; ITEM_KIND_COUNT] {
        let mut order = core::array::from_fn(|idx| idx);
        let mut state = seed ^ 0x9e37_79b9_7f4a_7c15;
        for i in (1..ITEM_KIND_COUNT).rev() {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let j = (state as usize) % (i + 1);
            order.swap(i, j);
        }
        order
    }

    pub(super) fn reset_item_lore(&mut self) {
        self.identified_items = [false; ITEM_KIND_COUNT];
        self.item_appearance_order = Self::roll_item_appearance_order(self.seed);
    }

    pub(super) fn item_appearance(&self, kind: ItemKind) -> &'static str {
        MYSTERY_ITEM_APPEARANCES[self.item_appearance_order[kind.index()]]
    }

    pub(super) fn item_is_identified(&self, kind: ItemKind) -> bool {
        self.identified_items[kind.index()]
    }

    pub(super) fn item_display_name(&self, item: &crate::player::Item) -> String {
        item.display_name(
            self.item_is_identified(item.kind()),
            self.item_appearance(item.kind()),
        )
    }

    pub(super) fn identify_item_kind(&mut self, kind: ItemKind) -> bool {
        let idx = kind.index();
        let newly_identified = !self.identified_items[idx];
        self.identified_items[idx] = true;
        newly_identified
    }

    pub(super) fn vocab_entry_by_hanzi(hanzi: &str) -> Option<&'static VocabEntry> {
        vocab::VOCAB.iter().find(|entry| entry.hanzi == hanzi)
    }

}


pub fn init_game() -> Result<(), JsValue> {
    let win = window().ok_or("no window")?;
    let doc = win.document().ok_or("no document")?;

    // Create canvas — fill the browser window
    let canvas: HtmlCanvasElement = doc.create_element("canvas")?.dyn_into()?;
    canvas.set_id("game-canvas");
    let inner_w = win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(800.0) as u32;
    let inner_h = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(600.0) as u32;
    canvas.set_width(inner_w);
    canvas.set_height(inner_h);
    canvas.set_attribute(
        "style",
        "display:block; width:100vw; height:100vh; background:#0d0b14; image-rendering:pixelated; position:fixed; top:0; left:0;",
    )?;
    doc.body().unwrap().append_child(&canvas)?;

    // Remove loading indicator
    if let Some(el) = doc.get_element_by_id("loading") {
        el.remove();
    }

    let renderer = Renderer::new(canvas).map_err(|e| JsValue::from_str(e))?;

    let seed = win.performance().map(|p| p.now() as u64).unwrap_or(42);
    let level = DungeonLevel::generate(MAP_W, MAP_H, seed, 1, crate::world::LocationType::OrbitalPlatform);
    let (sx, sy) = level.start_pos();
    let player = Player::new(sx, sy, PlayerClass::Envoy);

    let best_floor = GameState::load_high_score();
    let srs = crate::srs::load_srs();
    let settings = GameState::load_settings();
    let mut audio = Audio::new();
    if let Some(ref mut audio) = audio {
        audio.set_music_volume(settings.music_volume);
        audio.set_sfx_volume(settings.sfx_volume);
    }
    let total_runs = GameState::load_stat("radical_roguelike_runs");
    let total_kills = GameState::load_stat("radical_roguelike_kills");
    let item_appearance_order = GameState::roll_item_appearance_order(seed);

    let state = Rc::new(RefCell::new(GameState {
        level,
        player,
        renderer,
        audio,
        floor_num: 1,
        seed,
        enemies: Vec::new(),
        combat: CombatState::ClassSelect,
        typing: String::new(),
        message: String::new(),
        message_timer: 0,
        message_tick_delay: 0,
        discovered_recipes: GameState::load_recipes(),
        best_floor,
        srs,
        total_kills,
        total_runs,
        move_count: 0,
        particles: ParticleSystem::new(),
        shake_timer: 0,
        flash: None,
        achievements: AchievementTracker::load(),
        achievement_popup: None,
        codex: Codex::load(&vocab::VOCAB),
        show_codex: false,
        show_inventory: false,
        inventory_cursor: 0,
        inventory_inspect: None,
            show_spellbook: false,
            show_skill_tree: false,
            skill_tree_cursor: 0,
            show_crucible: false,
            crucible_cursor: 0,
            show_help: false,
        show_minimap: true,
        item_appearance_order,
        identified_items: [false; ITEM_KIND_COUNT],
        settings,
        show_settings: false,
        settings_cursor: 0,
        last_spell: None,
        spell_combo_timer: 0,
        listening_mode: ListenMode::Off,
        companion: None,
        guard_used_this_fight: false,
        guard_blocks_used: 0,
        companion_xp: 0,
        companion_bonds: [CompanionBond::default(); COMPANION_COUNT],
        merchant_reroll_used: false,
        quests: Vec::new(),
        daily_mode: false,

        tutorial: None,
        rng_state: seed,
        run_kills: 0,
        run_gold_earned: 0,
        run_correct_answers: 0,
        run_wrong_answers: 0,
        run_spells_forged: 0,
        run_bosses_killed: 0,
        mirror_hint: false,
        next_chain_id: 1,
        floor_profile: FloorProfile::Normal,
        answer_streak: 0,
        run_journal: RunJournal::default(),
        post_mortem_page: 0,
        class_cursor: 0,
        theft_catches: 0,
        shop_banned: false,
        saved_battle: None,
        console: console::DebugConsole::new(),
        god_mode: false,
        completed_special_rooms: HashSet::new(),
        demon_deal_floors: 0,
        crafting_mode: false,
        crafting_first: None,
        crafting_cursor: 0,
        game_mode: GameMode::Starmap,
        sector_map: Some({
            let s = generate_sector(0, 1, seed as u32);
            let start = s.start_system;
            SectorMap {
                sectors: vec![s],
                current_sector: 0,
                current_system: start,
            }
        }),
        ship_layout: generate_ship_layout(),
        ship: Ship {
            hull: 100,
            max_hull: 100,
            fuel: 50,
            max_fuel: 100,
            shields: 20,
            max_shields: 50,
            weapon_power: 10,
            engine_power: 10,
            sensor_range: 2,
            cargo_capacity: 100,
            cargo_used: 0,
            upgrades: Vec::new(),
        },
        crew: vec![
            CrewMember {
                name: "First Officer Chen".to_string(),
                role: CrewRole::Pilot,
                hp: 10,
                max_hp: 10,
                level: 1,
                xp: 0,
                morale: 50,
                skill: 1,
            },
            CrewMember {
                name: "Engineer Rodriguez".to_string(),
                role: CrewRole::Engineer,
                hp: 10,
                max_hp: 10,
                level: 1,
                xp: 0,
                morale: 50,
                skill: 1,
            },
        ],
        current_event: None,
        event_choice_cursor: 0,
        event_memory: EventMemory::default(),
        ship_player_x: 11,
        ship_player_y: 16,
        starmap_cursor: 0,
        current_location_type: None,
        show_class_select: true,
        class_selected: false,
        has_continue_option: GameState::has_save(),
        ship_upgrade_cursor: 0,
        show_ship_upgrades: false,
        show_ship_help: false,
        pending_recruit: None,
        enemy_ship: None,
        space_combat_phase: SpaceCombatPhase::Choosing,
        space_combat_cursor: 0,
        space_combat_log: vec![],
        space_combat_weapon: ShipWeapon::Laser,
        space_combat_target: SubsystemTarget::Hull,
        space_combat_target_cursor: 0,
        space_combat_evading: false,
        shop_sell_mode: false,
    }));

    // Initial setup
    {
        let s = state.borrow_mut();
        // Don't spawn enemies yet — class selection first
        s.render();
    }

    // Keyboard input
    {
        let state = Rc::clone(&state);
        let closure = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            // Allow IME and OS shortcut combos (Ctrl+Space, Ctrl+Shift, etc.) to pass through
            if event.ctrl_key() || event.alt_key() || event.meta_key() {
                return;
            }

            let key = event.key();
            let Ok(mut s) = state.try_borrow_mut() else {
                return;
            };

            // Resume audio context on first interaction (browser requirement)
            if let Some(ref audio) = s.audio {
                audio.resume();
            }

            // Cheat console toggle
            if key == "`" || key == "Dead" {
                event.prevent_default();
                s.console.active = !s.console.active;
                if let Some(ref audio) = s.audio {
                    audio.play_console_toggle();
                }
                if s.console.active {
                    s.console.input_buffer.clear();
                }
                s.render();
                return;
            }

            // Console input handling — intercepts ALL keys when console is open
            if s.console.active {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.console.active = false;
                    }
                    "Enter" => {
                        let cmd = s.console.input_buffer.trim().to_string();
                        if !cmd.is_empty() {
                            s.console.cmd_history.push(cmd.clone());
                            s.console.cmd_index = None;
                            s.execute_console_command(&cmd);
                            s.console.input_buffer.clear();
                        }
                    }
                    "Backspace" => {
                        s.console.input_buffer.pop();
                    }
                    "ArrowUp" => {
                        if !s.console.cmd_history.is_empty() {
                            let idx = match s.console.cmd_index {
                                Some(i) => i.saturating_sub(1),
                                None => s.console.cmd_history.len() - 1,
                            };
                            s.console.cmd_index = Some(idx);
                            s.console.input_buffer = s.console.cmd_history[idx].clone();
                        }
                    }
                    "ArrowDown" => {
                        if let Some(idx) = s.console.cmd_index {
                            if idx + 1 < s.console.cmd_history.len() {
                                let new_idx = idx + 1;
                                s.console.cmd_index = Some(new_idx);
                                s.console.input_buffer = s.console.cmd_history[new_idx].clone();
                            } else {
                                s.console.cmd_index = None;
                                s.console.input_buffer.clear();
                            }
                        }
                    }
                    "PageUp" => {
                        s.console.scroll_up(5);
                    }
                    "PageDown" => {
                        s.console.scroll_down(5);
                    }
                    "Tab" => {
                        s.tab_complete();
                    }
                    _ => {
                        // Reset tab completion state on any non-Tab key
                        s.console.tab_prefix.clear();
                        s.console.tab_matches.clear();
                        s.console.tab_cycle_index = 0;
                        if key.len() == 1 {
                            s.console.input_buffer.push_str(&key);
                        } else if key == "Space" {
                            s.console.input_buffer.push(' ');
                        }
                    }
                }
                s.render();
                return;
            }

            // ── Game mode input dispatch ──
            match s.game_mode {
                GameMode::Starmap => {
                    event.prevent_default();

                    // Class selection overlay intercepts all input
                    if s.show_class_select {
                        let continue_offset = if s.has_continue_option { 1 } else { 0 };
                        let total = crate::player::PlayerClass::all().len() + continue_offset;
                        match key.as_str() {
                            "ArrowUp" | "w" | "W" => {
                                if s.class_cursor == 0 {
                                    s.class_cursor = total - 1;
                                } else {
                                    s.class_cursor -= 1;
                                }
                            }
                            "ArrowDown" | "s" | "S" => {
                                s.class_cursor = (s.class_cursor + 1) % total;
                            }
                            "Enter" | " " => {
                                if s.has_continue_option && s.class_cursor == 0 {
                                    // Continue previous game
                                    if let Some(save_data) = GameState::load_game_data() {
                                        s.player.hp = parse_i32(&save_data, "hp", s.player.hp);
                                        s.player.max_hp = parse_i32(&save_data, "max_hp", s.player.max_hp);
                                        s.player.gold = parse_i32(&save_data, "gold", s.player.gold);
                                        s.floor_num = parse_i32(&save_data, "floor", s.floor_num);
                                        s.best_floor = parse_i32(&save_data, "best", s.best_floor);
                                        // Spirit fields removed
                                        let class_id = parse_u32(&save_data, "class", 0);
                                        let all_classes = crate::player::PlayerClass::all();
                                        let class = all_classes.get(class_id as usize).copied().unwrap_or(crate::player::PlayerClass::Soldier);
                                        let (px, py) = (s.player.x, s.player.y);
                                        s.player = Player::new(px, py, class);
                                        // Re-apply saved stats on top of fresh player
                                        s.player.hp = parse_i32(&save_data, "hp", s.player.hp);
                                        s.player.max_hp = parse_i32(&save_data, "max_hp", s.player.max_hp);
                                        s.player.gold = parse_i32(&save_data, "gold", s.player.gold);
                                        // Ship
                                        s.ship.hull = parse_i32(&save_data, "ship_hull", s.ship.hull);
                                        s.ship.max_hull = parse_i32(&save_data, "ship_max_hull", s.ship.max_hull);
                                        s.ship.fuel = parse_i32(&save_data, "ship_fuel", s.ship.fuel);
                                        s.ship.max_fuel = parse_i32(&save_data, "ship_max_fuel", s.ship.max_fuel);
                                        s.ship.shields = parse_i32(&save_data, "ship_shields", s.ship.shields);
                                        s.ship.max_shields = parse_i32(&save_data, "ship_max_shields", s.ship.max_shields);
                                        s.ship.weapon_power = parse_i32(&save_data, "ship_weapon", s.ship.weapon_power);
                                        s.ship.engine_power = parse_i32(&save_data, "ship_engine", s.ship.engine_power);
                                        s.ship.sensor_range = parse_i32(&save_data, "ship_sensor", s.ship.sensor_range);
                                        s.ship.cargo_capacity = parse_i32(&save_data, "ship_cargo_cap", s.ship.cargo_capacity);
                                        s.ship.cargo_used = parse_i32(&save_data, "ship_cargo_used", s.ship.cargo_used);
                                        // Sector map position
                                        if let Some(ref mut map) = s.sector_map {
                                            map.current_sector = parse_u32(&save_data, "sector", map.current_sector as u32) as usize;
                                            map.current_system = parse_u32(&save_data, "system", map.current_system as u32) as usize;
                                        }
                                        // Stats
                                        s.total_kills = parse_u32(&save_data, "kills", s.total_kills);
                                        s.total_runs = parse_u32(&save_data, "runs", s.total_runs);
                                        s.seed = parse_u64(&save_data, "seed", s.seed);

                                        // Skill tree
                                        if let Some(st_json) = save_data.get("skill_tree") {
                                            if let Some(st) = crate::skill_tree::SkillTreeState::from_json(st_json) {
                                                s.player.skill_tree = st;
                                            }
                                        }

                                        // Crucible states
                                        if let Some(json) = save_data.get("weapon_crucible") {
                                            s.player.weapon_crucible = crate::crucible::CrucibleState::from_json(json);
                                        }
                                        if let Some(json) = save_data.get("armor_crucible") {
                                            s.player.armor_crucible = crate::crucible::CrucibleState::from_json(json);
                                        }
                                        if let Some(json) = save_data.get("charm_crucible") {
                                            s.player.charm_crucible = crate::crucible::CrucibleState::from_json(json);
                                        }

                                        // Equipment rarity
                                        if let Some(json) = save_data.get("weapon_rarity") {
                                            s.player.weapon_rarity = crate::rarity::ItemRarity::from_json(json);
                                        }
                                        if let Some(json) = save_data.get("armor_rarity") {
                                            s.player.armor_rarity = crate::rarity::ItemRarity::from_json(json);
                                        }
                                        if let Some(json) = save_data.get("charm_rarity") {
                                            s.player.charm_rarity = crate::rarity::ItemRarity::from_json(json);
                                        }

                                        // Equipment affixes
                                        if let Some(json) = save_data.get("weapon_affixes") {
                                            s.player.weapon_affixes = crate::rarity::affixes_from_json(json);
                                        }
                                        if let Some(json) = save_data.get("armor_affixes") {
                                            s.player.armor_affixes = crate::rarity::affixes_from_json(json);
                                        }
                                        if let Some(json) = save_data.get("charm_affixes") {
                                            s.player.charm_affixes = crate::rarity::affixes_from_json(json);
                                        }

                                        s.show_class_select = false;
                                        s.class_selected = true;
                                        s.message = "Welcome back, Commander!".to_string();
                                        s.message_timer = 90;
                                    }
                                } else {
                                    let class_idx = if s.has_continue_option { s.class_cursor - 1 } else { s.class_cursor };
                                    let all = crate::player::PlayerClass::all();
                                    let class = all[class_idx];
                                    let (px, py) = (s.player.x, s.player.y);
                                    s.player = Player::new(px, py, class);
                                    s.show_class_select = false;
                                    s.class_selected = true;
                                    let data = class.data();
                                    s.message = format!("Class selected: {}!", data.name_en);
                                    s.message_timer = 90;
                                }
                            }
                            _ => {}
                        }
                        s.render();
                        return;
                    }

                    // Collect data first to avoid borrow conflicts
                    let (connections, current, current_name, current_loc_type) = if let Some(ref map) = s.sector_map {
                        let sector = &map.sectors[map.current_sector];
                        let current = map.current_system;
                        let connections = if current < sector.systems.len() {
                            sector.systems[current].connections.clone()
                        } else {
                            vec![]
                        };
                        let name = if current < sector.systems.len() {
                            sector.systems[current].name
                        } else {
                            "Unknown"
                        };
                        let loc_type = if current < sector.systems.len() {
                            sector.systems[current].location_type
                        } else {
                            crate::world::LocationType::OrbitalPlatform
                        };
                        (connections, current, name, loc_type)
                    } else {
                        (vec![], 0, "Unknown", crate::world::LocationType::OrbitalPlatform)
                    };

                    match key.as_str() {
                        "ArrowLeft" | "a" | "A" => {
                            // Move cursor to previous connected system
                            if !connections.is_empty() {
                                if s.starmap_cursor == 0 {
                                    s.starmap_cursor = connections.len() - 1;
                                } else {
                                    s.starmap_cursor -= 1;
                                }
                            }
                        }
                        "ArrowRight" | "d" | "D" => {
                            if !connections.is_empty() {
                                s.starmap_cursor = (s.starmap_cursor + 1) % connections.len();
                            }
                        }
                        "Enter" | " " => {
                            // Jump to selected system
                            if !connections.is_empty() && s.sector_map.is_some() {
                                let target = connections[s.starmap_cursor % connections.len()];
                                // Collect jump data
                                let (fuel_cost, target_name, event_id, hazard_dmg, hazard_msg) = {
                                    let map = s.sector_map.as_ref().unwrap();
                                    let sector = &map.sectors[map.current_sector];
                                    if target < sector.systems.len() && current < sector.systems.len() {
                                        let from = &sector.systems[current];
                                        let to = &sector.systems[target];
                                        let cost = jump_cost(from, to);
                                        let name = to.name;
                                        let evt = to.event_id;
                                        let (h_dmg, h_fuel, h_msg) = if let Some(ref hazard) = to.hazard {
                                            (hazard.hull_damage(), hazard.fuel_modifier(), 
                                             format!(" ⚠ {}! {}", hazard.name(), hazard.description()))
                                        } else {
                                            (0, 0, String::new())
                                        };
                                        (cost + h_fuel, name, evt, h_dmg, h_msg)
                                    } else {
                                        (0, "Unknown", None, 0, String::new())
                                    }
                                };
                                
                                // Pilot crew bonus: reduce fuel cost by 1 (min 1)
                                let pilot_bonus = if s.crew_bonus(CrewRole::Pilot) > 0 { 1 } else { 0 };
                                let fuel_cost = (fuel_cost - pilot_bonus).max(1);

                                if s.ship.fuel >= fuel_cost {
                                    s.ship.fuel -= fuel_cost;
                                    if let Some(ref audio) = s.audio { audio.sfx_jump(); }
                                    if hazard_dmg > 0 {
                                        s.ship.hull = (s.ship.hull - hazard_dmg).max(0);
                                    }
                                    // Engineer crew bonus: repair hull on jump
                                    let eng_repair = s.crew_bonus(CrewRole::Engineer);
                                    if eng_repair > 0 {
                                        s.ship.hull = (s.ship.hull + eng_repair).min(s.ship.max_hull);
                                    }
                                    if let Some(ref mut map) = s.sector_map {
                                        map.current_system = target;
                                        if target < map.sectors[map.current_sector].systems.len() {
                                            map.sectors[map.current_sector].systems[target].visited = true;
                                        }
                                    }
                                    s.starmap_cursor = 0;
                                    s.message = format!("Jumped to {}! (-{} fuel){}", target_name, fuel_cost, hazard_msg);
                                    s.message_timer = 90;
                                    // Apply passive ship upgrade effects on jump
                                    if s.ship.upgrades.contains(&crate::world::ship::ShipUpgrade::AutoRepairDrone) {
                                        s.ship.hull = (s.ship.hull + 2).min(s.ship.max_hull);
                                    }
                                    if s.ship.upgrades.contains(&crate::world::ship::ShipUpgrade::MedicalBay) {
                                        for crew in s.crew.iter_mut() {
                                            crew.hp = (crew.hp + 2).min(crew.max_hp);
                                            crew.morale = (crew.morale + 3).min(100);
                                        }
                                    }
                                    // Check for events at the new system
                                    if let Some(event_id) = event_id {
                                        // Use memory-aware selection: may swap in a conditional event
                                        let actual_id = crate::world::events::select_event_with_memory(
                                            event_id, &s.event_memory, s.seed as u32,
                                        );
                                        s.current_event = Some(actual_id);
                                        s.event_choice_cursor = 0;
                                        s.game_mode = GameMode::Event;
                                    }
                                    // 20% chance of pirate encounter after jump (only if no event)
                                    if s.game_mode != GameMode::Event {
                                        let encounter_roll = (s.seed.wrapping_mul(1664525).wrapping_add(1013904223)) % 100;
                                        s.seed = s.seed.wrapping_mul(1664525).wrapping_add(1013904223);
                                        if encounter_roll < 20 {
                                            let difficulty = s.floor_num as i32 + 1;
                                            let names = ["Pirate Raider", "Void Corsair", "Rogue Frigate", "Scav Interceptor"];
                                            let ship_name_idx = (encounter_roll as usize / 5) % 4;
                                            let sub_hp = 10 + difficulty * 5;
                                            s.enemy_ship = Some(EnemyShip {
                                                name: names[ship_name_idx].to_string(),
                                                hull: 30 + difficulty * 10,
                                                max_hull: 30 + difficulty * 10,
                                                shields: 10 + difficulty * 5,
                                                max_shields: 10 + difficulty * 5,
                                                weapon_power: 3 + difficulty,
                                                engine_power: 2 + difficulty / 2,
                                                loot_credits: 50 + difficulty * 20,
                                                weapons_sub: Subsystem::new(sub_hp),
                                                shields_sub: Subsystem::new(sub_hp),
                                                engines_sub: Subsystem::new(sub_hp),
                                                tactic: match ship_name_idx % 4 {
                                                    0 => EnemyTactic::Aggressive,
                                                    1 => EnemyTactic::Disabling,
                                                    2 => EnemyTactic::Balanced,
                                                    _ => EnemyTactic::Boarding,
                                                },
                                                turns_taken: 0,
                                                is_boss: false,
                                                initial_tactic: match ship_name_idx % 4 {
                                                    0 => EnemyTactic::Aggressive,
                                                    1 => EnemyTactic::Disabling,
                                                    2 => EnemyTactic::Balanced,
                                                    _ => EnemyTactic::Boarding,
                                                },
                                            });
                                            s.space_combat_phase = SpaceCombatPhase::Choosing;
                                            s.space_combat_cursor = 0;
                                            s.space_combat_weapon = ShipWeapon::Laser;
                                            s.space_combat_target = SubsystemTarget::Hull;
                                            s.space_combat_target_cursor = 0;
                                            s.space_combat_evading = false;
                                            s.space_combat_log = vec!["Enemy ship detected!".to_string()];
                                            s.game_mode = GameMode::SpaceCombat;
                                        }
                                    }
                                    s.save_game();
                                } else {
                                    s.message = format!("Not enough fuel! Need {} fuel.", fuel_cost);
                                    s.message_timer = 90;
                                }
                            }
                        }
                        "e" | "E" => {
                            // Enter current system (explore the location)
                            s.current_location_type = Some(current_loc_type);
                            s.game_mode = GameMode::LocationExploration;
                            s.level = DungeonLevel::generate(MAP_W, MAP_H, s.seed, s.floor_num, current_loc_type);
                            let (sx, sy) = s.level.start_pos();
                            s.player.move_to(sx, sy);
                            s.enemies.clear();
                            s.combat = CombatState::Explore;
                            s.spawn_enemies();
                            let (px, py) = (s.player.x, s.player.y);
                            compute_fov(&mut s.level, px, py, FOV_RADIUS);
                            // Apply location entry bonuses
                            match current_loc_type {
                                crate::world::LocationType::SpaceStation => {
                                    s.player.hp = s.player.effective_max_hp();
                                    // Crew recruitment at space stations
                                    if s.crew.len() < 6 {
                                        let recruit = s.generate_recruit();
                                        let recruit_info = format!(
                                            " | {} ({}, skill {}) wants to join! Press R to recruit.",
                                            recruit.name, recruit.role.name(), recruit.skill
                                        );
                                        s.pending_recruit = Some(recruit);
                                        s.message = format!(
                                            "Entering {} (Space Station) \u{2014} Fully healed!{}",
                                            current_name, recruit_info
                                        );
                                    } else {
                                        s.pending_recruit = None;
                                        s.message = format!("Entering {} (Space Station) \u{2014} Fully healed!", current_name);
                                    }
                                }
                                crate::world::LocationType::OrbitalPlatform => {
                                    s.ship.shields = s.ship.max_shields;
                                    s.message = format!("Entering {} (Orbital Platform) \u{2014} Shields recharged!", current_name);
                                }
                                crate::world::LocationType::DerelictShip => {
                                    s.message = format!("Entering {} (Derelict Ship) \u{2014} Enemies are stronger here, but loot is better!", current_name);
                                }
                                crate::world::LocationType::MiningColony => {
                                    s.message = format!("Entering {} (Mining Colony) \u{2014} Extra credits per kill!", current_name);
                                }
                                crate::world::LocationType::ResearchLab => {
                                    s.message = format!("Entering {} (Research Lab) \u{2014} Double vocab XP!", current_name);
                                }
                                crate::world::LocationType::AsteroidBase => {
                                    s.message = format!("Entering {} (Asteroid Base) \u{2014} Double gold from mining!", current_name);
                                }
                                crate::world::LocationType::AlienRuins => {
                                    s.message = format!("Entering {} (Alien Ruins) \u{2014} Bonus radicals from puzzles!", current_name);
                                }
                                crate::world::LocationType::TradingPost => {
                                    s.message = format!("Entering {} (Trading Post) \u{2014} 25% shop discount!", current_name);
                                }
                            }
                            s.message_timer = 90;
                            s.generate_quests();
                        }
                        "s" | "S" => {
                            s.game_mode = GameMode::ShipInterior;
                            s.ship_player_x = 11;
                            s.ship_player_y = 16;
                            s.message = "Entering ship...".to_string();
                            s.message_timer = 60;
                        }
                        "Escape" => {
                            // Could open menu or do nothing
                        }
                        _ => {}
                    }
                    s.render();
                    return;
                }
                GameMode::ShipInterior => {
                    event.prevent_default();
                    // Ship upgrade shop input interception
                    if s.show_ship_upgrades {
                        use crate::world::ship::ShipUpgrade;
                        let all = ShipUpgrade::all();
                        match key.as_str() {
                            "ArrowUp" | "w" | "W" => {
                                if s.ship_upgrade_cursor > 0 {
                                    s.ship_upgrade_cursor -= 1;
                                }
                            }
                            "ArrowDown" | "s" | "S" => {
                                if s.ship_upgrade_cursor + 1 < all.len() {
                                    s.ship_upgrade_cursor += 1;
                                }
                            }
                            "Enter" | " " => {
                                let upgrade = all[s.ship_upgrade_cursor];
                                if s.ship.upgrades.contains(&upgrade) {
                                    s.message = format!("{} already installed!", upgrade.name());
                                    s.message_timer = 60;
                                } else if s.player.gold >= upgrade.cost() {
                                    s.player.gold -= upgrade.cost();
                                    s.ship.upgrades.push(upgrade);
                                    match upgrade {
                                        ShipUpgrade::ReinforcedHull => {
                                            s.ship.max_hull += 20;
                                            s.ship.hull += 20;
                                        }
                                        ShipUpgrade::ExtendedFuelTanks => {
                                            s.ship.max_fuel += 30;
                                            s.ship.fuel += 30;
                                        }
                                        ShipUpgrade::AdvancedShields => {
                                            s.ship.max_shields += 10;
                                            s.ship.shields += 10;
                                        }
                                        ShipUpgrade::CargoExpansion => {
                                            s.ship.cargo_capacity += 5;
                                        }
                                        ShipUpgrade::SensorArray => {
                                            s.ship.sensor_range += 2;
                                        }
                                        ShipUpgrade::WeaponBooster => {
                                            s.ship.weapon_power += 2;
                                        }
                                        ShipUpgrade::EngineBooster => {
                                            s.ship.engine_power += 1;
                                        }
                                        // AutoRepairDrone, MedicalBay, QuantumForgeUpgrade
                                        // are passive — just being in upgrades vec is enough
                                        _ => {}
                                    }
                                    s.message = format!("Installed {}! (-{} credits)", upgrade.name(), upgrade.cost());
                                    s.message_timer = 90;
                                } else {
                                    s.message = format!("Not enough credits! Need {}.", upgrade.cost());
                                    s.message_timer = 60;
                                }
                            }
                            "Escape" => {
                                s.show_ship_upgrades = false;
                            }
                            _ => {}
                        }
                        s.render();
                        return;
                    }
                    // Ship help overlay interception
                    if s.show_ship_help {
                        match key.as_str() {
                            "?" | "Escape" => {
                                s.show_ship_help = false;
                            }
                            _ => {}
                        }
                        s.render();
                        return;
                    }
                    match key.as_str() {
                        "ArrowUp" | "w" | "W" => {
                            let ny = s.ship_player_y - 1;
                            if ny >= 0 {
                                let idx = (ny * s.ship_layout.width + s.ship_player_x) as usize;
                                if idx < s.ship_layout.tiles.len() {
                                    match s.ship_layout.tiles[idx] {
                                        ShipTile::Floor | ShipTile::Door | ShipTile::Console(_) 
                                        | ShipTile::CrewStation(_) | ShipTile::Decoration(_) => {
                                            s.ship_player_y = ny;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        "ArrowDown" | "s" | "S" => {
                            let ny = s.ship_player_y + 1;
                            if ny < s.ship_layout.height {
                                let idx = (ny * s.ship_layout.width + s.ship_player_x) as usize;
                                if idx < s.ship_layout.tiles.len() {
                                    match s.ship_layout.tiles[idx] {
                                        ShipTile::Floor | ShipTile::Door | ShipTile::Console(_) 
                                        | ShipTile::CrewStation(_) | ShipTile::Decoration(_) => {
                                            s.ship_player_y = ny;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        "ArrowLeft" | "a" | "A" => {
                            let nx = s.ship_player_x - 1;
                            if nx >= 0 {
                                let idx = (s.ship_player_y * s.ship_layout.width + nx) as usize;
                                if idx < s.ship_layout.tiles.len() {
                                    match s.ship_layout.tiles[idx] {
                                        ShipTile::Floor | ShipTile::Door | ShipTile::Console(_) 
                                        | ShipTile::CrewStation(_) | ShipTile::Decoration(_) => {
                                            s.ship_player_x = nx;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        "ArrowRight" | "d" | "D" => {
                            let nx = s.ship_player_x + 1;
                            if nx < s.ship_layout.width {
                                let idx = (s.ship_player_y * s.ship_layout.width + nx) as usize;
                                if idx < s.ship_layout.tiles.len() {
                                    match s.ship_layout.tiles[idx] {
                                        ShipTile::Floor | ShipTile::Door | ShipTile::Console(_) 
                                        | ShipTile::CrewStation(_) | ShipTile::Decoration(_) => {
                                            s.ship_player_x = nx;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        "e" | "E" => {
                            let px = s.ship_player_x;
                            let py = s.ship_player_y;
                            for (dx, dy) in [(0i32,0i32),(0,1),(0,-1),(1,0),(-1,0)] {
                                let nx = px + dx;
                                let ny = py + dy;
                                if nx >= 0 && ny >= 0 && nx < s.ship_layout.width && ny < s.ship_layout.height {
                                    let idx = (ny * s.ship_layout.width + nx) as usize;
                                    if idx < s.ship_layout.tiles.len() {
                                        match &s.ship_layout.tiles[idx] {
                                            ShipTile::Console(room) => {
                                                match room {
                                                    ShipRoom::Bridge => {
                                                        s.game_mode = GameMode::Starmap;
                                                        s.message = "Accessing navigation...".to_string();
                                                        s.message_timer = 60;
                                                    }
                                                    ShipRoom::Medbay => {
                                                        let heal = 10;
                                                        s.player.hp = (s.player.hp + heal).min(s.player.effective_max_hp());
                                                        s.message = format!("Medbay: Healed {} HP", heal);
                                                        s.message_timer = 60;
                                                    }
                                                    ShipRoom::EngineRoom => {
                                                        let repair = 5;
                                                        s.ship.hull = (s.ship.hull + repair).min(s.ship.max_hull);
                                                        s.message = format!("Engineering: Repaired {} hull", repair);
                                                        s.message_timer = 60;
                                                    }
                                                    ShipRoom::QuantumForge => {
                                                        s.message = "Quantum Forge: Use during exploration to craft spells.".to_string();
                                                        s.message_timer = 90;
                                                    }
                                                    ShipRoom::WeaponsBay => {
                                                        s.ship.weapon_power += 1;
                                                        s.message = format!("Weapons calibrated! Power: {}", s.ship.weapon_power);
                                                        s.message_timer = 60;
                                                    }
                                                    ShipRoom::CargoBay => {
                                                        s.show_ship_upgrades = true;
                                                        s.ship_upgrade_cursor = 0;
                                                        s.message = "Ship Upgrades \u{2014} browse available modules.".to_string();
                                                        s.message_timer = 60;
                                                    }
                                                    ShipRoom::Airlock => {
                                                        // Exit ship to explore current location
                                                        if let Some(ref map) = s.sector_map {
                                                            let sector = &map.sectors[map.current_sector];
                                                            if map.current_system < sector.systems.len() {
                                                                let loc_type = sector.systems[map.current_system].location_type;
                                                                let loc_name = sector.systems[map.current_system].name;
                                                                s.current_location_type = Some(loc_type);
                                                                s.game_mode = GameMode::LocationExploration;
                                                                s.level = DungeonLevel::generate(MAP_W, MAP_H, s.seed, s.floor_num, loc_type);
                                                                let (sx, sy) = s.level.start_pos();
                                                                s.player.move_to(sx, sy);
                                                                s.enemies.clear();
                                                                s.combat = CombatState::Explore;
                                                                s.spawn_enemies();
                                                                let (fpx, fpy) = (s.player.x, s.player.y);
                                                                compute_fov(&mut s.level, fpx, fpy, FOV_RADIUS);
                                                                s.message = format!("Exiting ship to {} \u{2014} Good luck!", loc_name);
                                                                s.message_timer = 90;
                                                                s.generate_quests();
                                                            }
                                                        } else {
                                                            s.message = "No location to explore. Use the Bridge to navigate.".to_string();
                                                            s.message_timer = 60;
                                                        }
                                                    }
                                                    ShipRoom::CrewQuarters => {
                                                        // Rest: restore full HP (once per visit)
                                                        if s.player.hp < s.player.effective_max_hp() {
                                                            s.player.hp = s.player.effective_max_hp();
                                                            s.message = "Crew Quarters: Rested and fully healed!".to_string();
                                                            s.message_timer = 90;
                                                        } else {
                                                            s.message = "Crew Quarters: You feel well-rested already.".to_string();
                                                            s.message_timer = 60;
                                                        }
                                                    }
                                                    _ => {
                                                        s.message = "Nothing to interact with here.".to_string();
                                                        s.message_timer = 60;
                                                    }
                                                }
                                                break;
                                            }
                                            ShipTile::CrewStation(crew_idx) => {
                                                if *crew_idx < s.crew.len() {
                                                    let crew = &s.crew[*crew_idx];
                                                    let dialogue = match crew.role {
                                                        CrewRole::Pilot => "Course is set, Captain. Ready to jump.",
                                                        CrewRole::Engineer => "Engines are holding steady. Hull integrity nominal.",
                                                        CrewRole::ScienceOfficer => "Sensors detecting interesting signals nearby.",
                                                        CrewRole::Medic => "Medical bay is stocked and ready.",
                                                        CrewRole::Quartermaster => "Supplies are accounted for, Captain.",
                                                        CrewRole::SecurityChief => "All secure. No threats detected.",
                                                    };
                                                    s.message = format!("{} ({}): \"{}\"", crew.name, crew.role.name(), dialogue);
                                                    s.message_timer = 120;
                                                }
                                                break;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        "m" | "M" => {
                            s.game_mode = GameMode::Starmap;
                            s.starmap_cursor = 0;
                            s.message = "Opening star map...".to_string();
                            s.message_timer = 60;
                        }
                        "Escape" => {
                            s.game_mode = GameMode::Starmap;
                        }
                        "?" => {
                            s.show_ship_help = !s.show_ship_help;
                        }
                        _ => {}
                    }
                    s.render();
                    return;
                }
                GameMode::Event => {
                    event.prevent_default();
                    if let Some(event_idx) = s.current_event {
                        if let Some(ev) = ALL_EVENTS.get(event_idx) {
                            let num_choices = ev.choices.len();
                            match key.as_str() {
                                "ArrowUp" | "w" | "W" => {
                                    if s.event_choice_cursor > 0 {
                                        s.event_choice_cursor -= 1;
                                    }
                                }
                                "ArrowDown" | "s" | "S" => {
                                    if s.event_choice_cursor + 1 < num_choices {
                                        s.event_choice_cursor += 1;
                                    }
                                }
                                "Enter" | " " => {
                                    let choice_idx = s.event_choice_cursor;
                                    let choice = &ev.choices[choice_idx];
                                    let outcome = choice.outcome.clone();
                                    let eid = ev.id;
                                    let result = apply_event_outcome(&mut *s, &outcome);
                                    record_event_consequence(&mut s.event_memory, eid, choice_idx);
                                    s.message = result;
                                    s.message_timer = 120;
                                    s.current_event = None;
                                    s.event_choice_cursor = 0;
                                    s.game_mode = GameMode::Starmap;
                                }
                                "Escape" => {
                                    s.current_event = None;
                                    s.event_choice_cursor = 0;
                                    s.game_mode = GameMode::Starmap;
                                }
                                _ => {}
                            }
                        }
                    }
                    s.render();
                    return;
                }
                GameMode::SpaceCombat => {
                    event.prevent_default();
                    if let Some(ref mut enemy) = s.enemy_ship.clone() {
                        match s.space_combat_phase {
                            SpaceCombatPhase::Victory => {
                                // Apply Quartermaster bonus (+25% loot)
                                let mut loot = enemy.loot_credits;
                                if s.crew.iter().any(|c| c.role == CrewRole::Quartermaster) {
                                    loot = (loot as f64 * 1.25) as i32;
                                }
                                s.player.gold += loot;
                                // Medic heals 1 crew HP after battle
                                if s.crew.iter().any(|c| c.role == CrewRole::Medic) {
                                    for crew in s.crew.iter_mut() {
                                        crew.hp = (crew.hp + 1).min(crew.max_hp);
                                    }
                                }
                                s.message = format!("Victory! Gained {} credits.", loot);
                                s.message_timer = 120;
                                s.enemy_ship = None;
                                s.space_combat_log.clear();
                                s.game_mode = GameMode::Starmap;
                            }
                            SpaceCombatPhase::Defeat => {
                                s.player.hp = 0;
                                s.enemy_ship = None;
                                s.space_combat_log.clear();
                                s.game_mode = GameMode::Starmap;
                                s.message = "Your ship was destroyed...".to_string();
                                s.message_timer = 120;
                            }
                            SpaceCombatPhase::TargetingSubsystem => {
                                match key.as_str() {
                                    "ArrowUp" | "w" | "W" => {
                                        if s.space_combat_target_cursor > 0 {
                                            s.space_combat_target_cursor -= 1;
                                        }
                                    }
                                    "ArrowDown" | "s" | "S" => {
                                        if s.space_combat_target_cursor < 3 {
                                            s.space_combat_target_cursor += 1;
                                        }
                                    }
                                    "Escape" => {
                                        s.space_combat_phase = SpaceCombatPhase::Choosing;
                                    }
                                    "Enter" | " " => {
                                        let targets = SubsystemTarget::all();
                                        s.space_combat_target = targets[s.space_combat_target_cursor];
                                        let weapon = s.space_combat_weapon;
                                        let target = s.space_combat_target;
                                        let target_name = target.name();
                                        let mut enemy = enemy.clone();

                                        // Crew bonus: SecurityChief (gunner) +2 damage
                                        let gunner_bonus: i32 = if s.crew.iter().any(|c| c.role == CrewRole::SecurityChief) { 2 } else { 0 };
                                        let base_wp = s.ship.weapon_power + gunner_bonus;

                                        match weapon {
                                            ShipWeapon::Laser => {
                                                let dmg = base_wp;
                                                GameState::apply_subsystem_damage(&mut enemy, target, dmg);
                                                s.space_combat_log.push(format!("{} {} => {} for {} dmg", weapon.icon(), weapon.name(), target_name, dmg));
                                                if let Some(ref audio) = s.audio { audio.play_laser_fire(); }
                                            }
                                            ShipWeapon::Missiles => {
                                                let roll = (s.seed.wrapping_mul(1664525).wrapping_add(1013904223)) % 100;
                                                s.seed = s.seed.wrapping_mul(1664525).wrapping_add(1013904223);
                                                if roll < 75 {
                                                    let dmg = base_wp * 2;
                                                    GameState::apply_subsystem_damage(&mut enemy, target, dmg);
                                                    s.space_combat_log.push(format!("{} {} HIT {} for {} dmg!", weapon.icon(), weapon.name(), target_name, dmg));
                                                    if let Some(ref audio) = s.audio { audio.play_missile_launch(); }
                                                } else {
                                                    s.space_combat_log.push(format!("{} {} MISSED {}!", weapon.icon(), weapon.name(), target_name));
                                                    if let Some(ref audio) = s.audio { audio.play_missile_miss(); }
                                                }
                                            }
                                            ShipWeapon::IonCannon => {
                                                let dmg = if target == SubsystemTarget::Shields {
                                                    base_wp * 2
                                                } else if target == SubsystemTarget::Hull {
                                                    (base_wp / 2).max(1)
                                                } else {
                                                    base_wp
                                                };
                                                GameState::apply_subsystem_damage(&mut enemy, target, dmg);
                                                s.space_combat_log.push(format!("{} {} => {} for {} dmg", weapon.icon(), weapon.name(), target_name, dmg));
                                                if let Some(ref audio) = s.audio { audio.play_ion_cannon(); }
                                            }
                                            ShipWeapon::Broadside => {
                                                // Should not reach here — broadside auto-fires
                                            }
                                        }

                                        // Apply subsystem destruction effects
                                        GameState::apply_subsystem_effects(&mut enemy);
                                        if enemy.weapons_sub.is_destroyed() || enemy.shields_sub.is_destroyed() || enemy.engines_sub.is_destroyed() {
                                            if let Some(ref audio) = s.audio { audio.play_subsystem_destroyed(); }
                                        }

                                        // Check victory
                                        if enemy.hull <= 0 {
                                            s.space_combat_phase = SpaceCombatPhase::Victory;
                                            s.space_combat_log.push(format!("{} destroyed!", enemy.name));
                                            s.enemy_ship = Some(enemy);
                                            s.render();
                                            return;
                                        }

                                        // Engineer passive: +2 shields per turn
                                        if s.crew.iter().any(|c| c.role == CrewRole::Engineer) {
                                            s.ship.shields = (s.ship.shields + 2).min(s.ship.max_shields);
                                        }

                                        // Enemy fires back
                                        enemy.turns_taken += 1;
                                        GameState::enemy_fires(&mut enemy, &mut *s, false);

                                        if s.ship.hull <= 0 {
                                            s.space_combat_phase = SpaceCombatPhase::Defeat;
                                            s.space_combat_log.push("Your ship has been destroyed!".to_string());
                                            s.enemy_ship = Some(enemy);
                                            s.render();
                                            return;
                                        }

                                        s.space_combat_phase = SpaceCombatPhase::Choosing;
                                        s.enemy_ship = Some(enemy);
                                    }
                                    _ => {}
                                }
                            }
                            SpaceCombatPhase::Choosing => {
                                let mut enemy = enemy.clone();
                                match key.as_str() {
                                    "ArrowUp" | "w" | "W" => {
                                        // Move up a row: 4-7 -> 0-3
                                        if s.space_combat_cursor >= 4 {
                                            s.space_combat_cursor -= 4;
                                        }
                                    }
                                    "ArrowDown" | "s" | "S" => {
                                        // Move down a row: 0-3 -> 4-7
                                        if s.space_combat_cursor < 4 {
                                            s.space_combat_cursor += 4;
                                        }
                                    }
                                    "ArrowLeft" | "a" | "A" => {
                                        let row_start = (s.space_combat_cursor / 4) * 4;
                                        if s.space_combat_cursor > row_start {
                                            s.space_combat_cursor -= 1;
                                        }
                                    }
                                    "ArrowRight" | "d" | "D" => {
                                        let row_end = (s.space_combat_cursor / 4) * 4 + 3;
                                        if s.space_combat_cursor < row_end {
                                            s.space_combat_cursor += 1;
                                        }
                                    }
                                    "Enter" | " " => {
                                        match s.space_combat_cursor {
                                            0 => {
                                                // Fire Laser -> targeting
                                                s.space_combat_weapon = ShipWeapon::Laser;
                                                s.space_combat_phase = SpaceCombatPhase::TargetingSubsystem;
                                                s.space_combat_target_cursor = 3; // default Hull
                                            }
                                            1 => {
                                                // Fire Missiles -> targeting
                                                s.space_combat_weapon = ShipWeapon::Missiles;
                                                s.space_combat_phase = SpaceCombatPhase::TargetingSubsystem;
                                                s.space_combat_target_cursor = 3;
                                            }
                                            2 => {
                                                // Fire Ion Cannon -> targeting
                                                s.space_combat_weapon = ShipWeapon::IonCannon;
                                                s.space_combat_phase = SpaceCombatPhase::TargetingSubsystem;
                                                s.space_combat_target_cursor = 1; // default Shields for ion
                                            }
                                            3 => {
                                                // Broadside — auto-fires all subsystems
                                                let gunner_bonus: i32 = if s.crew.iter().any(|c| c.role == CrewRole::SecurityChief) { 2 } else { 0 };
                                                let base_wp = s.ship.weapon_power + gunner_bonus;
                                                let dmg = (base_wp / 2).max(1);
                                                for target in SubsystemTarget::all().iter() {
                                                    GameState::apply_subsystem_damage(&mut enemy, *target, dmg);
                                                }
                                                s.space_combat_log.push(format!("== Broadside! {} dmg to all subsystems", dmg));
                                                if let Some(ref audio) = s.audio { audio.play_broadside(); }
                                                GameState::apply_subsystem_effects(&mut enemy);
                                                if enemy.weapons_sub.is_destroyed() || enemy.shields_sub.is_destroyed() || enemy.engines_sub.is_destroyed() {
                                                    if let Some(ref audio) = s.audio { audio.play_subsystem_destroyed(); }
                                                }

                                                if enemy.hull <= 0 {
                                                    s.space_combat_phase = SpaceCombatPhase::Victory;
                                                    s.space_combat_log.push(format!("{} destroyed!", enemy.name));
                                                    s.enemy_ship = Some(enemy);
                                                    s.render();
                                                    return;
                                                }

                                                // Engineer passive
                                                if s.crew.iter().any(|c| c.role == CrewRole::Engineer) {
                                                    s.ship.shields = (s.ship.shields + 2).min(s.ship.max_shields);
                                                }

                                                enemy.turns_taken += 1;
                                                GameState::enemy_fires(&mut enemy, &mut *s, false);

                                                if s.ship.hull <= 0 {
                                                    s.space_combat_phase = SpaceCombatPhase::Defeat;
                                                    s.space_combat_log.push("Your ship has been destroyed!".to_string());
                                                    s.enemy_ship = Some(enemy);
                                                    s.render();
                                                    return;
                                                }
                                                s.space_combat_phase = SpaceCombatPhase::Choosing;
                                            }
                                            4 => {
                                                // Raise Shields
                                                let restore = s.ship.max_shields / 3;
                                                // Engineer passive
                                                let eng_bonus = if s.crew.iter().any(|c| c.role == CrewRole::Engineer) { 2 } else { 0 };
                                                let total_restore = restore + eng_bonus;
                                                s.ship.shields = (s.ship.shields + total_restore).min(s.ship.max_shields);
                                                s.space_combat_log.push(format!("Shields recharged! +{} shields", total_restore));
                                                if let Some(ref audio) = s.audio { audio.play_shield_recharge(); }

                                                enemy.turns_taken += 1;
                                                GameState::enemy_fires(&mut enemy, &mut *s, false);

                                                if s.ship.hull <= 0 {
                                                    s.space_combat_phase = SpaceCombatPhase::Defeat;
                                                    s.space_combat_log.push("Your ship has been destroyed!".to_string());
                                                    s.enemy_ship = Some(enemy);
                                                    s.render();
                                                    return;
                                                }
                                                s.space_combat_phase = SpaceCombatPhase::Choosing;
                                            }
                                            5 => {
                                                // Evasive Maneuvers
                                                let mut evading = true;
                                                let pilot_bonus: i32 = if s.crew.iter().any(|c| c.role == CrewRole::Pilot) { 20 } else { 0 };
                                                if s.ship.engine_power > enemy.engine_power {
                                                    s.space_combat_log.push("Evasive maneuvers! Enemy damage halved.".to_string());
                                                } else if enemy.engines_sub.is_destroyed() {
                                                    s.space_combat_log.push("Enemy engines destroyed — evasion auto-succeeds!".to_string());
                                                } else {
                                                    let roll = (s.seed.wrapping_mul(1664525).wrapping_add(1013904223)) % 100;
                                                    s.seed = s.seed.wrapping_mul(1664525).wrapping_add(1013904223);
                                                    let threshold = 50 + pilot_bonus;
                                                    if (roll as i32) < threshold {
                                                        s.space_combat_log.push("Evasive maneuvers! Enemy damage halved.".to_string());
                                                    } else {
                                                        evading = false;
                                                        s.space_combat_log.push("Evasion failed!".to_string());
                                                    }
                                                }
                                                s.space_combat_evading = evading;
                                                if let Some(ref audio) = s.audio { audio.play_evasion(); }

                                                // Engineer passive
                                                if s.crew.iter().any(|c| c.role == CrewRole::Engineer) {
                                                    s.ship.shields = (s.ship.shields + 2).min(s.ship.max_shields);
                                                }

                                                enemy.turns_taken += 1;
                                                GameState::enemy_fires(&mut enemy, &mut *s, evading);

                                                s.space_combat_evading = false;
                                                if s.ship.hull <= 0 {
                                                    s.space_combat_phase = SpaceCombatPhase::Defeat;
                                                    s.space_combat_log.push("Your ship has been destroyed!".to_string());
                                                    s.enemy_ship = Some(enemy);
                                                    s.render();
                                                    return;
                                                }
                                                s.space_combat_phase = SpaceCombatPhase::Choosing;
                                            }
                                            6 => {
                                                // Board
                                                let skill_total: i32 = s.crew.iter().map(|c| c.skill).sum();
                                                if skill_total > enemy.weapon_power {
                                                    enemy.hull = 0;
                                                    s.space_combat_log.push(format!("Boarding successful! Crew skill {} vs enemy power {}", skill_total, enemy.weapon_power));
                                                    if let Some(ref audio) = s.audio { audio.play_boarding(); }
                                                    enemy.loot_credits = (enemy.loot_credits as f64 * 1.5) as i32;
                                                    s.space_combat_phase = SpaceCombatPhase::Victory;
                                                    s.space_combat_log.push(format!("{} captured!", enemy.name));
                                                    s.enemy_ship = Some(enemy);
                                                    s.render();
                                                    return;
                                                } else {
                                                    let dmg = enemy.weapon_power;
                                                    s.ship.hull -= dmg;
                                                    s.space_combat_log.push(format!("Boarding failed! Took {} hull dmg. (Skill {} vs power {})", dmg, skill_total, enemy.weapon_power));
                                                    if let Some(ref audio) = s.audio { audio.play_boarding(); }
                                                    if s.ship.hull <= 0 {
                                                        s.space_combat_phase = SpaceCombatPhase::Defeat;
                                                        s.space_combat_log.push("Your ship has been destroyed!".to_string());
                                                        s.enemy_ship = Some(enemy);
                                                        s.render();
                                                        return;
                                                    }
                                                    s.space_combat_phase = SpaceCombatPhase::Choosing;
                                                }
                                            }
                                            7 | _ => {
                                                // Flee
                                                let can_flee = enemy.engines_sub.is_destroyed() || s.ship.engine_power >= enemy.engine_power;
                                                if can_flee {
                                                    s.enemy_ship = None;
                                                    s.space_combat_log.clear();
                                                    s.game_mode = GameMode::Starmap;
                                                    s.message = "Escaped from combat!".to_string();
                                                    s.message_timer = 90;
                                                    s.render();
                                                    return;
                                                } else {
                                                    let pilot_bonus: i32 = if s.crew.iter().any(|c| c.role == CrewRole::Pilot) { 15 } else { 0 };
                                                    let roll = (s.seed.wrapping_mul(1664525).wrapping_add(1013904223)) % 100;
                                                    s.seed = s.seed.wrapping_mul(1664525).wrapping_add(1013904223);
                                                    if (roll as i32) < 50 + pilot_bonus {
                                                        s.enemy_ship = None;
                                                        s.space_combat_log.clear();
                                                        s.game_mode = GameMode::Starmap;
                                                        s.message = "Escaped from combat!".to_string();
                                                        s.message_timer = 90;
                                                        s.render();
                                                        return;
                                                    } else {
                                                        let dmg = enemy.weapon_power;
                                                        let shield_absorb = dmg.min(s.ship.shields);
                                                        s.ship.shields -= shield_absorb;
                                                        let hull_dmg = dmg - shield_absorb;
                                                        s.ship.hull -= hull_dmg;
                                                        s.space_combat_log.push(format!("Escape failed! Took {} damage.", dmg));
                                                        if s.ship.hull <= 0 {
                                                            s.space_combat_phase = SpaceCombatPhase::Defeat;
                                                            s.space_combat_log.push("Your ship has been destroyed!".to_string());
                                                            s.enemy_ship = Some(enemy);
                                                            s.render();
                                                            return;
                                                        }
                                                        s.space_combat_phase = SpaceCombatPhase::Choosing;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    "Escape" => {
                                        // Escape key = attempt flee
                                        let can_flee = enemy.engines_sub.is_destroyed() || s.ship.engine_power >= enemy.engine_power;
                                        if can_flee {
                                            s.enemy_ship = None;
                                            s.space_combat_log.clear();
                                            s.game_mode = GameMode::Starmap;
                                            s.message = "Escaped from combat!".to_string();
                                            s.message_timer = 90;
                                            s.render();
                                            return;
                                        } else {
                                            let pilot_bonus: i32 = if s.crew.iter().any(|c| c.role == CrewRole::Pilot) { 15 } else { 0 };
                                            let roll = (s.seed.wrapping_mul(1664525).wrapping_add(1013904223)) % 100;
                                            s.seed = s.seed.wrapping_mul(1664525).wrapping_add(1013904223);
                                            if (roll as i32) < 50 + pilot_bonus {
                                                s.enemy_ship = None;
                                                s.space_combat_log.clear();
                                                s.game_mode = GameMode::Starmap;
                                                s.message = "Escaped from combat!".to_string();
                                                s.message_timer = 90;
                                                s.render();
                                                return;
                                            } else {
                                                let dmg = enemy.weapon_power;
                                                let shield_absorb = dmg.min(s.ship.shields);
                                                s.ship.shields -= shield_absorb;
                                                let hull_dmg = dmg - shield_absorb;
                                                s.ship.hull -= hull_dmg;
                                                s.space_combat_log.push(format!("Escape failed! Took {} damage.", dmg));
                                            }
                                        }
                                    }
                                    _ => {}
                                }

                                // Check victory/defeat after player action
                                if enemy.hull <= 0 {
                                    s.space_combat_phase = SpaceCombatPhase::Victory;
                                    s.space_combat_log.push(format!("{} destroyed!", enemy.name));
                                    s.enemy_ship = Some(enemy);
                                    s.render();
                                    return;
                                }
                                if s.ship.hull <= 0 {
                                    s.space_combat_phase = SpaceCombatPhase::Defeat;
                                    s.space_combat_log.push("Your ship has been destroyed!".to_string());
                                    s.enemy_ship = Some(enemy);
                                    s.render();
                                    return;
                                }

                                s.enemy_ship = Some(enemy);
                            }
                            _ => {
                                // For transitional phases, any key returns to Choosing
                                s.space_combat_phase = SpaceCombatPhase::Choosing;
                            }
                        }
                    } else {
                        // No enemy, return to starmap
                        s.game_mode = GameMode::Starmap;
                    }
                    s.render();
                    return;
                }
                GameMode::LocationExploration | GameMode::GroundCombat => {
                    // Fall through to existing input handling
                }
            }

            // Crew recruitment: press R to recruit pending crew member
            if key == "r" || key == "R" {
                if s.pending_recruit.is_some() && s.crew.len() < 6 {
                    let recruit = s.pending_recruit.take().unwrap();
                    s.message = format!(
                        "🎉 {} ({}) has joined your crew!",
                        recruit.name,
                        recruit.role.name()
                    );
                    s.message_timer = 120;
                    s.crew.push(recruit);
                    s.render();
                    return;
                }
            }

            if key == "?" || key == "/" {
                event.prevent_default();
                s.show_help = !s.show_help;
                s.render();
                return;
            }

            if s.show_settings {
                event.prevent_default();
                match key.as_str() {
                    "Escape" | "o" | "O" => s.close_settings(),
                    "ArrowUp" | "w" | "W" => {
                        s.move_settings_cursor(-1);
                        if let Some(ref audio) = s.audio {
                            audio.play_menu_click();
                        }
                    }
                    "ArrowDown" | "s" | "S" => {
                        s.move_settings_cursor(1);
                        if let Some(ref audio) = s.audio {
                            audio.play_menu_click();
                        }
                    }
                    "ArrowLeft" | "a" | "A" => s.adjust_selected_setting(-1),
                    "ArrowRight" | "d" | "D" | "Enter" => s.adjust_selected_setting(1),
                    _ => {}
                }
                s.render();
                return;
            }

            if s.show_inventory {
                event.prevent_default();
                if s.crafting_mode {
                    // Crafting sub-mode input handling
                    let item_count = s.player.items.len();
                    match key.as_str() {
                        "Escape" | "Backspace" => {
                            if s.crafting_first.is_some() {
                                // Go back to selecting first item
                                s.crafting_first = None;
                            } else {
                                // Exit crafting mode entirely
                                s.crafting_mode = false;
                                s.crafting_cursor = 0;
                            }
                        }
                        "ArrowUp" | "w" | "W" => {
                            if s.crafting_cursor > 0 {
                                s.crafting_cursor -= 1;
                            }
                        }
                        "ArrowDown" | "s" | "S" => {
                            if item_count > 0 && s.crafting_cursor < item_count - 1 {
                                s.crafting_cursor += 1;
                            }
                        }
                        "Enter" => {
                            if item_count > 0 && s.crafting_cursor < item_count {
                                if let Some(first_idx) = s.crafting_first {
                                    let second_idx = s.crafting_cursor;
                                    if first_idx != second_idx {
                                        s.try_craft(first_idx, second_idx);
                                    } else {
                                        s.message = "Select a different item!".to_string();
                                        s.message_timer = 60;
                                    }
                                } else {
                                    // Select first item
                                    s.crafting_first = Some(s.crafting_cursor);
                                }
                            }
                        }
                        _ => {}
                    }
                } else if s.inventory_inspect.is_some() {
                    match key.as_str() {
                        "Escape" | "Backspace" => s.inventory_inspect = None,
                        _ => {}
                    }
                } else {
                    // Unified cursor: 0=weapon, 1=armor, 2=charm, 3+=consumables
                    let total_slots = 3 + s.player.items.len();
                    match key.as_str() {
                        "Escape" | "i" | "I" => s.close_inventory(),
                        "ArrowUp" | "w" | "W" => {
                            if s.inventory_cursor > 0 {
                                s.inventory_cursor -= 1;
                            }
                        }
                        "ArrowDown" | "s" | "S" => {
                            if s.inventory_cursor < total_slots.saturating_sub(1) {
                                s.inventory_cursor += 1;
                            }
                        }
                        "Enter" => {
                            if s.inventory_cursor < total_slots {
                                s.inventory_inspect = Some(s.inventory_cursor);
                            }
                        }
                        "c" | "C" => {
                            if s.player.items.len() >= 2 {
                                s.crafting_mode = true;
                                s.crafting_first = None;
                                s.crafting_cursor = 0;
                            } else {
                                s.message = "Need at least 2 items to craft.".to_string();
                                s.message_timer = 60;
                            }
                        }
                        _ => {}
                    }
                }
                s.render();
                return;
            }

            if s.show_skill_tree {
                event.prevent_default();
                match key.as_str() {
                    "Escape" | "t" | "T" => {
                        s.show_skill_tree = false;
                    }
                    "ArrowUp" => {
                        if s.skill_tree_cursor > 0 {
                            s.skill_tree_cursor -= 1;
                        }
                    }
                    "ArrowDown" => {
                        let max = crate::skill_tree::SKILL_TREE.nodes.len().saturating_sub(1);
                        if s.skill_tree_cursor < max {
                            s.skill_tree_cursor += 1;
                        }
                    }
                    "Enter" | " " => {
                        let idx = s.skill_tree_cursor;
                        if s.player.skill_tree.can_allocate(idx) {
                            s.player.skill_tree.allocate(idx);
                        }
                    }
                    _ => {}
                }
                s.render();
                return;
            }

            if s.show_crucible {
                event.prevent_default();
                match key.as_str() {
                    "Escape" | "u" | "U" => {
                        s.show_crucible = false;
                    }
                    "ArrowUp" => {
                        if s.crucible_cursor > 0 {
                            s.crucible_cursor -= 1;
                        }
                    }
                    "ArrowDown" => {
                        if s.crucible_cursor < 2 {
                            s.crucible_cursor += 1;
                        }
                    }
                    "ArrowLeft" => {
                        let cruc = match s.crucible_cursor {
                            0 => &mut s.player.weapon_crucible,
                            1 => &mut s.player.armor_crucible,
                            _ => &mut s.player.charm_crucible,
                        };
                        if cruc.pending_branch() {
                            cruc.choose_branch(true);
                        }
                    }
                    "ArrowRight" | "Enter" | " " => {
                        let cruc = match s.crucible_cursor {
                            0 => &mut s.player.weapon_crucible,
                            1 => &mut s.player.armor_crucible,
                            _ => &mut s.player.charm_crucible,
                        };
                        if cruc.pending_branch() {
                            cruc.choose_branch(false);
                        }
                    }
                    _ => {}
                }
                s.render();
                return;
            }

            if s.show_spellbook {
                event.prevent_default();
                match key.as_str() {
                    "Escape" | "b" | "B" => s.show_spellbook = false,
                    _ => {}
                }
                s.render();
                return;
            }

            if (key == "o" || key == "O")
                && !s.show_codex
                && matches!(
                    s.combat,
                    CombatState::Explore | CombatState::ClassSelect | CombatState::GameOver
                )
            {
                event.prevent_default();
                s.open_settings();
                s.render();
                return;
            }

            if (key == "i" || key == "I")
                && !s.show_codex
                && !s.show_skill_tree
                && !s.show_crucible
                && matches!(s.combat, CombatState::Explore | CombatState::GameOver)
            {
                event.prevent_default();
                s.open_inventory();
                s.render();
                return;
            }

            if (key == "b" || key == "B")
                && !s.show_codex
                && !s.show_inventory
                && !s.show_skill_tree
                && !s.show_crucible
                && matches!(s.combat, CombatState::Explore | CombatState::GameOver)
            {
                event.prevent_default();
                s.show_spellbook = true;
                s.render();
                return;
            }

            if (key == "t" || key == "T")
                && !s.show_codex
                && !s.show_inventory
                && !s.show_spellbook
                && !s.show_crucible
                && matches!(s.combat, CombatState::Explore | CombatState::GameOver)
            {
                event.prevent_default();
                s.show_skill_tree = !s.show_skill_tree;
                s.skill_tree_cursor = 0;
                s.render();
                return;
            }

            if (key == "u" || key == "U")
                && !s.show_codex
                && !s.show_inventory
                && !s.show_spellbook
                && !s.show_skill_tree
                && matches!(s.combat, CombatState::Explore | CombatState::GameOver)
            {
                event.prevent_default();
                s.show_crucible = !s.show_crucible;
                s.crucible_cursor = 0;
                s.render();
                return;
            }

            // Game over: press R to restart, arrows to page
            if matches!(s.combat, CombatState::GameOver) {
                if key == "r" || key == "R" {
                    s.restart();
                    s.render();
                } else if key == "ArrowRight" {
                    let max_fl = s.run_journal.max_floor();
                    let total_pages = ((max_fl as usize).saturating_sub(1)) / 8 + 1;
                    if s.post_mortem_page < total_pages {
                        s.post_mortem_page += 1;
                    }
                    s.render();
                } else if key == "ArrowLeft" {
                    if s.post_mortem_page > 0 {
                        s.post_mortem_page -= 1;
                    }
                    s.render();
                }
                return;
            }

            // Class selection screen

            // StrokeOrder input
            if matches!(s.combat, CombatState::StrokeOrder { .. }) {
                event.prevent_default();
                let mut completed = None;
                if let CombatState::StrokeOrder {
                    hanzi,
                    ref components,
                    ref correct_order,
                    ref mut cursor,
                    ref mut arranged,
                    pinyin,
                    meaning,
                } = s.combat
                {
                    let remaining: Vec<&str> = components
                        .iter()
                        .copied()
                        .filter(|c| !arranged.contains(c))
                        .collect();
                    match key.as_str() {
                        "ArrowUp" | "w" => {
                            if *cursor > 0 {
                                *cursor -= 1;
                            }
                        }
                        "ArrowDown" | "s" => {
                            if !remaining.is_empty() && *cursor + 1 < remaining.len() {
                                *cursor += 1;
                            }
                        }
                        "Enter" => {
                            if *cursor < remaining.len() {
                                let picked = remaining[*cursor];
                                arranged.push(picked);
                                *cursor = 0;
                                if arranged.len() == correct_order.len() {
                                    let correct = arranged
                                        .iter()
                                        .zip(correct_order.iter())
                                        .all(|(a, b)| a == b);
                                    completed = Some((correct, hanzi, pinyin, meaning));
                                }
                            }
                        }
                        "Backspace" => {
                            arranged.pop();
                            *cursor = 0;
                        }
                        "Escape" => {
                            completed = Some((false, hanzi, pinyin, meaning));
                        }
                        _ => {}
                    }
                }
                if let Some((correct, hanzi, pinyin, meaning)) = completed {
                    s.srs.record(hanzi, correct);
                    s.codex.record(hanzi, pinyin, meaning, correct);
                    let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                    if correct {
                        let gs = &mut *s;
                        gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                        s.message =
                            format!("✓ Correct stroke order for {}! (+1 bonus damage)", hanzi);
                        s.player.tone_bonus_damage += 1;
                    } else {
                        let gs = &mut *s;
                        gs.particles.spawn_damage(sx, sy, &mut gs.rng_state);
                        s.message =
                            format!("✗ Wrong order for {} ({} — {}).", hanzi, pinyin, meaning);
                    }
                    s.message_timer = 80;
                    s.combat = CombatState::Explore;
                }
                s.render();
                return;
            }

            // ToneDefense input
            if matches!(s.combat, CombatState::ToneDefense { .. }) {
                event.prevent_default();
                if let CombatState::ToneDefense {
                    round,
                    hanzi,
                    pinyin,
                    meaning,
                    correct_tone,
                    score,
                    last_result: _,
                } = s.combat.clone()
                {
                    let chosen = match key.as_str() {
                        "1" => Some(1u8),
                        "2" => Some(2u8),
                        "3" => Some(3u8),
                        "4" => Some(4u8),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Retreated from the Tone Wall.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(tone) = chosen {
                        let correct = tone == correct_tone;
                        s.srs.record(hanzi, correct);
                        s.codex.record(hanzi, pinyin, meaning, correct);
                        if !correct {
                            s.player.hp -= 1;
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_damage(sx, sy, &mut gs.rng_state);
                            s.trigger_shake(6);
                        }
                        let new_score = if correct { score + 1 } else { score };
                        if round >= 4 {
                            s.player.defense_bonus = new_score as i32;
                            s.combat = CombatState::Explore;
                            s.message = format!(
                                "Tone Wall complete! {}/5 correct — +{} defense next fight!",
                                new_score, new_score
                            );
                            s.message_timer = 120;
                        } else {
                            let pool = vocab::vocab_for_floor(s.floor_num);
                            let entry = if pool.is_empty() {
                                &vocab::VOCAB[s.rng_next() as usize % vocab::VOCAB.len()]
                            } else {
                                pool[s.rng_next() as usize % pool.len()]
                            };
                            let next_tone = entry
                                .pinyin
                                .chars()
                                .last()
                                .and_then(|c| c.to_digit(10))
                                .unwrap_or(1) as u8;
                            s.combat = CombatState::ToneDefense {
                                round: round + 1,
                                hanzi: entry.hanzi,
                                pinyin: entry.pinyin,
                                meaning: entry.meaning,
                                correct_tone: next_tone,
                                score: new_score,
                                last_result: Some(correct),
                            };
                            s.message = if correct {
                                format!("✓ Blocked! Round {}/5 — {}", round + 2, entry.hanzi)
                            } else {
                                format!(
                                    "✗ Hit! (was tone {}) Round {}/5 — {}",
                                    correct_tone,
                                    round + 2,
                                    entry.hanzi
                                )
                            };
                            s.message_timer = 80;
                        }
                    }
                }
                s.render();
                return;
            }

            // CompoundBuilder input
            if matches!(s.combat, CombatState::CompoundBuilder { .. }) {
                event.prevent_default();
                let mut completed = None;
                if let CombatState::CompoundBuilder {
                    ref parts,
                    correct_compound,
                    pinyin,
                    meaning,
                    ref mut cursor,
                    ref mut arranged,
                } = s.combat
                {
                    let remaining: Vec<&str> = parts
                        .iter()
                        .copied()
                        .filter(|p| !arranged.contains(p))
                        .collect();
                    match key.as_str() {
                        "ArrowUp" | "w" => {
                            if *cursor > 0 {
                                *cursor -= 1;
                            }
                        }
                        "ArrowDown" | "s" => {
                            if !remaining.is_empty() && *cursor + 1 < remaining.len() {
                                *cursor += 1;
                            }
                        }
                        "Enter" => {
                            if *cursor < remaining.len() {
                                let picked = remaining[*cursor];
                                arranged.push(picked);
                                *cursor = 0;
                                if arranged.len() == parts.len() {
                                    let built: String = arranged.iter().copied().collect();
                                    let correct = built == correct_compound;
                                    completed = Some((correct, correct_compound, pinyin, meaning));
                                }
                            }
                        }
                        "Backspace" => {
                            arranged.pop();
                            *cursor = 0;
                        }
                        "Escape" => {
                            completed = Some((false, correct_compound, pinyin, meaning));
                        }
                        _ => {}
                    }
                }
                if let Some((correct, compound, pinyin, meaning)) = completed {
                    s.srs.record(compound, correct);
                    s.codex.record(compound, pinyin, meaning, correct);
                    let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                    if correct {
                        let gs = &mut *s;
                        gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                        s.player.spell_power_temp_bonus = 2;
                        s.message = format!(
                            "✓ Correct! {} ({}) — +2 spell power next cast!",
                            compound, meaning
                        );
                    } else {
                        let gs = &mut *s;
                        gs.particles.spawn_damage(sx, sy, &mut gs.rng_state);
                        s.message = format!(
                            "✗ Wrong! The word was {} ({} — {}).",
                            compound, pinyin, meaning
                        );
                    }
                    s.message_timer = 80;
                    s.combat = CombatState::Explore;
                }
                s.render();
                return;
            }

            // ClassifierMatch input
            if matches!(s.combat, CombatState::ClassifierMatch { .. }) {
                event.prevent_default();
                if let CombatState::ClassifierMatch {
                    round,
                    noun,
                    noun_pinyin,
                    noun_meaning,
                    correct_classifier,
                    options: _,
                    correct_idx,
                    score,
                    last_result: _,
                } = s.combat.clone()
                {
                    let chosen = match key.as_str() {
                        "1" => Some(0usize),
                        "2" => Some(1usize),
                        "3" => Some(2usize),
                        "4" => Some(3usize),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Classifier Shrine.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(pick) = chosen {
                        let correct = pick == correct_idx;
                        s.srs.record(noun, correct);
                        s.codex.record(noun, noun_pinyin, noun_meaning, correct);
                        let new_score = if correct { score + 1 } else { score };
                        if round >= 2 {
                            let gold = new_score as i32 * 5;
                            s.player.gold += gold;
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_chest(sx, sy, &mut gs.rng_state);
                            s.combat = CombatState::Explore;
                            s.message =
                                format!("Classifier done! {}/3 correct — +{}g!", new_score, gold);
                            s.message_timer = 120;
                        } else {
                            let next_idx = s.rng_next() as usize % CLASSIFIER_DATA.len();
                            let (next_noun, next_correct, next_pinyin, next_meaning) =
                                CLASSIFIER_DATA[next_idx];
                            let mut opts: Vec<&'static str> = vec![next_correct];
                            let mut attempts = 0;
                            while opts.len() < 4 && attempts < 50 {
                                let c =
                                    ALL_CLASSIFIERS[s.rng_next() as usize % ALL_CLASSIFIERS.len()];
                                if !opts.contains(&c) {
                                    opts.push(c);
                                }
                                attempts += 1;
                            }
                            while opts.len() < 4 {
                                opts.push("个");
                            }
                            let n = opts.len();
                            for i in (1..n).rev() {
                                let j = s.rng_next() as usize % (i + 1);
                                opts.swap(i, j);
                            }
                            let next_correct_idx =
                                opts.iter().position(|&c| c == next_correct).unwrap_or(0);
                            s.combat = CombatState::ClassifierMatch {
                                round: round + 1,
                                noun: next_noun,
                                noun_pinyin: next_pinyin,
                                noun_meaning: next_meaning,
                                correct_classifier: next_correct,
                                options: [opts[0], opts[1], opts[2], opts[3]],
                                correct_idx: next_correct_idx,
                                score: new_score,
                                last_result: Some(correct),
                            };
                            s.message = if correct {
                                format!(
                                    "✓ Correct! ({}) Round {}/3 — {}",
                                    correct_classifier,
                                    round + 2,
                                    next_noun
                                )
                            } else {
                                format!(
                                    "✗ Wrong! (was {}) Round {}/3 — {}",
                                    correct_classifier,
                                    round + 2,
                                    next_noun
                                )
                            };
                            s.message_timer = 80;
                        }
                    }
                }
                s.render();
                return;
            }

            // InkWell input (press 1-9 to guess component count)
            if matches!(s.combat, CombatState::InkWellChallenge { .. }) {
                event.prevent_default();
                if let CombatState::InkWellChallenge {
                    hanzi,
                    correct_count,
                    pinyin,
                    meaning,
                } = s.combat.clone()
                {
                    let chosen: Option<u8> = match key.as_str() {
                        "1" => Some(1),
                        "2" => Some(2),
                        "3" => Some(3),
                        "4" => Some(4),
                        "5" => Some(5),
                        "6" => Some(6),
                        "7" => Some(7),
                        "8" => Some(8),
                        "9" => Some(9),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Ink Well.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(guess) = chosen {
                        let correct = guess == correct_count;
                        s.srs.record(hanzi, correct);
                        s.codex.record(hanzi, pinyin, meaning, correct);
                        if correct {
                            s.player.hp = (s.player.hp + 1).min(s.player.effective_max_hp());
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                            s.message = format!(
                                "✓ Correct! {} has {} components. +1 HP!",
                                hanzi, correct_count
                            );
                        } else {
                            s.message = format!(
                                "✗ Wrong! {} has {} components ({} — {}).",
                                hanzi, correct_count, pinyin, meaning
                            );
                        }
                        s.combat = CombatState::Explore;
                        s.message_timer = 120;
                    }
                }
                s.render();
                return;
            }

            // AncestorShrine input (press 1-4 to complete chengyu)
            if matches!(s.combat, CombatState::AncestorChallenge { .. }) {
                event.prevent_default();
                if let CombatState::AncestorChallenge {
                    first_half,
                    correct_second,
                    full,
                    pinyin,
                    meaning,
                    options: _,
                    correct_idx,
                } = s.combat.clone()
                {
                    let chosen = match key.as_str() {
                        "1" => Some(0usize),
                        "2" => Some(1usize),
                        "3" => Some(2usize),
                        "4" => Some(3usize),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Ancestor Shrine.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(pick) = chosen {
                        let correct = pick == correct_idx;
                        s.srs.record(first_half, correct);
                        s.codex.record(full, pinyin, meaning, correct);
                        if correct {
                            s.player.gold += 10;
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_chest(sx, sy, &mut gs.rng_state);
                            s.message = format!("✓ {}! ({} — {}) +10 gold!", full, pinyin, meaning);
                        } else {
                            s.message = format!(
                                "✗ Wrong! {} + {} = {} ({}).",
                                first_half, correct_second, full, meaning
                            );
                        }
                        s.combat = CombatState::Explore;
                        s.message_timer = 120;
                    }
                }
                s.render();
                return;
            }

            // TranslationAltar input (press 1-4, 3 rounds)
            if matches!(s.combat, CombatState::TranslationChallenge { .. }) {
                event.prevent_default();
                if let CombatState::TranslationChallenge {
                    round,
                    meaning,
                    correct_hanzi,
                    correct_pinyin,
                    options: _,
                    correct_idx,
                    score,
                } = s.combat.clone()
                {
                    let chosen = match key.as_str() {
                        "1" => Some(0usize),
                        "2" => Some(1usize),
                        "3" => Some(2usize),
                        "4" => Some(3usize),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Translation Altar.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(pick) = chosen {
                        let correct = pick == correct_idx;
                        s.srs.record(correct_hanzi, correct);
                        s.codex
                            .record(correct_hanzi, correct_pinyin, meaning, correct);
                        let new_score = if correct { score + 1 } else { score };
                        if round >= 2 {
                            if new_score >= 2 {
                                s.player.max_hp += 1;
                                s.player.hp += 1;
                                let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                                let gs = &mut *s;
                                gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                                s.message = format!(
                                    "Translation done! {}/3 correct — +1 max HP!",
                                    new_score
                                );
                            } else {
                                s.message = format!(
                                    "Translation done! {}/3 correct — not enough for a reward.",
                                    new_score
                                );
                            }
                            s.combat = CombatState::Explore;
                            s.message_timer = 120;
                        } else {
                            let vocab = vocab::vocab_for_floor(s.floor_num);
                            if vocab.len() >= 4 {
                                let next_idx = s.rng_next() as usize % vocab.len();
                                let next_entry = vocab[next_idx];
                                let mut opts: Vec<&'static str> = vec![next_entry.hanzi];
                                let mut attempts = 0;
                                while opts.len() < 4 && attempts < 50 {
                                    let oi = s.rng_next() as usize % vocab.len();
                                    if !opts.contains(&vocab[oi].hanzi) {
                                        opts.push(vocab[oi].hanzi);
                                    }
                                    attempts += 1;
                                }
                                while opts.len() < 4 {
                                    opts.push("?");
                                }
                                let n = opts.len();
                                for i in (1..n).rev() {
                                    let j = s.rng_next() as usize % (i + 1);
                                    opts.swap(i, j);
                                }
                                let next_correct_idx = opts
                                    .iter()
                                    .position(|&h| h == next_entry.hanzi)
                                    .unwrap_or(0);
                                s.combat = CombatState::TranslationChallenge {
                                    round: round + 1,
                                    meaning: next_entry.meaning,
                                    correct_hanzi: next_entry.hanzi,
                                    correct_pinyin: next_entry.pinyin,
                                    options: [opts[0], opts[1], opts[2], opts[3]],
                                    correct_idx: next_correct_idx,
                                    score: new_score,
                                };
                                let result_str = if correct {
                                    "✓ Correct!"
                                } else {
                                    "✗ Wrong!"
                                };
                                s.message = format!(
                                    "{} Round {}/3 — Which means \"{}\"?",
                                    result_str,
                                    round + 2,
                                    next_entry.meaning
                                );
                                s.message_timer = 80;
                            } else {
                                s.combat = CombatState::Explore;
                                s.message = "Not enough vocabulary.".to_string();
                                s.message_timer = 60;
                            }
                        }
                    }
                }
                s.render();
                return;
            }

            // RadicalGarden input (press 1-4)
            if matches!(s.combat, CombatState::RadicalGardenChallenge { .. }) {
                event.prevent_default();
                if let CombatState::RadicalGardenChallenge {
                    hanzi,
                    pinyin,
                    meaning,
                    correct_radical,
                    options: _,
                    correct_idx,
                } = s.combat.clone()
                {
                    let chosen = match key.as_str() {
                        "1" => Some(0usize),
                        "2" => Some(1usize),
                        "3" => Some(2usize),
                        "4" => Some(3usize),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Radical Garden.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(pick) = chosen {
                        let correct = pick == correct_idx;
                        s.srs.record(hanzi, correct);
                        s.codex.record(hanzi, pinyin, meaning, correct);
                        if correct {
                            let rads = radical::radicals_for_floor(s.floor_num.max(1));
                            if !rads.is_empty() {
                                let ri = s.rng_next() as usize % rads.len();
                                let rad = rads[ri];
                                if !s.player.radicals.contains(&rad.ch) {
                                    s.player.radicals.push(rad.ch);
                                }
                                let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                                let gs = &mut *s;
                                gs.particles.spawn_chest(sx, sy, &mut gs.rng_state);
                                s.message = format!(
                                    "✓ Radical of {} is {}! Free radical: {}",
                                    hanzi, correct_radical, rad.ch
                                );
                            } else {
                                s.message =
                                    format!("✓ Radical of {} is {}!", hanzi, correct_radical);
                            }
                        } else {
                            s.message = format!(
                                "✗ Wrong! Radical of {} is {} ({} — {}).",
                                hanzi, correct_radical, pinyin, meaning
                            );
                        }
                        s.combat = CombatState::Explore;
                        s.message_timer = 120;
                    }
                }
                s.render();
                return;
            }

            // MirrorPool input (type pinyin, Enter to submit)
            if matches!(s.combat, CombatState::MirrorPoolChallenge { .. }) {
                event.prevent_default();
                if let CombatState::MirrorPoolChallenge {
                    hanzi,
                    correct_pinyin,
                    meaning,
                    input,
                } = s.combat.clone()
                {
                    let mut current_input = input;
                    match key.as_str() {
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Mirror Pool.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        "Backspace" => {
                            current_input.pop();
                        }
                        "Enter" => {
                            let correct = current_input.trim() == correct_pinyin;
                            s.srs.record(hanzi, correct);
                            s.codex.record(hanzi, correct_pinyin, meaning, correct);
                            if correct {
                                s.player.spell_power_bonus += 1;
                                let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                                let gs = &mut *s;
                                gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                                s.message = format!(
                                    "✓ Correct! {} = {}. +1 spell power!",
                                    hanzi, correct_pinyin
                                );
                            } else {
                                s.message = format!(
                                    "✗ Wrong! {} = {} ({}).",
                                    hanzi, correct_pinyin, meaning
                                );
                            }
                            s.combat = CombatState::Explore;
                            s.message_timer = 120;
                            s.render();
                            return;
                        }
                        other => {
                            if other.len() == 1 {
                                let ch = other.chars().next().unwrap();
                                if ch.is_ascii_alphanumeric() {
                                    current_input.push(ch);
                                }
                            }
                        }
                    }
                    s.combat = CombatState::MirrorPoolChallenge {
                        hanzi,
                        correct_pinyin,
                        meaning,
                        input: current_input,
                    };
                }
                s.render();
                return;
            }

            // StoneTutor input (Space to advance from teach to quiz, 1-4 for tone quiz)
            if matches!(s.combat, CombatState::StoneTutorChallenge { .. }) {
                event.prevent_default();
                if let CombatState::StoneTutorChallenge {
                    round,
                    hanzi,
                    pinyin,
                    meaning,
                    correct_tone,
                    phase,
                    score,
                } = s.combat.clone()
                {
                    if phase == 0 {
                        if key.as_str() == " " || key.as_str() == "Enter" {
                            s.combat = CombatState::StoneTutorChallenge {
                                round,
                                hanzi,
                                pinyin,
                                meaning,
                                correct_tone,
                                phase: 1,
                                score,
                            };
                            s.message = format!("石 Quiz! What tone is {}? Press 1-4.", hanzi);
                            s.message_timer = 120;
                        } else if key.as_str() == "Escape" {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Stone Tutor.".to_string();
                            s.message_timer = 40;
                        }
                    } else {
                        let chosen: Option<u8> = match key.as_str() {
                            "1" => Some(1),
                            "2" => Some(2),
                            "3" => Some(3),
                            "4" => Some(4),
                            "Escape" => {
                                s.combat = CombatState::Explore;
                                s.message = "Left the Stone Tutor.".to_string();
                                s.message_timer = 40;
                                s.render();
                                return;
                            }
                            _ => None,
                        };
                        if let Some(guess) = chosen {
                            let correct = guess == correct_tone;
                            s.srs.record(hanzi, correct);
                            s.codex.record(hanzi, pinyin, meaning, correct);
                            if correct {
                                s.srs.record(hanzi, true);
                                s.codex.record(hanzi, pinyin, meaning, true);
                            }
                            let new_score = if correct { score + 1 } else { score };
                            if round >= 2 {
                                let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                                let gs = &mut *s;
                                gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                                s.combat = CombatState::Explore;
                                s.message = format!(
                                    "Stone Tutor done! {}/3 correct. SRS boosted!",
                                    new_score
                                );
                                s.message_timer = 120;
                            } else {
                                let vocab = vocab::vocab_for_floor(s.floor_num);
                                if !vocab.is_empty() {
                                    let next_idx = s.rng_next() as usize % vocab.len();
                                    let next = vocab[next_idx];
                                    let next_tone = next
                                        .pinyin
                                        .chars()
                                        .last()
                                        .and_then(|c| c.to_digit(10))
                                        .unwrap_or(1)
                                        as u8;
                                    let result_str = if correct {
                                        format!("✓ Correct! Tone {}.", correct_tone)
                                    } else {
                                        format!("✗ Wrong! Was tone {}.", correct_tone)
                                    };
                                    s.combat = CombatState::StoneTutorChallenge {
                                        round: round + 1,
                                        hanzi: next.hanzi,
                                        pinyin: next.pinyin,
                                        meaning: next.meaning,
                                        correct_tone: next_tone,
                                        phase: 0,
                                        score: new_score,
                                    };
                                    s.message = format!(
                                        "{} Study: {} — {} ({}). Press Space.",
                                        result_str, next.hanzi, next.pinyin, next.meaning
                                    );
                                    s.message_timer = 120;
                                } else {
                                    s.combat = CombatState::Explore;
                                    s.message = "No more vocabulary.".to_string();
                                    s.message_timer = 60;
                                }
                            }
                        }
                    }
                }
                s.render();
                return;
            }

            // CodexChallenge input (1-4 pick meaning, Escape to leave)
            if matches!(s.combat, CombatState::CodexChallenge { .. }) {
                event.prevent_default();
                if let CombatState::CodexChallenge {
                    round,
                    hanzi,
                    pinyin,
                    meaning,
                    options: _,
                    correct_idx,
                    score,
                } = s.combat.clone()
                {
                    let chosen: Option<usize> = match key.as_str() {
                        "1" => Some(0),
                        "2" => Some(1),
                        "3" => Some(2),
                        "4" => Some(3),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Codex Shrine.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(pick) = chosen {
                        let correct = pick == correct_idx;
                        s.srs.record(hanzi, correct);
                        s.codex.record(hanzi, pinyin, meaning, correct);
                        let new_score = if correct { score + 1 } else { score };
                        if round >= 2 {
                            let gold_earned = new_score as i32 * 5;
                            s.player.gold += gold_earned;
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_chest(sx, sy, &mut gs.rng_state);
                            s.combat = CombatState::Explore;
                            s.message = format!(
                                "Codex Shrine done! {}/3 correct. +{} gold!",
                                new_score, gold_earned
                            );
                            s.message_timer = 120;
                        } else {
                            let codex_entries = s.codex.sorted_entries();
                            let vocab = vocab::vocab_for_floor(s.floor_num);
                            let use_codex = codex_entries.len() >= 4;
                            let pool: Vec<(&'static str, &'static str, &'static str)> = if use_codex
                            {
                                codex_entries
                                    .iter()
                                    .map(|e| (e.hanzi, e.pinyin, e.meaning))
                                    .collect()
                            } else {
                                vocab
                                    .iter()
                                    .map(|e| (e.hanzi, e.pinyin, e.meaning))
                                    .collect()
                            };
                            if pool.len() >= 4 {
                                let next_idx = s.rng_next() as usize % pool.len();
                                let (nh, np, nm) = pool[next_idx];
                                let mut dist: Vec<&'static str> = pool
                                    .iter()
                                    .filter(|(h, _, _)| *h != nh)
                                    .map(|(_, _, m)| *m)
                                    .collect();
                                for i in (1..dist.len()).rev() {
                                    let j = s.rng_next() as usize % (i + 1);
                                    dist.swap(i, j);
                                }
                                let ci = s.rng_next() as usize % 4;
                                let mut opts = [dist[0], dist[1], dist[2], nm];
                                opts[3] = opts[ci];
                                opts[ci] = nm;
                                let result_str = if correct {
                                    format!("✓ Correct! {} = {}", hanzi, meaning)
                                } else {
                                    format!("✗ Wrong! {} = {}", hanzi, meaning)
                                };
                                s.combat = CombatState::CodexChallenge {
                                    round: round + 1,
                                    hanzi: nh,
                                    pinyin: np,
                                    meaning: nm,
                                    options: opts,
                                    correct_idx: ci,
                                    score: new_score,
                                };
                                s.message =
                                    format!("{} What does {} mean? Pick 1-4.", result_str, nh);
                                s.message_timer = 120;
                            } else {
                                s.combat = CombatState::Explore;
                                s.message = "No more vocabulary.".to_string();
                                s.message_timer = 60;
                            }
                        }
                    }
                }
                s.render();
                return;
            }

            // Journal input (PgUp/PgDn/Up/Down to scroll, Escape to close)
            if let CombatState::Journal { page } = s.combat {
                event.prevent_default();
                let total = s.codex.sorted_entries().len();
                let per_page = 12;
                let max_page = if total == 0 {
                    0
                } else {
                    (total - 1) / per_page
                };
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message = "Closed journal.".to_string();
                        s.message_timer = 40;
                    }
                    "ArrowDown" | "PageDown" | "s" | "S" => {
                        if page < max_page {
                            s.combat = CombatState::Journal { page: page + 1 };
                        }
                    }
                    "ArrowUp" | "PageUp" | "w" | "W" => {
                        if page > 0 {
                            s.combat = CombatState::Journal { page: page - 1 };
                        }
                    }
                    _ => {}
                }
                s.render();
                return;
            }

            // WordBridgeChallenge input (1-4 pick hanzi, Escape to leave)
            if matches!(s.combat, CombatState::WordBridgeChallenge { .. }) {
                event.prevent_default();
                if let CombatState::WordBridgeChallenge {
                    meaning,
                    correct_hanzi,
                    correct_pinyin,
                    options: _,
                    correct_idx,
                    bridge_x,
                    bridge_y,
                } = s.combat.clone()
                {
                    let chosen: Option<usize> = match key.as_str() {
                        "1" => Some(0),
                        "2" => Some(1),
                        "3" => Some(2),
                        "4" => Some(3),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the Word Bridge.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(pick) = chosen {
                        let correct = pick == correct_idx;
                        s.srs.record(correct_hanzi, correct);
                        s.codex
                            .record(correct_hanzi, correct_pinyin, meaning, correct);
                        if correct {
                            let bidx = s.level.idx(bridge_x, bridge_y);
                            s.level.tiles[bidx] = Tile::Catwalk;
                            let widx = s.level.idx(s.player.x, s.player.y);
                            if s.level.tiles[widx] == Tile::DataBridge {
                                s.level.tiles[widx] = Tile::MetalFloor;
                            }
                            let (sx, sy) = s.tile_to_screen(bridge_x, bridge_y);
                            let gs = &mut *s;
                            gs.particles.spawn_bridge(sx, sy, &mut gs.rng_state);
                            s.message = format!(
                                "✓ Correct! {} ({}). A bridge forms over the water!",
                                correct_hanzi, meaning
                            );
                        } else {
                            s.message = format!(
                                "✗ Wrong! The answer was {} ({}). Try again later.",
                                correct_hanzi, meaning
                            );
                        }
                        s.combat = CombatState::Explore;
                        s.message_timer = 120;
                    }
                }
                s.render();
                return;
            }

            // LockedDoorChallenge input (1-4 pick meaning, Escape to leave)
            if matches!(s.combat, CombatState::LockedDoorChallenge { .. }) {
                event.prevent_default();
                if let CombatState::LockedDoorChallenge {
                    hanzi,
                    pinyin,
                    correct_meaning,
                    options: _,
                    correct_idx,
                    door_x,
                    door_y,
                } = s.combat.clone()
                {
                    let chosen: Option<usize> = match key.as_str() {
                        "1" => Some(0),
                        "2" => Some(1),
                        "3" => Some(2),
                        "4" => Some(3),
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Stepped away from the locked door.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(pick) = chosen {
                        let correct = pick == correct_idx;
                        s.srs.record(hanzi, correct);
                        s.codex.record(hanzi, pinyin, correct_meaning, correct);
                        if correct {
                            let didx = s.level.idx(door_x, door_y);
                            s.level.tiles[didx] = Tile::MetalFloor;
                            let (sx, sy) = s.tile_to_screen(door_x, door_y);
                            let gs = &mut *s;
                            gs.particles.spawn_dig(sx, sy, &mut gs.rng_state);
                            if let Some(ref audio) = s.audio { audio.sfx_correct(); }
                            s.message = format!(
                                "✓ Correct! {} = {}. The door unlocks!",
                                hanzi, correct_meaning
                            );
                        } else {
                            s.player.hp = (s.player.hp - 1).max(0);
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_damage(sx, sy, &mut gs.rng_state);
                            if let Some(ref audio) = s.audio { audio.sfx_wrong(); }
                            s.message = format!(
                                "✗ Wrong! {} = {}. The door shocks you! (-1 HP)",
                                hanzi, correct_meaning
                            );
                            if s.player.hp <= 0 && !s.try_phoenix_revive() {
                                let fl = s.floor_num;
                                s.run_journal
                                    .log(RunEvent::DiedTo("Locked door trap".to_string(), fl));
                                s.post_mortem_page = 0;
                                s.combat = CombatState::GameOver;
                                s.message = s.run_summary();
                                s.message_timer = 255;
                                s.render();
                                return;
                            }
                        }
                        s.combat = CombatState::Explore;
                        s.message_timer = 120;
                    }
                }
                s.render();
                return;
            }

            // CursedFloorChallenge input (1-4 pick tone)
            if matches!(s.combat, CombatState::CursedFloorChallenge { .. }) {
                event.prevent_default();
                if let CombatState::CursedFloorChallenge {
                    hanzi,
                    pinyin,
                    meaning,
                    correct_tone,
                } = s.combat.clone()
                {
                    let chosen: Option<u8> = match key.as_str() {
                        "1" => Some(1),
                        "2" => Some(2),
                        "3" => Some(3),
                        "4" => Some(4),
                        "Escape" => {
                            s.player.gold = (s.player.gold - 2).max(0);
                            s.combat = CombatState::Explore;
                            s.message = "You flee the curse! (-2 gold)".to_string();
                            s.message_timer = 60;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(guess) = chosen {
                        let correct = guess == correct_tone;
                        s.srs.record(hanzi, correct);
                        s.codex.record(hanzi, pinyin, meaning, correct);
                        if correct {
                            s.player.gold += 1;
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_chest(sx, sy, &mut gs.rng_state);
                            if let Some(ref audio) = s.audio { audio.sfx_correct(); }
                            s.message = format!(
                                "✓ Curse averted! {} is tone {}. (+1 gold)",
                                hanzi, correct_tone
                            );
                        } else {
                            s.player.gold = (s.player.gold - 2).max(0);
                            let (sx, sy) = s.tile_to_screen(s.player.x, s.player.y);
                            let gs = &mut *s;
                            gs.particles.spawn_drain(sx, sy, &mut gs.rng_state);
                            if let Some(ref audio) = s.audio { audio.sfx_wrong(); }
                            s.message = format!(
                                "✗ Cursed! {} is tone {}, not {}. (-2 gold)",
                                hanzi, correct_tone, guess
                            );
                        }
                        s.combat = CombatState::Explore;
                        s.message_timer = 120;
                    }
                }
                s.render();
                return;
            }

            // Tone Battle input
            if matches!(s.combat, CombatState::ToneBattle { .. }) {
                event.prevent_default();
                if let CombatState::ToneBattle {
                    round,
                    hanzi: _,
                    correct_tone,
                    score,
                    last_result: _,
                } = s.combat.clone()
                {
                    let chosen = match key.as_str() {
                        "1" => Some(1u8),
                        "2" => Some(2u8),
                        "3" => Some(3u8),
                        "4" => Some(4u8),
                        "r" | "R" => {
                            // Replay tone
                            if let Some(ref audio) = s.audio {
                                audio.play_chinese_tone(correct_tone);
                            }
                            None
                        }
                        "Escape" => {
                            s.combat = CombatState::Explore;
                            s.message = "Left the shrine.".to_string();
                            s.message_timer = 40;
                            s.render();
                            return;
                        }
                        _ => None,
                    };
                    if let Some(tone) = chosen {
                        let correct = tone == correct_tone;
                        let new_score = if correct { score + 1 } else { score };
                        if round >= 4 {
                            // End of tone battle
                            let bonus_dmg = new_score as i32;
                            s.player.tone_bonus_damage = bonus_dmg;
                            s.combat = CombatState::Explore;
                            s.message = format!(
                                "Shrine complete! {}/5 correct — +{} bonus damage next fight!",
                                new_score, bonus_dmg
                            );
                            s.message_timer = 120;
                        } else {
                            // Next round
                            let (next_hanzi, next_tone) = s.pick_tone_battle_char();
                            if let Some(ref audio) = s.audio {
                                audio.play_chinese_tone(next_tone);
                            }
                            s.combat = CombatState::ToneBattle {
                                round: round + 1,
                                hanzi: next_hanzi,
                                correct_tone: next_tone,
                                score: new_score,
                                last_result: Some(correct),
                            };
                            s.message = if correct {
                                format!("✓ Correct! Round {}/5 — {}", round + 2, next_hanzi)
                            } else {
                                format!(
                                    "✗ Wrong (was tone {})! Round {}/5 — {}",
                                    correct_tone,
                                    round + 2,
                                    next_hanzi
                                )
                            };
                            s.message_timer = 80;
                        }
                    }
                }
                s.render();
                return;
            }

            // Sentence Challenge input
            if matches!(s.combat, CombatState::SentenceChallenge { .. }) {
                event.prevent_default();
                let mut completed = None;
                let mut escaped_mode = None;
                if let CombatState::SentenceChallenge {
                    ref tiles,
                    ref words,
                    ref mut cursor,
                    ref mut arranged,
                    meaning,
                    ref mode,
                } = s.combat
                {
                    let remaining: Vec<usize> = tiles
                        .iter()
                        .copied()
                        .filter(|t| !arranged.contains(t))
                        .collect();
                    match key.as_str() {
                        "ArrowLeft" | "a" => {
                            if *cursor > 0 {
                                *cursor -= 1;
                            }
                        }
                        "ArrowRight" | "d" => {
                            if *cursor + 1 < remaining.len() {
                                *cursor += 1;
                            }
                        }
                        "Enter" => {
                            if *cursor < remaining.len() {
                                arranged.push(remaining[*cursor]);
                                *cursor = 0;
                                // Check if complete
                                if arranged.len() == words.len() {
                                    let correct = arranged.iter().enumerate().all(|(i, &a)| a == i);
                                    completed = Some((
                                        correct,
                                        mode.clone(),
                                        meaning.to_string(),
                                        words.join(" "),
                                    ));
                                }
                            }
                        }
                        "Backspace" => {
                            arranged.pop();
                            *cursor = 0;
                        }
                        "Escape" => {
                            escaped_mode = Some(mode.clone());
                        }
                        _ => {}
                    }
                }
                if let Some((correct, mode, meaning, correct_text)) = completed {
                    match mode {
                        SentenceChallengeMode::BonusGold { reward } => {
                            if correct {
                                s.player.gold += reward;
                                s.message =
                                    format!("✓ Correct! \"{}\" — +{}g bonus!", meaning, reward);
                            } else {
                                s.message = format!("✗ Wrong order! Correct: {}", correct_text);
                            }
                            s.combat = CombatState::Explore;
                            s.message_timer = 120;
                        }
                        SentenceChallengeMode::ScholarTrial {
                            boss_idx,
                            success_damage,
                            failure_heal,
                        } => {
                            if boss_idx < s.enemies.len() && s.enemies[boss_idx].is_alive() {
                                if correct {
                                    let applied =
                                        success_damage.min((s.enemies[boss_idx].hp - 1).max(1));
                                    s.enemies[boss_idx].hp -= applied;
                                    s.enemies[boss_idx].stunned = true;
                                    s.message = format!(
                                        "✓ Correct! \"{}\" — The boss loses {} HP and is stunned!",
                                        meaning, applied
                                    );
                                } else {
                                    let before = s.enemies[boss_idx].hp;
                                    s.enemies[boss_idx].hp =
                                        (before + failure_heal).min(s.enemies[boss_idx].max_hp);
                                    let healed = s.enemies[boss_idx].hp - before;
                                    s.message = format!(
                                        "✗ Wrong order! Correct: {} — The boss regains {} HP.",
                                        correct_text, healed
                                    );
                                }
                                if let Some(mut battle) = s.saved_battle.take() {
                                    for unit in &mut battle.units {
                                        if let combat::UnitKind::Enemy(eidx) = unit.kind {
                                            if eidx == boss_idx {
                                                unit.hp = s.enemies[boss_idx].hp;
                                                unit.max_hp = s.enemies[boss_idx].max_hp;
                                                unit.stunned = s.enemies[boss_idx].stunned;
                                                break;
                                            }
                                        }
                                    }
                                    s.combat = CombatState::TacticalBattle(battle);
                                } else {
                                    s.combat = CombatState::Fighting {
                                        enemy_idx: boss_idx,
                                        timer_ms: 0.0,
                                    };
                                }
                                s.typing.clear();
                            } else {
                                s.combat = CombatState::Explore;
                                s.message = "The sentence duel fades.".to_string();
                            }
                            s.message_timer = 120;
                        }
                        SentenceChallengeMode::GatekeeperSeal {
                            boss_idx,
                            success_damage,
                            failure_damage_to_player,
                        } => {
                            if boss_idx < s.enemies.len() && s.enemies[boss_idx].is_alive() {
                                if correct {
                                    let applied =
                                        success_damage.min((s.enemies[boss_idx].hp - 1).max(1));
                                    s.enemies[boss_idx].hp -= applied;
                                    s.enemies[boss_idx].stunned = true;
                                    s.message = format!(
                                        "✓ Seal shattered! \"{}\" — The Pirate Captain loses {} HP and is stunned!",
                                        meaning, applied
                                    );
                                } else {
                                    s.player.hp = (s.player.hp - failure_damage_to_player).max(0);
                                    s.message = format!(
                                        "✗ The seal backfires! Correct: {} — You take {} damage!",
                                        correct_text, failure_damage_to_player
                                    );
                                    if s.player.hp <= 0 && !s.try_phoenix_revive() {
                                        let fl = s.floor_num;
                                        s.run_journal.log(crate::game::RunEvent::DiedTo(
                                            "Pirate Captain's Seal".to_string(),
                                            fl,
                                        ));
                                        s.combat = CombatState::GameOver;
                                        s.message_timer = 200;
                                        s.render();
                                        return;
                                    }
                                }
                                if let Some(mut battle) = s.saved_battle.take() {
                                    for unit in &mut battle.units {
                                        if let combat::UnitKind::Enemy(eidx) = unit.kind {
                                            if eidx == boss_idx {
                                                unit.hp = s.enemies[boss_idx].hp;
                                                unit.max_hp = s.enemies[boss_idx].max_hp;
                                                unit.stunned = s.enemies[boss_idx].stunned;
                                                break;
                                            }
                                        }
                                    }
                                    s.combat = CombatState::TacticalBattle(battle);
                                } else {
                                    s.combat = CombatState::Fighting {
                                        enemy_idx: boss_idx,
                                        timer_ms: 0.0,
                                    };
                                }
                                s.typing.clear();
                            } else {
                                s.combat = CombatState::Explore;
                                s.message = "The seal fades.".to_string();
                            }
                            s.message_timer = 120;
                        }
                    }
                } else if let Some(mode) = escaped_mode {
                    match mode {
                        SentenceChallengeMode::BonusGold { .. } => {
                            s.combat = CombatState::Explore;
                            s.message = "Skipped sentence challenge.".to_string();
                            s.message_timer = 40;
                        }
                        SentenceChallengeMode::ScholarTrial {
                            boss_idx,
                            failure_heal,
                            ..
                        } => {
                            if boss_idx < s.enemies.len() && s.enemies[boss_idx].is_alive() {
                                let before = s.enemies[boss_idx].hp;
                                s.enemies[boss_idx].hp =
                                    (before + failure_heal).min(s.enemies[boss_idx].max_hp);
                                let healed = s.enemies[boss_idx].hp - before;
                                if let Some(mut battle) = s.saved_battle.take() {
                                    for unit in &mut battle.units {
                                        if let combat::UnitKind::Enemy(eidx) = unit.kind {
                                            if eidx == boss_idx {
                                                unit.hp = s.enemies[boss_idx].hp;
                                                unit.max_hp = s.enemies[boss_idx].max_hp;
                                                unit.stunned = s.enemies[boss_idx].stunned;
                                                break;
                                            }
                                        }
                                    }
                                    s.combat = CombatState::TacticalBattle(battle);
                                } else {
                                    s.combat = CombatState::Fighting {
                                        enemy_idx: boss_idx,
                                        timer_ms: 0.0,
                                    };
                                }
                                s.typing.clear();
                                s.message = format!(
                                    "You abandon the syntax duel. The boss regains {} HP!",
                                    healed
                                );
                            } else {
                                s.combat = CombatState::Explore;
                                s.message = "The sentence duel fades.".to_string();
                            }
                            s.message_timer = 80;
                        }
                        SentenceChallengeMode::GatekeeperSeal {
                            boss_idx,
                            failure_damage_to_player,
                            ..
                        } => {
                            if boss_idx < s.enemies.len() && s.enemies[boss_idx].is_alive() {
                                s.player.hp = (s.player.hp - failure_damage_to_player).max(0);
                                if let Some(mut battle) = s.saved_battle.take() {
                                    for unit in &mut battle.units {
                                        if let combat::UnitKind::Enemy(eidx) = unit.kind {
                                            if eidx == boss_idx {
                                                unit.hp = s.enemies[boss_idx].hp;
                                                unit.max_hp = s.enemies[boss_idx].max_hp;
                                                unit.stunned = s.enemies[boss_idx].stunned;
                                                break;
                                            }
                                        }
                                    }
                                    s.combat = CombatState::TacticalBattle(battle);
                                } else {
                                    s.combat = CombatState::Fighting {
                                        enemy_idx: boss_idx,
                                        timer_ms: 0.0,
                                    };
                                }
                                s.typing.clear();
                                s.message = format!(
                                    "You abandon the seal! The backfire deals {} damage!",
                                    failure_damage_to_player
                                );
                                if s.player.hp <= 0 && !s.try_phoenix_revive() {
                                    let fl = s.floor_num;
                                    s.run_journal
                                        .log(RunEvent::DiedTo("Pirate Captain's Seal".to_string(), fl));
                                    s.post_mortem_page = 0;
                                    s.combat = CombatState::GameOver;
                                }
                            } else {
                                s.combat = CombatState::Explore;
                                s.message = "The seal fades.".to_string();
                            }
                            s.message_timer = 80;
                        }
                    }
                }
                s.render();
                return;
            }

            if matches!(s.combat, CombatState::ClassSelect) {
                event.prevent_default();
                // Daily challenge
                if key == "d" || key == "D" {
                    // Seed from date: year * 10000 + month * 100 + day
                    let date_seed = js_sys::Date::new_0();
                    let daily_seed = (date_seed.get_full_year() as u64) * 10000
                        + (date_seed.get_month() as u64 + 1) * 100
                        + date_seed.get_date() as u64;
                    s.seed = daily_seed;
                    s.rng_state = daily_seed;
                    s.daily_mode = true;
                    s.level = DungeonLevel::generate(MAP_W, MAP_H, daily_seed, 1, s.current_location_type.unwrap_or(crate::world::LocationType::SpaceStation));
                    let (sx, sy) = s.level.start_pos();
                    s.player = s.make_player(sx, sy, PlayerClass::Envoy);
                    s.reset_item_lore();
                    s.combat = CombatState::Explore;
                    s.message =
                        "🏆 Daily Challenge! Fixed seed — compete for high score!".to_string();
                    s.message_timer = 150;
                    s.spawn_enemies();
                    let (px, py) = (s.player.x, s.player.y);
                    compute_fov(&mut s.level, px, py, FOV_RADIUS);
                    s.render();
                    return;
                }
                let total_classes = PlayerClass::all().len();
                if key == "ArrowUp" || key == "w" || key == "W" {
                    if s.class_cursor > 0 {
                        s.class_cursor -= 1;
                    } else {
                        s.class_cursor = total_classes - 1;
                    }
                    if let Some(ref audio) = s.audio {
                        audio.play_menu_click();
                    }
                    s.render();
                    return;
                }
                if key == "ArrowDown" || key == "s" || key == "S" {
                    s.class_cursor = (s.class_cursor + 1) % total_classes;
                    if let Some(ref audio) = s.audio {
                        audio.play_menu_click();
                    }
                    s.render();
                    return;
                }

                let chosen_class = if key == "Enter" {
                    Some(PlayerClass::all()[s.class_cursor])
                } else {
                    None
                };
                if let Some(chosen_class) = chosen_class {
                    s.daily_mode = false;
                    if s.total_runs == 0 {
                        s.start_tutorial(chosen_class);
                    } else {
                        let (sx, sy) = s.level.start_pos();
                        s.player = s.make_player(sx, sy, chosen_class);
                        s.reset_item_lore();
                        s.combat = CombatState::Explore;
                        let class_name = chosen_class.data().name_en;
                        s.message = format!("You chose {}! Explore the dungeon...", class_name);
                        s.message_timer = 120;
                        s.spawn_enemies();
                        let (px, py) = (s.player.x, s.player.y);
                        compute_fov(&mut s.level, px, py, FOV_RADIUS);
                    }
                    s.render();
                }
                return;
            }

            if matches!(s.combat, CombatState::TacticalBattle(_)) {
                event.prevent_default();
                let gs = &mut *s;
                let mut old_combat = std::mem::replace(&mut gs.combat, CombatState::Explore);
                if let CombatState::TacticalBattle(ref mut battle) = old_combat {
                    let log_len_before = battle.log.len();
                    let result = combat::input::handle_input(battle, key.as_str());

                    // Drain queued audio events from combat
                    for audio_event in battle.audio_events.drain(..) {
                        if let Some(ref audio) = gs.audio {
                            match audio_event {
                                combat::AudioEvent::EnemyDeath => audio.play_enemy_death(),
                                combat::AudioEvent::CriticalHit => audio.play_critical_hit(),
                                combat::AudioEvent::ProjectileLaunch => {
                                    audio.play_projectile_launch()
                                }
                                combat::AudioEvent::ProjectileImpact => {
                                    audio.play_projectile_impact()
                                }
                                combat::AudioEvent::Heal => audio.play_heal(),
                                combat::AudioEvent::ShieldBlock => audio.play_shield_block(),
                                combat::AudioEvent::StatusBurn => audio.play_status_burn(),
                                combat::AudioEvent::StatusPoison => audio.play_status_poison(),
                                combat::AudioEvent::StatusSlow => audio.play_status_slow(),
                                combat::AudioEvent::SpellElement(ref elem) => {
                                    audio.play_spell_element(elem)
                                }
                                combat::AudioEvent::TurnTick => audio.play_turn_tick(),
                                combat::AudioEvent::TypingCorrect => audio.play_typing_correct(),
                                combat::AudioEvent::TypingError => audio.play_typing_error(),
                                combat::AudioEvent::WaterSplash => audio.play_water_splash(),
                                combat::AudioEvent::LavaRumble => audio.play_lava_rumble(),
                                combat::AudioEvent::ComboStrike => audio.play_critical_hit(),
                                combat::AudioEvent::GravityPull => audio.play_gravity_pull(),
                                combat::AudioEvent::SteamVent => audio.play_steam_vent(),
                                combat::AudioEvent::OilIgnition => audio.play_oil_ignition(),
                                combat::AudioEvent::CratePush => audio.play_crate_push(),
                                combat::AudioEvent::CrateCrush => audio.play_crate_crush(),
                                combat::AudioEvent::ConveyorMove => audio.play_conveyor(),
                                combat::AudioEvent::ChainExplosion => audio.play_chain_explosion(),
                            }
                        }
                    }
                    // Scan new log messages for particle/shake triggers
                    for msg in &battle.log[log_len_before..] {
                        if msg.contains("Collision!") || msg.contains("Slammed") {
                            gs.trigger_shake(4);
                            let px = gs.renderer.canvas_w / 2.0;
                            let py = gs.renderer.canvas_h / 2.0;
                            gs.particles
                                .spawn_knockback_collision(px, py, &mut gs.rng_state);
                        }
                        if msg.contains("CHENGYU!") {
                            gs.trigger_shake(6);
                            let px = gs.renderer.canvas_w / 2.0;
                            let py = gs.renderer.canvas_h / 3.0;
                            gs.particles.spawn_chengyu(px, py, &mut gs.rng_state);
                        }
                        if msg.contains("Super effective!") {
                            let px = gs.renderer.canvas_w / 2.0;
                            let py = gs.renderer.canvas_h / 2.0;
                            gs.particles
                                .spawn_wuxing_effective(px, py, &mut gs.rng_state);
                        }
                    }

                    // SRS tracking: consume last_answer from tactical battle
                    if let Some((hanzi, correct)) = battle.last_answer.take() {
                        gs.srs.record(hanzi, correct);
                        if correct {
                            gs.run_correct_answers += 1;
                            gs.answer_streak += 1;
                            // ResearchLab: double vocab XP
                            if gs.current_location_type == Some(crate::world::LocationType::ResearchLab) {
                                gs.srs.record(hanzi, true);
                                gs.run_correct_answers += 1;
                            }
                        } else {
                            gs.run_wrong_answers += 1;
                            gs.answer_streak = 0;
                        }
                    }

                    if let Some(spell_idx) = battle.spent_spell_index.take() {
                        if spell_idx < gs.player.spells.len() {
                            gs.player.spells.remove(spell_idx);
                        }
                        if spell_idx < battle.available_spells.len() {
                            battle.available_spells.remove(spell_idx);
                        }
                    }

                    for consumed in &battle.consumed_radicals {
                        if let Some(pos) = gs.player.radicals.iter().position(|r| r == consumed) {
                            gs.player.radicals.remove(pos);
                        }
                    }
                    battle.consumed_radicals.clear();

                    match result {
                        combat::input::BattleEvent::Flee => {
                            // Nearest alive enemy gets a free hit
                            let free_hit = battle
                                .units
                                .iter()
                                .filter(|u| matches!(u.kind, combat::UnitKind::Enemy(_)) && u.alive)
                                .map(|u| u.damage)
                                .next()
                                .unwrap_or(0);

                            combat::transition::exit_combat(
                                battle,
                                &mut gs.player,
                                &mut gs.enemies,
                            );

                            if free_hit > 0 {
                                if gs.player.shield {
                                    gs.player.shield = false;
                                    gs.message = "Fled! Shield absorbed the blow!".to_string();
                                } else {
                                    gs.player.hp -= free_hit;
                                    gs.message =
                                        format!("Fled! Hit for {} on the way out!", free_hit);
                                }
                            } else {
                                gs.message = "Fled from battle!".to_string();
                            }

                            if gs.player.hp <= 0 && !gs.try_phoenix_revive() {
                                gs.player.hp = 0;
                                gs.run_journal
                                    .log(RunEvent::DiedTo("fleeing".to_string(), gs.floor_num));
                                gs.post_mortem_page = 0;
                                gs.combat = CombatState::GameOver;
                                gs.message = gs.run_summary();
                                gs.message_timer = 255;
                                if let Some(ref audio) = gs.audio {
                                    audio.play_death();
                                }
                                gs.save_high_score();
                            } else {
                                gs.message_timer = 60;
                                gs.combat = CombatState::Explore;
                            }

                            gs.render();
                            return;
                        }
                        combat::input::BattleEvent::Victory => {
                            if let Some(ref audio) = gs.audio {
                                audio.play_victory();
                            }
                            let combo = battle.combo_streak;
                            let killed = combat::transition::exit_combat(
                                battle,
                                &mut gs.player,
                                &mut gs.enemies,
                            );
                            gs.handle_tactical_victory(&killed, combo);
                            gs.render();
                            return;
                        }
                        combat::input::BattleEvent::Defeat => {
                            let killer_name = battle
                                .units
                                .iter()
                                .find(|u| u.is_enemy() && u.alive)
                                .map(|u| u.hanzi.to_string())
                                .unwrap_or_else(|| "battle".to_string());
                            combat::transition::exit_combat(
                                battle,
                                &mut gs.player,
                                &mut gs.enemies,
                            );
                            gs.handle_tactical_defeat(killer_name);
                            gs.render();
                            return;
                        }
                        combat::input::BattleEvent::None => {}
                    }


                }
                gs.combat = old_combat;
                // Check for boss phase triggers after processing tactical battle input
                if let CombatState::TacticalBattle(ref battle) = gs.combat {
                    let mut trigger_idx = None;
                    for unit in &battle.units {
                        if let combat::UnitKind::Enemy(eidx) = unit.kind {
                            if unit.alive
                                && eidx < gs.enemies.len()
                                && gs.enemies[eidx].boss_kind.is_some()
                                && !gs.enemies[eidx].phase_triggered
                                && unit.hp <= unit.max_hp / 2
                            {
                                // Sync HP from battle unit to enemy array before triggering
                                gs.enemies[eidx].hp = unit.hp;
                                gs.enemies[eidx].max_hp = unit.max_hp;
                                trigger_idx = Some(eidx);
                                break;
                            }
                        }
                    }
                    if let Some(eidx) = trigger_idx {
                        gs.maybe_trigger_boss_phase(eidx);
                        gs.render();
                        return;
                    }
                }
                gs.render();
                return;
            }

            // Combat typing mode
            if matches!(s.combat, CombatState::Fighting { .. }) {
                event.prevent_default();
                match key.as_str() {
                    "Enter" => {
                        s.submit_answer();
                        s.render();
                    }
                    "Backspace" => {
                        s.backspace();
                        s.render();
                    }
                    "Escape" => {
                        // Flee — enemy gets a free hit (shield can block)
                        if let CombatState::Fighting { enemy_idx, .. } = s.combat {
                            if enemy_idx < s.enemies.len() && s.enemies[enemy_idx].is_alive() {
                                if s.player.shield {
                                    s.player.shield = false;
                                    s.message = "Fled! Shield absorbed the blow!".to_string();
                                    s.message_timer = 40;
                                } else {
                                    let dmg = s.enemies[enemy_idx].damage;
                                    s.player.hp -= dmg;
                                    s.message = format!(
                                        "Fled! {} hits for {}!",
                                        s.enemies[enemy_idx].hanzi, dmg
                                    );
                                    s.message_timer = 40;
                                }
                                if s.player.hp <= 0 && !s.try_phoenix_revive() {
                                    s.player.hp = 0;
                                    let cause = s.enemies[enemy_idx].hanzi.to_string();
                                    let fl = s.floor_num;
                                    s.run_journal.log(RunEvent::DiedTo(cause, fl));
                                    s.post_mortem_page = 0;
                                    s.combat = CombatState::GameOver;
                                    s.message = s.run_summary();
                                    s.message_timer = 255;
                                } else {
                                    s.combat = CombatState::Explore;
                                }
                            } else {
                                s.combat = CombatState::Explore;
                            }
                        }
                        s.typing.clear();
                        s.render();
                    }
                    " " => {
                        // Cast selected spell
                        s.use_spell();
                        s.render();
                    }
                    "r" | "R" => {
                        // Replay tone in listening mode
                        if s.listening_mode.is_active() {
                            if let CombatState::Fighting { enemy_idx, .. } = s.combat {
                                if enemy_idx < s.enemies.len() {
                                    let pinyin = s.enemies[enemy_idx].pinyin;
                                    let tone_num = pinyin
                                        .chars()
                                        .last()
                                        .and_then(|c| c.to_digit(10))
                                        .unwrap_or(1)
                                        as u8;
                                    if let Some(ref audio) = s.audio {
                                        audio.play_chinese_tone(tone_num);
                                    }
                                }
                            }
                        } else {
                            s.type_char(key.chars().next().unwrap_or('r'));
                            s.render();
                        }
                    }
                    _ => {
                        if let Some(ch) = key.chars().next() {
                            if key.len() == 1 && (ch.is_ascii_alphanumeric()) {
                                s.type_char(ch);
                                s.render();
                            }
                        }
                    }
                }
                return;
            }

            // Forge mode
            if matches!(s.combat, CombatState::Forging { .. }) {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message.clear();
                        s.message_timer = 0;
                        s.render();
                    }
                    "Enter" => {
                        s.forge_submit();
                        s.render();
                    }
                    "ArrowUp" | "w" | "W" => {
                        if let CombatState::Forging { ref mut cursor, .. } = s.combat {
                            if *cursor > 0 {
                                *cursor -= 1;
                                if let Some(ref audio) = s.audio {
                                    audio.play_menu_click();
                                }
                            }
                        }
                        s.render();
                    }
                    "ArrowDown" | "s" | "S" => {
                        if let CombatState::Forging {
                            ref recipes,
                            ref mut cursor,
                            ..
                        } = s.combat
                        {
                            if *cursor + 1 < recipes.len() {
                                *cursor += 1;
                                if let Some(ref audio) = s.audio {
                                    audio.play_menu_click();
                                }
                            }
                        }
                        s.render();
                    }
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                        let slot = key.parse::<usize>().unwrap_or(1) - 1;
                        if let CombatState::Forging {
                            ref recipes,
                            ref mut cursor,
                            ..
                        } = s.combat
                        {
                            if slot < recipes.len() {
                                *cursor = slot;
                            }
                        }
                        s.forge_submit();
                        s.render();
                    }
                    "e" | "E" => {
                        // Enter enchant mode — pick a slot first
                        let has_equip = s.player.weapon.is_some()
                            || s.player.armor.is_some()
                            || s.player.charm.is_some();
                        if !has_equip {
                            s.message = "No equipment to enchant!".to_string();
                            s.message_timer = 90;
                        } else if s.player.radicals.is_empty() {
                            s.message = "No radicals to enchant with!".to_string();
                            s.message_timer = 90;
                        } else {
                            s.combat = CombatState::Enchanting {
                                step: 0,
                                slot: 0,
                                page: 0,
                            };
                            s.message =
                                "Enchant: 1=Weapon 2=Armor 3=Charm. Pick slot, then radical."
                                    .to_string();
                            s.message_timer = 255;
                        }
                        s.render();
                    }
                    _ => {}
                }
                return;
            }

            // Enchanting mode (two-phase: step 0 = pick slot, step 1 = pick radical)
            if let CombatState::Enchanting { step, slot, page } = s.combat {
                event.prevent_default();
                if step == 0 {
                    // Phase 0: Select equipment slot
                    match key.as_str() {
                        "Escape" => {
                            s.combat = CombatState::Forging {
                                recipes: s.discovered_recipes.clone(),
                                cursor: 0,
                            };
                            s.message.clear();
                            s.message_timer = 0;
                        }
                        "1" => {
                            if s.player.weapon.is_some() {
                                s.combat = CombatState::Enchanting {
                                    step: 1,
                                    slot: 0,
                                    page: 0,
                                };
                                s.message = "Enchanting Weapon. Pick a radical.".to_string();
                                s.message_timer = 255;
                            } else {
                                s.message = "No Weapon equipped!".to_string();
                                s.message_timer = 90;
                            }
                        }
                        "2" => {
                            if s.player.armor.is_some() {
                                s.combat = CombatState::Enchanting {
                                    step: 1,
                                    slot: 1,
                                    page: 0,
                                };
                                s.message = "Enchanting Armor. Pick a radical.".to_string();
                                s.message_timer = 255;
                            } else {
                                s.message = "No Armor equipped!".to_string();
                                s.message_timer = 90;
                            }
                        }
                        "3" => {
                            if s.player.charm.is_some() {
                                s.combat = CombatState::Enchanting {
                                    step: 1,
                                    slot: 2,
                                    page: 0,
                                };
                                s.message = "Enchanting Charm. Pick a radical.".to_string();
                                s.message_timer = 255;
                            } else {
                                s.message = "No Charm equipped!".to_string();
                                s.message_timer = 90;
                            }
                        }
                        "ArrowUp" | "w" | "W" => {
                            if let CombatState::Enchanting { ref mut slot, .. } = s.combat {
                                if *slot > 0 {
                                    *slot -= 1;
                                }
                            }
                        }
                        "ArrowDown" | "s" | "S" => {
                            if let CombatState::Enchanting { ref mut slot, .. } = s.combat {
                                if *slot < 2 {
                                    *slot += 1;
                                }
                            }
                        }
                        "Enter" => {
                            let has_slot = match slot {
                                0 => s.player.weapon.is_some(),
                                1 => s.player.armor.is_some(),
                                2 => s.player.charm.is_some(),
                                _ => false,
                            };
                            if has_slot {
                                s.combat = CombatState::Enchanting {
                                    step: 1,
                                    slot,
                                    page: 0,
                                };
                                let slot_name = match slot {
                                    0 => "Weapon",
                                    1 => "Armor",
                                    _ => "Charm",
                                };
                                s.message = format!("Enchanting {}. Pick a radical.", slot_name);
                                s.message_timer = 255;
                            } else {
                                let slot_name = match slot {
                                    0 => "Weapon",
                                    1 => "Armor",
                                    _ => "Charm",
                                };
                                s.message = format!("No {} equipped!", slot_name);
                                s.message_timer = 90;
                            }
                        }
                        _ => {}
                    }
                } else {
                    // Phase 1: Select radical (keys 1-6 per page)
                    match key.as_str() {
                        "Escape" => {
                            s.combat = CombatState::Enchanting {
                                step: 0,
                                slot,
                                page: 0,
                            };
                            s.message = "Enchant: pick equipment slot.".to_string();
                            s.message_timer = 255;
                        }
                        "ArrowLeft" => {
                            if let CombatState::Enchanting { ref mut page, .. } = s.combat {
                                if *page > 0 {
                                    *page -= 1;
                                }
                            }
                        }
                        "ArrowRight" => {
                            let max_page = s.player.radicals.len().saturating_sub(1) / 6;
                            if let CombatState::Enchanting { ref mut page, .. } = s.combat {
                                if *page < max_page {
                                    *page += 1;
                                }
                            }
                        }
                        "1" | "2" | "3" | "4" | "5" | "6" => {
                            let key_idx = key.parse::<usize>().unwrap_or(1) - 1;
                            let abs_idx = page * 6 + key_idx;
                            if abs_idx < s.player.radicals.len() {
                                let radical = s.player.radicals[abs_idx];
                                s.player.enchantments[slot] = Some(radical);
                                s.player.radicals.remove(abs_idx);
                                let slot_name = match slot {
                                    0 => "Weapon",
                                    1 => "Armor",
                                    _ => "Charm",
                                };
                                let bonus = match radical {
                                    "力" | "火" => "+1 damage",
                                    "水" | "土" => "+1 defense",
                                    "心" => "+2 max HP",
                                    "金" => "+3 gold/kill",
                                    "目" => "+1 FOV",
                                    _ => "+1 damage",
                                };
                                if radical == "心" {
                                    s.player.max_hp += 2;
                                    s.player.hp = s.player.hp.min(s.player.effective_max_hp());
                                }
                                if let Some(ref audio) = s.audio {
                                    audio.play_forge();
                                }
                                let cam_x =
                                    s.player.x as f64 * 24.0 - s.renderer.canvas_w / 2.0 + 12.0;
                                let cam_y =
                                    s.player.y as f64 * 24.0 - s.renderer.canvas_h / 2.0 + 12.0;
                                let sx = s.player.x as f64 * 24.0 - cam_x + 12.0;
                                let sy = s.player.y as f64 * 24.0 - cam_y + 12.0;
                                let gs = &mut *s;
                                gs.particles.spawn_heal(sx, sy, &mut gs.rng_state);
                                s.message = format!(
                                    "Enchanted {} with {} ({})!",
                                    slot_name, radical, bonus
                                );
                                s.message_timer = 120;
                                s.combat = CombatState::Explore;
                                let recipe_count = s.discovered_recipes.len();
                                s.achievements.check_recipes(recipe_count);
                            } else {
                                s.message = "No radical at that slot.".to_string();
                                s.message_timer = 60;
                            }
                        }
                        _ => {}
                    }
                }
                s.render();
                return;
            }

            // Offering mode
            if let CombatState::Offering { altar_kind, cursor } = s.combat {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message.clear();
                        s.render();
                    }
                    "ArrowUp" | "w" => {
                        if cursor > 0 {
                            s.combat = CombatState::Offering {
                                altar_kind,
                                cursor: cursor - 1,
                            };
                        }
                        s.render();
                    }
                    "ArrowDown" | "s" => {
                        if cursor + 1 < s.player.items.len() {
                            s.combat = CombatState::Offering {
                                altar_kind,
                                cursor: cursor + 1,
                            };
                        }
                        s.render();
                    }
                    "Enter" => {
                        s.perform_offering(altar_kind, cursor);
                        s.render();
                    }
                    "p" | "P" => {
                        let has_curse = (s.player.weapon.is_some()
                            && s.player.weapon_state == ItemState::Cursed)
                            || (s.player.armor.is_some()
                                && s.player.armor_state == ItemState::Cursed)
                            || (s.player.charm.is_some()
                                && s.player.charm_state == ItemState::Cursed);
                        if has_curse {
                            if s.player.weapon_state == ItemState::Cursed {
                                s.player.weapon_state = ItemState::Normal;
                            }
                            if s.player.armor_state == ItemState::Cursed {
                                s.player.armor_state = ItemState::Normal;
                            }
                            if s.player.charm_state == ItemState::Cursed {
                                s.player.charm_state = ItemState::Normal;
                            }
                            s.message = "🔮 The altar purifies your cursed equipment!".to_string();
                            s.message_timer = 90;
                            s.combat = CombatState::Explore;
                        } else {
                            s.message = "You have no cursed equipment to purify.".to_string();
                            s.message_timer = 60;
                        }
                        s.render();
                    }
                    _ => {}
                }
                return;
            }

            // Dipping Source
            if let CombatState::DippingSource { cursor } = s.combat {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message.clear();
                        s.render();
                    }
                    "ArrowUp" | "w" => {
                        if cursor > 0 {
                            s.combat = CombatState::DippingSource { cursor: cursor - 1 };
                        }
                        s.render();
                    }
                    "ArrowDown" | "s" => {
                        if cursor + 1 < s.player.items.len() {
                            s.combat = CombatState::DippingSource { cursor: cursor + 1 };
                        }
                        s.render();
                    }
                    "Enter" => {
                        if cursor < s.player.items.len() {
                            let kind = s.player.items[cursor].kind();
                            if matches!(
                                kind,
                                ItemKind::MedHypo
                                    | ItemKind::ToxinGrenade
                                    | ItemKind::StimPack
                            ) {
                                s.combat = CombatState::DippingTarget {
                                    source_idx: cursor,
                                    cursor: 0,
                                };
                                s.message = "Dip into what? (Equip/Items)".to_string();
                            } else {
                                s.message = "Can only dip potions!".to_string();
                                s.message_timer = 60;
                            }
                        }
                        s.render();
                    }
                    _ => {}
                }
                return;
            }

            // Dipping Target
            if let CombatState::DippingTarget { source_idx, cursor } = s.combat {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message.clear();
                        s.render();
                    }
                    "ArrowUp" | "w" => {
                        if cursor > 0 {
                            s.combat = CombatState::DippingTarget {
                                source_idx,
                                cursor: cursor - 1,
                            };
                        }
                        s.render();
                    }
                    "ArrowDown" | "s" => {
                        // 0=Wep, 1=Arm, 2=Chm, 3+=Items
                        let max_cursor = 2 + s.player.items.len();
                        if cursor < max_cursor {
                            s.combat = CombatState::DippingTarget {
                                source_idx,
                                cursor: cursor + 1,
                            };
                        }
                        s.render();
                    }
                    "Enter" => {
                        s.perform_dip(source_idx, cursor);
                        s.render();
                    }
                    _ => {}
                }
                return;
            }

            // Shop mode
            if matches!(s.combat, CombatState::Shopping { .. }) {
                event.prevent_default();
                match key.as_str() {
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.shop_sell_mode = false;
                        s.message.clear();
                        s.message_timer = 0;
                        s.render();
                    }
                    "Tab" => {
                        s.shop_sell_mode = !s.shop_sell_mode;
                        if let CombatState::Shopping { ref mut cursor, .. } = s.combat {
                            *cursor = 0;
                        }
                        s.render();
                    }
                    "ArrowUp" | "w" | "W" => {
                        if let CombatState::Shopping {
                            ref mut cursor, ..
                        } = s.combat
                        {
                            if *cursor > 0 {
                                *cursor -= 1;
                            }
                        }
                        s.render();
                    }
                    "ArrowDown" | "s" | "S" => {
                        let max_len = if s.shop_sell_mode {
                            s.player.items.len()
                        } else if let CombatState::Shopping { ref items, .. } = s.combat {
                            items.len()
                        } else {
                            0
                        };
                        if let CombatState::Shopping {
                            ref mut cursor, ..
                        } = s.combat
                        {
                            if *cursor + 1 < max_len {
                                *cursor += 1;
                            }
                        }
                        s.render();
                    }
                    "Enter" => {
                        if s.shop_sell_mode {
                            s.shop_sell();
                        } else {
                            s.shop_buy();
                        }
                        s.render();
                    }
                    "r" | "R" => {
                        if !s.shop_sell_mode {
                            s.shop_reroll();
                        }
                        s.render();
                    }
                    "g" | "G" => {
                        if !s.shop_sell_mode {
                            s.shop_steal();
                        }
                        s.render();
                    }
                    _ => {}
                }
                return;
            }

            if matches!(s.combat, CombatState::Looking { .. }) {
                event.prevent_default();
                match key.as_str() {
                    "Escape" | "v" | "V" | "Enter" | " " => s.stop_look_mode(),
                    "ArrowUp" | "w" | "W" => s.move_look_cursor(0, -1),
                    "ArrowDown" | "s" | "S" => s.move_look_cursor(0, 1),
                    "ArrowLeft" | "a" | "A" => s.move_look_cursor(-1, 0),
                    "ArrowRight" | "d" | "D" => s.move_look_cursor(1, 0),
                    _ => {}
                }
                s.render();
                return;
            }

            if let CombatState::Aiming {
                spell_idx,
                ref mut dx,
                ref mut dy,
            } = s.combat
            {
                event.prevent_default();
                match key.as_str() {
                    "ArrowUp" | "w" | "W" => {
                        *dx = 0;
                        *dy = -1;
                    }
                    "ArrowDown" | "s" | "S" => {
                        *dx = 0;
                        *dy = 1;
                    }
                    "ArrowLeft" | "a" | "A" => {
                        *dx = -1;
                        *dy = 0;
                    }
                    "ArrowRight" | "d" | "D" => {
                        *dx = 1;
                        *dy = 0;
                    }
                    "Enter" | " " => {
                        let si = spell_idx;
                        let fdx = *dx;
                        let fdy = *dy;
                        s.fire_aimed_spell(si, fdx, fdy);
                    }
                    "Escape" => {
                        s.combat = CombatState::Explore;
                        s.message = "Cancelled aiming.".to_string();
                        s.message_timer = 30;
                    }
                    _ => {}
                }
                s.render();
                return;
            }

            // Exploration movement + item usage
            // Toggle codex with 'c'
            if key == "c" || key == "C" {
                s.show_codex = !s.show_codex;
                s.render();
                return;
            }
            // Toggle listening mode with 'l'
            if key == "l" || key == "L" {
                s.listening_mode = s.listening_mode.cycle();
                s.message = format!("Listening mode: {}", s.listening_mode.label());
                s.message_timer = 90;
                s.render();
                return;
            }
            // Close codex on Escape
            if s.show_codex {
                if key == "Escape" {
                    s.show_codex = false;
                    s.render();
                }
                return;
            }
            // Open star map with 'm'
            if key == "m" || key == "M" {
                s.game_mode = GameMode::Starmap;
                s.starmap_cursor = 0;
                s.message = "Opening star map...".to_string();
                s.message_timer = 60;
                s.render();
                return;
            }
            // Toggle minimap with 'n'
            if key == "n" || key == "N" {
                s.show_minimap = !s.show_minimap;
                s.render();
                return;
            }
            match key.as_str() {
                "1" | "2" | "3" | "4" | "5" => {
                    let idx = key.parse::<usize>().unwrap_or(1) - 1;
                    s.use_item(idx);
                    s.render();
                    return;
                }
                "x" | "X" => {
                    event.prevent_default();
                    s.descend_floor(true);
                    s.render();
                    return;
                }
                "v" | "V" => {
                    event.prevent_default();
                    if matches!(s.combat, CombatState::Explore) {
                        s.start_look_mode();
                    }
                    s.render();
                    return;
                }
                "o" | "O" => {
                    if let Tile::Terminal(kind) = s.level.tile(s.player.x, s.player.y) {
                        if s.player.items.is_empty() {
                            s.message = "You have nothing to offer.".to_string();
                            s.message_timer = 60;
                        } else {
                            s.combat = CombatState::Offering {
                                altar_kind: kind,
                                cursor: 0,
                            };
                            s.message = format!("Offer to {}? Select item.", kind.name());
                        }
                    } else {
                        s.message = "There is no altar here.".to_string();
                        s.message_timer = 60;
                    }
                    s.render();
                    return;
                }
                "p" | "P" => {
                    if let Tile::Terminal(kind) = s.level.tile(s.player.x, s.player.y) {
                        s.pray_at_altar(kind);
                    } else {
                        s.message = "You pray to the void. Silence.".to_string();
                        s.message_timer = 60;
                    }
                    s.render();
                    return;
                }
                "D" => {
                    if s.player.items.is_empty() {
                        s.message = "Inventory empty.".to_string();
                        s.message_timer = 60;
                    } else {
                        s.combat = CombatState::DippingSource { cursor: 0 };
                        s.message = "Dip which potion?".to_string();
                    }
                    s.render();
                    return;
                }
                "j" | "J" => {
                    s.combat = CombatState::Journal { page: 0 };
                    s.message = "📖 Character Journal".to_string();
                    s.message_timer = 120;
                    s.render();
                    return;
                }
                "h" | "H" => {
                    s.player.hubris_mode = !s.player.hubris_mode;
                    s.message = if s.player.hubris_mode {
                        "💀 HUBRIS MODE ON — enemies hit 1.5×, drops doubled!".to_string()
                    } else {
                        "Hubris mode deactivated.".to_string()
                    };
                    s.message_timer = 60;
                    s.render();
                    return;
                }
                "q" => {
                    s.player.cycle_spell();
                    if !s.player.spells.is_empty() {
                        let sp = &s.player.spells[s.player.selected_spell];
                        s.message =
                            format!("Spell: {} {} ({})", sp.hanzi, sp.meaning, sp.effect.label());
                        s.message_timer = 50;
                    }
                    s.render();
                    return;
                }
                " " => {
                    s.use_spell_explore();
                    s.render();
                    return;
                }
                _ => {}
            }
            let (dx, dy) = match key.as_str() {
                "ArrowUp" | "w" | "W" => (0, -1),
                "ArrowDown" | "s" | "S" => (0, 1),
                "ArrowLeft" | "a" | "A" => (-1, 0),
                "ArrowRight" | "d" => (1, 0),
                _ => return,
            };
            event.prevent_default();
            s.try_move(dx, dy);
            s.render();
        });
        doc.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Window resize handler — keep canvas filling the viewport
    {
        let state = Rc::clone(&state);
        let closure = Closure::<dyn FnMut()>::new(move || {
            let Ok(mut s) = state.try_borrow_mut() else { return; };
            if let Some(win) = window() {
                let w = win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(800.0) as u32;
                let h = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(600.0) as u32;
                s.renderer.canvas.set_width(w);
                s.renderer.canvas.set_height(h);
                s.renderer.sync_size();
                s.render();
            }
        });
        win.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Initial render
    state.borrow().render();

    // Animation loop for particles, screen shake, and flash effects
    {
        let state = Rc::clone(&state);
        let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
        let g = Rc::clone(&f);
        *g.borrow_mut() = Some(Closure::new(move || {
            {
                let Ok(mut s) = state.try_borrow_mut() else {
                    if let Some(win) = window() {
                        let _ = win.request_animation_frame(
                            f.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
                        );
                    }
                    return;
                }; // Tick music
                let mood = match s.combat {
                    CombatState::Fighting { enemy_idx, .. } => {
                        if enemy_idx < s.enemies.len() && s.enemies[enemy_idx].is_boss {
                            crate::audio::MusicMood::Boss
                        } else {
                            crate::audio::MusicMood::Combat
                        }
                    }
                    CombatState::TacticalBattle(ref battle) => {
                        if battle.is_boss_battle {
                            crate::audio::MusicMood::Boss
                        } else {
                            crate::audio::MusicMood::Combat
                        }
                    }
                    CombatState::GameOver => crate::audio::MusicMood::Silent,
                    _ => crate::audio::MusicMood::Explore,
                };
                if let Some(ref mut audio) = s.audio {
                    audio.set_mood(mood);
                    audio.tick_music();
                }

                // Tick achievement popup
                if s.achievement_popup.is_none() {
                    if let Some(id) = s.achievements.pop_popup() {
                        if let Some(def) = AchievementTracker::get_def(id) {
                            s.achievement_popup = Some((def.name, def.desc, 180));
                            // ~3 seconds at 60fps
                        }
                    }
                }
                if let Some((_, _, ref mut timer)) = s.achievement_popup {
                    if *timer > 0 {
                        *timer -= 1;
                    } else {
                        s.achievement_popup = None;
                    }
                }

                let had_message = s.message_timer > 0;
                if had_message {
                    s.tick_message();
                }

                {
                    let weather = if let CombatState::TacticalBattle(ref battle) = s.combat {
                        Some(battle.weather)
                    } else {
                        None
                    };
                    if let Some(weather) = weather {
                        let gs = &mut *s;
                        let cw = gs.renderer.canvas_w;
                        let ch = gs.renderer.canvas_h;
                        match weather {
                            combat::Weather::CoolantLeak => {
                                for _ in 0..3 {
                                    let x = (gs.rng_state & 0xFFFF) as f64 / 65536.0 * cw;
                                    gs.rng_state ^= gs.rng_state << 13;
                                    gs.rng_state ^= gs.rng_state >> 7;
                                    gs.rng_state ^= gs.rng_state << 17;
                                    gs.particles.spawn_rain_drop(x, 0.0, &mut gs.rng_state);
                                }
                            }
                            combat::Weather::SmokeScreen => {
                                if gs.rng_state % 4 == 0 {
                                    let x = (gs.rng_state & 0xFFFF) as f64 / 65536.0 * cw;
                                    gs.rng_state ^= gs.rng_state << 13;
                                    gs.rng_state ^= gs.rng_state >> 7;
                                    gs.rng_state ^= gs.rng_state << 17;
                                    let y = ch * 0.6
                                        + (gs.rng_state & 0xFFFF) as f64 / 65536.0 * ch * 0.4;
                                    gs.rng_state ^= gs.rng_state << 13;
                                    gs.rng_state ^= gs.rng_state >> 7;
                                    gs.rng_state ^= gs.rng_state << 17;
                                    gs.particles.spawn_fog_wisp(x, y, &mut gs.rng_state);
                                }
                            }
                            combat::Weather::DebrisStorm => {
                                for _ in 0..2 {
                                    let y = (gs.rng_state & 0xFFFF) as f64 / 65536.0 * ch;
                                    gs.rng_state ^= gs.rng_state << 13;
                                    gs.rng_state ^= gs.rng_state >> 7;
                                    gs.rng_state ^= gs.rng_state << 17;
                                    gs.particles.spawn_sand_grain(0.0, y, &mut gs.rng_state);
                                }
                            }
                            combat::Weather::EnergyFlux => {
                                if gs.rng_state % 3 == 0 {
                                    let x = (gs.rng_state & 0xFFFF) as f64 / 65536.0 * cw;
                                    gs.rng_state ^= gs.rng_state << 13;
                                    gs.rng_state ^= gs.rng_state >> 7;
                                    gs.rng_state ^= gs.rng_state << 17;
                                    let y = ch * 0.3
                                        + (gs.rng_state & 0xFFFF) as f64 / 65536.0 * ch * 0.5;
                                    gs.rng_state ^= gs.rng_state << 13;
                                    gs.rng_state ^= gs.rng_state >> 7;
                                    gs.rng_state ^= gs.rng_state << 17;
                                    gs.particles.spawn_ink_mote(x, y, &mut gs.rng_state);
                                }
                            }
                            combat::Weather::Normal => {}
                        }
                    }
                }

                s.particles.tick();
                if s.shake_timer > 0 {
                    s.shake_timer -= 1;
                }
                if let Some((_, _, _, ref mut a)) = s.flash {
                    *a -= 0.05;
                    if *a <= 0.0 {
                        s.flash = None;
                    }
                }

                // Tick tactical battle animations (Resolve/EnemyTurn/End phase timers).
                {
                    let gs = &mut *s;
                    let mut old_combat = std::mem::replace(&mut gs.combat, CombatState::Explore);
                    if let CombatState::TacticalBattle(ref mut battle) = old_combat {
                        let event = combat::tick::tick_battle(battle);

                        // Drain queued audio events from combat tick
                        for audio_event in battle.audio_events.drain(..) {
                            if let Some(ref audio) = gs.audio {
                                match audio_event {
                                    combat::AudioEvent::EnemyDeath => audio.play_enemy_death(),
                                    combat::AudioEvent::CriticalHit => audio.play_critical_hit(),
                                    combat::AudioEvent::ProjectileLaunch => {
                                        audio.play_projectile_launch()
                                    }
                                    combat::AudioEvent::ProjectileImpact => {
                                        audio.play_projectile_impact()
                                    }
                                    combat::AudioEvent::Heal => audio.play_heal(),
                                    combat::AudioEvent::ShieldBlock => audio.play_shield_block(),
                                    combat::AudioEvent::StatusBurn => audio.play_status_burn(),
                                    combat::AudioEvent::StatusPoison => audio.play_status_poison(),
                                    combat::AudioEvent::StatusSlow => audio.play_status_slow(),
                                    combat::AudioEvent::SpellElement(ref elem) => {
                                        audio.play_spell_element(elem)
                                    }
                                    combat::AudioEvent::TurnTick => audio.play_turn_tick(),
                                    combat::AudioEvent::TypingCorrect => {
                                        audio.play_typing_correct()
                                    }
                                    combat::AudioEvent::TypingError => audio.play_typing_error(),
                                    combat::AudioEvent::WaterSplash => audio.play_water_splash(),
                                    combat::AudioEvent::LavaRumble => audio.play_lava_rumble(),
                                    combat::AudioEvent::ComboStrike => audio.play_critical_hit(),
                                    combat::AudioEvent::GravityPull => audio.play_gravity_pull(),
                                    combat::AudioEvent::SteamVent => audio.play_steam_vent(),
                                    combat::AudioEvent::OilIgnition => audio.play_oil_ignition(),
                                    combat::AudioEvent::CratePush => audio.play_crate_push(),
                                    combat::AudioEvent::CrateCrush => audio.play_crate_crush(),
                                    combat::AudioEvent::ConveyorMove => audio.play_conveyor(),
                                    combat::AudioEvent::ChainExplosion => audio.play_chain_explosion(),
                                }
                            }
                        }

                        match event {
                            combat::input::BattleEvent::Victory => {
                                if let Some(ref audio) = gs.audio {
                                    audio.play_victory();
                                }
                                let combo = battle.combo_streak;
                                let killed = combat::transition::exit_combat(
                                    battle,
                                    &mut gs.player,
                                    &mut gs.enemies,
                                );
                                gs.handle_tactical_victory(&killed, combo);
                            }
                            combat::input::BattleEvent::Defeat => {
                                let killer_name = battle
                                    .units
                                    .iter()
                                    .find(|u| u.is_enemy() && u.alive)
                                    .map(|u| u.hanzi.to_string())
                                    .unwrap_or_else(|| "an enemy".to_string());
                                combat::transition::exit_combat(
                                    battle,
                                    &mut gs.player,
                                    &mut gs.enemies,
                                );
                                gs.handle_tactical_defeat(killer_name);
                            }
                            _ => {

                                gs.combat = old_combat;
                            }
                        }
                    } else {
                        gs.combat = old_combat;
                    }
                }

                s.render();
            }
            // Schedule next frame
            if let Some(win) = window() {
                let _ = win
                    .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
            }
        }));
        let win = window().ok_or("no window")?;
        let _ = win.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }

    Ok(())
}


#[cfg(test)]
mod tests;
