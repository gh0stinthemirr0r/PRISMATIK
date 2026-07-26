//! Prismatik server entrypoint. Author: Aaron Stovall · Version 0.1.0 · 2026-07-07
//! Structured JSON logs via tracing; binds loopback by default; prints the API
//! token once at startup. Execution defaults to paper everywhere; live trading
//! requires the double acknowledgment described in broker.rs.

mod api;
mod broker;
mod config;
mod data;
mod engine;
mod journal;
mod live;
mod strategy;
mod types;
mod walkforward;

use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let settings = match config::Settings::from_env() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("configuration error: {e}");
            std::process::exit(1);
        }
    };

    let ui_dir = std::env::var("PRISMATIK_UI_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("../ui/build"));

    let state = api::AppState {
        settings: settings.clone(),
        jobs: Arc::new(tokio::sync::Mutex::new(Default::default())),
        sessions: Arc::new(tokio::sync::Mutex::new(Default::default())),
        ui_dir: Arc::new(ui_dir),
    };

    let addr = format!("{}:{}", settings.host, settings.port);
    tracing::info!(
        addr = %addr,
        version = config::VERSION,
        alpaca_key = %settings.alpaca.redacted_key(),
        live_unlocked = settings.alpaca.live,
        "server_start"
    );
    println!(
        "\n  Prismatik {}  ->  http://{}\n  API token (also injected into the page): {}\n  \
         Execution defaults to PAPER. Live requires the double acknowledgment.\n",
        config::VERSION, addr, settings.api_token
    );

    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind");
    axum::serve(listener, api::router(state)).await.expect("serve");
}
