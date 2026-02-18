# THC-Intel: Competitive Analysis & Regulatory Tracker

## Product Requirements Document (PRD)

**Version:** 1.0
**Author:** Jacob — Reyes Beverage Group
**Date:** February 14, 2026
**Status:** Draft — Semi-official proof of concept

---

## 1. Executive Summary

THC-Intel is a CLI-first competitive intelligence and regulatory tracking platform built to support Reyes Beverage Group's growing THC/CBD beverage distribution portfolio. The tool automates data collection, structures competitive analysis, and generates actionable reports on competitor brands, market positioning, and the regulatory landscape at the federal and South Carolina state level.

Reyes currently distributes High Rise Beverage Co. THC seltzers out of South Carolina and is actively expanding its THC/CBD brand portfolio. The hemp-derived THC beverage market reached $1.1B in 2024 revenue with 25% projected YoY growth, but operates in a rapidly shifting regulatory environment — particularly in South Carolina, where 6+ competing bills are actively moving through the 126th General Assembly. This tool gives the team data infrastructure to make informed decisions about competitive positioning, market white space, and regulatory risk.

### 1.1 Project Context

| Attribute | Detail |
|---|---|
| **Organization** | Reyes Beverage Group |
| **Location** | South Carolina |
| **Anchor Brand** | High Rise Beverage Co. (SC-based, 3/5/10mg THC seltzers) |
| **Portfolio Status** | Growing — multiple THC/CBD brands in distribution |
| **Initiative Type** | Semi-official proof of concept |
| **Audience** | Jacob's team at Reyes |
| **Existing Data** | None — greenfield build, no seed data |
| **System Integration** | Standalone — no SipMarket or internal system dependencies |
| **Infrastructure** | Self-hosted on Jacob's homelab (Unraid, Docker, Tailscale) |
| **Existing Tooling** | Shopify `products.json` ingestion + Spider fallback + Qdrant/TEI pipeline |

---

## 2. Problem Statement

### 2.1 Market Context

The hemp-derived THC beverage category is experiencing explosive growth:

- **$1.1B in US sales in 2024** (Whitney Economics), up from $400K in 2020
- **500–750 active brands** competing nationally for shelf space; top 20 account for $377M
- **25% YoY growth projected** for 2025, with only ~11% of the estimated $15B TAM penetrated
- Distribution is rapidly shifting from dispensaries (<25% of sales) to liquor stores, grocery, convenience, and on-premise — directly into Reyes's existing retail account base
- The "female power shopper" (Whitney Economics) is the primary driver of traditional retail sales — consumers averse to dispensaries but open to purchasing THC in grocery and liquor stores

### 2.2 Competitive Pressure

High Rise Beverage Co. is competing for shelf space against well-funded, fast-growing national brands:

- **Cann** — $34.7M raised, 33 states, 200% YoY growth, Total Wine in 20 states
- **BRĒZ** — $28M revenue in 2024, projecting $50M in 2025, 2,000+ retail doors
- **Uncle Arnie's** — $7.5M Series A, 100% YoY growth, #1 cannabis beverage in IL dispensaries
- **Cycling Frog** — Multi-award winner, nationwide DTC + expanding retail, first stadium THC sales
- **Keef Brands** — $18.9M sales Jan–Aug 2024, 14 states, 70+ SKUs, dispensary + hemp dual play
- **Wynk** — 40 states DTC, 20 retail markets, zero-cal positioning, backed by major cannabis MSO

Most of these brands explicitly market themselves as alcohol alternatives and are targeting the same retail accounts Reyes services.

### 2.3 Regulatory Uncertainty

South Carolina has **zero current regulation** on hemp-derived THC products — no age restrictions, no licensing, no dosage caps. The 126th General Assembly (2025–2026) has 6+ active bills that range from comprehensive regulation to an outright ban:

| Bill | Approach | Key Provision |
|---|---|---|
| **H.3924** | Regulate (moderate) | 21+ age, licensing, testing, 1000ft school buffer. Passed House, in Senate. |
| **H.4004** | Regulate (beverage-specific) | Dedicated beverage licensing framework, SLED enforcement |
| **H.3935** | Regulate (comprehensive) | Three-tier licensing (producer/distributor/retailer), franchise protections |
| **H.4758** | **BAN** | Outright ban on all consumable hemp THC products, treats as contraband |
| **H.4759** | Regulate (restrictive) | 5mg/12oz cap, classifies THC beverage as "intoxicating alcoholic beverage" |
| **S.137 / H.3601** | Regulate (moderate) | Licensing, testing, $500 producer / $250 retailer fees |

Federally, the Farm Bill reauthorization and potential appropriations riders could ban hemp-derived THC products nationwide within ~1 year. SLED Chief Mark Keel has publicly advocated for a full ban in SC.

### 2.4 The Core Problem

Reyes's team currently has no systematic, automated way to:

1. **Monitor competitor activity** — New SKUs, pricing changes, flavor launches, retail expansion, and funding rounds happen weekly across 20+ relevant brands
2. **Benchmark High Rise** — No structured comparison of pricing, dosage tiers, flavor profiles, and distribution reach against the competitive set
3. **Identify market white space** — Gaps in dosage, flavor, format, or positioning that High Rise (and future portfolio brands) could exploit
4. **Track regulatory risk** — SC bill movement, federal activity, and enforcement actions that could reshape the market overnight
5. **Share intelligence** — No standardized reporting for the team; insights live in individual research and memory

---

## 3. Goals & Success Metrics

### 3.1 Primary Goals

| # | Goal | Description |
|---|---|---|
| G1 | **Competitive Positioning** | Maintain a structured, up-to-date view of competitor pricing, dosage tiers, flavors, retail presence, and brand positioning relative to High Rise and portfolio brands |
| G2 | **White Space Identification** | Surface gaps in the market (underserved dosage ranges, missing flavor profiles, untapped retail formats, geographic voids) that Reyes's portfolio brands could fill |
| G3 | **Shelf Space Tracking** | Monitor which competitor brands are gaining or losing distribution in Reyes's key markets |
| G4 | **Regulatory Risk Assessment** | Track SC state and federal legislative/regulatory activity and assess impact on High Rise and the broader THC distribution business |

### 3.2 Success Metrics

| Metric | Target | Measurement |
|---|---|---|
| Competitor coverage | Track Tier 1 brands (8) with full product catalog data within 30 days of MVP | Count of brands with structured product data in DB |
| Data freshness | Product and pricing data refreshed at least weekly | Avg age of most recent data point per competitor |
| Regulatory coverage | All active SC bills + key federal activity tracked with status updates within 48 hours of movement | Bill status accuracy vs. LegiScan / scstatehouse.gov |
| Report turnaround | Generate competitive comparison report in <60 seconds via CLI | CLI timing |
| Team adoption | At least 2 team members using reports within 60 days of MVP | Usage / feedback |

### 3.3 Non-Goals (v1)

- Interactive web dashboard (future phase)
- Real-time price monitoring or alerting
- Point-of-sale or scan data integration
- Multi-state regulatory tracking beyond SC + federal
- Automated brand recommendations or scoring
- Integration with SipMarket or other Reyes internal systems

---

## 4. User Personas

### 4.1 Primary: Jacob (Builder & Power User)

- **Role:** Software engineer at Reyes Beverage Group
- **Context:** Builds and operates the tool, conducts ad hoc analysis, generates reports for the team
- **Needs:** CLI interface, scriptable commands, raw data access, ability to extend with new collectors and extraction prompts
- **Interaction:** Daily — runs collection jobs, queries data, builds custom reports

### 4.2 Secondary: Reyes THC Distribution Team

- **Role:** Sales, strategy, and operations team members
- **Context:** Consumes reports and insights; may not interact with CLI directly
- **Needs:** Clear, formatted XLSX and markdown reports; competitive comparison matrices; regulatory status summaries
- **Interaction:** Weekly — receives scheduled reports, requests ad hoc analysis from Jacob

---

## 5. Solution Architecture

### 5.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     DATA COLLECTION                         │
├──────────────┬──────────────┬──────────────┬────────────────┤
│  Competitor  │  Sentiment   │  Financial   │  Regulatory    │
│  Products    │  & News      │  Activity    │  Tracker       │
│              │              │              │                │
│  Shopify     │  Spider +    │  Spider +    │  LegiScan API  │
│  products.   │  source RSS  │  source      │  + Spider      │
│  json        │  / news APIs │  scraping    │  (state agency │
│  + Spider    │              │              │  sites)        │
└──────┬───────┴──────┬───────┴──────┬───────┴───────┬────────┘
       │              │              │               │
       ▼              ▼              ▼               ▼
┌─────────────────────────────────────────────────────────────┐
│                     DATA STORAGE                            │
├─────────────────────────────┬───────────────────────────────┤
│  PostgreSQL                 │  Qdrant (direct ingest)       │
│  ─────────────────────      │  ─────────────────────        │
│  Structured data:           │  Unstructured / semantic:     │
│  • Product catalogs         │  • Full competitor page text  │
│  • Pricing history          │  • News articles & press      │
│  • Competitor profiles      │  • State law full text        │
│  • Bill tracker (status,    │  • SEC filings & reports      │
│    sponsors, votes)         │  • Investor presentations     │
│  • Collection job log       │                               │
└──────────────┬──────────────┴───────────────┬───────────────┘
               │                              │
               ▼                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     INTERFACE LAYER                          │
├─────────────────────────────┬───────────────────────────────┤
│  CLI (commander)            │  Qdrant ask/query             │
│  ─────────────────────      │  ─────────────────────        │
│  • thc-intel collect        │  • "What flavors does Cann    │
│  • thc-intel report         │     offer under $5?"          │
│  • thc-intel compare        │  • "What's SC's proposed      │
│  • thc-intel regs           │     THC limit per serving?"   │
│  • thc-intel schedule       │                               │
└──────────────┬──────────────┴───────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────────────────────┐
│                     REPORT OUTPUT                            │
├──────────────┬──────────────┬───────────────────────────────┤
│  XLSX        │  Markdown    │  Future: Email/Slack, Web UI  │
│  Reports     │  Summaries   │                               │
└──────────────┴──────────────┴───────────────────────────────┘
```

### 5.2 Tech Stack

| Component | Technology | Rationale |
|---|---|---|
| **Language** | TypeScript (CLI, orchestration, collectors) | Monorepo-friendly; strong ecosystem for Shopify + scraping workflows |
| **Language** | Python (NLP, regulatory text classification) | Better ML/NLP ecosystem; local LLM integration for bill relevance scoring |
| **Database** | PostgreSQL | Already running in homelab; rich querying for structured comparisons; JSONB for flexible product schemas |
| **Vector DB** | Qdrant (existing) | Already deployed in homelab; used for semantic retrieval over crawled/docs content |
| **Embeddings** | HuggingFace TEI (existing) | Already running on remote GPU server for local embedding generation |
| **Web/Data Collection** | Shopify `products.json` + Spider | Shopify storefront JSON for primary product ingestion; Spider for non-Shopify fallback pages |
| **CLI Framework** | commander | Lightweight, proven; good fit for scriptable internal tooling |
| **Reports** | ExcelJS (XLSX), markdown templates | Team-friendly output formats |
| **Scheduling** | node-cron or systemd timers | Homelab-native; no external scheduler dependency |
| **Regulatory API** | LegiScan API | Tracks bills across all 50 states; keyword alerts; free tier available |
| **Monorepo** | Turborepo | Consistent with Jacob's monorepo-first preference |

### 5.3 Monorepo Structure

```
thc-intel/
├── packages/
│   ├── core/                     # Shared types, DB client, ingestion adapters
│   │   ├── src/
│   │   │   ├── db/               # Drizzle ORM schema & queries
│   │   │   │   ├── schema.ts     # PostgreSQL table definitions
│   │   │   │   ├── migrate.ts    # Migration runner
│   │   │   │   └── queries/      # Query modules by domain
│   │   │   ├── ingestion/        # Shopify + Spider orchestration
│   │   │   │   ├── shopify.ts    # products.json fetch + normalization
│   │   │   │   ├── spider.ts     # Spider crawl/scrape fallback wrapper
│   │   │   │   └── extract.ts    # Prompt-schema-based extraction utilities
│   │   │   ├── types/            # Competitor, Product, Regulation, Bill types
│   │   │   └── config/           # YAML config loader
│   │   └── package.json
│   ├── collectors/               # Data ingestion modules
│   │   ├── src/
│   │   │   ├── products.ts       # Product catalog extraction
│   │   │   ├── pricing.ts        # Price tracking over time
│   │   │   ├── regulatory.ts     # LegiScan + state site crawling
│   │   │   ├── financial.ts      # Funding, M&A, press releases
│   │   │   └── sentiment.ts      # Brand mentions, news, Reddit
│   │   └── package.json
│   ├── cli/                      # CLI interface
│   │   ├── src/
│   │   │   ├── index.ts          # Entry point
│   │   │   └── commands/         # Command modules
│   │   │       ├── collect.ts
│   │   │       ├── report.ts
│   │   │       ├── compare.ts
│   │   │       ├── regs.ts
│   │   │       ├── competitors.ts
│   │   │       └── schedule.ts
│   │   └── package.json
│   └── reports/                  # Report generation
│       ├── src/
│       │   ├── xlsx/             # ExcelJS report builders
│       │   │   ├── competitive.ts
│       │   │   ├── pricing.ts
│       │   │   ├── regulatory.ts
│       │   │   └── whitespace.ts
│       │   └── markdown/         # Markdown report templates
│       └── package.json
├── config/
│   ├── brands.yaml               # Brand registry
│   ├── portfolio.yaml            # Reyes THC brand portfolio
│   ├── regulatory.yaml           # Tracked bills, agencies, feeds
│   └── extraction_prompt_schema.md # Extraction prompt schema definition
├── docker-compose.yml            # PostgreSQL + local supporting services
├── turbo.json
├── package.json
└── CLAUDE.md                     # Project context for Claude Code
```

### 5.4 Qdrant Collection Strategy

| Collection | Content | Primary Use |
|---|---|---|
| `thc-products` | Competitor product pages, catalogs, ingredient lists | "What 5mg seltzers exist under $6/can?" |
| `thc-regulatory` | State bill text, agency guidance, FDA updates | "What's SC's proposed dosage cap?" |
| `thc-news` | News articles, press releases, Reddit, social | "What's the latest on BRĒZ expansion?" |
| `thc-financial` | Funding announcements, SEC filings, investor decks | "Who raised a Series A this quarter?" |

---

## 6. Data Model

### 6.1 Core Entities (PostgreSQL)

**competitors**

| Column | Type | Description |
|---|---|---|
| id | uuid | Primary key |
| name | varchar | Brand name (e.g., "Cann") |
| slug | varchar | URL-safe identifier |
| website | varchar | Primary website URL |
| hq_location | varchar | Headquarters city/state |
| founded_year | integer | Year founded |
| tier | enum | tier_1, tier_2, tier_3, tier_4 |
| category | enum | hemp_derived, dispensary, dual |
| total_funding | decimal | Total known funding raised |
| employee_count | integer | Approximate headcount |
| distribution_states | integer | Number of states with retail presence |
| notes | text | Freeform notes |
| metadata | jsonb | Flexible additional data |
| created_at | timestamptz | Record creation |
| updated_at | timestamptz | Last update |

**products**

| Column | Type | Description |
|---|---|---|
| id | uuid | Primary key |
| competitor_id | uuid | FK → competitors |
| name | varchar | Product name |
| product_type | enum | seltzer, soda, tonic, tea, lemonade, shot, mixer, other |
| thc_mg | decimal | THC per serving (mg) |
| cbd_mg | decimal | CBD per serving (mg) |
| other_cannabinoids | jsonb | CBG, CBN, etc. |
| volume_oz | decimal | Container size (oz) |
| format | enum | can, bottle, pouch, shot |
| flavor | varchar | Flavor name |
| calories | integer | Calories per serving |
| sugar_g | decimal | Sugar per serving (g) |
| price_usd | decimal | Current retail price |
| pack_size | integer | Units per pack |
| price_per_unit | decimal | Computed: price / pack_size |
| price_per_mg_thc | decimal | Computed: price_per_unit / thc_mg |
| available | boolean | Currently in production |
| ingredients | text[] | Ingredient list |
| functional_adds | text[] | Adaptogens, mushrooms, etc. |
| source_url | varchar | Where data was scraped from |
| metadata | jsonb | Flexible additional data |
| collected_at | timestamptz | When this data was collected |
| created_at | timestamptz | Record creation |

**price_history**

| Column | Type | Description |
|---|---|---|
| id | uuid | Primary key |
| product_id | uuid | FK → products |
| price_usd | decimal | Price at this point in time |
| source | varchar | Where price was observed |
| observed_at | timestamptz | When price was recorded |

**bills**

| Column | Type | Description |
|---|---|---|
| id | uuid | Primary key |
| jurisdiction | enum | federal, south_carolina |
| bill_number | varchar | e.g., "H.4759" |
| legiscan_bill_id | integer | LegiScan API ID |
| title | varchar | Short title |
| summary | text | Bill summary |
| approach | enum | regulate_moderate, regulate_restrictive, regulate_comprehensive, ban, legalize |
| status | varchar | Current status (introduced, in_committee, passed_house, etc.) |
| last_action | text | Most recent legislative action |
| last_action_date | date | Date of most recent action |
| sponsors | jsonb | Array of sponsor names/parties |
| key_provisions | jsonb | Structured provisions (dosage caps, licensing, age reqs, etc.) |
| impact_assessment | text | Freeform analysis of impact on portfolio |
| source_url | varchar | Link to full bill text |
| metadata | jsonb | Flexible additional data |
| created_at | timestamptz | Record creation |
| updated_at | timestamptz | Last update |

**bill_events**

| Column | Type | Description |
|---|---|---|
| id | uuid | Primary key |
| bill_id | uuid | FK → bills |
| event_type | varchar | committee_hearing, vote, amendment, etc. |
| description | text | Event description |
| event_date | date | When event occurred |
| source_url | varchar | Source link |
| created_at | timestamptz | Record creation |

**collection_log**

| Column | Type | Description |
|---|---|---|
| id | uuid | Primary key |
| collector | varchar | products, pricing, regulatory, financial, sentiment |
| target | varchar | Competitor slug or bill number |
| status | enum | running, success, partial, failed |
| records_collected | integer | Count of records ingested |
| error_message | text | Error details if failed |
| duration_ms | integer | Execution time |
| started_at | timestamptz | Job start |
| completed_at | timestamptz | Job end |

---

## 7. Extraction Prompt Schema

Primary extraction is driven by Shopify storefront `products.json` payloads. For non-Shopify pages, Spider uses `extraction_prompt_schema.md` to normalize extracted fields consistently.

### 7.1 Product Catalog Mapping (Shopify `products.json` -> Internal Model)

```json
{
  "products": [
    {
      "name": "string — full product name including variant",
      "product_type": "string — one of: seltzer, soda, tonic, tea, lemonade, shot, mixer, other",
      "thc_mg": "number — milligrams of THC per serving",
      "cbd_mg": "number — milligrams of CBD per serving, 0 if none",
      "other_cannabinoids": "object — e.g. { 'CBG': 10, 'CBN': 5 }, empty object if none",
      "volume_oz": "number — container size in fluid ounces",
      "format": "string — one of: can, bottle, pouch, shot",
      "flavor": "string — flavor name",
      "calories": "number — calories per serving",
      "sugar_g": "number — grams of sugar per serving",
      "price_usd": "number — retail price in USD",
      "pack_size": "number — number of units in this pack/listing",
      "available": "boolean — true if currently purchasable",
      "ingredients": "array of strings — ingredient list",
      "functional_additions": "array of strings — adaptogens, mushrooms, vitamins, etc."
    }
  ]
}
```

### 7.2 Company Info / Brand Metadata Schema

```json
{
  "company_name": "string",
  "tagline": "string — brand tagline or positioning statement",
  "founded_year": "number",
  "hq_city": "string",
  "hq_state": "string",
  "key_retail_partners": "array of strings — named retail chains",
  "states_available": "array of strings — state abbreviations where products ship/sell",
  "certifications": "array of strings — organic, non-GMO, vegan, gluten-free, etc.",
  "social_links": "object — { instagram, twitter, tiktok, linkedin }",
  "distribution_model": "string — DTC, retail, both"
}
```

---

## 8. CLI Interface

### 8.1 Command Reference

**Competitor Management**

```bash
thc-intel competitors list                          # List all tracked competitors
thc-intel competitors add --name "Cann" --url "https://drinkcann.com" --tier 1
thc-intel competitors show cann                     # Show competitor detail + products
thc-intel competitors remove cann
```

**Data Collection**

```bash
thc-intel collect products --competitor cann         # Extract product catalog for one brand
thc-intel collect products --all                     # All tracked competitors
thc-intel collect products --tier 1                  # All Tier 1 competitors
thc-intel collect pricing --competitor cann           # Record current pricing snapshot
thc-intel collect regs                               # Pull latest SC + federal bill status
thc-intel collect news --brands "cann,brez,wynk"     # Search + scrape recent news/press
thc-intel collect financial --competitor cann         # Funding, M&A, press releases
```

**Analysis & Comparison**

```bash
thc-intel compare pricing --competitors "cann,brez,cycling-frog" --vs high-rise
thc-intel compare products --dosage 5 --format seltzer    # All 5mg seltzers across brands
thc-intel whitespace --dimension flavor                    # Underserved flavor profiles
thc-intel whitespace --dimension dosage                    # Gaps in dosage tiers
thc-intel whitespace --dimension format                    # Underserved product formats
```

**Regulatory**

```bash
thc-intel regs status                               # Current status of all tracked bills
thc-intel regs show H.4759                          # Detail on specific bill
thc-intel regs impact                               # Impact assessment summary
thc-intel regs timeline                             # Chronological bill activity
```

**Reports**

```bash
thc-intel report competitive --format xlsx           # Full competitive landscape report
thc-intel report pricing --format xlsx               # Pricing comparison matrix
thc-intel report regulatory --format markdown         # Regulatory status summary
thc-intel report whitespace --format xlsx             # Market gap analysis
thc-intel report executive --format xlsx              # Combined executive summary
```

**Semantic Search (Qdrant query wrappers)**

```bash
thc-intel ask "What THC beverages does Cann sell under $5?"
thc-intel ask "What's SC's proposed dosage cap for THC beverages?"
thc-intel ask "Has BRĒZ announced any new flavors recently?"
thc-intel query "mushroom adaptogen THC seltzer"     # Raw semantic search
```

**Scheduling**

```bash
thc-intel schedule list                              # Show active schedules
thc-intel schedule set --daily regs                  # Daily regulatory checks
thc-intel schedule set --weekly products,pricing      # Weekly product + price refresh
thc-intel schedule set --monthly financial,news       # Monthly funding + news sweep
```

### 8.2 Report Specifications

**Competitive Landscape Report (XLSX)**

Sheets:

1. **Dashboard** — Portfolio brands vs. top competitors at a glance (brand, tier, funding, states, SKU count, avg price/unit)
2. **Product Matrix** — Every tracked product: brand, name, type, THC mg, CBD mg, volume, flavor, price, price/unit, price/mg
3. **Pricing Comparison** — Side-by-side pricing by dosage tier (2.5mg, 5mg, 10mg) and format
4. **Flavor Map** — Matrix of brands × flavors showing coverage and gaps
5. **Distribution Footprint** — States and key retail partners per brand
6. **Funding & Growth** — Total raised, latest round, revenue estimates, growth signals

**Regulatory Status Report (Markdown)**

Sections:

1. Executive summary — one-paragraph risk assessment
2. SC bill tracker table — bill number, approach, status, last action, key provisions
3. Federal activity — Farm Bill, appropriations, FDA guidance
4. Impact analysis — what each scenario (regulate vs. ban) means for the portfolio
5. Timeline — chronological list of recent and upcoming events

**White Space Report (XLSX)**

Sheets:

1. **Dosage Gaps** — Heatmap of brands × dosage tiers showing market density and voids
2. **Flavor Gaps** — Brands × flavor categories with coverage indicators
3. **Format Gaps** — Product format availability across competitors
4. **Price Tier Gaps** — Under $4, $4–6, $6–8, $8+ per unit density
5. **Functional Ingredient Map** — Who's using adaptogens, mushrooms, CBD pairings, etc.

---

## 9. Competitor Registry (Initial Seed)

### 9.1 Tier 1 — Category Leaders

Track with full product catalog ingestion (`products.json` first), pricing history, and financial monitoring.

| Brand | Slug | Website | HQ | Key Differentiator |
|---|---|---|---|---|
| Cann | cann | drinkcann.com | Oakland, CA | Category pioneer, microdose social tonics, $34.7M raised, Total Wine in 20 states |
| BRĒZ | brez | drinkbrez.com | — | THC + adaptogens + mushrooms, $28M rev 2024, fastest revenue growth in category |
| Cycling Frog | cycling-frog | cyclingfrog.com | Seattle, WA | Single-source farm, 2:1 CBD:THC ratio, award-winning, Portland Pickles partnership |
| Wynk | wynk | drinkwynk.com | — | Zero-cal/zero-sugar, backed by cannabis MSO, 40 states DTC |
| Uncle Arnie's | uncle-arnies | drinkunclearnies.com | Covina, CA | High-dose leader, #1 in IL, $7.5M Series A, Boston Beer Co. founder on board |
| Keef Brands | keef | keefbrands.com | Denver, CO | Dispensary + hemp dual play, 70+ SKUs, 14 states, deploys branded retail fridges |
| Cantrip | cantrip | drinkcantrip.com | — | Top-20 brand per Whitney, seltzer focus |
| Wana | wana | wanabrands.com | Boulder, CO | Established edibles brand expanding into beverages, multi-state |

### 9.2 Tier 2 — Fast-Growing / Regional

Track with product catalog ingestion; financial monitoring on quarterly basis.

| Brand | Slug | Website | Notable |
|---|---|---|---|
| Cheech & Chong's Rebel Rabbit | rebel-rabbit | — | Celebrity brand, Whitney top-20 |
| Mary Jones | mary-jones | drinkmaryjones.com | Retro soda branding, conventional retail push |
| Dad Grass | dad-grass | dadgrass.com | Low-dose lifestyle brand (3mg THC + CBD + Lion's Mane) |
| Find Wunder | find-wunder | findwunder.com | VC-backed, clean sparkling water positioning |
| Calexo | calexo | drinkcalexo.com | Upscale/premium seltzer |
| Gigli | gigli | — | Top seller in Chicago-area retail ($17–18/4pk) |
| Delta Crush | delta-crush | — | Strong in Buffalo/NY retail |
| Trail Magic | trail-magic | — | Strong in MN market |
| High Rise Beverage Co. | high-rise | — | **Portfolio brand** — SC-based, 3/5/10mg seltzers |

### 9.3 Tier 3 — Niche / Watch List

Track with periodic news monitoring only.

| Brand | Slug | Notable |
|---|---|---|
| Adaptaphoria | adaptaphoria | Functional wellness + THC |
| Herbal Oasis | herbal-oasis | THC + CBG + 2,500mg mushroom blend |
| Señorita Drinks | senorita | THC margaritas, agave-based cocktail format |
| Flyers Cocktail Co. | flyers | CBD-only cocktail replacements |
| Recess | recess | Mainstream CBD sparkling water, major retail |
| Cornbread Hemp | cornbread-hemp | Kentucky heritage, seltzers + gummies |
| Buzzn | buzzn | Minimalist seltzer, clean microdosing |

### 9.4 Tier 4 — Dispensary-First (Context Only)

No active collection; reference data from BDSA/Headset reports.

| Brand | Notable |
|---|---|
| St. Ides | #1 CA dispensary beverage (Headset) |
| Ayrloom | Top-5 dispensary beverage (BDSA) |
| Sip Elixirs | Top-5 dispensary beverage (BDSA) |
| Levia | Strong in MA/Northeast |
| Not Your Father's Root Beer (cannabis) | Established brand crossover |

---

## 10. Regulatory Tracker Scope

### 10.1 South Carolina — Active Bills

| Bill | LegiScan ID | Track | Priority |
|---|---|---|---|
| H.3924 | TBD | Full: status, votes, amendments, text changes | **Critical** — furthest along, in Senate |
| H.4004 | TBD | Full | High — beverage-specific framework |
| H.3935 | TBD | Full | High — three-tier model affects distribution |
| H.4758 | TBD | Full | **Critical** — outright ban |
| H.4759 | TBD | Full | **Critical** — 5mg cap + alcohol-style regulation |
| S.137 | TBD | Full | Medium |
| H.3601 | TBD | Status only | Medium |
| H.3804 | TBD | Status only | Low — marijuana decrim, indirect relevance |

### 10.2 South Carolina — Agencies & Sources

| Source | URL / Feed | Data Type |
|---|---|---|
| SC Legislature | scstatehouse.gov | Bill text, status, votes, committee schedules |
| SC Dept. of Agriculture | scda.sc.gov | Rulemaking, licensing (if bills pass) |
| SLED | sled.sc.gov | Enforcement guidance, public statements |
| Senate Agriculture Committee | scstatehouse.gov | Hearing schedules, committee votes |

### 10.3 Federal — Tracked Activity

| Source | What to Track |
|---|---|
| 2018 Farm Bill / Reauthorization | Any amendments to hemp-derived THC legality |
| Appropriations riders | Provisions banning hemp THC products |
| FDA | Guidance on cannabinoids in food/beverages, NDI notifications |
| DEA | Scheduling changes affecting hemp-derived cannabinoids |
| LegiScan federal bills | Keyword alerts: "hemp beverage", "THC beverage", "cannabinoid food" |

---

## 11. Phasing & Roadmap

### Phase 0 — Foundation (Week 1–2)

**Goal:** Scaffolded monorepo with working database and brand config.

- [ ] Initialize Turborepo monorepo with packages: core, collectors, cli, reports
- [ ] Set up PostgreSQL schema with Drizzle ORM migrations
- [ ] Create `brands.yaml` with Tier 1 + Tier 2 registry
- [ ] Create `portfolio.yaml` with High Rise + other Reyes THC brands
- [ ] Create `regulatory.yaml` with SC bill tracker config
- [ ] Finalize `extraction_prompt_schema.md` (product + company extraction schema)
- [ ] CLI skeleton with `competitors list/add/show` commands
- [ ] Docker Compose for PostgreSQL

**Deliverable:** `thc-intel competitors list` returns all registered brands from `brands.yaml`.

### Phase 1 — Product Intelligence (Week 3–4)

**Goal:** Automated product catalog extraction and first competitive report.

- [ ] Product collector: pulls Shopify `products.json` and normalizes to product schema
- [ ] Pricing collector: snapshot current prices into price_history
- [ ] `thc-intel collect products --competitor cann` working end-to-end
- [ ] `thc-intel collect products --tier 1` batch collection
- [ ] `thc-intel compare products` — cross-brand product matrix
- [ ] Competitive landscape XLSX report (basic version)
- [ ] Spider fallback collector for non-Shopify brands
- [ ] Seed Qdrant collections: `thc-products` from normalized product + crawl content

**Deliverable:** XLSX report comparing High Rise vs. Tier 1 competitors on pricing, dosage, and flavor.

### Phase 2 — Regulatory Tracker (Week 5–6)

**Goal:** Automated SC + federal bill tracking with impact reporting.

- [ ] LegiScan API client (bill status, history, text, sponsors)
- [ ] Regulatory collector: poll LegiScan + crawl scstatehouse.gov
- [ ] `thc-intel regs status` — table of all tracked bills
- [ ] `thc-intel regs show H.4759` — detail view with provisions
- [ ] `thc-intel regs impact` — impact assessment summary
- [ ] Regulatory status markdown report
- [ ] Seed Qdrant `thc-regulatory` collection with bill full text
- [ ] Daily scheduled regulatory checks via cron

**Deliverable:** `thc-intel regs status` returns live status of all SC bills, `thc-intel ask` can answer questions about bill text.

### Phase 3 — Market Intelligence (Week 7–9)

**Goal:** Sentiment, financial, white space, and scheduling.

- [ ] News/sentiment collector: Spider + source APIs for brand mentions
- [ ] Financial collector: funding rounds, press releases, M&A
- [ ] White space analysis commands (dosage, flavor, format gaps)
- [ ] White space XLSX report
- [ ] Executive summary report (combined competitive + regulatory)
- [ ] Full scheduling system: daily regs, weekly products, monthly financial
- [ ] `thc-intel ask` wrapper routing to correct Qdrant collection

**Deliverable:** Complete automated intelligence pipeline with scheduled collection and executive reporting.

### Phase 4 — Polish & Expand (Week 10+)

**Goal:** Report delivery automation, team adoption, and feedback loop.

- [ ] Automated report delivery (email or Slack webhook)
- [ ] Report diffing: highlight changes since last report
- [ ] Competitor alert system: new products, price changes, funding rounds
- [ ] Regulatory alert: bill movement notifications
- [ ] Team feedback collection and iteration
- [ ] Documentation and onboarding guide for team members

### Future — Dashboard (Deferred)

- React web UI for interactive exploration
- Real-time regulatory status board
- Interactive pricing and white space visualizations
- Integration with Reyes systems (SipMarket, sales data) if approved

---

## 12. Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| **SC bans THC beverages outright** (H.4758) | Medium | Critical | Tool pivots to tracking ban enforcement + monitoring for regulatory reopening; competitive intel still valuable for strategic planning |
| **Competitor websites change structure** | High | Medium | Design schemas to be resilient; log extraction failures; manual review cadence |
| **LegiScan free tier rate limits** | Medium | Low | Cache aggressively; fallback to scstatehouse.gov scraping; upgrade to paid tier if needed |
| **Spider fallback extraction quality varies** | Medium | Medium | Prioritize Shopify `products.json`; validate fallback extraction against prompt schema; flag low-confidence records for manual review |
| **Team doesn't adopt reports** | Medium | Medium | Start with one high-value report (pricing comparison); iterate on format based on feedback |
| **Federal Farm Bill changes kill hemp THC** | Low–Medium | Critical | Regulatory tracker provides early warning; tool scope can expand to adjacent categories |
| **Data freshness degrades** | Medium | Medium | Collection log monitoring; automated freshness alerts; clear "as of" dates on all reports |

---

## 13. Open Questions

| # | Question | Impact | Status |
|---|---|---|---|
| 1 | What other THC/CBD brands are in Reyes's distribution portfolio beyond High Rise? | Shapes portfolio.yaml and comparison scope | **Open** |
| 2 | Which specific retail accounts / markets is Reyes distributing High Rise into? | Would enable geo-targeted competitive tracking | **Open** |
| 3 | Does Reyes have access to scan data (Nielsen/IRI/Circana) for the THC category? | Could provide ground-truth sales data to validate competitive analysis | **Open** |
| 4 | Is there appetite for a paid LegiScan subscription ($50/mo) for higher API limits? | Affects regulatory tracker update frequency | **Open** |
| 5 | What's High Rise's current product lineup (all SKUs, pricing, flavors)? | Needed to seed the "home team" data accurately | **Open** |
| 6 | Are there specific competitor brands the team is most concerned about? | Would help prioritize Tier 1 / collection frequency | **Open** |
| 7 | Does the team have preferred report formats or existing templates? | Shapes report design | **Open** |

---

## 14. Glossary

| Term | Definition |
|---|---|
| **Hemp-derived THC** | Delta-9 THC extracted from hemp plants containing <0.3% THC by dry weight, legal under the 2018 Farm Bill |
| **Cannabis-derived THC** | THC from marijuana plants (>0.3% THC), legal only in states with adult-use or medical programs |
| **MSO** | Multi-State Operator — large cannabis companies operating across multiple state markets |
| **RTD** | Ready-To-Drink — pre-mixed beverages (cocktails, seltzers, etc.) |
| **SKU** | Stock Keeping Unit — a distinct product variant |
| **DTC** | Direct-To-Consumer — online sales shipped to the customer |
| **Three-tier system** | Alcohol distribution model: producer → distributor → retailer |
| **Farm Bill** | Federal legislation governing agriculture; the 2018 version legalized hemp |
| **LegiScan** | Third-party API that tracks state and federal legislation |
| **Spider** | Web crawling/scraping fallback used when Shopify `products.json` is unavailable |
| **Shopify products.json** | Storefront product endpoint used as primary structured product source |
| **Qdrant** | Vector database used for semantic search over scraped content |
| **TEI** | Text Embeddings Inference — HuggingFace model server for generating vector embeddings |

---

*This document is a living spec. It will be updated as the project progresses through proof-of-concept validation and team feedback.*
