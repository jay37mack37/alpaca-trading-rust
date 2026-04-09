from playwright.sync_api import sync_playwright, expect
import time
import json

def test_options_chain(page):
    # Mock authentication
    page.add_init_script("""
        localStorage.setItem('token', 'mock-token');
        localStorage.setItem('username', 'admin');
    """)

    # Mock API calls
    page.route("**/api/verify", lambda route: route.fulfill(status=200, content_type="application/json", body=json.dumps({"valid": True})))
    page.route("**/api/account", lambda route: route.fulfill(status=200, content_type="application/json", body=json.dumps({
        "account_number": "123456789",
        "status": "ACTIVE",
        "buying_power": "100000.00",
        "portfolio_value": "105000.00",
        "cash": "50000.00",
        "equity": "105000.00"
    })))
    page.route("**/api/positions", lambda route: route.fulfill(status=200, content_type="application/json", body="[]"))
    page.route("**/api/orders", lambda route: route.fulfill(status=200, content_type="application/json", body="[]"))
    page.route("**/api/price/SPY", lambda route: route.fulfill(status=200, content_type="application/json", body=json.dumps({
        "quote": {"ap": 500.0, "bp": 499.9}
    })))

    mock_chain = {
        "symbol": "SPY",
        "underlying_price": 500.0,
        "strikes": [
            {"strike": 480.0, "call": {"symbol": "SPY240621C00480000", "bid": 22.0, "ask": 22.5, "size": 5}, "put": {"symbol": "SPY240621P00480000", "bid": 1.0, "ask": 1.1, "size": 5}},
            {"strike": 490.0, "call": {"symbol": "SPY240621C00490000", "bid": 12.0, "ask": 12.5, "size": 5}, "put": {"symbol": "SPY240621P00490000", "bid": 2.0, "ask": 2.1, "size": 5}},
            {"strike": 500.0, "call": {"symbol": "SPY240621C00500000", "bid": 5.0, "ask": 5.2, "size": 5}, "put": {"symbol": "SPY240621P00500000", "bid": 5.1, "ask": 5.3, "size": 5}},
            {"strike": 510.0, "call": {"symbol": "SPY240621C00510000", "bid": 1.5, "ask": 1.7, "size": 5}, "put": {"symbol": "SPY240621P00510000", "bid": 11.0, "ask": 11.5, "size": 5}},
            {"strike": 520.0, "call": {"symbol": "SPY240621C00520000", "bid": 0.5, "ask": 0.6, "size": 5}, "put": {"symbol": "SPY240621P00520000", "bid": 21.0, "ask": 21.5, "size": 5}},
        ]
    }
    page.route("**/api/option-chain/SPY", lambda route: route.fulfill(status=200, content_type="application/json", body=json.dumps(mock_chain)))

    # Go to dashboard
    page.goto("http://localhost:3000/")

    # Wait for dashboard to load
    page.wait_for_selector("#account-info")

    # Click Load Options
    page.fill("#options-symbol", "SPY")
    page.click("#load-options-btn")

    # Wait for chart
    page.wait_for_selector("#options-chart-container")

    # Manually trigger strike show
    page.evaluate("showStrikeDetails(optionsData.strikes[2])")

    # Take screenshot of the chart and details
    page.screenshot(path="verification/options_chart.png")

    # Click "Select Call"
    page.click("text=Select Call")

    # Verify order form is filled
    expect(page.locator("#option-fields")).to_be_visible()
    expect(page.locator("#strike-price")).to_have_value("500")

    # Take screenshot of the order form
    page.screenshot(path="verification/order_form_filled.png")

if __name__ == "__main__":
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(viewport={'width': 1280, 'height': 1600})
        page = context.new_page()
        try:
            test_options_chain(page)
            print("Verification successful!")
        except Exception as e:
            print(f"Verification failed: {e}")
            page.screenshot(path="verification/error.png")
        finally:
            browser.close()
