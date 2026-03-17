# Tactical Combat Design — Radical Roguelike × Mewgenics

> Transform pinyin-typing combat into Mewgenics-inspired tactical grid battles while preserving the Chinese-character-learning identity.

---

## 1. Design Philosophy

**Core tension**: Mewgenics is a pure tactics game (positioning, mana economy, terrain). Radical Roguelike is a language-learning game (pinyin recall, character composition, spaced repetition). The tactical combat must serve BOTH — every tactical action should reinforce language learning.

**Key principle**: **Hanzi typing IS the action economy.** You don't just click "Fire Breath" — you type the pinyin of the spell character to activate it. Correct input = ability fires. Wrong input = turn wasted + enemy counterattack opportunity. This preserves the educational loop while adding tactical depth.

---

## 2. Battle Arena

### 2.1 Grid

- **Separate battle arena** — NOT the dungeon map. When combat triggers, a small tactical grid is generated.
- **Size**: 7×7 (normal encounters), 9×9 (elite encounters), 11×11 (boss encounters).
- **Tile types** for the battle grid:
  - `Open` — normal walkable ground
  - `Obstacle` — impassable pillar/rock (blocks movement and line of sight)
  - `Grass` — provides cover (+30% dodge chance, burnable by fire spells → becomes `Scorched`)
  - `Water` — walkable but slowed (costs 2 movement), conducts lightning spells to adjacent water tiles
  - `Ice` — slippery, units that enter slide 1 tile in their movement direction; fire spells melt to `Water`
  - `Scorched` — burned grass, no effect (prevents re-burning)
  - `InkPool` — special terrain that boosts spell power by +1 for units standing in it
  - `BrokenGround` — rough terrain, costs 2 movement but no other effect

### 2.2 Arena Generation

- Arena layout is procedural, seeded by floor number + room index.
- Higher floors → more complex terrain, more obstacles.
- Boss arenas have fixed thematic layouts per `BossKind`.
- Arenas pull from the dungeon's existing hazard tiles (Water, Oil→InkPool, Spikes→BrokenGround) for thematic consistency.

### 2.3 Unit Placement

- **Player** starts at the bottom-center of the arena.
- **Enemies** are placed in the top half, spread out.
- Boss encounters place the boss center-top with 1–2 adds on flanks.

---

## 3. Turn System

### 3.1 Turn Order

Inspired by Mewgenics' SPD-based initiative:

- Each unit has a **Speed** value.
- Turn order is determined by Speed (highest goes first).
- Ties broken by: player > enemies (left-to-right, top-to-bottom).
- Turn order is displayed as a queue in the UI (portrait strip at top of screen).

### 3.2 Speed Values

- **Player base speed**: Determined by class.
  - Fast classes (Assassin, Thief, Swordsman): 5
  - Normal classes (Scholar, Warrior, Monk, etc.): 4
  - Slow classes (Ironclad, Earthmover): 3
- **Enemy base speed**: 3 (normal), 4 (elite), varies by boss.
- Haste status: +2 Speed.
- Tiger form: +2 Speed.
- Stone form: −1 Speed.

### 3.3 Action Economy (Per Turn)

Each unit gets **one turn** per round consisting of:

1. **Move** (optional): Up to N tiles (Manhattan distance).
   - Player base movement: 3 tiles.
   - Enemies: 2 tiles (normal), 3 tiles (elite/boss).
   - Haste: +1 movement.
   - Water/Ice/BrokenGround: costs 2 movement per tile.
   
2. **Action** (one of):
   - **Basic Attack** — target adjacent enemy. Player must type the enemy's pinyin correctly.
     - Correct → deal weapon damage + bonuses.
     - Wrong → turn ends, target gets a free counterattack.
   - **Cast Spell** — select a forged spell, type its pinyin to activate.
     - Correct → spell effect fires (AoE, heal, shield, etc.) with range/area targeting.
     - Wrong → spell fizzles, turn wasted.
   - **Use Item** — use a consumable (no typing required, but costs the action).
   - **Wait** — end turn, gain +1 to next turn's movement (stored movement, max +2).
   - **Defend** — end turn, reduce incoming damage by 50% until next turn.

3. **Free actions** (don't cost action):
   - Cycle selected spell
   - Rotate facing direction (4 cardinal directions)

### 3.4 Facing & Backstab

- Each unit faces a cardinal direction (N/S/E/W).
- Attacking from behind (opposite of facing): **+50% damage** (backstab).
- Attacking from the side: +25% damage (flank).
- Moving automatically updates facing toward movement direction.
- **Free rotation** at any point during your turn.

---

## 4. Hanzi Integration — The Educational Core

### 4.1 Basic Attack = Pinyin Recall

When the player selects "Attack" and targets an adjacent enemy:
- The enemy's **hanzi** is displayed prominently.
- Player types the **pinyin** (with tone number).
- **Correct**: Attack lands. Damage = base (2) + weapon bonus + equipment bonuses + class bonuses + backstab/flank.
- **Wrong**: Miss. Enemy gets a free counterattack (reduced damage: 50% of their normal).
- **Partial** (correct syllable but wrong tone): Half damage, no counterattack.

This is the existing mechanic adapted to tactical context — same SRS tracking, same combo chains.

### 4.2 Spell Casting = Pinyin + Targeting

When using a forged spell:
1. Player selects spell from spell list.
2. A **targeting reticle** appears on the grid showing range and area.
3. Player positions the reticle (arrow keys/WASD).
4. Player types the spell's **pinyin** to confirm.
5. Correct → Spell effect activates at target location.
6. Wrong → Spell fizzles, turn wasted.

**Spell ranges and areas** (new for tactical combat):

| SpellEffect | Range | Area | Notes |
|-------------|-------|------|-------|
| FireAoe(N) | 4 tiles | 3×3 cross | Burns Grass tiles, melts Ice |
| Heal(N) | Self only | Single | — |
| Reveal | — | Whole arena | Removes Fog (if fog variant is used) |
| Shield | Self only | Single | Blocks next hit |
| StrongHit(N) | 2 tiles (ranged strike) | Single target | — |
| Drain(N) | Adjacent (1 tile) | Single target | Must be next to target |
| Stun | 3 tiles | Single target | — |
| Pacify | 3 tiles | Single target | Removes enemy from battle (no loot) |

### 4.3 Combo System (Preserved)

- Sequential correct pinyin answers build combo streak (existing 6 tiers: Good→Radical).
- Combo damage multiplier carries across turns within the same battle.
- Wrong answer resets combo to 0.
- Combo streak displayed in battle HUD.

### 4.4 Elite Chain Combat (Adapted)

- Elite enemies still require multi-syllable pinyin chains.
- In tactical combat: each correct syllable deals damage proportional to the phase (1/N of total).
- All syllables must be typed in sequence during a single "attack" action.
- Failing mid-chain: partial damage dealt for completed syllables, enemy counterattacks.

### 4.5 Enemy Component Shields (Preserved)

- Complex enemies with component shields still require typing the component radicals first.
- In tactical combat: shield-breaking is a separate action ("Break Shield" — type the component's pinyin).
- Each shield-break action removes one component layer.
- Once shields are down, normal attacks work.

---

## 5. Enemy Tactical Behavior

### 5.1 Existing AI Behaviors → Tactical Grid

Map the 6 existing `AiBehavior` variants to tactical grid movement:

| AiBehavior | Tactical Movement | Tactical Ability Use |
|------------|-------------------|---------------------|
| Chase | Move toward player, attack when adjacent | Aggressive — uses offensive radical actions |
| Retreat | Keep 2–3 tiles away, use ranged radical actions | Defensive — heals, shields |
| Ambush | Stay still until player within 3 tiles, then rush | Burst — all offensive actions in one turn |
| Sentinel | Hold position, only engage adjacent | Tank — uses Fortify, Armor |
| Kiter | Maintain 3-tile distance, attack at range | Harass — uses ranged actions (FireBreath, Earthquake) |
| Pack | Move toward player only when 2+ allies nearby | Swarm — uses Rally, CallAlly |

### 5.2 Radical Actions as Tactical Abilities

The 18 existing `RadicalAction` variants become tactical grid abilities:

| RadicalAction | Tactical Adaptation | Range | Area |
|---------------|-------------------|-------|------|
| FireBreath | 3-tile line in facing direction, burns grass | 3 | Line |
| WaterShield | Self-heal 2 HP | Self | Single |
| PowerStrike | +2 damage on next melee attack | Self buff | — |
| SelfHeal | Heal 3 HP | Self | Single |
| WarCry | -10 spirit to all units in 2-tile radius | 2 | AoE |
| TrueSight | Remove player's Shield status | 4 | Single |
| Disarm | -1 player weapon bonus for 2 turns | Adjacent | Single |
| Root | Player can't move next turn | 3 | Single |
| Fortify | +1 damage permanently this battle | Self | — |
| Radiance | Next player hit deals half damage | 3 | AoE |
| ShadowStep | Teleport to any tile within 3, dodge next attack | 3 | Self |
| CallAlly | Summon 1 weak enemy adjacent to self | Self | — |
| Charm | Player's next action targets random adjacent tile | 3 | Single |
| Swift | Take an extra action this turn | Self | — |
| Leech | Melee attack that heals equal to damage | Adjacent | Single |
| Multiply | Next attack hits target + all adjacent to target | Self buff | — |
| Armor | -2 damage from next player hit | Self | — |
| Earthquake | 1 damage to all grounded units in 2 radius, creates BrokenGround | 2 | AoE |

### 5.3 Enemy Turn AI

On each enemy's turn:
1. Evaluate threat (distance to player, HP ratio, ally count).
2. Choose action based on `AiBehavior` + available `RadicalAction`s.
3. Each `RadicalAction` has a 30% proc chance per turn (same as current system).
4. If no action procs, perform basic attack (move toward player + melee).
5. Enemy basic melee: no typing required — they just deal their damage value.

### 5.4 Boss Tactical Mechanics

Each `BossKind` gets unique tactical behavior:

| BossKind | Tactical Mechanic |
|----------|-------------------|
| Gatekeeper | Summons 門 ward tiles that block movement; must type 門's pinyin to destroy |
| Scholar | At 50% HP, initiates sentence duel (existing mechanic, now as a "phase" on the grid) |
| Elementalist | Adapts resistance to last spell school used; terrain shifts each phase |
| MimicKing | Spawns decoy copies; must identify real one by meaning |
| InkSage | Creates InkPool terrain; calligraphy trial at 50% HP |
| RadicalThief | Steals a radical on wrong answer; stolen radicals appear as pickups on grid |

---

## 6. Terrain Interactions

Inspired by Mewgenics' environmental combat:

| Source | Target Terrain | Result |
|--------|---------------|--------|
| Fire spell | Grass | Burns → Scorched (damages units on tile) |
| Fire spell | Ice | Melts → Water |
| Fire spell | Water | Steam cloud (blocks LOS for 2 turns) |
| Lightning/Stun spell | Water | Conducts to all connected Water tiles (stun) |
| Ice/Water spell | Water | Freezes → Ice |
| Earthquake | Open/Grass | Creates BrokenGround |
| Any knockback | Into Obstacle | +1 bonus damage |
| Any knockback | Into Water | Unit is Slowed for 1 turn |
| Any knockback | Off grid edge | Prevented (units stop at edge) |

---

## 7. Exhaustion Timer (Anti-Turtle)

Adapted from Mewgenics' Turn 10 Exhaustion:

- **Turn 8**: Warning — "The ink grows restless..."
- **Turn 10**: Exhaustion begins — player takes 1 unblockable damage per turn.
- **Turn 12+**: Damage escalates: 2 per turn, then 3, etc.
- This prevents defensive stalling and keeps battles snappy.
- Boss battles: Exhaustion starts at Turn 15 instead.

---

## 8. Damage & Stats

### 8.1 Player Stats in Tactical Combat

No new stat system — reuse existing stats with tactical interpretation:

| Existing Stat | Tactical Role |
|---------------|--------------|
| HP / max_hp | Same — hit points |
| Weapon (BonusDamage) | +N to basic attack damage |
| Armor (DamageReduction) | -N from all incoming damage |
| Charm effects | Same passive effects |
| Enchantments | Same bonuses (力/火=+1 dmg, 水/土=+1 def, etc.) |
| Shield (bool) | Blocks one incoming hit |
| Spirit | Drains by 2 per battle (not per turn) — incentivizes fast combat |
| Form bonuses | Flame=fire immune, Tiger=+2 speed+1 dmg, Mist=+30% dodge, Stone=+2 def-1 speed |

### 8.2 Damage Formula

```
Raw Damage = base_damage + weapon_bonus + enchant_bonus + class_bonus 
             + form_bonus + deity_bonus + combo_multiplier
             + backstab_bonus + flank_bonus

Final Damage = max(1, Raw Damage - target_armor - target_damage_reduction)
```

Where:
- `base_damage` = 2 (basic attack), or spell damage value
- `backstab_bonus` = raw_damage × 0.50 (if attacking from behind)
- `flank_bonus` = raw_damage × 0.25 (if attacking from side)
- `combo_multiplier` = combo tier multiplier (1.0× to 2.0×)

### 8.3 Enemy Damage

```
Enemy Damage = enemy.damage + radical_action_bonus - player_armor - player_damage_reduction
Final = max(1, Enemy Damage)
```

---

## 9. Transition Flow

### 9.1 Entering Tactical Combat

When the player bumps an enemy (or enemy bumps player) during dungeon exploration:

1. Gather all enemies in the current room (or within alert radius).
2. Generate a tactical arena sized for the encounter.
3. Place terrain based on the dungeon room's existing tiles.
4. Place player and enemies on the grid.
5. Transition to `CombatState::TacticalBattle { ... }`.
6. Begin turn cycle.

### 9.2 Exiting Tactical Combat

When all enemies are defeated (or pacified):

1. Calculate rewards (gold, radicals, XP for companions).
2. Apply kill effects (heal-on-kill, gold bonus, etc.).
3. Return to `CombatState::Explore` at the player's dungeon position.
4. Spirit cost applied (2 per battle).

### 9.3 Fleeing

- Player can attempt to flee by moving to any edge tile and pressing Escape.
- Costs full turn. Enemies get one free attack each.
- Success rate: 60% base, +20% for Thief/Assassin, +10% per empty tile between player and nearest enemy.

---

## 10. State Machine — `CombatState` Changes

Replace `Fighting` with the new tactical states:

```rust
enum CombatState {
    // ... existing states ...
    
    // NEW: Tactical combat states
    TacticalBattle {
        arena: TacticalArena,       // The battle grid
        units: Vec<BattleUnit>,     // All units (player + enemies)
        turn_queue: Vec<usize>,     // Unit indices in turn order
        current_turn: usize,        // Index into turn_queue
        phase: TacticalPhase,       // What the current unit is doing
        turn_number: u32,           // For exhaustion timer
        combo_streak: u32,          // Combo chain counter
    },
}

enum TacticalPhase {
    /// Waiting for unit to choose action
    Choosing,
    /// Player is selecting a movement destination
    Moving { valid_tiles: Vec<(i32, i32)> },
    /// Player is selecting an attack target
    Targeting { range: i32, area: AreaShape },
    /// Player is typing pinyin for attack/spell
    Typing { target: (i32, i32), kind: TypingKind },
    /// Animating an action (brief delay for visual feedback)
    Animating { timer_ms: f64 },
    /// Enemy is executing their turn (AI-driven)
    EnemyTurn { timer_ms: f64 },
    /// Battle over — showing results
    Results { timer_ms: f64 },
    /// Boss special phase (sentence duel, calligraphy trial, etc.)
    BossPhase { kind: BossPhaseKind },
}

enum TypingKind {
    BasicAttack { enemy_idx: usize },
    SpellCast { spell_idx: usize },
    ShieldBreak { enemy_idx: usize, component: &'static str },
    EliteChain { enemy_idx: usize, syllable_idx: usize },
}
```

---

## 11. Code Architecture

### 11.1 New Module: `src/combat/`

```
src/combat/
  mod.rs          — TacticalArena, BattleUnit, public API
  grid.rs         — Arena grid, terrain, pathfinding, line of sight
  turn.rs         — Turn order calculation, turn cycling
  action.rs       — Action resolution (attack, spell, item, move, defend)
  ai.rs           — Enemy tactical AI (behavior → grid actions)
  transition.rs   — Enter/exit combat, arena generation from dungeon context
```

### 11.2 Key Structs

```rust
// combat/mod.rs
pub struct TacticalArena {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<BattleTile>,
    pub turn_number: u32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum BattleTile {
    Open,
    Obstacle,
    Grass,
    Water,
    Ice,
    Scorched,
    InkPool,
    BrokenGround,
}

pub struct BattleUnit {
    pub kind: UnitKind,
    pub x: i32,
    pub y: i32,
    pub facing: Direction,
    pub speed: i32,
    pub movement: i32,        // movement points per turn
    pub stored_movement: i32, // from Wait action
    pub hp: i32,
    pub max_hp: i32,
    pub defending: bool,
}

pub enum UnitKind {
    Player,
    Enemy(usize),  // index into GameState.enemies
}

#[derive(Clone, Copy, PartialEq)]
pub enum Direction {
    North,
    South,
    East,
    West,
}
```

### 11.3 Integration Points

- `game.rs`: New match arms in `handle_key()` for `CombatState::TacticalBattle`.
- `game.rs`: `submit_answer()` adapted for tactical typing (basic attack, spell cast, shield break).
- `render.rs`: New `draw_tactical_battle()` function for grid rendering.
- `enemy.rs`: No changes to `Enemy` struct — tactical behavior derived from existing fields.
- `player.rs`: Add `speed()` method based on class. No struct changes.
- `status.rs`: No changes — status effects tick per battle turn instead of per dungeon turn.

---

## 12. Rendering Plan

### 12.1 Battle Grid

- **Camera**: Centered on arena, zoomed in. No scrolling (arena fits on screen).
- **Tile size**: 32×32 pixels (larger than dungeon's 24×24 for clarity).
- **Grid lines**: Subtle dotted lines between tiles.
- **Terrain colors**:
  - Open: dark gray `#2a2a3a`
  - Obstacle: dark brown `#4a3a2a`
  - Grass: green `#3a5a2a` with `~` pattern
  - Water: blue `#2a3a6a` with wave animation
  - Ice: light blue `#8ac`
  - Scorched: dark `#1a1a1a`
  - InkPool: deep purple `#3a1a4a`
  - BrokenGround: orange-brown `#5a4a2a` with crack lines

### 12.2 Units

- **Player**: `@` glyph (or form glyph) in class color, 32×32.
- **Enemies**: Their **hanzi character** rendered large in the tile, colored by type:
  - Normal: white
  - Elite: gold
  - Boss: red with glow
- **Facing indicator**: Small triangle/arrow on the unit's facing edge.
- **HP bar**: Thin bar below each unit.

### 12.3 UI Overlays

- **Turn order queue**: Top of screen, horizontal strip of unit portraits.
- **Action menu**: Bottom of screen when it's player's turn:
  ```
  [M]ove  [A]ttack  [S]pell  [I]tem  [D]efend  [W]ait  [Esc]Flee
  ```
- **Targeting overlay**: Highlighted tiles showing valid targets (blue=move, red=attack range, yellow=spell area).
- **Typing input**: Large centered input field when typing pinyin (same style as current combat).
- **Combo badge**: Top-right, showing current streak tier.
- **Exhaustion warning**: Screen border pulses red after Turn 8.
- **Battle log**: Right side, scrolling text of actions taken.

---

## 13. Preserved Systems Mapping

| Existing System | Tactical Adaptation |
|----------------|-------------------|
| Pinyin typing combat | Now contextual: basic attack = type enemy pinyin, spell = type spell pinyin |
| Combo chains | Same mechanic, tracked per-battle across turns |
| Equipment bonuses | Apply to tactical damage/defense calculations |
| Enchantments | Same bonuses, apply in tactical context |
| Status effects | Tick per tactical turn (not dungeon turn) |
| Radical actions (enemy) | Become tactical grid abilities with range/area |
| Boss phases | Triggered at HP thresholds during tactical battle |
| Elite chain combat | Multi-syllable attack sequence within one action |
| Component shields | Shield-break as separate tactical action |
| Deity bonuses | Apply to damage/healing calculations |
| Polymorph forms | Modify speed, damage, defense, special immunities |
| Companion system | Companions act as 2nd unit in battle (future enhancement) |
| SRS tracking | Same — every pinyin typed is tracked for spaced repetition |
| Spirit energy | Costs 2 per battle (not per turn), incentivizes fast resolution |
| Items | Usable as tactical action (no typing cost) |
| Spells (forged) | Now have range/area, require pinyin to cast |
| Class bonuses | Map to speed, movement, and existing damage/HP bonuses |

---

## 14. Implementation Order

1. **Phase A**: Combat module foundation (`src/combat/mod.rs`, `grid.rs`) — arena struct, tile types, grid utilities.
2. **Phase B**: Turn system (`turn.rs`) — speed calculation, turn queue, turn cycling.
3. **Phase C**: Actions (`action.rs`) — movement, basic attack with pinyin, spell casting with targeting.
4. **Phase D**: Enemy AI (`ai.rs`) — behavior-to-grid-action mapping, radical action usage.
5. **Phase E**: Transition (`transition.rs`) — enter/exit combat, arena generation from dungeon context.
6. **Phase F**: Game state integration (`game.rs`) — new CombatState variant, input handling, answer submission.
7. **Phase G**: Rendering (`render.rs`) — grid, units, targeting overlays, action menu, typing input.
8. **Phase H**: Terrain interactions — fire+grass, lightning+water, knockback, etc.
9. **Phase I**: Boss mechanics — phase triggers, special encounters on tactical grid.
10. **Phase J**: Polish — exhaustion timer, flee mechanic, combo display, battle log.

---

## 15. What Changes vs. What Stays

### Changes
- Combat is now grid-based with movement, positioning, and facing.
- Enemies act independently on the grid with tactical AI.
- Spells have range and area of effect on the grid.
- Terrain matters — positioning and environmental interactions.
- Turn order based on Speed stat.
- Backstab/flank damage bonuses for positioning.
- Exhaustion timer to prevent turtling.
- New rendering for battle grid.

### Stays
- **Typing pinyin is still the core mechanic** — you type to attack and cast.
- All existing equipment, status effects, deity bonuses, enchantments.
- Combo chain system.
- Elite chain combat (multi-syllable).
- Component shield breaking.
- Boss phase mechanics (sentence duels, calligraphy trials).
- SRS tracking for all pinyin typed.
- Radical forge system (spells are still forged from radicals).
- Spirit energy, items, class system.
- All challenge mini-games (tone battles, etc.) remain as shrine interactions during exploration.
