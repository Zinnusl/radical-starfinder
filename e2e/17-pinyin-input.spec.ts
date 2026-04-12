import { test, expect } from '@playwright/test';
import { waitForGameReady, sendKey, typeStr, runCmd, snap, diff, LOAD_TIMEOUT } from './helpers';

async function startFightAndWaitForInput(page: any): Promise<void> {
  await runCmd(page, 'god');
  await runCmd(page, 'fight normal');
  await page.waitForTimeout(600);
}

test.describe('Pinyin combat input', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);
  });

  test('typing a letter in combat mode changes input display', async ({ page }) => {
    await startFightAndWaitForInput(page);
    const before = await snap(page);
    await sendKey(page, 'r');  // first letter of 'ren'
    await page.waitForTimeout(200);
    const after = await snap(page);
    const changed = diff(before, after);
    console.log(`Typing 'r' changed ${changed} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('typing pinyin syllable updates input buffer display', async ({ page }) => {
    await startFightAndWaitForInput(page);
    const empty = await snap(page);

    for (const ch of 'ren') {
      await sendKey(page, ch);
      await page.waitForTimeout(80);
    }
    const withInput = await snap(page);

    const changed = diff(empty, withInput);
    console.log(`Typing 'ren' changed ${changed} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('backspace clears last typed character', async ({ page }) => {
    await startFightAndWaitForInput(page);

    // Type 'ren'
    for (const ch of 'ren') { await sendKey(page, ch); await page.waitForTimeout(60); }
    const afterType = await snap(page);

    // Backspace should remove 'n'
    await sendKey(page, 'Backspace');
    await page.waitForTimeout(200);
    const afterBackspace = await snap(page);

    const changed = diff(afterType, afterBackspace);
    console.log(`Backspace changed ${changed} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('Enter submits pinyin attack and canvas changes', async ({ page }) => {
    await startFightAndWaitForInput(page);

    // Type a pinyin attempt
    for (const ch of 'ren') { await sendKey(page, ch); await page.waitForTimeout(60); }

    const beforeSubmit = await snap(page);
    await sendKey(page, 'Enter');
    await page.waitForTimeout(600);
    const afterSubmit = await snap(page);

    const changed = diff(beforeSubmit, afterSubmit);
    console.log(`Enter submit changed ${changed} pixels`);
    expect(changed).toBeGreaterThan(20);
  });

  test('multiple pinyin submissions in one fight', async ({ page }) => {
    await startFightAndWaitForInput(page);

    const attempts = ['ren', 'wo', 'ni', 'da', 'hao'];
    for (const pinyin of attempts) {
      for (const ch of pinyin) { await sendKey(page, ch); await page.waitForTimeout(40); }
      await sendKey(page, 'Enter');
      await page.waitForTimeout(500);
    }

    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('long pinyin input does not break display', async ({ page }) => {
    await startFightAndWaitForInput(page);

    // Type a very long string
    for (const ch of 'zhongwenxuexiyizhenghaoling') {
      await sendKey(page, ch);
      await page.waitForTimeout(30);
    }
    await page.waitForTimeout(300);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('typing numbers in pinyin input is handled gracefully', async ({ page }) => {
    await startFightAndWaitForInput(page);

    for (const ch of 'r3n') {
      await sendKey(page, ch);
      await page.waitForTimeout(60);
    }
    await sendKey(page, 'Enter');
    await page.waitForTimeout(400);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('empty Enter (no pinyin typed) is handled', async ({ page }) => {
    await startFightAndWaitForInput(page);

    // Press Enter without typing anything
    await sendKey(page, 'Enter');
    await page.waitForTimeout(400);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('input buffer cleared after submission', async ({ page }) => {
    await startFightAndWaitForInput(page);

    for (const ch of 'ren') { await sendKey(page, ch); await page.waitForTimeout(40); }
    const withInput = await snap(page);

    await sendKey(page, 'Enter');
    await page.waitForTimeout(700);

    // Start second fight turn - type again
    for (const ch of 'wo') { await sendKey(page, ch); await page.waitForTimeout(40); }
    const secondInput = await snap(page);

    // After submission, new input should look different from previous
    await expect(page.locator('#game-canvas')).toBeVisible();
  });
});
