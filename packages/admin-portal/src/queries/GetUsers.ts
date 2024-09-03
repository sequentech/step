// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const getUsers = (fields: any) => {
    console.log("fields", fields)

    let electionEventId = fields.filter?.election_event_id
        ? `"${fields.filter?.election_event_id}"`
        : "null"
    let showVotesInfo = fields.filter?.election_event_id ? "true" : "false"
    let electionId = fields.filter?.election_id ? `"${fields.filter?.election_id}"` : "null"
    let email = fields.filter?.email ? `"${fields.filter?.email}"` : "null"
    let username = fields.filter?.username ? `"${fields.filter?.username}"` : "null"
    let first_name = fields.filter?.first_name ? `"${fields.filter?.first_name}"` : "null"
    let last_name = fields.filter?.last_name ? `"${fields.filter?.last_name}"` : "null"
    let offset: number | null =
        fields.pagination?.page && fields.pagination?.perPage
            ? (fields.pagination.page - 1) * fields.pagination.perPage
            : null
    let limit: number | null = fields.pagination?.perPage ? fields.pagination?.perPage : null
    let attributes = fields.filter?.attributes
        ? `"${fields.filter?.attributes.toString()}"`
        : "null"
    console.log("attributes:::", attributes)

    return gql`
        query getUsers(
            $tenant_id: uuid! = "${fields.filter.tenant_id}"
            $election_event_id: uuid = ${electionEventId}
            $election_id: uuid = ${electionId}
            $email: String = ${email}
            $username: String = ${username}
            $first_name: String = ${first_name}
            $last_name: String = ${last_name}
            $limit: Int = ${limit}
            $offset: Int = ${offset}
            $showVotesInfo: Boolean = ${showVotesInfo}
            $attributes: jsonb = ${attributes}
        ) {
            get_users(body: {
                tenant_id: $tenant_id,
                election_event_id: $election_event_id,
                election_id: $election_id,
                email: $email,
                username: $username,
                first_name: $first_name,
                last_name: $last_name,
                limit: $limit,
                offset: $offset,
                show_votes_info: $showVotesInfo
                attributes: $attributes
            }) {
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
}
