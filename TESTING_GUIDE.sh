#!/usr/bin/env bash
# Test Execution Quick Reference
# Usage: Run commands from the repo root directory

# ============================================================================
# BASIC TEST COMMANDS
# ============================================================================

# Run all tests
cargo test

# Run tests with output displayed
cargo test -- --nocapture

# Run specific test
cargo test test_hash_password_consistency

# Run tests in a specific module
cargo test auth::tests
cargo test models::account::tests
cargo test api::alpaca::tests
cargo test route_tests::

# ============================================================================
# DETAILED TEST RUNS
# ============================================================================

# Show test names without running (dry run)
cargo test -- --list

# Run one test at a time (serial)
cargo test -- --test-threads=1

# Run with backtrace for debugging
RUST_BACKTRACE=1 cargo test test_hash_password_consistency -- --nocapture

# Run only integration tests
cargo test --test routes_integration_tests

# Run only unit tests
cargo test --bin alpaca_trading3web

# ============================================================================
# TEST FILTERING
# ============================================================================

# Run all tests matching "option" in name
cargo test option

# Run all tests matching "auth" in name
cargo test auth

# Run all serialization tests
cargo test serialization

# Run all deserialization tests
cargo test deserialization

# ============================================================================
# TEST ANALYSIS & REPORTING
# ============================================================================

# Show which tests compiled/exist
cargo test -- --list

# Count total tests
cargo test 2>&1 | grep "running"

# Show test times
cargo test -- --nocapture --test-threads=1

# Generate coverage report (requires cargo-tarpaulin)
# cargo install cargo-tarpaulin
cargo tarpaulin --out Html --output-dir coverage

# ============================================================================
# PERFORMANCE & CHECKING
# ============================================================================

# Check code compiles without running tests
cargo check

# Build in release mode and then test
cargo test --release

# Check for unused code
cargo clippy

# Format check
cargo fmt --check

# ============================================================================
# PRACTICAL EXAMPLES
# ============================================================================

# When adding new code - verify it works
cargo test new_feature_name

# Before committing - run full test suite
cargo test

# Debug a failing test
RUST_BACKTRACE=full cargo test failing_test_name -- --nocapture --test-threads=1

# Verify all model serialization
cargo test serialization

# Verify all model deserialization
cargo test deserialization

# Check auth system thoroughly
cargo test auth::

# Verify API client behavior
cargo test alpaca::tests::

# Check REST endpoint patterns
cargo test route_tests::

# ============================================================================
# EXPECTED OUTPUT
# ============================================================================

# Successful test run includes:
# running 50 tests
# ...
# test result: ok. 50 passed; 0 failed

# Expected totals:
# - Unit tests (src): 50 tests
# - Integration tests (tests/): 31 tests
# - Setup binary: 0 tests
# TOTAL: 81 tests

# ============================================================================
# TROUBLESHOOTING
# ============================================================================

# If tests fail to compile:
cargo clean
cargo build
cargo test

# If you get "library target not found":
# This project is a binary, not a library - use 'cargo test' directly

# If tests are too slow:
cargo test --release  # Release mode is faster

# If you need to debug a specific issue:
# 1. Run the test with output: cargo test test_name -- --nocapture
# 2. Check src/auth.rs, src/api/alpaca.rs, src/models/*, tests/
# 3. Review error messages for assertion failures

# ============================================================================
# TEST ORGANIZATION
# ============================================================================

# Authentication (src/auth.rs)           - 19 tests
# API Client (src/api/alpaca.rs)        - 24 tests
# Models (src/models/)                  - 27 tests
#   - account.rs                         - 3 tests
#   - position.rs                        - 4 tests
#   - order.rs                           - 3 tests
#   - option_chain.rs                    - 9 tests
# Integration (tests/routes_*.rs)       - 31 tests
#   - request/response validation
#   - error handling
#   - business logic validation

# ============================================================================
