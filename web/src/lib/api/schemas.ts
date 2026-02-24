import { z } from "zod";

export const ApiEnvelopeSchema = z.object({ data: z.unknown() });

export const BrandSchema = z
  .object({
    id: z.number(),
    name: z.string(),
    slug: z.string(),
    relationship: z.string(),
    tier: z.number(),
  })
  .passthrough();

export const ProductSchema = z
  .object({
    id: z.number(),
    brand_slug: z.string(),
    title: z.string(),
  })
  .passthrough();
