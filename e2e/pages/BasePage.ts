import { Page, expect } from '@playwright/test';

export class BasePage {
    constructor(protected page: Page) {}

    async navigate(path: string = '/') {
        await this.page.goto(path);
    }

    async getStatusText() {
        return await this.page.locator('#status-text').textContent();
    }

    async waitForConnected() {
        await expect(this.page.locator('.status-dot')).toHaveClass(/connected/);
    }
}
