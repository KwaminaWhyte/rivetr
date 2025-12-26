// Reverse proxy module - Phase 1.8 implementation
//
// This module implements an HTTP reverse proxy that routes requests
// to containers based on the Host header.

mod handler;
mod service;

use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

pub use handler::ProxyHandler;
pub use service::ProxyService;

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
}

impl Backend {
    pub fn new(container_id: String, host: String, port: u16) -> Self {
        Self {
            container_id,
            host,
            port,
            healthy: true,
        }
    }

    /// Get the backend address as a URI authority
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
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

    /// Get all registered domains
    pub fn domains(&self) -> Vec<String> {
        self.routes.iter().map(|r| r.key().clone()).collect()
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

    /// Start the proxy server
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
}
