// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {
    ResultsManifest,
    ResultsPublicationIndex,
    ResultsSqliteDataset,
} from "../../src/types/results"

export const resultsIds = {
    tenant: "tenant-results",
    event: "event-results",
    election: "council",
    contest: "representative",
} as const

/** Synthetic publication rows; assertions use literal vote totals and labels. */
export function resultsFixture() {
    const dataset: ResultsSqliteDataset = {
        election_event: [
            {
                id: resultsIds.event,
                name: "Community Election",
                presentation: {elections_order: "custom"},
            },
        ],
        election: [
            {
                id: "council",
                name: "Community Council",
                presentation: {sort_order: 0, contests_order: "custom"},
            },
            {
                id: "school",
                name: "School Board",
                presentation: {sort_order: 1, contests_order: "custom"},
            },
        ],
        contest: [
            {
                id: "representative",
                election_id: "council",
                name: "Council representative",
                counting_algorithm: "plurality-at-large",
                presentation: {candidates_order: "custom", sort_order: 0},
            },
            {
                id: "school-seat",
                election_id: "school",
                name: "School representative",
                counting_algorithm: "plurality-at-large",
                presentation: {candidates_order: "custom", sort_order: 0},
            },
        ],
        candidate: [
            {
                id: "alice",
                contest_id: "representative",
                name: "Alice Example",
                presentation: {sort_order: 0},
            },
            {
                id: "bob",
                contest_id: "representative",
                name: "Bob Example",
                presentation: {sort_order: 1},
            },
            {
                id: "charlie",
                contest_id: "school-seat",
                name: "Charlie Example",
                presentation: {sort_order: 0},
            },
        ],
        area: [
            {id: "north", name: "North district"},
            {id: "south", name: "South district"},
        ],
        results_event: [{id: "results-event", election_event_id: resultsIds.event}],
        results_election: [
            {
                id: "election-total",
                election_id: "council",
                elegible_census: 100,
                total_voters: 80,
                total_voters_percent: 0.8,
                total_valid_votes: 75,
                blank_ballots: 3,
            },
            {
                id: "school-total",
                election_id: "school",
                elegible_census: 50,
                total_voters: 20,
                total_voters_percent: 0.4,
                total_valid_votes: 20,
                blank_ballots: 0,
            },
        ],
        results_election_area: [],
        results_contest: [
            {
                id: "contest-total",
                election_id: "council",
                contest_id: "representative",
                elegible_census: 100,
                total_votes: 80,
                total_votes_percent: 0.8,
                total_valid_votes: 75,
                total_valid_votes_percent: 0.9375,
                total_invalid_votes: 2,
                total_invalid_votes_percent: 0.025,
                total_blank_votes: 3,
                total_blank_votes_percent: 0.0375,
            },
            {
                id: "school-contest-total",
                election_id: "school",
                contest_id: "school-seat",
                elegible_census: 50,
                total_votes: 20,
                total_valid_votes: 20,
            },
        ],
        results_contest_candidate: [
            {
                election_id: "council",
                contest_id: "representative",
                candidate_id: "alice",
                cast_votes: 45,
                cast_votes_percent: 0.6,
                winning_position: 1,
            },
            {
                election_id: "council",
                contest_id: "representative",
                candidate_id: "bob",
                cast_votes: 30,
                cast_votes_percent: 0.4,
                winning_position: null,
            },
            {
                election_id: "school",
                contest_id: "school-seat",
                candidate_id: "charlie",
                cast_votes: 20,
                cast_votes_percent: 1.0,
                winning_position: 1,
            },
        ],
        results_area_contest: [
            {
                id: "north-total",
                election_id: "council",
                contest_id: "representative",
                area_id: "north",
                elegible_census: 40,
                total_votes: 30,
                total_valid_votes: 30,
            },
        ],
        results_area_contest_candidate: [
            {
                election_id: "council",
                contest_id: "representative",
                area_id: "north",
                candidate_id: "alice",
                cast_votes: 18,
                cast_votes_percent: 0.6,
                winning_position: 1,
            },
            {
                election_id: "council",
                contest_id: "representative",
                area_id: "north",
                candidate_id: "bob",
                cast_votes: 12,
                cast_votes_percent: 0.4,
                winning_position: null,
            },
        ],
    }
    const manifest: ResultsManifest = {
        schema_version: 1,
        tenant_id: resultsIds.tenant,
        election_event_id: resultsIds.event,
        election_ids: ["council", "school"],
        route_scope: "event",
        publication_id: "publication-results",
        results_event_id: "results-event",
        version: 3,
        published_at: "2026-01-15T12:00:00Z",
        access: "public",
        visibility_scope: "full_event",
        default_locale: "en",
        available_languages: ["en"],
        title: {en: "Community Election Results"},
        contests: [
            {
                election_id: "council",
                contest_id: "representative",
                publication_state: "published",
                positions: 1,
            },
            {
                election_id: "council",
                contest_id: "representative",
                area_id: "north",
                publication_state: "published",
                positions: 1,
            },
            {
                election_id: "council",
                contest_id: "representative",
                area_id: "south",
                publication_state: "not_published",
                positions: 1,
            },
            {
                election_id: "school",
                contest_id: "school-seat",
                publication_state: "published",
                positions: 1,
            },
        ],
        artifacts: {full_sqlite: {public_path: "results/full.sqlite"}},
    }
    const index: ResultsPublicationIndex = {
        schema_version: 1,
        tenant_id: resultsIds.tenant,
        election_event_id: resultsIds.event,
        publications: [
            {
                publication_id: manifest.publication_id,
                route_scope: "event",
                route: `/${resultsIds.event}`,
                election_ids: ["council", "school"],
                access: "public",
                manifest_public_path: "results/manifest.json",
            },
        ],
    }
    return {dataset, manifest, index}
}
