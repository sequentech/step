// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

/** Preview or apply a complete configuration package (hasura action
 * apply_configuration_package, harvest POST /apply-configuration-package). */
export const APPLY_CONFIGURATION_PACKAGE = gql`
    mutation ApplyConfigurationPackage(
        $tenantId: String!
        $documentId: String!
        $mode: String!
        $requirements: jsonb
    ) {
        apply_configuration_package(
            tenant_id: $tenantId
            document_id: $documentId
            mode: $mode
            requirements: $requirements
        ) {
            task_execution {
                id
                execution_status
                annotations
            }
        }
    }
`

/** The tenant's earlier package applications, newest first. */
export const LIST_PACKAGE_APPLICATIONS = gql`
    query ListPackageApplications($tenantId: uuid!) {
        sequent_backend_tasks_execution(
            where: {tenant_id: {_eq: $tenantId}, type: {_eq: "APPLY_CONFIGURATION_PACKAGE"}}
            order_by: {created_at: desc}
            limit: 10
        ) {
            id
            created_at
            execution_status
            executed_by_user
            annotations
        }
    }
`

/** One package application's task row, looked up within the tenant only. */
export const GET_PACKAGE_TASK = gql`
    query GetPackageTask($taskId: uuid!, $tenantId: uuid!) {
        sequent_backend_tasks_execution(where: {id: {_eq: $taskId}, tenant_id: {_eq: $tenantId}}) {
            id
            execution_status
            annotations
        }
    }
`
