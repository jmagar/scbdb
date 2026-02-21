// Brand Intelligence Layer types — Phase 6

export type BrandSummaryItem = {
  id: number;
  slug: string;
  name: string;
  relationship: string;
  tier: number;
  logo_url: string | null;
  completeness_score: number;
};

export type BrandProfile = {
  tagline: string | null;
  description: string | null;
  founded_year: number | null;
  hq_city: string | null;
  hq_state: string | null;
  hq_country: string | null;
  parent_company: string | null;
  ceo_name: string | null;
  employee_count_approx: number | null;
  total_funding_usd: number | null;
  latest_valuation_usd: number | null;
  funding_stage: string | null;
};

export type SocialHandle = {
  platform: string;
  handle: string;
  profile_url: string | null;
  follower_count: number | null;
  is_verified: boolean;
};

export type BrandCompleteness = {
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

export type BrandProfileResponse = {
  id: number;
  slug: string;
  name: string;
  relationship: string;
  tier: number;
  logo_url: string | null;
  profile: BrandProfile | null;
  social_handles: SocialHandle[];
  domains: string[];
  completeness: BrandCompleteness;
};

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

export type PaginatedSignals = {
  items: SignalItem[];
  next_cursor: number | null;
};

export type FundingEventItem = {
  id: number;
  event_type: string;
  amount_usd: number | null;
  announced_at: string | null;
  investors: string[];
  acquirer: string | null;
  source_url: string | null;
  notes: string | null;
  created_at: string;
};

export type LabTestItem = {
  id: number;
  lab_name: string | null;
  test_date: string | null;
  thc_mg_actual: number | null;
  cbd_mg_actual: number | null;
  total_cannabinoids_mg: number | null;
  passed: boolean | null;
  report_url: string | null;
  created_at: string;
};

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

export type SponsorshipItem = {
  id: number;
  entity_name: string;
  entity_type: string | null;
  deal_type: string | null;
  announced_at: string | null;
  ends_at: string | null;
  source_url: string | null;
  notes: string | null;
  is_active: boolean;
  created_at: string;
};

export type DistributorItem = {
  id: number;
  distributor_name: string;
  distributor_slug: string | null;
  states: string[];
  territory_type: string | null;
  channel_type: string | null;
  started_at: string | null;
  ended_at: string | null;
  is_active: boolean;
  notes: string | null;
  created_at: string;
};

export type CompetitorItem = {
  id: number;
  brand_id: number;
  competitor_brand_id: number;
  relationship_type: string | null;
  distributor_name: string | null;
  states: string[];
  notes: string | null;
  first_observed_at: string | null;
  is_active: boolean;
  created_at: string;
};

export type MediaAppearanceItem = {
  id: number;
  appearance_type: string;
  outlet_name: string | null;
  title: string;
  host_or_author: string | null;
  aired_at: string | null;
  duration_seconds: number | null;
  source_url: string | null;
  notes: string | null;
  created_at: string;
};
