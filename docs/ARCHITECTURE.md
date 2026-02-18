# Architecture

## Backend (Rust)

### CLI

Rust binary built with **clap** for command parsing. Single entry point with subcommands for collection, reporting, and competitor management.

### API Server

**Axum**-based HTTP server exposing a REST API consumed by the frontend. Serves product data, competitor listings, and scrape status. The CLI and API server share the same Rust crate (library code) — the binary can run in either CLI or server mode.

### Data Collection

#### Shopify Scraper

Custom scraper that fetches `{domain}/products.json` from Shopify-powered competitor storefronts. Handles pagination, normalizes the Shopify product/variant schema into the internal product model, and persists structured data to the database.

This is the primary ingestion path — most tracked brands run Shopify storefronts (see `brands.yaml` for shop URLs).

## Frontend (React + TypeScript)

**React 19** SPA styled with **Tailwind CSS 4+** and **shadcn/ui** components. Communicates with the Axum backend over REST. Provides dashboards for browsing competitor products, comparing pricing, and monitoring scrape runs.
