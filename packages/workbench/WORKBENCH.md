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
  - `/wb/contest/:id` — contest detail: decoded selections, the
    Policy-overrides panel, and hand-offs (*Open in tally*, *Open in
    ballot pipeline*) — tally execution itself is centralised on
    `/tally`.
  - `/wb/voter/:id` — voter detail with attribution + booth CTA.
- `/pipeline` — single-contest **ballot pipeline** playground
  (`BallotPipeline`): walks one `DecodedVoteContest` through the full
  encode → encrypt → decrypt → decode → tally chain, each stage a
  textarea + button. No Redux integration; runs entirely against
  `velvet-wasm`. See *Ballot pipeline page* below.
- `/tally` — the **tally page** (`TallyPage`); all tally execution is
  centralised here (see *Tally page* below).
- `/diagnostics` — build status, shared-source drift, booth
  sequent-core provenance (see *Diagnostics page* below).
- `/tenant/:tenantId/event/:eventId/...` — the **production-mirror**
  booth subtree, mounted under `<BoothLayout />`. Paths and route shape
  are dictated by voting-portal and must not diverge (see `LIFTING.md`
  §E for why).

The split is deliberate: `/wb/...`, `/pipeline`, `/tally`, and
`/diagnostics` are workbench-owned chrome we are free to evolve;
`/tenant/:t/event/:e/...` is the production-mirror surface where we MUST
NOT diverge.

The `Shell` nav has four links: **Snapshots** → `/wb`, **Ballot
pipeline** → `/pipeline`, **Tally** → `/tally`, **Diagnostics** →
`/diagnostics`. The
`ReduxProvider` lives in `Shell` (outside the booth subtree) so the
workbench's own pages read the same store the booth writes to — the
same layering as `voting-portal/src/index.tsx`.

---

## Inspector (`app/src/WorkbenchInspector.tsx`)

Everything under `/wb/...` is **workbench-owned UI** — not lifted from
voting-portal, not from admin-portal. The decision is in `LIFTING.md`'s
*do-not-lift* rule: admin-portal is explicitly out of scope. Instead the
workbench ships its own minimal inspector to introspect the scenario and
the snapshot graph.

The entire surface lives in one file (`WorkbenchInspector.tsx`).
`main.tsx` mounts `<InspectorLayout>` twice: once for `/wb`'s five child
routes, and once for the `/pipeline` + `/tally` + `/diagnostics` trio.
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
  carry a *Load* button — both go through `loadSnapshotViaReload`
  (bundled with the bundled-id tag, checkpoint with the checkpoint-id
  tag as `parentId`); the working-copy row carries the *Save…* button. Counts come from `selectStateCounts`
  applied to each snapshot's `state` plus `snapshot.workbench?.voters`;
  the working-copy row uses scalar `useSelector`s (not object-returning)
  to avoid react-redux's referential-equality warning on every dispatch.
  Checkpoint rows live-update via `useCheckpointList`.

  Below the table sit three import buttons (*Import snapshot JSON…*,
  *Import portal ballot style…*, *Import velvet election…* — see
  *Import paths* below): paste a blob and it is first materialized as
  a timestamped checkpoint (`materializeAsCheckpoint`), then loaded
  via `loadSnapshotViaReload(parsed, ckptId)` — the snapshot is
  written into the auto-resume slot and the page is reloaded, so the
  boot path hydrates a fresh empty store from it, with the checkpoint
  as provenance parent.
- `SnapshotDetailPage` at `/wb/snapshot/:id`. The `:id` is a tagged id
  (`bundled:<name>` or `checkpoint:<name>`, URL-encoded). Renders the
  same summary `<dl>` as the overview plus a *Load* button (uses
  `loadSnapshotViaReload` with `bundledId(name)` or
  `checkpointId(name)` as the `parentId`) and a collapsed *Bundled
  JSON* block. The export strips `parentId` so a copy-pasted snapshot
  becomes a root.
- `BallotStyleDetailPage` at `/wb/ballot-style/:id`. Resolves the
  ballot style by `id` (note: `state.ballotStyles` is keyed by
  `election_id`, not by ballot-style id, so this is an
  `Object.values(...).find(b => b?.id === bsId)` scan). Surfaces the
  parent election, the public key, the secret key (always visible —
  this is a demo keypair), a contest list (NavLinks to
  `/wb/contest/...`), and the raw `ballot_eml` JSON. A `pk/sk`
  mismatch (the snapshot-level `workbench.keypair.pkB64` not
  matching this BS's `ballot_eml.public_key.public_key`) is surfaced
  as a warning row rather than silently swallowed.
- `ContestDetailPage` at `/wb/contest/:id`. Scans ballot styles for
  the first one containing the contest, shows decoded selections,
  hosts the **Policy-overrides panel**
  (`ContestPolicyOverridesPanel` — ephemeral, per-tab, wiped by a
  full page load), and provides two hand-offs: "Open in tally"
  (navigates to `/tally` with the contest's decoded ballots as a
  seed) and "Open in ballot pipeline". All tally execution is
  centralised in `TallyPage.tsx` — contest detail pages do not run
  tallies inline.
- `VoterDetailPage` at `/wb/voter/:id`. Shows the voter, all their
  cast votes (joined via the `castBy` attribution ledger to
  `repairedCastVotes` and `state.castVotes`), and a *Cast a ballot in
  …* CTA per ballot style which calls `setActiveVoter(voter.id)` and
  navigates to the booth at the production path.

### Cast-vote bin honesty

After `LIFTING.md` §L.1 fixed the demo's election-id bucket, cast votes
land where production puts them (`state.castVotes[electionId]`); the
inspector reads from that single bin everywhere.

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
  (`/wb/<kind>/:id`), reusing `InspectorLayout`. The tenant → event →
  election hierarchy is collapsed into the rail; there are no
  drilldown pages.

---

## Diagnostics page (`/diagnostics`)

An inspector page whose job is to answer questions about the *workbench
itself* rather than about the election under test. All of its data is
computed at build time by the `workbenchBuildInfo` Vite plugin and
served through the `virtual:workbench-build-info` module; the plugin
watches the relevant trees plus `.git/HEAD` and invalidates the module
on change, so the page refreshes without a restart.

Three cards:

**Build status.** Per-wasm-artifact mtimes against their crate sources,
so "is the wasm I'm looking at older than the Rust I just edited" is
answerable without running a build. Also lists the workspace-internal
crates baked into each artifact, with the versions resolved from
`Cargo.lock`. This is the card that catches the version-skew trap in the
README's embedding-strategy section.

**Shared-source drift.** One collapsible block per tree the workbench
shares with production — `voting-portal/src/`, `ui-core/src/`,
`ui-essentials/src/`, `velvet/`, `strand/`, `sequent-core/` — each
diffing `HEAD` against the merge-base with `origin/main`. Every block
carries an `expectation` string describing what *should* be there, so an
undocumented change reads as undocumented rather than merely present.
The card also reports how many commits `origin/main` has that this
branch does not — the other drift axis.

Two things worth knowing when reading it:

- The baseline moves. Once main is merged the merge-base advances to
  the merged commit and each diff collapses to this branch's own edits.
  A suspiciously large diff right after a merge usually means the merge
  is not committed yet, not that drift exploded.
- Patches are inlined into the virtual module, so any diff over 200 KB
  keeps its stat and prints the `git diff` command instead of the body
  (`strand` is normally the only one that trips this).

Anything appearing here should be described in `LIFTING.md` §L (edits to
production source) or the README's *Known gaps* (accepted divergences).
If it is in neither, that is the bug — either the edit should not exist
or the docs are stale. The reasoning for tracking drift this way, rather
than as a static per-row risk rating, is in the README.

There is deliberately **no** tally-lift block: the tally visualization
is imported unmodified from `ui-essentials` (nothing is copied), so git
history is the whole drift story.

**Current workbench state.** The live snapshot + overlay as importable
JSON — paste it into *Import snapshot JSON…* on the snapshots page to
reproduce a situation elsewhere. Same shape as a bundled snapshot's
*Bundled JSON*, except `parentId` is preserved so the receiving side
keeps the provenance link.

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
| Named checkpoint | Operator clicks *Save…* on the working-copy row at `/wb` | `localStorage["workbench:checkpoint:v1:<name>"]` (plus index at `workbench:checkpoints:v1`) | Until deleted | Frozen at save time |
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
- `loadSnapshotViaReload(snapshot, parentId)` is the UI's Load /
  Import primitive. It writes the snapshot into the auto-resume slot
  with the given `parentId`, then calls `location.reload()`. The boot
  path then hydrates a fresh empty store from that slot, which gives
  Load and Import **wipe semantics** (the working copy ends up
  matching the source exactly, with no leftovers from before) without
  needing per-slice reset actions in voting-portal. The page reload
  is the wipe. `hydrateFromSnapshot` remains the lower-level overlay
  primitive used at boot (where the store is already empty so overlay
  == wipe) and by `saveCheckpoint`'s tail write.
- `loadCheckpoint(store, name)` (lower-level) reads a checkpoint and
  feeds it through `hydrateFromSnapshot` without reloading. The UI no
  longer uses this; it's kept as a primitive for tests and headless
  scripts.
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
build-start and rejects snapshots that lack a `workbench.keypair`,
or whose `ballot_eml.public_key.public_key` (on any ballot style) does
not match the snapshot's single `workbench.keypair.pkB64`.

The shortest path to a new scenario is:

1. Boot the workbench fresh, mutate state via the booth and the
   inspector until it looks right.
2. Click *Save…* on the working-copy row at `/wb`.
3. Visit the checkpoint's `/wb/snapshot/<id>` page and copy its
   bundled JSON (the *Copy JSON* button strips `parentId` so the
   export becomes a root).
4. Paste it under `app/src/fixtures/snapshots/<name>.json`. (No
   `.json.license` sidecar — the bundled snapshots carry none; if REUSE
   compliance is wanted here it should be a single
   `packages/workbench/app/src/fixtures/snapshots/**` annotation in
   `REUSE.toml` covering all of them, not per-file sidecars.)
5. Restart Vite. The validator will refuse to start the dev server
   if anything is inconsistent.

Prefer one bundled snapshot per coherent scenario over many small
ones — the tree rail lists them as siblings, and switching scenarios
is a one-click *Load* on `/wb/snapshot/<id>`.

### What the validator does *not* check

It only enforces the keypair ↔ ballot-style invariant. It does not
validate persisted ballot selections against sequent-core's current
`DecodedVoteContest` shape, and that shape evolves. When
election-level decline-to-vote (#2687) landed, every bundled snapshot
predated the new **required** `is_decline_to_vote` field, and the
symptom was not a build failure but a red error on `/tally`:

```
invalid decoded ballot JSON: missing field `is_decline_to_vote` at line 1 column 373
```

The booth path kept working, because the portal's `resetBallotSelection`
constructs selections with the field; only *persisted* selections were
stale. So when a snapshot loads and votes fine but the tally rejects its
ballots, suspect a field added to `DecodedVoteContest` since the
snapshot was authored, and backfill it across
`state.ballotSelections[*]` and `workbench.repairedCastVotes[*].selection`.

Hand-written tally input hits the same wall: the *Input ballots* pane on
`/tally` is deserialised by sequent-core, so a pasted
`DecodedVoteContest` must carry every currently-required field.

---

## Workbench overlay state (`app/src/workbenchStore.ts`)

`LIFTING.md` §K describes the overlay's contract from the lift
perspective (why we don't add a slice to the portal store, how the
overlay is persisted alongside Redux state, the cast-votes watcher
canaries). This section is the workbench-side detail.

The mini-store keeps the operator-facing scenario data that has no
counterpart in voting-portal's Redux store: voter directory,
currently-impersonated voter, cast-vote → voter attribution ledger,
plaintext-selection bridge, and the snapshot-wide ElGamal keypair.

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
- `keypair: WorkbenchKeypair | null` — the single workbench-owned
  ElGamal keypair for the whole snapshot. `{pkB64, skB64}`,
  base64-no-pad. One keypair per snapshot (not per ballot style)
  because production runs a single trustee ceremony per election and
  every ballot style in that election ends up stamped with the same
  `public_key`; sharing one key here is what lets a contest tally
  span ballot styles. `setKeypair(kp)` is **first-call-wins** so a
  stray re-seed cannot orphan already-captured cast votes encrypted
  under the old pk.
- `ballotStylePool?: Record<electionId, BallotStyleRow[]>` — full
  set of ballot styles available for each election. Optional; legacy
  snapshots omit it and behave exactly as before. Pool rows are
  shaped like the portal `ballotStyles` slice entries (id,
  election_id, election_event_id, tenant_id, area_id, ballot_eml,
  …) but are stored opaquely here — `workbenchStore.ts` does not
  import portal types and treats every row as `unknown`. The
  authoritative interpretation lives in `persistence.ts`, which
  dispatches `setBallotStyle(row)` against the portal slice.
- `assignments?: Record<voterId, ballotStyleId[]>` — per-voter
  eligibility map. Optional; voters with no entry see whatever the
  portal slice currently holds (legacy behaviour). Ids that don't
  match a current voter are silently dropped on round-trip so
  snapshots stay clean after voter deletions.

### Eligibility overlay (`ballotStylePool` + `assignments`)

The voting portal's `ballotStyles` slice is keyed by `election_id`,
which means at most **one** ballot style per election can be active
in Redux at any time — a hard constraint of the production data
model that the workbench can't relax without forking the booth. To
test multi-ballot-style scenarios (one election, several area-scoped
styles, different voters eligible for different ones) the workbench
holds the **full pool** out-of-band in `workbench.ballotStylePool`
and rewrites the portal slice every time the operator switches
voters.

The swap is driven from `installPersistence` in `persistence.ts`:

1. Subscribe to workbench changes alongside the existing snapshot
   writer.
2. Track the previous `activeVoterId`. Only fire on transitions
   *to* a non-null voter — clearing the active voter (post-cast
   retirement, hydration, manual reset) must leave the slice alone
   so the booth screen the voter just used can finish rendering.
3. For each election present in `ballotStylePool`, look up
   `selectBallotStyleForVoter(voterId, electionId)` and, if it
   returns a row, dispatch `setBallotStyle(row)` against the portal
   store. Elections the voter has no assignment for are skipped
   (their slice entry is left untouched).

When a snapshot is loaded, `hydrateFromSnapshot` rebuilds the slice
from the snapshot's own `state.ballotStyles` — the pool isn't
consulted during hydration, because the snapshot is already
self-consistent for its persisted `activeVoterId`. The swap
subscriber's transition guard prevents a spurious swap from the
hydration boundary.

### Importers (three flows)

The snapshot overview page offers three import buttons, each
producing a `PersistedSnapshot` that is first materialized as a
timestamped checkpoint (`materializeAsCheckpoint`) and then fed
through `loadSnapshotViaReload(snap, ckptId)` — wipe + reload, with
the checkpoint as the provenance parent:

1. **Import snapshot JSON** — paste a full
   `PersistedSnapshot` (the shape the *Bundled JSON* block on any
   snapshot detail page emits). Validated before load: a blob
   without a `workbench.keypair` (pkB64/skB64) is rejected with a
   message steering you to the ballot-style / velvet variants.
2. **Import portal ballot style** — paste a single portal
   `IBallotStyle` row (the shape returned by
   `select * from public.ballot_styles where id = …`, or by the
   admin portal's BS detail export). The importer generates the
   snapshot's keypair, stamps the new pk into `ballot_eml.public_key`,
   synthesizes minimal `elections` and `electionEvent` slice rows,
   and spawns a single voter named *voter* assigned to the style.
3. **Import velvet election** — paste a velvet `ElectionConfig`
   JSON (see `fixtures/velvet/sample-election-config.json` for a
   working example). The importer wraps every velvet `BallotStyle`
   into the portal slice-row shape (`ballot_eml = <velvet BS
   payload>`), generates **one** snapshot-wide keypair and stamps
   the same pk onto every ballot style, and spawns one voter per
   `TreeNodeArea` named `voter (<area-short-id>)`
   (TreeNodeArea has no `name` field). Each voter is assigned to
   every ballot style whose `area_id` matches their area, so
   switching voters in the sidebar lands you on a different style
   for the same election.

All three flows funnel through `assembleSnapshot` in
`app/src/import/importHelpers.ts`, which sets `activeVoterId` to
the first voter so the eligibility swap fires on the first booth
render rather than waiting for a manual switch.

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

## velvet-wasm package (`workbench/velvet-wasm/`)

Thin `wasm-bindgen` surface around `velvet-core` / `sequent-core`,
exposed to the workbench app as the npm dep `velvet-wasm` (declared as
`"file:../velvet-wasm/pkg"` in `app/package.json`). Exports the
minimum the workbench needs to run the booth crypto end-to-end in the
browser without a backend:

- `tally_plaintext_ballots(contest, ballots)` — decode + tally.
- `encode_ballot(contest, decoded)` / `decode_bigint_to_decoded_vote_contest(contest, bigint)`
  — invertible pair turning a `DecodedVoteContest` into its decimal
  `BigUint` encoding and back.
- `encrypt_decoded_vote_contest(contest, decoded, pk_b64)` /
  `decrypt_ballot_content(content, sk_b64, contest_id)` — invertible
  pair under a workbench-generated ElGamal keypair. `encrypt_…`
  returns a `{contests: ["<base64 HashableBallotContest>"]}` envelope
  shaped like the portal's `castVote.content`, so its output feeds
  straight back into `decrypt_…` for round-trip checks.
- `generate_keypair()` — fresh single-party Ristretto ElGamal keypair.
  Production never holds a single secret key (decryption happens in
  the threshold mixnet); the workbench does, so it can exercise the
  encrypt/decrypt path in-browser. See the comment block above
  `generate_keypair` in `lib.rs` for the full reasoning.
- `get_sample_contest_json` / `get_sample_decoded_vote_contest_json` /
  `get_sample_ballots_json` — in-tree fixtures sourced from
  `sequent_core::fixtures::ballot_codec`. Used by `/pipeline` to
  bootstrap the textareas before a real contest editor exists.

### Ballot pipeline page (`app/src/BallotPipeline.tsx`)

Mounted at `/pipeline`. Single-contest playground that walks a
selection through every transformation a ballot undergoes on its way
to the tally:

```
plaintext ──encode──▶ encoded BigUint ──encrypt──▶ ciphertext envelope
                                                          │
                                                       decrypt
                                                          ▼
decoded plaintext ◀──decode── decrypted BigUint (=encoded)
       │
     tally
       ▼
     ContestResult
```

Each stage is a textarea + a single button that reads the textarea
above (plus the Setup pane: contest JSON, pk/sk) and writes the
textarea below. Every intermediate value is editable, so operators
can tweak any stage and rerun only the downstream buttons — useful
for probing "why did this vote fail to tally?" with a real captured
`castVote.content` pasted into stage 3.

The page is intentionally per-contest: the encrypted envelope it
produces wraps a single `HashableBallotContest`, which is enough to
round-trip through `decrypt_ballot_content`. The tally step (6)
consumes an array of encoded `BigUint` strings — the "Seed tally"
button on stage 5 prefills it from the just-decrypted BigUint so the
chain ends with a real `ContestResult` even though the tally itself
operates on encoded plaintext rather than decoded selections.

### Building, and the yarn-classic dep-cache gotcha

`app/package.json` runs `build:wasm` on `predev` / `prebuild`, which
executes `app/scripts/prepare-velvet-wasm.mjs`: it wasm-pack-builds
`velvet-wasm/pkg/` **and then copies it into
`node_modules/velvet-wasm`**. The copy step exists because the app
consumes the build through the `"velvet-wasm": "file:../velvet-wasm/pkg"`
dependency, and **yarn classic (1.x) resolves `file:` deps by copying,
not symlinking** — without the sync, `node_modules/velvet-wasm/` would
hold a snapshot taken at install time and freshly built artifacts would
be invisible to Vite (symptom: a `SyntaxError: … does not provide an
export named '<new_export>'` page error after adding a wasm export).
With the prepare script in place, starting the dev server is
sufficient — no manual copy or re-install.

The hazard the sync guards against is worth keeping in mind if it is
ever refactored away. A *missing export* throws, so you find it immediately.
A change that only alters **behaviour** — a velvet-core tally fix, a
sequent-core or strand bump underneath it — produces no error at all:
the stale copy keeps serving the old logic and the workbench reports old
numbers with total confidence. That is the same hazard the README calls
the version-skew trap, seen from the JS side, and the reason the §M.4
canary is the check after any Rust change.

### Sister loop: editing `sequent-core` source

`velvet-wasm` is the workbench's *own* wasm-bindgen layer; the
lifted booth, by contrast, imports the `sequent-core` package
directly (locale helpers, area tree, `IBallot*` types). **By default
that import is aliased to `packages/sequent-core/pkg`** — the
wasm-pack output of the in-tree crate — which `predev` / `prebuild`
build via `app/scripts/prepare-sequent-core.mjs`. `pkg/` is
gitignored: it is produced, never committed. (The prebuilt tgz
unpacked under `packages/node_modules/sequent-core/` — the artifact
production ships — is opt-in; see below.)

That default exists because the tally half always compiles the crate
from source (Cargo path dep → velvet-wasm): pointing the booth at a
committed snapshot would let the two halves run different code with no
error — just wrong numbers — and would make local Rust edits invisible
to the booth.

- Editing pure Rust internals consumed only via `velvet-wasm`:
  `predev`/`prebuild` rebuild `velvet-wasm/pkg/` and Cargo's path-dep
  pulls the fresh source.
- Editing the `#[wasm_bindgen]` surface in `packages/sequent-core/src/wasm/`
  (locale, area tree, encrypt): the same hooks rebuild
  `sequent-core/pkg/`, so it is picked up too.
- Either way a **restart is required** — `predev` is a one-shot hook and
  the alias is decided when the config loads. Creating or deleting
  `pkg/` under a running server changes nothing.

To reproduce what a *deployed* booth does, opt into the tarball
explicitly:

```sh
WORKBENCH_SEQUENT_CORE=tgz corepack yarn workspace "@sequentech/workbench-app" dev
```

That skips the build and leaves the alias unregistered. The choice is
made by this flag alone — never by whether `pkg/` happens to exist. In
default mode a missing `pkg/` makes the config **throw with
instructions** rather than quietly falling back to the tarball.

The active source is shown on the Diagnostics page as *Booth
sequent-core*, green for the local build and amber for the tarball.
Full rationale and canaries: `LIFTING.md` §A7.

---

## Validation characterization (pointer)

The largest workbench-owned machinery is not a page but a test suite:
`characterization/` records the vote-validation behaviour (checker →
gates → filter → tally classifier) as generated tables — seven headless
rule runners plus a browser DOM-validation lane that drives every cell
through the real booth (`dom-validate.mjs`, using the Policy-overrides
panel and reload-free client-side navigation). Its conventions, commands,
and outputs are documented in
[characterization/README.md](characterization/README.md); the findings it
surfaced live in [docs/UPSTREAM_FINDINGS.md](docs/UPSTREAM_FINDINGS.md)
with reviewer recipes in [docs/REPRODUCE.md](docs/REPRODUCE.md). This
document deliberately does not duplicate any of that — the workbench
pages it *does* describe (booth, panel, pipeline, tally) are the
instruments those tools drive.

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
