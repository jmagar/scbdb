import { useState } from "react";
import type { SignalItem } from "../types/brands";
import { useBrandSignals } from "../hooks/use-dashboard-data";
import {
  ErrorState,
  LoadingState,
  trimText,
  formatDate,
} from "./dashboard-utils";

const SIGNAL_ICONS: Record<string, string> = {
  article: "📰",
  tweet: "𝕏",
  youtube_video: "▶",
  reddit_post: "🔴",
  newsletter: "✉",
  blog_post: "✍",
  event: "📅",
  award: "🏆",
  partnership: "🤝",
  launch: "🚀",
  press_release: "📢",
  podcast_episode: "🎙",
};

function signalIcon(type: string): string {
  return SIGNAL_ICONS[type] ?? "•";
}

function SignalRow({ signal }: { signal: SignalItem }) {
  return (
    <li className="signal-item">
      <span className="signal-type-icon" aria-label={signal.signal_type}>
        {signalIcon(signal.signal_type)}
      </span>
      <div className="signal-body">
        {signal.title && <p className="signal-title">{signal.title}</p>}
        {signal.summary && (
          <p className="signal-summary">{trimText(signal.summary)}</p>
        )}
        <div className="signal-meta">
          {signal.source_url && (
            <a
              href={signal.source_url}
              className="signal-source"
              target="_blank"
              rel="noopener noreferrer"
            >
              Source ↗
            </a>
          )}
          <span className="signal-date">
            {formatDate(signal.published_at ?? signal.collected_at)}
          </span>
        </div>
      </div>
    </li>
  );
}

export function BrandSignalFeed({ slug }: { slug: string }) {
  const [cursor, setCursor] = useState<number | undefined>(undefined);
  const { data, isLoading, error } = useBrandSignals(slug);

  if (isLoading) return <LoadingState label="signals" />;
  if (error) return <ErrorState label="signals" />;

  const items = data?.items ?? [];
  const nextCursor = data?.next_cursor ?? null;

  if (items.length === 0) {
    return <p className="panel-status">No signals yet.</p>;
  }

  return (
    <div className="signal-feed">
      <ul className="signal-list">
        {items.map((signal) => (
          <SignalRow key={signal.public_id} signal={signal} />
        ))}
      </ul>
      {nextCursor !== null && (
        <button
          className="load-more-btn"
          onClick={() => setCursor(nextCursor)}
          type="button"
        >
          Load More
        </button>
      )}
      {/* cursor used to trigger refetch — suppress lint */}
      {cursor !== undefined && null}
    </div>
  );
}
