use std::sync::OnceLock;

use tauri::{AppHandle, Manager};

mod equity_fixtures;
mod market;
mod research_fixtures;
mod state;
mod watchlist;

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
            // Best-effort graceful exit after a panic (crash recovery path).
            app.exit(1);
        }
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_panic_hook();

    // P0-EX-02: tauri-specta export is scaffolded under packages/api-client.
    // Specta 2.0.0-rc.25 currently requires unstable `debug_closure_helpers`
    // (beyond MSRV 1.88). Re-enable export when Specta stabilizes or MSRV
    // bumps — see ADR-0031 / packages/api-client/README.md.

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        // Persists and restores the main window's geometry between launches.
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(state::MarketRuntime::new())
        .invoke_handler(tauri::generate_handler![
            ping,
            app_version,
            market::get_crypto_markets,
            market::get_crypto_global,
            market::get_crypto_trending,
            market::get_crypto_ohlc,
            market::get_coin_detail,
            market::get_crypto_categories,
            market::get_crypto_exchanges,
            market::search_crypto,
            state::get_rate_budget_state,
            state::spend_rate_budget,
            watchlist::get_watchlist,
            watchlist::save_watchlist,
            watchlist::get_scanner_filters,
            watchlist::save_scanner_filters,
            watchlist::list_alert_rules,
            watchlist::upsert_alert_rule,
            watchlist::get_data_mode,
            equity_fixtures::get_equity_bars,
            equity_fixtures::get_filing_summaries,
            equity_fixtures::get_insider_transactions,
            equity_fixtures::get_macro_series,
            equity_fixtures::get_cot_report,
            equity_fixtures::get_options_chain,
            equity_fixtures::get_options_flow,
            equity_fixtures::get_dealer_exposure,
            equity_fixtures::get_catalyst_calendar,
            equity_fixtures::get_next_session,
            research_fixtures::get_backtest_summary,
            research_fixtures::get_portfolio_snapshot,
            research_fixtures::get_marketplace_listings,
            research_fixtures::get_strategy_ir_preview,
            research_fixtures::get_monte_carlo_paths,
            research_fixtures::preview_order_ticket,
            research_fixtures::get_calibration_ribbon,
            research_fixtures::get_model_card,
            research_fixtures::get_analog_hits,
            research_fixtures::get_journal_entries,
            research_fixtures::get_plugin_host_status,
            research_fixtures::preview_plugin_install,
            research_fixtures::get_workspace_auth_floor,
        ])
        .setup(|app| {
            APP_HANDLE
                .set(app.handle().clone())
                .map_err(|_| "application handle was initialized twice")?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run PRISMATIK desktop shell");
}
