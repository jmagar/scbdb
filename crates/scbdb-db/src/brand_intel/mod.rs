//! Database operations for brand intelligence tables:
//! `brand_funding_events`, `brand_lab_tests`, `brand_legal_proceedings`,
//! and `brand_sponsorships`.

mod funding_events;
mod lab_tests;
mod legal_proceedings;
mod sponsorships;

pub use funding_events::{
    insert_brand_funding_event, list_brand_funding_events, BrandFundingEventRow,
    NewBrandFundingEvent,
};
pub use lab_tests::{
    insert_brand_lab_test, list_brand_lab_tests, BrandLabTestRow, NewBrandLabTest,
};
pub use legal_proceedings::{
    insert_brand_legal_proceeding, list_brand_legal_proceedings, BrandLegalProceedingRow,
    NewBrandLegalProceeding,
};
pub use sponsorships::{
    insert_brand_sponsorship, list_brand_sponsorships, BrandSponsorshipRow, NewBrandSponsorship,
};
