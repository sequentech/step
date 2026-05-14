// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Workbench-native pages (homepage + drilldown views).
//
// These are NOT lifted from voting-portal. They are the workbench's own
// chrome for navigating, inspecting and launching scenarios. The
// production system has admin-portal for this; we explicitly chose not
// to lift it (see LIFTING.md "Out of scope") because admin-portal carries
// inherited complexity we want to escape. Workbench-native pages stay
// small and direct, and consume the same Redux state the booth writes
// into.
//
// Conventions:
//   - Workbench pages live under `/wb/...` to keep them clearly distinct
//     from the production-mirror routes (`/tenant/:t/event/:e/...`)
//     that voting-portal screens own.
//   - Workbench links to the booth use the production paths so existing
//     in-portal navigation (back/edit/finish) keeps working unmodified.

import {Link, useNavigate, useParams} from "react-router-dom"
import {useSelector} from "react-redux"
import {useCallback, useState, type CSSProperties} from "react"
import type {RootState} from "voting-portal/src/store/store"
import {store} from "voting-portal/src/store/store"
import type {ICastVote} from "voting-portal/src/store/castVotes/castVotesSlice"
import type {IBallotStyle} from "voting-portal/src/store/ballotStyles/ballotStylesSlice"
import {
    runElectionTally,
    type ContestTallyOutcome,
} from "./electionTally"
import {
    deleteCheckpoint,
    listCheckpoints,
    loadCheckpoint,
    saveCheckpoint,
    type CheckpointMeta,
} from "./persistence"
import {
    addVoter,
    removeVoter,
    setActiveVoter,
    useWorkbench,
    type Voter,
} from "./workbenchStore"

const styles: Record<string, CSSProperties> = {
    main: {
        fontFamily: "system-ui, sans-serif",
        padding: "1rem 2rem",
        maxWidth: "70rem",
        margin: "0 auto",
    },
    h1: {marginBottom: "0.25rem"},
    crumbs: {
        fontSize: "0.85rem",
        color: "#666",
        marginBottom: "1rem",
    },
    card: {
        border: "1px solid #ddd",
        borderRadius: "6px",
        padding: "0.75rem 1rem",
        marginBottom: "0.75rem",
        background: "#fafafa",
    },
    section: {marginTop: "1.5rem"},
    sectionTitle: {
        fontSize: "0.9rem",
        textTransform: "uppercase",
        letterSpacing: "0.05em",
        color: "#555",
        marginBottom: "0.5rem",
    },
    empty: {fontStyle: "italic", color: "#888", fontSize: "0.9rem"},
    mono: {fontFamily: "ui-monospace, Menlo, Consolas, monospace", fontSize: "0.8rem"},
    table: {borderCollapse: "collapse", width: "100%", fontSize: "0.85rem"},
    th: {
        textAlign: "left",
        padding: "0.4rem 0.6rem",
        background: "#f0f0f0",
        borderBottom: "1px solid #ccc",
    },
    td: {padding: "0.4rem 0.6rem", borderBottom: "1px solid #eee"},
    cta: {
        display: "inline-block",
        marginTop: "0.5rem",
        padding: "0.4rem 0.9rem",
        background: "#1976d2",
        color: "white",
        borderRadius: "4px",
        textDecoration: "none",
        fontSize: "0.85rem",
    },
}

/**
 * Derive the tenant catalog from the Redux state.
 *
 * There is no `tenants` slice — production keeps tenants in Hasura.
 * The workbench reconstructs the set of tenants from `tenant_id` fields
 * on the entities we DO have (elections, events). For each tenant we
 * also collect the ids of its events and elections, which is what every
 * downstream page wants anyway.
 */
function selectTenantCatalog(state: RootState): TenantSummary[] {
    const byTenant: Record<string, TenantSummary> = {}
    const ensure = (tenantId: string) => {
        if (!byTenant[tenantId]) {
            byTenant[tenantId] = {tenantId, eventIds: [], electionIds: []}
        }
        return byTenant[tenantId]
    }
    for (const event of Object.values(state.electionEvent)) {
        if (!event) continue
        ensure(event.tenant_id).eventIds.push(event.id)
    }
    for (const election of Object.values(state.elections)) {
        if (!election) continue
        ensure(election.tenant_id).electionIds.push(election.id)
    }
    return Object.values(byTenant)
}

interface TenantSummary {
    tenantId: string
    eventIds: string[]
    electionIds: string[]
}

/**
 * Workbench homepage. Lists every tenant currently in Redux and lets
 * you drill into each one. This replaces the previous hardcoded
 * "Booth / Tally" nav links as the entry point.
 *
 * Intentionally not paginated: a workbench scenario rarely has more
 * than a handful of entities. When that stops being true, the
 * homepage gets a search box, not pagination.
 */
export function WorkbenchHome() {
    const tenants = useSelector(selectTenantCatalog)
    const castVotesByElection = useSelector(
        (state: RootState) => state.castVotes
    )

    const totalCastVotes = Object.values(castVotesByElection).reduce(
        (n, votes) => n + (votes?.length ?? 0),
        0
    )

    return (
        <main style={styles.main}>
            <h1 style={styles.h1}>Workbench</h1>
            <p style={styles.crumbs}>
                Self-contained sandbox for the voting-portal lift.{" "}
                <Link to="/tally">Raw-JSON tally sandbox →</Link>
            </p>

            <section style={styles.section}>
                <div style={styles.sectionTitle}>
                    Tenants ({tenants.length}){" "}
                    <span
                        style={{
                            float: "right",
                            color: "#666",
                            textTransform: "none",
                            letterSpacing: 0,
                        }}
                    >
                        {totalCastVotes} cast vote
                        {totalCastVotes === 1 ? "" : "s"} in state
                    </span>
                </div>
                {tenants.length === 0 ? (
                    <p style={styles.empty}>
                        No tenants in Redux. Reset the workbench or load a
                        snapshot.
                    </p>
                ) : (
                    tenants.map((t) => (
                        <TenantCard
                            key={t.tenantId}
                            summary={t}
                            castVotesByElection={castVotesByElection}
                        />
                    ))
                )}
            </section>

            <VotersPanel />
            <CheckpointsPanel />
        </main>
    )
}

function TenantCard({
    summary,
    castVotesByElection,
}: {
    summary: TenantSummary
    castVotesByElection: RootState["castVotes"]
}) {
    const electionVotes = summary.electionIds.reduce(
        (n, id) => n + (castVotesByElection[id]?.length ?? 0),
        0
    )
    return (
        <div style={styles.card}>
            <Link to={`/wb/tenant/${summary.tenantId}`}>
                <strong style={styles.mono}>{summary.tenantId}</strong>
            </Link>
            <div style={{fontSize: "0.85rem", color: "#666", marginTop: "0.25rem"}}>
                {summary.eventIds.length} event
                {summary.eventIds.length === 1 ? "" : "s"},{" "}
                {summary.electionIds.length} election
                {summary.electionIds.length === 1 ? "" : "s"},{" "}
                {electionVotes} cast vote
                {electionVotes === 1 ? "" : "s"}
            </div>
        </div>
    )
}

/**
 * Tenant detail page. Lists the tenant's events.
 */
export function WorkbenchTenant() {
    const {tenantId} = useParams<{tenantId: string}>()
    const events = useSelector((state: RootState) =>
        Object.values(state.electionEvent).filter(
            (e) => e && e.tenant_id === tenantId
        )
    )
    return (
        <main style={styles.main}>
            <p style={styles.crumbs}>
                <Link to="/">Workbench</Link> / tenant
            </p>
            <h1 style={styles.h1}>Tenant</h1>
            <div style={styles.mono}>{tenantId}</div>

            <section style={styles.section}>
                <div style={styles.sectionTitle}>
                    Events ({events.length})
                </div>
                {events.length === 0 ? (
                    <p style={styles.empty}>No events for this tenant.</p>
                ) : (
                    events.map(
                        (e) =>
                            e && (
                                <div key={e.id} style={styles.card}>
                                    <Link to={`/wb/tenant/${tenantId}/event/${e.id}`}>
                                        <strong>{e.name ?? "(unnamed event)"}</strong>
                                    </Link>
                                    <div
                                        style={{
                                            ...styles.mono,
                                            color: "#666",
                                            marginTop: "0.25rem",
                                        }}
                                    >
                                        {e.id}
                                    </div>
                                    {e.description && (
                                        <div
                                            style={{
                                                fontSize: "0.85rem",
                                                color: "#444",
                                                marginTop: "0.25rem",
                                            }}
                                        >
                                            {e.description}
                                        </div>
                                    )}
                                </div>
                            )
                    )
                )}
            </section>
        </main>
    )
}

/**
 * Event detail page. Lists the event's elections.
 */
export function WorkbenchEvent() {
    const {tenantId, eventId} = useParams<{
        tenantId: string
        eventId: string
    }>()
    const event = useSelector(
        (state: RootState) => state.electionEvent[eventId ?? ""]
    )
    const elections = useSelector((state: RootState) =>
        Object.values(state.elections).filter(
            (el) => el && el.election_event_id === eventId
        )
    )
    return (
        <main style={styles.main}>
            <p style={styles.crumbs}>
                <Link to="/">Workbench</Link> /{" "}
                <Link to={`/wb/tenant/${tenantId}`}>tenant</Link> / event
            </p>
            <h1 style={styles.h1}>{event?.name ?? "(unknown event)"}</h1>
            <div style={styles.mono}>{eventId}</div>

            <section style={styles.section}>
                <div style={styles.sectionTitle}>
                    Elections ({elections.length})
                </div>
                {elections.length === 0 ? (
                    <p style={styles.empty}>No elections in this event.</p>
                ) : (
                    elections.map(
                        (el) =>
                            el && (
                                <div key={el.id} style={styles.card}>
                                    <Link
                                        to={`/wb/tenant/${tenantId}/event/${eventId}/election/${el.id}`}
                                    >
                                        <strong>
                                            {el.name ?? "(unnamed election)"}
                                        </strong>
                                    </Link>
                                    <div
                                        style={{
                                            ...styles.mono,
                                            color: "#666",
                                            marginTop: "0.25rem",
                                        }}
                                    >
                                        {el.id}
                                    </div>
                                </div>
                            )
                    )
                )}
            </section>
        </main>
    )
}

/**
 * Election detail page. Shows the election's metadata, the cast votes
 * recorded against it (and against the event id, due to the demo's
 * known election_id/event_id conflation), and a CTA into the booth.
 */
export function WorkbenchElection() {
    const {tenantId, eventId, electionId} = useParams<{
        tenantId: string
        eventId: string
        electionId: string
    }>()
    const navigate = useNavigate()
    const election = useSelector(
        (state: RootState) => state.elections[electionId ?? ""]
    )
    const event = useSelector(
        (state: RootState) => state.electionEvent[eventId ?? ""]
    )
    const ballotStyle = useSelector((state: RootState) =>
        Object.values(state.ballotStyles).find(
            (bs) => bs && bs.election_id === electionId
        )
    )
    // Cast votes for this election. The `castVotes` slice is keyed by
    // `castVote.election_id`; the demo helper now sets that to the
    // real election id (see LIFTING.md section L), so a single lookup
    // suffices.
    const electionVotes = useSelector(
        (state: RootState) => state.castVotes[electionId ?? ""] ?? []
    )
    // Workbench-only state: which voter the operator is currently
    // impersonating, plus the (workbench-managed) cast-vote -> voter
    // attribution ledger.
    const voters = useWorkbench((s) => s.voters)
    const activeVoterId = useWorkbench((s) => s.activeVoterId)
    const castBy = useWorkbench((s) => s.castBy)
    const repairedCastVotes = useWorkbench((s) => s.repairedCastVotes)
    const activeVoter = voters.find((v) => v.id === activeVoterId) ?? null
    const voterById = new Map(voters.map((v) => [v.id, v]))

    const boothStart = `/tenant/${tenantId}/event/${eventId}/election/${electionId}/start`
    const boothChooser = `/tenant/${tenantId}/event/${eventId}/election-chooser`

    /** Render the "Voted by" cell for a cast vote. The portal's
     *  `useAddFakeCastVote` always writes `voter_id_string: null` in
     *  DISABLE_AUTH mode, so the only meaningful source of attribution
     *  is the workbench's own ledger. */
    const votedByCell = (castVoteId: string) => {
        const voterId = castBy[castVoteId]
        if (!voterId) return <em style={{color: "#999"}}>anonymous</em>
        const voter = voterById.get(voterId)
        if (!voter) {
            return (
                <em style={{color: "#b00"}} title={voterId}>
                    (deleted voter)
                </em>
            )
        }
        return voter.displayName
    }

    return (
        <main style={styles.main}>
            <p style={styles.crumbs}>
                <Link to="/">Workbench</Link> /{" "}
                <Link to={`/wb/tenant/${tenantId}`}>tenant</Link> /{" "}
                <Link to={`/wb/tenant/${tenantId}/event/${eventId}`}>
                    event
                </Link>{" "}
                / election
            </p>
            <h1 style={styles.h1}>{election?.name ?? "(unknown election)"}</h1>
            <div style={styles.mono}>{electionId}</div>
            {election?.description && (
                <p style={{fontSize: "0.9rem", color: "#444"}}>
                    {election.description}
                </p>
            )}

            <section style={styles.section}>
                <div style={styles.sectionTitle}>Launch booth</div>
                <div
                    style={{
                        ...styles.card,
                        display: "flex",
                        gap: "0.5rem",
                        alignItems: "center",
                        flexWrap: "wrap",
                    }}
                >
                    <label style={{fontSize: "0.85rem"}}>Vote as:</label>
                    <select
                        value={activeVoterId ?? ""}
                        onChange={(e) =>
                            setActiveVoter(e.target.value || null)
                        }
                        style={{
                            padding: "0.3rem 0.5rem",
                            fontSize: "0.85rem",
                            border: "1px solid #bbb",
                            borderRadius: "4px",
                        }}
                    >
                        <option value="">(anonymous)</option>
                        {voters.map((v) => (
                            <option key={v.id} value={v.id}>
                                {v.displayName}
                            </option>
                        ))}
                    </select>
                    <button
                        type="button"
                        onClick={() => navigate(boothStart)}
                        style={{...styles.cta, border: "none", cursor: "pointer"}}
                    >
                        {activeVoter
                            ? `Start voting as ${activeVoter.displayName} →`
                            : "Start voting (anonymous) →"}
                    </button>
                    <Link
                        to={boothChooser}
                        style={{...styles.cta, background: "#666"}}
                    >
                        Open chooser
                    </Link>
                </div>
                <p
                    style={{
                        fontSize: "0.8rem",
                        color: "#777",
                        marginTop: "0.5rem",
                    }}
                >
                    The booth still runs under DISABLE_AUTH (no real auth
                    handshake). "Vote as X" tags the resulting cast vote in
                    the workbench's own attribution ledger; the portal's{" "}
                    <code>voter_id_string</code> remains <code>null</code> in
                    Redux (the workbench does not modify portal source).
                </p>
            </section>

            <section style={styles.section}>
                <div style={styles.sectionTitle}>
                    Cast votes ({electionVotes.length})
                </div>
                {electionVotes.length === 0 ? (
                    <p style={styles.empty}>
                        No cast votes yet. Use the booth CTA above and
                        complete the flow.
                    </p>
                ) : (
                    <div>
                        {electionVotes.map((cv) => (
                            <CastVoteRow
                                key={cv.id}
                                castVote={cv}
                                voterCell={votedByCell(cv.id)}
                                plaintextSelection={
                                    repairedCastVotes[cv.id]?.selection
                                }
                            />
                        ))}
                    </div>
                )}
            </section>

            {ballotStyle && (
                <TallySection
                    ballotStyle={ballotStyle}
                    castVotes={electionVotes}
                    repairedCastVotes={repairedCastVotes}
                />
            )}

            {ballotStyle && (
                <section style={styles.section}>
                    <div style={styles.sectionTitle}>Ballot style</div>
                    <div style={styles.mono}>{ballotStyle.id}</div>
                    <div style={{fontSize: "0.85rem", color: "#666"}}>
                        {ballotStyle.ballot_eml.contests.length} contest
                        {ballotStyle.ballot_eml.contests.length === 1 ? "" : "s"}
                        {ballotStyle.ballot_eml.public_key?.is_demo
                            ? " · demo key"
                            : ""}
                    </div>
                </section>
            )}

            {event && (
                <section style={styles.section}>
                    <div style={styles.sectionTitle}>Parent event</div>
                    <Link to={`/wb/tenant/${tenantId}/event/${eventId}`}>
                        {event.name ?? "(unnamed)"}
                    </Link>
                </section>
            )}
        </main>
    )
}

/**
 * Named-checkpoints panel. Lets the operator save the current Redux
 * state under a name, list previously-saved checkpoints, and either
 * load or delete them.
 *
 * Two important semantics to remember:
 *  1. Saving a checkpoint does NOT pause the auto-resume slot. The
 *     auto-resume slot keeps tracking every dispatch. Checkpoints are
 *     side-stored copies.
 *  2. Loading a checkpoint rewrites the auto-resume slot (because
 *     hydrateFromSnapshot ends with a forced writeSnapshot). So after
 *     loading, a reload picks up the checkpoint's state as the new
 *     baseline.
 *
 * We deliberately reload the page after a load to drop any in-memory
 * derived state (Apollo cache, currently-mounted screens with their
 * own useState) and let the boot path replay hydration cleanly. The
 * alternative — live-hot-swapping state under mounted booth screens —
 * is full of subtle bugs (selections refer to election IDs that just
 * changed underneath them) and not worth the complexity for a tool
 * intended for short, scripted demo runs.
 */
function CheckpointsPanel() {
    const [checkpoints, setCheckpoints] = useState<CheckpointMeta[]>(() =>
        listCheckpoints()
    )
    const [draftName, setDraftName] = useState("")
    const [error, setError] = useState<string | null>(null)

    const refresh = useCallback(() => setCheckpoints(listCheckpoints()), [])

    const onSave = useCallback(() => {
        setError(null)
        try {
            saveCheckpoint(store, draftName)
            setDraftName("")
            refresh()
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e))
        }
    }, [draftName, refresh])

    const onLoad = useCallback(
        (name: string) => {
            // Confirm because loading is destructive to the current
            // session: the auto-resume slot will be overwritten.
            if (
                !confirm(
                    `Load checkpoint "${name}"? This overwrites the current workbench state and reloads the page.`
                )
            ) {
                return
            }
            const ok = loadCheckpoint(store, name)
            if (!ok) {
                setError(`Checkpoint "${name}" could not be loaded.`)
                return
            }
            location.reload()
        },
        []
    )

    const onDelete = useCallback(
        (name: string) => {
            if (!confirm(`Delete checkpoint "${name}"?`)) return
            deleteCheckpoint(name)
            refresh()
        },
        [refresh]
    )

    return (
        <section style={styles.section}>
            <div style={styles.sectionTitle}>
                Checkpoints ({checkpoints.length})
            </div>
            <div style={styles.card}>
                <div
                    style={{
                        display: "flex",
                        gap: "0.5rem",
                        alignItems: "center",
                    }}
                >
                    <input
                        type="text"
                        placeholder="Checkpoint name"
                        value={draftName}
                        onChange={(e) => setDraftName(e.target.value)}
                        onKeyDown={(e) => {
                            if (e.key === "Enter") onSave()
                        }}
                        style={{
                            flex: 1,
                            padding: "0.35rem 0.5rem",
                            fontSize: "0.85rem",
                            border: "1px solid #bbb",
                            borderRadius: "4px",
                        }}
                    />
                    <button
                        type="button"
                        onClick={onSave}
                        disabled={draftName.trim().length === 0}
                        style={{
                            padding: "0.35rem 0.8rem",
                            fontSize: "0.85rem",
                            cursor:
                                draftName.trim().length === 0
                                    ? "default"
                                    : "pointer",
                        }}
                    >
                        Save current state
                    </button>
                </div>
                {error && (
                    <p
                        style={{
                            color: "#b00",
                            fontSize: "0.8rem",
                            marginTop: "0.4rem",
                            marginBottom: 0,
                        }}
                    >
                        {error}
                    </p>
                )}
                <p
                    style={{
                        ...styles.empty,
                        marginTop: "0.4rem",
                        marginBottom: 0,
                    }}
                >
                    Saves a snapshot of the Redux store to localStorage
                    under <code>workbench:checkpoint:v1:&lt;name&gt;</code>.
                    The auto-resume slot is unaffected.
                </p>
            </div>
            {checkpoints.length === 0 ? (
                <p style={styles.empty}>No saved checkpoints yet.</p>
            ) : (
                <table style={styles.table}>
                    <thead>
                        <tr>
                            <th style={styles.th}>Name</th>
                            <th style={styles.th}>Saved at</th>
                            <th style={styles.th}></th>
                        </tr>
                    </thead>
                    <tbody>
                        {checkpoints.map((c) => (
                            <tr key={c.name}>
                                <td style={styles.td}>{c.name}</td>
                                <td style={{...styles.td, ...styles.mono}}>
                                    {c.savedAt}
                                </td>
                                <td
                                    style={{
                                        ...styles.td,
                                        textAlign: "right",
                                    }}
                                >
                                    <button
                                        type="button"
                                        onClick={() => onLoad(c.name)}
                                        style={{
                                            marginRight: "0.4rem",
                                            padding: "0.2rem 0.55rem",
                                            fontSize: "0.8rem",
                                            cursor: "pointer",
                                        }}
                                    >
                                        Load
                                    </button>
                                    <button
                                        type="button"
                                        onClick={() => onDelete(c.name)}
                                        style={{
                                            padding: "0.2rem 0.55rem",
                                            fontSize: "0.8rem",
                                            cursor: "pointer",
                                        }}
                                    >
                                        Delete
                                    </button>
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            )}
        </section>
    )
}

/**
 * Voter directory + "active voter" picker on the homepage.
 *
 * Workbench-owned state, NOT stored in the voting-portal Redux store
 * (there is no `voters` slice in production; adding one would mean
 * touching `voting-portal/src/store/store.ts`, which we explicitly
 * refuse — see LIFTING.md section I). The directory lives in
 * `workbenchStore.ts` instead and is folded into the same persisted
 * snapshot the portal store rides on, so checkpoints capture both.
 *
 * "Active voter" semantics: setting the active voter does NOT change
 * how the booth screens behave — DISABLE_AUTH is still in effect and
 * the portal's `useAddFakeCastVote` still writes
 * `voter_id_string: null`. What it does change is the workbench's
 * own attribution ledger: the cast-votes watcher in
 * `installPersistence` tags every newly-observed cast vote with
 * whichever voter was active at the time. Election-detail pages then
 * render that attribution in the "Voted by" column.
 */
function VotersPanel() {
    const voters = useWorkbench((s) => s.voters)
    const activeVoterId = useWorkbench((s) => s.activeVoterId)
    const [draftName, setDraftName] = useState("")
    const [error, setError] = useState<string | null>(null)

    const onAdd = useCallback(() => {
        setError(null)
        try {
            addVoter(draftName)
            setDraftName("")
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e))
        }
    }, [draftName])

    const onRemove = useCallback((voter: Voter) => {
        if (
            !confirm(
                `Remove voter "${voter.displayName}"? Their attribution rows in the cast-vote ledger will be dropped (the cast-vote records themselves stay in Redux).`
            )
        ) {
            return
        }
        removeVoter(voter.id)
    }, [])

    return (
        <section style={styles.section}>
            <div style={styles.sectionTitle}>
                Voters ({voters.length})
                {activeVoterId && (
                    <span
                        style={{
                            float: "right",
                            color: "#1976d2",
                            textTransform: "none",
                            letterSpacing: 0,
                            fontWeight: 600,
                        }}
                    >
                        active:{" "}
                        {voters.find((v) => v.id === activeVoterId)
                            ?.displayName ?? "(unknown)"}
                    </span>
                )}
            </div>
            <div style={styles.card}>
                <div
                    style={{
                        display: "flex",
                        gap: "0.5rem",
                        alignItems: "center",
                    }}
                >
                    <input
                        type="text"
                        placeholder="Voter display name"
                        value={draftName}
                        onChange={(e) => setDraftName(e.target.value)}
                        onKeyDown={(e) => {
                            if (e.key === "Enter") onAdd()
                        }}
                        style={{
                            flex: 1,
                            padding: "0.35rem 0.5rem",
                            fontSize: "0.85rem",
                            border: "1px solid #bbb",
                            borderRadius: "4px",
                        }}
                    />
                    <button
                        type="button"
                        onClick={onAdd}
                        disabled={draftName.trim().length === 0}
                        style={{
                            padding: "0.35rem 0.8rem",
                            fontSize: "0.85rem",
                            cursor:
                                draftName.trim().length === 0
                                    ? "default"
                                    : "pointer",
                        }}
                    >
                        Add voter
                    </button>
                </div>
                {error && (
                    <p
                        style={{
                            color: "#b00",
                            fontSize: "0.8rem",
                            marginTop: "0.4rem",
                            marginBottom: 0,
                        }}
                    >
                        {error}
                    </p>
                )}
                <p
                    style={{
                        ...styles.empty,
                        marginTop: "0.4rem",
                        marginBottom: 0,
                    }}
                >
                    Voters are workbench-only. Selecting one as active
                    tags subsequent cast votes in the attribution ledger;
                    the booth still runs under DISABLE_AUTH and the
                    portal's <code>voter_id_string</code> stays{" "}
                    <code>null</code>.
                </p>
            </div>
            {voters.length === 0 ? (
                <p style={styles.empty}>No voters yet.</p>
            ) : (
                <table style={styles.table}>
                    <thead>
                        <tr>
                            <th style={styles.th}>Active</th>
                            <th style={styles.th}>Display name</th>
                            <th style={styles.th}>ID</th>
                            <th style={styles.th}></th>
                        </tr>
                    </thead>
                    <tbody>
                        {voters.map((v) => (
                            <tr key={v.id}>
                                <td style={styles.td}>
                                    <input
                                        type="radio"
                                        name="active-voter"
                                        checked={activeVoterId === v.id}
                                        onChange={() =>
                                            setActiveVoter(v.id)
                                        }
                                        aria-label={`Make ${v.displayName} the active voter`}
                                    />
                                </td>
                                <td style={styles.td}>{v.displayName}</td>
                                <td style={{...styles.td, ...styles.mono}}>
                                    {v.id}
                                </td>
                                <td
                                    style={{
                                        ...styles.td,
                                        textAlign: "right",
                                    }}
                                >
                                    <button
                                        type="button"
                                        onClick={() => onRemove(v)}
                                        style={{
                                            padding: "0.2rem 0.55rem",
                                            fontSize: "0.8rem",
                                            cursor: "pointer",
                                        }}
                                    >
                                        Remove
                                    </button>
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            )}
            {activeVoterId && (
                <p
                    style={{
                        ...styles.empty,
                        marginTop: "0.4rem",
                    }}
                >
                    <button
                        type="button"
                        onClick={() => setActiveVoter(null)}
                        style={{
                            padding: "0.2rem 0.55rem",
                            fontSize: "0.8rem",
                            cursor: "pointer",
                            marginRight: "0.5rem",
                        }}
                    >
                        Clear active voter
                    </button>
                    Resets booth launches back to anonymous attribution.
                </p>
            )}
        </section>
    )
}

/**
 * Inline per-election tally panel.
 *
 * Runs the same `sequent-core` codec + tally that velvet would run
 * post-decryption in production, entirely in the browser via
 * `velvet-wasm`. Inputs:
 *   - `ballotStyle.ballot_eml.contests[i]` JSON, fed verbatim to the
 *     WASM (same serde shape as Rust's `Contest`);
 *   - the workbench-bridged `IDecodedVoteContest[]` per cast vote
 *     (from `repairedCastVotes`), encoded to BigUint via
 *     `encode_ballot` and aggregated via `tally_plaintext_ballots`.
 *
 * No portal-source changes — the bridge already captures everything
 * we need (`state.ballotSelections[electionId]` snapshot).
 */
function TallySection({
    ballotStyle,
    castVotes,
    repairedCastVotes,
}: {
    ballotStyle: IBallotStyle
    castVotes: ICastVote[]
    repairedCastVotes: Record<string, {selection: unknown}>
}) {
    const [outcomes, setOutcomes] = useState<ContestTallyOutcome[] | null>(
        null
    )
    const [busy, setBusy] = useState(false)
    const [error, setError] = useState<string | null>(null)

    const captured = castVotes.filter(
        (cv) => repairedCastVotes[cv.id]?.selection
    )

    const handleRun = useCallback(async () => {
        setBusy(true)
        setError(null)
        try {
            const selections = captured.map(
                (cv) => repairedCastVotes[cv.id]!.selection
            )
            const result = await runElectionTally(ballotStyle, selections)
            setOutcomes(result)
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e))
        } finally {
            setBusy(false)
        }
    }, [ballotStyle, captured, repairedCastVotes])

    const skipped = castVotes.length - captured.length

    return (
        <section style={styles.section}>
            <div style={styles.sectionTitle}>Tally</div>
            <p style={styles.empty}>
                Run the contest tally over the workbench-bridged
                plaintext selections, using the same{" "}
                <code>sequent-core</code> codec the production tally
                workflow uses (via <code>velvet-wasm</code>, in the
                browser).
                {skipped > 0
                    ? ` ${skipped} cast vote${skipped === 1 ? "" : "s"} skipped (no captured selection).`
                    : ""}
            </p>
            <button
                type="button"
                onClick={handleRun}
                disabled={busy || captured.length === 0}
                style={{
                    padding: "0.4rem 0.8rem",
                    cursor:
                        busy || captured.length === 0
                            ? "not-allowed"
                            : "pointer",
                }}
            >
                {busy
                    ? "Tallying…"
                    : outcomes
                      ? "Re-run tally"
                      : `Run tally (${captured.length} ballot${captured.length === 1 ? "" : "s"})`}
            </button>
            {error && (
                <p style={{color: "#b00", marginTop: "0.5rem"}}>{error}</p>
            )}
            {outcomes && (
                <div style={{marginTop: "0.5rem"}}>
                    {outcomes.map((o) => (
                        <TallyContestCard key={o.contestId} outcome={o} />
                    ))}
                </div>
            )}
        </section>
    )
}

function TallyContestCard({outcome}: {outcome: ContestTallyOutcome}) {
    return (
        <details
            open
            style={{...styles.card, padding: "0.5rem 0.75rem"}}
        >
            <summary style={{cursor: "pointer", fontSize: "0.85rem"}}>
                <strong>{outcome.contestName ?? "(unnamed contest)"}</strong>
                <span style={{color: "#666"}}>
                    {" "}— {outcome.ballotsCounted} ballot
                    {outcome.ballotsCounted === 1 ? "" : "s"} ·{" "}
                    {outcome.status === "ok"
                        ? "tallied"
                        : outcome.status === "no-data"
                          ? "no data"
                          : "error"}
                </span>
            </summary>
            <div style={{marginTop: "0.5rem", fontSize: "0.75rem", color: "#666"}}>
                contest id: <span style={styles.mono}>{outcome.contestId}</span>
            </div>
            {outcome.status === "ok" && (
                <pre
                    style={{
                        ...styles.mono,
                        background: "#f5f5f5",
                        padding: "0.5rem",
                        marginTop: "0.5rem",
                        maxHeight: "24rem",
                        overflow: "auto",
                    }}
                >
                    {JSON.stringify(outcome.result, null, 2)}
                </pre>
            )}
            {outcome.status === "no-data" && (
                <p style={styles.empty}>
                    No cast vote contained a selection for this contest.
                </p>
            )}
            {outcome.status === "error" && (
                <p style={{color: "#b00", marginTop: "0.5rem"}}>
                    {outcome.errorMessage}
                </p>
            )}
        </details>
    )
}

/**
 * One cast vote, rendered as an expandable card. The summary is the
 * minimum the operator needs to scan the list (id, voter, encrypted-
 * content size). Expanding reveals the human-readable selection
 * (captured by the workbench at cast time) and the encrypted ballot
 * exactly as it sits on `cv.content`.
 */
function CastVoteRow({
    castVote,
    voterCell,
    plaintextSelection,
}: {
    castVote: ICastVote
    voterCell: React.ReactNode
    plaintextSelection: unknown
}) {
    const content = castVote.content ?? ""
    return (
        <details style={{...styles.card, padding: "0.5rem 0.75rem"}}>
            <summary style={{cursor: "pointer", fontSize: "0.85rem"}}>
                <span style={styles.mono}>{castVote.id}</span>
                <span style={{color: "#666"}}>
                    {" "}— voted by {voterCell} · encrypted{" "}
                    {content.length} chars
                </span>
            </summary>
            <div style={{marginTop: "0.5rem"}}>
                <div style={styles.sectionTitle}>Selection</div>
                {plaintextSelection ? (
                    <pre
                        style={{
                            ...styles.mono,
                            background: "#f5f5f5",
                            padding: "0.5rem",
                            margin: 0,
                            maxHeight: "20rem",
                            overflow: "auto",
                        }}
                    >
                        {JSON.stringify(plaintextSelection, null, 2)}
                    </pre>
                ) : (
                    <p style={styles.empty}>
                        Not captured by the workbench bridge.
                    </p>
                )}
            </div>
            <div style={{marginTop: "0.5rem"}}>
                <div style={styles.sectionTitle}>Encrypted ballot</div>
                {content.length > 0 ? (
                    <pre
                        style={{
                            ...styles.mono,
                            background: "#f5f5f5",
                            padding: "0.5rem",
                            margin: 0,
                            maxHeight: "12rem",
                            overflow: "auto",
                            whiteSpace: "pre-wrap",
                            wordBreak: "break-all",
                        }}
                    >
                        {(() => {
                            try {
                                return JSON.stringify(
                                    JSON.parse(content),
                                    null,
                                    2
                                )
                            } catch {
                                return content
                            }
                        })()}
                    </pre>
                ) : (
                    <p style={styles.empty}>
                        <code>cv.content</code> is empty.
                    </p>
                )}
            </div>
        </details>
    )
}
