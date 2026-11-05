//! ACME order flow against a local mock CA: HTTP-01 and DNS-01 challenges,
//! failure isolation between domains, backoff bookkeeping.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use parking_lot::Mutex;
use serde_json::{json, Value};

use super::store::CertSource;
use super::{CertManager, CertRuntime, CertStore};
use crate::proxy::acme::{
    dns01_record_name, AcmeChallenges, AcmeClient, AcmeConfig, ChallengeSolver, Dns01Solver,
};

/// Recorded TXT records, shared between the fake DNS solver and the mock CA.
#[derive(Clone, Default)]
pub(crate) struct FakeDns {
    pub records: Arc<Mutex<HashMap<String, String>>>,
    pub removed: Arc<Mutex<Vec<String>>>,
}

#[async_trait::async_trait]
impl Dns01Solver for FakeDns {
    async fn present(&self, fqdn: &str, value: &str) -> anyhow::Result<String> {
        self.records
            .lock()
            .insert(fqdn.to_string(), value.to_string());
        Ok(format!("rec-{fqdn}"))
    }
    async fn wait_for_propagation(&self, _fqdn: &str, _value: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn cleanup(&self, fqdn: &str, record_id: &str) -> anyhow::Result<()> {
        assert_eq!(record_id, format!("rec-{fqdn}"));
        self.records.lock().remove(fqdn);
        self.removed.lock().push(fqdn.to_string());
        Ok(())
    }
}

struct Authz {
    domain: String,
    wildcard: bool,
    status: &'static str,
    error: Option<String>,
}

struct Order {
    authz: Vec<usize>,
    names: Vec<String>,
    status: &'static str,
    cert: Option<String>,
}

#[derive(Clone)]
struct Mock {
    base: String,
    http01: AcmeChallenges,
    dns: FakeDns,
    /// Domains whose validation the CA rejects.
    reject: Vec<String>,
    authz: Arc<Mutex<Vec<Authz>>>,
    orders: Arc<Mutex<Vec<Order>>>,
    ca: Arc<(rcgen::CertificateParams, rcgen::KeyPair)>,
}

fn payload(body: &Value) -> Value {
    let p = body["payload"].as_str().unwrap_or("");
    if p.is_empty() {
        return Value::Null;
    }
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(p).unwrap()).unwrap()
}

fn nonce_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert("replay-nonce", "n".parse().unwrap());
    h
}

async fn directory(State(m): State<Mock>) -> Json<Value> {
    Json(json!({
        "newNonce": format!("{}/nonce", m.base),
        "newAccount": format!("{}/account", m.base),
        "newOrder": format!("{}/order", m.base),
    }))
}

async fn nonce() -> Response {
    (StatusCode::OK, nonce_headers()).into_response()
}

async fn account(State(m): State<Mock>) -> Response {
    let mut h = nonce_headers();
    h.insert("location", format!("{}/acct/1", m.base).parse().unwrap());
    (StatusCode::CREATED, h, Json(json!({"status": "valid"}))).into_response()
}

fn order_json(m: &Mock, id: usize, o: &Order) -> Value {
    json!({
        "status": o.status,
        "authorizations": o.authz.iter().map(|a| format!("{}/authz/{a}", m.base)).collect::<Vec<_>>(),
        "finalize": format!("{}/finalize/{id}", m.base),
        "certificate": o.cert.as_ref().map(|_| format!("{}/cert/{id}", m.base)),
    })
}

async fn new_order(State(m): State<Mock>, Json(body): Json<Value>) -> Response {
    let ids: Vec<String> = payload(&body)["identifiers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["value"].as_str().unwrap().to_string())
        .collect();
    let mut authz = m.authz.lock();
    let mut refs = Vec::new();
    for name in &ids {
        authz.push(Authz {
            domain: name.trim_start_matches("*.").to_string(),
            wildcard: name.starts_with("*."),
            status: "pending",
            error: None,
        });
        refs.push(authz.len() - 1);
    }
    let mut orders = m.orders.lock();
    orders.push(Order {
        authz: refs,
        names: ids,
        status: "pending",
        cert: None,
    });
    let id = orders.len() - 1;
    let mut h = nonce_headers();
    h.insert(
        "location",
        format!("{}/orders/{id}", m.base).parse().unwrap(),
    );
    (
        StatusCode::CREATED,
        h,
        Json(order_json(&m, id, &orders[id])),
    )
        .into_response()
}

async fn get_authz(State(m): State<Mock>, Path(id): Path<usize>) -> Response {
    let authz = m.authz.lock();
    let a = &authz[id];
    let mut challenges = vec![json!({
        "type": "dns-01", "url": format!("{}/chall/{id}/dns", m.base), "token": format!("tok{id}"),
        "status": a.status, "error": a.error.as_ref().map(|d| json!({"detail": d})),
    })];
    if !a.wildcard {
        challenges.push(json!({
            "type": "http-01", "url": format!("{}/chall/{id}/http", m.base), "token": format!("tok{id}"),
            "status": a.status,
        }));
    }
    let body = json!({
        "status": a.status,
        "identifier": {"type": "dns", "value": a.domain},
        "challenges": challenges,
    });
    (StatusCode::OK, nonce_headers(), Json(body)).into_response()
}

async fn respond_challenge(
    State(m): State<Mock>,
    Path((id, kind)): Path<(usize, String)>,
) -> Response {
    let mut authz = m.authz.lock();
    let a = &mut authz[id];
    let published = match kind.as_str() {
        "http" => m.http01.get(&format!("tok{id}")).is_some(),
        _ => m
            .dns
            .records
            .lock()
            .contains_key(&dns01_record_name(&a.domain)),
    };
    if m.reject.contains(&a.domain) || !published {
        a.status = "invalid";
        a.error = Some(format!("Invalid response from {}", a.domain));
    } else {
        a.status = "valid";
    }
    (StatusCode::OK, nonce_headers(), Json(json!({}))).into_response()
}

async fn get_order(State(m): State<Mock>, Path(id): Path<usize>) -> Response {
    let authz = m.authz.lock();
    let mut orders = m.orders.lock();
    let o = &mut orders[id];
    if o.status == "pending" {
        if o.authz.iter().any(|a| authz[*a].status == "invalid") {
            o.status = "invalid";
        } else if o.authz.iter().all(|a| authz[*a].status == "valid") {
            o.status = "ready";
        }
    }
    let body = order_json(&m, id, o);
    (StatusCode::OK, nonce_headers(), Json(body)).into_response()
}

async fn finalize(
    State(m): State<Mock>,
    Path(id): Path<usize>,
    Json(body): Json<Value>,
) -> Response {
    let csr_der = URL_SAFE_NO_PAD
        .decode(payload(&body)["csr"].as_str().unwrap())
        .unwrap();
    let csr = rcgen::CertificateSigningRequestParams::from_der(&csr_der.into()).unwrap();
    let issuer = rcgen::Issuer::from_params(&m.ca.0, &m.ca.1);
    let cert = csr.signed_by(&issuer).unwrap();
    let mut orders = m.orders.lock();
    let o = &mut orders[id];
    assert_eq!(o.status, "ready");
    o.status = "valid";
    o.cert = Some(pem::encode(&pem::Pem::new(
        "CERTIFICATE",
        cert.der().to_vec(),
    )));
    let body = order_json(&m, id, o);
    (StatusCode::OK, nonce_headers(), Json(body)).into_response()
}

async fn download(State(m): State<Mock>, Path(id): Path<usize>) -> Response {
    let cert = m.orders.lock()[id].cert.clone().unwrap();
    (StatusCode::OK, nonce_headers(), cert).into_response()
}

struct MockCa {
    url: String,
    mock: Mock,
}

async fn spawn_mock(http01: AcmeChallenges, dns: FakeDns, reject: &[&str]) -> MockCa {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let ca_key = rcgen::KeyPair::generate().unwrap();
    let mut ca_params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    ca_params
        .distinguished_name
        .push(rcgen::DnType::OrganizationName, "Mock CA");
    let mock = Mock {
        base: base.clone(),
        http01,
        dns,
        reject: reject.iter().map(|s| s.to_string()).collect(),
        authz: Default::default(),
        orders: Default::default(),
        ca: Arc::new((ca_params, ca_key)),
    };
    let app = Router::new()
        .route("/dir", get(directory))
        .route("/nonce", get(nonce).head(nonce))
        .route("/account", post(account))
        .route("/order", post(new_order))
        .route("/authz/{id}", post(get_authz))
        .route("/chall/{id}/{kind}", post(respond_challenge))
        .route("/orders/{id}", post(get_order))
        .route("/finalize/{id}", post(finalize))
        .route("/cert/{id}", post(download))
        .with_state(mock.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    MockCa {
        url: format!("{base}/dir"),
        mock,
    }
}

async fn client(dir: &std::path::Path, url: &str, challenges: AcmeChallenges) -> AcmeClient {
    AcmeClient::with_challenges(
        AcmeConfig {
            email: "ops@example.com".into(),
            cache_dir: dir.to_path_buf(),
            staging: true,
            directory_url: Some(url.to_string()),
        },
        challenges,
    )
    .await
    .unwrap()
    .with_poll_interval(Duration::from_millis(5))
}

#[tokio::test]
async fn http01_order_reaches_valid_and_cleans_up_tokens() {
    let dir = tempfile::tempdir().unwrap();
    let challenges = AcmeChallenges::new();
    let ca = spawn_mock(challenges.clone(), FakeDns::default(), &[]).await;
    let acme = client(dir.path(), &ca.url, challenges.clone()).await;

    let result = acme
        .request_certificate_with(&["app.example.com".to_string()], &ChallengeSolver::Http01)
        .await
        .unwrap();
    let loaded =
        super::store::load_pem(&result.certificate_chain_pem, &result.private_key_pem).unwrap();
    assert_eq!(loaded.info.names, vec!["app.example.com"]);
    assert_eq!(loaded.info.issuer, "Mock CA");
    // The HTTP-01 token is withdrawn once the order is done.
    assert!(challenges.get("tok0").is_none());
    assert_eq!(ca.mock.orders.lock()[0].status, "valid");
    assert_eq!(ca.mock.orders.lock()[0].names, vec!["app.example.com"]);
}

#[tokio::test]
async fn dns01_wildcard_order_publishes_and_removes_txt_records() {
    let dir = tempfile::tempdir().unwrap();
    let dns = FakeDns::default();
    let ca = spawn_mock(AcmeChallenges::new(), dns.clone(), &[]).await;
    let acme = client(dir.path(), &ca.url, AcmeChallenges::new()).await;

    let names = vec!["example.com".to_string(), "*.example.com".to_string()];
    let result = acme
        .request_certificate_with(&names, &ChallengeSolver::Dns01(&dns))
        .await
        .unwrap();
    let info = super::store::load_pem(&result.certificate_chain_pem, &result.private_key_pem)
        .unwrap()
        .info;
    assert_eq!(info.names, vec!["example.com", "*.example.com"]);
    assert!(dns.records.lock().is_empty());
    assert_eq!(
        dns.removed.lock().as_slice(),
        ["_acme-challenge.example.com", "_acme-challenge.example.com"]
    );
}

#[tokio::test]
async fn failed_authorization_reports_the_ca_detail() {
    let dir = tempfile::tempdir().unwrap();
    let challenges = AcmeChallenges::new();
    let ca = spawn_mock(challenges.clone(), FakeDns::default(), &["bad.example.com"]).await;
    let acme = client(dir.path(), &ca.url, challenges.clone()).await;
    let err = acme
        .request_certificate_with(&["bad.example.com".to_string()], &ChallengeSolver::Http01)
        .await
        .unwrap_err();
    assert!(
        format!("{err:#}").contains("Invalid response from bad.example.com"),
        "{err:#}"
    );
    assert!(challenges.get("tok0").is_none());
}

#[tokio::test]
async fn manager_issues_per_domain_and_isolates_failures() {
    let data = tempfile::tempdir().unwrap();
    let db = crate::db::init(data.path()).await.unwrap();
    for (id, domain) in [
        ("a1", "good.example.com"),
        ("a2", "bad.example.com"),
        ("a3", "other.example.org"),
    ] {
        sqlx::query(
            "INSERT INTO apps (id, name, git_url, branch, port, domain) \
             VALUES (?, ?, 'https://example.com/r.git', 'main', 3000, ?)",
        )
        .bind(id)
        .bind(id)
        .bind(domain)
        .execute(&db)
        .await
        .unwrap();
    }

    let challenges = AcmeChallenges::new();
    let ca = spawn_mock(challenges.clone(), FakeDns::default(), &["bad.example.com"]).await;
    let acme_dir = data.path().join("acme");
    let acme = client(&acme_dir, &ca.url, challenges.clone()).await;

    let store = Arc::new(CertStore::new().unwrap());
    let runtime = Arc::new(CertRuntime::new(
        store.clone(),
        acme_dir.clone(),
        true,
        None,
    ));
    let mut manager =
        CertManager::new(db.clone(), runtime, None, challenges).with_client(Arc::new(acme));

    let outcomes = manager.tick().await;
    use super::manager::OrderOutcome::*;
    let issued: Vec<_> = outcomes
        .iter()
        .filter_map(|o| match o {
            Issued(d) => Some(d.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(issued, vec!["good.example.com", "other.example.org"]);
    assert!(outcomes
        .iter()
        .any(|o| matches!(o, Failed(d, _) if d == "bad.example.com")));

    // Each good domain has its own certificate in the store and on disk.
    assert!(store.get(CertSource::Acme, "good.example.com").is_some());
    assert!(store.covers("other.example.org"));
    assert!(!store.covers("bad.example.com"));
    assert!(super::domain_dir(&acme_dir, "good.example.com")
        .join("fullchain.pem")
        .exists());

    let bad = super::db::get(&db, "bad.example.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bad.status, "failed");
    assert_eq!(bad.failure_count, 1);
    assert!(bad
        .last_error
        .as_deref()
        .unwrap()
        .contains("Invalid response"));
    assert!(bad.next_attempt().unwrap() > chrono::Utc::now());
    let good = super::db::get(&db, "good.example.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(good.status, "issued");
    assert_eq!(good.issuer.as_deref(), Some("Mock CA"));

    // Nothing is due on the next pass: issued certs are fresh, the failure backs off.
    assert!(manager.tick().await.is_empty());
}
