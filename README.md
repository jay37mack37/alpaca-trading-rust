# AutoStonks Algo Suite

The **AutoStonks Algo Suite** is a high-performance, multi-strategy algorithmic trading workstation built with **Rust** (backend) and **Svelte** (frontend). It provides real-time market data streaming, automated strategy execution, and deep integration with Alpaca Markets.

## 🚀 Key Features

- **Multi-Strategy Workstation**: Execute and manage multiple trading strategies (VWAP Reflexive, RSI Mean Reversion, SMA Trend, Listing Arbitrage) simultaneously.
- **Kronos AI Bridge**: Advanced integration points for external AI-driven signals and trend filtering.
- **Real-time Streaming**: Sub-second market data updates and broker synchronization via WebSockets (backend) and SSE (frontend).
- **Hybrid Execution**: Support for Local Paper, Alpaca Paper, and Alpaca Live trading modes.
- **Advanced Options Support**: Specialized logic for 0DTE delta-neutral harvesting, bull/bear spreads, and Black-Scholes valuation.

## 📁 Project Structure

```text
.
├── src/                # Backend (Rust/Axum)
│   ├── handlers/       # REST API endpoint handlers
│   ├── services/       # Core business logic (DB, Streaming, Providers)
│   ├── models/         # Shared data structures and schemas
│   ├── strategies/     # Strategy evaluation logic
│   └── config/         # Environment and application configuration
├── ui/                 # Frontend (Svelte/Vite/TypeScript)
│   ├── src/
│   │   ├── components/ # Reusable UI components
│   │   ├── lib/        # API client and shared utilities
│   │   └── App.svelte  # Main dashboard entry point
└── data/               # Persistent storage (SQLite)
```

## 🛠 Setup

### Prerequisites

- **Rust**: 1.75+
- **Node.js**: 18+ (for the frontend)
- **Alpaca API Keys**: Required for live/paper trading.

### Backend Setup

1. Copy `.env.example` to `.env`.
2. Configure your `AUTO_STONKS_MASTER_KEY` and other environment variables.
3. Run the backend:
   ```bash
   cargo run --release
   ```

### Frontend Setup

1. Navigate to the `ui/` directory.
2. Install dependencies:
   ```bash
   npm install
   ```
3. Set your `VITE_API_TOKEN` (printed by the backend on first start) in `ui/.env`.
4. Run in development mode:
   ```bash
   npm run dev
   ```

## ⚖️ Safety Note

⚠️ **This is for educational purposes only.** Algorithmic trading involves significant risk. Always test thoroughly in paper trading mode before committing real capital.

## 📄 License

MIT
