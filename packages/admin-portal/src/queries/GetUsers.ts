// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const LIST_USERS = gql`
    query getUsers(
        $tenant_id: uuid!
        $election_event_id: uuid
        $election_id: uuid
        $email: jsonb
        $username: jsonb
        $first_name: jsonb
        $last_name: jsonb
        $limit: Int
        $offset: Int
        $showVotesInfo: Boolean
        $attributes: jsonb
        $enabled: Boolean
        $email_verified: Boolean
        $sort: jsonb
        $has_voted: Boolean
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
                has_voted: $has_voted
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

/* take attributes obj when key is attribute field.
if for example, originally the field looks like sequent.read-only.otp-method,
we will receive it as sequent%read-only%otp-method
*/
export const formatUserAtributesToJsonb = (attributes: any) => {
    const newUserAttributesObject: Record<string, any> = {}
    if (attributes) {
        Object.entries(attributes).forEach(([key, value]) => {
            const convertedKey = key.replaceAll("%", ".")
            newUserAttributesObject[`'${convertedKey}'`] = value
        })
        return newUserAttributesObject
    }
    return null
}

const ATTRIBUTES = "attributes"
export const formatUserSortToJsonb = (sort: Record<string, string>) => {
    const newUserSortObject: Record<string, string> = {}
    if (sort) {
        Object.entries(sort).forEach(([key, value]) => {
            let actuallValue = value
            // if value is as attributes['field'] it shoulde be just field
            if (value.includes(ATTRIBUTES)) {
                actuallValue = value.substring(ATTRIBUTES.length + 2, value.length - 2)
            }
            newUserSortObject[`'${key}'`] = actuallValue
        })
        return newUserSortObject
    }
    return null
}

export const customBuildGetUsersVariables =
    (introspectionResults: any) =>
    (resource: any, raFetchType: any, params: any, nullParam: any) => {
        const {filter, pagination, sort} = params
        return {
            tenant_id: filter.tenant_id || null,
            election_event_id: filter.election_event_id || null,
            election_id: filter.election_id || null,
            email: filter.email || null,
            username: filter.username || null,
            first_name: filter.first_name || null,
            last_name: filter.last_name || null,
            limit: pagination?.perPage ? pagination?.perPage : null,
            offset:
                pagination?.page && pagination?.perPage
                    ? (pagination.page - 1) * pagination.perPage
                    : null,
            showVotesInfo: filter.election_event_id ? true : false,
            attributes: filter.attributes ? formatUserAtributesToJsonb(filter.attributes) : null,
            enabled: filter.enabled ?? null,
            email_verified: filter.email_verified ?? null,
            sort: sort ? formatUserSortToJsonb(sort) : null,
            has_voted: filter.has_voted ?? null,
        }
    }
