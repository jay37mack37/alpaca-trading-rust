import { test, expect } from '@playwright/test';

test.describe('Authentication', () => {
    test.beforeEach(async ({ page }) => {
        await page.goto('/login.html');
    });

    test('should display login form', async ({ page }) => {
        const usernameInput = page.locator('input[id="username"]');
        const passwordInput = page.locator('input[id="password"]');
        const loginButton = page.locator('button:has-text("Login")');

        await expect(usernameInput).toBeVisible();
        await expect(passwordInput).toBeVisible();
        await expect(loginButton).toBeVisible();
    });

    test('should reject invalid credentials', async ({ page }) => {
        await page.fill('input[id="username"]', 'invaliduser');
        await page.fill('input[id="password"]', 'wrongpassword');
        await page.click('button:has-text("Login")');

        // Wait for error message
        const errorMsg = page.locator('text=Invalid credentials');
        await expect(errorMsg).toBeVisible({ timeout: 5000 });
    });

    test('should accept valid credentials', async ({ page }) => {
        await page.fill('input[id="username"]', 'admin');
        await page.fill('input[id="password"]', 'admin123');
        await page.click('button:has-text("Login")');

        // Should redirect to dashboard
        await page.waitForURL('/', { timeout: 10000 });
        const dashboardTitle = page.locator('h1:has-text("Dashboard")');
        await expect(dashboardTitle).toBeVisible();
    });

    test('should persist auth token in localStorage', async ({ page }) => {
        await page.fill('input[id="username"]', 'admin');
        await page.fill('input[id="password"]', 'admin123');
        await page.click('button:has-text("Login")');

        await page.waitForURL('/', { timeout: 10000 });

        // Check localStorage
        const token = await page.evaluate(() => localStorage.getItem('token'));
        expect(token).toBeTruthy();
        expect(token).toMatch(/^[A-Za-z0-9._-]+$/); // JWT format validation
    });

    test('should logout successfully', async ({ page }) => {
        // First login
        await page.goto('/login.html');
        await page.fill('input[id="username"]', 'admin');
        await page.fill('input[id="password"]', 'admin123');
        await page.click('button:has-text("Login")');
        await page.waitForURL('/', { timeout: 10000 });

        // Find and click logout button
        const logoutButton = page.locator('button:has-text("Logout")');
        await expect(logoutButton).toBeVisible();
        await logoutButton.click();

        // Should redirect to login
        await page.waitForURL('/login.html', { timeout: 5000 });
        await expect(page.locator('input[id="username"]')).toBeVisible();
    });

    test('should prevent access to dashboard without auth', async ({ page }) => {
        await page.goto('/');
        // Should redirect to login if not authenticated
        await page.waitForURL('/login.html', { timeout: 5000 });
    });

    test('should handle session timeout', async ({ page }) => {
        // Login first
        await page.goto('/login.html');
        await page.fill('input[id="username"]', 'admin');
        await page.fill('input[id="password"]', 'admin123');
        await page.click('button:has-text("Login")');
        await page.waitForURL('/', { timeout: 10000 });

        // Clear token to simulate session timeout
        await page.evaluate(() => localStorage.removeItem('token'));
        await page.reload();

        // Should redirect to login
        await page.waitForURL('/login.html', { timeout: 5000 });
    });
});
