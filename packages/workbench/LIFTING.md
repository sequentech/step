<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Lifting recipe: embedding voting-portal into the workbench

## Why this document exists

Unlike `velvet-core` — which the workbench consumes as a **shared crate**, so
production and workbench see the same Rust source automatically — the
voting-portal is **lifted**, not reused. We re-host its TypeScript source
files behind a different build harness (Vite, not craco) and supply
substitute providers for the services it normally talks to (Keycloak,
Hasura, REST APIs).

That means fidelity is *manual*. It only holds for as long as our glue keeps
mirroring what production does. When voting-portal evolves, our embedding
will silently drift unless we have an explicit, reproducible procedure for
re-applying our adaptations.

**This file is that procedure.** It documents every adaptation we made, in
order, with the rationale and the canary symptom that signals "voting-portal
changed in a way that breaks this step." A refresh is: re-run the recipe
top to bottom.

The procedure is also intentionally **conservative about portal source
edits**. The rule is:

> **Never modify files under `voting-portal/src/`** to make the workbench
> work. Every adaptation lives inside `packages/workbench/app/` (or in the
> Vite config, or in `public/global-settings.json`). If a step seems to
> require editing portal source, stop and reconsider — there is almost
> always a provider/alias/define alternative.

When that rule has to be relaxed (e.g. a `process.env.REACT_APP_*` reference
that Vite can't see), the workaround belongs in `vite.config.ts` (via
`define:` or a plugin), not in the portal source itself.

---

## Inventory of adaptations

Each adaptation has:

- **What** — the change.
- **Where** — the file(s) in `packages/workbench/app/`.
- **Why** — what production behaviour it substitutes for.
- **Canary** — the symptom that would prove voting-portal evolved past it.

### A. Bundler configuration (`vite.config.ts`)

| # | Adaptation | Why | Canary if portal changes |
|---|------------|-----|--------------------------|
| A1 | `@vitejs/plugin-react` | voting-portal source is `.tsx`. | Build fails with "Unexpected token <". |
| A2 | `vite-plugin-wasm` + `vite-plugin-top-level-await` | The workbench's own `velvet-wasm` module uses top-level await; unrelated to portal but required for the dev server to boot. | n/a (workbench-only). |
| A3 | `resolve.alias` `@sequentech/ui-core` → `<repo>/packages/ui-core/src/index.tsx` | Both ui-core and ui-essentials declare `"main": "dist/index.js"` but `dist/` is never built in the dev workflow. We point the alias at the TS sources so Vite compiles them on the fly. | Build fails with `Failed to resolve import "@sequentech/ui-core"`. Re-check `ui-core/package.json` — if `main` now points at a built artifact that exists, the alias may become unnecessary. |
| A4 | `resolve.alias` `@sequentech/ui-essentials` → `<repo>/packages/ui-essentials/src/index.tsx` | Same as A3 for ui-essentials. | Same as A3. |
| A5 | `resolve.alias` regex `^@root/(.*)$` → `<repo>/packages/ui-core/src/$1` | ui-core internally imports from `@root/...`, which is a tsconfig `paths` alias resolving to `ui-core/src/*`. Vite does not read tsconfig paths. (`ui-essentials` does not use `@root` — verified by grep when this alias was added.) | Build fails with `Failed to resolve import "@root/..."`. If voting-portal or ui-essentials starts using `@root` too with different semantics, the regex needs widening or splitting. |
| A6 | `optimizeDeps.exclude: ["velvet-wasm"]` | Vite's dep optimizer chokes on the `.wasm` URL import. | n/a (workbench-only). |

### B. Workspace dependency graph (`app/package.json`)

The workbench app needs `voting-portal` as a workspace dep (`"*"`), plus
every npm dep the lifted portal source files transitively `import`. Adding
these as direct deps of the workbench is the price of doing a source lift:
they have to be on the workbench's `node_modules` resolution path.

Current direct deps required *because* of the lift (in addition to what the
workbench itself needs):

- `@apollo/client`, `graphql` — voting-portal uses Apollo for GraphQL.
- `@emotion/react`, `@emotion/styled` — MUI's required peer.
- `@mui/material` — UI framework.
- `@reduxjs/toolkit`, `react-redux` — state.
- `@sequentech/ui-core`, `@sequentech/ui-essentials` — workspace deps.
- `i18next`, `i18next-browser-languagedetector`, `react-i18next` — translations.
- `react-router`, `react-router-dom` — routing.
- `voting-portal` — the lifted package itself, on path `"*"`.

**Canary:** if a new `import` line appears in any portal file the workbench
touches, the dev server will fail with `Failed to resolve import "X"`. Fix
is to add `X` to `app/package.json` deps.

**Tip when refreshing:** rather than wait for runtime errors, run

```
grep -rh "^import .* from " ../../../voting-portal/src/ | \
  awk -F'"' '{print $2}' | sort -u
```

from `packages/workbench/app/` and diff the result against the dep list.
Anything new is a candidate addition.

### C. Runtime configuration files

#### `app/public/global-settings.json` — substitute for `/global-settings.json`

Voting-portal's `SettingsContextProvider` fetches `/global-settings.json` at
startup. We ship a static one with:

- `DISABLE_AUTH: true` — short-circuits `KeycloakProvider` in
  `voting-portal/src/index.tsx` to render its children directly, bypassing
  all real auth. **This is the single most important toggle for the lift**:
  without it, the embedding would need a full Keycloak mock.
- Service URLs (`KEYCLOAK_URL`, `HASURA_URL`) pointing at
  `http://127.0.0.1:0/` so any code path that escapes our mocks fails fast
  rather than hitting real infrastructure.
- Sensible defaults for the remaining `SettingsContext` fields.

**Canary if portal changes:** new required keys in `SettingsContext` will
either crash at read time (`undefined.foo`) or behave in an unexpected
default. Inspect `voting-portal/src/contexts/SettingsContext` and add the
new keys.

#### `app/public/locales/*` — optional translation files

i18next will warn on missing keys but the page still renders. Not currently
mocked; add files here only if a screen's UI becomes unreadable.

### D. Provider stack (`app/src/BoothSpike.tsx`)

Every voting-portal screen relies on a chain of React Context providers.
Production wires them in `voting-portal/src/index.tsx`. The workbench wires
the *minimum* subset needed for the screen under test, in the same order.

Currently mounted for `StartScreen`:

1. `<ThemeProvider>` from `@mui/material`, with `theme` from
   `@sequentech/ui-essentials`. **Required** — MUI components throw without it.
2. `<ReduxProvider store={...}>` reusing the **production `store`** from
   `voting-portal/src/store/store`. This is deliberate: same reducers,
   same selectors, same shape. Diverging would defeat the lift.
3. `<Routes>` (inheriting the top-level `<BrowserRouter>` from `main.tsx`).
   StartScreen reads URL params via `useParams`, so it must be mounted
   under a `<Route path="tenant/:tenantId/event/:eventId/election/:electionId/start">`.

**Providers _not_ yet mounted** (and the screens that will demand them):

- `<ApolloProvider>` — needed once a screen issues a `useQuery`. Plan:
  use `@apollo/client/testing`'s `MockedProvider` with hand-rolled
  responses, *not* a live client.
- `<SettingsContextProvider>` — needed if a screen reads
  `useContext(SettingsContext)` (currently the global-settings.json fetch
  happens at the top of `voting-portal/src/index.tsx`, which we don't
  mount). Plan: lift this provider on its own.
- `<WasmContextProvider>` (or whatever the portal calls it) — needed once
  a screen does client-side crypto. Plan: identify the provider, mount it.
- `<KeycloakProvider>` is **never** mounted; `DISABLE_AUTH: true` keeps it
  out of the tree.

**Canary if portal changes:**

- New "must be used within a XProvider" error → mount XProvider (locate it
  in `voting-portal/src/index.tsx` to keep order consistent).
- New top-level provider added in `voting-portal/src/index.tsx` → decide
  whether the workbench needs it; if yes, add to `BoothSpike.tsx`.

### E. Routing (`app/src/main.tsx`)

The workbench has a single `<BrowserRouter>` at the root. The booth spike
mounts under `path="/booth/*"` and delegates to `<BoothSpike>`'s own
`<Routes>`. Production voting-portal uses a single `<BrowserRouter>` too —
we don't nest a second `<MemoryRouter>` because react-router-dom forbids
it (the spike originally tried and got a `<Router> inside another <Router>`
error; this is documented here so we don't repeat the mistake).

**Canary:** if portal-side route paths change, the workbench `<Route path>`
strings must follow. Check `voting-portal/src/routes/` for `<Route>`
declarations.

### F. Redux store fixtures (`app/src/fixtures/`)

Voting-portal screens read their data from a Redux store populated by GraphQL
subscriptions and Apollo cache writes. The workbench has neither. Instead,
each lifted screen gets a fixture module that calls the portal's **own**
action creators (`setElection`, `setElectionEvent`, `setBallotStyle`, ...)
to populate the **production store** with synthetic data. Same store
instance, same reducers, same selectors — only the data source differs.

Currently seeded (`app/src/fixtures/boothFixtures.ts`):

- `setElection({ id, election_event_id, tenant_id, contests, ... })` — a
  minimal election with one contest ("Favourite colour") and two
  candidates. Satisfies `selectElectionById` and `selectFirstBallotStyle`
  consumers downstream.
- `setElectionEvent({ id, name, ... })` — the parent event.

**Convention:** import action creators and slice types directly from
`voting-portal/src/store/*Slice` (NOT from a re-export under the workbench).
If the slice's `PayloadAction<T>` shape changes, TypeScript will fail the
workbench build at the fixture site — that's the early-warning signal we
want.

**Canary if portal changes:**

- New required field on `IElectionExtended` or `IElectionEvent` → TS
  error in the fixture file, telling you exactly which field to add.
- New slice altogether (e.g. `votersSlice`) that StartScreen starts
  consuming → screen returns `null` / spinner / navigates away. Add a
  `seedXyz` call to `seedBoothFixtures()`.
- New action creator API (e.g. `setElection` becoming
  `setElection({election, source})`) → TS error at the dispatch.

### G. Source code under `voting-portal/src/` — UNCHANGED

This is the central invariant of the lift. Every adaptation above lives
*outside* the portal source tree. If you ever feel tempted to edit a portal
file to make the workbench work:

1. **Stop.**
2. Re-read the "Why this document exists" section.
3. Try one of: `resolve.alias`, `define`, a new provider, a fixture seed,
   a substitute deep-import path. One of these almost always works.
4. If you genuinely cannot avoid a portal-source change, document it here
   under a new section "H. Concessions" with the exact diff and the reason
   it was unavoidable. Reviews of refresh PRs will then verify that the
   concession is still needed.

---

## Refresh procedure (when voting-portal evolves)

Run when voting-portal has changed and the workbench booth view is
broken or you want to validate fidelity.

1. **Smoke run.** `corepack yarn workspace "@sequentech/workbench-app" dev`
   and visit `http://localhost:5173/booth`. Check the browser console.
2. **Categorize the first error** using the canary table below:

   | Error pattern | Likely category | Section to revisit |
   |---------------|-----------------|--------------------|
   | `Failed to resolve import "<name>"` | New transitive dep | B (package.json) |
   | `Failed to resolve import "@root/..."` or `"@sequentech/..."` | Workspace path | A (vite.config) |
   | `<X> must be used within a <Y>Provider` | New required provider | D (BoothSpike providers) |
   | `Cannot read property of undefined` reading a settings field | New settings key | C (global-settings.json) |
   | `<Router> inside another <Router>` | Router nesting | E (routing) |
   | Screen renders a spinner / self-redirects | Missing or mis-shaped fixture | F (fixtures) |
   | TS error in `fixtures/*` about a slice payload field | Portal slice type evolved | F (fixtures) |
   | `process is not defined` or `process.env.X` undefined | env var | A (`define` in vite.config) |

3. **Fix the smallest possible thing**, restart the dev server, and re-test.
4. **Update this document.** If you added/changed an adaptation, edit the
   relevant section so the next refresh starts from accurate state.
5. **Run the workbench tally page too** (`http://localhost:5173/`). It uses
   `velvet-core` and is unaffected by portal changes; if it breaks, the
   problem is in workbench glue or wasm-pack output, not the lift.

## Adaptations to add as we lift more screens

When extending past `StartScreen`, the following are the most likely
next-step categories of work (in roughly the order they will be needed):

1. **Mounting `<ApolloProvider>` with a `MockedProvider`.** Define mocks
   keyed by the gql documents the portal already imports — don't redefine
   the gql.
2. **Adding `<WasmContextProvider>`** once a crypto-using screen is in
   scope. Reuse the portal's own provider; only the wasm module identity
   may need swapping (we may want it to point at the workbench's own
   `velvet-wasm` so encode/tally use the same code).
3. **Translation key fidelity.** Once a screen renders, missing
   translations show as raw keys (`booth.start.title`). Decide between
   shipping a copy of the portal's locales as a static asset and silencing
   the warnings.
4. **Apollo mocks vs. a fake transport.** If many screens are lifted at
   once, a shared in-memory schema (with `@graphql-tools/mock`) may scale
   better than per-test mocks. Decision deferred until pain is felt.
5. **Extending the fixture.** As later screens consume more of the store
   (ballot styles, voter info, encryption keys), `boothFixtures.ts` grows.
   Keep it a single module per screen group rather than one fixture file
   per slice — easier to keep coherent across slice boundaries.

Each of these adaptations, when added, should get its own row in the
inventory above with a canary entry. Treat the document as living.

---

## Anti-patterns to avoid

- **Re-implementing a portal component "just to keep things simple".**
  This defeats the goal of testing real code. If a component is hard to
  embed, the right answer is a better provider, not a fork.
- **Copying portal source into the workbench tree.** Same reasoning;
  copies don't auto-refresh when portal evolves.
- **Wide `resolve.alias` patterns** (e.g. aliasing `@/...` to a workbench
  path). They mask drift. Aliases should be either workspace paths or the
  specific tsconfig paths the portal itself defines.
- **Editing `voting-portal/src/`** to silence a workbench error. See
  section F.
- **Using `Stop-Process` on the Vite port mid-debug.** It races with the
  dev server's own startup and gives misleading "port in use" / exit-code
  noise. If port 5173 is busy, find and stop the specific old node
  process, or just change the port for one run.
