use std::sync::OnceLock;

use tauri::{AppHandle, Manager};

mod audit_timeline;
mod agent_council;
mod autonomous_research;
mod autonomy;
mod backtest_runner;
mod briefings;
mod evidence_store;
mod feed_control;
mod feed_runtime;
mod forecast_candidates;
mod institutional_data;
mod integrations;
mod intelligence;
mod model_integrations;
mod paper_oms;
mod provider_policy;
mod risk_runtime;
mod strategy_authoring;
mod terminal_feed;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn install_panic_hook() {
    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info| {
        eprintln!("PRISMATIK desktop panic: {panic_info}");
        default_hook(panic_info);

        if let Some(app) = APP_HANDLE.get() {
            app.exit(1);
        }
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(
    clippy::disallowed_types,
    reason = "Tauri's generated context owns its internal map type"
)]
pub fn run() {
    // WebKit inherits the POSIX `C` locale on some Linux desktops. `C` is not
    // a valid BCP-47 language tag and crashes Intl consumers such as charts.
    std::env::set_var("LANG", "en_US.UTF-8");
    std::env::set_var("LC_ALL", "en_US.UTF-8");
    install_panic_hook();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            ping,
            app_version,
            audit_timeline::get_audit_timeline,
            integrations::test_integration,
            integrations::integration_runtime_status,
            integrations::disconnect_integration,
            intelligence::intelligence_capabilities,
            institutional_data::get_macro_series,
            institutional_data::get_filing_summaries,
            terminal_feed::get_terminal_feed,
            autonomy::get_autonomy_budget,
            autonomy::configure_autonomy_budget,
            model_integrations::test_model_provider,
            model_integrations::model_runtime_status,
            model_integrations::disconnect_model_provider,
            model_integrations::run_market_research,
            autonomous_research::get_autonomous_research,
            autonomous_research::configure_autonomous_research,
            feed_control::list_feed_sources,
            feed_control::upsert_feed_source,
            feed_control::import_feed_sources,
            feed_control::test_feed_source,
            feed_runtime::feed_runtime_status,
            feed_runtime::run_feed_scheduler_once,
            forecast_candidates::list_forecast_candidates,
            forecast_candidates::generate_forecast_candidate,
            forecast_candidates::resolve_due_forecast_candidates,
            forecast_candidates::forecast_calibration_health,
            paper_oms::get_paper_oms,
            paper_oms::submit_paper_order,
            provider_policy::list_provider_policies,
            provider_policy::upsert_provider_policy,
            strategy_authoring::get_strategy_authoring_contract,
            strategy_authoring::validate_strategy,
            strategy_authoring::list_strategy_drafts,
            strategy_authoring::save_strategy,
            risk_runtime::get_risk_state,
            risk_runtime::configure_risk_budget,
            risk_runtime::rearm_circuit_breaker,
            risk_runtime::size_position_by_risk,
            backtest_runner::run_backtest,
            backtest_runner::list_seed_strategies,
            agent_council::run_agent_council,
            briefings::get_briefing,
        ])
        .setup(|app| {
            APP_HANDLE
                .set(app.handle().clone())
                .map_err(|_| "application handle was initialized twice")?;
            let data_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|error| format!("resolve app data directory: {error}"))?;
            intelligence::initialize(&data_dir)?;
            feed_control::initialize(&data_dir)?;
            feed_runtime::initialize(&data_dir)?;
            forecast_candidates::initialize(&data_dir)?;
            autonomy::initialize(&data_dir)?;
            paper_oms::initialize(&data_dir)?;
            provider_policy::initialize(&data_dir)?;
            evidence_store::initialize(&data_dir)?;
            autonomous_research::initialize(&data_dir)?;
            strategy_authoring::initialize(&data_dir)?;
            risk_runtime::initialize(&data_dir)?;
            feed_runtime::start_scheduler();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run PRISMATIK desktop shell");
}
