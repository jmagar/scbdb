//! `VTInfo` finder iframe extraction.

mod vtinfo_http;
mod vtinfo_parse;

use crate::locator::types::{LocatorError, RawStoreLocation};
use vtinfo_http::{
    build_vtinfo_form, fetch_vtinfo_iframe, fetch_vtinfo_search, vtinfo_brand_pacing_delay,
    BROWSER_FALLBACK_UA,
};
use vtinfo_parse::{
    extract_hidden_input_value, extract_js_string_assignment, parse_vtinfo_search_results,
    vtinfo_dedup_key,
};

/// Parameters needed to query `VTInfo` finder search.
#[derive(Debug, Clone)]
pub(in crate::locator) struct VtinfoEmbed {
    pub cust_id: String,
    pub uuid: Option<String>,
}

pub(in crate::locator) fn extract_vtinfo_embed(html: &str) -> Option<VtinfoEmbed> {
    if !html.contains("finder.vtinfo.com") {
        return None;
    }

    let normalized = html
        .replace("&amp;", "&")
        .replace("\\\\/", "/")
        .replace("\\/", "/")
        .replace('\n', "");
    let re = regex::Regex::new(r#"finder\.vtinfo\.com/finder/web/v2/iframe\?([^\"'\s>]+)"#)
        .expect("valid regex");
    let caps = re.captures(&normalized)?;
    let query = caps.get(1)?.as_str();

    let mut cust_id: Option<String> = None;
    let mut uuid: Option<String> = None;
    for part in query.split('&') {
        let mut kv = part.splitn(2, '=');
        let key = kv.next().unwrap_or_default();
        let value = kv.next().unwrap_or_default();
        match key {
            "custID" if !value.is_empty() => cust_id = Some(value.to_string()),
            "UUID" if !value.is_empty() => uuid = Some(value.to_string()),
            _ => {}
        }
    }

    Some(VtinfoEmbed {
        cust_id: cust_id?,
        uuid,
    })
}

pub(in crate::locator) async fn fetch_vtinfo_stores(
    embed: &VtinfoEmbed,
    locator_url: &str,
    timeout_secs: u64,
    user_agent: &str,
) -> Result<Vec<RawStoreLocation>, LocatorError> {
    let iframe_url = match &embed.uuid {
        Some(uuid) => format!(
            "https://finder.vtinfo.com/finder/web/v2/iframe?custID={}&UUID={uuid}",
            embed.cust_id
        ),
        None => format!(
            "https://finder.vtinfo.com/finder/web/v2/iframe?custID={}",
            embed.cust_id
        ),
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()?;

    let user_agents = if user_agent == BROWSER_FALLBACK_UA {
        vec![BROWSER_FALLBACK_UA.to_string()]
    } else {
        vec![user_agent.to_string(), BROWSER_FALLBACK_UA.to_string()]
    };

    let Some(iframe_html) = fetch_vtinfo_iframe(
        &client,
        &user_agents,
        &iframe_url,
        locator_url,
        timeout_secs,
    )
    .await
    else {
        return Ok(vec![]);
    };

    let pagesize =
        extract_hidden_input_value(&iframe_html, "pagesize").unwrap_or_else(|| "50".to_string());
    let implementation_id =
        extract_hidden_input_value(&iframe_html, "implementationID").unwrap_or_default();
    let resolved_uuid = extract_hidden_input_value(&iframe_html, "UUID")
        .or_else(|| embed.uuid.clone())
        .unwrap_or_default();
    let csrf_token = extract_js_string_assignment(&iframe_html, "CSRFToken").unwrap_or_default();
    let on_prem = extract_js_string_assignment(&iframe_html, "onPremDescription")
        .unwrap_or_else(|| "Restaurants and Bars".to_string());
    let off_prem = extract_js_string_assignment(&iframe_html, "offPremDescription")
        .unwrap_or_else(|| "Retail Stores".to_string());

    tracing::debug!(
        cust_id = embed.cust_id,
        uuid = resolved_uuid,
        pagesize,
        implementation_id,
        "vtinfo iframe parsed"
    );

    let search_points = [
        (44.9778_f64, -93.2650_f64, "55401"),
        (39.8283, -98.5795, "67202"),
        (34.0522, -118.2437, "90001"),
        (40.7128, -74.0060, "10001"),
        (41.8781, -87.6298, "60601"),
        (29.7604, -95.3698, "77001"),
        (39.7392, -104.9903, "80202"),
        (33.4484, -112.0740, "85001"),
    ];

    let mut dedup: std::collections::HashMap<String, RawStoreLocation> =
        std::collections::HashMap::new();

    for (request_index, (lat, lng, zip)) in search_points.into_iter().enumerate() {
        let pacing_delay = vtinfo_brand_pacing_delay(&embed.cust_id, request_index);
        tokio::time::sleep(pacing_delay).await;

        run_vtinfo_search_point(
            &client,
            &user_agents,
            &iframe_url,
            &embed.cust_id,
            &pagesize,
            if resolved_uuid.is_empty() {
                None
            } else {
                Some(resolved_uuid.as_str())
            },
            &implementation_id,
            &csrf_token,
            &on_prem,
            &off_prem,
            zip,
            lat,
            lng,
            timeout_secs,
            &mut dedup,
        )
        .await;

        if dedup.len() >= 100 {
            break;
        }
    }

    Ok(dedup.into_values().collect())
}

#[allow(clippy::too_many_arguments)]
async fn run_vtinfo_search_point(
    client: &reqwest::Client,
    user_agents: &[String],
    iframe_url: &str,
    cust_id: &str,
    pagesize: &str,
    uuid: Option<&str>,
    implementation_id: &str,
    csrf_token: &str,
    on_prem: &str,
    off_prem: &str,
    zip: &str,
    lat: f64,
    lng: f64,
    timeout_secs: u64,
    dedup: &mut std::collections::HashMap<String, RawStoreLocation>,
) {
    let form = build_vtinfo_form(
        cust_id,
        pagesize,
        uuid,
        implementation_id,
        csrf_token,
        on_prem,
        off_prem,
        zip,
        lat,
        lng,
    );

    let Some(html) =
        fetch_vtinfo_search(client, user_agents, iframe_url, &form, timeout_secs).await
    else {
        tracing::debug!(cust_id, zip, "vtinfo search request failed");
        return;
    };

    if html.contains("Invalid token") {
        tracing::debug!(cust_id, zip, "vtinfo invalid token response");
    }
    for location in parse_vtinfo_search_results(&html) {
        let key = vtinfo_dedup_key(&location);
        dedup.entry(key).or_insert(location);
    }
    tracing::debug!(
        cust_id,
        zip,
        count = dedup.len(),
        "vtinfo cumulative location count"
    );
}

#[cfg(test)]
mod tests {
    use super::extract_vtinfo_embed;
    use super::vtinfo_http::{
        build_vtinfo_form, retry_after_delay, vtinfo_brand_pacing_delay, vtinfo_retry_backoff_delay,
    };
    use super::vtinfo_parse::parse_vtinfo_search_results;

    #[test]
    fn extracts_vtinfo_embed_params_from_iframe_src() {
        let html = r#"<iframe src="https://finder.vtinfo.com/finder/web/v2/iframe?custID=S4V&UUID=OkBikYm1nil0ofVzncjAXRHK4lut8bWuxjbq"></iframe>"#;
        let embed = extract_vtinfo_embed(html).expect("embed should parse");
        assert_eq!(embed.cust_id, "S4V");
        assert_eq!(
            embed.uuid.as_deref(),
            Some("OkBikYm1nil0ofVzncjAXRHK4lut8bWuxjbq")
        );
    }

    #[test]
    fn extracts_vtinfo_embed_without_uuid() {
        let html = r#"<script>const url = "https:\/\/finder.vtinfo.com\/finder\/web\/v2\/iframe?custID=S4V";</script>"#;
        let embed = extract_vtinfo_embed(html).expect("embed should parse");
        assert_eq!(embed.cust_id, "S4V");
        assert_eq!(embed.uuid, None);
    }

    #[test]
    fn excludes_uuid_from_form_when_absent() {
        let form = build_vtinfo_form(
            "S4V", "50", None, "impl", "csrf", "on", "off", "10001", 40.0, -73.0,
        );
        assert!(form.iter().all(|(name, _)| *name != "UUID"));
    }

    #[test]
    fn parses_locations_from_vtinfo_search_html() {
        let html = r#"
<article class="card finder_location" data-latitude="44.98" data-longitude="-93.26">
  <h2 class="finder_dba_text">FOWLING WAREHOUSE</h2>
  <a class="finder_address"><span>401 ROYALSTON AVE</span>, <span class="finder_address_city">MINNEAPOLIS</span>, <span class="finder_address_state">MN</span></a>
  <a href="tel:6129463695"><span>(612) 946-3695</span></a>
</article>
"#;
        let rows = parse_vtinfo_search_results(html);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "FOWLING WAREHOUSE");
        assert_eq!(rows[0].city.as_deref(), Some("MINNEAPOLIS"));
        assert_eq!(rows[0].state.as_deref(), Some("MN"));
        assert_eq!(rows[0].latitude, Some(44.98));
        assert_eq!(rows[0].longitude, Some(-93.26));
    }

    #[test]
    fn retry_backoff_is_bounded_and_exponential() {
        assert_eq!(vtinfo_retry_backoff_delay(0).as_millis(), 250);
        assert_eq!(vtinfo_retry_backoff_delay(1).as_millis(), 500);
        assert_eq!(vtinfo_retry_backoff_delay(2).as_millis(), 1_000);
        assert_eq!(vtinfo_retry_backoff_delay(3).as_millis(), 2_000);
        assert_eq!(vtinfo_retry_backoff_delay(10).as_millis(), 2_000);
    }

    #[test]
    fn brand_pacing_is_deterministic_and_bounded() {
        let a = vtinfo_brand_pacing_delay("S4V", 2);
        let b = vtinfo_brand_pacing_delay("S4V", 2);
        assert_eq!(a, b);
        assert!(a.as_millis() >= 120);
        assert!(a.as_millis() < 220);
    }

    #[test]
    fn parses_retry_after_seconds() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_static("3"),
        );
        assert_eq!(retry_after_delay(&headers).map(|d| d.as_secs()), Some(3));
    }
}
