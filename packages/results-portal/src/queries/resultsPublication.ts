// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const RESOLVE_RESULTS_PUBLICATION = `
    query ResolveResultsPublication($eeId: String!, $electionId: String) {
        resolveResultsPublication(ee_id: $eeId, election_id: $electionId) {
            tenant_id
            election_event_id
            access
            route_scope
            election_ids
            publication_id
            manifest_public_path
            manifest_url
            manifest
        }
    }
`

export interface ResolveResultsPublicationVariables {
    eeId: string
    electionId?: string
}

export const FETCH_RESULTS_ARTIFACT = `
    query FetchResultsArtifact(
        $electionEventId: String!
        $electionId: String
        $publicationId: String!
    ) {
        fetchResultsArtifact(
            election_event_id: $electionEventId
            election_id: $electionId
            publication_id: $publicationId
        ) {
            urls
        }
    }
`

export interface FetchResultsArtifactVariables {
    electionEventId: string
    electionId?: string
    publicationId: string
}
