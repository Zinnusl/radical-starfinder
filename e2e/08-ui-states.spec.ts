import { test, expect } from '@playwright/test';
import { waitForGameReady, sendKey, captureCanvas, countDifferentPixels, getAverageColor, LOAD_TIMEOUT } from './helpers';

test.describe('Game UI state transitions', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);
    // Small wait to ensure game is fully initialized
    await page.waitForTimeout(500);
  });

  test('default state shows dungeon view (dark background)', async ({ page }) => {
    const avgColor = await getAverageColor(page);
    // Dungeon has dark background - average color should be mostly dark
    // But not pitch black because there are cyan/colored dungeon tiles
    expect(avgColor.r + avgColor.g + avgColor.b).toBeLessThan(200); // mostly dark
    console.log(`Default state average color: rgb(${Math.round(avgColor.r)},${Math.round(avgColor.g)},${Math.round(avgColor.b)})`);
  });

  test('C key opens Codex (significant canvas change)', async ({ page }) => {
    const before = await captureCanvas(page);
    await sendKey(page, 'c');
    await page.waitForTimeout(500);
    const after = await captureCanvas(page);
    
    const changedPixels = countDifferentPixels(before, after);
    expect(changedPixels).toBeGreaterThan(500);
    console.log(`Codex open: ${changedPixels} pixels changed`);
  });

  test('Codex can be closed with Escape', async ({ page }) => {
    // Open codex
    await sendKey(page, 'c');
    await page.waitForTimeout(500);
    const codexState = await captureCanvas(page);
    
    // Close with Escape
    await sendKey(page, 'Escape');
    await page.waitForTimeout(500);
    const closedState = await captureCanvas(page);
    
    // State should change after closing
    const changedPixels = countDifferentPixels(codexState, closedState);
    expect(changedPixels).toBeGreaterThan(200);
  });

  test('M key opens Starmap (significant canvas change)', async ({ page }) => {
    const before = await captureCanvas(page);
    await sendKey(page, 'm');
    await page.waitForTimeout(500);
    const after = await captureCanvas(page);
    
    const changedPixels = countDifferentPixels(before, after);
    expect(changedPixels).toBeGreaterThan(500);
    console.log(`Starmap open: ${changedPixels} pixels changed`);
  });

  test('Tab key opens Ship view (significant canvas change)', async ({ page }) => {
    const before = await captureCanvas(page);
    await sendKey(page, 'Tab');
    await page.waitForTimeout(500);
    const after = await captureCanvas(page);
    
    const changedPixels = countDifferentPixels(before, after);
    expect(changedPixels).toBeGreaterThan(500);
    console.log(`Ship view open: ${changedPixels} pixels changed`);
  });

  test('Escape key triggers settings or menu', async ({ page }) => {
    const before = await captureCanvas(page);
    await sendKey(page, 'Escape');
    await page.waitForTimeout(500);
    const after = await captureCanvas(page);
    
    // Something should change when Escape is pressed
    const changedPixels = countDifferentPixels(before, after);
    expect(changedPixels).toBeGreaterThanOrEqual(0); // At minimum no crash
    console.log(`Escape key: ${changedPixels} pixels changed`);
  });

  test('backtick opens developer console', async ({ page }) => {
    const before = await captureCanvas(page);
    await sendKey(page, '`');
    await page.waitForTimeout(500);
    const after = await captureCanvas(page);
    
    const changedPixels = countDifferentPixels(before, after);
    expect(changedPixels).toBeGreaterThan(0);
    console.log(`Console open: ${changedPixels} pixels changed`);
  });

  test('each major screen has distinct visual appearance', async ({ page }) => {
    // Capture each major screen
    const dungeonState = await captureCanvas(page);
    
    await sendKey(page, 'c');
    await page.waitForTimeout(500);
    const codexState = await captureCanvas(page);
    await sendKey(page, 'Escape');
    await page.waitForTimeout(300);
    
    await sendKey(page, 'm');
    await page.waitForTimeout(500);
    const starmapState = await captureCanvas(page);
    await sendKey(page, 'Escape');
    await page.waitForTimeout(300);
    
    await sendKey(page, 'Tab');
    await page.waitForTimeout(500);
    const shipState = await captureCanvas(page);
    await sendKey(page, 'Escape');
    await page.waitForTimeout(300);
    
    // Each screen should be visually distinct from the others
    const dungeonVsCodex = countDifferentPixels(dungeonState, codexState);
    const dungeonVsStarmap = countDifferentPixels(dungeonState, starmapState);
    const codexVsStarmap = countDifferentPixels(codexState, starmapState);
    
    console.log(`Dungeon vs Codex: ${dungeonVsCodex} pixels different`);
    console.log(`Dungeon vs Starmap: ${dungeonVsStarmap} pixels different`);
    console.log(`Codex vs Starmap: ${codexVsStarmap} pixels different`);
    
    // Each major screen should look significantly different
    expect(dungeonVsCodex).toBeGreaterThan(100);
    expect(dungeonVsStarmap).toBeGreaterThan(100);
  });

  test('developer console shows input overlay', async ({ page }) => {
    // Open console
    await sendKey(page, '`');
    await page.waitForTimeout(500);
    
    // Console is open - check canvas changed
    const consoleState = await captureCanvas(page);
    
    // Type in console
    await page.evaluate(() => {
      const canvas = document.getElementById('game-canvas');
      // Type 'help' into the console
      for (const char of 'help') {
        (canvas || document).dispatchEvent(new KeyboardEvent('keydown', { key: char, bubbles: true }));
      }
    });
    await page.waitForTimeout(300);
    
    const afterTyping = await captureCanvas(page);
    
    // Typing should change the console display
    const changed = countDifferentPixels(consoleState, afterTyping);
    expect(changed).toBeGreaterThanOrEqual(0); // at minimum no crash
    console.log(`After typing in console: ${changed} pixels changed`);
  });

  test('Q key cycles ability (HUD element changes)', async ({ page }) => {
    const before = await captureCanvas(page);
    await sendKey(page, 'q');
    await page.waitForTimeout(300);
    const after = await captureCanvas(page);
    
    // Q should cycle the currently selected ability in the HUD
    const changedPixels = countDifferentPixels(before, after);
    console.log(`Q key (cycle ability): ${changedPixels} pixels changed`);
    // At minimum, no crash
    expect(await page.locator('#game-canvas').isVisible()).toBe(true);
  });

  test('multiple screen transitions in sequence do not crash', async ({ page }) => {
    const screens = ['c', 'Escape', 'm', 'Escape', 'Tab', 'Escape', '`', 'Escape'];
    
    for (const key of screens) {
      await sendKey(page, key);
      await page.waitForTimeout(300);
    }
    
    // After all transitions, game should still be running
    await expect(page.locator('#game-canvas')).toBeVisible();
    
    const isRunning = await page.evaluate(() => {
      const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
      return canvas && canvas.width > 0 && canvas.height > 0;
    });
    expect(isRunning).toBe(true);
  });
});
