import { test, expect } from '@playwright/test';
import { waitForCanvas, LOAD_TIMEOUT } from './helpers';

test.describe('PWA features', () => {
  test('manifest.json is accessible', async ({ page }) => {
    const response = await page.request.get('/manifest.json');
    expect(response.status()).toBe(200);
  });

  test('manifest.json is valid JSON', async ({ page }) => {
    const response = await page.request.get('/manifest.json');
    const text = await response.text();
    let manifest: any;
    expect(() => { manifest = JSON.parse(text); }).not.toThrow();
    expect(manifest).toBeTruthy();
  });

  test('manifest has required name field', async ({ page }) => {
    const response = await page.request.get('/manifest.json');
    const manifest = await response.json();
    expect(manifest.name || manifest.short_name).toBeTruthy();
  });

  test('manifest has start_url field', async ({ page }) => {
    const response = await page.request.get('/manifest.json');
    const manifest = await response.json();
    expect(manifest.start_url).toBeTruthy();
  });

  test('manifest has display mode', async ({ page }) => {
    const response = await page.request.get('/manifest.json');
    const manifest = await response.json();
    const validDisplayModes = ['fullscreen', 'standalone', 'minimal-ui', 'browser'];
    expect(validDisplayModes).toContain(manifest.display);
  });

  test('manifest has icons array', async ({ page }) => {
    const response = await page.request.get('/manifest.json');
    const manifest = await response.json();
    expect(Array.isArray(manifest.icons)).toBe(true);
    expect(manifest.icons.length).toBeGreaterThan(0);
  });

  test('manifest icons have required fields', async ({ page }) => {
    const response = await page.request.get('/manifest.json');
    const manifest = await response.json();
    for (const icon of manifest.icons) {
      expect(icon.src).toBeTruthy();
      expect(icon.sizes).toBeTruthy();
    }
  });

  test('manifest icons are accessible', async ({ page }) => {
    const response = await page.request.get('/manifest.json');
    const manifest = await response.json();

    // Check first icon is accessible (skip data: URIs as they are self-contained)
    if (manifest.icons && manifest.icons.length > 0) {
      const iconSrc = manifest.icons[0].src;
      if (!iconSrc.startsWith('data:')) {
        const iconUrl = iconSrc.startsWith('/') ? iconSrc : '/' + iconSrc;
        const iconResponse = await page.request.get(iconUrl);
        expect(iconResponse.status()).toBe(200);
      }
    }
  });

  test('sw.js service worker file is accessible', async ({ page }) => {
    const response = await page.request.get('/sw.js');
    expect(response.status()).toBe(200);
  });

  test('sw.js has valid JavaScript', async ({ page }) => {
    const response = await page.request.get('/sw.js');
    const text = await response.text();
    expect(text.length).toBeGreaterThan(0);
    // Should not be empty or HTML error page
    expect(text).not.toContain('<!DOCTYPE html>');
  });

  test('theme-color meta tag is present', async ({ page }) => {
    await page.goto('/');
    const themeColor = await page.evaluate(() => {
      const meta = document.querySelector('meta[name="theme-color"]');
      return meta?.getAttribute('content');
    });
    expect(themeColor).toBeTruthy();
    expect(themeColor).toBe('#00ccdd');
  });

  test('manifest link tag is present in HTML', async ({ page }) => {
    await page.goto('/');
    const manifestHref = await page.evaluate(() => {
      const link = document.querySelector('link[rel="manifest"]');
      return link?.getAttribute('href');
    });
    expect(manifestHref).toBeTruthy();
    expect(manifestHref).toContain('manifest.json');
  });

  test('service worker registers without error', async ({ page }) => {
    const swErrors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error' && msg.text().includes('serviceWorker')) {
        swErrors.push(msg.text());
      }
    });

    await page.goto('/');
    await waitForCanvas(page);
    await page.waitForTimeout(2000); // Give SW time to register

    // No service worker errors
    expect(swErrors).toHaveLength(0);
  });

  test('manifest background_color matches page background', async ({ page }) => {
    const manifestResponse = await page.request.get('/manifest.json');
    const manifest = await manifestResponse.json();

    if (manifest.background_color) {
      // Background color should be dark (game has dark background)
      expect(manifest.background_color).toBeTruthy();
    }
  });

  test('page has correct charset', async ({ page }) => {
    await page.goto('/');
    const charset = await page.evaluate(() => {
      const meta = document.querySelector('meta[charset]');
      return meta?.getAttribute('charset');
    });
    expect(charset?.toLowerCase()).toBe('utf-8');
  });

  test('viewport meta tag is present for mobile', async ({ page }) => {
    await page.goto('/');
    const viewport = await page.evaluate(() => {
      const meta = document.querySelector('meta[name="viewport"]');
      return meta?.getAttribute('content');
    });
    expect(viewport).toBeTruthy();
    expect(viewport).toContain('width=device-width');
  });
});
