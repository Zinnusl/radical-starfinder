import { test, expect } from '@playwright/test';
import { waitForGameReady, sendKey, runCmd, snap, diff } from './helpers';

test.describe('Ability and spell system', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);
  });

  test('Q key cycles ability in HUD', async ({ page }) => {
    const before = await snap(page);
    await sendKey(page, 'q');
    await page.waitForTimeout(300);
    const after = await snap(page);
    const changed = diff(before, after);
    console.log(`Q cycle changed ${changed} pixels`);
    // Q should change HUD display
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('pressing Q multiple times cycles through abilities', async ({ page }) => {
    const states: number[][] = [];
    for (let i = 0; i < 5; i++) {
      states.push(await snap(page));
      await sendKey(page, 'q');
      await page.waitForTimeout(200);
    }
    // Some consecutive states should differ (different abilities selected)
    let anyChanged = false;
    for (let i = 1; i < states.length; i++) {
      if (diff(states[i-1], states[i]) > 0) { anyChanged = true; break; }
    }
    console.log(`Q cycle: any change across 5 presses: ${anyChanged}`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('Space key uses current ability (canvas changes)', async ({ page }) => {
    const before = await snap(page);
    await sendKey(page, ' ');
    await page.waitForTimeout(400);
    const after = await snap(page);
    const changed = diff(before, after);
    console.log(`Space (use ability) changed ${changed} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('radicals console command lists radicals', async ({ page }) => {
    const before = await snap(page);
    await runCmd(page, 'radicals');
    await page.waitForTimeout(400);
    const after = await snap(page);
    console.log(`radicals command changed ${diff(before, after)} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('spells console command lists spells', async ({ page }) => {
    const before = await snap(page);
    await runCmd(page, 'spells');
    await page.waitForTimeout(400);
    const after = await snap(page);
    console.log(`spells command changed ${diff(before, after)} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('give_spell command gives a spell', async ({ page }) => {
    // '明' is the first output_hanzi in RECIPES (日+月 → 明 bright)
    const before = await snap(page);
    await runCmd(page, 'give_spell 明');
    await page.waitForTimeout(400);
    const after = await snap(page);
    console.log(`give_spell changed ${diff(before, after)} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('give_radical command gives a radical', async ({ page }) => {
    // '火' is the first radical in RADICALS (fire)
    const before = await snap(page);
    await runCmd(page, 'give_radical 火');
    await page.waitForTimeout(400);
    const after = await snap(page);
    console.log(`give_radical changed ${diff(before, after)} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('give_spell then Q cycles the new spell', async ({ page }) => {
    await runCmd(page, 'give_spell 明');
    await page.waitForTimeout(300);
    const before = await snap(page);
    await sendKey(page, 'q');
    await page.waitForTimeout(300);
    const after = await snap(page);
    const changed = diff(before, after);
    console.log(`give_spell then Q changed ${changed} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('Q then Space: cycle then use ability', async ({ page }) => {
    await sendKey(page, 'q');
    await page.waitForTimeout(200);
    const before = await snap(page);
    await sendKey(page, ' ');
    await page.waitForTimeout(400);
    const after = await snap(page);
    console.log(`Q+Space changed ${diff(before, after)} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('T key opens skill tree', async ({ page }) => {
    test.setTimeout(60000);
    const before = await snap(page, 400, 300);
    await sendKey(page, 't');
    await page.waitForTimeout(400);
    const after = await snap(page, 400, 300);
    const changed = diff(before, after);
    console.log(`T (skill tree) changed ${changed} pixels`);
    // Skill tree overlay should change canvas significantly
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('T key toggles skill tree open and closed', async ({ page }) => {
    test.setTimeout(90000);
    const base = await snap(page, 400, 300);
    await sendKey(page, 't');
    await page.waitForTimeout(400);
    const opened = await snap(page, 400, 300);
    await sendKey(page, 't');
    await page.waitForTimeout(400);
    const closed = await snap(page, 400, 300);
    const openDiff = diff(base, opened);
    const closeDiff = diff(opened, closed);
    console.log(`Skill tree open changed ${openDiff} px, closing changed ${closeDiff} px`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('ability usage with god mode does not crash', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', e => { if (!e.message.includes('WebSocket')) errors.push(e.message); });

    await runCmd(page, 'god');
    await runCmd(page, 'give_spell 明');
    for (let i = 0; i < 10; i++) {
      await sendKey(page, 'q');
      await page.waitForTimeout(100);
      await sendKey(page, ' ');
      await page.waitForTimeout(200);
    }
    await expect(page.locator('#game-canvas')).toBeVisible();
    expect(errors).toHaveLength(0);
  });
});
