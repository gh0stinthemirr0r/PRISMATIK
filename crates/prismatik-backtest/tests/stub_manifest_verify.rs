//! End-to-end: stub backtest → temp manifest → `prismatik-cli verify`.

use prismatik_backtest::{BacktestConfig, BacktestEngine, BacktestWindow, ExecutionAssumptions};
use prismatik_cli::verify_manifest_file;
use tempfile::NamedTempFile;
use time::macros::datetime;
use time::Duration;
use time::OffsetDateTime;

#[test]
fn stub_run_manifest_accepted_by_cli_verify() {
    let config = BacktestConfig {
        strategy_id: "strategy/integration-stub".into(),
        window: BacktestWindow {
            start: OffsetDateTime::UNIX_EPOCH,
            end: OffsetDateTime::UNIX_EPOCH + Duration::days(1),
        },
        starting_capital_micros: 50_000_000_000,
        execution: ExecutionAssumptions::default(),
        bar_interval_seconds: 3_600,
    };

    let engine = BacktestEngine::new(config).unwrap();
    let at = datetime!(2026-07-26 14:32:00 UTC);
    let stub_run = engine.run_stub_signed(at).expect("signed manifest");

    let json = serde_json::to_string_pretty(&stub_run.manifest).expect("manifest json");
    let file = NamedTempFile::new().expect("temp file");
    std::fs::write(file.path(), &json).expect("write manifest");

    let path = file.path().to_string_lossy();
    let report = verify_manifest_file(path.as_ref(), true, at).expect("cli verify");
    assert!(report.overall, "{report:?}");
    assert!(report.signature_ed25519_ok);
    assert!(report.schema_version_ok);
}
