import { useQuery } from "@tanstack/react-query";

import {
  fetchBills,
  fetchLocationsByState,
  fetchLocationsSummary,
  fetchPricingSnapshots,
  fetchPricingSummary,
  fetchProducts,
  fetchSentimentSnapshots,
  fetchSentimentSummary,
} from "../lib/api/dashboard";

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
