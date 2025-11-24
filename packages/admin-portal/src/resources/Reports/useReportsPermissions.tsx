// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export function useReportsPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    /**
     * Permissions
     */
    const canReadReports = authContext.isAuthorized(true, tenantId, IPermissions.REPORT_READ)
    const canWriteReports = authContext.isAuthorized(true, tenantId, IPermissions.REPORT_WRITE)
    const canCreateReports = authContext.isAuthorized(true, tenantId, IPermissions.REPORT_CREATE)
    const canDeleteReports = authContext.isAuthorized(true, tenantId, IPermissions.REPORT_DELETE)
    const canGenerateReports = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.REPORT_GENERATE
    )
    const canPreviewReports = authContext.isAuthorized(true, tenantId, IPermissions.REPORT_PREVIEW)

    const showReportsColumns = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.EE_REPORTS_COLUMNS
    )
    /**
     * Permissions
     */

    return {
        canReadReports,
        canWriteReports,
        canCreateReports,
        canDeleteReports,
        canGenerateReports,
        canPreviewReports,
        showReportsColumns,
    }
}
