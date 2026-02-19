//! Normalization of `LegiScan` API types into domain types suitable for
//! database persistence.

use chrono::NaiveDate;

use crate::types::BillDetail;

/// A normalized bill ready for database persistence.
#[derive(Debug, Clone)]
pub struct NormalizedBill {
    pub bill_id_external: i64,
    pub jurisdiction: String,
    pub session: Option<String>,
    pub bill_number: String,
    pub title: String,
    pub summary: Option<String>,
    pub status: String,
    pub status_date: Option<NaiveDate>,
    pub introduced_date: Option<NaiveDate>,
    pub last_action_date: Option<NaiveDate>,
    pub source_url: Option<String>,
}

/// A normalized bill event (history action) ready for database persistence.
#[derive(Debug, Clone)]
pub struct NormalizedBillEvent {
    pub event_date: Option<NaiveDate>,
    pub event_type: Option<String>,
    pub chamber: Option<String>,
    pub description: String,
    pub source_url: Option<String>,
}

/// Maps a `LegiScan` integer status code to a human-readable string.
#[must_use]
pub fn map_status(status: i32) -> String {
    match status {
        1 => "introduced".to_string(),
        2 => "engrossed".to_string(),
        3 => "enrolled".to_string(),
        4 => "passed".to_string(),
        5 => "vetoed".to_string(),
        6 => "failed".to_string(),
        _ => format!("unknown({status})"),
    }
}

/// Parses a `"YYYY-MM-DD"` date string into a [`NaiveDate`].
///
/// Returns `None` if the string does not match the expected format.
#[must_use]
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Converts a [`BillDetail`] from the `LegiScan` API into a [`NormalizedBill`]
/// suitable for database persistence.
#[must_use]
pub fn normalize_bill(detail: &BillDetail) -> NormalizedBill {
    let introduced_date = detail.history.first().and_then(|h| parse_date(&h.date));
    let last_action_date = detail.history.last().and_then(|h| parse_date(&h.date));
    let status_date = detail.status_date.as_deref().and_then(parse_date);
    let session = detail.session.as_ref().map(|s| s.session_name.clone());

    NormalizedBill {
        bill_id_external: detail.bill_id,
        jurisdiction: detail.state.clone(),
        session,
        bill_number: detail.bill_number.clone(),
        title: detail.title.clone(),
        summary: detail.description.clone(),
        status: map_status(detail.status),
        status_date,
        introduced_date,
        last_action_date,
        source_url: detail.url.clone(),
    }
}

/// Converts the history entries of a [`BillDetail`] into a list of
/// [`NormalizedBillEvent`]s for database persistence.
#[must_use]
pub fn normalize_bill_events(detail: &BillDetail) -> Vec<NormalizedBillEvent> {
    detail
        .history
        .iter()
        .map(|h| NormalizedBillEvent {
            event_date: parse_date(&h.date),
            event_type: None,
            chamber: h.chamber.clone(),
            description: h.action.clone(),
            source_url: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BillHistory, SessionDetail};

    #[test]
    fn map_status_known_codes() {
        assert_eq!(map_status(1), "introduced");
        assert_eq!(map_status(4), "passed");
        assert_eq!(map_status(5), "vetoed");
    }

    #[test]
    fn map_status_unknown_code() {
        assert_eq!(map_status(99), "unknown(99)");
    }

    #[test]
    fn parse_date_valid() {
        let d = parse_date("2025-03-15");
        assert_eq!(d, Some(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()));
    }

    #[test]
    fn parse_date_invalid() {
        assert_eq!(parse_date("not-a-date"), None);
        assert_eq!(parse_date(""), None);
    }

    #[test]
    fn normalize_bill_extracts_dates_and_session() {
        let detail = BillDetail {
            bill_id: 42,
            bill_number: "HB100".to_string(),
            title: "Hemp Regulation".to_string(),
            description: Some("Regulates hemp beverages".to_string()),
            status: 4,
            status_date: Some("2025-06-01".to_string()),
            state: "SC".to_string(),
            session: Some(SessionDetail {
                session_id: 1,
                session_name: "2025 Regular Session".to_string(),
                year_start: 2025,
                year_end: 2026,
            }),
            url: Some("https://legiscan.com/SC/bill/HB100".to_string()),
            history: vec![
                BillHistory {
                    date: "2025-01-10".to_string(),
                    action: "Introduced".to_string(),
                    chamber: Some("House".to_string()),
                },
                BillHistory {
                    date: "2025-06-01".to_string(),
                    action: "Passed".to_string(),
                    chamber: Some("Senate".to_string()),
                },
            ],
            progress: vec![],
        };

        let normalized = normalize_bill(&detail);
        assert_eq!(normalized.jurisdiction, "SC");
        assert_eq!(normalized.session.as_deref(), Some("2025 Regular Session"));
        assert_eq!(normalized.status, "passed");
        assert_eq!(
            normalized.introduced_date,
            Some(NaiveDate::from_ymd_opt(2025, 1, 10).unwrap())
        );
        assert_eq!(
            normalized.last_action_date,
            Some(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap())
        );
    }

    #[test]
    fn normalize_bill_events_maps_history() {
        let detail = BillDetail {
            bill_id: 1,
            bill_number: "SB50".to_string(),
            title: "Test".to_string(),
            description: None,
            status: 1,
            status_date: None,
            state: "TX".to_string(),
            session: None,
            url: None,
            history: vec![BillHistory {
                date: "2025-03-01".to_string(),
                action: "Filed".to_string(),
                chamber: Some("Senate".to_string()),
            }],
            progress: vec![],
        };

        let events = normalize_bill_events(&detail);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].description, "Filed");
        assert_eq!(events[0].chamber.as_deref(), Some("Senate"));
    }
}
