//! Offline research-platform experience fixtures (backtest, portfolio, marketplace,
//! strategy, Monte Carlo, calibration, analog, journal, plugin host, RBAC).
//!
//! Deterministic demo payloads for desktop EX routes — no network, no live trading.

use serde::Serialize;

const AS_OF: &str = "2026-07-25T20:00:00Z";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestMetric {
    label: String,
    value: String,
    evidence: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestSummary {
    strategy_id: String,
    window_start: String,
    window_end: String,
    fill_model: String,
    total_return_pct: f64,
    max_drawdown_pct: f64,
    trade_count: u32,
    annualized_sharpe: f64,
    manifest_hash: String,
    manifest_digest: String,
    lineage_events: u32,
    provider: String,
    retrieved_at: String,
    metrics: Vec<BacktestMetric>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioPosition {
    symbol: String,
    side: String,
    quantity: i64,
    avg_cost: f64,
    mark: f64,
    unrealized_pnl: f64,
    avg_cost_source: String,
    multiplier: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioSnapshot {
    currency: String,
    total_equity: f64,
    cash: f64,
    unrealized_pnl: f64,
    realized_pnl: f64,
    provider: String,
    retrieved_at: String,
    positions: Vec<PortfolioPosition>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceListingRow {
    plugin_id: String,
    version: String,
    publisher: String,
    capabilities: Vec<String>,
    license_class: String,
    status: String,
    installable: bool,
    install_gate: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceCatalog {
    provider: String,
    retrieved_at: String,
    listings: Vec<MarketplaceListingRow>,
}

fn license_class_label(c: prismatik_oss_registry::LicenseClass) -> String {
    match c {
        prismatik_oss_registry::LicenseClass::FirstParty => "first_party".into(),
        prismatik_oss_registry::LicenseClass::PermissiveCommercial => "permissive_commercial".into(),
        prismatik_oss_registry::LicenseClass::NonCommercial => "non_commercial".into(),
        prismatik_oss_registry::LicenseClass::Copyleft => "copyleft".into(),
        prismatik_oss_registry::LicenseClass::Unknown => "unknown".into(),
    }
}

fn marketplace_status_label(s: prismatik_oss_registry::MarketplaceStatus) -> String {
    match s {
        prismatik_oss_registry::MarketplaceStatus::Submitted => "submitted".into(),
        prismatik_oss_registry::MarketplaceStatus::InReview => "in_review".into(),
        prismatik_oss_registry::MarketplaceStatus::Approved => "approved".into(),
        prismatik_oss_registry::MarketplaceStatus::Revoked => "revoked".into(),
    }
}

fn listing_row(listing: &prismatik_oss_registry::MarketplaceListing) -> MarketplaceListingRow {
    let (installable, install_gate) = match listing.assert_installable() {
        Ok(()) => (true, "ok".into()),
        Err(e) => (false, e.to_string()),
    };
    MarketplaceListingRow {
        plugin_id: listing.plugin_id.clone(),
        version: listing.version.clone(),
        publisher: listing.publisher.display_name.clone(),
        capabilities: listing.capabilities.clone(),
        license_class: license_class_label(listing.license_class),
        status: marketplace_status_label(listing.status),
        installable,
        install_gate,
    }
}

#[tauri::command]
pub fn get_backtest_summary(strategy_id: Option<String>) -> BacktestSummary {
    use prismatik_backtest::{
        purged_embargoed_walk_forward, BacktestConfig, BacktestEngine, BacktestWindow,
        ExecutionAssumptions, FillModel, compute_metrics,
    };
    use time::{Duration, OffsetDateTime};

    let strategy_id = strategy_id.unwrap_or_else(|| "sma-cross-v1".to_string());
    let start = OffsetDateTime::UNIX_EPOCH + Duration::days(19_723); // ~2024-01-02
    let end = start + Duration::days(545); // ~2025-06-30
    let config = BacktestConfig {
        strategy_id: strategy_id.clone(),
        window: BacktestWindow { start, end },
        starting_capital_micros: 100_000_000_000,
        execution: ExecutionAssumptions {
            fill_model: FillModel::NextOpen,
            slippage_bps: 5,
            commission_per_share_micros: 5_000,
            assignment_enabled: false,
        },
        bar_interval_seconds: 86_400,
    };

    // Deterministic demo equity (micros) for metrics — not live trading PnL.
    let equity_micros: Vec<u64> = [
        100_000, 102_000, 101_800, 103_500, 105_000, 104_800, 106_200, 108_000, 107_500,
        109_000, 110_500, 109_800, 112_000,
    ]
    .into_iter()
    .map(|x| x * 1_000_000)
    .collect();
    let trade_count = 47_u64;
    let metrics = compute_metrics(&equity_micros, trade_count);
    let folds = purged_embargoed_walk_forward(260, 120, 20, 5, 10)
        .map(|f| f.len())
        .unwrap_or(0);
    let embargo_bars = 10_u64;

    let (manifest_hash, manifest_digest, lineage_events, provider) =
        match BacktestEngine::new(config) {
            Ok(engine) => match engine.run_stub_signed(start) {
                Ok(run) => {
                    use prismatik_events::openlineage::{OpenLineageEmitter, RecordingOpenLineageSink, RunEvent};
                    use prismatik_manifest::unsigned_canonical_digest;

                    let digest = unsigned_canonical_digest(&run.manifest)
                        .map(|h| h.to_string())
                        .unwrap_or_else(|_| "blake3:unsigned-manifest-error".into());
                    let run_id = run.manifest.run_id.clone();
                    let sink = RecordingOpenLineageSink::new();
                    let producer = "https://github.com/mythos/prismatik";
                    let _ = sink.emit(&RunEvent::new(
                        "START",
                        run_id.clone(),
                        "prismatik.backtest.stub",
                        producer,
                    ));
                    let _ = sink.emit(&RunEvent::new(
                        "COMPLETE",
                        run_id.clone(),
                        "prismatik.backtest.stub",
                        producer,
                    ));
                    (
                        format!("manifest:{}", run.manifest.manifest_id),
                        digest,
                        sink.len() as u32,
                        "prismatik-backtest::run_stub_signed+openlineage".into(),
                    )
                }
                Err(_) => (
                    "sha256:demo-backtest-manifest-stub".into(),
                    "blake3:demo-backtest-manifest-stub".into(),
                    0,
                    "backtest-metrics-floor".into(),
                ),
            },
            Err(_) => (
                "sha256:demo-backtest-manifest-stub".into(),
                "blake3:demo-backtest-manifest-stub".into(),
                0,
                "backtest-metrics-floor".into(),
            ),
        };

    BacktestSummary {
        strategy_id,
        window_start: "2024-01-02T00:00:00Z".into(),
        window_end: "2025-06-30T00:00:00Z".into(),
        fill_model: "next_open".into(),
        total_return_pct: metrics.total_return * 100.0,
        max_drawdown_pct: -metrics.max_drawdown * 100.0,
        trade_count: trade_count as u32,
        annualized_sharpe: metrics.sharpe.unwrap_or(0.0),
        manifest_hash,
        manifest_digest,
        lineage_events,
        provider,
        retrieved_at: AS_OF.into(),
        metrics: vec![
            BacktestMetric {
                label: "Purged folds".into(),
                value: folds.to_string(),
                evidence: "purged_embargoed_walk_forward".into(),
            },
            BacktestMetric {
                label: "Embargo bars".into(),
                value: embargo_bars.to_string(),
                evidence: "purged + embargoed walk-forward floor".into(),
            },
            BacktestMetric {
                label: "Deflated Sharpe".into(),
                value: metrics
                    .deflated_sharpe
                    .map(|v| format!("{v:.3}"))
                    .unwrap_or_else(|| "n/a".into()),
                evidence: "deflated_sharpe_ratio".into(),
            },
            BacktestMetric {
                label: "Signed manifest".into(),
                value: "verified".into(),
                evidence: "prismatik-backtest::manifest".into(),
            },
        ],
    }
}

#[tauri::command]
pub fn get_portfolio_snapshot() -> PortfolioSnapshot {
    use prismatik_domain::CurrencyCode;
    use prismatik_identity::AssetId;
    use prismatik_portfolio::{
        AvgCostSource, PortfolioSnapshot as CorePortfolio, Position, PositionSide,
    };

    let usd = CurrencyCode::usd();
    let mut core = CorePortfolio::empty();
    core.cash_micros = 42_100_000_000; // $42,100.00

    let demo = [
        (
            "AAPL",
            PositionSide::Long,
            120_i64,
            178_420_000_i64,
            214_950_000_i64,
            AvgCostSource::Broker,
            0_i64,
        ),
        (
            "SPY",
            PositionSide::Long,
            50,
            512_100_000,
            548_220_000,
            AvgCostSource::Broker,
            0,
        ),
        (
            "BTC-USD",
            PositionSide::Long,
            1,
            58_200_000_000,
            67_842_000_000,
            AvgCostSource::Wallet,
            0,
        ),
    ];

    for (sym, side, qty, cost, mark, src, realized) in demo {
        core.upsert(Position {
            asset_id: AssetId::from_canonical_bytes(sym.as_bytes()),
            currency: usd,
            side,
            quantity: qty,
            avg_cost_micros: cost,
            market_price_micros: mark,
            multiplier: 1,
            avg_cost_source: src,
            realized_pnl_micros: realized,
        });
    }

    let unrealized = core.total_unrealized_pnl_micros();
    let realized = core.total_realized_pnl_micros();
    let equity_micros = core
        .positions
        .values()
        .map(Position::market_value_micros)
        .fold(core.cash_micros, i64::saturating_add);

    let mut positions: Vec<PortfolioPosition> = core
        .positions
        .values()
        .map(|p| {
            let symbol = demo
                .iter()
                .find(|(s, ..)| AssetId::from_canonical_bytes(s.as_bytes()) == p.asset_id)
                .map(|(s, ..)| (*s).to_string())
                .unwrap_or_else(|| format!("{:?}", p.asset_id));
            PortfolioPosition {
                symbol,
                side: match p.side {
                    PositionSide::Long => "long".into(),
                    PositionSide::Short => "short".into(),
                },
                quantity: p.quantity,
                avg_cost: p.avg_cost_micros as f64 / 1_000_000.0,
                mark: p.market_price_micros as f64 / 1_000_000.0,
                unrealized_pnl: p.unrealized_pnl_micros() as f64 / 1_000_000.0,
                avg_cost_source: match p.avg_cost_source {
                    AvgCostSource::Broker => "broker".into(),
                    AvgCostSource::Wallet => "wallet".into(),
                },
                multiplier: p.multiplier,
            }
        })
        .collect();
    positions.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    PortfolioSnapshot {
        currency: "USD".into(),
        total_equity: equity_micros as f64 / 1_000_000.0,
        cash: core.cash_micros as f64 / 1_000_000.0,
        unrealized_pnl: unrealized as f64 / 1_000_000.0,
        realized_pnl: realized as f64 / 1_000_000.0,
        provider: "prismatik-portfolio::PortfolioSnapshot".into(),
        retrieved_at: AS_OF.into(),
        positions,
    }
}

#[tauri::command]
pub fn get_marketplace_listings() -> MarketplaceCatalog {
    use prismatik_oss_registry::{
        LicenseClass, MarketplaceListing, MarketplaceStatus, PublisherIdentity,
    };

    let demo = vec![
        MarketplaceListing {
            plugin_id: "prismatik-indicators-ext".into(),
            version: "0.3.1".into(),
            publisher: PublisherIdentity {
                publisher_id: "mythos".into(),
                display_name: "Mythos Systems".into(),
                signing_pubkey_hex: "01".into(),
            },
            capabilities: vec![
                "indicator.read_bars".into(),
                "indicator.emit_series".into(),
            ],
            license_class: LicenseClass::PermissiveCommercial,
            status: MarketplaceStatus::Approved,
        },
        MarketplaceListing {
            plugin_id: "flow-quality-scorer".into(),
            version: "1.0.0".into(),
            publisher: PublisherIdentity {
                publisher_id: "community-labs".into(),
                display_name: "Community Labs".into(),
                signing_pubkey_hex: "02".into(),
            },
            capabilities: vec!["flow.read_alerts".into(), "flow.score".into()],
            license_class: LicenseClass::PermissiveCommercial,
            status: MarketplaceStatus::InReview,
        },
        MarketplaceListing {
            plugin_id: "legacy-alpha-bridge".into(),
            version: "0.9.4".into(),
            publisher: PublisherIdentity {
                publisher_id: "third-party".into(),
                display_name: "Third Party".into(),
                signing_pubkey_hex: "03".into(),
            },
            capabilities: vec!["network.http".into()],
            license_class: LicenseClass::PermissiveCommercial,
            status: MarketplaceStatus::Revoked,
        },
        MarketplaceListing {
            plugin_id: "noncommercial-research-pack".into(),
            version: "0.1.0".into(),
            publisher: PublisherIdentity {
                publisher_id: "research".into(),
                display_name: "Research Orgs".into(),
                signing_pubkey_hex: "04".into(),
            },
            capabilities: vec!["research.read".into()],
            license_class: LicenseClass::NonCommercial,
            status: MarketplaceStatus::Approved,
        },
    ];

    MarketplaceCatalog {
        provider: "prismatik-oss-registry::MarketplaceListing".into(),
        retrieved_at: AS_OF.into(),
        listings: demo.iter().map(listing_row).collect(),
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyBlock {
    id: String,
    kind: String,
    params: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyIrPreview {
    strategy_id: String,
    schema_version: String,
    ir_digest: String,
    dsl: String,
    blocks: Vec<StrategyBlock>,
    rule_kind: String,
    capabilities: StrategyCapabilitiesView,
    provider: String,
    retrieved_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StrategyCapabilitiesView {
    network: bool,
    filesystem: bool,
    ai: bool,
    max_compute_per_bar_ms: u32,
}

fn strategy_ir_digest(ir: &prismatik_strategy::StrategyIr) -> String {
    use prismatik_determinism::ContentHash;
    use prismatik_manifest::canonical_json;

    serde_json::to_value(ir)
        .ok()
        .map(|v| ContentHash::from_bytes(canonical_json(&v).as_bytes()).to_string())
        .unwrap_or_else(|| "blake3:strategy-ir-serialize-error".into())
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonteCarloPaths {
    seed: u64,
    source: String,
    terminal_mean: f64,
    max_drawdown_proxy: f64,
    ruin_probability: f64,
    provider: String,
    retrieved_at: String,
    paths: Vec<Vec<f64>>,
}

#[tauri::command]
pub fn get_strategy_ir_preview(strategy_id: Option<String>) -> StrategyIrPreview {
    let strategy_id = strategy_id.unwrap_or_else(|| "sma-cross-v1".to_string());
    let dsl = format!(
        "strategy {strategy_id} {{\n  sma(close, 10) > ema(close, 20) and rsi(close, 14) < 70\n}}"
    );
    match prismatik_strategy::parse_typecheck_to_ir_stub(&dsl) {
        Ok(ir) => {
            let mut blocks: Vec<StrategyBlock> = ir
                .indicators
                .iter()
                .enumerate()
                .map(|(i, ind)| StrategyBlock {
                    id: if ind.alias.is_empty() {
                        format!("ind-{i}")
                    } else {
                        ind.alias.clone()
                    },
                    kind: ind.kind.to_ascii_uppercase(),
                    params: vec![
                        "close".into(),
                        ind.alias.rsplit('_').next().unwrap_or("0").into(),
                    ],
                })
                .collect();
            if let Some(cond) = ir.rules.get("condition") {
                blocks.push(StrategyBlock {
                    id: "entry-rule".into(),
                    kind: "Rule".into(),
                    params: vec![cond.to_string()],
                });
            }
            let rule_kind = ir
                .rules
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("strategy")
                .to_string();
            StrategyIrPreview {
                strategy_id: ir.strategy_id.clone(),
                schema_version: ir.schema_version.clone(),
                ir_digest: strategy_ir_digest(&ir),
                dsl,
                blocks,
                rule_kind,
                capabilities: StrategyCapabilitiesView {
                    network: ir.capabilities.can_access_network,
                    filesystem: ir.capabilities.can_access_filesystem,
                    ai: ir.capabilities.can_invoke_ai,
                    max_compute_per_bar_ms: ir.capabilities.max_compute_per_bar_ms,
                },
                provider: "prismatik-strategy::parse_typecheck_to_ir_stub".into(),
                retrieved_at: AS_OF.into(),
            }
        }
        Err(_) => StrategyIrPreview {
            strategy_id,
            schema_version: prismatik_strategy::STRATEGY_IR_SCHEMA_VERSION.into(),
            ir_digest: "blake3:strategy-ir-fallback".into(),
            dsl,
            blocks: vec![
                StrategyBlock {
                    id: "sma-10".into(),
                    kind: "SMA".into(),
                    params: vec!["close".into(), "10".into()],
                },
                StrategyBlock {
                    id: "ema-20".into(),
                    kind: "EMA".into(),
                    params: vec!["close".into(), "20".into()],
                },
            ],
            rule_kind: "strategy".into(),
            capabilities: StrategyCapabilitiesView {
                network: false,
                filesystem: false,
                ai: false,
                max_compute_per_bar_ms: 50,
            },
            provider: "strategy-ir-fallback-floor".into(),
            retrieved_at: AS_OF.into(),
        },
    }
}

#[tauri::command]
pub fn get_monte_carlo_paths() -> MonteCarloPaths {
    use prismatik_simulation::{
        run_path_series, run_paths, summarize_terminals, MonteCarloConfig, SimulationSource,
    };

    let seed = 42_001_u64;
    let cfg = MonteCarloConfig {
        paths: 5,
        horizon: 5,
        seed,
    };
    let source = SimulationSource::StochasticVol;
    let paths = run_path_series(&source, &cfg);
    let terminals = run_paths(&source, &cfg);
    let summary = summarize_terminals(&terminals, 0.5);
    MonteCarloPaths {
        seed,
        source: "stochastic_vol".into(),
        terminal_mean: summary.terminal_mean,
        max_drawdown_proxy: summary.max_drawdown_proxy,
        ruin_probability: summary.ruin_probability,
        provider: "prismatik-simulation::run_path_series+summarize".into(),
        retrieved_at: AS_OF.into(),
        paths,
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderTicketDraft {
    symbol: String,
    side: String,
    quantity: u32,
    order_type: String,
    limit_price: Option<f64>,
    time_in_force: String,
    idempotency_key: String,
    signed_quantity: i64,
    risk_ok: bool,
    risk_failure: Option<String>,
    provider: String,
    retrieved_at: String,
}

#[tauri::command]
pub fn preview_order_ticket(
    symbol: Option<String>,
    side: Option<String>,
    quantity: Option<u32>,
) -> OrderTicketDraft {
    use prismatik_execution::{
        draft_paper_order_ticket, paper_max_position_policy, OrderSide, PAPER_DEMO_LIMIT_MICROS,
        RiskGateOutcome,
    };
    use prismatik_risk::{CheckId, RiskLimits, RiskPolicy};

    let symbol = symbol.unwrap_or_else(|| "AAPL".into());
    let side_raw = side.unwrap_or_else(|| "buy".into());
    let quantity = quantity.unwrap_or(10);

    let parsed_side = OrderSide::parse(&side_raw).unwrap_or(OrderSide::Buy);
    // Demo ceiling tighter than production paper default so EX can show a deny path.
    let policy = RiskPolicy::with_checks([CheckId::MaxPosition]).with_limits(RiskLimits {
        max_position_micros: 5_000_000_000, // $5k
        ..RiskLimits::default()
    });
    let ticket = draft_paper_order_ticket(
        symbol.clone(),
        parsed_side,
        quantity.max(1),
        Some(PAPER_DEMO_LIMIT_MICROS),
        &policy,
        "max_position",
    )
    .or_else(|_| {
        draft_paper_order_ticket(
            "AAPL",
            OrderSide::Buy,
            10,
            Some(PAPER_DEMO_LIMIT_MICROS),
            &paper_max_position_policy(),
            "max_position",
        )
    })
    .expect("paper ticket floor");

    let (risk_ok, risk_failure) = match &ticket.risk_outcome {
        RiskGateOutcome::Approved => (true, None),
        RiskGateOutcome::Denied { check } => (false, Some(format!("risk check failed: {check}"))),
    };

    OrderTicketDraft {
        symbol: ticket.intent.instrument_id.clone(),
        side: ticket.intent.side.as_str().into(),
        quantity: ticket.intent.quantity,
        order_type: ticket.intent.order_type.as_str().into(),
        limit_price: ticket
            .intent
            .limit_price_micros
            .map(|m| m as f64 / 1_000_000.0),
        time_in_force: ticket.intent.time_in_force.as_str().into(),
        idempotency_key: ticket.intent.idempotency_key.to_hex(),
        signed_quantity: ticket.intent.signed_quantity(),
        risk_ok,
        risk_failure,
        provider: "prismatik-execution::draft_paper_order_ticket".into(),
        retrieved_at: AS_OF.into(),
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationPoint {
    x: f64,
    y: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationRibbon {
    nominal: f64,
    realized_coverage: f64,
    points: Vec<CalibrationPoint>,
    drift_detector: String,
    drift_magnitude: f64,
    drift_action: String,
    backends_ok: u32,
    provider: String,
    retrieved_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCard {
    model_id: String,
    family: String,
    family_role: String,
    onboarding_status: String,
    license_spdx: String,
    drift_status: String,
    drift_action: String,
    ladder_rung: String,
    promotion_ok: bool,
    blind_spots: Vec<String>,
    provider: String,
    retrieved_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalogHit {
    id: String,
    score: f64,
    disclosures: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalogHits {
    query: String,
    hits: Vec<AnalogHit>,
    provider: String,
    retrieved_at: String,
}

fn drift_action_label(action: &prismatik_calibration::DriftAction) -> String {
    match action {
        prismatik_calibration::DriftAction::None => "none".into(),
        prismatik_calibration::DriftAction::Annotate { notice } => format!("annotate:{notice}"),
        prismatik_calibration::DriftAction::Widen { factor } => format!("widen:{factor:.2}"),
        prismatik_calibration::DriftAction::Suppress { fallback } => {
            format!("suppress:{fallback}")
        }
        prismatik_calibration::DriftAction::ProposeDemotion { magnitude } => {
            format!("propose_demotion:{magnitude:.2}")
        }
    }
}

fn drift_status_from_action(action: &prismatik_calibration::DriftAction) -> &'static str {
    match action {
        prismatik_calibration::DriftAction::None => "stable",
        prismatik_calibration::DriftAction::Annotate { .. } => "watch",
        prismatik_calibration::DriftAction::Widen { .. } => "watch",
        prismatik_calibration::DriftAction::Suppress { .. }
        | prismatik_calibration::DriftAction::ProposeDemotion { .. } => "suppressed",
    }
}

#[tauri::command]
pub fn get_calibration_ribbon() -> CalibrationRibbon {
    use prismatik_calibration::{
        evaluate, ArrowIpcBlob, CalibrationSidecarClient, DriftDetector, DriftThresholds,
        PredictionBatch, RealizationBatch, SidecarBackend, SidecarCalibrateRequest,
    };

    let client = CalibrationSidecarClient::loopback();
    let backends = [
        SidecarBackend::AdaptiveConformal,
        SidecarBackend::EnsembleBatch,
        SidecarBackend::Mondrian,
    ];
    let mut backends_ok = 0u32;
    let mut ribbon_points: Vec<CalibrationPoint> = Vec::new();
    let mut nominal = 0.90;
    let mut realized_coverage = 0.872;

    for backend in backends {
        let request = SidecarCalibrateRequest {
            backend,
            predictions_ipc: ArrowIpcBlob::empty(),
            realizations_ipc: ArrowIpcBlob::empty(),
            predictions: Some(PredictionBatch {
                points: vec![1.0, 2.0, 3.0, 4.0, 5.0],
            }),
            realizations: Some(RealizationBatch {
                values: vec![1.05, 1.9, 3.1, 3.8, 5.2],
            }),
        };
        if let Ok(resp) = client.invoke(&request) {
            backends_ok += 1;
            if ribbon_points.is_empty() {
                ribbon_points = resp
                    .record
                    .coverage_curve
                    .iter()
                    .map(|(n, r)| CalibrationPoint { x: n.0, y: r.0 })
                    .collect();
                if let Some(p) = ribbon_points.last() {
                    nominal = p.x;
                    realized_coverage = p.y;
                }
            }
        }
    }

    if ribbon_points.is_empty() {
        ribbon_points = vec![
            CalibrationPoint { x: 0.50, y: 0.512 },
            CalibrationPoint { x: 0.90, y: 0.872 },
        ];
    }

    let gap = (nominal - realized_coverage).abs();
    let assessment = evaluate(DriftDetector::Calibration, gap, &DriftThresholds::default());

    CalibrationRibbon {
        nominal,
        realized_coverage,
        points: ribbon_points,
        drift_detector: format!("{:?}", assessment.detector).to_ascii_lowercase(),
        drift_magnitude: assessment.magnitude,
        drift_action: drift_action_label(&assessment.action),
        backends_ok,
        provider: "prismatik-calibration::loopback+drift".into(),
        retrieved_at: AS_OF.into(),
    }
}

#[tauri::command]
pub fn get_model_card(model_id: Option<String>) -> ModelCard {
    use prismatik_calibration::{
        can_promote, evaluate, DriftDetector, DriftThresholds, LadderEntry, LadderRung,
        LadderScores,
    };
    use prismatik_tsfm::{FamilyOnboardingStatus, OnboardedTsfmFamily};

    let model_id = model_id.unwrap_or_else(|| "probabilistic-baseline-demo".to_string());
    let family = OnboardedTsfmFamily::ProbabilisticBaseline;
    let onboarding = match family.status() {
        FamilyOnboardingStatus::Onboarded => "onboarded",
        FamilyOnboardingStatus::RejectedCommercial => "rejected_commercial",
    };

    // Demo magnitudes: calibration annotate-band, embedding widen-band → watch.
    let cal = evaluate(DriftDetector::Calibration, 0.12, &DriftThresholds::default());
    let emb = evaluate(DriftDetector::Embedding, 0.30, &DriftThresholds::default());
    let worst = match (&cal.action, &emb.action) {
        (prismatik_calibration::DriftAction::ProposeDemotion { .. }, _)
        | (_, prismatik_calibration::DriftAction::ProposeDemotion { .. }) => &emb.action,
        (prismatik_calibration::DriftAction::Suppress { .. }, _)
        | (_, prismatik_calibration::DriftAction::Suppress { .. }) => {
            if matches!(
                emb.action,
                prismatik_calibration::DriftAction::Suppress { .. }
            ) {
                &emb.action
            } else {
                &cal.action
            }
        }
        (_, prismatik_calibration::DriftAction::Widen { .. }) => &emb.action,
        (prismatik_calibration::DriftAction::Widen { .. }, _) => &cal.action,
        (_, prismatik_calibration::DriftAction::Annotate { .. }) => &emb.action,
        _ => &cal.action,
    };

    let lower = [
        LadderEntry {
            rung: LadderRung::R0,
            model_id: "persist".into(),
            scores: LadderScores {
                discrimination: 0.40,
                calibration: 0.45,
            },
        },
        LadderEntry {
            rung: LadderRung::R1,
            model_id: "garch".into(),
            scores: LadderScores {
                discrimination: 0.52,
                calibration: 0.55,
            },
        },
        LadderEntry {
            rung: LadderRung::R2,
            model_id: "gbm-baseline".into(),
            scores: LadderScores {
                discrimination: 0.62,
                calibration: 0.70,
            },
        },
        LadderEntry {
            rung: LadderRung::R3,
            model_id: "task-nn".into(),
            scores: LadderScores {
                discrimination: 0.68,
                calibration: 0.72,
            },
        },
    ];
    let candidate = LadderEntry {
        rung: LadderRung::R4,
        model_id: model_id.clone(),
        scores: LadderScores {
            discrimination: 0.71,
            calibration: 0.74,
        },
    };
    let promotion_ok = can_promote(&candidate, &lower).is_ok();

    ModelCard {
        model_id,
        family: "probabilistic_forecast".into(),
        family_role: family.as_str().into(),
        onboarding_status: onboarding.into(),
        license_spdx: family.license_spdx().into(),
        drift_status: drift_status_from_action(worst).into(),
        drift_action: drift_action_label(worst),
        ladder_rung: format!("{:?}", candidate.rung).to_ascii_lowercase(),
        promotion_ok,
        blind_spots: vec![
            "Regime shifts after macro shocks (2020-style)".into(),
            "Thin liquidity in far-dated options".into(),
            "Corporate action gaps in symbology resolver".into(),
            format!("embedding drift → {}", drift_action_label(&emb.action)),
        ],
        provider: "prismatik-tsfm::OnboardedTsfmFamily+drift+ladder".into(),
        retrieved_at: AS_OF.into(),
    }
}

#[tauri::command]
pub fn get_analog_hits(query: Option<String>) -> AnalogHits {
    use prismatik_analog_store::AnalogStore;
    use serde_json::json;

    let query = query.unwrap_or_else(|| "AAPL earnings gap + vol crush".to_string());
    let mut store = AnalogStore::new();
    let _ = store.upsert(
        "analog-2019-q1",
        "AAPL earnings gap with subsequent IV crush after print",
        json!({"year": 2019}),
    );
    let _ = store.upsert(
        "analog-2022-oct",
        "Equity gap higher into earnings; vol crushed into expiry",
        json!({"year": 2022}),
    );
    let _ = store.upsert(
        "analog-2016-jan",
        "Macro risk-off gap with thin options liquidity",
        json!({"year": 2016}),
    );

    let result = store.search(&query, 3);
    let mut shared = vec![
        format!("sample_size={}", result.sample_size),
        format!(
            "leave_n_out n={} baseline={:.3}",
            result.leave_n_out_sensitivity.n, result.leave_n_out_sensitivity.baseline_top_score
        ),
    ];
    if let Some(delta) = result.leave_n_out_sensitivity.score_delta {
        shared.push(format!("leave_n_out score_delta={delta:.3}"));
    }
    if let Some(w) = &result.survivorship_warning {
        shared.push(w.message.clone());
    }
    for f in &result.filters_applied {
        shared.push(format!("filter={f:?}"));
    }

    let hits = result
        .neighbours
        .into_iter()
        .map(|n| {
            let mut disclosures = shared.clone();
            disclosures.push(format!("distance={:.4}", n.distance));
            AnalogHit {
                id: n.id,
                score: f64::from(n.score),
                disclosures,
            }
        })
        .collect();

    AnalogHits {
        query,
        hits,
        provider: "prismatik-analog-store::search".into(),
        retrieved_at: AS_OF.into(),
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalEntry {
    id: String,
    title: String,
    tags: Vec<String>,
    created_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalEntries {
    entries: Vec<JournalEntry>,
    provider: String,
    retrieved_at: String,
}

#[tauri::command]
pub fn get_journal_entries() -> JournalEntries {
    use prismatik_options::{turbo_fixture_corpus, turbo_fixture_summary};

    let corpus = turbo_fixture_corpus();
    let summary = turbo_fixture_summary();

    let mut entries: Vec<JournalEntry> = corpus
        .labels
        .iter()
        .map(|l| JournalEntry {
            id: l.print_id.clone(),
            title: if l.notes.is_empty() {
                format!("flow label {}", l.print_id)
            } else {
                l.notes.clone()
            },
            tags: vec![
                format!("{:?}", l.expected).to_ascii_lowercase(),
                l.labeled_by.clone(),
            ],
            created_at: AS_OF.into(),
        })
        .collect();
    entries.push(JournalEntry {
        id: "corpus-score".into(),
        title: format!(
            "hand-label score {}/{} (author target {})",
            summary.matched, summary.compared, summary.author_target
        ),
        tags: vec![
            "hand_label".into(),
            format!("meets_author_target={}", summary.meets_author_target),
            format!("directional={}", summary.class_counts.directional),
        ],
        created_at: AS_OF.into(),
    });

    JournalEntries {
        entries,
        provider: "prismatik-options::turbo_fixture_summary".into(),
        retrieved_at: AS_OF.into(),
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHostStatus {
    hardened_ok: bool,
    strategy: String,
    consume_fuel: bool,
    wasm_threads: bool,
    cosign_linked: bool,
    provider: String,
    retrieved_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallPreview {
    plugin_id: String,
    version: String,
    accepted: bool,
    gate: String,
    granted_capabilities: Vec<String>,
    provider: String,
    retrieved_at: String,
}

fn map_capability_token(raw: &str) -> Option<prismatik_plugin_host::Capability> {
    use prismatik_plugin_host::Capability;
    let t = raw.trim().to_ascii_lowercase();
    if t.contains("network") || t.starts_with("http") {
        Some(Capability::Network)
    } else if t.contains("filesystem") || t.contains("fs.") {
        Some(Capability::Filesystem)
    } else if t.contains("host_ai") || t.contains("host.ai") {
        Some(Capability::HostAi)
    } else if t.contains("entropy") || t.contains("random") {
        Some(Capability::Entropy)
    } else if t.contains("clock") || t.contains("indicator") || t.contains("flow") || t.contains("research") {
        Some(Capability::Clock)
    } else {
        None
    }
}

#[tauri::command]
pub fn get_plugin_host_status() -> PluginHostStatus {
    use prismatik_plugin_host::{assert_hardened, try_verify_at_load, CosignVerifyRequest, HardenedEngineConfig};

    let cfg = HardenedEngineConfig::mandatory();
    let hardened_ok = assert_hardened(&cfg).is_ok();
    let cosign_linked = try_verify_at_load(&CosignVerifyRequest {
        image_ref: "demo/plugin:0.1.0".into(),
        digest: "sha256:00".into(),
    })
    .is_ok();

    PluginHostStatus {
        hardened_ok,
        strategy: cfg.strategy,
        consume_fuel: cfg.consume_fuel,
        wasm_threads: cfg.wasm_threads,
        cosign_linked,
        provider: "prismatik-plugin-host::HardenedEngineConfig".into(),
        retrieved_at: AS_OF.into(),
    }
}

#[tauri::command]
pub fn preview_plugin_install(
    plugin_id: String,
    version: String,
    publisher_id: Option<String>,
    capabilities: Vec<String>,
) -> PluginInstallPreview {
    use prismatik_plugin_host::{install_plugin, CapabilitySet, PluginManifest};

    let mut requested = CapabilitySet::empty();
    for cap in &capabilities {
        if let Some(c) = map_capability_token(cap) {
            requested = requested.grant(c);
        }
    }
    // Operator grant for demo: clock + entropy only (network denied).
    let granted = CapabilitySet::empty()
        .grant(prismatik_plugin_host::Capability::Clock)
        .grant(prismatik_plugin_host::Capability::Entropy);

    let manifest = PluginManifest {
        id: plugin_id.clone(),
        version: version.clone(),
        capabilities: requested.clone(),
        ed25519_sig_hex: "deadbeef".into(),
        publisher_id: publisher_id.unwrap_or_else(|| "demo-publisher".into()),
    };

    let (accepted, gate, granted_capabilities) = match install_plugin(&manifest, &granted) {
        Ok(rec) => (
            true,
            "ok".into(),
            rec.capabilities.iter().map(|c| format!("{c:?}").to_ascii_lowercase()).collect(),
        ),
        Err(e) => (
            false,
            e.to_string(),
            requested
                .iter()
                .map(|c| format!("{c:?}").to_ascii_lowercase())
                .collect(),
        ),
    };

    PluginInstallPreview {
        plugin_id,
        version,
        accepted,
        gate,
        granted_capabilities,
        provider: "prismatik-plugin-host::install_plugin".into(),
        retrieved_at: AS_OF.into(),
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RbacRoleProbe {
    role: String,
    read_workspace: bool,
    write_research: bool,
    manage_paper: bool,
    enable_live: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceAuthFloor {
    session_max_age_secs: u64,
    demo_session_age_secs: u64,
    session_active: bool,
    live_step_up_required: bool,
    rbac: Vec<RbacRoleProbe>,
    provider: String,
    retrieved_at: String,
}

fn role_label(role: prismatik_security::Role) -> &'static str {
    match role {
        prismatik_security::Role::Viewer => "viewer",
        prismatik_security::Role::Analyst => "analyst",
        prismatik_security::Role::Operator => "operator",
        prismatik_security::Role::Admin => "admin",
    }
}

#[tauri::command]
pub fn get_workspace_auth_floor() -> WorkspaceAuthFloor {
    use prismatik_security::{Permission, RbacPolicy, Role, SessionPolicy};

    let session = SessionPolicy::enterprise_default();
    let demo_issued_at = 1_720_000_000_u64;
    let demo_now = demo_issued_at + 3_600;
    let demo_session_age_secs = session.age_secs(demo_issued_at, demo_now);
    let session_active = session.require_active(demo_issued_at, demo_now).is_ok();

    let rbac_policy = RbacPolicy::new();
    let rbac = [Role::Viewer, Role::Analyst, Role::Operator, Role::Admin]
        .into_iter()
        .map(|role| RbacRoleProbe {
            role: role_label(role).into(),
            read_workspace: rbac_policy.evaluate(&[role], Permission::ReadWorkspace),
            write_research: rbac_policy.evaluate(&[role], Permission::WriteResearch),
            manage_paper: rbac_policy.evaluate(&[role], Permission::ManagePaperTrading),
            enable_live: rbac_policy.evaluate(&[role], Permission::EnableLiveExecution),
        })
        .collect();

    WorkspaceAuthFloor {
        session_max_age_secs: session.max_age_secs,
        demo_session_age_secs,
        session_active,
        live_step_up_required: session.require_passkey_step_up_for_live,
        rbac,
        provider: "prismatik-security::SessionPolicy+RbacPolicy".into(),
        retrieved_at: AS_OF.into(),
    }
}
