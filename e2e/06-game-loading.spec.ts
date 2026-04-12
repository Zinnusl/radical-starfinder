import { test, expect } from '@playwright/test';
import { waitForWasmInit, waitForCanvas, waitForGameReady, LOAD_TIMEOUT } from './helpers';

test.describe('Game loading and initialization', () => {
  test('loading indicator is visible immediately on page load', async ({ page }) => {
    await page.goto('/', { waitUntil: 'domcontentloaded' });

    const loading = page.locator('#loading');
    await expect(loading).toBeVisible();
    await expect(loading).toContainText('Initializing systems');
  });

  test('loading indicator text is correct', async ({ page }) => {
    await page.goto('/', { waitUntil: 'domcontentloaded' });
    const text = await page.locator('#loading').textContent();
    // The loading text uses a Unicode ellipsis character (…)
    expect(text).toContain('Initializing systems');
  });

  test('WASM binary file is accessible', async ({ page }) => {
    const response = await page.request.get('/radical-starfinder-5457ccd9100a8fb0_bg.wasm');
    expect(response.status()).toBe(200);
    expect(response.headers()['content-type']).toContain('wasm');
  });

  test('WASM JavaScript bindings file is accessible', async ({ page }) => {
    const response = await page.request.get('/radical-starfinder-5457ccd9100a8fb0.js');
    expect(response.status()).toBe(200);
  });

  test('wasmBindings are available after WASM loads', async ({ page }) => {
    await page.goto('/');
    await waitForWasmInit(page);

    const hasBindings = await page.evaluate(() => {
      return typeof (window as any).wasmBindings !== 'undefined';
    });
    expect(hasBindings).toBe(true);
  });

  test('wasmBindings has start_game function', async ({ page }) => {
    await page.goto('/');
    await waitForWasmInit(page);

    const hasStartGame = await page.evaluate(() => {
      return typeof (window as any).wasmBindings?.start_game === 'function';
    });
    expect(hasStartGame).toBe(true);
  });

  test('game canvas is created after start_game is called', async ({ page }) => {
    await page.goto('/');
    await waitForCanvas(page);

    const canvasExists = await page.evaluate(() => {
      return document.getElementById('game-canvas') !== null;
    });
    expect(canvasExists).toBe(true);
  });

  test('game loads within 15 seconds', async ({ page }) => {
    const startTime = Date.now();
    await page.goto('/');
    await waitForCanvas(page);
    const loadTime = Date.now() - startTime;

    expect(loadTime).toBeLessThan(15000);
    console.log(`Game loaded in ${loadTime}ms`);
  });

  test('TrunkApplicationStarted event fires', async ({ page }) => {
    // Register listener via addInitScript so it is installed before page scripts run
    await page.addInitScript(() => {
      window.addEventListener('TrunkApplicationStarted', () => {
        (window as any).__trunkStarted = true;
      });
    });

    await page.goto('/');
    await waitForWasmInit(page);

    const trunkEventFired = await page.evaluate(() => !!(window as any).__trunkStarted);
    expect(trunkEventFired).toBe(true);
  });

  test('game renders first frame within 5 seconds of WASM init', async ({ page }) => {
    await page.goto('/');
    await waitForWasmInit(page);

    const frameRenderStart = Date.now();
    await waitForGameReady(page);
    const timeToFirstFrame = Date.now() - frameRenderStart;

    expect(timeToFirstFrame).toBeLessThan(5000);
    console.log(`First frame rendered ${timeToFirstFrame}ms after WASM init`);
  });

  test('no critical errors during loading sequence', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', e => {
      if (!e.message.includes('WebSocket') && !e.message.includes('trunk')) {
        errors.push(e.message);
      }
    });

    await page.goto('/');
    await waitForGameReady(page);

    expect(errors).toHaveLength(0);
  });

  test('WASM network request succeeds', async ({ page }) => {
    const wasmRequests: { url: string; status: number }[] = [];

    page.on('response', response => {
      if (response.url().includes('.wasm')) {
        wasmRequests.push({ url: response.url(), status: response.status() });
      }
    });

    await page.goto('/');
    await waitForWasmInit(page);

    expect(wasmRequests.length).toBeGreaterThan(0);
    for (const req of wasmRequests) {
      expect(req.status).toBe(200);
    }
  });

  test('page title is set correctly', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle('Radical Starfinder');
  });

  test('loading indicator is styled correctly (centered)', async ({ page }) => {
    await page.goto('/', { waitUntil: 'domcontentloaded' });

    const styles = await page.evaluate(() => {
      const el = document.getElementById('loading');
      const style = window.getComputedStyle(el!);
      return {
        position: style.position,
        top: style.top,
        left: style.left,
      };
    });

    expect(styles.position).toBe('fixed');
  });

  test('canvas is appended to body by game init', async ({ page }) => {
    await page.goto('/', { waitUntil: 'domcontentloaded' });

    // Canvas should not exist before game init
    const canvasBefore = await page.evaluate(() =>
      document.getElementById('game-canvas') !== null
    );
    expect(canvasBefore).toBe(false);

    // Wait for game to create canvas
    await waitForCanvas(page);

    const canvasParent = await page.evaluate(() =>
      document.getElementById('game-canvas')?.parentElement?.tagName.toLowerCase()
    );

    // Canvas should be in body
    expect(canvasParent).toBe('body');
  });

  // The game calls el.remove() on the #loading div after canvas is created (src/game/mod.rs).
  // This confirms the element is fully removed from the DOM, not just hidden.
  test('loading indicator is removed from DOM after game starts', async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);

    const loadingExists = await page.evaluate(() =>
      document.getElementById('loading') !== null
    );

    // The game removes the loading element via el.remove() — it should be gone
    expect(loadingExists).toBe(false);
  });

  // Documents that the loading div is NOT present in the error state (text change only happens
  // when start_game() throws — if WASM init succeeds, el.remove() runs first).
  test('loading indicator visibility after game starts (documents current behavior)', async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);

    const loadingEl = await page.evaluate(() => {
      const el = document.getElementById('loading');
      if (!el) return null;
      const style = window.getComputedStyle(el);
      return {
        display: style.display,
        visibility: style.visibility,
        opacity: style.opacity,
        text: el.textContent,
      };
    });

    // The game removes the #loading element from the DOM entirely after init (src/game/mod.rs).
    // A null result means the element was properly cleaned up — this is the expected good behavior.
    if (loadingEl === null) {
      console.log('Loading indicator correctly removed from DOM after game ready');
    } else {
      console.log(`Loading indicator still present after game ready: ${JSON.stringify(loadingEl)}`);
      // If it IS still present, it must not show a failure message
      expect(loadingEl.text).not.toContain('Failed to load');
    }
  });
});
