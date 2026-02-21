import {
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

export function SentimentPanel({ summary, snapshots }: Props) {
  return (
    <>
      <h2>Market Sentiment</h2>
      {summary.isLoading && <LoadingState label="sentiment summary" />}
      {summary.isError && <ErrorState label="sentiment summary" />}
      {!summary.isLoading && !summary.isError && (
        <div className="card-stack">
          {summary.data?.map((item) => (
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
                  <dt>Updated</dt>
                  <dd>{formatDate(item.captured_at)}</dd>
                </div>
              </dl>
            </article>
          ))}
        </div>
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
          {snapshots.data?.slice(0, 8).map((item) => (
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
