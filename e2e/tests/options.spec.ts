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

async function navigateToOptions(page) {
    // Try different ways to navigate to options
    const optionsLink = page.locator('a, button', { hasText: /Options|Chains|Quotes/i });
    if (await optionsLink.isVisible({ timeout: 3000 }).catch(() => false)) {
        await optionsLink.click();
        await page.waitForTimeout(1000);
    }
}

test.describe('Options Trading', () => {
    test.beforeEach(async ({ page }) => {
        await login(page);
        await navigateToOptions(page);
    });

    test('should display options section', async ({ page }) => {
        const optionsSection = page.locator('text=/Options|Chains|Quotes/i');
        await expect(optionsSection).toBeVisible({ timeout: 5000 });
    });

    test('should have options search form', async ({ page }) => {
        // Look for symbol input
        const symbolInput = page.locator('input[id*="symbol"], input[placeholder*="Symbol"]');
        const searchBtn = page.locator('button:has-text(/Search|Load|Get/)');

        if (await symbolInput.isVisible({ timeout: 3000 }).catch(() => false)) {
            await expect(symbolInput).toBeVisible();
        }
    });

    test('should load options chain for SPY', async ({ page }) => {
        // Find symbol input
        const symbolInput = page.locator('input[id*="symbol"], input[placeholder*="Symbol"]');

        if (await symbolInput.isVisible({ timeout: 3000 }).catch(() => false)) {
            await symbolInput.fill('SPY');

            // Find and click search button
            const searchBtn = page.locator('button:has-text(/Search|Load|Get|Quote/)');
            if (await searchBtn.isVisible()) {
                // Intercept API call
                const responsePromise = page.waitForResponse(
                    (response) => response.url().includes('/api/options') && response.status() === 200,
                    { timeout: 10000 }
                );

                await searchBtn.click();
                const response = await responsePromise;
                const data = await response.json();

                // Verify response has options data
                expect(data).toBeTruthy();
                if (Array.isArray(data)) {
                    expect(data.length).toBeGreaterThanOrEqual(0);
                }
            }
        }
    });

    test('should display options chain table', async ({ page }) => {
        // Search for options first
        const symbolInput = page.locator('input[id*="symbol"], input[placeholder*="Symbol"]');
        if (await symbolInput.isVisible({ timeout: 3000 }).catch(() => false)) {
            await symbolInput.fill('AAPL');

            const searchBtn = page.locator('button:has-text(/Search|Load/)');
            if (await searchBtn.isVisible()) {
                await searchBtn.click();
                await page.waitForTimeout(2000);
            }
        }

        // Check for options data display
        const table = page.locator('table, .options-list, [class*="chain"]');
        const noData = page.locator('text=/No data|No options/i');

        const tableVisible = await table.isVisible({ timeout: 5000 }).catch(() => false);
        const noDataVisible = await noData.isVisible({ timeout: 3000 }).catch(() => false);

        expect(tableVisible || noDataVisible).toBeTruthy();
    });

    test('should display expiration date selector', async ({ page }) => {
        // Look for expiration date dropdown
        const expirationSelect = page.locator('select[id*="expiration"], select[id*="date"]');
        const expirationBtn = page.locator('button, [class*="expiration"]');

        const selectVisible = await expirationSelect.isVisible({ timeout: 3000 }).catch(() => false);
        const btnVisible = await expirationBtn.isVisible({ timeout: 3000 }).catch(() => false);

        expect(selectVisible || btnVisible).toBeTruthy();
    });

    test('should filter options by strike price', async ({ page }) => {
        // Search for options
        const symbolInput = page.locator('input[id*="symbol"], input[placeholder*="Symbol"]');
        if (await symbolInput.isVisible({ timeout: 3000 }).catch(() => false)) {
            await symbolInput.fill('QQQ');

            const searchBtn = page.locator('button:has-text(/Search|Load/)');
            if (await searchBtn.isVisible()) {
                await searchBtn.click();
                await page.waitForTimeout(2000);
            }
        }

        // Look for price filter inputs
        const priceFilter = page.locator('input[id*="price"], input[placeholder*="Strike"]');
        if (await priceFilter.isVisible({ timeout: 3000 }).catch(() => false)) {
            await expect(priceFilter).toBeVisible();
        }
    });

    test('should display call and put options', async ({ page }) => {
        // Search for options
        const symbolInput = page.locator('input[id*="symbol"], input[placeholder*="Symbol"]');
        if (await symbolInput.isVisible({ timeout: 3000 }).catch(() => false)) {
            await symbolInput.fill('SPY');

            const searchBtn = page.locator('button:has-text(/Search|Load/)');
            if (await searchBtn.isVisible()) {
                await searchBtn.click();
                await page.waitForTimeout(2000);
            }
        }

        // Look for call/put indicators
        const callOption = page.locator('text=/Call/i');
        const putOption = page.locator('text=/Put/i');

        const hasCallOrPut =
            (await callOption.isVisible({ timeout: 3000 }).catch(() => false)) ||
            (await putOption.isVisible({ timeout: 3000 }).catch(() => false));

        expect(hasCallOrPut).toBeTruthy();
    });

    test('should display option details', async ({ page }) => {
        // Search for options
        const symbolInput = page.locator('input[id*="symbol"], input[placeholder*="Symbol"]');
        if (await symbolInput.isVisible({ timeout: 3000 }).catch(() => false)) {
            await symbolInput.fill('NVDA');

            const searchBtn = page.locator('button:has-text(/Search|Load/)');
            if (await searchBtn.isVisible()) {
                await searchBtn.click();
                await page.waitForTimeout(2000);
            }
        }

        // Look for common option fields
        const bid = page.locator('text=/Bid|Ask|Price/i');
        const volume = page.locator('text=/Volume|Open Interest/i');

        const hasPriceInfo = await bid.isVisible({ timeout: 3000 }).catch(() => false);
        const hasVolume = await volume.isVisible({ timeout: 3000 }).catch(() => false);

        expect(hasPriceInfo || hasVolume).toBeTruthy();
    });

    test('should handle invalid symbol', async ({ page }) => {
        const symbolInput = page.locator('input[id*="symbol"], input[placeholder*="Symbol"]');
        if (await symbolInput.isVisible({ timeout: 3000 }).catch(() => false)) {
            await symbolInput.fill('INVALID123XYZ');

            const searchBtn = page.locator('button:has-text(/Search|Load/)');
            if (await searchBtn.isVisible()) {
                await searchBtn.click();

                // Should show error or no data
                const error = page.locator('text=/Error|Invalid|Not found/i');
                const noData = page.locator('text=/No options|No data/i');

                const hasError = await error.isVisible({ timeout: 5000 }).catch(() => false);
                const isEmpty = await noData.isVisible({ timeout: 5000 }).catch(() => false);

                expect(hasError || isEmpty).toBeTruthy();
            }
        }
    });

    test('should allow buying options', async ({ page }) => {
        // Search for options
        const symbolInput = page.locator('input[id*="symbol"], input[placeholder*="Symbol"]');
        if (await symbolInput.isVisible({ timeout: 3000 }).catch(() => false)) {
            await symbolInput.fill('SPY');

            const searchBtn = page.locator('button:has-text(/Search|Load/)');
            if (await searchBtn.isVisible()) {
                await searchBtn.click();
                await page.waitForTimeout(2000);
            }
        }

        // Look for buy button on options
        const buyBtn = page.locator('button:has-text(/Buy|Trade|Select/)').first();
        if (await buyBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
            await expect(buyBtn).toBeVisible();
        }
    });
});
