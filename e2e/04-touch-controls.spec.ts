import { test, expect } from '@playwright/test';
import { waitForGameReady, LOAD_TIMEOUT } from './helpers';

test.describe('Touch controls - desktop (hidden)', () => {
  test.use({ viewport: { width: 1280, height: 720 } });

  test('touch controls are hidden on desktop viewport', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });

    const display = await page.evaluate(() => {
      const el = document.getElementById('touch-controls');
      return window.getComputedStyle(el!).display;
    });
    expect(display).toBe('none');
  });
});

test.describe('Touch controls - mobile viewport (visible)', () => {
  test.use({ viewport: { width: 375, height: 667 } }); // iPhone SE size, max-width:800px triggers show

  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });
  });

  test('touch controls container exists in DOM', async ({ page }) => {
    const el = page.locator('#touch-controls');
    await expect(el).toBeAttached();
  });

  test('d-pad buttons exist', async ({ page }) => {
    await expect(page.locator('#btn-up')).toBeAttached();
    await expect(page.locator('#btn-down')).toBeAttached();
    await expect(page.locator('#btn-left')).toBeAttached();
    await expect(page.locator('#btn-right')).toBeAttached();
  });

  test('action buttons exist', async ({ page }) => {
    const actionBtns = page.locator('.action-btn');
    const count = await actionBtns.count();
    expect(count).toBeGreaterThanOrEqual(4); // Enter, Esc, Spell, Q
  });

  test('d-pad up button has correct data-key attribute', async ({ page }) => {
    await expect(page.locator('#btn-up')).toHaveAttribute('data-key', 'ArrowUp');
  });

  test('d-pad down button has correct data-key attribute', async ({ page }) => {
    await expect(page.locator('#btn-down')).toHaveAttribute('data-key', 'ArrowDown');
  });

  test('d-pad left button has correct data-key attribute', async ({ page }) => {
    await expect(page.locator('#btn-left')).toHaveAttribute('data-key', 'ArrowLeft');
  });

  test('d-pad right button has correct data-key attribute', async ({ page }) => {
    await expect(page.locator('#btn-right')).toHaveAttribute('data-key', 'ArrowRight');
  });

  test('clicking up button dispatches ArrowUp keydown event', async ({ page }) => {
    await waitForGameReady(page);

    await page.evaluate(() => {
      (window as any).__testKeys = [];
      document.addEventListener('keydown', (e) => {
        (window as any).__testKeys.push(e.key);
      }, true);
    });

    await page.locator('#btn-up').click();
    await page.waitForTimeout(300);

    const keys = await page.evaluate(() => (window as any).__testKeys || []);
    expect(keys).toContain('ArrowUp');
  });

  test('clicking right button dispatches ArrowRight keydown event', async ({ page }) => {
    await waitForGameReady(page);

    await page.evaluate(() => {
      (window as any).__testKeys = [];
      document.addEventListener('keydown', (e) => {
        (window as any).__testKeys.push(e.key);
      }, true);
    });

    await page.locator('#btn-right').click();
    await page.waitForTimeout(300);

    const keys = await page.evaluate(() => (window as any).__testKeys || []);
    expect(keys).toContain('ArrowRight');
  });

  test('Enter action button dispatches Enter keydown event', async ({ page }) => {
    await waitForGameReady(page);

    await page.evaluate(() => {
      (window as any).__testKeys = [];
      document.addEventListener('keydown', (e) => {
        (window as any).__testKeys.push(e.key);
      }, true);
    });

    await page.locator('.action-btn[data-key="Enter"]').click();
    await page.waitForTimeout(300);

    const keys = await page.evaluate(() => (window as any).__testKeys || []);
    expect(keys).toContain('Enter');
  });

  test('Escape action button dispatches Escape keydown event', async ({ page }) => {
    await waitForGameReady(page);

    await page.evaluate(() => {
      (window as any).__testKeys = [];
      document.addEventListener('keydown', (e) => {
        (window as any).__testKeys.push(e.key);
      }, true);
    });

    await page.locator('.action-btn[data-key="Escape"]').click();
    await page.waitForTimeout(300);

    const keys = await page.evaluate(() => (window as any).__testKeys || []);
    expect(keys).toContain('Escape');
  });

  test('touch controls are positioned at bottom of viewport', async ({ page }) => {
    const controls = page.locator('#touch-controls');
    const box = await controls.boundingBox();
    const viewport = page.viewportSize()!;

    if (box) {
      // Touch controls bottom should be near viewport bottom
      const controlsBottom = box.y + box.height;
      expect(controlsBottom).toBeGreaterThan(viewport.height * 0.7);
    }
  });

  test('dpad center wait button exists', async ({ page }) => {
    await expect(page.locator('#btn-wait')).toBeAttached();
  });

  test('spell action button exists with space key', async ({ page }) => {
    const spellBtn = page.locator('.action-btn[data-key=" "]');
    await expect(spellBtn).toBeAttached();
    await expect(spellBtn).toContainText('Spell');
  });

  test('Q action button exists', async ({ page }) => {
    await expect(page.locator('.action-btn[data-key="q"]')).toBeAttached();
  });
});

test.describe('Touch controls - narrow viewport', () => {
  test.use({ viewport: { width: 500, height: 800 } }); // width <= 800px triggers touch controls

  test('touch controls show on narrow desktop viewport', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('#touch-controls', { timeout: LOAD_TIMEOUT });

    const display = await page.evaluate(() => {
      const el = document.getElementById('touch-controls');
      return window.getComputedStyle(el!).display;
    });
    // Should be flex (visible) on narrow viewport
    expect(display).toBe('flex');
  });
});
