import type {
  BillItem,
  LocationBrandSummary,
  LocationsByState,
  PricingSnapshotItem,
  PricingSummaryItem,
  ProductItem,
  SentimentSnapshotItem,
  SentimentSummaryItem,
} from "../../types/api";
import { apiGet } from "./client";

export async function fetchProducts(): Promise<ProductItem[]> {
  return apiGet<ProductItem[]>("/api/v1/products", { limit: 50 });
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
