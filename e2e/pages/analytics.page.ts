import { Page, expect } from '@playwright/test';

export class AnalyticsPage {
  constructor(private page: Page) {}

  async goto() {
    await this.page.click('.tab-btn[data-tab="analytics-tab"]');
  }

  async addWatchlist(symbol: string) {
    await this.page.fill('#watchlist-add-input', symbol);
    await this.page.click('#watchlist-add-btn');
  }

  async expectSymbolInWatchlist(symbol: string) {
    await expect(this.page.locator('#watchlist-symbols')).toContainText(symbol);
  }

  async runAnalysis() {
    await this.page.click('#analyze-btn');
  }

  async expectSignals() {
    // Wait for analysis to complete and results to be visible
    await expect(this.page.locator('#signals-table')).toBeVisible({ timeout: 60000 });
  }
}
