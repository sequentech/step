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
- **Validation characterization** — recorded tables of the vote-validation
  behaviour: an exhaustive headless sweep and a browser DOM-validation
  runner checking production against one executable spec, the analyses
  built on it, the findings they surfaced, and reviewer reproduction
  recipes. See
  [characterization/README.md](characterization/README.md) and
  [docs/UPSTREAM_FINDINGS.md](docs/UPSTREAM_FINDINGS.md).

## Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Node.js | ≥ 20 (verified on 24) | JS runtime. 20.x is EOL and no longer offered by most installers; nothing here depends on it. |
| Yarn | 1.x (classic) | Package manager — the repo is a Yarn workspace |
| Rust + wasm-pack | stable | Compile `velvet-wasm` and `sequent-core` to WASM (both built by `predev`/`prebuild`) |
| `wasm32-unknown-unknown` | — | `rustup target add wasm32-unknown-unknown` |

Nothing above requires a C toolchain: the workbench's Rust stack
(`velvet-core` → `velvet-wasm`) has no native dependencies. Building
**`velvet` itself** is a different matter — it pulls
`sequent-core/reports`, hence `reqwest`/rustls, hence `aws-lc-sys`, which
needs cmake and nasm on Windows. You only need those if you are
compile-checking velvet, not to run the workbench.

**Ordering gotcha on a fresh clone:** build `velvet-wasm` *before* the
first `yarn install`. The app depends on it as
`file:../velvet-wasm/pkg`, so if `pkg/` does not exist yet the install
fails with `Package "velvet-wasm" refers to a non-existing file`. The
`predev` / `prebuild` hooks cannot help here — they are yarn scripts,
and yarn cannot install yet. Run wasm-pack directly first:

```sh
cd workbench/velvet-wasm && wasm-pack build --target web --out-dir pkg
cd ../.. && yarn install
```

## Development

All commands run from the monorepo `packages/` root:

```sh
# Start the dev server (auto-builds velvet-wasm before starting)
corepack yarn workspace "@sequentech/workbench-app" dev
```

Vite serves on `http://localhost:5173` with HMR. The `predev` hook
compiles **both** wasm packages — `velvet-wasm` and
`sequent-core/pkg` — so Rust edits in either are picked up on the next
dev-server start. Neither is committed; both are build outputs.

TypeScript edits hot-reload. **Rust edits never do**: `predev` is a
one-shot hook and the `sequent-core` alias is resolved when the config
loads, so a Rust change always means rebuild + restart.

By default the booth runs the locally built `sequent-core`, matching
the tally half (which compiles the same crate into `velvet-wasm`). To
reproduce what a *deployed* booth does, opt into the committed tarball
for a run:

```sh
WORKBENCH_SEQUENT_CORE=tgz corepack yarn workspace "@sequentech/workbench-app" dev
```

The active source is shown on the Diagnostics page as *Booth
sequent-core*.

## Working on Rust: tally, encoding, cryptography

**The process is the same for `velvet-core`, `sequent-core` and
`strand`.** Edit the Rust, then:

```sh
corepack yarn workspace "@sequentech/workbench-app" dev
```

`predev` rebuilds both wasm packages and syncs the one Yarn copies, so
there is no manual step. What differs between the three crates is only
*which* artifact carries your change:

| Crate | Reaches the workbench through | Affects |
|---|---|---|
| `velvet-core` | `velvet-wasm` (Cargo path dep) | tally, decode |
| `sequent-core` | **both** `velvet-wasm` *and* `sequent-core/pkg` | decrypt/tally **and** the booth's encrypt, locale, area tree |
| `strand` | both, transitively (velvet-core and sequent-core depend on it) | anything cryptographic |

Because `sequent-core` and `strand` feed both halves, rebuilding only
one would put the booth's encrypt and the workbench's decrypt on
different code — which is why both are rebuilt together.

**A restart is always required.** TypeScript hot-reloads; Rust never
does. `predev` is a one-shot hook and the `sequent-core` alias is
resolved when the Vite config loads, so building under a running server
changes nothing.

### Checking your change actually landed

1. **Diagnostics → Build status** — artifact mtimes against crate
   sources, plus *Booth sequent-core* showing which build the booth is
   running.
2. **The §M.4 canary** in [LIFTING.md](LIFTING.md) — cast Blue on the
   bundled fixture and expect `decodedBigInts === "4"`. It crosses both
   halves, so it only passes if encrypt and decrypt agree.

If results change in a way you did not intend, suspect a stale artifact
before suspecting your code — a behaviour-only change throws no error,
it just returns old numbers.

## Production build

```sh
corepack yarn workspace "@sequentech/workbench-app" build
```

This runs `prebuild` (both wasm builds, via the prepare scripts that also
sync `node_modules/velvet-wasm`) → `tsc -b` (type-check) → `vite build`.
Output lands in `workbench/app/dist/` — fully static HTML/JS/WASM with no
runtime server dependencies.

> **This currently fails at the `tsc -b` step.** The dev server is the
> supported workflow; see [Known gaps](#known-gaps) for the three causes
> and what fixing it would involve.

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
│   │   ├── ContestPolicyOverridesPanel.tsx  Policy-overrides panel
│   │   ├── policyOverridesStore.ts Ephemeral per-tab override store
│   │   ├── BallotPipeline.tsx    Encode→encrypt→decrypt→decode sandbox
│   │   ├── TallyPage.tsx         Tally sandbox
│   │   ├── tally.ts              velvet-wasm tally bridge
│   │   ├── BoothSpike.tsx        Lifted voting-portal screens
│   │   ├── persistence.ts        Snapshot/checkpoint persistence
│   │   ├── import/               Portal / velvet import helpers
│   │   ├── fixtures/snapshots/   Bundled election snapshots (validated at build)
│   │   └── lib/                  Shared utilities
│   └── vite.config.ts            Build plugins & alias resolution
├── velvet-core/         Pure-computation tally crate (wasm32-capable).
│                        NOT workbench-only — `packages/velvet` depends
│                        on it and re-exports it; see Known gaps.
├── velvet-wasm/         wasm-bindgen wrapper exposing velvet-core to JS
├── validation-spec/     THE executable spec — one typed Rust crate, bug-
│                        compatible, its accidental complexity enumerated as
│                        a quirk registry. Production is checked against it
│                        exhaustively by characterization/headless-sweep.mjs
├── validation-adapters/ The injection layer: adapters from production's
│                        Contest / DecodedVoteContest to the rationalized
│                        query-provider, with a native conformance test
│                        (production codec + gates ≡ oracle ∘ adapters,
│                        cargo test -p validation-adapters)
├── docs/                Vote-validation deep dives (VOTE_VALIDATION.md,
│                        VALIDATION_LOGIC_DISTILLATION.md, FIXTURE_VARIANCE.md);
│                        findings (UPSTREAM_FINDINGS.md), reviewer
│                        reproduction recipes (REPRODUCE.md), policy-intent
│                        evidence (INVALID_VOTE_POLICY_INTENT.md). Plus one
│                        temporary file: EVIDENCE_RESTRUCTURE.md, a migration
│                        plan to be deleted when it lands
├── characterization/    The validation apparatus. Every file is exactly one
│                        of three kinds, told apart by whether it touches
│                        production and what a failure means.
│                        EVIDENCE (production ≡ the spec; a failure means
│                        fidelity is broken): headless-sweep.mjs,
│                        dom-validate.mjs, quotient-validate.mjs,
│                        browser-witnesses.mjs, classifier-table.mjs, the
│                        e2e pipeline runners.
│                        ANALYSIS (consumes the certified spec, never touches
│                        production; a failure means a finding stopped being
│                        derived): effect-dependencies.mjs, effect-map.mjs,
│                        no-silent-discount.mjs, gate-count-agreement.mjs.
│                        DOCUMENTATION (cannot fail): rule-tables.mjs, which
│                        renders the seven per-rule tables from the spec.
│                        Shared: domain.mjs (the certified domain, defined
│                        once), cell.mjs, rust-spec.mjs, rule-specs.mjs,
│                        harness.mjs, browser-harness.mjs, booth-cell.mjs.
│                        Commands and outputs:
│                        characterization/README.md ("Running the analysis")
├── WORKBENCH.md         Workbench-side design: inspector, snapshots,
│                        overlay state, Diagnostics, authoring workflow
├── LIFTING.md           Procedure for embedding voting-portal source
└── LIFTING-TALLY.md     Velvet → ui-essentials tally adapter mapping
```

The four root design documents divide as follows, and each says so at its
own top: **README** — what the workbench is, how to run it, and where
drift is tracked. **WORKBENCH.md** — everything workbench-owned that lives
*around* the lifted code. **LIFTING.md** — the voting-portal embedding
procedure and its canaries; wins over WORKBENCH.md on any lift fact.
**LIFTING-TALLY.md** — the velvet-to-ui-essentials adapter. (The
validation work documents itself in `docs/` and
`characterization/README.md`.)

## Embedding strategy

Every non-trivial dependency the workbench pulls in is listed below —
this table is exhaustive, not illustrative. (Ordinary npm packages —
React, MUI, Apollo, Redux, react-router, i18next — are omitted; they are
resolved normally and have no embedding story.)

| Strategy | Dependency | Mechanism |
|----------|-----------|-----------|
| **Shared source** | `velvet-core`, `sequent-core` (Rust side) | Real crate consumed identically by production and workbench, via a Cargo path dep. Breakage is a compile error. |
| **Local wasm build** | `sequent-core` (JS/wasm side) | **Default.** The lifted booth's `import … from "sequent-core"` is aliased to `packages/sequent-core/pkg` — the wasm-pack output of the in-tree crate, gitignored and rebuilt by `predev`/`prebuild`. Matches the tally half, which compiles the same crate from source. (§A7) |
| **Committed tarball** | `sequent-core` (JS/wasm side) — *opt-in* | Only with `WORKBENCH_SEQUENT_CORE=tgz`. Resolves to `node_modules/sequent-core`, unpacked from the `rust/sequent-core-0.1.0.tgz` that `voting-portal` / `ui-core` / `admin-portal` / `ballot-verifier` each commit. That tarball is a snapshot packed from the same `pkg/`, so it is the *artifact production ships* — use it to reproduce deployed behaviour, at the cost of possibly disagreeing with the tally half. |
| **Alias lift** (in-place) | `voting-portal`, `ui-core`, `ui-essentials` | Vite `resolve.alias` points at the original source files in their upstream packages; no files are copied. Substitute providers replace services the portal normally talks to (Keycloak, Hasura, REST). |
| **Upstream component** | Tally results (in `ui-essentials`) | The production tally visualization, imported unmodified. A thin adapter maps velvet's output to its props. Rides the `ui-essentials` alias above, but needs no substitutes. |
| **Workbench-owned** | `velvet-wasm` | The workbench's own `wasm-bindgen` layer over `velvet-core` + `sequent-core`, consumed as `file:../velvet-wasm/pkg`. Not embedded from anywhere — it is the vehicle that gets the Rust into the browser. |

### Where drift actually lives

This table deliberately does **not** rank drift risk per row: scoring how
faithfully workbench and production see the same code *within this branch*
is the wrong axis (for a shared crate it is trivially "none — same
source"). The risk that matters is **this branch versus `main`**, and by
that measure the rows scoring best above are among the worst. `velvet-core` is "shared source" and therefore drift-free
in-branch, yet it is the single largest divergence we carry, because the
extraction that makes it shared has not landed upstream and main's tally
logic keeps moving. Conversely a row can be modified in-branch and pose
almost no catch-up risk, as `sequent-core` does — build-enablement edits
that upstream will accept or that rebase trivially.

So drift is tracked where it can be measured rather than guessed:

- **Live**, per subtree, on the workbench's own **Diagnostics page**
  (`/diagnostics` → *Shared-source drift*). Each tracked tree is
  diffed `HEAD` vs the merge-base with `origin/main`, and carries an
  `expectation` describing what should be there — so an undocumented
  change reads as undocumented. It also reports how many commits
  `origin/main` has that this branch doesn't.
- **Narratively**, in [Known gaps](#known-gaps) below for the divergences
  we have accepted, and in `LIFTING.md` §L for every edit made to
  production source.

**The version-skew trap.** `sequent-core` reaches the workbench through
*two* of these rows at once: the booth's encrypt and `velvet-wasm`'s
decrypt/tally each carry their own build of the same crate. **By default
they cannot skew** — `predev` rebuilds both from the same source on every
dev-server start. The trap arms in two situations: opting into the tgz
(`WORKBENCH_SEQUENT_CORE=tgz` pins the booth to the *committed* artifact
while the tally half still builds from local source), or rebuilding one
half manually under a running server. Skewed halves run different versions
of the encoding rules, and the mismatch surfaces as a wrong `BigUint`
rather than an error. If numbers look wrong, restart the dev server (both
halves rebuild) and re-run the §M.4 canary in [LIFTING.md](LIFTING.md).

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

How current velvet-core's tally semantics are against `origin/main` is
not asserted here — it changes with every upstream merge. Read it live
from the Diagnostics page (`/diagnostics` → *Shared-source drift*, which
reports how many commits `origin/main` is ahead), and treat any upstream
`do_tally` change as a forward-port task per the paragraph above.

**strand carries an unreconciled divergence.** This branch removed the
obsolete openssl/FIPS backends to reach wasm32, and merges have resolved
`packages/strand` in favour of this branch — so any strand changes from
upstream feature PRs have been discarded rather than merged. Unlike
velvet-core, nothing has been forward-ported here.

**The workbench implements only the single-contest (`raw_ballot`)
encryption path, not the compact `multi_ballot` one.** Production's booth
chooses between them per election
(`election_event_presentation.contest_encryption_policy`): the
`MULTIPLE_CONTESTS` policy packs several plurality contests into one
`multi_ballot` payload; otherwise each contest is encrypted separately
(`raw_ballot`). The workbench always takes the per-contest route — its
encrypt bridge wraps a single `[decoded]` contest and its decrypt bridge
loops `contestIds` decrypting one ciphertext each. It handles ballots
with multiple contests, but by encrypting them independently, never
packed. Consequences: (1) the `MULTIPLE_CONTESTS` encoding and its
30-byte capacity limits (FIXTURE_VARIANCE §12) are unexercised end-to-end
here; (2) **decline-to-vote cannot round-trip** — the ballot-level
decline bit lives only in `multi_ballot`, and `raw_ballot::decode`
hardcodes `is_decline_to_vote: false`, so a ballot declined in the booth
would decode as not-declined and tally as blank/invalid rather than
`Declined`. The tally-side classification of `Declined` is still
characterized headlessly
([characterization/classifier-table.md](characterization/classifier-table.md)
feeds a decoded ballot directly); only the booth→wire→tally round-trip is
blocked — the one open cell in the characterization suite. Closing this
means adding a `multi_ballot` encrypt/decrypt path to the workbench —
deferred until the `MULTIPLE_CONTESTS` encoding is worth exercising for
its own sake.

**`yarn build` (`tsc -b`) does not pass.** The dev server is the
supported workflow. Three separate causes: `tsconfig.json` uses
`erasableSyntaxOnly` (TypeScript ≥ 5.8) while the app pins `~5.7.2`; the
deprecated `@types/minimatch` stub trips `TS2688`; and `tsc` does not
read Vite's `resolve.alias`, so it cannot resolve
`@sequentech/ui-core` / `@sequentech/ui-essentials` and ends up
type-checking the lifted portal sources under the workbench's stricter
flags. Fixing it means mirroring the Vite aliases as tsconfig `paths` and
excluding portal sources from the check.

## What's next

Where the work stands (2026-08-21, caught up to `origin/main`
`da683e463b`): the validation characterization is complete, and now runs
on merged code. The catch-up brought main's Blank Ballots work (velvet's
`do_tally` changes forward-ported into velvet-core) and the fifth
`invalid_vote_policy` value `allowed-with-exclusive-explicit`, which is
characterized (≡ `allowed` for every emitted effect; its marker-exclusivity
clear booth-confirmed). Every instrument was re-run green on merged code.
Production is compared against one executable spec
**exhaustively** on the headless domain — 345,600 cells, plurality and
preferential, zero disagreements — and **per cell** in the real booth
(229/229), with the browser-side independence claims discharged by
sufficiency (195,520 cells via 2,616 quotient classes)
([characterization/README.md](characterization/README.md)). The findings
are no longer things somebody noticed: each is a **property derived from
the certified spec**, with an acceptance test so the derivation cannot
lose it, and the serious ones are additionally confirmed booth-to-tally
through real crypto.

**The evidence restructure has landed.** There is now ONE executable
spec — the typed Rust crate [validation-spec/](validation-spec/),
bug-compatible with its accidental complexity enumerated as a quirk
registry — and every runner that compares against production targets it
directly. The apparatus divides into an **evidence layer** (production ≡
the spec) and an **analysis layer** (consuming the certified spec,
never touching production), with the per-rule tables as documentation
rendered from the spec. `spec.mjs`, `rust-conformance` and the seven
per-rule runners are gone
([docs/EVIDENCE_RESTRUCTURE.md](docs/EVIDENCE_RESTRUCTURE.md)).

One booth flow remains unreachable (decline-to-vote), and the
pipeline's coverage carries explicitly labelled residues — the unswept
region (decline, `max_votes = 0`), 4,384 quotient classes with no
booth-formable member — mostly preferential, awaiting a generic IRV booth
recipe — and 23 witnesses no bundled fixture can observe; each named with
its unblocking condition in its own artifact. What would close those residues is suite-internal work, and it
is collected in [characterization/README.md](characterization/README.md)
("Open work"); what follows here is what needs a decision or changes
which artifact ships, roughly by payoff:

1. **Consultation on the findings.** S1 (silent discounting) and S5
   (null-vote choice preservation) are documented — S1/S2/S3 also carry
   autonomous fix decisions in the rationalized reference, adjudicated
   at the upstream pull-request review (docs/UPSTREAM_FINDINGS.md) —
   ([docs/UPSTREAM_FINDINGS.md](docs/UPSTREAM_FINDINGS.md)), reproducible
   click-by-click ([docs/REPRODUCE.md](docs/REPRODUCE.md)) and in one
   command (`node characterization/reproduce-verify.mjs`), with
   policy-intent evidence assembled
   ([docs/INVALID_VOTE_POLICY_INTENT.md](docs/INVALID_VOTE_POLICY_INTENT.md)).
   Per the three-state model (characterized → suspect → adjudicated),
   escalating them to the parties with design authority is the step this
   repo cannot take by itself. meta#8235 has now been read (2026-08-14;
   evidence folded into that document's §5): it asks for *more* voter
   signal, never suppression — the residual intent questions go to the
   fix's authors. **One candidate is not yet filed as a suspect**: an
   unset `invalid_vote_policy` resolves to `allowed` in the Rust spec,
   but the TypeScript filter reads the policy raw (`?? undefined`), so
   "unset" and "allowed" are not the same configuration at the booth.
   Whether that becomes a suspect is a joint call — adjudication is
   nobody's alone.
2. **Consultation on S6** — the newest suspect, filed 2026-08-19. On a
   ranked ballot the submission gates count only first preferences where
   the checker counts every ranked selection, so the gates decide from a
   number unrelated to the ballot. It does not miscount any vote; it
   corrupts what the voter is told at casting time, in both directions —
   a dialog on a ballot the checker is content with, and no dialog where
   the policy promises one. Exhaustive over 345,600 certified cells,
   confirmed in a real booth, with git provenance showing the count
   predates preferential contests by fifteen months and was not revisited
   when they arrived ([docs/UPSTREAM_FINDINGS.md](docs/UPSTREAM_FINDINGS.md)
   S6; derived by `characterization/gate-count-agreement.mjs`).
3. **The decline-to-vote booth flow** — the one open characterization
   cell. Blocked on adding a `multi_ballot` encrypt/decrypt path (the
   decline bit does not exist in `raw_ballot`; see Known gaps above), so
   it is a feature lift, not a rule extension.
4. **Distillation step 5 — the rationalized implementation: phases 1–3
   are done.** [validation-spec/](validation-spec/) carries it alongside
   the frozen oracle: the query-provider (one derivation of the
   vote-state facts, every production site answered as a projection —
   the six-place duplication collapsed), forked from the oracle and
   measured against it by `characterization/fix-diff.md` — 86,304 of
   345,600 cells differ, every one attributed to a named fix, zero
   unexplained. The fix ledger (the `disposition` on each `quirks()`
   entry) is settled: **fixed by construction** S6, S4 (both halves),
   D3 (latent); **fixed by judgment** S1 — the inline mute removed,
   restoring "informed but uninterrupted", which makes silent
   discounting *unrepresentable* in `f_fixed` (asserted per cell in
   fix-diff, 0 of 345,600 vs the oracle's 6,336); **fixed by judgment**
   also S2 and S3 (2026-08-28: explicit blank votes are not subject to
   the min-vote rule — the marker-only ballot reports `ExplicitBlank`
   at every `min_votes` instead of `ImplicitInvalid` at min ≥ 2; 4,800
   cells, the fix-diff S2S3 bucket); **kept** S5 (intentional per
   #2949). Every fix-ledger judgment is made **autonomously**: the
   upstream pull-request review is where it is finally adjudicated, the
   decision stands until overturned there, and the workbench acts on it
   again only reactively — if the review rejects it. The ledger is
   complete, and the **production-injection branch**
   (`feat/workbench-rationalized-injection/main`) carries its first
   deliverable: [validation-adapters/](validation-adapters/) — the
   production-typed layer (`for_contest(&Contest)` /
   `for_vote(&DecodedVoteContest)` / `for_ballot(…)`), kept out of the
   spec crate to preserve its independence, with a native conformance
   test proving production's own codec and gate functions ≡ the frozen
   oracle through the adapters across the seven grids' matrices (~310
   cells, `cargo test -p validation-adapters`). **The first call site is
   injected**: `sequent-core`'s submission gates
   (`util/voting_screen.rs`) now route through the query-provider via
   `sequent_core::validation_provider` (the derivations moved into
   production; `validation-adapters` delegates to them), keeping only a
   record-driven path for the encoding-error class the vote state
   cannot express. Verified exhaustively with per-component
   expectations: the sweep compares production's gates/dialog against
   `f_fixed` and everything uninjected against the oracle — 345,600
   cells, 0 disagreements — the native conformance run agrees (6/6),
   and the browser lane is re-run against the injected booth on the
   same split (`dom-validate` 233/233 — exactly the three S2S3
   marker-only min ≥ 2 cells moved, the not-allowed one now reaching
   review; `browser-witnesses` and `quotient-validate` decide
   review-reachability by the injected gate). What remains: the decode
   site (emissions — carries the S4-checker and S2S3 tally changes)
   and eventually the TypeScript filter (S1's display fix).

Standing maintenance, as upstream moves:

- **Land the velvet-core extraction upstream** — what would stop the
  recurring `do_tally` forward-ports (Known gaps above); until then,
  budget the port on each catch-up merge and reconcile the strand
  divergence deliberately rather than by merge default.
- **The fifth `invalid_vote_policy` value is characterized** (done
  2026-08-21, when `allowed-with-exclusive-explicit` reached `main`): it
  is ≡ `allowed` for every emitted effect (certified by the sweep) and
  its marker-exclusivity clear is booth-confirmed
  (INVALID_VOTE_POLICY_INTENT.md §8; UPSTREAM_FINDINGS.md S1/S5).
- **After any portal refresh**, re-run the characterization suite
  (headless runners + `dom-validate.mjs`) and the consumer census —
  LIFTING.md's refresh runbook ends with this step.
- **Fix `yarn build`** (three known causes above) if a static production
  build is ever needed rather than the dev server.
- **Delete `docs/EVIDENCE_RESTRUCTURE.md`** — the migration it plans has
  landed and is marked complete in the document itself. It is a plan, not
  a record; the commit history is the record.

## License

AGPL-3.0-only
