import {
  type SentimentEvidence,
  type SentimentSnapshotItem,
  type SentimentSummaryItem,
} from "../types/api";
import {
  ErrorState,
  LoadingState,
  formatDate,
  formatScore,
  scoreClass,
  scorePct,
} from "./dashboard-utils";

type Props = {
  summary: {
    isLoading: boolean;
    isError: boolean;
    data: SentimentSummaryItem[] | undefined;
  };
  snapshots: {
    isLoading: boolean;
    isError: boolean;
    data: SentimentSnapshotItem[] | undefined;
  };
};

type InsightModel = {
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

export function SentimentPanel({ summary, snapshots }: Props) {
  const summaryItems = summary.data ?? [];
  const snapshotItems = snapshots.data ?? [];
  const insight = buildSentimentInsight(summaryItems, snapshotItems);

  return (
    <>
      <h2>Market Sentiment</h2>
      {summary.isLoading && <LoadingState label="sentiment summary" />}
      {summary.isError && <ErrorState label="sentiment summary" />}
      {!summary.isLoading && !summary.isError && (
        <>
          <div className="sentiment-context-grid">
            <article className="context-card">
              <h3>Market Posture</h3>
              <p>
                Avg score{" "}
                <strong>{formatSignedDelta(insight.marketAverage)}</strong>
              </p>
              <p>
                {insight.positiveCount} positive, {insight.negativeCount}{" "}
                negative, {insight.neutralCount} neutral
              </p>
            </article>

            <article className="context-card">
              <h3>Most Positive</h3>
              {insight.leaders.length === 0 && <p>No data</p>}
              {insight.leaders.map((item) => (
                <p key={`leader-${item.brand_slug}`}>
                  {item.brand_name} <strong>{formatScore(item.score)}</strong>
                </p>
              ))}
            </article>

            <article className="context-card">
              <h3>Most Negative</h3>
              {insight.laggards.length === 0 && <p>No data</p>}
              {insight.laggards.map((item) => (
                <p key={`laggard-${item.brand_slug}`}>
                  {item.brand_name} <strong>{formatScore(item.score)}</strong>
                </p>
              ))}
            </article>

            <article className="context-card">
              <h3>Signal Depth</h3>
              <p>
                Total signals <strong>{insight.totalSignals}</strong>
              </p>
              {insight.highestSignalBrand ? (
                <p>
                  Highest: {insight.highestSignalBrand.brand_name} ({" "}
                  {insight.highestSignalBrand.signal_count})
                </p>
              ) : (
                <p>No data</p>
              )}
            </article>
          </div>

          <article
            className="sentiment-transparency"
            aria-label="sentiment-transparency"
          >
            <h3>Data Transparency</h3>
            <p>
              Source signals are collected from multiple channels, currently
              including <strong>Google News RSS</strong>,{" "}
              <strong>Bing News RSS</strong>, <strong>Yahoo News RSS</strong>,{" "}
              <strong>Reddit</strong>, and optional <strong>Twitter/X</strong>{" "}
              via local CLI integration.
            </p>
            <p>
              Score scale: <strong>-1.00 to +1.00</strong>. Neutral band:{" "}
              <strong>-0.05 to +0.05</strong>.
              <br />
              Signal count is the number of documents scored for each brand
              snapshot.
            </p>
            <p>
              Freshest snapshot:{" "}
              <strong>{formatDate(insight.freshestAt)}</strong>
            </p>
            <p>
              Source mix:{" "}
              {insight.sourceMix.length === 0
                ? "unknown"
                : insight.sourceMix
                    .map(([source, count]) => `${source} (${count})`)
                    .join(", ")}
            </p>
            {insight.evidence.length > 0 && (
              <>
                <p>Sample evidence:</p>
                <ul className="sentiment-evidence-list">
                  {insight.evidence.map((row, idx) => (
                    <li key={`${row.url}-${idx}`}>
                      <a href={row.url} target="_blank" rel="noreferrer">
                        [{row.source}] {formatSignedDelta(row.score)}{" "}
                        {trimText(row.text_preview)}
                      </a>
                    </li>
                  ))}
                </ul>
              </>
            )}
          </article>

          <div className="card-stack">
            {summaryItems.map((item) => {
              const trend = insight.deltaByBrand.get(item.brand_slug) ?? null;
              return (
                <article className="data-card" key={item.brand_slug}>
                  <header>
                    <h3>{item.brand_name}</h3>
                    <span
                      className={`sentiment-badge sentiment-badge--${scoreClass(item.score)}`}
                    >
                      {formatScore(item.score)}
                    </span>
                  </header>
                  <div className="sentiment-meter">
                    <div
                      className="sentiment-meter-indicator"
                      style={{ left: `${scorePct(item.score)}%` }}
                    />
                  </div>
                  <dl>
                    <div>
                      <dt>Signals</dt>
                      <dd>{item.signal_count}</dd>
                    </div>
                    <div>
                      <dt>Momentum</dt>
                      <dd className={trendClass(trend)}>
                        {trend === null ? "n/a" : formatSignedDelta(trend)}
                      </dd>
                    </div>
                    <div>
                      <dt>Updated</dt>
                      <dd>{formatDate(item.captured_at)}</dd>
                    </div>
                  </dl>
                </article>
              );
            })}
          </div>
        </>
      )}

      <h3>Recent Runs</h3>
      {snapshots.isLoading && <LoadingState label="recent sentiment runs" />}
      {snapshots.isError && <ErrorState label="recent sentiment runs" />}
      {!snapshots.isLoading && !snapshots.isError && (
        <div
          className="mini-table"
          role="table"
          aria-label="recent-sentiment-snapshots"
        >
          {snapshotItems.slice(0, 8).map((item) => (
            <div
              className="mini-row"
              role="row"
              key={`${item.brand_slug}-${item.captured_at}`}
            >
              <span>{item.brand_name}</span>
              <strong
                className={`sentiment-badge sentiment-badge--${scoreClass(item.score)}`}
              >
                {formatScore(item.score)}
              </strong>
              <span>{item.signal_count} sig</span>
              <span>{formatDate(item.captured_at)}</span>
            </div>
          ))}
        </div>
      )}
    </>
  );
}

function buildSentimentInsight(
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

function parseScore(value: string): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function formatSignedDelta(value: number): string {
  const sign = value > 0 ? "+" : "";
  return `${sign}${value.toFixed(2)}`;
}

function trendClass(value: number | null): string {
  if (value === null) return "";
  if (value > 0.01) return "sentiment-trend sentiment-trend--up";
  if (value < -0.01) return "sentiment-trend sentiment-trend--down";
  return "sentiment-trend sentiment-trend--flat";
}

function trimText(value: string): string {
  const normalized = value.replace(/\\s+/g, " ").trim();
  if (normalized.length <= 120) {
    return normalized;
  }
  return `${normalized.slice(0, 117)}...`;
}
