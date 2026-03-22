# Radical Starfinder — Game Design Ideas

> A pixel-art space roguelike where you **forge Chinese characters from quantum glyphs**, **battle hostile aliens by typing pinyin**, and explore procedural space stations. The roguelike death loop naturally drives **spaced repetition** — words you struggle with haunt you more.

## Tech Stack
- **Rust → WebAssembly** (wasm-bindgen, web-sys) — same proven stack as hanzi-cat
- **Canvas 2D** rendering with pixel art sprites
- **Web Audio** for procedural sound effects
- Tiny binary, runs in any modern browser

---

## Core Gameplay Loop

```
Enter dungeon floor
    → Explore rooms (fog of war, tile movement)
    → Find radicals as loot
    → Combine radicals at Forge workbenches (氵 + 每 = 海)
    → Battle enemies by typing their pinyin
    → Use forged characters as combat spells
    → Descend to next floor
    → Boss every 5 floors
Die → keep Recipe Book → restart smarter
```

---

## Game Systems

### A. Radical Forge (the "Alchemist" half)

The signature mechanic. Forging teaches **character composition** — understanding radicals makes memorizing thousands of characters tractable.

- **Workbenches** appear in special forge rooms (1–2 per floor)
- Player selects 2–3 radicals from inventory → place on slots → attempt combination
- ✅ Valid combo → character forged + particle burst + added to spell inventory
- ❌ Invalid combo → hint system ("this needs a water radical…", "try fewer strokes")
- **Recipe Book** persists across runs (localStorage) — discovered combos remembered forever
- Undiscovered recipes shown as `???` silhouettes with radical count hints

**Starter radicals** (~20): 人 口 木 水 火 土 日 月 山 石 金 手 心 目 言 門 食 雨 女 子

**Example recipes**:
| Radicals | Result | Meaning | Spell Effect |
|----------|--------|---------|-------------|
| 木 + 木 | 林 | forest | Summon tree barrier (block corridor) |
| 氵 + 每 | 海 | sea | Flood room (damage all enemies) |
| 火 + 山 | 炭 | charcoal | Burn trail (damage over time) |
| 人 + 言 | 信 | trust | Pacify enemy (skip fight) |
| 金 + 同 | 銅 | copper | Reinforce armor (+defense) |
| 心 + 生 | 性 | nature | Reveal floor map |
| 日 + 月 | 明 | bright | Dispel fog of war in radius |

**Long-term content**: ~214 Kangxi radicals, 500+ forgeable characters. Start with 100 common ones, unlock radical sets as you reach deeper floors.

### B. Combat System (the "Roguelike" half)

Turn-based with timer pressure. Combines vocab drilling with tactical decisions.

**Basic attack**: Enemy Hanzi appears → type its pinyin within 3 seconds → deal damage
- Correct on first try = critical hit (2× damage)
- Correct on retry = normal damage (costs a turn)
- Timeout = enemy attacks you

**Spell attacks**: Use a forged character from inventory → special effect based on meaning:
- 🔥 Fire chars (火, 炎, 焰) = area damage
- 💧 Water chars (水, 海, 河) = slow enemies / extinguish fire
- 🛡️ Defense chars (盾, 甲, 壁) = block next attack
- 💚 Life chars (心, 生, 命) = heal HP
- 👁️ Vision chars (目, 見, 明) = reveal map / enemy info
- ⚡ Power chars (力, 電, 雷) = stun enemy

**Enemy types**:
| Type | Example | Challenge |
|------|---------|-----------|
| Common | 大 (dà) | Single character, 3s timer |
| Elite | 朋友 (péngyǒu) | Multi-char word, 4s timer, more HP |
| Mimic | Looks like loot, attacks when approached | Surprise typing test |
| Boss | Full sentence completion | Multi-phase, rapid recall |

### C. Dungeon Generation

Procedural floors keep each run fresh.

- **Algorithm**: BSP tree splits room space → connect rooms with corridors
- **5–8 rooms per floor**, connected by 1-tile-wide corridors
- **Room types**:
  - 🗡️ **Enemy room** — 1–3 enemies to fight
  - 🔨 **Forge room** — workbench + radical loot
  - 💎 **Treasure room** — equipment or gold
  - 🏪 **Shop room** — buy radicals, equipment, HP potions
  - 🚪 **Stairs room** — descend to next floor
  - 👹 **Boss room** — every 5th floor
- **Fog of war**: Only reveal tiles in player's line-of-sight (simple raycasting)
- **Floor themes** (tied to HSK difficulty):
  - Floors 1–5: Bamboo Forest (HSK1 vocab — 你好, 大小, 学生)
  - Floors 6–10: Stone Cave (HSK2 — 已经, 虽然, 环境)
  - Floors 11–15: Ancient Temple (HSK3 — 经济, 政治, 社会)
  - Floors 16+: Sky Palace (HSK4+ — advanced characters)

### D. Spaced Repetition Engine (SRS)

The roguelike loop IS the SRS. No flashcard UI needed — the game adapts organically.

- Track per-character stats: `{ seen: N, correct: N, last_seen: timestamp }`
- Characters with accuracy <70% get **3× spawn weight** as enemies
- Characters not seen in 5+ floors get **2× spawn weight** (prevent forgetting)
- New characters introduced gradually (max 5 new per floor)
- Boss fights = "exam" — rapid recall of all characters from current theme

### E. Equipment System

Lightweight — 3 slots, meaningful choices.

**Weapon** (affects typing/damage):
- *Brush of Clarity* — first pinyin attempt always crits
- *Jade Blade* — +30% damage, but timer reduced to 2s
- *Scholar's Pen* — hints show tone number on enemies

**Armor** (affects survivability):
- *Bamboo Mail* — block 1 hit per room
- *Silk Robe* — +2 max HP
- *Dragon Scale* — reflect 50% damage on timeout

**Charm** (passive effects):
- *Radical Magnet* — enemies drop radicals 2× more often
- *Memory Crystal* — +50% SRS accuracy weight (learn faster)
- *Merchant's Coin* — double gold drops

---

## Player Progression

### Per-Run (lost on death)
- Current HP, gold, equipped items
- Radical inventory (unforged)
- Spell inventory (forged characters)
- Current floor depth

### Persistent (kept forever via localStorage)
- **Recipe Book** — all discovered radical→character combos
- **Bestiary** — all encountered enemy characters + your accuracy stats
- **High scores** — deepest floor reached, most characters forged, best accuracy
- **Unlocked radical sets** — deeper runs unlock rarer radicals for future runs
- **Achievement badges** — "Forge 50 characters", "Clear floor 10", "100% accuracy on a boss"

---

## Architecture

### Module Structure
```
src/
  lib.rs                  — WASM entry point, init
  dungeon/
    mod.rs                — DungeonState, main game loop, tick
    generation.rs         — BSP floor generation
    room.rs               — Room types, layout, contents
    fov.rs                — Fog of war / line of sight
  combat/
    mod.rs                — Turn-based combat state machine
    enemy.rs              — Enemy types, AI, spawn weighting
    spells.rs             — Forged character spell effects
  forge/
    mod.rs                — Forge UI logic, recipe validation
    recipes.rs            — Radical → character mappings (build-time generated)
    inventory.rs          — Radical + spell inventory management
  player.rs               — Player state, HP, equipment, movement
  srs.rs                  — Spaced repetition tracker
  render.rs               — All canvas 2D rendering
  audio.rs                — Procedural Web Audio (reuse patterns from hanzi-cat)
  persist.rs              — localStorage save/load (recipe book, stats)
  data/
    radicals.json         — 214 Kangxi radicals + metadata
    recipes.json          — Valid radical combinations
    vocab.json            — Characters with pinyin, meaning, HSK level
```

### Data Pipeline (build.rs)
- Parse JSON data files at compile time
- Generate static Rust arrays (zero runtime cost)
- Include HSK level tags for floor theming
- Validate recipe consistency (no impossible combos)

### Rendering Layers (Canvas 2D, bottom to top)
1. Floor tiles (dungeon walls/ground tileset)
2. Fog of war overlay (alpha mask on unrevealed tiles)
3. Room decorations (workbenches, stairs, chests)
4. Entities (player sprite, enemies, items)
5. Combat UI (enemy display, pinyin input, timer bar, damage numbers)
6. Forge UI (modal overlay — radical slots, recipe book)
7. HUD (HP bar, minimap, inventory icons, gold count)

---

## Implementation Phases

### Phase 1: Core Foundation
- Project scaffolding (Cargo.toml, wasm-bindgen, index.html)
- BSP dungeon generation (rooms + corridors + walls)
- Player tile movement (arrow keys / WASD)
- Fog of war (simple LOS)
- Canvas rendering: floor tiles, player sprite, minimap
- **Milestone**: Walk around a procedurally generated dungeon

### Phase 2: Combat & Vocabulary
- Enemy placement in rooms
- Turn-based combat state machine (approach → type → resolve)
- Pinyin input + validation (reuse hanzi-cat's dictionary approach)
- HP system for player and enemies
- Basic enemy AI (aggro on sight, chase)
- Game over / restart flow
- Vocab data pipeline (build.rs)
- **Milestone**: Fight Hanzi enemies by typing their pinyin

### Phase 3: Radical Forge
- Radical item drops from enemies
- Inventory system (radicals + forged spells)
- Forge workbench interaction (room type)
- Recipe validation engine
- Recipe book UI (persistent via localStorage)
- Spell usage in combat (apply effects)
- **Milestone**: Full forge loop — find radicals, combine, use as spells

### Phase 4: Progression & SRS
- Multi-floor descent (stairs, floor counter)
- HSK-themed floor difficulty scaling
- Spaced repetition engine (accuracy tracking, spawn weighting)
- Equipment system (3 slots, loot tables)
- Shop rooms (buy with gold)
- Boss encounters (every 5 floors, multi-phase)
- **Milestone**: Complete roguelike loop with adaptive difficulty

### Phase 5: Polish & Content
- Pixel art tileset + character sprites
- Procedural audio (forge sounds, combat, ambient)
- Tutorial floor (guided first run)
- Full content: 200+ forgeable chars, 50+ equipment items
- Achievement system
- Stats / high score screen
- Mobile touch support
- **Milestone**: Release-ready game

---

## Visual Style Notes

- **Palette**: Dark dungeon tones + vibrant character effects (gold for Hanzi, red for fire spells, blue for water)
- **Pixel art**: 16×16 or 32×32 tiles, consistent with hanzi-cat's aesthetic
- **Hanzi rendering**: Large, crisp Chinese characters floating above enemy sprites — the characters ARE the monsters
- **Forge effects**: Radicals glow and orbit the workbench, combine in a flash of light
- **Screen shake** on critical hits, particle trails on spells

---

## Open Questions
1. **Language scope**: Chinese only, or also support Japanese kanji / Korean hanja?
2. **Multiplayer**: Co-op space station runs? Competitive leaderboards?
3. **Mobile**: Touch controls from Phase 1, or add later?
4. **Monetization**: Free + open source (like hanzi-cat)? Or premium content?
5. **Name**: "Radical Starfinder"? "Forge & Foe"? "漢字地牢" (Hànzì Dìláo)?
