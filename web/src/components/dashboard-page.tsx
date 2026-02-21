import { useMemo, useState } from "react";

import {
  useBills,
  useLocationsByState,
  useLocationsSummary,
  usePricingSnapshots,
  usePricingSummary,
  useProducts,
  useSentimentSnapshots,
  useSentimentSummary,
} from "../hooks/use-dashboard-data";
import { LocationsPanel } from "./locations-panel";
import { PricingPanel } from "./pricing-panel";
import { ProductsPanel } from "./products-panel";
import { RegulatoryPanel } from "./regulatory-panel";
import { SentimentPanel } from "./sentiment-panel";

type DashboardTab =
  | "products"
  | "pricing"
  | "regulatory"
  | "sentiment"
  | "locations";
type DashboardPageProps = {
  initialTab?: DashboardTab;
};

function IconBox() {
  return (
    <svg
      width="13"
      height="13"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M13.5 4.5L8 2 2.5 4.5v7L8 14l5.5-2.5v-7z" />
      <path d="M8 2v12" />
      <path d="M13.5 4.5L8 7 2.5 4.5" />
    </svg>
  );
}

function IconTrending() {
  return (
    <svg
      width="13"
      height="13"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <polyline points="1 11 5 7 9 9.5 15 3" />
      <polyline points="11 3 15 3 15 7" />
    </svg>
  );
}

function IconScale() {
  return (
    <svg
      width="13"
      height="13"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <line x1="8" y1="1.5" x2="8" y2="14.5" />
      <path d="M2.5 5.5L8 8l5.5-2.5" />
      <path d="M2.5 5.5 1 10.5h3l1.5-5" />
      <path d="M13.5 5.5 15 10.5h-3l-1.5-5" />
    </svg>
  );
}

function IconActivity() {
  return (
    <svg
      width="13"
      height="13"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <polyline points="1 8.5 4 8.5 6.5 2.5 9.5 14 12 8.5 15 8.5" />
    </svg>
  );
}

function IconMapPin() {
  return (
    <svg
      width="13"
      height="13"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M8 1.5C5.51 1.5 3.5 3.51 3.5 6c0 3.75 4.5 8.5 4.5 8.5s4.5-4.75 4.5-8.5c0-2.49-2.01-4.5-4.5-4.5z" />
      <circle cx="8" cy="6" r="1.5" />
    </svg>
  );
}

export function DashboardPage({ initialTab = "products" }: DashboardPageProps) {
  const [activeTab, setActiveTab] = useState<DashboardTab>(initialTab);

  const products = useProducts();
  const pricingSummary = usePricingSummary();
  const pricingSnapshots = usePricingSnapshots();
  const bills = useBills();
  const sentimentSummary = useSentimentSummary();
  const sentimentSnapshots = useSentimentSnapshots();
  const locationsSummary = useLocationsSummary();
  const locationsByState = useLocationsByState();

  const tabStats = useMemo(
    () =>
      [
        {
          key: "products",
          label: "Products",
          icon: <IconBox />,
          value: String(products.data?.length ?? 0),
        },
        {
          key: "pricing",
          label: "Pricing",
          icon: <IconTrending />,
          value: String(pricingSummary.data?.length ?? 0),
        },
        {
          key: "regulatory",
          label: "Bills",
          icon: <IconScale />,
          value: String(bills.data?.length ?? 0),
        },
        {
          key: "sentiment",
          label: "Sentiment",
          icon: <IconActivity />,
          value: String(sentimentSummary.data?.length ?? 0),
        },
        {
          key: "locations",
          label: "Locations",
          icon: <IconMapPin />,
          value: String(
            (locationsSummary.data ?? []).reduce(
              (acc, b) => acc + b.active_count,
              0,
            ),
          ),
        },
      ] as const,
    [
      bills.data?.length,
      locationsSummary.data,
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
      <header className="hero">
        <img
          src="/sc-header.png"
          alt="Southern Crown CBD DB"
          className="hero-logo"
        />
      </header>

      <nav className="hero-nav" aria-label="Main navigation">
        <a href="#/brands" className="hero-nav-link">
          Brands
        </a>
      </nav>

      <section className="stats-grid" aria-label="dashboard-summary">
        {tabStats.map((stat) => (
          <button
            key={stat.key}
            className={
              activeTab === stat.key ? "stat-card is-active" : "stat-card"
            }
            type="button"
            onClick={() => setActiveTab(stat.key)}
          >
            <div className="stat-card-top">
              <span className="stat-icon">{stat.icon}</span>
              <span className="stat-label">{stat.label}</span>
            </div>
            <strong className="stat-value">{stat.value}</strong>
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

        {activeTab === "locations" && (
          <LocationsPanel
            summary={locationsSummary}
            byState={locationsByState}
          />
        )}
      </section>
    </main>
  );
}
