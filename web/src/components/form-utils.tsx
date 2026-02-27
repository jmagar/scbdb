/** Shared primitives for brand edit forms. */

import type { ReactNode } from "react";

type FormFieldProps = {
  label: string;
  error?: string;
  required?: boolean;
  children: ReactNode;
};

export function FormField({
  label,
  error,
  required,
  children,
}: FormFieldProps) {
  return (
    <div className="form-field">
      <label className="form-label">
        {label}
        {required && (
          <span className="form-required" aria-hidden="true">
            {" "}
            *
          </span>
        )}
      </label>
      {children}
      {error && (
        <span className="form-error" role="alert">
          {error}
        </span>
      )}
    </div>
  );
}

type FormSectionProps = {
  title: string;
  danger?: boolean;
  children: ReactNode;
};

export function FormSection({ title, danger, children }: FormSectionProps) {
  return (
    <section className={`form-section${danger ? " form-section--danger" : ""}`}>
      <div className="form-section-header">
        <h3 className="form-section-title">{title}</h3>
      </div>
      {children}
    </section>
  );
}

type SaveButtonProps = {
  isPending: boolean;
  isError: boolean;
  label?: string;
};

export function SaveButton({
  isPending,
  isError,
  label = "Save",
}: SaveButtonProps) {
  return (
    <div className="form-save-row">
      <button type="submit" className="btn-primary" disabled={isPending}>
        {isPending ? "Saving…" : label}
      </button>
      {isError && (
        <span className="form-error" role="alert">
          Save failed — check inputs and try again.
        </span>
      )}
    </div>
  );
}

export function isValidUrl(v: string): boolean {
  try {
    new URL(v);
    return true;
  } catch {
    return false;
  }
}
