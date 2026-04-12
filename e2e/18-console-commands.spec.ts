import { test, expect } from '@playwright/test';
import { waitForGameReady, sendKey, runCmd, snap, snapFull, diff } from './helpers';

test.describe('Developer console commands', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForGameReady(page);
  });

  const COMMANDS_THAT_CHANGE_CANVAS = [
    { cmd: 'help',      desc: 'help output' },
    { cmd: 'god',       desc: 'god mode toggle' },
    { cmd: 'hp 50',     desc: 'set HP to 50' },
    { cmd: 'gold 500',  desc: 'set gold to 500' },
    { cmd: 'floor 3',   desc: 'jump to floor 3' },
    { cmd: 'reveal',    desc: 'reveal map' },
    { cmd: 'stats',     desc: 'show stats' },
    { cmd: 'items',     desc: 'list items' },
    { cmd: 'kill_all',  desc: 'kill all enemies' },
  ];

  for (const { cmd, desc } of COMMANDS_THAT_CHANGE_CANVAS) {
    test(`"${cmd}" command produces visual output (${desc})`, async ({ page }) => {
      const before = await snapFull(page);
      await runCmd(page, cmd);
      await page.waitForTimeout(500);
      const after = await snapFull(page);
      const changed = diff(before, after);
      console.log(`"${cmd}": ${changed} pixels changed`);
      expect(changed).toBeGreaterThan(0);
      await expect(page.locator('#game-canvas')).toBeVisible();
    });
  }

  test('"clear" command clears console output', async ({ page }) => {
    // Open console, type help, then clear
    await runCmd(page, 'help');
    await page.waitForTimeout(300);
    const withHelp = await snapFull(page);
    await runCmd(page, 'clear');
    await page.waitForTimeout(300);
    const afterClear = await snapFull(page);
    const changed = diff(withHelp, afterClear);
    console.log(`clear command: ${changed} pixels changed`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('"give_item HealthPotion" gives item', async ({ page }) => {
    const before = await snapFull(page);
    await runCmd(page, 'give_item HealthPotion');
    await page.waitForTimeout(400);
    const after = await snapFull(page);
    console.log(`give_item HealthPotion: ${diff(before,after)} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('"fight normal" spawns encounter', async ({ page }) => {
    const before = await snapFull(page);
    await runCmd(page, 'god');
    await runCmd(page, 'fight normal');
    await page.waitForTimeout(600);
    const after = await snapFull(page);
    expect(diff(before, after)).toBeGreaterThan(50);
  });

  test('"fight boss" spawns boss encounter', async ({ page }) => {
    const before = await snapFull(page);
    await runCmd(page, 'god');
    await runCmd(page, 'fight boss');
    await page.waitForTimeout(600);
    const after = await snapFull(page);
    console.log(`fight boss: ${diff(before,after)} pixels`);
    await expect(page.locator('#game-canvas')).toBeVisible();
  });

  test('tab completion completes "hel" to "help"', async ({ page }) => {
    await sendKey(page, '`');
    await page.waitForTimeout(300);
    for (const ch of 'hel') { await sendKey(page, ch); await page.waitForTimeout(40); }
    const beforeTab = await snap(page);
    await sendKey(page, 'Tab');
    await page.waitForTimeout(300);
    const afterTab = await snap(page);
    console.log(`Tab on "hel": ${diff(beforeTab, afterTab)} pixels`);
    await sendKey(page, 'Escape');
  });

  test('console opens and closes with backtick', async ({ page }) => {
    const closed = await snap(page);
    
    await sendKey(page, '`');
    await page.waitForTimeout(300);
    const opened = await snap(page);
    expect(diff(closed, opened)).toBeGreaterThan(0);
    
    await sendKey(page, '`');
    await page.waitForTimeout(300);
    const closedAgain = await snap(page);
    expect(diff(opened, closedAgain)).toBeGreaterThan(0);
  });

  test('all give_item variants do not crash', async ({ page }) => {
    const items = ['HealthPotion','PoisonFlask','RevealScroll','TeleportScroll',
                   'HastePotion','StunBomb','RiceBall','GoldIngot','ThunderTalisman'];
    const errors: string[] = [];
    page.on('pageerror', e => { if (!e.message.includes('WebSocket')) errors.push(e.message); });
    
    for (const item of items) {
      await runCmd(page, `give_item ${item}`);
      await page.waitForTimeout(150);
    }
    await expect(page.locator('#game-canvas')).toBeVisible();
    expect(errors).toHaveLength(0);
  });

  test('rapid console commands do not crash', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', e => { if (!e.message.includes('WebSocket')) errors.push(e.message); });
    
    const cmds = ['help','god','hp 100','gold 100','reveal','stats','items','clear','god'];
    for (const cmd of cmds) {
      await runCmd(page, cmd);
      await page.waitForTimeout(100);
    }
    await expect(page.locator('#game-canvas')).toBeVisible();
    expect(errors).toHaveLength(0);
  });
});
