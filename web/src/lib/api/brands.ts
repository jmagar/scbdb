import type {
  BrandSummaryItem,
  BrandProfileResponse,
  CompetitorItem,
  CreateBrandBody,
  CreateBrandResponse,
  DistributorItem,
  FundingEventItem,
  LabTestItem,
  LegalProceedingItem,
  MediaAppearanceItem,
  PaginatedSignals,
  SponsorshipItem,
  UpdateBrandMetaBody,
  UpdateBrandProfileBody,
  UpdateDomainsBody,
  UpdateSocialHandlesBody,
} from "../../types/brands";
import { apiGet, apiMutate } from "./client";

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

// ── Write functions ───────────────────────────────────────────────────────────

export async function createBrand(
  body: CreateBrandBody,
): Promise<CreateBrandResponse> {
  return apiMutate<CreateBrandBody, CreateBrandResponse>(
    "POST",
    "/api/v1/brands",
    body,
  );
}

export async function updateBrandMeta(
  slug: string,
  body: UpdateBrandMetaBody,
): Promise<void> {
  return apiMutate<UpdateBrandMetaBody, void>(
    "PATCH",
    `/api/v1/brands/${slug}`,
    body,
  );
}

export async function updateBrandProfile(
  slug: string,
  body: UpdateBrandProfileBody,
): Promise<void> {
  return apiMutate<UpdateBrandProfileBody, void>(
    "PUT",
    `/api/v1/brands/${slug}/profile`,
    body,
  );
}

export async function updateBrandSocial(
  slug: string,
  body: UpdateSocialHandlesBody,
): Promise<void> {
  return apiMutate<UpdateSocialHandlesBody, void>(
    "PUT",
    `/api/v1/brands/${slug}/social`,
    body,
  );
}

export async function updateBrandDomains(
  slug: string,
  body: UpdateDomainsBody,
): Promise<void> {
  return apiMutate<UpdateDomainsBody, void>(
    "PUT",
    `/api/v1/brands/${slug}/domains`,
    body,
  );
}

export async function deactivateBrand(slug: string): Promise<void> {
  return apiMutate<undefined, void>("DELETE", `/api/v1/brands/${slug}`);
}
