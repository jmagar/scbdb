import type {
  BillEventItem,
  BillItem,
  BillTextItem,
  BrandProfileResponse,
  BrandSignalType,
  BrandSummaryItem,
  CompetitorItem,
  DistributorItem,
  FundingEventItem,
  LabTestItem,
  LegalProceedingItem,
  LocationBrandSummary,
  LocationPin,
  LocationsByState,
  MediaAppearanceItem,
  PaginatedSignals,
  PricingSnapshotItem,
  PricingSummaryItem,
  ProductItem,
  SentimentSnapshotItem,
  SentimentSummaryItem,
  SponsorshipItem,
} from "../../types/api";
import { apiGet } from "./client";

export async function fetchProducts(): Promise<ProductItem[]> {
  return apiGet<ProductItem[]>("/api/v1/products");
}

export async function fetchPricingSummary(): Promise<PricingSummaryItem[]> {
  return apiGet<PricingSummaryItem[]>("/api/v1/pricing/summary");
}

export async function fetchPricingSnapshots(): Promise<PricingSnapshotItem[]> {
  return apiGet<PricingSnapshotItem[]>("/api/v1/pricing/snapshots", {
    limit: 30,
  });
}

export async function fetchBills(): Promise<BillItem[]> {
  return apiGet<BillItem[]>("/api/v1/bills", { limit: 30 });
}

export async function fetchBillEvents(
  billId: string,
): Promise<BillEventItem[]> {
  return apiGet<BillEventItem[]>(`/api/v1/bills/${billId}/events`);
}

export async function fetchBillTexts(billId: string): Promise<BillTextItem[]> {
  return apiGet<BillTextItem[]>(`/api/v1/bills/${billId}/texts`);
}

export async function fetchSentimentSummary(): Promise<SentimentSummaryItem[]> {
  return apiGet<SentimentSummaryItem[]>("/api/v1/sentiment/summary");
}

export async function fetchSentimentSnapshots(): Promise<
  SentimentSnapshotItem[]
> {
  return apiGet<SentimentSnapshotItem[]>("/api/v1/sentiment/snapshots", {
    limit: 30,
  });
}

export async function fetchLocationsSummary(): Promise<LocationBrandSummary[]> {
  return apiGet<LocationBrandSummary[]>("/api/v1/locations/summary");
}

export async function fetchLocationsByState(): Promise<LocationsByState[]> {
  return apiGet<LocationsByState[]>("/api/v1/locations/by-state");
}

export async function fetchLocationPins(): Promise<LocationPin[]> {
  return apiGet<LocationPin[]>("/api/v1/locations/pins");
}

// ── Brand Intelligence Layer ──────────────────────────────────────────────────

export async function fetchBrands(): Promise<BrandSummaryItem[]> {
  return apiGet<BrandSummaryItem[]>("/api/v1/brands");
}

export async function fetchBrandProfile(
  slug: string,
): Promise<BrandProfileResponse> {
  return apiGet<BrandProfileResponse>(`/api/v1/brands/${slug}`);
}

export async function fetchBrandSignals(
  slug: string,
  params?: { type?: BrandSignalType; limit?: number; cursor?: number },
): Promise<PaginatedSignals> {
  return apiGet<PaginatedSignals>(`/api/v1/brands/${slug}/signals`, params);
}

export async function fetchBrandFunding(
  slug: string,
): Promise<FundingEventItem[]> {
  return apiGet<FundingEventItem[]>(`/api/v1/brands/${slug}/funding`);
}

export async function fetchBrandLabTests(slug: string): Promise<LabTestItem[]> {
  return apiGet<LabTestItem[]>(`/api/v1/brands/${slug}/lab-tests`);
}

export async function fetchBrandLegal(
  slug: string,
): Promise<LegalProceedingItem[]> {
  return apiGet<LegalProceedingItem[]>(`/api/v1/brands/${slug}/legal`);
}

export async function fetchBrandSponsorships(
  slug: string,
): Promise<SponsorshipItem[]> {
  return apiGet<SponsorshipItem[]>(`/api/v1/brands/${slug}/sponsorships`);
}

export async function fetchBrandDistributors(
  slug: string,
): Promise<DistributorItem[]> {
  return apiGet<DistributorItem[]>(`/api/v1/brands/${slug}/distributors`);
}

export async function fetchBrandCompetitors(
  slug: string,
): Promise<CompetitorItem[]> {
  return apiGet<CompetitorItem[]>(`/api/v1/brands/${slug}/competitors`);
}

export async function fetchBrandMedia(
  slug: string,
): Promise<MediaAppearanceItem[]> {
  return apiGet<MediaAppearanceItem[]>(`/api/v1/brands/${slug}/media`);
}
