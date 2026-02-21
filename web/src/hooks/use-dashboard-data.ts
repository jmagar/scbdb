import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  fetchBillEvents,
  fetchBillTexts,
  fetchBills,
  fetchBrandCompetitors,
  fetchBrandDistributors,
  fetchBrandFunding,
  fetchBrandLabTests,
  fetchBrandLegal,
  fetchBrandMedia,
  fetchBrandProfile,
  fetchBrandSignals,
  fetchBrandSponsorships,
  fetchBrands,
  fetchLocationPins,
  fetchLocationsByState,
  fetchLocationsSummary,
  fetchPricingSnapshots,
  fetchPricingSummary,
  fetchProducts,
  fetchSentimentSnapshots,
  fetchSentimentSummary,
} from "../lib/api/dashboard";
import {
  createBrand,
  deactivateBrand,
  updateBrandDomains,
  updateBrandMeta,
  updateBrandProfile,
  updateBrandSocial,
} from "../lib/api/brands";
import type {
  CreateBrandBody,
  CreateBrandResponse,
  UpdateBrandMetaBody,
  UpdateBrandProfileBody,
  UpdateDomainsBody,
  UpdateSocialHandlesBody,
} from "../types/brands";
import type { BrandSignalType } from "../types/api";

const STALE_TIME_MS = 60_000;

export function useProducts() {
  return useQuery({
    queryKey: ["products"],
    queryFn: fetchProducts,
    staleTime: STALE_TIME_MS,
  });
}

export function usePricingSummary() {
  return useQuery({
    queryKey: ["pricing-summary"],
    queryFn: fetchPricingSummary,
    staleTime: STALE_TIME_MS,
  });
}

export function usePricingSnapshots() {
  return useQuery({
    queryKey: ["pricing-snapshots"],
    queryFn: fetchPricingSnapshots,
    staleTime: STALE_TIME_MS,
  });
}

export function useBills() {
  return useQuery({
    queryKey: ["bills"],
    queryFn: fetchBills,
    staleTime: STALE_TIME_MS,
  });
}

export function useBillEvents(billId: string | null) {
  return useQuery({
    queryKey: ["bill-events", billId],
    queryFn: () => fetchBillEvents(billId!),
    enabled: billId !== null,
    staleTime: STALE_TIME_MS,
  });
}

export function useBillTexts(billId: string | null) {
  return useQuery({
    queryKey: ["bill-texts", billId],
    queryFn: () => fetchBillTexts(billId!),
    enabled: billId !== null,
    staleTime: STALE_TIME_MS,
  });
}

export function useSentimentSummary() {
  return useQuery({
    queryKey: ["sentiment-summary"],
    queryFn: fetchSentimentSummary,
    staleTime: STALE_TIME_MS,
  });
}

export function useSentimentSnapshots() {
  return useQuery({
    queryKey: ["sentiment-snapshots"],
    queryFn: fetchSentimentSnapshots,
    staleTime: STALE_TIME_MS,
  });
}

export function useLocationsSummary() {
  return useQuery({
    queryKey: ["locations-summary"],
    queryFn: fetchLocationsSummary,
    staleTime: STALE_TIME_MS,
  });
}

export function useLocationsByState() {
  return useQuery({
    queryKey: ["locations-by-state"],
    queryFn: fetchLocationsByState,
    staleTime: STALE_TIME_MS,
  });
}

export function useLocationPins() {
  return useQuery({
    queryKey: ["location-pins"],
    queryFn: fetchLocationPins,
    staleTime: STALE_TIME_MS,
  });
}

// ── Brand Intelligence Layer ──────────────────────────────────────────────────

export function useBrands() {
  return useQuery({
    queryKey: ["brands"],
    queryFn: fetchBrands,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandProfile(slug: string) {
  return useQuery({
    queryKey: ["brand", slug],
    queryFn: () => fetchBrandProfile(slug),
    enabled: !!slug,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandSignals(
  slug: string,
  type?: BrandSignalType,
  cursor?: number,
) {
  return useQuery({
    queryKey: ["brand-signals", slug, type, cursor],
    queryFn: () =>
      fetchBrandSignals(
        slug,
        type ? { type, limit: 50, cursor } : { limit: 50, cursor },
      ),
    enabled: !!slug,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandFunding(slug: string) {
  return useQuery({
    queryKey: ["brand-funding", slug],
    queryFn: () => fetchBrandFunding(slug),
    enabled: !!slug,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandDistributors(slug: string) {
  return useQuery({
    queryKey: ["brand-distributors", slug],
    queryFn: () => fetchBrandDistributors(slug),
    enabled: !!slug,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandLabTests(slug: string) {
  return useQuery({
    queryKey: ["brand-lab-tests", slug],
    queryFn: () => fetchBrandLabTests(slug),
    enabled: !!slug,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandLegal(slug: string) {
  return useQuery({
    queryKey: ["brand-legal", slug],
    queryFn: () => fetchBrandLegal(slug),
    enabled: !!slug,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandSponsorships(slug: string) {
  return useQuery({
    queryKey: ["brand-sponsorships", slug],
    queryFn: () => fetchBrandSponsorships(slug),
    enabled: !!slug,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandCompetitors(slug: string) {
  return useQuery({
    queryKey: ["brand-competitors", slug],
    queryFn: () => fetchBrandCompetitors(slug),
    enabled: !!slug,
    staleTime: STALE_TIME_MS,
  });
}

export function useBrandMedia(slug: string) {
  return useQuery({
    queryKey: ["brand-media", slug],
    queryFn: () => fetchBrandMedia(slug),
    enabled: !!slug,
    staleTime: STALE_TIME_MS,
  });
}

// ── Mutation hooks ────────────────────────────────────────────────────────────

export function useCreateBrand() {
  const queryClient = useQueryClient();
  return useMutation<CreateBrandResponse, Error, CreateBrandBody>({
    mutationFn: (body) => createBrand(body),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["brands"] });
    },
  });
}

export function useUpdateBrandMeta(slug: string) {
  const queryClient = useQueryClient();
  return useMutation<void, Error, UpdateBrandMetaBody>({
    mutationFn: (body) => updateBrandMeta(slug, body),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["brands"] });
      void queryClient.invalidateQueries({ queryKey: ["brand", slug] });
    },
  });
}

export function useUpdateBrandProfile(slug: string) {
  const queryClient = useQueryClient();
  return useMutation<void, Error, UpdateBrandProfileBody>({
    mutationFn: (body) => updateBrandProfile(slug, body),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["brand", slug] });
    },
  });
}

export function useUpdateBrandSocial(slug: string) {
  const queryClient = useQueryClient();
  return useMutation<void, Error, UpdateSocialHandlesBody>({
    mutationFn: (body) => updateBrandSocial(slug, body),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["brand", slug] });
    },
  });
}

export function useUpdateBrandDomains(slug: string) {
  const queryClient = useQueryClient();
  return useMutation<void, Error, UpdateDomainsBody>({
    mutationFn: (body) => updateBrandDomains(slug, body),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["brand", slug] });
    },
  });
}

export function useDeactivateBrand(slug: string) {
  const queryClient = useQueryClient();
  return useMutation<void, Error, void>({
    mutationFn: () => deactivateBrand(slug),
    onSuccess: () => {
      queryClient.removeQueries({ queryKey: ["brand", slug] });
      void queryClient.invalidateQueries({ queryKey: ["brands"] });
    },
  });
}
