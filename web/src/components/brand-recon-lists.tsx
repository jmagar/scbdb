import type { ReactNode } from "react";
import type {
  CompetitorItem,
  DistributorItem,
  FundingEventItem,
  LabTestItem,
  LegalProceedingItem,
  SponsorshipItem,
} from "../types/brands";
import {
  useBrandCompetitors,
  useBrandDistributors,
  useBrandFunding,
  useBrandLabTests,
  useBrandLegal,
  useBrandSponsorships,
} from "../hooks/use-dashboard-data";
import { ErrorState, LoadingState, formatDate } from "./dashboard-utils";

function formatAmountUsd(amount: number | null): string {
  if (amount === null) return "-";
  if (amount >= 1_000_000) return `$${(amount / 1_000_000).toFixed(1)}M`;
  if (amount >= 1_000) return `$${(amount / 1_000).toFixed(0)}K`;
  return `$${amount.toLocaleString()}`;
}

type ReconListProps<T> = {
  label: string;
  emptyMessage: string;
  isLoading: boolean;
  error: Error | null;
  data: T[] | undefined;
  renderItem: (item: T) => ReactNode;
  keyFn: (item: T) => string | number;
};

function ReconList<T>({
  label,
  emptyMessage,
  isLoading,
  error,
  data,
  renderItem,
  keyFn,
}: ReconListProps<T>) {
  if (isLoading) return <LoadingState label={label} />;
  if (error) return <ErrorState label={label} />;
  const items = data ?? [];
  if (items.length === 0) return <p className="panel-status">{emptyMessage}</p>;
  return (
    <ul className="recon-list">
      {items.map((item) => (
        <li key={keyFn(item)} className="recon-item">
          {renderItem(item)}
        </li>
      ))}
    </ul>
  );
}

export function FundingList({ slug }: { slug: string }) {
  const { data, isLoading, error } = useBrandFunding(slug);
  return (
    <ReconList<FundingEventItem>
      label="funding"
      emptyMessage="No funding events on record."
      isLoading={isLoading}
      error={error}
      data={data}
      keyFn={(item) => item.id}
      renderItem={(item) => (
        <>
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
        </>
      )}
    />
  );
}

export function LabTestList({ slug }: { slug: string }) {
  const { data, isLoading, error } = useBrandLabTests(slug);
  return (
    <ReconList<LabTestItem>
      label="lab tests"
      emptyMessage="No lab tests on record."
      isLoading={isLoading}
      error={error}
      data={data}
      keyFn={(item) => item.id}
      renderItem={(item) => (
        <>
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
        </>
      )}
    />
  );
}

export function LegalList({ slug }: { slug: string }) {
  const { data, isLoading, error } = useBrandLegal(slug);
  return (
    <ReconList<LegalProceedingItem>
      label="legal proceedings"
      emptyMessage="No legal proceedings on record."
      isLoading={isLoading}
      error={error}
      data={data}
      keyFn={(item) => item.id}
      renderItem={(item) => (
        <>
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
        </>
      )}
    />
  );
}

export function SponsorshipList({ slug }: { slug: string }) {
  const { data, isLoading, error } = useBrandSponsorships(slug);
  return (
    <ReconList<SponsorshipItem>
      label="sponsorships"
      emptyMessage="No sponsorships on record."
      isLoading={isLoading}
      error={error}
      data={data}
      keyFn={(item) => item.id}
      renderItem={(item) => (
        <>
          <div className="recon-item-header">
            <span className="recon-item-type">{item.entity_name}</span>
            <span className="recon-status-badge">{item.deal_type}</span>
            {item.announced_at && (
              <span className="recon-item-date">
                {formatDate(item.announced_at)}
              </span>
            )}
          </div>
          <p className="recon-item-detail">{item.entity_type}</p>
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
        </>
      )}
    />
  );
}

export function DistributorList({ slug }: { slug: string }) {
  const { data, isLoading, error } = useBrandDistributors(slug);
  return (
    <ReconList<DistributorItem>
      label="distributors"
      emptyMessage="No distributor relationships on record."
      isLoading={isLoading}
      error={error}
      data={data}
      keyFn={(item) => item.id}
      renderItem={(item) => (
        <>
          <div className="recon-item-header">
            <span className="recon-item-type">{item.distributor_name}</span>
            <span className="recon-status-badge">{item.channel_type}</span>
            <span className="recon-status-badge">{item.territory_type}</span>
          </div>
          {item.states && item.states.length > 0 && (
            <p className="recon-item-detail">{item.states.join(", ")}</p>
          )}
          {item.started_at && (
            <p className="recon-item-detail">
              Since {formatDate(item.started_at)}
            </p>
          )}
        </>
      )}
    />
  );
}

export function CompetitorList({ slug }: { slug: string }) {
  const { data, isLoading, error } = useBrandCompetitors(slug);
  return (
    <ReconList<CompetitorItem>
      label="competitors"
      emptyMessage="No competitor relationships on record."
      isLoading={isLoading}
      error={error}
      data={data}
      keyFn={(item) => item.id}
      renderItem={(item) => (
        <>
          <div className="recon-item-header">
            <span className="recon-item-type">{item.relationship_type}</span>
            <span className="recon-status-badge">
              {item.is_active ? "active" : "inactive"}
            </span>
            <span className="recon-item-date">
              {formatDate(item.first_observed_at)}
            </span>
          </div>
          {item.states && item.states.length > 0 && (
            <p className="recon-item-detail">{item.states.join(", ")}</p>
          )}
          {item.notes && <p className="recon-item-detail">{item.notes}</p>}
        </>
      )}
    />
  );
}
