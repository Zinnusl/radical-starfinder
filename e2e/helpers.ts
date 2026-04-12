import { Page } from '@playwright/test';

export const GAME_URL = '/';
export const LOAD_TIMEOUT = 20000;

/** Wait for the canvas element to appear in the DOM */
export async function waitForCanvas(page: Page): Promise<void> {
  await page.waitForSelector('#game-canvas', { timeout: LOAD_TIMEOUT });
}

/** Wait for WASM bindings to be available on window */
export async function waitForWasmInit(page: Page): Promise<void> {
  await page.waitForFunction(() => {
    return typeof (window as any).wasmBindings !== 'undefined';
  }, { timeout: LOAD_TIMEOUT });
}

/** Wait for the game canvas to appear and have non-background pixels rendered */
export async function waitForGameReady(page: Page): Promise<void> {
  await waitForCanvas(page);
  await page.waitForFunction(() => {
    const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
    if (!canvas || canvas.width === 0) return false;
    const ctx = canvas.getContext('2d');
    if (!ctx) return false;
    const data = ctx.getImageData(0, 0, Math.min(canvas.width, 100), Math.min(canvas.height, 100));
    for (let i = 0; i < data.data.length; i += 4) {
      if (data.data[i] > 20 || data.data[i+1] > 20 || data.data[i+2] > 30) return true;
    }
    return false;
  }, { timeout: LOAD_TIMEOUT });
}

/** Send a keyboard key to the game canvas */
export async function sendKey(page: Page, key: string): Promise<void> {
  await page.evaluate((k) => {
    const canvas = document.getElementById('game-canvas');
    (canvas || document).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true }));
  }, key);
}

/** Send multiple keys sequentially with a delay between each */
export async function sendKeys(page: Page, keys: string[], delayMs = 100): Promise<void> {
  for (const key of keys) {
    await sendKey(page, key);
    if (delayMs > 0) await page.waitForTimeout(delayMs);
  }
}

/** Type a string character by character */
export async function typeStr(page: Page, text: string, delay = 40): Promise<void> {
  for (const ch of text) {
    await sendKey(page, ch);
    await page.waitForTimeout(delay);
  }
}

/** Alias for typeStr with different name used by some tests */
export async function typeString(page: Page, text: string, delayMs = 40): Promise<void> {
  await typeStr(page, text, delayMs);
}

/** Open debug console, type a command, execute it, and close console */
export async function runCmd(page: Page, cmd: string): Promise<void> {
  await sendKey(page, '`');
  await page.waitForTimeout(300);
  await typeStr(page, cmd);
  await page.waitForTimeout(100);
  await sendKey(page, 'Enter');
  await page.waitForTimeout(300);
  await sendKey(page, 'Escape');
  await page.waitForTimeout(200);
}

/** Alias for runCmd with different name used by some tests */
export async function runConsoleCommand(page: Page, cmd: string): Promise<void> {
  await runCmd(page, cmd);
}

/** Dismiss class-selection overlay and wait for Starmap */
export async function setupGame(page: Page): Promise<void> {
  await waitForGameReady(page);
  await sendKey(page, 'Enter');
  await page.waitForTimeout(400);
}

/** From the Starmap, press 's' to enter ShipInterior */
export async function enterShipInterior(page: Page): Promise<void> {
  await sendKey(page, 's');
  await page.waitForTimeout(400);
}

/** Navigate from any start state to dungeon explore mode */
export async function enterExploreMode(page: Page): Promise<void> {
  await sendKey(page, 'Escape');
  await page.waitForTimeout(300);
  await sendKey(page, 'Enter');
  await page.waitForTimeout(500);
  await sendKey(page, 'e');
  await page.waitForTimeout(800);
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
      if (data.data[i] > 20 || data.data[i+1] > 20 || data.data[i+2] > 30) return true;
    }
    return false;
  });
}

/** Check if at least 1% of canvas pixels are non-background */
export async function canvasNonBlank(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
    if (!canvas) return false;
    const ctx = canvas.getContext('2d')!;
    const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
    let nonBgPixels = 0;
    for (let i = 0; i < data.data.length; i += 4) {
      if (data.data[i] > 20 || data.data[i+1] > 20 || data.data[i+2] > 30) nonBgPixels++;
    }
    return nonBgPixels / (data.data.length / 4) > 0.01;
  });
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

/** Get a full canvas snapshot as an array */
export async function getCanvasSnapshot(page: Page): Promise<Uint8ClampedArray> {
  return page.evaluate(() => {
    const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
    const ctx = canvas.getContext('2d')!;
    return Array.from(ctx.getImageData(0, 0, canvas.width, canvas.height).data) as unknown as Uint8ClampedArray;
  });
}

/** Get the average color of the entire canvas */
export async function getAverageColor(page: Page): Promise<{r: number, g: number, b: number}> {
  return page.evaluate(() => {
    const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
    const ctx = canvas.getContext('2d')!;
    const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
    let r = 0, g = 0, b = 0, count = 0;
    for (let i = 0; i < data.length; i += 4) {
      r += data[i]; g += data[i+1]; b += data[i+2]; count++;
    }
    return { r: r/count, g: g/count, b: b/count };
  });
}

/** Get the dominant (most common) color on the canvas, quantized to 32-step buckets */
export async function getCanvasDominantColor(page: Page): Promise<{r: number, g: number, b: number}> {
  return page.evaluate(() => {
    const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
    const ctx = canvas.getContext('2d')!;
    const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
    const counts = new Map<string, {r: number, g: number, b: number, count: number}>();
    for (let i = 0; i < data.data.length; i += 4) {
      const r = Math.round(data.data[i] / 32) * 32;
      const g = Math.round(data.data[i+1] / 32) * 32;
      const b = Math.round(data.data[i+2] / 32) * 32;
      const key = `${r},${g},${b}`;
      const existing = counts.get(key) || { r, g, b, count: 0 };
      existing.count++;
      counts.set(key, existing);
    }
    let max = { r: 0, g: 0, b: 0, count: 0 };
    for (const v of counts.values()) {
      if (v.count > max.count) max = v;
    }
    return { r: max.r, g: max.g, b: max.b };
  });
}

/** Capture a region of the canvas as a pixel array */
export async function snap(page: Page, w = 800, h = 600, x = 0, y = 0): Promise<number[]> {
  return page.evaluate(({ x, y, w, h }) => {
    const c = document.getElementById('game-canvas') as HTMLCanvasElement;
    return Array.from(c.getContext('2d')!.getImageData(x, y, Math.min(w, c.width), Math.min(h, c.height)).data);
  }, { x, y, w, h });
}

/** Capture the full canvas as a pixel array */
export async function snapFull(page: Page): Promise<number[]> {
  return page.evaluate(() => {
    const c = document.getElementById('game-canvas') as HTMLCanvasElement;
    return Array.from(c.getContext('2d')!.getImageData(0, 0, c.width, c.height).data);
  });
}

/** Capture a full canvas as a pixel array (alias for captureCanvas) */
export async function captureCanvas(page: Page): Promise<number[]> {
  return page.evaluate(() => {
    const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
    const ctx = canvas.getContext('2d')!;
    const w = Math.min(canvas.width, 600);
    const h = Math.min(canvas.height, 400);
    return Array.from(ctx.getImageData(0, 0, w, h).data);
  });
}

/** Compute a fast hash of the full canvas (single number, very fast) */
export async function snapHash(page: Page): Promise<number> {
  return page.evaluate(() => {
    const c = document.getElementById('game-canvas') as HTMLCanvasElement;
    const ctx = c.getContext('2d')!;
    const d = ctx.getImageData(0, 0, c.width, c.height).data;
    let h = 0;
    for (let i = 0; i < d.length; i += 4) {
      h = (Math.imul(h, 31) + ((d[i] << 16) | (d[i + 1] << 8) | d[i + 2])) | 0;
    }
    return h;
  });
}

/** Count pixels that differ between two pixel arrays */
export function diff(a: number[], b: number[], t = 8): number {
  let n = 0;
  for (let i = 0; i < a.length; i += 4)
    if (Math.abs(a[i] - b[i]) > t || Math.abs(a[i+1] - b[i+1]) > t || Math.abs(a[i+2] - b[i+2]) > t) n++;
  return n;
}

/** Count different pixels (alias for diff with different default threshold) */
export function countDifferentPixels(a: number[], b: number[], threshold = 10): number {
  return diff(a, b, threshold);
}

/** Count different pixels (alias) */
export function pixelsDiffer(a: number[], b: number[], threshold = 10): number {
  return diff(a, b, threshold);
}

/** Store a full-canvas pixel snapshot in page memory under a given key name */
export async function storeSnapshot(page: Page, name: string): Promise<void> {
  await page.evaluate((n) => {
    const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
    const ctx = canvas.getContext('2d')!;
    (window as any)[`__snap_${n}`] = ctx.getImageData(0, 0, canvas.width, canvas.height).data.slice();
  }, name);
}

/** Count pixels changed since a stored snapshot */
export async function countChangedPixels(page: Page, name: string, threshold = 5): Promise<number> {
  return page.evaluate(([n, thresh]: [string, number]) => {
    const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
    const ctx = canvas.getContext('2d')!;
    const prev = (window as any)[`__snap_${n}`] as Uint8ClampedArray | undefined;
    if (!prev) return 0;
    const curr = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
    let count = 0;
    for (let i = 0; i < Math.min(curr.length, prev.length); i += 4) {
      if (Math.abs(curr[i] - prev[i]) > thresh ||
          Math.abs(curr[i+1] - prev[i+1]) > thresh ||
          Math.abs(curr[i+2] - prev[i+2]) > thresh) count++;
    }
    return count;
  }, [name, threshold]);
}

/** Poll until the canvas differs from the stored snapshot by at least minPixels */
export async function waitForCanvasChange(
  page: Page,
  name: string,
  minPixels = 1,
  timeout = 8000
): Promise<void> {
  await page.waitForFunction(
    ([n, thresh, minCount]: [string, number, number]) => {
      const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
      const ctx = canvas?.getContext('2d');
      if (!ctx) return false;
      const prev = (window as any)[`__snap_${n}`] as Uint8ClampedArray | undefined;
      if (!prev) return false;
      const curr = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
      let count = 0;
      for (let i = 0; i < Math.min(curr.length, prev.length); i += 4) {
        if (Math.abs(curr[i] - prev[i]) > thresh ||
            Math.abs(curr[i+1] - prev[i+1]) > thresh ||
            Math.abs(curr[i+2] - prev[i+2]) > thresh) {
          count++;
          if (count >= minCount) return true;
        }
      }
      return false;
    },
    [name, 5, minPixels] as [string, number, number],
    { timeout }
  );
}

/** Try moving in a direction until the canvas hash changes */
export async function moveUntilChanged(page: Page, key: string, maxTries = 15): Promise<boolean> {
  for (let i = 0; i < maxTries; i++) {
    const before = await snapHash(page);
    await sendKey(page, key);
    await page.waitForTimeout(120);
    const after = await snapHash(page);
    if (before !== after) return true;
  }
  return false;
}
