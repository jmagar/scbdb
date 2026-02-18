# MVP Scope

## Core Deliverables

1. **Rust CLI** — clap-based command-line tool as the primary interface
2. **Shopify product scraper** — Custom collector that pulls `products.json` from competitor Shopify storefronts, normalizes product data (name, THC/CBD mg, price, format, flavor, availability), and stores it locally
3. **Axum API server** — REST endpoints serving product and competitor data to the frontend
4. **React frontend** — React 19 + TypeScript SPA with shadcn/ui components and Tailwind CSS 4+ for browsing and comparing competitor products
