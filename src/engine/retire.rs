//! Retire the old container(s) after a deploy or rollback swapped proxy routes.
//!
//! The new container already serves traffic. Before stopping an old one we wait
//! for requests the proxy still has in flight to it (bounded by
//! `proxy.drain_timeout_secs`), so a deploy does not cut live requests.

use std::time::Duration;

use crate::proxy::inflight;
use crate::runtime::ContainerRuntime;

/// Drain, stop and remove each old container. IDs equal to `new_container_id`
/// are skipped. Containers are retired concurrently.
pub async fn retire_old_containers(
    runtime: &dyn ContainerRuntime,
    old_ids: &[String],
    new_container_id: &str,
) {
    futures::future::join_all(
        old_ids
            .iter()
            .filter(|id| id.as_str() != new_container_id)
            .map(|id| retire_one(runtime, id)),
    )
    .await;
}

async fn retire_one(runtime: &dyn ContainerRuntime, old_id: &str) {
    let timeout = inflight::drain_timeout();
    if !timeout.is_zero() {
        // The route table addresses containers by 127.0.0.1:<published port>.
        if let Some(port) = runtime.inspect(old_id).await.ok().and_then(|i| i.port) {
            let addr = format!("127.0.0.1:{}", port);
            let left = inflight::drain(&addr, timeout).await;
            if left > 0 {
                tracing::warn!(
                    container = %old_id,
                    in_flight = left,
                    timeout_secs = timeout.as_secs(),
                    "Drain timed out, stopping old container with requests still in flight"
                );
            }
        }
    }
    let _ = runtime.stop(old_id).await;
    let _ = runtime.remove(old_id).await;
}

/// Wait until `addr` accepts TCP connections, for apps without an HTTP
/// healthcheck. Returns false if it never did within `timeout`.
pub async fn wait_for_tcp(addr: &str, timeout: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
