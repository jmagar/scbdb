import { useState } from "react";

import {
  useBillEvents,
  useBills,
  useBillTexts,
} from "../hooks/use-dashboard-data";
import { type BillItem } from "../types/api";
import { ErrorState, LoadingState, formatDate } from "./dashboard-utils";

function billStatusClass(status: string): string | null {
  switch (status) {
    case "passed":
    case "enrolled":
      return "bill-status-badge--passed";
    case "vetoed":
    case "failed":
      return "bill-status-badge--failed";
    case "engrossed":
      return "bill-status-badge--advancing";
    default:
      return null;
  }
}

function statusBadgeClass(status: string): string {
  const modifier = billStatusClass(status);
  return modifier ? `bill-status-badge ${modifier}` : "bill-status-badge";
}

type DetailProps = {
  bill: BillItem;
  onBack: () => void;
};

function BillDetail({ bill, onBack }: DetailProps) {
  const events = useBillEvents(bill.bill_id);
  const texts = useBillTexts(bill.bill_id);

  return (
    <div className="bill-detail">
      <button className="bill-back-btn" type="button" onClick={onBack}>
        ← Bills
      </button>

      <div className="bill-detail-header">
        <div className="bill-detail-meta">
          <span className="bill-jurisdiction">{bill.jurisdiction}</span>
          <span className="bill-session">{bill.session ?? ""}</span>
        </div>
        <h2 className="bill-detail-number">{bill.bill_number}</h2>
        <p className="bill-detail-title">{bill.title}</p>
        <div className="bill-detail-status-row">
          <span className={statusBadgeClass(bill.status)}>{bill.status}</span>
          {bill.status_date && (
            <span className="bill-status-date">
              {formatDate(bill.status_date)}
            </span>
          )}
          {/* Only show the generic source link when no versioned texts are available */}
          {bill.source_url &&
            (!texts.data || texts.data.length === 0) &&
            !texts.isLoading && (
              <a
                className="bill-source-link"
                href={bill.source_url}
                target="_blank"
                rel="noopener noreferrer"
              >
                Read full text ↗
              </a>
            )}
        </div>
      </div>

      {/* Versioned text links (Introduced, Engrossed, etc.) */}
      {texts.data && texts.data.length > 0 && (
        <div className="bill-texts">
          {texts.data.map((t, i) =>
            t.url ? (
              <a
                key={i}
                className="bill-text-link"
                href={t.url}
                target="_blank"
                rel="noopener noreferrer"
              >
                {t.text_type} text ↗
              </a>
            ) : null,
          )}
        </div>
      )}

      {bill.summary && (
        <section className="bill-summary" aria-label="bill summary">
          <h3 className="bill-section-title">Summary</h3>
          <p className="bill-summary-text">{bill.summary}</p>
        </section>
      )}

      {!bill.summary &&
        (!texts.data || texts.data.length === 0) &&
        !texts.isLoading && (
          <p className="panel-status">No description available.</p>
        )}

      <h3 className="bill-section-title">Activity</h3>

      {events.isLoading && <LoadingState label="events" />}
      {events.isError && <ErrorState label="events" />}

      {!events.isLoading && !events.isError && (
        <ol className="bill-timeline" aria-label="bill activity timeline">
          {(events.data ?? []).length === 0 && (
            <li className="bill-timeline-empty">No recorded activity.</li>
          )}
          {(events.data ?? []).map((ev, i) => (
            // Events have no unique id from the API; index is stable for this read-only list
            <li className="bill-timeline-item" key={i}>
              <div className="bill-timeline-dot" />
              <div className="bill-timeline-body">
                <div className="bill-timeline-row">
                  {ev.event_date && (
                    <span className="bill-event-date">
                      {formatDate(ev.event_date)}
                    </span>
                  )}
                  {ev.event_type && (
                    <span className="bill-event-type">{ev.event_type}</span>
                  )}
                  {ev.chamber && (
                    <span className="bill-event-chamber">{ev.chamber}</span>
                  )}
                </div>
                <p className="bill-event-description">{ev.description}</p>
                {ev.source_url && (
                  <a
                    className="bill-event-link"
                    href={ev.source_url}
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    Source ↗
                  </a>
                )}
              </div>
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}

export function RegulatoryPanel({ isLoading, isError, data }: Props) {
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const selectedBill = selectedId
    ? ((data ?? []).find((b) => b.bill_id === selectedId) ?? null)
    : null;

  if (selectedBill) {
    return (
      <BillDetail bill={selectedBill} onBack={() => setSelectedId(null)} />
    );
  }

  return (
    <>
      <h2>Regulatory Timeline</h2>
      {isLoading && <LoadingState label="bills" />}
      {isError && <ErrorState label="bills" />}
      {!isLoading && !isError && data?.length === 0 && (
        <p className="panel-status">No bills tracked yet.</p>
      )}
      {!isLoading && !isError && data && data.length > 0 && (
        <div className="card-stack">
          {data.map((bill) => (
            <button
              key={bill.bill_id}
              className="data-card bill-card"
              type="button"
              onClick={() => setSelectedId(bill.bill_id)}
              aria-label={`View ${bill.jurisdiction} ${bill.bill_number}`}
            >
              <header>
                <h3>
                  {bill.jurisdiction} {bill.bill_number}
                </h3>
                <span className={statusBadgeClass(bill.status)}>
                  {bill.status}
                </span>
              </header>
              <span className="bill-card-title">{bill.title}</span>
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
              <span className="bill-card-chevron" aria-hidden="true">
                ›
              </span>
            </button>
          ))}
        </div>
      )}
    </>
  );
}
