# Frontend-Backend Endpoint Mapping

This document provides a mapping between the backend routes defined in `src/main.rs` and their corresponding frontend calls in `static/*.js` and `static/*.html`.

## Mapping Table

| Backend Route | Method | Frontend Caller | Frontend File | Description |
|---|---|---|---|---|
| `/api/login` | POST | `fetch()` | `login.html` | User authentication |
| `/api/verify` | GET | `fetchWithLogging()` | `app.js` | Token verification |
| `/api/logout` | POST | `fetchWithLogging()` / `fetch()` | `app.js`, `settings.html` | Session termination |
| `/api/config/status` | GET | `fetch()` | `settings.html` | Get API key configuration status |
| `/api/config/api-keys` | POST | `fetch()` | `settings.html` | Save Alpaca API keys |
| `/api/config/password` | POST | `fetch()` | `settings.html` | Change user password |
| `/api/account` | GET | `fetchWithLogging()` | `app.js` | Get Alpaca account info |
| `/api/positions` | GET | `fetchWithLogging()` | `app.js` | Get open positions |
| `/api/orders` | GET | `fetchWithLogging()` | `app.js` | Get orders (supports `status` query param) |
| `/api/orders` | POST | `fetchWithLogging()` | `app.js` | Create a new stock or option order |
| `/api/price/{symbol}` | GET | `fetchWithLogging()` | `app.js` | Get latest quote for a stock |
| `/api/option-strikes/{symbol}` | GET | `fetchWithLogging()` | `app.js` | Get recommended option strikes |
| `/api/option-quote/{symbol}` | GET | `fetchWithLogging()` | `app.js` | Get latest quote for an option contract |
| `/api/option-chain/{symbol}` | GET | `fetchWithLogging()` | `app.js` | Get full option chain data |
| `/api/orders/{id}` | GET | `fetchWithLogging()` | `app.js` | Get detailed information for a specific order |
| `/api/orders/{id}` | DELETE | `fetchWithLogging()` | `app.js` | Cancel a specific order |
| `/api/orders/cancel-all` | POST | `fetchWithLogging()` | `app.js` | Cancel all open orders |
| `/api/ws/prices` | GET | N/A | N/A | WebSocket for real-time prices (Currently unused by frontend) |

## Verification Findings

1.  **Missing Calls Fixed:**
    *   `POST /api/logout`: Was previously handled only on the frontend by clearing `localStorage`. Now called by the frontend to ensure backend session cleanup.
    *   `GET /api/orders/{id}`: Was defined in the backend but not used. Now used by the new "Order Details" modal feature.
2.  **Unused Backend Routes:**
    *   `/api/ws/prices`: The backend implements a WebSocket price streamer, but the frontend currently uses polling or on-demand fetches. This is flagged as an area for future enhancement.
3.  **Schema Consistency:**
    *   All identified routes use `snake_case` for request bodies (e.g., `order_type`, `limit_price`), matching the backend's expected schema.
    *   Frontend was updated to handle both `canceled` and `cancelled` status spellings for robust history tracking.
