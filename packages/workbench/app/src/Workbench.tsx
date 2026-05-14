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

import {Link, useParams} from "react-router-dom"
import {useSelector} from "react-redux"
import type {CSSProperties} from "react"
import type {RootState} from "voting-portal/src/store/store"

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
    // The demo's `useAddFakeCastVote` indexes cast votes by event_id, not
    // election_id (see LIFTING.md "Concession-ish quirks"), so we sum
    // those too to show an honest total.
    const eventVotes = summary.eventIds.reduce(
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
                {electionVotes + eventVotes} cast vote
                {electionVotes + eventVotes === 1 ? "" : "s"}
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
    // Cast votes are indexed in the slice by `castVote.election_id`, but
    // `useAddFakeCastVote` sets that field to the event id. To be honest
    // about what's in state we surface BOTH bins.
    const electionVotes = useSelector(
        (state: RootState) => state.castVotes[electionId ?? ""] ?? []
    )
    const eventBinVotes = useSelector(
        (state: RootState) => state.castVotes[eventId ?? ""] ?? []
    )

    const boothStart = `/tenant/${tenantId}/event/${eventId}/election/${electionId}/start`
    const boothChooser = `/tenant/${tenantId}/event/${eventId}/election-chooser`

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
                <Link to={boothStart} style={styles.cta}>
                    Start voting for this election →
                </Link>{" "}
                <Link
                    to={boothChooser}
                    style={{...styles.cta, background: "#666"}}
                >
                    Open chooser
                </Link>
                <p
                    style={{
                        fontSize: "0.8rem",
                        color: "#777",
                        marginTop: "0.5rem",
                    }}
                >
                    The booth runs as anonymous voter (DISABLE_AUTH=true).
                    Voter selection will land here once a voters slice is
                    introduced.
                </p>
            </section>

            <section style={styles.section}>
                <div style={styles.sectionTitle}>
                    Cast votes ({electionVotes.length + eventBinVotes.length})
                </div>
                {electionVotes.length + eventBinVotes.length === 0 ? (
                    <p style={styles.empty}>
                        No cast votes yet. Use the booth CTA above and
                        complete the flow.
                    </p>
                ) : (
                    <table style={styles.table}>
                        <thead>
                            <tr>
                                <th style={styles.th}>Cast vote ID</th>
                                <th style={styles.th}>Indexed under</th>
                                <th style={styles.th}>Content length</th>
                            </tr>
                        </thead>
                        <tbody>
                            {electionVotes.map((cv) => (
                                <tr key={`el-${cv.id}`}>
                                    <td style={{...styles.td, ...styles.mono}}>
                                        {cv.id}
                                    </td>
                                    <td style={styles.td}>election_id</td>
                                    <td style={styles.td}>
                                        {(cv.content ?? "").length}
                                    </td>
                                </tr>
                            ))}
                            {eventBinVotes.map((cv) => (
                                <tr key={`ev-${cv.id}`}>
                                    <td style={{...styles.td, ...styles.mono}}>
                                        {cv.id}
                                    </td>
                                    <td style={styles.td}>
                                        event_id{" "}
                                        <em
                                            style={{
                                                color: "#999",
                                                fontSize: "0.8em",
                                            }}
                                        >
                                            (demo path)
                                        </em>
                                    </td>
                                    <td style={styles.td}>
                                        {(cv.content ?? "").length}
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                )}
                <p
                    style={{
                        fontSize: "0.8rem",
                        color: "#777",
                        marginTop: "0.5rem",
                    }}
                >
                    Note: the demo's <code>useAddFakeCastVote</code> writes
                    cast-vote records with empty <code>content</code>{" "}
                    (encrypted ciphertext lives in <code>sessionStorage</code>{" "}
                    under <code>ballotData</code> until a bridging
                    middleware grafts it into Redux). Running a real
                    tally over these is a follow-up commit.
                </p>
            </section>

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
