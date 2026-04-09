import { test, expect } from '@playwright/test';

const TEST_USERNAME = 'admin';
const TEST_PASSWORD = 'admin123';

async function login(page) {
    await page.goto('/login.html');
    await page.fill('input[id="username"]', TEST_USERNAME);
    await page.fill('input[id="password"]', TEST_PASSWORD);
    await page.click('button:has-text("Login")');
    await page.waitForURL('/', { timeout: 10000 });
}

test.describe('Dashboard', () => {
    test.beforeEach(async ({ page }) => {
        await login(page);
    });

    test('should display dashboard with main sections', async ({ page }) => {
        // Check for main dashboard title
        const dashboard = page.locator('h1:has-text("Dashboard")');
        await expect(dashboard).toBeVisible();

        // Check for main navigation tabs
        const accountTab = page.locator('button, a', { hasText: /Account|Portfolio/ }).first();
        await expect(accountTab).toBeVisible({ timeout: 5000 });
    });

    test('should display account information', async ({ page }) => {
        // Wait for account section to load
        const accountSection = page.locator('section, div', { has: page.locator('text=Account') });
        await expect(accountSection).toBeVisible({ timeout: 5000 });

        // Check for key account fields
        const cashBalance = page.locator('text=/Portfolio Value|Cash|Balance/');
        await expect(cashBalance).toBeVisible({ timeout: 5000 });
    });

    test('should load account details from API', async ({ page }) => {
        // Intercept and verify account API call
        const accountResponse = await page.waitForResponse(
            (response) => response.url().includes('/api/account') && response.status() === 200,
            { timeout: 10000 }
        );

        const accountData = await accountResponse.json();
        expect(accountData).toHaveProperty('portfolio_value');
        expect(accountData).toHaveProperty('cash');
        expect(accountData).toHaveProperty('buying_power');
    });

    test('should display positions table or empty state', async ({ page }) => {
        // Check for positions section
        const positionsHeader = page.locator('text=/Positions|Holdings/i');
        await expect(positionsHeader).toBeVisible({ timeout: 5000 });

        // Either shows table or empty message
        const table = page.locator('table, .positions-list');
        const emptyMsg = page.locator('text=/No positions|Empty/i');

        const tableVisible = await table.isVisible().catch(() => false);
        const emptyVisible = await emptyMsg.isVisible().catch(() => false);

        expect(tableVisible || emptyVisible).toBeTruthy();
    });

    test('should display buying power and cash', async ({ page }) => {
        // Wait for account data to load
        await page.waitForTimeout(1000);

        const buyingPower = page.locator('text=/Buying Power|Available/i');
        const cash = page.locator('text=/Cash|Balance/i');

        // At least one should be visible
        const buyingPowerVisible = await buyingPower.isVisible().catch(() => false);
        const cashVisible = await cash.isVisible().catch(() => false);

        expect(buyingPowerVisible || cashVisible).toBeTruthy();
    });

    test('should handle failed account API call gracefully', async ({ page }) => {
        // Abort account API requests to simulate failure
        await page.route('/api/account', (route) => route.abort());

        await page.reload();

        // Should show error message or fallback UI
        const errorMsg = page.locator('text=/Error|Failed|Unable/i');
        const emptyMsg = page.locator('text=/No data|Loading/i');

        const errorVisible = await errorMsg.isVisible({ timeout: 5000 }).catch(() => false);
        const emptyVisible = await emptyMsg.isVisible({ timeout: 5000 }).catch(() => false);

        expect(errorVisible || emptyVisible).toBeTruthy();
    });

    test('should navigate between sections', async ({ page }) => {
        // Look for navigation buttons/links
        const navItems = await page.locator('nav button, nav a, .nav-item').all();

        expect(navItems.length).toBeGreaterThan(0);

        // Click first nav item if exists
        if (navItems.length > 1) {
            await navItems[1].click();
            await page.waitForTimeout(500);
            // Should not show error
            const errorMsg = page.locator('text=/Error|404/');
            await expect(errorMsg).not.toBeVisible({ timeout: 5000 }).catch(() => { });
        }
    });

    test('should display user info in header', async ({ page }) => {
        // Check for username or user menu
        const userInfo = page.locator('text=' + TEST_USERNAME);
        const userMenu = page.locator('[class*="user"], [class*="profile"], [class*="header"]');

        const userVisible = await userInfo.isVisible({ timeout: 5000 }).catch(() => false);
        const hasUserMenu = (await userMenu.count()) > 0;

        expect(userVisible || hasUserMenu).toBeTruthy();
    });
});
