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
import {NavLink, Outlet, useParams} from "react-router-dom"
import {useSelector} from "react-redux"
import {useSyncExternalStore} from "react"
import {subscribeWorkbench, useWorkbench} from "./workbenchStore"
import {
    bundledId,
    checkpointId,
    getCurrentParentId,
    listCheckpoints,
    type CheckpointMeta,
} from "./persistence"
import {BUNDLED_SNAPSHOTS} from "./fixtures/bundledSnapshots"
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
            <TenantsSection />
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
            {/* Working-copy entry. Always present; clicking it lands
                on the snapshot overview page for the live state. */}
            <NavLink to="/wb" end style={navLinkStyle}>
                ● Working copy
            </NavLink>
            <div
                style={{
                    fontSize: "0.7rem",
                    color: "#888",
                    margin: "0 0 0.5rem 1rem",
                }}
            >
                forked from{" "}
                <code>{currentParent ?? "<root>"}</code>
            </div>
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

// --- Tenants: tenant → event → election → {Contests, Ballot styles} -------

interface TenantNode {
    tenantId: string
    events: EventNode[]
}
interface EventNode {
    id: string
    name: string
    elections: ElectionNode[]
}
interface ElectionNode {
    id: string
    name: string
    contestIds: {id: string; name: string}[]
    ballotStyleIds: {id: string; name: string}[]
}

function selectTenantTree(state: RootState): TenantNode[] {
    // Group events by tenant_id.
    const byTenant = new Map<string, TenantNode>()
    const ensure = (tid: string): TenantNode => {
        let t = byTenant.get(tid)
        if (!t) {
            t = {tenantId: tid, events: []}
            byTenant.set(tid, t)
        }
        return t
    }
    const eventNodes = new Map<string, EventNode>()
    for (const ev of Object.values(state.electionEvent)) {
        if (!ev) continue
        const node: EventNode = {
            id: ev.id,
            name: ev.name ?? "(unnamed event)",
            elections: [],
        }
        eventNodes.set(ev.id, node)
        ensure(ev.tenant_id).events.push(node)
    }
    // Index ballot styles by election for cheap lookup.
    const bsByElection = new Map<string, {id: string; name: string}[]>()
    for (const bs of Object.values(state.ballotStyles)) {
        if (!bs) continue
        const list = bsByElection.get(bs.election_id) ?? []
        list.push({
            id: bs.id,
            name:
                state.elections[bs.election_id]?.name ??
                bs.id.slice(0, 8),
        })
        bsByElection.set(bs.election_id, list)
    }
    for (const el of Object.values(state.elections)) {
        if (!el) continue
        const node: ElectionNode = {
            id: el.id,
            name: el.name ?? "(unnamed election)",
            contestIds: [],
            ballotStyleIds: bsByElection.get(el.id) ?? [],
        }
        // Contests live on the ballot styles' EML. Dedupe by id
        // across all ballot styles of this election.
        const seen = new Set<string>()
        for (const bs of Object.values(state.ballotStyles)) {
            if (!bs || bs.election_id !== el.id) continue
            for (const c of bs.ballot_eml.contests) {
                if (seen.has(c.id)) continue
                seen.add(c.id)
                node.contestIds.push({id: c.id, name: c.name})
            }
        }
        // Attach to its event if known, otherwise to a synthetic
        // "(no event)" slot under the same tenant.
        const ev = eventNodes.get(el.election_event_id)
        if (ev) {
            ev.elections.push(node)
        } else {
            const t = ensure(el.tenant_id)
            let stray = t.events.find((e) => e.id === "__no_event__")
            if (!stray) {
                stray = {
                    id: "__no_event__",
                    name: "(no event)",
                    elections: [],
                }
                t.events.push(stray)
            }
            stray.elections.push(node)
        }
    }
    // Alphabetise everything for a stable rail.
    const tenants = [...byTenant.values()].sort((a, b) =>
        a.tenantId.localeCompare(b.tenantId)
    )
    for (const t of tenants) {
        t.events.sort((a, b) => a.name.localeCompare(b.name))
        for (const e of t.events) {
            e.elections.sort((a, b) => a.name.localeCompare(b.name))
            for (const el of e.elections) {
                el.contestIds.sort((a, b) =>
                    a.name.localeCompare(b.name)
                )
                el.ballotStyleIds.sort((a, b) =>
                    a.name.localeCompare(b.name)
                )
            }
        }
    }
    return tenants
}

function TenantsSection(): JSX.Element {
    const tenants = useSelector(selectTenantTree)
    return (
        <section>
            <SectionHeading>Tenants</SectionHeading>
            {tenants.length === 0 ? (
                <Empty>(none)</Empty>
            ) : (
                <ul style={listStyle}>
                    {tenants.map((t) => (
                        <li key={t.tenantId}>
                            <NodeLabel title={t.tenantId}>
                                {t.tenantId.slice(0, 8)}…
                            </NodeLabel>
                            <ul style={listStyle}>
                                {t.events.map((ev) => (
                                    <li
                                        key={ev.id}
                                        style={{marginLeft: "1rem"}}
                                    >
                                        <NodeLabel title={ev.id}>
                                            {ev.name}
                                        </NodeLabel>
                                        <ul style={listStyle}>
                                            {ev.elections.map((el) => (
                                                <li
                                                    key={el.id}
                                                    style={{
                                                        marginLeft: "1rem",
                                                    }}
                                                >
                                                    <NodeLabel title={el.id}>
                                                        {el.name}
                                                    </NodeLabel>
                                                    <ElectionChildren
                                                        election={el}
                                                    />
                                                </li>
                                            ))}
                                        </ul>
                                    </li>
                                ))}
                            </ul>
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
// Detail page placeholders (filled in by tasks 6–9)
// ---------------------------------------------------------------------------

export function SnapshotOverviewPage(): JSX.Element {
    return (
        <>
            <h1>Working copy</h1>
            <p style={{color: "#666"}}>
                Placeholder. The full overview (provenance lineage, voter
                count, ballot-style list, "Save as checkpoint" button)
                lands in task 6.
            </p>
        </>
    )
}

export function SnapshotDetailPage(): JSX.Element {
    const {id} = useParams()
    return (
        <>
            <h1>Snapshot</h1>
            <p>
                <code>{id}</code>
            </p>
            <p style={{color: "#666"}}>
                Placeholder. Bundled / checkpoint detail (Load button,
                copy-as-bundled JSON) lands in task 6.
            </p>
        </>
    )
}

export function BallotStyleDetailPage(): JSX.Element {
    const {id} = useParams()
    return (
        <>
            <h1>Ballot style</h1>
            <p>
                <code>{id}</code>
            </p>
            <p style={{color: "#666"}}>
                Placeholder. Keypair view and contest list land in task 7.
            </p>
        </>
    )
}

export function ContestDetailPage(): JSX.Element {
    const {id} = useParams()
    return (
        <>
            <h1>Contest</h1>
            <p>
                <code>{id}</code>
            </p>
            <p style={{color: "#666"}}>
                Placeholder. Candidate list and per-contest tally land in
                task 8.
            </p>
        </>
    )
}

export function VoterDetailPage(): JSX.Element {
    const {id} = useParams()
    return (
        <>
            <h1>Voter</h1>
            <p>
                <code>{id}</code>
            </p>
            <p style={{color: "#666"}}>
                Placeholder. Vote-as CTA + cast-vote rows with decoded
                BigUints land in task 9.
            </p>
        </>
    )
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
