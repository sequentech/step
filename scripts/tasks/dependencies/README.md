<!--
SPDX-FileCopyrightText: 2023-2024 Eduardo Robles <edu@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Third-Party Dependencies Documentation System

This directory contains an automated system for scanning, tracking, and
documenting all third-party dependencies across the Sequent Voting Platform
(SVP) monorepo packages.

## Overview

The system consists of three main components:

1. **`list_deps.py`** - Scans packages and generates CSV dependency data
2. **`generate_docs.py`** - Updates markdown documentation from CSV data  
3. **`generate-dependency-report.sh`** - Orchestrates the complete process

## Quick Start

### Using VS Code (Recommended)

The easiest way to update the dependency documentation is through VS Code:

1. Open the Command Palette (`Ctrl+Shift+P` / `Cmd+Shift+P`)
2. Search for `Tasks: Run Task`
3. Select `generate.dependency.report`

This will automatically:
- Scan all packages for direct dependencies
- Generate the CSV report with current license information
- Update the markdown documentation while preserving existing structure
- Output results to `docs/docusaurus/docs/reference/third_party_deps/`

### Manual Usage

Run the complete pipeline manually:

```bash
./scripts/tasks/dependencies/generate-dependency-report.sh
```

## System Architecture

### Dependency Scanning (`list_deps.py`)

Scans the step repository and extracts dependencies from:
- **Rust packages**: `Cargo.toml` files, queries [crates.io](https://crates.io)
  API
- **TypeScript/JavaScript packages**: `package.json` files, queries
  [npmjs.com](https://npmjs.com) API  
- **Java packages**: `pom.xml` files, queries [Maven
  Central](https://central.sonatype.com) API

Outputs a CSV file with columns: `Package`, `Dependency`, `Version`, `License`,
`Description`.

### Documentation Generation (`generate_docs.py`)

Intelligently updates the existing markdown documentation by:
- Automatically detecting package sections in the markdown (e.g., "Admin
  Portal", "Voting Portal")
- Mapping CSV package names to markdown section headers
- Preserving all existing content (descriptions, overview, license compliance
  sections)
- Updating only the dependency tables with current data from CSV

### Pipeline Orchestration (`generate-dependency-report.sh`)

The wrapper script:
1. Sets up Python virtual environment
2. Installs required dependencies (`requests`, `tomli`)
3. Runs dependency scanning to generate CSV
4. Updates markdown documentation from CSV data
5. Outputs to `docs/docusaurus/docs/reference/third_party_deps/`

## Output Files

- **CSV**: `docs/docusaurus/docs/reference/third_party_deps/assets/dependencies.csv`
- **Markdown**: `docs/docusaurus/docs/reference/third_party_deps/third_party_deps.md`

The markdown file is automatically included in the Docusaurus documentation site
and accessible at `/docs/reference/third_party_deps/third_party_deps`.

## License Information

The system fetches actual license information from package registries, ensuring
accurate compliance data instead of placeholder "N/A" values. Common license
formats include:
- `MIT`
- `Apache-2.0` 
- `MIT OR Apache-2.0`
- `BSD-3-Clause`

---

**Note:** All operations preserve existing documentation structure and only
update dependency data, making it safe to run repeatedly without losing manual
edits to package descriptions or other content.
