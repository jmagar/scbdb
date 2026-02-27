import type {
  BrandProfileResponse,
  BrandSignalType,
  BrandSummaryItem,
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

function getBrandPath(slug: string): string {
  return `/api/v1/brands/${encodeURIComponent(slug)}`;
}

export async function fetchBrands(): Promise<BrandSummaryItem[]> {
  return apiGet<BrandSummaryItem[]>("/api/v1/brands");
}

export async function fetchBrand(slug: string): Promise<BrandProfileResponse> {
  return apiGet<BrandProfileResponse>(getBrandPath(slug));
}

export async function fetchBrandProfile(
  slug: string,
): Promise<BrandProfileResponse> {
  return fetchBrand(slug);
}

export async function fetchBrandSignals(
  slug: string,
  params?: { type?: BrandSignalType; cursor?: number; limit?: number },
): Promise<PaginatedSignals> {
  return apiGet<PaginatedSignals>(
    `/api/v1/brands/${encodeURIComponent(slug)}/signals`,
    {
      type: params?.type,
      cursor: params?.cursor,
      limit: params?.limit,
    },
  );
}

export async function fetchBrandFunding(
  slug: string,
): Promise<FundingEventItem[]> {
  return apiGet<FundingEventItem[]>(
    `/api/v1/brands/${encodeURIComponent(slug)}/funding`,
  );
}

export async function fetchBrandLabTests(slug: string): Promise<LabTestItem[]> {
  return apiGet<LabTestItem[]>(
    `/api/v1/brands/${encodeURIComponent(slug)}/lab-tests`,
  );
}

export async function fetchBrandLegal(
  slug: string,
): Promise<LegalProceedingItem[]> {
  return apiGet<LegalProceedingItem[]>(
    `/api/v1/brands/${encodeURIComponent(slug)}/legal`,
  );
}

export async function fetchBrandSponsorships(
  slug: string,
): Promise<SponsorshipItem[]> {
  return apiGet<SponsorshipItem[]>(
    `/api/v1/brands/${encodeURIComponent(slug)}/sponsorships`,
  );
}

export async function fetchBrandDistributors(
  slug: string,
): Promise<DistributorItem[]> {
  return apiGet<DistributorItem[]>(
    `/api/v1/brands/${encodeURIComponent(slug)}/distributors`,
  );
}

export async function fetchBrandCompetitors(
  slug: string,
): Promise<CompetitorItem[]> {
  return apiGet<CompetitorItem[]>(
    `/api/v1/brands/${encodeURIComponent(slug)}/competitors`,
  );
}

export async function fetchBrandMedia(
  slug: string,
): Promise<MediaAppearanceItem[]> {
  return apiGet<MediaAppearanceItem[]>(
    `/api/v1/brands/${encodeURIComponent(slug)}/media`,
  );
}

// -- Write functions ----------------------------------------------------------

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
    getBrandPath(slug),
    body,
  );
}

export async function updateBrandProfile(
  slug: string,
  body: UpdateBrandProfileBody,
): Promise<void> {
  return apiMutate<UpdateBrandProfileBody, void>(
    "PUT",
    `/api/v1/brands/${encodeURIComponent(slug)}/profile`,
    body,
  );
}

export async function updateBrandSocial(
  slug: string,
  body: UpdateSocialHandlesBody,
): Promise<void> {
  return apiMutate<UpdateSocialHandlesBody, void>(
    "PUT",
    `/api/v1/brands/${encodeURIComponent(slug)}/social`,
    body,
  );
}

export async function updateBrandDomains(
  slug: string,
  body: UpdateDomainsBody,
): Promise<void> {
  return apiMutate<UpdateDomainsBody, void>(
    "PUT",
    `/api/v1/brands/${encodeURIComponent(slug)}/domains`,
    body,
  );
}

export async function deactivateBrand(slug: string): Promise<void> {
  return apiMutate<undefined, void>("DELETE", getBrandPath(slug));
}
