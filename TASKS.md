# AutoStonks Algo Suite - Technical Roadmap

This document outlines 10 independent, high-impact tasks designed to improve the performance, reliability, and developer experience of the AutoStonks Algo Suite. These tasks focus on foundational "force multipliers" that enable scaling and safer trading execution.

---

## Backend Tasks (70% Focus)

### 1. Registry & Strategy Migration Completion
*   **Goal:** Fully integrate all strategy types into the new trait-based system and eliminate orphaned/dead code.
*   **Technical Approach:**
    *   Implement the `TradingStrategy` trait for `ParitySniper` and `VwapReversion`.
    *   Register all strategy kinds in the `STRATEGY_REGISTRY` in `backend/src/strategies/mod.rs`.
    *   Refactor `listing_arb.rs` to move the complex logic from `v2` into the `ListingArbitrageStrategy` implementation and remove the stub.
    *   Clean up all compiler warnings related to unused imports and dead functions in the `strategies` module.

### 2. Mission-Critical Math & Parity Verification Suite
*   **Goal:** Ensure the absolute accuracy of Black-Scholes, Option Parity, and OCC symbol parsing logic.
*   **Technical Approach:**
    *   Implement a new integration test suite in `backend/tests/math_verification.rs`.
    *   Utilize the `proptest` crate for property-based testing of Greeks and Put-Call parity calculations across a vast range of market inputs.
    *   Verify that `parity_sniper` logic correctly identifies arbitrage opportunities with zero false positives.

### 3. Market Data Request Deduplication & Caching
*   **Goal:** Reduce API latency and eliminate redundant network calls when multiple agents or UI components track the same symbols.
*   **Technical Approach:**
    *   Introduce a `MarketCache` service using an async-aware cache like `moka`.
    *   Wrap `fetch_quote` and `fetch_candles` calls to check the cache (e.g., 500ms TTL) before falling back to external providers (Alpaca/Yahoo).
    *   Integrate the cache into `AppState` for global availability.

### 4. Local Backtesting Simulation Engine
*   **Goal:** Enable historical strategy verification by replaying recorded market data.
*   **Technical Approach:**
    *   Implement a `BacktestService` that can query `market_snapshots` and `option_snapshots` from the SQLite database.
    *   Create a virtual "Clock" that advances through the snapshots, triggering the `evaluate` method of the strategy under test.
    *   Expose a new API endpoint to initiate a backtest and return a performance report.

### 5. Strategy Persistence (State JSON)
*   **Goal:** Allow strategies to maintain internal state (counters, local trends, drift values) across execution cycles without hardcoding fields in the database.
*   **Technical Approach:**
    *   Add a `state_json` TEXT/JSON column to the `strategies` table in SQLite.
    *   Modify the `TradingStrategy` trait's `evaluate` method to accept and return a `serde_json::Value`.
    *   Update the agent loop to persist the returned state back to the database after each run.

### 6. Dedicated Kronos AI Bridge Service Expansion
*   **Goal:** Upgrade the AI signal bridge to be resilient and support multi-modal signals (Trend, Volatility, Sentiment).
*   **Technical Approach:**
    *   Expand `backend/src/services/kronos.rs` with strongly-typed signal responses.
    *   Implement a "Circuit Breaker" pattern to handle AI bridge downtime gracefully, providing default "neutral" signals to strategies when the bridge is unreachable.

### 7. Event-Driven Agent Triggering System
*   **Goal:** Minimize execution slippage by moving from polling-based execution to reactive, event-driven triggers.
*   **Technical Approach:**
    *   Refactor the agent loop in `agents.rs` to wait on a `tokio::sync::broadcast` channel.
    *   Update `StreamHub` to broadcast "Tick" events whenever new market data is received via WebSocket.
    *   Trigger relevant strategy evaluations immediately upon receipt of these events.

### 8. Alpaca WebSocket Multiplexer Pool
*   **Goal:** Scale the system to monitor hundreds of symbols simultaneously by bypassing Alpaca's per-connection limits.
*   **Technical Approach:**
    *   Implement a `StreamingPool` manager that dynamically spawns and maintains multiple Alpaca WebSocket clients.
    *   Distribute symbol subscriptions across the pool based on connection load.
    *   Merge the output of all pool clients into the global `StreamHub` broadcast feed.

### 9. Pluggable Pre-Trade Risk Engine
*   **Goal:** Provide a safety layer that validates all signals against global and strategy-specific risk limits before they reach the broker.
*   **Technical Approach:**
    *   Create a `RiskEngine` service in `backend/src/services/risk.rs` defining a `RiskValidator` trait.
    *   Implement standard validators for Max Position Size, Daily Max Loss, and Pattern Day Trader (PDT) rule protection.
    *   Inject the `RiskEngine` into the trade preparation workflow.

### 10. Dynamic Strategy Parameter Form Generator
*   **Goal:** Improve developer ergonomics by automatically generating strategy configuration UIs from backend metadata.
*   **Technical Approach:**
    *   Add a `metadata()` method to the `TradingStrategy` trait that returns a JSON schema of configurable parameters.
    *   Refactor the "Settings" view in the frontend to dynamically render form inputs based on this schema.
    *   This removes the need to manually update Svelte components whenever a new strategy or parameter is added.
