// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const getRoles = (fields: any) => {
    return gql`
        query getRoles(
            $tenant_id: String! = "${fields.tenant_id}"
        ) {
            get_roles(body: {
                tenant_id: $tenant_id
            }) {
                items {
                    id
                    name
                    permissions
                    access
                    attributes
                    client_roles
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
