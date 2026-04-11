import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages/login.page';
import { AnalyticsPage } from '../pages/analytics.page';

test.describe('Analytics', () => {
  let loginPage: LoginPage;
  let analyticsPage: AnalyticsPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    analyticsPage = new AnalyticsPage(page);
    await loginPage.goto();
    await loginPage.login('admin', 'admin123');
    await analyticsPage.goto();
  });

  test('should add symbol to watchlist', async () => {
    await analyticsPage.addWatchlist('AAPL');
    await analyticsPage.expectSymbolInWatchlist('AAPL');
  });

  test('should display pattern checkboxes', async ({ page }) => {
    const patterns = page.locator('#pattern-checkboxes label');
    await expect(patterns.first()).toBeVisible();
  });
});
