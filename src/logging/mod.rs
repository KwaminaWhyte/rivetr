//! Log draining system for forwarding container logs to external services.
//!
//! Supports Axiom, New Relic, Datadog, Logtail (Better Stack), and generic HTTP endpoints.
//! Logs are buffered and sent in batches for efficiency.

pub mod drain;

pub use drain::LogDrainManager;
