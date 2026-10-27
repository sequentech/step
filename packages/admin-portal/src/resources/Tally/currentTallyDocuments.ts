// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// Report approval changes these references after the immutable tally SQLite
// snapshot is written. Read only the references; retain the snapshot's results.
export const GET_CURRENT_TALLY_DOCUMENTS = gql`
    query GetCurrentTallyDocuments(
        $tenantId: uuid!
        $electionEventId: uuid!
        $resultsEventId: uuid!
        $electionIds: [uuid!]!
    ) {
        sequent_backend_results_election(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                results_event_id: {_eq: $resultsEventId}
                election_id: {_in: $electionIds}
            }
        ) {
            id
            tenant_id
            election_event_id
            results_event_id
            election_id
            documents
        }
        sequent_backend_results_election_area(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                results_event_id: {_eq: $resultsEventId}
                election_id: {_in: $electionIds}
            }
        ) {
            id
            tenant_id
            election_event_id
            results_event_id
            election_id
            area_id
            documents
        }
    }
`

export interface CurrentDocumentRow {
    id: string
    tenant_id: string
    election_event_id: string
    results_event_id: string
    election_id: string
    area_id?: string | null
    documents: Record<string, unknown> | null
}
export interface CurrentTallyDocuments {
    sequent_backend_results_election: CurrentDocumentRow[]
    sequent_backend_results_election_area: CurrentDocumentRow[]
}
export interface DocumentScope {
    tenantId: string | null
    electionEventId: string
    resultsEventId: string
    electionIds: string[]
}

function resultsDocument(raw: unknown): string | null {
    try {
        const documents: unknown = typeof raw === "string" ? JSON.parse(raw) : raw
        if (!documents || typeof documents !== "object" || Array.isArray(documents)) return null
        const json = (documents as Record<string, unknown>).json
        return typeof json === "string" && json.length > 0 ? json : null
    } catch {
        return null
    }
}

/** Never carry a stale export reference across a missing or denied current read. */
export function overlayCurrentDocuments<T extends Record<string, unknown>>(
    rows: T[],
    current: CurrentDocumentRow[] | undefined,
    scope: DocumentScope
): T[] {
    return rows.map((row) => {
        // SQLite and PostgreSQL independently allocate storage row IDs. The
        // result scope and immutable JSON document identify the same tally.
        const snapshotDocument = resultsDocument(row.documents)
        const matches = (current ?? []).filter(
            (live) =>
                snapshotDocument !== null &&
                resultsDocument(live.documents) === snapshotDocument &&
                live.tenant_id === scope.tenantId &&
                live.tenant_id === row.tenant_id &&
                live.election_event_id === scope.electionEventId &&
                live.election_event_id === row.election_event_id &&
                live.results_event_id === scope.resultsEventId &&
                live.results_event_id === row.results_event_id &&
                scope.electionIds.includes(live.election_id) &&
                live.election_id === row.election_id &&
                (live.area_id ?? null) === (row.area_id ?? null)
        )
        const documents = matches.length === 1 ? matches[0].documents : null
        return {
            ...row,
            documents:
                documents && typeof documents === "object" && !Array.isArray(documents)
                    ? documents
                    : {},
        }
    })
}
