// Reverse proxy module - Phase 1.8 implementation
//
// This module implements an HTTP reverse proxy that routes requests
// to containers based on the Host header.

pub mod acme;
mod handler;
mod health_checker;
mod service;
pub mod tls;

use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

pub use acme::{AcmeChallenges, AcmeClient, AcmeConfig, CertificateRenewalManager, CertificateResult};
pub use handler::ProxyHandler;
pub use health_checker::{HealthChecker, HealthCheckerConfig};
pub use service::ProxyService;
pub use tls::{CertStore, TlsConfig};

/// Backend target for proxied requests
#[derive(Debug, Clone)]
pub struct Backend {
    /// Container ID for reference
    pub container_id: String,
    /// Host address (usually 127.0.0.1 or container IP)
    pub host: String,
    /// Port the container is listening on
    pub port: u16,
    /// Whether the backend is healthy
    pub healthy: bool,
    /// Health check endpoint path (from app config)
    pub healthcheck_path: Option<String>,
    /// Consecutive failure count for health checks
    pub failure_count: u32,
}

impl Backend {
    pub fn new(container_id: String, host: String, port: u16) -> Self {
        Self {
            container_id,
            host,
            port,
            healthy: true,
            healthcheck_path: None,
            failure_count: 0,
        }
    }

    /// Create a new backend with a health check path
    pub fn with_healthcheck(mut self, path: Option<String>) -> Self {
        self.healthcheck_path = path;
        self
    }

    /// Get the backend address as a URI authority
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Get the health check URL
    pub fn health_url(&self) -> String {
        let path = self.healthcheck_path.as_deref().unwrap_or("/");
        format!("http://{}:{}{}", self.host, self.port, path)
    }
}

/// Thread-safe route table for mapping domains to backends
#[derive(Debug, Default)]
pub struct RouteTable {
    routes: DashMap<String, Backend>,
}

impl RouteTable {
    pub fn new() -> Self {
        Self {
            routes: DashMap::new(),
        }
    }

    /// Add or update a route for a domain
    pub fn add_route(&self, domain: String, backend: Backend) {
        info!(domain = %domain, backend = ?backend.addr(), "Adding proxy route");
        self.routes.insert(domain, backend);
    }

    /// Remove a route for a domain
    pub fn remove_route(&self, domain: &str) {
        info!(domain = %domain, "Removing proxy route");
        self.routes.remove(domain);
    }

    /// Get the backend for a domain
    pub fn get_backend(&self, domain: &str) -> Option<Backend> {
        // Try exact match first
        if let Some(backend) = self.routes.get(domain) {
            return Some(backend.clone());
        }

        // Try stripping port from domain (e.g., "example.com:8080" -> "example.com")
        if let Some(host) = domain.split(':').next() {
            if let Some(backend) = self.routes.get(host) {
                return Some(backend.clone());
            }
        }

        None
    }

    /// Mark a backend as healthy or unhealthy
    pub fn set_health(&self, domain: &str, healthy: bool) {
        if let Some(mut backend) = self.routes.get_mut(domain) {
            backend.healthy = healthy;
        }
    }

    /// Update health status based on check result
    /// Returns true if health status changed
    pub fn update_health(&self, domain: &str, check_passed: bool, failure_threshold: u32) -> bool {
        if let Some(mut backend) = self.routes.get_mut(domain) {
            let was_healthy = backend.healthy;

            if check_passed {
                // Reset failure count on success
                backend.failure_count = 0;
                backend.healthy = true;
            } else {
                // Increment failure count
                backend.failure_count += 1;
                if backend.failure_count >= failure_threshold {
                    backend.healthy = false;
                }
            }

            return was_healthy != backend.healthy;
        }
        false
    }

    /// Get all registered domains
    pub fn domains(&self) -> Vec<String> {
        self.routes.iter().map(|r| r.key().clone()).collect()
    }

    /// Get all backends with their domains for health checking
    pub fn all_backends(&self) -> Vec<(String, Backend)> {
        self.routes
            .iter()
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect()
    }

    /// Check if a domain is registered
    pub fn has_domain(&self, domain: &str) -> bool {
        self.routes.contains_key(domain)
    }
}

/// Proxy server that listens for incoming HTTP connections
pub struct ProxyServer {
    routes: Arc<ArcSwap<RouteTable>>,
    bind_addr: SocketAddr,
}

impl ProxyServer {
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self {
            routes: Arc::new(ArcSwap::new(Arc::new(RouteTable::new()))),
            bind_addr,
        }
    }

    /// Get a reference to the route table for updates
    pub fn routes(&self) -> Arc<ArcSwap<RouteTable>> {
        self.routes.clone()
    }

    /// Start the proxy server (HTTP)
    pub async fn run(self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!("Proxy server listening on http://{}", self.bind_addr);

        let handler = ProxyHandler::new(self.routes.clone());

        loop {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    let handler = handler.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handler.handle_connection(stream, remote_addr).await {
                            error!(error = %e, "Error handling proxy connection");
                        }
                    });
                }
                Err(e) => {
                    error!(error = %e, "Error accepting connection");
                }
            }
        }
    }
}

/// HTTPS proxy server that listens for TLS connections
pub struct HttpsProxyServer {
    routes: Arc<ArcSwap<RouteTable>>,
    bind_addr: SocketAddr,
    tls_config: tls::TlsConfig,
}

impl HttpsProxyServer {
    pub fn new(
        bind_addr: SocketAddr,
        routes: Arc<ArcSwap<RouteTable>>,
        tls_config: tls::TlsConfig,
    ) -> Self {
        Self {
            routes,
            bind_addr,
            tls_config,
        }
    }

    /// Start the HTTPS proxy server
    pub async fn run(self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!("Proxy server listening on https://{}", self.bind_addr);

        let handler = ProxyHandler::new(self.routes.clone());
        let acceptor = self.tls_config.acceptor;

        loop {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    let handler = handler.clone();
                    let acceptor = acceptor.clone();

                    tokio::spawn(async move {
                        // Perform TLS handshake
                        match acceptor.accept(stream).await {
                            Ok(tls_stream) => {
                                if let Err(e) = handler.handle_tls_connection(tls_stream, remote_addr).await {
                                    error!(error = %e, "Error handling HTTPS proxy connection");
                                }
                            }
                            Err(e) => {
                                error!(error = %e, remote = %remote_addr, "TLS handshake failed");
                            }
                        }
                    });
                }
                Err(e) => {
                    error!(error = %e, "Error accepting HTTPS connection");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_table_add_get() {
        let table = RouteTable::new();
        let backend = Backend::new("container-123".into(), "127.0.0.1".into(), 3000);

        table.add_route("example.com".into(), backend);

        let result = table.get_backend("example.com");
        assert!(result.is_some());
        let backend = result.unwrap();
        assert_eq!(backend.port, 3000);
    }

    #[test]
    fn test_route_table_strip_port() {
        let table = RouteTable::new();
        let backend = Backend::new("container-123".into(), "127.0.0.1".into(), 3000);

        table.add_route("example.com".into(), backend);

        // Should match even with port in the query
        let result = table.get_backend("example.com:8080");
        assert!(result.is_some());
    }

    #[test]
    fn test_route_table_remove() {
        let table = RouteTable::new();
        let backend = Backend::new("container-123".into(), "127.0.0.1".into(), 3000);

        table.add_route("example.com".into(), backend);
        table.remove_route("example.com");

        assert!(table.get_backend("example.com").is_none());
    }

    #[test]
    fn test_backend_health_url_default() {
        let backend = Backend::new("container-123".into(), "127.0.0.1".into(), 3000);
        assert_eq!(backend.health_url(), "http://127.0.0.1:3000/");
    }

    #[test]
    fn test_backend_health_url_with_path() {
        let backend = Backend::new("container-123".into(), "127.0.0.1".into(), 3000)
            .with_healthcheck(Some("/health".to_string()));
        assert_eq!(backend.health_url(), "http://127.0.0.1:3000/health");
    }

    #[test]
    fn test_route_table_update_health_threshold() {
        let table = RouteTable::new();
        let backend = Backend::new("container-123".into(), "127.0.0.1".into(), 3000);
        table.add_route("example.com".into(), backend);

        // Initial state: healthy
        let b = table.get_backend("example.com").unwrap();
        assert!(b.healthy);
        assert_eq!(b.failure_count, 0);

        // First failure: still healthy (threshold is 3)
        let changed = table.update_health("example.com", false, 3);
        assert!(!changed);
        let b = table.get_backend("example.com").unwrap();
        assert!(b.healthy);
        assert_eq!(b.failure_count, 1);

        // Second failure: still healthy
        let changed = table.update_health("example.com", false, 3);
        assert!(!changed);
        let b = table.get_backend("example.com").unwrap();
        assert!(b.healthy);
        assert_eq!(b.failure_count, 2);

        // Third failure: now unhealthy
        let changed = table.update_health("example.com", false, 3);
        assert!(changed);
        let b = table.get_backend("example.com").unwrap();
        assert!(!b.healthy);
        assert_eq!(b.failure_count, 3);
    }

    #[test]
    fn test_route_table_health_recovery() {
        let table = RouteTable::new();
        let backend = Backend::new("container-123".into(), "127.0.0.1".into(), 3000);
        table.add_route("example.com".into(), backend);

        // Make it unhealthy first
        table.update_health("example.com", false, 1);
        let b = table.get_backend("example.com").unwrap();
        assert!(!b.healthy);

        // Recovery: single success makes it healthy again
        let changed = table.update_health("example.com", true, 1);
        assert!(changed);
        let b = table.get_backend("example.com").unwrap();
        assert!(b.healthy);
        assert_eq!(b.failure_count, 0);
    }

    #[test]
    fn test_route_table_all_backends() {
        let table = RouteTable::new();

        let backend1 = Backend::new("container-1".into(), "127.0.0.1".into(), 3000);
        let backend2 = Backend::new("container-2".into(), "127.0.0.1".into(), 3001);

        table.add_route("app1.example.com".into(), backend1);
        table.add_route("app2.example.com".into(), backend2);

        let backends = table.all_backends();
        assert_eq!(backends.len(), 2);
    }
}
