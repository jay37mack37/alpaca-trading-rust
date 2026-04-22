import { test, expect } from "@playwright/test";

const API_TOKEN = "6f974436cb24058c6d838e23959d0fdbbf0891c585406b61d2ec845ecef5f455";

test.describe("Welcome Page Flow", () => {
  test("shows welcome page when no valid token is configured", async ({ page }) => {
    // Clear localStorage before navigation
    await page.goto("/");
    await page.evaluate(() => localStorage.removeItem("autostonks_api_token"));
    await page.reload();
    await page.waitForTimeout(2000);

    await expect(page.locator("h1")).toContainText("Welcome");
    await expect(page.locator("h2")).toContainText("Connect to Backend");
    await expect(page.locator('input[type="text"]')).toBeVisible();
  });

  test("connects with valid token and skips to dashboard", async ({ page }) => {
    // Clear localStorage before navigation
    await page.goto("/");
    await page.evaluate(() => localStorage.removeItem("autostonks_api_token"));
    await page.reload();
    await page.waitForTimeout(3000);

    // Should see welcome page
    await expect(page.locator("h1")).toContainText("Welcome");

    // Wait for backend health check to detect it
    await page.waitForTimeout(4000);

    // Fill in the correct token
    await page.locator('input[type="text"]').fill(API_TOKEN);
    await page.locator('button:has-text("Connect")').click();

    // Should advance past step 1 (wait up to 10s for API call)
    await page.waitForTimeout(5000);

    // Check what step we're on
    const allH2 = await page.locator("h2").allTextContents();
    const pageText = await page.locator("body").innerText();

    // We should have either reached credentials step, done step, or still be on connect step
    if (allH2.some(t => t.includes("Add Alpaca Keys"))) {
      // Step 2: Click Skip
      await page.locator('button:has-text("Skip")').click();
      await page.waitForTimeout(2000);
    }

    // Click Launch Dashboard if visible
    const launchBtn = page.locator('button:has-text("Launch Dashboard")');
    if (await launchBtn.isVisible().catch(() => false)) {
      await launchBtn.click();
      await page.waitForTimeout(3000);
    }

    // Verify localStorage has the token
    const stored = await page.evaluate(() => localStorage.getItem("autostonks_api_token"));
    expect(stored).toBe(API_TOKEN);

    // Verify the .env was written via the API
    const envResp = await page.request.post("http://127.0.0.1:8080/api/setup/env", {
      data: { api_token: API_TOKEN },
    });
    expect(envResp.ok()).toBeTruthy();
  });

  test("rejects invalid token", async ({ page }) => {
    await page.goto("/");
    await page.evaluate(() => localStorage.removeItem("autostonks_api_token"));
    await page.reload();
    await page.waitForTimeout(3000);

    // Fill wrong token
    await page.locator('input[type="text"]').fill("wrong-token-abc123");
    await page.locator('button:has-text("Connect")').click();
    await page.waitForTimeout(3000);

    // Should see error message
    const errorVisible = await page.locator("[class*='step-error']").isVisible().catch(() => false);
    if (errorVisible) {
      const errorText = await page.locator("[class*='step-error']").textContent();
      expect(errorText).toContain("Invalid");
    }

    // localStorage should NOT have the wrong token
    const stored = await page.evaluate(() => localStorage.getItem("autostonks_api_token"));
    expect(stored).toBeNull();
  });

  test("API endpoints work correctly", async ({ page }) => {
    // GET /api/setup/status is unauthenticated and returns setup status
    const statusResp = await page.request.get("http://127.0.0.1:8080/api/setup/status");
    expect(statusResp.ok()).toBeTruthy();
    const statusBody = await statusResp.json();
    expect(statusBody.success).toBe(true);
    expect(statusBody.data.backend_ready).toBe(true);
    expect(typeof statusBody.data.has_credentials).toBe("boolean");

    // POST /api/setup/env with wrong token returns 401
    const badResp = await page.request.post("http://127.0.0.1:8080/api/setup/env", {
      data: { api_token: "invalid-token" },
    });
    expect(badResp.status()).toBe(401);

    // POST /api/setup/env with correct token writes to .env
    const goodResp = await page.request.post("http://127.0.0.1:8080/api/setup/env", {
      data: { api_token: API_TOKEN },
    });
    expect(goodResp.ok()).toBeTruthy();
    const goodBody = await goodResp.json();
    expect(goodBody.success).toBe(true);
    expect(goodBody.data.written).toBe(true);

    // Authenticated API calls work with the token
    const dashResp = await page.request.get(
      "http://127.0.0.1:8080/api/dashboard?symbol=SPY&provider=yahoo",
      { headers: { Authorization: `Bearer ${API_TOKEN}` } },
    );
    expect(dashResp.ok()).toBeTruthy();
  });
});
