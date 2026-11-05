//! End-to-end tests for the proxy listeners: HTTP/2 over TLS (ALPN), h2c with
//! prior knowledge on the plain listener, and HTTP/1.1 WebSocket upgrades
//! through the TLS listener. Everything runs on loopback with a throwaway CA.

use std::net::SocketAddr;
use std::sync::Arc;

use arc_swap::ArcSwap;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode, Version};
use hyper_util::rt::{TokioExecutor, TokioIo};
use rustls::pki_types::{CertificateDer, ServerName};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::{Backend, ProxyHandler, RouteTable};

pub(crate) struct TestPki {
    pub ca_der: CertificateDer<'static>,
    pub cert_pem: String,
    pub key_pem: String,
}

/// A CA plus a leaf for `names`, signed by that CA.
pub(crate) fn test_pki(names: &[&str]) -> TestPki {
    use rcgen::{BasicConstraints, CertificateParams, IsCa, Issuer, KeyPair};
    let ca_key = KeyPair::generate().unwrap();
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca_cert = ca_params.self_signed(&ca_key).unwrap();
    let issuer = Issuer::new(ca_params, &ca_key);

    let leaf_key = KeyPair::generate().unwrap();
    let leaf_params =
        CertificateParams::new(names.iter().map(|n| n.to_string()).collect::<Vec<_>>()).unwrap();
    let leaf = leaf_params.signed_by(&leaf_key, &issuer).unwrap();
    TestPki {
        ca_der: ca_cert.der().clone(),
        cert_pem: pem::encode(&pem::Pem::new("CERTIFICATE", leaf.der().to_vec())),
        key_pem: pem::encode(&pem::Pem::new("PRIVATE KEY", leaf_key.serialize_der())),
    }
}

/// HTTP/1.1-only backend. Echoes what it saw; answers WebSocket upgrades with
/// 101 and then echoes raw bytes.
async fn spawn_backend() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let service = hyper::service::service_fn(|mut req: Request<Incoming>| async move {
                    if req.headers().contains_key(hyper::header::UPGRADE) {
                        let on_upgrade = hyper::upgrade::on(&mut req);
                        tokio::spawn(async move {
                            if let Ok(upgraded) = on_upgrade.await {
                                let mut io = TokioIo::new(upgraded);
                                let mut buf = [0u8; 64];
                                while let Ok(n) = io.read(&mut buf).await {
                                    if n == 0 || io.write_all(&buf[..n]).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        });
                        return Ok::<_, hyper::Error>(
                            Response::builder()
                                .status(StatusCode::SWITCHING_PROTOCOLS)
                                .header("upgrade", "websocket")
                                .header("connection", "Upgrade")
                                .body(Full::new(Bytes::new()))
                                .unwrap(),
                        );
                    }
                    let header = |name: &str| {
                        req.headers()
                            .get(name)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("-")
                            .to_string()
                    };
                    let body = format!(
                        "version={:?} host={} cookie={} proto={}",
                        req.version(),
                        header("host"),
                        header("cookie"),
                        header("x-forwarded-proto"),
                    );
                    Ok(Response::new(Full::new(Bytes::from(body))))
                });
                let _ = hyper::server::conn::http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .with_upgrades()
                    .await;
            });
        }
    });
    addr
}

fn routes_to(backend: SocketAddr) -> Arc<ArcSwap<RouteTable>> {
    let table = RouteTable::new();
    table.add_route(
        "app.test".into(),
        Backend::new("c1".into(), "127.0.0.1".into(), backend.port()),
    );
    Arc::new(ArcSwap::from_pointee(table))
}

/// Start the TLS listener the same way `HttpsProxyServer::run` does.
async fn spawn_tls_proxy(
    routes: Arc<ArcSwap<RouteTable>>,
    acceptor: tokio_rustls::TlsAcceptor,
) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handler = ProxyHandler::new(routes).with_tls();
    tokio::spawn(async move {
        loop {
            let Ok((stream, remote)) = listener.accept().await else {
                return;
            };
            let handler = handler.clone();
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(stream).await {
                    let _ = handler.handle_tls_connection(tls, remote).await;
                }
            });
        }
    });
    addr
}

async fn spawn_plain_proxy(routes: Arc<ArcSwap<RouteTable>>) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handler = ProxyHandler::new(routes);
    tokio::spawn(async move {
        loop {
            let Ok((stream, remote)) = listener.accept().await else {
                return;
            };
            let handler = handler.clone();
            tokio::spawn(async move {
                let _ = handler.handle_connection(stream, remote).await;
            });
        }
    });
    addr
}

async fn tls_client(
    addr: SocketAddr,
    ca: &CertificateDer<'static>,
    sni: &str,
    alpn: &[u8],
) -> tokio_rustls::client::TlsStream<TcpStream> {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(ca.clone()).unwrap();
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    config.alpn_protocols = vec![alpn.to_vec()];
    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    let tcp = TcpStream::connect(addr).await.unwrap();
    connector
        .connect(ServerName::try_from(sni.to_string()).unwrap(), tcp)
        .await
        .unwrap()
}

async fn body_string(resp: Response<Incoming>) -> String {
    String::from_utf8(
        resp.into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap()
}

pub(crate) async fn h2_get_over_tls(
    addr: SocketAddr,
    ca: &CertificateDer<'static>,
    host: &str,
) -> (Version, String) {
    let tls = tls_client(addr, ca, host, b"h2").await;
    assert_eq!(tls.get_ref().1.alpn_protocol(), Some(&b"h2"[..]));
    let (mut sender, conn) =
        hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(tls))
            .await
            .unwrap();
    tokio::spawn(conn);
    let req = Request::builder()
        .uri(format!("https://{host}/hello"))
        .header("cookie", "a=1")
        .header("cookie", "b=2")
        .body(Empty::<Bytes>::new())
        .unwrap();
    let resp = sender.send_request(req).await.unwrap();
    let version = resp.version();
    (version, body_string(resp).await)
}

#[tokio::test]
async fn tls_listener_serves_http2_via_alpn() {
    let pki = test_pki(&["app.test"]);
    let backend = spawn_backend().await;
    let acceptor = super::tls::TlsConfig::from_pem(&pki.cert_pem, &pki.key_pem)
        .unwrap()
        .acceptor;
    let proxy = spawn_tls_proxy(routes_to(backend), acceptor).await;

    let (version, body) = h2_get_over_tls(proxy, &pki.ca_der, "app.test").await;
    assert_eq!(version, Version::HTTP_2);
    // The backend got HTTP/1.1 with Host restored and cookies joined.
    assert!(body.contains("version=HTTP/1.1"), "{body}");
    assert!(body.contains("host=app.test"), "{body}");
    assert!(body.contains("cookie=a=1; b=2"), "{body}");
    assert!(body.contains("proto=https"), "{body}");
}

#[tokio::test]
async fn tls_listener_keeps_websocket_upgrades_on_http1() {
    let pki = test_pki(&["app.test"]);
    let backend = spawn_backend().await;
    let acceptor = super::tls::TlsConfig::from_pem(&pki.cert_pem, &pki.key_pem)
        .unwrap()
        .acceptor;
    let proxy = spawn_tls_proxy(routes_to(backend), acceptor).await;

    let tls = tls_client(proxy, &pki.ca_der, "app.test", b"http/1.1").await;
    let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(tls))
        .await
        .unwrap();
    tokio::spawn(conn.with_upgrades());
    let req = Request::builder()
        .uri("/ws")
        .header("host", "app.test")
        .header("upgrade", "websocket")
        .header("connection", "Upgrade")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("sec-websocket-version", "13")
        .body(Empty::<Bytes>::new())
        .unwrap();
    let resp = sender.send_request(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SWITCHING_PROTOCOLS);
    let upgraded = hyper::upgrade::on(resp).await.unwrap();
    let mut io = TokioIo::new(upgraded);
    io.write_all(b"ping").await.unwrap();
    let mut buf = [0u8; 4];
    io.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"ping");
}

#[tokio::test]
async fn plain_listener_accepts_h2c_prior_knowledge_and_http1() {
    let backend = spawn_backend().await;
    let proxy = spawn_plain_proxy(routes_to(backend)).await;

    // HTTP/2 with prior knowledge.
    let tcp = TcpStream::connect(proxy).await.unwrap();
    let (mut sender, conn) =
        hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(tcp))
            .await
            .unwrap();
    tokio::spawn(conn);
    let req = Request::builder()
        .uri("http://app.test/")
        .body(Empty::<Bytes>::new())
        .unwrap();
    let resp = sender.send_request(req).await.unwrap();
    assert_eq!(resp.version(), Version::HTTP_2);
    let body = body_string(resp).await;
    assert!(body.contains("host=app.test"), "{body}");
    assert!(body.ends_with("proto=http"), "{body}");

    // Plain HTTP/1.1 still works.
    let tcp = TcpStream::connect(proxy).await.unwrap();
    let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(tcp))
        .await
        .unwrap();
    tokio::spawn(conn);
    let req = Request::builder()
        .uri("/")
        .header("host", "app.test")
        .body(Empty::<Bytes>::new())
        .unwrap();
    let resp = sender.send_request(req).await.unwrap();
    assert_eq!(resp.version(), Version::HTTP_11);
    assert!(body_string(resp).await.contains("version=HTTP/1.1"));
}

#[test]
fn alpn_selects_http_version() {
    use super::handler::HttpVersions;
    assert_eq!(HttpVersions::from_alpn(Some(b"h2")), HttpVersions::Http2);
    assert_eq!(
        HttpVersions::from_alpn(Some(b"http/1.1")),
        HttpVersions::Http1
    );
    assert_eq!(HttpVersions::from_alpn(None), HttpVersions::Http1);
}
