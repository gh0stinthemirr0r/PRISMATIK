# Prismatik

**Quant research, validation, and execution platform · Mythos Systems · v0.1.0**
Author: Aaron Stovall · 2026-07-07

Prismatik is a self-hosted trading research workbench with live execution
capability. Rust (Axum) core, SvelteKit/TypeScript front end, optional Tauri
desktop shell. Crypto data via Coinbase Exchange public API (no key); order
execution via the customer's own Alpaca account and API keys.

```
prismatik/
├── server/     Rust core: engine, walk-forward, sessions, brokers, API
├── ui/         SvelteKit front end (adapter-static)
├── desktop/    Tauri 2 shell scaffold (build on a workstation)
└── README.md
```

## Run (web)

```bash
cd ui && npm install && npm run build
cd ../server && cargo build --release
PRISMATIK_UI_DIR=../ui/build ./target/release/prismatik-server
# open http://127.0.0.1:8787 — the API token is printed and injected into the page
```

## Configuration (environment)

| Variable | Default | Purpose |
|---|---|---|
| `PRISMATIK_HOST` / `PRISMATIK_PORT` | `127.0.0.1` / `8787` | Bind address (loopback by default) |
| `PRISMATIK_API_TOKEN` | generated | Bearer token for every route except health |
| `PRISMATIK_UI_DIR` | `../ui/build` | Built UI to serve |
| `PRISMATIK_FEE_RATE` / `PRISMATIK_SLIPPAGE_RATE` | `0.006` / `0.0005` | Cost model per unit turnover |
| `PRISMATIK_MAX_WEIGHT` | `1.0` | Position weight cap (no leverage above 1) |
| `PRISMATIK_MAX_DD_KILL` | `0.20` | Session drawdown kill switch |
| `PRISMATIK_MAX_NOTIONAL` | `1000` | Total exposure cap, USD |
| `PRISMATIK_MAX_ORDER` | `250` | Per-order cap, USD (orders above are rejected, never resized) |
| `PRISMATIK_JOURNAL_DIR` | `~/.prismatik/journal` | Append-only JSONL audit journals |
| `ALPACA_KEY_ID` / `ALPACA_SECRET_KEY` | unset | Customer's own Alpaca API keys |
| `PRISMATIK_LIVE_TRADING` | unset | Live gate, see below |

## Execution modes and the double gate

1. **Paper simulator (default)** — built-in account with real fee and slippage
   modeling, driven by the real Coinbase ticker.
2. **Alpaca paper** — real order flow against `paper-api.alpaca.markets` with
   the customer's keys. This is the required proving ground.
3. **Alpaca live** — `api.alpaca.markets`. Reachable only when BOTH are true:
   - the server was started with
     `PRISMATIK_LIVE_TRADING=I_ACCEPT_FULL_RESPONSIBILITY_FOR_LIVE_TRADING`
   - the session request repeats that exact phrase in `live_confirm`
     (the UI collects it in the session panel)

Two independent, deliberate acts: one by the operator at deploy time, one per
session. Every order passes pre-trade caps, carries an idempotency id, is
journaled before and after submission, and any ambiguous order state halts the
session rather than guessing. This architecture is the product's liability
posture as much as its safety posture.

## API (v1, breaking changes will ship as /api/v2)

| Route | Auth | Purpose |
|---|---|---|
| `GET /api/v1/health` | open | Liveness |
| `GET /api/v1/meta` | bearer | Strategies, risk limits, gate status |
| `POST /api/v1/jobs/backtest` | bearer | Cost-aware, lookahead-free backtest vs buy-and-hold |
| `POST /api/v1/jobs/walkforward` | bearer | Out-of-sample validation, stability, bootstrap |
| `GET /api/v1/jobs/:id` | bearer | Poll job |
| `POST /api/v1/sessions/start` | bearer | Start paper/alpaca session |
| `GET /api/v1/sessions/:id` | bearer | Status + journal tail |
| `POST /api/v1/sessions/:id/stop` | bearer | Graceful stop |
| `GET /api/v1/sessions/:id/stream?token=` | token | WebSocket event stream |

## Verification status

- `cargo test`: 18/18 passing (engine timing, cost math, drawdown, walk-forward
  accounting, bootstrap, broker gates, payload parsing)
- Contract-verified live: auth (401 without token), token injection, 422 on
  invalid strategy params, live-gate rejection without credentials, job
  lifecycle with honest failure on unreachable venue, paper session
  start/status/stop with journal on disk
- Not network-verifiable in the build container (venue domains blocked):
  Coinbase candle fetch and Alpaca order round-trips. Parsers are fixture-tested;
  first run on your machine exercises them against the real venues.

## Known scope limits (v0.1.0)

Equities data adapter (Alpaca bars) not yet in the Rust core, crypto only;
corporate-action adjustment N/A for crypto; strategy set is the transparent
baseline four plus vol targeting. The walk-forward panel reports configurations
searched and parameter stability because multiple-testing bias is the way these
products quietly lie; Prismatik surfaces it instead.
