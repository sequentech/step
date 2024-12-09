// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export function useKeysPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    const canExportCeremony = authContext.isAuthorized(true, tenantId, IPermissions.EXPORT_CEREMONY)
    const canCreateCeremony = authContext.isAuthorized(true, tenantId, IPermissions.CREATE_CEREMONY)
    const canAdminCeremony = authContext.isAuthorized(true, tenantId, IPermissions.ADMIN_CEREMONY)
    const canTrusteeCeremony = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.TRUSTEE_CEREMONY
    )

    const showKeysColumns = authContext.isAuthorized(true, tenantId, IPermissions.EE_KEYS_COLUMNS)
    const showTallyColumns = authContext.isAuthorized(true, tenantId, IPermissions.EE_TALLY_COLUMNS)
    const showTransmitionCremony = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.TRANSMITION_CEREMONY
    )
    const showTallyBackButton = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.EE_TALLY_BACK_BUTTON
    )

    return {
        canAdminCeremony,
        canTrusteeCeremony,
        canExportCeremony,
        canCreateCeremony,
        showKeysColumns,
        showTallyColumns,
        showTallyBackButton,
        showTransmitionCremony,
    }
}
