import { Page } from '@playwright/test';
import { BasePage } from './BasePage';

export class LoginPage extends BasePage {
    constructor(page: Page) {
        super(page);
    }

    async navigate() {
        await super.navigate('/login.html');
    }

    async login(username: string = 'admin', password: string = 'admin123') {
        await this.page.fill('#username', username);
        await this.page.fill('#password', password);
        await this.page.click('button[type="submit"]');
    }

    async getErrorMessage() {
        return await this.page.locator('#login-error').textContent();
    }
}
