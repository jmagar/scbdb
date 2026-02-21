import {
  type PricingSnapshotItem,
  type PricingSummaryItem,
} from "../types/api";
import {
  ErrorState,
  LoadingState,
  formatDate,
  formatMoney,
} from "./dashboard-utils";

type Props = {
  summary: {
    isLoading: boolean;
    isError: boolean;
    data: PricingSummaryItem[] | undefined;
  };
  snapshots: {
    isLoading: boolean;
    isError: boolean;
    data: PricingSnapshotItem[] | undefined;
  };
  logosByBrand: Map<string, string | undefined>;
};

export function PricingPanel({ summary, snapshots, logosByBrand }: Props) {
  return (
    <>
      <h2>Pricing Summary</h2>
      {summary.isLoading && <LoadingState label="pricing summary" />}
      {summary.isError && <ErrorState label="pricing summary" />}
      {!summary.isLoading && !summary.isError && (
        <div className="card-stack">
          {summary.data?.map((item) => (
            <article className="data-card" key={item.brand_slug}>
              {item.brand_logo_url ? (
                <img
                  className="brand-image"
                  src={item.brand_logo_url}
                  alt={`${item.brand_name} brand`}
                  loading="lazy"
                />
              ) : (
                <div
                  className="brand-image brand-image-empty"
                  aria-hidden="true"
                />
              )}
              <header>
                <h3>{item.brand_name}</h3>
                <span>{item.variant_count} variants</span>
              </header>
              <dl>
                <div>
                  <dt>Average</dt>
                  <dd>{formatMoney(item.avg_price)}</dd>
                </div>
                <div>
                  <dt>Range</dt>
                  <dd>
                    {formatMoney(item.min_price)} -{" "}
                    {formatMoney(item.max_price)}
                  </dd>
                </div>
                <div>
                  <dt>Updated</dt>
                  <dd>{formatDate(item.latest_capture_at)}</dd>
                </div>
              </dl>
            </article>
          ))}
        </div>
      )}

      <h3>Recent Snapshots</h3>
      {snapshots.isLoading && <LoadingState label="pricing snapshots" />}
      {snapshots.isError && <ErrorState label="pricing snapshots" />}
      {!snapshots.isLoading && !snapshots.isError && (
        <div
          className="mini-table"
          role="table"
          aria-label="recent-pricing-snapshots"
        >
          {snapshots.data?.slice(0, 8).map((item) => (
            <div
              className="mini-row"
              role="row"
              key={`${item.source_variant_id}-${item.captured_at}`}
            >
              <span className="mini-brand">
                {(item.brand_logo_url ?? logosByBrand.get(item.brand_slug)) ? (
                  <img
                    className="mini-brand-image"
                    src={
                      item.brand_logo_url ?? logosByBrand.get(item.brand_slug)
                    }
                    alt={`${item.brand_name} brand`}
                    loading="lazy"
                  />
                ) : (
                  <span className="mini-brand-fallback" aria-hidden="true" />
                )}
                <span>{item.brand_name}</span>
              </span>
              <strong>{formatMoney(item.price)}</strong>
              <span>{formatDate(item.captured_at)}</span>
            </div>
          ))}
        </div>
      )}
    </>
  );
}
