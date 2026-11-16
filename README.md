<div align="center">

  <img src="assets/logo.png" alt="Cielo Mio Logo" width="140" />

# Cielo Mio — SEBI BRSR Extractor & ESG Analytics Suite

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](<https://img.shields.io/badge/Rust-1.88%2B%20(Edition%202024)-orange.svg>)](https://www.rust-lang.org/)
[![PostgreSQL](<https://img.shields.io/badge/PostgreSQL-18%2B%20(pgvector)-blue.svg>)](https://www.postgresql.org/)
[![SEBI Compliance](https://img.shields.io/badge/SEBI-BRSR%20Core%20Aligned-green.svg)](https://www.sebi.gov.in/)

</div>

---

**Cielo Mio** (`v0.5.1`) is a native desktop extraction, normalization, and ESG analytics suite for the Securities and Exchange Board of India's (SEBI) **Business Responsibility and Sustainability Reporting (BRSR)** filings.

The application concurrently crawls, downloads, parses, and normalizes XBRL/XML and PDF corporate disclosures submitted by listed entities to the **National Stock Exchange (NSE)** and **Bombay Stock Exchange (BSE)**. It handles context priority resolution, exact base-10 unit conversions (`rust_decimal`), dynamic dimensional axis expansion, multi-tier industry classification, offline PIN-code centroid geocoding, and grounded AI Q&A.

---

## Architecture Overview

```text
[BSE / NSE Exchanges]
                                            │
                         (Simultaneous Dual-Market Sync Engine)
                                            ▼
                                [Raw XML/PDF Repository]
                                            │
┌───────────────────────────────────────────┴────────────────────────────────────────────┐
│                                 RUST EXTRACTION PIPELINE                               │
│                                                                                        │
│ [FormDictionary / xbrl_tag_rules]                 [Streaming quick-xml Parser]         │
│ • O(1) Tag-Indexed Axis Expansion                 • UTF-8 / UTF-16 BOM Auto-Decoding   │
│ • Suffix / Increment Cloning (`Axis1..N`)         • XML Control & Entity Sanitization  │
│ • Section / Principle / Grouping Routing          • Nested `<divide>` Unit Resolution  │
│                                                                                        │
│                                           │                                            │
│                                           ▼                                            │
│                               [Enrichment & Normalization]                             │
│ • End-to-End Exact `Decimal` Unit Conversions (GJ, t, KL, tNOxe)                       │
│ • Context Priority Layer (`DCYMain`/`ICYMain` > Axis > `DPYMain`/`PPY`)                │
│ • `ReportingBoundary` Normalization (`Standalone` / `Consolidated`)                    │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
                   [PostgreSQL 18+ / pgvector Storage Layer (Podman or Local)]
                   • `file_list` (Idempotent Sync & Retry State Machine)
                   • `entity_list` (`btree_gist` temporal FY bounds + auto-geo/industry triggers)
                   • `brsr_facts` (Normalized EAV Repository)
                   • `form_layout` (UI Hierarchy, Mojibake Cleaning & Visibility Rules)
                   • `view_company_esg_kpis` & `view_sector_benchmarks`
                                            │
                                            ▼
                        [Native Desktop Suite (egui 0.37 / eframe)]
                        • Download & Sync + Background XBRL Ingestion
                        • Table-Driven BRSR Form Explorer (`egui_extras::TableBuilder`)
                        • 19 Standardized ESG KPIs & Sector Median Benchmarking
                        • Interactive Geospatial Map (`walkers` / OpenStreetMap)
                        • Streaming CSV Export & `pg_dump` Backups
                        • Grounded ESG Copilot (Local Ollama / Google Gemini)
```

---

## Key Capabilities

### 1. Concurrent Exchange Ingestion & State Machine

- **Simultaneous Dual Sync:** Runs BSE (`GetCorXbrlDetails_ng/w` JSON + `CorpXbrlGen.aspx` fallback) and NSE (`corporate-bussiness-sustainabilitiy` API with cookie session warmup and $\le 365$-day sliding windows) crawlers concurrently via `tokio::join!`.
- **Atomic Streaming & Guardrails:** Streams filings to `.part` buffers before atomic rename, strips UTF-8 BOMs, rejects HTML error/WAF responses and 0-byte payloads, preserves unique filing suffixes when bounding filename lengths, and tracks state in `file_list` with one-click failed-filing reset.

### 2. Exact Decimal Parsing & Multi-Tier Enrichment

- **XML Sanitization & Namespace Isolation:** Auto-decodes UTF-8 and UTF-16 LE/BE BOMs, escapes bare ampersands and invalid control bytes, and isolates SEBI's `in-capmkt` taxonomy facts.
- **Exact Base-10 Unit Conversions (`rust_decimal`):** Parses and converts values directly in `Decimal` (without binary `f64` precision loss) using rules in `unit_conversions`, preserving case-sensitive `mL` (`MILLILITER`) vs `ML` distinctions, dynamic pollutant scaling (`KG*E` $\rightarrow$ `t*e`), and ratio numerators:
  - **Energy:** `MWh`, `kWh`, `toe`, `Gcal`, `MJ`, `J`, `TJ` $\rightarrow$ **Gigajoules (`GJ`)**
  - **Mass:** `kg`, `lbs`, `mg`, `g`, `kt`, `quintal` $\rightarrow$ **Metric Tonnes (`t`)**
  - **Volume:** `m³`, `L`, `mL`, `ML`, `gal`, `Million KL` $\rightarrow$ **Kiloliters (`KL`)**
  - **Emissions:** `tCO2e`, `kgCO2e`, `ktCO2e`, `MtCO2e`, `GtCO2e` $\rightarrow$ **Metric Tonnes $\text{CO}_2\text{e}$ (`tCO2e`)**
- **6-Tier Industry Resolution (`trg_enrich_entity_industry`):** Resolves entities via `exchange_industry_master` (ISIN $\rightarrow$ NSE Symbol $\rightarrow$ BSE Scrip $\rightarrow$ 21-char CIN $\rightarrow$ `pg_trgm` fuzzy name match), 5-digit & 2-digit NIC lookup in `industry_master`, and statutory entity keyword fallback.
- **Offline PIN-Code Geocoding (`trg_resolve_entity_coordinates`):** Scans registered and corporate office addresses right-to-left for Indian 6-digit PIN codes and resolves coordinates, state, and district centroids from `geo_pincode`.

### 3. Form Explorer, ESG Analytics, Geo Reports & Copilot

- **BRSR Form Explorer:** Driven by `form_layout` (`render_kind`, `visibility_parent_key`, `visibility_trigger`), rendering Section A (Q16 Assurance Scope, Q19 Locations, Q21a/b Workforce, Q22 Women Representation, Q23 3-Year Turnover, Q26 Stakeholder Complaints), Section B (9-Principle Matrix + Q7–9 governance cards), and Section C (P1 Training, P2 Reclaimed Waste, P3 Well-Being/Retirement/Parental Leave, P5 Remuneration, CY/PY Complaints matrices, and dynamic `Axis1..N` / CY-vs-PY tables).
- **19 Standardized ESG KPIs & Sector Medians:** Computes Environmental (Scope 1/2/3, Renewable Energy %, Water Withdrawal, Energy Intensity, Waste Intensity), Social (Board Women %, Female Wages %, Turnover %, Injuries, Fatalities, Female Workforce %), and Governance (POSH Upheld/Total, Complaints Filed/Pending, Disciplinary Actions, Well-Being Cost %) KPIs in `view_company_esg_kpis` while blocking Prior Year (`PY`/`PPY`) leakage and deduplicating sector medians to the latest filing per CIN.
- **Geospatial Map & Streaming Exports:** Interactive OpenStreetMap clustering via `walkers`, streaming `BufWriter` CSV exports (standard UTF-8 or Excel UTF-8 BOM), and streaming `pg_dump` backups supporting both native host and Podman/Docker PostgreSQL containers.
- **Grounded ESG Copilot:** Combines active view disclosures with batched PostgreSQL retrieval (`McpTools::query_company_metrics_batch`), routing prompts to Local Ollama (`/api/chat`) or Google Gemini (`v1beta` with `x-goog-api-key`), complete with latency/payload telemetry and Podman/Docker lifecycle controls.

---

## Directory Structure

```text
.
├── assets/
│   ├── cielo_mio.desktop                # Linux desktop launcher entry
│   ├── InclusiveSans-VariableFont_wght.ttf
│   └── logo.png
├── sql/
│   ├── 00_teardown_and_extensions.sql   # Drops legacy objects; enables btree_gist, pg_trgm, vector
│   ├── 01_file_list.sql                 # Download & ingestion state machine
│   ├── 02a_xbrl_tag_rules.sql           # Raw-to-canonical XBRL tag/context ETL rules
│   ├── 02b_form_layout.sql              # UI layout, mojibake cleaning trigger & visibility rules
│   ├── 03_industry_master.sql           # MOSPI 2-digit & 5-digit NIC fallback taxonomy
│   ├── 04_exchange_industry_master.sql  # SEBI 4-tier BSE/NSE master + normalize_company_name()
│   ├── 05_geo_pincode.sql               # Indian PIN master & coordinate resolution trigger
│   ├── 06_unit_conversions.sql          # Base-10 canonical unit conversion multipliers
│   ├── 07_entity_list.sql               # Corporate profiles, temporal FY bounds & industry triggers
│   ├── 08_brsr_facts.sql                # Normalized EAV disclosure repository
│   ├── 09_analytics_views.sql           # view_company_esg_kpis & view_sector_benchmarks
│   ├── 10_cluster_tuning.sql            # PostgreSQL 18 AIO/memory tuning parameters
│   ├── 99_data_dictionary.sql           # Snapshot table of schema columns & comments
│   ├── brsr_delete_create.sh            # Schema initialization & seed script
│   └── seeds/                           # Seed CSVs (industry_master, pin_code, unit_conversions, etc.)
├── src/
│   ├── ai/                              # RagAgent, LlmClient (Ollama/Gemini), McpTools
│   ├── domain/                          # Download types, EntityProfile, Facts, Taxonomy, Decimal Units
│   ├── pipeline/                        # GeocoderService, IngestionManager, BSE/NSE scrapers, XBRL parser & transform
│   ├── storage/                         # sqlx pool + repository.rs & repository/* modules
│   ├── system/                          # XDG TOML AppConfig, Podman/Docker lifecycle, OS Keyring SecretManager
│   ├── ui/                              # egui 0.37 shell, state, event drain & all 7 view tabs
│   ├── lib.rs
│   └── main.rs
└── Cargo.toml                           # Rust Edition 2024 dependencies & release/RPM metadata
```

---

## Quick Start

### Prerequisites

- **Rust:** Version 1.88+ (Edition 2024)
- **PostgreSQL:** Version 18+ with `btree_gist`, `pg_trgm`, and `vector` (`pgvector` — e.g., via Podman/Docker)
- **Optional Local LLM / Runtime:** Podman or Docker for managed PostgreSQL and Ollama containers

### Build

```bash
git clone <your-repo-url>
cd cielo_mio
cargo build --release
```

The optimized binary is written to `target/release/cielo_mio`.

### Database Initialization

Create the target database (default name: `brsr_db_test`) and initialize the ordered SQL scripts and seed data via `sql/brsr_delete_create.sh` or `psql`:

```bash
psql -h localhost -U postgres -d postgres -c "CREATE DATABASE brsr_db_test;"
for f in sql/[0-9]*.sql; do
    psql -h localhost -U postgres -d brsr_db_test -f "$f"
done
```

Load the seed CSVs in `sql/seeds/` (`unit_conversions.csv`, `industry_master.csv`, `pin_code.csv`, `xbrl_tag_rules`, and `form_layout`), then launch the application:

```bash
./target/release/cielo_mio
```

### Configuration & Credentials

- **Application config:** Automatically persisted at `$XDG_CONFIG_HOME/cielo_mio/config.toml` (defaults to `~/.config/cielo_mio/config.toml`).
- **Secrets:** Managed via the built-in **Settings** tab and stored in your OS Keyring (`cielo_mio` service) with environment-variable fallback:
  - Database password (`KEY_DB_PASSWORD` / `DB_PASSWORD`)
  - Google Gemini API key (`KEY_GEMINI_API` / `GEMINI_API_KEY`)

---

## Regulatory Alignment

- **SEBI BRSR & BRSR Core:** Supports Section A Q14–Q16 assessment/assurance schemas, Section B 9-principle disclosures, and Section C Essential/Leadership indicators.
- **MoEFCC Green Credit Programme:** Maps Principle 6 Leadership Indicator 8 disclosures.
- **Value Chain Thresholds:** Models upstream/downstream partner disclosures across SEBI's value chain boundary criteria.

---

## License

Distributed under the MIT License. See `LICENSE` for details.
