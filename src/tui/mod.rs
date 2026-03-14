//! Terminal UI for Rivetr — invoked via `rivetr tui`.
//!
//! Requires the `tui` feature flag:
//! ```
//! cargo run --features tui -- tui --url http://localhost:8080 --token <api_token>
//! ```

pub mod api;
pub mod app;
pub mod ui;

use anyhow::Result;

/// Entry point called from `src/cli/mod.rs`.
pub fn run(url: String, token: String) -> Result<()> {
    let client = api::ApiClient::new(url.clone(), token)?;
    let state = app::AppState::new(client, url);
    app::run_tui(state)
}
