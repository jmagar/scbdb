import type {
  BillEventItem,
  BillItem,
  BillTextItem,
  LocationBrandSummary,
  LocationPin,
  LocationsByState,
  PricingSnapshotItem,
  PricingSummaryItem,
  ProductItem,
  SentimentSnapshotItem,
  SentimentSummaryItem,
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
  return apiGet<BillEventItem[]>(
    `/api/v1/bills/${encodeURIComponent(billId)}/events`,
  );
}

export async function fetchBillTexts(billId: string): Promise<BillTextItem[]> {
  return apiGet<BillTextItem[]>(
    `/api/v1/bills/${encodeURIComponent(billId)}/texts`,
  );
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
