# Technology Stack

## Language

- **Rust** — Primary language for all components (CLI, collectors, data pipeline)

## CLI Framework

- **clap** — Argument parsing and command routing for the CLI interface

## Data Collection

- **Custom Shopify scraper** — Purpose-built HTTP client that pulls `products.json` from Shopify storefronts and normalizes the response into the internal product schema
