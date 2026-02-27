import { useState, type FormEvent } from "react";
import type { BrandProfileResponse } from "../types/brands";
import { useUpdateBrandMeta } from "../hooks/use-dashboard-data";
import { FormField, FormSection, SaveButton, isValidUrl } from "./form-utils";

type Props = {
  slug: string;
  brand: BrandProfileResponse;
};

export function BrandEditMetaForm({ slug, brand }: Props) {
  const [name, setName] = useState(brand.name);
  const [relationship, setRelationship] = useState<"portfolio" | "competitor">(
    brand.relationship as "portfolio" | "competitor",
  );
  const [tier, setTier] = useState<1 | 2 | 3>(brand.tier as 1 | 2 | 3);
  const [domain, setDomain] = useState(brand.domain ?? "");
  const [shopUrl, setShopUrl] = useState(brand.shop_url ?? "");
  const [locatorUrl, setLocatorUrl] = useState(brand.store_locator_url ?? "");
  const [twitterHandle, setTwitterHandle] = useState(
    brand.twitter_handle ?? "",
  );
  const [notes, setNotes] = useState(brand.notes ?? "");
  const [errors, setErrors] = useState<Record<string, string>>({});

  const mutation = useUpdateBrandMeta(slug);

  function validate(): boolean {
    const next: Record<string, string> = {};
    if (!name.trim()) next.name = "Name is required";
    if (relationship !== "portfolio" && relationship !== "competitor")
      next.relationship = "Must be portfolio or competitor";
    if (tier !== 1 && tier !== 2 && tier !== 3)
      next.tier = "Must be 1, 2, or 3";
    if (shopUrl && !isValidUrl(shopUrl)) next.shop_url = "Must be a valid URL";
    if (locatorUrl && !isValidUrl(locatorUrl))
      next.store_locator_url = "Must be a valid URL";
    setErrors(next);
    return Object.keys(next).length === 0;
  }

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (!validate()) return;
    mutation.mutate({
      name: name.trim(),
      relationship,
      tier,
      domain: domain.trim() || null,
      shop_url: shopUrl.trim() || null,
      store_locator_url: locatorUrl.trim() || null,
      twitter_handle: twitterHandle.trim() || null,
      notes: notes.trim() || null,
    });
  }

  return (
    <FormSection title="Core Metadata">
      <form onSubmit={handleSubmit}>
        <div className="form-row-2">
          <FormField label="Name" required error={errors.name}>
            <input
              className="form-input"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </FormField>
          <FormField label="Relationship" required error={errors.relationship}>
            <select
              className="form-select"
              value={relationship}
              onChange={(e) =>
                setRelationship(e.target.value as "portfolio" | "competitor")
              }
            >
              <option value="portfolio">Portfolio</option>
              <option value="competitor">Competitor</option>
            </select>
          </FormField>
          <FormField label="Tier" required error={errors.tier}>
            <select
              className="form-select-sm"
              value={tier}
              onChange={(e) => setTier(Number(e.target.value) as 1 | 2 | 3)}
            >
              <option value="1">1</option>
              <option value="2">2</option>
              <option value="3">3</option>
            </select>
          </FormField>
          <FormField label="Primary Domain" error={errors.domain}>
            <input
              className="form-input"
              value={domain}
              placeholder="e.g. drinkcann.com"
              onChange={(e) => setDomain(e.target.value)}
            />
          </FormField>
          <FormField label="Shop URL" error={errors.shop_url}>
            <input
              className="form-input"
              value={shopUrl}
              placeholder="https://…"
              onChange={(e) => setShopUrl(e.target.value)}
            />
          </FormField>
          <FormField label="Store Locator URL" error={errors.store_locator_url}>
            <input
              className="form-input"
              value={locatorUrl}
              placeholder="https://…"
              onChange={(e) => setLocatorUrl(e.target.value)}
            />
          </FormField>
          <FormField label="Twitter Handle" error={errors.twitter_handle}>
            <input
              className="form-input"
              value={twitterHandle}
              placeholder="e.g. drinkcann"
              onChange={(e) => setTwitterHandle(e.target.value)}
            />
          </FormField>
        </div>
        <FormField label="Notes" error={errors.notes}>
          <textarea
            className="form-textarea"
            value={notes}
            rows={3}
            onChange={(e) => setNotes(e.target.value)}
          />
        </FormField>
        <SaveButton isPending={mutation.isPending} isError={mutation.isError} />
      </form>
    </FormSection>
  );
}
