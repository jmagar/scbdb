import { useState, type FormEvent } from "react";
import { useUpdateBrandDomains } from "../hooks/use-dashboard-data";
import { FormSection, SaveButton, isValidUrl } from "./form-utils";

type Props = {
  slug: string;
  domains: string[];
};

export function BrandEditDomainsForm({ slug, domains }: Props) {
  const [entries, setEntries] = useState<string[]>(() =>
    domains.length > 0 ? [...domains] : [""],
  );
  const [errors, setErrors] = useState<Record<number, string>>({});

  const mutation = useUpdateBrandDomains(slug);

  function addEntry() {
    setEntries((prev) => [...prev, ""]);
  }

  function removeEntry(index: number) {
    setEntries((prev) => prev.filter((_, i) => i !== index));
    setErrors((prev) => {
      const next = { ...prev };
      delete next[index];
      return next;
    });
  }

  function updateEntry(index: number, value: string) {
    setEntries((prev) => prev.map((e, i) => (i === index ? value : e)));
    if (errors[index]) {
      setErrors((prev) => {
        const next = { ...prev };
        delete next[index];
        return next;
      });
    }
  }

  function validate(): boolean {
    const next: Record<number, string> = {};
    for (let i = 0; i < entries.length; i++) {
      const v = (entries[i] ?? "").trim();
      if (v && !isValidUrl(`https://${v}`) && !isValidUrl(v)) {
        next[i] = "Invalid domain";
      }
    }
    setErrors(next);
    return Object.keys(next).length === 0;
  }

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (!validate()) return;
    const cleaned = entries.map((d) => d.trim()).filter(Boolean);
    mutation.mutate({ domains: cleaned });
  }

  return (
    <FormSection title="Known Domains">
      <form onSubmit={handleSubmit}>
        <div className="domain-entry-list">
          {entries.map((entry, i) => (
            <div key={i} className="domain-entry-row">
              <input
                className={`form-input${errors[i] ? " is-error" : ""}`}
                value={entry}
                placeholder="e.g. drinkcann.com"
                onChange={(e) => updateEntry(i, e.target.value)}
              />
              {errors[i] && (
                <span className="form-error" role="alert">
                  {errors[i]}
                </span>
              )}
              <button
                type="button"
                className="btn-ghost-sm"
                onClick={() => removeEntry(i)}
                aria-label="Remove"
              >
                ×
              </button>
            </div>
          ))}
        </div>
        <button type="button" className="btn-ghost-sm" onClick={addEntry}>
          + Add domain
        </button>
        <SaveButton isPending={mutation.isPending} isError={mutation.isError} />
      </form>
    </FormSection>
  );
}
