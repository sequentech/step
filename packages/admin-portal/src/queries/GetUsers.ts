// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const getUsers = (fields: any) => {
    return gql`
        query getUsers(
            $tenant_id: String! = "${fields.tenant_id}"
            $election_event_id: String = ${
                fields?.election_event_id ? `"${fields?.election_event_id}"` : "null"
            }
        ) {
            get_users(body: {
                tenant_id: $tenant_id,
                election_event_id: $election_event_id
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
