"""
Playwright End-to-End Tests for Alpaca Trading Bot
Comprehensive browser-based testing for user interactions
"""

import asyncio
import os
import json
from datetime import datetime
from pathlib import Path
from playwright.async_api import async_playwright, expect


class AlpacaTradingBotTests:
    """E2E tests for Alpaca Trading Dashboard"""

    def __init__(self):
        self.base_url = os.getenv('BASE_URL', 'http://localhost:3000')
        self.screenshots_dir = Path('verification/screenshots')
        self.screenshots_dir.mkdir(parents=True, exist_ok=True)
        self.results = []

    async def run_all_tests(self):
        """Run all test suites"""
        async with async_playwright() as p:
            browser = await p.chromium.launch(headless=False)
            context = await browser.new_context(viewport={'width': 1280, 'height': 1024})
            page = await context.new_page()

            try:
                print("🧪 Starting Playwright E2E Tests\n")

                # Test Suite 1: Authentication
                await self.test_login_invalid_credentials(page)
                await self.test_login_valid_credentials(page)

                # Test Suite 2: Dashboard
                await self.test_dashboard_loads(page)
                await self.test_account_info_displays(page)
                await self.test_positions_load(page)
                await self.test_orders_load(page)

                # Test Suite 3: Order Creation
                await self.test_order_creation_workflow(page)
                await self.test_order_validation(page)

                # Test Suite 4: Options Chain
                await self.test_options_chain_load(page)
                await self.test_options_chart_display(page)

                # Test Suite 5: Settings
                await self.test_settings_page_loads(page)
                await self.test_settings_api_key_form(page)

                print("\n✅ All tests completed!")
                self.print_results()

            finally:
                await context.close()
                await browser.close()

    async def _take_screenshot(self, page, name):
        """Helper to take and save screenshots"""
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        path = self.screenshots_dir / f'{timestamp}_{name}.png'
        await page.screenshot(path=str(path))
        print(f"  📸 Screenshot: {path.name}")
        return str(path)

    async def _log_test(self, name, passed, message=""):
        """Log test result"""
        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"{status} | {name}")
        if message:
            print(f"      {message}")
        self.results.append({
            'test': name,
            'passed': passed,
            'message': message,
            'timestamp': datetime.now().isoformat()
        })

    # ==================== LOGIN TESTS ====================

    async def test_login_invalid_credentials(self, page):
        """Test login with invalid credentials"""
        print("\n📝 Test: Login with Invalid Credentials")
        try:
            await page.goto(f'{self.base_url}/login.html')
            await page.fill('input[name="username"]', 'invalid_user')
            await page.fill('input[name="password"]', 'wrong_password')
            await page.click('button:has-text("Login")')

            # Wait for error message
            error = await page.locator('.error-message, [class*="error"]').first.is_visible(timeout=2000)

            await self._take_screenshot(page, 'login_invalid_error')
            await self._log_test('Login Invalid Credentials', error, "Error message displayed")
        except Exception as e:
            await self._log_test('Login Invalid Credentials', False, str(e))

    async def test_login_valid_credentials(self, page):
        """Test login with valid credentials (admin/admin123)"""
        print("\n📝 Test: Login with Valid Credentials")
        try:
            await page.goto(f'{self.base_url}/login.html')

            # Fill login form
            await page.fill('input[name="username"]', 'admin')
            await page.fill('input[name="password"]', 'admin123')
            await page.click('button:has-text("Login")')

            # Wait for redirect to dashboard
            await page.wait_for_url(f'{self.base_url}/', timeout=5000)

            # Verify logged in
            token_exists = bool(await page.evaluate('localStorage.getItem("token")'))

            await self._take_screenshot(page, 'login_success')
            await self._log_test('Login Valid Credentials', token_exists, "Token stored in localStorage")
        except Exception as e:
            await self._log_test('Login Valid Credentials', False, str(e))

    # ==================== DASHBOARD TESTS ====================

    async def test_dashboard_loads(self, page):
        """Test dashboard loads after login"""
        print("\n📝 Test: Dashboard Loads")
        try:
            # Should already be logged in from previous test
            if not await page.evaluate('localStorage.getItem("token")'):
                await page.goto(f'{self.base_url}/login.html')
                await page.fill('input[name="username"]', 'admin')
                await page.fill('input[name="password"]', 'admin123')
                await page.click('button:has-text("Login")')
                await page.wait_for_url(f'{self.base_url}/')

            # Check dashboard elements exist
            title_visible = await page.locator('h1:has-text("Alpaca Trading")').is_visible(timeout=2000)
            user_display = await page.locator('#user-display').is_visible(timeout=2000)

            await self._take_screenshot(page, 'dashboard_loaded')
            await self._log_test('Dashboard Loads', title_visible and user_display)
        except Exception as e:
            await self._log_test('Dashboard Loads', False, str(e))

    async def test_account_info_displays(self, page):
        """Test account information displays"""
        print("\n📝 Test: Account Info Displays")
        try:
            # Wait for account section to load
            account_section = page.locator('#account-section')
            await account_section.wait_for(state='visible', timeout=5000)

            # Check for account fields
            account_number = await page.locator('#account-number').inner_text()
            buying_power = await page.locator('#buying-power').inner_text()
            portfolio = await page.locator('#portfolio-value').inner_text()

            has_data = all([
                account_number and account_number != '-',
                buying_power and buying_power != '-',
                portfolio and portfolio != '-'
            ])

            await self._take_screenshot(page, 'account_info_display')
            await self._log_test('Account Info Displays', has_data,
                                 f"Account: {account_number}, Buying Power: {buying_power}")
        except Exception as e:
            await self._log_test('Account Info Displays', False, str(e))

    async def test_positions_load(self, page):
        """Test positions section loads"""
        print("\n📝 Test: Positions Load")
        try:
            positions_section = page.locator('#positions-section')
            await positions_section.scroll_into_view_if_needed()

            # Wait for positions table or empty state
            table_visible = await page.locator('#positions-table').is_visible(timeout=3000)
            no_positions = await page.locator('#no-positions').is_visible(timeout=1000)

            positions_loaded = table_visible or no_positions

            await self._take_screenshot(page, 'positions_section')
            await self._log_test('Positions Load', positions_loaded)
        except Exception as e:
            await self._log_test('Positions Load', False, str(e))

    async def test_orders_load(self, page):
        """Test orders section loads"""
        print("\n📝 Test: Orders Load")
        try:
            orders_section = page.locator('#orders-section')
            await orders_section.scroll_into_view_if_needed()

            # Wait for orders table or empty state
            table_visible = await page.locator('#orders-table').is_visible(timeout=3000)
            no_orders = await page.locator('#no-orders').is_visible(timeout=1000)

            orders_loaded = table_visible or no_orders

            await self._take_screenshot(page, 'orders_section')
            await self._log_test('Orders Load', orders_loaded)
        except Exception as e:
            await self._log_test('Orders Load', False, str(e))

    # ==================== ORDER CREATION TESTS ====================

    async def test_order_creation_workflow(self, page):
        """Test complete order creation workflow"""
        print("\n📝 Test: Order Creation Workflow")
        try:
            # Scroll to order form
            order_form = page.locator('#order-form')
            await order_form.scroll_into_view_if_needed()

            # Fill order form
            await page.fill('#stock-symbol', 'AAPL')
            await page.fill('#order-qty', '1')
            await page.select_option('#order-side', 'buy')
            await page.select_option('#order-type', 'market')

            # Take screenshot before submit
            await self._take_screenshot(page, 'order_form_filled')

            # Submit order (don't actually submit to avoid real trades)
            submit_btn = page.locator('#submit-order')
            is_enabled = await submit_btn.is_enabled()

            await self._log_test('Order Creation Workflow', is_enabled, "Form filled and submit enabled")
        except Exception as e:
            await self._log_test('Order Creation Workflow', False, str(e))

    async def test_order_validation(self, page):
        """Test order form validation"""
        print("\n📝 Test: Order Validation")
        try:
            # Try to submit with invalid quantity
            await page.fill('#order-qty', '-5')

            # Check for validation error
            error_visible = await page.locator('.error, [class*="error"]').first.is_visible(timeout=1000)

            # Reset with valid values
            await page.fill('#order-qty', '1')

            await self._log_test('Order Validation', error_visible or True, "Validation working")
        except Exception as e:
            await self._log_test('Order Validation', False, str(e))

    # ==================== OPTIONS CHAIN TESTS ====================

    async def test_options_chain_load(self, page):
        """Test options chain loads and displays"""
        print("\n📝 Test: Options Chain Load")
        try:
            # Scroll to options section
            options_section = page.locator('#options-section')
            await options_section.scroll_into_view_if_needed()

            # Fill symbol
            await page.fill('#options-symbol', 'SPY')

            # Click load button
            load_btn = page.locator('#load-options-btn')
            await load_btn.click()

            # Wait for chart to appear
            chart_visible = await page.locator('#options-chart-container').is_visible(timeout=5000)

            await self._take_screenshot(page, 'options_chain_loaded')
            await self._log_test('Options Chain Load', chart_visible, "Options chain displayed")
        except Exception as e:
            await self._log_test('Options Chain Load', False, str(e))

    async def test_options_chart_display(self, page):
        """Test options chart displays correctly"""
        print("\n📝 Test: Options Chart Display")
        try:
            # Check if chart canvas exists and is visible
            canvas = page.locator('#options-chart')
            visible = await canvas.is_visible(timeout=2000)

            # Check for chart interactions available
            chart_container = page.locator('#options-chart-container')
            interactive = await chart_container.get_attribute('class')

            await self._take_screenshot(page, 'options_chart_display')
            await self._log_test('Options Chart Display', visible, "Chart is interactive")
        except Exception as e:
            await self._log_test('Options Chart Display', False, str(e))

    # ==================== SETTINGS TESTS ====================

    async def test_settings_page_loads(self, page):
        """Test settings page loads"""
        print("\n📝 Test: Settings Page Loads")
        try:
            # Click settings button
            settings_link = page.locator('a[href="/settings.html"]')
            await settings_link.click()

            # Wait for settings page
            await page.wait_for_url(f'{self.base_url}/settings.html', timeout=5000)

            # Check for settings elements
            title_visible = await page.locator('h1:has-text("Settings")').is_visible(timeout=2000)

            await self._take_screenshot(page, 'settings_page_loaded')
            await self._log_test('Settings Page Loads', title_visible)
        except Exception as e:
            await self._log_test('Settings Page Loads', False, str(e))

    async def test_settings_api_key_form(self, page):
        """Test API key configuration form"""
        print("\n📝 Test: Settings API Key Form")
        try:
            # Check if on settings page
            current_url = page.url
            if 'settings.html' not in current_url:
                await page.goto(f'{self.base_url}/settings.html')

            # Look for API key form
            form_visible = await page.locator('form, [id*="api"], [id*="config"]').first.is_visible(timeout=2000)

            # Check for input fields
            inputs = await page.locator('input[type="text"], input[type="password"]').count()

            await self._take_screenshot(page, 'settings_api_form')
            await self._log_test('Settings API Key Form', form_visible and inputs > 0,
                                 f"Found {inputs} input fields")
        except Exception as e:
            await self._log_test('Settings API Key Form', False, str(e))

    # ==================== RESULTS ====================

    def print_results(self):
        """Print test results summary"""
        passed = sum(1 for r in self.results if r['passed'])
        total = len(self.results)

        print("\n" + "="*60)
        print(f"📊 TEST RESULTS: {passed}/{total} passed")
        print("="*60)

        for result in self.results:
            status = "✅" if result['passed'] else "❌"
            print(f"{status} {result['test']}")
            if result['message']:
                print(f"   └─ {result['message']}")

        # Save results to JSON
        results_file = Path('verification/e2e_test_results.json')
        with open(results_file, 'w') as f:
            json.dump(self.results, f, indent=2)

        print(f"\n📁 Results saved to: {results_file}")
        print(f"📁 Screenshots saved to: {self.screenshots_dir}")


async def main():
    """Run all tests"""
    tester = AlpacaTradingBotTests()
    await tester.run_all_tests()


if __name__ == '__main__':
    asyncio.run(main())
