import { useState } from "react";
import {
  CompetitorList,
  DistributorList,
  FundingList,
  LabTestList,
  LegalList,
  SponsorshipList,
} from "./brand-recon-lists";

type ReconSubTab =
  | "funding"
  | "lab-tests"
  | "legal"
  | "sponsorships"
  | "distributors"
  | "competitors";

const RECON_SUB_TABS: readonly ReconSubTab[] = [
  "funding",
  "lab-tests",
  "legal",
  "sponsorships",
  "distributors",
  "competitors",
] as const;

const SUB_TAB_LABELS: Record<ReconSubTab, string> = {
  funding: "Funding",
  "lab-tests": "Lab Tests",
  legal: "Legal / Regulatory",
  sponsorships: "Sponsorships",
  distributors: "Distributors",
  competitors: "Competitors",
};

export function BrandReconTab({ slug }: { slug: string }) {
  const [subTab, setSubTab] = useState<ReconSubTab>("funding");

  return (
    <div className="recon-tab">
      <div className="recon-sub-tabs">
        {RECON_SUB_TABS.map((tab) => (
          <button
            key={tab}
            type="button"
            className={`recon-sub-tab-btn${subTab === tab ? " is-active" : ""}`}
            onClick={() => setSubTab(tab)}
          >
            {SUB_TAB_LABELS[tab]}
          </button>
        ))}
      </div>
      <div className="recon-sub-content">
        {subTab === "funding" && <FundingList slug={slug} />}
        {subTab === "lab-tests" && <LabTestList slug={slug} />}
        {subTab === "legal" && <LegalList slug={slug} />}
        {subTab === "sponsorships" && <SponsorshipList slug={slug} />}
        {subTab === "distributors" && <DistributorList slug={slug} />}
        {subTab === "competitors" && <CompetitorList slug={slug} />}
      </div>
    </div>
  );
}
