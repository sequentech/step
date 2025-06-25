// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export function useLogsPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    /**
     * Permissions
     */
    const canReadLogs = authContext.isAuthorized(true, tenantId, IPermissions.LOGS_READ)
    const canExportLogs = authContext.isAuthorized(true, tenantId, IPermissions.LOGS_EXPORT)

    const showLogsColumns = authContext.isAuthorized(true, tenantId, IPermissions.EE_LOGS_COLUMNS)
    const showLogsFilters = authContext.isAuthorized(true, tenantId, IPermissions.EE_LOGS_FILTERS)
    /**
     * Permissions
     */

    return {
        canReadLogs,
        canExportLogs,
        showLogsColumns,
        showLogsFilters,
    }
}
