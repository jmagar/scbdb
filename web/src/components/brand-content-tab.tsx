import type { MediaAppearanceItem } from "../types/brands";
import { useBrandMedia } from "../hooks/use-dashboard-data";
import { ErrorState, LoadingState, formatDate } from "./dashboard-utils";

function MediaItem({ item }: { item: MediaAppearanceItem }) {
  return (
    <li className="content-item content-item--media">
      <div className="content-body">
        <div className="content-item-header">
          <span className="content-outlet">{item.outlet_name}</span>
          <span className="content-type-badge">{item.appearance_type}</span>
          {item.aired_at && (
            <span className="content-date">{formatDate(item.aired_at)}</span>
          )}
        </div>
        {item.title && <p className="content-title">{item.title}</p>}
        {item.host_or_author && (
          <p className="content-author">{item.host_or_author}</p>
        )}
        {item.source_url && (
          <a
            href={item.source_url}
            className="content-link"
            target="_blank"
            rel="noopener noreferrer"
          >
            View ↗
          </a>
        )}
      </div>
    </li>
  );
}

export function BrandContentTab({ slug }: { slug: string }) {
  const { data, isLoading, error } = useBrandMedia(slug);

  if (isLoading) return <LoadingState label="media appearances" />;
  if (error) return <ErrorState label="media appearances" />;

  const items = data ?? [];
  if (items.length === 0) {
    return <p className="panel-status">No media appearances on record.</p>;
  }

  return (
    <div className="brand-content-tab">
      <ul className="content-list">
        {items.map((item) => (
          <MediaItem key={item.id} item={item} />
        ))}
      </ul>
    </div>
  );
}
