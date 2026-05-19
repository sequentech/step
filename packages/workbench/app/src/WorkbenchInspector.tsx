// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Workbench inspector: a tree-rail layout with detail pages.
//
// Why a new chrome:
//   The original `Workbench.tsx` mirrored production's
//   tenant/event/election drilldown. That made sense while we were
//   lifting the booth, but the workbench is not a portal — it is a
//   tool for inspecting and editing scenarios. A directory-style
//   left rail lets every entity (snapshot, ballot style, contest,
//   voter) have its own detail page reachable from one place.
//
// Layout (`InspectorLayout`):
//   ┌─────────────┬──────────────────────────┐
//   │  Tree rail  │       <Outlet />         │
//   │             │  (snapshot / ballot      │
//   │ Snapshots   │   style / contest /      │
//   │ Tenants     │   voter detail page)     │
//   │ Voters      │                          │
//   └─────────────┴──────────────────────────┘
//
// The tree rail has three top-level sections, mirroring the locked
// design:
//
//   - Snapshots: a provenance forest rooted at the bundled JSONs
//     (`src/fixtures/snapshots/*.json`). Named checkpoints from
//     localStorage are nested under whichever bundled / checkpoint
//     their `parentId` points at. Checkpoints whose parent no longer
//     resolves (because the operator deleted the parent through
//     DevTools) appear under a synthetic ⚠ Detached group.
//   - Tenants: a tenant → event → election → {Contests, Ballot styles}
//     drill-down derived from the Redux state.
//   - Voters: the workbench's voter directory at the root.
//
// Tasks 6–9 fill in the leaf detail pages (Snapshot, Ballot style,
// Contest, Voter).

import type {RootState} from "voting-portal/src/store/store"
import {NavLink, Outlet, useNavigate, useParams} from "react-router-dom"
import {useSelector, useStore} from "react-redux"
import {useCallback, useMemo, useState, useSyncExternalStore} from "react"
import {subscribeWorkbench, useWorkbench, recordTallyRun} from "./workbenchStore"
import {
    bundledId,
    checkpointId,
    getCurrentParentId,
    listCheckpoints,
    loadSnapshotViaReload,
    normalizeCheckpointName,
    readCheckpointSnapshot,
    saveCheckpoint,
    type CheckpointMeta,
    type PersistedSnapshot,
} from "./persistence"
import {BUNDLED_SNAPSHOTS} from "./fixtures/bundledSnapshots"
import {runElectionTally, type ContestTallyOutcome} from "./electionTally"
import {setActiveVoter} from "./workbenchStore"
import {importPortalBallotStyle} from "./import/portalBallotStyleImport"
import {importVelvetElection} from "./import/velvetElectionImport"
import type {PipelineSeed, PipelineSeedRow} from "./BallotPipeline"
import buildInfo from "virtual:workbench-build-info"
// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

const RAIL_WIDTH = 280
const railStyle: React.CSSProperties = {
    width: RAIL_WIDTH,
    minWidth: RAIL_WIDTH,
    borderRight: "1px solid #ddd",
    background: "#fafafa",
    padding: "1rem",
    fontFamily: "system-ui, sans-serif",
    fontSize: "0.85rem",
    overflowY: "auto",
}
const outletStyle: React.CSSProperties = {
    flex: 1,
    padding: "1.5rem 2rem",
    fontFamily: "system-ui, sans-serif",
    overflowY: "auto",
}

export function InspectorLayout(): JSX.Element {
    return (
        <div
            style={{
                display: "flex",
                alignItems: "stretch",
                // Fill the remaining viewport below the shared top nav
                // (~2.5rem). Using `min-height` rather than `height` so
                // long detail pages can still scroll inside the outlet.
                minHeight: "calc(100vh - 2.5rem)",
            }}
        >
            <aside style={railStyle} aria-label="Workbench inspector tree">
                <TreeRail />
            </aside>
            <main style={outletStyle}>
                <Outlet />
            </main>
        </div>
    )
}

// ---------------------------------------------------------------------------
// Tree rail
// ---------------------------------------------------------------------------

function TreeRail(): JSX.Element {
    return (
        <>
            <SnapshotsSection />
            <SectionDivider />
            <ElectionsSection />
            <SectionDivider />
            <VotersSection />
        </>
    )
}

function SectionDivider(): JSX.Element {
    return (
        <hr
            style={{
                margin: "1.25rem 0",
                border: 0,
                borderTop: "1px solid #ddd",
            }}
        />
    )
}

// --- Snapshots: provenance forest ------------------------------------------

/**
 * A node in the provenance forest. Each node is either a bundled
 * snapshot (root) or a checkpoint (interior / leaf). Children are
 * the checkpoints whose `parentId` resolves to this node.
 */
interface ProvenanceNode {
    /** Tagged id (`bundled:<name>` or `checkpoint:<name>`). */
    id: string
    /** Human-readable label shown in the rail. */
    label: string
    /** Discriminator for icon / styling decisions. */
    kind: "bundled" | "checkpoint"
    children: ProvenanceNode[]
}

/**
 * Build the provenance forest from the bundled registry and the
 * checkpoint index. Bundled snapshots are always roots. Checkpoints
 * attach to the node whose id matches their `parentId`; if no such
 * node exists (parent missing or `parentId === undefined` from a
 * legacy entry), the checkpoint is returned in `orphans`.
 *
 * Sorting: each level is alphabetised so the rail is stable across
 * renders regardless of checkpoint save order.
 */
function buildProvenanceForest(
    bundled: string[],
    checkpoints: CheckpointMeta[]
): {roots: ProvenanceNode[]; orphans: ProvenanceNode[]} {
    const byId = new Map<string, ProvenanceNode>()
    const roots: ProvenanceNode[] = []
    for (const name of [...bundled].sort()) {
        const node: ProvenanceNode = {
            id: bundledId(name),
            label: name,
            kind: "bundled",
            children: [],
        }
        byId.set(node.id, node)
        roots.push(node)
    }
    // Pre-create every checkpoint node so a checkpoint can parent
    // another checkpoint regardless of insertion order.
    const cpNodes: {meta: CheckpointMeta; node: ProvenanceNode}[] = []
    for (const cp of [...checkpoints].sort((a, b) =>
        a.name.localeCompare(b.name, undefined, {sensitivity: "base"})
    )) {
        const node: ProvenanceNode = {
            id: checkpointId(cp.name),
            label: cp.name,
            kind: "checkpoint",
            children: [],
        }
        byId.set(node.id, node)
        cpNodes.push({meta: cp, node})
    }
    const orphans: ProvenanceNode[] = []
    for (const {meta, node} of cpNodes) {
        const parent =
            meta.parentId != null ? byId.get(meta.parentId) : undefined
        if (parent) {
            parent.children.push(node)
        } else if (meta.parentId === null) {
            // Explicit root — rare but supported (e.g. a snapshot
            // saved before any bundled parent existed).
            roots.push(node)
        } else {
            orphans.push(node)
        }
    }
    return {roots, orphans}
}

function SnapshotsSection(): JSX.Element {
    const checkpoints = useCheckpointList()
    const currentParent = useCurrentParentId()
    const bundled = Object.keys(BUNDLED_SNAPSHOTS)
    const {roots, orphans} = buildProvenanceForest(bundled, checkpoints)
    return (
        <section>
            <SectionHeading>Snapshots</SectionHeading>
            {/* The working copy is intentionally *not* a node in the
                forest — per the locked design, the auto-resume slot
                stays out of the provenance tree. The working-copy
                overview lives at /wb (the index route) and is reached
                via the top-nav "Inspector" link. Task 6 will add the
                `Save current state as checkpoint…` button here. */}
            <ul style={listStyle}>
                {roots.map((n) => (
                    <ProvenanceTreeNode
                        key={n.id}
                        node={n}
                        currentParent={currentParent}
                        depth={0}
                    />
                ))}
            </ul>
            {orphans.length > 0 && (
                <>
                    <SubHeading>⚠ Detached</SubHeading>
                    <ul style={listStyle}>
                        {orphans.map((n) => (
                            <ProvenanceTreeNode
                                key={n.id}
                                node={n}
                                currentParent={currentParent}
                                depth={0}
                            />
                        ))}
                    </ul>
                </>
            )}
        </section>
    )
}

function ProvenanceTreeNode(props: {
    node: ProvenanceNode
    currentParent: string | null
    depth: number
}): JSX.Element {
    const {node, currentParent, depth} = props
    const icon = node.kind === "bundled" ? "▣" : "◇"
    return (
        <li style={{marginLeft: depth === 0 ? 0 : "1rem"}}>
            <NavLink
                to={`/wb/snapshot/${encodeURIComponent(node.id)}`}
                style={navLinkStyle}
                title={node.id}
            >
                <span style={{marginRight: "0.3rem"}}>{icon}</span>
                <span
                    style={{
                        fontWeight: currentParent === node.id ? 600 : 400,
                    }}
                >
                    {node.label}
                </span>
            </NavLink>
            {node.children.length > 0 && (
                <ul style={listStyle}>
                    {node.children.map((c) => (
                        <ProvenanceTreeNode
                            key={c.id}
                            node={c}
                            currentParent={currentParent}
                            depth={depth + 1}
                        />
                    ))}
                </ul>
            )}
        </li>
    )
}

// --- Elections: flat list of elections → {Contests, Ballot styles} -------
//
// Tenants and events exist in the source data (Redux holds them) but
// the workbench operates at the election level and has no
// tenant/event affordances — there's nothing to inspect on them and
// their labels are either UUIDs (tenant) or duplicate the election
// name (event). We deliberately flatten them out of the rail.

interface ElectionNode {
    id: string
    name: string
    contestIds: {id: string; name: string}[]
    ballotStyleIds: {id: string; name: string}[]
}

/** Short, BS-specific label for the rail. Velvet imports carry an
 *  `area_id` so we surface that; otherwise we fall back to a short
 *  slice of the BS id. We deliberately do NOT fall back to the
 *  election name \u2014 that hides multi-BS elections behind identical
 *  labels (see "Velvet sample election" being shown three times in
 *  the same subtree). */
function ballotStyleRailLabel(bs: {
    id: string
    area_id?: string | null
}): string {
    const short = `${bs.id.slice(0, 8)}\u2026`
    if (typeof bs.area_id === "string" && bs.area_id.length > 0) {
        return `${short} (area ${bs.area_id.slice(0, 4)}\u2026)`
    }
    return short
}

function buildElectionsList(
    state: RootState,
    // Optional workbench overlay pool, keyed by election id. When
    // present we use it as the authoritative BS catalogue for the
    // rail — the portal `state.ballotStyles` slice only ever holds
    // ONE BS per election (the one bound to the currently active
    // voter, see `applyEligibilitySwap`), so reading it would hide
    // every other BS that was imported.
    pool: Record<string, unknown[]> | undefined
): ElectionNode[] {
    type PortalBSRow = NonNullable<RootState["ballotStyles"][string]>
    // Union the portal slice (live BS) with the workbench pool
    // (everything imported), deduped by id. The pool wins on tie
    // because it carries the original imported data; the live slice
    // is the same object anyway.
    const allBs = new Map<string, PortalBSRow>()
    for (const bs of Object.values(state.ballotStyles)) {
        if (bs) allBs.set(bs.id, bs)
    }
    if (pool) {
        for (const rows of Object.values(pool)) {
            for (const row of rows) {
                const id = (row as {id?: unknown}).id
                if (typeof id === "string" && !allBs.has(id)) {
                    allBs.set(id, row as PortalBSRow)
                }
            }
        }
    }
    // Index ballot styles by election for cheap lookup. Labels use
    // the BS's own id (and its area id if present) rather than
    // falling back to the election name, which would make every BS
    // in a single-election fixture look identical in the rail.
    const bsByElection = new Map<string, {id: string; name: string}[]>()
    for (const bs of allBs.values()) {
        const list = bsByElection.get(bs.election_id) ?? []
        list.push({id: bs.id, name: ballotStyleRailLabel(bs)})
        bsByElection.set(bs.election_id, list)
    }
    const elections: ElectionNode[] = []
    for (const el of Object.values(state.elections)) {
        if (!el) continue
        const node: ElectionNode = {
            id: el.id,
            name: el.name ?? "(unnamed election)",
            contestIds: [],
            ballotStyleIds: bsByElection.get(el.id) ?? [],
        }
        // Contests live on the ballot styles' EML. Dedupe by id
        // across all ballot styles of this election — iterating the
        // union (allBs) so contests that only appear on a
        // pool-only BS still show up.
        const seen = new Set<string>()
        for (const bs of allBs.values()) {
            if (bs.election_id !== el.id) continue
            for (const c of bs.ballot_eml.contests) {
                if (seen.has(c.id)) continue
                seen.add(c.id)
                node.contestIds.push({id: c.id, name: c.name})
            }
        }
        elections.push(node)
    }
    // Alphabetise everything for a stable rail.
    elections.sort((a, b) => a.name.localeCompare(b.name))
    for (const el of elections) {
        el.contestIds.sort((a, b) => a.name.localeCompare(b.name))
        el.ballotStyleIds.sort((a, b) => a.name.localeCompare(b.name))
    }
    return elections
}

function ElectionsSection(): JSX.Element {
    // The portal `state.ballotStyles` slice only ever carries the
    // single live BS, so the rail also reads `ballotStylePool` (the
    // full imported catalogue) and merges them — see
    // `buildElectionsList`.
    const state = useSelector((s: RootState) => s)
    const pool = useWorkbench((w) => w.ballotStylePool)
    const elections = useMemo(
        () => buildElectionsList(state, pool),
        [state, pool]
    )
    return (
        <section>
            <SectionHeading>Elections</SectionHeading>
            {elections.length === 0 ? (
                <Empty>(none)</Empty>
            ) : (
                <ul style={listStyle}>
                    {elections.map((el) => (
                        <li key={el.id}>
                            <NodeLabel title={el.id}>{el.name}</NodeLabel>
                            <ElectionChildren election={el} />
                        </li>
                    ))}
                </ul>
            )}
        </section>
    )
}

function ElectionChildren({election}: {election: ElectionNode}): JSX.Element {
    return (
        <ul style={{...listStyle, marginLeft: "1rem"}}>
            <li>
                <NodeLabel>Contests</NodeLabel>
                <ul style={{...listStyle, marginLeft: "1rem"}}>
                    {election.contestIds.length === 0 ? (
                        <li>
                            <Empty>(none)</Empty>
                        </li>
                    ) : (
                        election.contestIds.map((c) => (
                            <li key={c.id}>
                                <NavLink
                                    to={`/wb/contest/${c.id}`}
                                    style={navLinkStyle}
                                >
                                    {c.name}
                                </NavLink>
                            </li>
                        ))
                    )}
                </ul>
            </li>
            <li>
                <NodeLabel>Ballot styles</NodeLabel>
                <ul style={{...listStyle, marginLeft: "1rem"}}>
                    {election.ballotStyleIds.length === 0 ? (
                        <li>
                            <Empty>(none)</Empty>
                        </li>
                    ) : (
                        election.ballotStyleIds.map((bs) => (
                            <li key={bs.id}>
                                <NavLink
                                    to={`/wb/ballot-style/${bs.id}`}
                                    style={navLinkStyle}
                                >
                                    {bs.name}
                                </NavLink>
                            </li>
                        ))
                    )}
                </ul>
            </li>
        </ul>
    )
}

// --- Voters: workbench directory at root -----------------------------------

function VotersSection(): JSX.Element {
    const voters = useWorkbench((w) => w.voters)
    return (
        <section>
            <SectionHeading>Voters</SectionHeading>
            {voters.length === 0 ? (
                <Empty>(none)</Empty>
            ) : (
                <ul style={listStyle}>
                    {voters.map((v) => (
                        <li key={v.id}>
                            <NavLink
                                to={`/wb/voter/${v.id}`}
                                style={navLinkStyle}
                            >
                                {v.displayName}
                            </NavLink>
                        </li>
                    ))}
                </ul>
            )}
        </section>
    )
}

// ---------------------------------------------------------------------------
// Detail pages
// ---------------------------------------------------------------------------

/**
 * `/wb` index — overview of the live working copy.
 *
 * Surfaces:
 *  - Provenance lineage ("forked from <parent>").
 *  - Summary counts (voters, elections, ballot styles, cast votes).
 *  - `Save current state as checkpoint…` button. The checkpoint
 *    inherits the working copy's `currentParentId` automatically.
 *
 * Intentionally minimal: there is no "copy current state as JSON"
 * button here — bundling new scenarios is done from a checkpoint's
 * detail page so the export already has a known name.
 */
export function SnapshotOverviewPage(): JSX.Element {
    const store = useStore()
    const parentId = useCurrentParentId()
    // Select individual scalars rather than a derived object so
    // useSelector's referential-equality check doesn't false-positive
    // on every dispatch.
    const electionCount = useSelector(
        (s: RootState) => Object.values(s.elections).filter(Boolean).length
    )
    // Working-copy BS count: prefer the workbench overlay's
    // `ballotStylePool` (full imported catalogue) over the portal
    // `ballotStyles` slice, which only holds the BS for the active
    // voter session and would under-count any multi-BS snapshot.
    // Mirrors the bundled/checkpoint rows' `selectStateCounts`.
    const ballotStyleCount = useWorkbench((w) =>
        w.ballotStylePool
            ? Object.values(w.ballotStylePool).reduce(
                  (n, rows) => n + rows.length,
                  0
              )
            : null
    )
    const ballotStyleSliceCount = useSelector(
        (s: RootState) =>
            Object.values(s.ballotStyles).filter(Boolean).length
    )
    const castVoteCount = useSelector((s: RootState) =>
        Object.values(s.castVotes).reduce(
            (n, list) => n + (list?.length ?? 0),
            0
        )
    )
    const voterCount = useWorkbench((w) => w.voters.length)
    const checkpoints = useCheckpointList()
    const [error, setError] = useState<string | null>(null)
    // Which import mode is currently open, if any. Each mode shows
    // its own textarea + parser; the three are mutually exclusive
    // because re-using a single textarea keeps the layout compact
    // and the operator never wants two import flows open at once.
    const [importMode, setImportMode] = useState<
        null | "snapshot" | "ballotStyle" | "velvet"
    >(null)
    const [importJson, setImportJson] = useState("")
    const [importError, setImportError] = useState<string | null>(null)
    const [importBusy, setImportBusy] = useState(false)

    const openImport = (
        mode: "snapshot" | "ballotStyle" | "velvet"
    ): void => {
        setImportMode(mode)
        setImportJson("")
        setImportError(null)
    }
    const cancelImport = (): void => {
        setImportMode(null)
        setImportError(null)
        setImportJson("")
    }

    const onImport = async (): Promise<void> => {
        setImportError(null)
        if (importMode === "snapshot") {
            let parsed: PersistedSnapshot
            try {
                parsed = JSON.parse(importJson) as PersistedSnapshot
            } catch (e) {
                setImportError(
                    "Invalid JSON: " +
                        (e instanceof Error ? e.message : String(e))
                )
                return
            }
            if (parsed == null || typeof parsed !== "object") {
                setImportError("Snapshot must be a JSON object.")
                return
            }
            if (parsed.version !== "v1") {
                setImportError(
                    `Unsupported snapshot version: ${String(
                        parsed.version
                    )} (expected "v1").`
                )
                return
            }
            if (parsed.state == null || typeof parsed.state !== "object") {
                setImportError("Snapshot is missing a `state` object.")
                return
            }
            // Require `workbench.keypair` on raw-snapshot import. The
            // BS and Velvet import variants generate their own
            // keypair (see rekeySnapshot in importHelpers); raw v1
            // imports are the only path that could otherwise land us
            // in a no-keypair state, where every subsequent cast
            // vote captured by `tryCaptureRepairedCastVote` would
            // silently skip decrypt and leave `decodedBigInts` empty
            // — indistinguishable in the UI from "decrypt pending"
            // or "decrypt failed". Rejecting up-front keeps the
            // failure modes of an installed snapshot to just (a) the
            // cast vote has no `content`, and (b) decrypt threw.
            const kp = parsed.workbench?.keypair as
                | {pkB64?: unknown; skB64?: unknown}
                | null
                | undefined
            if (
                !kp ||
                typeof kp.pkB64 !== "string" ||
                typeof kp.skB64 !== "string" ||
                kp.pkB64.length === 0 ||
                kp.skB64.length === 0
            ) {
                setImportError(
                    "Snapshot is missing `workbench.keypair` (pkB64/skB64). " +
                        "Raw v1 snapshots must carry the keypair they were " +
                        "captured with; use the Ballot style or Velvet " +
                        "election import variant to mint a fresh one."
                )
                return
            }
            try {
                // Wipe + reload: write the imported snapshot to the
                // auto-resume slot as a root (parentId = null), then
                // reload so the boot path hydrates a fresh, empty
                // store. This guarantees the resulting working copy
                // matches the source JSON exactly with no leftovers
                // from before. If the user wants to keep it, they
                // Save… after.
                loadSnapshotViaReload(parsed, null)
            } catch (e) {
                setImportError(
                    e instanceof Error ? e.message : String(e)
                )
            }
            return
        }
        // Both `ballotStyle` and `velvet` go through their dedicated
        // builder, which generates fresh keypairs (async via WASM)
        // and assembles the full PersistedSnapshot before reload.
        setImportBusy(true)
        try {
            const snap =
                importMode === "ballotStyle"
                    ? await importPortalBallotStyle(importJson)
                    : await importVelvetElection(importJson)
            loadSnapshotViaReload(snap, null)
        } catch (e) {
            setImportError(e instanceof Error ? e.message : String(e))
        } finally {
            setImportBusy(false)
        }
    }

    const onSave = (): void => {
        setError(null)
        const raw = window.prompt(
            "Checkpoint name (letters, digits, spaces, '.', '-', '_'; max 64 chars):"
        )
        if (raw == null) return
        try {
            saveCheckpoint(
                store as Parameters<typeof saveCheckpoint>[0],
                raw
            )
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e))
        }
    }
    const onLoadBundled = (name: string, snapshot: PersistedSnapshot): void => {
        // Wipe + reload (see loadSnapshotViaReload). `navigate` is a
        // no-op here because the reload tears down the SPA, but the
        // boot path lands us back on /wb anyway.
        loadSnapshotViaReload(snapshot, bundledId(name))
    }
    const onLoadCheckpoint = (name: string): void => {
        const snapshot = readCheckpointSnapshot(name)
        if (!snapshot) return
        loadSnapshotViaReload(snapshot, checkpointId(name))
    }

    // Build the unified row list. The working copy is always first;
    // bundled snapshots follow (alphabetical), then checkpoints
    // (newest savedAt first). Each row carries everything the table
    // needs to render without re-reading localStorage.
    type Row =
        | {
              kind: "working"
              name: string
              parentId: string | null
              savedAt: string
              counts: {
                  voters: number
                  elections: number
                  ballotStyles: number
                  castVotes: number
              }
          }
        | {
              kind: "bundled" | "checkpoint"
              id: string
              name: string
              parentId: string | null | undefined
              savedAt: string
              counts: {
                  voters: number
                  elections: number
                  ballotStyles: number
                  castVotes: number
              }
              snapshot: PersistedSnapshot
          }

    const rows: Row[] = []
    rows.push({
        kind: "working",
        name: "Working copy",
        parentId,
        savedAt: "(live)",
        counts: {
            voters: voterCount,
            elections: electionCount,
            ballotStyles: ballotStyleCount ?? ballotStyleSliceCount,
            castVotes: castVoteCount,
        },
    })
    const bundledEntries = Object.entries(BUNDLED_SNAPSHOTS).sort((a, b) =>
        a[0].localeCompare(b[0])
    )
    for (const [bName, bSnapshot] of bundledEntries) {
        const c = selectStateCounts(bSnapshot)
        rows.push({
            kind: "bundled",
            id: bundledId(bName),
            name: bName,
            parentId: bSnapshot.parentId ?? null,
            savedAt: "(shipped)",
            counts: {
                voters: bSnapshot.workbench?.voters.length ?? 0,
                elections: c.elections,
                ballotStyles: c.ballotStyles,
                castVotes: c.castVotes,
            },
            snapshot: bSnapshot,
        })
    }
    const checkpointEntries = [...checkpoints].sort((a, b) =>
        b.savedAt.localeCompare(a.savedAt)
    )
    for (const meta of checkpointEntries) {
        const snap = readCheckpointSnapshot(meta.name)
        if (!snap) continue
        const c = selectStateCounts(snap)
        rows.push({
            kind: "checkpoint",
            id: checkpointId(meta.name),
            name: meta.name,
            parentId: meta.parentId ?? snap.parentId ?? null,
            savedAt: meta.savedAt,
            counts: {
                voters: snap.workbench?.voters.length ?? 0,
                elections: c.elections,
                ballotStyles: c.ballotStyles,
                castVotes: c.castVotes,
            },
            snapshot: snap,
        })
    }

    return (
        <>
            <h1>Snapshots</h1>
            <p style={{color: "#666"}}>
                The working copy is the live in-memory state, auto-saved
                to localStorage on every change. Bundled snapshots ship
                in git; checkpoints are saved by you and live only in
                this browser.
            </p>
            <table style={snapshotTableStyle}>
                <thead>
                    <tr>
                        <th style={thStyle}>Name</th>
                        <th style={thStyle}>Kind</th>
                        <th style={thStyle}>Forked from</th>
                        <th style={thStyle}>Saved at</th>
                        <th style={thNumStyle}>Voters</th>
                        <th style={thNumStyle}>Elections</th>
                        <th style={thNumStyle}>Ballot styles</th>
                        <th style={thNumStyle}>Cast votes</th>
                        <th style={thStyle}>Action</th>
                    </tr>
                </thead>
                <tbody>
                    {rows.map((row) => (
                        <tr
                            key={
                                row.kind === "working" ? "__working" : row.id
                            }
                            style={
                                row.kind === "working"
                                    ? workingRowStyle
                                    : undefined
                            }
                        >
                            <td style={tdStyle}>
                                {row.kind === "working" ? (
                                    <strong>{row.name}</strong>
                                ) : (
                                    <NavLink
                                        to={`/wb/snapshot/${encodeURIComponent(
                                            row.id
                                        )}`}
                                        style={inlineLinkStyle}
                                    >
                                        {row.kind === "bundled" ? "▣" : "◇"}{" "}
                                        {row.name}
                                    </NavLink>
                                )}
                            </td>
                            <td style={tdMutedStyle}>{row.kind}</td>
                            <td style={tdStyle}>
                                <ParentCell parentId={row.parentId ?? null} />
                            </td>
                            <td style={tdMutedStyle}>
                                {row.savedAt.startsWith("(") ? (
                                    row.savedAt
                                ) : (
                                    <code>{row.savedAt}</code>
                                )}
                            </td>
                            <td style={tdNumStyle}>{row.counts.voters}</td>
                            <td style={tdNumStyle}>{row.counts.elections}</td>
                            <td style={tdNumStyle}>
                                {row.counts.ballotStyles}
                            </td>
                            <td style={tdNumStyle}>{row.counts.castVotes}</td>
                            <td style={tdStyle}>
                                {row.kind === "working" ? (
                                    <button
                                        type="button"
                                        style={primaryButtonStyle}
                                        onClick={onSave}
                                    >
                                        Save…
                                    </button>
                                ) : (
                                    <button
                                        type="button"
                                        style={secondaryButtonStyle}
                                        onClick={() => {
                                            if (row.kind === "bundled") {
                                                onLoadBundled(
                                                    row.name,
                                                    row.snapshot
                                                )
                                            } else {
                                                onLoadCheckpoint(row.name)
                                            }
                                        }}
                                    >
                                        Load
                                    </button>
                                )}
                            </td>
                        </tr>
                    ))}
                </tbody>
            </table>
            {error && (
                <p style={{color: "#b00020", marginTop: "0.5rem"}}>
                    {error}
                </p>
            )}
            <div style={{marginTop: "1.5rem"}}>
                {importMode === null ? (
                    <div style={{display: "flex", gap: "0.5rem"}}>
                        <button
                            type="button"
                            style={secondaryButtonStyle}
                            onClick={() => openImport("snapshot")}
                        >
                            Import snapshot JSON…
                        </button>
                        <button
                            type="button"
                            style={secondaryButtonStyle}
                            onClick={() => openImport("ballotStyle")}
                        >
                            Import portal ballot style…
                        </button>
                        <button
                            type="button"
                            style={secondaryButtonStyle}
                            onClick={() => openImport("velvet")}
                        >
                            Import velvet election…
                        </button>
                    </div>
                ) : (
                    <div style={importPanelStyle}>
                        <SubHeading>
                            {importMode === "snapshot"
                                ? "Import snapshot JSON into working copy"
                                : importMode === "ballotStyle"
                                  ? "Import portal ballot style"
                                  : "Import velvet ElectionConfig"}
                        </SubHeading>
                        {importMode === "snapshot" && (
                            <p style={{color: "#666", marginTop: 0}}>
                                Paste a full <code>PersistedSnapshot</code>{" "}
                                (same shape as the <em>Bundled JSON</em>{" "}
                                block on any snapshot detail page). It
                                is loaded straight into the working
                                copy as a root — the working copy's{" "}
                                <code>parentId</code> is set to{" "}
                                <code>null</code> regardless of what the
                                source JSON says, so the imported state
                                has no provenance. To keep it around,
                                click <em>Save…</em> on the
                                working-copy row after importing.
                            </p>
                        )}
                        {importMode === "ballotStyle" && (
                            <p style={{color: "#666", marginTop: 0}}>
                                Paste a single portal{" "}
                                <code>IBallotStyle</code> row (the
                                shape returned by{" "}
                                <code>
                                    select * from
                                    public.ballot_styles where id = …
                                </code>
                                ). A fresh workbench keypair is
                                generated and stamped into{" "}
                                <code>ballot_eml.public_key</code>; a
                                single voter named <em>voter</em> is
                                created and assigned to the ballot
                                style.
                            </p>
                        )}
                        {importMode === "velvet" && (
                            <p style={{color: "#666", marginTop: 0}}>
                                Paste a velvet{" "}
                                <code>ElectionConfig</code> JSON (see{" "}
                                <code>
                                    fixtures/velvet/sample-election-config.json
                                </code>
                                ). Each ballot style is re-keyed with a
                                fresh workbench keypair; one voter is
                                created per <code>TreeNodeArea</code>{" "}
                                and assigned to the ballot styles whose{" "}
                                <code>area_id</code> matches.
                            </p>
                        )}
                        <label style={importLabelStyle}>
                            JSON
                            <textarea
                                value={importJson}
                                onChange={(e) =>
                                    setImportJson(e.target.value)
                                }
                                placeholder={
                                    importMode === "snapshot"
                                        ? '{"version":"v1","state":{...}}'
                                        : importMode === "ballotStyle"
                                          ? '{"id":"…","election_id":"…","ballot_eml":{...}}'
                                          : '{"id":"…","ballot_styles":[…],"areas":[…]}'
                                }
                                style={importTextareaStyle}
                                spellCheck={false}
                                disabled={importBusy}
                            />
                        </label>
                        {importError && (
                            <p
                                style={{
                                    color: "#b00020",
                                    marginTop: "0.5rem",
                                }}
                            >
                                {importError}
                            </p>
                        )}
                        <div
                            style={{
                                marginTop: "0.75rem",
                                display: "flex",
                                gap: "0.5rem",
                            }}
                        >
                            <button
                                type="button"
                                style={primaryButtonStyle}
                                onClick={() => {
                                    void onImport()
                                }}
                                disabled={importBusy}
                            >
                                {importBusy ? "Importing…" : "Import"}
                            </button>
                            <button
                                type="button"
                                style={secondaryButtonStyle}
                                onClick={cancelImport}
                                disabled={importBusy}
                            >
                                Cancel
                            </button>
                        </div>
                    </div>
                )}
            </div>
            <BuildStatusCard />
        </>
    )
}

function ParentCell({parentId}: {parentId: string | null}): JSX.Element {
    if (parentId == null) {
        return <span style={{color: "#888"}}>(root)</span>
    }
    return (
        <NavLink
            to={`/wb/snapshot/${encodeURIComponent(parentId)}`}
            style={inlineLinkStyle}
        >
            <code>{parentId}</code>
        </NavLink>
    )
}

// ---------------------------------------------------------------------------
// Build-status card
// ---------------------------------------------------------------------------

/**
 * Renders a compact status table sourced from the
 * `virtual:workbench-build-info` module (see `workbenchBuildInfo`
 * plugin in `vite.config.ts`). One row per tracked wasm artifact:
 * its mtime, the newest mtime across its (transitive) crate source
 * dirs, and a "stale" pill when sources are newer than the built
 * artifact.
/**
 * Renders a compact build-provenance table sourced from the
 * `virtual:workbench-build-info` module (see `workbenchBuildInfo`
 * plugin in `vite.config.ts`). One row per tracked wasm artifact:
 * its mtime (when it was last compiled) and the workspace-internal
 * crates baked into it with their resolved Cargo.lock versions.
 *
 * Deliberately *not* a staleness checker: the same data could
 * support "is the wasm older than the Rust source?" but that only
 * makes sense in the monorepo and would silently mislead in a
 * standalone-packaged workbench (where source dirs don't ship).
 * Keeping the card to pure provenance lets it carry the same
 * meaning in dev and in any future packaged build.
 */
function BuildStatusCard(): JSX.Element {
    const fmt = (iso: string | null): string => {
        if (!iso) return "—"
        const d = new Date(iso)
        const ageMs = Date.now() - d.getTime()
        return `${d.toLocaleString()} (${humanAge(ageMs)} ago)`
    }
    return (
        <div style={buildStatusCardStyle}>
            <div style={buildStatusHeaderStyle}>
                <strong>Build status</strong>
                <span style={{color: "#888", fontSize: "0.8rem"}}>
                    Snapshot taken at{" "}
                    <code>
                        {new Date(buildInfo.generatedAt).toLocaleString()}
                    </code>
                </span>
            </div>
            <table style={buildStatusTableStyle}>
                <thead>
                    <tr>
                        <th style={thStyle}>Artifact</th>
                        <th style={thStyle}>Compiled</th>
                    </tr>
                </thead>
                <tbody>
                    {buildInfo.artifacts.map((a) => (
                        <tr key={a.artifactPath}>
                            <td style={tdStyle}>
                                <div>{a.label}</div>
                                <code
                                    style={{
                                        fontSize: "0.75rem",
                                        color: "#888",
                                    }}
                                >
                                    {a.artifactPath}
                                </code>
                                {a.internalDeps && (
                                    <div
                                        style={{
                                            fontSize: "0.75rem",
                                            color: "#555",
                                            marginTop: "0.25rem",
                                        }}
                                    >
                                        <span style={{color: "#888"}}>
                                            internal:
                                        </span>{" "}
                                        {a.internalDeps.map((d, i) => (
                                            <span key={d.name}>
                                                {i > 0 ? ", " : ""}
                                                <code>{d.name}</code>{" "}
                                                <span style={{color: "#888"}}>
                                                    {d.version}
                                                </span>
                                            </span>
                                        ))}
                                        {a.externalDepCount != null && (
                                            <span style={{color: "#888"}}>
                                                {" "}
                                                · +{a.externalDepCount}{" "}
                                                external
                                            </span>
                                        )}
                                    </div>
                                )}
                            </td>
                            <td style={tdMutedStyle}>{fmt(a.builtAt)}</td>
                        </tr>
                    ))}
                </tbody>
            </table>
            {buildInfo.git && (
                <div
                    style={{
                        color: "#888",
                        fontSize: "0.75rem",
                        marginTop: "0.4rem",
                    }}
                >
                    repo <code>{buildInfo.git.sha}</code>
                </div>
            )}
        </div>
    )
}

function humanAge(ms: number): string {
    if (ms < 0) return "in the future"
    const s = Math.floor(ms / 1000)
    if (s < 60) return `${s}s`
    const m = Math.floor(s / 60)
    if (m < 60) return `${m}m`
    const h = Math.floor(m / 60)
    if (h < 24) return `${h}h ${m % 60}m`
    const d = Math.floor(h / 24)
    return `${d}d ${h % 24}h`
}

const buildStatusCardStyle: React.CSSProperties = {
    border: "1px solid #ddd",
    borderRadius: 4,
    padding: "0.6rem 0.9rem",
    marginTop: "1.5rem",
    background: "#fafafa",
}
const buildStatusHeaderStyle: React.CSSProperties = {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "baseline",
    marginBottom: "0.4rem",
}
const buildStatusTableStyle: React.CSSProperties = {
    width: "100%",
    borderCollapse: "collapse",
    fontSize: "0.85rem",
}

const snapshotTableStyle: React.CSSProperties = {
    width: "100%",
    borderCollapse: "collapse",
    fontSize: "0.9rem",
    marginTop: "0.5rem",
}
const thStyle: React.CSSProperties = {
    textAlign: "left",
    borderBottom: "1px solid #ccc",
    padding: "0.4rem 0.6rem",
    fontWeight: 600,
    color: "#444",
}
const thNumStyle: React.CSSProperties = {
    ...thStyle,
    textAlign: "right",
}
const tdStyle: React.CSSProperties = {
    borderBottom: "1px solid #eee",
    padding: "0.4rem 0.6rem",
    verticalAlign: "middle",
}
const tdMutedStyle: React.CSSProperties = {
    ...tdStyle,
    color: "#666",
}
const tdNumStyle: React.CSSProperties = {
    ...tdStyle,
    textAlign: "right",
    fontVariantNumeric: "tabular-nums",
}
const workingRowStyle: React.CSSProperties = {
    background: "#f4f9ff",
}
const importPanelStyle: React.CSSProperties = {
    border: "1px solid #ddd",
    borderRadius: 4,
    padding: "0.75rem 1rem",
    background: "#fafafa",
}
const importLabelStyle: React.CSSProperties = {
    display: "flex",
    flexDirection: "column",
    gap: "0.25rem",
    marginTop: "0.75rem",
    fontSize: "0.9rem",
    color: "#444",
}
const importInputStyle: React.CSSProperties = {
    padding: "0.4rem 0.5rem",
    fontSize: "0.95rem",
    border: "1px solid #ccc",
    borderRadius: 3,
}
const importTextareaStyle: React.CSSProperties = {
    ...importInputStyle,
    fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
    fontSize: "0.8rem",
    minHeight: "12rem",
    resize: "vertical",
}

function selectStateCounts(snap: PersistedSnapshot): {
    elections: number
    ballotStyles: number
    castVotes: number
} {
    const s = snap.state
    // Ballot styles: prefer the workbench overlay's `ballotStylePool`
    // which is the full imported catalogue (every BS across every
    // election). The portal `state.ballotStyles` slice only ever holds
    // the BS for the *currently active* voter session, so on a fresh
    // snapshot it counts 1 even when the import carried more. Fall
    // back to the slice for legacy snapshots without an overlay.
    const pool = snap.workbench?.ballotStylePool
    const ballotStyles = pool
        ? Object.values(pool).reduce((n, rows) => n + rows.length, 0)
        : Object.values(s.ballotStyles).filter(Boolean).length
    return {
        elections: Object.values(s.elections).filter(Boolean).length,
        ballotStyles,
        castVotes: Object.values(s.castVotes).reduce(
            (n, list) => n + (list?.length ?? 0),
            0
        ),
    }
}

/**
 * `/wb/snapshot/:id` — detail page for a bundled snapshot or a named
 * checkpoint. The `:id` segment is a tagged id (`bundled:<name>` or
 * `checkpoint:<name>`) URL-encoded by the rail.
 *
 * Renders: type badge, lineage, summary counts, `Load` action, and a
 * collapsed copy-as-bundled JSON block. The bundled export form
 * always has `parentId` stripped so the user can paste it directly
 * under `src/fixtures/snapshots/` without editing.
 */
export function SnapshotDetailPage(): JSX.Element {
    const {id: rawId} = useParams()
    const id = rawId != null ? decodeURIComponent(rawId) : ""
    // Subscribe to checkpoint mutations so a Save-then-navigate flow
    // (or a Load that returns here) sees fresh data.
    useCheckpointList()
    const kind: "bundled" | "checkpoint" | "unknown" = id.startsWith("bundled:")
        ? "bundled"
        : id.startsWith("checkpoint:")
        ? "checkpoint"
        : "unknown"
    const name = id.slice(id.indexOf(":") + 1)
    const snapshot = useMemo<PersistedSnapshot | null>(() => {
        if (kind === "bundled") return BUNDLED_SNAPSHOTS[name] ?? null
        if (kind === "checkpoint") return readCheckpointSnapshot(name)
        return null
    // `BUNDLED_SNAPSHOTS` is frozen at build time; the checkpoint
    // read is keyed on `name` which is what we want to invalidate on.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [kind, name])
    const meta = useMemo<CheckpointMeta | undefined>(
        () =>
            kind === "checkpoint"
                ? listCheckpoints().find((c) => c.name === name)
                : undefined,
        [kind, name]
    )
    if (kind === "unknown" || !snapshot) {
        return (
            <>
                <h1>Snapshot not found</h1>
                <p>
                    <code>{id || "(missing id)"}</code>
                </p>
                <p style={{color: "#666"}}>
                    The bundled JSON or checkpoint may have been removed.
                </p>
            </>
        )
    }
    const stateCounts = selectStateCounts(snapshot)
    const voterCount = snapshot.workbench?.voters.length ?? 0
    const bundledExport = useMemo(() => {
        // Strip `parentId` for the copy-as-bundled form: a bundled
        // root snapshot has no parent by definition.
        const {parentId: _drop, ...rest} = snapshot
        return JSON.stringify(rest, null, 2)
    }, [snapshot])
    return (
        <>
            <h1>
                {kind === "bundled" ? "▣" : "◇"} {name}
            </h1>
            <p style={{color: "#666"}}>
                <code>{id}</code> &middot;{" "}
                {kind === "bundled" ? "Bundled snapshot" : "Checkpoint"}
            </p>
            <dl style={dlStyle}>
                <DlRow label="Forked from">
                    <code>
                        {snapshot.parentId ?? "(root — no parent)"}
                    </code>
                </DlRow>
                {meta && (
                    <DlRow label="Saved at">
                        <code>{meta.savedAt}</code>
                    </DlRow>
                )}
                <DlRow label="Voters">{voterCount}</DlRow>
                <DlRow label="Elections">{stateCounts.elections}</DlRow>
                <DlRow label="Ballot styles">
                    {stateCounts.ballotStyles}
                </DlRow>
                <DlRow label="Cast votes">{stateCounts.castVotes}</DlRow>
            </dl>
            <div style={{marginTop: "1.5rem"}}>
                <button
                    type="button"
                    style={primaryButtonStyle}
                    onClick={() => {
                        // Wipe + reload via the auto-resume slot.
                        const parentId =
                            kind === "checkpoint"
                                ? checkpointId(name)
                                : bundledId(name)
                        loadSnapshotViaReload(snapshot, parentId)
                    }}
                >
                    Load
                </button>
            </div>
            <details style={{marginTop: "1.5rem"}}>
                <summary style={{cursor: "pointer", color: "#444"}}>
                    Bundled JSON (copy-paste under{" "}
                    <code>src/fixtures/snapshots/</code> to ship)
                </summary>
                <CopyJsonBlock json={bundledExport} />
            </details>
        </>
    )
}

function CopyJsonBlock({json}: {json: string}): JSX.Element {
    const [copied, setCopied] = useState(false)
    return (
        <>
            <div style={{margin: "0.5rem 0"}}>
                <button
                    type="button"
                    style={secondaryButtonStyle}
                    onClick={() => {
                        void navigator.clipboard.writeText(json).then(() => {
                            setCopied(true)
                            window.setTimeout(
                                () => setCopied(false),
                                1500
                            )
                        })
                    }}
                >
                    {copied ? "Copied." : "Copy JSON"}
                </button>
            </div>
            <pre
                style={{
                    background: "#f4f4f4",
                    padding: "0.75rem",
                    borderRadius: 4,
                    fontSize: "0.75rem",
                    maxHeight: "24rem",
                    overflow: "auto",
                }}
            >
                <code>{json}</code>
            </pre>
        </>
    )
}

/**
 * `/wb/ballot-style/:id` — detail page for one ballot style.
 *
 * Surfaces what makes a ballot style operationally distinct in the
 * workbench:
 *  - the public key (pk) every voter encrypts against,
 *  - the matching secret key (sk) the tally uses, behind a reveal
 *    toggle so the page is screenshotable without burning the key,
 *  - the contests (NavLinks into the contest detail page),
 *  - the raw `ballot_eml` (collapsed JSON) for diffing against an
 *    upstream EML.
 *
 * Missing keypairs are not silently swallowed — they would mean the
 * decrypt bridge can't run, so we surface that as an explicit error
 * row rather than letting the page render half-broken.
 */
export function BallotStyleDetailPage(): JSX.Element {
    const {id} = useParams()
    const bsId = id ?? ""
    // BallotStylesState is keyed by election_id and only ever holds
    // the single live BS for the active voter, so it's missing every
    // pool-only BS (e.g. Area B in the velvet sample import while
    // Area A's voter is active). Fall through to the workbench pool,
    // which is the full imported catalogue.
    type PortalBSRow = NonNullable<RootState["ballotStyles"][string]>
    const liveBallotStyle = useSelector((s: RootState) =>
        Object.values(s.ballotStyles).find((bs) => bs?.id === bsId)
    )
    const pool = useWorkbench((w) => w.ballotStylePool)
    const ballotStyle = useMemo<PortalBSRow | undefined>(() => {
        if (liveBallotStyle) return liveBallotStyle
        if (!pool) return undefined
        for (const rows of Object.values(pool)) {
            for (const row of rows) {
                if ((row as {id?: unknown}).id === bsId) {
                    return row as PortalBSRow
                }
            }
        }
        return undefined
    }, [liveBallotStyle, pool, bsId])
    const election = useSelector((s: RootState) =>
        ballotStyle ? s.elections[ballotStyle.election_id] : undefined
    )
    // The workbench keypair is per-snapshot rather than per-ballot-style;
    // every BS in this snapshot shares the same pair. The detail page
    // still surfaces it here because the operator's mental model is
    // "this BS encrypts under this key", and the SecretKeyRow check
    // (`pk === keypair.pkB64`) catches snapshots whose pk drifted away
    // from the workbench-installed key.
    const keypair = useWorkbench((w) => w.keypair ?? undefined)
    // Reverse the workbench `assignments` map (voterId → bsId[]) to
    // surface every voter assigned to *this* ballot style. The voter
    // page links here via the bs id badge; mirroring the inverse
    // navigation closes the loop. Snapshots without an `assignments`
    // map (legacy single-voter imports) yield an empty list and the
    // section explains why.
    const voters = useWorkbench((w) => w.voters)
    const assignments = useWorkbench((w) => w.assignments)
    const assignedVoters = useMemo(() => {
        if (!assignments) return undefined
        return voters.filter((v) =>
            (assignments[v.id] ?? []).includes(bsId)
        )
    }, [voters, assignments, bsId])
    if (!ballotStyle) {
        return (
            <>
                <h1>Ballot style not found</h1>
                <p>
                    <code>{bsId || "(missing id)"}</code>
                </p>
            </>
        )
    }
    const pk = ballotStyle.ballot_eml.public_key?.public_key
    return (
        <>
            <h1>{election?.name ?? "(unnamed election)"}</h1>
            <p style={{color: "#666"}}>
                <code>{bsId}</code> &middot; Ballot style
            </p>
            <dl style={dlStyle}>
                <DlRow label="Election">
                    <code>{ballotStyle.election_id}</code>
                </DlRow>
                <DlRow label="Public key">
                    {pk ? (
                        <code style={codeBlockStyle}>{pk}</code>
                    ) : (
                        <em style={{color: "#b00020"}}>
                            missing on ballot_eml.public_key.public_key
                        </em>
                    )}
                </DlRow>
                <DlRow label="Secret key">
                    <SecretKeyRow keypair={keypair} pk={pk} />
                </DlRow>
            </dl>
            <h2 style={h2Style}>Contests</h2>
            {ballotStyle.ballot_eml.contests.length === 0 ? (
                <Empty>(none)</Empty>
            ) : (
                <ul style={{paddingLeft: "1.25rem"}}>
                    {ballotStyle.ballot_eml.contests.map((c) => (
                        <li key={c.id} style={{margin: "0.25rem 0"}}>
                            <NavLink
                                to={`/wb/contest/${c.id}`}
                                style={inlineLinkStyle}
                            >
                                {c.name}
                            </NavLink>{" "}
                            <span style={{color: "#888"}}>
                                <code>{c.id}</code>
                            </span>
                        </li>
                    ))}
                </ul>
            )}
            <h2 style={h2Style}>Assigned voters</h2>
            {assignedVoters === undefined ? (
                <Empty>
                    This snapshot has no <code>assignments</code> map
                    (legacy single-voter import). Voter ↔ ballot-style
                    binding is implicit.
                </Empty>
            ) : assignedVoters.length === 0 ? (
                <Empty>No voters are assigned to this ballot style.</Empty>
            ) : (
                <ul style={{paddingLeft: "1.25rem"}}>
                    {assignedVoters.map((v) => (
                        <li key={v.id} style={{margin: "0.25rem 0"}}>
                            <NavLink
                                to={`/wb/voter/${v.id}`}
                                style={inlineLinkStyle}
                            >
                                {v.displayName}
                            </NavLink>{" "}
                            <span style={{color: "#888"}}>
                                <code>{v.id}</code>
                            </span>
                        </li>
                    ))}
                </ul>
            )}
            <details style={{marginTop: "1.5rem"}}>
                <summary style={{cursor: "pointer", color: "#444"}}>
                    Raw <code>ballot_eml</code> JSON
                </summary>
                <CopyJsonBlock
                    json={JSON.stringify(ballotStyle.ballot_eml, null, 2)}
                />
            </details>
        </>
    )
}

function SecretKeyRow({
    keypair,
    pk,
}: {
    keypair: {pkB64: string; skB64: string} | undefined
    pk: string | undefined
}): JSX.Element {
    if (!keypair) {
        return (
            <em style={{color: "#b00020"}}>
                no keypair registered in this snapshot — the decrypt
                bridge will fall back to a fresh keypair
            </em>
        )
    }
    // Defence in depth: if the registered pk doesn't match the
    // ballot_eml public key, the sk is unusable and we want the
    // operator to know.
    const mismatch = pk != null && pk !== keypair.pkB64
    return (
        <div>
            {mismatch && (
                <p style={{color: "#b00020", margin: "0 0 0.4rem 0"}}>
                    ⚠ Registered pk does not match{" "}
                    <code>ballot_eml.public_key.public_key</code>. The
                    decrypt bridge will not work until they agree.
                </p>
            )}
            <code style={codeBlockStyle}>{keypair.skB64}</code>
        </div>
    )
}

/**
 * `/wb/contest/:id` — detail page for one contest.
 *
 * Surfaces:
 *  - which ballot style + election the contest lives on (NavLinks),
 *  - the contest's voting metadata (voting_type, min/max votes,
 *    winning_candidates_num),
 *  - its candidates,
 *  - and the live per-contest tally aggregated from the workbench
 *    bridge: every cast vote whose `repairedCastVotes[id]` has a
 *    decoded BigUint for this contest is fed to `runElectionTally`
 *    against a synthetic single-contest ballot style. The result is
 *    rendered as the parsed `ContestResult` JSON, which is what the
 *    velvet-wasm tally emits.
 *
 * A contest can in principle appear on multiple ballot styles. The
 * workbench dedupes them in the rail and we pick the first match
 * here too — for the workbench dataset that's fine, and the page
 * shows which ballot style was picked so the operator can see.
 */
export function ContestDetailPage(): JSX.Element {
    const {id} = useParams()
    const contestId = id ?? ""
    const navigate = useNavigate()
    // Live portal slice carries at most one BS per election (the one
    // bound to the active voter), so a contest that only exists on a
    // pool-only BS (e.g. Area B's contest while Area A's voter is
    // active) would be unreachable. Search both sources, live first.
    type PortalBSRow = NonNullable<RootState["ballotStyles"][string]>
    const liveFound = useSelector((s: RootState) => {
        for (const bs of Object.values(s.ballotStyles)) {
            if (!bs) continue
            const c = bs.ballot_eml.contests.find((c) => c.id === contestId)
            if (c) return {contest: c, ballotStyle: bs}
        }
        return null
    })
    const pool = useWorkbench((w) => w.ballotStylePool)
    const found = useMemo<
        {contest: {id: string; name: string} & Record<string, unknown>; ballotStyle: PortalBSRow} | null
    >(() => {
        if (liveFound) return liveFound
        if (!pool) return null
        for (const rows of Object.values(pool)) {
            for (const row of rows) {
                const bs = row as PortalBSRow
                const c = bs.ballot_eml.contests?.find(
                    (c) => c.id === contestId
                )
                if (c) return {contest: c, ballotStyle: bs}
            }
        }
        return null
    }, [liveFound, pool, contestId])
    const election = useSelector((s: RootState) =>
        found ? s.elections[found.ballotStyle.election_id] : undefined
    )
    const castVotes = useSelector((s: RootState) =>
        found ? s.castVotes[found.ballotStyle.election_id] ?? [] : []
    )
    const repaired = useWorkbench((w) => w.repairedCastVotes)
    // Snapshot-wide keypair, used to seed the pipeline page so its
    // encrypt/decrypt stages match what the bridge actually used.
    const keypair = useWorkbench((w) => w.keypair)
    // Decoded BigUint per cast vote for this contest, in cast order.
    // Cast votes whose bridge entry hasn't filled `decodedBigInts`
    // yet (e.g. the decrypt observer hasn't run) are simply absent —
    // `runElectionTally` skips them.
    //
    // We also skip cast votes whose captured ballot style doesn't
    // contain this contest. In a multi-BS election (e.g. the
    // sample-election-config fixture where Area A and Area B each get
    // their own contest on their own BS), `state.castVotes` is keyed
    // by election and so contains votes from *every* BS — but a vote
    // cast against BS-B says nothing about a contest that only lives
    // on BS-A. Without this filter such votes would render here as
    // "(not yet decoded)", which is misleading: the voter never voted
    // on this contest at all. If the bridge entry is missing entirely
    // (hydration race), leave the row in so the operator can still
    // see something is in flight.
    const decodedRows = useMemo(() => {
        if (!found) return []
        const ownBsId = found.ballotStyle.id
        const rows: Array<{
            castVoteId: string
            decoded: string | undefined
        }> = []
        for (const cv of castVotes) {
            const entry = repaired[cv.id]
            if (entry?.ballotStyleId && entry.ballotStyleId !== ownBsId) {
                // Cast against a different ballot style in the same
                // election — that BS does not include this contest.
                continue
            }
            rows.push({
                castVoteId: cv.id,
                decoded: entry?.decodedBigInts?.[contestId],
            })
        }
        return rows
    }, [castVotes, repaired, contestId, found?.ballotStyle.id])
    // Last tally run for this contest is stored in the workbench
    // store (see WorkbenchExtraState.tallyRuns) so it survives the
    // navigation cycle the operator goes through to cast another
    // ballot (Contest → Voter → booth → Review → back). Without
    // lifting it out of component state, the stale-results indicator
    // could never fire in practice because returning to the contest
    // page always re-mounted with a blank slate.
    const cachedRun = useWorkbench((w) => w.tallyRuns?.[contestId])
    const outcome = cachedRun?.outcome ?? null
    const tallyError = cachedRun?.errorMessage ?? null
    const lastTallyFingerprint = cachedRun?.fingerprint ?? null
    const [tallyBusy, setTallyBusy] = useState<boolean>(false)
    // Cheap content-hash of everything that would change the tally
    // output: the set of cast-vote ids in cast order plus the decoded
    // BigUint (or empty string when not yet decoded) for each. A new
    // cast vote, a decrypt completion, a fixture reload, or a
    // re-cast (which removes the prior vote via
    // supersedePriorCastVotes) all change this string. Computing it
    // is O(decodedRows.length) join; for realistic workbench sizes
    // (tens of cast votes) it's sub-millisecond per render.
    const currentTallyFingerprint = useMemo(
        () =>
            decodedRows
                .map((r) => `${r.castVoteId}:${r.decoded ?? ""}`)
                .join("|"),
        [decodedRows]
    )
    // Open this contest in the ballot pipeline pre-filled with one
    // row per captured cast vote. Each row's plaintext cell is
    // pulled from the bridge-captured `selection` (a
    // `BallotSelection = Array<IDecodedVoteContest>`, see
    // ui-core/services/wasm.ts), filtered to this contest's id; the
    // encrypted cell is the cast-vote's `content` envelope; the
    // decrypted cell is the workbench-bridge-decoded BigUint. The
    // operator can then re-run any stage in the pipeline and see
    // computed-vs-captured for every ballot side-by-side.
    //
    // Navigation state is the seed transport: react-router carries
    // it through history without persisting it, which is exactly the
    // semantics we want (a reload of /pipeline should fall back to
    // velvet-wasm fixtures, not re-seed from a stale contest view).
    const handleOpenInPipeline = useCallback(() => {
        if (!found) return
        const ownBsId = found.ballotStyle.id
        const rows: PipelineSeedRow[] = []
        for (const cv of castVotes) {
            const entry = repaired[cv.id]
            if (entry?.ballotStyleId && entry.ballotStyleId !== ownBsId) {
                // Cast against a different BS in this election; that
                // BS does not include this contest.
                continue
            }
            let plaintextJson: string | undefined
            const sel = entry?.selection as
                | Array<{contest_id?: unknown}>
                | undefined
            if (Array.isArray(sel)) {
                const match = sel.find(
                    (c) => (c as {contest_id?: unknown}).contest_id === contestId
                )
                if (match) plaintextJson = JSON.stringify(match, null, 2)
            }
            rows.push({
                label: cv.id.slice(0, 8) + "\u2026",
                plaintextJson,
                encryptedJson:
                    typeof cv.content === "string" && cv.content.length > 0
                        ? prettyJsonOrRaw(cv.content)
                        : undefined,
                // Deliberately NOT seeded from
                // `entry?.decodedBigInts?.[contestId]`. The pipeline
                // is a teaching surface: the operator opens it from a
                // contest to inspect *encrypted* cast votes and is
                // expected to perform the decrypt step themselves so
                // they see the encrypt -> decrypt round-trip. Pre-
                // filling the decrypted BigUint from the upstream
                // bridge would short-circuit that flow.
                decryptedBigInt: undefined,
            })
        }
        const seed: PipelineSeed = {
            contestName: found.contest?.name,
            contestJson: JSON.stringify(found.contest, null, 2),
            pkB64: keypair?.pkB64 ?? "",
            skB64: keypair?.skB64 ?? "",
            rows,
        }
        navigate("/pipeline", {state: seed})
    }, [found, castVotes, repaired, contestId, keypair, navigate])
    const handleRunTally = useCallback(async () => {
        if (!found) return
        const fingerprint = currentTallyFingerprint
        setTallyBusy(true)
        const decodedByCastVote = decodedRows.map((r) =>
            r.decoded ? {[contestId]: r.decoded} : {}
        )
        // Run the full ballot-style tally and pick the outcome for
        // this contest. We could pass a one-contest projection, but
        // passing the real ballot style keeps the tally call honest
        // about what's actually on the ballot.
        let nextOutcome: ContestTallyOutcome | null = null
        let nextError: string | null = null
        try {
            const outcomes = await runElectionTally(
                found.ballotStyle,
                decodedByCastVote
            )
            nextOutcome =
                outcomes.find((o) => o.contestId === contestId) ?? null
        } catch (e) {
            nextError = e instanceof Error ? e.message : String(e)
        }
        // Record the fingerprint we ran against whether the run
        // succeeded or failed: re-pressing the button on the same
        // inputs would just reproduce the same error, so the stale
        // notice would be misleading.
        recordTallyRun(contestId, {
            fingerprint,
            outcome: nextOutcome,
            errorMessage: nextError,
            ranAt: new Date().toISOString(),
        })
        setTallyBusy(false)
    }, [found, decodedRows, contestId, currentTallyFingerprint])
    if (!found) {
        return (
            <>
                <h1>Contest not found</h1>
                <p>
                    <code>{contestId || "(missing id)"}</code>
                </p>
            </>
        )
    }
    const {contest, ballotStyle} = found
    const contestName = contest.name ?? "(unnamed contest)"
    return (
        <>
            <h1>{contestName}</h1>
            <p style={{color: "#666"}}>
                <code>{contestId}</code> &middot; Contest
            </p>
            <dl style={dlStyle}>
                <DlRow label="Election">
                    {election?.name ? `${election.name} — ` : ""}
                    <code>{ballotStyle.election_id}</code>
                </DlRow>
                <DlRow label="Ballot style">
                    <NavLink
                        to={`/wb/ballot-style/${ballotStyle.id}`}
                        style={inlineLinkStyle}
                    >
                        <code>{ballotStyle.id}</code>
                    </NavLink>
                </DlRow>
                <DlRow label="Voting type">
                    {/* Velvet contests carry `counting_algorithm`
                      * (e.g. `plurality-at-large`) instead of the
                      * portal's `voting_type` enum; surface whichever
                      * is present so the operator isn't always
                      * staring at "(unspecified)" on velvet imports. */}
                    {(contest.voting_type as string | undefined) ??
                        (contest.counting_algorithm as
                            | string
                            | undefined) ??
                        "(unspecified)"}
                </DlRow>
                <DlRow label="Min / max votes">
                    {contest.min_votes} / {contest.max_votes}
                </DlRow>
                <DlRow label="Winners">
                    {contest.winning_candidates_num}
                </DlRow>
                <DlRow label="Encrypted">
                    {contest.is_encrypted ? "yes" : "no"}
                </DlRow>
            </dl>
            <h2 style={h2Style}>Candidates</h2>
            {contest.candidates.length === 0 ? (
                <Empty>(none)</Empty>
            ) : (
                <ul style={{paddingLeft: "1.25rem"}}>
                    {contest.candidates.map((cand) => (
                        <li key={cand.id} style={{margin: "0.25rem 0"}}>
                            {cand.name ?? "(unnamed)"}{" "}
                            <span style={{color: "#888"}}>
                                <code>{cand.id}</code>
                            </span>
                        </li>
                    ))}
                </ul>
            )}
            <div
                style={{
                    display: "flex",
                    alignItems: "baseline",
                    gap: "0.75rem",
                }}
            >
                <h2 style={h2Style}>Tally</h2>
                <button
                    type="button"
                    onClick={handleOpenInPipeline}
                    style={secondaryButtonStyle}
                    title="Open this contest's captured ballots in the
 encrypt/decrypt/decode pipeline for round-trip inspection"
                >
                    Open in ballot pipeline
                </button>
            </div>
            <ContestTallyView
                outcome={outcome}
                error={tallyError}
                decodedRows={decodedRows}
                busy={tallyBusy}
                stale={
                    lastTallyFingerprint !== null &&
                    lastTallyFingerprint !== currentTallyFingerprint
                }
                hasRun={lastTallyFingerprint !== null}
                onRun={handleRunTally}
            />
        </>
    )
}

function ContestTallyView({
    outcome,
    error,
    decodedRows,
    busy,
    stale,
    hasRun,
    onRun,
}: {
    outcome: ContestTallyOutcome | null
    error: string | null
    decodedRows: Array<{castVoteId: string; decoded: string | undefined}>
    busy: boolean
    stale: boolean
    hasRun: boolean
    onRun: () => void
}): JSX.Element {
    const decodedCount = decodedRows.filter((r) => !!r.decoded).length
    return (
        <>
            <p style={{margin: "0.3rem 0", color: "#444"}}>
                {decodedCount} of {decodedRows.length} cast vote
                {decodedRows.length === 1 ? "" : "s"} decoded for this
                contest.
            </p>
            {/*
              * Tally never runs automatically: re-tallying on every
              * cast vote / decrypt-completion is both wasteful and
              * misleading (an operator exploring a fixture wants
              * deliberate control over when results are computed).
              * The button is always enabled; staleness is signalled
              * separately and does not block re-runs, so the operator
              * is never stuck with no way to refresh.
              */}
            <div style={{margin: "0.5rem 0"}}>
                <button
                    type="button"
                    onClick={onRun}
                    disabled={busy}
                    style={primaryButtonStyle}
                >
                    {busy
                        ? "Running tally…"
                        : hasRun
                          ? "Re-run tally"
                          : "Run tally"}
                </button>
            </div>
            {stale && (
                <p
                    style={{
                        margin: "0.3rem 0",
                        color: "#7a5d00",
                        background: "#fff8d6",
                        border: "1px solid #e6cf6a",
                        padding: "0.4rem 0.6rem",
                        borderRadius: 4,
                        fontSize: "0.85rem",
                    }}
                >
                    Inputs have changed since the last run — the
                    results below are out of date. Press{" "}
                    <strong>Re-run tally</strong> to refresh.
                </p>
            )}
            {error ? (
                <p style={{color: "#b00020"}}>
                    Tally failed: <code>{error}</code>
                </p>
            ) : !hasRun ? (
                <Empty>
                    Press <strong>Run tally</strong> to compute results
                    from the decoded ballots above.
                </Empty>
            ) : outcome == null ? (
                <Empty>(running…)</Empty>
            ) : outcome.status === "no-data" ? (
                <Empty>
                    No decoded ballots yet. Cast votes from the booth
                    appear here once the bridge has decrypted them.
                </Empty>
            ) : outcome.status === "error" ? (
                <p style={{color: "#b00020"}}>
                    Tally failed: <code>{outcome.errorMessage}</code>
                </p>
            ) : (
                <pre style={tallyResultStyle}>
                    <code>{JSON.stringify(outcome.result, null, 2)}</code>
                </pre>
            )}
            <details style={{marginTop: "1rem"}}>
                <summary style={{cursor: "pointer", color: "#444"}}>
                    Decrypted BigUints per cast vote
                </summary>
                {decodedRows.length === 0 ? (
                    <Empty>(none)</Empty>
                ) : (
                    <ul style={{paddingLeft: "1.25rem"}}>
                        {decodedRows.map((r) => (
                            <li
                                key={r.castVoteId}
                                style={{
                                    margin: "0.25rem 0",
                                    fontFamily: "monospace",
                                    fontSize: "0.8rem",
                                }}
                            >
                                <span style={{color: "#888"}}>
                                    {r.castVoteId}
                                </span>
                                {" → "}
                                {r.decoded ? (
                                    <span style={{wordBreak: "break-all"}}>
                                        {r.decoded}
                                    </span>
                                ) : (
                                    // After the strict-keypair import
                                    // fix, this state strictly means
                                    // the bridge tried to decrypt and
                                    // either the cast vote had no
                                    // `content` or the decrypt threw
                                    // (logged to the console). It is
                                    // not "in progress" — nothing
                                    // retries decrypt after capture.
                                    <em
                                        style={{color: "#b22222"}}
                                        title="Decrypt failed: empty ballot content or decrypt error (see console)."
                                    >
                                        (decrypt failed)
                                    </em>
                                )}
                            </li>
                        ))}
                    </ul>
                )}
            </details>
        </>
    )
}

const tallyResultStyle: React.CSSProperties = {
    padding: "0.6rem 0.8rem",
    background: "#f4f4f4",
    border: "1px solid #ddd",
    borderRadius: 3,
    fontSize: "0.8rem",
    overflowX: "auto",
    maxWidth: "44rem",
}

/**
 * `/wb/voter/:id` — detail page for one voter persona.
 *
 * The voter directory is global to the workbench, so a voter isn't
 * scoped to one election. This page surfaces:
 *  - the voter's id, displayName, and notes;
 *  - one "Cast a ballot in <election>" CTA per ballot style available
 *    in the current snapshot. Each button (a) flips
 *    `workbench.activeVoterId` to this voter so the cast-vote
 *    observer can attribute the next cast vote, then (b) navigates
 *    to the booth start route for that election.
 *  - the cast votes attributed to this voter (via the workbench
 *    `castBy` ledger — the portal's `voter_id_string` is always null
 *    under DISABLE_AUTH, so castBy is the only source of attribution).
 *    For each cast vote, the decoded BigUints per contest are listed
 *    with NavLinks to the contest detail page.
 */
export function VoterDetailPage(): JSX.Element {
    const {id} = useParams()
    const voterId = id ?? ""
    const navigate = useNavigate()
    const voter = useWorkbench((w) =>
        w.voters.find((v) => v.id === voterId)
    )
    // Eligible ballot styles for this voter come from the workbench
    // overlay (Phase 1 eligibility), NOT the portal `ballotStyles`
    // slice — that slice only ever holds the BS for the *currently
    // active* session, so reading it would show whatever the last
    // impersonated voter saw, not what THIS voter is eligible for.
    //
    // When the overlay is absent (older snapshots, single-voter
    // imports without an `assignments` map), fall back to the portal
    // slice so the page still works.
    type PortalBSRow = NonNullable<RootState["ballotStyles"][string]>
    const ballotStylePool = useWorkbench((w) => w.ballotStylePool)
    const assignments = useWorkbench((w) => w.assignments)
    const portalSliceStyles = useSelector((s: RootState) =>
        Object.values(s.ballotStyles).filter(
            (b): b is PortalBSRow => !!b
        )
    )
    const ballotStyles = useMemo<PortalBSRow[]>(() => {
        if (!ballotStylePool || !assignments) {
            return portalSliceStyles
        }
        const assignedIds = new Set(assignments[voterId] ?? [])
        if (assignedIds.size === 0) return []
        const out: PortalBSRow[] = []
        for (const rows of Object.values(ballotStylePool)) {
            for (const row of rows) {
                const id = (row as {id?: unknown}).id
                if (typeof id === "string" && assignedIds.has(id)) {
                    out.push(row as PortalBSRow)
                }
            }
        }
        return out
    }, [ballotStylePool, assignments, portalSliceStyles, voterId])
    const elections = useSelector((s: RootState) => s.elections)
    const castVotesByElection = useSelector((s: RootState) => s.castVotes)
    const castBy = useWorkbench((w) => w.castBy)
    const repaired = useWorkbench((w) => w.repairedCastVotes)
    // Rows for cast votes attributed to this voter, across all
    // elections. We scan castBy rather than state.castVotes because
    // castBy is the smaller, voter-keyed table.
    const voterCastVotes = useMemo(() => {
        const out: Array<{
            castVoteId: string
            electionId: string
            electionName: string | undefined
            decoded: Record<string, string>
            createdAt: string | null | undefined
        }> = []
        for (const [castVoteId, vId] of Object.entries(castBy)) {
            if (vId !== voterId) continue
            const entry = repaired[castVoteId]
            const electionId = entry?.electionId ?? ""
            // The cast-vote record itself lives in state.castVotes,
            // keyed by electionId.
            const cv = electionId
                ? (castVotesByElection[electionId] ?? []).find(
                      (x) => x.id === castVoteId
                  )
                : undefined
            out.push({
                castVoteId,
                electionId,
                electionName: electionId
                    ? elections[electionId]?.name
                    : undefined,
                decoded: entry?.decodedBigInts ?? {},
                createdAt: cv?.created_at,
            })
        }
        // Most-recent first when timestamps are available.
        out.sort((a, b) => {
            const at = a.createdAt ?? ""
            const bt = b.createdAt ?? ""
            return bt.localeCompare(at)
        })
        return out
    }, [castBy, repaired, voterId, castVotesByElection, elections])
    // Lookup table for contest names so cast-vote rows can display
    // "Favourite shape (Area B) → 4" instead of the bare contest UUID.
    // We scan the workbench ballot-style pool (when present — it's
    // the source of truth for the multi-BS case) and fall back to
    // whatever's in the portal slice.
    const contestNameById = useMemo(() => {
        const map: Record<string, string> = {}
        const collectFrom = (bs: PortalBSRow | undefined): void => {
            if (!bs) return
            for (const c of bs.ballot_eml.contests) {
                if (c.name && !map[c.id]) map[c.id] = c.name
            }
        }
        if (ballotStylePool) {
            for (const rows of Object.values(ballotStylePool)) {
                for (const row of rows) collectFrom(row as PortalBSRow)
            }
        }
        for (const bs of portalSliceStyles) collectFrom(bs)
        return map
    }, [ballotStylePool, portalSliceStyles])
    if (!voter) {
        return (
            <>
                <h1>Voter not found</h1>
                <p>
                    <code>{voterId || "(missing id)"}</code>
                </p>
            </>
        )
    }
    const startVotingAs = (bs: (typeof ballotStyles)[number]) => {
        setActiveVoter(voter.id)
        navigate(
            `/tenant/${bs.tenant_id}/event/${bs.election_event_id}` +
                `/election/${bs.election_id}/start`
        )
    }
    return (
        <>
            <h1>{voter.displayName}</h1>
            <p style={{color: "#666"}}>
                <code>{voter.id}</code> &middot; Voter
            </p>
            {voter.notes && (
                <p style={{color: "#444"}}>{voter.notes}</p>
            )}
            <h2 style={h2Style}>Vote as {voter.displayName}</h2>
            {ballotStyles.length === 0 ? (
                <Empty>
                    {ballotStylePool && assignments
                        ? "This voter has no ballot-style assignments in the current snapshot."
                        : "No ballot styles in this snapshot."}
                </Empty>
            ) : (
                <ul style={{paddingLeft: 0, listStyle: "none"}}>
                    {ballotStyles.map((bs) => {
                        const election = elections[bs.election_id]
                        const label =
                            election?.name ??
                            `(unnamed election ${bs.election_id})`
                        // Workbench always allows unlimited revotes;
                        // each new cast overwrites the previous one
                        // in the slice (see persistence subscriber's
                        // supersedePriorCastVotes). Reflect that in
                        // the label so operators know subsequent
                        // clicks replace rather than stack.
                        const hasPriorCast = voterCastVotes.some(
                            (row) => row.electionId === bs.election_id
                        )
                        const buttonLabel = hasPriorCast
                            ? `Recast in ${label} (overwrites previous) →`
                            : `Cast a ballot in ${label} →`
                        return (
                            <li
                                key={bs.id}
                                style={{margin: "0.4rem 0"}}
                            >
                                <button
                                    type="button"
                                    style={primaryButtonStyle}
                                    onClick={() => startVotingAs(bs)}
                                >
                                    {buttonLabel}
                                </button>{" "}
                                <NavLink
                                    to={`/wb/ballot-style/${bs.id}`}
                                    style={{...inlineLinkStyle, color: "#888"}}
                                    title="Open this ballot style's detail page"
                                >
                                    <code>{bs.id}</code>
                                </NavLink>
                            </li>
                        )
                    })}
                </ul>
            )}
            <h2 style={h2Style}>Cast votes</h2>
            {voterCastVotes.length === 0 ? (
                <Empty>
                    No cast votes attributed to this voter yet. Start a
                    ballot above to cast one.
                </Empty>
            ) : (
                <ul style={{paddingLeft: 0, listStyle: "none"}}>
                    {voterCastVotes.map((row) => (
                        <li
                            key={row.castVoteId}
                            style={castVoteRowStyle}
                        >
                            <VoterCastVoteRow
                                row={row}
                                contestNameById={contestNameById}
                            />
                        </li>
                    ))}
                </ul>
            )}
        </>
    )
}

function VoterCastVoteRow({
    row,
    contestNameById,
}: {
    row: {
        castVoteId: string
        electionId: string
        electionName: string | undefined
        decoded: Record<string, string>
        createdAt: string | null | undefined
    }
    contestNameById: Record<string, string>
}): JSX.Element {
    const decodedEntries = Object.entries(row.decoded)
    return (
        <>
            <div style={{fontSize: "0.85rem", color: "#444"}}>
                <strong>{row.electionName ?? "(unknown election)"}</strong>
                {row.createdAt && (
                    <>
                        {" "}
                        &middot;{" "}
                        <span title={row.createdAt}>
                            {formatTimestamp(row.createdAt)}
                        </span>
                    </>
                )}
            </div>
            <div
                style={{
                    fontFamily: "monospace",
                    fontSize: "0.75rem",
                    color: "#888",
                    margin: "0.2rem 0",
                }}
            >
                {row.castVoteId}
            </div>
            {decodedEntries.length === 0 ? (
                // After the strict-keypair import fix, an empty
                // decoded map for a captured cast vote means every
                // contest's decrypt either short-circuited on missing
                // `content` or threw (logged). The bridge does not
                // retry, so this is not "in progress".
                <Empty>(decrypt failed — see console)</Empty>
            ) : (
                <ul
                    style={{
                        paddingLeft: "1.25rem",
                        margin: "0.3rem 0 0 0",
                    }}
                >
                    {decodedEntries.map(([contestId, big]) => {
                        const label =
                            contestNameById[contestId] ?? contestId
                        return (
                            <li
                                key={contestId}
                                style={{
                                    margin: "0.2rem 0",
                                    fontFamily: "monospace",
                                    fontSize: "0.8rem",
                                }}
                            >
                                <NavLink
                                    to={`/wb/contest/${contestId}`}
                                    style={inlineLinkStyle}
                                    title={contestId}
                                >
                                    {label}
                                </NavLink>
                                {" → "}
                                <span style={{wordBreak: "break-all"}}>
                                    {big}
                                </span>
                            </li>
                        )
                    })}
                </ul>
            )}
        </>
    )
}

function formatTimestamp(raw: string): string {
    // Cast-vote `created_at` is ISO8601 from the portal; falling back
    // to the raw string is fine for anything weird.
    const t = Date.parse(raw)
    if (Number.isNaN(t)) return raw
    return new Date(t).toLocaleString()
}

const castVoteRowStyle: React.CSSProperties = {
    margin: "0.6rem 0",
    padding: "0.6rem 0.8rem",
    background: "#fafafa",
    border: "1px solid #e4e4e4",
    borderRadius: 4,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const navLinkStyle = ({isActive}: {isActive: boolean}): React.CSSProperties => ({
    display: "block",
    padding: "0.15rem 0.3rem",
    borderRadius: 3,
    color: "#222",
    textDecoration: "none",
    background: isActive ? "#dde9ff" : "transparent",
    fontWeight: isActive ? 600 : 400,
})

const listStyle: React.CSSProperties = {
    listStyle: "none",
    padding: 0,
    margin: "0.25rem 0",
}

function SectionHeading({children}: {children: React.ReactNode}): JSX.Element {
    return (
        <h3
            style={{
                fontSize: "0.7rem",
                textTransform: "uppercase",
                letterSpacing: "0.05em",
                color: "#666",
                margin: "0 0 0.4rem 0",
            }}
        >
            {children}
        </h3>
    )
}

function SubHeading({children}: {children: React.ReactNode}): JSX.Element {
    return (
        <h4
            style={{
                fontSize: "0.75rem",
                color: "#444",
                margin: "0.75rem 0 0.25rem 0",
            }}
        >
            {children}
        </h4>
    )
}

function Empty({children}: {children: React.ReactNode}): JSX.Element {
    return <div style={{color: "#999", fontStyle: "italic"}}>{children}</div>
}

/** Try to pretty-print JSON; fall back to the raw string if parsing
 *  fails. Used when seeding the pipeline with a `castVote.content`
 *  envelope that we want to display readably without breaking on
 *  malformed input. */
function prettyJsonOrRaw(s: string): string {
    try {
        return JSON.stringify(JSON.parse(s), null, 2)
    } catch {
        return s
    }
}

// --- Detail-page presentational helpers -----------------------------------

const dlStyle: React.CSSProperties = {
    margin: "1rem 0",
    display: "grid",
    gridTemplateColumns: "max-content 1fr",
    gap: "0.4rem 1.5rem",
    alignItems: "baseline",
    fontSize: "0.9rem",
}

function DlRow({
    label,
    children,
}: {
    label: string
    children: React.ReactNode
}): JSX.Element {
    return (
        <>
            <dt style={{color: "#666"}}>{label}</dt>
            <dd style={{margin: 0}}>{children}</dd>
        </>
    )
}

const primaryButtonStyle: React.CSSProperties = {
    padding: "0.5rem 1rem",
    background: "#1976d2",
    color: "white",
    border: 0,
    borderRadius: 4,
    fontSize: "0.9rem",
    cursor: "pointer",
}

const secondaryButtonStyle: React.CSSProperties = {
    padding: "0.3rem 0.8rem",
    background: "#fff",
    color: "#222",
    border: "1px solid #bbb",
    borderRadius: 4,
    fontSize: "0.85rem",
    cursor: "pointer",
}

const codeBlockStyle: React.CSSProperties = {
    display: "inline-block",
    padding: "0.3rem 0.5rem",
    background: "#f4f4f4",
    border: "1px solid #ddd",
    borderRadius: 3,
    fontSize: "0.8rem",
    wordBreak: "break-all",
    maxWidth: "44rem",
}

const h2Style: React.CSSProperties = {
    fontSize: "1rem",
    margin: "1.5rem 0 0.5rem 0",
    color: "#222",
}

const inlineLinkStyle: React.CSSProperties = {
    color: "#1976d2",
    textDecoration: "none",
}

/** A non-clickable structural label used for tree nodes that have no
 *  detail page of their own (tenant, event, election, "Contests",
 *  "Ballot styles"). The tooltip surfaces the underlying id for
 *  debugging. */
function NodeLabel(props: {
    children: React.ReactNode
    title?: string
}): JSX.Element {
    return (
        <div
            title={props.title}
            style={{
                padding: "0.15rem 0.3rem",
                color: "#444",
                fontWeight: 500,
            }}
        >
            {props.children}
        </div>
    )
}

// `getCurrentParentId()` is module-level state in persistence.ts, not
// React state, so subscribe via useSyncExternalStore. We piggyback on
// the workbench store's listener since `currentParentId` only changes
// inside `hydrateFromSnapshot` and `saveCheckpoint`, which both also
// dispatch through the workbench store. Practical effect: any change
// that flips `currentParentId` also bumps `workbenchStore`, so the
// rail re-renders.
function useCurrentParentId(): string | null {
    return useWorkbench(() => getCurrentParentId())
}

// Surface checkpoint list reactively. `listCheckpoints` reads
// localStorage, which has no native React-friendly subscription, but
// checkpoints are written by `saveCheckpoint` which also bumps the
// workbench store. Same trick as `useCurrentParentId`, with a cache
// so the returned array is identity-stable across calls that produce
// the same content (useSyncExternalStore requires `Object.is`-stable
// snapshots to avoid infinite re-renders).
let cachedCheckpointsKey = ""
let cachedCheckpoints: CheckpointMeta[] = []
function getCheckpointsCached(): CheckpointMeta[] {
    const next = listCheckpoints()
    const key = JSON.stringify(next)
    if (key !== cachedCheckpointsKey) {
        cachedCheckpointsKey = key
        cachedCheckpoints = next
    }
    return cachedCheckpoints
}
function useCheckpointList(): CheckpointMeta[] {
    return useSyncExternalStore(
        // Re-read on any workbench mutation. `saveCheckpoint` bumps
        // workbench state via the trailing writeSnapshot's parentId
        // change; `deleteCheckpoint` will need to be wired similarly
        // when its UI lands.
        (cb) => subscribeWorkbench(cb),
        getCheckpointsCached,
        getCheckpointsCached
    )
}
