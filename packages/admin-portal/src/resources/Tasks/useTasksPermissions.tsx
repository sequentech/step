// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export function useTasksPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    /**
     * Permissions
     */
    const canReadTasks = authContext.isAuthorized(true, tenantId, IPermissions.TASKS_READ)
    const canExportTasks = authContext.isAuthorized(true, tenantId, IPermissions.TASKS_EXPORT)

    const showTasksColumns = authContext.isAuthorized(true, tenantId, IPermissions.EE_TASKS_COLUMNS)
    const showTasksFilters = authContext.isAuthorized(true, tenantId, IPermissions.EE_TASKS_FILTERS)
    const showTasksBackButton = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.EE_TASKS_BACK_BUTTON
    )
    /**
     * Permissions
     */

    return {
        canReadTasks,
        canExportTasks,
        showTasksColumns,
        showTasksFilters,
        showTasksBackButton,
    }
}
