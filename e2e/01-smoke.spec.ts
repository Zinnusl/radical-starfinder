import { test, expect } from '@playwright/test';
import { waitForGameReady, canvasHasContent, LOAD_TIMEOUT } from './helpers';

test.describe('Game smoke tests', () => {
  test('page loads with correct title', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle('Radical Starfinder');
  });

  test('loading indicator is present initially', async ({ page }) => {
    await page.goto('/');
    const loading = page.locator('#loading');
    await expect(loading).toBeVisible();
  });

  test('game canvas appears after WASM loads', async ({ page }) => {
    await page.goto('/');
    const canvas = page.locator('#game-canvas');
    await canvas.waitFor({ timeout: LOAD_TIMEOUT });
    await expect(canvas).toBeVisible();
  });

  test('canvas has non-zero dimensions', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });
    
    const dimensions = await page.evaluate(() => {
      const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
      return { width: canvas.width, height: canvas.height };
    });
    
    expect(dimensions.width).toBeGreaterThan(0);
    expect(dimensions.height).toBeGreaterThan(0);
  });

  test('no critical JavaScript errors on load', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    
    await page.goto('/');
    await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });
    await page.waitForTimeout(2000);
    
    // Filter out known benign errors (e.g. WebSocket connection failures for Trunk HMR)
    const criticalErrors = errors.filter(e => 
      !e.includes('WebSocket') && 
      !e.includes('trunk') &&
      !e.includes('sw.js') &&
      !e.includes('serviceWorker')
    );
    expect(criticalErrors).toHaveLength(0);
  });

  test('WASM module loads successfully', async ({ page }) => {
    const consoleMessages: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error') consoleMessages.push(msg.text());
    });
    
    await page.goto('/');
    await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });
    await page.waitForTimeout(3000); // give WASM time to fully init
    
    // Check wasmBindings is available (set by Trunk bootstrap)
    const wasmLoaded = await page.evaluate(() => {
      return typeof (window as any).wasmBindings !== 'undefined';
    });
    expect(wasmLoaded).toBe(true);
  });

  test('game canvas renders content after init', async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);
    
    const hasContent = await canvasHasContent(page);
    expect(hasContent).toBe(true);
  });
});
