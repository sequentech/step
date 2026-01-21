<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
# WBraid - Braid WASM Migration

Migration of the Braid crate to support WebAssembly targets alongside native builds. See [summary.md](summary.md) for detailed information about the migration strategy and architecture.

## Quick Start

### Prerequisites

- Rust (stable + nightly)
- Docker (for LocalStack)
- AWS CLI: `winget install Amazon.AWSCLI`
- Python 3 (for dev server)

### 1. Start LocalStack

```powershell
# Start LocalStack container
docker run -d -p 4566:4566 -p 4510-4559:4510-4559 `
  -e HOSTNAME_EXTERNAL=localhost `
  -e S3_HOSTNAME=localhost:4566 `
  localstack/localstack

# Create S3 bucket and configure CORS
aws --endpoint-url=http://localhost:4566 s3 mb s3://wbraid-messages
aws --endpoint-url=http://localhost:4566 s3api put-bucket-cors --bucket wbraid-messages --cors-configuration file://s3-cors.json
```

### 2. Run Bulletin Board Service

```powershell
.\bb.ps1
```

Service starts on `http://127.0.0.1:3000`

### 3. Run WASM Client

```powershell
.\serve.ps1
```

Opens browser at `http://127.0.0.1:8080`

### 4. Run Tests

```powershell
# Native tests (default features)
cargo test --release

# WASM-specific tests
cd crates/client
cargo test --target wasm32-unknown-unknown --release
```

## Key Files

- **bb.ps1** - Starts bulletin board service with LocalStack configuration
- **serve.ps1** - Builds WASM client and starts dev server
- **build-wasm.ps1** - Builds WASM with atomics support (called by serve.ps1)
- **summary.md** - Detailed migration documentation and architecture

## Architecture

This workspace demonstrates a feature-gated approach to WASM compatibility:

- **Native mode** (default): Full features including filesystem-based storage
- **WASM mode** (`wasm-core` feature): Browser-compatible subset with stub storage

Core crates:
- `b3` - Message types with `HttpB3Message` for universal HTTP transport
- `braid` - Main library with feature-gated `Trustee`/`WasmSession` implementations
- `client` - WASM proof-of-concept with atomics/SharedArrayBuffer support
- `service` - REST API bulletin board service

See [summary.md](summary.md) for complete details on the migration strategy, completed work, and next steps.
