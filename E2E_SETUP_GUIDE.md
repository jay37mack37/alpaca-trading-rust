# Playwright E2E Test Setup Guide

## Overview

This guide walks through setting up and running Playwright tests for the Alpaca Trading Bot project. The test suite is organized into two implementations:

- **Python** (`verification/e2e_tests.py`): Async pytest-based tests
- **TypeScript** (`e2e/`): npm-based Playwright tests

## Prerequisites

### System Requirements
- **Python 3.8+** (for Python tests)
- **Node.js 16+** (for TypeScript tests) 
- **Rust with Cargo** (to run backend server)

### Backend Server
Tests require the Rust backend running. The TypeScript config will auto-start it, but Python tests require manual start:

```bash
# Terminal 1: Start the backend server
cargo run --release
# Server will be available at http://localhost:3000
```

## Python Setup

### 1. Install Dependencies

```bash
# Create virtual environment (recommended)
python -m venv venv
source venv/Scripts/activate  # Windows: venv\Scripts\activate

# Install test dependencies
pip install playwright pytest pytest-asyncio
```

### 2. Configure Base URL

Edit `verification/e2e_tests.py` line 10:
```python
BASE_URL = 'http://localhost:3000'  # Update if using different port
```

### 3. Run Tests

```bash
# Run all tests with output
python verification/e2e_tests.py

# Run specific test method
python -m pytest verification/e2e_tests.py::AlpacaTradingBotTests::test_login_valid_credentials -v
```

### Python Test Results

Tests generate:
- **Screenshots**: `verification/screenshots/` (one per test)
- **Results JSON**: `verification/test_results.json` (summary and timing)
- **Console Output**: Test names, status, and timing

**Example Output:**
```
Running Alpaca Trading Bot E2E Tests...

✓ test_login_invalid_credentials (1.2s)
✓ test_login_valid_credentials (1.5s)
✓ test_dashboard_loads (2.1s)
✓ test_account_info_displays (1.8s)
✓ test_positions_load (1.4s)
✓ test_orders_load (1.6s)
✓ test_order_creation_workflow (2.3s)
✓ test_order_validation (1.9s)
✓ test_options_chain_load (2.4s)
✓ test_options_chart_display (1.5s)
✓ test_settings_page_loads (1.3s)
✓ test_settings_api_key_form (1.7s)

Tests complete! Results saved to verification/test_results.json
Screenshots saved to verification/screenshots/
```

## TypeScript Setup

### 1. Install Dependencies

```bash
cd e2e
npm install

# This installs @playwright/test and TypeScript
```

### 2. Configuration

The `playwright.config.ts` is pre-configured with:
- ✅ Multi-browser support (Chrome, Firefox, Safari)
- ✅ Mobile testing (Pixel 5, iPhone 12)
- ✅ HTML/JSON/JUnit reporting
- ✅ Auto-start backend server on `npm test`
- ✅ Screenshots on failure
- ✅ Video recording on failure

No additional config needed unless you:
- **Change port**: Update `baseURL` in `playwright.config.ts`
- **Add browsers**: Uncomment projects in config
- **Change timeouts**: Modify `timeout` values

### 3. Run Tests

```bash
# From e2e directory
cd e2e

# Run all tests (auto-starts backend)
npm test

# Run with UI mode (interactive)
npm run test:ui

# Run headed (see browser)
npm run test:headed

# Debug mode (step through)
npm run test:debug

# Run specific file
npx playwright test tests/auth.spec.ts

# Run specific test
npx playwright test tests/auth.spec.ts -g "should reject invalid"

# Generate HTML report
npm run test:report
```

### TypeScript Test Reports

Tests generate in `playwright-report/`:
- **index.html** - Interactive test report
- **test-results.json** - Machine-readable results
- **junit.xml** - CI/CD compatible format
- **test-results/** - Videos/traces on failure/retry

**To view HTML report:**
```bash
npm run test:report
# Opens browser to playwright-report/index.html
```

## Test Structure

### Authentication Tests (`auth.spec.ts`)

| Test | Purpose |
|------|---------|
| login form display | Verify login UI renders |
| invalid credentials | Reject wrong password |
| valid credentials | Accept admin/admin123 |
| token persistence | Check localStorage storage |
| logout | End session cleanly |
| no auth access | Redirect to login |
| session timeout | Handle missing token |

### Dashboard Tests (`dashboard.spec.ts`)

| Test | Purpose |
|------|---------|
| display sections | Main dashboard renders |
| account info | Account data displays |
| load from API | `/api/account` response |
| positions table | Show positions or empty |
| buying power/cash | Display account balance |
| handle API failure | Graceful error handling |
| navigation | Tab switching works |
| user info | Username display |

### Trading Tests (`trading.spec.ts`)

| Test | Purpose |
|------|---------|
| orders section | Orders UI visible |
| load from API | Fetch `/api/orders` |
| display table | Show orders list |
| creation form | Order form accessible |
| fill/validate | Form field defaults |
| required fields | Validation messages |
| recent orders | Display order details |
| submission errors | Handle API failures |
| cancel operation | Close form cleanly |

### Options Tests (`options.spec.ts`)

| Test | Purpose |
|------|---------|
| display section | Options UI visible |
| search form | Symbol input available |
| load chain | Fetch `/api/options` |
| display table | Show options data |
| expiration dates | Date selector available |
| strike filter | Price range filter |
| call/put display | Show option type |
| option details | Bid/ask/volume visible |
| invalid symbol | Handle bad tickers |
| buy options | Trading available |

### Settings Tests (`settings.spec.ts`)

| Test | Purpose |
|------|---------|
| display page | Settings UI renders |
| API section | Configuration visible |
| API key field | Input available |
| API secret field | Password input available |
| validate format | Reject bad keys |
| save button | Submit functionality |
| persist settings | localStorage updated |
| success notification | Confirmation message |
| trading preferences | Advanced options |
| reset defaults | Clear settings |
| theme preference | Dark/light mode |
| refresh rate | Update frequency |
| error handling | Network failure |

## Development Workflow

### Quick Test Loop

```bash
# Terminal 1: Backend
cargo run --release

# Terminal 2: Tests (watch mode)
cd e2e
npm test -- --watch

# Make code changes... tests will re-run automatically
```

### Debugging Specific Issues

```bash
# Stop and inspect
npx playwright test --debug

# See all browser interactions
PWDEBUG=1 npx playwright test

# Record new test
npx playwright codegen http://localhost:3000

# Step through test execution
npx playwright test tests/auth.spec.ts --debug
```

### Creating New Tests

1. **Open test file** (e.g., `tests/auth.spec.ts`)
2. **Add test block:**
   ```typescript
   test('should do something new', async ({ page }) => {
     // Your test code
   });
   ```
3. **Run test:**
   ```bash
   npm test -- -g "should do something new"
   ```
4. **Debug if needed:**
   ```bash
   npx playwright test tests/auth.spec.ts --debug
   ```

## Continuous Integration

### GitHub Actions Example

Create `.github/workflows/e2e-tests.yml`:
```yaml
name: E2E Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Install dependencies
        run: |
          cd e2e
          npm ci
      
      - name: Run Playwright tests
        run: |
          cd e2e
          npm test
      
      - name: Upload test report
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: playwright-report
          path: e2e/playwright-report/
          retention-days: 30
```

## Performance & Optimization

### Parallel Execution
Tests run in parallel by default. Adjust workers:
```bash
# Run with 4 workers (faster, higher resource usage)
npm test -- --workers=4

# Run sequentially (slower, lower resource usage)  
npm test -- --workers=1
```

### Browser Selection
Run only specific browsers:
```bash
# Chrome only (fastest)
npm test -- --project=chromium

# All browsers (comprehensive)
npm test

# Specific test in headless Firefox
npx playwright test tests/auth.spec.ts --project=firefox
```

### Performance Tips

1. **Run full suite before committing:**
   ```bash
   npm test  # All browsers, full suite
   ```

2. **Use UI mode for development:**
   ```bash
   npm run test:ui  # Interactive debugging
   ```

3. **Run smoke tests only during development:**
   ```bash
   npx playwright test --grep @smoke
   ```

## Troubleshooting

### Tests Fail with "Connection Refused"
**Problem:** Backend not running
**Solution:**
```bash
# Terminal 1: Start backend
cargo run --release

# Terminal 2: Run tests
cd e2e
npm test
```

### "Element not found" Errors
**Problem:** Selectors outdated or UI changed
**Solution:**
```bash
# Debug mode to inspect
npx playwright test --debug

# Record new interaction
npx playwright codegen http://localhost:3000
```

### Tests Timeout
**Problem:** Slow network or server overloaded
**Solution:**
```typescript
// In playwright.config.ts, increase timeout:
timeout: 300000,  // 5 minutes
use: {
  navigationTimeout: 30000,
  actionTimeout: 10000,
}
```

### Cannot Find Playwright Browsers
**Problem:** Playwright browsers not downloaded
**Solution:**
```bash
# Install browsers
npx playwright install

# Or with dependencies
npx playwright install-deps
```

## Reporting & Metrics

### View Test Results

```bash
# HTML report (interactive)
npm run test:report

# JSON results programmatically
cat test-results.json | jq '.stats'

# Quick summary
npx playwright show-report
```

### Test Metrics

Expected baseline performance:
- **Auth suite**: 2-3 seconds
- **Dashboard suite**: 3-5 seconds
- **Trading suite**: 3-5 seconds  
- **Options suite**: 4-6 seconds
- **Settings suite**: 3-5 seconds
- **Total**: 20-30 seconds (all 43 tests)

*Note: Times vary based on network and system load*

## Next Steps

1. **Run initial test suite:**
   ```bash
   npm test
   ```

2. **Review results:**
   ```bash
   npm run test:report
   ```

3. **Check dev console integration:**
   - Open app in browser
   - Press `Ctrl+Shift+D` or click 🛠️ Dev Console button
   - Verify network inspection working

4. **Set up CI/CD:**
   - Add `.github/workflows/e2e-tests.yml`
   - Push to GitHub
   - Tests run automatically on PRs

5. **Integrate with development:**
   - Keep tests running during development
   - Run full suite before commits
   - Review failures in UI mode

## Additional Resources

- [Playwright Docs](https://playwright.dev)
- [Best Practices](https://playwright.dev/docs/best-practices)
- [Debugging Guide](https://playwright.dev/docs/debug)
- [CI/CD Guide](https://playwright.dev/docs/ci)
- [Pytest Docs](https://docs.pytest.org)
