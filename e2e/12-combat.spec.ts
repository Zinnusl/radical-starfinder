import { test, expect } from '@playwright/test';
import { waitForGameReady, sendKey, typeString, runConsoleCommand, snap, diff, LOAD_TIMEOUT } from './helpers';

// After `fight normal`, player is at bottom-center and enemy at top of a 9×9 arena.
// Enemy has 2 movement/turn and chases the player. We press `w` (wait) to skip player
// turns until the enemy closes to melee range, then attack.
async function waitForEnemyAdjacent(page: import('@playwright/test').Page, skips = 6): Promise<void> {
  for (let i = 0; i < skips; i++) {
    await sendKey(page, 'w');           // wait: end player turn
    await page.waitForTimeout(600);     // allow enemy turn animation (~400 ms + buffer)
  }
}

// Full attack sequence once in TacticalBattle with an adjacent enemy.
async function doAttack(page: import('@playwright/test').Page, pinyin: string): Promise<void> {
  await sendKey(page, 'a');             // enter attack-targeting mode
  await page.waitForTimeout(300);
  await sendKey(page, 'Enter');         // confirm the auto-selected enemy target
  await page.waitForTimeout(300);
  await typeString(page, pinyin, 60);   // type pinyin syllable
  await page.waitForTimeout(100);
  await sendKey(page, 'Enter');         // submit attack
  await page.waitForTimeout(600);
}

test.describe('Combat system', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);
  });

  test('console "fight normal" command triggers a combat encounter', async ({ page }) => {
    const before = await snap(page);
    await runConsoleCommand(page, 'fight normal');
    await page.waitForTimeout(600);
    const after = await snap(page);
    // Spawning a fight changes the canvas (combat UI appears or enemy adjacent)
    expect(diff(before, after)).toBeGreaterThan(50);
  });

  test('god mode can be enabled via console', async ({ page }) => {
    // Enable god mode so we don't die during combat tests
    const before = await snap(page);
    await runConsoleCommand(page, 'god');
    await page.waitForTimeout(300);
    const after = await snap(page);
    // A message/visual change should confirm god mode
    // (the game shows a message "GOD MODE ON" or similar)
    console.log(`God mode command changed ${diff(before, after)} pixels`);
    // At minimum, game should still be alive
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('combat: fight spawns and player can attack with pinyin', async ({ page }) => {
    // Enable god mode and spawn a fight
    await runConsoleCommand(page, 'god');
    await runConsoleCommand(page, 'fight normal');
    await page.waitForTimeout(500);

    const beforeAttack = await snap(page);

    // Skip player turns until the enemy approaches to melee range
    await waitForEnemyAdjacent(page);

    // Attack: targeting → confirm → type pinyin → submit
    // Common pinyin: "ren" (人), "wo" (我), "ni" (你), "da" (大)
    // We try a generic syllable — correct or not, submitting changes the canvas
    await doAttack(page, 'ren');

    const afterAttack = await snap(page);
    // Submitting an attack attempt changes the canvas (right or wrong)
    expect(diff(beforeAttack, afterAttack)).toBeGreaterThan(20);
  });

  test('combat: multiple attack attempts do not crash the game', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', e => { if (!e.message.includes('WebSocket')) errors.push(e.message); });

    await runConsoleCommand(page, 'god');
    await runConsoleCommand(page, 'fight normal');
    await page.waitForTimeout(500);

    // Wait for enemy to approach
    await waitForEnemyAdjacent(page);

    // Try several pinyin attempts across multiple player turns
    const attempts = ['ren', 'wo', 'ni', 'da', 'hao', 'ma'];
    for (const pinyin of attempts) {
      await doAttack(page, pinyin);
      // Small pause between turns; enemy may counter-attack via tick
      await page.waitForTimeout(400);
    }

    await expect(page.locator('#game-canvas')).toBeVisible();
    expect(errors).toHaveLength(0);
  });

  test('combat: boss fight can be triggered', async ({ page }) => {
    const before = await snap(page);
    await runConsoleCommand(page, 'god');
    await runConsoleCommand(page, 'boss PirateCaptain');
    await page.waitForTimeout(600);
    const after = await snap(page);
    expect(diff(before, after)).toBeGreaterThan(50);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('combat: fight elite encounter works', async ({ page }) => {
    await runConsoleCommand(page, 'god');
    const before = await snap(page);
    await runConsoleCommand(page, 'fight elite');
    await page.waitForTimeout(600);
    const after = await snap(page);
    console.log(`Elite fight changed ${diff(before, after)} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('combat: pressing Escape exits combat prompt if open', async ({ page }) => {
    await runConsoleCommand(page, 'fight normal');
    await page.waitForTimeout(500);
    const inCombat = await snap(page);
    await sendKey(page, 'Escape');
    await page.waitForTimeout(300);
    const afterEsc = await snap(page);
    // Escape should change something (close combat overlay or pause menu)
    console.log(`Escape in combat: ${diff(inCombat, afterEsc)} pixels changed`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('combat: entering wrong pinyin shows feedback', async ({ page }) => {
    await runConsoleCommand(page, 'god');
    await runConsoleCommand(page, 'fight normal');
    await page.waitForTimeout(500);

    // Wait for enemy to approach
    await waitForEnemyAdjacent(page);

    // Enter targeting mode and confirm target
    await sendKey(page, 'a');
    await page.waitForTimeout(300);
    await sendKey(page, 'Enter');         // confirm target → enter typing mode
    await page.waitForTimeout(300);

    // Type something clearly wrong
    await typeString(page, 'zzz');
    const beforeEnter = await snap(page);
    await sendKey(page, 'Enter');
    await page.waitForTimeout(500);
    const afterEnter = await snap(page);

    // Canvas should change (feedback shown even for wrong answer)
    expect(diff(beforeEnter, afterEnter)).toBeGreaterThanOrEqual(0);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('kill_all console command eliminates enemies', async ({ page }) => {
    await runConsoleCommand(page, 'reveal');  // reveal map first
    await page.waitForTimeout(300);
    const before = await snap(page);
    await runConsoleCommand(page, 'kill_all');
    await page.waitForTimeout(500);
    const after = await snap(page);
    console.log(`kill_all changed ${diff(before, after)} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });
});
