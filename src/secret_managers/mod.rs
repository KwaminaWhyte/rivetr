//! External secret managers resolved at deploy time (gap #9).
//!
//! - [`providers`]: HashiCorp Vault / OpenBao, Infisical, Doppler and AWS Secrets
//!   Manager clients behind the [`providers::SecretProvider`] trait.
//! - [`reference`]: the `secret://<connection>/<path>#<key>` value syntax.
//! - [`store`]: connections (credentials encrypted), path imports, failure policy.
//! - [`resolve`]: resolution for app env, build secrets and compose services.
//! - [`session`]: per-deployment in-memory cache and log masking.
//!
//! Setup per provider: `docs/reference/secret-managers.md`.

pub mod providers;
pub mod reference;
pub mod resolve;
pub mod session;
pub mod sigv4;
pub mod store;

pub use reference::{is_reference, SecretRef};
pub use resolve::{resolve_app_build_secrets, resolve_app_env, resolve_service_env, Outcome};
