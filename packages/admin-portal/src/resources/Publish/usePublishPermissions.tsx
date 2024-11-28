// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export function usePublishPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    /**
     * Permissions
     */
    const canReadPublish = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_READ)
    // const canExportPublish = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_EXPORT)

    // const showPublishColumns = authContext.isAuthorized(
    //     true,
    //     tenantId,
    //     IPermissions.EE_PUBLISH_COLUMNS
    // )
    // const showPublishFilters = authContext.isAuthorized(
    //     true,
    //     tenantId,
    //     IPermissions.EE_PUBLISH_FILTERS
    // )
    /**
     * Permissions
     */

    return {
        canReadPublish,
        // canExportPublish,
        // showPublishColumns,
        // showPublishFilters,
    }
}
