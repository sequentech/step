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
  array as the election's. The `public_key` is **the public half of the
  workbench-owned ElGamal keypair** (`pkB64` from `WorkbenchKeypair`,
  see section M), passed in as `publicKeyB64` to `buildBallotStyle` /
  `seedBoothFixtures`. The booth therefore encrypts cast ballots under
  a key whose secret half the workbench also holds, which is what
  closes the encrypt → decrypt → decode → tally loop in the browser.
  An older revision of this fixture used `DEFAULT_PUBLIC_KEY_RISTRETTO_STR`
  from `packages/sequent-core/src/encrypt.rs` (whose secret half is
  not bundled); that only validated the *encrypt* path. Marked
  `is_demo: false` so StartScreen does not pop the "this is a demo"
  dialog.
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

   (One such concession exists today — see section **L. Concessions:
   edits to `voting-portal/src/`** — for the demo-mode-only fix to
   `useAddFakeCastVote` in `ReviewScreen.tsx`.)

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

### K. Workbench-owned overlay state (`app/src/workbenchStore.ts`)

Some operator-facing scenario data (voter directory, currently-
impersonated voter, cast-vote → voter attribution ledger) has no
counterpart in voting-portal's Redux store. Production keeps voters in
Hasura and reads identity from Keycloak; under `DISABLE_AUTH` the booth
runs anonymously and `useAddFakeCastVote` writes
`voter_id_string: null`.

Rather than add a new slice to the portal store (which would require
editing `voting-portal/src/store/store.ts` — see section I), the
workbench keeps this state in a tiny separate mini-store
(`workbenchStore.ts`):

- A module-local `WorkbenchExtraState` object.
- `useSyncExternalStore`-based subscription (`useWorkbench` hook).
- A handful of named mutations: `addVoter`, `removeVoter`,
  `setActiveVoter`, `attributeCastVote`, `captureRepairedCastVote`,
  `setRepairedDecodedBigInts`, `setKeypair`, `replaceWorkbenchState`,
  `seedDemoVoters`.

State fields:

- `voters`, `activeVoterId`, `castBy` — directory + attribution ledger.
- `repairedCastVotes` — per-cast-vote bridge data (plaintext selection
  snapshot, real election id, and `decodedBigInts: Record<contestId,
  decimalString>` filled in asynchronously by the decrypt bridge — see
  section M).
- `keypair: WorkbenchKeypair | null` — the workbench-owned ElGamal
  keypair (`{pkB64, skB64}`, base64-no-pad, strand/borsh-serialised
  via `velvet-wasm::generate_keypair`). Generated once on first boot,
  written through `setKeypair` which is first-call-wins so a stray
  re-seed cannot orphan already-captured cast votes encrypted under
  the old pk. Full lifecycle in section M.

**Persistence integration.** The mini-store is folded into the same
`PersistedSnapshot` the portal Redux store rides on, via an optional
`workbench?: WorkbenchExtraState` field. Both stores share one
`writeSnapshot()` call (in `installPersistence`) and one
`hydrateFromSnapshot()` entry point. As a result:

- The auto-resume slot captures voter directory changes alongside
  Redux state.
- Named checkpoints round-trip workbench state unchanged.
- A snapshot written before this field existed loads fine: missing
  `workbench` rehydrates to an empty directory.

**Attribution ledger.** `installPersistence` runs a small cast-votes
watcher: on each Redux dispatch it diffs `state.castVotes` against a
running set of seen ids, and for each newly-observed cast vote it
calls `attributeCastVote(id)`, which (when an active voter is set)
records `castBy[id] = activeVoterId`. Election-detail pages then
render this attribution in the "Voted by" column. The ledger is the
workbench's substitute for the production `voter_id_string` field,
which stays `null` in the portal store because we don't touch portal
source.

**Rules:**

- Never add Workbench-only state by modifying portal slices.
- Anything operator-facing that has no production counterpart goes
  in `workbenchStore.ts`. Reusable selectors and React integration
  live next to it.
- Mutations from the mini-store must trigger a snapshot rewrite. The
  `subscribeWorkbench` listener installed in `installPersistence`
  handles this — don't write to localStorage directly from mutation
  functions.
- Hydration is order-sensitive: in `hydrateFromSnapshot`, the
  workbench overlay is restored BEFORE replaying portal state, so the
  ledger is in place when the cast-votes watcher's seen-set is
  primed.

**Bridge from portal cast-vote records to the workbench overlay
(`repairedCastVotes`).** After the section L concessions, the demo's
`useAddFakeCastVote` writes a cast-vote record that matches
production shape:

- `id` is the per-cast `ballotId` (unique).
- `election_id` / `area_id` are the real election id (slice buckets
  correctly).
- `content` is `JSON.stringify(hashableBallot)` — the encrypted
  ballot, same bytes the backend would persist.
- `voter_id_string` is `null` (handled by the attribution ledger
  above).

The portal does NOT, however, retain the cleartext selection anywhere
after the user leaves the voting screen. `state.ballotSelections` is
the only place it ever lived, and in production nothing reads it
after the cast vote is dispatched. The workbench wants an inspection
view of "what did the voter actually pick", so:

The workbench's bridge — `tryCaptureRepairedCastVote()` in
`persistence.ts`, fired alongside `attributeCastVote()` by the
cast-votes watcher — captures **only the plaintext selection**:

1. Look up the matching ballot style by the cast vote's
   `election_id` (a one-shot lookup; no pivots needed now that L.1
   fixed the bucket).

2. Read `state.ballotSelections[cv.election_id]` — the structured
   `DecodedVoteContest[]` the voter just built. This is also the
   value the future inline tally will encode via `tally.ts`'s
   `encodeBallot` and feed into `runTally`. We JSON deep-clone
   before storing so later in-place Redux mutations can't corrupt
   the snapshot.

3. Record `RepairedCastVote { electionId, ballotStyleId, selection,
   capturedAt }`. There is intentionally no encrypted-ballot field
   here: `cv.content` already holds that, the bridge would just be
   duplicating it.

The bridge is **first-observation-wins**: `captureRepairedCastVote`
is a no-op if `repairedCastVotes[castVoteId]` already exists. This
makes both the watcher and `hydrateFromSnapshot` idempotent — a
reload that replays cast votes through the same watcher won't
overwrite a previously-snapshotted bridge entry.

---

### L. Concessions: edits to `voting-portal/src/`

Section I declares portal source untouched, and that remains true for
production code paths. The accepted edits are scoped strictly to the
`DISABLE_AUTH` / `isDemo` branch, and all live in one file
(`voting-portal/src/routes/ReviewScreen.tsx`). There are two of them;
both have the same shape ("make demo behave like production minus the
network call, not like a different system entirely").

#### L.1 `useAddFakeCastVote` — correct the synthetic cast-vote shape

The original demo helper wrote synthetic cast votes with
`id = eventId`, `election_id = eventId`, `area_id = eventId`. Both
fields were wrong:

- `id = eventId` collided across casts. The `castVotes` slice dedupes
  by `id` within its election bucket, so every new demo cast silently
  overwrote the previous one. With auth disabled and the booth used
  for repeat testing, the slice never accumulated more than one cast
  vote.
- `election_id = eventId` mis-bucketed the cast vote: the slice keys
  by `election_id`, so `selectCastVotesByElectionId(realElectionId)`
  returned `[]`. Anything reading cast votes by election (the booth's
  own confirmation screen, the workbench's election detail, a future
  tally) had to know to look under the event id instead.

The fix changes the helper's signature from `() => void` to
`(electionId, ballotId, content) => void` (see L.2 for the `content`
parameter) and uses those at the two call sites (both already had the
values in scope: `ballotStyle.election_id` + `ballotId` in
`castBallotAction`, and `ballotData.electionId` + `ballotData.ballotId`
in `goldenUserCastBallotAction`). `id` now takes the unique per-cast
`ballotId`; `election_id` and `area_id` both take the real
`electionId`; `election_event_id` continues to be the parent `eventId`.

**Why this was accepted:**

- It only runs when `isDemo || DISABLE_AUTH` — never in production.
- The whole helper is a stand-in for `INSERT_CAST_VOTE`; the real
  flow goes through `tryInsertCastVote` and the backend assigns the
  fields.
- The fix is type-checked by the existing call sites and isolated to
  one function plus its two callers (~10 lines diff).
- Without the fix, the workbench had to maintain a parallel
  "bridged election_id" column and a dual-bin cast-votes view in
  every screen, AND any future inline tally would need the same
  pivot. The bridge documented in section K still exists (for
  plaintext selection), but it no longer has to repair the
  election id.

**Refresh-PR guardrail.** If voting-portal renames the helper, changes
its parameter list, or adds new call sites, the refresh PR must
re-apply the same shape (real `electionId`, unique `ballotId`).
Reviewers should reject a refresh that silently brings back
`id = eventId` or `election_id = eventId`.

#### L.2 `castBallotAction` — populate `cv.content` in demo mode

The original demo branch of `castBallotAction` short-circuited *before*
the `toHashableBallot` call and handed `useAddFakeCastVote` no content
at all, so the synthetic cast vote carried `content: ""`. The
production branch, in contrast, runs `toHashableBallot(auditableBallot)`
and persists `JSON.stringify(hashableBallot)` as the cast vote's
content. This is the real asymmetry to point out:

- `VotingScreen` already ran `encryptBallotSelection` for both demo
  and production — the encrypted `IAuditableBallot` is sitting in
  `state.auditableBallots[electionId]` by the time the user reaches
  review. The demo branch wasn't skipping *encryption*; it was
  skipping a pure transform (`toHashableBallot`) and the backend
  insert. Conflating "don't hit the network" with "don't compute the
  ballot content" is what made the demo behave unlike production.

The fix lifts the `toHashableBallot` / `toHashableMultiBallot` call out
to the top of `castBallotAction` so both branches share it:

- The demo non-golden branch now passes `JSON.stringify(hashableBallot)`
  as the third arg to `addFakeCastVote`, so `cv.content` ends up
  byte-shape-identical to a production row.
- The demo golden branch now writes the real
  `JSON.stringify(hashableBallot)` into `sessionStorage["ballotData"].ballot`
  (previously the literal placeholder `"{}"`), and
  `goldenUserCastBallotAction` forwards `ballotData.ballot` straight
  through to `addFakeCastVote`.
- The production branch is byte-identical to before — `hashableBallot`
  just moved a few lines up, and its error handling (the
  `TO_HASHABLE_BALLOT_ERROR` path) is unchanged. No new code paths,
  no new throws.

**Why this was accepted:**

- Same scope as L.1: only runs when `isDemo || DISABLE_AUTH`.
- The transform we un-skipped is already imported in this file and
  already runs on production. We aren't introducing new behaviour;
  we're un-skipping work that was being elided for no real reason.
- The workbench bridge no longer has to sniff `sessionStorage` for an
  "encrypted hashable ballot" copy that, in the non-golden demo path,
  was never written there in the first place. The bridge becomes
  purely about the plaintext selection (which Redux discards after
  voting and which has no production counterpart at all), and the
  encrypted ballot is inspected directly from `cv.content` like in
  production.

**Refresh-PR guardrail.** If voting-portal restructures
`castBallotAction` (e.g. inlines `tryInsertCastVote` or moves
`toHashableBallot` into the service layer), the refresh must keep two
invariants: (a) the demo branch dispatches `addCastVotes` with a
populated `content` field, not an empty string; (b) `sessionStorage`'s
demo ballot data carries the real stringified hashable ballot, not a
placeholder. The workbench tests pin this down by inspecting
`cv.content.length > 0` after a demo cast.

---

### M. Workbench-owned keypair and the in-browser tally loop

Production elections key-share ElGamal trustees on a separate machine
and the booth never sees the secret half. The workbench operates in
the opposite regime: a single, ephemeral keypair lives entirely in
the operator's browser so that we can run the *whole* lifecycle —
encrypt, persist, decrypt, decode, tally — without leaving the page.
This section documents the keypair's lifecycle and the bridge that
turns a stored `cv.content` back into the plaintext `BigUint` the
tally consumes.

This is purely a workbench scaffold. No portal source is involved; if
the lift refreshes against a future portal that exposes its own
decrypt surface, this section can simply go away.

#### M.1 The `velvet-wasm` surface (`packages/workbench/velvet-wasm/`)

The workbench's local wasm package re-exports three thin
wasm-bindgen functions on top of `sequent-core` (in-tree source) and
`strand`:

- `generate_keypair() -> {pkB64, skB64}` — calls
  `strand::elgamal::generate_keypair::<RistrettoCtx>` and
  borsh-encodes each half to base64-no-pad.
- `decrypt_ballot_content(content_json, sk_b64, contest_id) -> string` —
  parses a `HashableBallot` JSON (NOT `AuditableBallot`; see canary
  below), finds the contest entry by id, runs
  `sk.decrypt(&target.ciphertext) → ctx.decode(&element) → [u8;30] →
  decode_array_to_vec → decode_bigint_from_bytes`, and returns the
  resulting `BigUint` as a decimal string.
- `encode_ballot(contest_json, decoded_vote_contest_json) -> string` —
  wraps `Contest::encode_plaintext_contest_bigint(&decoded)`. Used
  by the UI's round-trip badge: the value it produces must match
  what `decrypt_ballot_content` recovers from the same `Contest` +
  `DecodedVoteContest` pair.

The package is consumed by the workbench app via
`"velvet-wasm": "file:../velvet-wasm/pkg"` (section B). Voting-portal
continues to use its prebuilt `sequent-core` tgz for the encrypt
path; these are two different wasm artifacts, but they share the
in-tree `sequent-core` source for the wire formats and the encoding
rules, which is why they interoperate.

**Canary if `sequent-core` changes:**

- The borsh layout of `HashableBallotContest` changes (e.g. a field
  reordered, the proof type swapped) → `decrypt_ballot_content`
  fails to deserialise with *"Failed to decode scalar"* or
  *"unexpected variant"*. Re-derive the right struct against the
  current `sequent-core::ballot` module.
- The 30-byte plaintext encoding changes its length-prefix convention
  (`encode_vec_to_array` / `decode_array_to_vec` in
  `sequent-core/src/ballot_codec/vec.rs`) → `decrypt_ballot_content`
  returns a numerically wrong `BigUint` (off by `len + payload*256`
  on the first byte) and the round-trip badge stays red. Re-sync the
  decrypt post-processing with whatever the new convention is.

#### M.2 Keypair lifecycle (`BoothSpike.tsx` + `workbenchStore.ts`)

1. **Boot.** Before `seedBoothFixtures` runs (or before
   `hydrateFromSnapshot` finishes), the boot path checks
   `getWorkbenchState().keypair`. If absent, it calls
   `velvet-wasm::generate_keypair()` and passes the result to
   `setKeypair({pkB64, skB64})`.
2. **Fixture seeding.** `seedBoothFixtures(publicKeyB64)` is now
   parameterised on the public half. The bootstrap path reads
   `state.keypair.pkB64` and passes it in. As a result the ballot
   style's `public_key` is always the workbench's pk on first boot.
3. **Persistence.** The keypair is part of `WorkbenchExtraState`, so
   it round-trips through `PersistedSnapshot` like every other
   overlay field (section K). Pre-step-6 snapshots that lack
   `keypair` rehydrate as `null`; the boot path then generates and
   sets one before the cast-votes watcher runs.
4. **Reset.** `__resetWorkbench()` clears the snapshot, which wipes
   the keypair alongside everything else; a fresh pair is generated
   on the next boot.

`setKeypair` is **first-call-wins** by design. A snapshot already
holds a (pk, sk) pair; the cast votes inside it were encrypted under
that pk. Overwriting on rehydrate would silently make decryption
return garbage. The same rule protects against a stray dev hot-
reload re-generating a key.

**Canary if portal changes:**

- Portal starts validating `ballot_eml.public_key` against a server-
  side allowlist → the workbench's fresh pk gets rejected at vote
  time. Either feed the workbench pk to the portal's pre-flight
  validator, or pin a fixed pk and re-import the matching sk on
  boot.
- Portal moves `public_key` to a different path on `IBallotStyle` →
  TS error in `buildBallotStyle` at the assignment site.

#### M.3 Decrypt bridge (`persistence.ts` cast-votes watcher)

`installPersistence`'s existing cast-votes watcher gets a second
side effect: once `captureRepairedCastVote()` has recorded the
plaintext selection (synchronously, see section K), the watcher
launches an async pass that:

1. For each contest in `cv.content` (a `HashableBallot` JSON), calls
   `decryptBallotContent(content, skB64, contestId)` and collects
   `{contestId: decimalString}`.
2. Calls `setRepairedDecodedBigInts(castVote.id, decoded)` to merge
   the result into the existing `RepairedCastVote`.

The split into a sync capture + async fill is deliberate: the UI
sees the plaintext selection immediately (snapshot pulled from
Redux at cast time), and the decrypted `BigUint` appears a tick
later without blocking the React render. If decryption throws (no
keypair, malformed content, wrong key after a partial reset), the
entry is simply left empty; the round-trip badge then reads "—"
rather than asserting equality.

The decrypt does **not** re-run on hydrate. Re-decrypting a cast
vote at boot would risk doing so under a different keypair if the
operator wiped state in between, and the `decodedBigInts` are
already in the snapshot. Hydration just rehydrates whatever value
was last written.

#### M.4 Round-trip badge and tally consumption

The Cast Votes table in `Workbench.tsx` renders a
`DecodedContestRow` per `(repairedCastVote, contest)` pair:

- Decoded value: `repairedCastVote.decodedBigInts[contestId]`
  (decimal string).
- Round-trip check: call `encodeBallot(JSON.stringify(contest),
  JSON.stringify(selectionForThatContest))` and compare. Green when
  equal, red otherwise, "—" when either side is missing.

The "Run tally" button in `electionTally.ts` consumes the
`decodedBigInts` map directly. There is no re-encrypt-then-decrypt
trip through wasm — the BigUints flow straight from
`repairedCastVotes` into the tally code path, exactly the way a
production trustee would feed decrypted plaintexts in.

**End-to-end canary.** Cast a Blue vote on the bundled fixture
(plurality-at-large, two candidates, `max_votes=1`); expected
`decodedBigInts[<contestId>] === "4"` (bases `[2,2,2]`, choices
`[0,0,1]`, mixed-radix LSB), round-trip badge green, tally reports
`Blue: 100% (1), Red: 0% (0), valid=1, invalid=0`. If the BigUint
shows up as `1025`, the length-prefix unwrap in
`decrypt_ballot_content` regressed (see M.1).

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

### Future: generic state inspector

Not a portal adaptation — a workbench-only feature, parked here so it
doesn't get lost.

Today several panels render a bare mono id (ballot style id, parent
event link, cast-vote id, voter id, future encoded-ballot id, etc.).
Each id is opaque: the operator can see *that* a record exists but
not *what's in it* without dropping into Redux DevTools or the
browser console. The cast-vote `<details>` row we built for the
election page is essentially a hand-rolled mini-inspector for one
specific record type — a pattern that doesn't compose: every new
introspectable record type would otherwise grow its own bespoke
panel.

Sketch:

- A single route `/wb/inspect?kind=<kind>&id=<id>` (or similar) that
  renders a recursive collapsible JSON tree over whatever the resolver
  returns. No per-type detail page.
- A small resolver map keyed by `kind`: `ballotStyle`, `election`,
  `event`, `tenant`, `castVote`, `voter`, `checkpoint`, `snapshot`,
  later `encodedBallot`, `tallyResult`. Each resolver pulls from the
  appropriate store (portal Redux or workbench store) by id.
- Anywhere we render an id today, wrap it in a link/button to the
  inspector pre-focused on that record. Id stays visible; the
  inspector is the "what is this thing actually" escape hatch.
- Optional polish: search/filter, copy-to-clipboard, diff two
  snapshots (e.g. pre/post checkpoint), pretty-print known nested
  JSON strings (like `cv.content`).

When to build: **after** step 5 (inline per-election tally). Rationale
— the inspector's value compounds with the number of record types it
covers, and step 5 will introduce two more juicy ones (the encoded
ballot per cast vote and the tally result). Building it after step 5
lands it with 4–5 useful resolvers instead of 2, on stabilized bridge
schema.

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
