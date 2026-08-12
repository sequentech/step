// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export type ResultsAccess = "public" | "authenticated"
export type ResultsVisibilityScope = "full_event" | "area_based"
export type ResultsRouteScope = "event" | "election"
export type ContestPublicationState = "published" | "not_published"
export type ResultsSqlId = string | number
export type ResultsSqlNumber = string | number | null
export type ResultsJson =
    | string
    | number
    | boolean
    | null
    | ResultsJson[]
    | {[key: string]: ResultsJson}
export type ResultsSerializedJson = string | ResultsJson

export interface ResultsPublicationIndexEntry {
    publication_id: string
    route_scope: ResultsRouteScope
    route?: string | null
    route_election_id?: string | null
    election_ids?: string[]
    access: ResultsAccess
    visibility_scope?: ResultsVisibilityScope
    manifest_public_path?: string | null
    manifest_url?: string | null
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
    public_path?: string | null
    url?: string | null
    document_id?: string | null
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
        full_sqlite?: ResultsManifestArtifact | null
        areas?: Record<string, ResultsManifestArtifact> | null
    }
}

export interface ResultsResolverResponse {
    tenant_id: string
    election_event_id: string
    access: ResultsAccess
    route_scope: ResultsRouteScope
    election_ids: string[]
    publication_id: string
    manifest_public_path?: string | null
    manifest_url?: string | null
    manifest?: ResultsManifest | null
}

export interface ResultsRow {
    id?: ResultsSqlId | null
    election_event_id?: ResultsSqlId | null
    election_id?: ResultsSqlId | null
    contest_id?: ResultsSqlId | null
    candidate_id?: ResultsSqlId | null
    area_id?: ResultsSqlId | null
    results_event_id?: ResultsSqlId | null
    name?: string | null
    description?: string | null
    type?: string | null
    labels?: ResultsSerializedJson
    presentation?: ResultsSerializedJson
    annotations?: ResultsSerializedJson
    external_id?: string | null
    is_public?: boolean | number | null
    is_acclaimed?: boolean | number | null
    is_active?: boolean | number | null
    min_votes?: ResultsSqlNumber
    max_votes?: ResultsSqlNumber
    winning_candidates_num?: ResultsSqlNumber
    voting_type?: string | null
    counting_algorithm?: string | null
    elegible_census?: ResultsSqlNumber
    total_voters?: ResultsSqlNumber
    total_voters_percent?: ResultsSqlNumber
    total_valid_votes?: ResultsSqlNumber
    total_valid_votes_percent?: ResultsSqlNumber
    total_invalid_votes?: ResultsSqlNumber
    total_invalid_votes_percent?: ResultsSqlNumber
    explicit_invalid_votes?: ResultsSqlNumber
    explicit_invalid_votes_percent?: ResultsSqlNumber
    implicit_invalid_votes?: ResultsSqlNumber
    implicit_invalid_votes_percent?: ResultsSqlNumber
    total_blank_votes?: ResultsSqlNumber
    total_blank_votes_percent?: ResultsSqlNumber
    blank_votes?: ResultsSqlNumber
    blank_votes_percent?: ResultsSqlNumber
    explicit_blank_votes?: ResultsSqlNumber
    explicit_blank_votes_percent?: ResultsSqlNumber
    implicit_blank_votes?: ResultsSqlNumber
    implicit_blank_votes_percent?: ResultsSqlNumber
    total_votes?: ResultsSqlNumber
    total_votes_percent?: ResultsSqlNumber
    total_auditable_votes?: ResultsSqlNumber
    total_auditable_votes_percent?: ResultsSqlNumber
    cast_votes?: ResultsSqlNumber
    cast_votes_percent?: ResultsSqlNumber
    winning_position?: ResultsSqlNumber
    points?: ResultsSqlNumber
}

export interface ResultsElectionRow extends ResultsRow {
    election_id: ResultsSqlId
}

export interface ResultsElectionAreaRow extends ResultsElectionRow {
    area_id: ResultsSqlId
}

export interface ResultsContestRow extends ResultsElectionRow {
    contest_id: ResultsSqlId
}

export interface ResultsAreaContestRow extends ResultsContestRow {
    area_id: ResultsSqlId
}

export interface ResultsContestCandidateRow extends ResultsContestRow {
    candidate_id: ResultsSqlId
}

export interface ResultsAreaContestCandidateRow extends ResultsContestCandidateRow {
    area_id: ResultsSqlId
}

export interface ResultsSqliteDataset {
    election_event: ResultsRow[]
    election: ResultsRow[]
    contest: ResultsRow[]
    candidate: ResultsRow[]
    area: ResultsRow[]
    results_event: ResultsRow[]
    results_election: ResultsElectionRow[]
    results_election_area: ResultsElectionAreaRow[]
    results_contest: ResultsContestRow[]
    results_contest_candidate: ResultsContestCandidateRow[]
    results_area_contest: ResultsAreaContestRow[]
    results_area_contest_candidate: ResultsAreaContestCandidateRow[]
}

const isObject = (value: unknown): value is {[key: string]: unknown} =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const isStringArray = (value: unknown): value is string[] =>
    Array.isArray(value) && value.every((item) => typeof item === "string")

const isOptionalString = (value: unknown): value is string | null | undefined =>
    value === undefined || value === null || typeof value === "string"

const isResultsAccess = (value: unknown): value is ResultsAccess =>
    value === "public" || value === "authenticated"

const isResultsVisibilityScope = (value: unknown): value is ResultsVisibilityScope =>
    value === "full_event" || value === "area_based"

const isResultsRouteScope = (value: unknown): value is ResultsRouteScope =>
    value === "event" || value === "election"

const isPublicationIndexEntry = (value: unknown): value is ResultsPublicationIndexEntry =>
    isObject(value) &&
    typeof value.publication_id === "string" &&
    isResultsRouteScope(value.route_scope) &&
    isResultsAccess(value.access) &&
    isOptionalString(value.route) &&
    isOptionalString(value.route_election_id) &&
    (value.election_ids === undefined || isStringArray(value.election_ids)) &&
    (value.visibility_scope === undefined || isResultsVisibilityScope(value.visibility_scope)) &&
    isOptionalString(value.manifest_public_path) &&
    isOptionalString(value.manifest_url)

export const parseResultsPublicationIndex = (value: unknown): ResultsPublicationIndex => {
    if (isPublicationIndexEntry(value)) {
        return {schema_version: 1, publications: [value]}
    }

    if (
        !isObject(value) ||
        typeof value.schema_version !== "number" ||
        (value.publications !== undefined &&
            (!Array.isArray(value.publications) ||
                !value.publications.every(isPublicationIndexEntry)))
    ) {
        throw new Error("Invalid results publication index")
    }

    return value as ResultsPublicationIndex
}

const isManifestContest = (value: unknown): value is ResultsManifestContest =>
    isObject(value) &&
    typeof value.election_id === "string" &&
    typeof value.contest_id === "string" &&
    (value.publication_state === "published" || value.publication_state === "not_published")

const isManifestArtifact = (value: unknown): value is ResultsManifestArtifact =>
    isObject(value) &&
    isOptionalString(value.public_path) &&
    isOptionalString(value.url) &&
    isOptionalString(value.document_id)

const isManifestArtifactMap = (value: unknown): value is Record<string, ResultsManifestArtifact> =>
    isObject(value) && Object.values(value).every(isManifestArtifact)

const isManifestArtifacts = (value: unknown): value is ResultsManifest["artifacts"] =>
    isObject(value) &&
    (value.full_sqlite === undefined ||
        value.full_sqlite === null ||
        isManifestArtifact(value.full_sqlite)) &&
    (value.areas === undefined || value.areas === null || isManifestArtifactMap(value.areas))

export const parseResultsManifest = (value: unknown): ResultsManifest => {
    if (
        !isObject(value) ||
        typeof value.schema_version !== "number" ||
        typeof value.tenant_id !== "string" ||
        typeof value.election_event_id !== "string" ||
        !isStringArray(value.election_ids) ||
        !isResultsRouteScope(value.route_scope) ||
        typeof value.publication_id !== "string" ||
        typeof value.results_event_id !== "string" ||
        typeof value.version !== "number" ||
        !isResultsAccess(value.access) ||
        !isResultsVisibilityScope(value.visibility_scope) ||
        !Array.isArray(value.contests) ||
        !value.contests.every(isManifestContest) ||
        !isManifestArtifacts(value.artifacts)
    ) {
        throw new Error("Invalid results publication manifest")
    }

    return value as ResultsManifest
}
