import { useState, type FormEvent } from "react";
import type { BrandSocialHandleItem } from "../types/brands";
import { useUpdateBrandSocial } from "../hooks/use-dashboard-data";
import { FormSection, SaveButton } from "./form-utils";

const PLATFORMS = [
  "twitter",
  "instagram",
  "tiktok",
  "youtube",
  "facebook",
  "linkedin",
  "threads",
] as const;

type SocialEntry = { platform: string; handle: string };

type Props = {
  slug: string;
  handles: BrandSocialHandleItem[];
};

export function BrandEditSocialForm({ slug, handles }: Props) {
  const [entries, setEntries] = useState<SocialEntry[]>(() =>
    handles.length > 0
      ? handles.map((h) => ({ platform: h.platform, handle: h.handle }))
      : [{ platform: "twitter", handle: "" }],
  );

  const mutation = useUpdateBrandSocial(slug);

  function addEntry() {
    setEntries((prev) => [...prev, { platform: "twitter", handle: "" }]);
  }

  function removeEntry(index: number) {
    setEntries((prev) => prev.filter((_, i) => i !== index));
  }

  function updateEntry(index: number, field: keyof SocialEntry, value: string) {
    setEntries((prev) =>
      prev.map((e, i) => (i === index ? { ...e, [field]: value } : e)),
    );
  }

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    const handlesMap: Record<string, string> = {};
    for (const { platform, handle } of entries) {
      if (platform && handle.trim()) {
        handlesMap[platform] = handle.trim();
      }
    }
    mutation.mutate({ handles: handlesMap });
  }

  return (
    <FormSection title="Social Handles">
      <form onSubmit={handleSubmit}>
        <div className="social-entry-list">
          {entries.map((entry, i) => (
            <div key={i} className="social-entry-row">
              <select
                className="form-select-sm"
                value={entry.platform}
                onChange={(e) => updateEntry(i, "platform", e.target.value)}
              >
                {PLATFORMS.map((p) => (
                  <option key={p} value={p}>
                    {p}
                  </option>
                ))}
              </select>
              <input
                className="form-input"
                value={entry.handle}
                placeholder="handle or username"
                onChange={(e) => updateEntry(i, "handle", e.target.value)}
              />
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
          + Add handle
        </button>
        <SaveButton isPending={mutation.isPending} isError={mutation.isError} />
      </form>
    </FormSection>
  );
}
