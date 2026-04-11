import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages/login.page';
import { DashboardPage } from '../pages/dashboard.page';

test.describe('Authentication with POM', () => {
  let loginPage: LoginPage;
  let dashboardPage: DashboardPage;

  test.beforeEach(async ({ page }) => {
    loginPage = new LoginPage(page);
    dashboardPage = new DashboardPage(page);
    await loginPage.goto();
  });

  test('should login successfully with valid credentials', async ({ page }) => {
    await loginPage.login('admin', 'admin123');
    await dashboardPage.expectLoggedIn();

    const token = await page.evaluate(() => localStorage.getItem('token'));
    expect(token).toBeTruthy();
  });

  test('should logout successfully', async ({ page }) => {
    await loginPage.login('admin', 'admin123');
    await dashboardPage.expectLoggedIn();
    await dashboardPage.logout();
    await expect(page).toHaveURL(/.*login.html/);
  });
});
