---
name: end-to-end-testing
description: Playwright E2E testing guide for Radical Starfinder — a Rust/WASM browser-based Chinese-language roguelike. Use this when writing, debugging, or extending the Playwright test suite in the e2e/ directory.
---

# E2E Testing: Radical Starfinder

This skill captures everything learned from running 20 parallel Playwright agents testing this WASM game. Follow these patterns to avoid the failure modes that caused agents to iterate 3–5 times before passing.

---

## Game Architecture

- The game is compiled to WASM and served from `dist/`. Pre-built output is in `dist/`.
- `dist/index.html` is the entry point. It contains two `<script type="module">` blocks:
  1. Touch controls + service worker registration
  2. Trunk WASM loader — imports the JS bindings, inits WASM, sets `window.wasmBindings`, fires `TrunkApplicationStarted`
- `#game-canvas` does **not** exist in the HTML — it is created dynamically by Rust when `start_game()` runs. Always use `waitForSelector('#game-canvas')`.
- The `#loading` div is fully **removed** from the DOM after the game starts (not just hidden).
- WASM cold-start: ~2 minutes on first load (downloads + compiles). Subsequent loads: ~28–60s.

---

## Critical: Test Timeouts

All tests that load the game need long timeouts. Use these in `playwright.config.ts`:

```ts
use: { actionTimeout: 10_000 },
timeout: 120_000,   // minimum; use 180_000 for tests that trigger combat or death
```

The WASM binary takes 30–60s to load per test in `beforeEach`. The original 45s timeout was too short for almost every gameplay test.

---

## Critical: Key Event Target

**Always dispatch key events to `document`, not to `#game-canvas`.**

The WASM game attaches its `keydown` listener to `document` via `doc.add_event_listener_with_callback`. Sending events to the canvas element silently does nothing.

```ts
await page.evaluate((key) => {
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key, bubbles: true, cancelable: true
  }));
}, 'ArrowRight');
```

---

## Critical: Class-Select Overlay

The game starts with `show_class_select: true`. This overlay **intercepts all key input** except ArrowUp/ArrowDown/Enter. Before any gameplay test, dismiss it:

```ts
async function setupGame(page: Page) {
  await waitForGameReady(page);         // wait for #game-canvas
  await page.waitForTimeout(500);       // let game fully initialise
  await sendKey(page, 'Enter');         // dismiss class-select overlay
  await page.waitForTimeout(300);
}
```

Without this, movement keys, codex, starmap, inventory, and console tests will all silently fail.

---

## Game State Navigation

Different keys only work in specific game states:

| Key | Works in | Notes |
|-----|----------|-------|
| `ArrowUp/Down` | ClassSelect + ShipInterior + all | Select class or move |
| `ArrowLeft/Right` | ShipInterior, LocationExploration | Move player |
| `s` | Starmap | Enter ShipInterior |
| `m` | ShipInterior | Open Starmap |
| `i` | LocationExploration, GroundCombat | Open Inventory — **not ShipInterior** |
| `c` | LocationExploration | Open Codex |
| `q` | All gameplay | Cycle ability/spell |
| `Space` | All gameplay | Cast selected spell |
| `t` | All gameplay | Toggle skill tree overlay |
| `` ` `` | All gameplay | Open developer console |
| `Escape` | All states | Close overlay / open settings |
| `r`/`R` | Death screen | Restart game |

To enter LocationExploration from a fresh load:

```ts
async function enterExploreMode(page: Page) {
  await setupGame(page);        // dismiss class select
  // Game starts in ShipInterior after class select
  // Press 's' to go to Starmap, then navigate to a location
}
```

To enter ShipInterior (for movement tests):

```ts
await sendKey(page, 's');       // from Starmap → ShipInterior
await page.waitForTimeout(500);
```

---

## Canvas Snapshot Performance

**Do not transfer the full pixel array over CDP.** A full `1280×720` canvas snapshot transfers ~960KB per call, adding ~200ms latency per probe. This causes tests to timeout or take 6+ minutes.

**Use a browser-side hash instead:**

```ts
async function snapHash(page: Page): Promise<number> {
  return page.evaluate(() => {
    const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
    const ctx = canvas.getContext('2d')!;
    const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
    let h = 0;
    for (let i = 0; i < data.length; i += 4) {
      h = (Math.imul(31, h) + data[i] + data[i+1] + data[i+2]) | 0;
    }
    return h;
  });
}

// Detect any canvas change:
const before = await snapHash(page);
await sendKey(page, 'ArrowRight');
await page.waitForTimeout(500);
const after = await snapHash(page);
expect(before).not.toBe(after);
```

For pixel-count diffs (to verify a minimum number of changed pixels), keep computation browser-side:

```ts
async function countChangedPixels(page: Page, snapA: number[], snapB: number[]): Promise<number> {
  // Pass both arrays, compute diff in browser — or use a 400×300 bounded region
}
```

---

## Canvas Crop Warning

The player renders at approximately **(625, 530)** in a **1280×720** canvas. The top-left 400×300 region captures the HUD but **misses the player entirely**. Codex/starmap/overlays render centered on screen.

- For movement detection: use a full-canvas hash (browser-side)
- For HUD changes (HP, gold): a 400×300 top-left crop is fine
- For overlays (codex, starmap, inventory): use full-canvas or center-crop

---

## Player Spawn Position Notes

- Player starts at approximately `y=16` in the dungeon, with a wall at `y=17` (south)
- ArrowDown / `s` cannot move south immediately — move north first to create room
- ArrowRight is always available on spawn; Left/Up/Down depend on room layout

---

## Console Commands

Open with backtick (`` ` ``). Close with `Escape` after each command — the console stays open after pressing Enter.

```ts
async function runCmd(page: Page, cmd: string) {
  await sendKey(page, '`');
  await page.waitForTimeout(200);
  await page.keyboard.type(cmd);
  await sendKey(page, 'Enter');
  await page.waitForTimeout(300);
  await sendKey(page, 'Escape');  // ← essential: close console
  await page.waitForTimeout(200);
}
```

Available commands:
```
god          — toggle god mode (invincible)
hp N         — set HP to N
gold N       — set gold to N
floor N      — jump to floor N
reveal       — reveal all tiles on current floor
fight normal/elite/boss  — spawn and enter combat
kill_all     — kill all enemies on floor
give_item <NAME>         — e.g. give_item HealthPotion
give_radical <汉字>      — e.g. give_radical 火
give_spell <汉字>        — e.g. give_spell 明
stats        — show stats overlay
items        — list items
help         — list commands
clear        — clear console output
```

---

## Combat Mechanics

Combat is **tactical**, not bump-attack. The sequence to land an attack:

1. Use `fight normal` console command (or walk into an enemy)
2. Press `w` 6 times to pass turns — the enemy AI closes the distance (~400ms/turn)
3. Press `a` to enter targeting mode (auto-selects adjacent enemy)
4. Press `Enter` to confirm target and enter pinyin input mode
5. Type the pinyin for the displayed Chinese character + `Enter` to attack

Key pixel diff values measured during combat:
- Enemy spawns: ~217k pixel diff
- `kill_all`: ~152k pixel diff
- Full combat resolution: ~298–307k pixel diff
- God mode on: ~14k diff; god mode off: ~638k diff

---

## Pinyin Input

- Single characters (e.g. `r`) buffer silently — no visual change until a full syllable
- Full syllable `ren` → **55,594 pixel diff** (visible in combat UI)
- `Backspace` — clears buffer, may not cause pixel change in sampled region
- `Enter` to submit → **25,096 pixel diff** (combat result renders)
- Numbers and invalid input are handled gracefully (no crash)

---

## Inventory

- Key: `i` — only works in `LocationExploration` or `GroundCombat` state
- Navigate: `ArrowUp`/`ArrowDown`
- Inspect item: `Enter` (opens inspect view, does not use item)
- Use item: hotkeys `1`–`5` from main game view
- Craft: press `c` in inventory (needs ≥2 items), then `ArrowDown`/`Enter`
- Close: `Escape` or `i` again

---

## Death & Game Over

- No `die` console command exists. To trigger death: `hp 1` → `fight normal` → pass turns
- Phoenix/Undying abilities may auto-revive on fresh runs — use god mode off + `hp 1`
- Death screen text: `☠ You died on floor X! ... — Press R to restart`
- Restart key: `r` or `R`
- Death screen pixel diff vs gameplay: ~307k pixels

---

## Serving `dist/` in Worktrees

The `dist/` directory is gitignored. In a git worktree, create an NTFS junction:

```bat
cmd /c mklink /J dist ..\radical-starfinder\dist
```

Then use `npx serve dist -l PORT` as the web server in `playwright.config.ts`.

---

## Playwright Config Template

```ts
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 120_000,
  use: {
    baseURL: 'http://localhost:PORT',
    actionTimeout: 10_000,
  },
  webServer: {
    command: 'npx serve dist -l PORT --no-clipboard',
    url: 'http://localhost:PORT',
    reuseExistingServer: true,
    timeout: 30_000,
  },
});
```

---

## Bugs Fixed During Testing

| Bug | Fix |
|-----|-----|
| `dist/index.html` contained unresolved Trunk HMR template vars (`{{__TRUNK_ADDRESS__}}`) — dev artifact in production file | Removed the WebSocket reconnection `<script>` block |
| `dist/index.html` had duplicate WASM init (old `pkg/` import + Trunk loader) | Removed redundant try/catch import |
| `manifest.json` and `sw.js` missing from `dist/` — PWA broken | Copied both files from project root to `dist/` |
| Console `runCmd` helper left console open after Enter (each subsequent command typed into wrong field) | Send `Escape` after every command |

---

## Test Suite Structure

The complete suite lives on branches `e2e/worker-1` through `e2e/worker-20`, covering:
`01-smoke`, `02-canvas-rendering`, `03-keyboard-input`, `04-touch-controls`, `05-pwa-features`, `06-game-loading`, `07-error-handling`, `08-ui-states`, `09-visual-regression`, `10-integration`, `11-player-movement`, `12-combat`, `13-hp-system`, `14-abilities`, `15-floor-progression`, `16-death-gameover`, `17-pinyin-input`, `18-console-commands`, `19-inventory`, `20-full-session`.

**178 total tests, all passing.**
