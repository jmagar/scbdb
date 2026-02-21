import { useState, type FormEvent } from "react";
import type { BrandProfileDetail } from "../types/brands";
import { useUpdateBrandProfile } from "../hooks/use-dashboard-data";
import { FormField, FormSection, SaveButton } from "./form-utils";

type Props = {
  slug: string;
  profile: BrandProfileDetail | null;
};

export function BrandEditProfileForm({ slug, profile }: Props) {
  const [tagline, setTagline] = useState(profile?.tagline ?? "");
  const [description, setDescription] = useState(profile?.description ?? "");
  const [foundedYear, setFoundedYear] = useState(
    profile?.founded_year ? String(profile.founded_year) : "",
  );
  const [hqCity, setHqCity] = useState(profile?.hq_city ?? "");
  const [hqState, setHqState] = useState(profile?.hq_state ?? "");
  const [ceoName, setCeoName] = useState(profile?.ceo_name ?? "");
  const [fundingStage, setFundingStage] = useState(
    profile?.funding_stage ?? "",
  );
  const [employeeCount, setEmployeeCount] = useState(
    profile?.employee_count_approx ? String(profile.employee_count_approx) : "",
  );
  const [errors, setErrors] = useState<Record<string, string>>({});

  const mutation = useUpdateBrandProfile(slug);

  function validate(): boolean {
    const next: Record<string, string> = {};
    if (
      foundedYear &&
      (isNaN(Number(foundedYear)) || Number(foundedYear) < 1900)
    )
      next.founded_year = "Must be a valid year (≥ 1900)";
    if (employeeCount && isNaN(Number(employeeCount)))
      next.employee_count = "Must be a number";
    setErrors(next);
    return Object.keys(next).length === 0;
  }

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (!validate()) return;
    mutation.mutate({
      tagline: tagline.trim() || null,
      description: description.trim() || null,
      founded_year: foundedYear ? Number(foundedYear) : null,
      hq_city: hqCity.trim() || null,
      hq_state: hqState.trim() || null,
      ceo_name: ceoName.trim() || null,
      funding_stage: fundingStage.trim() || null,
      employee_count_approx: employeeCount ? Number(employeeCount) : null,
    });
  }

  return (
    <FormSection title="Brand Profile">
      <form onSubmit={handleSubmit}>
        <FormField label="Tagline">
          <input
            className="form-input"
            value={tagline}
            onChange={(e) => setTagline(e.target.value)}
          />
        </FormField>
        <FormField label="Description">
          <textarea
            className="form-textarea"
            value={description}
            rows={4}
            onChange={(e) => setDescription(e.target.value)}
          />
        </FormField>
        <div className="form-row-2">
          <FormField label="Founded Year" error={errors.founded_year}>
            <input
              className="form-input"
              value={foundedYear}
              placeholder="e.g. 2019"
              onChange={(e) => setFoundedYear(e.target.value)}
            />
          </FormField>
          <FormField label="HQ City">
            <input
              className="form-input"
              value={hqCity}
              onChange={(e) => setHqCity(e.target.value)}
            />
          </FormField>
          <FormField label="HQ State">
            <input
              className="form-input"
              value={hqState}
              placeholder="e.g. CA"
              onChange={(e) => setHqState(e.target.value)}
            />
          </FormField>
          <FormField label="CEO Name">
            <input
              className="form-input"
              value={ceoName}
              onChange={(e) => setCeoName(e.target.value)}
            />
          </FormField>
          <FormField label="Funding Stage">
            <input
              className="form-input"
              value={fundingStage}
              placeholder="e.g. Series A"
              onChange={(e) => setFundingStage(e.target.value)}
            />
          </FormField>
          <FormField
            label="Approx. Employee Count"
            error={errors.employee_count}
          >
            <input
              className="form-input"
              value={employeeCount}
              placeholder="e.g. 25"
              onChange={(e) => setEmployeeCount(e.target.value)}
            />
          </FormField>
        </div>
        <SaveButton isPending={mutation.isPending} isError={mutation.isError} />
      </form>
    </FormSection>
  );
}
