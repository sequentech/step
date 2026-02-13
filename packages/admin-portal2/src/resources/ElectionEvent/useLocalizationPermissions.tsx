// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export function useLocalizationPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    /**
     * Permissions
     */
    const canCreateLocalization = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.LOCALIZATION_CREATE
    )
    const canEditLocalization = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.LOCALIZATION_WRITE
    )
    const canDeleteLocalization = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.LOCALIZATION_DELETE
    )

    const showLocalizationSelector = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.EE_LOCALIZATION_SELECTOR
    )
    /**
     * Permissions
     */

    return {
        canCreateLocalization,
        canEditLocalization,
        canDeleteLocalization,
        showLocalizationSelector,
    }
}
