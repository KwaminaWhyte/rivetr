//! AI-generated Docker Compose from a natural-language prompt.
//!
//! `POST /api/ai/compose/generate` asks the configured AI provider for a
//! compose file (optionally modifying an existing one) and validates the
//! result with the same checks used when a service is created. It never
//! creates or deploys anything: the caller reviews the YAML and decides.

use std::future::Future;
use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::api::error::{ApiError, ErrorCode};
use crate::api::services::compose::validate_compose_content;
use crate::AppState;

/// Longest prompt we forward to the provider.
const MAX_PROMPT_CHARS: usize = 4_000;
/// Longest existing compose file we forward to the provider.
const MAX_EXISTING_COMPOSE_CHARS: usize = 60_000;

const SYSTEM_PROMPT: &str = "You write Docker Compose files for Rivetr, a self-hosted PaaS. \
Rules: \
1. Output a single JSON object and nothing else, with keys \"compose\" (the complete docker-compose YAML as a string) \
and \"explanation\" (2 to 4 short sentences on what the stack contains and anything the user must set). \
2. Use official, well-maintained images with a pinned major version tag, never `latest`. \
3. Do not set container_name. Use named volumes for persistent data. \
4. Never hardcode secrets. Use Rivetr magic variables instead: ${SERVICE_PASSWORD_<NAME>} for passwords, \
${SERVICE_BASE64_<NAME>} for random keys, ${SERVICE_USER_<NAME>} for usernames, ${SERVICE_FQDN_<NAME>} and \
${SERVICE_URL_<NAME>} for the public domain of a service. \
5. Add a healthcheck to every long-running service where the image supports one. \
6. Use depends_on with condition: service_healthy for databases. \
7. Expose only the main web port with `ports` or `expose`; databases stay internal. \
8. If an existing compose file is given, modify it as requested and return the full updated file.";

#[derive(Debug, Deserialize)]
pub struct GenerateComposeRequest {
    pub prompt: String,
    /// Optional existing compose file to modify instead of starting fresh.
    #[serde(default)]
    pub existing_compose: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct GenerateComposeResponse {
    pub compose: String,
    pub explanation: String,
    /// Service names found in the generated compose file.
    pub services: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AiStatusResponse {
    pub configured: bool,
}

/// `GET /api/ai/status`: whether an AI provider is configured.
pub async fn ai_status(State(state): State<Arc<AppState>>) -> Json<AiStatusResponse> {
    Json(AiStatusResponse {
        configured: state.ai_client.read().is_some(),
    })
}

/// `POST /api/ai/compose/generate`
pub async fn generate_compose(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GenerateComposeRequest>,
) -> Result<Json<GenerateComposeResponse>, ApiError> {
    let ai = state.ai_client.read().clone().ok_or_else(|| {
        ApiError::service_unavailable(
            "No AI provider is configured. Add one in Settings > AI provider.",
        )
    })?;

    let resp = generate_with(&req, |system, user| async move {
        ai.complete(&system, &user).await
    })
    .await?;
    Ok(Json(resp))
}

/// Core logic, generic over the completion call so tests can mock the provider.
pub async fn generate_with<F, Fut>(
    req: &GenerateComposeRequest,
    complete: F,
) -> Result<GenerateComposeResponse, ApiError>
where
    F: FnOnce(String, String) -> Fut,
    Fut: Future<Output = anyhow::Result<String>>,
{
    let prompt = req.prompt.trim();
    if prompt.is_empty() {
        return Err(ApiError::validation_field("prompt", "Describe what to run"));
    }
    if prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(ApiError::validation_field(
            "prompt",
            format!("Prompt is longer than {MAX_PROMPT_CHARS} characters"),
        ));
    }
    let existing = req
        .existing_compose
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(existing) = existing {
        if existing.len() > MAX_EXISTING_COMPOSE_CHARS {
            return Err(ApiError::validation_field(
                "existing_compose",
                "Existing compose file is too large",
            ));
        }
    }

    let user = build_user_prompt(prompt, existing);
    let raw = complete(SYSTEM_PROMPT.to_string(), user)
        .await
        .map_err(|e| {
            tracing::warn!("AI compose generation failed: {e:#}");
            ApiError::new(
                ErrorCode::ExternalServiceError,
                "The AI provider request failed. Check the provider settings and try again.",
            )
        })?;

    let (compose, explanation) = parse_ai_output(&raw).ok_or_else(|| {
        ApiError::new(
            ErrorCode::ExternalServiceError,
            "The AI provider returned something that is not a compose file. Try rephrasing.",
        )
    })?;

    validate_compose_content(&compose).map_err(|e| {
        ApiError::new(
            ErrorCode::UnprocessableEntity,
            format!("Generated compose failed validation: {e}"),
        )
    })?;

    let services = compose_service_names(&compose);
    if services.is_empty() {
        return Err(ApiError::new(
            ErrorCode::UnprocessableEntity,
            "Generated compose failed validation: 'services' is empty",
        ));
    }

    Ok(GenerateComposeResponse {
        compose,
        explanation,
        services,
    })
}

fn build_user_prompt(prompt: &str, existing: Option<&str>) -> String {
    match existing {
        Some(existing) => format!(
            "Modify this docker-compose file as requested.\n\nRequest:\n{prompt}\n\nCurrent docker-compose.yml:\n{existing}"
        ),
        None => format!("Write a docker-compose file for:\n{prompt}"),
    }
}

/// Extract `(compose, explanation)` from the model output.
///
/// Accepts the requested JSON object (optionally wrapped in a code fence) and
/// falls back to a bare ```yaml fenced block or raw YAML.
fn parse_ai_output(raw: &str) -> Option<(String, String)> {
    #[derive(Deserialize)]
    struct AiResp {
        compose: String,
        #[serde(default)]
        explanation: String,
    }

    let trimmed = raw.trim();
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if start < end {
            if let Ok(parsed) = serde_json::from_str::<AiResp>(&trimmed[start..=end]) {
                let compose = strip_fence(&parsed.compose);
                if !compose.is_empty() {
                    return Some((compose, parsed.explanation.trim().to_string()));
                }
            }
        }
    }

    // Fallback: a fenced YAML block, or the whole response as YAML.
    let compose = strip_fence(trimmed);
    if compose.contains("services:") {
        return Some((compose, String::new()));
    }
    None
}

/// Remove a surrounding markdown code fence (```yaml ... ```), if any.
fn strip_fence(s: &str) -> String {
    let s = s.trim();
    if let Some(start) = s.find("```") {
        let after = &s[start + 3..];
        let body_start = after.find('\n').map(|i| i + 1).unwrap_or(0);
        let body = &after[body_start..];
        let body = match body.find("```") {
            Some(end) => &body[..end],
            None => body,
        };
        return body.trim().to_string() + "\n";
    }
    s.to_string() + "\n"
}

fn compose_service_names(compose: &str) -> Vec<String> {
    serde_yaml::from_str::<serde_yaml::Value>(compose)
        .ok()
        .and_then(|v| v.get("services").cloned())
        .and_then(|s| s.as_mapping().cloned())
        .map(|m| {
            m.keys()
                .filter_map(|k| k.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn req(prompt: &str, existing: Option<&str>) -> GenerateComposeRequest {
        GenerateComposeRequest {
            prompt: prompt.to_string(),
            existing_compose: existing.map(str::to_string),
        }
    }

    const GOOD_YAML: &str =
        "services:\n  web:\n    image: nginx:1.27\n  db:\n    image: postgres:17\n";

    #[tokio::test]
    async fn returns_compose_from_json_response() {
        let body = serde_json::json!({
            "compose": GOOD_YAML,
            "explanation": "Nginx in front of Postgres."
        })
        .to_string();
        let out = generate_with(
            &req("nginx and postgres", None),
            |_, _| async move { Ok(body) },
        )
        .await
        .unwrap();
        assert_eq!(out.services, vec!["web".to_string(), "db".to_string()]);
        assert_eq!(out.explanation, "Nginx in front of Postgres.");
        assert!(out.compose.contains("postgres:17"));
    }

    #[tokio::test]
    async fn accepts_fenced_json_and_fenced_yaml() {
        let body = format!(
            "Here you go:\n```json\n{}\n```",
            serde_json::json!({"compose": format!("```yaml\n{GOOD_YAML}```"), "explanation": "x"})
        );
        let out = generate_with(&req("x", None), |_, _| async move { Ok(body) })
            .await
            .unwrap();
        assert!(out.compose.starts_with("services:"));

        let yaml_only = format!("```yaml\n{GOOD_YAML}```");
        let out = generate_with(&req("x", None), |_, _| async move { Ok(yaml_only) })
            .await
            .unwrap();
        assert_eq!(out.services.len(), 2);
        assert!(out.explanation.is_empty());
    }

    #[tokio::test]
    async fn rejects_invalid_compose() {
        let body = serde_json::json!({"compose": "version: '3'\nfoo: bar\n", "explanation": ""})
            .to_string();
        let err = generate_with(&req("x", None), |_, _| async move { Ok(body) })
            .await
            .unwrap_err();
        assert_eq!(err.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);

        let err = generate_with(&req("x", None), |_, _| async move {
            Ok("I cannot help with that.".to_string())
        })
        .await
        .unwrap_err();
        assert_eq!(err.status(), axum::http::StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn provider_error_maps_to_bad_gateway() {
        let err = generate_with(&req("x", None), |_, _| async move {
            Err(anyhow::anyhow!("401 from provider"))
        })
        .await
        .unwrap_err();
        assert_eq!(err.status(), axum::http::StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn empty_prompt_is_rejected_without_calling_provider() {
        let called = Mutex::new(false);
        let err = generate_with(&req("   ", None), |_, _| {
            *called.lock().unwrap() = true;
            async move { Ok(String::new()) }
        })
        .await
        .unwrap_err();
        assert_eq!(err.status(), axum::http::StatusCode::BAD_REQUEST);
        assert!(!*called.lock().unwrap());
    }

    #[tokio::test]
    async fn existing_compose_is_sent_to_provider() {
        let seen = Mutex::new(String::new());
        let body = serde_json::json!({"compose": GOOD_YAML, "explanation": "added db"}).to_string();
        generate_with(
            &req(
                "add postgres",
                Some("services:\n  web:\n    image: nginx:1.27\n"),
            ),
            |system, user| {
                assert!(system.contains("SERVICE_PASSWORD_"));
                *seen.lock().unwrap() = user;
                async move { Ok(body) }
            },
        )
        .await
        .unwrap();
        let user = seen.lock().unwrap().clone();
        assert!(user.contains("Modify this docker-compose file"));
        assert!(user.contains("image: nginx:1.27"));
        assert!(user.contains("add postgres"));
    }
}
