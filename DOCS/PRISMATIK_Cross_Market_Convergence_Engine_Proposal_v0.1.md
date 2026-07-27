# PRISMATIK — Cross-Market Convergence Engine

**Proposal v0.1 · Trend, Dependence, and Synthesis across US Equities and Crypto**
**Status:** PROPOSAL — not accepted, not scheduled, not costed
**Date:** 2026-07-27

---

## 0. How to Read This Document

This is a design proposal for a subsystem that does not exist yet. It is written against the
Unified Solution Architecture v1.0 and the Wave plan, and it deliberately reuses their vocabulary
(invariants, gates, pinned artifacts, evidence chains) because a feature proposal that cannot be
expressed in the platform's existing invariants is a feature proposal for a different platform.

Confidence markers follow the wave-plan convention: **H** = well-understood engineering,
**M** = understood but non-trivial, **L** = research-grade, may not survive contact with data.
Anything marked **L** should be prototyped in a sidecar before it earns a crate.

Section 16 lists what I would cut first. Read it before Section 5.

---

## 1. What the Solution Is Today

Thirty-six crates in a strict four-layer dependency order, all compiling, all passing
`clippy -D warnings`, most at contract-surface depth rather than production logic. The load-bearing
part — `prismatik-determinism` — is genuinely implemented, and `prismatik-identity` is real enough
that bitemporal symbology is no longer a promise.

The architecture's actual claim is narrow and unusually honest: *breadth is commodity, models are
commodity, the only defensible asset is the ability to prove what you knew and when.* Seven
invariants, each with a named CI gate. A pinned calendar artifact. A pinned tokenizer codebook.
Conformal calibration as the only path from model output to a rendered number.

**That constraint set is not a tax on this proposal. It is the reason this proposal is interesting.**

A cross-market correlation engine is one of the easiest things in finance to build badly and one of
the hardest to build honestly. Every failure mode below is a determinism, point-in-time, or
statistical-disclosure failure — which is to say, every one of them is something PRISMATIK already
has machinery to prevent and no competitor does.

---

## 2. Thesis

> **Crypto and US equities are two clocks observing one risk appetite. The product is the
> instrument that reads both clocks correctly and reports the disagreement with an error bar.**

Crypto trades 168 hours a week. US equities trade about 32.5, plus extended sessions, minus
holidays and early closes, with halts. For roughly 80% of wall-clock time, crypto is the only
continuously-priced liquid risk asset in the world. That is not a data-alignment nuisance. It is a
**standing information asymmetry** that almost no retail-facing product exploits correctly, because
exploiting it requires a calendar you can trust to the minute and a point-in-time discipline that
survives an eighteen-month backtest.

PRISMATIK already owns both.

Four capabilities follow from the thesis. Each is a product surface, each is defensible, and each is
a straight-line consequence of an invariant the platform already enforces:

| # | Capability | Rests on |
|---|---|---|
| C1 | Read the overnight crypto tape as a forecast of the US equity open, with calibrated intervals | I2 (calibration), §12.4 (pinned calendar) |
| C2 | Measure *directional* information flow between markets, not symmetric correlation | I1 (evidence), I5 (point-in-time) |
| C3 | Detect the moment cross-market dependence structurally breaks, and say so before the price does | I3 (determinism), §17.4 (drift) |
| C4 | Attribute today's move across both markets to shared latent factors, with residual disclosed | I1, I2 |

---

## 3. The Five Ways Every Existing Product Gets This Wrong

Stating these first, because each one becomes a gate in Section 15.

**F1 — Clock laundering.** Daily-close correlation between BTC and SPY compares a 00:00 UTC snapshot
to a 16:00 ET snapshot. The overnight equity gap silently absorbs 16 hours of crypto information,
which inflates measured correlation and destroys measured lead-lag. Almost every published
crypto/equity correlation number has this bug.

**F2 — Synchronicity assumption.** Pearson correlation on resampled series assumes both assets are
observed at the same instants. They are not. Resampling to a common grid introduces the
Epps effect: measured correlation collapses toward zero as the sampling interval shrinks, purely as
an artifact. Products report this artifact as a finding.

**F3 — Multiple testing.** A 500-name universe has 124,750 pairs. At p < 0.05 you get roughly 6,200
"significant" correlations from pure noise. A heatmap with no false-discovery control is a
random-number generator with a color ramp.

**F4 — Effective sample size.** Rolling 90-day correlation on autocorrelated daily returns has an
effective N far below 90. The confidence interval on r = 0.62 with 90 overlapping observations is
wide enough to include 0.35. Nobody draws it.

**F5 — Regime blindness.** "BTC and NASDAQ correlate at 0.6" is a statement with no truth value. The
number is near zero in calm regimes and near 0.9 in liquidity-stress regimes, and the average is a
number that describes no actual market state. Correlation reported unconditional on regime is
correlation reported wrong.

**A correlation surface that renders without an error bar, an effective sample size, an FDR
adjustment, and a regime label is a conclusion without evidence. Invariant I1 already forbids it.**

---

## 4. Proposed Crate Topology

Three new Layer 2 domain crates, following the existing dependency rules (ports only, no storage
backends, no vendor SDKs, no HTTP clients).

```text
Layer 2 — Domain

prismatik-lattice          NEW   Cross-venue temporal alignment. The clock.
  deps: prismatik-domain, prismatik-calendar, prismatik-determinism

prismatik-dependence       NEW   Estimators: dependence, lead-lag, information flow.
  deps: prismatik-domain, prismatik-lattice, prismatik-determinism

prismatik-regime           NEW   State inference, change points, regime artifacts.
  deps: prismatik-domain, prismatik-dependence, prismatik-features

prismatik-convergence      NEW   Synthesis: factor attribution, network structure,
                                 the Market State Tensor, conclusion assembly.
  deps: prismatik-regime, prismatik-dependence, prismatik-analog-store,
        prismatik-calibration, prismatik-market-data
```

Existing crates that gain surface, not new crates:

- `prismatik-features` — new `FeatureView`s per Section 11, each with an honest `observation_delay`.
- `prismatik-risk` — three new pre-trade checks (Section 14).
- `prismatik-renderer` — three new GPU surfaces (Section 13).
- `prismatik-calibration` — the regime labels this engine emits become the **Mondrian categories**
  the conformal calibrator already wants. See Section 9.3. This is the single cleanest architectural
  synergy in the proposal.
- `prismatik-analog-store` — the cross-market state vector becomes an embedding modality.

Sidecar (Python, isolated, untrusted-output, per §9.1): heavy estimation that is research-grade —
graphical lasso, VAR/spillover decomposition, HMM fitting. Rust owns the online path and the
contract; the sidecar owns the fitting. Its output is stamped with provenance like any external
provider, per the existing sidecar rule.

---

## 5. Layer 1 — The Synchronized Clock Lattice (`prismatik-lattice`)

**Confidence: H. This is the foundation and it is ordinary engineering. Build it first.**

The lattice is a canonical bitemporal grid onto which both markets project without either one being
distorted. It has three projections, and the choice is explicit at the call site — never a default.

```rust
/// How a series is projected onto the shared lattice. There is no default,
/// because every default is wrong for some pair, and a silent wrong default
/// is exactly the F1 failure this crate exists to prevent.
pub enum LatticeProjection {
    /// Continuous 24/7 grid. Equity series carry an explicit SessionMask
    /// marking closed intervals. Correlation estimators MUST consume the
    /// mask; an estimator that ignores it fails the conformance gate.
    Continuous { step: Duration },

    /// Equity session boundaries from the pinned calendar artifact.
    /// Crypto is aggregated into session buckets, and the overnight bucket
    /// (prior close -> next open) is a first-class observation, not a gap.
    SessionAligned { calendar: ArtifactRef, venue: MicCode },

    /// No projection. Estimators operate on raw irregular tick times.
    /// Required for the Hayashi-Yoshida path (Section 6.2).
    Asynchronous,
}

/// The overnight bucket. This type is the product.
pub struct OvernightWindow {
    pub prior_close: Instant,      // pinned-calendar close, incl. early closes
    pub next_open: Instant,        // pinned-calendar open
    pub crypto_path: PathSummary,  // realized vol, drift, max excursion, volume
    pub calendar: ArtifactRef,     // which calendar version produced these bounds
    pub session_kind: SessionKind, // Regular | EarlyClose | Holiday | Weekend
}
```

**Why the pinned calendar becomes a competitive weapon here.** The overnight window's boundaries are
determined entirely by the calendar. A calendar library that quietly revises a 2019 early close in a
point release changes the boundary, changes the aggregation, changes the correlation, changes the
backtest — and no seed, hash, or signature catches it. §12.4 already makes the calendar a signed,
content-addressed artifact. This is the first subsystem where that decision pays a visible dividend
rather than an invisible one.

**Gate `test_lattice_no_clock_laundering`:** for a random asset pair spanning both markets and a
random window, assert every returned observation pair has overlapping observability intervals under
the declared projection, and that no equity observation is drawn from a masked interval.

---

## 6. Layer 2 — Dependence Estimation (`prismatik-dependence`)

Pearson is one estimator in a ladder, and never the default.

### 6.1 The estimator ladder

| Estimator | Answers | Conf |
|---|---|:---:|
| Pearson / Spearman with ESS-corrected CI | "Do they move together, linearly / monotonically?" | H |
| **Hayashi–Yoshida** | "…without resampling either series, on their native tick times" | M |
| Distance correlation | "Do they depend at all, including non-linearly?" | M |
| Tail dependence coefficient (λ_L, λ_U) | "Do they crash together?" — the only question that matters in stress | M |
| Partial correlation (graphical lasso precision matrix) | "Do they move together *after removing the common factor*?" | L |
| **Transfer entropy** / Granger causality | "Which one moves *first*, and how much does knowing A reduce uncertainty about B?" | L |
| **Diebold–Yilmaz spillover** (VAR FEVD) | "How much of B's forecast error variance originates in A?" | L |

**Hayashi–Yoshida deserves its bold.** It was designed precisely for non-synchronously observed
asset pairs and estimates covariance without interpolation or resampling, which makes it immune to
the Epps effect (F2). Crypto/equity is the canonical asynchronous pair — one series has gaps
measured in hours. Using HY here is not exotic; it is the correct tool, and using Pearson-on-a-
resampled-grid instead is the standard error the whole industry makes.

**Tail dependence deserves a mention it never gets.** Correlation is an average over the whole
distribution. Portfolio survival depends only on the left tail. An asset pair with ρ = 0.3 and
λ_L = 0.7 is a diversification illusion — uncorrelated on ordinary days, joined at the hip on the
day it matters. Reporting λ_L alongside ρ on every pair is a small feature with a large honesty
payoff, and it is nearly free once the lattice exists.

### 6.2 The output type is where the honesty lives

```rust
/// No public constructor omits the disclosure fields. Same enforcement
/// pattern as ModelPrediction / CalibrationRecord under Invariant I2 —
/// the type system carries the discipline, not the reviewer.
pub struct DependenceEstimate {
    pub pair: (AssetId, AssetId),
    pub estimator: EstimatorKind,
    pub point: f64,

    /// Interval, not a scalar. I2 applies to dependence estimates exactly
    /// as it applies to forecasts.
    pub interval: ConfidenceInterval,

    /// Nominal N is a lie under autocorrelation and overlapping windows.
    /// This is the number that drives the interval width.
    pub effective_sample_size: f64,

    /// FDR-adjusted. Raw p-values are not exposed at all — exposing them
    /// invites the F3 error and there is no legitimate use for them here.
    pub q_value: f64,
    pub fdr_method: FdrMethod,        // BenjaminiYekutieli under dependence
    pub family_size: usize,           // how many pairs were tested together

    /// The estimate is conditional on this regime. Unconditional
    /// dependence estimates are not representable by this type. That is
    /// deliberate (F5).
    pub regime: RegimeLabel,

    pub projection: LatticeProjection,
    pub stationarity: StationarityVerdict,   // ADF/KPSS + structural break flag
    pub evidence: Vec<EvidenceRef>,
    pub lattice_calendar: ArtifactRef,
}
```

**Benjamini–Yekutieli, not Benjamini–Hochberg.** BH assumes positive regression dependence across
tests. Correlation-matrix entries are massively and arbitrarily dependent on each other. BY is valid
under arbitrary dependence at the cost of a log-factor of power, and losing power is the correct
trade when the alternative is systematically publishing noise.

**Gate `test_no_unqualified_dependence`:** every rendered dependence value resolves to a
`DependenceEstimate` with non-empty evidence, finite ESS, and a regime label. Mirrors
`test_no_orphan_conclusions`.

---

## 7. Layer 3 — Structure: the Market as a Graph

A 500 × 500 matrix is data. A graph is understanding. Three reductions, all standard in
econophysics, none common in retail products:

**Correlation metric embedding.** d(i,j) = √(2(1−ρ_ij)) is a true metric. Under it, the market is a
metric space and every metric-space tool becomes available: MST, hierarchical clustering, MDS
embedding into two dimensions for rendering.

**Minimum spanning tree.** N−1 edges out of N(N−1)/2. The MST is the skeleton of the market: it
reveals hub assets, the cluster boundary between crypto and equities, and — critically — *when a
bridge edge forms between the two clusters*. In calm regimes crypto and equities are two loosely
connected subtrees. In stress they collapse into one. **The MST's cross-market edge count is a
single scalar that measures market unification, and it moves before headline correlation does.**

**Partial-correlation network (graphical lasso).** Strips the common factor. BTC and COIN correlate
at 0.8; the interesting question is how much survives after conditioning on the whole system. An
edge that survives partial correlation is a direct link; one that vanishes was a shadow of a common
driver. This distinction is the difference between "these move together" and "this drives that."

**Directed spillover network.** Diebold–Yilmaz FEVD over a rolling VAR gives a directed, weighted
adjacency matrix: how much of asset B's 10-step forecast-error variance is attributable to shocks in
asset A. Summed, it produces a single **System Spillover Index** — one number for "how contagious is
this market right now" — plus per-node net-transmitter/net-receiver scores. A node flipping from net
receiver to net transmitter is a regime event.

---

## 8. Layer 4 — Regime (`prismatik-regime`)

**Confidence: M for detection, L for labeling.**

Regimes are inferred from a cross-market state vector, not from price alone:

- Realized volatility, both markets, multiple horizons
- Cross-market MST bridge-edge count and System Spillover Index (Section 7)
- Term structure of implied vol (equity) and perp funding rate / futures basis (crypto)
- Breadth in both markets
- A liquidity proxy: stablecoin net issuance for crypto, dealer positioning for equities
- Realized dispersion: are names moving idiosyncratically or as one block?

Two detectors, run in parallel, because they fail differently:

1. **Bayesian Online Change Point Detection** — emits a run-length posterior, i.e. "probability the
   current regime started k bars ago." Naturally probabilistic, no forced labeling, no lookahead.
2. **Hidden Markov / Markov-switching model** — emits state posteriors and a transition matrix, so
   you get "probability of transitioning out of the current state in the next 5 days."

Neither is allowed to emit a hard label without a posterior. `RegimeLabel` carries
`posterior: f64` and `run_length_posterior: Distribution`, and an unlabeled `Indeterminate` state is
first-class and expected to be used often. The flow-classification doctrine already in the
architecture — *a classifier that always decides is a classifier that is often wrong* — transfers
verbatim.

### 8.3 The synergy worth building for

The architecture already commits to **Mondrian (regime-conditioned) conformal calibration**, and
already commits to displaying per-regime realized coverage. But nothing in the current spec says
where regime labels come from. `prismatik-regime` is that source.

The loop closes: the convergence engine emits regime labels → the calibrator conditions on them →
per-regime coverage curves are computed → and *coverage degradation within a regime becomes itself a
regime-change signal*, because a regime whose calibration is decaying is a regime that is ending.
That feedback path is a genuinely novel monitoring primitive and it costs almost nothing once both
halves exist.

---

## 9. Layer 5 — Convergence and Synthesis (`prismatik-convergence`)

This is what the request called "converge and synthesize."

### 9.1 The Market State Tensor

One object per lattice instant:

```text
        assets  ×  modality  ×  horizon
                   ├─ price / return
                   ├─ realized & implied volatility
                   ├─ flow          (options flow, exchange netflow)
                   ├─ positioning   (COT, funding rate, 13F, dealer gamma)
                   ├─ liquidity     (spread, depth, stablecoin supply)
                   ├─ narrative     (filing & news embeddings, LanceDB)
                   └─ macro         (FRED vintage-correct)
```

Every slice carries its own `observation_delay`, so the tensor is point-in-time correct by
construction rather than by discipline. This is the existing feature-store rule applied at tensor
scale, and it is the only reason a tensor this heterogeneous is safe to build at all.

### 9.2 Cross-market factor attribution

Extract latent factors spanning both markets — via PCA on the lattice-aligned return panel, then
rotated and *named* by their loadings against interpretable anchors (dollar, real rates, breadth,
crypto beta, AI-complex thematic). The output is the **Attribution Ribbon**:

> *Today's 2.1% NASDAQ decline: 58% global risk-appetite factor (shared with crypto, which fell
> 4.3% on the same factor), 19% rates/duration, 6% AI-complex idiosyncratic, **17% unexplained**.*

**The unexplained residual is rendered as prominently as the explained components.** A factor model
that always explains everything is a factor model with too many factors. Showing the residual is the
Section 3 doctrine applied to attribution, and it is the detail that makes the surface trustworthy
rather than merely impressive.

### 9.3 Contagion Replay

Embed the current cross-market state vector, query `prismatik-analog-store` for nearest historical
neighbours, and replay what followed — as a **distribution of paths**, never a single analog.

The analog store's disclosure contract is already mandatory and non-optional in the type: sample
size, distance metric, filters applied, survivorship warning, and leave-N-out sensitivity. Applied
to contagion, `leave_n_out_sensitivity` answers the question that kills most analog analysis:
*"is this conclusion carried entirely by March 2020?"* If dropping the three nearest neighbours
changes the median outcome sign, the surface says so, loudly.

---

## 10. Layer 6 — The Bridge

**This is the most product-differentiated section and the one I would fund first after the lattice.**

Certain instruments live in both markets simultaneously. They are the transmission channel, and
almost nobody instruments them:

| Bridge | What it transmits |
|---|---|
| Spot BTC/ETH ETFs (IBIT, FBTC, ETHA…) | Regulated-wrapper demand; creation/redemption flow is a clean daily read on institutional crypto appetite, settling on the *equity* calendar |
| Crypto-levered equities (COIN, MSTR, HOOD, MARA, RIOT…) | Equity-market speculation *about* crypto |
| CME futures basis vs perp funding | The price of leverage in each venue |
| Stablecoin net issuance | Crypto-system liquidity, no equity analogue |

Three derived measures, all of which fall out of the machinery above:

**M1 — Transmission efficiency.** How completely and how fast does a BTC move propagate into MSTR
and IBIT, controlling for market beta? Estimated from the lead-lag and impulse-response machinery.
Degradation in transmission efficiency is a liquidity warning.

**M2 — The lead-lag flip.** The direction of information flow between BTC and its equity proxies is
not constant. When BTC leads MSTR, crypto-native flow is driving. When MSTR leads BTC, equity-market
speculation is driving — a structurally different, historically more fragile market. **Detecting and
timestamping that flip is, as far as I can find, not offered by any product**, and it falls directly
out of the transfer-entropy surface in Section 6.1 with no extra estimation machinery.

**M3 — The Night Watch.** For each US session, use the lattice's `OvernightWindow` crypto path to
produce a **conformally calibrated interval** on the equity open gap. Not a point forecast — an
interval with realized coverage displayed beside it, per regime, per the existing I2 contract.

The Night Watch is the single most legible expression of the whole thesis: it can only be built
correctly by a platform that has a trustworthy calendar, a point-in-time feature store, and a
calibration layer. It is a demo you can run every weekday morning at 09:29 ET, and its honesty is
verifiable in public over time — the coverage curve either holds or it doesn't.

---

## 11. Feature Views and Their Delays

The `observation_delay` field is where the whole thing lives or dies. Non-exhaustive, and every one
of these is a look-ahead bug waiting to happen:

| Feature view | Delay | Note |
|---|---|---|
| Crypto OHLCV | ~0 | Continuous; revision risk near zero |
| Equity OHLCV | ~0 intraday | But corporate actions apply at **read time** from the ledger, never destructively |
| ETF creations/redemptions | T+1 | Published next business day |
| CFTC COT | 3 days | Tuesday data, Friday publication |
| 13F holdings | 45 days | Observable at *filing* date, not period end. This is Wave 2 DoD criterion 4 |
| Form 4 insider | 2 business days | |
| FRED macro | Series-specific + **vintage** | The initial print, not the revised value. Vintage handling is mandatory, not optional |
| On-chain metrics | Chain-specific | Reorg depth must be encoded as a delay, or you have consumed a block that was later orphaned |
| Options flow | Sub-second to T+1 | Entitlement-dependent |

The on-chain row is the one that will be missed. **Blockchain reorganizations mean recent on-chain
data is not final.** An on-chain feature with `observation_delay = 0` is a look-ahead bug that only
manifests in backtests spanning a reorg. Delay must be at least the chain's practical finality
depth. I have not seen this handled correctly anywhere.

---

## 12. Determinism Constraints — the part that will actually bite

**Confidence: H that these are real problems. M that the mitigations are sufficient.**

**D1 — Parallel float reduction is non-deterministic.** A 500 × 500 correlation matrix computed with
Rayon produces bitwise-different results run to run, because floating-point addition is not
associative and work-stealing changes the summation order. This breaks I3 silently — the numbers
look fine. Mitigation: fixed-size chunking with a deterministic merge tree, plus pairwise or
compensated (Neumaier) summation within chunks. The architecture already flags this pattern for
Monte Carlo (`P5-QM-04`, deterministic parallel reduction under Rayon); the same discipline must be
declared here, and the DST replay suite must cover a correlation matrix build.

**D2 — Iterative solvers need pinned convergence criteria.** Graphical lasso, HMM/EM, and VAR
estimation all iterate to tolerance. Tolerance, max-iterations, initialization, and tie-breaking
must be part of the pinned artifact set, or "same seed, same data" still produces different answers.

**D3 — Eigendecomposition sign and order are arbitrary.** PCA factor loadings have arbitrary sign
and arbitrary order among near-degenerate eigenvalues. Canonicalize: fix sign by the loading of
largest magnitude, order by eigenvalue with a declared tie-break. Otherwise the Attribution Ribbon
flips colors between identical runs and every user correctly concludes the product is broken.

**D4 — BLAS backends differ.** OpenBLAS and MKL do not produce bit-identical results, and neither
does the same backend across CPU feature levels (AVX2 vs AVX-512). If cross-machine reproduction is
the claim — and per `P0-DK-10` it is — the linear-algebra backend and its dispatch level belong in
the manifest's pinned environment record. This is a genuine constraint on the claim, and it is
better to state it in the manifest than to discover it in a customer's verification failure.

---

## 13. Product Surfaces — the beautiful part

Six named surfaces. The `prismatik-renderer` wgpu layer already exists in the spec for exactly this
class of visual, with mandatory Canvas/CPU fallback.

**S1 — The Market Cartogram.** Force-directed embedding of the correlation metric space, both
markets in one field. Node size = liquidity, node color = market and sector, edge thickness =
*partial* correlation (direct links only, shadows removed), edge arrows = transfer-entropy
direction. The MST renders as a bright skeleton over a dimmed full graph. Scrub time and the map
*breathes*: clusters drift apart in calm regimes and collapse toward a single knot in stress. Regime
transitions show as visible topological events. A user learns more about market structure in thirty
seconds of scrubbing than from an hour of heatmaps — and unlike a heatmap, it is legible at a glance
to someone who has never seen it before.

**S2 — The Fracture Monitor.** A single timeline of structural-break probability from BOCPD, with
the run-length posterior rendered as a heat ribbon underneath. Answers "has the relationship I am
relying on stopped being true?" — which is the question that actually loses money, and the question
no dashboard asks.

**S3 — The Night Watch.** 09:29 ET. Overnight crypto path on the left, calibrated open-gap interval
on the right, realized-coverage curve for the current regime beneath it. Yesterday's interval and
whether it contained the realized open stays on screen. The product grades itself in public, every
day, forever.

**S4 — The Attribution Ribbon.** Today's move decomposed into named factors, residual rendered in
equal weight. Stacked, animated, both markets side by side so the shared component is visually
obvious rather than asserted.

**S5 — Contagion Replay.** Current state → nearest historical neighbours → forward path distribution
as a wgpu path cloud, with the leave-N-out sensitivity control as a *slider the user can drag*.
Dragging it and watching the conclusion evaporate is the most honest interaction in the product.

**S6 — The Bridge Panel.** BTC, its ETF complex, and its equity proxies on one lattice, with the
lead-lag flip state (Section 10, M2) as a large, unmissable, timestamped indicator.

**Design doctrine for all six:** every number carries its interval, every conclusion carries its
evidence chain, every regime-conditional statement carries its regime, and every surface has a
visible "what would change my mind" affordance. Beauty here is not decoration — it is the
consequence of showing uncertainty honestly, because honest uncertainty has *shape*, and shape is
what renders well.

---

## 14. Risk Integration

The engine is not an analytics toy bolted on the side; it feeds the pre-trade catalog. The existing
`correlation_cluster` SoftWarn currently has no defined source of correlation. This engine is it.

Three additions to the §19.1 catalog:

| Check | Severity | Rule |
|---|---|---|
| `hidden_concentration` | **SoftWarn** | Portfolio appears diversified by name but loads >X% on a single cross-market latent factor. The most common real portfolio failure, and invisible to name-level concentration checks |
| `regime_transition` | **SoftWarn** | BOCPD change-point posterior above threshold; dependence estimates in flight are stale in the structural sense even though every input is fresh |
| `dependence_stale` | **HardDeny** | The dependence estimate an intent relies on was computed under a regime that has since been superseded. Invariant I6 applied to structure rather than to ticks |

`dependence_stale` is the interesting one. I6 currently governs *data* staleness. Structural
staleness is a different and sharper failure: every input can be milliseconds old while the
relationship the position depends on stopped holding last Tuesday. Fail closed.

---

## 15. Sequencing

Mapped onto the existing wave plan rather than inventing a new track.

| Stage | Content | Wave | Conf |
|---|---|---|:---:|
| 1 | `prismatik-lattice`, `OvernightWindow`, session masks, projection gates | **Wave 1.5** (needs only crypto + calendar) | H |
| 2 | `prismatik-dependence` core ladder: Pearson/Spearman + ESS + BY-FDR + tail dependence | Wave 1.5 | H |
| 3 | Hayashi–Yoshida; **The Night Watch** with real conformal intervals | Wave 2A, after equity data lands | M |
| 4 | Correlation metric space, MST, Market Cartogram v1 | Wave 2A | M |
| 5 | `prismatik-regime` BOCPD; wire regime labels into Mondrian calibration | Wave 3B, alongside `P5-QM-12` | M |
| 6 | Transfer entropy, spillover index, partial-correlation network (sidecar first) | Wave 3C | L |
| 7 | Market State Tensor, factor attribution, Attribution Ribbon | Wave 4 | L |
| 8 | Contagion Replay on the analog store | Wave 4, after P5.5 | M |
| 9 | Bridge Panel, lead-lag flip detection | Wave 4 | L |

**Stages 1 and 2 are worth building even if nothing after them is.** The lattice is required
infrastructure for any honest cross-market claim, and the FDR/ESS discipline in Stage 2 is what
makes every subsequent number defensible. They are also the cheapest items on the list.

### Definition of Done (proposed gates)

| # | Criterion | Verified by |
|---|---|---|
| 1 | No dependence estimate renders without interval, ESS, q-value, and regime label | `test_no_unqualified_dependence`, type-level |
| 2 | Cross-market correlation over a window spanning an early close is unchanged when computed via either projection, within declared tolerance | Targeted test on a known half-day |
| 3 | A 500×500 correlation matrix is bit-identical across 100 runs and across two machines | DST replay + `P0-DK-10` cross-machine harness |
| 4 | No feature in the state tensor violates `event_time + observation_delay <= as_of` | Property test, 10k cases |
| 5 | On-chain features carry a finality-depth delay; a reorg-spanning backtest is unaffected | Replay against a known reorg |
| 6 | Night Watch realized coverage is within tolerance of nominal, reported per regime | Rolling 250-session coverage report |
| 7 | Contagion Replay leave-N-out sensitivity is computed, never optional | Type-level, inherited from `AnalogResult` |
| 8 | PCA loadings are sign- and order-canonical across reruns | Property test |
| 9 | Every Cartogram edge resolves to an evidence chain | `test_no_orphan_conclusions` extension |

---

## 16. What I Would Cut, and the Honest Risks

**Cut first, in this order:** transfer entropy (Stage 6) — it is data-hungry, sensitive to embedding
and binning choices, and its estimates on daily financial data are frequently indistinguishable from
noise. Then the Diebold–Yilmaz spillover network — beautiful output, but rolling VAR on a large
cross-section is fragile and the identification choices (Cholesky ordering vs generalized FEVD) are
contestable in a way that is hard to disclose honestly. Then partial-correlation networks —
graphical lasso regularization strength is a free parameter that changes the conclusion, which sits
uncomfortably against this platform's disclosure standards.

**Risks I would not paper over:**

| Risk | Reality |
|---|---|
| Most of Section 6's lower ladder is genuinely research-grade | Prototype in the sidecar. Do not give an estimator a Rust crate until it has survived out-of-sample validation on real data |
| Cross-market correlation is itself non-stationary at the *meta* level | The relationship between crypto and equities in 2021 (retail liquidity) is not the one in 2026. Long backtests of this engine are structurally suspect and the surfaces should say so |
| The Night Watch is a forecast product | It will be wrong regularly. That is fine and expected — but only if the coverage curve is displayed as prominently as the forecast. Ship it with the curve or do not ship it |
| Factor naming is interpretation, not measurement | PCA gives orthogonal directions, not economic factors. The names are a human overlay and must be labeled as such in the UI, not presented as discovered truth |
| Effort | This is plausibly 150–250 engineer-days across the full nine stages. Stages 1–2 alone are maybe 20–30. Treat the rest as optional |

---

## 17. The One-Paragraph Version

Build the clock first. `prismatik-lattice` turns the pinned calendar from invisible correctness
plumbing into the foundation of a product surface, because crypto/equity convergence is fundamentally
a clock problem that everyone solves wrong. On top of it, put a dependence estimator whose output
type cannot be constructed without an interval, an effective sample size, an FDR-adjusted q-value,
and a regime label — making the five standard industry errors unrepresentable rather than merely
discouraged. Then render the result as a living map of one market rather than two tables of two
markets. The differentiator is not that PRISMATIK computes correlations better. It is that PRISMATIK
is the only architecture on the table that can state a cross-market relationship, prove what it knew
when it stated it, show how often it has been right, and re-derive the same number byte for byte
three years later.

---

*Proposal only. No crates created, no wave commitments made, no work started.*
