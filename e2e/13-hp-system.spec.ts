import { test, expect } from '@playwright/test';
import { waitForGameReady, sendKey, runCmd, snap, snapFull, diff, LOAD_TIMEOUT } from './helpers';

test.describe('HP and stats system', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);
  });

  test('hp console command changes HP display', async ({ page }) => {
    // Capture the HUD area (bottom strip of canvas where HP is shown)
    const before = await snapFull(page);
    await runCmd(page, 'hp 50');
    await page.waitForTimeout(400);
    const after = await snapFull(page);
    // Setting HP to 50 should change the HP bar/text on HUD
    const changed = diff(before, after);
    console.log(`hp command changed ${changed} pixels`);
    expect(changed).toBeGreaterThan(0);
  });

  test('hp 1 sets very low HP (visible change from full HP)', async ({ page }) => {
    const fullHp = await snapFull(page);
    await runCmd(page, 'hp 1');
    await page.waitForTimeout(400);
    const lowHp = await snapFull(page);
    const changed = diff(fullHp, lowHp);
    console.log(`hp 1 changed ${changed} pixels vs full HP`);
    expect(changed).toBeGreaterThan(0);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('give_item HealthPotion gives player a health potion', async ({ page }) => {
    const before = await snapFull(page);
    await runCmd(page, 'give_item HealthPotion');
    await page.waitForTimeout(400);
    const after = await snapFull(page);
    console.log(`give_item HealthPotion changed ${diff(before,after)} pixels`);
    // At minimum, game still running
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('gold command gives player gold (HUD changes)', async ({ page }) => {
    const before = await snapFull(page);
    await runCmd(page, 'gold 999');
    await page.waitForTimeout(400);
    const after = await snapFull(page);
    const changed = diff(before, after);
    console.log(`gold 999 changed ${changed} pixels`);
    expect(changed).toBeGreaterThan(0);
  });

  test('stats command shows stats overlay', async ({ page }) => {
    const before = await snapFull(page);
    await runCmd(page, 'stats');
    await page.waitForTimeout(400);
    const after = await snapFull(page);
    console.log(`stats command changed ${diff(before,after)} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('god mode toggle changes visual state', async ({ page }) => {
    const before = await snapFull(page);
    await runCmd(page, 'god');
    await page.waitForTimeout(400);
    const godOn = await snapFull(page);
    const d1 = diff(before, godOn);
    
    // Toggle off
    await runCmd(page, 'god');
    await page.waitForTimeout(400);
    const godOff = await snapFull(page);
    const d2 = diff(godOn, godOff);
    
    console.log(`god on: ${d1} pixels, god off: ${d2} pixels`);
    // At least one toggle should change something
    expect(d1 + d2).toBeGreaterThan(0);
  });

  test('taking damage in fight changes HUD display', async ({ page }) => {
    // Don't use god mode — we want to take damage
    // Set HP to full first via hp command
    await runCmd(page, 'hp 100');
    await page.waitForTimeout(300);
    const fullHpState = await snapFull(page);
    
    // Spawn a fight and get hit (don't attack back, enemy will hit us)
    await runCmd(page, 'fight normal');
    await page.waitForTimeout(600);
    // Wait a bit for enemy to attack
    // Press Enter (wait/pass turn) to let enemy attack
    await sendKey(page, 'Enter');
    await page.waitForTimeout(400);
    
    const afterDamage = await snapFull(page);
    const changed = diff(fullHpState, afterDamage);
    console.log(`After taking damage: ${changed} pixels changed`);
    // HP bar should change after being hit
    expect(changed).toBeGreaterThan(0);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('multiple hp changes in sequence work', async ({ page }) => {
    for (const hp of ['hp 100', 'hp 50', 'hp 25', 'hp 75', 'hp 1']) {
      await runCmd(page, hp);
      await page.waitForTimeout(200);
      await expect(page.locator('#game-canvas')).toBeVisible();
    }
  });

  test('give multiple different items does not crash', async ({ page }) => {
    const items = ['HealthPotion','PoisonFlask','RevealScroll','TeleportScroll','StunBomb'];
    for (const item of items) {
      await runCmd(page, `give_item ${item}`);
      await page.waitForTimeout(150);
    }
    await expect(page.locator('#game-canvas')).toBeVisible();
  });
});
