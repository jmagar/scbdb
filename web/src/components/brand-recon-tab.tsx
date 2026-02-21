import { useState } from "react";
import type {
  FundingEventItem,
  LabTestItem,
  LegalProceedingItem,
} from "../types/brands";
import {
  useBrandFunding,
  useBrandLabTests,
  useBrandLegal,
} from "../hooks/use-dashboard-data";
import { ErrorState, LoadingState, formatDate } from "./dashboard-utils";

type ReconSubTab = "funding" | "lab-tests" | "legal";

function formatAmountUsd(amount: number | null): string {
  if (amount === null) return "-";
  if (amount >= 1_000_000) {
    return `$${(amount / 1_000_000).toFixed(1)}M`;
  }
  if (amount >= 1_000) {
    return `$${(amount / 1_000).toFixed(0)}K`;
  }
  return `$${amount.toLocaleString()}`;
}

function FundingList({ slug }: { slug: string }) {
  const { data, isLoading, error } = useBrandFunding(slug);
  if (isLoading) return <LoadingState label="funding" />;
  if (error) return <ErrorState label="funding" />;
  const items = data ?? [];
  if (items.length === 0)
    return <p className="panel-status">No funding events on record.</p>;
  return (
    <ul className="recon-list">
      {items.map((item: FundingEventItem) => (
        <li key={item.id} className="recon-item">
          <div className="recon-item-header">
            <span className="recon-item-type">{item.event_type}</span>
            <span className="recon-item-amount">
              {formatAmountUsd(item.amount_usd)}
            </span>
            <span className="recon-item-date">
              {formatDate(item.announced_at)}
            </span>
          </div>
          {item.investors && item.investors.length > 0 && (
            <p className="recon-item-detail">{item.investors.join(", ")}</p>
          )}
          {item.source_url && (
            <a
              href={item.source_url}
              className="recon-item-link"
              target="_blank"
              rel="noopener noreferrer"
            >
              Source ↗
            </a>
          )}
        </li>
      ))}
    </ul>
  );
}

function LabTestList({ slug }: { slug: string }) {
  const { data, isLoading, error } = useBrandLabTests(slug);
  if (isLoading) return <LoadingState label="lab tests" />;
  if (error) return <ErrorState label="lab tests" />;
  const items = data ?? [];
  if (items.length === 0)
    return <p className="panel-status">No lab tests on record.</p>;
  return (
    <ul className="recon-list">
      {items.map((item: LabTestItem) => (
        <li key={item.id} className="recon-item">
          <div className="recon-item-header">
            <span className="recon-item-type">{item.lab_name ?? "Lab"}</span>
            <span className="recon-passed">
              {item.passed === true ? "✅" : item.passed === false ? "❌" : "—"}
            </span>
            <span className="recon-item-date">
              {formatDate(item.test_date)}
            </span>
          </div>
          <div className="recon-item-detail">
            {item.thc_mg_actual && <span>THC: {item.thc_mg_actual}mg</span>}
            {item.cbd_mg_actual && <span> CBD: {item.cbd_mg_actual}mg</span>}
          </div>
          {item.report_url && (
            <a
              href={item.report_url}
              className="recon-item-link"
              target="_blank"
              rel="noopener noreferrer"
            >
              Report ↗
            </a>
          )}
        </li>
      ))}
    </ul>
  );
}

function LegalList({ slug }: { slug: string }) {
  const { data, isLoading, error } = useBrandLegal(slug);
  if (isLoading) return <LoadingState label="legal proceedings" />;
  if (error) return <ErrorState label="legal proceedings" />;
  const items = data ?? [];
  if (items.length === 0)
    return <p className="panel-status">No legal proceedings on record.</p>;
  return (
    <ul className="recon-list">
      {items.map((item: LegalProceedingItem) => (
        <li key={item.id} className="recon-item">
          <div className="recon-item-header">
            <span className="recon-item-type">{item.proceeding_type}</span>
            <span className="recon-status-badge">{item.status}</span>
            <span className="recon-item-date">{formatDate(item.filed_at)}</span>
          </div>
          <p className="recon-item-title">{item.title}</p>
          {item.jurisdiction && (
            <p className="recon-item-detail">{item.jurisdiction}</p>
          )}
          {item.source_url && (
            <a
              href={item.source_url}
              className="recon-item-link"
              target="_blank"
              rel="noopener noreferrer"
            >
              Source ↗
            </a>
          )}
        </li>
      ))}
    </ul>
  );
}

export function BrandReconTab({ slug }: { slug: string }) {
  const [subTab, setSubTab] = useState<ReconSubTab>("funding");

  return (
    <div className="recon-tab">
      <div className="recon-sub-tabs">
        {(["funding", "lab-tests", "legal"] as const).map((tab) => (
          <button
            key={tab}
            type="button"
            className={`recon-sub-tab-btn${subTab === tab ? " is-active" : ""}`}
            onClick={() => setSubTab(tab)}
          >
            {tab === "funding"
              ? "Funding"
              : tab === "lab-tests"
                ? "Lab Tests"
                : "Legal / Regulatory"}
          </button>
        ))}
      </div>
      <div className="recon-sub-content">
        {subTab === "funding" && <FundingList slug={slug} />}
        {subTab === "lab-tests" && <LabTestList slug={slug} />}
        {subTab === "legal" && <LegalList slug={slug} />}
      </div>
    </div>
  );
}
