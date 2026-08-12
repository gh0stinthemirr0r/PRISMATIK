# PRISMATIK

**A measured terminal for markets.**
Mythos Systems · Aaron Stovall · 2026

PRISMATIK is a desktop trading terminal built on a single rule: **every number
it shows must be traceable to something it measured.** Where there is no
measurement, it draws the absence rather than filling it. Forecasts are scored
against a climatology baseline and reported as skill *over* that baseline, so a
model that cannot beat the base rate is labelled unproven rather than confident.

It is research and decision-support software. It is not investment advice, and
nothing it displays is a recommendation to buy or sell any instrument.

---

## Table of contents

- [What it is](#what-it-is)
- [Design commitments](#design-commitments)
- [Architecture](#architecture)
- [The predictive loop](#the-predictive-loop)
- [Agents](#agents)
- [Autonomy and execution safety](#autonomy-and-execution-safety)
- [Repository layout](#repository-layout)
- [Getting started](#getting-started)
- [Development](#development)
- [Platform notes](#platform-notes)
- [Data providers](#data-providers)
- [Security posture](#security-posture)
- [License](#license)

---

## What it is

A Tauri 2 desktop application — Rust backend, SvelteKit/Svelte 5 frontend —
that combines four things usually kept apart:

| Layer | What it does |
| --- | --- |
| **Market truth** | Live tracking of equities and crypto, cross-venue, with an honest empty state when a feed is down. |
| **Deterministic analytics** | Regime classification, empirical forecasting, volatility and survival models — pure Rust, no model calls, reproducible from the same inputs. |
| **Scored cognition** | Frontier models (OpenAI, Anthropic, Google) and persistent specialist analysts, each a calibration cohort scored on resolved forecasts. |
| **Gated execution** | Advisory, paper and live modes, with every gate re-evaluated on every decision. |

## Design commitments

These are not aspirations; they are enforced in code and in tests.

**1. Measured, not asserted.**
Forecast quality is a Brier **skill score against climatology** — the base rate
computed from the instrument's own history — never against an earlier window of
the same model, which measures drift rather than skill. Cohorts are keyed on
`(provider_id, model, target, horizon_minutes)`, so one model's good record on
one horizon cannot vouch for another.

**2. Absence is reported.**
Retrieval always emits a gap section listing terms and symbols the corpus could
not cover. Forecasts declare `sufficient: false` when the sample is too small.
Every data surface has a `NoData` state. A retrieval that returns six documents
for a question the corpus knows nothing about launders absence into confidence.

**3. Quoted voices are data.**
Every piece of evidence, transcript line and feed item reaches a model inside
explicit fences marking it as material to respond to, never as instructions to
follow — however it is phrased.

**4. Live execution is gated on measurement.**
An agent cannot trade real money because it sounds confident. It trades when it
has a measured, positive, resolved track record — and every gate is re-checked
on every single decision.

## Architecture

```
PRISMATIK/
├── crates/                     39 Rust crates, layered by dependency depth
│   ├── prismatik-determinism/  Clocks, content hashing, reproducibility
│   ├── prismatik-regime/       Regime classification, empirical forecasting
│   ├── prismatik-brain/        BM25 + graph knowledge retrieval
│   ├── prismatik-calibration/  Cohorts, Brier scoring, skill
│   ├── prismatik-quant-kernel/ Numeric core
│   ├── prismatik-risk/         Limits, exposure, drawdown ladder
│   ├── prismatik-execution/    Order lifecycle
│   └── …                       market data, options, filings, backtest, …
│
└── apps/desktop/
    ├── src-tauri/src/          Tauri commands — the desk's runtime
    │   ├── analysts.rs         Persistent specialist experts (scored cohorts)
    │   ├── room.rs             Multi-agent debate with fenced transcripts
    │   ├── knowledge.rs        Document store over prismatik-brain
    │   ├── signal.rs           Edge-gated trading signal
    │   ├── autonomy_mode.rs    Advisory / Paper / Live gate evaluation
    │   ├── quant_context.rs    Deterministic analytics for agent packets
    │   ├── scheduler.rs        Supervised loop
    │   └── …
    └── src/routes/workspace/   ~30 surfaces, grouped by the question asked
```

The Rust workspace and `apps/desktop/src-tauri` are **separate** Cargo
workspaces; the desktop crate is deliberately excluded from the root workspace.

### Navigation

The rail groups surfaces by the question the desk is asking:

- **Market** — what is happening (instruments, options, macro, filings, screener)
- **Intelligence** — what we believe (news, feeds, predictions, analogs)
- **Research** — what we are testing (strategy, backtest, simulation, calibration, council, analysts, knowledge)
- **Execution** — what we are doing about it (trader, orders, risk, agent log, journal)
- **System** — integrations, autonomy, extensions, about

## The predictive loop

1. **Regime classification** — realized-volatility percentile against the
   instrument's own history, a Lo–MacKinlay overlapping variance ratio, and a
   drift *t*-statistic, combined with hysteresis bands so the classification
   does not flip on noise. Five regimes: calm-trending, calm-mean-reverting,
   volatile-trending, volatile-mean-reverting, crisis.

2. **Empirical forecast** — conditional on the regime, produces P(up), P(down),
   P(flat) with a 5bps flat band, plus the **climatology** probability and the
   **edge** over it. Every forecast reports whether its sample was sufficient.

3. **Filing** — the forecast becomes a candidate with an explicit resolution
   horizon, filed against a cohort.

4. **Resolution and scoring** — when the horizon elapses the outcome resolves,
   the Brier score is taken on the probability of the *stated direction*, and
   the cohort's skill against climatology updates.

5. **Signal** — a trade signal is only produced when the edge clears
   `MIN_EDGE_PPM`; conviction and size come from *measured* cohort skill. A
   negative-skill cohort sizes to zero.

The loop is closed. Nothing in it is a placeholder.

## Agents

### Specialist analysts

You can author a persistent expert scoped to **one instrument**, a **sector**,
or the **whole desk** (`/workspace/analysts` · rail: Research → Analysts). Each
analyst carries a mandate describing what it specializes in and how it should
reason.

Each analyst is its own calibration cohort (`analyst-{ts}.v{n}`), scored through
the same path as the statistical forecaster. Editing the mandate bumps the
version and starts a fresh cohort — a changed mandate is a changed analyst.
Renaming does *not* bump it, because that would let a bad record be laundered
by a new name. Until an analyst has resolved forecasts it reads
`unproven — no resolved forecasts yet`, and an analyst scoped to a symbol the
desk does not track says so on the roster, because it will be reasoning from
prose alone.

### The room

Analysts and standing critics (bear, bull, regime critic, risk) can be convened
in **PRISMATIK COMMAND → Room**. Participants answer each other, with every
quoted voice fenced as data. Bounded at 6 turns per round and 400 messages.

### Knowledge base

Documents — notes, filings, news, research, decisions, transcripts — indexed by
BM25 plus graph traversal over instrument links. Entity extraction is
zero-LLM, bounded by the vocabulary of instruments the desk actually tracks, so
an extracted symbol is always a real one. Nothing PRISMATIK *computes* is stored
here: regime, edge and skill are recomputed on every read and would be stale the
moment they were written down.

## Autonomy and execution safety

Three modes: **Advisory** (recommend only), **Paper** (simulated fills through
the paper OMS), **Live** (real orders).

Live requires **all** of the following, re-evaluated on **every decision**:

- an arming window that expires (≤ 8 hours)
- a connected broker
- ≥ 40 resolved forecasts in the cohort
- ≥ 5 percentage points of measured skill over climatology
- an armed circuit breaker
- a drawdown ladder within limits
- remaining budget

Any failure downgrades the decision to paper and records the reasons. The
**Agent log** (rail: Execution → Agent log) shows what the agent did and why,
including every downgrade.

## Repository layout

| Path | Contents |
| --- | --- |
| `crates/` | 39 Rust library crates |
| `apps/desktop/` | Tauri shell — SvelteKit frontend + `src-tauri` backend |
| `packages/design-tokens/` | Shared tokens, contrast checking |
| `scripts/` | Determinism checks, NOTICE generation |
| `.github/workflows/` | CI across Linux, Windows and macOS |

## Getting started

### Prerequisites

- **Rust 1.88.0** (pinned in `rust-toolchain.toml`)
- **Node 20+** and **pnpm 10.13.1**
- Tauri 2 system dependencies for your platform — see
  [tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/)

On Debian/Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

### Install and run

```bash
pnpm install
```

```bash
pnpm tauri dev
```

The frontend alone (no Tauri commands available, so most surfaces will show
their empty state) can be run with:

```bash
pnpm dev
```

### Build a release bundle

```bash
pnpm tauri build
```

## Development

Run the Rust workspace tests:

```bash
cargo test --workspace
```

Run the desktop backend tests:

```bash
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
```

Lint and format — CI runs clippy with `-D warnings`:

```bash
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
```

Typecheck the frontend:

```bash
pnpm check
```

## Platform notes

PRISMATIK draws its own window chrome, and that is a different job on each host:

- **Linux / Windows** — the window runs `decorations: false`; the frontend
  supplies minimise / maximise / close and resize grips. Window state is
  persisted with `StateFlags::all() & !StateFlags::DECORATIONS`, because
  restoring the decoration flag brings the system title bar back.
- **macOS** — configured in `tauri.macos.conf.json` with `decorations: true`,
  `titleBarStyle: "Overlay"` and `hiddenTitle`, so the system keeps the traffic
  lights. Frontend controls are suppressed there, and resize grips are not
  drawn: tao returns `NotSupported` for `drag_resize_window` on macOS, so a grip
  cannot resize — but its hit strip would still swallow the mousedown and block
  the native edge resize.

CI builds the desktop application on all three platforms.

## Data providers

- **Equities and crypto** — TradingView's scanner endpoint, ranked by
  **turnover**, not unit volume (volume is meaningless across price scales and
  surfaces OTC shells). Turnover uses the per-market field: `Value.Traded` for
  equities, `24h_vol|5` for crypto.
- **Charting** — TradingView Lightweight Charts v4.2.0 (Apache-2.0), rendered
  in-process. The TradingView *widget* is not used: it cannot satisfy the
  content security policy.
- **Frontier models** — OpenAI, Anthropic and Google, each supporting OAuth
  tokens, session tokens or API keys, configured under
  System → Integrations.

## Security posture

- **Strict CSP.** `default-src 'self'; script-src 'self'; style-src 'self';
  connect-src 'self' ipc: http://ipc.localhost; frame-ancestors 'none'`. Inline
  `style` attributes are dropped; dynamic styling goes through the CSSOM.
- **Least-privilege capabilities.** Tauri capability grants are pruned to the
  commands actually invoked.
- **Prompt-injection fencing.** All untrusted content — feeds, documents,
  transcripts — is fenced and labelled as data before it reaches a model.
- **Tool classes.** Agent tools are declared read-only; nothing an agent can
  call writes to a broker directly.
- **Determinism and audit.** Content-hashed journal records with an integrity
  check over the *stored bytes*, so adding a field does not invalidate history.

## License

Proprietary. © 2026 Aaron Stovall · Mythos Systems.
