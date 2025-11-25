// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GetCastVotesByIp = (params: any) => {
    const {filter, pagination} = params
    const offset: number | null =
        pagination?.page && pagination?.perPage ? (pagination.page - 1) * pagination.perPage : null
    const limit: number | null = pagination?.perPage ? pagination?.perPage : null

    const ip = filter && filter.ip ? `"${filter.ip}"` : "null"
    const country = filter && filter.country ? `"${filter.country}"` : "null"
    const election_id = filter && filter.election_id ? `"${filter.election_id}"` : "null"

    return gql`
        query GetCastVotesByIp(
            $election_event_id: uuid! = "${filter.election_event_id}"
            $limit: Int = ${limit}
            $offset: Int = ${offset}
            $ip: String = ${ip}
            $country: String = ${country}
            $election_id: String = ${election_id}
        ) {
            get_top_votes_by_ip(body: {
                election_event_id: $election_event_id
                limit: $limit
                offset: $offset
                ip: $ip
                country: $country
                election_id: $election_id
            }) {
                items {
                    id
                    ip
                    country
                    election_name
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
}
