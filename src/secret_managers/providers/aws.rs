//! AWS Secrets Manager: `GetSecretValue` and `ListSecrets` over the JSON protocol, signed
//! with SigV4. Credentials are an access key pair or the EC2 instance role (IMDSv2).

use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use super::{
    base_url, http_client, json_object_to_map, non_empty, required, ConnectionConfig,
    ConnectionCredentials, Listing, ListingEntry, ProviderError, SecretData, SecretProvider,
};
use crate::secret_managers::sigv4::{self, AwsCredentials, SignableRequest};

const DEFAULT_IMDS: &str = "http://169.254.169.254";
const SERVICE: &str = "secretsmanager";

enum CredSource {
    Static(AwsCredentials),
    InstanceRole { imds: String },
}

pub struct AwsSecretsManagerProvider {
    client: reqwest::Client,
    region: String,
    endpoint: String,
    source: CredSource,
    instance_creds: Mutex<Option<AwsCredentials>>,
}

impl AwsSecretsManagerProvider {
    pub fn new(
        config: &ConnectionConfig,
        creds: &ConnectionCredentials,
        timeout: Duration,
    ) -> Result<Self, ProviderError> {
        let region = required(&config.region, "AWS region")?.to_string();
        if !region
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(ProviderError::Config(format!(
                "'{}' is not an AWS region",
                region
            )));
        }
        let endpoint = match non_empty(&config.endpoint) {
            Some(e) => base_url(e),
            None => format!("https://secretsmanager.{}.amazonaws.com", region),
        };
        let source = match non_empty(&config.auth_method).unwrap_or("access_key") {
            "access_key" => CredSource::Static(AwsCredentials {
                access_key_id: required(&config.access_key_id, "AWS access key ID")?.to_string(),
                secret_access_key: required(&creds.secret_access_key, "AWS secret access key")?
                    .to_string(),
                session_token: non_empty(&creds.session_token).map(str::to_string),
            }),
            "instance_role" => CredSource::InstanceRole {
                imds: DEFAULT_IMDS.to_string(),
            },
            other => {
                return Err(ProviderError::Config(format!(
                    "unsupported AWS auth method '{}' (use access_key or instance_role)",
                    other
                )))
            }
        };
        Ok(Self {
            client: http_client(timeout),
            region,
            endpoint,
            source,
            instance_creds: Mutex::new(None),
        })
    }

    /// Point the instance-role lookup at another metadata endpoint (tests).
    #[cfg(test)]
    fn with_imds(mut self, imds: &str) -> Self {
        if let CredSource::InstanceRole { imds: i } = &mut self.source {
            *i = imds.to_string();
        }
        self
    }

    async fn credentials(&self) -> Result<AwsCredentials, ProviderError> {
        match &self.source {
            CredSource::Static(c) => Ok(c.clone()),
            CredSource::InstanceRole { imds } => {
                let mut cached = self.instance_creds.lock().await;
                if let Some(c) = cached.as_ref() {
                    return Ok(c.clone());
                }
                let c = imds_credentials(&self.client, imds).await?;
                *cached = Some(c.clone());
                Ok(c)
            }
        }
    }

    async fn call(&self, target: &str, body: &Value) -> Result<Value, ProviderError> {
        let creds = self.credentials().await?;
        let url = reqwest::Url::parse(&format!("{}/", self.endpoint))
            .map_err(|_| ProviderError::Config("invalid AWS endpoint URL".into()))?;
        let host = match (url.host_str(), url.port()) {
            (Some(h), Some(p)) => format!("{}:{}", h, p),
            (Some(h), None) => h.to_string(),
            _ => return Err(ProviderError::Config("AWS endpoint has no host".into())),
        };
        let payload = serde_json::to_vec(body).unwrap_or_default();
        let amz_target = format!("secretsmanager.{}", target);
        let content_type = "application/x-amz-json-1.1";
        let signed = sigv4::sign(
            &creds,
            &self.region,
            SERVICE,
            &sigv4::amz_date_now(),
            &SignableRequest {
                method: "POST",
                host: &host,
                path: "/",
                query: &[],
                headers: &[
                    ("content-type", content_type),
                    ("x-amz-target", &amz_target),
                ],
                payload: &payload,
            },
        );
        let mut req = self
            .client
            .post(url)
            .header("content-type", content_type)
            .header("x-amz-target", &amz_target)
            .body(payload);
        for (k, v) in signed {
            req = req.header(k, v);
        }
        let resp = req.send().await.map_err(ProviderError::from_reqwest)?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if status.is_success() {
            return serde_json::from_str(&text)
                .map_err(|e| ProviderError::Invalid(format!("{} response: {}", target, e)));
        }
        let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        let kind = parsed["__type"]
            .as_str()
            .unwrap_or("")
            .rsplit('#')
            .next()
            .unwrap_or("")
            .to_string();
        let message = parsed["message"]
            .as_str()
            .or_else(|| parsed["Message"].as_str())
            .unwrap_or("")
            .chars()
            .take(200)
            .collect::<String>();
        let msg = format!(
            "{} returned HTTP {} {}: {}",
            target,
            status.as_u16(),
            kind,
            message
        );
        Err(match kind.as_str() {
            "ResourceNotFoundException" => ProviderError::NotFound(msg),
            "AccessDeniedException"
            | "UnrecognizedClientException"
            | "InvalidSignatureException"
            | "ExpiredTokenException"
            | "IncompleteSignature" => ProviderError::Auth(msg),
            _ if status.as_u16() == 401 || status.as_u16() == 403 => ProviderError::Auth(msg),
            _ => ProviderError::Http(msg),
        })
    }
}

/// Temporary credentials of the EC2 instance role through IMDSv2.
async fn imds_credentials(
    client: &reqwest::Client,
    imds: &str,
) -> Result<AwsCredentials, ProviderError> {
    let imds = base_url(imds);
    let fail = |what: &str| {
        ProviderError::Auth(format!(
            "instance role: {} (is Rivetr running on EC2 with an instance profile?)",
            what
        ))
    };
    let token = client
        .put(format!("{}/latest/api/token", imds))
        .header("X-aws-ec2-metadata-token-ttl-seconds", "21600")
        .send()
        .await
        .map_err(|_| fail("instance metadata service unreachable"))?;
    if !token.status().is_success() {
        return Err(fail("metadata token request refused"));
    }
    let token = token.text().await.map_err(|_| fail("no metadata token"))?;
    let base = format!("{}/latest/meta-data/iam/security-credentials/", imds);
    let roles = client
        .get(&base)
        .header("X-aws-ec2-metadata-token", &token)
        .send()
        .await
        .map_err(ProviderError::from_reqwest)?;
    if !roles.status().is_success() {
        return Err(fail("no instance role attached"));
    }
    let roles = roles.text().await.unwrap_or_default();
    let role = roles
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .ok_or_else(|| fail("no instance role attached"))?
        .to_string();
    let creds: Value = client
        .get(format!("{}{}", base, sigv4::uri_encode(&role)))
        .header("X-aws-ec2-metadata-token", &token)
        .send()
        .await
        .map_err(ProviderError::from_reqwest)?
        .json()
        .await
        .map_err(|_| fail("unreadable role credentials"))?;
    match (
        creds["AccessKeyId"].as_str(),
        creds["SecretAccessKey"].as_str(),
    ) {
        (Some(id), Some(secret)) => Ok(AwsCredentials {
            access_key_id: id.to_string(),
            secret_access_key: secret.to_string(),
            session_token: creds["Token"].as_str().map(str::to_string),
        }),
        _ => Err(fail("role credentials missing")),
    }
}

#[async_trait]
impl SecretProvider for AwsSecretsManagerProvider {
    async fn read(&self, path: &str) -> Result<SecretData, ProviderError> {
        let id = path.trim_matches('/');
        if id.is_empty() {
            return Err(ProviderError::Config(
                "an AWS Secrets Manager reference needs a secret name or ARN".into(),
            ));
        }
        let body = self
            .call("GetSecretValue", &json!({ "SecretId": id }))
            .await?;
        let raw = body["SecretString"].as_str().ok_or_else(|| {
            ProviderError::Invalid(format!(
                "secret '{}' has no SecretString (binary secrets are not supported)",
                id
            ))
        })?;
        let values = match serde_json::from_str::<Value>(raw) {
            Ok(Value::Object(obj)) => json_object_to_map(&obj),
            _ => Default::default(),
        };
        Ok(SecretData {
            values,
            raw: Some(raw.to_string()),
        })
    }

    async fn list(&self, path: &str) -> Result<Listing, ProviderError> {
        let id = path.trim_matches('/');
        if !id.is_empty() {
            return Ok(Listing {
                entries: vec![],
                keys: self.read(id).await?.values.into_keys().collect(),
            });
        }
        let mut listing = Listing::default();
        let mut next: Option<String> = None;
        for _ in 0..10 {
            let mut req = json!({ "MaxResults": 100 });
            if let Some(t) = &next {
                req["NextToken"] = json!(t);
            }
            let body = self.call("ListSecrets", &req).await?;
            for s in body["SecretList"].as_array().into_iter().flatten() {
                if let Some(name) = s["Name"].as_str() {
                    listing.entries.push(ListingEntry {
                        name: name.to_string(),
                        path: name.to_string(),
                        kind: "secret",
                    });
                }
            }
            next = body["NextToken"].as_str().map(str::to_string);
            if next.is_none() {
                break;
            }
        }
        Ok(listing)
    }

    async fn test(&self) -> Result<String, ProviderError> {
        match self.call("ListSecrets", &json!({ "MaxResults": 1 })).await {
            Ok(_) => Ok(format!("Authenticated in {}", self.region)),
            // A signed request that is denied proves the credentials work; the policy
            // just does not grant ListSecrets (browsing is then unavailable).
            Err(ProviderError::Auth(m)) if m.contains("AccessDeniedException") => Ok(format!(
                "Authenticated in {} (ListSecrets is not allowed, so browsing is unavailable)",
                self.region
            )),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_server::serve;
    use super::*;
    use axum::{
        http::{HeaderMap, StatusCode},
        routing::{get, post, put},
        Json, Router,
    };

    const T: Duration = Duration::from_secs(5);

    /// Accept requests signed by `AKID` (or the instance role key), checking the scope
    /// and target headers the way AWS routes them.
    fn router() -> Router {
        Router::new().route(
            "/",
            post(|h: HeaderMap, body: String| async move {
                let auth = h
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                assert_eq!(
                    h.get("content-type").unwrap(),
                    "application/x-amz-json-1.1"
                );
                assert!(auth.contains("/eu-west-1/secretsmanager/aws4_request"));
                assert!(auth.contains("SignedHeaders=content-type;host;x-amz-date"));
                if auth.contains("Credential=ROLEKEY/") {
                    assert_eq!(h.get("x-amz-security-token").unwrap(), "role-session");
                } else if !auth.contains("Credential=AKID/") {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"__type": "UnrecognizedClientException", "message": "The security token included in the request is invalid."})),
                    );
                }
                let b: Value = serde_json::from_str(&body).unwrap();
                match h.get("x-amz-target").and_then(|v| v.to_str().ok()) {
                    Some("secretsmanager.GetSecretValue") => match b["SecretId"].as_str() {
                        Some("prod/db") => (
                            StatusCode::OK,
                            Json(json!({"Name": "prod/db", "SecretString": "{\"username\":\"app\",\"password\":\"pw\",\"port\":5432}"})),
                        ),
                        Some("prod/token") => (
                            StatusCode::OK,
                            Json(json!({"Name": "prod/token", "SecretString": "plain-token"})),
                        ),
                        _ => (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"__type": "com.amazonaws.secretsmanager#ResourceNotFoundException", "message": "Secrets Manager can't find the specified secret."})),
                        ),
                    },
                    Some("secretsmanager.ListSecrets") => {
                        if b["NextToken"].is_null() {
                            (
                                StatusCode::OK,
                                Json(json!({"SecretList": [{"Name": "prod/db"}], "NextToken": "p2"})),
                            )
                        } else {
                            (StatusCode::OK, Json(json!({"SecretList": [{"Name": "prod/token"}]})))
                        }
                    }
                    _ => (StatusCode::BAD_REQUEST, Json(json!({"__type": "UnknownOperationException"}))),
                }
            }),
        )
    }

    fn imds_router() -> Router {
        Router::new()
            .route("/latest/api/token", put(|| async { "imds-token" }))
            .route(
                "/latest/meta-data/iam/security-credentials/",
                get(|h: HeaderMap| async move {
                    assert_eq!(h.get("x-aws-ec2-metadata-token").unwrap(), "imds-token");
                    "rivetr-role\n"
                }),
            )
            .route(
                "/latest/meta-data/iam/security-credentials/rivetr-role",
                get(|| async {
                    Json(json!({"AccessKeyId": "ROLEKEY", "SecretAccessKey": "rs", "Token": "role-session"}))
                }),
            )
    }

    fn cfg(endpoint: &str) -> ConnectionConfig {
        ConnectionConfig {
            region: Some("eu-west-1".into()),
            access_key_id: Some("AKID".into()),
            endpoint: Some(endpoint.into()),
            ..Default::default()
        }
    }

    fn creds() -> ConnectionCredentials {
        ConnectionCredentials {
            secret_access_key: Some("secret".into()),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn json_secret_and_plain_secret() {
        let ep = serve(router()).await;
        let p = AwsSecretsManagerProvider::new(&cfg(&ep), &creds(), T).unwrap();
        let d = p.read("prod/db").await.unwrap();
        assert_eq!(d.values["password"], "pw");
        assert_eq!(d.values["port"], "5432");
        let d = p.read("prod/token").await.unwrap();
        assert!(d.values.is_empty());
        assert_eq!(d.raw.as_deref(), Some("plain-token"));
        assert!(matches!(
            p.read("nope").await,
            Err(ProviderError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn list_paginates_and_test_passes() {
        let ep = serve(router()).await;
        let p = AwsSecretsManagerProvider::new(&cfg(&ep), &creds(), T).unwrap();
        let l = p.list("").await.unwrap();
        let names: Vec<_> = l.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["prod/db", "prod/token"]);
        assert_eq!(
            p.list("prod/db").await.unwrap().keys,
            vec!["password", "port", "username"]
        );
        assert!(p.test().await.unwrap().contains("eu-west-1"));
    }

    #[tokio::test]
    async fn wrong_key_is_auth_error() {
        let ep = serve(router()).await;
        let mut c = cfg(&ep);
        c.access_key_id = Some("OTHER".into());
        let p = AwsSecretsManagerProvider::new(&c, &creds(), T).unwrap();
        assert!(matches!(p.test().await, Err(ProviderError::Auth(_))));
    }

    #[tokio::test]
    async fn instance_role_through_imdsv2() {
        let ep = serve(router()).await;
        let imds = serve(imds_router()).await;
        let mut c = cfg(&ep);
        c.auth_method = Some("instance_role".into());
        c.access_key_id = None;
        let p = AwsSecretsManagerProvider::new(&c, &ConnectionCredentials::default(), T)
            .unwrap()
            .with_imds(&imds);
        assert_eq!(p.read("prod/db").await.unwrap().values["username"], "app");
    }

    #[tokio::test]
    async fn instance_role_without_imds_fails_cleanly() {
        let mut c = cfg("http://127.0.0.1:1");
        c.auth_method = Some("instance_role".into());
        let p = AwsSecretsManagerProvider::new(&c, &ConnectionCredentials::default(), T)
            .unwrap()
            .with_imds("http://127.0.0.1:1");
        assert!(matches!(p.read("x").await, Err(ProviderError::Auth(_))));
    }

    #[test]
    fn config_validation() {
        let mut c = cfg("http://x");
        c.region = Some("EU WEST".into());
        assert!(AwsSecretsManagerProvider::new(&c, &creds(), T).is_err());
        assert!(AwsSecretsManagerProvider::new(
            &cfg("http://x"),
            &ConnectionCredentials::default(),
            T
        )
        .is_err());
    }
}
