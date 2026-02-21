//! Low-level HTTP helpers for the store locator pipeline.

use super::types::LocatorError;

const BROWSER_FALLBACK_UA: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// Fetch the HTML body of a URL, trying the supplied user-agent first and
/// then the browser fallback UA.  Returns the first successful body.
pub(crate) async fn fetch_html(
    url: &str,
    timeout_secs: u64,
    user_agent: &str,
) -> Result<String, LocatorError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()?;

    let mut user_agents = vec![user_agent.to_string()];
    if user_agent != BROWSER_FALLBACK_UA {
        user_agents.push(BROWSER_FALLBACK_UA.to_string());
    }

    for ua in user_agents {
        let response = client
            .get(url)
            .header(reqwest::header::USER_AGENT, &ua)
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
            .send()
            .await?;
        if response.status().is_success() {
            return Ok(response.text().await?);
        }
    }

    // If every attempt returned a non-2xx, return empty string so callers
    // fall through to the "no parseable locator" path rather than erroring.
    Ok(String::new())
}

/// Perform a simple GET and parse the body as JSON.
pub(crate) async fn fetch_json(
    url: &str,
    timeout_secs: u64,
    user_agent: &str,
) -> Result<serde_json::Value, LocatorError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()?;
    let value = client
        .get(url)
        .header(reqwest::header::USER_AGENT, user_agent)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    Ok(value)
}
