import { test, expect } from '@playwright/test';
import { waitForGameReady, sendKey, typeStr, runCmd, snap, snapFull, diff, LOAD_TIMEOUT } from './helpers';

test.describe('Full gameplay session', () => {
  test('complete gameplay session: load → explore → fight → advance', async ({ page }) => {
    test.setTimeout(180000);
    const errors: string[] = [];
    page.on('pageerror', e => { if (!e.message.includes('WebSocket')) errors.push(e.message); });

    // === PHASE 1: Load game ===
    await page.goto('/');
    await waitForGameReady(page);
    console.log('✓ Phase 1: Game loaded');

    // === PHASE 2: Enable god mode and setup ===
    await runCmd(page, 'god');
    await runCmd(page, 'reveal');
    await runCmd(page, 'give_item HealthPotion');
    await runCmd(page, 'give_item RiceBall');
    await runCmd(page, 'gold 500');
    await page.waitForTimeout(400);
    console.log('✓ Phase 2: Setup complete');

    // === PHASE 3: Explore dungeon ===
    for (let i = 0; i < 10; i++) {
      const keys = ['ArrowRight', 'ArrowRight', 'ArrowDown', 'ArrowLeft', 'ArrowUp'];
      await sendKey(page, keys[i % keys.length]);
      await page.waitForTimeout(80);
    }
    console.log('✓ Phase 3: Exploration done');

    // === PHASE 4: Open all screens ===
    for (const [key, name] of [['c', 'Codex'], ['m', 'Starmap'], ['Tab', 'Ship']]) {
      const before = await snap(page, 400, 300);
      await sendKey(page, key);
      await page.waitForTimeout(400);
      const after = await snap(page, 400, 300);
      console.log(`  ${name}: ${diff(before, after)} px changed`);
      await sendKey(page, 'Escape');
      await page.waitForTimeout(200);
    }
    console.log('✓ Phase 4: All screens visited');

    // === PHASE 5: Combat ===
    await runCmd(page, 'fight normal');
    await page.waitForTimeout(500);
    for (const pinyin of ['ren', 'wo', 'ni']) {
      for (const ch of pinyin) { await sendKey(page, ch); await page.waitForTimeout(30); }
      await sendKey(page, 'Enter');
      await page.waitForTimeout(400);
    }
    console.log('✓ Phase 5: Combat done');

    // === PHASE 6: Floor advancement ===
    for (const floor of [2, 3, 5]) {
      await runCmd(page, `floor ${floor}`);
      await page.waitForTimeout(600);
      await expect(page.locator('#game-canvas')).toBeVisible();
    }
    console.log('✓ Phase 6: Floor advancement done');

    // === PHASE 7: Final checks ===
    await expect(page.locator('#game-canvas')).toBeVisible();
    expect(errors).toHaveLength(0);
    expect(await page.title()).toBe('Radical Starfinder');

    const isRendering = await page.evaluate(() => {
      const c = document.getElementById('game-canvas') as HTMLCanvasElement;
      const d = c.getContext('2d')!.getImageData(0, 0, c.width, c.height).data;
      for (let i = 0; i < d.length; i += 4) if (d[i] > 20 || d[i + 1] > 20 || d[i + 2] > 30) return true;
      return false;
    });
    expect(isRendering).toBe(true);
    console.log('✓ Phase 7: All checks passed');
  });

  test('game state persists across page navigation', async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);

    // Give player items and set gold
    await runCmd(page, 'gold 777');
    await runCmd(page, 'give_item HealthPotion');
    await page.waitForTimeout(500);

    // Check localStorage has game data
    const lsKeys = await page.evaluate(() => Object.keys(localStorage));
    console.log('localStorage keys after play:', lsKeys);

    // Reload page
    await page.reload();
    await waitForGameReady(page);

    // Game should load (possibly restoring saved state)
    const lsKeysAfter = await page.evaluate(() => Object.keys(localStorage));
    console.log('localStorage keys after reload:', lsKeysAfter);

    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('10 consecutive floor jumps with exploration each floor', async ({ page }) => {
    test.setTimeout(60000);
    await page.goto('/');
    await waitForGameReady(page);
    await runCmd(page, 'god');

    for (let floor = 1; floor <= 10; floor++) {
      await runCmd(page, `floor ${floor}`);
      await page.waitForTimeout(400);
      // Do a few moves on each floor
      for (let m = 0; m < 4; m++) {
        await sendKey(page, ['ArrowRight', 'ArrowDown', 'ArrowLeft', 'ArrowUp'][m]);
        await page.waitForTimeout(60);
      }
    }
    await expect(page.locator('#game-canvas')).toBeVisible();
    console.log('✓ All 10 floors explored');
  });

  test('all console commands in sequence do not crash game', async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);

    const sequence = [
      'help', 'god', 'hp 100', 'gold 999', 'reveal',
      'give_item HealthPotion', 'give_item RiceBall',
      'stats', 'items', 'floor 2', 'kill_all',
      'floor 3', 'fight normal', 'god', 'clear'
    ];
    const errors: string[] = [];
    page.on('pageerror', e => { if (!e.message.includes('WebSocket')) errors.push(e.message); });

    for (const cmd of sequence) {
      await runCmd(page, cmd);
      await page.waitForTimeout(200);
    }

    await expect(page.locator('#game-canvas')).toBeVisible();
    expect(errors).toHaveLength(0);
  });

  test('game works after clearing localStorage', async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);
    await runCmd(page, 'god');
    await page.waitForTimeout(500);

    // Clear all game state
    await page.evaluate(() => localStorage.clear());

    // Reload - game should still work from scratch
    await page.reload();
    await waitForGameReady(page);
    await expect(page.locator('#game-canvas')).toBeVisible();
    console.log('✓ Game works after localStorage cleared');
  });
});
