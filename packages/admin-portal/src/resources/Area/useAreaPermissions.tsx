// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export function useAreaPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    /**
     * Permissions
     */
    const canCreateArea = authContext.isAuthorized(true, tenantId, IPermissions.AREA_CREATE)
    const canEditArea = authContext.isAuthorized(true, tenantId, IPermissions.AREA_WRITE)
    const canReadArea = authContext.isAuthorized(true, tenantId, IPermissions.AREA_READ)
    const canDeleteArea = authContext.isAuthorized(true, tenantId, IPermissions.AREA_DELETE)
    const canImportArea = authContext.isAuthorized(true, tenantId, IPermissions.AREA_IMPORT)
    const canExportArea = authContext.isAuthorized(true, tenantId, IPermissions.AREA_EXPORT)
    const canUpsertArea = authContext.isAuthorized(true, tenantId, IPermissions.AREA_UPSERT)

    const showAreaColumns = authContext.isAuthorized(true, tenantId, IPermissions.EE_AREAS_COLUMNS)
    const showAreaFilters = authContext.isAuthorized(true, tenantId, IPermissions.EE_AREAS_FILTERS)
    /**
     * Permissions
     */

    return {
        canCreateArea,
        canEditArea,
        canReadArea,
        canDeleteArea,
        canImportArea,
        canExportArea,
        canUpsertArea,
        showAreaColumns,
        showAreaFilters,
    }
}
