//! Session and master-list endpoints for the `LegiScan` API client.

use crate::client::LegiscanClient;
use crate::error::LegiscanError;
use crate::types::{
    ApiResponse, MasterListEntry, MasterListResponse, SessionDetail, SessionInfo,
    SessionListResponse,
};

impl LegiscanClient {
    /// Gets the list of legislative sessions for a state (e.g., `"SC"`).
    ///
    /// # Errors
    ///
    /// - [`LegiscanError::BudgetExceeded`] if the session budget is reached.
    /// - [`LegiscanError::ApiError`] on API-level failure.
    /// - [`LegiscanError::Http`] on network failure.
    /// - [`LegiscanError::Deserialize`] if the response shape is unexpected.
    pub async fn get_session_list(&self, state: &str) -> Result<Vec<SessionInfo>, LegiscanError> {
        let state_upper = state.to_uppercase();
        let url = self.build_url("getSessionList", &[("state", &state_upper)]);
        let body = self.request_json(&url).await?;
        Self::check_api_error(&body)?;
        let envelope: ApiResponse<SessionListResponse> =
            serde_json::from_value(body).map_err(|e| LegiscanError::Deserialize {
                context: format!("getSessionList(state={state})"),
                source: e,
            })?;
        Ok(envelope.data.sessions)
    }

    /// Gets the master bill list for a specific session by ID.
    ///
    /// Use this to retrieve bills from prior or expired sessions during a
    /// historical backfill. The response shape is identical to [`get_master_list`].
    ///
    /// # Errors
    ///
    /// - [`LegiscanError::BudgetExceeded`] if the session budget is reached.
    /// - [`LegiscanError::ApiError`] on API-level failure.
    /// - [`LegiscanError::Http`] on network failure.
    /// - [`LegiscanError::Deserialize`] if the response shape is unexpected.
    pub async fn get_master_list_by_session(
        &self,
        session_id: i64,
    ) -> Result<(SessionDetail, Vec<MasterListEntry>), LegiscanError> {
        let id_str = session_id.to_string();
        let url = self.build_url("getMasterList", &[("id", &id_str)]);
        let body = self.request_json(&url).await?;
        Self::check_api_error(&body)?;
        let envelope: ApiResponse<MasterListResponse> =
            serde_json::from_value(body).map_err(|e| LegiscanError::Deserialize {
                context: format!("getMasterList(session_id={session_id})"),
                source: e,
            })?;
        let data = envelope.data.masterlist;
        let entries = data
            .bills
            .into_iter()
            .filter(|(k, _)| k != "session")
            .filter_map(|(k, v)| {
                serde_json::from_value::<MasterListEntry>(v)
                    .map_err(|e| {
                        tracing::warn!(key = %k, error = %e, "get_master_list_by_session: skipping malformed entry");
                    })
                    .ok()
            })
            .collect();
        Ok((data.session, entries))
    }

    /// Gets the master bill list for a state's current session.
    ///
    /// # Errors
    ///
    /// - [`LegiscanError::BudgetExceeded`] if the session budget is reached.
    /// - [`LegiscanError::ApiError`] on API-level failure.
    /// - [`LegiscanError::Http`] on network failure.
    /// - [`LegiscanError::Deserialize`] if the top-level response shape is unexpected.
    pub async fn get_master_list(
        &self,
        state: &str,
    ) -> Result<(SessionDetail, Vec<MasterListEntry>), LegiscanError> {
        let state_upper = state.to_uppercase();
        let url = self.build_url("getMasterList", &[("state", &state_upper)]);
        let body = self.request_json(&url).await?;
        Self::check_api_error(&body)?;
        let envelope: ApiResponse<MasterListResponse> =
            serde_json::from_value(body).map_err(|e| LegiscanError::Deserialize {
                context: format!("getMasterList(state={state})"),
                source: e,
            })?;
        let data = envelope.data.masterlist;
        let entries = data
            .bills
            .into_iter()
            .filter(|(k, _)| k != "session")
            .filter_map(|(k, v)| {
                serde_json::from_value::<MasterListEntry>(v)
                    .map_err(|e| {
                        tracing::warn!(key = %k, error = %e, "get_master_list: skipping malformed entry");
                    })
                    .ok()
            })
            .collect();
        Ok((data.session, entries))
    }
}
