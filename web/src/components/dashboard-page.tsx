import { useMemo, useState } from "react";

import {
  useBills,
  usePricingSnapshots,
  usePricingSummary,
  useProducts,
  useSentimentSnapshots,
  useSentimentSummary,
} from "../hooks/use-dashboard-data";
import { PricingPanel } from "./pricing-panel";
import { ProductsPanel } from "./products-panel";
import { RegulatoryPanel } from "./regulatory-panel";
import { SentimentPanel } from "./sentiment-panel";

type DashboardTab = "products" | "pricing" | "regulatory" | "sentiment";
type DashboardPageProps = {
  initialTab?: DashboardTab;
};

export function DashboardPage({ initialTab = "products" }: DashboardPageProps) {
  const [activeTab, setActiveTab] = useState<DashboardTab>(initialTab);

  const products = useProducts();
  const pricingSummary = usePricingSummary();
  const pricingSnapshots = usePricingSnapshots();
  const bills = useBills();
  const sentimentSummary = useSentimentSummary();
  const sentimentSnapshots = useSentimentSnapshots();

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
        {
          key: "sentiment",
          label: "Sentiment",
          value: String(sentimentSummary.data?.length ?? 0),
        },
      ] as const,
    [
      bills.data?.length,
      pricingSummary.data?.length,
      products.data?.length,
      sentimentSummary.data?.length,
    ],
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
          <ProductsPanel
            isLoading={products.isLoading}
            isError={products.isError}
            data={products.data}
          />
        )}

        {activeTab === "pricing" && (
          <PricingPanel
            summary={pricingSummary}
            snapshots={pricingSnapshots}
            logosByBrand={pricingLogosByBrand}
          />
        )}

        {activeTab === "regulatory" && (
          <RegulatoryPanel
            isLoading={bills.isLoading}
            isError={bills.isError}
            data={bills.data}
          />
        )}

        {activeTab === "sentiment" && (
          <SentimentPanel
            summary={sentimentSummary}
            snapshots={sentimentSnapshots}
          />
        )}
      </section>
    </main>
  );
}
