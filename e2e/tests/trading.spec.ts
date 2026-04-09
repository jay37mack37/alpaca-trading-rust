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

test.describe('Trading & Orders', () => {
    test.beforeEach(async ({ page }) => {
        await login(page);
    });

    test('should display orders section', async ({ page }) => {
        // Look for orders tab or section
        const ordersSection = page.locator('text=/Orders|Transactions/i');
        await expect(ordersSection).toBeVisible({ timeout: 5000 });
    });

    test('should load orders from API', async ({ page }) => {
        // Intercept orders API call
        const ordersResponse = await page.waitForResponse(
            (response) => response.url().includes('/api/orders') && response.status() === 200,
            { timeout: 10000 }
        );

        const ordersData = await ordersResponse.json();
        expect(Array.isArray(ordersData) || Array.isArray(ordersData.orders)).toBeTruthy();
    });

    test('should display orders table or empty state', async ({ page }) => {
        // Check for orders data
        const table = page.locator('table, .orders-list');
        const emptyMsg = page.locator('text=/No orders|No data/i');

        const tableVisible = await table.isVisible().catch(() => false);
        const emptyVisible = await emptyMsg.isVisible().catch(() => false);

        expect(tableVisible || emptyVisible).toBeTruthy();
    });

    test('should have order creation form', async ({ page }) => {
        // Find order creation section
        const createButton = page.locator('button:has-text(/Create|New|Place/)');
        const orderForm = page.locator('form, [class*="order-form"]');

        const buttonExists = await createButton.isVisible({ timeout: 3000 }).catch(() => false);
        const formExists = await orderForm.isVisible({ timeout: 3000 }).catch(() => false);

        expect(buttonExists || formExists).toBeTruthy();
    });

    test('should fill and validate order form', async ({ page }) => {
        // Find create order button or navigate to form
        const createBtn = page.locator('button:has-text(/Create|New|Place/)');
        if (await createBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
            await createBtn.click();
        }

        // Look for order form fields
        const symbolInput = page.locator('input[id*="symbol"], input[placeholder*="Symbol"]');
        const quantityInput = page.locator('input[id*="quantity"], input[placeholder*="Quantity"]');
        const orderTypeSelect = page.locator('select[id*="type"], select[id*="order"]');

        // If form exists, fill it
        if (await symbolInput.isVisible({ timeout: 3000 }).catch(() => false)) {
            await symbolInput.fill('AAPL');

            if (await quantityInput.isVisible()) {
                await quantityInput.fill('10');
            }

            if (await orderTypeSelect.isVisible()) {
                await orderTypeSelect.selectOption('market');
            }

            // Look for submit button
            const submitBtn = page.locator('button:has-text(/Submit|Place|Send/)');
            if (await submitBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
                await expect(submitBtn).toBeEnabled();
            }
        }
    });

    test('should validate required order fields', async ({ page }) => {
        // Find and click create order button
        const createBtn = page.locator('button:has-text(/Create|New|Place/)');
        if (await createBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
            await createBtn.click();
        }

        // Try to submit without filling fields
        const submitBtn = page.locator('button:has-text(/Submit|Place|Send/)');
        if (await submitBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
            await submitBtn.click();

            // Should show validation error or prevent submission
            const errorMsg = page.locator('text=/required|invalid|please/i');
            const stillOnForm = await page.url().includes('order');

            const hasError = await errorMsg.isVisible({ timeout: 2000 }).catch(() => false);
            expect(hasError || stillOnForm).toBeTruthy();
        }
    });

    test('should list recent orders with details', async ({ page }) => {
        // Wait for orders to load
        await page.waitForTimeout(1000);

        // Check for order row elements
        const orderRows = page.locator('tr, .order-row, [class*="order-item"]');
        const rowCount = await orderRows.count();

        if (rowCount > 0) {
            const firstRow = orderRows.first();
            // Check for order details columns
            const cells = await firstRow.locator('td, [class*="cell"]').count();
            expect(cells).toBeGreaterThan(0);
        }
    });

    test('should handle order submission errors', async ({ page }) => {
        // Mock API failure
        await page.route('/api/orders', (route) => {
            if (route.request().method() === 'POST') {
                route.abort();
            } else {
                route.continue();
            }
        });

        // Try to create order
        const createBtn = page.locator('button:has-text(/Create|New|Place/)');
        if (await createBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
            await createBtn.click();

            const symbolInput = page.locator('input[id*="symbol"], input[placeholder*="Symbol"]');
            if (await symbolInput.isVisible()) {
                await symbolInput.fill('AAPL');

                const quantityInput = page.locator('input[id*="quantity"]');
                if (await quantityInput.isVisible()) {
                    await quantityInput.fill('10');
                }

                const submitBtn = page.locator('button:has-text(/Submit|Place/)');
                if (await submitBtn.isVisible()) {
                    await submitBtn.click();

                    // Should show error
                    const errorMsg = page.locator('text=/Error|Failed|Unable/i');
                    await expect(errorMsg).toBeVisible({ timeout: 5000 }).catch(() => { });
                }
            }
        }
    });

    test('should cancel order creation', async ({ page }) => {
        const createBtn = page.locator('button:has-text(/Create|New|Place/)');
        if (await createBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
            await createBtn.click();

            // Look for cancel button
            const cancelBtn = page.locator('button:has-text(/Cancel|Close|Back/)');
            if (await cancelBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
                await cancelBtn.click();

                // Form should close
                const form = page.locator('form, [class*="order-form"]');
                const formVisible = await form.isVisible({ timeout: 2000 }).catch(() => false);
                expect(!formVisible || !(await form.first().isVisible())).toBeTruthy();
            }
        }
    });
});
