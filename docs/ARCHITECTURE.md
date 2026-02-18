# Architecture

## Backend (Rust)

### CLI

Rust binary built with **clap** for command parsing. Single entry point with subcommands for collection, reporting, legislative queries, and competitor management.

### API Server

**Axum**-based HTTP server exposing a REST API consumed by the frontend. Serves product data, competitor listings, legislative tracking, market sentiment, and scrape status. The CLI and API server share the same Rust crate (library code) — the binary can run in either CLI or server mode.

### Data Collection

#### Shopify Scraper

Custom scraper that fetches `{domain}/products.json` from Shopify-powered competitor storefronts. Handles pagination, normalizes the Shopify product/variant schema into the internal product model, and persists structured data to the database.

This is the primary ingestion path — most tracked brands run Shopify storefronts (see `brands.yaml` for shop URLs).

#### LegiScan Extraction

Integration with the LegiScan API for tracking cannabis-related legislation. Ingests bills, amendments, and vote records into the database.

### Market Sentiment

Pipeline for aggregating and scoring market sentiment signals alongside product and legislative data.

## Frontend (Vite + React + TypeScript)

**Vite**-powered **React 19** SPA styled with **Tailwind CSS 4+** and **shadcn/ui** components. Communicates with the Axum backend over REST. Provides dashboards for browsing competitor products, tracking legislation, comparing pricing, viewing market sentiment, and monitoring scrape runs.
