use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

// -- Shared response types ---------------------------------------------------

#[derive(Serialize)]
pub struct AiUnavailable {
    pub error: &'static str,
}

fn ai_unavailable() -> (StatusCode, Json<AiUnavailable>) {
    (
        StatusCode::NOT_FOUND,
        Json(AiUnavailable {
            error: "AI provider not configured. Add [ai] section to rivetr.toml.",
        }),
    )
}

// -- 1. Deployment Error Diagnosis -------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct DiagnosisResponse {
    pub diagnosis: String,
    pub suggestions: Vec<String>,
}

pub async fn diagnose_deployment(
    State(state): State<Arc<AppState>>,
    Path((app_id, deployment_id)): Path<(String, String)>,
) -> Result<Json<DiagnosisResponse>, (StatusCode, Json<AiUnavailable>)> {
    let ai = match state.ai_client.read().clone() {
        Some(c) => c,
        None => return Err(ai_unavailable()),
    };

    // Fetch the last 80 log lines for this deployment
    let logs: Vec<(String,)> = sqlx::query_as(
        "SELECT message FROM deployment_logs WHERE deployment_id = ? ORDER BY id DESC LIMIT 80",
    )
    .bind(&deployment_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| ai_unavailable())?;

    // Get app + deployment info
    let app_info: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT a.name, a.build_type, d.error_message FROM apps a \
         JOIN deployments d ON d.app_id=a.id WHERE d.id=? AND a.id=?",
    )
    .bind(&deployment_id)
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| ai_unavailable())?;

    let (app_name, build_type, error_message) = app_info.unwrap_or_default();
    let log_text: Vec<String> = logs.into_iter().rev().map(|(m,)| m).collect();
    let log_str = log_text.join("\n");

    let system = "You are an expert DevOps engineer analyzing deployment failures in a PaaS platform. \
        Be concise and actionable. Return a JSON object with keys: \
        \"diagnosis\" (string, 1-2 sentences), \"suggestions\" (array of strings, max 4 items, each <= 20 words).";

    let user = format!(
        "App: {app_name}\nBuild type: {build_type}\nError: {}\n\nLast deployment logs:\n{log_str}",
        error_message.unwrap_or_default()
    );

    let raw = ai.complete(system, &user).await.map_err(|_| ai_unavailable())?;

    // Try to parse as JSON; fall back to wrapping the raw text
    if let Ok(parsed) = serde_json::from_str::<DiagnosisResponse>(&raw) {
        return Ok(Json(parsed));
    }
    // Extract JSON object from response if wrapped in markdown code fences
    let json_str = raw
        .lines()
        .skip_while(|l| !l.trim_start().starts_with('{'))
        .take_while(|l| {
            !l.trim().is_empty()
                || l.trim_start().starts_with('{')
                || l.trim_start().starts_with('"')
                || l.trim_start().starts_with('}')
        })
        .collect::<Vec<_>>()
        .join("\n");
    if let Ok(parsed) = serde_json::from_str::<DiagnosisResponse>(&json_str) {
        return Ok(Json(parsed));
    }
    Ok(Json(DiagnosisResponse {
        diagnosis: raw
            .lines()
            .next()
            .unwrap_or("Unable to parse diagnosis")
            .to_string(),
        suggestions: raw
            .lines()
            .skip(1)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .take(4)
            .collect(),
    }))
}

// -- 2. Deployment Insights --------------------------------------------------

#[derive(Serialize)]
pub struct DeploymentInsights {
    pub summary: String,
    pub avg_build_minutes: f64,
    pub success_rate_percent: f64,
    pub total_deployments: i64,
    /// "improving" | "degrading" | "stable"
    pub trend: String,
}

pub async fn get_deployment_insights(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<DeploymentInsights>, (StatusCode, Json<AiUnavailable>)> {
    let ai = match state.ai_client.read().clone() {
        Some(c) => c,
        None => return Err(ai_unavailable()),
    };

    // Aggregate stats for last 30 days
    let stats: Option<(i64, i64, Option<f64>)> = sqlx::query_as(
        "SELECT \
            COUNT(*) as total, \
            SUM(CASE WHEN status='running' OR status='replaced' THEN 1 ELSE 0 END) as successes, \
            AVG(CASE WHEN finished_at IS NOT NULL AND started_at IS NOT NULL \
                THEN (julianday(finished_at) - julianday(started_at)) * 1440 \
                ELSE NULL END) as avg_minutes \
           FROM deployments \
           WHERE app_id=? AND started_at >= datetime('now', '-30 days')",
    )
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| ai_unavailable())?;

    let (total, successes, avg_mins) = stats.unwrap_or((0, 0, None));
    let success_rate = if total > 0 {
        (successes as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let avg_build_minutes = avg_mins.unwrap_or(0.0);

    // Get app name
    let app_name: Option<(String,)> = sqlx::query_as("SELECT name FROM apps WHERE id=?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| ai_unavailable())?;
    let app_name = app_name.map(|(n,)| n).unwrap_or_default();

    let system = "You are a DevOps insights assistant. Given deployment statistics, write a 1-2 sentence plain-English summary. Be specific and actionable. Do not use markdown.";
    let user = format!(
        "App: {app_name}\nLast 30 days: {total} deployments, {:.0}% success rate, avg build time {:.1} minutes.",
        success_rate, avg_build_minutes
    );

    let summary = ai.complete(system, &user).await.unwrap_or_else(|_| {
        format!(
            "{total} deployments in 30 days with {success_rate:.0}% success rate and {avg_build_minutes:.1}m avg build time."
        )
    });

    let trend = if success_rate >= 95.0 {
        "improving"
    } else if success_rate >= 80.0 {
        "stable"
    } else {
        "degrading"
    }
    .to_string();

    Ok(Json(DeploymentInsights {
        summary,
        avg_build_minutes,
        success_rate_percent: success_rate,
        total_deployments: total,
        trend,
    }))
}

// -- 3. Cost Optimization ----------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct CostSuggestion {
    pub title: String,
    pub description: String,
    pub action: String,
}

#[derive(Serialize)]
pub struct CostSuggestionsResponse {
    pub suggestions: Vec<CostSuggestion>,
}

pub async fn get_cost_suggestions(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<CostSuggestionsResponse>, (StatusCode, Json<AiUnavailable>)> {
    let ai = match state.ai_client.read().clone() {
        Some(c) => c,
        None => return Err(ai_unavailable()),
    };

    let info: Option<(String, Option<String>, Option<String>, i64)> = sqlx::query_as(
        "SELECT name, memory_limit, cpu_limit, replica_count FROM apps WHERE id=?",
    )
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| ai_unavailable())?;

    let (name, memory_limit, cpu_limit, replicas) = info.unwrap_or_default();

    let system = "You are a cloud cost optimization advisor. Given app resource config, return a JSON array of up to 3 suggestions. Each item: {\"title\": string, \"description\": string, \"action\": string (one short sentence)}. Return only the JSON array.";
    let user = format!(
        "App: {name}\nMemory limit: {}\nCPU limit: {}\nReplicas: {replicas}",
        memory_limit.as_deref().unwrap_or("not set"),
        cpu_limit.as_deref().unwrap_or("not set"),
    );

    let raw = ai.complete(system, &user).await.map_err(|_| ai_unavailable())?;

    // Parse JSON array from response
    let json_start = raw.find('[').unwrap_or(0);
    let json_end = raw.rfind(']').map(|i| i + 1).unwrap_or(raw.len());
    let suggestions =
        serde_json::from_str::<Vec<CostSuggestion>>(&raw[json_start..json_end]).unwrap_or_default();

    Ok(Json(CostSuggestionsResponse { suggestions }))
}

// -- 4. Dockerfile Optimizer -------------------------------------------------

#[derive(Serialize)]
pub struct DockerfileOptimization {
    pub original: String,
    pub suggested: String,
    pub improvements: Vec<String>,
}

pub async fn suggest_dockerfile(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<DockerfileOptimization>, (StatusCode, Json<AiUnavailable>)> {
    let ai = match state.ai_client.read().clone() {
        Some(c) => c,
        None => return Err(ai_unavailable()),
    };

    let info: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT name, dockerfile, build_type FROM apps WHERE id=?",
    )
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| ai_unavailable())?;

    let (app_name, dockerfile, build_type) = info.unwrap_or_default();
    let dockerfile = dockerfile.unwrap_or_default();

    if dockerfile.trim().is_empty() {
        return Err(ai_unavailable());
    }

    let system = "You are a Docker expert. Analyze the provided Dockerfile and return a JSON object with: \
        {\"suggested\": \"<optimized Dockerfile as a string>\", \"improvements\": [\"<improvement 1>\", ...]}. \
        Focus on: layer caching (copy manifests before source), multi-stage builds, removing unnecessary deps, \
        .dockerignore usage, security (non-root user). Return ONLY the JSON object, no markdown fences.";

    let user = format!(
        "App: {app_name}\nBuild type: {}\n\nDockerfile:\n{dockerfile}",
        build_type.as_deref().unwrap_or("dockerfile")
    );

    let raw = ai.complete(system, &user).await.map_err(|_| ai_unavailable())?;

    #[derive(Deserialize)]
    struct AiResp {
        suggested: String,
        improvements: Vec<String>,
    }

    let json_start = raw.find('{').unwrap_or(0);
    let json_end = raw.rfind('}').map(|i| i + 1).unwrap_or(raw.len());
    let parsed = serde_json::from_str::<AiResp>(&raw[json_start..json_end])
        .map_err(|_| ai_unavailable())?;

    Ok(Json(DockerfileOptimization {
        original: dockerfile,
        suggested: parsed.suggested,
        improvements: parsed.improvements,
    }))
}

// -- 5. Security & Compliance Advisor ----------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Serialize)]
pub struct SecurityFinding {
    pub severity: Severity,
    pub category: String,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

#[derive(Serialize)]
pub struct SecurityScanResponse {
    pub app_id: String,
    pub app_name: String,
    pub findings: Vec<SecurityFinding>,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub ai_summary: Option<String>,
}

pub async fn scan_app_security(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<SecurityScanResponse>, StatusCode> {
    // Rule-based scan -- works without AI
    type AppSecurityInfo = (String, Option<String>, Option<String>, Option<String>, i64);
    let info: Option<AppSecurityInfo> = sqlx::query_as(
        "SELECT name, docker_image, domain, healthcheck, replica_count FROM apps WHERE id=?",
    )
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (app_name, docker_image, domain, healthcheck, _replicas) = info.unwrap_or_default();

    let mut findings: Vec<SecurityFinding> = Vec::new();

    // Check 1: untagged/latest image
    if let Some(ref img) = docker_image {
        if img.ends_with(":latest") || !img.contains(':') {
            findings.push(SecurityFinding {
                severity: Severity::Medium,
                category: "image_pinning".into(),
                title: "Docker image uses :latest tag".into(),
                description: format!(
                    "Image '{img}' uses :latest which is mutable and can change unexpectedly."
                ),
                recommendation:
                    "Pin to a specific version tag (e.g. :1.2.3) for reproducible deployments."
                        .into(),
            });
        }
    }

    // Check 2: no health check configured
    if healthcheck.as_deref().unwrap_or("").is_empty() {
        findings.push(SecurityFinding {
            severity: Severity::Low,
            category: "availability".into(),
            title: "No health check configured".into(),
            description:
                "App has no health check URL. Unhealthy containers may receive traffic.".into(),
            recommendation: "Set a health check path (e.g. /health or /up) in app Settings."
                .into(),
        });
    }

    // Check 3: custom domain without HTTPS hint
    if let Some(ref d) = domain {
        if !d.is_empty() && d.starts_with("http://") {
            findings.push(SecurityFinding {
                severity: Severity::High,
                category: "transport_security".into(),
                title: "Custom domain uses HTTP".into(),
                description: format!("Domain '{d}' is configured with plain HTTP."),
                recommendation: "Use an HTTPS domain and enable TLS/SSL termination.".into(),
            });
        }
    }

    // Check 4: scan last 200 log lines for common secret patterns
    let logs: Vec<(String,)> = sqlx::query_as(
        "SELECT message FROM deployment_logs dl \
         JOIN deployments d ON dl.deployment_id=d.id \
         WHERE d.app_id=? ORDER BY dl.id DESC LIMIT 200",
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let secret_patterns = [
        ("AWS", r"AKIA[0-9A-Z]{16}"),
        ("Private key", r"-----BEGIN .{0,20}PRIVATE KEY-----"),
        (
            "Generic secret",
            r"(?i)(password|secret|token|api_key)\s*[=:]\s*['\x22]?[A-Za-z0-9+/]{16,}",
        ),
    ];
    for (label, pattern) in &secret_patterns {
        let re = regex::Regex::new(pattern);
        if let Ok(re) = re {
            if logs.iter().any(|(msg,)| re.is_match(msg)) {
                findings.push(SecurityFinding {
                    severity: Severity::Critical,
                    category: "exposed_secret".into(),
                    title: format!("{label} pattern found in deployment logs"),
                    description: format!(
                        "A {label} credential pattern was detected in deployment logs. \
                         Credentials in logs can be harvested from log storage."
                    ),
                    recommendation: "Rotate the credential immediately. Use environment variables, \
                        not inline config, for secrets. Ensure logs are redacted before storage."
                        .into(),
                });
                break;
            }
        }
    }

    // Check 5: env vars with secret-sounding names but empty values
    let env_vars: Vec<(String, String)> =
        sqlx::query_as("SELECT key, value FROM env_vars WHERE app_id=?")
            .bind(&app_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    let secret_key_patterns = [
        "password",
        "secret",
        "token",
        "api_key",
        "private_key",
        "access_key",
    ];
    for (key, value) in &env_vars {
        let key_lower = key.to_lowercase();
        if secret_key_patterns
            .iter()
            .any(|p| key_lower.contains(p))
            && value.is_empty()
        {
            findings.push(SecurityFinding {
                severity: Severity::Medium,
                category: "missing_secret".into(),
                title: format!("Secret env var '{}' is empty", key),
                description: format!(
                    "The env var '{}' appears to be a secret but has no value set.",
                    key
                ),
                recommendation: "Set a value for this secret or remove it if unused.".into(),
            });
            break; // only report once
        }
    }

    // AI summary (optional) — clone out of the lock before any await
    let maybe_ai = state.ai_client.read().clone();
    let ai_summary = if let Some(ai) = maybe_ai {
        let finding_list = findings
            .iter()
            .map(|f| format!("[{:?}] {} - {}", f.severity, f.title, f.recommendation))
            .collect::<Vec<_>>()
            .join("\n");
        let system = "You are a security advisor. Summarize the following security findings for a \
            developer in 2-3 sentences. Be direct and prioritize the most critical action. No markdown.";
        let user = format!("App: {app_name}\n\nFindings:\n{finding_list}");
        ai.complete(system, &user).await.ok()
    } else {
        None
    };

    let critical = findings
        .iter()
        .filter(|f| matches!(f.severity, Severity::Critical))
        .count();
    let high = findings
        .iter()
        .filter(|f| matches!(f.severity, Severity::High))
        .count();
    let medium = findings
        .iter()
        .filter(|f| matches!(f.severity, Severity::Medium))
        .count();
    let low = findings
        .iter()
        .filter(|f| matches!(f.severity, Severity::Low))
        .count();

    Ok(Json(SecurityScanResponse {
        app_id,
        app_name,
        findings,
        critical,
        high,
        medium,
        low,
        ai_summary,
    }))
}

/// Global security scan across all apps
pub async fn scan_all_security(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let apps: Vec<(String,)> = sqlx::query_as("SELECT id FROM apps")
        .fetch_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut results = Vec::new();
    for (app_id,) in apps {
        if let Ok(Json(scan)) = scan_app_security(State(state.clone()), Path(app_id)).await {
            results.push(serde_json::to_value(scan).unwrap_or_default());
        }
    }
    Ok(Json(results))
}
