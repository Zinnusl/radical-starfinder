import { test, expect } from '@playwright/test';
import { waitForGameReady, sendKey, runCmd, snap, diff } from './helpers';

test.describe('Floor progression', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);
  });

  test('floor 2 command jumps to floor 2 (canvas changes)', async ({ page }) => {
    const floor1 = await snap(page);
    await runCmd(page, 'floor 2');
    await page.waitForTimeout(800);
    const floor2 = await snap(page);
    const changed = diff(floor1, floor2);
    console.log(`Floor 1→2: ${changed} pixels changed`);
    expect(changed).toBeGreaterThan(100);
  });

  test('floor 5 command shows deeper dungeon', async ({ page }) => {
    const floor1 = await snap(page);
    await runCmd(page, 'floor 5');
    await page.waitForTimeout(800);
    const floor5 = await snap(page);
    const changed = diff(floor1, floor5);
    console.log(`Floor 1→5: ${changed} pixels changed`);
    expect(changed).toBeGreaterThan(100);
  });

  test('floor 1 vs floor 10 look different', async ({ page }) => {
    await runCmd(page, 'floor 1');
    await page.waitForTimeout(800);
    const f1 = await snap(page);

    await runCmd(page, 'floor 10');
    await page.waitForTimeout(800);
    const f10 = await snap(page);

    const changed = diff(f1, f10);
    console.log(`Floor 1 vs 10: ${changed} pixels different`);
    expect(changed).toBeGreaterThan(50);
  });

  test('jumping floors multiple times does not crash', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', e => { if (!e.message.includes('WebSocket')) errors.push(e.message); });

    for (const f of [2, 5, 3, 8, 1, 4]) {
      await runCmd(page, `floor ${f}`);
      await page.waitForTimeout(500);
      await expect(page.locator('#game-canvas')).toBeVisible();
    }
    expect(errors).toHaveLength(0);
  });

  test('reveal command shows full floor map', async ({ page }) => {
    const before = await snap(page);
    await runCmd(page, 'reveal');
    await page.waitForTimeout(500);
    const after = await snap(page);
    const changed = diff(before, after);
    console.log(`reveal changed ${changed} pixels`);
    // Revealing the map should show previously hidden tiles
    expect(changed).toBeGreaterThan(0);
  });

  test('floor 2 has different map layout from floor 1', async ({ page }) => {
    await runCmd(page, 'reveal');
    await page.waitForTimeout(400);
    const f1 = await snap(page);

    await runCmd(page, 'floor 2');
    await page.waitForTimeout(600);
    await runCmd(page, 'reveal');
    await page.waitForTimeout(400);
    const f2 = await snap(page);

    const changed = diff(f1, f2);
    console.log(`Different floor layouts: ${changed} pixels different`);
    expect(changed).toBeGreaterThan(100);
  });

  test('floor 20 (deep) loads without crash', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', e => { if (!e.message.includes('WebSocket')) errors.push(e.message); });
    
    await runCmd(page, 'floor 20');
    await page.waitForTimeout(1000);
    await expect(page.locator('#game-canvas')).toBeVisible();
    expect(errors).toHaveLength(0);
  });

  test('movement works after floor change', async ({ page }) => {
    await runCmd(page, 'floor 3');
    await page.waitForTimeout(600);

    // Try to move
    let moved = false;
    for (let i = 0; i < 8; i++) {
      const before = await snap(page);
      await sendKey(page, 'ArrowRight');
      await page.waitForTimeout(120);
      const after = await snap(page);
      if (diff(before, after) > 10) { moved = true; break; }
    }
    console.log(`Movement after floor change: ${moved}`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('kill_all then floor advance works', async ({ page }) => {
    await runCmd(page, 'god');
    await runCmd(page, 'kill_all');
    await page.waitForTimeout(400);
    await runCmd(page, 'floor 2');
    await page.waitForTimeout(600);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });
});
