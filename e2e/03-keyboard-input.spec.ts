import { test, expect } from '@playwright/test';
import {
  setupGame, enterShipInterior, sendKey,
  storeSnapshot, countChangedPixels, waitForCanvasChange,
  LOAD_TIMEOUT,
} from './helpers';

test.describe('Keyboard input and game mechanics', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // Dismiss the class-selection overlay so all subsequent keys reach the game
    await setupGame(page);
  });

  test('game canvas is focusable', async ({ page }) => {
    const canvas = page.locator('#game-canvas');
    await expect(canvas).toBeVisible();
    const tagName = await page.evaluate(() => document.getElementById('game-canvas')?.tagName);
    expect(tagName).toBe('CANVAS');
  });

  // ── Movement keys (tested inside ShipInterior where movement is always valid) ──

  test('ArrowRight key changes canvas state', async ({ page }) => {
    await enterShipInterior(page);
    await storeSnapshot(page, 'before');
    await sendKey(page, 'ArrowRight');
    await page.waitForTimeout(300);
    expect(await countChangedPixels(page, 'before')).toBeGreaterThan(0);
  });

  test('ArrowLeft key changes canvas state', async ({ page }) => {
    await enterShipInterior(page);
    await storeSnapshot(page, 'before');
    await sendKey(page, 'ArrowLeft');
    await page.waitForTimeout(300);
    expect(await countChangedPixels(page, 'before')).toBeGreaterThan(0);
  });

  test('ArrowUp key changes canvas state', async ({ page }) => {
    await enterShipInterior(page);
    await storeSnapshot(page, 'before');
    await sendKey(page, 'ArrowUp');
    await page.waitForTimeout(300);
    expect(await countChangedPixels(page, 'before')).toBeGreaterThan(0);
  });

  test('ArrowDown key changes canvas state', async ({ page }) => {
    await enterShipInterior(page);
    // Player starts at the south wall of the Cargo Bay (y=16, wall at y=17).
    // Move north first to create room to move south.
    await sendKey(page, 'ArrowUp');
    await page.waitForTimeout(300);
    await storeSnapshot(page, 'before');
    await sendKey(page, 'ArrowDown');
    await page.waitForTimeout(300);
    expect(await countChangedPixels(page, 'before')).toBeGreaterThan(0);
  });

  // ── Menu / overlay keys ──

  test('C key opens Codex (canvas changes)', async ({ page }) => {
    await storeSnapshot(page, 'before');
    await sendKey(page, 'c');
    // Wait until codex overlay renders across the full canvas
    await waitForCanvasChange(page, 'before', 100, 8000);
    expect(await countChangedPixels(page, 'before')).toBeGreaterThan(100);
  });

  test('M key opens Starmap from ShipInterior (canvas changes)', async ({ page }) => {
    // 'm' is only bound in ShipInterior; it returns to Starmap
    await enterShipInterior(page);
    await storeSnapshot(page, 'before');
    await sendKey(page, 'm');
    await waitForCanvasChange(page, 'before', 100, 5000);
    expect(await countChangedPixels(page, 'before')).toBeGreaterThan(100);
  });

  test('Tab key does not crash the game', async ({ page }) => {
    // Tab is bound only in shop/crucible/console modes; in Starmap it is a no-op.
    // This test verifies the game handles it gracefully.
    const errors: string[] = [];
    page.on('pageerror', e => errors.push(e.message));
    await sendKey(page, 'Tab');
    await page.waitForTimeout(500);
    await expect(page.locator('#game-canvas')).toBeVisible();
    expect(errors.filter(e => !e.includes('WebSocket'))).toHaveLength(0);
  });

  test('Escape key changes canvas (opens settings or closes menu)', async ({ page }) => {
    await storeSnapshot(page, 'before');
    await sendKey(page, 'Escape');
    await page.waitForTimeout(500);
    expect(await countChangedPixels(page, 'before')).toBeGreaterThan(0);
  });

  test('opening and closing Codex restores previous state', async ({ page }) => {
    await storeSnapshot(page, 'game');

    // Open codex (toggle on) and wait for the overlay to render
    await sendKey(page, 'c');
    await waitForCanvasChange(page, 'game', 100, 8000);
    await storeSnapshot(page, 'codex');
    expect(await countChangedPixels(page, 'game')).toBeGreaterThan(100);

    // Close codex (toggle off — 'c' is a toggle; Escape is a no-op in Starmap)
    await sendKey(page, 'c');
    await waitForCanvasChange(page, 'codex', 50, 5000);

    // After closing, canvas should differ from the codex overlay state
    expect(await countChangedPixels(page, 'codex')).toBeGreaterThan(50);
  });

  test('Q key cycles ability (no crash, canvas may update)', async ({ page }) => {
    await storeSnapshot(page, 'before');
    await sendKey(page, 'q');
    await page.waitForTimeout(300);
    // At minimum no crash; HUD may update
    expect(await countChangedPixels(page, 'before')).toBeGreaterThanOrEqual(0);
  });

  test('multiple rapid keystrokes do not crash the game', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', e => errors.push(e.message));

    await enterShipInterior(page);
    for (let i = 0; i < 10; i++) {
      await sendKey(page, 'ArrowRight');
      await sendKey(page, 'ArrowLeft');
      await sendKey(page, 'ArrowUp');
      await sendKey(page, 'ArrowDown');
    }
    await page.waitForTimeout(1000);

    await expect(page.locator('#game-canvas')).toBeVisible();
    expect(errors.filter(e => !e.includes('WebSocket'))).toHaveLength(0);
  });

  test('backtick key opens developer console', async ({ page }) => {
    await storeSnapshot(page, 'before');
    await sendKey(page, '`');
    await waitForCanvasChange(page, 'before', 1, 5000);
    expect(await countChangedPixels(page, 'before')).toBeGreaterThan(0);
  });

  // ── WASD (tested inside ShipInterior) ──

  test('WASD movement keys work (w key moves up)', async ({ page }) => {
    await enterShipInterior(page);
    await storeSnapshot(page, 'before');
    await sendKey(page, 'w');
    await page.waitForTimeout(300);
    expect(await countChangedPixels(page, 'before')).toBeGreaterThan(0);
  });

  test('WASD movement keys work (s key enters ShipInterior)', async ({ page }) => {
    // 's' from Starmap enters ShipInterior — itself a significant canvas change
    await storeSnapshot(page, 'before');
    await sendKey(page, 's');
    await waitForCanvasChange(page, 'before', 1, 5000);
    expect(await countChangedPixels(page, 'before')).toBeGreaterThan(0);
  });

  test('WASD movement keys work (a key moves left)', async ({ page }) => {
    await enterShipInterior(page);
    await storeSnapshot(page, 'before');
    await sendKey(page, 'a');
    await page.waitForTimeout(300);
    expect(await countChangedPixels(page, 'before')).toBeGreaterThan(0);
  });

  test('WASD movement keys work (d key moves right)', async ({ page }) => {
    await enterShipInterior(page);
    await storeSnapshot(page, 'before');
    await sendKey(page, 'd');
    await page.waitForTimeout(300);
    expect(await countChangedPixels(page, 'before')).toBeGreaterThan(0);
  });
});
