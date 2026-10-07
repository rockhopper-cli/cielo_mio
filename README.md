<div align="center">

  <img src="assets/logo.png" alt="CieloMio Logo" width="140" />

# SEBI BRSR Extractor & ESG Analytics Suite

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B%20(Edition%202024)-orange.svg)](https://www.rust-lang.org/)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-18%2B-blue.svg)](https://www.postgresql.org/)
[![SEBI Compliance](https://img.shields.io/badge/SEBI-BRSR%20Core%20Aligned-green.svg)](https://www.sebi.gov.in/)

</div>

---
**Cielo Mio** is an enterprise-grade extraction, normalization, and analytics pipeline for the Securities and Exchange Board of India's (SEBI) **Business Responsibility and Sustainability Reporting (BRSR)** filings.

The application crawls, downloads, parses, and standardizes heterogeneous XBRL/XML corporate disclosures submitted by listed entities to the **National Stock Exchange (NSE)** and **Bombay Stock Exchange (BSE)**. It handles context reconciliation, non-standard unit conversions, dimensional axis expansions, and taxonomy adjustments (including BRSR Core, Green Credits, and Value Chain updates).

---

## Architecture Overview
```Code
[BSE / NSE Exchanges]
                                            │
                                (Automated Sync Engine)
                                            ▼
                                [Raw XML/XBRL Repository]
                                            │
┌──────────────────────────────────────────────┴─────────────────────────────────────────────┐
│ EXTRACTION PIPELINE │
│ │
│ [Master Taxonomy / form_master] [Multi-Threaded / Async Parser] │
│ • Dynamic Axis Expansion • Namespace Isolation (in-capmkt) │
│ • Suffix / Increment Rules • Context Mapping (CY vs. PY) │
│ • Datatype & Routing Metadata • Nested Unit Resolution (<divide>) │
│ │ │ │
│ └────────────────────────────┬────────────────────────┘ │
│ ▼ │
│ [Enrichment & Transform] │
│ • Canonical Unit Normalization (GJ, t, KL, tCO2e) │
│ • Number Parsing (Parentheses, Currencies, Zeroish) │
│ • Statutory Priority Layer (CY > PY Protection) │
└──────────────────────────────────────────────┬─────────────────────────────────────────────┘
│
┌────────────────────────┴────────────────────────┐
▼ ▼
[Relational Database] [Pivoted Relational CSVs]
PostgreSQL Storage Layer • entity_list.csv (Wide Profile)
• entity_list (Corporate Profile) • {table}_{datatype}.csv
• brsr_facts (Normalized Disclosures)
• file_list (Sync State Machine)
│
▼
[Analytics & Copilot]
• 18 Standardized ESG KPIs
• Cross-Company Benchmarking
• Local (Ollama) / Cloud (Gemini) RAG
```
---

## Key Capabilities

### 1. Robust Exchange Ingestion
* **Simultaneous Dual Sync:** Concurrent crawlers for BSE (ASP.NET postback state management) and NSE (session warmup with sliding $\le 365$-day query windows).
* **Idempotent Storage:** Tracks MD5 hashes, timestamps, and upload dates to prevent duplicate downloads and bypass previously mapped filings.
* **Failure Cutoff Engine:** Isolates damaged filings, records error reasons (`unmapped_reason`), and prevents infinite reprocessing loops during automated ingestion runs.

### 2. High-Precision Parsing & Normalization
* **Namespace Resolution:** Targets SEBI's canonical `in-capmkt` taxonomy across all filing iterations (2021–2026+).
* **Canonical Unit Conversions:** Automatically converts heterogeneous measures into standard physical bases:
  * **Energy:** `MWh`, `kWh`, `toe`, `Gcal`, `MJ` $\rightarrow$ **Gigajoules (`GJ`)**
  * **Mass:** `kg`, `lbs`, `mg`, `quintal` $\rightarrow$ **Metric Tonnes (`t`)**
  * **Volume:** `m³`, `L`, `gal`, `million liters` $\rightarrow$ **Kiloliters (`KL`)**
  * **Emissions:** Direct conversion to standard metric tons of $\text{CO}_2$ equivalent ($\text{tCO}_2\text{e}$).
  * **Ratios:** Dynamic numerator conversions for complex rates (e.g., `MWhPerINR` $\rightarrow$ `GJ/INR`).
* **Dynamic Suffix Expansion (`contextRef_increment`):** Scans multi-member XBRL dimensions (e.g., multiple stock exchanges, assessment providers, trade unions) and clones taxonomy rules dynamically.

### 3. Comprehensive Governance & ESG Analytics
* **Current-Year vs. Prior-Year Context Segregation:** Disclosures are tagged by temporal priority (`CurrentYear` vs. `PreviousYear`), protecting KPIs against inversion bugs where prior-year values exceed current-year values.
* **18 Standardized Core Metrics:** Instant extraction across Environmental (Scope 1/2/3, Water, Energy Intensity), Social (Gender Diversity, Pay Gap, Fatalities, Turnover), and Governance (POSH Complaints, Disciplinary Penalties).
* **Interactive Desktop GUI:** Built using native `egui` with zero-allocation memoized filtering, collapsible navigation rails, and searchable dropdowns for directories of 2,000+ listed companies.
* **Statutory Grounded RAG (Copilot):** In-memory and PostgreSQL-backed Retrieval-Augmented Generation using local Ollama (`qwen2.5-coder`) or Google Gemini, citing verified figures directly from statutory forms.

---

## Directory Structure
```Code
.
├── assets/ # Static branding, fonts, and launcher icons
├── docs/
│ ├── LOGIC_AND_SPECIFICATION.md # Complete mathematical & pipeline logic specification
│ └── database_schema.sql # PostgreSQL DDL for relational deployment
├── src/ # Core Rust Desktop Suite (Cielo Mio)
│ ├── ai/ # Grounded RAG agent, client, and tools
│ ├── app/ # egui desktop views (Analytics, Viewer, Downloader)
│ ├── config.rs # XDG-compliant persistent configuration
│ ├── container.rs # Podman/Docker automated container lifecycle
│ ├── downloader/ # BSE/NSE HTTP engine, stream writers, scrapers
│ ├── parser/ # quick-xml parsing, unit conversions, taxonomy mappings
│ └── security.rs # OS Keyring credentials integration (KWallet/SecretService)
├── python_legacy/ # Legacy Python CLI & Multi-processing ETL pipeline
│ ├── config_loader.py # Excel/CSV taxonomy loader
│ ├── parsing.py # Multi-core XBRL parsing pool
│ ├── pipeline.py # Chunked CSV batch flusher
│ └── transform.py # Dynamic axis expansion & pivoting
├── Cargo.toml # Optimized Rust compilation profiles & dependencies
└── requirements.txt # Dependencies for legacy Python ETL scripts
```
---

## Quick Start

### Prerequisites
* **Rust:** Version 1.85+ (Edition 2024 recommended)
* **PostgreSQL:** Version 14+ (Local instance or containerized via Podman/Docker)
* **Python (Optional):** Version 3.10+ (for running legacy batch ETL scripts)

### Installation & Build

```bash
# Clone the repository
git clone https://github.com/your-org/cielo_mio.git
cd cielo_mio

# Compile optimized desktop release binary
cargo build --release
```

The resulting binary will be located at target/release/cielo_mio.

### Database Initialization

Ensure PostgreSQL is running and initialize the schema:
```bash
psql -h localhost -U postgres -d postgres -c "CREATE DATABASE brsr_db_test;"
psql -h localhost -U postgres -d brsr_db_test -f docs/database_schema.sql
```

### Running the Desktop Application
```bash
./target/release/cielo_mio
```

Configure your database password via the built-in Settings tab. The password will be stored securely in your OS Keyring (GNOME Keyring, KWallet, or Windows Credential Manager).

## Running the Python Batch ETL (Headless)

If you are running batch migrations without the GUI:

```bash

cd python_legacy
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt

# Run headless pipeline across all CPU cores
python cli.py \
    --input ./downloads/nse/xml \
    --output ./exports \
    --config ./assets/brsr_form_master.xlsx \
    --market NSE

```
## Master Taxonomy Table (`form_master`)

The extraction engine relies on a standardized mapping dictionary. Out of 35 metadata fields, the pipeline relies on **7 core architectural columns**:

| Field | Type | Function |
| :--- | :--- | :--- |
| `in_capmkt` | `Text` | Primary raw XBRL tag identifier within the XML element. |
| `contextRef` | `Text` | Explicit context ID defining company dimension, duration, or axis. |
| `standard_in_capmkt` | `Text` | Unified target taxonomy tag across filing iterations. |
| `standard_contextRef` | `Text` | Canonical context identifier (`CurrentYear`, `PreviousYear`, etc.). |
| `associated_table` | `Text` | Relational table or CSV output file route (e.g., `section_a_0_1`). |
| `friendly_grouping` | `Text` | Analytical functional category (e.g., `details_ghg`, `entity_list`). |
| `contextRef_increment` | `SmallInt` | Flag (`1` or `0`) signaling dynamic index cloning (`Axis1`, `Axis2`...). |

---

## Regulatory Compliance & SEBI Updates

Cielo Mio incorporates recent regulatory adjustments from SEBI:

1. **MoEFCC Green Credit Programme:** Integrated Leadership Indicator 8 under Principle 6 tracking both standalone entity and top-10 value chain partner generation/procurement of Green Credits.
2. **BRSR Core "Assessment or Assurance":** Updated Section A assurance schemas supporting Industry Standards Forum (ISF) reasonable/limited verification standards alongside classic audit checks.
3. **Value Chain Thresholding:** Data structures model upstream and downstream partner impacts across the revised threshold of partners accounting for **≥ 2%** of purchases/sales (up to the 75% coverage ceiling).

---

## License

Distributed under the MIT License. See `LICENSE` for more information.