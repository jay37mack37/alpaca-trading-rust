import { Page, expect } from '@playwright/test';

export class LoginPage {
  constructor(private page: Page) {}

  async goto() {
    await this.page.goto('/login.html');
  }

  async login(username = 'admin', password = 'admin123') {
    await this.page.fill('#username', username);
    await this.page.fill('#password', password);
    await this.page.click('button:has-text("Login")');
  }

  async expectLoginError() {
    await expect(this.page.locator('.error-message')).toBeVisible();
  }
}
