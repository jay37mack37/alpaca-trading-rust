import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

export class DashboardPage extends BasePage {
    constructor(page: Page) {
        super(page);
    }

    async switchTab(tabName: 'dashboard-tab' | 'history-tab' | 'analytics-tab') {
        await this.page.click(`button[data-tab="${tabName}"]`);
    }

    async getPortfolioValue() {
        return await this.page.locator('#portfolio-value').textContent();
    }

    async logout() {
        await this.page.click('#logout-btn');
    }

    async placeStockOrder(symbol: string, side: 'buy' | 'sell', qty: number) {
        await this.page.fill('#symbol', symbol);
        await this.page.selectOption('#side', side);
        await this.page.fill('#qty', qty.toString());
        await this.page.click('#submit-order');
    }
}
