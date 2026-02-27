use std::{
    collections::HashMap,
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

/// Marker for the authenticated token, used to key rate limits.
#[derive(Debug, Clone)]
struct AuthenticatedToken(String);

/// API key auth settings used by middleware.
#[derive(Debug, Clone)]
pub struct AuthState {
    api_keys: Arc<Vec<String>>,
    pub enabled: bool,
}

#[allow(dead_code)]
impl AuthState {
    /// Builds auth config from `SCBDB_API_KEYS` (comma-separated bearer tokens).
    ///
    /// In development, empty/missing keys disable auth for local iteration.
    /// In non-development envs, empty/missing keys fail startup.
    pub fn from_env(is_development: bool) -> anyhow::Result<Self> {
        let raw = std::env::var("SCBDB_API_KEYS").unwrap_or_default();
        let keys: Vec<String> = raw
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
                    api_keys: Arc::new(Vec::new()),
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

    /// Timing-safe key comparison.
    ///
    /// Iterates ALL stored keys using `.fold()` with constant-time byte
    /// comparison so that neither short-circuit nor variable-length comparison
    /// leaks which key (or whether any key) matched.
    fn allows(&self, token: &str) -> bool {
        self.api_keys.iter().fold(false, |acc, stored| {
            acc | ct_eq(token.as_bytes(), stored.as_bytes())
        })
    }
}

/// Constant-time byte-slice equality for same-length byte slices.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let diff = a
        .iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y));
    diff == 0
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RateLimitWindow {
    started_at: Instant,
    count: usize,
}

/// Sliding fixed-window limiter per client.
#[derive(Debug, Clone)]
pub struct RateLimitState {
    max_requests: usize,
    window: Duration,
    state: Arc<Mutex<HashMap<String, RateLimitWindow>>>,
}

#[allow(dead_code)]
impl RateLimitState {
    #[must_use]
    pub fn new(max_requests: usize, window: Duration) -> Self {
        let state = Self {
            max_requests,
            window,
            state: Arc::new(Mutex::new(HashMap::new())),
        };
        state.spawn_cleanup_task();
        state
    }

    /// Starts periodic cleanup when a Tokio runtime is active.
    fn spawn_cleanup_task(&self) {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };

        let state = self.clone();
        let cleanup_interval = self.window.max(Duration::from_secs(30));

        handle.spawn(async move {
            loop {
                tokio::time::sleep(cleanup_interval).await;
                state.cleanup().await;
            }
        });
    }

    /// Evicts entries whose window has fully elapsed.
    ///
    /// Should be called periodically (e.g. from a background task) to prevent
    /// the state map from growing without bound.
    pub async fn cleanup(&self) {
        let mut state = self.state.lock().await;
        state.retain(|_, w| w.started_at.elapsed() < self.window);
    }
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct MiddlewareErrorBody {
    error: MiddlewareError,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct MiddlewareError {
    code: &'static str,
    message: &'static str,
}

// MiddlewareErrorBody intentionally does NOT implement IntoResponse — callers
// construct `(StatusCode, Json(body))` tuples directly so each call site can
// specify the correct status code (UNAUTHORIZED for auth, TOO_MANY_REQUESTS
// for rate limiting, etc.).

/// Maximum length for a client-supplied request ID.
const MAX_REQUEST_ID_LEN: usize = 128;

/// Returns `true` if `id` is a valid request ID: non-empty, at most
/// [`MAX_REQUEST_ID_LEN`] characters, and contains only alphanumeric
/// characters, hyphens, or underscores.
fn is_valid_request_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_REQUEST_ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Axum middleware that extracts or generates a request ID.
///
/// If the incoming request has a valid `x-request-id` header, that value is
/// used. Invalid IDs (wrong characters, too long) are silently replaced with
/// a fresh `UUIDv4`. The ID is:
/// - Inserted into request extensions as [`RequestId`]
/// - Set on the response as the `x-request-id` header
pub async fn request_id(mut req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| is_valid_request_id(v))
        .map_or_else(|| Uuid::new_v4().to_string(), String::from);

    req.extensions_mut().insert(RequestId(id.clone()));

    let mut res = next.run(req).await;

    if let Ok(val) = HeaderValue::from_str(&id) {
        res.headers_mut().insert("x-request-id", val);
    }

    res
}

/// Middleware enforcing Bearer token auth when enabled.
#[allow(dead_code)]
pub async fn require_bearer_auth(
    State(auth): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Response {
    if !auth.enabled {
        return next.run(req).await;
    }

    let token = extract_bearer_token(req.headers().get(AUTHORIZATION)).map(ToOwned::to_owned);

    match token {
        Some(token) if auth.allows(&token) => {
            req.extensions_mut().insert(AuthenticatedToken(token));
            next.run(req).await
        }
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

/// Middleware enforcing a request-per-window limit per client.
///
/// If authenticated, uses the bearer token as a key.
/// Otherwise falls back to the `X-Forwarded-For` header or a global default.
#[allow(dead_code)]
pub async fn enforce_rate_limit(
    State(rate_limit): State<RateLimitState>,
    req: Request,
    next: Next,
) -> Response {
    let client_key = if let Some(token) = req.extensions().get::<AuthenticatedToken>() {
        token.0.clone()
    } else {
        req.headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map_or_else(|| "global".to_string(), |ip| ip.trim().to_string())
    };

    let mut state = rate_limit.state.lock().await;
    let window = state
        .entry(client_key.clone())
        .or_insert_with(|| RateLimitWindow {
            started_at: Instant::now(),
            count: 0,
        });

    let elapsed = window.started_at.elapsed();

    if elapsed >= rate_limit.window {
        window.started_at = Instant::now();
        window.count = 0;
    }

    if window.count >= rate_limit.max_requests {
        tracing::warn!(client = %client_key, "rate limit exceeded");
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
    drop(state);

    next.run(req).await
}

/// Extract the token from a `Bearer <token>` Authorization header.
///
/// The "Bearer" scheme is case-insensitive per RFC 6750 §1.1, so we compare
/// the first 7 characters in a case-insensitive manner.
#[allow(dead_code)]
fn extract_bearer_token(value: Option<&HeaderValue>) -> Option<&str> {
    value
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            if v.len() > 7 && v[..7].eq_ignore_ascii_case("bearer ") {
                Some(&v[7..])
            } else {
                None
            }
        })
        .filter(|s| !s.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_bearer_token ────────────────────────────────────────────

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
    fn extract_bearer_token_is_case_insensitive() {
        let lower = HeaderValue::from_static("bearer my-token");
        assert_eq!(extract_bearer_token(Some(&lower)), Some("my-token"));

        let upper = HeaderValue::from_static("BEARER my-token");
        assert_eq!(extract_bearer_token(Some(&upper)), Some("my-token"));

        let mixed = HeaderValue::from_static("bEaReR my-token");
        assert_eq!(extract_bearer_token(Some(&mixed)), Some("my-token"));
    }

    #[test]
    fn extract_bearer_token_rejects_empty_token() {
        let header = HeaderValue::from_static("Bearer ");
        assert_eq!(extract_bearer_token(Some(&header)), None);

        let header_spaces = HeaderValue::from_static("Bearer   ");
        assert_eq!(extract_bearer_token(Some(&header_spaces)), None);
    }

    #[test]
    fn extract_bearer_token_rejects_none() {
        assert_eq!(extract_bearer_token(None), None);
    }

    // ── AuthState ───────────────────────────────────────────────────────

    /// Helper to construct an AuthState directly (avoids process-global env var mutation).
    fn auth_with_keys(keys: &[&str]) -> AuthState {
        AuthState {
            api_keys: Arc::new(keys.iter().map(|s| (*s).to_string()).collect()),
            enabled: !keys.is_empty(),
        }
    }

    #[test]
    fn auth_state_disables_when_no_keys_in_dev() {
        let state = auth_with_keys(&[]);
        assert!(!state.enabled);
    }

    #[test]
    fn test_auth_allows_valid_key() {
        let state = auth_with_keys(&["alpha-key", "bravo-key"]);
        assert!(state.enabled);
        assert!(state.allows("alpha-key"));
        assert!(state.allows("bravo-key"));
    }

    #[test]
    fn test_auth_rejects_invalid_key() {
        let state = auth_with_keys(&["correct-key"]);
        assert!(!state.allows("wrong-key"));
        assert!(!state.allows(""));
        assert!(!state.allows("correct-ke")); // prefix
        assert!(!state.allows("correct-keys")); // suffix
    }

    #[test]
    fn test_auth_disabled_when_no_keys() {
        let state = auth_with_keys(&[]);
        assert!(!state.enabled);
        // Even calling allows on a disabled state should return false
        assert!(!state.allows("anything"));
    }

    // ── Request ID validation ───────────────────────────────────────────

    #[test]
    fn test_request_id_validation_accepts_valid() {
        // Alphanumeric
        assert!(is_valid_request_id("abc123"));
        // Hyphens
        assert!(is_valid_request_id("req-id-42"));
        // Underscores
        assert!(is_valid_request_id("req_id_42"));
        // Mixed
        assert!(is_valid_request_id("a1-b2_c3"));
        // UUID format
        assert!(is_valid_request_id("550e8400-e29b-41d4-a716-446655440000"));
        // Exactly 128 chars
        let max_len = "a".repeat(MAX_REQUEST_ID_LEN);
        assert!(is_valid_request_id(&max_len));
    }

    #[test]
    fn test_request_id_validation_rejects_invalid() {
        // Empty
        assert!(!is_valid_request_id(""));
        // Over 128 chars
        let too_long = "a".repeat(MAX_REQUEST_ID_LEN + 1);
        assert!(!is_valid_request_id(&too_long));
        // Special characters
        assert!(!is_valid_request_id("id with spaces"));
        assert!(!is_valid_request_id("id;drop table"));
        assert!(!is_valid_request_id("id\nwith\nnewlines"));
        assert!(!is_valid_request_id("<script>alert(1)</script>"));
        assert!(!is_valid_request_id("id/../../etc/passwd"));
        assert!(!is_valid_request_id("id@domain.com"));
    }

    // ── Constant-time equality ──────────────────────────────────────────

    #[test]
    fn ct_eq_matches_equal_slices() {
        assert!(ct_eq(b"hello", b"hello"));
        assert!(ct_eq(b"", b""));
    }

    #[test]
    fn ct_eq_rejects_unequal_slices() {
        assert!(!ct_eq(b"hello", b"world"));
        assert!(!ct_eq(b"hello", b"hell"));
        assert!(!ct_eq(b"a", b"b"));
    }

    // ── Rate limiter eviction ───────────────────────────────────────────

    #[tokio::test]
    async fn test_rate_limiter_eviction() {
        // Use a tiny window so entries expire immediately.
        let limiter = RateLimitState::new(100, Duration::from_millis(1));

        // Insert >1000 entries directly into the state map.
        {
            let mut state = limiter.state.lock().await;
            for i in 0..1_100 {
                state.insert(
                    format!("client-{i}"),
                    RateLimitWindow {
                        // Already expired: started 10 seconds ago with a 1ms window.
                        started_at: Instant::now().checked_sub(Duration::from_secs(10)).unwrap(),
                        count: 1,
                    },
                );
            }
            assert_eq!(state.len(), 1_100);
        }

        // Cleanup should evict all expired entries.
        limiter.cleanup().await;

        let state = limiter.state.lock().await;
        assert_eq!(state.len(), 0, "all expired entries should be pruned");
    }

    #[tokio::test]
    async fn test_rate_limiter_eviction_preserves_active() {
        let limiter = RateLimitState::new(100, Duration::from_secs(60));

        {
            let mut state = limiter.state.lock().await;
            // One active entry
            state.insert(
                "active-client".to_string(),
                RateLimitWindow {
                    started_at: Instant::now(),
                    count: 5,
                },
            );
            // Several expired entries
            for i in 0..50 {
                state.insert(
                    format!("expired-{i}"),
                    RateLimitWindow {
                        started_at: Instant::now()
                            .checked_sub(Duration::from_secs(120))
                            .unwrap(),
                        count: 1,
                    },
                );
            }
            assert_eq!(state.len(), 51);
        }

        limiter.cleanup().await;

        let state = limiter.state.lock().await;
        assert_eq!(state.len(), 1, "only the active entry should remain");
        assert!(state.contains_key("active-client"));
    }
}
