// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_TALLY_DATA = gql`
    query GetTallyData(
        $tenantId: uuid!
        $resultsEventId: uuid!
        $electionEventId: uuid!
        $contestIds: [uuid!]!
        $electionIds: [uuid!]!
    ) {
        sequent_backend_area(
            where: {election_event_id: {_eq: $electionEventId}, tenant_id: {_eq: $tenantId}}
        ) {
            annotations
            created_at
            description
            election_event_id
            id
            labels
            last_updated_at
            name
            tenant_id
            type
        }
        sequent_backend_area_contest(
            where: {
                election_event_id: {_eq: $electionEventId}
                tenant_id: {_eq: $tenantId}
                contest_id: {_in: $contestIds}
            }
        ) {
            tenant_id
            last_updated_at
            labels
            id
            election_event_id
            created_at
            contest_id
            area_id
            annotations
        }
        sequent_backend_election(
            where: {
                election_event_id: {_eq: $electionEventId}
                tenant_id: {_eq: $tenantId}
                id: {_in: $electionIds}
            }
        ) {
            tenant_id
            status
            alias
            statistics
            spoil_ballot_option
            receipts
            presentation
            num_allowed_revotes
            name
            last_updated_at
            labels
            is_kiosk
            is_consolidated_ballot_encoding
            image_document_id
            id
            eml
            election_event_id
            description
            created_at
        }
        sequent_backend_candidate(
            where: {
                election_event_id: {_eq: $electionEventId}
                tenant_id: {_eq: $tenantId}
                contest_id: {_in: $contestIds}
            }
        ) {
            type
            tenant_id
            presentation
            name
            last_updated_at
            labels
            is_public
            image_document_id
            id
            election_event_id
            description
            created_at
            contest_id
            annotations
            alias
        }
        sequent_backend_contest(
            where: {
                election_event_id: {_eq: $electionEventId}
                tenant_id: {_eq: $tenantId}
                id: {_in: $contestIds}
            }
        ) {
            winning_candidates_num
            voting_type
            tenant_id
            tally_configuration
            presentation
            name
            min_votes
            max_votes
            last_updated_at
            labels
            is_encrypted
            is_active
            is_acclaimed
            image_document_id
            id
            election_id
            election_event_id
            description
            created_at
            counting_algorithm
            conditions
            annotations
            alias
        }
        sequent_backend_results_event(
            where: {
                election_event_id: {_eq: $electionEventId}
                tenant_id: {_eq: $tenantId}
                id: {_eq: $resultsEventId}
            }
        ) {
            tenant_id
            name
            last_updated_at
            labels
            id
            election_event_id
            documents
            created_at
            annotations
        }
        sequent_backend_results_election(
            where: {
                election_event_id: {_eq: $electionEventId}
                tenant_id: {_eq: $tenantId}
                results_event_id: {_eq: $resultsEventId}
            }
        ) {
            total_voters_percent
            total_voters
            tenant_id
            results_event_id
            name
            last_updated_at
            labels
            id
            elegible_census
            election_id
            election_event_id
            documents
            created_at
            annotations
        }
        sequent_backend_results_contest_candidate(
            where: {
                election_event_id: {_eq: $electionEventId}
                tenant_id: {_eq: $tenantId}
                results_event_id: {_eq: $resultsEventId}
            }
        ) {
            winning_position
            tenant_id
            results_event_id
            points
            last_updated_at
            labels
            id
            election_id
            election_event_id
            documents
            created_at
            contest_id
            cast_votes_percent
            cast_votes
            candidate_id
            annotations
        }
        sequent_backend_results_contest(
            where: {
                election_event_id: {_eq: $electionEventId}
                tenant_id: {_eq: $tenantId}
                results_event_id: {_eq: $resultsEventId}
            }
        ) {
            voting_type
            total_votes_percent
            total_votes
            total_auditable_votes
            total_auditable_votes_percent
            total_valid_votes_percent
            total_valid_votes
            total_invalid_votes_percent
            total_invalid_votes
            tenant_id
            results_event_id
            name
            last_updated_at
            labels
            implicit_invalid_votes_percent
            implicit_invalid_votes
            id
            explicit_invalid_votes_percent
            explicit_invalid_votes
            elegible_census
            election_id
            election_event_id
            documents
            created_at
            counting_algorithm
            contest_id
            blank_votes_percent
            blank_votes
            annotations
        }
        sequent_backend_results_area_contest_candidate(
            where: {
                election_event_id: {_eq: $electionEventId}
                tenant_id: {_eq: $tenantId}
                results_event_id: {_eq: $resultsEventId}
            }
        ) {
            winning_position
            tenant_id
            results_event_id
            points
            last_updated_at
            labels
            id
            election_id
            election_event_id
            documents
            created_at
            contest_id
            cast_votes_percent
            cast_votes
            candidate_id
            area_id
            annotations
        }
        sequent_backend_results_area_contest(
            where: {
                election_event_id: {_eq: $electionEventId}
                tenant_id: {_eq: $tenantId}
                results_event_id: {_eq: $resultsEventId}
            }
        ) {
            annotations
            area_id
            blank_votes
            blank_votes_percent
            contest_id
            created_at
            documents
            election_event_id
            election_id
            elegible_census
            explicit_invalid_votes
            explicit_invalid_votes_percent
            id
            implicit_invalid_votes
            implicit_invalid_votes_percent
            labels
            last_updated_at
            results_event_id
            tenant_id
            total_invalid_votes
            total_invalid_votes_percent
            total_valid_votes
            total_valid_votes_percent
            total_votes
            total_votes_percent
            total_auditable_votes
            total_auditable_votes_percent
        }
        sequent_backend_results_election_area(
            where: {
                election_event_id: {_eq: $electionEventId}
                tenant_id: {_eq: $tenantId}
                results_event_id: {_eq: $resultsEventId}
            }
        ) {
            id
            tenant_id
            election_event_id
            election_id
            area_id
            results_event_id
            created_at
            last_updated_at
            documents
            name
        }
    }
`
