export type ApiResponse<T> = {
  data: T;
  meta: {
    request_id: string;
    timestamp: string;
  };
};

export type ProductItem = {
  product_id: number;
  product_name: string;
  product_status: string | null;
  vendor: string | null;
  source_url: string | null;
  primary_image_url: string | null;
  brand_name: string;
  brand_slug: string;
  brand_logo_url: string | null;
  relationship: string;
  tier: number;
  variant_count: number;
  latest_price: string | null;
  latest_price_captured_at: string | null;
};

export type PricingSummaryItem = {
  brand_name: string;
  brand_slug: string;
  brand_logo_url: string | null;
  variant_count: number;
  avg_price: string;
  min_price: string;
  max_price: string;
  latest_capture_at: string;
};

export type PricingSnapshotItem = {
  captured_at: string;
  currency_code: string;
  price: string;
  compare_at_price: string | null;
  variant_title: string | null;
  source_variant_id: string;
  product_name: string;
  brand_name: string;
  brand_slug: string;
  brand_logo_url: string | null;
};

export type BillItem = {
  bill_id: string;
  jurisdiction: string;
  session: string | null;
  bill_number: string;
  title: string;
  status: string;
  status_date: string | null;
  last_action_date: string | null;
  source_url: string | null;
  event_count: number;
};

export type BillEventItem = {
  event_date: string | null;
  event_type: string | null;
  chamber: string | null;
  description: string;
  source_url: string | null;
};

export type SentimentSummaryItem = {
  brand_name: string;
  brand_slug: string;
  score: string;
  signal_count: number;
  captured_at: string;
  metadata?: SentimentMetadata;
};

export type SentimentSnapshotItem = {
  brand_name: string;
  brand_slug: string;
  score: string;
  signal_count: number;
  captured_at: string;
  metadata?: SentimentMetadata;
};

export type SentimentEvidence = {
  source: string;
  url: string;
  score: number;
  text_preview: string;
};

export type SentimentMetadata = {
  version?: number;
  brand_slug?: string;
  source_counts?: Record<string, number>;
  top_signals?: SentimentEvidence[];
  captured_at?: string;
};

export type LocationBrandSummary = {
  brand_name: string;
  brand_slug: string;
  active_count: number;
  new_this_week: number;
  states_covered: number;
  locator_source: string | null;
  last_seen_at: string | null;
};

export type LocationsByState = {
  state: string;
  brand_count: number;
  location_count: number;
};

// ── Brand Intelligence Layer — re-exported from ./brands ──────────────────────
export type {
  BrandCompleteness,
  BrandProfile,
  BrandProfileResponse,
  BrandSignalType,
  BrandSummaryItem,
  CompetitorItem,
  DistributorItem,
  FundingEventItem,
  LabTestItem,
  LegalProceedingItem,
  MediaAppearanceItem,
  PaginatedSignals,
  SignalItem,
  SocialHandle,
  SponsorshipItem,
} from "./brands";
