# Playwright E2E Testing Suite

Comprehensive end-to-end testing for the Alpaca Trading Bot using Playwright for both Python and TypeScript/Node.js.

## Quick Start

### Python E2E Tests (Async Implementation)

```bash
# Install dependencies
pip install playwright pytest pytest-asyncio

# Run all tests
python verification/e2e_tests.py

# Tests run in parallel with async/await
```

**Features:**
- 7+ comprehensive test suites
- Screenshot capture at each step
- JSON result reporting
- Async/await pattern for parallel execution
- Mock patterns built-in

**Test Coverage:**
1. **Authentication** - Login flow, invalid credentials, session management
2. **Dashboard** - Account info, portfolio display, data loading
3. **Trading** - Order creation, validation, history
4. **Options** - Chain loading, filtering, strike display
5. **Settings** - Configuration, API key management

**Example Test:**
```python
async def test_login_valid_credentials(self):
    await self.page.goto(f'{self.base_url}/login.html')
    await self.page.fill('input[id="username"]', 'admin')
    await self.page.fill('input[id="password"]', 'admin123')
    await self.page.click('button:has-text("Login")')
    
    await self.page.screenshot(path=f'{self.screenshots_dir}/login_success.png')
    assert self.page.url.endswith('/')
```

### TypeScript E2E Tests (npm/Node.js)

```bash
# Install dependencies
cd e2e
npm install

# Run all tests
npm test

# Run with UI mode
npm run test:ui

# Run in headed mode (see browser)
npm run test:headed

# Run specific test file
npx playwright test tests/auth.spec.ts

# Generate HTML report
npm run test:report
```

**Playwright Config:**
- [e2e/playwright.config.ts](e2e/playwright.config.ts)
- Multi-browser testing (Chrome, Firefox, Safari)
- Mobile viewport testing (Pixel 5, iPhone 12)
- HTML/JSON/JUnit reporting
- Automatic webserver startup (`cargo run --release`)
- Screenshots and video on failure
- Full trace collection on first retry

**Test Files:**

| File | Tests | Coverage |
|------|-------|----------|
| [e2e/tests/auth.spec.ts](e2e/tests/auth.spec.ts) | 7 | Login flow, credentials, session, logout, timeout |
| [e2e/tests/dashboard.spec.ts](e2e/tests/dashboard.spec.ts) | 8 | Dashboard loading, account info, positions, navigation |
| [e2e/tests/trading.spec.ts](e2e/tests/trading.spec.ts) | 8 | Order creation, validation, submission, history |
| [e2e/tests/options.spec.ts](e2e/tests/options.spec.ts) | 9 | Options chain, filters, symbols, bid/ask display |
| [e2e/tests/settings.spec.ts](e2e/tests/settings.spec.ts) | 11 | Settings UI, API config, persistence, validation |

**Total: 43 TypeScript test cases**

## System Requirements

### Python Tests
```
Python 3.8+
pytest
pytest-asyncio
playwright==1.40.0+
```

### TypeScript Tests
```
Node.js 16+
npm or yarn
@playwright/test 1.40.0+
TypeScript 5.0.0+
```

## Development Workflow

### Running Tests During Development

**Watch Mode (Python):**
```bash
# Run tests once developers change code
python verification/e2e_tests.py
```

**Watch Mode (TypeScript):**
```bash
cd e2e
npx playwright test --watch
```

**Debug Mode:**
```bash
npx playwright test --debug
```

**UI Mode (Interactive):**
```bash
npm run test:ui
```

### Creating New Tests

**Python Pattern:**
```python
async def test_new_feature(self):
    await self.page.goto(f'{self.base_url}/page')
    
    # Interact with page
    await self.page.fill('#input-id', 'value')
    await self.page.click('button:has-text("Click me")')
    
    # Assert expectations
    await expect(page.locator('text=Success')).to_be_visible()
    
    # Capture screenshot
    await self.page.screenshot(path=f'{self.screenshots_dir}/feature_result.png')
    
    # Add to results
    self.test_results.append({
        'name': 'test_new_feature',
        'status': 'PASSED',
        'duration': time.time() - start_time,
        'screenshot': 'feature_result.png'
    })
```

**TypeScript Pattern:**
```typescript
test('should do something', async ({ page }) => {
  await page.goto('/');
  await page.fill('#input-id', 'value');
  await page.click('button:has-text("Click me")');
  
  await expect(page.locator('text=Success')).toBeVisible();
});
```

## Configuration

### Python Configuration
Edit `verification/e2e_tests.py`:
```python
BASE_URL = 'http://localhost:3000'  # Target URL
HEADLESS = True  # Run headless browser
TIMEOUT = 10000  # Page timeout (ms)
```

### TypeScript Configuration
Edit `e2e/playwright.config.ts`:
```typescript
use: {
  baseURL: process.env.BASE_URL || 'http://localhost:3000',
  trace: 'on-first-retry',
  screenshot: 'only-on-failure',
  video: 'retain-on-failure',
}
```

## Test Reports

### Python Reports
- Screenshots saved to `verification/screenshots/`
- JSON results in `verification/test_results.json`
- Console output with test summary

### TypeScript Reports
- HTML report: `playwright-report/index.html`
- JSON report: `test-results.json`
- JUnit XML: `junit.xml`
- Videos: `test-results/` (on failure)
- Traces: `test-results/` (on retry)

## CI/CD Integration

### GitHub Actions (Recommended)
Create `.github/workflows/e2e-tests.yml`:
```yaml
name: E2E Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Build and run E2E tests
        run: |
          npm install --prefix e2e
          npm test --prefix e2e
      - uses: actions/upload-artifact@v3
        if: always()
        with:
          name: playwright-report
          path: playwright-report/
```

### Local CI Simulation
```bash
# Run full test suite
npm test

# Generate reports
npm run test:report

# Check browser compatibility
npm test -- --project=chromium --project=firefox --project=webkit
```

## Troubleshooting

### Tests Hang on Login
**Cause:** Backend not running or auth endpoint failing
**Fix:**
```bash
# Ensure backend is running
cargo run --release

# Check BASE_URL matches your server
BASE_URL='http://localhost:3000'
```

### "Element Not Found" Errors
**Cause:** Page layout changed or selectors outdated
**Fix:**
- Run with `--debug` to inspect selectors
- Use `page.pause()` to halt and inspect
- Update selectors in test file

### Screenshots/Videos Not Saving
**Cause:** Directory doesn't exist
**Fix:**
```bash
# Ensure directories exist
mkdir -p verification/screenshots
mkdir -p test-results
```

## Best Practices

1. **Use Meaningful Locators**
   - Prefer `id` and `data-test` attributes
   - Use semantic selectors: `button:has-text("Login")`
   - Avoid brittle CSS selectors

2. **Add Waits Strategically**
   - Use `page.waitForResponse()` for API calls
   - Use `page.waitForNavigation()` for page loads
   - Avoid hard-coded `sleep()` calls

3. **Organize Tests Into Suites**
   - Group related tests with `test.describe`
   - Use `test.beforeEach()` for setup
   - Use `test.afterEach()` for cleanup

4. **Keep Tests Independent**
   - Each test should run in isolation
   - Don't depend on test execution order
   - Use fixtures for setup/teardown

5. **Use Test Tags for Organization**
   ```typescript
   test('@smoke @auth should login', async ({ page }) => {
     // ...
   });
   
   // Run only smoke tests
   npx playwright test --grep @smoke
   ```

## Debugging Tips

### Enable Debug Logging
```bash
# Python
DEBUG=1 python verification/e2e_tests.py

# TypeScript
PWDEBUG=1 npx playwright test
```

### Inspect Selectors
```bash
npx playwright inspector
```

### Record Tests
```bash
npx playwright codegen http://localhost:3000
```

## Performance Metrics

Typical test execution times:

| Suite | Count | Duration |
|-------|-------|----------|
| Auth | 7 | ~2-3s |
| Dashboard | 8 | ~3-4s |
| Trading | 8 | ~3-5s |
| Options | 9 | ~4-6s |
| Settings | 11 | ~3-4s |
| **Total** | **43** | **~20-25s** |

*Times vary based on network latency and system load*

## Resources

- [Playwright Documentation](https://playwright.dev)
- [Playwright API Reference](https://playwright.dev/docs/api/class-browser)
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
- [pytest Documentation](https://docs.pytest.org)
- [Getting Started with Playwright](https://playwright.dev/docs/intro)
