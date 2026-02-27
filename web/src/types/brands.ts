/** Brand Intelligence Layer types -- matches Rust API response structs */

/** Valid signal type values matching the `brand_signal_type` DB enum. */
export type BrandSignalType =
  | "article"
  | "blog_post"
  | "tweet"
  | "youtube_video"
  | "reddit_post"
  | "newsletter"
  | "press_release"
  | "podcast_episode"
  | "event"
  | "award"
  | "partnership"
  | "launch";

/** Brand list item returned by GET /api/v1/brands */
export type BrandSummaryItem = {
  id: number;
  slug: string;
  name: string;
  relationship: BrandRelationship;
  tier: BrandTier;
  logo_url: string | null;
  completeness_score: number;
};

/** Full brand profile returned by GET /api/v1/brands/:slug */
export type BrandProfileResponse = {
  id: number;
  slug: string;
  name: string;
  relationship: BrandRelationship;
  tier: BrandTier;
  domain: string | null;
  shop_url: string | null;
  store_locator_url: string | null;
  twitter_handle: string | null;
  notes: string | null;
  logo_url: string | null;
  profile: BrandProfileDetail | null;
  social_handles: BrandSocialHandleItem[];
  domains: string[];
  completeness: BrandCompletenessDetail;
};

export type BrandProfileDetail = {
  tagline: string | null;
  description: string | null;
  founded_year: number | null;
  hq_city: string | null;
  hq_state: string | null;
  hq_country: string;
  parent_company: string | null;
  ceo_name: string | null;
  employee_count_approx: number | null;
  total_funding_usd: number | null;
  latest_valuation_usd: number | null;
  funding_stage: string | null;
};

export type BrandSocialHandleItem = {
  platform: string;
  handle: string;
  profile_url: string | null;
  follower_count: number | null;
  is_verified: boolean | null;
};

export type BrandCompletenessDetail = {
  score: number;
  has_profile: boolean;
  has_description: boolean;
  has_tagline: boolean;
  has_founded_year: boolean;
  has_location: boolean;
  has_social_handles: boolean;
  has_domains: boolean;
  has_signals: boolean;
  has_funding: boolean;
  has_lab_tests: boolean;
  has_legal: boolean;
  has_sponsorships: boolean;
  has_distributors: boolean;
  has_media: boolean;
};

/** Signal feed item returned by GET /api/v1/brands/:slug/signals */
export type SignalItem = {
  id: number;
  public_id: string;
  signal_type: string;
  title: string | null;
  summary: string | null;
  source_url: string | null;
  image_url: string | null;
  published_at: string | null;
  collected_at: string;
};

/** Cursor-paginated signal feed wrapper */
export type PaginatedSignals = {
  items: SignalItem[];
  next_cursor: number | null;
};

/** Funding event returned by GET /api/v1/brands/:slug/funding */
export type FundingEventItem = {
  id: number;
  event_type: string;
  amount_usd: number | null;
  announced_at: string | null;
  investors: string[] | null;
  acquirer: string | null;
  source_url: string | null;
  notes: string | null;
  created_at: string;
};

/** Lab test returned by GET /api/v1/brands/:slug/lab-tests */
export type LabTestItem = {
  id: number;
  product_id: number | null;
  variant_id: number | null;
  lab_name: string | null;
  test_date: string | null;
  report_url: string | null;
  thc_mg_actual: string | null;
  cbd_mg_actual: string | null;
  total_cannabinoids_mg: string | null;
  passed: boolean | null;
  created_at: string;
};

/** Legal proceeding returned by GET /api/v1/brands/:slug/legal */
export type LegalProceedingItem = {
  id: number;
  proceeding_type: string;
  jurisdiction: string | null;
  case_number: string | null;
  title: string;
  summary: string | null;
  status: string;
  filed_at: string | null;
  resolved_at: string | null;
  source_url: string | null;
  created_at: string;
};

/** Sponsorship returned by GET /api/v1/brands/:slug/sponsorships */
export type SponsorshipItem = {
  id: number;
  entity_name: string;
  entity_type: string;
  deal_type: string;
  announced_at: string | null;
  ends_at: string | null;
  source_url: string | null;
  notes: string | null;
  is_active: boolean;
  created_at: string;
};

/** Distributor returned by GET /api/v1/brands/:slug/distributors */
export type DistributorItem = {
  id: number;
  distributor_name: string;
  distributor_slug: string;
  states: string[] | null;
  territory_type: string;
  channel_type: string;
  started_at: string | null;
  ended_at: string | null;
  is_active: boolean;
  notes: string | null;
  created_at: string;
};

/** Competitor relationship returned by GET /api/v1/brands/:slug/competitors */
export type CompetitorItem = {
  id: number;
  brand_id: number;
  competitor_brand_id: number;
  relationship_type: string;
  distributor_name: string | null;
  states: string[] | null;
  notes: string | null;
  first_observed_at: string;
  is_active: boolean;
  created_at: string;
};

/** Media appearance returned by GET /api/v1/brands/:slug/media */
export type MediaAppearanceItem = {
  id: number;
  brand_signal_id: number | null;
  appearance_type: string;
  outlet_name: string;
  title: string | null;
  host_or_author: string | null;
  aired_at: string | null;
  duration_seconds: number | null;
  source_url: string | null;
  notes: string | null;
  created_at: string;
};

// ── Write body types ──────────────────────────────────────────────────────────

export type BrandRelationship = "portfolio" | "competitor";
export type BrandTier = 1 | 2 | 3;

export type CreateBrandBody = {
  name: string;
  relationship: BrandRelationship;
  tier: 1 | 2 | 3;
  domain?: string;
  shop_url?: string;
  store_locator_url?: string;
  twitter_handle?: string;
  notes?: string;
};

export type CreateBrandResponse = { id: number; slug: string };

export type UpdateBrandMetaBody = {
  name?: string;
  relationship?: BrandRelationship;
  tier?: 1 | 2 | 3;
  domain?: string | null;
  shop_url?: string | null;
  store_locator_url?: string | null;
  twitter_handle?: string | null;
  notes?: string | null;
};

export type UpdateBrandProfileBody = {
  tagline?: string | null;
  description?: string | null;
  founded_year?: number | null;
  hq_city?: string | null;
  hq_state?: string | null;
  ceo_name?: string | null;
  funding_stage?: string | null;
  employee_count_approx?: number | null;
};

/** platform → handle */
export type UpdateSocialHandlesBody = { handles: Record<string, string> };
export type UpdateDomainsBody = { domains: string[] };
