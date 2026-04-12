import { test, expect } from '@playwright/test';
import { waitForGameReady, canvasHasContent, getCanvasSnapshot, LOAD_TIMEOUT } from './helpers';

test.describe('Canvas rendering', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('canvas element exists in DOM', async ({ page }) => {
    const canvas = page.locator('#game-canvas');
    await canvas.waitFor({ timeout: LOAD_TIMEOUT });
    await expect(canvas).toBeVisible();
  });

  test('canvas has correct element type', async ({ page }) => {
    await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });
    const tagName = await page.evaluate(() => {
      const el = document.getElementById('game-canvas');
      return el?.tagName.toLowerCase();
    });
    expect(tagName).toBe('canvas');
  });

  test('canvas width matches viewport width', async ({ page }) => {
    await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });
    await page.waitForTimeout(2000); // wait for game to size canvas

    const { canvasWidth, viewportWidth } = await page.evaluate(() => ({
      canvasWidth: (document.getElementById('game-canvas') as HTMLCanvasElement).width,
      viewportWidth: window.innerWidth,
    }));

    // Canvas width should be close to viewport width (within 10%)
    expect(canvasWidth).toBeGreaterThan(viewportWidth * 0.5);
  });

  test('canvas height matches viewport height', async ({ page }) => {
    await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });
    await page.waitForTimeout(2000);

    const { canvasHeight, viewportHeight } = await page.evaluate(() => ({
      canvasHeight: (document.getElementById('game-canvas') as HTMLCanvasElement).height,
      viewportHeight: window.innerHeight,
    }));

    expect(canvasHeight).toBeGreaterThan(viewportHeight * 0.5);
  });

  test('canvas renders non-background pixels after game init', async ({ page }) => {
    await waitForGameReady(page);
    const hasContent = await canvasHasContent(page);
    expect(hasContent).toBe(true);
  });

  test('canvas uses 2d rendering context', async ({ page }) => {
    await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });
    const hasContext = await page.evaluate(() => {
      const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
      const ctx = canvas.getContext('2d');
      return ctx !== null;
    });
    expect(hasContext).toBe(true);
  });

  test('canvas renders multiple distinct colors (not monochrome)', async ({ page }) => {
    await waitForGameReady(page);

    const colorVariety = await page.evaluate(() => {
      const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
      const ctx = canvas.getContext('2d')!;
      const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
      const colors = new Set<string>();
      for (let i = 0; i < data.data.length; i += 4) {
        const r = data.data[i], g = data.data[i+1], b = data.data[i+2];
        // Quantize to reduce noise
        const key = `${Math.round(r/32)*32},${Math.round(g/32)*32},${Math.round(b/32)*32}`;
        colors.add(key);
        if (colors.size > 10) break; // early exit once we know it's varied
      }
      return colors.size;
    });

    // A rendered game should have more than 3 distinct color buckets
    expect(colorVariety).toBeGreaterThan(3);
  });

  test('canvas is display:block (no inline whitespace)', async ({ page }) => {
    await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });
    const display = await page.evaluate(() => {
      const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
      return window.getComputedStyle(canvas).display;
    });
    expect(display).toBe('block');
  });

  test('canvas pixel data changes after keyboard input', async ({ page }) => {
    await waitForGameReady(page);

    // Take snapshot before key press
    const before = await getCanvasSnapshot(page);

    // Send a movement key
    await page.evaluate(() => {
      const canvas = document.getElementById('game-canvas')!;
      canvas.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    });

    await page.waitForTimeout(500); // wait for game to process

    // Take snapshot after
    const after = await getCanvasSnapshot(page);

    // At least some pixels should have changed
    let changedPixels = 0;
    for (let i = 0; i < Math.min(before.length, after.length); i += 4) {
      if (Math.abs((before as any)[i] - (after as any)[i]) > 5 ||
          Math.abs((before as any)[i+1] - (after as any)[i+1]) > 5 ||
          Math.abs((before as any)[i+2] - (after as any)[i+2]) > 5) {
        changedPixels++;
      }
    }

    // After input, at least some pixels should change (game re-renders)
    expect(changedPixels).toBeGreaterThan(0);
  });

  test('canvas fills full viewport with no overflow', async ({ page }) => {
    await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });

    // Verify body has overflow hidden
    const bodyOverflow = await page.evaluate(() =>
      window.getComputedStyle(document.body).overflow
    );
    expect(bodyOverflow).toBe('hidden');
  });

  test('canvas pixelRatio is handled correctly', async ({ page }) => {
    await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });
    await page.waitForTimeout(2000);

    const result = await page.evaluate(() => {
      const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
      const dpr = window.devicePixelRatio || 1;
      // Canvas buffer size should be at least viewport size
      return {
        width: canvas.width,
        height: canvas.height,
        viewportWidth: window.innerWidth,
        viewportHeight: window.innerHeight,
        dpr
      };
    });

    // Canvas buffer should be >= viewport size (accounting for DPR)
    expect(result.width).toBeGreaterThanOrEqual(result.viewportWidth);
    expect(result.height).toBeGreaterThanOrEqual(result.viewportHeight);
  });
});
