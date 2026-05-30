//! Chronos-Graph server binary.

use chronos_embedded::FactStore;
use chronos_server::{serve, AppState};
use std::sync::Arc;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber_init();

    let addr = std::env::var("CHRONOS_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()
        .expect("CHRONOS_ADDR must be a valid socket address");

    let state = AppState::new(Arc::new(FactStore::new()));
    serve(addr, state).await
}

/// Minimal tracing setup without pulling tracing-subscriber: log to stderr via
/// the `tracing` macros is a no-op without a subscriber, so we just print the
/// bind address. A real subscriber is wired in M3+ observability work.
fn tracing_subscriber_init() {
    eprintln!("chronos-server starting (set CHRONOS_ADDR to override 127.0.0.1:8080)");
}
