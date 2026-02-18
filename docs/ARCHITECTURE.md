# Architecture

## CLI

Rust binary built with **clap** for command parsing. Single entry point with subcommands for collection, reporting, and competitor management.

## Data Collection

### Shopify Scraper

Custom scraper that fetches `{domain}/products.json` from Shopify-powered competitor storefronts. Handles pagination, normalizes the Shopify product/variant schema into the internal product model, and persists structured data to the database.

This is the primary ingestion path — most tracked brands run Shopify storefronts (see `brands.yaml` for shop URLs).
