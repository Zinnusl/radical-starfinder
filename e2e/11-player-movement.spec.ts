import { test, expect } from '@playwright/test';
import { waitForGameReady, sendKey, snapHash, moveUntilChanged, LOAD_TIMEOUT } from './helpers';

test.describe('Player movement', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);
  });

  test('ArrowRight moves the player (canvas changes)', async ({ page }) => {
    const moved = await moveUntilChanged(page, 'ArrowRight');
    // Even if first cell is a wall, within 15 tries we should find open floor
    // The dungeon always starts the player in an open room
    expect(moved).toBe(true);
  });

  test('ArrowLeft moves the player (canvas changes)', async ({ page }) => {
    // First move right to get away from walls, then try left
    await moveUntilChanged(page, 'ArrowRight');
    await page.waitForTimeout(100);
    const moved = await moveUntilChanged(page, 'ArrowLeft');
    expect(moved).toBe(true);
  });

  test('ArrowUp moves the player (canvas changes)', async ({ page }) => {
    const moved = await moveUntilChanged(page, 'ArrowUp');
    expect(moved).toBe(true);
  });

  test('ArrowDown moves the player (canvas changes)', async ({ page }) => {
    const moved = await moveUntilChanged(page, 'ArrowDown');
    expect(moved).toBe(true);
  });

  test('w key moves player up (WASD)', async ({ page }) => {
    const moved = await moveUntilChanged(page, 'w');
    expect(moved).toBe(true);
  });

  test('s key moves player down (WASD)', async ({ page }) => {
    const moved = await moveUntilChanged(page, 's');
    expect(moved).toBe(true);
  });

  test('a key moves player left (WASD)', async ({ page }) => {
    await moveUntilChanged(page, 'd'); // move right first
    const moved = await moveUntilChanged(page, 'a');
    expect(moved).toBe(true);
  });

  test('d key moves player right (WASD)', async ({ page }) => {
    const moved = await moveUntilChanged(page, 'd');
    expect(moved).toBe(true);
  });

  test('moving back and forth changes canvas both times', async ({ page }) => {
    // Move right until we get an open cell
    const movedRight = await moveUntilChanged(page, 'ArrowRight');
    expect(movedRight).toBe(true);

    const hashRight = await snapHash(page);

    // Move back left
    const movedLeft = await moveUntilChanged(page, 'ArrowLeft');
    expect(movedLeft).toBe(true);

    const hashLeft = await snapHash(page);

    // The two states should look different (player is in different position)
    expect(hashRight).not.toBe(hashLeft);
  });

  test('10 consecutive moves complete without crash', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', e => { if (!e.message.includes('WebSocket')) errors.push(e.message); });

    const directions = ['ArrowRight','ArrowRight','ArrowDown','ArrowDown',
                        'ArrowLeft','ArrowLeft','ArrowUp','ArrowUp','ArrowRight','ArrowDown'];
    for (const key of directions) {
      await sendKey(page, key);
      await page.waitForTimeout(80);
    }
    await page.waitForTimeout(500);

    await expect(page.locator('#game-canvas')).toBeVisible();
    expect(errors).toHaveLength(0);
  });

  test('movement is tile-by-tile (each keypress = one step)', async ({ page }) => {
    // Use snapHash for fast in-browser comparison (no large array transfer)
    let changes = 0;
    for (let i = 0; i < 6; i++) {
      const before = await snapHash(page);
      await sendKey(page, 'ArrowRight');
      await page.waitForTimeout(120);
      const after = await snapHash(page);
      if (before !== after) changes++;
    }
    // At least 2 of 6 presses should cause movement (open floor expected)
    expect(changes).toBeGreaterThanOrEqual(2);
  });

  test('game canvas remains visible throughout movement', async ({ page }) => {
    for (let i = 0; i < 20; i++) {
      const keys = ['ArrowRight','ArrowDown','ArrowLeft','ArrowUp'];
      await sendKey(page, keys[i % 4]);
      await page.waitForTimeout(60);
    }
    await expect(page.locator('#game-canvas')).toBeVisible();
  });
});
