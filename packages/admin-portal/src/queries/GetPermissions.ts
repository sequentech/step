// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const getPermissions = (fields: any) => {
    return gql`
        query getPermissions(
            $tenant_id: String! = "${fields.tenant_id}"
        ) {
            get_permissions(body: {
                tenant_id: $tenant_id
            }) {
                items {
                    id
                    attributes
                    container_id
                    description
                    name
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
