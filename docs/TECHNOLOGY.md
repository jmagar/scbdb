# Technology Stack

## Backend

- **Rust** — Primary language for backend, CLI, collectors, and data pipeline
- **Axum** — HTTP framework for the REST/API layer
- **clap** — Argument parsing and command routing for the CLI

## Frontend

- **Vite** — Dev server and build tooling
- **React 19** — UI framework
- **TypeScript** — Frontend language
- **Tailwind CSS 4+** — Utility-first styling
- **shadcn/ui** — Component library (built on Radix UI primitives)

## Data Collection

- **Custom Shopify scraper** — Purpose-built HTTP client that pulls `products.json` from Shopify storefronts and normalizes the response into the internal product schema
- **LegiScan API** — Legislative data extraction for cannabis-related bills and votes

## Analysis

- **Market sentiment pipeline** — Aggregation and scoring of market signals
