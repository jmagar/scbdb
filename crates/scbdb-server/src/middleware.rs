use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use uuid::Uuid;

/// Newtype wrapping a request ID string, stored as a request extension.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

/// Axum middleware that extracts or generates a request ID.
///
/// If the incoming request has an `x-request-id` header, that value is used.
/// Otherwise a new `UUIDv4` is generated. The ID is:
/// - Inserted into request extensions as [`RequestId`]
/// - Set on the response as the `x-request-id` header
pub async fn request_id(mut req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map_or_else(|| Uuid::new_v4().to_string(), String::from);

    req.extensions_mut().insert(RequestId(id.clone()));

    let mut res = next.run(req).await;

    if let Ok(val) = HeaderValue::from_str(&id) {
        res.headers_mut().insert("x-request-id", val);
    }

    res
}
