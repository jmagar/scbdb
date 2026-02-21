import { type BillItem } from "../types/api";
import { ErrorState, LoadingState, formatDate } from "./dashboard-utils";

type Props = {
  isLoading: boolean;
  isError: boolean;
  data: BillItem[] | undefined;
};

export function RegulatoryPanel({ isLoading, isError, data }: Props) {
  return (
    <>
      <h2>Regulatory Timeline</h2>
      {isLoading && <LoadingState label="bills" />}
      {isError && <ErrorState label="bills" />}
      {!isLoading && !isError && (
        <div className="card-stack">
          {data?.map((bill) => (
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
  );
}
