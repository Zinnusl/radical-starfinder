import { Page, expect } from '@playwright/test';

export const GAME_URL = '/';
export const LOAD_TIMEOUT = 15000; // 15s for WASM to load

/** Wait for the game canvas to appear and WASM to initialize */
export async function waitForGameReady(page: Page): Promise<void> {
  // Wait for canvas to exist
  await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });
  
  // Wait for loading indicator to disappear (or canvas to get pixels)
  await page.waitForFunction(() => {
    const loading = document.getElementById('loading');
    // Loading is hidden when game starts
    return !loading || loading.style.display === 'none' || loading.textContent?.includes('Failed') ||
      // Or check canvas has content
      (() => {
        const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
        if (!canvas || canvas.width === 0) return false;
        const ctx = canvas.getContext('2d');
        if (!ctx) return false;
        const data = ctx.getImageData(0, 0, Math.min(canvas.width, 100), Math.min(canvas.height, 100));
        // Check if any pixel has non-background color (not just #060612)
        for (let i = 0; i < data.data.length; i += 4) {
          const r = data.data[i], g = data.data[i+1], b = data.data[i+2];
          // Background is #060612 (6, 6, 18) - check if any pixel is different
          if (r > 20 || g > 20 || b > 30) return true;
        }
        return false;
      })();
  }, { timeout: LOAD_TIMEOUT });
}

/** Send a keyboard key to the game canvas */
export async function sendKey(page: Page, key: string): Promise<void> {
  const canvas = page.locator('#game-canvas');
  await canvas.dispatchEvent('keydown', { key, bubbles: true });
}

/** Get pixel data from the canvas at a specific position */
export async function getCanvasPixel(page: Page, x: number, y: number): Promise<{r: number, g: number, b: number, a: number}> {
  return page.evaluate(({x, y}) => {
    const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
    const ctx = canvas.getContext('2d')!;
    const pixel = ctx.getImageData(x, y, 1, 1).data;
    return { r: pixel[0], g: pixel[1], b: pixel[2], a: pixel[3] };
  }, {x, y});
}

/** Check if canvas has any non-background pixels */
export async function canvasHasContent(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
    if (!canvas || canvas.width === 0) return false;
    const ctx = canvas.getContext('2d');
    if (!ctx) return false;
    const w = Math.min(canvas.width, 200);
    const h = Math.min(canvas.height, 200);
    const data = ctx.getImageData(0, 0, w, h);
    for (let i = 0; i < data.data.length; i += 4) {
      const r = data.data[i], g = data.data[i+1], b = data.data[i+2];
      if (r > 20 || g > 20 || b > 30) return true;
    }
    return false;
  });
}
