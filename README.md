<div align="center">

  <img src="assets/logo.png" alt="CieloMio Logo" width="140" />

# SEBI BRSR Extractor & ESG Analytics Suite

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](<https://img.shields.io/badge/Rust-1.85%2B%20(Edition%202024)-orange.svg>)](https://www.rust-lang.org/)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-18%2B-blue.svg)](https://www.postgresql.org/)
[![SEBI Compliance](https://img.shields.io/badge/SEBI-BRSR%20Core%20Aligned-green.svg)](https://www.sebi.gov.in/)

</div>

---

**Cielo Mio** is an enterprise-grade extraction, normalization, and analytics suite for the Securities and Exchange Board of India's (SEBI) **Business Responsibility and Sustainability Reporting (BRSR)** filings.

The application crawls, downloads, parses, and standardizes heterogeneous XBRL/XML corporate disclosures submitted by listed entities to the **National Stock Exchange (NSE)** and **Bombay Stock Exchange (BSE)**. It handles context reconciliation, exact base-10 unit conversions (`rust_decimal`), dimensional axis expansions, multi-tier industry classification, and taxonomy adjustments (including BRSR Core, Green Credits, and Value Chain updates).

---

## Architecture Overview

```Code
[BSE / NSE Exchanges]
                                            │
                                (Automated Sync Engine)
                                            ▼
                                [Raw XML/XBRL Repository]
                                            │
┌───────────────────────────────────────────┴────────────────────────────────────────────┐
│                                 RUST EXTRACTION PIPELINE                               │
│                                                                                        │
│ [Master Taxonomy / form_master]                  [Streaming quick-xml Parser]          │
│ • O(1) Tag-Indexed Axis Expansion                • UTF-8 / UTF-16 BOM Auto-Decoding    │
│ • Suffix / Increment Rules                       • Namespace Isolation (in-capmkt)     │
│ • Datatype & Section/Principle Routing           • Nested Unit Resolution (<divide>)   │
│                                                                                        │
│                                           │                                            │
│                                           ▼                                            │
│                               [Enrichment & Transform]                                 │
│ • Exact Decimal Unit Normalization (GJ, t, KL, tCO2e)                                  │
│ • Context Priority Layer (Main/CY > Axis > PY/PPY)                                     │
│ • Multi-Key Industry Classification (Exchange Master -> NIC 5/2-Digit -> Statutory)    │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
                       [PostgreSQL 18+ Relational Storage Layer]
                       • entity_list (Corporate Profile + Geo/Industry)
                       • brsr_facts (Normalized EAV Disclosures)
                       • file_list (Idempotent Sync State Machine)
                       • view_company_esg_kpis & view_sector_benchmarks
                                            │
                                            ▼
                        [Native Desktop Suite (egui 0.37 / eframe)]
                        • Form Explorer (egui_extras::TableBuilder Grids)
                        • 19 Standardized ESG KPIs & Sector Benchmarking
                        • Interactive Geospatial Map (walkers / OpenStreetMap)
                        • Grounded RAG Copilot (Local Ollama / Google Gemini)
```

---

## Key Capabilities

### 1. Robust Exchange Ingestion

- **Simultaneous Dual Sync:** Concurrent crawlers for BSE (JSON + `CorpXbrlGen.aspx` fallback) and NSE (session warmup with sliding $\le 365$-day query windows).
- **Idempotent Storage:** Tracks upload dates, timestamps, authoritative exchange tickers (`symbol`, `scrip_code`), and mapped status in `file_list` while rejecting HTML error/WAF responses.
- **Failure Cutoff & Recovery:** Isolates damaged filings (`unmapped_reason`) and supports one-click failure reset in the Downloader tab.

### 2. High-Precision Parsing & Normalization

- **Namespace & Encoding Resolution:** Targets SEBI's canonical `in-capmkt` taxonomy across filing iterations (2021–2026+) with UTF-8/UTF-16 BOM support and XML entity sanitization.
- **Exact Decimal Unit Conversions:** Uses `rust_decimal` base-10 multipliers loaded from `unit_conversions` (preserving `mL` vs `ML` and ratio numerators):
  - **Energy:** `MWh`, `kWh`, `toe`, `Gcal`, `MJ`, `J`, `TJ` $\rightarrow$ **Gigajoules (`GJ`)**
  - **Mass:** `kg`, `lbs`, `mg`, `g`, `kt`, `quintal` $\rightarrow$ **Metric Tonnes (`t`)**
  - **Volume:** `m³`, `L`, `mL`, `ML`, `gal`, `Million KL` $\rightarrow$ **Kiloliters (`KL`)**
  - **Emissions:** `tCO2e`, `kgCO2e`, `ktCO2e`, `MtCO2e`, `GtCO2e` $\rightarrow$ **Metric Tonnes $\text{CO}_2\text{e}$ (`tCO2e`)**
- **Dynamic Suffix Expansion (`contextref_increment`):** Indexes axis rules by tag in $O(1)$ time to clone multi-member dimensions (`Axis1`, `Axis2`...) without misrouting prior-year variants.
- **Multi-Tier Industry Classification:** Resolves every entity through `exchange_industry_master` (ISIN $\rightarrow$ NSE Symbol $\rightarrow$ BSE Scrip $\rightarrow$ CIN $\rightarrow$ trigram name), deterministic 5-digit & 2-digit NIC-2008 / legacy NIC-2004 lookup via `industry_master`, and statutory keyword fallback, with automatic backfill on DB connection.

### 3. Comprehensive Form Explorer, ESG Analytics & Copilot

- **Table-Driven Form Explorer (`egui_extras::TableBuilder`):** Renders statutory BRSR tables natively across Section A (Q10 Exchanges, Q16 BRSR Core Assurance, Q17–18 Activities/Products, Q19–20 Locations/Markets, Q21a/b Employees & Differently Abled, Q22 Women Representation, Q23 3-Year Turnover, Q24 Subsidiaries, Q26 Stakeholder Complaints, Q27 Material Issues), Section B (9-Principle Matrix + Q7–9 cards), and Section C (dynamic axis tables, multi-dimensional HR/Well-being/Wages/Complaints matrices, and Current FY vs Previous FY comparative grids).
- **19 Standardized ESG KPIs & Sector Medians:** Tracks Environmental (Scope 1/2/3, Renewable Energy %, Water, Energy Intensity, Waste Intensity), Social (Board Women %, Female Wages %, Turnover %, Injuries, Fatalities, Female Workforce %), and Governance (POSH Upheld/Total, Complaints Filed/Pending, Disciplinary Actions, Well-Being Cost %), protecting KPIs against prior-year (`PY`) context inversion.
- **Geospatial & Export Tooling:** Interactive OpenStreetMap clustering via `walkers` backed by offline `geo_pincode` centroid resolution, UTF-8 / Excel-BOM CSV exports, and direct `pg_dump` backups.
- **Statutory Grounded RAG (Copilot):** PostgreSQL-backed and in-memory Retrieval-Augmented Generation using local Ollama (`qwen2.5-coder`) or Google Gemini (`x-goog-api-key`), citing verified statutory figures and units.

---

## Directory Structure

```Code
.
├── assets/                     # Static branding, fonts, and launcher icons
├── docs/
│   └── database_schema.sql     # PostgreSQL 18+ DDL, triggers, seeds & analytical views
├── src/                        # Core Rust Desktop Suite (cielo_mio)
│   ├── ai/                     # Grounded RAG agent, Gemini/Ollama client, SQL tools
│   ├── domain/                 # Download types, EntityProfile, Facts, Taxonomy, Decimal Units
│   ├── pipeline/               # Geocoder, IngestionManager, BSE/NSE scrapers, XBRL parser & transform
│   ├── storage/                # sqlx PostgreSQL pool & repositories (analytics, entities, facts, files, taxonomy)
│   ├── system/                 # XDG TOML config, Podman/Docker lifecycle, OS Keyring security
│   ├── ui/                     # Native egui 0.37 shell, app state, events & all 7 view tabs
│   ├── lib.rs
│   └── main.rs
└── Cargo.toml                  # Rust Edition 2024 dependencies & release profiles
```

---

## Quick Start

### Prerequisites

- **Rust:** Version 1.85+ (Edition 2024)
- **PostgreSQL:** Version 18+ (Uses native `uuidv7()`, `btree_gist`, `pg_trgm`, and `vector` extensions — local or Podman/Docker)

### Installation & Build

```bash
# Clone the repository
git clone https://github.com/your-org/cielo_mio.git
cd cielo_mio

# Compile optimized desktop release binary
cargo build --release
```

The resulting binary will be located at `target/release/cielo_mio`.

### Database Initialization

Ensure PostgreSQL 18+ is running and initialize the schema:

```bash
psql -h localhost -U postgres -d postgres -c "CREATE DATABASE brsr_db_test;"
psql -h localhost -U postgres -d brsr_db_test -f docs/database_schema.sql
```

Load your `form_master.csv` (and optional `geo_pincode` GeoNames postal seed) into `form_master` and `geo_pincode`, then launch the application:

```bash
./target/release/cielo_mio
```

Configure your database password and Gemini API key via the built-in **Settings** tab (persisted securely in your OS Keyring: Secret Service / KWallet / Windows Credential Manager).

---

## Regulatory Compliance & SEBI Updates

Cielo Mio incorporates recent regulatory adjustments from SEBI:

1. **MoEFCC Green Credit Programme:** Integrated Leadership Indicator 8 under Principle 6 tracking both standalone entity and top-10 value chain partner generation/procurement of Green Credits.
2. **BRSR Core "Assessment or Assurance":** Updated Section A Q14–Q16 assurance schemas supporting reasonable/limited assessment & assurance verification alongside classic audit checks.
3. **Value Chain Thresholding:** Data structures model upstream and downstream partner impacts across the revised threshold of partners accounting for **≥ 2%** of purchases/sales (up to the 75% coverage ceiling).

---

## License

Distributed under the MIT License. See `LICENSE` for more information.
