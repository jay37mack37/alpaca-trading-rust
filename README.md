# Alpaca Trading Web API

Educational trading web application built with Rust for speed and reliability.

## Features

- **Fast**: Built with Axum web framework on Tokio async runtime
- **Alpaca API Integration**: Connect to Alpaca Markets for paper/live trading
- **REST API**: Clean endpoints for account, positions, and orders

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/` | API info |
| GET | `/health` | Health check |
| GET | `/api/account` | Get account information |
| GET | `/api/positions` | Get all open positions |
| GET | `/api/orders` | Get orders |
| POST | `/api/orders` | Create a new order |

## Setup

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Alpaca Markets account ([sign up free](https://alpaca.markets/))

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/jay37mack37/alpaca-trading-rust.git
   cd alpaca-trading-rust
   ```

2. Copy `.env.example` to `.env` and add your Alpaca API credentials:
   ```bash
   cp .env.example .env
   ```

3. Edit `.env` with your credentials:
   ```
   ALPACA_API_KEY=your_key_here
   ALPACA_API_SECRET=your_secret_here
   ```

4. Build and run:
   ```bash
   cargo build --release
   cargo run --release
   ```

The server will start on `http://localhost:3000`.

## Development

```bash
# Run in development mode with hot reload (requires cargo-watch)
cargo install cargo-watch
cargo watch -x run
```

## Testing

```bash
cargo test
```

## Safety Note

⚠️ **This is for educational purposes only.** 

- This project uses Alpaca's **paper trading** API by default
- Never commit your `.env` file with real API credentials
- Always test strategies in paper trading mode first

## License

MIT