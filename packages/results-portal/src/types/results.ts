// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export type ResultsAccess = "public" | "authenticated"
export type ResultsVisibilityScope = "full_event" | "area_based"
export type ResultsRouteScope = "event" | "election"
export type ContestPublicationState = "published" | "not_published"

export interface ResultsPublicationIndexEntry {
    publication_id: string
    route_scope: ResultsRouteScope
    route?: string
    route_election_id?: string | null
    election_ids?: string[]
    access: ResultsAccess
    visibility_scope?: ResultsVisibilityScope
    manifest_public_path?: string
    manifest_url?: string
}

export interface ResultsPublicationIndex {
    schema_version: number
    tenant_id?: string
    election_event_id?: string
    publications?: ResultsPublicationIndexEntry[]
}

export interface ResultsManifestContest {
    election_id: string
    contest_id: string
    area_id?: string | null
    publication_state: ContestPublicationState
    positions?: number | null
}

export interface ResultsManifestArtifact {
    public_path?: string
    url?: string
    document_id?: string
}

export interface ResultsManifestCustomCss {
    election_event?: string | null
    elections?: Record<string, string | null>
}

export interface ResultsManifest {
    schema_version: number
    tenant_id: string
    election_event_id: string
    election_ids: string[]
    route_scope: ResultsRouteScope
    route_election_id?: string | null
    publication_id: string
    tally_session_id?: string
    tally_session_execution_id?: string
    results_event_id: string
    version: number
    published_at?: string
    access: ResultsAccess
    visibility_scope: ResultsVisibilityScope
    default_locale?: string
    available_languages?: string[]
    title?: Record<string, string> | string
    custom_css?: ResultsManifestCustomCss
    contests: ResultsManifestContest[]
    artifacts: {
        full_sqlite?: ResultsManifestArtifact
        areas?: Record<string, ResultsManifestArtifact>
    }
}

export interface ResultsResolverResponse {
    tenant_id: string
    election_event_id: string
    access: ResultsAccess
    route_scope: ResultsRouteScope
    election_ids: string[]
    publication_id: string
    manifest_public_path?: string
    manifest_url?: string
    manifest?: ResultsManifest
}

export type ResultsRow = Record<string, unknown>

export interface ResultsSqliteDataset {
    election_event: ResultsRow[]
    election: ResultsRow[]
    contest: ResultsRow[]
    candidate: ResultsRow[]
    area: ResultsRow[]
    results_event: ResultsRow[]
    results_election: ResultsRow[]
    results_election_area: ResultsRow[]
    results_contest: ResultsRow[]
    results_contest_candidate: ResultsRow[]
    results_area_contest: ResultsRow[]
    results_area_contest_candidate: ResultsRow[]
}
