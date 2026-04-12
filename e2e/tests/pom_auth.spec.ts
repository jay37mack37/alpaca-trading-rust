import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages/LoginPage';
import { DashboardPage } from '../pages/DashboardPage';

test.describe('Authentication with POM', () => {
    test('should login successfully with valid credentials', async ({ page }) => {
        const loginPage = new LoginPage(page);
        await loginPage.navigate();
        await loginPage.login('admin', 'admin123');

        await expect(page).toHaveURL(/.*index.html/);
        const dashboard = new DashboardPage(page);
        await expect(page.locator('header h1')).toHaveText(/Alpaca Trading Dashboard/);
    });

    test('should show error with invalid credentials', async ({ page }) => {
        const loginPage = new LoginPage(page);
        await loginPage.navigate();
        await loginPage.login('wrong', 'wrong');

        const error = await loginPage.getErrorMessage();
        expect(error).toContain('Invalid credentials');
    });

    test('should logout successfully', async ({ page }) => {
        const loginPage = new LoginPage(page);
        await loginPage.navigate();
        await loginPage.login('admin', 'admin123');

        const dashboard = new DashboardPage(page);
        await dashboard.logout();

        await expect(page).toHaveURL(/.*login.html/);
    });
});
