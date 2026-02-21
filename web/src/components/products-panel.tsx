import { type ProductItem } from "../types/api";
import { ErrorState, LoadingState, formatMoney } from "./dashboard-utils";

type Props = {
  isLoading: boolean;
  isError: boolean;
  data: ProductItem[] | undefined;
};

export function ProductsPanel({ isLoading, isError, data }: Props) {
  return (
    <>
      <h2>Product Catalog</h2>
      {isLoading && <LoadingState label="products" />}
      {isError && <ErrorState label="products" />}
      {!isLoading && !isError && data?.length === 0 && (
        <p className="panel-status">
          No products collected yet. Run <code>collect products</code> to
          populate.
        </p>
      )}
      {!isLoading && !isError && data && data.length > 0 && (
        <div className="card-stack">
          {data.map((item) => (
            <article className="data-card" key={item.product_id}>
              {item.primary_image_url || item.brand_logo_url ? (
                <img
                  className="product-image"
                  src={
                    item.primary_image_url ?? item.brand_logo_url ?? undefined
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
                <span className={`rel-badge rel-badge--${item.relationship}`}>
                  {item.brand_name}
                </span>
              </header>
              <dl>
                <div>
                  <dt>Variants</dt>
                  <dd>{item.variant_count}</dd>
                </div>
                <div>
                  <dt>Latest Price</dt>
                  <dd>
                    {item.latest_price ? formatMoney(item.latest_price) : "-"}
                  </dd>
                </div>
                <div>
                  <dt>Tier</dt>
                  <dd>
                    <span className={`tier-badge tier-badge--${item.tier}`}>
                      T{item.tier}
                    </span>
                  </dd>
                </div>
              </dl>
            </article>
          ))}
        </div>
      )}
    </>
  );
}
