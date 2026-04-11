import { Page, expect } from '@playwright/test';

export class DashboardPage {
  constructor(private page: Page) {}

  async goto() {
    await this.page.goto('/');
  }

  async getAccountNumber() {
    return this.page.locator('#account-number');
  }

  async expectLoggedIn() {
    await expect(this.page.locator('#user-display')).toContainText('admin');
  }

  async logout() {
    await this.page.click('#logout-btn');
  }
}
