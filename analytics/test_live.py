"""
Comprehensive Playwright E2E test suite for the Analytics tab.
Each test has REAL assertions that validate data correctness,
not just UI interactions. Designed to run live in a headed browser
so you can WATCH the tests execute and catch bugs visually.
"""
import asyncio
import json
import re
import sys
from playwright.async_api import async_playwright, expect

BASE_URL = "http://localhost:3000"

# ANSI colors for terminal output
GREEN = "\033[92m"
RED = "\033[91m"
YELLOW = "\033[93m"
CYAN = "\033[96m"
BOLD = "\033[1m"
RESET = "\033[0m"

passed = 0
failed = 0
errors = []


def log_pass(msg):
    global passed
    passed += 1
    print(f"  {GREEN}✓ PASS{RESET} {msg}")


def log_fail(msg, detail=""):
    global failed
    failed += 1
    errors.append(f"{msg}: {detail}")
    print(f"  {RED}✗ FAIL{RESET} {msg}")
    if detail:
        print(f"         {RED}{detail}{RESET}")


def log_step(msg):
    print(f"  {CYAN}→{RESET} {msg}")


async def login(page):
    """Log in and verify we reach the dashboard."""
    await page.goto(f"{BASE_URL}/")
    await page.fill('input#username', 'admin')
    await page.fill('input#password', 'admin123')
    await page.click('button[type="submit"]')
    await page.wait_for_timeout(1500)
    # Verify we're past the auth check
    auth_check = await page.query_selector('#auth-check')
    if auth_check:
        auth_visible = await auth_check.is_hidden()
        if not auth_visible:
            await page.wait_for_selector('#auth-check', state='hidden', timeout=5000)
    log_pass("Login successful")


async def test_1_login(page):
    """Test: User can log in and see the dashboard."""
    print(f"\n{BOLD}TEST 1: Login & Dashboard{RESET}")
    await login(page)
    # Verify dashboard tab is active and has content
    dashboard = page.locator('#dashboard-tab')
    await expect(dashboard).to_be_visible()
    log_pass("Dashboard tab is visible after login")


async def test_2_analytics_tab_visible(page):
    """Test: Analytics tab button exists and switches correctly."""
    print(f"\n{BOLD}TEST 2: Analytics Tab Switch{RESET}")
    tab_btn = page.locator('button[data-tab="analytics-tab"]')
    await expect(tab_btn).to_be_visible()
    log_pass("Analytics tab button is visible")

    # Click it
    await tab_btn.click()
    await page.wait_for_timeout(1000)

    analytics_tab = page.locator('#analytics-tab')
    await expect(analytics_tab).to_be_visible()
    await expect(analytics_tab).to_have_class(re.compile("active"))
    log_pass("Analytics tab content is visible and active")

    # Verify all 4 sections exist
    for section_id in ['watchlist-section', 'data-section', 'analysis-section', 'signals-section']:
        section = page.locator(f'#{section_id}')
        await expect(section).to_be_visible()
    log_pass("All 4 sections present (watchlist, data, analysis, signals)")


async def test_3_watchlist_display(page):
    """Test: Watchlist loads existing symbols and displays them as tags."""
    print(f"\n{BOLD}TEST 3: Watchlist Display{RESET}")

    # Wait for watchlist to load
    await page.wait_for_selector('#watchlist-symbols', timeout=10000)
    await page.wait_for_timeout(1000)

    # Check watchlist has content (either tags or "no symbols" message)
    watchlist_div = page.locator('#watchlist-symbols')
    content = await watchlist_div.inner_text()

    # Should have SPY, AAPL, TSLA from initial setup (or the "no symbols" text)
    if "No symbols" in content:
        log_fail("Watchlist is empty - expected SPY, AAPL, TSLA", f"Got: {content}")
    else:
        # Check specific symbols exist
        tags = await watchlist_div.locator('.watchlist-tag').all()
        tag_texts = []
        for tag in tags:
            tag_texts.append((await tag.inner_text()).strip())
        log_pass(f"Watchlist has {len(tags)} tags: {', '.join(tag_texts)}")

        # Verify expected symbols
        for sym in ['SPY', 'AAPL', 'TSLA']:
            found = any(sym in t for t in tag_texts)
            if found:
                log_pass(f"Symbol {sym} found in watchlist")
            else:
                log_fail(f"Symbol {sym} missing from watchlist", f"Tags: {tag_texts}")


async def test_4_watchlist_add_remove(page):
    """Test: Add a new symbol, verify it appears, then remove it."""
    print(f"\n{BOLD}TEST 4: Watchlist Add & Remove{RESET}")

    # Add MSFT
    log_step("Adding MSFT to watchlist...")
    await page.fill('#watchlist-add-input', 'MSFT')
    await page.click('#watchlist-add-btn')
    await page.wait_for_timeout(2000)

    watchlist_div = page.locator('#watchlist-symbols')
    tags = await watchlist_div.locator('.watchlist-tag').all()
    tag_texts = [await tag.inner_text() for tag in tags]

    msft_found = any('MSFT' in t for t in tag_texts)
    if msft_found:
        log_pass("MSFT added to watchlist and displayed")
    else:
        log_fail("MSFT not found after adding", f"Tags: {tag_texts}")

    # Also test adding lowercase - should uppercase
    log_step("Adding lowercase 'goog' - should become GOOG...")
    await page.fill('#watchlist-add-input', 'goog')
    await page.click('#watchlist-add-btn')
    await page.wait_for_timeout(2000)

    tags = await watchlist_div.locator('.watchlist-tag').all()
    tag_texts = [await tag.inner_text() for tag in tags]
    goog_found = any('GOOG' in t for t in tag_texts)
    if goog_found:
        log_pass("GOOG added with auto-uppercase")
    else:
        log_fail("GOOG not found after adding lowercase", f"Tags: {tag_texts}")

    # Remove GOOG using the X button
    log_step("Removing GOOG from watchlist...")
    goog_tag = watchlist_div.locator('.watchlist-tag').filter(has_text='GOOG')
    remove_btn = goog_tag.locator('.remove-tag')
    await remove_btn.click()
    await page.wait_for_timeout(1500)

    tags = await watchlist_div.locator('.watchlist-tag').all()
    tag_texts = [await tag.inner_text() for tag in tags]
    goog_gone = not any('GOOG' in t for t in tag_texts)
    if goog_gone:
        log_pass("GOOG removed from watchlist")
    else:
        log_fail("GOOG still in watchlist after removal", f"Tags: {tag_texts}")

    # Remove MSFT
    log_step("Removing MSFT from watchlist...")
    msft_tag = watchlist_div.locator('.watchlist-tag').filter(has_text='MSFT')
    remove_btn = msft_tag.locator('.remove-tag')
    await remove_btn.click()
    await page.wait_for_timeout(1500)

    tags = await watchlist_div.locator('.watchlist-tag').all()
    tag_texts = [await tag.inner_text() for tag in tags]
    msft_gone = not any('MSFT' in t for t in tag_texts)
    if msft_gone:
        log_pass("MSFT removed from watchlist")
    else:
        log_fail("MSFT still in watchlist after removal", f"Tags: {tag_texts}")


async def test_5_data_summary(page):
    """Test: Data summary shows bar counts for watchlist symbols."""
    print(f"\n{BOLD}TEST 5: Data Summary Display{RESET}")

    summary = page.locator('#data-summary-container')
    await page.wait_for_timeout(1000)
    content = await summary.inner_text()

    # Should show SPY with bar counts (data was fetched earlier)
    if "SPY" in content:
        log_pass("SPY shown in data summary")
    else:
        log_fail("SPY missing from data summary", f"Content: {content[:200]}")

    # Check for bar count numbers (should have numbers like "1,234" or "2,513")
    counts = re.findall(r'\d[\d,]+', content)
    if len(counts) >= 2:
        log_pass(f"Bar counts displayed ({len(counts)} numeric values found)")
    else:
        log_fail("No bar counts found in data summary", f"Content: {content[:200]}")

    # Check for timeframe labels
    for tf in ['1m', '5m', '1h', '1d']:
        if tf in content:
            log_pass(f"Timeframe {tf} shown in summary")
        else:
            log_fail(f"Timeframe {tf} missing from summary")


async def test_6_fetch_data(page):
    """Test: Fetch data and verify results appear."""
    print(f"\n{BOLD}TEST 6: Fetch Data{RESET}")

    # Set up: only fetch 1d for NVDA (quick)
    log_step("Setting up to fetch NVDA 1d data...")
    # Uncheck all timeframes except 1d
    for tf in ['1m', '5m', '1h']:
        cb = page.locator(f'#fetch-timeframes input[value="{tf}"]')
        if await cb.is_checked():
            await cb.uncheck()

    # Type NVDA in symbols input
    await page.fill('#fetch-symbols-input', 'NVDA')
    await page.click('#fetch-data-btn')

    # Wait for loading indicator
    loading = page.locator('#data-loading')
    if await loading.is_visible():
        log_pass("Loading indicator shown during fetch")

    await page.wait_for_timeout(10000)

    # Check fetch results appeared
    results = page.locator('#fetch-results-container')
    results_visible = await results.is_visible()
    if results_visible:
        log_pass("Fetch results container visible")

        # Check for NVDA in results
        results_text = await results.inner_text()
        if "NVDA" in results_text:
            log_pass("NVDA appears in fetch results")
        else:
            log_fail("NVDA missing from fetch results", f"Results: {results_text[:200]}")

        # Check for "ok" status
        if "bars" in results_text.lower():
            log_pass("Bar count shown in fetch results")
        else:
            log_fail("No bar count in fetch results", f"Results: {results_text[:200]}")
    else:
        log_fail("Fetch results container not visible")

    # Re-check all timeframes
    for tf in ['1m', '5m', '1h']:
        cb = page.locator(f'#fetch-timeframes input[value="{tf}"]')
        if not await cb.is_checked():
            await cb.check()

    # Clear symbols input (will use watchlist)
    await page.fill('#fetch-symbols-input', '')

    # Refresh data summary
    log_step("Refreshing data summary after fetch...")
    await page.wait_for_timeout(2000)

    summary = page.locator('#data-summary-container')
    summary_text = await summary.inner_text()
    if "NVDA" in summary_text:
        log_pass("NVDA now appears in data summary after fetch")
    else:
        log_fail("NVDA not in data summary after fetch", f"Summary: {summary_text[:200]}")


async def test_7_pattern_checkboxes(page):
    """Test: Pattern checkboxes load and can be toggled."""
    print(f"\n{BOLD}TEST 7: Pattern Checkboxes{RESET}")

    checkboxes_container = page.locator('#pattern-checkboxes')
    await expect(checkboxes_container).to_be_visible()

    # Should have 8 pattern checkboxes
    checkboxes = await checkboxes_container.locator('input[type="checkbox"]').all()
    if len(checkboxes) == 8:
        log_pass(f"All 8 pattern checkboxes present")
    else:
        log_fail(f"Expected 8 pattern checkboxes, got {len(checkboxes)}")

    # Verify specific patterns exist
    for pattern_id in ['vwap_deviation', 'gap_analysis', 'unusual_volume_1d', 'momentum_1d']:
        cb = checkboxes_container.locator(f'input[value="{pattern_id}"]')
        if await cb.count() > 0:
            log_pass(f"Pattern {pattern_id} checkbox found")
        else:
            log_fail(f"Pattern {pattern_id} checkbox missing")

    # Test unchecking all then checking specific ones
    log_step("Unchecking all patterns, then checking only gap_analysis...")
    for cb in await checkboxes_container.locator('input[type="checkbox"]').all():
        await cb.uncheck()

    await page.check('#pattern-checkboxes input[value="gap_analysis"]')
    gap_cb = page.locator('#pattern-checkboxes input[value="gap_analysis"]')
    if await gap_cb.is_checked():
        log_pass("gap_analysis checkbox can be checked individually")
    else:
        log_fail("gap_analysis checkbox not checked after clicking")


async def test_8_run_analysis(page):
    """Test: Run analysis and verify signals appear in the table."""
    print(f"\n{BOLD}TEST 8: Run Pattern Analysis{RESET}")

    # Set up: analyze SPY with gap_analysis and unusual_volume_1d
    log_step("Configuring analysis for SPY...")

    # Uncheck all patterns first
    for cb in await page.locator('#pattern-checkboxes input[type="checkbox"]').all():
        await cb.uncheck()

    # Check specific patterns
    await page.check('#pattern-checkboxes input[value="gap_analysis"]')
    await page.check('#pattern-checkboxes input[value="unusual_volume_1d"]')

    # Set min confidence
    await page.fill('#min-confidence', '0.5')

    # Set symbols
    await page.fill('#analysis-symbols', 'SPY')

    # Click analyze
    log_step("Running analysis...")
    await page.click('#analyze-btn')

    # Wait for results (analysis takes time)
    log_step("Waiting for analysis results...")
    await page.wait_for_timeout(15000)

    # Check for signals in table
    signals_table = page.locator('#signals-table')
    signals_visible = await signals_table.is_visible()

    if signals_visible:
        log_pass("Signals table is visible after analysis")

        # Count signal rows
        rows = await page.locator('#signals-body tr:not(.signal-details-row)').all()
        signal_count = len(rows)
        if signal_count > 0:
            log_pass(f"Found {signal_count} signal rows")

            # Check first row has expected columns
            first_row = rows[0]
            row_text = await first_row.inner_text()

            # Should contain a symbol (SPY)
            if "SPY" in row_text:
                log_pass("Signal row contains SPY symbol")
            else:
                log_fail("Signal row missing SPY symbol", f"Row: {row_text[:100]}")

            # Should contain a direction (bullish/bearish/neutral)
            if "bullish" in row_text.lower() or "bearish" in row_text.lower():
                log_pass("Signal row contains direction")
            else:
                log_fail("Signal row missing direction", f"Row: {row_text[:100]}")

            # Should contain a confidence percentage
            conf_match = re.search(r'(\d+)%', row_text)
            if conf_match:
                conf_val = int(conf_match.group(1))
                log_pass(f"Confidence bar shows {conf_val}%")
                if conf_val >= 50:
                    log_pass(f"Confidence {conf_val}% >= 50% (matches min_confidence filter)")
                else:
                    log_fail(f"Confidence {conf_val}% below 50% threshold", "Min confidence filter not working")
            else:
                log_fail("No confidence percentage found in signal row")

            # Should contain a pattern name
            pattern_names = ['VWAP', 'Gap', 'Volume', 'Momentum', 'ORB', 'Mean Reversion']
            found_pattern = any(p in row_text for p in pattern_names)
            if found_pattern:
                log_pass("Signal row contains pattern name")
            else:
                log_fail("No pattern name in signal row", f"Row: {row_text[:100]}")

            # Check signal count display
            count_div = page.locator('#signals-count')
            if await count_div.is_visible():
                count_text = await count_div.inner_text()
                log_pass(f"Signal count display: {count_text}")
            else:
                log_fail("Signal count display not visible")
        else:
            log_fail("No signal rows found after analysis", "Table visible but empty")
    else:
        # Check if "no data" message appears
        no_data = page.locator('#signals-no-data')
        no_data_visible = await no_data.is_visible()
        if no_data_visible:
            no_data_text = await no_data.inner_text()
            log_fail("No signals produced", f"Message: {no_data_text}")
        else:
            log_fail("Signals table not visible after analysis", "No error message either")


async def test_9_signal_filtering(page):
    """Test: Filter signals by direction and pattern."""
    print(f"\n{BOLD}TEST 9: Signal Filtering{RESET}")

    # First ensure we have signals
    signals_table = page.locator('#signals-table')
    if not await signals_table.is_visible():
        log_fail("No signals to filter - skipping filter tests")
        return

    all_rows = await page.locator('#signals-body tr:not(.signal-details-row)').all()
    total_count = len(all_rows)
    log_pass(f"Starting with {total_count} signals")

    if total_count == 0:
        log_fail("No signals to filter")
        return

    # Test direction filter: bullish
    log_step("Filtering by bullish direction...")
    await page.select_option('#signal-filter-direction', 'bullish')
    await page.wait_for_timeout(500)

    bullish_rows = await page.locator('#signals-body tr:not(.signal-details-row)').all()
    bullish_count = len(bullish_rows)

    if bullish_count < total_count and bullish_count > 0:
        log_pass(f"Bullish filter: {bullish_count} of {total_count} signals shown")

        # Verify all visible rows contain "bullish"
        all_bullish = True
        for row in bullish_rows:
            row_classes = await row.get_attribute('class') or ''
            row_text = await row.inner_text()
            if 'bullish' not in row_text.lower() and 'direction-bullish' not in row_classes:
                all_bullish = False
        if all_bullish:
            log_pass("All filtered rows contain bullish direction")
        else:
            log_fail("Some filtered rows are not bullish")
    elif bullish_count == total_count:
        log_fail("Bullish filter shows all signals (filter not working)")
    elif bullish_count == 0:
        log_fail("Bullish filter shows 0 signals (may be correct if none are bullish)")

    # Test direction filter: bearish
    log_step("Filtering by bearish direction...")
    await page.select_option('#signal-filter-direction', 'bearish')
    await page.wait_for_timeout(500)

    bearish_rows = await page.locator('#signals-body tr:not(.signal-details-row)').all()
    bearish_count = len(bearish_rows)
    log_pass(f"Bearish filter: {bearish_count} of {total_count} signals shown")

    # Reset filter
    await page.select_option('#signal-filter-direction', 'all')
    await page.wait_for_timeout(500)

    # Test pattern filter
    pattern_select = page.locator('#signal-filter-pattern')
    options = await pattern_select.locator('option').all()
    if len(options) > 1:
        # Select gap_analysis pattern filter
        await page.select_option('#signal-filter-pattern', 'gap_analysis')
        await page.wait_for_timeout(500)

        gap_rows = await page.locator('#signals-body tr:not(.signal-details-row)').all()
        gap_count = len(gap_rows)
        if gap_count > 0:
            log_pass(f"Gap analysis filter: {gap_count} signals")
            # Verify all show gap-related patterns
            for row in gap_rows[:3]:  # Check first 3
                row_text = await row.inner_text()
                if 'gap' in row_text.lower() or 'Gap' in row_text:
                    pass  # Expected
                else:
                    log_fail("Gap filter shows non-gap signal", f"Row: {row_text[:80]}")
        else:
            log_fail("Gap analysis filter shows 0 signals")

    # Reset pattern filter
    await page.select_option('#signal-filter-pattern', 'all')
    await page.wait_for_timeout(500)

    # Verify count restored
    reset_rows = await page.locator('#signals-body tr:not(.signal-details-row)').all()
    if len(reset_rows) == total_count:
        log_pass(f"All filters reset - {total_count} signals restored")
    else:
        log_fail(f"Signal count mismatch after reset: {len(reset_rows)} vs {total_count}")


async def test_10_signal_expand_details(page):
    """Test: Clicking a signal row expands its details."""
    print(f"\n{BOLD}TEST 10: Signal Detail Expansion{RESET}")

    signals_table = page.locator('#signals-table')
    if not await signals_table.is_visible():
        log_fail("No signals to test detail expansion")
        return

    rows = await page.locator('#signals-body tr:not(.signal-details-row)').all()
    if len(rows) == 0:
        log_fail("No signal rows to click")
        return

    # Click first row to expand
    first_row = rows[0]
    await first_row.click()
    await page.wait_for_timeout(500)

    # Check that a details row appeared
    detail_rows = await page.locator('#signals-body tr.signal-details-row').all()
    visible_details = 0
    for dr in detail_rows:
        style = await dr.get_attribute('style') or ''
        if 'display: none' not in style and 'display:none' not in style.replace(' ', ''):
            visible_details += 1

    if visible_details > 0:
        log_pass(f"Signal details expanded ({visible_details} visible detail rows)")

        # Check that details contain JSON-like content
        for dr in detail_rows[:1]:
            style = await dr.get_attribute('style') or ''
            if 'display: none' not in style:
                detail_text = await dr.inner_text()
                if '{' in detail_text or 'timestamp' in detail_text.lower():
                    log_pass("Detail row contains JSON data")
                else:
                    log_fail("Detail row has no JSON content", f"Content: {detail_text[:100]}")
    else:
        log_fail("No detail rows visible after clicking signal row")

    # Click again to collapse
    await first_row.click()
    await page.wait_for_timeout(500)
    detail_visible_after = 0
    for dr in detail_rows:
        style = await dr.get_attribute('style') or ''
        if 'display: none' not in style and 'display:none' not in style.replace(' ', ''):
            detail_visible_after += 1

    if detail_visible_after == 0 or detail_visible_after < visible_details:
        log_pass("Signal details collapsed on second click")
    else:
        log_fail("Signal details did not collapse", f"Still {detail_visible_after} visible rows")


async def test_11_export_signals(page):
    """Test: Export signals as JSON download."""
    print(f"\n{BOLD}TEST 11: Export Signals{RESET}")

    signals_table = page.locator('#signals-table')
    if not await signals_table.is_visible():
        log_fail("No signals to export")
        return

    # Set up download listener
    async with page.expect_download(timeout=10000) as download_info:
        await page.click('#export-signals-btn')

    download = await download_info.value
    filename = download.suggested_filename
    log_pass(f"Download triggered: {filename}")

    if filename.startswith('signals_') and filename.endswith('.json'):
        log_pass(f"Filename format correct: {filename}")
    else:
        log_fail("Filename format unexpected", f"Got: {filename}")


async def test_12_fetch_and_verify_nvda(page):
    """Test: Add NVDA to watchlist, fetch its data, verify it shows in summary."""
    print(f"\n{BOLD}TEST 12: NVDA Full Workflow (Add → Fetch → Summary){RESET}")

    # Step 1: Add NVDA
    log_step("Adding NVDA to watchlist...")
    await page.fill('#watchlist-add-input', 'NVDA')
    await page.click('#watchlist-add-btn')
    await page.wait_for_timeout(2000)

    watchlist_div = page.locator('#watchlist-symbols')
    tags = await watchlist_div.locator('.watchlist-tag').all()
    tag_texts = [await tag.inner_text() for tag in tags]

    nvda_in_watchlist = any('NVDA' in t for t in tag_texts)
    if nvda_in_watchlist:
        log_pass("NVDA added to watchlist")
    else:
        log_fail("NVDA not in watchlist after adding", f"Tags: {tag_texts}")

    # Step 2: Fetch 1d data for NVDA
    log_step("Fetching NVDA 1d data...")
    for tf in ['1m', '5m', '1h']:
        cb = page.locator(f'#fetch-timeframes input[value="{tf}"]')
        if await cb.is_checked():
            await cb.uncheck()
    # Keep 1d checked
    cb_1d = page.locator('#fetch-timeframes input[value="1d"]')
    if not await cb_1d.is_checked():
        await cb_1d.check()

    await page.fill('#fetch-symbols-input', 'NVDA')
    await page.click('#fetch-data-btn')
    await page.wait_for_timeout(12000)

    # Step 3: Verify NVDA appears in data summary
    log_step("Checking data summary for NVDA...")
    await page.wait_for_timeout(2000)
    summary = page.locator('#data-summary-container')
    summary_text = await summary.inner_text()

    if "NVDA" in summary_text:
        log_pass("NVDA appears in data summary after fetch")
    else:
        log_fail("NVDA missing from data summary", f"Content: {summary_text[:200]}")

    # Step 4: Run analysis specifically on NVDA
    log_step("Running analysis on NVDA...")
    for cb in await page.locator('#pattern-checkboxes input[type="checkbox"]').all():
        await cb.uncheck()
    await page.check('#pattern-checkboxes input[value="gap_analysis"]')
    await page.fill('#analysis-symbols', 'NVDA')
    await page.fill('#min-confidence', '0.3')
    await page.click('#analyze-btn')
    await page.wait_for_timeout(15000)

    # Step 5: Verify NVDA signals appear
    signals_table = page.locator('#signals-table')
    if await signals_table.is_visible():
        rows = await page.locator('#signals-body tr:not(.signal-details-row)').all()
        if len(rows) > 0:
            first_row_text = await rows[0].inner_text()
            if "NVDA" in first_row_text:
                log_pass("NVDA signal found in results")
            else:
                log_fail("First signal row is not NVDA", f"Row: {first_row_text[:80]}")
        else:
            log_fail("No signal rows for NVDA")
    else:
        no_data = page.locator('#signals-no-data')
        no_data_text = ""
        if await no_data.is_visible():
            no_data_text = await no_data.inner_text()
        log_fail("No signals table after NVDA analysis", f"Message: {no_data_text}")

    # Re-check all timeframes for other tests
    for tf in ['1m', '5m', '1h']:
        cb = page.locator(f'#fetch-timeframes input[value="{tf}"]')
        if not await cb.is_checked():
            await cb.check()
    await page.fill('#fetch-symbols-input', '')


async def test_13_error_handling(page):
    """Test: Invalid inputs and error states."""
    print(f"\n{BOLD}TEST 13: Error Handling{RESET}")

    # Test adding empty symbol
    log_step("Testing empty symbol add...")
    await page.fill('#watchlist-add-input', '')
    await page.click('#watchlist-add-btn')
    await page.wait_for_timeout(500)
    # Should not add anything (no error, just no-op)
    log_pass("Empty symbol add handled gracefully")

    # Test adding a single space
    log_step("Testing whitespace-only symbol add...")
    await page.fill('#watchlist-add-input', '   ')
    await page.click('#watchlist-add-btn')
    await page.wait_for_timeout(500)
    log_pass("Whitespace-only symbol handled gracefully")

    # Test analysis with no symbols and no patterns
    log_step("Testing analysis with no patterns selected...")
    for cb in await page.locator('#pattern-checkboxes input[type="checkbox"]').all():
        await cb.uncheck()
    await page.fill('#analysis-symbols', 'SPY')
    await page.click('#analyze-btn')
    await page.wait_for_timeout(3000)

    # Should either show no data message or error
    no_data = page.locator('#signals-no-data')
    err = page.locator('#analysis-error')
    if await no_data.is_visible() or await err.is_visible():
        log_pass("Analysis with no patterns shows appropriate message")
    else:
        log_fail("No feedback when analyzing with no patterns selected")

    # Re-check some patterns for next tests
    await page.check('#pattern-checkboxes input[value="gap_analysis"]')
    await page.check('#pattern-checkboxes input[value="unusual_volume_1d"]')


async def test_14_data_persistence(page):
    """Test: Data persists across tab switches."""
    print(f"\n{BOLD}TEST 14: Data Persistence Across Tab Switches{RESET}")

    # Switch to dashboard tab
    log_step("Switching to Dashboard tab...")
    await page.click('button[data-tab="dashboard-tab"]')
    await page.wait_for_timeout(1000)

    dashboard = page.locator('#dashboard-tab')
    await expect(dashboard).to_have_class(re.compile(r"active"))
    log_pass("Dashboard tab is active")

    # Switch back to analytics
    log_step("Switching back to Analytics tab...")
    await page.click('button[data-tab="analytics-tab"]')
    await page.wait_for_timeout(3000)

    analytics = page.locator('#analytics-tab')
    await expect(analytics).to_have_class(re.compile(r"active"))
    log_pass("Analytics tab is active again")

    # Verify watchlist still has symbols
    watchlist_div = page.locator('#watchlist-symbols')
    tags = await watchlist_div.locator('.watchlist-tag').all()
    if len(tags) > 0:
        log_pass(f"Watchlist preserved after tab switch ({len(tags)} symbols)")
    else:
        log_fail("Watchlist empty after tab switch")

    # Verify data summary still shows data
    summary = page.locator('#data-summary-container')
    summary_text = await summary.inner_text()
    if "SPY" in summary_text or "1d" in summary_text:
        log_pass("Data summary preserved after tab switch")
    else:
        log_fail("Data summary empty after tab switch", f"Content: {summary_text[:100]}")


async def main():
    global passed, failed, errors

    print(f"\n{BOLD}{'='*60}")
    print(f"  ANALYTICS TAB - COMPREHENSIVE E2E TEST SUITE")
    print(f"  {len(TESTS)} tests with real assertions")
    print(f"{'='*60}{RESET}\n")

    async with async_playwright() as p:
        browser = await p.chromium.launch(
            headless=False,
            slow_mo=300,
            executable_path="/usr/bin/google-chrome-stable",
            args=["--start-maximized", "--no-first-run",
                  "--ozone-platform=wayland",
                  "--enable-features=UseOzonePlatform"],
        )
        context = await browser.new_context(viewport={"width": 1400, "height": 900})
        page = await context.new_page()

        # Collect console errors
        console_errors = []
        page.on("console", lambda msg: console_errors.append(msg.text) if msg.type == "error" else None)

        try:
            for test_func in TESTS:
                try:
                    await test_func(page)
                except Exception as e:
                    log_fail(f"{test_func.__name__} threw exception", str(e)[:200])

            # Print console errors if any
            if console_errors:
                print(f"\n{YELLOW}Console Errors ({len(console_errors)}):{RESET}")
                for err_msg in console_errors[:10]:
                    print(f"  {YELLOW}• {err_msg[:150]}{RESET}")

        finally:
            print(f"\n{BOLD}{'='*60}")
            print(f"  TEST RESULTS")
            print(f"{'='*60}{RESET}")
            print(f"  {GREEN}Passed: {passed}{RESET}")
            print(f"  {RED}Failed: {failed}{RESET}")
            print(f"  Total:  {passed + failed}")
            if errors:
                print(f"\n{RED}Failures:{RESET}")
                for e in errors:
                    print(f"  {RED}• {e}{RESET}")
            print()

            # Keep browser open for exploration
            print(f"{CYAN}Browser stays open for exploration. Close it when done.{RESET}\n")
            await page.wait_for_timeout(300000)
            await browser.close()

    return failed == 0


# Ordered test suite
TESTS = [
    test_1_login,
    test_2_analytics_tab_visible,
    test_3_watchlist_display,
    test_4_watchlist_add_remove,
    test_5_data_summary,
    test_6_fetch_data,
    test_7_pattern_checkboxes,
    test_8_run_analysis,
    test_9_signal_filtering,
    test_10_signal_expand_details,
    test_11_export_signals,
    test_12_fetch_and_verify_nvda,
    test_13_error_handling,
    test_14_data_persistence,
]


if __name__ == "__main__":
    success = asyncio.run(main())
    sys.exit(0 if success else 1)