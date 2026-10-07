// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const USER_PROFILE_CONFIGURATION = gql`
    query GetUserProfileConfiguration($tenantId: String!, $electionEventId: String) {
        get_user_profile_configuration(tenant_id: $tenantId, election_event_id: $electionEventId) {
            attributes {
                annotations
                display_name
                group
                multivalued
                name
                required
                validations
                permissions
                selector
            }
            groups {
                annotations
                display_description
                display_header
                name
            }
        }
    }
`
