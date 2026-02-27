import {
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";

import {
  createBrand,
  deactivateBrand,
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

export function useBrandSignals(slug: string, type?: BrandSignalType) {
  return useInfiniteQuery({
    queryKey: ["brand-signals", slug, type],
    queryFn: ({ pageParam }) =>
      fetchBrandSignals(
        slug,
        type
          ? { type, limit: 50, cursor: pageParam }
          : { limit: 50, cursor: pageParam },
      ),
    getNextPageParam: (lastPage) => lastPage.next_cursor ?? undefined,
    initialPageParam: undefined as number | undefined,
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

// -- Mutation hooks -----------------------------------------------------------

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
