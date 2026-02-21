import { useQuery } from "@tanstack/react-query";

import {
  fetchBrand,
  fetchBrandCompetitors,
  fetchBrandDistributors,
  fetchBrandFunding,
  fetchBrandLabTests,
  fetchBrandLegal,
  fetchBrandMedia,
  fetchBrands,
  fetchBrandSignals,
  fetchBrandSponsorships,
} from "../lib/api/brands";

const STALE_TIME_MS = 60_000;

export function useBrands() {
  return useQuery({
    queryKey: ["brands"],
    queryFn: fetchBrands,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrand(slug: string | null) {
  return useQuery({
    queryKey: ["brands", slug],
    queryFn: () => fetchBrand(slug!),
    enabled: slug !== null,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandSignals(
  slug: string | null,
  params?: { type?: string; cursor?: number; limit?: number },
) {
  return useQuery({
    queryKey: ["brands", slug, "signals", params],
    queryFn: () => fetchBrandSignals(slug!, params),
    enabled: slug !== null,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandFunding(slug: string | null) {
  return useQuery({
    queryKey: ["brands", slug, "funding"],
    queryFn: () => fetchBrandFunding(slug!),
    enabled: slug !== null,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandLabTests(slug: string | null) {
  return useQuery({
    queryKey: ["brands", slug, "lab-tests"],
    queryFn: () => fetchBrandLabTests(slug!),
    enabled: slug !== null,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandLegal(slug: string | null) {
  return useQuery({
    queryKey: ["brands", slug, "legal"],
    queryFn: () => fetchBrandLegal(slug!),
    enabled: slug !== null,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandSponsorships(slug: string | null) {
  return useQuery({
    queryKey: ["brands", slug, "sponsorships"],
    queryFn: () => fetchBrandSponsorships(slug!),
    enabled: slug !== null,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandDistributors(slug: string | null) {
  return useQuery({
    queryKey: ["brands", slug, "distributors"],
    queryFn: () => fetchBrandDistributors(slug!),
    enabled: slug !== null,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandCompetitors(slug: string | null) {
  return useQuery({
    queryKey: ["brands", slug, "competitors"],
    queryFn: () => fetchBrandCompetitors(slug!),
    enabled: slug !== null,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandMedia(slug: string | null) {
  return useQuery({
    queryKey: ["brands", slug, "media"],
    queryFn: () => fetchBrandMedia(slug!),
    enabled: slug !== null,
    staleTime: STALE_TIME_MS,
  });
}
