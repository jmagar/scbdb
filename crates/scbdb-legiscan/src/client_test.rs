use super::*;

fn test_client(base_url: &str) -> LegiscanClient {
    LegiscanClient::with_base_url("test-key", 30, base_url)
        .expect("client construction should not fail")
}

#[test]
fn build_url_constructs_correct_query_string() {
    let client = test_client("https://api.legiscan.com");
    let url = client.build_url("getBill", &[("id", "42")]);
    assert_eq!(
        url.as_str(),
        "https://api.legiscan.com/?key=test-key&op=getBill&id=42"
    );
}

#[test]
fn build_url_strips_trailing_slash() {
    let client = test_client("https://api.legiscan.com/");
    let url = client.build_url("search", &[("query", "hemp"), ("state", "SC")]);
    assert_eq!(
        url.as_str(),
        "https://api.legiscan.com/?key=test-key&op=search&query=hemp&state=SC"
    );
}

#[test]
fn build_url_encodes_special_characters() {
    let client = test_client("https://api.legiscan.com");
    let url = client.build_url("search", &[("query", "hemp & cbd")]);
    assert!(
        url.as_str().contains("hemp+%26+cbd") || url.as_str().contains("hemp%20%26%20cbd"),
        "query param should be percent-encoded: {url}"
    );
}
