#!/bin/bash
# Quick reference for running Playwright E2E tests
# Copy and paste commands from below into your terminal

# ============================================================
# PYTHON E2E TESTS (Recommended for quick testing)
# ============================================================

echo "=== PYTHON TESTS ==="

# 1. Set up Python environment (first time only)
python -m venv venv
source venv/Scripts/activate  # On Windows
# source venv/bin/activate    # On Mac/Linux

# 2. Install Python dependencies
pip install playwright pytest pytest-asyncio

# 3. Start backend server in Terminal 1
cargo run --release

# 4. Run tests in Terminal 2
python verification/e2e_tests.py

# Results will be in:
# - verification/screenshots/ (one per test)
# - verification/test_results.json


# ============================================================
# TYPESCRIPT E2E TESTS (Recommended for CI/CD)
# ============================================================

echo "=== TYPESCRIPT TESTS ==="

# 1. Navigate to e2e directory and install (first time only)
cd e2e
npm install

# 2. Run all tests (auto-starts backend)
npm test

# 3. Run tests in UI mode (interactive)
npm run test:ui

# 4. Run tests with browser visible
npm run test:headed

# 5. Debug mode (step through)
npm run test:debug

# 6. Run specific test file
npx playwright test tests/auth.spec.ts

# 7. Run specific test by name
npx playwright test -g "should reject invalid credentials"

# 8. View HTML report
npm run test:report

# Results will be in:
# - playwright-report/index.html (interactive report)
# - test-results.json
# - junit.xml


# ============================================================
# DEVELOPMENT WORKFLOW
# ============================================================

echo "=== DEVELOPMENT ==="

# Terminal 1: Start backend server
cargo run --release

# Terminal 2: Watch mode for tests (TypeScript)
cd e2e
npm test -- --watch

# Make code changes, tests re-run automatically


# ============================================================
# DEV CONSOLE
# ============================================================

echo "=== DEV CONSOLE ==="

# Option 1: Click the 🛠️ Dev Console button in the app header
# Option 2: Press Ctrl+Shift+D (Cmd+Shift+D on Mac)
# Features:
# - Console tab: View app logs
# - Network tab: Inspect API calls
# - State tab: View application state


# ============================================================
# TROUBLESHOOTING
# ============================================================

echo "=== TROUBLESHOOTING ==="

# Problem: Tests fail with "Connection refused"
# Solution: Make sure backend is running with: cargo run --release

# Problem: Browser not found
# Solution: Install browsers with: npx playwright install

# Problem: Element not found in tests
# Solution: Run debug mode: npx playwright test --debug

# Problem: Tests timeout
# Solution: Increase timeout in playwright.config.ts


# ============================================================
# USEFUL COMMAND REFERENCE
# ============================================================

echo "=== COMMAND REFERENCE ==="

# Check Node.js installation
node --version
npm --version

# Check Python installation
python --version

# List all available Playwright tests
npx playwright test --list

# Run tests in specific order
npx playwright test tests/auth.spec.ts tests/dashboard.spec.ts

# Run tests on specific browser only
npx playwright test --project=chromium
npx playwright test --project=firefox
npx playwright test --project=webkit

# Run tests with specific number of workers
npx playwright test --workers=1     # Sequential
npx playwright test --workers=4     # 4 parallel

# Clear test artifacts
rm -rf playwright-report/
rm -rf test-results/
python -c "import shutil; shutil.rmtree('verification/screenshots/', ignore_errors=True)"

# Update browsers to latest
npx playwright install

# Record new test interactively
npx playwright codegen http://localhost:3000

# Open playwright inspector
npx playwright inspector


# ============================================================
# GIT WORKFLOW
# ============================================================

echo "=== GIT ==="

# Stage test files
git add e2e/ verification/ PLAYWRIGHT_TESTING.md E2E_SETUP_GUIDE.md

# Commit
git commit -m "feat: Add comprehensive Playwright E2E testing framework"

# Push
git push origin main


# ============================================================
# CI/CD SETUP (Optional)
# ============================================================

echo "=== CI/CD ==="

# Create GitHub Actions workflow
mkdir -p .github/workflows
# Copy workflow content from E2E_SETUP_GUIDE.md
# Save as .github/workflows/e2e-tests.yml

# Push to GitHub - tests will run automatically on PR


# ============================================================
# PERFORMANCE BENCHMARKS
# ============================================================

echo "=== PERFORMANCE ==="

# Expected test execution times (all tests, all browsers):
# Auth tests: 2-3s
# Dashboard tests: 3-5s  
# Trading tests: 3-5s
# Options tests: 4-6s
# Settings tests: 3-5s
# Total: 20-30s

# Run and time
time npm test


# ============================================================
# DOCUMENTATION
# ============================================================

echo "=== DOCUMENTATION ==="

# Read complete setup guide
cat E2E_SETUP_GUIDE.md

# Read Playwright testing guide
cat PLAYWRIGHT_TESTING.md

# Read implementation summary
cat PLAYWRIGHT_IMPLEMENTATION_COMPLETE.md

# Playwright official docs
open https://playwright.dev/docs/intro


# ============================================================
# ONE-LINER QUICK START
# ============================================================

echo "=== QUICK START (Copy & Paste) ==="

echo "Python tests:"
echo "cargo run --release &"
echo "python verification/e2e_tests.py"

echo "TypeScript tests:"
echo "cd e2e && npm install && npm test"

echo "Interactive mode:"
echo "cd e2e && npm run test:ui"
