# MVP Index

## Document Metadata

- Version: 1.1
- Status: Active
- Last Updated (EST): 18:55:35 | 02/18/2026 EST

This file is the index for execution phases. Detailed implementation content lives in phase docs.

## Phase Documents

1. [mvp_phases/phase-1-foundation.md](mvp_phases/phase-1-foundation.md)
2. [mvp_phases/phase-2-collection-cli.md](mvp_phases/phase-2-collection-cli.md)
3. [mvp_phases/phase-3-regulatory-tracking.md](mvp_phases/phase-3-regulatory-tracking.md)
4. [mvp_phases/phase-4-sentiment-pipeline.md](mvp_phases/phase-4-sentiment-pipeline.md)
5. [mvp_phases/phase-5-api-dashboard.md](mvp_phases/phase-5-api-dashboard.md)

## Scope Guardrails

- MVP execution is CLI-first.
- Qdrant and TEI are integrated in Phase 4 for sentiment signal dedup and embedding storage.
- Spider remains a planned post-MVP capability.
- `config/brands.yaml` is the single brand registry for both portfolio and competitor brands.
