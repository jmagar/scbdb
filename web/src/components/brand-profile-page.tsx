import { useState } from "react";
import type {
  BrandProfileDetail,
  BrandSocialHandleItem,
} from "../types/brands";
import { useBrandProfile } from "../hooks/use-dashboard-data";
import { ErrorState, LoadingState } from "./dashboard-utils";
import { BrandSignalFeed } from "./brand-signal-feed";
import { BrandContentTab } from "./brand-content-tab";
import { BrandReconTab } from "./brand-recon-tab";
import { BrandEditTab } from "./brand-edit-tab";

type ProfileTab = "feed" | "content" | "recon" | "edit";

function MetaItem({ label, value }: { label: string; value: string | number }) {
  return (
    <span className="brand-meta-item">
      <span className="brand-meta-label">{label}</span>
      <span className="brand-meta-value">{value}</span>
    </span>
  );
}

function ProfileMeta({ profile }: { profile: BrandProfileDetail }) {
  const items: Array<{ label: string; value: string | number }> = [];
  if (profile.founded_year)
    items.push({ label: "Founded", value: profile.founded_year });
  if (profile.hq_city && profile.hq_state) {
    items.push({
      label: "HQ",
      value: `${profile.hq_city}, ${profile.hq_state}`,
    });
  } else if (profile.hq_city) {
    items.push({ label: "HQ", value: profile.hq_city });
  }
  if (profile.parent_company)
    items.push({ label: "Parent", value: profile.parent_company });
  if (profile.funding_stage)
    items.push({ label: "Stage", value: profile.funding_stage });
  if (items.length === 0) return null;
  return (
    <div className="brand-meta-row">
      {items.map((item) => (
        <MetaItem key={item.label} label={item.label} value={item.value} />
      ))}
    </div>
  );
}

function SocialLinks({ handles }: { handles: BrandSocialHandleItem[] }) {
  if (handles.length === 0) return null;
  return (
    <div className="social-links">
      {handles.map((handle) => (
        <a
          key={`${handle.platform}-${handle.handle}`}
          href={handle.profile_url ?? "#"}
          className="social-link"
          target="_blank"
          rel="noopener noreferrer"
        >
          {handle.platform}
        </a>
      ))}
    </div>
  );
}

export function BrandProfilePage({ slug }: { slug: string }) {
  const [activeTab, setActiveTab] = useState<ProfileTab>("feed");
  const { data: brand, isLoading, error } = useBrandProfile(slug);

  if (isLoading) return <LoadingState label="brand profile" />;
  if (error || !brand) return <ErrorState label="brand profile" />;

  return (
    <div className="brand-profile">
      <div className="page-header">
        <a href="#/brands" className="back-link">
          ← Brands
        </a>
      </div>

      <div className="brand-profile-header">
        {brand.logo_url && (
          <img
            src={brand.logo_url}
            alt={brand.name}
            className="brand-hero-logo"
          />
        )}
        <div className="brand-header-meta">
          <div className="brand-header-title">
            <h1 className="brand-name">{brand.name}</h1>
            <div className="brand-badges">
              <span className={`tier-badge tier-${brand.tier}`}>
                T{brand.tier}
              </span>
              <span className={`rel-badge rel-${brand.relationship}`}>
                {brand.relationship}
              </span>
            </div>
          </div>
          {brand.profile?.tagline && (
            <p className="brand-tagline">{brand.profile.tagline}</p>
          )}
          {brand.profile && <ProfileMeta profile={brand.profile} />}
          <SocialLinks handles={brand.social_handles} />
        </div>
      </div>

      <div className="completeness-bar-wrap">
        <div className="completeness-bar">
          <div
            className="completeness-fill"
            style={{ width: `${brand.completeness.score}%` }}
          />
        </div>
        <span className="completeness-label">
          {brand.completeness.score}% complete
        </span>
      </div>

      <div className="profile-tabs">
        {(["feed", "content", "recon", "edit"] as const).map((tab) => (
          <button
            key={tab}
            type="button"
            className={`profile-tab-btn${activeTab === tab ? " is-active" : ""}`}
            onClick={() => setActiveTab(tab)}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </div>

      <div className="profile-tab-content panel">
        {activeTab === "feed" && <BrandSignalFeed slug={slug} />}
        {activeTab === "content" && <BrandContentTab slug={slug} />}
        {activeTab === "recon" && <BrandReconTab slug={slug} />}
        {activeTab === "edit" && <BrandEditTab slug={slug} brand={brand} />}
      </div>
    </div>
  );
}
