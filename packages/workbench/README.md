<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Workbench

A self-contained sandbox for exercising production components of the Sequent
Voting Platform in isolation — no external services, no remote calls. All
computation (ballot encoding/decoding, encryption, tally) runs client-side
via WASM.

## Features

- **Snapshot inspector** — load bundled or custom election snapshots;
  browse ballot styles, contests, voters, and their relationships.
- **Ballot pipeline** — per-stage encode → encrypt → decrypt → decode
  round-trip playground with N-ballot generation.
- **Tally sandbox** — run velvet tally algorithms in-browser against
  decrypted plaintexts.
- **Booth spike** — lifted production voting-portal screens running
  against local fixture data.
- **Policy overrides** — per-contest runtime overrides for presentation
  policies and min/max vote bounds.

## Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Node.js | ≥ 20 | JS runtime |
| Corepack / Yarn | (bundled) | Package manager |
| Rust + wasm-pack | stable | Compile `velvet-wasm` and `sequent-core` to WASM |

## Development

All commands run from the monorepo `packages/` root:

```sh
# Start the dev server (auto-builds velvet-wasm before starting)
corepack yarn workspace "@sequentech/workbench-app" dev
```

Vite serves on `http://localhost:5173` with HMR. The `predev` hook
compiles `velvet-wasm` automatically; if you also edit sequent-core Rust
you must manually run:

```sh
corepack yarn workspace "@sequentech/workbench-app" build:sequent-core
```

## Production build

```sh
corepack yarn workspace "@sequentech/workbench-app" build
```

This runs `prebuild` (wasm-pack) → `tsc -b` (type-check) → `vite build`.
Output lands in `workbench/app/dist/` — fully static HTML/JS/WASM with no
runtime server dependencies.

### Serving the build

The app uses client-side routing (`createBrowserRouter`), so the HTTP server
must rewrite unknown paths to `index.html` (SPA fallback). It must also
serve `.wasm` files with `application/wasm` MIME type.

| Method | Command |
|--------|---------|
| **Local preview** | `corepack yarn workspace "@sequentech/workbench-app" preview` |
| **One-liner** | `npx serve -s workbench/app/dist` |
| **nginx** | Use the repo's [`default.conf`](../default.conf) (`try_files $uri $uri/ /index.html`) |
| **Docker** | `docker build -f Dockerfile.prod -t workbench .` then `docker run -p 8000:8000 workbench` |

> **Note:** Simple servers without SPA fallback (e.g. `python -m http.server`)
> will 404 on deep-link refresh. Use one of the options above.

## Project structure

```
workbench/
├── app/                 React + Vite application
│   ├── src/
│   │   ├── main.tsx              Router & app shell
│   │   ├── WorkbenchInspector.tsx Snapshot/contest/voter pages
│   │   ├── BallotPipeline.tsx    Encode→encrypt→decrypt→decode sandbox
│   │   ├── TallyPage.tsx         Tally sandbox
│   │   ├── BoothSpike.tsx        Lifted voting-portal screens
│   │   ├── fixtures/snapshots/   Bundled election snapshots (validated at build)
│   │   └── lib/                  Shared utilities
│   └── vite.config.ts            Build plugins & alias resolution
├── velvet-core/         Pure-computation tally crate (wasm32 target)
├── velvet-wasm/         wasm-bindgen wrapper exposing velvet-core to JS
├── LIFTING.md           Procedure for embedding voting-portal source
└── LIFTING-TALLY.md    Tally-specific lifting notes
```

## Embedding strategy

Dependencies fall into three categories:

| Strategy | Example | Mechanism | Drift risk |
|----------|---------|-----------|------------|
| **Shared source** | `velvet-core` | Real crate consumed identically by production and workbench | None — same source |
| **Alias lift** (in-place) | `voting-portal`, `ui-core` | Vite `resolve.alias` points at the original source files in their upstream packages; no files are copied. Substitute providers replace services the portal normally talks to (Keycloak, Hasura, REST). | High — many upstream files; silent drift requires periodic reconciliation |
| **Copy lift** | Tally result components from `admin-portal` | Source files copied (with adaptations) into `ui-essentials/src/components/TallyResults/`. Thin adapter maps velvet's output to the shape the components expect. | Low — small surface, stable upstream |

### Voting-portal (alias lift)

The portal's TypeScript sources under `packages/voting-portal/src/` are
**never modified or duplicated**. Instead, `vite.config.ts` aliases
`@sequentech/ui-core`, `@sequentech/ui-essentials`, and tsconfig path
shims (e.g. `@root/*`) so Vite compiles the portal sources on the fly.
Runtime services are stubbed by mock providers inside `workbench/app/`.

Full procedure, invariants, and canary list: [LIFTING.md](LIFTING.md).

### Tally results (copy lift)

The admin-portal's tally visualization (pie charts, results tables, IRV
round-by-round) was re-hosted into `ui-essentials` as a permanent
library component. A `velvetTallyAdapter.ts` in the workbench maps
velvet's snake_case `ContestResult` to the plain TS shape the components
expect. i18n is replaced by a hardcoded English string shim.

Full procedure and adaptation inventory: [LIFTING-TALLY.md](LIFTING-TALLY.md).

## License

AGPL-3.0-only
