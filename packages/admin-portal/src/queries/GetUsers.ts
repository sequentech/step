// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const LIST_USERS = gql`
    query getUsers(
        $tenant_id: uuid!
        $election_event_id: uuid
        $election_id: uuid
        $email: String
        $username: String
        $first_name: String
        $last_name: String
        $limit: Int
        $offset: Int
        $showVotesInfo: Boolean
        $attributes: jsonb
        $enabled: Boolean
        $email_verified: Boolean
        $sort: jsonb
    ) {
        get_users(
            body: {
                tenant_id: $tenant_id
                election_event_id: $election_event_id
                election_id: $election_id
                email: $email
                username: $username
                first_name: $first_name
                last_name: $last_name
                limit: $limit
                offset: $offset
                show_votes_info: $showVotesInfo
                attributes: $attributes
                enabled: $enabled
                email_verified: $email_verified
                sort: $sort
            }
        ) {
            items {
                id
                attributes
                email
                email_verified
                enabled
                first_name
                groups
                last_name
                username
                area {
                    id
                    name
                }
                votes_info {
                    election_id
                    num_votes
                    last_voted_at
                }
            }
            total {
                aggregate {
                    count
                }
            }
        }
    }
`

export const formatUserAtributestoJsonb = (obj: any) => {
    const newUserAttributesObject: Record<string, any> = {}
    if (obj) {
        Object.entries(obj).forEach(([key, value]) => {
            const new_key = key.replaceAll("%", ".")
            newUserAttributesObject[`'${new_key}'`] = value
        })
        return newUserAttributesObject
    }
    return null
}

export const customBuildGetUsersVariables =
    (introspectionResults: any) =>
    (resource: any, raFetchType: any, params: any, nullParam: any) => {
        const {filter, pagination, sort} = params
        console.log("sort: ", sort)

        return {
            tenant_id: filter.tenant_id || null,
            election_event_id: filter.election_event_id || null,
            election_id: filter.election_id || null,
            email: filter.email || null,
            username: filter.username || null,
            first_name: filter.first_name || null,
            last_name: filter.last_name || null,
            limit: pagination.perPage || 10,
            offset:
                pagination?.page && pagination?.perPage
                    ? (pagination.page - 1) * pagination.perPage
                    : null,
            showVotesInfo: filter.showVotesInfo || false,
            attributes: filter.attributes ? formatUserAtributestoJsonb(filter.attributes) : null,
            enabled: filter.enabled ?? null,
            email_verified: filter.email_verified ?? null,
            sort: sort ? formatUserAtributestoJsonb(sort) : null,
        }
    }
