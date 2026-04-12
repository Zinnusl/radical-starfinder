import { test, expect } from '@playwright/test';
import { waitForGameReady, canvasNonBlank, LOAD_TIMEOUT } from './helpers';
import * as path from 'path';
import * as fs from 'fs';

test.describe('Visual regression and screenshot tests', () => {
  test.describe('Desktop 1280x720', () => {
    test.use({ viewport: { width: 1280, height: 720 } });

    test('game renders at 1280x720 - canvas not blank', async ({ page }) => {
      await page.goto('/');
      await waitForGameReady(page);
      expect(await canvasNonBlank(page)).toBe(true);
    });

    test('game canvas fills viewport at 1280x720', async ({ page }) => {
      await page.goto('/');
      await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });
      await page.waitForTimeout(2000);
      
      const dims = await page.evaluate(() => {
        const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
        const rect = canvas.getBoundingClientRect();
        return { 
          cssWidth: rect.width, 
          cssHeight: rect.height,
          bufWidth: canvas.width,
          bufHeight: canvas.height,
          vw: window.innerWidth,
          vh: window.innerHeight
        };
      });
      
      console.log('Canvas dimensions at 1280x720:', dims);
      expect(dims.bufWidth).toBeGreaterThan(600);
      expect(dims.bufHeight).toBeGreaterThan(400);
    });

    test('game screenshot is not all background color', async ({ page }) => {
      await page.goto('/');
      await waitForGameReady(page);
      
      // Take a screenshot of the canvas element
      const canvas = page.locator('#game-canvas');
      const screenshot = await canvas.screenshot();
      
      // Screenshot should not be empty
      expect(screenshot.length).toBeGreaterThan(1000);
      
      // Save screenshot for manual review
      const dir = path.join(process.cwd(), 'e2e', 'screenshots');
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, 'game-1280x720.png'), screenshot);
    });
  });

  test.describe('Desktop 1920x1080', () => {
    test.use({ viewport: { width: 1920, height: 1080 } });

    test('game renders at 1920x1080 - canvas not blank', async ({ page }) => {
      await page.goto('/');
      await waitForGameReady(page);
      expect(await canvasNonBlank(page)).toBe(true);
    });

    test('game screenshot at 1920x1080', async ({ page }) => {
      await page.goto('/');
      await waitForGameReady(page);
      
      const screenshot = await page.locator('#game-canvas').screenshot();
      const dir = path.join(process.cwd(), 'e2e', 'screenshots');
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, 'game-1920x1080.png'), screenshot);
      
      expect(screenshot.length).toBeGreaterThan(1000);
    });
  });

  test.describe('Mobile viewport', () => {
    test.use({ viewport: { width: 375, height: 667 } });

    test('game renders on mobile viewport', async ({ page }) => {
      await page.goto('/');
      await waitForGameReady(page);
      expect(await canvasNonBlank(page)).toBe(true);
    });

    test('game screenshot on mobile', async ({ page }) => {
      await page.goto('/');
      await waitForGameReady(page);
      
      const screenshot = await page.locator('#game-canvas').screenshot();
      const dir = path.join(process.cwd(), 'e2e', 'screenshots');
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, 'game-mobile-375x667.png'), screenshot);
      
      expect(screenshot.length).toBeGreaterThan(1000);
    });
  });

  test.describe('Game state screenshots', () => {
    test.use({ viewport: { width: 1280, height: 720 } });

    test('capture codex screen screenshot', async ({ page }) => {
      await page.goto('/');
      await waitForGameReady(page);
      
      await page.evaluate(() => {
        const canvas = document.getElementById('game-canvas');
        (canvas || document).dispatchEvent(new KeyboardEvent('keydown', { key: 'c', bubbles: true }));
      });
      await page.waitForTimeout(700);
      
      const screenshot = await page.locator('#game-canvas').screenshot();
      const dir = path.join(process.cwd(), 'e2e', 'screenshots');
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, 'codex-screen.png'), screenshot);
      
      expect(screenshot.length).toBeGreaterThan(1000);
    });

    test('capture starmap screen screenshot', async ({ page }) => {
      await page.goto('/');
      await waitForGameReady(page);
      
      await page.evaluate(() => {
        const canvas = document.getElementById('game-canvas');
        (canvas || document).dispatchEvent(new KeyboardEvent('keydown', { key: 'm', bubbles: true }));
      });
      await page.waitForTimeout(700);
      
      const screenshot = await page.locator('#game-canvas').screenshot();
      const dir = path.join(process.cwd(), 'e2e', 'screenshots');
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, 'starmap-screen.png'), screenshot);
      
      expect(screenshot.length).toBeGreaterThan(1000);
    });

    test('capture ship view screenshot', async ({ page }) => {
      await page.goto('/');
      await waitForGameReady(page);
      
      await page.evaluate(() => {
        const canvas = document.getElementById('game-canvas');
        (canvas || document).dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }));
      });
      await page.waitForTimeout(700);
      
      const screenshot = await page.locator('#game-canvas').screenshot();
      const dir = path.join(process.cwd(), 'e2e', 'screenshots');
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, 'ship-view.png'), screenshot);
      
      expect(screenshot.length).toBeGreaterThan(1000);
    });

    test('capture full page screenshot', async ({ page }) => {
      await page.goto('/');
      await waitForGameReady(page);
      
      const screenshot = await page.screenshot({ fullPage: true });
      const dir = path.join(process.cwd(), 'e2e', 'screenshots');
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, 'full-page.png'), screenshot);
      
      expect(screenshot.length).toBeGreaterThan(1000);
    });
  });

  test.describe('Canvas pixel statistics', () => {
    test.use({ viewport: { width: 1280, height: 720 } });

    test('canvas has expected pixel distribution (not all black)', async ({ page }) => {
      await page.goto('/');
      await waitForGameReady(page);
      
      const stats = await page.evaluate(() => {
        const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
        const ctx = canvas.getContext('2d')!;
        const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
        let bgPixels = 0;
        let nonBgPixels = 0;
        let brightPixels = 0;
        const total = data.data.length / 4;
        
        for (let i = 0; i < data.data.length; i += 4) {
          const r = data.data[i], g = data.data[i+1], b = data.data[i+2];
          const brightness = r + g + b;
          if (brightness < 30) bgPixels++;
          else if (brightness > 400) brightPixels++;
          else nonBgPixels++;
        }
        
        return {
          total,
          bgPercent: (bgPixels / total * 100).toFixed(1),
          nonBgPercent: (nonBgPixels / total * 100).toFixed(1),
          brightPercent: (brightPixels / total * 100).toFixed(1),
        };
      });
      
      console.log('Canvas pixel stats:', stats);
      
      // Should have some non-background pixels (game content)
      expect(parseFloat(stats.nonBgPercent) + parseFloat(stats.brightPercent)).toBeGreaterThan(1);
    });

    test('canvas has cyan accent color present (#00ccdd region)', async ({ page }) => {
      await page.goto('/');
      await waitForGameReady(page);
      
      // Check for cyan pixels (the game's primary accent color #00ccdd = rgb(0,204,221))
      const cyanPixelCount = await page.evaluate(() => {
        const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
        const ctx = canvas.getContext('2d')!;
        const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
        let count = 0;
        for (let i = 0; i < data.data.length; i += 4) {
          const r = data.data[i], g = data.data[i+1], b = data.data[i+2];
          // Cyan: low R, high G, high B
          if (r < 50 && g > 150 && b > 180) count++;
        }
        return count;
      });
      
      console.log(`Cyan pixels found: ${cyanPixelCount}`);
      // The game uses cyan as primary accent, should have some cyan pixels
      expect(cyanPixelCount).toBeGreaterThan(0);
    });
  });
});
