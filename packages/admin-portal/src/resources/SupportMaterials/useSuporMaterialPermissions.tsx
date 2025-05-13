// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export function useSuportMaterialPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    /**
     * Permissions
     */
    const canReadSuportMaterial = authContext.isAuthorized(true, tenantId, IPermissions.SUPPORT_MATERIAL_READ)
    const canWriteSuportMaterial = authContext.isAuthorized(true, tenantId, IPermissions.SUPPORT_MATERIAL_WRITE)
    const canCreateSuportMaterial = authContext.isAuthorized(true, tenantId, IPermissions.SUPPORT_MATERIAL_WRITE)
    const canDeleteSuportMaterial = authContext.isAuthorized(true, tenantId, IPermissions.SUPPORT_MATERIAL_WRITE)
    /**
     * Permissions
     */

    return {
        canReadSuportMaterial,
        canWriteSuportMaterial,
        canCreateSuportMaterial,
        canDeleteSuportMaterial,
    }
}
