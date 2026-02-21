import type { SignalItem } from "../types/brands";
import { useBrandSignals } from "../hooks/use-dashboard-data";
import { ErrorState, LoadingState, formatDate } from "./dashboard-utils";

const CONTENT_TYPES = [
  "youtube_video",
  "tweet",
  "article",
  "blog_post",
] as const;
type ContentType = (typeof CONTENT_TYPES)[number];

const CONTENT_LABELS: Record<ContentType, string> = {
  youtube_video: "YouTube",
  tweet: "Tweets",
  article: "Articles",
  blog_post: "Blog Posts",
};

function YoutubeItem({ signal }: { signal: SignalItem }) {
  return (
    <li className="content-item content-item--youtube">
      {signal.image_url && (
        <img
          src={signal.image_url}
          alt={signal.title ?? "YouTube thumbnail"}
          className="content-thumbnail"
        />
      )}
      <div className="content-body">
        {signal.title && <p className="content-title">{signal.title}</p>}
        {signal.source_url && (
          <a
            href={signal.source_url}
            className="content-link"
            target="_blank"
            rel="noopener noreferrer"
          >
            Watch ↗
          </a>
        )}
      </div>
    </li>
  );
}

function TweetItem({ signal }: { signal: SignalItem }) {
  return (
    <li className="content-item content-item--tweet">
      <div className="content-body">
        {signal.title && <p className="content-title">{signal.title}</p>}
        {signal.summary && <p className="content-summary">{signal.summary}</p>}
        {signal.source_url && (
          <a
            href={signal.source_url}
            className="content-link"
            target="_blank"
            rel="noopener noreferrer"
          >
            View tweet ↗
          </a>
        )}
      </div>
    </li>
  );
}

function ArticleItem({ signal }: { signal: SignalItem }) {
  return (
    <li className="content-item content-item--article">
      <div className="content-body">
        {signal.title && <p className="content-title">{signal.title}</p>}
        <div className="content-meta">
          {signal.source_url && (
            <a
              href={signal.source_url}
              className="content-link"
              target="_blank"
              rel="noopener noreferrer"
            >
              Read ↗
            </a>
          )}
          <span className="content-date">
            {formatDate(signal.published_at ?? signal.collected_at)}
          </span>
        </div>
      </div>
    </li>
  );
}

function ContentGroup({
  type,
  signals,
}: {
  type: ContentType;
  signals: SignalItem[];
}) {
  if (signals.length === 0) return null;
  return (
    <section className="content-group">
      <h3 className="content-group-heading">{CONTENT_LABELS[type]}</h3>
      <ul className="content-list">
        {signals.map((s) => {
          if (type === "youtube_video")
            return <YoutubeItem key={s.public_id} signal={s} />;
          if (type === "tweet")
            return <TweetItem key={s.public_id} signal={s} />;
          return <ArticleItem key={s.public_id} signal={s} />;
        })}
      </ul>
    </section>
  );
}

export function BrandContentTab({ slug }: { slug: string }) {
  const { data, isLoading, error } = useBrandSignals(slug);

  if (isLoading) return <LoadingState label="content" />;
  if (error) return <ErrorState label="content" />;

  const allItems = data?.items ?? [];
  const byType = Object.fromEntries(
    CONTENT_TYPES.map((t) => [t, allItems.filter((s) => s.signal_type === t)]),
  ) as Record<ContentType, SignalItem[]>;

  const hasContent = CONTENT_TYPES.some((t) => byType[t].length > 0);
  if (!hasContent) {
    return <p className="panel-status">No content signals yet.</p>;
  }

  return (
    <div className="brand-content-tab">
      {CONTENT_TYPES.map((type) => (
        <ContentGroup key={type} type={type} signals={byType[type]} />
      ))}
    </div>
  );
}
