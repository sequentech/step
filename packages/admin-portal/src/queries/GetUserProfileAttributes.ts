// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const USER_PROFILE_ATTRIBUTES = gql`
    query getUserProfileAttributes($tenantId: String!, $electionEventId: String) {
        get_user_profile_attributes(tenant_id: $tenantId, election_event_id: $electionEventId) {
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
    }
`
