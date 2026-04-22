import { test, expect } from '@playwright/test';

test('Verification of UI functionality', async ({ page }) => {
  test.setTimeout(60000);

  await page.goto('http://localhost:3000');

  // Wait for the dashboard to load (market layout should be visible)
  await expect(page.locator('.market-layout')).toBeVisible({ timeout: 15000 });

  // 1. Verify dashboard loads
  await expect(page.locator('.eyebrow')).toContainText('AutoStonks');

  // 2. Test Tab Switching
  console.log('Switching to Agents tab');
  await page.click('button:has-text("Remodeling")');
  await expect(page.locator('h2:has-text("Remodeling & Setup")')).toBeVisible({ timeout: 5000 });

  console.log('Switching to Analytics tab');
  await page.click('button:has-text("Analytics")');
  await expect(page.locator('h2:has-text("Analytics Workspace")')).toBeVisible();

  console.log('Switching back to Market tab');
  await page.click('button:has-text("Market")');
  await expect(page.locator('.ticker-stage h2')).toBeVisible();

  // 3. Test symbol loading
  await page.fill('#market-symbol', 'AAPL');
  await page.click('button:has-text("Load ticker")');
  await expect(page.locator('.ticker-stage h2')).toHaveText('AAPL', { timeout: 10000 });

  // 4. Test Agent Creation
  await page.click('button:has-text("Remodeling")');
  await page.fill('input[id="agent-create-name"]', 'Test Agent');
  await page.fill('input[id="agent-create-symbols"]', 'AAPL');
  await page.click('button:has-text("Create and run")');

  await expect(page.locator('.banner.status')).toContainText('Test Agent created', { timeout: 10000 });

  // 5. Check if it's on the Workstation view now
  await expect(page.locator('h2:has-text("Multi-Strategy Workstation")')).toBeVisible({ timeout: 5000 });

  // 6. Stop the agent
  const stopButton = page.locator('button.btn-stop').first();
  await stopButton.click();
});
