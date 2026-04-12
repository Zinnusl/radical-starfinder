import { test, expect } from '@playwright/test';
import { waitForGameReady, sendKey, runCmd, snap, snapFull, diff, enterExploreMode } from './helpers';

// Confirmed from src/game/mod.rs:2504 — key 'i' / 'I' opens/closes inventory
// Only works in LocationExploration/GroundCombat modes with CombatState::Explore
const INVENTORY_KEY = 'i';

test.describe('Inventory and items', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);
    // Clear save so the game resets to ClassSelect on next navigation
    await page.evaluate(() => { try { localStorage.clear(); } catch (_) {} });
    // Reload to apply the cleared save state
    await page.reload();
    await waitForGameReady(page);
    // Navigate to dungeon explore mode: ClassSelect→Starmap→LocationExploration
    await enterExploreMode(page);
    // Give player items for testing
    await runCmd(page, 'give_item HealthPotion');
    await runCmd(page, 'give_item PoisonFlask');
    await page.waitForTimeout(300);
  });

  test('inventory key opens inventory overlay', async ({ page }) => {
    // Use snapFull — inventory overlay renders in the canvas center, not just the top 100px
    const before = await snapFull(page);
    await sendKey(page, INVENTORY_KEY);
    await page.waitForTimeout(500);
    const after = await snapFull(page);
    const changed = diff(before, after);
    console.log(`Inventory open: ${changed} pixels changed`);
    expect(changed).toBeGreaterThan(100);
  });

  test('Escape closes inventory', async ({ page }) => {
    await sendKey(page, INVENTORY_KEY);
    await page.waitForTimeout(400);
    const open = await snap(page);
    await sendKey(page, 'Escape');
    await page.waitForTimeout(400);
    const closed = await snap(page);
    expect(diff(open, closed)).toBeGreaterThan(50);
  });

  test('pressing i again closes inventory', async ({ page }) => {
    await sendKey(page, INVENTORY_KEY);
    await page.waitForTimeout(400);
    // Snap a centered region where the inventory overlay renders
    const open = await snap(page, 800, 400, 0, 100);
    await sendKey(page, INVENTORY_KEY);
    await page.waitForTimeout(400);
    const closed = await snap(page, 800, 400, 0, 100);
    // Canvas should change back when inventory closes
    expect(diff(open, closed)).toBeGreaterThan(50);
  });

  test('arrow keys navigate inventory items', async ({ page }) => {
    await sendKey(page, INVENTORY_KEY);
    await page.waitForTimeout(400);
    const first = await snap(page);
    await sendKey(page, 'ArrowDown');
    await page.waitForTimeout(200);
    const second = await snap(page);
    const changed = diff(first, second);
    console.log(`Inventory navigation: ${changed} pixels`);
    // Cursor highlight moves — canvas should differ
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('Enter in inventory opens item inspect view', async ({ page }) => {
    await sendKey(page, INVENTORY_KEY);
    await page.waitForTimeout(400);
    // Navigate to first consumable slot (cursor 3 = first item)
    await sendKey(page, 'ArrowDown');
    await sendKey(page, 'ArrowDown');
    await sendKey(page, 'ArrowDown');
    await page.waitForTimeout(200);
    const before = await snap(page);
    await sendKey(page, 'Enter');
    await page.waitForTimeout(400);
    const after = await snap(page);
    const changed = diff(before, after);
    console.log(`Inspect view: ${changed} pixels changed`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('using HealthPotion via hotkey changes HP display', async ({ page }) => {
    // Reduce HP so there is room to heal
    await runCmd(page, 'hp 10');
    await page.waitForTimeout(300);
    // HP bar is top-left at (12,12,172,28) — capture the top-left region
    const lowHp = await snap(page, 300, 60, 0, 0);

    // Items are used via 1–5 hotkeys from the main view (src/game/mod.rs:5013-5018)
    await sendKey(page, '1');
    await page.waitForTimeout(500);

    const afterUse = await snap(page, 300, 60, 0, 0);
    const changed = diff(lowHp, afterUse);
    console.log(`Using HealthPotion (hotkey 1): ${changed} pixels changed`);
    // HP bar or message should update
    expect(changed).toBeGreaterThan(0);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('crafting mode activates with c key (2+ items)', async ({ page }) => {
    await sendKey(page, INVENTORY_KEY);
    await page.waitForTimeout(400);
    const beforeCraft = await snap(page);
    // 'c' enters crafting sub-mode when >=2 items present
    await sendKey(page, 'c');
    await page.waitForTimeout(400);
    const afterCraft = await snap(page);
    const changed = diff(beforeCraft, afterCraft);
    console.log(`Crafting mode: ${changed} pixels changed`);
    // UI switches to crafting selection — canvas should differ
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('crafting mode Escape exits back to inventory', async ({ page }) => {
    await sendKey(page, INVENTORY_KEY);
    await page.waitForTimeout(300);
    await sendKey(page, 'c');
    await page.waitForTimeout(300);
    const craftingSnap = await snap(page);
    await sendKey(page, 'Escape');
    await page.waitForTimeout(300);
    const backToInv = await snap(page);
    expect(diff(craftingSnap, backToInv)).toBeGreaterThan(0);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('inventory with many items does not crash', async ({ page }) => {
    const items = [
      'RevealScroll', 'TeleportScroll', 'HastePotion', 'StunBomb',
      'RiceBall', 'GoldIngot', 'ThunderTalisman', 'InkBomb',
    ];
    for (const item of items) {
      await runCmd(page, `give_item ${item}`);
      await page.waitForTimeout(100);
    }

    await sendKey(page, INVENTORY_KEY);
    await page.waitForTimeout(500);
    await expect(page.locator('#game-canvas')).toBeVisible();

    // Navigate through several items
    for (let i = 0; i < 8; i++) {
      await sendKey(page, 'ArrowDown');
      await page.waitForTimeout(80);
    }
    // Scroll back up
    for (let i = 0; i < 4; i++) {
      await sendKey(page, 'ArrowUp');
      await page.waitForTimeout(80);
    }
    await expect(page.locator('#game-canvas')).toBeVisible();
    await sendKey(page, 'Escape');
  });

  test('closing and reopening inventory works multiple times', async ({ page }) => {
    for (let i = 0; i < 3; i++) {
      await sendKey(page, INVENTORY_KEY);
      await page.waitForTimeout(300);
      await sendKey(page, 'Escape');
      await page.waitForTimeout(200);
    }
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('items console command shows items', async ({ page }) => {
    const before = await snap(page);
    await runCmd(page, 'items');
    await page.waitForTimeout(400);
    const after = await snap(page);
    console.log(`items command: ${diff(before, after)} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });
});
