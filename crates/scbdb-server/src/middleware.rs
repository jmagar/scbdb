use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Newtype wrapping a request ID string, stored as a request extension.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

/// API key auth settings used by middleware.
#[derive(Debug, Clone)]
pub struct AuthState {
    api_keys: Arc<HashSet<String>>,
    pub enabled: bool,
}

impl AuthState {
    /// Builds auth config from `SCBDB_API_KEYS` (comma-separated bearer tokens).
    ///
    /// In development, empty/missing keys disable auth for local iteration.
    /// In non-development envs, empty/missing keys fail startup.
    pub fn from_env(is_development: bool) -> anyhow::Result<Self> {
        let raw = std::env::var("SCBDB_API_KEYS").unwrap_or_default();
        let keys: HashSet<String> = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect();

        if keys.is_empty() {
            if is_development {
                tracing::warn!(
                    "SCBDB_API_KEYS not set; bearer auth disabled in development environment"
                );
                return Ok(Self {
                    api_keys: Arc::new(HashSet::new()),
                    enabled: false,
                });
            }

            anyhow::bail!(
                "SCBDB_API_KEYS is required outside development; provide comma-separated bearer tokens"
            );
        }

        Ok(Self {
            api_keys: Arc::new(keys),
            enabled: true,
        })
    }

    fn allows(&self, token: &str) -> bool {
        self.api_keys.contains(token)
    }
}

#[derive(Debug, Clone)]
struct RateLimitWindow {
    started_at: Instant,
    count: usize,
}

/// Sliding fixed-window limiter for simple API protection.
#[derive(Debug, Clone)]
pub struct RateLimitState {
    max_requests: usize,
    window: Duration,
    state: Arc<Mutex<RateLimitWindow>>,
}

impl RateLimitState {
    #[must_use]
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            state: Arc::new(Mutex::new(RateLimitWindow {
                started_at: Instant::now(),
                count: 0,
            })),
        }
    }
}

#[derive(Debug, Serialize)]
struct MiddlewareErrorBody {
    error: MiddlewareError,
}

#[derive(Debug, Serialize)]
struct MiddlewareError {
    code: &'static str,
    message: &'static str,
}

impl IntoResponse for MiddlewareErrorBody {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, Json(self)).into_response()
    }
}

/// Axum middleware that extracts or generates a request ID.
///
/// If the incoming request has an `x-request-id` header, that value is used.
/// Otherwise a new `UUIDv4` is generated. The ID is:
/// - Inserted into request extensions as [`RequestId`]
/// - Set on the response as the `x-request-id` header
pub async fn request_id(mut req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map_or_else(|| Uuid::new_v4().to_string(), String::from);

    req.extensions_mut().insert(RequestId(id.clone()));

    let mut res = next.run(req).await;

    if let Ok(val) = HeaderValue::from_str(&id) {
        res.headers_mut().insert("x-request-id", val);
    }

    res
}

/// Middleware enforcing Bearer token auth when enabled.
pub async fn require_bearer_auth(
    State(auth): State<AuthState>,
    req: Request,
    next: Next,
) -> Response {
    if !auth.enabled {
        return next.run(req).await;
    }

    let token = extract_bearer_token(req.headers().get(AUTHORIZATION));

    match token {
        Some(token) if auth.allows(token) => next.run(req).await,
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(MiddlewareErrorBody {
                error: MiddlewareError {
                    code: "unauthorized",
                    message: "missing or invalid bearer token",
                },
            }),
        )
            .into_response(),
    }
}

/// Middleware enforcing a fixed request-per-window limit.
pub async fn enforce_rate_limit(
    State(rate_limit): State<RateLimitState>,
    req: Request,
    next: Next,
) -> Response {
    let mut window = rate_limit.state.lock().await;
    let elapsed = window.started_at.elapsed();

    if elapsed >= rate_limit.window {
        window.started_at = Instant::now();
        window.count = 0;
    }

    if window.count >= rate_limit.max_requests {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(MiddlewareErrorBody {
                error: MiddlewareError {
                    code: "rate_limited",
                    message: "rate limit exceeded",
                },
            }),
        )
            .into_response();
    }

    window.count += 1;
    drop(window);

    next.run(req).await
}

fn extract_bearer_token(value: Option<&HeaderValue>) -> Option<&str> {
    value
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|s| !s.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_bearer_token_accepts_valid_header() {
        let header = HeaderValue::from_static("Bearer test-token");
        assert_eq!(extract_bearer_token(Some(&header)), Some("test-token"));
    }

    #[test]
    fn extract_bearer_token_rejects_non_bearer_header() {
        let header = HeaderValue::from_static("Basic abc123");
        assert_eq!(extract_bearer_token(Some(&header)), None);
    }

    #[test]
    fn auth_state_disables_when_no_keys_in_dev() {
        std::env::remove_var("SCBDB_API_KEYS");
        let state = AuthState::from_env(true).expect("dev should allow missing keys");
        assert!(!state.enabled);
    }
}
