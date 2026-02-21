import { type ProductItem } from "../types/api";
import {
  ErrorState,
  LoadingState,
  formatDate,
  formatMoney,
} from "./dashboard-utils";

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
      {!isLoading && !isError && (
        <div className="card-stack">
          {data?.map((item) => (
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
                <span>{item.brand_name}</span>
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
                  <dd>{item.tier}</dd>
                </div>
              </dl>
            </article>
          ))}
        </div>
      )}
    </>
  );
}
