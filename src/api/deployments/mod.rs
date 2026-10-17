//! Deployment management API endpoints.
//!
//! Organized into focused submodules:
//! - `handlers`  — list, get, trigger, upload, stats, commits, tags
//! - `rollback`  — rollback to previous deployment
//! - `snapshots` - pre-deploy database snapshots (list; restored by rollback)
//! - `approval`  — approve/reject pending deployments
//! - `freeze`    — deployment freeze windows (CRUD + check helper)
//! - `shared`    — shared helpers (encryption key)

mod approval;
mod freeze;
mod handlers;
mod rollback;
mod shared;
mod snapshots;

pub use approval::*;
pub use freeze::*;
pub use handlers::*;
pub use rollback::*;
pub use snapshots::*;
