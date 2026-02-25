//! Low-level HTTP helpers for the store locator pipeline.

use super::types::LocatorError;

const BROWSER_FALLBACK_UA: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
const HTML_FETCH_ATTEMPTS: usize = 3;
const HTML_FETCH_BACKOFF_MS: [u64; 3] = [0, 300, 900];

/// Fetch the HTML body of a URL, trying the supplied user-agent first and
/// then the browser fallback UA.  Returns the first successful body.
pub(crate) async fn fetch_html(
    client: &reqwest::Client,
    url: &str,
    timeout_secs: u64,
    user_agent: &str,
) -> Result<String, LocatorError> {
    for attempt in 0..HTML_FETCH_ATTEMPTS {
        if let Some(delay_ms) = HTML_FETCH_BACKOFF_MS.get(attempt).copied() {
            if delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }
        // Prefer curl for storefront HTML: some anti-bot stacks block reqwest
        // while allowing curl/browser fingerprints.
        let curl_output = tokio::process::Command::new("curl")
            .arg("-Lsf")
            .arg("--proto")
            .arg("=https,http")
            .arg("--max-filesize")
            .arg("10485760")
            .arg("--max-time")
            .arg(timeout_secs.to_string())
            .arg("--user-agent")
            .arg(BROWSER_FALLBACK_UA)
            .arg(url)
            .output()
            .await;

        if let Ok(output) = curl_output {
            if output.status.success() {
                let body = String::from_utf8_lossy(&output.stdout).to_string();
                if is_usable_html(&body) {
                    return Ok(body);
                }
            }
        }

        let mut user_agents = vec![user_agent.to_string()];
        if user_agent != BROWSER_FALLBACK_UA {
            user_agents.push(BROWSER_FALLBACK_UA.to_string());
        }

        let mut custom_ua_body: Option<String> = None;
        let mut last_error: Option<reqwest::Error> = None;

        for ua in user_agents {
            let response = match client
                .get(url)
                .header(reqwest::header::USER_AGENT, &ua)
                .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(err) => {
                    last_error = Some(err);
                    continue;
                }
            };

            if response.status().is_success() {
                let body = response.text().await?;
                if !is_usable_html(&body) {
                    continue;
                }
                if ua == BROWSER_FALLBACK_UA {
                    // Prefer browser-like content when available; many locator pages
                    // hide embeds for bot user-agents.
                    return Ok(body);
                }
                custom_ua_body = Some(body);
            }
        }

        if let Some(body) = custom_ua_body {
            return Ok(body);
        }
        if let Some(err) = last_error {
            tracing::debug!(url, attempt, error = %err, "reqwest fetch_html failed; retrying");
        }
    }

    // Every attempt returned non-2xx or unusable HTML — surface the failure
    // so callers can distinguish "page unreachable" from "no locator found".
    Err(LocatorError::AllAttemptsFailed {
        url: url.to_owned(),
    })
}

fn is_usable_html(body: &str) -> bool {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return false;
    }
    if contains_locator_hint(trimmed) {
        return true;
    }
    !looks_like_bot_challenge(trimmed)
}

fn looks_like_bot_challenge(body: &str) -> bool {
    let lowered = body.to_ascii_lowercase();
    let has_cloudflare_banner = lowered.contains("attention required! | cloudflare");
    let has_challenge_platform = lowered.contains("/cdn-cgi/challenge-platform/");
    let has_just_a_moment = lowered.contains("just a moment...");
    let has_cookie_gate = lowered.contains("please enable cookies");
    let has_cf_chl = lowered.contains("cf-chl-");

    has_cloudflare_banner
        || has_challenge_platform
        || (has_just_a_moment && has_cookie_gate)
        || (has_just_a_moment && has_cf_chl)
}

fn contains_locator_hint(body: &str) -> bool {
    let lowered = body.to_ascii_lowercase();
    let markers = [
        "hoodieembedwtbv2(",
        "askhoodie.com",
        "finder.vtinfo.com",
        "storemapper",
        "stockist",
        "storepoint",
        "beveragefinder",
        "locally.com",
    ];
    markers.iter().any(|m| lowered.contains(m))
}

/// Fetch a plain-text resource body.
pub(crate) async fn fetch_text(
    client: &reqwest::Client,
    url: &str,
    user_agent: &str,
) -> Result<String, LocatorError> {
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, user_agent)
        .send()
        .await?
        .error_for_status()?;

    Ok(response.text().await?)
}

/// Perform a simple GET and parse the body as JSON.
pub(crate) async fn fetch_json(
    client: &reqwest::Client,
    url: &str,
    user_agent: &str,
) -> Result<serde_json::Value, LocatorError> {
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, user_agent)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(LocatorError::HttpStatus {
            status: response.status().as_u16(),
            url: url.to_owned(),
        });
    }
    let value = response.json::<serde_json::Value>().await?;
    Ok(value)
}
