//! Integration tests for `LegiscanClient` using wiremock HTTP mocks.

use scbdb_legiscan::LegiscanClient;
use wiremock::matchers::{method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_client(base_url: &str) -> LegiscanClient {
    LegiscanClient::with_base_url("test-key", 30, base_url)
        .expect("client construction should not fail")
}

#[tokio::test]
async fn get_bill_returns_parsed_bill() {
    let server = MockServer::start().await;

    let body = serde_json::json!({
        "status": "OK",
        "bill": {
            "bill_id": 12345,
            "bill_number": "HB100",
            "title": "Hemp Beverage Act",
            "description": "Regulates hemp-derived THC beverages",
            "status": 1,
            "status_date": "2025-01-15",
            "state": "SC",
            "session": {
                "session_id": 99,
                "session_name": "2025 Regular",
                "year_start": 2025,
                "year_end": 2026
            },
            "url": "https://legiscan.com/SC/bill/HB100/2025",
            "history": [
                { "date": "2025-01-10", "action": "Introduced", "chamber": "House" }
            ],
            "progress": [
                { "date": "2025-01-10", "event": 1 }
            ]
        }
    });

    Mock::given(method("GET"))
        .and(query_param("op", "getBill"))
        .and(query_param("key", "test-key"))
        .and(query_param("id", "12345"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let bill = client.get_bill(12345).await.expect("should parse bill");

    assert_eq!(bill.bill_id, 12345);
    assert_eq!(bill.bill_number, "HB100");
    assert_eq!(bill.title, "Hemp Beverage Act");
    assert_eq!(bill.state, "SC");
    assert_eq!(bill.status, 1);
    assert_eq!(bill.history.len(), 1);
    assert_eq!(bill.history[0].action, "Introduced");
}

#[tokio::test]
async fn search_bills_returns_results() {
    let server = MockServer::start().await;

    let body = serde_json::json!({
        "status": "OK",
        "searchresult": {
            "summary": {
                "page": 1,
                "range": 50,
                "relevancy": 100,
                "count": 1
            },
            "results": [
                {
                    "bill_id": 999,
                    "bill_number": "SB200",
                    "title": "THC Regulation",
                    "state": "SC",
                    "status": 4,
                    "status_date": "2025-06-01",
                    "last_action_date": "2025-06-01",
                    "last_action": "Signed by Governor",
                    "url": "https://legiscan.com/SC/bill/SB200"
                }
            ]
        }
    });

    Mock::given(method("GET"))
        .and(query_param("op", "search"))
        .and(query_param("query", "hemp"))
        .and(query_param("state", "SC"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let results = client
        .search_bills("hemp", Some("SC"))
        .await
        .expect("should parse search results");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].bill_id, 999);
    assert_eq!(results[0].bill_number, "SB200");
}

#[tokio::test]
async fn get_session_list_returns_sessions() {
    let server = MockServer::start().await;

    let body = serde_json::json!({
        "status": "OK",
        "sessions": [
            {
                "session_id": 10,
                "state_id": 40,
                "year_start": 2025,
                "year_end": 2026,
                "session_name": "2025-2026 Regular",
                "special": 0,
                "prior": 0,
                "sine_die": 0
            }
        ]
    });

    Mock::given(method("GET"))
        .and(query_param("op", "getSessionList"))
        .and(query_param("state", "SC"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let sessions = client
        .get_session_list("SC")
        .await
        .expect("should parse sessions");

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_name, "2025-2026 Regular");
}

#[tokio::test]
async fn get_master_list_parses_bill_entries() {
    let server = MockServer::start().await;

    let body = serde_json::json!({
        "status": "OK",
        "masterlist": {
            "session": {
                "session_id": 10,
                "session_name": "2025 Regular",
                "year_start": 2025,
                "year_end": 2026
            },
            "0": {
                "bill_id": 100,
                "number": "HB1",
                "title": "Test Bill",
                "status": 1,
                "status_date": "2025-01-05",
                "last_action_date": "2025-01-05",
                "last_action": "Filed",
                "url": "https://legiscan.com/SC/bill/HB1",
                "change_hash": "abc123"
            },
            "1": {
                "bill_id": 101,
                "number": "SB1",
                "title": "Another Bill",
                "status": 2,
                "change_hash": "def456"
            }
        }
    });

    Mock::given(method("GET"))
        .and(query_param("op", "getMasterList"))
        .and(query_param("state", "SC"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let (session, entries) = client
        .get_master_list("SC")
        .await
        .expect("should parse master list");

    assert_eq!(session.session_name, "2025 Regular");
    assert_eq!(entries.len(), 2);

    let ids: Vec<i64> = entries.iter().map(|e| e.bill_id).collect();
    assert!(ids.contains(&100));
    assert!(ids.contains(&101));
}

#[tokio::test]
async fn api_error_response_returns_err() {
    let server = MockServer::start().await;

    let body = serde_json::json!({
        "status": "ERROR",
        "alert": {
            "message": "Invalid API key"
        }
    });

    Mock::given(method("GET"))
        .and(query_param("op", "getBill"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let result = client.get_bill(1).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Invalid API key"),
        "expected error message to contain 'Invalid API key', got: {msg}"
    );
}
