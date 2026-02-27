//! Database operations for brand intelligence tables:
//! `brand_funding_events`, `brand_lab_tests`, `brand_legal_proceedings`,
//! `brand_sponsorships`, `brand_distributors`, `brand_competitor_relationships`,
//! `brand_newsletters`, and `brand_media_appearances`.

mod competitor_relationships;
mod distributors;
mod funding_events;
mod lab_tests;
mod legal_proceedings;
mod media_appearances;
mod newsletters;
mod sponsorships;

pub use competitor_relationships::{
    insert_brand_competitor_relationship, list_brand_competitor_relationships,
    BrandCompetitorRelationshipRow, NewBrandCompetitorRelationship,
};
pub use distributors::{
    insert_brand_distributor, list_brand_distributors, BrandDistributorRow, NewBrandDistributor,
};
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
pub use media_appearances::{
    insert_brand_media_appearance, list_brand_media_appearances, BrandMediaAppearanceRow,
    NewBrandMediaAppearance,
};
pub use newsletters::{
    insert_brand_newsletter, list_brand_newsletters, BrandNewsletterRow, NewBrandNewsletter,
};
pub use sponsorships::{
    insert_brand_sponsorship, list_brand_sponsorships, BrandSponsorshipRow, NewBrandSponsorship,
};
