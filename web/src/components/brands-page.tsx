import type { BrandSummaryItem } from "../types/api";
import { useBrands } from "../hooks/use-dashboard-data";
import { ErrorState, LoadingState } from "./dashboard-utils";
import { BrandCreateDialog } from "./brand-create-dialog";

export function BrandsPage() {
  const { data: brands, isLoading, error } = useBrands();

  if (isLoading) return <LoadingState label="brands" />;
  if (error) return <ErrorState label="brands" />;

  return (
    <div className="brands-page">
      <div className="page-header">
        <h1 className="page-title">Brands</h1>
        <a href="#/" className="back-link">
          ← Dashboard
        </a>
      </div>
      <BrandCreateDialog
        onCreated={(slug) => {
          window.location.hash = `#/brands/${slug}`;
        }}
      />
      <div className="brand-grid">
        {brands?.map((brand) => (
          <BrandCard key={brand.slug} brand={brand} />
        ))}
      </div>
    </div>
  );
}

function BrandCard({ brand }: { brand: BrandSummaryItem }) {
  return (
    <a href={`#/brands/${brand.slug}`} className="brand-card">
      <div className="brand-card-body">
        <div className="brand-card-header">
          {brand.logo_url && (
            <img
              src={brand.logo_url}
              alt={brand.name}
              className="brand-logo-sm"
            />
          )}
          <h3 className="brand-name">{brand.name}</h3>
        </div>
        <div className="brand-badges">
          <span className={`tier-badge tier-${brand.tier}`}>T{brand.tier}</span>
          <span className={`rel-badge rel-${brand.relationship}`}>
            {brand.relationship}
          </span>
        </div>
        <div className="completeness-bar-wrap">
          <div className="completeness-bar">
            <div
              className="completeness-fill"
              style={{ width: `${brand.completeness_score}%` }}
            />
          </div>
          <span className="completeness-label">
            {brand.completeness_score}% complete
          </span>
        </div>
      </div>
    </a>
  );
}
