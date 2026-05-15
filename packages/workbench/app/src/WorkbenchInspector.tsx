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
//   │ Provenance  │   style / contest /      │
//   │ + scenario  │   voter detail page)     │
//   └─────────────┴──────────────────────────┘
//
// The tree rail is split in two sections:
//   - "Current snapshot": entities in the live store the user can
//     drill into (ballot styles → contests, voters).
//   - "Provenance": the forest of saved snapshots (bundled JSONs
//     under src/fixtures/snapshots/ + named checkpoints in
//     localStorage) keyed by `parentId`. The current working copy
//     is highlighted under whichever entry it forked from.
//
// Task 5 of the step-6 plan brings the layout + rail + placeholder
// detail pages online. Tasks 6–9 flesh out each detail page.

import type {RootState} from "voting-portal/src/store/store"
import {NavLink, Outlet, useParams} from "react-router-dom"
import {useSelector} from "react-redux"
import {useSyncExternalStore} from "react"
import {subscribeWorkbench, useWorkbench} from "./workbenchStore"
import {
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
            <CurrentSnapshotSection />
            <hr style={{margin: "1.25rem 0", border: 0, borderTop: "1px solid #ddd"}} />
            <ProvenanceSection />
        </>
    )
}

// --- Current snapshot: ballot styles → contests, voters --------------------

function CurrentSnapshotSection(): JSX.Element {
    const ballotStyles = useSelector((s: RootState) => s.ballotStyles)
    const elections = useSelector((s: RootState) => s.elections)
    const voters = useWorkbench((w) => w.voters)
    const parentId = useCurrentParentId()
    return (
        <section>
            <SectionHeading>Current snapshot</SectionHeading>
            <div style={{fontSize: "0.75rem", color: "#666", marginBottom: "0.5rem"}}>
                forked from <code>{parentId ?? "<root>"}</code>
            </div>
            <NavLink to="/wb" end style={navLinkStyle}>
                Snapshot overview
            </NavLink>

            <SubHeading>Ballot styles</SubHeading>
            {Object.values(ballotStyles).length === 0 ? (
                <Empty>(none)</Empty>
            ) : (
                <ul style={listStyle}>
                    {Object.values(ballotStyles).map((bs) => {
                        if (!bs) return null
                        const election = elections[bs.election_id]
                        return (
                            <li key={bs.id}>
                                <NavLink
                                    to={`/wb/ballot-style/${bs.id}`}
                                    style={navLinkStyle}
                                >
                                    {election?.name ?? bs.id.slice(0, 8)}
                                </NavLink>
                                <ul style={{...listStyle, marginLeft: "1rem"}}>
                                    {bs.ballot_eml.contests.map((c) => (
                                        <li key={c.id}>
                                            <NavLink
                                                to={`/wb/contest/${c.id}`}
                                                style={navLinkStyle}
                                            >
                                                {c.name}
                                            </NavLink>
                                        </li>
                                    ))}
                                </ul>
                            </li>
                        )
                    })}
                </ul>
            )}

            <SubHeading>Voters</SubHeading>
            {voters.length === 0 ? (
                <Empty>(none)</Empty>
            ) : (
                <ul style={listStyle}>
                    {voters.map((v) => (
                        <li key={v.id}>
                            <NavLink to={`/wb/voter/${v.id}`} style={navLinkStyle}>
                                {v.displayName}
                            </NavLink>
                        </li>
                    ))}
                </ul>
            )}
        </section>
    )
}

// --- Provenance: bundled + checkpoint snapshots ----------------------------

function ProvenanceSection(): JSX.Element {
    // Checkpoints live in localStorage and may grow during the
    // session; subscribe to a `storage`-like signal so the rail
    // updates without needing the operator to reload. The list-then-
    // hash trick keeps re-renders cheap.
    const checkpoints = useCheckpointList()
    const bundled = Object.keys(BUNDLED_SNAPSHOTS).sort()
    const currentParent = useCurrentParentId()
    return (
        <section>
            <SectionHeading>Provenance</SectionHeading>
            <SubHeading>Bundled</SubHeading>
            <ul style={listStyle}>
                {bundled.map((name) => {
                    const id = `bundled:${name}`
                    return (
                        <li key={id}>
                            <ProvenanceRow id={id} current={currentParent === id}>
                                {name}
                            </ProvenanceRow>
                        </li>
                    )
                })}
            </ul>
            <SubHeading>Checkpoints</SubHeading>
            {checkpoints.length === 0 ? (
                <Empty>(none saved)</Empty>
            ) : (
                <ul style={listStyle}>
                    {checkpoints.map((cp) => {
                        const id = `checkpoint:${cp.name}`
                        return (
                            <li key={id}>
                                <ProvenanceRow
                                    id={id}
                                    current={currentParent === id}
                                >
                                    {cp.name}
                                </ProvenanceRow>
                            </li>
                        )
                    })}
                </ul>
            )}
        </section>
    )
}

function ProvenanceRow(props: {
    id: string
    current: boolean
    children: React.ReactNode
}): JSX.Element {
    return (
        <span
            title={props.id}
            style={{
                display: "inline-block",
                padding: "0.1rem 0.3rem",
                borderRadius: 3,
                background: props.current ? "#dde9ff" : "transparent",
                fontWeight: props.current ? 600 : 400,
            }}
        >
            {props.children}
        </span>
    )
}

// ---------------------------------------------------------------------------
// Detail page placeholders (filled in by tasks 6–9)
// ---------------------------------------------------------------------------

export function SnapshotOverviewPage(): JSX.Element {
    return (
        <>
            <h1>Snapshot overview</h1>
            <p style={{color: "#666"}}>
                Placeholder. The full overview (provenance lineage, voter
                count, ballot-style list, "Save as checkpoint" button, JSON
                export) lands in task 6.
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
