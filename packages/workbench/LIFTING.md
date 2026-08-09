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

The same conservatism applies to **any** production source the workbench
touches, not just the portal — `sequent-core`, `velvet` and `strand`
included. Where an edit was genuinely unavoidable it is inventoried in
section **L**, and the workbench's Diagnostics page (*Shared-source
drift*) diffs every such tree against `origin/main` so an undocumented
edit is visible rather than discovered during the next catch-up merge.

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
| A7 | `resolve.alias` `^sequent-core$` → `<repo>/packages/sequent-core/pkg` + opt-in `build:sequent-core` script (NOT chained into `predev`/`prebuild`) | The lifted booth source imports `from "sequent-core"`. By default Vite resolves that to the workspace-hoisted `node_modules/sequent-core/` unpacked from the committed prebuilt tgz at `voting-portal/rust/sequent-core-0.1.0.tgz` — so workbench iteration sees that frozen snapshot, not your local Rust edits. The alias gives you an escape hatch: when `packages/sequent-core/pkg/` exists, Vite reads `sequent-core` from there instead. The companion `yarn build:sequent-core` script runs `wasm-pack build --out-name index --release --target web --features=wasm,default_features` to refresh `pkg/`. Works on stable Rust without the Nix devshell (the wasm-pack-enablement commit in sequent-core makes that possible). The script is intentionally opt-in — contributors who haven't touched sequent-core Rust pay no toolchain cost, and the alias falls through to the hoisted tgz copy when `pkg/` doesn't exist. That fall-through is **not automatic**: `resolve.alias` rewrites unconditionally once registered, so the entry is wrapped in an `fs.existsSync(sequentCorePkg)` guard and simply isn't registered when `pkg/` is absent. Before that guard existed, a fresh clone failed on every `sequent-core` import with `Failed to resolve import "sequent-core"` (from `src/tally.ts` and `ui-core/src/services/{i18n,wasm}.ts`). Note that **Rust changes to non-`#[wasm_bindgen]` parts of sequent-core already propagate via velvet-wasm**, which depends on sequent-core as a Cargo path-dep and rebuilds it on every `yarn build:wasm` — so this alias only matters for edits to the `#[wasm_bindgen]` API surface in `packages/sequent-core/src/wasm/` (the exports the lifted booth calls directly). | Build fails with `Failed to resolve import "sequent-core"` only if you've deleted `packages/sequent-core/pkg/` after running the script at least once **and** node hoisting hasn't placed a copy at `packages/node_modules/sequent-core/`. Normal fix: run `yarn install` (re-hoists) or `yarn build:sequent-core` (re-creates `pkg/`). If voting-portal renames the import, update the regex. |

### B. Workspace dependency graph (`app/package.json`)

The workbench app needs `voting-portal` as a workspace dep (`"*"`), plus
any npm dep that the lifted portal source files `import` **and that Vite
fails to resolve** under the workbench's `node_modules` layout. Most
transitive deps of voting-portal resolve via yarn workspace hoisting
through the `voting-portal` package itself — e.g. `@mui/icons-material`,
`@mui/system`, `lodash`, `keycloak-js`, `web-vitals`,
`@fortawesome/free-solid-svg-icons`, `@graphql-typed-document-node/core`
all appear in portal `import` lines and yet are absent from
`app/package.json` because the workbench never has to resolve them
directly. The practical rule is the canary clause below: add a dep only
when the dev server reports `Failed to resolve import "X"`.

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
grep -rhE "^import .* from ['\"]" ../../voting-portal/src/ | \
  sed -E 's/.*from ['\''\"]([^'\''\"]+)['\''\"].*/\1/' | \
  grep -v '^[./]' | sort -u
```

from `packages/workbench/app/` and diff the result against the dep list.
Anything new is a candidate addition — but only matters if Vite can't
resolve it through the workspace already (see paragraph above).

### C. Runtime configuration files

#### `app/public/global-settings.json` — substitute for `/global-settings.json`

Voting-portal's `SettingsContextProvider` fetches `/global-settings.json` at
startup. We ship a static one with:

- `DISABLE_AUTH: true` — short-circuits `KeycloakProvider` in
  `voting-portal/src/index.tsx` to render its children directly, bypassing
  all real auth. **This is the single most important toggle for the lift**:
  without it, the embedding would need a full Keycloak mock.
- Service URLs (`KEYCLOAK_URL`, `HASURA_URL`, `BALLOT_VERIFIER_URL`,
  `RESULTS_PORTAL_URL`, `PUBLIC_BUCKET_URL`) pointing at
  `http://127.0.0.1:0/` so any code path that escapes our mocks fails fast
  rather than hitting real infrastructure.
- Sensible defaults for the remaining `SettingsContext` fields.

`RESULTS_PORTAL_URL` is the most recent addition and is a worked example
of the canary below: it appeared on the `GlobalSettings` interface
upstream, our file did not have it, and nothing failed — the key simply
read as `undefined` wherever a screen consumed it. It was found by
diffing the interface, not by using the app.

**Canary if portal changes:** new required keys in `SettingsContext`
usually fail *silently* — auth-aware code branches take their
non-demo path, screens omit affordances, or queries fire against
`http://127.0.0.1:0/` and hang. A hard `undefined.foo` crash is the
exception, not the rule (verified by probe C: renaming
`HASURA_URL`/`DISABLE_AUTH` keys produced silent degradation, not
errors). If a lifted screen renders "slightly wrong" with no console
error, suspect this. Inspect
`voting-portal/src/providers/SettingsContextProvider.tsx`
(specifically the `GlobalSettings` interface and the
`defaultSettingsValues` constant) and add the new keys.

#### `app/public/locales/*` — optional translation files

i18next will warn on missing keys but the page still renders. Not currently
mocked; add files here only if a screen's UI becomes unreadable.

### D. Provider stack (`app/src/BoothSpike.tsx`)

Every voting-portal screen relies on a chain of React Context providers.
Production wires most of them in `voting-portal/src/index.tsx`, plus
`<ApolloWrapper>` one layer deeper in `voting-portal/src/App.tsx`. The
production order, outermost first, is:
`WasmWrapper → SettingsWrapper → KeycloakProviderContainer → Redux Provider → ThemeProvider → RouterProvider`,
then inside the router `App` mounts `ApolloWrapper` around its `<Outlet />`.

The workbench wires the *minimum* subset needed for the screen under test.
The order in `BoothLayout` (`Theme → Settings → Apollo → Wasm`) is
different from production order — none of these providers consume each
other's context during render, so the order is functionally interchangeable.
If you ever lift a new provider whose render *does* depend on another's
context (rare), match production's nesting at that point.

Currently mounted:

**`Shell`** (in `app/src/main.tsx`, wraps every workbench page including
the workbench-native `/pipeline` view), outermost first:

1. `<ReduxProvider store={...}>` reusing the **production `store`** from
   `voting-portal/src/store/store`. This is deliberate: same reducers,
   same selectors, same shape. Diverging would defeat the lift. Hoisted
   to `Shell` so the workbench's own pages (e.g. `/pipeline`, the
   inspector) read the same store the booth writes to. Production also
   wraps Redux outside its routes in `voting-portal/src/index.tsx`, so
   the layering matches.

**`BoothLayout`** (mounted under `Shell` for booth routes only),
outermost first:

1. `<ThemeProvider>` from `@mui/material`, with `theme` from
   `@sequentech/ui-essentials`. **Required** — MUI components throw without it.
2. `<SettingsWrapper>` from `voting-portal/src/providers/SettingsContextProvider`.
   This is the production wrapper (`SettingsContextProvider` + a
   `SettingsGate` that gates children behind a `<Loader />` until the
   fetch resolves), lifted as-is. It fetches `/global-settings.json`
   (served from `app/public/`). **Required** because the SettingsContext **default**
   ships with `DISABLE_AUTH: false` and `HASURA_URL: "http://localhost:8080/v1/graphql"`,
   so any screen that reads `globalSettings` before settings load would
   see production-pointing defaults and (for ReviewScreen) fire the
   `GET_ELECTIONS` query against a real Hasura URL.
3. `<ApolloProvider client={apolloClient}>` from `@apollo/client/react`,
   with a workbench-local `ApolloClient` whose link is `ApolloLink.empty()`
   (the observable completes with no data). In production this is mounted
   one layer deeper than the rest, inside `voting-portal/src/App.tsx`
   (via the local `ApolloWrapper`); the workbench mounts it directly in
   `BoothLayout` instead. **Required** by ReviewScreen
   (it calls `useMutation(INSERT_CAST_VOTE)` and `useQuery(GET_ELECTIONS)`)
   even though under `DISABLE_AUTH: true` neither operation actually
   executes against a server: the `useQuery` is skipped, and ReviewScreen
   takes the `useAddFakeCastVote` branch (search for `useAddFakeCastVote` —
   gated on `isDemo || globalSettings.DISABLE_AUTH`) which mutates Redux directly
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
   specific routes are `/wb/...` and `/pipeline`.

Concretely, `main.tsx`:

- Builds a `createBrowserRouter` with a `<Shell>` layout containing a
  small nav, the global `<ReduxProvider>`, and an `<Outlet />`.
- **Workbench-owned subtree** under `/wb/...` and `/pipeline`. Lift-irrelevant
  detail — see `WORKBENCH.md` for the inspector routes. The only fact the
  lift cares about is that nothing under `/wb/...` resolves to a lifted
  portal screen.
- **Booth subtree**: `<BoothLayout />` mounting `boothChildren` (defined
  in `BoothSpike.tsx`). `boothChildren` mirrors the portal's
  `tenant/:tenantId/event/:eventId/{election-chooser, election/:electionId/*}`
  subtree and pairs each route with the same `action` the portal wires.

`BoothSpike.tsx` exports two things only:

- `BoothLayout` — the booth-screen-only providers (Theme, Settings,
  Apollo, Wasm). The ReduxProvider lives in `Shell` instead, so
  `BoothLayout` does NOT mount Redux itself.
- `boothChildren: RouteObject[]` — the route data with elements and actions.
  The shape mirrors the portal's own `tenant/:tenantId/event/:eventId`
  subtree: `election-chooser` and `election/:electionId/*` are siblings
  under a common parent (NOT cousins). Keeping that parent-child
  structure intact is what lets the chooser's absolute-path navigation
  (`navigate(\`/tenant/.../election/${id}/start\`)`) resolve at the
  same URLs the portal produces in production.

**Workbench-native pages reach the booth via production paths.** Any
CTA from `/wb/...` into the booth links to
`/tenant/:t/event/:e/election/:el/start`, never to a `/wb/...` path.
That keeps a single source of truth for "what URL the booth is at":
the portal's own absolute-`<Link>` strings.

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
subscriptions and Apollo cache writes. The workbench has neither. Instead
the workbench ships **bundled snapshots** — full `PersistedSnapshot` JSON
files under `app/src/fixtures/snapshots/` — that capture the Redux state
(plus the workbench overlay) the screens need to render. The boot path
hydrates one of these on first run, and `hydrateFromSnapshot` calls the
portal's **own** action creators (`setElection`, `setElectionEvent`,
`setBallotStyle`, ...) to populate the **production store**. Same store
instance, same reducers, same selectors — only the data source differs.
This is the lift-relevant contract; the snapshot-authoring workflow,
storage tiers and provenance model are workbench-side concerns covered in
`WORKBENCH.md`.

The shipping `default.json` snapshot encodes:

- An `election` with one contest ("Favourite colour") and two candidates.
  `num_allowed_revotes: 0` is interpreted by `canVoteSomeElection` as
  *unlimited revotes*, which keeps the selector truthy without seeding
  a working `castVotes` feed.
- An `electionEvent` (the parent of the election).
- A `ballotStyle` whose `ballot_eml.contests` is the same `IContest`
  array as the election's. The `ballot_eml.public_key.public_key` is
  the **`pkB64`** of the workbench-owned ElGamal keypair stored under
  `workbench.keypair` (section M). The booth therefore
  encrypts cast ballots under a key whose secret half the workbench
  also holds, closing the encrypt → decrypt → decode → tally loop in
  the browser. An older revision of the fixture used
  `DEFAULT_PUBLIC_KEY_RISTRETTO_STR` from `packages/sequent-core/src/encrypt.rs`
  (whose secret half is not bundled); that only validated the
  *encrypt* path. The ballot style is marked `is_demo: false` so
  StartScreen does not pop the "this is a demo" dialog.
- A pre-initialized `ballotSelections[electionId]` entry with all
  candidates at `selected: -1`. **Critical**: in production this
  dispatch happens inside `StartScreen` when the voter clicks *Start
  Voting*. `setBallotSelectionVoteChoice` is a silent no-op
  (`if (!currentElection) return state`) until that initialization has
  happened, so a user clicking a candidate on `/vote` after a hot
  reload would see the visual highlight flicker but the redux state
  never update, and the *Next* button's `encryptAndReview` would
  early-return because `selectionState` is `undefined`. Every URL has
  to be a valid entry point (hot reload on `/vote`, deep links), so
  the empty selection structure is pre-seeded in the snapshot.
- **`election.status` and `electionEvent.status` with `voting_status:
  OPEN`** (and `kiosk` / `early` set to `CLOSED`). Required for
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
**Editing the bundled snapshot.** Do not hand-edit `default.json`
blindly — a Vite plugin enforces snapshot-keypair ↔ ballot-style
consistency at build start (see §M.2 below). The authoring workflow
(save a checkpoint, copy its JSON, paste under
`app/src/fixtures/snapshots/`) is described in `WORKBENCH.md`.

**Convention:** because hydration goes through the portal's own action
creators (`hydrateFromSnapshot` in `persistence.ts`), payload-shape
drift surfaces as TS errors in `persistence.ts` at the dispatch site —
this is the same canary discipline as direct-dispatch fixtures, just
centralized.

**Canary if portal changes:**

- New required field on `IElectionExtended` or `IElectionEvent` → TS
  error in `persistence.ts` at the `setElection` / `setElectionEvent`
  dispatch, plus a runtime error when the screen reads the field. Add
  the field to every bundled snapshot.
- New slice altogether (e.g. `votersSlice`) that StartScreen starts
  consuming → screen returns `null` / spinner / navigates away. Add
  the slice to `hydrateFromSnapshot` and seed it in `default.json`.
- New action creator API (e.g. `setElection` becoming
  `setElection({election, source})`) → TS error at the dispatch in
  `persistence.ts`.
- **A new required field on `IDecodedVoteContest`** → the booth keeps
  working and the tally breaks. This one is worth internalising because
  it defeats the usual instincts: it is not a TS error (the snapshots
  are JSON, not typed), the `validateBundledSnapshots` plugin does not
  catch it (it only checks the keypair ↔ ballot-style invariant), and
  the booth is unaffected because the portal's `resetBallotSelection`
  builds selections containing the field. Only the *persisted*
  selections are stale, so the failure appears at the far end of the
  pipeline as a red error on `/tally`:

  ```
  invalid decoded ballot JSON: missing field `is_decline_to_vote`
  ```

  That is sequent-core refusing to deserialise the tally input. Fix by
  backfilling the field across every bundled snapshot, in both
  `state.ballotSelections[*]` and
  `workbench.repairedCastVotes[*].selection`. Election-level
  decline-to-vote (#2687) is the case that established this;
  `setBallotSelectionBlankVote` gaining a required `candidateId`
  argument in the same wave is the shape to watch for next.

### G. Workbench-only debug affordances (`BoothSpike.tsx`)

To debug whether a click reaches a reducer, the workbench exposes the
production Redux store on `window.__store` and patches `store.dispatch`
to push every action into `window.__dispatchLog`. It also exposes
`window.__resetWorkbench()`, a convenience that clears the persisted
snapshot and reloads — the same effect as deleting the
`workbench:state:v1` key in DevTools → Application → Local Storage.
From the browser console or a Playwright `page.evaluate`:

```js
window.__store.getState().ballotSelections
window.__dispatchLog.map(x => x.type)
window.__resetWorkbench()       // wipe + reload
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

### H. Persistence (`app/src/persistence.ts`)

The workbench mirrors the entire voting-portal Redux state — plus the
workbench's own overlay state (section K) — to `localStorage` on every
dispatch and rehydrates from it on boot. The result: cast a ballot,
close the tab, reopen tomorrow — the ballot is still cast. The same
plumbing is reused by bundled snapshots (shipped in git) and named
checkpoints (saved by the operator) — the lift-irrelevant details of
that storage hierarchy and the inspector's snapshot UI are in
`WORKBENCH.md`.

For the lift, only two facts matter: **how state is rehydrated**, and
**what happens when voting-portal slices evolve**.

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

**Hook point in `BoothSpike.tsx`.** Boot performs, in order:
`loadPersistedSnapshot()` → `hydrateFromSnapshot(...)` (warm-boot from
the auto-resume slot, falling back to the bundled `default.json` on
first run) → `installPersistence(store)`. The persistence subscription
MUST be installed **after** hydration so that we never persist an
in-progress hydration. Hydration internally toggles a `suspendWrites`
flag so that the many small dispatches it issues don't each trigger a
full snapshot write — only the post-hydration state hits
`localStorage`.

**Schema versioning.** Snapshots tag themselves with `version: "v1"`
and the storage key carries the same suffix. When the persisted shape
becomes incompatible (e.g. voting-portal removes a slice we relied on),
bump the suffix in `PERSISTENCE_KEY` *and* the literal in
`PersistedSnapshot.version`. Old data is then silently ignored at boot
and the user gets a fresh fixture instead of a crash.

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
workbench's own pages — e.g. `/pipeline`, the inspector — can read the
same store), every workbench page now sees the same Redux state. The
booth screens themselves are unaffected — the layering matches
`voting-portal/src/index.tsx`, which also wraps Redux outside its
routes.

### I. Source code under `voting-portal/src/` — UNCHANGED
*outside* the portal source tree. If you ever feel tempted to edit a portal
file to make the workbench work:

1. **Stop.**
2. Re-read the "Why this document exists" section.
3. Try one of: `resolve.alias`, `define`, a new provider, a fixture seed,
   a substitute deep-import path. One of these almost always works.
4. If you genuinely cannot avoid a portal-source change, document it in
   section **L. Concessions: edits to production source** with the exact
   diff and the reason it was unavoidable. Reviews of refresh PRs will
   then verify that the concession is still needed.

   (Section L is the complete inventory of production source this branch
   modifies, and is *not* limited to the portal: L.1–L.3 cover the
   demo-mode-only edits to `useAddFakeCastVote` / `castBallotAction` in
   `ReviewScreen.tsx` and the additive `removeCastVotes` reducer on
   `castVotesSlice`; L.4 covers `sequent-core`'s wasm32 build
   enablement. The larger `velvet` / `strand` refactors are owned by the
   README's *Known gaps*.)

### J. Workbench-native chrome (`app/src/WorkbenchInspector.tsx`)

Workbench-owned UI under `/wb/...` — not lifted from voting-portal, not
from admin-portal. **Lift-relevant rules only** (full design narrative
in `WORKBENCH.md`):

- Workbench-native pages MUST NOT import or re-implement voting-portal
  screens. They may freely import portal Redux slices, selectors, and
  action creators — those are part of the same package the booth uses.
- CTAs that enter the booth MUST link to the production paths
  (`/tenant/:t/event/:e/...`), never to `/wb/...`. See section E.
- After section L.1 fixed the demo's election-id bucket, cast votes
  land where production puts them (`state.castVotes[electionId]`);
  the inspector reads from that single bin everywhere.

There are no lift canaries here: nothing under `/wb/...` resolves to a
portal screen, so portal changes can't break it directly. A portal
change CAN break the inspector indirectly via a renamed Redux slice or
action creator — those canaries already live in §F / §H.

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
  `setRepairedDecodedBigInts`, `setKeypair`, `replaceWorkbenchState`.

State fields:

- `voters`, `activeVoterId`, `castBy` — directory + attribution
  ledger. `activeVoterId` is the *currently-impersonated* voter,
  cleared automatically once their cast vote lands (see
  `installPersistence` cast-votes watcher: the watcher snapshots the
  pre-attribution `activeVoterId`, records `castBy[id] = activeBefore`,
  and then calls `setActiveVoter(null)` to retire the persona). The
  next visit to a voter's detail page therefore offers a fresh *Cast
  a ballot* CTA rather than silently re-impersonating the last voter.
- `repairedCastVotes` — per-cast-vote bridge data (plaintext selection
  snapshot, real election id, and `decodedBigInts: Record<contestId,
  decimalString>` filled in asynchronously by the decrypt bridge — see
  section M).
- `keypair: WorkbenchKeypair | null` — the single workbench-owned
  ElGamal keypair for the whole snapshot. `{pkB64, skB64}`,
  base64-no-pad, strand/borsh-serialised via
  `velvet-wasm::generate_keypair`. One keypair per snapshot (not per
  ballot style) because production runs a single trustee ceremony per
  election and every ballot style in that election ends up stamped
  with the same `public_key`; sharing one key here is what lets a
  contest tally span ballot styles. `setKeypair(kp)` is
  **first-call-wins** so a stray re-seed cannot orphan already-
  captured cast votes encrypted under the old pk. Full lifecycle in
  section M.
- `ballotStylePool?: Record<electionId, BallotStyleRow[]>` — optional
  out-of-band pool of *all* ballot styles available per election. The
  portal's `ballotStyles` slice is keyed by `election_id` and only
  ever holds **one** row per election at a time (the one the current
  voter is eligible for), so the workbench keeps the full set here
  and swaps the slice on active-voter change (see "Eligibility swap"
  below). Rows are stored as `unknown[]` so `workbenchStore.ts` does
  not have to import voting-portal types.
- `assignments?: Record<voterId, ballotStyleId[]>` — optional
  per-voter eligibility map. Intersected with the per-election
  entries of `ballotStylePool` to pick which BS to dispatch into the
  Redux slice. Both fields are populated via importers (snapshot,
  portal-style, velvet-election) and round-trip through
  `replaceWorkbenchState`; there are no dedicated mutations because
  assignments today are immutable once imported. Snapshots written
  before Phase 1 omit both fields, in which case the swap is a no-op
  and the slice retains whatever was hydrated.

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
running set of seen ids, and for each newly-observed cast vote it:

1. Snapshots `activeVoterId` *before* dispatching attribution.
2. Calls `attributeCastVote(id)`, which (when an active voter was set)
   records `castBy[id] = activeVoterId`.
3. Calls `tryCaptureRepairedCastVote(...)` to snapshot the plaintext
   selection from `state.ballotSelections` (section L bridge).
4. If an active voter was in effect, calls `setActiveVoter(null)` so
   the persona retires once their ballot is cast.

The `VoterDetailPage` reads this attribution to display each voter's
cast-vote history. The ledger is the workbench's substitute for the
production `voter_id_string` field, which stays `null` in the portal
store because we don't touch portal source.

The watcher's hydration branch (`suspendWrites`) seeds
`seenCastVoteIds` from the restored state without firing attribution
or active-voter retirement — those would be replays of events that
have already been recorded in the snapshot.

**Eligibility swap (`activeVoterId` → portal `ballotStyles` slice).**
`installPersistence` also subscribes to mini-store changes and tracks
`lastActiveVoterId`. When `activeVoterId` transitions to a non-null
voter, it calls `applyEligibilitySwap(store, voterId)`, which iterates
`workbench.ballotStylePool` keys and, for each `electionId`, calls
`selectBallotStyleForVoter(voterId, electionId)` (matches the voter's
`assignments` against the pool) and dispatches `setBallotStyle(row)`
into the portal slice. Transitions *to* `null` are deliberately skipped
so that the post-cast booth render (which clears `activeVoterId` via
the attribution watcher) keeps showing the just-voted ballot style
rather than blanking it.

This is the workbench's substitute for the production flow where the
backend hands the booth exactly one eligible ballot style per session
based on the authenticated voter. The lift never sees that backend, so
the workbench picks the row out of its own pool on impersonation
change. The swap fires from the same `subscribeWorkbench` listener
that drives snapshot writes, so a voter change is one mini-store
mutation → one swap → one snapshot write.

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

### L. Concessions: edits to production source

This section is the **complete inventory of production source this
branch modifies**. It is not limited to `voting-portal/src/` — anything
under `packages/` that production also compiles belongs here, because
the reason for the inventory is the same in every case: when we catch up
to a later upstream version, each entry is a decision that has to be
re-made, and an undocumented edit is one nobody knows to re-apply or
retire.

Three groups, in descending order of how much scrutiny a refresh needs:

| Group | Where | Documented in |
|---|---|---|
| Demo-path concessions | `voting-portal/src/` | L.1–L.3 below |
| Build enablement | `packages/sequent-core/` | L.4 below |
| Structural / obsolete-removal | `packages/velvet/`, `packages/strand/` | README *Known gaps* |

The last group is deliberately **not** written out file-by-file here.
`velvet` (tally logic extracted into `velvet-core`) and `strand`
(obsolete openssl/FIPS backends removed for wasm32) are large, coherent
refactors rather than surgical concessions, and their catch-up cost is a
design question — forward-port or land upstream — not a diff to re-apply.
The README's *Known gaps* section owns that discussion.

Whatever the group, the live picture is on the workbench's own
**Diagnostics page** (`/wb` → Diagnostics → *Shared-source drift*),
which diffs each tracked subtree against the merge-base with
`origin/main`. If something shows up there that is not described in this
section or in *Known gaps*, that is the bug — either the edit shouldn't
exist, or this document is stale.

#### L.1–L.3: `voting-portal/src/`

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

#### L.3 `castVotesSlice` — add a `removeCastVotes` reducer

The portal's `castVotesSlice` originally exported one mutating action,
`addCastVotes(ids[])`, which dedupes by `id` within each per-election
bucket. There is no production code path that removes a cast vote from
the slice: in real life a re-cast is appended (the backend keeps the
full history up to `num_allowed_revotes`) and tallying picks the latest.

The workbench needs the *opposite* behaviour: when an operator re-casts
as the same voter persona, the previous cast vote should disappear
from the slice so the inline tally sees exactly one input per voter.
Stacking would inflate every demo tally by the number of times the
operator clicked "Recast" while exploring a fixture, which makes the
workbench useless for sanity-checking results.

The accepted edit adds a second reducer to the slice:

```ts
removeCastVotes(state, action: PayloadAction<string[]>) {
    // filter ids from each bucket, prune empty buckets
}
```

and exports it alongside `addCastVotes`. The workbench wires it from
`persistence.ts` via `supersedePriorCastVotes(store, voterId,
electionId, newCastVoteId)` — invoked from the cast-votes watcher
just before `attributeCastVote`. The helper inspects the workbench
overlay's `castBy` map to find prior cast votes by the same persona in
the same election, dispatches `removeCastVotes(priorIds)` on the portal
store, and drops the corresponding overlay rows via
`dropCastVoteOverlay(priorIds)`. Net effect: a re-cast as voter V in
election E replaces V's prior cast in both the slice and the overlay,
in one tick.

**Why this was accepted:**

- The new reducer is **additive**. `addCastVotes` is unchanged; no
  production caller of the slice references `removeCastVotes`, so
  production behaviour is byte-identical.
- The semantics are workbench-specific (operator convenience for
  multi-cast exploration), but they live entirely behind a slice
  action; the *slice* doesn't know about workbench personas, it just
  exposes a generic remove primitive. Any future portal caller could
  use it for an unrelated reason without conflict.
- The alternative — keeping the slice pristine and physically
  rewriting it from the workbench via `replaceWorkbenchState`-style
  imperative dispatch — would require either (a) a private import
  of the slice's internals (forbidden) or (b) re-dispatching the full
  remaining bucket through `addCastVotes` after wiping, which is more
  intrusive than a single targeted reducer.
- The diff is ~10 lines in `castVotesSlice.ts` plus the export. The
  workbench cap-and-counter machinery that briefly sat on top of this
  was removed in a follow-up; only the reducer remains.

**Refresh-PR guardrail.** If voting-portal refactors `castVotesSlice`
(e.g. moves it to RTK Query, changes the bucketing key away from
`election_id`, or migrates ids into a Set), the refresh must keep a
mutation that removes a given set of cast-vote ids from the per-
election buckets and is exported by name `removeCastVotes`. Reviewers
should reject a refresh that drops the reducer without providing an
equivalent.

#### L.4 `sequent-core` — wasm32 build enablement

Two commits (`b659c4c83f`, `399433741b`) modify `packages/sequent-core`.
Unlike L.1–L.3 these are **not** demo-path edits and touch no logic, no
behaviour and no wire format — nothing under `ballot_codec`, nothing in
the encoding rules. They exist so the crate compiles to
`wasm32-unknown-unknown` and so `wasm-pack build --features=wasm` works
against sequent-core *standalone*, without velvet-core being in the
workspace graph to supply the wasm32 bits. Two files:

**`Cargo.toml`**

1. `ed25519-dalek` `=3.0.0-pre.1` → `=3.0.0-pre.7` and `curve25519-dalek`
   `=5.0.0-pre.1` → `=5.0.0-pre.6`, matching this branch's `strand`.
   Without this the workspace does not resolve at all (`failed to select
   a version for curve25519-dalek`).
2. `ring` (with `wasm32_unknown_unknown_js`) replaced by
   `getrandom 0.2` with the `js` feature, and the target cfg narrowed
   from `cfg(target_arch = "wasm32")` to
   `cfg(all(target_arch = "wasm32", target_os = "unknown"))`. `ring`
   does not build for that target; getrandom-0.2/js is the version
   curve25519-dalek transitively requires, and mirrors what velvet-core
   already declares.
3. `dep:web-sys` added to the `wasm` feature. `src/util/console_log.rs`
   uses `::web_sys` under `cfg(feature = "wasm")`, but the dependency was
   only being pulled in by `wasmtest`.

**`src/wasm/mod.rs`**

4. `pub mod areas;` and `pub mod wasm;` move from `#[cfg(feature =
   "wasmtest")]` to `#[cfg(feature = "wasm")]`. This is what makes
   `yarn build:sequent-core` (which runs `--features=wasm,default_features`)
   emit the area-tree and locale exports the lifted booth calls — see
   §A7.

**Why this was accepted.**

- **The shipped artifact is unchanged.** `wasmtest` is defined as
  `["wasm", ...]`, so it already implies `wasm`, and the production tgz
  is built by `.devcontainer/scripts/build-sequent-core.sh` with
  `--features=wasmtest,default_features`. Those modules were compiled
  into production builds before and after; only the `wasm`-only path
  (what the workbench uses) is widened.
- Items 2 and 3 are strictly additive on non-wasm32 targets: the `ring`
  swap sits behind a wasm32 cfg that production native builds never
  evaluate, and a feature gaining a dependency it already used cannot
  break an existing consumer.
- Item 1 is a version alignment forced by strand, not a preference. It
  disappears the moment strand and sequent-core agree upstream.

**Refresh-PR guardrail.** These are the entries most likely to become
*unnecessary* rather than to break — upstream may adopt the same wasm32
enablement independently, at which point the right move is to drop our
version rather than re-apply it. On each refresh, check whether upstream
already pins the same dalek versions, already declares the wasm32
getrandom dep, and already gates `areas`/`wasm` on `wasm`. Delete the
matching item here for each one that has landed. If nothing is left,
delete L.4 — a concession that no longer differs from upstream should
not be carried as if it did.

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

The workbench's local wasm package exposes wasm-bindgen functions on
top of `sequent-core` (in-tree source) and `strand`. The four
functions that participate in the encrypt → decrypt → tally loop are:

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
  by tests and by future round-trip checks: the value it produces
  must match what `decrypt_ballot_content` recovers from the same
  `Contest` + `DecodedVoteContest` pair.
- `tally_plaintext_ballots(...)` — the lower-level tally entry used
  by `TallyPage.tsx` (the centralised tally execution point) once
  decrypted `BigUint`s have been collected. See §M.4 for how the
  workbench feeds it.

The package also re-exports a handful of `get_sample_*` JSON helpers
used only by `/pipeline` (the ballot-pipeline sandbox) and by ad-hoc
REPL experiments; they are workbench-internal and have no canary.

**Canonical-surface rule.** `velvet-wasm` is reserved for operations
that have no canonical `sequent-core` wasm-bindgen counterpart — at
the time of writing those are exactly the four listed above plus the
`get_sample_*` fixtures. **Any pipeline stage whose underlying logic
already lives behind a `#[wasm_bindgen]` export in
`sequent-core/src/wasm/` MUST call that export directly**, not a
re-implementation in `velvet-wasm`. The `/pipeline` encrypt stage is
the working precedent: `tally.ts` imports `encrypt_decoded_contest_js`
+ `to_hashable_ballot_js` from `"sequent-core"` (the same chain the
lifted booth's Cast button traverses), so workbench and booth share
one encrypt implementation and a fidelity-check edit to
`sequent-core/src/wasm/*.rs` shows up in `/pipeline` after
`yarn build:sequent-core`. A previous hand-rolled
`encrypt_decoded_vote_contest` in `velvet-wasm/src/lib.rs` was
removed for violating this rule (it duplicated the canonical encrypt
path with a slightly different envelope shape, defeating the point
of having a workbench in the first place).

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
  on the first byte) and the tally outcome on `ContestDetailPage` is
  garbage. Re-sync the decrypt post-processing with whatever the new
  convention is.

#### M.2 Keypair bundling and validator invariant

The keypair is **bundled into the snapshot** rather than generated at
boot. The shipping `default.json` carries
`workbench.keypair = {pkB64, skB64}` whose `pkB64` is written into
*every* ballot style's `ballot_eml.public_key.public_key`. The
`validateBundledSnapshots` Vite plugin enforces this consistency at
build start (see `app/vite.config.ts`):

1. `workbench.keypair` must be present with string `pkB64` and
   `skB64` halves.
2. Every `state.ballotStyles[*].ballot_eml.public_key.public_key`
   must equal `workbench.keypair.pkB64`.

Full workbench-side lifecycle (boot, per-id generation, persistence,
reset) lives in `WORKBENCH.md`.

**Canary if portal changes:**

- Portal starts validating `ballot_eml.public_key` against a server-
  side allowlist → the workbench's bundled pk gets rejected at vote
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
entry is simply left empty; consumers (e.g. `ContestDetailPage`)
then show the cast vote as not contributing to the tally rather
than asserting equality.

The decrypt does **not** re-run on hydrate. Re-decrypting a cast
vote at boot would risk doing so under a different keypair if the
operator wiped state in between, and the `decodedBigInts` are
already in the snapshot. Hydration just rehydrates whatever value
was last written.

#### M.4 Tally consumption

The decrypted BigUints in `repairedCastVotes[*].decodedBigInts` are
fed to `TallyPage.tsx` (the centralised tally execution point) via
the "Open in tally" hand-off from the contest detail page or by
navigating to `/tally` directly. There is no re-encrypt-then-decrypt
trip through wasm at tally time; the BigUints flow straight from the
decrypt bridge into the tally code path, exactly the way a production
trustee would feed decrypted plaintexts in. Policy overlays are
applied at tally execution time by `TallyPage.tsx` (re-encodes and
re-decodes with `applyPolicyOverlayToContest` before tallying).

**End-to-end canary.** Cast a Blue vote on the bundled fixture
(plurality-at-large, two candidates, `max_votes=1`); expected
`decodedBigInts[<contestId>] === "4"` (bases `[2,2,2]`, choices
`[0,0,1]`, mixed-radix LSB), tally reports `Blue: 100% (1), Red: 0%
(0), valid=1, invalid=0`. If the BigUint shows up as `1025`, the
length-prefix unwrap in `decrypt_ballot_content` regressed (see
M.1).

---


## Refresh procedure (when voting-portal evolves)

Run when voting-portal has changed and the workbench booth view is
broken or you want to validate fidelity.

1. **Smoke run.** `corepack yarn workspace "@sequentech/workbench-app" dev`
   and visit `http://localhost:5173/` (which redirects to `/wb`, the
   inspector). From a ballot style or voter detail page, click the
   *Start voting* CTA to exercise the booth at
   `/tenant/:t/event/:e/election/:el/start`. Check the browser console.
2. **Categorize the first error** using the canary table below:

   | Error pattern | Likely category | Section to revisit |
   |---------------|-----------------|--------------------|
   | `Failed to resolve import "<name>"` | New transitive dep | B (package.json) |
   | `Failed to resolve import "@root/..."` or `"@sequentech/..."` | Workspace path | A (vite.config) |
   | `<X> must be used within a <Y>Provider` | New required provider | D (BoothSpike providers) |
   | `Cannot read property of undefined` reading a settings field, **or** silent UI degradation with no console error | New settings key | C (global-settings.json) |
   | `<Router> inside another <Router>` | Router nesting | E (routing) |
   | `useSubmit must be used within a data router` | Legacy router used | E (use `createBrowserRouter`) |
   | `No routes matched location "/tenant/..."` | Path mirror is stale | E (extend `boothChildren`) |
   | Screen renders a spinner / self-redirects | Missing or mis-shaped fixture | F (fixtures) |
   | TS error in `fixtures/*` about a slice payload field | Portal slice type evolved | F (fixtures) |
   | `expected magic word 00 61 73 6d, found 3c 21 2d 2d` | Wasm pkg pre-bundled by Vite | A6 (`optimizeDeps.exclude`) |
   | `Cannot read properties of undefined (reading 'check_voting_...')` etc. coming from `sequent-core.js` | Wasm not initialized | D (mount `<WasmWrapper>`) |
   | `process is not defined` or `process.env.X` undefined | env var | A (`define` in vite.config) |
   | `invalid decoded ballot JSON: missing field ...` on `/tally`, but the booth still votes fine | Bundled snapshots predate a new required `IDecodedVoteContest` field | F (backfill every snapshot) |
   | `Failed to resolve import "sequent-core"` on a fresh clone | `packages/sequent-core/pkg` absent and the alias guard removed | A7 (`fs.existsSync` guard) |
   | Tally numbers look plausible but disagree with production | velvet-core lagging upstream, or a percentage recomputed instead of forwarded | README *Known gaps*; `LIFTING-TALLY.md` |

3. **Fix the smallest possible thing**, restart the dev server, and re-test.
4. **Update this document.** If you added/changed an adaptation, edit the
   relevant section so the next refresh starts from accurate state.
5. **Run the ballot pipeline sandbox too** (`http://localhost:5173/pipeline`).
   It uses `velvet-wasm` (the wasm-bindgen wrapper around `velvet-core`)
   directly and is unaffected by portal changes; if it breaks, the
   problem is in workbench glue or wasm-pack output, not the lift.

## Adaptations to add as we lift more screens

**Lifted screens so far** (full Vote-cast journey, plus the entry chooser):

- `ElectionSelectionScreen` at `tenant/:tenantId/event/:eventId/election-chooser`.
- `StartScreen`, `VotingScreen`, `ReviewScreen`, `ConfirmationScreen` at
  the portal's existing `tenant/.../election/:electionId/{start,vote,review,confirmation}`
  paths.

When extending past these, the following are the most likely next-step
categories of work (in roughly the order they will be needed):

1. **A real election public key in the ballot style fixture.** ✅ Done,
   and since superseded. The fixture briefly used
   `DEFAULT_PUBLIC_KEY_RISTRETTO_STR` from sequent-core, which made
   `encryptBallotSelection` produce real ciphertext but validated the
   *encrypt* path only — nobody holds that key's secret half. It now
   carries the workbench-owned keypair from `workbench.keypair`, so the
   whole encrypt → decrypt → decode → tally loop closes in the browser.
   See sections F and M.
2. **Initialize `ballotSelections` from the fixture, not only from
   StartScreen.** ✅ Done — the bundled `default.json` snapshot ships a
   pre-initialized `ballotSelections[electionId]` entry, so every URL
   is a valid entry point (hot reload on `/vote`, deep links). See
   section F.
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
7. **Extending the bundled snapshot.** As later screens consume more
   of the store (cast votes, audit data, etc.), `default.json` grows.
   The `validateBundledSnapshots` plugin keeps the snapshot-keypair ↔
   ballot-style invariants honest on every build. Prefer one snapshot
   per
   coherent scenario over many small ones. The authoring workflow
   (save a checkpoint, copy its JSON, paste it under
   `app/src/fixtures/snapshots/`) lives in `WORKBENCH.md`.

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

## Companion: `WORKBENCH.md`

Workbench-side design that lives *around* the lifted code — the
inspector at `/wb`, the snapshot / checkpoint / provenance model, the
mini-store's UI contract, the bundled-snapshot authoring workflow, and
parked workbench-only ideas — is documented in
[`WORKBENCH.md`](./WORKBENCH.md). When the two documents disagree
about a fact related to a voting-portal lift step, `LIFTING.md` wins.

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
