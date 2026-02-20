//! Rate limiting and retry utilities for the Shopify scraper.
//!
//! Provides exponential backoff retry logic for transient HTTP errors such as
//! 429 Rate Limited responses. Non-retriable errors (parse failures, 404s,
//! normalization errors) are propagated immediately without retrying.

use std::future::Future;
use std::time::Duration;

use crate::error::ScraperError;

/// Returns `true` if `err` represents a transient condition that should be
/// retried after a backoff delay.
///
/// Retriable errors:
/// - [`ScraperError::RateLimited`] — HTTP 429; the server has asked us to back off.
/// - [`ScraperError::Http`] — network-level failure (connection reset, timeout, etc.).
/// - [`ScraperError::UnexpectedStatus`] with status >= 500 — transient server errors
///   (502/503/504 are common from CDNs like Shopify's).
///
/// Non-retriable errors (propagated immediately):
/// - [`ScraperError::NotFound`] — 404; retrying would return the same result.
/// - [`ScraperError::UnexpectedStatus`] with status < 500 — client errors (e.g., 403).
/// - [`ScraperError::Deserialize`] — response body does not parse; retrying won't fix it.
/// - [`ScraperError::Normalization`] — data shape issue; retrying won't fix it.
/// - [`ScraperError::PaginationLimit`] — guard against infinite loops; not a transient error.
fn is_retriable(err: &ScraperError) -> bool {
    match err {
        ScraperError::RateLimited { .. } | ScraperError::Http(_) => true,
        ScraperError::UnexpectedStatus { status, .. } => *status >= 500,
        _ => false,
    }
}

/// Executes `operation` with exponential backoff retries on transient errors.
///
/// On success the result is returned immediately.
///
/// On a retriable error ([`ScraperError::RateLimited`], [`ScraperError::Http`],
/// or [`ScraperError::UnexpectedStatus`] with status >= 500), the function sleeps
/// for `backoff_base_secs * 2^attempt` seconds and tries again, up to `max_retries`
/// additional attempts after the first try. If all retries are exhausted the last
/// error is returned.
///
/// For [`ScraperError::RateLimited`] errors, the delay is
/// `max(computed_backoff, retry_after_secs)` so we never retry sooner than the
/// server requested.
///
/// Non-retriable errors are returned immediately without sleeping or retrying.
///
/// # Backoff schedule (example with `backoff_base_secs = 1`)
///
/// | Attempt | Sleep before next attempt |
/// |---------|--------------------------|
/// | 0 (initial) | — (no sleep before first try) |
/// | 1 (first retry) | 1 × 2^0 = 1 s |
/// | 2 (second retry) | 1 × 2^1 = 2 s |
/// | 3 (third retry) | 1 × 2^2 = 4 s |
///
/// With `max_retries = 3` the operation is attempted at most 4 times total.
pub(crate) async fn retry_with_backoff<T, F, Fut>(
    max_retries: u32,
    backoff_base_secs: u64,
    mut operation: F,
) -> Result<T, ScraperError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, ScraperError>>,
{
    let mut attempt = 0u32;

    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                if !is_retriable(&err) || attempt >= max_retries {
                    return Err(err);
                }

                // Exponential backoff: base * 2^attempt seconds, with ±25% jitter.
                // For rate-limited responses, honour the server-supplied Retry-After
                // value as a floor so we never retry sooner than the server asked.
                let computed = backoff_base_secs.saturating_mul(1u64 << attempt.min(62));
                let base_delay = if let ScraperError::RateLimited {
                    retry_after_secs, ..
                } = &err
                {
                    computed.max(*retry_after_secs)
                } else {
                    computed
                };

                // Apply ±25% jitter to spread out retries and avoid thundering herd.
                let jitter_factor = rand::random::<f64>() * 0.5 + 0.75; // [0.75, 1.25)
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    clippy::cast_precision_loss
                )]
                let delay_secs = (base_delay as f64 * jitter_factor) as u64;

                tracing::warn!(
                    attempt,
                    max_retries,
                    delay_secs,
                    error = %err,
                    "transient scraper error — retrying after backoff"
                );
                tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                attempt += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    /// Helper: make a RateLimited error with a given retry_after value.
    fn rate_limited(retry_after_secs: u64) -> ScraperError {
        ScraperError::RateLimited {
            domain: "test.example.com".to_owned(),
            retry_after_secs,
        }
    }

    #[tokio::test]
    async fn succeeds_immediately_on_first_try() {
        let call_count = Arc::new(AtomicU32::new(0));
        let cc = Arc::clone(&call_count);
        let result = retry_with_backoff(3, 0, || {
            let cc = Arc::clone(&cc);
            async move {
                cc.fetch_add(1, Ordering::SeqCst);
                Ok::<u32, ScraperError>(42)
            }
        })
        .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retries_on_rate_limited_then_succeeds() {
        let call_count = Arc::new(AtomicU32::new(0));
        let cc = Arc::clone(&call_count);
        let result = retry_with_backoff(3, 0, || {
            let cc = Arc::clone(&cc);
            async move {
                let n = cc.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(rate_limited(0))
                } else {
                    Ok::<u32, ScraperError>(99)
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), 99);
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn propagates_last_error_after_exhausting_retries() {
        let call_count = Arc::new(AtomicU32::new(0));
        let cc = Arc::clone(&call_count);
        let result = retry_with_backoff(2, 0, || {
            let cc = Arc::clone(&cc);
            async move {
                cc.fetch_add(1, Ordering::SeqCst);
                Err::<u32, ScraperError>(rate_limited(0))
            }
        })
        .await;
        // max_retries=2 → 3 total attempts
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
        assert!(matches!(result, Err(ScraperError::RateLimited { .. })));
    }

    #[tokio::test]
    async fn does_not_retry_non_retriable_error() {
        let call_count = Arc::new(AtomicU32::new(0));
        let cc = Arc::clone(&call_count);
        let result = retry_with_backoff(3, 0, || {
            let cc = Arc::clone(&cc);
            async move {
                cc.fetch_add(1, Ordering::SeqCst);
                Err::<u32, ScraperError>(ScraperError::NotFound {
                    url: "https://example.com/products.json".to_owned(),
                })
            }
        })
        .await;
        // Should have tried exactly once — no retries for NotFound.
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        assert!(matches!(result, Err(ScraperError::NotFound { .. })));
    }

    #[tokio::test]
    async fn does_not_retry_deserialize_error() {
        let call_count = Arc::new(AtomicU32::new(0));
        let cc = Arc::clone(&call_count);
        let result = retry_with_backoff(3, 0, || {
            let cc = Arc::clone(&cc);
            async move {
                cc.fetch_add(1, Ordering::SeqCst);
                let e = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
                Err::<u32, ScraperError>(ScraperError::Deserialize {
                    context: "test".to_owned(),
                    source: e,
                })
            }
        })
        .await;
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        assert!(matches!(result, Err(ScraperError::Deserialize { .. })));
    }

    #[tokio::test]
    async fn retries_on_5xx_unexpected_status_then_succeeds() {
        let call_count = Arc::new(AtomicU32::new(0));
        let cc = Arc::clone(&call_count);
        let result = retry_with_backoff(3, 0, || {
            let cc = Arc::clone(&cc);
            async move {
                let n = cc.fetch_add(1, Ordering::SeqCst);
                if n < 1 {
                    Err(ScraperError::UnexpectedStatus {
                        status: 503,
                        url: "https://example.com/products.json".to_owned(),
                    })
                } else {
                    Ok::<u32, ScraperError>(7)
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), 7);
        // 1 failure + 1 success = 2 calls
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn does_not_retry_4xx_unexpected_status() {
        let call_count = Arc::new(AtomicU32::new(0));
        let cc = Arc::clone(&call_count);
        let result = retry_with_backoff(3, 0, || {
            let cc = Arc::clone(&cc);
            async move {
                cc.fetch_add(1, Ordering::SeqCst);
                Err::<u32, ScraperError>(ScraperError::UnexpectedStatus {
                    status: 403,
                    url: "https://example.com/products.json".to_owned(),
                })
            }
        })
        .await;
        // 403 is not retriable — exactly 1 attempt.
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        assert!(matches!(
            result,
            Err(ScraperError::UnexpectedStatus { status: 403, .. })
        ));
    }
}
