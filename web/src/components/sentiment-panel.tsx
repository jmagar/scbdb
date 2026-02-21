import type { SentimentSnapshotItem, SentimentSummaryItem } from "../types/api";
import {
  ErrorState,
  LoadingState,
  formatDate,
  formatScore,
  scoreClass,
  scorePct,
  trimText,
} from "./dashboard-utils";
import {
  buildSentimentInsight,
  formatSignedDelta,
  sourceLabel,
  trendClass,
} from "./sentiment-utils";

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
              <strong>Reddit</strong>, <strong>brand newsroom posts</strong>,
              and optional <strong>Twitter/X</strong> via local CLI integration.
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
                    .map(
                      ([source, count]) => `${sourceLabel(source)} (${count})`,
                    )
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
