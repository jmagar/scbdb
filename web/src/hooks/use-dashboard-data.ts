import { useQuery } from "@tanstack/react-query";

import {
  fetchBillEvents,
  fetchBillTexts,
  fetchBills,
  fetchLocationPins,
  fetchLocationsByState,
  fetchLocationsSummary,
  fetchPricingSnapshots,
  fetchPricingSummary,
  fetchProducts,
  fetchSentimentSnapshots,
  fetchSentimentSummary,
} from "../lib/api/dashboard";

// Re-export brand hooks for backward compatibility
export {
  useBrandCompetitors,
  useBrandDistributors,
  useBrandFunding,
  useBrandLabTests,
  useBrandLegal,
  useBrandMedia,
  useBrandProfile,
  useBrandSignals,
  useBrandSponsorships,
  useBrands,
  useCreateBrand,
  useDeactivateBrand,
  useUpdateBrandDomains,
  useUpdateBrandMeta,
  useUpdateBrandProfile,
  useUpdateBrandSocial,
} from "./use-brand-data";

const STALE_TIME_MS = 60_000;

export function useProducts(enabled = true) {
  return useQuery({
    queryKey: ["products"],
    queryFn: fetchProducts,
    staleTime: STALE_TIME_MS,
    enabled,
  });
}

export function usePricingSummary(enabled = true) {
  return useQuery({
    queryKey: ["pricing-summary"],
    queryFn: fetchPricingSummary,
    staleTime: STALE_TIME_MS,
    enabled,
  });
}

export function usePricingSnapshots(enabled = true) {
  return useQuery({
    queryKey: ["pricing-snapshots"],
    queryFn: fetchPricingSnapshots,
    staleTime: STALE_TIME_MS,
    enabled,
  });
}

export function useBills(enabled = true) {
  return useQuery({
    queryKey: ["bills"],
    queryFn: fetchBills,
    staleTime: STALE_TIME_MS,
    enabled,
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

export function useSentimentSummary(enabled = true) {
  return useQuery({
    queryKey: ["sentiment-summary"],
    queryFn: fetchSentimentSummary,
    staleTime: STALE_TIME_MS,
    enabled,
  });
}

export function useSentimentSnapshots(enabled = true) {
  return useQuery({
    queryKey: ["sentiment-snapshots"],
    queryFn: fetchSentimentSnapshots,
    staleTime: STALE_TIME_MS,
    enabled,
  });
}

export function useLocationsSummary(enabled = true) {
  return useQuery({
    queryKey: ["locations-summary"],
    queryFn: fetchLocationsSummary,
    staleTime: STALE_TIME_MS,
    enabled,
  });
}

export function useLocationsByState(enabled = true) {
  return useQuery({
    queryKey: ["locations-by-state"],
    queryFn: fetchLocationsByState,
    staleTime: STALE_TIME_MS,
    enabled,
  });
}

export function useLocationPins(enabled = true) {
  return useQuery({
    queryKey: ["location-pins"],
    queryFn: fetchLocationPins,
    staleTime: STALE_TIME_MS,
    enabled,
  });
}
