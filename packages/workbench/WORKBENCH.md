<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Workbench design notes

`LIFTING.md` is the replay procedure for re-hosting voting-portal in the
workbench. This file is its companion: it documents the **workbench-owned
chrome and machinery** that sits around the lifted code — the inspector,
the snapshot / checkpoint / provenance model, the workbench mini-store,
and the design rules that govern what may or may not live where.

When this document and `LIFTING.md` disagree about a fact related to a
voting-portal lift step, `LIFTING.md` wins. This is design narrative and
moves more freely; the lift procedure has to stay precise.

---

## Routing layout

The workbench mounts everything under one `createBrowserRouter` (data
router; required by the lifted portal — see `LIFTING.md` §E):

- `/` → `<Navigate to="/wb" replace />`. There is no separate landing
  page; the inspector at `/wb` is the home.
- `/wb/...` — workbench-owned UI living in `app/src/WorkbenchInspector.tsx`.
  Five children, all sharing `InspectorLayout`:
  - `/wb` (index) — working-copy overview (`SnapshotOverviewPage`).
  - `/wb/snapshot/:id` — bundled snapshot or named checkpoint detail.
  - `/wb/ballot-style/:id` — per-ballot-style detail (pk, sk, contests).
  - `/wb/contest/:id` — contest detail with inline tally.
  - `/wb/voter/:id` — voter detail with attribution + booth CTA.
- `/tally` — focused velvet-wasm playground (`App`), no Redux integration.
- `/tenant/:tenantId/event/:eventId/...` — the **production-mirror**
  booth subtree, mounted under `<BoothLayout />`. Paths and route shape
  are dictated by voting-portal and must not diverge (see `LIFTING.md`
  §E for why).

The split is deliberate: `/wb/...` is workbench-owned chrome we are free
to evolve; `/tenant/:t/event/:e/...` is the production-mirror surface
where we MUST NOT diverge.

The `Shell` nav has two links only: **Workbench** → `/wb`, **Raw-JSON
tally** → `/tally`. The `ReduxProvider` lives in `Shell` (outside the
booth subtree) so the workbench's own pages read the same store the
booth writes to — the same layering as `voting-portal/src/index.tsx`.

---

## Inspector (`app/src/WorkbenchInspector.tsx`)

Everything under `/wb/...` is **workbench-owned UI** — not lifted from
voting-portal, not from admin-portal. The decision is in `LIFTING.md`'s
*do-not-lift* rule: admin-portal is explicitly out of scope. Instead the
workbench ships its own minimal inspector to introspect the scenario and
the snapshot graph.

The entire surface lives in one file (`WorkbenchInspector.tsx`) and is
mounted by `main.tsx` as `<InspectorLayout>` with five child routes.
The layout is a fixed two-pane chrome:

```
┌───────────────┬──────────────────────────────────┐
│  Tree rail    │  <Outlet />                      │
│  (left, ~20%) │  one of five detail pages        │
│               │                                  │
│  Snapshots    │                                  │
│  ─────────    │                                  │
│  Tenants      │                                  │
│  ─────────    │                                  │
│  Voters       │                                  │
└───────────────┴──────────────────────────────────┘
```

### Tree rail (left pane)

Three sections, always visible. The locked design for each:

- **Snapshots.** A provenance forest (`buildProvenanceForest`):
  bundled snapshots are roots, checkpoints attach under their
  `parentId`. Each node links to `/wb/snapshot/<encoded id>`; the
  current working-copy parent is highlighted. The working copy itself
  is *not* a tree node — it appears instead as the pinned top row of
  the snapshot index table at `/wb` (the index route).
- **Tenants.** Derived from `state.electionEvent` and `state.elections`
  by grouping events under their `tenant_id`. Each leaf is a ballot
  style; clicking opens `/wb/ballot-style/<id>` directly (no
  intermediate drilldown pages). Tenants and events are shown as
  static labels in the rail; they have no detail pages because the
  workbench has nothing interesting to surface for them.
- **Voters.** The workbench mini-store's voter directory, each row a
  link to `/wb/voter/<id>`.

### Detail pages (right pane)

All five are exported from `WorkbenchInspector.tsx`:

- `SnapshotOverviewPage` at `/wb` (index). A unified snapshot index
  table with one row per snapshot: the working copy first (pinned,
  tinted, with a *Save…* action that prompts for a checkpoint name),
  then bundled snapshots (alphabetical), then checkpoints (newest
  `savedAt` first). Columns: name, kind, forked-from (NavLink to the
  parent row's snapshot detail page), saved-at, voters, elections,
  ballot styles, cast votes, action. Bundled and checkpoint rows
  carry a *Load* button (bundled → `hydrateFromSnapshot` with the
  bundled-id tag; checkpoint → `loadCheckpoint`); the working-copy
  row carries the *Save…* button. Counts come from `selectStateCounts`
  applied to each snapshot's `state` plus `snapshot.workbench?.voters`;
  the working-copy row uses scalar `useSelector`s (not object-returning)
  to avoid react-redux's referential-equality warning on every dispatch.
  Checkpoint rows live-update via `useCheckpointList`.

  Below the table sits an *Import JSON into working copy…* panel:
  paste a full `PersistedSnapshot` blob and it is hydrated straight
  into the live store via `hydrateFromSnapshot(store, parsed, null)`.
  The `null` `sourceId` makes the imported state a root — whatever
  `parentId` the source JSON carried is discarded. Use case: iterate
  on a hand-edited or externally-generated fixture without going
  through the bundle + rebuild cycle. To keep the imported state,
  click *Save…* on the working-copy row after importing; if not, it
  lives only in the auto-resume slot until the next change. (Same as
  Load on a checkpoint/bundled row, the import overlays onto the
  existing store rather than wiping it first, so imported state
  composes with whatever was there before.)
- `SnapshotDetailPage` at `/wb/snapshot/:id`. The `:id` is a tagged id
  (`bundled:<name>` or `checkpoint:<name>`, URL-encoded). Renders the
  same summary `<dl>` as the overview plus a *Load* button
  (bundled → `hydrateFromSnapshot` with the bundled-id tag; checkpoint
  → `loadCheckpoint`) and a collapsed *Bundled JSON* block. The export
  strips `parentId` so a copy-pasted snapshot becomes a root.
- `BallotStyleDetailPage` at `/wb/ballot-style/:id`. Resolves the
  ballot style by `id` (note: `state.ballotStyles` is keyed by
  `election_id`, not by ballot-style id, so this is an
  `Object.values(...).find(b => b?.id === bsId)` scan). Surfaces the
  parent election, the public key, the secret key (always visible —
  this is a demo keypair), a contest list (NavLinks to
  `/wb/contest/...`), and the raw `ballot_eml` JSON. A `pk/sk`
  mismatch (the stored `keypairs[bsId].pkB64` not matching
  `ballot_eml.public_key.public_key`) is surfaced as a warning row
  rather than silently swallowed.
- `ContestDetailPage` at `/wb/contest/:id`. Scans ballot styles for
  the first one containing the contest, then runs the inline tally
  against that ballot style and the live `decodedBigInts` (no
  synthetic ballot style — the real one is what production would
  decrypt against). A `useEffect` calls
  `runElectionTally(ballotStyle, decodedByCastVote)` from
  `electionTally.ts` whenever the decoded set changes, and renders
  the outcome for the focused contest in a `<pre>` block
  (running / no-data / error / result states).
- `VoterDetailPage` at `/wb/voter/:id`. Shows the voter, all their
  cast votes (joined via the `castBy` attribution ledger to
  `repairedCastVotes` and `state.castVotes`), and a *Cast a ballot in
  …* CTA per ballot style which calls `setActiveVoter(voter.id)` and
  navigates to the booth at the production path.

### Cast-vote bin honesty

After `LIFTING.md` §L.1 fixed the demo's election-id bucket, cast votes
land where production puts them (`state.castVotes[electionId]`); the
inspector reads from that single bin everywhere. Earlier revisions had
a dual-bin display (which had to show both `state.castVotes[electionId]`
and `state.castVotes[eventId]`); that is no longer needed.

### Rules

- Workbench-native pages MUST NOT import or re-implement voting-portal
  screens. They may freely import portal Redux slices, selectors, and
  action creators — those are part of the same package the booth uses.
- CTAs that enter the booth MUST link to the production paths
  (`/tenant/:t/event/:e/...`), never to `/wb/...`. See `LIFTING.md` §E.
- These pages own their styling inline. There is no design system to
  match — admin-portal is out of scope by policy, and matching the
  booth's MUI theme would imply that these are booth screens, which
  they are not.
- New detail pages should be added as inspector children
  (`/wb/<kind>/:id`), reusing `InspectorLayout`. There is no longer a
  workbench-native drilldown (tenant → event → election) — that
  hierarchy is collapsed into the rail.

---

## Snapshots, checkpoints and the provenance forest

The workbench mirrors the entire voting-portal Redux state — plus the
workbench's own overlay state (next section) — to `localStorage` on
every dispatch and rehydrates from it on boot. The result: cast a
ballot, close the tab, reopen tomorrow — the ballot is still cast.

The inspector's snapshot UI sits on top of this. There are three
storage tiers, all sharing the same JSON shape (`PersistedSnapshot =
{ version: "v1", state: RootState, workbench?: WorkbenchExtraState,
parentId?: string | null }`):

| Tier | Trigger | Storage key | Lifetime | Mutability |
|---|---|---|---|---|
| Auto-resume slot | Every Redux or workbench-overlay dispatch | `localStorage["workbench:state:v1"]` | Until reset / wiped | Constantly overwritten |
| Named checkpoint | Operator clicks *Save current state as checkpoint…* on `/wb` | `localStorage["workbench:checkpoint:v1:<name>"]` (plus index at `workbench:checkpoints:v1`) | Until deleted | Frozen at save time |
| Bundled snapshot | Shipped in git | `app/src/fixtures/snapshots/*.json` | Forever | Read-only at runtime |

All three go through the same `hydrateFromSnapshot` /
`PersistedSnapshot` plumbing — only the storage location differs.
Bundled snapshots are imported at build time via `import.meta.glob`
into a static `BUNDLED_SNAPSHOTS` dictionary
(`fixtures/bundledSnapshots.ts`), so the runtime never touches the
filesystem.

### Provenance forest

Snapshots form a forest:

- Bundled snapshots are roots (`parentId === null` in their on-disk
  form, conventionally stripped on export so a copy-pasted snapshot
  is automatically a root).
- Checkpoints carry a `parentId` recording the snapshot they were
  forked from — either a bundled root (`"bundled:<name>"`) or another
  checkpoint (`"checkpoint:<name>"`).
- The auto-resume slot is *not* a node; it is the working copy and
  inherits its `parentId` from whatever was most recently loaded.

This is tracked at runtime via a module-level `currentParentId` in
`persistence.ts`. `hydrateFromSnapshot(store, snapshot, sourceId?)`
updates it: if `sourceId` is supplied (bundled load → `bundledId(name)`,
checkpoint load → `checkpointId(name)`) we adopt that as the new
parent; if omitted (warm boot recovering from the auto-resume slot)
we keep the snapshot's own `parentId`. Every subsequent `writeSnapshot()`
stamps the working copy with the current value, and `saveCheckpoint`
records it on the frozen entry so the inspector tree rail can draw the
lineage.

`getCurrentParentId()` exposes it to UI; `bundledId(name)` /
`checkpointId(name)` produce the tagged ids used everywhere (URL
params, `parentId` fields, tree-rail keys). Reserving the `bundled:` /
`checkpoint:` namespaces keeps a single id space for the two snapshot
kinds without ambiguity.

### Named-checkpoint semantics

- `saveCheckpoint(store, name)` writes `store.getState()` under
  `workbench:checkpoint:v1:<name>` and adds/refreshes the entry in
  the sorted index. Names are normalized to letters/digits/`._- `
  with a 64-char cap (`normalizeCheckpointName`); illegal input
  throws so the UI can surface a precise message.
- `loadCheckpoint(store, name)` dispatches the snapshot through the
  same `hydrateFromSnapshot` used at boot, which means the
  auto-resume slot gets overwritten as a side-effect. The UI follows
  up with a `location.reload()` to drop any in-memory derived state
  (Apollo cache, mounted screens' local `useState`) so the boot path
  replays hydration cleanly.
- `deleteCheckpoint(name)` removes both the snapshot key and its
  entry in the index.
- Saving does NOT pause the auto-resume slot. The two tiers are
  independent: saving a checkpoint is purely additive; the
  auto-resume slot keeps tracking every dispatch.

### Boot sequence in `BoothSpike.tsx`

Module-eval order matters:

1. `loadPersistedSnapshot()` — reads `localStorage["workbench:state:v1"]`,
   returns `null` on first run, schema mismatch, or parse failure.
2. If a snapshot exists → `hydrateFromSnapshot(store, persisted)`
   (no `sourceId`; provenance is recovered from the snapshot's own
   `parentId`). Else →
   `hydrateFromSnapshot(store, loadBundledSnapshot("default"), bundledId("default"))`
   (first boot: the working copy is born as a fork of
   `bundled:default`). If the default snapshot is also missing —
   which the Vite build pipeline forbids — we surface a console
   error rather than booting into an empty store.
3. `installPersistence(store)` — subscribes to both the Redux store
   and the workbench mini-store; **after** step 2, so we never
   persist an in-progress hydration.

Hydration internally toggles a `suspendWrites` flag so that the many
small dispatches it issues don't each trigger a full snapshot write —
only the post-hydration state hits `localStorage`.

### Schema versioning

Snapshots tag themselves with `version: "v1"` and the storage key
carries the same suffix. When the persisted shape becomes incompatible
(e.g. voting-portal removes a slice we relied on), bump the suffix in
`PERSISTENCE_KEY` *and* the literal in `PersistedSnapshot.version`.
Old data is then silently ignored at boot and the user gets a fresh
fixture instead of a crash.

### Reset path

From the browser console: `__resetWorkbench()` (a global installed
alongside `__store` and `__dispatchLog` in `BoothSpike.tsx`). It calls
`clearPersistedSnapshot()` and reloads the page; on next boot,
`loadPersistedSnapshot()` returns `null` and the `bundled:default`
snapshot re-seeds. There is intentionally no nav-bar Reset button:
the inspector's *Load* action on `/wb/snapshot/...` is the in-app
equivalent (overwrite the working copy from a known good state), and
a one-click *wipe-and-reload* in the chrome was too easy to hit by
accident.

---

## Bundled-snapshot authoring workflow

Do not hand-edit `default.json` blindly. The Vite plugin
`validateBundledSnapshots()` (see `app/vite.config.ts`) runs at
build-start and rejects snapshots whose `state.ballotStyles[*].id` has
no matching `workbench.keypairs[id]` entry, or whose
`ballot_eml.public_key.public_key` does not match the stored `pkB64`.

The shortest path to a new scenario is:

1. Boot the workbench fresh, mutate state via the booth and the
   inspector until it looks right.
2. Click *Save current state as checkpoint…* on `/wb`.
3. Visit the checkpoint's `/wb/snapshot/<id>` page and copy its
   bundled JSON (the *Copy JSON* button strips `parentId` so the
   export becomes a root).
4. Paste it under `app/src/fixtures/snapshots/<name>.json` (with a
   `.json.license` sidecar).
5. Restart Vite. The validator will refuse to start the dev server
   if anything is inconsistent.

Prefer one bundled snapshot per coherent scenario over many small
ones — the tree rail lists them as siblings, and switching scenarios
is a one-click *Load* on `/wb/snapshot/<id>`.

---

## Workbench overlay state (`app/src/workbenchStore.ts`)

`LIFTING.md` §K describes the overlay's contract from the lift
perspective (why we don't add a slice to the portal store, how the
overlay is persisted alongside Redux state, the cast-votes watcher
canaries). This section is the workbench-side detail.

The mini-store keeps the operator-facing scenario data that has no
counterpart in voting-portal's Redux store: voter directory,
currently-impersonated voter, cast-vote → voter attribution ledger,
plaintext-selection bridge, and per-ballot-style keypairs.

It is a tiny `useSyncExternalStore`-based subscription, separate from
Redux, with named mutations only:
`addVoter`, `removeVoter`, `setActiveVoter`, `attributeCastVote`,
`captureRepairedCastVote`, `setRepairedDecodedBigInts`, `setKeypair`,
`replaceWorkbenchState`.

### State fields

- `voters`, `activeVoterId`, `castBy` — directory + attribution
  ledger. `activeVoterId` is the *currently-impersonated* voter,
  cleared automatically once their cast vote lands.
- `repairedCastVotes` — per-cast-vote bridge data (plaintext
  selection snapshot, real election id, and `decodedBigInts:
  Record<contestId, decimalString>` filled in asynchronously by the
  decrypt bridge — see `LIFTING.md` §M.3).
- `keypairs: Record<string, WorkbenchKeypair>` — workbench-owned
  ElGamal keypairs, keyed by ballot-style id. Each entry is
  `{pkB64, skB64}`, base64-no-pad. Per-ballot-style rather than
  global because every ballot style stamps its own `public_key` into
  `ballot_eml`. `setKeypair(bsId, kp)` is **first-call-wins per id**
  so a stray re-seed cannot orphan already-captured cast votes
  encrypted under the old pk.

### Auto-clear active-voter lifecycle

The cast-votes watcher in `installPersistence` retires the
impersonated persona once a cast vote attributed to them lands:

1. Snapshot `activeVoterId` *before* dispatching attribution.
2. Call `attributeCastVote(id)`, which (when an active voter was set)
   records `castBy[id] = activeVoterId`.
3. Call `tryCaptureRepairedCastVote(...)` to snapshot the plaintext
   selection from `state.ballotSelections`.
4. If an active voter was in effect, call `setActiveVoter(null)`.

The next visit to a voter's detail page therefore offers a fresh
*Cast a ballot* CTA rather than silently re-impersonating the last
voter.

The watcher's hydration branch (`suspendWrites`) seeds
`seenCastVoteIds` from the restored state without firing attribution
or active-voter retirement — those would be replays of events
already recorded in the snapshot.

---

## Future work (parked)

Not lift-related — workbench-only ideas, parked so they don't get
lost.

**Richer per-record introspection.** Each detail page already
collapses its record's raw JSON (e.g. `ballot_eml` on the ballot-style
page, the bundled export on the snapshot page). When two more record
types land (the encoded ballot per cast vote, and the tally result),
introduce a single recursive collapsible JSON tree component and
reuse it across pages, rather than growing each page's own `<pre>`
blocks. Anywhere we render a bare id today (cast-vote id, voter id,
ballot-style id), wrap it as a NavLink into the right `/wb/<kind>/:id`
page so the inspector is always one click away.

When to build: best **after** the encoded-ballot and tally-result
record types are stable enough to type — building it earlier risks
designing around the wrong schema.
