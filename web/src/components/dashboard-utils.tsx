const USD = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  maximumFractionDigits: 2,
});

export function formatMoney(value: string): string {
  const parsed = Number(value);
  if (Number.isNaN(parsed)) return value;
  return USD.format(parsed);
}

export function formatDate(value: string | null): string {
  if (!value) {
    return "-";
  }
  // ISO date-only strings (YYYY-MM-DD) are parsed as UTC midnight by spec.
  // Appending T00:00:00 forces local-time parsing so the displayed calendar
  // day matches the written date regardless of the viewer's timezone.
  const normalized = value.length === 10 ? `${value}T00:00:00` : value;
  const date = new Date(normalized);
  if (Number.isNaN(date.getTime())) return "—";
  return date.toLocaleDateString();
}

export function formatScore(value: string): string {
  const parsed = Number(value);
  if (Number.isNaN(parsed)) return value;
  const sign = parsed > 0 ? "+" : "";
  return `${sign}${parsed.toFixed(2)}`;
}

export function scoreClass(value: string): "positive" | "negative" | "neutral" {
  const parsed = Number(value);
  if (Number.isNaN(parsed) || Math.abs(parsed) < 0.05) return "neutral";
  return parsed > 0 ? "positive" : "negative";
}

export function scorePct(value: string): number {
  const parsed = Number(value);
  if (Number.isNaN(parsed)) return 50;
  return ((Math.max(-1, Math.min(1, parsed)) + 1) / 2) * 100;
}

/** Trims and normalises whitespace; truncates at 120 chars with an ellipsis. */
export function trimText(value: string): string {
  const normalized = value.replace(/\s+/g, " ").trim();
  if (normalized.length <= 120) {
    return normalized;
  }
  return `${normalized.slice(0, 117)}...`;
}

export function LoadingState({ label }: { label: string }) {
  return (
    <div
      className="skeleton-grid"
      role="status"
      aria-label={`Loading ${label}`}
      aria-busy="true"
    >
      {[0, 1, 2].map((i) => (
        <div
          key={i}
          className="skeleton-card"
          style={{ animationDelay: `${i * 75}ms` }}
        >
          <div className="skeleton-line skeleton-line--title" />
          <div className="skeleton-line skeleton-line--sub" />
          <div className="skeleton-dl">
            <div className="skeleton-line skeleton-line--stat" />
            <div className="skeleton-line skeleton-line--stat" />
            <div className="skeleton-line skeleton-line--stat" />
          </div>
        </div>
      ))}
    </div>
  );
}

export function ErrorState({ label }: { label: string }) {
  return (
    <p className="panel-status panel-status-error">Failed to load {label}.</p>
  );
}
