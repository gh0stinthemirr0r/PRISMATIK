use std::sync::OnceLock;

use tauri::{AppHandle, Manager};

mod agent_council;
mod agent_loop;
mod agent_tools;
mod analysts;
mod analytics;
mod audit_timeline;
mod autonomous_research;
mod autonomous_trader;
mod autonomy;
mod autonomy_mode;
mod backtest_runner;
mod briefings;
mod chart_indicators;
mod chat;
mod compliance;
mod consensus;
mod crowd;
mod decision_memory;
mod dex_scanner;
mod evidence_store;
mod execution;
mod feed_control;
mod feed_runtime;
mod fixed_income;
mod forecast_candidates;
mod fundamentals;
mod historical_data;
mod institutional_data;
mod integrations;
mod intelligence;
mod knowledge;
mod model_integrations;
mod paper_oms;
mod predictions;
mod prompts;
mod provider_policy;
mod quant_context;
mod risk_runtime;
mod room;
mod scheduler;
mod screener;
mod signal;
mod source_register;
mod strategy_authoring;
mod tape;
mod technique_corpus;
mod terminal_feed;
mod tracking;
mod tsfm_integration;
mod voice;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

/// Handle for backend modules that need app-scoped state (the tracked-instrument
/// store) outside of a command invocation, where Tauri cannot inject it.
pub(crate) fn app_handle() -> Result<AppHandle, String> {
    APP_HANDLE
        .get()
        .cloned()
        .ok_or_else(|| "application handle is not initialized yet".to_owned())
}

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Host operating system, for chrome that must differ per platform.
///
/// The webview user-agent is not a reliable source for this — it reports the
/// engine, not the host — and the frontend needs the answer before it can
/// decide whether to draw window controls or resize grips at all.
#[tauri::command]
fn host_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
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
    // WebKitGTK inherits the POSIX `C` locale on some Linux desktops. `C` is not
    // a valid BCP-47 language tag and crashes Intl consumers such as charts.
    // WKWebView (macOS) and WebView2 (Windows) take their locale from the OS,
    // so forcing it there would override the user's regional settings and
    // silently change how every number and date in the terminal is formatted.
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("LANG", "en_US.UTF-8");
        std::env::set_var("LC_ALL", "en_US.UTF-8");
    }
    install_panic_hook();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        // Persist geometry, but never decorations: PRISMATIK draws its own
        // window chrome (TopBar) and the window is configured decorationless.
        // Restoring the saved `decorated` flag would hand the frame back to the
        // desktop compositor and re-introduce a native title bar.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::all()
                        & !tauri_plugin_window_state::StateFlags::DECORATIONS,
                )
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            ping,
            app_version,
            host_platform,
            audit_timeline::get_audit_timeline,
            integrations::test_integration,
            integrations::integration_runtime_status,
            integrations::disconnect_integration,
            integrations::list_unofficial_sources,
            integrations::set_unofficial_source,
            screener::screen_instruments,
            intelligence::intelligence_capabilities,
            institutional_data::get_macro_series,
            institutional_data::get_filing_summaries,
            terminal_feed::get_terminal_feed,
            tracking::get_tracked_instruments,
            tracking::add_tracked_instrument,
            tracking::remove_tracked_instrument,
            tracking::reorder_tracked_instruments,
            tracking::search_instruments,
            analytics::analyze_instrument,
            analytics::regime_snapshot,
            analytics::sweep_empirical_forecasts,
            analytics::regime_catalog,
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
            agent_loop::run_agent,
            decision_memory::resolve_decision_memory,
            decision_memory::list_decision_memory,
            decision_memory::decision_track_record,
            prompts::list_prompts,
            analysts::list_analysts,
            analysts::upsert_analyst,
            analysts::delete_analyst,
            analysts::ask_analyst,
            room::open_room,
            room::list_rooms,
            room::close_room,
            room::say,
            room::advance_room,
            room::list_critic_roles,
            knowledge::add_document,
            knowledge::remove_document,
            knowledge::list_documents,
            knowledge::query_knowledge,
            technique_corpus::list_techniques,
            source_register::list_sources,
            briefings::get_briefing,
            dex_scanner::scan_dex_arbitrage,
            chart_indicators::compute_chart_indicators,
            autonomous_trader::get_autonomous_trader,
            autonomous_trader::configure_autonomous_trader,
            autonomous_trader::run_trader_loop,
            autonomous_trader::rearm_trader_ladder,
            autonomous_trader::de_escalate_trader_ladder,
            autonomous_trader::arm_live_trading,
            autonomous_trader::disarm_live_trading,
            autonomous_trader::get_live_arming,
            scheduler::start_autonomy_loop,
            scheduler::stop_autonomy_loop,
            scheduler::autonomy_loop_status,
            chat::chat_send,
            chat::chat_get_state,
            chat::chat_clear,
            chat::chat_set_active_provider,
            predictions::generate_live_prediction,
            predictions::get_predictions,
            predictions::resolve_prediction,
            predictions::clear_predictions,
            voice::tts_speak,
            voice::stt_transcribe,
            historical_data::get_historical_ohlcv,
            historical_data::get_crypto_historical,
            fundamentals::get_fundamentals,
            fixed_income::get_yield_curve,
            fixed_income::get_credit_spreads,
            execution::connect_broker,
            execution::get_broker_account,
            execution::submit_order,
            execution::list_orders,
            compliance::generate_compliance_report,
            compliance::check_pdt_rules,
            crowd::crowd_connect,
            crowd::crowd_status,
            crowd::crowd_run_simulation,
            crowd::crowd_check_status,
            crowd::crowd_get_prediction,
            crowd::crowd_history,
            crowd::crowd_build_seeds,
            crowd::crowd_file_forecast,
            consensus::consensus_for,
            tsfm_integration::tsfm_forecast,
            tape::tape_predict,
            tape::tape_batch_predict,
            tape::tape_models,
            tape::tape_predict_from_market,
            tape::tape_file_forecast,
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
            decision_memory::initialize(&data_dir)?;
            knowledge::initialize(&data_dir)?;
            analysts::initialize(&data_dir)?;
            room::initialize(&data_dir)?;
            autonomy_mode::initialize(&data_dir)?;
            autonomy::initialize(&data_dir)?;
            paper_oms::initialize(&data_dir)?;
            provider_policy::initialize(&data_dir)?;
            evidence_store::initialize(&data_dir)?;
            autonomous_research::initialize(&data_dir)?;
            strategy_authoring::initialize(&data_dir)?;
            risk_runtime::initialize(&data_dir)?;
            autonomous_trader::initialize(&data_dir)?;
            feed_runtime::start_scheduler();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run PRISMATIK desktop shell");
}
