import { test, expect } from '@playwright/test';

test('Verification of UI functionality', async ({ page }) => {
  test.setTimeout(60000);

  await page.goto('http://localhost:3000');

  // Wait for the dashboard to load (market layout should be visible)
  await expect(page.locator('.market-layout')).toBeVisible({ timeout: 15000 });

  // 1. Verify dashboard loads
  await expect(page.locator('.eyebrow')).toContainText('AutoStonks');

  // Screenshot: Market tab loaded
  await page.screenshot({ path: 'test-results/01-market-tab.png', fullPage: true });

  // 2. Test Tab Switching
  console.log('Switching to Workstation tab');
  await page.click('button:has-text("Workstation")');
  await expect(page.locator('.workstation-page')).toBeVisible({ timeout: 5000 });
  await page.screenshot({ path: 'test-results/02-workstation-tab.png', fullPage: true });

  console.log('Switching to Analytics tab');
  await page.click('button:has-text("Analytics")');
  await expect(page.locator('.analytics-page')).toBeVisible();
  await page.screenshot({ path: 'test-results/03-analytics-tab.png', fullPage: true });

  console.log('Switching back to Market tab');
  await page.click('button:has-text("Market")');
  await expect(page.locator('.ticker-stage h2')).toBeVisible();

  // 3. Test symbol loading
  await page.fill('#market-symbol', 'AAPL');
  await page.click('button:has-text("Load ticker")');
  await expect(page.locator('.ticker-stage h2')).toHaveText('AAPL', { timeout: 10000 });
  await page.screenshot({ path: 'test-results/04-aapl-loaded.png', fullPage: true });
});