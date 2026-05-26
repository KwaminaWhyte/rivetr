//! Panic-isolation helpers for background tasks.
//!
//! With `panic = "unwind"` (see Cargo.toml), a panic in a spawned Tokio task
//! unwinds only that task instead of aborting the process. These helpers turn
//! that raw isolation into useful behavior:
//!
//! - [`guarded`]: wrap one iteration of a periodic loop (or the body of a
//!   long-running task) so a panic logs and the surrounding loop keeps running
//!   instead of the task silently dying — a transient bug doesn't kill the
//!   feature, and with `panic = "unwind"` it never reaches the process.

use std::future::Future;

use futures::FutureExt;

/// Run `fut`, catching any panic so the surrounding loop survives.
///
/// Returns `Some(output)` on normal completion, `None` if the future panicked
/// (the panic is logged against `task`).
pub async fn guarded<F>(task: &'static str, fut: F) -> Option<F::Output>
where
    F: Future,
{
    match std::panic::AssertUnwindSafe(fut).catch_unwind().await {
        Ok(v) => Some(v),
        Err(_) => {
            tracing::error!(task, "background tick panicked; loop continues");
            None
        }
    }
}
