import type {
  BrandSummaryItem,
  BrandProfileResponse,
  CompetitorItem,
  DistributorItem,
  FundingEventItem,
  LabTestItem,
  LegalProceedingItem,
  MediaAppearanceItem,
  PaginatedSignals,
  SponsorshipItem,
} from "../../types/brands";
import { apiGet } from "./client";

export async function fetchBrands(): Promise<BrandSummaryItem[]> {
  return apiGet<BrandSummaryItem[]>("/api/v1/brands");
}

export async function fetchBrand(slug: string): Promise<BrandProfileResponse> {
  return apiGet<BrandProfileResponse>(`/api/v1/brands/${slug}`);
}

export async function fetchBrandSignals(
  slug: string,
  params?: { type?: string; cursor?: number; limit?: number },
): Promise<PaginatedSignals> {
  return apiGet<PaginatedSignals>(`/api/v1/brands/${slug}/signals`, {
    type: params?.type,
    cursor: params?.cursor,
    limit: params?.limit,
  });
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
