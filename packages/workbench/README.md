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
| **Upstream component** | Tally results (`ui-essentials`) | The production tally visualization, imported unmodified. A thin adapter maps velvet's output to its props. | None — same component production renders |

### Voting-portal (alias lift)

The portal's TypeScript sources under `packages/voting-portal/src/` are
**never modified or duplicated**. Instead, `vite.config.ts` aliases
`@sequentech/ui-core`, `@sequentech/ui-essentials`, and tsconfig path
shims (e.g. `@root/*`) so Vite compiles the portal sources on the fly.
Runtime services are stubbed by mock providers inside `workbench/app/`.

Full procedure, invariants, and canary list: [LIFTING.md](LIFTING.md).

### Tally results (upstream component)

The tally visualization (participation summary, pie charts, results
tables, IRV round-by-round) lives in `ui-essentials` as production code
shared with `results-portal`. The workbench imports it unmodified; a
`velvetTallyAdapter.ts` maps velvet's snake_case `ContestResult` onto
`ResultsAndParticipation`'s props. Labels are injected via the
component's own `labels` prop, which ships English defaults — no i18n
provider needed.

This was previously a copy lift into `ui-essentials`; that copy was
deleted once upstream shipped its own version.

Adapter mapping table and the two percentage conventions:
[LIFTING-TALLY.md](LIFTING-TALLY.md).

## Known gaps

**velvet-core lags production tally semantics.** The pure tally logic was
extracted from `packages/velvet` into `workbench/velvet-core`, which
`packages/velvet` now re-exports. Upstream subsequently landed ~950 lines
of new tally behaviour into velvet's `do_tally` across five feature PRs —
explicit blank votes in encoding (#2842), tally sheets input (#1929),
consistent invalid vote policy (#2697), election-level decline to vote
(#2687) and browser-based trustees (#2198). Those changes have **not**
been forward-ported into `velvet-core`, so tally results computed here
can diverge from production for ballots exercising those features. Since
the workbench exists to check fidelity against production, treat results
involving blank/invalid/decline semantics as unverified until this is
reconciled. The same caveat applies to whatever those PRs changed in
`strand`.

**`yarn build` (`tsc -b`) does not pass.** The dev server is the
supported workflow. Three separate causes: `tsconfig.json` uses
`erasableSyntaxOnly` (TypeScript ≥ 5.8) while the app pins `~5.7.2`; the
deprecated `@types/minimatch` stub trips `TS2688`; and `tsc` does not
read Vite's `resolve.alias`, so it cannot resolve
`@sequentech/ui-core` / `@sequentech/ui-essentials` and ends up
type-checking the lifted portal sources under the workbench's stricter
flags. Fixing it means mirroring the Vite aliases as tsconfig `paths` and
excluding portal sources from the check.

## License

AGPL-3.0-only
