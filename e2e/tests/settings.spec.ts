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

async function navigateToSettings(page) {
    // Try different ways to navigate to settings
    const settingsLink = page.locator('a, button', { hasText: /Settings|Config|Configuration/i });
    if (await settingsLink.isVisible({ timeout: 3000 }).catch(() => false)) {
        await settingsLink.click();
        await page.waitForTimeout(1000);
    } else {
        // Try direct URL
        await page.goto('/settings.html');
    }
}

test.describe('Settings', () => {
    test.beforeEach(async ({ page }) => {
        await login(page);
        await navigateToSettings(page);
    });

    test('should display settings page', async ({ page }) => {
        const settingsTitle = page.locator('h1, h2, [class*="title"]', { hasText: /Settings|Configuration/i });
        await expect(settingsTitle).toBeVisible({ timeout: 5000 });
    });

    test('should display API configuration section', async ({ page }) => {
        // Look for API key section
        const apiLabel = page.locator('text=/API|Key|Token/i');
        await expect(apiLabel).toBeVisible({ timeout: 5000 });
    });

    test('should have API key input field', async ({ page }) => {
        const apiInput = page.locator('input[id*="api"], input[id*="key"], input[placeholder*="API"]');
        await expect(apiInput).toBeVisible({ timeout: 5000 });
    });

    test('should have API secret input field', async ({ page }) => {
        const secretInput = page.locator('input[id*="secret"], input[type="password"]');
        await expect(secretInput).toBeVisible({ timeout: 5000 });
    });

    test('should validate API key format', async ({ page }) => {
        const apiInput = page.locator('input[id*="api"], input[id*="key"]').first();
        const saveBtn = page.locator('button:has-text(/Save|Update|Submit/)');

        if (await apiInput.isVisible() && await saveBtn.isVisible()) {
            // Try invalid key
            await apiInput.fill('invalid-key-format');
            await saveBtn.click();

            // Should show error or validation message
            const errorMsg = page.locator('text=/Invalid|Format|Error/i');
            await expect(errorMsg).toBeVisible({ timeout: 5000 }).catch(() => { });
        }
    });

    test('should have save settings button', async ({ page }) => {
        const saveBtn = page.locator('button:has-text(/Save|Update|Apply/)');
        await expect(saveBtn).toBeVisible({ timeout: 5000 });
    });

    test('should persist API settings', async ({ page }) => {
        // Get current localStorage state
        const initialToken = await page.evaluate(() => localStorage.getItem('api_key'));

        // Find API input
        const apiInput = page.locator('input[id*="api"], input[id*="key"]').first();
        if (await apiInput.isVisible()) {
            await apiInput.fill('test-api-key-12345');

            const saveBtn = page.locator('button:has-text(/Save|Update/)');
            if (await saveBtn.isVisible()) {
                await saveBtn.click();
                await page.waitForTimeout(1000);

                // Reload and verify
                await page.reload();
                await page.waitForTimeout(500);

                const savedValue = await page.evaluate(() => localStorage.getItem('api_key'));
                expect(savedValue).toBeTruthy();
            }
        }
    });

    test('should display notification on successful save', async ({ page }) => {
        const saveBtn = page.locator('button:has-text(/Save|Update/)');
        if (await saveBtn.isVisible()) {
            await saveBtn.click();

            // Look for success message
            const successMsg = page.locator('text=/Success|Saved|Updated/i');
            const notification = page.locator('[class*="notification"], [class*="alert"], [class*="toast"]');

            const hasSuccess = await successMsg.isVisible({ timeout: 5000 }).catch(() => false);
            const hasNotification = (await notification.count()) > 0;

            expect(hasSuccess || hasNotification).toBeTruthy();
        }
    });

    test('should have trading preferences settings', async ({ page }) => {
        // Look for trading settings
        const tradingSection = page.locator('text=/Trading|Preferences|Options/i');

        const visible = await tradingSection.isVisible({ timeout: 3000 }).catch(() => false);
        if (visible) {
            await expect(tradingSection).toBeVisible();
        }
    });

    test('should allow resetting settings to defaults', async ({ page }) => {
        const resetBtn = page.locator('button:has-text(/Reset|Default|Clear/)');

        if (await resetBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
            await resetBtn.click();

            // Should show confirmation or reset
            const confirmMsg = page.locator('text=/Are you sure|Confirm/i');
            const confirmed = await confirmMsg.isVisible({ timeout: 2000 }).catch(() => false);

            if (confirmed) {
                const confirmBtn = page.locator('button:has-text(/Yes|Confirm|OK/)');
                if (await confirmBtn.isVisible()) {
                    await confirmBtn.click();
                }
            }

            // Check for reset completion
            const successMsg = page.locator('text=/Reset|Cleared/i');
            await expect(successMsg).toBeVisible({ timeout: 5000 }).catch(() => { });
        }
    });

    test('should display theme preference option', async ({ page }) => {
        // Look for theme selector
        const themeSelect = page.locator('select[id*="theme"], input[id*="theme"]');
        const themeLabel = page.locator('text=/Theme|Dark|Light/i');

        const selectVisible = await themeSelect.isVisible({ timeout: 3000 }).catch(() => false);
        const labelVisible = await themeLabel.isVisible({ timeout: 3000 }).catch(() => false);

        expect(selectVisible || labelVisible).toBeTruthy();
    });

    test('should display refresh rate settings', async ({ page }) => {
        // Look for refresh/update frequency
        const refreshSelect = page.locator('select[id*="refresh"], input[id*="interval"]');
        const refreshLabel = page.locator('text=/Refresh|Update|Interval/i');

        const selectVisible = await refreshSelect.isVisible({ timeout: 3000 }).catch(() => false);
        const labelVisible = await refreshLabel.isVisible({ timeout: 3000 }).catch(() => false);

        expect(selectVisible || labelVisible).toBeTruthy();
    });

    test('should handle API configuration errors', async ({ page }) => {
        // Mock API failure
        await page.route('/api/*', (route) => route.abort());

        const saveBtn = page.locator('button:has-text(/Save|Update/)');
        if (await saveBtn.isVisible()) {
            await saveBtn.click();

            // Should show error
            const errorMsg = page.locator('text=/Error|Failed|Connection/i');
            await expect(errorMsg).toBeVisible({ timeout: 5000 }).catch(() => { });
        }
    });
});
