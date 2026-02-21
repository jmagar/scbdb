import type {
  SentimentEvidence,
  SentimentSnapshotItem,
  SentimentSummaryItem,
} from "../types/api";

export type InsightModel = {
  positiveCount: number;
  negativeCount: number;
  neutralCount: number;
  totalSignals: number;
  marketAverage: number;
  leaders: SentimentSummaryItem[];
  laggards: SentimentSummaryItem[];
  highestSignalBrand: SentimentSummaryItem | null;
  deltaByBrand: Map<string, number>;
  freshestAt: string | null;
  sourceMix: Array<[string, number]>;
  evidence: SentimentEvidence[];
};

export function buildSentimentInsight(
  summary: SentimentSummaryItem[],
  snapshots: SentimentSnapshotItem[],
): InsightModel {
  const scored = summary.map((item) => ({
    item,
    score: parseScore(item.score),
  }));

  const positiveCount = scored.filter((x) => x.score > 0.05).length;
  const negativeCount = scored.filter((x) => x.score < -0.05).length;
  const neutralCount = scored.length - positiveCount - negativeCount;
  const totalSignals = summary.reduce(
    (sum, item) => sum + item.signal_count,
    0,
  );
  const marketAverage =
    scored.length === 0
      ? 0
      : scored.reduce((sum, x) => sum + x.score, 0) / scored.length;

  const sortedByScore = [...summary].sort(
    (a, b) => parseScore(b.score) - parseScore(a.score),
  );
  const leaders = sortedByScore.slice(0, 3);
  const laggards = [...sortedByScore].reverse().slice(0, 3);

  const highestSignalBrand =
    summary.length === 0
      ? null
      : ([...summary].sort((a, b) => b.signal_count - a.signal_count)[0] ??
        null);

  const byBrand = new Map<string, SentimentSnapshotItem[]>();
  for (const item of snapshots) {
    const rows = byBrand.get(item.brand_slug);
    if (rows) {
      rows.push(item);
    } else {
      byBrand.set(item.brand_slug, [item]);
    }
  }

  for (const rows of byBrand.values()) {
    rows.sort(
      (a, b) =>
        new Date(b.captured_at).getTime() - new Date(a.captured_at).getTime(),
    );
  }

  const deltaByBrand = new Map<string, number>();
  for (const item of summary) {
    const history = byBrand.get(item.brand_slug) ?? [];
    const currentTime = new Date(item.captured_at).getTime();
    const previous = history.find(
      (row) => new Date(row.captured_at).getTime() < currentTime,
    );
    if (!previous) {
      continue;
    }
    deltaByBrand.set(
      item.brand_slug,
      parseScore(item.score) - parseScore(previous.score),
    );
  }

  const freshestAt =
    [...summary, ...snapshots]
      .map((item) => item.captured_at)
      .sort((a, b) => new Date(b).getTime() - new Date(a).getTime())[0] ?? null;

  const sourceCounts = new Map<string, number>();
  const evidence: SentimentEvidence[] = [];
  const pushMetadata = (row: SentimentSummaryItem | SentimentSnapshotItem) => {
    const counts = row.metadata?.source_counts;
    if (counts) {
      for (const [source, count] of Object.entries(counts)) {
        sourceCounts.set(source, (sourceCounts.get(source) ?? 0) + count);
      }
    }
    const topSignals = row.metadata?.top_signals ?? [];
    for (const signal of topSignals) {
      evidence.push(signal);
    }
  };
  for (const row of summary) pushMetadata(row);
  for (const row of snapshots) pushMetadata(row);

  const evidenceByUrl = new Map<string, SentimentEvidence>();
  for (const row of evidence) {
    const existing = evidenceByUrl.get(row.url);
    if (!existing || Math.abs(row.score) > Math.abs(existing.score)) {
      evidenceByUrl.set(row.url, row);
    }
  }
  const topEvidence = [...evidenceByUrl.values()]
    .sort((a, b) => Math.abs(b.score) - Math.abs(a.score))
    .slice(0, 6);
  const sourceMix = [...sourceCounts.entries()].sort((a, b) => b[1] - a[1]);

  return {
    positiveCount,
    negativeCount,
    neutralCount,
    totalSignals,
    marketAverage,
    leaders,
    laggards,
    highestSignalBrand,
    deltaByBrand,
    freshestAt,
    sourceMix,
    evidence: topEvidence,
  };
}

export function parseScore(value: string): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function formatSignedDelta(value: number): string {
  const sign = value > 0 ? "+" : "";
  return `${sign}${value.toFixed(2)}`;
}

export function trendClass(value: number | null): string {
  if (value === null) return "";
  if (value > 0.01) return "sentiment-trend sentiment-trend--up";
  if (value < -0.01) return "sentiment-trend sentiment-trend--down";
  return "sentiment-trend sentiment-trend--flat";
}

export function sourceLabel(source: string): string {
  switch (source) {
    case "google_news":
      return "Google News";
    case "bing_news":
      return "Bing News";
    case "yahoo_news":
      return "Yahoo News";
    case "gdelt_news":
      return "GDELT News";
    case "brand_newsroom":
      return "Brand Newsroom";
    case "twitter_brand":
      return "Twitter Brand Posts";
    case "twitter_replies":
      return "Twitter Replies";
    case "reddit_post":
      return "Reddit Posts";
    case "reddit_comment":
      return "Reddit Comments";
    default:
      return source
        .split("_")
        .filter(Boolean)
        .map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
        .join(" ");
  }
}
