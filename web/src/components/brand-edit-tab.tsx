import { useState } from "react";
import type { BrandProfileResponse } from "../types/brands";
import { useDeactivateBrand } from "../hooks/use-dashboard-data";
import { ROUTES } from "../lib/routes";
import { FormSection } from "./form-utils";
import { BrandEditMetaForm } from "./brand-edit-meta-form";
import { BrandEditProfileForm } from "./brand-edit-profile-form";
import { BrandEditSocialForm } from "./brand-edit-social-form";
import { BrandEditDomainsForm } from "./brand-edit-domains-form";

type Props = {
  slug: string;
  brand: BrandProfileResponse;
};

function DeactivateSection({ slug, name }: { slug: string; name: string }) {
  const [confirming, setConfirming] = useState(false);
  const mutation = useDeactivateBrand(slug);

  function handleDeactivate() {
    mutation.mutate(undefined, {
      onSuccess: () => {
        window.location.hash = ROUTES.brands;
      },
    });
  }

  return (
    <FormSection title="Danger Zone" danger>
      {!confirming ? (
        <button
          type="button"
          className="btn-danger-ghost"
          onClick={() => setConfirming(true)}
        >
          Deactivate brand
        </button>
      ) : (
        <div className="deactivate-confirm">
          <p>
            Deactivate <strong>{name}</strong>? This will hide the brand from
            all views and cannot be undone from the UI.
          </p>
          <div className="form-save-row">
            <button
              type="button"
              className="btn-danger"
              disabled={mutation.isPending}
              onClick={handleDeactivate}
            >
              {mutation.isPending ? "Deactivating…" : "Yes, Deactivate"}
            </button>
            <button
              type="button"
              className="btn-ghost-sm"
              onClick={() => setConfirming(false)}
            >
              Cancel
            </button>
          </div>
          {mutation.isError && (
            <span className="form-error" role="alert">
              Deactivation failed — try again.
            </span>
          )}
        </div>
      )}
    </FormSection>
  );
}

export function BrandEditTab({ slug, brand }: Props) {
  return (
    <div className="brand-edit-tab">
      <BrandEditMetaForm key={slug} slug={slug} brand={brand} />
      <BrandEditProfileForm key={slug} slug={slug} profile={brand.profile} />
      <BrandEditSocialForm
        key={slug}
        slug={slug}
        handles={brand.social_handles}
      />
      <BrandEditDomainsForm key={slug} slug={slug} domains={brand.domains} />
      <DeactivateSection slug={slug} name={brand.name} />
    </div>
  );
}
