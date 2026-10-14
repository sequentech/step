// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

import {GraphQLInt, GraphQLString, astFromValue, visit} from "graphql"

const CAST_VOTES_BY_IP = gql`
    query GetCastVotesByIp(
        $election_event_id: uuid!
        $limit: Int = null
        $offset: Int = null
        $ip: String = null
        $country: String = null
        $election_id: String = null
    ) {
        get_top_votes_by_ip(
            body: {
                election_event_id: $election_event_id
                limit: $limit
                offset: $offset
                ip: $ip
                country: $country
                election_id: $election_id
            }
        ) {
            items {
                id
                ip
                country
                election_presentation
                vote_count
                voters_id
            }
            total {
                aggregate {
                    count
                }
            }
        }
    }
`

/** Retain the existing per-call defaults without interpolating GraphQL source. */
export const GetCastVotesByIp = (params: any) => {
    const {filter, pagination} = params
    const defaults: Record<string, string | number | null> = {
        election_event_id: filter.election_event_id,
        limit: pagination?.perPage || null,
        offset:
            pagination?.page && pagination?.perPage
                ? (pagination.page - 1) * pagination.perPage
                : null,
        ip: filter?.ip || null,
        country: filter?.country || null,
        election_id: filter?.election_id || null,
    }
    return visit(CAST_VOTES_BY_IP, {
        VariableDefinition(node) {
            const name = node.variable.name.value
            return {
                ...node,
                defaultValue: astFromValue(
                    defaults[name],
                    name === "limit" || name === "offset" ? GraphQLInt : GraphQLString
                ),
            }
        },
    })
}
