//! HTTP transport helpers for `VTInfo` finder requests.
//!
//! Handles iframe fetch, search POST, retry/backoff, pacing, and form
//! construction. Separated from `mod.rs` to keep the orchestration layer
//! under the 300-line production-code limit.

pub(super) const BROWSER_FALLBACK_UA: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
pub(super) const VTINFO_MAX_FETCH_ATTEMPTS: usize = 5;
const VTINFO_BACKOFF_BASE_MS: u64 = 500;
const VTINFO_BACKOFF_MAX_MS: u64 = 6_000;
const VTINFO_BRAND_PACING_BASE_MS: u64 = 350;
const VTINFO_BRAND_PACING_SPREAD_MS: u64 = 400;
const VTINFO_GLOBAL_MIN_REQUEST_GAP_MS: u64 = 900;

pub(super) async fn fetch_vtinfo_iframe(
    client: &reqwest::Client,
    user_agents: &[String],
    iframe_url: &str,
    referer: &str,
    timeout_secs: u64,
) -> Option<String> {
    for attempt in 0..VTINFO_MAX_FETCH_ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(vtinfo_retry_backoff_delay(attempt - 1)).await;
        }

        for ua in user_agents {
            wait_for_vtinfo_request_slot().await;
            let Ok(response) = client
                .get(iframe_url)
                .header(reqwest::header::USER_AGENT, ua)
                .header(reqwest::header::REFERER, referer)
                .send()
                .await
            else {
                continue;
            };
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                if let Some(delay) = retry_after_delay(response.headers()) {
                    tokio::time::sleep(delay).await;
                }
                continue;
            }
            if !is_retryable_status(response.status()) {
                continue;
            }
            if let Ok(text) = response.text().await {
                if !text.trim().is_empty() && !is_vtinfo_rate_limited_body(&text) {
                    return Some(text);
                }
                if is_vtinfo_rate_limited_body(&text) {
                    tokio::time::sleep(vtinfo_retry_backoff_delay(attempt)).await;
                }
            }
        }

        // Curl fallback.
        wait_for_vtinfo_request_slot().await;
        let curl_output = tokio::process::Command::new("curl")
            .arg("-Ls")
            .arg("--max-time")
            .arg(timeout_secs.to_string())
            .arg("--user-agent")
            .arg(BROWSER_FALLBACK_UA)
            .arg("-H")
            .arg(format!("Referer: {referer}"))
            .arg(iframe_url)
            .output()
            .await;
        if let Ok(output) = curl_output {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                if !text.trim().is_empty() && !is_vtinfo_rate_limited_body(&text) {
                    return Some(text);
                }
                if is_vtinfo_rate_limited_body(&text) {
                    tokio::time::sleep(vtinfo_retry_backoff_delay(attempt)).await;
                }
            }
        }
    }

    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_vtinfo_form<'a>(
    cust_id: &'a str,
    pagesize: &'a str,
    uuid: Option<&'a str>,
    implementation_id: &'a str,
    csrf_token: &'a str,
    on_prem: &'a str,
    off_prem: &'a str,
    zip: &'a str,
    lat: f64,
    lng: f64,
) -> Vec<(&'a str, String)> {
    let mut form: Vec<(&str, String)> = vec![
        ("custID", cust_id.to_string()),
        ("pagesize", pagesize.to_string()),
        ("implementationID", implementation_id.to_string()),
        ("action", "results".to_string()),
        ("d", zip.to_string()),
        ("z", zip.to_string()),
        ("m", "100".to_string()),
        ("lat", lat.to_string()),
        ("long", lng.to_string()),
        ("themeVersion", "3".to_string()),
        ("onPremDescription", on_prem.to_string()),
        ("offPremDescription", off_prem.to_string()),
        ("CSRFToken", csrf_token.to_string()),
        ("storeType", "on".to_string()),
        ("storeType", "off".to_string()),
    ];
    if let Some(uuid) = uuid.filter(|v| !v.is_empty()) {
        form.push(("UUID", uuid.to_string()));
    }
    // Some deployments include these fields and reject requests missing them.
    form.push(("minResults", String::new()));
    form.push(("minSold", String::new()));
    form
}

pub(super) async fn fetch_vtinfo_search(
    client: &reqwest::Client,
    user_agents: &[String],
    iframe_url: &str,
    form: &[(&str, String)],
    timeout_secs: u64,
) -> Option<String> {
    for attempt in 0..VTINFO_MAX_FETCH_ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(vtinfo_retry_backoff_delay(attempt - 1)).await;
        }

        for ua in user_agents {
            wait_for_vtinfo_request_slot().await;
            let Ok(response) = client
                .post("https://finder.vtinfo.com/finder/web/v2/iframe/search")
                .header(reqwest::header::USER_AGENT, ua)
                .header(reqwest::header::REFERER, iframe_url)
                .form(form)
                .send()
                .await
            else {
                continue;
            };
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                if let Some(delay) = retry_after_delay(response.headers()) {
                    tokio::time::sleep(delay).await;
                }
                continue;
            }
            if !is_retryable_status(response.status()) {
                continue;
            }
            if let Ok(text) = response.text().await {
                if !text.trim().is_empty() && !is_vtinfo_rate_limited_body(&text) {
                    return Some(text);
                }
                if is_vtinfo_rate_limited_body(&text) {
                    tokio::time::sleep(vtinfo_retry_backoff_delay(attempt)).await;
                }
            }
        }

        // Curl fallback.
        let mut command = tokio::process::Command::new("curl");
        wait_for_vtinfo_request_slot().await;
        command
            .arg("-Ls")
            .arg("--max-time")
            .arg(timeout_secs.to_string())
            .arg("--user-agent")
            .arg(BROWSER_FALLBACK_UA)
            .arg("-H")
            .arg(format!("Referer: {iframe_url}"))
            .arg("-H")
            .arg("Content-Type: application/x-www-form-urlencoded; charset=UTF-8")
            .arg("https://finder.vtinfo.com/finder/web/v2/iframe/search");
        for (name, value) in form {
            command
                .arg("--data-urlencode")
                .arg(format!("{name}={value}"));
        }
        let curl_output = command.output().await;
        if let Ok(output) = curl_output {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                if !text.trim().is_empty() && !is_vtinfo_rate_limited_body(&text) {
                    return Some(text);
                }
                if is_vtinfo_rate_limited_body(&text) {
                    tokio::time::sleep(vtinfo_retry_backoff_delay(attempt)).await;
                }
            }
        }
    }

    None
}

pub(super) fn vtinfo_retry_backoff_delay(attempt: usize) -> std::time::Duration {
    let growth = 1_u64 << attempt.min(6);
    let delay_ms = VTINFO_BACKOFF_BASE_MS
        .saturating_mul(growth)
        .min(VTINFO_BACKOFF_MAX_MS);
    std::time::Duration::from_millis(delay_ms)
}

pub(super) fn vtinfo_brand_pacing_delay(
    cust_id: &str,
    request_index: usize,
) -> std::time::Duration {
    let spread = if VTINFO_BRAND_PACING_SPREAD_MS == 0 {
        0
    } else {
        stable_hash(cust_id, request_index) % VTINFO_BRAND_PACING_SPREAD_MS
    };
    std::time::Duration::from_millis(VTINFO_BRAND_PACING_BASE_MS + spread)
}

pub(super) fn retry_after_delay(
    headers: &reqwest::header::HeaderMap,
) -> Option<std::time::Duration> {
    let retry_after = headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim();

    let seconds = retry_after.parse::<u64>().ok()?;
    Some(std::time::Duration::from_secs(seconds.min(10)))
}

fn stable_hash(seed: &str, request_index: usize) -> u64 {
    // FNV-1a 64-bit constants: offset basis and prime.
    const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64 = 1_099_511_628_211;
    let mut hash = FNV_OFFSET;
    for byte in seed.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash ^ (request_index as u64).wrapping_mul(FNV_PRIME)
}

fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    if status.is_server_error() {
        return false;
    }
    status.is_success()
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::REQUEST_TIMEOUT
}

fn is_vtinfo_rate_limited_body(body: &str) -> bool {
    let lowered = body.to_ascii_lowercase();
    lowered.contains("429 too many requests")
        || lowered.contains("you have sent too many requests")
        || lowered.contains("rate limit")
}

async fn wait_for_vtinfo_request_slot() {
    static LAST_REQUEST: std::sync::OnceLock<tokio::sync::Mutex<Option<std::time::Instant>>> =
        std::sync::OnceLock::new();

    let gate = LAST_REQUEST.get_or_init(|| tokio::sync::Mutex::new(None));
    let mut last = gate.lock().await;
    let min_gap = std::time::Duration::from_millis(VTINFO_GLOBAL_MIN_REQUEST_GAP_MS);
    if let Some(previous) = *last {
        let elapsed = previous.elapsed();
        if elapsed < min_gap {
            tokio::time::sleep(min_gap.saturating_sub(elapsed)).await;
        }
    }
    *last = Some(std::time::Instant::now());
}
