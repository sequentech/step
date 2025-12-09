// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const LIST_KEYS_CEREMONY = gql`
    query ListKeysCeremony($electionEventId: String!) {
        list_keys_ceremony(election_event_id: $electionEventId) {
            items {
                id
                created_at
                last_updated_at
                tenant_id
                election_event_id
                trustee_ids
                status
                execution_status
                labels
                annotations
                threshold
                name
                settings
                is_default
                permission_label
            }
            total {
                aggregate {
                    count
                }
            }
        }
    }
`
