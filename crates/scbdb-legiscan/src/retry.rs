//! Retry with exponential back-off and jitter for the `LegiScan` client.
//!
//! [`retry_with_backoff`] wraps any fallible async operation and retries on
//! transient errors (network failures, 5xx). Non-transient errors —
//! including [`LegiscanError::QuotaExceeded`] and [`LegiscanError::BudgetExceeded`]
//! — are returned immediately without any retry to protect the monthly API budget.

use std::future::Future;
use std::time::Duration;

use crate::error::LegiscanError;

/// Returns `true` for errors that are worth retrying after a back-off delay.
///
/// **Retriable:**
/// - Network-level failures: timeout, connection reset.
/// - HTTP 5xx responses: transient server/infrastructure errors.
///
/// **Not retriable (hard stop):**
/// - [`LegiscanError::BudgetExceeded`] — per-session cap; stop immediately.
/// - [`LegiscanError::QuotaExceeded`] — API monthly/daily quota; stop immediately.
/// - [`LegiscanError::ApiError`] — application-level error; retrying won't fix it.
/// - [`LegiscanError::Deserialize`] — malformed response; retrying won't fix it.
pub(crate) fn is_retriable(err: &LegiscanError) -> bool {
    match err {
        LegiscanError::Http(e) => {
            e.is_timeout() || e.is_connect() || e.status().is_some_and(|s| s.is_server_error())
        }
        LegiscanError::BudgetExceeded { .. }
        | LegiscanError::QuotaExceeded(_)
        | LegiscanError::ApiError(_)
        | LegiscanError::Deserialize { .. } => false,
    }
}

/// Runs `operation` with up to `max_retries` additional attempts on transient errors.
///
/// Back-off schedule with `backoff_base_ms = 1_000`:
///
/// | Attempt | Sleep before next attempt        |
/// |---------|----------------------------------|
/// | 1       | 1 000 ms × 2⁰ ± 25 % jitter     |
/// | 2       | 1 000 ms × 2¹ ± 25 % jitter     |
/// | 3       | 1 000 ms × 2² ± 25 % jitter     |
///
/// Delay is capped at 60 s. Non-retriable errors are returned immediately.
pub(crate) async fn retry_with_backoff<T, F, Fut>(
    max_retries: u32,
    backoff_base_ms: u64,
    mut operation: F,
) -> Result<T, LegiscanError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, LegiscanError>>,
{
    const MAX_DELAY_MS: u64 = 60_000;
    let mut attempt = 0u32;
    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                if !is_retriable(&err) || attempt >= max_retries {
                    return Err(err);
                }
                attempt += 1;
                let computed = backoff_base_ms.saturating_mul(1u64 << (attempt - 1).min(10));
                let capped = computed.min(MAX_DELAY_MS);
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    clippy::cast_precision_loss
                )]
                let delay_ms = (capped as f64 * (rand::random::<f64>() * 0.5 + 0.75)) as u64;
                tracing::warn!(
                    attempt,
                    max_retries,
                    delay_ms,
                    error = %err,
                    "LegiScan transient error — retrying after back-off"
                );
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deserialize_err() -> LegiscanError {
        let src = serde_json::from_str::<()>("invalid").unwrap_err();
        LegiscanError::Deserialize {
            context: "test".to_owned(),
            source: src,
        }
    }

    #[test]
    fn quota_exceeded_is_not_retriable() {
        assert!(!is_retriable(&LegiscanError::QuotaExceeded(
            "limit".to_owned()
        )));
    }

    #[test]
    fn budget_exceeded_is_not_retriable() {
        assert!(!is_retriable(&LegiscanError::BudgetExceeded {
            used: 300,
            limit: 300
        }));
    }

    #[test]
    fn api_error_is_not_retriable() {
        assert!(!is_retriable(&LegiscanError::ApiError("bad".to_owned())));
    }

    #[test]
    fn deserialize_error_is_not_retriable() {
        assert!(!is_retriable(&deserialize_err()));
    }

    #[tokio::test]
    async fn succeeds_immediately_on_first_try() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        let calls = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&calls);
        let result = retry_with_backoff(3, 0, || {
            let c = Arc::clone(&c);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<u32, LegiscanError>(42)
            }
        })
        .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn does_not_retry_quota_exceeded() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        let calls = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&calls);
        let result = retry_with_backoff(3, 0, || {
            let c = Arc::clone(&c);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err::<u32, _>(LegiscanError::QuotaExceeded("Daily Limit".to_owned()))
            }
        })
        .await;
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "QuotaExceeded must not be retried"
        );
        assert!(matches!(result, Err(LegiscanError::QuotaExceeded(_))));
    }

    #[tokio::test]
    async fn retries_then_succeeds() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        let calls = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&calls);
        let result = retry_with_backoff(3, 0, || {
            let c = Arc::clone(&c);
            async move {
                let attempt = c.fetch_add(1, Ordering::SeqCst) + 1;
                if attempt < 3 {
                    // Simulate a retriable HTTP connect error
                    let resp = reqwest::Client::new()
                        .get("http://0.0.0.0:1")
                        .send()
                        .await
                        .unwrap_err();
                    Err::<u32, _>(LegiscanError::Http(resp))
                } else {
                    Ok(99)
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), 99, "should succeed after retries");
        assert_eq!(
            calls.load(Ordering::SeqCst),
            3,
            "should have been called 3 times (2 failures + 1 success)"
        );
    }

    #[tokio::test]
    async fn does_not_retry_budget_exceeded() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        let calls = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&calls);
        let result = retry_with_backoff(3, 0, || {
            let c = Arc::clone(&c);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err::<u32, _>(LegiscanError::BudgetExceeded {
                    used: 300,
                    limit: 300,
                })
            }
        })
        .await;
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "BudgetExceeded must not be retried"
        );
        assert!(matches!(result, Err(LegiscanError::BudgetExceeded { .. })));
    }
}
