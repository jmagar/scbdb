export function formatMoney(value: string): string {
  const parsed = Number(value);
  if (Number.isNaN(parsed)) {
    return value;
  }

  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 2,
  }).format(parsed);
}

export function formatDate(value: string | null): string {
  if (!value) {
    return "-";
  }
  return new Date(value).toLocaleDateString();
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
  return ((parsed + 1) / 2) * 100;
}

export function LoadingState({ label }: { label: string }) {
  return <p className="panel-status">Loading {label}...</p>;
}

export function ErrorState({ label }: { label: string }) {
  return (
    <p className="panel-status panel-status-error">Failed to load {label}.</p>
  );
}
