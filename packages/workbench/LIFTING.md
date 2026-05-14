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
| A6 | `optimizeDeps.exclude: ["velvet-wasm", "sequent-core"]` | Both packages compute their `.wasm` URL with `new URL("..._bg.wasm", import.meta.url)`. Vite's dep optimizer rewrites `import.meta.url` to a path under `.vite/deps/`, where the wasm binary is missing — the dev server then SPA-falls back to `index.html`, and the wasm loader fails with *expected magic word 00 61 73 6d, found 3c 21 2d 2d* (the bytes of `<!--`). Excluding keeps the original module path. | If voting-portal swaps `sequent-core` for a different wasm package, add it to the exclude list. |

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

Currently mounted:

**`Shell`** (in `app/src/main.tsx`, wraps every workbench page including
the workbench-native `/tally` view), outermost first:

1. `<ReduxProvider store={...}>` reusing the **production `store`** from
   `voting-portal/src/store/store`. This is deliberate: same reducers,
   same selectors, same shape. Diverging would defeat the lift. Hoisted
   to `Shell` so the workbench's own pages (e.g. `/tally`) read the same
   store the booth writes to. Production also wraps Redux outside its
   routes in `voting-portal/src/index.tsx`, so the layering matches.

**`BoothLayout`** (mounted under `Shell` for booth routes only),
outermost first:

1. `<ThemeProvider>` from `@mui/material`, with `theme` from
   `@sequentech/ui-essentials`. **Required** — MUI components throw without it.
2. `<SettingsWrapper>` from `voting-portal/src/providers/SettingsContextProvider`.
   Lifted as-is. It fetches `/global-settings.json` (served from
   `app/public/`) and gates children behind a `<Loader />` until the
   fetch resolves. **Required** because the SettingsContext **default**
   ships with `DISABLE_AUTH: false` and `HASURA_URL: "http://localhost:8080/v1/graphql"`,
   so any screen that reads `globalSettings` before settings load would
   see production-pointing defaults and (for ReviewScreen) fire the
   `GET_ELECTIONS` query against a real Hasura URL.
3. `<ApolloProvider client={apolloClient}>` from `@apollo/client/react`,
   with a workbench-local `ApolloClient` whose link is `ApolloLink.empty()`
   (the observable completes with no data). **Required** by ReviewScreen
   (it calls `useMutation(INSERT_CAST_VOTE)` and `useQuery(GET_ELECTIONS)`)
   even though under `DISABLE_AUTH: true` neither operation actually
   executes against a server: the `useQuery` is skipped, and ReviewScreen
   takes the `useAddFakeCastVote` branch (line 510, gated on
   `isDemo || globalSettings.DISABLE_AUTH`) which mutates Redux directly
   and never calls `tryInsertCastVote`. The `ApolloProvider` is still
   mandatory because `useMutation` runs at component render time, before
   any branch can short-circuit it, and throws the famous
   *"Could not find 'client' in the context"* invariant if no provider
   is in scope. The empty link is the smallest thing that satisfies the
   invariant. We deliberately do **not** use `MockedProvider` from
   `@apollo/client/testing`: it requires pre-declared mocks for every
   operation and throws on misses, which is the wrong default for an
   exploratory workbench. When a screen lifted later needs real GraphQL
   results, swap the empty link for a small pattern-matching `ApolloLink`
   under `fixtures/gql/`.
4. `<WasmWrapper>` from `voting-portal/src/providers/WasmWrapper`. Lifted
   as-is from the portal: it mounts `<WasmContextProvider>` (from ui-core)
   and gates children behind a `<Loader />` until `initCore()` resolves.
   Required by every screen that calls into ui-core wasm helpers (e.g.
   `check_voting_not_allowed_next_bool` in VotingScreen, all crypto in
   ReviewScreen).
5. `<Outlet />` from react-router for the data-router child routes.

**Providers _not_ yet mounted** (and the screens that will demand them):

- `<AuthContextProvider>` is **not** mounted; `useContext(AuthContext)`
  returns the module-level `defaultAuthContextValues` (with `logout: () => {}`,
  `isAuthenticated: false`, `hasRole: () => false`, ...), which is enough
  for the screens lifted so far. If a screen starts requiring authenticated
  state, mount the real provider with a fixture that satisfies it.
- `<KeycloakProvider>` is **never** mounted; `DISABLE_AUTH: true` keeps it
  out of the tree.

**Canary if portal changes:**

- New "must be used within a XProvider" error → mount XProvider (locate it
  in `voting-portal/src/index.tsx` to keep order consistent).
- New top-level provider added in `voting-portal/src/index.tsx` → decide
  whether the workbench needs it; if yes, add to `BoothSpike.tsx`.

### E. Routing (`app/src/main.tsx` + route mirroring)

Two structural decisions follow from inspecting `voting-portal/src/index.tsx`:

1. **Use `createBrowserRouter` + `RouterProvider`, not `<BrowserRouter>`**.
   The portal uses a v6 "data router" with route-level `action` handlers
   (e.g. `votingAction` from `VotingScreen`, `castBallotAction` from
   `ReviewScreen`). Screen components call `useSubmit`, `useActionData`,
   `useNavigation` — these only function under a data router; using the
   legacy `<BrowserRouter>` produces *"useSubmit must be used within a
   data router"* the moment the screen mounts.
2. **Mirror the portal's exact paths under `/`, not under a `/booth` prefix**.
   The portal builds links as absolute strings:
   `/tenant/${tenantId}/event/${eventId}/election/${electionId}/vote`. A
   `/booth/...` prefix would 404 on every internal `<Link>`. So the
   workbench root path tree mirrors the portal, and the only workbench-
   specific route is `/tally`.

Concretely, `main.tsx`:

- Builds a `createBrowserRouter` with a `<Shell>` layout containing the
  workbench nav bar, the global `<ReduxProvider>`, and an `<Outlet />`.
- **Index route**: `/ → <WorkbenchHome />`. Workbench-native landing
  page that lists tenants by introspecting `state.electionEvent` and
  `state.elections`. This is the entry point — not the booth.
- **Drilldown routes** under `/wb/...` (workbench-native, all in
  `app/src/Workbench.tsx`):
  - `/wb/tenant/:tenantId` — list of events for the tenant.
  - `/wb/tenant/:tenantId/event/:eventId` — list of elections in the event.
  - `/wb/tenant/:tenantId/event/:eventId/election/:electionId` —
    election detail with metadata, cast-vote table (honestly surfacing
    both the `election_id` and `event_id` cast-vote bins because the
    demo path conflates them), ballot-style summary, and CTAs into the
    booth at the production-mirroring paths.
- **Raw-JSON tally sandbox**: `/tally → <App />`. Kept as a focused
  velvet-wasm playground; no Redux integration so it can run
  independently of the scenario state.
- **Booth subtree**: `<BoothLayout />` mounting `boothChildren` (defined
  in `BoothSpike.tsx`). `boothChildren` mirrors the portal's
  `tenant/:tenantId/event/:eventId/{election-chooser, election/:electionId/*}`
  subtree and pairs each route with the same `action` the portal wires.

The split is deliberate: `/wb/...` is workbench-owned chrome we are free
to evolve; `/tenant/:t/event/:e/...` is the production-mirror surface
where we MUST NOT diverge. Internal portal `<Link to="/tenant/...">`
calls keep resolving at the production paths because we never moved
them.

`BoothSpike.tsx` exports two things only:

- `BoothLayout` — the booth-screen-only providers (Theme, Settings,
  Apollo, Wasm). The ReduxProvider lives in `Shell` instead (see
  section H), so `BoothLayout` does NOT mount Redux itself.
- `boothChildren: RouteObject[]` — the route data with elements and actions.
  The shape mirrors the portal's own `tenant/:tenantId/event/:eventId`
  subtree: `election-chooser` and `election/:electionId/*` are siblings
  under a common parent (NOT cousins). Keeping that parent-child
  structure intact is what lets the chooser's absolute-path navigation
  (`navigate(\`/tenant/.../election/${id}/start\`)`) resolve at the
  same URLs the portal produces in production.

**Workbench-native pages reach the booth via production paths.** The
election detail page renders a "Start voting for this election" CTA
that links to
`/tenant/:t/event/:e/election/:el/start`, not to a `/wb/...` path. That
keeps a single source of truth for "what URL the booth is at": the
portal's own absolute-`<Link>` strings.

**Canary if portal changes:**

- Portal adds a new screen under `election/:electionId/<new-path>` →
  workbench navigates to it and shows the `*` fallback. Add a route to
  `boothChildren`. Always import the screen **and** any exported `action`
  from the same module.
- Portal renames `votingAction` / `castBallotAction` exports → TS error in
  `BoothSpike.tsx` at the import.
- Portal restructures route paths (e.g. removes `tenant/event` parents) →
  links break with *"No routes matched location"*. Re-mirror the new tree
  in `boothChildren`.
- Portal switches off the data router (unlikely) → revert to
  `<BrowserRouter>`.

### F. Redux store fixtures (`app/src/fixtures/`)

Voting-portal screens read their data from a Redux store populated by GraphQL
subscriptions and Apollo cache writes. The workbench has neither. Instead,
each lifted screen gets a fixture module that calls the portal's **own**
action creators (`setElection`, `setElectionEvent`, `setBallotStyle`, ...)
to populate the **production store** with synthetic data. Same store
instance, same reducers, same selectors — only the data source differs.

Currently seeded (`app/src/fixtures/boothFixtures.ts`):

- `setElection({ id, election_event_id, tenant_id, contests, num_allowed_revotes: 0, ... })` —
  a minimal election with one contest ("Favourite colour") and two
  candidates. `num_allowed_revotes: 0` is interpreted by
  `canVoteSomeElection` as *unlimited revotes*, which keeps the selector
  truthy without seeding a working `castVotes` feed.
- `setElectionEvent({ id, name, ... })` — the parent event.
- `setBallotStyle({ id, election_id, ballot_eml: { contests, public_key, ... }, ... })`
  — a ballot style whose `ballot_eml.contests` is the same `IContest`
  array as the election's. The `public_key` is
  `"ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4"` — the exact
  `DEFAULT_PUBLIC_KEY_RISTRETTO_STR` constant that
  `packages/sequent-core/src/encrypt.rs` ships and uses in its own
  fixtures. It is a real point on the Ristretto curve, so
  `encrypt_decoded_contest` succeeds end-to-end (the workbench validates
  the *encrypt* path; the matching private key is intentionally not
  bundled). Marked `is_demo: false` so StartScreen does not pop the
  "this is a demo" dialog.
- `resetBallotSelection({ ballotStyle, force: true })` — initializes the
  per-election `ballotSelections[electionId]` entry with all candidates
  at `selected: -1`. **Critical**: in production this dispatch happens
  inside `StartScreen` when the voter clicks *Start Voting*. The
  `setBallotSelectionVoteChoice` reducer is a silent no-op
  (`if (!currentElection) return state`) until that initialization has
  happened, so a user clicking a candidate on `/vote` after a hot reload
  would see the visual highlight flicker but the redux state never
  update, and the *Next* button's `encryptAndReview` would early-return
  because `selectionState` is `undefined`. The workbench needs every URL
  to be a valid entry point (hot reload on `/vote`, deep links), so we
  pre-seed the empty selection structure.
- **`election.status` and `electionEvent.status` set to `voting_status:
  OPEN`** (with `kiosk` / `early` set to `CLOSED`). Required for
  `ElectionSelectionScreen`'s `ElectionWrapper`, which calls
  `isVotingOpen()` during render (`<SelectElection isOpen={isVotingOpen()}>`).
  For non-kiosk voters that resolves to
  `(online OPEN && eventOnline OPEN) || (earlyOn && eventEarly OPEN)`;
  with both online statuses OPEN the first conjunct short-circuits
  before the early-voting branch — which would otherwise dereference
  `ballot_eml.area_presentation.allow_early_voting` and crash if absent.
- **`ballot_eml.area_presentation: { allow_early_voting: NO_EARLY_VOTING }`**.
  Belt-and-braces with the previous point: even if a future change
  makes the early-voting branch reachable, the fixture won't crash —
  it will just report "no early voting policy enabled" and fall back
  to the online-OPEN path.

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

### G. Workbench-only debug affordances (`BoothSpike.tsx`)

To debug whether a click reaches a reducer, the workbench exposes the
production Redux store on `window.__store` and patches `store.dispatch`
to push every action into `window.__dispatchLog`. From the browser
console or a Playwright `page.evaluate`:

```js
window.__store.getState().ballotSelections
window.__dispatchLog.map(x => x.type)
```

This is **workbench-only** — it lives in `BoothSpike.tsx` and does not
touch portal source. It was instrumental in diagnosing the
"`setBallotSelectionVoteChoice` dispatched but state empty" issue that
led to seeding `resetBallotSelection` from the fixture (section F). Keep
the affordance: every future "click does nothing visible" bug should
start with `__dispatchLog`.

**Canary if portal changes:** if the portal exports `store` from a
different path or moves to a non-Redux state container, both the import
and the patch need updating. The patch is bypass-safe (it forwards to
the original dispatch), so if it ever stops working, just verify the
order of operations in `BoothSpike.tsx` puts the patch BEFORE the first
component render.

### H. Persistence + the auto-resume snapshot (`app/src/persistence.ts`)

The workbench mirrors the entire voting-portal Redux state to
`localStorage` on every dispatch and rehydrates from it on boot. The
result: cast a ballot, close the tab, reopen tomorrow — the ballot is
still cast.

This is the foundational layer the user-facing "save / load named
checkpoint" UI sits on top of. There are three storage tiers, all
sharing the same JSON shape (`PersistedSnapshot = { version: "v1",
state: RootState }`):

| Tier | Trigger | Storage key | Lifetime | Mutability |
|---|---|---|---|---|
| Auto-resume slot | Every Redux dispatch | `localStorage["workbench:state:v1"]` | Until reset / wiped | Constantly overwritten |
| Named checkpoint | Operator clicks "Save current state" on `/` | `localStorage["workbench:checkpoint:v1:<name>"]` (plus index at `workbench:checkpoints:v1`) | Until deleted | Frozen at save time |
| Bundled fixture snapshot *(future)* | Author wrote it | `app/src/fixtures/snapshots/*.json` | Forever (in git) | Read-only at runtime |

Named checkpoints reuse the exact same load/save plumbing as the
auto-resume slot (`hydrateFromSnapshot`, `PersistedSnapshot`); only the
storage key and the index differ. Bundled snapshots will plug into the
same `hydrateFromSnapshot` entry point when implemented.

**Named-checkpoint semantics:**

- `saveCheckpoint(store, name)` writes `store.getState()` under
  `workbench:checkpoint:v1:<name>` and adds/refreshes the entry in the
  sorted index. Names are normalized to letters/digits/`._- ` with a
  64-char cap (`normalizeCheckpointName`); illegal input throws so the
  UI can surface a precise message.
- `loadCheckpoint(store, name)` dispatches the snapshot through the
  same `hydrateFromSnapshot` used at boot, which means the auto-resume
  slot gets overwritten as a side-effect. The UI follows up with a
  `location.reload()` to drop any in-memory derived state (Apollo
  cache, mounted screens' local `useState`) so the boot path replays
  hydration cleanly.
- `deleteCheckpoint(name)` removes both the snapshot key and its
  entry in the index.
- Saving does NOT pause the auto-resume slot. The two tiers are
  independent: saving a checkpoint is purely additive; the auto-resume
  slot keeps tracking every dispatch.

**How rehydration works.** `hydrateFromSnapshot(store, snapshot)`
dispatches the portal's own `setX` action creators per persisted entity
(`setElection`, `setBallotStyle`, `setElectionEvent`,
`setBallotSelection`, `addCastVotes`, `setBypassChooser`, `setIsVoted`).
Order matters: `ballotStyles` must precede `ballotSelections` because
`setBallotSelection` is a no-op when its election entry is absent — the
hydration code first issues `resetBallotSelection({ballotStyle, force:
true})` to create the slot, then `setBallotSelection({ballotStyle,
ballotSelection})` to populate it.

**Why dispatch action creators rather than wholesale-replace state.**
We MUST NOT modify `voting-portal/src/store/store.ts`, so we cannot
inject a `preloadedState` into `configureStore` after the fact. Calling
the slice's own action creators per entity has a useful side effect: if
the portal renames an action or changes a payload, this file fails to
type-check at the dispatch site, telling us exactly what to update.
That is the same canary discipline as the Redux fixtures (section F).

**Slices we currently rehydrate**: `elections`, `electionEvent`,
`ballotStyles`, `ballotSelections`, `castVotes`, `extra` (only
`bypassChooser` and `isVoted`).

**Slices we deliberately skip**: `supportMaterials`, `documents`,
`auditableBallots`, `confirmationScreenData`. They will simply be empty
after a reload; the screens that consume them have not yet been lifted,
so nothing visible regresses. When one of those screens is lifted, add
its slice to `hydrateFromSnapshot` (and update the canary table below).

**Boot sequence in `BoothSpike.tsx`** (module-eval order matters):

1. `loadPersistedSnapshot()` — reads `localStorage`, returns `null` on
   first run, schema mismatch, or parse failure.
2. If a snapshot exists → `hydrateFromSnapshot(store, snapshot)`. Else
   → `seedBoothFixtures()` (bootstraps the bundled minimum fixture).
3. `installPersistence(store)` — subscribes to the store; **after**
   step 2, so we never persist an in-progress hydration.

Hydration internally toggles a `suspendWrites` flag so that the
many small dispatches it issues don't each trigger a full snapshot
write — only the post-hydration state hits `localStorage`.

**Schema versioning.** Snapshots tag themselves with `version: "v1"` and
the storage key carries the same suffix. When the persisted shape
becomes incompatible (e.g. voting-portal removes a slice we relied on),
bump the suffix in `PERSISTENCE_KEY` *and* the literal in
`PersistedSnapshot.version`. Old data is then silently ignored at boot
and the user gets a fresh fixture instead of a crash.

**Reset paths.** Two equivalent ways to wipe the persisted state:

- Click the **Reset workbench state** button in the workbench nav
  (added in `main.tsx`).
- From the browser console: `__resetWorkbench()` (a global installed
  alongside `__store` and `__dispatchLog` in `BoothSpike.tsx`).

Both call `clearPersistedSnapshot()` and reload the page; on next boot,
`loadPersistedSnapshot()` returns `null` and the bundled fixture
re-seeds.

**Canary if portal changes:**

- New slice added that booth screens consume → after a reload,
  affected screens show empty / spinner state. Add the slice to
  `hydrateFromSnapshot` and bump the `PERSISTENCE_KEY` suffix so
  pre-existing snapshots are discarded rather than partially applied.
- An existing `setX` action creator changes its payload shape → TS
  error in `persistence.ts` at the dispatch site.
- A slice's `RootState` field renames → TS error at the iteration over
  `state.<name>`.

**Cross-cutting note: where the booth must live now.** Because the
ReduxProvider was hoisted out of `BoothLayout` and into `Shell` (so the
workbench's own `/tally` page can read the same store), every workbench
page now sees the same Redux state. The booth screens themselves are
unaffected — the layering matches `voting-portal/src/index.tsx`, which
also wraps Redux outside its routes.

### I. Source code under `voting-portal/src/` — UNCHANGED
*outside* the portal source tree. If you ever feel tempted to edit a portal
file to make the workbench work:

1. **Stop.**
2. Re-read the "Why this document exists" section.
3. Try one of: `resolve.alias`, `define`, a new provider, a fixture seed,
   a substitute deep-import path. One of these almost always works.
4. If you genuinely cannot avoid a portal-source change, document it here
   under a new section "K. Concessions" with the exact diff and the reason
   it was unavoidable. Reviews of refresh PRs will then verify that the
   concession is still needed.

### J. Workbench-native chrome (`app/src/Workbench.tsx`)

Everything under `/wb/...` is **workbench-owned UI** — not lifted from
voting-portal, not from admin-portal. The decision is documented at the
top of section A's *do-not-lift* list: admin-portal is explicitly out
of scope. Instead the workbench ships its own minimal screens to
navigate the scenario.

Pages:

- `WorkbenchHome` at `/` — lists tenants. There is no `tenants` Redux
  slice in voting-portal, so the page derives a tenant catalog by
  scanning `state.electionEvent` and `state.elections` for `tenant_id`
  values and grouping by them. Per-tenant counters (events,
  elections, cast votes) come from the same scan.
- `WorkbenchTenant` at `/wb/tenant/:tenantId` — lists events for that
  tenant by filtering `state.electionEvent`.
- `WorkbenchEvent` at `/wb/tenant/:tenantId/event/:eventId` — lists
  elections in that event by filtering `state.elections`.
- `WorkbenchElection` at
  `/wb/tenant/:tenantId/event/:eventId/election/:electionId` — election
  detail with: metadata, cast-vote table, ballot-style summary, and
  CTAs into the booth at the production paths (see section E).

**Cast-vote bin honesty.** The election-detail page deliberately reads
`state.castVotes[electionId]` **and** `state.castVotes[eventId]` and
labels each row with which bin it came from. The portal's
`useAddFakeCastVote` under `DISABLE_AUTH` writes everything keyed by
`event_id` (it actually conflates id/election_id/area_id all to
`eventId`), so the only entries you will see in the real flow are the
"(demo path)" rows. We surface both bins because masking that quirk
would mislead the operator about what is actually in state.

**Rules:**

- Workbench-native pages MUST NOT import or re-implement voting-portal
  screens. They may freely import portal Redux slices, selectors, and
  action creators — those are part of the same package the booth uses.
- CTAs that enter the booth MUST link to the production paths
  (`/tenant/:t/event/:e/...`), never to `/wb/...`. Section E.
- These pages own their styling inline. There is no design system to
  match — admin-portal is out of scope by policy, and matching the
  booth's MUI theme would imply that these are booth screens, which
  they are not.

**When the scenario data model grows** (named checkpoints, voter
directory, scenario imports, ...), add a new workbench-native page
under `/wb/...` here. Do not reach for admin-portal as inspiration; the
whole point is to design the operator surface from scratch around the
workbench's actual needs.

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
   | `useSubmit must be used within a data router` | Legacy router used | E (use `createBrowserRouter`) |
   | `No routes matched location "/tenant/..."` | Path mirror is stale | E (extend `boothChildren`) |
   | Screen renders a spinner / self-redirects | Missing or mis-shaped fixture | F (fixtures) |
   | TS error in `fixtures/*` about a slice payload field | Portal slice type evolved | F (fixtures) |
   | `expected magic word 00 61 73 6d, found 3c 21 2d 2d` | Wasm pkg pre-bundled by Vite | A6 (`optimizeDeps.exclude`) |
   | `Cannot read properties of undefined (reading 'check_voting_...')` etc. coming from `sequent-core.js` | Wasm not initialized | D (mount `<WasmWrapper>`) |
   | `process is not defined` or `process.env.X` undefined | env var | A (`define` in vite.config) |

3. **Fix the smallest possible thing**, restart the dev server, and re-test.
4. **Update this document.** If you added/changed an adaptation, edit the
   relevant section so the next refresh starts from accurate state.
5. **Run the workbench tally page too** (`http://localhost:5173/`). It uses
   `velvet-core` and is unaffected by portal changes; if it breaks, the
   problem is in workbench glue or wasm-pack output, not the lift.

## Adaptations to add as we lift more screens

**Lifted screens so far** (full Vote-cast journey, plus the entry chooser):

- `ElectionSelectionScreen` at `tenant/:tenantId/event/:eventId/election-chooser`.
- `StartScreen`, `VotingScreen`, `ReviewScreen`, `ConfirmationScreen` at
  the portal's existing `tenant/.../election/:electionId/{start,vote,review,confirmation}`
  paths.

When extending past these, the following are the most likely next-step
categories of work (in roughly the order they will be needed):

1. **A real election public key in the ballot style fixture.** ✅ Done.
   The fixture uses `DEFAULT_PUBLIC_KEY_RISTRETTO_STR` from sequent-core
   directly, so `encryptBallotSelection` produces real ciphertext and the
   booth navigates from `/vote` to `/review`. See section F.
2. **Initialize `ballotSelections` from the fixture, not only from
   StartScreen.** ✅ Done — `seedBoothFixtures()` dispatches
   `resetBallotSelection`. See section F.
3. **Mounting `<ApolloProvider>`.** ✅ Done. The workbench mounts an
   `ApolloProvider` with a client whose link is `ApolloLink.empty()`, so
   `useQuery`/`useMutation` are satisfied at context level but no network
   call is ever made. Under `DISABLE_AUTH: true` ReviewScreen takes
   `useAddFakeCastVote` and the real mutation is never invoked, so the
   full booth flow (Start → Vote → Review → Confirmation) succeeds
   end-to-end without any GraphQL plumbing. See section D, layer 4.
4. **Per-operation GraphQL fixtures for screens whose query results
   aren't already in Redux.** **Deferred — not required by any
   voting-portal screen lifted to date.** Every `useQuery` in the booth
   path **and in `ElectionSelectionScreen`** is gated on
   `globalSettings.DISABLE_AUTH` (skip) and reads its data from Redux
   instead. `BallotLocator` has one unguarded `useQuery(GET_BALLOT_STYLES)`
   but it's only used to dispatch `updateBallotStyleAndSelection` if the
   response arrives — with `ApolloLink.empty()` the dispatch is silently
   skipped, and the screen falls back to `selectFirstBallotStyle` from
   Redux. `ConfirmationScreen` likewise has all `useQuery` sites either
   `DISABLE_AUTH`-gated or `!documentId`-gated. So the entire
   demo-flagged voting-portal surface is satisfied by the **empty link
   + Redux fixtures** pair, without any per-operation mocks. If a future
   screen actually reads from `useQuery` results without a Redux fallback,
   the pattern below is the minimum solution; until then it stays unbuilt:

   ```ts
   // app/src/fixtures/gql/index.ts
   import {ApolloLink, Observable} from "@apollo/client"
   import {entitledElectionsFixture} from "./entitledElections"

   export const workbenchLink = new ApolloLink((op) => new Observable((sink) => {
       switch (op.operationName) {
           case "GetEntitledElections":
               sink.next({data: entitledElectionsFixture})
               break
           default:
               sink.next({data: null})
       }
       sink.complete()
   }))
   ```

   The division of labour stays clean: data that **production puts in
   Redux** gets a `setX` dispatch in a fixture module; data that
   **production reads via GraphQL** would get an `ApolloLink` entry. Do
   not smear one across the other.
5. **Translation key fidelity.** Once a screen renders, missing
   translations show as raw keys (`booth.start.title`). Decide between
   shipping a copy of the portal's locales as a static asset and silencing
   the warnings.
6. **Apollo mocks vs. a fake transport.** If a single workbench-local
   `ApolloLink` switch becomes unwieldy (dozens of operations,
   interdependent state), an in-memory schema (with
   `@graphql-tools/mock`) may scale better. Decision deferred until pain
   is felt.
7. **Extending the fixture.** As later screens consume more of the store
   (cast votes, audit data, etc.), `boothFixtures.ts` grows. Keep it a
   single module per screen group rather than one fixture file per slice
   — easier to keep coherent across slice boundaries.

**Out of scope.** The workbench lifts **voting-portal** screens only,
plus whatever direct dependencies of those screens (ui-core, ui-essentials,
sequent-core wasm) come along for the ride. We do not lift admin-portal
functionality — it is a separate app with its own concerns (election
setup, results, audit dashboards) and a different GraphQL surface.
If a future workbench task does need admin-portal coverage, it should be
a parallel `BoothSpike`-equivalent module with its own provider stack
and fixture tree, not an extension of this one.

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
