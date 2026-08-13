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
import {Fragment, useCallback, useEffect, useMemo, useState, useSyncExternalStore} from "react"
import {getWorkbenchState, subscribeWorkbench, useWorkbench} from "./workbenchStore"
import {
    buildCurrentSnapshot,
    bundledId,
    checkpointId,
    deleteCheckpoint,
    getCurrentParentId,
    listCheckpoints,
    loadSnapshotById,
    loadSnapshotViaReload,
    materializeAsCheckpoint,
    normalizeCheckpointName,
    canonicalCompareJson,
    readCheckpointSnapshot,
    saveCheckpoint,
    type CheckpointMeta,
    type PersistedSnapshot,
} from "./persistence"
import {BUNDLED_SNAPSHOTS} from "./fixtures/bundledSnapshots"
import type {PipelineSeed, PipelineSeedRow} from "./BallotPipeline"
import type {TallySeed} from "./TallyPage"
import {decodeBigIntToDecodedVoteContest} from "./tally"
import {setActiveVoter} from "./workbenchStore"
import {importPortalBallotStyle} from "./import/portalBallotStyleImport"
import {importVelvetElection} from "./import/velvetElectionImport"
import {ContestPolicyOverridesPanel} from "./ContestPolicyOverridesPanel"
import buildInfo, {
    type WorkbenchBuildInfo,
} from "virtual:workbench-build-info"
// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

const RAIL_WIDTH = 280
const railStyle: React.CSSProperties = {
    width: RAIL_WIDTH,
    minWidth: RAIL_WIDTH,
    borderRight: "1px solid #3a3a3a",
    background: "#1e1e1e",
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
                borderTop: "1px solid #3a3a3a",
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
    const isDirty = useIsWorkingDirty()
    const bundled = Object.keys(BUNDLED_SNAPSHOTS).sort()
    const {roots, orphans} = buildProvenanceForest(bundled, checkpoints)
    return (
        <section>
            <SectionHeading>Snapshots</SectionHeading>
            {/* The working copy is intentionally *not* a node in the
                forest — per the locked design, the auto-resume slot
                stays out of the provenance tree. The working-copy
                overview lives at /wb (the index route), where its
                pinned row carries the Save… action. */}
            <ul style={listStyle}>
                {roots.map((n) => (
                    <ProvenanceTreeNode
                        key={n.id}
                        node={n}
                        currentParent={currentParent}
                        isDirty={isDirty}
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
                                isDirty={isDirty}
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
    isDirty: boolean
    depth: number
}): JSX.Element {
    const {node, currentParent, isDirty, depth} = props
    const icon = node.kind === "bundled" ? "▣" : "◇"
    const isActive = currentParent === node.id
    return (
        <li style={{marginLeft: depth === 0 ? 0 : "1rem"}}>
            <NavLink
                to={`/wb/snapshot/${encodeURIComponent(node.id)}`}
                style={navLinkStyle}
                title={
                    isActive && isDirty
                        ? `${node.id} — currently loaded (working copy has unsaved changes)`
                        : isActive
                        ? `${node.id} — currently loaded`
                        : node.id
                }
            >
                <span style={{marginRight: "0.3rem"}}>{icon}</span>
                <span
                    style={{
                        fontWeight: isActive ? 600 : 400,
                    }}
                >
                    {node.label}
                </span>
                {isActive && (
                    <span
                        style={{
                            marginLeft: "0.3rem",
                            color: isDirty ? "#f0c200" : "#4ade80",
                        }}
                        aria-label={
                            isDirty
                                ? "active snapshot, working copy modified"
                                : "active snapshot"
                        }
                    >
                        {isDirty ? "●*" : "●"}
                    </span>
                )}
            </NavLink>
            {node.children.length > 0 && (
                <ul style={listStyle}>
                    {node.children.map((c) => (
                        <ProvenanceTreeNode
                            key={c.id}
                            node={c}
                            currentParent={currentParent}
                            isDirty={isDirty}
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
                // Wipe + reload via an on-the-fly checkpoint: write
                // the imported snapshot to the checkpoint index
                // under a timestamped name, then point the auto-
                // resume slot at it as parent. After the reload the
                // working copy's `currentParentId` is the new
                // checkpoint id, so the imported state has a stable
                // identity in the rail / dirty-check infrastructure
                // (the same path bundled snapshots and saved
                // checkpoints take). See LIFTING.md section J.
                const ckptId = materializeAsCheckpoint(
                    parsed,
                    `imported-snapshot-${formatImportTimestamp()}`
                )
                loadSnapshotViaReload(parsed, ckptId)
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
            // Same auto-checkpoint trick as the raw-snapshot path
            // above: give the imported state an identity in the
            // checkpoint index so the rail's active-snapshot
            // highlight has something to point at after reload.
            const ckptId = materializeAsCheckpoint(
                snap,
                `imported-${
                    importMode === "ballotStyle"
                        ? "ballot-style"
                        : "velvet"
                }-${formatImportTimestamp()}`
            )
            loadSnapshotViaReload(snap, ckptId)
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
    const onDeleteCheckpoint = (name: string): void => {
        // Permanent: removes the checkpoint from localStorage. The
        // workbench overlay's parent pointer is left alone, so any
        // working copy forked from this checkpoint will appear under
        // the "⚠ Detached" group in the rail.
        if (
            !window.confirm(
                `Delete checkpoint "${name}" permanently?\n\n` +
                    `This removes it from localStorage. Cannot be undone.`
            )
        ) {
            return
        }
        deleteCheckpoint(name)
    }

    // Bundled snapshots are immutable in-memory: the dictionary is
    // built once at module load from `import.meta.glob` and never
    // mutated, so a plain sorted key list is fine.
    const bundledIds = Object.keys(BUNDLED_SNAPSHOTS).sort()

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
    const bundledEntries = bundledIds
        .map((name) => [name, BUNDLED_SNAPSHOTS[name]] as const)
        .filter(([, snap]) => !!snap)
        .sort((a, b) => a[0].localeCompare(b[0]))
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
            <p style={{color: "#999"}}>
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
                                    <div
                                        style={{
                                            display: "flex",
                                            gap: "0.4rem",
                                        }}
                                    >
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
                                        {row.kind === "checkpoint" && (
                                            <button
                                                type="button"
                                                style={{
                                                    ...secondaryButtonStyle,
                                                    color:
                                                        row.id === parentId
                                                            ? "#999"
                                                            : "#ef4444",
                                                    borderColor:
                                                        row.id === parentId
                                                            ? "#3a3a3a"
                                                            : "#ef4444",
                                                    opacity:
                                                        row.id === parentId
                                                            ? 0.6
                                                            : 1,
                                                    cursor:
                                                        row.id === parentId
                                                            ? "not-allowed"
                                                            : "pointer",
                                                }}
                                                disabled={
                                                    row.id === parentId
                                                }
                                                title={
                                                    row.id === parentId
                                                        ? "Can't delete the active snapshot — load a different one first."
                                                        : "Permanently delete from localStorage"
                                                }
                                                onClick={() =>
                                                    onDeleteCheckpoint(
                                                        row.name
                                                    )
                                                }
                                            >
                                                Delete
                                            </button>
                                        )}
                                    </div>
                                )}
                            </td>
                        </tr>
                    ))}
                </tbody>
            </table>
            {error && (
                <p style={{color: "#ef4444", marginTop: "0.5rem"}}>
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
                            <p style={{color: "#999", marginTop: 0}}>
                                Paste a full <code>PersistedSnapshot</code>{" "}
                                (same shape as the <em>Bundled JSON</em>{" "}
                                block on any snapshot detail page). It
                                is first saved as a timestamped
                                checkpoint, then loaded into the
                                working copy with that checkpoint as
                                its provenance parent — so the import
                                survives a reset and appears in the
                                snapshot tree.
                            </p>
                        )}
                        {importMode === "ballotStyle" && (
                            <p style={{color: "#999", marginTop: 0}}>
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
                            <p style={{color: "#999", marginTop: 0}}>
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
                                    color: "#ef4444",
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
        </>
    )
}

/**
 * Standalone diagnostics page rendered at `/diagnostics`. Two
 * sections, both troubleshooting/reproduction surfaces:
 *   1. Build provenance ({@link BuildStatusCard}) — git SHA + wasm
 *      artifact list, sourced at build time from the
 *      `virtual:workbench-build-info` module.
 *   2. Live working-copy state, serialised in the same
 *      {@link PersistedSnapshot} JSON shape that the snapshots
 *      overview's "Import snapshot JSON…" textarea accepts. Mirrors
 *      the bundled snapshot's "Bundled JSON" disclosure (collapsed
 *      by default, Copy button on top, scrollable `<pre>` body).
 *
 * Lives behind a less-prominent right-aligned nav link rather than
 * cluttering the snapshots overview (which is otherwise pure
 * snapshot/election data).
 */
export function DiagnosticsPage(): JSX.Element {
    const store = useStore()
    // Subscribe to redux *and* the workbench overlay so the displayed
    // JSON stays in sync as the operator runs the pipeline / votes
    // through the booth / loads a different snapshot. Both
    // `store.getState` and `getWorkbenchState` return identity-stable
    // refs that change only on a real mutation, so this composes
    // safely with `useSyncExternalStore` (no infinite-loop risk).
    const reduxState = useSyncExternalStore(
        store.subscribe,
        store.getState
    ) as RootState
    const workbenchState = useSyncExternalStore(
        subscribeWorkbench,
        getWorkbenchState
    )
    const currentSnapshotJson = useMemo(
        () =>
            JSON.stringify(buildCurrentSnapshot(reduxState), null, 2),
        // `workbenchState` is read inside `buildCurrentSnapshot` via
        // `getWorkbenchState()`; we still list it here so the
        // recomputation fires when the workbench overlay mutates
        // without a redux dispatch (e.g. captured ballots).
        [reduxState, workbenchState]
    )
    return (
        <div style={{padding: "1.5rem 2rem"}}>
            <h1 style={{margin: "0 0 0.5rem 0"}}>Diagnostics</h1>
            <p style={{color: "#999", margin: "0 0 1rem 0"}}>
                Build provenance for the wasm artifacts baked into this
                workbench bundle (sourced at build time from the{" "}
                <code>virtual:workbench-build-info</code> module), plus
                the live workbench state in importable JSON form.
            </p>
            <BuildStatusCard />
            <LiftedSourceDriftSection />
            <details style={diagnosticsCardStyle}>
                <summary
                    style={{
                        ...diagnosticsCardHeaderStyle,
                        cursor: "pointer",
                        marginBottom: 0,
                    }}
                >
                    <strong>Current workbench state</strong>
                    <span style={{color: "#888", fontSize: "0.8rem"}}>
                        click to expand
                    </span>
                </summary>
                <p
                    style={{
                        color: "#999",
                        fontSize: "0.85rem",
                        margin: "0.5rem 0",
                    }}
                >
                    Paste into <em>Import snapshot JSON…</em> on the
                    snapshots page to reproduce. Same shape as a bundled
                    snapshot's <em>Bundled JSON</em> block, except{" "}
                    <code>parentId</code> is preserved so the receiving
                    workbench can re-attach the lineage. Drops into{" "}
                    <code>src/fixtures/snapshots/</code> too (strip{" "}
                    <code>parentId</code> first for a root bundled fixture).
                </p>
                <CopyJsonBlock json={currentSnapshotJson} />
            </details>
        </div>
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
/**
 * Which sequent-core the lifted booth is running. Rendered because the
 * booth and the tally half get sequent-core by different routes — the
 * booth through this import, the tally by compiling the crate into
 * velvet-wasm — and when they disagree there is no error, only wrong
 * numbers. `local` (the default) is the setting where they match.
 */
function SequentCoreSourceLine(): JSX.Element {
    const sc = buildInfo.sequentCore
    const isLocal = sc.source === "local"
    return (
        <p style={diagnosticsHintStyle}>
            <strong>Booth sequent-core:</strong>{" "}
            <span style={{color: isLocal ? "#4ade80" : "#f0c200"}}>
                {isLocal ? "local build" : "committed tarball"}
            </span>{" "}
            — <code>{sc.resolvedFrom}</code>
            {sc.builtAt && (
                <>
                    , built {humanAge(Date.now() - new Date(sc.builtAt).getTime())} ago
                </>
            )}
            {!isLocal && (
                <span style={{fontStyle: "italic", marginLeft: "0.4rem"}}>
                    (WORKBENCH_SEQUENT_CORE=tgz — may disagree with the tally half)
                </span>
            )}
        </p>
    )
}

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
            <SequentCoreSourceLine />
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
                                            color: "#999",
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
            {buildInfo.git.sha && (
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

/**
 * Drift surface for every tree the workbench shares with production —
 * the lifted `voting-portal/src/`, the aliased `ui-core` /
 * `ui-essentials`, and the Rust crates (`velvet`, `strand`,
 * `sequent-core`) this branch modified. See the embedding-strategy
 * table in the workbench README for which strategy each one uses.
 *
 * Every row is the same question asked of a different subtree: *what
 * has this branch changed in code it shares with production, and is
 * all of it documented?* Each diff is `HEAD` vs the merge-base with
 * `origin/main`, so nothing can be edited quietly — a concession that
 * isn't in LIFTING.md section L shows up here as an unexplained row.
 *
 * Note the baseline moves: once main is merged into this branch the
 * merge-base advances to the merged commit and each diff collapses to
 * the branch's own edits. A suspiciously large diff right after a
 * merge usually means the merge hasn't been committed yet.
 *
 * There is no longer a tally-lift section. The tally components used
 * to be *copied* from admin-portal into ui-essentials and were diffed
 * path-against-path; that copy was deleted once upstream shipped its
 * own tally visualization, which the workbench now imports unmodified.
 * Nothing is copied any more, so git history is the whole story.
 *
 * All build-time data; the plugin watches the relevant trees plus
 * `.git/HEAD` and invalidates this virtual module on change.
 */
function LiftedSourceDriftSection(): JSX.Element {
    const git = buildInfo.git
    return (
        <section style={diagnosticsCardStyle}>
            <div style={diagnosticsCardHeaderStyle}>
                <strong>Shared-source drift</strong>
            </div>
            <BranchBaseLine />
            <UpstreamDistanceLine behind={git.behindUpstream} />
            {git.sourceDrift == null ? (
                <p style={diagnosticsHintStyle}>
                    <span style={{color: "#ef4444"}}>
                        drift unavailable (git probe failed)
                    </span>
                </p>
            ) : (
                git.sourceDrift.map((row) => (
                    <SourceDriftBlock key={row.subtree} row={row} />
                ))
            )}
        </section>
    )
}

/**
 * The other drift axis: the per-subtree diffs say what *we* changed,
 * this says how far *upstream* has moved since we last merged. A large
 * number means the next merge will be big — not that anything is
 * currently wrong.
 */
function UpstreamDistanceLine({behind}: {behind: number | null}): JSX.Element | null {
    if (behind == null) return null
    return (
        <p style={diagnosticsHintStyle}>
            <strong>Behind upstream:</strong>{" "}
            {behind === 0 ? (
                <span style={{color: "#4ade80"}}>
                    up to date with <code>origin/main</code>
                </span>
            ) : (
                <span style={{color: behind > 50 ? "#f0c200" : "#e0e0e0"}}>
                    {behind} commit{behind === 1 ? "" : "s"} on{" "}
                    <code>origin/main</code> not in this branch
                </span>
            )}
        </p>
    )
}

function BranchBaseLine(): JSX.Element {
    const git = buildInfo.git
    if (git.base == null) {
        return (
            <p style={diagnosticsHintStyle}>
                <strong>Branch base:</strong>{" "}
                <span style={{color: "#ef4444"}}>
                    unavailable
                    {git.baseUnavailableReason
                        ? ` — ${git.baseUnavailableReason}`
                        : ""}
                </span>
            </p>
        )
    }
    const {sha, subject, author, date} = git.base
    return (
        <p style={diagnosticsHintStyle}>
            <strong>Branch base:</strong> <code>{sha}</code> — &ldquo;
            {subject}&rdquo; ({author}, {date.slice(0, 10)})
            {git.baseUnavailableReason && (
                <span
                    style={{
                        color: "#ef4444",
                        marginLeft: "0.4rem",
                        fontStyle: "italic",
                    }}
                >
                    ({git.baseUnavailableReason})
                </span>
            )}
        </p>
    )
}

type SourceDriftRow = NonNullable<
    WorkbenchBuildInfo["git"]["sourceDrift"]
>[number]

/**
 * One tracked subtree's drift vs the branch base. Collapsed by default;
 * a clean subtree is muted so the eye lands on the ones that changed.
 */
function SourceDriftBlock({row}: {row: SourceDriftRow}): JSX.Element {
    const empty = row.stat.trim().length === 0
    return (
        <details style={{marginTop: "0.75rem"}}>
            <summary
                style={{cursor: "pointer", color: empty ? "#888" : "#e0e0e0"}}
            >
                <code>{row.subtree}</code>
                {empty ? " — clean (matches base)" : " — changed"}
                {row.dirty && (
                    <span
                        style={{
                            color: "#ef4444",
                            marginLeft: "0.4rem",
                            fontStyle: "italic",
                        }}
                    >
                        (uncommitted edits present — patch below is
                        committed-only)
                    </span>
                )}
            </summary>
            <p style={diagnosticsHintStyle}>{row.expectation}</p>
            {empty ? (
                <p style={{...diagnosticsHintStyle, color: "#888"}}>
                    No diff against branch base.
                </p>
            ) : (
                <>
                    <pre style={diffStatPreStyle}>
                        <code>{row.stat}</code>
                    </pre>
                    {row.patch == null ? (
                        <p style={{...diagnosticsHintStyle, color: "#f0c200"}}>
                            {row.patchOmittedReason}
                        </p>
                    ) : (
                        <>
                            <PerFileDiffList patch={row.patch} />
                            <details style={{marginTop: "0.5rem"}}>
                                <summary
                                    style={{
                                        cursor: "pointer",
                                        color: "#999",
                                        fontSize: "0.85rem",
                                    }}
                                >
                                    Full combined diff
                                </summary>
                                <CopyJsonBlock
                                    json={row.patch}
                                    copyLabel="Copy diff"
                                />
                            </details>
                        </>
                    )}
                </>
            )}
        </details>
    )
}

/**
 * Split a multi-file unified diff into per-file blocks. Each block in
 * a `git diff` output starts with a line of the form
 * `diff --git a/<path> b/<path>`, so splitting on `\ndiff --git ` gives
 * us one chunk per file (with the leading `diff --git ` re-prepended).
 *
 * Renders each file with its own header + Copy button so the boundary
 * between, e.g., `ReviewScreen.tsx` and `castVotesSlice.ts` is
 * visually unmistakable instead of getting lost in one wall of text.
 */
function PerFileDiffList({patch}: {patch: string}): JSX.Element {
    const trimmed = patch.replace(/^\s+/, "")
    const chunks = trimmed
        .split(/\n(?=diff --git )/g)
        .map((c) => (c.startsWith("diff --git ") ? c : `diff --git ${c}`))
        .filter((c) => c.trim().length > 0)
    return (
        <>
            {chunks.map((chunk, i) => {
                const match = /^diff --git a\/(\S+) b\/(\S+)/.exec(chunk)
                const path = match ? match[2] : `file ${i + 1}`
                return (
                    <div
                        key={`${path}-${i}`}
                        style={{
                            marginTop: "0.75rem",
                            border: "1px solid #3a3a3a",
                            borderRadius: 4,
                        }}
                    >
                        <div
                            style={{
                                padding: "0.4rem 0.6rem",
                                background: "#2a2a2a",
                                borderBottom: "1px solid #3a3a3a",
                                fontFamily:
                                    "ui-monospace, SFMono-Regular, Menlo, monospace",
                                fontSize: "0.8rem",
                                color: "#e0e0e0",
                            }}
                        >
                            {path}
                        </div>
                        <div style={{padding: "0.25rem 0.5rem"}}>
                            <CopyJsonBlock
                                json={chunk}
                                copyLabel="Copy file diff"
                            />
                        </div>
                    </div>
                )
            })}
        </>
    )
}

const diagnosticsHintStyle: React.CSSProperties = {
    color: "#999",
    fontSize: "0.85rem",
    margin: "0.5rem 0",
}
const diffStatPreStyle: React.CSSProperties = {
    background: "#252525",
    padding: "0.4rem 0.6rem",
    borderRadius: 4,
    fontSize: "0.75rem",
    margin: "0.5rem 0 0.25rem 0",
    overflow: "auto",
    color: "#e0e0e0",
    border: "1px solid #4a4a4a",
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

// Shared card style for every section on the Diagnostics page (build
// status, lifted-source drift, current workbench state). Giving every
// section the same grey-boxed frame turns the page into a visually
// scannable stack instead of one long flow with subtle dividers.
const diagnosticsCardStyle: React.CSSProperties = {
    border: "1px solid #3a3a3a",
    borderRadius: 4,
    padding: "0.6rem 0.9rem",
    marginTop: "1.5rem",
    background: "#2a2a2a",
}
const diagnosticsCardHeaderStyle: React.CSSProperties = {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "baseline",
    marginBottom: "0.4rem",
}
const buildStatusCardStyle: React.CSSProperties = diagnosticsCardStyle
const buildStatusHeaderStyle: React.CSSProperties =
    diagnosticsCardHeaderStyle
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
    borderBottom: "1px solid #3a3a3a",
    padding: "0.4rem 0.6rem",
    fontWeight: 600,
    color: "#999",
}
const thNumStyle: React.CSSProperties = {
    ...thStyle,
    textAlign: "right",
}
const tdStyle: React.CSSProperties = {
    borderBottom: "1px solid #3a3a3a",
    padding: "0.4rem 0.6rem",
    verticalAlign: "middle",
}
const tdMutedStyle: React.CSSProperties = {
    ...tdStyle,
    color: "#999",
}
const tdNumStyle: React.CSSProperties = {
    ...tdStyle,
    textAlign: "right",
    fontVariantNumeric: "tabular-nums",
}
const workingRowStyle: React.CSSProperties = {
    background: "#2a2a2a",
}
const importPanelStyle: React.CSSProperties = {
    border: "1px solid #3a3a3a",
    borderRadius: 4,
    padding: "0.75rem 1rem",
    background: "#2a2a2a",
}
const importLabelStyle: React.CSSProperties = {
    display: "flex",
    flexDirection: "column",
    gap: "0.25rem",
    marginTop: "0.75rem",
    fontSize: "0.9rem",
    color: "#e0e0e0",
}
const importInputStyle: React.CSSProperties = {
    padding: "0.4rem 0.5rem",
    fontSize: "0.95rem",
    border: "1px solid #4a4a4a",
    borderRadius: 3,
    background: "#303030",
    color: "#e0e0e0",
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
                <p style={{color: "#999"}}>
                    The bundled JSON or checkpoint may have been removed.
                </p>
            </>
        )
    }
    return (
        <SnapshotDetailPageBody
            id={id}
            kind={kind}
            name={name}
            snapshot={snapshot}
            meta={meta}
        />
    )
}

/**
 * Body of {@link SnapshotDetailPage}, split out so the navigation-time
 * auto-load effect runs only when we actually have a resolved snapshot
 * — the `kind === "unknown"` / not-found branch above doesn't have a
 * snapshot to load and shouldn't be touching `currentParentId`.
 *
 * Auto-load semantics (matches the design discussed before this
 * change): if this page's `id` differs from the currently-active
 * snapshot AND the working copy is clean (matches the active
 * snapshot byte-for-byte), we treat the route itself as the load
 * instruction and reload the auto-resume slot pointed at this
 * snapshot. If the working copy is dirty, we *don't* destroy it —
 * we render a divergence banner with explicit Load / Discard
 * options instead, so the operator decides.
 */
function SnapshotDetailPageBody({
    id,
    kind,
    name,
    snapshot,
    meta,
}: {
    id: string
    kind: "bundled" | "checkpoint"
    name: string
    snapshot: PersistedSnapshot
    meta: CheckpointMeta | undefined
}): JSX.Element {
    const currentParent = useCurrentParentId()
    const isDirty = useIsWorkingDirty()
    const isActive = currentParent === id
    useEffect(() => {
        // Only auto-load when (a) the route points at a different
        // snapshot than the one currently loaded, and (b) the
        // working copy has no unsaved divergence from its active
        // snapshot. The dirty case falls through to the banner
        // below; the operator decides whether to lose their work.
        if (isActive) return
        if (isDirty) return
        loadSnapshotViaReload(snapshot, id)
    }, [id, isActive, isDirty, snapshot])
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
            <p style={{color: "#999"}}>
                <code>{id}</code> &middot;{" "}
                {kind === "bundled" ? "Bundled snapshot" : "Checkpoint"}
                {isActive && (
                    <>
                        {" "}
                        &middot;{" "}
                        <span style={{color: "#4ade80"}}>
                            ● currently loaded
                            {isDirty && " (working copy modified)"}
                        </span>
                    </>
                )}
            </p>
            {!isActive && isDirty && (
                <DivergenceBanner
                    routeId={id}
                    activeId={currentParent}
                    snapshot={snapshot}
                />
            )}
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
                {/* Button visibility/labelling matrix:
                    - active + clean: disabled "Reload" (nothing to do)
                    - active + dirty: "Reload (discard working changes)"
                    - !active + clean: "Load" — usually transient, the
                      auto-load effect above fires on mount
                    - !active + dirty: hidden; the divergence banner
                      above owns the discard-and-load action so we
                      don't offer two buttons with the same effect
                      but only one of them warns about data loss. */}
                {(isActive || !isDirty) && (
                    <button
                        type="button"
                        style={primaryButtonStyle}
                        onClick={() => {
                            // Wipe + reload via the auto-resume slot.
                            loadSnapshotViaReload(snapshot, id)
                        }}
                        disabled={isActive && !isDirty}
                        title={
                            isActive && !isDirty
                                ? "This snapshot is already loaded and the working copy matches it."
                                : undefined
                        }
                    >
                        {isActive
                            ? isDirty
                                ? "Reload (discard working changes)"
                                : "Reload"
                            : "Load"}
                    </button>
                )}
            </div>
            <details style={{marginTop: "1.5rem"}}>
                <summary style={{cursor: "pointer", color: "#e0e0e0"}}>
                    Bundled JSON (copy-paste under{" "}
                    <code>src/fixtures/snapshots/</code> to ship)
                </summary>
                <CopyJsonBlock json={bundledExport} />
            </details>
        </>
    )
}

/**
 * Yellow callout shown on the snapshot detail page when the route's
 * snapshot id differs from the active snapshot AND the working copy
 * has unsaved changes. Surfaces the otherwise-invisible
 * route-vs-active divergence and offers the two reasonable resolutions
 * explicitly so the operator never silently loses work.
 */
function DivergenceBanner({
    routeId,
    activeId,
    snapshot,
}: {
    routeId: string
    activeId: string | null
    snapshot: PersistedSnapshot
}): JSX.Element {
    return (
        <div
            style={{
                marginTop: "1rem",
                padding: "0.75rem 1rem",
                border: "1px solid #f0c200",
                background: "#3d3000",
                borderRadius: 4,
                fontSize: "0.9rem",
            }}
        >
            <strong>Working copy has unsaved changes.</strong>
            <p style={{margin: "0.4rem 0", color: "#e0e0e0"}}>
                Viewing <code>{routeId}</code>, but{" "}
                <code>{activeId ?? "(no active snapshot)"}</code> is still
                loaded with unsaved modifications. We did not auto-switch
                to this snapshot — pick one:
            </p>
            <div style={{display: "flex", gap: "0.5rem", flexWrap: "wrap"}}>
                <button
                    type="button"
                    style={primaryButtonStyle}
                    onClick={() => loadSnapshotViaReload(snapshot, routeId)}
                >
                    Discard changes &amp; load this snapshot
                </button>
                <span
                    style={{
                        alignSelf: "center",
                        color: "#999",
                        fontSize: "0.85rem",
                    }}
                >
                    (Or save the working copy as a checkpoint first via
                    the inspector home, then click again.)
                </span>
            </div>
        </div>
    )
}

export function CopyJsonBlock({
    json,
    copyLabel = "Copy JSON",
}: {
    json: string
    /**
     * Override the button label for non-JSON payloads (e.g. unified
     * diffs). The clipboard write is always the raw `json` string —
     * the label is purely cosmetic.
     */
    copyLabel?: string
}): JSX.Element {
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
                    {copied ? "Copied." : copyLabel}
                </button>
            </div>
            <pre
                style={{
                    background: "#252525",
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
            <p style={{color: "#999"}}>
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
                        <em style={{color: "#ef4444"}}>
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
                <summary style={{cursor: "pointer", color: "#e0e0e0"}}>
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
            <em style={{color: "#ef4444"}}>
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
                <p style={{color: "#ef4444", margin: "0 0 0.4rem 0"}}>
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
 *  - and a hand-off to the standalone `/tally` sandbox via the
 *    "Open in tally" button. Tally execution itself lives on that
 *    page (see TallyPage.tsx); this page assembles the decoded
 *    ballots (one `DecodedVoteContest` per cast vote whose bridge
 *    entry has a decoded BigUint for this contest) and passes them
 *    plus the contest descriptor through react-router state.
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
    // Set of every ballot-style id in this election whose EML
    // includes this contest. When two ballot styles share the
    // contest (e.g. velvet-multi-bs's Area A and Area B both carry
    // `...c1`), cast votes from voters on EITHER BS must show up
    // here — filtering by a single `found.ballotStyle.id` would
    // hide votes from the other BS. Empty when no BS is found.
    const validBsIds = useMemo<Set<string>>(() => {
        const out = new Set<string>()
        if (!found) return out
        const electionId = found.ballotStyle.election_id
        const collect = (bs: PortalBSRow | undefined): void => {
            if (!bs) return
            if (bs.election_id !== electionId) return
            if (bs.ballot_eml.contests?.some((c) => c.id === contestId)) {
                out.add(bs.id)
            }
        }
        if (pool) {
            for (const rows of Object.values(pool)) {
                for (const row of rows) collect(row as PortalBSRow)
            }
        }
        // Live portal slice may add the active session's BS in the
        // rare case where the overlay is absent.
        return out
    }, [found, pool, contestId])
    // Snapshot-wide keypair, used to seed the pipeline page so its
    // encrypt/decrypt stages match what the bridge actually used.
    const keypair = useWorkbench((w) => w.keypair)
    // Decoded BigUint per cast vote for this contest, in cast order.
    // Cast votes whose bridge entry hasn't filled `decodedBigInts`
    // yet (e.g. the decrypt observer hasn't run) are simply absent.
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
        const rows: Array<{
            castVoteId: string
            decoded: string | undefined
        }> = []
        for (const cv of castVotes) {
            const entry = repaired[cv.id]
            if (
                entry?.ballotStyleId &&
                validBsIds.size > 0 &&
                !validBsIds.has(entry.ballotStyleId)
            ) {
                // Cast against a ballot style in this election that
                // does not include this contest.
                continue
            }
            rows.push({
                castVoteId: cv.id,
                decoded: entry?.decodedBigInts?.[contestId],
            })
        }
        return rows
    }, [castVotes, repaired, contestId, validBsIds, found])
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
        const rows: PipelineSeedRow[] = []
        for (const cv of castVotes) {
            const entry = repaired[cv.id]
            if (
                entry?.ballotStyleId &&
                validBsIds.size > 0 &&
                !validBsIds.has(entry.ballotStyleId)
            ) {
                // Cast against a BS in this election that does not
                // include this contest.
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
    }, [found, castVotes, repaired, contestId, keypair, navigate, validBsIds])

    // "Open in tally" — sibling of the pipeline hand-off. Decode each
    // bridge-captured BigUint into a `DecodedVoteContest` (the tally
    // entry shape) and ship the array plus the contest descriptor to
    // `/tally`, which is the single execution site for tallies in
    // the workbench. The contest page intentionally does *not* run
    // tallies itself — keeping execution in one place means there's
    // exactly one code path to debug, and the standalone page's
    // visualization is always the canonical view.
    const handleOpenInTally = useCallback(async () => {
        if (!found) return
        const contestJson = JSON.stringify(found.contest, null, 2)
        const decoded: unknown[] = []
        for (const row of decodedRows) {
            if (!row.decoded) continue
            try {
                const json = await decodeBigIntToDecodedVoteContest(
                    contestJson,
                    row.decoded
                )
                decoded.push(JSON.parse(json))
            } catch {
                // Skip rows that fail to decode; the standalone tool
                // is still useful with whatever survived.
            }
        }
        const seed: TallySeed = {
            contestName: found.contest?.name,
            contestJson,
            decodedBallots: decoded,
        }
        navigate("/tally", {state: seed})
    }, [found, decodedRows, navigate])

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
            <p style={{color: "#999"}}>
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
            <ContestPolicyOverridesPanel
                contest={{
                    id: contestId,
                    counting_algorithm: contest.counting_algorithm as
                        | string
                        | null
                        | undefined,
                    voting_type: contest.voting_type as
                        | string
                        | null
                        | undefined,
                    presentation: contest.presentation as
                        | Record<string, unknown>
                        | null
                        | undefined,
                    min_votes:
                        typeof contest.min_votes === "number"
                            ? contest.min_votes
                            : undefined,
                    max_votes:
                        typeof contest.max_votes === "number"
                            ? contest.max_votes
                            : undefined,
                }}
            />
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
                <button
                    type="button"
                    onClick={handleOpenInTally}
                    style={secondaryButtonStyle}
                    title="Open this contest's decoded ballots in the
 standalone tally sandbox"
                >
                    Open in tally
                </button>
            </div>
            <ContestTallyView
                decodedRows={decodedRows}
            />
        </>
    )
}

/** Renders the workbench's contribution to tally work for one
 *  contest: a one-line summary of how many cast votes were
 *  successfully decoded. Tally execution itself lives on `/tally`
 *  (see the "Open in tally" button above this view); per-row decoded
 *  BigUints are inspectable on `/pipeline` (see "Open in ballot
 *  pipeline" — the Decrypt stage cell of each row holds the BigUint).
 *  Keeping that detail in one place avoids duplicating it here. */
function ContestTallyView({
    decodedRows,
}: {
    decodedRows: Array<{castVoteId: string; decoded: string | undefined}>
}): JSX.Element {
    const decodedCount = decodedRows.filter((r) => !!r.decoded).length
    return (
        <p style={{margin: "0.3rem 0", color: "#e0e0e0"}}>
            {decodedCount} of {decodedRows.length} cast vote
            {decodedRows.length === 1 ? "" : "s"} decoded for this
            contest. Click <strong>Open in tally</strong> above to run
            the tally on these ballots in the standalone sandbox, or{" "}
            <strong>Open in ballot pipeline</strong> to inspect the
            per-row decrypted BigUints and full encode/encrypt/decrypt
            round-trip.
        </p>
    )
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
            <p style={{color: "#999"}}>
                <code>{voter.id}</code> &middot; Voter
            </p>
            {voter.notes && (
                <p style={{color: "#e0e0e0"}}>{voter.notes}</p>
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
                                <BallotStyleContestLinks bs={bs} />
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

/**
 * Inline summary of the contests that the given ballot style entitles
 * a voter to vote in. Rendered next to each "Cast a ballot…" button
 * on the voter page in place of the raw ballot-style UUID, which
 * carried no operator-meaningful information.
 *
 * Each contest is a `NavLink` into `/wb/contest/:id` so the operator
 * can jump straight to the contest detail page (selections, tally
 * stage cells, etc.). The ballot-style UUID is kept reachable via a
 * trailing `(style)` link so anyone who actually wanted that detail
 * page can still get there in one click.
 *
 * Falls back to just the ballot-style link if `ballot_eml.contests`
 * is missing or empty — better to show nothing extra than to render
 * an empty `Contests:` label.
 */
function BallotStyleContestLinks({
    bs,
}: {
    bs: {
        id: string
        ballot_eml: {
            contests?: ReadonlyArray<{
                id: string
                name?: string | null
            }>
        }
    }
}): JSX.Element {
    const contests = bs.ballot_eml.contests ?? []
    return (
        <span style={{color: "#999", fontSize: "0.85rem"}}>
            {contests.length > 0 && (
                <>
                    Contests:{" "}
                    {contests.map((c, i) => (
                        <Fragment key={c.id}>
                            {i > 0 ? ", " : ""}
                            <NavLink
                                to={`/wb/contest/${c.id}`}
                                style={inlineLinkStyle}
                                title={c.id}
                            >
                                {c.name || c.id}
                            </NavLink>
                        </Fragment>
                    ))}
                    {" \u00b7 "}
                </>
            )}
            <NavLink
                to={`/wb/ballot-style/${bs.id}`}
                style={{...inlineLinkStyle, color: "#888"}}
                title={`Ballot style ${bs.id}`}
            >
                style
            </NavLink>
        </span>
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
            <div style={{fontSize: "0.85rem", color: "#e0e0e0"}}>
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
    background: "#2a2a2a",
    border: "1px solid #3a3a3a",
    borderRadius: 4,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const navLinkStyle = ({isActive}: {isActive: boolean}): React.CSSProperties => ({
    display: "block",
    padding: "0.15rem 0.3rem",
    borderRadius: 3,
    color: "#e0e0e0",
    textDecoration: "none",
    background: isActive ? "#383838" : "transparent",
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
                color: "#888",
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
                color: "#e0e0e0",
                margin: "0.75rem 0 0.25rem 0",
            }}
        >
            {children}
        </h4>
    )
}

function Empty({children}: {children: React.ReactNode}): JSX.Element {
    return <div style={{color: "#888", fontStyle: "italic"}}>{children}</div>
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
            <dt style={{color: "#999"}}>{label}</dt>
            <dd style={{margin: 0}}>{children}</dd>
        </>
    )
}

const primaryButtonStyle: React.CSSProperties = {
    padding: "0.5rem 1rem",
    background: "#2563eb",
    color: "white",
    border: 0,
    borderRadius: 4,
    fontSize: "0.9rem",
    cursor: "pointer",
}

const secondaryButtonStyle: React.CSSProperties = {
    padding: "0.3rem 0.8rem",
    background: "#383838",
    color: "#e0e0e0",
    border: "1px solid #555",
    borderRadius: 4,
    fontSize: "0.85rem",
    cursor: "pointer",
}

const codeBlockStyle: React.CSSProperties = {
    display: "inline-block",
    padding: "0.3rem 0.5rem",
    background: "#252525",
    border: "1px solid #4a4a4a",
    borderRadius: 3,
    fontSize: "0.8rem",
    wordBreak: "break-all",
    maxWidth: "44rem",
    color: "#e0e0e0",
}

const h2Style: React.CSSProperties = {
    fontSize: "1rem",
    margin: "1.5rem 0 0.5rem 0",
    color: "#e0e0e0",
}

const inlineLinkStyle: React.CSSProperties = {
    color: "#5b9aff",
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
                color: "#e0e0e0",
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

/**
 * Whether the live working copy diverges from the snapshot it was
 * forked off of. Computed by stringifying the canonical state shape
 * on both sides — same cost as the `buildCurrentSnapshot` JSON dump
 * the diagnostics page already pays once per render, and only
 * computed on screens that actually consume it (rail + snapshot
 * detail). For workbench-sized state this is microseconds, so we
 * don't bother caching across components.
 *
 * Returns `false` when the active snapshot cannot be resolved
 * (legacy `parentId == null`, or a checkpoint that was deleted from
 * under the working copy). "Unknown" defaults to "not dirty" so we
 * never spuriously gate the auto-load behavior on this hook — the
 * /wb table's Delete button is disabled on the active row precisely
 * so this unresolved state stays impossible in practice; if it ever
 * does occur we want loud breakage rather than a silent bypass.
 */
function useIsWorkingDirty(): boolean {
    const store = useStore()
    const reduxState = useSyncExternalStore(
        store.subscribe,
        store.getState
    ) as RootState
    const workbenchState = useSyncExternalStore(
        subscribeWorkbench,
        getWorkbenchState
    )
    const parentId = useCurrentParentId()
    return useMemo(() => {
        if (parentId == null) return false
        const active = loadSnapshotById(parentId)
        if (!active) return false
        // Compare canonical projections on both sides. The blob is
        // already a projection (see `persistence.ts` write path), but
        // bundled fixtures may carry stray `{}` slices and re-projecting
        // the live state on every check makes the comparison robust
        // against any drift between what the store carries and what
        // counts as a scenario. See `CANONICAL_STATE_KEYS`.
        const liveCanonical = canonicalCompareJson(reduxState)
        const savedCanonical = canonicalCompareJson(active.state as RootState)
        if (liveCanonical !== savedCanonical) return true
        const liveWb = JSON.stringify(workbenchState)
        const activeWb = JSON.stringify(
            active.workbench ?? {
                voters: [],
                activeVoterId: null,
                castBy: {},
                repairedCastVotes: {},
                keypair: null,
            }
        )
        return liveWb !== activeWb
    }, [reduxState, workbenchState, parentId])
}

// Temporary diagnostic: dumps the first divergence between the live
// working copy and the currently-active snapshot. Exposed on
// `window.__dirtyDiff` so we can call it from the browser console
// when the dirty indicator looks wrong.
if (typeof window !== "undefined") {
    ;(window as unknown as {__dirtyDiff: () => unknown}).__dirtyDiff =
        (): unknown => {
            const parentId = getCurrentParentId()
            if (parentId == null) return {reason: "no active parent"}
            const active = loadSnapshotById(parentId)
            if (!active) return {reason: "active snapshot unresolvable", parentId}
            const liveState = (
                window as unknown as {__store: {getState: () => RootState}}
            ).__store.getState()
            const liveWb = getWorkbenchState()
            const stateA = canonicalCompareJson(liveState)
            const stateB = canonicalCompareJson(active.state as RootState)
            const wbA = JSON.stringify(liveWb)
            const wbB = JSON.stringify(active.workbench)
            const firstDiff = (a: string, b: string): unknown => {
                if (a === b) return null
                for (let i = 0; i < Math.min(a.length, b.length); i++) {
                    if (a[i] !== b[i]) {
                        return {
                            index: i,
                            live: a.slice(Math.max(0, i - 60), i + 120),
                            saved: b.slice(Math.max(0, i - 60), i + 120),
                            liveLen: a.length,
                            savedLen: b.length,
                        }
                    }
                }
                return {
                    index: "tail",
                    liveLen: a.length,
                    savedLen: b.length,
                    liveTail: a.slice(-150),
                    savedTail: b.slice(-150),
                }
            }
            return {
                parentId,
                stateEq: stateA === stateB,
                wbEq: wbA === wbB,
                stateDiff: firstDiff(stateA, stateB),
                wbDiff: firstDiff(wbA, wbB),
            }
        }
}

/**
 * Format `now()` as `YYYY-MM-DD-HH-MM-SS`. Used to name auto-
 * checkpoints created on raw-JSON import — sortable, collision-
 * resistant for human-paced operation, and inside the charset
 * `normalizeCheckpointName` accepts. Avoids `:` (rejected) and `T`
 * (looks weird in the rail).
 */
function formatImportTimestamp(): string {
    const d = new Date()
    const pad = (n: number): string => String(n).padStart(2, "0")
    return (
        `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}` +
        `-${pad(d.getHours())}-${pad(d.getMinutes())}-${pad(d.getSeconds())}`
    )
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
        // Re-read on any workbench mutation. `saveCheckpoint` and
        // `deleteCheckpoint` both bump workbench state (the latter
        // via the trailing `replaceWorkbenchState(getWorkbenchState())`
        // call) so a single subscription on `subscribeWorkbench`
        // covers both writers.
        (cb) => subscribeWorkbench(cb),
        getCheckpointsCached,
        getCheckpointsCached
    )
}

// --- Bundled-snapshot list -----------------------------------------------
//
// Bundled snapshots are immutable: the dictionary in
// `fixtures/bundledSnapshots.ts` is built once at module load from
// `import.meta.glob` and never mutated. Components that need the id
// list compute `Object.keys(BUNDLED_SNAPSHOTS).sort()` inline (5
// entries, microseconds per render); there's no reactive subscription
// hook because nothing changes after boot.
