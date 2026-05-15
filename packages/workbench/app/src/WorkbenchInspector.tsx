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
import {useMemo, useState, useSyncExternalStore} from "react"
import {subscribeWorkbench, useWorkbench} from "./workbenchStore"
import {
    bundledId,
    checkpointId,
    getCurrentParentId,
    hydrateFromSnapshot,
    listCheckpoints,
    loadCheckpoint,
    normalizeCheckpointName,
    readCheckpointSnapshot,
    saveCheckpoint,
    type CheckpointMeta,
    type PersistedSnapshot,
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
    const ballotStyleCount = useSelector(
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
    const [error, setError] = useState<string | null>(null)
    return (
        <>
            <h1>Working copy</h1>
            <p style={{color: "#666"}}>
                Live in-memory state of the workbench. Auto-saved to
                localStorage on every change.
            </p>
            <dl style={dlStyle}>
                <DlRow label="Forked from">
                    <code>{parentId ?? "(root — no parent)"}</code>
                </DlRow>
                <DlRow label="Voters">{voterCount}</DlRow>
                <DlRow label="Elections">{electionCount}</DlRow>
                <DlRow label="Ballot styles">{ballotStyleCount}</DlRow>
                <DlRow label="Cast votes">{castVoteCount}</DlRow>
            </dl>
            <div style={{marginTop: "1.5rem"}}>
                <button
                    type="button"
                    style={primaryButtonStyle}
                    onClick={() => {
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
                            setError(
                                e instanceof Error ? e.message : String(e)
                            )
                        }
                    }}
                >
                    Save current state as checkpoint…
                </button>
                {error && (
                    <p style={{color: "#b00020", marginTop: "0.5rem"}}>
                        {error}
                    </p>
                )}
            </div>
        </>
    )
}

function selectStateCounts(s: RootState): {
    elections: number
    ballotStyles: number
    castVotes: number
} {
    return {
        elections: Object.values(s.elections).filter(Boolean).length,
        ballotStyles: Object.values(s.ballotStyles).filter(Boolean).length,
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
    const store = useStore()
    const navigate = useNavigate()
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
    const stateCounts = selectStateCounts(snapshot.state)
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
                        const typedStore = store as Parameters<
                            typeof saveCheckpoint
                        >[0]
                        if (kind === "checkpoint") {
                            loadCheckpoint(typedStore, name)
                        } else {
                            hydrateFromSnapshot(
                                typedStore,
                                snapshot,
                                bundledId(name)
                            )
                        }
                        // Land on the working-copy overview so the
                        // operator can see what they just loaded.
                        navigate("/wb")
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
    // BallotStylesState is keyed by election_id, not by ballot style id,
    // so we scan rather than index-lookup. The set is tiny in any
    // realistic workbench scenario.
    const ballotStyle = useSelector((s: RootState) =>
        Object.values(s.ballotStyles).find((bs) => bs?.id === bsId)
    )
    const election = useSelector((s: RootState) =>
        ballotStyle ? s.elections[ballotStyle.election_id] : undefined
    )
    const keypair = useWorkbench((w) => w.keypairs[bsId])
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
                no keypair registered for this ballot style — the
                decrypt bridge will fall back to a fresh keypair
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
