import { useState, useRef, useEffect, type FormEvent } from "react";
import { useCreateBrand } from "../hooks/use-dashboard-data";
import { FormField, SaveButton } from "./form-utils";

type Props = {
  onCreated: (slug: string) => void;
};

export function BrandCreateDialog({ onCreated }: Props) {
  const [expanded, setExpanded] = useState(false);
  const [name, setName] = useState("");
  const [relationship, setRelationship] = useState<"portfolio" | "competitor">(
    "competitor",
  );
  const [tier, setTier] = useState<"1" | "2" | "3">("3");
  const [errors, setErrors] = useState<Record<string, string>>({});

  const mutation = useCreateBrand();
  const nameRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (expanded) nameRef.current?.focus();
  }, [expanded]);

  function reset() {
    setName("");
    setRelationship("competitor");
    setTier("3");
    setErrors({});
    setExpanded(false);
  }

  function validate(): boolean {
    const next: Record<string, string> = {};
    if (!name.trim()) next.name = "Name is required";
    setErrors(next);
    return Object.keys(next).length === 0;
  }

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (!validate()) return;
    mutation.mutate(
      {
        name: name.trim(),
        relationship,
        tier: Number(tier) as 1 | 2 | 3,
      },
      {
        onSuccess: (data) => {
          reset();
          onCreated(data.slug);
        },
      },
    );
  }

  if (!expanded) {
    return (
      <div className="create-dialog">
        <button
          type="button"
          className="btn-primary"
          onClick={() => setExpanded(true)}
        >
          + New Brand
        </button>
      </div>
    );
  }

  return (
    <div className="create-dialog">
      <div className="form-section">
        <div className="form-section-header">
          <h3 className="form-section-title">New Brand</h3>
          <button
            type="button"
            className="btn-ghost-sm"
            onClick={reset}
            aria-label="Cancel"
          >
            ×
          </button>
        </div>
        <form onSubmit={handleSubmit}>
          <div className="form-row-2">
            <FormField label="Name" required error={errors.name}>
              <input
                ref={nameRef}
                className="form-input"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </FormField>
            <FormField label="Relationship" required>
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
            <FormField label="Tier" required>
              <select
                className="form-select-sm"
                value={tier}
                onChange={(e) => setTier(e.target.value as "1" | "2" | "3")}
              >
                <option value="1">1</option>
                <option value="2">2</option>
                <option value="3">3</option>
              </select>
            </FormField>
          </div>
          <SaveButton
            isPending={mutation.isPending}
            isError={mutation.isError}
            label="Create Brand"
          />
        </form>
      </div>
    </div>
  );
}
