//! `LegiScan` API client for SCBDB regulatory tracking.
//!
//! Provides a typed client for the [`LegiScan` API](https://legiscan.com/legiscan),
//! along with domain-level normalization for persisting bill and event data.

pub mod client;
pub mod error;
pub mod normalize;
pub mod types;

pub use client::LegiscanClient;
pub use error::LegiscanError;
pub use normalize::{normalize_bill, normalize_bill_events, NormalizedBill, NormalizedBillEvent};
