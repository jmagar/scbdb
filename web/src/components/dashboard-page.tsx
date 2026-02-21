import { useMemo, useState } from "react";

import {
  useBills,
  usePricingSnapshots,
  usePricingSummary,
  useProducts,
} from "../hooks/use-dashboard-data";

type DashboardTab = "products" | "pricing" | "regulatory";
type DashboardPageProps = {
  initialTab?: DashboardTab;
};

function formatMoney(value: string): string {
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

function formatDate(value: string | null): string {
  if (!value) {
    return "-";
  }
  return new Date(value).toLocaleDateString();
}

function LoadingState({ label }: { label: string }) {
  return <p className="panel-status">Loading {label}...</p>;
}

function ErrorState({ label }: { label: string }) {
  return (
    <p className="panel-status panel-status-error">Failed to load {label}.</p>
  );
}

export function DashboardPage({ initialTab = "products" }: DashboardPageProps) {
  const [activeTab, setActiveTab] = useState<DashboardTab>(initialTab);

  const products = useProducts();
  const pricingSummary = usePricingSummary();
  const pricingSnapshots = usePricingSnapshots();
  const bills = useBills();

  const tabStats = useMemo(
    () =>
      [
        {
          key: "products",
          label: "Products",
          value: String(products.data?.length ?? 0),
        },
        {
          key: "pricing",
          label: "Pricing",
          value: String(pricingSummary.data?.length ?? 0),
        },
        {
          key: "regulatory",
          label: "Bills",
          value: String(bills.data?.length ?? 0),
        },
      ] as const,
    [bills.data?.length, pricingSummary.data?.length, products.data?.length],
  );
  const pricingLogosByBrand = useMemo(
    () =>
      new Map(
        (pricingSummary.data ?? []).map((item) => [
          item.brand_slug,
          item.brand_logo_url ?? undefined,
        ]),
      ),
    [pricingSummary.data],
  );

  return (
    <main className="app-shell">
      <section className="hero">
        <p className="kicker">SCBDB Command View</p>
        <h1>API Dashboard</h1>
        <p>
          Track product movement, live pricing, and bill activity from one
          mobile-ready control surface.
        </p>
      </section>

      <section className="stats-grid" aria-label="dashboard-summary">
        {tabStats.map((stat) => (
          <button
            key={stat.key}
            className={`stat-card ${activeTab === stat.key ? "is-active" : ""}`}
            type="button"
            onClick={() => setActiveTab(stat.key)}
          >
            <span>{stat.label}</span>
            <strong>{stat.value}</strong>
          </button>
        ))}
      </section>

      <section className="panel" aria-live="polite">
        {activeTab === "products" && (
          <>
            <h2>Product Catalog</h2>
            {products.isLoading && <LoadingState label="products" />}
            {products.isError && <ErrorState label="products" />}
            {!products.isLoading && !products.isError && (
              <div className="card-stack">
                {products.data?.map((item) => (
                  <article className="data-card" key={item.product_id}>
                    {item.primary_image_url || item.brand_logo_url ? (
                      <img
                        className="product-image"
                        src={
                          item.primary_image_url ??
                          item.brand_logo_url ??
                          undefined
                        }
                        alt={`${item.product_name} product`}
                        loading="lazy"
                      />
                    ) : (
                      <div
                        className="product-image product-image-empty"
                        aria-hidden="true"
                      />
                    )}
                    <header>
                      <h3>{item.product_name}</h3>
                      <span>{item.brand_name}</span>
                    </header>
                    <dl>
                      <div>
                        <dt>Variants</dt>
                        <dd>{item.variant_count}</dd>
                      </div>
                      <div>
                        <dt>Latest Price</dt>
                        <dd>
                          {item.latest_price
                            ? formatMoney(item.latest_price)
                            : "-"}
                        </dd>
                      </div>
                      <div>
                        <dt>Tier</dt>
                        <dd>{item.tier}</dd>
                      </div>
                    </dl>
                  </article>
                ))}
              </div>
            )}
          </>
        )}

        {activeTab === "pricing" && (
          <>
            <h2>Pricing Summary</h2>
            {pricingSummary.isLoading && (
              <LoadingState label="pricing summary" />
            )}
            {pricingSummary.isError && <ErrorState label="pricing summary" />}
            {!pricingSummary.isLoading && !pricingSummary.isError && (
              <div className="card-stack">
                {pricingSummary.data?.map((item) => (
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
            {pricingSnapshots.isLoading && (
              <LoadingState label="pricing snapshots" />
            )}
            {pricingSnapshots.isError && (
              <ErrorState label="pricing snapshots" />
            )}
            {!pricingSnapshots.isLoading && !pricingSnapshots.isError && (
              <div
                className="mini-table"
                role="table"
                aria-label="recent-pricing-snapshots"
              >
                {pricingSnapshots.data?.slice(0, 8).map((item) => (
                  <div
                    className="mini-row"
                    role="row"
                    key={`${item.source_variant_id}-${item.captured_at}`}
                  >
                    <span className="mini-brand">
                      {(item.brand_logo_url ??
                      pricingLogosByBrand.get(item.brand_slug)) ? (
                        <img
                          className="mini-brand-image"
                          src={
                            item.brand_logo_url ??
                            pricingLogosByBrand.get(item.brand_slug)
                          }
                          alt={`${item.brand_name} brand`}
                          loading="lazy"
                        />
                      ) : (
                        <span
                          className="mini-brand-fallback"
                          aria-hidden="true"
                        />
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
        )}

        {activeTab === "regulatory" && (
          <>
            <h2>Regulatory Timeline</h2>
            {bills.isLoading && <LoadingState label="bills" />}
            {bills.isError && <ErrorState label="bills" />}
            {!bills.isLoading && !bills.isError && (
              <div className="card-stack">
                {bills.data?.map((bill) => (
                  <article className="data-card" key={bill.bill_id}>
                    <header>
                      <h3>
                        {bill.jurisdiction} {bill.bill_number}
                      </h3>
                      <span>{bill.status}</span>
                    </header>
                    <p>{bill.title}</p>
                    <dl>
                      <div>
                        <dt>Events</dt>
                        <dd>{bill.event_count}</dd>
                      </div>
                      <div>
                        <dt>Last Action</dt>
                        <dd>{formatDate(bill.last_action_date)}</dd>
                      </div>
                    </dl>
                  </article>
                ))}
              </div>
            )}
          </>
        )}
      </section>
    </main>
  );
}
