//! Wave 3 DoD #10 floor — same logical strategy from hand-built IR, DSL, and
//! Python emitter fixture yields structurally equal canonical fields.
//!
//! Full IR byte equality is not required: `strategy_id` may differ (UUID vs
//! DSL stub id). Compared: name, schema_version, capability defaults,
//! universe kind, and SMA indicator presence/shape.

use prismatik_strategy::{
    parse_typecheck_to_ir_stub, IndicatorRef, StrategyCapabilities, StrategyIr, UniverseSpec,
    STRATEGY_IR_SCHEMA_VERSION,
};

const DSL_SOURCE: &str = "strategy Momentum { sma(close, 20) > 0 }";
const STRATEGY_NAME: &str = "Momentum";
const PYTHON_FIXTURE: &str = include_str!("fixtures/momentum_sma20_strategy_ir.json");

fn hand_built_momentum() -> StrategyIr {
    StrategyIr {
        schema_version: STRATEGY_IR_SCHEMA_VERSION.into(),
        strategy_id: "06d2e7e0-0000-4000-8000-000000000010".into(),
        name: STRATEGY_NAME.into(),
        description: None,
        capabilities: StrategyCapabilities::default(),
        universe: UniverseSpec::Static {
            static_members: Vec::new(),
        },
        indicators: vec![IndicatorRef {
            kind: "sma".into(),
            alias: "sma_20".into(),
            warmup: 19,
        }],
        rules: serde_json::json!({ "entries": [], "exits": [] }),
    }
}

fn universe_kind(ir: &StrategyIr) -> &'static str {
    match ir.universe {
        UniverseSpec::Static { .. } => "static",
        UniverseSpec::DynamicQuery { .. } => "dynamic_query",
    }
}

fn has_sma20(ir: &StrategyIr) -> bool {
    ir.indicators
        .iter()
        .any(|i| i.kind.eq_ignore_ascii_case("sma") && i.alias == "sma_20" && i.warmup == 19)
}

/// Canonical floor fields shared across authoring modes (excludes strategy_id / rules).
fn assert_canonical_parity(label: &str, ir: &StrategyIr, reference: &StrategyIr) {
    assert_eq!(
        ir.name, reference.name,
        "{label}: name mismatch ({} vs {})",
        ir.name, reference.name
    );
    assert_eq!(
        ir.schema_version, reference.schema_version,
        "{label}: schema_version mismatch"
    );
    assert_eq!(
        ir.schema_version, STRATEGY_IR_SCHEMA_VERSION,
        "{label}: expected schema {STRATEGY_IR_SCHEMA_VERSION}"
    );
    assert_eq!(
        ir.capabilities, reference.capabilities,
        "{label}: capabilities structural mismatch"
    );
    assert_eq!(
        ir.capabilities,
        StrategyCapabilities::default(),
        "{label}: capabilities must be deny-by-default floor"
    );
    assert_eq!(
        universe_kind(ir),
        universe_kind(reference),
        "{label}: universe kind mismatch"
    );
    assert!(
        has_sma20(ir),
        "{label}: missing SMA indicator (kind=sma, alias=sma_20, warmup=19); got {:?}",
        ir.indicators
    );
    assert_eq!(
        ir.indicators, reference.indicators,
        "{label}: indicators structural mismatch"
    );
}

#[test]
fn dod10_hand_built_and_dsl_structural_parity() {
    let hand = hand_built_momentum();
    let dsl = parse_typecheck_to_ir_stub(DSL_SOURCE).expect("DSL parse+typecheck+ir");

    assert_canonical_parity("dsl", &dsl, &hand);
    // strategy_id intentionally excluded: DSL stub ids differ from SDK UUIDs.
}

#[test]
fn dod10_python_emitter_fixture_structural_parity() {
    let hand = hand_built_momentum();
    let dsl = parse_typecheck_to_ir_stub(DSL_SOURCE).expect("DSL parse+typecheck+ir");
    let python: StrategyIr =
        serde_json::from_str(PYTHON_FIXTURE).expect("python golden fixture deserializes");

    assert_canonical_parity("python-fixture", &python, &hand);
    assert_canonical_parity("python-vs-dsl", &python, &dsl);
    assert_eq!(
        python.strategy_id, hand.strategy_id,
        "committed Python golden uses the hand-built UUID"
    );
}
