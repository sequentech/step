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

Every non-trivial dependency the workbench pulls in is listed below —
this table is exhaustive, not illustrative. (Ordinary npm packages —
React, MUI, Apollo, Redux, react-router, i18next — are omitted; they are
resolved normally and have no embedding story.)

| Strategy | Dependency | Mechanism |
|----------|-----------|-----------|
| **Shared source** | `velvet-core`, `sequent-core` (Rust side) | Real crate consumed identically by production and workbench, via a Cargo path dep. Breakage is a compile error. |
| **Prebuilt artifact** | `sequent-core` (JS/wasm side) | The lifted booth's `import … from "sequent-core"` resolves to `node_modules/sequent-core`, unpacked from the committed `rust/sequent-core-0.1.0.tgz` that `voting-portal` / `ui-core` declare — the workbench app never declares it. An opt-in `resolve.alias` redirects to `packages/sequent-core/pkg` when a local `wasm-pack` build exists (§A7). |
| **Alias lift** (in-place) | `voting-portal`, `ui-core`, `ui-essentials` | Vite `resolve.alias` points at the original source files in their upstream packages; no files are copied. Substitute providers replace services the portal normally talks to (Keycloak, Hasura, REST). |
| **Upstream component** | Tally results (in `ui-essentials`) | The production tally visualization, imported unmodified. A thin adapter maps velvet's output to its props. Rides the `ui-essentials` alias above, but needs no substitutes. |
| **Workbench-owned** | `velvet-wasm` | The workbench's own `wasm-bindgen` layer over `velvet-core` + `sequent-core`, consumed as `file:../velvet-wasm/pkg`. Not embedded from anywhere — it is the vehicle that gets the Rust into the browser. |

### Where drift actually lives

This table deliberately does **not** rank drift risk per row. An earlier
version did, and it was misleading: it scored the mechanisms on how
faithfully workbench and production see the same code *within this
branch*, which for a shared crate is trivially "none — same source".

That is the wrong axis. The risk that matters is **this branch versus
`main`**, and by that measure the rows scoring best above are among the
worst. `velvet-core` is "shared source" and therefore drift-free
in-branch, yet it is the single largest divergence we carry, because the
extraction that makes it shared has not landed upstream and main's tally
logic keeps moving. Conversely a row can be modified in-branch and pose
almost no catch-up risk, as `sequent-core` does — build-enablement edits
that upstream will accept or that rebase trivially.

So drift is tracked where it can be measured rather than guessed:

- **Live**, per subtree, on the workbench's own **Diagnostics page**
  (`/wb` → Diagnostics → *Shared-source drift*). Each tracked tree is
  diffed `HEAD` vs the merge-base with `origin/main`, and carries an
  `expectation` describing what should be there — so an undocumented
  change reads as undocumented. It also reports how many commits
  `origin/main` has that this branch doesn't.
- **Narratively**, in [Known gaps](#known-gaps) below for the divergences
  we have accepted, and in `LIFTING.md` §L for every edit made to
  production source.

**The version-skew trap.** `sequent-core` reaches the workbench through
*two* of these rows at once: the booth encrypts using the prebuilt tgz,
while `velvet-wasm` decrypts and tallies using `packages/sequent-core`
Rust source compiled by `wasm-pack`. Nothing keeps those in step. If the
tgz is regenerated upstream (or you edit the Rust) without rebuilding
`velvet-wasm`, the two halves of the encrypt → decrypt → tally loop run
different versions of the encoding rules, and the mismatch surfaces as a
wrong `BigUint` rather than an error. After any change to either, rebuild
`velvet-wasm` and re-run the §M.4 canary in [LIFTING.md](LIFTING.md).

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

**The velvet-core extraction has not landed upstream.** The pure tally
logic was extracted from `packages/velvet` into `workbench/velvet-core`,
which `packages/velvet` now re-exports — but `main` still owns an inline
copy, so `packages/workbench` does not exist there at all. Every catch-up
merge therefore re-conflicts on velvet's `do_tally`, and upstream tally
changes have to be *forward-ported* into velvet-core rather than merged.
The split is real, not cosmetic — for example velvet-core's IRV
tie-break takes `rng: &mut dyn RngCore` from its caller where upstream
calls `thread_rng()` internally, because velvet-core depends only on
`rand_core` and not on `rand`, keeping the wasm `getrandom` version
footprint minimal. Landing the extraction upstream is what would stop
this recurring; until then, budget for the port on each catch-up.

Tally semantics are currently **up to date** with `origin/main`: the
explicit/implicit blank split, the invalid-vote policy, decline-to-vote
and participation-by-channel were all ported into velvet-core, along
with upstream's tests for them (16 passing).

**strand carries an unreconciled divergence.** This branch removed the
obsolete openssl/FIPS backends to reach wasm32, and merges have resolved
`packages/strand` in favour of this branch — so any strand changes from
upstream feature PRs have been discarded rather than merged. Unlike
velvet-core, nothing has been forward-ported here.

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
