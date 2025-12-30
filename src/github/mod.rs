//! GitHub integration module for GitHub App support.
//!
//! This module provides:
//! - JWT token generation for GitHub App authentication
//! - Installation access token management
//! - GitHub API client for repository operations

pub mod api_client;
pub mod token_manager;

pub use api_client::GitHubClient;
pub use token_manager::{generate_app_jwt, get_installation_token};
